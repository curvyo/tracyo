//! The regions on the canvas: the extra wash, the black outlines, the hover box, the selection
//! with its handles or corner squares, the drag ghost and the feedback while a rectangle or a
//! lasso is drawn (ux-notes of 0002, section 5).

use eframe::egui::{
    Color32, Id, LayerId, Mesh, Order, Painter, Pos2, Rect, Shape, Stroke, StrokeKind, Ui, pos2,
    vec2,
};
use tracyo_core::region_edit::{self, Handle};
use tracyo_core::regions::{Form, PxPoint, PxRect, Region};

use crate::mask::{Drag, MaskState, Tool};
use crate::region_tool::{CORNER_HIT, SLOP};
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
/// The side of a lasso corner square.
const CORNER: f32 = 14.0;
/// The diameter of the ring round a hovered or last corner.
const RING: f32 = 18.0;
/// Dash and gap of a lasso's closing side at the close target.
const CLOSE_DASH: f32 = 6.0;
const CLOSE_GAP: f32 = 4.0;
/// The ground colour a corner of a lasso in progress is filled with.
const GROUND: Color32 = theme::CANVAS_BG;
/// The accent at 20 %: the ring round a corner.
const RING_COLOUR: Color32 = Color32::from_rgba_premultiplied(0x09, 0x16, 0x30, 0x33);

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

/// The screen position of a picture-pixel point.
fn at(p: PxPoint, origin: Pos2, zoom: f32) -> Pos2 {
    origin + vec2(p.x as f32, p.y as f32) * zoom
}

/// The screen corners of a lasso.
fn lasso_points(corners: &[PxPoint], origin: Pos2, zoom: f32) -> Vec<Pos2> {
    corners.iter().map(|c| at(*c, origin, zoom)).collect()
}

/// `points` closed again at the first point, for one dashed call.
fn closed(points: &[Pos2]) -> Vec<Pos2> {
    let mut out = points.to_vec();
    out.extend(points.first().copied());
    out
}

/// A lasso's outline as a closed line, solid or dashed (one call, so the dashes run round the
/// corners).
fn polygon(painter: &Painter, points: &[Pos2], stroke: Stroke, dashed: bool) {
    if dashed {
        painter.extend(Shape::dashed_line(&closed(points), stroke, DASH, GAP));
    } else {
        painter.add(Shape::closed_line(points.to_vec(), stroke));
    }
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
        for [a, b, c] in &mask.wash {
            let p = |v: &kurbo::Point| origin + vec2(v.x as f32, v.y as f32) * zoom;
            let base = u32::try_from(mesh.vertices.len()).unwrap_or(0);
            for v in [a, b, c] {
                mesh.colored_vertex(p(v), WASH);
            }
            mesh.add_triangle(base, base + 1, base + 2);
        }
        painter.add(Shape::mesh(mesh));
    }
    let ppp = painter.ctx().pixels_per_point();
    let width = line_width(ppp, zoom);
    let stroke = Stroke::new(width, Color32::BLACK);
    // The regions of the other parts are drawn as they are, not editable.
    for region in others.iter().chain(&mask.regions) {
        match &region.form {
            Form::Rect(r) => {
                let rect = snapped(screen(*r, origin, zoom), width, ppp);
                if region.subtract {
                    painter.extend(Shape::dashed_line(&corners(rect), stroke, DASH, GAP));
                } else {
                    painter.rect_stroke(rect, 0, stroke, StrokeKind::Middle);
                }
            }
            Form::Lasso(points) => polygon(
                &painter,
                &lasso_points(points, origin, zoom),
                stroke,
                region.subtract,
            ),
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
fn casing_line(painter: &Painter, region: &Region, origin: Pos2, zoom: f32, alpha: f32) {
    let white = Color32::WHITE.gamma_multiply(alpha);
    let blue = theme::ACCENT.gamma_multiply(alpha);
    match &region.form {
        Form::Rect(r) => {
            let rect = screen(*r, origin, zoom);
            outline(painter, rect, 4.5, white, false);
            outline(painter, rect, 1.5, blue, false);
        }
        Form::Lasso(points) => {
            let points = lasso_points(points, origin, zoom);
            polygon(painter, &points, Stroke::new(4.5, white), false);
            polygon(painter, &points, Stroke::new(1.5, blue), false);
        }
    }
}

/// The live outline of a rectangle being drawn: 1.5 px accent over a white casing.
fn live_rect(painter: &Painter, rect: Rect, dashed: bool) {
    outline(painter, rect, 4.5, Color32::WHITE, dashed);
    outline(painter, rect, 1.5, theme::ACCENT, dashed);
}

/// The ring of 18 px round a corner.
fn ring(painter: &Painter, centre: Pos2) {
    // egui strokes a circle half a pixel outside the radius it is given: this puts the ring at
    // 8 to 9 px from the centre, an outer diameter of 18.
    painter.circle_stroke(centre, RING / 2.0 - 1.0, Stroke::new(1.0, RING_COLOUR));
}

/// The hover box, the drag ghost, the selection with its handles or corner squares and the
/// drawing feedback, above the preview.
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
    let editing = mask.tool == Tool::None && !space && mask.drag.is_none();
    if let Some(start) = mask.drag_ghost() {
        casing_line(&painter, &start, origin, zoom, 0.6);
    }
    if editing && let Some(p) = pointer {
        let px = kurbo::Point::new(
            f64::from((p.x - origin.x) / zoom),
            f64::from((p.y - origin.y) / zoom),
        );
        if let Some(i) = region_edit::region_at(&mask.regions, px, f64::from(SLOP / zoom)) {
            let rect = snapped(screen(mask.regions[i].bounds(), origin, zoom), 1.0, ppp);
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
    if let Some(region) = mask.selected.and_then(|i| mask.regions.get(i)) {
        let rect = snapped(screen(region.bounds(), origin, zoom), 1.0, ppp);
        outline(&painter, rect, 1.0, theme::ACCENT, true);
        match &region.form {
            Form::Rect(_) => handles(&painter, mask, rect),
            Form::Lasso(points) => corner_squares(&painter, mask, points, origin, zoom, pointer),
        }
    }
    if let Some(Drag::Drawing { from, to }) = mask.drag {
        let mods = ui.input(|i| i.modifiers);
        if let Some(rect) = region_edit::drawn(from, to, mods.shift, mods.command) {
            live_rect(&painter, screen(rect, origin, zoom), mask.subtract);
            if let Some(p) = pointer {
                size_chip(ui, canvas, p, rect);
                if mask.subtract {
                    remove_badge(ui, canvas, p);
                }
            }
        }
    }
    if !mask.draft.is_empty() {
        lasso_in_progress(ui, &painter, mask, origin, zoom, canvas, pointer);
    }
}

/// The eight resize handles of a selected rectangle whose box is `rect` on screen.
fn handles(painter: &Painter, mask: &MaskState, rect: Rect) {
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

/// The corner squares of a selected lasso: white with a 1 px outline, the corner pressed last
/// accent edge to edge, the one under the pointer with its ring.
fn corner_squares(
    painter: &Painter,
    mask: &MaskState,
    points: &[PxPoint],
    origin: Pos2,
    zoom: f32,
    pointer: Option<Pos2>,
) {
    let screen_points = lasso_points(points, origin, zoom);
    let hovered = pointer.filter(|_| mask.drag.is_none()).and_then(|p| {
        let all: Vec<kurbo::Point> = screen_points
            .iter()
            .map(|q| kurbo::Point::new(f64::from(q.x), f64::from(q.y)))
            .collect();
        region_edit::corner_at(
            &all,
            kurbo::Point::new(f64::from(p.x), f64::from(p.y)),
            CORNER_HIT,
        )
    });
    for (i, p) in screen_points.iter().enumerate() {
        let centre = pos2(p.x.round(), p.y.round());
        if hovered == Some(i) {
            ring(painter, centre);
        }
        let square = Rect::from_center_size(centre, vec2(CORNER, CORNER));
        let selected = mask.selected_corner == Some(i);
        painter.rect_filled(
            square,
            0,
            if selected {
                theme::ACCENT
            } else {
                Color32::WHITE
            },
        );
        painter.rect_stroke(
            square,
            0,
            Stroke::new(
                1.0,
                if selected {
                    theme::ACCENT
                } else {
                    theme::TOOLBAR_ICON
                },
            ),
            StrokeKind::Inside,
        );
    }
}

/// The lasso being drawn: the sides so far, a hollow square on every corner, the ring on the last,
/// the rubber band to the pointer or, over the close target, the dashed closing side.
fn lasso_in_progress(
    ui: &Ui,
    painter: &Painter,
    mask: &MaskState,
    origin: Pos2,
    zoom: f32,
    canvas: Rect,
    pointer: Option<Pos2>,
) {
    let ppp = ui.ctx().pixels_per_point();
    let points = lasso_points(mask.draft.corners(), origin, zoom);
    let stroke = Stroke::new(line_width(ppp, zoom), theme::ACCENT);
    let thin = Stroke::new(1.0, theme::ACCENT);
    let (Some(&first), Some(&last)) = (points.first(), points.last()) else {
        return;
    };
    if points.len() > 1 {
        if mask.subtract {
            painter.extend(Shape::dashed_line(&points, stroke, DASH, GAP));
        } else {
            painter.add(Shape::line(points.clone(), stroke));
        }
    }
    let on_target =
        pointer.is_some_and(|p| points.len() >= 3 && p.distance(first) <= CORNER_HIT as f32);
    if let Some(p) = pointer {
        if on_target {
            painter.extend(Shape::dashed_line(
                &[last, first],
                thin,
                CLOSE_DASH,
                CLOSE_GAP,
            ));
        } else if mask.subtract {
            painter.extend(Shape::dashed_line(&[last, p], thin, DASH, GAP));
        } else {
            painter.line_segment([last, p], thin);
        }
    }
    ring(painter, pos2(last.x.round(), last.y.round()));
    if on_target {
        ring(painter, pos2(first.x.round(), first.y.round()));
    }
    for p in &points {
        let square = Rect::from_center_size(pos2(p.x.round(), p.y.round()), vec2(CORNER, CORNER));
        painter.rect_filled(square, 0, GROUND);
        painter.rect_stroke(square, 0, thin, StrokeKind::Inside);
    }
    if mask.subtract
        && let Some(p) = pointer
    {
        remove_badge(ui, canvas, p);
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
