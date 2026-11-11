//! The UI font: the system's own where Curvyo shows it, Liberation Sans embedded as the fallback.

use std::collections::BTreeMap;
use std::sync::Arc;

use eframe::egui::{self, FontData, FontDefinitions, FontFamily};

use crate::theme::{BOLD, REGULAR};

const LIBERATION_REGULAR: &[u8] =
    include_bytes!("../../../assets/fonts/LiberationSans-Regular.ttf");
const LIBERATION_BOLD: &[u8] = include_bytes!("../../../assets/fonts/LiberationSans-Bold.ttf");

/// Where the faces come from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// The system's UI font per OS (Curvyo's list), the embedded face as the last fallback.
    System,
    /// Only the embedded Liberation Sans (tests).
    Embedded,
}

/// The families Curvyo's stack names, in order, for this OS.
#[must_use]
pub fn wanted_families() -> &'static [&'static str] {
    if cfg!(target_os = "windows") {
        &["Segoe UI"]
    } else if cfg!(target_os = "macos") {
        // San Francisco has proportional digits and egui has no OpenType features.
        &[]
    } else {
        &["Roboto", "Noto Sans", "Arial", "Liberation Sans"]
    }
}

/// The regular and bold bytes of the first installed wanted family.
fn system_faces() -> Option<(FontData, FontData)> {
    let mut db = fontdb::Database::new();
    db.load_system_fonts();
    for name in wanted_families() {
        let find = |weight| {
            let query = fontdb::Query {
                families: &[fontdb::Family::Name(name)],
                weight,
                stretch: fontdb::Stretch::Normal,
                style: fontdb::Style::Normal,
            };
            let id = db.query(&query)?;
            db.with_face_data(id, |data, index| {
                let mut face = FontData::from_owned(data.to_vec());
                face.index = index;
                face
            })
        };
        if let (Some(regular), Some(bold)) =
            (find(fontdb::Weight::NORMAL), find(fontdb::Weight::BOLD))
        {
            return Some((regular, bold));
        }
    }
    None
}

/// The font definitions: regular for text, the bold face for weight 600, embedded fallback last.
#[must_use]
pub fn definitions(source: Source) -> FontDefinitions {
    let mut font_data: BTreeMap<String, Arc<FontData>> = BTreeMap::new();
    let mut regular = Vec::new();
    let mut bold = Vec::new();
    if source == Source::System
        && let Some((r, b)) = system_faces()
    {
        font_data.insert("system-regular".into(), Arc::new(r));
        font_data.insert("system-bold".into(), Arc::new(b));
        regular.push("system-regular".to_owned());
        bold.push("system-bold".to_owned());
    }
    font_data.insert(
        REGULAR.into(),
        Arc::new(FontData::from_static(LIBERATION_REGULAR)),
    );
    font_data.insert(
        BOLD.into(),
        Arc::new(FontData::from_static(LIBERATION_BOLD)),
    );
    regular.push(REGULAR.to_owned());
    bold.push(BOLD.to_owned());
    let mut families = BTreeMap::new();
    families.insert(FontFamily::Proportional, regular.clone());
    families.insert(FontFamily::Monospace, regular);
    families.insert(FontFamily::Name(BOLD.into()), bold);
    FontDefinitions {
        font_data,
        families,
    }
}

/// Installs the fonts into `ctx`.
pub fn install(ctx: &egui::Context, source: Source) {
    ctx.set_fonts(definitions(source));
}
