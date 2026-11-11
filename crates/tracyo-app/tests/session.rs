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
