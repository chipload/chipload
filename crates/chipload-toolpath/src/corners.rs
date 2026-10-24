// SPDX-License-Identifier: GPL-3.0-or-later
//! Dogbone and T-bone corners (docs/specs/dogbones.md): where a round tool leaves an inside
//! corner round, the ring goes out to a tip from which the tool's edge touches the corner,
//! and back.

use std::f64::consts::PI;

use chipload_core::Point;
use chipload_core::chain::{Contour, OrientedSegment};
use chipload_core::primitives::{Primitive, distance, signed_area};
use chipload_core::tolerances::{ANGLE_TOL_RAD, ARC_APPROX_TOL_MM, CHAIN_TOL_MM};
use chipload_job::Corners;

use crate::ToolpathError;
use crate::geom::segment_distance;

/// An inside corner of a drawn contour, seen from the tool's side.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Corner {
    pub at: Point,
    pub contour_id: usize,
    /// The parts of the contour that meet here (incoming, outgoing).
    pub parts: (usize, usize),
    /// Unit vector from the corner into the tool's side, halving the corner.
    pub bisector: Point,
    /// The corner's angle on the tool's side, radians, 0 < θ < π.
    pub angle: f64,
    /// Unit vector from the corner along the longer wall.
    pub longer: Point,
}

/// A dogbone's or T-bone's way out and back: from the ring vertex to the tip.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Leg {
    pub from: Point,
    pub tip: Point,
    pub corner: Corner,
}

fn sub(a: Point, b: Point) -> Point {
    (a.0 - b.0, a.1 - b.1)
}

fn add(a: Point, b: Point) -> Point {
    (a.0 + b.0, a.1 + b.1)
}

fn scale(a: Point, k: f64) -> Point {
    (a.0 * k, a.1 * k)
}

fn dot(a: Point, b: Point) -> f64 {
    a.0 * b.0 + a.1 * b.1
}

fn cross(a: Point, b: Point) -> f64 {
    a.0 * b.1 - a.1 * b.0
}

fn unit(a: Point) -> Option<Point> {
    let len = a.0.hypot(a.1);
    (len > 0.0).then(|| (a.0 / len, a.1 / len))
}

/// The direction of travel at a part's start and at its end (unit vectors).
fn tangents(part: &OrientedSegment) -> Option<(Point, Point)> {
    let (start, end) = match part.segment.prim {
        Primitive::Line(line) => {
            let d = unit(sub(line.end, line.start))?;
            (d, d)
        }
        Primitive::Arc(arc) => {
            let along = |angle: f64| {
                let sign = arc.sweep.signum();
                (-sign * angle.sin(), sign * angle.cos())
            };
            (along(arc.start_angle), along(arc.start_angle + arc.sweep))
        }
    };
    Some(if part.reversed {
        (scale(end, -1.0), scale(start, -1.0))
    } else {
        (start, end)
    })
}

/// The corners of a closed contour on the tool's side (`waste_inside`: the tool cuts inside
/// the contour) of at most `max_angle` (docs/specs/dogbones.md §1).
pub fn find_corners(contour: &Contour, waste_inside: bool, max_angle: f64) -> Vec<Corner> {
    let n = contour.parts.len();
    if !contour.closed || n < 2 {
        return Vec::new();
    }
    let ccw = signed_area(&contour.points()) > 0.0;
    let mut corners = Vec::new();
    for i in 0..n {
        let j = (i + 1) % n;
        let (Some((_, incoming)), Some((outgoing, _))) =
            (tangents(&contour.parts[i]), tangents(&contour.parts[j]))
        else {
            continue;
        };
        // Signed turn, counter-clockwise positive, as if the contour ran counter-clockwise:
        // the inside is then on the left, and a left turn is a corner of the inside.
        let mut turn = cross(incoming, outgoing).atan2(dot(incoming, outgoing));
        if !ccw {
            turn = -turn;
        }
        let on_tool_side = if waste_inside { turn > 0.0 } else { turn < 0.0 };
        if !on_tool_side || turn.abs() <= ANGLE_TOL_RAD {
            continue;
        }
        let angle = PI - turn.abs();
        if angle > max_angle + ANGLE_TOL_RAD {
            continue;
        }
        let back = scale(incoming, -1.0);
        let Some(bisector) = unit(add(back, outgoing)) else {
            continue;
        };
        let at = contour.parts[j].points()[0];
        let length = |k: usize| contour.parts[k].segment.prim.length();
        let longer = if length(i) >= length(j) - CHAIN_TOL_MM {
            back
        } else {
            outgoing
        };
        corners.push(Corner {
            at,
            contour_id: contour.id,
            parts: (i, j),
            bisector,
            angle,
            longer,
        });
    }
    corners
}

/// The corner moved so its walls are `clearance` into the part (docs/specs/dogbones.md §3).
fn moved_corner(corner: &Corner, clearance: f64) -> Point {
    let half = (corner.angle / 2.0).sin();
    sub(corner.at, scale(corner.bisector, clearance / half))
}

/// Where the tool goes from the ring vertex `v` so its edge touches the corner; None when it
/// touches it already.
pub fn tip(
    corner: &Corner,
    v: Point,
    style: Corners,
    tool_radius: f64,
    clearance: f64,
) -> Option<Point> {
    let c = moved_corner(corner, clearance);
    let w = sub(v, c);
    if w.0.hypot(w.1) <= tool_radius + CHAIN_TOL_MM {
        return None;
    }
    match style {
        Corners::Dogbone { .. } => Some(add(c, scale(unit(w)?, tool_radius))),
        Corners::Tbone { .. } => {
            // Along the longer wall toward the corner: v + t·u at distance r from c.
            let u = scale(corner.longer, -1.0);
            let wu = dot(w, u);
            let disc = (wu * wu - dot(w, w) + tool_radius * tool_radius).max(0.0);
            let t = -wu - disc.sqrt();
            (t > CHAIN_TOL_MM).then(|| add(v, scale(u, t)))
        }
    }
}

/// The ring vertex where the tool stops in `corner`: the vertex nearest to it, if it lies
/// on the tool's side no farther than straight walls would put it (docs/specs/dogbones.md
/// §5). `rings`: (ring index, the vertices that belong to the ring itself).
fn corner_vertex(
    corner: &Corner,
    slots: &[RingSlot],
    offset: f64,
    clearance: f64,
) -> Option<(usize, usize)> {
    let half = (corner.angle / 2.0).sin();
    let c = moved_corner(corner, clearance);
    // Straight walls put the vertex (offset + clearance) / sin(θ/2) from the moved corner;
    // a curved wall moves it a little (5 % for a line meeting a half circle at 90°). A rounded ring (the corner out of reach) has only
    // small turns there, so the vertex must also turn at least half as much as the corner.
    let reach = 1.25 * (offset + clearance) / half + ARC_APPROX_TOL_MM;
    let sharp = (PI - corner.angle) / 2.0;
    let mut best: Option<(f64, usize, usize)> = None;
    for (r, slot) in slots.iter().enumerate() {
        for k in slot.range.clone() {
            let p = slot.points[k];
            let d = distance(p, c);
            if dot(sub(p, corner.at), corner.bisector) <= 0.0 || d > reach {
                continue;
            }
            if slot.turn_at(k) < sharp {
                continue;
            }
            if best.is_none_or(|(bd, _, _)| d < bd) {
                best = Some((d, r, k));
            }
        }
    }
    best.map(|(_, r, k)| (r, k))
}

/// What the corners need from the operation.
#[derive(Debug, Clone, Copy)]
pub struct CornerCut<'a> {
    pub style: Corners,
    pub tool_radius: f64,
    pub clearance: f64,
    /// Every contour of the drawing (moved with the copy), for the legs' check.
    pub everything: &'a [Contour],
    /// Contours a leg may cross besides its own walls (a pocket's other boundaries).
    pub exempt: &'a [usize],
    pub label: &'a str,
}

/// One ring the corners may go into: its points, and the part of them that is the ring
/// itself (without leads). The ring may or may not repeat its start at its end.
pub struct RingSlot<'a> {
    pub points: &'a mut Vec<Point>,
    pub range: std::ops::Range<usize>,
}

impl RingSlot<'_> {
    /// Whether the ring's last point is its start again.
    fn repeats_start(&self) -> bool {
        self.range.end > self.range.start + 1
            && distance(
                self.points[self.range.start],
                self.points[self.range.end - 1],
            ) <= CHAIN_TOL_MM
    }

    /// The ring's turn at its point `k` (radians, 0 straight on), from its neighbours
    /// round the ring.
    fn turn_at(&self, k: usize) -> f64 {
        let start = self.range.start;
        let mut n = self.range.end - start;
        if self.repeats_start() {
            n -= 1;
        }
        if n < 3 {
            return 0.0;
        }
        let i = (k - start) % n;
        let at = |j: usize| self.points[start + j % n];
        let (prev, here, next) = (at(i + n - 1), at(i), at(i + 1));
        let (Some(a), Some(b)) = (unit(sub(here, prev)), unit(sub(next, here))) else {
            return 0.0;
        };
        cross(a, b).atan2(dot(a, b)).abs()
    }
}

/// What [`add_legs`] did.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Added {
    pub legs: Vec<Leg>,
    /// Corners no ring reaches.
    pub unreached: Vec<Corner>,
    /// Points added to each slot (two per leg), to move ranges by.
    pub points: Vec<usize>,
}

/// Add a leg for every corner to the ring that stops in it (docs/specs/dogbones.md §3),
/// check each leg (§6), and return the legs, the corners no ring reaches (§5) and, per ring,
/// how many points were added (two per leg) so ranges can be moved.
pub fn add_legs(
    corners: &[Corner],
    slots: &mut [RingSlot],
    cut: &CornerCut,
) -> Result<Added, ToolpathError> {
    let mut legs = Vec::new();
    let mut unreached = Vec::new();
    let mut added = vec![0; slots.len()];
    let offset = cut.tool_radius - cut.clearance;
    for corner in corners {
        let found = corner_vertex(corner, slots, offset, cut.clearance);
        let Some((r, k)) = found else {
            unreached.push(*corner);
            continue;
        };
        let slot = &mut slots[r];
        let v = slot.points[k];
        let Some(t) = tip(corner, v, cut.style, cut.tool_radius, cut.clearance) else {
            continue;
        };
        let leg = Leg {
            from: v,
            tip: t,
            corner: *corner,
        };
        check_leg(&leg, cut)?;
        // At the ring's start, the leg goes at its end, so the ring still leaves its start
        // along the wall (leads and spirals start there).
        let repeats = slot.repeats_start();
        let at_start = k == slot.range.start || (repeats && k == slot.range.end - 1);
        let tip_index = if at_start && repeats {
            // ... → V, T, V (the repeated start).
            let end = slot.range.end;
            slot.points.splice(end..end, [t, v]);
            end
        } else if at_start {
            // ... → V, T, and the ring closes from T back to its start V.
            let end = slot.range.end;
            slot.points.splice(end..end, [v, t]);
            end + 1
        } else {
            slot.points.splice(k + 1..k + 1, [t, v]);
            k + 1
        };
        slot.range.end += 2;
        // The tip is reached from its vertex and left back to it, nothing else.
        let before = slot.points[tip_index - 1];
        let after = if tip_index + 1 < slot.range.end {
            slot.points[tip_index + 1]
        } else {
            slot.points[slot.range.start]
        };
        if distance(before, v) > CHAIN_TOL_MM || distance(after, v) > CHAIN_TOL_MM {
            return Err(ToolpathError::Geometry(format!(
                "{}: the leg at corner ({:.3}, {:.3}) mm did not go into the ring as out and \
                 back",
                cut.label, corner.at.0, corner.at.1
            )));
        }
        added[r] += 2;
        legs.push(leg);
    }
    Ok(Added {
        legs,
        unreached,
        points: added,
    })
}

/// The leg may cut into the two walls of its corner, nothing else (docs/specs/dogbones.md
/// §6).
fn check_leg(leg: &Leg, cut: &CornerCut) -> Result<(), ToolpathError> {
    let offset = cut.tool_radius - cut.clearance;
    for contour in cut.everything {
        for (i, part) in contour.parts.iter().enumerate() {
            let own = contour.id == leg.corner.contour_id
                && (i == leg.corner.parts.0 || i == leg.corner.parts.1);
            let exempt = contour.id != leg.corner.contour_id && cut.exempt.contains(&contour.id);
            if own || exempt {
                continue;
            }
            if segment_distance(leg.from, leg.tip, &part.segment.prim) < offset - CHAIN_TOL_MM {
                let style = match cut.style {
                    Corners::Dogbone { .. } => "dogbone",
                    Corners::Tbone { .. } => "T-bone",
                };
                return Err(ToolpathError::Geometry(format!(
                    "{}: the {style} at corner ({:.3}, {:.3}) mm would cut into contour #{}; \
                     lower the corners' max angle or change the drawing",
                    cut.label, leg.corner.at.0, leg.corner.at.1, contour.id
                )));
            }
        }
    }
    Ok(())
}

/// Whether `p` lies inside the closed contour (ray casting on its flattened points).
pub fn encloses(contour: &Contour, p: Point) -> bool {
    let ring = contour.points();
    let n = ring.len();
    let mut inside = false;
    for i in 0..n {
        let (a, b) = (ring[i], ring[(i + 1) % n]);
        if (a.1 > p.1) != (b.1 > p.1) {
            let x = a.0 + (p.1 - a.1) / (b.1 - a.1) * (b.0 - a.0);
            if p.0 < x {
                inside = !inside;
            }
        }
    }
    inside
}

/// The note for a corner no ring reaches (docs/specs/dogbones.md §5).
pub fn unreached_note(
    label: &chipload_i18n::Message,
    corner: &Corner,
    style: Corners,
) -> chipload_i18n::Message {
    let style = match style {
        Corners::Dogbone { .. } => "dogbone",
        Corners::Tbone { .. } => "tbone",
    };
    let note = chipload_i18n::Message::new("toolpath-corner-unreached")
        .arg(
            "point",
            chipload_i18n::Value::Point(corner.at.0, corner.at.1, chipload_i18n::Digits::Fixed(3)),
        )
        .arg("style", style);
    crate::profile::labelled_note(label, note)
}
