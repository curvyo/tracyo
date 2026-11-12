//! A result written as numbers by `output_wire` reads back as the same result with vectrace's
//! reader: plain, Multicolour, with text lines and with recognised shapes.

#![allow(clippy::unwrap_used)]

mod common;

use tracyo_core::job::Session;
use tracyo_core::output_wire::to_numbers;
use tracyo_core::palette::ColourSpec;
use tracyo_core::picture::Picture;
use vectrace::ocr::OcrLine;
use vectrace::wire;

use common::{p1, request_at};

fn shapes_picture() -> Picture {
    let path = format!(
        "{}/tests/fixtures/pictures/shapes.png",
        env!("CARGO_MANIFEST_DIR")
    );
    Picture::decode(&std::fs::read(path).unwrap()).unwrap()
}

fn round_trip(picture: &Picture, request: &tracyo_core::request::TraceRequest) -> wire::Output {
    let mut session = Session::new(picture.clone());
    let numbers = session.run(request, &mut |_| {});
    let read = wire::read(&numbers).unwrap();
    assert_eq!(wire::read(&to_numbers(&read)).as_ref(), Some(&read));
    read
}

#[test]
fn a_plain_trace_is_written_back_as_it_was() {
    let out = round_trip(&p1(), &request_at(70.0));
    assert!(out.paths >= 3);
}

#[test]
fn a_cut_trace_with_an_unclipped_count_round_trips() {
    let mut request = request_at(70.0);
    request.clip = tracyo_core::clip::Clip::of_regions(
        &[tracyo_core::regions::Region::rect(
            tracyo_core::regions::PxRect::new(0, 0, 500, 1000),
            false,
        )],
        tracyo_core::regions::MaskMode::Inside,
        common::full_map(1000),
    )
    .unwrap();
    round_trip(&p1(), &request);
}

#[test]
fn shapes_and_colours_round_trip() {
    let mut request = request_at(60.0);
    request.shapes = true;
    let out = round_trip(&shapes_picture(), &request);
    assert!(
        !out.shapes.is_empty(),
        "the picture has recognisable shapes"
    );
    request.colours = Some(ColourSpec {
        count: None,
        fixed: Vec::new(),
        excluded: Vec::new(),
    });
    request.selection = vectrace::Selection::RemoveBackground {
        background: Some(vectrace::Rgb8::new(255, 255, 255)),
        tolerance: 40,
    };
    let out = round_trip(&shapes_picture(), &request);
    assert_ne!(out.layers.len(), 0);
}

#[test]
fn text_lines_with_their_fills_round_trip() {
    let mut rgba = vec![255_u8; 60 * 40 * 4];
    for y in 10..30 {
        for x in 10..50 {
            let i = (y * 60 + x) * 4;
            rgba[i..i + 3].copy_from_slice(&[200, 30, 30]);
        }
    }
    let picture = Picture {
        width: 60,
        height: 40,
        rgba,
    };
    let line = OcrLine {
        quad: [[8.0, 8.0], [52.0, 8.0], [52.0, 32.0], [8.0, 32.0]],
        text: "AB \u{e9}".to_owned(),
        confidence: vec![0.95; 4],
        positions: Vec::new(),
    };
    for colours in [false, true] {
        let mut request = request_at(128.0);
        request.lines = vec![line.clone()];
        if colours {
            request.colours = Some(ColourSpec {
                count: None,
                fixed: Vec::new(),
                excluded: Vec::new(),
            });
            request.selection = vectrace::Selection::RemoveBackground {
                background: Some(vectrace::Rgb8::new(255, 255, 255)),
                tolerance: 40,
            };
        }
        let out = round_trip(&picture, &request);
        assert_eq!(out.texts.lines.len(), 1, "colours: {colours}");
    }
}
