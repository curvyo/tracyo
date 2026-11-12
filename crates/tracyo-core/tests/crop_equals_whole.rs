//! A part traced from the crop of its clip equals the whole-picture trace cut to the clip
//! (criterion 25's prerequisite): within 0.1 % of the area, for settings that may crop.

#![allow(clippy::unwrap_used)]

mod common;

use i_overlay::core::overlay_rule::OverlayRule;
use kurbo::BezPath;
use tracyo_core::clip::Clip;
use tracyo_core::crop::crop_box;
use tracyo_core::grid;
use tracyo_core::regions::{MaskMode, PxRect, Region};
use tracyo_core::request::TraceRequest;
use vectrace::mask::Mask;

use common::{full_map, p1, p2, request_at, trace};

fn rings(outlines: &[BezPath]) -> Vec<grid::Path64> {
    outlines
        .iter()
        .flat_map(|p| grid::flatten_snap(p).unwrap())
        .collect()
}

fn clip(rect: PxRect) -> Clip {
    Clip::of_regions(
        &[Region {
            rect,
            subtract: false,
        }],
        MaskMode::Inside,
        full_map(1000),
    )
    .unwrap()
    .unwrap()
}

fn check(label: &str, picture: &tracyo_core::picture::Picture, request: &TraceRequest, c: &Clip) {
    assert!(
        crop_box(Some(c), request, 1000, 1000, full_map(1000)).is_some(),
        "{label}: the clip is cropped"
    );
    let mut cropped = request.clone();
    cropped.clip = Some(c.clone());
    let from_crop = trace(picture, &cropped);
    let whole = trace(picture, request);
    let cut = c.clip(&whole.outlines).outlines;
    let xor = grid::run(OverlayRule::Xor, &rings(&from_crop.outlines), &rings(&cut));
    let all = grid::area_mm2(&rings(&cut)).abs();
    println!("{label}: xor {} of {all}", grid::area_mm2(&xor).abs());
    assert!(
        grid::area_mm2(&xor).abs() <= 0.001 * all,
        "{label}: crop and whole differ"
    );
}

#[test]
fn a_cropped_part_equals_the_whole_trace_cut_to_its_clip() {
    let c = clip(PxRect::new(100, 300, 700, 700));
    for (label, smooth, ignore) in [
        ("default", 50, 0.1),
        ("smooth 20", 20, 0.1),
        ("smooth 90", 90, 0.1),
        ("ignore 2 mm", 50, 2.0),
    ] {
        let mut r = request_at(70.0);
        r.settings.smooth = smooth;
        r.settings.ignore_below = ignore;
        check(label, &p1(), &r, &c);
    }
    let mut remove = request_at(70.0);
    remove.selection = vectrace::Selection::RemoveBackground {
        background: Some(vectrace::Rgb8::new(0xD8, 0xD8, 0xD8)),
        tolerance: 40,
    };
    check(
        "remove background on P2",
        &p2(),
        &remove,
        &clip(PxRect::new(520, 100, 980, 900)),
    );
}
