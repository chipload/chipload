// SPDX-License-Identifier: GPL-3.0-or-later
//! Drawing shapes with the mouse: turning clicks into a circle, rectangle or polyline,
//! rounding clicks to a grid, and outlines for the preview. Plain data, no GUI, so every
//! rule is unit tested. See docs/specs/drawing-tools.md section 6.

use chipload_core::Point;
use chipload_core::primitives::distance;
use chipload_core::tolerances::CHAIN_TOL_MM;
use chipload_dxf::primitive;
use chipload_job::{Geometry, Shape};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    Circle,
    Rectangle,
    Polyline,
}

impl Tool {
    pub fn name(self) -> &'static str {
        match self {
            Tool::Circle => "circle",
            Tool::Rectangle => "rectangle",
            Tool::Polyline => "polyline",
        }
    }

    /// What the next click does.
    fn hint(self, clicks: usize) -> &'static str {
        match (self, clicks) {
            (Tool::Circle, 0) => "click the centre",
            (Tool::Circle, _) => "click a point on the circle",
            (Tool::Rectangle, 0) => "click a corner",
            (Tool::Rectangle, _) => "click the opposite corner",
            (Tool::Polyline, 0) => "click the first point",
            (Tool::Polyline, _) => {
                "click the next point; Enter or double-click finishes it open, the first \
                 point closes it, Backspace removes the last point"
            }
        }
    }
}

/// A shape being drawn: the tool and the points clicked so far (already on the grid).
#[derive(Debug, Clone, PartialEq)]
pub struct Sketch {
    pub tool: Tool,
    pub points: Vec<Point>,
}

/// What a click or Enter did.
#[derive(Debug, Clone, PartialEq)]
pub enum Step {
    /// Keep drawing.
    Continue,
    /// The shape is complete.
    Done(Geometry),
    /// Nothing happened, and why (empty when there's nothing worth saying, e.g. the second
    /// click of a double-click on the same point).
    Ignored(String),
}

impl Sketch {
    pub fn new(tool: Tool) -> Self {
        Sketch {
            tool,
            points: Vec::new(),
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
            return Step::Ignored(String::new());
        }
        match self.tool {
            Tool::Circle | Tool::Rectangle if self.points.is_empty() => {
                self.points.push(p);
                Step::Continue
            }
            Tool::Circle | Tool::Rectangle => match self.geometry_to(p) {
                Some(geometry) => Step::Done(geometry),
                None => Step::Ignored(format!("the {} would have no size", self.tool.name())),
            },
            Tool::Polyline => {
                if on_first && self.points.len() >= 3 {
                    return Step::Done(Geometry::Polyline {
                        closed: true,
                        points: self.points.iter().map(|&(x, y)| [x, y]).collect(),
                    });
                }
                self.points.push(p);
                Step::Continue
            }
        }
    }

    /// Enter or double-click: finish a polyline open.
    pub fn finish(&mut self) -> Step {
        if self.tool != Tool::Polyline {
            return Step::Ignored(String::new());
        }
        if self.points.len() < 2 {
            return Step::Ignored("a polyline needs at least 2 points".into());
        }
        Step::Done(Geometry::Polyline {
            closed: false,
            points: self.points.iter().map(|&(x, y)| [x, y]).collect(),
        })
    }

    /// Backspace: forget the last click.
    pub fn undo_point(&mut self) {
        self.points.pop();
    }

    /// The circle or rectangle from the first click to `p`; None if it would have no size.
    fn geometry_to(&self, p: Point) -> Option<Geometry> {
        let &first = self.points.first()?;
        match self.tool {
            Tool::Circle => {
                let radius = distance(first, p);
                (radius >= CHAIN_TOL_MM).then_some(Geometry::Circle {
                    center: [first.0, first.1],
                    radius,
                })
            }
            Tool::Rectangle => {
                let (length, width) = ((p.0 - first.0).abs(), (p.1 - first.1).abs());
                (length >= CHAIN_TOL_MM && width >= CHAIN_TOL_MM).then_some(Geometry::Rectangle {
                    center: [(first.0 + p.0) / 2.0, (first.1 + p.1) / 2.0],
                    length,
                    width,
                    angle: 0.0,
                })
            }
            Tool::Polyline => None,
        }
    }

    /// The shape as it would be with the cursor at `p`, for the preview.
    pub fn preview(&self, p: Point) -> Vec<Vec<Point>> {
        match self.tool {
            Tool::Circle | Tool::Rectangle => {
                self.geometry_to(p).map(|g| outline(&g)).unwrap_or_default()
            }
            Tool::Polyline if self.points.is_empty() => Vec::new(),
            Tool::Polyline => {
                let mut line = self.points.clone();
                line.push(p);
                vec![line]
            }
        }
    }

    /// Status bar text: what is being drawn, its size at the cursor, the next step.
    pub fn status(&self, p: Point, step: f64) -> String {
        let size = match (self.tool, self.points.first()) {
            (Tool::Circle, Some(&c)) => format!(" radius {:.3}", distance(c, p)),
            (Tool::Rectangle, Some(&c)) => {
                format!(" {:.3} x {:.3}", (p.0 - c.0).abs(), (p.1 - c.1).abs())
            }
            (Tool::Polyline, _) => format!(" {} points", self.points.len()),
            _ => String::new(),
        };
        format!(
            "Drawing a {}{size}: {} (grid {step} mm, Esc cancels)",
            self.tool.name(),
            self.tool.hint(self.points.len())
        )
    }
}

/// The grid clicks are rounded to: the finest of 0.01, 0.1, 0.5, 1, 5, 10, 50 mm that is at
/// least 8 pixels at this zoom (pixels per mm).
pub fn grid_step(scale: f64) -> f64 {
    const STEPS: [f64; 7] = [0.01, 0.1, 0.5, 1.0, 5.0, 10.0, 50.0];
    STEPS
        .into_iter()
        .find(|s| s * scale >= 8.0)
        .unwrap_or(STEPS[STEPS.len() - 1])
}

/// `p` rounded to the grid. Steps below 1 divide by a whole number of steps per mm, so
/// 0.3 comes out as the double nearest 0.3, not 3 × 0.1.
pub fn snap(p: Point, step: f64) -> Point {
    let round = |v: f64| {
        if step < 1.0 {
            let per_mm = (1.0 / step).round();
            (v * per_mm).round() / per_mm
        } else {
            (v / step).round() * step
        }
    };
    (round(p.0), round(p.1))
}

/// A drawn geometry as display polylines (arcs flattened), for the preview.
pub fn outline(geometry: &Geometry) -> Vec<Vec<Point>> {
    let shape = Shape {
        id: 0,
        layer: String::new(),
        geometry: geometry.clone(),
    };
    let mut points: Vec<Point> = Vec::new();
    for segment in shape.segments() {
        let prim = primitive(&segment);
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
                angle: 0.0
            })
        );
        // A flat rectangle is refused, and drawing goes on.
        let mut flat = Sketch::new(Tool::Rectangle);
        flat.click((0.0, 0.0), false);
        assert!(
            matches!(flat.click((10.0, 0.0), false), Step::Ignored(m) if m.contains("no size"))
        );
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
        assert_eq!(
            sketch.click((10.0, 5.0), false),
            Step::Ignored(String::new())
        );
        let mut open = sketch.clone();
        assert_eq!(
            open.finish(),
            Step::Done(Geometry::Polyline {
                closed: false,
                points: vec![[0.0, 0.0], [10.0, 0.0], [10.0, 5.0]]
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
    fn the_grid_follows_the_zoom_and_snaps_to_round_values() {
        assert_eq!(grid_step(2.0), 5.0); // 2 px/mm: the whole part on screen
        assert_eq!(grid_step(10.0), 1.0);
        assert_eq!(grid_step(20.0), 0.5);
        assert_eq!(grid_step(100.0), 0.1);
        assert_eq!(grid_step(10_000.0), 0.01);
        assert_eq!(grid_step(0.01), 50.0);
        assert_eq!(snap((0.31, -0.26), 0.1), (0.3, -0.3));
        assert_eq!(snap((12.26, 7.74), 0.5), (12.5, 7.5));
        assert_eq!(snap((12.6, -2.4), 5.0), (15.0, 0.0));
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
