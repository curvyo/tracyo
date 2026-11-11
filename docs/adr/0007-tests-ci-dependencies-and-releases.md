# ADR 0007: Tests, CI, dependency policy and release builds

**Status:** Accepted (lead, slice 5, 2026-11-11; proposed by the architect 2026-11-11).

Deviations: `ci.yml` is split into one job per check plus a `required` job and a `third-party` job that checks `THIRD-PARTY.md` (section 7); `release.yml` adds `SHA256SUMS`, builds macOS x86_64 on the arm64 runner and has a dry run by `workflow_dispatch` (section 9); the nightly Windows run skips the tests named `unix_*` instead of whole binaries. Release builds are unsigned (spec Frage 6, default A, not answered otherwise); `nightly.yml` and `release.yml` have not run on Windows and macOS yet (debt 16).

## Context

The spec asks for golden and XML tests of the SVG (criterion 20), a rendered comparison
(criterion 21), a no-network proof (criterion 23), a frame budget in CPU time (criterion 7),
builds on three systems (criterion 25), and a pixel match with Curvyo's pictures (ux-notes §12).
Measured here: `egui_kittest` 0.37 with features `wgpu, snapshot` renders the app headless to a
PNG with Mesa llvmpipe through EGL, without an X server or a Vulkan driver.

| Option for UI pictures | Verdict |
|---|---|
| **A. `egui_kittest` (chosen)** | egui's own harness: drives the real `App` by AccessKit names (click "Save as…", type into "Smooth"), steps frames, `render()` gives an `image::RgbaImage` at `pixels_per_point = 1`. Uses wgpu, the app's renderer (ADR 0003). |
| B. Start the binary under Xvfb and screenshot | Real window, but timing-dependent, no access to state, window-manager decorations. Kept only as a start smoke test. |
| C. Own offscreen renderer | Rebuilds what kittest is. |

## Decision

1. **Unit and golden tests** in `tracyo-core`: settings (ranges, steps, presets, TOML round trip,
   corrupt file), `picture` (EXIF orientation fixtures, errors, reduced copy), `progress` (the 400
   / 300 ms rule with fake time, Esc order), `protocol` (round trips, truncated frames), `svg`
   (golden files `tests/fixtures/svg/*.svg` from fixed `Output`s and from traces of
   `tests/fixtures/pictures/`; a structure test with `roxmltree` checks every point of criterion
   20; a render test rasterises the SVG with `resvg` and compares the covered area with the
   traced outlines, at most 0.5 % apart, criterion 21's automatic substitute).
2. **App tests** in `crates/tracyo-app/tests/` with `egui_kittest` and the real worker
   (`CARGO_BIN_EXE_tracyo`): end-to-end flows per slice (open via argument, trace, stop, retry,
   save to a temp folder, settings file). The save dialog is behind the `dialogs` module, which
   tests replace by a fixed path (the one seam; the real `rfd` path is the second implementation).
3. **Pixel tests**: one kittest scene per `reference/px-NN-*.png` state at 1280 x 800, the
   embedded Liberation Sans, then the comparison of ux-notes §12.2, ported from `compare.py` to
   a Rust test helper (same alignment, 1 px neighbourhood, tolerance 24, at most 2 % and no run of
   12); its self-test cases (identical, 0.5 px blur passes, 30 x 20 block and 2 px narrower field
   fail) are tests too. The rendered pictures and diffs are kept as CI artefacts. Linux only.
4. **Frame budget** (criterion 7): a test traces a 12 Mpx picture in the worker and measures the
   UI thread's CPU time per `harness.step()` with `CLOCK_THREAD_CPUTIME_ID` (`rustix`, dev, unix),
   never wall time. The budget (50 ms) is timed in release only: the test is `ignore`d in debug builds
   (`cfg_attr(debug_assertions)`) and CI job `perf` runs it with `cargo nextest run --release`.
   **Out of memory** (criterion 24): the worker is started under `prlimit --as` in a Linux test and the app must show "Failed" with "out of memory".
5. **No network** (criterion 23): (a) `deny.toml` bans HTTP, TLS and browser-opening crates
   (`reqwest`, `hyper`, `ureq`, `ehttp`, `rustls`, `native-tls`, `openssl`, `openssl-sys`,
   `webbrowser`; `open` only as a dependency of `egui_kittest`). Local IPC (X11, Wayland, D-Bus
   for the portal) is not network and stays. (b) CI runs the full-session test (open, Recognise
   text and shapes, save) with `sudo unshare -n` (Ubuntu 24.04 runners forbid unprivileged user
   namespaces, so `unshare -rn` is replaced by sudo there) and compares the file with the run
   with network.
6. **`deny.toml`**: targets Linux x86_64 and aarch64, Windows x86_64 MSVC, macOS x86_64 and
   aarch64. Allowed licences: MIT, Apache-2.0, Apache-2.0 WITH LLVM-exception, BSD-2-Clause,
   BSD-3-Clause, ISC, Unicode-3.0, Zlib, MPL-2.0 (`option-ext` via `dirs`; file-level copyleft,
   AGPL-compatible); own crates by exception AGPL-3.0-or-later. Sources: crates.io and
   `https://github.com/curvyo/vectrace` only; the git dependency carries `version = "0.1.0"` (else
   it counts as a wildcard). `multiple-versions = "warn"`. A spike with the full dependency set
   (eframe/wgpu, rfd, image, resvg, fontdb, toml, dirs, tempfile, vectrace, egui_kittest) passed
   `cargo deny check licenses bans sources` with exactly this list, every licence encountered.
7. **CI** (`.github/workflows/`): `ci.yml` on every pull request and push to `main`, ubuntu-24.04:
   install Mesa (`mesa-vulkan-drivers libegl1`), Xvfb and Liberation fonts; the gate of `CLAUDE.md`
   (fmt, clippy, nextest incl. kittest and pixel tests, deny, doc), job `perf` (the release run of
   the frame budget, same system packages), the `unshare -n` session test,
   and a start smoke test (`timeout 5 xvfb-run target/debug/tracyo` must end by the timeout, not by
   an error). `nightly.yml`: windows-latest and macos-latest build, clippy and nextest without the
   pixel and Linux-only tests. Toolchain from `rust-toolchain.toml` (1.99.0), `Swatinem/rust-cache`.
8. **Renovate**: as vectrace: `.github/workflows/renovate.yml` (weekly cron and manual, self-hosted
   `renovatebot/github-action` with the `RENOVATE_TOKEN` secret) and `renovate.json`
   (`config:recommended`, Monday before 6 am). Groups: `eframe`, `egui*` and `epaint*` together;
   `vectrace` tag bumps alone (they may need the model files copied, ADR 0006); the toolchain file
   alone.
9. **Releases**: `release.yml` on a tag `v*`: x86_64 Linux (built on ubuntu-22.04 for an older
   glibc), x86_64 Windows (MSVC, `windows_subsystem = "windows"`), macOS aarch64 and x86_64; each
   an archive with the binary, `LICENSE` and the licence files of `assets/`, attached to the
   GitHub release. Unsigned until Frage 6 is answered.

## Consequences

- The pixel references were made on the maintainer's machine with Liberation Sans; kittest in CI
  renders with the same embedded font, so only rasteriser and GPU-driver differences remain, which
  the comparison tolerates. If they do not, the references are re-captured (ux-notes §12.2 step 1).
- App tests are slower (the worker re-spawns, 12 Mpx traces): the 12 Mpx and OOM tests are in a
  nextest group that runs alone.
