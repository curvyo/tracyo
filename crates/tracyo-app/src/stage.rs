//! The canvas, the status bar and the notice slot.

use eframe::egui::{self, Align2, Color32, Id, Key, PointerButton, Rect, Sense, Ui, pos2, vec2};
use tracyo_core::progress::Button;

use crate::app::{App, STATUS_HEIGHT, dashed_frame};
use crate::canvas::View;
use crate::theme;
use crate::widgets;

impl App {
    /// The picture, the veil and the preview, with pan and zoom.
    pub(crate) fn canvas(&mut self, ui: &mut Ui, canvas: Rect) {
        let resp = ui.interact(canvas, Id::unique("canvas"), Sense::click_and_drag());
        let bar_bottom = self.bar_bottom;
        let fit = std::mem::take(&mut self.fit_next_frame);
        let enlarge = std::mem::take(&mut self.fit_zoom_enlarge);
        self.pointer_picture = None;
        let Some(doc) = &mut self.doc else { return };
        if fit {
            let area = Rect::from_min_max(
                pos2(12.0, bar_bottom + 12.0),
                pos2(canvas.width() - 12.0, canvas.height() - 12.0),
            );
            doc.view = View::fit(area, (doc.picture.width, doc.picture.height), enlarge);
        }
        let space = ui.input(|i| i.key_down(Key::Space));
        if resp.dragged_by(PointerButton::Middle)
            || (space && resp.dragged_by(PointerButton::Primary))
        {
            doc.view.offset += resp.drag_delta();
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
        } else if space && resp.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
        }
        if resp.hovered() {
            let (scroll, zoom, hover) =
                ui.input(|i| (i.smooth_scroll_delta, i.zoom_delta(), i.pointer.hover_pos()));
            if (zoom - 1.0).abs() > f32::EPSILON
                && let Some(p) = hover
            {
                doc.view.zoom_at(p - canvas.min, zoom);
            } else {
                doc.view.offset += scroll;
            }
        }
        if resp.double_clicked() && !space {
            let ctx = ui.ctx().clone();
            self.enter_action(&ctx);
            return self.paint_canvas(ui, canvas, resp.hovered());
        }
        self.paint_canvas(ui, canvas, resp.hovered());
    }

    fn paint_canvas(&mut self, ui: &mut Ui, canvas: Rect, canvas_hovered: bool) {
        let state = self.progress.button(self.now());
        let alpha = if matches!(state, Button::Rest) {
            1.0
        } else {
            0.5
        };
        let hover = ui.input(|i| i.pointer.hover_pos());
        let hovered_files = ui.input(|i| !i.raw.hovered_files.is_empty());
        let Some(doc) = &mut self.doc else {
            if hovered_files {
                dashed_frame(ui, canvas);
            }
            return;
        };
        let origin = canvas.min + doc.view.offset;
        let zoom = doc.view.zoom;
        let painter = ui.painter().clone();
        doc.tiles
            .paint(&painter, &doc.picture, origin, zoom, canvas);
        let picture_rect = Rect::from_min_size(
            origin,
            vec2(doc.picture.width as f32, doc.picture.height as f32) * zoom,
        )
        .intersect(canvas);
        painter.with_clip_rect(canvas).rect_filled(
            picture_rect,
            0.0,
            Color32::from_white_alpha(166),
        );
        if let Some(result) = &mut self.result {
            result.preview.paint(&painter, origin, zoom, canvas, alpha);
        }
        if let Some(p) = hover.filter(|p| canvas.contains(*p) && canvas_hovered) {
            let pic = doc.view.picture_point(p - canvas.min);
            if pic.x >= 0.0
                && pic.y >= 0.0
                && pic.x < doc.picture.width as f32
                && pic.y < doc.picture.height as f32
            {
                // invariant: checked to be inside the picture.
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                {
                    self.pointer_picture = Some((pic.x as u32, pic.y as u32));
                }
            }
        }
        if hovered_files {
            dashed_frame(ui, canvas);
        }
    }

    /// The status bar: pointer position, zoom, picture size.
    pub(crate) fn status_bar(&self, ui: &Ui, screen: Rect) {
        let bar = Rect::from_min_max(
            pos2(screen.left(), screen.bottom() - STATUS_HEIGHT),
            screen.max,
        );
        ui.painter().rect_filled(bar, 0.0, theme::TOOLBAR_BG);
        let font = theme::text12();
        let y = bar.center().y;
        let zoom = self
            .doc
            .as_ref()
            .map_or(100.0, |d| (d.view.zoom * 100.0).round());
        widgets::text(
            ui,
            pos2(bar.center().x, y),
            Align2::CENTER_CENTER,
            &format!("{zoom}%"),
            font.clone(),
            theme::FOREGROUND,
        );
        if let Some((x, py)) = self.pointer_picture {
            widgets::text(
                ui,
                pos2(bar.left() + 8.0, y),
                Align2::LEFT_CENTER,
                &format!("x: {x}  y: {py} px"),
                font.clone(),
                theme::FOREGROUND,
            );
        }
        if let Some(doc) = &self.doc {
            widgets::text(
                ui,
                pos2(bar.right() - 8.0, y),
                Align2::RIGHT_CENTER,
                &format!("{} \u{d7} {} px", doc.picture.width, doc.picture.height),
                font,
                theme::FOREGROUND,
            );
        }
    }

    /// The notice slot: a short message centred above the status bar.
    pub(crate) fn notice_slot(&self, ctx: &egui::Context, canvas: Rect) {
        let Some(notice) = &self.notice else { return };
        let font = theme::text12();
        let colour = if notice.alert {
            theme::INVALID
        } else {
            theme::TOOLBAR_ICON
        };
        let galleys: Vec<_> = notice
            .lines
            .iter()
            .map(|l| ctx.fonts_mut(|f| f.layout_no_wrap(l.clone(), font.clone(), colour)))
            .collect();
        let width = galleys.iter().map(|g| g.size().x).fold(0.0_f32, f32::max) + 16.0;
        let height = galleys.iter().map(|g| g.size().y).sum::<f32>() + 16.0;
        let centre = 60.0 + (canvas.width() - 60.0) / 2.0;
        let pos = pos2(centre - width / 2.0, canvas.bottom() - 12.0 - height);
        egui::Area::new(Id::unique("notice"))
            .order(egui::Order::Foreground)
            .fixed_pos(pos)
            .interactable(false)
            .show(ctx, |ui| {
                let (rect, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
                ui.painter().add(theme::SHADOW.as_shape(rect, 8));
                ui.painter().rect_filled(rect, 8, theme::TOOLBAR_BG);
                if notice.alert {
                    ui.painter().rect_stroke(
                        rect,
                        8,
                        egui::Stroke::new(1.0, theme::INVALID),
                        egui::StrokeKind::Inside,
                    );
                }
                let mut y = rect.top() + 8.0;
                for galley in galleys {
                    let h = galley.size().y;
                    ui.painter()
                        .galley(pos2(rect.left() + 8.0, y), galley, colour);
                    y += h;
                }
                let _ = notice;
                let _ = vec2(0.0, 0.0);
            });
    }
}
