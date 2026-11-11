//! The Inkscape SVG of a trace result.

use std::fmt::Write;

use kurbo::{BezPath, PathEl, Point};

use crate::request::size_mm;

/// What the SVG is made from.
#[derive(Debug, Clone, Copy)]
pub struct Document<'a> {
    /// The picture's width in pixels.
    pub width_px: u32,
    /// The picture's height in pixels.
    pub height_px: u32,
    /// The outlines in millimetres.
    pub outlines: &'a [BezPath],
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

/// The SVG text of `document`: one `#000000` layer with one path.
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
    out.push_str("  <g inkscape:groupmode=\"layer\" inkscape:label=\"#000000\" id=\"layer1\">\n");
    let _ = writeln!(
        out,
        "    <path id=\"path1\" fill=\"#000000\" fill-rule=\"evenodd\" stroke=\"none\" d=\"{}\"/>",
        path_data(document.outlines)
    );
    out.push_str("  </g>\n</svg>\n");
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

    fn fixture(name: &str) -> String {
        std::fs::read_to_string(format!("{}/tests/fixtures/svg/{name}", env!("CARGO_MANIFEST_DIR"))).unwrap()
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
        let svg = write(&Document { width_px: 256, height_px: 192, outlines: &outlines });
        if std::env::var_os("TRACYO_UPDATE_GOLDEN").is_some() {
            std::fs::write(format!("{}/tests/fixtures/svg/square-with-hole.svg", env!("CARGO_MANIFEST_DIR")), &svg).unwrap();
        }
        assert_eq!(svg, fixture("square-with-hole.svg"));
    }

    #[test]
    fn the_same_result_gives_the_same_bytes() {
        let outlines = square_with_hole();
        let d = Document { width_px: 64, height_px: 64, outlines: &outlines };
        assert_eq!(write(&d), write(&d));
    }

    #[test]
    fn the_structure_follows_criterion_20() {
        let outlines = square_with_hole();
        let svg = write(&Document { width_px: 256, height_px: 192, outlines: &outlines });
        let doc = roxmltree::Document::parse(&svg).unwrap();
        let root = doc.root_element();
        assert_eq!(root.tag_name().name(), "svg");
        assert_eq!(root.tag_name().namespace(), Some("http://www.w3.org/2000/svg"));
        assert_eq!(root.attribute("version"), Some("1.1"));
        let ns: Vec<_> = root.namespaces().map(|n| (n.name().unwrap_or(""), n.uri())).collect();
        assert!(ns.contains(&("inkscape", "http://www.inkscape.org/namespaces/inkscape")));
        assert!(ns.contains(&("sodipodi", "http://sodipodi.sourceforge.net/DTD/sodipodi-0.dtd")));
        // 256 px and 192 px at 96 dpi.
        assert_eq!(root.attribute("width"), Some("67.733mm"));
        assert_eq!(root.attribute("height"), Some("50.8mm"));
        assert_eq!(root.attribute("viewBox"), Some("0 0 67.733 50.8"));
        let children: Vec<_> = root.children().filter(roxmltree::Node::is_element).collect();
        assert_eq!(children.len(), 2);
        assert_eq!(children[0].tag_name().name(), "namedview");
        assert_eq!(children[0].attribute(("http://www.inkscape.org/namespaces/inkscape", "document-units")), Some("mm"));
        let layer = children[1];
        assert_eq!(layer.tag_name().name(), "g");
        let inkscape = "http://www.inkscape.org/namespaces/inkscape";
        assert_eq!(layer.attribute((inkscape, "groupmode")), Some("layer"));
        assert_eq!(layer.attribute((inkscape, "label")), Some("#000000"));
        assert_eq!(layer.attribute("id"), Some("layer1"));
        let paths: Vec<_> = layer.children().filter(roxmltree::Node::is_element).collect();
        assert_eq!(paths.len(), 1);
        let path = paths[0];
        assert_eq!(path.tag_name().name(), "path");
        assert_eq!(path.attribute("id"), Some("path1"));
        assert_eq!(path.attribute("fill"), Some("#000000"));
        assert_eq!(path.attribute("fill-rule"), Some("evenodd"));
        assert_eq!(path.attribute("stroke"), Some("none"));
        let d = path.attribute("d").unwrap();
        assert!(d.split_whitespace().all(|t| matches!(t, "M" | "L" | "C" | "Z") || t.parse::<f64>().is_ok() || t.ends_with('Z')));
        assert_eq!(d.matches('M').count(), 2);
        assert_eq!(d.matches('Z').count(), 2);
        assert!(d.ends_with('Z'));
        // 20h: nothing else.
        let lower = svg.to_lowercase();
        for forbidden in ["curvyo", "tracyo", "<metadata", "<script", "href", "url(", "<image"] {
            assert!(!lower.contains(forbidden), "{forbidden}");
        }
        assert!(svg.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"));
    }

    #[test]
    fn the_rendered_svg_covers_the_area_of_the_outlines() {
        let outlines = square_with_hole();
        let svg = write(&Document { width_px: 256, height_px: 192, outlines: &outlines });
        let scale = 8.0_f32;
        let tree = resvg::usvg::Tree::from_str(&svg, &resvg::usvg::Options::default()).unwrap();
        let mut pixmap = resvg::tiny_skia::Pixmap::new(
            (tree.size().width() * scale).ceil() as u32,
            (tree.size().height() * scale).ceil() as u32,
        )
        .unwrap();
        resvg::render(&tree, resvg::tiny_skia::Transform::from_scale(scale, scale), &mut pixmap.as_mut());
        // The reader works in 96 dpi pixels; one millimetre of the file is 96 / 25.4 of them.
        let per_mm = f64::from(scale) * 96.0 / 25.4;
        let covered: f64 = pixmap.pixels().iter().map(|p| f64::from(p.alpha()) / 255.0).sum::<f64>() / (per_mm * per_mm);
        let expected: f64 = outlines[0].area().abs() - outlines[1].area().abs();
        assert!(((covered - expected) / expected).abs() < 0.005, "covered {covered}, expected {expected}");
    }
}
