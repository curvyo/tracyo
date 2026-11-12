//! The Lasso region with the real app and the real worker: drawing (criterion 28), editing
//! (criterion 29), the non-zero fill, Add and Subtract, and the looks C5 to C8 of ux-notes 12.2 by
//! sampled pixels. The saved SVG of a slanted lasso side is checked in millimetres.

// Pixel geometry converts between integers and f32 everywhere; the values are screen coordinates.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::many_single_char_names
)]

#[path = "common/canvas.rs"]
mod canvas;
mod common;
#[path = "common/compare.rs"]
mod compare;

use std::path::{Path, PathBuf};

use canvas::{
    button, click, click_px, click_with, corner, drag_begin, drag_px, open_advanced, press,
    release, saved_paths, settle,
};
use eframe::egui::{Event, Key, Modifiers, Pos2, pos2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use image::RgbImage;
use tracyo_app::App;
use tracyo_app::dialogs::Dialogs;
use tracyo_core::regions::Form;

/// L1 of ux-notes 12.2: picture pixels (60, 270), (230, 290), (200, 400).
const L1: [(i32, i32); 3] = [(60, 270), (230, 290), (200, 400)];

fn fixed(save: Option<PathBuf>) -> Dialogs {
    Dialogs::Fixed { open: None, save }
}

fn opened<'a>(dir: &Path, save: Option<PathBuf>) -> Harness<'a, App> {
    let mut h = common::harness(dir, Some(common::logo()), fixed(save));
    settle(&mut h);
    h.run_steps(3);
    open_advanced(&mut h);
    h
}

fn at(h: &Harness<'_, App>, p: (i32, i32)) -> Pos2 {
    corner(h, p.0 as f32, p.1 as f32)
}

/// Lasso pressed and the corners of `points` clicked, none finished.
fn set_corners(h: &mut Harness<'_, App>, points: &[(i32, i32)]) {
    for &p in points {
        let pos = at(h, p);
        click(h, pos);
    }
}

fn corners_of(region: &tracyo_core::regions::Region) -> Vec<(i32, i32)> {
    match &region.form {
        Form::Lasso(points) => points.iter().map(|c| (c.x, c.y)).collect(),
        Form::Rect(_) => panic!("a rectangle"),
    }
}

fn lassos(h: &Harness<'_, App>) -> Vec<Vec<(i32, i32)>> {
    h.state().mask_shapes().iter().map(corners_of).collect()
}

fn pressed(h: &Harness<'_, App>) -> (bool, bool) {
    h.state().mask_tools()
}

// ---- criterion 28: drawing ----

#[test]
fn clicks_set_corners_on_whole_pixels_and_a_click_on_the_first_corner_closes_the_lasso() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = opened(dir.path(), None);
    button(&mut h, "Lasso");
    assert!(h.state().lasso_pressed());
    assert_eq!(pressed(&h), (false, false), "Rectangle is not pressed");
    // A click lands on the nearest pixel: half a pixel off still sets (60, 270).
    let near = at(&h, L1[0]) + eframe::egui::vec2(0.2, -0.3);
    click(&mut h, near);
    set_corners(&mut h, &L1[1..]);
    assert_eq!(h.state().lasso_in_progress(), L1.to_vec());
    assert_eq!(
        lassos(&h),
        Vec::<Vec<(i32, i32)>>::new(),
        "nothing stored yet"
    );
    // The first corner with three corners set: closes, adds no corner, keeps Lasso pressed.
    let first = at(&h, L1[0]);
    click(&mut h, first);
    settle(&mut h);
    assert_eq!(lassos(&h), vec![L1.to_vec()]);
    assert_eq!(h.state().lasso_in_progress(), Vec::<(i32, i32)>::new());
    assert!(
        h.state().lasso_pressed(),
        "Lasso stays pressed after a lasso"
    );
    assert!(h.state().mask_regions()[0].0.width() > 100);
}

#[test]
fn a_double_click_finishes_and_its_second_click_adds_no_corner() {
    let dir = tempfile::tempdir().unwrap();
    let mut h =
        common::harness_stepped(dir.path(), Some(common::logo()), fixed(None), 1280.0, 0.02);
    settle(&mut h);
    open_advanced(&mut h);
    button(&mut h, "Lasso");
    set_corners(&mut h, &L1);
    // Press, release, press, release at one place within 0.3 s: the first press sets the fourth
    // corner, the second finishes without a fifth.
    let d = at(&h, (60, 400));
    h.event(Event::PointerMoved(d));
    h.run_steps(2);
    for pressed in [true, false, true, false] {
        h.event(Event::PointerButton {
            pos: d,
            button: eframe::egui::PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        });
        h.run_steps(2);
    }
    settle(&mut h);
    assert_eq!(
        lassos(&h),
        vec![vec![(60, 270), (230, 290), (200, 400), (60, 400)]]
    );
    assert_eq!(h.state().lasso_in_progress().len(), 0);
}

#[test]
fn enter_finishes_the_lasso_and_is_not_save_as() {
    let dir = tempfile::tempdir().unwrap();
    let save = dir.path().join("enter.svg");
    let mut h = opened(dir.path(), Some(save.clone()));
    button(&mut h, "Lasso");
    set_corners(&mut h, &L1);
    press(&mut h, Key::Enter, Modifiers::NONE);
    settle(&mut h);
    assert_eq!(lassos(&h), vec![L1.to_vec()]);
    assert!(h.state().lasso_pressed(), "Enter keeps Lasso pressed");
    assert!(!save.exists(), "Enter was consumed: nothing was saved");
    // With nothing in progress Enter is Save as… again.
    press(&mut h, Key::Enter, Modifiers::NONE);
    common::wait_until(&mut h, 60.0, |_| save.exists());
}

#[test]
fn escape_finishes_the_lasso_and_releases_the_tool() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = opened(dir.path(), None);
    button(&mut h, "Lasso");
    set_corners(&mut h, &L1);
    press(&mut h, Key::Escape, Modifiers::NONE);
    settle(&mut h);
    assert_eq!(lassos(&h), vec![L1.to_vec()], "closed by a straight side");
    assert!(!h.state().lasso_pressed());
    // Without a lasso in progress Esc releases the tool as it does for Rectangle.
    button(&mut h, "Lasso");
    press(&mut h, Key::Escape, Modifiers::NONE);
    assert!(!h.state().lasso_pressed());
}

#[test]
fn a_lasso_in_progress_is_closed_when_another_tool_a_chip_or_add_part_ends_it() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = opened(dir.path(), None);
    button(&mut h, "Lasso");
    set_corners(&mut h, &L1);
    button(&mut h, "Rectangle");
    settle(&mut h);
    assert_eq!(lassos(&h), vec![L1.to_vec()]);
    assert_eq!(pressed(&h), (true, false), "the press of Rectangle acts");
    assert!(
        !h.state().lasso_pressed(),
        "pressing one tool releases the other"
    );
    // Lasso pressed again, the tool left by Add part: the lasso stays with part 1.
    button(&mut h, "Lasso");
    set_corners(&mut h, &[(300, 100), (400, 100), (400, 200)]);
    button(&mut h, "Add part");
    settle(&mut h);
    assert_eq!(h.state().part_count(), 2);
    assert_eq!(
        h.state().part_regions(0).len(),
        2,
        "both lassos are part 1's"
    );
    assert_eq!(
        h.state().mask_regions().len(),
        0,
        "part 2 starts without regions"
    );
    // Picking a chip ends a lasso as well.
    set_corners(&mut h, &[(300, 100), (400, 100), (400, 200)]);
    let chip = h.state().chip_name(0).unwrap();
    button(&mut h, &chip);
    settle(&mut h);
    assert_eq!(h.state().part_regions(1).len(), 1);
    assert_eq!(h.state().active_part(), 0);
}

#[test]
fn fewer_than_three_corners_or_no_area_store_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = opened(dir.path(), None);
    button(&mut h, "Lasso");
    set_corners(&mut h, &[(60, 270), (230, 290)]);
    press(&mut h, Key::Escape, Modifiers::NONE);
    assert_eq!(lassos(&h), Vec::<Vec<(i32, i32)>>::new());
    assert_eq!(h.state().lasso_in_progress().len(), 0);
    // Three corners on one line have no area.
    button(&mut h, "Lasso");
    set_corners(&mut h, &[(60, 270), (100, 270), (200, 270)]);
    press(&mut h, Key::Enter, Modifiers::NONE);
    assert_eq!(lassos(&h), Vec::<Vec<(i32, i32)>>::new());
    assert_eq!(h.state().mask_regions().len(), 0);
}

#[test]
fn backspace_takes_the_last_corner_and_the_only_corner_cancels() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = opened(dir.path(), None);
    button(&mut h, "Lasso");
    set_corners(&mut h, &L1);
    press(&mut h, Key::Backspace, Modifiers::NONE);
    assert_eq!(h.state().lasso_in_progress(), L1[..2].to_vec());
    press(&mut h, Key::Backspace, Modifiers::NONE);
    press(&mut h, Key::Backspace, Modifiers::NONE);
    assert_eq!(
        h.state().lasso_in_progress().len(),
        0,
        "the only corner cancels"
    );
    assert_eq!(h.state().mask_regions().len(), 0, "no region was stored");
}

#[test]
fn no_modifier_does_anything_while_a_lasso_is_drawn() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = opened(dir.path(), None);
    button(&mut h, "Lasso");
    let held = [Modifiers::SHIFT, Modifiers::COMMAND, Modifiers::ALT];
    for (p, m) in L1.iter().zip(held) {
        let pos = at(&h, *p);
        click_with(&mut h, pos, m);
    }
    // Every corner lands at the pointer's pixel: no angle lock, no snap, no square.
    assert_eq!(h.state().lasso_in_progress(), L1.to_vec());
    assert_eq!(h.state().mask_regions().len(), 0);
}

#[test]
fn clear_mask_drops_a_lasso_in_progress_with_the_regions() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = opened(dir.path(), None);
    button(&mut h, "Lasso");
    set_corners(&mut h, &L1);
    press(&mut h, Key::Enter, Modifiers::NONE);
    settle(&mut h);
    set_corners(&mut h, &[(300, 100), (400, 100), (400, 200)]);
    assert_eq!(h.state().lasso_in_progress().len(), 3);
    button(&mut h, "Clear mask");
    settle(&mut h);
    assert_eq!(h.state().lasso_in_progress().len(), 0);
    assert_eq!(h.state().mask_regions().len(), 0);
}

fn area(triangles: &[[kurbo::Point; 3]]) -> f64 {
    triangles
        .iter()
        .map(|[a, b, c]| ((b.x - a.x) * (c.y - a.y) - (c.x - a.x) * (b.y - a.y)).abs() / 2.0)
        .sum()
}

/// L1's area by the shoelace formula.
fn l1_area() -> f64 {
    let p = |i: usize| (f64::from(L1[i].0), f64::from(L1[i].1));
    let (a, b, c) = (p(0), p(1), p(2));
    ((b.0 - a.0) * (c.1 - a.1) - (c.0 - a.0) * (b.1 - a.1)).abs() / 2.0
}

#[test]
fn a_lasso_is_added_or_subtracted_and_the_wash_follows_its_slanted_sides() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = opened(dir.path(), None);
    button(&mut h, "Lasso");
    set_corners(&mut h, &L1);
    press(&mut h, Key::Enter, Modifiers::NONE);
    settle(&mut h);
    // Inside: everything but L1 is washed; the wash is a mesh of triangles in picture pixels.
    let around = area(&h.state().washed_triangles());
    println!("washed around L1: {around}, picture {}", 640.0 * 420.0);
    assert!((around - (640.0 * 420.0 - l1_area())).abs() < 0.01);
    button(&mut h, "Mask: Outside");
    settle(&mut h);
    assert!((area(&h.state().washed_triangles()) - l1_area()).abs() < 0.01);
    button(&mut h, "Mask: Inside");
    // A lasso finished while Subtract is pressed takes its area away from the mask.
    button(&mut h, "Subtract");
    let small = [(150, 300), (190, 300), (170, 340)];
    set_corners(&mut h, &small);
    press(&mut h, Key::Enter, Modifiers::NONE);
    settle(&mut h);
    let shapes = h.state().mask_shapes();
    assert_eq!(shapes.len(), 2);
    assert!(!shapes[0].subtract && shapes[1].subtract);
    let hole = 20.0 * 40.0;
    let around = area(&h.state().washed_triangles());
    assert!(
        (around - (640.0 * 420.0 - l1_area() + hole)).abs() < 0.01,
        "{around}"
    );
}

#[test]
fn a_figure_eight_is_filled_wherever_it_is_covered() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = opened(dir.path(), None);
    button(&mut h, "Lasso");
    set_corners(&mut h, &[(100, 100), (300, 300), (300, 100), (100, 300)]);
    press(&mut h, Key::Enter, Modifiers::NONE);
    settle(&mut h);
    button(&mut h, "Mask: Outside");
    settle(&mut h);
    // Two triangles of 200 x 100 each: 2 x 10,000.
    let covered = area(&h.state().washed_triangles());
    println!("covered by the figure 8: {covered}");
    assert!((covered - 20_000.0).abs() < 0.01);
}

// ---- criterion 29: editing ----

/// L1 finished, the tool released, the pointer gone.
fn with_l1<'a>(dir: &Path) -> Harness<'a, App> {
    let mut h = opened(dir, None);
    button(&mut h, "Lasso");
    set_corners(&mut h, &L1);
    press(&mut h, Key::Enter, Modifiers::NONE);
    settle(&mut h);
    button(&mut h, "Lasso");
    h.remove_cursor();
    h.run_steps(2);
    h
}

#[test]
fn a_click_inside_or_within_eight_px_of_a_side_selects_a_lasso_and_empty_canvas_deselects() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = with_l1(dir.path());
    assert!(h.state().selected_region().is_none());
    let inside = corner(&h, 163.0, 320.0);
    click(&mut h, inside);
    assert!(h.state().selected_region().is_some());
    click_px(&mut h, 400.0, 100.0);
    assert!(h.state().selected_region().is_none());
    // The side from (60, 270) to (230, 290) is at y = 280 in the middle: 5 px above it is within
    // the 8 px, 30 px above it is not.
    click_px(&mut h, 145.0, 275.0);
    assert!(
        h.state().selected_region().is_some(),
        "within 8 px of a side"
    );
    click_px(&mut h, 145.0, 250.0);
    assert!(h.state().selected_region().is_none(), "30 px away");
}

#[test]
fn a_drag_inside_moves_the_lasso_whole_and_esc_during_the_drag_restores_it() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = with_l1(dir.path());
    drag_px(&mut h, (163.0, 320.0), (193.0, 340.0));
    settle(&mut h);
    let moved: Vec<(i32, i32)> = L1.iter().map(|&(x, y)| (x + 30, y + 20)).collect();
    assert_eq!(lassos(&h), vec![moved.clone()], "moved by whole pixels");
    // Esc during a drag restores the start.
    let (from, to) = (corner(&h, 193.0, 340.0), corner(&h, 253.0, 380.0));
    drag_begin(&mut h, from, to);
    press(&mut h, Key::Escape, Modifiers::NONE);
    release(&mut h, to);
    assert_eq!(lassos(&h), vec![moved]);
}

#[test]
fn a_drag_on_a_corner_moves_that_corner_and_it_stays_selected() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = with_l1(dir.path());
    click_px(&mut h, 163.0, 320.0);
    // Corner 1 is at (230, 290): drag it by (+10, -15).
    drag_px(&mut h, (230.0, 290.0), (240.0, 275.0));
    settle(&mut h);
    assert_eq!(lassos(&h), vec![vec![(60, 270), (240, 275), (200, 400)]]);
    assert_eq!(h.state().selected_corner(), Some(1));
    // A corner may go anywhere, also across the lasso: it then crosses itself.
    drag_px(&mut h, (200.0, 400.0), (260.0, 270.0));
    settle(&mut h);
    assert_eq!(lassos(&h), vec![vec![(60, 270), (240, 275), (260, 270)]]);
}

#[test]
fn arrows_move_a_selected_lasso_and_delete_or_backspace_remove_it() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = with_l1(dir.path());
    click_px(&mut h, 163.0, 320.0);
    press(&mut h, Key::ArrowRight, Modifiers::NONE);
    press(&mut h, Key::ArrowDown, Modifiers::SHIFT);
    settle(&mut h);
    let moved: Vec<(i32, i32)> = L1.iter().map(|&(x, y)| (x + 1, y + 10)).collect();
    assert_eq!(lassos(&h), vec![moved]);
    press(&mut h, Key::Delete, Modifiers::NONE);
    settle(&mut h);
    assert_eq!(h.state().mask_regions().len(), 0);
    // Backspace does it too.
    button(&mut h, "Lasso");
    set_corners(&mut h, &L1);
    press(&mut h, Key::Enter, Modifiers::NONE);
    settle(&mut h);
    button(&mut h, "Lasso");
    click_px(&mut h, 163.0, 320.0);
    press(&mut h, Key::Backspace, Modifiers::NONE);
    settle(&mut h);
    assert_eq!(h.state().mask_regions().len(), 0);
}

#[test]
fn a_lasso_has_no_resize_handles_and_its_corner_squares_are_14_px() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = with_l1(dir.path());
    click_px(&mut h, 163.0, 320.0);
    h.event(Event::PointerMoved(pos2(1000.0, 700.0)));
    h.run_steps(3);
    h.set_render_cursor(false);
    let image = compare::rgb(&h.render().unwrap());
    let pixel = |p: Pos2| image.get_pixel(p.x.round() as u32, p.y.round() as u32).0;
    // The box of L1 is x 60 to 230, y 270 to 400: no 8 x 8 handle in the middle of a side.
    let mid_top = corner(&h, 145.0, 270.0);
    let white_run = (0..9)
        .filter(|k| pixel(mid_top + eframe::egui::vec2(*k as f32 - 4.0, 0.0)) == [255, 255, 255])
        .count();
    assert!(white_run < 5, "no handle at the middle of the top side");
    // A corner square: 14 px, white inside, a 1 px #3A3A3F edge.
    let c = at(&h, L1[0]);
    let (cx, cy) = (c.x.round(), c.y.round());
    assert_eq!(pixel(pos2(cx, cy)), [255, 255, 255]);
    assert_eq!(pixel(pos2(cx - 7.0, cy)), [0x3A, 0x3A, 0x3F]);
    assert_eq!(pixel(pos2(cx + 6.0, cy)), [0x3A, 0x3A, 0x3F]);
    assert_ne!(pixel(pos2(cx + 8.0, cy)), [0x3A, 0x3A, 0x3F]);
}

// ---- the cut, criterion 28 ----

fn p1_picture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../tracyo-core/tests/fixtures/pictures/p1.png")
}

/// Criterion 28 in the app: P1 and the lasso (0, 0), (600, 0), (0, 1000) px Inside, saved: every
/// vertex of a shape cut by the slanted side lies within 0.005 mm of it, and no shape lies outside.
#[test]
fn criterion_28_the_saved_svg_is_cut_on_the_slanted_side() {
    use kurbo::Shape;
    let dir = tempfile::tempdir().unwrap();
    let save = dir.path().join("lasso.svg");
    let mut h = common::harness(dir.path(), Some(p1_picture()), fixed(Some(save.clone())));
    settle(&mut h);
    open_advanced(&mut h);
    h.get_by_label("Method").click();
    h.run_steps(3);
    h.get_by_label("Brightness").click();
    h.run_steps(3);
    settle(&mut h);
    h.get_by_label("Threshold").focus();
    h.run_steps(2);
    press(&mut h, Key::Enter, Modifiers::NONE);
    h.event(Event::Text("70".into()));
    h.run_steps(2);
    press(&mut h, Key::Enter, Modifiers::NONE);
    settle(&mut h);
    button(&mut h, "Lasso");
    set_corners(&mut h, &[(0, 0), (600, 0), (0, 1000)]);
    press(&mut h, Key::Enter, Modifiers::NONE);
    settle(&mut h);
    press(&mut h, Key::S, Modifiers::COMMAND | Modifiers::SHIFT);
    common::wait_until(&mut h, 60.0, |_| save.exists());
    h.run_steps(5);
    let paths = saved_paths(&save);
    assert_ne!(paths.len(), 0);
    let s = 25.4 / 96.0;
    // The signed distance in mm from the slanted side, negative on L1's side.
    let distance =
        |p: kurbo::Point| (1000.0 * p.x + 600.0 * p.y - 600_000.0 * s) / 1000.0_f64.hypot(600.0);
    let vertices: Vec<kurbo::Point> = paths
        .iter()
        .flat_map(|p| {
            p.elements().iter().filter_map(|e| match e {
                kurbo::PathEl::MoveTo(q)
                | kurbo::PathEl::LineTo(q)
                | kurbo::PathEl::CurveTo(_, _, q) => Some(*q),
                _ => None,
            })
        })
        .collect();
    let near: Vec<f64> = vertices
        .iter()
        .map(|p| distance(*p))
        .filter(|d| d.abs() < 0.05)
        .collect();
    println!(
        "{} vertices near the side, worst {:?}",
        near.len(),
        near.iter().map(|d| d.abs()).fold(0.0, f64::max)
    );
    assert!(near.len() >= 2, "the big disc is cut: {}", near.len());
    assert!(near.iter().all(|d| d.abs() <= 0.005), "{near:?}");
    assert!(
        vertices.iter().all(|p| distance(*p) <= 0.005),
        "no shape lies outside L1"
    );
    let bounds = paths
        .iter()
        .map(Shape::bounding_box)
        .reduce(|a, b| a.union(b))
        .unwrap();
    assert!(bounds.x1 < 600.0 * s + 0.005);
}

/// Criterion 22 with a lasso: the same cut gives the same bytes with one worker thread and many.
#[cfg(unix)]
#[test]
fn unix_the_same_lasso_cut_gives_the_same_bytes_with_one_thread_and_with_many() {
    let dir = tempfile::tempdir().unwrap();
    let mut files = Vec::new();
    for (i, threads) in ["1", "1", "4", ""].into_iter().enumerate() {
        let save = dir.path().join(format!("run{i}.svg"));
        let mut h = opened(dir.path(), Some(save.clone()));
        if !threads.is_empty() {
            h.state_mut().wrap_worker(vec![
                "/usr/bin/env".into(),
                format!("RAYON_NUM_THREADS={threads}"),
            ]);
        }
        button(&mut h, "Lasso");
        set_corners(&mut h, &L1);
        press(&mut h, Key::Enter, Modifiers::NONE);
        settle(&mut h);
        press(&mut h, Key::S, Modifiers::COMMAND | Modifiers::SHIFT);
        common::wait_until(&mut h, 60.0, |_| save.exists());
        h.run_steps(5);
        files.push(std::fs::read(&save).unwrap());
    }
    assert!(
        files.windows(2).all(|w| w[0] == w[1]),
        "the bytes differ between runs"
    );
    assert!(
        !saved_paths_of(&files[0]).is_empty(),
        "the cut result has paths"
    );
}

fn saved_paths_of(bytes: &[u8]) -> Vec<kurbo::BezPath> {
    let text = std::str::from_utf8(bytes).unwrap();
    let doc = roxmltree::Document::parse(text).unwrap();
    doc.descendants()
        .filter(|n| n.has_tag_name("path"))
        .map(|n| kurbo::BezPath::from_svg(n.attribute("d").unwrap()).unwrap())
        .collect()
}

/// Criterion 14 with a lasso: the Inside and the Outside file of one slanted lasso fit.
#[test]
fn criterion_14_inside_and_outside_files_of_a_lasso_have_the_area_of_the_whole_trace() {
    use kurbo::Shape;
    let dir = tempfile::tempdir().unwrap();
    let area_of =
        |file: &Path| -> f64 { saved_paths(file).iter().map(Shape::area).sum::<f64>().abs() };
    let mut sizes = Vec::new();
    for (name, lasso, outside) in [
        ("whole", false, false),
        ("inside", true, false),
        ("outside", true, true),
    ] {
        let file = dir.path().join(format!("{name}.svg"));
        let mut h = opened(dir.path(), Some(file.clone()));
        // Brightness with a typed Threshold: the same settings whatever the mask keeps.
        h.get_by_label("Method").click();
        h.run_steps(3);
        h.get_by_label("Brightness").click();
        h.run_steps(3);
        settle(&mut h);
        h.get_by_label("Threshold").focus();
        h.run_steps(2);
        press(&mut h, Key::Enter, Modifiers::NONE);
        h.event(Event::Text("50".into()));
        h.run_steps(2);
        press(&mut h, Key::Enter, Modifiers::NONE);
        settle(&mut h);
        if lasso {
            button(&mut h, "Lasso");
            // A lasso across the block, the circle's edge and the letters of the logo.
            set_corners(&mut h, &[(150, 20), (300, 120), (200, 300), (100, 200)]);
            press(&mut h, Key::Enter, Modifiers::NONE);
            settle(&mut h);
        }
        if outside {
            button(&mut h, "Mask: Outside");
            settle(&mut h);
        }
        press(&mut h, Key::S, Modifiers::COMMAND | Modifiers::SHIFT);
        common::wait_until(&mut h, 60.0, |_| file.exists());
        h.run_steps(5);
        sizes.push(area_of(&file));
    }
    let (all, inside, outside) = (sizes[0], sizes[1], sizes[2]);
    println!(
        "inside {inside} + outside {outside} = {}, whole {all}",
        inside + outside
    );
    assert!((inside + outside - all).abs() <= 0.001 * all);
    assert!(inside > 0.0 && outside > 0.0);
}

// ---- looks C5 to C8 of ux-notes 12.2, by sampled pixels ----

const ACCENT: [u8; 3] = [0x2F, 0x6F, 0xEE];
const GROUND: [u8; 3] = [0xE8, 0xE8, 0xEB];

fn near(a: [u8; 3], b: [u8; 3], tolerance: i32) -> bool {
    a.iter()
        .zip(&b)
        .all(|(x, y)| (i32::from(*x) - i32::from(*y)).abs() <= tolerance)
}

/// The window as an image; with `LOOK_DIR` set it is also saved there as `<name>.png`.
fn render(h: &mut Harness<'_, App>, name: &str) -> RgbImage {
    h.set_render_cursor(false);
    h.run_steps(1);
    let image = compare::rgb(&h.render().unwrap());
    if let Some(dir) = std::env::var_os("LOOK_DIR") {
        image
            .save(Path::new(&dir).join(format!("{name}.png")))
            .unwrap();
    }
    image
}

fn pixel(image: &RgbImage, x: f32, y: f32) -> [u8; 3] {
    image.get_pixel(x.round() as u32, y.round() as u32).0
}

/// A hollow square 14 x 14 with a 1 px accent edge and the canvas ground inside, centred on
/// (`cx`, `cy`).
fn hollow_square(image: &RgbImage, cx: f32, cy: f32) -> bool {
    let at = |dx: f32, dy: f32| pixel(image, cx + dx, cy + dy);
    near(at(0.0, 0.0), GROUND, 6)
        && near(at(-7.0, 0.0), ACCENT, 24)
        && near(at(6.0, 0.0), ACCENT, 24)
        && near(at(0.0, -7.0), ACCENT, 24)
        && near(at(0.0, 6.0), ACCENT, 24)
        && !near(at(8.0, 0.0), ACCENT, 60)
}

/// C5: Lasso pressed, L1 set, the pointer at (90, 380): every corner a hollow 14 x 14 square, the
/// sides and the rubber band 1 px of the accent colour, the ring round the last corner.
#[test]
fn c5_the_lasso_being_drawn_looks_like_the_pen_in_progress() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = opened(dir.path(), None);
    button(&mut h, "Lasso");
    set_corners(&mut h, &L1);
    let pointer = corner(&h, 90.0, 380.0);
    h.event(Event::PointerMoved(pointer));
    h.run_steps(3);
    let image = render(&mut h, "c5");
    for p in L1 {
        let c = at(&h, p);
        assert!(
            hollow_square(&image, c.x.round(), c.y.round()),
            "corner {p:?}"
        );
    }
    // A side: the middle of (60, 270)-(230, 290) is a line of the accent colour within a pixel
    // (a 1 px line between two pixel rows is two half-strong rows).
    let bluish = |p: [u8; 3]| i32::from(p[2]) - i32::from(p[0]) >= 60;
    let mid = corner(&h, 145.0, 280.0);
    assert!(
        (-1..=1).any(|d| bluish(pixel(&image, mid.x, mid.y + d as f32))),
        "the first side is drawn"
    );
    // The rubber band from the last corner (200, 400) to the pointer (90, 380).
    let band = corner(&h, 145.0, 390.0);
    assert!((-1..=1).any(|d| bluish(pixel(&image, band.x, band.y + d as f32))));
    // The ring of 18 px round the last corner, 9 px to its left where no side runs: #D5E2FC on
    // white, here over the veiled picture's white (the corner lies below the triangle).
    let last = at(&h, L1[2]);
    let ring = pixel(&image, last.x.round() - 9.0, last.y.round());
    println!("ring round the last corner {ring:02X?}");
    assert!(near(ring, [0xD5, 0xE2, 0xFC], 6), "{ring:02X?}");
    // The first corner has no ring while the pointer is far from it: white round it.
    let first = at(&h, L1[0]);
    assert_eq!(
        pixel(&image, first.x.round() - 9.0, first.y.round()),
        [255, 255, 255]
    );
}

/// C6: the pointer at (61, 271), over the close target: the ring round the first corner and the
/// closing side dashed from the last corner to the first.
#[test]
fn c6_over_the_close_target_the_first_corner_has_the_ring_and_the_closing_side_is_dashed() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = opened(dir.path(), None);
    button(&mut h, "Lasso");
    set_corners(&mut h, &L1);
    h.event(Event::PointerMoved(corner(&h, 61.0, 271.0)));
    h.run_steps(3);
    let image = render(&mut h, "c6");
    let first = at(&h, L1[0]);
    let ring = pixel(&image, first.x.round() - 9.0, first.y.round());
    println!("ring at the first corner {ring:02X?}");
    assert!(near(ring, [0xD5, 0xE2, 0xFC], 6), "{ring:02X?}");
    // Along the closing side (200, 400) to (60, 270): 6 px on, 4 px off, so about 60 % of its
    // pixels are the accent colour.
    let (a, b) = (at(&h, L1[2]), at(&h, L1[0]));
    let steps = 120;
    let on = (20..steps - 20)
        .filter(|i| {
            let t = *i as f32 / steps as f32;
            let p = a + (b - a) * t;
            (-1..=1).any(|d| near(pixel(&image, p.x, p.y + d as f32), ACCENT, 60))
        })
        .count();
    let share = on as f32 / (steps - 40) as f32;
    println!("dashed closing side: {share:.2} of its pixels are drawn");
    assert!((0.45..0.85).contains(&share), "{share}");
}

/// C7: L1 finished, Inside: a 1 px black line along its three sides, the wash at 12 % outside it,
/// the veiled picture at 35 % inside it.
#[test]
fn c7_a_finished_lasso_has_a_black_line_and_washes_what_is_not_traced() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = opened(dir.path(), None);
    // Method Brightness, as every case of ux-notes 12.2.
    h.get_by_label("Method").click();
    h.run_steps(3);
    h.get_by_label("Brightness").click();
    h.run_steps(3);
    settle(&mut h);
    button(&mut h, "Lasso");
    set_corners(&mut h, &L1);
    press(&mut h, Key::Enter, Modifiers::NONE);
    settle(&mut h);
    button(&mut h, "Lasso");
    h.event(Event::PointerMoved(pos2(1000.0, 700.0)));
    h.run_steps(3);
    let image = render(&mut h, "c7");
    // The line: black, a pixel wide, along the three sides. The traced outline covers it where a
    // shape is cut by a side, so a share of each side's pixels is what is looked for.
    for (a, b) in [(L1[0], L1[1]), (L1[1], L1[2]), (L1[2], L1[0])] {
        let (pa, pb) = (at(&h, a), at(&h, b));
        let steps = 60;
        let black = (3..steps - 3)
            .filter(|i| {
                let p = pa + (pb - pa) * (*i as f32 / steps as f32);
                (-1..=1).any(|d| {
                    near(pixel(&image, p.x, p.y + d as f32), [0, 0, 0], 70)
                        || near(pixel(&image, p.x + d as f32, p.y), [0, 0, 0], 70)
                })
            })
            .count();
        println!("side {a:?}-{b:?}: {black} of {} samples black", steps - 6);
        assert!(black >= 6, "black line along {a:?}-{b:?}: {black}");
    }
    let p = |x: f32, y: f32| {
        let c = corner(&h, x, y);
        pixel(&image, c.x, c.y)
    };
    // The yellow triangle inside L1 at 35 %, outside it at 12 %; the LOGO block at 12 %.
    let inside = p(163.0, 320.0);
    let outside = p(240.0, 375.0);
    let block = p(50.0, 50.0);
    println!("inside {inside:02X?}, outside {outside:02X?}, block {block:02X?}");
    assert!(near(inside, [0xFA, 0xE6, 0xA8], 6), "{inside:02X?}");
    assert!(near(outside, [0xFD, 0xF6, 0xE1], 3), "{outside:02X?}");
    assert!(near(block, [0xE6, 0xEA, 0xF3], 3), "{block:02X?}");
}

/// C8: L1 selected, corner 1 pressed last: three 14 x 14 corner squares, white inside with a 1 px
/// #3A3A3F edge, corner 1 accent edge to edge, the dashed box without handles.
#[test]
fn c8_a_selected_lasso_has_corner_squares_and_the_pressed_corner_is_accent() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = with_l1(dir.path());
    click_px(&mut h, 163.0, 320.0);
    // Press and release corner 1 without moving.
    {
        let p = at(&h, L1[1]);
        click(&mut h, p);
    }
    assert_eq!(h.state().selected_corner(), Some(1));
    h.event(Event::PointerMoved(pos2(1000.0, 700.0)));
    h.run_steps(3);
    let image = render(&mut h, "c8");
    for (i, p) in L1.iter().enumerate() {
        let c = at(&h, *p);
        let (cx, cy) = (c.x.round(), c.y.round());
        let (inside, edge) = (pixel(&image, cx, cy), pixel(&image, cx - 7.0, cy));
        println!("corner {i}: inside {inside:02X?}, edge {edge:02X?}");
        if i == 1 {
            assert!(
                near(inside, ACCENT, 6) && near(edge, ACCENT, 6),
                "selected corner"
            );
        } else {
            assert!(near(inside, [255, 255, 255], 6), "corner {i} inside");
            assert!(near(edge, [0x3A, 0x3A, 0x3F], 12), "corner {i} edge");
        }
    }
}
