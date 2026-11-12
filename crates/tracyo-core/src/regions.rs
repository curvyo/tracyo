//! The region model: rectangles in picture pixels, added to or taken from the mask of a part.

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

/// One rectangle of a mask and its sign.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Region {
    /// The rectangle in picture pixels.
    pub rect: PxRect,
    /// Taken from the mask instead of added to it.
    pub subtract: bool,
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
}
