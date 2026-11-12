//! The parts of the open picture on the app side: the live copy of the active part, the changes
//! of the list, the cache of finished parts and the job all parts make together.

use eframe::egui;
use tracyo_core::parts::Part;
use tracyo_core::protocol::{TraceParts, encode_trace};
use tracyo_core::request::{self, TraceRequest, size_mm};
use tracyo_core::settings::Settings;
use tracyo_core::stack::{part_clips, parts_job};

use crate::app::{App, TextState};

/// The finished parts the app keeps (ADR 0008 section 4).
pub(crate) const CACHE_LIMIT: usize = 32;

/// Finished parts by the bytes of the job that made them, the least recently used going first.
#[derive(Default)]
pub(crate) struct PartCache {
    entries: Vec<(Vec<u8>, Vec<f64>)>,
}

impl PartCache {
    /// The wire under `key`, which becomes the most recent.
    pub fn get(&mut self, key: &[u8]) -> Option<Vec<f64>> {
        let at = self.entries.iter().position(|(k, _)| k == key)?;
        let entry = self.entries.remove(at);
        let wire = entry.1.clone();
        self.entries.push(entry);
        Some(wire)
    }

    /// Keeps `wire` under `key`.
    pub fn insert(&mut self, key: Vec<u8>, wire: Vec<f64>) {
        self.entries.retain(|(k, _)| *k != key);
        self.entries.push((key, wire));
        if self.entries.len() > CACHE_LIMIT {
            self.entries.remove(0);
        }
    }

    /// How many parts are kept.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Forgets everything (a new picture).
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

/// The key of each part that is traced in a job, by part number.
pub(crate) type PartKeys = Vec<(u8, Vec<u8>)>;

impl App {
    /// The remembered settings: part 1's, with the window and the Advanced block as they are now.
    pub(crate) fn remembered(&self) -> Settings {
        let mut s = match &self.doc {
            Some(d) if d.parts.active_index() != 0 => d.parts.first().settings,
            _ => self.settings,
        };
        s.advanced_open = self.settings.advanced_open;
        s.window = self.settings.window;
        s
    }

    /// Writes the remembered settings.
    pub(crate) fn save_settings(&self) {
        self.config.save(&self.remembered());
    }

    /// The active part as the bar and the mask hold it now.
    pub(crate) fn live_part(&self) -> Option<Part> {
        let doc = self.doc.as_ref()?;
        Some(Part {
            mode: doc.mask.mode,
            regions: doc.mask.regions.clone(),
            settings: self.settings,
            values: doc.values.clone(),
            analysis: doc.analysis,
            threshold_set: doc.threshold_set,
        })
    }

    /// Puts `part` into the bar and the mask; the window and the Advanced block stay as they are.
    fn load_part(&mut self, part: Part) {
        let (advanced_open, window) = (self.settings.advanced_open, self.settings.window);
        self.settings = part.settings;
        self.settings.advanced_open = advanced_open;
        self.settings.window = window;
        self.pipette = crate::app::Pipette::Off;
        self.picked_stop = 0;
        self.palette_stop = 0;
        self.fields
            .iter_mut()
            .for_each(crate::widgets::FieldState::cancel_entry);
        if let Some(doc) = &mut self.doc {
            doc.values = part.values;
            doc.analysis = part.analysis;
            doc.threshold_set = part.threshold_set;
            doc.mask.mode = part.mode;
            doc.mask.regions = part.regions;
            doc.mask.selected = None;
            doc.mask.drag = None;
            let (w, h) = (doc.picture.width, doc.picture.height);
            doc.mask.recompute(w, h);
        }
    }

    /// Whether a palette shown for the active part is another one after a change of the active
    /// part (a trace follows then).
    fn palette_differs(before: &Settings, after: &Settings) -> bool {
        before.multicolour || after.multicolour
    }

    /// Add part: a part copied from the active one, without regions, Inside, becomes active.
    pub(crate) fn add_part(&mut self, ctx: &egui::Context) {
        let Some(live) = self.live_part() else { return };
        let Some(doc) = &mut self.doc else { return };
        let Some(new) = doc.parts.add(live) else {
            return;
        };
        self.trash = None;
        self.load_part(new);
        self.start_trace(ctx, false, false);
    }

    /// A click, an arrow, Home or End on a chip: part `index` becomes active.
    pub(crate) fn select_part(&mut self, ctx: &egui::Context, index: usize) {
        let Some(live) = self.live_part() else { return };
        let Some(doc) = &mut self.doc else { return };
        if index == doc.parts.active_index() {
            return;
        }
        let before = self.settings;
        let Some(next) = doc.parts.select(live, index) else {
            return;
        };
        self.trash = None;
        self.load_part(next);
        if Self::palette_differs(&before, &self.settings) {
            self.start_trace(ctx, false, false);
        }
    }

    /// The trash: the active part goes, the part before it becomes active.
    pub(crate) fn remove_part(&mut self, ctx: &egui::Context) {
        let Some(live) = self.live_part() else { return };
        let Some(doc) = &mut self.doc else { return };
        doc.parts.store(live);
        let Some(next) = doc.parts.remove_active() else {
            return;
        };
        self.trash = None;
        self.load_part(next);
        self.start_trace(ctx, false, false);
    }

    /// The notice when no part keeps any of the picture (the live copy is stored first).
    pub(crate) fn nothing_notice(&mut self) -> Option<&'static str> {
        let live = self.live_part()?;
        let doc = self.doc.as_mut()?;
        doc.parts.store(live);
        doc.parts
            .nothing_notice(doc.picture.width, doc.picture.height)
    }

    /// The request that traces `part` of the shown picture.
    pub(crate) fn trace_request(&self, part: &Part, reduced: bool) -> Option<TraceRequest> {
        let doc = self.doc.as_ref()?;
        let lines = match &doc.text {
            TextState::Read(lines) if part.settings.recognise_text => lines.clone(),
            _ => Vec::new(),
        };
        Some(TraceRequest {
            threshold: part.values.threshold,
            settings: part.settings,
            selection: part
                .values
                .selection(&part.settings, part.analysis.suggested_background),
            invert: part.values.invert,
            drop_border: part.values.drop_border,
            colours: part.values.colour_spec(&part.settings),
            reduced,
            shapes: part.settings.recognise_shapes,
            lines,
            clip: None,
        })
    }

    /// Whether any part reads text.
    pub(crate) fn any_part_reads_text(&self) -> bool {
        self.settings.recognise_text
            || self
                .doc
                .as_ref()
                .is_some_and(|d| d.parts.all().iter().any(|p| p.settings.recognise_text))
    }

    /// The job all parts that add something make, with the key of each part it must trace and the
    /// notice when no part adds anything. Finished parts are taken from the cache.
    pub(crate) fn parts_job(
        &mut self,
        dragging: bool,
        for_saving: bool,
    ) -> Result<(TraceParts, PartKeys, bool), Option<&'static str>> {
        let live = self.live_part().ok_or(None)?;
        let doc = self.doc.as_mut().ok_or(None)?;
        doc.parts.store(live);
        let (w, h) = (doc.picture.width, doc.picture.height);
        if let Some(notice) = doc.parts.nothing_notice(w, h) {
            return Err(Some(notice));
        }
        let areas = doc.parts.areas(w, h);
        let clips = part_clips(&areas, kurbo::Rect::new(0.0, 0.0, size_mm(w), size_mm(h)));
        let active = doc.parts.active_index();
        let parts: Vec<Part> = doc.parts.all().to_vec();
        let picture = std::sync::Arc::clone(&doc.picture);
        let mut requests = Vec::with_capacity(parts.len());
        let mut deleted = Vec::with_capacity(parts.len());
        for (i, part) in parts.iter().enumerate() {
            // While a value is dragged only the active part is traced reduced; the others are
            // as they were (a part not traced before is traced at its own size).
            let reduced = request::use_reduced(&picture, dragging && i == active, for_saving);
            requests.push(self.trace_request(part, reduced).ok_or(None)?);
            deleted.push(part.values.palette_deleted(&part.settings));
        }
        let mut job = parts_job(&clips, requests, active).ok_or(None)?;
        let mut keys = PartKeys::new();
        let mut reduced = false;
        for j in &mut job.jobs {
            reduced |= j.request.reduced;
            if deleted[usize::from(j.part)] {
                j.cached = Some(tracyo_core::job::empty_colour_wire());
                continue;
            }
            let key = encode_trace(&j.request);
            j.cached = self.part_cache.get(&key);
            if j.cached.is_none() {
                keys.push((j.part, key));
            }
        }
        Ok((job, keys, reduced))
    }

    /// The tooltip of the blue button at rest: "Save as… (Ctrl+Shift+S), 3 parts joined" with
    /// two or more parts.
    pub(crate) fn save_tooltip(&self) -> String {
        match self.doc.as_ref().map(|d| d.parts.len()) {
            Some(n) if n >= 2 => format!("Save as\u{2026} (Ctrl+Shift+S), {n} parts joined"),
            _ => "Save as\u{2026} (Ctrl+Shift+S)".to_owned(),
        }
    }

    // ---- what a test or the bar reads ----

    /// The number of parts of the shown picture (0 without one).
    #[must_use]
    pub fn part_count(&self) -> usize {
        self.doc.as_ref().map_or(0, |d| d.parts.len())
    }

    /// The index of the active part, 0 for part 1.
    #[must_use]
    pub fn active_part(&self) -> usize {
        self.doc.as_ref().map_or(0, |d| d.parts.active_index())
    }

    /// The tooltip text of chip `index`: "Part 2 of 3, outside, 1 path".
    #[must_use]
    pub fn chip_name(&self, index: usize) -> Option<String> {
        let doc = self.doc.as_ref()?;
        (index < doc.parts.len()).then(|| {
            // The active part's snapshot is as of the last list change; its regions are live.
            let mut parts = doc.parts.clone();
            if let Some(live) = self.live_part() {
                parts.store(live);
            }
            parts.chip_name(index)
        })
    }

    /// The mask's part of the hidden status text: "Inside 2 paths", "3 parts", "Part 3 has no
    /// paths".
    #[must_use]
    pub fn announcement(&self) -> String {
        let Some(doc) = self.doc.as_ref() else {
            return String::new();
        };
        let mut parts = doc.parts.clone();
        if let Some(live) = self.live_part() {
            parts.store(live);
        }
        parts.announcement()
    }

    /// How many parts were traced (not taken from the cache) since the app started: a trace of
    /// one part counts one, a job counts the parts it sends without a cached result.
    #[must_use]
    pub fn part_traces(&self) -> u64 {
        self.part_traces
    }

    /// How many paths the shown (joined) result has.
    #[must_use]
    pub fn result_paths(&self) -> Option<usize> {
        self.result.as_ref().map(|r| r.output.paths)
    }

    /// How many finished parts the cache holds.
    #[must_use]
    pub fn cached_parts(&self) -> usize {
        self.part_cache.len()
    }

    /// Whether the trash is armed.
    #[must_use]
    pub fn trash_armed(&self) -> bool {
        self.trash.is_some()
    }

    /// The settings of part `index` as last stored (the active part's are the bar's).
    #[must_use]
    pub fn part_settings(&self, index: usize) -> Option<Settings> {
        let doc = self.doc.as_ref()?;
        if index == doc.parts.active_index() {
            return Some(self.settings);
        }
        doc.parts.all().get(index).map(|p| p.settings)
    }

    /// The regions of part `index`: (pixels, subtracted).
    #[must_use]
    pub fn part_regions(&self, index: usize) -> Vec<(tracyo_core::regions::PxRect, bool)> {
        let Some(doc) = self.doc.as_ref() else {
            return Vec::new();
        };
        if index == doc.parts.active_index() {
            return self.mask_regions();
        }
        doc.parts.all().get(index).map_or_else(Vec::new, |p| {
            p.regions.iter().map(|r| (r.rect, r.subtract)).collect()
        })
    }
}
