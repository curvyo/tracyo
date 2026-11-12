//! What each part's trace is clipped to once the later parts are taken out: a later part wins
//! where parts overlap, so no area is traced twice.
//!
//! Every kept area is a set of grid polygons (outer outlines positive, holes negative). A part
//! that no later part overlaps keeps its own clip, so its curves stay; a part that a later one
//! overlaps gets the visible polygons as an Inside clip; a part nothing is left of gets no job.

use i_overlay::core::overlay_rule::OverlayRule;

use crate::clip::Clip;
use crate::grid::{self, Path64};
use crate::protocol::{PartJob, TraceParts};
use crate::request::TraceRequest;

/// How far the box that stands for "everything" reaches past the picture, millimetres: far
/// enough that no traced outline (which never leaves the picture) touches its edge.
pub const BEYOND_MM: f64 = 1.0;

/// What one part asks to keep.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PartArea {
    /// The whole picture.
    Whole,
    /// Its mask, Inside or Outside.
    Mask(Clip),
    /// Nothing.
    Nothing,
}

/// What one part's trace is clipped to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PartClip {
    /// The part adds nothing: it has no area, or later parts cover all of it.
    Nothing,
    /// The whole picture, unclipped.
    Whole,
    /// This clip.
    Region(Clip),
}

impl PartClip {
    /// The clip the trace goes through; `None` for the whole picture and for nothing.
    #[must_use]
    pub fn region(&self) -> Option<&Clip> {
        match self {
            Self::Region(clip) => Some(clip),
            Self::Whole | Self::Nothing => None,
        }
    }

    /// Whether the part adds anything.
    #[must_use]
    pub fn contributes(&self) -> bool {
        !matches!(self, Self::Nothing)
    }
}

/// The clip of each part of `areas` (stacking order, bottom first) over a picture whose box is
/// `picture_mm`.
#[must_use]
pub fn part_clips(areas: &[PartArea], picture_mm: kurbo::Rect) -> Vec<PartClip> {
    let own = |area: &PartArea| match area {
        PartArea::Whole => PartClip::Whole,
        PartArea::Mask(clip) => PartClip::Region(clip.clone()),
        PartArea::Nothing => PartClip::Nothing,
    };
    let mut clips: Vec<PartClip> = areas.iter().map(|_| PartClip::Nothing).collect();
    let Some(everything) = grown_box(picture_mm) else {
        // A picture the grid cannot hold: nothing is stacked, every part keeps its own area.
        return areas.iter().map(own).collect();
    };
    let kept: Vec<(usize, Vec<Path64>)> = areas
        .iter()
        .enumerate()
        .filter_map(|(i, area)| {
            let polygons = match area {
                PartArea::Whole => vec![everything.clone()],
                PartArea::Mask(clip) if !clip.outside() => clip.polygons().to_vec(),
                PartArea::Mask(clip) => grid::canonical(grid::run(
                    OverlayRule::Difference,
                    std::slice::from_ref(&everything),
                    clip.polygons(),
                )),
                PartArea::Nothing => Vec::new(),
            };
            (!polygons.is_empty()).then_some((i, polygons))
        })
        .collect();
    for (k, (i, polygons)) in kept.iter().enumerate() {
        let later: Vec<Path64> = kept[k + 1..]
            .iter()
            .flat_map(|(_, p)| p.iter().cloned())
            .collect();
        if later.is_empty() {
            clips[*i] = own(&areas[*i]);
            continue;
        }
        if grid::run(OverlayRule::Intersect, polygons, &later).is_empty() {
            clips[*i] = own(&areas[*i]);
            continue;
        }
        let visible = grid::canonical(grid::run(OverlayRule::Difference, polygons, &later));
        clips[*i] = if visible.is_empty() {
            PartClip::Nothing
        } else {
            PartClip::Region(Clip::from_polygons(visible, false))
        };
    }
    clips
}

/// The job for the parts that add something: `requests` holds one request per part (its clip is
/// replaced by the part's), `active` is the part whose palette the join keeps. `None` when no part
/// adds anything.
#[must_use]
pub fn parts_job(
    clips: &[PartClip],
    requests: Vec<TraceRequest>,
    active: usize,
) -> Option<TraceParts> {
    let mut jobs = Vec::new();
    let mut palette_of = None;
    for (part, (clip, mut request)) in clips.iter().zip(requests).enumerate() {
        request.clip = match clip {
            PartClip::Nothing => continue,
            PartClip::Whole => None,
            PartClip::Region(c) => Some(c.clone()),
        };
        if part == active || palette_of.is_none() {
            palette_of = Some(jobs.len());
        }
        jobs.push(PartJob {
            // invariant: at most 8 parts.
            part: u8::try_from(part).unwrap_or(u8::MAX),
            request,
            cached: None,
        });
    }
    // invariant: at most 8 jobs.
    palette_of.map(|p| TraceParts {
        jobs,
        palette_of: u8::try_from(p).unwrap_or(0),
    })
}

/// The picture's box grown by [`BEYOND_MM`] as a grid polygon.
fn grown_box(picture: kurbo::Rect) -> Option<Path64> {
    let a = grid::snap(picture.x0 - BEYOND_MM, picture.y0 - BEYOND_MM)?;
    let b = grid::snap(picture.x1 + BEYOND_MM, picture.y1 + BEYOND_MM)?;
    Some(grid::rect_path(
        a.x.min(b.x),
        a.y.min(b.y),
        a.x.max(b.x),
        a.y.max(b.y),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::regions::{MaskMode, PxRect, Region};
    use vectrace::PixelMap;

    /// 1 px = 1 mm.
    fn map() -> PixelMap {
        PixelMap::scale(1.0, 1.0)
    }

    fn mask(x0: i32, y0: i32, x1: i32, y1: i32, mode: MaskMode) -> PartArea {
        let region = Region::rect(PxRect::new(x0, y0, x1, y1), false);
        PartArea::Mask(Clip::of_regions(&[region], mode, map()).unwrap().unwrap())
    }

    fn picture() -> kurbo::Rect {
        kurbo::Rect::new(0.0, 0.0, 100.0, 100.0)
    }

    fn area(clip: &PartClip) -> f64 {
        match clip {
            PartClip::Region(c) => grid::area_mm2(c.polygons()),
            _ => 0.0,
        }
    }

    #[test]
    fn a_single_part_keeps_its_own_area() {
        let clips = part_clips(&[PartArea::Whole], picture());
        assert_eq!(clips, [PartClip::Whole]);
        let clips = part_clips(&[mask(10, 10, 20, 20, MaskMode::Inside)], picture());
        assert!(matches!(clips[0], PartClip::Region(_)));
    }

    #[test]
    fn a_later_part_takes_its_area_out_of_the_earlier_one() {
        let clips = part_clips(
            &[PartArea::Whole, mask(0, 0, 50, 100, MaskMode::Inside)],
            picture(),
        );
        // The earlier part is the grown box (102 x 102) minus the 50 x 100 mask.
        assert!((area(&clips[0]) - (102.0 * 102.0 - 5000.0)).abs() < 1e-6);
        assert!(matches!(&clips[1], PartClip::Region(c) if !c.outside()));
    }

    #[test]
    fn parts_that_do_not_overlap_keep_their_own_clips() {
        // a lies in the hole of b's Outside area: they share nothing.
        let a = mask(60, 60, 70, 70, MaskMode::Inside);
        let b = mask(50, 50, 90, 90, MaskMode::Outside);
        let clips = part_clips(&[a.clone(), b.clone()], picture());
        let (PartArea::Mask(a), PartArea::Mask(b)) = (a, b) else {
            unreachable!()
        };
        assert_eq!(clips, [PartClip::Region(a), PartClip::Region(b)]);
    }

    #[test]
    fn a_part_covered_by_a_later_one_adds_nothing() {
        let clips = part_clips(
            &[
                mask(10, 10, 20, 20, MaskMode::Inside),
                mask(0, 0, 50, 50, MaskMode::Inside),
            ],
            picture(),
        );
        assert_eq!(clips[0], PartClip::Nothing);
        assert!(clips[1].contributes());
    }

    #[test]
    fn parts_without_area_add_nothing_and_do_not_hide_others() {
        let clips = part_clips(
            &[PartArea::Whole, PartArea::Nothing, PartArea::Nothing],
            picture(),
        );
        assert_eq!(
            clips,
            [PartClip::Whole, PartClip::Nothing, PartClip::Nothing]
        );
    }

    #[test]
    fn the_visible_areas_and_the_last_part_tile_the_union_exactly() {
        let areas = [
            PartArea::Whole,
            mask(10, 10, 60, 60, MaskMode::Inside),
            mask(40, 40, 90, 90, MaskMode::Inside),
        ];
        let clips = part_clips(&areas, picture());
        let total: f64 = clips.iter().map(area).sum();
        assert!((total - 102.0 * 102.0).abs() < 1e-6, "{total}");
    }
}
