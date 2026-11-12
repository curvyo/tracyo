//! The area a part keeps, built from rectangles and applied to traced outlines as vectrace's
//! [`Mask`].
//!
//! The mask is the union of the added rectangles minus the union of the subtracted ones, on the
//! 0.001 mm grid. An outline whose control box meets no edge of the mask lies wholly inside or
//! outside it and stays or goes with its curves; the outlines that meet an edge go through one
//! overlay (Intersection for Inside, Difference for Outside) and come back as polylines.

use i_overlay::core::overlay_rule::OverlayRule;
use kurbo::{BezPath, PathEl, Shape};
use vectrace::PixelMap;
use vectrace::mask::{Clipped, Mask};

use crate::grid::{self, Path64, Point64};
use crate::regions::{MaskMode, Region};

/// The notice of a part whose mask covers none of the picture.
pub const OUTSIDE_IMAGE: &str = "0 paths: the mask is outside the image.";
/// The notice of a mask the geometry could not combine.
pub const COULD_NOT_COMBINE: &str = "0 paths: the mask paths could not be combined.";

/// Why the regions make no mask.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ClipError {
    /// The overlay returned nothing for rectangles of positive area (never expected).
    #[error("the mask paths could not be combined")]
    Kernel,
}

/// The area a part keeps: the mask, or everything outside it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Clip {
    /// The mask's polygons in canonical order: outer outlines with positive area, holes negative.
    polygons: Vec<Path64>,
    /// The trace keeps what lies outside the polygons.
    outside: bool,
    /// The polygons' box in grid units: min x, min y, max x, max y.
    bounds: [i64; 4],
}

/// What the regions of a part ask of the trace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MaskPlan {
    /// No region, or Outside a mask that touches no picture pixel: the whole picture is traced.
    Whole,
    /// Trace through this clip.
    Clip(Clip),
    /// Nothing is traced; the text says why.
    Nothing(&'static str),
}

/// The plan of `regions` for a picture of `width` x `height` px, Inside or Outside, with the
/// full picture's `map`.
#[must_use]
pub fn plan(
    regions: &[Region],
    mode: MaskMode,
    map: PixelMap,
    width: u32,
    height: u32,
) -> MaskPlan {
    if regions.is_empty() {
        return MaskPlan::Whole;
    }
    let outside = mode == MaskMode::Outside;
    let clip = match Clip::of_regions(regions, mode, map) {
        Ok(Some(clip)) => clip,
        Ok(None) if outside => return MaskPlan::Whole,
        Ok(None) => return MaskPlan::Nothing(OUTSIDE_IMAGE),
        Err(ClipError::Kernel) => return MaskPlan::Nothing(COULD_NOT_COMBINE),
    };
    let picture = map.to_kurbo().transform_rect_bbox(kurbo::Rect::new(
        0.0,
        0.0,
        f64::from(width),
        f64::from(height),
    ));
    if clip.meets(picture) {
        MaskPlan::Clip(clip)
    } else if outside {
        MaskPlan::Whole
    } else {
        MaskPlan::Nothing(OUTSIDE_IMAGE)
    }
}

impl Clip {
    /// The clip of `regions` (corners in picture pixels, taken to millimetres by `map`), Inside
    /// or Outside. `Ok(None)` when the mask is empty: no added region, or all of it subtracted.
    ///
    /// # Errors
    /// [`ClipError::Kernel`] when the overlay returns nothing for added rectangles and no
    /// rectangle is subtracted.
    pub fn of_regions(
        regions: &[Region],
        mode: MaskMode,
        map: PixelMap,
    ) -> Result<Option<Self>, ClipError> {
        let corner = |x: i32, y: i32| -> Option<Point64> {
            let p = map.apply_point(kurbo::Point::new(f64::from(x), f64::from(y)));
            grid::snap(p.x, p.y)
        };
        let mut added = Vec::new();
        let mut taken = Vec::new();
        for region in regions {
            let r = region.rect;
            let ring: Option<Path64> = [(r.x0, r.y0), (r.x1, r.y0), (r.x1, r.y1), (r.x0, r.y1)]
                .into_iter()
                .map(|(x, y)| corner(x, y))
                .collect();
            let Some(ring) = ring else {
                return Err(ClipError::Kernel);
            };
            // A mirrored map winds the corners the other way round; the overlay wants one winding.
            let ring = if grid::double_area(&ring) < 0 {
                ring.into_iter().rev().collect()
            } else {
                ring
            };
            if region.subtract {
                taken.push(ring);
            } else {
                added.push(ring);
            }
        }
        if added.is_empty() {
            return Ok(None);
        }
        let polygons = grid::canonical(grid::run(OverlayRule::Difference, &added, &taken));
        if polygons.is_empty() {
            return if taken.is_empty() {
                Err(ClipError::Kernel)
            } else {
                Ok(None)
            };
        }
        Ok(Some(Self::from_polygons(
            polygons,
            mode == MaskMode::Outside,
        )))
    }

    /// A clip of ready polygons (canonical order, outer outlines positive, holes negative).
    pub(crate) fn from_polygons(polygons: Vec<Path64>, outside: bool) -> Self {
        let mut bounds = [i64::MAX, i64::MAX, i64::MIN, i64::MIN];
        for p in polygons.iter().flatten() {
            bounds = [
                bounds[0].min(p.x),
                bounds[1].min(p.y),
                bounds[2].max(p.x),
                bounds[3].max(p.y),
            ];
        }
        Self {
            polygons,
            outside,
            bounds,
        }
    }

    /// Whether the trace keeps what lies outside the mask.
    #[must_use]
    pub fn outside(&self) -> bool {
        self.outside
    }

    /// The mask's polygons, in grid units.
    #[must_use]
    pub fn polygons(&self) -> &[Path64] {
        &self.polygons
    }

    /// The box of the mask's polygons in millimetres.
    #[must_use]
    pub fn bounds_mm(&self) -> kurbo::Rect {
        let [x0, y0, x1, y1] = self.bounds.map(grid::to_mm);
        kurbo::Rect::new(x0, y0, x1, y1)
    }

    /// Whether the mask covers any of `picture` (millimetres), whatever the mode.
    #[must_use]
    pub fn meets(&self, picture: kurbo::Rect) -> bool {
        let Some(a) = grid::snap(picture.x0, picture.y0) else {
            return false;
        };
        let Some(b) = grid::snap(picture.x1, picture.y1) else {
            return false;
        };
        let rect = grid::rect_path(a.x.min(b.x), a.y.min(b.y), a.x.max(b.x), a.y.max(b.y));
        let overlap = grid::run(OverlayRule::Intersect, &self.polygons, &[rect]);
        grid::area_mm2(&overlap) > 0.0
    }

    /// Whether any edge of the mask meets the closed `bounds` box (millimetres).
    #[must_use]
    pub fn edge_meets(&self, bounds: kurbo::Rect) -> bool {
        let (min, max) = (
            kurbo::Point::new(bounds.x0, bounds.y0),
            kurbo::Point::new(bounds.x1, bounds.y1),
        );
        let [bx0, by0, bx1, by1] = self.bounds.map(grid::to_mm);
        if max.x < bx0 || min.x > bx1 || max.y < by0 || min.y > by1 {
            return false;
        }
        self.polygons.iter().any(|polygon| {
            let Some(mut a) = polygon.last().map(mm_of) else {
                return false;
            };
            polygon.iter().any(|q| {
                let b = mm_of(q);
                let meets = segment_meets_box(a, b, min, max);
                a = b;
                meets
            })
        })
    }

    /// Whether the mask contains `p` (millimetres; nonzero winding).
    fn contains(&self, p: kurbo::Point) -> bool {
        let [bx0, by0, bx1, by1] = self.bounds.map(grid::to_mm);
        if p.x < bx0 || p.x > bx1 || p.y < by0 || p.y > by1 {
            return false;
        }
        let mut winding = 0_i32;
        for polygon in &self.polygons {
            let Some(mut a) = polygon.last().map(mm_of) else {
                continue;
            };
            for q in polygon {
                let b = mm_of(q);
                if a.y <= p.y {
                    if b.y > p.y && cross(a, b, p) > 0.0 {
                        winding += 1;
                    }
                } else if b.y <= p.y && cross(a, b, p) < 0.0 {
                    winding -= 1;
                }
                a = b;
            }
        }
        winding != 0
    }

    /// Per pixel of a `width` x `height` picture whose pixel corners `map` takes to millimetres:
    /// whether the trace keeps it (its centre lies in the kept area). `None` when the map has no
    /// inverse. The cost is rows times edges, not pixels times edges.
    #[must_use]
    pub fn kept_pixels(&self, map: PixelMap, width: u32, height: u32) -> Option<Vec<bool>> {
        let inverse = map.inverse()?;
        let (w, h) = (width as usize, height as usize);
        let polygons: Vec<Vec<kurbo::Point>> = self
            .polygons
            .iter()
            .map(|p| p.iter().map(|q| inverse.apply_point(mm_of(q))).collect())
            .collect();
        let mut inside = vec![false; w * h];
        let mut crossings: Vec<(f64, i32)> = Vec::new();
        for y in 0..h {
            // invariant: rows are far below 2^52.
            #[allow(clippy::cast_precision_loss)]
            let centre = y as f64 + 0.5;
            crossings.clear();
            for polygon in &polygons {
                let Some(mut a) = polygon.last().copied() else {
                    continue;
                };
                for &b in polygon {
                    let up = a.y <= centre && centre < b.y;
                    let down = b.y <= centre && centre < a.y;
                    if up || down {
                        let x = a.x + (centre - a.y) * (b.x - a.x) / (b.y - a.y);
                        crossings.push((x, if up { 1 } else { -1 }));
                    }
                    a = b;
                }
            }
            crossings.sort_by(|p, q| p.0.total_cmp(&q.0).then(p.1.cmp(&q.1)));
            let row = &mut inside[y * w..(y + 1) * w];
            let mut winding = 0;
            for pair in crossings.windows(2) {
                winding += pair[0].1;
                if winding != 0 {
                    let (from, to) = (first_centre(pair[0].0, w), first_centre(pair[1].0, w));
                    row[from..to].fill(true);
                }
            }
        }
        if self.outside {
            for kept in &mut inside {
                *kept = !*kept;
            }
        }
        Some(inside)
    }

    /// The clip as flat numbers for the worker: the polygon count, then per polygon its vertex
    /// count and its coordinates in grid units. The mode travels separately.
    #[must_use]
    pub fn to_units(&self) -> Vec<i64> {
        let mut units = Vec::with_capacity(1 + self.polygons.len() + 2 * self.points());
        // invariant: counts are far below 2^62.
        #[allow(clippy::cast_possible_wrap)]
        {
            units.push(self.polygons.len() as i64);
            for polygon in &self.polygons {
                units.push(polygon.len() as i64);
                units.extend(polygon.iter().flat_map(|p| [p.x, p.y]));
            }
        }
        units
    }

    /// The clip of [`Self::to_units`]; `None` unless the numbers are well formed.
    #[must_use]
    pub fn from_units(units: &[i64], outside: bool) -> Option<Self> {
        let mut at = 0;
        let mut next = || -> Option<i64> {
            let v = *units.get(at)?;
            at += 1;
            Some(v)
        };
        let count = |v: i64| usize::try_from(v).ok();
        let polygons = count(next()?)?;
        if polygons > units.len() {
            return None;
        }
        let mut read = Vec::with_capacity(polygons);
        for _ in 0..polygons {
            let n = count(next()?)?;
            if n < 3 || n > units.len() {
                return None;
            }
            let mut polygon = Vec::with_capacity(n);
            for _ in 0..n {
                polygon.push(Point64::new(next()?, next()?));
            }
            read.push(polygon);
        }
        (at == units.len() && !read.is_empty()).then(|| Self::from_polygons(read, outside))
    }

    fn points(&self) -> usize {
        self.polygons.iter().map(Vec::len).sum()
    }

    /// The crossing `outlines` as one operand through the overlay, as polylines whose outer
    /// outlines have the kurbo area sign `sign`, and how many were kept whole after a refusal.
    fn clip_crossing(&self, outlines: &[&BezPath], sign: f64) -> (Vec<BezPath>, usize) {
        let mut subject: Vec<Path64> = Vec::new();
        for path in outlines {
            if let Some(rings) = grid::flatten_snap(path) {
                subject.extend(rings);
            } else {
                // A coordinate the grid cannot hold: the crossing outlines are not lost but kept
                // whole when the clip keeps their first point.
                let whole: Vec<BezPath> = outlines
                    .iter()
                    .filter(|p| first_point(p).is_some_and(|q| self.keeps_point(q)))
                    .map(|p| (*p).clone())
                    .collect();
                let count = whole.len();
                return (whole, count);
            }
        }
        let rule = if self.outside {
            OverlayRule::Difference
        } else {
            OverlayRule::Intersect
        };
        let result = grid::canonical(grid::run(rule, &subject, &self.polygons));
        (wound_polylines(&result, sign), 0)
    }

    fn keeps_point(&self, p: kurbo::Point) -> bool {
        self.contains(p) != self.outside
    }
}

/// `outlines` (closed, millimetres, holes wound against their outer outline) clipped to the kept
/// area. The outlines that stay whole keep their order and curves; the clipped ones come back as
/// polylines in the place of the first outline that crossed, wound as the trace winds its
/// outlines.
impl Mask for Clip {
    fn keeps(&self, p: kurbo::Point) -> bool {
        self.keeps_point(p)
    }

    fn clip(&self, outlines: &[BezPath]) -> Clipped {
        let mut kept = Vec::with_capacity(outlines.len());
        let mut crossing: Vec<&BezPath> = Vec::new();
        let mut crossing_at = None;
        for outline in outlines {
            if self.edge_meets(crossing_box(outline)) {
                crossing_at.get_or_insert(kept.len());
                crossing.push(outline);
                continue;
            }
            if first_point(outline).is_some_and(|p| self.keeps_point(p)) {
                kept.push(outline.clone());
            }
        }
        let Some(at) = crossing_at else {
            let changed = kept.len() != outlines.len();
            return Clipped {
                outlines: kept,
                changed,
                kept_whole: 0,
            };
        };
        let (pieces, kept_whole) = self.clip_crossing(&crossing, outer_sign(outlines));
        kept.splice(at..at, pieces);
        Clipped {
            outlines: kept,
            changed: true,
            kept_whole,
        }
    }
}

pub(crate) fn mm_of(p: &Point64) -> kurbo::Point {
    kurbo::Point::new(grid::to_mm(p.x), grid::to_mm(p.y))
}

/// The box of `path` that decides whether it crosses an edge: its exact bounding box, not the
/// box of its control points, so a cut outline (a polyline whose vertices lie on the curve) crosses
/// the edges again exactly when the curve did and a second cut changes nothing.
#[must_use]
pub fn crossing_box(path: &BezPath) -> kurbo::Rect {
    path.bounding_box()
}

/// The z component of `(b - a) x (p - a)`.
fn cross(a: kurbo::Point, b: kurbo::Point, p: kurbo::Point) -> f64 {
    (b.x - a.x) * (p.y - a.y) - (p.x - a.x) * (b.y - a.y)
}

/// Whether the segment `a`..`b` meets the closed box `min`..`max` (separating axes: the box's two
/// and the segment's normal).
fn segment_meets_box(
    a: kurbo::Point,
    b: kurbo::Point,
    min: kurbo::Point,
    max: kurbo::Point,
) -> bool {
    if a.x.max(b.x) < min.x || a.x.min(b.x) > max.x || a.y.max(b.y) < min.y || a.y.min(b.y) > max.y
    {
        return false;
    }
    let sides = [
        cross(a, b, min),
        cross(a, b, kurbo::Point::new(max.x, min.y)),
        cross(a, b, max),
        cross(a, b, kurbo::Point::new(min.x, max.y)),
    ];
    !(sides.iter().all(|&s| s > 0.0) || sides.iter().all(|&s| s < 0.0))
}

/// The first pixel whose centre lies at or right of `x`, within `0..=w`.
fn first_centre(x: f64, w: usize) -> usize {
    let first = (x - 0.5).ceil();
    if first <= 0.0 {
        0
    } else {
        // invariant: positive and compared with `w` before the cast.
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            clippy::cast_precision_loss
        )]
        if first >= w as f64 { w } else { first as usize }
    }
}

/// The first point of `path`.
fn first_point(path: &BezPath) -> Option<kurbo::Point> {
    path.elements().iter().find_map(|element| match *element {
        PathEl::MoveTo(p) => Some(p),
        _ => None,
    })
}

/// The kurbo area sign of the trace's outer outlines: the sign of the largest outline, which no
/// hole can be.
pub(crate) fn outer_sign(outlines: &[BezPath]) -> f64 {
    let largest =
        outlines.iter().map(Shape::area).fold(
            0.0_f64,
            |best, a| if a.abs() > best.abs() { a } else { best },
        );
    if largest < 0.0 { -1.0 } else { 1.0 }
}

/// The overlay's `polygons` as polyline paths whose outer outlines (positive area) have the kurbo
/// area sign `sign` and whose holes have the other one, as the trace winds them.
pub(crate) fn wound_polylines(polygons: &[Path64], sign: f64) -> Vec<BezPath> {
    polygons
        .iter()
        .map(|polygon| {
            let mut path = BezPath::new();
            for (i, q) in polygon.iter().enumerate() {
                let p = mm_of(q);
                if i == 0 {
                    path.move_to(p);
                } else {
                    path.line_to(p);
                }
            }
            path.close_path();
            let wanted = if grid::double_area(polygon) > 0 {
                sign
            } else {
                -sign
            };
            if path.area() * wanted < 0.0 {
                path = path.reverse_subpaths();
            }
            path
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::regions::PxRect;

    /// 1 px = 1 mm keeps the numbers readable.
    fn map() -> PixelMap {
        PixelMap::scale(1.0, 1.0)
    }

    fn add(x0: i32, y0: i32, x1: i32, y1: i32) -> Region {
        Region {
            rect: PxRect::new(x0, y0, x1, y1),
            subtract: false,
        }
    }

    fn take(x0: i32, y0: i32, x1: i32, y1: i32) -> Region {
        Region {
            subtract: true,
            ..add(x0, y0, x1, y1)
        }
    }

    fn clip(regions: &[Region], mode: MaskMode) -> Clip {
        Clip::of_regions(regions, mode, map()).unwrap().unwrap()
    }

    fn square(x0: f64, y0: f64, x1: f64, y1: f64) -> BezPath {
        let mut p = BezPath::new();
        p.move_to((x0, y0));
        p.line_to((x1, y0));
        p.line_to((x1, y1));
        p.line_to((x0, y1));
        p.close_path();
        p
    }

    /// The same mask with the other mode.
    fn flipped(c: &Clip) -> Clip {
        Clip::from_units(&c.to_units(), !c.outside()).unwrap()
    }

    fn area_of(c: &Clip) -> f64 {
        grid::area_mm2(c.polygons())
    }

    #[test]
    fn the_mask_is_the_added_rectangles_minus_the_subtracted_ones() {
        let c = clip(
            &[add(0, 0, 10, 10), add(5, 5, 20, 20), take(2, 2, 4, 4)],
            MaskMode::Inside,
        );
        // 100 + 225 - 25 overlap - the hole of 4.
        assert!((area_of(&c) - 296.0).abs() < 1e-9);
        assert_eq!(c.polygons().len(), 2, "an outline and a hole");
    }

    #[test]
    fn the_order_of_the_regions_does_not_change_the_bytes() {
        let regions = [add(0, 0, 10, 10), take(2, 2, 4, 4), add(5, 5, 20, 20)];
        let reference = clip(&regions, MaskMode::Inside);
        for order in [[2, 1, 0], [1, 0, 2], [0, 2, 1], [2, 0, 1]] {
            let shuffled: Vec<Region> = order.iter().map(|&i| regions[i]).collect();
            assert_eq!(clip(&shuffled, MaskMode::Inside), reference);
        }
    }

    #[test]
    fn corners_are_snapped_to_the_grid() {
        let c = Clip::of_regions(
            &[add(0, 0, 10, 10)],
            MaskMode::Inside,
            PixelMap::scale(25.4 / 96.0, 25.4 / 96.0),
        )
        .unwrap()
        .unwrap();
        let xs: Vec<i64> = c.polygons()[0].iter().map(|p| p.x).collect();
        // 10 px = 2.6458 mm = 2646 units.
        assert!(xs.contains(&2646) && xs.contains(&0));
    }

    #[test]
    fn a_mask_without_added_area_is_empty() {
        assert_eq!(
            Clip::of_regions(&[take(0, 0, 5, 5)], MaskMode::Inside, map()),
            Ok(None)
        );
        assert_eq!(
            Clip::of_regions(
                &[add(1, 1, 5, 5), take(0, 0, 9, 9)],
                MaskMode::Inside,
                map()
            ),
            Ok(None)
        );
        assert_eq!(Clip::of_regions(&[], MaskMode::Inside, map()), Ok(None));
    }

    #[test]
    fn the_mask_off_the_picture_does_not_meet_it() {
        let picture = kurbo::Rect::new(0.0, 0.0, 100.0, 100.0);
        assert!(clip(&[add(90, 90, 200, 200)], MaskMode::Inside).meets(picture));
        assert!(!clip(&[add(100, 0, 200, 100)], MaskMode::Inside).meets(picture));
        assert!(!clip(&[add(-50, -50, 0, 0)], MaskMode::Inside).meets(picture));
        // A hole in the middle of a mask that covers the picture still meets it.
        let framed = clip(
            &[add(-10, -10, 110, 110), take(0, 0, 100, 100)],
            MaskMode::Inside,
        );
        assert!(!framed.meets(picture), "the frame lies around the picture");
    }

    #[test]
    fn whole_outlines_stay_or_go_and_crossing_ones_are_cut() {
        let c = clip(&[add(0, 0, 10, 10)], MaskMode::Inside);
        let inside = square(1.0, 1.0, 2.0, 2.0);
        let outside = square(20.0, 1.0, 22.0, 2.0);
        let crossing = square(8.0, 1.0, 12.0, 3.0);
        let result = c.clip(&[inside.clone(), outside.clone()]);
        assert!(result.changed);
        assert_eq!(result.outlines, std::slice::from_ref(&inside));
        let result = c.clip(&[inside.clone(), crossing.clone()]);
        assert_eq!(result.outlines.len(), 2);
        assert_eq!(result.outlines[0], inside);
        assert!((result.outlines[1].area().abs() - 4.0).abs() < 1e-6);
        assert!(
            result.outlines[1].area() * inside.area() > 0.0,
            "the trace's winding is kept"
        );
        let out = flipped(&c).clip(&[inside, outside.clone(), crossing]);
        assert_eq!(out.outlines[0], outside);
        assert!((out.outlines[1].area().abs() - 4.0).abs() < 1e-6);
    }

    #[test]
    fn an_outline_the_mask_does_not_touch_is_not_marked_changed() {
        let c = clip(&[add(0, 0, 10, 10)], MaskMode::Inside);
        let result = c.clip(&[square(1.0, 1.0, 2.0, 2.0)]);
        assert!(!result.changed);
    }

    #[test]
    fn a_coordinate_the_grid_cannot_hold_keeps_the_outline_whole_by_its_first_point() {
        let c = clip(&[add(0, 0, 10, 10)], MaskMode::Inside);
        let far = square(5.0, 1.0, 2.0e7, 3.0);
        let result = c.clip(std::slice::from_ref(&far));
        assert_eq!(result.outlines, std::slice::from_ref(&far));
        assert_eq!(result.kept_whole, 1);
        assert_eq!(flipped(&c).clip(&[far]).outlines, []);
    }

    #[test]
    fn the_centre_of_a_text_box_is_kept_by_the_same_rule() {
        let c = clip(&[add(0, 0, 10, 10), take(4, 4, 6, 6)], MaskMode::Inside);
        assert!(Mask::keeps(&c, kurbo::Point::new(1.0, 1.0)));
        assert!(!Mask::keeps(&c, kurbo::Point::new(5.0, 5.0)), "in the hole");
        assert!(!Mask::keeps(&c, kurbo::Point::new(11.0, 5.0)));
        assert!(Mask::keeps(&flipped(&c), kurbo::Point::new(11.0, 5.0)));
    }

    #[test]
    fn the_numbers_round_trip_and_bad_ones_are_refused() {
        let c = clip(&[add(0, 0, 10, 10), take(2, 2, 4, 4)], MaskMode::Outside);
        let units = c.to_units();
        assert_eq!(Clip::from_units(&units, true), Some(c.clone()));
        assert_ne!(Clip::from_units(&units, false), Some(c));
        assert_eq!(Clip::from_units(&units[..units.len() - 1], true), None);
        assert_eq!(Clip::from_units(&[1, 2, 0, 0, 1, 1], false), None);
        assert_eq!(Clip::from_units(&[], false), None);
        assert_eq!(Clip::from_units(&[0], false), None);
        assert_eq!(Clip::from_units(&[i64::MAX], false), None);
    }

    #[test]
    fn the_kept_pixels_are_those_whose_centre_is_in_the_mask() {
        let c = clip(&[add(2, 1, 5, 3), take(3, 1, 4, 2)], MaskMode::Inside);
        let kept = c.kept_pixels(map(), 8, 4).unwrap();
        let grid_of = |kept: &[bool]| -> Vec<String> {
            kept.chunks(8)
                .map(|row| row.iter().map(|k| if *k { '#' } else { '.' }).collect())
                .collect()
        };
        assert_eq!(
            grid_of(&kept),
            ["........", "..#.#...", "..###...", "........"]
        );
        let outside = flipped(&c).kept_pixels(map(), 8, 4).unwrap();
        assert!(kept.iter().zip(&outside).all(|(a, b)| a != b));
    }

    #[test]
    fn a_picture_scaled_by_the_map_keeps_the_same_pixels() {
        let mm = 25.4 / 96.0;
        let c = Clip::of_regions(
            &[add(3, 0, 6, 4)],
            MaskMode::Inside,
            PixelMap::scale(mm, mm),
        )
        .unwrap()
        .unwrap();
        let kept = c.kept_pixels(PixelMap::scale(mm, mm), 10, 4).unwrap();
        assert_eq!(kept.iter().filter(|k| **k).count(), 3 * 4);
    }
}
