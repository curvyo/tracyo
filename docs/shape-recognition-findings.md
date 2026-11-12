# Shape recognition findings (2026-11-12)

What vectrace v0.1.0 recognises in the three reference pictures `forms.png`, `forms2.png` and
`stars.png` (vectrace `assets/trace-references/`), traced as Tracyo does on opening (analysis
threshold 128, Brightness, default settings, Recognise shapes on, 794 x 1123 px). The numbers are
`Output.shapes` per outline, in picture pixels. Overlays (recognised outlines green, unrecognised
red) were written next to this work as `<name>-overlay.svg`; they are not part of the repository.

Summary: of 52 outlines, 47 are recognised. `forms.png` and `stars.png` are recognised completely;
the "only paths" the customer saw there came from Tracyo writing polygons and stars as `<path>`
(fixed by ADR 0005 section 6). `forms2.png` has four shapes that stay in the compound path.

## forms.png (5 outlines, 4 shapes, all recognised)

| Outlines | Picture | vectrace |
|---|---|---|
| #0 | filled rounded rectangle 154 x 275 | rect, corner radius 25 |
| #1 | filled ellipse | ellipse 112 x 94.5 |
| #2 | filled 15-pointed star | star n=15, inner ratio 0.51 |
| #3, #4 | stroked 8-pointed star outline | band: star n=8, ratio 0.51, width 3.7 |

## stars.png (20 outlines, 10 stars, all recognised)

All ten are stroked 8-pointed stars (inner ratio 0.57 to 0.58), stroke widths 1.9 to 7.6 px,
one touching the left picture edge (#14/#15, still a closed ring). Each outer/hole pair (#0/#1 to
#18/#19) is one band: star n=8. Nothing is missed.

## forms2.png (27 outlines, 14 shapes)

| Outlines | Picture | vectrace | Class |
|---|---|---|---|
| #1, #2 | small stroked 8-star | band star n=8, ratio 0.49 | ok |
| #3, #4 | big stroked 10-star | band star n=10, ratio 0.50 | ok |
| #7, #8 | circle inside the big star | band circle r=77.9 | ok |
| #5, #6 | stroked rounded rectangle | band rect, r=25 | ok |
| #11, #12 | wobbly stroked 8-gon | band star n=8, ratio 0.83 | ok (near a polygon) |
| #13, #14 | stroked square at the right edge | band rect | ok |
| #15, #16 | stroked octagon | band polygon n=8 | ok |
| #19, #20 | stroked 8-gon, lower left | band polygon n=8 | ok |
| #23, #24 | stroked 6-star | band star n=6, ratio 0.49 | ok |
| #25, #26 | stroked 10-star, lower left | band star n=10, ratio 0.49 | ok |
| #9, #10 | thin ring between two ellipses | region: outer ellipse 81 x 27.2, hole ellipse 76.3 x 23.6, **no band** | likely vectrace gap |
| #0 | stroked 10-star cut by the top edge | **none** | irregular input |
| #17, #18 | irregular stroked star, 11 tips, lower right | **none** | irregular input |
| #21, #22 | small stroked star, 9 tips, stretched | **none** | irregular input |

### The misses

- **#0, cut by the picture edge.** The star's stroke is open at the top edge, so the trace is one
  58-node loop that runs around the stroke instead of an outer and a hole (tip radii 42 to 52 px
  around its centre, valleys 20 to 30). No regular star fits a clipped outline. Not a gap.
- **#17/#18, irregular star.** Its tips lie at 105 to 129 px from the centre (valleys 86 to 105),
  tip angles spaced 40 to 46 degrees apart. Far outside the tolerance (2 px floor, 1 % of
  the square root of the area). The picture is not regular.
- **#21/#22, stretched star.** Tips at 63 to 71 px, valleys at 33 to 39 px: a regular star drawn
  about 10 % wider than high. The spread is about 7 px against the 2 px floor, so it is not a
  tolerance problem (4 px would be needed, and would also accept many things that are not
  stars). `Primitive` has no stretched star; `star::polygon_or_star` is right to refuse it.
- **#9/#10, the ellipse ring.** The two ellipses are within the fit tolerance, and they are
  close to offsets of one another (width 4.7 px at the tips, 3.6 px at the sides). In vectrace
  `band::ellipse_band` returns the width as `midpoint(da, db)` = 4.15 px, then `shapes::band`
  checks the stroked area (Steiner: perimeter 348.6 x 4.15 = 1447 px2) against the traced ring
  (1265 px2) and refuses at 14 % (limit `BAND_AREA_SHARE` = 3 %). A width taken from the traced
  area (1265 / 348.6 = 3.63 px) passes the same check, and every point of the ring is then within
  about 0.6 px of the stroked ellipse, inside the tolerance. Stars already take their width from
  the traced area (`band_of` passes `traced` for that). **Likely vectrace gap: `ellipse_band`
  (src/shapes/band.rs) should take the width that covers the traced area, as `star_band` does.**
  Until then Tracyo writes the ring as part of the layer's compound path with the ellipses'
  geometry (ADR 0005 section 6), so it is a path in Inkscape, not an `<ellipse>`.

## Tracyo's output for the three pictures (real app, Recognise shapes on, Save)

Saved through the kittest harness with the fixed save dialog under `xvfb-run` (the real dialog
cannot be driven there, plan.md slice 4):

| Picture | Elements in the SVG |
|---|---|
| forms.png | 1 rect, 1 ellipse, 2 stars (Inkscape objects), 2 paths |
| forms2.png | 2 rect, 1 circle, 2 polygon, 5 stars, 6 paths (compound path and the ring) |
| stars.png | 10 stars |

`rsvg-convert` draws `forms2.png`'s SVG at the source's size and every shape is where the
picture has it; the four misses above are drawn as traced.
