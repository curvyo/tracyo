//! The Parts row and the parts of a picture with the real worker: the pixel cases P7 to P15 of
//! ux-notes 12.2 against the references of `specs/0002-trace-parts`, the flows (add, select, remove,
//! change the active settings, Stop keeps finished parts) and the criteria of slice 2.

// Pixel geometry converts between integers and f32 everywhere; the values are screen coordinates.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]

mod common;
#[path = "common/compare.rs"]
mod compare;
#[path = "common/px.rs"]
mod px;

use eframe::egui::{Event, Key, Modifiers, PointerButton, Pos2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use px::Region;
use tracyo_app::App;
use tracyo_app::dialogs::Dialogs;
use tracyo_core::progress::Button;

const SPEC: &str = "0002-trace-parts";
/// The window of ux-notes 12.2: the bar caps at 1164 px.
const WINDOW: f32 = 1248.0;

fn check(name: &str, h: &mut Harness<'_, App>, regions: &[Region]) {
    px::check_regions(SPEC, name, &h.render().unwrap(), regions);
}

fn fixed() -> Dialogs {
    Dialogs::Fixed {
        open: None,
        save: None,
    }
}

/// The logo with Method Brightness, as the reference script leaves it.
fn opened<'a>(dir: &std::path::Path) -> Harness<'a, App> {
    let mut h = common::harness_sized(dir, Some(common::logo()), fixed(), WINDOW);
    common::wait_for_result(&mut h, 60.0);
    h.run_steps(3);
    h.get_by_label("Method").click();
    h.run_steps(3);
    h.get_by_label("Brightness").click();
    h.run_steps(3);
    common::wait_for_result(&mut h, 60.0);
    h.remove_cursor();
    h.run_steps(3);
    h
}

fn click_label(h: &mut Harness<'_, App>, label: &str) {
    h.get_by_label(label).click();
    h.run_steps(3);
    h.remove_cursor();
    h.run_steps(2);
}

fn corner(h: &Harness<'_, App>, x: f32, y: f32) -> Pos2 {
    h.state().picture_corner_on_screen(x, y).unwrap()
}

fn drag(h: &mut Harness<'_, App>, from: Pos2, to: Pos2) {
    h.event(Event::PointerMoved(from));
    h.run_steps(2);
    h.event(Event::PointerButton {
        pos: from,
        button: PointerButton::Primary,
        pressed: true,
        modifiers: Modifiers::NONE,
    });
    h.run_steps(2);
    for k in 1..=4 {
        h.event(Event::PointerMoved(from + (to - from) * (k as f32 / 4.0)));
        h.run_steps(2);
    }
    h.event(Event::PointerButton {
        pos: to,
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::NONE,
    });
    h.run_steps(3);
}

/// Draws a region with the Rectangle tool (which stays pressed) and waits for the trace.
fn draw(h: &mut Harness<'_, App>, (x0, y0, x1, y1): (f32, f32, f32, f32)) {
    let (a, b) = (corner(h, x0, y0), corner(h, x1, y1));
    drag(h, a, b);
    common::wait_for_result(h, 60.0);
    h.run_steps(2);
}

const R1: (f32, f32, f32, f32) = (24.0, 25.0, 324.0, 235.0);
const R2: (f32, f32, f32, f32) = (344.0, 315.0, 619.0, 405.0);
const R3: (f32, f32, f32, f32) = (100.0, 450.0, 300.0, 520.0);

/// The Threshold typed as 65 %: the references show Curvyo's automatic 65 %.
fn threshold_65(h: &mut Harness<'_, App>) {
    h.get_by_label("Threshold").focus();
    h.run_steps(2);
    h.key_press(Key::Enter);
    h.run_steps(2);
    h.event(Event::Text("65".into()));
    h.run_steps(2);
    h.key_press(Key::Enter);
    common::wait_for_result(h, 60.0);
}

/// The Advanced block opened, as the references of the Parts row are taken.
fn advanced<'a>(dir: &std::path::Path) -> Harness<'a, App> {
    let mut h = opened(dir);
    threshold_65(&mut h);
    click_label(&mut h, "More trace options");
    h
}

fn bar_rows3() -> Vec<Region> {
    // Two rows of the bar and the block row: 72 + 4 + 1 + 4 + 36 + 4, as the references are cut.
    vec![Region::same("bar", (0, 0), (1164, 121))]
}

#[test]
fn p7_one_part() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = advanced(dir.path());
    check("px-08-parts-one", &mut h, &bar_rows3());
}

/// R4 of ux-notes 12.2: the box of Curvyo's E1.
const R4: (f32, f32, f32, f32) = (330.0, 30.0, 610.0, 310.0);

/// Three parts as the references show them: part 1 with R1 and R2 Inside, part 2 with R4
/// Outside (active), part 3 without regions.
fn three_parts<'a>(dir: &std::path::Path) -> Harness<'a, App> {
    let mut h = advanced(dir);
    click_label(&mut h, "Rectangle");
    draw(&mut h, R1);
    draw(&mut h, R2);
    click_label(&mut h, "Rectangle");
    click_label(&mut h, "Add part");
    click_label(&mut h, "Mask: Outside");
    click_label(&mut h, "Rectangle");
    draw(&mut h, R4);
    click_label(&mut h, "Rectangle");
    click_label(&mut h, "Add part");
    click_label(&mut h, "Part 2 of 3, outside, 1 path");
    common::wait_until(&mut h, 30.0, |a| !a.is_analysing());
    common::wait_for_result(&mut h, 60.0);
    h.remove_cursor();
    h.run_steps(3);
    h
}

#[test]
fn p8_three_parts() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = three_parts(dir.path());
    assert_eq!((h.state().part_count(), h.state().active_part()), (3, 1));
    check("px-09-parts-three", &mut h, &bar_rows3());
}

#[test]
fn p9_chip_hover() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = three_parts(dir.path());
    h.get_by_label("Part 1 of 3, inside, 2 paths").hover();
    h.run_steps(3);
    h.set_render_cursor(false);
    h.run_steps(1);
    check("px-11-chip-hover", &mut h, &bar_rows3());
}

#[test]
fn p10_muted_chip_hover() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = three_parts(dir.path());
    h.get_by_label("Part 3 of 3, no paths").hover();
    h.run_steps(3);
    h.set_render_cursor(false);
    h.run_steps(1);
    check("px-12-muted-chip-hover", &mut h, &bar_rows3());
}

#[test]
fn p11_add_part_hover() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = three_parts(dir.path());
    h.get_by_label("Add part").hover();
    h.run_steps(3);
    h.set_render_cursor(false);
    h.run_steps(1);
    check("px-13-add-part-tooltip", &mut h, &bar_rows3());
}

/// Waits past the 400 ms a second press needs.
fn after_delay() {
    std::thread::sleep(std::time::Duration::from_millis(450));
}

#[test]
fn p12_trash_armed() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = three_parts(dir.path());
    h.get_by_label("Remove part 2").click();
    h.run_steps(3);
    assert!(h.state().trash_armed());
    h.get_by_label("Press again to remove part 2").hover();
    h.run_steps(40);
    h.set_render_cursor(false);
    h.run_steps(1);
    check("px-14-trash-armed", &mut h, &bar_rows3());
    let _ = after_delay;
}

#[test]
fn p13_chip_focus() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = three_parts(dir.path());
    // The keyboard reaches the active chip, the one Tab stop of the group.
    h.get_by_label("Part 2 of 3, outside, 1 path, active")
        .focus();
    h.run_steps(2);
    h.key_press_modifiers(Modifiers::NONE, Key::Tab);
    h.key_press_modifiers(Modifiers::SHIFT, Key::Tab);
    h.run_steps(3);
    h.remove_cursor();
    h.run_steps(2);
    check("px-15-chip-focus", &mut h, &bar_rows3());
}

#[test]
fn p15_no_part_to_trace() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = advanced(dir.path());
    click_label(&mut h, "Rectangle");
    draw(&mut h, R3);
    click_label(&mut h, "Rectangle");
    click_label(&mut h, "Add part");
    assert_eq!(
        h.state().nothing_to_save(),
        Some("0 paths: no part has anything to trace.")
    );
    h.get_by_label("Save as\u{2026}").hover();
    h.run_steps(30);
    h.set_render_cursor(false);
    h.run_steps(1);
    // The tooltip of the window of this test lies where Curvyo's does not (it is clamped to the
    // window), so the lower row is compared left of it.
    let regions = [
        Region::same("rows", (0, 0), (1164, 72)),
        Region::same("parts", (0, 72), (900, 49)),
    ];
    check("px-17-no-part-to-trace", &mut h, &regions);
}

// ---- flows with the real worker ----

/// Types `text` into the value field `label` and waits for the trace that follows.
fn type_value(h: &mut Harness<'_, App>, label: &str, text: &str) {
    h.get_by_label(label).focus();
    h.run_steps(2);
    h.key_press(Key::Enter);
    h.run_steps(2);
    h.event(Event::Text(text.into()));
    h.run_steps(2);
    h.key_press(Key::Enter);
    common::wait_for_result(h, 60.0);
    h.run_steps(2);
}

fn chip(h: &Harness<'_, App>, index: usize) -> String {
    h.state().chip_name(index).unwrap()
}

/// Clicks the chip of part `index` (0 for part 1), named as the list names it now.
fn click_chip(h: &mut Harness<'_, App>, index: usize) {
    let mut name = chip(h, index);
    if h.state().active_part() == index {
        name.push_str(", active");
    }
    click_label(h, &name);
}

#[test]
fn add_part_copies_the_active_settings_and_traces_nothing_new() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = advanced(dir.path());
    type_value(&mut h, "Smooth", "33");
    assert_eq!((h.state().part_count(), h.state().active_part()), (1, 0));
    assert_eq!(chip(&h, 0), "Part 1 of 1, whole image");
    let traced = h.state().part_traces();
    click_label(&mut h, "Add part");
    common::wait_for_result(&mut h, 60.0);
    assert_eq!((h.state().part_count(), h.state().active_part()), (2, 1));
    assert_eq!(chip(&h, 1), "Part 2 of 2, no paths");
    assert_eq!(h.state().part_settings(1).unwrap().smooth, 33, "copied");
    assert_eq!(
        h.state().part_traces(),
        traced,
        "part 1 came from the cache, part 2 adds nothing"
    );
    assert_eq!(h.state().announcement(), "2 parts. Part 2 has no paths");
    // Until it has a region its chip is muted: the part adds nothing and the result is part 1's.
    assert!(h.state().has_result());
}

#[test]
fn eight_parts_at_most_and_the_trash_keeps_one() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = advanced(dir.path());
    assert!(
        h.query_by_label("Keep at least one part").is_some(),
        "with one part the trash says why it does nothing"
    );
    for _ in 1..8 {
        click_label(&mut h, "Add part");
    }
    common::wait_for_result(&mut h, 60.0);
    assert_eq!(h.state().part_count(), 8);
    click_label(&mut h, "Add part");
    assert_eq!(h.state().part_count(), 8, "Add part is dimmed at 8 parts");
}

#[test]
fn the_chips_move_with_the_arrows_home_and_end_without_wrapping() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = advanced(dir.path());
    click_label(&mut h, "Add part");
    click_label(&mut h, "Add part");
    click_label(&mut h, "Part 3 of 3, no paths, active");
    h.get_by_label("Part 3 of 3, no paths, active").focus();
    h.run_steps(2);
    press(&mut h, Key::ArrowLeft);
    assert_eq!(h.state().active_part(), 1);
    press(&mut h, Key::Home);
    assert_eq!(h.state().active_part(), 0);
    press(&mut h, Key::ArrowLeft);
    assert_eq!(h.state().active_part(), 0, "no wrap");
    press(&mut h, Key::End);
    assert_eq!(h.state().active_part(), 2);
    press(&mut h, Key::ArrowRight);
    assert_eq!(h.state().active_part(), 2, "no wrap");
}

fn press(h: &mut Harness<'_, App>, key: Key) {
    h.key_press(key);
    h.run_steps(3);
}

#[test]
fn the_trash_arms_waits_400_ms_and_removes_the_active_part_then_the_part_before_is_active() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = advanced(dir.path());
    click_label(&mut h, "Add part");
    click_label(&mut h, "Add part");
    assert_eq!(h.state().active_part(), 2);
    h.get_by_label("Remove part 3").click();
    h.run_steps(3);
    assert!(h.state().trash_armed());
    // A second press at once does nothing: it comes before 400 ms.
    h.get_by_label("Press again to remove part 3").click();
    h.run_steps(3);
    assert_eq!(h.state().part_count(), 3);
    after_delay();
    h.get_by_label("Press again to remove part 3").click();
    h.run_steps(3);
    common::wait_for_result(&mut h, 60.0);
    assert_eq!((h.state().part_count(), h.state().active_part()), (2, 1));
    assert!(!h.state().trash_armed());
    assert_eq!(chip(&h, 1), "Part 2 of 2, no paths");
}

#[test]
fn escape_disarms_the_trash_and_is_consumed_and_a_chip_pick_disarms_it_too() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = advanced(dir.path());
    click_label(&mut h, "Add part");
    h.get_by_label("Remove part 2").click();
    h.run_steps(3);
    assert!(h.state().trash_armed());
    press(&mut h, Key::Escape);
    assert!(!h.state().trash_armed());
    assert_eq!(h.state().part_count(), 2, "the first Escape only disarms");
    h.get_by_label("Remove part 2").click();
    h.run_steps(3);
    assert!(h.state().trash_armed());
    click_chip(&mut h, 0);
    assert!(!h.state().trash_armed(), "a chip pick disarms");
    // A press anywhere else disarms as well.
    h.get_by_label("Remove part 1").click();
    h.run_steps(3);
    assert!(h.state().trash_armed());
    click_label(&mut h, "Method");
    assert!(!h.state().trash_armed());
}

#[test]
fn delete_on_a_focused_chip_arms_the_trash_and_the_second_delete_removes() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = advanced(dir.path());
    click_label(&mut h, "Add part");
    h.get_by_label("Part 2 of 2, no paths, active").focus();
    h.run_steps(2);
    press(&mut h, Key::Delete);
    assert!(h.state().trash_armed());
    after_delay();
    press(&mut h, Key::Delete);
    common::wait_for_result(&mut h, 60.0);
    assert_eq!((h.state().part_count(), h.state().active_part()), (1, 0));
}

#[test]
fn only_part_ones_settings_are_remembered() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = advanced(dir.path());
    type_value(&mut h, "Smooth", "33");
    click_label(&mut h, "Add part");
    type_value(&mut h, "Smooth", "66");
    let file = std::fs::read_to_string(dir.path().join("settings.toml")).unwrap();
    assert!(file.contains("smooth = 33"), "{file}");
    assert!(!file.contains("66"), "{file}");
    // Part 2's mask mode and regions are not in the file either; a new start has one part.
    drop(h);
    let mut again = common::harness_sized(dir.path(), Some(common::logo()), fixed(), WINDOW);
    common::wait_for_result(&mut again, 60.0);
    assert_eq!(again.state().part_count(), 1);
    assert_eq!(again.state().settings().smooth, 33);
}

#[test]
fn opening_a_picture_again_resets_to_one_empty_part_with_part_ones_settings() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = advanced(dir.path());
    type_value(&mut h, "Smooth", "33");
    click_label(&mut h, "Add part");
    type_value(&mut h, "Smooth", "66");
    assert_eq!(h.state().settings().smooth, 66);
    let ctx = h.ctx.clone();
    h.state_mut().open_path(&ctx, common::logo());
    common::wait_until(&mut h, 60.0, |a| a.part_count() == 1 && a.has_result());
    assert_eq!(h.state().active_part(), 0);
    assert_eq!(h.state().settings().smooth, 33);
    assert_eq!(h.state().mask_regions().len(), 0);
}

/// P1 of the specification: 1000 x 1000 px, the fixture of the core tests.
fn p1() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../tracyo-core/tests/fixtures/pictures/p1.png")
}

fn opened_p1<'a>(dir: &std::path::Path, save: Option<std::path::PathBuf>) -> Harness<'a, App> {
    let dialogs = Dialogs::Fixed { open: None, save };
    let mut h = common::harness_sized(dir, Some(p1()), dialogs, WINDOW);
    common::wait_for_result(&mut h, 60.0);
    h.run_steps(3);
    click_label(&mut h, "More trace options");
    h
}

/// R1 of the specification: x = 0 to 500 px, the whole height.
const SPEC_R1: (f32, f32, f32, f32) = (0.0, 0.0, 500.0, 1000.0);

#[test]
fn criterion_20_a_changed_setting_traces_the_active_part_only_and_a_mask_change_the_parts_below() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = opened_p1(dir.path(), None);
    click_label(&mut h, "Add part");
    click_label(&mut h, "Rectangle");
    draw(&mut h, SPEC_R1);
    click_label(&mut h, "Rectangle");
    common::wait_until(&mut h, 30.0, |a| !a.is_analysing());
    common::wait_for_result(&mut h, 60.0);
    h.run_steps(3);
    assert_eq!(h.state().part_regions(1).len(), 1);

    let before = h.state().part_traces();
    type_value(&mut h, "Smooth", "66");
    assert_eq!(h.state().part_traces() - before, 1, "part 2 only");

    click_chip(&mut h, 0);
    let before = h.state().part_traces();
    type_value(&mut h, "Smooth", "44");
    assert_eq!(h.state().part_traces() - before, 1, "part 1 only");

    // Moving part 2's region changes its area and the area part 1 keeps: both are traced.
    click_chip(&mut h, 1);
    let inside = corner(&h, 250.0, 500.0);
    h.event(Event::PointerMoved(inside));
    h.run_steps(2);
    for pressed in [true, false] {
        h.event(Event::PointerButton {
            pos: inside,
            button: PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        });
        h.run_steps(2);
    }
    assert!(h.state().selected_region().is_some());
    let before = h.state().part_traces();
    h.key_press(Key::ArrowRight);
    h.run_steps(3);
    common::wait_for_result(&mut h, 60.0);
    assert_eq!(
        h.state().part_traces() - before,
        2,
        "part 2 and the part below it"
    );

    // Selecting a chip traces nothing.
    let before = h.state().part_traces();
    click_chip(&mut h, 0);
    h.run_steps(3);
    assert_eq!(h.state().part_traces(), before);
}

/// The vertices of the first path of the saved SVG, in millimetres.
fn path_points(svg: &str) -> Vec<(f64, f64)> {
    let doc = roxmltree::Document::parse(svg).unwrap();
    let d = doc
        .descendants()
        .find(|n| n.has_tag_name("path"))
        .and_then(|n| n.attribute("d"))
        .unwrap();
    let numbers: Vec<&str> = d.split_whitespace().collect();
    let mut points = Vec::new();
    let mut i = 0;
    while i < numbers.len() {
        match numbers[i] {
            "M" | "L" => {
                points.push((
                    numbers[i + 1].parse().unwrap(),
                    numbers[i + 2].parse().unwrap(),
                ));
                i += 3;
            }
            "C" => {
                points.push((
                    numbers[i + 5].parse().unwrap(),
                    numbers[i + 6].parse().unwrap(),
                ));
                i += 7;
            }
            _ => i += 1,
        }
    }
    points
}

#[test]
fn criterion_15_and_21_save_writes_the_joined_inkscape_svg_with_one_path_and_no_seam_line() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("joined.svg");
    let mut h = opened_p1(dir.path(), Some(out.clone()));
    type_value(&mut h, "Threshold", "40");
    type_value(&mut h, "Smooth", "20");
    click_label(&mut h, "Add part");
    click_label(&mut h, "Rectangle");
    draw(&mut h, SPEC_R1);
    click_label(&mut h, "Rectangle");
    common::wait_until(&mut h, 30.0, |a| !a.is_analysing());
    type_value(&mut h, "Threshold", "70");
    type_value(&mut h, "Smooth", "80");
    assert_eq!(h.state().part_settings(0).unwrap().smooth, 20);
    // The preview shows the joined result: one outline for the joined disc, bar and square and
    // the small disc.
    assert_eq!(h.state().result_paths(), Some(2));

    h.get_by_label("Save as\u{2026}").click();
    // The progress of the job never runs backwards.
    let mut seen = Vec::new();
    for _ in 0..200 {
        h.step();
        if let Button::Running(Some(p)) = h.state_mut().button() {
            seen.push(p);
        }
        if out.exists() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(out.exists(), "the file is written");
    assert!(seen.windows(2).all(|w| w[0] <= w[1]), "{seen:?}");
    let svg = std::fs::read_to_string(&out).unwrap();
    let doc = roxmltree::Document::parse(&svg).unwrap();
    let layers: Vec<_> = doc
        .descendants()
        .filter(|n| {
            n.attribute(("http://www.inkscape.org/namespaces/inkscape", "groupmode"))
                == Some("layer")
        })
        .collect();
    assert_eq!(layers.len(), 1, "no layer per part");
    assert_eq!(
        layers[0].attribute(("http://www.inkscape.org/namespaces/inkscape", "label")),
        Some("#000000")
    );
    let paths: Vec<_> = doc
        .descendants()
        .filter(|n| n.has_tag_name("path"))
        .collect();
    assert_eq!(paths.len(), 1);
    let d = paths[0].attribute("d").unwrap();
    assert_eq!(
        (d.matches('M').count(), d.matches('Z').count()),
        (2, 2),
        "the joined outline and the small disc"
    );
    assert!(!svg.contains("<metadata"), "no metadata of parts");
    // No segment of the result runs along the seam x = 132.292 mm.
    let points = path_points(&svg);
    let on_seam = points
        .iter()
        .filter(|p| (p.0 - 132.292).abs() <= 0.005)
        .count();
    println!("{on_seam} vertices on the seam");
    for pair in points.windows(2) {
        let along = (pair[0].0 - 132.292).abs() <= 0.005 && (pair[1].0 - 132.292).abs() <= 0.005;
        assert!(
            !along || (pair[0].1 - pair[1].1).abs() <= 0.01,
            "a segment along the seam"
        );
    }
}

#[test]
fn a_part_off_the_picture_and_nothing_to_trace_dim_save_with_the_notice() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = opened_p1(dir.path(), None);
    click_label(&mut h, "Rectangle");
    draw(&mut h, (-150.0, 100.0, -50.0, 300.0));
    click_label(&mut h, "Rectangle");
    assert_eq!(
        h.state().nothing_to_save(),
        Some("0 paths: the mask is outside the image.")
    );
    click_label(&mut h, "Add part");
    assert_eq!(
        h.state().nothing_to_save(),
        Some("0 paths: no part has anything to trace.")
    );
    // A region in part 2 gives the picture something to trace again.
    click_label(&mut h, "Rectangle");
    draw(&mut h, (10.0, 10.0, 500.0, 500.0));
    click_label(&mut h, "Rectangle");
    assert_eq!(h.state().nothing_to_save(), None);
}

#[test]
fn saving_stopping_and_the_about_window_leave_the_parts_alone() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("saved.svg");
    let mut h = opened_p1(dir.path(), Some(out.clone()));
    click_label(&mut h, "Add part");
    click_label(&mut h, "Rectangle");
    draw(&mut h, SPEC_R1);
    click_label(&mut h, "Rectangle");
    common::wait_for_result(&mut h, 60.0);
    let regions = h.state().part_regions(1);
    h.get_by_label("Save as\u{2026}").click();
    common::wait_until(&mut h, 30.0, |_| out.exists());
    common::wait_for_result(&mut h, 60.0);
    click_label(&mut h, "About Tracyo");
    press(&mut h, Key::Escape);
    assert_eq!((h.state().part_count(), h.state().active_part()), (2, 1));
    assert_eq!(h.state().part_regions(1), regions);
}

/// A 3000 x 2200 picture (6.6 Mpx: previewed reduced, saved at full size) of dots: a
/// whole-picture trace of it at full size takes a while.
fn busy_picture(path: &std::path::Path) {
    let image = image::RgbImage::from_fn(3000, 2200, |x, y| {
        let (cx, cy) = (f64::from(x % 100) - 50.0, f64::from(y % 100) - 50.0);
        if cx * cx + cy * cy < 38.0 * 38.0 && (x / 100 + y / 100) % 3 != 0 {
            image::Rgb([20, 20, 20])
        } else {
            image::Rgb([250, 250, 250])
        }
    });
    image.save(path).unwrap();
}

#[test]
fn stop_keeps_the_parts_that_are_finished_and_the_next_job_traces_only_the_rest() {
    let dir = tempfile::tempdir().unwrap();
    let picture = dir.path().join("busy.png");
    busy_picture(&picture);
    let out = dir.path().join("busy.svg");
    let dialogs = Dialogs::Fixed {
        open: None,
        save: Some(out.clone()),
    };
    let mut h = common::harness_sized(dir.path(), Some(picture), dialogs, WINDOW);
    common::wait_for_result(&mut h, 120.0);
    h.run_steps(3);
    click_label(&mut h, "More trace options");
    // Part 1 keeps a small square: traced from a crop, it is quick.
    click_label(&mut h, "Rectangle");
    draw(&mut h, (200.0, 200.0, 400.0, 400.0));
    click_label(&mut h, "Rectangle");
    common::wait_until(&mut h, 60.0, |a| !a.is_analysing());
    common::wait_for_result(&mut h, 120.0);
    // Part 2 keeps everything outside a larger square: a whole-picture trace, slow at full size.
    click_label(&mut h, "Add part");
    click_label(&mut h, "Mask: Outside");
    click_label(&mut h, "Rectangle");
    draw(&mut h, (100.0, 100.0, 800.0, 800.0));
    click_label(&mut h, "Rectangle");
    common::wait_until(&mut h, 60.0, |a| !a.is_analysing());
    common::wait_for_result(&mut h, 120.0);
    // Save traces every part at full size in one job: part 1 first.
    let before = h.state().cached_parts();
    h.get_by_label("Save as\u{2026}").click();
    common::wait_until(&mut h, 120.0, |a| a.cached_parts() > before);
    press(&mut h, Key::Escape);
    assert!(!h.state().has_result(), "the preview is out of date");
    assert!(!out.exists(), "nothing is written");
    assert_eq!(h.state().cached_parts(), before + 1, "part 1 is kept");
    assert_eq!(h.state().part_count(), 2);
    // The preview comes back from the cache; the next Save takes part 1 from it and traces part 2
    // only.
    press(&mut h, Key::Enter);
    common::wait_for_result(&mut h, 120.0);
    let traced = h.state().part_traces();
    h.get_by_label("Save as\u{2026}").click();
    common::wait_until(&mut h, 120.0, |_| out.exists());
    assert_eq!(h.state().part_traces() - traced, 1, "part 2 only");
}
