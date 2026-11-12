//! The Inkscape SVG of a trace result.

use std::f64::consts::PI;
use std::fmt::Write;

use kurbo::{BezPath, PathEl, Point};
use vectrace::shapes::{Join, Primitive};
use vectrace::wire::Output;
use vectrace::{Angle, Rgb8};

use crate::method::hex;
use crate::request::size_mm;
use crate::shapes::{self, Element, Style};
use crate::text::{self, TextPlace};

/// Everything of one colour: one layer of the file.
#[derive(Debug, Clone)]
pub struct Layer {
    /// The layer's colour; its label and the fill of its path.
    pub colour: Rgb8,
    /// The outlines of the layer's one compound path in millimetres, holes included.
    pub outlines: Vec<BezPath>,
    /// The recognised shapes written as elements after the path.
    pub shapes: Vec<Element>,
    /// The recognised text lines after the shapes.
    pub texts: Vec<TextPlace>,
}

impl Layer {
    /// A layer of `colour` that holds only a compound path.
    #[must_use]
    pub fn new(colour: Rgb8, outlines: Vec<BezPath>) -> Self {
        Self {
            colour,
            outlines,
            shapes: Vec::new(),
            texts: Vec::new(),
        }
    }

    fn is_empty(&self) -> bool {
        self.outlines.is_empty() && self.shapes.is_empty() && self.texts.is_empty()
    }
}

/// What the SVG is made from.
#[derive(Debug, Clone, Copy)]
pub struct Document<'a> {
    /// The picture's width in pixels.
    pub width_px: u32,
    /// The picture's height in pixels.
    pub height_px: u32,
    /// The layers, the first at the bottom.
    pub layers: &'a [Layer],
}

/// The layers of a trace result: one `#000000` layer for Single colour, else one per colour in
/// the result's order (largest area first, so the largest ends at the bottom). Recognised shapes
/// that are primitives leave their layer's path and become elements of it; a text line goes to
/// the layer of its fill, a layer of its own on top when the result has none of that colour.
#[must_use]
pub fn layers_of(output: &Output) -> Vec<Layer> {
    let plan = shapes::plan(output);
    let mut layers: Vec<Layer> = if output.layers.is_empty() {
        vec![Layer::new(Rgb8::new(0, 0, 0), Vec::new())]
    } else {
        output
            .layers
            .iter()
            .map(|l| Layer::new(l.colour, Vec::new()))
            .collect()
    };
    for (index, traced) in output.outlines.iter().enumerate() {
        if plan.elsewhere[index] {
            continue;
        }
        let path = plan.drawn[index].as_ref().unwrap_or(traced);
        layers[shapes::layer_of(output, index)]
            .outlines
            .push(path.clone());
    }
    for (outer, element) in &plan.elements {
        layers[shapes::layer_of(output, *outer)]
            .shapes
            .push(*element);
    }
    for line in output.texts.lines.iter().filter(|t| !t.excluded) {
        let Some(place) = text::place(line) else {
            continue;
        };
        let colour = line.fill.unwrap_or(Rgb8::new(0, 0, 0));
        let at = layers.iter().position(|l| l.colour == colour);
        let at = at.unwrap_or_else(|| {
            layers.push(Layer::new(colour, Vec::new()));
            layers.len() - 1
        });
        layers[at].texts.push(place);
    }
    layers
}

/// A number with at most three decimals, no trailing zeros, no "-0".
#[must_use]
pub fn number(value: f64) -> String {
    let mut text = format!("{value:.3}");
    if text.contains('.') {
        text.truncate(text.trim_end_matches('0').trim_end_matches('.').len());
    }
    if text == "-0" { "0".into() } else { text }
}

fn pair(p: Point) -> String {
    format!("{} {}", number(p.x), number(p.y))
}

/// The `d` attribute of `outlines`: absolute `M L C Z` only.
fn path_data(outlines: &[BezPath]) -> String {
    let mut d = String::new();
    for outline in outlines {
        let mut current = Point::ZERO;
        for element in outline.elements() {
            if !d.is_empty() {
                d.push(' ');
            }
            match *element {
                PathEl::MoveTo(p) => {
                    let _ = write!(d, "M {}", pair(p));
                    current = p;
                }
                PathEl::LineTo(p) => {
                    let _ = write!(d, "L {}", pair(p));
                    current = p;
                }
                PathEl::QuadTo(c, p) => {
                    let c1 = current + (c - current) * (2.0 / 3.0);
                    let c2 = p + (c - p) * (2.0 / 3.0);
                    let _ = write!(d, "C {} {} {}", pair(c1), pair(c2), pair(p));
                    current = p;
                }
                PathEl::CurveTo(c1, c2, p) => {
                    let _ = write!(d, "C {} {} {}", pair(c1), pair(c2), pair(p));
                    current = p;
                }
                PathEl::ClosePath => d.push('Z'),
            }
        }
    }
    d
}

/// The numbers of the ids: layers, then each element kind, in document order.
#[derive(Default)]
struct Ids {
    path: usize,
    polygon: usize,
    star: usize,
    rect: usize,
    circle: usize,
    ellipse: usize,
    text: usize,
}

/// `value` with the characters of the SVG text escaped and those XML 1.0 forbids dropped.
fn escape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for c in value.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\t'
            | '\n'
            | '\r'
            | '\u{20}'..='\u{D7FF}'
            | '\u{E000}'..='\u{FFFD}'
            | '\u{10000}'..='\u{10FFFF}' => {
                out.push(c);
            }
            _ => {}
        }
    }
    out
}

/// The paint attributes of an element of colour `colour`.
fn paint(style: Style, colour: &str) -> String {
    match style {
        Style::Fill => format!("fill=\"{colour}\" stroke=\"none\""),
        Style::Stroke { width, join } => {
            let join = match join {
                Join::Miter => "miter",
                Join::Round => "round",
                Join::Bevel => "bevel",
            };
            format!(
                "fill=\"none\" stroke=\"{colour}\" stroke-width=\"{}\" stroke-linejoin=\"{join}\"",
                number(width.as_mm())
            )
        }
    }
}

/// `transform="rotate(deg cx cy)"` with a leading space, empty when the turn is below the
/// precision of the file.
fn turn(rotation: Angle, centre: Point) -> String {
    let degrees = number(rotation.as_radians().to_degrees());
    if degrees == "0" {
        String::new()
    } else {
        format!(" transform=\"rotate({degrees} {})\"", pair(centre))
    }
}

/// The corners of the closed straight-sided `outline`, without the closing segment.
fn corners(outline: &BezPath) -> Vec<Point> {
    outline
        .elements()
        .iter()
        .filter_map(|e| match *e {
            PathEl::MoveTo(p) | PathEl::LineTo(p) => Some(p),
            _ => None,
        })
        .collect()
}

/// One element for a recognised shape: `<rect>`, `<circle>`, `<ellipse>`, `<polygon>` or
/// Inkscape's star object (a `<path>` with `sodipodi:*` attributes and the star's own `d`).
fn element(element: &Element, colour: &str, ids: &mut Ids) -> String {
    let paint = paint(element.style, colour);
    match element.primitive {
        Primitive::Rect {
            origin,
            width,
            height,
            corner_radius,
        } => {
            ids.rect += 1;
            let (w, h) = (width.as_mm(), height.as_mm());
            let centre = Point::new(origin.x + w / 2.0, origin.y + h / 2.0);
            let r = number(corner_radius.as_mm().clamp(0.0, w.min(h) / 2.0));
            format!(
                "<rect id=\"rect{}\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"{r}\" ry=\"{r}\"{} {paint}/>",
                ids.rect,
                number(origin.x),
                number(origin.y),
                number(w),
                number(h),
                turn(element.rotation, centre)
            )
        }
        Primitive::Ellipse { centre, rx, ry } => {
            let transform = turn(element.rotation, centre);
            if (rx.as_mm() - ry.as_mm()).abs() < 1e-9 {
                ids.circle += 1;
                format!(
                    "<circle id=\"circle{}\" cx=\"{}\" cy=\"{}\" r=\"{}\"{transform} {paint}/>",
                    ids.circle,
                    number(centre.x),
                    number(centre.y),
                    number(rx.as_mm())
                )
            } else {
                ids.ellipse += 1;
                format!(
                    "<ellipse id=\"ellipse{}\" cx=\"{}\" cy=\"{}\" rx=\"{}\" ry=\"{}\"{transform} {paint}/>",
                    ids.ellipse,
                    number(centre.x),
                    number(centre.y),
                    number(rx.as_mm()),
                    number(ry.as_mm())
                )
            }
        }
        Primitive::Polygon { .. } => {
            ids.polygon += 1;
            let outline = shapes::primitive_path(&element.primitive, element.rotation);
            let points: Vec<String> = corners(&outline)
                .into_iter()
                .map(|p| format!("{},{}", number(p.x), number(p.y)))
                .collect();
            format!(
                "<polygon id=\"polygon{}\" points=\"{}\" {paint}/>",
                ids.polygon,
                points.join(" ")
            )
        }
        Primitive::Star {
            centre,
            radius,
            angle,
            points,
            inner_ratio,
        } => {
            ids.star += 1;
            let outline = shapes::primitive_path(&element.primitive, element.rotation);
            let arg1 = angle.as_radians() + element.rotation.as_radians();
            let arg2 = arg1 + PI / f64::from(points);
            format!(
                "<path id=\"star{}\" sodipodi:type=\"star\" sodipodi:cx=\"{}\" sodipodi:cy=\"{}\" sodipodi:r1=\"{}\" sodipodi:r2=\"{}\" sodipodi:arg1=\"{}\" sodipodi:arg2=\"{}\" sodipodi:sides=\"{points}\" inkscape:flatsided=\"false\" inkscape:rounded=\"0\" inkscape:randomized=\"0\" {paint} d=\"{}\"/>",
                ids.star,
                number(centre.x),
                number(centre.y),
                number(radius.as_mm()),
                number(radius.as_mm() * inner_ratio),
                number(arg1),
                number(arg2),
                path_data(&[outline])
            )
        }
    }
}

/// One `<text>` for a recognised line of colour `colour`.
fn text_element(place: &TextPlace, colour: &str, ids: &mut Ids) -> String {
    ids.text += 1;
    let turn = if number(place.degrees) == "0" {
        String::new()
    } else {
        format!(" rotate({})", number(place.degrees))
    };
    format!(
        "<text id=\"text{}\" xml:space=\"preserve\" transform=\"translate({} {}){turn}\" font-family=\"sans-serif\" font-size=\"{}\" fill=\"{colour}\" textLength=\"{}\" lengthAdjust=\"spacingAndGlyphs\"><tspan>{}</tspan></text>",
        ids.text,
        number(place.x),
        number(place.y),
        number(place.font_size),
        number(place.length),
        escape(&place.text)
    )
}

/// The SVG text of `document`: one layer per colour with its compound path, then its shapes and
/// text lines; a layer with none of them is left out.
#[must_use]
pub fn write(document: &Document<'_>) -> String {
    let width = number(size_mm(document.width_px));
    let height = number(size_mm(document.height_px));
    let mut out = String::new();
    out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    let _ = writeln!(
        out,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" xmlns:inkscape=\"http://www.inkscape.org/namespaces/inkscape\" xmlns:sodipodi=\"http://sodipodi.sourceforge.net/DTD/sodipodi-0.dtd\" version=\"1.1\" width=\"{width}mm\" height=\"{height}mm\" viewBox=\"0 0 {width} {height}\">"
    );
    out.push_str("  <sodipodi:namedview inkscape:document-units=\"mm\"/>\n");
    let mut ids = Ids::default();
    for (index, layer) in document.layers.iter().filter(|l| !l.is_empty()).enumerate() {
        let n = index + 1;
        let colour = hex(layer.colour);
        let _ = writeln!(
            out,
            "  <g inkscape:groupmode=\"layer\" inkscape:label=\"{colour}\" id=\"layer{n}\">"
        );
        if !layer.outlines.is_empty() {
            ids.path += 1;
            let _ = writeln!(
                out,
                "    <path id=\"path{}\" fill=\"{colour}\" fill-rule=\"evenodd\" stroke=\"none\" d=\"{}\"/>",
                ids.path,
                path_data(&layer.outlines)
            );
        }
        for shape in &layer.shapes {
            let _ = writeln!(out, "    {}", element(shape, &colour, &mut ids));
        }
        for place in &layer.texts {
            let _ = writeln!(out, "    {}", text_element(place, &colour, &mut ids));
        }
        out.push_str("  </g>\n");
    }
    out.push_str("</svg>\n");
    out
}

#[cfg(test)]
mod tests {
    #![allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    use super::*;
    use kurbo::Shape;

    fn square_with_hole() -> Vec<BezPath> {
        let mut outer = BezPath::new();
        outer.move_to((10.0, 10.0));
        outer.line_to((50.0, 10.0));
        outer.line_to((50.0, 40.0));
        outer.line_to((10.0, 40.0));
        outer.close_path();
        let mut hole = BezPath::new();
        hole.move_to((20.0, 20.0));
        hole.line_to((20.0, 30.0));
        hole.curve_to((25.0, 33.3333), (35.0, 33.3333), (40.0, 30.0));
        hole.line_to((40.0, 20.0));
        hole.close_path();
        vec![outer, hole]
    }

    fn black(outlines: &[BezPath]) -> Layer {
        Layer::new(Rgb8::new(0, 0, 0), outlines.to_vec())
    }

    fn square(x: f64, y: f64, side: f64) -> BezPath {
        let mut path = BezPath::new();
        path.move_to((x, y));
        path.line_to((x + side, y));
        path.line_to((x + side, y + side));
        path.line_to((x, y + side));
        path.close_path();
        path
    }

    fn fixture(name: &str) -> String {
        std::fs::read_to_string(format!(
            "{}/tests/fixtures/svg/{name}",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap()
    }

    #[test]
    fn numbers_have_three_decimals_without_trailing_zeros() {
        assert_eq!(number(1.0), "1");
        assert_eq!(number(1.5), "1.5");
        assert_eq!(number(0.123_456), "0.123");
        assert_eq!(number(2.0004), "2");
        assert_eq!(number(2.000_500_1), "2.001");
        assert_eq!(number(100.0), "100");
        assert_eq!(number(-0.0001), "0");
        assert_eq!(number(-0.0), "0");
        assert_eq!(number(-1.25), "-1.25");
    }

    #[test]
    fn the_square_with_a_hole_matches_its_golden_file() {
        let outlines = square_with_hole();
        let svg = write(&Document {
            width_px: 256,
            height_px: 192,
            layers: &[black(&outlines)],
        });
        if std::env::var_os("TRACYO_UPDATE_GOLDEN").is_some() {
            std::fs::write(
                format!(
                    "{}/tests/fixtures/svg/square-with-hole.svg",
                    env!("CARGO_MANIFEST_DIR")
                ),
                &svg,
            )
            .unwrap();
        }
        assert_eq!(svg, fixture("square-with-hole.svg"));
    }

    #[test]
    fn the_same_result_gives_the_same_bytes() {
        let outlines = square_with_hole();
        let d = Document {
            width_px: 64,
            height_px: 64,
            layers: &[black(&outlines)],
        };
        assert_eq!(write(&d), write(&d));
    }

    #[test]
    fn the_structure_follows_criterion_20() {
        let outlines = square_with_hole();
        let svg = write(&Document {
            width_px: 256,
            height_px: 192,
            layers: &[black(&outlines)],
        });
        let doc = roxmltree::Document::parse(&svg).unwrap();
        let root = doc.root_element();
        assert_eq!(root.tag_name().name(), "svg");
        assert_eq!(
            root.tag_name().namespace(),
            Some("http://www.w3.org/2000/svg")
        );
        assert_eq!(root.attribute("version"), Some("1.1"));
        let ns: Vec<_> = root
            .namespaces()
            .map(|n| (n.name().unwrap_or(""), n.uri()))
            .collect();
        assert!(ns.contains(&("inkscape", "http://www.inkscape.org/namespaces/inkscape")));
        assert!(ns.contains(&(
            "sodipodi",
            "http://sodipodi.sourceforge.net/DTD/sodipodi-0.dtd"
        )));
        // 256 px and 192 px at 96 dpi.
        assert_eq!(root.attribute("width"), Some("67.733mm"));
        assert_eq!(root.attribute("height"), Some("50.8mm"));
        assert_eq!(root.attribute("viewBox"), Some("0 0 67.733 50.8"));
        let children: Vec<_> = root
            .children()
            .filter(roxmltree::Node::is_element)
            .collect();
        assert_eq!(children.len(), 2);
        assert_eq!(children[0].tag_name().name(), "namedview");
        assert_eq!(
            children[0].attribute((
                "http://www.inkscape.org/namespaces/inkscape",
                "document-units"
            )),
            Some("mm")
        );
        let layer = children[1];
        assert_eq!(layer.tag_name().name(), "g");
        let inkscape = "http://www.inkscape.org/namespaces/inkscape";
        assert_eq!(layer.attribute((inkscape, "groupmode")), Some("layer"));
        assert_eq!(layer.attribute((inkscape, "label")), Some("#000000"));
        assert_eq!(layer.attribute("id"), Some("layer1"));
        let paths: Vec<_> = layer
            .children()
            .filter(roxmltree::Node::is_element)
            .collect();
        assert_eq!(paths.len(), 1);
        let path = paths[0];
        assert_eq!(path.tag_name().name(), "path");
        assert_eq!(path.attribute("id"), Some("path1"));
        assert_eq!(path.attribute("fill"), Some("#000000"));
        assert_eq!(path.attribute("fill-rule"), Some("evenodd"));
        assert_eq!(path.attribute("stroke"), Some("none"));
        let d = path.attribute("d").unwrap();
        assert!(
            d.split_whitespace()
                .all(|t| matches!(t, "M" | "L" | "C" | "Z")
                    || t.parse::<f64>().is_ok()
                    || t.ends_with('Z'))
        );
        assert_eq!(d.matches('M').count(), 2);
        assert_eq!(d.matches('Z').count(), 2);
        assert!(d.ends_with('Z'));
        // 20h: nothing else.
        let lower = svg.to_lowercase();
        for forbidden in [
            "curvyo",
            "tracyo",
            "<metadata",
            "<script",
            "href",
            "url(",
            "<image",
        ] {
            assert!(!lower.contains(forbidden), "{forbidden}");
        }
        assert!(svg.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"));
    }

    #[test]
    fn the_rendered_svg_covers_the_area_of_the_outlines() {
        let outlines = square_with_hole();
        let svg = write(&Document {
            width_px: 256,
            height_px: 192,
            layers: &[black(&outlines)],
        });
        let scale = 8.0_f32;
        let tree = resvg::usvg::Tree::from_str(&svg, &resvg::usvg::Options::default()).unwrap();
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
        // The reader works in 96 dpi pixels; one millimetre of the file is 96 / 25.4 of them.
        let per_mm = f64::from(scale) * 96.0 / 25.4;
        let covered: f64 = pixmap
            .pixels()
            .iter()
            .map(|p| f64::from(p.alpha()) / 255.0)
            .sum::<f64>()
            / (per_mm * per_mm);
        let expected: f64 = outlines[0].area().abs() - outlines[1].area().abs();
        assert!(
            ((covered - expected) / expected).abs() < 0.005,
            "covered {covered}, expected {expected}"
        );
    }

    const RED: Rgb8 = Rgb8::new(0xC8, 0x10, 0x2E);
    const BLUE: Rgb8 = Rgb8::new(0x2B, 0x4C, 0x9B);
    const GREEN: Rgb8 = Rgb8::new(0x1E, 0x78, 0x3C);

    /// Red with a hole, blue and green, largest first; an empty layer in between is left out.
    fn three_colours() -> (Vec<BezPath>, Vec<BezPath>, Vec<BezPath>) {
        let red = vec![square(5.0, 5.0, 40.0), square(15.0, 15.0, 10.0)];
        let blue = vec![square(46.0, 5.0, 20.0)];
        let green = vec![square(46.0, 30.0, 10.0), square(49.0, 33.0, 4.0)];
        (red, blue, green)
    }

    fn three_layers(red: &[BezPath], blue: &[BezPath], green: &[BezPath]) -> [Layer; 4] {
        [
            Layer::new(RED, red.to_vec()),
            Layer::new(Rgb8::new(1, 2, 3), Vec::new()),
            Layer::new(BLUE, blue.to_vec()),
            Layer::new(GREEN, green.to_vec()),
        ]
    }

    #[test]
    fn three_colours_match_their_golden_file() {
        let (red, blue, green) = three_colours();
        let layers = three_layers(&red, &blue, &green);
        let svg = write(&Document {
            width_px: 256,
            height_px: 192,
            layers: &layers,
        });
        if std::env::var_os("TRACYO_UPDATE_GOLDEN").is_some() {
            std::fs::write(
                format!(
                    "{}/tests/fixtures/svg/three-colours.svg",
                    env!("CARGO_MANIFEST_DIR")
                ),
                &svg,
            )
            .unwrap();
        }
        assert_eq!(svg, fixture("three-colours.svg"));
    }

    #[test]
    fn multicolour_layers_are_labelled_by_upper_case_hex_in_the_given_order() {
        let (red, blue, green) = three_colours();
        let layers = three_layers(&red, &blue, &green);
        let svg = write(&Document {
            width_px: 256,
            height_px: 192,
            layers: &layers,
        });
        let doc = roxmltree::Document::parse(&svg).unwrap();
        let inkscape = "http://www.inkscape.org/namespaces/inkscape";
        let groups: Vec<_> = doc
            .root_element()
            .children()
            .filter(|n| n.is_element() && n.tag_name().name() == "g")
            .collect();
        let labels: Vec<_> = groups
            .iter()
            .map(|g| g.attribute((inkscape, "label")).unwrap())
            .collect();
        assert_eq!(labels, ["#C8102E", "#2B4C9B", "#1E783C"], "no empty layer");
        let ids: Vec<_> = groups.iter().map(|g| g.attribute("id").unwrap()).collect();
        assert_eq!(ids, ["layer1", "layer2", "layer3"]);
        for (group, label) in groups.iter().zip(&labels) {
            assert_eq!(group.attribute((inkscape, "groupmode")), Some("layer"));
            let paths: Vec<_> = group
                .children()
                .filter(roxmltree::Node::is_element)
                .collect();
            assert_eq!(paths.len(), 1, "one path per layer");
            assert_eq!(paths[0].attribute("fill"), Some(*label));
            assert_eq!(paths[0].attribute("fill-rule"), Some("evenodd"));
            assert_eq!(paths[0].attribute("stroke"), Some("none"));
        }
        let path_ids: Vec<_> = doc
            .descendants()
            .filter(|n| n.tag_name().name() == "path")
            .map(|n| n.attribute("id").unwrap())
            .collect();
        assert_eq!(path_ids, ["path1", "path2", "path3"], "unique ids");
        let holes = groups[0]
            .descendants()
            .find(|n| n.tag_name().name() == "path");
        assert_eq!(
            holes.unwrap().attribute("d").unwrap().matches('Z').count(),
            2
        );
        for label in labels {
            assert_eq!(label, label.to_ascii_uppercase());
        }
    }

    fn output_with(layers: &[(Rgb8, usize)], outlines: usize) -> Output {
        Output {
            status: vectrace::wire::Status::Traced,
            paths: outlines,
            nodes: 0,
            specks_removed: 0,
            foreground_pixels: 0,
            covers_whole_image: false,
            outlines: (0..u8::try_from(outlines).unwrap())
                .map(|i| square(f64::from(i) * 10.0, 0.0, 5.0))
                .collect(),
            palette: Vec::new(),
            layers: layers
                .iter()
                .map(|&(colour, outlines)| vectrace::wire::Layer { colour, outlines })
                .collect(),
            joined: 0,
            texts: vectrace::wire::Texts::default(),
            shapes: Vec::new(),
            unclipped: 0,
        }
    }

    #[test]
    fn a_single_colour_result_is_one_black_layer() {
        let out = output_with(&[], 3);
        let layers = layers_of(&out);
        assert_eq!(layers.len(), 1);
        assert_eq!(hex(layers[0].colour), "#000000");
        assert_eq!(layers[0].outlines.len(), 3);
    }

    #[test]
    fn a_multicolour_result_is_split_into_its_layers_in_order() {
        let out = output_with(&[(RED, 2), (BLUE, 1), (GREEN, 3)], 6);
        let layers = layers_of(&out);
        let split: Vec<_> = layers
            .iter()
            .map(|l| (hex(l.colour), l.outlines.len()))
            .collect();
        assert_eq!(
            split,
            [
                ("#C8102E".to_owned(), 2),
                ("#2B4C9B".to_owned(), 1),
                ("#1E783C".to_owned(), 3)
            ]
        );
        assert_eq!(layers[1].outlines[0], out.outlines[2]);
    }

    #[test]
    fn each_layer_renders_in_its_colour_with_its_holes() {
        let (red, blue, green) = three_colours();
        let layers = three_layers(&red, &blue, &green);
        let svg = write(&Document {
            width_px: 256,
            height_px: 192,
            layers: &layers,
        });
        let scale = 8.0_f32;
        let tree = resvg::usvg::Tree::from_str(&svg, &resvg::usvg::Options::default()).unwrap();
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
        // A point of a millimetre as pixel of the render, one mm is 96 / 25.4 px at 96 dpi.
        let at = |x: f64, y: f64| {
            let per_mm = f64::from(scale) * 96.0 / 25.4;
            let p = pixmap
                .pixel((x * per_mm) as u32, (y * per_mm) as u32)
                .unwrap();
            (p.red(), p.green(), p.blue(), p.alpha())
        };
        assert_eq!(at(8.0, 8.0), (RED.r, RED.g, RED.b, 255));
        assert_eq!(at(56.0, 10.0), (BLUE.r, BLUE.g, BLUE.b, 255));
        assert_eq!(at(47.0, 31.0), (GREEN.r, GREEN.g, GREEN.b, 255));
        assert_eq!(at(20.0, 20.0).3, 0, "the hole of red is a hole");
        assert_eq!(at(51.0, 35.0).3, 0, "the hole of green is a hole");
        // The covered area is the area of all outlines of all layers, holes subtracted.
        let per_mm = f64::from(scale) * 96.0 / 25.4;
        let covered: f64 = pixmap
            .pixels()
            .iter()
            .map(|p| f64::from(p.alpha()) / 255.0)
            .sum::<f64>()
            / (per_mm * per_mm);
        let expected = (1600.0 - 100.0) + 400.0 + (100.0 - 16.0);
        assert!(
            ((covered - expected) / expected).abs() < 0.005,
            "covered {covered}, expected {expected}"
        );
    }

    #[test]
    fn characters_xml_forbids_are_dropped_and_the_rest_is_escaped() {
        assert_eq!(escape("A\u{0}\u{1}B\u{b}\u{fffe}C"), "ABC");
        assert_eq!(
            escape("a<b>&\"c\" \u{c4}\u{1f600}\tz"),
            "a&lt;b&gt;&amp;&quot;c&quot; \u{c4}\u{1f600}\tz"
        );
    }

    #[test]
    fn a_layer_with_only_text_is_written_and_an_empty_one_is_not() {
        let place = TextPlace {
            text: "Hi".to_owned(),
            x: 1.0,
            y: 2.0,
            degrees: 90.0,
            font_size: 3.0,
            length: 4.0,
        };
        let mut only_text = Layer::new(RED, Vec::new());
        only_text.texts.push(place);
        let svg = write(&Document {
            width_px: 64,
            height_px: 64,
            layers: &[only_text, Layer::new(BLUE, Vec::new())],
        });
        assert_eq!(svg.matches("<g ").count(), 1);
        assert!(
            svg.contains("transform=\"translate(1 2) rotate(90)\""),
            "{svg}"
        );
        assert!(!svg.contains("<path"));
    }
}
