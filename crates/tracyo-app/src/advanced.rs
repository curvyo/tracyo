//! The Advanced block under the rows of the bar: the toggles, Denoise, Upscale, Corners, Analyse.

use eframe::egui::{self, Align2, Pos2, Rect, Stroke, vec2};
use tracyo_core::settings;

use crate::app::App;
use crate::bar::{BlockFlow, bar_id, block_flow, field_widget, plain};
use crate::controls::{self, segmented, segmented_width};
use crate::icons::Icon;
use crate::theme;
use crate::widgets::{self, Side};

const DENOISE: [&str; 3] = ["Off", "Low", "High"];
const UPSCALE: [&str; 2] = ["Auto", "Off"];
/// The label slot of Denoise and Upscale.
const LABEL_W: f32 = 64.0;
/// Width of a cell that holds a label and a pill (at least 150).
const MIN_CELL: f32 = 150.0;
/// Width of the Analyse button.
const ANALYSE_W: f32 = 75.0;

impl App {
    /// Where the block's items sit when `inner` px are inside the bar's padding.
    pub(crate) fn block_layout(&self, ui: &egui::Ui, inner: f32) -> BlockFlow {
        let cell = |labels: &[&str]| (LABEL_W + segmented_width(ui, labels, 8.0)).max(MIN_CELL);
        let widths = [
            28.0 + 4.0 + 28.0,
            cell(&DENOISE),
            cell(&UPSCALE),
            MIN_CELL,
        ];
        block_flow(&widths, ANALYSE_W, inner - 4.0)
    }

    /// Paints and drives the block; `origin` is its content corner. Returns whether a value that
    /// traces changed (and whether it is being dragged).
    pub(crate) fn advanced_block(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        block: &BlockFlow,
        origin: Pos2,
        inner: f32,
    ) -> Option<bool> {
        let mut changed = None;
        // The divider: 4 px under the rows, full width of the content.
        let y = origin.y;
        ui.painter().line_segment(
            [Pos2::new(origin.x, y + 0.5), Pos2::new(origin.x + inner, y + 0.5)],
            Stroke::new(1.0, theme::DIVIDER),
        );
        let content = origin + vec2(0.0, 1.0 + 4.0);
        let at = |p: Pos2, w: f32| Rect::from_min_size(content + p.to_vec2(), vec2(w, 28.0));

        // Invert and Drop border shapes.
        let (invert, drop_border) = self
            .doc
            .as_ref()
            .map_or((false, false), |d| (d.values.invert, d.values.drop_border));
        for (k, (name, icon, on)) in [
            ("Invert", Icon::Contrast, invert),
            ("Drop border shapes", Icon::ImageMinus, drop_border),
        ]
        .into_iter()
        .enumerate()
        {
            let rect = at(block.items[0] + vec2(32.0 * k as f32, 0.0), 28.0);
            let resp = widgets::icon_button(
                ui,
                &mut self.icons,
                rect,
                bar_id("toggle").with(name),
                icon,
                name,
                on,
            );
            widgets::tooltip(ctx, &resp, Side::Below, &plain(name));
            widgets::reset_tooltip(ctx, &resp);
            widgets::release_focus_after_mouse(ctx, &resp);
            if resp.clicked()
                && let Some(doc) = &mut self.doc
            {
                if k == 0 {
                    doc.values.invert = !doc.values.invert;
                } else {
                    doc.values.drop_border = !doc.values.drop_border;
                }
                changed = Some(false);
            }
        }

        // Denoise and Upscale: a 12 px label in a 64 px slot, then the pill.
        let denoise = at(block.items[1], 0.0).min;
        widgets::text(
            ui,
            Pos2::new(denoise.x, denoise.y + 14.0),
            Align2::LEFT_CENTER,
            "Denoise",
            theme::text12(),
            theme::TOOLBAR_ICON,
        );
        if let Some(i) = segmented(
            ui,
            denoise + vec2(LABEL_W, 0.0),
            bar_id("denoise"),
            "Denoise",
            &DENOISE,
            usize::from(self.settings.denoise),
            8.0,
        ) {
            // invariant: three choices.
            self.settings.denoise = u8::try_from(i).unwrap_or(0);
            changed = Some(false);
        }
        let upscale = at(block.items[2], 0.0).min;
        widgets::text(
            ui,
            Pos2::new(upscale.x, upscale.y + 14.0),
            Align2::LEFT_CENTER,
            "Upscale",
            theme::text12(),
            theme::TOOLBAR_ICON,
        );
        if let Some(i) = segmented(
            ui,
            upscale + vec2(LABEL_W, 0.0),
            bar_id("upscale"),
            "Upscale",
            &UPSCALE,
            usize::from(!self.settings.upscale_auto),
            8.0,
        ) {
            self.settings.upscale_auto = i == 0;
            changed = Some(false);
        }

        // Corners.
        if let Some((v, dragging)) = field_widget(
            ctx,
            ui,
            at(block.items[3], 150.0),
            "corners",
            &settings::CORNERS,
            f64::from(self.settings.corners),
            &mut self.fields[4],
        ) {
            // invariant: the field clamps to 0..=100.
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            {
                self.settings.corners = v.round().clamp(0.0, 100.0) as u8;
            }
            changed = Some(dragging);
        }

        // Analyse.
        let analyse = controls::accent_button(
            ui,
            at(block.tail, ANALYSE_W),
            bar_id("analyse"),
            "Analyse",
        );
        widgets::release_focus_after_mouse(ctx, &analyse);
        if analyse.clicked() {
            self.analyse_again(ctx);
        }
        changed
    }
}
