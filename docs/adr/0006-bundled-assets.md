# ADR 0006: Bundled assets: OCR model, fonts and icons

**Status:** Accepted (lead, slice 5, 2026-11-11; proposed by the architect 2026-11-11). The embedded model follows spec Frage 4 at its default.

Followed as written: the model, fonts and icons are embedded and hash-checked; the About panel names the model's licence with the first 12 hex digits of its three SHA-256 values; the release archives carry the licence files of `assets/` and `THIRD-PARTY.md`.

## Context

Recognise text needs the PaddleOCR Latin weights (detector 2.4 MB, recogniser 7.9 MB, alphabet
1.6 kB; Apache-2.0). The first use must need no download (criteria 16 and 23). vectrace keeps
them in `assets/models/paddleocr-latin/` with a licence record (`model.toml`, SHA-256 per file)
and checks the hashes in `OcrModel::load`. The UI needs Liberation Sans (SIL OFL 1.1) and
lucide 1.50.0 icons (ISC).

| Option | Verdict |
|---|---|
| **A. Copy into this repo, plain git, `include_bytes!` (chosen)** | One self-contained binary, offline builds, same files and hashes as vectrace. Cost: about 10 MB in the repository history per model change; the binary grows by 9.9 MB. |
| B. Read from vectrace's git checkout at build time (`build.rs`) | Ties the build to Cargo's internal checkout path; fragile, and vectrace keeps the files for its tests, not as an API. |
| C. Git LFS | Needs LFS in CI and every clone for 10 MB; not worth it. |
| D. Files next to the executable | Two artefacts to ship and keep together on three platforms. |

## Decision

```
assets/ASSETS.md                         source, licence, version and hash of every file
assets/models/paddleocr-latin/           det.onnx rec.onnx dict.txt LICENSE model.toml (copied from vectrace v0.1.0)
assets/fonts/                            LiberationSans-Regular.ttf LiberationSans-Bold.ttf OFL.txt
assets/icons/lucide/                     the SVGs of ux-notes §11 from lucide-static@1.50.0, LICENSE
```

- `tracyo-app::worker` embeds the model with `include_bytes!` and calls `OcrModel::load` only for a
  `ReadText` request. `tracyo-app::fonts` and `icons` embed theirs the same way.
- A test (`crates/tracyo-app/tests/assets.rs`) checks that every model file has an entry in
  `model.toml` with the SHA-256 vectrace expects (`vectrace::ocr::DETECTOR_SHA256`,
  `RECOGNISER_SHA256`, `ALPHABET_SHA256`), and that every file under `assets/` is named in
  `ASSETS.md`.
- A vectrace bump that changes the hashes fails that test; the files are copied again from the
  new tag in the same pull request.
- The About panel names the model's licence (criterion 26); the release archive carries the
  licence files of `assets/`.

## Consequences

- The binary is about 29 MB (19 MB code measured in the spike, plus the model).
- Core tests that need OCR read the files from `assets/` by path; they do not embed them.
