//! The open and save dialogs; tests replace the system dialogs by fixed answers.

use std::path::{Path, PathBuf};

/// Where the file choices come from.
#[derive(Debug, Clone)]
pub enum Dialogs {
    /// The operating system's dialogs.
    System,
    /// Fixed answers (tests): the file the Open dialog returns and the one Save returns.
    Fixed {
        /// The answer of Open.
        open: Option<PathBuf>,
        /// The answer of Save as.
        save: Option<PathBuf>,
    },
}

impl Dialogs {
    /// Asks for a picture, starting in `start`.
    #[must_use]
    pub fn open(&self, start: Option<&Path>) -> Option<PathBuf> {
        match self {
            Self::System => {
                let mut dialog = rfd::FileDialog::new()
                    .set_title("Open image")
                    .add_filter(
                        "Images (*.png, *.jpg, *.jpeg, *.webp)",
                        &["png", "jpg", "jpeg", "webp"],
                    )
                    .add_filter("All files", &["*"]);
                if let Some(dir) = start {
                    dialog = dialog.set_directory(dir);
                }
                dialog.pick_file()
            }
            Self::Fixed { open, .. } => open.clone(),
        }
    }

    /// Asks for the SVG file to write: `name` in `dir` is suggested.
    #[must_use]
    pub fn save(&self, dir: Option<&Path>, name: &str) -> Option<PathBuf> {
        match self {
            Self::System => {
                let mut dialog = rfd::FileDialog::new()
                    .set_title("Save as")
                    .set_file_name(name)
                    .add_filter("SVG (*.svg)", &["svg"]);
                if let Some(dir) = dir {
                    dialog = dialog.set_directory(dir);
                }
                dialog.save_file()
            }
            Self::Fixed { save, .. } => save.clone(),
        }
    }
}
