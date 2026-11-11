//! Starts, feeds and kills the trace worker process.

use std::io::{BufReader, Read, Write};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use eframe::egui;
use tracyo_core::picture::Picture;
use tracyo_core::progress::failure_reason;
use tracyo_core::protocol::{HEADER_LEN, Reply, encode_picture, parse_header};
use tracyo_core::request::TraceRequest;

/// What the runner tells the app.
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    /// The job of `job` reached this percent.
    Percent {
        /// The job.
        job: u64,
        /// Whole percent.
        percent: u8,
    },
    /// The job finished with vectrace's numbers.
    Wire {
        /// The job.
        job: u64,
        /// The result.
        numbers: Vec<f64>,
    },
    /// The job failed.
    Failed {
        /// The job.
        job: u64,
        /// "out of memory at 16 Mpx" or "internal error".
        reason: String,
    },
}

enum Raw {
    Reply(u64, Reply),
    Eof(u64),
}

enum Out {
    Picture(Arc<Picture>),
    Frame(Vec<u8>),
}

struct Worker {
    generation: u64,
    child: Child,
    outbox: Sender<Out>,
    picture_id: Option<u64>,
    stderr: Arc<Mutex<String>>,
    stderr_thread: Option<JoinHandle<()>>,
}

/// One worker process at a time, replaced for every job that interrupts another.
pub struct Runner {
    exe: PathBuf,
    ctx: egui::Context,
    raw_tx: Sender<Raw>,
    raw_rx: Receiver<Raw>,
    worker: Option<Worker>,
    next_generation: u64,
    job: Option<(u64, u64)>,
    pixels: u64,
    spawn_prefix: Vec<String>,
}

fn read_frames(mut stdout: impl Read, generation: u64, tx: &Sender<Raw>, ctx: &egui::Context) {
    loop {
        let mut header = [0_u8; HEADER_LEN];
        if stdout.read_exact(&mut header).is_err() {
            break;
        }
        let (tag, len) = parse_header(header);
        let mut payload = vec![0_u8; len];
        if stdout.read_exact(&mut payload).is_err() {
            break;
        }
        match Reply::decode(tag, &payload) {
            Ok(reply) => {
                if tx.send(Raw::Reply(generation, reply)).is_err() {
                    return;
                }
                ctx.request_repaint();
            }
            Err(_) => break,
        }
    }
    let _ = tx.send(Raw::Eof(generation));
    ctx.request_repaint();
}

impl Runner {
    /// A runner that starts `exe --trace-worker`.
    #[must_use]
    pub fn new(exe: PathBuf, ctx: egui::Context) -> Self {
        let (raw_tx, raw_rx) = channel();
        Self {
            exe,
            ctx,
            raw_tx,
            raw_rx,
            worker: None,
            next_generation: 1,
            job: None,
            pixels: 0,
            spawn_prefix: Vec::new(),
        }
    }

    /// Wraps the worker command (for example `prlimit --as=...`): the first element is the
    /// program, the rest come before the worker's own program path.
    pub fn wrap_command(&mut self, prefix: Vec<String>) {
        self.kill_worker();
        self.job = None;
        self.spawn_prefix = prefix;
    }

    fn spawn(&mut self) -> std::io::Result<Worker> {
        let mut command = if let Some((program, rest)) = self.spawn_prefix.split_first() {
            let mut c = Command::new(program);
            c.args(rest).arg(&self.exe);
            c
        } else {
            Command::new(&self.exe)
        };
        command
            .arg("--trace-worker")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x0800_0000);
        }
        let mut child = command.spawn()?;
        let generation = self.next_generation;
        self.next_generation += 1;
        let stdin = child.stdin.take();
        let stdout = child.stdout.take();
        let stderr_pipe = child.stderr.take();
        let (outbox, from_app) = channel::<Out>();
        let stderr = Arc::new(Mutex::new(String::new()));
        let mut stderr_thread = None;
        if let Some(mut stdin) = stdin {
            std::thread::spawn(move || {
                while let Ok(out) = from_app.recv() {
                    let bytes = match out {
                        Out::Picture(p) => encode_picture(&p),
                        Out::Frame(f) => f,
                    };
                    if stdin
                        .write_all(&bytes)
                        .and_then(|()| stdin.flush())
                        .is_err()
                    {
                        break;
                    }
                }
            });
        }
        if let Some(stdout) = stdout {
            let (tx, ctx) = (self.raw_tx.clone(), self.ctx.clone());
            std::thread::spawn(move || read_frames(BufReader::new(stdout), generation, &tx, &ctx));
        }
        if let Some(mut pipe) = stderr_pipe {
            let sink = Arc::clone(&stderr);
            stderr_thread = Some(std::thread::spawn(move || {
                let mut text = String::new();
                let mut bytes = Vec::new();
                let _ = pipe.read_to_end(&mut bytes);
                text.push_str(&String::from_utf8_lossy(&bytes));
                if let Ok(mut guard) = sink.lock() {
                    *guard = text;
                }
            }));
        }
        Ok(Worker {
            generation,
            child,
            outbox,
            picture_id: None,
            stderr,
            stderr_thread,
        })
    }

    fn kill_worker(&mut self) {
        if let Some(mut worker) = self.worker.take() {
            let _ = worker.child.kill();
            let _ = worker.child.wait();
            // The reader and writer threads end by themselves when the pipes close.
            drop(worker.outbox);
        }
    }

    /// Whether a job is running.
    #[must_use]
    pub fn busy(&self) -> bool {
        self.job.is_some()
    }

    /// Starts job number `job`: the picture (identified by `picture_id`) is sent when this worker
    /// does not hold it. A running job is killed first (latest wins). `Err` is the reason a worker
    /// could not be started.
    ///
    /// # Errors
    /// The system's message when the worker process cannot be started.
    pub fn start(
        &mut self,
        job: u64,
        picture_id: u64,
        picture: &Arc<Picture>,
        request: TraceRequest,
    ) -> Result<(), String> {
        if self.job.is_some() {
            self.kill_worker();
            self.job = None;
        }
        if self.worker.is_none() {
            self.worker = Some(self.spawn().map_err(|e| e.to_string())?);
        }
        let worker = self.worker.as_mut().ok_or_else(|| "no worker".to_owned())?;
        if worker.picture_id != Some(picture_id) {
            let _ = worker.outbox.send(Out::Picture(Arc::clone(picture)));
            worker.picture_id = Some(picture_id);
        }
        let _ = worker.outbox.send(Out::Frame(
            tracyo_core::protocol::Request::Trace(request).encode(),
        ));
        self.job = Some((job, worker.generation));
        self.pixels = picture.pixels();
        Ok(())
    }

    /// Kills the running job (Stop, Esc, a new picture); the worker is replaced on the next start.
    pub fn stop(&mut self) {
        if self.job.take().is_some() {
            self.kill_worker();
        }
    }

    /// The events that arrived since the last call.
    pub fn poll(&mut self) -> Vec<Event> {
        let mut events = Vec::new();
        while let Ok(raw) = self.raw_rx.try_recv() {
            let Some((job, generation)) = self.job else {
                continue;
            };
            match raw {
                Raw::Reply(g, _) | Raw::Eof(g) if g != generation => {}
                Raw::Reply(_, Reply::Percent(percent)) => {
                    events.push(Event::Percent { job, percent });
                }
                Raw::Reply(_, Reply::Wire(numbers)) => {
                    self.job = None;
                    events.push(Event::Wire { job, numbers });
                }
                Raw::Reply(_, Reply::Error(_)) => {
                    self.job = None;
                    self.kill_worker();
                    events.push(Event::Failed {
                        job,
                        reason: "internal error".into(),
                    });
                }
                Raw::Eof(_) => {
                    self.job = None;
                    let reason = self.exit_reason();
                    self.kill_worker();
                    events.push(Event::Failed { job, reason });
                }
            }
        }
        events
    }

    fn exit_reason(&mut self) -> String {
        let Some(worker) = self.worker.as_mut() else {
            return "internal error".into();
        };
        let status = worker.child.wait().ok();
        if let Some(thread) = worker.stderr_thread.take() {
            let _ = thread.join();
        }
        let stderr = worker.stderr.lock().map(|s| s.clone()).unwrap_or_default();
        #[cfg(unix)]
        let killed = {
            use std::os::unix::process::ExitStatusExt;
            status.is_some_and(|s| s.signal() == Some(9))
        };
        #[cfg(not(unix))]
        let killed = {
            let _ = status;
            false
        };
        failure_reason(&stderr, self.pixels, killed)
    }
}

impl Drop for Runner {
    fn drop(&mut self) {
        self.kill_worker();
    }
}
