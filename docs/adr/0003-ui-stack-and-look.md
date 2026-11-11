# ADR 0003: UI stack, platform services and how Curvyo's look is reproduced

**Status:** Accepted (lead, slice 5, 2026-11-11; proposed by the architect 2026-11-11). egui itself is the customer's decision (ADR 0001).
Fonts follow ux-notes Frage 5 at its default A; the settings file follows spec criterion 18.

Deviations: the picture is drawn without mipmaps (technical debt 1); the pipette cursor is the system crosshair (2); the About panel's text is a Tab stop that scrolls with the keys; the `clipboard` feature of section 1 is now on (it was missing until slice 5; `BSL-1.0` joined the allowed licences of `deny.toml` for the Windows clipboard crates); a file dialog that closes within 150 ms without a file is reported as "no file dialog" instead of a cancel (debt 3); the hidden AccessKit status region is not built (5).

## Context

egui is immediate mode and paints its own widgets; Curvyo's Trace bar is HTML/CSS rendered by
Chromium (Skia). The ux-notes give every token, size and state, and require a pixel comparison
(ux-notes §12). Facts measured in a spike on this machine (Arch, Mesa 26.2, no Vulkan driver,
Xvfb available), release, LTO, stripped, Linux x86_64:

| Fact | glow | wgpu |
|---|---|---|
| minimal eframe 0.37.0 window, binary | 9.8 MB | 14.1 MB |
| starts under `xvfb-run` (llvmpipe) | yes | yes (GL backend through EGL) |
| `egui_kittest` 0.37 renders a PNG headless, no X server | no (kittest renders with wgpu only) | yes, 0.7 s |
| macOS | OpenGL (deprecated by Apple) | Metal |
| Windows | OpenGL driver needed | DX12 (WARP software fallback on VMs) |

Full stack with wgpu, vectrace, image, resvg, fontdb and rfd: 19.1 MB, plus 9.9 MB of OCR model
(ADR 0006).

## Decision

1. **eframe/egui `=0.37.0`** (exact pin: egui changes look between minor versions; Renovate bumps
   the egui group together and the pixel tests decide). Features, everything else off:
   `wgpu`, `x11`, `wayland`, `accesskit`, `clipboard`. Off on purpose: `links` (opens a browser,
   spec criterion 23), `persistence` (own settings file), `default_fonts` (own fonts),
   `system_fonts`.
2. **Renderer wgpu** (rejected glow): the same renderer as the `egui_kittest` pictures the pixel
   tests compare, Metal on macOS, DX12 with a software fallback on Windows. Cost: 4.3 MB more.
   The picture is uploaded in tiles of at most 2048 px (wgpu's GL fallback guarantees no more),
   linear filtering with mipmaps, nearest from 800 % (ux-notes §6).
3. **Linux backend:** X11 (through XWayland on a Wayland desktop) when `DISPLAY` is set, else
   Wayland. Reason: winit 0.30.13 (used by eframe 0.37) delivers dropped and hovered files on
   Windows, macOS and X11 only; spec criterion 1 requires drop. Cost: blurrier text at fractional
   scaling on Wayland. Revisit when winit delivers drops on Wayland. **(Proposal)**
4. **Dialogs: `rfd` 0.18**, `default-features = false, features = ["xdg-portal"]` on Linux
   (no GTK build dependency; needs a running xdg-desktop-portal, present on GNOME, KDE and most
   desktops). Called synchronously from the frame; the trace keeps running in its process.
   Filters and titles: ux-notes §9.
5. **Pictures: `image` 0.25** with `png`, `jpeg`, `webp` only, in `tracyo-core::picture`:
   format from the bytes, the decoder's EXIF orientation applied, grey and RGB widened to RGBA
   with alpha 255, 16-bit rounded to 8 bit with `(v + 128) / 257` (as vectrace's test helper
   `tests/common/decode.rs` and Curvyo's `decode_working`). Decoder `Limits` stop huge headers;
   a limit or allocation error is "Not enough memory to open this image.". Decoding runs on a
   thread of the app, not on the UI thread.
6. **Fonts (ux-notes Frage 5, default A):** at start the app resolves Curvyo's list per OS with
   `fontdb` 0.24 (pure Rust, scans the system font folders): Windows Segoe UI; macOS Liberation
   Sans (San Francisco has proportional digits and egui has no OpenType features); Linux Roboto,
   Noto Sans, Arial/Liberation Sans, first installed. Liberation Sans Regular and Bold are
   embedded as the last fallback (ADR 0006). Weight 600 uses the face's bold file, 500 its regular.
   The fonts are an input of `theme::install`; tests pass the embedded Liberation Sans so the
   pixel tests do not depend on the machine. The menu check mark "✓" is drawn as a path
   (Liberation Sans has no U+2713).
7. **Icons:** the lucide 1.50.0 SVGs of ux-notes §11 are files in `assets/icons/lucide/` and are
   rasterised with `resvg` 0.50 (`default-features = false`; tiny-skia, the Skia port, so strokes
   look like Chromium's) at the device pixel size, `currentColor` replaced by the wanted colour,
   cached as textures per icon, size, colour and pixels per point. The chevron, check mark and
   swatch `X`/`Ban` overlays are painted with egui shapes.
8. **Settings file:** `serde` + `toml` 1.x in `tracyo-core::settings` (text in, text out);
   the app reads and writes `dirs::config_dir()/tracyo/settings.toml` (`dirs` 7), writing to a
   temporary file in the same folder and renaming it (`tempfile`). A missing or unreadable file
   gives the defaults; it is rewritten on the next change. Window size is saved on exit.
9. **The look:** a `theme` module holds every token of ux-notes §2 as named constants
   (colours as `Color32`, radii, paddings, type sizes with explicit line heights 20/16) and sets
   egui's `Style`/`Visuals` (selection `#3367D1`, tooltip delay 0.4 s, thin scroll bars). Light
   theme only (ux-notes §1).

   | Painted by Tracyo (widgets module) | egui as is |
   |---|---|
   | Trace bar surface and its flow layout (fixed item widths of ux-notes §3.2, wrap by whole items, last item right-aligned), icon buttons, Preset/Method buttons and menus, value field (bar, edge, drag words), blue button in all states (fill to the percent, x, Stopped/Failed + Retry, dimmed), segmented control, swatch cells, background chip, open card, notice slot, status bar, focus rings (keyboard only), house tooltip anchored below the control | text entry inside a value field (restyled `TextEdit`), `Area` and `Frame` for floating surfaces, scroll area of the About panel and Advanced block, AccessKit names and the live region |

   Group opacity is not available: the flattened colours of ux-notes §2.1 are used. Shadows use
   `epaint::Shadow { offset [0, 2], blur 8, color black α 61 }`.
10. **Pipette cursor:** `CursorIcon::Crosshair` (Curvyo's own fallback); eframe 0.37 exposes no
    custom cursor images. Debt item.

## Consequences

- Text is rasterised by egui, not Skia: glyph edges differ (accepted by the comparison's
  tolerance); fixed item widths keep a different text advance from moving any box.
- The Linux file dialog fails without a portal (a bare window manager): rfd returns no file.
  Tracyo cannot tell this from a cancel. Debt item.
- Wayland users get X11 windows; dropping files works everywhere.
- Every look decision is in `theme`; a ux change touches one module.
