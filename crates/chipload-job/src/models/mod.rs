// SPDX-License-Identifier: GPL-3.0-or-later
//! The project model: a declarative description of how to machine a drawing
//! (docs/specs/project-file.md), by part: the project itself (`project`), the stock, machine
//! and tools (`machine`), the drawing (`drawing`, `shapes`, `sources`), what operations cut
//! (`selector`), the operations (`operations`), and the project's rules (`validate`).
//!
//! All lengths are mm, all feeds mm/min, angles radians; a depth is measured down from the
//! stock top. Unknown fields are rejected; value constraints are checked by
//! [`Job::validate`].

mod drawing;
mod machine;
mod operations;
mod project;
mod selector;
mod shapes;
mod sources;
mod validate;

pub use drawing::*;
pub use machine::*;
pub use operations::*;
pub use project::*;
pub use selector::*;
pub use shapes::*;
pub use sources::*;
pub use validate::*;

use std::collections::BTreeMap;

use chipload_core::chain::ShapeFrom;
use chipload_core::primitives::{Arc, Line, Primitive, distance};
use chipload_core::tolerances::{
    ANGLE_TOL_RAD, ARC_APPROX_TOL_MM, CHAIN_TOL_MM, DEPTH_TOL_MM, REST_TOL_MM,
};

use crate::Problem;
use chipload_i18n::{Digits, Message, Value};
use serde::{Deserialize, Serialize};

/// The schema version this code reads and writes.
pub const CURRENT_VERSION: i64 = 35;

/// The most depth passes a V-carve takes (docs/specs/v-carve.md decision 12).
pub const MAX_VCARVE_PASSES: u32 = 100;

/// A V-carve's *Ignore kinks* (docs/specs/vcarve-smooth.md §3): the largest, and what a new
/// V-carve starts with, degrees.
pub const MAX_IGNORE_KINKS_DEG: f64 = 90.0;
pub const DEFAULT_IGNORE_KINKS_DEG: f64 = 20.0;

/// The oldest schema version that still loads (decision 2026-10-10: versions from before
/// projects, 1-3, were never released and are no longer read).
pub const OLDEST_VERSION: i64 = 4;

#[cfg(test)]
mod tests {
    use super::*;

    fn shape(geometry: Geometry) -> Shape {
        Shape {
            construction: false,
            id: 1,
            layer: "0".into(),
            name: None,
            geometry,
        }
    }

    fn line_starts(shape: &Shape) -> Vec<[f64; 2]> {
        shape
            .segments()
            .iter()
            .map(|s| match s {
                PathSegment::Line(l) => l.start,
                PathSegment::Arc(_) => panic!("expected lines"),
            })
            .collect()
    }

    fn close(a: [f64; 2], b: [f64; 2]) -> bool {
        (a[0] - b[0]).abs() < 1e-12 && (a[1] - b[1]).abs() < 1e-12
    }

    #[test]
    fn a_circle_is_one_full_arc_from_angle_zero() {
        let c = shape(Geometry::Circle {
            center: [5.0, 6.0],
            radius: 2.0,
        });
        assert_eq!(
            c.segments(),
            [PathSegment::Arc(ArcSegment {
                center: [5.0, 6.0],
                radius: 2.0,
                start_angle: 0.0,
                sweep: std::f64::consts::TAU,
            })]
        );
        assert!(c.closed() && c.is_drawn() && c.dxf_handle().is_none());
    }

    #[test]
    fn rectangle_corners_run_counter_clockwise_and_rotate_about_the_centre() {
        let rect = |angle: f64| {
            shape(Geometry::Rectangle {
                center: [10.0, 20.0],
                length: 8.0,
                width: 4.0,
                angle,
                corner_radius: 0.0,
            })
        };
        let flat = line_starts(&rect(0.0));
        assert_eq!(flat, [[6.0, 18.0], [14.0, 18.0], [14.0, 22.0], [6.0, 22.0]]);
        // 90°: length now runs along Y.
        let up = line_starts(&rect(std::f64::consts::FRAC_PI_2));
        let expected = [[12.0, 16.0], [12.0, 24.0], [8.0, 24.0], [8.0, 16.0]];
        assert!(
            up.iter().zip(&expected).all(|(a, b)| close(*a, *b)),
            "{up:?}"
        );
        // 30°: the first corner is (-4, -2) rotated by 30° about the centre.
        let (sin, cos) = 30f64.to_radians().sin_cos();
        let first = line_starts(&rect(30f64.to_radians()))[0];
        assert!(close(
            first,
            [10.0 - 4.0 * cos + 2.0 * sin, 20.0 - 4.0 * sin - 2.0 * cos]
        ));
        // Closed: the last line ends at the first corner.
        let segments = rect(0.3).segments();
        let (PathSegment::Line(first), PathSegment::Line(last)) = (&segments[0], &segments[3])
        else {
            panic!("lines");
        };
        assert_eq!(last.end, first.start);
    }

    fn rounded(length: f64, width: f64, angle: f64, corner_radius: f64) -> Geometry {
        Geometry::Rectangle {
            center: [10.0, 20.0],
            length,
            width,
            angle,
            corner_radius,
        }
    }

    /// Start and end of a segment.
    fn ends(segment: &PathSegment) -> ([f64; 2], [f64; 2]) {
        match segment {
            PathSegment::Line(l) => (l.start, l.end),
            PathSegment::Arc(a) => {
                let at = |t: f64| {
                    [
                        a.center[0] + a.radius * t.cos(),
                        a.center[1] + a.radius * t.sin(),
                    ]
                };
                (at(a.start_angle), at(a.start_angle + a.sweep))
            }
        }
    }

    fn joined(segments: &[PathSegment]) -> bool {
        (0..segments.len()).all(|i| {
            let end = ends(&segments[i]).1;
            let next = ends(&segments[(i + 1) % segments.len()]).0;
            (end[0] - next[0]).hypot(end[1] - next[1]) <= CHAIN_TOL_MM
        })
    }

    /// docs/specs/rectangle-corner-radius.md §2.
    #[test]
    fn rounded_corners_are_quarter_arcs_joined_to_the_sides() {
        let segments = shape(rounded(8.0, 4.0, 0.0, 1.0)).segments();
        assert_eq!(segments.len(), 8);
        assert!(joined(&segments));
        // Starts r right of the lower-left corner, along the bottom.
        assert_eq!(ends(&segments[0]), ([7.0, 18.0], [13.0, 18.0]));
        for arc in segments.iter().skip(1).step_by(2) {
            let PathSegment::Arc(arc) = arc else {
                panic!("arc");
            };
            assert_eq!(arc.radius, 1.0);
            assert_eq!(arc.sweep, std::f64::consts::FRAC_PI_2);
        }
        // The same box as the sharp rectangle.
        let xs: Vec<f64> = segments
            .iter()
            .flat_map(|s| [ends(s).0[0], ends(s).1[0]])
            .collect();
        let lowest = xs.iter().copied().fold(f64::INFINITY, f64::min);
        let highest = xs.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        assert!(close([lowest, highest], [6.0, 14.0]));
        // Turned: the unturned segments turned about the centre.
        let angle = 0.4_f64;
        let (sin, cos) = angle.sin_cos();
        let turn = |p: [f64; 2]| {
            let (x, y) = (p[0] - 10.0, p[1] - 20.0);
            [10.0 + x * cos - y * sin, 20.0 + x * sin + y * cos]
        };
        let turned = shape(rounded(8.0, 4.0, angle, 1.0)).segments();
        assert!(joined(&turned));
        for (a, b) in segments.iter().zip(&turned) {
            assert!(close(turn(ends(a).0), ends(b).0) && close(turn(ends(a).1), ends(b).1));
        }
        // Radius 0: the four lines as before.
        assert_eq!(
            shape(rounded(8.0, 4.0, 0.3, 0.0)).segments(),
            shape(Geometry::Rectangle {
                center: [10.0, 20.0],
                length: 8.0,
                width: 4.0,
                angle: 0.3,
                corner_radius: 0.0,
            })
            .segments()
        );
        assert_eq!(shape(rounded(8.0, 4.0, 0.3, 0.0)).segments().len(), 4);
    }

    #[test]
    fn a_radius_of_half_a_side_leaves_that_side_out() {
        // A slot: the short sides are all arc.
        let slot = shape(rounded(8.0, 4.0, 0.0, 2.0)).segments();
        assert_eq!(slot.len(), 6);
        assert!(joined(&slot));
        // A square at half its side: a circle of four arcs.
        let round = shape(rounded(4.0, 4.0, 0.7, 2.0)).segments();
        assert_eq!(round.len(), 4);
        assert!(
            round
                .iter()
                .all(|s| matches!(s, PathSegment::Arc(a) if a.center == [10.0, 20.0]))
        );
        assert!(joined(&round));
    }

    #[test]
    fn a_corner_radius_is_at_most_half_the_shorter_side() {
        let ok = |r: f64| shape_problems(&rounded(8.0, 4.0, 0.0, r)).is_empty();
        assert!(ok(0.0) && ok(1.0) && ok(2.0) && ok(CHAIN_TOL_MM));
        for bad in [2.0001, -1.0, f64::NAN, f64::INFINITY, CHAIN_TOL_MM / 2.0] {
            assert!(!ok(bad), "{bad}");
        }
        let problems = shape_problems(&rounded(8.0, 4.0, 0.0, 3.0));
        assert!(problems[0].text.contains("at most 2 mm"), "{problems:?}");
    }

    #[test]
    fn polylines_close_only_when_asked() {
        let points = vec![[0.0, 0.0], [4.0, 0.0], [2.0, 3.0]];
        let open = shape(Geometry::Polyline {
            closed: false,
            points: points.clone(),
            bulges: vec![0.0; 2],
        });
        let closed = shape(Geometry::Polyline {
            closed: true,
            points,
            bulges: vec![0.0; 3],
        });
        assert_eq!(open.segments().len(), 2);
        assert_eq!(closed.segments().len(), 3);
        assert!(!open.closed() && closed.closed());
    }

    #[test]
    fn a_bulged_segment_is_the_arc_a_dxf_bulge_describes() {
        let bulged = shape(Geometry::Polyline {
            closed: true,
            points: vec![[0.0, 0.0], [40.0, 0.0], [20.0, 30.0]],
            bulges: vec![0.0, 0.4, 0.0],
        });
        let segments = bulged.segments();
        assert!(matches!(segments[0], PathSegment::Line(_)));
        assert!(matches!(segments[2], PathSegment::Line(_)));
        let PathSegment::Arc(arc) = segments[1] else {
            panic!("arc");
        };
        let expected = Arc::from_bulge((40.0, 0.0), (20.0, 30.0), 0.4).unwrap();
        assert_eq!(arc.center, [expected.center.0, expected.center.1]);
        assert_eq!(arc.radius, expected.radius);
        assert_eq!(arc.start_angle, expected.start_angle);
        assert_eq!(arc.sweep, 4.0 * 0.4f64.atan());
        // It ends on the next point.
        let end_angle = arc.start_angle + arc.sweep;
        let end = [
            arc.center[0] + arc.radius * end_angle.cos(),
            arc.center[1] + arc.radius * end_angle.sin(),
        ];
        assert!((end[0] - 20.0).abs() < 1e-9 && (end[1] - 30.0).abs() < 1e-9);
    }
}
