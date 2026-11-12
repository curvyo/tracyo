//! Criteria 14, 15, 16 and 22 of specification 0002 on traced parts, run through the worker's own
//! entry point (`Session::run_parts`): the join of Inside and Outside, one outline across a seam,
//! the later part wins, byte-identical results. Golden: `parts-15.svg`.

#![allow(clippy::unwrap_used)]

mod common;

use i_overlay::core::overlay_rule::OverlayRule;
use kurbo::{BezPath, PathEl, Shape};
use tracyo_core::clip::Clip;
use tracyo_core::grid;
use tracyo_core::job::Session;
use tracyo_core::picture::Picture;
use tracyo_core::regions::{MaskMode, PxRect, Region};
use tracyo_core::request::{TraceRequest, size_mm};
use tracyo_core::stack::{PartArea, part_clips, parts_job};
use tracyo_core::svg;
use vectrace::wire::{self, Output};

use common::{full_map, p1, request_at};

const SEAM_MM: f64 = 132.292;

fn r1() -> Region {
    Region::rect(PxRect::new(0, 0, 500, 1000), false)
}

fn mask(regions: &[Region], mode: MaskMode) -> PartArea {
    PartArea::Mask(
        Clip::of_regions(regions, mode, full_map(1000))
            .unwrap()
            .unwrap(),
    )
}

/// A request at Threshold `percent` with Smooth `smooth`.
fn request(percent: f64, smooth: u8) -> TraceRequest {
    let mut r = request_at(percent);
    r.settings.smooth = smooth;
    r
}

/// The joined result of `parts` (area and request, bottom first) on `picture`, with the part
/// events seen on the way.
fn run(picture: &Picture, parts: Vec<(PartArea, TraceRequest)>) -> (Output, Vec<u8>) {
    let (areas, requests): (Vec<_>, Vec<_>) = parts.into_iter().unzip();
    let side = size_mm(picture.width);
    let clips = part_clips(&areas, kurbo::Rect::new(0.0, 0.0, side, side));
    let job = parts_job(&clips, requests, 0).unwrap();
    let mut session = Session::new(picture.clone());
    let mut seen = Vec::new();
    let numbers = session.run_parts(&job, &mut |p, _| seen.push(p), &mut |_| {});
    (wire::read(&numbers).unwrap(), seen)
}

fn svg_of(out: &Output) -> String {
    svg::write(&svg::Document {
        width_px: 1000,
        height_px: 1000,
        layers: &svg::layers_of(out),
    })
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

/// The longest stretch along which two segments of `outlines` lie on one axis-parallel line.
fn longest_shared_run(outlines: &[BezPath]) -> f64 {
    let mut vertical: Vec<(i64, f64, f64)> = Vec::new();
    let mut horizontal: Vec<(i64, f64, f64)> = Vec::new();
    for outline in outlines {
        let pts = points(outline);
        for k in 0..pts.len() {
            let (a, b) = (pts[k], pts[(k + 1) % pts.len()]);
            #[allow(clippy::cast_possible_truncation)]
            let key = |v: f64| (v * 100.0).round() as i64; // 0.01 mm
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
                let overlap = set[i].2.min(set[j].2) - set[j].1.max(set[i].1);
                longest = longest.max(overlap);
            }
        }
    }
    longest
}

fn rings(outlines: &[BezPath]) -> Vec<grid::Path64> {
    outlines
        .iter()
        .flat_map(|p| grid::flatten_snap(p).unwrap())
        .collect()
}

fn area(out: &Output) -> f64 {
    out.outlines.iter().map(Shape::area).sum::<f64>().abs()
}

#[test]
fn criterion_14_inside_and_outside_parts_fit_and_equal_the_whole_trace() {
    let same = || request(70.0, 50);
    let a = run(&p1(), vec![(mask(&[r1()], MaskMode::Outside), same())]).0;
    let b = run(&p1(), vec![(mask(&[r1()], MaskMode::Inside), same())]).0;
    let joined = run(
        &p1(),
        vec![
            (mask(&[r1()], MaskMode::Outside), same()),
            (mask(&[r1()], MaskMode::Inside), same()),
        ],
    )
    .0;
    let whole = run(&p1(), vec![(PartArea::Whole, same())]).0;
    println!(
        "A {} + B {} = {}, joined {}, whole {}",
        area(&a),
        area(&b),
        area(&a) + area(&b),
        area(&joined),
        area(&whole)
    );
    assert!((area(&joined) - (area(&a) + area(&b))).abs() <= 0.01);
    let overlap = grid::run(
        OverlayRule::Intersect,
        &rings(&a.outlines),
        &rings(&b.outlines),
    );
    assert!(grid::area_mm2(&overlap).abs() <= 0.01);
    let xor = grid::run(
        OverlayRule::Xor,
        &rings(&joined.outlines),
        &rings(&whole.outlines),
    );
    assert!(
        grid::area_mm2(&xor).abs() <= 0.001 * area(&whole),
        "within 0.1 % of the whole trace"
    );
    assert!(longest_shared_run(&joined.outlines) <= 0.01, "no seam line");
}

fn criterion_15() -> Output {
    run(
        &p1(),
        vec![
            (PartArea::Whole, request(40.0, 20)),
            (mask(&[r1()], MaskMode::Inside), request(70.0, 80)),
        ],
    )
    .0
}

#[test]
fn criterion_15_the_parts_join_into_one_outline_without_a_seam_line() {
    let out = criterion_15();
    let text = svg_of(&out);
    let doc = roxmltree::Document::parse(&text).unwrap();
    let layers: Vec<_> = doc
        .descendants()
        .filter(|n| {
            n.attribute(("http://www.inkscape.org/namespaces/inkscape", "groupmode"))
                == Some("layer")
        })
        .collect();
    assert_eq!(layers.len(), 1);
    assert_eq!(
        layers[0].attribute(("http://www.inkscape.org/namespaces/inkscape", "label")),
        Some("#000000")
    );
    let paths: Vec<_> = doc
        .descendants()
        .filter(|n| n.has_tag_name("path"))
        .collect();
    assert_eq!(paths.len(), 1, "one layer, one path");
    let d = paths[0].attribute("d").unwrap();
    let count = |c: char| d.chars().filter(|&x| x == c).count();
    assert_eq!((count('M'), count('Z')), (2, 2), "{d}");
    assert!(longest_shared_run(&out.outlines) <= 0.01, "no seam line");
    // No segment of the result runs along the seam at all.
    let along: f64 = out
        .outlines
        .iter()
        .map(|o| {
            let pts = points(o);
            (0..pts.len())
                .map(|k| (pts[k], pts[(k + 1) % pts.len()]))
                .filter(|(a, b)| (a.x - SEAM_MM).abs() <= 0.005 && (b.x - SEAM_MM).abs() <= 0.005)
                .map(|(a, b)| (a.y - b.y).abs())
                .sum::<f64>()
        })
        .sum();
    assert!(along <= 0.01, "{along} mm of segments along the seam");
    // The grey square (#808080, x 225 to 251 mm, y 13 to 40 mm) is not traced at Threshold 40.
    assert!(
        !out.outlines.iter().any(|o| {
            let b = o.bounding_box();
            b.x0 > 224.0 && b.y1 < 41.0
        }),
        "no grey square"
    );
}

#[test]
fn the_joined_svg_of_criterion_15_matches_its_golden_file() {
    let text = svg_of(&criterion_15());
    let golden = format!(
        "{}/tests/fixtures/svg/parts-15.svg",
        env!("CARGO_MANIFEST_DIR")
    );
    if std::env::var_os("TRACYO_UPDATE_GOLDEN").is_some() {
        std::fs::write(&golden, &text).unwrap();
    }
    assert_eq!(text, std::fs::read_to_string(golden).unwrap());
}

/// P1 with the grey square moved into R1 (centre (100, 800) px).
fn p1_grey_in_r1() -> Picture {
    let mut picture = p1();
    for y in 50..150 {
        for x in 850..950 {
            let i = (y * 1000 + x) * 4;
            picture.rgba[i..i + 3].copy_from_slice(&[255, 255, 255]);
        }
    }
    for y in 750..850 {
        for x in 50..150 {
            let i = (y * 1000 + x) * 4;
            picture.rgba[i..i + 3].copy_from_slice(&[0x80, 0x80, 0x80]);
        }
    }
    picture
}

fn has_grey(out: &Output) -> bool {
    // The grey square's box in mm: x 13.2 to 39.7, y 198.4 to 224.9.
    out.outlines.iter().any(|o| {
        let b = o.bounding_box();
        b.x0 > 12.0 && b.x1 < 41.0 && b.y0 > 197.0 && b.y1 < 226.0
    })
}

#[test]
fn criterion_16_the_later_part_wins_in_both_orders() {
    let picture = p1_grey_in_r1();
    let (whole40, inside70) = (request(40.0, 50), request(70.0, 50));
    let (out, _) = run(
        &picture,
        vec![
            (PartArea::Whole, whole40.clone()),
            (mask(&[r1()], MaskMode::Inside), inside70.clone()),
        ],
    );
    assert!(has_grey(&out), "the grey square is in the result");
    let (swapped, _) = run(
        &picture,
        vec![
            (mask(&[r1()], MaskMode::Inside), inside70.clone()),
            (PartArea::Whole, whole40.clone()),
        ],
    );
    assert!(!has_grey(&swapped), "the later whole-image part wins in R1");
    // The result inside R1 is the Threshold 70 trace clipped to R1, outside it the Threshold 40
    // trace without R1 (0.1 % of the area).
    let clipped = |percent: f64, mode: MaskMode| {
        run(&picture, vec![(mask(&[r1()], mode), request(percent, 50))]).0
    };
    let in70 = clipped(70.0, MaskMode::Inside);
    let out40 = clipped(40.0, MaskMode::Outside);
    // Inside R1 of the result: intersect the result with R1's polygon.
    let r1_poly = match mask(&[r1()], MaskMode::Inside) {
        PartArea::Mask(c) => c.polygons().to_vec(),
        _ => unreachable!(),
    };
    let result_in_r1 = grid::run(OverlayRule::Intersect, &rings(&out.outlines), &r1_poly);
    let want_in_r1 = rings(&in70.outlines);
    let diff = grid::area_mm2(&grid::run(OverlayRule::Xor, &result_in_r1, &want_in_r1)).abs();
    assert!(diff <= 0.001 * area(&in70), "inside R1: {diff}");
    let result_out = grid::run(OverlayRule::Difference, &rings(&out.outlines), &r1_poly);
    let want_out = rings(&out40.outlines);
    let diff = grid::area_mm2(&grid::run(OverlayRule::Xor, &result_out, &want_out)).abs();
    assert!(diff <= 0.001 * area(&out40), "outside R1: {diff}");
}

#[test]
fn criterion_22_the_same_parts_give_the_same_bytes() {
    let first = svg_of(&criterion_15());
    for _ in 0..2 {
        assert_eq!(svg_of(&criterion_15()), first);
    }
}

#[test]
fn a_part_is_reported_once_it_is_done_and_a_cached_part_is_not_traced() {
    let areas = [PartArea::Whole, mask(&[r1()], MaskMode::Inside)];
    let side = size_mm(1000);
    let clips = part_clips(&areas, kurbo::Rect::new(0.0, 0.0, side, side));
    let mut job = parts_job(&clips, vec![request(40.0, 20), request(70.0, 80)], 1).unwrap();
    assert_eq!(job.palette_of, 1);
    let mut session = Session::new(p1());
    let mut parts = Vec::new();
    let mut percents = Vec::new();
    let first = session.run_parts(&job, &mut |p, w| parts.push((p, w.to_vec())), &mut |p| {
        percents.push(p);
    });
    assert_eq!(parts.iter().map(|p| p.0).collect::<Vec<_>>(), [0, 1]);
    assert!(percents.windows(2).all(|w| w[0] < w[1]), "{percents:?}");
    assert_eq!(percents.last(), Some(&100));
    job.jobs[0].cached = Some(parts[0].1.clone());
    let mut again = Vec::new();
    let second = session.run_parts(&job, &mut |p, _| again.push(p), &mut |_| {});
    assert_eq!(again, [1], "part 1 came from the cache");
    assert_eq!(first, second, "the same numbers either way");
}

#[test]
fn the_join_survives_the_settings_a_part_may_not_crop_for() {
    // Denoise, Drop border shapes and Upscale Auto trace from the whole picture.
    let mut denoise = request(70.0, 50);
    denoise.settings.denoise = 1;
    let (out, _) = run(
        &p1(),
        vec![
            (PartArea::Whole, request(40.0, 20)),
            (mask(&[r1()], MaskMode::Inside), denoise),
        ],
    );
    assert!(out.paths >= 1);
}

/// Criterion 15, the automatic substitute for a reader of its own: a second reader draws the
/// joined SVG with the even-odd rule, so a gap or an overlap at the seam would show as a light
/// line inside the bar; the whole bar across the seam is covered.
#[test]
fn a_second_reader_draws_the_bar_across_the_seam_without_gap_or_overlap() {
    let text = svg_of(&criterion_15());
    let scale = 8.0_f32;
    let tree = resvg::usvg::Tree::from_str(&text, &resvg::usvg::Options::default()).unwrap();
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let mut pixmap = resvg::tiny_skia::Pixmap::new(
        (tree.size().width() * scale).ceil() as u32,
        (tree.size().height() * scale).ceil() as u32,
    )
    .unwrap();
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    // The bar is y = 127.000 to 137.583 mm; 6 mm either side of the seam at x = 132.292 mm, one
    // pixel inside its edges.
    let mut light = 0;
    let mut checked = 0;
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    for y in ((127.1 * f64::from(scale)) as u32)..((137.4 * f64::from(scale)) as u32) {
        for x in (((SEAM_MM - 6.0) * f64::from(scale)) as u32)
            ..(((SEAM_MM + 6.0) * f64::from(scale)) as u32)
        {
            checked += 1;
            if pixmap.pixel(x, y).unwrap().alpha() < 250 {
                light += 1;
            }
        }
    }
    println!("{light} of {checked} pixels of the bar across the seam are not covered");
    assert_eq!(light, 0);
}
