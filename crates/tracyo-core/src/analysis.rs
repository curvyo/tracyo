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
        let suggested_background = vectrace::suggest_background(raster);
        let fallback = Self {
            suggested_background,
            method: Method::Brightness,
            threshold: vectrace::auto_threshold(raster),
            tolerance: vectrace::DEFAULT_BACKGROUND_TOLERANCE,
        };
        let Ok(report) = vectrace::analyse(raster) else {
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
}
