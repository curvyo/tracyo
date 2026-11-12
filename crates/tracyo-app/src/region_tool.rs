//! The pointer and keys of the regions on the canvas: drawing, selecting, moving, resizing,
//! nudging and deleting. The geometry is `tracyo_core::region_edit`; this turns screen input into
//! picture pixels and back.

use eframe::egui::{self, CursorIcon, Key, Modifiers, PointerButton, Pos2, Rect, Response, Ui};
use kurbo::Point;
use tracyo_core::region_edit::{self, Handle};
use tracyo_core::regions::Region;

use crate::app::{App, Pipette};
use crate::mask::Drag;

/// How far from a rectangle's outline a click still selects it, in screen pixels.
pub(crate) const SLOP: f32 = 8.0;

/// The cursor for a drag by `handle`.
pub(crate) fn resize_cursor(handle: Handle) -> CursorIcon {
    match handle {
        Handle::N | Handle::S => CursorIcon::ResizeVertical,
        Handle::E | Handle::W => CursorIcon::ResizeHorizontal,
        Handle::NE | Handle::SW => CursorIcon::ResizeNeSw,
        Handle::NW | Handle::SE => CursorIcon::ResizeNwSe,
    }
}

impl App {
    /// Handles the pointer on the canvas for the regions: starts, follows and ends the drags,
    /// selects on a click and sets the cursor.
    pub(crate) fn region_input(&mut self, ui: &Ui, resp: &Response, canvas: Rect, space: bool) {
        let armed = self.pipette != Pipette::Off;
        let (pointer, press, mods) = ui.input(|i| {
            (
                i.pointer.latest_pos(),
                i.pointer.press_origin(),
                i.modifiers,
            )
        });
        let Some(doc) = &mut self.doc else { return };
        let zoom = doc.view.zoom;
        let to_px = |p: Pos2| -> Point {
            let v = doc.view.picture_point(p - canvas.min);
            Point::new(f64::from(v.x), f64::from(v.y))
        };
        let screen_rect = |r: tracyo_core::regions::PxRect| -> Rect {
            Rect::from_min_max(
                canvas.min + doc.view.offset + egui::vec2(r.x0 as f32, r.y0 as f32) * zoom,
                canvas.min + doc.view.offset + egui::vec2(r.x1 as f32, r.y1 as f32) * zoom,
            )
        };
        let mask = &mut doc.mask;
        let mut changed = false;
        let tool = mask.rectangle && !space && !armed;
        let editing = !mask.rectangle && !space && !armed;

        if resp.drag_started_by(PointerButton::Primary) && (tool || editing) {
            let from = press.or(pointer).map(to_px);
            if let Some(from) = from {
                if tool {
                    mask.drag = Some(Drag::Drawing { from, to: from });
                } else {
                    let origin = press.or(pointer).unwrap_or(canvas.min);
                    let handle = mask
                        .selected
                        .and_then(|i| mask.regions.get(i).map(|r| (i, screen_rect(r.rect))))
                        .and_then(|(i, rect)| {
                            region_edit::handle_at(
                                kurbo::Rect::new(
                                    f64::from(rect.left()),
                                    f64::from(rect.top()),
                                    f64::from(rect.right()),
                                    f64::from(rect.bottom()),
                                ),
                                Point::new(f64::from(origin.x), f64::from(origin.y)),
                            )
                            .map(|h| (i, h))
                        });
                    if let Some((index, handle)) = handle {
                        let start = mask.regions[index].rect;
                        mask.drag = Some(Drag::Resizing {
                            index,
                            handle,
                            start,
                        });
                    } else if let Some(index) =
                        region_edit::region_at(&mask.regions, from, f64::from(SLOP / zoom))
                    {
                        mask.selected = Some(index);
                        let start = mask.regions[index].rect;
                        mask.drag = Some(Drag::Moving { index, start, from });
                    }
                }
            }
        }
        if resp.dragged_by(PointerButton::Primary)
            && let Some(p) = pointer.map(to_px)
        {
            match &mut mask.drag {
                Some(Drag::Drawing { to, .. }) => *to = p,
                Some(Drag::Moving {
                    index, start, from, ..
                }) => {
                    let (dx, dy) = ((p.x - from.x).round(), (p.y - from.y).round());
                    // invariant: a drag stays within a few hundred thousand picture pixels.
                    #[allow(clippy::cast_possible_truncation)]
                    let moved = region_edit::moved(*start, dx as i32, dy as i32);
                    if let Some(r) = mask.regions.get_mut(*index) {
                        r.rect = moved;
                    }
                }
                Some(Drag::Resizing {
                    index,
                    handle,
                    start,
                }) => {
                    let resized =
                        region_edit::resized(*start, *handle, p, mods.shift, mods.command);
                    if let Some(r) = mask.regions.get_mut(*index) {
                        r.rect = resized;
                    }
                }
                None => {}
            }
        }
        if resp.drag_stopped_by(PointerButton::Primary)
            && let Some(drag) = mask.drag.take()
        {
            match drag {
                Drag::Drawing { from, to } => {
                    if let Some(rect) = region_edit::drawn(from, to, mods.shift, mods.command) {
                        mask.regions.push(Region {
                            rect,
                            subtract: mask.subtract,
                        });
                        changed = true;
                    }
                }
                Drag::Moving { index, start, .. } | Drag::Resizing { index, start, .. } => {
                    changed = mask.regions.get(index).is_some_and(|r| r.rect != start);
                }
            }
        }
        if resp.clicked_by(PointerButton::Primary) && editing {
            mask.selected = pointer.and_then(|p| {
                region_edit::region_at(&mask.regions, to_px(p), f64::from(SLOP / zoom))
            });
        }

        // The cursor.
        if let Some(Drag::Resizing { handle, .. }) = mask.drag {
            ui.ctx().set_cursor_icon(resize_cursor(handle));
        } else if resp.hovered() && !space {
            if tool {
                ui.ctx().set_cursor_icon(CursorIcon::Crosshair);
            } else if editing
                && mask.drag.is_none()
                && let (Some(i), Some(p)) = (mask.selected, pointer)
                && let Some(r) = mask.regions.get(i)
            {
                let rect = screen_rect(r.rect);
                let rect = kurbo::Rect::new(
                    f64::from(rect.left()),
                    f64::from(rect.top()),
                    f64::from(rect.right()),
                    f64::from(rect.bottom()),
                );
                if let Some(h) =
                    region_edit::handle_at(rect, Point::new(f64::from(p.x), f64::from(p.y)))
                {
                    ui.ctx().set_cursor_icon(resize_cursor(h));
                }
            }
        }
        if changed {
            let ctx = ui.ctx().clone();
            self.mask_changed(&ctx);
        }
    }

    /// Whether the pointer is on a region (within the click slop).
    pub(crate) fn pointer_over_region(&self, ui: &Ui, canvas: Rect) -> bool {
        let (Some(doc), Some(p)) = (&self.doc, ui.input(|i| i.pointer.latest_pos())) else {
            return false;
        };
        let v = doc.view.picture_point(p - canvas.min);
        let at = Point::new(f64::from(v.x), f64::from(v.y));
        region_edit::region_at(&doc.mask.regions, at, f64::from(SLOP / doc.view.zoom)).is_some()
    }

    /// Delete, Backspace and the arrows act on the selected region while the canvas has the keys.
    pub(crate) fn region_keys(&mut self, ctx: &egui::Context) {
        if self.doc.as_ref().is_none_or(|d| d.mask.selected.is_none()) {
            return;
        }
        let key = |m: Modifiers, k: Key| ctx.input_mut(|i| i.consume_key(m, k));
        if key(Modifiers::NONE, Key::Delete) || key(Modifiers::NONE, Key::Backspace) {
            self.delete_selected(ctx);
            return;
        }
        // Shift first: a key without modifiers also matches while Shift is held.
        for (step, modifiers) in [(10, Modifiers::SHIFT), (1, Modifiers::NONE)] {
            let (mut dx, mut dy) = (0, 0);
            if key(modifiers, Key::ArrowLeft) {
                dx = -step;
            }
            if key(modifiers, Key::ArrowRight) {
                dx = step;
            }
            if key(modifiers, Key::ArrowUp) {
                dy = -step;
            }
            if key(modifiers, Key::ArrowDown) {
                dy = step;
            }
            if dx != 0 || dy != 0 {
                self.nudge_selected(ctx, dx, dy);
                return;
            }
        }
    }
}
