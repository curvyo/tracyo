//! The pixel box a part is traced from: the box of its clip plus a margin, so a part that keeps a
//! small area does not pay for the whole picture.
//!
//! The margin is [`CROP_MARGIN_PX`], or Ignore below in pixels when that is more, so the specks
//! and the fit near the clip's edge see what they see in the whole picture. Settings whose
//! result is not local keep the whole picture.

use vectrace::PixelMap;

use crate::clip::Clip;
use crate::request::TraceRequest;

/// The margin round the clip's box, in pixels.
pub const CROP_MARGIN_PX: u32 = 16;

/// The grid snaps a clip edge by up to 0.0007 mm (0.003 px); a box edge within this of a whole pixel
/// is that pixel.
const GRID_SLACK_PX: f64 = 0.01;

/// A box of pixels: `x1` and `y1` are exclusive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PxBox {
    /// Left edge.
    pub x0: u32,
    /// Top edge.
    pub y0: u32,
    /// Right edge, exclusive.
    pub x1: u32,
    /// Bottom edge, exclusive.
    pub y1: u32,
}

impl PxBox {
    /// Width in pixels.
    #[must_use]
    pub fn width(&self) -> u32 {
        self.x1 - self.x0
    }

    /// Height in pixels.
    #[must_use]
    pub fn height(&self) -> u32 {
        self.y1 - self.y0
    }
}

/// Whether the result of `request` near a clip's edge depends only on the pixels round it: not
/// with Recognise text (the lines are in the picture's pixels), Drop border shapes (the cut's edge
/// is no border), Denoise and Upscale Auto (fits that are not local).
#[must_use]
pub fn may_crop(request: &TraceRequest) -> bool {
    request.lines.is_empty()
        && !request.drop_border
        && request.settings.denoise == 0
        && !request.settings.upscale_auto
}

/// The box of a `width` x `height` raster (whose pixel corners `map` takes to millimetres) that
/// `request` is traced from; `None` traces the whole raster: no clip, Outside, a setting that
/// is not local, or a box that covers the raster.
#[must_use]
pub fn crop_box(
    clip: Option<&Clip>,
    request: &TraceRequest,
    width: u32,
    height: u32,
    map: PixelMap,
) -> Option<PxBox> {
    let clip = clip.filter(|c| !c.outside() && may_crop(request))?;
    let inverse = map.inverse()?;
    let bounds = inverse.to_kurbo().transform_rect_bbox(clip.bounds_mm());
    let mm_per_px = map.determinant().abs().sqrt();
    let speck_px = request.settings.ignore_below / mm_per_px;
    if !bounds.is_finite() || !speck_px.is_finite() {
        return None;
    }
    let margin = f64::from(CROP_MARGIN_PX).max(speck_px.ceil());
    let low = |v: f64, limit: u32| -> u32 { clamp_px((v - margin + GRID_SLACK_PX).floor(), limit) };
    let high = |v: f64, limit: u32| -> u32 { clamp_px((v + margin - GRID_SLACK_PX).ceil(), limit) };
    let b = PxBox {
        x0: low(bounds.x0, width),
        y0: low(bounds.y0, height),
        x1: high(bounds.x1, width),
        y1: high(bounds.y1, height),
    };
    let covers = b.x0 == 0 && b.y0 == 0 && b.x1 == width && b.y1 == height;
    (b.x0 < b.x1 && b.y0 < b.y1 && !covers).then_some(b)
}

/// `v` as a pixel coordinate in `0..=limit`.
fn clamp_px(v: f64, limit: u32) -> u32 {
    // invariant: clamped into 0..=limit before the cast.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    {
        v.clamp(0.0, f64::from(limit)) as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::regions::{MaskMode, PxRect, Region};
    use crate::request::pixel_map;
    use crate::settings::Settings;

    fn request() -> TraceRequest {
        TraceRequest {
            threshold: 128,
            settings: Settings::default(),
            selection: vectrace::Selection::Brightness,
            invert: false,
            drop_border: false,
            colours: None,
            reduced: false,
            shapes: false,
            lines: Vec::new(),
            clip: None,
        }
    }

    fn clip(rect: PxRect, mode: MaskMode) -> Clip {
        let region = Region::rect(rect, false);
        Clip::of_regions(&[region], mode, pixel_map(1000, 1000))
            .unwrap()
            .unwrap()
    }

    #[test]
    fn the_box_is_the_clip_plus_the_margin() {
        let c = clip(PxRect::new(100, 200, 300, 400), MaskMode::Inside);
        let b = crop_box(Some(&c), &request(), 1000, 1000, pixel_map(1000, 1000)).unwrap();
        assert_eq!(
            b,
            PxBox {
                x0: 84,
                y0: 184,
                x1: 316,
                y1: 416
            }
        );
        assert_eq!((b.width(), b.height()), (232, 232));
    }

    #[test]
    fn the_box_stays_in_the_picture_and_a_box_that_covers_it_is_no_crop() {
        let c = clip(PxRect::new(-50, -50, 300, 400), MaskMode::Inside);
        let b = crop_box(Some(&c), &request(), 1000, 1000, pixel_map(1000, 1000)).unwrap();
        assert_eq!((b.x0, b.y0), (0, 0));
        let all = clip(PxRect::new(5, 5, 995, 995), MaskMode::Inside);
        assert_eq!(
            crop_box(Some(&all), &request(), 1000, 1000, pixel_map(1000, 1000)),
            None
        );
    }

    #[test]
    fn a_large_ignore_below_widens_the_margin() {
        let c = clip(PxRect::new(400, 400, 500, 500), MaskMode::Inside);
        let mut r = request();
        r.settings.ignore_below = 10.0;
        let b = crop_box(Some(&c), &r, 1000, 1000, pixel_map(1000, 1000)).unwrap();
        // 10 mm is 37.8 px at 96 dpi, rounded up to 38.
        assert_eq!((b.x0, b.x1), (362, 538));
    }

    #[test]
    fn some_settings_and_outside_keep_the_whole_picture() {
        let c = clip(PxRect::new(100, 200, 300, 400), MaskMode::Inside);
        let map = pixel_map(1000, 1000);
        let mut text = request();
        text.lines.push(vectrace::ocr::OcrLine {
            quad: [[0.0; 2]; 4],
            text: "a".into(),
            confidence: vec![1.0],
            positions: Vec::new(),
        });
        let mut border = request();
        border.drop_border = true;
        let mut denoise = request();
        denoise.settings.denoise = 1;
        let mut upscale = request();
        upscale.settings.upscale_auto = true;
        for r in [text, border, denoise, upscale] {
            assert_eq!(crop_box(Some(&c), &r, 1000, 1000, map), None);
        }
        let outside = clip(PxRect::new(100, 200, 300, 400), MaskMode::Outside);
        assert_eq!(crop_box(Some(&outside), &request(), 1000, 1000, map), None);
        assert_eq!(crop_box(None, &request(), 1000, 1000, map), None);
    }
}
