# UX notes: Trace and save

Status: Draft (ux-engineer, 2026-11-11). Reference: Curvyo `main` at `fc8edca8`, its dev server
rendered in Chromium at 1x. Rule: **what Curvyo draws, Tracyo draws**; where Curvyo's code and
`docs/design-system.md` disagree, the code (what renders) wins and the difference is named.
Own choices are marked **(Proposal)**. No hint or tip text; labels that name a control or a state
stay. All sizes are logical px at scale 1.

Revised 2026-11-13 (ux-engineer, customer decisions of 2026-11-13): **Open is the first control of
the bar**, **the bar is centred** in the window, and **there is no tool rail, now or later** (the Open
card, its 12 px inset and the 72 px left column are gone). Changed: sections 1, 2.2, 3.1 to 3.4, 4,
5, 6, 7, 11, 12.2, 13. The Mask group and the Parts row of `0002` live in Advanced (`0002` ux-notes
§2, §3).

Revised 2026-11-15 (ux-engineer, customer request of 2026-11-15): **Analyse leaves the Advanced block
and stands in the first row, in the tail group right before the expander and "Save as…"** (section
14). This **deliberately departs from Curvyo's bar** (Curvyo keeps Analyse last in its Advanced row and
stays unchanged unless the customer asks). Changed: 3.2, 3.3, 3.4, 4, 12.2 step 6, 13, new 14. Numbers
in 3.1 and 3.3 that 14.3 restates are superseded there.

Sources: `~/curvyo/frontend/src/index.css`, `components/TraceBar.tsx`, `TraceAdvanced.tsx`,
`TraceField.tsx`, `TracePresetMenu.tsx`, `TracePalette.tsx`, `TraceColourGroup.tsx`,
`ColourPickChip.tsx`, `ImportCard.tsx`, `CanvasNotice.tsx`, `StatusBar.tsx`, `panel/ValueField.tsx`,
`panel/EntryField.tsx`, `panel/Swatch.tsx`, `panel/TraceInfoPage.tsx`, `panel/FileTab.tsx`,
`lib/traceText.ts`, `lib/tracePaletteText.ts`, `lib/cursors.ts`; `docs/tracing-images.md`.

Reference pictures (`reference/`, made by `reference/capture.cjs` with `reference/tracyo-bar.cjs` from `reference/input-logo.png`):

| File set | What |
|---|---|
| `curvyo-NN-*.png` | Curvyo as it is (1280 x 800, panel open): 00 bar closed, 01 empty, 02 preview, 03 Advanced open, 04 Multicolour with palette, 05 running 42 %, 06 failed, 07 stopped, 08 Preset menu, 09 Method menu, 10 swatch hover, 11 swatch excluded, 12 value-field tooltip, 13 Trace button focus, 14 Preset hover, 15 tooltip |
| `px-NN-*.png` | The same states, **as Tracyo draws them** (re-captured 2026-11-13, last paragraph): Open first in the bar, the bar centred in a 1280 x 800 window (cap 1256), no tool rail, the Mask group in the Parts row of Advanced (`0002`), finish button "Save as…", empty bar without its sentence and with "Open image…". These are the pixel references of section 12 |

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
| Regions at W x H | **Canvas** (0, 0) to (W, H - 24), ground `#E8E8EB`. **Trace bar** floating, top 12, width w = min(content, W - 24), centred (section 3.1). **Status bar** (0, H - 24), W x 24. **About panel** (W - 280, 0), 280 x (H - 24), docked, only while open; the canvas then ends at W - 280, the bar centres over the canvas that is left, and the view keeps its origin. No tool rail and no card left of the bar |
| Stacking | canvas < picture < wash < preview outline < drop cue < notice slot < bar < menus < tooltips |

**Where Open and Save live, and why.** Tracyo has no Properties panel and no tool rail (customer,
2026-11-13: the left toolbox is gone for good). **Open** is the **first control of the bar**, an icon
button in the bar's own look with Curvyo's Import glyph, so the bar holds the whole flow from left to
right: open, adjust, save. **Save** is the blue finish button at the far right of the bar (spec
criterion 22), Curvyo's place for "finish this tool's job". The File tab's vocabulary stays: labels
end in "…" (U+2026) as "Open…" and "Save as…" do there, and tooltips name the chord.

## 2. Tokens

### 2.1 Colours (copied from `index.css`; mixes flattened to the ground they sit on)

| Token | Hex | Use |
|---|---|---|
| `--canvas-bg` | `#E8E8EB` | canvas ground (also the ground under transparent pixels) |
| `--toolbar-bg` = `--panel-bg` = `--statusbar-bg` | `#DCDCE0` | bar, menus, chip, notice, About panel, status bar |
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
| Radii | bar, menus: **10 px** (`rounded-lg`; the design doc says 8, the code renders 10); buttons, segmented outline, tooltips, notice, chip, menu rows: **8 px**; value field, outline button, Swatch: **5 px**; swatch in a cell: **4 px** |
| Shadow | `0 2px 8px rgba(0,0,0,0.24)` on bar, menus, notice; none on tooltips, chip, status bar |
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
far right of their row (`margin-left: auto`). The bar is as wide as its items, at most **W - 24**
(12 px from each window edge; W is the canvas width, so the About panel narrows it). The Mask group
and the Parts row are in the Advanced block (`0002`); no state label while a picture is shown, no
status or notice text in the bar; one hidden status region (AccessKit live region) carries what
Curvyo announces.

**Centring (customer, 2026-11-13).** The bar's left edge is x = floor((W - w₀) / 2), where w₀ is the
width of the same bar at rest (blue button "Save as…", 89 px) with Advanced closed; the bar's right
end follows its own width, and x is pulled left if that end would pass W - 12. So a trace starting
(Running is 14 px narrower), Stopped or Failed (17 or 21 px wider) and opening Advanced (8 px) change
the right end only: nothing under the pointer moves sideways **(Proposal)**. A change of the items
(another method, a picked colour, another window width) re-centres at once, without animation. When
w₀ is W - 24 or more the bar takes the whole cap, x = 12, and wraps (section 3.3). At **1280 x 800**
(Brightness, Advanced closed) w = w₀ = 1190: x = **45**, right margin **45**; with Advanced open the bar
is 1198 (right margin 37), while running 1176 (59). A wider window keeps the bar at its width and
widens both margins equally; a narrower one shrinks the margins to 12 px each from W = 1214, and below
that the bar wraps. It never leaves the window. The empty bar (section 6) is centred the same way
(165 px: x = 557 at 1280).

### 3.2 Items, left to right (with a picture)

| # | Item | Size | Shown |
|---|---|---|---|
| 0 | **Open** (`Import`) | 28 x 28 | always (with a picture; the empty bar has its own "Open image…", section 6) |
| 1 | About ("i") | 28 x 28 | always |
| 2 | Preset | 132 x 28 | always |
| 3 | Method | 176 x 28 | always |
| 4 | Threshold (Brightness) or Colour tolerance | 150 x 28 | always |
| 5 | Remove background group: chip 28, hex 56, Auto 44, pipette 28, 4 px apart (168) / Pick colours group: pipette 28 + 24 per picked colour (220 at 8) | as stated | that method |
| 6 | Smooth | 150 x 28 | always |
| 7 | Tolerance | 150 x 28 | always |
| 8 | Ignore below | 171 x 28 | always |
| 9 | **Analyse** (blue) 75, 8, expander 28, 8, **Save as…** (blue): one tail group (section 14) | 75 + 8 + 28 + 8 + 89 = 208 | always (with a picture, Advanced open or closed), far right |
| 10 | Advanced block | full width under the rows, without Analyse; its last line is the Parts row with the Mask group (`0002` §3) | while open |

### 3.3 Rows (Brightness, Advanced closed)

**Since 2026-11-15 the tail holds Analyse: the rest bar is 1273 px, two rows at 1280 px (section
14.3).** The paragraphs below give the widths before that move; 14.3 replaces their totals.

Content width: 8 + 28 (Open) + 28 + 132 + 176 + 150 + 150 + 150 + 171 + 28 + 89 + 9 x 8 + 8 =
**1190 px** (measured on Curvyo's patched bar: Open 8..36, About 44..72, Preset 80..212, Method
220..396, Save as… 1093..1182). At 1280 x 800 (bar width up to 1256): **one row, 36 px**, centred
(section 3.1). Remove background adds 176 (1366): two rows, the bar takes the cap. Pick colours: one
picked colour still fits (1250), the second wraps. At 800 x 600 (up to 776): two rows (Open to Smooth
720 px; Tolerance, Ignore below, expander and button). `px-00-bar-closed.png` shows the one-row bar.

**Blue button width (lead, 2026-11-11; the reference wins over the earlier Proposal):** at rest
"Save as…" is 89 px (Liberation Sans 14 px, 12 px side padding); while running it is 75 px, as in
`px-05-running-42.png` (Curvyo's `w-[75px]`), so the expander moves 14 px right while a trace
shows its percentage. Stopped is 8 + "Stopped" (12 px) + 4 + Retry (8 + text + 8) = 106 px, Failed
8 + 12 + 4 + "Failed" + 4 + Retry = 110 px (measured in `px-07`, `px-06`).

**Bar width, as the reference measures it.** The bar is as wide as all its items on one line (the
open Advanced block counts as one more zero-width item, so one more 8 px gap: 1198 px at rest, 1184
while running, 1215 Stopped and 1219 Failed), at most the cap. When the items do not fit on one line
the bar takes the whole cap, the items wrap by whole items and the expander with the blue button goes
to the far right of its row. All references are captured with the cap of a 1280 px window (1256), so
every state of `0001` is one row there; there is no second test window any more.

### 3.4 Tab order

Open, About, Preset, Method, main field, method group (Auto, pipette; or pipette, then the
picked swatches as one stop), Smooth, Tolerance, Ignore below, **Analyse**, expander, blue button (or
its x, or Retry), then the Advanced block (Output one stop, Colours, palette pipette, palette one stop,
Invert, Drop border shapes, Recognise text, Recognise shapes, Denoise one stop, Upscale one stop,
Corners, then the Parts row of `0002` §2), then the About panel when it scrolls (revised 2026-11-15,
section 14.4). A mouse press on any bar button hands
the keyboard back to the canvas; a key press keeps focus on the control.

## 4. Controls

Tooltips: Curvyo's house tooltip (section 6) after 400 ms, below the control, 8 px away, unless
stated. "Value-field tip" = two lines: "<Name>: <value> <unit>" (weight 500) and "Click to set,
double-click to type" (70 %), opening to the **left** of the field.

| Control | Label (drawn) | Accessible name | Default | Range, unit, steps | Tooltip |
|---|---|---|---|---|---|
| Open | glyph `Import` 16 px, 28 x 28 icon button, the bar's first control | "Open image" | | | "Open image (Ctrl+O)" (Cmd on macOS), below, as every bar tooltip |
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
| Analyse | "Analyse", 75 x 28 blue, first row before the expander (section 14) | "Analyse" | | | none |

Preset values (Smooth, Tolerance, Ignore below, Corners, Upscale, Denoise): Logo / Line art 80,
0.5, 0.5, 70, Off, Off; Photo 50, 0.5, 0.1, 30, Off, Off; Silhouette for cutting/carving 100, 1.5,
2.0, 70, Off, Low; Engraving 80, 0.35, 0.1, 70, Auto, Off. Second lines: "Sharp corners, clean
curves.", "Many shapes. Photos trace roughly.", "Closed, simple, few nodes.", "Keeps fine lines.".
Method second lines: "Dark and light pixels by a cut-off.", "Everything unlike the background.",
"Pixels like the colours you picked.".

## 5. Anatomy of each kind of control

| Kind | Rest | Hover | Pressed / on | Focus (keyboard) |
|---|---|---|---|---|
| Icon button 28 x 28 (Open, About, pipettes, toggles, expander) | no ground, glyph 16 px `#3A3A3F` | ground `#B9C6E3`, radius 8 | ground `#2F6FEE`, glyph white | ring 2 + gap 1 |
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
| No picture | one row: "Trace" (12 px `#3A3A3F`, 8 px vertical padding), 12 px, text button "Open image…" (the Open control of the empty bar: same name, same command, Ctrl+O); nothing else; centred, 165 px (`px-01-empty.png`) **(Proposal)** | `#E8E8EB` only; status bar zoom "100%" |
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
| Notice slot | centred in the canvas, 12 px above the canvas bottom; `#DCDCE0`, radius 8, padding 8, 12 px, shadow; status: `#3A3A3F`, 3 s (5 s above 70 characters); alert: text `#B3261E`, plus 1 px `#B3261E` ring, 8 s; the next notice replaces it; no controls, no focus |
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
| `Import` | 16 | 1.5 (1.0 px) | Open, first in the bar |
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
2. Tracyo: **1280 x 800** window for every case, `pixels_per_point = 1`, the same picture
   (`input-logo.png`), the same states, steps and values as the script (Method Brightness; Advanced;
   Multicolour; Ctrl+click the second swatch; Smooth +1 for running). Render the whole window by
   `egui_kittest` snapshot rendering or a window screenshot at 1x; nothing is cut off first (the
   x = 60 cut of the Open card is gone with the card).
3. Run `python3 reference/compare.py px-00-bar-closed.png tracyo-00-bar-closed.png diff.png` for
   each state. It aligns both on the bar's top and left edge, wherever the bar stands: the top edge is
   the first row holding a 200 px run of `#DCDCE0`, the left edge the first column that is `#DCDCE0`
   for 8 rows from 18 px below it. A centred bar is found the same way, because nothing left of it
   in those rows is `#DCDCE0` any more (the Open card was; the status bar is below). Then it compares with a 1 px neighbourhood
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
   px-03, 469 px with the Multicolour row), the gap, Analyse. Slices 3 and 4 added the Output pill,
   Colours, the palette and the four toggles, so the whole block row is compared: the left padding, the Output row up to
   Recognise shapes, the 8 px gap, Denoise to Corners, the gap and Analyse. Since 2026-11-13 the
   references with the block open also show the Parts row of `0002` with one part (the last 36 px
   line); it is compared by `0002`'s cases, not here. No region is shifted any more: the 232 px
   offset of the old Mask group and the 1248 px window are gone. **From 2026-11-15** (section 14)
   Analyse is part of the first row's tail and no block region holds it; every `px-*` picture with a
   picture open is re-captured (14.5).

## 13. Differences from Curvyo and from the spec draft (for the lead)

| # | Item | Here | Note |
|---|---|---|---|
| 1 | Mask group, Parts | in the Advanced block's last line (`0002` §2, §3) | spec criterion 10 (replaced by `0002`/1) |
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
| 12 | Open | first control of the bar, 28 px icon button; no rail, no card (customer, 2026-11-13) | Curvyo: the Import card of its tool rail |
| 13 | Bar position | centred, 12 px margins at least (customer, 2026-11-13) | Curvyo: left-aligned after the rail (x 72) |
| 14 | Analyse | first row, tail group: Analyse, expander, Save as…; shown whenever a picture is (customer, 2026-11-15; section 14) | **deliberate departure**: Curvyo has it last in the Advanced row, flush right; Curvyo stays unchanged unless the customer asks. Spec 0001/14 ("the blue Analyse button last") and 0002/1 ("behind Analyse") need the product owner's rewording |

**Frage 5 (UI-FONT): Which UI font?** Curvyo shows the system UI font (Segoe UI on Windows, San
Francisco on macOS, Roboto, Noto Sans or Liberation Sans on Linux), so it looks different per
machine. A: Tracyo resolves the same list per OS (looks like Curvyo on each machine; on macOS SF
without tabular digits or semibold, so there Liberation Sans). B: Tracyo always uses the embedded
Liberation Sans (identical everywhere and in tests; differs from Curvyo on Windows and macOS).
C: always an embedded Roboto (Curvyo's look on a Linux machine with Roboto). Recommendation A.
Default: A.

## Reference pictures, re-captured 2026-11-13

`px-00` .. `px-15` were re-captured with `reference/capture.cjs` (`SETS=px`) and the new
`reference/tracyo-bar.cjs`, which patches Curvyo's rendered bar in the DOM only: rail hidden, the
overlay pinned to a 1280 x 800 window at (24, 24) of a 1360 x 824 viewport (right of and below
Curvyo's rulers), the bar centred on whole pixels with a 1256 px cap, Open (a copy of About with
lucide `import`) first, the Mask group moved into the Parts row of Advanced. Curvyo: the frontend of
`fc8edca8` with the wasm built from it (`main` has moved on and its frontend needs a newer wasm than
the checkout holds), Vite dev server, Chromium, Liberation Sans, scale 1. `curvyo-*` are untouched.

Affected cases (all of them; `tests/pixels.rs` needs the implementer): every bar starts 36 px later
(Open), so the 232 px Mask shift, `with_mask_group`, the 1512 and 1480 px windows and the x = 60 cut
go: all cases run in 1280 x 800 and compare their regions in place. The block cases (03, 04, 05, 06,
07, 10, 11, 13, 14, 15) gain the Parts row as a third 36 px line (bar 121 px high). 06 and 07 were two
rows at the old 1164 px cap and are one row now; 09 (Remove background) stays two rows at the full
1256 px cap. 01 (empty bar) is the same picture up to 23 anti-aliased pixels. Checked: `compare.py`
of each against itself PASS (0 %); against the 2026-11-11 file of the same name FAIL for all 15 bar
pictures (15.7 to 30.9 %, runs of 85 to 1155 px); 01 has no 200 px bar run, as before (its test uses
a fixed size).

## 14. Analyse next to Save as… (2026-11-15)

Status: Draft (ux-engineer, 2026-11-15), Frage 9 and Frage 10 open, written with their defaults (A,
A). Customer, 2026-11-15: Analyse, at the far right of the first Advanced line, is lost there; it is
to stand at the top, next to the Trace button. Curvyo's Trace button is Tracyo's blue finish button
(`bar.rs` `tail`: "Save as…" at rest, the percent while running, Retry when Stopped or Failed; the
reference `px-13-trace-button-focus` is named after it).

**Deliberate departure from Curvyo.** Curvyo keeps Analyse last in its Advanced row, flush right
(`TraceAdvanced.tsx`); Tracyo moves it to the first row. The look stays Curvyo's: blue button (section
5), "Analyse", 75 x 28, hover `#347AFF`, ring 2 + gap 1, no tooltip (Curvyo's has none, and no hint
text is added), accessible name "Analyse", no shortcut. **Curvyo is not changed** unless the customer
asks for the same move there.

### 14.1 Place

The tail group of the first row becomes **Analyse 75, 8, expander 28, 8, Save as… 89 = 208 px** at
rest (Running 194, Stopped 225, Failed 229). It is **one whole item**: it wraps as one and stays flush
right of its row, so Analyse never leaves Save as… (as an item of its own it would stay in row 1 at
1280 px while Save as… wraps to row 2). Order (Frage 9 A): Analyse before the expander. The pair
expander + finish button stays Curvyo's, the finish button stays the bar's last control, and the two
blue buttons do not touch (36 px apart, the expander between), so they do not read as one split button.

### 14.2 States

| State | Analyse |
|---|---|
| No picture | absent; the empty bar is unchanged (165 px) |
| Picture, Advanced closed | **shown** (the point of the move: it is no longer behind the expander) |
| Advanced open | shown in the first row; the block has no Analyse any more |
| Running, Reading text, Stopped, Failed | shown and pressable, behaviour unchanged (it analyses what the active part keeps and sets Method, background, Colour tolerance and Threshold back) |
| Nothing to save (Save as… dimmed) | not dimmed |
| Expander changed-dot, "Changed: …" line | unchanged: Analyse is a command, not a value |
| Mouse press | hands the keyboard back to the canvas, as every bar button |

### 14.3 Geometry (Brightness, Single colour, one part, blue text button)

**First row.** Rest bar **1273 px** (1190 + 75 + 8), Running 1259, Stopped 1290, Failed 1294; with
Advanced open each + 8 (1281, 1267, 1298, 1302). Centring as section 3.1 on the rest width 1273:
x = 323 at 1920 (was 365), 163 at 1600. One row from a **1297 px** window (closed) and **1305 px**
(Advanced open); below that the rest bar meets the cap, x = 12, the bar is the cap wide and the tail
wraps **alone** to row 2, flush right.

| Window | Row 1 (from the bar's left edge) | Tail | Bar |
|---|---|---|---|
| 1280 x 800 (cap 1256) | Open 8..36 … Ignore below 878..1049, as before | row 2, controls y 40..68: Analyse 1040..1115, expander 1123..1151, Save as… 1159..1248; Running: Analyse 1054..1129, expander 1137..1165, button 1173..1248 (the tail moves 14 px right while running, as the expander of a wrapped bar does today) | 1256 x 72, x 12 |
| ≥ 1297 closed | as before | row 1: Analyse 1057..1132, expander 1140..1168, Save as… 1176..1265 | 1273 x 36, centred |
| ≥ 1305 open | as before | row 1: Analyse 1065..1140, expander 1148..1176, Save as… 1184..1273 | 1281 wide |

**Advanced block without Analyse.** Its line is Output, Denoise, Upscale, Corners, then the Parts item
(Parts group, divider, Mask group, `0002` §3), **25 px behind Corners** with the divider at its 13th px.
Nothing in the block has `margin-left: auto` any more, so the line packs left as Curvyo's `flex-wrap`
would; spare width stays at the right end **(Proposal)**. The block on one line is **1264 px of bar**
(today 1347, less Analyse 75 and its 8 px gap); from the bar's left edge Corners ends at 818, the label
"Parts" starts at 843, Clear mask ends at 1252. The bar with Advanced open is as wide as its first row
(1281) unless the whole block line is wider and fits the cap (the `0002` §3 rule, unchanged). Each
further part adds 32 px to the block line (two parts 1296, eight 1488).

| Window (Advanced open, one part) | First row | Block | Bar |
|---|---|---|---|
| ≥ 1305 (1920) | one row | one line, 17 px free at its right | 1281 x 85 |
| 1288 .. 1304 | two rows | one line | cap x 121 |
| ≤ 1287 (1280) | two rows | Parts item on its own line | cap x 157 |

At 1280 x 800 open: row 1 y 0..36, row 2 36..72, block divider 76, Output line 81..117, **Parts row
117..153** (controls 121..149), bar 157 high (bottom y 169). Closed: 72 high (bottom 84).

**Frage 10 B (28 px icon button) for comparison:** rest 1226 (x 27 at 1280), open 1234, one row at
1280; the block wraps at 1280 as today (bar 121 high); from 1288 px the bar widens to the 1264 px block
line (85 high). Option A costs 36 px of canvas height at 1280 x 800 (72 / 157 against 36 / 121); B costs
none but makes Analyse a 16 px glyph among nine other icon buttons.

### 14.4 Tab order and About

Open, About, Preset, Method, main field, method group, Smooth, Tolerance, Ignore below, **Analyse**,
expander, blue button (or its x, or Retry); then the block: Output … Upscale, Corners, Parts (one stop),
Add part, Rectangle, Lasso, Subtract, Mask (one stop), Clear mask. The same order when the tail has
wrapped to row 2. About panel: the entry "Denoise, Upscale, Corners, Analyse" becomes "Denoise, Upscale,
Corners" (its first three sentences), and a new entry "Analyse" with the Analyse sentence, unchanged,
follows "Ignore below", as the panel follows the bar's order **(Proposal)**.

### 14.5 Pixel cases and tests that change (option A)

- References: `reference/tracyo-bar.cjs` moves Curvyo's Analyse button from the Advanced row into the
  tail before the expander and drops the block's auto margin; re-capture `0001` px-00, 02 .. 15 and
  `0002` px-00 .. 23 at 1280 x 800 (px-01, the empty bar, stays). `curvyo-*` stay.
- `tests/pixels.rs`: `bar("bar", 1190, 36)` → (1256, 72); 1198 / 1215 / 1219 and `OPEN_H` 121 →
  1256 and 157; the running case likewise.
- `tests/parts.rs`, `tests/masks.rs`: `BAR_W` 1198 → 1256, `BAR_H` 121 → 157; the px-17 regions row
  (0, 0) 1256 x 76, output (0, 76) 850 x 32, parts (0, 108) 1256 x 49. P18: `bar_x` 45 → 12, controls
  top 12 + 85 → 12 + 121, divider y 125..145 of the bar; x unchanged (chips 44..296 … Clear mask
  613..641).
- `tests/wrap.rs`: `REST` 1190 → 1273, `bar_left(1920)` 365 → 323. P19 (1920): Parts label at
  Corners right + 25, dividers at Corners right + 12 and trash right + 12, bar 85 high, bar right =
  left + 1281 = Clear mask right + 29 (ground at right − 1, not at right + 2); the `px-08` crop stays.
  P20 runs at **1280** (at 1300 the block now fits one line): Parts item at left + 8, one line below the
  Output line, bar 157 high, no divider at the line start. Threshold: **1288** one line, **1287** wraps
  (was 1371 / 1370); the measured block line (Clear mask right + 12 − bar left) is 1264. Eight parts at
  1920: Add part left = Corners right + 25 + 292; a second part at 1288 moves the Parts item down.
  Analyse asserts change to: Analyse left = bar right − 8 − 89 − 8 − 28 − 8 − 75.
- `tests/regions.rs` Tab test: "Ignore below, Analyse, More trace options" in row 1; from Corners Tab
  reaches the Parts chip directly. `bar.rs` unit tests: rest 1190 → 1273 and the tail of 208.

### 14.6 Tasks for the implementer

1. `bar.rs`: tail = Analyse 75 + 8 + expander + 8 + blue; `rest` and `layout` use the 208 px tail (194 running); unit tests to 14.3.
2. `bar.rs` `tail`: draw `controls::accent_button` "Analyse" first, `release_focus_after_mouse`, click → `analyse_again`; Tab order 14.4.
3. `advanced.rs`: remove Analyse from the block; `block_line_width` without `ANALYSE_W` and its gap; `block_layout` without a tail.
4. `bar.rs` `rows`/`block_flow`: with no tail the Parts item stands `LEAD_GAP` behind Corners (pack left), divider rule unchanged.
5. `about.rs`: split the entry as 14.4; update its test text.
6. `tests/wrap.rs`, `parts.rs` (P18), `masks.rs`, `pixels.rs`, `regions.rs` (Tab): numbers of 14.5.
7. References: patch `tracyo-bar.cjs`, re-capture px sets (needs Curvyo's dev server at `fc8edca8`; ux-engineer or lead), then the pixel cases.
8. Spec 0001/14 and 0002/1 wording (product owner): Analyse in the first row before the expander.
9. No tooltip, no hint text, no shortcut for Analyse.

**Frage 9 (ANALYSE-ORDER): Where exactly next to Save as…?** A: Analyse, expander, Save as… (Curvyo's
pair expander + finish stays, the blue buttons do not touch). B: expander, Analyse, Save as… (Analyse
right beside Save as…, two blue buttons 8 px apart). C: Analyse right of Save as… (the finish is no
longer the last control). Recommendation A. Default A.

**Frage 10 (ANALYSE-LOOK): Text button or icon?** A: the blue "Analyse" text button as today (most
visible; at the 1280 x 800 start size the bar gets a second row holding only Analyse, expander and Save
as…, one row from a 1297 px window). B: a 28 px icon button (lucide `ScanSearch`, tooltip "Analyse"),
the bar stays one row at 1280. C: A, and the start window grows to 1320 x 800 so the first start shows
one row. Recommendation A (the customer's complaint is findability). Default A.
