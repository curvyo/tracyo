//! The methods, the pipettes, the Advanced block and the Analyse button with the real worker.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use std::path::Path;

use eframe::egui::{Event, Key, Modifiers, PointerButton, Pos2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use tracyo_app::App;
use tracyo_app::dialogs::Dialogs;
use tracyo_core::method::hex;
use vectrace::Rgb8;

fn fixed() -> Dialogs {
    Dialogs::Fixed {
        open: Some(common::logo()),
        save: None,
    }
}

fn opened<'a>(dir: &Path) -> Harness<'a, App> {
    let mut h = common::harness(dir, Some(common::logo()), fixed());
    common::wait_until(&mut h, 60.0, App::has_result);
    h.run_steps(3);
    h
}

fn press(h: &mut Harness<'_, App>, key: Key, modifiers: Modifiers) {
    h.key_press_modifiers(modifiers, key);
    h.run_steps(2);
}

/// Chooses a method in the menu and waits for its preview.
fn choose(h: &mut Harness<'_, App>, method: &str) {
    h.get_by_label("Method").click();
    h.run_steps(3);
    h.get_by_label(method).click();
    h.run_steps(3);
    common::wait_until(h, 60.0, App::has_result);
    h.run_steps(2);
}

fn click_picture(h: &mut Harness<'_, App>, x: u32, y: u32, modifiers: Modifiers) {
    let at: Pos2 = h.state().picture_point_on_screen(x, y).unwrap();
    h.event(Event::PointerMoved(at));
    h.run_steps(2);
    h.event(Event::ModifiersChanged(modifiers));
    for pressed in [true, false] {
        h.event(Event::PointerButton {
            pos: at,
            button: PointerButton::Primary,
            pressed,
            modifiers,
        });
        h.run_steps(2);
    }
    h.event(Event::ModifiersChanged(Modifiers::NONE));
    h.run_steps(1);
}

fn settle(h: &mut Harness<'_, App>) {
    common::wait_until(h, 60.0, App::has_result);
    h.run_steps(2);
}

const RED: Rgb8 = Rgb8::new(0xC8, 0x10, 0x2E);

/// Criterion 14: the analysis sets the method of a picture when it opens (Remove background for
/// the logo, whose yellow Brightness would miss).
#[test]
fn the_analysis_chooses_remove_background_for_the_logo() {
    let dir = tempfile::tempdir().unwrap();
    let h = opened(dir.path());
    assert_eq!(h.state().settings().method, 1);
    let values = h.state().picture_values().unwrap();
    assert!(values.background_auto());
    assert_eq!(values.swatches.len(), 0);
    assert_eq!(h.state().nothing_to_save(), None);
}

/// Criterion 12: Pick colours arms the pipette, waits for a colour, a click replaces and Shift
/// adds, an x takes one out, and Esc disarms.
#[test]
fn pick_colours_arms_the_pipette_and_collects_colours() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = opened(dir.path());
    choose(&mut h, "Pick colours");
    assert!(h.state().pipette_armed());
    assert_eq!(
        h.state().nothing_to_save(),
        Some("0 paths: no colour picked.")
    );

    click_picture(&mut h, 470, 170, Modifiers::NONE);
    settle(&mut h);
    let picked = h.state().picture_values().unwrap().swatches.clone();
    assert_eq!(picked.len(), 1);
    assert!(
        (i32::from(picked[0].r) - i32::from(RED.r)).abs() < 3,
        "{picked:?}"
    );
    assert_eq!(h.state().nothing_to_save(), None);
    assert!(h.state().pipette_armed(), "the pipette stays armed");

    click_picture(&mut h, 100, 100, Modifiers::SHIFT);
    settle(&mut h);
    assert_eq!(h.state().picture_values().unwrap().swatches.len(), 2);
    click_picture(&mut h, 100, 100, Modifiers::SHIFT);
    settle(&mut h);
    assert_eq!(
        h.state().picture_values().unwrap().swatches.len(),
        2,
        "a colour already picked is not added twice"
    );

    click_picture(&mut h, 480, 360, Modifiers::NONE);
    settle(&mut h);
    let only = h.state().picture_values().unwrap().swatches.clone();
    assert_eq!(only.len(), 1, "a plain click replaces the picked colours");

    h.get_by_label(&format!("Remove {}", hex(only[0]))).click();
    settle(&mut h);
    assert_eq!(h.state().picture_values().unwrap().swatches.len(), 0);
    assert_eq!(
        h.state().nothing_to_save(),
        Some("0 paths: no colour picked.")
    );

    press(&mut h, Key::Escape, Modifiers::NONE);
    assert!(!h.state().pipette_armed(), "Esc disarms the pipette");
}

/// Criterion 12: the background pipette takes the colour and releases Auto, Auto takes it back.
#[test]
fn the_background_pipette_releases_auto() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = opened(dir.path());
    h.get_by_label("Pick background colour from the image")
        .click();
    h.run_steps(2);
    assert!(h.state().pipette_armed());
    click_picture(&mut h, 480, 360, Modifiers::NONE);
    settle(&mut h);
    let values = h.state().picture_values().unwrap();
    assert!(!values.background_auto());
    assert!(values.manual_background.is_some());
    assert!(!h.state().pipette_armed(), "the background pipette disarms");
    h.get_by_label("Auto").click();
    settle(&mut h);
    assert!(h.state().picture_values().unwrap().background_auto());
}

/// Criterion 12: the Colour tolerance is remembered per method.
#[test]
fn the_colour_tolerance_is_remembered_per_method() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = opened(dir.path());
    let before = h.state().settings().remove_tolerance;
    h.get_by_label("Colour tolerance").focus();
    h.run_steps(2);
    press(&mut h, Key::ArrowLeft, Modifiers::SHIFT);
    settle(&mut h);
    let after = h.state().settings().remove_tolerance;
    assert_eq!(after, before.saturating_sub(10));
    choose(&mut h, "Pick colours");
    assert_eq!(h.state().settings().pick_tolerance, 19);
    assert_eq!(h.state().settings().remove_tolerance, after);
    let text = std::fs::read_to_string(dir.path().join("settings.toml")).unwrap();
    assert!(text.contains("pick_tolerance = 19"), "{text}");
    assert!(
        text.contains(&format!("remove_tolerance = {after}")),
        "{text}"
    );
    assert!(text.contains("method = 2"), "{text}");
}

/// Criterion 14: the expander opens the block, open or closed is remembered, and the block's
/// values change the trace and make the preset read Custom.
#[test]
fn the_advanced_block_works_and_is_remembered() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = opened(dir.path());
    h.get_by_label("More trace options").click();
    h.run_steps(3);
    assert!(h.state().settings().advanced_open);
    assert!(h.query_by_label("Fewer trace options").is_some());

    let jobs = h.state().trace_jobs();
    h.get_by_label("Invert").click();
    settle(&mut h);
    assert!(h.state().picture_values().unwrap().invert);
    assert!(h.state().trace_jobs() > jobs);
    h.get_by_label("Drop border shapes").click();
    settle(&mut h);
    assert!(h.state().picture_values().unwrap().drop_border);

    assert_eq!(h.state().settings().preset_label(), "Logo / Line art");
    h.get_by_label("Denoise: Low").click();
    settle(&mut h);
    assert_eq!(h.state().settings().denoise, 1);
    assert_eq!(h.state().settings().preset_label(), "Custom");
    h.get_by_label("Upscale: Auto").click();
    settle(&mut h);
    assert!(h.state().settings().upscale_auto);
    h.get_by_label("Corners").focus();
    h.run_steps(2);
    press(&mut h, Key::ArrowRight, Modifiers::NONE);
    settle(&mut h);
    assert_eq!(h.state().settings().corners, 71);

    drop(h);
    let mut again = common::harness(dir.path(), Some(common::logo()), fixed());
    common::wait_until(&mut again, 60.0, App::has_result);
    again.run_steps(3);
    assert!(again.state().settings().advanced_open);
    assert!(again.query_by_label("Fewer trace options").is_some());
    assert_eq!(again.state().settings().corners, 71);
    assert!(
        !again.state().picture_values().unwrap().invert,
        "Invert starts off with every start"
    );
}

/// Criterion 11: picking a preset restores its six values after Custom.
#[test]
fn a_preset_restores_the_six_values_after_custom() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = opened(dir.path());
    h.get_by_label("More trace options").click();
    h.run_steps(3);
    h.get_by_label("Corners").focus();
    h.run_steps(2);
    press(&mut h, Key::ArrowLeft, Modifiers::SHIFT);
    settle(&mut h);
    assert_eq!(h.state().settings().preset_label(), "Custom");
    h.get_by_label("Preset").click();
    h.run_steps(3);
    h.get_by_label("Logo / Line art").click();
    settle(&mut h);
    assert_eq!(h.state().settings().preset_label(), "Logo / Line art");
    assert_eq!(h.state().settings().corners, 70);
}

/// Criterion 14: Analyse sets the analysed values back.
#[test]
fn analyse_sets_the_analysed_values_back() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = opened(dir.path());
    choose(&mut h, "Brightness");
    h.get_by_label("Threshold").focus();
    h.run_steps(2);
    press(&mut h, Key::ArrowLeft, Modifiers::SHIFT);
    settle(&mut h);
    let moved = h.state().picture_values().unwrap().threshold;
    assert_eq!(h.state().settings().method, 0);
    h.get_by_label("More trace options").click();
    h.run_steps(3);
    h.get_by_label("Analyse").click();
    common::wait_until(&mut h, 60.0, |app| {
        app.settings().method == 1 && app.has_result()
    });
    assert_ne!(h.state().picture_values().unwrap().threshold, moved);
}

/// Criterion 1: a second picture resets the picked colours, Invert and Drop border shapes, and
/// keeps the settings.
#[test]
fn a_new_picture_resets_the_picture_values_and_keeps_the_settings() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = opened(dir.path());
    h.get_by_label("More trace options").click();
    h.run_steps(3);
    h.get_by_label("Invert").click();
    settle(&mut h);
    h.get_by_label("Corners").focus();
    h.run_steps(2);
    press(&mut h, Key::ArrowRight, Modifiers::NONE);
    settle(&mut h);
    press(&mut h, Key::O, Modifiers::COMMAND);
    common::wait_until(&mut h, 60.0, |app| {
        app.has_result() && app.picture_values().is_some_and(|v| !v.invert)
    });
    assert_eq!(h.state().settings().corners, 71);
}
