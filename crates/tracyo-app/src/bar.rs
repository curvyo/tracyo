//! The Trace bar and its menus: layout, centring, painting and what each control does.

use eframe::egui::{self, Align2, Id, Key, Modifiers, Rect, Vec2, pos2, vec2};
use tracyo_core::method::Method;
use tracyo_core::progress::Button;
use tracyo_core::settings::{self, Field};

use crate::app::{App, BAR_MARGIN, MenuKind};
use crate::controls;
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

/// Whole items in lines: x and y of each, the tail's, and the height of all lines.
struct Rows {
    items: Vec<(f32, f32)>,
    tail: (f32, f32),
    height: f32,
}

/// Flows `widths` in whole items, wrapping when `inner` would be passed; `heights` are the items'
/// heights (a line is as tall as its tallest item, items sit at its top); `tail` is the last
/// item, 36 px high, pushed to the far right of its line (`margin-left: auto`).
fn rows(widths: &[f32], heights: &[f32], tail: f32, inner: f32) -> Rows {
    let mut y = 0.0_f32;
    let mut line = ROW;
    let mut x = 0.0_f32;
    let mut items = Vec::with_capacity(widths.len());
    for (&w, &h) in widths.iter().zip(heights) {
        if x > 0.0 && x + w > inner {
            y += line;
            line = ROW;
            x = 0.0;
        }
        items.push((x, y));
        line = line.max(h);
        x += w + PAD;
    }
    if x > 0.0 && x + tail > inner {
        y += line;
        line = ROW;
    }
    Rows {
        items,
        tail: (inner - tail, y),
        height: y + line,
    }
}

/// The flow of the bar's rows. The bar is as wide as its items on one line (with the open
/// Advanced block's zero-width item adding one more gap), at most `max_inner`; when they do not
/// fit on one line the bar takes all of `max_inner` and the items wrap by whole items.
#[must_use]
pub fn flow(widths: &[f32], tail: f32, max_inner: f32, block_open: bool) -> Flow {
    let gaps = widths.len() as f32 + if block_open { 1.0 } else { 0.0 };
    let single = widths.iter().sum::<f32>() + tail + gaps * PAD;
    let inner = single.min(max_inner);
    let r = rows(widths, &vec![ROW; widths.len()], tail, inner);
    Flow {
        items: r
            .items
            .iter()
            .map(|&(x, y)| pos2(PAD + x, y + 4.0))
            .collect(),
        tail: pos2(PAD + r.tail.0, r.tail.1 + 4.0),
        size: vec2(inner + 2.0 * PAD, r.height),
    }
}

/// Where the items of the Advanced block sit, relative to the block's content corner.
#[derive(Debug, Clone, PartialEq)]
pub struct BlockFlow {
    /// Top-left of each item.
    pub items: Vec<egui::Pos2>,
    /// Top-left of the last item (Analyse), at the far right of its row.
    pub tail: egui::Pos2,
    /// Top-left of the Parts row, the block's last line, a line of its own.
    pub parts: egui::Pos2,
    /// Height of all the lines, the Parts row's included, in px.
    pub height: f32,
}

/// Flows the Advanced block's items of `widths` and `heights` in `inner` px; `tail` is Analyse.
/// The Parts row of height `parts` (36 or 72 px) is the last line under all of them.
#[must_use]
pub fn block_flow(widths: &[f32], heights: &[f32], tail: f32, inner: f32, parts: f32) -> BlockFlow {
    let r = rows(widths, heights, tail, inner);
    BlockFlow {
        items: r.items.iter().map(|&(x, y)| pos2(x, y + 4.0)).collect(),
        tail: pos2(r.tail.0, r.tail.1 + 4.0),
        parts: pos2(0.0, r.height + 4.0),
        height: r.height + parts,
    }
}

/// Height of the block under the rows: 4 px margin, the 1 px divider, 4 px, the lines, 4 px.
#[must_use]
pub fn block_height(lines: f32) -> f32 {
    4.0 + 1.0 + 4.0 + lines + 4.0
}

/// The bar's left edge in a canvas `canvas_w` wide (0001 ux-notes 3.1): the bar at rest, `rest_w`
/// wide, is centred on whole pixels; the bar of now, `width` wide, keeps that left edge and is only
/// pulled left when its right end would pass the margin. A rest bar that fills the cap starts at
/// the margin.
#[must_use]
pub fn bar_x(canvas_w: f32, rest_w: f32, width: f32) -> f32 {
    if rest_w >= canvas_w - 2.0 * BAR_MARGIN {
        return BAR_MARGIN;
    }
    let centred = ((canvas_w - rest_w) / 2.0).floor();
    centred.min(canvas_w - BAR_MARGIN - width).max(BAR_MARGIN)
}

/// The tooltip of a value field; `shown` is the text the field draws instead of the value's own.
pub(crate) fn field_tip(field: &Field, value: f64, shown: Option<&str>) -> Vec<Line> {
    let text = shown.map_or_else(|| field.format(value), str::to_owned);
    vec![
        Line {
            text: format!("{}: {} {}", field.label, text, field.unit)
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

pub(crate) fn plain(text: &str) -> Vec<Line> {
    vec![Line {
        text: text.to_owned(),
        font: theme::text12(),
        colour: theme::FOREGROUND,
    }]
}

/// A value field with its tooltip; the value it was set to and whether the pointer still holds it.
pub(crate) fn field_widget(
    ctx: &egui::Context,
    ui: &mut egui::Ui,
    rect: Rect,
    name: &str,
    field: &Field,
    value: f64,
    state: &mut widgets::FieldState,
) -> Option<(f64, bool)> {
    let (event, resp) = widgets::value_field(ui, rect, bar_id(name), field, value, state);
    widgets::tooltip(
        ctx,
        &resp,
        Side::Left,
        &field_tip(field, value, state.shown.as_deref()),
    );
    widgets::reset_tooltip(ctx, &resp);
    match event {
        FieldEvent::Set { value, dragging } => Some((value, dragging)),
        FieldEvent::None => None,
    }
}

impl App {
    /// The Open button, the bar's first control.
    fn open_button(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, rect: Rect) {
        let open = widgets::icon_button(
            ui,
            &mut self.icons,
            rect,
            bar_id("open"),
            Icon::Import,
            "Open image",
            false,
        );
        widgets::tooltip(ctx, &open, Side::Below, &plain("Open image (Ctrl+O)"));
        widgets::reset_tooltip(ctx, &open);
        widgets::release_focus_after_mouse(ctx, &open);
        if open.clicked() {
            self.ask_open(ctx);
        }
    }

    /// The Trace bar: the rows of controls and, while open, the Advanced block. `root` measures
    /// the text so that the bar's place is known before it is drawn.
    pub(crate) fn bar(&mut self, root: &egui::Ui, ctx: &egui::Context, canvas: Rect) {
        let canvas_w = canvas.width();
        if self.doc.is_none() {
            let size = empty_size(root);
            let x = bar_x(canvas_w, size.x, size.x);
            egui::Area::new(Id::unique("bar"))
                .order(egui::Order::Foreground)
                .fixed_pos(pos2(canvas.min.x + x, BAR_MARGIN))
                .show(ctx, |ui| self.empty_bar(ui, size));
            return;
        }
        let state = self.progress.button(self.now());
        let blue_w = widgets::blue_width(root, &state);
        let method = Method::from_index(self.settings.method);
        let group_w = self.group_width(method);
        let mut widths = vec![28.0, 28.0, 132.0, 176.0, 150.0];
        widths.extend(group_w);
        widths.extend([150.0, 150.0, 171.0]);
        let open = self.settings.advanced_open;
        let max_inner = (canvas_w - 2.0 * BAR_MARGIN - 2.0 * PAD).max(200.0);
        let layout = flow(&widths, 28.0 + PAD + blue_w, max_inner, open);
        // The bar at rest (Save as… 89 px, Advanced closed) decides where it stands.
        let rest = flow(&widths, 28.0 + PAD + 89.0, max_inner, false);
        let x = bar_x(canvas_w, rest.size.x, layout.size.x);
        let inner = layout.size.x - 2.0 * PAD;
        let block = open.then(|| self.block_layout(root, inner));
        let height = layout.size.y + block.as_ref().map_or(0.0, |b| block_height(b.height));
        egui::Area::new(Id::unique("bar"))
            .order(egui::Order::Foreground)
            .fixed_pos(pos2(canvas.min.x + x, BAR_MARGIN))
            .show(ctx, |ui| {
                let (bar, _) =
                    ui.allocate_exact_size(vec2(layout.size.x, height), egui::Sense::hover());
                self.bar_bottom = bar.bottom();
                ui.painter().add(theme::SHADOW.as_shape(bar, 10));
                ui.painter().rect_filled(bar, 10, theme::TOOLBAR_BG);
                let at = |p: egui::Pos2, w: f32| {
                    Rect::from_min_size(bar.min + p.to_vec2(), vec2(w, 28.0))
                };
                let mut changed: Option<bool> = None;

                self.open_button(ui, ctx, at(layout.items[0], 28.0));
                let about = widgets::icon_button(
                    ui,
                    &mut self.icons,
                    at(layout.items[1], 28.0),
                    bar_id("about"),
                    Icon::Info,
                    crate::about::TITLE,
                    self.about_open,
                );
                if about.clicked() {
                    self.about_open = !self.about_open;
                }
                widgets::tooltip(ctx, &about, Side::Below, &plain(crate::about::TITLE));
                widgets::reset_tooltip(ctx, &about);
                widgets::release_focus_after_mouse(ctx, &about);

                self.preset_button(ui, ctx, at(layout.items[2], 132.0));
                self.method_button(ui, ctx, method, at(layout.items[3], 176.0));

                let mut next = 4;
                changed = self
                    .main_field(ui, ctx, at(layout.items[next], 150.0))
                    .or(changed);
                next += 1;
                if group_w.is_some() {
                    let origin = bar.min + layout.items[next].to_vec2();
                    self.colour_group(ui, ctx, method, origin);
                    next += 1;
                }
                for (i, (name, field)) in [
                    ("smooth", &settings::SMOOTH),
                    ("tolerance", &settings::TOLERANCE),
                    ("ignore", &settings::IGNORE_BELOW),
                ]
                .into_iter()
                .enumerate()
                {
                    let value = self.setting_value(i);
                    let width = if i == 2 { 171.0 } else { 150.0 };
                    let rect = at(layout.items[next + i], width);
                    if let Some((v, dragging)) =
                        field_widget(ctx, ui, rect, name, field, value, &mut self.fields[1 + i])
                    {
                        self.set_setting(i, v);
                        changed = Some(dragging);
                    }
                }

                self.tail(ui, ctx, &state, blue_w, bar.min + layout.tail.to_vec2());
                if let Some(block) = block {
                    let origin = bar.min + vec2(PAD, layout.size.y + 4.0);
                    changed = self
                        .advanced_block(ui, ctx, &block, origin, inner)
                        .or(changed);
                }
                if let Some(dragging) = changed {
                    self.values_changed(ctx, dragging);
                }
            });
    }

    /// The width of the method's group beside the main field, if it has one.
    fn group_width(&self, method: Method) -> Option<f32> {
        match method {
            Method::Brightness => None,
            Method::RemoveBackground => Some(168.0),
            Method::PickColours => {
                let n = self.doc.as_ref().map_or(0, |d| d.values.swatches.len());
                Some(28.0 + 24.0 * n as f32)
            }
        }
    }

    /// Smooth (0), Tolerance (1) or Ignore below (2) as a number.
    fn setting_value(&self, index: usize) -> f64 {
        match index {
            0 => f64::from(self.settings.smooth),
            1 => self.settings.tolerance,
            _ => self.settings.ignore_below,
        }
    }

    fn set_setting(&mut self, index: usize, value: f64) {
        match index {
            // invariant: Smooth is clamped to 0..=100.
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            0 => self.settings.smooth = value.round().clamp(0.0, 100.0) as u8,
            1 => self.settings.tolerance = value,
            _ => self.settings.ignore_below = value,
        }
    }

    /// The Preset button.
    fn preset_button(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, rect: Rect) {
        let preset = widgets::drop_button(
            ui,
            rect,
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
                rect,
                self.settings.preset_index().unwrap_or(0),
            );
        }
    }

    /// The Method button.
    fn method_button(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        method: Method,
        rect: Rect,
    ) {
        let button = widgets::drop_button(
            ui,
            rect,
            bar_id("method"),
            method.label(),
            "Method",
            self.menu == Some(MenuKind::Method),
        );
        if button.clicked()
            || (button.has_focus()
                && ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::ArrowDown)))
        {
            self.toggle_menu(MenuKind::Method, rect, usize::from(method.index()));
        }
    }

    /// The Threshold, or the Colour tolerance of the colour methods, in the main slot.
    fn main_field(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, rect: Rect) -> Option<bool> {
        let method = Method::from_index(self.settings.method);
        let doc_threshold = self.doc.as_ref().map_or(0, |d| d.values.threshold);
        let (field, value, name) = match method {
            Method::Brightness => (
                &settings::THRESHOLD,
                settings::percent_of_luma(doc_threshold),
                "threshold",
            ),
            Method::RemoveBackground => (
                &settings::COLOUR_TOLERANCE,
                f64::from(self.settings.remove_tolerance),
                "colour-tolerance",
            ),
            Method::PickColours => (
                &settings::COLOUR_TOLERANCE,
                f64::from(self.settings.pick_tolerance),
                "colour-tolerance",
            ),
        };
        let (v, dragging) = field_widget(ctx, ui, rect, name, field, value, &mut self.fields[0])?;
        // invariant: the field clamps to 0..=100.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let whole = v.round().clamp(0.0, 100.0) as u8;
        match method {
            Method::Brightness => {
                if let Some(doc) = &mut self.doc {
                    doc.values.threshold = settings::luma_of_percent(v);
                    doc.threshold_set = true;
                }
            }
            Method::RemoveBackground => self.settings.remove_tolerance = whole,
            Method::PickColours => self.settings.pick_tolerance = whole,
        }
        Some(dragging)
    }

    /// The expander and the blue button at the far right of the last row.
    fn tail(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        state: &Button,
        blue_w: f32,
        origin: egui::Pos2,
    ) {
        let open = self.settings.advanced_open;
        let changed_text = self.doc.as_ref().and_then(|d| {
            tracyo_core::method::changed_advanced_with_parts(
                &self.settings,
                &d.values,
                d.parts.len(),
            )
        });
        let expander = controls::expander(
            ui,
            &mut self.icons,
            Rect::from_min_size(origin, vec2(28.0, 28.0)),
            bar_id("expander"),
            open,
            !open && changed_text.is_some(),
        );
        let mut lines = plain(if open {
            "Fewer trace options"
        } else {
            "More trace options"
        });
        if let (false, Some(text)) = (open, &changed_text) {
            lines.extend(plain(text));
        }
        widgets::tooltip(ctx, &expander, Side::Below, &lines);
        widgets::reset_tooltip(ctx, &expander);
        widgets::release_focus_after_mouse(ctx, &expander);
        if expander.clicked() {
            self.settings.advanced_open = !open;
            self.save_settings();
        }

        let blue_rect = Rect::from_min_size(origin + vec2(28.0 + PAD, 0.0), vec2(blue_w, 28.0));
        let reason = self.result.as_ref().and_then(|r| r.reason.clone());
        let dimmed = matches!(state, Button::Rest) && reason.is_some();
        let blue = widgets::blue_button(
            ui,
            &mut self.icons,
            blue_rect,
            bar_id("blue"),
            state,
            match state {
                Button::Running(_) if self.reading => "Reading text",
                Button::Running(_) => "Tracing",
                _ => "Save as\u{2026}",
            },
            dimmed,
            blue_w,
        );
        let tip: Vec<Line> = match state {
            Button::Rest if dimmed => plain(&reason.clone().unwrap_or_default()),
            Button::Rest => plain(&self.save_tooltip()),
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

    fn empty_bar(&mut self, ui: &mut egui::Ui, size: Vec2) {
        let label_w = widgets::text_width(ui, "Trace", &theme::text12());
        let button_w = widgets::text_width(ui, "Open image\u{2026}", &theme::text14()) + 16.0;
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
        let method_rows = Method::ALL.map(|m| Row {
            name: m.label(),
            detail: m.detail(),
        });
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
            MenuKind::Method => (&method_rows, usize::from(self.settings.method)),
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
                self.close_menu(ctx);
                match kind {
                    MenuKind::Preset => {
                        self.settings = self.settings.with_preset(i);
                        self.values_changed(ctx, false);
                    }
                    MenuKind::Method => {
                        self.set_method(ctx, Method::from_index(u8::try_from(i).unwrap_or(0)));
                    }
                }
            }
        }
    }
}

/// The size of the bar without a picture: "Trace", 12 px, "Open image…".
fn empty_size(ui: &egui::Ui) -> Vec2 {
    let label_w = widgets::text_width(ui, "Trace", &theme::text12());
    let button_w = widgets::text_width(ui, "Open image\u{2026}", &theme::text14()) + 16.0;
    vec2(PAD + label_w + 12.0 + button_w + PAD, ROW)
}

pub(crate) fn bar_id(name: &str) -> Id {
    Id::unique("bar").with(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Open, About, Preset, Method, Threshold, Smooth, Tolerance, Ignore below.
    const WIDTHS: [f32; 8] = [28.0, 28.0, 132.0, 176.0, 150.0, 150.0, 150.0, 171.0];

    #[test]
    fn at_rest_the_bar_is_one_row_of_1190_px() {
        let f = flow(&WIDTHS, 28.0 + 8.0 + 89.0, 1256.0 - 16.0, false);
        assert_eq!(f.size, vec2(1190.0, 36.0));
        assert_eq!(f.items[0], pos2(8.0, 4.0), "Open is the first control");
        assert_eq!(f.items[1], pos2(8.0 + 28.0 + 8.0, 4.0), "About 44..72");
        assert_eq!(f.items[2].x, 80.0, "Preset 80..212");
        assert_eq!(f.items[3].x, 220.0, "Method 220..396");
        assert_eq!(f.tail.x + 28.0 + 8.0 + 89.0 + 8.0, 1190.0);
        assert_eq!(f.tail.x + 28.0 + 8.0, 1093.0, "Save as… 1093..1182");
    }

    #[test]
    fn at_800_wide_the_bar_wraps_by_whole_items_into_two_rows() {
        let f = flow(&WIDTHS, 125.0, 776.0 - 16.0, false);
        assert_eq!(f.size.y, 72.0);
        // Open to Smooth fit the first row (720 px); Tolerance starts row two.
        assert_eq!(f.items[5].y, 4.0);
        assert_eq!(f.items[6], pos2(8.0, 40.0));
        assert!(f.tail.y >= 40.0);
        assert!(f.size.x <= 776.0);
        // The last group sits at the far right of its row.
        assert!((f.tail.x + 125.0 + 8.0 - f.size.x).abs() < 1e-3);
    }

    #[test]
    fn an_open_block_adds_one_gap_to_the_bar() {
        let rest = flow(&WIDTHS, 125.0, 1240.0, true);
        assert_eq!(rest.size, vec2(1198.0, 36.0));
        // The blue group stays flush right: the extra gap sits before it.
        assert_eq!(rest.tail.x + 125.0 + 8.0, 1198.0);
        let running = flow(&WIDTHS, 28.0 + 8.0 + 75.0, 1240.0, true);
        assert_eq!(running.size.x, 1184.0);
    }

    #[test]
    fn a_wider_blue_button_wraps_the_group_when_the_cap_is_met() {
        // Stopped is 106 px wide; the bar is capped at 1256 px.
        let f = flow(&WIDTHS, 28.0 + 8.0 + 106.0, 1240.0, true);
        assert_eq!(f.size.x, 1215.0, "Stopped fits on one row at 1280");
        assert_eq!(f.size.y, 36.0);
        let narrow = flow(&WIDTHS, 28.0 + 8.0 + 106.0, 1180.0, true);
        assert_eq!(narrow.size, vec2(1196.0, 72.0));
        assert_eq!(narrow.tail.y, 40.0);
        assert_eq!(narrow.items[7].y, 4.0);
    }

    #[test]
    fn remove_background_takes_the_whole_width_and_wraps_the_blue_group() {
        let widths = [28.0, 28.0, 132.0, 176.0, 150.0, 168.0, 150.0, 150.0, 171.0];
        let f = flow(&widths, 125.0, 1240.0, false);
        assert_eq!(f.size, vec2(1256.0, 72.0));
        // Ignore below still fits the first row (px-09 shows it there); only the expander and
        // the blue button wrap.
        assert_eq!(f.items[8].y, 4.0);
        assert_eq!(f.tail.y, 40.0);
        assert!((f.tail.x + 125.0 + 8.0 - 1256.0).abs() < 1e-3);
    }

    #[test]
    fn the_bar_is_centred_on_its_rest_width_and_keeps_its_left_edge() {
        // 1280 wide: the rest bar of 1190 px stands at x = 45; opening Advanced (1198) and
        // running (1176) move only the right end.
        assert_eq!(bar_x(1280.0, 1190.0, 1190.0), 45.0);
        assert_eq!(bar_x(1280.0, 1190.0, 1198.0), 45.0);
        assert_eq!(bar_x(1280.0, 1190.0, 1176.0), 45.0);
        // The empty bar of 165 px stands at x = 557.
        assert_eq!(bar_x(1280.0, 165.0, 165.0), 557.0);
        // A wider window widens both margins; a narrow one pulls the bar left and then to 12 px.
        assert_eq!(bar_x(1600.0, 1190.0, 1190.0), 205.0);
        assert_eq!(bar_x(1210.0, 1190.0, 1198.0), 12.0);
        assert_eq!(bar_x(1230.0, 1190.0, 1198.0), 20.0, "pulled left by 8 px");
        // A rest bar that fills the cap starts at the margin.
        assert_eq!(bar_x(1280.0, 1256.0, 1256.0), 12.0);
    }

    #[test]
    fn the_block_puts_analyse_flush_right_and_wraps_whole_items() {
        let widths = [60.0, 185.0, 150.0, 150.0];
        let b = block_flow(&widths, &[36.0; 4], 75.0, 1142.0, 36.0);
        assert_eq!(b.height, 36.0 + 36.0, "the Parts row is a line of its own");
        assert_eq!(b.parts, pos2(0.0, 36.0 + 4.0));
        assert_eq!(b.items[0], pos2(0.0, 4.0));
        assert_eq!(b.items[1].x, 68.0);
        assert_eq!(b.tail.x, 1142.0 - 75.0);
        let narrow = block_flow(&widths, &[36.0; 4], 75.0, 500.0, 36.0);
        assert!(narrow.height > 72.0);
        assert!(narrow.items.iter().all(|p| p.x + 150.0 <= 500.0 + 40.0));
    }

    #[test]
    fn a_tall_item_makes_its_line_as_tall_and_the_next_line_starts_below_it() {
        let widths = [700.0, 150.0, 150.0];
        let b = block_flow(&widths, &[92.0, 36.0, 36.0], 75.0, 800.0, 36.0);
        // Denoise and Upscale do not fit beside the 700 px item and its gap.
        assert_eq!(b.items[0], pos2(0.0, 4.0));
        assert_eq!(b.items[1], pos2(0.0, 96.0));
        assert_eq!(b.items[2], pos2(158.0, 96.0));
        assert_eq!(b.height, 92.0 + 36.0 + 36.0);
        assert_eq!(b.parts, pos2(0.0, 92.0 + 36.0 + 4.0));
        let beside = block_flow(&[300.0, 150.0], &[92.0, 36.0], 75.0, 1000.0, 36.0);
        assert_eq!(
            beside.items[1],
            pos2(308.0, 4.0),
            "a short item sits at the line's top"
        );
        assert_eq!(beside.height, 92.0 + 36.0);
    }

    #[test]
    fn the_colours_tooltip_names_what_the_field_shows() {
        let tip = field_tip(&settings::COLOURS, 1.0, Some("Auto 3"));
        assert_eq!(tip[0].text, "Colours: Auto 3");
        assert_eq!(tip[1].text, "Click to set, double-click to type");
        let plain = field_tip(&settings::SMOOTH, 80.0, None);
        assert_eq!(plain[0].text, "Smooth: 80");
    }
}
