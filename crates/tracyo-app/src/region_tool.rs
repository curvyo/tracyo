//! The pointer and keys of the regions on the canvas: drawing, selecting, moving, resizing,
//! nudging and deleting. The geometry is `tracyo_core::region_edit`; this turns screen input into
//! picture pixels and back.

use eframe::egui::{self, CursorIcon, Key, Modifiers, PointerButton, Pos2, Rect, Response, Ui};
use kurbo::Point;
use tracyo_core::lasso::Click;
use tracyo_core::region_edit::{self, Handle};
use tracyo_core::regions::{Form, PxPoint, PxRect, Region};

use crate::app::{App, Pipette};
use crate::mask::{Drag, MaskState, Tool};

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

/// The radius round a lasso corner that hits it or closes a lasso, in screen pixels.
pub(crate) const CORNER_HIT: f64 = 16.0;
/// Two presses this close in time (seconds) and place (screen pixels) are a double click.
const DOUBLE_CLICK_S: f64 = 0.3;
const DOUBLE_CLICK_PX: f32 = 6.0;

impl App {
    /// Handles the pointer on the canvas for the regions: starts, follows and ends the drags,
    /// sets the corners of a lasso, selects on a click and sets the cursor.
    #[allow(clippy::too_many_lines)] // one pass over the pointer's state, split by what is pressed
    pub(crate) fn region_input(&mut self, ui: &Ui, resp: &Response, canvas: Rect, space: bool) {
        let armed = self.pipette != Pipette::Off;
        let (pointer, press, mods, time, pressed) = ui.input(|i| {
            (
                i.pointer.latest_pos(),
                i.pointer.press_origin(),
                i.modifiers,
                i.time,
                i.pointer.button_pressed(PointerButton::Primary),
            )
        });
        let Some(doc) = &mut self.doc else { return };
        let zoom = doc.view.zoom;
        let to_px = |p: Pos2| -> Point {
            let v = doc.view.picture_point(p - canvas.min);
            Point::new(f64::from(v.x), f64::from(v.y))
        };
        let to_screen = |c: PxPoint| -> Point {
            let p = canvas.min + doc.view.offset + egui::vec2(c.x as f32, c.y as f32) * zoom;
            Point::new(f64::from(p.x), f64::from(p.y))
        };
        let screen_box = |r: PxRect| -> kurbo::Rect {
            let a = to_screen(PxPoint::new(r.x0, r.y0));
            let b = to_screen(PxPoint::new(r.x1, r.y1));
            kurbo::Rect::new(a.x, a.y, b.x, b.y)
        };
        let mask = &mut doc.mask;
        let mut changed = false;
        let mut finish = false;
        let rectangle_tool = mask.tool == Tool::Rectangle && !space && !armed;
        let lasso_tool = mask.tool == Tool::Lasso && !space && !armed;
        let editing = mask.tool == Tool::None && !space && !armed;
        let corners_of_selected = |mask: &MaskState| -> Option<(usize, Vec<Point>)> {
            let i = mask.selected?;
            match &mask.regions.get(i)?.form {
                Form::Lasso(points) => Some((i, points.iter().map(|c| to_screen(*c)).collect())),
                Form::Rect(_) => None,
            }
        };

        // A press with Lasso pressed sets a corner, closes on the first corner or, as the second
        // press of a double click, finishes the lasso without a corner.
        if lasso_tool
            && pressed
            && resp.hovered()
            && let Some(at) = pointer
        {
            let double = mask.last_press.is_some_and(|(t, p)| {
                time - t < DOUBLE_CLICK_S && p.distance(at) < DOUBLE_CLICK_PX
            });
            if double {
                finish = true;
            } else {
                match mask.draft.click(to_px(at), CORNER_HIT / f64::from(zoom)) {
                    Click::Close => finish = true,
                    Click::Added | Click::Ignored => mask.last_press = Some((time, at)),
                }
            }
        }
        // A press on a corner of the selected lasso selects that corner and holds it for the
        // drag or click that follows.
        if editing && pressed && resp.hovered() {
            mask.pressed_corner = None;
            if let (Some((index, corners)), Some(at)) =
                (corners_of_selected(mask), press.or(pointer))
            {
                let at = Point::new(f64::from(at.x), f64::from(at.y));
                mask.pressed_corner =
                    region_edit::corner_at(&corners, at, CORNER_HIT).map(|c| (index, c));
                if let Some((_, c)) = mask.pressed_corner {
                    mask.selected_corner = Some(c);
                }
            }
        }

        if resp.drag_started_by(PointerButton::Primary) && (rectangle_tool || editing) {
            let from = press.or(pointer).map(to_px);
            if let Some(from) = from {
                if rectangle_tool {
                    mask.drag = Some(Drag::Drawing { from, to: from });
                } else {
                    let origin = press.or(pointer).unwrap_or(canvas.min);
                    let origin = Point::new(f64::from(origin.x), f64::from(origin.y));
                    let handle = mask
                        .selected
                        .and_then(|i| match mask.regions.get(i) {
                            Some(Region {
                                form: Form::Rect(r),
                                ..
                            }) => Some((i, screen_box(*r))),
                            _ => None,
                        })
                        .and_then(|(i, rect)| region_edit::handle_at(rect, origin).map(|h| (i, h)));
                    let corner = mask.pressed_corner;
                    if let Some((index, handle)) = handle {
                        let Form::Rect(start) = mask.regions[index].form else {
                            return;
                        };
                        mask.drag = Some(Drag::Resizing {
                            index,
                            handle,
                            start,
                        });
                    } else if let Some((index, corner)) = corner {
                        mask.drag = Some(Drag::Corner {
                            index,
                            corner,
                            start: mask.regions[index].clone(),
                        });
                    } else if let Some(index) =
                        region_edit::region_at(&mask.regions, from, f64::from(SLOP / zoom))
                    {
                        mask.selected = Some(index);
                        mask.selected_corner = None;
                        let start = mask.regions[index].clone();
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
                    let moved = start.moved(dx as i32, dy as i32);
                    if let Some(r) = mask.regions.get_mut(*index) {
                        *r = moved;
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
                        *r = Region::rect(resized, r.subtract);
                    }
                }
                Some(Drag::Corner {
                    index,
                    corner,
                    start,
                }) => {
                    let moved = start.with_corner(*corner, region_edit::nearest_pixel(p));
                    if let Some(r) = mask.regions.get_mut(*index) {
                        *r = moved;
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
                        mask.regions.push(Region::rect(rect, mask.subtract));
                        changed = true;
                    }
                }
                Drag::Moving { index, start, .. } | Drag::Corner { index, start, .. } => {
                    changed = mask.regions.get(index).is_some_and(|r| *r != start);
                }
                Drag::Resizing { index, start, .. } => {
                    changed = mask
                        .regions
                        .get(index)
                        .is_some_and(|r| r.form != Form::Rect(start));
                }
            }
        }
        if resp.clicked_by(PointerButton::Primary) && editing && mask.pressed_corner.is_none() {
            mask.selected = pointer.and_then(|p| {
                region_edit::region_at(&mask.regions, to_px(p), f64::from(SLOP / zoom))
            });
            mask.selected_corner = None;
        }

        // The cursor.
        if let Some(Drag::Resizing { handle, .. }) = mask.drag {
            ui.ctx().set_cursor_icon(resize_cursor(handle));
        } else if resp.hovered() && !space {
            if rectangle_tool || lasso_tool {
                ui.ctx().set_cursor_icon(CursorIcon::Crosshair);
            } else if editing
                && mask.drag.is_none()
                && let (Some(i), Some(p)) = (mask.selected, pointer)
                && let Some(Region {
                    form: Form::Rect(r),
                    ..
                }) = mask.regions.get(i)
                && let Some(h) = region_edit::handle_at(
                    screen_box(*r),
                    Point::new(f64::from(p.x), f64::from(p.y)),
                )
            {
                ui.ctx().set_cursor_icon(resize_cursor(h));
            }
        }
        if ui.input(|i| i.pointer.primary_released()) {
            mask.pressed_corner = None;
        }
        let ctx = ui.ctx().clone();
        if changed {
            self.mask_changed(&ctx);
        }
        if finish {
            self.finish_lasso(&ctx);
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

    /// Backspace takes the last corner off a lasso in progress; else Delete, Backspace and the
    /// arrows act on the selected region while the canvas has the keys.
    pub(crate) fn region_keys(&mut self, ctx: &egui::Context) {
        let key = |m: Modifiers, k: Key| ctx.input_mut(|i| i.consume_key(m, k));
        if let Some(doc) = &mut self.doc
            && !doc.mask.draft.is_empty()
        {
            if key(Modifiers::NONE, Key::Backspace) {
                doc.mask.draft.backspace();
            }
            return;
        }
        if self.doc.as_ref().is_none_or(|d| d.mask.selected.is_none()) {
            return;
        }
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
