//! The Advanced block under the rows of the bar: Output with the Multicolour palette, the
//! toggles, Denoise, Upscale, Corners and Analyse.

use eframe::egui::{self, Align2, Color32, Key, Modifiers, Pos2, Rect, Stroke, vec2};
use tracyo_core::method::Method;
use tracyo_core::palette::{self, SwatchKey, SwatchKeyName};
use tracyo_core::settings;
use vectrace::Rgb8;

use crate::app::{App, Pipette};
use crate::bar::{BlockFlow, bar_id, block_flow, field_widget, plain};
use crate::controls::{self, CellPress, segmented, segmented_width};
use crate::icons::Icon;
use crate::theme;
use crate::widgets::{self, Line, Side};

const DENOISE: [&str; 3] = ["Off", "Low", "High"];
const UPSCALE: [&str; 2] = ["Auto", "Off"];
/// The label slot of Denoise and Upscale.
const LABEL_W: f32 = 64.0;
/// Width of a cell that holds a label and a pill (at least 150).
const MIN_CELL: f32 = 150.0;
/// Width of the Analyse button.
const ANALYSE_W: f32 = 75.0;
/// Width of the Colours field.
const COLOURS_W: f32 = 120.0;
/// Width of the four toggles (Invert, Drop border shapes, Recognise text, Recognise shapes) with
/// their gaps.
const TOGGLES_W: f32 = 4.0 * 28.0 + 3.0 * 4.0;
/// Padding of a segment of the Output pill.
const OUTPUT_PAD: f32 = 6.0;
const OUTPUT: [&str; 2] = ["Single colour", "Multicolour"];

/// What the Output row shows besides the pill.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RowShape {
    /// Multicolour: the Colours field and the palette.
    multi: bool,
    /// Pick colours: the picked colours are the palette, so no pipette and no swatches here.
    pick: bool,
    /// Swatches in the palette row.
    swatches: usize,
}

/// Where the parts of the Output row start, from its left edge: Colours, the palette pipette,
/// the first swatch cell and the toggles.
#[derive(Debug, Clone, Copy)]
struct RowPlaces {
    colours: f32,
    pipette: f32,
    swatches: f32,
    toggles: f32,
    width: f32,
}

impl RowShape {
    fn places(self, pill: f32) -> RowPlaces {
        let colours = pill + 4.0;
        let mut end = pill;
        let (mut pipette, mut swatches) = (0.0, 0.0);
        if self.multi {
            end = colours + COLOURS_W;
            if !self.pick {
                pipette = end + 4.0;
                end = pipette + 28.0;
                // The palette row has 2 px before it and -2 px after it.
                swatches = end + 2.0;
                end += 24.0 * self.swatches as f32;
            }
        }
        let toggles = end + 4.0;
        RowPlaces {
            colours,
            pipette,
            swatches,
            toggles,
            width: toggles + TOGGLES_W,
        }
    }
}

fn colour32(c: Rgb8) -> Color32 {
    Color32::from_rgb(c.r, c.g, c.b)
}

impl App {
    /// What the Output row shows now.
    fn row_shape(&self) -> RowShape {
        RowShape {
            multi: self.settings.multicolour,
            pick: Method::from_index(self.settings.method) == Method::PickColours,
            swatches: self.palette_strip().len(),
        }
    }

    /// Where the block's items sit when `inner` px are inside the bar's padding.
    pub(crate) fn block_layout(&self, ui: &egui::Ui, inner: f32) -> BlockFlow {
        let cell = |labels: &[&str]| (LABEL_W + segmented_width(ui, labels, 8.0)).max(MIN_CELL);
        let pill = segmented_width(ui, &OUTPUT, OUTPUT_PAD);
        let widths = [
            self.row_shape().places(pill).width,
            cell(&DENOISE),
            cell(&UPSCALE),
            MIN_CELL,
        ];
        block_flow(&widths, ANALYSE_W, inner - 4.0)
    }

    /// Paints and drives the block; `origin` is its content corner. Returns whether a value that
    /// traces changed (and whether it is being dragged).
    pub(crate) fn advanced_block(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        block: &BlockFlow,
        origin: Pos2,
        inner: f32,
    ) -> Option<bool> {
        // The divider: 4 px under the rows, full width of the content.
        let y = origin.y;
        ui.painter().line_segment(
            [
                Pos2::new(origin.x, y + 0.5),
                Pos2::new(origin.x + inner, y + 0.5),
            ],
            Stroke::new(1.0, theme::DIVIDER),
        );
        let content = origin + vec2(0.0, 1.0 + 4.0);
        let at = |p: Pos2, w: f32| Rect::from_min_size(content + p.to_vec2(), vec2(w, 28.0));

        let mut changed = self.output_row(ui, ctx, content + block.items[0].to_vec2());
        changed = self
            .denoise_and_upscale(ui, at(block.items[1], 0.0).min, at(block.items[2], 0.0).min)
            .or(changed);

        // Corners.
        if let Some((v, dragging)) = field_widget(
            ctx,
            ui,
            at(block.items[3], 150.0),
            "corners",
            &settings::CORNERS,
            f64::from(self.settings.corners),
            &mut self.fields[4],
        ) {
            // invariant: the field clamps to 0..=100.
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            {
                self.settings.corners = v.round().clamp(0.0, 100.0) as u8;
            }
            changed = Some(dragging);
        }

        // Analyse.
        let analyse =
            controls::accent_button(ui, at(block.tail, ANALYSE_W), bar_id("analyse"), "Analyse");
        widgets::release_focus_after_mouse(ctx, &analyse);
        if analyse.clicked() {
            self.analyse_again(ctx);
        }
        changed
    }

    /// Denoise and Upscale: a 12 px label in a 64 px slot, then the pill.
    fn denoise_and_upscale(
        &mut self,
        ui: &mut egui::Ui,
        denoise: Pos2,
        upscale: Pos2,
    ) -> Option<bool> {
        let mut changed = None;
        widgets::text(
            ui,
            Pos2::new(denoise.x, denoise.y + 14.0),
            Align2::LEFT_CENTER,
            "Denoise",
            theme::text12(),
            theme::TOOLBAR_ICON,
        );
        if let Some(i) = segmented(
            ui,
            denoise + vec2(LABEL_W, 0.0),
            bar_id("denoise"),
            "Denoise",
            &DENOISE,
            usize::from(self.settings.denoise),
            8.0,
        ) {
            // invariant: three choices.
            self.settings.denoise = u8::try_from(i).unwrap_or(0);
            changed = Some(false);
        }
        widgets::text(
            ui,
            Pos2::new(upscale.x, upscale.y + 14.0),
            Align2::LEFT_CENTER,
            "Upscale",
            theme::text12(),
            theme::TOOLBAR_ICON,
        );
        if let Some(i) = segmented(
            ui,
            upscale + vec2(LABEL_W, 0.0),
            bar_id("upscale"),
            "Upscale",
            &UPSCALE,
            usize::from(!self.settings.upscale_auto),
            8.0,
        ) {
            self.settings.upscale_auto = i == 0;
            changed = Some(false);
        }
        changed
    }

    /// The Output row: the pill, with Multicolour the Colours field, the palette pipette and the
    /// palette, then Invert and Drop border shapes.
    fn output_row(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, origin: Pos2) -> Option<bool> {
        let mut changed = None;
        let shape = self.row_shape();
        let pill = segmented_width(ui, &OUTPUT, OUTPUT_PAD);
        let places = shape.places(pill);
        let top = origin;
        if let Some(i) = segmented(
            ui,
            top,
            bar_id("output"),
            "Output",
            &OUTPUT,
            usize::from(shape.multi),
            OUTPUT_PAD,
        ) {
            self.settings.multicolour = i == 1;
            self.config.save(&self.settings);
            changed = Some(false);
        }
        if shape.multi {
            changed = self
                .colours_field(
                    ui,
                    ctx,
                    Rect::from_min_size(top + vec2(places.colours, 0.0), vec2(COLOURS_W, 28.0)),
                    shape.pick,
                )
                .or(changed);
            if !shape.pick {
                self.pipette_button(
                    ui,
                    ctx,
                    Rect::from_min_size(top + vec2(places.pipette, 0.0), vec2(28.0, 28.0)),
                    "Pick palette colour from the image",
                    Pipette::Palette,
                );
                changed = self
                    .palette_row(ui, ctx, top + vec2(places.swatches, 0.0))
                    .or(changed);
            }
        }
        self.toggles(ui, ctx, top + vec2(places.toggles, 0.0))
            .or(changed)
    }

    /// The Colours field: Auto at the lowest stop with the found count, 2 to 16; with Pick
    /// colours dimmed, showing the number of picked colours.
    fn colours_field(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        rect: Rect,
        pick: bool,
    ) -> Option<bool> {
        let picked = self.doc.as_ref().map_or(0, |d| d.values.swatches.len());
        let found = self.result.as_ref().map(|r| r.output.palette.len());
        let (position, shown) = if pick {
            (picked.max(1) as f64, picked.to_string())
        } else {
            let text = match (self.settings.colours, found) {
                (0, Some(n)) => format!("Auto {n}"),
                (0, None) => "Auto".to_owned(),
                (n, _) => n.to_string(),
            };
            (self.settings.colours_position(), text)
        };
        self.fields[5].shown = Some(shown);
        let state = &mut self.fields[5];
        let event = ui
            .scope(|ui| {
                if pick {
                    ui.disable();
                }
                field_widget(
                    ctx,
                    ui,
                    rect,
                    "colours",
                    &settings::COLOURS,
                    position,
                    state,
                )
            })
            .inner;
        let (value, dragging) = event?;
        if pick {
            return None;
        }
        let before = self.settings.colours;
        self.settings.set_colours_position(value);
        if self.settings.colours == before {
            return None;
        }
        if let Some(doc) = &mut self.doc {
            doc.values.palette.forget_kept();
        }
        Some(dragging)
    }

    /// The palette's swatches, one Tab stop with roving arrows. A press on the x deletes a
    /// colour, Ctrl+click excludes it.
    fn palette_row(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        origin: Pos2,
    ) -> Option<bool> {
        let strip = self.palette_strip();
        let stale = self.progress.result_is_old();
        let stop = self.palette_stop.min(strip.len().saturating_sub(1));
        let mut action = None;
        ui.scope(|ui| {
            for (i, entry) in strip.iter().enumerate() {
                let cell =
                    Rect::from_min_size(origin + vec2(24.0 * i as f32, 0.0), vec2(24.0, 28.0));
                let id = bar_id("palette").with(i);
                let excluded = self
                    .doc
                    .as_ref()
                    .is_some_and(|d| d.values.palette.is_excluded(entry.colour));
                let name = palette::swatch_label(entry.colour, excluded);
                let (resp, press) = controls::palette_cell(
                    ui,
                    &mut self.icons,
                    cell,
                    id,
                    colour32(entry.colour),
                    &name,
                    i == stop,
                    excluded,
                    stale,
                );
                widgets::tooltip(ctx, &resp, Side::Below, &swatch_tip(entry.colour));
                widgets::reset_tooltip(ctx, &resp);
                match press {
                    CellPress::Delete => action = Some((SwatchKey::Delete(i), true)),
                    CellPress::ToggleExclude => action = Some((SwatchKey::Toggle(i), true)),
                    CellPress::None => {}
                }
                if press != CellPress::None {
                    widgets::release_focus_after_mouse(ctx, &resp);
                }
                if resp.has_focus()
                    && let Some(key) = swatch_key(ui, i, strip.len())
                {
                    action = Some((key, false));
                }
            }
        });
        let (key, _by_mouse) = action?;
        let colours: Vec<Rgb8> = strip.iter().map(|p| p.colour).collect();
        match key {
            SwatchKey::Move(to) => {
                self.palette_stop = to;
                ctx.memory_mut(|m| m.request_focus(bar_id("palette").with(to)));
                None
            }
            SwatchKey::Toggle(i) => {
                self.palette_stop = i;
                self.doc.as_mut()?.values.palette.toggle_exclude(colours[i]);
                Some(false)
            }
            SwatchKey::Delete(i) => {
                self.doc
                    .as_mut()?
                    .values
                    .palette
                    .delete(&colours, colours[i]);
                let next = i.min(colours.len().saturating_sub(2));
                self.palette_stop = next;
                if colours.len() > 1 {
                    ctx.memory_mut(|m| m.request_focus(bar_id("palette").with(next)));
                }
                Some(false)
            }
        }
    }

    /// Invert, Drop border shapes, Recognise text and Recognise shapes.
    fn toggles(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, origin: Pos2) -> Option<bool> {
        let mut changed = None;
        let (invert, drop_border) = self
            .doc
            .as_ref()
            .map_or((false, false), |d| (d.values.invert, d.values.drop_border));
        for (k, (name, icon, on)) in [
            ("Invert", Icon::Contrast, invert),
            ("Drop border shapes", Icon::ImageMinus, drop_border),
            (
                "Recognise text",
                Icon::ScanText,
                self.settings.recognise_text,
            ),
            (
                "Recognise shapes",
                Icon::Shapes,
                self.settings.recognise_shapes,
            ),
        ]
        .into_iter()
        .enumerate()
        {
            let rect = Rect::from_min_size(origin + vec2(32.0 * k as f32, 0.0), vec2(28.0, 28.0));
            let resp = widgets::icon_button(
                ui,
                &mut self.icons,
                rect,
                bar_id("toggle").with(name),
                icon,
                name,
                on,
            );
            widgets::tooltip(ctx, &resp, Side::Below, &plain(name));
            widgets::reset_tooltip(ctx, &resp);
            widgets::release_focus_after_mouse(ctx, &resp);
            if resp.clicked() {
                match k {
                    0 | 1 => {
                        let Some(doc) = &mut self.doc else { continue };
                        if k == 0 {
                            doc.values.invert = !doc.values.invert;
                        } else {
                            doc.values.drop_border = !doc.values.drop_border;
                        }
                    }
                    2 => self.settings.recognise_text = !self.settings.recognise_text,
                    _ => self.settings.recognise_shapes = !self.settings.recognise_shapes,
                }
                if k >= 2 {
                    self.config.save(&self.settings);
                }
                changed = Some(false);
            }
        }
        changed
    }
}

/// The tooltip of a palette swatch: its colour code and name, and small how to exclude it.
fn swatch_tip(colour: Rgb8) -> Vec<Line> {
    let how = if cfg!(target_os = "macos") {
        "Cmd+click: exclude"
    } else {
        "Ctrl+click: exclude"
    };
    vec![
        Line {
            text: palette::swatch_tooltip(colour),
            font: theme::text12(),
            colour: theme::FOREGROUND,
        },
        Line {
            text: how.to_owned(),
            font: theme::text10(),
            colour: theme::POPOVER_SECOND,
        },
    ]
}

/// What a key does on the focused palette swatch `index` of `count`; the roving arrows are
/// kept by the swatch.
fn swatch_key(ui: &mut egui::Ui, index: usize, count: usize) -> Option<SwatchKey> {
    ui.memory_mut(|m| {
        if let Some(id) = m.focused() {
            m.set_focus_lock_filter(
                id,
                egui::EventFilter {
                    horizontal_arrows: true,
                    ..Default::default()
                },
            );
        }
    });
    let keys = [
        (Key::ArrowRight, SwatchKeyName::Right),
        (Key::ArrowLeft, SwatchKeyName::Left),
        (Key::Home, SwatchKeyName::Home),
        (Key::End, SwatchKeyName::End),
        (Key::Space, SwatchKeyName::Space),
        (Key::Delete, SwatchKeyName::Delete),
        (Key::Backspace, SwatchKeyName::Backspace),
    ];
    ui.input_mut(|i| {
        if i.consume_key(Modifiers::SHIFT, Key::Enter) {
            return Some(palette::swatch_key(
                SwatchKeyName::Enter,
                true,
                index,
                count,
            ));
        }
        if i.consume_key(Modifiers::NONE, Key::Enter) {
            return Some(palette::swatch_key(
                SwatchKeyName::Enter,
                false,
                index,
                count,
            ));
        }
        keys.iter()
            .find(|(key, _)| i.consume_key(Modifiers::NONE, *key))
            .map(|(_, name)| palette::swatch_key(*name, false, index, count))
    })
}
