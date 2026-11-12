# UX notes: Trace and save

Status: Draft (ux-engineer, 2026-11-11). Reference: Curvyo `main` at `fc8edca8`, its dev server
rendered in Chromium at 1x. Rule: **what Curvyo draws, Tracyo draws**; where Curvyo's code and
`docs/design-system.md` disagree, the code (what renders) wins and the difference is named.
Own choices are marked **(Proposal)**. No hint or tip text; labels that name a control or a state
stay. All sizes are logical px at scale 1.

Sources: `~/curvyo/frontend/src/index.css`, `components/TraceBar.tsx`, `TraceAdvanced.tsx`,
`TraceField.tsx`, `TracePresetMenu.tsx`, `TracePalette.tsx`, `TraceColourGroup.tsx`,
`ColourPickChip.tsx`, `ImportCard.tsx`, `CanvasNotice.tsx`, `StatusBar.tsx`, `panel/ValueField.tsx`,
`panel/EntryField.tsx`, `panel/Swatch.tsx`, `panel/TraceInfoPage.tsx`, `panel/FileTab.tsx`,
`lib/traceText.ts`, `lib/tracePaletteText.ts`, `lib/cursors.ts`; `docs/tracing-images.md`.

Reference pictures (`reference/`, made by `reference/capture.cjs` from `reference/input-logo.png`):

| File set | What |
|---|---|
| `curvyo-NN-*.png` | Curvyo as it is (1280 x 800, panel open): 00 bar closed, 01 empty, 02 preview, 03 Advanced open, 04 Multicolour with palette, 05 running 42 %, 06 failed, 07 stopped, 08 Preset menu, 09 Method menu, 10 swatch hover, 11 swatch excluded, 12 value-field tooltip, 13 Trace button focus, 14 Preset hover, 15 tooltip |
| `px-NN-*.png` | The same states, **Tracyo-equivalent**: Mask group and Parts hidden, finish button "Save as…", empty bar without its sentence and with "Open image…", 1360 x 800 with the panel hidden, so the bar has exactly the width it has in a 1280 x 800 Tracyo window. These are the pixel references of section 12 |

Running and failed were staged by wrapping the trace worker (progress 42 %, an out-of-memory
error); everything drawn is Curvyo's own code. The failed tooltip in the pictures is not shown.
**The pictures were rendered with Liberation Sans** (this machine has no Segoe UI, Roboto or Noto
Sans; Chromium takes "Arial" from Curvyo's font stack and fontconfig maps it to Liberation Sans).

## 1. Window

| Item | Value |
|---|---|
| Title | "Tracyo" without a picture; "logo.png — Tracyo" with one (file name, space, U+2014, space; Curvyo's "<file> — Curvyo"). No dirty mark |
| Size | 1280 x 800 at first start, then the remembered size; minimum 800 x 600 (Curvyo's check size). Native decorations |
| Theme | Light only. Curvyo has one theme (its `.dark` block is shadcn's and redefines none of the tokens below) |
| Regions at W x H | **Canvas** (0, 0) to (W, H - 24), ground `#E8E8EB`. **Open card** (12, 12), 48 x 48, floating. **Trace bar** (72, 12), floating, width min(content, W - 84). **Status bar** (0, H - 24), W x 24. **About panel** (W - 280, 0), 280 x (H - 24), docked, only while open; the canvas then ends at W - 280 and the view keeps its origin |
| Stacking | canvas < picture < wash < preview outline < drop cue < notice slot < Open card, bar < menus < tooltips |

**Where Open and Save live, and why.** Tracyo has no Properties panel; a 312 px panel for two
actions would take a quarter of the canvas. **Save** is the blue finish button at the far right of
the bar (spec criterion 22): Curvyo's place for "finish this tool's job", so the end of the flow is
where the eye already is. **Open** is Curvyo's own **Import card**: the rail toolbox card that
holds Curvyo's button for bringing a raster image in, unchanged in look, at the rail's place
(12, 12). It costs 60 px of width instead of 126 px inside the bar, it never moves between the
empty and the loaded state, and the bar stays one row at 1280 px (section 3.3). The File tab's
own look (outline text buttons) would need a panel to sit in; its vocabulary stays: labels end
in "…" (U+2026) as "Open…" and "Save as…" do there, and tooltips name the chord. **(Proposal)**

## 2. Tokens

### 2.1 Colours (copied from `index.css`; mixes flattened to the ground they sit on)

| Token | Hex | Use |
|---|---|---|
| `--canvas-bg` | `#E8E8EB` | canvas ground (also the ground under transparent pixels) |
| `--toolbar-bg` = `--panel-bg` = `--statusbar-bg` | `#DCDCE0` | bar, Open card, menus, chip, notice, About panel, status bar |
| `--toolbar-icon` | `#3A3A3F` | text and glyphs of the chrome, swatch border |
| `--toolbar-icon-active-bg` = `--editor-accent` = `--preview-new` | `#2F6FEE` | pressed ground, blue buttons, focus ring, value edge, preview line |
| `--toolbar-icon-active-fg` | `#FFFFFF` | text and glyph on pressed or blue ground |
| `--editor-accent-hover` | `#2F6FEE` at 20 % = `#B9C6E3` on the bar, `#D5E2FC` on white | hover ground; focused or marked menu row |
| `--panel-muted-fg` | `#5A5A5F` | unit text, About texts, modifier word in a drag |
| `--field-border` | `#3A3A3F` at 60 % = `#89898C` on white | value-field border at rest |
| border at 40 % | `#3A3A3F` at 40 % = `#9B9BA0` on the bar | Preset and Method button border, segmented outline |
| divider at 25 % | `#3A3A3F` at 25 % = `#B4B4B8` on `#DCDCE0` | Advanced divider, About panel edge, pipette chip border |
| `--value-fill` | `#C5D7FA` | value-field bar (accent 28 % into white) |
| `--field-invalid` | `#B3261E` | alert notice text and ring |
| `--swatch-border` | `#3A3A3F` | swatch 1 px border |
| `--checker-a` / `--checker-b` | `#FFFFFF` / `#C9C9CE` | background chip checkerboard |
| foreground | `#0A0A0A` (oklch 0.145) | status-bar text, tooltip text |
| popover | `#FFFFFF` | tooltip ground; ring `#0A0A0A` at 10 % = `#E7E7E7` |
| blue hover | `#347AFF` | `brightness-110` of `#2F6FEE` (Save as…, Analyse) |
| running fill | `#6393F2` | white 25 % over `#2F6FEE`, from the left to the percent |
| white 20 % on blue | `#598CF1` | hover of the x and of Retry inside the blue button |
| dimmed blue button | ground `#86A6E7`, text `#EEEEF0` | the whole button at 50 % over the bar (flattened) |
| text selection | `#3367D1`, text `#FFFFFF` | selected text in a number entry (Chromium's) |

### 2.2 Shape, spacing, shadow

| Item | Value |
|---|---|
| Radii | bar, menus, Open card: **10 px** (`rounded-lg`; the design doc says 8, the code renders 10); buttons, segmented outline, tooltips, notice, chip, menu rows: **8 px**; value field, outline button, Swatch: **5 px**; swatch in a cell: **4 px** |
| Shadow | `0 2px 8px rgba(0,0,0,0.24)` on bar, Open card, menus, notice; none on tooltips, chip, status bar |
| Bar | padding 0 8 px; rows 36 px; controls 28 px with 4 px above and below; 8 px between items, 0 between rows |
| Focus ring (most) | 2 px `#2F6FEE` outside a 1 px `#DCDCE0` gap (`ring-2 ring-offset-1`): icon buttons, value fields, blue buttons |
| Focus ring (Preset, Method, outline and text buttons) | 2 px `#2F6FEE`, no gap |
| Focus ring (segments) | 2 px `#2F6FEE` inside the segment |
| Focus visibility | keyboard focus only (`:focus-visible`); a mouse press shows no ring |

### 2.3 Type

| Use | Size / line | Weight | Colour |
|---|---|---|---|
| controls, field label and value, menu name, About text | 14 / 20 | 400; menu name 600 | `#3A3A3F` |
| unit in a field | 14 / 20 | 400 | `#5A5A5F` |
| small labels ("Trace", "Denoise", "Upscale"), hex, percent in the running button, "Stopped", "Failed", tooltips, status bar, notice, chip | 12 / 16 | 400; tooltip first line of a value field 500 | as the surface says |
| menu second line | 12 / 16 | 400 | `#3A3A3F` at 70 % = `#6B6B6F` |
| palette tooltip second line | 10 / 14 | 400 | 70 % |

Font: Curvyo's stack `-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue",
"Noto Sans", Arial, sans-serif`. Resolved: Windows Segoe UI; macOS San Francisco; Linux the first of
Roboto, Noto Sans, Arial (Liberation Sans) that is installed. Digits are tabular in values,
percent, hex and the status bar. Tracyo resolves the same list in the same order per OS and
ships Liberation Sans (SIL OFL 1.1) Regular and Bold as the last fallback; weight 600 takes the
face's semibold or bold, weight 500 its regular (what Chromium does) **(Proposal; Frage 5)**.

## 3. The Trace bar

### 3.1 Surface and flow

Pill `#DCDCE0`, radius 10, shadow, padding 0 8. One flow of whole items, wrapping by whole items
when the width runs out, top-aligned; the expander and the blue button are one item pushed to the
far right of their row (`margin-left: auto`). The bar is as wide as its items, at most W - 84. No
Mask group, no Parts, no state label while a picture is shown, no status or notice text in the
bar; one hidden status region (AccessKit live region) carries what Curvyo announces.

### 3.2 Items, left to right (with a picture)

| # | Item | Size | Shown |
|---|---|---|---|
| 1 | About ("i") | 28 x 28 | always |
| 2 | Preset | 132 x 28 | always |
| 3 | Method | 176 x 28 | always |
| 4 | Threshold (Brightness) or Colour tolerance | 150 x 28 | always |
| 5 | Remove background group: chip 28, hex 56, Auto 44, pipette 28, 4 px apart (168) / Pick colours group: pipette 28 + 24 per picked colour (220 at 8) | as stated | that method |
| 6 | Smooth | 150 x 28 | always |
| 7 | Tolerance | 150 x 28 | always |
| 8 | Ignore below | 171 x 28 | always |
| 9 | expander 28, 8, **Save as…** (blue) | 28 + 8 + 89 | always, far right |
| 10 | Advanced block | full width under the rows | while open |

### 3.3 Rows (Brightness, Advanced closed)

Content width: 8 + 28 + 132 + 176 + 150 + 150 + 150 + 171 + 28 + 89 + 8 x 8 + 8 = **1154 px**.
At 1280 x 800 (bar width up to 1196): **one row, 36 px**. Remove background adds 176: two rows.
At 800 x 600 (up to 716): two rows (i to Smooth 684 px; Tolerance, Ignore below, expander and button). `px-00-bar-closed.png` shows the one-row bar.

**Blue button width (lead, 2026-11-11; the reference wins over the earlier Proposal):** at rest
"Save as…" is 89 px (Liberation Sans 14 px, 12 px side padding); while running it is 75 px, as in
`px-05-running-42.png` (Curvyo's `w-[75px]`), so the expander moves 14 px right while a trace
shows its percentage. Stopped is 8 + "Stopped" (12 px) + 4 + Retry (8 + text + 8) = 106 px, Failed
8 + 12 + 4 + "Failed" + 4 + Retry = 110 px (measured in `px-07`, `px-06`).

**Bar width, as the reference measures it.** The bar is as wide as all its items on one line (the
open Advanced block counts as one more zero-width item, so one more 8 px gap: 1162 px at rest, 1148
while running), at most the cap. When the items do not fit on one line the bar takes the whole cap,
the items wrap by whole items and the expander with the blue button goes to the far right of its
row. The reference pictures were captured with a cap of 1164 px (`px-06`, `px-07`, `px-09`), not
the 1196 px of a 1280 px window; the pixel tests for those three use a 1248 px wide window (cap =
width - 84). With the cap of 1196 a Stopped bar with the Advanced block open (1179 px) stays on one
row.

### 3.4 Tab order

Open (card), About, Preset, Method, main field, method group (Auto, pipette; or pipette, then the
picked swatches as one stop), Smooth, Tolerance, Ignore below, expander, blue button (or its x, or
Retry), then the Advanced block (Output one stop, Colours, palette pipette, palette one stop,
Invert, Drop border shapes, Recognise text, Recognise shapes, Denoise one stop, Upscale one stop,
Corners, Analyse), then the About panel when it scrolls. A mouse press on any bar button hands
the keyboard back to the canvas; a key press keeps focus on the control.

## 4. Controls

Tooltips: Curvyo's house tooltip (section 6) after 400 ms, below the control, 8 px away, unless
stated. "Value-field tip" = two lines: "<Name>: <value> <unit>" (weight 500) and "Click to set,
double-click to type" (70 %), opening to the **left** of the field.

| Control | Label (drawn) | Accessible name | Default | Range, unit, steps | Tooltip |
|---|---|---|---|---|---|
| Open card button | glyph `Import` 20 px | "Open image" | | | "Open image (Ctrl+O)" (Cmd on macOS), to the right, 10 px past the card |
| About | glyph `Info` | "About Tracyo" | | | "About Tracyo" |
| Preset | preset name or "Custom", 10 px chevron | "Preset" | Logo / Line art | Logo / Line art, Photo, Silhouette for cutting/carving, Engraving | none |
| Method | method name, chevron | "Method" | Brightness (analysis may change it) | Brightness, Remove background, Pick colours | none |
| Threshold | "Threshold" … "62 %" | "Threshold" | image's automatic value | 0 to 100 %, step 1, Shift 10, Ctrl 1 | value-field tip |
| Colour tolerance | "Colour tolerance" … "40" | "Colour tolerance" | 40 (Remove background), 19 (Pick colours) | 0 to 100, step 1, Shift 10 | value-field tip |
| Background chip | 20 px swatch (checkerboard when transparent) | none (decorative) | Auto's colour | | none |
| Background hex | "#FFFFFF" or "none", 12 px | (text) | | | none |
| Auto | "Auto", 44 x 28 text toggle | "Auto" | pressed | | none |
| Background pipette | `Pipette` | "Pick background colour from the image" | | | "Pick a colour from the image" |
| Pick colours pipette | `Pipette` | "Pick colour from the image" | armed on choosing the method | up to 8 colours | "Pick a colour from the image" |
| Picked swatch | 20 px colour | "Remove #3366CC" | | | "#3366CC" |
| Smooth | "Smooth" … "80" | "Smooth" | 80 | 0 to 100, step 1, Shift 10 | value-field tip |
| Tolerance | "Tolerance" … "0.50 px" | "Tolerance" | 0.50 px | 0.10 to 3.00 px, step 0.05, Shift 0.5, Ctrl 0.01 | value-field tip |
| Ignore below | "Ignore below" … "0.50 mm" | "Ignore below" | 0.50 mm | 0 to 10 mm, step 0.1, Shift 1, Ctrl 0.01 | value-field tip |
| Expander | `ChevronsDown` / `ChevronsUp` | "More trace options" / "Fewer trace options" | closed (remembered) | | the name; while closed with changed Advanced values a second line "Changed: Invert, Corners" |
| Blue button | "Save as…" | "Save as…" | | | "Save as… (Ctrl+Shift+S)"; dimmed: the reason (section 8) |
| Stop | `X` inside the blue button | "Stop (Esc)" | | | "Stop (Esc)" |
| Retry | "Retry" | "Retry" | | | none; the "Stopped" text has "Preview is out of date", "Failed" has the failure sentence |
| Output | "Single colour" \| "Multicolour" | "Output" | Single colour (remembered) | | none |
| Colours | "Colours" … "Auto 3" (one space, one colour: the reference renders it so; a bar at 0 draws no bar and no edge) | "Colours" | Auto | Auto, 2 to 16, step 1, Shift 4; 120 px wide; with Pick colours dimmed, showing the number of picked colours | value-field tip |
| Palette pipette | `Pipette` | "Pick palette colour from the image" | | | "Pick a colour from the image" |
| Palette swatch | 20 px colour | "#4B392F · Brown" (", excluded") | | | "#4B392F · Brown" and, 10 px, "Ctrl+click: exclude" (Cmd on macOS; Curvyo's text, kept) |
| Invert | `Contrast` | "Invert" | off at every start | | "Invert" |
| Drop border shapes | `ImageMinus` | "Drop border shapes" | off at every start | | "Drop border shapes" |
| Recognise text | `ScanText` | "Recognise text" | off (remembered) | | "Recognise text" |
| Recognise shapes | `Shapes` | "Recognise shapes" | off (remembered) | | "Recognise shapes" |
| Denoise | label "Denoise" (64 px slot), "Off" \| "Low" \| "High" | "Denoise" | Off | | none |
| Upscale | label "Upscale", "Auto" \| "Off" | "Upscale" | Auto (preset) | | none |
| Corners | "Corners" … "70" | "Corners" | 70 | 0 to 100, step 1, Shift 10 | value-field tip |
| Analyse | "Analyse", 75 x 28 blue | "Analyse" | | | none |

Preset values (Smooth, Tolerance, Ignore below, Corners, Upscale, Denoise): Logo / Line art 80,
0.5, 0.5, 70, Off, Off; Photo 50, 0.5, 0.1, 30, Off, Off; Silhouette for cutting/carving 100, 1.5,
2.0, 70, Off, Low; Engraving 80, 0.35, 0.1, 70, Auto, Off. Second lines: "Sharp corners, clean
curves.", "Many shapes. Photos trace roughly.", "Closed, simple, few nodes.", "Keeps fine lines.".
Method second lines: "Dark and light pixels by a cut-off.", "Everything unlike the background.",
"Pixels like the colours you picked.".

## 5. Anatomy of each kind of control

| Kind | Rest | Hover | Pressed / on | Focus (keyboard) |
|---|---|---|---|---|
| Icon button 28 x 28 (About, pipettes, toggles, expander) | no ground, glyph 16 px `#3A3A3F` | ground `#B9C6E3`, radius 8 | ground `#2F6FEE`, glyph white | ring 2 + gap 1 |
| Preset, Method | 1 px `#9B9BA0` border, radius 8, text 14 px left at 8 px, chevron 10 px right at 8 px, text truncated with "…" | ground `#B9C6E3` | menu open: same as hover | ring 2, no gap |
| Text button (Auto, empty-state "Open image…") | no border, 8 px side padding, 14 px | ground `#B9C6E3` | Auto on: `#2F6FEE` / white | ring 2, no gap |
| Segmented (Output, Denoise, Upscale) | 1 px outline inside in `#3A3A3F` at 40 % drawn over the segments too (`#9B9BA0` on the bar, darker blue over the selected segment), radius 8, segments min 28 px, 8 px padding (Output 6 px), no gaps | segment ground `#B9C6E3` | segment `#2F6FEE` / white (corner segments clipped to the radius) | ring 2 inside the segment; one Tab stop, arrows move and select, no wrap |
| Value field | white, 1 px `#89898C`, radius 5; label at 8 px left; value 8 px from the right, unit 4 px after it in `#5A5A5F`; bar `#C5D7FA` from the left to the value with a 2 px `#2F6FEE` right edge; where the edge would cross the label or value text only its top and bottom 6 px are drawn | border `#3A3A3F`; cursor ew-resize | while dragged: centred 12 px word "coarse" (Shift) or "fine" (Ctrl) in `#5A5A5F`; cursor ew-resize everywhere until release | ring 2 + gap 1 |
| Value field typing (double-click, Enter, F2, a digit) | white, 1 px `#2F6FEE` border plus 1 px inset `#2F6FEE`, label stays left, number right-aligned before the unit, all selected (`#3367D1` / white) | | | Enter or Tab commit, Escape restores |
| Blue button | `#2F6FEE`, white 14 px, radius 8, 12 px padding | `#347AFF` | | ring 2 + gap 1 |
| Swatch cell 24 x 28 | 20 x 20 colour, 1 px `#3A3A3F`, radius 4, opaque; cells touch (4 px between swatches) | swatch at 40 % (flatten to the bar) and `X` 12 px over a 3 px `#DCDCE0` casing; with Ctrl held `Ban` instead | excluded: 40 % and `Ban`; hover shows `X` | ring 2 around the 20 px swatch, and the hover look |
| Menu (Preset, Method) | 8 px below the button (above if it would leave the window; the reference shows 8), left edges aligned, min 232 px wide, `#DCDCE0`, radius 10, shadow, 4 px top and bottom; rows 44 px: 4 px inset, 8 px padding, radius 8; a 12 px check column ("✓" for the current), name 14/20 weight 600, second line 12/16 at 70 % with 20 px indent | row ground `#B9C6E3` (follows the pointer and the keys) | after an edit ("Custom") the preset's row keeps `#B9C6E3` and no check | ring 2 on the row |

Hidden, not disabled: a control that does not apply is absent (method groups, Colours, palette).
The only dimmed controls are the blue button (nothing to save), the Colours field with Pick
colours (50 %, showing the number of picked colours) and nothing else.

## 6. States of the bar and the canvas

| State | Bar | Canvas |
|---|---|---|
| No picture | one row: "Trace" (12 px `#3A3A3F`, 8 px vertical padding), 12 px, text button "Open image…"; nothing else (`px-01-empty.png`) **(Proposal)** | `#E8E8EB` only; status bar zoom "100%" |
| Opening | unchanged | cursor wait; files of 5 MB or more: status notice "Opening photo.jpg..." until shown |
| Preview | as section 3 (`px-00`, `px-02`) | picture with a white veil at 65 % (shows 35 %), outline 1.5 px `#2F6FEE` over a 4.5 px white casing, holes outlined, no fill, constant on screen |
| Running (from 400 ms, held at least 300 ms once shown; at once for a full trace before saving) | blue button: fill `#6393F2` from the left to the percent, "42 %" 12 px at 8 px left ("..." before the first percent, no fill), `X` 28 x 28 at the right (`px-05`) | outline and casing at 50 % alpha |
| Reading text (Recognise text, first preview) | same as Running; progress name "Reading text" | same |
| Stopped (x or Esc) | blue button: "Stopped" 12 px at 90 %, then "Retry" 14 px (white, hover `#598CF1`) (`px-07`) | outline stays at 50 % |
| Failed | blue button: `CircleAlert` 12 px, 4 px, "Failed" 12 px, then "Retry" (`px-06`); tooltip "Trace failed: <reason>. Nothing was saved." (reason "out of memory at 16 Mpx" or "internal error") | outline as before the failure |
| Nothing to save | blue button dimmed (section 2.1), no hover, tooltip = reason | |
| Pipette armed | the pipette pressed | cursor: Curvyo's pipette (black 1.5 px over white 4 px, 24 px, hotspot at the tip, 2/22); chip 12 px up and right of the pointer, flips at the edges: `#DCDCE0`, 1 px `#B4B4B8`, radius 8, 6 x 2 px padding, 16 px swatch, 6 px, "#3366CC" 12 px tabular, or "No colour here" |
| Files dragged over the window | unchanged | 2 px dashed outline inside the canvas edge, no text, no animation (see section 11 for the colour) |
| Open failed | unchanged; picture and values stay | alert in the notice slot (section 8) |

Picture placement: on open, centred in the canvas area below the bar (top = bar bottom + 12,
12 px from the other edges), at min(100 %, fit) **(Proposal)**; Ctrl+0 fits (may enlarge). No
frame, no shadow, no checkerboard (Curvyo shows the ground through transparent pixels).
Filtering: linear with mipmaps; nearest from 800 % (a picture pixel 8 screen px or more), as
Curvyo's image renderer. Status bar: 24 px `#DCDCE0`, 12 px `#0A0A0A` tabular, 8 px side padding,
three columns: left "x: 312  y: 87 px" (pointer over the picture, picture pixels, two spaces),
centre zoom "100%", right "640 × 420 px" **(Proposal: Curvyo's bar with picture pixels)**.

## 7. Tooltips, notices, About panel

| Surface | Look |
|---|---|
| Tooltip | `#FFFFFF`, text `#0A0A0A` 12/16, radius 8, padding 4 x 8, 1 px ring outside the box in `#0A0A0A` at 10 % (`px-10`), max 256 px wide, no shadow, 400 ms, below the control 8 px (value fields: left); flips when it would leave the window. A second line of 10 px text sits in a 13 px line, 1 px above its middle |
| Notice slot | centred in the canvas right of x = 60, 12 px above the canvas bottom; `#DCDCE0`, radius 8, padding 8, 12 px, shadow; status: `#3A3A3F`, 3 s (5 s above 70 characters); alert: text `#B3261E`, plus 1 px `#B3261E` ring, 8 s; the next notice replaces it; no controls, no focus |
| About panel | 280 px, `#DCDCE0`, 1 px `#B4B4B8` line on its canvas side; header 12 px from the top: "About Tracyo" 14 px `#3A3A3F` weight 400; 8 px; body 12 px side padding, 244 px text: intro 14 px `#3A3A3F`, 12 px, entries 12 px apart, term 14 px weight 600 `#3A3A3F`, text 14 px `#5A5A5F`, 4 px between; scrolls with a thin bar when long. Content per spec criterion 26. Opened and closed only by "i" (pressed while open); Esc in the panel returns focus to the canvas and leaves it open (Curvyo) **(Proposal: a docked panel, not a separate window)** |

## 8. Empty, error and reason texts

| Case | Where | Text |
|---|---|---|
| No picture | bar | "Trace", button "Open image…" |
| Unsupported file | notice, alert | first line the file name (cut to 24 characters, ellipsis in the middle, extension kept), then "This file type isn't supported. Use PNG, JPEG or WebP." |
| Damaged file | notice, alert | name, then "The image is damaged and could not be read." |
| Out of memory while opening | notice, alert | name, then "Not enough memory to open this image." |
| Several files dropped | | the first image file is opened, the rest ignored, no notice **(Proposal)** |
| Nothing to save | blue button tooltip | "0 paths at this Threshold: too low." / "too high."; "0 paths at this Colour tolerance: too high." (Remove background) / "too low." (Pick colours); "No colours found."; "0 paths: no colour picked."; "0 paths: no colour in the palette."; "Too much detail: raise Ignore below." |
| Trace failed | blue button tooltip | "Trace failed: out of memory at 16 Mpx. Nothing was saved." / "Trace failed: internal error. Nothing was saved." |
| Write failed | blue button "Failed" + Retry | "Could not save logo.svg: <system reason>. Nothing was written." **(Proposal)** |
| Saved | nothing (Curvyo shows no success notice) | |

The notice texts are Curvyo's import notices (`docs/importing-images.md`) with SVG removed from
the list. Spec criterion 2 asks for "Cannot read this image." in the bar; Curvyo's bar never
shows a notice, so Tracyo uses Curvyo's notice slot and words **(Proposal)**.

## 9. Open and save dialogs

| | Open | Save as… |
|---|---|---|
| Dialog | OS native (`rfd`, the crate Tauri uses for Curvyo's dialogs) | OS native |
| Title | "Open image" | "Save as" |
| Filters | "Images (*.png, *.jpg, *.jpeg, *.webp)", then "All files"; one file | "SVG (*.svg)" |
| Start | the folder of the last open | the picture's folder, name "<picture stem>.svg" |
| Overwrite | | the OS dialog's own question; no dialog of Tracyo's |
| Cancel | nothing happens; focus back on the Open button | nothing happens; focus back on the blue button |
| Done | picture shown, keyboard on the canvas | no notice; keyboard stays where it was, a mouse press returns it to the canvas |
| Busy | cursor wait while decoding | the blue button shows Running while the full picture traces |

## 10. Keyboard

Ctrl means Cmd on macOS. Keys on the canvas act when no text entry has focus.

| Key | Action |
|---|---|
| Ctrl+O | Open image… |
| Ctrl+Shift+S, Ctrl+S **(Proposal: both)** | Save as… |
| Enter (canvas) | Save as… at rest; Retry when Stopped or Failed |
| Ctrl+Enter (in a field or its entry) | Save as… |
| Double-click on the picture | what Enter does (Curvyo) |
| Esc | stop a running trace; else disarm the pipette; else close an open menu or entry; else return the keyboard to the canvas. Never closes the window |
| Tab, Shift+Tab | section 3.4 |
| Value field | Left/Down -1 step, Right/Up +1; Shift big step, Ctrl fine step; Home/End the limits; Enter or F2 or a digit opens the entry; Enter or Tab commits (clamped), Escape restores; Backspace and Delete do nothing |
| Preset, Method | Enter, Space or Down opens on the current item; Up, Down, Home, End without wrap; Enter or Space picks; Escape or Tab closes |
| Segmented | arrows move and select, no wrap; Escape returns to the canvas |
| Picked swatches | Left, Right, Home, End move; Delete, Backspace, Enter, Space remove |
| Palette swatches | Left, Right, Home, End move; Enter or Space exclude/include; Shift+Enter, Delete, Backspace delete |
| Pipette armed | click picks (Pick colours: replaces), Shift+click adds (up to 8), right press disarms |
| Ctrl+0 | fit the picture |
| Ctrl+scroll, pinch | zoom at the pointer, 2 % to 8000 % (Curvyo's range; spec says 5 to 6400, either is fine) |
| Scroll, middle drag, Space+drag | pan |

## 11. Icons

Lucide 1.50.0 (Curvyo's `lucide-react` version, ISC licence: ship its notice). Take the SVGs of
the same version (`lucide-static@1.50.0`). Stroke widths are in the 24-unit viewBox unless
"absolute"; the rendered width is stroke x size / 24.

| Icon | Size | Stroke (rendered) | Where |
|---|---|---|---|
| `Import` | 20 | 2 (1.67 px) | Open card |
| `Info` | 16 | 1.5 (1.0 px) | About |
| `ChevronsDown`, `ChevronsUp` | 16 | 1.5 (1.0 px) | expander |
| `X` | 16 | 1.5 (1.0 px) | Stop in the blue button |
| `CircleAlert` | 12 | 2 (1.0 px) | Failed |
| `Pipette` | 16 | 1.5 (1.0 px) | the three pipettes |
| `Contrast`, `ImageMinus`, `ScanText`, `Shapes` | 16 | 1.5 (1.0 px) | Invert, Drop border shapes, Recognise text, Recognise shapes |
| `X`, `Ban` | 12 | absolute 1.5 over absolute 7.5 in `#DCDCE0` | swatch remove, excluded |
| chevron (Curvyo's own) | 10 x 10 | 1.5, path `M2 3.5 L5 6.5 L8 3.5` | Preset, Method |
| pipette cursor (Curvyo's own, `lib/cursors.ts` `EYEDROPPER`) | 24 | black 1.5 over white 4 | armed pipette |
| "✓" (U+2713, a text glyph) | 14 px text | | current menu row |

Expander changed-dot: 6 px `#2F6FEE` disc, 2 px from the button's top and right, 1 px `#DCDCE0`
ring. **Drop cue colour:** Curvyo's code uses `var(--accent)`, which is shadcn's near-white
`#F5F5F5`, not the `#2F6FEE` its design system names; Tracyo draws `#2F6FEE` as designed
**(Proposal; a Curvyo bug to report)**. Its dash pattern was not rendered here; use 6 px dash,
6 px gap and check against Curvyo once the cue is fixed there.

## 12. egui mapping and pixel comparison

### 12.1 What egui cannot reproduce 1:1, and the nearest approach

| Curvyo look | egui | Nearest |
|---|---|---|
| Glyph rasterising (Skia: hinting, subpixel positioning; ClearType colour fringes on Windows; CoreText on macOS) | different rasteriser, greyscale only | same font files and sizes; accept glyph-edge differences (12.2 tolerates them); line height set to 20/16 px explicitly |
| macOS San Francisco (variable font, weight axis) and `tabular-nums` (OpenType `tnum`) | no OpenType features; variable axes only if the egui version's rasteriser supports them | on macOS SF digits are proportional by default, so values would jitter: use Liberation Sans or Roboto there **(Frage 5)** |
| Weights 500/600 synthesised or picked by CSS matching | one file per weight, no synthetic bold | load the face's semibold or bold file for 600; 500 uses regular |
| "✓" from a fallback font | no system fallback | add a fallback font that has U+2713, or draw the check as a path |
| Group opacity (`opacity-50` dimmed button, 40 % swatch fade, 50 % stale palette) | opacity multiplies each shape, it does not composite the group | use the flattened colours of section 2.1 |
| `color-mix`, `oklch` | sRGB only | precomputed hex (section 2.1) |
| Box shadow `0 2px 8px rgba(0,0,0,.24)` | `Shadow { offset: [0, 2], blur: 8, spread: 0, color: black alpha 61 }`; egui's falloff is not Gaussian | accept a low-contrast halo difference |
| Backdrop blur | not used by Curvyo's Trace UI | nothing to do |
| Focus ring only after keyboard focus | egui has no focus modality | remember whether the last focus change came from a key |
| Tooltip anchored below the control, flipping at edges, 400 ms | default tooltips follow the pointer | own `Area` anchored to the widget rect; `tooltip_delay = 0.4` |
| Rounded clipping of segment grounds | rectangular clip only | per-corner radii on the first and last segment |
| Custom cursor image (pipette) | eframe offers system cursors only | `CursorIcon::Crosshair` (Curvyo's own fallback); exact only with a winit custom cursor outside eframe |
| Drag-and-drop hover cue | winit reports hovered files on Windows, macOS and X11, **not on Wayland** (no drop at all there) | run under XWayland if the cue and drop are required on Wayland; Ctrl+O always works |
| Thin overlay scrollbar (Advanced block over 360 px, About panel) | egui scroll bars look different | `ScrollStyle::thin()`; not compared |
| Number entry selection colour `#3367D1` | egui selection colours are settable | set `selection.bg_fill` to `#3367D1`, text white |

### 12.2 Pixel comparison (tester)

1. Reference: the `px-NN-*.png` pictures. They must be made on the machine where Tracyo is
   photographed, with the same UI font; re-run `reference/capture.cjs` there (instructions at its
   top) after checking which font Chromium uses (DevTools: Rendered Fonts of the bar; on this
   machine Liberation Sans).
2. Tracyo: 1280 x 800 window, `pixels_per_point = 1`, the same picture (`input-logo.png`), the
   same states, steps and values as the script (Method Brightness; Advanced; Multicolour; Ctrl+click
   the second swatch; Smooth +1 for running). Capture the bar with 12 px around it, by
   `egui_kittest` snapshot rendering or a window screenshot at 1x.
3. Run `python3 reference/compare.py px-00-bar-closed.png tracyo-00-bar-closed.png diff.png` for
   each state. It aligns both on the bar's top and left edge, compares with a 1 px neighbourhood
   and a tolerance of 24 per channel, and passes when at most 2 % of the pixels differ and no row
   or column holds a run of 12 or more differing pixels. Self-test: identical 0 %; a 0.5 px blur
   (stands for another rasteriser) passes; a 30 x 20 px block or a field 2 px narrower fails.
4. Look at `diff.png`: red may sit on glyph edges only, never along a border, a gap or a fill.
5. Not compared by pixels: shadow halo, tooltips' exact x, scroll bars, the cursor.
6. **Regions (slice 2).** The references with the Advanced block open show controls of later
   slices (the Output pill, with Multicolour its Colours field and palette, the Recognise
   toggles). `tests/pixels.rs` compares each control that exists where it is, region by region,
   each with the tolerance of step 3: the first row or both rows, the divider, Invert and Drop
   border shapes (px-03 only), Denoise to Corners (shifted by the missing controls: 241 px in
   px-03, 469 px with the Multicolour row), the gap, Analyse. Slice 3 added the Output pill,
   Colours and the palette; only the two Recognise toggles (slice 4) are still skipped (64 px, Denoise to Analyse shifted by them). The tests that need the
   reference's 1164 px cap run in a 1248 px window (section 3.3).

## 13. Differences from Curvyo and from the spec draft (for the lead)

| # | Item | Here | Note |
|---|---|---|---|
| 1 | Mask group, Parts, Part label | absent | spec criterion 10 |
| 2 | Finish button | "Save as…", 89 px at rest, 75 px while running (as the reference) | spec criterion 22; section 3.3 |
| 3 | Empty state | Curvyo's one-row bar without the sentence; no dimmed controls | spec criterion 3 asks for dimmed controls; Curvyo never shows them without an image |
| 4 | Open error | notice slot with Curvyo's import texts | spec criterion 2 asks for "Cannot read this image." in the bar |
| 5 | About | docked 280 px panel, "About Tracyo", Esc keeps it open | spec criterion 26 says a small window, Esc closes |
| 6 | "…" vs "..." | U+2026 in "Open image…", "Save as…" (Curvyo's File tab) | spec writes "..."; Curvyo's Trace bar has "Import image..." with three dots |
| 7 | Theme | light only | spec Out of scope says light and dark follow the system "as Curvyo does"; Curvyo has no dark theme |
| 8 | Status bar | Curvyo's, with picture pixels | not in the spec |
| 9 | Blue button tooltip at rest | "Save as… (Ctrl+Shift+S)" | Curvyo's Trace button has none at rest (the design doc says "Create paths (Enter)"); File tab tooltips name the chord |
| 10 | Drop cue colour | `#2F6FEE` | Curvyo renders `#F5F5F5` (bug) |
| 11 | Ctrl+S | also Save as… | spec names Ctrl+Shift+S only |

**Frage 5 (UI-FONT): Which UI font?** Curvyo shows the system UI font (Segoe UI on Windows, San
Francisco on macOS, Roboto, Noto Sans or Liberation Sans on Linux), so it looks different per
machine. A: Tracyo resolves the same list per OS (looks like Curvyo on each machine; on macOS SF
without tabular digits or semibold, so there Liberation Sans). B: Tracyo always uses the embedded
Liberation Sans (identical everywhere and in tests; differs from Curvyo on Windows and macOS).
C: always an embedded Roboto (Curvyo's look on a Linux machine with Roboto). Recommendation A.
Default: A.
