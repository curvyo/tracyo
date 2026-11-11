# ADR 0001: Product, licence, repository and the tracing library

**Status:** Accepted (customer, 2026-11-11). Records decisions the customer made before the first
spec; written down by the architect so later ADRs can point at them.

## Context

The customer wants the Trace tool of Curvyo as a small desktop app of its own. Five things were
decided by him, not by the team: the name, the licence, the repository, the UI framework and
which library does the tracing. Options considered at the time (as far as recorded): shipping the
tool inside Curvyo only (rejected: Curvyo is a whole vector editor, the point is a small app);
a Tauri/web UI like Curvyo's (rejected by the customer in favour of a native Rust UI); copying the
tracer into this repository (rejected: one tracer, maintained once, in `vectrace`).

## Decision

1. The product is called **Tracyo**; binary `tracyo`.
2. Licence **AGPL-3.0-or-later** for all code of this repository (`LICENSE`). Dependencies may be
   under any licence that is compatible with it (ADR 0007 lists the allowed ones).
3. Its own repository (this one). Curvyo (`~/curvyo`) is a read-only reference for look and
   behaviour; nothing is shared by path.
4. UI framework **egui** (with eframe). Which version and renderer: ADR 0003.
5. Tracing, OCR and shape recognition are done by **`vectrace`**
   (https://github.com/curvyo/vectrace, MIT OR Apache-2.0). Tracyo never reimplements or patches
   them; a needed change is a pull request to vectrace and Tracyo bumps the tag.

## Consequences

- A behaviour of the tracer that the spec needs and vectrace lacks blocks the story until a
  vectrace release has it. ADR 0004 is designed so that `vectrace` v0.1.0 is enough.
- The AGPL obliges a source offer to users: the About panel names the source address (spec
  criterion 26).
- egui is not a browser: Curvyo's look is reproduced, not reused (ADR 0003, ux-notes §12).
