//! Colours, text and shapes per part with the real app and worker: Multicolour in two parts, each
//! part's own automatic values, text on in one part and off in the other, shapes after the cut
//! (criteria 17 to 19 of specification 0002).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]

mod common;

use std::path::{Path, PathBuf};

use eframe::egui::{Event, Modifiers, PointerButton, Pos2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use tracyo_app::App;
use tracyo_app::dialogs::Dialogs;
use tracyo_core::analysis::Analysis;
use tracyo_core::clip::Clip;
use tracyo_core::picture::Picture;
use tracyo_core::regions::{MaskMode, PxRect, Region};
use tracyo_core::request::pixel_map;

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../tracyo-core/tests/fixtures/pictures")
        .join(name)
}

fn opened<'a>(dir: &Path, picture: PathBuf, save: Option<PathBuf>) -> Harness<'a, App> {
    let dialogs = Dialogs::Fixed { open: None, save };
    let mut h = common::harness_sized(dir, Some(picture), dialogs, 1280.0);
    common::wait_for_result(&mut h, 120.0);
    h.run_steps(3);
    if h.query_by_label("More trace options").is_some() {
        button(&mut h, "More trace options");
    }
    h
}

fn button(h: &mut Harness<'_, App>, label: &str) {
    h.get_by_label(label).click();
    h.run_steps(3);
    h.remove_cursor();
    h.run_steps(2);
}

/// Clicks the chip of part `index` (0 for part 1), named as the list names it now.
fn click_chip(h: &mut Harness<'_, App>, index: usize) {
    let mut name = h.state().chip_name(index).unwrap();
    if h.state().active_part() == index {
        name.push_str(", active");
    }
    button(h, &name);
    settle(h);
}

fn settle(h: &mut Harness<'_, App>) {
    common::wait_until(h, 60.0, |a| !a.is_analysing());
    common::wait_for_result(h, 120.0);
    h.run_steps(3);
}

fn corner(h: &Harness<'_, App>, x: f32, y: f32) -> Pos2 {
    h.state().picture_corner_on_screen(x, y).unwrap()
}

/// Draws a rectangle with the Rectangle tool and releases the tool.
fn draw(h: &mut Harness<'_, App>, (x0, y0, x1, y1): (f32, f32, f32, f32)) {
    button(h, "Rectangle");
    let (from, to) = (corner(h, x0, y0), corner(h, x1, y1));
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
    button(h, "Rectangle");
    settle(h);
}

fn save_to(h: &mut Harness<'_, App>, out: &Path) -> String {
    let _ = std::fs::remove_file(out);
    h.get_by_label("Save as\u{2026}").click();
    common::wait_until(h, 120.0, |_| out.exists());
    h.run_steps(2);
    std::fs::read_to_string(out).unwrap()
}

fn elements(svg: &str, name: &str) -> usize {
    let doc = roxmltree::Document::parse(svg).unwrap();
    doc.descendants().filter(|n| n.has_tag_name(name)).count()
}

/// Criterion 18: part 1 (the whole picture) keeps the left half of P2 and part 2 the right half;
/// each reads only its own pixels for the automatic Threshold and the Auto background.
#[test]
fn criterion_18_each_part_reads_its_own_area() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = opened(dir.path(), fixture("p2.png"), None);
    let picture = Picture::decode(&std::fs::read(fixture("p2.png")).unwrap()).unwrap();
    let analysis_of = |mode: MaskMode| {
        let clip = Clip::of_regions(
            &[Region::rect(PxRect::new(500, 0, 1000, 1000), false)],
            mode,
            pixel_map(1000, 1000),
        )
        .unwrap()
        .unwrap();
        let kept = clip.kept_pixels(pixel_map(1000, 1000), 1000, 1000).unwrap();
        Analysis::of_kept(&picture, &kept)
    };
    let (left, right) = (
        analysis_of(MaskMode::Outside),
        analysis_of(MaskMode::Inside),
    );
    assert_ne!(left.threshold, right.threshold, "the halves differ");

    button(&mut h, "Add part");
    draw(&mut h, (500.0, 0.0, 1000.0, 1000.0));
    assert_eq!(h.state().part_count(), 2);
    assert_eq!(h.state().active_part(), 1);
    assert_eq!(
        h.state().auto_background(),
        Some(vectrace::Rgb8::new(0xD8, 0xD8, 0xD8))
    );
    assert_eq!(
        h.state().picture_values().unwrap().threshold,
        right.threshold
    );

    // Part 1's area is the left half now: the later part took the right one.
    click_chip(&mut h, 0);
    assert_eq!(h.state().active_part(), 0);
    assert_eq!(
        h.state().auto_background(),
        Some(vectrace::Rgb8::new(255, 255, 255))
    );
    assert_eq!(
        h.state().picture_values().unwrap().threshold,
        left.threshold
    );

    // Removing part 2 gives part 1 the whole picture's values back.
    click_chip(&mut h, 1);
    button(&mut h, "Remove part 2");
    std::thread::sleep(std::time::Duration::from_millis(450));
    button(&mut h, "Press again to remove part 2");
    settle(&mut h);
    assert_eq!(h.state().part_count(), 1);
    let whole = Analysis::of(&picture);
    assert_eq!(h.state().auto_background(), whole.suggested_background);
    assert_eq!(
        h.state().picture_values().unwrap().threshold,
        whole.threshold
    );
}

/// Criterion 17: Multicolour in both parts saves one layer per colour and one path per layer.
#[test]
fn criterion_17_two_multicolour_parts_save_one_layer_per_colour() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("colours.svg");
    let mut h = opened(dir.path(), common::logo(), Some(out.clone()));
    button(&mut h, "Output: Multicolour");
    common::wait_for_result_where(&mut h, 60.0, |a| !a.palette_colours().is_empty());
    h.run_steps(3);
    let single = save_to(&mut h, &out);
    let single_labels = labels(&single);
    assert!(single_labels.len() >= 3, "{single_labels:?}");

    button(&mut h, "Add part");
    draw(&mut h, (0.0, 0.0, 330.0, 560.0));
    assert!(h.state().part_settings(1).unwrap().multicolour);
    let joined = save_to(&mut h, &out);
    let layers = labels(&joined);
    println!("single {single_labels:?}\njoined {layers:?}");
    for (i, l) in layers.iter().enumerate() {
        assert!(!layers[i + 1..].contains(l), "{l} has two layers");
    }
    assert_eq!(
        elements(&joined, "path"),
        layers.len(),
        "one path per layer, no layer per part"
    );
    assert!(
        layers.iter().all(|l| single_labels.contains(l)),
        "no colour the picture has not"
    );
}

fn labels(svg: &str) -> Vec<String> {
    let doc = roxmltree::Document::parse(svg).unwrap();
    doc.root_element()
        .children()
        .filter(|n| n.is_element() && n.tag_name().name() == "g")
        .filter_map(|g| {
            g.attribute(("http://www.inkscape.org/namespaces/inkscape", "label"))
                .map(str::to_owned)
        })
        .collect()
}

/// Criterion 19: Recognise text on in part 1 and off in part 2 (a box round the big word): the
/// big word stays paths of part 2, the small lines are `<text>` of part 1.
#[test]
fn criterion_19_text_in_one_part_and_not_in_the_other() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("text.svg");
    let mut h = opened(dir.path(), fixture("precision.png"), Some(out.clone()));
    button(&mut h, "Recognise text");
    common::wait_until(&mut h, 120.0, |a| a.recognised().0 >= 3 && !a.is_reading());
    settle(&mut h);
    let all = save_to(&mut h, &out);
    assert!(all.contains("PRAZISION") || all.contains("PR\u{c4}ZISION"));

    button(&mut h, "Add part");
    assert!(h.state().part_settings(1).unwrap().recognise_text);
    button(&mut h, "Recognise text");
    settle(&mut h);
    draw(&mut h, (590.0, 380.0, 1250.0, 500.0));
    let text = save_to(&mut h, &out);
    assert!(text.contains("BERATUNG"), "the small lines stay text");
    assert!(
        !text.contains("PRAZISION") && !text.contains("PR\u{c4}ZISION"),
        "the big word is paths of part 2"
    );
    assert!(elements(&text, "text") >= 3);
    // Part 1 reads text, part 2 does not, and each keeps its own toggle.
    assert!(h.state().part_settings(0).unwrap().recognise_text);
    assert!(!h.state().part_settings(1).unwrap().recognise_text);
}

/// Criterion 19: a rectangle cut by a seam is a path, one wholly inside its part is a `<rect>`.
#[test]
fn criterion_19_shapes_are_recognised_after_the_cut() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("shapes.svg");
    let mut h = opened(dir.path(), fixture("shapes.png"), Some(out.clone()));
    button(&mut h, "Recognise shapes");
    settle(&mut h);
    let whole = save_to(&mut h, &out);
    assert!(elements(&whole, "rect") >= 1, "the square is a rectangle");

    // Part 2 covers the left half of the square (x 10 to 40): the seam goes through it.
    button(&mut h, "Add part");
    draw(&mut h, (0.0, 0.0, 25.0, 64.0));
    let cut = save_to(&mut h, &out);
    assert_eq!(elements(&cut, "rect"), 0, "a cut square is a path");
    assert!(
        elements(&cut, "circle") + elements(&cut, "ellipse") >= 1,
        "the ring, wholly in part 1's area, is still a circle"
    );
}
