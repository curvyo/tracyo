//! The region model: rectangles and lassos in picture pixels, added to or taken from the mask of a
//! part.

/// A rectangle with whole-pixel corners in picture pixels; it may lie partly or wholly off the
/// picture. `x0 < x1` and `y0 < y1` always hold for one made by [`PxRect::new`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PxRect {
    /// Left edge.
    pub x0: i32,
    /// Top edge.
    pub y0: i32,
    /// Right edge, exclusive.
    pub x1: i32,
    /// Bottom edge, exclusive.
    pub y1: i32,
}

impl PxRect {
    /// The rectangle between two corners given in any order; a side that would be 0 px long is
    /// made 1 px long, so a region always has an area.
    #[must_use]
    pub fn new(xa: i32, ya: i32, xb: i32, yb: i32) -> Self {
        let (x0, x1) = ordered(xa, xb);
        let (y0, y1) = ordered(ya, yb);
        Self { x0, y0, x1, y1 }
    }

    /// The picture's own rectangle.
    #[must_use]
    pub fn of_picture(width: u32, height: u32) -> Self {
        Self::new(
            0,
            0,
            i32::try_from(width).unwrap_or(i32::MAX),
            i32::try_from(height).unwrap_or(i32::MAX),
        )
    }

    /// Width in px.
    #[must_use]
    pub fn width(&self) -> i32 {
        self.x1.saturating_sub(self.x0)
    }

    /// Height in px.
    #[must_use]
    pub fn height(&self) -> i32 {
        self.y1.saturating_sub(self.y0)
    }

    /// The part of `self` inside `other`, if it has an area.
    #[must_use]
    pub fn intersection(&self, other: &Self) -> Option<Self> {
        let r = Self {
            x0: self.x0.max(other.x0),
            y0: self.y0.max(other.y0),
            x1: self.x1.min(other.x1),
            y1: self.y1.min(other.y1),
        };
        (r.x0 < r.x1 && r.y0 < r.y1).then_some(r)
    }

    /// Whether `self` lies wholly inside `other`.
    #[must_use]
    pub fn within(&self, other: &Self) -> bool {
        self.x0 >= other.x0 && self.y0 >= other.y0 && self.x1 <= other.x1 && self.y1 <= other.y1
    }

    /// The same rectangle as a kurbo rectangle.
    #[must_use]
    pub fn to_kurbo(&self) -> kurbo::Rect {
        kurbo::Rect::new(
            f64::from(self.x0),
            f64::from(self.y0),
            f64::from(self.x1),
            f64::from(self.y1),
        )
    }
}

/// `a` and `b` as (low, high), 1 apart when they are equal.
fn ordered(a: i32, b: i32) -> (i32, i32) {
    match a.cmp(&b) {
        std::cmp::Ordering::Less => (a, b),
        std::cmp::Ordering::Greater => (b, a),
        std::cmp::Ordering::Equal if a == i32::MAX => (a - 1, a),
        std::cmp::Ordering::Equal => (a, a + 1),
    }
}

/// A point in whole picture pixels (a corner of a lasso).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PxPoint {
    /// Distance from the left edge.
    pub x: i32,
    /// Distance from the top edge.
    pub y: i32,
}

impl PxPoint {
    /// The point at (`x`, `y`).
    #[must_use]
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    /// The point `dx`, `dy` further on.
    #[must_use]
    pub fn moved(self, dx: i32, dy: i32) -> Self {
        Self::new(self.x.saturating_add(dx), self.y.saturating_add(dy))
    }

    /// The same point as a kurbo point.
    #[must_use]
    pub fn to_kurbo(self) -> kurbo::Point {
        kurbo::Point::new(f64::from(self.x), f64::from(self.y))
    }
}

/// What a region is: a rectangle, or a lasso, a closed polygon of straight sides whose area is
/// its non-zero winding area (a figure 8 is filled wherever it is covered).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Form {
    /// A rectangle with its sides parallel to the picture's.
    Rect(PxRect),
    /// The corners of a lasso in drawing order, at least three and not all on one line; the last
    /// side runs back to the first.
    Lasso(Vec<PxPoint>),
}

/// One region of a mask and its sign.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Region {
    /// The area.
    pub form: Form,
    /// Taken from the mask instead of added to it.
    pub subtract: bool,
}

impl Region {
    /// A rectangle region.
    #[must_use]
    pub const fn rect(rect: PxRect, subtract: bool) -> Self {
        Self {
            form: Form::Rect(rect),
            subtract,
        }
    }

    /// A lasso region of `corners`; consecutive equal corners count once. `None` for fewer than
    /// three corners or corners that all lie on one line (the lasso has no area).
    #[must_use]
    pub fn lasso(corners: &[PxPoint], subtract: bool) -> Option<Self> {
        let mut points: Vec<PxPoint> = Vec::with_capacity(corners.len());
        for &p in corners {
            if points.last() != Some(&p) {
                points.push(p);
            }
        }
        while points.len() > 1 && points.first() == points.last() {
            points.pop();
        }
        (points.len() >= 3 && !collinear(&points)).then_some(Self {
            form: Form::Lasso(points),
            subtract,
        })
    }

    /// The smallest rectangle that holds the region (at least 1 px each way).
    #[must_use]
    pub fn bounds(&self) -> PxRect {
        match &self.form {
            Form::Rect(r) => *r,
            Form::Lasso(points) => {
                let (mut x0, mut y0, mut x1, mut y1) = (i32::MAX, i32::MAX, i32::MIN, i32::MIN);
                for p in points {
                    x0 = x0.min(p.x);
                    y0 = y0.min(p.y);
                    x1 = x1.max(p.x);
                    y1 = y1.max(p.y);
                }
                PxRect::new(x0, y0, x1, y1)
            }
        }
    }

    /// The corners of the outline in order: a rectangle's four from the top-left corner
    /// clockwise, a lasso's own.
    #[must_use]
    pub fn corners(&self) -> Vec<PxPoint> {
        match &self.form {
            Form::Rect(r) => vec![
                PxPoint::new(r.x0, r.y0),
                PxPoint::new(r.x1, r.y0),
                PxPoint::new(r.x1, r.y1),
                PxPoint::new(r.x0, r.y1),
            ],
            Form::Lasso(points) => points.clone(),
        }
    }

    /// The region moved by (`dx`, `dy`) pixels.
    #[must_use]
    pub fn moved(&self, dx: i32, dy: i32) -> Self {
        let form = match &self.form {
            Form::Rect(r) => Form::Rect(crate::region_edit::moved(*r, dx, dy)),
            Form::Lasso(points) => Form::Lasso(points.iter().map(|p| p.moved(dx, dy)).collect()),
        };
        Self {
            form,
            subtract: self.subtract,
        }
    }

    /// A lasso with corner `index` put at `to`; any other region as it is. A corner that would
    /// leave the lasso without area is moved all the same (the maker may be on the way across).
    #[must_use]
    pub fn with_corner(&self, index: usize, to: PxPoint) -> Self {
        let mut out = self.clone();
        if let Form::Lasso(points) = &mut out.form
            && let Some(p) = points.get_mut(index)
        {
            *p = to;
        }
        out
    }
}

/// Whether all `points` lie on one line (exact, in 64 bits).
fn collinear(points: &[PxPoint]) -> bool {
    let Some(&first) = points.first() else {
        return true;
    };
    let Some(&second) = points.iter().find(|&&p| p != first) else {
        return true;
    };
    let (dx, dy) = (
        i64::from(second.x) - i64::from(first.x),
        i64::from(second.y) - i64::from(first.y),
    );
    points.iter().all(|p| {
        dx * (i64::from(p.y) - i64::from(first.y)) == dy * (i64::from(p.x) - i64::from(first.x))
    })
}

/// Which side of the mask a part traces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MaskMode {
    /// What lies inside the mask.
    #[default]
    Inside,
    /// What lies outside the mask.
    Outside,
}

/// The label of the mode in the bar and in the notices.
impl MaskMode {
    /// "Inside" or "Outside".
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Inside => "Inside",
            Self::Outside => "Outside",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn corners_in_any_order_make_the_same_rectangle() {
        assert_eq!(PxRect::new(10, 20, 4, 2), PxRect::new(4, 2, 10, 20));
        assert_eq!(
            (
                PxRect::new(4, 2, 10, 20).width(),
                PxRect::new(4, 2, 10, 20).height()
            ),
            (6, 18)
        );
    }

    #[test]
    fn a_rectangle_is_never_flat() {
        let r = PxRect::new(5, 5, 5, 9);
        assert_eq!((r.width(), r.height()), (1, 4));
        assert_eq!(PxRect::new(i32::MAX, 0, i32::MAX, 3).width(), 1);
    }

    #[test]
    fn intersection_needs_an_area() {
        let a = PxRect::new(0, 0, 10, 10);
        assert_eq!(
            a.intersection(&PxRect::new(5, 5, 20, 20)),
            Some(PxRect::new(5, 5, 10, 10))
        );
        assert_eq!(a.intersection(&PxRect::new(10, 0, 20, 10)), None);
        assert!(PxRect::new(2, 2, 3, 3).within(&a));
        assert!(!PxRect::new(2, 2, 11, 3).within(&a));
    }

    fn p(x: i32, y: i32) -> PxPoint {
        PxPoint::new(x, y)
    }

    #[test]
    fn a_lasso_needs_three_corners_and_an_area() {
        assert!(Region::lasso(&[p(0, 0), p(5, 5)], false).is_none());
        assert!(Region::lasso(&[p(0, 0), p(5, 5), p(10, 10)], false).is_none());
        assert!(Region::lasso(&[p(0, 0), p(5, 0), p(5, 0), p(0, 0)], false).is_none());
        let l = Region::lasso(&[p(0, 0), p(10, 0), p(0, 10)], true).unwrap();
        assert!(l.subtract);
        assert_eq!(l.bounds(), PxRect::new(0, 0, 10, 10));
    }

    #[test]
    fn equal_neighbours_count_once_and_a_figure_eight_keeps_its_area() {
        let l = Region::lasso(&[p(0, 0), p(0, 0), p(10, 0), p(0, 0)], false);
        assert!(l.is_none(), "two distinct corners are a line");
        // A bow tie has zero signed area but is not flat.
        let bow = Region::lasso(&[p(0, 0), p(10, 10), p(10, 0), p(0, 10)], false).unwrap();
        assert_eq!(bow.corners().len(), 4);
    }

    #[test]
    fn a_lasso_moves_and_a_corner_moves_alone() {
        let l = Region::lasso(&[p(0, 0), p(10, 0), p(0, 10)], false).unwrap();
        assert_eq!(l.moved(5, -2).corners(), vec![p(5, -2), p(15, -2), p(5, 8)]);
        assert_eq!(
            l.with_corner(1, p(20, 3)).corners(),
            vec![p(0, 0), p(20, 3), p(0, 10)]
        );
        let r = Region::rect(PxRect::new(0, 0, 4, 4), false);
        assert_eq!(
            r.with_corner(0, p(9, 9)),
            r,
            "a rectangle has no corner drag"
        );
        assert_eq!(r.moved(1, 1).bounds(), PxRect::new(1, 1, 5, 5));
    }
}
