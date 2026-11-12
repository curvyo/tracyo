# ADR 0008: Trace parts: where the cut and the join live, and how parts are traced

**Status:** Proposed (architect, 2026-11-12). Extends ADR 0002 (modules) and ADR 0004 §2, §3, §5
(protocol, cancel, reduced preview); supersedes nothing.

## Context

Spec `specs/0002-trace-parts`: up to 8 parts, each with its own settings and a mask made of
rectangles (customer, 2026-11-12: rectangles only), added or subtracted; the trace of a part is cut
exactly at its mask edge (criterion 13, vertices within 0.005 mm), Inside and Outside fit without
gap or overlap (14), parts that meet at a seam are joined into one outline without a seam line (15),
a later part wins where parts overlap (5, 16), results are byte-identical on repeated runs (22).

Facts. vectrace v0.1.1 has the seam for this: `job::Job::mask: Option<&dyn mask::Mask>`, applied
in all four job paths (mono, colour, with and without text), text lines kept by their box centre,
shapes recognised after the clip. Its own ADR (Curvyo ADR 0048 §4, option C rejected) keeps the
boolean kernel out of vectrace on purpose: the caller brings the clip. Curvyo implements it on its
kernel (`curvyo-ui-core::trace_region`, `trace_parts`, `trace_region_raster`, about 1,170 lines;
`curvyo-geometry-core` on `i_overlay` `=9.0.1` on a 0.001 mm integer grid, ADR 0003 of Curvyo).

**Spike** (throwaway crate, `i_overlay` 9.0.1 `i64` engine on a 0.001 mm grid, vectrace 0.1.1,
the spec's P1 traced in release):

| Check | Result |
|---|---|
| 13, R1 Inside, Threshold 70 | 2 seam vertices, worst 0.0003 mm off x = 132.292; corners (132.292, 127.000) and (132.292, 137.583); the cut shape has only `M`/`L`/`Z`, the small disc keeps its 4 curves |
| 14, R1 Inside + R1 Outside | A + B − joined = 0.00000 mm², A ∩ B = 0, joined XOR whole trace = 0 % |
| 15, Threshold 40/Smooth 20 + R1 Inside Threshold 70/Smooth 80 | 2 `M`, 2 `Z`; 0 mm of segments along the seam |
| 22 | two runs byte-identical (Debug output compared) |
| Rectangles on pixel lines | P1's trace has 5 straight segments lying exactly on x = 600 px or y = 480 px. A mask with edges on those lines, an L of two rectangles and a frame (rectangle minus rectangle): Inside + Outside = whole exactly, Inside ∩ Outside = 0, join XOR whole = 0 |
| Crop (criterion 25) | R1 Inside traced from a 516 px wide crop (16 px margin) is byte-identical to the whole-picture trace clipped to R1, at three settings |
| Cost | trace 33 ms, both clips 0.13 ms |
| Build | `wasm32-unknown-unknown` builds (not needed by Tracyo; matters for a later move to vectrace); `cargo deny check licenses` with Tracyo's `deny.toml` passes |

Weight: five crates, `i_overlay` (MIT OR Apache-2.0), `i_float`, `i_shape`, `i_tree`, `i_key_sort`
(MIT), about 43,000 source lines; their other dependency `libm` is already in `Cargo.lock`. They use
`unsafe` internally (about 130 sites), as Curvyo accepted in its ADR 0003; Tracyo's own crates keep
`forbid(unsafe_code)`.

| Option | Verdict |
|---|---|
| A. A `parts` module in vectrace (polygon `Mask`, stacking, join) on `i_overlay`, then a vectrace PR and tag | Lost for now. Its only user would be Tracyo: Curvyo keeps its kernel, so this is a speculative generalisation (`CLAUDE.md` §5), against vectrace's own decision to leave the clip with the caller, and it puts a vectrace release before Tracyo's first slice. It stays the way to share the code later (Consequences). |
| **B. In `tracyo-core`, on `i_overlay` `=9.0.1` (chosen)** | Tracyo implements `Mask` as Curvyo does (the trait's second production implementation). No vectrace change. Every spec case of the spike passes; the cut costs well under 1 % of a trace. |
| B'. In `tracyo-core` without a boolean crate: clip outlines against the rectangles (Sutherland–Hodgman per rectangle), then repair | Lost. The mask is rectilinear, but the subjects are arbitrary traced outlines. Clipping a concave outline to a rectangle leaves zero-width bridges along the cut; a mask of several rectangles splits a crossing shape into pieces with seams inside one part; the join of two parts whose pieces meet the seam at different points (different Threshold and Smooth) is a union along that line. Repairing all three is a union restricted to axis-parallel lines: cancelling collinear edges in 1-D on each cut line and re-linking rings at shared vertices, i.e. a boolean kernel by another name (estimate 400 to 600 lines plus property tests). Its hard case is the normal case here: traced straight edges lie on pixel lines (spike) and region edges too. Collinear and shared edges are exactly where Curvyo's first kernel library failed its property tests (12 to 16 of 5,877 cases, Curvyo ADR 0003 amendment 2026-10-09). |
| C. Rasterise the mask and trace only the pixels inside | Lost, as in Curvyo 0094 decision 1(b): the edge follows the fit, not the mask, and Inside plus Outside leave a seam of up to a pixel (fails 13 and 14). |

## Decision

### 1. The cut and the join live in `tracyo-core`

New modules (pure, one responsibility each); only `grid` imports `i_overlay`:

| Module | Responsibility |
|---|---|
| `grid` | the 0.001 mm integer grid: snapping, the overlay calls (`Overlay::<i64>`, nonzero), the canonical order of polygons |
| `clip` | a part's kept area on the grid, Inside or Outside, as vectrace's `Mask` |
| `stack` | each part's clip once the later parts are taken out |
| `join` | the parts' results joined into one `vectrace::wire::Output` |
| `crop` | the pixel box a part is traced from, with the moved pixel map |
| `regions` | the region model: rectangles in picture pixels with a sign |
| `region_edit` | the geometry of drawing, hit testing, handles, moving and resizing a rectangle |
| `wash` | the picture area the active part does not trace, as disjoint rectangles |
| `parts` | the parts of the open picture: the list, the active part, each part's values |

Signatures (doc comments and details in the code):

```rust
// regions
pub struct PxRect { pub x0: i32, pub y0: i32, pub x1: i32, pub y1: i32 } // x0 < x1, y0 < y1; may lie off the picture
pub struct Region { pub rect: PxRect, pub subtract: bool }
pub enum MaskMode { Inside, Outside }
// clip
pub struct Clip { /* grid polygons (canonical), outside: bool, bounds */ }
pub enum ClipError { Kernel }                                   // "could not be combined"
impl Clip {
    pub fn of_regions(regions: &[Region], mode: MaskMode, map: PixelMap) -> Result<Option<Self>, ClipError>; // None: empty mask
    pub fn meets(&self, picture: kurbo::Rect) -> bool;          // false: "the mask is outside the image"
    pub fn edge_meets(&self, bounds: kurbo::Rect) -> bool;
    pub fn kept_pixels(&self, map: PixelMap, width: u32, height: u32) -> Option<Vec<bool>>;
    pub fn to_units(&self) -> Vec<i64>;  pub fn from_units(units: &[i64], outside: bool) -> Option<Self>;
}
impl vectrace::mask::Mask for Clip { fn keeps(..); fn clip(..) -> Clipped; }
// stack
pub enum PartArea { Whole, Mask(Clip), Nothing }
pub enum PartClip { Nothing, Whole, Region(Clip) }
pub fn part_clips(areas: &[PartArea], picture_mm: kurbo::Rect) -> Vec<PartClip>;
// join
pub fn join(parts: &[(&Output, &PartClip)], palette_of: usize) -> Output;
// crop
pub struct PxBox { pub x0: u32, pub y0: u32, pub x1: u32, pub y1: u32 }
pub fn crop_box(clip: &PartClip, request: &TraceRequest, width: u32, height: u32, map: PixelMap) -> Option<PxBox>;
// wash
pub fn not_traced(regions: &[Region], mode: MaskMode, picture: PxRect) -> Vec<PxRect>;
// region_edit
pub enum Handle { N, NE, E, SE, S, SW, W, NW }
pub fn drawn(from: Point, to: Point, centre: bool, square: bool) -> Option<PxRect>; // None under 2 px
pub fn region_at(regions: &[Region], p: Point, slop_px: f64) -> Option<usize>;    // topmost
pub fn handle_at(rect: kurbo::Rect, pointer: kurbo::Point) -> Option<Handle>;      // screen px, ux §5 radii
pub fn resized(rect: PxRect, handle: Handle, to: Point, centre: bool, keep_ratio: bool) -> PxRect; // >= 1 px
pub fn moved(rect: PxRect, dx: i32, dy: i32) -> PxRect;
// parts
pub const MAX_PARTS: usize = 8;
pub struct Part { pub mode: MaskMode, pub regions: Vec<Region>, pub settings: Settings, pub values: PictureValues, pub analysis: Option<Analysis> }
pub struct Parts { /* parts, active */ }
impl Parts { pub fn new(settings: Settings, values: PictureValues) -> Self; pub fn add(&mut self) -> bool; pub fn remove_active(&mut self) -> bool;
             pub fn select(&mut self, index: usize); pub fn active(&self) -> &Part; pub fn active_mut(&mut self) -> &mut Part;
             pub fn first(&self) -> &Part; pub fn areas(&self, map: PixelMap) -> Vec<Result<PartArea, ClipError>>; }
```

`clip`, `stack` and `join` are ports of Curvyo's `trace_region` (the `Mask` half), `trace_parts`
and `trace_region_raster`, with Curvyo's rules: an outline whose control box meets no clip edge
stays whole with its curves (one point test decides in or out); the crossing outlines go through
one overlay as one operand (Intersection for Inside, Difference for Outside) and come back as
polylines in the place of the first crossing one; a kernel refusal keeps them whole when the clip
keeps their first point. The join keeps every outline whose box meets no edge of its part's clip
and unites the pieces on clip edges per fill colour, one operand per part, on the same grid; pieces
of a single part are not united; layers merge by exact RGB, largest painted area first; a
single-colour part counts as `#000000`; shape regions follow their outlines and are dropped when an
outline went into a union; text lines are concatenated; a part that did not trace fails the join.

### 2. Regions, the mask and its tolerance

- A region is a rectangle with integer picture-pixel corners (rounded while drawing, moving and
  resizing; **Proposal**), at least 1 px wide and high, allowed off the picture. Its sign is
  `subtract`. The mask is `union(added) − union(subtracted)`, one Difference overlay, nonzero rule.
  Drawing order does not matter: the result is put in canonical order (start vertex, area, vertices,
  as Curvyo's kernel), so any permutation of the regions gives the same bytes.
- Corners are mapped to mm by the full picture's pixel map (`25.4 / 96` per px) and snapped to the
  0.001 mm grid; a rectangle needs no flattening. Traced curves that cross an edge are flattened at
  0.003 mm. Budget against criterion 13's 0.005 mm: 0.003 + 0.0007 (half the grid diagonal); the
  spike measured 0.0003 mm.
- "Whole" and Outside are taken against the picture's box grown by 1 mm, never against the box
  itself, so traced outlines touching the picture border never count as crossing (Curvyo 0094
  decision 1). The picture box is used for `Clip::meets` (notice "the mask is outside the image"),
  the wash, the kept pixels and the crop only.
- Notices: all added regions off the picture, or an empty mask with Inside: "outside the image".
  `ClipError::Kernel` (the overlay returns nothing for regions of positive area, never expected):
  "could not be combined". Neither starts a job.

### 3. Parts

- `Parts` holds 1 to 8 parts in creation order (bottom first) and the active index. `add` appends,
  copies the active part's `Settings` and `PictureValues` without picked colours and palette edits,
  Inside, no regions, and makes it active; `remove_active` (two or more parts) makes the part before
  active; opening a picture is `Parts::new` with part 1's remembered settings (0001/1).
- Each part's area: part 1 without regions is `Whole`; a later part without regions, or one whose
  mask misses the picture, is `Nothing`; otherwise `Mask(clip)`. `part_clips` takes out every later
  part's area (later wins): untouched parts keep their own clip (curves intact), trimmed ones get the
  visible polygons as an Inside clip, hidden ones get no job.
- The bar and Advanced read and write `parts.active_mut()`. `tracyo-app::config` writes only
  `parts.first().settings` to `settings.toml`, as today; no new key (criterion 23).
- The app recomputes areas and clips on the UI thread when a region or a mode changes (release of a
  drag, a key, Clear mask, Add or remove part), not while a drag runs. Cost: overlays of at most
  160 rectangles, far below a millisecond (criterion 27 measures it).

### 4. Worker protocol (extends ADR 0004)

- One part without regions sends today's `Trace` request: criterion 1 of 0001 stays bit-identical.
- Otherwise one job carries all contributing parts: `Request::TraceParts(Vec<PartJob>)` with
  `PartJob { part: u8, request: TraceRequest, clip: Option<Vec<i64>> /* grid units, None = whole */,
  outside: bool, crop: Option<PxBox>, cached: Option<Vec<f64>> }`. The worker traces each job
  without a cached wire, in part order, from its crop (a copied sub-raster and the moved map) with
  the clip as `Job::mask`, sends `Reply::PartWire { part, wire }` as soon as it is done, then joins
  and sends `Reply::Wire(joined)`.
- **Cache and cancel.** Latest wins and Stop kills the worker, as before. The app keeps every
  `PartWire` it has received under the part job's encoded bytes without `cached` (settings, lines,
  clip, crop, reduced), at most 32 entries, so a killed job loses only the part that was running.
  A change of the active part's setting changes only its key (criterion 20); a mask change changes
  the clips, hence the keys, of the parts below it.
- **Progress.** One 0 to 100 % over the job: each part to trace weighs its raster's pixel count;
  percent = ⌊99 · (done + weight · part percent / 100) / total⌋, 100 after the join; it never runs
  backwards; cached parts weigh nothing.
- **Reduced preview.** Per part, as ADR 0004 §5: while a value is dragged only the active part is
  traced reduced, the others come from the cache; over 4 Mpx every preview part is reduced;
  Save as… traces every part full in one job (criterion 21). Reduced and full results are in the same
  mm, so they join.
- **Crop** (criterion 25): the clip's pixel box plus Curvyo's margin (`CROP_MARGIN_PX` = 16 px, or
  Ignore below in px when that is more), clamped to the picture. The whole picture is sent with Recognise text (lines are in the
  picture's pixels), Drop border shapes, Denoise and Upscale Auto (fits that are not local; Curvyo
  tester 2026-11-06), for `Whole` and for a box that covers the picture.
- Rejected: one job per part with the join on the UI thread. Progress would restart per part, the
  join's union would run on the UI thread (frame budget, criterion 25), and latest-wins would have to
  cancel a queue.

### 5. Interaction in egui (`tracyo-app`)

- `region_tool`: the input state (idle, drawing, moving, resizing, each with the start geometry so
  Esc restores), the canvas keys (Delete, Backspace, arrows, Esc order of criterion 8), the cursors
  of ux §5. `region_paint`: region outlines of every part, the selection box and handles, the drag
  ghost, the size chip, the Remove badge, the wash `Mesh` from `wash::not_traced`. `parts_row`: the
  chips, Add part and the armed trash (time passed in as `Duration`, like `progress`).
- All geometry is `region_edit` in core, in picture pixels, unit-tested; the app only converts
  between screen and picture with the canvas's view transform and paints. Undo is out of scope.

### 6. Tests

- `tracyo-core` unit tests per module; ports of Curvyo's `trace_region` and `trace_parts` tests.
- Property tests (a fixed-seed generator in the test, no new dependency, 2,000 cases): rectangles on
  a 6 px lattice (coincident edges with pixel-aligned traced edges are the point) over traced
  fixtures. Inside + Outside = whole within 0.01 mm², Inside ∩ Outside ≤ 0.01 mm², the clip is
  idempotent (byte-equal), the join of Inside and Outside differs from the whole trace by ≤ 0.1 % and
  has no collinear overlapping segments on a clip edge above 0.01 mm, a permutation of the regions
  gives the same bytes.
- Goldens: `tests/fixtures/pictures/p1.png`, `p2.png` (written by a generator test), P3 from Curvyo's
  0091 set with its source noted; `tests/fixtures/svg/parts-*.svg` for criteria 13, 15, 17; the
  structure checks of 13 to 19 and 22 with `roxmltree`. A crop test compares a cropped part with its
  whole-picture trace (≤ 0.1 %) for the settings that may crop.
- App (`egui_kittest`, real worker): drawing, selection, resize, delete, Esc order, parts row, trash,
  Save as…; the `PartWire` count for criterion 20; monotone progress for 21; the pixel cases of
  ux-notes §12.2 against re-captured references.
- `perf` job (release, thread CPU time): criterion 25 in-process in `tracyo-core` (the worker runs the
  same code; `rustix` as a unix dev-dependency), and the UI frame budget with 8 parts × 20 regions
  (criterion 27) in `tracyo-app`, as 0001's frame test.

## Consequences

- New dependency of `tracyo-core`: `i_overlay = { version = "=9.0.1", default-features = false }`
  (exact pin, Curvyo's version; `allow_multithreading` off). Renovate bumps it alone and never
  automerges; a bump reruns the property tests. `THIRD-PARTY.md` is regenerated.
- No vectrace change: v0.1.1's `Mask` is enough.
- Cut shapes are polylines over their whole outline (criterion 13); node counts rise where shapes
  cross an edge.
- ADR 0002's module list grows by nine core and three app modules; no new crate.
- Two implementations of the cut and join exist (Curvyo's on its kernel, Tracyo's). **Later
  migration:** `grid`, `clip`, `stack` and `join` depend only on vectrace, kurbo and `i_overlay`, take
  grid polygons rather than Tracyo's rectangles, and build for wasm32. When Curvyo wants to drop
  its own clip half of `trace_region` and `trace_parts` (a second user), they move into vectrace as
  `vectrace::parts` with a vectrace ADR, a PR and a tag, and Curvyo passes its kernel's union
  polygons as the area. Precondition: Curvyo's 0094 acceptance tests pass unchanged; its goldens may
  change in vertex order (canonical forms differ), which is a Curvyo story.
- Debt 6 (analysis on an app thread) now runs once per part clip (criterion 18).
