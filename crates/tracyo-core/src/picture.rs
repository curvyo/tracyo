//! Picture bytes to an upright RGBA8 picture, its read errors and the reduced copy.

use std::io::Cursor;

use image::{DynamicImage, ImageDecoder, ImageFormat, ImageReader, Limits};

/// The longest side of the reduced copy used while a value is dragged and for big pictures.
pub const REDUCED_SIDE: u32 = 1000;

/// The largest memory the decoder may allocate for one picture (a guard against huge headers).
const DECODE_ALLOC_LIMIT: u64 = 3 << 30;

/// Why a file could not be turned into a picture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ReadError {
    /// Not a PNG, JPEG or WebP file.
    #[error("This file type isn't supported. Use PNG, JPEG or WebP.")]
    Unsupported,
    /// A PNG, JPEG or WebP file that is truncated, damaged or has no pixels.
    #[error("The image is damaged and could not be read.")]
    Damaged,
    /// The decoder hit its memory limit or an allocation failed.
    #[error("Not enough memory to open this image.")]
    OutOfMemory,
}

/// An upright straight-alpha RGBA8 picture, row-major, top row first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Picture {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// `width * height * 4` bytes.
    pub rgba: Vec<u8>,
}

fn classify(error: &image::ImageError) -> ReadError {
    match error {
        image::ImageError::Limits(_) => ReadError::OutOfMemory,
        _ => ReadError::Damaged,
    }
}

/// 16-bit values rounded to 8 bit as `(v + 128) / 257`.
fn narrow(value: u16) -> u8 {
    // invariant: (65535 + 128) / 257 = 255.
    u8::try_from((u32::from(value) + 128) / 257).unwrap_or(u8::MAX)
}

impl Picture {
    /// Decodes PNG, JPEG or WebP `bytes` with the EXIF orientation applied.
    ///
    /// # Errors
    /// [`ReadError`] for another type, a damaged file, an empty picture or a memory limit.
    pub fn decode(bytes: &[u8]) -> Result<Self, ReadError> {
        let reader = ImageReader::new(Cursor::new(bytes))
            .with_guessed_format()
            .map_err(|_| ReadError::Damaged)?;
        if !matches!(
            reader.format(),
            Some(ImageFormat::Png | ImageFormat::Jpeg | ImageFormat::WebP)
        ) {
            return Err(ReadError::Unsupported);
        }
        let mut decoder = reader.into_decoder().map_err(|e| classify(&e))?;
        let mut limits = Limits::default();
        limits.max_alloc = Some(DECODE_ALLOC_LIMIT);
        decoder.set_limits(limits).map_err(|e| classify(&e))?;
        let orientation = decoder.orientation().map_err(|e| classify(&e))?;
        let mut image = DynamicImage::from_decoder(decoder).map_err(|e| classify(&e))?;
        image.apply_orientation(orientation);
        let wide = image.color().bits_per_pixel() / u16::from(image.color().channel_count()) > 8;
        let (width, height) = (image.width(), image.height());
        if width == 0 || height == 0 {
            return Err(ReadError::Damaged);
        }
        let rgba = if wide {
            image
                .into_rgba16()
                .into_raw()
                .into_iter()
                .map(narrow)
                .collect()
        } else {
            image.into_rgba8().into_raw()
        };
        Ok(Self {
            width,
            height,
            rgba,
        })
    }

    /// The number of pixels.
    #[must_use]
    pub fn pixels(&self) -> u64 {
        u64::from(self.width) * u64::from(self.height)
    }

    /// A copy whose longest side is at most `longest` pixels: every new pixel is the
    /// area-weighted mean of the pixels it covers, colour weighted by alpha so transparent pixels
    /// lend no colour. A small picture comes back unchanged.
    #[must_use]
    pub fn reduced(&self, longest: u32) -> Self {
        let long = self.width.max(self.height);
        if long <= longest || longest == 0 {
            return self.clone();
        }
        let scale = f64::from(longest) / f64::from(long);
        // invariant: the product is at most `longest`.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let side = |s: u32| ((f64::from(s) * scale).round() as u32).clamp(1, longest);
        let (width, height) = (side(self.width), side(self.height));
        let (columns, rows) = (weights(self.width, width), weights(self.height, height));
        let column_weights: Vec<f64> = columns
            .iter()
            .map(|c| c.iter().map(|&(_, w)| w).sum())
            .collect();
        let stride = self.width as usize * 4;
        let mut pixels = Vec::with_capacity(width as usize * height as usize * 4);
        let mut row_sums = vec![[0.0_f64; 4]; width as usize];
        for row in &rows {
            row_sums.fill([0.0; 4]);
            for &(sy, wy) in row {
                let source = &self.rgba[sy * stride..(sy + 1) * stride];
                for (sum, column) in row_sums.iter_mut().zip(&columns) {
                    for &(sx, wx) in column {
                        let p = &source[sx * 4..sx * 4 + 4];
                        let alpha = f64::from(p[3]) * wx * wy;
                        sum[0] += f64::from(p[0]) * alpha;
                        sum[1] += f64::from(p[1]) * alpha;
                        sum[2] += f64::from(p[2]) * alpha;
                        sum[3] += alpha;
                    }
                }
            }
            let row_weight: f64 = row.iter().map(|&(_, w)| w).sum();
            for (sum, &column_weight) in row_sums.iter().zip(&column_weights) {
                let alpha = sum[3] / (column_weight * row_weight);
                let colour = |c: f64| if sum[3] > 0.0 { c / sum[3] } else { 0.0 };
                // invariant: every value is a mean of bytes, so it rounds into 0..=255.
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                let byte = |v: f64| v.round().clamp(0.0, 255.0) as u8;
                pixels.extend([
                    byte(colour(sum[0])),
                    byte(colour(sum[1])),
                    byte(colour(sum[2])),
                    byte(alpha),
                ]);
            }
        }
        Self {
            width,
            height,
            rgba: pixels,
        }
    }
}

/// For each output index along an axis of `from` pixels reduced to `to`: the source indices it
/// covers with their overlap.
fn weights(from: u32, to: u32) -> Vec<Vec<(usize, f64)>> {
    let ratio = f64::from(from) / f64::from(to);
    (0..to)
        .map(|i| {
            let (start, end) = (f64::from(i) * ratio, f64::from(i + 1) * ratio);
            // invariant: `start` and `end` lie in `0..=from`, a `u32`.
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let (first, last) = (
                start.floor() as usize,
                (end.ceil() as usize).min(from as usize),
            );
            (first..last)
                .filter_map(|s| {
                    #[allow(clippy::cast_precision_loss)]
                    let (s0, s1) = (s as f64, s as f64 + 1.0);
                    let overlap = s1.min(end) - s0.max(start);
                    (overlap > 0.0).then_some((s, overlap))
                })
                .collect()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ExtendedColorType, ImageEncoder, codecs::png::PngEncoder};

    fn png(width: u32, height: u32, colour: ExtendedColorType, bytes: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        PngEncoder::new(&mut out)
            .write_image(bytes, width, height, colour)
            .unwrap();
        out
    }

    #[test]
    fn a_png_with_alpha_decodes_unchanged() {
        let bytes = png(2, 1, ExtendedColorType::Rgba8, &[1, 2, 3, 4, 5, 6, 7, 8]);
        let p = Picture::decode(&bytes).unwrap();
        assert_eq!((p.width, p.height), (2, 1));
        assert_eq!(p.rgba, [1, 2, 3, 4, 5, 6, 7, 8]);
    }

    #[test]
    fn grey_and_rgb_are_widened_with_full_alpha() {
        let grey = Picture::decode(&png(1, 1, ExtendedColorType::L8, &[9])).unwrap();
        assert_eq!(grey.rgba, [9, 9, 9, 255]);
        let rgb = Picture::decode(&png(1, 1, ExtendedColorType::Rgb8, &[1, 2, 3])).unwrap();
        assert_eq!(rgb.rgba, [1, 2, 3, 255]);
    }

    #[test]
    fn sixteen_bit_values_round_to_eight_bit() {
        let bytes = png(
            1,
            1,
            ExtendedColorType::Rgba16,
            &[0xFF, 0xFF, 0x80, 0x80, 0x00, 0x00, 0xFF, 0xFF],
        );
        let p = Picture::decode(&bytes).unwrap();
        assert_eq!(p.rgba, [255, 128, 0, 255]);
    }

    #[test]
    fn other_types_are_unsupported() {
        assert_eq!(
            Picture::decode(b"<svg xmlns='http://www.w3.org/2000/svg'/>"),
            Err(ReadError::Unsupported)
        );
        assert_eq!(Picture::decode(b""), Err(ReadError::Unsupported));
        assert_eq!(Picture::decode(b"just text"), Err(ReadError::Unsupported));
    }

    #[test]
    fn a_truncated_png_is_damaged() {
        let bytes = png(8, 8, ExtendedColorType::Rgb8, &[7; 8 * 8 * 3]);
        assert_eq!(
            Picture::decode(&bytes[..bytes.len() / 2]),
            Err(ReadError::Damaged)
        );
    }

    #[test]
    fn exif_orientation_is_applied_to_a_jpeg() {
        let bytes = std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/pictures/orientation-6.jpg"
        ))
        .unwrap();
        let p = Picture::decode(&bytes).unwrap();
        // Stored 4 x 2 with orientation 6 (rotate 90 clockwise): upright it is 2 x 4.
        assert_eq!((p.width, p.height), (2, 4));
    }

    #[test]
    fn the_reduced_copy_has_the_longest_side_given() {
        let big = Picture {
            width: 3000,
            height: 1500,
            rgba: vec![255; 3000 * 1500 * 4],
        };
        let small = big.reduced(REDUCED_SIDE);
        assert_eq!((small.width, small.height), (1000, 500));
        assert!(small.rgba.iter().all(|&b| b == 255));
        let tall = Picture {
            width: 10,
            height: 2500,
            rgba: vec![0; 10 * 2500 * 4],
        };
        let small = tall.reduced(REDUCED_SIDE);
        assert_eq!((small.width, small.height), (4, 1000));
    }

    #[test]
    fn the_reduced_copy_averages_by_area_and_keeps_small_pictures() {
        let p = Picture {
            width: 3,
            height: 1,
            rgba: vec![0, 0, 0, 255, 255, 255, 255, 255, 0, 0, 0, 255],
        };
        let copy = p.reduced(2);
        assert_eq!((copy.width, copy.height), (2, 1));
        assert_eq!(&copy.rgba[..4], &[85, 85, 85, 255]);
        assert_eq!(p.reduced(3), p);
    }

    #[test]
    fn transparent_pixels_lend_no_colour_to_the_reduced_copy() {
        let p = Picture {
            width: 2,
            height: 1,
            rgba: vec![255, 0, 0, 0, 0, 0, 0, 255],
        };
        assert_eq!(p.reduced(1).rgba, [0, 0, 0, 128]);
    }
}
