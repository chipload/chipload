// SPDX-License-Identifier: GPL-3.0-or-later
//! Pocket clearing: contour-parallel rings, cut from the inside out. See
//! docs/specs/pocket.md.
//!
//! Offsets use cavalier_contours shapes (outer loop plus islands, true arcs). Every result is
//! verified afterwards, and uncut material is found with exact distances, not by buffering
//! the paths.

use chipload_core::Point;
use chipload_core::chain::Contour;
use chipload_i18n::{Digits, Message, Value};
use chipload_job::{Direction, Side, Spindle};

use crate::ToolpathError;
use crate::arcs::ArcTable;
use crate::cancel::{self, Cancel};
use crate::cavalier::{Area, geometry};
use crate::geom::PrimIndex;
use crate::helix::Helix;
use crate::lead::{Leads, MIN_LEAD_DIAMETERS, fitted_leads, roomiest_start};
use crate::profile::cut_clockwise;
use crate::start::{nearest_ties, restart};

mod moves;
mod order;
mod region;
mod rounding;
mod starts;
mod verify;
use order::step_out;
use rounding::{Attempt, Clearing, cleared_piece};
use starts::{HelixFit, choose_starts, fit_helix, rest_pieces};
use verify::{back_link_is_safe, link_is_safe, uncut_material, verify};

pub use moves::{PocketCut, pocket_moves};
pub use region::build_region;

/// Guard against runaway loops on absurd input; a 2 m pocket at 0.1 mm stepover is 10 000.
pub const MAX_RING_LEVELS: usize = 20_000;

/// One path of a piece: a ring closed back to its start, or an open path, with what the
/// planner decided about it.
#[derive(Debug, Clone, PartialEq)]
pub struct PocketRing {
    /// Oriented and starting at points[0]; a closed ring also ends there.
    pub points: Vec<Point>,
    /// An open path ending at its last point, not closed back to its start.
    pub open: bool,
    /// A wall ring (the last ring along a wall or island at each depth), whose start the
    /// user can move.
    pub wall: bool,
    /// The index in the operation's `starts` of the point that set its start
    /// (docs/specs/helix-and-leads.md part A).
    pub chosen: Option<usize>,
    /// A wall ring with a lead-in before it and a lead-out after it, as one open path: the
    /// ring itself is `points[range]` (part B).
    pub lead: Option<std::ops::Range<usize>>,
    /// The ring steps out onto the next one (the ring outside it) over its last stretch
    /// instead of closing, so the two are one spiral (docs/specs/pocket-spiral.md). Set by
    /// the cut order where the next ring is the one outside; true only where it was done.
    pub steps_out: bool,
    /// No ring inside it is cut before it: it cuts full width, a slot, and goes down in slot
    /// steps (docs/specs/cutting-data.md §4).
    pub full_width: bool,
    /// The closed ring it was before it stepped out: what its slot steps cut.
    pub unstepped: Option<Vec<Point>>,
}

impl PocketRing {
    /// A closed ring whose start the planner chose; `wall` says whether it runs along a
    /// wall or island.
    pub fn closed(points: Vec<Point>, wall: bool) -> Self {
        PocketRing {
            points,
            open: false,
            wall,
            chosen: None,
            lead: None,
            steps_out: false,
            full_width: false,
            unstepped: None,
        }
    }

    /// Where the tool is after cutting it: back at its start if closed, at its end if open.
    pub fn end(&self) -> Option<Point> {
        if self.open {
            self.points.last()
        } else {
            self.points.first()
        }
        .copied()
    }

    /// The ring itself, without its leads.
    pub fn plain(&self) -> &[Point] {
        match &self.lead {
            Some(range) => self.points.get(range.clone()).unwrap_or(&[]),
            None => &self.points,
        }
    }
}

/// One connected area to clear, with its paths in cut order.
#[derive(Debug, Clone, PartialEq)]
pub struct PocketPiece {
    pub rings: Vec<PocketRing>,
    /// feed_links[i]: stay at depth from the end of rings[i] into rings[i + 1].
    pub feed_links: Vec<bool>,
    /// Selected contours that make up this piece, for messages.
    pub contour_ids: Vec<usize>,
    /// The helix each depth pass goes down round, then feeds at depth to the first ring's
    /// start (part C); None: the first path's own entry.
    pub helix: Option<Helix>,
    /// Between depth passes the tool stays down: from the last ring's end, lifted off the
    /// floor, back to where the next pass starts (docs/specs/pocket-stay-down.md). False:
    /// it retracts to safe Z.
    pub back_link: bool,
}

/// How far the tool lifts off the floor just cut to move back between depth passes
/// (docs/specs/pocket-stay-down.md decision 2).
const BACK_LINK_LIFT_MM: f64 = 0.5;

#[derive(Debug, Clone, PartialEq)]
pub struct PocketPlan {
    pub pieces: Vec<PocketPiece>,
    /// Warnings: narrow parts, uncut material.
    pub notes: Vec<Message>,
    /// Which ring chords lie on arcs (docs/specs/arc-output.md).
    pub arcs: ArcTable,
}

/// A length with 3 decimals, for notes.
fn fixed3(mm: f64) -> Value {
    Value::Length(mm, Digits::Fixed(3))
}

fn ids_text(ids: &[usize]) -> String {
    ids.iter()
        .map(|i| format!("#{i}"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// One selected boundary minus the selected islands inside it.
#[derive(Debug, Clone)]
pub struct PocketArea {
    pub area: Area,
    /// The boundary first, then its islands.
    pub contour_ids: Vec<usize>,
}

/// What a pocket is cleared with (docs/specs/pocket.md): the shapes, the tool and how.
#[derive(Debug, Clone, Copy)]
pub struct PocketRequest<'a> {
    /// The selected closed contours: boundaries and the islands inside them.
    pub contours: &'a [&'a Contour],
    pub tool_radius: f64,
    /// Moves the cut into the kept material (docs/specs/depths-and-clearance.md): the tool
    /// centre runs `tool_radius - clearance` from the walls and islands.
    pub clearance: f64,
    /// Between rings, mm.
    pub stepover: f64,
    pub direction: Direction,
    pub spindle: Spindle,
    /// Where the tool comes from.
    pub start: Point,
    /// Start points, leads, helix, rounded corners and rest machining
    /// (docs/specs/helix-and-leads.md); none by default.
    pub choices: Choices<'a>,
}

impl<'a> PocketRequest<'a> {
    /// A pocket cut climb with a clockwise spindle from the origin, the tool centre
    /// `tool_radius` from the walls, with no choices made.
    pub fn new(contours: &'a [&'a Contour], tool_radius: f64, stepover: f64) -> Self {
        PocketRequest {
            contours,
            tool_radius,
            clearance: 0.0,
            stepover,
            direction: Direction::Climb,
            spindle: Spindle::Cw,
            start: (0.0, 0.0),
            choices: Choices::default(),
        }
    }
}

/// Where the wall rings start, their leads, and the helix the tool goes down round
/// (docs/specs/helix-and-leads.md); the default is none of them, and sharp rings.
#[derive(Debug, Clone, Copy, Default)]
pub struct Choices<'a> {
    /// Start points, moved with the copy (part A).
    pub starts: &'a [Point],
    /// Lead radius (part B).
    pub lead: Option<f64>,
    /// Helix radius (part C).
    pub helix: Option<f64>,
    /// Round the inner rings' corners (docs/specs/pocket-round-corners.md). Every project
    /// pocket has it; off is for comparing with sharp rings in tests.
    pub round_corners: bool,
    /// Rings step out onto the next as one spiral (docs/specs/pocket-spiral.md). Every
    /// project pocket has it; off is for tests that measure the rings themselves.
    pub spiral: bool,
    /// Stop with [`ToolpathError::Cancelled`] once set (docs/specs/background-generation.md).
    pub cancel: Option<&'a Cancel>,
    /// Rest machining: the radius of the larger tool whose leftovers are all this pocket
    /// clears (docs/specs/rest-machining.md).
    pub rest: Option<f64>,
    /// A V-carve's floor pass (docs/specs/vcarve-wide-strokes.md §4): a region the tool
    /// doesn't fit in is left to the V-bit, with a note when it has a floor, instead of
    /// refusing the operation.
    pub floor: bool,
}

/// One piece's paths before its links are chosen and it is verified.
struct Draft {
    region: Area,
    ids: Vec<usize>,
    rings: Vec<PocketRing>,
    level0: Vec<Area>,
}

/// Rings, cut order and links for every piece of the pocket `request` asks for.
pub fn plan_pocket(request: &PocketRequest) -> Result<PocketPlan, ToolpathError> {
    let PocketRequest {
        contours,
        tool_radius,
        clearance,
        stepover,
        direction,
        spindle,
        start,
        choices:
            Choices {
                starts,
                lead,
                helix,
                round_corners,
                spiral,
                cancel,
                rest,
                floor,
            },
    } = *request;
    if stepover <= 0.0 || stepover.is_nan() {
        return Err(geometry(
            Message::new("toolpath-error-stepover")
                .arg("got", Value::Number(stepover, Digits::Shortest)),
        ));
    }
    let offset = tool_radius - clearance;
    if offset <= 0.0 || offset.is_nan() {
        return Err(geometry(
            Message::new("toolpath-clearance-radius")
                .arg("clearance", Value::Length(clearance, Digits::Shortest))
                .arg("radius", Value::Length(tool_radius, Digits::Shortest)),
        ));
    }
    let mut remaining = build_region(contours)?;
    if remaining.is_empty() {
        let mut ids: Vec<usize> = contours.iter().map(|c| c.id).collect();
        ids.sort_unstable();
        return Err(geometry(
            Message::new("toolpath-error-nothing-to-clear").arg("contours", ids_text(&ids)),
        ));
    }
    let mut pieces = Vec::new();
    let mut notes = Vec::new();
    let mut arcs = ArcTable::default();
    // Rest machining: only the larger tool's leftovers are cleared.
    if let Some(large) = rest {
        let found = rest_pieces(&remaining, large, tool_radius, clearance, offset)?;
        remaining = found.pieces;
        notes.extend(found.notes);
    }
    let mut position = start;
    let mut drafts: Vec<Draft> = Vec::new();
    while !remaining.is_empty() {
        cancel::check(cancel)?;
        let here = position;
        // The nearest region; equally near ones (`nearest_ties`) by place and contours.
        let index =
            nearest_ties((0..remaining.len()).map(|i| (i, remaining[i].area.distance(here))))
                .into_iter()
                .min_by_key(|&i| {
                    (
                        remaining[i].area.sort_key(),
                        remaining[i].contour_ids.clone(),
                    )
                })
                .unwrap_or(0);
        let PocketArea {
            area: region,
            contour_ids: ids,
        } = remaining.remove(index);
        if floor && region.shrunk(offset)?.is_empty() {
            // Only a region with a floor is worth a word: narrow strokes have none.
            if !region.shrunk(-clearance)?.is_empty() {
                notes.push(
                    Message::new("toolpath-floor-no-fit")
                        .arg("contours", ids_text(&ids))
                        .arg("diameter", Value::mm(2.0 * tool_radius)),
                );
            }
            continue;
        }
        let clearing = Clearing {
            round_corners,
            offset,
            tool_radius,
            stepover,
            position,
            direction,
            spindle,
            cancel,
        };
        let Attempt {
            rings,
            level0,
            arcs: used_arcs,
            notes: used_notes,
        } = cleared_piece(&region, &ids, &clearing)?;
        arcs.extend(used_arcs);
        notes.extend(used_notes);
        // A floor pass's narrow parts are in the V-carve's coverage note.
        if level0.len() > 1 && !floor {
            notes.push(
                Message::new("toolpath-pocket-narrow")
                    .arg("contours", ids_text(&ids))
                    .arg("areas", level0.len()),
            );
        }
        position = rings[0].points[0]; // every depth starts again at the first ring
        drafts.push(Draft {
            region,
            ids,
            rings,
            level0,
        });
    }
    // The order and the clearing were chosen from the planner's starts; only the wall
    // rings move to the user's.
    choose_starts(
        &mut drafts,
        starts,
        2.0 * tool_radius,
        &mut arcs,
        &mut notes,
    );
    for draft in drafts {
        let Draft {
            region,
            ids,
            mut rings,
            level0,
        } = draft;
        let walls = PrimIndex::new(region.boundary(), tool_radius.max(1.0));
        if let Some(radius) = lead {
            // Climb with M3 keeps the walls on the right: the cleared side is on the left.
            let waste_left = !cut_clockwise(Side::Inside, direction, spindle);
            let fits =
                |l: &Leads| l.all_points(|p| region.contains(p)) && l.clear_of(&walls, offset);
            let min_radius = 2.0 * tool_radius * MIN_LEAD_DIAMETERS;
            for ring in rings.iter_mut().filter(|r| r.wall) {
                if ring.chosen.is_none()
                    && let Some(at) = roomiest_start(&ring.points)
                {
                    ring.points = restart(&ring.points, at, &mut arcs);
                }
                match fitted_leads(&ring.points, radius, min_radius, waste_left, fits) {
                    Some((l, used)) => {
                        if used < radius {
                            notes.push(
                                Message::new("toolpath-lead-reduced")
                                    .arg("contours", ids_text(&ids))
                                    .arg("used", fixed3(used))
                                    .arg("radius", fixed3(radius)),
                            );
                        }
                        l.tag(&mut arcs);
                        let plain = std::mem::take(&mut ring.points);
                        let before = l.lead_in.len() - 1;
                        let mut path = l.lead_in[..before].to_vec();
                        path.extend(plain.iter().copied());
                        path.extend(l.lead_out.iter().copied());
                        ring.lead = Some(before..before + plain.len());
                        ring.points = path;
                        ring.open = true;
                    }
                    None => notes.push(
                        Message::new("toolpath-no-lead")
                            .arg("contours", ids_text(&ids))
                            .arg("least", fixed3(min_radius)),
                    ),
                }
            }
        }
        // Rings step out onto the next where they can (docs/specs/pocket-spiral.md); if
        // that leaves material the uncut check finds, the piece is cut with closed rings
        // and links instead, as before.
        let plain = rings.clone();
        let spiralled = spiral
            && step_out(
                &mut rings,
                &mut arcs,
                stepover,
                2.0 * tool_radius,
                &region,
                &walls,
                offset,
            );
        let linked = |rings: &[PocketRing]| -> Result<(Vec<bool>, Option<Message>), ToolpathError> {
            // Between rings the tool stays down wherever the link is safe (docs/specs/pocket.md
            // as built 2026-10-15): verified with the rings.
            let links: Vec<bool> = rings
                .windows(2)
                .map(|pair| {
                    pair[0].end().is_some_and(|from| {
                        link_is_safe(
                            &region,
                            &walls,
                            from,
                            pair[1].points[0],
                            tool_radius,
                            offset,
                        )
                    })
                })
                .collect();
            verify(&region, &walls, rings, &links, offset, &ids, &arcs)?;
            let (uncut, _) =
                uncut_material(&region, &level0, rings, &links, tool_radius, &ids, cancel)?;
            Ok((links, uncut))
        };
        let (mut links, mut uncut) = linked(&rings)?;
        if spiralled && uncut.is_some() {
            let (plain_links, plain_uncut) = linked(&plain)?;
            if plain_uncut.is_none() {
                rings = plain;
                links = plain_links;
                uncut = plain_uncut;
            }
        }
        let helix = match helix {
            Some(radius) => {
                let wall_cw = cut_clockwise(Side::Inside, direction, spindle);
                let fit = HelixFit {
                    region: &region,
                    walls: &walls,
                    offset,
                    tool_radius,
                    clockwise: wall_cw,
                };
                fit_helix(&fit, radius, &rings, &ids, &mut notes)?
            }
            None => None,
        };
        cancel::check(cancel)?;
        let next_start = helix
            .as_ref()
            .map(|h| h.start)
            .or_else(|| rings.first().and_then(|r| r.points.first().copied()));
        let back_link = uncut.is_none()
            && match (rings.last().and_then(PocketRing::end), next_start) {
                (Some(from), Some(to)) => back_link_is_safe(&region, &walls, from, to, offset),
                _ => false,
            };
        notes.extend(uncut);
        pieces.push(PocketPiece {
            rings,
            feed_links: links,
            contour_ids: ids,
            helix,
            back_link,
        });
    }
    Ok(PocketPlan {
        pieces,
        notes,
        arcs,
    })
}
