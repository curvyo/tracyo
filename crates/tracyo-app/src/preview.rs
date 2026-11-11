//! The outline preview: the traced outlines as a white casing and a blue line, in screen pixels.

use eframe::egui::{
    self, Color32, Mesh, Pos2, Rect, Shape, Stroke,
    epaint::{PathShape, Tessellator},
};
use kurbo::{Affine, BezPath, PathEl, Shape as _};

use crate::theme;

/// Picture pixels per millimetre of a result (96 dpi).
const PX_PER_MM: f64 = 96.0 / 25.4;
const CASING_WIDTH: f32 = 4.5;
const LINE_WIDTH: f32 = 1.5;
const FLATTEN_TOLERANCE: f64 = 0.05;

struct Cache {
    zoom: f32,
    ppp: f32,
    covered: Rect,
    casing: Mesh,
    line: Mesh,
}

/// The outlines of one result, with the meshes of the zoom they were last drawn at.
pub struct Preview {
    outlines: Vec<(BezPath, kurbo::Rect)>,
    cache: Option<Cache>,
}

impl Preview {
    /// The preview of `outlines` given in millimetres.
    #[must_use]
    pub fn new(outlines: &[BezPath]) -> Self {
        let scale = Affine::scale(PX_PER_MM);
        let outlines = outlines
            .iter()
            .map(|path| {
                let mut px = path.clone();
                px.apply_affine(scale);
                let bounds = px.bounding_box();
                (px, bounds)
            })
            .collect();
        Self {
            outlines,
            cache: None,
        }
    }

    fn build(&self, ctx: &egui::Context, zoom: f32, visible: Rect) -> Cache {
        let margin = visible.width().max(visible.height());
        let covered = visible.expand(margin);
        let mut tessellator = Tessellator::new(
            ctx.pixels_per_point(),
            ctx.tessellation_options(Clone::clone),
            [1, 1],
            Vec::new(),
        );
        let (mut casing, mut line) = (Mesh::default(), Mesh::default());
        let zoom64 = f64::from(zoom);
        let view = kurbo::Rect::new(
            f64::from(covered.left()) / zoom64,
            f64::from(covered.top()) / zoom64,
            f64::from(covered.right()) / zoom64,
            f64::from(covered.bottom()) / zoom64,
        );
        let scale = Affine::scale(zoom64);
        for (path, bounds) in &self.outlines {
            if bounds.x1 < view.x0
                || bounds.x0 > view.x1
                || bounds.y1 < view.y0
                || bounds.y0 > view.y1
            {
                continue;
            }
            let scaled = scale * path.clone();
            let mut points: Vec<Pos2> = Vec::new();
            let mut flush = |points: &mut Vec<Pos2>, closed: bool| {
                if points.len() >= 2 {
                    let pts = open_loop(std::mem::take(points), closed);
                    let closed = false;
                    tessellator.tessellate_path(
                        &PathShape {
                            points: pts.clone(),
                            closed,
                            fill: Color32::TRANSPARENT,
                            stroke: Stroke::new(CASING_WIDTH, Color32::WHITE).into(),
                        },
                        &mut casing,
                    );
                    tessellator.tessellate_path(
                        &PathShape {
                            points: pts,
                            closed,
                            fill: Color32::TRANSPARENT,
                            stroke: Stroke::new(LINE_WIDTH, theme::ACCENT).into(),
                        },
                        &mut line,
                    );
                }
                points.clear();
            };
            kurbo::flatten(
                scaled.elements().iter().copied(),
                FLATTEN_TOLERANCE,
                |el| match el {
                    PathEl::MoveTo(p) => {
                        flush(&mut points, false);
                        points.push(to_pos(p));
                    }
                    PathEl::LineTo(p) => points.push(to_pos(p)),
                    PathEl::ClosePath => {
                        if points.len() > 2 && points.first() == points.last() {
                            points.pop();
                        }
                        flush(&mut points, true);
                    }
                    PathEl::QuadTo(..) | PathEl::CurveTo(..) => {}
                },
            );
            flush(&mut points, false);
        }
        Cache {
            zoom,
            ppp: ctx.pixels_per_point(),
            covered,
            casing,
            line,
        }
    }

    /// Paints the outlines with the picture's top-left corner at `origin` on screen and `zoom`,
    /// clipped to `clip`; `alpha` is 1 for a current result, 0.5 for an old one.
    pub fn paint(
        &mut self,
        painter: &egui::Painter,
        origin: Pos2,
        zoom: f32,
        clip: Rect,
        alpha: f32,
    ) {
        if self.outlines.is_empty() {
            return;
        }
        let ctx = painter.ctx();
        let visible = clip.translate(-origin.to_vec2());
        let stale = self.cache.as_ref().is_none_or(|c| {
            (c.zoom - zoom).abs() > f32::EPSILON
                || (c.ppp - ctx.pixels_per_point()).abs() > f32::EPSILON
                || !c.covered.contains_rect(visible)
        });
        if stale {
            self.cache = Some(self.build(ctx, zoom, visible));
        }
        let Some(cache) = &self.cache else { return };
        let painter = painter.with_clip_rect(clip);
        for mesh in [&cache.casing, &cache.line] {
            let mut moved = mesh.clone();
            moved.translate(origin.to_vec2());
            if alpha < 1.0 {
                for v in &mut moved.vertices {
                    v.color = v.color.gamma_multiply(alpha);
                }
            }
            painter.add(Shape::mesh(moved));
        }
    }
}

/// A closed outline as an open polyline that starts and ends in the middle of its first segment:
/// epaint's closed strokes keep every miter, so a spike where an outline folds back (two colours
/// sharing a one-pixel sliver) grows without limit, while its open strokes cut such corners off.
fn open_loop(points: Vec<Pos2>, closed: bool) -> Vec<Pos2> {
    if !closed || points.len() < 3 {
        return points;
    }
    let middle = points[0].lerp(points[1], 0.5);
    let mut open = Vec::with_capacity(points.len() + 2);
    open.push(middle);
    open.extend_from_slice(&points[1..]);
    open.push(points[0]);
    open.push(middle);
    open
}

fn to_pos(p: kurbo::Point) -> Pos2 {
    #[allow(clippy::cast_possible_truncation)]
    Pos2::new(p.x as f32, p.y as f32)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A sliver of 0.2 px width and 100 px length: two colours sharing a one-pixel sliver make
    /// outlines that fold back like this, and a closed stroke's miter would grow without limit.
    fn sliver() -> BezPath {
        let mut path = BezPath::new();
        path.move_to((0.0, 0.0));
        path.line_to((100.0 / PX_PER_MM, 0.0));
        path.line_to((0.0, 0.2 / PX_PER_MM));
        path.close_path();
        path
    }

    #[test]
    fn a_folded_back_outline_has_no_spike_longer_than_its_line_width() {
        let preview = Preview::new(&[sliver()]);
        let ctx = egui::Context::default();
        let visible = Rect::from_min_max(Pos2::new(-50.0, -50.0), Pos2::new(150.0, 50.0));
        let cache = preview.build(&ctx, 1.0, visible);
        let reach = CASING_WIDTH;
        for mesh in [&cache.casing, &cache.line] {
            assert!(mesh.vertices.len() > 4);
            for v in &mesh.vertices {
                assert!(
                    v.pos.x > -reach && v.pos.x < 100.0 + reach && v.pos.y.abs() < reach,
                    "a vertex at {:?} lies far from the outline",
                    v.pos
                );
            }
        }
    }
}
