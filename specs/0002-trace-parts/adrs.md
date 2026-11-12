# ADRs for 0002 Trace parts

Status: Proposed (architect, 2026-11-12). Everything below is vetoable at the demo; nothing here is
a `needs-customer` decision.

## ADRs this feature rests on

| ADR | What it decides for this feature |
|---|---|
| [0002](../../docs/adr/0002-workspace-and-crate-layout.md) | still two crates; the new modules of ADR 0008 go into `tracyo-core` (pure) and `tracyo-app` (UI) |
| [0003](../../docs/adr/0003-ui-stack-and-look.md) | egui look of the Mask group, the Parts row and the canvas glyphs; new lucide icons rasterised as before |
| [0004](../../docs/adr/0004-trace-worker-process.md) | criteria 20, 21: still one worker process, killed to cancel; ADR 0008 §4 extends §2 (requests `TraceParts`, reply `PartWire`), §3 (the app keeps part results across a kill) and §5 (reduced copy per part) |
| [0005](../../docs/adr/0005-inkscape-svg-output.md) | criteria 17, 21: the joined result is written as one result, layers per colour, unchanged writer |
| [0006](../../docs/adr/0006-bundled-assets.md) | icons `square`, `square-minus`, `plus`, `trash-2` added to `assets/icons/lucide/` with hashes |
| [0007](../../docs/adr/0007-tests-ci-dependencies-and-releases.md) | goldens, kittest, pixel comparison, `perf` job, `deny.toml`; Renovate gets an `i_overlay` rule (alone, no automerge) |
| [0008](../../docs/adr/0008-trace-parts-cut-and-join.md) | criteria 5, 12 to 16, 18, 20 to 22, 25, 27: cut and join in `tracyo-core` on `i_overlay` `=9.0.1` (0.001 mm grid), rectangle regions with integer pixel corners, the parts state, one worker job with all parts, crop rules, the egui interaction modules, the tests |

## Notes (too small for an ADR)

**2026-11-12, no vectrace change.** vectrace v0.1.1's `Job::mask` applies the mask in all job paths,
keeps text lines by their box centre and recognises shapes after the clip, which is what criteria 13
and 19 need. A vectrace PR is needed by no slice.

**2026-11-12, rectangle corners on whole pixels (for the product owner).** Rounded while drawing,
moving and resizing; a resize stops at 1 px. So no rectangle has zero width or height, and the
notice "could not be combined" is left for a kernel refusal only. The size chip shows whole pixels.
**(Proposal)**

**2026-11-12, Subtract applies to rectangles drawn while it is pressed.** It never flips a region
already drawn and is not released by Esc (Esc releases Rectangle). **(Proposal)**

**2026-11-12, criterion 25's "8 parts each covering the whole picture".** Later parts hide the
earlier ones completely, so the hidden parts get no job (ADR 0008 §3): that case costs about one
trace, well under 9 times. The tester reports the measured factor.

**2026-11-12, region-only analysis.** Each part's automatic Threshold, background and analysis read
the pixels its clip keeps (`Clip::kept_pixels`, then Curvyo's `trace_region_defaults` rule: Otsu
over the kept pixels, the background from the kept border pixels, `analyse` on the kept box with the
rest transparent). It runs on the analysis thread of debt 6, once per clip, and only fills values the
maker has not set (Curvyo `0094` slice 2 note).

## Risks

| Risk | Effect | Mitigation |
|---|---|---|
| `i_overlay` is a large dependency with internal `unsafe` | a bug in it is a wrong cut | exact pin shared with Curvyo (proven by its 5,877-case property tests); Tracyo's property tests with coincident edges; bumps reviewed, never automerged |
| A cropped part is traced differently near the crop edge | a step or sliver at a seam | crop only for settings whose fit is local (ADR 0008 §4); crop test ≤ 0.1 % per allowed setting; spike: byte-identical on P1 |
| Pieces of two parts with different settings meet the seam at different heights | a visible step at the seam (no gap, no overlap) | expected; the join closes the shared part of the seam; shown at the demo |
| Cut shapes become polylines | more nodes in the SVG | criterion 13 asks for it; the same as Curvyo |
| Clips and the wash are computed on the UI thread | a slow frame with many regions | at most 160 rectangles; criterion 27's frame test in the `perf` job |
| The `px-*` references must be re-captured with Curvyo's dev server | pixel tests cannot run until then | slice 1 task for the ux-engineer; `capture.cjs` change listed in ux-notes §0 |
| Subtract has no Curvyo reference picture | its look is unverified by pixels | P16 compares it with the bar's own pressed Rectangle (ux-notes §12.2) |
| 8 analyses of a 12 Mpx picture on the analysis thread (debt 6) | values arrive late, cannot be stopped | once per clip, cached by clip; debt 6 stays open |
| Two implementations of cut and join (Curvyo and Tracyo) drift | different results for the same masks | Tracyo's tests port Curvyo's 0094 cases; the later move to vectrace (ADR 0008 Consequences) removes one |
