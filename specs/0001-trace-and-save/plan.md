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
- [x] kittest end-to-end (open by argument, trace, stop, retry, save), frame-budget CPU-time test (timed in release, CI job `perf`), OOM test, `unshare -n` session test, Xvfb start test — 7, 8, 19, 23b, 24
- [x] Pixel tests: px-00, px-01, px-02 (bar), px-05 (bar left of the blue button) and px-08 (menu), Rust port of `compare.py` with its self-tests (ux §12) — see the notes below
- [x] Pixel tests px-06, px-07 and px-13: their references have the Advanced block open; done with slice 2, when the block exists (see the slice 2 notes)

Notes of the implementer (slice 1, branch `story/open-trace-save`):

- The pixel comparison covers the bar rectangle only (the 12 px margin of the references holds
  Curvyo's own page, a darker ground and "Page 1", which Tracyo does not have); the Open card
  (x 12 to 60) is cut away before the bar is found. Measured: px-00 0.56 % (longest run 8 px),
  px-01 1.57 % (8), px-02 0.56 % (8), px-05 left part 0.39 % (8), px-08 0.30 % (8); the limits are
  those of ux-notes 12.2 (2 %, run under 12), nothing was tuned.
- px-05 whole row: the reference shows the running button 75 px wide, ux-notes 3.3 kept it 89 px;
  compared as it was the row differed by 1.95 % with runs of 28 px along both button edges
  (resolved in slice 2: the button is 75 px).
- px-00 to px-02 differ from the reference only at glyph edges, the "mm" unit, the corners and the
  slot of the expander (reserved and empty in this slice).
- The menu opens 8 px below its button (the reference); ux-notes 5 said 4 px (corrected in slice 2).
- Debt added: `docs/technical-debt.md` 1 to 8.


## Slice 2: Methods and Advanced settings (`story/methods-advanced`, stacked on `story/open-trace-save`)

Demo: Remove background and Pick colours with pipettes; the Advanced block with Invert, Drop
border shapes, Denoise, Upscale, Corners, Analyse; all presets; every reason a save is dimmed;
the values come back after a restart.

- [x] `method`: Method, per-picture values (Threshold, Invert, Drop border shapes, background Auto or picked, up to 8 picked colours), the tracer's selection, expander's "Changed: ..." text; unit tests first — 12, 14
- [x] `analysis`: vectrace's analysis reduced to method, Threshold, background and Colour tolerance; applied when a picture opens (the picture is shown only after it answered, so no value can be changed before it) and by Analyse; with Pick colours only the Threshold is set — 14
- [x] Trace request and protocol carry the selection, Invert and Drop border shapes; job tests for both methods, Invert, Drop border shapes — 4, 12
- [x] `settings`: method, each method's Colour tolerance, Advanced open, Corners field; Custom after any of the six values, a pick restores; file tests — 11, 18
- [x] Reasons of the colour methods, "0 paths: no colour picked." — 12, 15
- [x] App: Method menu with three rows, Colour tolerance field, background chip, hex, Auto, pipettes (canvas chip, crosshair, click replaces, Shift adds, right press and Esc disarm), picked swatches with x, expander with changed-dot and tooltip, Advanced block (toggles, Denoise, Upscale, Corners, Analyse), blue button states of the reference — 12, 14, 9
- [x] Kittest flows with the real worker (`tests/methods.rs`): analysis choice, Pick colours, background pipette, tolerance per method, block and restart, presets after Custom, Analyse, second picture — 1, 11, 12, 14, 18
- [x] Pixel tests px-03, px-05 (whole row), px-06, px-07, px-09, px-12, px-13, px-14, by regions (ux-notes 12.2 step 6)
- [x] Output, Colours and the palette moved to slice 3 (px-04, px-10, px-11 are done there); Recognise text and shapes are slice 4, px-15 slice 5

Notes of the implementer (slice 2, branch `story/methods-advanced`):

- Pixel numbers (share of differing pixels, longest run; limit 2 % and under 12): px-00 0.51 %
  (8), px-01 1.57 % (8), px-02 0.51 % (8), px-03 row 0.39 % (8), toggles 0.56 % (1), settings
  0.74 % (9), analyse 0.21 % (4); px-05 row 0.41 % (8), analyse 0.74 % (10); px-06 rows 0.25 %
  (9); px-07 rows 0.24 % (8); px-08 0.30 % (8); px-09 rows 0.34 % (8), menu 0.18 % (4); px-12
  0.73 % (8); px-13 row 0.42 % (8); px-14 row 0.40 % (8). Divider and gap regions 0 %. Nothing
  was tuned.
- px-05 now compares the whole first row: the running button is 75 px wide (lead's decision 1).
  The menus open 8 px below their button and the expander has its slot filled (decisions 2, 3);
  ux-notes 3.3, 5, 12.2 and 13 are corrected.
- The bar width rule was measured on the references, not taken from ux-notes 3.3: all items on one
  line plus one gap while the block is open, at most the cap; a wrapped bar takes the whole cap.
  The references have a cap of 1164 px; px-06, px-07 and px-09 run in a 1248 px window.
- The analysis runs on a thread of the app (as slice 1 ran `auto_threshold`), not in the worker
  process of ADR 0004 section 2: killing the worker for every dragged value would kill the analysis.
  Debt 6.
- The logo's analysis is Remove background, so a first start shows it, as Curvyo does (px-09);
  the session golden file `logo-session.svg` is the trace of that.

## Slice 3: Multicolour and excluded colours (`story/multicolour`, stacked on `story/methods-advanced`)

Demo: Output Multicolour, Colours Auto or 2 to 16, the palette largest first, delete and Ctrl+click
exclude; the SVG has one labelled layer per colour, largest at the bottom.

- [x] `palette` (core): the palette strip order, colour names, same-colour test, delete, exclude and pick edits, swatch keys, the tracer's colour request; unit tests first — 14
- [x] `settings`: Output and Colours (Auto at position 1, 2 to 16), remembered, clamped; request, protocol and job carry the palette; job tests for Auto, exclude, count, deleted palette — 14, 18
- [x] Reasons: "No colours found.", "0 paths: no colour in the palette." (nothing is traced when every colour was deleted) — 15
- [x] `svg`: one layer per colour in the result's order, never an empty layer, ids unique; goldens `three-colours.svg` and `multicolour-shapes.svg`, structure tests, resvg colour and hole check — 20d
- [x] Coverage test for Multicolour (second reader draws every colour where the picture has it, covered area equals the outlines' area); manual Inkscape check list stays that of slice 5 — 21
- [x] App: Output pill, Colours field ("Auto 3", dimmed with Pick colours), palette pipette, palette swatches (hover x, Ctrl+click exclude with the crossed circle, keys, tooltip with colour name), the palette at half strength while a trace runs or is stopped — 14
- [x] Pixel tests px-04, px-10, px-11 and the whole-bar comparison of px-03, 05, 06, 07, 13, 14 (all but the two Recognise toggles of slice 4)

Notes of the implementer (slice 3, branch `story/multicolour`):

- Pixel numbers (limit 2 % and under 12): see the table in the report of the lead; each region of each test
  prints its own line. Output row (pill to Drop border shapes): px-03 0.48 % (4), px-04 0.58 % (10), px-05 0.58 % (10), px-06
  0.60 % (10), px-07 0.58 % (10), px-10 0.58 % (10) and its tooltip 0.49 % (6), px-11 0.60 % (10), px-13 0.60 % (10),
  px-14 0.60 % (10). First row(s), divider, Denoise to Corners, gap and Analyse as in slice 2. Nothing was tuned; the reference
  decided: the pill's outline is the chrome colour at 40 % drawn over the blue segment too, the palette at half strength is
  flattened per swatch (not group opacity), the tooltip ring is the foreground at 10 % outside the box and its second line
  sits in a 13 px line.
- The two Recognise toggles (Recognise text, Recognise shapes) belong to slice 4; the references draw them, Tracyo does not
  yet, so the tests skip exactly those 64 px of the block row and compare what follows shifted by them (debt 7).
- The Colours field shows "Auto 3" in one colour with one space, as the reference renders it (ux-notes 4 said two spaces and a
  muted count; the HTML collapses the spaces and Curvyo does not mute). A field with its bar at 0 draws no bar and no edge.
- The multicolour preview is the same blue outline as with Single colour (Curvyo draws no layer colours in its preview); the
  colours show in the palette row and in the saved file.
- Bug found by the real-app run on `blütenzeit.png`: epaint's closed strokes keep every miter, so an outline that folds back
  (two colours sharing a one-pixel sliver) drew a spike thousands of pixels long and covered the canvas; the preview now
  strokes closed outlines as open polylines, which epaint cuts at sharp corners (unit test with a sliver).
- Analysis in the worker process (ADR 0004 section 2) is not done: the worker is killed for every dragged value, so an
  analysis in it would be killed too, and a second worker process only for Analyse is a change of that ADR (architect).
  Debt 6 stays.
- Ctrl+click needs `modifiers.command` in the frame of the release; tests send press and release in frames of their own
  (`common::ctrl_click`), because egui_kittest queues a click's events into one frame whose modifiers end empty.

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
