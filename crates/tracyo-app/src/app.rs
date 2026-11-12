//! The application state, its frame loop and the open, trace and save flows.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, channel};
use std::time::{Duration, Instant};

use eframe::egui::{self, Key, Modifiers, Pos2, Rect, Shape, Stroke, Ui, pos2, vec2};
use tracyo_core::analysis::Analysis;
use tracyo_core::method::{Method, PictureValues};
use tracyo_core::picture::{Picture, ReadError};
use tracyo_core::progress::{self, Esc, EscState, Progress, SaveCase};
use tracyo_core::request::{self, TraceRequest};
use tracyo_core::settings::Settings;
use tracyo_core::shapes;
use tracyo_core::stages::{self, Stage};
use tracyo_core::svg;
use tracyo_core::text::TEXT_FAILED_NOTICE;
use vectrace::ocr::OcrLine;
use vectrace::wire::Output;

use crate::canvas::{Tiles, View};
use crate::config::{self, Config};
use crate::dialogs::{Choice, Dialogs};
use crate::icons::Icons;
use crate::preview::Preview;
use crate::runner::{Event, Runner};
use crate::widgets::{FieldState, set_focus_visible};
use crate::{fonts, theme};

/// Height of the status bar.
pub const STATUS_HEIGHT: f32 = 24.0;
/// Where the bar and the Open card float.
pub const BAR_POS: Pos2 = pos2(72.0, 12.0);

/// What the app is started with.
pub struct Options {
    /// The executable that is started with `--trace-worker`.
    pub exe: PathBuf,
    /// Where file choices come from.
    pub dialogs: Dialogs,
    /// The settings file.
    pub config: Config,
    /// Where the UI font comes from.
    pub fonts: fonts::Source,
    /// A picture to open at start.
    pub open: Option<PathBuf>,
}

pub(crate) struct Loaded {
    pub path: PathBuf,
    pub name: String,
    pub picture: Picture,
    pub analysis: Analysis,
}

/// What is known of the text of a picture.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum TextState {
    /// Not read yet.
    Unread,
    /// The lines read from the full picture, kept for every later trace.
    Read(Vec<OcrLine>),
    /// The reader could not be used: the picture is traced without text.
    Failed,
}

pub(crate) struct Doc {
    pub path: PathBuf,
    pub name: String,
    pub picture: Arc<Picture>,
    pub id: u64,
    pub tiles: Tiles,
    pub values: PictureValues,
    pub analysis: Analysis,
    pub view: View,
    /// Set while the view is still where the last fit put it: whether that fit may enlarge.
    pub auto_fit: Option<bool>,
    pub text: TextState,
}

pub(crate) struct TraceResult {
    pub preview: Preview,
    pub output: Output,
    pub reduced: bool,
    pub stale: bool,
    pub reason: Option<String>,
}

pub(crate) struct Notice {
    pub lines: Vec<String>,
    pub alert: bool,
    pub until: f64,
}

/// Which menu is open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MenuKind {
    Preset,
    Method,
}

/// Which pipette is armed on the canvas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Pipette {
    Off,
    /// Remove background's: takes the background colour, then disarms.
    Background,
    /// Pick colours': replaces the picked colours, Shift adds; stays armed.
    Colours,
    /// Multicolour's: adds a colour to the palette; stays armed.
    Palette,
}

/// The whole application.
#[allow(clippy::struct_excessive_bools)] // independent frame flags
pub struct App {
    pub(crate) settings: Settings,
    pub(crate) config: Config,
    pub(crate) dialogs: Dialogs,
    pub(crate) runner: Runner,
    pub(crate) icons: Icons,
    pub(crate) start: Instant,
    pub(crate) doc: Option<Doc>,
    pub(crate) loading: Option<Receiver<Result<Loaded, (String, ReadError)>>>,
    pub(crate) progress: Progress,
    pub(crate) result: Option<TraceResult>,
    pub(crate) menu: Option<MenuKind>,
    pub(crate) menu_highlight: usize,
    pub(crate) fields: [FieldState; 6],
    pub(crate) pipette: Pipette,
    pub(crate) picked_stop: usize,
    pub(crate) palette_stop: usize,
    pub(crate) analysing: Option<Receiver<Analysis>>,
    pub(crate) notice: Option<Notice>,
    pub(crate) bar_bottom: f32,
    pub(crate) pending_save: Option<PathBuf>,
    pub(crate) failed_write: Option<(PathBuf, Vec<u8>)>,
    pub(crate) last_open_dir: Option<PathBuf>,
    pub(crate) dragging_value: bool,
    pub(crate) job_counter: u64,
    pub(crate) picture_counter: u64,
    pub(crate) pointer_picture: Option<(u32, u32)>,
    pub(crate) fit_next_frame: bool,
    pub(crate) fit_zoom_enlarge: bool,
    pub(crate) last_request_reduced: bool,
    /// The text of the picture is being read; the trace follows.
    pub(crate) reading: bool,
    /// What to trace when the read ends: (dragging, for saving).
    pub(crate) after_read: Option<(bool, bool)>,
    /// The running trace follows a read of the same job.
    pub(crate) trace_after_read: bool,
    pub(crate) menu_anchor: Rect,
    pub(crate) about_open: bool,
    /// The About text's scroll offset, and whether it is longer than the panel.
    pub(crate) about_scroll: f32,
    pub(crate) about_scrolls: bool,
    pub(crate) title_sent: Option<String>,
    pub(crate) window_size: Option<[f32; 2]>,
}

impl App {
    /// Creates the app: installs fonts and style, loads the settings, opens a picture if asked.
    #[must_use]
    pub fn new(ctx: &egui::Context, options: Options) -> Self {
        fonts::install(ctx, options.fonts);
        theme::install_style(ctx);
        let settings = options.config.load();
        let mut app = Self {
            settings,
            config: options.config,
            dialogs: options.dialogs,
            runner: Runner::new(options.exe, ctx.clone()),
            icons: Icons::default(),
            start: Instant::now(),
            doc: None,
            loading: None,
            progress: Progress::default(),
            result: None,
            menu: None,
            menu_highlight: 0,
            fields: Default::default(),
            pipette: Pipette::Off,
            picked_stop: 0,
            palette_stop: 0,
            analysing: None,
            notice: None,
            bar_bottom: BAR_POS.y + 36.0,
            pending_save: None,
            failed_write: None,
            last_open_dir: None,
            dragging_value: false,
            job_counter: 0,
            picture_counter: 0,
            pointer_picture: None,
            fit_next_frame: false,
            fit_zoom_enlarge: false,
            last_request_reduced: false,
            reading: false,
            after_read: None,
            trace_after_read: false,
            menu_anchor: Rect::NOTHING,
            about_open: false,
            about_scroll: 0.0,
            about_scrolls: false,
            title_sent: None,
            window_size: None,
        };
        if let Some(path) = options.open {
            app.open_path(ctx, path);
        }
        app
    }

    /// The number of traces started so far (pan and zoom never change it).
    #[must_use]
    pub fn trace_jobs(&self) -> u64 {
        self.job_counter
    }

    /// Whether a settled trace result is there.
    #[must_use]
    pub fn has_result(&self) -> bool {
        self.result.as_ref().is_some_and(|r| !r.stale)
    }

    /// The text lines set and the shapes recognised in the settled result (0, 0 without one).
    #[must_use]
    pub fn recognised(&self) -> (usize, usize) {
        self.result
            .as_ref()
            .filter(|r| !r.stale)
            .map_or((0, 0), |r| {
                (
                    r.output.texts.lines.iter().filter(|t| !t.excluded).count(),
                    r.output
                        .shapes
                        .iter()
                        .map(vectrace::shapes::ShapeRegion::primitives)
                        .sum(),
                )
            })
    }

    /// Whether the picture's text is being read.
    #[must_use]
    pub fn is_reading(&self) -> bool {
        self.reading
    }

    /// The zoom of the picture on the canvas, if a picture is shown.
    #[must_use]
    pub fn zoom(&self) -> Option<f32> {
        self.doc.as_ref().map(|d| d.view.zoom)
    }

    /// The window title last sent to the window system.
    #[must_use]
    pub fn window_title(&self) -> Option<&str> {
        self.title_sent.as_deref()
    }

    /// The name of the shown picture's file.
    #[must_use]
    pub fn picture_name(&self) -> Option<&str> {
        self.doc.as_ref().map(|d| d.name.as_str())
    }

    /// The text of the notice slot, lines joined by a newline.
    #[must_use]
    pub fn notice_text(&self) -> Option<String> {
        self.notice.as_ref().map(|n| n.lines.join("\n"))
    }

    /// The reason a result cannot be saved, if it cannot.
    #[must_use]
    pub fn nothing_to_save(&self) -> Option<&str> {
        self.result.as_ref().and_then(|r| r.reason.as_deref())
    }

    /// What the blue button looks like now.
    pub fn button(&mut self) -> tracyo_core::progress::Button {
        let now = self.now();
        self.progress.button(now)
    }

    /// Wraps the worker command with `prefix` (tests stage a slow or a failing worker this way).
    pub fn wrap_worker(&mut self, prefix: Vec<String>) {
        self.runner.wrap_command(prefix);
    }

    /// The remembered settings as they are now.
    #[must_use]
    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    /// The values that belong to the shown picture (not remembered), if one is shown.
    #[must_use]
    pub fn picture_values(&self) -> Option<&PictureValues> {
        self.doc.as_ref().map(|d| &d.values)
    }

    /// The colours of the Multicolour palette's swatches, in the order they are shown.
    #[must_use]
    pub fn palette_colours(&self) -> Vec<vectrace::Rgb8> {
        self.palette_strip().iter().map(|p| p.colour).collect()
    }

    /// Whether a pipette is armed on the canvas.
    #[must_use]
    pub fn pipette_armed(&self) -> bool {
        self.pipette != Pipette::Off
    }

    /// Where picture pixel (`x`, `y`) is shown on the canvas, in window points.
    #[must_use]
    pub fn picture_point_on_screen(&self, x: u32, y: u32) -> Option<Pos2> {
        let doc = self.doc.as_ref()?;
        Some(pos2(
            doc.view.offset.x + x as f32 * doc.view.zoom + 0.5 * doc.view.zoom,
            doc.view.offset.y + y as f32 * doc.view.zoom + 0.5 * doc.view.zoom,
        ))
    }

    /// The bottom edge of the Trace bar as it was laid out in the last frame, in window points.
    #[must_use]
    pub fn bar_bottom(&self) -> f32 {
        self.bar_bottom
    }

    /// The elapsed time the progress rule is measured in.
    pub(crate) fn now(&self) -> Duration {
        self.start.elapsed()
    }

    // ---- opening ----

    /// Starts opening `path` on a thread.
    pub fn open_path(&mut self, ctx: &egui::Context, path: PathBuf) {
        let (tx, rx) = channel();
        let name = path.file_name().map_or_else(
            || path.display().to_string(),
            |n| n.to_string_lossy().into_owned(),
        );
        if std::fs::metadata(&path).is_ok_and(|m| m.len() >= 5_000_000) {
            self.set_notice(ctx, vec![format!("Opening {name}...")], false);
        }
        let repaint = ctx.clone();
        let dir = path.parent().map(Path::to_path_buf);
        std::thread::spawn(move || {
            let result = std::fs::read(&path)
                .map_err(|_| ReadError::Damaged)
                .and_then(|bytes| Picture::decode(&bytes))
                .map(|picture| {
                    let analysis = Analysis::of(&picture);
                    Loaded {
                        path: path.clone(),
                        name: name.clone(),
                        picture,
                        analysis,
                    }
                });
            let _ = tx.send(result.map_err(|e| (name, e)));
            repaint.request_repaint();
        });
        if let Some(dir) = dir {
            self.last_open_dir = Some(dir);
        }
        self.loading = Some(rx);
    }

    fn finish_loading(&mut self, ctx: &egui::Context) {
        let Some(rx) = &self.loading else { return };
        let Ok(result) = rx.try_recv() else { return };
        self.loading = None;
        match result {
            Ok(loaded) => self.show_picture(ctx, loaded),
            Err((name, error)) => {
                let lines = vec![shorten_name(&name), error.to_string()];
                self.set_notice(ctx, lines, true);
            }
        }
    }

    fn show_picture(&mut self, ctx: &egui::Context, loaded: Loaded) {
        self.runner.stop();
        self.reading = false;
        self.after_read = None;
        self.progress.clear();
        self.pending_save = None;
        self.failed_write = None;
        self.result = None;
        self.picture_counter += 1;
        self.notice = None;
        self.pipette = Pipette::Off;
        self.analysing = None;
        let mut values = PictureValues::new(loaded.analysis.threshold);
        loaded.analysis.apply(&mut self.settings, &mut values);
        let picture = Arc::new(loaded.picture);
        let tiles = Tiles::new(picture.width, picture.height);
        self.doc = Some(Doc {
            path: loaded.path,
            name: loaded.name,
            picture,
            id: self.picture_counter,
            tiles,
            values,
            analysis: loaded.analysis,
            view: View {
                zoom: 1.0,
                offset: vec2(0.0, 0.0),
            },
            auto_fit: None,
            text: TextState::Unread,
        });
        self.fit_next_frame = true;
        self.start_trace(ctx, false, false);
    }

    pub(crate) fn set_notice(&mut self, ctx: &egui::Context, lines: Vec<String>, alert: bool) {
        let chars: usize = lines.iter().map(|l| l.chars().count()).sum();
        let seconds = if alert {
            8.0
        } else if chars > 70 {
            5.0
        } else {
            3.0
        };
        self.notice = Some(Notice {
            lines,
            alert,
            until: ctx.input(|i| i.time) + seconds,
        });
        ctx.request_repaint_after_secs(seconds as f32 + 0.05);
    }

    /// Opens the Open dialog.
    pub(crate) fn ask_open(&mut self, ctx: &egui::Context) {
        match self.dialogs.open(self.last_open_dir.as_deref()) {
            Choice::Picked(path) => self.open_path(ctx, path),
            Choice::Cancelled => {}
            Choice::Unavailable => self.no_dialog(ctx),
        }
    }

    /// The notice for a system without a file dialog.
    fn no_dialog(&mut self, ctx: &egui::Context) {
        self.set_notice(
            ctx,
            NO_DIALOG.iter().map(|l| (*l).to_owned()).collect(),
            true,
        );
    }

    // ---- tracing ----

    /// Starts a trace of the picture with the current values. `dragging` traces the reduced copy;
    /// `for_saving` traces the full picture and shows the progress at once. With Recognise text on
    /// and the picture's text not read yet, the text is read first and the trace follows it.
    pub(crate) fn start_trace(&mut self, ctx: &egui::Context, dragging: bool, for_saving: bool) {
        let Some(doc) = &self.doc else { return };
        if doc.values.palette_deleted(&self.settings) {
            self.trace_nothing(ctx);
            return;
        }
        let needs_read = self.settings.recognise_text && matches!(doc.text, TextState::Unread);
        if self.reading && needs_read {
            // The read goes on; what is traced after it is what was asked last.
            let saved = self.after_read.is_some_and(|(_, saving)| saving);
            self.after_read = Some((dragging, for_saving || saved));
            if let Some(result) = &mut self.result {
                result.stale = true;
            }
            return;
        }
        if self.reading {
            // Recognise text was switched off while the text was read.
            self.runner.stop();
            self.reading = false;
            self.after_read = None;
        }
        if needs_read {
            self.start_read(ctx, dragging, for_saving);
        } else {
            self.dispatch_trace(ctx, dragging, for_saving, false);
        }
    }

    fn start_read(&mut self, ctx: &egui::Context, dragging: bool, for_saving: bool) {
        let Some(doc) = &self.doc else { return };
        self.job_counter += 1;
        let now = self.now();
        match self
            .runner
            .start_read(self.job_counter, doc.id, &doc.picture)
        {
            Ok(()) => {
                self.reading = true;
                self.after_read = Some((dragging, for_saving));
                self.progress.start(now, for_saving);
            }
            Err(_) => self.progress.fail(failure_text("internal error")),
        }
        if let Some(result) = &mut self.result {
            result.stale = true;
        }
        ctx.request_repaint();
    }

    /// Sends the trace request of the current values; `continuing` is the second part of a job
    /// whose first part was the read, so its progress goes on instead of starting again.
    fn dispatch_trace(
        &mut self,
        ctx: &egui::Context,
        dragging: bool,
        for_saving: bool,
        continuing: bool,
    ) {
        let Some(doc) = &self.doc else { return };
        let reduced = request::use_reduced(&doc.picture, dragging, for_saving);
        let lines = match &doc.text {
            TextState::Read(lines) if self.settings.recognise_text => lines.clone(),
            _ => Vec::new(),
        };
        let req = TraceRequest {
            threshold: doc.values.threshold,
            settings: self.settings,
            selection: doc
                .values
                .selection(&self.settings, doc.analysis.suggested_background),
            invert: doc.values.invert,
            drop_border: doc.values.drop_border,
            colours: doc.values.colour_spec(&self.settings),
            reduced,
            shapes: self.settings.recognise_shapes,
            lines,
        };
        self.last_request_reduced = reduced;
        self.job_counter += 1;
        self.trace_after_read = continuing;
        let now = self.now();
        match self
            .runner
            .start(self.job_counter, doc.id, &doc.picture, req)
        {
            Ok(()) if continuing => {}
            Ok(()) => self.progress.start(now, for_saving),
            Err(_) => self.progress.fail(failure_text("internal error")),
        }
        if let Some(result) = &mut self.result {
            result.stale = true;
        }
        ctx.request_repaint();
    }

    /// The read ended (with lines or a failed reader): the trace of the same job follows.
    fn text_read(&mut self, ctx: &egui::Context, text: TextState) {
        self.reading = false;
        if text == TextState::Failed {
            self.set_notice(ctx, vec![TEXT_FAILED_NOTICE.to_owned()], false);
        }
        if let Some(doc) = &mut self.doc {
            doc.text = text;
        }
        let (dragging, for_saving) = self.after_read.take().unwrap_or((false, false));
        self.dispatch_trace(ctx, dragging, for_saving, true);
    }

    /// Every colour of the palette was deleted: nothing is traced, the result is empty at once.
    fn trace_nothing(&mut self, ctx: &egui::Context) {
        self.runner.stop();
        self.reading = false;
        self.after_read = None;
        self.job_counter += 1;
        self.progress.clear();
        self.pending_save = None;
        self.last_request_reduced = false;
        self.finish_trace(ctx, &tracyo_core::job::empty_colour_wire());
    }

    pub(crate) fn stop_trace(&mut self) {
        self.runner.stop();
        self.reading = false;
        self.after_read = None;
        self.progress.stop();
        self.pending_save = None;
        if let Some(result) = &mut self.result {
            result.stale = true;
        }
    }

    fn handle_events(&mut self, ctx: &egui::Context) {
        for event in self.runner.poll() {
            match event {
                Event::Percent { job, percent } if job == self.job_counter => {
                    let stage = if self.reading {
                        Stage::Read
                    } else if self.trace_after_read {
                        Stage::TraceAfterRead
                    } else {
                        Stage::Trace
                    };
                    self.progress
                        .set_percent(stages::job_percent(stage, percent));
                }
                Event::Lines { job, lines } if job == self.job_counter && self.reading => {
                    self.text_read(ctx, TextState::Read(lines));
                }
                Event::TextFailed { job } if job == self.job_counter && self.reading => {
                    self.text_read(ctx, TextState::Failed);
                }
                Event::Wire { job, numbers } if job == self.job_counter => {
                    self.finish_trace(ctx, &numbers);
                }
                Event::Failed { job, reason } if job == self.job_counter => {
                    self.reading = false;
                    self.after_read = None;
                    self.pending_save = None;
                    self.progress.fail(failure_text(&reason));
                }
                _ => {}
            }
        }
    }

    fn finish_trace(&mut self, ctx: &egui::Context, numbers: &[f64]) {
        let Some(doc) = &self.doc else { return };
        let Some(output) = vectrace::wire::read(numbers) else {
            self.pending_save = None;
            self.progress.fail(failure_text("internal error"));
            return;
        };
        let reason = progress::nothing_to_save(
            &output,
            &SaveCase {
                picture_pixels: doc.picture.pixels(),
                method: Method::from_index(self.settings.method),
                picked: doc.values.swatches.len(),
                multicolour: self.settings.multicolour,
                palette_deleted: doc.values.palette_deleted(&self.settings),
            },
        );
        let mut drawn = shapes::plan(&output).drawn_outlines(&output);
        drawn.extend(
            output
                .texts
                .lines
                .iter()
                .filter(|t| !t.excluded)
                .flat_map(|t| t.outlines.iter().cloned()),
        );
        let preview = Preview::new(&drawn);
        let now = self.now();
        self.progress.finish(now);
        self.result = Some(TraceResult {
            preview,
            output,
            reduced: self.last_request_reduced,
            stale: false,
            reason,
        });
        if let Some(path) = self.pending_save.take()
            && !self.last_request_reduced
        {
            self.write_svg(ctx, &path);
        }
    }

    // ---- saving ----

    /// What Save as… does: asks for a file, traces the full picture first when needed, writes.
    pub(crate) fn save_action(&mut self, ctx: &egui::Context) {
        let Some(doc) = &self.doc else { return };
        let Some(result) = &self.result else { return };
        if result.reason.is_some() || result.stale || self.progress.is_running() {
            return;
        }
        let name = format!(
            "{}.svg",
            Path::new(&doc.name)
                .file_stem()
                .map_or_else(|| doc.name.clone(), |s| s.to_string_lossy().into_owned())
        );
        let path = match self.dialogs.save(doc.path.parent(), &name) {
            Choice::Picked(path) => path,
            Choice::Cancelled => return,
            Choice::Unavailable => {
                self.no_dialog(ctx);
                return;
            }
        };
        if result.reduced {
            self.pending_save = Some(path);
            self.start_trace(ctx, false, true);
        } else {
            self.write_svg(ctx, &path);
        }
    }

    fn write_svg(&mut self, ctx: &egui::Context, path: &Path) {
        let (Some(doc), Some(result)) = (&self.doc, &self.result) else {
            return;
        };
        let text = svg::write(&svg::Document {
            width_px: doc.picture.width,
            height_px: doc.picture.height,
            layers: &svg::layers_of(&result.output),
        });
        match config::write_atomic(path, text.as_bytes()) {
            Ok(()) => {
                self.failed_write = None;
            }
            Err(e) => {
                let name = path
                    .file_name()
                    .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
                self.progress.fail(format!(
                    "Could not save {name}: {}. Nothing was written.",
                    config::reason(&e)
                ));
                self.failed_write = Some((path.to_path_buf(), text.into_bytes()));
            }
        }
        ctx.request_repaint();
    }

    /// Retry: writes the file again after a write failure, else traces again.
    pub(crate) fn retry(&mut self, ctx: &egui::Context) {
        if let Some((path, bytes)) = self.failed_write.take() {
            self.progress.clear();
            match config::write_atomic(&path, &bytes) {
                Ok(()) => {}
                Err(e) => {
                    let name = path
                        .file_name()
                        .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
                    self.progress.fail(format!(
                        "Could not save {name}: {}. Nothing was written.",
                        config::reason(&e)
                    ));
                    self.failed_write = Some((path, bytes));
                }
            }
        } else {
            self.start_trace(ctx, false, false);
        }
    }

    // ---- values ----

    /// A value of the bar changed (to a settled or a dragged value).
    pub(crate) fn values_changed(&mut self, ctx: &egui::Context, dragging: bool) {
        self.dragging_value = dragging;
        if !dragging {
            self.config.save(&self.settings);
        }
        self.start_trace(ctx, dragging, false);
    }

    // ---- frame ----

    fn begin_frame(&mut self, ctx: &egui::Context) {
        let (key_event, pressed, dropped, size) = ctx.input(|i| {
            let key = i
                .events
                .iter()
                .any(|e| matches!(e, egui::Event::Key { pressed: true, .. }));
            (
                key,
                i.pointer.any_pressed(),
                i.raw.dropped_files.clone(),
                i.content_rect().size(),
            )
        });
        if key_event {
            set_focus_visible(ctx, true);
        }
        if pressed {
            set_focus_visible(ctx, false);
        }
        self.window_size = Some([size.x, size.y]);
        if let Some(file) = dropped.first().map(|f| f.path().to_path_buf()) {
            self.open_path(ctx, file);
        }
        self.finish_loading(ctx);
        self.handle_events(ctx);
        self.finish_analysing(ctx);
        if let Some(n) = &self.notice
            && ctx.input(|i| i.time) >= n.until
        {
            self.notice = None;
        }
        self.keys(ctx);
        let title = self.doc.as_ref().map_or_else(
            || "Tracyo".to_owned(),
            |d| format!("{} \u{2014} Tracyo", d.name),
        );
        if self.title_sent.as_deref() != Some(&title) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.title_sent = Some(title);
        }
        if self.loading.is_some() {
            ctx.set_cursor_icon(egui::CursorIcon::Wait);
        }
    }

    fn keys(&mut self, ctx: &egui::Context) {
        let entry_open = self.fields.iter().any(FieldState::entry_open);
        let esc = ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape));
        if esc {
            let state = EscState {
                running: self.progress.is_running(),
                pipette_armed: self.pipette != Pipette::Off,
                menu_open: self.menu.is_some(),
                entry_open,
            };
            match progress::esc(state) {
                Esc::StopTrace => self.stop_trace(),
                Esc::DisarmPipette => self.pipette = Pipette::Off,
                Esc::CloseMenu => self.close_menu(ctx),
                Esc::CloseEntry => self.fields.iter_mut().for_each(FieldState::cancel_entry),
                Esc::FocusCanvas => ctx.memory_mut(|m| {
                    if let Some(id) = m.focused() {
                        m.surrender_focus(id);
                    }
                }),
            }
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::O)) {
            self.ask_open(ctx);
        }
        let save = ctx.input_mut(|i| {
            i.consume_key(Modifiers::COMMAND | Modifiers::SHIFT, Key::S)
                || i.consume_key(Modifiers::COMMAND, Key::S)
        });
        if save {
            self.save_action(ctx);
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::Num0)) {
            self.fit_next_frame = true;
            self.fit_zoom_enlarge = true;
        }
        let canvas_has_keys =
            ctx.memory(|m| m.focused().is_none()) && !entry_open && self.menu.is_none();
        if canvas_has_keys && ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Enter)) {
            self.enter_action(ctx);
        }
    }

    /// Closes the open menu; the keyboard returns to the button that opened it.
    pub(crate) fn close_menu(&mut self, ctx: &egui::Context) {
        if let Some(kind) = self.menu.take() {
            let name = match kind {
                MenuKind::Preset => "preset",
                MenuKind::Method => "method",
            };
            ctx.memory_mut(|m| m.request_focus(crate::bar::bar_id(name)));
        }
    }

    /// What Enter (or a double-click on the picture) does.
    pub(crate) fn enter_action(&mut self, ctx: &egui::Context) {
        if self.progress.is_stopped_or_failed() {
            self.retry(ctx);
        } else {
            self.save_action(ctx);
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.begin_frame(&ctx);
        let screen = ui.max_rect();
        ui.painter().rect_filled(screen, 0.0, theme::CANVAS_BG);
        let docked = if self.about_open {
            crate::about::PANEL_WIDTH
        } else {
            0.0
        };
        let canvas = Rect::from_min_max(
            screen.min,
            pos2(screen.max.x - docked, screen.max.y - STATUS_HEIGHT),
        );
        // The bar first: the picture is fitted below the bar's height of this very frame.
        self.open_card(&ctx);
        self.bar(&ctx, canvas);
        self.canvas(ui, canvas);
        self.status_bar(ui, screen);
        if self.about_open {
            let panel = Rect::from_min_max(
                pos2(canvas.right(), screen.top()),
                pos2(screen.max.x, canvas.bottom()),
            );
            self.about_panel(&ctx, panel);
        }
        self.menus(&ctx);
        self.notice_slot(&ctx, canvas);
        if let Some(wake) = self.progress.wake_in(self.now()) {
            ctx.request_repaint_after(wake + Duration::from_millis(5));
        }
    }

    fn on_exit(&mut self) {
        if let Some(size) = self.window_size {
            self.settings.window = Some(size);
            self.config.save(&self.settings);
        }
    }
}

/// What the notice says when the system has no file dialog (Linux without a portal).
const NO_DIALOG: [&str; 2] = [
    "There is no file dialog on this system.",
    "Install xdg-desktop-portal with a backend such as xdg-desktop-portal-gtk.",
];

/// The failure tooltip for a trace that did not finish.
pub(crate) fn failure_text(reason: &str) -> String {
    format!("Trace failed: {reason}. Nothing was saved.")
}

/// The file name cut to 24 characters with an ellipsis in the middle, extension kept.
#[must_use]
pub fn shorten_name(name: &str) -> String {
    let chars: Vec<char> = name.chars().collect();
    if chars.len() <= 24 {
        return name.to_owned();
    }
    let ext_len = name
        .rfind('.')
        .map_or(0, |i| name[i..].chars().count())
        .min(8);
    let tail = ext_len + 4;
    let head = 24 - tail - 1;
    let start: String = chars[..head].iter().collect();
    let end: String = chars[chars.len() - tail..].iter().collect();
    format!("{start}\u{2026}{end}")
}

pub(crate) fn dashed_frame(ui: &Ui, rect: Rect) {
    let r = rect.shrink(1.0);
    let points = [
        r.left_top(),
        r.right_top(),
        r.right_bottom(),
        r.left_bottom(),
        r.left_top(),
    ];
    ui.painter().extend(Shape::dashed_line(
        &points,
        Stroke::new(2.0, theme::ACCENT),
        6.0,
        6.0,
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_names_stay() {
        assert_eq!(shorten_name("logo.png"), "logo.png");
    }

    #[test]
    fn long_names_are_cut_in_the_middle_and_keep_the_extension() {
        let cut = shorten_name("a-very-long-file-name-of-a-scan-from-last-summer.jpeg");
        assert_eq!(cut.chars().count(), 24);
        assert!(cut.to_lowercase().ends_with(".jpeg"));
        assert!(cut.contains('\u{2026}'));
    }
}
