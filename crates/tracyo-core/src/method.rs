//! The method of a trace, the values that belong to the shown picture and the tracer's selection
//! they make.

use vectrace::{MAX_SWATCHES, Rgb8, Selection, Swatches};

use crate::palette::{ColourSpec, Edits};
use crate::settings::Settings;

/// How pixels become shapes (the Method menu).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    /// Pixels darker than the Threshold.
    Brightness,
    /// Pixels unlike the background colour.
    RemoveBackground,
    /// Pixels like the picked colours.
    PickColours,
}

impl Method {
    /// The methods in menu order.
    pub const ALL: [Self; 3] = [Self::Brightness, Self::RemoveBackground, Self::PickColours];

    /// The method at menu position `index`; Brightness for anything else.
    #[must_use]
    pub fn from_index(index: u8) -> Self {
        Self::ALL
            .get(usize::from(index))
            .copied()
            .unwrap_or(Self::Brightness)
    }

    /// The menu position.
    #[must_use]
    pub fn index(self) -> u8 {
        match self {
            Self::Brightness => 0,
            Self::RemoveBackground => 1,
            Self::PickColours => 2,
        }
    }

    /// The name in the menu and on the button.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Brightness => "Brightness",
            Self::RemoveBackground => "Remove background",
            Self::PickColours => "Pick colours",
        }
    }

    /// The menu's second line.
    #[must_use]
    pub fn detail(self) -> &'static str {
        match self {
            Self::Brightness => "Dark and light pixels by a cut-off.",
            Self::RemoveBackground => "Everything unlike the background.",
            Self::PickColours => "Pixels like the colours you picked.",
        }
    }
}

/// `#RRGGBB` of a colour, upper case.
#[must_use]
pub fn hex(colour: Rgb8) -> String {
    format!("#{:02X}{:02X}{:02X}", colour.r, colour.g, colour.b)
}

/// The values that belong to the shown picture and are not remembered: they start from the
/// analysis of each picture.
#[derive(Debug, Clone, PartialEq)]
pub struct PictureValues {
    /// The Threshold as a luma, 0 to 255.
    pub threshold: u8,
    /// Invert: trace what the method leaves out.
    pub invert: bool,
    /// Drop border shapes.
    pub drop_border: bool,
    /// The background picked with the pipette; `None` is Auto (the analysis's colour).
    pub manual_background: Option<Rgb8>,
    /// The picked colours of Pick colours, in the order they were picked.
    pub swatches: Vec<Rgb8>,
    /// The Multicolour palette's edits: added, excluded and deleted colours.
    pub palette: Edits,
}

impl PictureValues {
    /// The values of a picture that was just opened, with the automatic `threshold`.
    #[must_use]
    pub fn new(threshold: u8) -> Self {
        Self {
            threshold,
            invert: false,
            drop_border: false,
            manual_background: None,
            swatches: Vec::new(),
            palette: Edits::default(),
        }
    }

    /// Whether Auto is pressed.
    #[must_use]
    pub fn background_auto(&self) -> bool {
        self.manual_background.is_none()
    }

    /// The background colour in use: the picked one, or `suggested` while Auto is pressed;
    /// `None` is a transparent background.
    #[must_use]
    pub fn background(&self, suggested: Option<Rgb8>) -> Option<Rgb8> {
        self.manual_background.or(suggested)
    }

    /// A pick of the pipette: it replaces the picked colours, or with `add` joins them (a
    /// colour already picked, or a ninth, is not added).
    pub fn pick(&mut self, colour: Rgb8, add: bool) {
        if !add {
            self.swatches = vec![colour];
        } else if !self.swatches.contains(&colour) && self.swatches.len() < MAX_SWATCHES {
            self.swatches.push(colour);
        }
    }

    /// Takes the picked colour at `index` out.
    pub fn remove(&mut self, index: usize) {
        if index < self.swatches.len() {
            self.swatches.remove(index);
        }
    }

    /// The tracer's selection for the method of `settings`; `suggested` is the analysis's
    /// background colour.
    #[must_use]
    pub fn selection(&self, settings: &Settings, suggested: Option<Rgb8>) -> Selection {
        match Method::from_index(settings.method) {
            Method::Brightness => Selection::Brightness,
            Method::RemoveBackground => Selection::RemoveBackground {
                background: self.background(suggested),
                tolerance: settings.remove_tolerance,
            },
            Method::PickColours => Selection::PickColours {
                swatches: Swatches::from_slice(&self.swatches),
                background: suggested,
                tolerance: settings.pick_tolerance,
            },
        }
    }
}

impl PictureValues {
    /// What Multicolour asks the tracer for, `None` for Single colour. A deleted palette is traced
    /// as exactly its colours; Pick colours has its picked colours as the palette.
    #[must_use]
    pub fn colour_spec(&self, settings: &Settings) -> Option<ColourSpec> {
        if !settings.multicolour {
            return None;
        }
        let own_palette = Method::from_index(settings.method) != Method::PickColours;
        let kept = self.palette.kept.as_ref().filter(|_| own_palette);
        let count = match kept {
            // invariant: a kept palette holds at most 16 colours.
            Some(kept) => Some(u8::try_from(kept.len()).unwrap_or(u8::MAX)),
            None => (settings.colours != 0).then_some(settings.colours),
        };
        let fixed = match kept {
            Some(kept) => kept.clone(),
            None if own_palette => self.palette.fixed.clone(),
            None => Vec::new(),
        };
        Some(ColourSpec {
            count,
            fixed,
            excluded: self.palette.excluded.clone(),
        })
    }

    /// Whether every colour of the Multicolour palette was deleted: nothing is traced.
    #[must_use]
    pub fn palette_deleted(&self, settings: &Settings) -> bool {
        settings.multicolour
            && Method::from_index(settings.method) != Method::PickColours
            && self.palette.kept.as_ref().is_some_and(Vec::is_empty)
    }
}

/// The changed Advanced values, for the expander's tooltip ("Changed: Invert, Corners"), `None`
/// when all are at their defaults. Denoise, Upscale and Corners are compared with the Logo
/// preset's values, the defaults of vectrace's schema.
#[must_use]
pub fn changed_advanced(settings: &Settings, values: &PictureValues) -> Option<String> {
    let default = Settings::default();
    let mut changed = Vec::new();
    if values.invert {
        changed.push("Invert");
    }
    if values.drop_border {
        changed.push("Drop border shapes");
    }
    if settings.recognise_text {
        changed.push("Recognise text");
    }
    if settings.recognise_shapes {
        changed.push("Recognise shapes");
    }
    if settings.denoise != default.denoise {
        changed.push("Denoise");
    }
    if settings.upscale_auto != default.upscale_auto {
        changed.push("Upscale");
    }
    if settings.corners != default.corners {
        changed.push("Corners");
    }
    (!changed.is_empty()).then(|| format!("Changed: {}", changed.join(", ")))
}

#[cfg(test)]
mod tests {
    use super::*;

    const RED: Rgb8 = Rgb8::new(255, 0, 0);
    const BLUE: Rgb8 = Rgb8::new(0, 0, 255);
    const WHITE: Rgb8 = Rgb8::new(255, 255, 255);

    #[test]
    fn the_menu_has_three_methods_with_their_texts() {
        let rows: Vec<_> = Method::ALL
            .iter()
            .map(|m| (m.label(), m.detail()))
            .collect();
        assert_eq!(
            rows,
            [
                ("Brightness", "Dark and light pixels by a cut-off."),
                ("Remove background", "Everything unlike the background."),
                ("Pick colours", "Pixels like the colours you picked."),
            ]
        );
        for (i, m) in Method::ALL.iter().enumerate() {
            assert_eq!(usize::from(m.index()), i);
            assert_eq!(Method::from_index(m.index()), *m);
        }
        assert_eq!(Method::from_index(9), Method::Brightness);
    }

    #[test]
    fn hex_is_upper_case_with_a_hash() {
        assert_eq!(hex(Rgb8::new(0x33, 0x66, 0xcc)), "#3366CC");
        assert_eq!(hex(Rgb8::new(0, 0, 0)), "#000000");
    }

    #[test]
    fn a_click_replaces_the_picked_colours_and_shift_adds_up_to_eight() {
        let mut v = PictureValues::new(128);
        v.pick(RED, false);
        v.pick(BLUE, true);
        assert_eq!(v.swatches, [RED, BLUE]);
        v.pick(BLUE, true);
        assert_eq!(
            v.swatches,
            [RED, BLUE],
            "a colour already picked is not added twice"
        );
        for i in 0..10 {
            v.pick(Rgb8::new(i, 1, 1), true);
        }
        assert_eq!(v.swatches.len(), MAX_SWATCHES);
        v.pick(WHITE, false);
        assert_eq!(v.swatches, [WHITE]);
    }

    #[test]
    fn removing_a_swatch_keeps_the_order_of_the_rest() {
        let mut v = PictureValues::new(128);
        v.swatches = vec![RED, BLUE, WHITE];
        v.remove(1);
        assert_eq!(v.swatches, [RED, WHITE]);
        v.remove(7);
        assert_eq!(v.swatches, [RED, WHITE]);
    }

    #[test]
    fn auto_follows_the_suggestion_and_a_pick_releases_it() {
        let mut v = PictureValues::new(128);
        assert!(v.background_auto());
        assert_eq!(v.background(Some(WHITE)), Some(WHITE));
        assert_eq!(v.background(None), None);
        v.manual_background = Some(RED);
        assert!(!v.background_auto());
        assert_eq!(v.background(Some(WHITE)), Some(RED));
    }

    #[test]
    fn the_selection_follows_the_method_and_its_own_tolerance() {
        let mut s = Settings::default();
        let mut v = PictureValues::new(128);
        v.swatches = vec![RED];
        assert_eq!(v.selection(&s, Some(WHITE)), Selection::Brightness);
        s.method = 1;
        assert_eq!(
            v.selection(&s, Some(WHITE)),
            Selection::RemoveBackground {
                background: Some(WHITE),
                tolerance: 40
            }
        );
        s.method = 2;
        let Selection::PickColours {
            swatches,
            background,
            tolerance,
        } = v.selection(&s, Some(WHITE))
        else {
            panic!("not Pick colours");
        };
        assert_eq!(swatches.as_slice(), [RED]);
        assert_eq!((background, tolerance), (Some(WHITE), 19));
    }

    #[test]
    fn pick_colours_takes_the_suggested_background_not_the_picked_one() {
        let s = Settings {
            method: 2,
            ..Settings::default()
        };
        let mut v = PictureValues::new(128);
        v.manual_background = Some(RED);
        let Selection::PickColours { background, .. } = v.selection(&s, Some(WHITE)) else {
            panic!("not Pick colours");
        };
        assert_eq!(background, Some(WHITE));
    }

    #[test]
    fn the_expander_names_the_changed_advanced_values() {
        let mut s = Settings::default();
        let mut v = PictureValues::new(128);
        assert_eq!(changed_advanced(&s, &v), None);
        v.invert = true;
        s.corners = 71;
        assert_eq!(
            changed_advanced(&s, &v).as_deref(),
            Some("Changed: Invert, Corners")
        );
        v.invert = false;
        v.drop_border = true;
        s.corners = 70;
        s.denoise = 1;
        s.upscale_auto = true;
        assert_eq!(
            changed_advanced(&s, &v).as_deref(),
            Some("Changed: Drop border shapes, Denoise, Upscale")
        );
    }

    #[test]
    fn the_recognise_switches_are_named_between_drop_border_shapes_and_denoise() {
        let mut v = PictureValues::new(128);
        v.drop_border = true;
        let s = Settings {
            recognise_text: true,
            recognise_shapes: true,
            denoise: 2,
            ..Settings::default()
        };
        assert_eq!(
            changed_advanced(&s, &v).as_deref(),
            Some("Changed: Drop border shapes, Recognise text, Recognise shapes, Denoise")
        );
    }

    #[test]
    fn single_colour_asks_for_no_palette() {
        let v = PictureValues::new(128);
        assert_eq!(v.colour_spec(&Settings::default()), None);
    }

    #[test]
    fn multicolour_asks_for_auto_or_the_typed_count_with_the_edits() {
        let mut v = PictureValues::new(128);
        let mut s = Settings {
            multicolour: true,
            ..Settings::default()
        };
        v.palette.fixed = vec![RED];
        v.palette.excluded = vec![BLUE];
        let spec = v.colour_spec(&s).unwrap();
        assert_eq!(spec.count, None);
        assert_eq!((spec.fixed, spec.excluded), (vec![RED], vec![BLUE]));
        s.colours = 5;
        assert_eq!(v.colour_spec(&s).unwrap().count, Some(5));
    }

    #[test]
    fn a_deleted_palette_is_traced_as_exactly_its_colours() {
        let mut v = PictureValues::new(128);
        let s = Settings {
            multicolour: true,
            colours: 9,
            ..Settings::default()
        };
        v.palette.fixed = vec![WHITE];
        v.palette.delete(&[RED, BLUE, WHITE], BLUE);
        let spec = v.colour_spec(&s).unwrap();
        assert_eq!(spec.count, Some(2));
        assert_eq!(spec.fixed, [RED, WHITE]);
        assert!(!v.palette_deleted(&s));
        v.palette.delete(&[RED, WHITE], RED);
        v.palette.delete(&[WHITE], WHITE);
        assert!(v.palette_deleted(&s));
    }

    #[test]
    fn pick_colours_has_its_picked_colours_as_the_palette() {
        let mut v = PictureValues::new(128);
        v.swatches = vec![RED];
        v.palette.fixed = vec![BLUE];
        v.palette.delete(&[RED, BLUE], BLUE);
        let s = Settings {
            multicolour: true,
            colours: 4,
            method: 2,
            ..Settings::default()
        };
        let spec = v.colour_spec(&s).unwrap();
        assert!(spec.fixed.is_empty(), "the picked colours are the palette");
        assert_eq!(spec.count, Some(4));
        assert!(!v.palette_deleted(&s));
    }
}
