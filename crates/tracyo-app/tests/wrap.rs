//! Where the Advanced block breaks its line (ux-notes of 0002 section 3, "Fit and wrap"): on a
//! wide window the Output line, Analyse, the Parts group and the Mask group are one line; on a
//! window too narrow for that the Parts group, with the Mask group behind it, moves to the next
//! line as a whole. Curvyo's bar was captured at 1280 x 800 only, so these cases are asserted by
//! geometry from the verified widths of ux-notes section 3 (label 32, chips 28, gaps 4, the divider
//! 12 px into a 25 px gap, the Mask group 256), not against a reference picture; the 1280 x 800
//! pixel cases of `parts.rs` and `pixels.rs` stay as they were.

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

use eframe::egui::{Rect, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use tracyo_app::App;
use tracyo_app::dialogs::Dialogs;

/// The bar at rest is 1190 px wide and centred on that width (0001 ux-notes 3.1).
const REST: f32 = 1190.0;
/// The bar's divider colour on its ground (`#B4B4B8`).
const DIVIDER: [u8; 3] = [0xB4, 0xB4, 0xB8];

fn fixed() -> Dialogs {
    Dialogs::Fixed {
        open: None,
        save: None,
    }
}

/// The logo in a window of `width` x `height` with Advanced open.
fn advanced<'a>(dir: &std::path::Path, width: f32, height: f32) -> Harness<'a, App> {
    let mut h = common::harness_window(
        dir,
        Some(common::logo()),
        fixed(),
        vec2(width, height),
        0.25,
    );
    common::wait_for_result(&mut h, 60.0);
    h.run_steps(3);
    // Method Brightness, as the references are taken: a first row of 1190 px.
    h.get_by_label("Method").click();
    h.run_steps(3);
    h.get_by_label("Brightness").click();
    h.run_steps(3);
    common::wait_for_result(&mut h, 60.0);
    h.remove_cursor();
    h.run_steps(3);
    h.get_by_label("More trace options").click();
    h.run_steps(3);
    h.remove_cursor();
    h.run_steps(3);
    h
}

fn rect(h: &Harness<'_, App>, label: &str) -> Rect {
    h.get_by_label(label).rect()
}

fn bar_left(window: f32) -> f32 {
    ((window - REST) / 2.0).floor()
}

/// Asserts the positions of the Parts group and the Mask group from the left edge `p` of the label
/// "Parts" and the top `y` of their controls (one part).
fn assert_parts_and_mask(h: &Harness<'_, App>, p: f32, y: f32) {
    let near = |name: &str, got: f32, want: f32| {
        assert!((got - want).abs() <= 1.0, "{name}: {got}, want {want}");
    };
    let chip = rect(h, "Part 1 of 1, whole image, active");
    near("chip left", chip.left(), p + 36.0);
    for (label, left) in [
        ("Add part", 68.0),
        ("Keep at least one part", 100.0),
        ("Rectangle", 153.0),
        ("Lasso", 185.0),
        ("Subtract", 217.0),
        ("Clear mask", 381.0),
    ] {
        let r = rect(h, label);
        near(label, r.left(), p + left);
        near(&format!("{label} top"), r.top(), y);
    }
    let (inside, outside) = (rect(h, "Mask: Inside"), rect(h, "Mask: Outside"));
    near("Inside", inside.left(), p + 249.0);
    near("Outside right", outside.right(), p + 377.0);
    near("Inside top", inside.top(), y);
}

/// 1920 x 1080: the Advanced block is one line, Analyse then the Parts group and the Mask group
/// right behind it, and the bar is as wide as that line.
#[test]
fn a_wide_window_keeps_advanced_parts_and_mask_on_one_line() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = advanced(dir.path(), 1920.0, 1080.0);
    let analyse = rect(&h, "Analyse");
    let left = bar_left(1920.0);
    assert_eq!(left, 365.0);
    // Parts group 25 px (divider at the 13th) behind Analyse.
    let p = analyse.right() + 25.0;
    assert_parts_and_mask(&h, p, analyse.top());
    // One line: the bar is 36 (first row) + 4 + 1 + 4 + 36 + 4 = 85 px high from y 12.
    assert!(
        (h.state().bar_bottom() - (12.0 + 85.0)).abs() <= 0.5,
        "bar bottom {}",
        h.state().bar_bottom()
    );
    // The bar ends 4 + 8 px behind Clear mask, inside the window.
    let clear = rect(&h, "Clear mask");
    let right = clear.right() + 12.0;
    assert!(right < 1920.0 - 12.0);
    let full = compare::rgb(&h.render().unwrap());
    let (bx, by) = compare::bar_origin(&full).unwrap();
    assert_eq!(bx as f32, left);
    let at = |x: f32, y: f32| full.get_pixel(x as u32, y as u32).0;
    let mid = analyse.top() + 14.0;
    assert_eq!(at(right - 1.0, mid), [0xDC, 0xDC, 0xE0], "bar ground");
    assert_ne!(at(right + 2.0, mid), [0xDC, 0xDC, 0xE0], "the bar ends");
    // The dividers: before the Parts group and between the trash and the Mask group.
    for x in [
        analyse.right() + 12.0,
        rect(&h, "Keep at least one part").right() + 12.0,
    ] {
        assert_eq!(at(x, analyse.top() + 4.0), DIVIDER, "divider at {x}");
        assert_eq!(at(x, analyse.top() + 23.0), DIVIDER);
        assert_eq!(at(x - 1.0, mid), [0xDC, 0xDC, 0xE0]);
    }
    // The Parts and Mask groups are drawn as in Curvyo's picture of the Parts row, wherever the
    // line puts them: the 409 x 36 px crop of `px-08` (x 8, y 81 of its bar), 1 px of blur allowed.
    let reference = px::reference("0002-trace-parts", "px-08-parts-one.png");
    let (crop_x, crop_y) = (p - left, analyse.top() - 4.0 - by as f32);
    let outcome = compare::compare_between(
        &reference,
        &full,
        (8, 81),
        (crop_x as u32, crop_y as u32),
        409,
        36,
    );
    outcome
        .diff
        .save(compare::out_dir().join("diff-wide-parts.png"))
        .unwrap();
    assert!(outcome.passed(), "{}", outcome.summary());
}

/// 1300 x 800: the line holds the Output line and Analyse but not the Parts group, so the Parts
/// group with the Mask group behind it moves down as a whole, no divider at the wrap; the bar keeps
/// its 1198 px.
#[test]
fn a_window_too_narrow_for_one_line_moves_the_parts_group_down_as_a_whole() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = advanced(dir.path(), 1300.0, 800.0);
    let analyse = rect(&h, "Analyse");
    let left = bar_left(1300.0);
    assert!(
        (analyse.right() - (left + REST - 4.0)).abs() <= 1.0,
        "Analyse stays 4 px (the block's padding) from the far right of the 1198 px bar: {}",
        analyse.right()
    );
    let p = left + 8.0;
    assert_parts_and_mask(&h, p, analyse.top() + 36.0);
    assert!(
        (h.state().bar_bottom() - (12.0 + 121.0)).abs() <= 0.5,
        "two lines: bar bottom {}",
        h.state().bar_bottom()
    );
    // The Mask group stays behind the trash with its divider; nothing is drawn at the line start.
    let full = compare::rgb(&h.render().unwrap());
    let at = |x: f32, y: f32| full.get_pixel(x as u32, y as u32).0;
    let y = analyse.top() + 36.0 + 4.0;
    assert_eq!(
        at(rect(&h, "Keep at least one part").right() + 12.0, y),
        DIVIDER
    );
    for x in 0..8 {
        assert_eq!(at(left + x as f32, y), [0xDC, 0xDC, 0xE0]);
    }
}

/// The window width where one line stops fitting: one pixel less wraps, the line is the same.
#[test]
fn the_parts_group_wraps_exactly_when_the_line_no_longer_fits_the_window() {
    let dir = tempfile::tempdir().unwrap();
    let wide = advanced(dir.path(), 1920.0, 1080.0);
    // The bar is as wide as the line; the cap is the window less 24 px.
    let bar = rect(&wide, "Clear mask").right() + 12.0 - bar_left(1920.0);
    let need = bar + 24.0;
    println!("one line is {bar} px of bar, so the window must be {need} px");
    drop(wide);
    for (window, one_line) in [(need.ceil(), true), (need.ceil() - 1.0, false)] {
        let dir = tempfile::tempdir().unwrap();
        let h = advanced(dir.path(), window, 800.0);
        let analyse = rect(&h, "Analyse");
        let parts_y = rect(&h, "Add part").top();
        assert_eq!(
            (parts_y - analyse.top()).abs() < 1.0,
            one_line,
            "window {window}"
        );
    }
}

/// Each further part widens the row by 32 px: at 1920 px eight parts still share the line, and at
/// the window that just fits one part, a second part moves the whole row down.
#[test]
fn more_parts_widen_the_line_until_the_window_does_not_suffice() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = advanced(dir.path(), 1920.0, 1080.0);
    for _ in 1..8 {
        h.get_by_label("Add part").click();
        h.run_steps(3);
        h.remove_cursor();
        h.run_steps(2);
    }
    let analyse = rect(&h, "Analyse");
    let add = rect(&h, "Add part");
    assert!(
        (add.top() - analyse.top()).abs() < 1.0,
        "eight parts, one line"
    );
    assert!(
        (add.left() - (analyse.right() + 25.0 + 292.0)).abs() <= 1.0,
        "Add part follows the eight chips: {}",
        add.left()
    );
    let dir = tempfile::tempdir().unwrap();
    let mut h = advanced(dir.path(), 1371.0, 800.0);
    assert!((rect(&h, "Add part").top() - rect(&h, "Analyse").top()).abs() < 1.0);
    h.get_by_label("Add part").click();
    h.run_steps(3);
    h.remove_cursor();
    h.run_steps(2);
    let analyse = rect(&h, "Analyse");
    assert!(
        (rect(&h, "Add part").top() - (analyse.top() + 36.0)).abs() < 1.0,
        "two parts do not fit: the Parts row moves down"
    );
    assert!((analyse.right() - (bar_left(1371.0) + REST - 4.0)).abs() <= 1.0);
}
