// SPDX-License-Identifier: GPL-3.0-or-later
//! Chamfer (docs/specs/chamfer.md): a V-bit breaks the top edge of closed shapes. The tool
//! centre runs half the bit's tip into the waste, so its flank passes through the line; it
//! goes down as deep as the width over the tangent of the half angle.
//!
//! No part is cut beyond the chamfer: every centre point is at least t/2 from the line, so a
//! point of the part the cone reaches h above its tip is within h·tan θ of the line, which
//! is the chamfer's face there (§2).

use chipload_core::Point;
use chipload_core::chain::Contour;
use chipload_core::tolerances::CHAIN_TOL_MM;
use chipload_i18n::Message;
use chipload_job::{ChamferOp, Side, Spindle};

use crate::ToolpathError;
use crate::arcs::ArcTable;
use crate::offset::offset_rings_tagged;
use crate::path::FollowedPath;
use crate::profile::{PathRing, contour_context, contour_note, cut_clockwise, order_rings, orient};

/// The chamfer's tool-centre rings, in cutting order from `start`, as closed paths; notes
/// for a shape whose path splits at a neck narrower than the bit's tip.
pub struct ChamferRings {
    pub paths: Vec<FollowedPath>,
    pub notes: Vec<Message>,
    pub arcs: ArcTable,
}

/// The rings for `contours` with a bit whose flat tip is `tip_diameter` wide: each shape's
/// line offset half the tip into the waste (on the line for a pointed bit), turned for the
/// cutting direction (as a profile on the same side), holes before the outlines round them.
pub fn chamfer_rings(
    op: &ChamferOp,
    label: &Message,
    contours: &[&Contour],
    tip_diameter: f64,
    spindle: Spindle,
    start: Point,
) -> Result<ChamferRings, ToolpathError> {
    let side = op.side.side();
    let clockwise = cut_clockwise(side, op.direction, spindle);
    let offset = tip_diameter.max(0.0) / 2.0;
    let mut arcs = ArcTable::default();
    let mut notes = Vec::new();
    let mut rings = Vec::new();
    for contour in contours {
        let found = if offset > CHAIN_TOL_MM {
            offset_rings_tagged(contour, side, offset, &mut arcs)
        } else {
            offset_rings_tagged(contour, Side::On, 0.0, &mut arcs)
        }
        .map_err(|err| err.with_context(contour_context(label, contour.id)))?;
        if found.len() > 1 {
            notes.push(contour_note(
                label,
                contour.id,
                Message::new("toolpath-narrow-loops").arg("loops", found.len()),
            ));
        }
        let depth = contour.nesting_depth.unwrap_or(0);
        rings.extend(found.iter().map(|ring| PathRing {
            points: orient(ring, clockwise),
            nesting_depth: depth,
            contour_id: contour.id,
        }));
    }
    let paths = order_rings(rings, start)
        .into_iter()
        .map(|ring| FollowedPath {
            points: ring.points,
            closed: true,
            contour_id: ring.contour_id,
        })
        .collect();
    Ok(ChamferRings { paths, notes, arcs })
}
