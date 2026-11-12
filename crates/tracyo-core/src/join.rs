//! The results of the parts joined into one: the outlines that end on a clip edge are united per
//! colour across the parts, so a shape that crosses a seam becomes one path and no line is left
//! along the seam.
//!
//! Every outline whose exact box meets no edge of its part's clip stays as it is, curves
//! included; the pieces on an edge are united per fill colour, one operand per part, on the
//! 0.001 mm grid. Layers merge by exact RGB, largest painted area first; a single-colour part
//! counts as `#000000`.

use std::collections::HashMap;

use i_overlay::core::overlay_rule::OverlayRule;
use kurbo::{BezPath, Shape};
use vectrace::shapes::ShapeRegion;
use vectrace::wire::{Layer, Output, Status, Texts};
use vectrace::{Rgb8, node_count};

use crate::clip::{crossing_box, outer_sign, wound_polylines};
use crate::grid::{self, Path64};
use crate::stack::PartClip;

/// The parts' traced results joined into one (`parts`: each part's result and its clip, in
/// stacking order); the palette is that of part `palette_of` when it adds anything, else of the
/// last part that does. One part that adds anything is returned as it is; a part that did not
/// trace (too detailed, invalid) makes the join say so.
#[must_use]
pub fn join(parts: &[(&Output, &PartClip)], palette_of: usize) -> Output {
    let live: Vec<(usize, &Output, &PartClip)> = parts
        .iter()
        .enumerate()
        .filter(|(_, (_, clip))| clip.contributes())
        .map(|(k, (wire, clip))| (k, *wire, *clip))
        .collect();
    if let [(_, wire, _)] = live.as_slice() {
        return (*wire).clone();
    }
    if let Some((_, failed, _)) = live.iter().find(|(_, w, _)| w.status != Status::Traced) {
        return empty(failed.status);
    }
    let multicolour = live.iter().any(|(_, w, _)| !w.layers.is_empty());
    let sign = outer_sign(
        &live
            .iter()
            .flat_map(|(_, w, _)| w.outlines.iter().cloned())
            .collect::<Vec<_>>(),
    );
    let mut joined = empty(Status::Traced);
    // Where each part's kept outline went: (part, old index) to new index.
    let mut moved: HashMap<(usize, usize), usize> = HashMap::new();
    for bucket in sort_into_colours(&live, multicolour) {
        let start = joined.outlines.len();
        for (key, outline) in bucket.kept {
            moved.insert(key, joined.outlines.len());
            joined.outlines.push(outline);
        }
        for (key, outline) in bucket.seam.unite(sign) {
            if let Some(key) = key {
                moved.insert(key, joined.outlines.len());
            }
            joined.outlines.push(outline);
        }
        if multicolour {
            joined.layers.push(Layer {
                colour: bucket.colour,
                outlines: joined.outlines.len() - start,
            });
        }
    }
    for (k, wire, _) in &live {
        joined.specks_removed += wire.specks_removed;
        joined.foreground_pixels += wire.foreground_pixels;
        joined.joined += wire.joined;
        joined.unclipped += wire.unclipped;
        joined.texts.found = joined.texts.found.max(wire.texts.found);
        joined.texts.lines.extend(wire.texts.lines.iter().cloned());
        joined.shapes.extend(
            wire.shapes
                .iter()
                .filter_map(|region| moved_region(region, *k, &moved)),
        );
    }
    if multicolour {
        // Lines of a single-colour part are black beside the coloured fills of the others.
        for line in &mut joined.texts.lines {
            line.fill.get_or_insert(Rgb8::new(0, 0, 0));
        }
        let from = live
            .iter()
            .find(|(k, _, _)| *k == palette_of)
            .or_else(|| live.iter().rev().find(|(_, w, _)| !w.palette.is_empty()))
            .map(|(_, w, _)| *w);
        if let Some(from) = from {
            joined.palette.clone_from(&from.palette);
        }
    }
    joined.paths = joined.outlines.len();
    joined.nodes = joined.outlines.iter().map(node_count).sum();
    joined
}

/// The outlines of one fill colour over all parts.
struct Bucket {
    colour: Rgb8,
    /// The outlines that stay as they are, with their part and index.
    kept: Vec<((usize, usize), BezPath)>,
    seam: Seam,
}

/// The pieces of one colour that end on a clip edge, per part.
#[derive(Default)]
struct Seam {
    parts: Vec<(usize, Vec<(usize, BezPath)>)>,
}

impl Seam {
    fn push(&mut self, part: usize, index: usize, outline: BezPath) {
        match self.parts.last_mut() {
            Some((k, pieces)) if *k == part => pieces.push((index, outline)),
            _ => self.parts.push((part, vec![(index, outline)])),
        }
    }

    /// The pieces united when they come from two parts or more, as polylines wound with `sign`;
    /// pieces of a single part stay as they are, with their part and index.
    fn unite(self, sign: f64) -> Vec<(Option<(usize, usize)>, BezPath)> {
        let keep_all = |parts: Vec<(usize, Vec<(usize, BezPath)>)>| {
            parts
                .into_iter()
                .flat_map(|(k, pieces)| {
                    pieces
                        .into_iter()
                        .map(move |(index, outline)| (Some((k, index)), outline))
                })
                .collect()
        };
        if self.parts.len() < 2 {
            return keep_all(self.parts);
        }
        // One operand per part: each part's pieces become a clean set (outer outlines positive,
        // holes negative) before the sets are added up, so one part's winding cannot cancel
        // another's.
        let mut all: Vec<Path64> = Vec::new();
        for (_, pieces) in &self.parts {
            let mut rings: Vec<Path64> = Vec::new();
            for (_, path) in pieces {
                let Some(r) = grid::flatten_snap(path) else {
                    // A coordinate the grid cannot hold: the pieces stay as they are.
                    return keep_all(self.parts);
                };
                rings.extend(r);
            }
            all.extend(grid::run(OverlayRule::Subject, &rings, &[]));
        }
        let united = grid::canonical(grid::run(OverlayRule::Subject, &all, &[]));
        wound_polylines(&united, sign)
            .into_iter()
            .map(|outline| (None, outline))
            .collect()
    }
}

/// The live parts' outlines sorted into one bucket per fill colour (black for a single-colour
/// part), largest painted area first; ties keep the order in which the colours appear.
fn sort_into_colours(live: &[(usize, &Output, &PartClip)], multicolour: bool) -> Vec<Bucket> {
    let black = Rgb8::new(0, 0, 0);
    let mut buckets: Vec<Bucket> = Vec::new();
    for (k, wire, clip) in live {
        let single = [Layer {
            colour: black,
            outlines: wire.outlines.len(),
        }];
        let layers: &[Layer] = if wire.layers.is_empty() {
            &single
        } else {
            &wire.layers
        };
        let mut index = 0;
        for layer in layers {
            let colour = if multicolour { layer.colour } else { black };
            let at = buckets
                .iter()
                .position(|b| b.colour == colour)
                .unwrap_or_else(|| {
                    buckets.push(Bucket {
                        colour,
                        kept: Vec::new(),
                        seam: Seam::default(),
                    });
                    buckets.len() - 1
                });
            for outline in &wire.outlines[index..index + layer.outlines] {
                let on_edge = clip
                    .region()
                    .is_some_and(|region| region.edge_meets(crossing_box(outline)));
                if on_edge {
                    buckets[at].seam.push(*k, index, outline.clone());
                } else {
                    buckets[at].kept.push(((*k, index), outline.clone()));
                }
                index += 1;
            }
        }
    }
    let painted = |b: &Bucket| -> f64 {
        b.kept
            .iter()
            .map(|(_, o)| o.area())
            .chain(
                b.seam
                    .parts
                    .iter()
                    .flat_map(|(_, p)| p.iter().map(|(_, o)| o.area())),
            )
            .sum::<f64>()
            .abs()
    };
    let mut keyed: Vec<(f64, Bucket)> = buckets.into_iter().map(|b| (painted(&b), b)).collect();
    // Stable: equal areas keep the order in which the colours appeared.
    keyed.sort_by(|a, b| b.0.total_cmp(&a.0));
    keyed.into_iter().map(|(_, b)| b).collect()
}

/// A shape region of part `part` with its outlines at their joined places; `None` when one of
/// them went into a seam union.
fn moved_region(
    region: &ShapeRegion,
    part: usize,
    moved: &HashMap<(usize, usize), usize>,
) -> Option<ShapeRegion> {
    let outlines = region
        .outlines
        .iter()
        .map(|&i| moved.get(&(part, i)).copied())
        .collect::<Option<Vec<usize>>>()?;
    Some(ShapeRegion {
        outlines,
        ..region.clone()
    })
}

/// A result with `status` and nothing in it.
fn empty(status: Status) -> Output {
    Output {
        status,
        paths: 0,
        nodes: 0,
        specks_removed: 0,
        foreground_pixels: 0,
        covers_whole_image: false,
        outlines: Vec::new(),
        palette: Vec::new(),
        layers: Vec::new(),
        joined: 0,
        texts: Texts::default(),
        shapes: Vec::new(),
        unclipped: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clip::Clip;
    use crate::regions::{MaskMode, PxRect, Region};
    use vectrace::PixelMap;
    use vectrace::wire::Palette;

    fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> BezPath {
        let mut p = BezPath::new();
        p.move_to((x0, y0));
        p.line_to((x1, y0));
        p.line_to((x1, y1));
        p.line_to((x0, y1));
        p.close_path();
        p
    }

    /// A result with one layer per `(colour, outline)`; no layers when `colours` is false.
    fn output(outlines: &[(Rgb8, BezPath)], colours: bool) -> Output {
        let mut o = empty(Status::Traced);
        for (_, p) in outlines {
            o.outlines.push(p.clone());
        }
        o.paths = o.outlines.len();
        if colours {
            for (c, _) in outlines {
                o.layers.push(Layer {
                    colour: *c,
                    outlines: 1,
                });
                o.palette.push(Palette {
                    colour: *c,
                    fixed: false,
                    pixels: 1,
                });
            }
        }
        o
    }

    /// 1 px = 1 mm, the mask x 0 to 50.
    fn clips() -> (PartClip, PartClip) {
        let region = Region {
            rect: PxRect::new(0, -100, 50, 200),
            subtract: false,
        };
        let map = PixelMap::scale(1.0, 1.0);
        let inside = Clip::of_regions(&[region], MaskMode::Inside, map)
            .unwrap()
            .unwrap();
        let outside = Clip::of_regions(&[region], MaskMode::Outside, map)
            .unwrap()
            .unwrap();
        (PartClip::Region(outside), PartClip::Region(inside))
    }

    const RED: Rgb8 = Rgb8::new(255, 0, 0);
    const BLUE: Rgb8 = Rgb8::new(0, 0, 255);

    #[test]
    fn equal_colours_merge_across_the_seam_into_one_outline() {
        let (below, above) = clips();
        let a = output(&[(RED, rect(50.0, 10.0, 90.0, 30.0))], true);
        let b = output(&[(RED, rect(20.0, 10.0, 50.0, 30.0))], true);
        let j = join(&[(&a, &below), (&b, &above)], 1);
        assert_eq!(j.layers.len(), 1);
        assert_eq!((j.layers[0].colour, j.layers[0].outlines), (RED, 1));
        assert_eq!(j.paths, 1);
        let bounds = j.outlines[0].bounding_box();
        assert_eq!((bounds.x0, bounds.x1), (20.0, 90.0));
        assert_eq!(j.palette.len(), 1);
    }

    #[test]
    fn different_colours_stay_in_their_own_layers_largest_first() {
        let (below, above) = clips();
        let a = output(&[(BLUE, rect(60.0, 10.0, 70.0, 20.0))], true);
        let b = output(&[(RED, rect(20.0, 10.0, 50.0, 30.0))], true);
        let j = join(&[(&a, &below), (&b, &above)], 0);
        let colours: Vec<_> = j.layers.iter().map(|l| l.colour).collect();
        assert_eq!(colours, [RED, BLUE]);
        assert_eq!(j.paths, 2);
        assert_eq!(j.palette[0].colour, BLUE, "the palette of part 1");
    }

    #[test]
    fn a_single_colour_part_counts_as_black_beside_a_multicolour_one() {
        let (below, above) = clips();
        let a = output(&[(RED, rect(50.0, 10.0, 90.0, 30.0))], false);
        let black = Rgb8::new(0, 0, 0);
        let b = output(&[(black, rect(20.0, 10.0, 50.0, 30.0))], true);
        let j = join(&[(&a, &below), (&b, &above)], 1);
        assert_eq!(
            j.layers.len(),
            1,
            "the single-colour part is black, as the other is"
        );
        assert_eq!(j.layers[0].colour, black);
        assert_eq!(j.paths, 1, "united across the seam");
    }

    #[test]
    fn one_part_that_adds_anything_is_returned_as_it_is() {
        let (below, _) = clips();
        let a = output(&[(RED, rect(50.0, 10.0, 90.0, 30.0))], true);
        let nothing = output(&[], false);
        let j = join(&[(&a, &below), (&nothing, &PartClip::Nothing)], 0);
        assert_eq!(j, a);
    }

    #[test]
    fn a_part_that_did_not_trace_fails_the_join() {
        let (below, above) = clips();
        let a = output(&[(RED, rect(50.0, 10.0, 90.0, 30.0))], false);
        let mut b = output(&[], false);
        b.status = Status::TooDetailed;
        let j = join(&[(&a, &below), (&b, &above)], 0);
        assert_eq!((j.status, j.paths), (Status::TooDetailed, 0));
    }

    #[test]
    fn a_shape_region_follows_its_outline_and_goes_when_the_outline_was_united() {
        let (below, above) = clips();
        let mut a = output(
            &[
                (RED, rect(50.0, 10.0, 90.0, 30.0)),
                (RED, rect(70.0, 40.0, 80.0, 50.0)),
            ],
            false,
        );
        a.shapes.push(ShapeRegion {
            outlines: vec![1],
            shapes: vec![None],
            band: None,
            bands: Vec::new(),
        });
        let b = output(&[(RED, rect(20.0, 10.0, 50.0, 30.0))], false);
        let j = join(&[(&a, &below), (&b, &above)], 1);
        assert_eq!(
            j.shapes.len(),
            1,
            "the lone square is untouched by the seam"
        );
        let moved = j.shapes[0].outlines[0];
        assert_eq!(j.outlines[moved].bounding_box().x0, 70.0);
    }
}
