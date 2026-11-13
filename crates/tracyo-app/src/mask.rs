//! The mask of the one part: its rectangles, the tools that draw them and what a change of them
//! does to the trace and to the values the analysis proposes.

use std::sync::mpsc::channel;

use eframe::egui;
use kurbo::Point;
use tracyo_core::analysis::Analysis;
use tracyo_core::clip::{self, Clip, MaskPlan};
use tracyo_core::lasso::Draft;
use tracyo_core::region_edit::Handle;
use tracyo_core::regions::{MaskMode, PxRect, Region};
use tracyo_core::request::{pixel_map, size_mm};
use tracyo_core::stack::{PartClip, part_clips};
use tracyo_core::wash;

use crate::app::App;

/// How many clips' analyses the app keeps.
pub(crate) const CLIP_ANALYSES: usize = 32;

/// The region tool that is pressed: at most one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Tool {
    /// Neither: the pointer selects and edits regions.
    None,
    /// Drags draw rectangles.
    Rectangle,
    /// Clicks set the corners of a lasso.
    Lasso,
}

/// A drag on the canvas that makes or changes a region.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Drag {
    /// A new rectangle from `from` to the pointer (picture pixels).
    Drawing { from: Point, to: Point },
    /// Region `index` moved with the pointer, which pressed at `from`; `start` is how it was.
    Moving {
        index: usize,
        start: Region,
        from: Point,
    },
    /// Rectangle `index` resized by `handle`; `start` is how it was.
    Resizing {
        index: usize,
        handle: Handle,
        start: PxRect,
    },
    /// Corner `corner` of lasso `index` moved with the pointer; `start` is how the lasso was.
    Corner {
        index: usize,
        corner: usize,
        start: Region,
    },
}

/// The regions of the part, how they apply, and the tools that edit them.
pub(crate) struct MaskState {
    pub regions: Vec<Region>,
    pub mode: MaskMode,
    pub selected: Option<usize>,
    /// The corner of the selected lasso that was pressed last.
    pub selected_corner: Option<usize>,
    /// The corner of a lasso (region, corner) under the primary button while it is down.
    pub pressed_corner: Option<(usize, usize)>,
    /// The pressed region tool.
    pub tool: Tool,
    /// The Subtract toggle is pressed.
    pub subtract: bool,
    pub drag: Option<Drag>,
    /// The lasso being drawn.
    pub draft: Draft,
    /// When and where the last corner of the draft was pressed, to tell a double click.
    pub last_press: Option<(f64, egui::Pos2)>,
    /// What the regions ask of the trace.
    pub plan: MaskPlan,
    /// The picture area the trace leaves out, washed to 12 %, as triangles in picture pixels.
    pub wash: Vec<[Point; 3]>,
}

impl MaskState {
    /// No region, Inside, both tools released.
    pub fn new() -> Self {
        Self {
            regions: Vec::new(),
            mode: MaskMode::Inside,
            selected: None,
            selected_corner: None,
            pressed_corner: None,
            tool: Tool::None,
            subtract: false,
            drag: None,
            draft: Draft::new(),
            last_press: None,
            plan: MaskPlan::Whole,
            wash: Vec::new(),
        }
    }

    /// Recomputes the plan and the wash for a picture of `width` x `height` px.
    pub fn recompute(&mut self, width: u32, height: u32) {
        self.plan = clip::plan(
            &self.regions,
            self.mode,
            pixel_map(width, width),
            width,
            height,
        );
        self.wash = wash::not_traced(&self.regions, self.mode, PxRect::of_picture(width, height));
    }

    /// The region as it was before the drag that moves or changes it, drawn as its ghost.
    pub fn drag_ghost(&self) -> Option<Region> {
        match self.drag.as_ref()? {
            Drag::Moving { start, .. } | Drag::Corner { start, .. } => Some(start.clone()),
            Drag::Resizing { start, .. } => Some(Region::rect(*start, false)),
            Drag::Drawing { .. } => None,
        }
    }
}

impl App {
    /// The regions, the mode or the picture's mask changed: plan again, read the values of the
    /// kept pixels, trace.
    pub(crate) fn mask_changed(&mut self, ctx: &egui::Context) {
        let Some(doc) = &mut self.doc else { return };
        let (w, h) = (doc.picture.width, doc.picture.height);
        doc.mask.recompute(w, h);
        if let Some(i) = doc.mask.selected
            && i >= doc.mask.regions.len()
        {
            doc.mask.selected = None;
        }
        self.analyse_kept(ctx, false);
        self.start_trace(ctx, false, false);
    }

    /// Clear mask: every region and a lasso in progress go; the mode, the tools and the settings
    /// stay.
    pub(crate) fn clear_mask(&mut self, ctx: &egui::Context) {
        let Some(doc) = &mut self.doc else { return };
        doc.mask.draft = Draft::new();
        doc.mask.last_press = None;
        if doc.mask.regions.is_empty() {
            return;
        }
        doc.mask.regions.clear();
        doc.mask.selected = None;
        doc.mask.selected_corner = None;
        doc.mask.drag = None;
        self.mask_changed(ctx);
    }

    /// Inside or Outside.
    pub(crate) fn set_mask_mode(&mut self, ctx: &egui::Context, mode: MaskMode) {
        let Some(doc) = &mut self.doc else { return };
        if doc.mask.mode == mode {
            return;
        }
        doc.mask.mode = mode;
        self.mask_changed(ctx);
    }

    /// Finishes the lasso in progress: with three or more corners and an area it becomes a
    /// region (subtracted while Subtract is pressed), closed by a straight side from the last
    /// corner to the first; else nothing is stored. Returns whether a region was made.
    pub(crate) fn finish_lasso(&mut self, ctx: &egui::Context) -> bool {
        let Some(doc) = &mut self.doc else {
            return false;
        };
        if doc.mask.draft.is_empty() {
            return false;
        }
        let region = doc.mask.draft.finish(doc.mask.subtract);
        doc.mask.draft = Draft::new();
        doc.mask.last_press = None;
        let Some(region) = region else { return false };
        doc.mask.regions.push(region);
        self.mask_changed(ctx);
        true
    }

    /// Esc: the lasso in progress is thrown away, nothing is stored.
    pub(crate) fn abort_lasso(&mut self) {
        if let Some(doc) = &mut self.doc {
            doc.mask.draft = Draft::new();
            doc.mask.last_press = None;
        }
    }

    /// Presses or releases a region tool; a lasso in progress is finished first.
    pub(crate) fn set_tool(&mut self, ctx: &egui::Context, tool: Tool) {
        self.finish_lasso(ctx);
        if let Some(doc) = &mut self.doc {
            doc.mask.tool = if doc.mask.tool == tool {
                Tool::None
            } else {
                tool
            };
        }
    }

    /// Delete and Backspace: the selected region goes.
    pub(crate) fn delete_selected(&mut self, ctx: &egui::Context) {
        let Some(doc) = &mut self.doc else { return };
        let Some(i) = doc.mask.selected.take() else {
            return;
        };
        doc.mask.selected_corner = None;
        if i < doc.mask.regions.len() {
            doc.mask.regions.remove(i);
            self.mask_changed(ctx);
        }
    }

    /// The arrow keys: the selected region moves by `dx`, `dy` picture pixels.
    pub(crate) fn nudge_selected(&mut self, ctx: &egui::Context, dx: i32, dy: i32) {
        let Some(doc) = &mut self.doc else { return };
        let Some(region) = doc.mask.selected.and_then(|i| doc.mask.regions.get_mut(i)) else {
            return;
        };
        *region = region.moved(dx, dy);
        self.mask_changed(ctx);
    }

    /// Esc during a drag: the rectangle goes back where it was, or the new one is dropped.
    /// Returns whether a drag was running.
    pub(crate) fn cancel_drag(&mut self) -> bool {
        let Some(doc) = &mut self.doc else {
            return false;
        };
        let Some(drag) = doc.mask.drag.take() else {
            return false;
        };
        match drag {
            Drag::Moving { index, start, .. } | Drag::Corner { index, start, .. } => {
                if let Some(r) = doc.mask.regions.get_mut(index) {
                    *r = start;
                }
            }
            Drag::Resizing { index, start, .. } => {
                if let Some(r) = doc.mask.regions.get_mut(index) {
                    *r = Region::rect(start, r.subtract);
                }
            }
            Drag::Drawing { .. } => {}
        }
        true
    }

    /// The background colour Remove background's Auto uses now (`None`: a transparent one).
    #[must_use]
    pub fn auto_background(&self) -> Option<vectrace::Rgb8> {
        let doc = self.doc.as_ref()?;
        doc.values.background(doc.analysis.suggested_background)
    }

    /// Whether an analysis of the picture is still being read on its thread.
    #[must_use]
    pub fn is_analysing(&self) -> bool {
        self.analysing.is_some() || self.region_analysing.is_some()
    }

    /// The notice that the dimmed Save as… tooltip shows for the mask, if the mask traces nothing.
    #[must_use]
    pub fn mask_notice(&self) -> Option<&'static str> {
        match self.doc.as_ref()?.mask.plan {
            MaskPlan::Nothing(text) => Some(text),
            _ => None,
        }
    }

    /// The bounding boxes of the regions of the mask: (pixels, subtracted).
    #[must_use]
    pub fn mask_regions(&self) -> Vec<(PxRect, bool)> {
        self.doc.as_ref().map_or_else(Vec::new, |d| {
            d.mask
                .regions
                .iter()
                .map(|r| (r.bounds(), r.subtract))
                .collect()
        })
    }

    /// The regions of the mask as they are (rectangles and lassos).
    #[must_use]
    pub fn mask_shapes(&self) -> Vec<Region> {
        self.doc
            .as_ref()
            .map_or_else(Vec::new, |d| d.mask.regions.clone())
    }

    /// Whether the Lasso tool is pressed.
    #[must_use]
    pub fn lasso_pressed(&self) -> bool {
        self.doc
            .as_ref()
            .is_some_and(|d| d.mask.tool == Tool::Lasso)
    }

    /// The corners of the lasso being drawn, in picture pixels.
    #[must_use]
    pub fn lasso_in_progress(&self) -> Vec<(i32, i32)> {
        self.doc.as_ref().map_or_else(Vec::new, |d| {
            d.mask.draft.corners().iter().map(|c| (c.x, c.y)).collect()
        })
    }

    /// The corner of the selected lasso that was pressed last.
    #[must_use]
    pub fn selected_corner(&self) -> Option<usize> {
        self.doc.as_ref()?.mask.selected_corner
    }

    /// The mask mode of the shown picture.
    #[must_use]
    pub fn mask_mode(&self) -> Option<MaskMode> {
        self.doc.as_ref().map(|d| d.mask.mode)
    }

    /// Whether the Rectangle tool is pressed, and the Subtract toggle.
    #[must_use]
    pub fn mask_tools(&self) -> (bool, bool) {
        self.doc.as_ref().map_or((false, false), |d| {
            (d.mask.tool == Tool::Rectangle, d.mask.subtract)
        })
    }

    /// The bounding box of the selected region.
    #[must_use]
    pub fn selected_region(&self) -> Option<PxRect> {
        let d = self.doc.as_ref()?;
        d.mask.regions.get(d.mask.selected?).map(Region::bounds)
    }

    /// The selected region as it is.
    #[must_use]
    pub fn selected_shape(&self) -> Option<Region> {
        let d = self.doc.as_ref()?;
        d.mask.regions.get(d.mask.selected?).cloned()
    }

    /// The picture area the active part does not trace (the extra wash), as triangles in picture
    /// pixels.
    #[must_use]
    pub fn washed_triangles(&self) -> Vec<[Point; 3]> {
        self.doc
            .as_ref()
            .map_or_else(Vec::new, |d| d.mask.wash.clone())
    }

    // ---- analysis of the area each part keeps ----

    /// What each part keeps once the later parts are taken out (the live part stored first).
    fn visible_clips(&mut self) -> Vec<PartClip> {
        let Some(live) = self.live_part() else {
            return Vec::new();
        };
        let Some(doc) = self.doc.as_mut() else {
            return Vec::new();
        };
        doc.parts.store(live);
        let (w, h) = (doc.picture.width, doc.picture.height);
        part_clips(
            &doc.parts.areas(w, h),
            kurbo::Rect::new(0.0, 0.0, size_mm(w), size_mm(h)),
        )
    }

    /// Reads the values of the pixels each part keeps on a thread, once per clip (a clip that was
    /// read before comes from the cache; the whole picture's values are those of the picture).
    /// With `full` the active part's answer replaces every analysed value, as the Analyse button
    /// does; else only what the maker has not set.
    pub(crate) fn analyse_kept(&mut self, ctx: &egui::Context, full: bool) {
        let clips = self.visible_clips();
        let Some(doc) = &self.doc else { return };
        let active = doc.parts.active_index();
        let active_whole = matches!(clips.get(active), Some(PartClip::Whole));
        let missing: Vec<Clip> = clips
            .iter()
            .filter_map(PartClip::region)
            .filter(|c| !self.clip_analyses.iter().any(|(known, _)| known == *c))
            .fold(Vec::new(), |mut all, c| {
                if !all.contains(c) {
                    all.push(c.clone());
                }
                all
            });
        if missing.is_empty() {
            self.region_analysing = None;
            self.apply_analyses(ctx, &clips, full);
            return;
        }
        let picture = std::sync::Arc::clone(&doc.picture);
        let repaint = ctx.clone();
        let (tx, rx) = channel();
        std::thread::spawn(move || {
            let map = pixel_map(picture.width, picture.width);
            let found: Vec<(Clip, Analysis)> = missing
                .into_iter()
                .map(|c| {
                    let analysis = c
                        .kept_pixels(map, picture.width, picture.height)
                        .map_or_else(
                            || Analysis::of(&picture),
                            |k| Analysis::of_kept(&picture, &k),
                        );
                    (c, analysis)
                })
                .collect();
            let _ = tx.send(found);
            repaint.request_repaint();
        });
        // With the whole picture as the active part's area the Analyse answer comes from its own
        // thread, not from this one.
        self.region_analysing = Some((rx, full && !active_whole));
        if active_whole {
            // The active part's own values stand for the whole picture now: take them while the
            // others are read.
            self.apply_analyses(ctx, &clips, full);
        }
    }

    /// Applies an answer of [`Self::analyse_kept`] that has arrived.
    pub(crate) fn finish_region_analysis(&mut self, ctx: &egui::Context) {
        let Some((rx, full)) = &self.region_analysing else {
            return;
        };
        let full = *full;
        let found = match rx.try_recv() {
            Ok(found) => found,
            Err(std::sync::mpsc::TryRecvError::Empty) => return,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                self.region_analysing = None;
                return;
            }
        };
        self.region_analysing = None;
        for (clip, analysis) in found {
            self.clip_analyses.retain(|(known, _)| *known != clip);
            self.clip_analyses.push((clip, analysis));
            if self.clip_analyses.len() > CLIP_ANALYSES {
                self.clip_analyses.remove(0);
            }
        }
        let clips = self.visible_clips();
        self.apply_analyses(ctx, &clips, full);
    }

    /// Gives every part the analysis of its own area (the clips are read from the cache; a part
    /// whose clip is not there yet keeps what it has); a changed value traces again.
    fn apply_analyses(&mut self, ctx: &egui::Context, clips: &[PartClip], full: bool) {
        let Some(doc) = &mut self.doc else { return };
        let active = doc.parts.active_index();
        let mut changed = false;
        let mut active_analysis = None;
        for (i, clip) in clips.iter().enumerate() {
            let (analysis, whole) = match clip {
                PartClip::Nothing => continue,
                PartClip::Whole => (doc.whole_analysis, true),
                PartClip::Region(c) => {
                    let Some((_, a)) = self.clip_analyses.iter().find(|(known, _)| known == c)
                    else {
                        continue;
                    };
                    (*a, false)
                }
            };
            if i == active {
                active_analysis = Some((analysis, whole));
            } else {
                changed |= doc.parts.take_analysis(i, analysis);
            }
        }
        match active_analysis {
            Some((_, true)) if full => self.analyse_whole(ctx),
            Some((analysis, _)) if full => self.apply_full_analysis(ctx, analysis, false),
            Some((analysis, _)) => changed |= self.take_region_analysis(analysis),
            None => {}
        }
        if changed {
            self.start_trace(ctx, false, false);
        }
    }

    /// Takes the Threshold (unless the maker set it) and the Auto background of `analysis` for
    /// the active part; whether a value changed.
    fn take_region_analysis(&mut self, analysis: Analysis) -> bool {
        let Some(doc) = &mut self.doc else {
            return false;
        };
        let before = (doc.values.threshold, doc.analysis.suggested_background);
        doc.analysis.suggested_background = analysis.suggested_background;
        if !doc.threshold_set {
            doc.values.threshold = analysis.threshold;
        }
        before != (doc.values.threshold, doc.analysis.suggested_background)
    }
}
