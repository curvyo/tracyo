//! The automatic analysis of a picture: the values it proposes and how they are applied.

use vectrace::{Rgb8, Selection};

use crate::method::{Method, PictureValues};
use crate::picture::Picture;
use crate::settings::Settings;

/// What the analysis found, reduced to the values Tracyo applies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Analysis {
    /// The background colour of the border (Auto); `None` is a transparent background.
    pub suggested_background: Option<Rgb8>,
    /// Brightness, or Remove background when Brightness would miss pale colours.
    pub method: Method,
    /// The proposed Threshold, a luma.
    pub threshold: u8,
    /// The proposed Colour tolerance of Remove background.
    pub tolerance: u8,
}

impl Analysis {
    /// The analysis of `picture`; when vectrace cannot analyse it the automatic Threshold and
    /// Brightness stand.
    #[must_use]
    pub fn of(picture: &Picture) -> Self {
        let raster = picture.raster();
        Self::from_report(
            vectrace::suggest_background(raster),
            vectrace::auto_threshold(raster),
            vectrace::analyse(raster).ok(),
        )
    }

    /// The analysis of the pixels of `picture` that `kept` marks (one flag per pixel): the
    /// automatic Threshold of those pixels, the background of those that touch the edge of the
    /// kept area or of the picture, and the proposal of the kept area's box with the other pixels
    /// transparent. The whole picture's analysis when no pixel is kept or `kept` does not fit.
    #[must_use]
    pub fn of_kept(picture: &Picture, kept: &[bool]) -> Self {
        let (w, h) = (picture.width as usize, picture.height as usize);
        if kept.len() != w * h || !kept.iter().any(|k| *k) || picture.rgba.len() != w * h * 4 {
            return Self::of(picture);
        }
        let pick = |keep: &dyn Fn(usize) -> bool| -> Vec<u8> {
            (0..w * h)
                .filter(|&i| keep(i))
                .flat_map(|i| &picture.rgba[i * 4..i * 4 + 4])
                .copied()
                .collect()
        };
        let inside = pick(&|i| kept[i]);
        let border = pick(&|i| kept[i] && on_border(kept, w, h, i));
        Self::from_report(
            vectrace::suggest_background(one_row(&border)),
            vectrace::auto_threshold(one_row(&inside)),
            box_analysis(picture, kept),
        )
    }

    fn from_report(
        suggested_background: Option<Rgb8>,
        threshold: u8,
        report: Option<vectrace::AnalysisReport>,
    ) -> Self {
        let fallback = Self {
            suggested_background,
            method: Method::Brightness,
            threshold,
            tolerance: vectrace::DEFAULT_BACKGROUND_TOLERANCE,
        };
        let Some(report) = report else {
            return fallback;
        };
        let proposal = report.proposal;
        match proposal.selection {
            Selection::RemoveBackground { tolerance, .. } => Self {
                method: Method::RemoveBackground,
                threshold: proposal.threshold,
                tolerance,
                ..fallback
            },
            _ => Self {
                threshold: proposal.threshold,
                ..fallback
            },
        }
    }

    /// Sets the analysed values: the Threshold, and the method with its background and Colour
    /// tolerance. With Pick colours the method stays (the picked colours are the maker's) and
    /// only the Threshold is set.
    pub fn apply(&self, settings: &mut Settings, values: &mut PictureValues) {
        values.threshold = self.threshold;
        if Method::from_index(settings.method) == Method::PickColours {
            return;
        }
        if settings.method != self.method.index() {
            values.palette.forget_kept();
        }
        settings.method = self.method.index();
        if self.method == Method::RemoveBackground {
            values.manual_background = None;
            settings.remove_tolerance = self.tolerance;
        }
    }
}

/// `rgba` as one row of pixels.
fn one_row(rgba: &[u8]) -> vectrace::Raster<'_> {
    vectrace::Raster {
        width: u32::try_from(rgba.len() / 4).unwrap_or(u32::MAX),
        height: 1,
        rgba,
    }
}

/// Whether kept pixel `at` of a `width` x `height` grid lies on the edge of the kept area: on the
/// picture's edge or next to a pixel that is not kept.
fn on_border(kept: &[bool], width: usize, height: usize, at: usize) -> bool {
    let (column, row) = (at % width, at / width);
    column == 0
        || row == 0
        || column + 1 == width
        || row + 1 == height
        || !kept[at - 1]
        || !kept[at + 1]
        || !kept[at - width]
        || !kept[at + width]
}

/// The analysis of the box of the `kept` pixels, the others transparent.
fn box_analysis(picture: &Picture, kept: &[bool]) -> Option<vectrace::AnalysisReport> {
    let w = picture.width as usize;
    let (mut x0, mut y0, mut x1, mut y1) = (usize::MAX, usize::MAX, 0, 0);
    for (i, _) in kept.iter().enumerate().filter(|(_, k)| **k) {
        let (x, y) = (i % w, i / w);
        (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x + 1), y1.max(y + 1));
    }
    let (cw, ch) = (x1.checked_sub(x0)?, y1.checked_sub(y0)?);
    let mut crop = Vec::with_capacity(cw * ch * 4);
    for y in y0..y1 {
        for x in x0..x1 {
            let i = y * w + x;
            if kept[i] {
                crop.extend_from_slice(&picture.rgba[i * 4..i * 4 + 4]);
            } else {
                crop.extend_from_slice(&[255, 255, 255, 0]);
            }
        }
    }
    vectrace::analyse(vectrace::Raster {
        width: u32::try_from(cw).ok()?,
        height: u32::try_from(ch).ok()?,
        rgba: &crop,
    })
    .ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn picture(width: u32, height: u32, paint: impl Fn(u32, u32) -> [u8; 4]) -> Picture {
        let mut rgba = Vec::new();
        for y in 0..height {
            for x in 0..width {
                rgba.extend(paint(x, y));
            }
        }
        Picture {
            width,
            height,
            rgba,
        }
    }

    /// Black strokes on white.
    fn black_on_white() -> Picture {
        picture(120, 120, |x, y| {
            if (20..100).contains(&x) && (50..60).contains(&y) {
                [0, 0, 0, 255]
            } else {
                [255, 255, 255, 255]
            }
        })
    }

    /// A pale yellow panel beside a black bar on white: Brightness would miss the panel.
    fn pale_on_white() -> Picture {
        picture(200, 200, |x, y| {
            if (20..60).contains(&x) && (20..180).contains(&y) {
                [0, 0, 0, 255]
            } else if (90..180).contains(&x) && (40..160).contains(&y) {
                [0xFF, 0xF2, 0xA8, 255]
            } else {
                [255, 255, 255, 255]
            }
        })
    }

    #[test]
    fn black_on_white_stays_brightness_with_a_white_background() {
        let a = Analysis::of(&black_on_white());
        assert_eq!(a.method, Method::Brightness);
        assert_eq!(a.suggested_background, Some(Rgb8::new(255, 255, 255)));
        assert!((1..255).contains(&a.threshold));
    }

    #[test]
    fn pale_colours_make_the_analysis_propose_remove_background() {
        let a = Analysis::of(&pale_on_white());
        assert_eq!(a.method, Method::RemoveBackground);
        assert!((10..=100).contains(&a.tolerance));
    }

    #[test]
    fn applying_sets_method_background_tolerance_and_threshold() {
        let a = Analysis {
            suggested_background: Some(Rgb8::new(255, 255, 255)),
            method: Method::RemoveBackground,
            threshold: 99,
            tolerance: 25,
        };
        let mut s = Settings::default();
        let mut v = PictureValues::new(10);
        v.manual_background = Some(Rgb8::new(1, 2, 3));
        a.apply(&mut s, &mut v);
        assert_eq!((s.method, s.remove_tolerance, v.threshold), (1, 25, 99));
        assert!(v.background_auto());
        assert_eq!(s.pick_tolerance, 19, "the other method's tolerance stays");
    }

    #[test]
    fn brightness_proposals_leave_the_tolerances_alone() {
        let a = Analysis {
            suggested_background: None,
            method: Method::Brightness,
            threshold: 130,
            tolerance: 40,
        };
        let mut s = Settings {
            method: 1,
            remove_tolerance: 55,
            ..Settings::default()
        };
        let mut v = PictureValues::new(10);
        a.apply(&mut s, &mut v);
        assert_eq!((s.method, s.remove_tolerance, v.threshold), (0, 55, 130));
    }

    #[test]
    fn with_pick_colours_only_the_threshold_is_set() {
        let a = Analysis {
            suggested_background: None,
            method: Method::RemoveBackground,
            threshold: 77,
            tolerance: 33,
        };
        let mut s = Settings {
            method: 2,
            ..Settings::default()
        };
        let mut v = PictureValues::new(10);
        v.swatches = vec![Rgb8::new(1, 2, 3)];
        a.apply(&mut s, &mut v);
        assert_eq!((s.method, s.remove_tolerance, v.threshold), (2, 40, 77));
        assert_eq!(v.swatches.len(), 1);
    }

    #[test]
    fn a_new_method_forgets_a_deleted_palette() {
        let mut settings = Settings::default();
        let mut values = PictureValues::new(1);
        values.palette.kept = Some(vec![Rgb8::new(1, 2, 3)]);
        let remove = Analysis {
            suggested_background: None,
            method: Method::RemoveBackground,
            threshold: 100,
            tolerance: 30,
        };
        remove.apply(&mut settings, &mut values);
        assert_eq!(values.palette.kept, None);
        values.palette.kept = Some(vec![Rgb8::new(1, 2, 3)]);
        remove.apply(&mut settings, &mut values);
        assert!(values.palette.kept.is_some(), "the method did not change");
    }

    /// Left half white with a black bar, right half #D8D8D8 with a black bar.
    fn two_halves() -> Picture {
        picture(200, 100, |x, y| {
            let bar = (30..70).contains(&y) && (x % 100 > 20 && x % 100 < 80);
            match (bar, x >= 100) {
                (true, _) => [0, 0, 0, 255],
                (false, false) => [255, 255, 255, 255],
                (false, true) => [0xD8, 0xD8, 0xD8, 255],
            }
        })
    }

    fn kept_right(p: &Picture) -> Vec<bool> {
        (0..p.rgba.len() / 4)
            .map(|i| i % p.width as usize >= 100)
            .collect()
    }

    #[test]
    fn the_analysis_of_a_half_reads_only_that_half() {
        let p = two_halves();
        let right = Analysis::of_kept(&p, &kept_right(&p));
        assert_eq!(
            right.suggested_background,
            Some(Rgb8::new(0xD8, 0xD8, 0xD8))
        );
        let left: Vec<bool> = kept_right(&p).iter().map(|k| !k).collect();
        let left = Analysis::of_kept(&p, &left);
        assert_eq!(left.suggested_background, Some(Rgb8::new(255, 255, 255)));
        assert_ne!(
            left.threshold, right.threshold,
            "each half has its own Threshold"
        );
    }

    #[test]
    fn nothing_kept_or_a_mask_of_the_wrong_size_falls_back_to_the_whole_picture() {
        let p = two_halves();
        assert_eq!(
            Analysis::of_kept(&p, &vec![false; 200 * 100]),
            Analysis::of(&p)
        );
        assert_eq!(Analysis::of_kept(&p, &[true; 3]), Analysis::of(&p));
        assert_eq!(
            Analysis::of_kept(&p, &vec![true; 200 * 100]).threshold,
            Analysis::of(&p).threshold
        );
    }
}
