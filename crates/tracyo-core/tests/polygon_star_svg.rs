//! Polygons and stars as the elements of criterion 20f: `<polygon>` and Inkscape's star object.
//! The golden file, the attributes, and a second reader drawing the same pixels as the path.

#![allow(clippy::unwrap_used, clippy::cast_possible_truncation)]

use std::f64::consts::{FRAC_PI_2, PI, TAU};

use kurbo::Point;
use tracyo_core::shapes::{Element, Style, primitive_path};
use tracyo_core::svg::{self, Document, Layer};
use vectrace::shapes::{Join, Primitive};
use vectrace::{Angle, Mm, Rgb8};

const WIDTH: u32 = 400;
const HEIGHT: u32 = 200;

fn mm(px: f64) -> Mm {
    Mm::from_mm(px * 25.4 / 96.0)
}

fn polygon(cx: f64, cy: f64, r: f64, points: u32, angle: f64, turn: f64, style: Style) -> Element {
    Element {
        primitive: Primitive::Polygon {
            centre: Point::new(cx * 25.4 / 96.0, cy * 25.4 / 96.0),
            radius: mm(r),
            angle: Angle::from_radians(angle),
            points,
        },
        rotation: Angle::from_radians(turn),
        style,
    }
}

fn star(cx: f64, cy: f64, r: f64, points: u32, ratio: f64, turn: f64, style: Style) -> Element {
    Element {
        primitive: Primitive::Star {
            centre: Point::new(cx * 25.4 / 96.0, cy * 25.4 / 96.0),
            radius: mm(r),
            angle: Angle::from_radians(-FRAC_PI_2),
            points,
            inner_ratio: ratio,
        },
        rotation: Angle::from_radians(turn),
        style,
    }
}

fn band() -> Style {
    Style::Stroke {
        width: mm(5.0),
        join: Join::Round,
    }
}

/// A hexagon, a rotated triangle, a rotated pentagon band, a default star, a star with a thin
/// inner ratio and a rotated seven-pointed star band.
fn elements() -> Vec<Element> {
    vec![
        polygon(45.0, 50.0, 35.0, 6, 0.0, 0.0, Style::Fill),
        polygon(125.0, 50.0, 35.0, 3, -FRAC_PI_2, 0.5, Style::Fill),
        polygon(205.0, 50.0, 32.0, 5, -FRAC_PI_2, 0.3, band()),
        star(45.0, 145.0, 38.0, 5, 0.5, 0.0, Style::Fill),
        star(135.0, 145.0, 38.0, 6, 0.3, 0.4, Style::Fill),
        star(235.0, 145.0, 36.0, 7, 0.6, 0.2, band()),
    ]
}

fn document_of(layers: &[Layer]) -> String {
    svg::write(&Document {
        width_px: WIDTH,
        height_px: HEIGHT,
        layers,
    })
}

fn elements_svg() -> String {
    let mut layer = Layer::new(Rgb8::new(0, 0, 0), Vec::new());
    layer.shapes = elements();
    document_of(&[layer])
}

/// The same shapes as plain paths: the primitives' outlines, stroked as the elements are.
fn paths_svg() -> String {
    let mut fills = Layer::new(Rgb8::new(0, 0, 0), Vec::new());
    let mut bands = String::new();
    for e in elements() {
        let path = primitive_path(&e.primitive, e.rotation);
        match e.style {
            Style::Fill => fills.outlines.push(path),
            Style::Stroke { width, .. } => {
                bands.push_str(&format!(
                    "<path fill=\"none\" stroke=\"#000000\" stroke-width=\"{}\" stroke-linejoin=\"round\" d=\"{}\"/>",
                    svg::number(width.as_mm()),
                    path.to_svg()
                ));
            }
        }
    }
    let text = document_of(&[fills]);
    text.replace("</g>", &format!("{bands}</g>"))
}

#[test]
fn polygons_and_stars_match_their_golden_file() {
    let text = elements_svg();
    let golden = format!(
        "{}/tests/fixtures/svg/polygon-star.svg",
        env!("CARGO_MANIFEST_DIR")
    );
    if std::env::var_os("TRACYO_UPDATE_GOLDEN").is_some() {
        std::fs::write(&golden, &text).unwrap();
    }
    assert_eq!(text, std::fs::read_to_string(golden).unwrap());
}

fn numbers(text: &str) -> Vec<f64> {
    text.split(|c: char| c == ',' || c.is_whitespace())
        .filter(|s| !s.is_empty())
        .map(|s| s.parse().unwrap())
        .collect()
}

#[test]
fn a_polygon_is_a_polygon_element_with_the_rotation_in_its_points() {
    let text = elements_svg();
    let doc = roxmltree::Document::parse(&text).unwrap();
    let polygons: Vec<_> = doc
        .descendants()
        .filter(|n| n.has_tag_name("polygon"))
        .collect();
    assert_eq!(polygons.len(), 3);
    for (n, sides) in polygons.iter().zip([6, 3, 5]) {
        assert!(n.attribute("transform").is_none(), "rotation is baked in");
        assert_eq!(numbers(n.attribute("points").unwrap()).len(), sides * 2);
    }
    assert_eq!(polygons[0].attribute("id"), Some("polygon1"));
    assert_eq!(polygons[2].attribute("id"), Some("polygon3"));
    assert_eq!(polygons[0].attribute("fill"), Some("#000000"));
    assert_eq!(polygons[0].attribute("stroke"), Some("none"));
    // The band keeps fill none, stroke, width and join on the same element.
    assert_eq!(polygons[2].attribute("fill"), Some("none"));
    assert_eq!(polygons[2].attribute("stroke"), Some("#000000"));
    assert_eq!(polygons[2].attribute("stroke-width"), Some("1.323"));
    assert_eq!(polygons[2].attribute("stroke-linejoin"), Some("round"));
    // The triangle is turned by 0.5 rad about its centre: its first corner is at -pi/2 + 0.5.
    let p = numbers(polygons[1].attribute("points").unwrap());
    let (cx, cy, r) = (125.0 * 25.4 / 96.0, 50.0 * 25.4 / 96.0, 35.0 * 25.4 / 96.0);
    let angle = -FRAC_PI_2 + 0.5;
    assert!((p[0] - (cx + r * angle.cos())).abs() < 0.002, "{p:?}");
    assert!((p[1] - (cy + r * angle.sin())).abs() < 0.002, "{p:?}");
}

#[test]
fn a_star_is_inkscapes_star_object_whose_d_is_the_same_star() {
    let text = elements_svg();
    let doc = roxmltree::Document::parse(&text).unwrap();
    let sodipodi = "http://sodipodi.sourceforge.net/DTD/sodipodi-0.dtd";
    let inkscape = "http://www.inkscape.org/namespaces/inkscape";
    let stars: Vec<_> = doc
        .descendants()
        .filter(|n| n.attribute((sodipodi, "type")) == Some("star"))
        .collect();
    assert_eq!(stars.len(), 3);
    for (n, (sides, ratio)) in stars.iter().zip([(5.0, 0.5), (6.0, 0.3), (7.0, 0.6)]) {
        assert!(n.has_tag_name("path"));
        let get = |name: &str| -> f64 { n.attribute((sodipodi, name)).unwrap().parse().unwrap() };
        assert!((get("sides") - sides).abs() < f64::EPSILON);
        assert!((get("r2") / get("r1") - ratio).abs() < 0.002, "inner ratio");
        assert!(
            (get("arg2") - get("arg1") - PI / sides).abs() < 0.002,
            "the first inner corner is half a step on"
        );
        assert_eq!(n.attribute((inkscape, "flatsided")), Some("false"));
        assert_eq!(n.attribute((inkscape, "rounded")), Some("0"));
        assert_eq!(n.attribute((inkscape, "randomized")), Some("0"));
        assert!(n.attribute("transform").is_none());
        // d is M tip L inner L tip ... Z with the corners where Inkscape puts them.
        let d = n.attribute("d").unwrap();
        let v = numbers(&d.replace(['M', 'L', 'Z'], " "));
        assert_eq!(v.len() as f64, sides * 4.0);
        let (cx, cy) = (get("cx"), get("cy"));
        for (k, c) in v.chunks(2).enumerate() {
            let (radius, angle) = if k % 2 == 0 {
                (get("r1"), get("arg1") + TAU * (k / 2) as f64 / sides)
            } else {
                (get("r2"), get("arg2") + TAU * (k / 2) as f64 / sides)
            };
            assert!(
                (c[0] - (cx + radius * angle.cos())).abs() < 0.005,
                "x of corner {k}"
            );
            assert!(
                (c[1] - (cy + radius * angle.sin())).abs() < 0.005,
                "y of corner {k}"
            );
        }
    }
    assert_eq!(stars[0].attribute("id"), Some("star1"));
    assert_eq!(stars[2].attribute("id"), Some("star3"));
    assert_eq!(stars[0].attribute("fill"), Some("#000000"));
    assert_eq!(stars[2].attribute("fill"), Some("none"));
    assert_eq!(stars[2].attribute("stroke"), Some("#000000"));
    assert_eq!(stars[2].attribute("stroke-width"), Some("1.323"));
    assert_eq!(stars[2].attribute("stroke-linejoin"), Some("round"));
}

fn render(text: &str) -> Vec<bool> {
    let tree = resvg::usvg::Tree::from_str(text, &resvg::usvg::Options::default()).unwrap();
    let mut pixmap = resvg::tiny_skia::Pixmap::new(WIDTH, HEIGHT).unwrap();
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::identity(),
        &mut pixmap.as_mut(),
    );
    pixmap.pixels().iter().map(|p| p.alpha() >= 128).collect()
}

/// A second reader draws the elements on the pixels of the plain-path version (under 0.5 % of
/// the drawn pixels differ), including the rotated polygons and the star with a thin inner ratio.
#[test]
fn a_second_reader_draws_the_elements_as_it_draws_the_paths() {
    let (a, b) = (render(&elements_svg()), render(&paths_svg()));
    let drawn = a.iter().filter(|&&p| p).count();
    assert!(drawn > 10_000, "something is drawn: {drawn}");
    let differing = a.iter().zip(&b).filter(|(x, y)| x != y).count();
    let share = differing as f64 / drawn as f64;
    assert!(
        share < 0.005,
        "{differing} of {drawn} pixels differ ({share:.4})"
    );
}
