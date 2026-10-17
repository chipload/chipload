// SPDX-License-Identifier: GPL-3.0-or-later
//! Surfacing (docs/specs/surfacing.md): the whole stock top taken down flat in straight
//! passes back and forth along the stock's longer side. Every pass runs from beside the
//! stock to beside it, so the tool only ever goes down in the air.

use chipload_core::Point;
use chipload_core::tolerances::CHAIN_TOL_MM;

use crate::model::Move;

/// How far the tool's edge goes past the stock's sides, across the passes, as a fraction of
/// the tool diameter (docs/specs/surfacing.md §3): a clean edge even where the stock is a
/// little larger than entered, and the tool centre still within the stock grown by its
/// radius, the margin the checker allows.
pub const SIDE_OVERLAP_DIAMETERS: f64 = 0.25;

/// What a surface cuts.
#[derive(Debug, Clone, Copy)]
pub struct SurfaceCut {
    /// The stock's corners in drawing coordinates: (min, max).
    pub stock: (Point, Point),
    pub tool_radius: f64,
    /// The distance between passes, mm.
    pub stepover: f64,
    pub safe_z: f64,
    pub feed: f64,
    pub plunge_feed: f64,
}

/// The passes' positions across the stock (along its shorter side): `lo` and `hi` are that
/// side's ends. The first and last let the tool's edge pass the sides by
/// [`SIDE_OVERLAP_DIAMETERS`]; between them the passes are equal, at most `stepover` apart.
/// One pass in the middle where the tool covers it all.
pub fn pass_positions(lo: f64, hi: f64, tool_radius: f64, stepover: f64) -> Vec<f64> {
    let inset = tool_radius - SIDE_OVERLAP_DIAMETERS * 2.0 * tool_radius;
    let (first, last) = (lo + inset, hi - inset);
    let span = last - first;
    if span <= CHAIN_TOL_MM {
        return vec![(lo + hi) / 2.0];
    }
    // Rounded up: the passes are never farther apart than the stepover.
    let gaps = (span / stepover).ceil().max(1.0);
    let count = gaps as usize;
    (0..=count)
        .map(|k| first + span * k as f64 / gaps)
        .collect()
}

/// A surface's moves at the depths `pass_z` (going down), from above `start` to above where it ends: down beside the stock at
/// the corner nearest `start`, the passes back and forth at each depth, each depth going
/// down beside the stock where the one before ended, then up to the safe height.
pub fn surface_moves(cut: &SurfaceCut, pass_z: &[f64], start: Point) -> (Vec<Move>, Point) {
    let ((x0, y0), (x1, y1)) = cut.stock;
    let r = cut.tool_radius;
    // Along the longer side; a square goes along X.
    let along_x = x1 - x0 >= y1 - y0 - CHAIN_TOL_MM;
    let (u_ends, across) = if along_x {
        ((x0 - r, x1 + r), pass_positions(y0, y1, r, cut.stepover))
    } else {
        ((y0 - r, y1 + r), pass_positions(x0, x1, r, cut.stepover))
    };
    let point = |u: f64, v: f64| if along_x { (u, v) } else { (v, u) };
    // The corner to start at: the one nearest where the tool is.
    let first_v = across.first().copied().unwrap_or(0.0);
    let last_v = across.last().copied().unwrap_or(0.0);
    let corners = [
        (u_ends.0, first_v, false, false),
        (u_ends.1, first_v, true, false),
        (u_ends.0, last_v, false, true),
        (u_ends.1, last_v, true, true),
    ];
    let mut best = corners[0];
    let mut best_d = f64::INFINITY;
    for c in corners {
        let p = point(c.0, c.1);
        let d = (p.0 - start.0).hypot(p.1 - start.1);
        if d < best_d - CHAIN_TOL_MM {
            best = c;
            best_d = d;
        }
    }
    let (_, _, mut at_hi_u, reversed) = best;
    let mut order: Vec<f64> = across.clone();
    if reversed {
        order.reverse();
    }
    let mut moves = vec![
        Move::rapid_z(cut.safe_z),
        Move::rapid_xy(point(best.0, best.1)),
    ];
    let mut end = point(best.0, best.1);
    for &z in pass_z {
        // Down beside the stock: the tool's edge at most touches its side.
        moves.push(Move::feed_z(z, cut.plunge_feed));
        for (k, &v) in order.iter().enumerate() {
            if k > 0 {
                // Across to the next pass, beside the stock's end.
                let u = if at_hi_u { u_ends.1 } else { u_ends.0 };
                moves.push(Move::feed_xy(point(u, v), cut.feed));
            }
            at_hi_u = !at_hi_u;
            let u = if at_hi_u { u_ends.1 } else { u_ends.0 };
            end = point(u, v);
            moves.push(Move::feed_xy(end, cut.feed));
        }
        // The next depth comes back the other way.
        order.reverse();
    }
    moves.push(Move::rapid_z(cut.safe_z));
    (moves, end)
}
