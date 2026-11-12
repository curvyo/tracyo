//! The About panel and the first frame of a picture under a tall bar.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use eframe::egui::{Key, Role};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use tracyo_app::App;
use tracyo_app::about;
use tracyo_app::dialogs::Dialogs;

fn fixed() -> Dialogs {
    Dialogs::Fixed {
        open: None,
        save: None,
    }
}

fn opened<'a>(dir: &std::path::Path) -> Harness<'a, App> {
    let mut h = common::harness(dir, Some(common::logo()), fixed());
    common::wait_for_result(&mut h, 60.0);
    h.run_steps(3);
    h
}

fn press_about(h: &mut Harness<'_, App>) {
    h.get_by_role_and_label(Role::Button, "About Tracyo")
        .click();
    h.run_steps(3);
}

fn pixel(h: &mut Harness<'_, App>, x: u32, y: u32) -> [u8; 3] {
    let image = h.render().unwrap();
    let p = image.get_pixel(x, y).0;
    [p[0], p[1], p[2]]
}

#[test]
fn the_button_opens_and_closes_the_docked_panel() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = opened(dir.path());
    assert!(!h.state().about_open());
    let before = h.state().picture_point_on_screen(0, 0).unwrap();
    press_about(&mut h);
    assert!(h.state().about_open());
    // The panel is 280 px wide at the right edge, from the top to the status bar, with its line
    // on the canvas side; the view keeps its origin.
    assert_eq!(
        pixel(&mut h, 1000, 5),
        [0xB4, 0xB4, 0xB8],
        "the panel's edge line"
    );
    assert_eq!(
        pixel(&mut h, 1005, 700),
        [0xDC, 0xDC, 0xE0],
        "the panel's ground"
    );
    assert_eq!(
        pixel(&mut h, 999, 5),
        [0xE8, 0xE8, 0xEB],
        "the canvas ends at the line"
    );
    assert_eq!(h.state().picture_point_on_screen(0, 0).unwrap(), before);
    h.get_by_role(Role::Region);
    press_about(&mut h);
    assert!(!h.state().about_open());
    assert_eq!(
        pixel(&mut h, 1005, 700),
        [0xE8, 0xE8, 0xEB],
        "the canvas is whole again"
    );
}

#[test]
fn escape_leaves_the_panel_open_and_returns_the_keyboard_to_the_canvas() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = opened(dir.path());
    h.get_by_role_and_label(Role::Button, "About Tracyo")
        .focus();
    h.run_steps(2);
    h.key_press(Key::Enter);
    h.run_steps(3);
    assert!(
        h.state().about_open(),
        "Enter on the focused button opens it"
    );
    h.key_press(Key::Escape);
    h.run_steps(3);
    assert!(h.state().about_open(), "Esc never closes the panel");
    assert!(
        h.get_all_by_role(Role::Button)
            .all(|b| !b.accesskit_node().is_focused()),
        "the keyboard is back on the canvas"
    );
}

#[test]
fn the_long_text_is_a_tab_stop_that_the_arrow_keys_scroll() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = opened(dir.path());
    press_about(&mut h);
    h.run_steps(3);
    // The last Tab stop of the window, after the bar.
    h.get_by_role_and_label(Role::ScrollView, "About Tracyo text")
        .focus();
    h.run_steps(2);
    assert_eq!(h.state().about_scroll(), 0.0);
    for _ in 0..3 {
        h.key_press(Key::ArrowDown);
        h.run_steps(2);
    }
    assert!(
        (h.state().about_scroll() - 120.0).abs() < 1.0,
        "{}",
        h.state().about_scroll()
    );
    h.key_press(Key::End);
    h.run_steps(3);
    let end = h.state().about_scroll();
    assert!(end > 500.0, "End scrolls to the bottom ({end})");
    h.key_press(Key::Home);
    h.run_steps(3);
    assert_eq!(h.state().about_scroll(), 0.0);
    h.key_press(Key::Escape);
    h.run_steps(2);
    assert!(h.state().about_open());
}

#[test]
fn the_panel_says_what_the_spec_asks_for() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = opened(dir.path());
    press_about(&mut h);
    // Words a screen reader can reach (the panel scrolls, so only the first entries are laid out
    // on a window of this height, but the heading, the intro and the licence are on top).
    h.get_by_label_contains("Tracyo turns a picture into paths");
    h.get_by_label_contains("AGPL-3.0-or-later");
    let text = about::text();
    assert!(text.starts_with("About Tracyo\n"));
    for needed in [
        env!("CARGO_PKG_VERSION"),
        "AGPL-3.0-or-later",
        about::SOURCE,
        "vectrace, MIT OR Apache-2.0",
        "PaddleOCR Latin, Apache-2.0",
        "THIRD-PARTY.md",
        &vectrace::ocr::DETECTOR_SHA256[..12],
        &vectrace::ocr::RECOGNISER_SHA256[..12],
        &vectrace::ocr::ALPHABET_SHA256[..12],
    ] {
        assert!(text.contains(needed), "the panel lacks {needed:?}");
    }
    for absent in ["Mask", "Parts", "Add part"] {
        assert!(!text.contains(absent), "Tracyo has no {absent}");
    }
}

#[test]
fn every_control_of_the_bar_has_an_entry() {
    let terms: Vec<String> = about::entries().into_iter().map(|(t, _)| t).collect();
    for control in [
        "Preset",
        "Method",
        "Threshold",
        "Colour tolerance",
        "Smooth",
        "Tolerance",
        "Ignore below",
        "Output",
        "Colours",
        "Palette",
        "Invert",
        "Drop border shapes",
        "Recognise text",
        "Recognise shapes",
    ] {
        assert!(terms.iter().any(|t| t == control), "no entry for {control}");
    }
}

/// A 400 x 1600 picture is fitted to the height; its top edge must sit 12 px below the bar of the
/// same frame even when that bar has three rows (technical debt 11).
#[test]
fn the_first_frame_fits_below_a_bar_of_three_rows() {
    let dir = tempfile::tempdir().unwrap();
    let tall = dir.path().join("tall.png");
    image::RgbaImage::from_fn(400, 1600, |x, y| {
        if (x / 50 + y / 50) % 2 == 0 {
            image::Rgba([20, 20, 20, 255])
        } else {
            image::Rgba([240, 240, 240, 255])
        }
    })
    .save(&tall)
    .unwrap();
    std::fs::write(
        dir.path().join("settings.toml"),
        "advanced_open = true\nmethod = 1\n",
    )
    .unwrap();
    let mut h = common::harness_sized(dir.path(), Some(tall), fixed(), 800.0);
    common::wait_for_result(&mut h, 60.0);
    h.run_steps(3);
    let app = h.state();
    let zoom = app.zoom().unwrap();
    let top = app.picture_point_on_screen(0, 0).unwrap().y - 0.5 * zoom;
    assert!(
        app.bar_bottom() > 12.0 + 72.0 + 40.0,
        "the bar has at least three rows, it ends at {}",
        app.bar_bottom()
    );
    assert!(
        (top - (app.bar_bottom() + 12.0)).abs() < 1.0,
        "picture top {top}, bar bottom {}",
        app.bar_bottom()
    );
}

/// Eight bands of the 1600 px picture, in eight colours that stay eight in the palette.
const BANDS: [[u8; 3]; 8] = [
    [200, 30, 40],
    [30, 120, 60],
    [30, 60, 200],
    [240, 200, 40],
    [120, 40, 160],
    [240, 120, 30],
    [30, 180, 190],
    [20, 20, 20],
];

/// The palette of a Multicolour trace arrives after the first frame and wraps the bar onto
/// another line; the picture, still where the fit put it, is fitted again below the bar. A
/// picture the user has panned stays where it is.
#[test]
fn a_bar_that_grows_after_the_fit_does_not_cover_the_picture() {
    let dir = tempfile::tempdir().unwrap();
    let tall = dir.path().join("tall.png");
    image::RgbaImage::from_fn(400, 1600, |_, y| {
        let [r, g, b] = BANDS[(y / 200) as usize];
        image::Rgba([r, g, b, 255])
    })
    .save(&tall)
    .unwrap();
    std::fs::write(
        dir.path().join("settings.toml"),
        "advanced_open = true\nmethod = 1\nmulticolour = true\n",
    )
    .unwrap();
    let mut h = common::harness(dir.path(), Some(tall), fixed());
    common::wait_for_result_where(&mut h, 20.0, |a| !a.palette_colours().is_empty());
    h.run_steps(5);
    let app = h.state();
    let zoom = app.zoom().unwrap();
    let top = app.picture_point_on_screen(0, 0).unwrap().y - 0.5 * zoom;
    assert!(
        top >= app.bar_bottom() + 12.0 - 1.0,
        "picture top {top}, bar bottom {}",
        app.bar_bottom()
    );
}
