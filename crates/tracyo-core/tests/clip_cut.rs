//! Criteria 13 and 14 (one trace) of specification 0002: the trace is cut on the region's edge, and
//! Inside and Outside of one trace fit without gap or overlap.

#![allow(clippy::unwrap_used)]

mod common;

use kurbo::{PathEl, Shape};
use tracyo_core::clip::{Clip, MaskPlan, plan};
use tracyo_core::grid;
use tracyo_core::regions::{MaskMode, PxRect, Region};
use tracyo_core::svg;

use common::{full_map, p1, request_at, trace};

const SEAM_MM: f64 = 132.292;

fn r1() -> Region {
    Region::rect(PxRect::new(0, 0, 500, 1000), false)
}

fn clip_of(regions: &[Region], mode: MaskMode) -> Clip {
    Clip::of_regions(regions, mode, full_map(1000))
        .unwrap()
        .unwrap()
}

fn traced(mode: MaskMode) -> vectrace::wire::Output {
    let mut request = request_at(70.0);
    request.clip = Some(clip_of(&[r1()], mode));
    trace(&p1(), &request)
}

fn points(path: &kurbo::BezPath) -> Vec<kurbo::Point> {
    path.elements()
        .iter()
        .filter_map(|e| match e {
            PathEl::MoveTo(p) | PathEl::LineTo(p) | PathEl::CurveTo(_, _, p) => Some(*p),
            _ => None,
        })
        .collect()
}

#[test]
fn criterion_13_the_bar_is_cut_at_the_region_edge() {
    let out = traced(MaskMode::Inside);
    let all: Vec<kurbo::Point> = out.outlines.iter().flat_map(points).collect();
    let on_seam: Vec<_> = all.iter().filter(|p| (p.x - SEAM_MM).abs() < 1.0).collect();
    println!("seam vertices: {on_seam:?}");
    assert_eq!(on_seam.len(), 2, "the bar leaves the region at two corners");
    for p in &on_seam {
        assert!((p.x - SEAM_MM).abs() <= 0.005, "x = {}", p.x);
    }
    let mut ys: Vec<f64> = on_seam.iter().map(|p| p.y).collect();
    ys.sort_by(f64::total_cmp);
    assert!(
        (ys[0] - 127.000).abs() <= 0.005 && (ys[1] - 137.583).abs() <= 0.005,
        "ys = {ys:?}"
    );
    assert!(
        all.iter().all(|p| p.x <= SEAM_MM + 0.005),
        "no shape lies outside the mask"
    );
}

#[test]
fn criterion_13_a_cut_shape_has_only_lines_and_the_small_disc_keeps_its_curves() {
    let out = traced(MaskMode::Inside);
    let (mut cut, mut curved) = (0, 0);
    for outline in &out.outlines {
        let has_curve = outline
            .elements()
            .iter()
            .any(|e| matches!(e, PathEl::CurveTo(..)));
        let touches_seam = points(outline)
            .iter()
            .any(|p| (p.x - SEAM_MM).abs() <= 0.005);
        if touches_seam {
            cut += 1;
            assert!(!has_curve, "a cut shape holds only M, L and Z");
        } else if has_curve {
            curved += 1;
        }
    }
    assert_eq!(cut, 1);
    assert!(curved >= 1, "the small disc inside keeps its curves");
}

#[test]
fn criterion_13_an_l_and_a_frame_are_cut_along_their_edges() {
    let l = [
        Region::rect(PxRect::new(0, 0, 500, 1000), false),
        Region::rect(PxRect::new(500, 600, 1000, 1000), false),
    ];
    let frame = [
        Region::rect(PxRect::new(60, 60, 960, 960), false),
        Region::rect(PxRect::new(120, 120, 900, 900), true),
    ];
    for (regions, label) in [(&l[..], "L"), (&frame[..], "frame")] {
        let clip = clip_of(regions, MaskMode::Inside);
        let mut request = request_at(70.0);
        request.clip = Some(clip.clone());
        let out = trace(&p1(), &request);
        let edges: Vec<f64> = clip
            .polygons()
            .iter()
            .flatten()
            .flat_map(|p| [grid::to_mm(p.x), grid::to_mm(p.y)])
            .collect();
        // Every vertex of the result lies on an edge of the mask or on the traced shape inside it:
        // the ones that are cut run along an edge within 0.005 mm.
        let mut on_edge = 0;
        for p in out.outlines.iter().flat_map(points) {
            if edges
                .chunks(2)
                .any(|e| (p.x - e[0]).abs() <= 0.005 || (p.y - e[1]).abs() <= 0.005)
            {
                on_edge += 1;
            }
        }
        assert!(on_edge >= 4, "{label}: {on_edge} vertices on the cut");
        assert!(out.paths > 0, "{label}");
    }
}

#[test]
fn criterion_14_inside_and_outside_of_one_trace_fit_without_gap_or_overlap() {
    let whole = trace(&p1(), &request_at(70.0));
    let inside = traced(MaskMode::Inside);
    let outside = traced(MaskMode::Outside);
    let area = |o: &vectrace::wire::Output| common::net_area(&o.outlines).abs();
    let sum = area(&inside) + area(&outside);
    let all = area(&whole);
    println!(
        "inside {} + outside {} = {sum}, whole {all}",
        area(&inside),
        area(&outside)
    );
    assert!(
        (sum - all).abs() <= 0.001 * all,
        "within 0.1 % of the whole trace"
    );
    let rings = |o: &vectrace::wire::Output| -> Vec<grid::Path64> {
        o.outlines
            .iter()
            .flat_map(|p| grid::flatten_snap(p).unwrap())
            .collect()
    };
    let overlap = grid::run(
        i_overlay::core::overlay_rule::OverlayRule::Intersect,
        &rings(&inside),
        &rings(&outside),
    );
    assert!(grid::area_mm2(&overlap).abs() <= 0.01);
}

#[test]
fn a_mask_off_the_picture_is_a_notice_not_a_trace() {
    let off = [Region::rect(PxRect::new(2000, 0, 2100, 100), false)];
    let p = |mode| plan(&off, mode, full_map(1000), 1000, 1000);
    assert_eq!(
        p(MaskMode::Inside),
        MaskPlan::Nothing(tracyo_core::clip::OUTSIDE_IMAGE)
    );
    assert_eq!(
        p(MaskMode::Outside),
        MaskPlan::Whole,
        "outside of a mask off the picture is everything"
    );
    assert_eq!(
        plan(&[], MaskMode::Inside, full_map(1000), 1000, 1000),
        MaskPlan::Whole
    );
    let subtracted = [Region::rect(PxRect::new(0, 0, 100, 100), true)];
    assert_eq!(
        plan(&subtracted, MaskMode::Inside, full_map(1000), 1000, 1000),
        MaskPlan::Nothing(tracyo_core::clip::OUTSIDE_IMAGE)
    );
    let all = [
        Region::rect(PxRect::new(0, 0, 100, 100), false),
        Region::rect(PxRect::new(-5, -5, 200, 200), true),
    ];
    assert_eq!(
        plan(&all, MaskMode::Inside, full_map(1000), 1000, 1000),
        MaskPlan::Nothing(tracyo_core::clip::OUTSIDE_IMAGE)
    );
}

#[test]
fn the_saved_svg_of_r1_inside_matches_its_golden_file() {
    let out = traced(MaskMode::Inside);
    let text = svg::write(&svg::Document {
        width_px: 1000,
        height_px: 1000,
        layers: &svg::layers_of(&out),
    });
    let golden = format!(
        "{}/tests/fixtures/svg/parts-13.svg",
        env!("CARGO_MANIFEST_DIR")
    );
    if std::env::var_os("TRACYO_UPDATE_GOLDEN").is_some() {
        std::fs::write(&golden, &text).unwrap();
    }
    assert_eq!(text, std::fs::read_to_string(golden).unwrap());
    let doc = roxmltree::Document::parse(&text).unwrap();
    let paths: Vec<_> = doc
        .descendants()
        .filter(|n| n.has_tag_name("path"))
        .collect();
    assert_eq!(paths.len(), 1, "one layer, one path");
    let area: f64 = out.outlines.iter().map(Shape::area).sum::<f64>().abs();
    assert!(area > 1000.0);
}

/// Criterion 28 of specification 0002: P1 and the lasso L1 = (0, 0), (600, 0), (0, 1000) px, Inside:
/// every vertex of a shape cut by the slanted side lies within 0.005 mm of it and no shape lies
/// outside L1.
#[test]
fn criterion_28_a_slanted_lasso_side_cuts_within_five_microns() {
    use tracyo_core::regions::PxPoint;
    let l1 = Region::lasso(
        &[
            PxPoint::new(0, 0),
            PxPoint::new(600, 0),
            PxPoint::new(0, 1000),
        ],
        false,
    )
    .unwrap();
    let clip = clip_of(&[l1], MaskMode::Inside);
    let mut request = request_at(70.0);
    request.clip = Some(clip);
    let out = trace(&p1(), &request);
    let s = 25.4 / 96.0;
    // The signed distance in mm of (x, y) mm from the slanted side, negative inside.
    let distance =
        |p: kurbo::Point| (1000.0 * p.x + 600.0 * p.y - 600_000.0 * s) / 1000.0_f64.hypot(600.0);
    let all: Vec<kurbo::Point> = out.outlines.iter().flat_map(points).collect();
    let on_side: Vec<f64> = all
        .iter()
        .map(|p| distance(*p))
        .filter(|d| d.abs() < 0.05)
        .collect();
    println!(
        "{} vertices near the slanted side, worst {:?}",
        on_side.len(),
        on_side.iter().map(|d| d.abs()).fold(0.0, f64::max)
    );
    assert_eq!(
        on_side.len(),
        2,
        "the side enters and leaves the big disc once each"
    );
    assert!(on_side.iter().all(|d| d.abs() <= 0.005), "{on_side:?}");
    assert!(
        all.iter()
            .all(|p| distance(*p) <= 0.005 && p.x >= -0.005 && p.y >= -0.005),
        "no shape lies outside L1"
    );
    // The cut shapes hold only lines.
    for outline in &out.outlines {
        let near = points(outline).iter().any(|p| distance(*p).abs() <= 0.005);
        if near {
            assert!(
                !outline
                    .elements()
                    .iter()
                    .any(|e| matches!(e, PathEl::CurveTo(..))),
                "a cut shape holds only M, L and Z"
            );
        }
    }
}

/// Inside and Outside of a lasso with a slanted side fit without gap or overlap, as criterion 14.
#[test]
fn criterion_14_holds_across_a_slanted_lasso_side() {
    use tracyo_core::regions::PxPoint;
    let l1 = Region::lasso(
        &[
            PxPoint::new(0, 0),
            PxPoint::new(600, 0),
            PxPoint::new(0, 1000),
        ],
        false,
    )
    .unwrap();
    let whole = trace(&p1(), &request_at(70.0));
    let traced_with = |mode| {
        let mut request = request_at(70.0);
        request.clip = Some(clip_of(std::slice::from_ref(&l1), mode));
        trace(&p1(), &request)
    };
    let area = |o: &vectrace::wire::Output| common::net_area(&o.outlines).abs();
    let (a, b) = (
        traced_with(MaskMode::Inside),
        traced_with(MaskMode::Outside),
    );
    let (sum, all) = (area(&a) + area(&b), area(&whole));
    println!(
        "inside {} + outside {} = {sum}, whole {all}",
        area(&a),
        area(&b)
    );
    assert!((sum - all).abs() <= 0.001 * all);
}
