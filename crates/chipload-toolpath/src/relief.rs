// SPDX-License-Identifier: GPL-3.0-or-later
//! Reliefs (docs/specs/stl-relief.md §3, §5, §7): a model placed on the board, carved in
//! parallel lines across an area, each point as low as the drop cutter lets the tool go,
//! and the finished path checked against the model by a test of its own.

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use chipload_core::Point;
use chipload_core::cutter::Cutter;
use chipload_core::tolerances::{ARC_APPROX_TOL_MM, CHAIN_TOL_MM};
use chipload_i18n::{Message, Value};
use chipload_job::{Model, ModelUp};
use chipload_mesh::{Mesh, Up};

use crate::drop::DropMesh;
use crate::model::{Move, MoveKind};

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

/// The model's mesh read from the project, turned so its up axis points up, in mm as its
/// unit reads.
pub fn turned_mesh(model: &Model) -> Result<Mesh, ReliefError> {
    let bytes = STANDARD.decode(&model.data).map_err(|e| {
        refused(
            Message::new("toolpath-model-unreadable")
                .arg("id", model.id)
                .arg("error", Value::Text(e.to_string())),
        )
    })?;
    let read = std::panic::catch_unwind(|| chipload_mesh::read_stl(&bytes));
    let (mesh, _) = match read {
        Ok(Ok(read)) => read,
        Ok(Err(e)) => return Err(refused(e.message())),
        Err(_) => {
            return Err(refused(
                Message::new("toolpath-model-unreadable")
                    .arg("id", model.id)
                    .arg("error", Value::Text(String::new())),
            ));
        }
    };
    let k = match model.units {
        chipload_job::Units::Mm => 1.0,
        chipload_job::Units::Inch => 25.4,
    };
    Ok(mesh
        .turned(up(model.up))
        .mapped(|v| [v[0] * k, v[1] * k, v[2] * k]))
}

/// Where `model`'s turned mesh lies on the board (spec §3): its depth in proportion to the
/// width when the model says so.
pub fn placement(model: &Model, turned: &Mesh) -> Result<Placement, ReliefError> {
    let (lo, hi) = turned
        .bounds()
        .ok_or_else(|| refused(Message::new("mesh-error-empty")))?;
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

/// What a relief's finishing lines are made from.
pub struct ReliefCut<'a> {
    /// The model on the board, its top at Z `top`.
    pub mesh: &'a DropMesh,
    pub cutter: Cutter,
    /// The model's top in program Z, and the lowest the tool goes (its lowest point).
    pub top: f64,
    pub floor: f64,
    /// The area the tool's centre stays in: closed loops, holes by the even-odd rule.
    pub area: &'a [Vec<Point>],
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
        drop.max(self.floor)
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
            // The next line's start is near: along the surface to it.
            Some(from) if near(from.0, first.0, cut.step) => {
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
fn runs(cut: &ReliefCut) -> Vec<Run> {
    let (sin, cos) = cut.angle.sin_cos();
    // (u, v): u along the lines, v across them.
    let to_uv = |p: Point| (p.0 * cos + p.1 * sin, -p.0 * sin + p.1 * cos);
    let from_uv = |u: f64, v: f64| (u * cos - v * sin, u * sin + v * cos);
    let loops: Vec<Vec<Point>> = cut
        .area
        .iter()
        .map(|l| l.iter().map(|p| to_uv(*p)).collect())
        .collect();
    let (v0, v1) = loops
        .iter()
        .flatten()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), p| {
            (lo.min(p.1), hi.max(p.1))
        });
    if v0 > v1 {
        return Vec::new();
    }
    let count = ((v1 - v0) / cut.step).ceil().max(1.0) as usize;
    let gap = (v1 - v0) / count as f64;
    let mut out = Vec::new();
    for k in 0..count {
        let v = v0 + (k as f64 + 0.5) * gap;
        let mut spans = spans(&loops, v);
        let backwards = cut.zigzag && k % 2 == 1;
        if backwards {
            spans.reverse();
        }
        for (a, b) in spans {
            let (from, to) = if backwards { (b, a) } else { (a, b) };
            out.push(follow(cut, from_uv(from, v), from_uv(to, v)));
        }
    }
    out
}

/// Where the line `v` lies inside the loops (even-odd), as (u0, u1) from low to high u.
fn spans(loops: &[Vec<Point>], v: f64) -> Vec<(f64, f64)> {
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
    simplified(points)
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
        let area = square(22.0);
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
    fn spans_follow_the_even_odd_rule() {
        let loops = vec![
            vec![(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)],
            vec![(3.0, 3.0), (7.0, 3.0), (7.0, 7.0), (3.0, 7.0)],
        ];
        assert_eq!(spans(&loops, 5.0), vec![(0.0, 3.0), (7.0, 10.0)]);
        assert_eq!(spans(&loops, 1.0), vec![(0.0, 10.0)]);
        assert!(spans(&loops, 11.0).is_empty());
    }
}
