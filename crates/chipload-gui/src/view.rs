// SPDX-License-Identifier: GPL-3.0-or-later
//! The drawing canvas: world (mm, Y up) to screen transform, what to draw, and picking.
//! No geometry decisions here: it only shows core results.

use std::collections::BTreeSet;

use chipload_core::Point;
use chipload_core::chain::Contour;
use chipload_core::order::Ordered;
use chipload_toolpath::OpSpan;
use chipload_toolpath::model::{MoveKind, Program};
use eframe::egui::{Align2, Color32, FontId, Mesh, Painter, Pos2, Rect, Shape, Stroke, Vec2};

/// The canvas's colours, one set for each theme (docs/specs/browser-and-layout.md §6b).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Palette {
    /// The background: a gradient from the top colour to the bottom one (`background`).
    pub background_top: Color32,
    pub background: Color32,
    /// The grid's lines: every step, and every tenth.
    pub grid_minor: Color32,
    pub grid_major: Color32,
    pub outer: Color32,
    pub hole: Color32,
    pub open: Color32,
    pub invalid: Color32,
    pub stock: Color32,
    pub feed: Color32,
    pub feed_rejected: Color32,
    pub rapid: Color32,
    pub label: Color32,
    pub axis_x: Color32,
    pub axis_y: Color32,
    pub axis_z: Color32,
    pub selected: Color32,
    /// The snap marker: a point of the drawing the cursor is on.
    pub snap: Color32,
    /// View → Measure's line and text, and the box behind the text.
    pub measure: Color32,
    pub measure_back: Color32,
    /// Middle handles of a polyline's segments: they insert a point or bend the segment,
    /// unlike the square point handles, which move a point.
    pub middle_handle: Color32,
    /// Inside the handles.
    pub handle_fill: Color32,
    pub tab: Color32,
    /// Copies of the part: outlines, and the toolpath's colour faded by GHOST_FADE.
    pub ghost: Color32,
    /// Contours of the operation selected in the Browser, and its rings' start markers.
    pub operation: Color32,
    /// The playback's tool.
    pub tool_mark: Color32,
    /// What the highest cut in 3D is shaded towards (the deepest is the plain colour).
    pub shade_toward: Color32,
}

impl Palette {
    /// The dark canvas.
    pub const DARK: Palette = Palette {
        background_top: Color32::from_rgb(0x1f, 0x21, 0x24),
        background: Color32::from_rgb(0x16, 0x17, 0x1a),
        grid_minor: Color32::from_rgba_premultiplied(9, 9, 9, 9),
        grid_major: Color32::from_rgba_premultiplied(22, 22, 22, 22),
        outer: Color32::from_rgb(0xe8, 0xea, 0xed),
        hole: Color32::from_rgb(0x8a, 0xb4, 0xf8),
        open: Color32::from_rgb(0xfb, 0xbc, 0x04),
        invalid: Color32::from_rgb(0xf2, 0x8b, 0x82),
        stock: Color32::from_rgb(0x80, 0x86, 0x8b),
        feed: Color32::from_rgb(0x34, 0xa8, 0x53),
        feed_rejected: Color32::from_rgb(0xea, 0x43, 0x35),
        rapid: Color32::from_rgb(0x9a, 0xa0, 0xa6),
        label: Color32::from_rgb(0xfd, 0xd6, 0x63),
        axis_x: Color32::from_rgb(0xf2, 0x8b, 0x82),
        axis_y: Color32::from_rgb(0x81, 0xc9, 0x95),
        axis_z: Color32::from_rgb(0x8a, 0xb4, 0xf8),
        selected: Color32::from_rgb(0xff, 0x8b, 0xcb),
        snap: Color32::from_rgb(0xfd, 0xd6, 0x63),
        measure: Color32::from_rgb(0x7f, 0xe8, 0xff),
        measure_back: Color32::from_black_alpha(190),
        middle_handle: Color32::from_rgb(0x78, 0xd9, 0xec),
        handle_fill: Color32::from_gray(20),
        tab: Color32::from_rgb(0xff, 0x9e, 0x40),
        ghost: Color32::from_rgb(0x74, 0x78, 0x7e),
        operation: Color32::from_rgb(0xc5, 0x8a, 0xf9),
        tool_mark: Color32::from_rgb(0xfd, 0xd6, 0x63),
        shade_toward: Color32::WHITE,
    };

    /// The light canvas: the same meanings in darker inks.
    pub const LIGHT: Palette = Palette {
        background_top: Color32::from_rgb(0xf3, 0xf5, 0xf7),
        background: Color32::from_rgb(0xdc, 0xe0, 0xe5),
        grid_minor: Color32::from_rgba_premultiplied(0, 0, 0, 14),
        grid_major: Color32::from_rgba_premultiplied(0, 0, 0, 34),
        outer: Color32::from_rgb(0x20, 0x21, 0x24),
        hole: Color32::from_rgb(0x1a, 0x56, 0xc4),
        open: Color32::from_rgb(0x8a, 0x4b, 0x00),
        invalid: Color32::from_rgb(0xb3, 0x26, 0x1e),
        stock: Color32::from_rgb(0x5f, 0x63, 0x68),
        feed: Color32::from_rgb(0x0a, 0x4f, 0x22),
        feed_rejected: Color32::from_rgb(0xb3, 0x26, 0x1e),
        rapid: Color32::from_rgb(0x3c, 0x40, 0x43),
        label: Color32::from_rgb(0x8a, 0x4b, 0x00),
        axis_x: Color32::from_rgb(0xc5, 0x22, 0x1f),
        axis_y: Color32::from_rgb(0x13, 0x73, 0x33),
        axis_z: Color32::from_rgb(0x1a, 0x56, 0xc4),
        selected: Color32::from_rgb(0xb0, 0x0e, 0x5a),
        snap: Color32::from_rgb(0x8a, 0x4b, 0x00),
        measure: Color32::from_rgb(0x00, 0x5f, 0x70),
        measure_back: Color32::from_rgba_premultiplied(220, 220, 220, 220),
        middle_handle: Color32::from_rgb(0x00, 0x6d, 0x77),
        handle_fill: Color32::WHITE,
        tab: Color32::from_rgb(0xb3, 0x47, 0x00),
        ghost: Color32::from_rgb(0x5f, 0x63, 0x68),
        operation: Color32::from_rgb(0x7b, 0x1f, 0xa2),
        tool_mark: Color32::from_rgb(0x8a, 0x4b, 0x00),
        shade_toward: Color32::BLACK,
    };

    /// The palette for a theme.
    pub fn of(dark: bool) -> Palette {
        if dark { Palette::DARK } else { Palette::LIGHT }
    }
}

impl Default for Palette {
    fn default() -> Self {
        Palette::DARK
    }
}

/// How far a stale toolpath and the copies are faded.
const GHOST_FADE: f32 = 0.65;

/// Distance between direction arrows along the cutting path.
pub const ARROW_SPACING_MM: f64 = 40.0;
const AXIS_LENGTH_MM: f64 = 15.0;
/// A stronger grid line every this many steps.
const GRID_MAJOR_EVERY: i64 = 10;
/// More lines than this across the area and the finer ones are left out.
const GRID_MAX_LINES: usize = 400;
/// In 3D the grid reaches at least this far past the stock.
const GRID_MARGIN_MM: f64 = 50.0;
/// The axis lines across the grid, against the short strokes at the zero.
const AXIS_LINE_FADE: f32 = 0.55;
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
    /// The move each of `lines3` is (index in the program), for playback.
    pub line_moves: Vec<usize>,
    pub rejected: bool,
    /// The project changed since: drawn faded until a new one replaces it.
    pub stale: bool,
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
        (shape.lines3, shape.line_moves) = lines_3d(program);
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

/// What to emphasise when painting.
#[derive(Debug, Clone, Default)]
pub struct Highlight {
    /// The canvas selection.
    pub selected: BTreeSet<usize>,
    /// Contours of the operation selected in the operations list (drawn wider, under the
    /// selection colour).
    pub operation_contours: BTreeSet<usize>,
    /// That operation's index.
    pub operation: Option<usize>,
    /// Open contours that a follow path cuts: drawn solid, not dashed as "not cut".
    pub followed: BTreeSet<usize>,
}

/// Every move as a 3D segment, once the tool's position is known on all three axes.
type Lines3 = (Vec<(Point3, Point3, bool)>, Vec<usize>);

fn lines_3d(program: &Program) -> Lines3 {
    let mut lines = Vec::new();
    let mut moves = Vec::new();
    let (mut x, mut y, mut z): (Option<f64>, Option<f64>, Option<f64>) = (None, None, None);
    for (index, m) in program.moves.iter().enumerate() {
        let from = match (x, y, z) {
            (Some(x), Some(y), Some(z)) => Some((x, y, z)),
            _ => None,
        };
        (x, y, z) = (m.x.or(x), m.y.or(y), m.z.or(z));
        if let (Some(from), Some(x), Some(y), Some(z)) = (from, x, y, z)
            && from != (x, y, z)
        {
            lines.push((from, (x, y, z), m.kind != MoveKind::Rapid));
            moves.push(index);
        }
    }
    (lines, moves)
}

/// Where playback is (docs/specs/playback.md §1): the moves done, the one under way and
/// the tool on it.
#[derive(Debug, Clone, PartialEq)]
pub struct Progress {
    /// Moves `0..done` are done.
    pub done: usize,
    /// The tool's point on move `done` (it is under way), if known.
    pub tool_at: Option<Point3>,
    pub tool: ToolLook,
}

/// The tool's outline for the playback marker.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ToolLook {
    pub radius: f64,
    /// A cone's flat tip radius and how much it rises per mm out; None: a flat end.
    pub cone: Option<(f64, f64)>,
}

/// How faint the moves still to come are drawn.
const TO_COME_FADE: f32 = 0.3;

impl Progress {
    /// The moves done over the faint toolpath, the move under way up to the tool, and the
    /// tool.
    fn paint(&self, painter: &Painter, view: &View, path: &ToolpathShape, pal: &Palette) {
        let (high, low) = path.cut_z_range().unwrap_or((0.0, 0.0));
        let span = (high - low).max(f64::EPSILON);
        let flat = view.is_top();
        for (k, &(a, b, cutting)) in path.lines3.iter().enumerate() {
            let index = path.line_moves.get(k).copied().unwrap_or(usize::MAX);
            let b = if index < self.done {
                b
            } else if index == self.done
                && let Some(t) = self.tool_at
            {
                t
            } else {
                continue;
            };
            let ends = [view.to_screen3(a), view.to_screen3(b)];
            if cutting {
                let color = if flat {
                    pal.feed
                } else {
                    let depth = ((high - (a.2 + b.2) / 2.0) / span).clamp(0.0, 1.0) as f32;
                    shade(pal.feed, depth, pal.shade_toward)
                };
                painter.line_segment(ends, Stroke::new(2.0, color));
            } else if !flat || (a.0, a.1) != (b.0, b.1) {
                painter.extend(Shape::dashed_line(
                    &ends,
                    Stroke::new(1.0, pal.rapid),
                    DASH_PX,
                    GAP_PX,
                ));
            }
        }
        if let Some(at) = self.tool_at {
            self.paint_tool(painter, view, at, pal.tool_mark);
        }
    }

    /// A circle of the tool's size at its point; in 3D also its outline: a cylinder, or a
    /// cone on a cylinder.
    fn paint_tool(&self, painter: &Painter, view: &View, (x, y, z): Point3, mark: Color32) {
        let r = self.tool.radius;
        let stroke = Stroke::new(2.0, mark);
        let ring = |z: f64, radius: f64| -> Vec<Pos2> {
            (0..=32)
                .map(|k| {
                    let a = k as f64 / 32.0 * std::f64::consts::TAU;
                    view.to_screen3((x + radius * a.cos(), y + radius * a.sin(), z))
                })
                .collect()
        };
        if view.is_top() {
            painter.add(Shape::line(ring(z, r), stroke));
            painter.circle_filled(view.to_screen3((x, y, z)), 2.5, mark);
            return;
        }
        // The cutting end: flat, or a cone up to the full radius.
        let (tip, rise) = match self.tool.cone {
            Some((tip, slope)) => (tip, slope * (r - tip)),
            None => (r, 0.0),
        };
        let shank = (z + rise, z + rise + 3.0 * r);
        painter.add(Shape::line(ring(z, tip), stroke));
        painter.add(Shape::line(ring(shank.0, r), stroke));
        painter.add(Shape::line(ring(shank.1, r), stroke));
        for k in 0..4 {
            let a = k as f64 * std::f64::consts::FRAC_PI_2 + view.turn;
            let (c, s) = (a.cos(), a.sin());
            let at = |radius: f64, z: f64| view.to_screen3((x + radius * c, y + radius * s, z));
            painter.line_segment([at(tip, z), at(r, shank.0)], stroke);
            painter.line_segment([at(r, shank.0), at(r, shank.1)], stroke);
        }
    }
}

/// The simulated stock, ready to paint (docs/specs/simulation.md §4): an image for the top
/// view (its texture kept alive by the app) and a coarser surface for 3D.
#[derive(Debug, Clone)]
pub struct SimLayer {
    pub texture: eframe::egui::TextureId,
    /// The grid's lower left and upper right corners, drawing coordinates.
    pub min: Point,
    pub max: Point,
    pub surface: SimSurface,
}

/// A height map as a surface: one vertex per cell centre, row 0 at the lowest Y.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SimSurface {
    pub origin: Point,
    pub cell: f64,
    pub columns: usize,
    pub rows: usize,
    pub heights: Vec<f32>,
    pub colors: Vec<Color32>,
}

impl SimSurface {
    /// The surface as projected triangles, far quads first so near ones cover them.
    pub fn mesh(&self, view: &View, fade: bool) -> Mesh {
        let mut mesh = Mesh::default();
        if self.columns < 2 || self.rows < 2 {
            return mesh;
        }
        let tint = if fade { GHOST_FADE } else { 1.0 };
        mesh.reserve_vertices(self.columns * self.rows);
        for j in 0..self.rows {
            for i in 0..self.columns {
                let k = j * self.columns + i;
                let x = self.origin.0 + (i as f64 + 0.5) * self.cell;
                let y = self.origin.1 + (j as f64 + 0.5) * self.cell;
                let at = view.to_screen3((x, y, f64::from(self.heights[k])));
                mesh.colored_vertex(at, self.colors[k].gamma_multiply(tint));
            }
        }
        // Depth into the screen grows with x·sin(turn) + y·cos(turn) (View::camera).
        let (sin_t, cos_t) = view.turn.sin_cos();
        let mut quads: Vec<(f64, u32)> = Vec::with_capacity((self.columns - 1) * (self.rows - 1));
        for j in 0..self.rows - 1 {
            for i in 0..self.columns - 1 {
                let depth = i as f64 * sin_t + j as f64 * cos_t;
                // The grid's size is far within u32 (at most 250 000 cells).
                quads.push((depth, (j * self.columns + i) as u32));
            }
        }
        quads.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
        mesh.reserve_triangles(2 * quads.len());
        let columns = self.columns as u32;
        for (_, k) in quads {
            let (a, b, c, d) = (k, k + 1, k + columns + 1, k + columns);
            mesh.add_triangle(a, b, c);
            mesh.add_triangle(a, c, d);
        }
        mesh
    }
}

impl SimLayer {
    fn paint(&self, painter: &Painter, view: &View, fade: bool) {
        if view.is_top() {
            let rect = Rect::from_two_pos(
                view.to_screen((self.min.0, self.max.1)),
                view.to_screen((self.max.0, self.min.1)),
            );
            let uv = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
            let tint = if fade {
                Color32::WHITE.gamma_multiply(GHOST_FADE)
            } else {
                Color32::WHITE
            };
            painter.image(self.texture, rect, uv, tint);
        } else {
            painter.add(Shape::mesh(self.surface.mesh(view, fade)));
        }
    }
}

/// A picture traced into shapes, drawn faintly under the drawing in top view
/// (View > Show images; docs/specs/image-trace.md §6).
#[derive(Debug, Clone, PartialEq)]
pub struct PictureLayer {
    pub texture: eframe::egui::TextureId,
    /// The picture's corners in drawing coordinates, turned as the picture is: its
    /// lower-left, lower-right, upper-right and upper-left (docs/specs/direct-placement.md).
    pub corners: [Point; 4],
}

/// How strongly a picture shows under the drawing.
const PICTURE_FADE: f32 = 0.35;

impl PictureLayer {
    /// On the stock top, in top view and 3D.
    fn paint(&self, painter: &Painter, view: &View) {
        // Two triangles, so a turned picture is drawn turned. The picture's top row is
        // v = 0.
        let color = Color32::WHITE.gamma_multiply(PICTURE_FADE);
        let uv = [(0.0, 1.0), (1.0, 1.0), (1.0, 0.0), (0.0, 0.0)];
        let mut mesh = eframe::egui::Mesh::with_texture(self.texture);
        for (corner, (u, v)) in self.corners.iter().zip(uv) {
            mesh.vertices.push(eframe::egui::epaint::Vertex {
                pos: view.to_screen(*corner),
                uv: Pos2::new(u, v),
                color,
            });
        }
        mesh.add_triangle(0, 1, 2);
        mesh.add_triangle(0, 2, 3);
        painter.add(Shape::mesh(mesh));
    }
}

/// Everything on the canvas, in world coordinates.
#[derive(Debug, Clone, Default)]
pub struct Scene {
    pub contours: Vec<ContourShape>,
    pub stock: Option<(Point, Point)>,
    pub toolpath: Option<ToolpathShape>,
    pub tabs: Vec<Vec<Point>>,
    /// The shape being drawn, as it would be with the cursor's point.
    pub preview: Vec<Vec<Point>>,
    /// The points clicked so far while drawing.
    pub sketch_points: Vec<Point>,
    /// Handles of the selected drawn shape (top view only).
    pub handles: Vec<Point>,
    /// Middles of the selected polyline's segments, drawn as round handles in their own
    /// colour (top view only).
    pub middle_handles: Vec<Point>,
    /// The point the cursor snapped to, marked with a diamond.
    pub snap_mark: Option<Point>,
    /// View → Measure: from, to and the text (docs/specs/small-items.md §3).
    pub measure: Option<(Point, Point, String)>,
    /// The G-code's X0 Y0, in drawing coordinates: where the axes are drawn.
    pub work_zero: Point,
    /// The stock's (top, bottom) Z, for the 3D view.
    pub stock_z: Option<(f64, f64)>,
    /// The copies (copy 1 excluded): a contour's id and a shift, one for each copy of each
    /// part that cuts it. Drawn faded, never picked (docs/specs/copies-grid.md, parts.md).
    pub copies: Vec<(usize, Point)>,
    /// The material-removal simulation, when shown (docs/specs/simulation.md §4).
    pub sim: Option<SimLayer>,
    /// The project's pictures, when shown (docs/specs/image-trace.md §6).
    pub pictures: Vec<PictureLayer>,
    /// Playback away from the end (docs/specs/playback.md); None at the end.
    pub progress: Option<Progress>,
    /// Where each ring of the last generated toolpath starts (docs/specs/helix-and-leads.md
    /// part A); the focused operation's are shown and can be dragged.
    pub start_markers: Vec<StartMarker>,
    /// View → Show toolpath off: the toolpath, its plunges, tabs and ring starts aren't drawn;
    /// a playback's tool still is (docs/specs/drawing-sources.md §5).
    pub hide_toolpath: bool,
    /// View → Show drawing off: the contours, their copies and highlights aren't drawn and
    /// can't be picked (docs/specs/view-and-settings.md §2).
    pub hide_drawing: bool,
    /// The canvas's colours, for the app's theme.
    pub palette: Palette,
    /// View → Show grid: lines on the snap grid, on the stock top (docs/specs/
    /// browser-and-layout.md §6b).
    pub grid: bool,
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
        points.extend(
            self.contours
                .iter()
                .flat_map(|c| c.points.iter().map(|&p| on_top(p))),
        );
        for &(id, shift) in &self.copies {
            if let Some(c) = self.contours.iter().find(|c| c.id == id) {
                points.extend(
                    c.points
                        .iter()
                        .map(|&p| on_top((p.0 + shift.0, p.1 + shift.1))),
                );
            }
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
        if self.hide_drawing {
            return None;
        }
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
            .min_by_key(|&(d, id)| (Ordered(f64::from(d)), id))
            .map(|(_, id)| id)
    }

    /// Plunges (indices into the toolpath's starts) within pick distance of a screen point.
    pub fn plunges_at(&self, view: &View, at: Pos2) -> Vec<usize> {
        let Some(path) = self.toolpath.as_ref().filter(|_| !self.hide_toolpath) else {
            return Vec::new();
        };
        path.starts
            .iter()
            .enumerate()
            .filter(|(_, s)| view.to_screen(s.at).distance(at) <= PICK_TOLERANCE_PX)
            .map(|(i, _)| i)
            .collect()
    }

    /// Where the grid goes: what is on screen; in 3D, where the plane runs off to the
    /// horizon, no further than around the stock (or the drawing).
    fn grid_area(&self, painter: &Painter, view: &View) -> (Point, Point) {
        let rect = painter.clip_rect();
        let corners = [
            rect.left_top(),
            rect.right_top(),
            rect.right_bottom(),
            rect.left_bottom(),
        ]
        .map(|c| view.to_world(c));
        let lo = corners.iter().fold((f64::INFINITY, f64::INFINITY), |a, c| {
            (a.0.min(c.0), a.1.min(c.1))
        });
        let hi = corners
            .iter()
            .fold((f64::NEG_INFINITY, f64::NEG_INFINITY), |a, c| {
                (a.0.max(c.0), a.1.max(c.1))
            });
        if view.is_top() {
            return (lo, hi);
        }
        let around = self.stock.or_else(|| {
            let points = self.contours.iter().flat_map(|c| c.points.iter());
            let (mut min, mut max) = (
                (f64::INFINITY, f64::INFINITY),
                (f64::NEG_INFINITY, f64::NEG_INFINITY),
            );
            for p in points {
                min = (min.0.min(p.0), min.1.min(p.1));
                max = (max.0.max(p.0), max.1.max(p.1));
            }
            (min.0 <= max.0).then_some((min, max))
        });
        let ((x0, y0), (x1, y1)) = around.unwrap_or(((-100.0, -100.0), (100.0, 100.0)));
        let margin = ((x1 - x0).max(y1 - y0) * 0.5).max(GRID_MARGIN_MM);
        (
            (lo.0.max(x0 - margin), lo.1.max(y0 - margin)),
            (hi.0.min(x1 + margin), hi.1.min(y1 + margin)),
        )
    }

    /// The grid: a line every snap step (draw::grid_step), a stronger one every ten, so a
    /// click lands on a line it can see.
    fn paint_grid(&self, painter: &Painter, view: &View) {
        let pal = &self.palette;
        let step = crate::draw::grid_step(view.scale);
        let major = step * GRID_MAJOR_EVERY as f64;
        let ((x0, y0), (x1, y1)) = self.grid_area(painter, view);
        if x0 >= x1 || y0 >= y1 {
            return;
        }
        // Too many lines (a steep 3D view): the major ones alone.
        let count = ((x1 - x0) / step + (y1 - y0) / step) as usize;
        let minor = count <= GRID_MAX_LINES;
        let z = view.top_z;
        let line = |a: Point, b: Point, color: Color32| {
            painter.line_segment(
                [
                    view.to_screen3((a.0, a.1, z)),
                    view.to_screen3((b.0, b.1, z)),
                ],
                Stroke::new(1.0, color),
            );
        };
        for (spacing, color, is_major) in
            [(step, pal.grid_minor, false), (major, pal.grid_major, true)]
        {
            if !is_major && !minor {
                continue;
            }
            if (x1 - x0) / spacing + (y1 - y0) / spacing > (2 * GRID_MAX_LINES) as f64 {
                continue;
            }
            let first = (x0 / spacing).ceil() as i64;
            let last = (x1 / spacing).floor() as i64;
            for k in first..=last {
                if !is_major && k.rem_euclid(GRID_MAJOR_EVERY) == 0 {
                    continue;
                }
                let x = k as f64 * spacing;
                line((x, y0), (x, y1), color);
            }
            let first = (y0 / spacing).ceil() as i64;
            let last = (y1 / spacing).floor() as i64;
            for k in first..=last {
                if !is_major && k.rem_euclid(GRID_MAJOR_EVERY) == 0 {
                    continue;
                }
                let y = k as f64 * spacing;
                line((x0, y), (x1, y), color);
            }
        }
    }

    /// Lines through the work zero across the grid: X red, Y green; the short thick
    /// strokes drawn after them point the positive way.
    fn paint_axis_lines(&self, painter: &Painter, view: &View) {
        let pal = &self.palette;
        let ((x0, y0), (x1, y1)) = self.grid_area(painter, view);
        let (zx, zy) = self.work_zero;
        let z = view.top_z;
        let faint = |c: Color32| c.gamma_multiply(AXIS_LINE_FADE);
        if (y0..=y1).contains(&zy) {
            painter.line_segment(
                [view.to_screen3((x0, zy, z)), view.to_screen3((x1, zy, z))],
                Stroke::new(1.0, faint(pal.axis_x)),
            );
        }
        if (x0..=x1).contains(&zx) {
            painter.line_segment(
                [view.to_screen3((zx, y0, z)), view.to_screen3((zx, y1, z))],
                Stroke::new(1.0, faint(pal.axis_y)),
            );
        }
    }

    pub fn paint(&self, painter: &Painter, view: &View, highlight: &Highlight) {
        let pal = &self.palette;
        paint_background(painter, pal);
        if self.grid {
            self.paint_grid(painter, view);
        }
        self.paint_axis_lines(painter, view);
        let screen =
            |pts: &[Point]| -> Vec<Pos2> { pts.iter().map(|&p| view.to_screen(p)).collect() };
        let dashed = |pts: Vec<Pos2>, stroke: Stroke| {
            painter.extend(Shape::dashed_line(&pts, stroke, DASH_PX, GAP_PX));
        };

        let flat = view.is_top();
        if let Some(((x0, y0), (x1, y1))) = self.stock {
            let corners = [(x0, y0), (x1, y0), (x1, y1), (x0, y1), (x0, y0)];
            dashed(screen(&corners), Stroke::new(1.0, pal.stock));
            // In 3D the stock is a box: its bottom outline and the vertical edges.
            if !flat && let Some((top, bottom)) = self.stock_z {
                let at = |p: Point, z: f64| view.to_screen3((p.0, p.1, z));
                let bottom_ring: Vec<Pos2> = corners.iter().map(|&p| at(p, bottom)).collect();
                dashed(bottom_ring, Stroke::new(1.0, pal.stock));
                for &p in &corners[..4] {
                    dashed(vec![at(p, top), at(p, bottom)], Stroke::new(1.0, pal.stock));
                }
            }
        }
        // The axes mark the G-code zero (docs/specs/work-zero.md).
        let (zx, zy) = self.work_zero;
        let origin = view.to_screen((zx, zy));
        painter.line_segment(
            [origin, view.to_screen((zx + AXIS_LENGTH_MM, zy))],
            Stroke::new(2.0, pal.axis_x),
        );
        painter.line_segment(
            [origin, view.to_screen((zx, zy + AXIS_LENGTH_MM))],
            Stroke::new(2.0, pal.axis_y),
        );
        if !flat {
            painter.line_segment(
                [
                    origin,
                    view.to_screen3((zx, zy, view.top_z + AXIS_LENGTH_MM / view.z_scale)),
                ],
                Stroke::new(2.0, pal.axis_z),
            );
        }
        if let Some(sim) = &self.sim {
            let fade = self.toolpath.as_ref().is_some_and(|t| t.stale);
            sim.paint(painter, view, fade);
        }
        // Over the stock, so the trace can be compared with the cut.
        for picture in &self.pictures {
            picture.paint(painter, view);
        }
        if !self.hide_drawing {
            self.paint_drawing(painter, view, highlight);
        }
        self.paint_toolpath_and_marks(painter, view, highlight);
    }

    /// The drawing's contours, their copies and the selection and operation highlights.
    fn paint_drawing(&self, painter: &Painter, view: &View, highlight: &Highlight) {
        let (selected, operation) = (&highlight.selected, &highlight.operation_contours);
        let pal = &self.palette;
        let screen =
            |pts: &[Point]| -> Vec<Pos2> { pts.iter().map(|&p| view.to_screen(p)).collect() };
        let dashed = |pts: Vec<Pos2>, stroke: Stroke| {
            painter.extend(Shape::dashed_line(&pts, stroke, DASH_PX, GAP_PX));
        };
        for &(id, (dx, dy)) in &self.copies {
            if let Some(c) = self.contours.iter().find(|c| c.id == id) {
                let moved: Vec<Point> = c.points.iter().map(|p| (p.0 + dx, p.1 + dy)).collect();
                painter.add(Shape::line(screen(&moved), Stroke::new(1.0, pal.ghost)));
            }
        }
        for c in &self.contours {
            let pts = screen(&c.points);
            let is_selected = selected.contains(&c.id);
            if operation.contains(&c.id) {
                painter.add(Shape::line(pts.clone(), Stroke::new(6.0, pal.operation)));
            }
            let (color, width) = match (is_selected, c.kind) {
                (true, _) => (pal.selected, 3.0),
                (false, Kind::Outer) => (pal.outer, 1.5),
                (false, Kind::Hole) => (pal.hole, 1.5),
                (false, Kind::Open) => (pal.open, 1.5),
                (false, Kind::Invalid) => (pal.invalid, 2.5),
            };
            if c.kind == Kind::Open && !is_selected && !highlight.followed.contains(&c.id) {
                dashed(pts, Stroke::new(width, color));
            } else {
                painter.add(Shape::line(pts, Stroke::new(width, color)));
            }
        }
    }

    /// The toolpath with its plunges and tabs, the playback, sketches, handles and markers.
    fn paint_toolpath_and_marks(&self, painter: &Painter, view: &View, highlight: &Highlight) {
        let pal = &self.palette;
        let flat = view.is_top();
        let screen =
            |pts: &[Point]| -> Vec<Pos2> { pts.iter().map(|&p| view.to_screen(p)).collect() };
        let dashed = |pts: Vec<Pos2>, stroke: Stroke| {
            painter.extend(Shape::dashed_line(&pts, stroke, DASH_PX, GAP_PX));
        };
        let shown_path = self.toolpath.as_ref().filter(|_| !self.hide_toolpath);
        if let Some(path) = shown_path {
            // While playing back, the whole toolpath faint; the moves done go on top.
            let fade = match (&self.progress, path.stale) {
                (Some(_), _) => TO_COME_FADE,
                (None, true) => GHOST_FADE,
                (None, false) => 1.0,
            };
            let feed = if path.rejected {
                pal.feed_rejected
            } else {
                pal.feed
            }
            .gamma_multiply(fade);
            if flat {
                for &(a, b) in &path.rapids {
                    dashed(
                        screen(&[a, b]),
                        Stroke::new(1.0, pal.rapid.gamma_multiply(fade)),
                    );
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
                paint_path_3d(painter, view, path, feed, pal);
            }
            // Each operation is numbered once, at its first plunge (its number in the
            // operations list); other plunges get a small marker, once per position.
            let mut labelled: BTreeSet<Option<usize>> = BTreeSet::new();
            let mut marked: Vec<Point> = Vec::new();
            for plunge in &path.starts {
                let dot = view.to_screen(plunge.at);
                if plunge.operation.is_some() && labelled.insert(plunge.operation) {
                    painter.circle_stroke(dot, 5.0, Stroke::new(2.0, pal.label));
                    painter.text(
                        dot + Vec2::new(7.0, -19.0),
                        Align2::LEFT_TOP,
                        plunge.operation.map_or(0, |op| op + 1).to_string(),
                        FontId::proportional(14.0),
                        pal.label,
                    );
                } else if !marked.contains(&plunge.at) {
                    painter.circle_stroke(dot, 3.0, Stroke::new(1.0, pal.label));
                }
                marked.push(plunge.at);
            }
        }
        match (&self.progress, shown_path) {
            (Some(progress), Some(path)) => progress.paint(painter, view, path, pal),
            (Some(progress), None) => {
                if let Some(at) = progress.tool_at {
                    progress.paint_tool(painter, view, at, pal.tool_mark);
                }
            }
            _ => {}
        }
        // Tabs are marked in top view; in 3D the toolpath's lift over them shows them.
        for line in self.tabs.iter().filter(|_| flat && !self.hide_toolpath) {
            if line.len() > 1 {
                painter.add(Shape::line(screen(line), Stroke::new(6.0, pal.tab)));
            }
        }
        for line in &self.preview {
            if line.len() > 1 {
                dashed(screen(line), Stroke::new(1.5, pal.selected));
            }
        }
        for &p in &self.sketch_points {
            painter.circle_stroke(view.to_screen(p), 4.0, Stroke::new(1.5, pal.selected));
        }
        for &p in self.handles.iter().filter(|_| flat) {
            let square = Rect::from_center_size(view.to_screen(p), Vec2::splat(7.0));
            painter.rect_filled(square, 0.0, pal.handle_fill);
            painter.rect_stroke(
                square,
                0.0,
                Stroke::new(1.5, pal.selected),
                eframe::egui::StrokeKind::Middle,
            );
        }
        for &p in self.middle_handles.iter().filter(|_| flat) {
            let at = view.to_screen(p);
            painter.circle_filled(at, 4.0, pal.handle_fill);
            painter.circle_stroke(at, 4.0, Stroke::new(1.5, pal.middle_handle));
        }
        for marker in self.start_markers.iter().filter(|_| !self.hide_toolpath) {
            if flat && highlight.operation == Some(marker.operation) {
                let at = view.to_screen(marker.at);
                painter.circle_filled(at, 6.0, pal.background);
                painter.circle_stroke(at, 6.0, Stroke::new(2.0, pal.operation));
                if marker.stored.is_some() {
                    painter.circle_filled(at, 3.0, pal.operation);
                }
                let ahead = at
                    + Vec2::new(
                        (16.0 * marker.heading.cos()) as f32,
                        (-16.0 * marker.heading.sin()) as f32,
                    );
                painter.add(chevron(ahead, marker.heading, pal.operation));
            }
        }
        if let Some((from, to, text)) = self.measure.as_ref().filter(|_| flat) {
            let (a, b) = (view.to_screen(*from), view.to_screen(*to));
            painter.line_segment([a, b], Stroke::new(1.5, pal.measure));
            for end in [a, b] {
                painter.circle_stroke(end, 4.0, Stroke::new(1.5, pal.measure));
            }
            let label = painter.layout_no_wrap(text.clone(), FontId::monospace(13.0), pal.measure);
            let at = a.lerp(b, 0.5) + Vec2::new(8.0, -8.0 - label.size().y);
            let back = Rect::from_min_size(at, label.size()).expand(3.0);
            painter.rect_filled(back, 3.0, pal.measure_back);
            painter.galley(at, label, pal.measure);
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
            painter.add(Shape::closed_line(diamond, Stroke::new(1.5, pal.snap)));
        }
    }
}

/// The toolpath in 3D: rapids dashed, cutting moves (plunges included) shaded from light at
/// the highest cut to the plain colour at the deepest.
fn paint_path_3d(
    painter: &Painter,
    view: &View,
    path: &ToolpathShape,
    feed: Color32,
    pal: &Palette,
) {
    let (high, low) = path.cut_z_range().unwrap_or((0.0, 0.0));
    let span = (high - low).max(f64::EPSILON);
    for &(a, b, cutting) in &path.lines3 {
        let ends = [view.to_screen3(a), view.to_screen3(b)];
        if cutting {
            let depth = ((high - (a.2 + b.2) / 2.0) / span).clamp(0.0, 1.0) as f32;
            painter.line_segment(ends, Stroke::new(2.0, shade(feed, depth, pal.shade_toward)));
        } else {
            painter.extend(Shape::dashed_line(
                &ends,
                Stroke::new(1.0, pal.rapid),
                DASH_PX,
                GAP_PX,
            ));
        }
    }
}

/// How far towards white the highest cut in 3D is lightened.
const SHADE_LIGHTEN: f32 = 0.5;

/// `color` moved towards `toward` (white on the dark canvas, black on the light one) at
/// depth 0, the plain colour at depth 1: the deepest cut stays as easy to see as a cut in
/// top view.
fn shade(color: Color32, depth: f32, toward: Color32) -> Color32 {
    let k = SHADE_LIGHTEN * (1.0 - depth);
    let mix = |c: u8, t: u8| (f32::from(c) + (f32::from(t) - f32::from(c)) * k).round() as u8;
    Color32::from_rgb(
        mix(color.r(), toward.r()),
        mix(color.g(), toward.g()),
        mix(color.b(), toward.b()),
    )
}

/// The background gradient, from `background_top` down to `background`.
fn paint_background(painter: &Painter, pal: &Palette) {
    let rect = painter.clip_rect();
    let mut mesh = Mesh::default();
    mesh.colored_vertex(rect.left_top(), pal.background_top);
    mesh.colored_vertex(rect.right_top(), pal.background_top);
    mesh.colored_vertex(rect.right_bottom(), pal.background);
    mesh.colored_vertex(rect.left_bottom(), pal.background);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    painter.add(Shape::mesh(mesh));
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

    /// Every line and mark on the canvas stands out from it by at least 3:1 (WCAG's minimum
    /// for graphics), faded and 3D-shaded ones included; the measure text by 4.5:1. In both
    /// palettes, against both ends of the background gradient.
    #[test]
    fn canvas_colours_stand_out_from_the_background() {
        use crate::style::contrast;
        let over = |c: Color32, bg: Color32| bg.blend(c);
        for (name, pal) in [("dark", Palette::DARK), ("light", Palette::LIGHT)] {
            for background in [pal.background_top, pal.background] {
                let faded = |c: Color32| over(c.gamma_multiply(GHOST_FADE), background);
                let lines = [
                    ("outline", pal.outer),
                    ("hole", pal.hole),
                    ("open", pal.open),
                    ("invalid", pal.invalid),
                    ("stock", pal.stock),
                    ("feed", pal.feed),
                    ("rejected feed", pal.feed_rejected),
                    ("rapid", pal.rapid),
                    ("label", pal.label),
                    ("axis x", pal.axis_x),
                    ("axis y", pal.axis_y),
                    ("axis z", pal.axis_z),
                    ("selected", pal.selected),
                    ("snap", pal.snap),
                    ("measure", pal.measure),
                    ("middle handle", pal.middle_handle),
                    ("tab", pal.tab),
                    ("copies", pal.ghost),
                    ("operation", pal.operation),
                    ("tool", pal.tool_mark),
                    ("stale feed", faded(pal.feed)),
                    ("stale rapid", faded(pal.rapid)),
                    ("deepest cut in 3D", shade(pal.feed, 1.0, pal.shade_toward)),
                    (
                        "deepest rejected cut in 3D",
                        shade(pal.feed_rejected, 1.0, pal.shade_toward),
                    ),
                    ("highest cut in 3D", shade(pal.feed, 0.0, pal.shade_toward)),
                ];
                for (what, color) in lines {
                    let ratio = contrast(color, background);
                    assert!(ratio >= 3.0, "{name}: {what} is {ratio:.2}:1 on the canvas");
                }
                let behind_measure = over(pal.measure_back, background);
                let ratio = contrast(pal.measure, behind_measure);
                assert!(ratio >= 4.5, "{name}: measure text {ratio:.2}:1");
            }
            // The grid stays in the background: fainter than any line drawn on it.
            let bottom = pal.background;
            let major = contrast(over(pal.grid_major, bottom), bottom);
            assert!(major < 1.6, "{name}: the grid shouts ({major:.2}:1)");
        }
    }

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
            comments: Vec::new(),
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
            comments: Vec::new(),
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
}

/// What a legend entry looks like: drawn the way the canvas draws that thing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Swatch {
    Line(Color32, f32),
    Dashed(Color32),
    /// A plunge: the ring the canvas draws where the tool goes down.
    Plunge,
    /// A ring's start marker.
    Start,
}

/// What the canvas shows, explained: contours always; the toolpath and what goes with it once
/// generated; copies when there are some.
pub fn legend(scene: &Scene) -> Vec<(Swatch, &'static str)> {
    let pal = &scene.palette;
    let mut items = vec![
        (Swatch::Line(pal.outer, 1.5), "outline"),
        (Swatch::Line(pal.hole, 1.5), "hole"),
        (Swatch::Dashed(pal.open), "open contour (not cut)"),
        (Swatch::Line(pal.selected, 3.0), "selected"),
        (Swatch::Line(pal.operation, 6.0), "the operation's contours"),
        (Swatch::Dashed(pal.stock), "stock"),
    ];
    if !scene.copies.is_empty() {
        items.push((Swatch::Line(pal.ghost, 1.0), "copies"));
    }
    if let Some(path) = scene.toolpath.as_ref().filter(|_| !scene.hide_toolpath) {
        let feed = if path.rejected {
            pal.feed_rejected
        } else {
            pal.feed
        };
        items.push((Swatch::Line(feed, 2.0), "cutting move"));
        items.push((Swatch::Dashed(pal.rapid), "rapid, above the stock"));
        items.push((Swatch::Plunge, "where the tool goes down"));
        if !scene.tabs.is_empty() {
            items.push((Swatch::Line(pal.tab, 6.0), "tab"));
        }
        if !scene.start_markers.is_empty() {
            items.push((Swatch::Start, "ring start (drag to move)"));
        }
    }
    items
}

/// Paint a swatch in `rect`, on the canvas background.
pub fn paint_swatch(painter: &Painter, rect: Rect, swatch: Swatch, pal: &Palette) {
    painter.rect_filled(rect, 2.0, pal.background);
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
        Swatch::Plunge => {
            painter.circle_stroke(rect.center(), 5.0, Stroke::new(2.0, pal.label));
        }
        Swatch::Start => {
            painter.circle_stroke(rect.center(), 5.0, Stroke::new(2.0, pal.operation));
        }
    }
}
