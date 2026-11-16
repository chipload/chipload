// SPDX-License-Identifier: GPL-3.0-or-later
//! Planning profiles and their tabs.

use chipload_core::Point;
use chipload_core::math;
use chipload_core::tolerances::CHAIN_TOL_MM;
use chipload_i18n::{Digits, Message, Value};
use chipload_job::{ProfileOp, Side, Tabs};

use super::check::comma_list;
use super::check_tools::short_ramp_refusal;
use super::op_pocket::profile_legs;
use super::{Context, Planned, job_error};
use crate::ToolpathError;
use crate::corners::{CornerCut, unreached_note};
use crate::lap::{RingCut, TabSurface, WallLap, carry_tabs, farthest_from, ring_moves, wall_lap};
use crate::profile::{
    PathRing, contour_context, contour_note, labelled_note, pass_depths, plan_profile,
};
use crate::select::Selection;
use crate::tabs::{
    TabLayout, along_tabs, arc_position, place_tabs, point_at_arc, tab_polylines, tabs_at,
};

pub(super) fn plan_profile_op(
    op: &ProfileOp,
    ctx: &Context,
    selection: &Selection,
    start: Point,
    starts: &[Point],
) -> Result<Planned, ToolpathError> {
    let machine = &ctx.job.machine;
    let radius = ctx.tool.diameter / 2.0;
    // The rings at the wall; with a finishing pass the roughing rings run `stock` farther
    // out (docs/specs/finish-and-skin.md §2).
    let mut plan = plan_profile(
        op,
        ctx.label_message,
        &selection.contours,
        radius,
        ctx.sense,
        start,
        starts,
    )?;
    let rough_op = op.finish.map(|finish| ProfileOp {
        clearance: op.clearance - finish.stock,
        ..op.clone()
    });
    let rough = match &rough_op {
        Some(rough_op) => Some(plan_profile(
            rough_op,
            ctx.label_message,
            &selection.contours,
            radius,
            ctx.sense,
            start,
            starts,
        )?),
        None => None,
    };
    let mut rough = rough;
    for ring in plan.rings.iter().chain(rough.iter().flat_map(|r| &r.rings)) {
        let length = crate::ramp::polyline_length(&crate::profile::closed_path(&ring.points));
        short_ramp_refusal(ctx.tool, length).map_err(|why| {
            job_error(labelled_note(
                &contour_context(ctx.label_message, ring.contour_id),
                why,
            ))
        })?;
    }
    let mut unreached_at = Vec::new();
    // Dogbones go into every ring before the tabs are placed (docs/specs/dogbones.md §4).
    if op.corners.is_some() || !ctx.choices.is_empty() {
        let cut = CornerCut {
            corners: op.corners,
            choices: ctx.choices,
            tool_radius: radius,
            clearance: op.clearance,
            everything: ctx.cut_contours,
            exempt: &[],
            walls: &selection.contours,
            clears_inside: false,
            label: ctx.label_message,
        };
        let (unreached, warnings) =
            profile_legs(&mut plan.rings, &selection.contours, op.side, &cut)?;
        plan.notes.extend(warnings);
        unreached_at.extend(unreached.iter().map(|(c, _)| c.at));
        plan.notes.extend(
            unreached
                .iter()
                .map(|(c, bone)| unreached_note(ctx.label_message, c, *bone)),
        );
        if let (Some(rough), Some(rough_op)) = (&mut rough, &rough_op) {
            let rough_cut = CornerCut {
                clearance: rough_op.clearance,
                ..cut
            };
            // The roughing's legs are the wall's, a little short: the same warnings.
            profile_legs(&mut rough.rings, &selection.contours, op.side, &rough_cut)?;
        }
    }
    let mut out = Planned {
        moves: Vec::new(),
        notes: plan.notes,
        tabs: Vec::new(),
        pockets: Vec::new(),
        arcs: plan.arcs,
        end: start,
        starts: plan
            .rings
            .iter()
            .zip(&plan.chosen)
            .map(|(ring, &stored)| (ring.points.clone(), stored))
            .collect(),
        unreached: unreached_at,
        tab_centres: Vec::new(),
        floor_hint: None,
        facts: Vec::new(),
    };
    if let Some(rough) = &rough {
        out.arcs.extend(rough.arcs.clone());
        // The roughing may add a narrow part the wall doesn't have; its notes on start
        // points say the same as the wall's.
        let mut words: Vec<String> = out.notes.iter().map(Message::english).collect();
        for note in &rough.notes {
            let english = note.english();
            if !words.contains(&english) {
                words.push(english);
                out.notes.push(note.clone());
            }
        }
    }
    // The notes on start points come after the geometry's, as before.
    out.notes.extend(plan.start_notes);
    let end_z = ctx.pass_z.last().copied().unwrap_or(ctx.top_z);
    // With a skin the roughing stops `thickness` above the end depth.
    let rough_z: Vec<f64> = match op.skin {
        Some(skin) => pass_depths(op.start_depth, op.end_depth - skin.thickness, op.step_down)?
            .into_iter()
            .map(|d| ctx.top_z - (d - op.start_depth))
            .collect(),
        None => ctx.pass_z.to_vec(),
    };
    let rough_bottom = rough_z.last().copied().unwrap_or(end_z);
    // The tabs are placed on the wall rings; the roughing rings carry them over.
    let (layouts, curved) = profile_tabs(op, ctx, &plan.rings, &mut out)?;
    let surface = |layout: &Option<TabLayout>| -> Option<(TabLayout, f64, f64)> {
        let tabs = op.tabs.as_ref()?;
        Some((layout.clone()?, end_z + tabs.height, tabs.flank_angle))
    };
    let rough_rings = rough.as_ref().map_or(&plan.rings, |r| &r.rings);
    // Roughing: every ring as a spiral, down to the skin.
    for (k, ring) in rough_rings.iter().enumerate() {
        let context = |err: ToolpathError| {
            err.with_context(contour_context(ctx.label_message, ring.contour_id))
        };
        let layout = if rough.is_some() {
            carried_tabs(&plan.rings, &layouts, k, rough_rings).map_err(context)?
        } else {
            // The roughing rings are the wall rings.
            layouts.get(k).cloned().flatten()
        };
        let tabs = surface(&layout);
        let cut = RingCut {
            ring: &ring.points,
            tabs: as_surface(&tabs),
            pass_z: &rough_z,
            top_z: ctx.top_z,
            safe_z: machine.safe_z,
            feed: op.feed,
            plunge_feed: op.plunge_feed,
            ramp_angle: Some(op.ramp_angle),
            tool_diameter: ctx.tool.diameter,
        };
        let cut = ring_moves(&cut, &out.arcs).map_err(context)?;
        out.moves.extend(cut.moves);
        out.end = cut.end;
    }
    // Then a finishing lap on every wall ring, then a skin lap (§4).
    let stages = [op.finish.map(|_| rough_bottom), op.skin.map(|_| end_z)];
    for z in stages.into_iter().flatten() {
        for (ring, layout) in plan.rings.iter().zip(&layouts) {
            let context = |err: ToolpathError| {
                err.with_context(contour_context(ctx.label_message, ring.contour_id))
            };
            let slot: Vec<&[Point]> = rough_rings
                .iter()
                .filter(|r| r.contour_id == ring.contour_id)
                .map(|r| r.points.as_slice())
                .collect();
            if let Some(finish) = op.finish
                && z == rough_bottom
            {
                slot_note(
                    ring,
                    &slot,
                    radius,
                    finish.stock,
                    ctx.label_message,
                    &mut out.notes,
                )?;
            }
            let tabs = surface(layout);
            let lap = WallLap {
                ring: &ring.points,
                tabs: as_surface(&tabs),
                slot: &slot,
                slot_z: rough_bottom,
                z,
                safe_z: machine.safe_z,
                feed: op.feed,
                plunge_feed: op.plunge_feed,
                ramp_angle: op.ramp_angle,
                tool_diameter: ctx.tool.diameter,
            };
            let cut = wall_lap(&lap, &out.arcs).map_err(context)?;
            out.moves.extend(cut.moves);
            out.end = cut.end;
        }
    }
    if !curved.is_empty() {
        let ids: Vec<String> = curved.iter().map(|c| format!("#{c}")).collect();
        out.notes.push(labelled_note(
            ctx.label_message,
            Message::new("toolpath-tabs-on-curves").arg("contours", ids.join(", ")),
        ));
    }
    Ok(out)
}

/// The tabs on every wall ring (docs/specs/profile-spiral.md §4): each ring's layout (None
/// without tabs), with the tabs' polylines and notes added to `out`, and the contours
/// whose tabs sit on a curve.
fn profile_tabs(
    op: &ProfileOp,
    ctx: &Context,
    rings: &[PathRing],
    out: &mut Planned,
) -> Result<(Vec<Option<TabLayout>>, Vec<usize>), ToolpathError> {
    let Some(tabs) = &op.tabs else {
        return Ok((vec![None; rings.len()], Vec::new()));
    };
    if op.side == Side::Inside {
        out.notes.push(labelled_note(
            ctx.label_message,
            Message::new("toolpath-tabs-inside"),
        ));
    }
    // Tabs placed by hand go to their nearest ring (docs/specs/tabs-by-hand.md §2).
    let mut by_ring: Vec<Vec<Point>> = vec![Vec::new(); rings.len()];
    for &p in ctx.tab_points {
        let nearest = rings
            .iter()
            .enumerate()
            .filter_map(|(k, r)| arc_position(&r.points, p).map(|(_, d)| (k, d)))
            .min_by(|a, b| a.1.total_cmp(&b.1));
        match nearest {
            Some((k, d)) if d <= ctx.tool.diameter => by_ring[k].push(p),
            _ => {
                return Err(ToolpathError::Geometry(
                    Message::new("toolpath-tab-no-ring")
                        .arg("label", ctx.label_message.clone())
                        .arg("at", Value::Point(p.0, p.1, Digits::Fixed(3))),
                ));
            }
        }
    }
    let mut layouts: Vec<Option<TabLayout>> = Vec::with_capacity(rings.len());
    let mut curved: Vec<usize> = Vec::new();
    for (k, (ring, points)) in rings.iter().zip(&by_ring).enumerate() {
        // Validated: count >= 1.
        let count = usize::try_from(tabs.count).unwrap_or(0);
        let placed = if let (true, Some(axis)) = (points.is_empty(), tabs.along) {
            along_tabs(&ring.points, count, axis, tabs.width, ctx.tool.diameter)
        } else if points.is_empty() {
            place_tabs(&ring.points, count, tabs.width, ctx.tool.diameter)
        } else {
            tabs_at(&ring.points, points, tabs.width, ctx.tool.diameter)
        }
        .map_err(|err| err.with_context(contour_context(ctx.label_message, ring.contour_id)))?;
        out.tab_centres.extend(placed.spans.iter().map(|&(s0, s1)| {
            (
                k,
                point_at_arc(&ring.points, (s0 + s1) / 2.0),
                !points.is_empty(),
            )
        }));
        if placed.on_curves > 0 {
            curved.push(ring.contour_id);
        }
        out.tabs.extend(tab_polylines(&ring.points, &placed));
        if let Some(note) = joined_feet(&placed, tabs) {
            out.notes
                .push(contour_note(ctx.label_message, ring.contour_id, note));
        }
        layouts.push(Some(placed));
    }
    Ok((layouts, curved))
}

/// A tab layout with its top and flank angle, as the laps follow it.
fn as_surface(tabs: &Option<(TabLayout, f64, f64)>) -> Option<TabSurface<'_>> {
    tabs.as_ref()
        .map(|(layout, z_tab, flank_angle)| TabSurface {
            layout,
            z_tab: *z_tab,
            flank_angle: *flank_angle,
        })
}

/// The tabs of the wall rings of a contour, carried over to roughing ring `k` of `rough`
/// (docs/specs/finish-and-skin.md §2): every lap leaves the same tabs.
fn carried_tabs(
    walls: &[PathRing],
    layouts: &[Option<TabLayout>],
    k: usize,
    rough: &[PathRing],
) -> Result<Option<TabLayout>, ToolpathError> {
    let ring = &rough[k];
    let others: Vec<&[Point]> = rough
        .iter()
        .enumerate()
        .filter(|&(j, r)| r.contour_id == ring.contour_id && j != k)
        .map(|(_, r)| r.points.as_slice())
        .collect();
    let mut carried: Option<TabLayout> = None;
    for (wall, layout) in walls.iter().zip(layouts) {
        let Some(layout) = layout else { continue };
        if wall.contour_id != ring.contour_id {
            continue;
        }
        let more = carry_tabs(&wall.points, layout, &ring.points, &others)?;
        carried = Some(match carried {
            None => more,
            Some(mut so_far) => {
                so_far.spans.extend(more.spans);
                so_far.spans.sort_by(|a, b| a.0.total_cmp(&b.0));
                let mut joined: Vec<(f64, f64)> = Vec::new();
                for span in so_far.spans {
                    match joined.last_mut() {
                        Some(last) if span.0 <= last.1 => last.1 = last.1.max(span.1),
                        _ => joined.push(span),
                    }
                }
                so_far.spans = joined;
                so_far
            }
        });
    }
    Ok(carried)
}

/// The finishing lap must run in the roughing's slot: refused where it is a tool radius or
/// more from the roughing path (the roughing couldn't get there), a note where it cuts
/// more than twice the stock (docs/specs/finish-and-skin.md §6).
fn slot_note(
    ring: &PathRing,
    slot: &[&[Point]],
    radius: f64,
    stock: f64,
    label_message: &Message,
    notes: &mut Vec<Message>,
) -> Result<(), ToolpathError> {
    let farthest = farthest_from(&ring.points, slot, radius / 2.0);
    let id = ring.contour_id;
    if farthest >= radius {
        return Err(ToolpathError::Geometry(contour_note(
            label_message,
            id,
            Message::new("toolpath-error-roughing-reach").arg("farthest", Value::mm(farthest)),
        )));
    }
    if farthest > 2.0 * stock + CHAIN_TOL_MM {
        notes.push(contour_note(
            label_message,
            id,
            Message::new("toolpath-finish-lap-cuts")
                .arg("most", Value::mm(farthest))
                .arg("stock", Value::mm(stock)),
        ));
    }
    Ok(())
}

/// A note when neighbouring sloped tabs meet below their top: their feet, each
/// `height / tan(flank)` long, together are longer than the gap between them
/// (docs/specs/profile-spiral.md §4).
fn joined_feet(layout: &TabLayout, tabs: &Tabs) -> Option<Message> {
    if tabs.is_straight() || layout.spans.len() < 2 {
        return None;
    }
    let feet = 2.0 * tabs.height / math::tan(tabs.flank_angle);
    let count = layout.spans.len();
    let mut pairs = Vec::new();
    for k in 0..count {
        let (_, end) = layout.spans[k];
        let next = (k + 1) % count;
        let mut gap = layout.spans[next].0 - end;
        if next == 0 {
            gap += layout.length;
        }
        if gap < feet {
            pairs.push(
                Message::new("toolpath-and")
                    .arg("list", k + 1)
                    .arg("last", next + 1),
            );
        }
    }
    let pairs = comma_list(pairs)?;
    Some(
        Message::new("toolpath-tabs-joined")
            .arg("pairs", pairs)
            .arg(
                "flank",
                Value::Number(tabs.flank_angle.to_degrees(), Digits::Fixed(1)),
            ),
    )
}
