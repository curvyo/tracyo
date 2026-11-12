//! The lucide icons, rasterised by resvg at the device pixel size and cached as textures.

use std::collections::HashMap;

use eframe::egui::{self, Color32, ColorImage, TextureHandle, TextureOptions, Vec2};

/// An icon of the bundled set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Icon {
    /// The Open card.
    Import,
    /// About.
    Info,
    /// Stop inside the blue button.
    X,
    /// Failed.
    CircleAlert,
    /// The pipettes.
    Pipette,
    /// Invert.
    Contrast,
    /// Drop border shapes.
    ImageMinus,
    /// Recognise text.
    ScanText,
    /// Recognise shapes.
    Shapes,
    /// An excluded palette colour.
    Ban,
    /// The expander while the Advanced block is closed.
    ChevronsDown,
    /// The expander while it is open.
    ChevronsUp,
    /// The Rectangle tool.
    Square,
    /// The Subtract toggle.
    SquareMinus,
}

impl Icon {
    fn svg(self) -> &'static str {
        match self {
            Self::Import => include_str!("../../../assets/icons/lucide/import.svg"),
            Self::Info => include_str!("../../../assets/icons/lucide/info.svg"),
            Self::X => include_str!("../../../assets/icons/lucide/x.svg"),
            Self::CircleAlert => include_str!("../../../assets/icons/lucide/circle-alert.svg"),
            Self::ScanText => include_str!("../../../assets/icons/lucide/scan-text.svg"),
            Self::Shapes => include_str!("../../../assets/icons/lucide/shapes.svg"),
            Self::Ban => include_str!("../../../assets/icons/lucide/ban.svg"),
            Self::Pipette => include_str!("../../../assets/icons/lucide/pipette.svg"),
            Self::Contrast => include_str!("../../../assets/icons/lucide/contrast.svg"),
            Self::ImageMinus => include_str!("../../../assets/icons/lucide/image-minus.svg"),
            Self::ChevronsDown => include_str!("../../../assets/icons/lucide/chevrons-down.svg"),
            Self::ChevronsUp => include_str!("../../../assets/icons/lucide/chevrons-up.svg"),
            Self::Square => include_str!("../../../assets/icons/lucide/square.svg"),
            Self::SquareMinus => include_str!("../../../assets/icons/lucide/square-minus.svg"),
        }
    }
}

type Key = (Icon, u32, u32, [u8; 4], u32);

/// The texture cache of rendered icons.
#[derive(Default)]
pub struct Icons {
    cache: HashMap<Key, TextureHandle>,
}

fn render(icon: Icon, size_px: u32, stroke: f32, colour: Color32) -> Option<ColorImage> {
    let [r, g, b, _] = colour.to_array();
    let svg = icon
        .svg()
        .replace("currentColor", &format!("#{r:02x}{g:02x}{b:02x}"))
        .replace("stroke-width=\"2\"", &format!("stroke-width=\"{stroke}\""));
    let tree = resvg::usvg::Tree::from_str(&svg, &resvg::usvg::Options::default()).ok()?;
    let mut pixmap = resvg::tiny_skia::Pixmap::new(size_px, size_px)?;
    // invariant: icon sizes are tiny, the cast to f32 is exact.
    #[allow(clippy::cast_precision_loss)]
    let scale = size_px as f32 / 24.0;
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    Some(ColorImage::from_rgba_premultiplied(
        [size_px as usize, size_px as usize],
        pixmap.data(),
    ))
}

impl Icons {
    /// The texture of `icon` for a square of `size` points with `stroke` (in the 24-unit viewBox)
    /// in `colour`, at the device's `pixels_per_point`.
    pub fn texture(
        &mut self,
        ctx: &egui::Context,
        icon: Icon,
        size: f32,
        stroke: f32,
        colour: Color32,
    ) -> Option<egui::TextureId> {
        let ppp = ctx.pixels_per_point();
        // invariant: sizes are small positive numbers.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let size_px = (size * ppp).round().max(1.0) as u32;
        let key = (
            icon,
            size_px,
            stroke.to_bits(),
            colour.to_array(),
            ppp.to_bits(),
        );
        if let std::collections::hash_map::Entry::Vacant(slot) = self.cache.entry(key) {
            let image = render(icon, size_px, stroke, colour)?;
            slot.insert(ctx.load_texture(
                format!("icon-{icon:?}-{size_px}"),
                image,
                TextureOptions::LINEAR,
            ));
        }
        self.cache.get(&key).map(TextureHandle::id)
    }

    /// Paints `icon` centred in `rect`.
    pub fn paint(
        &mut self,
        ui: &egui::Ui,
        rect: egui::Rect,
        icon: Icon,
        size: f32,
        stroke: f32,
        colour: Color32,
    ) {
        if let Some(id) = self.texture(ui.ctx(), icon, size, stroke, colour) {
            let target = egui::Rect::from_center_size(rect.center(), Vec2::splat(size));
            ui.painter().image(
                id,
                target,
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                Color32::WHITE,
            );
        }
    }
}
