//! A joined result as vectrace's numbers, so it crosses the worker's pipe like a plain trace.
//!
//! The layout is the one `vectrace::wire` documents; the reader there is the test of this writer
//! (`vectrace::wire::read` of the numbers gives the result back).

use kurbo::{BezPath, PathEl};
use vectrace::shapes::{Band, Join, Primitive, ShapeRegion};
use vectrace::wire::{Output, Status};

/// The number that opens the unclipped count.
const UNCLIPPED_MARK: f64 = -5.0;
/// The number that opens the text section.
const TEXT_MARK: f64 = -1.0;
/// The number that opens the lines' fills.
const FILL_MARK: f64 = -2.0;
/// The number that opens the excluded lines.
const EXCLUDED_MARK: f64 = -3.0;
/// The number that opens the shape section.
const SHAPE_MARK: f64 = -4.0;

/// Numbers per outline's primitive in the shape section: the kind and six parameters.
const SHAPE_NUMBERS: usize = 7;

/// The numbers of `output`.
#[must_use]
#[allow(clippy::cast_precision_loss)] // counts far below 2^52
pub fn to_numbers(output: &Output) -> Vec<f64> {
    let mut wire = vec![
        match output.status {
            Status::Traced => 0.0,
            Status::TooDetailed => 1.0,
            Status::Invalid => 2.0,
        },
        output.paths as f64,
        output.nodes as f64,
        output.specks_removed as f64,
        output.foreground_pixels as f64,
        f64::from(u8::from(output.covers_whole_image)),
        output.outlines.len() as f64,
    ];
    for outline in &output.outlines {
        write_outline(&mut wire, outline);
    }
    if output.unclipped > 0 {
        wire.extend([UNCLIPPED_MARK, output.unclipped as f64]);
    }
    if output.texts.found > 0 || !output.texts.lines.is_empty() {
        write_texts(&mut wire, output);
    }
    if !output.shapes.is_empty() {
        wire.extend([SHAPE_MARK, output.shapes.len() as f64]);
        for region in &output.shapes {
            write_region(&mut wire, region);
        }
    }
    if !output.palette.is_empty() || !output.layers.is_empty() || output.joined > 0 {
        wire.push(output.palette.len() as f64);
        for p in &output.palette {
            wire.extend([
                f64::from(p.colour.r),
                f64::from(p.colour.g),
                f64::from(p.colour.b),
                f64::from(u8::from(p.fixed)),
                p.pixels as f64,
            ]);
        }
        wire.push(output.layers.len() as f64);
        for l in &output.layers {
            wire.extend([
                f64::from(l.colour.r),
                f64::from(l.colour.g),
                f64::from(l.colour.b),
                l.outlines as f64,
            ]);
        }
        wire.push(output.joined as f64);
    }
    wire
}

#[allow(clippy::cast_precision_loss)]
fn write_outline(wire: &mut Vec<f64>, outline: &BezPath) {
    wire.push(outline.elements().len() as f64);
    for element in outline.elements() {
        match *element {
            PathEl::MoveTo(p) => wire.extend([0.0, p.x, p.y]),
            // The reader knows no quadratic segment and the trace never makes one: it is a line.
            PathEl::LineTo(p) | PathEl::QuadTo(_, p) => wire.extend([1.0, p.x, p.y]),
            PathEl::CurveTo(a, b, p) => wire.extend([3.0, a.x, a.y, b.x, b.y, p.x, p.y]),
            PathEl::ClosePath => wire.push(4.0),
        }
    }
}

#[allow(clippy::cast_precision_loss)]
fn write_texts(wire: &mut Vec<f64>, output: &Output) {
    let texts = &output.texts;
    wire.extend([TEXT_MARK, texts.found as f64, texts.lines.len() as f64]);
    for line in &texts.lines {
        let chars: Vec<char> = line.text.chars().collect();
        wire.push(chars.len() as f64);
        wire.extend(chars.iter().map(|&c| f64::from(u32::from(c))));
        wire.extend([
            line.direction.x,
            line.direction.y,
            line.masked as f64,
            f64::from(u8::from(line.plausible)),
            line.pixel.as_mm(),
            line.positions.len() as f64,
        ]);
        wire.extend(line.positions.iter().map(|p| p.as_mm()));
        wire.push(line.outlines.len() as f64);
        for outline in &line.outlines {
            write_outline(wire, outline);
        }
    }
    if texts.lines.iter().any(|l| l.fill.is_some()) {
        wire.push(FILL_MARK);
        for line in &texts.lines {
            let c = line.fill.unwrap_or(vectrace::Rgb8::new(0, 0, 0));
            wire.extend([f64::from(c.r), f64::from(c.g), f64::from(c.b)]);
        }
        let excluded: Vec<f64> = texts
            .lines
            .iter()
            .enumerate()
            .filter(|(_, l)| l.excluded)
            .map(|(k, _)| k as f64)
            .collect();
        if !excluded.is_empty() {
            wire.extend([EXCLUDED_MARK, excluded.len() as f64]);
            wire.extend(excluded);
        }
    }
}

#[allow(clippy::cast_precision_loss)]
fn write_region(wire: &mut Vec<f64>, region: &ShapeRegion) {
    wire.push(region.outlines.len() as f64);
    wire.extend(region.outlines.iter().map(|&i| i as f64));
    for shape in &region.shapes {
        wire.extend(shape_numbers(shape.as_ref()));
    }
    match (&region.band, &region.bands[..]) {
        (Some(band), _) => write_band(wire, band),
        (None, []) => wire.push(0.0),
        (None, bands) => {
            wire.push(-(bands.len() as f64));
            for band in bands {
                write_band(wire, band);
            }
        }
    }
}

fn write_band(wire: &mut Vec<f64>, band: &Band) {
    wire.push(band.width.as_mm());
    wire.push(match band.join {
        Join::Miter => 0.0,
        Join::Bevel => 1.0,
        Join::Round => 2.0,
    });
    wire.extend(shape_numbers(Some(&(band.primitive, band.rotation))));
}

fn shape_numbers(shape: Option<&(Primitive, vectrace::Angle)>) -> [f64; SHAPE_NUMBERS] {
    let Some((shape, rotation)) = shape else {
        return [0.0; SHAPE_NUMBERS];
    };
    let turn = rotation.as_radians();
    match shape {
        Primitive::Rect {
            origin,
            width,
            height,
            corner_radius,
        } => {
            let (w, h) = (width.as_mm(), height.as_mm());
            [
                1.0,
                origin.x + w / 2.0,
                origin.y + h / 2.0,
                w,
                h,
                corner_radius.as_mm(),
                turn,
            ]
        }
        Primitive::Ellipse { centre, rx, ry } => {
            [2.0, centre.x, centre.y, rx.as_mm(), ry.as_mm(), turn, 0.0]
        }
        Primitive::Polygon {
            centre,
            radius,
            angle,
            points,
        } => [
            3.0,
            centre.x,
            centre.y,
            radius.as_mm(),
            angle.as_radians(),
            f64::from(*points),
            0.0,
        ],
        Primitive::Star {
            centre,
            radius,
            angle,
            points,
            inner_ratio,
        } => [
            4.0,
            centre.x,
            centre.y,
            radius.as_mm(),
            angle.as_radians(),
            f64::from(*points),
            *inner_ratio,
        ],
    }
}
