// SPDX-License-Identifier: GPL-3.0-or-later
//! Where each piece starts: helix fits, chosen starts, and rest pieces.

use cavalier_contours::polyline::PlineSource;
use chipload_core::Point;
use chipload_core::order::Ordered;
use chipload_core::primitives::distance;
use chipload_core::tolerances::ARC_APPROX_TOL_MM;
use chipload_i18n::{Digits, Message, Value};

use super::verify::{clear_of, min_clearance};
use super::{Draft, MAX_RING_LEVELS, PocketArea, PocketRing, fixed3, ids_text};
use crate::ToolpathError;
use crate::arcs::{ArcTable, Circle};
use crate::cavalier::{Area, first_point, shrunk_farther};
use crate::geom::PrimIndex;
use crate::helix::Helix;
use crate::left::{cleared, left_by, rest_areas};
use crate::start::{assign, nearest_ties, point_value, restart};

/// What a helix must fit: the piece, its walls, and how far the tool centre keeps from them.
pub(super) struct HelixFit<'a> {
    pub(super) region: &'a Area,
    pub(super) walls: &'a PrimIndex,
    pub(super) offset: f64,
    pub(super) tool_radius: f64,
    pub(super) clockwise: bool,
}

/// The helix for a piece (docs/specs/helix-and-leads.md part C): round the point furthest
/// from the walls in the area nearest the first path, its circle at least the tool centre's
/// offset from every wall. The radius shrinks to the largest that fits, down to a tenth of
/// the tool diameter; the helix and the feed from it to the first path are checked against
/// the walls. None, with a note, if no helix fits: the piece then ramps in.
pub(super) fn fit_helix(
    fit: &HelixFit,
    radius: f64,
    rings: &[PocketRing],
    ids: &[usize],
    notes: &mut Vec<Message>,
) -> Result<Option<Helix>, ToolpathError> {
    let Some(&first) = rings.first().and_then(|r| r.points.first()) else {
        return Ok(None);
    };
    let min_radius = 0.2 * fit.tool_radius; // a tenth of the diameter
    let without = |why: Message| why.arg("contours", ids_text(ids));
    // The areas a helix centre of radius r may lie in.
    let room = |r: f64| fit.region.shrunk(fit.offset + r);
    let mut used = radius;
    if room(radius)?.is_empty() {
        if room(min_radius)?.is_empty() {
            notes.push(without(
                Message::new("toolpath-no-helix-room").arg("least", fixed3(min_radius)),
            ));
            return Ok(None);
        }
        // The largest radius that fits, to within a thousandth of the range.
        let (mut lo, mut hi) = (min_radius, radius);
        for _ in 0..10 {
            let mid = (lo + hi) / 2.0;
            if room(mid)?.is_empty() {
                hi = mid;
            } else {
                lo = mid;
            }
        }
        used = lo;
        notes.push(
            Message::new("toolpath-helix-reduced")
                .arg("contours", ids_text(ids))
                .arg("used", fixed3(used))
                .arg("radius", fixed3(radius)),
        );
    }
    let areas = room(used)?;
    let nearest = nearest_ties(areas.iter().map(|a| (a, a.distance(first))))
        .into_iter()
        .min_by_key(|a| a.sort_key());
    let Some(nearest) = nearest else {
        notes.push(without(Message::new("toolpath-no-helix-narrow")));
        return Ok(None);
    };
    // Deepest: the last non-empty shrink of that area, a tool radius at a time.
    let mut center = first_point(&nearest.outer);
    for k in 1..MAX_RING_LEVELS {
        let inner = nearest.shrunk(k as f64 * fit.tool_radius)?;
        let Some(area) = inner.first() else {
            break;
        };
        center = first_point(&area.outer);
    }
    // Start on the side towards the first path, so the feed to it is short.
    let towards = match distance(center, first) {
        d if d > 0.0 => ((first.0 - center.0) / d, (first.1 - center.1) / d),
        _ => (1.0, 0.0),
    };
    let helix = Helix {
        circle: Circle {
            center,
            radius: used,
        },
        start: (center.0 + used * towards.0, center.1 + used * towards.1),
        clockwise: fit.clockwise,
    };
    let clearance = min_clearance(fit.offset);
    let points = helix.points();
    let clear = points.iter().all(|&p| fit.region.contains(p))
        && points
            .windows(2)
            .all(|w| clear_of(fit.walls, w[0], w[1], clearance))
        && clear_of(fit.walls, helix.start, first, clearance);
    if !clear {
        notes.push(without(Message::new("toolpath-no-helix-way")));
        return Ok(None);
    }
    Ok(Some(helix))
}

/// Restart each wall ring at the stored point nearest to it (among all the pieces' wall
/// rings), with notes for points far from their ring (more than `far`) and for a second
/// point on one ring.
pub(super) fn choose_starts(
    drafts: &mut [Draft],
    starts: &[Point],
    far: f64,
    arcs: &mut ArcTable,
    notes: &mut Vec<Message>,
) {
    if starts.is_empty() {
        return;
    }
    let mut at: Vec<(usize, usize)> = Vec::new();
    for (d, draft) in drafts.iter().enumerate() {
        at.extend(
            (0..draft.rings.len())
                .filter(|&k| draft.rings[k].wall)
                .map(|k| (d, k)),
        );
    }
    let rings: Vec<&[Point]> = at
        .iter()
        .map(|&(d, k)| drafts[d].rings[k].points.as_slice())
        .collect();
    let (chosen, duplicates) = assign(starts, &rings);
    for (&(d, k), choice) in at.iter().zip(&chosen) {
        let Some(choice) = choice else { continue };
        let ring = &mut drafts[d].rings[k];
        ring.points = restart(&ring.points, choice.stored, arcs);
        ring.chosen = Some(choice.index);
        if choice.distance > far {
            notes.push(
                Message::new("toolpath-start-far-wall")
                    .arg("contours", ids_text(&drafts[d].ids))
                    .arg("point", point_value(choice.stored))
                    .arg("distance", fixed3(choice.distance)),
            );
        }
    }
    for (index, r) in duplicates {
        let (d, _) = at[r];
        notes.push(
            Message::new("toolpath-start-twice-wall")
                .arg("contours", ids_text(&drafts[d].ids))
                .arg("point", point_value(starts[index])),
        );
    }
}

/// What a rest pocket clears (docs/specs/rest-machining.md §3).
pub(super) struct RestPieces {
    /// The areas to pocket, each with its region's contours.
    pub(super) pieces: Vec<PocketArea>,
    /// The larger tool's leftovers in every region.
    left: Vec<Area>,
    pub(super) notes: Vec<Message>,
}

/// The areas a rest pocket clears in each region: within a diameter of what a tool of
/// radius `large` leaves. An area this pocket's tool fits nowhere in is skipped, with a
/// note.
pub(super) fn rest_pieces(
    regions: &[PocketArea],
    large: f64,
    tool_radius: f64,
    clearance: f64,
    offset: f64,
) -> Result<RestPieces, ToolpathError> {
    let mut found = RestPieces {
        pieces: Vec::new(),
        left: Vec::new(),
        notes: Vec::new(),
    };
    // (width, height) of the leftovers too small for this tool too.
    let mut skipped: Vec<(f64, f64)> = Vec::new();
    for region in regions {
        let level0 = shrunk_farther(&region.area, large - clearance)?;
        let mut material = Vec::new();
        for target in cleared(&region.area, clearance)? {
            material.extend(left_by(&target, &level0, large)?);
        }
        for area in rest_areas(&region.area, &material, 2.0 * tool_radius)? {
            if shrunk_farther(&area, offset)?.is_empty() {
                skipped.extend(
                    material
                        .iter()
                        .filter(|m| area.distance(first_point(&m.outer)) <= ARC_APPROX_TOL_MM)
                        .filter_map(|m| m.outer.extents())
                        .map(|e| (e.max_x - e.min_x, e.max_y - e.min_y)),
                );
                continue;
            }
            found.pieces.push(PocketArea {
                area,
                contour_ids: region.contour_ids.clone(),
            });
        }
        found.left.extend(material);
    }
    if found.left.is_empty() {
        found.notes.push(Message::new("toolpath-rest-nothing-left"));
    }
    if let Some(&(w, h)) = skipped.iter().max_by_key(|&&(w, h)| Ordered(w * h)) {
        found.notes.push(
            Message::new("toolpath-rest-too-small")
                .arg("count", skipped.len())
                .arg("width", Value::LengthNumber(w, Digits::Fixed(2)))
                .arg("height", Value::Length(h, Digits::Fixed(2))),
        );
    }
    Ok(found)
}
