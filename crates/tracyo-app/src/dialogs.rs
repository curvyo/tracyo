//! The open and save dialogs; tests replace the system dialogs by fixed answers.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// A dialog that closes without a file sooner than a person could have dismissed it never
/// opened: on Linux `rfd` answers like that when no `xdg-desktop-portal` is running.
const NEVER_SHOWN: Duration = Duration::from_millis(150);

/// What a file dialog came back with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Choice {
    /// The file chosen.
    Picked(PathBuf),
    /// The person closed the dialog.
    Cancelled,
    /// The system has no file dialog to show.
    Unavailable,
}

impl Choice {
    /// The answer of a system dialog that took `elapsed` and returned `picked`.
    fn of_system(picked: Option<PathBuf>, elapsed: Duration) -> Self {
        match picked {
            Some(path) => Self::Picked(path),
            None if elapsed < NEVER_SHOWN => Self::Unavailable,
            None => Self::Cancelled,
        }
    }

    /// A fixed answer: a file, or a cancel.
    fn fixed(picked: Option<PathBuf>) -> Self {
        picked.map_or(Self::Cancelled, Self::Picked)
    }
}

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
    pub fn open(&self, start: Option<&Path>) -> Choice {
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
                let shown = Instant::now();
                Choice::of_system(dialog.pick_file(), shown.elapsed())
            }
            Self::Fixed { open, .. } => Choice::fixed(open.clone()),
        }
    }

    /// Asks for the SVG file to write: `name` in `dir` is suggested.
    #[must_use]
    pub fn save(&self, dir: Option<&Path>, name: &str) -> Choice {
        match self {
            Self::System => {
                let mut dialog = rfd::FileDialog::new()
                    .set_title("Save as")
                    .set_file_name(name)
                    .add_filter("SVG (*.svg)", &["svg"]);
                if let Some(dir) = dir {
                    dialog = dialog.set_directory(dir);
                }
                let shown = Instant::now();
                Choice::of_system(dialog.save_file(), shown.elapsed())
            }
            Self::Fixed { save, .. } => Choice::fixed(save.clone()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_dialog_that_returns_at_once_without_a_file_never_opened() {
        let quick = Duration::from_millis(20);
        let slow = Duration::from_secs(3);
        assert_eq!(Choice::of_system(None, quick), Choice::Unavailable);
        assert_eq!(Choice::of_system(None, slow), Choice::Cancelled);
        let file = PathBuf::from("a.png");
        assert_eq!(
            Choice::of_system(Some(file.clone()), quick),
            Choice::Picked(file)
        );
    }
}
