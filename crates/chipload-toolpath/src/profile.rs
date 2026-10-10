// SPDX-License-Identifier: GPL-3.0-or-later
//! Profile toolpaths around closed contours. See docs/specs/v0-profile-pipeline.md §4.

use chipload_core::Point;
use chipload_core::chain::Contour;
use chipload_core::primitives::{distance, signed_area};
use chipload_core::tolerances::DEPTH_TOL_MM;
use chipload_job::{Direction, ProfileOp, Side, Spindle};

use crate::ToolpathError;
use crate::arcs::ArcTable;
use crate::left::{LeftPiece, cleared, reach, unreachable_inside, unreachable_outside};
use crate::model::Move;
use crate::offset::{contour_pline, offset_rings_tagged, start_index};
use crate::pocket::{Area, ccw, guarded};

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

/// Retract, travel, then for each pass plunge and go once around. Ends retracted.
pub fn contour_moves(
    ring: &[Point],
    pass_z: &[f64],
    safe_z: f64,
    feed: f64,
    plunge_feed: f64,
) -> Vec<Move> {
    let Some(&start) = ring.first() else {
        return Vec::new();
    };
    let mut moves = vec![Move::rapid_z(safe_z), Move::rapid_xy(start)];
    for &z in pass_z {
        moves.push(Move::feed_z(z, plunge_feed));
        moves.extend(ring[1..].iter().map(|&p| Move::feed_xy(p, feed)));
        moves.push(Move::feed_xy(start, feed));
    }
    moves.push(Move::rapid_z(safe_z));
    moves
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
                (distance(here, s), s.1, s.0, r.contour_id)
            };
            let nearest = remaining
                .iter()
                .enumerate()
                .filter(|(_, r)| r.nesting_depth == depth && !r.points.is_empty())
                .min_by(|(_, a), (_, b)| {
                    let (ka, kb) = (key(a), key(b));
                    ka.0.total_cmp(&kb.0)
                        .then(ka.1.total_cmp(&kb.1))
                        .then(ka.2.total_cmp(&kb.2))
                        .then(ka.3.cmp(&kb.3))
                })
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
    /// Material the tool can't reach (inside corners), for display.
    pub left: Vec<LeftPiece>,
    /// Which ring chords lie on arcs (docs/specs/arc-output.md).
    pub arcs: ArcTable,
}

pub fn plan_profile(
    op: &ProfileOp,
    label: &str,
    contours: &[&Contour],
    tool_radius: f64,
    spindle: Spindle,
    start: Point,
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
    let mut left = Vec::new();
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
        // Display only: any failure, even a panic in the geometry library, becomes a note.
        let unreachable = guarded("Material left", || {
            unreachable_beside(contour, op.side, offset, tool_radius)
        })
        .and_then(|result| result);
        match unreachable {
            Ok(pieces) => left.extend(pieces),
            Err(_) => notes.push(format!(
                "{label}, contour #{}: could not compute the material left for display",
                contour.id
            )),
        }
        let depth = contour.nesting_depth.unwrap_or(0);
        rings.extend(paths.iter().map(|p| PathRing {
            points: orient(p, clockwise),
            nesting_depth: depth,
            contour_id: contour.id,
        }));
    }
    Ok(ProfilePlan {
        rings: order_rings(rings, start),
        notes,
        left,
        arcs,
    })
}

/// Material on the cut side of the contour that the tool can't reach (spec material-left.md
/// section 2). Nothing for a cut on the line.
fn unreachable_beside(
    contour: &Contour,
    side: Side,
    offset: f64,
    tool_radius: f64,
) -> Result<Vec<LeftPiece>, ToolpathError> {
    let pline = contour_pline(contour);
    match side {
        Side::On => Ok(Vec::new()),
        Side::Outside => unreachable_outside(&pline, offset, tool_radius),
        Side::Inside => {
            let region = Area::of(ccw(pline));
            let level0 = region.shrunk(offset)?;
            let reached = reach(&level0, tool_radius)?;
            let mut pieces = Vec::new();
            for target in cleared(&region, tool_radius - offset)? {
                pieces.extend(unreachable_inside(&target, &reached)?);
            }
            Ok(pieces)
        }
    }
}
