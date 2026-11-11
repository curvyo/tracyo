# ADR 0002: Workspace and crate layout

**Status:** Accepted (lead, slice 5, 2026-11-11; proposed by the architect 2026-11-11).

Deviation: `tracyo-core` has more modules than listed (`analysis`, `method`, `palette`, `shapes`, `stages`, `text`; each pure, none needs I/O) and `tracyo-app` has `advanced`, `controls`, `groups`, `methods`, `stage` besides the ones named (UI only). No new crate.

## Context

Tracyo is one window, one picture, one output format. Most of its logic (settings and presets,
the trace request for a picture, the progress state machine, the Inkscape SVG writer, picture
decoding) can be tested without a window, and those tests should not compile egui and wgpu
(about 1 minute of extra build per clean test run, measured in the spike). The UI, the file
dialogs, the settings file and the worker process need the platform.

| Option | Verdict |
|---|---|
| **A. Two crates: `tracyo-core` (pure) and `tracyo-app` (egui binary) (chosen)** | The core/platform split of the Curvyo blueprint at its smallest. Core tests build in seconds; the boundary is enforced by the dependency graph, not by review. |
| B. One crate | Simplest, but every logic test compiles the UI stack, and nothing stops a module from reaching for the file system or the clock. |
| C. More crates (`tracyo-svg-core`, `tracyo-worker`, `tracyo-io`) | No second user for any of them; a separate worker binary would break "one binary" (spec criterion 25, ADR 0004). Split later if a module outgrows its crate. |

## Decision

```
Cargo.toml              workspace: members, [workspace.package], [workspace.lints], profiles
rust-toolchain.toml     1.99.0 (as vectrace), rustfmt + clippy
deny.toml  renovate.json  .github/workflows/
assets/                 OCR model, fonts, icons with ASSETS.md (ADR 0006)
crates/tracyo-core/     lib, pure
crates/tracyo-app/      lib + [[bin]] name = "tracyo"
docs/adr/  docs/technical-debt.md  specs/
```

**`tracyo-core`**: no file system, network, clock, threads, processes or UI. Callers pass bytes,
times (`Duration` since an origin the app chooses) and sizes in. `#![forbid(unsafe_code)]`.
Modules (one responsibility each):

| Module | Responsibility |
|---|---|
| `settings` | the remembered values, presets (from `vectrace::PRESETS`), ranges and steps, TOML text in and out |
| `picture` | bytes to an upright RGBA8 picture (EXIF orientation), its read errors, the 1000 px reduced copy |
| `request` | the trace request a picture and the settings make (`TraceParams`/`ColourParams`, reduced or full, OCR lines) |
| `protocol` | the worker's request and reply frames as bytes, both directions (ADR 0004) |
| `job` | runs one decoded request with vectrace (`job::run`, `ocr::read_text`, `analyse`) and reports progress through a callback |
| `progress` | the state of the blue button: idle, running (400 ms / 300 ms rule), stopped, failed, nothing to save, with its reason text |
| `svg` | the Inkscape SVG of a result (ADR 0005) |

Dependencies: `vectrace`, `kurbo` (vectrace's public geometry type), `image`, `serde`, `toml`,
`thiserror`. Dev: `roxmltree`, `resvg`.

**`tracyo-app`**: `main` (dispatches `--trace-worker` to the worker loop before any window code,
ADR 0004), `worker` (stdin/stdout loop around `tracyo_core::job`, the embedded model), `runner`
(spawns, feeds, reads and kills the worker process), `theme` (tokens of ux-notes §2), `widgets`
(the custom controls), `bar`, `canvas`, `preview` (outline meshes), `about`, `dialogs` (rfd),
`config` (settings file in the config folder), `icons`, `fonts`. It is a library plus the binary
so that `egui_kittest` tests drive the real app. `anyhow` only in `main.rs`.

Workspace lints: `unsafe_code = "forbid"` (spec criterion 25), `clippy::pedantic` warn,
`clippy::unwrap_used` deny, `missing_docs` warn; the gate treats warnings as errors. Profiles:
release `lto = true`, `codegen-units = 1`, `strip = true`; dev builds `vectrace`, `rten*` and
`sha2` at `opt-level = 3` (as vectrace does; OCR and 12 Mpx traces are minutes long unoptimised).

`tracyo-core` is not built for wasm32: Tracyo has no web target (no speculative targets).

## Consequences

- A new module in core must not need I/O; anything that does goes into the app.
- The app crate is the only one that links egui, wgpu, rfd, fontdb, resvg and the model bytes.
- Crate names do not follow Curvyo's `-io` split: the app is the only I/O user. A third crate needs
  a new ADR.
