//! Pixel comparison of the app's states with the references of `specs/0001-trace-and-save`,
//! re-captured on 2026-11-13: a 1280 x 800 window for every case, the bar centred with Open as its
//! first control, the Advanced block with the Parts row and the Mask group of `0002`. Each region
//! is compared where it stands in the reference; the 0002 cases of the Parts row and the Mask
//! group are in `masks.rs` and `parts.rs`.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;
#[path = "common/compare.rs"]
mod compare;
#[path = "common/px.rs"]
mod px;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use px::Region;
use tracyo_app::App;
use tracyo_app::dialogs::Dialogs;
use tracyo_core::progress::Button;

const SPEC: &str = "0001-trace-and-save";

fn reference(name: &str) -> image::RgbImage {
    px::reference(SPEC, name)
}

/// A bar of `width` x `height` compared as one region of the reference.
fn bar(name: &'static str, width: u32, height: u32) -> Vec<Region> {
    vec![Region::same(name, (0, 0), (width, height))]
}

/// Compares every region with the reference `name` of 0001 and asserts that all pass.
fn check_regions(name: &str, rendered: &image::RgbaImage, regions: &[Region]) {
    px::check_regions(SPEC, name, rendered, regions);
}

/// The bar of the first row alone is 36 px high; with Advanced open it is 121 px (the Output line,
/// the Parts row and the block's padding).
const OPEN_H: u32 = 121;

/// The three swatches of the logo's Brightness palette.
const SWATCHES: u32 = 3;

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
    common::wait_for_result(&mut h, 60.0);
    h.run_steps(3);
    h
}

/// Chooses `method` in the Method menu and waits for the new preview.
fn choose_method(h: &mut Harness<'_, App>, method: &str) {
    h.get_by_label("Method").click();
    h.run_steps(3);
    h.get_by_label(method).click();
    h.run_steps(3);
    common::wait_for_result(h, 60.0);
    h.remove_cursor();
    h.run_steps(3);
}

/// The window of every case.
const WINDOW: f32 = 1280.0;

/// The logo with Brightness chosen, as the reference script leaves it for most states.
fn opened<'a>(dir: &std::path::Path) -> Harness<'a, App> {
    let mut h = analysed(dir, WINDOW);
    choose_method(&mut h, "Brightness");
    h
}

/// The swatch label of colour number `index` of the palette.
fn swatch(h: &Harness<'_, App>, index: usize, excluded: bool) -> String {
    tracyo_core::palette::swatch_label(h.state().palette_colours()[index], excluded)
}

/// Chooses Output Multicolour and waits for the palette, as the script does.
fn choose_multicolour(h: &mut Harness<'_, App>) {
    h.get_by_label("Output: Multicolour").click();
    h.run_steps(3);
    common::wait_for_result_where(h, 60.0, |a| a.palette_colours().len() == SWATCHES as usize);
    h.remove_cursor();
    h.run_steps(3);
}

/// Ctrl+clicks the second swatch (Exclude) and waits for the result.
fn exclude_second(h: &mut Harness<'_, App>) {
    let name = swatch(h, 1, false);
    common::ctrl_click(h, &name);
    common::wait_for_result(h, 60.0);
    h.remove_cursor();
    h.run_steps(3);
}

/// The state every picture after px-11 is taken in: Multicolour with the second colour excluded.
fn multicolour_excluded(h: &mut Harness<'_, App>) {
    choose_multicolour(h);
    exclude_second(h);
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
    // Smooth stands at 562..712 of the bar, which stands at x = 45; 81 % of its width.
    let at = pos2(45.0 + 562.0 + 0.81 * 150.0, 30.0);
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
    check_regions(
        "px-00-bar-closed",
        &h.render().unwrap(),
        &bar("bar", 1190, 36),
    );
}

#[test]
fn px_01_empty() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = common::harness(dir.path(), None, fixed());
    h.run_steps(3);
    check_regions("px-01-empty", &h.render().unwrap(), &bar("bar", 165, 36));
}

#[test]
fn px_02_preview() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = opened(dir.path());
    check_regions("px-02-preview", &h.render().unwrap(), &bar("bar", 1190, 36));
}

#[test]
fn px_03_advanced_open() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = opened(dir.path());
    open_advanced(&mut h);
    check_regions(
        "px-03-advanced-open",
        &h.render().unwrap(),
        &bar("bar", 1198, OPEN_H),
    );
}

#[test]
fn px_04_multicolour_palette() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = opened(dir.path());
    open_advanced(&mut h);
    choose_multicolour(&mut h);
    check_regions(
        "px-04-multicolour-palette",
        &h.render().unwrap(),
        &bar("bar", 1198, OPEN_H),
    );
}

#[test]
fn px_10_palette_swatch_hover() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = opened(dir.path());
    open_advanced(&mut h);
    choose_multicolour(&mut h);
    let name = swatch(&h, 1, false);
    h.get_by_label(&name).hover();
    h.run_steps(30);
    h.set_render_cursor(false);
    h.run_steps(1);
    let mut regions = bar("bar", 1198, OPEN_H);
    // The tooltip under the second swatch with its 1 px ring: two lines, 8 px below the cell. It
    // reaches 2 px below the bar.
    regions.push(Region::same("tooltip", (323, 84), (104, 39)));
    check_regions("px-10-palette-swatch-hover", &h.render().unwrap(), &regions);
}

#[test]
fn px_11_palette_excluded() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = opened(dir.path());
    open_advanced(&mut h);
    multicolour_excluded(&mut h);
    check_regions(
        "px-11-palette-excluded",
        &h.render().unwrap(),
        &bar("bar", 1198, OPEN_H),
    );
}

#[test]
fn px_05_running_42() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = opened(dir.path());
    open_advanced(&mut h);
    multicolour_excluded(&mut h);
    h.state_mut().wrap_worker(staged_running());
    smooth_81(&mut h);
    let start = std::time::Instant::now();
    while !matches!(h.state_mut().button(), Button::Running(Some(42))) {
        h.step();
        std::thread::sleep(std::time::Duration::from_millis(20));
        assert!(start.elapsed().as_secs() < 20, "never showed 42 %");
    }
    h.run_steps(2);
    // The blue button is 75 px wide as in the reference: the bar is 14 px narrower.
    check_regions(
        "px-05-running-42",
        &h.render().unwrap(),
        &bar("bar", 1184, OPEN_H),
    );
    assert!(matches!(h.state_mut().button(), Button::Running(Some(42))));
}

/// Smooth 81, Advanced open, Multicolour with the second colour excluded.
fn custom_with_block<'a>(dir: &std::path::Path) -> Harness<'a, App> {
    let mut h = opened(dir);
    open_advanced(&mut h);
    multicolour_excluded(&mut h);
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
    // One row at 1280 px: "Stopped" with Retry makes the bar 17 px wider than at rest.
    check_regions(
        "px-07-stopped",
        &h.render().unwrap(),
        &bar("bar", 1215, OPEN_H),
    );
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
    check_regions(
        "px-06-failed",
        &h.render().unwrap(),
        &bar("bar", 1219, OPEN_H),
    );
}

#[test]
fn px_13_trace_button_focus() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = opened(dir.path());
    open_advanced(&mut h);
    multicolour_excluded(&mut h);
    smooth_81(&mut h);
    common::wait_for_result(&mut h, 60.0);
    h.get_by_label("Fewer trace options").focus();
    h.run_steps(2);
    h.key_press(eframe::egui::Key::Tab);
    h.run_steps(3);
    h.remove_cursor();
    h.run_steps(2);
    check_regions(
        "px-13-trace-button-focus",
        &h.render().unwrap(),
        &bar("bar", 1198, OPEN_H),
    );
}

#[test]
fn px_09_method_menu() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = analysed(dir.path(), WINDOW);
    h.get_by_label("Method").click();
    h.run_steps(3);
    h.remove_cursor();
    h.run_steps(3);
    // Both rows of the Remove background bar (it takes the 1256 px cap) and the menu below them.
    let regions = vec![
        Region::same("rows", (0, 0), (1256, 72)),
        Region::same("menu", (220, 72), (232, 108)),
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
    check_regions(
        "px-12-value-field-tooltip",
        &h.render().unwrap(),
        &bar("bar", 1190, 36),
    );
}

#[test]
fn px_14_preset_hover() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = opened(dir.path());
    open_advanced(&mut h);
    multicolour_excluded(&mut h);
    smooth_81(&mut h);
    common::wait_for_result(&mut h, 60.0);
    h.get_by_label("Preset").hover();
    h.run_steps(3);
    h.set_render_cursor(false);
    h.run_steps(1);
    check_regions(
        "px-14-preset-hover",
        &h.render().unwrap(),
        &bar("bar", 1198, OPEN_H),
    );
}

#[test]
fn px_08_preset_menu() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = opened(dir.path());
    h.get_by_label("Preset").click();
    h.run_steps(3);
    h.remove_cursor();
    h.run_steps(2);
    // The menu: under Preset, 80 px right of the bar's left edge, 4 px below it, 238 x 184.
    let regions = vec![
        Region::same("row", (0, 0), (1190, 36)),
        Region::same("menu", (80, 40), (238, 184)),
    ];
    check_regions("px-08-preset-menu", &h.render().unwrap(), &regions);
}

/// The longest vertical white run in column `x` of `image` that starts below row `from`: first
/// row and length. In the box's left padding it is the box's height.
fn white_run(image: &image::RgbImage, x: u32, from: u32) -> (u32, u32) {
    let white = |y: u32| image.get_pixel(x, y).0 == [255, 255, 255];
    let first = (from..image.height())
        .find(|&y| white(y))
        .expect("a tooltip box");
    let rows = (first..image.height()).take_while(|&y| white(y)).count();
    (first, u32::try_from(rows).unwrap())
}

/// The last white column of the tooltip box on the row through its middle (row 64 of both).
fn box_right(image: &image::RgbImage) -> u32 {
    (0..image.width())
        .rev()
        .find(|&x| (62..66).all(|y| image.get_pixel(x, y).0 == [255, 255, 255]) && x < 120)
        .expect("the box's right edge")
}

/// The first column from `from` on that holds a dark text pixel in the rows `top..top + rows`.
fn text_start(image: &image::RgbImage, from: u32, top: u32, rows: u32) -> u32 {
    (from..image.width())
        .find(|&x| (top..top + rows).any(|y| image.get_pixel(x, y).0[0] < 200))
        .expect("tooltip text")
}

/// The reference reads "About Trace", Tracyo's tooltip "About Tracyo": the box is centred under
/// the button, so it is 5 px wider. Compared: the hovered button, the box's rows and the glyphs
/// of "About T" (same text, aligned on the first glyph).
#[test]
fn px_15_tooltip() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = opened(dir.path());
    open_advanced(&mut h);
    multicolour_excluded(&mut h);
    smooth_81(&mut h);
    common::wait_for_result(&mut h, 60.0);
    h.get_by_label("About Tracyo").hover();
    h.run_steps(30);
    h.set_render_cursor(false);
    h.run_steps(1);
    let rendered = h.render().unwrap();
    let out = compare::out_dir();
    let got = px::aligned(&compare::rgb(&rendered));
    got.save(out.join("render-px-15-tooltip.png")).unwrap();
    let want = reference("px-15-tooltip.png");

    // The hovered button.
    let outcome = compare::compare_at(&want, &got, (44, 4), 28, 28);
    println!("px-15 button: {}", outcome.summary());
    assert!(outcome.passed(), "button: {}", outcome.summary());

    // The box: same top row and height.
    let rows = |image: &image::RgbImage| {
        let right = box_right(image);
        (right - 8..=right - 6)
            .map(|x| white_run(image, x, 49))
            .max_by_key(|&(_, rows)| rows)
            .unwrap()
    };
    let (ref_top, ref_rows) = rows(&want);
    let (got_top, got_rows) = rows(&got);
    println!("px-15 box: reference rows {ref_top}+{ref_rows}, render {got_top}+{got_rows}");
    assert_eq!((got_top, got_rows), (ref_top, ref_rows), "the box's rows");

    // The glyphs of "About T", aligned on the first one.
    let box_left = |image: &image::RgbImage, top: u32| {
        (0..image.width())
            .find(|&x| image.get_pixel(x, top + 11).0 == [255, 255, 255])
            .expect("the box's left edge")
    };
    let ref_x = text_start(&want, box_left(&want, ref_top), ref_top + 8, 8);
    let got_x = text_start(&got, box_left(&got, got_top), got_top + 10, 8);
    println!("px-15 text starts at {ref_x} (reference) and {got_x} (render)");
    let width = 38;
    let crop = |image: &image::RgbImage, x: u32| {
        image::imageops::crop_imm(image, x.saturating_sub(2), ref_top - 1, width, ref_rows + 2)
            .to_image()
    };
    let outcome = compare::compare_crops(&crop(&want, ref_x), &crop(&got, got_x));
    outcome
        .diff
        .save(out.join("diff-px-15-tooltip-text.png"))
        .unwrap();
    println!("px-15 text: {}", outcome.summary());
    assert!(outcome.passed(), "text: {}", outcome.summary());
}
