// SPDX-License-Identifier: GPL-3.0-or-later
//! Profile toolpaths around closed contours. See docs/specs/v0-profile-pipeline.md §4.

use chipload_core::Point;
use chipload_core::chain::Contour;
use chipload_core::order::Ordered;
use chipload_core::primitives::{distance, signed_area};
use chipload_core::tolerances::DEPTH_TOL_MM;
use chipload_job::{Direction, ProfileOp, Side, Spindle};

use crate::ToolpathError;
use crate::arcs::ArcTable;
use crate::offset::{offset_rings_tagged, start_index};
use crate::start::{assign, point_text, restart};

/// Travel direction around the ring.
///
/// With a clockwise (M3) spindle, climb milling keeps the material on the right of the
/// direction of travel. Outside (and `on`, by convention) has the part on the inside of the
/// loop, so climb is clockwise; inside has the material outside the loop, so climb is
/// counter-clockwise. Conventional and a counter-clockwise spindle each flip the result.
pub fn cut_clockwise(side: Side, direction: Direction, spindle: Spindle) -> bool {
    let clockwise = (side != Side::Inside) == (direction == Direction::Climb);
    if spindle == Spindle::Cw {
        clockwise
    } else {
        !clockwise
    }
}

/// Order the ring for the travel direction and start at the lowest (then leftmost) vertex.
pub fn orient(ring: &[Point], clockwise: bool) -> Vec<Point> {
    let mut ordered = ring.to_vec();
    if (signed_area(ring) < 0.0) != clockwise {
        ordered.reverse();
    }
    let start = start_index(&ordered);
    ordered.rotate_left(start);
    ordered
}

/// Equal passes no deeper than step_down; the last one is exactly `depth`.
pub fn pass_depths(
    start_depth: f64,
    end_depth: f64,
    step_down: f64,
) -> Result<Vec<f64>, ToolpathError> {
    let range = end_depth - start_depth;
    if !(start_depth >= 0.0 && range > 0.0 && step_down > 0.0) {
        return Err(ToolpathError::Geometry(format!(
            "depths must satisfy 0 <= start ({start_depth}) < end ({end_depth}) and step_down \
             ({step_down}) must be positive"
        )));
    }
    // Small positive whole number: depths and step_down are validated job values.
    let count = ((range - DEPTH_TOL_MM) / step_down).ceil().max(1.0) as u32;
    // The last pass is exactly the end depth (start + range can differ in the last bit).
    Ok((1..=count)
        .map(|i| {
            if i == count {
                end_depth
            } else {
                start_depth + range * f64::from(i) / f64::from(count)
            }
        })
        .collect())
}

/// A closed ring as a path that returns to its start.
pub fn closed_path(ring: &[Point]) -> Vec<Point> {
    let mut path = ring.to_vec();
    if let Some(&first) = ring.first() {
        path.push(first);
    }
    path
}

/// An oriented tool-centre ring, starting at points[0].
#[derive(Debug, Clone, PartialEq)]
pub struct PathRing {
    pub points: Vec<Point>,
    pub nesting_depth: usize,
    pub contour_id: usize,
}

/// Deepest nesting first (holes before the outline that contains them), then greedy
/// nearest-neighbour from the previous ring's start. Ties break on start y, x, contour id.
pub fn order_rings(rings: Vec<PathRing>, start: Point) -> Vec<PathRing> {
    let mut depths: Vec<usize> = rings.iter().map(|r| r.nesting_depth).collect();
    depths.sort_unstable();
    depths.dedup();
    let mut remaining = rings;
    let mut ordered = Vec::with_capacity(remaining.len());
    let mut position = start;
    for depth in depths.into_iter().rev() {
        loop {
            let here = position;
            let key = |r: &PathRing| {
                let s = r.points[0];
                (
                    Ordered(distance(here, s)),
                    Ordered(s.1),
                    Ordered(s.0),
                    r.contour_id,
                )
            };
            let nearest = remaining
                .iter()
                .enumerate()
                .filter(|(_, r)| r.nesting_depth == depth && !r.points.is_empty())
                .min_by_key(|(_, r)| key(r))
                .map(|(i, _)| i);
            let Some(index) = nearest else { break };
            let ring = remaining.remove(index);
            position = ring.points[0];
            ordered.push(ring);
        }
    }
    ordered
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProfilePlan {
    pub rings: Vec<PathRing>,
    /// Warnings for the user, e.g. material left in a narrow neck.
    pub notes: Vec<String>,
    /// Warnings about the operation's start points: one far from its ring, two on one.
    pub start_notes: Vec<String>,
    /// Which ring chords lie on arcs (docs/specs/arc-output.md).
    pub arcs: ArcTable,
    /// For each ring, the index in `starts` of the point that set its start.
    pub chosen: Vec<Option<usize>>,
}

/// `starts`: the operation's start points, moved with the copy (docs/specs/helix-and-leads.md
/// part A).
pub fn plan_profile(
    op: &ProfileOp,
    label: &str,
    contours: &[&Contour],
    tool_radius: f64,
    spindle: Spindle,
    start: Point,
    starts: &[Point],
) -> Result<ProfilePlan, ToolpathError> {
    let clockwise = cut_clockwise(op.side, op.direction, spindle);
    // The tool centre runs this far from the contour (docs/specs/depths-and-clearance.md).
    let offset = tool_radius - op.clearance;
    if op.side != Side::On && (offset <= 0.0 || offset.is_nan()) {
        return Err(ToolpathError::Job(format!(
            "{label}: clearance {} must be less than the tool radius {tool_radius}",
            op.clearance
        )));
    }
    let mut rings = Vec::new();
    let mut notes = Vec::new();
    let mut arcs = ArcTable::default();
    for contour in contours {
        let paths = offset_rings_tagged(contour, op.side, offset, &mut arcs)
            .map_err(|err| err.with_context(&format!("{label}, contour #{}", contour.id)))?;
        if paths.len() > 1 {
            notes.push(format!(
                "{label}, contour #{}: tool does not fit through a narrow part; cut as {} \
                 separate loops and material remains in between",
                contour.id,
                paths.len()
            ));
        }
        let depth = contour.nesting_depth.unwrap_or(0);
        rings.extend(paths.iter().map(|p| PathRing {
            points: orient(p, clockwise),
            nesting_depth: depth,
            contour_id: contour.id,
        }));
    }
    // The order is chosen from the planner's starts, so moving a start doesn't reorder.
    let mut rings = order_rings(rings, start);
    let mut start_notes = Vec::new();
    let chosen = apply_starts(
        &mut rings,
        starts,
        label,
        2.0 * tool_radius,
        &mut arcs,
        &mut start_notes,
    );
    Ok(ProfilePlan {
        rings,
        notes,
        start_notes,
        arcs,
        chosen,
    })
}

/// Restart each ring at the stored point nearest to it, with notes for points far from
/// their ring (more than `far`) and for a second point on one ring.
fn apply_starts(
    rings: &mut [PathRing],
    starts: &[Point],
    label: &str,
    far: f64,
    arcs: &mut ArcTable,
    notes: &mut Vec<String>,
) -> Vec<Option<usize>> {
    let all: Vec<&[Point]> = rings.iter().map(|r| r.points.as_slice()).collect();
    let (chosen, duplicates) = assign(starts, &all);
    for (ring, choice) in rings.iter_mut().zip(&chosen) {
        let Some(choice) = choice else { continue };
        ring.points = restart(&ring.points, choice.stored, arcs);
        if choice.distance > far {
            notes.push(format!(
                "{label}: start point {} is {:.3} mm from contour #{}; the nearest point is used",
                point_text(choice.stored),
                choice.distance,
                ring.contour_id
            ));
        }
    }
    for (index, r) in duplicates {
        notes.push(format!(
            "{label}: start point {} is on contour #{} too, which already has one; it is \
             ignored",
            point_text(starts[index]),
            rings[r].contour_id
        ));
    }
    chosen.iter().map(|c| c.map(|c| c.index)).collect()
}
