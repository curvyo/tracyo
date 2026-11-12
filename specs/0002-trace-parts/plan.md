# Plan: 0002 Trace parts

Status: Skeleton (architect, 2026-11-12). The implementer of each slice fills in the tasks, keeps
them test-first for core logic and ticks them off. Each slice is one branch and one pull request the
customer can try end to end, opened against `main` after the slice before it is merged (no
stacking). Criteria numbers are those of `specification.md`; "ux §n" points at `ux-notes.md`;
ADRs at `docs/adr/`. **No slice needs a vectrace PR first**: vectrace v0.1.1's `Job::mask` is
enough (`adrs.md`, note of 2026-11-12).

## Slice 1: Rectangle regions and one masked part (`story/trace-parts-mask`)

Demo: open a picture, press Rectangle, draw rectangles (Shift from the centre, Ctrl square), with
Subtract pressed draw holes; select, move, resize, nudge, delete; Inside and Outside, Clear mask;
the outside is washed to 12 %, the outline ends exactly at the mask; Save as… writes an SVG cut at
the rectangle's edge. One part only (no Parts row yet).

- [x] Dependency `i_overlay =9.0.1` in `tracyo-core`, `deny.toml` check, `THIRD-PARTY.md`, Renovate rule (alone, no automerge) (ADR 0008 Consequences) — 26
- [x] `grid`, `regions`, `clip` test-first: snap, canonical order, union minus subtracted, ring, permutation gives the same bytes, empty mask, mask off the picture, `Mask` impl (whole outlines keep curves, crossing ones cut, refusal keeps whole), `kept_pixels` (ADR 0008 §1, §2) — 6, 12, 13
- [x] Property tests of the cut on a 6 px lattice: Inside + Outside = whole, no overlap, idempotent (ADR 0008 §6) — 13, 14 (Inside/Outside of one trace)
- [x] Golden `parts-13.svg` from P1 and R1; fixture generator for P1 and P2 — 13
- [x] `protocol`: `Trace` carries an optional clip (grid units, Inside/Outside); `job` passes it as `Job::mask`; round-trip tests; no clip = bit-identical to 0001 (ADR 0008 §4) — 13, 22
- [x] `region_edit` test-first: `drawn` (2 px rule, centre, square), `region_at` (topmost, 8 px slop), `handle_at` (radii, edge handles from 24 px), `resized` (centre, keep ratio, 1 px minimum), `moved` (ADR 0008 §5) — 7, 9
- [x] `wash`: not-traced area as disjoint rectangles, Inside and Outside, regions off the picture — 11
- [x] App: Mask group (Rectangle, Subtract, Inside/Outside segments, Clear mask; icons `square`, `square-minus` in `assets/` with hashes), Tab order, hidden without a picture (ux §1, §2, §8) — 1 (Mask group), 6, 10
- [x] App `region_tool` and `region_paint`: drawing with live outline, size chip and Remove badge, selection box and 8 handles, hover box, drag ghost, Esc restores a drag, arrows and Delete, cursors, Esc order, black 0.25 mm outline, dashed subtracted outline, 12 % wash mesh, the preview outline only inside the mask (ux §5 to §7) — 7, 8, 9, 11
- [x] Notices "the mask is outside the image" and "could not be combined" as the dimmed Save as… tooltip and hidden status text — 12
- [x] Region-only analysis of the one part (automatic Threshold, background, Analyse read the kept pixels; `adrs.md` note) — 18 (one part)
- [x] ux-engineer: `capture.cjs` with Rectangle and Subtract, re-capture `px-00` .. `px-17` (ux §0)
- [x] Kittest flows: draw, select, resize, nudge, delete, Esc order, Inside/Outside, Clear mask, save; pixel cases P1 to P6, P16, C1 to C4 (ux §12.2) — 1, 7 to 13

Done on `story/trace-parts` (implementer, 2026-11-12). Notes: the crossing test uses the exact
bounding box, not the control box (ADR 0008 deviation 1); P14 (`px-16`) is compared without the
Parts row, which comes with slice 2; the notices are the tooltip of the dimmed Save as… (the hidden
status text is debt 24); the dashed look of the selection box and of subtracted outlines is checked
by sampled pixels, not against a reference.

## Slice 2: Parts and the joined result (`story/trace-parts-join`)

Demo: Add part, give it its own rectangle and settings, switch between chips (mouse and keys),
remove a part with the armed trash; the preview and the saved SVG show one joined result without a
seam line; a later part wins where parts overlap; Stop during a save keeps the parts.

- [x] `parts` test-first: 1 to 8 parts, add copies the active part without colours, remove makes the part before active, select, open resets, part areas (Whole, Mask, Nothing) (ADR 0008 §3) — 2, 3, 4, 6, 23
- [x] `stack` test-first: later wins, untouched keeps its clip, trimmed, hidden; ports of Curvyo's `trace_parts` tests — 5, 16
- [x] `join` test-first: kept outlines, seam union per colour, single-part pieces not united, shapes follow or drop, failed part fails the join; property: join of Inside and Outside equals the whole trace (≤ 0.1 %, no seam segment) — 14, 15
- [x] Golden `parts-15.svg`; criterion 16 tests in both orders — 15, 16
- [x] `crop` test-first: box plus margin, exemptions (text, Drop border shapes, Denoise, Upscale Auto, whole, covering box); crop vs whole ≤ 0.1 % — 25 (prerequisite)
- [x] `protocol` `TraceParts` / `PartWire`; worker traces uncached parts from their crops, sends each wire, joins; progress weights; round trips (ADR 0008 §4) — 20, 21, 22
- [x] App: part cache (32 entries, key = part job bytes), latest wins, kill keeps received parts, reduced per part while dragging, Save as… traces all parts full — 20, 21, 24
- [x] App `parts_row`: label, chips with marks and muted state, Add part, trash armed (400 ms, 4 s, Esc, focus rules), keys; only part 1's settings saved; regions of other parts drawn, not editable (ux §3) — 1 (Parts row), 2, 3, 11, 23
- [x] Notice "no part has anything to trace"; tooltips and accessible names of ux §4 — 4, 11, 12
- [x] Determinism test: same parts twice, byte-identical SVG — 22
- [x] Kittest flows: add, select, remove, retrace counter (only the changed part), progress never backwards, Stop and failure; pixel cases P7 to P15 — 2, 3, 20, 21, 24

Done on `story/trace-parts-row` (implementer, 2026-11-12). Notes:

- `parts`: the app keeps the active part live in the bar and the mask; `Parts` holds snapshots and
  every list change takes the live copy first (`add`, `select` and `remove_active` return the part
  to load). `Part` also carries `threshold_set` and a plain `Analysis`.
- `stack` tests are written for this code, not literal ports of Curvyo's `trace_parts` tests (those
  run on Curvyo's session and kernel); they check the same rules: later wins, untouched parts keep
  their clip, trimmed parts get the visible polygons, hidden parts get no job.
- `join` needed a writer for vectrace's numbers (`output_wire`, 220 lines, read back by
  `vectrace::wire::read` in four round-trip tests), because the worker joins and vectrace has no
  writer for an `Output`.
- Property tests of the join: 2,000 cases each for area, no seam line, determinism, fixed point
  and the order rule; criterion 14 to 16 and 22 on traced P1 through `Session::run_parts`.
- The crop is made by the worker from the clip and the raster it traces (reduced or full), so
  `PartJob` has no crop field (ADR 0008 deviation 6); `Session::run` crops too.
- The mark on a chip is the mask mode on every chip but a muted one, part 1 included (px-08 shows
  it on a single whole-image part; ux-notes section 3's "none = whole image" is superseded by the
  picture).
- Criterion 25's timing (the plan puts it in slice 4) is already in `parts_speed.rs`: 1.03 times for
  8 tiled parts of 12 Mpx, 0.98 times for 8 covering parts (one is traced), release, thread CPU time.
- Open in this slice: debts 25 to 30; the analysis per part clip (18), Multicolour (17) and text
  and shapes (19) are slice 3.

## Slice 3: Colours, text and shapes per part (`story/trace-parts-colour-text`)

Demo: Multicolour in two parts gives one layer per colour; part 2 on the darker right half finds its
own background; Recognise text in one part and not in the other; a rectangle cut by a seam is a path,
one wholly inside is a `<rect>`.

- [ ] Join of multicolour parts: layers by exact RGB, largest first, single-colour parts as `#000000`, palette of the active part; golden `parts-17.svg` — 17
- [ ] Region-only analysis per part clip, cached per clip, only unset values filled (`adrs.md` note) — 18
- [ ] Text per part: lines filtered by each part's clip (box centre), whole-picture trace for parts with text, OCR once per picture; P3 fixture with its source noted — 19
- [ ] Shapes after the cut; shape regions dropped when their outline went into a seam union — 19
- [ ] Kittest flows for 17 to 19 with the real worker — 17, 18, 19

## Slice 4: Speed, the last pixels and polish (`story/trace-parts-polish`)

Demo: a 12 Mpx photo in 8 parts traces about as fast as one part; editing 8 parts with 20 rectangles
stays smooth; every bar state matches Curvyo's pictures.

- [ ] `perf` job: criterion 25 in-process in `tracyo-core` (thread CPU time, 12 Mpx, 8 tiled parts vs one trace; 8 covering parts), the result reported by the tester, a miss to `docs/technical-debt.md` with the customer — 25
- [ ] `perf` job: UI frame budget with parts tracing and with 8 parts × 20 regions being edited — 25, 27
- [ ] Invalid geometry never panics (fuzz-style cases through `Clip::of_regions` and `join`) — 27
- [ ] Remaining pixel cases and announcements ("Rectangle on", "Subtract on", "3 parts", Save as… tooltip with parts); expander "Changed: 3 parts" — 1, 11
- [ ] No-network session test with parts (0001/23 unchanged) — 26
- [ ] Docs: `docs/technical-debt.md` entries for whatever stays open; ADR 0008 status to Accepted with its deviations — DoD

## Risks

See `adrs.md` "Risks". The two that can move the plan: the re-capture of the `px-*` references needs
Curvyo's dev server (blocks the pixel tests of slice 1, not its code), and a crop that is not local
for some setting (would widen the exemptions of ADR 0008 §4 and cost speed in criterion 25).
