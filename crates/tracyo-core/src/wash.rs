//! The part of the picture the active part does not trace, as triangles, for the extra wash of the
//! preview.
//!
//! The area is computed on an integer grid of 1/1024 px (rectangles and lassos, the lassos by
//! their non-zero winding area; whole pixels are exact, the overlay rounds the crossings of slanted
//! sides to 0.001 px), then cut into trapezoids between the horizontal lines
//! through its vertices and each trapezoid into two triangles, because egui paints meshes, not
//! filled outlines with holes.

use i_overlay::core::overlay_rule::OverlayRule;
use kurbo::Point;

use crate::grid::{self, Path64, Point64};
use crate::regions::{Form, MaskMode, PxRect, Region};

/// Grid units per picture pixel: a power of two, so whole pixels are exact.
const UNITS: i64 = 1024;

/// The triangles of `picture` that a trace of `regions` leaves out: outside the mask when the mode
/// is Inside, inside it when the mode is Outside. No region means no extra wash (the whole picture
/// is traced). The triangles do not overlap; the same input gives the same list.
#[must_use]
pub fn not_traced(regions: &[Region], mode: MaskMode, picture: PxRect) -> Vec<[Point; 3]> {
    if regions.is_empty() {
        return Vec::new();
    }
    let mut added: Vec<Path64> = Vec::new();
    let mut taken: Vec<Path64> = Vec::new();
    for region in regions {
        let ring: Path64 = region
            .corners()
            .iter()
            .map(|c| Point64::new(i64::from(c.x) * UNITS, i64::from(c.y) * UNITS))
            .collect();
        let rings = match region.form {
            Form::Rect(_) => vec![ring],
            Form::Lasso(_) => grid::run(OverlayRule::Subject, &[ring], &[]),
        };
        if region.subtract {
            taken.extend(rings);
        } else {
            added.extend(rings);
        }
    }
    let mask = if added.is_empty() {
        Vec::new()
    } else {
        grid::run(OverlayRule::Difference, &added, &taken)
    };
    let whole = grid::rect_path(
        i64::from(picture.x0) * UNITS,
        i64::from(picture.y0) * UNITS,
        i64::from(picture.x1) * UNITS,
        i64::from(picture.y1) * UNITS,
    );
    let area = match mode {
        MaskMode::Inside => grid::run(OverlayRule::Difference, &[whole], &mask),
        MaskMode::Outside if mask.is_empty() => Vec::new(),
        MaskMode::Outside => grid::run(OverlayRule::Intersect, &[whole], &mask),
    };
    triangles(&area)
}

/// One edge of the area that crosses the slab being cut: x at the slab's top and bottom.
struct Cross {
    middle: f64,
    top: f64,
    bottom: f64,
}

/// The area of `polygons` (outer outlines and holes that do not cross) as triangles.
fn triangles(polygons: &[Path64]) -> Vec<[Point; 3]> {
    let mut ys: Vec<i64> = polygons.iter().flatten().map(|p| p.y).collect();
    ys.sort_unstable();
    ys.dedup();
    let mut out = Vec::new();
    let mut crossing: Vec<Cross> = Vec::new();
    for slab in ys.windows(2) {
        let (top, bottom) = (slab[0], slab[1]);
        crossing.clear();
        for polygon in polygons {
            let Some(mut a) = polygon.last().copied() else {
                continue;
            };
            for &b in polygon {
                let (low, high) = (a.y.min(b.y), a.y.max(b.y));
                if low < high && low <= top && high >= bottom {
                    let at = |y: i64| edge_x(a, b, y);
                    crossing.push(Cross {
                        middle: f64::midpoint(at(top), at(bottom)),
                        top: at(top),
                        bottom: at(bottom),
                    });
                }
                a = b;
            }
        }
        crossing.sort_by(|p, q| p.middle.total_cmp(&q.middle));
        for pair in crossing.as_chunks::<2>().0 {
            let (left, right) = (&pair[0], &pair[1]);
            let (yt, yb) = (px(to_f64(top)), px(to_f64(bottom)));
            let corners = [
                Point::new(px(left.top), yt),
                Point::new(px(right.top), yt),
                Point::new(px(right.bottom), yb),
                Point::new(px(left.bottom), yb),
            ];
            if right.top > left.top || right.bottom > left.bottom {
                out.push([corners[0], corners[1], corners[2]]);
                out.push([corners[0], corners[2], corners[3]]);
            }
        }
    }
    out
}

/// x of the line through `a` and `b` (which differ in y) at height `y`.
fn edge_x(a: Point64, b: Point64, y: i64) -> f64 {
    if y == a.y {
        return to_f64(a.x);
    }
    if y == b.y {
        return to_f64(b.x);
    }
    let t = to_f64(y - a.y) / to_f64(b.y - a.y);
    to_f64(a.x) + t * to_f64(b.x - a.x)
}

fn to_f64(v: i64) -> f64 {
    // invariant: picture pixels stay far below 2^53 grid units.
    #[allow(clippy::cast_precision_loss)]
    let r = v as f64;
    r
}

/// A grid coordinate in picture pixels.
fn px(v: f64) -> f64 {
    v / 1024.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::regions::PxPoint;

    fn add(x0: i32, y0: i32, x1: i32, y1: i32) -> Region {
        Region::rect(PxRect::new(x0, y0, x1, y1), false)
    }

    fn take(x0: i32, y0: i32, x1: i32, y1: i32) -> Region {
        Region::rect(PxRect::new(x0, y0, x1, y1), true)
    }

    fn lasso(points: &[(i32, i32)], subtract: bool) -> Region {
        let corners: Vec<PxPoint> = points.iter().map(|&(x, y)| PxPoint::new(x, y)).collect();
        Region::lasso(&corners, subtract).unwrap()
    }

    fn area(triangles: &[[Point; 3]]) -> f64 {
        triangles
            .iter()
            .map(|[a, b, c]| ((b.x - a.x) * (c.y - a.y) - (c.x - a.x) * (b.y - a.y)).abs() / 2.0)
            .sum()
    }

    fn covers(triangles: &[[Point; 3]], p: Point) -> usize {
        let side = |a: Point, b: Point| (b.x - a.x) * (p.y - a.y) - (p.x - a.x) * (b.y - a.y);
        triangles
            .iter()
            .filter(|[a, b, c]| {
                let s = [side(*a, *b), side(*b, *c), side(*c, *a)];
                s.iter().all(|v| *v > 0.0) || s.iter().all(|v| *v < 0.0)
            })
            .count()
    }

    const PICTURE: PxRect = PxRect {
        x0: 0,
        y0: 0,
        x1: 100,
        y1: 80,
    };

    const EPS: f64 = 1e-9;

    #[test]
    fn without_regions_nothing_is_washed() {
        assert_eq!(not_traced(&[], MaskMode::Inside, PICTURE).len(), 0);
        assert_eq!(not_traced(&[], MaskMode::Outside, PICTURE).len(), 0);
    }

    #[test]
    fn inside_washes_what_is_around_the_mask_and_outside_what_is_in_it() {
        let regions = [add(10, 10, 30, 30)];
        let around = not_traced(&regions, MaskMode::Inside, PICTURE);
        let within = not_traced(&regions, MaskMode::Outside, PICTURE);
        assert!((area(&around) - f64::from(100 * 80 - 20 * 20)).abs() < EPS);
        assert!((area(&within) - 400.0).abs() < EPS);
        assert_eq!(covers(&around, Point::new(20.31, 20.67)), 0);
        assert_eq!(covers(&around, Point::new(5.31, 5.67)), 1);
        assert_eq!(covers(&within, Point::new(20.31, 20.67)), 1);
    }

    #[test]
    fn a_subtracted_region_is_a_hole_of_the_mask() {
        let regions = [add(0, 0, 50, 50), take(10, 10, 20, 20)];
        let within = not_traced(&regions, MaskMode::Outside, PICTURE);
        assert!((area(&within) - 2400.0).abs() < EPS);
        let around = not_traced(&regions, MaskMode::Inside, PICTURE);
        assert!((area(&around) + area(&within) - 8000.0).abs() < EPS);
        assert_eq!(covers(&within, Point::new(15.31, 15.67)), 0);
        assert_eq!(covers(&around, Point::new(15.31, 15.67)), 1);
    }

    #[test]
    fn regions_off_the_picture_are_cut_to_it() {
        let regions = [add(-50, -50, 40, 40), add(90, 70, 500, 500)];
        let within = not_traced(&regions, MaskMode::Outside, PICTURE);
        assert!((area(&within) - 1700.0).abs() < EPS);
        let off = [add(200, 0, 300, 10)];
        assert_eq!(not_traced(&off, MaskMode::Outside, PICTURE).len(), 0);
        let all = not_traced(&off, MaskMode::Inside, PICTURE);
        assert!(
            (area(&all) - 8000.0).abs() < EPS,
            "an empty mask washes everything with Inside"
        );
    }

    #[test]
    fn only_subtracted_regions_make_an_empty_mask() {
        let only = [take(10, 10, 20, 20)];
        let all = not_traced(&only, MaskMode::Inside, PICTURE);
        assert!((area(&all) - 8000.0).abs() < EPS);
        assert_eq!(not_traced(&only, MaskMode::Outside, PICTURE).len(), 0);
    }

    #[test]
    fn overlapping_regions_count_once_and_the_order_does_not_matter() {
        let a = [add(0, 0, 40, 40), add(20, 20, 60, 60), take(30, 30, 35, 35)];
        let b = [a[2].clone(), a[1].clone(), a[0].clone()];
        let one = not_traced(&a, MaskMode::Outside, PICTURE);
        let two = not_traced(&b, MaskMode::Outside, PICTURE);
        assert!((area(&one) - (1600.0 + 1600.0 - 400.0 - 25.0)).abs() < EPS);
        assert!((area(&one) - area(&two)).abs() < EPS);
    }

    #[test]
    fn a_slanted_lasso_side_is_cut_into_trapezoids_of_the_right_area() {
        // The triangle (0,0), (60,0), (0,60): area 1800 inside, 8000 - 1800 around.
        let regions = [lasso(&[(0, 0), (60, 0), (0, 60)], false)];
        let within = not_traced(&regions, MaskMode::Outside, PICTURE);
        let around = not_traced(&regions, MaskMode::Inside, PICTURE);
        assert!((area(&within) - 1800.0).abs() < 1e-6);
        assert!((area(&around) - 6200.0).abs() < 1e-6);
        assert_eq!(covers(&within, Point::new(10.31, 10.67)), 1);
        assert_eq!(covers(&within, Point::new(40.31, 40.67)), 0);
        assert_eq!(covers(&around, Point::new(40.31, 40.67)), 1);
        assert_eq!(covers(&around, Point::new(10.31, 10.67)), 0);
    }

    #[test]
    fn a_figure_eight_is_filled_wherever_it_is_covered() {
        // Two triangles meeting at (50, 40), wound against each other.
        let eight = [lasso(&[(20, 10), (80, 70), (80, 10), (20, 70)], false)];
        let within = not_traced(&eight, MaskMode::Outside, PICTURE);
        // Each lobe is a triangle with base 60 and height 30: 900 each.
        assert!((area(&within) - 1800.0).abs() < 1e-6);
        assert_eq!(covers(&within, Point::new(70.31, 40.67)), 1);
        assert_eq!(covers(&within, Point::new(30.31, 40.67)), 1);
        assert_eq!(covers(&within, Point::new(50.31, 20.67)), 0);
    }

    #[test]
    fn a_lasso_and_a_rectangle_overlapping_count_once_whatever_their_winding() {
        let a = lasso(&[(0, 0), (60, 0), (0, 60)], false);
        let b = lasso(&[(0, 0), (0, 60), (60, 0)], false);
        let r = add(10, 10, 30, 30);
        let one = not_traced(&[a.clone(), r.clone()], MaskMode::Outside, PICTURE);
        let two = not_traced(&[b, r], MaskMode::Outside, PICTURE);
        assert!((area(&one) - 1800.0).abs() < 1e-6);
        assert!((area(&two) - 1800.0).abs() < 1e-6);
        assert_eq!(covers(&one, Point::new(15.31, 15.67)), 1);
    }

    #[test]
    fn many_shapes_cover_the_picture_once_between_inside_and_outside() {
        let mut state = 12_345_u64;
        let mut next = |n: i32| {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1);
            i32::try_from((state >> 33) % u64::try_from(n).unwrap()).unwrap()
        };
        let regions: Vec<Region> = (0..40)
            .map(|i| {
                if i % 3 == 0 {
                    let pts: Vec<(i32, i32)> =
                        (0..5).map(|_| (next(140) - 20, next(120) - 20)).collect();
                    let corners: Vec<PxPoint> =
                        pts.iter().map(|&(x, y)| PxPoint::new(x, y)).collect();
                    Region::lasso(&corners, i % 4 == 3).unwrap_or_else(|| add(0, 0, 1, 1))
                } else {
                    Region::rect(
                        PxRect::new(
                            next(140) - 20,
                            next(120) - 20,
                            next(140) - 20,
                            next(120) - 20,
                        ),
                        i % 4 == 3,
                    )
                }
            })
            .collect();
        let inside = not_traced(&regions, MaskMode::Inside, PICTURE);
        let outside = not_traced(&regions, MaskMode::Outside, PICTURE);
        assert!((area(&inside) + area(&outside) - 8000.0).abs() < 0.05);
        for y in 0..80 {
            for x in 0..100 {
                let p = Point::new(f64::from(x) + 0.31, f64::from(y) + 0.67);
                assert_eq!(covers(&inside, p) + covers(&outside, p), 1, "{x} {y}");
            }
        }
    }
}
