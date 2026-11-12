//! Recognise text and Recognise shapes: the toggles, the read before the trace, its progress and
//! stop, the lines kept per picture, the notice of a failed reader and the saved file, with the
//! real worker (and a staged one where timing matters).

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use std::path::{Path, PathBuf};

use eframe::egui::Key;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use tracyo_app::App;
use tracyo_app::dialogs::Dialogs;
use tracyo_core::progress::Button;

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../tracyo-core/tests/fixtures/pictures")
        .join(name)
}

fn dialogs(save: Option<PathBuf>) -> Dialogs {
    Dialogs::Fixed { open: None, save }
}

/// The app with `picture` open and settled, the Advanced block open.
fn opened<'a>(dir: &Path, picture: PathBuf, save: Option<PathBuf>) -> Harness<'a, App> {
    let mut h = common::harness(dir, Some(picture), dialogs(save));
    common::wait_for_result(&mut h, 120.0);
    h.run_steps(3);
    if h.query_by_label("More trace options").is_some() {
        h.get_by_label("More trace options").click();
        h.run_steps(3);
    }
    h
}

fn settle(h: &mut Harness<'_, App>) {
    common::wait_for_result(h, 120.0);
    h.run_steps(3);
}

/// A worker that sends `frames` (printf escapes) and then waits.
fn staged(frames: &str) -> Vec<String> {
    vec![
        "/bin/sh".into(),
        "-c".into(),
        format!("printf '{frames}'; exec sleep 600"),
    ]
}

/// Percent frame: tag 1, length 1, the percent (30).
const PERCENT_30: &str = "\\001\\001\\000\\000\\000\\036";
/// The text reader's failure: tag 5, length 0.
const TEXT_FAILED: &str = "\\005\\000\\000\\000\\000";

fn svg_names(path: &Path) -> Vec<String> {
    let text = std::fs::read_to_string(path).unwrap();
    let doc = roxmltree::Document::parse(&text).unwrap();
    doc.descendants()
        .filter(roxmltree::Node::is_element)
        .map(|n| n.tag_name().name().to_owned())
        .collect()
}

/// The toggles are in the Output row, named for what they do, off at the first start, and the two
/// are remembered between starts (criterion 18).
#[test]
fn the_two_toggles_are_remembered_between_starts() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = opened(dir.path(), common::logo(), None);
    assert!(!h.state().settings().recognise_text && !h.state().settings().recognise_shapes);
    let before = h.state().trace_jobs();
    h.get_by_label("Recognise shapes").click();
    settle(&mut h);
    assert!(h.state().settings().recognise_shapes);
    assert!(h.state().trace_jobs() > before, "a toggle traces again");
    let text = std::fs::read_to_string(dir.path().join("settings.toml")).unwrap();
    assert!(text.contains("recognise_shapes = true"), "{text}");
    drop(h);
    let again = opened(dir.path(), common::logo(), None);
    assert!(again.state().settings().recognise_shapes);
    assert!(!again.state().settings().recognise_text);
}

/// Criterion 17 and 20f: with Recognise shapes the saved file has primitives, without it one path.
#[test]
fn recognise_shapes_saves_primitives_and_off_saves_one_path() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("shapes.svg");
    let mut h = opened(dir.path(), fixture("shapes.png"), Some(out.clone()));
    assert_eq!(h.state().recognised(), (0, 0));
    h.get_by_label("Save as\u{2026}").click();
    h.run_steps(3);
    let plain = svg_names(&out);
    assert!(
        !plain
            .iter()
            .any(|n| n == "rect" || n == "circle" || n == "ellipse")
    );

    h.get_by_label("Recognise shapes").click();
    settle(&mut h);
    let (_, shapes) = h.state().recognised();
    assert!(shapes > 0, "the fixture has a square and a dot");
    h.get_by_label("Save as\u{2026}").click();
    h.run_steps(3);
    let names = svg_names(&out);
    assert!(names.iter().any(|n| n == "rect"), "{names:?}");
    assert!(
        names.iter().any(|n| n == "circle" || n == "ellipse"),
        "{names:?}"
    );

    h.get_by_label("Recognise shapes").click();
    settle(&mut h);
    assert_eq!(h.state().recognised().1, 0);
}

/// Criterion 16: the text is read first ("Reading text" in the button), the lines are kept for
/// the picture, the file has `<text>`.
#[test]
fn recognise_text_reads_once_and_the_file_has_text_elements() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("precision.svg");
    let mut h = opened(dir.path(), fixture("precision.png"), Some(out.clone()));
    assert_eq!(h.state().recognised().0, 0);
    h.get_by_label("Recognise text").click();
    h.step();
    let start = std::time::Instant::now();
    let mut saw_reading = h.state().is_reading();
    while !h.state().has_result() || h.state().is_reading() {
        h.step();
        saw_reading |= h.state().is_reading();
        std::thread::sleep(std::time::Duration::from_millis(10));
        assert!(start.elapsed().as_secs() < 120, "never settled");
    }
    h.run_steps(3);
    assert!(saw_reading, "the read ran before the trace");
    assert!(h.state().recognised().0 >= 3);

    // A later preview of the same picture reuses the lines: it never reads again.
    h.get_by_label("Recognise shapes").click();
    h.step();
    let mut read_again = h.state().is_reading();
    while !h.state().has_result() {
        h.step();
        read_again |= h.state().is_reading();
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(!read_again);
    assert!(h.state().recognised().0 >= 3, "the lines are still set");

    h.get_by_label("Save as\u{2026}").click();
    h.run_steps(3);
    let names = svg_names(&out);
    assert!(names.iter().filter(|n| n.as_str() == "text").count() >= 3);
    assert!(std::fs::read_to_string(&out).unwrap().contains("BERATUNG"));
}

/// With Recognise text off the picture is traced exactly as before: no `<text>`, no read.
#[test]
fn with_recognise_text_off_nothing_is_read() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = opened(dir.path(), fixture("precision.png"), None);
    let mut read = false;
    for _ in 0..20 {
        h.step();
        read |= h.state().is_reading();
    }
    h.get_by_label("Recognise shapes").click();
    settle(&mut h);
    assert!(!read && !h.state().is_reading());
    assert_eq!(h.state().recognised().0, 0);
}

/// Criterion 16: the read shows as "Reading text" with the job's percent, and Esc stops it; Retry
/// reads again.
#[test]
fn the_read_is_named_in_the_button_and_esc_stops_it() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = opened(dir.path(), common::logo(), None);
    h.state_mut().wrap_worker(staged(PERCENT_30));
    h.get_by_label("Recognise text").click();
    let start = std::time::Instant::now();
    // 30 % of the read is 21 % of the job.
    while !matches!(h.state_mut().button(), Button::Running(Some(21))) {
        h.step();
        std::thread::sleep(std::time::Duration::from_millis(20));
        assert!(start.elapsed().as_secs() < 20, "never showed 21 %");
    }
    h.run_steps(2);
    assert!(h.state().is_reading());
    assert!(h.query_by_label("Reading text").is_some());
    assert!(h.query_by_label("Stop (Esc)").is_some());
    let jobs = h.state().trace_jobs();

    // A value changed while the text is read does not start the read again.
    h.get_by_label("Corners").focus();
    h.run_steps(2);
    h.key_press(Key::ArrowUp);
    h.run_steps(3);
    assert_eq!(h.state().trace_jobs(), jobs, "the read goes on");

    h.key_press(Key::Escape);
    h.run_steps(3);
    assert!(!h.state().is_reading());
    assert!(matches!(h.state_mut().button(), Button::Stopped));
    h.get_by_label("Retry").click();
    h.run_steps(3);
    assert!(h.state().is_reading(), "Retry reads the text again");
    assert_eq!(h.state().trace_jobs(), jobs + 1);
}

/// Switching Recognise text off while the text is read ends the read and traces without text.
#[test]
fn switching_the_text_off_during_the_read_traces_without_it() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = opened(dir.path(), common::logo(), None);
    h.state_mut().wrap_worker(staged(PERCENT_30));
    h.get_by_label("Recognise text").click();
    h.run_steps(5);
    assert!(h.state().is_reading());
    let jobs = h.state().trace_jobs();
    h.get_by_label("Recognise text").click();
    h.run_steps(3);
    assert!(!h.state().is_reading());
    assert_eq!(h.state().trace_jobs(), jobs + 1);
}

/// A reader that cannot be used: Curvyo's notice, and the trace runs without text.
#[test]
fn a_failed_reader_says_so_and_traces_without_text() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = opened(dir.path(), common::logo(), None);
    h.state_mut().wrap_worker(staged(TEXT_FAILED));
    let jobs = h.state().trace_jobs();
    h.get_by_label("Recognise text").click();
    let start = std::time::Instant::now();
    while h.state().notice_text().is_none() {
        h.step();
        std::thread::sleep(std::time::Duration::from_millis(20));
        assert!(start.elapsed().as_secs() < 20, "no notice");
    }
    assert_eq!(
        h.state().notice_text().as_deref(),
        Some("Text recognition failed. Traced without text.")
    );
    assert!(!h.state().is_reading());
    assert_eq!(h.state().trace_jobs(), jobs + 2, "the read, then the trace");
}
