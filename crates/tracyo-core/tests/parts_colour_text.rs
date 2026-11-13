//! Criteria 17 and 19 of specification 0002 on traced parts, through the worker's own entry point
//! (`Session::run_parts`): one layer per colour over all parts, text per part by its clip, and
//! shapes recognised after the cut. Golden: `parts-17.svg`.
//!
//! P3 is the poster `precision.png` of Curvyo's 0091 reference set (CC0, `assets/ASSETS.md`).

#![allow(clippy::unwrap_used)]

mod common;

use std::path::Path;

use i_overlay::core::overlay_rule::OverlayRule;
use kurbo::{BezPath, Shape};
use tracyo_core::clip::Clip;
use tracyo_core::grid;
use tracyo_core::job::Session;
use tracyo_core::palette::ColourSpec;
use tracyo_core::picture::Picture;
use tracyo_core::regions::{MaskMode, PxRect, Region};
use tracyo_core::request::{TraceRequest, size_mm};
use tracyo_core::stack::{PartArea, part_clips, parts_job};
use tracyo_core::svg;
use vectrace::Rgb8;
use vectrace::ocr::{OcrLine, OcrModel};
use vectrace::wire::{self, Output};

use common::{full_map, p1, request_at};

fn mask(width: u32, rect: PxRect, mode: MaskMode) -> PartArea {
    PartArea::Mask(
        Clip::of_regions(&[Region::rect(rect, false)], mode, full_map(width))
            .unwrap()
            .unwrap(),
    )
}

/// The joined result of `parts` (area and request, bottom first) on `picture`.
fn run(picture: &Picture, parts: Vec<(PartArea, TraceRequest)>) -> Output {
    let (areas, requests): (Vec<_>, Vec<_>) = parts.into_iter().unzip();
    let clips = part_clips(
        &areas,
        kurbo::Rect::new(
            0.0,
            0.0,
            size_mm(picture.width),
            size_mm(picture.width) * f64::from(picture.height) / f64::from(picture.width),
        ),
    );
    let job = parts_job(&clips, requests, 0).unwrap();
    let mut session = Session::new(picture.clone());
    wire::read(&session.run_parts(&job, &mut |_, _| {}, &mut |_| {})).unwrap()
}

fn svg_of(picture: &Picture, out: &Output) -> String {
    svg::write(&svg::Document {
        width_px: picture.width,
        height_px: picture.height,
        layers: &svg::layers_of(out),
    })
}

fn rings(outlines: &[BezPath]) -> Vec<grid::Path64> {
    outlines
        .iter()
        .flat_map(|p| grid::flatten_snap(p).unwrap())
        .collect()
}

// ---- criterion 17: Multicolour ----

const RED: [u8; 3] = [0xC8, 0x10, 0x2E];
const BLUE: [u8; 3] = [0x2B, 0x4C, 0x9B];
const GREEN: [u8; 3] = [0x1E, 0x78, 0x3C];

/// P1 in colours: the left disc and the bar red, the square blue, the small disc green; the grey
/// square stays.
fn p1_colours() -> Picture {
    let mut picture = p1();
    for y in 0..1000_usize {
        for x in 0..1000_usize {
            let i = (y * 1000 + x) * 4;
            if picture.rgba[i..i + 3] != [0, 0, 0] {
                continue;
            }
            let colour = if y < 300 {
                GREEN
            } else if (600..900).contains(&x) && !(480..520).contains(&y) {
                BLUE
            } else {
                RED
            };
            picture.rgba[i..i + 3].copy_from_slice(&colour);
        }
    }
    picture
}

/// A Multicolour request on a white background.
fn multicolour() -> TraceRequest {
    let mut r = request_at(50.0);
    r.selection = vectrace::Selection::RemoveBackground {
        background: Some(Rgb8::new(255, 255, 255)),
        tolerance: 40,
    };
    r.colours = Some(ColourSpec {
        count: None,
        fixed: Vec::new(),
        excluded: Vec::new(),
    });
    r
}

fn criterion_17() -> Output {
    let r1 = PxRect::new(0, 0, 500, 1000);
    run(
        &p1_colours(),
        vec![
            (PartArea::Whole, multicolour()),
            (mask(1000, r1, MaskMode::Inside), multicolour()),
        ],
    )
}

fn layer_outlines(out: &Output) -> Vec<(Rgb8, Vec<BezPath>)> {
    let mut at = 0;
    out.layers
        .iter()
        .map(|l| {
            let o = out.outlines[at..at + l.outlines].to_vec();
            at += l.outlines;
            (l.colour, o)
        })
        .collect()
}

#[test]
fn criterion_17_one_layer_per_colour_over_all_parts() {
    let out = criterion_17();
    let layers = layer_outlines(&out);
    let colours: Vec<Rgb8> = layers.iter().map(|(c, _)| *c).collect();
    println!("layers: {colours:?}");
    for (i, c) in colours.iter().enumerate() {
        assert!(!colours[i + 1..].contains(c), "{c:?} has two layers");
    }
    for expected in [RED, BLUE, GREEN] {
        assert!(
            colours.contains(&Rgb8::new(expected[0], expected[1], expected[2])),
            "{expected:?} has a layer"
        );
    }
    // The red bar crosses the seam: its red outlines were united into one piece with the disc.
    let red = &layers
        .iter()
        .find(|(c, _)| *c == Rgb8::new(RED[0], RED[1], RED[2]))
        .unwrap()
        .1;
    let positive = red.iter().filter(|o| o.area() > 0.0).count();
    assert_eq!(
        positive, 1,
        "the disc, the bar and the red left half are one outline"
    );
    // Layers of different colours overlap by at most 0.01 mm^2.
    for (i, (_, a)) in layers.iter().enumerate() {
        for (_, b) in &layers[i + 1..] {
            let overlap = grid::run(OverlayRule::Intersect, &rings(a), &rings(b));
            assert!(grid::area_mm2(&overlap).abs() <= 0.01);
        }
    }
    assert_eq!(out.paths, out.outlines.len());
    // The SVG has one `<path>` per layer and one layer per colour.
    let text = svg_of(&p1_colours(), &out);
    let doc = roxmltree::Document::parse(&text).unwrap();
    let groups = doc
        .descendants()
        .filter(|n| {
            n.attribute(("http://www.inkscape.org/namespaces/inkscape", "groupmode"))
                == Some("layer")
        })
        .count();
    assert_eq!(groups, layers.len());
    let paths = doc.descendants().filter(|n| n.has_tag_name("path")).count();
    assert_eq!(paths, layers.len(), "one path per layer");
}

#[test]
fn criterion_17_a_single_colour_part_is_black_beside_a_multicolour_one() {
    let r1 = PxRect::new(0, 0, 500, 1000);
    let out = run(
        &p1_colours(),
        vec![
            (PartArea::Whole, request_at(40.0)),
            (mask(1000, r1, MaskMode::Inside), multicolour()),
        ],
    );
    let colours: Vec<Rgb8> = out.layers.iter().map(|l| l.colour).collect();
    assert!(colours.contains(&Rgb8::new(0, 0, 0)), "{colours:?}");
    assert!(colours.contains(&Rgb8::new(RED[0], RED[1], RED[2])));
    for (i, c) in colours.iter().enumerate() {
        assert!(!colours[i + 1..].contains(c), "{c:?} has two layers");
    }
}

#[test]
fn the_multicolour_join_is_the_same_on_every_run() {
    assert_eq!(
        svg_of(&p1_colours(), &criterion_17()),
        svg_of(&p1_colours(), &criterion_17())
    );
}

#[test]
fn the_joined_svg_of_criterion_17_matches_its_golden_file() {
    let text = svg_of(&p1_colours(), &criterion_17());
    let golden = format!(
        "{}/tests/fixtures/svg/parts-17.svg",
        env!("CARGO_MANIFEST_DIR")
    );
    if std::env::var_os("TRACYO_UPDATE_GOLDEN").is_some() {
        std::fs::write(&golden, &text).unwrap();
    }
    assert_eq!(text, std::fs::read_to_string(golden).unwrap());
}

// ---- criterion 19: text and shapes ----

fn model() -> OcrModel {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/models/paddleocr-latin");
    OcrModel::load(
        &std::fs::read(dir.join("det.onnx")).unwrap(),
        &std::fs::read(dir.join("rec.onnx")).unwrap(),
        &std::fs::read_to_string(dir.join("dict.txt")).unwrap(),
    )
    .unwrap()
}

fn poster() -> Picture {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/pictures/precision.png");
    Picture::decode(&std::fs::read(path).unwrap()).unwrap()
}

fn centre(line: &OcrLine) -> (f64, f64) {
    let q = line.quad;
    (
        (q[0][0] + q[1][0] + q[2][0] + q[3][0]) / 4.0,
        (q[0][1] + q[1][1] + q[2][1] + q[3][1]) / 4.0,
    )
}

fn poster_request(lines: Vec<OcrLine>) -> TraceRequest {
    let mut r = request_at(63.0);
    r.lines = lines;
    r
}

fn text_of(out: &Output) -> Vec<String> {
    let mut strings: Vec<String> = out.texts.lines.iter().map(|l| l.text.clone()).collect();
    strings.sort();
    strings
}

/// Part 1 reads text, part 2 (a box round the big word) does not.
#[test]
fn criterion_19_text_is_kept_per_part_by_the_box_centre() {
    let picture = poster();
    let lines = Session::new(picture.clone())
        .read_text(&model(), &mut |_| {})
        .unwrap();
    let big = lines
        .iter()
        .find(|l| l.text.contains("ISION"))
        .expect("the big word is read");
    let (cx, cy) = centre(big);
    #[allow(clippy::cast_possible_truncation)]
    let around = PxRect::new(
        cx as i32 - 400,
        cy as i32 - 80,
        cx as i32 + 400,
        cy as i32 + 80,
    );
    let all_text = text_of(&run(
        &picture,
        vec![(PartArea::Whole, poster_request(lines.clone()))],
    ));
    println!("whole picture: {all_text:?}");
    assert!(all_text.iter().any(|t| t.contains("ISION")));
    // Part 2 has no text: the big word stays paths of part 2, the other lines stay text of part 1.
    let out = run(
        &picture,
        vec![
            (PartArea::Whole, poster_request(lines.clone())),
            (
                mask(1536, around, MaskMode::Inside),
                poster_request(Vec::new()),
            ),
        ],
    );
    let kept = text_of(&out);
    println!("part 2 without text: {kept:?}");
    assert!(!kept.iter().any(|t| t.contains("ISION")), "{kept:?}");
    assert!(kept.iter().any(|t| t.contains("BERATUNG")), "{kept:?}");
    let word_paths = out
        .outlines
        .iter()
        .filter(|o| {
            let b = o.bounding_box();
            let mm = 25.4 / 96.0;
            b.x0 > (cx - 400.0) * mm && b.x1 < (cx + 400.0) * mm && b.y0 > (cy - 80.0) * mm
        })
        .count();
    assert!(word_paths > 5, "the word is traced as paths: {word_paths}");
    // Text on in the later part and off in the first: the word is text, the rest is paths.
    let swapped = run(
        &picture,
        vec![
            (PartArea::Whole, poster_request(Vec::new())),
            (
                mask(1536, around, MaskMode::Inside),
                poster_request(lines.clone()),
            ),
        ],
    );
    let kept = text_of(&swapped);
    println!("part 2 with text: {kept:?}");
    assert!(kept.iter().any(|t| t.contains("ISION")), "{kept:?}");
    assert!(!kept.iter().any(|t| t.contains("BERATUNG")), "{kept:?}");
}

/// With Inside a line is kept whole when its centre is inside; a line whose centre is outside is
/// neither text nor traced.
#[test]
fn criterion_19_a_line_is_kept_whole_or_not_at_all() {
    let picture = poster();
    let lines = Session::new(picture.clone())
        .read_text(&model(), &mut |_| {})
        .unwrap();
    let small = lines
        .iter()
        .find(|l| l.text.contains("BERATUNG"))
        .expect("the small line is read");
    let (cx, cy) = centre(small);
    let mm = 25.4 / 96.0;
    // A box that ends at the line's centre plus 40 px: the centre is inside, the end is not.
    #[allow(clippy::cast_possible_truncation)]
    let (centre_in, centre_out) = (
        PxRect::new(0, cy as i32 - 60, cx as i32 + 40, cy as i32 + 60),
        PxRect::new(0, cy as i32 - 60, cx as i32 - 40, cy as i32 + 60),
    );
    let whole = |out: &Output| out.texts.lines.iter().any(|l| l.text.contains("BERATUNG"));
    let inside = run(
        &picture,
        vec![(
            mask(1536, centre_in, MaskMode::Inside),
            poster_request(lines.clone()),
        )],
    );
    assert!(whole(&inside), "kept whole: {:?}", text_of(&inside));
    let outside = run(
        &picture,
        vec![(
            mask(1536, centre_out, MaskMode::Inside),
            poster_request(lines.clone()),
        )],
    );
    assert!(!whole(&outside), "{:?}", text_of(&outside));
    let in_line = |o: &BezPath| {
        let b = o.bounding_box();
        b.y0 > (cy - 25.0) * mm && b.y1 < (cy + 25.0) * mm && b.x0 > (cx - 300.0) * mm
    };
    // The same mask with no text pass traces the letters in the line's left part.
    let plain = run(
        &picture,
        vec![(
            mask(1536, centre_out, MaskMode::Inside),
            poster_request(Vec::new()),
        )],
    );
    let (letters, left) = (
        plain.outlines.iter().filter(|o| in_line(o)).count(),
        outside.outlines.iter().filter(|o| in_line(o)).count(),
    );
    println!("outlines in the line: {letters} without the text pass, {left} with");
    // What is left is the picture's own strokes through the line's band, not its letters.
    assert!(
        letters >= left + 3,
        "the line is not traced either: {left} of {letters}"
    );
}

/// 1000 x 500 px, white: black squares A (100..300), B (400..700, crossing x = 500) and C (800..
/// 950), all y 100..300.
fn squares() -> Picture {
    let mut rgba = vec![255_u8; 1000 * 500 * 4];
    for (x0, x1) in [(100, 300), (400, 700), (800, 950)] {
        for y in 100..300 {
            for x in x0..x1 {
                let i = (y * 1000 + x) * 4;
                rgba[i..i + 3].fill(0);
            }
        }
    }
    Picture {
        width: 1000,
        height: 500,
        rgba,
    }
}

fn rect_count(out: &Output) -> usize {
    let text = svg_of(&squares(), out);
    let doc = roxmltree::Document::parse(&text).unwrap();
    doc.descendants().filter(|n| n.has_tag_name("rect")).count()
}

#[test]
fn criterion_19_a_rectangle_cut_by_a_seam_is_a_path_one_wholly_inside_is_a_rect() {
    let shapes = || {
        let mut r = request_at(50.0);
        r.shapes = true;
        r
    };
    let one = run(&squares(), vec![(PartArea::Whole, shapes())]);
    assert_eq!(rect_count(&one), 3, "three squares, three rectangles");
    let left = PxRect::new(0, 0, 500, 500);
    let out = run(
        &squares(),
        vec![
            (PartArea::Whole, shapes()),
            (mask(1000, left, MaskMode::Inside), shapes()),
        ],
    );
    assert_eq!(
        out.shapes.len(),
        2,
        "A and C stay shapes, B went into the seam union"
    );
    assert_eq!(rect_count(&out), 2, "B is a path now");
    assert_eq!(out.paths, 3);
    // Shapes are recognised after the cut: with the right part's shapes off, C is a path.
    let mut off = shapes();
    off.shapes = false;
    let out = run(
        &squares(),
        vec![
            (PartArea::Whole, off),
            (mask(1000, left, MaskMode::Inside), shapes()),
        ],
    );
    assert_eq!(rect_count(&out), 1, "only A, inside the second part");
}
