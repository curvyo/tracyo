//! The pixel comparison of ux-notes section 12.2, ported from `reference/compare.py`.

#![allow(
    dead_code,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]

use image::{Rgb, RgbImage, RgbaImage};

const BAR: [u8; 3] = [0xDC, 0xDC, 0xE0];
/// Per-channel tolerance.
pub const TOLERANCE: i32 = 24;
/// At most this share of the pixels may differ.
pub const PASS_SHARE: f64 = 0.02;
/// No row or column may hold a run of this many differing pixels.
pub const PASS_RUN: usize = 12;

/// The outcome of one comparison.
pub struct Outcome {
    /// Compared width.
    pub width: u32,
    /// Compared height.
    pub height: u32,
    /// Differing pixels.
    pub count: usize,
    /// Share of differing pixels.
    pub share: f64,
    /// Longest straight run of differing pixels.
    pub run: usize,
    /// Differing pixels in red over a faded reference.
    pub diff: RgbImage,
}

impl Outcome {
    /// Whether the comparison passes section 12.2.
    pub fn passed(&self) -> bool {
        self.share <= PASS_SHARE && self.run < PASS_RUN
    }

    /// One line for the report.
    pub fn summary(&self) -> String {
        format!(
            "{} x {} px compared, {} differ ({:.2}%), longest run {} px: {}",
            self.width,
            self.height,
            self.count,
            self.share * 100.0,
            self.run,
            if self.passed() { "PASS" } else { "FAIL" }
        )
    }
}

/// Converts a rendered RGBA picture to RGB.
pub fn rgb(image: &RgbaImage) -> RgbImage {
    RgbImage::from_fn(image.width(), image.height(), |x, y| {
        let p = image.get_pixel(x, y).0;
        Rgb([p[0], p[1], p[2]])
    })
}

/// The (x, y) of the bar's top edge row and left edge column.
pub fn bar_origin(image: &RgbImage) -> Option<(u32, u32)> {
    let (w, h) = image.dimensions();
    let mut top = None;
    'rows: for y in 0..h {
        let mut run = 0;
        for x in 0..w {
            run = if image.get_pixel(x, y).0 == BAR {
                run + 1
            } else {
                0
            };
            if run >= 100 {
                top = Some(y);
                break 'rows;
            }
        }
    }
    let top = top?;
    let probe = top + 18;
    (0..w)
        .find(|&x| (probe..probe + 8).all(|y| y < h && image.get_pixel(x, y).0 == BAR))
        .map(|x| (x, top))
}

fn crop(image: &RgbImage, x: u32, y: u32, w: u32, h: u32) -> RgbImage {
    image::imageops::crop_imm(image, x, y, w, h).to_image()
}

/// Pixels of `a` with no match within one pixel in `b`.
fn mismatch(a: &RgbImage, b: &RgbImage) -> Vec<bool> {
    let (w, h) = a.dimensions();
    let mut out = vec![false; (w * h) as usize];
    for y in 0..h {
        for x in 0..w {
            let pa = a.get_pixel(x, y).0;
            let mut low = [255_i32; 3];
            let mut high = [0_i32; 3];
            for dy in -1_i32..=1 {
                for dx in -1_i32..=1 {
                    let nx = (i64::from(x) + i64::from(dx)).clamp(0, i64::from(w) - 1) as u32;
                    let ny = (i64::from(y) + i64::from(dy)).clamp(0, i64::from(h) - 1) as u32;
                    let pb = b.get_pixel(nx, ny).0;
                    for c in 0..3 {
                        low[c] = low[c].min(i32::from(pb[c]));
                        high[c] = high[c].max(i32::from(pb[c]));
                    }
                }
            }
            let mut peak = 0;
            for c in 0..3 {
                let under = (low[c] - i32::from(pa[c])).max(0);
                let over = (i32::from(pa[c]) - high[c]).max(0);
                peak = peak.max(under.max(over));
            }
            out[(y * w + x) as usize] = peak > TOLERANCE;
        }
    }
    out
}

fn longest_run(mask: &[bool], w: u32, h: u32) -> usize {
    let mut best = 0;
    for y in 0..h {
        let mut run = 0;
        for x in 0..w {
            run = if mask[(y * w + x) as usize] {
                run + 1
            } else {
                0
            };
            best = best.max(run);
        }
    }
    for x in 0..w {
        let mut run = 0;
        for y in 0..h {
            run = if mask[(y * w + x) as usize] {
                run + 1
            } else {
                0
            };
            best = best.max(run);
        }
    }
    best
}

/// Compares `got` with `reference`, both aligned on the bar's top and left edge.
pub fn compare(reference: &RgbImage, got: &RgbImage) -> Outcome {
    let (rx, ry) = bar_origin(reference).expect("no Trace bar in the reference");
    let (gx, gy) = bar_origin(got).expect("no Trace bar in the render");
    let (left, top) = (rx.min(gx), ry.min(gy));
    let width = (reference.width() - rx).min(got.width() - gx) + left;
    let height = (reference.height() - ry).min(got.height() - gy) + top;
    compare_crops(
        &crop(reference, rx - left, ry - top, width, height),
        &crop(got, gx - left, gy - top, width, height),
    )
}

/// Compares the Trace bar alone: both pictures are cut at the bar's top-left corner to `width` x
/// `height`. The margin around the bar holds Curvyo's own page (a darker ground, page lines,
/// "Page 1"), which Tracyo does not have, so it is not compared.
pub fn compare_region(reference: &RgbImage, got: &RgbImage, width: u32, height: u32) -> Outcome {
    compare_at(reference, got, (0, 0), width, height)
}

/// As [`compare_region`] for a region whose top-left corner lies `(dx, dy)` from the bar's.
pub fn compare_at(
    reference: &RgbImage,
    got: &RgbImage,
    (dx, dy): (u32, u32),
    width: u32,
    height: u32,
) -> Outcome {
    let (rx, ry) = bar_origin(reference).expect("no Trace bar in the reference");
    let (gx, gy) = bar_origin(got).expect("no Trace bar in the render");
    compare_crops(
        &crop(reference, rx + dx, ry + dy, width, height),
        &crop(got, gx + dx, gy + dy, width, height),
    )
}

/// Compares two pictures of the same size.
pub fn compare_crops(reference: &RgbImage, got: &RgbImage) -> Outcome {
    let (width, height) = reference.dimensions();
    let a = mismatch(reference, got);
    let b = mismatch(got, reference);
    let mask: Vec<bool> = a.iter().zip(&b).map(|(x, y)| *x || *y).collect();
    let count = mask.iter().filter(|m| **m).count();
    let share = count as f64 / f64::from(width * height);
    let run = longest_run(&mask, width, height);
    let diff = RgbImage::from_fn(width, height, |x, y| {
        if mask[(y * width + x) as usize] {
            Rgb([230, 0, 0])
        } else {
            let p = reference.get_pixel(x, y).0;
            Rgb([0, 1, 2].map(|c| (f64::from(p[c]) * 0.4 + 255.0 * 0.6) as u8))
        }
    });
    Outcome {
        width,
        height,
        count,
        share,
        run,
        diff,
    }
}

/// Where rendered pictures and diffs are written.
pub fn out_dir() -> std::path::PathBuf {
    let dir = std::env::var_os("TRACYO_PIXEL_OUT").map_or_else(
        || std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/pixel"),
        std::path::PathBuf::from,
    );
    std::fs::create_dir_all(&dir).expect("create output directory");
    dir
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A grey picture with a bar of 300 x 36 at (12, 12) holding a white field of `field_w` x 28
    /// with a dark border and text-like strokes.
    fn scene(field_w: u32) -> RgbImage {
        let mut image = RgbImage::from_pixel(400, 80, Rgb([0xE8, 0xE8, 0xEB]));
        for y in 12..48 {
            for x in 12..312 {
                image.put_pixel(x, y, Rgb(BAR));
            }
        }
        for y in 16..44 {
            for x in 20..20 + field_w {
                let edge = y == 16 || y == 43 || x == 20 || x == 19 + field_w;
                image.put_pixel(
                    x,
                    y,
                    if edge {
                        Rgb([0x89, 0x89, 0x8C])
                    } else {
                        Rgb([0xFF, 0xFF, 0xFF])
                    },
                );
            }
        }
        for x in (30..110).step_by(12) {
            for y in 24..36 {
                for dx in 0..4 {
                    image.put_pixel(x + dx, y, Rgb([0x3A, 0x3A, 0x3F]));
                }
            }
        }
        image
    }

    #[test]
    fn identical_pictures_pass_with_no_difference() {
        let a = scene(150);
        let o = compare(&a, &a);
        assert_eq!(o.count, 0);
        assert!(o.passed());
    }

    #[test]
    fn half_a_pixel_of_blur_passes() {
        let a = scene(150);
        let blurred = image::imageops::blur(&a, 0.5);
        let o = compare(&a, &blurred);
        assert!(o.passed(), "{}", o.summary());
    }

    #[test]
    fn a_block_of_30_by_20_fails() {
        let a = scene(150);
        let mut b = a.clone();
        for y in 20..40 {
            for x in 220..250 {
                b.put_pixel(x, y, Rgb([0x2F, 0x6F, 0xEE]));
            }
        }
        let o = compare(&a, &b);
        assert!(!o.passed(), "{}", o.summary());
    }

    #[test]
    fn a_field_two_pixels_narrower_fails() {
        let a = scene(150);
        let b = scene(148);
        let o = compare(&a, &b);
        assert!(!o.passed(), "{}", o.summary());
    }

    #[test]
    fn the_bar_is_found_by_its_ground() {
        assert_eq!(bar_origin(&scene(150)), Some((12, 12)));
    }
}
