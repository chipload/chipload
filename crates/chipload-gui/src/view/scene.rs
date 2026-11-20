// SPDX-License-Identifier: GPL-3.0-or-later
//! Everything one frame shows, and how it is painted.

use chipload_core::Point;
use chipload_core::math;
use chipload_core::order::Ordered;
use eframe::egui::{Align2, Color32, FontId, Painter, Pos2, Rect, Shape, Stroke, Vec2};

use super::paint::{
    chevron, paint_background, paint_compass, paint_corner_marker, paint_path_3d, paint_tab_marker,
    paint_turn_axis, segment_distance,
};
use super::progress::Progress;
use super::progress::TO_COME_FADE;
use super::shapes::{ContourShape, Highlight, Kind, RimLines, ToolpathShape};
use super::sim::{PictureLayer, SimLayer, SimSurface};
use super::{
    AXIS_LENGTH_MM, AXIS_LINE_FADE, CONSTRUCTION_DASH_PX, CONSTRUCTION_GAP_PX, DASH_PX, GAP_PX,
    GHOST_FADE, GRID_MARGIN_MM, GRID_MAX_LINES, LOCK_PX, OP_LABEL_RADIUS, OP_LABEL_TEXT_OFFSET,
    PAIR_MARK_MM, PAIR_MARK_PX, PICK_TOLERANCE_PX, Palette, Point3, View, ZERO_HANDLE_PX,
};

/// Where the Compass's arrows start and end from its centre, its ring's radius and half its
/// centre square's side, in screen pixels: drawn here, picked in the app's compass module.
pub const COMPASS_ARM_START_PX: f32 = 26.0;
pub const COMPASS_ARM_END_PX: f32 = 64.0;
pub const COMPASS_RING_PX: f32 = 18.0;
pub const COMPASS_CENTRE_PX: f32 = 6.0;

/// An instance's badge (docs/specs/instances.md §4): its number with a move mark, above the
/// top of its ghost, outside the shapes, the handle that drags it. Height, padding, the move
/// mark's size and a character's width, px.
pub const BADGE_HEIGHT_PX: f32 = 18.0;
const BADGE_PAD_PX: f32 = 5.0;
const BADGE_MARK_PX: f32 = 10.0;
const BADGE_CHAR_PX: f32 = 7.5;
/// The selected instance's turn handle's radius, px.
pub const TURN_HANDLE_PX: f32 = 7.0;

/// Where instance `ghost`'s badge is on the screen: its bottom left just above the
/// ghost's top point. Drawn in the view, picked in the app.
pub fn badge_rect(view: &View, ghost: &InstanceGhost) -> Rect {
    let corner = view.to_screen(ghost.label_at) + Vec2::new(0.0, -4.0);
    let chars = ghost.label.chars().count() as f32;
    let width = 3.0 * BADGE_PAD_PX + BADGE_MARK_PX + chars * BADGE_CHAR_PX;
    Rect::from_min_max(
        corner - Vec2::new(0.0, BADGE_HEIGHT_PX),
        corner + Vec2::new(width, 0.0),
    )
}

/// The badge: a rounded box with a four-way arrow and the instance's number.
fn paint_badge(
    painter: &Painter,
    rect: Rect,
    label: &str,
    color: Color32,
    fill: Color32,
    width: f32,
) {
    painter.rect_filled(rect, 4.0, fill);
    painter.rect_stroke(
        rect,
        4.0,
        Stroke::new(width, color),
        eframe::egui::StrokeKind::Inside,
    );
    let mark = Pos2::new(
        rect.left() + BADGE_PAD_PX + BADGE_MARK_PX / 2.0,
        rect.center().y,
    );
    paint_move_mark(painter, mark, BADGE_MARK_PX / 2.0, Stroke::new(1.2, color));
    painter.text(
        Pos2::new(
            rect.left() + 2.0 * BADGE_PAD_PX + BADGE_MARK_PX,
            rect.center().y,
        ),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(12.0),
        color,
    );
}

/// A four-way arrow of arm `r` about `c`: moves it.
fn paint_move_mark(painter: &Painter, c: Pos2, r: f32, stroke: Stroke) {
    let head = r * 0.45;
    for dir in [Vec2::X, -Vec2::X, Vec2::Y, -Vec2::Y] {
        let tip = c + dir * r;
        let side = Vec2::new(-dir.y, dir.x) * head;
        painter.line_segment([c, tip], stroke);
        painter.line_segment([tip, tip - dir * head + side], stroke);
        painter.line_segment([tip, tip - dir * head - side], stroke);
    }
}

/// A curved arrow round `c` of radius `r`: turns it.
fn paint_turn_mark(painter: &Painter, c: Pos2, r: f32, stroke: Stroke) {
    let (start, end) = (0.6_f32, 5.2_f32);
    let steps = 12;
    let at = |a: f32| {
        c + Vec2::new(
            math::cos(f64::from(a)) as f32,
            -(math::sin(f64::from(a)) as f32),
        ) * r
    };
    let points: Vec<Pos2> = (0..=steps)
        .map(|k| at(start + (end - start) * k as f32 / steps as f32))
        .collect();
    painter.add(Shape::line(points, stroke));
    // The head at the end, along the turn.
    let tip = at(end);
    let along = (at(end) - at(end - 0.3)).normalized();
    let side = Vec2::new(-along.y, along.x) * (r * 0.6);
    painter.line_segment([tip, tip - along * r * 0.6 + side], stroke);
    painter.line_segment([tip, tip - along * r * 0.6 - side], stroke);
}

/// The Compass as drawn: its centre, its X axis's angle (degrees), whether it is unlocked
/// (grey), and the part to highlight (X, Y, ring, centre).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CompassShape {
    pub centre: Point,
    pub angle_deg: f64,
    pub unlocked: bool,
    pub hot: Option<usize>,
}

/// One instance of a part drawn where it is cut (docs/specs/instances.md §4).
#[derive(Debug, Clone, Default)]
pub struct InstanceGhost {
    pub part: u64,
    pub id: u32,
    /// The part's cut contours, placed as the instance lies.
    pub outlines: Vec<Vec<Point>>,
    /// Its number, "#2", written above its top left.
    pub label: String,
    pub label_at: Point,
    pub selected: bool,
    pub enabled: bool,
    /// The point it turns about.
    pub centre: Point,
    /// Its turn handle, when selected.
    pub handle: Option<Point>,
    /// Under the pointer: its badge, turn handle or outline would take a press.
    pub hot: bool,
}

/// Everything on the canvas, in world coordinates.
/// Another setup, drawn as a sheet beside the active board (docs/specs/parts-and-setups.md
/// §7). Everything in it is moved by `offset` for the view only: never saved, never cut.
#[derive(Debug, Clone, PartialEq)]
pub struct Sheet {
    pub setup: u64,
    pub name: String,
    /// Its board, moved beside the active one.
    pub board: (Point, Point),
    /// Its instances' outlines, moved with it, closed ones closed.
    pub outlines: Vec<Vec<Point>>,
    /// How far it is drawn from where it lies in the drawing.
    pub offset: (f64, f64),
    /// It and the board left of it are a front and its back: a turn-over mark between them
    /// (docs/specs/two-sided.md §10a).
    pub paired: bool,
}

impl Scene {
    /// The sheet whose board holds drawing point `p`.
    pub fn sheet_at(&self, p: Point) -> Option<&Sheet> {
        self.sheets.iter().find(|s| {
            let ((x0, y0), (x1, y1)) = s.board;
            (x0..=x1).contains(&p.0) && (y0..=y1).contains(&p.1)
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct Scene {
    pub contours: Vec<ContourShape>,
    pub stock: Option<(Point, Point)>,
    /// The board drawn faintly: behind the drawing space's shapes, to draw against
    /// (docs/specs/drawing-and-setup-spaces.md §3).
    pub stock_faint: bool,
    /// A setup's space: the drawn shapes, pictures and models are neither drawn nor picked,
    /// only the instances (docs/specs/drawing-and-setup-spaces.md §4).
    pub shapes_hidden: bool,
    /// The other setups as sheets beside the active board (docs/specs/parts-and-setups.md
    /// §7): each its board and its instances' outlines, moved there for the view only.
    pub sheets: Vec<Sheet>,
    /// The sheet under the pointer: drawn lit, a click makes it the active setup.
    pub hot_sheet: Option<u64>,
    /// Contours only another setup's parts cut: drawn as ghosts.
    pub ghost_contours: std::collections::BTreeSet<usize>,
    /// The active setup's pieces when it is cut in pieces: each one's name and loops
    /// (docs/specs/pieces.md §9).
    pub pieces: Vec<(String, Vec<Vec<Point>>)>,
    pub toolpath: Option<ToolpathShape>,
    pub tabs: Vec<Vec<Point>>,
    /// The shape being drawn, as it would be with the cursor's point.
    pub preview: Vec<Vec<Point>>,
    /// The points clicked so far while drawing.
    pub sketch_points: Vec<Point>,
    /// Handles of the selected drawn shape (top view only).
    pub handles: Vec<Point>,
    /// Its points picked to move together, drawn filled (top view only).
    pub picked_handles: Vec<Point>,
    /// Middles of the selected polyline's segments, drawn as round handles in their own
    /// colour (top view only).
    pub middle_handles: Vec<Point>,
    /// The point the cursor snapped to, marked with a diamond.
    pub snap_mark: Option<Point>,
    /// View → Measure: from, to and the text (docs/specs/small-items.md §3).
    pub measure: Option<(Point, Point, String)>,
    /// The G-code's X0 Y0, in drawing coordinates: where the axes are drawn.
    pub work_zero: Point,
    /// The line a two-sided board turns about, across the board, and its label
    /// (docs/specs/two-sided-wizard.md §2).
    pub turn_axis: Option<(Point, Point, String)>,
    /// The work zero's handle and whether the zero is selected (top view only;
    /// docs/specs/work-zero-marker.md §3).
    pub zero_handle: Option<(Point, bool)>,
    /// The zero is a back's, its front's mirrored: drawn with a lock, never dragged
    /// (docs/specs/two-sided.md §10a).
    pub zero_locked: bool,
    /// A back's zero seen from its front, while the front's is selected or dragged: where it
    /// lies on this board, its X and Y directions, and the back's name (two-sided.md §10a).
    pub back_zero: Option<(Point, (f64, f64), String)>,
    /// The Compass on the selection (top view only; docs/specs/compass.md §3).
    pub compass: Option<CompassShape>,
    /// The stock's (top, bottom) Z, for the 3D view.
    pub stock_z: Option<(f64, f64)>,
    /// The active setup's parts' instances, as ghosts where they are cut: faded, numbered,
    /// picked by their outlines; the selected one with its turn handle
    /// (docs/specs/instances.md §4).
    pub instances: Vec<InstanceGhost>,
    /// A nest's preview (docs/specs/nesting.md §7): each instance's closed outlines where
    /// the nest would put it, parked (true) right of the board when it didn't fit.
    pub nest_preview: Vec<(Vec<Vec<Point>>, bool)>,
    /// The board the preview is nested on, when it differs from the stock shown.
    pub nest_board: Option<(Point, Point)>,
    /// The boards that take what didn't fit, in a row right of it, moved there for the view
    /// only (docs/specs/parts-and-setups.md §9).
    pub nest_more_boards: Vec<(Point, Point)>,
    /// Mirrored parts' contours where they are cut (docs/specs/mirror-part.md §5): drawn
    /// faded, never picked; the shapes as drawn stay to edit.
    pub mirrored: Vec<Vec<Point>>,
    /// The open relief's rim edges, closed loops, each with its line width, in the
    /// relief's colour (docs/specs/stl-relief.md §16.8).
    pub rim: Option<(Color32, RimLines)>,
    /// The material-removal simulation, when shown (docs/specs/simulation.md §4).
    pub sim: Option<SimLayer>,
    /// The project's pictures, when shown (docs/specs/image-trace.md §6).
    pub pictures: Vec<PictureLayer>,
    /// The project's 3D models, drawn faintly in 3D when no simulation is shown
    /// (docs/specs/stl-relief.md §9).
    pub models: Vec<SimSurface>,
    /// Playback away from the end (docs/specs/playback.md); None at the end.
    pub progress: Option<Progress>,
    /// Where each ring of the last generated toolpath starts (docs/specs/helix-and-leads.md
    /// part A); the focused operation's are shown and can be dragged.
    pub start_markers: Vec<StartMarker>,
    /// Every profile tab of the last generated toolpath; the focused operation's are shown
    /// and can be dragged (docs/specs/tabs-by-hand.md §5).
    pub tab_markers: Vec<TabMarker>,
    /// The focused profile's or pocket's corners on the tool's side, to click for a corner's
    /// own bone (docs/specs/dogbone-per-corner.md §6).
    pub corner_markers: Vec<CornerMarker>,
    /// The bone a hovered choice in a corner's menu would cut: the tool's centre and radius.
    pub corner_preview: Option<(Point, f64)>,
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
    /// The grid in inch steps (display units inch, docs/specs/inch-units.md §5.1).
    pub inch_grid: bool,
}

/// A profile's tab on the canvas (docs/specs/tabs-by-hand.md §5).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TabMarker {
    /// The tab as planned.
    pub tab: chipload_toolpath::plan::TabCentre,
    /// Where it is drawn: the tab's centre, or where it is being dragged to.
    pub at: Point,
}

/// A corner of the focused operation (docs/specs/dogbone-per-corner.md §6).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CornerMarker {
    pub at: Point,
    /// The tool at the tip of its bone, (centre, radius); None: the corner stays round.
    pub notch: Option<(Point, f64)>,
    /// Whether the corner has its own choice.
    pub chosen: bool,
    /// It would get a bone, but the last generation's tool couldn't reach into it.
    pub unreached: bool,
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
        self.heading = math::atan2(b.1 - a.1, b.0 - a.0);
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
        for ghost in &self.instances {
            points.extend(ghost.outlines.iter().flatten().map(|&p| on_top(p)));
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
        // The other setups' sheets beside the board (docs/specs/parts-and-setups.md §7).
        for sheet in &self.sheets {
            let ((x0, y0), (x1, y1)) = sheet.board;
            points.extend([on_top((x0, y0)), on_top((x1, y1))]);
        }
        points
    }

    /// The contour nearest to a screen position, within the pick tolerance.
    pub fn pick(&self, view: &View, at: Pos2) -> Option<usize> {
        if self.hide_drawing || self.shapes_hidden {
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

    /// The operation whose number (or the circle by it) is at a screen point: the numbers
    /// say which operation cuts there, so clicking one opens it (the user, 2026-10-28).
    pub fn op_label_at(&self, view: &View, at: Pos2) -> Option<usize> {
        let path = self.toolpath.as_ref().filter(|_| !self.hide_toolpath)?;
        path.op_labels()
            .into_iter()
            .map(|(p, op)| (view.to_screen(p), op))
            .filter(|&(dot, op)| {
                let digits = (op + 1).to_string().len() as f32;
                let text =
                    Rect::from_min_size(dot + OP_LABEL_TEXT_OFFSET, Vec2::new(9.0 * digits, 17.0));
                dot.distance(at) <= OP_LABEL_RADIUS + PICK_TOLERANCE_PX
                    || text.expand(2.0).contains(at)
            })
            .min_by_key(|&(dot, _)| Ordered(f64::from(dot.distance(at))))
            .map(|(_, op)| op)
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
        let units = if self.inch_grid {
            chipload_job::Units::Inch
        } else {
            chipload_job::Units::Mm
        };
        let step = crate::draw::grid_step(view.scale, units);
        let every = crate::draw::grid_major_every(step, units);
        let major = step * every as f64;
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
                if !is_major && k.rem_euclid(every) == 0 {
                    continue;
                }
                let x = k as f64 * spacing;
                line((x, y0), (x, y1), color);
            }
            let first = (y0 / spacing).ceil() as i64;
            let last = (y1 / spacing).floor() as i64;
            for k in first..=last {
                if !is_major && k.rem_euclid(every) == 0 {
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
        for sheet in &self.sheets {
            let hot = self.hot_sheet == Some(sheet.setup);
            let fade = if hot { 1.0 } else { 0.6 };
            let ((x0, y0), (x1, y1)) = sheet.board;
            let corners = [(x0, y0), (x1, y0), (x1, y1), (x0, y1), (x0, y0)];
            let width = if hot { 1.5 } else { 1.0 };
            dashed(
                screen(&corners),
                Stroke::new(width, pal.ghost.gamma_multiply(fade)),
            );
            for outline in &sheet.outlines {
                painter.add(Shape::line(
                    screen(outline),
                    Stroke::new(1.0, pal.ghost.gamma_multiply(fade * 0.8)),
                ));
            }
            // A front and its back: the turn-over mark over the gap on their left.
            if sheet.paired {
                let middle =
                    view.to_screen((x0 - PAIR_MARK_MM, y1)) + Vec2::new(0.0, -PAIR_MARK_PX * 1.4);
                let size = Vec2::splat(PAIR_MARK_PX);
                crate::icons::paint(
                    painter,
                    Rect::from_center_size(middle, size),
                    crate::icons::Icon::TurnOver,
                    pal.ghost.gamma_multiply(fade),
                );
            }
            // Its name above its top left corner.
            painter.text(
                view.to_screen((x0, y1)) + Vec2::new(0.0, -4.0),
                Align2::LEFT_BOTTOM,
                &sheet.name,
                FontId::proportional(12.0),
                pal.ghost.gamma_multiply(fade),
            );
        }
        if let Some(((x0, y0), (x1, y1))) = self.stock {
            let corners = [(x0, y0), (x1, y0), (x1, y1), (x0, y1), (x0, y0)];
            let board = if self.stock_faint {
                pal.stock.gamma_multiply(0.45)
            } else {
                pal.stock
            };
            dashed(screen(&corners), Stroke::new(1.0, board));
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
        if let Some((a, b, label)) = &self.turn_axis {
            paint_turn_axis(painter, view, (*a, *b), label, pal);
        }
        // The pieces of a setup cut in pieces: their outlines dashed, their names inside.
        for (name, rings) in &self.pieces {
            let mut lo = (f64::INFINITY, f64::INFINITY);
            let mut hi = (f64::NEG_INFINITY, f64::NEG_INFINITY);
            for ring in rings {
                let mut closed = ring.clone();
                if let Some(&first) = ring.first() {
                    closed.push(first);
                }
                dashed(screen(&closed), Stroke::new(1.5, pal.stock));
                for p in ring {
                    lo = (lo.0.min(p.0), lo.1.min(p.1));
                    hi = (hi.0.max(p.0), hi.1.max(p.1));
                }
            }
            if lo.0.is_finite() {
                painter.text(
                    view.to_screen(((lo.0 + hi.0) / 2.0, (lo.1 + hi.1) / 2.0)),
                    eframe::egui::Align2::CENTER_CENTER,
                    name,
                    eframe::egui::FontId::proportional(18.0),
                    pal.stock,
                );
            }
        }
        // The drawing origin as ghost axes once the work zero is elsewhere, so it can still
        // be found (docs/specs/work-zero-marker.md §3).
        let (zx, zy) = self.work_zero;
        let origin = view.to_screen((zx, zy));
        let drawing_origin = view.to_screen((0.0, 0.0));
        if origin.distance(drawing_origin) > 1.0 {
            let ghost = Stroke::new(1.5, pal.ghost);
            painter.line_segment(
                [drawing_origin, view.to_screen((AXIS_LENGTH_MM, 0.0))],
                ghost,
            );
            painter.line_segment(
                [drawing_origin, view.to_screen((0.0, AXIS_LENGTH_MM))],
                ghost,
            );
            painter.circle_stroke(drawing_origin, ZERO_HANDLE_PX - 1.0, ghost);
        }
        // The axes mark the G-code zero (docs/specs/work-zero.md).
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
        if !flat && self.sim.is_none() {
            for model in self.models.iter().filter(|_| !self.shapes_hidden) {
                painter.add(Shape::mesh(model.mesh(view, true)));
            }
        }
        // Over the stock, so the trace can be compared with the cut.
        for picture in self.pictures.iter().filter(|_| !self.shapes_hidden) {
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
        for ghost in &self.instances {
            let (color, width) = match (ghost.selected, ghost.hot) {
                (true, hot) => (pal.selected, if hot { 2.5 } else { 1.5 }),
                // Hovered: thicker in its own colour, never mistaken for the selected one.
                (false, true) => (pal.outer, 2.5),
                (false, false) => (pal.outer, 1.5),
            };
            for outline in &ghost.outlines {
                // An instance that is off is kept, not cut: dashed.
                if ghost.enabled {
                    painter.add(Shape::line(screen(outline), Stroke::new(width, color)));
                } else {
                    dashed(
                        screen(outline),
                        Stroke::new(width, color.gamma_multiply(0.6)),
                    );
                }
            }
            let badge = if ghost.enabled {
                color
            } else {
                color.gamma_multiply(0.6)
            };
            let stroke = if ghost.selected || ghost.hot {
                1.5
            } else {
                1.0
            };
            if ghost.label.is_empty() {
                continue;
            }
            paint_badge(
                painter,
                badge_rect(view, ghost),
                &ghost.label,
                badge,
                pal.handle_fill,
                stroke,
            );
        }
        for (outlines, parked) in &self.nest_preview {
            let color = if *parked { pal.open } else { pal.selected };
            for outline in outlines {
                painter.add(Shape::line(screen(outline), Stroke::new(1.5, color)));
            }
        }
        for &(low, high) in self.nest_board.iter().chain(&self.nest_more_boards) {
            let corners = [low, (high.0, low.1), high, (low.0, high.1), low];
            dashed(
                corners.iter().map(|&p| view.to_screen(p)).collect(),
                Stroke::new(1.5, pal.selected),
            );
        }
        // The drawn shapes and what lies on them: not in a setup's space.
        if self.shapes_hidden {
            return;
        }
        if let Some((color, lines)) = &self.rim {
            for (line, width) in lines {
                let mut pts = screen(line);
                if let Some(&first) = pts.first() {
                    pts.push(first);
                }
                dashed(pts, Stroke::new(*width, *color));
            }
        }
        for outline in &self.mirrored {
            let stroke = Stroke::new(1.0, pal.ghost);
            painter.extend(Shape::dashed_line(
                &screen(outline),
                stroke,
                DASH_PX,
                GAP_PX,
            ));
        }
        for c in &self.contours {
            let pts = screen(&c.points);
            let is_selected = selected.contains(&c.id);
            if operation.contains(&c.id) {
                painter.add(Shape::line(pts.clone(), Stroke::new(6.0, pal.operation)));
            }
            let ghost = self.ghost_contours.contains(&c.id);
            let (color, width) = match (is_selected, c.kind) {
                (true, _) => (pal.selected, 3.0),
                (false, _) if ghost => (pal.ghost.gamma_multiply(0.6), 1.0),
                (false, Kind::Outer) => (pal.outer, 1.5),
                (false, Kind::Hole) => (pal.hole, 1.5),
                (false, Kind::Open) => (pal.open, 1.5),
                (false, Kind::Invalid) => (pal.invalid, 2.5),
                (false, Kind::Construction) => (pal.open.gamma_multiply(0.7), 1.0),
            };
            if c.kind == Kind::Construction {
                // Fine dots: a guide, told apart from an open (uncut) line's dashes.
                painter.extend(Shape::dashed_line(
                    &pts,
                    Stroke::new(width, color),
                    CONSTRUCTION_DASH_PX,
                    CONSTRUCTION_GAP_PX,
                ));
            } else if c.kind == Kind::Open && !is_selected && !highlight.followed.contains(&c.id) {
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
            let labels = path.op_labels();
            for &(at, op) in &labels {
                let dot = view.to_screen(at);
                painter.circle_stroke(dot, OP_LABEL_RADIUS, Stroke::new(2.0, pal.label));
                painter.text(
                    dot + OP_LABEL_TEXT_OFFSET,
                    Align2::LEFT_TOP,
                    (op + 1).to_string(),
                    FontId::proportional(14.0),
                    pal.label,
                );
            }
            let mut marked: Vec<Point> = labels.iter().map(|&(at, _)| at).collect();
            for plunge in &path.starts {
                if !marked.contains(&plunge.at) {
                    let dot = view.to_screen(plunge.at);
                    painter.circle_stroke(dot, 3.0, Stroke::new(1.0, pal.label));
                    marked.push(plunge.at);
                }
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
        for &p in self.picked_handles.iter().filter(|_| flat) {
            let square = Rect::from_center_size(view.to_screen(p), Vec2::splat(7.0));
            painter.rect_filled(square, 0.0, pal.selected);
        }
        if let Some((p, selected)) = self.zero_handle.filter(|_| flat) {
            let at = view.to_screen(p);
            if selected {
                painter.circle_filled(at, ZERO_HANDLE_PX, pal.selected);
                painter.circle_stroke(at, ZERO_HANDLE_PX + 2.0, Stroke::new(1.5, pal.selected));
            } else {
                painter.circle_filled(at, ZERO_HANDLE_PX, pal.handle_fill);
                painter.circle_stroke(at, ZERO_HANDLE_PX, Stroke::new(1.5, pal.axis_x));
            }
            if self.zero_locked {
                let size = Vec2::splat(LOCK_PX);
                let corner = at + Vec2::new(ZERO_HANDLE_PX + 2.0, -ZERO_HANDLE_PX - LOCK_PX);
                crate::icons::paint(
                    painter,
                    eframe::egui::Rect::from_min_size(corner, size),
                    crate::icons::Icon::Lock,
                    pal.stock,
                );
            }
        }
        // The back's zero seen from the front, faint, dashed, named (two-sided.md §10a).
        if let Some(((zx, zy), (sx, sy), name)) = self.back_zero.as_ref().filter(|_| flat) {
            let faint = Stroke::new(1.5, pal.ghost);
            let origin = view.to_screen((*zx, *zy));
            for end in [
                (zx + sx * AXIS_LENGTH_MM, *zy),
                (*zx, zy + sy * AXIS_LENGTH_MM),
            ] {
                painter.extend(Shape::dashed_line(
                    &[origin, view.to_screen(end)],
                    faint,
                    DASH_PX,
                    GAP_PX,
                ));
            }
            painter.circle_stroke(origin, ZERO_HANDLE_PX - 1.0, faint);
            painter.text(
                origin + Vec2::new(ZERO_HANDLE_PX + 3.0, ZERO_HANDLE_PX + 3.0),
                Align2::LEFT_TOP,
                name,
                FontId::proportional(12.0),
                pal.ghost,
            );
        }
        // The selected instance's turn handle, on a line from the centre it turns about.
        for ghost in self.instances.iter().filter(|_| flat && !self.hide_drawing) {
            if let Some(handle) = ghost.handle {
                let (from, to) = (view.to_screen(ghost.centre), view.to_screen(handle));
                painter.line_segment([from, to], Stroke::new(1.0, pal.selected));
                painter.circle_filled(to, TURN_HANDLE_PX, pal.handle_fill);
                painter.circle_stroke(to, TURN_HANDLE_PX, Stroke::new(1.5, pal.selected));
                paint_turn_mark(
                    painter,
                    to,
                    TURN_HANDLE_PX * 0.55,
                    Stroke::new(1.2, pal.selected),
                );
            }
        }
        if let Some(compass) = self.compass.filter(|_| flat) {
            paint_compass(painter, view, &compass, pal);
        }
        for &p in self.middle_handles.iter().filter(|_| flat) {
            let at = view.to_screen(p);
            painter.circle_filled(at, 4.0, pal.handle_fill);
            painter.circle_stroke(at, 4.0, Stroke::new(1.5, pal.middle_handle));
        }
        for marker in self.corner_markers.iter().filter(|_| flat) {
            paint_corner_marker(painter, view, marker, pal);
        }
        if let Some((centre, radius)) = self.corner_preview.filter(|_| flat) {
            let at = view.to_screen(centre);
            let r = (radius * view.scale) as f32;
            painter.circle_stroke(at, r, Stroke::new(2.0, pal.selected));
        }
        for marker in self
            .tab_markers
            .iter()
            .filter(|_| flat && !self.hide_toolpath)
        {
            if highlight.operation == Some(marker.tab.operation) {
                paint_tab_marker(painter, view, marker, pal);
            }
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
                        (16.0 * math::cos(marker.heading)) as f32,
                        (-16.0 * math::sin(marker.heading)) as f32,
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
