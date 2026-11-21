// SPDX-License-Identifier: GPL-3.0-or-later
//! A fillet or chamfer between two drawn open polylines (docs/specs/fillet-between-shapes.md
//! §5): the round picked by where each is clicked, both trimmed or extended to it, then
//! joined as after a Trim; between two straight sides the round is kept as the corner's
//! radius. The polylines are worked on as their points, so their other corners keep their
//! radii.

use chipload_core::Point;
use chipload_core::corners::round::{Curve, crossings, most_that_fits, picked_round};
use chipload_core::math;
use chipload_core::primitives::{Arc, Line, Primitive, distance};
use chipload_core::tolerances::{ANGLE_TOL_RAD, CHAIN_TOL_MM};
use chipload_i18n::{Digits, Message, Value};
use chipload_job::{Geometry, Job, Shape};
use std::f64::consts::PI;

use crate::PipelineError;
use crate::shapes::reshape;

use super::trim::{join_group_of, joined_where_ends_meet};
use super::{Edited, add_all, drawn, refused};

/// What joins the two shapes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Between {
    /// A round of this radius; 0: the two meet in a sharp corner.
    Fillet(f64),
    /// A straight cut this far back along each line from where they meet.
    Chamfer(f64),
}

/// An open polyline as its points, bulges and radii (one per point, all listed).
#[derive(Debug, Clone)]
struct Open {
    points: Vec<Point>,
    bulges: Vec<f64>,
    radii: Vec<f64>,
}

/// Why the two can't be joined.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Why {
    /// No round of the size touches the parts kept (too large, or too far apart).
    NoRound,
    /// The two lines are parallel.
    Parallel,
    /// Shape `0` would need a middle side longer.
    Middle(u64),
    /// Shape `0`'s arc would grow by a whole turn.
    Turn(u64),
    /// A chamfer and shape `0`'s picked side is an arc.
    ChamferArc(u64),
}

/// Which way from the tangent point the shape is kept.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Keep {
    ToEnd,
    ToStart,
}

/// The two shapes as they become, and the round (or chamfer) between them, if any.
struct Joint {
    first: Open,
    second: Open,
    piece: Option<Geometry>,
    /// Where the two lines meet, when the round is between two lines: its corner.
    corner: Option<(Point, Point, Point)>,
}

/// Join drawn open polylines `a` and `b` (lines and arcs included) with a fillet or chamfer,
/// the parts kept picked by points `at_a` and `at_b` near them. Both are trimmed or
/// extended to it; the round is a new shape on `a`'s layer, and the three join as after a
/// Trim. One edit, refused with the reason (and the most that fits, when there is one).
pub fn fillet_between(
    job: &Job,
    (a, at_a): (u64, Point),
    (b, at_b): (u64, Point),
    how: Between,
) -> Result<Edited, PipelineError> {
    if a == b {
        return Err(refused(Message::new("pipeline-between-same")));
    }
    let first = drawn(job, a)?.clone();
    let second = drawn(job, b)?;
    let (Some(one), Some(two)) = (open_polyline(&first), open_polyline(second)) else {
        let id = if open_polyline(&first).is_none() {
            a
        } else {
            b
        };
        return Err(refused(Message::new("pipeline-between-kind").arg("id", id)));
    };
    let size = match how {
        Between::Fillet(r) | Between::Chamfer(r) => r,
    };
    if !(size.is_finite() && size >= 0.0) {
        return Err(refused(Message::new("pipeline-corner-size")));
    }
    let attempt = |how: Between| -> Result<Edited, Refusal> {
        let joint = joint((a, &one, at_a), (b, &two, at_b), how).map_err(Refusal::Why)?;
        build(job, &first, b, joint, how).map_err(Refusal::Job)
    };
    match attempt(how) {
        Ok(edited) => Ok(edited),
        Err(Refusal::Job(e)) => Err(e),
        Err(Refusal::Why(why)) => Err(refusal(why, (a, b), how, |r| {
            attempt(Between::Fillet(r)).is_ok()
        })),
    }
}

/// A refusal: a reason of this module's, or the project's own.
enum Refusal {
    Why(Why),
    Job(PipelineError),
}

fn refusal(
    why: Why,
    (a, b): (u64, u64),
    how: Between,
    fits: impl Fn(f64) -> bool,
) -> PipelineError {
    let message = match (why, how) {
        (Why::NoRound, Between::Fillet(r)) => {
            let most = if r > 0.0 {
                most_that_fits(r, fits)
            } else {
                0.0
            };
            let base = if most > 0.0 && most < r {
                Message::new("pipeline-between-too-large")
                    .arg("most", Value::Length(most, Digits::Fixed(3)))
            } else {
                Message::new("pipeline-between-none")
            };
            base.arg("radius", Value::Length(r, Digits::Trim(3)))
                .arg("a", a)
                .arg("b", b)
        }
        (Why::NoRound, Between::Chamfer(d)) => Message::new("pipeline-between-chamfer-large")
            .arg("distance", Value::Length(d, Digits::Trim(3)))
            .arg("a", a)
            .arg("b", b),
        (Why::Parallel, _) => Message::new("pipeline-between-parallel")
            .arg("a", a)
            .arg("b", b),
        (Why::Middle(id), _) => Message::new("pipeline-between-middle").arg("id", id),
        (Why::Turn(id), _) => Message::new("pipeline-between-turn").arg("id", id),
        (Why::ChamferArc(id), _) => Message::new("pipeline-between-chamfer-arc").arg("id", id),
    };
    refused(message)
}

/// A drawn open polyline (a line or an arc too) as its points; None for any other shape.
fn open_polyline(shape: &Shape) -> Option<Open> {
    match &shape.geometry {
        Geometry::Polyline {
            closed: false,
            points,
            bulges,
            radii,
        } if points.len() >= 2 => Some(Open {
            points: points.iter().map(|p| (p[0], p[1])).collect(),
            bulges: (0..points.len() - 1)
                .map(|i| bulges.get(i).copied().unwrap_or(0.0))
                .collect(),
            radii: (0..points.len())
                .map(|i| radii.get(i).copied().unwrap_or(0.0))
                .collect(),
        }),
        _ => None,
    }
}

impl Open {
    /// Side `i` as drawn (before its corners are rounded); None without length.
    fn side(&self, i: usize) -> Option<Primitive> {
        let (a, b) = (*self.points.get(i)?, *self.points.get(i + 1)?);
        if distance(a, b) < CHAIN_TOL_MM {
            return None;
        }
        let bulge = self.bulges[i];
        if bulge == 0.0 {
            Some(Primitive::Line(Line { start: a, end: b }))
        } else {
            Arc::from_bulge(a, b, bulge).map(Primitive::Arc)
        }
    }

    /// The side nearest `p`: its number, and the side.
    fn picked(&self, p: Point) -> Option<(usize, Primitive)> {
        (0..self.points.len() - 1)
            .filter_map(|i| self.side(i).map(|s| (i, s)))
            .min_by(|x, y| gap(&x.1, p).total_cmp(&gap(&y.1, p)))
    }

    fn geometry(&self) -> Geometry {
        let radii = if self.radii.iter().all(|&r| r == 0.0) {
            Vec::new()
        } else {
            self.radii.clone()
        };
        Geometry::Polyline {
            closed: false,
            points: self.points.iter().map(|&(x, y)| [x, y]).collect(),
            bulges: self.bulges.clone(),
            radii,
        }
    }
}

/// How far `p` is from side `s`.
fn gap(s: &Primitive, p: Point) -> f64 {
    let along = position(s, foot(s, p));
    if (0.0..=s.length()).contains(&along) {
        distance(p, foot(s, p))
    } else {
        distance(p, s.start()).min(distance(p, s.end()))
    }
}

/// The point of side `s`'s line or circle nearest `p`.
fn foot(s: &Primitive, p: Point) -> Point {
    Curve::of(s).foot(p).unwrap_or_else(|| s.start())
}

/// Where point `p` (on side `s`'s line or circle) lies along it from its start: negative
/// before it, more than its length past its end. On a circle, outside the arc counts on
/// the nearer end's side.
fn position(s: &Primitive, p: Point) -> f64 {
    match s {
        Primitive::Line(l) => {
            let len = s.length();
            ((p.0 - l.start.0) * (l.end.0 - l.start.0) + (p.1 - l.start.1) * (l.end.1 - l.start.1))
                / len
        }
        Primitive::Arc(arc) => {
            let angle = math::atan2(p.1 - arc.center.1, p.0 - arc.center.0);
            let u = ((angle - arc.start_angle) * arc.sweep.signum()).rem_euclid(2.0 * PI);
            let sweep = arc.sweep.abs();
            if u <= sweep || u - sweep <= 2.0 * PI - u {
                u * arc.radius
            } else {
                -(2.0 * PI - u) * arc.radius
            }
        }
    }
}

/// The direction side `s` runs at point `p` on it (from its start to its end).
fn heading(s: &Primitive, p: Point) -> Point {
    match s {
        Primitive::Line(l) => {
            let len = s.length();
            ((l.end.0 - l.start.0) / len, (l.end.1 - l.start.1) / len)
        }
        Primitive::Arc(arc) => {
            let angle = math::atan2(p.1 - arc.center.1, p.0 - arc.center.0);
            let dir = arc.sweep.signum();
            (-dir * math::sin(angle), dir * math::cos(angle))
        }
    }
}

fn dot(a: Point, b: Point) -> f64 {
    a.0 * b.0 + a.1 * b.1
}

/// Which way to keep side `s` from `t`, the shape running on from `t` in direction `d`.
fn keep_along(s: &Primitive, t: Point, d: Point) -> Keep {
    if dot(d, heading(s, t)) > 0.0 {
        Keep::ToEnd
    } else {
        Keep::ToStart
    }
}

/// The round (or chamfer) between the two and both shapes cut or grown to it.
fn joint(
    (a, one, at_a): (u64, &Open, Point),
    (b, two, at_b): (u64, &Open, Point),
    how: Between,
) -> Result<Joint, Why> {
    let (i, s1) = one.picked(at_a).ok_or(Why::NoRound)?;
    let (j, s2) = two.picked(at_b).ok_or(Why::NoRound)?;
    let (k1, k2) = (foot(&s1, at_a), foot(&s2, at_b));
    let (c1, c2) = (Curve::of(&s1), Curve::of(&s2));
    let lines = matches!((s1, s2), (Primitive::Line(_), Primitive::Line(_)));
    let parallel = lines && crossings(&c1, &c2).is_empty();
    if parallel {
        return Err(Why::Parallel);
    }
    let (t1, t2, keep1, keep2, piece) = match how {
        Between::Chamfer(d) if d > 0.0 => {
            for (id, s) in [(a, &s1), (b, &s2)] {
                if matches!(s, Primitive::Arc(_)) {
                    return Err(Why::ChamferArc(id));
                }
            }
            let corner = crossings(&c1, &c2)[0];
            // Back along each line from the corner, towards the click.
            let back = |s: &Primitive, k: Point| {
                let h = heading(s, corner);
                let sign = if dot((k.0 - corner.0, k.1 - corner.1), h) >= 0.0 {
                    1.0
                } else {
                    -1.0
                };
                (corner.0 + h.0 * sign * d, corner.1 + h.1 * sign * d)
            };
            let (t1, t2) = (back(&s1, k1), back(&s2, k2));
            let away = |t: Point| (t.0 - corner.0, t.1 - corner.1);
            let piece = Geometry::Polyline {
                closed: false,
                points: vec![[t1.0, t1.1], [t2.0, t2.1]],
                bulges: vec![0.0],
                radii: Vec::new(),
            };
            (
                t1,
                t2,
                keep_along(&s1, t1, away(t1)),
                keep_along(&s2, t2, away(t2)),
                Some(piece),
            )
        }
        Between::Chamfer(_) | Between::Fillet(_) => {
            let r = match how {
                Between::Fillet(r) => r,
                Between::Chamfer(_) => 0.0,
            };
            let round = picked_round(&c1, k1, &c2, k2, r).ok_or(Why::NoRound)?;
            let (t1, t2) = (round.first, round.second);
            match round.arc() {
                Some(arc) => {
                    // The shapes run on from the round tangentially: on the first, the way
                    // the round arrives from the second; on the second, the way it leaves.
                    let along = |angle: f64| {
                        let dir = arc.sweep.signum();
                        (-dir * math::sin(angle), dir * math::cos(angle))
                    };
                    let into_first = along(arc.start_angle);
                    let d1 = (-into_first.0, -into_first.1);
                    let d2 = along(arc.start_angle + arc.sweep);
                    (
                        t1,
                        t2,
                        keep_along(&s1, t1, d1),
                        keep_along(&s2, t2, d2),
                        Some(Geometry::Polyline {
                            closed: false,
                            points: vec![[t1.0, t1.1], [t2.0, t2.1]],
                            bulges: vec![arc.bulge()],
                            radii: Vec::new(),
                        }),
                    )
                }
                None => {
                    // They meet: each kept towards its click.
                    let d1 = (k1.0 - t1.0, k1.1 - t1.1);
                    let d2 = (k2.0 - t2.0, k2.1 - t2.1);
                    if math::hypot(d1.0, d1.1) < CHAIN_TOL_MM
                        || math::hypot(d2.0, d2.1) < CHAIN_TOL_MM
                    {
                        return Err(Why::NoRound);
                    }
                    (
                        t1,
                        t2,
                        keep_along(&s1, t1, d1),
                        keep_along(&s2, t2, d2),
                        None,
                    )
                }
            }
        }
    };
    let first = cut_at(one, i, &s1, t1, keep1).map_err(|w| w.of(a))?;
    let second = cut_at(two, j, &s2, t2, keep2).map_err(|w| w.of(b))?;
    let corner = match (how, lines) {
        (Between::Fillet(r), true) if r > 0.0 => crossings(&c1, &c2).first().map(|&c| (c, t1, t2)),
        _ => None,
    };
    Ok(Joint {
        first,
        second,
        piece,
        corner,
    })
}

/// Why a cut failed, before the shape's id is known.
enum CutWhy {
    TooLarge,
    Middle,
    Turn,
}

impl CutWhy {
    fn of(self, id: u64) -> Why {
        match self {
            CutWhy::TooLarge => Why::NoRound,
            CutWhy::Middle => Why::Middle(id),
            CutWhy::Turn => Why::Turn(id),
        }
    }
}

/// `o` cut (or grown) at `t`, a point on side `i`'s line or circle, keeping the part
/// towards its end or start. Only an end side grows; an arc by less than a whole turn.
fn cut_at(o: &Open, i: usize, s: &Primitive, t: Point, keep: Keep) -> Result<Open, CutWhy> {
    let len = s.length();
    let at = position(s, t);
    let last = o.points.len() - 2;
    // The side's new bulge from `from` to `to` along it (lengths from its start).
    let bulge = |from: f64, to: f64| -> Result<f64, CutWhy> {
        match s {
            Primitive::Line(_) => Ok(0.0),
            Primitive::Arc(arc) => {
                let sweep = arc.sweep.signum() * (to - from) / arc.radius;
                if sweep.abs() >= 2.0 * PI - ANGLE_TOL_RAD {
                    Err(CutWhy::Turn)
                } else {
                    Ok(math::tan(sweep / 4.0))
                }
            }
        }
    };
    match keep {
        Keep::ToEnd => {
            if at < -CHAIN_TOL_MM && i != 0 {
                return Err(CutWhy::Middle);
            }
            if at > len + CHAIN_TOL_MM {
                return Err(CutWhy::TooLarge);
            }
            if at > len - CHAIN_TOL_MM {
                // At the side's end: the shape starts at its next point.
                if i == last {
                    return Err(CutWhy::TooLarge);
                }
                let mut radii = o.radii[i + 1..].to_vec();
                radii[0] = 0.0;
                return Ok(Open {
                    points: o.points[i + 1..].to_vec(),
                    bulges: o.bulges[i + 1..].to_vec(),
                    radii,
                });
            }
            let mut points = vec![t];
            points.extend_from_slice(&o.points[i + 1..]);
            let mut bulges = vec![bulge(at, len)?];
            bulges.extend_from_slice(&o.bulges[i + 1..]);
            let mut radii = vec![0.0];
            radii.extend_from_slice(&o.radii[i + 1..]);
            Ok(Open {
                points,
                bulges,
                radii,
            })
        }
        Keep::ToStart => {
            if at > len + CHAIN_TOL_MM && i != last {
                return Err(CutWhy::Middle);
            }
            if at < -CHAIN_TOL_MM {
                return Err(CutWhy::TooLarge);
            }
            if at < CHAIN_TOL_MM {
                // At the side's start: the shape ends at that point.
                if i == 0 {
                    return Err(CutWhy::TooLarge);
                }
                let mut radii = o.radii[..=i].to_vec();
                radii[i] = 0.0;
                return Ok(Open {
                    points: o.points[..=i].to_vec(),
                    bulges: o.bulges[..i].to_vec(),
                    radii,
                });
            }
            let mut points = o.points[..=i].to_vec();
            points.push(t);
            let mut bulges = o.bulges[..i].to_vec();
            bulges.push(bulge(0.0, at)?);
            let mut radii = o.radii[..=i].to_vec();
            radii.push(0.0);
            Ok(Open {
                points,
                bulges,
                radii,
            })
        }
    }
}

/// The project with both shapes changed, the piece added on the first's layer, and the
/// three joined as after a Trim; a round between two lines kept as the corner's radius.
fn build(
    job: &Job,
    first: &Shape,
    second: u64,
    joint: Joint,
    how: Between,
) -> Result<Edited, PipelineError> {
    let out = reshape(job, first.id, joint.first.geometry())?;
    let out = reshape(&out, second, joint.second.geometry())?;
    let mut new_ids = Vec::new();
    let mut notes = Vec::new();
    let mut out = match joint.piece {
        Some(piece) => add_all(out, first, vec![piece], &mut new_ids, &mut notes)?,
        None => out,
    };
    join_group_of(&mut out, first.id, &new_ids);
    let edited = joined_where_ends_meet(
        Edited {
            job: out,
            new_ids,
            notes,
            joined: None,
        },
        first,
    )?;
    match (joint.corner, edited.joined, how) {
        (Some(corner), Some(kept), Between::Fillet(r)) => keep_as_radius(edited, kept, corner, r),
        _ => Ok(edited),
    }
}

/// In joined shape `kept`, the round from `t1` to `t2` between two straight sides replaced
/// by the corner `c` with radius `r`, so it stays editable; as it is when it isn't there.
fn keep_as_radius(
    edited: Edited,
    kept: u64,
    (c, t1, t2): (Point, Point, Point),
    r: f64,
) -> Result<Edited, PipelineError> {
    let Some(Geometry::Polyline {
        closed,
        points,
        bulges,
        radii,
    }) = edited
        .job
        .drawing
        .shapes
        .iter()
        .find(|s| s.id == kept)
        .map(|s| s.geometry.clone())
    else {
        return Ok(edited);
    };
    let n = points.len();
    let segments = if closed { n } else { n.saturating_sub(1) };
    let near = |p: [f64; 2], q: Point| distance((p[0], p[1]), q) <= CHAIN_TOL_MM;
    let bulge = |k: usize| bulges.get(k).copied().unwrap_or(0.0);
    let found = (0..segments).find(|&k| {
        let (p, q) = (points[k], points[(k + 1) % n]);
        let ends = (near(p, t1) && near(q, t2)) || (near(p, t2) && near(q, t1));
        let before = if k > 0 {
            Some(k - 1)
        } else {
            closed.then(|| n - 1)
        };
        let after = if k + 1 < segments {
            Some(k + 1)
        } else {
            closed.then_some(0)
        };
        ends && bulge(k) != 0.0
            && before.is_some_and(|b| bulge(b) == 0.0)
            && after.is_some_and(|a| bulge(a) == 0.0)
    });
    let Some(mut k) = found else {
        return Ok(edited);
    };
    let mut radii: Vec<f64> = (0..n)
        .map(|i| radii.get(i).copied().unwrap_or(0.0))
        .collect();
    let mut points = points;
    let mut bulges: Vec<f64> = (0..segments).map(bulge).collect();
    // A closed one's closing side: turned a point on first, so the round isn't the last
    // side.
    if closed && k + 1 == n {
        points.rotate_left(1);
        radii.rotate_left(1);
        bulges.rotate_left(1);
        k -= 1;
    }
    // The round's two points become the one corner, its radius r.
    let next = k + 1;
    points[k] = [c.0, c.1];
    radii[k] = r;
    points.remove(next);
    radii.remove(next);
    bulges.remove(k);
    let out = reshape(
        &edited.job,
        kept,
        Geometry::Polyline {
            closed,
            points,
            bulges,
            radii,
        },
    )?;
    Ok(Edited { job: out, ..edited })
}
