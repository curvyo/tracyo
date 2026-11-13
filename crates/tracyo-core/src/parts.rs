//! The parts of the open picture: the list, the active part and what each part keeps.
//!
//! The app works on a live copy of the active part; [`Parts`] holds the snapshots. Every change
//! of the list takes the live copy first, so the active snapshot is current when it is read.

use vectrace::PixelMap;

use crate::analysis::Analysis;
use crate::clip::{self, COULD_NOT_COMBINE, MaskPlan, OUTSIDE_IMAGE};
use crate::method::PictureValues;
use crate::regions::{MaskMode, Region};
use crate::request::pixel_map;
use crate::settings::Settings;
use crate::stack::PartArea;

/// At most this many parts (specification 0002, criterion 2).
pub const MAX_PARTS: usize = 8;

/// "0 paths: no part has anything to trace."
pub const NO_PART: &str = "0 paths: no part has anything to trace.";

/// One part: its mask and its own trace values.
#[derive(Debug, Clone, PartialEq)]
pub struct Part {
    /// Which side of the mask it traces.
    pub mode: MaskMode,
    /// The regions (rectangles and lassos) of its mask.
    pub regions: Vec<Region>,
    /// Its settings (Method, Smooth, Output, ...).
    pub settings: Settings,
    /// Its Threshold, Invert, picked colours and palette edits.
    pub values: PictureValues,
    /// What the analysis found for it.
    pub analysis: Analysis,
    /// The maker set the Threshold by hand: a mask does not change it.
    pub threshold_set: bool,
}

impl Part {
    /// The part a new chip starts as: this part's settings and values without picked colours and
    /// palette edits, Inside, no regions.
    #[must_use]
    pub fn copied_for_new(&self) -> Self {
        Self {
            mode: MaskMode::Inside,
            regions: Vec::new(),
            settings: self.settings,
            values: self.values.without_colours(),
            analysis: self.analysis,
            threshold_set: self.threshold_set,
        }
    }

    /// What this part keeps of a `width` x `height` picture, and why not when it keeps nothing.
    fn plan(&self, first: bool, width: u32, height: u32) -> (PartArea, Option<&'static str>) {
        if self.regions.is_empty() {
            return if first {
                (PartArea::Whole, None)
            } else {
                (PartArea::Nothing, None)
            };
        }
        match clip::plan(
            &self.regions,
            self.mode,
            pixel_map(width, width),
            width,
            height,
        ) {
            MaskPlan::Whole => (PartArea::Whole, None),
            MaskPlan::Clip(c) => (PartArea::Mask(c), None),
            MaskPlan::Nothing(text) => (PartArea::Nothing, Some(text)),
        }
    }
}

/// The parts in creation order (bottom first) and the active one.
#[derive(Debug, Clone, PartialEq)]
pub struct Parts {
    parts: Vec<Part>,
    active: usize,
}

impl Parts {
    /// One part, active.
    #[must_use]
    pub fn new(first: Part) -> Self {
        Self {
            parts: vec![first],
            active: 0,
        }
    }

    /// The number of parts, 1 to [`MAX_PARTS`].
    #[must_use]
    pub fn len(&self) -> usize {
        self.parts.len()
    }

    /// Never true: there is always a part.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.parts.is_empty()
    }

    /// The index of the active part.
    #[must_use]
    pub fn active_index(&self) -> usize {
        self.active
    }

    /// The parts as last stored.
    #[must_use]
    pub fn all(&self) -> &[Part] {
        &self.parts
    }

    /// Part 1, whose settings are the remembered ones.
    #[must_use]
    pub fn first(&self) -> &Part {
        &self.parts[0]
    }

    /// Whether Add part still adds.
    #[must_use]
    pub fn can_add(&self) -> bool {
        self.parts.len() < MAX_PARTS
    }

    /// Stores `live` as the active part.
    pub fn store(&mut self, live: Part) {
        self.parts[self.active] = live;
    }

    /// Stores `live`, appends a part copied from it and makes that one active; `None` (nothing
    /// changes) at [`MAX_PARTS`].
    pub fn add(&mut self, live: Part) -> Option<Part> {
        if !self.can_add() {
            return None;
        }
        let new = live.copied_for_new();
        self.store(live);
        self.parts.push(new.clone());
        self.active = self.parts.len() - 1;
        Some(new)
    }

    /// Stores `live` and makes part `index` active; its snapshot, or `None` for an index there
    /// is no part for.
    pub fn select(&mut self, live: Part, index: usize) -> Option<Part> {
        if index >= self.parts.len() {
            return None;
        }
        self.store(live);
        self.active = index;
        Some(self.parts[index].clone())
    }

    /// Removes the active part; the part before it (or the new part 1) becomes active and is
    /// returned. `None` (nothing changes) with one part.
    pub fn remove_active(&mut self) -> Option<Part> {
        if self.parts.len() < 2 {
            return None;
        }
        self.parts.remove(self.active);
        self.active = self.active.saturating_sub(1);
        Some(self.parts[self.active].clone())
    }

    /// What each part keeps of a `width` x `height` picture: part 1 without regions is the whole
    /// picture, a later part without regions or with a mask off the picture nothing.
    #[must_use]
    pub fn areas(&self, width: u32, height: u32) -> Vec<PartArea> {
        self.parts
            .iter()
            .enumerate()
            .map(|(i, part)| part.plan(i == 0, width, height).0)
            .collect()
    }

    /// Why nothing is traced, when no part keeps any of the picture: the active part's own
    /// notice when its regions are the reason, else "no part has anything to trace". `None` when
    /// some part keeps something.
    #[must_use]
    pub fn nothing_notice(&self, width: u32, height: u32) -> Option<&'static str> {
        let none = self
            .parts
            .iter()
            .enumerate()
            .all(|(i, p)| matches!(p.plan(i == 0, width, height).0, PartArea::Nothing));
        if !none {
            return None;
        }
        let active = &self.parts[self.active];
        Some(match active.plan(self.active == 0, width, height).1 {
            Some(OUTSIDE_IMAGE) => OUTSIDE_IMAGE,
            Some(COULD_NOT_COMBINE) => COULD_NOT_COMBINE,
            _ => NO_PART,
        })
    }

    /// Gives part `index` the background and, unless the maker set it, the Threshold that the
    /// analysis of its own area found; whether anything changed. Another part's snapshot only: the
    /// active part's live copy is the app's.
    pub fn take_analysis(&mut self, index: usize, analysis: Analysis) -> bool {
        let Some(part) = self.parts.get_mut(index) else {
            return false;
        };
        let before = (part.values.threshold, part.analysis.suggested_background);
        part.analysis.suggested_background = analysis.suggested_background;
        if !part.threshold_set {
            part.values.threshold = analysis.threshold;
        }
        before != (part.values.threshold, part.analysis.suggested_background)
    }

    /// The mask's part of the hidden status text: "Inside 2 paths", with two or more parts
    /// "3 parts" instead, and last "Part 3 has no paths" for an active later part without regions.
    #[must_use]
    pub fn announcement(&self) -> String {
        let active = &self.parts[self.active];
        let mut segments = Vec::new();
        if self.parts.len() >= 2 {
            segments.push(format!("{} parts", self.parts.len()));
        } else if !active.regions.is_empty() {
            let n = active.regions.len();
            segments.push(format!(
                "{} {n} {}",
                active.mode.label(),
                if n == 1 { "path" } else { "paths" }
            ));
        }
        if self.parts.len() >= 2 && self.active > 0 && active.regions.is_empty() {
            segments.push(format!("Part {} has no paths", self.active + 1));
        }
        segments.join(". ")
    }

    /// The chip's tooltip text of part `index`: "Part 1 of 3, inside, 2 paths".
    #[must_use]
    pub fn chip_name(&self, index: usize) -> String {
        let part = &self.parts[index];
        let what = if part.regions.is_empty() {
            if index == 0 {
                "whole image".to_owned()
            } else {
                "no paths".to_owned()
            }
        } else {
            let n = part.regions.len();
            format!(
                "{}, {n} {}",
                part.mode.label().to_lowercase(),
                if n == 1 { "path" } else { "paths" }
            )
        };
        format!("Part {} of {}, {what}", index + 1, self.parts.len())
    }
}

/// The pixel map of the full picture of `width` px.
#[must_use]
pub fn full_map(width: u32) -> PixelMap {
    pixel_map(width, width)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::regions::PxRect;

    fn part() -> Part {
        Part {
            mode: MaskMode::Inside,
            regions: Vec::new(),
            settings: Settings::default(),
            values: PictureValues::new(128),
            analysis: Analysis {
                suggested_background: None,
                method: crate::method::Method::Brightness,
                threshold: 128,
                tolerance: 40,
            },
            threshold_set: false,
        }
    }

    /// Add part with the stored active part as the live copy.
    fn add(parts: &mut Parts) -> Option<Part> {
        parts.add(parts.all()[parts.active_index()].clone())
    }

    fn region(x0: i32, y0: i32, x1: i32, y1: i32) -> Region {
        Region::rect(PxRect::new(x0, y0, x1, y1), false)
    }

    #[test]
    fn add_copies_the_settings_but_not_the_colours_the_regions_or_the_mode() {
        let mut live = part();
        live.settings.smooth = 77;
        live.mode = MaskMode::Outside;
        live.regions.push(region(0, 0, 5, 5));
        live.values.threshold = 99;
        live.values.pick(vectrace::Rgb8::new(1, 2, 3), false);
        let mut parts = Parts::new(part());
        let new = parts.add(live.clone()).unwrap();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts.active_index(), 1);
        assert_eq!(new.settings.smooth, 77);
        assert_eq!(new.values.threshold, 99);
        assert_eq!(new.values.swatches.len(), 0);
        assert_eq!(new.regions.len(), 0);
        assert_eq!(new.mode, MaskMode::Inside);
        assert_eq!(parts.first(), &live, "the live copy was stored as part 1");
    }

    #[test]
    fn there_are_at_most_eight_parts() {
        let mut parts = Parts::new(part());
        for _ in 1..MAX_PARTS {
            assert!(add(&mut parts).is_some());
        }
        assert!(!parts.can_add());
        assert!(add(&mut parts).is_none());
        assert_eq!(parts.len(), MAX_PARTS);
    }

    #[test]
    fn removing_makes_the_part_before_active_and_keeps_the_last_part() {
        let mut parts = Parts::new(part());
        let mut second = part();
        second.settings.smooth = 1;
        let mut third = part();
        third.settings.smooth = 2;
        add(&mut parts);
        parts.store(second.clone());
        add(&mut parts);
        parts.store(third);
        let now = parts.remove_active().unwrap();
        assert_eq!((parts.len(), parts.active_index()), (2, 1));
        assert_eq!(now.settings.smooth, 1, "the part before");
        let now = parts.select(now, 0).unwrap();
        assert_eq!(parts.active_index(), 0);
        let after = parts.remove_active().unwrap();
        assert_eq!((parts.len(), parts.active_index()), (1, 0));
        assert_eq!(after.settings.smooth, 1, "the old part 2 is part 1 now");
        assert_eq!(now.settings, Settings::default());
        assert!(parts.remove_active().is_none());
    }

    #[test]
    fn select_stores_the_live_part_and_returns_the_other() {
        let mut parts = Parts::new(part());
        add(&mut parts);
        let mut live = part();
        live.settings.smooth = 5;
        let first = parts.select(live.clone(), 0).unwrap();
        assert_eq!(first.settings.smooth, Settings::default().smooth);
        assert_eq!(parts.all()[1], live);
        assert!(parts.select(part(), 2).is_none());
    }

    #[test]
    fn part_areas_whole_mask_and_nothing() {
        let mut parts = Parts::new(part());
        let mut second = part();
        second.regions.push(region(10, 10, 50, 50));
        let mut third = part();
        third.regions.push(region(500, 500, 600, 600));
        add(&mut parts);
        parts.store(second);
        add(&mut parts);
        parts.store(third);
        add(&mut parts);
        let areas = parts.areas(100, 100);
        assert!(matches!(areas[0], PartArea::Whole));
        assert!(matches!(areas[1], PartArea::Mask(_)));
        assert!(matches!(areas[2], PartArea::Nothing), "off the picture");
        assert!(matches!(areas[3], PartArea::Nothing), "no region");
    }

    #[test]
    fn the_notice_names_the_reason_nothing_is_traced() {
        let mut off = part();
        off.regions.push(region(500, 500, 600, 600));
        let mut parts = Parts::new(off);
        assert_eq!(parts.nothing_notice(100, 100), Some(OUTSIDE_IMAGE));
        add(&mut parts);
        assert_eq!(parts.nothing_notice(100, 100), Some(NO_PART));
        let mut live = part();
        live.regions.push(region(1, 1, 9, 9));
        parts.store(live);
        assert_eq!(parts.nothing_notice(100, 100), None);
        assert_eq!(Parts::new(part()).nothing_notice(100, 100), None);
    }

    #[test]
    fn a_part_takes_the_analysis_of_its_own_area_unless_the_threshold_was_set() {
        let found = Analysis {
            suggested_background: Some(vectrace::Rgb8::new(0xD8, 0xD8, 0xD8)),
            method: crate::method::Method::Brightness,
            threshold: 77,
            tolerance: 40,
        };
        let mut parts = Parts::new(part());
        add(&mut parts);
        assert!(parts.take_analysis(0, found));
        assert_eq!(parts.all()[0].values.threshold, 77);
        assert_eq!(
            parts.all()[0].analysis.suggested_background,
            found.suggested_background
        );
        assert!(!parts.take_analysis(0, found), "nothing new");
        let mut typed = part();
        typed.values.threshold = 33;
        typed.threshold_set = true;
        let mut parts = Parts::new(typed);
        assert!(parts.take_analysis(0, found), "the background changed");
        assert_eq!(parts.all()[0].values.threshold, 33, "the typed value stays");
        assert!(!parts.take_analysis(5, found), "no such part");
    }

    #[test]
    fn the_hidden_status_text_names_the_mask_the_parts_and_an_empty_part() {
        let mut live = part();
        let mut parts = Parts::new(live.clone());
        assert_eq!(parts.announcement(), "");
        live.regions = vec![region(0, 0, 5, 5), region(9, 9, 20, 20)];
        parts.store(live);
        assert_eq!(parts.announcement(), "Inside 2 paths");
        let mut outside = part();
        outside.mode = MaskMode::Outside;
        outside.regions = vec![region(0, 0, 5, 5)];
        let mut single = Parts::new(outside);
        assert_eq!(single.announcement(), "Outside 1 path");
        add(&mut single);
        assert_eq!(single.announcement(), "2 parts. Part 2 has no paths");
        single.select(single.all()[1].clone(), 0);
        assert_eq!(single.announcement(), "2 parts");
    }

    #[test]
    fn chips_are_named_as_curvyo_names_them() {
        let mut parts = Parts::new(part());
        assert_eq!(parts.chip_name(0), "Part 1 of 1, whole image");
        let mut second = part();
        second.mode = MaskMode::Outside;
        second.regions.push(region(0, 0, 5, 5));
        add(&mut parts);
        parts.store(second);
        add(&mut parts);
        assert_eq!(parts.chip_name(0), "Part 1 of 3, whole image");
        assert_eq!(parts.chip_name(1), "Part 2 of 3, outside, 1 path");
        assert_eq!(parts.chip_name(2), "Part 3 of 3, no paths");
    }
}
