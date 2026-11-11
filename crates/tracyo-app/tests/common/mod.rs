//! Shared helpers of the app tests: a harness around the real app and the real worker.

#![allow(dead_code, clippy::unwrap_used, clippy::expect_used)]

use std::path::{Path, PathBuf};

use egui_kittest::Harness;
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
    let config = Config::at(Some(dir.join("settings.toml")));
    let exe = PathBuf::from(env!("CARGO_BIN_EXE_tracyo"));
    let mut harness = Harness::builder()
        .with_size(eframe::egui::vec2(1280.0, 800.0))
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
