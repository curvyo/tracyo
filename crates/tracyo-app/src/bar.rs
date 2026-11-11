//! The Trace bar, the Open card and the menus: layout, painting and what each control does.

use eframe::egui::{self, Align2, Id, Key, Modifiers, Rect, Vec2, pos2, vec2};
use tracyo_core::progress::Button;
use tracyo_core::settings::{self, Field};

use crate::app::{App, BAR_POS, MenuKind};
use crate::icons::Icon;
use crate::theme;
use crate::widgets::{self, FieldEvent, Line, MenuEvent, Row, Side};

/// Row height of the bar.
const ROW: f32 = 36.0;
/// Side padding and gap between items.
const PAD: f32 = 8.0;

/// Where the items of a bar sit, relative to the bar's top-left corner.
#[derive(Debug, Clone, PartialEq)]
pub struct Flow {
    /// Top-left of each item.
    pub items: Vec<egui::Pos2>,
    /// Top-left of the last group (expander slot and blue button).
    pub tail: egui::Pos2,
    /// The bar's size.
    pub size: Vec2,
}

/// Flows `widths` in whole items, wrapping when `max_inner` would be passed; `tail` is the last
/// group, pushed to the far right of its row.
#[must_use]
pub fn flow(widths: &[f32], tail: f32, max_inner: f32) -> Flow {
    let mut row = 0.0_f32;
    let mut x = 0.0_f32;
    let mut extent = 0.0_f32;
    let mut items = Vec::with_capacity(widths.len());
    for &w in widths {
        if x > 0.0 && x + w > max_inner {
            row += 1.0;
            x = 0.0;
        }
        items.push(pos2(PAD + x, row * ROW + 4.0));
        extent = extent.max(x + w);
        x += w + PAD;
    }
    if x > 0.0 && x + tail > max_inner {
        row += 1.0;
        x = 0.0;
    }
    extent = extent.max(x + tail);
    let tail_x = if row > 0.0 || extent > x + tail {
        extent - tail
    } else {
        x
    };
    Flow {
        items,
        tail: pos2(PAD + tail_x, row * ROW + 4.0),
        size: vec2(extent + 2.0 * PAD, (row + 1.0) * ROW),
    }
}

fn field_tip(field: &Field, value: f64) -> Vec<Line> {
    vec![
        Line {
            text: format!("{}: {} {}", field.label, field.format(value), field.unit)
                .trim_end()
                .to_owned(),
            font: theme::text12(),
            colour: theme::FOREGROUND,
        },
        Line {
            text: "Click to set, double-click to type".into(),
            font: theme::text12(),
            colour: theme::MENU_SECOND,
        },
    ]
}

fn plain(text: &str) -> Vec<Line> {
    vec![Line {
        text: text.to_owned(),
        font: theme::text12(),
        colour: theme::FOREGROUND,
    }]
}

impl App {
    /// The Open card: the rail card with the Import button.
    pub(crate) fn open_card(&mut self, ctx: &egui::Context) {
        let rect = Rect::from_min_size(pos2(12.0, 12.0), vec2(48.0, 48.0));
        let mut clicked = false;
        egui::Area::new(Id::unique("open-card"))
            .order(egui::Order::Foreground)
            .fixed_pos(rect.min)
            .show(ctx, |ui| {
                let (card, _) = ui.allocate_exact_size(rect.size(), egui::Sense::hover());
                ui.painter().add(theme::SHADOW.as_shape(card, 10));
                ui.painter().rect_filled(card, 10, theme::TOOLBAR_BG);
                let button = Rect::from_center_size(card.center(), vec2(40.0, 40.0));
                let resp =
                    ui.interact(button, Id::unique("open-card-button"), egui::Sense::click());
                resp.widget_info(|| {
                    egui::WidgetInfo::labeled(egui::Role::Button, true, "Open image")
                });
                if resp.hovered() {
                    ui.painter().rect_filled(button, 8, theme::HOVER_ON_BAR);
                }
                self.icons
                    .paint(ui, button, Icon::Import, 20.0, 2.0, theme::TOOLBAR_ICON);
                if resp.has_focus() && widgets::focus_visible(ui.ctx()) {
                    ui.painter().rect_stroke(
                        button.expand(1.0),
                        9,
                        egui::Stroke::new(2.0, theme::ACCENT),
                        egui::StrokeKind::Outside,
                    );
                }
                widgets::tooltip(ui.ctx(), &resp, Side::Right, &plain("Open image (Ctrl+O)"));
                widgets::reset_tooltip(ui.ctx(), &resp);
                clicked = resp.clicked();
            });
        if clicked {
            self.ask_open(ctx);
        }
    }

    /// The Trace bar.
    #[allow(clippy::too_many_lines)]
    pub(crate) fn bar(&mut self, ctx: &egui::Context, canvas: Rect) {
        let max_inner = (canvas.width() - 84.0 - 2.0 * PAD).max(200.0);
        egui::Area::new(Id::unique("bar"))
            .order(egui::Order::Foreground)
            .fixed_pos(BAR_POS)
            .show(ctx, |ui| {
                if self.doc.is_none() {
                    self.empty_bar(ui);
                    return;
                }
                let state = self.progress.button(self.now());
                let blue_w = widgets::blue_width(ui, &state);
                let widths = [28.0, 132.0, 176.0, 150.0, 150.0, 150.0, 171.0];
                let tail_w = 28.0 + PAD + blue_w;
                let layout = flow(&widths, tail_w, max_inner);
                let (bar, _) = ui.allocate_exact_size(layout.size, egui::Sense::hover());
                self.bar_bottom = bar.bottom();
                ui.painter().add(theme::SHADOW.as_shape(bar, 10));
                ui.painter().rect_filled(bar, 10, theme::TOOLBAR_BG);
                let at = |p: egui::Pos2, w: f32| {
                    Rect::from_min_size(bar.min + p.to_vec2(), vec2(w, 28.0))
                };
                let mut changed: Option<bool> = None;

                // About.
                let about = widgets::icon_button(
                    ui,
                    &mut self.icons,
                    at(layout.items[0], 28.0),
                    bar_id("about"),
                    Icon::Info,
                    "About Tracyo",
                    false,
                );
                widgets::tooltip(ctx, &about, Side::Below, &plain("About Tracyo"));
                widgets::reset_tooltip(ctx, &about);
                widgets::release_focus_after_mouse(ctx, &about);

                // Preset and Method.
                let preset_rect = at(layout.items[1], 132.0);
                let preset = widgets::drop_button(
                    ui,
                    preset_rect,
                    bar_id("preset"),
                    self.settings.preset_label(),
                    "Preset",
                    self.menu == Some(MenuKind::Preset),
                );
                if preset.clicked()
                    || (preset.has_focus()
                        && ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::ArrowDown)))
                {
                    self.toggle_menu(
                        MenuKind::Preset,
                        preset_rect,
                        self.settings.preset_index().unwrap_or(0),
                    );
                }
                let method_rect = at(layout.items[2], 176.0);
                let method = widgets::drop_button(
                    ui,
                    method_rect,
                    bar_id("method"),
                    "Brightness",
                    "Method",
                    self.menu == Some(MenuKind::Method),
                );
                if method.clicked()
                    || (method.has_focus()
                        && ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::ArrowDown)))
                {
                    self.toggle_menu(MenuKind::Method, method_rect, 0);
                }

                // Value fields.
                let threshold = self.doc.as_ref().map_or(0, |d| d.threshold);
                let threshold_percent = settings::percent_of_luma(threshold);
                let (ev, resp) = widgets::value_field(
                    ui,
                    at(layout.items[3], 150.0),
                    bar_id("threshold"),
                    &settings::THRESHOLD,
                    threshold_percent,
                    &mut self.fields[0],
                );
                widgets::tooltip(
                    ctx,
                    &resp,
                    Side::Left,
                    &field_tip(&settings::THRESHOLD, threshold_percent),
                );
                widgets::reset_tooltip(ctx, &resp);
                if let FieldEvent::Set { value, dragging } = ev {
                    if let Some(doc) = &mut self.doc {
                        doc.threshold = settings::luma_of_percent(value);
                    }
                    changed = Some(dragging);
                }
                let (ev, resp) = widgets::value_field(
                    ui,
                    at(layout.items[4], 150.0),
                    bar_id("smooth"),
                    &settings::SMOOTH,
                    f64::from(self.settings.smooth),
                    &mut self.fields[1],
                );
                widgets::tooltip(
                    ctx,
                    &resp,
                    Side::Left,
                    &field_tip(&settings::SMOOTH, f64::from(self.settings.smooth)),
                );
                widgets::reset_tooltip(ctx, &resp);
                if let FieldEvent::Set { value, dragging } = ev {
                    // invariant: Smooth is clamped to 0..=100.
                    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                    {
                        self.settings.smooth = value.round().clamp(0.0, 100.0) as u8;
                    }
                    changed = Some(dragging);
                }
                let (ev, resp) = widgets::value_field(
                    ui,
                    at(layout.items[5], 150.0),
                    bar_id("tolerance"),
                    &settings::TOLERANCE,
                    self.settings.tolerance,
                    &mut self.fields[2],
                );
                widgets::tooltip(
                    ctx,
                    &resp,
                    Side::Left,
                    &field_tip(&settings::TOLERANCE, self.settings.tolerance),
                );
                widgets::reset_tooltip(ctx, &resp);
                if let FieldEvent::Set { value, dragging } = ev {
                    self.settings.tolerance = value;
                    changed = Some(dragging);
                }
                let (ev, resp) = widgets::value_field(
                    ui,
                    at(layout.items[6], 171.0),
                    bar_id("ignore"),
                    &settings::IGNORE_BELOW,
                    self.settings.ignore_below,
                    &mut self.fields[3],
                );
                widgets::tooltip(
                    ctx,
                    &resp,
                    Side::Left,
                    &field_tip(&settings::IGNORE_BELOW, self.settings.ignore_below),
                );
                widgets::reset_tooltip(ctx, &resp);
                if let FieldEvent::Set { value, dragging } = ev {
                    self.settings.ignore_below = value;
                    changed = Some(dragging);
                }

                // The blue button (the expander's slot before it stays empty until slice 2).
                let blue_rect = at(pos2(layout.tail.x + 28.0 + PAD, layout.tail.y), blue_w);
                let reason = self.result.as_ref().and_then(|r| r.reason.clone());
                let dimmed = matches!(state, Button::Rest) && reason.is_some();
                let blue = widgets::blue_button(
                    ui,
                    &mut self.icons,
                    blue_rect,
                    bar_id("blue"),
                    &state,
                    "Save as\u{2026}",
                    dimmed,
                    blue_w,
                );
                let tip: Vec<Line> = match &state {
                    Button::Rest if dimmed => plain(&reason.clone().unwrap_or_default()),
                    Button::Rest => plain("Save as\u{2026} (Ctrl+Shift+S)"),
                    Button::Running(_) => Vec::new(),
                    Button::Stopped => plain("Preview is out of date"),
                    Button::Failed(text) => plain(text),
                };
                widgets::tooltip(ctx, &blue.main, Side::Below, &tip);
                widgets::reset_tooltip(ctx, &blue.main);
                if let Some(stop) = &blue.stop {
                    widgets::tooltip(ctx, stop, Side::Below, &plain("Stop (Esc)"));
                    widgets::reset_tooltip(ctx, stop);
                    if stop.clicked() {
                        self.stop_trace();
                    }
                }
                widgets::release_focus_after_mouse(ctx, &blue.main);
                if blue.main.clicked() {
                    match state {
                        Button::Rest => self.save_action(ctx),
                        Button::Stopped | Button::Failed(_) => self.retry(ctx),
                        Button::Running(_) => {}
                    }
                }
                if let Some(dragging) = changed {
                    self.values_changed(ctx, dragging);
                }
                let _ = Align2::LEFT_TOP;
            });
    }

    fn toggle_menu(&mut self, kind: MenuKind, anchor: Rect, current: usize) {
        if self.menu == Some(kind) {
            self.menu = None;
        } else {
            self.menu = Some(kind);
            self.menu_anchor = anchor;
            self.menu_highlight = current;
        }
    }

    fn empty_bar(&mut self, ui: &mut egui::Ui) {
        let label_w = widgets::text_width(ui, "Trace", &theme::text12());
        let button_w = widgets::text_width(ui, "Open image\u{2026}", &theme::text14()) + 16.0;
        let size = vec2(PAD + label_w + 12.0 + button_w + PAD, ROW);
        let (bar, _) = ui.allocate_exact_size(size, egui::Sense::hover());
        self.bar_bottom = bar.bottom();
        ui.painter().add(theme::SHADOW.as_shape(bar, 10));
        ui.painter().rect_filled(bar, 10, theme::TOOLBAR_BG);
        widgets::text(
            ui,
            pos2(bar.left() + PAD, bar.center().y),
            Align2::LEFT_CENTER,
            "Trace",
            theme::text12(),
            theme::TOOLBAR_ICON,
        );
        let button = Rect::from_min_size(
            pos2(bar.left() + PAD + label_w + 12.0, bar.top() + 4.0),
            vec2(button_w, 28.0),
        );
        let resp = widgets::text_button(
            ui,
            button,
            bar_id("open-image"),
            "Open image\u{2026}",
            "Open image\u{2026}",
        );
        if resp.clicked() {
            let ctx = ui.ctx().clone();
            self.ask_open(&ctx);
        }
    }

    /// The Preset and Method menus.
    pub(crate) fn menus(&mut self, ctx: &egui::Context) {
        let Some(kind) = self.menu else { return };
        let presets = vectrace::PRESETS;
        let method_rows = [Row {
            name: "Brightness",
            detail: "Dark and light pixels by a cut-off.",
        }];
        let preset_rows: Vec<Row<'_>> = presets
            .iter()
            .map(|p| Row {
                name: p.label,
                detail: p.detail,
            })
            .collect();
        let (rows, current): (&[Row<'_>], usize) = match kind {
            MenuKind::Preset => (
                &preset_rows,
                self.settings.preset_index().unwrap_or(usize::MAX),
            ),
            MenuKind::Method => (&method_rows, 0),
        };
        let event = widgets::menu(
            ctx,
            Id::unique("menu").with(format!("{kind:?}")),
            self.menu_anchor,
            rows,
            current,
            &mut self.menu_highlight,
        );
        match event {
            MenuEvent::None => {}
            MenuEvent::Closed => self.menu = None,
            MenuEvent::Picked(i) => {
                self.menu = None;
                if kind == MenuKind::Preset {
                    self.settings = self.settings.with_preset(i);
                    self.values_changed(ctx, false);
                }
            }
        }
    }
}

fn bar_id(name: &str) -> Id {
    Id::unique("bar").with(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    const WIDTHS: [f32; 7] = [28.0, 132.0, 176.0, 150.0, 150.0, 150.0, 171.0];

    #[test]
    fn at_rest_the_bar_is_one_row_of_1154_px() {
        let f = flow(&WIDTHS, 28.0 + 8.0 + 89.0, 1196.0 - 16.0);
        assert_eq!(f.size, vec2(1154.0, 36.0));
        assert_eq!(f.items[0], pos2(8.0, 4.0));
        assert_eq!(f.items[1], pos2(8.0 + 28.0 + 8.0, 4.0));
        assert_eq!(
            f.tail,
            pos2(
                8.0 + 28.0
                    + 8.0
                    + 132.0
                    + 8.0
                    + 176.0
                    + 8.0
                    + 150.0
                    + 8.0
                    + 150.0
                    + 8.0
                    + 150.0
                    + 8.0
                    + 171.0
                    + 8.0,
                4.0
            )
        );
        assert_eq!(f.tail.x + 28.0 + 8.0 + 89.0 + 8.0, 1154.0);
    }

    #[test]
    fn at_800_wide_the_bar_wraps_by_whole_items_into_two_rows() {
        let f = flow(&WIDTHS, 125.0, 716.0 - 16.0);
        assert_eq!(f.size.y, 72.0);
        // i, Preset, Method, Threshold, Smooth fit the first row (684 px); Tolerance starts row two.
        assert_eq!(f.items[4].y, 4.0);
        assert_eq!(f.items[5], pos2(8.0, 40.0));
        assert!(f.tail.y >= 40.0);
        assert!(f.size.x <= 716.0);
        // The last group sits at the far right of its row.
        assert!((f.tail.x + 125.0 + 8.0 - f.size.x).abs() < 1e-3);
    }
}
