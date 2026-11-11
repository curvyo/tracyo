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

/// The embedded model is the one `vectrace::ocr` expects, and its licence record names the same
/// hashes (ADR 0006): a vectrace bump that changes them fails here until the files are copied
/// again from the new tag.
#[test]
fn the_model_files_have_the_hashes_vectrace_expects() {
    let dir = assets().join("models/paddleocr-latin");
    let record = std::fs::read_to_string(dir.join("model.toml")).unwrap();
    for (file, expected) in [
        ("det.onnx", vectrace::ocr::DETECTOR_SHA256),
        ("rec.onnx", vectrace::ocr::RECOGNISER_SHA256),
        ("dict.txt", vectrace::ocr::ALPHABET_SHA256),
    ] {
        let hash = format!(
            "{:x}",
            Sha256::digest(std::fs::read(dir.join(file)).unwrap())
        );
        assert_eq!(hash, expected, "{file} differs from what vectrace expects");
        assert!(
            record.contains(&format!("sha256 = \"{expected}\"")),
            "model.toml has no entry for {file}"
        );
    }
    assert!(record.contains("licence = \"Apache-2.0\""));
    assert!(dir.join("LICENSE").is_file());
}
