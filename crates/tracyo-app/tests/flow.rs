//! End-to-end flows of the app with the real worker process: open, trace, stop, retry, save.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use std::path::{Path, PathBuf};

use eframe::egui::{self, Event, Key, Modifiers, pos2, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use tracyo_app::App;
use tracyo_app::dialogs::Dialogs;
use tracyo_core::progress::Button;

fn fixed(open: Option<PathBuf>, save: Option<PathBuf>) -> Dialogs {
    Dialogs::Fixed { open, save }
}

fn opened<'a>(dir: &Path, save: Option<PathBuf>) -> Harness<'a, App> {
    let mut h = common::harness(dir, Some(common::logo()), fixed(None, save));
    common::wait_for_result(&mut h, 60.0);
    h.run_steps(2);
    h
}

fn wait_for_button(h: &mut Harness<'_, App>, want: impl Fn(&Button) -> bool) {
    let start = std::time::Instant::now();
    loop {
        h.step();
        let button = h.state_mut().button();
        if want(&button) {
            return;
        }
        assert!(
            start.elapsed().as_secs() < 30,
            "the button never reached the state, it is {button:?}"
        );
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
}

fn slow_worker() -> Vec<String> {
    vec!["/bin/sh".into(), "-c".into(), "exec sleep 600".into()]
}

fn dying_worker(message: &str) -> Vec<String> {
    vec![
        "/bin/sh".into(),
        "-c".into(),
        format!("echo '{message}' >&2; exit 1"),
    ]
}

fn press_key(h: &mut Harness<'_, App>, key: Key, modifiers: Modifiers) {
    h.key_press_modifiers(modifiers, key);
    h.run_steps(2);
}

/// Criterion 1: a picture given as the command-line argument is shown and traced.
#[test]
fn the_picture_of_the_argument_is_shown_and_traced() {
    let dir = tempfile::tempdir().unwrap();
    let h = opened(dir.path(), None);
    let app = h.state();
    assert_eq!(app.picture_name(), Some("input-logo.png"));
    assert!(app.has_result());
    assert_eq!(app.nothing_to_save(), None);
    // 640 x 420 fits the window: 100 %.
    assert_eq!(app.zoom(), Some(1.0));
}

/// Criterion 1: Ctrl+O asks the system dialog and opens its answer.
#[test]
fn ctrl_o_opens_the_picture_the_dialog_returns() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = common::harness(dir.path(), None, fixed(Some(common::logo()), None));
    h.run_steps(2);
    assert!(h.state().picture_name().is_none());
    press_key(&mut h, Key::O, Modifiers::COMMAND);
    common::wait_for_result(&mut h, 60.0);
    assert_eq!(h.state().picture_name(), Some("input-logo.png"));
}

/// Criterion 1: the Open card does the same.
#[test]
fn the_open_card_button_opens_the_picture() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = common::harness(dir.path(), None, fixed(Some(common::logo()), None));
    h.run_steps(2);
    h.get_by_label("Open image").click();
    common::wait_for_result(&mut h, 60.0);
    assert!(h.state().picture_name().is_some());
}

/// Criterion 1: a dropped file opens.
#[test]
fn a_dropped_file_opens() {
    #[derive(Debug)]
    struct Dropped(PathBuf);
    impl egui::DroppedFile for Dropped {
        fn path(&self) -> &Path {
            &self.0
        }
        fn bytes(&self) -> Result<Vec<u8>, String> {
            std::fs::read(&self.0).map_err(|e| e.to_string())
        }
    }
    let dir = tempfile::tempdir().unwrap();
    let mut h = common::harness(dir.path(), None, fixed(None, None));
    h.run_steps(2);
    h.input_mut()
        .dropped_files
        .push(std::sync::Arc::new(Dropped(common::logo())));
    common::wait_for_result(&mut h, 60.0);
    assert_eq!(h.state().picture_name(), Some("input-logo.png"));
}

/// Criterion 2: a file that is not a picture changes nothing and says why.
#[test]
fn an_unreadable_file_keeps_the_picture_and_shows_a_notice() {
    let dir = tempfile::tempdir().unwrap();
    let junk = dir.path().join("notes.txt");
    std::fs::write(&junk, "not a picture").unwrap();
    let mut h = common::harness(dir.path(), Some(common::logo()), fixed(Some(junk), None));
    common::wait_for_result(&mut h, 60.0);
    press_key(&mut h, Key::O, Modifiers::COMMAND);
    common::wait_until(&mut h, 20.0, |app| app.notice_text().is_some());
    let notice = h.state().notice_text().unwrap();
    assert!(notice.starts_with("notes.txt\n"), "{notice}");
    assert!(notice.contains("This file type isn't supported. Use PNG, JPEG or WebP."));
    assert_eq!(h.state().picture_name(), Some("input-logo.png"));
    assert!(h.state().has_result());
}

/// Criterion 5: pan and zoom never trace again.
#[test]
fn pan_and_zoom_do_not_trace_again() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = opened(dir.path(), None);
    let jobs = h.state().trace_jobs();
    let zoom = h.state().zoom().unwrap();
    let at = pos2(640.0, 400.0);
    h.event(Event::PointerMoved(at));
    h.run_steps(1);
    h.event_modifiers(
        Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: vec2(0.0, 40.0),
            phase: egui::TouchPhase::Move,
            source: egui::MouseWheelSource::Wheel,
            modifiers: Modifiers::COMMAND,
        },
        Modifiers::COMMAND,
    );
    h.run_steps(3);
    assert!(h.state().zoom().unwrap() > zoom, "ctrl+scroll zooms in");
    h.event(Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        delta: vec2(30.0, 20.0),
        phase: egui::TouchPhase::Move,
        source: egui::MouseWheelSource::Wheel,
        modifiers: Modifiers::NONE,
    });
    h.run_steps(3);
    press_key(&mut h, Key::Num0, Modifiers::COMMAND);
    assert_eq!(h.state().trace_jobs(), jobs);
}

/// Criterion 8: Esc stops a running trace, the button says Stopped, Retry traces again.
#[test]
fn unix_esc_stops_and_retry_traces_again() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = opened(dir.path(), None);
    h.state_mut().wrap_worker(slow_worker());
    h.get_by_label("Smooth").focus();
    h.run_steps(2);
    press_key(&mut h, Key::ArrowRight, Modifiers::NONE);
    wait_for_button(&mut h, |b| matches!(b, Button::Running(_)));
    press_key(&mut h, Key::Escape, Modifiers::NONE);
    assert_eq!(h.state_mut().button(), Button::Stopped);
    h.state_mut().wrap_worker(Vec::new());
    h.get_by_label("Retry").click();
    common::wait_for_result(&mut h, 60.0);
    assert_eq!(h.state_mut().button(), Button::Rest);
    h.run_steps(1);
}

/// Criterion 8: the x inside the running button stops as well.
#[test]
fn unix_the_x_of_the_running_button_stops() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = opened(dir.path(), None);
    h.state_mut().wrap_worker(slow_worker());
    h.get_by_label("Smooth").focus();
    h.run_steps(2);
    press_key(&mut h, Key::ArrowRight, Modifiers::NONE);
    wait_for_button(&mut h, |b| matches!(b, Button::Running(_)));
    h.get_by_label("Stop (Esc)").click();
    h.run_steps(2);
    assert_eq!(h.state_mut().button(), Button::Stopped);
}

/// Criterion 24: a worker that dies without a reply is "Failed" with its reason, never a crash.
#[test]
fn unix_a_dying_worker_is_failed_with_a_reason() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = opened(dir.path(), None);
    h.state_mut()
        .wrap_worker(dying_worker("memory allocation of 99 bytes failed"));
    h.get_by_label("Smooth").focus();
    h.run_steps(2);
    press_key(&mut h, Key::ArrowRight, Modifiers::NONE);
    wait_for_button(&mut h, |b| matches!(b, Button::Failed(_)));
    match h.state_mut().button() {
        Button::Failed(text) => assert!(
            text.starts_with("Trace failed: out of memory at ")
                && text.ends_with(". Nothing was saved."),
            "{text}"
        ),
        other => panic!("{other:?}"),
    }
    h.state_mut().wrap_worker(dying_worker("thread panicked"));
    press_key(&mut h, Key::ArrowRight, Modifiers::NONE);
    wait_for_button(
        &mut h,
        |b| matches!(b, Button::Failed(t) if t.contains("internal error")),
    );
    // Retry with a healthy worker recovers.
    h.state_mut().wrap_worker(Vec::new());
    h.get_by_label("Retry").click();
    common::wait_for_result(&mut h, 60.0);
}

/// Criterion 19 and 20: Save as… writes a well-formed SVG of the logo's size, atomically.
#[test]
fn save_as_writes_the_svg() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("out.svg");
    let mut h = opened(dir.path(), Some(target.clone()));
    h.get_by_label("Save as\u{2026}").click();
    h.run_steps(3);
    let text = std::fs::read_to_string(&target).expect("the file was written");
    let doc = roxmltree::Document::parse(&text).unwrap();
    let root = doc.root_element();
    // 640 x 420 px at 96 dpi.
    assert_eq!(root.attribute("width"), Some("169.333mm"));
    assert_eq!(root.attribute("height"), Some("111.125mm"));
    assert!(text.contains("inkscape:label=\"#000000\""));
    let leftovers: Vec<_> = std::fs::read_dir(dir.path())
        .unwrap()
        .filter_map(Result::ok)
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.starts_with(".tmp"))
        .collect();
    assert!(leftovers.is_empty(), "{leftovers:?}");
    // The app stays open with the picture and the values unchanged.
    assert_eq!(h.state().picture_name(), Some("input-logo.png"));
    assert_eq!(h.state().settings().smooth, 80);
}

/// Criterion 19: Enter on the canvas does what the button does.
#[test]
fn enter_on_the_canvas_saves() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("enter.svg");
    let mut h = opened(dir.path(), Some(target.clone()));
    press_key(&mut h, Key::Enter, Modifiers::NONE);
    assert!(target.exists());
}

/// Criterion 19: a cancelled dialog writes nothing.
#[test]
fn a_cancelled_save_dialog_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = opened(dir.path(), None);
    h.get_by_label("Save as\u{2026}").click();
    h.run_steps(3);
    assert!(
        !std::fs::read_dir(dir.path())
            .unwrap()
            .filter_map(Result::ok)
            .any(|e| e.path().extension().is_some_and(|x| x == "svg"))
    );
}

/// Criterion 19: a write error shows Failed with the reason, and Retry writes once it is fixed.
#[test]
fn a_write_error_is_failed_and_retry_writes() {
    let dir = tempfile::tempdir().unwrap();
    let missing_parent_is_made = dir.path().join("readonly");
    std::fs::create_dir(&missing_parent_is_made).unwrap();
    let mut perms = std::fs::metadata(&missing_parent_is_made)
        .unwrap()
        .permissions();
    std::os::unix::fs::PermissionsExt::set_mode(&mut perms, 0o500);
    std::fs::set_permissions(&missing_parent_is_made, perms.clone()).unwrap();
    let target = missing_parent_is_made.join("out.svg");
    let mut h = opened(dir.path(), Some(target.clone()));
    h.get_by_label("Save as\u{2026}").click();
    h.run_steps(3);
    match h.state_mut().button() {
        Button::Failed(text) => assert!(
            text.starts_with("Could not save out.svg: ")
                && text.ends_with(". Nothing was written."),
            "{text}"
        ),
        other => {
            // Running as root ignores the permission bits; nothing to test then.
            assert!(target.exists(), "{other:?}");
            return;
        }
    }
    std::os::unix::fs::PermissionsExt::set_mode(&mut perms, 0o700);
    std::fs::set_permissions(&missing_parent_is_made, perms).unwrap();
    h.get_by_label("Retry").click();
    h.run_steps(3);
    assert!(target.exists());
    assert_eq!(h.state_mut().button(), Button::Rest);
}

/// Criterion 18: values are remembered between starts in the settings file.
#[test]
fn values_are_remembered_between_starts() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = opened(dir.path(), None);
    h.get_by_label("Smooth").focus();
    h.run_steps(2);
    press_key(&mut h, Key::ArrowRight, Modifiers::SHIFT);
    common::wait_for_result(&mut h, 60.0);
    let text = std::fs::read_to_string(dir.path().join("settings.toml")).unwrap();
    assert!(text.contains("smooth = 90"), "{text}");
    drop(h);
    let again = common::harness(dir.path(), None, fixed(None, None));
    assert_eq!(again.state().settings().smooth, 90);
}

/// Criterion 11: picking a preset sets its values; an edit reads Custom.
#[test]
fn a_preset_sets_its_values_and_an_edit_reads_custom() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = opened(dir.path(), None);
    h.get_by_label("Preset").click();
    h.run_steps(2);
    h.get_by_label("Photo").click();
    common::wait_for_result(&mut h, 60.0);
    assert_eq!(h.state().settings().preset_label(), "Photo");
    assert_eq!(h.state().settings().smooth, 50);
    h.get_by_label("Smooth").focus();
    h.run_steps(2);
    press_key(&mut h, Key::ArrowRight, Modifiers::NONE);
    assert_eq!(h.state().settings().preset_label(), "Custom");
}

/// Criterion 13: Enter opens the number entry, a typed value is clamped, Escape restores.
#[test]
fn a_typed_value_is_clamped_and_escape_restores() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = opened(dir.path(), None);
    h.get_by_label("Smooth").focus();
    h.run_steps(2);
    press_key(&mut h, Key::Enter, Modifiers::NONE);
    h.run_steps(2);
    h.event(Event::Text("250".into()));
    h.run_steps(2);
    press_key(&mut h, Key::Enter, Modifiers::NONE);
    common::wait_for_result(&mut h, 60.0);
    assert_eq!(h.state().settings().smooth, 100);
    h.get_by_label("Smooth").focus();
    h.run_steps(2);
    press_key(&mut h, Key::F2, Modifiers::NONE);
    h.run_steps(2);
    h.event(Event::Text("7".into()));
    h.run_steps(2);
    press_key(&mut h, Key::Escape, Modifiers::NONE);
    h.run_steps(2);
    assert_eq!(
        h.state().settings().smooth,
        100,
        "Escape restores the value"
    );
}

/// Criterion 3 of ux-notes section 6: with no picture the bar offers only Open image….
#[test]
fn without_a_picture_only_open_is_offered() {
    let dir = tempfile::tempdir().unwrap();
    let h = common::harness(dir.path(), None, fixed(None, None));
    assert!(h.query_by_label("Open image\u{2026}").is_some());
    assert!(h.query_by_label("Smooth").is_none());
    assert!(h.query_by_label("Save as\u{2026}").is_none());
}

/// A picture of more than 4 Mpx is previewed from the reduced copy; saving traces the full one.
#[test]
fn a_big_picture_saves_from_the_full_trace() {
    let dir = tempfile::tempdir().unwrap();
    let big = dir.path().join("big.png");
    let (w, h_px) = (2400_u32, 2000_u32);
    let img = image::RgbImage::from_fn(w, h_px, |x, y| {
        if (x / 300 + y / 250) % 2 == 0 {
            image::Rgb([0, 0, 0])
        } else {
            image::Rgb([255, 255, 255])
        }
    });
    img.save(&big).unwrap();
    let target = dir.path().join("big.svg");
    let mut h = common::harness(dir.path(), Some(big), fixed(None, Some(target.clone())));
    common::wait_for_result(&mut h, 120.0);
    let jobs = h.state().trace_jobs();
    h.get_by_label("Save as\u{2026}").click();
    let start = std::time::Instant::now();
    while !target.exists() {
        h.step();
        std::thread::sleep(std::time::Duration::from_millis(20));
        assert!(
            start.elapsed().as_secs() < 120,
            "the file was never written"
        );
    }
    assert_eq!(
        h.state().trace_jobs(),
        jobs + 1,
        "one full trace before writing"
    );
    let text = std::fs::read_to_string(&target).unwrap();
    assert!(roxmltree::Document::parse(&text).is_ok());
    // 2400 px at 96 dpi.
    assert!(text.contains("width=\"635mm\""));
}

/// Section 1 of ux-notes: the window title is "Tracyo" without a picture, else "<file> \u{2014} Tracyo".
#[test]
fn the_window_title_follows_the_picture() {
    let dir = tempfile::tempdir().unwrap();
    let mut empty = common::harness(dir.path(), None, fixed(None, None));
    empty.run_steps(2);
    assert_eq!(empty.state().window_title(), Some("Tracyo"));
    let h = opened(dir.path(), None);
    assert_eq!(
        h.state().window_title(),
        Some("input-logo.png \u{2014} Tracyo")
    );
}
