//! Which recognised regions of a result are written as primitives, and the outline of a primitive.

use std::f64::consts::TAU;

use kurbo::{Affine, BezPath, Point, Shape as _};
use vectrace::shapes::{Band, Join, Primitive};
use vectrace::wire::Output;
use vectrace::{Angle, Mm};

/// The control-point distance of a cubic quarter circle, as a share of the radius.
const KAPPA: f64 = 0.552_284_749_830_793_4;

/// How a primitive is painted.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Style {
    /// Filled with the layer's colour.
    Fill,
    /// A band: no fill, a stroke of this width in millimetres with this join.
    Stroke {
        /// The stroke width.
        width: Mm,
        /// The line join.
        join: Join,
    },
}

/// A recognised shape written as an element of its layer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Element {
    /// The shape, before its rotation.
    pub primitive: Primitive,
    /// Its rotation about its frame's centre.
    pub rotation: Angle,
    /// How it is painted.
    pub style: Style,
}

/// What is done with the outlines of a result's recognised regions.
#[derive(Debug, Clone, Default)]
pub struct Plan {
    /// Per outline: the path drawn in its place (the primitive's, wound as the traced one), or
    /// `None` for the traced outline.
    pub drawn: Vec<Option<BezPath>>,
    /// Per outline: written as an element, not in its layer's compound path.
    pub elsewhere: Vec<bool>,
    /// The elements and the outline whose layer they belong to, in the result's order.
    pub elements: Vec<(usize, Element)>,
}

/// The layer that holds outline `index` of `output` (0 for a single colour result).
#[must_use]
pub fn layer_of(output: &Output, index: usize) -> usize {
    let mut end = 0;
    for (layer, l) in output.layers.iter().enumerate() {
        end += l.outlines;
        if index < end {
            return layer;
        }
    }
    output.layers.len().saturating_sub(1)
}

/// The plan for `output`. A hole-free recognised region becomes an element; a band becomes a
/// stroked element; any other recognised region stays in the compound path with the primitives'
/// geometry, unless that would change what is filled.
#[must_use]
pub fn plan(output: &Output) -> Plan {
    let n = output.outlines.len();
    let mut plan = Plan {
        drawn: vec![None; n],
        elsewhere: vec![false; n],
        elements: Vec::new(),
    };
    for region in &output.shapes {
        if region.outlines.iter().any(|&i| i >= n) {
            continue;
        }
        let outer = region.outlines[0];
        if let Some(band) = region.band {
            plan.elsewhere_all(&region.outlines);
            plan.elements.push((outer, stroked(band)));
        } else if !region.bands.is_empty() {
            plan.elsewhere_all(&region.outlines);
            plan.elements
                .extend(region.bands.iter().map(|&band| (outer, stroked(band))));
        } else if let [Some((primitive, rotation))] = region.shapes[..] {
            let path = primitive_path_like(&primitive, rotation, &output.outlines[outer]);
            plan.drawn[outer] = Some(path);
            plan.elsewhere[outer] = true;
            plan.elements.push((
                outer,
                Element {
                    primitive,
                    rotation,
                    style: Style::Fill,
                },
            ));
        } else {
            let traced: Vec<&BezPath> = region
                .outlines
                .iter()
                .map(|&i| &output.outlines[i])
                .collect();
            let drawn: Vec<BezPath> = region
                .outlines
                .iter()
                .zip(&region.shapes)
                .map(|(&i, shape)| match shape {
                    Some((primitive, rotation)) => {
                        primitive_path_like(primitive, *rotation, &output.outlines[i])
                    }
                    None => output.outlines[i].clone(),
                })
                .collect();
            if keeps_fill(&traced, &drawn) {
                for (&i, path) in region.outlines.iter().zip(drawn) {
                    plan.drawn[i] = Some(path);
                }
            }
        }
    }
    plan
}

fn stroked(band: Band) -> Element {
    Element {
        primitive: band.primitive,
        rotation: band.rotation,
        style: Style::Stroke {
            width: band.width,
            join: band.join,
        },
    }
}

impl Plan {
    fn elsewhere_all(&mut self, outlines: &[usize]) {
        for &i in outlines {
            self.elsewhere[i] = true;
        }
    }

    /// Every outline of the result as it is drawn: the primitive's path where one replaces it.
    #[must_use]
    pub fn drawn_outlines(&self, output: &Output) -> Vec<BezPath> {
        output
            .outlines
            .iter()
            .zip(&self.drawn)
            .map(|(traced, drawn)| drawn.clone().unwrap_or_else(|| traced.clone()))
            .collect()
    }
}

/// Whether `drawn` fills what `traced` fills (outline for outline, nonzero): the net signed area
/// changes by no more than the outlines' own area changes add up to.
fn keeps_fill(traced: &[&BezPath], drawn: &[BezPath]) -> bool {
    let area = |p: &BezPath| p.area();
    let before: f64 = traced.iter().map(|p| area(p)).sum();
    let after: f64 = drawn.iter().map(area).sum();
    let own: f64 = traced
        .iter()
        .zip(drawn)
        .map(|(t, d)| (area(t).abs() - area(d).abs()).abs())
        .sum();
    (after - before).abs() <= own + 1e-9 * before.abs().max(1.0)
}

/// [`primitive_path`] wound as `traced` is, so a hole stays a hole under the even-odd and
/// nonzero fills alike.
fn primitive_path_like(primitive: &Primitive, rotation: Angle, traced: &BezPath) -> BezPath {
    let path = primitive_path(primitive, rotation);
    if path.area() * traced.area() < 0.0 {
        path.reverse_subpaths()
    } else {
        path
    }
}

fn corner(c: Point, radius: f64, angle: f64) -> Point {
    Point::new(c.x + radius * angle.cos(), c.y + radius * angle.sin())
}

/// The closed outline `primitive` draws, turned by `rotation` about its frame's centre. Polygons
/// and stars have straight sides; a circle or ellipse is four cubics.
#[must_use]
pub fn primitive_path(primitive: &Primitive, rotation: Angle) -> BezPath {
    let (mut path, centre) = match *primitive {
        Primitive::Rect {
            origin,
            width,
            height,
            corner_radius,
        } => {
            let (w, h) = (width.as_mm(), height.as_mm());
            let r = corner_radius.as_mm().clamp(0.0, w.min(h) / 2.0);
            (
                rounded_rect(origin, w, h, r),
                Point::new(origin.x + w / 2.0, origin.y + h / 2.0),
            )
        }
        Primitive::Ellipse { centre, rx, ry } => (ellipse(centre, rx.as_mm(), ry.as_mm()), centre),
        Primitive::Polygon {
            centre,
            radius,
            angle,
            points,
        } => {
            let step = TAU / f64::from(points);
            let vertices = (0..points).map(|k| {
                corner(
                    centre,
                    radius.as_mm(),
                    angle.as_radians() + step * f64::from(k),
                )
            });
            (closed(vertices), centre)
        }
        Primitive::Star {
            centre,
            radius,
            angle,
            points,
            inner_ratio,
        } => {
            let step = TAU / f64::from(points);
            let (outer, inner) = (radius.as_mm(), radius.as_mm() * inner_ratio);
            let vertices = (0..points).flat_map(|k| {
                let tip = angle.as_radians() + step * f64::from(k);
                [
                    corner(centre, outer, tip),
                    corner(centre, inner, tip + step / 2.0),
                ]
            });
            (closed(vertices), centre)
        }
    };
    if rotation.as_radians() != 0.0 {
        path.apply_affine(Affine::rotate_about(rotation.as_radians(), centre));
    }
    path
}

fn closed(vertices: impl Iterator<Item = Point>) -> BezPath {
    let mut path = BezPath::new();
    for (k, v) in vertices.enumerate() {
        if k == 0 {
            path.move_to(v);
        } else {
            path.line_to(v);
        }
    }
    path.close_path();
    path
}

fn ellipse(centre: Point, rx: f64, ry: f64) -> BezPath {
    let c = centre;
    let (kx, ky) = (rx * KAPPA, ry * KAPPA);
    let mut path = BezPath::new();
    path.move_to((c.x + rx, c.y));
    path.curve_to((c.x + rx, c.y + ky), (c.x + kx, c.y + ry), (c.x, c.y + ry));
    path.curve_to((c.x - kx, c.y + ry), (c.x - rx, c.y + ky), (c.x - rx, c.y));
    path.curve_to((c.x - rx, c.y - ky), (c.x - kx, c.y - ry), (c.x, c.y - ry));
    path.curve_to((c.x + kx, c.y - ry), (c.x + rx, c.y - ky), (c.x + rx, c.y));
    path.close_path();
    path
}

fn rounded_rect(origin: Point, width: f64, height: f64, radius: f64) -> BezPath {
    let mut path = BezPath::new();
    let (left, top) = (origin.x, origin.y);
    let (right, bottom) = (left + width, top + height);
    if radius <= 0.0 {
        path.move_to(origin);
        path.line_to((right, top));
        path.line_to((right, bottom));
        path.line_to((left, bottom));
        path.close_path();
        return path;
    }
    let bend = radius * KAPPA;
    path.move_to((left + radius, top));
    path.line_to((right - radius, top));
    path.curve_to(
        (right - radius + bend, top),
        (right, top + radius - bend),
        (right, top + radius),
    );
    path.line_to((right, bottom - radius));
    path.curve_to(
        (right, bottom - radius + bend),
        (right - radius + bend, bottom),
        (right - radius, bottom),
    );
    path.line_to((left + radius, bottom));
    path.curve_to(
        (left + radius - bend, bottom),
        (left, bottom - radius + bend),
        (left, bottom - radius),
    );
    path.line_to((left, top + radius));
    path.curve_to(
        (left, top + radius - bend),
        (left + radius - bend, top),
        (left + radius, top),
    );
    path.close_path();
    path
}

#[cfg(test)]
mod tests {
    use super::*;
    use vectrace::shapes::ShapeRegion;

    fn rect_primitive(x: f64, y: f64, w: f64, h: f64) -> Primitive {
        Primitive::Rect {
            origin: Point::new(x, y),
            width: Mm::from_mm(w),
            height: Mm::from_mm(h),
            corner_radius: Mm::from_mm(0.0),
        }
    }

    fn square(x: f64, side: f64, clockwise: bool) -> BezPath {
        let c = [(x, x), (x + side, x), (x + side, x + side), (x, x + side)];
        let order: Vec<_> = if clockwise {
            c.to_vec()
        } else {
            c.iter().rev().copied().collect()
        };
        closed(order.into_iter().map(Point::from))
    }

    fn output(outlines: Vec<BezPath>, shapes: Vec<ShapeRegion>) -> Output {
        Output {
            status: vectrace::wire::Status::Traced,
            paths: outlines.len(),
            nodes: 0,
            specks_removed: 0,
            foreground_pixels: 0,
            covers_whole_image: false,
            outlines,
            palette: Vec::new(),
            layers: Vec::new(),
            joined: 0,
            texts: vectrace::wire::Texts::default(),
            shapes,
            unclipped: 0,
        }
    }

    fn region(outlines: &[usize], shapes: Vec<Option<(Primitive, Angle)>>) -> ShapeRegion {
        ShapeRegion {
            outlines: outlines.to_vec(),
            shapes,
            band: None,
            bands: Vec::new(),
        }
    }

    const NONE: Angle = Angle::from_radians(0.0);

    #[test]
    fn a_hole_free_region_becomes_an_element_and_leaves_the_compound_path() {
        let out = output(
            vec![square(0.0, 10.0, true), square(20.0, 5.0, true)],
            vec![region(
                &[0],
                vec![Some((rect_primitive(0.0, 0.0, 10.0, 10.0), NONE))],
            )],
        );
        let plan = plan(&out);
        assert_eq!(plan.elsewhere, [true, false]);
        assert_eq!(plan.elements.len(), 1);
        assert_eq!(plan.elements[0].0, 0);
        assert_eq!(plan.elements[0].1.style, Style::Fill);
        assert!(plan.drawn[1].is_none());
    }

    #[test]
    fn a_region_with_a_hole_stays_in_the_path_with_the_primitives_geometry() {
        let out = output(
            vec![square(0.0, 10.0, true), square(2.0, 6.0, false)],
            vec![region(
                &[0, 1],
                vec![Some((rect_primitive(0.0, 0.0, 10.0, 10.0), NONE)), None],
            )],
        );
        let plan = plan(&out);
        assert_eq!(plan.elsewhere, [false, false]);
        assert_eq!(plan.elements.len(), 0);
        assert!(plan.drawn[0].is_some(), "the outer is the primitive's");
        assert!(plan.drawn[1].is_some(), "the hole is kept, as drawn");
        let drawn = plan.drawn_outlines(&out);
        let net: f64 = drawn.iter().map(kurbo::Shape::area).sum();
        assert!((net - (100.0 - 36.0)).abs() < 1e-9, "net area {net}");
    }

    /// A hole drawn the outer's way round would fill the frame: that is not the same fill.
    #[test]
    fn a_hole_that_no_longer_cuts_is_not_the_same_fill() {
        let (outer, hole) = (square(0.0, 10.0, true), square(2.0, 6.0, false));
        let traced = [&outer, &hole];
        assert!(keeps_fill(&traced, &[outer.clone(), hole.clone()]));
        assert!(keeps_fill(
            &traced,
            &[square(0.0, 10.01, true), square(2.0, 5.99, false)]
        ));
        assert!(!keeps_fill(
            &traced,
            &[outer.clone(), square(2.0, 6.0, true)]
        ));
    }

    #[test]
    fn a_band_is_one_stroked_element_and_takes_its_outlines_out_of_the_path() {
        let band = Band {
            primitive: rect_primitive(1.0, 1.0, 8.0, 8.0),
            rotation: NONE,
            width: Mm::from_mm(2.0),
            join: Join::Bevel,
        };
        let mut r = region(&[0, 1], vec![None, None]);
        r.band = Some(band);
        let out = output(
            vec![square(0.0, 10.0, true), square(2.0, 6.0, false)],
            vec![r],
        );
        let plan = plan(&out);
        assert_eq!(plan.elsewhere, [true, true]);
        assert_eq!(
            plan.elements[0].1.style,
            Style::Stroke {
                width: Mm::from_mm(2.0),
                join: Join::Bevel
            }
        );
        // The preview keeps the traced ring.
        assert!(plan.drawn.iter().all(Option::is_none));
    }

    #[test]
    fn touching_bands_are_one_stroked_element_each() {
        let band = |x: f64| Band {
            primitive: rect_primitive(x, 1.0, 3.0, 8.0),
            rotation: NONE,
            width: Mm::from_mm(1.0),
            join: Join::Miter,
        };
        let mut r = region(&[0, 1, 2], vec![None, None, None]);
        r.bands = vec![band(1.0), band(5.0)];
        let out = output(
            vec![
                square(0.0, 10.0, true),
                square(1.0, 3.0, false),
                square(5.0, 3.0, false),
            ],
            vec![r],
        );
        let plan = plan(&out);
        assert_eq!(plan.elements.len(), 2);
        assert_eq!(plan.elsewhere, [true, true, true]);
    }

    #[test]
    fn an_index_outside_the_result_is_ignored() {
        let out = output(
            vec![square(0.0, 10.0, true)],
            vec![region(
                &[5],
                vec![Some((rect_primitive(0.0, 0.0, 1.0, 1.0), NONE))],
            )],
        );
        assert_eq!(plan(&out).elements.len(), 0);
    }

    #[test]
    fn a_rect_has_its_area_and_a_rotated_one_the_same() {
        let p = rect_primitive(2.0, 3.0, 10.0, 4.0);
        assert!((primitive_path(&p, NONE).area().abs() - 40.0).abs() < 1e-9);
        let turned = primitive_path(&p, Angle::from_radians(std::f64::consts::FRAC_PI_2));
        assert!((turned.area().abs() - 40.0).abs() < 1e-9);
        let b = turned.bounding_box();
        // Turned a quarter about its centre (7, 5): 4 wide and 10 high.
        assert!((b.width() - 4.0).abs() < 1e-9 && (b.height() - 10.0).abs() < 1e-9);
        assert!((b.center().x - 7.0).abs() < 1e-9 && (b.center().y - 5.0).abs() < 1e-9);
    }

    #[test]
    fn a_rounded_rect_has_w_h_minus_the_corner_loss() {
        let p = Primitive::Rect {
            origin: Point::new(0.0, 0.0),
            width: Mm::from_mm(20.0),
            height: Mm::from_mm(10.0),
            corner_radius: Mm::from_mm(3.0),
        };
        let expected = 200.0 - (4.0 - std::f64::consts::PI) * 9.0;
        assert!((primitive_path(&p, NONE).area().abs() - expected).abs() < 0.05);
    }

    #[test]
    fn an_ellipse_has_pi_rx_ry() {
        let p = Primitive::Ellipse {
            centre: Point::new(5.0, 5.0),
            rx: Mm::from_mm(4.0),
            ry: Mm::from_mm(2.0),
        };
        let expected = std::f64::consts::PI * 8.0;
        assert!((primitive_path(&p, NONE).area().abs() - expected).abs() < 0.01);
    }

    #[test]
    fn a_regular_polygon_and_a_star_have_straight_sides_and_the_right_area() {
        let hexagon = Primitive::Polygon {
            centre: Point::new(0.0, 0.0),
            radius: Mm::from_mm(10.0),
            angle: NONE,
            points: 6,
        };
        let area = primitive_path(&hexagon, NONE).area().abs();
        assert!((area - 1.5 * 3.0_f64.sqrt() * 100.0).abs() < 1e-9);
        let star = Primitive::Star {
            centre: Point::new(0.0, 0.0),
            radius: Mm::from_mm(10.0),
            angle: NONE,
            points: 5,
            inner_ratio: 0.5,
        };
        let path = primitive_path(&star, NONE);
        assert_eq!(path.elements().len(), 11, "ten corners and the close");
        assert!(
            path.elements()
                .iter()
                .all(|e| !matches!(e, kurbo::PathEl::CurveTo(..) | kurbo::PathEl::QuadTo(..)))
        );
    }
}
