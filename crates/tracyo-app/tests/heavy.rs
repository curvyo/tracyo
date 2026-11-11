//! Tests with a 12 Mpx picture: the UI thread's frame budget and running out of memory.
//! They run alone (`.config/nextest.toml`).

mod common;

use std::path::Path;

use egui_kittest::kittest::Queryable;
use tracyo_app::App;
use tracyo_app::dialogs::Dialogs;
use tracyo_core::progress::Button;

/// A 4000 x 3000 picture of circles and bars.
fn big_picture(path: &Path, width: u32, height: u32) {
    let image = image::RgbImage::from_fn(width, height, |x, y| {
        let (cx, cy) = (f64::from(x % 400) - 200.0, f64::from(y % 400) - 200.0);
        let circle = cx * cx + cy * cy < 150.0 * 150.0;
        let bar = (y / 40) % 7 == 0 && x % 400 > 20;
        if circle || bar { image::Rgb([20, 20, 20]) } else { image::Rgb([250, 250, 250]) }
    });
    image.save(path).unwrap();
}

#[cfg(unix)]
fn thread_cpu() -> std::time::Duration {
    let t = rustix::time::clock_gettime(rustix::time::ClockId::ThreadCPUTime);
    std::time::Duration::new(t.tv_sec.try_into().unwrap_or(0), t.tv_nsec.try_into().unwrap_or(0))
}

/// Criterion 7: a frame of the UI thread takes under 50 ms of CPU time on a 12 Mpx picture,
/// while it opens, while it traces and while it is panned; never wall time.
#[cfg(unix)]
#[test]
fn a_frame_on_a_12_mpx_picture_stays_under_50_ms_of_cpu_time() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("big.png");
    big_picture(&path, 4000, 3000);
    let mut h = common::harness(dir.path(), Some(path), Dialogs::Fixed { open: None, save: None });
    let mut worst = std::time::Duration::ZERO;
    let mut frames = 0;
    let start = std::time::Instant::now();
    while !h.state().has_result() {
        let before = thread_cpu();
        h.step();
        let spent = thread_cpu() - before;
        worst = worst.max(spent);
        if spent.as_millis() > 30 {
            println!("slow frame {frames} while loading/tracing: {spent:?}");
        }
        frames += 1;
        assert!(start.elapsed().as_secs() < 300, "never traced");
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    // A drag of the picture and a render of the settled state.
    for _ in 0..20 {
        h.event(eframe::egui::Event::MouseWheel {
            unit: eframe::egui::MouseWheelUnit::Point,
            delta: eframe::egui::vec2(25.0, 10.0),
            phase: eframe::egui::TouchPhase::Move,
            source: eframe::egui::MouseWheelSource::Wheel,
            modifiers: eframe::egui::Modifiers::NONE,
        });
        let before = thread_cpu();
        h.step();
        let spent = thread_cpu() - before;
        worst = worst.max(spent);
        if spent.as_millis() > 30 {
            println!("slow frame {frames} while panning: {spent:?}");
        }
        frames += 1;
    }
    println!("{frames} frames, worst UI-thread CPU time {worst:?}");
    assert!(worst.as_millis() < 50, "worst frame {worst:?}");
}

/// Criterion 24: running out of memory in the worker shows Failed with the reason, no crash.
#[cfg(target_os = "linux")]
#[test]
fn running_out_of_memory_shows_failed() {
    use std::io::Write;
    use std::process::{Command, Stdio};
    let Ok(prlimit) = Command::new("prlimit").arg("--version").stdout(Stdio::null()).status() else {
        eprintln!("prlimit is not installed: test skipped");
        return;
    };
    let _ = prlimit;
    // The address space an idle worker needs, then 20 MB more: a 3.9 Mpx picture (15.6 MB, held
    // three times while it arrives) cannot fit.
    let mut idle = Command::new(env!("CARGO_BIN_EXE_tracyo")).arg("--trace-worker").stdin(Stdio::piped()).stdout(Stdio::null()).stderr(Stdio::null()).spawn().unwrap();
    std::thread::sleep(std::time::Duration::from_millis(300));
    let status = std::fs::read_to_string(format!("/proc/{}/status", idle.id())).unwrap();
    let vm_kb: u64 = status.lines().find_map(|l| l.strip_prefix("VmSize:")).and_then(|v| v.trim().trim_end_matches("kB").trim().parse().ok()).unwrap();
    let _ = idle.stdin.take().map(|mut s| s.flush());
    idle.kill().unwrap();
    idle.wait().unwrap();
    let limit = vm_kb * 1024 + 20_000_000;

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("mid.png");
    big_picture(&path, 2000, 1950);
    let mut h = common::harness(dir.path(), None, Dialogs::Fixed { open: None, save: None });
    h.state_mut().wrap_worker(vec!["prlimit".into(), format!("--as={limit}")]);
    let ctx = h.ctx.clone();
    h.state_mut().open_path(&ctx, path);
    let start = std::time::Instant::now();
    loop {
        h.step();
        if let Button::Failed(text) = h.state_mut().button() {
            assert!(text.starts_with("Trace failed: out of memory at 4 Mpx"), "{text}");
            break;
        }
        assert!(start.elapsed().as_secs() < 120, "no Failed state");
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    // The app is still alive and the picture is still shown.
    assert_eq!(h.state().picture_name(), Some("mid.png"));
    assert!(h.query_by_label("Retry").is_some());
}
