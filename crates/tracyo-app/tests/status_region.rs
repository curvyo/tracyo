//! The hidden status region (AccessKit live region, ux-notes of 0002 section 4): the texts the
//! polite region says when a region tool, Subtract, the parts or the mask change.

#![allow(clippy::unwrap_used, clippy::expect_used)]

#[path = "common/canvas.rs"]
mod canvas;
mod common;

use eframe::egui::accesskit::{Live, Role};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use tracyo_app::App;
use tracyo_app::dialogs::Dialogs;

fn opened<'a>(dir: &std::path::Path) -> Harness<'a, App> {
    let mut h = common::harness_sized(
        dir,
        Some(common::logo()),
        Dialogs::Fixed {
            open: None,
            save: None,
        },
        1280.0,
    );
    common::wait_for_result(&mut h, 60.0);
    h.run_steps(3);
    canvas::open_advanced(&mut h);
    h
}

/// What the live region says now.
fn said(h: &Harness<'_, App>) -> String {
    let region = h.get_by_role(Role::Status);
    assert_eq!(region.accesskit_node().live(), Live::Polite);
    region.accesskit_node().label().unwrap_or_default()
}

#[test]
fn the_region_says_the_state_of_the_tools_and_of_the_parts() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = opened(dir.path());
    canvas::button(&mut h, "Rectangle");
    assert_eq!(said(&h), "Rectangle on");
    canvas::button(&mut h, "Lasso");
    assert_eq!(
        said(&h),
        "Lasso on",
        "pressing one region tool says only the new one"
    );
    canvas::button(&mut h, "Lasso");
    assert_eq!(said(&h), "Lasso off");
    canvas::button(&mut h, "Subtract");
    assert_eq!(said(&h), "Subtract on");
    canvas::button(&mut h, "Subtract");
    assert_eq!(said(&h), "Subtract off");
    canvas::button(&mut h, "Add part");
    canvas::button(&mut h, "Add part");
    canvas::settle(&mut h);
    assert_eq!(
        said(&h),
        format!("3 parts. {}. Part 3 has no paths", counts_of(&h)),
        "the counts come after the parts and before the part without paths"
    );
    canvas::button(&mut h, "Part 1 of 3, whole image");
    assert_eq!(said(&h), "3 parts");
}

#[test]
fn the_region_says_the_mask_after_a_rectangle_is_drawn_and_after_it_is_cleared() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = opened(dir.path());
    canvas::button(&mut h, "Rectangle");
    canvas::drag_px(&mut h, (100.0, 100.0), (300.0, 260.0));
    canvas::settle(&mut h);
    assert!(said(&h).starts_with("Inside 1 path"), "{}", said(&h));
    canvas::button(&mut h, "Clear mask");
    assert_eq!(said(&h), "", "nothing is said when no mask is left");
}

#[test]
fn the_save_tooltip_counts_the_parts_joined() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = opened(dir.path());
    assert_eq!(h.state().save_tooltip(), "Save as\u{2026} (Ctrl+Shift+S)");
    canvas::button(&mut h, "Add part");
    assert_eq!(
        h.state().save_tooltip(),
        "Save as\u{2026} (Ctrl+Shift+S), 2 parts joined"
    );
    canvas::button(&mut h, "Add part");
    assert_eq!(
        h.state().save_tooltip(),
        "Save as\u{2026} (Ctrl+Shift+S), 3 parts joined"
    );
}

#[test]
fn the_save_tooltip_without_a_picture_is_the_plain_text() {
    let dir = tempfile::tempdir().unwrap();
    let h = common::harness_sized(
        dir.path(),
        None,
        Dialogs::Fixed {
            open: None,
            save: None,
        },
        1280.0,
    );
    assert_eq!(h.state().save_tooltip(), "Save as\u{2026} (Ctrl+Shift+S)");
}

#[test]
fn the_region_says_the_trash_arming_text_and_nothing_after_the_removal() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = opened(dir.path());
    canvas::button(&mut h, "Add part");
    canvas::button(&mut h, "Remove part 2");
    assert!(h.state().trash_armed());
    assert_eq!(said(&h), "Press again to remove part 2");
    std::thread::sleep(std::time::Duration::from_millis(450));
    h.get_by_role_and_label(Role::Button, "Press again to remove part 2")
        .click();
    h.run_steps(3);
    canvas::settle(&mut h);
    assert_eq!(h.state().part_count(), 1);
    assert_eq!(
        said(&h),
        counts_of(&h),
        "nothing about the removal is said, only the counts of the new result"
    );
}

#[test]
fn the_region_says_the_reason_of_a_dimmed_save_as() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = opened(dir.path());
    canvas::button(&mut h, "Rectangle");
    canvas::drag_px(&mut h, (100.0, 450.0), (300.0, 520.0));
    canvas::settle(&mut h);
    let reason = h.state().nothing_to_save().expect("a dimmed Save as");
    assert_eq!(said(&h), reason);
}

/// The counts the settled result has, worked out from the result itself.
fn counts_of(h: &Harness<'_, App>) -> String {
    h.state().result_counts().expect("a settled result")
}

#[test]
fn the_region_says_the_counts_of_a_settled_single_part() {
    let dir = tempfile::tempdir().unwrap();
    let h = opened(dir.path());
    let counts = counts_of(&h);
    assert!(
        counts.contains(" path") && counts.contains(", ") && counts.contains(" node"),
        "{counts}"
    );
    assert_eq!(said(&h), counts, "one part, no mask: only the counts");
}

#[test]
fn the_region_says_the_counts_of_the_joined_parts_after_the_parts() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = opened(dir.path());
    let single = counts_of(&h);
    canvas::button(&mut h, "Add part");
    canvas::button(&mut h, "Rectangle");
    canvas::drag_px(&mut h, (100.0, 100.0), (300.0, 260.0));
    canvas::settle(&mut h);
    let counts = counts_of(&h);
    assert_ne!(counts, single, "the joined result has other counts");
    assert_eq!(said(&h), format!("2 parts. {counts}"));
}

#[test]
fn the_counts_are_said_once_per_settled_result() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = opened(dir.path());
    let counts = counts_of(&h);
    canvas::button(&mut h, "Subtract");
    assert_eq!(said(&h), "Subtract on", "a toggle says only itself");
    h.run_steps(5);
    assert_eq!(
        said(&h),
        "Subtract on",
        "unrelated redraws do not repeat the counts"
    );
    canvas::button(&mut h, "Subtract");
    h.run_steps(5);
    assert_eq!(said(&h), "Subtract off");
    // A new trace says its counts again.
    canvas::button(&mut h, "Rectangle");
    canvas::drag_px(&mut h, (100.0, 100.0), (300.0, 260.0));
    canvas::settle(&mut h);
    let masked = counts_of(&h);
    assert_eq!(said(&h), format!("Inside 1 path. {masked}"));
    assert_ne!(counts, "", "the first result had counts");
}

#[test]
fn the_counts_use_singular_forms() {
    use tracyo_core::parts::Parts;
    assert_eq!(Parts::counts_text(1, 4), "1 path, 4 nodes");
    assert_eq!(Parts::counts_text(7, 412), "7 paths, 412 nodes");
    assert_eq!(Parts::counts_text(0, 1), "0 paths, 1 node");
}
