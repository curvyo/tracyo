//! The segmented pills, text toggles, accent button, swatches and the expander of the Trace bar.

use eframe::egui::{
    self, Align2, Color32, CornerRadius, Id, Key, Modifiers, Pos2, Rect, Response, Role, Sense,
    Stroke, StrokeKind, Ui, pos2, vec2,
};

use crate::icons::{Icon, Icons};
use crate::theme;
use crate::widgets::{self, focus_visible, labelled, radius, ring, text, text_width};

/// Height of every control of the bar.
pub const CONTROL_H: f32 = 28.0;

fn seg_width(ui: &Ui, label: &str, pad: f32) -> f32 {
    (text_width(ui, label, &theme::text14()) + 2.0 * pad).max(CONTROL_H)
}

/// Whether the primary button was released inside `rect` this frame (a click on a control that
/// is not a Tab stop and so has no click sense of its own).
fn released_in(ui: &Ui, rect: Rect) -> bool {
    ui.input(|inp| inp.pointer.primary_released()) && ui.rect_contains_pointer(rect)
}

/// The width of a segmented pill of `labels` with `pad` px of side padding per segment.
#[must_use]
pub fn segmented_width(ui: &Ui, labels: &[&str], pad: f32) -> f32 {
    labels.iter().map(|l| seg_width(ui, l, pad)).sum()
}

/// A segmented pill at `origin` (its top-left corner): a radio group with one Tab stop, the
/// arrows move and select without wrapping. Returns the segment the maker chose this frame.
pub fn segmented(
    ui: &mut Ui,
    origin: Pos2,
    id: Id,
    name: &str,
    labels: &[&str],
    selected: usize,
    pad: f32,
) -> Option<usize> {
    let widths: Vec<f32> = labels.iter().map(|l| seg_width(ui, l, pad)).collect();
    let total: f32 = widths.iter().sum();
    let whole = Rect::from_min_size(origin, vec2(total, CONTROL_H));
    let mut chosen = None;
    let mut x = origin.x;
    for (i, label) in labels.iter().enumerate() {
        let rect = Rect::from_min_size(pos2(x, origin.y), vec2(widths[i], CONTROL_H));
        x += widths[i];
        let on = i == selected;
        let corner = CornerRadius {
            nw: if i == 0 { 8 } else { 0 },
            sw: if i == 0 { 8 } else { 0 },
            ne: if i + 1 == labels.len() { 8 } else { 0 },
            se: if i + 1 == labels.len() { 8 } else { 0 },
        };
        // Only the chosen segment is a Tab stop; the others are plain pointer targets.
        let sense = if on { Sense::click() } else { Sense::hover() };
        let resp = ui.interact(rect, id.with(i), sense);
        let clicked = resp.clicked() || (!on && released_in(ui, rect));
        resp.widget_info(|| {
            egui::WidgetInfo::selected(Role::RadioButton, true, on, format!("{name}: {label}"))
        });
        if on {
            ui.painter().rect_filled(rect, corner, theme::ACCENT);
        } else if resp.hovered() {
            ui.painter().rect_filled(rect, corner, theme::HOVER_ON_BAR);
        }
        text(
            ui,
            rect.center(),
            Align2::CENTER_CENTER,
            label,
            theme::text14(),
            if on {
                Color32::WHITE
            } else {
                theme::TOOLBAR_ICON
            },
        );
        if on && resp.has_focus() && focus_visible(ui.ctx()) {
            ui.painter().rect_stroke(
                rect,
                corner,
                Stroke::new(2.0, theme::ACCENT),
                StrokeKind::Inside,
            );
        }
        if on && resp.has_focus() {
            ui.memory_mut(|m| {
                m.set_focus_lock_filter(
                    resp.id,
                    egui::EventFilter {
                        horizontal_arrows: true,
                        ..Default::default()
                    },
                );
            });
            let step = ui.input_mut(|inp| {
                i32::from(inp.consume_key(Modifiers::NONE, Key::ArrowRight))
                    - i32::from(inp.consume_key(Modifiers::NONE, Key::ArrowLeft))
            });
            if step != 0 {
                let next = if step < 0 {
                    selected.saturating_sub(1)
                } else {
                    (selected + 1).min(labels.len() - 1)
                };
                if next != selected {
                    chosen = Some(next);
                    ui.memory_mut(|m| m.request_focus(id.with(next)));
                }
            }
        }
        if clicked && !on {
            chosen = Some(i);
        }
    }
    ui.painter().rect_stroke(
        whole,
        radius(theme::RADIUS_BUTTON),
        Stroke::new(1.0, theme::PILL_OUTLINE),
        StrokeKind::Inside,
    );
    chosen
}

/// A text button that is pressed or not ("Auto"): blue with white text while on.
pub fn text_toggle(ui: &mut Ui, rect: Rect, id: Id, label: &str, pressed: bool) -> Response {
    let resp = ui.interact(rect, id, Sense::click());
    resp.widget_info(|| egui::WidgetInfo::selected(Role::Button, true, pressed, label));
    if pressed {
        ui.painter()
            .rect_filled(rect, radius(theme::RADIUS_BUTTON), theme::ACCENT);
    } else if resp.hovered() {
        ui.painter()
            .rect_filled(rect, radius(theme::RADIUS_BUTTON), theme::HOVER_ON_BAR);
    }
    text(
        ui,
        rect.center(),
        Align2::CENTER_CENTER,
        label,
        theme::text14(),
        if pressed {
            Color32::WHITE
        } else {
            theme::TOOLBAR_ICON
        },
    );
    if resp.has_focus() && focus_visible(ui.ctx()) {
        ring(ui, rect, theme::RADIUS_BUTTON, false);
    }
    resp
}

/// The blue text button of the Advanced block ("Analyse").
pub fn accent_button(ui: &mut Ui, rect: Rect, id: Id, label: &str) -> Response {
    let resp = ui.interact(rect, id, Sense::click());
    labelled(&resp, Role::Button, label);
    ui.painter().rect_filled(
        rect,
        radius(theme::RADIUS_BUTTON),
        if resp.hovered() {
            theme::BLUE_HOVER
        } else {
            theme::ACCENT
        },
    );
    text(
        ui,
        rect.center(),
        Align2::CENTER_CENTER,
        label,
        theme::text14(),
        Color32::WHITE,
    );
    if resp.has_focus() && focus_visible(ui.ctx()) {
        ring(ui, rect, theme::RADIUS_BUTTON, true);
    }
    resp
}

/// A colour swatch of the panel look (the background chip): a 1 px border, a 1 px white line,
/// the colour over a checkerboard; `None` is fully transparent.
pub fn swatch(ui: &Ui, rect: Rect, colour: Option<Color32>) {
    let painter = ui.painter();
    painter.rect_filled(rect, 5, theme::SWATCH_BORDER);
    let inner = rect.shrink(1.0);
    painter.rect_filled(inner, 4, theme::CHECKER_A);
    if colour.is_none() {
        let cell = if rect.width() <= 16.0 { 4.0 } else { 7.0 };
        let clipped = painter.with_clip_rect(inner);
        let mut y = 0;
        while (y as f32) * cell < inner.height() {
            let mut x = 0;
            while (x as f32) * cell < inner.width() {
                if (x + y) % 2 == 1 {
                    let cell_rect = Rect::from_min_size(
                        inner.min + vec2(x as f32 * cell, y as f32 * cell),
                        vec2(cell, cell),
                    );
                    clipped.rect_filled(cell_rect, 0, theme::CHECKER_B);
                }
                x += 1;
            }
            y += 1;
        }
    }
    if let Some(c) = colour {
        painter.rect_filled(inner, 4, c);
    }
    painter.rect_stroke(
        inner,
        4,
        Stroke::new(1.0, Color32::WHITE),
        StrokeKind::Inside,
    );
}

/// `colour` at `alpha` over the bar's ground (what a faded swatch looks like).
fn over_bar(colour: Color32, alpha: f32) -> Color32 {
    let mix = |a: u8, b: u8| (f32::from(a) * (1.0 - alpha) + f32::from(b) * alpha).round() as u8;
    let bar = theme::TOOLBAR_BG;
    Color32::from_rgb(
        mix(bar.r(), colour.r()),
        mix(bar.g(), colour.g()),
        mix(bar.b(), colour.b()),
    )
}

/// One picked colour of a swatch row: a 24 x 28 cell around the 20 px swatch; on hover or focus
/// the swatch fades to 40 % and an x shows over it. The flag is whether it was pressed.
pub fn swatch_cell(
    ui: &mut Ui,
    icons: &mut Icons,
    cell: Rect,
    id: Id,
    colour: Color32,
    name: &str,
    stop: bool,
) -> (Response, bool) {
    let sense = if stop { Sense::click() } else { Sense::hover() };
    let resp = ui.interact(cell, id, sense);
    let clicked = resp.clicked() || (!stop && released_in(ui, cell));
    labelled(&resp, Role::Button, name);
    let faded = resp.hovered() || (resp.has_focus() && focus_visible(ui.ctx()));
    let swatch_rect = Rect::from_center_size(cell.center(), vec2(20.0, 20.0));
    let (border, fill) = if faded {
        (over_bar(theme::SWATCH_BORDER, 0.4), over_bar(colour, 0.4))
    } else {
        (theme::SWATCH_BORDER, colour)
    };
    ui.painter().rect_filled(swatch_rect, 4, border);
    ui.painter().rect_filled(swatch_rect.shrink(1.0), 3, fill);
    if faded {
        let glyph = Rect::from_center_size(swatch_rect.center(), vec2(12.0, 12.0));
        icons.paint(ui, glyph, Icon::X, 12.0, 15.0, theme::TOOLBAR_BG);
        icons.paint(ui, glyph, Icon::X, 12.0, 3.0, theme::TOOLBAR_ICON);
    }
    if resp.has_focus() && focus_visible(ui.ctx()) {
        ui.painter().rect_stroke(
            swatch_rect,
            4,
            Stroke::new(2.0, theme::ACCENT),
            StrokeKind::Outside,
        );
    }
    (resp, clicked)
}

/// What a press on a palette swatch did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellPress {
    /// Nothing: a plain click beside the x does nothing.
    None,
    /// The press landed on the x: delete the colour.
    Delete,
    /// Ctrl+click (Cmd on macOS): exclude the colour or bring it back.
    ToggleExclude,
}

/// Half the x glyph with its casing: a press this close to the swatch's centre deletes.
const X_REACH: f32 = 7.5;

/// One colour of the Multicolour palette: the cell and swatch of a picked colour, an excluded
/// colour faded with a crossed circle, and the x on hover or focus (a crossed circle with Ctrl
/// held). A press deletes only on the x; Ctrl+click excludes.
#[allow(clippy::too_many_arguments)] // a cell is its place, name, colour and two states
pub fn palette_cell(
    ui: &mut Ui,
    icons: &mut Icons,
    cell: Rect,
    id: Id,
    colour: Color32,
    name: &str,
    stop: bool,
    excluded: bool,
    stale: bool,
) -> (Response, CellPress) {
    let sense = if stop { Sense::click() } else { Sense::hover() };
    let resp = ui.interact(cell, id, sense);
    let clicked = resp.clicked() || (!stop && released_in(ui, cell));
    labelled(&resp, Role::Button, name);
    let swatch_rect = Rect::from_center_size(cell.center(), vec2(20.0, 20.0));
    let (ctrl, pointer) = ui.input(|i| (i.modifiers.command, i.pointer.interact_pos()));
    let press = if !clicked {
        CellPress::None
    } else if ctrl {
        CellPress::ToggleExclude
    } else if pointer.is_some_and(|p| (p - swatch_rect.center()).abs().max_elem() <= X_REACH) {
        CellPress::Delete
    } else {
        CellPress::None
    };
    let active = resp.hovered() || (resp.has_focus() && focus_visible(ui.ctx()));
    // Fading, hover or exclusion at 40 % and an old result at 50 %, each flattened onto the bar.
    let alpha = if active || excluded { 0.4 } else { 1.0 } * if stale { 0.5 } else { 1.0 };
    let fade = |c: Color32| if alpha < 1.0 { over_bar(c, alpha) } else { c };
    ui.painter()
        .rect_filled(swatch_rect, 4, fade(theme::SWATCH_BORDER));
    ui.painter()
        .rect_filled(swatch_rect.shrink(1.0), 3, fade(colour));
    let glyph = if ctrl {
        (active || excluded).then_some(Icon::Ban)
    } else if active {
        Some(Icon::X)
    } else {
        excluded.then_some(Icon::Ban)
    };
    if let Some(icon) = glyph {
        let area = Rect::from_center_size(swatch_rect.center(), vec2(12.0, 12.0));
        let glyph = if stale {
            over_bar(theme::TOOLBAR_ICON, 0.5)
        } else {
            theme::TOOLBAR_ICON
        };
        icons.paint(ui, area, icon, 12.0, 15.0, theme::TOOLBAR_BG);
        icons.paint(ui, area, icon, 12.0, 3.0, glyph);
    }
    if resp.has_focus() && focus_visible(ui.ctx()) {
        ui.painter().rect_stroke(
            swatch_rect,
            4,
            Stroke::new(2.0, theme::ACCENT),
            StrokeKind::Outside,
        );
    }
    (resp, press)
}

/// The expander: a double chevron, with a changed-dot while closed and an Advanced value is off
/// its default.
pub fn expander(
    ui: &mut Ui,
    icons: &mut Icons,
    rect: Rect,
    id: Id,
    open: bool,
    dot: bool,
) -> Response {
    let name = if open {
        "Fewer trace options"
    } else {
        "More trace options"
    };
    let icon = if open {
        Icon::ChevronsUp
    } else {
        Icon::ChevronsDown
    };
    let resp = widgets::icon_button(ui, icons, rect, id, icon, name, false);
    if dot {
        let centre = pos2(rect.right() - 2.0 - 3.0, rect.top() + 2.0 + 3.0);
        ui.painter().circle_filled(centre, 4.0, theme::TOOLBAR_BG);
        ui.painter().circle_filled(centre, 3.0, theme::ACCENT);
    }
    resp
}
