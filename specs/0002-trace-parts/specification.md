# Trace parts

Status: Draft
Priority: Must
Origin: Customer ("Ich brauche parts."). Everything marked **(Proposal)** is the product owner's and vetoable at the demo.
Revised 2026-11-12 (architect for the product owner): reconciled with `ux-notes.md` (Curvyo's behaviour wins); regions are rectangles only (customer, 2026-11-12); subtraction by a Subtract toggle (lead, ux Frage 6 C). Criterion numbers are unchanged; nothing was removed whole.

## User value

As a maker whose picture needs different trace settings in different places (a logo on a photographed sheet, a dark label on a light box, text next to a drawing) I want to mark pieces of the picture, give each its own Method, Threshold, Smooth and the rest, and save one SVG in which the pieces fit together, so that I do not trace the picture several times and join the results by hand in Inkscape.

Behaviour reference: Curvyo `docs/tracing-images.md` "Mask" and "Parts", `specs/0094-masked-composite-trace`. Where this spec says "as in Curvyo", that document decides; labels, bar texts and notices are copied verbatim. The difference: Curvyo masks with ordinary paths of the document. Tracyo has no objects, so a part's area is made of rectangular **regions** drawn on the picture in the Trace tool. Spec 0001 stays valid except where this spec names a criterion it replaces.

## Terms

- **Part**: an area of the picture with its own mask mode (Inside or Outside) and its own trace settings. 1 to 8 parts. Part 1 starts as the whole image.
- **Region**: a rectangle drawn on the picture, its sides parallel to the picture's, its corners on whole picture pixels **(Proposal)**. The regions of a part together are its mask.
- **Mask**: the area of the regions of a part. A region has a sign: added (default) or subtracted (drawn while **Subtract** is pressed). Mask = union of all added regions minus union of all subtracted ones, independent of drawing order. Overlaps count once. A part with only subtracted regions has an empty mask.
- **Seam**: the edge where the mask of a part cuts a shape.

## Acceptance criteria

Fixtures (committed in `tests/fixtures/`, synthetic). **P1**: 1000 x 1000 px, so W = H = 264.583 mm (0001/20b, 96 dpi). White background; black disc of 300 px diameter centred at (250, 500); black square of 300 px side centred at (750, 500); black bar y = 480 to 520 across the full width (joins disc and square); black disc of 100 px diameter centred at (150, 150); grey (#808080) square of 100 px side with its centre at (900, 100). **R1**: rectangle x = 0 to 500, y = 0 to 1000 px (x = 0 to 132.292 mm). **P2**: P1 with the right half's white replaced by #D8D8D8. **P3**: the poster of Curvyo 0091's reference set (text and shapes). Areas are compared by polygon clipping in the test.

### Parts and regions in the bar

1. Given a picture, then the bar shows after **Preset** the group **Rectangle**, **Subtract** (toggle buttons with an icon and the name as accessible name and tooltip), **Inside**, **Outside**, **Clear mask**. Advanced shows as its last row, after Corners and before Analyse, the muted label "Parts", one chip per part numbered in the order made, **Add part** (+) and the trash. This replaces the sentence "There is no Mask group ... no Parts row" of 0001/10 and the order of 0001/14. Without a picture none of it is shown (the bar is "Trace" and "Open image…" only, as 0001 and Curvyo).
2. **Add part** adds a chip at the end, makes it active and copies the settings of the active part except its picked colours and palette edits (as Curvyo); no regions, Inside. Until it has a region its chip is muted (dashed) and it adds nothing. At 8 parts Add part is dimmed. A click on a chip, or the arrows, Home and End on the focused chips, make that part active: its settings load into the bar and Advanced, and its regions are the ones drawn and editable. Everything the bar shows belongs to the active part; the active chip is the only mark of it.
3. The **trash** removes the active part (Delete on a focused chip arms it too): the first press turns the row soft red, the second after 400 ms removes it; the parts renumber and the part before becomes active. With one part the trash is dimmed.
4. Part 1 with no region covers the whole image. A later part with no region contributes nothing. When no part has anything to trace the bar's save button is dimmed with the tooltip "0 paths: no part has anything to trace."
5. Order: left to right is bottom to top. Where parts overlap, the later one wins: inside its mask only its shapes exist, and the earlier parts are cut exactly at its edge, so no area is traced twice.
6. **Inside** (default on every start and for every new part) traces only what lies inside the part's mask, **Outside** only what lies outside it. Without regions the whole image is traced either way; the choice applies once the first region exists. It is a setting of the part, not part of a Preset.

### Drawing and editing regions

7. **Rectangle** stays pressed until pressed again, Esc, or a picture is opened; while pressed, a drag on the picture draws a rectangle from corner to corner (Shift: from the centre; Ctrl: a square; as Curvyo's Rectangle tool); a drag under 2 px draws nothing; the drag is not a pan (middle drag, Space+drag and scroll still pan). **Subtract** is a toggle of its own: while it is pressed, every rectangle drawn is subtracted; it stays pressed until pressed again or a picture is opened, does not press Rectangle and does not change regions already drawn. Edges may lie outside the picture; the mask is cut to the picture.
8. Esc order (extends 0001/9): stops a running trace; then disarms an armed pipette; then releases Rectangle; then clears the region selection. It never closes the app.
9. With Rectangle not pressed, a click on a region of the active part selects it (the topmost, the last drawn; a click on empty canvas deselects). A selected region shows 8 handles on its box; dragging a handle resizes (Shift: about the centre; Ctrl: keeps the proportions at a corner handle; as Curvyo's Select tool), never below 1 px; dragging inside the region moves it, arrow keys move it 1 px, with Shift 10 px; Delete or Backspace removes it. A region can be moved or resized past the picture's edge; it is cut to the picture in the trace only. Every change retraces the active part and those below it (their area changes, 0002/20).
10. **Clear mask** removes all regions of the active part at once; the part and its settings stay. Without regions it does nothing. It replaces Curvyo's "Pick mask paths" (the regions are made here, not picked from objects) and its Clear mask (which only released paths); Tracyo's regions have no life outside the part, so they are deleted.
11. The regions of every part are drawn over the preview as a black line 0.25 mm wide at the view's zoom and at least 1 px (Curvyo's mask path in its Rectangle tool's default style); a subtracted one dashed. Only the active part's regions can be selected and edited. In the preview the picture outside the active part's mask is washed further (to 12 %) and the outline is drawn only inside the mask, as in Curvyo. Accessible names: "Inside 2 paths" / "Outside 1 path" for the mask mode (the count is that of regions, wording as Curvyo), with two or more parts "3 parts", for an active later part without region "Part 3 has no paths".
12. Notices (bar tooltip of the dimmed save button and screen-reader text, verbatim): regions that cover none of the image (all added regions outside it, or an empty mask with Inside): "0 paths: the mask is outside the image." Regions that cannot be combined into one area (the geometry kernel refuses them; a rectangle cannot be drawn or resized to zero width or height): "0 paths: the mask paths could not be combined." Neither is a failure; nothing is written for that part.

### What the trace does

13. **Cut.** Given P1 and one part, R1 Inside, Threshold 70, Single colour, then the saved SVG is as 0001/20 and: the bar is cut at x = 132.292 mm (every vertex that lies on the seam is within 0.005 mm of x = 132.292, the bar corners at the seam are (132.292, 127.000) and (132.292, 137.583) within 0.005 mm); a cut shape contains only `M`, `L` and `Z` commands over its whole outline; the small disc at (150, 150) px, wholly inside, still has `C` commands. For a mask of several regions (an L, a frame) the vertices of the cut run along its edges within the same 0.005 mm. No shape lies outside the mask.
14. **Fit.** Given P1, part A = R1 Outside and part B = R1 Inside with the same settings, then: area(A joined with B) equals area(A alone) + area(B alone) within 0.01 mm^2, the intersection of A alone and B alone is at most 0.01 mm^2, and the joined result differs from a single whole-image trace with the same settings by at most 0.1 % of its area (symmetric difference). "Alone" means a document with that one part only.
15. **Join.** Given P1, part 1 = whole image, Threshold 40, Smooth 20; part 2 = R1 Inside, Threshold 70, Smooth 80, Single colour, then the SVG has exactly one layer `#000000` with exactly one `<path>`, whose `d` has exactly two `M` and two `Z` (the joined disc, bar and square as one outline without a hole, and the small disc), and no two segments of the result overlap along x = 132.292 mm for a length above 0.01 mm (no seam line). The grey square is not in the result (Threshold 40).
16. **Later part wins.** Given part 1 = whole image at Threshold 40 and part 2 = R1 Inside at Threshold 70 on P1 with the grey square moved into R1, then the grey square is in the result; and with parts swapped (R1 Inside at 70 first, then the whole image at 40) the grey square is not, and the later part's shapes are alone inside its mask. The result inside R1 differs from a single-part trace at Threshold 70 clipped to R1 by at most 0.1 %, the result outside R1 from a single-part trace at Threshold 40 minus R1 by at most 0.1 %.
17. **Multicolour.** Given Multicolour in both parts, then the SVG has one layer per distinct colour over all parts (equal hex merges, different hex stays separate; label and order as 0001/20d), one `<path>` per layer for the colour's outlines, and different-coloured shapes overlap by at most 0.01 mm^2. There are no layers per part (**Proposal**, Frage 3).
18. **Own area.** Given P2 with part 1 = whole image and part 2 = right half Inside, Remove background with Auto, then part 1's automatic background and Threshold are those of the left half and part 2's those of the right half (white and #D8D8D8 respectively); the automatic Threshold, the background of Auto and the Analysis of a part read only the pixels inside that part's mask.
19. **Text and shapes.** Given P3 with Recognise text on in part 1 and off in part 2, then a line whose box centre lies inside part 2's mask stays paths of part 2, a line in part 1's own area (centre not in the mask of a later part) is `<text>` as 0001/20g; with Inside the line is kept whole, with its centre outside the mask it is neither text nor traced. Recognise shapes looks only at the cut result: a rectangle cut by a seam is not a `<rect>`, one wholly inside is.
20. **Retrace.** Given a change of one setting of the active part, then only that part is traced again (trace-call counter 1 in a test) and the others come from the cache; for a change of the mask or of the regions the parts below the active one follow. The preview shows the joined result; the dragging reduced copy of 0001/7 is used per part.
21. **Save.** Save as... (0001/19) traces all parts at full size, joined, with one progress from 0 to 100 % over the whole job, never running backwards; Stop and failure behave as in 0001/8. The SVG is exactly the joined Inkscape SVG of 0001/20 (one result, layers per colour, no per-part layers, no `<metadata>` of parts), so 0001/20 and /21 hold.
22. **Determinism.** The same picture, parts, regions and settings give a byte-identical SVG on repeated runs and with 1 and with N worker threads.

### Life of parts

23. Parts and their regions last while the picture is open. They are not saved between starts: a start has one part, no region. Opening another picture, or the same one again, resets to one part without regions (the settings of part 1 stay, as 0001/1). Only part 1's settings are remembered in `settings.toml` (0001/18); the settings of parts 2 to 8, Inside/Outside and regions are not. This replaces nothing in the file format; `settings.toml` gets no new key.
24. Saving the SVG, stopping a trace, or opening the About window does not change parts or regions.

### Speed and safety

25. Release build, P1-like picture of 12 Mpx (4000 x 3000 px), 8 parts of equal size tiling the picture, same settings, Brightness, Single colour: thread CPU time of one full trace of all parts is at most 1.3 times that of one whole-image trace with those settings (each part is traced from the crop of its mask with a margin; parts that read text, drop border shapes, use Denoise or Upscale are exempt). 8 parts each covering the whole picture: at most 9 times. A change of one part's setting retraces that part only (0002/20). A frame of the UI thread takes under 50 ms while parts trace (CPU time of that thread, 0001/7). Timing tests measure thread CPU time, not wall time. **(Proposal numbers; reported by the tester, a miss goes to `docs/technical-debt.md` if the customer agrees.)**
26. No network (0001/23 holds unchanged, the dependency list gets nothing new unless the ADR justifies it).
27. 8 parts with 20 regions each (added and subtracted, overlapping) edit and trace without a frame over 50 ms on the UI thread and without a panic; invalid geometry gives the notice of criterion 12, never a panic.

## Out of scope

- Ellipse and polygon regions (customer, 2026-11-12: rectangles only) and importing a mask from an SVG (paths of an SVG as regions): possible later stories.
- Saving parts or regions (not in `settings.toml`, not in the SVG, no project file).
- One layer or group per part in the SVG; a Join command for existing SVGs.
- Undo/redo of region edits, selecting several regions, rotating regions, snapping, rulers, numeric entry of region coordinates, other region shapes (ellipse, polygon, star, freehand, magic wand).
- Overlapping parts as separate results; per-part Preset memory.
- Any hint or tip text in the UI.

## Open questions

- **Frage 1 (PARTS-SAVE): Parts are lost when the picture is closed.** A: not saved (this spec). B: saved next to the picture or in a project file, so a later start re-traces. Recommendation A (no document in Tracyo; B needs a file format). Default: A.
- **Frage 2 (REGION-SUB): How a region subtracts.** Decided (lead, 2026-11-12, ux-notes Frage 6 C): a **Subtract** toggle after Rectangle, because Linux window managers take Alt+drag; mask = added minus subtracted, order-independent.
- **Frage 3 (PARTS-LAYERS): Layers in the SVG.** A: one result with layers per colour only (this spec, as the customer said). B: also a layer per part. Default: A.
- **Frage 4 (REGION-WORDS): Notices say "mask paths"** (verbatim, as asked) though Tracyo has regions. A: keep verbatim. B: say "regions". Default: A.
- **Frage 5 (PARTS-SPEED): The 1.3 / 9 times CPU budget of criterion 25** is the product owner's proposal; Curvyo only reports its 1.3. Default: blocking for 1.3, reported for 9.
