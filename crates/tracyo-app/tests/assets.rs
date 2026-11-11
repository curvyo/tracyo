//! Every bundled asset is named in `assets/ASSETS.md` with its SHA-256, and the hash is right.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

fn assets() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets")
}

fn files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            files(&path, out);
        } else {
            out.push(path);
        }
    }
}

#[test]
fn every_asset_is_listed_with_its_hash() {
    let notes = std::fs::read_to_string(assets().join("ASSETS.md")).unwrap();
    let mut all = Vec::new();
    files(&assets(), &mut all);
    for path in all {
        let relative = path
            .strip_prefix(assets())
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        if relative == "ASSETS.md" {
            continue;
        }
        let line = notes
            .lines()
            .find(|l| l.starts_with(&format!("| `{relative}`")))
            .unwrap_or_else(|| panic!("{relative} is not named in ASSETS.md"));
        let hash = format!("{:x}", Sha256::digest(std::fs::read(&path).unwrap()));
        assert!(
            line.contains(&hash),
            "{relative}: ASSETS.md has a different SHA-256 than {hash}"
        );
    }
}
