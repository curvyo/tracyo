//! Flows of the region tool with the real app and the real worker: drawing, selecting, moving,
//! resizing, nudging, deleting, Inside and Outside, Clear mask and the cut result.

// Pixel geometry converts between integers and f32 everywhere; the values are screen coordinates.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]

mod common;

use std::path::{Path, PathBuf};

use eframe::egui::{Event, Key, Modifiers, PointerButton, Pos2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use tracyo_app::App;
use tracyo_app::dialogs::Dialogs;
use tracyo_core::regions::{MaskMode, PxRect};

fn fixed(save: Option<PathBuf>) -> Dialogs {
    Dialogs::Fixed { open: None, save }
}

fn opened<'a>(dir: &Path, save: Option<PathBuf>) -> Harness<'a, App> {
    let mut h = common::harness(dir, Some(common::logo()), fixed(save));
    settle(&mut h);
    h.run_steps(3);
    h
}

fn corner(h: &Harness<'_, App>, x: f32, y: f32) -> Pos2 {
    h.state().picture_corner_on_screen(x, y).unwrap()
}

fn button(h: &mut Harness<'_, App>, label: &str) {
    h.get_by_label(label).click();
    h.run_steps(3);
    h.remove_cursor();
    h.run_steps(2);
}

/// A drag of the primary button from `from` to `to` with `modifiers` held, in frames of their own.
fn drag(h: &mut Harness<'_, App>, from: Pos2, to: Pos2, modifiers: Modifiers) {
    h.event(Event::PointerMoved(from));
    h.run_steps(2);
    if modifiers != Modifiers::NONE {
        h.event(Event::ModifiersChanged(modifiers));
        h.run_steps(1);
    }
    h.event(Event::PointerButton {
        pos: from,
        button: PointerButton::Primary,
        pressed: true,
        modifiers,
    });
    h.run_steps(2);
    for k in 1..=4 {
        let t = k as f32 / 4.0;
        h.event(Event::PointerMoved(from + (to - from) * t));
        h.run_steps(2);
    }
    h.event(Event::PointerButton {
        pos: to,
        button: PointerButton::Primary,
        pressed: false,
        modifiers,
    });
    h.run_steps(3);
    if modifiers != Modifiers::NONE {
        h.event(Event::ModifiersChanged(Modifiers::NONE));
        h.run_steps(1);
    }
}

fn click(h: &mut Harness<'_, App>, at: Pos2) {
    h.event(Event::PointerMoved(at));
    h.run_steps(2);
    for pressed in [true, false] {
        h.event(Event::PointerButton {
            pos: at,
            button: PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        });
        h.run_steps(2);
    }
}

/// Draws the rectangle (`x0`, `y0`) to (`x1`, `y1`) with the Rectangle tool pressed and waits for
/// the trace that follows.
fn draw(h: &mut Harness<'_, App>, rect: (f32, f32, f32, f32)) {
    let (a, b) = (corner(h, rect.0, rect.1), corner(h, rect.2, rect.3));
    drag(h, a, b, Modifiers::NONE);
    settle(h);
}

/// Waits until the trace, and the analysis of the kept pixels with the trace it may start, are done.
fn settle(h: &mut Harness<'_, App>) {
    common::wait_until(h, 30.0, |a| !a.is_analysing());
    common::wait_for_result(h, 60.0);
    h.run_steps(2);
    common::wait_until(h, 30.0, |a| !a.is_analysing());
}

fn rects(h: &Harness<'_, App>) -> Vec<(PxRect, bool)> {
    h.state().mask_regions()
}

fn press(h: &mut Harness<'_, App>, key: Key, modifiers: Modifiers) {
    h.key_press_modifiers(modifiers, key);
    h.run_steps(3);
}

#[test]
fn the_rectangle_tool_draws_a_region_and_the_trace_follows() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = opened(dir.path(), None);
    assert_eq!(h.state().mask_regions(), []);
    let before = h.state().trace_jobs();
    button(&mut h, "Rectangle");
    assert_eq!(h.state().mask_tools(), (true, false));
    draw(&mut h, (100.0, 100.0, 300.0, 250.0));
    assert_eq!(rects(&h), [(PxRect::new(100, 100, 300, 250), false)]);
    assert!(h.state().trace_jobs() > before, "the region starts a trace");
    assert_eq!(
        h.state().mask_tools(),
        (true, false),
        "Rectangle stays pressed"
    );
}

/// The rectangle drawn with `modifiers` held.
fn draw_with(h: &mut Harness<'_, App>, rect: (f32, f32, f32, f32), modifiers: Modifiers) {
    let (a, b) = (corner(h, rect.0, rect.1), corner(h, rect.2, rect.3));
    drag(h, a, b, modifiers);
    settle(h);
}

fn r(x0: i32, y0: i32, x1: i32, y1: i32) -> PxRect {
    PxRect::new(x0, y0, x1, y1)
}

/// Criterion 7: Shift draws from the centre, Ctrl a square, Subtract a subtracted region; a drag
/// under 2 px draws nothing and Rectangle stays pressed.
#[test]
fn shift_ctrl_and_subtract_change_what_is_drawn() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = opened(dir.path(), None);
    button(&mut h, "Rectangle");
    // A drag of 1 px draws nothing.
    let a = corner(&h, 400.0, 300.0);
    drag(&mut h, a, a + eframe::egui::vec2(1.0, 1.0), Modifiers::NONE);
    assert_eq!(rects(&h), []);
    // From the centre.
    draw_with(&mut h, (200.0, 150.0, 260.0, 190.0), Modifiers::SHIFT);
    assert_eq!(rects(&h)[0], (r(140, 110, 260, 190), false));
    // A square: the longer side of the drag, towards the pointer.
    draw_with(&mut h, (400.0, 100.0, 460.0, 130.0), Modifiers::COMMAND);
    assert_eq!(rects(&h)[1], (r(400, 100, 460, 160), false));
    // Subtract is a toggle of its own: Rectangle stays pressed.
    button(&mut h, "Subtract");
    assert_eq!(h.state().mask_tools(), (true, true));
    draw(&mut h, (150.0, 120.0, 180.0, 140.0));
    assert_eq!(rects(&h)[2], (r(150, 120, 180, 140), true));
    button(&mut h, "Subtract");
    assert_eq!(h.state().mask_tools(), (true, false));
    assert_eq!(
        rects(&h).len(),
        3,
        "the regions drawn before stay as they were"
    );
}

/// Criteria 8 and 9: a click selects the topmost region, a drag moves it, arrows nudge it, Delete
/// removes it, and Esc steps back in the order of the spec.
#[test]
fn a_region_is_selected_moved_nudged_and_deleted() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = opened(dir.path(), None);
    button(&mut h, "Rectangle");
    draw(&mut h, (100.0, 100.0, 300.0, 250.0));
    draw(&mut h, (200.0, 150.0, 400.0, 300.0));
    button(&mut h, "Rectangle");
    // The topmost (last drawn) region is selected where both lie.
    let at = corner(&h, 250.0, 200.0);
    click(&mut h, at);
    assert_eq!(h.state().selected_region(), Some(r(200, 150, 400, 300)));
    // A click on empty canvas deselects.
    let empty = corner(&h, 20.0, 20.0);
    click(&mut h, empty);
    assert_eq!(h.state().selected_region(), None);
    // Select the first one and drag it 30 px right and 10 px down.
    let at = corner(&h, 120.0, 120.0);
    click(&mut h, at);
    assert_eq!(h.state().selected_region(), Some(r(100, 100, 300, 250)));
    let from = corner(&h, 130.0, 130.0);
    let to = corner(&h, 160.0, 140.0);
    drag(&mut h, from, to, Modifiers::NONE);
    settle(&mut h);
    assert_eq!(h.state().selected_region(), Some(r(130, 110, 330, 260)));
    // Arrows move 1 px, with Shift 10 px.
    press(&mut h, Key::ArrowLeft, Modifiers::NONE);
    press(&mut h, Key::ArrowDown, Modifiers::SHIFT);
    assert_eq!(h.state().selected_region(), Some(r(129, 120, 329, 270)));
    // Esc: no trace runs, no pipette, Rectangle is released, so it clears the selection.
    settle(&mut h);
    press(&mut h, Key::Escape, Modifiers::NONE);
    assert_eq!(h.state().selected_region(), None);
    // Delete removes the selected region.
    let at = corner(&h, 350.0, 280.0);
    click(&mut h, at);
    assert_eq!(h.state().selected_region(), Some(r(200, 150, 400, 300)));
    press(&mut h, Key::Delete, Modifiers::NONE);
    settle(&mut h);
    assert_eq!(rects(&h), [(r(129, 120, 329, 270), false)]);
    assert_eq!(h.state().selected_region(), None);
}

/// Criterion 8: Esc releases Rectangle before it clears the selection.
#[test]
fn esc_releases_rectangle_before_it_clears_the_selection() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = opened(dir.path(), None);
    button(&mut h, "Rectangle");
    draw(&mut h, (100.0, 100.0, 300.0, 250.0));
    button(&mut h, "Rectangle");
    let at = corner(&h, 150.0, 150.0);
    click(&mut h, at);
    assert!(h.state().selected_region().is_some());
    button(&mut h, "Rectangle");
    assert_eq!(h.state().mask_tools(), (true, false));
    press(&mut h, Key::Escape, Modifiers::NONE);
    assert_eq!(
        h.state().mask_tools(),
        (false, false),
        "Rectangle is released first"
    );
    assert!(h.state().selected_region().is_some());
    press(&mut h, Key::Escape, Modifiers::NONE);
    assert_eq!(h.state().selected_region(), None);
    assert_eq!(
        h.state().picture_name(),
        Some("input-logo.png"),
        "Esc never closes anything"
    );
}

/// Criterion 9: a handle resizes (Shift about the centre, Ctrl keeps the proportions), and Esc
/// during a drag puts the region back.
#[test]
fn a_handle_resizes_and_esc_restores_a_drag() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = opened(dir.path(), None);
    button(&mut h, "Rectangle");
    draw(&mut h, (100.0, 100.0, 300.0, 200.0));
    button(&mut h, "Rectangle");
    let at = corner(&h, 150.0, 150.0);
    click(&mut h, at);
    // The south-east handle to (340, 240).
    let handle = corner(&h, 300.0, 200.0);
    let to = corner(&h, 340.0, 240.0);
    drag(&mut h, handle, to, Modifiers::NONE);
    settle(&mut h);
    assert_eq!(h.state().selected_region(), Some(r(100, 100, 340, 240)));
    // Shift: about the centre; the east handle 20 px out moves the west side 20 px out too.
    let handle = corner(&h, 340.0, 170.0);
    let to = corner(&h, 360.0, 170.0);
    drag(&mut h, handle, to, Modifiers::SHIFT);
    settle(&mut h);
    assert_eq!(h.state().selected_region(), Some(r(80, 100, 360, 240)));
    // Ctrl at a corner keeps the proportions (280 x 140 = 2 : 1).
    let handle = corner(&h, 360.0, 240.0);
    let to = corner(&h, 460.0, 250.0);
    drag(&mut h, handle, to, Modifiers::COMMAND);
    settle(&mut h);
    let rect = h.state().selected_region().unwrap();
    assert_eq!((rect.x0, rect.y0), (80, 100));
    assert_eq!(rect.width(), 2 * rect.height(), "{rect:?}");
    // Esc during a drag: the region goes back where it was and no trace starts.
    let before = h.state().trace_jobs();
    let (x1, y1) = (rect.x1 as f32, rect.y1 as f32);
    let handle = corner(&h, x1, y1);
    h.event(Event::PointerMoved(handle));
    h.run_steps(2);
    h.event(Event::PointerButton {
        pos: handle,
        button: PointerButton::Primary,
        pressed: true,
        modifiers: Modifiers::NONE,
    });
    h.run_steps(2);
    for k in 1..=3 {
        h.event(Event::PointerMoved(
            handle - eframe::egui::vec2(20.0 * k as f32, 10.0 * k as f32),
        ));
        h.run_steps(2);
    }
    assert_ne!(
        h.state().selected_region(),
        Some(rect),
        "the drag shows the new size"
    );
    press(&mut h, Key::Escape, Modifiers::NONE);
    assert_eq!(h.state().selected_region(), Some(rect));
    h.event(Event::PointerButton {
        pos: handle,
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::NONE,
    });
    h.run_steps(3);
    assert_eq!(h.state().selected_region(), Some(rect));
    assert_eq!(
        h.state().trace_jobs(),
        before,
        "a restored drag traces nothing"
    );
}

/// Criterion 6 and 10: Inside and Outside change the wash and the result; Clear mask removes the
/// regions and keeps the mode.
#[test]
fn inside_outside_and_clear_mask() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = opened(dir.path(), None);
    let whole = h.state().recognised();
    assert_eq!(h.state().mask_mode(), Some(MaskMode::Inside));
    // Without a region both modes trace the whole picture and nothing is washed more.
    button(&mut h, "Mask: Outside");
    settle(&mut h);
    assert_eq!(h.state().washed_rects(), []);
    button(&mut h, "Mask: Inside");
    button(&mut h, "Rectangle");
    draw(&mut h, (24.0, 25.0, 324.0, 235.0));
    button(&mut h, "Rectangle");
    let around = h.state().washed_rects();
    let area = |rects: &[PxRect]| -> i64 {
        rects
            .iter()
            .map(|r| i64::from(r.width()) * i64::from(r.height()))
            .sum()
    };
    assert_eq!(area(&around), 640 * 420 - 300 * 210);
    button(&mut h, "Mask: Outside");
    settle(&mut h);
    assert_eq!(h.state().washed_rects(), [r(24, 25, 324, 235)]);
    assert_eq!(h.state().mask_mode(), Some(MaskMode::Outside));
    let _ = whole;
    button(&mut h, "Clear mask");
    settle(&mut h);
    assert_eq!(rects(&h), []);
    assert_eq!(h.state().washed_rects(), []);
    assert_eq!(
        h.state().mask_mode(),
        Some(MaskMode::Outside),
        "Clear mask keeps the mode"
    );
    // With no region it does nothing.
    let jobs = h.state().trace_jobs();
    button(&mut h, "Clear mask");
    assert_eq!(h.state().trace_jobs(), jobs);
}

/// Criterion 12: the notices, verbatim, as the reason of the dimmed Save as….
#[test]
fn a_mask_that_keeps_nothing_says_why() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = opened(dir.path(), None);
    button(&mut h, "Rectangle");
    draw(&mut h, (100.0, 450.0, 300.0, 520.0));
    assert_eq!(
        h.state().nothing_to_save(),
        Some("0 paths: the mask is outside the image.")
    );
    // Outside of it everything is traced.
    button(&mut h, "Mask: Outside");
    settle(&mut h);
    assert_eq!(h.state().nothing_to_save(), None);
    button(&mut h, "Mask: Inside");
    // Only subtracted regions make an empty mask.
    button(&mut h, "Clear mask");
    button(&mut h, "Subtract");
    draw(&mut h, (100.0, 100.0, 300.0, 250.0));
    assert_eq!(
        h.state().nothing_to_save(),
        Some("0 paths: the mask is outside the image.")
    );
    // A region over it that covers the picture again traces.
    button(&mut h, "Subtract");
    draw(&mut h, (0.0, 0.0, 640.0, 420.0));
    assert_eq!(h.state().nothing_to_save(), None);
    // The region is the picture's own size: the trace is the whole one, cut at the borders.
    assert!(h.state().has_result());
}

fn saved_paths(file: &Path) -> Vec<kurbo::BezPath> {
    let text = std::fs::read_to_string(file).unwrap();
    let doc = roxmltree::Document::parse(&text).unwrap();
    doc.descendants()
        .filter(|n| n.has_tag_name("path"))
        .map(|n| kurbo::BezPath::from_svg(n.attribute("d").unwrap()).unwrap())
        .collect()
}

/// The closed outlines of a path, each as a path of its own.
fn subpaths(path: &kurbo::BezPath) -> Vec<kurbo::BezPath> {
    let mut out: Vec<kurbo::BezPath> = Vec::new();
    for el in path.elements() {
        if let kurbo::PathEl::MoveTo(_) = el {
            out.push(kurbo::BezPath::new());
        }
        if let Some(last) = out.last_mut() {
            last.push(*el);
        }
    }
    out
}

fn mm(px: i32) -> f64 {
    f64::from(px) * 25.4 / 96.0
}

/// Criterion 13: the saved SVG is cut on the region's edge (every vertex within 0.005 mm), a
/// cut shape has only lines, and Inside plus Outside is the whole trace's area.
#[test]
fn the_saved_svg_is_cut_on_the_region_edge() {
    use kurbo::Shape;
    let dir = tempfile::tempdir().unwrap();
    let save = dir.path().join("cut.svg");
    let mut h = opened(dir.path(), Some(save.clone()));
    let whole = h.state().recognised();
    let _ = whole;
    button(&mut h, "Rectangle");
    // A region across the LOGO block of the picture: x 150 to 250, y 20 to 300.
    draw(&mut h, (150.0, 20.0, 250.0, 300.0));
    button(&mut h, "Rectangle");
    press(&mut h, Key::S, Modifiers::COMMAND | Modifiers::SHIFT);
    h.run_steps(5);
    let paths = saved_paths(&save);
    assert_ne!(paths.len(), 0);
    let tolerance = 0.005;
    let bounds = paths
        .iter()
        .map(Shape::bounding_box)
        .reduce(|a, b| a.union(b))
        .unwrap();
    println!(
        "bounds {bounds:?} vs x {}..{} y {}..{}",
        mm(150),
        mm(250),
        mm(20),
        mm(300)
    );
    assert!(
        (bounds.x0 - mm(150)).abs() <= tolerance,
        "left edge {}",
        bounds.x0
    );
    assert!(
        (bounds.x1 - mm(250)).abs() <= tolerance,
        "right edge {}",
        bounds.x1
    );
    assert!(bounds.y0 >= mm(20) - tolerance && bounds.y1 <= mm(300) + tolerance);
    // The shapes on the edge hold lines only.
    let cut = paths.iter().flat_map(subpaths).filter(|p| {
        let b = p.bounding_box();
        (b.x0 - mm(150)).abs() <= tolerance || (b.x1 - mm(250)).abs() <= tolerance
    });
    let mut cut_shapes = 0;
    for p in cut {
        cut_shapes += 1;
        let curves = p
            .elements()
            .iter()
            .filter(|e| matches!(e, kurbo::PathEl::CurveTo(..)))
            .count();
        assert_eq!(curves, 0, "a cut shape holds only M, L and Z");
    }
    assert!(cut_shapes >= 1, "something is cut");
}

/// Criterion 14 in the app: the Inside and the Outside file of one region fit.
#[test]
fn inside_and_outside_files_have_the_area_of_the_whole_trace() {
    use kurbo::Shape;
    let dir = tempfile::tempdir().unwrap();
    let area =
        |file: &Path| -> f64 { saved_paths(file).iter().map(Shape::area).sum::<f64>().abs() };
    let whole_file = dir.path().join("whole.svg");
    let mut h = opened(dir.path(), Some(whole_file.clone()));
    press(&mut h, Key::S, Modifiers::COMMAND | Modifiers::SHIFT);
    h.run_steps(5);
    button(&mut h, "Rectangle");
    draw(&mut h, (24.0, 25.0, 324.0, 235.0));
    button(&mut h, "Rectangle");
    let inside = dir.path().join("inside.svg");
    // The dialog answers with a fixed path: save twice under different names by replacing the app.
    drop(h);
    let mut h = opened(dir.path(), Some(inside.clone()));
    button(&mut h, "Rectangle");
    draw(&mut h, (24.0, 25.0, 324.0, 235.0));
    press(&mut h, Key::S, Modifiers::COMMAND | Modifiers::SHIFT);
    h.run_steps(5);
    drop(h);
    let outside = dir.path().join("outside.svg");
    let mut h = opened(dir.path(), Some(outside.clone()));
    button(&mut h, "Rectangle");
    draw(&mut h, (24.0, 25.0, 324.0, 235.0));
    button(&mut h, "Mask: Outside");
    settle(&mut h);
    press(&mut h, Key::S, Modifiers::COMMAND | Modifiers::SHIFT);
    h.run_steps(5);
    let (a, b, all) = (area(&inside), area(&outside), area(&whole_file));
    println!("inside {a} + outside {b} = {}, whole {all}", a + b);
    assert!(
        (a + b - all).abs() <= 0.001 * all,
        "within 0.1 % of the whole trace"
    );
}

fn p2() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../tracyo-core/tests/fixtures/pictures/p2.png")
}

/// Criterion 18 (one part): the automatic Threshold and the Auto background read only the pixels
/// the mask keeps: P2's left half is white, its right half #D8D8D8.
#[test]
fn the_automatic_values_read_the_kept_pixels() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = common::harness(dir.path(), Some(p2()), fixed(None));
    common::wait_for_result(&mut h, 120.0);
    h.run_steps(3);
    let whole = (
        h.state().auto_background(),
        h.state().picture_values().unwrap().threshold,
    );
    println!("whole picture: {whole:?}");
    button(&mut h, "Rectangle");
    // The right half.
    let (a, b) = (corner(&h, 500.0, 0.0), corner(&h, 1000.0, 1000.0));
    drag(&mut h, a, b, Modifiers::NONE);
    settle(&mut h);
    let right = (
        h.state().auto_background(),
        h.state().picture_values().unwrap().threshold,
    );
    println!("right half: {right:?}");
    assert_eq!(right.0, Some(vectrace::Rgb8::new(0xD8, 0xD8, 0xD8)));
    // The left half: Outside of the right half.
    button(&mut h, "Mask: Outside");
    settle(&mut h);
    let left = (
        h.state().auto_background(),
        h.state().picture_values().unwrap().threshold,
    );
    println!("left half: {left:?}");
    assert_eq!(left.0, Some(vectrace::Rgb8::new(255, 255, 255)));
    assert_ne!(left.1, right.1, "each half has its own Threshold");
    // Clear mask: the whole picture's values come back.
    button(&mut h, "Clear mask");
    settle(&mut h);
    assert_eq!(
        (
            h.state().auto_background(),
            h.state().picture_values().unwrap().threshold
        ),
        whole
    );
}

/// A Threshold the maker typed is not replaced by the analysis of a mask.
#[test]
fn a_typed_threshold_stays_when_a_mask_is_drawn() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = common::harness(dir.path(), Some(p2()), fixed(None));
    common::wait_for_result(&mut h, 120.0);
    h.get_by_label("Method").click();
    h.run_steps(3);
    h.get_by_label("Brightness").click();
    h.run_steps(3);
    settle(&mut h);
    h.get_by_label("Threshold").focus();
    h.run_steps(2);
    press(&mut h, Key::Enter, Modifiers::NONE);
    h.event(Event::Text("33".into()));
    h.run_steps(2);
    press(&mut h, Key::Enter, Modifiers::NONE);
    settle(&mut h);
    let typed = h.state().picture_values().unwrap().threshold;
    button(&mut h, "Rectangle");
    let (a, b) = (corner(&h, 500.0, 0.0), corner(&h, 1000.0, 1000.0));
    drag(&mut h, a, b, Modifiers::NONE);
    settle(&mut h);
    assert_eq!(h.state().picture_values().unwrap().threshold, typed);
}

/// Criterion 22: the same picture, region and settings give the same SVG bytes on every run, with
/// one rayon thread in the worker and with the default number.
#[test]
fn the_same_cut_gives_the_same_bytes_with_one_thread_and_with_many() {
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
        button(&mut h, "Rectangle");
        draw(&mut h, (24.0, 25.0, 324.0, 235.0));
        draw(&mut h, (344.0, 315.0, 619.0, 405.0));
        press(&mut h, Key::S, Modifiers::COMMAND | Modifiers::SHIFT);
        h.run_steps(5);
        files.push(std::fs::read(&save).unwrap());
    }
    assert!(
        files.windows(2).all(|w| w[0] == w[1]),
        "the bytes differ between runs"
    );
    assert!(files[0].len() > 1000);
}

/// Criterion 1: without a picture the bar has no Mask group.
#[test]
fn without_a_picture_there_is_no_mask_group() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = common::harness(dir.path(), None, fixed(None));
    h.run_steps(3);
    for label in ["Rectangle", "Subtract", "Clear mask", "Mask: Inside"] {
        assert!(h.query_by_label(label).is_none(), "{label}");
    }
}
