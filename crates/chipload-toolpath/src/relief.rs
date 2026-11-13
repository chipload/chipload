// SPDX-License-Identifier: GPL-3.0-or-later
//! Reliefs (docs/specs/stl-relief.md §3, §5, §7): a model placed on the board, carved in
//! parallel lines across an area, each point as low as the drop cutter lets the tool go,
//! and the finished path checked against the model by a test of its own.

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use chipload_core::Point;
use chipload_core::cutter::Cutter;
use chipload_core::tolerances::{ARC_APPROX_TOL_MM, CHAIN_TOL_MM, DEPTH_TOL_MM};
use chipload_i18n::{Message, Value};
use chipload_job::{Model, ModelUp};
use chipload_mesh::{Mesh, Up, Vertex};

use crate::drop::DropMesh;
use crate::model::{Move, MoveKind};
use crate::rim::{Area, Ramp};

/// Points along a line are first this far apart at most, mm; more are added where the
/// surface asks (`refine`).
const FIRST_SPACING_MM: f64 = 0.5;
/// A line's points are split in two at most this many times: 1/256 of the first spacing.
const MAX_REFINE: u32 = 8;
/// How far below the surface the refinement and the joining of points may each leave the
/// path: together under the arc tolerance the check holds it to.
const SHARE_OF_TOLERANCE: f64 = 0.4;
/// The path check tests at most this many tool positions.
const VERIFY_POSITIONS: usize = 2000;
/// ... and at most about this many triangle tests in all.
const VERIFY_WORK: usize = 60_000_000;

/// Why a relief can't be made.
#[derive(Debug, Clone, PartialEq)]
pub enum ReliefError {
    /// A message (`toolpath.ftl`).
    Refused(Message),
}

impl ReliefError {
    pub fn message(&self) -> Message {
        match self {
            ReliefError::Refused(m) => m.clone(),
        }
    }
}

fn refused(message: Message) -> ReliefError {
    ReliefError::Refused(message)
}

fn up(up: ModelUp) -> Up {
    match up {
        ModelUp::Z => Up::Z,
        ModelUp::Y => Up::Y,
        ModelUp::X => Up::X,
        ModelUp::MinusZ => Up::MinusZ,
        ModelUp::MinusY => Up::MinusY,
        ModelUp::MinusX => Up::MinusX,
    }
}

/// How a model lies on the board: its size there and the scales that put it there.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placement {
    pub lo: [f64; 3],
    pub hi: [f64; 3],
    pub scale: f64,
    pub z_scale: f64,
    pub width: f64,
    pub height: f64,
    pub depth: f64,
}

/// Why a model's file could not be read: what the text alone tells, without the model's id.
enum ReadFailure {
    /// Not base64: the decoder's words.
    Data(String),
    /// Not an STL file Chipload reads.
    Mesh(Message),
    /// The reader gave up (a panic, caught).
    Reader,
}

/// A model's file as read: the full mesh, what is cut; its light copy, for showing it only
/// (docs/specs/relief-wizard.md §3); and the full mesh's box.
struct ReadMesh {
    full: Mesh,
    light: Mesh,
    bounds: (Vertex, Vertex),
}

impl ReadMesh {
    fn of(full: Mesh) -> Result<ReadMesh, ReadFailure> {
        let bounds = full
            .bounds()
            .ok_or(ReadFailure::Mesh(Message::new("mesh-error-empty")))?;
        let light = full.light(chipload_mesh::LIGHT_TRIANGLES);
        Ok(ReadMesh {
            full,
            light,
            bounds,
        })
    }
}

/// The model's file read, as its text gives it: decoded and parsed once for the text, and
/// remembered with it (docs/specs/relief-wizard.md §2).
fn read_model(model: &Model) -> Result<std::sync::Arc<Result<ReadMesh, ReadFailure>>, ReliefError> {
    let read = model.data.remembered(|text| {
        let bytes = STANDARD
            .decode(text)
            .map_err(|e| ReadFailure::Data(e.to_string()))?;
        match std::panic::catch_unwind(|| chipload_mesh::read_stl(&bytes)) {
            Ok(Ok((mesh, _))) => ReadMesh::of(mesh),
            Ok(Err(e)) => Err(ReadFailure::Mesh(e.message())),
            Err(_) => Err(ReadFailure::Reader),
        }
    });
    let unreadable = |error: String| {
        refused(
            Message::new("toolpath-model-unreadable")
                .arg("id", model.id)
                .arg("error", Value::Text(error)),
        )
    };
    match &*read {
        Ok(_) => Ok(read),
        Err(ReadFailure::Data(error)) => Err(unreadable(error.clone())),
        Err(ReadFailure::Mesh(message)) => Err(refused(message.clone())),
        Err(ReadFailure::Reader) => Err(unreadable(String::new())),
    }
}

/// The read mesh of a read that succeeded ([`read_model`] returns only those).
fn read_mesh(read: &Result<ReadMesh, ReadFailure>) -> Result<&ReadMesh, ReliefError> {
    read.as_ref()
        .map_err(|_| refused(Message::new("mesh-error-empty")))
}

/// A model's data from its file's base64 `text` and the mesh already read from it, so it
/// isn't read again (docs/specs/relief-wizard.md §2). `mesh` must be what the text reads
/// as. Its light copy is made here too, on the caller's thread.
pub fn model_data(text: String, mesh: Mesh) -> chipload_job::ModelData {
    let data = chipload_job::ModelData::new(text);
    data.remembered(|_| ReadMesh::of(mesh));
    data
}

/// The model's unit as a scale to mm.
fn to_mm(model: &Model) -> f64 {
    match model.units {
        chipload_job::Units::Mm => 1.0,
        chipload_job::Units::Inch => 25.4,
    }
}

/// `mesh` turned so the model's up axis points up, in mm.
fn turned_in_mm(model: &Model, mesh: &Mesh) -> Mesh {
    let (turn, k) = (up(model.up), to_mm(model));
    mesh.mapped(|v| {
        let t = turn.turn(v);
        [t[0] * k, t[1] * k, t[2] * k]
    })
}

/// The model's mesh read from the project, turned so its up axis points up, in mm as its
/// unit reads, and cut at its cut level: the part kept (docs/specs/relief-wizard.md §5).
pub fn turned_mesh(model: &Model) -> Result<Mesh, ReliefError> {
    let read = read_model(model)?;
    let turned = turned_in_mm(model, &read_mesh(&read)?.full);
    Ok(match cut_level(model)? {
        Some(level) => turned.above(level),
        None => turned,
    })
}

/// The Z the turned model is cut at, or None when it is kept whole: `keep` of its height
/// from its top (docs/specs/relief-wizard.md §5).
fn cut_level(model: &Model) -> Result<Option<f64>, ReliefError> {
    if model.keep >= 1.0 {
        return Ok(None);
    }
    let (lo, hi) = whole_turned_bounds(model)?;
    Ok(Some(hi[2] - model.keep * (hi[2] - lo[2])))
}

/// The box of [`turned_mesh`]: for a model kept whole, its own box's corners turned (a
/// turn of axes keeps a box a box), without turning the mesh; for one cut, the kept
/// part's.
pub fn turned_bounds(model: &Model) -> Result<(Vertex, Vertex), ReliefError> {
    if model.keep >= 1.0 {
        return whole_turned_bounds(model);
    }
    turned_mesh(model)?
        .bounds()
        .ok_or_else(|| refused(Message::new("pipeline-model-nothing-kept").arg("id", model.id)))
}

/// The box of the whole turned model, before any cut.
fn whole_turned_bounds(model: &Model) -> Result<(Vertex, Vertex), ReliefError> {
    let read = read_model(model)?;
    let (lo, hi) = read_mesh(&read)?.bounds;
    let (turn, k) = (up(model.up), to_mm(model));
    let mut out: Option<(Vertex, Vertex)> = None;
    for corner in 0..8 {
        let pick = |axis: usize| {
            if corner & (1 << axis) == 0 {
                lo[axis]
            } else {
                hi[axis]
            }
        };
        let t = turn.turn([pick(0), pick(1), pick(2)]);
        let v = [t[0] * k, t[1] * k, t[2] * k];
        out = Some(match out {
            None => (v, v),
            Some((a, b)) => (
                [a[0].min(v[0]), a[1].min(v[1]), a[2].min(v[2])],
                [b[0].max(v[0]), b[1].max(v[1]), b[2].max(v[2])],
            ),
        });
    }
    out.ok_or_else(|| refused(Message::new("mesh-error-empty")))
}

/// Where `model`'s turned mesh lies on the board (spec §3): its depth in proportion to the
/// width when the model says so.
pub fn placement(model: &Model, turned: &Mesh) -> Result<Placement, ReliefError> {
    let (lo, hi) = turned
        .bounds()
        .ok_or_else(|| refused(Message::new("mesh-error-empty")))?;
    placement_in(model, lo, hi)
}

/// [`placement`] from the turned mesh's box.
pub fn placement_in(model: &Model, lo: Vertex, hi: Vertex) -> Result<Placement, ReliefError> {
    let own_width = hi[0] - lo[0];
    if own_width <= 0.0 {
        return Err(refused(
            Message::new("pipeline-model-no-width").arg("id", model.id),
        ));
    }
    let scale = model.width / own_width;
    let tall = hi[2] - lo[2];
    let depth = if model.proportional {
        tall * scale
    } else {
        model.depth
    };
    Ok(Placement {
        lo,
        hi,
        scale,
        z_scale: if tall > 0.0 { depth / tall } else { 0.0 },
        width: model.width,
        height: (hi[1] - lo[1]) * scale,
        depth,
    })
}

/// The turned mesh on the board, in the drawing's coordinates: scaled, its footprint's
/// lower-left corner at the model's (x, y), turned about the footprint's centre, its top at
/// Z 0 and its lowest point at −depth.
pub fn placed(model: &Model, turned: &Mesh, p: &Placement) -> Mesh {
    let (cx, cy) = (p.width / 2.0, p.height / 2.0);
    let (sin, cos) = model.rotation.to_radians().sin_cos();
    turned.mapped(|v| {
        let (u, w) = (
            (v[0] - p.lo[0]) * p.scale - cx,
            (v[1] - p.lo[1]) * p.scale - cy,
        );
        [
            model.x + cx + u * cos - w * sin,
            model.y + cy + u * sin + w * cos,
            (v[2] - p.hi[2]) * p.z_scale,
        ]
    })
}

/// `model` placed on the board ([`placed`]).
pub fn placed_mesh(model: &Model) -> Result<Mesh, ReliefError> {
    let turned = turned_mesh(model)?;
    let p = placement(model, &turned)?;
    Ok(placed(model, &turned, &p))
}

/// `model`'s light copy turned so its up axis points up, in mm, whole, and the Z its cut
/// level lies at (None: kept whole): for showing it while it is turned and cut
/// (docs/specs/relief-wizard.md §4).
pub fn turned_light(model: &Model) -> Result<(Mesh, Option<f64>), ReliefError> {
    let read = read_model(model)?;
    let light = turned_in_mm(model, &read_mesh(&read)?.light);
    Ok((light, cut_level(model)?))
}

/// `model`'s light copy placed on the board as the full mesh is: for showing it only, never
/// for cutting (docs/specs/relief-wizard.md §3).
pub fn placed_light_mesh(model: &Model) -> Result<Mesh, ReliefError> {
    let read = read_model(model)?;
    let read = read_mesh(&read)?;
    let (lo, hi) = turned_bounds(model)?;
    let p = placement_in(model, lo, hi)?;
    let light = turned_in_mm(model, &read.light);
    let light = match cut_level(model)? {
        Some(level) => light.above(level),
        None => light,
    };
    Ok(placed(model, &light, &p))
}

/// What a relief's finishing lines are made from.
pub struct ReliefCut<'a> {
    /// The model on the board, its top at Z `top`.
    pub mesh: &'a DropMesh,
    pub cutter: Cutter,
    /// The model's top in program Z, and the lowest the tool goes (its lowest point).
    pub top: f64,
    pub floor: f64,
    /// The area the tool's centre stays in: closed loops, holes by the even-odd rule, or
    /// loops grown by a rim (spec §16).
    pub area: &'a Area,
    /// The rim's ramp, a surface besides the model (spec §16).
    pub ramp: Option<&'a Ramp>,
    /// Between lines, mm.
    pub step: f64,
    /// The lines' direction, radians from X.
    pub angle: f64,
    pub zigzag: bool,
    pub safe_z: f64,
    pub feed: f64,
    pub plunge_feed: f64,
    /// The copy's shift: the area is moved already, the model is moved by this.
    pub shift: Point,
}

impl ReliefCut<'_> {
    /// The tip's height over `p`: as the drop cutter has it, never below the floor.
    fn z_at(&self, p: Point) -> f64 {
        let drop = self
            .mesh
            .drop(self.cutter, (p.0 - self.shift.0, p.1 - self.shift.1))
            .map_or(f64::NEG_INFINITY, |z| self.top + z);
        let ramp = self
            .ramp
            .map_or(f64::NEG_INFINITY, |r| r.drop(self.cutter, p));
        drop.max(ramp).max(self.floor)
    }
}

/// One stretch the tool follows the surface along: points with their tip heights.
type Run = Vec<(Point, f64)>;

/// The relief's moves from above `start`, and where the tool ends up.
pub fn relief_moves(cut: &ReliefCut, start: Point) -> (Vec<Move>, Point) {
    let runs = runs(cut);
    let mut moves = vec![Move::rapid_z(cut.safe_z)];
    let mut end = start;
    let mut at: Option<(Point, f64)> = None;
    for run in runs {
        let Some(&first) = run.first() else {
            continue;
        };
        match at {
            // The next line's start is near: along the surface to it, inside the area.
            Some(from)
                if near(from.0, first.0, cut.step) && cut.area.link_inside(from.0, first.0) =>
            {
                for (p, z) in follow(cut, from.0, first.0).into_iter().skip(1) {
                    moves.push(Move::feed_xyz(p, z, cut.feed));
                }
            }
            _ => {
                if at.is_some() {
                    moves.push(Move::rapid_z(cut.safe_z));
                }
                moves.push(Move::rapid_xy(first.0));
                moves.push(Move::feed_z(first.1, cut.plunge_feed));
            }
        }
        for &(p, z) in &run[1..] {
            moves.push(Move::feed_xyz(p, z, cut.feed));
        }
        let last = run[run.len() - 1];
        at = Some(last);
        end = last.0;
    }
    moves.push(Move::rapid_z(cut.safe_z));
    (moves, end)
}

/// Whether the tool may go along the surface from `a` to `b` rather than lifting: the next
/// line's start, close by.
fn near(a: Point, b: Point, step: f64) -> bool {
    (b.0 - a.0).hypot(b.1 - a.1) <= 2.0 * step + CHAIN_TOL_MM
}

/// The finishing lines across the area, each cut into the runs that lie inside it, in
/// cutting order: back and forth, or all the same way.
/// Each stretch is worked on its own, on all cores (spec §15).
fn runs(cut: &ReliefCut) -> Vec<Run> {
    let stretches: Vec<(Point, Point)> = lines(cut).into_iter().flatten().collect();
    crate::parallel::map_ordered(&stretches, |&(a, b)| follow(cut, a, b))
}

/// The lines across the area, one stepover apart, each as the stretches (start, end) that
/// lie inside it, in cutting order: back and forth, or all the same way.
fn lines(cut: &ReliefCut) -> Vec<Vec<(Point, Point)>> {
    let (sin, cos) = cut.angle.sin_cos();
    // (u, v): u along the lines, v across them.
    let to_uv = |p: Point| (p.0 * cos + p.1 * sin, -p.0 * sin + p.1 * cos);
    let from_uv = |u: f64, v: f64| (u * cos - v * sin, u * sin + v * cos);
    let area = cut.area.mapped(to_uv);
    let Some((v0, v1)) = area.y_range() else {
        return Vec::new();
    };
    let count = ((v1 - v0) / cut.step).ceil().max(1.0) as usize;
    let gap = (v1 - v0) / count as f64;
    let mut out = Vec::new();
    for k in 0..count {
        let v = v0 + (k as f64 + 0.5) * gap;
        let mut spans = area.spans(v);
        let backwards = cut.zigzag && k % 2 == 1;
        if backwards {
            spans.reverse();
        }
        out.push(
            spans
                .into_iter()
                .map(|(a, b)| {
                    let (from, to) = if backwards { (b, a) } else { (a, b) };
                    (from_uv(from, v), from_uv(to, v))
                })
                .collect(),
        );
    }
    out
}

/// Where the line `v` lies inside the loops (even-odd), as (u0, u1) from low to high u.
pub(crate) fn loop_spans(loops: &[Vec<Point>], v: f64) -> Vec<(f64, f64)> {
    let mut cross: Vec<f64> = Vec::new();
    for l in loops {
        for k in 0..l.len() {
            let (a, b) = (l[k], l[(k + 1) % l.len()]);
            // Half-open in v, so a corner on the line counts once.
            if (a.1 <= v) != (b.1 <= v) {
                let t = (v - a.1) / (b.1 - a.1);
                cross.push(a.0 + t * (b.0 - a.0));
            }
        }
    }
    cross.sort_by(f64::total_cmp);
    cross
        .as_chunks::<2>()
        .0
        .iter()
        .map(|c| (c[0], c[1]))
        .filter(|(a, b)| b - a > CHAIN_TOL_MM)
        .collect()
}

/// The tool along the straight line from `a` to `b` on the surface: points no farther
/// apart than [`FIRST_SPACING_MM`], with more where the tool must stand higher between two
/// of them than the straight move gives, then joined where they lie on one line within the
/// arc tolerance (spec §5).
fn follow(cut: &ReliefCut, a: Point, b: Point) -> Run {
    simplified(sampled(cut, a, b))
}

/// [`follow`]'s points before they are joined.
fn sampled(cut: &ReliefCut, a: Point, b: Point) -> Run {
    let length = (b.0 - a.0).hypot(b.1 - a.1);
    let pieces = (length / FIRST_SPACING_MM.min(cut.step)).ceil().max(1.0) as usize;
    let at = |t: f64| (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t);
    let mut points: Run = Vec::with_capacity(pieces + 1);
    let mut prev = (a, cut.z_at(a));
    points.push(prev);
    for k in 1..=pieces {
        let p = at(k as f64 / pieces as f64);
        let next = (p, cut.z_at(p));
        refine(cut, prev, next, MAX_REFINE, &mut points);
        points.push(next);
        prev = next;
    }
    points
}

/// The levels a roughing goes down to (spec §14 step 3): one step down apart from
/// `stock_top`, the last at `bottom`. None when the stock top is not above it.
pub fn levels(stock_top: f64, bottom: f64, step_down: f64) -> Vec<f64> {
    let depth = stock_top - bottom;
    if depth <= DEPTH_TOL_MM || step_down <= 0.0 {
        return Vec::new();
    }
    let count = ((depth - DEPTH_TOL_MM) / step_down).ceil().max(1.0) as usize;
    (1..=count)
        .map(|k| (stock_top - k as f64 * step_down).max(bottom))
        .collect()
}

/// A relief's roughing (spec §14 step 3), from above `start`: `cut` is made with the tool
/// grown by the allowance and its top and floor raised by it, so its heights keep the
/// allowance. Level by level from `stock_top`, each line cut where material is left above
/// the level, stretches joined along the next line when near. The moves and where the
/// tool ends up.
pub fn rough_moves(
    cut: &ReliefCut,
    stock_top: f64,
    step_down: f64,
    start: Point,
) -> (Vec<Move>, Point) {
    // Each line's heights, found once for every level, the lines on all cores (spec §15).
    let lines: Vec<Vec<Run>> = crate::parallel::map_ordered(&lines(cut), |line| {
        line.iter().map(|&(a, b)| sampled(cut, a, b)).collect()
    });
    let mut moves = vec![Move::rapid_z(cut.safe_z)];
    let mut end = start;
    let mut down = false;
    let mut above = stock_top;
    for level in levels(stock_top, cut.floor, step_down) {
        let mut left: Vec<Vec<Option<Run>>> = lines
            .iter()
            .map(|line| {
                line.iter()
                    .flat_map(|run| stretches(run, above, level))
                    .map(Some)
                    .collect()
            })
            .collect();
        let mut at: Option<(Point, usize)> = None;
        while let Some((line, k, reversed, linked)) = next_stretch(cut, &left, at) {
            let Some(mut run) = left[line][k].take() else {
                break;
            };
            if reversed {
                run.reverse();
            }
            let Some(&first) = run.first() else {
                continue;
            };
            match (linked, at) {
                (true, Some((from, _))) => {
                    let link: Run = sampled(cut, from, first.0)
                        .into_iter()
                        .map(|(p, z)| (p, z.max(level)))
                        .collect();
                    for (p, z) in simplified(link).into_iter().skip(1) {
                        moves.push(Move::feed_xyz(p, z, cut.feed));
                    }
                }
                _ => {
                    if down {
                        moves.push(Move::rapid_z(cut.safe_z));
                    }
                    moves.push(Move::rapid_xy(first.0));
                    moves.push(Move::feed_z(first.1, cut.plunge_feed));
                    down = true;
                }
            }
            for &(p, z) in &run[1..] {
                moves.push(Move::feed_xyz(p, z, cut.feed));
            }
            let last = run[run.len() - 1].0;
            at = Some((last, line));
            end = last;
        }
        above = level;
    }
    moves.push(Move::rapid_z(cut.safe_z));
    (moves, end)
}

/// The next stretch to cut: on the line after the last one, its nearer end close to where
/// the tool is and joined to it by a straight move inside the area (reversed when that end
/// is its last, back and forth only); otherwise the first one left, in line order. The
/// line, its index there, whether it is reversed, and whether it is joined.
fn next_stretch(
    cut: &ReliefCut,
    left: &[Vec<Option<Run>>],
    at: Option<(Point, usize)>,
) -> Option<(usize, usize, bool, bool)> {
    if let Some((p, line)) = at
        && let Some(next) = left.get(line + 1)
    {
        for (k, run) in next.iter().enumerate() {
            let Some(run) = run else {
                continue;
            };
            let ends = [(run.first(), false), (run.last(), true)];
            for (end, reversed) in ends {
                let Some(&(q, _)) = end else {
                    continue;
                };
                if (cut.zigzag || !reversed) && near(p, q, cut.step) && cut.area.link_inside(p, q) {
                    return Some((line + 1, k, reversed, true));
                }
            }
        }
    }
    left.iter().enumerate().find_map(|(line, runs)| {
        runs.iter()
            .position(Option::is_some)
            .map(|k| (line, k, false, false))
    })
}

/// The stretches of a line's points `run` that a level at `level` cuts: where material is
/// left above it (the height before, `above`, higher than the surface's), with the points
/// on each side so the tool comes onto them along the surface, at the level or the surface,
/// whichever is higher, then joined.
fn stretches(run: &Run, above: f64, level: f64) -> Vec<Run> {
    let cuts: Vec<bool> = run
        .iter()
        .map(|&(_, z)| above - z > ARC_APPROX_TOL_MM)
        .collect();
    let keep =
        |i: usize| cuts[i] || (i > 0 && cuts[i - 1]) || cuts.get(i + 1).copied().unwrap_or(false);
    let mut out = Vec::new();
    let mut current: Run = Vec::new();
    for (i, &(p, z)) in run.iter().enumerate() {
        if keep(i) {
            current.push((p, z.max(level)));
        } else if !current.is_empty() {
            out.push(simplified(std::mem::take(&mut current)));
        }
    }
    if !current.is_empty() {
        out.push(simplified(current));
    }
    out
}

/// Whether the straight move from `a` to `b` stays inside the area: it crosses none of its
/// edges and its middle and quarter points lie inside (even-odd) or on an edge. `a` and `b`
/// may lie on the edges, and the move may run along one.
pub(crate) fn inside_loops_link(area: &[Vec<Point>], a: Point, b: Point) -> bool {
    let at = |t: f64| (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t);
    let within = |p: Point| contains(area, p) || on_edge(area, p);
    if ![0.25, 0.5, 0.75].iter().all(|&t| within(at(t))) {
        return false;
    }
    !area
        .iter()
        .any(|l| (0..l.len()).any(|k| crosses(a, b, l[k], l[(k + 1) % l.len()])))
}

/// Whether `p` lies inside the loop `l`.
pub fn encloses(l: &[Point], p: Point) -> bool {
    let mut inside = false;
    for k in 0..l.len() {
        let (a, b) = (l[k], l[(k + 1) % l.len()]);
        if (a.1 <= p.1) != (b.1 <= p.1) {
            let x = a.0 + (p.1 - a.1) / (b.1 - a.1) * (b.0 - a.0);
            if x > p.0 {
                inside = !inside;
            }
        }
    }
    inside
}

/// Whether `p` lies on an edge of the loops, within the chaining tolerance.
fn on_edge(area: &[Vec<Point>], p: Point) -> bool {
    area.iter().any(|l| {
        (0..l.len()).any(|k| {
            let (a, b) = (l[k], l[(k + 1) % l.len()]);
            let (dx, dy) = (b.0 - a.0, b.1 - a.1);
            let length2 = dx * dx + dy * dy;
            let t = if length2 > 0.0 {
                (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / length2).clamp(0.0, 1.0)
            } else {
                0.0
            };
            (a.0 + t * dx - p.0).hypot(a.1 + t * dy - p.1) <= CHAIN_TOL_MM
        })
    })
}

/// Whether `p` lies inside the loops by the even-odd rule.
fn contains(area: &[Vec<Point>], p: Point) -> bool {
    area.iter().filter(|l| encloses(l, p)).count() % 2 == 1
}

/// Whether the segments `ab` and `cd` cross each other, each end of one lying clearly on
/// its own side of the other (more than the chaining tolerance): touching does not count.
fn crosses(a: Point, b: Point, c: Point, d: Point) -> bool {
    // The signed distance of `p` from the line through `s` and `e`; 0 when within the
    // tolerance.
    let side = |s: Point, e: Point, p: Point| {
        let length = (e.0 - s.0).hypot(e.1 - s.1);
        if length <= CHAIN_TOL_MM {
            return 0.0;
        }
        let d = ((e.0 - s.0) * (p.1 - s.1) - (e.1 - s.1) * (p.0 - s.0)) / length;
        if d.abs() <= CHAIN_TOL_MM {
            0.0
        } else {
            d.signum()
        }
    };
    side(a, b, c) * side(a, b, d) < 0.0 && side(c, d, a) * side(c, d, b) < 0.0
}

/// Points between `a` and `b` where the tool must stand higher than the straight move from
/// `a` to `b` by more than [`SHARE_OF_TOLERANCE`] of the arc tolerance, found by halving.
fn refine(cut: &ReliefCut, a: (Point, f64), b: (Point, f64), depth: u32, out: &mut Run) {
    if depth == 0 {
        return;
    }
    let m = ((a.0.0 + b.0.0) / 2.0, (a.0.1 + b.0.1) / 2.0);
    let z = cut.z_at(m);
    if z - (a.1 + b.1) / 2.0 <= SHARE_OF_TOLERANCE * ARC_APPROX_TOL_MM {
        return;
    }
    let mid = (m, z);
    refine(cut, a, mid, depth - 1, out);
    out.push(mid);
    refine(cut, mid, b, depth - 1, out);
}

/// Points dropped where the straight move past them stays within the arc tolerance above
/// them and within [`SHARE_OF_TOLERANCE`] of it below: the move may run a little above,
/// barely deeper.
fn simplified(points: Run) -> Run {
    if points.len() < 3 {
        return points;
    }
    let mut out = vec![points[0]];
    let mut anchor = 0;
    let mut k = 2;
    while k < points.len() {
        let (a, b) = (points[anchor], points[k]);
        let length = (b.0.0 - a.0.0).hypot(b.0.1 - a.0.1);
        let fits = (anchor + 1..k).all(|i| {
            let p = points[i];
            let t = if length > 0.0 {
                ((p.0.0 - a.0.0).hypot(p.0.1 - a.0.1) / length).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let chord = a.1 + (b.1 - a.1) * t;
            // Below the point would cut into what the point keeps clear.
            p.1 - chord <= SHARE_OF_TOLERANCE * ARC_APPROX_TOL_MM
                && chord - p.1 <= ARC_APPROX_TOL_MM
        });
        if fits {
            k += 1;
        } else {
            out.push(points[k - 1]);
            anchor = k - 1;
            k = anchor + 2;
        }
    }
    out.push(points[points.len() - 1]);
    out
}

/// The path check (spec §7), apart from the drop cutter: at sampled tool positions of the
/// cutting moves, no corner of the model and no point sampled on its faces lies inside the
/// tool by more than the arc tolerance. Brute force over the triangles near each position,
/// so it stays simple. Returns the worst cut into the model and where, if any.
pub fn verify(
    moves: &[Move],
    mesh: &Mesh,
    cutter: Cutter,
    top: f64,
    shift: Point,
) -> Option<(f64, Point)> {
    let positions = positions(moves);
    if positions.is_empty() || mesh.triangles.is_empty() {
        return None;
    }
    let per = (VERIFY_WORK / mesh.triangles.len().max(1)).clamp(20, VERIFY_POSITIONS);
    let stride = positions.len().div_ceil(per).max(1);
    let r = cutter.radius();
    let mut worst: Option<(f64, Point)> = None;
    for &(p, z) in positions.iter().step_by(stride) {
        let c = (p.0 - shift.0, p.1 - shift.1);
        for t in &mesh.triangles {
            let near = t.iter().all(|v| v[0] < c.0 - r)
                || t.iter().all(|v| v[0] > c.0 + r)
                || t.iter().all(|v| v[1] < c.1 - r)
                || t.iter().all(|v| v[1] > c.1 + r);
            if near {
                continue;
            }
            for q in samples(t) {
                let d = (q[0] - c.0).hypot(q[1] - c.1);
                if let Some(h) = cutter.height_at(d) {
                    let into = (top + q[2]) - (z + h);
                    if into > ARC_APPROX_TOL_MM && worst.is_none_or(|w| into > w.0) {
                        worst = Some((into, p));
                    }
                }
            }
        }
    }
    worst
}

/// Rings under the tool the ramp check tests, and points on each.
const RAMP_RINGS: usize = 4;
const RAMP_RING_POINTS: usize = 16;

/// The path check for a rim's ramp (spec §16): at sampled tool positions of the cutting
/// moves, points on rings under the tool against the ramp's exact surface. Returns the
/// worst cut into it and where, if any.
pub fn verify_ramp(moves: &[Move], ramp: &Ramp, cutter: Cutter) -> Option<(f64, Point)> {
    let positions = positions(moves);
    let stride = positions.len().div_ceil(VERIFY_POSITIONS).max(1);
    let r = cutter.radius();
    let mut worst: Option<(f64, Point)> = None;
    for &(p, z) in positions.iter().step_by(stride) {
        for ring in 0..=RAMP_RINGS {
            let rho = r * ring as f64 / RAMP_RINGS as f64;
            let Some(h) = cutter.height_at(rho) else {
                continue;
            };
            for k in 0..RAMP_RING_POINTS {
                let a = std::f64::consts::TAU * k as f64 / RAMP_RING_POINTS as f64;
                let q = (p.0 + rho * a.cos(), p.1 + rho * a.sin());
                let into = ramp.surface(q) - (z + h);
                if into > ARC_APPROX_TOL_MM && worst.is_none_or(|w| into > w.0) {
                    worst = Some((into, p));
                }
            }
        }
    }
    worst
}

/// The tool's positions along the cutting moves: every end, and a point between ends far
/// apart.
fn positions(moves: &[Move]) -> Vec<(Point, f64)> {
    let mut out = Vec::new();
    let (mut x, mut y, mut z) = (0.0, 0.0, 0.0);
    for m in moves {
        let (nx, ny, nz) = (m.x.unwrap_or(x), m.y.unwrap_or(y), m.z.unwrap_or(z));
        if m.kind == MoveKind::Feed {
            let length = (nx - x).hypot(ny - y);
            let n = (length / FIRST_SPACING_MM).ceil().max(1.0) as usize;
            for k in 1..=n {
                let t = k as f64 / n as f64;
                out.push(((x + (nx - x) * t, y + (ny - y) * t), z + (nz - z) * t));
            }
        }
        (x, y, z) = (nx, ny, nz);
    }
    out
}

/// A triangle's corners, the middles of its edges and its centre.
fn samples(t: &[[f64; 3]; 3]) -> [[f64; 3]; 7] {
    let mid = |a: [f64; 3], b: [f64; 3]| {
        [
            (a[0] + b[0]) / 2.0,
            (a[1] + b[1]) / 2.0,
            (a[2] + b[2]) / 2.0,
        ]
    };
    let centre = [
        (t[0][0] + t[1][0] + t[2][0]) / 3.0,
        (t[0][1] + t[1][1] + t[2][1]) / 3.0,
        (t[0][2] + t[1][2] + t[2][2]) / 3.0,
    ];
    [
        t[0],
        t[1],
        t[2],
        mid(t[0], t[1]),
        mid(t[1], t[2]),
        mid(t[2], t[0]),
        centre,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A dome: a hemisphere of radius 20 at (0, 0), its top at 0, as triangles.
    fn dome() -> Mesh {
        let (n, m) = (24, 48);
        let at = |i: usize, j: usize| {
            let polar = std::f64::consts::FRAC_PI_2 * i as f64 / n as f64;
            let az = std::f64::consts::TAU * j as f64 / m as f64;
            [
                20.0 * polar.sin() * az.cos(),
                20.0 * polar.sin() * az.sin(),
                20.0 * polar.cos() - 20.0,
            ]
        };
        let mut triangles = Vec::new();
        for i in 0..n {
            for j in 0..m {
                let (a, b, c, d) = (at(i, j), at(i, j + 1), at(i + 1, j), at(i + 1, j + 1));
                if i > 0 {
                    triangles.push([a, c, b]);
                }
                triangles.push([b, c, d]);
            }
        }
        Mesh { triangles }
    }

    fn square(half: f64) -> Vec<Vec<Point>> {
        vec![vec![
            (-half, -half),
            (half, -half),
            (half, half),
            (-half, half),
        ]]
    }

    #[test]
    fn a_dome_is_carved_without_cutting_into_it() {
        let mesh = dome();
        let index = DropMesh::new(&mesh, 6.0);
        let area = Area::Loops(square(22.0));
        for cutter in [
            Cutter::Ball { radius: 1.5 },
            Cutter::Flat { radius: 3.0 },
            Cutter::TaperedBall {
                radius: 3.175,
                ball: 0.5,
                half_angle: 10f64.to_radians(),
            },
        ] {
            let cut = ReliefCut {
                mesh: &index,
                cutter,
                top: 0.0,
                floor: -20.0,
                area: &area,
                ramp: None,
                step: 1.5,
                angle: 30f64.to_radians(),
                zigzag: true,
                safe_z: 5.0,
                feed: 1500.0,
                plunge_feed: 500.0,
                shift: (0.0, 0.0),
            };
            let (moves, _) = relief_moves(&cut, (0.0, 0.0));
            assert!(moves.len() > 50, "{cutter:?}: {}", moves.len());
            assert_eq!(
                verify(&moves, &mesh, cutter, 0.0, (0.0, 0.0)),
                None,
                "{cutter:?}"
            );
            // The top of the dome is reached: some point near the middle stands at Z 0.
            let highest = moves
                .iter()
                .filter(|m| m.kind == MoveKind::Feed)
                .filter_map(|m| m.z)
                .fold(f64::NEG_INFINITY, f64::max);
            assert!(highest > -0.5, "{cutter:?}: {highest}");
        }
    }

    /// A dome with a rim (spec §16): a flat `foot` wide round its outline (the dome's circle
    /// as a 48-gon), a 45° ramp up to the stock top.
    fn rimmed(foot: f64, floor: f64) -> (Area, Ramp) {
        let circle: Vec<Point> = (0..48)
            .map(|k| {
                let a = std::f64::consts::TAU * k as f64 / 48.0;
                (20.0 * a.cos(), 20.0 * a.sin())
            })
            .collect();
        let ramp = Ramp {
            loops: vec![circle.clone()],
            foot,
            tan: 1.0,
            floor,
            top: 0.0,
        };
        let area = Area::Grown {
            loops: vec![circle],
            reach: ramp.reach(),
        };
        (area, ramp)
    }

    #[test]
    fn a_rim_is_cut_flat_then_ramped_up_to_the_top() {
        let mesh = dome();
        let ball = Cutter::Ball { radius: 1.0 };
        let index = DropMesh::new(&mesh, 2.0);
        let (area, ramp) = rimmed(3.0, -20.0);
        let cut = ReliefCut {
            mesh: &index,
            cutter: ball,
            top: 0.0,
            floor: -20.0,
            area: &area,
            ramp: Some(&ramp),
            step: 0.8,
            angle: 0.0,
            zigzag: true,
            safe_z: 5.0,
            feed: 1500.0,
            plunge_feed: 500.0,
            shift: (0.0, 0.0),
        };
        let (moves, _) = relief_moves(&cut, (0.0, 0.0));
        assert_eq!(verify(&moves, &mesh, ball, 0.0, (0.0, 0.0)), None);
        assert_eq!(verify_ramp(&moves, &ramp, ball), None);
        // Every plunge goes down to the stock top: lines start where the ramp meets it.
        let plunges: Vec<f64> = moves
            .windows(2)
            .filter(|w| w[0].kind == MoveKind::Rapid && w[1].kind == MoveKind::Feed)
            .filter(|w| w[1].x.is_none() && w[1].y.is_none())
            .filter_map(|w| w[1].z)
            .collect();
        assert!(!plunges.is_empty());
        assert!(plunges.iter().all(|z| *z > -0.01), "{plunges:?}");
        // The flat at the floor: on the X axis between the dome (r 20) and the ramp's foot
        // (r 23), within the ball's radius of it, the tool stands at the floor.
        let feeds: Vec<(f64, f64, f64)> = positions(&moves)
            .into_iter()
            .map(|((x, y), z)| (x, y, z))
            .collect();
        let on_flat = feeds
            .iter()
            .filter(|(x, y, _)| y.abs() < 0.5 && (*x - 21.5).abs() < 0.4)
            .collect::<Vec<_>>();
        assert!(!on_flat.is_empty());
        assert!(
            on_flat.iter().all(|(_, _, z)| (z + 20.0).abs() < 0.02),
            "{on_flat:?}"
        );
        // Halfway up the ramp (10 mm out from its foot), a ball 1/cos45° − 1 above it.
        let up = feeds
            .iter()
            .filter(|(x, y, _)| y.abs() < 0.5 && (*x - 33.0).abs() < 0.2)
            .collect::<Vec<_>>();
        assert!(!up.is_empty());
        let rest = 2f64.sqrt() - 1.0;
        for (x, y, z) in up {
            let expected = ramp.surface((*x, *y)) + rest;
            assert!((z - expected).abs() < 0.02, "{x} {y} {z} {expected}");
        }
    }

    #[test]
    fn a_rim_is_roughed_keeping_the_allowance_off_its_ramp() {
        let mesh = dome();
        let allowance = 0.5;
        let flat = Cutter::Flat { radius: 3.0 };
        let grown = flat.grown(allowance).unwrap();
        let index = DropMesh::new(&mesh, 2.0 * grown.radius());
        let (area, ramp) = rimmed(10.0, -20.0 + allowance);
        let ramp = Ramp {
            top: allowance,
            ..ramp
        };
        let cut = ReliefCut {
            mesh: &index,
            cutter: grown,
            top: allowance,
            floor: -20.0 + allowance,
            area: &area.shrunk(2.0).unwrap(),
            ramp: Some(&ramp),
            step: 2.4,
            angle: 0.0,
            zigzag: true,
            safe_z: 5.0,
            feed: 1500.0,
            plunge_feed: 500.0,
            shift: (0.0, 0.0),
        };
        let (moves, _) = rough_moves(&cut, 0.0, 6.0, (0.0, 0.0));
        assert_eq!(verify(&moves, &mesh, grown, allowance, (0.0, 0.0)), None);
        assert_eq!(verify_ramp(&moves, &ramp, grown), None);
        let lowest = moves
            .iter()
            .filter(|m| m.kind == MoveKind::Feed)
            .filter_map(|m| m.z)
            .fold(f64::INFINITY, f64::min);
        assert!((lowest + 19.5).abs() < 1e-6, "{lowest}");
    }

    #[test]
    fn the_check_finds_a_cut_into_the_model() {
        let mesh = dome();
        let into = vec![
            Move::rapid_z(5.0),
            Move::rapid_xy((0.0, 0.0)),
            Move::feed_z(-2.0, 500.0),
            Move::feed_xy((5.0, 0.0), 1500.0),
        ];
        let found = verify(&into, &mesh, Cutter::Ball { radius: 1.5 }, 0.0, (0.0, 0.0));
        assert!(found.is_some_and(|(depth, _)| depth > 1.0), "{found:?}");
    }

    #[test]
    fn a_dome_is_roughed_keeping_the_allowance() {
        let mesh = dome();
        let area = Area::Loops(square(22.0));
        let allowance = 0.5;
        for tool in [Cutter::Flat { radius: 3.0 }, Cutter::Ball { radius: 3.0 }] {
            let grown = tool.grown(allowance).unwrap();
            let index = DropMesh::new(&mesh, 2.0 * grown.radius());
            let cut = ReliefCut {
                mesh: &index,
                cutter: grown,
                top: allowance,
                floor: -20.0 + allowance,
                area: &area,
                ramp: None,
                step: 2.4,
                angle: 0.0,
                zigzag: true,
                safe_z: 5.0,
                feed: 1500.0,
                plunge_feed: 500.0,
                shift: (0.0, 0.0),
            };
            let (moves, _) = rough_moves(&cut, 0.0, 6.0, (0.0, 0.0));
            // The allowance is kept: the grown tool, raised by it, stays off the model.
            assert_eq!(
                verify(&moves, &mesh, grown, allowance, (0.0, 0.0)),
                None,
                "{tool:?}"
            );
            let zs: Vec<f64> = moves
                .iter()
                .filter(|m| m.kind == MoveKind::Feed)
                .filter_map(|m| m.z)
                .collect();
            let lowest = zs.iter().copied().fold(f64::INFINITY, f64::min);
            // Down to the floor plus the allowance round the dome, never below it.
            assert!((lowest - (-19.5)).abs() < 1e-6, "{tool:?}: {lowest}");
            // Each level is cut: some moves at -6, -12 and -18 (the flat floors round the
            // dome's sides).
            for level in [-6.0, -12.0, -18.0] {
                assert!(
                    zs.iter().any(|z| (z - level).abs() < 1e-9),
                    "{tool:?}: no moves at {level}"
                );
            }
            // Down level by level: the tool lifts between them, fewer times than lines.
            let lifts = moves
                .iter()
                .filter(|m| m.kind == MoveKind::Rapid && m.z == Some(5.0))
                .count();
            assert!(lifts < 40, "{tool:?}: {lifts} lifts");
        }
    }

    #[test]
    fn levels_go_one_step_down_apart_to_the_bottom() {
        assert_eq!(levels(0.0, -10.0, 4.0), vec![-4.0, -8.0, -10.0]);
        assert_eq!(levels(0.0, -8.0, 4.0), vec![-4.0, -8.0]);
        assert_eq!(levels(2.0, -1.0, 5.0), vec![-1.0]);
        assert!(levels(0.0, 0.0, 4.0).is_empty());
        assert!(levels(0.0, 1.0, 4.0).is_empty());
    }

    #[test]
    fn a_link_must_stay_inside_the_area() {
        // An L: the notch's corner at (5, 5).
        let l = vec![vec![
            (0.0, 0.0),
            (10.0, 0.0),
            (10.0, 5.0),
            (5.0, 5.0),
            (5.0, 10.0),
            (0.0, 10.0),
        ]];
        assert!(inside_loops_link(&l, (1.0, 1.0), (9.0, 4.0)));
        // Along an edge's ends.
        assert!(inside_loops_link(&l, (10.0, 1.0), (10.0, 4.0)));
        // Across the notch.
        assert!(!inside_loops_link(&l, (9.0, 4.0), (4.0, 9.0)));
        // Outside, between two ends on the boundary.
        assert!(!inside_loops_link(&l, (10.0, 5.0), (5.0, 10.0)));
    }

    #[test]
    fn spans_follow_the_even_odd_rule() {
        let loops = vec![
            vec![(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)],
            vec![(3.0, 3.0), (7.0, 3.0), (7.0, 7.0), (3.0, 7.0)],
        ];
        assert_eq!(loop_spans(&loops, 5.0), vec![(0.0, 3.0), (7.0, 10.0)]);
        assert_eq!(loop_spans(&loops, 1.0), vec![(0.0, 10.0)]);
        assert!(loop_spans(&loops, 11.0).is_empty());
    }
}
