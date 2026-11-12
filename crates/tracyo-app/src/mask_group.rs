//! The Mask group in the Parts row: Rectangle, Lasso, Subtract, Inside | Outside and Clear mask
//! (ux-notes of 0002, sections 1 and 2).

use eframe::egui::{self, Pos2, Rect, vec2};
use tracyo_core::regions::MaskMode;

use crate::app::App;
use crate::bar::{bar_id, plain};
use crate::controls::segmented_equal;
use crate::icons::Icon;
use crate::mask::Tool;
use crate::widgets::{self, Side};

/// The width of the group: three icon buttons, the pill of two 64 px segments and Clear mask,
/// 4 px apart.
pub(crate) const WIDTH: f32 = 3.0 * 28.0 + 3.0 * 4.0 + 128.0 + 4.0 + 28.0;
const SEGMENT: f32 = 64.0;

impl App {
    /// The group with its top-left corner at `origin`. It is only drawn while a picture is shown.
    pub(crate) fn mask_group(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, origin: Pos2) {
        let Some(doc) = &self.doc else { return };
        let (tool, subtract, mode) = (doc.mask.tool, doc.mask.subtract, doc.mask.mode);
        let button = |x: f32| Rect::from_min_size(origin + vec2(x, 0.0), vec2(28.0, 28.0));
        let mut toggle = |this: &mut Self, x: f32, id: &str, icon: Icon, name: &str, on: bool| {
            let resp =
                widgets::icon_button(ui, &mut this.icons, button(x), bar_id(id), icon, name, on);
            widgets::tooltip(ctx, &resp, Side::Below, &plain(name));
            widgets::reset_tooltip(ctx, &resp);
            widgets::release_focus_after_mouse(ctx, &resp);
            resp.clicked()
        };
        let rectangle = toggle(
            self,
            0.0,
            "rectangle",
            Icon::Square,
            "Rectangle",
            tool == Tool::Rectangle,
        );
        let lasso = toggle(
            self,
            32.0,
            "lasso",
            Icon::Lasso,
            "Lasso",
            tool == Tool::Lasso,
        );
        let minus = toggle(
            self,
            64.0,
            "subtract",
            Icon::SquareMinus,
            "Subtract",
            subtract,
        );

        let chosen = segmented_equal(
            ui,
            origin + vec2(96.0, 0.0),
            bar_id("mask"),
            "Mask",
            &[MaskMode::Inside.label(), MaskMode::Outside.label()],
            usize::from(mode == MaskMode::Outside),
            SEGMENT,
        );

        let clear = widgets::icon_button(
            ui,
            &mut self.icons,
            button(228.0),
            bar_id("clear-mask"),
            Icon::X,
            "Clear mask",
            false,
        );
        widgets::tooltip(ctx, &clear, Side::Below, &plain("Clear mask"));
        widgets::reset_tooltip(ctx, &clear);
        widgets::release_focus_after_mouse(ctx, &clear);

        if rectangle {
            self.set_tool(ctx, Tool::Rectangle);
        }
        if lasso {
            self.set_tool(ctx, Tool::Lasso);
        }
        if minus && let Some(doc) = &mut self.doc {
            doc.mask.subtract = !doc.mask.subtract;
        }
        if let Some(i) = chosen {
            self.choose_mode(ctx, i);
        }
        if clear.clicked() {
            self.clear_mask(ctx);
        }
    }

    fn choose_mode(&mut self, ctx: &egui::Context, i: usize) {
        self.set_mask_mode(
            ctx,
            if i == 0 {
                MaskMode::Inside
            } else {
                MaskMode::Outside
            },
        );
    }
}
