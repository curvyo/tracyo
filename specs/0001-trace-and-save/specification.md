# Trace and save

Status: Draft
Priority: Must
Origin: Customer (product Tracyo: the Trace tool of Curvyo as its own desktop app; egui in its own repository, `vectrace` for tracing, OCR and shape recognition, AGPL-3.0-or-later). Everything marked **(Proposal)** is the product owner's and vetoable.

## User value

As a maker with a raster picture (logo, drawing, scan, photo of a sheet) I want to open it, see on the picture what the tracer would make, adjust it with the controls I know from Curvyo, and save the result as an Inkscape SVG, so that I get clean paths for Inkscape, Ink/Stitch or a cutter without installing or learning a whole vector editor.

Behaviour reference: Curvyo `docs/tracing-images.md`, specs 0025, 0089, 0091, `frontend/src/components/Trace*.tsx`. Where this spec says "as in Curvyo", that document decides; labels and messages are copied verbatim. Targets: Linux, Windows, macOS (one binary each, no installer assumptions).

## Scope

One window: the Trace bar on top, the canvas below (picture, preview, pan, zoom). One picture at a time. Open a picture, trace, save. Nothing else.

## Acceptance criteria

### Opening a picture

1. Given the app, when I use **Open image...** (also Ctrl+O, Cmd+O on macOS) and pick a PNG, JPEG or WebP, then it is shown on the canvas, fitted to the window, and its preview starts. Given the same file as the first command-line argument (`tracyo picture.png`), or dropped on the window, then the same happens. A second picture replaces the first (the settings stay, picked colours, Invert and Drop border shapes are reset as in Curvyo). **(Proposal)** The window title is the file name.
2. Given a file that is not a readable PNG, JPEG or WebP (wrong type, truncated, 0 pixels), then the current picture and settings stay and the bar shows the message "Cannot read this image." **(Proposal)** The app does not exit and writes nothing.
3. Given no picture, then the canvas is empty and the bar shows **Open image...** (Curvyo shows "Import image..." in the same place); the Trace controls are shown but dimmed and the save button is dimmed. **(Proposal)** The empty state shows no sentence of instruction.
4. Given a picture with transparency, then transparent pixels are never shapes, with Invert neither, and a partly transparent pixel counts by its opacity once (as in Curvyo "Method").

### Canvas and preview

5. Pan with the middle mouse button, Space+drag, or two-finger scroll; zoom with Ctrl/Cmd+scroll or pinch, toward the pointer; Ctrl/Cmd+0 fits the picture; the zoom range is 5 % to 6400 % **(Proposal)**. Pan and zoom redraw the preview without tracing again (observable: the trace job counter does not change).
6. The preview is exactly as in Curvyo "The preview": the picture washed out by a white veil and the outline drawn over it as a thin blue line on a white casing, holes included, in screen pixels (width does not grow with zoom). With Recognise shapes on, recognised shapes are drawn where the paths were, in the same line style. Recognised text lines are drawn as their outlines in the same style.
7. A change of any value starts a trace in the background and the window stays responsive (the UI thread never runs vectrace; a frame takes under 50 ms on a 12 Mpx picture while tracing, measured as CPU time of the UI thread, not wall time). While a value is dragged a reduced copy (longest side 1000 px) is traced, on release the full picture. For a picture over 4 Mpx the preview is always the reduced copy and saving traces the full picture.
8. Progress, Stop, Stopped, Failed and Retry are as in Curvyo "The preview": after 400 ms of running the blue button shows the percentage (for example "42 %") and an x with the name "Stop (Esc)"; the percentage stays at least 300 ms once shown; the old outline is drawn at half strength; the x or the first Esc stops the trace and the button says "Stopped" (tooltip "Preview is out of date") with **Retry**; Enter, a double-click on the picture or a changed value also trace again. A failed trace says "Failed" with **Retry**; its tooltip gives the reason, for example "Trace failed: out of memory at 16 Mpx. Nothing was saved." **(Proposal wording of the last sentence)**; the values stay.
9. Esc order: first stops a running trace, then disarms an armed pipette; it never closes the app.

### The bar

10. The bar has, in this order, left to right: the About button ("i"), **Preset**, **Method**, the method's field(s), the palette group (criterion 14), **Smooth**, **Tolerance**, **Ignore below**, and at the far right the expander ("More trace options" / "Fewer trace options") and the blue save button (criterion 22). There is no Mask group (Pick mask paths, Inside, Outside, Clear mask), no Parts row and no Layers tab; a screen shows nothing of them.
11. **Preset** offers Logo / Line art, Photo, Silhouette for cutting/carving and Engraving with the descriptions and the values of the table in Curvyo "Presets" (Smooth, Tolerance, Ignore below, Corners, Upscale, Denoise); picking one never changes Method, Threshold or the toggles; after any change of those six values the button reads "Custom". The first start uses Logo / Line art. Menu keys as in Curvyo (Enter or Down opens, Up/Down, Enter picks, Esc closes).
12. **Method** offers Brightness ("Dark and light pixels by a cut-off."), Remove background ("Everything unlike the background.") and Pick colours ("Pixels like the colours you picked."), behaving as in Curvyo "Method": Threshold (0 to 100 %, Brightness only, starts at the picture's automatic value, brightness `0.2126 R + 0.7152 G + 0.0722 B`); Colour tolerance (0 to 100; 40 for Remove background, 19 for Pick colours, remembered per method) with the background chip, hex or "none", **Auto** and the pipette; Pick colours with the pipette armed on choosing the method, click replaces, Shift+click adds (up to 8), swatches with the colour code on hover and an x, and the save button dimmed with the tooltip "0 paths: no colour picked." while none is picked.
13. **Smooth** (0 to 100, 80), **Tolerance** (0.1 to 3 px, 0.5 px) and **Ignore below** (0 to 10 mm, 0.5 mm) are value fields as in Curvyo "Settings": click sets, drag changes (reduced copy while dragging), double-click, Enter or F2 opens number entry; Up/Down one step (Threshold 1 %, Tolerance 0.05 px, Ignore below 0.1 mm), Shift a big step (10 %, 0.5 px, 1 mm), Ctrl/Cmd the finest (0.01 px, 0.01 mm), Home/End the limits; a typed value is clamped to the range.
14. The expander opens the Advanced block (open or closed is remembered) with, top to bottom: **Output** (Single colour, Multicolour) and with Multicolour **Colours** (2 to 16, "Auto" at the lowest stop, shown muted with the found count, for example "Auto  6") and the palette with its pipette; the four toggles **Invert**, **Drop border shapes**, **Recognise text**, **Recognise shapes**; **Denoise** (Off, Low, High), **Upscale** (Auto, Off), **Corners** (0 to 100, 70); the blue **Analyse** button last. Semantics as in Curvyo "Advanced", "Multicolour" and "Analysis", including: the palette swatches largest area first; the x deletes a colour and the rest are matched again; Ctrl+click (Cmd on macOS) excludes a colour (faded, crossed circle) and a second one brings it back; "0 paths: no colour in the palette." when none is left; the automatic analysis runs once per picture, sets only Method, background, Colour tolerance and Threshold, never overrides a value the maker changed, and **Analyse** sets them back.
15. Disabled-save reasons are the tooltips of Curvyo "Status" verbatim: "0 paths at this Threshold: too low." (or "too high."), "0 paths at this Colour tolerance: too high." (Remove background) / "too low." (Pick colours), "No colours found.", "0 paths: no colour picked.", "0 paths: no colour in the palette.", "Too much detail: raise Ignore below.". The bar prints no counts and no status line. **(Proposal)** The window shows no notice slot.
16. **Recognise text** reads the text first as in Curvyo "Recognise text" (OCR by vectrace's model; confidence and 'looks like no text' rules are vectrace's); while it reads the button shows "Reading text" and can be stopped; later previews of the same picture reuse the lines. With it off the model is not loaded. The model files are inside the executable or its install folder; the first use needs no download.
17. **Recognise shapes** is as in Curvyo "Recognise shapes" (rectangles with or without rounded corners, circles, ellipses, regular polygons, stars; bands; holes; text decided first); all rules and tolerances are vectrace's.
18. Remembered between starts, in one plain-text file in the platform's per-user config folder **(Proposal: TOML, `tracyo/settings.toml`)**: Preset or Custom with its six values, Method, each method's Colour tolerance, Smooth, Tolerance, Ignore below, Output, Colours, Recognise text, Recognise shapes, whether Advanced is open, window size. Not remembered: Threshold, Invert, Drop border shapes, picked colours, Analysis results, the picture, its zoom. A missing or corrupt file starts with defaults and is replaced silently on the next change.

### Saving

19. **Save as...** (the blue button, Enter on the canvas, Ctrl+Shift+S / Cmd+Shift+S **(Proposal)**) opens the system save dialog with `<picture name>.svg` in the picture's folder. If only the reduced copy has been traced the full picture is traced first with the progress in the button; Esc stops before anything is written. The file is written completely or not at all (write to a temporary name next to it, then rename). The app stays open and the picture and settings are unchanged. No confirmation text appears **(Proposal)**; a write error shows "Failed" with Retry and the reason in the tooltip.
20. **Structure of the SVG** (an XML test checks all of this without Inkscape):
    a. Well-formed XML 1.0 in UTF-8, one root `<svg>` with `xmlns="http://www.w3.org/2000/svg"`, `xmlns:inkscape="http://www.inkscape.org/namespaces/inkscape"` and `xmlns:sodipodi="http://sodipodi.sourceforge.net/DTD/sodipodi-0.dtd"`, and `version="1.1"`.
    b. `width="<W>mm"`, `height="<H>mm"` and `viewBox="0 0 <W> <H>"` with the same numbers, so one user unit is 1 mm. W and H are the picture's size in mm; a picture is placed at 96 dpi (one pixel = 25.4/96 mm) **(Proposal, Frage 2)**. Numbers have at most 3 decimals.
    c. A `<sodipodi:namedview>` with `inkscape:document-units="mm"` **(Proposal)**.
    d. Children of the root other than `<defs>` and the namedview are only layers: `<g inkscape:groupmode="layer" inkscape:label="<LABEL>" id="layer<N>">`. Single colour: one layer labelled `#000000`. Multicolour: one layer per colour in the result, labelled with its upper-case hex colour (`#4B392F`), largest area at the bottom (first in the file); an excluded or deleted colour has no layer; a layer is never empty.
    e. Each layer holds one `<path fill="<hex>" fill-rule="evenodd" stroke="none" d="...">` for the colour's outlines, holes as sub-paths (the path is closed with `Z`, only `M L C Z` commands, absolute coordinates), plus the recognised primitives and texts of that colour (f, g). The path's `id` is unique (`path<N>`).
    f. Recognised shapes are written as primitives **(Proposal)**: rectangle as `<rect x y width height rx ry>`, circle as `<circle cx cy r>`, ellipse as `<ellipse cx cy rx ry>`, a rotated one with `transform="rotate(<deg> <cx> <cy>)"`; a regular polygon and a star as `<path>` with straight segments (not `<polygon>`, no `sodipodi:type="star"`), so every reader shows the same. A shape is filled like the path of its colour; a band is the same primitive with `fill="none"`, `stroke="<hex>"` and `stroke-width` the band's width in mm. The shape is not also in the colour's `<path>`.
    g. Recognised text is written as `<text xml:space="preserve" transform="..." font-family="sans-serif" font-size="<mm>" fill="<hex>" textLength="<mm>" lengthAdjust="spacingAndGlyphs"><tspan>STRING</tspan></text>` **(Proposal)**: baseline start and rotation from the line's direction, font size from the height of its outlines, `textLength` the line's width, so the text fills the place of the original line in any font. Special characters are escaped. The text's pixels are not also in the paths (vectrace leaves them out). Font fitting is not done (Frage 1).
    h. The file contains no `curvyo` or `tracyo` metadata, no `<metadata>`, no script, no external reference (`href`, `url(`), no raster image.
21. **Other readers (manual check, one per release):** open the saved SVG of the Logo reference picture, once in Single colour and once in Multicolour with 4 colours, in Inkscape 1.2 or later: the size reads W x H mm, the Layers and Objects dialog lists the layers with their colour labels in the order of 20d, holes are holes (Edit > Select All in All Layers, Object > Fill and Stroke shows the colour), and a recognised rectangle and circle are editable with the rectangle and ellipse tools. In a browser and in a second reader (for example `rsvg-convert`) the rendering matches the preview outline to within 0.5 px at 96 dpi. **Automatic substitute:** render with `resvg` or `rsvg-convert` in the tests and compare to the area of the traced outlines (coverage differs by at most 0.5 %).
22. The blue button at rest says **Save as...** and does what Enter does; the states of criteria 8 and 15 use the same button.

### Safety and portability

23. **No network.** The app opens no network connection of any kind: no update check, no telemetry, no model download, no font download, no crash report, no link opening (the source address in About is text). Checked by (a) `cargo tree` and `Cargo.lock` containing no HTTP, TLS or socket client crate (`cargo deny` ban list), and (b) running a full session (open, trace with Recognise text and Recognise shapes, save) in a network-less namespace (`unshare -rn`) with the same result as with network.
24. A picture of 12 Mpx traces and saves on a mid-range desktop without the app crashing; running out of memory shows "Failed" (criterion 8), never a panic; closing the window during a trace ends the app within 2 s.
25. Builds and passes the gate on Linux, Windows and macOS (CI nightly for the last two); the only unsafe code is that of dependencies.
26. **About.** The "i" button opens a small About window **(Proposal)**: "Tracyo", version, "AGPL-3.0-or-later", the licences of vectrace (MIT OR Apache-2.0) and the OCR model (PaddleOCR Latin, Apache-2.0), the "About Trace" description of Curvyo without its Mask and Parts lines, and the source address as text. It never opens by itself; the button is pressed while it shows; Esc closes it.

## Out of scope

- Drawing, objects, object model, selection, undo/redo and history, pages, layers panel, masks (Mask group), parts, multi-image tracing, batch.
- Any other export (DXF, PDF, G-code, plain SVG variants), copy to clipboard, print.
- Laser, plotter, embroidery or CNC features; units other than mm in the file.
- Accounts, cloud, telemetry, update checks, plugins, themes beyond Curvyo's look (light and dark follow the system as Curvyo does).
- Font matching for recognised text; editing recognised text in the app.
- Screen-reader announcements of Curvyo ("7 paths, 412 nodes", "3 text lines set"): Could, tracked as debt; tooltips and control names (criteria 12 to 15) are required.
- Reading DPI from picture metadata (Frage 2).
- Hint or tip text anywhere in the UI; only labels naming a control or a state.

## Open questions

- **Frage 1 (TEXT-FONT): Recognised text without font fitting.** vectrace gives the read string and the line's outlines, not a font. A: `<text>` in `sans-serif` stretched to the line's width (criterion 20g). B: write the line's outlines as paths and no `<text>` (looks exact, not editable text). C: both, the outlines in a hidden layer. Recommendation A. Default: A.
- **Frage 2 (DPI): Size of the picture in mm.** A: always 96 dpi. B: the dpi stored in the PNG/JPEG when present, else 96. Recommendation A for the first slice (B changes the size silently). Default: A.
- **Frage 3 (STAR-POLY): Polygons and stars as `<path>`** (identical everywhere) or as Inkscape `sodipodi:type="star"` (editable as star in Inkscape, needs extra attributes that other readers ignore). Recommendation `<path>`. Default: `<path>`.
- **Frage 4 (OCR-MODEL): The ~10 MB OCR model** (Apache-2.0) is embedded in the executable. Default: embedded; no download, so criterion 23 holds.
