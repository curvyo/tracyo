//! Property tests of the join (ADR 0008 section 6): rectangles on a 6 px lattice, whose edges
//! coincide with the straight, pixel-aligned edges of P1's trace, cut into an Inside and an
//! Outside part; the join keeps the area, leaves no seam line, obeys the order rule, gives the
//! same bytes twice and does not change when its parts are cut and joined again. A fixed-seed
//! generator, 2,000 cases.

#![allow(clippy::unwrap_used)]

mod common;

use i_overlay::core::overlay_rule::OverlayRule;
use kurbo::{BezPath, PathEl, Shape};
use tracyo_core::clip::Clip;
use tracyo_core::grid;
use tracyo_core::join::join;
use tracyo_core::regions::{MaskMode, PxPoint, PxRect, Region};
use tracyo_core::request::size_mm;
use tracyo_core::stack::{PartArea, PartClip, part_clips};
use vectrace::mask::Mask;
use vectrace::wire::Output;

use common::{full_map, p1, request_at, trace};

const CASES: usize = 2_000;
const LATTICE: i32 = 6;

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
    fn corner(&mut self) -> i32 {
        i32::try_from(self.below(190)).unwrap() * LATTICE - 60
    }
}

fn regions(rng: &mut Rng) -> Vec<Region> {
    let added = 1 + rng.below(3);
    let taken = rng.below(2);
    (0..added + taken)
        .map(|i| {
            Region::rect(
                PxRect::new(rng.corner(), rng.corner(), rng.corner(), rng.corner()),
                i >= added,
            )
        })
        .collect()
}

fn area_of(regions: &[Region], mode: MaskMode) -> Option<PartArea> {
    Clip::of_regions(regions, mode, full_map(1000))
        .unwrap()
        .map(PartArea::Mask)
}

/// What vectrace makes of `whole` through `clip`: the traced outlines cut, the counts as they are.
fn cut(whole: &Output, clip: Option<&Clip>) -> Output {
    let Some(clip) = clip else {
        return whole.clone();
    };
    let outlines = clip.clip(&whole.outlines).outlines;
    Output {
        paths: outlines.len(),
        outlines,
        ..whole.clone()
    }
}

fn clips_of(areas: &[PartArea]) -> Vec<PartClip> {
    let side = size_mm(1000);
    part_clips(areas, kurbo::Rect::new(0.0, 0.0, side, side))
}

fn rings(outlines: &[BezPath]) -> Vec<grid::Path64> {
    outlines
        .iter()
        .flat_map(|p| grid::flatten_snap(p).unwrap())
        .collect()
}

/// The length of the polygons of a mask, in mm.
fn mask_length(polygons: &[grid::Path64]) -> f64 {
    polygons
        .iter()
        .map(|p| {
            (0..p.len())
                .map(|i| {
                    let (a, b) = (p[i], p[(i + 1) % p.len()]);
                    grid::to_mm(a.x - b.x).hypot(grid::to_mm(a.y - b.y))
                })
                .sum::<f64>()
        })
        .sum()
}

/// The length of all outlines in mm.
fn length(outlines: &[BezPath]) -> f64 {
    outlines.iter().map(|o| o.perimeter(0.001)).sum()
}

fn net(outlines: &[BezPath]) -> f64 {
    outlines.iter().map(Shape::area).sum()
}

fn points(path: &BezPath) -> Vec<kurbo::Point> {
    path.elements()
        .iter()
        .filter_map(|e| match e {
            PathEl::MoveTo(p) | PathEl::LineTo(p) | PathEl::CurveTo(_, _, p) => Some(*p),
            _ => None,
        })
        .collect()
}

/// The longest stretch along which two segments lie on one axis-parallel line (0.01 mm bins).
fn longest_shared_run(outlines: &[BezPath]) -> f64 {
    let mut vertical: Vec<(i64, f64, f64)> = Vec::new();
    let mut horizontal: Vec<(i64, f64, f64)> = Vec::new();
    for outline in outlines {
        let pts = points(outline);
        for k in 0..pts.len() {
            let (a, b) = (pts[k], pts[(k + 1) % pts.len()]);
            #[allow(clippy::cast_possible_truncation)]
            let key = |v: f64| (v * 100.0).round() as i64;
            if (a.x - b.x).abs() < 0.0006 {
                vertical.push((key(a.x), a.y.min(b.y), a.y.max(b.y)));
            } else if (a.y - b.y).abs() < 0.0006 {
                horizontal.push((key(a.y), a.x.min(b.x), a.x.max(b.x)));
            }
        }
    }
    let mut longest = 0.0_f64;
    for set in [&mut vertical, &mut horizontal] {
        set.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.total_cmp(&b.1)));
        for i in 0..set.len() {
            for j in i + 1..set.len() {
                if set[j].0 != set[i].0 {
                    break;
                }
                longest = longest.max(set[i].2.min(set[j].2) - set[j].1.max(set[i].1));
            }
        }
    }
    longest
}

/// Rectangles and lassos: a lasso has 3 to 6 corners on whole pixels from 60 px beyond the picture's
/// edges, so its sides are slanted and may cross each other.
fn mixed_regions(rng: &mut Rng) -> Vec<Region> {
    let added = 1 + rng.below(3);
    let taken = rng.below(2);
    (0..added + taken)
        .map(|i| {
            let subtract = i >= added;
            if rng.below(3) == 0 {
                return Region::rect(
                    PxRect::new(rng.corner(), rng.corner(), rng.corner(), rng.corner()),
                    subtract,
                );
            }
            let n = 3 + rng.below(4);
            let corners: Vec<PxPoint> = (0..n)
                .map(|_| {
                    // invariant: below(1120) fits an i32.
                    let mut any = || i32::try_from(rng.below(1120)).unwrap() - 60;
                    let x = any();
                    PxPoint::new(x, any())
                })
                .collect();
            Region::lasso(&corners, subtract)
                .unwrap_or_else(|| Region::rect(PxRect::new(100, 100, 200, 200), subtract))
        })
        .collect()
}

/// The longest stretch along which two segments of any direction lie on one line (within 0.002 mm
/// of each other, 0.001 rad), for the outlines' segments longer than 0.5 mm.
fn longest_shared_slanted_run(outlines: &[BezPath]) -> f64 {
    let mut segments: Vec<(kurbo::Point, kurbo::Point)> = Vec::new();
    for outline in outlines {
        let pts = points(outline);
        for k in 0..pts.len() {
            let (a, b) = (pts[k], pts[(k + 1) % pts.len()]);
            if a.distance(b) > 0.5 {
                segments.push((a, b));
            }
        }
    }
    let mut longest = 0.0_f64;
    for (i, &(a, b)) in segments.iter().enumerate() {
        let (d, len) = (b - a, a.distance(b));
        for &(c, e) in &segments[i + 1..] {
            let (f, flen) = (e - c, c.distance(e));
            let parallel = (d.x * f.y - d.y * f.x).abs() / (len * flen) < 0.001;
            let on_line = |p: kurbo::Point| ((p - a).cross(d)).abs() / len <= 0.002;
            if parallel && on_line(c) && on_line(e) {
                let along = |p: kurbo::Point| (p - a).dot(d) / len;
                let (lo, hi) = (
                    along(c).min(along(e)).max(0.0),
                    along(c).max(along(e)).min(len),
                );
                longest = longest.max(hi - lo);
            }
        }
    }
    longest
}

fn join_checks(seed: u64, cases: usize, make: fn(&mut Rng) -> Vec<Region>, slanted: bool) {
    let whole = trace(&p1(), &request_at(70.0));
    let mut rng = Rng(seed);
    let (mut seams, mut checked) = (0, 0);
    for case in 0..cases {
        let regions = make(&mut rng);
        let (Some(inside), Some(outside)) = (
            area_of(&regions, MaskMode::Inside),
            area_of(&regions, MaskMode::Outside),
        ) else {
            continue;
        };
        let (PartArea::Mask(ci), PartArea::Mask(co)) = (&inside, &outside) else {
            unreachable!()
        };
        let (a, b) = (cut(&whole, Some(co)), cut(&whole, Some(ci)));
        let (ka, kb) = (PartClip::Region(co.clone()), PartClip::Region(ci.clone()));
        let joined = join(&[(&a, &ka), (&b, &kb)], 1);
        checked += 1;
        seams += usize::from(
            whole
                .outlines
                .iter()
                .any(|o| ci.edge_meets(tracyo_core::clip::crossing_box(o))),
        );
        // Area conservation: the join is the two parts together.
        let want = net(&a.outlines) + net(&b.outlines);
        let got = net(&joined.outlines);
        assert!(
            (got - want).abs() <= 0.01,
            "case {case}: {got} vs {want} for {regions:?}"
        );
        // No seam line.
        let shared = if slanted {
            longest_shared_slanted_run(&joined.outlines)
        } else {
            longest_shared_run(&joined.outlines)
        };
        assert!(
            shared <= 0.01,
            "case {case}: {shared} mm shared for {regions:?}"
        );
        // Determinism.
        let again = join(&[(&a, &ka), (&b, &kb)], 1);
        assert_eq!(format!("{joined:?}"), format!("{again:?}"), "case {case}");
        // Idempotence: cut the join along the same clips and join the pieces again. With slanted
        // seams a vertex a grid unit off a side may go or an outline split at a pinch of that
        // width, so the area stays within one grid unit (0.001 mm) times the outlines' length
        // instead of the bytes.
        let (a2, b2) = (cut(&joined, Some(co)), cut(&joined, Some(ci)));
        let twice = join(&[(&a2, &ka), (&b2, &kb)], 1);
        let third = join(
            &[(&cut(&twice, Some(co)), &ka), (&cut(&twice, Some(ci)), &kb)],
            1,
        );
        if slanted {
            let tol = 0.001 * length(&joined.outlines);
            for (label, outlines) in [("second", &twice.outlines), ("third", &third.outlines)] {
                assert!(
                    (net(outlines) - got).abs() <= tol.max(0.01),
                    "case {case}: area after the {label} join, {regions:?}"
                );
            }
            continue;
        }
        assert!(
            (net(&twice.outlines) - got).abs() <= 0.01,
            "case {case}: area after a second join, {regions:?}"
        );
        assert_eq!(
            twice.outlines.len(),
            joined.outlines.len(),
            "case {case}: outline count after a second join, {regions:?}"
        );
        assert_eq!(
            format!("{:?}", third.outlines),
            format!("{:?}", twice.outlines),
            "case {case}: the join is a fixed point, {regions:?}"
        );
    }
    assert!(
        checked > cases / 2 && seams > cases / 4,
        "{checked} {seams}"
    );
}

#[test]
fn the_join_of_inside_and_outside_keeps_the_area_leaves_no_seam_and_is_stable() {
    join_checks(0xC2B2_AE3D_27D4_EB4F, CASES, regions, false);
}

fn order_checks(seed: u64, cases: usize, make: fn(&mut Rng) -> Vec<Region>, slanted: bool) {
    let low = trace(&p1(), &request_at(40.0));
    let high = trace(&p1(), &request_at(70.0));
    let mut rng = Rng(seed);
    let mut checked = 0;
    for case in 0..cases {
        let regions = make(&mut rng);
        let Some(top) = area_of(&regions, MaskMode::Inside) else {
            continue;
        };
        let PartArea::Mask(top_clip) = &top else {
            unreachable!()
        };
        let clips = clips_of(&[PartArea::Whole, top.clone()]);
        let (under, over) = (cut(&low, clips[0].region()), cut(&high, clips[1].region()));
        let joined = join(&[(&under, &clips[0]), (&over, &clips[1])], 1);
        checked += 1;
        let mask = top_clip.polygons().to_vec();
        // The cut and the cut of the cut differ by one grid unit along a slanted mask edge.
        let tolerance = if slanted {
            0.01 + 0.001 * mask_length(&mask)
        } else {
            0.01
        };
        let in_mask = grid::run(OverlayRule::Intersect, &rings(&joined.outlines), &mask);
        let want_in = grid::run(
            OverlayRule::Intersect,
            &rings(&cut(&high, Some(top_clip)).outlines),
            &mask,
        );
        let diff = grid::area_mm2(&grid::run(OverlayRule::Xor, &in_mask, &want_in)).abs();
        assert!(
            diff <= tolerance,
            "case {case}: inside, {diff} for {regions:?}"
        );
        let out_mask = grid::run(OverlayRule::Difference, &rings(&joined.outlines), &mask);
        let outside_low = cut(
            &low,
            Clip::of_regions(&regions, MaskMode::Outside, full_map(1000))
                .unwrap()
                .as_ref(),
        );
        let diff = grid::area_mm2(&grid::run(
            OverlayRule::Xor,
            &out_mask,
            &rings(&outside_low.outlines),
        ))
        .abs();
        assert!(
            diff <= tolerance,
            "case {case}: outside, {diff} for {regions:?}"
        );
    }
    assert!(checked > cases / 2);
}

#[test]
fn the_later_part_wins_inside_its_mask_and_the_earlier_one_outside_it() {
    order_checks(0x1656_67B1_9E37_79F9, CASES, regions, false);
}

/// The same checks with lassos in the masks: slanted seams, crossing sides.
const LASSO_CASES: usize = 600;

#[test]
fn the_join_of_inside_and_outside_holds_across_lasso_seams() {
    join_checks(0x2545_F491_4F6C_DD1D, LASSO_CASES, mixed_regions, true);
}

#[test]
fn the_later_part_wins_inside_a_lasso_mask_too() {
    order_checks(0x9E37_79B9_85EB_CA6B, LASSO_CASES, mixed_regions, true);
}
