//! The real binary starts under Xvfb and keeps running until it is stopped.

use std::process::{Command, Stdio};

/// `timeout 5 xvfb-run tracyo` must end by the timeout (status 124), not by an error.
#[test]
fn the_window_starts_under_xvfb() {
    if Command::new("xvfb-run").arg("--help").stdout(Stdio::null()).stderr(Stdio::null()).status().is_err() {
        eprintln!("xvfb-run is not installed: start test skipped");
        return;
    }
    let status = Command::new("timeout")
        .args(["5", "xvfb-run", "-a", env!("CARGO_BIN_EXE_tracyo")])
        .env("XDG_CONFIG_HOME", tempfile::tempdir().unwrap().path())
        .env_remove("WAYLAND_DISPLAY")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(124), "the app ended by itself: {status:?}");
}
