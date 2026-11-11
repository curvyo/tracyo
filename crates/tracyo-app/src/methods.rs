//! What the method controls do: choosing a method, the pipettes, picked colours, Auto and the
//! Analyse job.

use std::sync::mpsc::channel;

use eframe::egui;
use tracyo_core::analysis::Analysis;
use tracyo_core::method::Method;
use vectrace::Rgb8;

use crate::app::{App, Pipette};

impl App {
    /// The method chosen in the menu: Remove background starts with Auto pressed, Pick colours
    /// with the pipette armed while no colour is picked.
    pub(crate) fn set_method(&mut self, ctx: &egui::Context, method: Method) {
        self.settings.method = method.index();
        let Some(doc) = &mut self.doc else { return };
        if method == Method::RemoveBackground {
            doc.values.manual_background = None;
        }
        self.pipette = if method == Method::PickColours && doc.values.swatches.is_empty() {
            Pipette::Colours
        } else {
            Pipette::Off
        };
        self.values_changed(ctx, false);
    }

    /// A press on a pipette button: armed, or disarmed when it was the armed one.
    pub(crate) fn toggle_pipette(&mut self, kind: Pipette) {
        self.pipette = if self.pipette == kind {
            Pipette::Off
        } else {
            kind
        };
    }

    /// The colour a pipette takes at picture pixel (`x`, `y`): the middle value of the 3 x 3
    /// pixels around it, `None` on a transparent pixel.
    pub(crate) fn colour_at(&self, x: u32, y: u32) -> Option<Rgb8> {
        let doc = self.doc.as_ref()?;
        vectrace::sample_colour(doc.picture.raster(), x, y)
    }

    /// A click of the armed pipette at picture pixel (`x`, `y`); `add` is Shift.
    pub(crate) fn pick_at(&mut self, ctx: &egui::Context, x: u32, y: u32, add: bool) {
        let Some(colour) = self.colour_at(x, y) else {
            return;
        };
        let armed = self.pipette;
        let Some(doc) = &mut self.doc else { return };
        match armed {
            Pipette::Off => return,
            Pipette::Background => {
                doc.values.manual_background = Some(colour);
                self.pipette = Pipette::Off;
            }
            Pipette::Colours => doc.values.pick(colour, add),
        }
        self.values_changed(ctx, false);
    }

    /// Takes the picked colour at `index` out.
    pub(crate) fn remove_picked(&mut self, ctx: &egui::Context, index: usize) {
        let Some(doc) = &mut self.doc else { return };
        doc.values.remove(index);
        self.picked_stop = self
            .picked_stop
            .min(doc.values.swatches.len().saturating_sub(1));
        self.values_changed(ctx, false);
    }

    /// The Auto toggle: the background follows the analysis again.
    pub(crate) fn set_auto(&mut self, ctx: &egui::Context) {
        let Some(doc) = &mut self.doc else { return };
        if doc.values.manual_background.take().is_some() {
            self.values_changed(ctx, false);
        }
    }

    /// The Analyse button: analyses the shown picture again on a thread and applies the result
    /// when it arrives.
    pub(crate) fn analyse_again(&mut self, ctx: &egui::Context) {
        let Some(doc) = &self.doc else { return };
        let picture = std::sync::Arc::clone(&doc.picture);
        let repaint = ctx.clone();
        let (tx, rx) = channel();
        std::thread::spawn(move || {
            let _ = tx.send(Analysis::of(&picture));
            repaint.request_repaint();
        });
        self.analysing = Some(rx);
    }

    /// Applies an Analyse answer that has arrived.
    pub(crate) fn finish_analysing(&mut self, ctx: &egui::Context) {
        let Some(rx) = &self.analysing else { return };
        let analysis = match rx.try_recv() {
            Ok(analysis) => analysis,
            Err(std::sync::mpsc::TryRecvError::Empty) => return,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                self.analysing = None;
                return;
            }
        };
        self.analysing = None;
        let Some(doc) = &mut self.doc else { return };
        analysis.apply(&mut self.settings, &mut doc.values);
        doc.analysis = analysis;
        self.values_changed(ctx, false);
    }
}
