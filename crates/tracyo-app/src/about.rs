//! The About panel: what Tracyo is, its licences and what each value of the bar does.

use eframe::egui::{
    self, Id, Key, Modifiers, Rect, RichText, Role, ScrollArea, Sense, Ui, pos2, vec2,
};
use vectrace::ocr::{ALPHABET_SHA256, DETECTOR_SHA256, RECOGNISER_SHA256};

use crate::app::App;
use crate::theme;
use crate::widgets;

/// Width of the docked panel.
pub const PANEL_WIDTH: f32 = 280.0;
/// Width of the text inside it (12 px padding on both sides).
const TEXT_WIDTH: f32 = 244.0;

/// The source address, shown as text (the app opens no links).
pub const SOURCE: &str = "https://github.com/curvyo/tracyo";

/// The heading of the panel and the name of the button that opens it.
pub const TITLE: &str = "About Tracyo";

/// The text under the heading.
pub const INTRO: &str = "Tracyo turns a picture into paths and saves them as an Inkscape SVG. The method decides which parts become shapes. Press Enter or Save as\u{2026} to save the paths.";

/// The first characters of a SHA-256, as far as a person can compare them.
fn short(sha: &str) -> &str {
    sha.get(..12).unwrap_or(sha)
}

/// Term and text of every entry, in the order of the panel: Tracyo's own, then Curvyo's Trace
/// entries in bar order (without Mask and Parts, which Tracyo has not).
#[must_use]
pub fn entries() -> Vec<(String, String)> {
    let own = [
        ("Version", format!("Tracyo {}.", env!("CARGO_PKG_VERSION"))),
        (
            "Licence",
            "AGPL-3.0-or-later. You may use, study, share and change Tracyo under the terms of the GNU Affero General Public License, version 3 or later."
                .to_owned(),
        ),
        ("Source", SOURCE.to_owned()),
        (
            "Tracing library",
            "vectrace, MIT OR Apache-2.0. It traces, recognises text and finds shapes.".to_owned(),
        ),
        (
            "Text recognition model",
            format!(
                "PaddleOCR Latin, Apache-2.0, built into the program. SHA-256 of the files: det.onnx {}, rec.onnx {}, dict.txt {}.",
                short(DETECTOR_SHA256),
                short(RECOGNISER_SHA256),
                short(ALPHABET_SHA256),
            ),
        ),
        (
            "Third-party code",
            "The licences of every library in this program are in THIRD-PARTY.md, next to the program and in the source."
                .to_owned(),
        ),
        (
            "Network",
            "Tracyo makes no network connection. Nothing leaves your computer.".to_owned(),
        ),
    ];
    let mut all: Vec<(String, String)> = own.into_iter().map(|(t, x)| (t.to_owned(), x)).collect();
    all.extend(
        TRACE_ENTRIES
            .iter()
            .map(|(t, x)| ((*t).to_owned(), (*x).to_owned())),
    );
    all
}

/// Curvyo's "About Trace" entries (`traceText.ts`) without Mask and Parts.
const TRACE_ENTRIES: [(&str, &str); 24] = [
    (
        "Preset",
        "a named set of values for what you do with the paths.",
    ),
    ("Logo / Line art", "sharp corners, clean curves."),
    ("Photo", "many shapes, photos trace roughly."),
    (
        "Silhouette for cutting/carving",
        "closed, simple, few nodes; drops shapes and holes below 2 mm.",
    ),
    ("Engraving", "keeps fine lines, drops shapes below 0.1 mm."),
    ("Method", "how pixels become shapes."),
    ("Brightness", "pixels darker than the Threshold."),
    (
        "Remove background",
        "everything unlike the background colour.",
    ),
    (
        "Background",
        "the colour Remove background leaves out; Auto takes it from the image border.",
    ),
    (
        "Pick colours",
        "pixels like the colours picked with the pipette; Shift+click adds a colour, up to 8. The picked colours sit beside the pipette; a click on one takes it out.",
    ),
    (
        "Pipette",
        "takes a colour from the image under a click; Esc ends it.",
    ),
    (
        "Colour tolerance",
        "how far a colour may be from the background or a picked colour and still count as it.",
    ),
    (
        "Threshold",
        "pixels darker than this brightness (0 to 100 %) become shapes.",
    ),
    (
        "Smooth",
        "calms the pixel steps of the outline before the fit; higher is calmer.",
    ),
    (
        "Tolerance",
        "how far in pixels the paths may stray from the calmed outline; higher gives fewer nodes.",
    ),
    (
        "Ignore below",
        "drops shapes and holes smaller than this size in mm.",
    ),
    (
        "Output",
        "Single colour traces the content as one black shape; Multicolour gives one filled shape per colour, neighbours sharing their edges.",
    ),
    (
        "Colours",
        "how many colours Multicolour finds, 2 to 16; Auto counts them in the image. With Pick colours the picked colours are the palette.",
    ),
    (
        "Palette",
        "the colours of a Multicolour trace beside the palette pipette, largest area first. The x on a colour deletes it: the others are matched again as if it had never been found; with no colour left nothing is traced. Ctrl+click excludes a colour: it becomes background and is not traced; a second Ctrl+click brings it back.",
    ),
    (
        "Invert",
        "traces what the method leaves out; transparent pixels never become shapes.",
    ),
    (
        "Drop border shapes",
        "drops shapes that touch the image border.",
    ),
    (
        "Recognise text",
        "reads the text in the picture first, sets each line as text in the nearest font and leaves it out of the paths.",
    ),
    (
        "Recognise shapes",
        "finds rectangles, circles, ellipses, polygons and stars in the traced outlines and sets them as shapes.",
    ),
    (
        "Denoise, Upscale, Corners, Analyse",
        "Denoise smooths out noise before tracing. Upscale doubles small images before tracing; Auto does it below 1 Mpx. Corners: higher turns more bends into sharp corners. Analyse sets the method, its background and Colour tolerance, and the Threshold back to what the analysis found in the image; never the preset or other values.",
    ),
];

/// All the panel's words as one text, for tests and screen readers.
#[must_use]
pub fn text() -> String {
    let mut lines = vec![TITLE.to_owned(), INTRO.to_owned()];
    for (term, body) in entries() {
        lines.push(term);
        lines.push(body);
    }
    lines.join("\n") + "\n"
}

impl App {
    /// Whether the About panel is open.
    #[must_use]
    pub fn about_open(&self) -> bool {
        self.about_open
    }

    /// How far the About text is scrolled, in points.
    #[must_use]
    pub fn about_scroll(&self) -> f32 {
        self.about_scroll
    }

    /// The docked panel at the right edge of the canvas, `panel` being its whole rectangle.
    pub(crate) fn about_panel(&mut self, ctx: &egui::Context, panel: Rect) {
        egui::Area::new(Id::unique("about-panel"))
            .order(egui::Order::Middle)
            .fixed_pos(panel.min)
            .show(ctx, |ui| {
                let (rect, resp) = ui.allocate_exact_size(panel.size(), Sense::hover());
                widgets::labelled(&resp, Role::Region, TITLE);
                ui.painter().rect_filled(rect, 0.0, theme::TOOLBAR_BG);
                ui.painter().vline(
                    rect.left() + 0.5,
                    rect.y_range(),
                    egui::Stroke::new(1.0, theme::DIVIDER),
                );
                ui.painter().text(
                    pos2(rect.left() + 12.0, rect.top() + 12.0),
                    egui::Align2::LEFT_TOP,
                    TITLE,
                    theme::text14(),
                    theme::TOOLBAR_ICON,
                );
                let body = Rect::from_min_max(
                    pos2(rect.left() + 12.0, rect.top() + 12.0 + 20.0 + 8.0),
                    rect.max,
                );
                // The body is a Tab stop only when it scrolls; its arrow, page and Home / End keys
                // then scroll it.
                let id = Id::unique("about-body");
                let mut jump = None;
                if self.about_scrolls {
                    let resp = ui.interact(body, id, Sense::focusable_noninteractive());
                    widgets::labelled(&resp, Role::ScrollView, "About Tracyo text");
                    if resp.has_focus() {
                        jump = scrolled(ui, self.about_scroll, body.height());
                        if widgets::focus_visible(ctx) {
                            widgets::ring(ui, body, 0.0, false);
                        }
                    }
                }
                let mut child = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(body)
                        .layout(egui::Layout::top_down(egui::Align::Min)),
                );
                let mut area = ScrollArea::vertical()
                    .id_salt("about-scroll")
                    .auto_shrink([false, false]);
                if let Some(to) = jump {
                    area = area.vertical_scroll_offset(to);
                }
                let out = area.show(&mut child, body_text);
                self.about_scroll = out.state.clamped_offset().y;
                self.about_scrolls = out.content_size.y > out.inner_rect.height() + 1.0;
            });
    }
}

/// The offset a focused scroll area is sent to by the keys of this frame, `page` px high.
fn scrolled(ui: &Ui, offset: f32, page: f32) -> Option<f32> {
    let step = |key: Key| ui.input_mut(|i| i.consume_key(Modifiers::NONE, key));
    let mut to = None;
    for (key, target) in [
        (Key::ArrowDown, offset + 40.0),
        (Key::ArrowUp, offset - 40.0),
        (Key::PageDown, offset + page * 0.9),
        (Key::PageUp, offset - page * 0.9),
        (Key::Home, 0.0),
        (Key::End, 1.0e6),
    ] {
        if step(key) {
            to = Some(target.max(0.0));
        }
    }
    to
}

/// A paragraph of 14 px text on a 20 px line.
fn paragraph(ui: &mut Ui, text: &str, bold: bool, muted: bool) {
    let font = if bold {
        theme::text14_bold()
    } else {
        theme::text14()
    };
    let colour = if muted {
        theme::MUTED
    } else {
        theme::TOOLBAR_ICON
    };
    ui.add(
        egui::Label::new(
            RichText::new(text)
                .font(font)
                .color(colour)
                .line_height(Some(20.0)),
        )
        .wrap(),
    );
}

fn body_text(ui: &mut Ui) {
    ui.set_width(TEXT_WIDTH);
    ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
    paragraph(ui, INTRO, false, false);
    for (term, body) in entries() {
        ui.add_space(12.0);
        paragraph(ui, &term, true, false);
        ui.add_space(4.0);
        paragraph(ui, &body, false, true);
    }
}
