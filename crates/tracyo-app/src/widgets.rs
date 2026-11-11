//! The custom controls of the Trace bar, painted as ux-notes sections 3 to 5 describe them.

use eframe::egui::{
    self, Align2, Color32, CornerRadius, FontId, Id, Key, Modifiers, Pos2, Rect, Response, Role,
    Sense, Stroke, StrokeKind, Ui, WidgetInfo, pos2, vec2,
};
use tracyo_core::progress::Button;
use tracyo_core::settings::{Field, Stride};

use crate::icons::{Icon, Icons};
use crate::theme;

/// Whether the last focus change came from the keyboard (a focus ring is drawn only then).
#[must_use]
pub fn focus_visible(ctx: &egui::Context) -> bool {
    ctx.data(|d| d.get_temp::<bool>(Id::unique("focus-visible")))
        .unwrap_or(false)
}

/// Remembers whether focus is keyboard focus.
pub fn set_focus_visible(ctx: &egui::Context, visible: bool) {
    ctx.data_mut(|d| d.insert_temp(Id::unique("focus-visible"), visible));
}

fn radius(r: f32) -> CornerRadius {
    // invariant: radii are small positive numbers.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    CornerRadius::same(r.round() as u8)
}

/// Text at `pos` with the given anchor; returns its rectangle.
pub fn text(ui: &Ui, pos: Pos2, anchor: Align2, text: &str, font: FontId, colour: Color32) -> Rect {
    ui.painter().text(pos, anchor, text, font, colour)
}

/// The width of `text` in `font`.
#[must_use]
pub fn text_width(ui: &Ui, text: &str, font: &FontId) -> f32 {
    ui.painter()
        .layout_no_wrap(text.to_owned(), font.clone(), Color32::BLACK)
        .size()
        .x
}

/// The text cut with "…" so it fits `max_width`.
#[must_use]
pub fn truncate(ui: &Ui, text: &str, font: &FontId, max_width: f32) -> String {
    if text_width(ui, text, font) <= max_width {
        return text.to_owned();
    }
    let mut chars: Vec<char> = text.chars().collect();
    while !chars.is_empty() {
        chars.pop();
        let candidate: String = chars.iter().collect::<String>() + "…";
        if text_width(ui, &candidate, font) <= max_width {
            return candidate;
        }
    }
    "…".to_owned()
}

/// The focus ring: 2 px accent outside a 1 px gap (or none).
fn ring(ui: &Ui, rect: Rect, corner: f32, gap: bool) {
    let grow = if gap { 1.0 } else { 0.0 };
    ui.painter().rect_stroke(
        rect.expand(grow),
        radius(corner + grow),
        Stroke::new(2.0, theme::ACCENT),
        StrokeKind::Outside,
    );
}

fn labelled(resp: &Response, kind: Role, name: &str) {
    resp.widget_info(|| WidgetInfo::labeled(kind, true, name));
}

/// An icon button of 28 x 28 (About, pipettes, toggles, the expander).
pub fn icon_button(
    ui: &mut Ui,
    icons: &mut Icons,
    rect: Rect,
    id: Id,
    icon: Icon,
    name: &str,
    pressed: bool,
) -> Response {
    let resp = ui.interact(rect, id, Sense::click());
    labelled(&resp, Role::Button, name);
    let hover = resp.hovered() && ui.is_enabled();
    if pressed {
        ui.painter()
            .rect_filled(rect, radius(theme::RADIUS_BUTTON), theme::ACCENT);
    } else if hover {
        ui.painter()
            .rect_filled(rect, radius(theme::RADIUS_BUTTON), theme::HOVER_ON_BAR);
    }
    let colour = if pressed {
        Color32::WHITE
    } else {
        theme::TOOLBAR_ICON
    };
    icons.paint(ui, rect, icon, 16.0, 1.5, colour);
    if resp.has_focus() && focus_visible(ui.ctx()) {
        ring(ui, rect, theme::RADIUS_BUTTON, true);
    }
    resp
}

/// The Preset and Method buttons: label left, chevron right, a border, hover ground.
pub fn drop_button(
    ui: &mut Ui,
    rect: Rect,
    id: Id,
    label: &str,
    name: &str,
    open: bool,
) -> Response {
    let resp = ui.interact(rect, id, Sense::click());
    labelled(&resp, Role::Button, name);
    if resp.hovered() || open {
        ui.painter()
            .rect_filled(rect, radius(theme::RADIUS_BUTTON), theme::HOVER_ON_BAR);
    }
    ui.painter().rect_stroke(
        rect,
        radius(theme::RADIUS_BUTTON),
        Stroke::new(1.0, theme::BUTTON_BORDER),
        StrokeKind::Inside,
    );
    let font = theme::text14();
    let shown = truncate(ui, label, &font, rect.width() - 8.0 - 8.0 - 10.0 - 6.0);
    text(
        ui,
        pos2(rect.left() + 8.0, rect.center().y),
        Align2::LEFT_CENTER,
        &shown,
        font,
        theme::TOOLBAR_ICON,
    );
    chevron(
        ui,
        pos2(rect.right() - 8.0 - 5.0, rect.center().y),
        theme::TOOLBAR_ICON,
    );
    if resp.has_focus() && focus_visible(ui.ctx()) {
        ring(ui, rect, theme::RADIUS_BUTTON, false);
    }
    resp
}

/// Curvyo's own chevron: `M2 3.5 L5 6.5 L8 3.5` in a 10 x 10 box, 1.5 px.
fn chevron(ui: &Ui, centre: Pos2, colour: Color32) {
    let o = centre - vec2(5.0, 5.0);
    let points = vec![o + vec2(2.0, 3.5), o + vec2(5.0, 6.5), o + vec2(8.0, 3.5)];
    ui.painter()
        .add(egui::Shape::line(points, Stroke::new(1.5, colour)));
}

/// A text button without border (the empty state's "Open image…").
pub fn text_button(ui: &mut Ui, rect: Rect, id: Id, label: &str, name: &str) -> Response {
    let resp = ui.interact(rect, id, Sense::click());
    labelled(&resp, Role::Button, name);
    if resp.hovered() {
        ui.painter()
            .rect_filled(rect, radius(theme::RADIUS_BUTTON), theme::HOVER_ON_BAR);
    }
    text(
        ui,
        rect.center(),
        Align2::CENTER_CENTER,
        label,
        theme::text14(),
        theme::TOOLBAR_ICON,
    );
    if resp.has_focus() && focus_visible(ui.ctx()) {
        ring(ui, rect, theme::RADIUS_BUTTON, false);
    }
    resp
}

/// What a value field did this frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FieldEvent {
    /// Nothing.
    None,
    /// The value is now this; `dragging` while the pointer still holds it (reduced preview).
    Set {
        /// The new value.
        value: f64,
        /// Whether the pointer is still down.
        dragging: bool,
    },
}

/// The open number entry and the drag of a value field.
#[derive(Debug, Default, Clone)]
pub struct FieldState {
    /// The text of an open entry.
    pub entry: Option<String>,
    pressing: bool,
    just_opened: bool,
}

impl FieldState {
    /// Whether the entry is open.
    #[must_use]
    pub fn entry_open(&self) -> bool {
        self.entry.is_some()
    }

    /// Closes the entry without committing.
    pub fn cancel_entry(&mut self) {
        self.entry = None;
    }
}

fn quantise(field: &Field, fraction: f64, modifiers: Modifiers) -> f64 {
    let quantum = if modifiers.shift {
        field.big_step
    } else if modifiers.ctrl || modifiers.command {
        field.fine_step
    } else {
        field.step
    };
    field.clamp((field.at_fraction(fraction) / quantum).round() * quantum)
}

fn stride_of(modifiers: Modifiers) -> Stride {
    if modifiers.shift {
        Stride::Big
    } else if modifiers.ctrl || modifiers.command {
        Stride::Fine
    } else {
        Stride::Step
    }
}

/// A value field: label, value and unit over a bar; click and drag set, keys step, double-click
/// opens the number entry.
#[allow(clippy::too_many_lines)]
pub fn value_field(
    ui: &mut Ui,
    rect: Rect,
    id: Id,
    field: &Field,
    value: f64,
    state: &mut FieldState,
) -> (FieldEvent, Response) {
    let resp = ui.interact(rect, id, Sense::click_and_drag());
    labelled(&resp, Role::SpinButton, field.label);
    resp.widget_info(|| {
        let mut info = WidgetInfo::labeled(Role::SpinButton, true, field.label);
        info.value = Some(value);
        info
    });
    let mut event = FieldEvent::None;
    let modifiers = ui.input(|i| i.modifiers);
    let editing = state.entry.is_some();

    if !editing {
        // Pointer: press sets, drag changes, release settles.
        let down = resp.is_pointer_button_down_on() && ui.input(|i| i.pointer.primary_down());
        if down {
            if let Some(pos) = ui.input(|i| i.pointer.interact_pos()) {
                let fraction = f64::from((pos.x - rect.left()) / rect.width());
                let new = quantise(field, fraction, modifiers);
                if (new - value).abs() > 1e-9 || !state.pressing {
                    event = FieldEvent::Set {
                        value: new,
                        dragging: true,
                    };
                }
            }
            state.pressing = true;
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
        } else if state.pressing {
            state.pressing = false;
            event = FieldEvent::Set {
                value,
                dragging: false,
            };
        } else if resp.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
        }
        if resp.double_clicked() {
            open_entry(state, field, value);
        }
        // Keys.
        if resp.has_focus() {
            let mut key_value = None;
            ui.input_mut(|i| {
                for (key, dir) in [
                    (Key::ArrowLeft, -1),
                    (Key::ArrowDown, -1),
                    (Key::ArrowRight, 1),
                    (Key::ArrowUp, 1),
                ] {
                    for mods in [
                        Modifiers::NONE,
                        Modifiers::SHIFT,
                        Modifiers::CTRL,
                        Modifiers::COMMAND,
                    ] {
                        if i.consume_key(mods, key) {
                            key_value = Some(field.nudge(value, dir, stride_of(mods)));
                        }
                    }
                }
                if i.consume_key(Modifiers::NONE, Key::Home) {
                    key_value = Some(field.min);
                }
                if i.consume_key(Modifiers::NONE, Key::End) {
                    key_value = Some(field.max);
                }
            });
            if let Some(v) = key_value {
                event = FieldEvent::Set {
                    value: v,
                    dragging: false,
                };
            }
            let open = ui.input_mut(|i| {
                i.consume_key(Modifiers::NONE, Key::Enter)
                    || i.consume_key(Modifiers::NONE, Key::F2)
                    || (i.events.iter().any(|e| matches!(e, egui::Event::Text(t) if t.chars().all(|c| c.is_ascii_digit() || c == '.' || c == ',') && !t.is_empty())))
            });
            if open {
                open_entry(state, field, value);
            }
        }
    }

    paint_field(
        ui,
        rect,
        field,
        value,
        state,
        resp.hovered() || state.pressing,
        modifiers,
    );

    if state.entry.is_some() {
        let committed = entry_ui(ui, rect, id, field, state);
        if let Some(v) = committed {
            event = FieldEvent::Set {
                value: v,
                dragging: false,
            };
        }
    } else if resp.has_focus() && focus_visible(ui.ctx()) {
        ring(ui, rect, theme::RADIUS_FIELD, true);
    }
    (event, resp)
}

fn open_entry(state: &mut FieldState, field: &Field, value: f64) {
    state.entry = Some(field.format(value));
    state.just_opened = true;
}

fn paint_field(
    ui: &Ui,
    rect: Rect,
    field: &Field,
    value: f64,
    state: &FieldState,
    hover: bool,
    modifiers: Modifiers,
) {
    let painter = ui.painter();
    let corner = radius(theme::RADIUS_FIELD);
    painter.rect_filled(rect, corner, Color32::WHITE);
    let editing = state.entry.is_some();
    let inner = rect.shrink(1.0);
    let label_font = theme::text14();
    let label_rect = Rect::from_min_size(
        pos2(rect.left() + 8.0, rect.center().y - 10.0),
        vec2(text_width(ui, field.label, &label_font), 20.0),
    );
    let value_text = field.format(value);
    let unit_width = if field.unit.is_empty() {
        0.0
    } else {
        4.0 + text_width(ui, field.unit, &label_font)
    };
    let value_width = text_width(ui, &value_text, &label_font);
    let value_right = rect.right() - 8.0 - unit_width;
    let value_rect = Rect::from_min_size(
        pos2(value_right - value_width, rect.center().y - 10.0),
        vec2(value_width, 20.0),
    );
    if !editing {
        let fraction = field.fraction(value) as f32;
        let edge_x =
            (rect.left() + fraction * rect.width()).clamp(inner.left() + 2.0, inner.right());
        let bar = Rect::from_min_max(inner.min, pos2(edge_x, inner.max.y));
        let bar_radius = CornerRadius {
            nw: 4,
            sw: 4,
            ne: if fraction >= 0.999 { 4 } else { 0 },
            se: if fraction >= 0.999 { 4 } else { 0 },
        };
        painter.rect_filled(bar, bar_radius, theme::VALUE_FILL);
        let edge = Rect::from_min_max(
            pos2(edge_x - 2.0, inner.top()),
            pos2(edge_x, inner.bottom()),
        );
        let crosses = |r: Rect| edge.right() > r.left() - 1.0 && edge.left() < r.right() + 1.0;
        if crosses(label_rect) || crosses(value_rect) {
            let top = Rect::from_min_max(edge.min, pos2(edge.max.x, edge.min.y + 6.0));
            let bottom = Rect::from_min_max(pos2(edge.min.x, edge.max.y - 6.0), edge.max);
            painter.rect_filled(top, 0.0, theme::ACCENT);
            painter.rect_filled(bottom, 0.0, theme::ACCENT);
        } else {
            painter.rect_filled(edge, 0.0, theme::ACCENT);
        }
    }
    let border = if editing {
        Color32::TRANSPARENT
    } else if hover {
        theme::TOOLBAR_ICON
    } else {
        theme::FIELD_BORDER
    };
    if editing {
        painter.rect_stroke(
            rect,
            corner,
            Stroke::new(1.0, theme::ACCENT),
            StrokeKind::Inside,
        );
        painter.rect_stroke(
            rect.shrink(1.0),
            radius(theme::RADIUS_FIELD - 1.0),
            Stroke::new(1.0, theme::ACCENT),
            StrokeKind::Inside,
        );
    } else {
        painter.rect_stroke(rect, corner, Stroke::new(1.0, border), StrokeKind::Inside);
    }
    text(
        ui,
        label_rect.left_center(),
        Align2::LEFT_CENTER,
        field.label,
        label_font.clone(),
        theme::TOOLBAR_ICON,
    );
    if !editing {
        text(
            ui,
            value_rect.left_center(),
            Align2::LEFT_CENTER,
            &value_text,
            label_font.clone(),
            theme::TOOLBAR_ICON,
        );
    }
    if !field.unit.is_empty() {
        text(
            ui,
            pos2(rect.right() - 8.0, rect.center().y),
            Align2::RIGHT_CENTER,
            field.unit,
            label_font,
            theme::MUTED,
        );
    }
    if state.pressing && (modifiers.shift || modifiers.ctrl || modifiers.command) {
        let word = if modifiers.shift { "coarse" } else { "fine" };
        text(
            ui,
            rect.center(),
            Align2::CENTER_CENTER,
            word,
            theme::text12(),
            theme::MUTED,
        );
    }
}

/// The text entry inside the field; returns the committed value.
fn entry_ui(ui: &mut Ui, rect: Rect, id: Id, field: &Field, state: &mut FieldState) -> Option<f64> {
    let unit_width = if field.unit.is_empty() {
        0.0
    } else {
        4.0 + text_width(ui, field.unit, &theme::text14())
    };
    let entry_rect = Rect::from_min_max(
        pos2(
            rect.left() + 8.0 + text_width(ui, field.label, &theme::text14()) + 4.0,
            rect.top() + 2.0,
        ),
        pos2(rect.right() - 8.0 - unit_width, rect.bottom() - 2.0),
    );
    let entry_id = id.with("entry");
    let mut text_value = state.entry.clone().unwrap_or_default();
    let output = ui.put(
        entry_rect,
        egui::TextEdit::singleline(&mut text_value)
            .id(entry_id)
            .font(theme::text14())
            .frame(egui::Frame::NONE)
            .align(Align2::RIGHT_CENTER)
            .margin(egui::Margin::ZERO)
            .text_color(theme::TOOLBAR_ICON),
    );
    state.entry = Some(text_value.clone());
    if state.just_opened {
        state.just_opened = false;
        output.request_focus();
        let mut edit_state =
            egui::text_edit::TextEditState::load(ui.ctx(), entry_id).unwrap_or_default();
        let end = egui::text::CCursor::new(text_value.chars().count());
        edit_state
            .cursor
            .set_char_range(Some(egui::text::CCursorRange::two(
                egui::text::CCursor::new(0),
                end,
            )));
        edit_state.store(ui.ctx(), entry_id);
        return None;
    }
    let enter = ui.input(|i| i.key_pressed(Key::Enter));
    let tab = ui.input(|i| i.key_pressed(Key::Tab));
    let ctrl_enter = ui.input(|i| i.key_pressed(Key::Enter) && i.modifiers.command);
    if enter || tab || ctrl_enter || output.lost_focus() {
        state.entry = None;
        if let Some(v) = field.parse(&text_value) {
            return Some(v);
        }
    }
    None
}

/// The blue finish button in all its states; returns its response and, in the running look,
/// the response of its x.
pub struct BlueButton {
    /// The button itself (or Retry in Stopped and Failed).
    pub main: Response,
    /// The x inside a running button.
    pub stop: Option<Response>,
}

/// The width of the blue button in a state.
#[must_use]
pub fn blue_width(ui: &Ui, state: &Button) -> f32 {
    match state {
        Button::Rest | Button::Running(_) => 89.0,
        Button::Stopped => {
            12.0 + text_width(ui, "Stopped", &theme::text12())
                + 8.0
                + text_width(ui, "Retry", &theme::text14())
                + 12.0
        }
        Button::Failed(_) => {
            12.0 + 12.0
                + 4.0
                + text_width(ui, "Failed", &theme::text12())
                + 8.0
                + text_width(ui, "Retry", &theme::text14())
                + 12.0
        }
    }
}

/// Paints and drives the blue button at `rect`.
#[allow(clippy::too_many_lines)]
pub fn blue_button(
    ui: &mut Ui,
    icons: &mut Icons,
    rect: Rect,
    id: Id,
    state: &Button,
    label: &str,
    dimmed: bool,
    width: f32,
) -> BlueButton {
    let rect = Rect::from_min_size(rect.min, vec2(width, rect.height()));
    let corner = radius(theme::RADIUS_BUTTON);
    let sense = if dimmed {
        Sense::hover()
    } else {
        Sense::click()
    };
    let main = ui.interact(rect, id, sense);
    let mut stop = None;
    match state {
        Button::Rest => {
            labelled(&main, Role::Button, label);
            let fill = if dimmed {
                theme::DIMMED_BLUE
            } else if main.hovered() {
                theme::BLUE_HOVER
            } else {
                theme::ACCENT
            };
            ui.painter().rect_filled(rect, corner, fill);
            let colour = if dimmed {
                theme::DIMMED_TEXT
            } else {
                Color32::WHITE
            };
            text(
                ui,
                rect.center(),
                Align2::CENTER_CENTER,
                label,
                theme::text14(),
                colour,
            );
            if main.has_focus() && focus_visible(ui.ctx()) && !dimmed {
                ring(ui, rect, theme::RADIUS_BUTTON, true);
            }
        }
        Button::Running(percent) => {
            ui.painter().rect_filled(rect, corner, theme::ACCENT);
            if let Some(p) = percent {
                let fill_w = rect.width() * f32::from(*p) / 100.0;
                let fill = Rect::from_min_size(rect.min, vec2(fill_w, rect.height()));
                let r = CornerRadius {
                    nw: 8,
                    sw: 8,
                    ne: if *p >= 100 { 8 } else { 0 },
                    se: if *p >= 100 { 8 } else { 0 },
                };
                ui.painter().rect_filled(fill, r, theme::RUNNING_FILL);
            }
            let shown = percent.map_or_else(|| "...".to_owned(), |p| format!("{p} %"));
            text(
                ui,
                pos2(rect.left() + 8.0, rect.center().y),
                Align2::LEFT_CENTER,
                &shown,
                theme::text12(),
                Color32::WHITE,
            );
            let x_rect = Rect::from_min_size(
                pos2(rect.right() - 28.0, rect.top()),
                vec2(28.0, rect.height()),
            );
            let x_resp = ui.interact(x_rect, id.with("stop"), Sense::click());
            labelled(&x_resp, Role::Button, "Stop (Esc)");
            if x_resp.hovered() {
                ui.painter()
                    .rect_filled(x_rect, corner, theme::WHITE_20_ON_BLUE);
            }
            icons.paint(ui, x_rect, Icon::X, 16.0, 1.5, Color32::WHITE);
            if x_resp.has_focus() && focus_visible(ui.ctx()) {
                ring(ui, x_rect, theme::RADIUS_BUTTON, true);
            }
            stop = Some(x_resp);
        }
        Button::Stopped | Button::Failed(_) => {
            ui.painter().rect_filled(rect, corner, theme::ACCENT);
            let retry_width = text_width(ui, "Retry", &theme::text14());
            let retry_rect = Rect::from_min_max(
                pos2(rect.right() - 12.0 - retry_width - 4.0, rect.top()),
                rect.max,
            );
            let retry = ui.interact(retry_rect, id.with("retry"), Sense::click());
            labelled(&retry, Role::Button, "Retry");
            if retry.hovered() {
                ui.painter().rect_filled(
                    retry_rect.shrink2(vec2(0.0, 0.0)),
                    corner,
                    theme::WHITE_20_ON_BLUE,
                );
            }
            text(
                ui,
                pos2(rect.right() - 12.0, rect.center().y),
                Align2::RIGHT_CENTER,
                "Retry",
                theme::text14(),
                Color32::WHITE,
            );
            let mut x = rect.left() + 12.0;
            if matches!(state, Button::Failed(_)) {
                icons.paint(
                    ui,
                    Rect::from_min_size(pos2(x, rect.center().y - 6.0), vec2(12.0, 12.0)),
                    Icon::CircleAlert,
                    12.0,
                    2.0,
                    Color32::WHITE,
                );
                x += 12.0 + 4.0;
            }
            let word = if matches!(state, Button::Failed(_)) {
                "Failed"
            } else {
                "Stopped"
            };
            text(
                ui,
                pos2(x, rect.center().y),
                Align2::LEFT_CENTER,
                word,
                theme::text12(),
                if matches!(state, Button::Failed(_)) {
                    Color32::WHITE
                } else {
                    theme::STOPPED_TEXT
                },
            );
            if retry.has_focus() && focus_visible(ui.ctx()) {
                ring(ui, retry_rect, theme::RADIUS_BUTTON, true);
            }
            return BlueButton {
                main: retry,
                stop: None,
            };
        }
    }
    BlueButton { main, stop }
}

/// The preferred side of a tooltip relative to its control.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    /// Below the control, 8 px away.
    Below,
    /// Left of the control.
    Left,
    /// Right of the control, 10 px away.
    Right,
}

/// One line of a tooltip: text, font and colour.
pub struct Line {
    /// The text.
    pub text: String,
    /// The font.
    pub font: FontId,
    /// The colour.
    pub colour: Color32,
}

/// Shows the house tooltip for `resp` after 400 ms of hover.
pub fn tooltip(ctx: &egui::Context, resp: &Response, side: Side, lines: &[Line]) {
    if lines.is_empty() || !resp.hovered() || resp.dragged() || ctx.input(|i| i.pointer.any_down())
    {
        return;
    }
    let shown = resp.enabled() && resp.hovered();
    let delay = ctx.global_style().interaction.tooltip_delay;
    let now = ctx.input(|i| i.time);
    let since = ctx.data_mut(|d| {
        *d.get_temp_mut_or_insert_with(Id::unique("tooltip-since").with(resp.id), || now)
    });
    let waited = now - since;
    if !shown || waited < f64::from(delay) {
        ctx.request_repaint_after_secs((f64::from(delay) - waited).max(0.0) as f32 + 0.01);
        return;
    }
    let layouts: Vec<_> = lines
        .iter()
        .map(|l| ctx.fonts_mut(|f| f.layout(l.text.clone(), l.font.clone(), l.colour, 240.0)))
        .collect();
    let width = layouts.iter().map(|g| g.size().x).fold(0.0_f32, f32::max) + 16.0;
    let height = layouts.iter().map(|g| g.size().y).sum::<f32>() + 8.0;
    let screen = ctx.content_rect();
    let size = vec2(width, height);
    let anchor = resp.rect;
    let mut pos = match side {
        Side::Below => pos2(anchor.center().x - size.x / 2.0, anchor.bottom() + 8.0),
        Side::Left => pos2(
            anchor.left() - 8.0 - size.x,
            anchor.center().y - size.y / 2.0,
        ),
        Side::Right => pos2(anchor.right() + 10.0, anchor.center().y - size.y / 2.0),
    };
    if side == Side::Below && pos.y + size.y > screen.bottom() {
        pos.y = anchor.top() - 8.0 - size.y;
    }
    if side == Side::Left && pos.x < screen.left() {
        pos.x = anchor.right() + 8.0;
    }
    pos.x = pos.x.clamp(
        screen.left() + 2.0,
        (screen.right() - size.x - 2.0).max(screen.left()),
    );
    pos.y = pos.y.clamp(
        screen.top() + 2.0,
        (screen.bottom() - size.y - 2.0).max(screen.top()),
    );
    egui::Area::new(Id::unique("tooltip").with(resp.id))
        .order(egui::Order::Tooltip)
        .fixed_pos(pos)
        .interactable(false)
        .show(ctx, |ui| {
            let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
            ui.painter()
                .rect_filled(rect, radius(theme::RADIUS_BUTTON), theme::POPOVER);
            ui.painter().rect_stroke(
                rect,
                radius(theme::RADIUS_BUTTON),
                Stroke::new(1.0, theme::POPOVER_RING),
                StrokeKind::Inside,
            );
            let mut y = rect.top() + 4.0;
            for galley in layouts {
                let h = galley.size().y;
                ui.painter()
                    .galley(pos2(rect.left() + 8.0, y), galley, theme::FOREGROUND);
                y += h;
            }
        });
}

/// Forgets that a tooltip was waiting (call when the pointer is not over `resp`).
pub fn reset_tooltip(ctx: &egui::Context, resp: &Response) {
    if !resp.hovered() {
        ctx.data_mut(|d| d.remove::<f64>(Id::unique("tooltip-since").with(resp.id)));
    }
}

/// A row of a menu.
pub struct Row<'a> {
    /// First line (weight 600).
    pub name: &'a str,
    /// Second line.
    pub detail: &'a str,
}

/// What the menu did this frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuEvent {
    /// Nothing.
    None,
    /// Row `n` was picked.
    Picked(usize),
    /// Closed by a click outside, Tab or the keys.
    Closed,
}

/// The Preset and Method menu below `anchor`; `highlight` is the row under the pointer or keys.
pub fn menu(
    ctx: &egui::Context,
    id: Id,
    anchor: Rect,
    rows: &[Row<'_>],
    current: usize,
    highlight: &mut usize,
) -> MenuEvent {
    let row_h = 44.0;
    let mut width = 232.0_f32;
    let font = theme::text14_bold();
    let detail_font = theme::text12();
    let measure = |text: &str, font: &FontId| {
        ctx.fonts_mut(|f| {
            f.layout_no_wrap(text.to_owned(), font.clone(), Color32::BLACK)
                .size()
                .x
        })
    };
    for row in rows {
        let w = 4.0
            + 8.0
            + 12.0
            + 8.0
            + measure(row.name, &font).max(measure(row.detail, &detail_font) + 0.0)
            + 8.0
            + 4.0;
        width = width.max(w);
    }
    let height = rows.len() as f32 * row_h + 8.0;
    let screen = ctx.content_rect();
    let mut top = anchor.bottom() + 8.0;
    if top + height > screen.bottom() {
        top = anchor.top() - 8.0 - height;
    }
    let size = vec2(width, height);
    let mut event = MenuEvent::None;
    let area = egui::Area::new(id)
        .order(egui::Order::Foreground)
        .fixed_pos(pos2(anchor.left(), top))
        .show(ctx, |ui| {
            let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
            ui.painter()
                .add(theme::SHADOW.as_shape(rect, radius(theme::RADIUS_BAR)));
            ui.painter()
                .rect_filled(rect, radius(theme::RADIUS_BAR), theme::TOOLBAR_BG);
            for (i, row) in rows.iter().enumerate() {
                let row_rect = Rect::from_min_size(
                    pos2(rect.left() + 4.0, rect.top() + 4.0 + i as f32 * row_h),
                    vec2(width - 8.0, row_h),
                );
                let resp = ui.interact(row_rect, id.with(i), Sense::click());
                resp.widget_info(|| WidgetInfo::labeled(Role::Button, true, row.name));
                if resp.hovered() && ui.input(|inp| inp.pointer.delta() != vec2(0.0, 0.0)) {
                    *highlight = i;
                }
                if *highlight == i {
                    ui.painter().rect_filled(
                        row_rect,
                        radius(theme::RADIUS_BUTTON),
                        theme::HOVER_ON_BAR,
                    );
                }
                if i == current {
                    check_mark(
                        ui,
                        pos2(row_rect.left() + 8.0 + 6.0, row_rect.top() + 14.0),
                        theme::TOOLBAR_ICON,
                    );
                }
                text(
                    ui,
                    pos2(
                        row_rect.left() + 8.0 + 12.0 + 8.0 - 8.0 + 8.0,
                        row_rect.top() + 6.0,
                    ),
                    Align2::LEFT_TOP,
                    row.name,
                    font.clone(),
                    theme::TOOLBAR_ICON,
                );
                text(
                    ui,
                    pos2(row_rect.left() + 8.0 + 20.0, row_rect.top() + 6.0 + 20.0),
                    Align2::LEFT_TOP,
                    row.detail,
                    detail_font.clone(),
                    theme::MENU_SECOND,
                );
                if resp.clicked() {
                    event = MenuEvent::Picked(i);
                }
            }
            rect
        });
    let menu_rect = area.inner;
    ctx.input(|i| {
        if i.pointer.any_pressed()
            && let Some(p) = i.pointer.interact_pos()
            && !menu_rect.contains(p)
            && !anchor.contains(p)
        {
            event = MenuEvent::Closed;
        }
    });
    let keys = |k| ctx.input_mut(|i| i.consume_key(Modifiers::NONE, k));
    if keys(Key::ArrowDown) {
        *highlight = (*highlight + 1).min(rows.len() - 1);
    }
    if keys(Key::ArrowUp) {
        *highlight = highlight.saturating_sub(1);
    }
    if keys(Key::Home) {
        *highlight = 0;
    }
    if keys(Key::End) {
        *highlight = rows.len() - 1;
    }
    if keys(Key::Enter) || keys(Key::Space) {
        event = MenuEvent::Picked(*highlight);
    }
    if keys(Key::Tab) {
        event = MenuEvent::Closed;
    }
    event
}

/// The menu's check mark, drawn as a path (Liberation Sans has no U+2713).
fn check_mark(ui: &Ui, centre: Pos2, colour: Color32) {
    let o = centre - vec2(5.0, 4.0);
    let points = vec![o + vec2(1.0, 4.5), o + vec2(3.8, 7.5), o + vec2(9.0, 1.0)];
    ui.painter()
        .add(egui::Shape::line(points, Stroke::new(1.8, colour)));
}
