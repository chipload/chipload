// SPDX-License-Identifier: GPL-3.0-or-later
//! Drawing shapes with the mouse: turning clicks into a circle, rectangle or polyline,
//! rounding clicks to a grid, and outlines for the preview. Plain data, no GUI, so every
//! rule is unit tested. See docs/specs/drawing-tools.md section 6.

use chipload_core::math;
use std::collections::BTreeSet;

use chipload_core::MM_PER_INCH;
use chipload_core::Point;
use chipload_core::chain::Contour;
use chipload_core::primitives::distance;
use chipload_core::tolerances::CHAIN_TOL_MM;
use chipload_draw::construct::{
    arc_as_polyline, arc_by_centre, arc_middle_for_radius, arc_slot, arc_through,
    circle_by_diameter, circle_through, left_of, polygon_by_side, polygon_on_edge,
    rectangle_aligned, rectangle_by_centre, rectangle_by_corners, side_end_heading,
    tangent_arc_bulge, tangent_arc_end,
};
use chipload_job::{Geometry, Shape, Units};
use chipload_pipeline::units::{length_text, parse_length};

use crate::i18n::{Lang, t};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Tool {
    /// The centre, then a point on it.
    Circle,
    /// The two ends of a diameter (docs/specs/drawing-variants.md §2).
    CircleTwoPoints,
    /// Three points on it (docs/specs/drawing-variants.md §2).
    CircleThreePoints,
    /// Two opposite corners.
    Rectangle,
    /// The centre, then a corner (docs/specs/drawing-variants.md §2).
    RectangleCentre,
    /// One edge's two ends, then the width across it (docs/specs/drawing-variants.md §2).
    RectangleAligned,
    Polyline,
    /// Points the smooth curve passes through, as many as it takes (docs/specs/splines.md).
    Spline,
    /// Start, end, then a point it passes through: an open polyline of arcs.
    Arc,
    /// The centre, the start (the radius), then the end's direction, the way the cursor
    /// went round (docs/specs/drawing-variants.md §2).
    ArcCentre,
    /// The centres of its two ends, then its width: a rectangle with round ends.
    Slot,
    /// An arc slot: the centre, the start of its centre line, its end (the way the cursor
    /// went round), then its width (docs/specs/drawing-improvements.md §3).
    SlotArc,
    /// The centre, then a corner: a regular polygon of `Sketch::sides` sides.
    Polygon,
    /// The centre, then the middle of a side (docs/specs/drawing-variants.md §2).
    PolygonSide,
    /// The two ends of one side; the polygon lies to its left.
    PolygonEdge,
}

impl Tool {
    /// The shape this tool draws, as its button and key name it: a variant's first tool
    /// (docs/specs/drawing-variants.md §3).
    pub fn shape(self) -> Tool {
        match self {
            Tool::RectangleCentre | Tool::RectangleAligned => Tool::Rectangle,
            Tool::CircleTwoPoints | Tool::CircleThreePoints => Tool::Circle,
            Tool::ArcCentre => Tool::Arc,
            Tool::SlotArc => Tool::Slot,
            Tool::PolygonSide | Tool::PolygonEdge => Tool::Polygon,
            other => other,
        }
    }

    /// The ways to draw this tool's shape, the first today's (one when it has no others).
    pub fn variants(self) -> &'static [Tool] {
        match self.shape() {
            Tool::Rectangle => &[
                Tool::Rectangle,
                Tool::RectangleCentre,
                Tool::RectangleAligned,
            ],
            Tool::Circle => &[Tool::Circle, Tool::CircleTwoPoints, Tool::CircleThreePoints],
            Tool::Polyline => &[Tool::Polyline],
            Tool::Spline => &[Tool::Spline],
            Tool::Arc => &[Tool::Arc, Tool::ArcCentre],
            Tool::Slot => &[Tool::Slot, Tool::SlotArc],
            _ => &[Tool::Polygon, Tool::PolygonSide, Tool::PolygonEdge],
        }
    }

    /// The next way to draw the same shape, round to the first (the key pressed again).
    pub fn next_variant(self) -> Tool {
        let variants = self.variants();
        let at = variants.iter().position(|&t| t == self).unwrap_or(0);
        variants[(at + 1) % variants.len()]
    }
}

/// How close the spline's preview follows its curve, mm: fine enough on screen, cheap
/// enough for every frame (the shape itself is cut within `ARC_APPROX_TOL_MM`).
const SPLINE_PREVIEW_TOL_MM: f64 = 0.02;

/// A new polygon's sides until the user picks another number.
pub const POLYGON_SIDES_DEFAULT: u32 = 6;

/// A shape being drawn: the tool and the points clicked so far (already on the grid).
#[derive(Debug, Clone, PartialEq)]
pub struct Sketch {
    pub tool: Tool,
    pub points: Vec<Point>,
    /// A polygon's number of sides.
    pub sides: u32,
    /// The radius a polyline's corners get as they are drawn, mm (0 sharp;
    /// docs/specs/polyline-radii.md §4).
    pub corner_radius: f64,
    /// An arc by its centre: how far the cursor has gone round the centre since the start
    /// was clicked (radians, counter-clockwise positive), and its last direction from the
    /// centre; the arc goes that way round (docs/specs/drawing-variants.md §2).
    pub turned: f64,
    last_direction: Option<f64>,
    /// A polyline: the next side is a tangent arc, not a line
    /// (docs/specs/polyline-tangent-arcs.md §3).
    pub arcs: bool,
    /// A polyline: each side's bulge so far, one fewer than the points.
    pub bulges: Vec<f64>,
    /// A polyline started on an open shape's end: the direction it goes on in there, so
    /// its first side can be an arc.
    pub start_heading: Option<f64>,
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
    /// A polyline's tangent arc can't reach the point: it lies straight behind.
    ArcBehind,
    /// A polyline finished with fewer than 2 points.
    /// A polyline or spline finished with fewer than 2 points.
    TooFewPoints(Tool),
}

impl Why {
    /// For the status bar.
    pub fn text(self, lang: &impl Lang) -> String {
        match self {
            Why::NoSize(
                Tool::Circle
                | Tool::CircleTwoPoints
                | Tool::Polygon
                | Tool::PolygonSide
                | Tool::PolygonEdge,
            ) => {
                t!(lang, "draw-no-size-circle")
            }
            Why::NoSize(Tool::CircleThreePoints) => t!(lang, "draw-circle-on-a-line"),
            Why::NoSize(
                Tool::Rectangle
                | Tool::RectangleCentre
                | Tool::RectangleAligned
                | Tool::Polyline
                | Tool::Spline,
            ) => t!(lang, "draw-no-size-rectangle"),
            Why::NoSize(Tool::Arc) => t!(lang, "draw-arc-on-a-line"),
            Why::NoSize(Tool::ArcCentre) => t!(lang, "draw-no-size-arc"),
            Why::NoSize(Tool::Slot) => t!(lang, "draw-no-size-slot"),
            Why::NoSize(Tool::SlotArc) => t!(lang, "draw-no-size-slot-arc"),
            Why::ArcBehind => t!(lang, "draw-arc-behind"),
            Why::TooFewPoints(Tool::Spline) => t!(lang, "draw-spline-too-short"),
            Why::TooFewPoints(_) => t!(lang, "draw-polyline-too-short"),
        }
    }
}

impl Sketch {
    pub fn new(tool: Tool) -> Self {
        Sketch {
            tool,
            points: Vec::new(),
            sides: POLYGON_SIDES_DEFAULT,
            corner_radius: 0.0,
            turned: 0.0,
            last_direction: None,
            arcs: false,
            bulges: Vec::new(),
            start_heading: None,
        }
    }

    /// A polyline: the direction its last side ends in (the start's, from the shape it
    /// started on, before any side); None when there is none yet.
    pub fn heading(&self) -> Option<f64> {
        match &self.points[..] {
            [] => None,
            [_] => self.start_heading,
            [.., a, b] => Some(side_end_heading(
                *a,
                *b,
                self.bulges.last().copied().unwrap_or(0.0),
            )),
        }
    }

    /// Whether the next side is drawn as an arc: in arc mode, with a direction to go on
    /// from.
    pub fn drawing_arc(&self) -> bool {
        self.tool == Tool::Polyline && self.arcs && self.heading().is_some()
    }

    /// The bulge of the next side, from the last point to `p`: a tangent arc in arc mode
    /// (with a direction), else straight.
    fn next_bulge(&self, p: Point) -> Result<f64, Why> {
        match (self.drawing_arc(), self.points.last(), self.heading()) {
            (true, Some(&last), Some(heading)) => {
                tangent_arc_bulge(last, heading, p).ok_or(Why::ArcBehind)
            }
            _ => Ok(0.0),
        }
    }

    /// With Shift in arc mode: the end of the arc to the cursor at `p` with its sweep held
    /// to 15° steps, the chord as long as from the last point to `p` (rounded to `step`
    /// when given); None when not drawing an arc.
    pub fn arc_step(&self, p: Point, step: Option<f64>, units: Units) -> Option<Point> {
        if !self.drawing_arc() {
            return None;
        }
        let (&last, heading) = (self.points.last()?, self.heading()?);
        let chord = distance(last, p);
        if chord < CHAIN_TOL_MM {
            return Some(last);
        }
        let theta = math::atan2(p.1 - last.1, p.0 - last.0) - heading;
        let theta =
            (theta + std::f64::consts::PI).rem_euclid(std::f64::consts::TAU) - std::f64::consts::PI;
        let step_rad = ANGLE_STEP_DEG.to_radians();
        let most = std::f64::consts::TAU - step_rad;
        let sweep = ((2.0 * theta / step_rad).round() * step_rad).clamp(-most, most);
        let chord = match step {
            Some(step) => snap((chord, 0.0), step, units).0.max(step),
            None => chord,
        };
        Some(tangent_arc_end(last, heading, sweep, chord))
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
        self.follow(p);
        match self.tool {
            Tool::Spline => {
                if on_first && self.points.len() >= 3 {
                    return Step::Done(self.open_shape(true));
                }
                self.points.push(p);
                Step::Continue
            }
            Tool::Polyline => {
                let first = self.points.first().copied();
                let closing = on_first && self.points.len() >= 3;
                let target = if closing { first.unwrap_or(p) } else { p };
                let bulge = match self.next_bulge(target) {
                    Ok(bulge) => bulge,
                    Err(why) => return Step::Ignored(Some(why)),
                };
                if closing {
                    self.bulges.push(bulge);
                    return Step::Done(self.open_shape(true));
                }
                if !self.points.is_empty() {
                    self.bulges.push(bulge);
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
            Tool::Circle
            | Tool::CircleTwoPoints
            | Tool::Rectangle
            | Tool::RectangleCentre
            | Tool::Polygon
            | Tool::PolygonSide
            | Tool::PolygonEdge => 2,
            Tool::Arc
            | Tool::ArcCentre
            | Tool::Slot
            | Tool::RectangleAligned
            | Tool::CircleThreePoints => 3,
            Tool::SlotArc => 4,
            Tool::Polyline | Tool::Spline => usize::MAX,
        }
    }

    /// Enter or double-click: finish a polyline or spline open.
    pub fn finish(&mut self) -> Step {
        if !matches!(self.tool, Tool::Polyline | Tool::Spline) {
            return Step::Ignored(None);
        }
        if self.points.len() < 2 {
            return Step::Ignored(Some(Why::TooFewPoints(self.tool)));
        }
        Step::Done(self.open_shape(false))
    }

    /// The polyline (its sides lines or arcs) or spline through the points clicked; a
    /// closed polyline's closing side is the last bulge.
    fn open_shape(&self, closed: bool) -> Geometry {
        let points: Vec<[f64; 2]> = self.points.iter().map(|&(x, y)| [x, y]).collect();
        match self.tool {
            Tool::Spline => Geometry::Spline { closed, points },
            _ => rounded_polyline(points, self.bulges.clone(), closed, self.corner_radius),
        }
    }

    /// The last click that a line goes on from (perpendicular and tangent snaps): a
    /// polyline's, an arc's, a slot's or an aligned rectangle's last point. A circle's
    /// centre or a rectangle's corner isn't one.
    pub fn last_point(&self) -> Option<Point> {
        match self.tool {
            Tool::Polyline
            | Tool::Spline
            | Tool::Arc
            | Tool::Slot
            | Tool::RectangleAligned
            | Tool::CircleTwoPoints
            | Tool::CircleThreePoints
            | Tool::PolygonEdge => self.points.last().copied(),
            // From the centre: the arc's start, or a side's middle (perpendicular to a
            // line: the polygon touches it).
            Tool::ArcCentre | Tool::SlotArc | Tool::PolygonSide if self.points.len() == 1 => {
                self.points.first().copied()
            }
            Tool::Circle
            | Tool::Rectangle
            | Tool::RectangleCentre
            | Tool::Polygon
            | Tool::ArcCentre
            | Tool::SlotArc
            | Tool::PolygonSide => None,
        }
    }

    /// The cursor at `p`: an arc by its centre counts how far it goes round, so the arc
    /// goes the way the cursor turned. Called each frame and on each click; nothing for
    /// other tools.
    pub fn follow(&mut self, p: Point) {
        // An arc slot keeps its way round while its width is given.
        if self.tool == Tool::SlotArc && self.points.len() == 3 {
            return;
        }
        let (Tool::ArcCentre | Tool::SlotArc, &[c, start]) = (self.tool, &self.points[..]) else {
            self.turned = 0.0;
            self.last_direction = None;
            return;
        };
        if distance(c, p) < CHAIN_TOL_MM {
            return;
        }
        let direction = math::atan2(p.1 - c.1, p.0 - c.0);
        let last = self
            .last_direction
            .unwrap_or_else(|| math::atan2(start.1 - c.1, start.0 - c.0));
        self.turned += (direction - last + std::f64::consts::PI).rem_euclid(std::f64::consts::TAU)
            - std::f64::consts::PI;
        self.last_direction = Some(direction);
    }

    /// Backspace: forget the last click (a polyline's last side with it).
    pub fn undo_point(&mut self) {
        self.points.pop();
        self.bulges.truncate(self.points.len().saturating_sub(1));
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
            (Tool::CircleTwoPoints, _) => circle_by_diameter(first, p).map(circle),
            (Tool::CircleThreePoints, &[a, b]) => circle_through(a, b, p).map(circle),
            (Tool::Rectangle, _) => rectangle_by_corners(first, p).map(rectangle),
            (Tool::RectangleCentre, _) => rectangle_by_centre(first, p).map(rectangle),
            (Tool::RectangleAligned, &[a, b]) => rectangle_aligned(a, b, p).map(rectangle),
            (Tool::Arc, &[start, end]) => {
                let arc = arc_through(start, p, end)?;
                let (points, bulges) = arc_as_polyline(&arc);
                Some(Geometry::Polyline {
                    closed: false,
                    points: points.iter().map(|&(x, y)| [x, y]).collect(),
                    bulges,
                    radii: Vec::new(),
                })
            }
            (Tool::ArcCentre, &[c, start]) => {
                let (points, bulges) = arc_as_polyline(&arc_by_centre(c, start, p, self.turned)?);
                Some(Geometry::Polyline {
                    closed: false,
                    points: points.iter().map(|&(x, y)| [x, y]).collect(),
                    bulges,
                    radii: Vec::new(),
                })
            }
            (Tool::PolygonSide, _) => {
                polygon_by_side(first, p, self.sides).map(|g| polygon(g, self.sides))
            }
            (Tool::PolygonEdge, _) => {
                polygon_on_edge(first, p, self.sides).map(|g| polygon(g, self.sides))
            }
            (Tool::Slot, &[a, b]) => slot(a, b, p),
            (Tool::SlotArc, &[c, start]) => {
                // While the end is chosen: the centre line.
                let (points, bulges) = arc_as_polyline(&arc_by_centre(c, start, p, self.turned)?);
                Some(Geometry::Polyline {
                    closed: false,
                    points: points.iter().map(|&(x, y)| [x, y]).collect(),
                    bulges,
                    radii: Vec::new(),
                })
            }
            (Tool::SlotArc, &[c, start, end]) => {
                let arc = arc_by_centre(c, start, end, self.turned)?;
                let width = 2.0 * (distance(c, p) - arc.radius).abs();
                let (points, bulges) = arc_slot(&arc, width)?;
                Some(Geometry::Polyline {
                    closed: true,
                    points: points.iter().map(|&(x, y)| [x, y]).collect(),
                    bulges,
                    radii: Vec::new(),
                })
            }
            (Tool::Polygon, _) => {
                let radius = distance(first, p);
                (radius >= CHAIN_TOL_MM).then_some(Geometry::Polygon {
                    center: [first.0, first.1],
                    radius,
                    sides: self.sides,
                    angle: math::atan2(p.1 - first.1, p.0 - first.0),
                    radii: Vec::new(),
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
            Tool::Circle
            | Tool::CircleTwoPoints
            | Tool::Rectangle
            | Tool::RectangleCentre
            | Tool::Polygon
            | Tool::PolygonSide
            | Tool::PolygonEdge => self.geometry_to(p).map(|g| outline(&g)).unwrap_or_default(),
            Tool::Arc
            | Tool::ArcCentre
            | Tool::Slot
            | Tool::RectangleAligned
            | Tool::CircleThreePoints
                if self.points.len() + 1 == self.clicks() =>
            {
                match self.geometry_to(p) {
                    Some(g) => outline(&g),
                    None => vec![self.points.clone()],
                }
            }
            // A polyline: its sides as drawn, the next to the cursor, corners rounded as
            // they will be.
            Tool::Polyline => {
                let mut through: Vec<[f64; 2]> = self.points.iter().map(|&(x, y)| [x, y]).collect();
                let mut bulges = self.bulges.clone();
                if through
                    .last()
                    .is_none_or(|l| distance((l[0], l[1]), p) >= CHAIN_TOL_MM)
                {
                    match self.next_bulge(p) {
                        Ok(bulge) => {
                            through.push([p.0, p.1]);
                            bulges.push(bulge);
                        }
                        // Straight behind: the line to the cursor shows where it is.
                        Err(_) => {
                            let mut lines = outline(&rounded_polyline(
                                through.clone(),
                                bulges.clone(),
                                false,
                                self.corner_radius,
                            ));
                            if let Some(&last) = self.points.last() {
                                lines.push(vec![last, p]);
                            }
                            return lines;
                        }
                    }
                }
                if through.len() < 2 {
                    return Vec::new();
                }
                outline(&rounded_polyline(
                    through,
                    bulges,
                    false,
                    self.corner_radius,
                ))
            }
            Tool::Arc
            | Tool::ArcCentre
            | Tool::Slot
            | Tool::RectangleAligned
            | Tool::CircleThreePoints => {
                let mut line = self.points.clone();
                line.push(p);
                vec![line]
            }
            // An arc slot: the radius line, then its centre line, then the slot.
            Tool::SlotArc => match &self.points[..] {
                [c] => vec![vec![*c, p]],
                [c, start, end] => match self.geometry_to(p) {
                    Some(g) => outline(&g),
                    None => Sketch {
                        points: vec![*c, *start],
                        ..self.clone()
                    }
                    .geometry_to(*end)
                    .map(|g| outline(&g))
                    .unwrap_or_default(),
                },
                _ => self.geometry_to(p).map(|g| outline(&g)).unwrap_or_default(),
            },
            // The curve through the points and the cursor.
            Tool::Spline => {
                let mut through = self.points.clone();
                if through
                    .last()
                    .is_none_or(|&last| distance(last, p) >= CHAIN_TOL_MM)
                {
                    through.push(p);
                }
                let pieces = chipload_core::spline::beziers(&through, false);
                vec![chipload_core::spline::flatten(
                    &pieces,
                    false,
                    SPLINE_PREVIEW_TOL_MM,
                )]
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
            (Tool::CircleTwoPoints, &[a, ..]) => t!(
                lang,
                "draw-status-circle-two",
                diameter = n(distance(a, p)),
                grid = grid
            ),
            (Tool::CircleTwoPoints, _) => t!(lang, "draw-status-circle-two-start", grid = grid),
            (Tool::CircleThreePoints, []) => {
                t!(lang, "draw-status-circle-three-start", grid = grid)
            }
            (Tool::CircleThreePoints, &[_]) => {
                t!(lang, "draw-status-circle-three-second", grid = grid)
            }
            (Tool::CircleThreePoints, &[a, b, ..]) => t!(
                lang,
                "draw-status-circle-three",
                radius = circle_through(a, b, p).map_or_else(String::new, |(_, r)| n(r)),
                grid = grid
            ),
            (Tool::Rectangle, &[c, ..]) => t!(
                lang,
                "draw-status-rectangle",
                length = n((p.0 - c.0).abs()),
                width = n((p.1 - c.1).abs()),
                grid = grid
            ),
            (Tool::Rectangle, _) => t!(lang, "draw-status-rectangle-start", grid = grid),
            (Tool::RectangleCentre, &[c, ..]) => t!(
                lang,
                "draw-status-rectangle-centre",
                length = n(2.0 * (p.0 - c.0).abs()),
                width = n(2.0 * (p.1 - c.1).abs()),
                grid = grid
            ),
            (Tool::RectangleCentre, _) => {
                t!(lang, "draw-status-rectangle-centre-start", grid = grid)
            }
            (Tool::RectangleAligned, []) => {
                t!(lang, "draw-status-rectangle-aligned-start", grid = grid)
            }
            (Tool::RectangleAligned, &[a]) => t!(
                lang,
                "draw-status-rectangle-aligned-edge",
                length = n(distance(a, p)),
                grid = grid
            ),
            (Tool::RectangleAligned, &[a, b, ..]) => t!(
                lang,
                "draw-status-rectangle-aligned",
                length = n(distance(a, b)),
                width = n(left_of(a, b, p).abs()),
                grid = grid
            ),
            (Tool::Polyline, []) => t!(
                lang,
                "draw-status-polyline-start",
                count = count,
                grid = grid
            ),
            (Tool::Polyline, _) if self.drawing_arc() => {
                match self.next_bulge(p).ok().and_then(|b| {
                    chipload_core::primitives::Arc::from_bulge(self.points[count - 1], p, b)
                }) {
                    Some(arc) => t!(
                        lang,
                        "draw-status-polyline-arc",
                        count = count,
                        radius = n(arc.radius),
                        grid = grid
                    ),
                    None => t!(lang, "draw-status-polyline", count = count, grid = grid),
                }
            }
            (Tool::Polyline, _) if self.arcs => t!(
                lang,
                "draw-status-polyline-arc-first",
                count = count,
                grid = grid
            ),
            (Tool::Polyline, _) => {
                t!(lang, "draw-status-polyline", count = count, grid = grid)
            }
            (Tool::Spline, []) => t!(lang, "draw-status-spline-start", count = count, grid = grid),
            (Tool::Spline, _) => t!(lang, "draw-status-spline", count = count, grid = grid),
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
            (Tool::PolygonSide, &[c, ..]) => t!(
                lang,
                "draw-status-polygon-side",
                flats = n(2.0 * distance(c, p)),
                grid = grid
            ),
            (Tool::PolygonSide, _) => t!(lang, "draw-status-polygon-side-start", grid = grid),
            (Tool::PolygonEdge, &[a, ..]) => t!(
                lang,
                "draw-status-polygon-edge",
                length = n(distance(a, p)),
                grid = grid
            ),
            (Tool::PolygonEdge, _) => t!(lang, "draw-status-polygon-edge-start", grid = grid),
            (Tool::ArcCentre, []) => t!(lang, "draw-status-arc-centre-start", grid = grid),
            (Tool::ArcCentre, &[c]) => t!(
                lang,
                "draw-status-arc-centre-radius",
                radius = n(distance(c, p)),
                grid = grid
            ),
            (Tool::ArcCentre, &[c, start, ..]) => t!(
                lang,
                "draw-status-arc-centre",
                radius = n(distance(c, start)),
                angle = arc_by_centre(c, start, p, self.turned)
                    .map_or_else(String::new, |a| format!("{:.1}", a.sweep.to_degrees())),
                grid = grid
            ),
            (Tool::SlotArc, []) => t!(lang, "draw-status-slot-arc-start", grid = grid),
            (Tool::SlotArc, &[c]) => t!(
                lang,
                "draw-status-slot-arc-radius",
                radius = n(distance(c, p)),
                grid = grid
            ),
            (Tool::SlotArc, &[c, start]) => t!(
                lang,
                "draw-status-slot-arc-end",
                radius = n(distance(c, start)),
                angle = arc_by_centre(c, start, p, self.turned)
                    .map_or_else(String::new, |a| format!("{:.1}", a.sweep.to_degrees())),
                grid = grid
            ),
            (Tool::SlotArc, &[c, start, ..]) => t!(
                lang,
                "draw-status-slot-arc",
                width = n(2.0 * (distance(c, p) - distance(c, start)).abs()),
                grid = grid
            ),
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

/// The direction a polyline side from `a` to `b` with `bulge` ends in.
pub fn side_end(a: Point, b: Point, bulge: f64) -> f64 {
    side_end_heading(a, b, bulge)
}

/// The direction a polyline side from `a` to `b` with `bulge` leaves `a` in: an arc
/// leaves its chord half its sweep the other way round from where it meets it again.
pub fn side_start(a: Point, b: Point, bulge: f64) -> f64 {
    math::atan2(b.1 - a.1, b.0 - a.0) - 2.0 * math::atan(bulge)
}

/// A polyline through `points` with `bulges` (missing ones straight) and every corner
/// rounded by `radius`, a corner whose sides are too short for it by the most they have
/// room for (0: all sharp). Sides that meet tangentially have no corner.
fn rounded_polyline(
    points: Vec<[f64; 2]>,
    bulges: Vec<f64>,
    closed: bool,
    radius: f64,
) -> Geometry {
    let sides = chipload_job::polyline_segment_count(points.len(), closed);
    let bulges: Vec<f64> = (0..sides)
        .map(|i| bulges.get(i).copied().unwrap_or(0.0))
        .collect();
    let mut geometry = Geometry::Polyline {
        closed,
        points,
        bulges,
        radii: Vec::new(),
    };
    if radius <= 0.0 {
        return geometry;
    }
    if let Geometry::Polyline {
        points,
        bulges,
        radii,
        closed,
    } = &mut geometry
    {
        let pts: Vec<Point> = points.iter().map(|p| (p[0], p[1])).collect();
        let path = chipload_core::corners::Path {
            points: &pts,
            bulges,
            closed: *closed,
        };
        let mut rounds: Vec<f64> = (0..pts.len())
            .map(|k| if path.turn(k).is_some() { radius } else { 0.0 })
            .collect();
        // Corners that share a short side share it: each takes what is left, in turn.
        for _ in 0..pts.len() {
            let problems = chipload_core::corners::check(path, &rounds);
            let mut changed = false;
            for problem in problems {
                if let chipload_core::corners::RadiusError::TooLarge { at, most } = problem {
                    rounds[at] = most.max(0.0);
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        if rounds.iter().any(|&r| r > 0.0) {
            *radii = rounds;
        }
    }
    geometry
}

/// A drawn polygon of `sides` from its values.
fn polygon(g: chipload_draw::construct::Polygon, sides: u32) -> Geometry {
    Geometry::Polygon {
        center: [g.center.0, g.center.1],
        radius: g.radius,
        sides,
        angle: g.angle,
        radii: Vec::new(),
    }
}

/// A drawn circle from its centre and radius.
fn circle((center, radius): (Point, f64)) -> Geometry {
    Geometry::Circle {
        center: [center.0, center.1],
        radius,
    }
}

/// A drawn rectangle, square-cornered, from its values.
fn rectangle(r: chipload_draw::construct::Rectangle) -> Geometry {
    Geometry::Rectangle {
        center: [r.center.0, r.center.1],
        length: r.length,
        width: r.width,
        angle: r.angle,
        corner_radius: 0.0,
        radii: Vec::new(),
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
        angle: math::atan2(b.1 - a.1, b.0 - a.0),
        corner_radius: width / 2.0,
        radii: Vec::new(),
    })
}

/// Twice the distance of `p` from the line through `a` and `b` (0 when they coincide).
fn slot_width(a: Point, b: Point, p: Point) -> f64 {
    2.0 * left_of(a, b, p).abs()
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
    /// `<a`: at this angle from the last point, as far as the cursor is (radians, CCW
    /// from +X); an arc by its centre's end at this angle (drawing-variants.md §4).
    Angle(f64),
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

/// Reads a typed value: `x,y`, `@dx,dy`, `d`, `d<a` or `<a` (a in degrees), lengths in `units`
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
        let degrees = chipload_pipeline::units::parse_number_expr(&a.replace(',', "."))
            .ok_or(EntryError::Unreadable)?;
        if !degrees.is_finite() {
            return Err(EntryError::Unreadable);
        }
        if d.trim().is_empty() && !relative {
            return Ok(Entry::Angle(degrees.to_radians()));
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
        Entry::Angle(angle) => {
            let d = distance(from, cursor);
            if d < CHAIN_TOL_MM {
                return Err(EntryError::NoDirection);
            }
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
    let angle = (math::atan2(dy, dx) / step_rad).round() * step_rad;
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
        math::sin_cos(angle)
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
            (Tool::RectangleAligned, &[a, b], Entry::Distance(width)) => {
                let between = distance(a, b);
                if between < CHAIN_TOL_MM {
                    return Err(EntryError::NoDirection);
                }
                // The width across the edge, on the cursor's side (left when it is on it).
                let side = if left_of(a, b, cursor) < 0.0 {
                    -1.0
                } else {
                    1.0
                };
                let (nx, ny) = (-(b.1 - a.1) / between, (b.0 - a.0) / between);
                Ok((b.0 + side * nx * width, b.1 + side * ny * width))
            }
            // An arc slot's width: half of it either side of the centre line, out towards
            // the cursor.
            (Tool::SlotArc, &[c, start, _], Entry::Distance(width)) => {
                let d = distance(c, cursor);
                if d < CHAIN_TOL_MM {
                    return Err(EntryError::NoDirection);
                }
                let r = distance(c, start) + width / 2.0;
                Ok((
                    c.0 + (cursor.0 - c.0) / d * r,
                    c.1 + (cursor.1 - c.1) / d * r,
                ))
            }
            // An arc by its centre: the end at the angle, on the circle.
            (
                Tool::ArcCentre | Tool::SlotArc,
                &[c, start],
                Entry::Polar(_, angle) | Entry::Angle(angle),
            ) => {
                let r = distance(c, start);
                let (sin, cos) = exact_sin_cos(angle);
                Ok((c.0 + r * cos, c.1 + r * sin))
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
            (Tool::RectangleAligned, &[a, b]) => {
                format!("{} × {}", n(distance(a, b)), n(left_of(a, b, p).abs()))
            }
            (Tool::CircleTwoPoints, _) => format!(
                "Ø {}  R {}",
                n(distance(first, p)),
                n(distance(first, p) / 2.0)
            ),
            (Tool::CircleThreePoints, &[a, b]) => {
                format!("R {}", n(circle_through(a, b, p)?.1))
            }
            (Tool::ArcCentre | Tool::SlotArc, &[c]) => format!("R {}", n(distance(c, p))),
            (Tool::SlotArc, &[c, start, _]) => {
                format!("W {}", n(2.0 * (distance(c, p) - distance(c, start)).abs()))
            }
            (Tool::ArcCentre | Tool::SlotArc, &[c, start]) => {
                let arc = arc_by_centre(c, start, p, self.turned)?;
                format!("R {}  {:.1}°", n(arc.radius), arc.sweep.to_degrees())
            }
            (Tool::Polyline, _) if self.drawing_arc() => {
                let bulge = self.next_bulge(p).ok()?;
                match chipload_core::primitives::Arc::from_bulge(from, p, bulge) {
                    Some(arc) => format!("R {}  {:.1}°", n(arc.radius), arc.sweep.to_degrees()),
                    None => format!("L {}", n(distance(from, p))),
                }
            }
            (Tool::PolygonSide, _) => {
                let g = polygon_by_side(first, p, self.sides)?;
                format!("R {}  Ø {}", n(g.radius), n(2.0 * distance(first, p)))
            }
            (
                Tool::Polyline
                | Tool::Spline
                | Tool::Arc
                | Tool::Slot
                | Tool::RectangleAligned
                | Tool::CircleThreePoints
                | Tool::ArcCentre
                | Tool::SlotArc
                | Tool::PolygonEdge,
                _,
            ) => {
                let (dx, dy) = (p.0 - from.0, p.1 - from.1);
                let angle = if dx == 0.0 && dy == 0.0 {
                    0.0
                } else {
                    math::atan2(dy, dx).to_degrees()
                };
                let angle = if angle.abs() < 0.05 { 0.0 } else { angle };
                format!("L {}  {angle:.1}°", n(math::hypot(dx, dy)))
            }
            (Tool::Circle | Tool::Polygon, _) => format!("R {}", n(distance(first, p))),
            (Tool::Rectangle, _) => format!(
                "{} × {}",
                n((p.0 - first.0).abs()),
                n((p.1 - first.1).abs())
            ),
            (Tool::RectangleCentre, _) => format!(
                "{} × {}",
                n(2.0 * (p.0 - first.0).abs()),
                n(2.0 * (p.1 - first.1).abs())
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

/// The segments of `contours`, ready to snap to. A spline's segments are fitted to its
/// curve: they offer no ends or middles of their own, its points snap as ends instead
/// (docs/specs/splines.md §2).
pub fn snapper(contours: &[Contour], shapes: &[Shape]) -> Snapper {
    let splines: std::collections::BTreeMap<u64, &[[f64; 2]]> = shapes
        .iter()
        .filter_map(|s| match &s.geometry {
            Geometry::Spline { points, .. } => Some((s.id, points.as_slice())),
            _ => None,
        })
        .collect();
    let is_spline = |part: &chipload_core::chain::OrientedSegment| {
        part.segment
            .shape
            .is_some_and(|id| splines.contains_key(&id))
    };
    // Each spline's points once, with the contour it lies in.
    let mut seen = BTreeSet::new();
    let mut points = Vec::new();
    for c in contours {
        for id in c.parts.iter().filter_map(|p| p.segment.shape) {
            if let Some(own) = splines.get(&id)
                && seen.insert(id)
            {
                points.extend(own.iter().map(|p| (c.id, (p[0], p[1]))));
            }
        }
    }
    Snapper::with_curves(
        contours.iter().flat_map(|c| {
            c.parts
                .iter()
                .map(move |part| (c.id, part.segment.prim, is_spline(part)))
        }),
        points,
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
mod tests;
