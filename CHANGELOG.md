# Changelog

All notable changes, newest first. The format follows [Keep a Changelog](https://keepachangelog.com/).

## [Unreleased]

First release, `0.1.0`. Polygons and stars are saved as `<polygon>` and Inkscape stars, and vectrace 0.1.1 recognises a thin ellipse ring as one stroked ellipse.

### Added

- Colours, text and shapes per part (specification 0002, slice 3): Multicolour parts join into one
  layer per colour (equal colours merge across a seam); every part reads the automatic Threshold and
  Auto background of the area it keeps once later parts are cut out (kept per area, so switching
  chips or removing a part is instant); Recognise text can be on in one part and off in another;
  Recognise shapes looks at the cut result only.
- Lasso regions (specification 0002, criteria 28 and 29): click corners of a straight-sided
  polygon, close it on the first corner, with a double click, Enter or a right click (Esc aborts it), select it, move it,
  drag its corners; it is added or subtracted like a rectangle, a figure 8 is filled wherever it is
  covered, and the trace is cut exactly on its slanted sides.
- The bar is centred with Open as its first control; the left toolbox is gone, and the Mask group
  (Rectangle, Lasso, Subtract, Inside | Outside, Clear mask) stands in the Parts row of Advanced.
- Parts (specification 0002, slice 2): the Parts row in Advanced (up to 8 parts, Add part, the
  armed trash, arrows, Home, End and Delete on the chips); every part has its own settings and
  rectangles, a later part cuts the earlier ones exactly at its edge, the preview and Save as…
  show one joined result in which shapes across a seam are one path, and a change retraces only
  the part it changes. Stop keeps the parts that were finished.
- Rectangle regions (specification 0002, slice 1): the Mask group of the bar (Rectangle, Subtract,
  Inside | Outside, Clear mask); rectangles are drawn, selected, moved, resized, nudged and deleted
  on the picture; the trace is cut exactly on their edge, the rest of the picture is washed to
  12 %, and the automatic Threshold and background read only the pixels inside the mask.
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
