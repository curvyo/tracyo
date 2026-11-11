//! Pixel comparison of the app's states with the references of `specs/0001-trace-and-save`.

mod common;

use common::compare;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use tracyo_app::App;
use tracyo_app::dialogs::Dialogs;
use tracyo_core::progress::Button;

fn reference(name: &str) -> image::RgbImage {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../specs/0001-trace-and-save/reference")
        .join(name);
    image::open(path).expect("reference picture").to_rgb8()
}

/// Compares the `width` x `height` bar of `rendered` with the reference `name`.
fn check(name: &str, rendered: &image::RgbaImage, width: u32, height: u32) {
    check_at(name, rendered, (0, 0), width, height);
}

/// As `check` for a region `(dx, dy)` from the bar's top-left corner.
fn check_at(name: &str, rendered: &image::RgbaImage, offset: (u32, u32), width: u32, height: u32) {
    let out = compare::out_dir();
    // The Open card (x 12 to 60) is Tracyo's own rail element; the references have none, so the
    // picture starts right of it and the bar is found by its ground.
    let full = compare::rgb(rendered);
    let got = image::imageops::crop_imm(&full, 60, 0, full.width() - 60, full.height()).to_image();
    got.save(out.join(format!("render-{name}.png")))
        .expect("save render");
    let outcome = compare::compare_at(
        &reference(&format!("{name}.png")),
        &got,
        offset,
        width,
        height,
    );
    outcome
        .diff
        .save(out.join(format!("diff-{name}.png")))
        .expect("save diff");
    println!("{name}: {}", outcome.summary());
    assert!(outcome.passed(), "{name}: {}", outcome.summary());
}

fn fixed() -> Dialogs {
    Dialogs::Fixed {
        open: None,
        save: None,
    }
}

fn opened<'a>(dir: &std::path::Path) -> Harness<'a, App> {
    let mut h = common::harness(dir, Some(common::logo()), fixed());
    common::wait_until(&mut h, 60.0, App::has_result);
    h.run_steps(3);
    h
}

/// A worker that sends 42 % and then waits.
fn staged_running() -> Vec<String> {
    vec![
        "/bin/sh".into(),
        "-c".into(),
        "printf '\\001\\001\\000\\000\\000*'; exec sleep 600".into(),
    ]
}

/// Presses the Smooth field at 81 % and moves the pointer away: Smooth is 81, the preset Custom.
fn smooth_81(h: &mut Harness<'_, App>) {
    use eframe::egui::{Event, Modifiers, PointerButton, pos2};
    let at = pos2(598.0 + 0.81 * 150.0, 30.0);
    h.event(Event::PointerMoved(at));
    h.run_steps(1);
    h.event(Event::PointerButton {
        pos: at,
        button: PointerButton::Primary,
        pressed: true,
        modifiers: Modifiers::NONE,
    });
    h.run_steps(1);
    h.event(Event::PointerButton {
        pos: at,
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::NONE,
    });
    h.run_steps(1);
    h.remove_cursor();
}

#[test]
fn px_00_bar_closed() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = opened(dir.path());
    check("px-00-bar-closed", &h.render().unwrap(), 1154, 36);
}

#[test]
fn px_01_empty() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = common::harness(dir.path(), None, fixed());
    h.run_steps(3);
    check("px-01-empty", &h.render().unwrap(), 165, 36);
}

#[test]
fn px_02_preview() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = opened(dir.path());
    check("px-02-preview", &h.render().unwrap(), 1154, 36);
}

#[test]
fn px_05_running_42() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = opened(dir.path());
    h.state_mut().wrap_worker(staged_running());
    smooth_81(&mut h);
    let start = std::time::Instant::now();
    while !matches!(h.state_mut().button(), Button::Running(Some(42))) {
        h.step();
        std::thread::sleep(std::time::Duration::from_millis(20));
        assert!(start.elapsed().as_secs() < 20, "never showed 42 %");
    }
    h.run_steps(2);
    // Left of the expander: Custom and Smooth 81. The blue button is not compared: the reference
    // shows it 75 px wide, ux-notes section 3.3 keeps it at 89 px while running.
    check("px-05-running-42", &h.render().unwrap(), 1020, 36);
    assert!(matches!(h.state_mut().button(), Button::Running(Some(42))));
}

#[test]
fn px_08_preset_menu() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = opened(dir.path());
    h.get_by_label("Preset").click();
    h.run_steps(3);
    h.remove_cursor();
    h.run_steps(2);
    // The menu: 44 px right of the bar's left edge, 4 px below it, 238 x 184.
    check_at(
        "px-08-preset-menu",
        &h.render().unwrap(),
        (44, 40),
        238,
        184,
    );
}
