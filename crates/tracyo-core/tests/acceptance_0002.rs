//! Independent acceptance checks of specification 0002 (criteria 13, 15, 22), black box: they read
//! only the SVG text the joined result is written as, and the public entry points the worker uses.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::cast_precision_loss)]

mod common;

use common::{p1, request_at};
use tracyo_core::clip::Clip;
use tracyo_core::job::Session;
use tracyo_core::picture::Picture;
use tracyo_core::regions::{MaskMode, PxRect, Region};
use tracyo_core::request::{TraceRequest, size_mm};
use tracyo_core::stack::{PartArea, part_clips, parts_job};
use tracyo_core::svg;
use vectrace::PixelMap;
use vectrace::wire;

const PX_MM: f64 = 264.583 / 1000.0;
const SEAM_MM: f64 = 132.292;
const TOL_MM: f64 = 0.005;

/// One `<path>` of the SVG text: its fill and its `d` attribute.
struct Shape {
    fill: String,
    d: String,
}

/// The `<path>` elements of an SVG text, in document order.
fn shapes(svg_text: &str) -> Vec<Shape> {
    let mut out = Vec::new();
    let mut rest = svg_text;
    while let Some(start) = rest.find("<path ") {
        let tail = &rest[start..];
        let end = tail.find("/>").expect("path element closes");
        let element = &tail[..end];
        let attr = |name: &str| -> String {
            let key = format!(" {name}=\"");
            let at = element.find(&key).expect("attribute present") + key.len();
            let len = element[at..].find('"').unwrap();
            element[at..at + len].to_string()
        };
        out.push(Shape {
            fill: attr("fill"),
            d: attr("d"),
        });
        rest = &tail[end..];
    }
    out
}

/// The vertices (mm) of a path's `d`, and the command letters it uses.
fn vertices(d: &str) -> (Vec<(f64, f64)>, String) {
    let mut letters = String::new();
    let mut nums: Vec<f64> = Vec::new();
    let mut points = Vec::new();
    let mut cmd = ' ';
    let flush = |cmd: char, nums: &mut Vec<f64>, points: &mut Vec<(f64, f64)>| {
        match cmd {
            'M' | 'L' => {
                for pair in nums.chunks(2) {
                    points.push((pair[0], pair[1]));
                }
            }
            'C' => {
                for pair in nums.chunks(2).skip(2).step_by(3) {
                    points.push((pair[0], pair[1]));
                }
            }
            _ => {}
        }
        nums.clear();
    };
    for token in d.split_whitespace() {
        if let Ok(v) = token.parse::<f64>() {
            nums.push(v);
        } else {
            flush(cmd, &mut nums, &mut points);
            cmd = token.chars().next().unwrap();
            letters.push(cmd);
        }
    }
    flush(cmd, &mut nums, &mut points);
    (points, letters)
}

fn r1() -> Region {
    Region::rect(PxRect::new(0, 0, 500, 1000), false)
}

fn full_map() -> PixelMap {
    tracyo_core::request::pixel_map(1000, 1000)
}

fn mask(mode: MaskMode) -> PartArea {
    PartArea::Mask(
        Clip::of_regions(&[r1()], mode, full_map())
            .unwrap()
            .unwrap(),
    )
}

fn request(percent: f64, smooth: u8) -> TraceRequest {
    let mut r = request_at(percent);
    r.settings.smooth = smooth;
    r
}

/// The joined SVG text of `parts` (area and request, bottom first) on `picture`.
fn joined_svg(picture: &Picture, parts: Vec<(PartArea, TraceRequest)>) -> String {
    let (areas, requests): (Vec<_>, Vec<_>) = parts.into_iter().unzip();
    let side = size_mm(picture.width);
    let clips = part_clips(&areas, kurbo::Rect::new(0.0, 0.0, side, side));
    let job = parts_job(&clips, requests, 0).unwrap();
    let mut session = Session::new(picture.clone());
    let numbers = session.run_parts(&job, &mut |_, _| {}, &mut |_| {});
    let out = wire::read(&numbers).unwrap();
    svg::write(&svg::Document {
        width_px: 1000,
        height_px: 1000,
        layers: &svg::layers_of(&out),
    })
}

/// Criterion 13: the cut at x = 132.292 mm, straight edges only on the cut shapes, curves kept on
/// the small disc, nothing outside the mask.
#[test]
fn criterion_13_cut_is_exact_and_only_lines_on_the_cut_shapes() {
    let text = joined_svg(&p1(), vec![(mask(MaskMode::Inside), request(70.0, 50))]);
    let all = shapes(&text);
    assert!(!all.is_empty(), "the Inside part writes shapes");

    let mut max_x = f64::MIN;
    let mut seam_points = Vec::new();
    let mut lines_only = 0;
    let mut small_disc_has_curves = false;
    let outlines: Vec<String> = all
        .iter()
        .flat_map(|s| {
            s.d.split('M')
                .skip(1)
                .map(|o| format!("M{o}"))
                .collect::<Vec<_>>()
        })
        .collect();
    for outline in &outlines {
        let (points, letters) = vertices(outline);
        for (x, y) in &points {
            max_x = max_x.max(*x);
            if (x - SEAM_MM).abs() <= TOL_MM {
                seam_points.push((*x, *y));
            }
        }
        if letters.chars().all(|c| matches!(c, 'M' | 'L' | 'Z')) {
            lines_only += 1;
        }
        // The small disc: centre (150, 150) px, radius 50 px.
        // The small disc: centre (150, 150) px = 39.688 mm, radius 50 px = 13.229 mm.
        let c = 150.0 * PX_MM;
        let inside_disc = !points.is_empty()
            && points
                .iter()
                .all(|(x, y)| ((x - c).powi(2) + (y - c).powi(2)).sqrt() < 13.229 + 0.5);
        if inside_disc && letters.contains('C') {
            small_disc_has_curves = true;
        }
    }
    assert!(
        max_x <= SEAM_MM + TOL_MM,
        "no vertex lies right of the seam, found x = {max_x}"
    );
    assert!(lines_only >= 1, "the cut shapes are M, L and Z only");
    assert!(
        small_disc_has_curves,
        "the wholly inside disc keeps its C commands"
    );
    let corner_top = (SEAM_MM, 127.000);
    let corner_bottom = (SEAM_MM, 137.583);
    for corner in [corner_top, corner_bottom] {
        assert!(
            seam_points
                .iter()
                .any(|p| (p.0 - corner.0).abs() <= TOL_MM && (p.1 - corner.1).abs() <= TOL_MM),
            "bar corner {corner:?} lies on the seam within 0.005 mm"
        );
    }
}

/// Criterion 15: the join of a whole-image part and an Inside part is one layer `#000000` with one
/// path of two outlines (M/Z twice), and no seam line remains at x = 132.292 mm.
#[test]
fn criterion_15_join_is_one_path_with_two_outlines_and_no_seam() {
    let text = joined_svg(
        &p1(),
        vec![
            (PartArea::Whole, request(40.0, 20)),
            (mask(MaskMode::Inside), request(70.0, 80)),
        ],
    );
    assert_eq!(text.matches("inkscape:groupmode=\"layer\"").count(), 1);
    assert!(text.contains("inkscape:label=\"#000000\""));
    let all = shapes(&text);
    assert_eq!(all.len(), 1, "exactly one <path>, found {}", all.len());
    let d = &all[0].d;
    assert_eq!(all[0].fill, "#000000");
    assert_eq!(d.matches('M').count(), 2, "two outlines");
    assert_eq!(d.matches('Z').count(), 2, "two closed outlines");
    // No segment runs along the seam for more than 0.01 mm.
    let (points, _) = vertices(d);
    for pair in points.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        let along = (a.0 - SEAM_MM).abs() <= TOL_MM && (b.0 - SEAM_MM).abs() <= TOL_MM;
        assert!(
            !along || (a.1 - b.1).abs() <= 0.01,
            "a seam segment of {} mm lies on x = seam",
            (a.1 - b.1).abs()
        );
    }
    // The grey square (#808080) is not in the result at Threshold 40.
    assert!(!text.contains("#808080"));
}

/// Criterion 22: the same picture, parts and settings give byte-identical SVG on repeated runs.
#[test]
fn criterion_22_repeated_runs_are_byte_identical() {
    let parts = || {
        vec![
            (PartArea::Whole, request(40.0, 20)),
            (mask(MaskMode::Inside), request(70.0, 80)),
        ]
    };
    let first = joined_svg(&p1(), parts());
    for _ in 0..2 {
        assert_eq!(first, joined_svg(&p1(), parts()));
    }
}

/// Criterion 1 (Terms): a part with only subtracted regions has an empty mask.
#[test]
fn terms_subtracted_only_mask_is_empty() {
    let sub = Region::rect(PxRect::new(0, 0, 500, 1000), true);
    let clip = Clip::of_regions(&[sub], MaskMode::Inside, full_map()).unwrap();
    assert!(clip.is_none(), "a subtracted-only mask has no area");
}
