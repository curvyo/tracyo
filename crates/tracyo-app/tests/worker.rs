//! The worker process itself and the runner that starts, feeds and kills it.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::time::{Duration, Instant};

use tracyo_app::runner::{Event, Runner};
use tracyo_core::picture::Picture;
use tracyo_core::protocol::{HEADER_LEN, Reply, Request, parse_header};
use tracyo_core::request::TraceRequest;
use tracyo_core::settings::Settings;

fn exe() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_tracyo"))
}

fn square_picture() -> Picture {
    let mut rgba = vec![255; 60 * 60 * 4];
    for y in 15..45 {
        for x in 15..45 {
            let i = (y * 60 + x) * 4;
            rgba[i..i + 3].fill(0);
        }
    }
    Picture {
        width: 60,
        height: 60,
        rgba,
    }
}

fn request() -> TraceRequest {
    TraceRequest {
        threshold: 128,
        settings: Settings::default(),
        selection: vectrace::Selection::Brightness,
        invert: false,
        drop_border: false,
        reduced: false,
    }
}

/// The binary started as `tracyo --trace-worker` answers a picture and a trace over its pipes.
#[test]
fn the_worker_binary_traces_over_its_pipes() {
    let mut child = Command::new(exe())
        .arg("--trace-worker")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    stdin
        .write_all(&Request::Picture(square_picture()).encode())
        .unwrap();
    stdin
        .write_all(&Request::Trace(request()).encode())
        .unwrap();
    drop(stdin);
    let mut stdout = child.stdout.take().unwrap();
    let mut wire = None;
    loop {
        let mut header = [0_u8; HEADER_LEN];
        if stdout.read_exact(&mut header).is_err() {
            break;
        }
        let (tag, len) = parse_header(header);
        let mut payload = vec![0; len];
        stdout.read_exact(&mut payload).unwrap();
        if let Reply::Wire(numbers) = Reply::decode(tag, &payload).unwrap() {
            wire = Some(numbers);
        }
    }
    assert!(
        child.wait().unwrap().success(),
        "a closed stdin ends the worker"
    );
    let output = vectrace::wire::read(&wire.expect("a result")).unwrap();
    assert_eq!(output.paths, 1);
}

fn pid_alive(pid: u32) -> bool {
    std::path::Path::new(&format!("/proc/{pid}")).exists()
}

fn wait_for_pid(file: &std::path::Path) -> u32 {
    let start = Instant::now();
    loop {
        if let Ok(text) = std::fs::read_to_string(file)
            && let Ok(pid) = text.trim().parse()
        {
            return pid;
        }
        assert!(
            start.elapsed() < Duration::from_secs(10),
            "the fake worker never started"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn staged(dir: &std::path::Path) -> (Runner, PathBuf) {
    let pid_file = dir.join("pid");
    let mut runner = Runner::new(exe(), eframe::egui::Context::default());
    runner.wrap_command(vec![
        "/bin/sh".into(),
        "-c".into(),
        format!("echo $$ > {}; exec sleep 600", pid_file.display()),
    ]);
    (runner, pid_file)
}

/// Criterion 8: stopping kills the worker process at once.
#[test]
fn stop_kills_the_worker_process() {
    let dir = tempfile::tempdir().unwrap();
    let (mut runner, pid_file) = staged(dir.path());
    runner
        .start(1, 1, &Arc::new(square_picture()), request())
        .unwrap();
    let pid = wait_for_pid(&pid_file);
    assert!(pid_alive(pid));
    let start = Instant::now();
    runner.stop();
    assert!(!pid_alive(pid), "the worker is gone right after stop");
    assert!(start.elapsed() < Duration::from_secs(2));
    assert!(!runner.busy());
}

/// Criterion 24: closing the window (dropping the runner) ends the worker within 2 s.
#[test]
fn dropping_the_runner_kills_the_worker() {
    let dir = tempfile::tempdir().unwrap();
    let (mut runner, pid_file) = staged(dir.path());
    runner
        .start(1, 1, &Arc::new(square_picture()), request())
        .unwrap();
    let pid = wait_for_pid(&pid_file);
    drop(runner);
    let start = Instant::now();
    while pid_alive(pid) {
        assert!(
            start.elapsed() < Duration::from_secs(2),
            "the worker outlived the app"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// A job that interrupts another replaces the worker (latest wins) and the new result arrives.
#[test]
fn a_new_job_replaces_a_running_one() {
    let dir = tempfile::tempdir().unwrap();
    let (mut runner, pid_file) = staged(dir.path());
    let picture = Arc::new(square_picture());
    runner.start(1, 1, &picture, request()).unwrap();
    let old = wait_for_pid(&pid_file);
    runner.wrap_command(Vec::new());
    runner.start(2, 1, &picture, request()).unwrap();
    assert!(!pid_alive(old));
    let start = Instant::now();
    loop {
        let events = runner.poll();
        if let Some(Event::Wire { job, numbers }) =
            events.into_iter().find(|e| matches!(e, Event::Wire { .. }))
        {
            assert_eq!(job, 2);
            assert_eq!(vectrace::wire::read(&numbers).unwrap().paths, 1);
            break;
        }
        assert!(start.elapsed() < Duration::from_secs(20), "no result");
        std::thread::sleep(Duration::from_millis(10));
    }
}
