// SPDX-License-Identifier: GPL-3.0-or-later
//! Where an instance of a part lies (docs/specs/instances.md §2): the part mirrored, then
//! turned, about its centre, then moved. Points, contours and the drawing's other places
//! go into the instance's place with [`Placement::apply`] and back into the part's own
//! frame with [`Placement::inverse`].

use chipload_core::Point;
use chipload_core::chain::Contour;
use chipload_core::primitives::{rotate_point, rotation};
use chipload_job::{Mirror, Spot};

/// One instance's placement, with its turn worked out once.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placement {
    /// The part's centre: what it is mirrored and turned about.
    pub centre: Point,
    pub mirror: Option<Mirror>,
    /// Radians, counter-clockwise.
    pub angle: f64,
    /// The move after the turn, mm.
    pub at: Point,
    /// cos and sin of `angle`, exact on quarter turns.
    turn: (f64, f64),
}

impl Placement {
    pub fn new(spot: &Spot, centre: Point) -> Placement {
        Placement {
            centre,
            mirror: spot.mirror,
            angle: spot.angle,
            at: (spot.at[0], spot.at[1]),
            turn: rotation(spot.angle),
        }
    }

    /// Whether the instance is mirrored: its paths run the other way round, its arcs turn
    /// the other way.
    pub fn mirrors(&self) -> bool {
        self.mirror.is_some()
    }

    /// `p` on the part as drawn, where it lies on the instance.
    pub fn apply(&self, p: Point) -> Point {
        let mirrored = self.mirrored(p);
        let turned = rotate_point(mirrored, self.centre, self.turn);
        (turned.0 + self.at.0, turned.1 + self.at.1)
    }

    /// `p` on the instance, where it lies on the part as drawn: [`Placement::apply`] undone.
    pub fn inverse(&self, p: Point) -> Point {
        let moved = (p.0 - self.at.0, p.1 - self.at.1);
        let (cos, sin) = self.turn;
        let turned = rotate_point(moved, self.centre, (cos, -sin));
        self.mirrored(turned)
    }

    /// A contour on the instance, back where it lies on the part as drawn: what another
    /// part cuts, seen from the part's own frame.
    pub fn inverse_contour(&self, contour: &Contour) -> Contour {
        let moved = contour.translated((-self.at.0, -self.at.1));
        let turned = moved.rotated(self.centre, -self.angle);
        match self.mirror {
            Some(m) => turned.mirrored(m == Mirror::LeftRight, self.mirror_line(m)),
            None => turned,
        }
    }

    fn mirror_line(&self, mirror: Mirror) -> f64 {
        match mirror {
            Mirror::LeftRight => self.centre.0,
            Mirror::TopBottom => self.centre.1,
        }
    }

    fn mirrored(&self, p: Point) -> Point {
        match self.mirror {
            Some(Mirror::LeftRight) => (2.0 * self.centre.0 - p.0, p.1),
            Some(Mirror::TopBottom) => (p.0, 2.0 * self.centre.1 - p.1),
            None => p,
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use std::f64::consts::FRAC_PI_2;

    fn close(a: Point, b: Point) -> bool {
        (a.0 - b.0).abs() < 1e-9 && (a.1 - b.1).abs() < 1e-9
    }

    #[test]
    fn a_quarter_turn_lands_on_round_numbers_and_inverse_undoes_it() {
        let spot = Spot {
            mirror: None,
            angle: FRAC_PI_2,
            at: [100.0, 0.0],
        };
        let place = Placement::new(&spot, (10.0, 10.0));
        // (20, 10) is 10 right of the centre: turned, 10 above it, then moved.
        assert_eq!(place.apply((20.0, 10.0)), (110.0, 20.0));
        assert_eq!(place.inverse((110.0, 20.0)), (20.0, 10.0));
    }

    #[test]
    fn a_contour_brought_back_is_where_its_points_go_back() {
        use chipload_core::chain::{OrientedSegment, Segment};
        use chipload_core::primitives::{Arc, Line, Primitive};
        let segment = |prim| OrientedSegment {
            segment: Segment {
                prim,
                layer: String::new(),
                handle: String::new(),
                source_type: String::new(),
                shape: None,
                from: None,
                construction: false,
            },
            reversed: false,
        };
        let contour = Contour {
            id: 1,
            parts: vec![
                segment(Primitive::Line(Line {
                    start: (0.0, 0.0),
                    end: (30.0, 5.0),
                })),
                segment(Primitive::Arc(Arc {
                    center: (30.0, 15.0),
                    radius: 10.0,
                    start_angle: -FRAC_PI_2,
                    sweep: 2.0,
                })),
            ],
            closed: false,
            invalid_reason: None,
            nesting_depth: None,
        };
        for mirror in [None, Some(Mirror::LeftRight), Some(Mirror::TopBottom)] {
            let place = Placement::new(
                &Spot {
                    mirror,
                    angle: 0.8,
                    at: [120.0, -40.0],
                },
                (12.0, 7.0),
            );
            let back = place.inverse_contour(&contour);
            for (part, original) in back.parts.iter().zip(&contour.parts) {
                let (got, want) = (&part.segment.prim, &original.segment.prim);
                assert!(close(place.apply(got.start()), want.start()), "{mirror:?}");
                assert!(close(place.apply(got.end()), want.end()), "{mirror:?}");
                assert!(
                    close(place.apply(got.midpoint()), want.midpoint()),
                    "{mirror:?}"
                );
            }
        }
    }

    #[test]
    fn mirrored_then_turned_then_moved() {
        let spot = Spot {
            mirror: Some(Mirror::LeftRight),
            angle: 0.3,
            at: [-7.0, 42.0],
        };
        let place = Placement::new(&spot, (3.0, -4.0));
        for p in [(0.0, 0.0), (12.5, -3.25), (-40.0, 17.0)] {
            assert!(close(place.inverse(place.apply(p)), p));
        }
        // Mirrored about X = 3 first: (5, -4) goes to (1, -4), 2 left of the centre.
        let turned = Placement::new(
            &Spot {
                angle: 0.0,
                at: [0.0, 0.0],
                ..spot
            },
            (3.0, -4.0),
        );
        assert_eq!(turned.apply((5.0, -4.0)), (1.0, -4.0));
    }
}
