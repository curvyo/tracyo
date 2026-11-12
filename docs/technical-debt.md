# Technical debt

Known shortcuts, with where they come from. Remove an entry when it is fixed.

| # | Item | Why it is open | Source |
|---|---|---|---|
| 1 | The picture is drawn with linear filtering and no mipmaps | `egui-wgpu` 0.37 ignores `TextureOptions::mipmap_mode` (only `egui_glow` has it), so a picture shown far below 100 % is not as smooth as in Curvyo | ADR 0003 section 2, slice 1 |
| 2 | Pipette cursor is the system crosshair | eframe 0.37 has no custom cursor images | ADR 0003 section 10 |
| 3 | Linux file dialogs need a running `xdg-desktop-portal`; a missing portal looks like a cancel | `rfd` cannot tell them apart | ADR 0003 |
| 4 | `RUSTSEC-2026-0370..0372` (`zvariant`, `zbus_names`, `zvariant_derive` unmaintained) are ignored in `deny.toml` | they come in through egui's accessibility stack and have no upgrade | `deny.toml` |
| 5 | The hidden status region (AccessKit live region) of ux-notes section 3.1 is not built | not needed by a slice-1 criterion | ux-notes 3.1 |
| 6 | The analysis runs on an app thread, not in the worker process of ADR 0004 section 2, and cannot be stopped | the worker is killed for every dragged value, which would kill a running analysis; an out-of-memory abort in it would take the app down; a second worker process only for Analyse needs a change of ADR 0004 section 2 (architect), slice 3 left it | slice 2, 3, ADR 0004 |
| 8 | px-15 (About tooltip) is not compared | the reference reads "About Trace", Tracyo's text is "About Tracyo"; slice 5 decides the pixel test | plan slice 5 |
| 9 | The picked-swatch row, its hover x and the pipette chip have no pixel reference | no reference picture shows them (px-10 and px-11 show the palette swatches, not these) | ux-notes 5 |
| 10 | The palette row does not wrap inside the Output row | Curvyo's row wraps its swatches onto more lines; Tracyo draws one line, so many swatches in a narrow window may run past the block's edge | slice 3, ux-notes 5 |
| 11 | The picture is fitted below the bar of the first frame, so a bar of three rows (Advanced open, Remove background, many colours) covers its top until Ctrl+0 | `fit_next_frame` is consumed before the bar is laid out for the new picture | slice 3 real-app run |
| 12 | The font size of a recognised text is the ink's extent across the line and its baseline the ink's far edge, so a line with descenders or an umlaut is a little too large and sits a little too low | no font fitting (spec Frage 1 A, ADR 0005 section 7); a line of capitals only is the right size | slice 4 |
| 13 | `rsvg-convert` and other readers that ignore `textLength` draw a recognised text at the font's natural width, not stretched to the original | spec criterion 21 compares with `rsvg-convert`; the automatic substitute covers paths and shapes, text is checked by structure and in Inkscape | slice 4 |
| 14 | The read lines are kept for the picture while it is shown, not in the settings or beside the file | a new open of the same file reads again (about 1 to 3 s at 1.5 Mpx); a cache needs a decision on where it lives | slice 4 |
| 15 | The OCR model is loaded again by each new worker process that gets a `ReadText` (after a Stop or a changed picture) | the worker is killed to cancel (ADR 0004); loading takes well under a second | slice 4 |
