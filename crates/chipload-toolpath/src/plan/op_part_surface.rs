// SPDX-License-Identifier: GPL-3.0-or-later
//! A part surface (docs/specs/surface-levels.md §3): the selected closed shapes grown by the
//! margin, overlapping ones joined, shapes inside another faced over, then cleared as a
//! pocket's rings with its helix entry, so no new clearing code reaches the machine.

use cavalier_contours::polyline::PlineSource;
use chipload_core::Point;
use chipload_core::chain::{Contour, OrientedSegment, Segment};
use chipload_core::primitives::Primitive;
use chipload_core::tolerances::{ARC_APPROX_TOL_MM, OFFSET_CHECK_SLACK_MM, OFFSET_FAR_SLACK_MM};
use chipload_i18n::Message;
use chipload_job::{Direction, Entry, PocketOp, SurfaceOp};

use super::op_pocket::plan_pocket_op;
use super::{Context, Planned};
use crate::ToolpathError;
use crate::cavalier::{Area, ccw, encloses, first_point};
use crate::merge::merge_overlaps;
use crate::offset::{contour_pline, pline_segments, point_distance};
use crate::select::Selection;

/// Plan `op` on its selected `selection`: each outer shape grown by the margin, verified,
/// joined where they overlap, and cleared as a pocket with the surface's tool, depths,
/// step down, stepover and feeds.
pub(super) fn plan_part_surface_op(
    op: &SurfaceOp,
    ctx: &Context,
    selection: &Selection,
    start: Point,
) -> Result<Planned, ToolpathError> {
    let areas = faced_areas(&selection.contours, op.margin)?;
    let selected = Selection {
        contours: areas.iter().collect(),
        notes: selection.notes.clone(),
    };
    let pocket = PocketOp {
        name: op.name.clone(),
        enabled: op.enabled,
        part: op.part,
        material: op.material.clone(),
        select: op.select.clone(),
        tool: op.tool,
        start_depth: op.start_depth,
        end_depth: op.end_depth,
        through: None,
        clearance: 0.0,
        step_down: op.step_down,
        stepover: op.stepover,
        slot_feed: None,
        slot_step_down: None,
        direction: Direction::Climb,
        feed: op.feed,
        plunge_feed: op.plunge_feed,
        entry: Entry::default_helix(ctx.tool.diameter),
        starts: Vec::new(),
        lead: None,
        corners: None,
        corner_choices: Vec::new(),
        rest: None,
        rest_passes: Vec::new(),
        finish: None,
    };
    plan_pocket_op(&pocket, ctx, &selected, start, &[])
}

/// The areas a part surface faces: the selected loops that no other selected loop encloses,
/// each grown by `margin` (verified), joined where they overlap; as closed contours.
fn faced_areas(contours: &[&Contour], margin: f64) -> Result<Vec<Contour>, ToolpathError> {
    let plines: Vec<_> = contours.iter().map(|c| ccw(contour_pline(c))).collect();
    let outer: Vec<usize> = (0..plines.len())
        .filter(|&i| {
            let p = first_point(&plines[i]);
            !(0..plines.len()).any(|j| {
                j != i && encloses(&plines[j], p) && plines[j].area().abs() > plines[i].area().abs()
            })
        })
        .collect();
    let mut loops: Vec<Vec<Primitive>> = Vec::new();
    let mut from: Vec<usize> = Vec::new();
    for &i in &outer {
        for grown in grown_loops(&plines[i], margin)? {
            loops.push(grown);
            from.push(i);
        }
    }
    let merged = merge_overlaps(&loops, &from)?;
    Ok(merged
        .into_iter()
        .map(|(prims, i)| as_contour(contours[i], prims))
        .collect())
}

/// `pline` (counter-clockwise) grown by `margin`, each loop's boundary checked to lie
/// `margin` from the original and outside it. Voids a bay closes up are faced over.
fn grown_loops(
    pline: &cavalier_contours::polyline::Polyline<f64>,
    margin: f64,
) -> Result<Vec<Vec<Primitive>>, ToolpathError> {
    if margin <= 0.0 {
        return Ok(vec![pline_segments(pline)]);
    }
    let original = pline_segments(pline);
    let lowest = margin - ARC_APPROX_TOL_MM - OFFSET_CHECK_SLACK_MM;
    let highest = margin + OFFSET_FAR_SLACK_MM;
    let mut out = Vec::new();
    for area in Area::of(pline.clone()).grown(margin)? {
        let grown = pline_segments(&area.outer);
        let checked = grown.iter().all(|prim| {
            sample_points(prim).into_iter().all(|p| {
                let d = original
                    .iter()
                    .map(|s| point_distance(s, p))
                    .fold(f64::INFINITY, f64::min);
                d >= lowest && d <= highest && !encloses(pline, p)
            })
        });
        if !checked {
            return Err(ToolpathError::Geometry(Message::new(
                "toolpath-error-offset-verification",
            )));
        }
        out.push(grown);
    }
    Ok(out)
}

/// The ends and middle of a line or arc.
fn sample_points(prim: &Primitive) -> [Point; 3] {
    match prim {
        Primitive::Line(l) => [
            l.start,
            ((l.start.0 + l.end.0) / 2.0, (l.start.1 + l.end.1) / 2.0),
            l.end,
        ],
        Primitive::Arc(a) => [
            a.point_at(a.start_angle),
            a.point_at(a.start_angle + a.sweep / 2.0),
            a.point_at(a.start_angle + a.sweep),
        ],
    }
}

/// A closed contour of `prims`, numbered and attributed as `like` (the selected shape it
/// was grown from), so the pocket's pieces and notes name it.
fn as_contour(like: &Contour, prims: Vec<Primitive>) -> Contour {
    let template = like.parts.first().map(|p| p.segment.clone());
    let parts = prims
        .into_iter()
        .map(|prim| OrientedSegment {
            segment: Segment {
                prim,
                ..template.clone().unwrap_or_else(|| Segment {
                    prim,
                    layer: String::new(),
                    handle: String::new(),
                    source_type: String::new(),
                    shape: None,
                    from: None,
                    construction: false,
                })
            },
            reversed: false,
        })
        .collect();
    Contour {
        id: like.id,
        parts,
        closed: true,
        invalid_reason: None,
        nesting_depth: Some(0),
    }
}
