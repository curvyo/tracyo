//! Criterion 27 of specification 0002: invalid geometry never panics. Fixed-seed fuzz cases
//! (degenerate, huge, collinear, repeated and self-crossing regions, far off the picture) go
//! through the plan, the wash, the cut, the kept pixels and the join; the answer may be a notice
//! or an empty mask, never a panic.

#![allow(clippy::unwrap_used, clippy::cast_possible_truncation)]

mod common;

use tracyo_core::clip::{Clip, MaskPlan, plan};
use tracyo_core::join::join;
use tracyo_core::regions::{MaskMode, PxPoint, PxRect, Region};
use tracyo_core::request::size_mm;
use tracyo_core::stack::{PartArea, part_clips};
use tracyo_core::wash;
use vectrace::mask::Mask;

use common::{full_map, p1, request_at, trace};

const CASES: usize = 600;

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
    /// A coordinate: mostly on the picture, sometimes far off it, at the i32 limits or repeated.
    fn coord(&mut self) -> i32 {
        match self.below(24) {
            0 => i32::MAX - self.below(3) as i32,
            1 => i32::MIN + self.below(3) as i32,
            2 => (self.below(2_000_001) as i32 - 1_000_000) * 1000,
            3..=5 => 500,
            _ => self.below(1200) as i32 - 100,
        }
    }
    fn point(&mut self) -> PxPoint {
        PxPoint {
            x: self.coord(),
            y: self.coord(),
        }
    }
}

fn region(rng: &mut Rng) -> Option<Region> {
    let subtract = rng.below(3) == 0;
    if rng.below(2) == 0 {
        return Some(Region::rect(
            PxRect::new(rng.coord(), rng.coord(), rng.coord(), rng.coord()),
            subtract,
        ));
    }
    let n = rng.below(14) as usize;
    let mut corners: Vec<PxPoint> = (0..n).map(|_| rng.point()).collect();
    // Repeated, collinear and returning corners.
    if rng.below(3) == 0 && !corners.is_empty() {
        let first = corners[0];
        corners.push(first);
        corners.push(first);
    }
    if rng.below(4) == 0 {
        let line: Vec<PxPoint> = (0..5).map(|i| PxPoint { x: i * 7, y: i * 7 }).collect();
        corners = line;
    }
    Region::lasso(&corners, subtract)
}

fn regions(rng: &mut Rng) -> Vec<Region> {
    (0..rng.below(7)).filter_map(|_| region(rng)).collect()
}

#[test]
fn invalid_regions_never_panic_in_the_plan_the_wash_the_cut_and_the_kept_pixels() {
    let picture = p1();
    let whole = trace(&picture, &request_at(50.0));
    let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
    let (mut clips, mut notices) = (0, 0);
    for _ in 0..CASES {
        let regions = regions(&mut rng);
        for mode in [MaskMode::Inside, MaskMode::Outside] {
            let _ = wash::not_traced(&regions, mode, PxRect::of_picture(1000, 1000));
            match plan(&regions, mode, full_map(1000), 1000, 1000) {
                MaskPlan::Clip(clip) => {
                    clips += 1;
                    let _ = clip.clip(&whole.outlines);
                    let _ = clip.kept_pixels(full_map(1000), 1000, 1000);
                    let _ = clip.to_units();
                }
                MaskPlan::Nothing(_) => notices += 1,
                MaskPlan::Whole => {}
            }
        }
    }
    println!("{clips} clips, {notices} notices");
    assert!(clips > 50, "only {clips} cases made a clip");
    assert!(notices > 5, "only {notices} cases gave a notice");
}

#[test]
fn invalid_regions_never_panic_in_the_stack_and_the_join() {
    let picture = p1();
    let whole = trace(&picture, &request_at(50.0));
    let side = size_mm(1000);
    let mut rng = Rng(0x2545_f491_4f6c_dd1d);
    for _ in 0..CASES / 3 {
        let count = 2 + rng.below(4) as usize;
        let areas: Vec<PartArea> = (0..count)
            .map(|_| {
                let mode = if rng.below(2) == 0 {
                    MaskMode::Inside
                } else {
                    MaskMode::Outside
                };
                match Clip::of_regions(&regions(&mut rng), mode, full_map(1000)) {
                    Ok(Some(clip)) => PartArea::Mask(clip),
                    Ok(None) if rng.below(2) == 0 => PartArea::Whole,
                    _ => PartArea::Nothing,
                }
            })
            .collect();
        let clips = part_clips(&areas, kurbo::Rect::new(0.0, 0.0, side, side));
        assert_eq!(clips.len(), areas.len());
        let outputs: Vec<_> = clips
            .iter()
            .map(|c| match c.region() {
                Some(clip) => {
                    let outlines = clip.clip(&whole.outlines).outlines;
                    vectrace::wire::Output {
                        paths: outlines.len(),
                        outlines,
                        ..whole.clone()
                    }
                }
                None => whole.clone(),
            })
            .collect();
        let pairs: Vec<_> = outputs.iter().zip(clips.iter()).collect();
        let _ = join(&pairs, 0);
    }
}
