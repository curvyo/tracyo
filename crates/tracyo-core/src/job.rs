//! Runs one trace of a kept picture with vectrace and reports whole percents.

use std::ops::ControlFlow;

use vectrace::job::{Job, Mode};
use vectrace::ocr::{OcrError, OcrLine, OcrModel, read_text};
use vectrace::{ColourParams, Raster, TraceParams};

use crate::picture::{Picture, REDUCED_SIDE};
use crate::request::{TraceRequest, pixel_map};

/// The numbers of a Multicolour result with no outline and no palette colour (every colour of the
/// palette was deleted): nothing is traced.
#[must_use]
pub fn empty_colour_wire() -> Vec<f64> {
    // The header (status Traced, no paths, nodes, specks, pixels, whole picture, outlines), then
    // the palette, layer and joined-region counts.
    vec![0.0; 10]
}

/// The picture a worker holds and its lazily made reduced copy.
#[derive(Debug)]
pub struct Session {
    full: Picture,
    reduced: Option<Picture>,
}

impl Session {
    /// A session for `picture`.
    #[must_use]
    pub fn new(picture: Picture) -> Self {
        Self {
            full: picture,
            reduced: None,
        }
    }

    /// The text lines of the full picture read with `model`, top to bottom; `on_percent` gets each
    /// higher whole percent.
    ///
    /// # Errors
    /// The reader's error for a picture or a model it cannot use.
    pub fn read_text(
        &self,
        model: &OcrModel,
        on_percent: &mut dyn FnMut(u8),
    ) -> Result<Vec<OcrLine>, OcrError> {
        read_text(
            model,
            &self.full.rgba,
            self.full.width,
            self.full.height,
            &mut |percent| {
                on_percent(percent);
                ControlFlow::Continue(())
            },
        )
    }

    /// The trace of `request` as vectrace's numbers; `on_percent` gets each higher whole percent.
    pub fn run(&mut self, request: &TraceRequest, on_percent: &mut dyn FnMut(u8)) -> Vec<f64> {
        let full_width = self.full.width;
        let picture = if request.reduced {
            let full = &self.full;
            &*self
                .reduced
                .get_or_insert_with(|| full.reduced(REDUCED_SIDE))
        } else {
            &self.full
        };
        let params = TraceParams {
            selection: request.selection,
            invert: request.invert,
            ignore_background: request.drop_border,
            ..request.settings.trace_params(request.threshold)
        };
        let colour_params = request.colours.as_ref().map(|spec| ColourParams {
            trace: params,
            colours: spec.count,
            fixed: spec.fixed.clone(),
            excluded: spec.excluded.clone(),
        });
        // The lines are in the full picture's pixels; the reduced copy has fewer of them.
        let factor = f64::from(picture.width) / f64::from(full_width.max(1));
        let lines: Vec<OcrLine> = request.lines.iter().map(|l| l.scaled(factor)).collect();
        let job = Job {
            raster: Raster {
                width: picture.width,
                height: picture.height,
                rgba: &picture.rgba,
            },
            map: pixel_map(full_width, picture.width),
            mode: colour_params
                .as_ref()
                .map_or(Mode::Mono(&params), Mode::Colours),
            lines: &lines,
            mask: None,
            shapes: request.shapes,
        };
        vectrace::job::run(&job, on_percent)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::palette::ColourSpec;
    use crate::settings::Settings;
    use vectrace::wire;

    /// A white 40 x 40 picture with a black 20 x 20 square at (10, 10).
    fn square() -> Picture {
        let mut rgba = vec![255; 40 * 40 * 4];
        for y in 10..30 {
            for x in 10..30 {
                let i = (y * 40 + x) * 4;
                rgba[i..i + 3].fill(0);
            }
        }
        Picture {
            width: 40,
            height: 40,
            rgba,
        }
    }

    fn request(threshold: u8, reduced: bool) -> TraceRequest {
        TraceRequest {
            threshold,
            settings: Settings::default(),
            selection: vectrace::Selection::Brightness,
            invert: false,
            drop_border: false,
            colours: None,
            reduced,
            shapes: false,
            lines: Vec::new(),
        }
    }

    #[test]
    fn a_black_square_traces_to_one_outline_in_millimetres() {
        let mut session = Session::new(square());
        let mut percents = Vec::new();
        let numbers = session.run(&request(128, false), &mut |p| percents.push(p));
        let out = wire::read(&numbers).unwrap();
        assert_eq!(out.paths, 1);
        let bounds = kurbo::Shape::bounding_box(&out.outlines[0]);
        let mm = 25.4 / 96.0;
        assert!((bounds.x0 - 10.0 * mm).abs() < 0.05 && (bounds.x1 - 30.0 * mm).abs() < 0.05);
        assert!(percents.windows(2).all(|w| w[0] < w[1]));
        assert_eq!(percents.last(), Some(&100));
    }

    #[test]
    fn a_threshold_of_zero_finds_nothing() {
        let mut session = Session::new(square());
        let out = wire::read(&session.run(&request(0, false), &mut |_| {})).unwrap();
        assert_eq!(out.paths, 0);
    }

    #[test]
    fn the_reduced_copy_lands_in_the_same_millimetres() {
        let mut big = Picture {
            width: 2000,
            height: 2000,
            rgba: vec![255; 2000 * 2000 * 4],
        };
        for y in 500..1500 {
            for x in 500..1500 {
                let i = (y * 2000 + x) * 4;
                big.rgba[i..i + 3].fill(0);
            }
        }
        let mut session = Session::new(big);
        let out = wire::read(&session.run(&request(128, true), &mut |_| {})).unwrap();
        let bounds = kurbo::Shape::bounding_box(&out.outlines[0]);
        let mm = 25.4 / 96.0;
        assert!((bounds.x0 - 500.0 * mm).abs() < 0.5, "x0 = {}", bounds.x0);
        assert!((bounds.x1 - 1500.0 * mm).abs() < 0.5, "x1 = {}", bounds.x1);
    }

    /// A white 40 x 40 picture with a red 20 x 20 square at (10, 10) and a blue 6 x 6 one
    /// touching the top-left corner.
    fn colours() -> Picture {
        let mut rgba = vec![255; 40 * 40 * 4];
        let mut paint = |x0: usize, x1: usize, y0: usize, y1: usize, c: [u8; 3]| {
            for y in y0..y1 {
                for x in x0..x1 {
                    let i = (y * 40 + x) * 4;
                    rgba[i..i + 3].copy_from_slice(&c);
                }
            }
        };
        paint(10, 30, 10, 30, [255, 0, 0]);
        paint(0, 6, 0, 6, [0, 0, 255]);
        Picture {
            width: 40,
            height: 40,
            rgba,
        }
    }

    fn output_of(selection: vectrace::Selection, invert: bool, drop_border: bool) -> wire::Output {
        let mut session = Session::new(colours());
        let request = TraceRequest {
            selection,
            invert,
            drop_border,
            ..request(128, false)
        };
        wire::read(&session.run(&request, &mut |_| {})).unwrap()
    }

    fn paths_of(selection: vectrace::Selection, invert: bool, drop_border: bool) -> usize {
        output_of(selection, invert, drop_border).paths
    }

    #[test]
    fn remove_background_traces_everything_unlike_the_background() {
        let white = Some(vectrace::Rgb8::new(255, 255, 255));
        let selection = vectrace::Selection::RemoveBackground {
            background: white,
            tolerance: 40,
        };
        assert_eq!(paths_of(selection, false, false), 2);
        assert_eq!(
            paths_of(selection, false, true),
            1,
            "the blue one touches the border"
        );
    }

    #[test]
    fn pick_colours_traces_only_the_picked_colour() {
        let selection = vectrace::Selection::PickColours {
            swatches: vectrace::Swatches::from_slice(&[vectrace::Rgb8::new(255, 0, 0)]),
            background: Some(vectrace::Rgb8::new(255, 255, 255)),
            tolerance: 19,
        };
        assert_eq!(paths_of(selection, false, false), 1);
        let none = vectrace::Selection::PickColours {
            swatches: vectrace::Swatches::from_slice(&[]),
            background: None,
            tolerance: 19,
        };
        assert_eq!(paths_of(none, false, false), 0);
    }

    #[test]
    fn invert_traces_what_brightness_leaves_out() {
        let plain = output_of(vectrace::Selection::Brightness, false, false);
        let inverted = output_of(vectrace::Selection::Brightness, true, false);
        assert_eq!(
            plain.foreground_pixels + inverted.foreground_pixels,
            40 * 40
        );
    }

    fn multicolour(spec: ColourSpec) -> wire::Output {
        let mut session = Session::new(colours());
        let request = TraceRequest {
            selection: vectrace::Selection::RemoveBackground {
                background: Some(vectrace::Rgb8::new(255, 255, 255)),
                tolerance: 40,
            },
            colours: Some(spec),
            ..request(128, false)
        };
        wire::read(&session.run(&request, &mut |_| {})).unwrap()
    }

    fn auto() -> ColourSpec {
        ColourSpec {
            count: None,
            fixed: Vec::new(),
            excluded: Vec::new(),
        }
    }

    #[test]
    fn multicolour_gives_one_layer_per_colour_largest_first() {
        let out = multicolour(auto());
        let colours: Vec<_> = out.layers.iter().map(|l| l.colour).collect();
        assert_eq!(
            colours,
            [
                vectrace::Rgb8::new(255, 0, 0),
                vectrace::Rgb8::new(0, 0, 255)
            ]
        );
        assert_eq!(out.palette.len(), 2);
        let held: usize = out.layers.iter().map(|l| l.outlines).sum();
        assert_eq!(held, out.outlines.len());
    }

    #[test]
    fn an_excluded_colour_has_no_layer_but_stays_in_the_palette() {
        let out = multicolour(ColourSpec {
            excluded: vec![vectrace::Rgb8::new(0, 0, 255)],
            ..auto()
        });
        assert_eq!(out.layers.len(), 1);
        assert_eq!(out.layers[0].colour, vectrace::Rgb8::new(255, 0, 0));
        assert_eq!(out.palette.len(), 2);
    }

    #[test]
    fn a_kept_palette_of_one_colour_traces_everything_in_it() {
        let out = multicolour(ColourSpec {
            count: Some(1),
            fixed: vec![vectrace::Rgb8::new(255, 0, 0)],
            excluded: Vec::new(),
        });
        assert_eq!(out.palette.len(), 1);
        assert_eq!(out.layers.len(), 1);
    }

    #[test]
    fn the_empty_wire_reads_as_a_traced_result_without_paths_or_colours() {
        let out = wire::read(&empty_colour_wire()).unwrap();
        assert_eq!(out.status, wire::Status::Traced);
        assert_eq!((out.paths, out.palette.len(), out.layers.len()), (0, 0, 0));
    }

    fn line_around(x0: f64, y0: f64, x1: f64, y1: f64) -> OcrLine {
        OcrLine {
            quad: [[x0, y0], [x1, y0], [x1, y1], [x0, y1]],
            text: "AB".to_owned(),
            confidence: vec![0.95, 0.95],
            positions: Vec::new(),
        }
    }

    #[test]
    fn a_read_line_is_set_as_text_and_left_out_of_the_paths() {
        let mut session = Session::new(square());
        let plain = wire::read(&session.run(&request(128, false), &mut |_| {})).unwrap();
        assert_eq!((plain.paths, plain.texts.lines.len()), (1, 0));
        let with = TraceRequest {
            lines: vec![line_around(8.0, 8.0, 32.0, 32.0)],
            ..request(128, false)
        };
        let out = wire::read(&session.run(&with, &mut |_| {})).unwrap();
        assert_eq!((out.texts.found, out.texts.lines.len()), (1, 1));
        assert_eq!(out.texts.lines[0].text, "AB");
        assert_eq!(out.paths, 0, "the text's pixels are not in the main trace");
    }

    #[test]
    fn lines_of_the_full_picture_land_in_the_same_millimetres_on_the_reduced_copy() {
        let mut big = Picture {
            width: 2000,
            height: 2000,
            rgba: vec![255; 2000 * 2000 * 4],
        };
        for y in 500..1500 {
            for x in 500..1500 {
                let i = (y * 2000 + x) * 4;
                big.rgba[i..i + 3].fill(0);
            }
        }
        let mut session = Session::new(big);
        let lines = vec![line_around(450.0, 450.0, 1550.0, 1550.0)];
        let mm = 25.4 / 96.0;
        for reduced in [false, true] {
            let r = TraceRequest {
                lines: lines.clone(),
                ..request(128, reduced)
            };
            let out = wire::read(&session.run(&r, &mut |_| {})).unwrap();
            assert_eq!(out.texts.lines.len(), 1, "reduced: {reduced}");
            let bounds = kurbo::Shape::bounding_box(&out.texts.lines[0].outlines[0]);
            assert!((bounds.x0 - 500.0 * mm).abs() < 0.5, "x0 = {}", bounds.x0);
            assert!((bounds.x1 - 1500.0 * mm).abs() < 0.5, "x1 = {}", bounds.x1);
        }
    }

    /// A white 80 x 80 picture with a black 40 x 30 rectangle.
    fn rectangle() -> Picture {
        let mut rgba = vec![255; 80 * 80 * 4];
        for y in 20..50 {
            for x in 10..50 {
                let i = (y * 80 + x) * 4;
                rgba[i..i + 3].fill(0);
            }
        }
        Picture {
            width: 80,
            height: 80,
            rgba,
        }
    }

    #[test]
    fn shapes_are_recognised_only_when_asked() {
        let mut session = Session::new(rectangle());
        let off = wire::read(&session.run(&request(128, false), &mut |_| {})).unwrap();
        assert_eq!(off.shapes.len(), 0);
        let on = TraceRequest {
            shapes: true,
            ..request(128, false)
        };
        let out = wire::read(&session.run(&on, &mut |_| {})).unwrap();
        assert_eq!(out.shapes.len(), 1);
        assert!(matches!(
            out.shapes[0].shapes[0],
            Some((vectrace::shapes::Primitive::Rect { .. }, _))
        ));
    }
}
