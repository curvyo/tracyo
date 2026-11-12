//! Shared helpers of the core tests: the spec's fixture pictures P1 and P2, a trace of a picture
//! and the areas of outlines.

#![allow(dead_code, clippy::unwrap_used)]

use kurbo::{BezPath, Shape};
use tracyo_core::job::Session;
use tracyo_core::picture::Picture;
use tracyo_core::request::{self, TraceRequest};
use tracyo_core::settings::{self, Settings};
use vectrace::PixelMap;

/// The pixel map of a full picture.
pub fn full_map(width: u32) -> PixelMap {
    request::pixel_map(width, width)
}

fn paint(rgba: &mut [u8], width: u32, test: impl Fn(f64, f64) -> bool, colour: [u8; 3]) {
    let height = rgba.len() / 4 / width as usize;
    for y in 0..height {
        for x in 0..width as usize {
            #[allow(clippy::cast_precision_loss)]
            if test(x as f64 + 0.5, y as f64 + 0.5) {
                let i = (y * width as usize + x) * 4;
                rgba[i..i + 3].copy_from_slice(&colour);
            }
        }
    }
}

/// P1 of specification 0002: 1000 x 1000 px, white, a black disc of 300 px at (250, 500), a black
/// square of 300 px at (750, 500), a black bar y = 480 to 520 across the width, a black disc of
/// 100 px at (150, 150) and a grey (#808080) square of 100 px at (900, 100).
pub fn p1() -> Picture {
    let mut rgba = vec![255; 1000 * 1000 * 4];
    let disc = |cx: f64, cy: f64, r: f64| {
        move |x: f64, y: f64| (x - cx).powi(2) + (y - cy).powi(2) <= r * r
    };
    let rect = |x0: f64, y0: f64, x1: f64, y1: f64| {
        move |x: f64, y: f64| x >= x0 && x < x1 && y >= y0 && y < y1
    };
    paint(&mut rgba, 1000, disc(250.0, 500.0, 150.0), [0, 0, 0]);
    paint(&mut rgba, 1000, rect(600.0, 350.0, 900.0, 650.0), [0, 0, 0]);
    paint(&mut rgba, 1000, rect(0.0, 480.0, 1000.0, 520.0), [0, 0, 0]);
    paint(&mut rgba, 1000, disc(150.0, 150.0, 50.0), [0, 0, 0]);
    paint(
        &mut rgba,
        1000,
        rect(850.0, 50.0, 950.0, 150.0),
        [0x80, 0x80, 0x80],
    );
    Picture {
        width: 1000,
        height: 1000,
        rgba,
    }
}

/// P2: P1 with the white of the right half (x >= 500) replaced by #D8D8D8.
pub fn p2() -> Picture {
    let mut picture = p1();
    for y in 0..1000_usize {
        for x in 500..1000_usize {
            let i = (y * 1000 + x) * 4;
            if picture.rgba[i..i + 3] == [255, 255, 255] {
                picture.rgba[i..i + 3].copy_from_slice(&[0xD8, 0xD8, 0xD8]);
            }
        }
    }
    picture
}

/// A Brightness, Single colour request at Threshold `percent` with the preset values.
pub fn request_at(percent: f64) -> TraceRequest {
    TraceRequest {
        threshold: settings::luma_of_percent(percent),
        settings: Settings::default(),
        selection: vectrace::Selection::Brightness,
        invert: false,
        drop_border: false,
        colours: None,
        reduced: false,
        shapes: false,
        lines: Vec::new(),
        clip: None,
    }
}

/// The result of `request` on `picture`.
pub fn trace(picture: &Picture, request: &TraceRequest) -> vectrace::wire::Output {
    let mut session = Session::new(picture.clone());
    vectrace::wire::read(&session.run(request, &mut |_| {})).unwrap()
}

/// The sum of the signed areas of `outlines` (holes wound against their outer outline).
pub fn net_area(outlines: &[BezPath]) -> f64 {
    outlines.iter().map(Shape::area).sum()
}
