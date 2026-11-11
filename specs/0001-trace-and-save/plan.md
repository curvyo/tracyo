# Plan: 0001 Trace and save

Status: Skeleton (architect, 2026-11-11). The implementer of each slice fills in the tasks, keeps
them test-first for core logic and ticks them off. Each slice is one branch `story/<slug>` and one
pull request the customer can try end to end. Criteria numbers are those of `specification.md`;
"ux §n" points at `ux-notes.md`; ADRs at `docs/adr/`.

## Slice 1: Open, trace as Logo, preview, save (`story/trace-and-save-basic`)

Demo: `tracyo logo.png` or Ctrl+O or drop; the picture with the blue outline; drag Smooth,
Tolerance, Ignore below and Threshold; Stop with Esc; Save as… writes an SVG that opens in Inkscape
with its size in mm. Method is Brightness only; no Advanced block yet.

- [x] Workspace, two crates, lints, profiles, `rust-toolchain.toml`, `deny.toml`, CI (`ci.yml`, `nightly.yml`), Renovate (ADR 0002, 0007) — 23a, 25
- [x] `picture`: decode PNG/JPEG/WebP with EXIF orientation, errors, reduced copy (ADR 0003 §5, 0004 §5) — 1, 2, 4, 7
- [x] `settings`: Logo preset and the six preset values, Smooth/Tolerance/Ignore below/Threshold ranges and steps, TOML round trip, corrupt file (ADR 0003 §8) — 11 (Logo only), 13, 18 (those values)
- [x] `protocol`, `job`, worker mode, runner with kill and re-spawn, out-of-memory reason (ADR 0004) — 7, 8, 24
- [x] `progress`: 400/300 ms rule, Stopped, Failed, Retry, Esc order; nothing-to-save reasons of Brightness (`too low`/`too high`, `Too much detail`) — 8, 9, 15 (part), 22
- [x] `svg`: frame, namedview, one `#000000` layer, path, numbers; goldens, XML-structure test, resvg coverage test (ADR 0005) — 20a-e, 20h, 21 (automatic)
- [x] App: window and title, open card, empty state, canvas (pan, zoom, fit, tiles), preview meshes, bar with About button (no panel yet), Preset (menu), Method button (Brightness), value fields, expander placeholder hidden, blue button states; theme tokens and fonts (ADR 0003) — 1, 3, 5, 6 (outline), 10, 11, 13, 19, 22
- [x] Dialogs (rfd) and atomic write; save traces the full picture first — 19
- [x] kittest end-to-end (open by argument, trace, stop, retry, save), frame-budget CPU-time test, OOM test, `unshare -n` session test, Xvfb start test — 7, 8, 19, 23b, 24
- [x] Pixel tests: px-00, px-01, px-02 (bar), px-05 (bar left of the blue button) and px-08 (menu), Rust port of `compare.py` with its self-tests (ux §12) — see the notes below
- [ ] Pixel tests px-06, px-07 and px-13: their references have the Advanced block open, so the blue button sits at another place; they come with slice 2, when the block exists

Notes of the implementer (slice 1, branch `story/open-trace-save`):

- The pixel comparison covers the bar rectangle only (the 12 px margin of the references holds
  Curvyo's own page, a darker ground and "Page 1", which Tracyo does not have); the Open card
  (x 12 to 60) is cut away before the bar is found. Measured: px-00 0.56 % (longest run 8 px),
  px-01 1.57 % (8), px-02 0.56 % (8), px-05 left part 0.39 % (8), px-08 0.30 % (8); the limits are
  those of ux-notes 12.2 (2 %, run under 12), nothing was tuned.
- px-05 whole row: the reference shows the running button 75 px wide, ux-notes 3.3 keeps it 89 px;
  compared as it is the row differs by 1.95 % with runs of 28 px along both button edges.
- px-00 to px-02 differ from the reference only at glyph edges, the "mm" unit, the corners and the
  slot of the expander (reserved and empty in this slice).
- The menu opens 8 px below its button (the reference), ux-notes 5 says 4 px.
- Debt added: `docs/technical-debt.md` 1 to 8.


## Slice 2: Methods and Advanced settings (`story/trace-methods-advanced`)

Demo: Remove background and Pick colours with pipettes; the Advanced block with Invert, Drop
border shapes, Denoise, Upscale, Corners, Analyse; all presets; every reason a save is dimmed;
the values come back after a restart.

- [ ] Method menu, Threshold vs Colour tolerance per method, background chip, hex, Auto, pipettes, picked swatches (up to 8) — 12
- [ ] Analyse job and the one-time automatic analysis that never overrides a changed value — 14 (analysis part)
- [ ] Advanced block with Invert, Drop border shapes, Denoise, Upscale, Corners and the Analyse button (Output comes in slice 3, Recognise text and shapes in slice 4); expander remembered, changed-dot — 14
- [ ] All presets, "Custom" after an edit — 11
- [ ] All nothing-to-save reasons — 15
- [ ] Settings file complete — 18
- [ ] Pixel tests px-03, px-09, px-12, px-14

## Slice 3: Multicolour and excluded colours (`story/trace-multicolour`)

Demo: Output Multicolour, Colours Auto or 2 to 16, the palette largest first, delete and Ctrl+click
exclude; the SVG has one labelled layer per colour, largest at the bottom.

- [ ] Output segmented control, Colours field with "Auto  N", palette swatches, pipette, delete, exclude, keys — 14 (multicolour part), 15 (`No colours found.`, `no colour in the palette`)
- [ ] `svg`: one layer per colour, order, excluded and deleted colours, never an empty layer; goldens — 20d
- [ ] Coverage test for multicolour; manual Inkscape check list prepared — 21
- [ ] Pixel tests px-04, px-10, px-11

## Slice 4: Recognise text and Recognise shapes (`story/trace-text-shapes`)

Demo: a logo with text and a circle; "Reading text" with Stop; the preview shows recognised text
outlines and shapes; in Inkscape the rectangle and circle are editable and the text is text.

- [ ] Assets: model copied from vectrace v0.1.0, `ASSETS.md`, hash test (ADR 0006) — 16, 23
- [ ] ReadText job, lines kept per picture and scaled for the reduced copy, "Reading text" progress — 16
- [ ] Shapes flag in the trace request; preview of shapes and text outlines — 6, 17
- [ ] `svg`: primitives, bands, regions with holes, polygons and stars as paths, text elements, escaping; goldens — 20f, 20g
- [ ] `unshare -n` session test with text and shapes on — 23b

## Slice 5: Polish to the pixel references, About, platforms, releases (`story/trace-polish`)

Demo: the app next to Curvyo's pictures; About panel; tooltips, hover and keyboard focus as in
Curvyo; builds for Windows and macOS.

- [ ] About panel with licences and source address — 26
- [ ] House tooltips, focus rings on keyboard focus only, full keyboard map and tab order (ux §3.4, §10), notice slot texts (ux §8), drop cue, status bar
- [ ] Fonts per OS (ADR 0003 §6), checked on Windows and macOS nightly — 25
- [ ] All pixel tests green, including px-15; diffs reviewed for glyph-edge-only red (ux §12.2 step 4)
- [ ] `release.yml` and a first tagged test build (signing per Frage 6) — 25
- [ ] Manual Inkscape and second-reader check of criterion 21 recorded in the PR
