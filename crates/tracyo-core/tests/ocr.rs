//! The OCR fixture picture read with the shipped model: the lines, the progress, and the text in
//! the trace and in the SVG. The model is read from `assets/` by path.

#![allow(clippy::unwrap_used)]

use std::path::Path;

use tracyo_core::job::Session;
use tracyo_core::picture::Picture;
use tracyo_core::request::TraceRequest;
use tracyo_core::settings::Settings;
use tracyo_core::svg;
use vectrace::ocr::OcrModel;

fn model() -> OcrModel {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/models/paddleocr-latin");
    OcrModel::load(
        &std::fs::read(dir.join("det.onnx")).unwrap(),
        &std::fs::read(dir.join("rec.onnx")).unwrap(),
        &std::fs::read_to_string(dir.join("dict.txt")).unwrap(),
    )
    .unwrap()
}

fn picture() -> Picture {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/pictures/precision.png");
    Picture::decode(&std::fs::read(path).unwrap()).unwrap()
}

fn request(lines: Vec<vectrace::ocr::OcrLine>, shapes: bool, reduced: bool) -> TraceRequest {
    TraceRequest {
        threshold: 160,
        settings: Settings::default(),
        selection: vectrace::Selection::Brightness,
        invert: false,
        drop_border: false,
        colours: None,
        reduced,
        shapes,
        lines,
    }
}

#[test]
fn the_fixture_is_read_traced_with_its_text_and_written_as_text_elements() {
    let mut session = Session::new(picture());
    let mut percents = Vec::new();
    let lines = session
        .read_text(&model(), &mut |p| percents.push(p))
        .unwrap();
    assert!(percents.windows(2).all(|w| w[0] < w[1]), "{percents:?}");
    assert_eq!(percents.last(), Some(&100));
    let said: Vec<&str> = lines.iter().map(|l| l.text.as_str()).collect();
    assert!(said.iter().any(|t| t.contains("BERATUNG")), "{said:?}");
    assert!(said.iter().any(|t| t.contains("ISION")), "{said:?}");

    let plain = vectrace::wire::read(&session.run(&request(Vec::new(), false, false), &mut |_| {}))
        .unwrap();
    let with =
        vectrace::wire::read(&session.run(&request(lines, true, false), &mut |_| {})).unwrap();
    assert!(with.texts.found >= 3, "found {}", with.texts.found);
    assert!(
        with.texts.lines.len() >= 3,
        "set {}",
        with.texts.lines.len()
    );
    assert!(
        with.texts.lines.iter().all(|t| t.masked > 0),
        "the text is left out of the paths"
    );
    assert_eq!(plain.texts.lines.len(), 0);

    let text = svg::write(&svg::Document {
        width_px: 1536,
        height_px: 1024,
        layers: &svg::layers_of(&with),
    });
    let doc = roxmltree::Document::parse(&text).unwrap();
    let texts: Vec<_> = doc
        .descendants()
        .filter(|n| n.has_tag_name("text"))
        .collect();
    assert_eq!(texts.len(), with.texts.lines.len());
    assert!(texts.iter().all(|t| t.attribute("textLength").is_some()));
    let strings: Vec<_> = texts
        .iter()
        .filter_map(|t| {
            t.text()
                .or_else(|| t.first_element_child().and_then(|c| c.text()))
        })
        .collect();
    assert!(
        strings.iter().any(|s| s.contains("BERATUNG")),
        "{strings:?}"
    );
}

#[test]
fn the_reduced_copy_traces_the_same_lines_in_the_same_millimetres() {
    let mut session = Session::new(picture());
    let lines = session.read_text(&model(), &mut |_| {}).unwrap();
    let full =
        vectrace::wire::read(&session.run(&request(lines.clone(), false, false), &mut |_| {}))
            .unwrap();
    let small =
        vectrace::wire::read(&session.run(&request(lines, false, true), &mut |_| {})).unwrap();
    assert_eq!(small.texts.lines.len(), full.texts.lines.len());
    let first =
        |o: &vectrace::wire::Output| kurbo::Shape::bounding_box(&o.texts.lines[0].outlines[0]);
    let (a, b) = (first(&full), first(&small));
    assert!(
        (a.x0 - b.x0).abs() < 1.0 && (a.y0 - b.y0).abs() < 1.0,
        "{a:?} {b:?}"
    );
}
