//! The mask of the one part: its rectangles, the tools that draw them and what a change of them
//! does to the trace and to the values the analysis proposes.

use std::sync::mpsc::channel;

use eframe::egui;
use kurbo::Point;
use tracyo_core::analysis::Analysis;
use tracyo_core::clip::{self, MaskPlan};
use tracyo_core::region_edit::{self, Handle};
use tracyo_core::regions::{MaskMode, PxRect, Region};
use tracyo_core::request::pixel_map;
use tracyo_core::wash;

use crate::app::App;

/// A drag on the canvas that makes or changes a rectangle.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Drag {
    /// A new rectangle from `from` to the pointer (picture pixels).
    Drawing { from: Point, to: Point },
    /// Region `index` moved with the pointer, which pressed at `from`; `start` is where it was.
    Moving {
        index: usize,
        start: PxRect,
        from: Point,
    },
    /// Region `index` resized by `handle`; `start` is how it was.
    Resizing {
        index: usize,
        handle: Handle,
        start: PxRect,
    },
}

/// The rectangles of the part, how they apply, and the tools that edit them.
pub(crate) struct MaskState {
    pub regions: Vec<Region>,
    pub mode: MaskMode,
    pub selected: Option<usize>,
    /// The Rectangle tool is pressed.
    pub rectangle: bool,
    /// The Subtract toggle is pressed.
    pub subtract: bool,
    pub drag: Option<Drag>,
    /// What the regions ask of the trace.
    pub plan: MaskPlan,
    /// The picture area the trace leaves out, washed to 12 %.
    pub wash: Vec<PxRect>,
}

impl MaskState {
    /// No region, Inside, both tools released.
    pub fn new() -> Self {
        Self {
            regions: Vec::new(),
            mode: MaskMode::Inside,
            selected: None,
            rectangle: false,
            subtract: false,
            drag: None,
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

    /// The region that a drag shows moved or resized, or the region as it is.
    pub fn drag_ghost(&self) -> Option<PxRect> {
        match self.drag? {
            Drag::Moving { start, .. } | Drag::Resizing { start, .. } => Some(start),
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

    /// Clear mask: every region goes; the mode, the tools and the settings stay.
    pub(crate) fn clear_mask(&mut self, ctx: &egui::Context) {
        let Some(doc) = &mut self.doc else { return };
        if doc.mask.regions.is_empty() {
            return;
        }
        doc.mask.regions.clear();
        doc.mask.selected = None;
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

    /// Delete and Backspace: the selected region goes.
    pub(crate) fn delete_selected(&mut self, ctx: &egui::Context) {
        let Some(doc) = &mut self.doc else { return };
        let Some(i) = doc.mask.selected.take() else {
            return;
        };
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
        region.rect = region_edit::moved(region.rect, dx, dy);
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
            Drag::Moving { index, start, .. } | Drag::Resizing { index, start, .. } => {
                if let Some(r) = doc.mask.regions.get_mut(index) {
                    r.rect = start;
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

    /// The rectangles of the mask: (pixels, subtracted).
    #[must_use]
    pub fn mask_regions(&self) -> Vec<(PxRect, bool)> {
        self.doc.as_ref().map_or_else(Vec::new, |d| {
            d.mask
                .regions
                .iter()
                .map(|r| (r.rect, r.subtract))
                .collect()
        })
    }

    /// The mask mode of the shown picture.
    #[must_use]
    pub fn mask_mode(&self) -> Option<MaskMode> {
        self.doc.as_ref().map(|d| d.mask.mode)
    }

    /// Whether the Rectangle tool is pressed, and the Subtract toggle.
    #[must_use]
    pub fn mask_tools(&self) -> (bool, bool) {
        self.doc
            .as_ref()
            .map_or((false, false), |d| (d.mask.rectangle, d.mask.subtract))
    }

    /// The selected region's rectangle.
    #[must_use]
    pub fn selected_region(&self) -> Option<PxRect> {
        let d = self.doc.as_ref()?;
        d.mask.regions.get(d.mask.selected?).map(|r| r.rect)
    }

    /// The picture area the active part does not trace (the extra wash).
    #[must_use]
    pub fn washed_rects(&self) -> Vec<PxRect> {
        self.doc
            .as_ref()
            .map_or_else(Vec::new, |d| d.mask.wash.clone())
    }

    // ---- analysis of the kept pixels ----

    /// Reads the values of the pixels the mask keeps on a thread (or, with no mask, takes the
    /// whole picture's back). With `full` the answer replaces every analysed value, as the Analyse
    /// button does; else only what the maker has not set.
    pub(crate) fn analyse_kept(&mut self, ctx: &egui::Context, full: bool) {
        let Some(doc) = &mut self.doc else { return };
        let kept = match &doc.mask.plan {
            MaskPlan::Clip(c) => {
                let map = pixel_map(doc.picture.width, doc.picture.width);
                c.kept_pixels(map, doc.picture.width, doc.picture.height)
            }
            _ => None,
        };
        let Some(kept) = kept else {
            self.region_analysing = None;
            let whole = doc.whole_analysis;
            if full {
                self.analyse_whole(ctx);
            } else {
                self.take_region_analysis(ctx, whole);
            }
            return;
        };
        let picture = std::sync::Arc::clone(&doc.picture);
        let repaint = ctx.clone();
        let (tx, rx) = channel();
        std::thread::spawn(move || {
            let _ = tx.send(Analysis::of_kept(&picture, &kept));
            repaint.request_repaint();
        });
        self.region_analysing = Some((rx, full));
    }

    /// Applies an answer of [`Self::analyse_kept`] that has arrived.
    pub(crate) fn finish_region_analysis(&mut self, ctx: &egui::Context) {
        let Some((rx, full)) = &self.region_analysing else {
            return;
        };
        let full = *full;
        let analysis = match rx.try_recv() {
            Ok(analysis) => analysis,
            Err(std::sync::mpsc::TryRecvError::Empty) => return,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                self.region_analysing = None;
                return;
            }
        };
        self.region_analysing = None;
        if full {
            self.apply_full_analysis(ctx, analysis);
        } else {
            self.take_region_analysis(ctx, analysis);
        }
    }

    /// Takes the Threshold (unless the maker set it) and the Auto background of `analysis`; a
    /// changed value traces again.
    fn take_region_analysis(&mut self, ctx: &egui::Context, analysis: Analysis) {
        let Some(doc) = &mut self.doc else { return };
        let before = (doc.values.threshold, doc.analysis.suggested_background);
        doc.analysis.suggested_background = analysis.suggested_background;
        if !doc.threshold_set {
            doc.values.threshold = analysis.threshold;
        }
        if before != (doc.values.threshold, doc.analysis.suggested_background) {
            self.start_trace(ctx, false, false);
        }
    }
}
