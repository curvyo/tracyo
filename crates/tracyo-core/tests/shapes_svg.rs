//! A picture of recognisable shapes traced with Recognise shapes and written as SVG: the golden
//! file, the element structure and a second reader's rendering.

#![allow(
    clippy::unwrap_used,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]

use tracyo_core::job::Session;
use tracyo_core::picture::Picture;
use tracyo_core::request::TraceRequest;
use tracyo_core::settings::Settings;
use tracyo_core::svg;

const WIDTH: u32 = 360;
const HEIGHT: u32 = 190;

/// Whether the point (x, y) is inside the convex or star polygon `corners` (even-odd).
fn inside(corners: &[(f64, f64)], x: f64, y: f64) -> bool {
    let mut in_poly = false;
    let mut j = corners.len() - 1;
    for i in 0..corners.len() {
        let (xi, yi) = corners[i];
        let (xj, yj) = corners[j];
        if (yi > y) != (yj > y) && x < (xj - xi) * (y - yi) / (yj - yi) + xi {
            in_poly = !in_poly;
        }
        j = i;
    }
    in_poly
}

fn ngon(cx: f64, cy: f64, r: f64, n: u32, inner: Option<f64>) -> Vec<(f64, f64)> {
    let mut out = Vec::new();
    for k in 0..n {
        let a = -std::f64::consts::FRAC_PI_2 + std::f64::consts::TAU * f64::from(k) / f64::from(n);
        out.push((cx + r * a.cos(), cy + r * a.sin()));
        if let Some(ratio) = inner {
            let b = a + std::f64::consts::PI / f64::from(n);
            out.push((cx + r * ratio * b.cos(), cy + r * ratio * b.sin()));
        }
    }
    out
}

fn rotated_rect(cx: f64, cy: f64, w: f64, h: f64, degrees: f64) -> Vec<(f64, f64)> {
    let (s, c) = degrees.to_radians().sin_cos();
    [(-w, -h), (w, -h), (w, h), (-w, h)]
        .iter()
        .map(|&(x, y)| (cx + (x * c - y * s) * 0.5, cy + f64::midpoint(x * s, y * c)))
        .collect()
}

/// The picture: a rectangle, a circle, an ellipse, a hexagon, a five-pointed star, a ring, a
/// rounded rectangle with an off-centre hole and a rotated rectangle, black on white.
fn picture() -> Picture {
    let hexagon = ngon(250.0, 40.0, 34.0, 6, None);
    let triangle = ngon(320.0, 40.0, 34.0, 3, None);
    let star = ngon(40.0, 130.0, 32.0, 5, Some(0.45));
    let tilted = rotated_rect(300.0, 150.0, 44.0, 16.0, 30.0);
    let black = |x: f64, y: f64| {
        let in_rect = |x0: f64, y0: f64, x1: f64, y1: f64| x >= x0 && x < x1 && y >= y0 && y < y1;
        let dist = |cx: f64, cy: f64| ((x - cx).powi(2) + (y - cy).powi(2)).sqrt();
        let rounded = {
            let (x0, y0, x1, y1, r) = (160.0, 100.0, 240.0, 160.0, 10.0);
            let (qx, qy) = (x.clamp(x0 + r, x1 - r), y.clamp(y0 + r, y1 - r));
            in_rect(x0, y0, x1, y1) && ((x - qx).powi(2) + (y - qy).powi(2)).sqrt() <= r
        };
        in_rect(10.0, 10.0, 70.0, 50.0)
            || dist(110.0, 30.0) <= 20.0
            || ((x - 180.0) / 30.0).powi(2) + ((y - 30.0) / 15.0).powi(2) <= 1.0
            || inside(&hexagon, x, y)
            || inside(&triangle, x, y)
            || inside(&star, x, y)
            || (dist(110.0, 130.0) <= 32.0 && dist(110.0, 130.0) >= 24.0)
            || (rounded && !in_rect(170.0, 112.0, 225.0, 148.0))
            || inside(&tilted, x, y)
    };
    let mut rgba = vec![255_u8; (WIDTH * HEIGHT * 4) as usize];
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            // 4 x 4 samples per pixel for clean edges.
            let mut hits = 0;
            for sy in 0..4 {
                for sx in 0..4 {
                    if black(
                        f64::from(x) + (f64::from(sx) + 0.5) / 4.0,
                        f64::from(y) + (f64::from(sy) + 0.5) / 4.0,
                    ) {
                        hits += 1;
                    }
                }
            }
            let v = 255 - (255 * hits / 16) as u8;
            let i = ((y * WIDTH + x) * 4) as usize;
            rgba[i..i + 3].fill(v);
        }
    }
    Picture {
        width: WIDTH,
        height: HEIGHT,
        rgba,
    }
}

fn trace(shapes: bool) -> (Picture, vectrace::wire::Output) {
    let picture = picture();
    let mut session = Session::new(picture.clone());
    let request = TraceRequest {
        threshold: 128,
        settings: Settings::default(),
        selection: vectrace::Selection::Brightness,
        invert: false,
        drop_border: false,
        colours: None,
        reduced: false,
        shapes,
        lines: Vec::new(),
    };
    let output = vectrace::wire::read(&session.run(&request, &mut |_| {})).unwrap();
    (picture, output)
}

fn svg_of(picture: &Picture, output: &vectrace::wire::Output) -> String {
    svg::write(&svg::Document {
        width_px: picture.width,
        height_px: picture.height,
        layers: &svg::layers_of(output),
    })
}

fn element_names(doc: &roxmltree::Document<'_>) -> Vec<String> {
    doc.descendants()
        .filter(roxmltree::Node::is_element)
        .map(|n| n.tag_name().name().to_owned())
        .collect()
}

fn count(names: &[String], name: &str) -> usize {
    names.iter().filter(|n| n.as_str() == name).count()
}

#[test]
fn the_shapes_match_their_golden_file() {
    let (picture, output) = trace(true);
    let text = svg_of(&picture, &output);
    let golden = format!(
        "{}/tests/fixtures/svg/recognised-shapes.svg",
        env!("CARGO_MANIFEST_DIR")
    );
    if std::env::var_os("TRACYO_UPDATE_GOLDEN").is_some() {
        std::fs::write(&golden, &text).unwrap();
    }
    assert_eq!(text, std::fs::read_to_string(golden).unwrap());
}

/// Criterion 20f: rectangles, circles and ellipses are primitives, a polygon is a `<polygon>`, a star
/// is Inkscape's star object, a band is a stroked primitive with no fill, and nothing is written twice.
#[test]
fn recognised_shapes_are_primitives_with_the_elements_of_criterion_20f() {
    let (picture, output) = trace(true);
    let text = svg_of(&picture, &output);
    let doc = roxmltree::Document::parse(&text).unwrap();
    let names = element_names(&doc);
    assert!(
        count(&names, "rect") >= 2,
        "the rectangle and the tilted one"
    );
    assert!(
        count(&names, "circle") >= 2,
        "the circle and the ring's centre line"
    );
    assert!(count(&names, "ellipse") >= 1);
    assert!(
        count(&names, "polygon") >= 2,
        "the hexagon and the triangle"
    );
    assert_eq!(
        text.matches("sodipodi:type=\"star\"").count(),
        1,
        "the star is Inkscape's star object"
    );

    let rect = doc.descendants().find(|n| n.has_tag_name("rect")).unwrap();
    for attribute in ["x", "y", "width", "height", "rx", "ry"] {
        assert!(rect.attribute(attribute).is_some(), "rect {attribute}");
    }
    let tilted = doc
        .descendants()
        .find(|n| n.has_tag_name("rect") && n.attribute("transform").is_some())
        .unwrap();
    let transform = tilted.attribute("transform").unwrap();
    assert!(
        transform.starts_with("rotate(29.") && transform.ends_with(')'),
        "{transform}"
    );
    assert_eq!(transform.split_whitespace().count(), 3, "rotate(deg cx cy)");

    // The ring is a circle along its middle: no fill, a stroke as wide as the ring.
    let ring = doc
        .descendants()
        .find(|n| n.has_tag_name("circle") && n.attribute("fill") == Some("none"))
        .unwrap();
    assert_eq!(ring.attribute("stroke"), Some("#000000"));
    let width: f64 = ring.attribute("stroke-width").unwrap().parse().unwrap();
    // The ring is 8 px wide.
    assert!(
        (width - 8.0 * 25.4 / 96.0).abs() < 0.3,
        "stroke-width {width}"
    );
    assert!(ring.attribute("stroke-linejoin").is_some());

    // Ids are unique over the whole file.
    let ids: Vec<_> = doc
        .descendants()
        .filter_map(|n| n.attribute("id"))
        .collect();
    let mut sorted = ids.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(ids.len(), sorted.len(), "unique ids: {ids:?}");

    // A recognised shape is not also in the layer's compound path: every compound path holds
    // fewer outlines than the result has.
    let compound = doc
        .descendants()
        .find(|n| n.has_tag_name("path") && n.attribute("d").is_some_and(|d| d.contains('C')))
        .unwrap();
    assert_eq!(
        compound.attribute("d").unwrap().matches('Z').count(),
        2,
        "the frame and its hole"
    );
}

#[test]
fn without_recognise_shapes_the_file_is_one_compound_path() {
    let (picture, output) = trace(false);
    let text = svg_of(&picture, &output);
    let doc = roxmltree::Document::parse(&text).unwrap();
    let names = element_names(&doc);
    for primitive in ["rect", "circle", "ellipse", "polygon"] {
        assert_eq!(count(&names, primitive), 0, "{primitive}");
    }
    assert_eq!(count(&names, "path"), 1);
}

/// The share of the dark pixels of `picture` in the box `[x0, x1) x [y0, y1)` that the second
/// reader's drawing of `text` gets wrong (drawn where the picture is light, or the other way).
fn mismatch(picture: &Picture, text: &str, bounds: (u32, u32, u32, u32)) -> f64 {
    let tree = resvg::usvg::Tree::from_str(text, &resvg::usvg::Options::default()).unwrap();
    let mut pixmap = resvg::tiny_skia::Pixmap::new(picture.width, picture.height).unwrap();
    // The file is 96 dpi: its size in pixels is the picture's.
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::identity(),
        &mut pixmap.as_mut(),
    );
    let (x0, y0, x1, y1) = bounds;
    let (mut black, mut wrong) = (0_u32, 0_u32);
    for y in y0..y1 {
        for x in x0..x1 {
            let i = (y * picture.width + x) as usize;
            let drawn = pixmap.pixels()[i].alpha() >= 128;
            let dark = picture.rgba[i * 4] < 128;
            black += u32::from(dark);
            wrong += u32::from(drawn != dark);
        }
    }
    f64::from(wrong) / f64::from(black)
}

/// The second reader draws what the picture shows: the recognised file differs from the dark
/// pixels of the picture by edge pixels only (the shapes lie within the tolerance of 2 px).
#[test]
fn a_second_reader_draws_the_shapes_where_the_picture_has_them() {
    let (picture, output) = trace(true);
    let text = svg_of(&picture, &output);
    let whole = mismatch(&picture, &text, (0, 0, WIDTH, HEIGHT));
    assert!(whole < 0.03, "{whole:.4}");
}

/// A turn in the wrong direction would draw the tilted rectangle 60 degrees off; the box around
/// it alone shows it.
#[test]
fn the_tilted_rectangle_is_turned_the_right_way() {
    let (picture, output) = trace(true);
    let text = svg_of(&picture, &output);
    assert!(
        text.contains("rotate(29."),
        "the tilted rectangle is recognised"
    );
    let tilted = mismatch(&picture, &text, (270, 115, 335, 185));
    assert!(tilted < 0.10, "{tilted:.4}");
}

/// A band stroked in the file covers what the traced ring covers.
#[test]
fn the_ring_drawn_as_a_stroked_circle_covers_the_traced_ring() {
    let (picture, output) = trace(true);
    let text = svg_of(&picture, &output);
    let ring = mismatch(&picture, &text, (70, 90, 150, 170));
    assert!(ring < 0.06, "{ring:.4}");
}
