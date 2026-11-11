# Architecture Decision Records

One flat numbering for the whole repository. Format and rules: Curvyo's
`docs/guides/rust-workspace-blueprint.md` §2 (Context with the rejected options, Decision,
Consequences). Once an ADR is accepted its decision text is not edited; a change is a new ADR that
supersedes it. A decision too small for an ADR is a dated note in the feature's
`specs/<NNNN-slug>/adrs.md`.

| ADR | Title | Scope | Status |
|---|---|---|---|
| [0001](0001-product-licence-and-repository.md) | Product, licence, repository and the tracing library | workspace | Accepted (customer, 2026-11-11) |
| [0002](0002-workspace-and-crate-layout.md) | Workspace and crate layout | workspace | Proposed |
| [0003](0003-ui-stack-and-look.md) | UI stack, platform services and how Curvyo's look is reproduced | tracyo-app | Proposed |
| [0004](0004-trace-worker-process.md) | The trace runs in a worker process; cancel, progress and preview | workspace | Proposed |
| [0005](0005-inkscape-svg-output.md) | The Inkscape SVG Tracyo writes | tracyo-core | Proposed |
| [0006](0006-bundled-assets.md) | Bundled assets: OCR model, fonts and icons | workspace | Proposed |
| [0007](0007-tests-ci-dependencies-and-releases.md) | Tests, CI, dependency policy and release builds | workspace | Proposed |
