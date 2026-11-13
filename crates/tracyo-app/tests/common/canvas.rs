//! Pointer and key helpers of the tests that work on the canvas: clicks, drags, the settle after a
//! change of the mask and the paths of a saved SVG.

#![allow(
    dead_code,
    clippy::unwrap_used,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]

use std::path::Path;

use eframe::egui::{Event, Key, Modifiers, PointerButton, Pos2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use tracyo_app::App;

use crate::common::{wait_for_result, wait_until};

/// The window position of the corner of picture pixel (`x`, `y`).
pub fn corner(h: &Harness<'_, App>, x: f32, y: f32) -> Pos2 {
    h.state().picture_corner_on_screen(x, y).unwrap()
}

/// A click on the control named `label`.
pub fn button(h: &mut Harness<'_, App>, label: &str) {
    h.get_by_label(label).click();
    h.run_steps(3);
    h.remove_cursor();
    h.run_steps(2);
}

/// Opens the Advanced block unless a start with the remembered state has it open already.
pub fn open_advanced(h: &mut Harness<'_, App>) {
    if h.query_by_label("More trace options").is_some() {
        button(h, "More trace options");
    }
}

/// Waits until the trace, and the analysis of the kept pixels with the trace it may start, are done.
pub fn settle(h: &mut Harness<'_, App>) {
    wait_until(h, 30.0, |a| !a.is_analysing());
    wait_for_result(h, 60.0);
    h.run_steps(2);
    wait_until(h, 30.0, |a| !a.is_analysing());
}

/// A press and release of the primary button at `at`, each in frames of their own, `modifiers`
/// held.
pub fn click_with(h: &mut Harness<'_, App>, at: Pos2, modifiers: Modifiers) {
    h.event(Event::PointerMoved(at));
    h.run_steps(2);
    if modifiers != Modifiers::NONE {
        h.event(Event::ModifiersChanged(modifiers));
        h.run_steps(1);
    }
    for pressed in [true, false] {
        h.event(Event::PointerButton {
            pos: at,
            button: PointerButton::Primary,
            pressed,
            modifiers,
        });
        h.run_steps(2);
    }
    if modifiers != Modifiers::NONE {
        h.event(Event::ModifiersChanged(Modifiers::NONE));
        h.run_steps(1);
    }
}

/// A click at `at`.
pub fn click(h: &mut Harness<'_, App>, at: Pos2) {
    click_with(h, at, Modifiers::NONE);
}

/// A press and release of the secondary button at `at`.
pub fn right_click(h: &mut Harness<'_, App>, at: Pos2) {
    h.event(Event::PointerMoved(at));
    h.run_steps(2);
    for pressed in [true, false] {
        h.event(Event::PointerButton {
            pos: at,
            button: PointerButton::Secondary,
            pressed,
            modifiers: Modifiers::NONE,
        });
        h.run_steps(2);
    }
}

/// The press of a drag from `from` to `to` with its first moves, the button still down.
pub fn drag_begin(h: &mut Harness<'_, App>, from: Pos2, to: Pos2) {
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
}

/// The release of the button at `at`.
pub fn release(h: &mut Harness<'_, App>, at: Pos2) {
    h.event(Event::PointerButton {
        pos: at,
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::NONE,
    });
    h.run_steps(3);
}

/// A drag of the primary button from `from` to `to`, in frames of their own.
pub fn drag(h: &mut Harness<'_, App>, from: Pos2, to: Pos2) {
    drag_begin(h, from, to);
    release(h, to);
}

/// A key with `modifiers`, then a few frames.
pub fn press(h: &mut Harness<'_, App>, key: Key, modifiers: Modifiers) {
    h.key_press_modifiers(modifiers, key);
    h.run_steps(3);
}

/// The closed outlines of the SVG written to `file`, one path per `<path>`.
pub fn saved_paths(file: &Path) -> Vec<kurbo::BezPath> {
    let text = std::fs::read_to_string(file).unwrap();
    let doc = roxmltree::Document::parse(&text).unwrap();
    doc.descendants()
        .filter(|n| n.has_tag_name("path"))
        .map(|n| kurbo::BezPath::from_svg(n.attribute("d").unwrap()).unwrap())
        .collect()
}

/// A click on the corner of picture pixel (`x`, `y`).
pub fn click_px(h: &mut Harness<'_, App>, x: f32, y: f32) {
    let at = corner(h, x, y);
    click(h, at);
}

/// A drag from the corner of picture pixel `from` to that of `to`.
pub fn drag_px(h: &mut Harness<'_, App>, from: (f32, f32), to: (f32, f32)) {
    let (a, b) = (corner(h, from.0, from.1), corner(h, to.0, to.1));
    drag(h, a, b);
}
