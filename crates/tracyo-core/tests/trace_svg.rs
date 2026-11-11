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

fn trace() -> (Picture, vectrace::wire::Output) {
    let bytes = std::fs::read(format!("{}/tests/fixtures/pictures/shapes.png", dir())).unwrap();
    let picture = Picture::decode(&bytes).unwrap();
    let mut session = Session::new(picture.clone());
    let request = TraceRequest {
        threshold: 128,
        settings: Settings::default(),
        reduced: false,
    };
    let output = vectrace::wire::read(&session.run(&request, &mut |_| {})).unwrap();
    (picture, output)
}

#[test]
fn the_traced_shapes_match_their_golden_file() {
    let (picture, output) = trace();
    assert_eq!(output.paths, 5, "square, ring with its hole, dot, triangle");
    let text = svg::write(&svg::Document {
        width_px: picture.width,
        height_px: picture.height,
        outlines: &output.outlines,
    });
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
    let text = svg::write(&svg::Document {
        width_px: picture.width,
        height_px: picture.height,
        outlines: &output.outlines,
    });
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
    let text = svg::write(&svg::Document {
        width_px: picture.width,
        height_px: picture.height,
        outlines: &output.outlines,
    });
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
