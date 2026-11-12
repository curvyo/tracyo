//! A lasso being drawn: the corners set so far, clicks that add or close, Backspace, and the
//! region that finishing makes.

use kurbo::Point;

use crate::region_edit::nearest_pixel;
use crate::regions::{PxPoint, Region};

/// What a click did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Click {
    /// A corner was added.
    Added,
    /// The click was on the pixel of the last corner: nothing changed.
    Ignored,
    /// The click was on the first corner of a lasso with three or more corners: finish it.
    Close,
}

/// The corners of a lasso that is not finished.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Draft {
    corners: Vec<PxPoint>,
}

impl Draft {
    /// A lasso with no corner.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The corners set so far, in order.
    #[must_use]
    pub fn corners(&self) -> &[PxPoint] {
        &self.corners
    }

    /// Whether no corner is set.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.corners.is_empty()
    }

    /// Whether `at` (picture pixels) is on the close target: three or more corners and within
    /// `radius_px` of the first.
    #[must_use]
    pub fn closes_at(&self, at: Point, radius_px: f64) -> bool {
        self.corners.len() >= 3
            && self
                .corners
                .first()
                .is_some_and(|first| first.to_kurbo().distance(at) <= radius_px)
    }

    /// A click at `at` (picture pixels): closes on the first corner, else sets a corner on the
    /// nearest whole pixel, unless that pixel is the last corner's.
    pub fn click(&mut self, at: Point, close_radius_px: f64) -> Click {
        if self.closes_at(at, close_radius_px) {
            return Click::Close;
        }
        let corner = nearest_pixel(at);
        if self.corners.last() == Some(&corner) {
            return Click::Ignored;
        }
        self.corners.push(corner);
        Click::Added
    }

    /// Backspace: the last corner goes; the only corner goes with the lasso.
    pub fn backspace(&mut self) {
        self.corners.pop();
    }

    /// The region the lasso makes when it is finished, closed by a straight side from the last
    /// corner to the first; `None` for fewer than three corners or no area.
    #[must_use]
    pub fn finish(&self, subtract: bool) -> Option<Region> {
        Region::lasso(&self.corners, subtract)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::regions::Form;

    fn at(x: f64, y: f64) -> Point {
        Point::new(x, y)
    }

    #[test]
    fn a_click_sets_a_corner_on_the_nearest_pixel() {
        let mut d = Draft::new();
        assert_eq!(d.click(at(10.4, 20.6), 16.0), Click::Added);
        assert_eq!(d.corners(), &[PxPoint::new(10, 21)]);
    }

    #[test]
    fn a_click_on_the_last_corner_adds_nothing() {
        let mut d = Draft::new();
        d.click(at(10.0, 10.0), 16.0);
        assert_eq!(d.click(at(10.3, 9.8), 16.0), Click::Ignored);
        assert_eq!(d.corners().len(), 1);
    }

    #[test]
    fn the_first_corner_closes_only_with_three_corners() {
        let mut d = Draft::new();
        d.click(at(0.0, 0.0), 16.0);
        d.click(at(100.0, 0.0), 16.0);
        // Two corners: a click near the first is a new corner.
        assert_eq!(d.click(at(5.0, 5.0), 16.0), Click::Added);
        // Three corners: within 16 px closes, 17 px does not.
        assert_eq!(d.click(at(16.0, 0.0), 16.0), Click::Close);
        assert!(!d.closes_at(at(17.0, 0.0), 16.0));
        assert_eq!(d.corners().len(), 3, "a closing click adds no corner");
    }

    #[test]
    fn backspace_removes_the_last_corner_and_the_only_corner_cancels() {
        let mut d = Draft::new();
        d.click(at(0.0, 0.0), 16.0);
        d.click(at(50.0, 0.0), 16.0);
        d.backspace();
        assert_eq!(d.corners(), &[PxPoint::new(0, 0)]);
        d.backspace();
        assert!(d.is_empty());
        d.backspace();
        assert!(d.is_empty());
    }

    #[test]
    fn finishing_closes_with_a_straight_side_or_drops_the_lasso() {
        let mut d = Draft::new();
        d.click(at(0.0, 0.0), 16.0);
        d.click(at(100.0, 0.0), 16.0);
        assert!(d.finish(false).is_none(), "two corners store nothing");
        d.click(at(0.0, 100.0), 16.0);
        let region = d.finish(true).unwrap();
        assert!(region.subtract);
        assert_eq!(region.form, Form::Lasso(d.corners().to_vec()));
        let mut flat = Draft::new();
        for x in [0.0, 10.0, 20.0] {
            flat.click(at(x, 5.0), 16.0);
        }
        assert!(
            flat.finish(false).is_none(),
            "corners on one line have no area"
        );
    }
}
