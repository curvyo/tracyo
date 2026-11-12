//! A whole session: open the logo, trace, save. CI runs it once as it is and once in a network-less
//! namespace (`unshare -n`) and compares the files (criterion 23).

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use egui_kittest::kittest::Queryable;
use tracyo_app::dialogs::Dialogs;

#[test]
fn open_trace_and_save_the_logo() {
    let dir = tempfile::tempdir().unwrap();
    let target = std::env::var_os("TRACYO_SESSION_OUT")
        .map_or_else(|| dir.path().join("logo.svg"), std::path::PathBuf::from);
    let mut h = common::harness(
        dir.path(),
        Some(common::logo()),
        Dialogs::Fixed {
            open: None,
            save: Some(target.clone()),
        },
    );
    common::wait_for_result(&mut h, 120.0);
    h.get_by_label("Save as\u{2026}").click();
    h.run_steps(3);
    let text = std::fs::read_to_string(&target).expect("the SVG was written");
    let golden = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/logo-session.svg"
    );
    if std::env::var_os("TRACYO_UPDATE_GOLDEN").is_some() {
        std::fs::write(golden, &text).unwrap();
    }
    assert_eq!(
        text,
        std::fs::read_to_string(golden).unwrap(),
        "the saved SVG differs from the golden file"
    );
}

/// The picture of the OCR tests, read as text and traced with its shapes (criterion 23b): the
/// model is inside the executable, so nothing is fetched.
#[test]
fn open_read_text_recognise_shapes_and_save() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("settings.toml"),
        "recognise_text = true\nrecognise_shapes = true\n",
    )
    .unwrap();
    let target = std::env::var_os("TRACYO_SESSION_TEXT_OUT").map_or_else(
        || dir.path().join("precision.svg"),
        std::path::PathBuf::from,
    );
    let picture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../tracyo-core/tests/fixtures/pictures/precision.png");
    let mut h = common::harness(
        dir.path(),
        Some(picture),
        Dialogs::Fixed {
            open: None,
            save: Some(target.clone()),
        },
    );
    common::wait_for_result(&mut h, 240.0);
    let (texts, _) = h.state().recognised();
    assert!(texts >= 3, "{texts} text lines set");
    h.get_by_label("Save as\u{2026}").click();
    h.run_steps(3);
    let text = std::fs::read_to_string(&target).expect("the SVG was written");
    let doc = roxmltree::Document::parse(&text).unwrap();
    assert!(doc.descendants().filter(|n| n.has_tag_name("text")).count() >= 3);
    assert!(text.contains("BERATUNG"));
}
