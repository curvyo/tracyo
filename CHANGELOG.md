# Changelog

All notable changes, newest first. The format follows [Keep a Changelog](https://keepachangelog.com/).

## [Unreleased]

First release, `0.1.0`. Polygons and stars are saved as `<polygon>` and Inkscape stars, and vectrace 0.1.1 recognises a thin ellipse ring as one stroked ellipse.

### Added

- Open (button, Ctrl+O, drop, command line), pan and zoom, a live outline preview while values
  are dragged, Stop and Retry, Save as an Inkscape SVG in millimetres.
- The Trace bar of Curvyo: Preset, Method (Brightness, Remove background, Pick colours), Threshold,
  Colour tolerance, Smooth, Tolerance, Ignore below, the Advanced block (Output, Multicolour
  palette, Invert, Drop border shapes, Denoise, Upscale, Corners, Analyse); one SVG layer per colour.
- Recognise text (PaddleOCR Latin, built in, no download) and Recognise shapes.
- An About panel with the licences and the source address; `THIRD-PARTY.md` with the licence of
  every library.
- Release builds for Linux x86_64, Windows x86_64 and macOS arm64 and x86_64, unsigned.

### Fixed

- The picture is fitted below the bar as it is laid out in the same frame, and again when the
  bar grows over it before it was moved.
- The Output row and its palette wrap in a narrow window instead of running past the block.
- A recognised text takes its size and baseline from the glyphs it has, not from the height of
  its ink, so lines with descenders or umlauts are no longer too large and too low.
- Without a file dialog (Linux without `xdg-desktop-portal`) Open and Save as say so.
