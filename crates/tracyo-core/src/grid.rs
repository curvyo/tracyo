//! The 0.001 mm integer grid the cut works on: snapping, the overlay calls and the canonical order
//! of the result. Everything between the snap and the conversion back to millimetres is integer
//! arithmetic, so the result is the same on every target.

use i_overlay::core::fill_rule::FillRule;
use i_overlay::core::overlay::{Overlay, ShapeType};
use i_overlay::core::overlay_rule::OverlayRule;
use i_overlay::i_float::int::point::IntPoint;
use kurbo::{BezPath, PathEl};

/// A vertex on the grid, in grid units.
pub type Point64 = IntPoint<i64>;
/// A closed polygon on the grid.
pub type Path64 = Vec<Point64>;

/// Grid units per millimetre: a power of ten that is exact as a float.
pub const UNITS_PER_MM: f64 = 1000.0;
/// The largest absolute coordinate the cut accepts, in millimetres (10 km); far inside the
/// 64-bit engine's range.
pub const MAX_COORDINATE_MM: f64 = 1.0e7;
/// The distance at which traced curves are flattened before they are cut, in millimetres. With
/// the grid's half diagonal (0.0007 mm) it stays within the 0.005 mm of criterion 13.
pub const FLATTEN_MM: f64 = 0.003;

/// The grid point nearest to (`x`, `y`) millimetres; `None` when a coordinate is not finite or
/// beyond [`MAX_COORDINATE_MM`].
#[must_use]
pub fn snap(x: f64, y: f64) -> Option<Point64> {
    let ok = |v: f64| v.is_finite() && v.abs() <= MAX_COORDINATE_MM;
    // invariant: checked finite and within 10^7, so the products are exact enough to cast.
    #[allow(clippy::cast_possible_truncation)]
    (ok(x) && ok(y)).then(|| {
        Point64::new(
            (x * UNITS_PER_MM).round() as i64,
            (y * UNITS_PER_MM).round() as i64,
        )
    })
}

/// A grid coordinate in millimetres.
#[must_use]
pub fn to_mm(units: i64) -> f64 {
    // invariant: grid units stay far below 2^53.
    #[allow(clippy::cast_precision_loss)]
    let v = units as f64;
    v / UNITS_PER_MM
}

/// Twice the signed area of a grid polygon in square grid units (shoelace; exact in 128 bits);
/// positive for the winding the overlay gives outer outlines.
#[must_use]
pub fn double_area(path: &[Point64]) -> i128 {
    let mut sum: i128 = 0;
    let Some(mut previous) = path.last().copied() else {
        return 0;
    };
    for point in path {
        sum += i128::from(previous.x) * i128::from(point.y)
            - i128::from(point.x) * i128::from(previous.y);
        previous = *point;
    }
    sum
}

/// The area of `paths` in square millimetres, holes (negative winding) subtracted.
#[must_use]
pub fn area_mm2(paths: &[Path64]) -> f64 {
    let sum: i128 = paths.iter().map(|p| double_area(p)).sum();
    // invariant: areas of a picture are far below 2^53 square units.
    #[allow(clippy::cast_precision_loss)]
    let v = sum as f64;
    v / 2.0 / (UNITS_PER_MM * UNITS_PER_MM)
}

/// Snaps a polygon of millimetre points and drops consecutive duplicates, the wrap-around pair
/// included.
fn snap_ring(points: &[kurbo::Point]) -> Option<Path64> {
    let mut path: Path64 = Vec::with_capacity(points.len());
    for p in points {
        let snapped = snap(p.x, p.y)?;
        if path.last() != Some(&snapped) {
            path.push(snapped);
        }
    }
    while path.len() > 1 && path.first() == path.last() {
        path.pop();
    }
    Some(path)
}

/// The closed subpaths of `path` flattened at [`FLATTEN_MM`] and snapped to the grid; rings of
/// fewer than three points are dropped. `None` when a coordinate is not finite or out of range.
#[must_use]
pub fn flatten_snap(path: &BezPath) -> Option<Vec<Path64>> {
    let mut rings: Vec<Vec<kurbo::Point>> = Vec::new();
    kurbo::flatten(path.elements().iter().copied(), FLATTEN_MM, |el| match el {
        PathEl::MoveTo(p) => rings.push(vec![p]),
        PathEl::LineTo(p) => {
            if let Some(ring) = rings.last_mut() {
                ring.push(p);
            }
        }
        _ => {}
    });
    let mut out = Vec::with_capacity(rings.len());
    for ring in &rings {
        let snapped = snap_ring(ring)?;
        if snapped.len() >= 3 {
            out.push(snapped);
        }
    }
    Some(out)
}

/// One overlay with the nonzero fill rule on both inputs. Outer outlines come back with positive
/// area, holes with negative.
#[must_use]
pub fn run(rule: OverlayRule, subjects: &[Path64], clips: &[Path64]) -> Vec<Path64> {
    let capacity = subjects.iter().chain(clips).map(Vec::len).sum();
    let mut overlay = Overlay::<i64>::new(capacity);
    for contour in subjects {
        overlay.add_contour(contour, ShapeType::Subject);
    }
    for contour in clips {
        overlay.add_contour(contour, ShapeType::Clip);
    }
    overlay
        .overlay(rule, FillRule::NonZero)
        .into_iter()
        .flatten()
        .collect()
}

/// The axis-parallel rectangle (`x0`, `y0`) to (`x1`, `y1`) in grid units, wound as an outer
/// outline.
#[must_use]
pub fn rect_path(x0: i64, y0: i64, x1: i64, y1: i64) -> Path64 {
    vec![
        Point64::new(x0, y0),
        Point64::new(x1, y0),
        Point64::new(x1, y1),
        Point64::new(x0, y1),
    ]
}

/// `path` rotated to start at its smallest vertex (x, then y); when that vertex occurs twice the
/// rotation whose vertex sequence compares smallest wins.
fn rotate_to_start(path: &[Point64]) -> Path64 {
    let key = |p: &Point64| (p.x, p.y);
    let n = path.len();
    let Some(smallest) = path.iter().map(key).min() else {
        return Vec::new();
    };
    let mut best: Option<usize> = None;
    for (i, p) in path.iter().enumerate() {
        if key(p) != smallest {
            continue;
        }
        best = match best {
            None => Some(i),
            Some(b) => {
                let candidate = (0..n).map(|k| key(&path[(i + k) % n]));
                let incumbent = (0..n).map(|k| key(&path[(b + k) % n]));
                Some(if candidate.lt(incumbent) { i } else { b })
            }
        };
    }
    let start = best.unwrap_or(0);
    let mut out = Path64::with_capacity(n);
    out.extend_from_slice(&path[start..]);
    out.extend_from_slice(&path[..start]);
    out
}

/// The canonical form of an overlay result: each outline starts at its smallest vertex, and the
/// outlines are ordered by that vertex, then by area (larger first), then by their vertices, so
/// the same area always gives the same bytes.
#[must_use]
pub fn canonical(paths: Vec<Path64>) -> Vec<Path64> {
    let mut keyed: Vec<(i128, Path64)> = paths
        .into_iter()
        .filter(|p| p.len() >= 3)
        .map(|p| (double_area(&p), rotate_to_start(&p)))
        .collect();
    keyed.sort_by(|(area_a, a), (area_b, b)| {
        let start = |p: &Path64| (p[0].x, p[0].y);
        start(a)
            .cmp(&start(b))
            .then(area_b.cmp(area_a))
            .then_with(|| {
                a.iter()
                    .map(|p| (p.x, p.y))
                    .cmp(b.iter().map(|p| (p.x, p.y)))
            })
    });
    keyed.into_iter().map(|(_, p)| p).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapping_rounds_to_the_grid_and_refuses_what_it_cannot_hold() {
        assert_eq!(snap(1.0004, -2.0006), Some(Point64::new(1000, -2001)));
        assert_eq!(snap(f64::NAN, 0.0), None);
        assert_eq!(snap(0.0, 2.0e7), None);
    }

    #[test]
    fn a_rectangle_has_its_area_and_a_hole_takes_it_away() {
        let outer = rect_path(0, 0, 10_000, 4_000);
        assert!((area_mm2(std::slice::from_ref(&outer)) - 40.0).abs() < 1e-9);
        let mut hole = rect_path(1_000, 1_000, 2_000, 2_000);
        hole.reverse();
        assert!((area_mm2(&[outer, hole]) - 39.0).abs() < 1e-9);
    }

    #[test]
    fn the_canonical_order_ignores_the_start_vertex_and_the_order_of_the_outlines() {
        let a = rect_path(0, 0, 10, 10);
        let mut rotated = a.clone();
        rotated.rotate_left(2);
        let b = rect_path(20, 0, 30, 10);
        assert_eq!(canonical(vec![b.clone(), rotated]), canonical(vec![a, b]));
    }

    #[test]
    fn a_union_of_touching_rectangles_is_one_outline() {
        let left = rect_path(0, 0, 10, 10);
        let right = rect_path(10, 0, 20, 10);
        let union = run(OverlayRule::Subject, &[left, right], &[]);
        assert_eq!(union.len(), 1);
        assert_eq!(double_area(&union[0]), 2 * 200);
    }

    #[test]
    fn a_curve_is_flattened_within_the_tolerance() {
        let mut path = BezPath::new();
        path.move_to((0.0, 0.0));
        path.curve_to((0.0, 5.0), (10.0, 5.0), (10.0, 0.0));
        path.close_path();
        let rings = flatten_snap(&path).unwrap();
        assert_eq!(rings.len(), 1);
        let exact = kurbo::Shape::area(&path).abs();
        assert!((area_mm2(&rings) - exact).abs() < 0.01 * exact);
    }
}
