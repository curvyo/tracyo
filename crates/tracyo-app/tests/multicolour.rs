//! Multicolour: Output, Colours, the palette with exclude and delete, the palette pipette and the
//! SVG with one layer per colour, with the real worker.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use std::path::Path;

use eframe::egui::{Event, Key, Modifiers, PointerButton};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use tracyo_app::App;
use tracyo_app::dialogs::Dialogs;
use tracyo_core::palette::swatch_label;
use vectrace::Rgb8;

fn opened<'a>(dir: &Path, save: Option<std::path::PathBuf>) -> Harness<'a, App> {
    let dialogs = Dialogs::Fixed {
        open: Some(common::logo()),
        save,
    };
    let mut h = common::harness(dir, Some(common::logo()), dialogs);
    common::wait_for_result(&mut h, 60.0);
    h.run_steps(3);
    h
}

fn settle(h: &mut Harness<'_, App>) {
    common::wait_for_result(h, 60.0);
    h.run_steps(3);
}

/// Opens Advanced and chooses Multicolour; waits for the palette.
fn multicolour<'a>(dir: &Path, save: Option<std::path::PathBuf>) -> Harness<'a, App> {
    let mut h = opened(dir, save);
    h.get_by_label("More trace options").click();
    h.run_steps(3);
    h.get_by_label("Output: Multicolour").click();
    h.run_steps(3);
    common::wait_for_result_where(&mut h, 60.0, |a| !a.palette_colours().is_empty());
    h.run_steps(3);
    h
}

fn ctrl_click(h: &mut Harness<'_, App>, colour: Rgb8, excluded: bool) {
    common::ctrl_click(h, &swatch_label(colour, excluded));
}

fn layer_labels(path: &Path) -> Vec<String> {
    let text = std::fs::read_to_string(path).unwrap();
    let doc = roxmltree::Document::parse(&text).unwrap();
    doc.root_element()
        .children()
        .filter(|n| n.is_element() && n.tag_name().name() == "g")
        .map(|g| {
            g.attribute(("http://www.inkscape.org/namespaces/inkscape", "label"))
                .unwrap()
                .to_owned()
        })
        .collect()
}

fn hexes(colours: &[Rgb8]) -> Vec<String> {
    colours
        .iter()
        .map(|c| tracyo_core::method::hex(*c))
        .collect()
}

/// Criterion 14: Output Multicolour shows Colours at Auto and the palette, largest area first.
#[test]
fn multicolour_shows_the_palette_largest_area_first() {
    let dir = tempfile::tempdir().unwrap();
    let h = multicolour(dir.path(), None);
    assert!(h.state().settings().multicolour);
    let colours = h.state().palette_colours();
    assert_eq!(colours.len(), 4, "the logo has four colours: {colours:?}");
    assert_eq!(h.state().settings().colours, 0, "Colours starts at Auto");
    assert_eq!(h.state().nothing_to_save(), None);
    for c in &colours {
        assert!(h.query_by_label(&swatch_label(*c, false)).is_some());
    }
}

/// Criterion 14: Colours runs Auto, 2 to 16; Home is Auto, an arrow adds one colour.
#[test]
fn the_colours_field_sets_a_count_and_auto_again() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = multicolour(dir.path(), None);
    h.get_by_label("Colours").focus();
    h.run_steps(2);
    h.key_press(Key::ArrowDown);
    h.run_steps(2);
    assert_eq!(h.state().settings().colours, 0, "Auto is the lowest stop");
    h.key_press(Key::ArrowUp);
    settle(&mut h);
    assert_eq!(h.state().settings().colours, 2);
    assert_eq!(h.state().palette_colours().len(), 2);
    h.key_press_modifiers(Modifiers::SHIFT, Key::ArrowUp);
    settle(&mut h);
    assert_eq!(h.state().settings().colours, 6);
    h.key_press(Key::End);
    settle(&mut h);
    assert_eq!(h.state().settings().colours, 16);
    h.key_press(Key::Home);
    settle(&mut h);
    assert_eq!(h.state().settings().colours, 0);
    assert_eq!(h.state().palette_colours().len(), 4);
}

/// Criterion 18: Output and Colours are remembered, the palette edits are not.
#[test]
fn output_and_colours_are_remembered_but_not_the_exclusions() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = multicolour(dir.path(), None);
    h.get_by_label("Colours").focus();
    h.run_steps(2);
    h.key_press_modifiers(Modifiers::SHIFT, Key::ArrowUp);
    settle(&mut h);
    let colours = h.state().palette_colours();
    ctrl_click(&mut h, colours[0], false);
    settle(&mut h);
    assert_eq!(
        h.state().picture_values().unwrap().palette.excluded.len(),
        1
    );
    let text = std::fs::read_to_string(dir.path().join("settings.toml")).unwrap();
    assert!(text.contains("multicolour = true"), "{text}");
    assert!(text.contains("colours = 5"), "{text}");
    drop(h);
    let again = opened(dir.path(), None);
    assert!(again.state().settings().multicolour);
    assert_eq!(again.state().settings().colours, 5);
    assert_eq!(
        again
            .state()
            .picture_values()
            .unwrap()
            .palette
            .excluded
            .len(),
        0
    );
}

/// Criterion 14: Ctrl+click excludes a colour (faded, crossed circle), a second one brings it
/// back; an excluded colour has no layer in the file.
#[test]
fn ctrl_click_excludes_a_colour_and_a_second_brings_it_back() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("out.svg");
    let mut h = multicolour(dir.path(), Some(out.clone()));
    let colours = h.state().palette_colours();
    ctrl_click(&mut h, colours[1], false);
    settle(&mut h);
    assert!(h.query_by_label(&swatch_label(colours[1], true)).is_some());
    assert_eq!(
        h.state().palette_colours(),
        colours,
        "an excluded colour stays in the row"
    );
    h.get_by_label("Save as\u{2026}").click();
    h.run_steps(5);
    let expected: Vec<Rgb8> = vec![colours[0], colours[2], colours[3]];
    assert_eq!(
        layer_labels(&out),
        hexes(&expected),
        "no layer for the excluded colour"
    );

    ctrl_click(&mut h, colours[1], true);
    settle(&mut h);
    assert!(h.query_by_label(&swatch_label(colours[1], false)).is_some());
    h.get_by_label("Save as\u{2026}").click();
    h.run_steps(5);
    assert_eq!(layer_labels(&out), hexes(&colours));
}

/// Criterion 20d: one layer per colour, labelled with its upper-case hex code, largest area
/// first in the file (at the bottom).
#[test]
fn the_saved_file_has_one_labelled_layer_per_colour_largest_first() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("logo.svg");
    let mut h = multicolour(dir.path(), Some(out.clone()));
    h.get_by_label("Save as\u{2026}").click();
    h.run_steps(5);
    let labels = layer_labels(&out);
    assert_eq!(labels, hexes(&h.state().palette_colours()));
    assert_eq!(labels.len(), 4);
    for label in &labels {
        assert_eq!(label.len(), 7);
        assert_eq!(*label, label.to_ascii_uppercase());
    }
}

/// Criterion 14: the x deletes a colour and the rest are matched again; with none left nothing
/// is traced and the reason is named; the palette pipette adds colours again.
#[test]
fn deleting_every_colour_traces_nothing_and_the_pipette_adds_one_again() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = multicolour(dir.path(), None);
    let colours = h.state().palette_colours();
    h.get_by_label(&swatch_label(colours[0], false)).click();
    settle(&mut h);
    let rest = h.state().palette_colours();
    assert_eq!(rest.len(), 3);
    assert!(!rest.contains(&colours[0]), "the deleted colour is gone");
    assert_eq!(h.state().nothing_to_save(), None);
    for c in &rest {
        h.get_by_label(&swatch_label(*c, false)).click();
        settle(&mut h);
    }
    assert_eq!(h.state().palette_colours().len(), 0);
    assert_eq!(
        h.state().nothing_to_save(),
        Some("0 paths: no colour in the palette.")
    );
    h.get_by_label("Pick palette colour from the image").click();
    h.run_steps(2);
    assert!(h.state().pipette_armed());
    let at = h.state().picture_point_on_screen(470, 170).unwrap();
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
    settle(&mut h);
    assert_eq!(h.state().palette_colours().len(), 1);
    assert_eq!(h.state().nothing_to_save(), None);
    assert!(h.state().pipette_armed(), "the palette pipette stays armed");
    h.key_press(Key::Escape);
    h.run_steps(2);
    assert!(!h.state().pipette_armed(), "Esc disarms it");
}

/// A pick of a colour the palette already has adds nothing.
#[test]
fn a_pick_of_a_colour_the_palette_has_adds_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = multicolour(dir.path(), None);
    h.get_by_label("Pick palette colour from the image").click();
    h.run_steps(2);
    let at = h.state().picture_point_on_screen(470, 170).unwrap();
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
    h.run_steps(5);
    let values = h.state().picture_values().unwrap();
    assert!(values.palette.fixed.is_empty(), "{:?}", values.palette);
    assert_eq!(h.state().palette_colours().len(), 4);
}

/// Criterion 14: a swatch is a keyboard stop: the arrows move, Enter excludes, Delete deletes.
#[test]
fn a_swatch_answers_the_keyboard() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = multicolour(dir.path(), None);
    let colours = h.state().palette_colours();
    h.get_by_label(&swatch_label(colours[0], false)).focus();
    h.run_steps(2);
    h.key_press(Key::ArrowRight);
    h.run_steps(3);
    h.key_press(Key::Enter);
    settle(&mut h);
    assert!(
        h.query_by_label(&swatch_label(colours[1], true)).is_some(),
        "Enter excluded the second swatch the arrow moved to"
    );
    h.key_press(Key::Enter);
    settle(&mut h);
    assert!(h.query_by_label(&swatch_label(colours[1], false)).is_some());
    h.key_press(Key::Delete);
    settle(&mut h);
    assert_eq!(h.state().palette_colours().len(), 3);
    assert!(!h.state().palette_colours().contains(&colours[1]));
    h.key_press_modifiers(Modifiers::SHIFT, Key::Enter);
    settle(&mut h);
    assert_eq!(h.state().palette_colours().len(), 2, "Shift+Enter deletes");
}

/// A new method starts from the found palette again.
#[test]
fn a_new_method_starts_from_the_found_palette() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = multicolour(dir.path(), None);
    let colours = h.state().palette_colours();
    h.get_by_label(&swatch_label(colours[0], false)).click();
    settle(&mut h);
    assert_eq!(h.state().palette_colours().len(), 3);
    h.get_by_label("Method").click();
    h.run_steps(3);
    h.get_by_label("Brightness").click();
    settle(&mut h);
    assert!(h.state().picture_values().unwrap().palette.kept.is_none());
}

/// Criterion 15: a multicolour trace that finds nothing says "No colours found.".
#[test]
fn a_multicolour_trace_without_colours_says_so() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = multicolour(dir.path(), None);
    h.get_by_label("Method").click();
    h.run_steps(3);
    h.get_by_label("Brightness").click();
    settle(&mut h);
    h.get_by_label("Threshold").focus();
    h.run_steps(2);
    h.key_press(Key::Home);
    settle(&mut h);
    assert_eq!(h.state().nothing_to_save(), Some("No colours found."));
}

/// Pick colours with Multicolour: the picked colours are the palette, so there are no swatches
/// and no palette pipette in the block.
#[test]
fn pick_colours_has_no_palette_row() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = multicolour(dir.path(), None);
    h.get_by_label("Method").click();
    h.run_steps(3);
    h.get_by_label("Pick colours").click();
    h.run_steps(5);
    assert!(
        h.query_by_label("Pick palette colour from the image")
            .is_none()
    );
    assert!(h.query_by_label("Pick colour from the image").is_some());
}
