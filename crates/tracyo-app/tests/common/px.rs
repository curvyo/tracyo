//! The pixel comparison of regions of the bar with the references of a spec folder.

#![allow(dead_code, clippy::unwrap_used, clippy::expect_used)]

use super::compare;

/// The reference picture `name` of `specs/<spec>/reference`.
pub fn reference(spec: &str, name: &str) -> image::RgbImage {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../specs")
        .join(spec)
        .join("reference")
        .join(name);
    image::open(path).expect("reference picture").to_rgb8()
}

/// One rectangle of a comparison: `size` pixels at `reference_at` from the reference bar's corner
/// and at `got_at` from the render's.
#[derive(Clone)]
pub struct Region {
    pub name: &'static str,
    pub reference_at: (u32, u32),
    pub got_at: (u32, u32),
    pub size: (u32, u32),
}

impl Region {
    /// A region at the same place in both pictures.
    pub fn same(name: &'static str, at: (u32, u32), size: (u32, u32)) -> Self {
        Self {
            name,
            reference_at: at,
            got_at: at,
            size,
        }
    }
}

/// The render cut so that the bar's top-left corner lies at (12, 12), where the references have it:
/// the bar of a 1280 x 800 window is centred, the references are crops with a 12 px margin. Both
/// are found by the bar's ground, as `compare.py` does.
pub fn aligned(full: &image::RgbImage) -> image::RgbImage {
    let (x, y) = compare::bar_origin(full).expect("no Trace bar in the render");
    let (left, top) = (x.saturating_sub(12), y.saturating_sub(12));
    image::imageops::crop_imm(full, left, top, full.width() - left, full.height() - top).to_image()
}

/// Compares every region of the render with the reference `name` of `spec` and asserts that all
/// pass; each has its own numbers and diff picture. The render is the whole 1280 x 800 window; the
/// bar is found by its ground.
pub fn check_regions(spec: &str, name: &str, rendered: &image::RgbaImage, regions: &[Region]) {
    let out = compare::out_dir();
    let got = aligned(&compare::rgb(rendered));
    got.save(out.join(format!("render-{name}.png")))
        .expect("save render");
    let reference = reference(spec, &format!("{name}.png"));
    let mut failed = Vec::new();
    for r in regions {
        let outcome = compare::compare_between(
            &reference,
            &got,
            r.reference_at,
            r.got_at,
            r.size.0,
            r.size.1,
        );
        outcome
            .diff
            .save(out.join(format!("diff-{name}-{}.png", r.name)))
            .expect("save diff");
        println!("{name} {}: {}", r.name, outcome.summary());
        if !outcome.passed() {
            failed.push(r.name);
        }
    }
    assert!(failed.is_empty(), "{name}: failed regions {failed:?}");
}

/// How much wider a bar is since Analyse joined the tail (customer decision 2026-11-15: a 28 px
/// icon button and an 8 px gap).
pub const ANALYSE_SHIFT: u32 = 36;

/// The regions of a reference bar that still compare when Analyse stands in the tail, between the
/// expander and the blue button. The reference bar is `old_w` wide and `height` high, its blue
/// button `blue_w` wide; the rendered bar is `ANALYSE_SHIFT` px wider. Compared: the first row up
/// to 4 px before the blue button (it holds the expander, which has not moved), the blue button
/// with its 4 px ring room and the bar's end, and, with the block open, the block's lines up to 4 px
/// before where the reference has its Analyse button, and the bar's right end. No reference exists
/// for the Analyse button itself (Curvyo has none there): `assert_analyse` checks it.
pub fn bar_regions(old_w: u32, blue_w: u32, height: u32) -> Vec<Region> {
    let blue_left = old_w - 8 - blue_w;
    let mut regions = vec![
        Region::same("first row", (0, 0), (blue_left - 4, 36)),
        Region {
            name: "blue button",
            reference_at: (blue_left - 4, 0),
            got_at: (blue_left - 4 + ANALYSE_SHIFT, 0),
            size: (blue_w + 8 + 4, 36),
        },
    ];
    if height > 36 {
        // Curvyo's Analyse stood 75 px wide, flush right in the block's 4 px padding.
        regions.push(Region::same(
            "block",
            (0, 36),
            (old_w - 12 - 75 - 4, height - 36),
        ));
        regions.push(Region {
            name: "block end",
            reference_at: (old_w - 12, 36),
            got_at: (old_w - 12 + ANALYSE_SHIFT, 36),
            // Not the 12 px of the bottom corner: its shadow falls on the picture behind it,
            // which ends elsewhere now that the bar is wider.
            size: (12, height - 36 - 12),
        });
    }
    regions
}

/// Asserts that the render has the Analyse button 8 px before the blue button, `blue_w` wide, of a
/// bar `new_w` wide, in the row that starts `row_y` px below the bar's top: a 28 px blue square (`#2F6FEE`) with white glyph pixels in it.
pub fn assert_analyse(rendered: &image::RgbaImage, new_w: u32, blue_w: u32, row_y: u32) {
    let got = aligned(&compare::rgb(rendered));
    let left = new_w - 8 - blue_w - 8 - 28;
    let blue = [0x2F, 0x6F, 0xEE];
    // Offsets from the bar's corner at (12, 12): the button is 4 px below the row's top.
    let at = |x: u32, y: u32| got.get_pixel(12 + x, 12 + row_y + y).0;
    for (x, y) in [(left + 6, 4 + 3), (left + 21, 4 + 24), (left + 3, 4 + 14)] {
        assert_eq!(at(x, y), blue, "Analyse ground at ({x}, {y})");
    }
    let white = (left..left + 28)
        .flat_map(|x| (4..32).map(move |y| (x, y)))
        .filter(|&(x, y)| at(x, y)[0] > 150)
        .count();
    assert!(white >= 12, "the glyph is light ({white} px)");
    // Both sides of the button are the bar's ground.
    assert_eq!(at(left - 4, 18), [0xDC, 0xDC, 0xE0], "gap before Analyse");
    assert_eq!(
        at(left + 28 + 4, 18),
        [0xDC, 0xDC, 0xE0],
        "gap after Analyse"
    );
}
