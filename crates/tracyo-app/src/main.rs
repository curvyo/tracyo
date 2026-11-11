//! The `tracyo` binary: the window, or the trace worker with `--trace-worker`.

#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

use std::ffi::OsString;
use std::path::PathBuf;

fn main() -> anyhow::Result<()> {
    let first: Option<OsString> = std::env::args_os().nth(1);
    if first.as_deref() == Some(std::ffi::OsStr::new("--trace-worker")) {
        tracyo_app::worker::serve(std::io::stdin().lock(), std::io::stdout().lock())?;
        return Ok(());
    }
    tracyo_app::run(first.map(PathBuf::from)).map_err(|e| anyhow::anyhow!("{e}"))
}
