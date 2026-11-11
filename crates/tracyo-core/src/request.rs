//! The trace request a picture and the settings make, and the map from its pixels to millimetres.

use vectrace::ocr::OcrLine;
use vectrace::{PixelMap, Selection};

use crate::palette::ColourSpec;
use crate::picture::Picture;
use crate::settings::Settings;

/// Pictures with more pixels than this are always previewed from the reduced copy (4 Mpx).
pub const REDUCED_OVER_PIXELS: u64 = 4_000_000;

/// One trace of the picture the worker holds.
#[derive(Debug, Clone, PartialEq)]
#[allow(clippy::struct_excessive_bools)] // independent switches of the bar
pub struct TraceRequest {
    /// The Threshold as a luma, 0 to 255.
    pub threshold: u8,
    /// The values of the bar (only the six preset values and the sliders are read).
    pub settings: Settings,
    /// The method with its background, picked colours and Colour tolerance.
    pub selection: Selection,
    /// Invert.
    pub invert: bool,
    /// Drop border shapes.
    pub drop_border: bool,
    /// Multicolour with its palette; `None` is Single colour.
    pub colours: Option<ColourSpec>,
    /// Trace the reduced copy instead of the full picture.
    pub reduced: bool,
    /// Recognise shapes.
    pub shapes: bool,
    /// The text lines read from the full picture (source pixels); empty traces no text. The
    /// worker scales them for the reduced copy.
    pub lines: Vec<OcrLine>,
}

/// Whether a trace of `picture` is made from the reduced copy: while a value is dragged, and
/// always for a picture over 4 Mpx unless the trace is for saving.
#[must_use]
pub fn use_reduced(picture: &Picture, dragging: bool, for_saving: bool) -> bool {
    dragging || (!for_saving && picture.pixels() > REDUCED_OVER_PIXELS)
}

/// Maps the pixels of a picture sent as `sent_width` wide, of which the full picture is
/// `full_width` wide, to millimetres at 96 dpi: a reduced result lands in the same millimetres.
#[must_use]
pub fn pixel_map(full_width: u32, sent_width: u32) -> PixelMap {
    let scale = f64::from(full_width) / f64::from(sent_width.max(1));
    let mm = 25.4 / 96.0 * scale;
    PixelMap::scale(mm, mm)
}

/// The size of a picture in millimetres at 96 dpi.
#[must_use]
pub fn size_mm(picture_px: u32) -> f64 {
    f64::from(picture_px) * 25.4 / 96.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn picture(width: u32, height: u32) -> Picture {
        Picture {
            width,
            height,
            rgba: Vec::new(),
        }
    }

    #[test]
    fn dragging_always_uses_the_reduced_copy() {
        assert!(use_reduced(&picture(10, 10), true, false));
    }

    #[test]
    fn a_picture_over_four_mpx_is_reduced_unless_it_is_for_saving() {
        let big = picture(2001, 2000);
        assert!(use_reduced(&big, false, false));
        assert!(!use_reduced(&big, false, true));
        assert!(!use_reduced(&picture(2000, 2000), false, false));
    }

    #[test]
    fn the_map_scales_a_reduced_copy_into_the_same_millimetres() {
        let full = pixel_map(2000, 2000).to_array();
        let reduced = pixel_map(2000, 1000).to_array();
        assert!((full[0] - 25.4 / 96.0).abs() < 1e-12);
        assert!((reduced[0] - 2.0 * full[0]).abs() < 1e-12);
        assert!((reduced[3] - 2.0 * full[3]).abs() < 1e-12);
    }

    #[test]
    fn a_picture_is_placed_at_96_dpi() {
        assert!((size_mm(96) - 25.4).abs() < 1e-12);
    }
}
