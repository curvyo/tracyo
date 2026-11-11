# CLAUDE.md

Tracyo: a standalone Trace app (egui, Rust) on the `vectrace` library, looking like the Trace
tool of Curvyo (`~/curvyo`, read-only reference for look and behaviour). Short on purpose.

## Language and reporting
- Everything in the repo is English. Plain, specific, no marketing.
- Talk to the customer in German, informal "du". Final reply: exactly three short lines
  (status 🟢/🟡/🔴 with the exact check; what the customer must do; what is left).
- Questions only when a decision is the customer's; own block `Frage <n> (<KURZ-ID>): Titel`,
  options A/B/C, recommendation, default.

## Roles and workflow
Lead (main session) plans, delegates, integrates, opens PRs. Roles and their playbooks are those
of Curvyo: `~/curvyo/docs/playbooks/<role>.md` (product-owner, architect, ux-engineer,
implementer, tester, refactorer). Subagents report in ~30 lines, never spawn subagents, never open
PRs. One feature folder `specs/<NNNN-slug>/`: `specification.md`, `adrs.md`, `ux-notes.md`,
`plan.md`. Branches `story/<slug>`, `fix/`, `chore/`, Conventional Commits, rebase-merge, never
push to `main`.

## Engineering rules
- One responsibility per module, doc comment says it without "and". Delete dead code.
- Errors: `thiserror` in libraries, `anyhow` only in the binary. No `unwrap`/`expect` outside
  tests unless preceded by `// invariant: ...`. Units are newtypes, comparisons have tolerances.
- No trait without a second implementation, no speculative options. A new dependency must pass
  `cargo deny`; justify it in the PR.
- No hint/tip text in the UI; labels that name a control or state stay.
- Tracing, OCR and shape recognition belong in `vectrace`, not here. A needed change goes to
  https://github.com/curvyo/vectrace as a PR; Tracyo bumps the tag.

## Quality gate (before every push)
```text
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo nextest run --workspace --status-level fail
cargo deny check
RUSTDOCFLAGS="-D warnings" cargo doc -q --workspace --no-deps
```
