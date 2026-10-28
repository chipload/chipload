// SPDX-License-Identifier: GPL-3.0-or-later
//! Editing operations on drawn shapes (docs/specs/drawing-editing.md): the geometry is
//! `chipload-draw`'s; here shapes become its chains and its results become shapes, keeping
//! a shape's kind where the result is exactly that kind, with ids as the spec says.

use std::f64::consts::PI;

use chipload_core::tolerances::CHAIN_TOL_MM;
pub use chipload_draw::offset::Corners;
use chipload_draw::offset::{self, Chain, OffsetError, Side};
use chipload_i18n::{Digits, Message, Value};
use chipload_job::{Geometry, Job, Shape};

use crate::PipelineError;
use crate::shapes::{add_shape, delete_shape, no_shape, reshape};
pub use chipload_draw::boolean::Combine;
use chipload_draw::boolean::{self, BooleanError};
pub use chipload_draw::corner::Cut;
use chipload_draw::corner::{self, CornerError, Poly};
pub use chipload_draw::trim::End;
use chipload_draw::trim::{self, TrimError};

fn refused(message: Message) -> PipelineError {
    PipelineError::Job(message)
}

/// What an editing operation did: the project, the new shapes' ids, and notes for the user.
#[derive(Debug, Clone)]
pub struct Edited {
    pub job: Job,
    pub new_ids: Vec<u64>,
    pub notes: Vec<Message>,
}

/// The drawn shape `id`, refused when it is imported.
fn drawn(job: &Job, id: u64) -> Result<&Shape, PipelineError> {
    let shape = job
        .drawing
        .shapes
        .iter()
        .find(|s| s.id == id)
        .ok_or_else(|| no_shape(id))?;
    if !shape.is_drawn() {
        return Err(refused(
            Message::new("pipeline-edit-imported").arg("id", id),
        ));
    }
    Ok(shape)
}

/// A shape's outline as a chain of lines and arcs.
pub fn chain_of(shape: &Shape) -> Chain {
    Chain {
        prims: shape.segments().iter().map(|s| s.to_primitive()).collect(),
        closed: shape.closed(),
    }
}

/// A chain as a drawn polyline.
pub fn polyline_of(chain: &Chain) -> Geometry {
    let (points, bulges) = chain.as_polyline();
    Geometry::Polyline {
        closed: chain.closed,
        points: points.iter().map(|&(x, y)| [x, y]).collect(),
        bulges,
    }
}

/// Notes naming the operations that select `layer` by its name, which now cut the new
/// shapes there too (decision 1).
fn layer_cutters(job: &Job, layer: &str, count: usize) -> Option<Message> {
    let ops: Vec<String> = job
        .operations
        .iter()
        .enumerate()
        .filter(|(_, op)| {
            let select = op.select();
            select
                .layer
                .as_deref()
                .is_some_and(|l| l.to_lowercase() == layer.to_lowercase())
                && select.dxf.is_none()
                && select.image.is_none()
                && select.text.is_none()
        })
        .map(|(i, _)| (i + 1).to_string())
        .collect();
    (!ops.is_empty()).then(|| {
        Message::new("pipeline-new-shapes-cut")
            .arg("count", count)
            .arg("layer", layer.to_string())
            .arg("operations", ops.join(", "))
            .arg("ops", ops.len())
    })
}

/// Add `geometries` on `source`'s layer, construction geometry when it is, returning their
/// ids, with the note on the operations that cut that layer (none for construction).
fn add_all(
    mut job: Job,
    source: &Shape,
    geometries: Vec<Geometry>,
    new_ids: &mut Vec<u64>,
    notes: &mut Vec<Message>,
) -> Result<Job, PipelineError> {
    let layer = source.layer.as_str();
    let count = geometries.len();
    for geometry in geometries {
        let (mut next, id) = add_shape(&job, layer, geometry)?;
        if let Some(added) = next.drawing.shapes.iter_mut().find(|s| s.id == id) {
            added.construction = source.construction;
        }
        job = next;
        new_ids.push(id);
    }
    if !source.construction
        && let Some(note) = layer_cutters(&job, layer, count)
    {
        notes.push(note);
    }
    Ok(job)
}

// -- offset (§2)

/// Which side an offset goes: for a closed shape outside or inside, for an open one left
/// or right. None: outside, or left for an open shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OffsetSide {
    Outside,
    Inside,
    Left,
    Right,
}

/// Offset drawn shapes `ids` by `distance` (mm), `count` times (each `distance` further),
/// adding the results on each shape's layer; the shapes stay. Kinds are kept where the
/// offset is exactly that kind: a circle, a rectangle (or slot), a polygon inward or with
/// sharp corners.
pub fn offset_shapes(
    job: &Job,
    ids: &[u64],
    distance: f64,
    side: Option<OffsetSide>,
    corners: Corners,
    count: u32,
) -> Result<Edited, PipelineError> {
    if !(distance.is_finite() && distance > 0.0) {
        return Err(refused(
            Message::new("pipeline-offset-distance")
                .arg("got", Value::Length(distance, Digits::Shortest)),
        ));
    }
    let mut out = job.clone();
    let mut new_ids = Vec::new();
    let mut notes = Vec::new();
    for &id in ids {
        let shape = drawn(job, id)?.clone();
        let side = match (side, shape.closed()) {
            (Some(side), _) => side,
            (None, true) => OffsetSide::Outside,
            (None, false) => OffsetSide::Left,
        };
        let mut geometries = Vec::new();
        for k in 1..=count.max(1) {
            let d = distance * f64::from(k);
            geometries.extend(offset_geometry(&shape, d, side, corners)?);
        }
        out = add_all(out, &shape, geometries, &mut new_ids, &mut notes)?;
    }
    notes.dedup();
    Ok(Edited {
        job: out,
        new_ids,
        notes,
    })
}

/// The offset the cursor at `at` asks for, from the nearest of drawn shapes `ids`: its
/// distance to it and the side it is on. None when none of them is drawn.
pub fn offset_at(job: &Job, ids: &[u64], at: (f64, f64)) -> Option<(f64, OffsetSide)> {
    let nearest = ids
        .iter()
        .filter_map(|&id| drawn(job, id).ok())
        .map(|shape| {
            let chain = chain_of(shape);
            (offset::nearest_distance(&chain, at), chain)
        })
        .min_by(|a, b| a.0.total_cmp(&b.0))?;
    let side = match offset::side_at(&nearest.1, at) {
        Side::Outside => OffsetSide::Outside,
        Side::Inside => OffsetSide::Inside,
        Side::Left => OffsetSide::Left,
        Side::Right => OffsetSide::Right,
    };
    Some((nearest.0, side))
}

/// One shape offset by `d`: its own kind where that is exact, else polylines.
fn offset_geometry(
    shape: &Shape,
    d: f64,
    side: OffsetSide,
    corners: Corners,
) -> Result<Vec<Geometry>, PipelineError> {
    let id = shape.id;
    let vanishes = || {
        refused(
            Message::new("pipeline-offset-vanishes")
                .arg("id", id)
                .arg("distance", Value::Length(d, Digits::Shortest)),
        )
    };
    let outside = side == OffsetSide::Outside;
    let closed_side = matches!(side, OffsetSide::Outside | OffsetSide::Inside);
    match (&shape.geometry, closed_side) {
        (Geometry::Circle { center, radius }, true) => {
            let radius = if outside { radius + d } else { radius - d };
            if radius < CHAIN_TOL_MM {
                return Err(vanishes());
            }
            Ok(vec![Geometry::Circle {
                center: *center,
                radius,
            }])
        }
        (
            Geometry::Rectangle {
                center,
                length,
                width,
                angle,
                corner_radius,
            },
            true,
        ) => {
            let (length, width, corner_radius) = if outside {
                let r = if *corner_radius > 0.0 || corners == Corners::Round {
                    corner_radius + d
                } else {
                    0.0
                };
                (length + 2.0 * d, width + 2.0 * d, r)
            } else {
                let r = corner_radius - d;
                (
                    length - 2.0 * d,
                    width - 2.0 * d,
                    if r < CHAIN_TOL_MM { 0.0 } else { r },
                )
            };
            if length < CHAIN_TOL_MM || width < CHAIN_TOL_MM {
                return Err(vanishes());
            }
            Ok(vec![Geometry::Rectangle {
                center: *center,
                length,
                width,
                angle: *angle,
                corner_radius,
            }])
        }
        // Inward, and outward with sharp corners, a regular polygon's sides move by d: its
        // corners by d / cos(π / n).
        (
            Geometry::Polygon {
                center,
                radius,
                sides,
                angle,
            },
            true,
        ) if !outside || corners == Corners::Sharp => {
            let step = d / (PI / f64::from(*sides)).cos();
            let radius = if outside {
                radius + step
            } else {
                radius - step
            };
            if radius < CHAIN_TOL_MM {
                return Err(vanishes());
            }
            Ok(vec![Geometry::Polygon {
                center: *center,
                radius,
                sides: *sides,
                angle: *angle,
            }])
        }
        _ => {
            let side = match side {
                OffsetSide::Outside => Side::Outside,
                OffsetSide::Inside => Side::Inside,
                OffsetSide::Left => Side::Left,
                OffsetSide::Right => Side::Right,
            };
            let results =
                offset::offset(&chain_of(shape), d, side, corners).map_err(|e| match e {
                    OffsetError::Vanishes => vanishes(),
                    OffsetError::WrongSide => refused(
                        Message::new("pipeline-offset-side")
                            .arg("id", id)
                            .arg("closed", if shape.closed() { "closed" } else { "open" }),
                    ),
                    OffsetError::BadDistance => refused(
                        Message::new("pipeline-offset-distance")
                            .arg("got", Value::Length(d, Digits::Shortest)),
                    ),
                    OffsetError::Library => refused(
                        Message::new("pipeline-offset-failed")
                            .arg("id", id)
                            .arg("why", "the offset library failed"),
                    ),
                    OffsetError::Unverified(why) => refused(
                        Message::new("pipeline-offset-failed")
                            .arg("id", id)
                            .arg("why", why),
                    ),
                })?;
            Ok(results.iter().map(polyline_of).collect())
        }
    }
}

// -- fillet and chamfer (§3)

/// A drawn shape as a polyline of corners for filleting, with the kind it has: None for a
/// circle, or a rectangle whose corners are round already (no corners to cut).
fn corner_poly(geometry: &Geometry) -> Option<Poly> {
    let points = |corners: Vec<[f64; 2]>| corners.iter().map(|p| (p[0], p[1])).collect();
    match geometry {
        Geometry::Rectangle {
            center,
            length,
            width,
            angle,
            corner_radius,
        } if *corner_radius == 0.0 => Some(Poly {
            points: points(
                chipload_job::edit::rectangle_corners(*center, *length, *width, *angle).to_vec(),
            ),
            bulges: vec![0.0; 4],
            closed: true,
        }),
        Geometry::Polygon {
            center,
            radius,
            sides,
            angle,
        } => Some(Poly {
            points: points(chipload_job::polygon_corners(
                *center, *radius, *sides, *angle,
            )),
            bulges: vec![0.0; *sides as usize],
            closed: true,
        }),
        Geometry::Polyline {
            closed,
            points: p,
            bulges,
        } => Some(Poly {
            points: points(p.clone()),
            bulges: bulges.clone(),
            closed: *closed,
        }),
        _ => None,
    }
}

/// The corners of a drawn shape that can be filleted or chamfered: (vertex, where).
pub fn shape_corners(shape: &Shape) -> Vec<(usize, (f64, f64))> {
    corner_poly(&shape.geometry)
        .map(|poly| {
            corner::corners(&poly)
                .into_iter()
                .map(|k| (k, poly.points[k]))
                .collect()
        })
        .unwrap_or_default()
}

/// Fillet or chamfer corner `corner` of drawn shape `id` (None: every corner). The shape
/// keeps its id; a rectangle filleted all round keeps its kind (its corner radius), any
/// other becomes a polyline.
pub fn cut_shape_corners(
    job: &Job,
    id: u64,
    corner: Option<usize>,
    cut: Cut,
) -> Result<Job, PipelineError> {
    let shape = drawn(job, id)?;
    if let (
        Geometry::Rectangle {
            center,
            length,
            width,
            angle,
            ..
        },
        None,
        Cut::Fillet(r),
    ) = (&shape.geometry, corner, cut)
    {
        return reshape(
            job,
            id,
            Geometry::Rectangle {
                center: *center,
                length: *length,
                width: *width,
                angle: *angle,
                corner_radius: r,
            },
        );
    }
    let poly = corner_poly(&shape.geometry)
        .ok_or_else(|| refused(Message::new("pipeline-corner-none").arg("id", id)))?;
    let done = corner::cut_corners(&poly, corner, cut).map_err(|e| {
        refused(match e {
            CornerError::NotACorner(k) => Message::new("pipeline-corner-not")
                .arg("id", id)
                .arg("corner", k),
            CornerError::Straight(k) => Message::new("pipeline-corner-straight")
                .arg("id", id)
                .arg("corner", k),
            CornerError::NoCorners => Message::new("pipeline-corner-none").arg("id", id),
            CornerError::TooLarge { most } => Message::new("pipeline-corner-too-large")
                .arg("id", id)
                .arg("most", Value::Length(most, Digits::Fixed(3))),
            CornerError::BadSize => Message::new("pipeline-corner-size"),
        })
    })?;
    reshape(
        job,
        id,
        Geometry::Polyline {
            closed: done.closed,
            points: done.points.iter().map(|&(x, y)| [x, y]).collect(),
            bulges: done.bulges,
        },
    )
}

// -- trim, extend, break (§4)

/// The cutting edges: the segments of every shape the drawing shows, drawn and imported.
pub fn shown_cutters(job: &Job) -> Vec<chipload_core::primitives::Primitive> {
    job.drawing
        .shapes
        .iter()
        .filter(|s| !job.drawing.is_hidden(s))
        .flat_map(|s| s.segments().into_iter().map(|seg| seg.to_primitive()))
        .collect()
}

fn trim_refusal(id: u64, why: TrimError) -> PipelineError {
    let key = match why {
        TrimError::NothingCrosses => "pipeline-trim-nothing-crosses",
        TrimError::EndNotStraight => "pipeline-extend-not-straight",
        TrimError::NothingAhead => "pipeline-extend-nothing-ahead",
        TrimError::AtAnEnd => "pipeline-break-at-end",
        TrimError::Closed => "pipeline-extend-closed",
    };
    refused(Message::new(key).arg("id", id))
}

/// Shape `id` replaced by `pieces`: the first keeps its id (operations follow it), the
/// others are new shapes on its layer.
fn replace_by_pieces(job: &Job, shape: &Shape, pieces: &[Chain]) -> Result<Edited, PipelineError> {
    let Some((first, rest)) = pieces.split_first() else {
        return Err(refused(
            Message::new("pipeline-trim-everything").arg("id", shape.id),
        ));
    };
    let out = reshape(job, shape.id, polyline_of(first))?;
    let mut new_ids = Vec::new();
    let mut notes = Vec::new();
    let out = if rest.is_empty() {
        out
    } else {
        add_all(
            out,
            shape,
            rest.iter().map(polyline_of).collect(),
            &mut new_ids,
            &mut notes,
        )?
    };
    Ok(Edited {
        job: out,
        new_ids,
        notes,
    })
}

/// Trim drawn shape `id` at `at`: the piece under it, between the nearest crossings with
/// the shown shapes, goes. An open shape cut in its middle becomes two (the first keeps
/// the id); a closed one becomes one open polyline.
pub fn trim_shape(job: &Job, id: u64, at: (f64, f64)) -> Result<Edited, PipelineError> {
    let shape = drawn(job, id)?.clone();
    let pieces = trim::trim(&chain_of(&shape), &shown_cutters(job), at)
        .map_err(|why| trim_refusal(id, why))?;
    replace_by_pieces(job, &shape, &pieces)
}

/// The piece a trim of drawn shape `id` at `at` would remove, for a preview.
pub fn trim_preview(job: &Job, id: u64, at: (f64, f64)) -> Option<Geometry> {
    let shape = drawn(job, id).ok()?;
    trim::trimmed_piece(&chain_of(shape), &shown_cutters(job), at).map(|c| polyline_of(&c))
}

/// Break drawn shape `id` at the point of it nearest `at`: an open shape becomes two, a
/// closed one opens there.
pub fn break_shape(job: &Job, id: u64, at: (f64, f64)) -> Result<Edited, PipelineError> {
    let shape = drawn(job, id)?.clone();
    let pieces = trim::break_at(&chain_of(&shape), at).map_err(|why| trim_refusal(id, why))?;
    replace_by_pieces(job, &shape, &pieces)
}

/// Extend drawn open shape `id`'s straight `end` along itself to the nearest shown shape
/// ahead.
pub fn extend_shape(job: &Job, id: u64, end: End) -> Result<Edited, PipelineError> {
    let shape = drawn(job, id)?.clone();
    let longer = trim::extend(&chain_of(&shape), &shown_cutters(job), end)
        .map_err(|why| trim_refusal(id, why))?;
    Ok(Edited {
        job: reshape(job, id, polyline_of(&longer))?,
        new_ids: Vec::new(),
        notes: Vec::new(),
    })
}

/// Of shapes `ids`, the drawn one nearest `at` (a contour can join several drawn
/// polylines end to end; a canvas tool works on the one under the cursor).
pub fn nearest_drawn(job: &Job, ids: &[u64], at: (f64, f64)) -> Option<u64> {
    ids.iter()
        .filter_map(|&id| drawn(job, id).ok())
        .map(|shape| (offset::nearest_distance(&chain_of(shape), at), shape.id))
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, id)| id)
}

/// The point of drawn shape `id` nearest `at`: where a break there falls.
pub fn nearest_point(job: &Job, id: u64, at: (f64, f64)) -> Option<(f64, f64)> {
    let shape = drawn(job, id).ok()?;
    chain_of(shape)
        .prims
        .iter()
        .map(|prim| chipload_draw::intersect::nearest_on(prim, at))
        .min_by(|a, b| {
            let d = chipload_core::primitives::distance;
            d(*a, at).total_cmp(&d(*b, at))
        })
}

/// The end of open drawn shape `id` nearer `at`.
pub fn nearer_end(job: &Job, id: u64, at: (f64, f64)) -> Option<End> {
    let shape = drawn(job, id).ok()?;
    let chain = chain_of(shape);
    if chain.closed {
        return None;
    }
    let start = chain.prims.first()?.start();
    let end = chain.prims.last()?.end();
    let d = chipload_core::primitives::distance;
    Some(if d(at, start) <= d(at, end) {
        End::Start
    } else {
        End::End
    })
}

// -- union, subtract, intersect (§5)

/// Combine closed drawn shapes `ids` (two or more; for a subtraction the first is cut by the
/// others). A result that is one of the shapes unchanged keeps it as drawn, with its id;
/// the first other result takes the first shape's id (when that shape isn't kept as it
/// was), the rest are new shapes on its layer. The shapes not kept are removed, and the
/// operations that used them are flagged.
pub fn combine_shapes(job: &Job, op: Combine, ids: &[u64]) -> Result<Edited, PipelineError> {
    for (i, id) in ids.iter().enumerate() {
        if ids[..i].contains(id) {
            return Err(refused(Message::new("pipeline-shape-twice").arg("id", *id)));
        }
    }
    let shapes: Vec<Shape> = ids
        .iter()
        .map(|&id| drawn(job, id).cloned())
        .collect::<Result<_, _>>()?;
    let chains: Vec<Chain> = shapes.iter().map(chain_of).collect();
    let first = shapes.first().map(|s| s.id).unwrap_or_default();
    let pieces = boolean::combine(op, &chains).map_err(|e| match e {
        BooleanError::TooFew => refused(Message::new("pipeline-combine-two")),
        BooleanError::Open(i) => refused(Message::new("pipeline-combine-open").arg("id", ids[i])),
        BooleanError::Empty => refused(
            Message::new("pipeline-combine-empty")
                .arg("id", first)
                .arg("op", combine_word(op)),
        ),
        BooleanError::Library => refused(
            Message::new("pipeline-combine-failed").arg("why", "the boolean library failed"),
        ),
        BooleanError::Unverified(why) => {
            refused(Message::new("pipeline-combine-failed").arg("why", why))
        }
    })?;
    let kept: Vec<u64> = pieces
        .iter()
        .filter_map(|p| p.from)
        .map(|i| ids[i])
        .collect();
    let mut changed = pieces.iter().filter(|p| p.from.is_none()).map(|p| &p.chain);
    let mut out = job.clone();
    let mut survivors = kept.clone();
    if !kept.contains(&first)
        && let Some(chain) = changed.next()
    {
        out = reshape(&out, first, polyline_of(chain))?;
        survivors.push(first);
    }
    let mut new_ids = Vec::new();
    let mut notes = Vec::new();
    let rest: Vec<Geometry> = changed.map(polyline_of).collect();
    if !rest.is_empty() {
        out = add_all(out, &shapes[0], rest, &mut new_ids, &mut notes)?;
    }
    for &id in ids.iter().filter(|id| !survivors.contains(id)) {
        let (next, _) = delete_shape(&out, id)?;
        out = next;
        notes.extend(gone_note(job, id));
    }
    Ok(Edited {
        job: out,
        new_ids,
        notes,
    })
}

fn combine_word(op: Combine) -> &'static str {
    match op {
        Combine::Union => "union",
        Combine::Subtract => "subtract",
        Combine::Intersect => "intersect",
    }
}

/// A note naming the operations that selected shape `id`, which is gone: they are flagged
/// until their geometry is chosen again.
fn gone_note(job: &Job, id: u64) -> Option<Message> {
    let ops: Vec<String> = job
        .operations
        .iter()
        .enumerate()
        .filter(|(_, op)| op.select().shapes.as_ref().is_some_and(|s| s.contains(&id)))
        .map(|(i, _)| (i + 1).to_string())
        .collect();
    (!ops.is_empty()).then(|| {
        Message::new("pipeline-shape-gone")
            .arg("id", id)
            .arg("operations", ops.join(", "))
            .arg("ops", ops.len())
    })
}

// -- arrays (§6)

/// The most copies one array makes: more is a typing slip, not a drawing.
pub const ARRAY_MOST: u32 = 10_000;

/// How a polar array turns its copies.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Polar {
    /// The copies including the original.
    pub count: u32,
    /// The centre they go round, mm.
    pub center: (f64, f64),
    /// The sweep, degrees: 360 spreads them evenly round, less puts the first and last at
    /// its ends; negative goes clockwise.
    pub angle_deg: f64,
    /// Each copy turned with its position (false: moved only).
    pub rotate: bool,
}

/// A grid of copies of drawn shapes `ids`: `columns` × `rows` including the originals,
/// `dx`, `dy` apart (mm; negative goes left or down). The copies are new shapes on the
/// originals' layers.
pub fn array_grid(
    job: &Job,
    ids: &[u64],
    columns: u32,
    rows: u32,
    (dx, dy): (f64, f64),
) -> Result<Edited, PipelineError> {
    let total = u64::from(columns) * u64::from(rows);
    if columns == 0 || rows == 0 || total < 2 || total > u64::from(ARRAY_MOST) {
        return Err(refused(
            Message::new("pipeline-array-count")
                .arg("got", total)
                .arg("most", ARRAY_MOST),
        ));
    }
    if !(dx.is_finite() && dy.is_finite())
        || (columns > 1 && dx.abs() < CHAIN_TOL_MM)
        || (rows > 1 && dy.abs() < CHAIN_TOL_MM)
    {
        return Err(refused(Message::new("pipeline-array-spacing")));
    }
    let mut moves = Vec::new();
    for row in 0..rows {
        for column in 0..columns {
            if (row, column) != (0, 0) {
                moves.push((f64::from(column) * dx, f64::from(row) * dy, 0.0));
            }
        }
    }
    copies(job, ids, &moves, (0.0, 0.0))
}

/// Copies of drawn shapes `ids` round a centre (`polar`): the copies are new shapes on the
/// originals' layers.
pub fn array_polar(job: &Job, ids: &[u64], polar: Polar) -> Result<Edited, PipelineError> {
    let Polar {
        count,
        center,
        angle_deg,
        rotate,
    } = polar;
    if !(2..=ARRAY_MOST).contains(&count) {
        return Err(refused(
            Message::new("pipeline-array-count")
                .arg("got", count)
                .arg("most", ARRAY_MOST),
        ));
    }
    if !(angle_deg.is_finite() && angle_deg.abs() > 0.0 && angle_deg.abs() <= 360.0)
        || !(center.0.is_finite() && center.1.is_finite())
    {
        return Err(refused(
            Message::new("pipeline-array-angle")
                .arg("got", Value::Number(angle_deg, Digits::Shortest)),
        ));
    }
    // A whole turn: evenly round, the last copy one step short of the original.
    let step = if angle_deg.abs() == 360.0 {
        angle_deg / f64::from(count)
    } else {
        angle_deg / f64::from(count - 1)
    };
    let middle = crate::shapes::drawn_middle(job, ids)?;
    let moves: Vec<(f64, f64, f64)> = (1..count)
        .map(|k| {
            let turn = step * f64::from(k);
            if rotate {
                (0.0, 0.0, turn)
            } else {
                let (sin, cos) = turn.to_radians().sin_cos();
                let (x, y) = (middle.0 - center.0, middle.1 - center.1);
                let to = (center.0 + x * cos - y * sin, center.1 + x * sin + y * cos);
                (to.0 - middle.0, to.1 - middle.1, 0.0)
            }
        })
        .collect();
    copies(job, ids, &moves, center)
}

/// A copy of shapes `ids` for each of `moves`: moved by (dx, dy), then turned by its
/// degrees about `about`. The notes name the operations that now cut the copies.
fn copies(
    job: &Job,
    ids: &[u64],
    moves: &[(f64, f64, f64)],
    about: (f64, f64),
) -> Result<Edited, PipelineError> {
    let mut out = job.clone();
    let mut new_ids = Vec::new();
    for &(dx, dy, degrees) in moves {
        let (next, added) = crate::shapes::copy_shapes(&out, ids, (dx, dy))?;
        out = if degrees == 0.0 {
            next
        } else {
            crate::shapes::transform_shapes(&next, &added, about, about, 1.0, degrees)?
        };
        new_ids.extend(added);
    }
    let mut layers: Vec<(String, usize)> = Vec::new();
    for shape in out
        .drawing
        .shapes
        .iter()
        .filter(|s| new_ids.contains(&s.id))
    {
        match layers.iter_mut().find(|(l, _)| *l == shape.layer) {
            Some((_, n)) => *n += 1,
            None => layers.push((shape.layer.clone(), 1)),
        }
    }
    let notes = layers
        .iter()
        .filter_map(|(layer, n)| layer_cutters(&out, layer, *n))
        .collect();
    Ok(Edited {
        job: out,
        new_ids,
        notes,
    })
}

// -- align and distribute (§7)

/// Which side or middle of the boxes lines up.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlignTo {
    Left,
    Centre,
    Right,
    Top,
    Middle,
    Bottom,
}

type Bounds = ((f64, f64), (f64, f64));

/// Each drawn shape's box, in `ids`' order; refused when one is given twice.
fn boxes(job: &Job, ids: &[u64]) -> Result<Vec<(u64, Bounds)>, PipelineError> {
    ids.iter()
        .enumerate()
        .map(|(i, &id)| {
            if ids[..i].contains(&id) {
                return Err(refused(Message::new("pipeline-shape-twice").arg("id", id)));
            }
            drawn(job, id)?;
            crate::shapes::shapes_bounds(job, &[id])
                .map(|b| (id, b))
                .ok_or_else(|| no_shape(id))
        })
        .collect()
}

/// Move drawn shape `id` by `by`, keeping its id (operations and their points follow).
fn shift(job: Job, id: u64, by: (f64, f64)) -> Result<Job, PipelineError> {
    if by == (0.0, 0.0) {
        return Ok(job);
    }
    crate::shapes::transform_shapes(&job, &[id], (0.0, 0.0), by, 1.0, 0.0)
}

/// Line up drawn shapes `ids`: each box's side or middle `to` that of the box round them
/// all, or with `stock` the stock's. Shapes move whole and keep their ids. Without the
/// stock, two shapes or more.
pub fn align_shapes(
    job: &Job,
    ids: &[u64],
    to: AlignTo,
    stock: bool,
) -> Result<Job, PipelineError> {
    if ids.is_empty() || (ids.len() < 2 && !stock) {
        return Err(refused(Message::new("pipeline-align-two")));
    }
    let boxes = boxes(job, ids)?;
    let target = if stock {
        job.stock.xy_bounds()
    } else {
        boxes
            .iter()
            .map(|(_, b)| *b)
            .reduce(|(lo, hi), (a, b)| {
                (
                    (lo.0.min(a.0), lo.1.min(a.1)),
                    (hi.0.max(b.0), hi.1.max(b.1)),
                )
            })
            .unwrap_or_default()
    };
    let mut out = job.clone();
    for (id, (lo, hi)) in boxes {
        let ((tlo, thi), mid) = (target, |a: f64, b: f64| (a + b) / 2.0);
        let by = match to {
            AlignTo::Left => (tlo.0 - lo.0, 0.0),
            AlignTo::Right => (thi.0 - hi.0, 0.0),
            AlignTo::Centre => (mid(tlo.0, thi.0) - mid(lo.0, hi.0), 0.0),
            AlignTo::Bottom => (0.0, tlo.1 - lo.1),
            AlignTo::Top => (0.0, thi.1 - hi.1),
            AlignTo::Middle => (0.0, mid(tlo.1, thi.1) - mid(lo.1, hi.1)),
        };
        out = shift(out, id, by)?;
    }
    Ok(out)
}

/// Even out the gaps between drawn shapes `ids`' boxes, along X (`horizontal`) or Y: the
/// first and last (by where their boxes start) stay, the others move along that axis only.
/// Three shapes or more.
pub fn distribute_shapes(job: &Job, ids: &[u64], horizontal: bool) -> Result<Job, PipelineError> {
    if ids.len() < 3 {
        return Err(refused(Message::new("pipeline-distribute-three")));
    }
    let axis = |p: (f64, f64)| if horizontal { p.0 } else { p.1 };
    let mut spans: Vec<(u64, f64, f64)> = boxes(job, ids)?
        .into_iter()
        .map(|(id, (lo, hi))| (id, axis(lo), axis(hi)))
        .collect();
    spans.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
    let (first, last) = (spans[0], spans[spans.len() - 1]);
    let sizes: f64 = spans.iter().map(|(_, lo, hi)| hi - lo).sum();
    let gap = (last.2 - first.1 - sizes) / (spans.len() - 1) as f64;
    let mut out = job.clone();
    let mut at = first.2 + gap;
    for &(id, lo, hi) in &spans[1..spans.len() - 1] {
        let d = at - lo;
        out = shift(out, id, if horizontal { (d, 0.0) } else { (0.0, d) })?;
        at += hi - lo + gap;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chipload_job::load_job;
    use std::path::Path;

    fn drawn_part() -> Job {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/corpus/jobs/v5/drawn_part.chipload");
        load_job(&path).unwrap()
    }

    fn geometry(job: &Job, id: u64) -> &Geometry {
        &job.drawing
            .shapes
            .iter()
            .find(|s| s.id == id)
            .unwrap()
            .geometry
    }

    #[test]
    fn offsets_keep_the_kind_where_exact_and_go_on_the_layer() {
        // S1 rectangle 120 x 80, S2 circle r 12, S3 a triangle, all on layer "drawn".
        let job = drawn_part();
        let edited = offset_shapes(&job, &[1, 2, 3], 2.0, None, Corners::Round, 1).unwrap();
        assert_eq!(edited.new_ids, [4, 5, 6]);
        assert!(matches!(
            geometry(&edited.job, 4),
            Geometry::Rectangle { length, width, corner_radius, .. }
                if *length == 124.0 && *width == 84.0 && *corner_radius == 2.0
        ));
        assert!(
            matches!(geometry(&edited.job, 5), Geometry::Circle { radius, .. } if *radius == 14.0)
        );
        assert!(matches!(
            geometry(&edited.job, 6),
            Geometry::Polyline { closed: true, .. }
        ));
        // The originals stay.
        assert_eq!(geometry(&edited.job, 1), geometry(&job, 1));
        // Sharp: the rectangle keeps sharp corners; inside, twice.
        let sharp = offset_shapes(&job, &[1], 2.0, None, Corners::Sharp, 1).unwrap();
        assert!(matches!(
            geometry(&sharp.job, 4),
            Geometry::Rectangle { corner_radius, .. } if *corner_radius == 0.0
        ));
        let rings =
            offset_shapes(&job, &[2], 5.0, Some(OffsetSide::Inside), Corners::Round, 2).unwrap();
        assert!(
            matches!(geometry(&rings.job, 5), Geometry::Circle { radius, .. } if *radius == 2.0)
        );
        // A third ring would vanish: refused, and nothing is added.
        assert!(
            offset_shapes(&job, &[2], 5.0, Some(OffsetSide::Inside), Corners::Round, 3).is_err()
        );
    }

    #[test]
    fn corners_are_filleted_and_chamfered_keeping_the_id() {
        // S1: the turned 120 x 80 rectangle; S3: the triangle.
        let job = drawn_part();
        let round = cut_shape_corners(&job, 1, None, Cut::Fillet(10.0)).unwrap();
        assert!(matches!(
            geometry(&round, 1),
            Geometry::Rectangle { corner_radius, .. } if *corner_radius == 10.0
        ));
        let one = cut_shape_corners(&job, 1, Some(2), Cut::Chamfer(5.0)).unwrap();
        assert!(
            matches!(geometry(&one, 1), Geometry::Polyline { points, .. } if points.len() == 5)
        );
        let triangle = cut_shape_corners(&job, 3, None, Cut::Fillet(2.0)).unwrap();
        assert!(
            matches!(geometry(&triangle, 3), Geometry::Polyline { points, .. } if points.len() == 6)
        );
        // Too large: refused with the most that fits; a circle has no corners.
        let err = cut_shape_corners(&job, 3, None, Cut::Fillet(50.0)).unwrap_err();
        assert!(
            err.message().english().contains("at most"),
            "{}",
            err.message().english()
        );
        assert!(cut_shape_corners(&job, 2, None, Cut::Fillet(1.0)).is_err());
        assert_eq!(
            shape_corners(job.drawing.shapes.iter().find(|s| s.id == 3).unwrap()).len(),
            3
        );
    }

    #[test]
    fn operations_that_select_the_layer_are_named() {
        let mut job = drawn_part();
        if let Some(select) = job.operations[0].select_mut() {
            select.layer = Some("Drawn".into());
            select.shapes = None;
        }
        let edited = offset_shapes(&job, &[2], 1.0, None, Corners::Round, 1).unwrap();
        assert_eq!(edited.notes.len(), 1);
        let text = edited.notes[0].render(chipload_i18n::Language::En, Default::default());
        assert!(text.contains("operation 1"), "{text}");
    }

    #[test]
    fn trimming_a_shape_keeps_its_id_for_the_first_piece() {
        // A polyline across the circle S2 (r 12 at (75, 80)): trimmed inside the circle,
        // it becomes two, and the second is a new shape on its layer.
        let job = drawn_part();
        let (job, id) = add_shape(
            &job,
            "drawn",
            Geometry::straight_polyline(false, vec![[50.0, 80.0], [100.0, 80.0]]),
        )
        .unwrap();
        let edited = trim_shape(&job, id, (75.0, 80.0)).unwrap();
        assert_eq!(edited.new_ids.len(), 1);
        let Geometry::Polyline { points, .. } = geometry(&edited.job, id) else {
            panic!()
        };
        assert_eq!(points.len(), 2);
        assert!((points[1][0] - 63.0).abs() < 1e-9);
        // The circle trimmed by the line: an open arc, same id.
        let arc = trim_shape(&job, 2, (75.0, 92.0)).unwrap();
        assert!(matches!(
            geometry(&arc.job, 2),
            Geometry::Polyline { closed: false, .. }
        ));
        // Nothing crosses the triangle S3 (the line ends at x 100, before it): refused.
        assert!(trim_shape(&job, 3, (125.0, 95.0)).is_err());
        // Extend the line's end to the triangle, and break it.
        let longer = extend_shape(&job, id, End::End).unwrap();
        let Geometry::Polyline { points, .. } = geometry(&longer.job, id) else {
            panic!()
        };
        assert!(points[1][0] > 100.0 && points[1][0] < 125.0);
        let broken = break_shape(&job, id, (60.0, 80.0)).unwrap();
        assert_eq!(broken.new_ids.len(), 1);
    }

    fn ids(job: &Job) -> Vec<u64> {
        job.drawing.shapes.iter().map(|s| s.id).collect()
    }

    #[test]
    fn holes_cut_where_they_lie_leave_every_shape_as_drawn() {
        // S2 and S3 lie inside S1: cut out of it they are its holes, as they were.
        let job = drawn_part();
        let edited = combine_shapes(&job, Combine::Subtract, &[1, 2, 3]).unwrap();
        assert_eq!(edited.job.drawing.shapes, job.drawing.shapes);
        assert!(edited.new_ids.is_empty() && edited.notes.is_empty());
    }

    #[test]
    fn a_union_keeps_the_first_shapes_id_and_flags_the_others_operations() {
        let job = drawn_part();
        let (job, s4) = add_shape(
            &job,
            "drawn",
            Geometry::Circle {
                center: [87.0, 80.0],
                radius: 5.0,
            },
        )
        .unwrap();
        let edited = combine_shapes(&job, Combine::Union, &[s4, 2]).unwrap();
        assert_eq!(ids(&edited.job), [1, 3, s4]);
        assert!(matches!(
            geometry(&edited.job, s4),
            Geometry::Polyline { closed: true, .. }
        ));
        // Operation 1 profiled S2 (and S3).
        assert_eq!(edited.notes.len(), 1);
        assert_eq!(edited.notes[0].id, "pipeline-shape-gone");
    }

    #[test]
    fn an_intersection_that_is_one_of_the_shapes_keeps_that_shape() {
        // S2 lies inside S1: their intersection is S2, unchanged; S1 goes, and operation 2
        // that profiled it is flagged.
        let job = drawn_part();
        let edited = combine_shapes(&job, Combine::Intersect, &[1, 2]).unwrap();
        assert_eq!(ids(&edited.job), [2, 3]);
        assert_eq!(geometry(&edited.job, 2), geometry(&job, 2));
        assert_eq!(edited.notes.len(), 1);
    }

    #[test]
    fn combining_refuses_open_and_twice_given_shapes() {
        let job = drawn_part();
        let (job, open) = add_shape(
            &job,
            "drawn",
            Geometry::Polyline {
                closed: false,
                points: vec![[0.0, 0.0], [10.0, 0.0]],
                bulges: vec![0.0],
            },
        )
        .unwrap();
        let key = |r: Result<Edited, PipelineError>| match r {
            Err(PipelineError::Job(m)) => m.id,
            other => panic!("{other:?}"),
        };
        assert_eq!(
            key(combine_shapes(&job, Combine::Union, &[1, open])),
            "pipeline-combine-open"
        );
        assert_eq!(
            key(combine_shapes(&job, Combine::Union, &[1, 1])),
            "pipeline-shape-twice"
        );
        assert_eq!(
            key(combine_shapes(&job, Combine::Union, &[1])),
            "pipeline-combine-two"
        );
        assert_eq!(
            key(combine_shapes(&job, Combine::Subtract, &[2, 1])),
            "pipeline-combine-empty"
        );
    }

    #[test]
    fn a_grid_array_copies_on_the_layer_in_columns_and_rows() {
        let job = drawn_part();
        let edited = array_grid(&job, &[2], 3, 2, (30.0, -40.0)).unwrap();
        assert_eq!(edited.new_ids, [4, 5, 6, 7, 8]);
        let centres: Vec<[f64; 2]> = edited
            .new_ids
            .iter()
            .map(|&id| match geometry(&edited.job, id) {
                Geometry::Circle { center, .. } => *center,
                other => panic!("{other:?}"),
            })
            .collect();
        assert_eq!(
            centres,
            [
                [105.0, 80.0],
                [135.0, 80.0],
                [75.0, 40.0],
                [105.0, 40.0],
                [135.0, 40.0]
            ]
        );
        assert!(array_grid(&job, &[2], 1, 1, (30.0, 0.0)).is_err());
        assert!(array_grid(&job, &[2], 3, 1, (0.0, 10.0)).is_err());
        assert!(array_grid(&job, &[2], 200, 200, (1.0, 1.0)).is_err());
    }

    #[test]
    fn a_polar_array_of_a_whole_turn_has_no_copy_on_the_original() {
        let job = drawn_part();
        let polar = Polar {
            count: 4,
            center: (100.0, 80.0),
            angle_deg: 360.0,
            rotate: true,
        };
        let edited = array_polar(&job, &[2], polar).unwrap();
        let centres: Vec<(f64, f64)> = edited
            .new_ids
            .iter()
            .map(|&id| match geometry(&edited.job, id) {
                Geometry::Circle { center, .. } => (
                    (center[0] * 1e6).round() / 1e6,
                    (center[1] * 1e6).round() / 1e6,
                ),
                other => panic!("{other:?}"),
            })
            .collect();
        assert_eq!(centres, [(100.0, 55.0), (125.0, 80.0), (100.0, 105.0)]);
        // A sweep: the first and last at its ends.
        let half = array_polar(
            &job,
            &[2],
            Polar {
                count: 3,
                angle_deg: 180.0,
                ..polar
            },
        )
        .unwrap();
        assert!(matches!(geometry(&half.job, half.new_ids[1]),
            Geometry::Circle { center, .. } if (center[0] - 125.0).abs() < 1e-9 && (center[1] - 80.0).abs() < 1e-9));
        assert!(array_polar(&job, &[2], Polar { count: 1, ..polar }).is_err());
        assert!(
            array_polar(
                &job,
                &[2],
                Polar {
                    angle_deg: 0.0,
                    ..polar
                }
            )
            .is_err()
        );
    }

    #[test]
    fn a_polar_array_turns_its_copies_unless_asked_not_to() {
        // S1, a rectangle at 10°, copied a quarter turn round its own centre.
        let job = drawn_part();
        let polar = Polar {
            count: 2,
            center: (100.0, 80.0),
            angle_deg: 90.0,
            rotate: true,
        };
        let turned = array_polar(&job, &[1], polar).unwrap();
        assert!(matches!(geometry(&turned.job, 4),
            Geometry::Rectangle { angle, .. } if (angle.to_degrees() - 100.0).abs() < 1e-9));
        let moved = array_polar(
            &job,
            &[1],
            Polar {
                rotate: false,
                ..polar
            },
        )
        .unwrap();
        assert!(matches!(geometry(&moved.job, 4),
            Geometry::Rectangle { angle, .. } if (angle.to_degrees() - 10.0).abs() < 1e-9));
    }

    fn bounds_of(job: &Job, id: u64) -> ((f64, f64), (f64, f64)) {
        crate::shapes::shapes_bounds(job, &[id]).unwrap()
    }

    fn near(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn shapes_align_to_their_box_or_the_stock() {
        // S2 a circle at 75,80 r 12; S3 a triangle from 110,65 to 140,95.
        let job = drawn_part();
        let left = align_shapes(&job, &[2, 3], AlignTo::Left, false).unwrap();
        assert!(near(bounds_of(&left, 3).0.0, 63.0));
        assert!(near(bounds_of(&left, 2).0.0, 63.0));
        let top = align_shapes(&job, &[2, 3], AlignTo::Top, false).unwrap();
        assert!(near(bounds_of(&top, 2).1.1, 95.0));
        let stock = job.stock.xy_bounds();
        let middle = align_shapes(&job, &[2], AlignTo::Centre, true).unwrap();
        assert!(near(
            (bounds_of(&middle, 2).0.0 + bounds_of(&middle, 2).1.0) / 2.0,
            (stock.0.0 + stock.1.0) / 2.0
        ));
        assert!(align_shapes(&job, &[2], AlignTo::Left, false).is_err());
    }

    #[test]
    fn distributing_evens_the_gaps_and_keeps_the_ends() {
        let job = drawn_part();
        let (job, a) = add_shape(
            &job,
            "drawn",
            Geometry::Circle {
                center: [0.0, 0.0],
                radius: 5.0,
            },
        )
        .unwrap();
        let (job, b) = add_shape(
            &job,
            "drawn",
            Geometry::Circle {
                center: [12.0, 0.0],
                radius: 5.0,
            },
        )
        .unwrap();
        let (job, c) = add_shape(
            &job,
            "drawn",
            Geometry::Circle {
                center: [100.0, 0.0],
                radius: 10.0,
            },
        )
        .unwrap();
        let out = distribute_shapes(&job, &[c, a, b], true).unwrap();
        // From -5 to 110, 40 mm of circles: two gaps of 37.5.
        assert!(near(bounds_of(&out, a).0.0, -5.0));
        assert!(near(bounds_of(&out, b).0.0, 42.5));
        assert!(near(bounds_of(&out, c).0.0, 90.0));
        assert!(distribute_shapes(&job, &[a, b], true).is_err());
    }
}
