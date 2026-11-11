//! The remembered values, their ranges and steps, the presets and the settings file text.

use serde::{Deserialize, Serialize};
use vectrace::{Denoise, PRESETS, PixelLength, Preset, Upscale};

/// One value field of the Trace bar: its label, unit, range and steps (ux-notes section 4).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Field {
    /// The label inside the field and its accessible name.
    pub label: &'static str,
    /// The unit shown after the value ("" for none).
    pub unit: &'static str,
    /// The smallest value.
    pub min: f64,
    /// The largest value.
    pub max: f64,
    /// One step (Up, Down, a click on the edge).
    pub step: f64,
    /// The big step (Shift).
    pub big_step: f64,
    /// The fine step (Ctrl).
    pub fine_step: f64,
    /// Decimals shown.
    pub decimals: usize,
}

/// The Threshold in whole percent.
pub const THRESHOLD: Field = Field {
    label: "Threshold",
    unit: "%",
    min: 0.0,
    max: 100.0,
    step: 1.0,
    big_step: 10.0,
    fine_step: 1.0,
    decimals: 0,
};
/// Smooth, 0 to 100.
pub const SMOOTH: Field = Field {
    label: "Smooth",
    unit: "",
    min: 0.0,
    max: 100.0,
    step: 1.0,
    big_step: 10.0,
    fine_step: 1.0,
    decimals: 0,
};
/// Tolerance in source pixels.
pub const TOLERANCE: Field = Field {
    label: "Tolerance",
    unit: "px",
    min: 0.1,
    max: 3.0,
    step: 0.05,
    big_step: 0.5,
    fine_step: 0.01,
    decimals: 2,
};
/// Ignore below, in millimetres.
pub const IGNORE_BELOW: Field = Field {
    label: "Ignore below",
    unit: "mm",
    min: 0.0,
    max: 10.0,
    step: 0.1,
    big_step: 1.0,
    fine_step: 0.01,
    decimals: 2,
};

/// Corners, 0 to 100 (Advanced).
pub const CORNERS: Field = Field {
    label: "Corners",
    unit: "",
    min: 0.0,
    max: 100.0,
    step: 1.0,
    big_step: 10.0,
    fine_step: 1.0,
    decimals: 0,
};
/// The Colour tolerance of Remove background and Pick colours, 0 to 100.
pub const COLOUR_TOLERANCE: Field = Field {
    label: "Colour tolerance",
    unit: "",
    min: 0.0,
    max: 100.0,
    step: 1.0,
    big_step: 10.0,
    fine_step: 1.0,
    decimals: 0,
};

/// How far a key press moves a value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stride {
    /// Up and Down.
    Step,
    /// Shift.
    Big,
    /// Ctrl.
    Fine,
}

impl Field {
    /// `value` clamped to the range and rounded to the field's finest step.
    #[must_use]
    pub fn clamp(&self, value: f64) -> f64 {
        let value = if value.is_finite() { value } else { self.min };
        let rounded = (value / self.fine_step).round() * self.fine_step;
        // Remove binary noise such as 0.30000000000000004.
        let cleaned = format!("{rounded:.6}").parse().unwrap_or(rounded);
        cleaned.clamp(self.min, self.max)
    }

    /// `value` moved by `stride` in `direction` (+1 or -1), clamped.
    #[must_use]
    pub fn nudge(&self, value: f64, direction: i32, stride: Stride) -> f64 {
        let amount = match stride {
            Stride::Step => self.step,
            Stride::Big => self.big_step,
            Stride::Fine => self.fine_step,
        };
        self.clamp(value + amount * f64::from(direction))
    }

    /// The value text without its unit ("0.50").
    #[must_use]
    pub fn format(&self, value: f64) -> String {
        format!("{value:.*}", self.decimals)
    }

    /// The value typed as text, clamped; `None` when it is not a number.
    #[must_use]
    pub fn parse(&self, text: &str) -> Option<f64> {
        let text = text
            .trim()
            .trim_end_matches(self.unit)
            .trim()
            .replace(',', ".");
        text.parse::<f64>()
            .ok()
            .filter(|v| v.is_finite())
            .map(|v| self.clamp(v))
    }

    /// Where `value` lies in the range, 0 to 1.
    #[must_use]
    pub fn fraction(&self, value: f64) -> f64 {
        ((value - self.min) / (self.max - self.min)).clamp(0.0, 1.0)
    }

    /// The value at `fraction` of the range (a click or a drag), clamped.
    #[must_use]
    pub fn at_fraction(&self, fraction: f64) -> f64 {
        self.clamp(self.min + (self.max - self.min) * fraction.clamp(0.0, 1.0))
    }
}

/// The Threshold luma (0 to 255) of a whole percent, rounded half up.
#[must_use]
pub fn luma_of_percent(percent: f64) -> u8 {
    let luma = (percent * 255.0 / 100.0 + 0.5).floor().clamp(0.0, 255.0);
    // invariant: clamped into 0..=255 above.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    {
        luma as u8
    }
}

/// The whole percent of a Threshold luma, rounded half up.
#[must_use]
pub fn percent_of_luma(luma: u8) -> f64 {
    (f64::from(luma) * 100.0 / 255.0 + 0.5).floor()
}

/// The values the settings file remembers (spec criterion 18; Output, Colours and the two
/// Recognise switches arrive with the slices that have them).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Smooth, 0 to 100.
    pub smooth: u8,
    /// Tolerance in source pixels.
    pub tolerance: f64,
    /// Ignore below in millimetres.
    pub ignore_below: f64,
    /// Corners, 0 to 100 (set by presets).
    pub corners: u8,
    /// Upscale: `true` is Auto (set by presets).
    pub upscale_auto: bool,
    /// Denoise index: 0 Off, 1 Low, 2 High (set by presets).
    pub denoise: u8,
    /// The method: 0 Brightness, 1 Remove background, 2 Pick colours.
    pub method: u8,
    /// The Colour tolerance of Remove background.
    pub remove_tolerance: u8,
    /// The Colour tolerance of Pick colours.
    pub pick_tolerance: u8,
    /// Whether the Advanced block is open.
    pub advanced_open: bool,
    /// The window size in logical points at the last exit.
    pub window: Option<[f32; 2]>,
}

impl Default for Settings {
    fn default() -> Self {
        Self::from_preset(&PRESETS[0])
    }
}

impl Settings {
    fn from_preset(preset: &Preset) -> Self {
        Self {
            smooth: preset.smooth,
            tolerance: preset.tolerance_px.as_px(),
            ignore_below: preset.specks_mm.as_mm(),
            corners: preset.corners,
            upscale_auto: preset.upscale == Upscale::Auto,
            denoise: preset.denoise.index(),
            method: 0,
            remove_tolerance: vectrace::DEFAULT_BACKGROUND_TOLERANCE,
            pick_tolerance: vectrace::DEFAULT_PICK_TOLERANCE,
            advanced_open: false,
            window: None,
        }
    }

    /// The six preset values of preset number `index` taken over; the method, its tolerances,
    /// the Advanced block and the window size stay.
    #[must_use]
    pub fn with_preset(self, index: usize) -> Self {
        let preset = PRESETS.get(index).unwrap_or(&PRESETS[0]);
        let values = Self::from_preset(preset);
        Self {
            smooth: values.smooth,
            tolerance: values.tolerance,
            ignore_below: values.ignore_below,
            corners: values.corners,
            upscale_auto: values.upscale_auto,
            denoise: values.denoise,
            ..self
        }
    }

    fn matches(&self, preset: &Preset) -> bool {
        let same = |a: f64, b: f64| (a - b).abs() < 1e-9;
        self.smooth == preset.smooth
            && same(self.tolerance, preset.tolerance_px.as_px())
            && same(self.ignore_below, preset.specks_mm.as_mm())
            && self.corners == preset.corners
            && self.upscale_auto == (preset.upscale == Upscale::Auto)
            && self.denoise == preset.denoise.index()
    }

    /// The index of the preset the six values equal, `None` for "Custom".
    #[must_use]
    pub fn preset_index(&self) -> Option<usize> {
        PRESETS.iter().position(|preset| self.matches(preset))
    }

    /// The Preset button's text.
    #[must_use]
    pub fn preset_label(&self) -> &'static str {
        self.preset_index().map_or("Custom", |i| PRESETS[i].label)
    }

    /// Every value inside its range.
    #[must_use]
    pub fn clamped(self) -> Self {
        Self {
            smooth: self.smooth.min(100),
            tolerance: TOLERANCE.clamp(self.tolerance),
            ignore_below: IGNORE_BELOW.clamp(self.ignore_below),
            corners: self.corners.min(100),
            upscale_auto: self.upscale_auto,
            denoise: self.denoise.min(2),
            method: self.method.min(2),
            remove_tolerance: self.remove_tolerance.min(100),
            pick_tolerance: self.pick_tolerance.min(100),
            advanced_open: self.advanced_open,
            window: self
                .window
                .filter(|[w, h]| w.is_finite() && h.is_finite() && *w >= 100.0 && *h >= 100.0),
        }
    }

    /// The tracer's values for `threshold` (a luma 0 to 255).
    #[must_use]
    pub fn trace_params(&self, threshold: u8) -> vectrace::TraceParams {
        vectrace::TraceParams {
            threshold,
            smooth: self.smooth,
            tolerance: PixelLength::from_px(self.tolerance),
            specks: vectrace::Mm::from_mm(self.ignore_below),
            corners: self.corners,
            upscale: if self.upscale_auto {
                Upscale::Auto
            } else {
                Upscale::Off
            },
            denoise: Denoise::from_index(self.denoise),
            ..vectrace::TraceParams::with_threshold(threshold)
        }
    }

    /// The settings file text.
    #[must_use]
    pub fn to_toml(&self) -> String {
        // invariant: plain numbers and booleans always serialise.
        toml::to_string(self).unwrap_or_default()
    }

    /// The settings from file text; a corrupt text gives the defaults.
    #[must_use]
    pub fn from_toml(text: &str) -> Self {
        toml::from_str::<Self>(text)
            .map(Self::clamped)
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_four_fields_have_the_ranges_and_steps_of_the_spec() {
        assert_eq!(
            (
                THRESHOLD.min,
                THRESHOLD.max,
                THRESHOLD.step,
                THRESHOLD.big_step
            ),
            (0.0, 100.0, 1.0, 10.0)
        );
        assert_eq!(
            (SMOOTH.min, SMOOTH.max, SMOOTH.step, SMOOTH.big_step),
            (0.0, 100.0, 1.0, 10.0)
        );
        assert_eq!(
            (TOLERANCE.min, TOLERANCE.max, TOLERANCE.step),
            (0.1, 3.0, 0.05)
        );
        assert_eq!((TOLERANCE.big_step, TOLERANCE.fine_step), (0.5, 0.01));
        assert_eq!(
            (IGNORE_BELOW.min, IGNORE_BELOW.max, IGNORE_BELOW.step),
            (0.0, 10.0, 0.1)
        );
        assert_eq!((IGNORE_BELOW.big_step, IGNORE_BELOW.fine_step), (1.0, 0.01));
    }

    #[test]
    fn nudging_steps_and_clamps_without_float_noise() {
        assert_eq!(TOLERANCE.nudge(0.5, 1, Stride::Step), 0.55);
        assert_eq!(TOLERANCE.nudge(0.5, -1, Stride::Big), 0.1);
        assert_eq!(TOLERANCE.nudge(0.5, 1, Stride::Fine), 0.51);
        assert_eq!(IGNORE_BELOW.nudge(0.3, 1, Stride::Step), 0.4);
        assert_eq!(IGNORE_BELOW.nudge(9.5, 1, Stride::Big), 10.0);
        assert_eq!(SMOOTH.nudge(80.0, 1, Stride::Step), 81.0);
        assert_eq!(SMOOTH.nudge(95.0, 1, Stride::Big), 100.0);
    }

    #[test]
    fn a_typed_value_is_clamped_and_text_is_refused() {
        assert_eq!(TOLERANCE.parse("9"), Some(3.0));
        assert_eq!(TOLERANCE.parse("0,25 px"), Some(0.25));
        assert_eq!(IGNORE_BELOW.parse("-4"), Some(0.0));
        assert_eq!(SMOOTH.parse("abc"), None);
        assert_eq!(THRESHOLD.parse("62 %"), Some(62.0));
    }

    #[test]
    fn values_show_with_their_decimals() {
        assert_eq!(TOLERANCE.format(0.5), "0.50");
        assert_eq!(SMOOTH.format(80.0), "80");
        assert_eq!(IGNORE_BELOW.format(0.5), "0.50");
    }

    #[test]
    fn drag_fractions_map_to_the_range() {
        assert_eq!(SMOOTH.at_fraction(0.8), 80.0);
        assert_eq!(SMOOTH.fraction(80.0), 0.8);
        assert_eq!(SMOOTH.at_fraction(2.0), 100.0);
    }

    #[test]
    fn threshold_percent_and_luma_round_half_up() {
        assert_eq!(luma_of_percent(62.0), 158);
        assert_eq!(percent_of_luma(158), 62.0);
        assert_eq!(luma_of_percent(100.0), 255);
        assert_eq!(percent_of_luma(0), 0.0);
    }

    #[test]
    fn the_first_start_is_logo_and_a_change_reads_custom() {
        let mut settings = Settings::default();
        assert_eq!(settings.preset_label(), "Logo / Line art");
        settings.smooth = 81;
        assert_eq!(settings.preset_label(), "Custom");
        settings.smooth = 80;
        assert_eq!(settings.preset_label(), "Logo / Line art");
    }

    #[test]
    fn every_preset_sets_its_six_values() {
        let expected = [
            ("Logo / Line art", 80, 0.5, 0.5, 70, false, 0),
            ("Photo", 50, 0.5, 0.1, 30, false, 0),
            (
                "Silhouette for cutting/carving",
                100,
                1.5,
                2.0,
                70,
                false,
                1,
            ),
            ("Engraving", 80, 0.35, 0.1, 70, true, 0),
        ];
        for (index, (label, smooth, tolerance, ignore, corners, auto, denoise)) in
            expected.into_iter().enumerate()
        {
            let s = Settings::default().with_preset(index);
            assert_eq!(s.preset_label(), label);
            assert_eq!(
                (
                    s.smooth,
                    s.tolerance,
                    s.ignore_below,
                    s.corners,
                    s.upscale_auto,
                    s.denoise
                ),
                (smooth, tolerance, ignore, corners, auto, denoise)
            );
        }
    }

    #[test]
    fn picking_a_preset_keeps_the_window_size() {
        let s = Settings {
            window: Some([900.0, 700.0]),
            ..Settings::default()
        };
        assert_eq!(s.with_preset(1).window, Some([900.0, 700.0]));
    }

    #[test]
    fn the_toml_text_round_trips() {
        let s = Settings {
            smooth: 12,
            tolerance: 1.25,
            ignore_below: 3.5,
            window: Some([1000.0, 640.0]),
            ..Settings::default()
        };
        assert_eq!(Settings::from_toml(&s.to_toml()), s);
    }

    #[test]
    fn a_corrupt_or_empty_file_gives_the_defaults() {
        assert_eq!(Settings::from_toml("smooth = [[["), Settings::default());
        assert_eq!(Settings::from_toml(""), Settings::default());
        assert_eq!(
            Settings::from_toml("smooth = \"loud\""),
            Settings::default()
        );
    }

    #[test]
    fn out_of_range_values_in_the_file_are_clamped() {
        let s = Settings::from_toml(
            "smooth = 250\ntolerance = 99.0\nignore_below = -3.0\ndenoise = 9\n",
        );
        assert_eq!(
            (s.smooth, s.tolerance, s.ignore_below, s.denoise),
            (100, 3.0, 0.0, 2)
        );
    }

    #[test]
    fn missing_keys_take_the_logo_values() {
        let s = Settings::from_toml("smooth = 30\n");
        assert_eq!((s.smooth, s.tolerance, s.corners), (30, 0.5, 70));
    }

    #[test]
    fn trace_params_carry_the_settings() {
        let p = Settings::default().with_preset(3).trace_params(120);
        assert_eq!((p.threshold, p.smooth, p.corners), (120, 80, 70));
        assert_eq!(p.upscale, Upscale::Auto);
        assert!((p.tolerance.as_px() - 0.35).abs() < 1e-12);
    }

    #[test]
    fn each_of_the_six_values_alone_makes_the_preset_read_custom() {
        let base = Settings::default();
        let edits: [fn(&mut Settings); 6] = [
            |s| s.smooth = 81,
            |s| s.tolerance = 0.55,
            |s| s.ignore_below = 0.6,
            |s| s.corners = 71,
            |s| s.upscale_auto = true,
            |s| s.denoise = 2,
        ];
        for edit in edits {
            let mut s = base;
            edit(&mut s);
            assert_eq!(s.preset_label(), "Custom");
            assert_eq!(s.preset_index(), None);
            assert_eq!(s.with_preset(0).preset_label(), "Logo / Line art");
        }
    }

    #[test]
    fn picking_a_preset_never_changes_method_tolerances_or_the_block() {
        let s = Settings {
            method: 2,
            remove_tolerance: 12,
            pick_tolerance: 77,
            advanced_open: true,
            ..Settings::default()
        };
        for i in 0..4 {
            let p = s.with_preset(i);
            assert_eq!(
                (p.method, p.remove_tolerance, p.pick_tolerance, p.advanced_open),
                (2, 12, 77, true)
            );
        }
    }

    #[test]
    fn a_custom_set_of_six_values_survives_the_file() {
        let s = Settings {
            corners: 12,
            denoise: 2,
            upscale_auto: true,
            method: 1,
            remove_tolerance: 25,
            pick_tolerance: 5,
            advanced_open: true,
            ..Settings::default()
        };
        let back = Settings::from_toml(&s.to_toml());
        assert_eq!(back, s);
        assert_eq!(back.preset_label(), "Custom");
    }

    #[test]
    fn the_colour_tolerances_start_at_40_and_19() {
        let s = Settings::default();
        assert_eq!((s.remove_tolerance, s.pick_tolerance, s.method), (40, 19, 0));
        assert!(!s.advanced_open);
    }

    #[test]
    fn out_of_range_methods_and_tolerances_in_the_file_are_clamped() {
        let s = Settings::from_toml("method = 9\nremove_tolerance = 200\npick_tolerance = 101\n");
        assert_eq!((s.method, s.remove_tolerance, s.pick_tolerance), (2, 100, 100));
    }

    #[test]
    fn the_new_fields_have_the_ranges_of_the_spec() {
        assert_eq!((CORNERS.min, CORNERS.max, CORNERS.step, CORNERS.big_step), (0.0, 100.0, 1.0, 10.0));
        assert_eq!(
            (COLOUR_TOLERANCE.min, COLOUR_TOLERANCE.max, COLOUR_TOLERANCE.big_step),
            (0.0, 100.0, 10.0)
        );
        assert_eq!(COLOUR_TOLERANCE.label, "Colour tolerance");
    }
}
