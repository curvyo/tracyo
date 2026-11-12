//! A traced fixture picture written as SVG: the golden file, the area check and the XML check.

#![allow(clippy::unwrap_used)]

use kurbo::Shape;
use tracyo_core::job::Session;
use tracyo_core::picture::Picture;
use tracyo_core::request::TraceRequest;
use tracyo_core::settings::Settings;
use tracyo_core::svg;

fn dir() -> String {
    env!("CARGO_MANIFEST_DIR").to_owned()
}

fn write(picture: &Picture, output: &vectrace::wire::Output) -> String {
    svg::write(&svg::Document {
        width_px: picture.width,
        height_px: picture.height,
        layers: &svg::layers_of(output),
    })
}

fn trace() -> (Picture, vectrace::wire::Output) {
    let bytes = std::fs::read(format!("{}/tests/fixtures/pictures/shapes.png", dir())).unwrap();
    let picture = Picture::decode(&bytes).unwrap();
    let mut session = Session::new(picture.clone());
    let request = TraceRequest {
        threshold: 128,
        settings: Settings::default(),
        selection: vectrace::Selection::Brightness,
        invert: false,
        drop_border: false,
        colours: None,
        reduced: false,
        shapes: false,
        lines: Vec::new(),
    };
    let output = vectrace::wire::read(&session.run(&request, &mut |_| {})).unwrap();
    (picture, output)
}

#[test]
fn the_traced_shapes_match_their_golden_file() {
    let (picture, output) = trace();
    assert_eq!(output.paths, 5, "square, ring with its hole, dot, triangle");
    let text = write(&picture, &output);
    let golden = format!("{}/tests/fixtures/svg/shapes.svg", dir());
    if std::env::var_os("TRACYO_UPDATE_GOLDEN").is_some() {
        std::fs::write(&golden, &text).unwrap();
    }
    assert_eq!(text, std::fs::read_to_string(golden).unwrap());
}

/// Criterion 21, the automatic substitute: the SVG, rendered by a second reader, covers the area
/// of the traced outlines to within 0.5 %.
#[test]
fn a_second_reader_covers_the_traced_area() {
    let (picture, output) = trace();
    let text = write(&picture, &output);
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
    let per_mm = f64::from(scale) * 96.0 / 25.4;
    let covered: f64 = pixmap
        .pixels()
        .iter()
        .map(|p| f64::from(p.alpha()) / 255.0)
        .sum::<f64>()
        / (per_mm * per_mm);
    // Even-odd area of the outlines: outlines nest, so the signed areas of outer outlines and
    // holes cancel as in the fill.
    let expected: f64 = output.outlines.iter().map(Shape::area).sum::<f64>().abs();
    assert!(
        ((covered - expected) / expected).abs() < 0.005,
        "covered {covered} mm2, outlines {expected} mm2"
    );
}

#[test]
fn the_file_is_well_formed_xml_with_one_layer_and_one_path() {
    let (picture, output) = trace();
    let text = write(&picture, &output);
    let doc = roxmltree::Document::parse(&text).unwrap();
    let layers = doc
        .root_element()
        .children()
        .filter(|n| n.is_element() && n.tag_name().name() == "g")
        .count();
    assert_eq!(layers, 1);
    assert_eq!(
        doc.descendants()
            .filter(|n| n.tag_name().name() == "path")
            .count(),
        1
    );
}

/// A white 160 x 120 picture with a red 80 x 80 square holding a 30 x 30 white hole, a blue
/// 40 x 40 square and a green 30 x 30 one: areas 5500, 1600 and 900 pixels.
fn colour_picture() -> Picture {
    let (width, height) = (160_usize, 120_usize);
    let mut rgba = vec![255_u8; width * height * 4];
    let mut paint = |x0: usize, y0: usize, side: usize, c: [u8; 3]| {
        for y in y0..y0 + side {
            for x in x0..x0 + side {
                let i = (y * width + x) * 4;
                rgba[i..i + 3].copy_from_slice(&c);
            }
        }
    };
    paint(10, 10, 80, [0xC8, 0x10, 0x2E]);
    paint(35, 35, 30, [255, 255, 255]);
    paint(100, 10, 40, [0x2B, 0x4C, 0x9B]);
    paint(100, 70, 30, [0x1E, 0x78, 0x3C]);
    Picture {
        width: 160,
        height: 120,
        rgba,
    }
}

fn trace_colours(excluded: Vec<vectrace::Rgb8>) -> (Picture, vectrace::wire::Output) {
    let picture = colour_picture();
    let mut session = Session::new(picture.clone());
    let request = TraceRequest {
        threshold: 128,
        settings: Settings::default(),
        selection: vectrace::Selection::RemoveBackground {
            background: Some(vectrace::Rgb8::new(255, 255, 255)),
            tolerance: 40,
        },
        invert: false,
        drop_border: false,
        colours: Some(tracyo_core::palette::ColourSpec {
            count: None,
            fixed: Vec::new(),
            excluded,
        }),
        reduced: false,
        shapes: false,
        lines: Vec::new(),
    };
    let output = vectrace::wire::read(&session.run(&request, &mut |_| {})).unwrap();
    (picture, output)
}

fn layer_labels(text: &str) -> Vec<String> {
    let doc = roxmltree::Document::parse(text).unwrap();
    doc.root_element()
        .children()
        .filter(|n| n.is_element() && n.tag_name().name() == "g")
        .map(|g| {
            g.attribute(("http://www.inkscape.org/namespaces/inkscape", "label"))
                .unwrap()
                .to_owned()
        })
        .collect()
}

#[test]
fn a_multicolour_trace_has_one_layer_per_colour_largest_at_the_bottom() {
    let (picture, output) = trace_colours(Vec::new());
    let text = write(&picture, &output);
    assert_eq!(layer_labels(&text), ["#C8102E", "#2B4C9B", "#1E783C"]);
    let golden = format!("{}/tests/fixtures/svg/multicolour-shapes.svg", dir());
    if std::env::var_os("TRACYO_UPDATE_GOLDEN").is_some() {
        std::fs::write(&golden, &text).unwrap();
    }
    assert_eq!(text, std::fs::read_to_string(golden).unwrap());
}

#[test]
fn an_excluded_colour_has_no_layer_in_the_file() {
    let (picture, output) = trace_colours(vec![vectrace::Rgb8::new(0x2B, 0x4C, 0x9B)]);
    assert_eq!(
        layer_labels(&write(&picture, &output)),
        ["#C8102E", "#1E783C"]
    );
}

fn render(text: &str, scale: f32) -> resvg::tiny_skia::Pixmap {
    let tree = resvg::usvg::Tree::from_str(text, &resvg::usvg::Options::default()).unwrap();
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
    pixmap
}

/// Criterion 21, the automatic substitute, for Multicolour: a second reader draws every colour
/// where the picture has it, holes stay holes, and the covered area is the area of the outlines.
#[test]
fn a_second_reader_draws_every_colour_where_the_picture_has_it() {
    let (picture, output) = trace_colours(Vec::new());
    let text = write(&picture, &output);
    let scale = 8.0_f32;
    let pixmap = render(&text, scale);
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let at = |x: f32, y: f32| {
        let p = pixmap
            .pixel((x * scale) as u32, (y * scale) as u32)
            .unwrap();
        (p.red(), p.green(), p.blue(), p.alpha())
    };
    assert_eq!(at(20.0, 20.0), (0xC8, 0x10, 0x2E, 255));
    assert_eq!(at(120.0, 30.0), (0x2B, 0x4C, 0x9B, 255));
    assert_eq!(at(115.0, 85.0), (0x1E, 0x78, 0x3C, 255));
    assert_eq!(at(50.0, 50.0).3, 0, "the hole of the red square");
    assert_eq!(at(150.0, 110.0).3, 0, "the background");
    // The file's unit is a millimetre, drawn at 96 dpi, so one picture pixel is `scale` render
    // pixels wide.
    let per_px = f64::from(scale);
    let covered: f64 = pixmap
        .pixels()
        .iter()
        .map(|p| f64::from(p.alpha()) / 255.0)
        .sum::<f64>()
        / (per_px * per_px);
    // The red square's hole is an outline too, so the signed sum is the covered area.
    let mm_per_px = 25.4 / 96.0;
    let outlines_px =
        output.outlines.iter().map(Shape::area).sum::<f64>().abs() / (mm_per_px * mm_per_px);
    assert!(
        ((covered - outlines_px) / outlines_px).abs() < 0.005,
        "covered {covered} px2, outlines {outlines_px} px2"
    );
    assert!((outlines_px - 8000.0).abs() < 100.0, "5500 + 1600 + 900");
}
