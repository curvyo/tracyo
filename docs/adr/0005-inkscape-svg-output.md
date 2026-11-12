# ADR 0005: The Inkscape SVG Tracyo writes

**Status:** Proposed (architect, 2026-11-11). Follows spec criterion 20 with the defaults of its
Fragen 1 (text as stretched `<text>`) and 2 (96 dpi). Frage 3 (polygons and stars as `<path>`)
was overruled by the customer on 2026-11-12, see §6.
The only file format Tracyo writes, so it is the one thing here that is expensive to change.

## Context

Makers open the file in Inkscape, Ink/Stitch or a cutter program. Options for the writer:

| Option | Verdict |
|---|---|
| **A. Own string writer in `tracyo-core::svg` (chosen)** | About 300 lines; full control of attribute order, number format and escaping, which the golden files pin. No dependency. |
| B. `quick-xml` writer | Escaping for free, but the writer is still ours; one more dependency for a few `push_str`. |
| C. `svg` crate (document builder) | Its own element model; attribute order and numbers less under our control. |

## Decision

1. `svg::write(&Document) -> String` is pure. `Document` is built from `vectrace::wire::Output`,
   the picture size in px and the excluded/deleted colours; the app writes the string to
   `<name>.svg.tmp` beside the target and renames it (spec criterion 19).
2. **Frame:** XML declaration, root `<svg xmlns xmlns:inkscape xmlns:sodipodi version="1.1"
   width="<W>mm" height="<H>mm" viewBox="0 0 <W> <H>">`, then
   `<sodipodi:namedview inkscape:document-units="mm"/>`. W, H = pixels · 25.4 / 96.
   Coordinates are vectrace's millimetres unchanged (its `PixelMap::scale(25.4/96, 25.4/96)`).
3. **Numbers:** at most 3 decimals, trailing zeros and a trailing dot removed, `-0` written `0`;
   one formatter for every number.
4. **Layers** (`<g inkscape:groupmode="layer" inkscape:label="#RRGGBB" id="layerN">`): single
   colour one layer `#000000`; multicolour one per `Output.layers` entry in order (largest area
   first = bottom), skipping excluded and deleted colours and empty layers. Ids are numbered in
   document order.
5. **Paths:** per layer one `<path id="pathN" fill fill-rule="evenodd" stroke="none" d>` with all
   the layer's outlines not written as primitives, absolute `M L C Z` only (vectrace emits only
   move, line, cubic, close).
6. **Shapes** (`Output.shapes`, indices into the outlines): a region becomes primitive elements
   only when the fill stays the same (Curvyo's `keeps_fill` rule, ported): a hole-free region
   whose outer outline has a primitive becomes `<rect x y width height rx ry>`, `<circle>` (rx =
   ry within 1e-9 mm) or `<ellipse>`, with `transform="rotate(deg cx cy)"` when rotated. A
   regular polygon becomes `<polygon points="x,y …">` with the rotation baked into the points (no
   `transform`). A star becomes Inkscape's star object: `<path sodipodi:type="star" sodipodi:cx
   sodipodi:cy sodipodi:r1 sodipodi:r2 sodipodi:arg1 sodipodi:arg2 sodipodi:sides
   inkscape:flatsided="false" inkscape:rounded="0" inkscape:randomized="0" d>`. Meaning, checked
   against the star objects in Inkscape's own example files: cx, cy = centre, r1 = outer radius,
   r2 = inner radius (outer · inner_ratio), arg1 = angle of the first tip in radians (y down,
   so clockwise on screen; polygon angle plus rotation), arg2 = angle of the first inner corner
   (arg1 + π / sides), sides = number of tips, all in the file's millimetres. `d` is the straight
   outline from the first tip, so readers that ignore `sodipodi:*` draw the same star, and Inkscape
   redraws it identically. Ids are numbered per kind: `polygonN`, `starN`. **(Customer, 2026-11-12:
   overrules the `<path>` default of spec Frage 3.)** A band becomes its centre-line primitive with
   `fill="none" stroke="<hex>" stroke-width="<mm>"` and `stroke-linejoin` from its `Join`. A
   recognised region **with holes that is not a band** stays in the layer's `<path>` with the
   primitives' geometry (SVG cannot cut a hole into a `<rect>` without a mask, and 20h forbids
   references). **(Proposal)**
7. **Text** (Frage 1 A): each `Output.texts.lines` entry with a fill that is not excluded (black in
   single colour) becomes `<text xml:space="preserve" transform="translate(x y) rotate(a)"
   font-family="sans-serif" font-size textLength lengthAdjust="spacingAndGlyphs"><tspan>…`.
   From the line's outlines projected on its `direction` and the normal: x = start along the
   line, baseline = far edge across the line, font-size = extent across the line, textLength =
   extent along it. `&`, `<`, `>`, `"` are escaped; characters not allowed in XML 1.0 are dropped.
8. Nothing else: no `<metadata>`, no `curvyo`/`tracyo` names, no `href`, no `url(`, no image,
   no script (criterion 20h). Output is deterministic: the same result gives the same bytes.

## Consequences

- Golden files in `crates/tracyo-core/tests/fixtures/svg/` pin the bytes; a change of the format
  is a reviewed golden update.
- A rounded-off recognised rectangle with a hole is not editable as a rectangle in Inkscape.
- Font size from the ink extent makes text with descenders slightly larger than the original;
  accepted with Frage 1 A (no font fitting).
