// SPDX-License-Identifier: GPL-3.0-or-later
//! A setup cut as a puzzle (docs/specs/pieces.md): the seams of a grid, their joints
//! (puzzle knobs or rounded fingers) and the cells they bound. Each seam is one curve of
//! lines and arcs, shared by the cells on both sides, so neighbouring pieces fit.
//!
//! Every seam is built in its own frame: `u` along the seam, `v` across it, `v = 0` the
//! straight seam. A vertical seam at `x = at` maps (u, v) to (at + v, u), a horizontal one at
//! `y = at` to (u, at + v).

use std::f64::consts::{FRAC_PI_2, PI};

use chipload_core::Point;
use chipload_core::math;
use chipload_core::primitives::{Arc, Line, Primitive};
use chipload_core::tolerances::{ANGLE_TOL_RAD, CHAIN_TOL_MM};
use chipload_i18n::{Digits, Message, Value};
use chipload_job::Joint;

/// A knob's fillets, as a share of its head's radius: the arcs joining the head to the seam.
const KNOB_FILLET: f64 = 0.4;
/// A knob's neck at its narrowest, as a share of its head's radius on each side.
const KNOB_NECK: f64 = 0.6;
/// How far a joint stays from where two seams cross, in joint sizes (§4).
const FROM_CROSSING: f64 = 2.0;
/// How far a joint stays from where its seam leaves the material, in joint sizes (§4).
const FROM_EDGE: f64 = 1.0;
/// How many columns or rows the automatic grid tries at most.
pub const MAX_GRID: u32 = 50;

/// The joints along every seam of a setup.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct JointShape {
    pub joint: Joint,
    /// A knob's head diameter, a finger's length, mm.
    pub size: f64,
    /// Knob to knob, or a finger and a gap, mm.
    pub spacing: f64,
    /// The tightest radius the cut-out tool can cut on both pieces: its radius plus the
    /// clearance. The fingers' corners are rounded to it.
    pub round: f64,
}

impl JointShape {
    /// A knob's head radius, fillet radius, neck half width and its head centre's distance
    /// from the seam.
    fn knob(&self) -> Knob {
        let head = self.size / 2.0;
        let fillet = KNOB_FILLET * head;
        let reach = KNOB_NECK * head + fillet;
        let rise = fillet + ((head + fillet).powi(2) - reach.powi(2)).max(0.0).sqrt();
        Knob {
            head,
            fillet,
            reach,
            rise,
        }
    }

    /// How far a joint sticks out of its seam, mm: a knob's head top, a finger's length.
    pub fn protrusion(&self) -> f64 {
        match self.joint {
            Joint::Knobs => {
                let k = self.knob();
                k.rise + k.head
            }
            Joint::Fingers => self.size,
        }
    }

    /// Why the cut-out tool can't cut these joints, or None: an arc tighter than the tool's
    /// radius plus the clearance, a knob's neck narrower than the tool, a finger narrower
    /// than two of its corners (docs/specs/pieces.md §4).
    pub fn problem(&self) -> Option<Message> {
        let len = |v: f64| Value::Length(v, Digits::Fixed(2));
        let tight = match self.joint {
            Joint::Knobs => {
                let k = self.knob();
                // The fillets are the tightest arcs, and the neck the narrowest opening.
                k.fillet.min(k.reach - k.fillet) < self.round
            }
            Joint::Fingers => self.size < 2.0 * self.round || self.spacing / 2.0 < 2.0 * self.round,
        };
        tight.then(|| {
            Message::new("toolpath-pieces-joint-too-small")
                .arg("size", len(self.size))
                .arg("radius", len(self.round))
                .arg("least", len(self.least_size()))
        })
    }

    /// The smallest joint size the cut-out tool can cut, mm.
    pub fn least_size(&self) -> f64 {
        match self.joint {
            // The fillet (0.2 of the size) and the neck's half width (0.3) must both hold
            // the tool's round.
            Joint::Knobs => self.round / (KNOB_FILLET / 2.0).min(KNOB_NECK / 2.0),
            Joint::Fingers => 2.0 * self.round,
        }
    }
}

struct Knob {
    head: f64,
    fillet: f64,
    /// From the knob's centre to where a fillet meets the seam: also the head's radius.
    reach: f64,
    /// From the seam to the head's centre.
    rise: f64,
}

/// One joint on a seam, in the seam's frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Feature {
    /// A knob centred at `at`, its head on the `side` (+1 or -1) of the seam.
    Knob { at: f64, side: f64 },
    /// `teeth` fingers of `width` from `from`, sticking out to +v, with gaps of `width`.
    Fingers { from: f64, teeth: usize, width: f64 },
}

impl Feature {
    /// Where the feature starts and ends along its seam.
    fn span(&self, shape: &JointShape) -> (f64, f64) {
        match *self {
            Feature::Knob { at, .. } => {
                let reach = shape.knob().reach;
                (at - reach, at + reach)
            }
            Feature::Fingers { from, teeth, width } => (
                from - shape.round,
                from + (2 * teeth - 1) as f64 * width + shape.round,
            ),
        }
    }
}

/// A seam: a straight line of the grid with its joints.
#[derive(Debug, Clone, PartialEq)]
pub struct Seam {
    /// True: along Y at `x = at`; false: along X at `y = at`.
    pub vertical: bool,
    pub at: f64,
    pub features: Vec<Feature>,
}

impl Seam {
    /// A point of the seam's frame in drawing coordinates.
    fn xy(&self, (u, v): Point) -> Point {
        if self.vertical {
            (self.at + v, u)
        } else {
            (u, self.at + v)
        }
    }

    /// An arc of the seam's frame in drawing coordinates. A vertical seam's frame is
    /// mirrored (u and v swap), which turns angles about 45° and the sweep around.
    fn arc(&self, center: Point, radius: f64, start: f64, sweep: f64) -> Primitive {
        if self.vertical {
            Primitive::Arc(Arc {
                center: self.xy(center),
                radius,
                start_angle: FRAC_PI_2 - start,
                sweep: -sweep,
            })
        } else {
            Primitive::Arc(Arc {
                center: self.xy(center),
                radius,
                start_angle: start,
                sweep,
            })
        }
    }

    /// The seam from `from` to `to` along it (`from < to`), as lines and arcs in drawing
    /// coordinates. Joints lie wholly inside or wholly outside: they keep away from crossing
    /// seams, where portions end.
    pub fn portion(&self, from: f64, to: f64, shape: &JointShape) -> Vec<Primitive> {
        let mut out = Vec::new();
        let mut chain: Vec<Point> = vec![(from, 0.0)];
        for feature in &self.features {
            let (a, b) = feature.span(shape);
            if a < from || b > to {
                continue;
            }
            match *feature {
                Feature::Knob { at, side } => {
                    let k = shape.knob();
                    chain.push((at - k.reach, 0.0));
                    self.flush(&mut chain, shape.round, &mut out);
                    self.knob(at, side, &k, &mut out);
                    chain = vec![(at + k.reach, 0.0)];
                }
                Feature::Fingers {
                    from: z,
                    teeth,
                    width,
                } => {
                    for t in 0..teeth {
                        let x0 = z + 2.0 * t as f64 * width;
                        chain.extend([
                            (x0, 0.0),
                            (x0, shape.size),
                            (x0 + width, shape.size),
                            (x0 + width, 0.0),
                        ]);
                    }
                }
            }
        }
        chain.push((to, 0.0));
        self.flush(&mut chain, shape.round, &mut out);
        out
    }

    /// The lines through `chain`'s points, each corner rounded by `radius`.
    fn flush(&self, chain: &mut Vec<Point>, radius: f64, out: &mut Vec<Primitive>) {
        chain.dedup_by(|a, b| {
            (a.0 - b.0).abs() <= CHAIN_TOL_MM && (a.1 - b.1).abs() <= CHAIN_TOL_MM
        });
        let Some(&first) = chain.first() else {
            return;
        };
        let mut at = first;
        for i in 1..chain.len() {
            let p = chain[i];
            let corner = chain
                .get(i + 1)
                .and_then(|&next| fillet(at, p, next, radius));
            match corner {
                Some((t1, t2, center, start, sweep)) => {
                    self.line(at, t1, out);
                    out.push(self.arc(center, radius, start, sweep));
                    at = t2;
                }
                None => {
                    self.line(at, p, out);
                    at = p;
                }
            }
        }
        chain.clear();
    }

    fn line(&self, a: Point, b: Point, out: &mut Vec<Primitive>) {
        if (a.0 - b.0).abs() > CHAIN_TOL_MM || (a.1 - b.1).abs() > CHAIN_TOL_MM {
            out.push(Primitive::Line(Line {
                start: self.xy(a),
                end: self.xy(b),
            }));
        }
    }

    /// A knob's four arcs, from the seam before it to the seam after it.
    fn knob(&self, at: f64, side: f64, k: &Knob, out: &mut Vec<Primitive>) {
        let alpha = math::atan2(k.rise - k.fillet, k.reach);
        // On side +1: a fillet turning up into the neck, the head over the top, a fillet
        // turning back down onto the seam. Side -1 mirrors v.
        let arcs = [
            (
                (at - k.reach, k.fillet),
                k.fillet,
                -FRAC_PI_2,
                FRAC_PI_2 + alpha,
            ),
            ((at, k.rise), k.head, PI + alpha, -(PI + 2.0 * alpha)),
            (
                (at + k.reach, k.fillet),
                k.fillet,
                PI - alpha,
                FRAC_PI_2 + alpha,
            ),
        ];
        for ((cu, cv), radius, start, sweep) in arcs {
            if side > 0.0 {
                out.push(self.arc((cu, cv), radius, start, sweep));
            } else {
                out.push(self.arc((cu, -cv), radius, -start, -sweep));
            }
        }
    }
}

/// The corner at `p`, between the lines from `a` and to `b`, rounded by `radius`: where the
/// arc meets each line, its centre, start angle and sweep. None where the lines run on
/// straight.
fn fillet(a: Point, p: Point, b: Point, radius: f64) -> Option<(Point, Point, Point, f64, f64)> {
    let unit = |from: Point, to: Point| {
        let len = math::hypot(to.0 - from.0, to.1 - from.1);
        (len > CHAIN_TOL_MM).then(|| ((to.0 - from.0) / len, (to.1 - from.1) / len))
    };
    let d1 = unit(a, p)?;
    let d2 = unit(p, b)?;
    let cross = d1.0 * d2.1 - d1.1 * d2.0;
    let dot = d1.0 * d2.0 + d1.1 * d2.1;
    let turn = math::atan2(cross, dot);
    if turn.abs() <= ANGLE_TOL_RAD {
        return None;
    }
    let back = radius * math::tan(turn.abs() / 2.0);
    let t1 = (p.0 - d1.0 * back, p.1 - d1.1 * back);
    let t2 = (p.0 + d2.0 * back, p.1 + d2.1 * back);
    // The centre lies to the left of the first line for a left turn, else to its right.
    let side = turn.signum();
    let center = (t1.0 - d1.1 * radius * side, t1.1 + d1.0 * radius * side);
    let start = math::atan2(t1.1 - center.1, t1.0 - center.0);
    Some((t1, t2, center, start, turn))
}

/// Where a straight line across the drawing lies inside the material: the stretches of
/// `x = at` (vertical) or `y = at` along it, by the even-odd rule over `loops` (closed
/// loops of lines and arcs, outer outlines and holes). Arcs are followed within the arc
/// tolerance, which is plenty to place joints.
pub fn stretches(loops: &[Vec<Primitive>], vertical: bool, at: f64) -> Vec<(f64, f64)> {
    let mut crossings = Vec::new();
    for ring in loops {
        let points: Vec<Point> = ring
            .iter()
            .flat_map(|p| {
                let mut pts = p.points();
                pts.pop();
                pts
            })
            .collect();
        for i in 0..points.len() {
            let (p, q) = (points[i], points[(i + 1) % points.len()]);
            let (pa, qa, pu, qu) = if vertical {
                (p.0, q.0, p.1, q.1)
            } else {
                (p.1, q.1, p.0, q.0)
            };
            // Half-open, so a vertex on the line counts once.
            if (pa < at) != (qa < at) {
                let t = (at - pa) / (qa - pa);
                crossings.push(pu + t * (qu - pu));
            }
        }
    }
    crossings.sort_by(f64::total_cmp);
    crossings
        .as_chunks::<2>()
        .0
        .iter()
        .map(|&[a, b]| (a, b))
        .filter(|(a, b)| b - a > CHAIN_TOL_MM)
        .collect()
}

/// The joints along one seam: in every stretch through the material, cut at the seams that
/// cross it, away from the crossings and from the material's edge. Knobs alternate sides
/// along the seam (docs/specs/pieces.md §4).
pub fn features(stretches: &[(f64, f64)], crossings: &[f64], shape: &JointShape) -> Vec<Feature> {
    let mut out = Vec::new();
    let mut knobs = 0usize;
    for &(a, b) in stretches {
        // The stretch cut at the crossings inside it; each piece remembers which of its ends
        // is a crossing.
        let mut cuts: Vec<(f64, bool)> = vec![(a, false)];
        cuts.extend(
            crossings
                .iter()
                .filter(|&&c| c > a && c < b)
                .map(|&c| (c, true)),
        );
        cuts.push((b, false));
        for w in cuts.windows(2) {
            let ((s, s_cross), (e, e_cross)) = (w[0], w[1]);
            let margin = |cross: bool| shape.size * if cross { FROM_CROSSING } else { FROM_EDGE };
            let lo = s + margin(s_cross);
            let hi = e - margin(e_cross);
            match shape.joint {
                Joint::Knobs => {
                    if hi < lo {
                        continue;
                    }
                    let count = ((hi - lo) / shape.spacing).floor() as usize + 1;
                    let span = (count - 1) as f64 * shape.spacing;
                    let first = lo + (hi - lo - span) / 2.0;
                    for i in 0..count {
                        let side = if knobs.is_multiple_of(2) { 1.0 } else { -1.0 };
                        out.push(Feature::Knob {
                            at: first + i as f64 * shape.spacing,
                            side,
                        });
                        knobs += 1;
                    }
                }
                Joint::Fingers => {
                    let width = shape.spacing / 2.0;
                    // Teeth and the gaps between them, rounded corners inside the room.
                    let room = hi - lo - 2.0 * shape.round;
                    if room < width {
                        continue;
                    }
                    let teeth = ((room / width + 1.0) / 2.0).floor() as usize;
                    let span = (2 * teeth - 1) as f64 * width;
                    out.push(Feature::Fingers {
                        from: lo + (hi - lo - span) / 2.0,
                        teeth,
                        width,
                    });
                }
            }
        }
    }
    out
}

/// The inner seams' positions between `lo` and `hi` for `count` cells, each moved by its
/// shift (missing shifts: 0).
pub fn seam_positions(lo: f64, hi: f64, count: u32, shifts: &[f64]) -> Vec<f64> {
    let step = (hi - lo) / f64::from(count.max(1));
    (1..count)
        .map(|i| lo + f64::from(i) * step + shifts.get(i as usize - 1).copied().unwrap_or(0.0))
        .collect()
}

/// The seams of a grid over the material and the cells they bound.
#[derive(Debug, Clone, PartialEq)]
pub struct Layout {
    pub shape: JointShape,
    /// Column edges along X: the far left, the inner seams, the far right.
    pub xs: Vec<f64>,
    /// Row edges along Y: the far bottom, the inner seams, the far top.
    pub ys: Vec<f64>,
    /// The seams between columns, left to right.
    pub vertical: Vec<Seam>,
    /// The seams between rows, bottom to top.
    pub horizontal: Vec<Seam>,
}

/// The grid of `columns` × `rows` over the material `loops` within `bounds`, its seams moved
/// by the shifts, with joints of `shape` where the seams cross material.
pub fn layout(
    loops: &[Vec<Primitive>],
    bounds: (Point, Point),
    (columns, rows): (u32, u32),
    (shift_x, shift_y): (&[f64], &[f64]),
    shape: JointShape,
) -> Layout {
    let ((x0, y0), (x1, y1)) = bounds;
    let inner_x = seam_positions(x0, x1, columns, shift_x);
    let inner_y = seam_positions(y0, y1, rows, shift_y);
    let seams = |vertical: bool, at: &[f64], across: &[f64]| -> Vec<Seam> {
        at.iter()
            .map(|&a| Seam {
                vertical,
                at: a,
                features: features(&stretches(loops, vertical, a), across, &shape),
            })
            .collect()
    };
    let vertical = seams(true, &inner_x, &inner_y);
    let horizontal = seams(false, &inner_y, &inner_x);
    // Far enough out that every joint and the material lie inside the outer cells.
    let far = shape.protrusion() + shape.size + 1.0;
    let edges = |lo: f64, hi: f64, inner: Vec<f64>| {
        let mut e = vec![lo - far];
        e.extend(inner);
        e.push(hi + far);
        e
    };
    Layout {
        shape,
        xs: edges(x0, x1, inner_x),
        ys: edges(y0, y1, inner_y),
        vertical,
        horizontal,
    }
}

impl Layout {
    pub fn columns(&self) -> usize {
        self.xs.len() - 1
    }

    pub fn rows(&self) -> usize {
        self.ys.len() - 1
    }

    /// The cell in `column` and `row` (from 0, from the bottom left) as one closed
    /// counter-clockwise loop: its bottom, right, top and left edges, each a seam's portion
    /// or a straight outer edge.
    pub fn cell(&self, column: usize, row: usize) -> Vec<Primitive> {
        let (xa, xb) = (self.xs[column], self.xs[column + 1]);
        let (ya, yb) = (self.ys[row], self.ys[row + 1]);
        let straight = |vertical: bool, at: f64| Seam {
            vertical,
            at,
            features: Vec::new(),
        };
        let bottom = row
            .checked_sub(1)
            .map_or_else(|| straight(false, ya), |r| self.horizontal[r].clone());
        let top = self
            .horizontal
            .get(row)
            .cloned()
            .unwrap_or_else(|| straight(false, yb));
        let left = column
            .checked_sub(1)
            .map_or_else(|| straight(true, xa), |c| self.vertical[c].clone());
        let right = self
            .vertical
            .get(column)
            .cloned()
            .unwrap_or_else(|| straight(true, xb));
        let mut ring = bottom.portion(xa, xb, &self.shape);
        ring.extend(right.portion(ya, yb, &self.shape));
        ring.extend(reversed(top.portion(xa, xb, &self.shape)));
        ring.extend(reversed(left.portion(ya, yb, &self.shape)));
        ring
    }

    /// A cell's name: its row's letter from the bottom and its column's number from the
    /// left, "A1", "B3"; past Z, "AA1".
    pub fn name(column: usize, row: usize) -> String {
        let mut letters = String::new();
        let mut n = row + 1;
        while n > 0 {
            let d = (n - 1) % 26;
            letters.insert(0, char::from(b'A' + d as u8));
            n = (n - 1) / 26;
        }
        format!("{letters}{}", column + 1)
    }
}

/// A path walked the other way.
pub fn reversed(path: Vec<Primitive>) -> Vec<Primitive> {
    path.into_iter()
        .rev()
        .map(|p| match p {
            Primitive::Line(l) => Primitive::Line(Line {
                start: l.end,
                end: l.start,
            }),
            Primitive::Arc(a) => Primitive::Arc(Arc {
                start_angle: a.start_angle + a.sweep,
                sweep: -a.sweep,
                ..a
            }),
        })
        .collect()
}

/// The fewest columns × rows whose cells, each grown by the joints that may stick out of it
/// and by the board's `margin` on every side, fit a board of `board` (x, y). None when even
/// [`MAX_GRID`] × [`MAX_GRID`] doesn't.
pub fn auto_grid(
    bounds: (Point, Point),
    board: (f64, f64),
    margin: f64,
    (shift_x, shift_y): (&[f64], &[f64]),
    shape: &JointShape,
) -> Option<(u32, u32)> {
    let ((x0, y0), (x1, y1)) = bounds;
    // The widest cell of `count` along one axis, a joint sticking out past each inner seam.
    let widest = |lo: f64, hi: f64, count: u32, shifts: &[f64]| {
        let mut edges = vec![lo];
        edges.extend(seam_positions(lo, hi, count, shifts));
        edges.push(hi);
        edges
            .windows(2)
            .enumerate()
            .map(|(i, w)| {
                let seams = usize::from(i > 0) + usize::from(i + 2 < edges.len());
                w[1] - w[0] + seams as f64 * shape.protrusion()
            })
            .fold(0.0, f64::max)
            + 2.0 * margin
    };
    let fits = |lo: f64, hi: f64, room: f64, shifts: &[f64]| {
        (1..=MAX_GRID).find(|&n| widest(lo, hi, n, shifts) <= room)
    };
    Some((
        fits(x0, x1, board.0, shift_x)?,
        fits(y0, y1, board.1, shift_y)?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn knobs() -> JointShape {
        JointShape {
            joint: Joint::Knobs,
            size: 30.0,
            spacing: 120.0,
            round: 3.2,
        }
    }

    fn fingers() -> JointShape {
        JointShape {
            joint: Joint::Fingers,
            size: 15.0,
            spacing: 60.0,
            round: 3.2,
        }
    }

    fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<Primitive> {
        let pts = [(x0, y0), (x1, y0), (x1, y1), (x0, y1)];
        (0..4)
            .map(|i| {
                Primitive::Line(Line {
                    start: pts[i],
                    end: pts[(i + 1) % 4],
                })
            })
            .collect()
    }

    fn close(a: Point, b: Point) -> bool {
        math::hypot(a.0 - b.0, a.1 - b.1) < 1e-9
    }

    /// The direction a primitive leaves its start and arrives at its end.
    fn directions(p: &Primitive) -> (Point, Point) {
        match p {
            Primitive::Line(l) => {
                let len = math::hypot(l.end.0 - l.start.0, l.end.1 - l.start.1);
                let d = ((l.end.0 - l.start.0) / len, (l.end.1 - l.start.1) / len);
                (d, d)
            }
            Primitive::Arc(a) => {
                let s = a.sweep.signum();
                let at = |angle: f64| (-math::sin(angle) * s, math::cos(angle) * s);
                (at(a.start_angle), at(a.start_angle + a.sweep))
            }
        }
    }

    fn connected_and_smooth(path: &[Primitive], smooth: bool) {
        for w in path.windows(2) {
            assert!(close(w[0].end(), w[1].start()), "{w:?}");
            if smooth {
                let (_, out) = directions(&w[0]);
                let (into, _) = directions(&w[1]);
                assert!(close(out, into), "a corner between {w:?}");
            }
        }
    }

    fn area(ring: &[Primitive]) -> f64 {
        let pts: Vec<Point> = ring
            .iter()
            .flat_map(|p| {
                let mut v = p.points();
                v.pop();
                v
            })
            .collect();
        chipload_core::primitives::signed_area(&pts)
    }

    #[test]
    fn a_knob_is_smooth_and_as_wide_as_its_size() {
        for vertical in [false, true] {
            for side in [1.0, -1.0] {
                let seam = Seam {
                    vertical,
                    at: 10.0,
                    features: vec![Feature::Knob { at: 100.0, side }],
                };
                let path = seam.portion(0.0, 200.0, &knobs());
                assert_eq!(path.len(), 5, "{path:?}");
                connected_and_smooth(&path, true);
                let start = seam.xy((0.0, 0.0));
                let end = seam.xy((200.0, 0.0));
                assert!(close(path[0].start(), start) && close(path[4].end(), end));
                // The head is the size across and stands out on its side.
                let Primitive::Arc(head) = path[2] else {
                    panic!("{path:?}")
                };
                assert!((head.radius - 15.0).abs() < 1e-9);
                let out = if vertical {
                    head.center.0 - 10.0
                } else {
                    head.center.1 - 10.0
                };
                assert!(out * side > 0.0, "{head:?}");
                // More than half a circle: the head is wider than the neck and locks.
                assert!(head.sweep.abs() > PI);
            }
        }
    }

    #[test]
    fn a_knob_sticks_out_about_one_size_and_its_neck_is_narrower_than_its_head() {
        let shape = knobs();
        let k = shape.knob();
        assert!(
            (shape.protrusion() - 35.7).abs() < 0.1,
            "{}",
            shape.protrusion()
        );
        assert!((k.reach - k.fillet) * 2.0 < 2.0 * k.head);
        assert!((k.reach - k.head).abs() < 1e-9);
    }

    #[test]
    fn fingers_have_rounded_corners_and_stay_in_their_room() {
        let shape = fingers();
        let seam = Seam {
            vertical: false,
            at: 0.0,
            features: vec![Feature::Fingers {
                from: 50.0,
                teeth: 3,
                width: 30.0,
            }],
        };
        let path = seam.portion(0.0, 250.0, &shape);
        connected_and_smooth(&path, true);
        let arcs: Vec<&Arc> = path
            .iter()
            .filter_map(|p| match p {
                Primitive::Arc(a) => Some(a),
                Primitive::Line(_) => None,
            })
            .collect();
        // Four corners per tooth, every one the tool's round.
        assert_eq!(arcs.len(), 12);
        assert!(arcs.iter().all(|a| (a.radius - 3.2).abs() < 1e-9));
        for p in &path {
            let ((_, lo), (_, hi)) = p.bounds();
            assert!(lo > -1e-9 && hi < 15.0 + 1e-9, "{p:?}");
        }
    }

    #[test]
    fn stretches_follow_the_material_and_its_holes() {
        let outer = rect(0.0, 0.0, 100.0, 50.0);
        let hole = rect(40.0, 10.0, 60.0, 40.0);
        let loops = vec![outer, hole];
        assert_eq!(
            stretches(&loops, true, 50.0),
            vec![(0.0, 10.0), (40.0, 50.0)]
        );
        assert_eq!(stretches(&loops, true, 20.0), vec![(0.0, 50.0)]);
        assert!(stretches(&loops, true, 150.0).is_empty());
        assert_eq!(
            stretches(&loops, false, 25.0),
            vec![(0.0, 40.0), (60.0, 100.0)]
        );
    }

    #[test]
    fn knobs_alternate_and_keep_away_from_crossings_and_edges() {
        let shape = knobs();
        let feats = features(&[(0.0, 600.0)], &[300.0], &shape);
        let ats: Vec<(f64, f64)> = feats
            .iter()
            .map(|f| match *f {
                Feature::Knob { at, side } => (at, side),
                Feature::Fingers { .. } => panic!(),
            })
            .collect();
        // Each half: from 30 to 240, two knobs 120 apart, centred.
        assert_eq!(
            ats,
            vec![(75.0, 1.0), (195.0, -1.0), (405.0, 1.0), (525.0, -1.0)]
        );
        // A stretch too short for a knob stays straight.
        assert!(features(&[(0.0, 50.0)], &[], &shape).is_empty());
    }

    #[test]
    fn neighbouring_cells_share_their_seam_and_tile_the_plane() {
        for shape in [knobs(), fingers()] {
            let loops = vec![rect(0.0, 0.0, 900.0, 500.0)];
            let lay = layout(
                &loops,
                ((0.0, 0.0), (900.0, 500.0)),
                (3, 2),
                (&[], &[20.0]),
                shape,
            );
            assert_eq!((lay.columns(), lay.rows()), (3, 2));
            assert!(lay.vertical.iter().all(|s| !s.features.is_empty()));
            let mut total = 0.0;
            for c in 0..3 {
                for r in 0..2 {
                    let ring = lay.cell(c, r);
                    connected_and_smooth(&ring, false);
                    assert!(close(ring[0].start(), ring.last().unwrap().end()));
                    let a = area(&ring);
                    assert!(a > 0.0, "cell {c},{r} not counter-clockwise");
                    total += a;
                }
            }
            let w = lay.xs[3] - lay.xs[0];
            let h = lay.ys[2] - lay.ys[0];
            // The joints move area between cells, never add or lose any.
            assert!((total - w * h).abs() < 1.0, "{total} vs {}", w * h);
            // Cell A1's right edge is cell A2's left edge walked the other way.
            let a1 = lay.vertical[0].portion(lay.ys[0], lay.ys[1], &shape);
            let a2 = lay.cell(1, 0);
            let left = reversed(a2[a2.len() - a1.len()..].to_vec());
            for (p, q) in left.iter().zip(&a1) {
                assert!(
                    close(p.start(), q.start()) && close(p.end(), q.end()),
                    "{p:?} {q:?}"
                );
            }
        }
    }

    #[test]
    fn names_go_by_row_letter_and_column_number() {
        assert_eq!(Layout::name(0, 0), "A1");
        assert_eq!(Layout::name(2, 1), "B3");
        assert_eq!(Layout::name(0, 26), "AA1");
    }

    #[test]
    fn a_joint_too_small_for_the_tool_is_named() {
        let mut shape = knobs();
        assert!(shape.problem().is_none());
        shape.round = 6.5;
        assert!(shape.problem().is_some());
        assert!((shape.least_size() - 32.5).abs() < 1e-9);
        let mut f = fingers();
        assert!(f.problem().is_none());
        f.size = 6.0;
        assert!(f.problem().is_some());
    }

    #[test]
    fn the_automatic_grid_is_the_fewest_that_fit() {
        let shape = knobs();
        // 900 wide on 600 boards: two columns of 450 plus a knob and two 10 mm margins.
        let grid = auto_grid(
            ((0.0, 0.0), (900.0, 500.0)),
            (600.0, 600.0),
            10.0,
            (&[], &[]),
            &shape,
        );
        assert_eq!(grid, Some((2, 1)));
        let grid = auto_grid(
            ((0.0, 0.0), (900.0, 500.0)),
            (400.0, 400.0),
            10.0,
            (&[], &[]),
            &shape,
        );
        assert_eq!(grid, Some((3, 2)));
        assert_eq!(
            auto_grid(
                ((0.0, 0.0), (900.0, 500.0)),
                (50.0, 50.0),
                10.0,
                (&[], &[]),
                &shape
            ),
            None
        );
    }
}
