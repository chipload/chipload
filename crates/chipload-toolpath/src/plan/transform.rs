// SPDX-License-Identifier: GPL-3.0-or-later
//! Moving planned cuts: the plug board, mirrored and turned instances, approaches.

use super::{APPROACH_CLEARANCE_MM, Planned};
use crate::model::{Move, MoveKind};
use crate::place::Placement;
use chipload_core::Point;
use chipload_core::chain::Contour;
use chipload_core::math;
use chipload_core::tolerances::DEPTH_TOL_MM;
use chipload_job::{Job, Mirror, Operation, Spindle};

/// The board an inlay plug at operation `index` carves, as a contour round the artwork in
/// the frame the part is planned in (mirrored back for a mirrored part): the stock grown by
/// as much as keeps every wall and the floor pass's end mill clear of its edges, with round
/// corners (docs/specs/vcarve-inlay.md §2.2). Grown by the glue gap's width, g·tan(α/2),
/// the V-bit's walls meet the channel floor at the board's edge; by the end mill's radius
/// more, that floor reaches the edge too, corners included. None for anything but a plug.
pub(super) fn plug_board(
    job: &Job,
    index: usize,
    mirror: Option<(Mirror, f64)>,
) -> Option<Contour> {
    let Some(Operation::Vcarve(vcarve)) = job.operations.get(index) else {
        return None;
    };
    let plug = vcarve.plug?;
    let slope = math::tan(job.tool(vcarve.tool)?.angle? / 2.0);
    let floor = vcarve
        .floor
        .filter(|f| f.enabled)
        .and_then(|f| job.tool(f.tool))
        .map_or(0.0, |t| t.diameter / 2.0);
    let grow = (vcarve.end_depth - plug.start_depth).max(0.0) * slope + floor;
    let (low, high) = job.stock_bounds();
    let (a, b) = match mirror {
        Some(m) => (mirrored_point(m, low), mirrored_point(m, high)),
        None => (low, high),
    };
    let (x0, y0, x1, y1) = (a.0.min(b.0), a.1.min(b.1), a.0.max(b.0), a.1.max(b.1));
    Some(rounded_rectangle((x0, y0), (x1, y1), grow))
}

/// The rectangle (`low`, `high`) grown by `grow` all round, its corners quarter circles,
/// counter-clockwise, as a closed contour of no shape (id 0).
fn rounded_rectangle(low: Point, high: Point, grow: f64) -> Contour {
    use chipload_core::chain::{OrientedSegment, Segment};
    use chipload_core::primitives::{Arc, Line, Primitive};
    use std::f64::consts::FRAC_PI_2;
    let (x0, y0, x1, y1) = (low.0, low.1, high.0, high.1);
    let g = grow;
    let mut prims = vec![
        Primitive::Line(Line {
            start: (x0, y0 - g),
            end: (x1, y0 - g),
        }),
        Primitive::Line(Line {
            start: (x1 + g, y0),
            end: (x1 + g, y1),
        }),
        Primitive::Line(Line {
            start: (x1, y1 + g),
            end: (x0, y1 + g),
        }),
        Primitive::Line(Line {
            start: (x0 - g, y1),
            end: (x0 - g, y0),
        }),
    ];
    if g > 0.0 {
        let corners = [(x1, y0), (x1, y1), (x0, y1), (x0, y0)];
        for (k, &center) in corners.iter().enumerate() {
            let arc = Primitive::Arc(Arc {
                center,
                radius: g,
                start_angle: -FRAC_PI_2 + k as f64 * FRAC_PI_2,
                sweep: FRAC_PI_2,
            });
            prims.insert(2 * k + 1, arc);
        }
    }
    let parts = prims
        .into_iter()
        .map(|prim| OrientedSegment {
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
        })
        .collect();
    Contour {
        id: 0,
        parts,
        closed: true,
        invalid_reason: None,
        nesting_depth: Some(0),
    }
}

/// `p` mirrored about X = at (left to right) or Y = at (top to bottom).
pub(super) fn mirrored_point((mirror, at): (Mirror, f64), p: Point) -> Point {
    match mirror {
        Mirror::LeftRight => (2.0 * at - p.0, p.1),
        Mirror::TopBottom => (p.0, 2.0 * at - p.1),
    }
}

/// One step's plan as cut on a mirrored part (docs/specs/mirror-part.md §3): the moves,
/// their arcs, the tabs and pocket pieces shown with them, and where the tool ends,
/// mirrored. Arcs turn the other way, and so does a helix. The markers the user moves (ring
/// starts, tab centres, unreached corners) stay on the shapes as drawn, where they are
/// stored.
pub(super) fn mirrored_planned(mut planned: Planned, mirror: (Mirror, f64)) -> Planned {
    let flip = |p: Point| mirrored_point(mirror, p);
    for m in &mut planned.moves {
        match mirror.0 {
            Mirror::LeftRight => m.x = m.x.map(|x| 2.0 * mirror.1 - x),
            Mirror::TopBottom => m.y = m.y.map(|y| 2.0 * mirror.1 - y),
        }
        if let Some(circle) = &mut m.circle {
            circle.center = flip(circle.center);
        }
        m.kind = match m.kind {
            MoveKind::ArcCw => MoveKind::ArcCcw,
            MoveKind::ArcCcw => MoveKind::ArcCw,
            kind => kind,
        };
    }
    planned.arcs = planned.arcs.mapped(flip);
    for tab in &mut planned.tabs {
        for p in tab.iter_mut() {
            *p = flip(*p);
        }
    }
    for piece in &mut planned.pockets {
        for ring in &mut piece.rings {
            for p in &mut ring.points {
                *p = flip(*p);
            }
            for p in ring.unstepped.iter_mut().flatten() {
                *p = flip(*p);
            }
        }
        if let Some(helix) = &mut piece.helix {
            helix.circle.center = flip(helix.circle.center);
            helix.start = flip(helix.start);
            helix.clockwise = !helix.clockwise;
        }
    }
    planned.end = flip(planned.end);
    planned
}

/// The other cutting sense.
pub(super) fn turned_sense(sense: Spindle) -> Spindle {
    match sense {
        Spindle::Cw => Spindle::Ccw,
        Spindle::Ccw => Spindle::Cw,
    }
}

/// One step's plan, made on the part as drawn, put where its instance lies
/// (docs/specs/instances.md §3): the moves, their arcs, the tabs and pocket pieces shown
/// with them, and where the tool ends. Turning mixes X and Y, so a move that gave only one
/// of them gets both, from where the tool was; `from` is where the step started, on the
/// part as drawn. A mirrored instance's arcs and helixes turn the other way. The markers the
/// user moves stay on the part (copy 1).
pub(super) fn placed_planned(mut planned: Planned, place: &Placement, from: Point) -> Planned {
    let put = |p: Point| place.apply(p);
    let mirrors = place.mirrors();
    let (mut x, mut y) = from;
    for m in &mut planned.moves {
        if m.x.is_some() || m.y.is_some() {
            x = m.x.unwrap_or(x);
            y = m.y.unwrap_or(y);
            let (px, py) = put((x, y));
            (m.x, m.y) = (Some(px), Some(py));
        }
        if let Some(circle) = &mut m.circle {
            circle.center = put(circle.center);
        }
        if mirrors {
            m.kind = match m.kind {
                MoveKind::ArcCw => MoveKind::ArcCcw,
                MoveKind::ArcCcw => MoveKind::ArcCw,
                kind => kind,
            };
        }
    }
    planned.arcs = planned.arcs.mapped(put);
    for tab in &mut planned.tabs {
        for p in tab.iter_mut() {
            *p = put(*p);
        }
    }
    for piece in &mut planned.pockets {
        for ring in &mut piece.rings {
            for p in &mut ring.points {
                *p = put(*p);
            }
            for p in ring.unstepped.iter_mut().flatten() {
                *p = put(*p);
            }
        }
        if let Some(helix) = &mut piece.helix {
            helix.circle.center = put(helix.circle.center);
            helix.start = put(helix.start);
            helix.clockwise ^= mirrors;
        }
    }
    planned.end = put(planned.end);
    planned
}

/// Every plunge from safe Z gets a rapid down to the approach height first
/// (docs/specs/approach.md): a feed straight down that starts at safe Z and ends below
/// `top_z + APPROACH_CLEARANCE_MM`. Nothing goes below the stock top as a rapid, and a
/// safe Z at or below the approach height leaves the moves as they are.
pub(super) fn lower_approaches(moves: Vec<Move>, top_z: f64, safe_z: f64) -> Vec<Move> {
    let approach = top_z + APPROACH_CLEARANCE_MM;
    if safe_z <= approach + DEPTH_TOL_MM {
        return moves;
    }
    let mut out = Vec::with_capacity(moves.len() + 8);
    let mut z: Option<f64> = None;
    for m in moves {
        let plunge = m.kind == MoveKind::Feed && m.x.is_none() && m.y.is_none();
        if plunge
            && let (Some(from), Some(to)) = (z, m.z)
            && (from - safe_z).abs() <= DEPTH_TOL_MM
            && to < approach - DEPTH_TOL_MM
        {
            out.push(Move::rapid_z(approach));
        }
        if let Some(mz) = m.z {
            z = Some(mz);
        }
        out.push(m);
    }
    out
}
