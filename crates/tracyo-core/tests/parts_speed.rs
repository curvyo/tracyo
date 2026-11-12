//! Criterion 25 of specification 0002, in-process: the thread CPU time of one full trace of 8
//! parts that tile a 12 Mpx picture is at most 1.3 times that of one whole-picture trace; 8 parts
//! that each cover the whole picture are reported (at most 9 times; later parts win, so only the
//! last is traced). Timed in release only (`ci.yml` job `perf`); never wall time.

#![allow(
    clippy::unwrap_used,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap
)]

#[cfg(unix)]
mod timing {
    use std::time::Duration;

    use tracyo_core::clip::Clip;
    use tracyo_core::job::Session;
    use tracyo_core::picture::Picture;
    use tracyo_core::regions::{MaskMode, PxRect, Region};
    use tracyo_core::request::{TraceRequest, pixel_map, size_mm};
    use tracyo_core::settings::Settings;
    use tracyo_core::stack::{PartArea, part_clips, parts_job};

    const WIDTH: u32 = 4000;
    const HEIGHT: u32 = 3000;

    /// A 4000 x 3000 picture of circles and bars.
    fn picture() -> Picture {
        let mut rgba = Vec::with_capacity((WIDTH * HEIGHT * 4) as usize);
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                let (cx, cy) = (f64::from(x % 400) - 200.0, f64::from(y % 400) - 200.0);
                let circle = cx * cx + cy * cy < 150.0 * 150.0;
                let bar = (y / 40) % 7 == 0 && x % 400 > 20;
                let v = if circle || bar { 20 } else { 250 };
                rgba.extend([v, v, v, 255]);
            }
        }
        Picture {
            width: WIDTH,
            height: HEIGHT,
            rgba,
        }
    }

    fn thread_cpu() -> Duration {
        let t = rustix::time::clock_gettime(rustix::time::ClockId::ThreadCPUTime);
        Duration::new(
            t.tv_sec.try_into().unwrap_or(0),
            t.tv_nsec.try_into().unwrap_or(0),
        )
    }

    fn request() -> TraceRequest {
        TraceRequest {
            threshold: 128,
            settings: Settings::default(),
            selection: vectrace::Selection::Brightness,
            invert: false,
            drop_border: false,
            colours: None,
            reduced: false,
            shapes: false,
            lines: Vec::new(),
            clip: None,
        }
    }

    fn mask(x0: i32, y0: i32, x1: i32, y1: i32) -> PartArea {
        let region = Region::rect(PxRect::new(x0, y0, x1, y1), false);
        let clip = Clip::of_regions(&[region], MaskMode::Inside, pixel_map(WIDTH, WIDTH))
            .unwrap()
            .unwrap();
        PartArea::Mask(clip)
    }

    /// The CPU time of the job of `areas`, each part traced with the same settings.
    fn cpu_of_parts(session: &mut Session, areas: &[PartArea]) -> (Duration, usize) {
        let clips = part_clips(
            areas,
            kurbo::Rect::new(0.0, 0.0, size_mm(WIDTH), size_mm(HEIGHT)),
        );
        let job = parts_job(&clips, vec![request(); areas.len()], 0).unwrap();
        let traced = job.jobs.len();
        let start = thread_cpu();
        let numbers = session.run_parts(&job, &mut |_, _| {}, &mut |_| {});
        let spent = thread_cpu().checked_sub(start).unwrap();
        assert!(vectrace::wire::read(&numbers).is_some_and(|o| o.paths > 0));
        (spent, traced)
    }

    #[test]
    #[cfg_attr(
        debug_assertions,
        ignore = "timing: measured in release, see ci.yml job perf"
    )]
    fn eight_tiled_parts_cost_at_most_1_3_times_one_whole_trace_and_covering_parts_at_most_9() {
        let mut session = Session::new(picture());
        // One whole-picture trace, the best of two (the first loads the caches).
        let whole = (0..2)
            .map(|_| {
                let start = thread_cpu();
                let numbers = session.run(&request(), &mut |_| {});
                let spent = thread_cpu().checked_sub(start).unwrap();
                assert!(vectrace::wire::read(&numbers).is_some());
                spent
            })
            .min()
            .unwrap();
        let (w, h) = (WIDTH as i32 / 4, HEIGHT as i32 / 2);
        let squares: Vec<PartArea> = (0..8)
            .map(|k| {
                let (cx, cy) = (k % 4, k / 4);
                mask(cx * w, cy * h, (cx + 1) * w, (cy + 1) * h)
            })
            .collect();
        let (tiled, traced) = cpu_of_parts(&mut session, &squares);
        let covering: Vec<PartArea> = (0..8)
            .map(|_| mask(0, 0, WIDTH as i32, HEIGHT as i32))
            .collect();
        let (all, covering_traced) = cpu_of_parts(&mut session, &covering);
        let ratio = tiled.as_secs_f64() / whole.as_secs_f64();
        let covering_ratio = all.as_secs_f64() / whole.as_secs_f64();
        println!(
            "whole {whole:?}; 8 tiled parts ({traced} traced) {tiled:?} = {ratio:.2}x; \
             8 covering parts ({covering_traced} traced) {all:?} = {covering_ratio:.2}x"
        );
        assert_eq!(traced, 8);
        assert!(
            ratio <= 1.3,
            "8 tiled parts cost {ratio:.2} times one trace"
        );
        assert!(
            covering_ratio <= 9.0,
            "8 covering parts cost {covering_ratio:.2} times one trace"
        );
    }
}
