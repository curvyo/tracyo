//! The geometry of drawing, finding, moving and resizing a rectangle region, in picture pixels.

use kurbo::{Point, Rect};

use crate::regions::{Form, PxRect, Region};

/// A resize handle of a selected rectangle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Handle {
    /// Top edge.
    N,
    /// Top-right corner.
    NE,
    /// Right edge.
    E,
    /// Bottom-right corner.
    SE,
    /// Bottom edge.
    S,
    /// Bottom-left corner.
    SW,
    /// Left edge.
    W,
    /// Top-left corner.
    NW,
}

impl Handle {
    /// The eight handles, corners and edges, clockwise from the top-left corner.
    pub const ALL: [Self; 8] = [
        Self::NW,
        Self::N,
        Self::NE,
        Self::E,
        Self::SE,
        Self::S,
        Self::SW,
        Self::W,
    ];

    /// The handle's direction: -1, 0 or 1 along x and along y.
    #[must_use]
    pub const fn direction(self) -> (i32, i32) {
        match self {
            Self::N => (0, -1),
            Self::NE => (1, -1),
            Self::E => (1, 0),
            Self::SE => (1, 1),
            Self::S => (0, 1),
            Self::SW => (-1, 1),
            Self::W => (-1, 0),
            Self::NW => (-1, -1),
        }
    }

    /// Whether the handle sits on a corner.
    #[must_use]
    pub const fn is_corner(self) -> bool {
        let (x, y) = self.direction();
        x != 0 && y != 0
    }

    /// Where the handle sits on `rect`.
    #[must_use]
    pub fn position(self, rect: Rect) -> Point {
        let (dx, dy) = self.direction();
        let pick = |d: i32, lo: f64, hi: f64| match d {
            -1 => lo,
            1 => hi,
            _ => f64::midpoint(lo, hi),
        };
        Point::new(pick(dx, rect.x0, rect.x1), pick(dy, rect.y0, rect.y1))
    }
}

/// The picture pixel nearest to `v`, held to a range that cannot overflow later sums.
fn px(v: f64) -> i32 {
    const LIMIT: f64 = 1.0e9;
    // invariant: clamped into i32's range and rounded above, a NaN becomes 0.
    #[allow(clippy::cast_possible_truncation)]
    let r = v.round().clamp(-LIMIT, LIMIT) as i32;
    r
}

/// The rectangle drawn by a drag from `from` to `to` (picture pixels): corner to corner, from the
/// centre when `centre`, a square when `square`. A drag shorter than 2 px in both directions draws
/// nothing; corners land on whole pixels and a side is at least 1 px.
#[must_use]
pub fn drawn(from: Point, to: Point, centre: bool, square: bool) -> Option<PxRect> {
    let (mut dx, mut dy) = (to.x - from.x, to.y - from.y);
    if dx.abs().max(dy.abs()) < 2.0 {
        return None;
    }
    if square {
        let side = dx.abs().max(dy.abs());
        dx = side.copysign(if dx < 0.0 { -1.0 } else { 1.0 });
        dy = side.copysign(if dy < 0.0 { -1.0 } else { 1.0 });
    }
    let (ax, ay) = (px(from.x), px(from.y));
    if centre {
        let (hx, hy) = if square {
            let half = px(dx.abs());
            (half, half)
        } else {
            (px(dx.abs()), px(dy.abs()))
        };
        return Some(PxRect::new(
            ax.saturating_sub(hx),
            ay.saturating_sub(hy),
            ax.saturating_add(hx),
            ay.saturating_add(hy),
        ));
    }
    if square {
        let side = px(dx.abs());
        let sx = if dx < 0.0 { -side } else { side };
        let sy = if dy < 0.0 { -side } else { side };
        return Some(PxRect::new(
            ax,
            ay,
            ax.saturating_add(sx),
            ay.saturating_add(sy),
        ));
    }
    Some(PxRect::new(ax, ay, px(to.x), px(to.y)))
}

/// The topmost (last) region that holds `p`: a rectangle grown by `slop_px`, a lasso by its
/// non-zero winding area or within `slop_px` of a side.
#[must_use]
pub fn region_at(regions: &[Region], p: Point, slop_px: f64) -> Option<usize> {
    regions.iter().rposition(|r| match &r.form {
        Form::Rect(rect) => rect.to_kurbo().inflate(slop_px, slop_px).contains(p),
        Form::Lasso(points) => {
            let ring: Vec<Point> = points.iter().map(|c| c.to_kurbo()).collect();
            winding(&ring, p) != 0 || near_a_side(&ring, p, slop_px)
        }
    })
}

/// The winding number of the closed polygon `ring` round `p`.
#[must_use]
pub fn winding(ring: &[Point], p: Point) -> i32 {
    let Some(mut a) = ring.last().copied() else {
        return 0;
    };
    let mut winding = 0;
    for &b in ring {
        let side = (b.x - a.x) * (p.y - a.y) - (p.x - a.x) * (b.y - a.y);
        if a.y <= p.y {
            if b.y > p.y && side > 0.0 {
                winding += 1;
            }
        } else if b.y <= p.y && side < 0.0 {
            winding -= 1;
        }
        a = b;
    }
    winding
}

/// Whether `p` is within `distance` of a side of the closed polygon `ring`.
fn near_a_side(ring: &[Point], p: Point, distance: f64) -> bool {
    let Some(mut a) = ring.last().copied() else {
        return false;
    };
    for &b in ring {
        let (dx, dy) = (b.x - a.x, b.y - a.y);
        let len2 = dx * dx + dy * dy;
        let t = if len2 > 0.0 {
            (((p.x - a.x) * dx + (p.y - a.y) * dy) / len2).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let nearest = Point::new(a.x + t * dx, a.y + t * dy);
        if nearest.distance(p) <= distance {
            return true;
        }
        a = b;
    }
    false
}

/// The corner of a selected lasso (`corners` in screen pixels) the pointer is within `radius` of;
/// the nearest centre wins.
#[must_use]
pub fn corner_at(corners: &[Point], pointer: Point, radius: f64) -> Option<usize> {
    corners
        .iter()
        .enumerate()
        .map(|(i, c)| (i, c.distance(pointer)))
        .filter(|(_, d)| *d <= radius)
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(i, _)| i)
}

/// The picture pixel nearest to `p`, for a lasso corner.
#[must_use]
pub fn nearest_pixel(p: Point) -> crate::regions::PxPoint {
    crate::regions::PxPoint::new(px(p.x), px(p.y))
}

/// The handle of the selected rectangle `rect` (screen pixels) the pointer is on: corners always,
/// edge handles only while the shorter side is 24 px or more; the hit radius of a handle is a third
/// of the shorter side, between 4 and 16 px; the nearest centre wins.
#[must_use]
pub fn handle_at(rect: Rect, pointer: Point) -> Option<Handle> {
    let short = rect.width().min(rect.height());
    let radius = (short / 3.0).clamp(4.0, 16.0);
    Handle::ALL
        .into_iter()
        .filter(|h| h.is_corner() || short >= 24.0)
        .map(|h| (h, h.position(rect).distance(pointer)))
        .filter(|(_, d)| *d <= radius)
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(h, _)| h)
}

/// `rect` with `handle` dragged to `to` (picture pixels): the opposite side stays, or with `centre`
/// the opposite side mirrors about the centre; with `keep_ratio` a corner handle keeps the
/// proportions. A side never gets shorter than 1 px.
#[must_use]
pub fn resized(rect: PxRect, handle: Handle, to: Point, centre: bool, keep_ratio: bool) -> PxRect {
    let (hx, hy) = handle.direction();
    let (w, h) = (f64::from(rect.width()), f64::from(rect.height()));
    let (mut tx, mut ty) = (to.x, to.y);
    if keep_ratio && handle.is_corner() {
        let cx = f64::midpoint(f64::from(rect.x0), f64::from(rect.x1));
        let cy = f64::midpoint(f64::from(rect.y0), f64::from(rect.y1));
        let (anchor_x, anchor_y, full_w, full_h) = if centre {
            (cx, cy, w / 2.0, h / 2.0)
        } else {
            (
                f64::from(if hx > 0 { rect.x0 } else { rect.x1 }),
                f64::from(if hy > 0 { rect.y0 } else { rect.y1 }),
                w,
                h,
            )
        };
        let scale = ((to.x - anchor_x).abs() / full_w).max((to.y - anchor_y).abs() / full_h);
        tx = anchor_x + f64::from(hx) * full_w * scale;
        ty = anchor_y + f64::from(hy) * full_h * scale;
    }
    let (x0, x1) = side(rect.x0, rect.x1, hx, px(tx), centre);
    let (y0, y1) = side(rect.y0, rect.y1, hy, px(ty), centre);
    PxRect { x0, y0, x1, y1 }
}

/// One axis of a resize: the `lo`..`hi` extent with the handle's direction `d` (-1 moves `lo`, 1
/// moves `hi`, 0 neither) dragged to `t`.
fn side(lo: i32, hi: i32, d: i32, t: i32, centre: bool) -> (i32, i32) {
    match d {
        0 => (lo, hi),
        _ if centre => {
            let sum = i64::from(lo) + i64::from(hi);
            // The dragged edge, mirrored: at least one pixel of width remains.
            let moved = i64::from(if d > 0 {
                t
            } else {
                sum_to_i32(sum - i64::from(t))
            });
            let high = moved.max((sum + 2).div_euclid(2));
            (sum_to_i32(sum - high), sum_to_i32(high))
        }
        1 => (lo, t.max(lo.saturating_add(1))),
        _ => (t.min(hi.saturating_sub(1)), hi),
    }
}

fn sum_to_i32(v: i64) -> i32 {
    i32::try_from(v).unwrap_or(if v < 0 { i32::MIN } else { i32::MAX })
}

/// `rect` moved by (`dx`, `dy`) pixels.
#[must_use]
pub fn moved(rect: PxRect, dx: i32, dy: i32) -> PxRect {
    PxRect {
        x0: rect.x0.saturating_add(dx),
        y0: rect.y0.saturating_add(dy),
        x1: rect.x1.saturating_add(dx),
        y1: rect.y1.saturating_add(dy),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pt(x: f64, y: f64) -> Point {
        Point::new(x, y)
    }

    #[test]
    fn a_drag_runs_from_corner_to_corner_in_any_direction() {
        let a = drawn(pt(10.2, 20.7), pt(60.4, 50.5), false, false).unwrap();
        assert_eq!(a, PxRect::new(10, 21, 60, 51));
        let b = drawn(pt(60.4, 50.5), pt(10.2, 20.7), false, false).unwrap();
        assert_eq!(a.width(), b.width());
        assert_eq!(b, PxRect::new(10, 21, 60, 51));
    }

    #[test]
    fn a_drag_under_two_pixels_draws_nothing() {
        assert_eq!(drawn(pt(5.0, 5.0), pt(6.5, 6.9), false, false), None);
        assert_eq!(drawn(pt(5.0, 5.0), pt(5.0, 5.0), true, true), None);
        assert!(drawn(pt(5.0, 5.0), pt(7.0, 5.0), false, false).is_some());
    }

    #[test]
    fn a_drag_that_rounds_flat_still_has_an_area() {
        let r = drawn(pt(5.0, 5.0), pt(9.0, 5.2), false, false).unwrap();
        assert_eq!((r.width(), r.height()), (4, 1));
    }

    #[test]
    fn shift_draws_from_the_centre() {
        let r = drawn(pt(50.0, 40.0), pt(70.0, 55.0), true, false).unwrap();
        assert_eq!(r, PxRect::new(30, 25, 70, 55));
    }

    #[test]
    fn ctrl_draws_a_square_towards_the_pointer() {
        let r = drawn(pt(50.0, 40.0), pt(70.0, 55.0), false, true).unwrap();
        assert_eq!(r, PxRect::new(50, 40, 70, 60));
        let l = drawn(pt(50.0, 40.0), pt(30.0, 45.0), false, true).unwrap();
        assert_eq!(l, PxRect::new(30, 40, 50, 60));
        let both = drawn(pt(50.0, 40.0), pt(70.0, 55.0), true, true).unwrap();
        assert_eq!(both, PxRect::new(30, 20, 70, 60));
        assert_eq!(both.width(), both.height());
    }

    #[test]
    fn the_topmost_region_wins_and_the_slop_reaches_past_the_outline() {
        let regions = [
            Region::rect(PxRect::new(0, 0, 100, 100), false),
            Region::rect(PxRect::new(40, 40, 60, 60), true),
        ];
        assert_eq!(region_at(&regions, pt(50.0, 50.0), 0.0), Some(1));
        assert_eq!(region_at(&regions, pt(10.0, 10.0), 0.0), Some(0));
        assert_eq!(region_at(&regions, pt(104.0, 50.0), 0.0), None);
        assert_eq!(region_at(&regions, pt(104.0, 50.0), 8.0), Some(0));
        assert_eq!(region_at(&regions, pt(-9.0, 50.0), 8.0), None);
        assert_eq!(region_at(&[], pt(0.0, 0.0), 8.0), None);
    }

    #[test]
    fn a_handle_is_hit_near_its_centre_and_the_nearest_wins() {
        let rect = Rect::new(100.0, 100.0, 300.0, 200.0);
        assert_eq!(handle_at(rect, pt(101.0, 102.0)), Some(Handle::NW));
        assert_eq!(handle_at(rect, pt(200.0, 98.0)), Some(Handle::N));
        assert_eq!(handle_at(rect, pt(302.0, 150.0)), Some(Handle::E));
        assert_eq!(handle_at(rect, pt(200.0, 150.0)), None);
        // 16 px is the largest radius.
        assert_eq!(handle_at(rect, pt(300.0 + 15.0, 100.0)), Some(Handle::NE));
        assert_eq!(handle_at(rect, pt(300.0 + 17.0, 100.0)), None);
    }

    #[test]
    fn a_small_rectangle_has_only_corner_handles_and_small_radii() {
        let small = Rect::new(0.0, 0.0, 20.0, 20.0);
        assert_eq!(
            handle_at(small, pt(10.0, 0.0)),
            None,
            "no edge handle under 24 px"
        );
        // A third of 20 px is 6.7 px: the corner at 6 px away is hit, 7 px away is not.
        assert_eq!(handle_at(small, pt(6.0, 0.0)), Some(Handle::NW));
        assert_eq!(handle_at(small, pt(7.0, 0.0)), None);
        let tiny = Rect::new(0.0, 0.0, 6.0, 6.0);
        // The radius is never under 4 px; the nearest of the overlapping corners wins.
        assert_eq!(handle_at(tiny, pt(1.0, 1.0)), Some(Handle::NW));
        assert_eq!(handle_at(tiny, pt(5.0, 5.0)), Some(Handle::SE));
        let edge_ok = Rect::new(0.0, 0.0, 100.0, 24.0);
        assert_eq!(handle_at(edge_ok, pt(50.0, 0.0)), Some(Handle::N));
    }

    #[test]
    fn an_edge_handle_moves_one_side() {
        let r = PxRect::new(10, 10, 50, 40);
        assert_eq!(
            resized(r, Handle::E, pt(70.4, 99.0), false, false),
            PxRect::new(10, 10, 70, 40)
        );
        assert_eq!(
            resized(r, Handle::N, pt(0.0, 2.0), false, false),
            PxRect::new(10, 2, 50, 40)
        );
        assert_eq!(
            resized(r, Handle::W, pt(-5.0, 0.0), false, false),
            PxRect::new(-5, 10, 50, 40)
        );
    }

    #[test]
    fn a_corner_handle_moves_two_sides_and_stops_at_one_pixel() {
        let r = PxRect::new(10, 10, 50, 40);
        assert_eq!(
            resized(r, Handle::SE, pt(60.0, 80.0), false, false),
            PxRect::new(10, 10, 60, 80)
        );
        let squeezed = resized(r, Handle::SE, pt(-30.0, -30.0), false, false);
        assert_eq!(squeezed, PxRect::new(10, 10, 11, 11));
        let nw = resized(r, Handle::NW, pt(90.0, 90.0), false, false);
        assert_eq!(nw, PxRect::new(49, 39, 50, 40));
    }

    #[test]
    fn shift_resizes_about_the_centre() {
        let r = PxRect::new(10, 10, 50, 40);
        let wide = resized(r, Handle::E, pt(70.0, 0.0), true, false);
        assert_eq!(wide, PxRect::new(-10, 10, 70, 40));
        let corner = resized(r, Handle::NW, pt(0.0, 0.0), true, false);
        assert_eq!(corner, PxRect::new(0, 0, 60, 50));
        let squeezed = resized(r, Handle::E, pt(-100.0, 0.0), true, false);
        assert_eq!(
            (squeezed.width(), squeezed.height()),
            (2, 30),
            "the centre stays, 2 px is the least with an even width"
        );
        let odd = resized(
            PxRect::new(10, 10, 51, 40),
            Handle::E,
            pt(-100.0, 0.0),
            true,
            false,
        );
        assert_eq!(odd.width(), 1);
    }

    #[test]
    fn ctrl_keeps_the_proportions_at_a_corner() {
        let r = PxRect::new(0, 0, 40, 20);
        let grown = resized(r, Handle::SE, pt(80.0, 30.0), false, true);
        assert_eq!(grown, PxRect::new(0, 0, 80, 40));
        let from_nw = resized(r, Handle::NW, pt(-40.0, -5.0), false, true);
        assert_eq!(from_nw, PxRect::new(-40, -20, 40, 20));
        let about_centre = resized(r, Handle::SE, pt(60.0, 30.0), true, true);
        assert_eq!(about_centre, PxRect::new(-20, -10, 60, 30));
        // An edge handle ignores the ratio.
        assert_eq!(
            resized(r, Handle::E, pt(100.0, 0.0), false, true),
            PxRect::new(0, 0, 100, 20)
        );
    }

    #[test]
    fn a_rectangle_moves_by_whole_pixels() {
        assert_eq!(
            moved(PxRect::new(1, 2, 5, 9), -3, 10),
            PxRect::new(-2, 12, 2, 19)
        );
        assert_eq!(moved(PxRect::new(1, 2, 5, 9), i32::MAX, 0).x1, i32::MAX);
    }

    fn lasso(points: &[(i32, i32)], subtract: bool) -> Region {
        let corners: Vec<crate::regions::PxPoint> = points
            .iter()
            .map(|&(x, y)| crate::regions::PxPoint::new(x, y))
            .collect();
        Region::lasso(&corners, subtract).unwrap()
    }

    #[test]
    fn a_lasso_is_hit_inside_by_its_winding_and_within_the_slop_of_a_side() {
        let triangle = [lasso(&[(0, 0), (100, 0), (0, 100)], false)];
        assert_eq!(region_at(&triangle, pt(20.0, 20.0), 0.0), Some(0));
        // Inside the box, outside the slanted side: not hit without slop, hit within 8 px of it.
        assert_eq!(region_at(&triangle, pt(70.0, 70.0), 0.0), None);
        assert_eq!(region_at(&triangle, pt(53.0, 53.0), 8.0), Some(0));
        assert_eq!(region_at(&triangle, pt(70.0, 70.0), 8.0), None);
        assert_eq!(region_at(&triangle, pt(-6.0, 50.0), 8.0), Some(0));
    }

    #[test]
    fn a_figure_eight_is_hit_in_both_lobes_and_not_between_them() {
        let eight = [lasso(&[(20, 10), (80, 70), (80, 10), (20, 70)], false)];
        assert_eq!(region_at(&eight, pt(70.0, 40.0), 0.0), Some(0));
        assert_eq!(region_at(&eight, pt(30.0, 40.0), 0.0), Some(0));
        assert_eq!(region_at(&eight, pt(50.0, 20.0), 0.0), None);
    }

    #[test]
    fn the_topmost_of_a_lasso_and_a_rectangle_wins() {
        let regions = [
            lasso(&[(0, 0), (100, 0), (0, 100)], false),
            Region::rect(PxRect::new(10, 10, 30, 30), false),
        ];
        assert_eq!(region_at(&regions, pt(20.0, 20.0), 0.0), Some(1));
        assert_eq!(region_at(&regions, pt(60.0, 20.0), 0.0), Some(0));
    }

    #[test]
    fn a_corner_is_hit_within_the_radius_and_the_nearest_wins() {
        let corners = [pt(100.0, 100.0), pt(110.0, 100.0), pt(200.0, 300.0)];
        assert_eq!(corner_at(&corners, pt(104.0, 100.0), 16.0), Some(0));
        assert_eq!(corner_at(&corners, pt(107.0, 100.0), 16.0), Some(1));
        assert_eq!(corner_at(&corners, pt(200.0, 316.0), 16.0), Some(2));
        assert_eq!(corner_at(&corners, pt(200.0, 317.0), 16.0), None);
        assert_eq!(corner_at(&[], pt(0.0, 0.0), 16.0), None);
    }

    #[test]
    fn a_corner_lands_on_the_nearest_whole_pixel() {
        let c = nearest_pixel(pt(10.4, 20.6));
        assert_eq!((c.x, c.y), (10, 21));
    }
}
