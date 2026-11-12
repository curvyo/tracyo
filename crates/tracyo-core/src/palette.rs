//! The palette of a Multicolour trace: what the tracer is asked for, the order and names of the
//! swatches, and the edits delete, exclude and pick make.

use vectrace::wire::Palette;
use vectrace::{MAX_COLOURS, Rgb8};

use crate::method::hex;

/// What Multicolour asks the tracer for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColourSpec {
    /// How many colours (2 to 16, the fixed ones included); `None` finds the count.
    pub count: Option<u8>,
    /// Colours the maker put into the palette; kept exactly.
    pub fixed: Vec<Rgb8>,
    /// Palette colours that become background.
    pub excluded: Vec<Rgb8>,
}

/// Colours closer than this merge, in bytes (0.03 of the sRGB unit cube, vectrace's distance).
const MERGE_DISTANCE: f64 = 0.03 * 255.0;

/// The swatch row's order: fixed colours first, then by area, most first (ties by colour code).
#[must_use]
pub fn strip_order(palette: &[Palette]) -> Vec<Palette> {
    let mut row = palette.to_vec();
    row.sort_by(|a, b| {
        b.fixed
            .cmp(&a.fixed)
            .then(b.pixels.cmp(&a.pixels))
            .then_with(|| hex(a.colour).cmp(&hex(b.colour)))
    });
    row
}

/// The colour of `shown` within the merge distance of `colour`, if any.
#[must_use]
pub fn same_colour(shown: &[Rgb8], colour: Rgb8) -> Option<Rgb8> {
    let channel = |a: u8, b: u8| f64::from(a) - f64::from(b);
    shown.iter().copied().find(|c| {
        channel(c.r, colour.r).hypot(channel(c.g, colour.g).hypot(channel(c.b, colour.b)))
            < MERGE_DISTANCE
    })
}

/// The hue words up to each hue angle.
const HUE_WORDS: [(f64, &str); 9] = [
    (15.0, "red"),
    (40.0, "orange"),
    (65.0, "yellow"),
    (160.0, "green"),
    (200.0, "teal"),
    (255.0, "blue"),
    (290.0, "purple"),
    (335.0, "pink"),
    (360.0, "red"),
];

/// A colour's name from a fixed hue and lightness table: greys by lightness, else the hue word,
/// "Pale" when light, "Dark" when dark, dark orange is "Brown".
#[must_use]
pub fn colour_name(colour: Rgb8) -> String {
    let [r, g, b] = [colour.r, colour.g, colour.b].map(|c| f64::from(c) / 255.0);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let light = f64::midpoint(max, min);
    let chroma = max - min;
    let saturation = if chroma == 0.0 {
        0.0
    } else {
        chroma / (1.0 - (2.0 * light - 1.0).abs())
    };
    if saturation < 0.12 || chroma < 0.04 {
        return match light {
            l if l < 0.12 => "Black",
            l if l < 0.35 => "Dark grey",
            l if l < 0.65 => "Grey",
            l if l < 0.92 => "Light grey",
            _ => "White",
        }
        .to_owned();
    }
    let hue = if (max - r).abs() < f64::EPSILON {
        ((g - b) / chroma) % 6.0
    } else if (max - g).abs() < f64::EPSILON {
        (b - r) / chroma + 2.0
    } else {
        (r - g) / chroma + 4.0
    };
    let hue = (hue * 60.0 + 360.0) % 360.0;
    let word = HUE_WORDS
        .iter()
        .find(|(up_to, _)| hue < *up_to)
        .map_or("red", |(_, w)| *w);
    if light < 0.3 {
        if word == "orange" {
            "Brown".to_owned()
        } else {
            format!("Dark {word}")
        }
    } else if light > 0.78 {
        format!("Pale {word}")
    } else {
        let mut chars = word.chars();
        chars.next().map_or_else(String::new, |first| {
            first.to_uppercase().collect::<String>() + chars.as_str()
        })
    }
}

/// A swatch's tooltip line: "#4B392F · Brown".
#[must_use]
pub fn swatch_tooltip(colour: Rgb8) -> String {
    format!("{} \u{b7} {}", hex(colour), colour_name(colour))
}

/// A swatch's accessible name: its colour code and name, then ", excluded" when it is.
#[must_use]
pub fn swatch_label(colour: Rgb8, excluded: bool) -> String {
    if excluded {
        format!("{}, excluded", swatch_tooltip(colour))
    } else {
        swatch_tooltip(colour)
    }
}

/// What a key does on a palette swatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwatchKey {
    /// Move the roving stop to this swatch.
    Move(usize),
    /// Exclude or bring back this swatch.
    Toggle(usize),
    /// Delete this swatch's colour.
    Delete(usize),
}

/// What `key` does on swatch `index` of `count`: arrows, Home and End move, Enter or Space
/// toggle Exclude, Shift+Enter, Delete and Backspace delete.
#[must_use]
pub fn swatch_key(key: SwatchKeyName, shift: bool, index: usize, count: usize) -> SwatchKey {
    let last = count.saturating_sub(1);
    match key {
        SwatchKeyName::Right => SwatchKey::Move(last.min(index + 1)),
        SwatchKeyName::Left => SwatchKey::Move(index.saturating_sub(1)),
        SwatchKeyName::Home => SwatchKey::Move(0),
        SwatchKeyName::End => SwatchKey::Move(last),
        SwatchKeyName::Enter if shift => SwatchKey::Delete(index),
        SwatchKeyName::Enter | SwatchKeyName::Space => SwatchKey::Toggle(index),
        SwatchKeyName::Delete | SwatchKeyName::Backspace => SwatchKey::Delete(index),
    }
}

/// The keys a palette swatch answers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwatchKeyName {
    /// Arrow right.
    Right,
    /// Arrow left.
    Left,
    /// Home.
    Home,
    /// End.
    End,
    /// Enter.
    Enter,
    /// Space.
    Space,
    /// Delete.
    Delete,
    /// Backspace.
    Backspace,
}

/// The palette after the maker's edits: the colours kept exactly, the ones excluded, and the
/// ones the palette pipette added.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Edits {
    /// Colours the palette pipette added (the found colours fill the other slots).
    pub fixed: Vec<Rgb8>,
    /// Colours that became background.
    pub excluded: Vec<Rgb8>,
    /// The palette after a delete: exactly these colours, every pixel matched again among them.
    pub kept: Option<Vec<Rgb8>>,
}

/// What a pick of the palette pipette did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Picked {
    /// The colour joined the palette.
    Added,
    /// A colour of the palette is within the merge distance; nothing was added.
    Same(Rgb8),
}

impl Edits {
    /// Whether `colour` is excluded.
    #[must_use]
    pub fn is_excluded(&self, colour: Rgb8) -> bool {
        self.excluded.contains(&colour)
    }

    /// Exclude on or off for `colour`.
    pub fn toggle_exclude(&mut self, colour: Rgb8) {
        if let Some(at) = self.excluded.iter().position(|c| *c == colour) {
            self.excluded.remove(at);
        } else {
            self.excluded.push(colour);
        }
    }

    /// Deletes `colour` from the `shown` palette: the palette becomes the other colours, and
    /// `colour` is no longer excluded. Every colour can go, down to none.
    pub fn delete(&mut self, shown: &[Rgb8], colour: Rgb8) {
        if !shown.contains(&colour) {
            return;
        }
        self.kept = Some(shown.iter().copied().filter(|c| *c != colour).collect());
        self.excluded.retain(|c| *c != colour);
    }

    /// A pick of the palette pipette: a new fixed colour (joining the kept palette after a
    /// delete), or the `shown` colour it equals.
    pub fn pick(&mut self, shown: &[Rgb8], colour: Rgb8) -> Picked {
        if let Some(twin) = same_colour(shown, colour) {
            return Picked::Same(twin);
        }
        match &mut self.kept {
            Some(kept) => {
                if kept.len() < MAX_COLOURS {
                    kept.push(colour);
                }
            }
            None => {
                if !self.fixed.contains(&colour) {
                    self.fixed.push(colour);
                }
            }
        }
        Picked::Added
    }

    /// The deleted palette is forgotten (a new method or Colours value).
    pub fn forget_kept(&mut self) {
        self.kept = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RED: Rgb8 = Rgb8::new(200, 16, 46);
    const BLUE: Rgb8 = Rgb8::new(0x2B, 0x4C, 0x9B);
    const GREEN: Rgb8 = Rgb8::new(30, 120, 60);

    fn entry(colour: Rgb8, fixed: bool, pixels: u64) -> Palette {
        Palette {
            colour,
            fixed,
            pixels,
        }
    }

    #[test]
    fn the_row_has_fixed_colours_first_then_the_largest_area() {
        let row = strip_order(&[
            entry(BLUE, false, 10),
            entry(GREEN, false, 30),
            entry(RED, true, 1),
            entry(Rgb8::new(1, 2, 3), false, 30),
        ]);
        let colours: Vec<Rgb8> = row.iter().map(|p| p.colour).collect();
        assert_eq!(
            colours,
            [RED, Rgb8::new(1, 2, 3), GREEN, BLUE],
            "ties by code"
        );
    }

    #[test]
    fn names_follow_the_hue_and_lightness_table() {
        assert_eq!(colour_name(BLUE), "Blue");
        assert_eq!(colour_name(Rgb8::new(0x4B, 0x39, 0x2F)), "Brown");
        assert_eq!(colour_name(Rgb8::new(255, 255, 255)), "White");
        assert_eq!(colour_name(Rgb8::new(0, 0, 0)), "Black");
        assert_eq!(colour_name(Rgb8::new(128, 128, 128)), "Grey");
        assert_eq!(colour_name(Rgb8::new(255, 0, 0)), "Red");
        assert_eq!(colour_name(Rgb8::new(255, 200, 200)), "Pale red");
        assert_eq!(colour_name(Rgb8::new(0, 60, 0)), "Dark green");
    }

    #[test]
    fn the_tooltip_and_the_accessible_name_give_code_name_and_state() {
        assert_eq!(swatch_tooltip(BLUE), "#2B4C9B \u{b7} Blue");
        assert_eq!(swatch_label(BLUE, false), "#2B4C9B \u{b7} Blue");
        assert_eq!(swatch_label(BLUE, true), "#2B4C9B \u{b7} Blue, excluded");
    }

    #[test]
    fn exclude_toggles_and_a_second_press_brings_the_colour_back() {
        let mut e = Edits::default();
        e.toggle_exclude(BLUE);
        assert!(e.is_excluded(BLUE) && !e.is_excluded(RED));
        e.toggle_exclude(BLUE);
        assert!(!e.is_excluded(BLUE));
    }

    #[test]
    fn delete_keeps_exactly_the_other_colours_and_unexcludes_the_deleted() {
        let mut e = Edits::default();
        e.toggle_exclude(BLUE);
        e.toggle_exclude(RED);
        e.delete(&[RED, BLUE, GREEN], BLUE);
        assert_eq!(e.kept, Some(vec![RED, GREEN]));
        assert_eq!(
            e.excluded,
            [RED],
            "the deleted colour is no longer excluded"
        );
        e.delete(&[RED, GREEN], RED);
        e.delete(&[GREEN], GREEN);
        assert_eq!(e.kept, Some(vec![]), "every colour can be deleted");
        let before = e.clone();
        e.delete(&[], BLUE);
        assert_eq!(e, before, "a colour that is not shown changes nothing");
    }

    #[test]
    fn a_pick_adds_a_fixed_colour_unless_the_palette_has_it() {
        let mut e = Edits::default();
        let shown = [RED, GREEN];
        assert_eq!(e.pick(&shown, BLUE), Picked::Added);
        assert_eq!(e.fixed, [BLUE]);
        assert_eq!(
            e.pick(&shown, Rgb8::new(201, 17, 45)),
            Picked::Same(RED),
            "within the merge distance"
        );
        assert_eq!(e.fixed, [BLUE]);
        assert_eq!(e.pick(&[RED, GREEN, BLUE], BLUE), Picked::Same(BLUE));
    }

    #[test]
    fn after_a_delete_a_pick_joins_the_kept_palette_up_to_sixteen() {
        let mut e = Edits::default();
        e.delete(&[RED, GREEN], GREEN);
        assert_eq!(e.pick(&[RED], BLUE), Picked::Added);
        assert_eq!(e.kept, Some(vec![RED, BLUE]));
        assert_eq!(e.fixed.len(), 0);
        e.kept = Some((0..16).map(|i| Rgb8::new(i * 10, 0, 0)).collect::<Vec<_>>());
        let shown: Vec<Rgb8> = e.kept.clone().unwrap_or_default();
        e.pick(&shown, Rgb8::new(0, 255, 0));
        assert_eq!(e.kept.as_ref().map(Vec::len), Some(16));
    }

    #[test]
    fn keys_move_toggle_and_delete() {
        use SwatchKeyName::{Backspace, Delete, End, Enter, Home, Left, Right, Space};
        assert_eq!(swatch_key(Right, false, 2, 3), SwatchKey::Move(2));
        assert_eq!(swatch_key(Right, false, 0, 3), SwatchKey::Move(1));
        assert_eq!(swatch_key(Left, false, 0, 3), SwatchKey::Move(0));
        assert_eq!(swatch_key(Home, false, 2, 3), SwatchKey::Move(0));
        assert_eq!(swatch_key(End, false, 0, 3), SwatchKey::Move(2));
        assert_eq!(swatch_key(Enter, false, 1, 3), SwatchKey::Toggle(1));
        assert_eq!(swatch_key(Space, false, 1, 3), SwatchKey::Toggle(1));
        assert_eq!(swatch_key(Enter, true, 1, 3), SwatchKey::Delete(1));
        assert_eq!(swatch_key(Delete, false, 1, 3), SwatchKey::Delete(1));
        assert_eq!(swatch_key(Backspace, false, 1, 3), SwatchKey::Delete(1));
    }
}
