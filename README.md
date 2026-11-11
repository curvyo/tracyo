# Tracyo

A small desktop app that turns a raster image into vector paths: open or drop a PNG, JPEG or
WebP, trace it with [vectrace](https://github.com/curvyo/vectrace), save the result as an
Inkscape SVG. It has the Trace tool of [Curvyo](https://github.com/curvyo/curvyo) and nothing
else, and looks like it. Written in Rust with egui.

Licence: AGPL-3.0-or-later (see `LICENSE`).

## Use

```text
tracyo                 # empty window, then Ctrl+O or drop a picture on it
tracyo picture.png     # open a picture at start
```

- **Open:** the card at the top left, Ctrl+O, a dropped file, or the first command-line argument.
- **Look:** the picture is veiled and the outline the tracer makes is drawn over it in blue.
  Smooth, Tolerance, Ignore below and Threshold change it as you drag them; Preset sets Smooth,
  Tolerance, Ignore below, Corners, Upscale and Denoise together.
- **Pan and zoom:** scroll or middle-drag or Space+drag pans, Ctrl+scroll zooms at the pointer,
  Ctrl+0 fits.
- **Stop:** Esc or the x in the blue button; Retry traces again.
- **Save:** the blue button, Enter, Ctrl+S or Ctrl+Shift+S writes an Inkscape SVG (millimetres,
  one layer `#000000`). Nothing else leaves the app: it makes no network connection.

Your values are kept in `settings.toml` in the per-user config folder (`~/.config/tracyo/` on
Linux).

## Build

```text
cargo build --release     # target/release/tracyo
cargo nextest run --workspace
```

The trace runs in a second process, the same executable started as `tracyo --trace-worker`, so
Stop ends it at once. Design decisions are in `docs/adr/`, the feature in `specs/`.

On Linux the window needs a running `xdg-desktop-portal` for the file dialogs, and uses X11 (or
XWayland) so that dropping files works.
