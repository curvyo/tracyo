# Technical debt

Known shortcuts, with where they come from. Remove an entry when it is fixed.

| # | Item | Why it is open | Source |
|---|---|---|---|
| 1 | The picture is drawn with linear filtering and no mipmaps | `egui-wgpu` 0.37 ignores `TextureOptions::mipmap_mode` (only `egui_glow` has it), so a picture shown far below 100 % is not as smooth as in Curvyo | ADR 0003 section 2, slice 1 |
| 2 | Pipette cursor is the system crosshair | eframe 0.37 has no custom cursor images | ADR 0003 section 10 |
| 3 | Linux file dialogs need a running `xdg-desktop-portal`; a missing portal looks like a cancel | `rfd` cannot tell them apart | ADR 0003 |
| 4 | `RUSTSEC-2026-0370..0372` (`zvariant`, `zbus_names`, `zvariant_derive` unmaintained) are ignored in `deny.toml` | they come in through egui's accessibility stack and have no upgrade | `deny.toml` |
| 5 | The hidden status region (AccessKit live region) of ux-notes section 3.1 is not built | not needed by a slice-1 criterion | ux-notes 3.1 |
| 6 | The Method menu has the one row Brightness | the other methods arrive with slice 2 | plan slice 2 |
| 7 | The expander's slot (28 px) is reserved and empty so the bar has its final width | the Advanced block arrives with slice 2 | plan slice 2 |
| 8 | Pixel test px-05 compares the bar left of the blue button only | the reference shows the running button 75 px wide, ux-notes section 3.3 keeps it 89 px | ux-notes 3.3 |
