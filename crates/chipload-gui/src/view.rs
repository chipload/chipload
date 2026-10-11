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

pub const BACKGROUND: Color32 = Color32::from_rgb(0x1f, 0x21, 0x24);
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
const AXIS_Z: Color32 = Color32::from_rgb(0x8a, 0xb4, 0xf8);
const SELECTED: Color32 = Color32::from_rgb(0xff, 0x8b, 0xcb);
/// The snap marker: a point of the drawing the cursor is on.
const SNAP: Color32 = Color32::from_rgb(0xfd, 0xd6, 0x63);
const TAB: Color32 = Color32::from_rgb(0xff, 0x9e, 0x40);
/// Contours of the operation selected in the operations list.
/// Copies of the part: outlines, and the toolpath's colour faded by this much.
const GHOST: Color32 = Color32::from_rgb(0x6b, 0x6f, 0x75);
const GHOST_FADE: f32 = 0.4;
const OPERATION: Color32 = Color32::from_rgb(0xc5, 0x8a, 0xf9);
/// Start markers of the focused operation's rings: as bright as the operation's outline.
const START: Color32 = OPERATION;
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

/// A point in 3D, mm (X, Y as drawn, Z as the program uses it).
pub type Point3 = (f64, f64, f64);

/// The most the 3D view tilts: just short of looking from the side, so a click still lands
/// on the stock top (docs/specs/3d-view.md section 3).
pub const MAX_TILT_RAD: f64 = 85.0 * std::f64::consts::PI / 180.0;

/// The camera: an orthographic view that turns about the vertical axis (`turn`), tilts
/// towards the viewer (`tilt`, 0 = straight down), then pans and zooms as the 2D view did.
/// With no turn and no tilt it is exactly the old 2D mapping: screen = offset + (x, −y) · scale.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct View {
    /// Pixels per millimetre.
    pub scale: f64,
    /// Screen position of the world origin.
    pub offset: (f64, f64),
    /// Rotation about the vertical axis, radians.
    pub turn: f64,
    /// 0 (top view) to MAX_TILT_RAD.
    pub tilt: f64,
    /// Stretch of Z in the 3D view (1 = true scale).
    pub z_scale: f64,
    /// Where 2D items (contours, selection, material left) sit: the stock top's Z.
    pub top_z: f64,
}

impl Default for View {
    fn default() -> Self {
        View {
            scale: 2.0,
            offset: (100.0, 400.0),
            turn: 0.0,
            tilt: 0.0,
            z_scale: 1.0,
            top_z: 0.0,
        }
    }
}

impl View {
    pub fn is_top(self) -> bool {
        self.turn == 0.0 && self.tilt == 0.0
    }

    /// (across, up) of a world point in camera units (mm), before pan and zoom.
    fn camera(self, (x, y, z): Point3) -> (f64, f64) {
        if self.is_top() {
            return (x, y); // exactly the 2D mapping, whatever Z is
        }
        let (sin_t, cos_t) = self.turn.sin_cos();
        let (sin_a, cos_a) = self.tilt.sin_cos();
        let across = x * cos_t - y * sin_t;
        let depth = x * sin_t + y * cos_t;
        let height = (z - self.top_z) * self.z_scale;
        (across, depth * cos_a + height * sin_a)
    }

    pub fn to_screen3(self, p: Point3) -> Pos2 {
        let (across, up) = self.camera(p);
        Pos2::new(
            (self.offset.0 + across * self.scale) as f32,
            (self.offset.1 - up * self.scale) as f32,
        )
    }

    /// A drawing point on the stock top.
    pub fn to_screen(self, p: Point) -> Pos2 {
        self.to_screen3((p.0, p.1, self.top_z))
    }

    /// The point on the stock top under a screen position.
    pub fn to_world(self, p: Pos2) -> Point {
        let across = (f64::from(p.x) - self.offset.0) / self.scale;
        let up = (self.offset.1 - f64::from(p.y)) / self.scale;
        if self.is_top() {
            return (across, up);
        }
        let (sin_t, cos_t) = self.turn.sin_cos();
        let depth = up / self.tilt.cos(); // on the stock top the height term is 0
        (
            across * cos_t + depth * sin_t,
            -across * sin_t + depth * cos_t,
        )
    }

    /// Show every point inside `screen` with a 5 % margin, in the current orientation.
    pub fn fit(&mut self, points: &[Point3], screen: Rect) {
        if points.is_empty() {
            return;
        }
        let projected: Vec<(f64, f64)> = points.iter().map(|&p| self.camera(p)).collect();
        let fold = |f: fn(f64, f64) -> f64, start: f64, pick: fn(&(f64, f64)) -> f64| {
            projected.iter().map(pick).fold(start, f)
        };
        let (x0, x1) = (
            fold(f64::min, f64::INFINITY, |p| p.0),
            fold(f64::max, f64::NEG_INFINITY, |p| p.0),
        );
        let (y0, y1) = (
            fold(f64::min, f64::INFINITY, |p| p.1),
            fold(f64::max, f64::NEG_INFINITY, |p| p.1),
        );
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

    /// Zoom by `factor`, keeping the stock-top point under `anchor` where it is.
    pub fn zoom_at(&mut self, anchor: Pos2, factor: f64) {
        let world = self.to_world(anchor);
        self.scale = (self.scale * factor).clamp(1e-4, 1e5);
        self.keep_at(world, anchor);
    }

    pub fn pan(&mut self, delta: Vec2) {
        self.offset.0 += f64::from(delta.x);
        self.offset.1 += f64::from(delta.y);
    }

    /// Turn and tilt by a mouse drag (pixels), keeping `pivot` where it is on screen.
    pub fn orbit(&mut self, delta: Vec2, pivot: Point) {
        let at = self.to_screen(pivot);
        let per_pixel = 0.01;
        self.turn = (self.turn - f64::from(delta.x) * per_pixel).rem_euclid(std::f64::consts::TAU);
        self.tilt = (self.tilt - f64::from(delta.y) * per_pixel).clamp(0.0, MAX_TILT_RAD);
        self.keep_at(pivot, at);
    }

    /// Set the orientation, keeping `pivot` where it is on screen.
    pub fn orient(&mut self, turn: f64, tilt: f64, pivot: Point) {
        let at = self.to_screen(pivot);
        self.turn = turn.rem_euclid(std::f64::consts::TAU);
        self.tilt = tilt.clamp(0.0, MAX_TILT_RAD);
        self.keep_at(pivot, at);
    }

    /// Move the offset so the stock-top point `world` lands on `screen`.
    fn keep_at(&mut self, world: Point, screen: Pos2) {
        let (across, up) = self.camera((world.0, world.1, self.top_z));
        self.offset = (
            f64::from(screen.x) - across * self.scale,
            f64::from(screen.y) + up * self.scale,
        );
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
    /// The same for the copies of the part (docs/specs/copies-grid.md), drawn faded.
    pub ghost_feeds: Vec<Vec<Point>>,
    pub rapids: Vec<(Point, Point)>,
    /// Where each rapid ends, in cut order.
    pub starts: Vec<Plunge>,
    /// (position, direction angle in radians, CCW from +X)
    pub arrows: Vec<(Point, f64)>,
    /// Every move in 3D for the 3D view, plunges and retracts included: (from, to, is a
    /// cutting move).
    pub lines3: Vec<(Point3, Point3, bool)>,
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
        let mut ghost = false;
        for (index, m) in program.moves.iter().enumerate() {
            let span = spans.iter().find(|s| s.moves.contains(&index));
            // Moves of a copy: faded, without plunge markers or arrows.
            let in_copy = span.is_some_and(|s| s.copy != 1);
            if in_copy != ghost {
                shape.flush(&mut run, ghost);
                ghost = in_copy;
            }
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
                    if !ghost {
                        shape.starts.push(Plunge {
                            at: next,
                            operation: span.map(|s| s.operation),
                        });
                    }
                    since_arrow = ARROW_SPACING_MM;
                    shape.flush(&mut run, ghost);
                }
                // The canvas shows the planned program, whose arcs are still chords; an arc
                // move (never shown today) is drawn as its chord.
                MoveKind::Feed | MoveKind::ArcCw | MoveKind::ArcCcw => {
                    if let Some(from) = here {
                        if run.last() != Some(&from) {
                            shape.flush(&mut run, ghost);
                            run = vec![from];
                        }
                        run.push(next);
                        let (dx, dy) = (next.0 - from.0, next.1 - from.1);
                        if since_arrow >= ARROW_SPACING_MM && !ghost {
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
        shape.flush(&mut run, ghost);
        shape.lines3 = lines_3d(program);
        shape
    }

    /// End a run of feed moves: kept if it draws anything, then started afresh.
    fn flush(&mut self, run: &mut Vec<Point>, ghost: bool) {
        let done = std::mem::take(run);
        if done.len() > 1 {
            if ghost {
                self.ghost_feeds.push(done);
            } else {
                self.feeds.push(done);
            }
        }
    }

    /// The Z range of the cutting moves: (highest, lowest).
    pub fn cut_z_range(&self) -> Option<(f64, f64)> {
        self.lines3
            .iter()
            .filter(|l| l.2)
            .flat_map(|(a, b, _)| [a.2, b.2])
            .fold(None, |range, z| match range {
                None => Some((z, z)),
                Some((hi, lo)) => Some((f64::max(hi, z), f64::min(lo, z))),
            })
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

/// Every move as a 3D segment, once the tool's position is known on all three axes.
fn lines_3d(program: &Program) -> Vec<(Point3, Point3, bool)> {
    let mut lines = Vec::new();
    let (mut x, mut y, mut z): (Option<f64>, Option<f64>, Option<f64>) = (None, None, None);
    for m in &program.moves {
        let from = match (x, y, z) {
            (Some(x), Some(y), Some(z)) => Some((x, y, z)),
            _ => None,
        };
        (x, y, z) = (m.x.or(x), m.y.or(y), m.z.or(z));
        if let (Some(from), Some(x), Some(y), Some(z)) = (from, x, y, z)
            && from != (x, y, z)
        {
            lines.push((from, (x, y, z), m.kind != MoveKind::Rapid));
        }
    }
    lines
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
    /// Handles of the selected drawn shape (top view only).
    pub handles: Vec<Point>,
    /// The point the cursor snapped to, marked with a diamond.
    pub snap_mark: Option<Point>,
    /// The G-code's X0 Y0, in drawing coordinates: where the axes are drawn.
    pub work_zero: Point,
    /// The stock's (top, bottom) Z, for the 3D view.
    pub stock_z: Option<(f64, f64)>,
    /// Where the copies of the part are, as shifts of the drawing (copy 1 excluded): drawn
    /// faded, never picked (docs/specs/copies-grid.md).
    pub copies: Vec<Point>,
    /// Where each ring of the last generated toolpath starts (docs/specs/helix-and-leads.md
    /// part A); the focused operation's are shown and can be dragged.
    pub start_markers: Vec<StartMarker>,
}

/// A ring's start that can be dragged along the ring.
#[derive(Debug, Clone, PartialEq)]
pub struct StartMarker {
    /// Index in the project's operations.
    pub operation: usize,
    /// The tool-centre ring, for moving the marker along it.
    pub ring: Vec<Point>,
    pub at: Point,
    /// The cutting direction there, radians CCW from +X.
    pub heading: f64,
    /// Index in the operation's `starts` of the point that set it; None: the default.
    pub stored: Option<usize>,
}

impl StartMarker {
    pub fn new(operation: usize, ring: Vec<Point>, stored: Option<usize>) -> Self {
        let at = ring.first().copied().unwrap_or((0.0, 0.0));
        let mut marker = StartMarker {
            operation,
            ring,
            at,
            heading: 0.0,
            stored,
        };
        marker.move_to(at);
        marker
    }

    /// Put the marker on the ring's point nearest to `to`, heading along the ring there.
    pub fn move_to(&mut self, to: Point) {
        let Some((i, at, _)) = chipload_toolpath::start::nearest_on_ring(&self.ring, to) else {
            return;
        };
        let n = self.ring.len();
        let (a, b) = (self.ring[i], self.ring[(i + 1) % n]);
        self.at = at;
        self.heading = (b.1 - a.1).atan2(b.0 - a.0);
    }
}

impl Scene {
    /// (min, max) of everything shown, including the origin axes.
    /// Every point that should be on screen after *Fit*, in 3D (2D items on `top_z`).
    pub fn fit_points(&self, top_z: f64) -> Vec<Point3> {
        if self.contours.is_empty() && self.stock.is_none() && self.toolpath.is_none() {
            return Vec::new();
        }
        let on_top = |p: Point| (p.0, p.1, top_z);
        let (zx, zy) = self.work_zero;
        let mut points: Vec<Point3> = vec![
            on_top((zx, zy)),
            on_top((zx + AXIS_LENGTH_MM, zy + AXIS_LENGTH_MM)),
        ];
        for shift in std::iter::once((0.0, 0.0)).chain(self.copies.iter().copied()) {
            points.extend(self.contours.iter().flat_map(|c| {
                c.points
                    .iter()
                    .map(move |&p| on_top((p.0 + shift.0, p.1 + shift.1)))
            }));
        }
        if let Some(((x0, y0), (x1, y1))) = self.stock {
            let (top, bottom) = self.stock_z.unwrap_or((top_z, top_z));
            for z in [top, bottom] {
                points.extend([(x0, y0, z), (x1, y0, z), (x1, y1, z), (x0, y1, z)]);
            }
        }
        if let Some(path) = &self.toolpath {
            points.extend(path.lines3.iter().flat_map(|(a, b, _)| [*a, *b]));
            points.extend(path.starts.iter().map(|s| on_top(s.at)));
        }
        points
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

        let flat = view.is_top();
        if let Some(((x0, y0), (x1, y1))) = self.stock {
            let corners = [(x0, y0), (x1, y0), (x1, y1), (x0, y1), (x0, y0)];
            dashed(screen(&corners), Stroke::new(1.0, STOCK));
            // In 3D the stock is a box: its bottom outline and the vertical edges.
            if !flat && let Some((top, bottom)) = self.stock_z {
                let at = |p: Point, z: f64| view.to_screen3((p.0, p.1, z));
                let bottom_ring: Vec<Pos2> = corners.iter().map(|&p| at(p, bottom)).collect();
                dashed(bottom_ring, Stroke::new(1.0, STOCK));
                for &p in &corners[..4] {
                    dashed(vec![at(p, top), at(p, bottom)], Stroke::new(1.0, STOCK));
                }
            }
        }
        // The axes mark the G-code zero (docs/specs/work-zero.md).
        let (zx, zy) = self.work_zero;
        let origin = view.to_screen((zx, zy));
        painter.line_segment(
            [origin, view.to_screen((zx + AXIS_LENGTH_MM, zy))],
            Stroke::new(2.0, AXIS_X),
        );
        painter.line_segment(
            [origin, view.to_screen((zx, zy + AXIS_LENGTH_MM))],
            Stroke::new(2.0, AXIS_Y),
        );
        if !flat {
            painter.line_segment(
                [
                    origin,
                    view.to_screen3((zx, zy, view.top_z + AXIS_LENGTH_MM / view.z_scale)),
                ],
                Stroke::new(2.0, AXIS_Z),
            );
        }
        self.paint_left(painter, view, highlight);

        for &(dx, dy) in &self.copies {
            for c in &self.contours {
                let moved: Vec<Point> = c.points.iter().map(|p| (p.0 + dx, p.1 + dy)).collect();
                painter.add(Shape::line(screen(&moved), Stroke::new(1.0, GHOST)));
            }
        }
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
            if flat {
                for &(a, b) in &path.rapids {
                    dashed(screen(&[a, b]), Stroke::new(1.0, RAPID));
                }
                for run in &path.ghost_feeds {
                    let faded = feed.gamma_multiply(GHOST_FADE);
                    painter.add(Shape::line(screen(run), Stroke::new(1.5, faded)));
                }
                for run in &path.feeds {
                    painter.add(Shape::line(screen(run), Stroke::new(2.0, feed)));
                }
                for &(at, angle) in &path.arrows {
                    painter.add(chevron(view.to_screen(at), angle, feed));
                }
            } else {
                paint_path_3d(painter, view, path, feed);
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
        // Tabs are marked in top view; in 3D the toolpath's lift over them shows them.
        for line in self.tabs.iter().filter(|_| flat) {
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
        for &p in self.handles.iter().filter(|_| flat) {
            let square = Rect::from_center_size(view.to_screen(p), Vec2::splat(7.0));
            painter.rect_filled(square, 0.0, Color32::from_gray(20));
            painter.rect_stroke(
                square,
                0.0,
                Stroke::new(1.5, SELECTED),
                eframe::egui::StrokeKind::Middle,
            );
        }
        for marker in &self.start_markers {
            if flat && highlight.operation == Some(marker.operation) {
                let at = view.to_screen(marker.at);
                painter.circle_filled(at, 6.0, BACKGROUND);
                painter.circle_stroke(at, 6.0, Stroke::new(2.0, START));
                if marker.stored.is_some() {
                    painter.circle_filled(at, 3.0, START);
                }
                let ahead = at
                    + Vec2::new(
                        (16.0 * marker.heading.cos()) as f32,
                        (-16.0 * marker.heading.sin()) as f32,
                    );
                painter.add(chevron(ahead, marker.heading, START));
            }
        }
        if let Some(p) = self.snap_mark {
            let c = view.to_screen(p);
            let r = 6.0;
            let diamond = vec![
                c + Vec2::new(0.0, -r),
                c + Vec2::new(r, 0.0),
                c + Vec2::new(0.0, r),
                c + Vec2::new(-r, 0.0),
            ];
            painter.add(Shape::closed_line(diamond, Stroke::new(1.5, SNAP)));
        }
    }
}

/// The toolpath in 3D: rapids dashed, cutting moves (plunges included) shaded from light at
/// the highest cut to dark at the deepest.
fn paint_path_3d(painter: &Painter, view: &View, path: &ToolpathShape, feed: Color32) {
    let (high, low) = path.cut_z_range().unwrap_or((0.0, 0.0));
    let span = (high - low).max(f64::EPSILON);
    for &(a, b, cutting) in &path.lines3 {
        let ends = [view.to_screen3(a), view.to_screen3(b)];
        if cutting {
            let depth = ((high - (a.2 + b.2) / 2.0) / span).clamp(0.0, 1.0) as f32;
            painter.line_segment(ends, Stroke::new(2.0, shade(feed, depth)));
        } else {
            painter.extend(Shape::dashed_line(
                &ends,
                Stroke::new(1.0, RAPID),
                DASH_PX,
                GAP_PX,
            ));
        }
    }
}

/// `color` at depth 0, darkening towards a third of its brightness at depth 1.
fn shade(color: Color32, depth: f32) -> Color32 {
    let k = 1.0 - 0.65 * depth;
    let scale = |c: u8| (f32::from(c) * k).round() as u8;
    Color32::from_rgb(scale(color.r()), scale(color.g()), scale(color.b()))
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
            ..View::default()
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
        view.fit(&[(-50.0, 0.0, 0.0), (150.0, 100.0, 0.0)], screen);
        for corner in [(-50.0, 0.0), (150.0, 100.0)] {
            assert!(screen.contains(view.to_screen(corner)));
        }
    }

    // -- 3D (docs/specs/3d-view.md)

    fn tilted() -> View {
        let mut view = View {
            scale: 4.0,
            offset: (100.0, 300.0),
            top_z: 0.0,
            ..View::default()
        };
        view.orient(-30f64.to_radians(), 55f64.to_radians(), (50.0, 50.0));
        view
    }

    #[test]
    fn top_view_projects_exactly_as_the_2d_view_did() {
        let view = View {
            scale: 4.0,
            offset: (100.0, 300.0),
            top_z: -3.0,
            ..View::default()
        };
        for p in [(10.0, 20.0), (-7.25, 1e3), (0.1, -0.3)] {
            let old = Pos2::new((100.0 + p.0 * 4.0) as f32, (300.0 - p.1 * 4.0) as f32);
            assert_eq!(view.to_screen(p), old);
            // Z doesn't matter looking straight down.
            assert_eq!(view.to_screen3((p.0, p.1, -12.0)), old);
        }
    }

    #[test]
    fn tilting_lifts_higher_points_and_picking_still_works() {
        let view = tilted();
        let low = view.to_screen3((40.0, 30.0, -10.0));
        let high = view.to_screen3((40.0, 30.0, 0.0));
        assert!(high.y < low.y, "higher is further up the screen");
        // A click lands on the stock top where that point is drawn.
        let p = (40.0, 30.0);
        let back = view.to_world(view.to_screen(p));
        assert!(
            (back.0 - p.0).abs() < 1e-3 && (back.1 - p.1).abs() < 1e-3,
            "{back:?}"
        );
        // Zoom keeps the stock-top point under the cursor, tilted too.
        let mut zoomed = view;
        let anchor = Pos2::new(321.0, 123.0);
        let before = zoomed.to_world(anchor);
        zoomed.zoom_at(anchor, 2.5);
        let after = zoomed.to_world(anchor);
        assert!((before.0 - after.0).abs() < 1e-6 && (before.1 - after.1).abs() < 1e-6);
    }

    #[test]
    fn orbiting_keeps_the_pivot_and_clamps_the_tilt() {
        let mut view = tilted();
        let pivot = (50.0, 50.0);
        let at = view.to_screen(pivot);
        view.orbit(Vec2::new(40.0, -1000.0), pivot);
        assert_eq!(view.tilt, MAX_TILT_RAD);
        assert!(view.to_screen(pivot).distance(at) < 1e-3);
        view.orbit(Vec2::new(-10_000.0, 1000.0), pivot);
        assert_eq!(view.tilt, 0.0);
        assert!((0.0..std::f64::consts::TAU).contains(&view.turn));
    }

    #[test]
    fn fit_in_3d_keeps_the_stock_box_on_screen() {
        let mut view = tilted();
        let screen = Rect::from_min_size(Pos2::new(0.0, 0.0), Vec2::new(800.0, 600.0));
        let mut corners = Vec::new();
        for z in [0.0, -18.0] {
            corners.extend([
                (0.0, 0.0, z),
                (300.0, 0.0, z),
                (300.0, 200.0, z),
                (0.0, 200.0, z),
            ]);
        }
        view.fit(&corners, screen);
        for &c in &corners {
            assert!(screen.contains(view.to_screen3(c)), "{c:?}");
        }
    }

    #[test]
    fn the_toolpath_keeps_its_depths_for_3d() {
        let program = Program {
            tool_id: 1,
            tool_diameter: 6.0,
            rpm: 18000.0,
            spindle: Spindle::Cw,
            safe_z: 5.0,
            moves: vec![
                Move::rapid_z(5.0),
                Move::rapid_xy((0.0, 0.0)),
                Move::feed_z(-2.0, 300.0),
                Move::feed_xy((10.0, 0.0), 1000.0),
                Move::feed_z(-4.0, 300.0),
                Move::feed_xy((0.0, 0.0), 1000.0),
            ],
        };
        let shape = ToolpathShape::new(&program, false, &[]);
        assert!(
            shape
                .lines3
                .contains(&((0.0, 0.0, 5.0), (0.0, 0.0, -2.0), true)),
            "plunge"
        );
        assert!(
            shape
                .lines3
                .contains(&((0.0, 0.0, -2.0), (10.0, 0.0, -2.0), true))
        );
        assert!(
            shape
                .lines3
                .contains(&((10.0, 0.0, -4.0), (0.0, 0.0, -4.0), true))
        );
        assert_eq!(shape.cut_z_range(), Some((5.0, -4.0)));
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
            ..View::default()
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
                copy: 1,
                moves: 0..6,
                tool_radius: 3.0,
            },
            OpSpan {
                operation: 3,
                copy: 1,
                moves: 6..9,
                tool_radius: 3.0,
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

/// What a legend entry looks like: drawn the way the canvas draws that thing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Swatch {
    Line(Color32, f32),
    Dashed(Color32),
    /// A filled patch at this opacity.
    Patch(Color32, f32),
    /// A plunge: the ring the canvas draws where the tool goes down.
    Plunge,
    /// A ring's start marker.
    Start,
}

/// What the canvas shows, explained: contours always; the toolpath and what goes with it once
/// generated; material left when shown; copies when there are some.
pub fn legend(scene: &Scene, material_left: bool) -> Vec<(Swatch, &'static str)> {
    let mut items = vec![
        (Swatch::Line(OUTER, 1.5), "outline"),
        (Swatch::Line(HOLE, 1.5), "hole"),
        (Swatch::Dashed(OPEN), "open contour (not cut)"),
        (Swatch::Line(SELECTED, 3.0), "selected"),
        (Swatch::Line(OPERATION, 6.0), "the operation's contours"),
        (Swatch::Dashed(STOCK), "stock"),
    ];
    if !scene.copies.is_empty() {
        items.push((Swatch::Line(GHOST, 1.0), "copies"));
    }
    if let Some(path) = &scene.toolpath {
        let feed = if path.rejected { FEED_REJECTED } else { FEED };
        items.push((Swatch::Line(feed, 2.0), "cutting move"));
        items.push((Swatch::Dashed(RAPID), "rapid, above the stock"));
        items.push((Swatch::Plunge, "where the tool goes down"));
        if !scene.tabs.is_empty() {
            items.push((Swatch::Line(TAB, 6.0), "tab"));
        }
        if !scene.start_markers.is_empty() {
            items.push((Swatch::Start, "ring start (drag to move)"));
        }
    }
    if material_left && !scene.left.is_empty() {
        items.push((
            Swatch::Patch(LEFT_UNREACHABLE, 0.4),
            "material the tool can't reach",
        ));
        if scene.left.iter().any(|l| l.kind == LeftKind::Missed) {
            items.push((Swatch::Patch(LEFT_MISSED, 0.6), "material missed"));
        }
    }
    items
}

/// Paint a swatch in `rect`, on the canvas background.
pub fn paint_swatch(painter: &Painter, rect: Rect, swatch: Swatch) {
    painter.rect_filled(rect, 2.0, BACKGROUND);
    let (left, right) = (rect.left_center(), rect.right_center());
    let (a, b) = (left + Vec2::new(3.0, 0.0), right - Vec2::new(3.0, 0.0));
    match swatch {
        Swatch::Line(color, width) => {
            painter.line_segment([a, b], Stroke::new(width.min(rect.height() - 4.0), color));
        }
        Swatch::Dashed(color) => {
            painter.extend(Shape::dashed_line(
                &[a, b],
                Stroke::new(1.5, color),
                4.0,
                3.0,
            ));
        }
        Swatch::Patch(color, alpha) => {
            painter.rect_filled(rect.shrink(2.0), 1.0, color.gamma_multiply(alpha));
        }
        Swatch::Plunge => {
            painter.circle_stroke(rect.center(), 5.0, Stroke::new(2.0, LABEL));
        }
        Swatch::Start => {
            painter.circle_stroke(rect.center(), 5.0, Stroke::new(2.0, START));
        }
    }
}
