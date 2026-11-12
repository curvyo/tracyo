//! The regions on the canvas: the extra wash, the black outlines, the hover box, the selection
//! with its handles, the drag ghost and the feedback while a rectangle is drawn (ux-notes
//! section 5).

use eframe::egui::{
    Color32, Id, LayerId, Mesh, Order, Painter, Pos2, Rect, Shape, Stroke, StrokeKind, Ui, pos2,
    vec2,
};
use tracyo_core::region_edit::{self, Handle};
use tracyo_core::regions::{PxRect, Region};

use crate::mask::{Drag, MaskState};
use crate::region_tool::SLOP;
use crate::theme;

/// White over the 35 % veil that leaves the picture at 12 %: 1 - 12/35 of full opacity.
const WASH: Color32 = Color32::from_rgba_premultiplied(168, 168, 168, 168);
/// 0.25 mm in px at 96 dpi.
const LINE_PX: f32 = 0.944_9;
/// Dash and gap of subtracted outlines and the selection box, in screen px.
const DASH: f32 = 4.0;
const GAP: f32 = 3.0;
/// The side of a resize handle.
const HANDLE: f32 = 8.0;

/// The screen rectangle of `r` for a picture whose top-left corner is at `origin` at `zoom`.
pub(crate) fn screen(r: PxRect, origin: Pos2, zoom: f32) -> Rect {
    Rect::from_min_max(
        origin + vec2(r.x0 as f32, r.y0 as f32) * zoom,
        origin + vec2(r.x1 as f32, r.y1 as f32) * zoom,
    )
}

fn corners(rect: Rect) -> [Pos2; 5] {
    [
        rect.left_top(),
        rect.right_top(),
        rect.right_bottom(),
        rect.left_bottom(),
        rect.left_top(),
    ]
}

/// The line width of a region outline: 0.25 mm at the view's zoom, at least 1 px, in whole pixels.
fn line_width(ppp: f32, zoom: f32) -> f32 {
    ((LINE_PX * zoom * ppp).round().max(1.0)) / ppp
}

/// `v` moved to where a line of `width` is drawn sharp: the middle of a device pixel for an odd
/// number of them, a pixel edge for an even one.
fn sharp(v: f32, width: f32, ppp: f32) -> f32 {
    let odd = ((width * ppp).round() % 2.0 - 1.0).abs() < 0.5;
    let phase = if odd { 0.5 } else { 0.0 };
    ((v * ppp - phase).round() + phase) / ppp
}

fn snapped(rect: Rect, width: f32, ppp: f32) -> Rect {
    Rect::from_min_max(
        pos2(
            sharp(rect.left(), width, ppp),
            sharp(rect.top(), width, ppp),
        ),
        pos2(
            sharp(rect.right(), width, ppp),
            sharp(rect.bottom(), width, ppp),
        ),
    )
}

/// The extra wash of the active part and the black outline of every region of every part, below
/// the preview.
pub(crate) fn wash_and_outlines(
    painter: &Painter,
    mask: &MaskState,
    others: &[Region],
    origin: Pos2,
    zoom: f32,
    canvas: Rect,
) {
    let painter = painter.with_clip_rect(canvas);
    if !mask.wash.is_empty() {
        let mut mesh = Mesh::default();
        for r in &mask.wash {
            mesh.add_colored_rect(screen(*r, origin, zoom), WASH);
        }
        painter.add(Shape::mesh(mesh));
    }
    let ppp = painter.ctx().pixels_per_point();
    let width = line_width(ppp, zoom);
    let stroke = Stroke::new(width, Color32::BLACK);
    // The regions of the other parts are drawn as they are, not editable.
    for region in others.iter().chain(&mask.regions) {
        let rect = snapped(screen(region.rect, origin, zoom), width, ppp);
        if region.subtract {
            painter.extend(Shape::dashed_line(&corners(rect), stroke, DASH, GAP));
        } else {
            painter.rect_stroke(rect, 0, stroke, StrokeKind::Middle);
        }
    }
}

fn outline(painter: &Painter, rect: Rect, width: f32, colour: Color32, dashed: bool) {
    let stroke = Stroke::new(width, colour);
    if dashed {
        painter.extend(Shape::dashed_line(&corners(rect), stroke, DASH, GAP));
    } else {
        painter.rect_stroke(rect, 0, stroke, StrokeKind::Middle);
    }
}

/// A 1.5 px accent line over a casing of 1.5 px of white on each side, at `alpha` of full.
fn casing_line(painter: &Painter, rect: Rect, alpha: f32, dashed: bool) {
    let white = Color32::WHITE.gamma_multiply(alpha);
    let blue = theme::ACCENT.gamma_multiply(alpha);
    outline(painter, rect, 4.5, white, dashed);
    outline(painter, rect, 1.5, blue, dashed);
}

/// The hover box, the drag ghost, the selection with its handles and the drawing feedback, above
/// the preview.
pub(crate) fn overlays(
    ui: &Ui,
    mask: &MaskState,
    origin: Pos2,
    zoom: f32,
    canvas: Rect,
    pointer: Option<Pos2>,
    space: bool,
) {
    let painter = ui.painter().with_clip_rect(canvas);
    let ppp = ui.ctx().pixels_per_point();
    let editing = !mask.rectangle && !space && mask.drag.is_none();
    if let Some(start) = mask.drag_ghost() {
        casing_line(&painter, screen(start, origin, zoom), 0.6, false);
    }
    if editing && let Some(p) = pointer {
        let px = kurbo::Point::new(
            f64::from((p.x - origin.x) / zoom),
            f64::from((p.y - origin.y) / zoom),
        );
        if let Some(i) = region_edit::region_at(&mask.regions, px, f64::from(SLOP / zoom)) {
            let rect = snapped(screen(mask.regions[i].rect, origin, zoom), 1.0, ppp);
            outline(
                &painter,
                rect,
                3.0,
                Color32::WHITE.gamma_multiply(0.65),
                false,
            );
            outline(
                &painter,
                rect,
                1.0,
                theme::ACCENT.gamma_multiply(0.65),
                false,
            );
        }
    }
    if let Some(r) = mask.selected.and_then(|i| mask.regions.get(i)) {
        let rect = snapped(screen(r.rect, origin, zoom), 1.0, ppp);
        outline(&painter, rect, 1.0, theme::ACCENT, true);
        let held = match mask.drag {
            Some(Drag::Resizing { handle, .. }) => Some(handle),
            _ => None,
        };
        let short = rect.width().min(rect.height());
        for handle in Handle::ALL {
            if !handle.is_corner() && short < 24.0 {
                continue;
            }
            let at = handle.position(kurbo::Rect::new(
                f64::from(rect.left()),
                f64::from(rect.top()),
                f64::from(rect.right()),
                f64::from(rect.bottom()),
            ));
            let centre = pos2(at.x.round() as f32, at.y.round() as f32);
            let square = Rect::from_center_size(centre, vec2(HANDLE, HANDLE));
            let fill = if held == Some(handle) {
                theme::ACCENT
            } else {
                Color32::WHITE
            };
            painter.rect_filled(square, 2, fill);
            painter.rect_stroke(
                square,
                2,
                Stroke::new(1.0, theme::ACCENT),
                StrokeKind::Inside,
            );
        }
    }
    if let Some(Drag::Drawing { from, to }) = mask.drag {
        let mods = ui.input(|i| i.modifiers);
        if let Some(rect) = region_edit::drawn(from, to, mods.shift, mods.command) {
            casing_line(&painter, screen(rect, origin, zoom), 1.0, mask.subtract);
            if let Some(p) = pointer {
                size_chip(ui, canvas, p, rect);
                if mask.subtract {
                    remove_badge(ui, canvas, p);
                }
            }
        }
    }
}

/// "160 × 140 px" 12 px up and right of the pointer.
fn size_chip(ui: &Ui, canvas: Rect, pointer: Pos2, rect: PxRect) {
    let font = theme::text12();
    let label = format!("{} \u{d7} {} px", rect.width(), rect.height());
    let galley = ui
        .ctx()
        .fonts_mut(|f| f.layout_no_wrap(label, font, theme::TOOLBAR_ICON));
    let size = galley.size() + vec2(2.0 * (6.0 + 1.0), 2.0 * (2.0 + 1.0));
    let mut at = pos2(pointer.x + 12.0, pointer.y - 12.0 - size.y);
    at.x = at.x.min(canvas.right() - size.x).max(canvas.left());
    at.y = at.y.max(canvas.top());
    let chip = Rect::from_min_size(at, size);
    let painter = ui
        .ctx()
        .layer_painter(LayerId::new(Order::Tooltip, Id::unique("size-chip")));
    painter.rect_filled(chip, 8, theme::TOOLBAR_BG);
    painter.rect_stroke(
        chip,
        8,
        Stroke::new(1.0, theme::DIVIDER),
        StrokeKind::Inside,
    );
    painter.galley(chip.min + vec2(7.0, 3.0), galley, theme::TOOLBAR_ICON);
}

/// The minus badge of a subtracted rectangle, 16 px left of and 16 px below the pointer.
fn remove_badge(ui: &Ui, canvas: Rect, pointer: Pos2) {
    let mut centre = pointer + vec2(-16.0, 16.0);
    if centre.x - 9.0 < canvas.left() {
        centre.x = pointer.x + 16.0;
    }
    let painter = ui
        .ctx()
        .layer_painter(LayerId::new(Order::Tooltip, Id::unique("remove-badge")));
    painter.circle(centre, 8.0, theme::ACCENT, Stroke::new(1.0, Color32::WHITE));
    painter.line_segment(
        [centre - vec2(4.0, 0.0), centre + vec2(4.0, 0.0)],
        Stroke::new(1.5, Color32::WHITE),
    );
}
