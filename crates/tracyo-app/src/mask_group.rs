//! The Mask group of the bar: Rectangle, Subtract, Inside | Outside and Clear mask (ux-notes
//! section 2).

use eframe::egui::{self, Pos2, Rect, vec2};
use tracyo_core::regions::MaskMode;

use crate::app::App;
use crate::bar::{bar_id, plain};
use crate::controls::segmented_equal;
use crate::icons::Icon;
use crate::widgets::{self, Side};

/// The width of the group: two icon buttons, the pill of two 64 px segments and Clear mask,
/// 4 px apart.
pub(crate) const WIDTH: f32 = 28.0 + 4.0 + 28.0 + 4.0 + 128.0 + 4.0 + 28.0;
const SEGMENT: f32 = 64.0;

impl App {
    /// The group with its top-left corner at `origin`. It is only drawn while a picture is shown.
    pub(crate) fn mask_group(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, origin: Pos2) {
        let Some(doc) = &self.doc else { return };
        let (rectangle, subtract, mode) = (doc.mask.rectangle, doc.mask.subtract, doc.mask.mode);
        let button = |x: f32| Rect::from_min_size(origin + vec2(x, 0.0), vec2(28.0, 28.0));

        let rect = widgets::icon_button(
            ui,
            &mut self.icons,
            button(0.0),
            bar_id("rectangle"),
            Icon::Square,
            "Rectangle",
            rectangle,
        );
        widgets::tooltip(ctx, &rect, Side::Below, &plain("Rectangle"));
        widgets::reset_tooltip(ctx, &rect);
        widgets::release_focus_after_mouse(ctx, &rect);
        let minus = widgets::icon_button(
            ui,
            &mut self.icons,
            button(32.0),
            bar_id("subtract"),
            Icon::SquareMinus,
            "Subtract",
            subtract,
        );
        widgets::tooltip(ctx, &minus, Side::Below, &plain("Subtract"));
        widgets::reset_tooltip(ctx, &minus);
        widgets::release_focus_after_mouse(ctx, &minus);

        let chosen = segmented_equal(
            ui,
            origin + vec2(64.0, 0.0),
            bar_id("mask"),
            "Mask",
            &[MaskMode::Inside.label(), MaskMode::Outside.label()],
            usize::from(mode == MaskMode::Outside),
            SEGMENT,
        );

        let clear = widgets::icon_button(
            ui,
            &mut self.icons,
            button(196.0),
            bar_id("clear-mask"),
            Icon::X,
            "Clear mask",
            false,
        );
        widgets::tooltip(ctx, &clear, Side::Below, &plain("Clear mask"));
        widgets::reset_tooltip(ctx, &clear);
        widgets::release_focus_after_mouse(ctx, &clear);

        if let Some(doc) = &mut self.doc {
            if rect.clicked() {
                doc.mask.rectangle = !doc.mask.rectangle;
            }
            if minus.clicked() {
                doc.mask.subtract = !doc.mask.subtract;
            }
        }
        if let Some(i) = chosen {
            self.set_mask_mode(
                ctx,
                if i == 0 {
                    MaskMode::Inside
                } else {
                    MaskMode::Outside
                },
            );
        }
        if clear.clicked() {
            self.clear_mask(ctx);
        }
    }
}
