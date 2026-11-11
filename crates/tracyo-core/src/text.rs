//! Where a recognised text line sits in the file: baseline start, turn, size and width.

use kurbo::{BezPath, PathEl, Point, Vec2};
use vectrace::wire::Text;

/// The notice when the text reader could not be used (Curvyo's wording).
pub const TEXT_FAILED_NOTICE: &str = "Text recognition failed. Traced without text.";

/// How finely a glyph outline is flattened to find the line's extent, in millimetres.
const FLATTEN_TOLERANCE: f64 = 0.01;

/// A recognised line as the file places it: the string stretched over the original's width.
#[derive(Debug, Clone, PartialEq)]
pub struct TextPlace {
    /// The string.
    pub text: String,
    /// The baseline's start, millimetres.
    pub x: f64,
    /// The baseline's start, millimetres.
    pub y: f64,
    /// The turn of the line's direction, degrees (y down, clockwise positive).
    pub degrees: f64,
    /// The extent of the line's outlines across the line, millimetres.
    pub font_size: f64,
    /// The extent of the line's outlines along it, millimetres.
    pub length: f64,
}

/// The extent of `outlines` along `along` and across it (`across` is `along` turned a quarter
/// clockwise on a y-down page), as (along min, along max, across min, across max).
fn extent(outlines: &[BezPath], along: Vec2, across: Vec2) -> Option<[f64; 4]> {
    let mut bounds: Option<[f64; 4]> = None;
    for outline in outlines {
        kurbo::flatten(
            outline.elements().iter().copied(),
            FLATTEN_TOLERANCE,
            |el| {
                let (PathEl::MoveTo(point) | PathEl::LineTo(point)) = el else {
                    return;
                };
                let at = point.to_vec2();
                let (a, x) = (at.dot(along), at.dot(across));
                let b = bounds.get_or_insert([a, a, x, x]);
                *b = [b[0].min(a), b[1].max(a), b[2].min(x), b[3].max(x)];
            },
        );
    }
    bounds
}

/// The place of `line`: its start along the line's direction, its baseline at the far edge across
/// the line, its size the extent across and its length the extent along. `None` for a line
/// without outlines or with no extent.
#[must_use]
pub fn place(line: &Text) -> Option<TextPlace> {
    let length = line.direction.hypot();
    let along = if length > 0.0 {
        line.direction / length
    } else {
        Vec2::new(1.0, 0.0)
    };
    let across = Vec2::new(-along.y, along.x);
    let [u0, u1, v0, v1] = extent(&line.outlines, along, across)?;
    let (width, size) = (u1 - u0, v1 - v0);
    if !(width > 0.0 && size > 0.0) {
        return None;
    }
    let origin = Point::ZERO + along * u0 + across * v1;
    Some(TextPlace {
        text: line.text.clone(),
        x: origin.x,
        y: origin.y,
        degrees: along.y.atan2(along.x).to_degrees(),
        font_size: size,
        length: width,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use vectrace::Mm;

    fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> BezPath {
        let mut p = BezPath::new();
        p.move_to((x0, y0));
        p.line_to((x1, y0));
        p.line_to((x1, y1));
        p.line_to((x0, y1));
        p.close_path();
        p
    }

    fn line(direction: Vec2, outlines: Vec<BezPath>) -> Text {
        Text {
            text: "AB".to_owned(),
            direction,
            masked: 0,
            plausible: true,
            pixel: Mm::from_mm(0.26),
            positions: Vec::new(),
            fill: None,
            excluded: false,
            outlines,
        }
    }

    #[test]
    fn a_level_line_starts_at_its_left_and_sits_on_its_bottom_edge() {
        let t = line(
            Vec2::new(1.0, 0.0),
            vec![rect(10.0, 4.0, 14.0, 12.0), rect(20.0, 5.0, 50.0, 12.0)],
        );
        let p = place(&t).unwrap();
        assert!((p.x - 10.0).abs() < 1e-9 && (p.y - 12.0).abs() < 1e-9);
        assert!((p.length - 40.0).abs() < 1e-9 && (p.font_size - 8.0).abs() < 1e-9);
        assert!(p.degrees.abs() < 1e-9);
    }

    #[test]
    fn a_line_going_down_is_turned_a_quarter_with_its_glyph_bottom_on_the_left() {
        // Direction (0, 1): the text runs down the page, its glyphs' bottom is the left edge.
        let t = line(Vec2::new(0.0, 2.0), vec![rect(30.0, 10.0, 38.0, 50.0)]);
        let p = place(&t).unwrap();
        assert!((p.degrees - 90.0).abs() < 1e-9);
        assert!(
            (p.x - 30.0).abs() < 1e-9 && (p.y - 10.0).abs() < 1e-9,
            "{p:?}"
        );
        assert!((p.length - 40.0).abs() < 1e-9 && (p.font_size - 8.0).abs() < 1e-9);
    }

    #[test]
    fn a_line_without_outlines_has_no_place() {
        assert_eq!(place(&line(Vec2::new(1.0, 0.0), Vec::new())), None);
        assert_eq!(
            place(&line(Vec2::new(1.0, 0.0), vec![rect(1.0, 1.0, 1.0, 5.0)])),
            None
        );
    }
}
