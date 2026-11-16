// SPDX-License-Identifier: GPL-3.0-or-later
//! The pocket's region (outline less islands) and the tree of rings offset inside it.

use cavalier_contours::polyline::{
    BooleanOp, PlineContainsResult, PlineSource, PlineSourceMut, Polyline,
};
use chipload_core::chain::Contour;
use chipload_core::nesting::{Ring, contains};
use chipload_core::tolerances::CHAIN_TOL_MM;
use chipload_i18n::{Digits, Message, Value};

use super::{MAX_RING_LEVELS, PocketArea, ids_text};
use crate::ToolpathError;
use crate::cancel::{self, Cancel};
use crate::cavalier::{Area, boolean, ccw, cw, encloses, first_point, geometry, step, too_small};
use crate::offset::{contour_pline, dedupe_ring};
use crate::retry::{Failure, Nudge};

/// Areas to clear from selected closed contours, sorted by (y, x).
///
/// Every selected contour at even nesting depth (among the selection) is its own pocket, even
/// if it crosses another one: crossing shapes are separate geometries and are never merged
/// (user decision 2026-10-08). Selected contours lying completely inside it one level deeper
/// are its islands.
pub fn build_region(contours: &[&Contour]) -> Result<Vec<PocketArea>, ToolpathError> {
    let shapes: Vec<(usize, Polyline<f64>, Ring)> = contours
        .iter()
        .map(|c| {
            let points = dedupe_ring(&c.points());
            let mut pline = Polyline::new_closed();
            for &(x, y) in &points {
                pline.add(x, y, 0.0);
            }
            (c.id, ccw(contour_pline(c)), Ring { points, pline })
        })
        .collect();
    refuse_duplicates(&shapes)?;
    let depth: Vec<usize> = shapes
        .iter()
        .map(|(id, _, ring)| {
            shapes
                .iter()
                .filter(|(other, _, outer)| other != id && contains(outer, ring))
                .count()
        })
        .collect();
    let mut result = Vec::new();
    for (i, (id, boundary, ring)) in shapes.iter().enumerate() {
        if depth[i] % 2 == 1 {
            continue;
        }
        let islands: Vec<(usize, Polyline<f64>)> = shapes
            .iter()
            .enumerate()
            .filter(|(j, (_, _, island))| depth[*j] == depth[i] + 1 && contains(ring, island))
            .map(|(_, (iid, pline, _))| (*iid, pline.clone()))
            .collect();
        let mut ids = vec![*id];
        ids.extend(islands.iter().map(|(iid, _)| *iid));
        let islands = merge_islands(islands.into_iter().map(|(_, p)| p).collect(), &ids)?;
        for area in subtract_islands(boundary.clone(), islands)? {
            result.push(PocketArea {
                area,
                contour_ids: ids.clone(),
            });
        }
    }
    result.sort_by(|a, b| {
        (a.area.sort_key(), &a.contour_ids).cmp(&(b.area.sort_key(), &b.contour_ids))
    });
    Ok(result)
}

fn boolean_area(result: &cavalier_contours::polyline::BooleanResult<Polyline<f64>>) -> f64 {
    let pos: f64 = result.pos_plines.iter().map(|p| p.pline.area().abs()).sum();
    let neg: f64 = result.neg_plines.iter().map(|p| p.pline.area().abs()).sum();
    pos - neg
}

/// The same shape selected twice can't be both boundary and island: refuse.
fn refuse_duplicates(shapes: &[(usize, Polyline<f64>, Ring)]) -> Result<(), ToolpathError> {
    for (i, (id_a, a, _)) in shapes.iter().enumerate() {
        for (id_b, b, _) in &shapes[i + 1..] {
            let (Some(ea), Some(eb)) = (a.extents(), b.extents()) else {
                continue;
            };
            if ea.max_x < eb.min_x
                || eb.max_x < ea.min_x
                || ea.max_y < eb.min_y
                || eb.max_y < ea.min_y
            {
                continue;
            }
            let common = boolean(step("comparing-shapes"), a, b, BooleanOp::And)?;
            let symmetric = a.area().abs() + b.area().abs() - 2.0 * boolean_area(&common);
            let slack = CHAIN_TOL_MM * a.path_length().max(b.path_length());
            if symmetric <= slack {
                return Err(ToolpathError::Selector(
                    Message::new("toolpath-error-same-shape")
                        .arg("a", *id_a)
                        .arg("b", *id_b),
                ));
            }
        }
    }
    Ok(())
}

/// Islands that cross or touch each other become one island.
fn merge_islands(
    islands: Vec<Polyline<f64>>,
    ids: &[usize],
) -> Result<Vec<Polyline<f64>>, ToolpathError> {
    let mut merged: Vec<Polyline<f64>> = Vec::new();
    for island in islands {
        let mut current = island;
        while let Some(j) = merged
            .iter()
            .position(|other| !matches!(current.contains(other), PlineContainsResult::Disjoint))
        {
            let other = merged.remove(j);
            let mut union = boolean(step("merging-islands"), &current, &other, BooleanOp::Or)?;
            if union.pos_plines.len() != 1 || !union.neg_plines.is_empty() {
                return Err(geometry(
                    Message::new("toolpath-error-islands-enclose").arg("contours", ids_text(ids)),
                ));
            }
            current = ccw(union.pos_plines.swap_remove(0).pline);
        }
        merged.push(current);
    }
    Ok(merged)
}

/// The boundary minus islands. An island strictly inside becomes a hole; one touching or
/// crossing the boundary is cut away, which can split the area.
fn subtract_islands(
    boundary: Polyline<f64>,
    islands: Vec<Polyline<f64>>,
) -> Result<Vec<Area>, ToolpathError> {
    let mut areas = vec![Area {
        outer: boundary,
        holes: Vec::new(),
    }];
    for island in islands {
        let mut next = Vec::new();
        for mut area in areas {
            match area.outer.contains(&island) {
                PlineContainsResult::Pline2InsidePline1 => {
                    area.holes.push(cw(island.clone()));
                    next.push(area);
                }
                PlineContainsResult::Intersected | PlineContainsResult::Pline1InsidePline2 => {
                    let cut = boolean(
                        step("subtracting-an-island"),
                        &area.outer,
                        &island,
                        BooleanOp::Not,
                    )?;
                    let mut pieces: Vec<Area> = cut
                        .pos_plines
                        .into_iter()
                        .map(|p| Area {
                            outer: ccw(p.pline),
                            holes: Vec::new(),
                        })
                        .collect();
                    let holes = area
                        .holes
                        .into_iter()
                        .chain(cut.neg_plines.into_iter().map(|p| cw(p.pline)));
                    for hole in holes {
                        let inside = first_point(&hole);
                        if let Some(piece) = pieces.iter_mut().find(|p| encloses(&p.outer, inside))
                        {
                            piece.holes.push(hole);
                        }
                    }
                    next.extend(pieces);
                }
                PlineContainsResult::Disjoint | PlineContainsResult::InvalidInput => {
                    next.push(area);
                }
            }
        }
        areas = next;
    }
    Ok(areas.into_iter().filter(|a| !too_small(&a.outer)).collect())
}

/// Offset levels as a tree: every level k+1 area lies inside one level k area.
pub(super) struct Node {
    pub(super) area: Area,
    pub(super) children: Vec<usize>,
}

/// `radius` is the tool-centre offset from the walls (tool radius minus clearance);
/// `tool_radius` is only for messages. `nudge` moves the offsets for a retry
/// (docs/specs/offset-retry.md).
pub(super) fn ring_tree(
    region: &Area,
    radius: f64,
    tool_radius: f64,
    stepover: f64,
    ids: &[usize],
    nudge: Nudge,
    cancel: Option<&Cancel>,
) -> Result<(Vec<Node>, Vec<usize>), Failure> {
    let numerical = |wall: bool| move |error| Failure::Numerical { error, wall };
    let wall_offset = radius + nudge.wall;
    let stepover = stepover - nudge.step;
    let mut nodes: Vec<Node> = Vec::new();
    let mut roots = Vec::new();
    let flat;
    let region = if nudge.flat {
        flat = region.flattened();
        &flat
    } else {
        region
    };
    for area in region
        .shrunk_with(wall_offset, nudge.tight)
        .map_err(numerical(true))?
    {
        roots.push(nodes.len());
        nodes.push(Node {
            area,
            children: Vec::new(),
        });
    }
    if roots.is_empty() {
        let message = if radius == tool_radius {
            Message::new("toolpath-error-pocket-no-fit")
        } else {
            Message::new("toolpath-error-pocket-no-fit-clearance").arg(
                "clearance",
                Value::Length(tool_radius - radius, Digits::Fixed(3)),
            )
        };
        return Err(Failure::Final(geometry(
            message.arg("contours", ids_text(ids)).arg(
                "diameter",
                Value::Length(2.0 * tool_radius, Digits::Fixed(3)),
            ),
        )));
    }
    let mut level = roots.clone();
    for k in 1..=MAX_RING_LEVELS {
        // Each ring is an offset of the whole region: a big pocket's rings are most of a
        // run, so a cancel is seen ring by ring (docs/specs/background-generation.md).
        cancel::check(cancel).map_err(Failure::Final)?;
        let areas = region
            .shrunk_with(wall_offset + k as f64 * stepover, nudge.tight)
            .map_err(numerical(false))?;
        if areas.is_empty() {
            return Ok((nodes, roots));
        }
        let mut next_level = Vec::new();
        for area in areas {
            let inside = first_point(&area.outer);
            let Some(&parent) = level.iter().find(|&&n| nodes[n].area.contains(inside)) else {
                return Err(Failure::Numerical {
                    error: geometry(
                        Message::new("toolpath-error-not-nested").arg("contours", ids_text(ids)),
                    ),
                    wall: false,
                });
            };
            let id = nodes.len();
            nodes.push(Node {
                area,
                children: Vec::new(),
            });
            nodes[parent].children.push(id);
            next_level.push(id);
        }
        level = next_level;
    }
    Err(Failure::Final(geometry(
        Message::new("toolpath-error-too-many-rings")
            .arg("contours", ids_text(ids))
            .arg("most", MAX_RING_LEVELS),
    )))
}
