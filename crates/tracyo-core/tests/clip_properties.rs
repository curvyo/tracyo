//! Property tests of the cut (ADR 0008 section 6): rectangles on a 6 px lattice, whose edges
//! coincide with the straight, pixel-aligned edges of P1's trace, give an Inside and an Outside
//! that fit, a clip that does not change when applied again, and the same bytes for any order of
//! the regions. A fixed-seed generator, 2,000 cases.

#![allow(clippy::unwrap_used)]

mod common;

use i_overlay::core::overlay_rule::OverlayRule;
use kurbo::{BezPath, Shape};
use tracyo_core::clip::{Clip, crossing_box};
use tracyo_core::grid;
use tracyo_core::regions::{MaskMode, PxPoint, PxRect, Region};
use vectrace::mask::Mask;

use common::{full_map, p1, request_at, trace};

const CASES: usize = 2_000;
/// Lattice of the rectangle corners, in px. P1's straight edges (x = 600, 900, y = 480, 350,
/// 650) are multiples of it.
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
        // From 60 px left of the picture to 60 px right of it, on the lattice.
        // invariant: below(n) < 200, fits an i32.
        i32::try_from(self.below(190)).unwrap() * LATTICE - 60
    }
}

fn regions(rng: &mut Rng) -> Vec<Region> {
    let added = 1 + rng.below(4);
    let taken = rng.below(3);
    let mut out = Vec::new();
    for i in 0..added + taken {
        let rect = PxRect::new(rng.corner(), rng.corner(), rng.corner(), rng.corner());
        out.push(Region::rect(rect, i >= added));
    }
    out
}

fn clip(regions: &[Region], mode: MaskMode) -> Option<Clip> {
    Clip::of_regions(regions, mode, full_map(1000)).unwrap()
}

fn rings(outlines: &[BezPath]) -> Vec<grid::Path64> {
    outlines
        .iter()
        .flat_map(|p| grid::flatten_snap(p).unwrap())
        .collect()
}

/// The area the whole trace has in the cut's own terms: an outline the clip does not cross keeps
/// its curves, one it crosses counts as the polygon it is flattened to.
fn reference_area(clip: &Clip, outlines: &[BezPath]) -> f64 {
    outlines
        .iter()
        .map(|o| {
            if clip.edge_meets(crossing_box(o)) {
                // The kurbo sign of a ring is that of its outline: scale by the outline's own.
                let ring = grid::area_mm2(&grid::flatten_snap(o).unwrap());
                ring.abs() * o.area().signum()
            } else {
                o.area()
            }
        })
        .sum()
}

fn net(outlines: &[BezPath]) -> f64 {
    outlines.iter().map(Shape::area).sum()
}

/// The length of all outlines in mm.
fn length(outlines: &[BezPath]) -> f64 {
    outlines.iter().map(|o| o.perimeter(0.001)).sum()
}

fn cut(clip: &Clip, outlines: &[BezPath]) -> Vec<BezPath> {
    clip.clip(outlines).outlines
}

/// Rectangles and lassos: a lasso has 3 to 6 corners on whole pixels anywhere from 60 px beyond the
/// picture's edges, so its sides are slanted, cross traced curves and cross each other.
fn mixed_regions(rng: &mut Rng) -> Vec<Region> {
    let added = 1 + rng.below(4);
    let taken = rng.below(3);
    let mut out = Vec::new();
    for i in 0..added + taken {
        let subtract = i >= added;
        if rng.below(3) == 0 {
            out.push(Region::rect(
                PxRect::new(rng.corner(), rng.corner(), rng.corner(), rng.corner()),
                subtract,
            ));
            continue;
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
        out.push(
            Region::lasso(&corners, subtract)
                .unwrap_or_else(|| Region::rect(PxRect::new(100, 100, 200, 200), subtract)),
        );
    }
    out
}

#[test]
fn inside_and_outside_fit_and_the_cut_is_stable_for_rectangles() {
    let whole = trace(&p1(), &request_at(70.0)).outlines;
    assert!(whole.len() >= 3);
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
    let mut crossings = 0;
    for case in 0..CASES {
        let regions = regions(&mut rng);
        let (Some(inside), Some(outside)) = (
            clip(&regions, MaskMode::Inside),
            clip(&regions, MaskMode::Outside),
        ) else {
            continue;
        };
        let a = cut(&inside, &whole);
        let b = cut(&outside, &whole);
        crossings += usize::from(whole.iter().any(|o| inside.edge_meets(crossing_box(o))));
        // Inside plus Outside is the whole, within 0.01 mm^2.
        let sum = net(&a) + net(&b);
        let want = reference_area(&inside, &whole);
        assert!(
            (sum - want).abs() <= 0.01,
            "case {case}: {sum} vs {want} for {regions:?}"
        );
        // They share no area.
        let overlap = grid::run(OverlayRule::Intersect, &rings(&a), &rings(&b));
        assert!(
            grid::area_mm2(&overlap).abs() <= 0.01,
            "case {case}: overlap for {regions:?}"
        );
        // Cutting again changes nothing.
        assert_eq!(
            format!("{:?}", cut(&inside, &a)),
            format!("{a:?}"),
            "case {case}: not idempotent, inside"
        );
        assert_eq!(
            format!("{:?}", cut(&outside, &b)),
            format!("{b:?}"),
            "case {case}: not idempotent, outside"
        );
        // Any order of the regions gives the same bytes.
        let mut shuffled = regions.clone();
        for i in (1..shuffled.len()).rev() {
            shuffled.swap(i, usize::try_from(rng.below(i as u64 + 1)).unwrap());
        }
        let again = clip(&shuffled, MaskMode::Inside).unwrap();
        assert_eq!(again, inside, "case {case}: order of {regions:?}");
        assert_eq!(
            format!("{:?}", cut(&again, &whole)),
            format!("{a:?}"),
            "case {case}"
        );
    }
    assert!(
        crossings > CASES / 4,
        "most cases cross the traced shapes ({crossings})"
    );
}

#[test]
fn inside_and_outside_fit_and_the_cut_is_stable_for_lassos() {
    let whole = trace(&p1(), &request_at(70.0)).outlines;
    assert!(whole.len() >= 3);
    let mut rng = Rng(0xC2B2_AE3D_27D4_EB4F);
    let mut crossings = 0;
    for case in 0..CASES {
        let regions = mixed_regions(&mut rng);
        let (Some(inside), Some(outside)) = (
            clip(&regions, MaskMode::Inside),
            clip(&regions, MaskMode::Outside),
        ) else {
            continue;
        };
        let a = cut(&inside, &whole);
        let b = cut(&outside, &whole);
        crossings += usize::from(whole.iter().any(|o| inside.edge_meets(crossing_box(o))));
        // Inside plus Outside is the whole, within 0.01 mm^2.
        let sum = net(&a) + net(&b);
        let want = reference_area(&inside, &whole);
        assert!(
            (sum - want).abs() <= 0.01,
            "case {case}: {sum} vs {want} for {regions:?}"
        );
        // They share no area.
        let overlap = grid::run(OverlayRule::Intersect, &rings(&a), &rings(&b));
        assert!(
            grid::area_mm2(&overlap).abs() <= 0.01,
            "case {case}: overlap for {regions:?}"
        );
        // Cutting again changes nothing but the vertices the first cut left within a grid unit of
        // a slanted side (the overlay drops them as collinear; a vertex 0.0005 mm off a 20 mm side
        // moves 0.005 mm^2): over three more cuts (a pinch a grid unit wide may split an outline or a sliver may go) each
        // cut moves the area by at most one grid unit (0.001 mm) times the outlines' length.
        for (clip, once) in [(&inside, &a), (&outside, &b)] {
            let mut last = once.clone();
            for _ in 0..3 {
                let next = cut(clip, &last);
                assert!(
                    (net(&next) - net(&last)).abs() <= 0.001 * length(&last),
                    "case {case}: area moved by a repeated cut of {regions:?}"
                );
                last = next;
            }
        }
        // Any order of the regions gives the same bytes.
        let mut shuffled = regions.clone();
        for i in (1..shuffled.len()).rev() {
            shuffled.swap(i, usize::try_from(rng.below(i as u64 + 1)).unwrap());
        }
        let again = clip(&shuffled, MaskMode::Inside).unwrap();
        assert_eq!(again, inside, "case {case}: order of {regions:?}");
        assert_eq!(
            format!("{:?}", cut(&again, &whole)),
            format!("{a:?}"),
            "case {case}"
        );
    }
    assert!(
        crossings > CASES / 4,
        "most cases cross the traced shapes ({crossings})"
    );
}

#[test]
fn edges_on_the_pixel_lines_of_the_trace_cut_exactly() {
    // x = 600 and y = 480 are straight edges of P1's trace (the square's left side, the bar's top);
    // an L of two rectangles and a frame have their edges there.
    let whole = trace(&p1(), &request_at(70.0)).outlines;
    let cases: [&[Region]; 3] = [
        &[
            Region::rect(PxRect::new(0, 0, 600, 1000), false),
            Region::rect(PxRect::new(600, 480, 1000, 1000), false),
        ],
        &[
            Region::rect(PxRect::new(0, 0, 1000, 1000), false),
            Region::rect(PxRect::new(600, 350, 900, 650), true),
        ],
        &[
            Region::rect(PxRect::new(100, 100, 900, 900), false),
            Region::rect(PxRect::new(600, 480, 900, 520), true),
        ],
    ];
    for regions in cases {
        let inside = clip(regions, MaskMode::Inside).unwrap();
        let outside = clip(regions, MaskMode::Outside).unwrap();
        let (a, b) = (cut(&inside, &whole), cut(&outside, &whole));
        let sum = net(&a) + net(&b);
        let want = reference_area(&inside, &whole);
        assert!(
            (sum - want).abs() <= 0.01,
            "{sum} vs {want} for {regions:?}"
        );
        let overlap = grid::run(OverlayRule::Intersect, &rings(&a), &rings(&b));
        assert!(grid::area_mm2(&overlap).abs() <= 0.0001, "{regions:?}");
    }
}

#[test]
fn the_same_input_gives_the_same_bytes() {
    let whole = trace(&p1(), &request_at(70.0)).outlines;
    let regions = [
        Region::rect(PxRect::new(30, 30, 640, 700), false),
        Region::rect(PxRect::new(300, 300, 400, 400), true),
    ];
    let first = format!(
        "{:?}",
        cut(&clip(&regions, MaskMode::Inside).unwrap(), &whole)
    );
    for _ in 0..3 {
        assert_eq!(
            first,
            format!(
                "{:?}",
                cut(&clip(&regions, MaskMode::Inside).unwrap(), &whole)
            )
        );
    }
}
