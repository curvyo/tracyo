//! Tracyo's window, trace worker, dialogs and settings file.

// Pixel and point geometry converts between integer sizes and `f32` everywhere; the values are
// picture sizes (at most a few thousand) and screen coordinates, far inside `f32`'s exact range.
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
// Painting a control is one straight list of shapes; splitting it would only scatter the layout.
#![allow(
    clippy::too_many_lines,
    clippy::too_many_arguments,
    clippy::similar_names
)]

mod advanced;
mod app;
mod bar;
mod canvas;
pub mod config;
mod controls;
pub mod dialogs;
pub mod fonts;
mod groups;
mod icons;
mod methods;
mod preview;
pub mod runner;
mod stage;
pub mod theme;
mod widgets;
pub mod worker;

use std::path::PathBuf;

pub use app::{App, Options};

/// The window title and size of a first start.
const FIRST_SIZE: [f32; 2] = [1280.0, 800.0];

/// Starts the window; `open` is the picture of the first command-line argument.
///
/// # Errors
/// The graphics or window system's error when the window cannot be created.
pub fn run(open: Option<PathBuf>) -> eframe::Result {
    let config = config::Config::standard();
    let size = config.load().window.unwrap_or(FIRST_SIZE);
    let mut native = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("Tracyo")
            .with_inner_size(size)
            .with_min_inner_size([800.0, 600.0])
            .with_drag_and_drop(true),
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };
    prefer_x11(&mut native);
    eframe::run_native(
        "tracyo",
        native,
        Box::new(move |cc| {
            let exe = std::env::current_exe().map_err(|e| e.to_string())?;
            let options = Options {
                exe,
                dialogs: dialogs::Dialogs::System,
                config,
                fonts: fonts::Source::System,
                open,
            };
            Ok(Box::new(App::new(&cc.egui_ctx, options)))
        }),
    )
}

/// On Linux the X11 backend is used when `DISPLAY` is set: winit delivers dropped files there
/// (through `XWayland` on a Wayland desktop) and not on Wayland.
#[cfg(target_os = "linux")]
fn prefer_x11(native: &mut eframe::NativeOptions) {
    if std::env::var_os("DISPLAY").is_some() {
        native.event_loop_builder = Some(Box::new(|builder| {
            use winit::platform::x11::EventLoopBuilderExtX11;
            builder.with_x11();
        }));
    }
}

#[cfg(not(target_os = "linux"))]
fn prefer_x11(_native: &mut eframe::NativeOptions) {}
