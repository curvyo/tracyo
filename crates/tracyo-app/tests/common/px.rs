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

/// Compares every region of the render with the reference `name` of `spec` and asserts that all
/// pass; each has its own numbers and diff picture. The Open card (x 12 to 60) is Tracyo's own, so
/// the render is cut at x = 60 and the bar is found by its ground.
pub fn check_regions(spec: &str, name: &str, rendered: &image::RgbaImage, regions: &[Region]) {
    let out = compare::out_dir();
    let full = compare::rgb(rendered);
    let got = image::imageops::crop_imm(&full, 60, 0, full.width() - 60, full.height()).to_image();
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
