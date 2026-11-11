//! The settings file in the platform's per-user config folder.

use std::io::Write;
use std::path::{Path, PathBuf};

use tracyo_core::settings::Settings;

/// The settings file's place and the last values written.
#[derive(Debug, Clone)]
pub struct Config {
    path: Option<PathBuf>,
}

impl Config {
    /// The file `<config dir>/tracyo/settings.toml`.
    #[must_use]
    pub fn standard() -> Self {
        Self {
            path: dirs::config_dir().map(|dir| dir.join("tracyo").join("settings.toml")),
        }
    }

    /// A file at `path` (tests); `None` keeps nothing.
    #[must_use]
    pub fn at(path: Option<PathBuf>) -> Self {
        Self { path }
    }

    /// The remembered settings; the defaults for a missing or corrupt file.
    #[must_use]
    pub fn load(&self) -> Settings {
        self.path
            .as_deref()
            .and_then(|path| std::fs::read_to_string(path).ok())
            .map_or_else(Settings::default, |text| Settings::from_toml(&text))
    }

    /// Writes `settings` through a temporary file in the same folder; a failure is ignored (the
    /// next change writes again).
    pub fn save(&self, settings: &Settings) {
        if let Some(path) = &self.path {
            let _ = write_atomic(path, settings.to_toml().as_bytes());
        }
    }
}

/// Writes `bytes` to a temporary file next to `path` and renames it over `path`.
///
/// # Errors
/// The system's reason when the folder cannot be made, written or renamed into.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let dir = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(dir)?;
    let mut file = tempfile::NamedTempFile::new_in(dir)?;
    file.write_all(bytes)?;
    file.persist(path).map_err(|e| e.error)?;
    Ok(())
}

/// The system's reason of an I/O error without its "(os error N)" tail.
#[must_use]
pub fn reason(error: &std::io::Error) -> String {
    let text = error.to_string();
    text.find(" (os error")
        .map_or(text.clone(), |i| text[..i].to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_survive_a_restart_and_a_corrupt_file_gives_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let config = Config::at(Some(dir.path().join("tracyo").join("settings.toml")));
        assert_eq!(config.load(), Settings::default());
        let s = Settings {
            smooth: 41,
            ..Settings::default()
        };
        config.save(&s);
        assert_eq!(config.load(), s);
        std::fs::write(
            dir.path().join("tracyo").join("settings.toml"),
            "smooth = [[",
        )
        .unwrap();
        assert_eq!(config.load(), Settings::default());
        config.save(&s);
        assert_eq!(config.load(), s);
    }

    #[test]
    fn an_atomic_write_leaves_no_temporary_file() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("out.svg");
        write_atomic(&target, b"x").unwrap();
        assert_eq!(std::fs::read(&target).unwrap(), b"x");
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
    }
}
