//! The colour groups beside the main field: Remove background's chip, hex, Auto and pipette,
//! and Pick colours' pipette with its picked swatches.

use eframe::egui::{self, Align2, Color32, Id, Key, Modifiers, Pos2, Rect, vec2};
use tracyo_core::method::{Method, hex};
use vectrace::Rgb8;

use crate::app::{App, Pipette};
use crate::bar::{bar_id, plain};
use crate::controls;
use crate::icons::Icon;
use crate::theme;
use crate::widgets::{self, Side};

fn colour32(c: Rgb8) -> Color32 {
    Color32::from_rgb(c.r, c.g, c.b)
}

impl App {
    /// The group of `method` with its top-left corner at `origin`.
    pub(crate) fn colour_group(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        method: Method,
        origin: Pos2,
    ) {
        match method {
            Method::Brightness => {}
            Method::RemoveBackground => self.background_group(ui, ctx, origin),
            Method::PickColours => self.picked_group(ui, ctx, origin),
        }
    }

    fn pipette_button(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        rect: Rect,
        name: &str,
        kind: Pipette,
    ) {
        let resp = widgets::icon_button(
            ui,
            &mut self.icons,
            rect,
            bar_id("pipette").with(name),
            Icon::Pipette,
            name,
            self.pipette == kind,
        );
        widgets::tooltip(ctx, &resp, Side::Below, &plain("Pick a colour from the image"));
        widgets::reset_tooltip(ctx, &resp);
        widgets::release_focus_after_mouse(ctx, &resp);
        if resp.clicked() {
            self.toggle_pipette(kind);
        }
    }

    /// The background chip (28), its hex (56), Auto (44) and the pipette (28), 4 px apart.
    fn background_group(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, origin: Pos2) {
        let Some(doc) = &self.doc else { return };
        let background = doc.values.background(doc.analysis.suggested_background);
        let auto = doc.values.background_auto();
        let chip = Rect::from_min_size(origin, vec2(28.0, 28.0));
        controls::swatch(
            ui,
            Rect::from_center_size(chip.center(), vec2(20.0, 20.0)),
            background.map(colour32),
        );
        widgets::text(
            ui,
            Pos2::new(origin.x + 32.0, origin.y + 14.0),
            Align2::LEFT_CENTER,
            &background.map_or_else(|| "none".to_owned(), hex),
            theme::text12(),
            theme::TOOLBAR_ICON,
        );
        let auto_rect = Rect::from_min_size(origin + vec2(92.0, 0.0), vec2(44.0, 28.0));
        let resp = controls::text_toggle(ui, auto_rect, bar_id("auto"), "Auto", auto);
        widgets::release_focus_after_mouse(ctx, &resp);
        if resp.clicked() {
            self.set_auto(ctx);
        }
        let pipette = Rect::from_min_size(origin + vec2(140.0, 0.0), vec2(28.0, 28.0));
        self.pipette_button(
            ui,
            ctx,
            pipette,
            "Pick background colour from the image",
            Pipette::Background,
        );
    }

    /// The pipette (28) and 24 px cells of the picked colours, one Tab stop.
    fn picked_group(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, origin: Pos2) {
        let swatches: Vec<Rgb8> = self
            .doc
            .as_ref()
            .map(|d| d.values.swatches.clone())
            .unwrap_or_default();
        self.pipette_button(
            ui,
            ctx,
            Rect::from_min_size(origin, vec2(28.0, 28.0)),
            "Pick colour from the image",
            Pipette::Colours,
        );
        let stop = self.picked_stop.min(swatches.len().saturating_sub(1));
        let mut remove = None;
        let mut focus = None;
        for (i, colour) in swatches.iter().enumerate() {
            let cell = Rect::from_min_size(
                origin + vec2(30.0 + 24.0 * i as f32, 0.0),
                vec2(24.0, 28.0),
            );
            let id: Id = bar_id("picked").with(i);
            let name = format!("Remove {}", hex(*colour));
            let (resp, clicked) =
                controls::swatch_cell(ui, &mut self.icons, cell, id, colour32(*colour), &name, i == stop);
            widgets::tooltip(ctx, &resp, Side::Below, &plain(&hex(*colour)));
            widgets::reset_tooltip(ctx, &resp);
            if clicked {
                remove = Some(i);
                widgets::release_focus_after_mouse(ctx, &resp);
            }
            if resp.has_focus() {
                ui.memory_mut(|m| {
                    m.set_focus_lock_filter(
                        id,
                        egui::EventFilter {
                            horizontal_arrows: true,
                            ..Default::default()
                        },
                    );
                });
                let key = |k| ctx.input_mut(|inp| inp.consume_key(Modifiers::NONE, k));
                if key(Key::ArrowLeft) {
                    focus = Some(i.saturating_sub(1));
                } else if key(Key::ArrowRight) {
                    focus = Some((i + 1).min(swatches.len() - 1));
                } else if key(Key::Home) {
                    focus = Some(0);
                } else if key(Key::End) {
                    focus = Some(swatches.len() - 1);
                } else if key(Key::Delete) || key(Key::Backspace) {
                    remove = Some(i);
                }
            }
        }
        if let Some(i) = focus {
            self.picked_stop = i;
            ctx.memory_mut(|m| m.request_focus(bar_id("picked").with(i)));
        }
        if let Some(i) = remove {
            self.remove_picked(ctx, i);
        }
    }
}
