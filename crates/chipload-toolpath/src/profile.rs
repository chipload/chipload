// SPDX-License-Identifier: GPL-3.0-or-later
//! Profile toolpaths around closed contours. See docs/specs/v0-profile-pipeline.md §4.

use chipload_core::Point;
use chipload_core::chain::Contour;
use chipload_core::primitives::{distance, signed_area};
use chipload_core::tolerances::DEPTH_TOL_MM;
use chipload_job::{Direction, Entry, ProfileOp, Side, Spindle};

use crate::ToolpathError;
use crate::arcs::ArcTable;
use crate::lead::{Leads, roomiest_start};
use crate::left::{LeftPiece, cleared, reach, unreachable_inside, unreachable_outside};
use crate::model::Move;
use crate::offset::{contour_pline, offset_rings_tagged, start_index};
use crate::pocket::{Area, ccw, guarded};
use crate::ramp::{enter, polyline_length};
use crate::start::{assign, point_text, restart};
use crate::tabs::{TabLayout, path_ahead, split_ring, tab_lap, verify as verify_tabs};

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

/// Retract, travel, then for each pass go down (`entry`) and once around. Ends retracted.
/// `top_z` is the Z of the start depth, cut already (docs/specs/ramp-entry.md).
#[allow(clippy::too_many_arguments)]
pub fn contour_moves(
    ring: &[Point],
    pass_z: &[f64],
    top_z: f64,
    safe_z: f64,
    feed: f64,
    plunge_feed: f64,
    entry: Entry,
) -> Result<Vec<Move>, ToolpathError> {
    let Some(&start) = ring.first() else {
        return Ok(Vec::new());
    };
    let path = closed_path(ring);
    let around = polyline_length(&path);
    let mut moves = vec![Move::rapid_z(safe_z), Move::rapid_xy(start)];
    let mut above = (safe_z, top_z);
    for &z in pass_z {
        let (from_z, cleared) = above;
        moves.extend(enter(
            &path,
            around,
            from_z,
            cleared,
            z,
            entry,
            feed,
            plunge_feed,
        )?);
        moves.extend(ring[1..].iter().map(|&p| Move::feed_xy(p, feed)));
        moves.push(Move::feed_xy(start, feed));
        above = (z, z);
    }
    moves.push(Move::rapid_z(safe_z));
    Ok(moves)
}

/// Tabs on a ring: where they are, and the Z of their top.
#[derive(Debug, Clone, Copy)]
pub struct TabsAt<'a> {
    pub layout: &'a TabLayout,
    pub z_tab: f64,
}

/// Each pass with leads (docs/specs/helix-and-leads.md part B): retract, rapid to the
/// lead-in's start, go down there (`entry`; a ramp runs along the lead-in and on along the
/// ring), lead-in, the lap (over the tabs, if any), lead-out, retract. Every pass starts and
/// ends in the waste; the lead-in's start was cut to the previous pass's depth by the
/// previous lead-in. Ends retracted.
#[allow(clippy::too_many_arguments)]
pub fn led_contour_moves(
    ring: &[Point],
    leads: &Leads,
    tabs: Option<TabsAt>,
    pass_z: &[f64],
    top_z: f64,
    safe_z: f64,
    feed: f64,
    plunge_feed: f64,
    entry: Entry,
) -> Result<Vec<Move>, ToolpathError> {
    let (Some(&start), Some(&from)) = (ring.first(), leads.lead_in.first()) else {
        return Ok(Vec::new());
    };
    let whole = closed_path(ring);
    let around = polyline_length(&whole);
    let lead_length = polyline_length(&leads.lead_in);
    let split = tabs.map(|t| split_ring(ring, &t.layout.spans));
    let mut moves = Vec::new();
    // The laps alone, each from a plunge at the start, for the tab verification.
    let mut laps = Vec::new();
    let mut cleared = top_z;
    for &z in pass_z {
        moves.extend([Move::rapid_z(safe_z), Move::rapid_xy(from)]);
        let tabbed = tabs.filter(|t| z < t.z_tab - DEPTH_TOL_MM);
        let (ahead, room) = match (tabbed, &split) {
            (Some(_), Some(path)) => path_ahead(start, path, 0),
            _ => (whole.clone(), around),
        };
        let mut path = leads.lead_in.clone();
        path.extend(ahead.iter().skip(1));
        moves.extend(enter(
            &path,
            lead_length + room,
            safe_z,
            cleared,
            z,
            entry,
            feed,
            plunge_feed,
        )?);
        moves.extend(leads.lead_in[1..].iter().map(|&p| Move::feed_xy(p, feed)));
        let lap = match (tabbed, &split) {
            (Some(t), Some(path)) => tab_lap(path, start, z, t.z_tab, entry, feed, plunge_feed)?,
            _ => {
                let mut lap: Vec<Move> =
                    ring[1..].iter().map(|&p| Move::feed_xy(p, feed)).collect();
                lap.push(Move::feed_xy(start, feed));
                lap
            }
        };
        laps.push(Move::feed_z(z, plunge_feed));
        laps.extend(lap.iter().copied());
        moves.extend(lap);
        moves.extend(leads.lead_out[1..].iter().map(|&p| Move::feed_xy(p, feed)));
        moves.push(Move::rapid_z(safe_z));
        cleared = z;
    }
    if let Some(t) = tabs {
        verify_tabs(&laps, t.layout, start, t.z_tab)?;
    }
    Ok(moves)
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
    // The order is chosen from the planner's starts, so moving a start doesn't reorder.
    let mut rings = order_rings(rings, start);
    let chosen = apply_starts(
        &mut rings,
        starts,
        label,
        2.0 * tool_radius,
        &mut arcs,
        &mut notes,
    );
    if op.lead.is_some() {
        for (ring, choice) in rings.iter_mut().zip(&chosen) {
            if choice.is_none()
                && let Some(at) = roomiest_start(&ring.points)
            {
                ring.points = restart(&ring.points, at, &mut arcs);
            }
        }
    }
    Ok(ProfilePlan {
        rings,
        notes,
        left,
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
