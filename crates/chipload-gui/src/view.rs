// SPDX-License-Identifier: GPL-3.0-or-later
//! The drawing canvas: world (mm, Y up) to screen transform, what to draw, and picking.
//! No geometry decisions here: it only shows core results.

use std::collections::BTreeSet;

use chipload_core::Point;
use chipload_core::chain::Contour;
use chipload_toolpath::OpSpan;
use chipload_toolpath::left::{LeftKind, MaterialLeft};
use chipload_toolpath::model::{MoveKind, Program};
use eframe::egui::{Align2, Color32, FontId, Mesh, Painter, Pos2, Rect, Shape, Stroke, Vec2};

const BACKGROUND: Color32 = Color32::from_rgb(0x1f, 0x21, 0x24);
const OUTER: Color32 = Color32::from_rgb(0xe8, 0xea, 0xed);
const HOLE: Color32 = Color32::from_rgb(0x8a, 0xb4, 0xf8);
const OPEN: Color32 = Color32::from_rgb(0xfb, 0xbc, 0x04);
const INVALID: Color32 = Color32::from_rgb(0xf2, 0x8b, 0x82);
const STOCK: Color32 = Color32::from_rgb(0x5f, 0x63, 0x68);
const FEED: Color32 = Color32::from_rgb(0x34, 0xa8, 0x53);
const FEED_REJECTED: Color32 = Color32::from_rgb(0xea, 0x43, 0x35);
const RAPID: Color32 = Color32::from_rgb(0x9a, 0xa0, 0xa6);
const LABEL: Color32 = Color32::from_rgb(0xfd, 0xd6, 0x63);
const AXIS_X: Color32 = Color32::from_rgb(0xf2, 0x8b, 0x82);
const AXIS_Y: Color32 = Color32::from_rgb(0x81, 0xc9, 0x95);
const SELECTED: Color32 = Color32::from_rgb(0xff, 0x8b, 0xcb);
const TAB: Color32 = Color32::from_rgb(0xff, 0x9e, 0x40);
/// Contours of the operation selected in the operations list.
const OPERATION: Color32 = Color32::from_rgb(0xc5, 0x8a, 0xf9);
/// Material the tool can't reach: faint, it's normal (docs/specs/material-left.md).
const LEFT_UNREACHABLE: Color32 = Color32::from_rgb(0xae, 0xcb, 0xfa);
/// Material the toolpath missed: stronger, it comes with a warning.
const LEFT_MISSED: Color32 = Color32::from_rgb(0xf2, 0x8b, 0x82);

/// Distance between direction arrows along the cutting path.
pub const ARROW_SPACING_MM: f64 = 40.0;
const AXIS_LENGTH_MM: f64 = 15.0;
/// How close (on screen) a click must be to a contour line.
pub const PICK_TOLERANCE_PX: f32 = 6.0;
const DASH_PX: f32 = 6.0;
const GAP_PX: f32 = 4.0;

/// Pan and zoom: screen = offset + (x, −y) · scale.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct View {
    /// Pixels per millimetre.
    pub scale: f64,
    /// Screen position of the world origin.
    pub offset: (f64, f64),
}

impl Default for View {
    fn default() -> Self {
        View {
            scale: 2.0,
            offset: (100.0, 400.0),
        }
    }
}

impl View {
    pub fn to_screen(self, p: Point) -> Pos2 {
        Pos2::new(
            (self.offset.0 + p.0 * self.scale) as f32,
            (self.offset.1 - p.1 * self.scale) as f32,
        )
    }

    pub fn to_world(self, p: Pos2) -> Point {
        (
            (f64::from(p.x) - self.offset.0) / self.scale,
            (self.offset.1 - f64::from(p.y)) / self.scale,
        )
    }

    /// Show `bounds` (min, max) inside `screen` with a 5 % margin.
    pub fn fit(&mut self, bounds: (Point, Point), screen: Rect) {
        let ((x0, y0), (x1, y1)) = bounds;
        let span = (x1 - x0).max(y1 - y0).max(1e-3);
        let margin = span * 0.05;
        let (w, h) = (x1 - x0 + 2.0 * margin, y1 - y0 + 2.0 * margin);
        self.scale = (f64::from(screen.width()) / w).min(f64::from(screen.height()) / h);
        let centre = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);
        let c = screen.center();
        self.offset = (
            f64::from(c.x) - centre.0 * self.scale,
            f64::from(c.y) + centre.1 * self.scale,
        );
    }

    /// Zoom by `factor`, keeping the world point under `anchor` where it is.
    pub fn zoom_at(&mut self, anchor: Pos2, factor: f64) {
        let world = self.to_world(anchor);
        self.scale = (self.scale * factor).clamp(1e-4, 1e5);
        self.offset = (
            f64::from(anchor.x) - world.0 * self.scale,
            f64::from(anchor.y) + world.1 * self.scale,
        );
    }

    pub fn pan(&mut self, delta: Vec2) {
        self.offset.0 += f64::from(delta.x);
        self.offset.1 += f64::from(delta.y);
    }
}

/// How a contour is shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Outer,
    Hole,
    Open,
    Invalid,
}

impl Kind {
    pub fn of(contour: &Contour) -> Kind {
        if !contour.closed {
            Kind::Open
        } else if contour.invalid_reason.is_some() {
            Kind::Invalid
        } else if contour.nesting_depth.unwrap_or(0) % 2 == 1 {
            Kind::Hole
        } else {
            Kind::Outer
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Kind::Outer => "outer",
            Kind::Hole => "hole",
            Kind::Open => "open",
            Kind::Invalid => "INVALID",
        }
    }
}

/// A contour ready to draw: its id, kind and polyline (closed ones repeat the start).
#[derive(Debug, Clone, PartialEq)]
pub struct ContourShape {
    pub id: usize,
    pub kind: Kind,
    pub points: Vec<Point>,
}

impl ContourShape {
    pub fn new(contour: &Contour) -> Self {
        let mut points = contour.points();
        if contour.closed
            && let Some(&first) = points.first()
        {
            points.push(first);
        }
        ContourShape {
            id: contour.id,
            kind: Kind::of(contour),
            points,
        }
    }
}

/// Where the tool goes down to cut: the end of a rapid move to a new XY position.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Plunge {
    pub at: Point,
    /// Index in `job.operations` (None for a rejected program shown without them).
    pub operation: Option<usize>,
}

/// A program as drawable pieces. Feeds solid, rapids dashed, plunge points (each operation
/// numbered once), arrows in the cutting direction.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ToolpathShape {
    /// Runs of consecutive XY feed moves.
    pub feeds: Vec<Vec<Point>>,
    pub rapids: Vec<(Point, Point)>,
    /// Where each rapid ends, in cut order.
    pub starts: Vec<Plunge>,
    /// (position, direction angle in radians, CCW from +X)
    pub arrows: Vec<(Point, f64)>,
    pub rejected: bool,
}

impl ToolpathShape {
    /// `spans` says which moves each operation made (empty: unknown).
    pub fn new(program: &Program, rejected: bool, spans: &[OpSpan]) -> Self {
        let mut shape = ToolpathShape {
            rejected,
            ..Default::default()
        };
        let mut here: Option<Point> = None;
        let (mut x, mut y): (Option<f64>, Option<f64>) = (None, None);
        let mut since_arrow = ARROW_SPACING_MM; // so the first cut of every ring gets an arrow
        let mut run: Vec<Point> = Vec::new();
        for (index, m) in program.moves.iter().enumerate() {
            let (nx, ny) = (m.x.or(x), m.y.or(y));
            let (Some(px), Some(py)) = (nx, ny) else {
                (x, y) = (nx, ny);
                continue;
            };
            let next = (px, py);
            if here == Some(next) {
                continue; // Z only (plunge, retract, tab lift): nothing to draw in plan view
            }
            match m.kind {
                MoveKind::Rapid => {
                    if let Some(from) = here {
                        shape.rapids.push((from, next));
                    }
                    let operation = spans
                        .iter()
                        .find(|s| s.moves.contains(&index))
                        .map(|s| s.operation);
                    shape.starts.push(Plunge {
                        at: next,
                        operation,
                    });
                    since_arrow = ARROW_SPACING_MM;
                    if run.len() > 1 {
                        shape.feeds.push(std::mem::take(&mut run));
                    }
                    run.clear();
                }
                MoveKind::Feed => {
                    if let Some(from) = here {
                        if run.last() != Some(&from) {
                            if run.len() > 1 {
                                shape.feeds.push(std::mem::take(&mut run));
                            }
                            run = vec![from];
                        }
                        run.push(next);
                        let (dx, dy) = (next.0 - from.0, next.1 - from.1);
                        if since_arrow >= ARROW_SPACING_MM {
                            let mid = ((from.0 + next.0) / 2.0, (from.1 + next.1) / 2.0);
                            shape.arrows.push((mid, dy.atan2(dx)));
                            since_arrow = 0.0;
                        }
                        since_arrow += dx.hypot(dy);
                    }
                }
            }
            here = Some(next);
            (x, y) = (nx, ny);
        }
        if run.len() > 1 {
            shape.feeds.push(run);
        }
        shape
    }
}

/// One piece of material an operation leaves, ready to fill.
#[derive(Debug, Clone)]
pub struct LeftShape {
    pub operation: usize,
    pub kind: LeftKind,
    /// Width and height of its bounding box (mm).
    pub size: (f64, f64),
    rings: Vec<Vec<Point>>,
    /// The even-odd fill as trapezoids, world coordinates.
    trapezoids: Vec<[Point; 4]>,
}

impl LeftShape {
    pub fn new(left: &MaterialLeft) -> Self {
        let ((x0, y0), (x1, y1)) = left.bounds();
        LeftShape {
            operation: left.operation,
            kind: left.kind,
            size: (x1 - x0, y1 - y0),
            trapezoids: trapezoids(&left.rings),
            rings: left.rings.clone(),
        }
    }

    /// Even-odd point in polygon, as it is filled.
    pub fn contains(&self, p: Point) -> bool {
        let mut inside = false;
        for ring in &self.rings {
            let n = ring.len();
            for i in 0..n {
                let (a, b) = (ring[i], ring[(i + 1) % n]);
                if (a.1 > p.1) != (b.1 > p.1) && p.0 < a.0 + (p.1 - a.1) / (b.1 - a.1) * (b.0 - a.0)
                {
                    inside = !inside;
                }
            }
        }
        inside
    }
}

/// Even-odd fill of closed rings as trapezoids [bottom left, bottom right, top right, top
/// left]. egui only fills convex polygons, and these pieces aren't. Between two successive
/// vertex heights no edge starts or ends, so the edges crossing that band pair up left to
/// right into trapezoids.
pub fn trapezoids(rings: &[Vec<Point>]) -> Vec<[Point; 4]> {
    let edges: Vec<(Point, Point)> = rings
        .iter()
        .flat_map(|ring| {
            let n = ring.len();
            (0..n).map(move |i| (ring[i], ring[(i + 1) % n]))
        })
        .filter(|(a, b)| a.1 != b.1)
        .collect();
    let mut ys: Vec<f64> = rings.iter().flatten().map(|p| p.1).collect();
    ys.sort_by(f64::total_cmp);
    ys.dedup();
    let x_at = |(a, b): (Point, Point), y: f64| a.0 + (y - a.1) / (b.1 - a.1) * (b.0 - a.0);
    let mut out = Vec::new();
    for band in ys.windows(2) {
        let (y0, y1) = (band[0], band[1]);
        let mid = (y0 + y1) / 2.0;
        let mut crossing: Vec<(f64, f64, f64)> = edges
            .iter()
            .filter(|(a, b)| (a.1 > mid) != (b.1 > mid))
            .map(|&e| (x_at(e, mid), x_at(e, y0), x_at(e, y1)))
            .collect();
        crossing.sort_by(|a, b| a.0.total_cmp(&b.0));
        for [left, right] in crossing.as_chunks::<2>().0 {
            out.push([(left.1, y0), (right.1, y0), (right.2, y1), (left.2, y1)]);
        }
    }
    out
}

/// What to emphasise when painting.
#[derive(Debug, Clone, Default)]
pub struct Highlight {
    /// The canvas selection.
    pub selected: BTreeSet<usize>,
    /// Contours of the operation selected in the operations list (drawn wider, under the
    /// selection colour).
    pub operation_contours: BTreeSet<usize>,
    /// That operation's index, whose material left is emphasised.
    pub operation: Option<usize>,
    pub show_material_left: bool,
}

/// Everything on the canvas, in world coordinates.
#[derive(Debug, Clone, Default)]
pub struct Scene {
    pub contours: Vec<ContourShape>,
    pub stock: Option<(Point, Point)>,
    pub toolpath: Option<ToolpathShape>,
    pub tabs: Vec<Vec<Point>>,
    /// Missed pieces after unreachable ones, so they are painted on top.
    pub left: Vec<LeftShape>,
    /// The shape being drawn, as it would be with the cursor's point.
    pub preview: Vec<Vec<Point>>,
    /// The points clicked so far while drawing.
    pub sketch_points: Vec<Point>,
}

impl Scene {
    /// (min, max) of everything shown, including the origin axes.
    pub fn bounds(&self) -> Option<(Point, Point)> {
        let mut points: Vec<Point> = vec![(0.0, 0.0), (AXIS_LENGTH_MM, AXIS_LENGTH_MM)];
        points.extend(self.contours.iter().flat_map(|c| c.points.iter().copied()));
        if let Some((lo, hi)) = self.stock {
            points.extend([lo, hi]);
        }
        if let Some(path) = &self.toolpath {
            points.extend(path.feeds.iter().flatten().copied());
            points.extend(path.starts.iter().map(|s| s.at));
        }
        if self.contours.is_empty() && self.stock.is_none() && self.toolpath.is_none() {
            return None;
        }
        Some(points.iter().fold(
            (
                (f64::INFINITY, f64::INFINITY),
                (f64::NEG_INFINITY, f64::NEG_INFINITY),
            ),
            |((x0, y0), (x1, y1)), p| ((x0.min(p.0), y0.min(p.1)), (x1.max(p.0), y1.max(p.1))),
        ))
    }

    /// The contour nearest to a screen position, within the pick tolerance.
    pub fn pick(&self, view: &View, at: Pos2) -> Option<usize> {
        self.contours
            .iter()
            .filter_map(|c| {
                let screen: Vec<Pos2> = c.points.iter().map(|&p| view.to_screen(p)).collect();
                let d = screen
                    .windows(2)
                    .map(|w| segment_distance(at, w[0], w[1]))
                    .fold(f32::INFINITY, f32::min);
                (d <= PICK_TOLERANCE_PX).then_some((d, c.id))
            })
            .min_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)))
            .map(|(_, id)| id)
    }

    /// Material left under the cursor: missed before unreachable, the highlighted operation
    /// first.
    pub fn left_at(&self, p: Point, operation: Option<usize>) -> Option<&LeftShape> {
        self.left
            .iter()
            .filter(|l| l.contains(p))
            .min_by_key(|l| (l.kind != LeftKind::Missed, Some(l.operation) != operation))
    }

    /// Plunges (indices into the toolpath's starts) within pick distance of a screen point.
    pub fn plunges_at(&self, view: &View, at: Pos2) -> Vec<usize> {
        let Some(path) = &self.toolpath else {
            return Vec::new();
        };
        path.starts
            .iter()
            .enumerate()
            .filter(|(_, s)| view.to_screen(s.at).distance(at) <= PICK_TOLERANCE_PX)
            .map(|(i, _)| i)
            .collect()
    }

    /// Material left: under everything else, so it never hides a line.
    fn paint_left(&self, painter: &Painter, view: &View, highlight: &Highlight) {
        if !highlight.show_material_left {
            return;
        }
        let mut mesh = Mesh::default();
        for piece in &self.left {
            let (base, normal) = match piece.kind {
                LeftKind::Unreachable => (LEFT_UNREACHABLE, 0.22),
                LeftKind::Missed => (LEFT_MISSED, 0.55),
            };
            let factor = match highlight.operation {
                None => normal,
                Some(op) if op == piece.operation => normal * 1.8,
                Some(_) => normal * 0.4,
            };
            let color = base.gamma_multiply(factor);
            for quad in &piece.trapezoids {
                let first = mesh.vertices.len() as u32;
                for &p in quad {
                    mesh.colored_vertex(view.to_screen(p), color);
                }
                mesh.add_triangle(first, first + 1, first + 2);
                mesh.add_triangle(first, first + 2, first + 3);
            }
        }
        if !mesh.is_empty() {
            painter.add(Shape::mesh(mesh));
        }
    }

    pub fn paint(&self, painter: &Painter, view: &View, highlight: &Highlight) {
        let (selected, operation) = (&highlight.selected, &highlight.operation_contours);
        painter.rect_filled(painter.clip_rect(), 0.0, BACKGROUND);
        let screen =
            |pts: &[Point]| -> Vec<Pos2> { pts.iter().map(|&p| view.to_screen(p)).collect() };
        let dashed = |pts: Vec<Pos2>, stroke: Stroke| {
            painter.extend(Shape::dashed_line(&pts, stroke, DASH_PX, GAP_PX));
        };

        if let Some(((x0, y0), (x1, y1))) = self.stock {
            let corners = [(x0, y0), (x1, y0), (x1, y1), (x0, y1), (x0, y0)];
            dashed(screen(&corners), Stroke::new(1.0, STOCK));
        }
        let origin = view.to_screen((0.0, 0.0));
        painter.line_segment(
            [origin, view.to_screen((AXIS_LENGTH_MM, 0.0))],
            Stroke::new(2.0, AXIS_X),
        );
        painter.line_segment(
            [origin, view.to_screen((0.0, AXIS_LENGTH_MM))],
            Stroke::new(2.0, AXIS_Y),
        );
        self.paint_left(painter, view, highlight);

        for c in &self.contours {
            let pts = screen(&c.points);
            let is_selected = selected.contains(&c.id);
            if operation.contains(&c.id) {
                painter.add(Shape::line(pts.clone(), Stroke::new(6.0, OPERATION)));
            }
            let (color, width) = match (is_selected, c.kind) {
                (true, _) => (SELECTED, 3.0),
                (false, Kind::Outer) => (OUTER, 1.5),
                (false, Kind::Hole) => (HOLE, 1.5),
                (false, Kind::Open) => (OPEN, 1.5),
                (false, Kind::Invalid) => (INVALID, 2.5),
            };
            if c.kind == Kind::Open && !is_selected {
                dashed(pts, Stroke::new(width, color));
            } else {
                painter.add(Shape::line(pts, Stroke::new(width, color)));
            }
        }

        if let Some(path) = &self.toolpath {
            let feed = if path.rejected { FEED_REJECTED } else { FEED };
            for &(a, b) in &path.rapids {
                dashed(screen(&[a, b]), Stroke::new(1.0, RAPID));
            }
            for run in &path.feeds {
                painter.add(Shape::line(screen(run), Stroke::new(2.0, feed)));
            }
            for &(at, angle) in &path.arrows {
                painter.add(chevron(view.to_screen(at), angle, feed));
            }
            // Each operation is numbered once, at its first plunge (its number in the
            // operations list); other plunges get a small marker, once per position.
            let mut labelled: BTreeSet<Option<usize>> = BTreeSet::new();
            let mut marked: Vec<Point> = Vec::new();
            for plunge in &path.starts {
                let dot = view.to_screen(plunge.at);
                if plunge.operation.is_some() && labelled.insert(plunge.operation) {
                    painter.circle_stroke(dot, 5.0, Stroke::new(2.0, LABEL));
                    painter.text(
                        dot + Vec2::new(7.0, -19.0),
                        Align2::LEFT_TOP,
                        plunge.operation.map_or(0, |op| op + 1).to_string(),
                        FontId::proportional(14.0),
                        LABEL,
                    );
                } else if !marked.contains(&plunge.at) {
                    painter.circle_stroke(dot, 3.0, Stroke::new(1.0, LABEL));
                }
                marked.push(plunge.at);
            }
        }
        for line in &self.tabs {
            if line.len() > 1 {
                painter.add(Shape::line(screen(line), Stroke::new(6.0, TAB)));
            }
        }
        for line in &self.preview {
            if line.len() > 1 {
                dashed(screen(line), Stroke::new(1.5, SELECTED));
            }
        }
        for &p in &self.sketch_points {
            painter.circle_stroke(view.to_screen(p), 4.0, Stroke::new(1.5, SELECTED));
        }
    }
}

/// A ">" pointing along `angle` (world, CCW from +X); screen Y is down, so it flips.
fn chevron(at: Pos2, angle: f64, color: Color32) -> Shape {
    let (s, c) = (-angle).sin_cos();
    let rotate = |x: f64, y: f64| at + Vec2::new((x * c - y * s) as f32, (x * s + y * c) as f32);
    Shape::line(
        vec![rotate(-7.0, -5.0), rotate(0.0, 0.0), rotate(-7.0, 5.0)],
        Stroke::new(2.0, color),
    )
}

fn segment_distance(p: Pos2, a: Pos2, b: Pos2) -> f32 {
    let ab = b - a;
    let len2 = ab.length_sq();
    let t = if len2 == 0.0 {
        0.0
    } else {
        ((p - a).dot(ab) / len2).clamp(0.0, 1.0)
    };
    (p - (a + ab * t)).length()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chipload_job::Spindle;
    use chipload_toolpath::model::Move;

    #[test]
    fn world_and_screen_round_trip_with_y_up() {
        let view = View {
            scale: 4.0,
            offset: (100.0, 300.0),
        };
        let p = view.to_screen((10.0, 20.0));
        assert_eq!(p, Pos2::new(140.0, 220.0)); // up in the world is up on screen
        let back = view.to_world(p);
        assert!((back.0 - 10.0).abs() < 1e-9 && (back.1 - 20.0).abs() < 1e-9);
    }

    #[test]
    fn zoom_keeps_the_point_under_the_cursor() {
        let mut view = View::default();
        let anchor = Pos2::new(321.0, 123.0);
        let before = view.to_world(anchor);
        view.zoom_at(anchor, 3.0);
        let after = view.to_world(anchor);
        assert!((before.0 - after.0).abs() < 1e-9 && (before.1 - after.1).abs() < 1e-9);
    }

    #[test]
    fn fit_puts_the_bounds_inside_the_screen() {
        let mut view = View::default();
        let screen = Rect::from_min_size(Pos2::new(0.0, 0.0), Vec2::new(800.0, 600.0));
        view.fit(((-50.0, 0.0), (150.0, 100.0)), screen);
        for corner in [(-50.0, 0.0), (150.0, 100.0)] {
            assert!(screen.contains(view.to_screen(corner)));
        }
    }

    fn square(id: usize, x: f64) -> ContourShape {
        ContourShape {
            id,
            kind: Kind::Outer,
            points: vec![
                (x, 0.0),
                (x + 10.0, 0.0),
                (x + 10.0, 10.0),
                (x, 10.0),
                (x, 0.0),
            ],
        }
    }

    #[test]
    fn picking_takes_the_nearest_contour_within_tolerance() {
        let scene = Scene {
            contours: vec![square(1, 0.0), square(2, 12.0)],
            ..Default::default()
        };
        let view = View {
            scale: 10.0,
            offset: (0.0, 500.0),
        };
        // 0.3 mm = 3 px right of square 1's right edge, 1.7 mm from square 2.
        assert_eq!(scene.pick(&view, view.to_screen((10.3, 5.0))), Some(1));
        assert_eq!(scene.pick(&view, view.to_screen((11.8, 5.0))), Some(2));
        // In the middle of square 1: farther than 6 px from any line.
        assert_eq!(scene.pick(&view, view.to_screen((5.0, 5.0))), None);
    }

    #[test]
    fn toolpath_pieces() {
        let program = Program {
            tool_id: 1,
            tool_diameter: 6.0,
            rpm: 18000.0,
            spindle: Spindle::Cw,
            safe_z: 5.0,
            moves: vec![
                Move::rapid_z(5.0),
                Move::rapid_xy((0.0, 0.0)),
                Move::feed_z(-1.0, 300.0),
                Move::feed_xy((100.0, 0.0), 1000.0),
                Move::feed_xy((100.0, 10.0), 1000.0),
                Move::rapid_z(5.0),
                Move::rapid_xy((50.0, 50.0)),
                Move::feed_z(-1.0, 300.0),
                Move::feed_xy((60.0, 50.0), 1000.0),
            ],
        };
        let spans = [
            OpSpan {
                operation: 0,
                moves: 0..6,
            },
            OpSpan {
                operation: 3,
                moves: 6..9,
            },
        ];
        let shape = ToolpathShape::new(&program, false, &spans);
        let plunge = |at, operation| Plunge { at, operation };
        assert_eq!(
            shape.starts,
            [plunge((0.0, 0.0), Some(0)), plunge((50.0, 50.0), Some(3))]
        );
        assert_eq!(shape.rapids, [((100.0, 10.0), (50.0, 50.0))]);
        assert_eq!(
            shape.feeds,
            [
                vec![(0.0, 0.0), (100.0, 0.0), (100.0, 10.0)],
                vec![(50.0, 50.0), (60.0, 50.0)]
            ]
        );
        // One arrow at the first cut of each ring; the 10 mm stretch comes 100 mm later,
        // past the spacing, so it gets one too.
        assert_eq!(shape.arrows.len(), 3);
        assert_eq!(shape.arrows[0], ((50.0, 0.0), 0.0));
    }

    fn polygon_area(ring: &[Point]) -> f64 {
        let n = ring.len();
        (0..n)
            .map(|i| {
                let (a, b) = (ring[i], ring[(i + 1) % n]);
                a.0 * b.1 - b.0 * a.1
            })
            .sum::<f64>()
            / 2.0
    }

    fn trapezoid_area(rings: &[Vec<Point>]) -> f64 {
        trapezoids(rings)
            .iter()
            .map(|q| polygon_area(q).abs())
            .sum()
    }

    #[test]
    fn concave_pieces_fill_exactly() {
        let l = vec![
            (0.0, 0.0),
            (50.0, 0.0),
            (50.0, 20.0),
            (20.0, 20.0),
            (20.0, 40.0),
            (0.0, 40.0),
        ];
        assert!((trapezoid_area(std::slice::from_ref(&l)) - polygon_area(&l)).abs() < 1e-9);
        // A corner fillet: two walls and a concave arc, flattened.
        let mut fillet = vec![(0.0, 0.0), (3.0, 0.0)];
        fillet.extend((1..20).map(|i| {
            let a =
                -std::f64::consts::FRAC_PI_2 - f64::from(i) * std::f64::consts::FRAC_PI_2 / 20.0;
            (3.0 + 3.0 * a.cos(), 3.0 + 3.0 * a.sin())
        }));
        fillet.push((0.0, 3.0));
        let area = polygon_area(&fillet).abs();
        assert!((trapezoid_area(&[fillet]) - area).abs() < 1e-9);
    }

    #[test]
    fn holes_and_touching_squares_fill_even_odd() {
        let square =
            |x0: f64, y0: f64, s: f64| vec![(x0, y0), (x0 + s, y0), (x0 + s, y0 + s), (x0, y0 + s)];
        // A 10 x 10 outline with a 4 x 4 hole.
        let rings = [square(0.0, 0.0, 10.0), square(3.0, 3.0, 4.0)];
        assert!((trapezoid_area(&rings) - 84.0).abs() < 1e-9);
        let piece = LeftShape::new(&MaterialLeft {
            operation: 0,
            kind: LeftKind::Unreachable,
            rings: rings.to_vec(),
        });
        assert!(piece.contains((1.0, 1.0)) && !piece.contains((5.0, 5.0)));
        // Quadtree cells of missed material share edges.
        let cells = [
            square(0.0, 0.0, 1.0),
            square(1.0, 0.0, 1.0),
            square(0.0, 1.0, 0.5),
        ];
        assert!((trapezoid_area(&cells) - 2.25).abs() < 1e-9);
    }
}
