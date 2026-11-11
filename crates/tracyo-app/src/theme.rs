//! Every token of the look (ux-notes section 2) as a named constant, and the egui style.

use eframe::egui::{self, Color32, FontFamily, FontId, Stroke, Visuals, epaint::Shadow};

/// Canvas ground.
pub const CANVAS_BG: Color32 = Color32::from_rgb(0xE8, 0xE8, 0xEB);
/// Bar, Open card, menus, chip, notice, status bar.
pub const TOOLBAR_BG: Color32 = Color32::from_rgb(0xDC, 0xDC, 0xE0);
/// Text and glyphs of the chrome.
pub const TOOLBAR_ICON: Color32 = Color32::from_rgb(0x3A, 0x3A, 0x3F);
/// Pressed ground, blue buttons, focus ring, value edge, preview line.
pub const ACCENT: Color32 = Color32::from_rgb(0x2F, 0x6F, 0xEE);
/// Hover ground on the bar.
pub const HOVER_ON_BAR: Color32 = Color32::from_rgb(0xB9, 0xC6, 0xE3);
/// Muted text.
pub const MUTED: Color32 = Color32::from_rgb(0x5A, 0x5A, 0x5F);
/// Value-field border at rest.
pub const FIELD_BORDER: Color32 = Color32::from_rgb(0x89, 0x89, 0x8C);
/// Preset and Method button border.
pub const BUTTON_BORDER: Color32 = Color32::from_rgb(0x9B, 0x9B, 0xA0);
/// Divider and panel edge.
pub const DIVIDER: Color32 = Color32::from_rgb(0xB4, 0xB4, 0xB8);
/// Value-field bar.
pub const VALUE_FILL: Color32 = Color32::from_rgb(0xC5, 0xD7, 0xFA);
/// Alert notice text and ring.
pub const INVALID: Color32 = Color32::from_rgb(0xB3, 0x26, 0x1E);
/// Status-bar text and tooltip text.
pub const FOREGROUND: Color32 = Color32::from_rgb(0x0A, 0x0A, 0x0A);
/// Tooltip ground.
pub const POPOVER: Color32 = Color32::WHITE;
/// Tooltip ring.
pub const POPOVER_RING: Color32 = Color32::from_rgb(0xE7, 0xE7, 0xE7);
/// Blue button hover.
pub const BLUE_HOVER: Color32 = Color32::from_rgb(0x34, 0x7A, 0xFF);
/// Running fill of the blue button.
pub const RUNNING_FILL: Color32 = Color32::from_rgb(0x63, 0x93, 0xF2);
/// White 20 % on blue.
pub const WHITE_20_ON_BLUE: Color32 = Color32::from_rgb(0x59, 0x8C, 0xF1);
/// Dimmed blue button ground.
pub const DIMMED_BLUE: Color32 = Color32::from_rgb(0x86, 0xA6, 0xE7);
/// Dimmed blue button text.
pub const DIMMED_TEXT: Color32 = Color32::from_rgb(0xEE, 0xEE, 0xF0);
/// Selected text in a number entry.
pub const SELECTION: Color32 = Color32::from_rgb(0x33, 0x67, 0xD1);
/// Menu second line, 70 % of the text colour.
pub const MENU_SECOND: Color32 = Color32::from_rgb(0x6B, 0x6B, 0x6F);
/// "Stopped" at 90 % white on blue.
pub const STOPPED_TEXT: Color32 = Color32::from_rgb(0xEA, 0xF1, 0xFD);

/// Radius of bar, menus and the Open card.
pub const RADIUS_BAR: f32 = 10.0;
/// Radius of buttons, tooltips, notice and menu rows.
pub const RADIUS_BUTTON: f32 = 8.0;
/// Radius of the value field.
pub const RADIUS_FIELD: f32 = 5.0;

/// The shadow of floating surfaces: `0 2px 8px rgba(0,0,0,.24)`.
pub const SHADOW: Shadow = Shadow {
    offset: [0, 2],
    blur: 8,
    spread: 0,
    color: Color32::from_rgba_premultiplied(0, 0, 0, 61),
};

/// The regular face.
pub const REGULAR: &str = "tracyo-regular";
/// The bold face, used for weight 600.
pub const BOLD: &str = "tracyo-bold";

/// 14 px regular text.
#[must_use]
pub fn text14() -> FontId {
    FontId::new(14.0, FontFamily::Proportional)
}

/// 14 px weight 600 text.
#[must_use]
pub fn text14_bold() -> FontId {
    FontId::new(14.0, FontFamily::Name(BOLD.into()))
}

/// 12 px regular text.
#[must_use]
pub fn text12() -> FontId {
    FontId::new(12.0, FontFamily::Proportional)
}

/// 10 px regular text.
#[must_use]
pub fn text10() -> FontId {
    FontId::new(10.0, FontFamily::Proportional)
}

/// Sets egui's visuals and text selection for the light theme.
pub fn install_style(ctx: &egui::Context) {
    ctx.set_theme(egui::Theme::Light);
    ctx.all_styles_mut(|style| {
        let mut visuals = Visuals::light();
        visuals.selection.bg_fill = SELECTION;
        visuals.selection.stroke = Stroke::new(1.0, Color32::WHITE);
        visuals.override_text_color = Some(TOOLBAR_ICON);
        visuals.panel_fill = CANVAS_BG;
        visuals.window_fill = TOOLBAR_BG;
        visuals.extreme_bg_color = Color32::WHITE;
        visuals.text_cursor.stroke = Stroke::new(1.0, TOOLBAR_ICON);
        style.visuals = visuals;
        style.interaction.tooltip_delay = 0.4;
        style.spacing.scroll = egui::style::ScrollStyle::thin();
    });
}
