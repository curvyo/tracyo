//! The picture on the canvas: tiles of at most 2048 px, pan and zoom.

use eframe::egui::{
    self, Color32, ColorImage, Pos2, Rect, TextureHandle, TextureOptions, Vec2, pos2, vec2,
};
use tracyo_core::picture::Picture;

/// The side of a texture tile: at most 2048 (wgpu's GL fallback guarantees no more), and small
/// enough that one tile's upload fits a frame of the UI thread.
const TILE: u32 = 512;
/// The time one frame may spend uploading picture tiles.
const UPLOAD_BUDGET: std::time::Duration = std::time::Duration::from_millis(8);
/// The zoom range, 2 % to 8000 %.
const MIN_ZOOM: f32 = 0.02;
const MAX_ZOOM: f32 = 80.0;
/// From this zoom a picture pixel is 8 screen pixels or more: nearest filtering.
const NEAREST_FROM: f32 = 8.0;

struct Tile {
    x: u32,
    y: u32,
    w: u32,
    h: u32,
    smooth: TextureHandle,
    sharp: Option<TextureHandle>,
}

/// The textures of one picture, uploaded one tile per frame.
pub struct Tiles {
    tiles: Vec<Tile>,
    todo: std::collections::VecDeque<(u32, u32, u32, u32)>,
}

/// The pixels of the tile at (`x`, `y`) of size `w` x `h`.
fn tile_image(picture: &Picture, x: u32, y: u32, w: u32, h: u32) -> ColorImage {
    let stride = picture.width as usize * 4;
    let mut bytes = Vec::with_capacity(w as usize * h as usize * 4);
    for row in y..y + h {
        let start = row as usize * stride + x as usize * 4;
        bytes.extend_from_slice(&picture.rgba[start..start + w as usize * 4]);
    }
    ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &bytes)
}

impl Tiles {
    /// The tiles of a picture of `width` x `height`, none uploaded yet.
    #[must_use]
    pub fn new(width: u32, height: u32) -> Self {
        let mut todo = std::collections::VecDeque::new();
        let mut y = 0;
        while y < height {
            let h = TILE.min(height - y);
            let mut x = 0;
            while x < width {
                let w = TILE.min(width - x);
                todo.push_back((x, y, w, h));
                x += w;
            }
            y += h;
        }
        Self { tiles: Vec::new(), todo }
    }

    /// Uploads tiles for at most `UPLOAD_BUDGET` of the frame (at least one); true while tiles
    /// are left.
    fn upload_next(&mut self, ctx: &egui::Context, picture: &Picture) -> bool {
        let start = std::time::Instant::now();
        while let Some((x, y, w, h)) = self.todo.pop_front() {
            let image = tile_image(picture, x, y, w, h);
            let smooth = ctx.load_texture(format!("picture-{x}-{y}"), image, TextureOptions::LINEAR);
            self.tiles.push(Tile { x, y, w, h, smooth, sharp: None });
            if start.elapsed() >= UPLOAD_BUDGET {
                break;
            }
        }
        !self.todo.is_empty()
    }

    /// Paints the picture with its top-left corner at `origin` at `zoom`, clipped to `clip`.
    pub fn paint(&mut self, painter: &egui::Painter, picture: &Picture, origin: Pos2, zoom: f32, clip: Rect) {
        let painter = painter.with_clip_rect(clip);
        let ctx = painter.ctx().clone();
        if self.upload_next(&ctx, picture) {
            ctx.request_repaint();
        }
        for tile in &mut self.tiles {
            let rect = Rect::from_min_size(origin + vec2(tile.x as f32, tile.y as f32) * zoom, vec2(tile.w as f32, tile.h as f32) * zoom);
            if !rect.intersects(clip) {
                continue;
            }
            let id = if zoom >= NEAREST_FROM {
                let (x, y, w, h) = (tile.x, tile.y, tile.w, tile.h);
                tile.sharp.get_or_insert_with(|| ctx.load_texture("picture-sharp", tile_image(picture, x, y, w, h), TextureOptions::NEAREST)).id()
            } else {
                tile.smooth.id()
            };
            painter.image(id, rect, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
        }
    }
}

/// Where the picture sits in the canvas: its zoom and its top-left corner relative to the
/// canvas's top-left corner.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct View {
    /// Screen pixels per picture pixel.
    pub zoom: f32,
    /// The picture's top-left corner, relative to the canvas.
    pub offset: Vec2,
}

impl View {
    /// The picture of `size` centred in `area` (relative to the canvas), at 100 % or smaller; with
    /// `enlarge` it may grow to fit.
    #[must_use]
    pub fn fit(area: Rect, size: (u32, u32), enlarge: bool) -> Self {
        let (w, h) = (size.0 as f32, size.1 as f32);
        let fit = (area.width() / w).min(area.height() / h).max(MIN_ZOOM);
        let zoom = if enlarge { fit } else { fit.min(1.0) }.clamp(MIN_ZOOM, MAX_ZOOM);
        let offset = area.center().to_vec2() - vec2(w, h) * zoom / 2.0;
        Self { zoom, offset }
    }

    /// Zooms by `factor` keeping the point `anchor` (relative to the canvas) in place.
    pub fn zoom_at(&mut self, anchor: Vec2, factor: f32) {
        let new = (self.zoom * factor).clamp(MIN_ZOOM, MAX_ZOOM);
        let ratio = new / self.zoom;
        self.offset = anchor - (anchor - self.offset) * ratio;
        self.zoom = new;
    }

    /// The picture pixel under `point` (relative to the canvas).
    #[must_use]
    pub fn picture_point(&self, point: Vec2) -> Vec2 {
        (point - self.offset) / self.zoom
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area() -> Rect {
        Rect::from_min_size(pos2(12.0, 60.0), vec2(1000.0, 700.0))
    }

    #[test]
    fn a_small_picture_opens_at_100_percent_centred() {
        let v = View::fit(area(), (640, 420), false);
        assert_eq!(v.zoom, 1.0);
        assert_eq!(v.offset, vec2(12.0 + 180.0, 60.0 + 140.0));
    }

    #[test]
    fn a_big_picture_opens_fitted() {
        let v = View::fit(area(), (4000, 1000), false);
        assert!((v.zoom - 0.25).abs() < 1e-6);
    }

    #[test]
    fn fit_may_enlarge_with_ctrl_zero() {
        let v = View::fit(area(), (100, 100), true);
        assert!((v.zoom - 7.0).abs() < 1e-6);
    }

    #[test]
    fn zooming_keeps_the_pointer_over_the_same_picture_point() {
        let mut v = View {
            zoom: 1.0,
            offset: vec2(100.0, 50.0),
        };
        let anchor = vec2(300.0, 200.0);
        let before = v.picture_point(anchor);
        v.zoom_at(anchor, 2.5);
        let after = v.picture_point(anchor);
        assert!((before - after).length() < 1e-4);
        assert!((v.zoom - 2.5).abs() < 1e-6);
    }

    #[test]
    fn zoom_stays_between_2_and_8000_percent() {
        let mut v = View {
            zoom: 1.0,
            offset: Vec2::ZERO,
        };
        v.zoom_at(Vec2::ZERO, 1e6);
        assert!((v.zoom - 80.0).abs() < 1e-4);
        v.zoom_at(Vec2::ZERO, 1e-9);
        assert!((v.zoom - 0.02).abs() < 1e-6);
    }
}
