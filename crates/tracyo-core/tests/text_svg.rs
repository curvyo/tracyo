//! Recognised text written as `<text>`: the golden file, the structure of criterion 20g, the
//! escaping and the layer a line goes to.

#![allow(clippy::unwrap_used)]

use tracyo_core::job::Session;
use tracyo_core::palette::ColourSpec;
use tracyo_core::picture::Picture;
use tracyo_core::request::TraceRequest;
use tracyo_core::settings::Settings;
use tracyo_core::svg;
use vectrace::ocr::OcrLine;

/// A white picture with black rectangles `(x0, y0, x1, y1)` standing for lines of text.
fn picture(width: u32, height: u32, blocks: &[(u32, u32, u32, u32, [u8; 3])]) -> Picture {
    let mut rgba = vec![255_u8; (width * height * 4) as usize];
    for &(x0, y0, x1, y1, colour) in blocks {
        for y in y0..y1 {
            for x in x0..x1 {
                let i = ((y * width + x) * 4) as usize;
                rgba[i..i + 3].copy_from_slice(&colour);
            }
        }
    }
    Picture {
        width,
        height,
        rgba,
    }
}

fn line(text: &str, x0: f64, y0: f64, x1: f64, y1: f64) -> OcrLine {
    OcrLine {
        quad: [[x0, y0], [x1, y0], [x1, y1], [x0, y1]],
        text: text.to_owned(),
        confidence: vec![0.95; text.chars().count()],
        positions: Vec::new(),
    }
}

fn trace(
    picture: &Picture,
    lines: Vec<OcrLine>,
    colours: Option<ColourSpec>,
) -> vectrace::wire::Output {
    let selection = if colours.is_some() {
        vectrace::Selection::RemoveBackground {
            background: Some(vectrace::Rgb8::new(255, 255, 255)),
            tolerance: 40,
        }
    } else {
        vectrace::Selection::Brightness
    };
    let request = TraceRequest {
        threshold: 128,
        settings: Settings::default(),
        selection,
        invert: false,
        drop_border: false,
        colours,
        reduced: false,
        shapes: false,
        lines,
        clip: None,
    };
    let mut session = Session::new(picture.clone());
    vectrace::wire::read(&session.run(&request, &mut |_| {})).unwrap()
}

fn write(picture: &Picture, output: &vectrace::wire::Output) -> String {
    svg::write(&svg::Document {
        width_px: picture.width,
        height_px: picture.height,
        layers: &svg::layers_of(output),
    })
}

const BLACK: [u8; 3] = [0, 0, 0];

fn two_lines() -> (Picture, vectrace::wire::Output) {
    let picture = picture(120, 70, &[(10, 10, 90, 30, BLACK), (10, 40, 70, 56, BLACK)]);
    let lines = vec![
        line("HELLO", 8.0, 8.0, 92.0, 32.0),
        line("A&B <\"x\">", 8.0, 38.0, 72.0, 58.0),
    ];
    let output = trace(&picture, lines, None);
    (picture, output)
}

#[test]
fn two_lines_match_their_golden_file() {
    let (picture, output) = two_lines();
    assert_eq!(output.texts.lines.len(), 2);
    let text = write(&picture, &output);
    let golden = format!(
        "{}/tests/fixtures/svg/recognised-text.svg",
        env!("CARGO_MANIFEST_DIR")
    );
    if std::env::var_os("TRACYO_UPDATE_GOLDEN").is_some() {
        std::fs::write(&golden, &text).unwrap();
    }
    assert_eq!(text, std::fs::read_to_string(golden).unwrap());
}

/// Criterion 20g: one `<text>` per line with the attributes of the criterion, the string escaped,
/// the file well formed, and the text's pixels not also in the paths.
#[test]
fn a_line_is_a_text_element_stretched_over_the_original_width() {
    let (picture, output) = two_lines();
    let text = write(&picture, &output);
    let doc = roxmltree::Document::parse(&text).unwrap();
    let texts: Vec<_> = doc
        .descendants()
        .filter(|n| n.has_tag_name("text"))
        .collect();
    assert_eq!(texts.len(), 2);
    let first = texts[0];
    assert_eq!(
        first.attribute(("http://www.w3.org/XML/1998/namespace", "space")),
        Some("preserve")
    );
    assert_eq!(first.attribute("font-family"), Some("sans-serif"));
    assert_eq!(first.attribute("fill"), Some("#000000"));
    assert_eq!(first.attribute("lengthAdjust"), Some("spacingAndGlyphs"));
    let mm = 25.4 / 96.0;
    // 80 px wide and 20 px high ink of capitals (0.69 em), its bottom edge at y = 30 px.
    let length: f64 = first.attribute("textLength").unwrap().parse().unwrap();
    let size: f64 = first.attribute("font-size").unwrap().parse().unwrap();
    assert!((length - 80.0 * mm).abs() < 0.05, "textLength {length}");
    assert!((size - 20.0 * mm / 0.69).abs() < 0.05, "font-size {size}");
    let transform = first.attribute("transform").unwrap();
    assert!(transform.starts_with("translate("), "{transform}");
    let numbers: Vec<f64> = transform
        .trim_start_matches("translate(")
        .trim_end_matches(')')
        .split_whitespace()
        .map(|n| n.parse().unwrap())
        .collect();
    assert!((numbers[0] - 10.0 * mm).abs() < 0.05 && (numbers[1] - 30.0 * mm).abs() < 0.05);
    let tspan = first.children().find(|n| n.has_tag_name("tspan")).unwrap();
    assert_eq!(tspan.text(), Some("HELLO"));
    assert_eq!(
        texts[1]
            .children()
            .find(|n| n.has_tag_name("tspan"))
            .unwrap()
            .text(),
        Some("A&B <\"x\">"),
        "escaped in the file, read back whole"
    );
    assert!(text.contains("A&amp;B &lt;&quot;x&quot;&gt;"));
    // Nothing of the text is traced as a path.
    assert_eq!(output.paths, 0);
    assert_eq!(
        doc.descendants().filter(|n| n.has_tag_name("path")).count(),
        0
    );
}

#[test]
fn the_text_is_left_out_of_the_paths_and_other_shapes_stay() {
    let picture = picture(
        140,
        70,
        &[(10, 10, 90, 30, BLACK), (100, 10, 130, 60, BLACK)],
    );
    let output = trace(&picture, vec![line("HELLO", 8.0, 8.0, 92.0, 32.0)], None);
    assert_eq!((output.paths, output.texts.lines.len()), (1, 1));
    let text = write(&picture, &output);
    let doc = roxmltree::Document::parse(&text).unwrap();
    assert_eq!(
        doc.descendants().filter(|n| n.has_tag_name("path")).count(),
        1
    );
    assert_eq!(
        doc.descendants().filter(|n| n.has_tag_name("text")).count(),
        1
    );
}

/// With Multicolour a line goes to the layer of its colour; a layer that only has text is still
/// a layer.
#[test]
fn a_line_goes_to_the_layer_of_its_colour() {
    let red = [200, 16, 46];
    let picture = picture(
        140,
        70,
        &[(10, 10, 90, 30, red), (100, 10, 130, 60, [43, 76, 155])],
    );
    let spec = ColourSpec {
        count: None,
        fixed: Vec::new(),
        excluded: Vec::new(),
    };
    let output = trace(
        &picture,
        vec![line("HELLO", 8.0, 8.0, 92.0, 32.0)],
        Some(spec),
    );
    assert_eq!(output.texts.lines.len(), 1);
    let text = write(&picture, &output);
    let doc = roxmltree::Document::parse(&text).unwrap();
    let inkscape = "http://www.inkscape.org/namespaces/inkscape";
    let layers: Vec<_> = doc
        .root_element()
        .children()
        .filter(|n| n.has_tag_name("g"))
        .collect();
    let labelled = |label: &str| {
        layers
            .iter()
            .find(|g| g.attribute((inkscape, "label")) == Some(label))
    };
    let red_layer = labelled("#C8102E").expect("the red layer exists for its text");
    let t = red_layer
        .children()
        .find(|n| n.has_tag_name("text"))
        .unwrap();
    assert_eq!(t.attribute("fill"), Some("#C8102E"));
    assert!(
        red_layer.children().all(|n| !n.has_tag_name("path")),
        "no red path is left"
    );
    let blue = labelled("#2B4C9B").unwrap();
    assert!(blue.children().any(|n| n.has_tag_name("path")));
    assert!(blue.children().all(|n| !n.has_tag_name("text")));
}
