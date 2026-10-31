// SPDX-License-Identifier: GPL-3.0-or-later
//! Drawing shapes with the mouse: turning clicks into a circle, rectangle or polyline,
//! rounding clicks to a grid, and outlines for the preview. Plain data, no GUI, so every
//! rule is unit tested. See docs/specs/drawing-tools.md section 6.

use std::collections::BTreeSet;

use chipload_core::MM_PER_INCH;
use chipload_core::Point;
use chipload_core::chain::Contour;
use chipload_core::primitives::distance;
use chipload_core::tolerances::CHAIN_TOL_MM;
use chipload_draw::construct::{arc_as_polyline, arc_middle_for_radius, arc_through};
use chipload_job::{Geometry, Shape, Units};
use chipload_pipeline::units::{length_text, parse_length};

use crate::i18n::{Lang, t};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    Circle,
    Rectangle,
    Polyline,
    /// Start, end, then a point it passes through: an open polyline of arcs.
    Arc,
    /// The centres of its two ends, then its width: a rectangle with round ends.
    Slot,
    /// The centre, then a corner: a regular polygon of `Sketch::sides` sides.
    Polygon,
}

/// A new polygon's sides until the user picks another number.
pub const POLYGON_SIDES_DEFAULT: u32 = 6;

/// A shape being drawn: the tool and the points clicked so far (already on the grid).
#[derive(Debug, Clone, PartialEq)]
pub struct Sketch {
    pub tool: Tool,
    pub points: Vec<Point>,
    /// A polygon's number of sides.
    pub sides: u32,
}

/// What a click or Enter did.
#[derive(Debug, Clone, PartialEq)]
pub enum Step {
    /// Keep drawing.
    Continue,
    /// The shape is complete.
    Done(Geometry),
    /// Nothing happened, and why (None when there's nothing worth saying, e.g. the second
    /// click of a double-click on the same point).
    Ignored(Option<Why>),
}

/// Why a click or Enter did nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Why {
    /// The shape would have no size (an arc: its three points lie on a line).
    NoSize(Tool),
    /// A polyline finished with fewer than 2 points.
    TooFewPoints,
}

impl Why {
    /// For the status bar.
    pub fn text(self, lang: &impl Lang) -> String {
        match self {
            Why::NoSize(Tool::Circle | Tool::Polygon) => t!(lang, "draw-no-size-circle"),
            Why::NoSize(Tool::Rectangle | Tool::Polyline) => t!(lang, "draw-no-size-rectangle"),
            Why::NoSize(Tool::Arc) => t!(lang, "draw-arc-on-a-line"),
            Why::NoSize(Tool::Slot) => t!(lang, "draw-no-size-slot"),
            Why::TooFewPoints => t!(lang, "draw-polyline-too-short"),
        }
    }
}

impl Sketch {
    pub fn new(tool: Tool) -> Self {
        Sketch {
            tool,
            points: Vec::new(),
            sides: POLYGON_SIDES_DEFAULT,
        }
    }

    /// A click at `p`. `on_first` says the click hit the polyline's first point on screen,
    /// which closes it.
    pub fn click(&mut self, p: Point, on_first: bool) -> Step {
        if self
            .points
            .last()
            .is_some_and(|&last| distance(last, p) < CHAIN_TOL_MM)
        {
            return Step::Ignored(None);
        }
        match self.tool {
            Tool::Polyline => {
                if on_first && self.points.len() >= 3 {
                    return Step::Done(Geometry::straight_polyline(
                        true,
                        self.points.iter().map(|&(x, y)| [x, y]).collect(),
                    ));
                }
                self.points.push(p);
                Step::Continue
            }
            _ if self.points.len() + 1 < self.clicks() => {
                self.points.push(p);
                Step::Continue
            }
            _ => match self.geometry_to(p) {
                Some(geometry) => Step::Done(geometry),
                None => Step::Ignored(Some(Why::NoSize(self.tool))),
            },
        }
    }

    /// How many clicks make the shape (a polyline: as many as it takes).
    fn clicks(&self) -> usize {
        match self.tool {
            Tool::Circle | Tool::Rectangle | Tool::Polygon => 2,
            Tool::Arc | Tool::Slot => 3,
            Tool::Polyline => usize::MAX,
        }
    }

    /// Enter or double-click: finish a polyline open.
    pub fn finish(&mut self) -> Step {
        if self.tool != Tool::Polyline {
            return Step::Ignored(None);
        }
        if self.points.len() < 2 {
            return Step::Ignored(Some(Why::TooFewPoints));
        }
        Step::Done(Geometry::straight_polyline(
            false,
            self.points.iter().map(|&(x, y)| [x, y]).collect(),
        ))
    }

    /// The last click that a line goes on from (perpendicular and tangent snaps): a
    /// polyline's, an arc's or a slot's last point. A circle's centre or a rectangle's
    /// corner isn't one.
    pub fn last_point(&self) -> Option<Point> {
        match self.tool {
            Tool::Polyline | Tool::Arc | Tool::Slot => self.points.last().copied(),
            Tool::Circle | Tool::Rectangle | Tool::Polygon => None,
        }
    }

    /// Backspace: forget the last click.
    pub fn undo_point(&mut self) {
        self.points.pop();
    }

    /// The shape with its last click at `p`; None if it would have no size.
    fn geometry_to(&self, p: Point) -> Option<Geometry> {
        let &first = self.points.first()?;
        match (self.tool, &self.points[..]) {
            (Tool::Circle, _) => {
                let radius = distance(first, p);
                (radius >= CHAIN_TOL_MM).then_some(Geometry::Circle {
                    center: [first.0, first.1],
                    radius,
                })
            }
            (Tool::Rectangle, _) => {
                let (length, width) = ((p.0 - first.0).abs(), (p.1 - first.1).abs());
                (length >= CHAIN_TOL_MM && width >= CHAIN_TOL_MM).then_some(Geometry::Rectangle {
                    center: [(first.0 + p.0) / 2.0, (first.1 + p.1) / 2.0],
                    length,
                    width,
                    angle: 0.0,
                    corner_radius: 0.0,
                })
            }
            (Tool::Arc, &[start, end]) => {
                let arc = arc_through(start, p, end)?;
                let (points, bulges) = arc_as_polyline(&arc);
                Some(Geometry::Polyline {
                    closed: false,
                    points: points.iter().map(|&(x, y)| [x, y]).collect(),
                    bulges,
                })
            }
            (Tool::Slot, &[a, b]) => slot(a, b, p),
            (Tool::Polygon, _) => {
                let radius = distance(first, p);
                (radius >= CHAIN_TOL_MM).then_some(Geometry::Polygon {
                    center: [first.0, first.1],
                    radius,
                    sides: self.sides,
                    angle: (p.1 - first.1).atan2(p.0 - first.0),
                })
            }
            _ => None,
        }
    }

    /// The shape as it would be with the cursor at `p`, for the preview.
    pub fn preview(&self, p: Point) -> Vec<Vec<Point>> {
        if self.points.is_empty() {
            return Vec::new();
        }
        match self.tool {
            Tool::Circle | Tool::Rectangle | Tool::Polygon => {
                self.geometry_to(p).map(|g| outline(&g)).unwrap_or_default()
            }
            Tool::Arc | Tool::Slot if self.points.len() + 1 == self.clicks() => {
                match self.geometry_to(p) {
                    Some(g) => outline(&g),
                    None => vec![self.points.clone()],
                }
            }
            Tool::Polyline | Tool::Arc | Tool::Slot => {
                let mut line = self.points.clone();
                line.push(p);
                vec![line]
            }
        }
    }

    /// Status bar text: what is being drawn, its size at the cursor, the next step; lengths
    /// in `units`.
    pub fn status(&self, p: Point, step: f64, units: Units, lang: &impl Lang) -> String {
        let n = |mm: f64| fixed(mm, units);
        let grid = length_text(step, units);
        let count = self.points.len();
        match (self.tool, &self.points[..]) {
            (Tool::Circle, &[c, ..]) => t!(
                lang,
                "draw-status-circle",
                radius = n(distance(c, p)),
                grid = grid
            ),
            (Tool::Circle, _) => t!(lang, "draw-status-circle-start", grid = grid),
            (Tool::Rectangle, &[c, ..]) => t!(
                lang,
                "draw-status-rectangle",
                length = n((p.0 - c.0).abs()),
                width = n((p.1 - c.1).abs()),
                grid = grid
            ),
            (Tool::Rectangle, _) => t!(lang, "draw-status-rectangle-start", grid = grid),
            (Tool::Polyline, []) => t!(
                lang,
                "draw-status-polyline-start",
                count = count,
                grid = grid
            ),
            (Tool::Polyline, _) => {
                t!(lang, "draw-status-polyline", count = count, grid = grid)
            }
            (Tool::Arc, []) => t!(lang, "draw-status-arc-start", grid = grid),
            (Tool::Arc, &[s]) => t!(
                lang,
                "draw-status-arc-end",
                length = n(distance(s, p)),
                grid = grid
            ),
            (Tool::Arc, &[s, e, ..]) => t!(
                lang,
                "draw-status-arc",
                radius = arc_through(s, p, e).map_or_else(String::new, |a| n(a.radius)),
                grid = grid
            ),
            (Tool::Polygon, &[c, ..]) => t!(
                lang,
                "draw-status-polygon",
                radius = n(distance(c, p)),
                grid = grid
            ),
            (Tool::Polygon, _) => t!(lang, "draw-status-polygon-start", grid = grid),
            (Tool::Slot, []) => t!(lang, "draw-status-slot-start", grid = grid),
            (Tool::Slot, &[a]) => t!(
                lang,
                "draw-status-slot-end",
                length = n(distance(a, p)),
                grid = grid
            ),
            (Tool::Slot, &[a, b, ..]) => t!(
                lang,
                "draw-status-slot",
                width = n(slot_width(a, b, p)),
                grid = grid
            ),
        }
    }
}

/// The slot from the centre of one end `a` to the other `b`, as wide as twice the distance
/// of `p` from the line between them: a rectangle whose corner radius is half its width.
fn slot(a: Point, b: Point, p: Point) -> Option<Geometry> {
    let between = distance(a, b);
    let width = slot_width(a, b, p);
    if between < CHAIN_TOL_MM || width < CHAIN_TOL_MM {
        return None;
    }
    Some(Geometry::Rectangle {
        center: [(a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0],
        length: between + width,
        width,
        angle: (b.1 - a.1).atan2(b.0 - a.0),
        corner_radius: width / 2.0,
    })
}

/// Twice the distance of `p` from the line through `a` and `b` (0 when they coincide).
fn slot_width(a: Point, b: Point, p: Point) -> f64 {
    let between = distance(a, b);
    if between < CHAIN_TOL_MM {
        return 0.0;
    }
    let cross = (b.0 - a.0) * (p.1 - a.1) - (b.1 - a.1) * (p.0 - a.0);
    2.0 * cross.abs() / between
}

// -- typed values and angle steps (docs/specs/drawing-input.md §2)

/// A value typed while drawing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Entry {
    /// `x,y`: this point.
    At(Point),
    /// `@dx,dy`: this far from the last point.
    By(Point),
    /// `d`: this far from the last point, towards the cursor.
    Distance(f64),
    /// `d<a`: this far from the last point at this angle (radians, CCW from +X).
    Polar(f64, f64),
}

/// Why a typed value can't be used.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryError {
    /// It doesn't read as any form of value.
    Unreadable,
    /// It counts from a last point and there is none yet.
    NeedsLastPoint,
    /// A plain distance with the cursor on the last point: no direction.
    NoDirection,
    /// An arc's radius less than half the distance between its ends.
    RadiusTooSmall,
    /// An arc's radius with the cursor on the line between its ends: no side to bow to.
    NoSide,
}

impl EntryError {
    /// For the status bar.
    pub fn text(self, lang: &impl Lang) -> String {
        match self {
            EntryError::Unreadable => t!(lang, "draw-entry-unreadable"),
            EntryError::NeedsLastPoint => t!(lang, "draw-entry-needs-point"),
            EntryError::NoDirection => t!(lang, "draw-entry-no-direction"),
            EntryError::RadiusTooSmall => t!(lang, "draw-entry-radius-too-small"),
            EntryError::NoSide => t!(lang, "draw-entry-no-side"),
        }
    }
}

/// Reads a typed value: `x,y`, `@dx,dy`, `d` or `d<a` (a in degrees), lengths in `units`
/// unless they name theirs. With a semicolon (`12,5;30`) the semicolon separates and a
/// comma is a decimal point; in `d<a` a comma is always a decimal point.
pub fn parse_entry(text: &str, units: Units) -> Result<Entry, EntryError> {
    let text = text.trim();
    let (relative, body) = match text.strip_prefix('@') {
        Some(rest) => (true, rest.trim()),
        None => (false, text),
    };
    let length = |t: &str| parse_length(t, units).ok_or(EntryError::Unreadable);
    if let Some((d, a)) = body.split_once('<') {
        let degrees: f64 = a
            .trim()
            .replace(',', ".")
            .parse()
            .map_err(|_| EntryError::Unreadable)?;
        if !degrees.is_finite() {
            return Err(EntryError::Unreadable);
        }
        return Ok(Entry::Polar(
            length(&d.replace(',', "."))?,
            degrees.to_radians(),
        ));
    }
    let pair = if body.contains(';') {
        body.split_once(';')
            .map(|(x, y)| (x.replace(',', "."), y.replace(',', ".")))
    } else {
        body.split_once(',')
            .map(|(x, y)| (x.to_string(), y.to_string()))
    };
    match pair {
        Some((x, y)) => {
            let point = (length(&x)?, length(&y)?);
            Ok(if relative {
                Entry::By(point)
            } else {
                Entry::At(point)
            })
        }
        None => Ok(Entry::Distance(length(body)?)),
    }
}

/// The point a typed value gives, counting from `anchor` (the last click), with the cursor
/// at `cursor` for a plain distance's direction.
pub fn typed_point(
    entry: Entry,
    anchor: Option<Point>,
    cursor: Point,
) -> Result<Point, EntryError> {
    let from = match (entry, anchor) {
        (Entry::At(p), _) => return Ok(p),
        (_, Some(from)) => from,
        (_, None) => return Err(EntryError::NeedsLastPoint),
    };
    match entry {
        Entry::At(p) => Ok(p),
        Entry::By((dx, dy)) => Ok((from.0 + dx, from.1 + dy)),
        Entry::Distance(d) => {
            let length = distance(from, cursor);
            if length < CHAIN_TOL_MM {
                return Err(EntryError::NoDirection);
            }
            let (ux, uy) = ((cursor.0 - from.0) / length, (cursor.1 - from.1) / length);
            Ok((from.0 + ux * d, from.1 + uy * d))
        }
        Entry::Polar(d, angle) => {
            let (sin, cos) = exact_sin_cos(angle);
            Ok((from.0 + d * cos, from.1 + d * sin))
        }
    }
}

/// Shift's angle steps while drawing.
pub const ANGLE_STEP_DEG: f64 = 15.0;

/// The cursor at `at` held to a multiple of 15° from `anchor`: projected onto that
/// direction, its length rounded to the grid of `step` (None: not rounded, Alt).
pub fn angle_step(anchor: Point, at: Point, step: Option<f64>, units: Units) -> Point {
    let (dx, dy) = (at.0 - anchor.0, at.1 - anchor.1);
    if dx == 0.0 && dy == 0.0 {
        return anchor;
    }
    let step_rad = ANGLE_STEP_DEG.to_radians();
    let angle = (dy.atan2(dx) / step_rad).round() * step_rad;
    let (sin, cos) = exact_sin_cos(angle);
    let along = dx * cos + dy * sin;
    let along = match step {
        Some(step) => snap((along, 0.0), step, units).0,
        None => along,
    };
    (anchor.0 + along * cos, anchor.1 + along * sin)
}

/// sin and cos, exact at multiples of 90° (so a line held to 90° is exactly vertical).
fn exact_sin_cos(angle: f64) -> (f64, f64) {
    let quarters = angle / std::f64::consts::FRAC_PI_2;
    if (quarters - quarters.round()).abs() < chipload_core::tolerances::ANGLE_TOL_RAD {
        match (quarters.round() as i64).rem_euclid(4) {
            0 => (0.0, 1.0),
            1 => (1.0, 0.0),
            2 => (0.0, -1.0),
            _ => (-1.0, 0.0),
        }
    } else {
        angle.sin_cos()
    }
}

impl Sketch {
    /// Where typed values count from: the last click.
    pub fn anchor(&self) -> Option<Point> {
        self.points.last().copied()
    }

    /// The point a typed value gives at this step, with the cursor at `cursor`. A plain
    /// length is an arc's radius at its last step (bowing to the cursor's side) and a
    /// slot's width; anything else counts from the last click.
    pub fn typed(&self, entry: Entry, cursor: Point) -> Result<Point, EntryError> {
        match (self.tool, &self.points[..], entry) {
            (Tool::Arc, &[start, end], Entry::Distance(radius)) => {
                if radius < distance(start, end) / 2.0 {
                    return Err(EntryError::RadiusTooSmall);
                }
                arc_middle_for_radius(start, end, radius, cursor).ok_or(EntryError::NoSide)
            }
            (Tool::Slot, &[a, b], Entry::Distance(width)) => {
                let between = distance(a, b);
                if between < CHAIN_TOL_MM {
                    return Err(EntryError::NoDirection);
                }
                // Half the width off the line, to its left (the width is the same either
                // side).
                let (nx, ny) = (-(b.1 - a.1) / between, (b.0 - a.0) / between);
                Ok((b.0 + nx * width / 2.0, b.1 + ny * width / 2.0))
            }
            _ => typed_point(entry, self.anchor(), cursor),
        }
    }

    /// Beside the cursor at `p`: the line's length and angle (polyline, an arc's or slot's
    /// first line), the radius (circle, arc), the width (slot), length × width
    /// (rectangle). None before the first click.
    pub fn readout(&self, p: Point, units: Units) -> Option<String> {
        let n = |mm: f64| fixed(mm, units);
        let from = self.anchor()?;
        let first = self.points.first().copied().unwrap_or(from);
        Some(match (self.tool, &self.points[..]) {
            (Tool::Arc, &[start, end]) => {
                format!("R {}", n(arc_through(start, p, end)?.radius))
            }
            (Tool::Slot, &[a, b]) => format!("W {}", n(slot_width(a, b, p))),
            (Tool::Polyline | Tool::Arc | Tool::Slot, _) => {
                let (dx, dy) = (p.0 - from.0, p.1 - from.1);
                let angle = if dx == 0.0 && dy == 0.0 {
                    0.0
                } else {
                    dy.atan2(dx).to_degrees()
                };
                let angle = if angle.abs() < 0.05 { 0.0 } else { angle };
                format!("L {}  {angle:.1}°", n(dx.hypot(dy)))
            }
            (Tool::Circle | Tool::Polygon, _) => format!("R {}", n(distance(first, p))),
            (Tool::Rectangle, _) => format!(
                "{} × {}",
                n((p.0 - first.0).abs()),
                n((p.1 - first.1).abs())
            ),
        })
    }
}

/// The grid clicks are rounded to, in mm: the finest step that is at least 8 pixels at this
/// zoom (pixels per mm). In mm: 0.01, 0.1, 0.5, 1, 5, 10, 50 mm; in inches 0.001", 0.01",
/// 1/32", 1/16", 1/8", 1/4", 1/2", 1", 2", 6", 12" (docs/specs/inch-units.md §5.1).
pub fn grid_step(scale: f64, units: Units) -> f64 {
    const MM: [f64; 7] = [0.01, 0.1, 0.5, 1.0, 5.0, 10.0, 50.0];
    const INCH: [f64; 11] = [
        0.001, 0.01, 0.03125, 0.0625, 0.125, 0.25, 0.5, 1.0, 2.0, 6.0, 12.0,
    ];
    let steps: Vec<f64> = match units {
        Units::Mm => MM.to_vec(),
        Units::Inch => INCH.iter().map(|s| s * MM_PER_INCH).collect(),
    };
    let last = steps.last().copied().unwrap_or(1.0);
    steps.into_iter().find(|s| s * scale >= 8.0).unwrap_or(last)
}

/// A stronger grid line every this many steps: every ten in mm; in inches every inch, or
/// every foot from 1" up.
pub fn grid_major_every(step: f64, units: Units) -> i64 {
    match units {
        Units::Mm => 10,
        Units::Inch => {
            let inches = step / MM_PER_INCH;
            let per = if inches < 1.0 { 1.0 } else { 12.0 };
            ((per / inches).round() as i64).max(1)
        }
    }
}

/// A ruler's tick (docs/specs/rulers.md): where it is (mm, drawing coordinates),
/// how long it is drawn, and its number on the longest.
#[derive(Debug, Clone, PartialEq)]
pub struct Tick {
    pub at: f64,
    pub size: TickSize,
    pub label: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TickSize {
    Minor,
    Half,
    Major,
}

/// The ticks of a ruler from `lo` to `hi` (mm) at `scale` (pixels per mm): one on every
/// grid line (`grid_step`), longer half way between the stronger lines, numbered (in
/// `units`) on the stronger lines, so the ruler and the grid agree.
pub fn ruler_ticks(lo: f64, hi: f64, scale: f64, units: Units) -> Vec<Tick> {
    let step = grid_step(scale, units);
    let every = grid_major_every(step, units);
    // A tick on either end counts, within float noise (3" in mm over its step).
    let first = ((lo - CHAIN_TOL_MM) / step).ceil() as i64;
    let last = ((hi + CHAIN_TOL_MM) / step).floor() as i64;
    if last < first || last - first > 10_000 {
        return Vec::new();
    }
    (first..=last)
        .map(|k| {
            let at = k as f64 * step;
            let size = if k.rem_euclid(every) == 0 {
                TickSize::Major
            } else if every % 2 == 0 && k.rem_euclid(every) == every / 2 {
                TickSize::Half
            } else {
                TickSize::Minor
            };
            let label = (size == TickSize::Major).then(|| ruler_number(at, units));
            Tick { at, size, label }
        })
        .collect()
}

/// A ruler's number: the length in `units`, no unit, no trailing zeros ("150", "0.5",
/// "-2.25").
fn ruler_number(mm: f64, units: Units) -> String {
    let value = mm / units.mm_per_unit();
    let text = format!("{value:.3}");
    let text = text.trim_end_matches('0').trim_end_matches('.');
    if text == "-0" {
        "0".to_string()
    } else {
        text.to_string()
    }
}

/// `p` rounded to the grid. Steps below 1 (mm or inch) divide by a whole number of steps per
/// unit, so 0.3 comes out as the double nearest 0.3, not 3 × 0.1.
pub fn snap(p: Point, step: f64, units: Units) -> Point {
    let size = units.mm_per_unit();
    let unit_step = step / size;
    let round = |v: f64| {
        let v = v / size;
        let r = if unit_step < 1.0 {
            let per_unit = (1.0 / unit_step).round();
            (v * per_unit).round() / per_unit
        } else {
            (v / unit_step).round() * unit_step
        };
        r * size
    };
    (round(p.0), round(p.1))
}

/// A length in `units` with a fixed number of decimals (3 in mm, 4 in inches), for the status
/// bar.
pub fn fixed(mm: f64, units: Units) -> String {
    let d = chipload_pipeline::units::decimals(units);
    format!("{:.d$}", chipload_pipeline::units::from_mm(mm, units))
}

// -- snapping (docs/specs/drawing-input.md §1; the rules are in chipload-draw)

pub use chipload_draw::snap::{SnapKind, SnapPoint, Snapper};

/// How far from the cursor a snap point is taken, in screen pixels.
pub const SNAP_RADIUS_PX: f64 = 8.0;

/// For the status bar: "end of #4".
pub fn snap_name(point: &SnapPoint, lang: &impl Lang) -> String {
    let contour = point.contour;
    match point.kind {
        SnapKind::End => t!(lang, "draw-snap-end", contour = contour),
        SnapKind::Intersection => match point.other.filter(|&o| o != contour) {
            Some(other) => t!(
                lang,
                "draw-snap-intersection",
                contour = contour,
                other = other
            ),
            None => t!(lang, "draw-snap-intersection-self", contour = contour),
        },
        SnapKind::Centre => t!(lang, "draw-snap-centre", contour = contour),
        SnapKind::Quadrant => t!(lang, "draw-snap-quadrant", contour = contour),
        SnapKind::Middle => t!(lang, "draw-snap-middle", contour = contour),
        SnapKind::Perpendicular => t!(lang, "draw-snap-perpendicular", contour = contour),
        SnapKind::Tangent => t!(lang, "draw-snap-tangent", contour = contour),
        SnapKind::Nearest => t!(lang, "draw-snap-nearest", contour = contour),
    }
}

/// The segments of `contours`, ready to snap to.
pub fn snapper(contours: &[Contour]) -> Snapper {
    Snapper::new(
        contours
            .iter()
            .flat_map(|c| c.parts.iter().map(move |part| (c.id, part.segment.prim))),
    )
}

/// The cursor while snapping: where it is, the tool's last point (for perpendicular and
/// tangent points), and the contours left out (the shape being dragged).
pub struct Cursor<'a> {
    pub at: Point,
    pub from: Option<Point>,
    pub skip: &'a BTreeSet<usize>,
}

/// Where the cursor lands: the snap point chosen within `SNAP_RADIUS_PX` on screen, else
/// the grid of `step`. `free` (Alt held): exactly at the cursor. `scale` is pixels per mm
/// (top view).
pub fn snap_cursor(
    cursor: &Cursor,
    snapper: &Snapper,
    scale: f64,
    step: f64,
    units: Units,
    free: bool,
) -> (Point, Option<SnapPoint>) {
    if free {
        return (cursor.at, None);
    }
    let reach = SNAP_RADIUS_PX / scale;
    match snapper.find(cursor.at, reach, cursor.from, cursor.skip) {
        Some(p) => (p.at, Some(p)),
        None => (snap(cursor.at, step, units), None),
    }
}

/// A drawn geometry as display polylines (arcs flattened), for the preview.
pub fn outline(geometry: &Geometry) -> Vec<Vec<Point>> {
    let shape = Shape {
        construction: false,
        id: 0,
        layer: String::new(),
        name: None,
        geometry: geometry.clone(),
    };
    let mut points: Vec<Point> = Vec::new();
    for segment in shape.segments() {
        let prim = segment.to_primitive();
        let pts = prim.points();
        let skip = usize::from(!points.is_empty());
        points.extend(pts.into_iter().skip(skip));
    }
    // Closed shapes' segments already end where they start; just make sure the loop shows.
    if shape.closed()
        && let (Some(&first), Some(&last)) = (points.first(), points.last())
        && first != last
    {
        points.push(first);
    }
    vec![points]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::Language::En;
    use chipload_core::primitives::Primitive;

    #[test]
    fn a_circle_is_its_centre_and_a_point_on_it() {
        let mut sketch = Sketch::new(Tool::Circle);
        assert_eq!(sketch.click((10.0, 20.0), false), Step::Continue);
        assert_eq!(
            sketch.click((13.0, 24.0), false),
            Step::Done(Geometry::Circle {
                center: [10.0, 20.0],
                radius: 5.0
            })
        );
    }

    #[test]
    fn a_rectangle_is_two_opposite_corners_in_any_order() {
        let mut sketch = Sketch::new(Tool::Rectangle);
        sketch.click((40.0, 30.0), false);
        assert_eq!(
            sketch.click((10.0, 10.0), false),
            Step::Done(Geometry::Rectangle {
                center: [25.0, 20.0],
                length: 30.0,
                width: 20.0,
                angle: 0.0,
                corner_radius: 0.0,
            })
        );
        // A flat rectangle is refused, and drawing goes on.
        let mut flat = Sketch::new(Tool::Rectangle);
        flat.click((0.0, 0.0), false);
        assert!(matches!(
            flat.click((10.0, 0.0), false),
            Step::Ignored(Some(Why::NoSize(Tool::Rectangle)))
        ));
        assert_eq!(flat.points.len(), 1);
    }

    #[test]
    fn a_polyline_closes_on_its_first_point_or_finishes_open() {
        let mut sketch = Sketch::new(Tool::Polyline);
        assert!(matches!(sketch.finish(), Step::Ignored(_)));
        for p in [(0.0, 0.0), (10.0, 0.0), (10.0, 5.0)] {
            assert_eq!(sketch.click(p, false), Step::Continue);
        }
        // The second click of a double-click lands on the same point: ignored.
        assert_eq!(sketch.click((10.0, 5.0), false), Step::Ignored(None));
        let mut open = sketch.clone();
        assert_eq!(
            open.finish(),
            Step::Done(Geometry::Polyline {
                closed: false,
                points: vec![[0.0, 0.0], [10.0, 0.0], [10.0, 5.0]],
                bulges: vec![0.0, 0.0],
            })
        );
        assert!(matches!(
            sketch.click((0.0, 0.0), true),
            Step::Done(Geometry::Polyline { closed: true, .. })
        ));
        // With only two points, clicking the first one adds it instead (no 2-point loop).
        let mut short = Sketch::new(Tool::Polyline);
        short.click((0.0, 0.0), false);
        short.click((5.0, 0.0), false);
        assert_eq!(short.click((0.0, 0.0), true), Step::Continue);
        short.undo_point();
        assert_eq!(short.points, [(0.0, 0.0), (5.0, 0.0)]);
    }

    #[test]
    fn ruler_ticks_sit_on_the_grid_and_number_its_stronger_lines() {
        // 2 px/mm: a tick every 5 mm, longer every 25, numbered every 50.
        let ticks = ruler_ticks(-12.0, 101.0, 2.0, Units::Mm);
        assert_eq!(ticks.first().unwrap().at, -10.0);
        assert_eq!(ticks.last().unwrap().at, 100.0);
        let numbered: Vec<_> = ticks.iter().filter_map(|t| t.label.clone()).collect();
        assert_eq!(numbered, ["0", "50", "100"]);
        let half: Vec<f64> = ticks
            .iter()
            .filter(|t| t.size == TickSize::Half)
            .map(|t| t.at)
            .collect();
        assert_eq!(half, [25.0, 75.0]);
        // Zoomed in: tenths, numbered every millimetre.
        let ticks = ruler_ticks(0.95, 2.05, 100.0, Units::Mm);
        let numbered: Vec<_> = ticks.iter().filter_map(|t| t.label.clone()).collect();
        assert_eq!(numbered, ["1", "2"]);
        // Inches: numbered in inches, every inch.
        let ticks = ruler_ticks(0.0, 3.0 * MM_PER_INCH, 4.0, Units::Inch);
        let numbered: Vec<_> = ticks.iter().filter_map(|t| t.label.clone()).collect();
        assert_eq!(numbered, ["0", "1", "2", "3"]);
    }

    #[test]
    fn the_grid_follows_the_zoom_and_snaps_to_round_values() {
        assert_eq!(grid_step(2.0, Units::Mm), 5.0); // 2 px/mm: the whole part on screen
        assert_eq!(grid_step(10.0, Units::Mm), 1.0);
        assert_eq!(grid_step(20.0, Units::Mm), 0.5);
        assert_eq!(grid_step(100.0, Units::Mm), 0.1);
        assert_eq!(grid_step(10_000.0, Units::Mm), 0.01);
        assert_eq!(grid_step(0.01, Units::Mm), 50.0);
        assert_eq!(snap((0.31, -0.26), 0.1, Units::Mm), (0.3, -0.3));
        assert_eq!(snap((12.26, 7.74), 0.5, Units::Mm), (12.5, 7.5));
        assert_eq!(snap((12.6, -2.4), 5.0, Units::Mm), (15.0, 0.0));
        // Inches: a 1/8" grid at 4 px/mm (1/16" would be 6.35 px), lines each inch.
        let eighth = grid_step(4.0, Units::Inch);
        assert_eq!(eighth, 0.125 * 25.4);
        assert_eq!(grid_major_every(eighth, Units::Inch), 8);
        assert_eq!(grid_major_every(25.4, Units::Inch), 12);
        assert_eq!(grid_major_every(5.0, Units::Mm), 10);
        // 10.1 mm is 0.3976": the nearest 1/8" is 3/8" = 9.525 mm.
        assert_eq!(snap((10.1, -0.1), eighth, Units::Inch), (0.375 * 25.4, 0.0));
    }

    fn contour(id: usize, prims: Vec<Primitive>) -> Contour {
        use chipload_core::chain::{OrientedSegment, Segment};
        Contour {
            id,
            parts: prims
                .into_iter()
                .map(|prim| OrientedSegment {
                    segment: Segment {
                        construction: false,
                        prim,
                        layer: "0".into(),
                        handle: String::new(),
                        source_type: String::new(),
                        shape: None,
                        from: None,
                    },
                    reversed: false,
                })
                .collect(),
            closed: false,
            invalid_reason: None,
            nesting_depth: None,
        }
    }

    #[test]
    fn the_cursor_snaps_to_the_nearest_point_then_the_grid() {
        use chipload_core::primitives::{Arc, Line};
        let line = |a: Point, b: Point| Primitive::Line(Line { start: a, end: b });
        let contours = vec![
            contour(1, vec![line((0.0, 0.0), (10.0, 0.0))]),
            contour(
                2,
                vec![Primitive::Arc(Arc {
                    center: (5.0, 0.0),
                    radius: 5.0,
                    start_angle: 0.0,
                    sweep: std::f64::consts::PI,
                })],
            ),
        ];
        let snapper = snapper(&contours);
        let none = BTreeSet::new();
        let cursor = |at: Point, skip| Cursor {
            at,
            from: None,
            skip,
        };
        // 10 px per mm: 8 px is 0.8 mm.
        let at = |p: Point| snap_cursor(&cursor(p, &none), &snapper, 10.0, 1.0, Units::Mm, false);
        let (p, hit) = at((9.5, 0.3));
        assert_eq!(p, (10.0, 0.0));
        // Line 1 ends at (10, 0), and so does the arc of #2: the end of #1 wins (lowest id).
        assert_eq!(snap_name(&hit.unwrap(), &En), "end of #1");
        // The line's middle and the arc's centre are both (5, 0): the centre wins.
        assert_eq!(snap_name(&at((5.2, 0.1)).1.unwrap(), &En), "centre of #2");
        // The arc's top is its quadrant (and its middle: the quadrant wins).
        assert_eq!(snap_name(&at((5.0, 4.6)).1.unwrap(), &En), "quadrant of #2");
        // Near the line only: on it.
        assert_eq!(snap_name(&at((2.6, 0.3)).1.unwrap(), &En), "on #1");
        // Nothing within 0.8 mm: the grid.
        assert_eq!(at((2.6, 2.4)), ((3.0, 2.0), None));
        // Alt: exactly where the cursor is.
        assert_eq!(
            snap_cursor(
                &cursor((9.5, 0.3), &none),
                &snapper,
                10.0,
                1.0,
                Units::Mm,
                true
            ),
            ((9.5, 0.3), None)
        );
        // The dragged shape's own points are skipped.
        let skip: BTreeSet<usize> = [1].into();
        let (_, hit) = snap_cursor(
            &cursor((9.5, 0.3), &skip),
            &snapper,
            10.0,
            1.0,
            Units::Mm,
            false,
        );
        assert_eq!(snap_name(&hit.unwrap(), &En), "end of #2");
        // Zoomed out, the same distance is within reach of more.
        let far = cursor((8.0, 2.0), &none);
        assert!(
            snap_cursor(&far, &snapper, 1.0, 1.0, Units::Mm, false)
                .1
                .is_some()
        );
        assert!(
            snap_cursor(&far, &snapper, 10.0, 1.0, Units::Mm, false)
                .1
                .is_none()
        );
    }

    #[test]
    fn typed_values_read_in_every_form() {
        let mm = |t: &str| parse_entry(t, Units::Mm);
        assert_eq!(mm("20,10"), Ok(Entry::At((20.0, 10.0))));
        assert_eq!(mm(" @-5, 2.5 "), Ok(Entry::By((-5.0, 2.5))));
        assert_eq!(mm("40"), Ok(Entry::Distance(40.0)));
        assert_eq!(mm("40<90"), Ok(Entry::Polar(40.0, 90f64.to_radians())));
        assert_eq!(mm("@40<90"), mm("40<90"));
        // The decimal comma: with a semicolon, and always in d<a.
        assert_eq!(mm("12,5;30"), Ok(Entry::At((12.5, 30.0))));
        assert_eq!(
            mm("12,5<22,5"),
            Ok(Entry::Polar(12.5, 22.5f64.to_radians()))
        );
        // Units: a plain number is in the display units; any value can name its own.
        assert_eq!(parse_entry("1", Units::Inch), Ok(Entry::Distance(25.4)));
        assert_eq!(parse_entry("10mm", Units::Inch), Ok(Entry::Distance(10.0)));
        assert_eq!(mm("1/2in,1in"), Ok(Entry::At((12.7, 25.4))));
        for bad in ["", "abc", "1,2,3", "4<", "<30", "4<x", "1;"] {
            assert_eq!(mm(bad), Err(EntryError::Unreadable), "{bad:?}");
        }
    }

    #[test]
    fn typed_values_count_from_the_last_click() {
        let last = Some((10.0, 10.0));
        let cursor = (10.0, 30.0); // straight up
        assert_eq!(
            typed_point(Entry::At((1.0, 2.0)), None, cursor),
            Ok((1.0, 2.0))
        );
        assert_eq!(
            typed_point(Entry::By((5.0, -1.0)), last, cursor),
            Ok((15.0, 9.0))
        );
        assert_eq!(
            typed_point(Entry::Distance(4.0), last, cursor),
            Ok((10.0, 14.0))
        );
        // Exactly at 90° and 180°: no rounding error.
        let up = typed_point(Entry::Polar(4.0, 90f64.to_radians()), last, cursor);
        assert_eq!(up, Ok((10.0, 14.0)));
        let left = typed_point(Entry::Polar(4.0, 180f64.to_radians()), last, cursor);
        assert_eq!(left, Ok((6.0, 10.0)));
        // A first point must be absolute; a distance needs a direction.
        assert_eq!(
            typed_point(Entry::Distance(4.0), None, cursor),
            Err(EntryError::NeedsLastPoint)
        );
        assert_eq!(
            typed_point(Entry::Distance(4.0), last, (10.0, 10.0)),
            Err(EntryError::NoDirection)
        );
        // A circle's typed distance is its radius.
        let mut circle = Sketch::new(Tool::Circle);
        circle.click((0.0, 0.0), false);
        let p = typed_point(Entry::Distance(7.5), circle.anchor(), (1.0, 0.0)).unwrap();
        assert!(matches!(
            circle.click(p, false),
            Step::Done(Geometry::Circle { radius, .. }) if radius == 7.5
        ));
    }

    #[test]
    fn shift_holds_the_line_to_fifteen_degree_steps() {
        let o = (10.0, 10.0);
        // Nearly flat: exactly flat, the length on the 1 mm grid.
        assert_eq!(
            angle_step(o, (22.3, 10.4), Some(1.0), Units::Mm),
            (22.0, 10.0)
        );
        // Nearly vertical, downwards.
        assert_eq!(
            angle_step(o, (10.2, -5.6), Some(1.0), Units::Mm),
            (10.0, -6.0)
        );
        // 15°: on that line, 10 mm long.
        let p = angle_step(o, (19.7, 12.6), Some(1.0), Units::Mm);
        assert!((distance(o, p) - 10.0).abs() < 1e-9);
        assert!(((p.1 - o.1).atan2(p.0 - o.0).to_degrees() - 15.0).abs() < 1e-9);
        // 345° is -15°.
        let q = angle_step(o, (19.7, 7.4), None, Units::Mm);
        assert!(((q.1 - o.1).atan2(q.0 - o.0).to_degrees() + 15.0).abs() < 1e-9);
        assert_eq!(angle_step(o, o, Some(1.0), Units::Mm), o);
    }

    #[test]
    fn the_readout_gives_the_size_being_drawn() {
        let mut line = Sketch::new(Tool::Polyline);
        assert_eq!(line.readout((1.0, 1.0), Units::Mm), None);
        line.click((0.0, 0.0), false);
        assert_eq!(
            line.readout((0.0, 20.0), Units::Mm).unwrap(),
            "L 20.000  90.0°"
        );
        let mut rect = Sketch::new(Tool::Rectangle);
        rect.click((0.0, 0.0), false);
        assert_eq!(
            rect.readout((-40.0, 20.0), Units::Mm).unwrap(),
            "40.000 × 20.000"
        );
        let mut circle = Sketch::new(Tool::Circle);
        circle.click((0.0, 0.0), false);
        assert_eq!(
            circle.readout((0.0, 25.4), Units::Inch).unwrap(),
            "R 1.0000"
        );
    }

    #[test]
    fn an_arc_is_start_end_and_a_point_on_it() {
        let mut arc = Sketch::new(Tool::Arc);
        assert_eq!(arc.click((0.0, 0.0), false), Step::Continue);
        assert_eq!(arc.click((10.0, 0.0), false), Step::Continue);
        // On the line between them: refused, drawing goes on.
        assert_eq!(
            arc.click((5.0, 0.0), false),
            Step::Ignored(Some(Why::NoSize(Tool::Arc)))
        );
        // Through (5, -5): the lower half circle, clockwise from (0, 0), one segment.
        let Step::Done(Geometry::Polyline {
            closed,
            points,
            bulges,
        }) = arc.click((5.0, -5.0), false)
        else {
            panic!()
        };
        assert!(!closed);
        assert_eq!(points.len(), 2);
        assert!((bulges[0] - 1.0).abs() < 1e-9);
        // A typed radius bows to the cursor's side; too small is refused.
        let mut typed = Sketch::new(Tool::Arc);
        typed.click((0.0, 0.0), false);
        typed.click((10.0, 0.0), false);
        let p = typed.typed(Entry::Distance(5.0), (3.0, 2.0)).unwrap();
        assert!(distance(p, (5.0, 5.0)) < 1e-9);
        assert_eq!(
            typed.typed(Entry::Distance(4.0), (3.0, 2.0)),
            Err(EntryError::RadiusTooSmall)
        );
        assert_eq!(
            typed.typed(Entry::Distance(6.0), (3.0, 0.0)),
            Err(EntryError::NoSide)
        );
        assert_eq!(typed.readout((5.0, 5.0), Units::Mm).unwrap(), "R 5.000");
    }

    #[test]
    fn a_slot_is_a_rectangle_with_half_circle_ends() {
        let mut slot = Sketch::new(Tool::Slot);
        slot.click((0.0, 0.0), false);
        slot.click((0.0, 30.0), false);
        assert_eq!(slot.readout((4.0, 10.0), Units::Mm).unwrap(), "W 8.000");
        // Its width typed: 8 mm.
        let p = slot.typed(Entry::Distance(8.0), (100.0, 100.0)).unwrap();
        let Step::Done(geometry) = slot.click(p, false) else {
            panic!()
        };
        let Geometry::Rectangle {
            center,
            length,
            width,
            angle,
            corner_radius,
        } = geometry
        else {
            panic!()
        };
        assert_eq!(center, [0.0, 15.0]);
        assert!((length - 38.0).abs() < 1e-9 && (width - 8.0).abs() < 1e-9);
        assert!((angle - std::f64::consts::FRAC_PI_2).abs() < 1e-12);
        assert!((corner_radius - 4.0).abs() < 1e-9);
        assert!(chipload_job::edit::is_slot(&geometry));
        // No width: refused.
        let mut flat = Sketch::new(Tool::Slot);
        flat.click((0.0, 0.0), false);
        flat.click((10.0, 0.0), false);
        assert_eq!(
            flat.click((20.0, 0.0), false),
            Step::Ignored(Some(Why::NoSize(Tool::Slot)))
        );
    }

    #[test]
    fn previews_follow_the_cursor() {
        let mut sketch = Sketch::new(Tool::Rectangle);
        assert!(sketch.preview((5.0, 5.0)).is_empty());
        sketch.click((0.0, 0.0), false);
        let preview = sketch.preview((4.0, 2.0));
        assert_eq!(preview[0].len(), 5); // four corners, closed
        let mut line = Sketch::new(Tool::Polyline);
        line.click((0.0, 0.0), false);
        assert_eq!(line.preview((1.0, 1.0)), [vec![(0.0, 0.0), (1.0, 1.0)]]);
        let circle = outline(&Geometry::Circle {
            center: [0.0, 0.0],
            radius: 10.0,
        });
        assert!(circle[0].len() > 8);
        assert!(
            circle[0]
                .iter()
                .all(|&p| (distance(p, (0.0, 0.0)) - 10.0).abs() < 1e-9)
        );
    }
}
