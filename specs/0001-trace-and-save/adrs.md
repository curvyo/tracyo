# ADRs for 0001 Trace and save

Status: Proposed (architect, 2026-11-11). Everything below is vetoable at the demo unless marked
as the customer's.

## ADRs this feature rests on

| ADR | What it decides for this feature |
|---|---|
| [0001](../../docs/adr/0001-product-licence-and-repository.md) | Tracyo, AGPL-3.0-or-later, own repository, egui, vectrace does tracing, OCR and shapes (customer) |
| [0002](../../docs/adr/0002-workspace-and-crate-layout.md) | `tracyo-core` (pure: settings, picture, request, protocol, job, progress, svg) and `tracyo-app` (egui, worker, dialogs, settings file) |
| [0003](../../docs/adr/0003-ui-stack-and-look.md) | eframe/egui 0.37.0 on wgpu; X11 preferred on Linux for file drop; rfd portal dialogs; `image` PNG/JPEG/WebP with EXIF orientation; system UI font per OS with embedded Liberation Sans (Frage 5 A); lucide icons rasterised by resvg; `theme` module; TOML settings in the config folder |
| [0004](../../docs/adr/0004-trace-worker-process.md) | criteria 7, 8, 9, 16, 24: the trace, analysis and OCR run in `tracyo --trace-worker`, killed to stop; 1000 px reduced copy while dragging and over 4 Mpx; 400/300 ms progress rule; cached preview meshes |
| [0005](../../docs/adr/0005-inkscape-svg-output.md) | criterion 20: the SVG writer, layers, primitives, text, numbers |
| [0006](../../docs/adr/0006-bundled-assets.md) | criteria 16, 23, 26: model, fonts and icons in `assets/`, embedded, hash-checked |
| [0007](../../docs/adr/0007-tests-ci-dependencies-and-releases.md) | criteria 20, 21, 23, 24, 25 and ux-notes §12: kittest, golden files, pixel comparison in Rust, `unshare -n`, `deny.toml`, CI, Renovate, releases |

## Notes (too small for an ADR)

**2026-11-11, vectrace v0.1.0 is enough.** `job::run` cannot be cancelled; ADR 0004 stops it by
killing the worker process, so no vectrace change is needed for this feature. Calls on the UI
thread are limited to O(1) ones (`sample_colour` for the pipette, the schema and preset tables);
anything over the whole picture (`analyse`, `auto_threshold`, traces, OCR) runs in the worker.

**2026-11-11, recognised regions with holes.** Only hole-free filled regions and bands become
SVG primitives; a region with holes keeps the primitives' geometry inside the layer's `<path>`
(ADR 0005 §6). For the product owner: criterion 20f could say so. **(Proposal)**

**2026-11-11, picture limits (for the product owner).** The spec names no limits. Proposal: take
Curvyo's import limits (`curvyo-image-core/src/limits.rs`): files up to 100 MiB, headers up to
50 Mpx, larger ones "Not enough memory to open this image."; pictures over 16 Mpx or 8192 px per
side are reduced on open to that working size, the SVG keeps the original size in mm. Until the
spec decides, the implementer applies the first two and does not reduce.

**2026-11-11, the Analyse job sets the automatic Threshold.** `auto_threshold` and `analyse` run
together in the worker once per picture; until the reply the Threshold field shows the last value
and the first trace of a picture waits for the analysis, so it is traced once with the proposed
values. **(Proposal)**

## Risks

| Risk | Effect | Mitigation |
|---|---|---|
| egui's glyph rasteriser differs from Skia's | pixel tests fail on text, not on boxes | fixed item widths (ux §3.2), explicit line heights, same font file; comparison tolerates glyph edges; re-capture references on the CI font if needed |
| Preview of a 100k-node result tessellated by egui on the UI thread | frames over 50 ms during zoom (criterion 7) | mesh cached per result and zoom, translated on pan, view culling; measured in slice 1; fallback wgpu paint callback (ADR 0004 §7) |
| wgpu headless in CI (lavapipe/llvmpipe) renders slightly differently from this machine's EGL path | pixel tests flaky between machines | tolerance of ux §12.2; artefacts kept; references re-made on the CI image if the share is systematically high |
| wgpu GL fallback limits textures to 2048 px | large pictures fail to show | tiles of at most 2048 px from slice 1 |
| `sudo unshare -n` on GitHub runners | no-network test cannot run | the step uses sudo; if a runner forbids it, the test runs on the tester VM and the PR says so |
| Worker re-spawn per cancelled job while dragging | stutter on slow machines | reduced copy (4 MB) per job, picture sent once per worker; measured in slice 1 |
| Wayland without XWayland | no file drop | Ctrl+O and the command line always work; debt #3 |
| No xdg-desktop-portal on minimal Linux desktops | Open and Save dialogs do not appear | debt #4; command-line open still works |
| Binary size about 29 MB (wgpu, OCR model) | larger download | reported, not limited (`CLAUDE.md`); glow would save 4 MB, a model download would break criterion 23 |

## Questions for the customer

**Frage 6 (SIGNING): Signed release builds?**
Unsigned builds work, but macOS shows "cannot be opened because the developer cannot be
verified" (right-click > Open once) and Windows SmartScreen warns. Signing needs paid accounts.
- A: unsigned builds, the release notes say how to open them.
- B: sign and notarise for macOS (Apple Developer Program, 99 USD per year, an Apple ID of yours).
- C: B plus a Windows code-signing certificate (about 200 to 400 USD per year).
Recommendation A until the first public release. Default: A.
