//! Pixel comparison of the app's states with the references of `specs/0001-trace-and-save`.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;
#[path = "common/compare.rs"]
mod compare;
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

/// One rectangle of a comparison: `size` pixels at `reference_at` from the reference bar's corner
/// and at `got_at` from the render's.
struct Region {
    name: &'static str,
    reference_at: (u32, u32),
    got_at: (u32, u32),
    size: (u32, u32),
}

impl Region {
    /// A region at the same place in both pictures.
    fn same(name: &'static str, at: (u32, u32), size: (u32, u32)) -> Self {
        Self {
            name,
            reference_at: at,
            got_at: at,
            size,
        }
    }
}

/// Compares every region and asserts that all pass; each has its own numbers and diff picture.
fn check_regions(name: &str, rendered: &image::RgbaImage, regions: &[Region]) {
    let out = compare::out_dir();
    let full = compare::rgb(rendered);
    let got = image::imageops::crop_imm(&full, 60, 0, full.width() - 60, full.height()).to_image();
    got.save(out.join(format!("render-{name}.png")))
        .expect("save render");
    let reference = reference(&format!("{name}.png"));
    let mut failed = Vec::new();
    for r in regions {
        let outcome = compare::compare_between(
            &reference,
            &got,
            r.reference_at,
            r.got_at,
            r.size.0,
            r.size.1,
        );
        outcome
            .diff
            .save(out.join(format!("diff-{name}-{}.png", r.name)))
            .expect("save diff");
        println!("{name} {}: {}", r.name, outcome.summary());
        if !outcome.passed() {
            failed.push(r.name);
        }
    }
    assert!(failed.is_empty(), "{name}: failed regions {failed:?}");
}

/// Where the Denoise cell starts in the render when no Output pill and no Recognise toggles
/// stand before it: the block's 8 px padding, two toggles and a gap.
const DENOISE_AT: u32 = 8 + 60 + 8;
/// Denoise, Upscale and Corners with their gaps.
const SETTINGS_W: u32 = 185 + 8 + 150 + 8 + 150;

/// The regions of the Advanced block for a bar `bar_w` wide with `rows` rows above the block.
/// The reference block has controls that do not exist yet (the Output pill, with Multicolour its
/// Colours field and palette, and the two Recognise toggles), so Denoise, Upscale and Corners sit
/// `shift` px further right there; the regions compare each control that exists where it is.
fn block_regions(bar_w: u32, rows: u32, shift: u32, with_toggles: bool) -> Vec<Region> {
    let top = rows * 36;
    let controls = top + 9;
    let mut regions = vec![Region::same("divider", (0, top), (bar_w, 13))];
    if with_toggles {
        regions.push(Region {
            name: "toggles",
            reference_at: (185, controls),
            got_at: (8, controls),
            size: (60, 36),
        });
    }
    regions.push(Region {
        name: "settings",
        reference_at: (DENOISE_AT + shift, controls),
        got_at: (DENOISE_AT, controls),
        size: (SETTINGS_W, 36),
    });
    let analyse = bar_w - 91;
    regions.push(Region {
        name: "gap",
        reference_at: (DENOISE_AT + SETTINGS_W + shift, controls),
        got_at: (DENOISE_AT + SETTINGS_W, controls),
        size: (analyse - (DENOISE_AT + SETTINGS_W + shift), 36),
    });
    regions.push(Region::same("analyse", (analyse, controls), (79, 36)));
    regions
}

/// Denoise's shift in the references that have Multicolour with its Colours field and palette.
const MULTICOLOUR_SHIFT: u32 = 469;
/// The same in px-03, which has Single colour: the Output pill and the four toggles.
const SINGLE_SHIFT: u32 = 241;

fn fixed() -> Dialogs {
    Dialogs::Fixed {
        open: None,
        save: None,
    }
}

/// The logo opened with the analysis's values (Remove background for this picture), as the
/// reference script sees it before it chooses a method.
fn analysed<'a>(dir: &std::path::Path, width: f32) -> Harness<'a, App> {
    let mut h = common::harness_sized(dir, Some(common::logo()), fixed(), width);
    common::wait_until(&mut h, 60.0, App::has_result);
    h.run_steps(3);
    h
}

/// Chooses `method` in the Method menu and waits for the new preview.
fn choose_method(h: &mut Harness<'_, App>, method: &str) {
    h.get_by_label("Method").click();
    h.run_steps(3);
    h.get_by_label(method).click();
    h.run_steps(3);
    common::wait_until(h, 60.0, App::has_result);
    h.remove_cursor();
    h.run_steps(3);
}

/// The logo with Brightness chosen, as the reference script leaves it for most states.
fn opened<'a>(dir: &std::path::Path) -> Harness<'a, App> {
    let mut h = analysed(dir, 1280.0);
    choose_method(&mut h, "Brightness");
    h
}

/// Opens the Advanced block with the expander.
fn open_advanced(h: &mut Harness<'_, App>) {
    h.get_by_label("More trace options").click();
    h.run_steps(3);
    h.remove_cursor();
    h.run_steps(3);
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
fn px_03_advanced_open() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = opened(dir.path());
    open_advanced(&mut h);
    let mut regions = vec![Region::same("row", (0, 0), (1162, 36))];
    regions.extend(block_regions(1162, 1, SINGLE_SHIFT, true));
    check_regions("px-03-advanced-open", &h.render().unwrap(), &regions);
}

#[test]
fn px_05_running_42() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = opened(dir.path());
    open_advanced(&mut h);
    h.state_mut().wrap_worker(staged_running());
    smooth_81(&mut h);
    let start = std::time::Instant::now();
    while !matches!(h.state_mut().button(), Button::Running(Some(42))) {
        h.step();
        std::thread::sleep(std::time::Duration::from_millis(20));
        assert!(start.elapsed().as_secs() < 20, "never showed 42 %");
    }
    h.run_steps(2);
    // The whole first row, the blue button 75 px wide as in the reference.
    let mut regions = vec![Region::same("row", (0, 0), (1148, 36))];
    regions.extend(block_regions(1148, 1, MULTICOLOUR_SHIFT, false));
    check_regions("px-05-running-42", &h.render().unwrap(), &regions);
    assert!(matches!(h.state_mut().button(), Button::Running(Some(42))));
}

/// Smooth 81, Advanced open, in a window whose bar is capped at the reference's 1164 px.
fn custom_with_block<'a>(dir: &std::path::Path) -> Harness<'a, App> {
    let mut h = analysed(dir, 1248.0);
    choose_method(&mut h, "Brightness");
    open_advanced(&mut h);
    h
}

fn stopped_state(h: &mut Harness<'_, App>) {
    h.state_mut().wrap_worker(staged_running());
    smooth_81(h);
    let start = std::time::Instant::now();
    while !matches!(h.state_mut().button(), Button::Running(Some(42))) {
        h.step();
        std::thread::sleep(std::time::Duration::from_millis(20));
        assert!(start.elapsed().as_secs() < 20, "never showed 42 %");
    }
    h.run_steps(2);
    h.get_by_label("Stop (Esc)").click();
    h.run_steps(3);
    h.remove_cursor();
    h.run_steps(3);
    assert!(matches!(h.state_mut().button(), Button::Stopped));
}

#[test]
fn px_07_stopped() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = custom_with_block(dir.path());
    stopped_state(&mut h);
    // Both rows: the expander and "Stopped" with Retry wrap to the second row at the cap.
    let mut regions = vec![Region::same("rows", (0, 0), (1164, 72))];
    regions.extend(block_regions(1164, 2, MULTICOLOUR_SHIFT, false));
    check_regions("px-07-stopped", &h.render().unwrap(), &regions);
}

#[test]
fn px_06_failed() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = custom_with_block(dir.path());
    stopped_state(&mut h);
    h.state_mut().wrap_worker(vec![
        "/bin/sh".into(),
        "-c".into(),
        "echo 'memory allocation of 9 bytes failed' >&2; exit 1".into(),
    ]);
    h.get_by_label("Retry").click();
    let start = std::time::Instant::now();
    while !matches!(h.state_mut().button(), Button::Failed(_)) {
        h.step();
        std::thread::sleep(std::time::Duration::from_millis(20));
        assert!(start.elapsed().as_secs() < 20, "never failed");
    }
    h.remove_cursor();
    h.run_steps(3);
    let mut regions = vec![Region::same("rows", (0, 0), (1164, 72))];
    regions.extend(block_regions(1164, 2, MULTICOLOUR_SHIFT, false));
    check_regions("px-06-failed", &h.render().unwrap(), &regions);
}

#[test]
fn px_13_trace_button_focus() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = opened(dir.path());
    open_advanced(&mut h);
    smooth_81(&mut h);
    common::wait_until(&mut h, 60.0, App::has_result);
    h.get_by_label("Fewer trace options").focus();
    h.run_steps(2);
    h.key_press(eframe::egui::Key::Tab);
    h.run_steps(3);
    h.remove_cursor();
    h.run_steps(2);
    let mut regions = vec![Region::same("row", (0, 0), (1162, 36))];
    regions.extend(block_regions(1162, 1, MULTICOLOUR_SHIFT, false));
    check_regions("px-13-trace-button-focus", &h.render().unwrap(), &regions);
}

#[test]
fn px_09_method_menu() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = analysed(dir.path(), 1248.0);
    h.get_by_label("Method").click();
    h.run_steps(3);
    h.remove_cursor();
    h.run_steps(3);
    // Both rows of the Remove background bar (capped at 1164 px) and the menu below them.
    let regions = [
        Region::same("rows", (0, 0), (1164, 72)),
        Region::same("menu", (184, 72), (232, 108)),
    ];
    check_regions("px-09-method-menu", &h.render().unwrap(), &regions);
}

#[test]
fn px_12_value_field_tooltip() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = opened(dir.path());
    // As the reference script: open the Preset menu, close it with Esc (the button keeps the
    // keyboard focus ring), then rest the pointer on Smooth until its tooltip shows.
    h.get_by_label("Preset").click();
    h.run_steps(3);
    h.key_press(eframe::egui::Key::Escape);
    h.run_steps(3);
    h.get_by_label("Smooth").hover();
    h.run_steps(30);
    h.set_render_cursor(false);
    h.run_steps(1);
    check("px-12-value-field-tooltip", &h.render().unwrap(), 1154, 36);
}

#[test]
fn px_14_preset_hover() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = opened(dir.path());
    open_advanced(&mut h);
    smooth_81(&mut h);
    common::wait_until(&mut h, 60.0, App::has_result);
    h.get_by_label("Preset").hover();
    h.run_steps(3);
    h.set_render_cursor(false);
    h.run_steps(1);
    let mut regions = vec![Region::same("row", (0, 0), (1162, 36))];
    regions.extend(block_regions(1162, 1, MULTICOLOUR_SHIFT, false));
    check_regions("px-14-preset-hover", &h.render().unwrap(), &regions);
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
