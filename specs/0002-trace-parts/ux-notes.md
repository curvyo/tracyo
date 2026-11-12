# UX notes: Trace parts

Status: Draft (ux-engineer, 2026-11-12; revised by the architect 2026-11-12: regions are rectangles only (customer), subtraction is a Subtract toggle (lead, Frage 6 C), the spec follows section 13's rows 1 to 4). Reference: Curvyo `main` at `fc8edca8`, its dev server
rendered in Chromium at 1x with Liberation Sans (as `0001`). Rule unchanged: **what Curvyo draws,
Tracyo draws**; where Curvyo's code and its documents disagree, the code wins. Own choices are
marked **(Proposal)**. No hint or tip text; labels that name a control or a state stay. All sizes
are logical px at scale 1. Section numbers of `0001` are written `0001 §n`; tokens, type, focus rings
and tooltips are those of `0001 §2`, `§5` and `§7` and are not repeated where they are unchanged.

Sources: `~/curvyo/frontend/src/components/TraceBar.tsx` (`MaskGroup`, `MaskPill`, `ProgressSlot`),
`TraceParts.tsx`, `hooks/useDeleteArm.ts`, `lib/deleteArm.ts`, `lib/traceParts.ts`,
`lib/traceText.ts`, `lib/cursors.ts`, `index.css`; `curvyo-editor-wasm/src/session/trace_region.rs`
(the 12 % wash), `curvyo-render-core/src/theme.rs` (canvas glyphs); `docs/tracing-images.md`
"Mask" and "Parts"; `docs/design-system.md` (Color tokens, Transform handles, Pen, Modifier badge);
`specs/0094-masked-composite-trace/ux-notes.md` and its supersessions in
`specs/0089-trace-quality-and-colour/ux-notes.md` §24 and §27 (no visible notices; no
"Part 2 of 3" label).

## 0. Reference pictures (`reference/`)

Made by `reference/capture.cjs` (instructions at its top) from `0001`'s `input-logo.png`. The mask
paths were drawn with Curvyo's real Rectangle and Ellipse tools and picked with Pick mask paths:
R1 round the LOGO block, R2 round the green bar, E1 round the disc, R3 below the picture.

| File | What |
|---|---|
| `curvyo-00` .. `curvyo-17` | Curvyo as it is (1280 x 800, panel open, so its bar has two rows): 00 Mask group at rest, 01 Pick armed with the hover box on R1 (view), 02 Inside with R1 and R2 (view), 03 the same bar, 04 Outside hovered, 05 Outside (view), 06 Clear mask tooltip, 07 Pick mask paths keyboard focus, 08 Advanced with one part, 09 three parts (1 inside, 2 active outside, 3 muted), 10 three parts (view), 11 chip hover with tooltip, 12 muted chip hover, 13 Add part tooltip, 14 trash armed, 15 chip keyboard focus, 16 one part whose mask is outside the image (dimmed Trace, tooltip), 17 that part plus an empty part 2 (dimmed Trace, tooltip) |
| `curvyo-20` .. `curvyo-27` | Curvyo's canvas look: 20 selected rectangle (Select tool), 21 hover box on a rectangle, 22 selected ellipse, 23 rectangle being drawn (live outline and size chip), 24 Pen polygon being drawn (rubber band), 25 Pen over its close target, 26 Node tool on a closed polygon with one node selected, 27 the Remove badge (Ctrl held over empty canvas) |
| `px-00` .. `px-17` | **To be re-captured** (section 12.2). The bar states 00 to 17 **as Tracyo draws them**: panel hidden, 1360 x 800 (bar cap 1164 px, see section 12), Pick mask paths replaced by **Rectangle, Subtract** in Curvyo's own icon-button look (Rectangle pressed in px-01, Subtract focused in px-07; the files committed before 2026-11-12 still show Rectangle, Ellipse, Polygon), finish button "Save as…". These are the pixel references of section 12 |

**Validity after the revision of 2026-11-12.** Still valid: `curvyo-00` .. `curvyo-17`, `curvyo-20`,
`curvyo-21`, `curvyo-23`, `curvyo-27` (Curvyo as it is; 09 and 10 show E1, an ellipse, but are used for
the chip and wash look only). No case left: `curvyo-22` (ellipse), `curvyo-24`, `curvyo-25`, `curvyo-26`
(Pen and Node tool). **All of `px-00` .. `px-17` must be re-captured**: they show three region tools
(a 256 px group); with Rectangle and Subtract every item after the group moves 32 px left.
`capture.cjs` changes: the injected buttons are Rectangle (`square`) and Subtract (`square-minus`),
and part 2's mask in px-09 .. px-12 is the rectangle R4 (section 12.2) instead of the ellipse E1.

## 1. What replaces Pick mask paths

Curvyo builds a mask by picking document paths; Tracyo has no objects, so the maker draws the
regions, and regions are rectangles only (customer, 2026-11-12). The pick button is replaced by
**Rectangle**, followed by the **Subtract** toggle (lead, Frage 6 C: Alt+drag is taken by Linux
window managers), in the same group, in Curvyo's icon-button look (the look of Pick mask paths
itself, of the Advanced toggles and of every 28 px bar toggle).

| # | Button | Lucide icon (1.50.0) | Accessible name and tooltip | Why this icon |
|---|---|---|---|---|
| 1 | Rectangle | `Square` | "Rectangle" | Curvyo's Rectangle tool on the rail |
| 2 | Subtract | `SquareMinus` | "Subtract" | the Rectangle glyph with a minus: "take a rectangle away"; same outline weight as `Square`. `Minus` alone is one thin stroke at 16 px and reads as a separator or zoom-out **(Proposal)** |

Look (as `0001 §5` "Icon button"): 28 x 28, radius 8, glyph 16 px, stroke 1.5 (1.0 px rendered),
`#3A3A3F`; hover ground `#B9C6E3`; **pressed** ground `#2F6FEE`, glyph `#FFFFFF`, exactly as the
pressed pipette of Pick colours and the pressed Advanced toggles (Invert, Drop border shapes) draw it
(`px-01` for Rectangle); keyboard focus ring 2 px `#2F6FEE` outside a 1 px `#DCDCE0` gap. The two
are independent toggles: each press flips that button only, both may be pressed. Rectangle is
released by its own press, Esc or opening a picture; Subtract only by its own press or opening a
picture. Subtract changes no region already drawn; it applies to the rectangles drawn while it is
pressed. No shortcut letters (Curvyo's Trace bar has none, `0094` UX notes §4); the tooltip is the
name alone. Each is its own Tab stop, Rectangle first.

## 2. The Mask group in the bar

One flow item directly after Preset, always drawn while a picture is shown, never dimmed
(`0089` §24.4). Inside, 4 px apart: Rectangle 28, Subtract 28, the Mask segments 128,
Clear mask 28 = **224 px** (Curvyo's group is 192: Pick 28, segments 128, Clear mask 28).

| Control | Drawn | States | Accessible |
|---|---|---|---|
| Rectangle, Subtract | section 1 | rest, hover, pressed, focus | toggle buttons, names as section 1, pressed state exposed |
| Mask segments | "Inside" \| "Outside", each 64 x 28, 14 px, centred; 1 px outline inside in `#3A3A3F` at 40 % (`#9B9BA0` on the bar), radius 8, drawn over the segments; no gap | selected `#2F6FEE` / white (default Inside); other: text `#3A3A3F`, hover ground `#B9C6E3` (`px-04`); focus ring 2 px `#2F6FEE` inside the segment | radio group "Mask", radios "Inside", "Outside", one Tab stop, arrows move and select, no wrap |
| Clear mask | `X` 16 px, 28 x 28 icon button | rest, hover `#B9C6E3`, focus ring 2 + gap 1; never dimmed; without regions a press does nothing | "Clear mask", tooltip "Clear mask" (`px-06`) |

Inside / Outside is selectable without regions; the choice applies with the first region. It
belongs to the active part (a new part starts Inside).

**Bar positions** (bar left edge = 0, Brightness, one row 36 px, controls at y 4..32): About 8..36,
Preset 44..176, Rectangle 184..212, Subtract 216..244, Inside 248..312, Outside 312..376, Clear mask
380..408, Method 416..592, Threshold 600..750, Smooth 758..908, Tolerance 916..1066. Ignore below
(171) would need 1074..1245: with the cap of 1164 (reference) or 1196 (1280 px window) it does not fit,
and it would not with Curvyo's 192 px group either (1042..1213):
**the bar has two rows** (`px-00`): row 2 is Ignore below 8..179 and the expander with
"Save as…" at the far right. This replaces `0001 §3.3`'s one-row bar; the wrap is by whole items as
before (Remove background: Smooth stays on row 1 at 934..1084, Tolerance would need 1092..1242 and wraps).

**Tab order** (replaces `0001 §3.4` up to Method): Open, About, Preset, **Rectangle, Subtract,
Mask (one stop), Clear mask**, Method, ... as `0001`; in the Advanced block after Corners:
**Parts (one stop), Add part**, then Analyse. The trash is no Tab stop.

## 3. The Parts row (last row of Advanced)

After Corners, 8 px, as one whole flow item; Analyse stays at the far right (`px-08`, `px-09`).
Height 36, items 4 px apart, vertically centred:

| Item | Size | Look | Accessible |
|---|---|---|---|
| Label "Parts" | 32 px slot | 12 / 16, `#3A3A3F` at 70 % = `#6B6B6F` on the bar, left-aligned | hidden (the radio group carries the name) |
| Chips | 28 x 28 each, 4 px apart | icon-button look, number 14 px tabular, centred; **mark** 6 x 6 at 2 px from the right and bottom: solid square in the text colour = Inside, 1 px outlined square = Outside, none = whole image or muted | radio group "Parts", roving focus, one Tab stop |
| Add part | 28 x 28, `Plus` 16 / 1.5 | icon button; at 8 parts: whole button 50 % (glyph `#8B8B8F` flattened), no hover, default cursor | "Add part"; tooltip "Add part" or "8 parts at most" |
| Remove part | 28 x 28, `Trash2` 16 / 1.5 | icon button; with one part 50 % (`#8B8B8F`), no hover; armed: see below | "Remove part 2" / "Press again to remove part 2" / "Keep at least one part"; same as tooltip |

Group width: 32 + 4 + chips + 4 + 28 + 4 + 28 = 128 (1 part), 160, 192 (3), ..., 352 (8). Past the
block's width Add and Remove wrap together to a second 36 px line; label and chips stay.

**Chip states** (`px-09`, `px-11`, `px-12`, `px-15`):

| State | Ground | Number | Mark | Border |
|---|---|---|---|---|
| active | `#2F6FEE` | white | white | none |
| other, with regions | none | `#3A3A3F` | `#3A3A3F` | none |
| hover (not active) | `#B9C6E3`, radius 8 | as rest | as rest | as rest |
| muted (a later part without regions, not active) | none | `#3A3A3F` at 50 % = `#8B8B8F` | none | 1 px **dashed** `#3A3A3F` at 60 % = `#7B7B7F`, radius 8 (dashes as Chromium draws a 1 px CSS dashed border: 3 on, 3 off along the straight edges) |
| muted and active | `#2F6FEE` | white at 50 % = `#97B7F7` | none | none (`curvyo-17`) |
| keyboard focus | as its state | | | ring 2 px `#2F6FEE` outside a 1 px `#DCDCE0` gap |

**Armed trash** (`px-14`): the whole Parts group (label to trash, its own box, radius 8) takes
`--destructive-soft` `#F7D3CF`; the trash becomes a filled `#B3261E` square, radius 8, glyph white;
no size change; colours change over 120 ms ease-out (none with reduced motion). The second press
removes when it comes 400 ms or more after the first; the arm ends after 4 s, on Escape (consumed:
the first Escape only disarms), on a pointer press anywhere but the trash, when focus leaves the
group, and when a chip is picked or a part added.

**Behaviour** (as Curvyo): a click, or arrows, Home, End on the focused chips, make that part
active, load its settings into the bar and Advanced and show its regions as the editable ones; a
mouse press hands the keyboard back to the canvas, a key keeps it. Add part appends the next chip,
active, settings copied from the active part except its colours (`0089` §27.8), no regions, Inside;
focus stays on Add part. After a removal the chips renumber and the part before becomes active.

## 4. Tooltips, notices and announcements (exact texts)

No visible notice anywhere (Curvyo `0089` §27.1): a reason is the tooltip of the dimmed blue
button and is said by the hidden status region.

| Where | Text |
|---|---|
| Dimmed "Save as…" tooltip and hidden region: the active part's regions cover none of the picture, or its mask is empty with Inside | "0 paths: the mask is outside the image." (`px-16`) |
| same: the regions cannot be combined into an area | "0 paths: the mask paths could not be combined." |
| same: no part has anything to trace | "0 paths: no part has anything to trace." (`px-17`) |
| "Save as…" tooltip at rest with two or more parts | "Save as… (Ctrl+Shift+S), 3 parts joined" (Curvyo: "Create paths (Enter), 3 parts joined") **(Proposal)** |
| Chip tooltip | "Part 1 of 3, inside, 2 paths"; "Part 2 of 3, outside, 1 path"; "Part 3 of 3, no paths"; "Part 1 of 1, whole image" (counts are regions; wording "path(s)" kept, spec Frage 4) |
| Chip accessible name | the tooltip plus ", active" for the active chip |
| Expander tooltip while Advanced is closed and there are two or more parts | second line "Changed: 3 parts" (before the other changed names: "Changed: 3 parts, Invert"); the 6 px changed-dot shows (`0001 §11`) |
| Hidden status region (polite, `0001 §3.1`), after a change has settled, segments joined by ". " | the mask: "Inside 2 paths" / "Outside 1 path"; with two or more parts "3 parts" instead; then the counts as `0001`; last, for an active later part without regions "Part 3 has no paths" |
| Rectangle or Subtract pressed or released | the toggle state; the polite region says "Rectangle on" / "Rectangle off", "Subtract on" / "Subtract off" (Curvyo: "Pick mask paths on" / "off") **(Proposal)** |
| Trash | polite: "Press again to remove part 2" on arming; nothing after the removal |

Curvyo's refusal "Use Object to path for text; one image at a time." has no case in Tracyo.

## 5. The regions on the canvas

Order, bottom to top: ground, picture, white veil (shows 35 %), **extra wash** (below), region
outlines, preview outline with its casing (`0001 §6`), drag ghost, hover box, selection box and
handles, drawing feedback, chips and badges.

| Item | Look | Curvyo source |
|---|---|---|
| **Extra wash** | Where the active part does not trace (outside its mask with Inside, inside it with Outside), a white layer at alpha 1 - 12/35 = 0.657 over the already veiled picture, so the picture shows at **12 %**. Checked colours: LOGO blue `#2B4C9B` at 35 % `#B5C1DC`, at 12 % `#E6EAF3` (`curvyo-02`, `curvyo-05`). No extra wash for a part without regions. Only the active part's; other parts' outlines stay at full strength (`curvyo-10`) | `trace_region.rs` (`REGION_WASH_LEFT` 0.12, `WASH_LEFT` 0.35) |
| **Preview outline** | Only inside the traced area, ending exactly on the mask edge (as `0001 §6`; 50 % while stale) | `0094` §2 |
| **Region outline** (added) | `#000000`, solid, no fill, width 0.25 mm at the view's zoom and at least 1 px (1 px at 100 %, 3.8 px at 400 %); butt ends, miter joins. Drawn for the regions of **every** part; only the active part's are hit-tested (spec criterion 11) | Curvyo's mask paths are ordinary paths in the Rectangle and Ellipse tools' default style `#000000`, 0.25 mm, min display width 1 px (`MIN_DISPLAY_STROKE_WIDTH_PX`) |
| **Region outline** (subtracted) | the same line, dashed 4 on, 3 off (screen px, phase restarting at each region's first point) **(Proposal: Curvyo has no subtracted mask; 4/3 is its dash for the selection box and reference lines)** | `SELECTION_BOX_DASH_PX`, `--dim-reference` |
| **Hover box** (no tool pressed, pointer within 8 px of a region's outline or over its interior, active part only, no drag running) | 1 px `#2F6FEE` at 65 % over a white casing at 65 %, on the region's box (it covers its outline; `curvyo-21`: `#6992E5` on white) | `--hover-box` |
| **Selected rectangle** | the bounding box as a 1 px dashed `#2F6FEE` line, 4 on, 3 off; 8 resize handles 8 x 8, radius 2, white fill, 1 px `#2F6FEE`; a handle being dragged is filled `#2F6FEE`. Edge handles only while the shorter side on screen is 24 px or more. Hit radius per handle min(16, shorter side / 3), at least 4; nearest centre wins (`curvyo-20` without rotate, skew, radius knobs and the centre handle) | `TRANSFORM_RESIZE_HANDLE_*`, `SELECTION_BOX_DASH_PX` / `GAP_PX` |
| **Move and resize drag** | the region drawn as itself at the new geometry; its old outline stays as the drag ghost, 1.5 px `#2F6FEE` at 60 % over a 1.5 px-each-side white casing; box and handles follow the new geometry; Esc during the drag restores; the preview retraces after release | "Drag ghost" |
| **Rectangle being drawn** | live outline 1.5 px `#2F6FEE`, hollow, over a 1.5 px-each-side white casing; size chip 12 px up and right of the pointer: `#DCDCE0`, 1 px `#B4B4B8` border, 12 px `#3A3A3F` tabular, radius 8, padding 2 x 6, no shadow, "160 × 140 px" (picture pixels; Curvyo "42.3 × 37.0 mm") (`curvyo-23`) | "Live preview outline", "Live transform readout" |
| **Subtracted region being drawn** | the same, the live outline dashed 4 / 3, plus the **Remove badge**: 16 px `#2F6FEE` disc with a 1 px white outer ring and a white minus 8 px / 1.5 px, its centre 16 px left of and 16 px below the pointer (mirrored right at the canvas edge), shown from the press until release while Subtract is pressed (`curvyo-27`) **(Proposal)** | "Modifier badge", Remove |

**Cursors** (egui system cursors; Curvyo's custom images are not available, section 12):

| Where | Cursor |
|---|---|
| canvas with Rectangle pressed | crosshair (Curvyo's creation tools: `cursor-crosshair`), also during the drag |
| over a resize handle | `ResizeNwSe`, `ResizeNeSw`, `ResizeVertical`, `ResizeHorizontal` by the handle's direction (Curvyo's rotated double arrow falls back to these) |
| over a region, during a move drag | default arrow (Curvyo's Select and Node tools keep the arrow) |
| pan (Space held, middle drag) | as `0001` (grab, grabbing) |
| pipette armed | as `0001` (crosshair) |

## 6. Drawing and editing (modifiers)

As Curvyo's Rectangle tool and Select-tool resize (spec criteria 7 and 9):

| Gesture | Modifier | Does |
|---|---|---|
| draw a rectangle | none | from corner to corner |
| | Shift | from the centre |
| | Ctrl | square |
| | Subtract pressed | subtracted region |
| resize by a handle | Shift | about the centre |
| | Ctrl | keep proportions (corner handles) |

A drag of less than 2 px draws nothing; a resize stops at 1 px; corners land on whole picture pixels (rounded) **(Proposal)**. A press with a region tool pressed is never a pan; middle
drag, Space+drag and scroll still pan.

## 7. Keyboard

Ctrl means Cmd on macOS. Canvas keys act when no text entry has focus. Additions to `0001 §10`:

| Key | Where | Does |
|---|---|---|
| Esc | anywhere | stop a running trace; else disarm the pipette; else release Rectangle; else deselect the region; else as `0001` (focus to the canvas). An armed trash consumes the first Esc (disarm only) |
| Enter | canvas | Save as… (`0001`) |
| Delete, Backspace | canvas, a region selected | removes it |
| Arrows | canvas, a region selected | move it 1 picture px; Shift 10 px (Curvyo nudges 1 mm / 10 mm) |
| Space, Enter | focused Rectangle or Subtract | press / release it |
| Arrows | Mask segments | move and select, no wrap |
| Arrows, Home, End | Parts chips | move and make that part active, no wrap |
| Delete | a focused chip | arms the trash for the active part; Delete or Enter again (after 400 ms) removes |
| Esc | a focused chip, trash not armed | focus to the canvas |
| Enter, Space | Add part | adds a part |

## 8. Without a picture

As `0001 §6`: the bar is "Trace" and "Open image…" only; no Mask group and no Parts (Curvyo shows
no controls without an image; spec criterion 1).

## 9. Life of the state on screen

Opening a picture resets to one part, Inside, no regions, Rectangle and Subtract released, Advanced as it was.
Rectangle stays pressed after each region (Curvyo's pick mode stays armed after each
pick); a chip pick or Add part keeps Rectangle and Subtract as they are.

## 10. Contrast and hit targets

Every new control is 28 px. Chip number `#3A3A3F` on the bar 8.2:1; the muted number (50 %) is
informational (its state is also in the dashed border and the name). Armed row: `#3A3A3F` on
`#F7D3CF` 8.2:1. The black region line on the veiled picture: the picture is at most 35 % strong, so
the darkest ground under the line is about `#A5A5A5` (black at 35 %): 8:1; on the 12 % wash 16:1.

## 11. Icons to bundle (lucide 1.50.0, add to `icons.rs`)

`Square`, `SquareMinus` (16 px, 1.5), `Plus`, `Trash2` (16 px, 1.5). `X` exists. All four are in
`lucide-react@1.50.0` (`square`, `square-minus`, `plus`, `trash-2`) and are written into
`assets/icons/lucide/` like the existing ones (ADR 0006, `ASSETS.md` hashes).

## 12. egui mapping and pixel comparison

### 12.1 What egui cannot draw 1:1, and the nearest

| Curvyo | egui | Nearest |
|---|---|---|
| CSS `border: 1px dashed` on the rounded muted chip (Chromium places 3 / 3 dashes and corner pieces itself) | no dashed rounded-rect stroke | draw the rounded rectangle path and dash it with `Shape::dashed_line` (3 on, 3 off, starting at the top-left corner's end); compare the chip only with the neighbourhood tolerance |
| Dashed selection box and subtracted outlines with a dash phase that runs round the corners | `Shape::dashed_line` restarts per call | build each outline as one closed polyline and dash it in one call |
| The 12 % wash as a white compound path with holes (non-zero fill) | egui fills convex or simple polygons only | have `tracyo-core` give the wash area as disjoint rectangles (ADR 0008 §5), paint as one `Mesh` |
| Hairline width "0.25 mm at zoom, at least 1 px" | strokes are logical px | compute `max(1, 0.945 * zoom)` per frame |
| Custom cursors (Curvyo's rotated resize arrow, pen nib, pick-mask arrow) | system cursors only | section 5's system cursors (Curvyo's own fallbacks) |
| Tooltip delay skipped after another tooltip was just open (Radix) | egui's delay is per hover | keep 400 ms each time; tooltips are not compared by pixels |
| `transition-colors 120ms` of the armed row | no CSS transitions | `ctx.animate_value_with_time` per colour, 0.12 s, off with reduced motion |
| Group opacity 50 % (dimmed Add part, trash, Save as…) | per-shape alpha | the flattened colours given above |
| Chip number at 50 % on blue (muted and active) | per-shape alpha | `#97B7F7` |

### 12.2 Pixel comparison (tester)

Method, tolerance and pass rule are `0001 §12.2` with `reference/compare.py` (copied unchanged):
1 px neighbourhood, 24 per channel, at most 2 % differing pixels, no run of 12 or more. The
pictures were rendered with Liberation Sans; re-run `capture.cjs` on the machine that photographs
Tracyo if its UI font differs. **Window:** the `px-*` bars wrap at a cap of **1164 px**: run these
tests in a **1248 x 800** window (cap = width - 84), `pixels_per_point = 1`, picture
`0001-trace-and-save/reference/input-logo.png`, Method Brightness. Regions: R1 = picture px
(24, 25)-(324, 235), R2 = (344, 315)-(619, 405), R4 = (330, 30)-(610, 310) (the box of Curvyo's E1),
R3 = (100, 450)-(300, 520) (below the picture). The `px-*` pictures are those re-captured after
2026-11-12 (section 0).

| Case | Reference | Tracyo state | Compare |
|---|---|---|---|
| P1 Mask group at rest | `px-00` | no regions, Inside | whole bar |
| P2 Rectangle pressed | `px-01` (bar part) | Rectangle pressed | bar only (crop the top 60 px of both) |
| P3 Inside with regions | `px-03` | R1, R2, Inside | whole bar (Threshold reads 66 %) |
| P4 Outside hovered | `px-04` | as P3, pointer over Outside | whole bar |
| P5 Clear mask tooltip | `px-06` | as P3, hover Clear mask 900 ms | bar; tooltip box by eye |
| P6 Region tool focus | `px-07` | Shift+Tab from the Mask segments (focus on Subtract) | whole bar |
| P7 One part | `px-08` | Advanced open | whole bar |
| P8 Three parts | `px-09` | part 1 R1+R2 Inside, part 2 R4 Outside (active), part 3 no regions | whole bar |
| P9 Chip hover | `px-11` | pointer over chip 1 | bar |
| P10 Muted chip hover | `px-12` | pointer over chip 3 | bar |
| P11 Add part hover | `px-13` | pointer over Add part | bar |
| P12 Trash armed | `px-14` | first press on the trash, pointer on it, after 900 ms | bar (the soft red group and the red trash must match) |
| P13 Chip focus | `px-15` | keyboard focus on chip 2 | bar |
| P14 Mask outside the image | `px-16` | one part, R3 only, Inside; hover Save as… | bar: dimmed Save as… |
| P15 No part to trace | `px-17` | P14 plus Add part | bar: dimmed Save as…, chip 2 muted and active |
| P16 Subtract pressed | none (Curvyo has no Subtract) | as P2 plus Subtract pressed | against Tracyo's own states only: the bar outside the Subtract box equals P2's render (same tolerance); the Subtract box's ground equals the pressed Rectangle box's ground of P2 pixel for pixel outside the glyphs (`#2F6FEE` ± 24), its glyph pixels are white (`#FFFFFF` ± 24); at rest (P1) its box equals the rest look of Rectangle's outside the glyph |
| C1 Wash Inside | `curvyo-02` | R1, R2 Inside | sample, not a pixel diff: the LOGO block inside R1 `#B5C1DC` ± 3, the triangle (outside) at 12 % (white `#FFFFFF` stays white; the yellow `#F2B705` becomes `#FDF6E1` ± 3) |
| C2 Wash Outside | `curvyo-05` | R1, R2 Outside | LOGO block `#E6EAF3` ± 3; no preview outline inside R1 or R2 |
| C3 Region line | `curvyo-02` | R1 at 100 % | a 1 px line `#000000` (± 24) along R1; at 400 % 4 px (3.8 rounded) |
| C4 Selected rectangle | `curvyo-20` | R1 selected | 8 handles 8 x 8 at the corners and edge midpoints of R1's box, white inside, `#2F6FEE` ring; the dashed box `#2F6FEE`; no other glyph |

Not compared by pixels: the canvas outside C1 to C5 (Curvyo's page, rulers and placement differ),
cursors, shadow halos, tooltip x, the drag ghost.

## 13. Differences from Curvyo and from the spec draft (for the lead)

| # | Item | Here | Spec draft / Curvyo |
|---|---|---|---|
| 1 | Region outline | black, 0.25 mm at zoom, at least 1 px (Curvyo's mask path) | spec 11 follows (2026-11-12) |
| 2 | Other parts' regions | drawn, not editable (Curvyo's mask paths are always visible) | spec 11 follows |
| 3 | Draw modifiers | Shift from the centre, Ctrl square; resize Shift about the centre, Ctrl keep proportions (Curvyo) | spec 7, 9 follow |
| 4 | Without a picture | no Mask group, no Parts (`0001`) | spec 1 follows |
| 5 | Bar rows | two rows at 1280 px (the group is 32 px wider than Curvyo's; with Curvyo's 192 px the bar would wrap too) | `0001 §3.3` one row |
| 6 | Selected region | no rotate, skew, radius or centre handles | Curvyo's Select tool shows them |
| 7 | Size chip | picture px | Curvyo mm |
| 8 | Region tool announcement, Save as… tooltip with parts | "Rectangle on"; "Save as… (Ctrl+Shift+S), 3 parts joined" | new strings |

**Frage 6 (REGION-ALT): Which key subtracts?** Decided by the lead, 2026-11-12: **C**, a Subtract
toggle after Rectangle (section 1). Alt+drag is taken by Linux window managers (`docs/design-system.md`
of Curvyo, "Bend readout and badge"), and the customer uses Linux. The original options were A: Alt
held at the press; B: Ctrl+Alt; C: a toggle; the original recommendation was A (most desktops use Super for window moves).
