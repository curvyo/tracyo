# Technical debt

Known shortcuts, with where they come from. Remove an entry when it is fixed.

| # | Item | Why it is open | Source |
|---|---|---|---|
| 1 | The picture is drawn with linear filtering and no mipmaps | `egui-wgpu` 0.37 ignores `TextureOptions::mipmap_mode` (only `egui_glow` has it), so a picture shown far below 100 % is not as smooth as in Curvyo | ADR 0003 section 2, slice 1 |
| 2 | Pipette cursor is the system crosshair | eframe 0.37 has no custom cursor images | ADR 0003 section 10 |
| 3 | Linux file dialogs need a running `xdg-desktop-portal`; a missing portal looks like a cancel | `rfd` cannot tell them apart | ADR 0003 |
| 4 | `RUSTSEC-2026-0370..0372` (`zvariant`, `zbus_names`, `zvariant_derive` unmaintained) are ignored in `deny.toml` | they come in through egui's accessibility stack and have no upgrade | `deny.toml` |
| 5 | The hidden status region (AccessKit live region) of ux-notes section 3.1 is not built | not needed by a slice-1 criterion | ux-notes 3.1 |
| 6 | The analysis runs on an app thread, not in the worker process of ADR 0004 section 2, and cannot be stopped | the worker is killed for every dragged value, which would kill a running analysis; an out-of-memory abort in it would take the app down | slice 2, ADR 0004 |
| 7 | The pixel tests of references with the Advanced block open compare region by region | the references show the Output pill, the Multicolour palette and the Recognise toggles, which later slices build; Denoise to Corners are compared shifted | ux-notes 12.2 step 6 |
| 8 | px-15 (About tooltip) is not compared | the reference reads "About Trace", Tracyo's text is "About Tracyo"; slice 5 decides the pixel test | plan slice 5 |
| 9 | The picked-swatch row, its hover x and the pipette chip have no pixel reference | no reference picture shows them (px-10 and px-11 show the palette, slice 3) | ux-notes 5 |
