//! Shared helpers of the app tests: a harness around the real app and the real worker.

#![allow(dead_code, clippy::unwrap_used, clippy::expect_used)]

use std::path::{Path, PathBuf};

use eframe::egui::{Event, Modifiers, PointerButton};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use tracyo_app::config::Config;
use tracyo_app::dialogs::Dialogs;
use tracyo_app::fonts::Source;
use tracyo_app::{App, Options};

/// The logo picture of the reference set.
pub fn logo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../specs/0001-trace-and-save/reference/input-logo.png")
}

/// A harness of 1280 x 800 around the app, with the embedded font and a settings file in `dir`.
pub fn harness<'a>(dir: &Path, open: Option<PathBuf>, dialogs: Dialogs) -> Harness<'a, App> {
    harness_sized(dir, open, dialogs, 1280.0)
}

/// As [`harness`] with a window `width` points wide.
pub fn harness_sized<'a>(
    dir: &Path,
    open: Option<PathBuf>,
    dialogs: Dialogs,
    width: f32,
) -> Harness<'a, App> {
    harness_stepped(dir, open, dialogs, width, 0.25)
}

/// As [`harness_sized`] with `step_dt` seconds of simulated time per frame (the default of 0.25 s
/// keeps two clicks of a test apart; a double click needs a smaller one).
pub fn harness_stepped<'a>(
    dir: &Path,
    open: Option<PathBuf>,
    dialogs: Dialogs,
    width: f32,
    step_dt: f32,
) -> Harness<'a, App> {
    let config = Config::at(Some(dir.join("settings.toml")));
    let exe = PathBuf::from(env!("CARGO_BIN_EXE_tracyo"));
    let mut harness = Harness::builder()
        .with_size(eframe::egui::vec2(width, 800.0))
        .with_step_dt(step_dt)
        .wgpu()
        .build_eframe(move |cc| {
            App::new(
                &cc.egui_ctx,
                Options {
                    exe,
                    dialogs,
                    config,
                    fonts: Source::Embedded,
                    open,
                },
            )
        });
    harness.set_pixels_per_point(1.0);
    harness
}

/// Steps frames until `done` or panics after `limit` seconds of real time.
pub fn wait_until(harness: &mut Harness<'_, App>, limit: f32, mut done: impl FnMut(&App) -> bool) {
    let start = std::time::Instant::now();
    loop {
        harness.step();
        if done(harness.state()) {
            return;
        }
        assert!(start.elapsed().as_secs_f32() < limit, "timed out");
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
}

/// Steps frames until a settled result is there and the blue button offers Save as…. A result
/// alone is not enough: a running look that was shown stays up for 300 ms after the job ends.
pub fn wait_for_result(harness: &mut Harness<'_, App>, limit: f32) {
    wait_for_result_where(harness, limit, |_| true);
}

/// As [`wait_for_result`], and `also` holds for the app.
pub fn wait_for_result_where(
    harness: &mut Harness<'_, App>,
    limit: f32,
    also: impl Fn(&App) -> bool,
) {
    use egui_kittest::kittest::Queryable;
    let start = std::time::Instant::now();
    loop {
        harness.step();
        if harness.state().has_result()
            && also(harness.state())
            && harness.query_by_label("Save as\u{2026}").is_some()
        {
            return;
        }
        assert!(start.elapsed().as_secs_f32() < limit, "timed out");
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
}

/// A Ctrl+click (Cmd on macOS) on the control named `label`, its press and release in frames of
/// their own as a mouse makes them (a queued press and release share one frame and its end
/// state of the modifiers).
pub fn ctrl_click(harness: &mut Harness<'_, App>, label: &str) {
    let at = harness.get_by_label(label).rect().center();
    harness.event(Event::PointerMoved(at));
    harness.run_steps(2);
    harness.event(Event::ModifiersChanged(Modifiers::COMMAND));
    for pressed in [true, false] {
        harness.event(Event::PointerButton {
            pos: at,
            button: PointerButton::Primary,
            pressed,
            modifiers: Modifiers::COMMAND,
        });
        harness.run_steps(2);
    }
    harness.event(Event::ModifiersChanged(Modifiers::NONE));
    harness.run_steps(1);
}
