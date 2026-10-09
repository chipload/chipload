// SPDX-License-Identifier: GPL-3.0-or-later
//! The drawing canvas: world (mm, Y up) to screen transform, what to draw, and picking.
//! No geometry decisions here: it only shows core results.

use std::collections::BTreeSet;

use chipload_core::Point;
use chipload_core::chain::Contour;
use chipload_toolpath::model::{MoveKind, Program};
use eframe::egui::{Align2, Color32, FontId, Painter, Pos2, Rect, Shape, Stroke, Vec2};

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

/// A program as drawable pieces. Feeds solid, rapids dashed, numbered start points, arrows in
/// the cutting direction.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ToolpathShape {
    /// Runs of consecutive XY feed moves.
    pub feeds: Vec<Vec<Point>>,
    pub rapids: Vec<(Point, Point)>,
    /// Where each rapid ends, in order (numbered on screen).
    pub starts: Vec<Point>,
    /// (position, direction angle in radians, CCW from +X)
    pub arrows: Vec<(Point, f64)>,
    pub rejected: bool,
}

impl ToolpathShape {
    pub fn new(program: &Program, rejected: bool) -> Self {
        let mut shape = ToolpathShape {
            rejected,
            ..Default::default()
        };
        let mut here: Option<Point> = None;
        let (mut x, mut y): (Option<f64>, Option<f64>) = (None, None);
        let mut since_arrow = ARROW_SPACING_MM; // so the first cut of every ring gets an arrow
        let mut run: Vec<Point> = Vec::new();
        for m in &program.moves {
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
                    shape.starts.push(next);
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

/// Everything on the canvas, in world coordinates.
#[derive(Debug, Clone, Default)]
pub struct Scene {
    pub contours: Vec<ContourShape>,
    pub stock: Option<(Point, Point)>,
    pub toolpath: Option<ToolpathShape>,
    pub tabs: Vec<Vec<Point>>,
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
            points.extend(path.starts.iter().copied());
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

    /// `selected`: the canvas selection; `operation`: contours of the operation selected in
    /// the operations list (drawn wider, under the selection colour).
    pub fn paint(
        &self,
        painter: &Painter,
        view: &View,
        selected: &BTreeSet<usize>,
        operation: &BTreeSet<usize>,
    ) {
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
            for (i, &p) in path.starts.iter().enumerate() {
                let dot = view.to_screen(p);
                painter.circle_stroke(dot, 4.0, Stroke::new(1.5, LABEL));
                painter.text(
                    dot + Vec2::new(6.0, -18.0),
                    Align2::LEFT_TOP,
                    (i + 1).to_string(),
                    FontId::proportional(12.0),
                    LABEL,
                );
            }
        }
        for line in &self.tabs {
            if line.len() > 1 {
                painter.add(Shape::line(screen(line), Stroke::new(6.0, TAB)));
            }
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
        let shape = ToolpathShape::new(&program, false);
        assert_eq!(shape.starts, [(0.0, 0.0), (50.0, 50.0)]);
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
}
