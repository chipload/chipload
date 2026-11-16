// SPDX-License-Identifier: GPL-3.0-or-later
//! A setup cut as a puzzle (docs/specs/pieces.md): the seams of a grid, their joints
//! (puzzle knobs or rounded fingers) and the cells they bound. Each seam is one curve of
//! lines and arcs, shared by the cells on both sides, so neighbouring pieces fit.
//!
//! Every seam is built in its own frame: `u` along the seam, `v` across it, `v = 0` the
//! straight seam. A vertical seam at `x = at` maps (u, v) to (at + v, u), a horizontal one at
//! `y = at` to (u, at + v).

use std::f64::consts::{FRAC_PI_2, PI};

use cavalier_contours::polyline::{BooleanOp, PlineSource, PlineSourceMut, Polyline};
use chipload_core::Point;
use chipload_core::chain::Contour;
use chipload_core::math;
use chipload_core::primitives::{Arc, Line, Primitive};
use chipload_core::tolerances::{ANGLE_TOL_RAD, CHAIN_TOL_MM};
use chipload_i18n::{Digits, Message, Value};
use chipload_job::{Job, Joint, Operation, Side};

use crate::ToolpathError;
use crate::cavalier::{Area, boolean, ccw, geometry, step};
use crate::model::{Move, MoveKind};
use crate::offset::{contour_pline, flatten, pline_segments};
use crate::plan::op_tool;
use crate::select::select_contours;

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

/// A closed loop of lines and arcs as a polyline with bulges; arcs over half a turn are
/// split, since a bulge holds at most a full turn and loses precision near it.
pub fn pline_of(ring: &[Primitive]) -> Polyline<f64> {
    let mut pline = Polyline::new_closed();
    for prim in ring {
        match *prim {
            Primitive::Line(line) => pline.add(line.start.0, line.start.1, 0.0),
            Primitive::Arc(arc) => {
                let pieces: u32 = if arc.sweep.abs() > PI { 2 } else { 1 };
                let sweep = arc.sweep / f64::from(pieces);
                for i in 0..pieces {
                    let piece = Arc {
                        start_angle: arc.start_angle + sweep * f64::from(i),
                        sweep,
                        ..arc
                    };
                    let start = piece.point_at(piece.start_angle);
                    pline.add(start.0, start.1, piece.bulge());
                }
            }
        }
    }
    pline
}

/// A closed loop of lines and arcs as a drawn polyline's points and bulges.
pub fn bulged(ring: &[Primitive]) -> (Vec<[f64; 2]>, Vec<f64>) {
    let pline = pline_of(ring);
    (0..pline.vertex_count())
        .map(|i| {
            let v = pline.at(i);
            ([v.x, v.y], v.bulge)
        })
        .unzip()
}

/// A piece's material: the cell shrunk by half the `clearance` (its seam edges move in, so
/// two pieces leave the clearance between them), intersected with each outer outline the
/// cut-out operations cut (docs/specs/pieces.md §5, §6). Counter-clockwise loops of lines
/// and arcs; none when the cell holds no material.
pub fn regions(
    outers: &[Vec<Primitive>],
    cell: &[Primitive],
    clearance: f64,
) -> Result<Vec<Vec<Primitive>>, ToolpathError> {
    let cell = ccw(pline_of(cell));
    let cells: Vec<Polyline<f64>> = if clearance > 0.0 {
        Area::of(cell)
            .shrunk(clearance / 2.0)?
            .into_iter()
            .map(|area| area.outer)
            .collect()
    } else {
        vec![cell]
    };
    let mut out = Vec::new();
    for outer in outers {
        let outer = ccw(pline_of(outer));
        for cell in &cells {
            let result = boolean(step("piece-outline"), &outer, cell, BooleanOp::And)?;
            if !result.neg_plines.is_empty() {
                return Err(geometry(
                    Message::new("toolpath-error-library-failed")
                        .arg("what", step("piece-outline")),
                ));
            }
            for piece in result.pos_plines {
                let pline = ccw(piece.pline);
                if pline.area().abs() > CHAIN_TOL_MM {
                    out.push(pline_segments(&pline));
                }
            }
        }
    }
    Ok(out)
}

/// Where a tool's centre may go for a piece: its cell grown by the tool's `radius`, as
/// points within the arc tolerance (docs/specs/pieces.md §6.2). Each side then cuts a
/// little past its seam, into what is waste on its own board.
pub fn reach(cell: &[Primitive], radius: f64) -> Result<Vec<Point>, ToolpathError> {
    let cell = ccw(pline_of(cell));
    let grown = if radius > 0.0 {
        Area::of(cell).shrunk(-radius)?
    } else {
        vec![Area::of(cell)]
    };
    match grown.as_slice() {
        [one] => Ok(flatten(&one.outer)),
        _ => Err(geometry(
            Message::new("toolpath-error-library-failed").arg("what", step("piece-reach")),
        )),
    }
}

/// Whether `p` lies inside the polygon `ring` (even-odd, half-open, so a point on an edge
/// counts for one side only).
fn inside(ring: &[Point], p: Point) -> bool {
    let mut odd = false;
    for i in 0..ring.len() {
        let (a, b) = (ring[i], ring[(i + 1) % ring.len()]);
        if (a.1 > p.1) != (b.1 > p.1) {
            let x = a.0 + (p.1 - a.1) / (b.1 - a.1) * (b.0 - a.0);
            if p.0 < x {
                odd = !odd;
            }
        }
    }
    odd
}

/// Where along the segment from `p` to `q` (0 to 1) it crosses the polygon's edges.
fn crossings(ring: &[Point], p: Point, q: Point) -> Vec<f64> {
    let d = (q.0 - p.0, q.1 - p.1);
    let mut out = Vec::new();
    for i in 0..ring.len() {
        let (a, b) = (ring[i], ring[(i + 1) % ring.len()]);
        let e = (b.0 - a.0, b.1 - a.1);
        let denom = d.0 * e.1 - d.1 * e.0;
        // Parallel: no crossing point; the test of each part's middle decides.
        if denom == 0.0 {
            continue;
        }
        let w = (a.0 - p.0, a.1 - p.1);
        let t = (w.0 * e.1 - w.1 * e.0) / denom;
        let s = (w.0 * d.1 - w.1 * d.0) / denom;
        if t > 0.0 && t < 1.0 && (0.0..=1.0).contains(&s) {
            out.push(t);
        }
    }
    out.sort_by(f64::total_cmp);
    out
}

/// The moves of one piece's program, built from operations' moves kept only where the
/// tool's centre lies within the piece's reach (docs/specs/pieces.md §6). A cut that leaves
/// the reach ends; where it comes back the tool retracts to the safe height, rapids over,
/// rapids down to just above the board and plunges at the plunge feed.
pub struct Trimmer {
    pub moves: Vec<Move>,
    safe_z: f64,
    /// A rapid down stops this high, above the board's top: below it the tool feeds.
    approach_z: f64,
    /// Where the tool is in the trimmed program; None before the first move.
    at: Option<(f64, f64, f64)>,
    /// The trimmed tool follows the original path without a break.
    following: bool,
    /// A comment waiting for the next move written.
    comment: Option<u32>,
    /// Entries made in the middle of a cut, below the board's top: where a cut was split.
    pub split_entries: usize,
}

impl Trimmer {
    pub fn new(safe_z: f64, approach_z: f64) -> Trimmer {
        Trimmer {
            moves: Vec::new(),
            safe_z,
            approach_z,
            at: None,
            following: false,
            comment: None,
            split_entries: 0,
        }
    }

    fn push(&mut self, mut m: Move) {
        if m.comment.is_none() {
            m.comment = self.comment.take();
        }
        self.moves.push(m);
    }

    /// The moves `moves`, starting from `from` (where the original program stood before
    /// them), kept within `reach` (None: all of them). `plunge` is the feed for an entry's
    /// plunge.
    pub fn add(
        &mut self,
        moves: &[Move],
        from: (f64, f64, f64),
        reach: Option<&[Point]>,
        plunge: f64,
    ) -> Result<(), ToolpathError> {
        let mut p = from;
        let mut cutting = false;
        // Another operation's moves, or another program's: never followed on from.
        self.following = false;
        for m in moves {
            if m.comment.is_some() && self.comment.is_none() {
                self.comment = m.comment;
            }
            let q = (m.x.unwrap_or(p.0), m.y.unwrap_or(p.1), m.z.unwrap_or(p.2));
            match m.kind {
                MoveKind::Rapid => {
                    self.following = false;
                    cutting = false;
                }
                MoveKind::Feed => {
                    let feed = m.feed.unwrap_or(plunge);
                    self.feed(p, q, feed, m, reach, plunge, cutting);
                    cutting = true;
                }
                MoveKind::ArcCw | MoveKind::ArcCcw => {
                    return Err(geometry(
                        Message::new("toolpath-error-library-failed")
                            .arg("what", step("piece-reach")),
                    ));
                }
            }
            p = q;
        }
        Ok(())
    }

    /// One feed from `p` to `q`, kept where it lies within `reach`.
    #[allow(clippy::too_many_arguments)]
    fn feed(
        &mut self,
        p: (f64, f64, f64),
        q: (f64, f64, f64),
        feed: f64,
        original: &Move,
        reach: Option<&[Point]>,
        plunge: f64,
        cutting: bool,
    ) {
        let lerp = |t: f64| {
            (
                p.0 + t * (q.0 - p.0),
                p.1 + t * (q.1 - p.1),
                p.2 + t * (q.2 - p.2),
            )
        };
        let mut ts = vec![0.0];
        if let Some(reach) = reach {
            ts.extend(crossings(reach, (p.0, p.1), (q.0, q.1)));
        }
        ts.push(1.0);
        let pieces = ts.len() - 1;
        for w in ts.windows(2) {
            let (ta, tb) = (w[0], w[1]);
            let mid = lerp((ta + tb) / 2.0);
            if reach.is_some_and(|reach| !inside(reach, (mid.0, mid.1))) {
                self.following = false;
                continue;
            }
            let (a, b) = (lerp(ta), lerp(tb));
            if !self.following {
                // The original was cutting here already: the share starts mid-cut.
                if cutting || ta > 0.0 {
                    self.split_entries += usize::from(a.2 < self.approach_z);
                }
                self.enter(a, plunge);
            }
            let mut m = Move::feed_xyz((b.0, b.1), b.2, feed);
            // A whole chord keeps its arc; a part of one is a straight line.
            if pieces == 1 {
                m.circle = original.circle;
            }
            self.push(m);
            self.at = Some(b);
            self.following = tb >= 1.0;
        }
    }

    /// Up to the safe height, over to `a`, down to just above the board and in at the
    /// plunge feed.
    fn enter(&mut self, a: (f64, f64, f64), plunge: f64) {
        if self.at.is_none_or(|at| at.2 < self.safe_z) {
            self.push(Move::rapid_z(self.safe_z));
        }
        self.push(Move::rapid_xy((a.0, a.1)));
        if a.2 >= self.approach_z {
            self.push(Move::rapid_z(a.2));
        } else {
            self.push(Move::rapid_z(self.approach_z));
            self.push(Move::feed_z(a.2, plunge));
        }
        self.at = Some(a);
        self.following = true;
    }

    /// A tool group's end: back up to the safe height, and the next group, a program or a
    /// tool change of its own, starts from an unknown height, so it climbs to the safe
    /// height before anything else.
    pub fn end_group(&mut self) {
        self.finish();
        self.at = None;
    }

    /// The program's end: back up to the safe height.
    pub fn finish(&mut self) {
        if self.at.is_some_and(|at| at.2 < self.safe_z) {
            self.push(Move::rapid_z(self.safe_z));
            if let Some(at) = &mut self.at {
                at.2 = self.safe_z;
            }
        }
        self.following = false;
    }
}

/// Where the tool stands before each move of `moves`, starting from `start`.
pub fn positions(moves: &[Move], start: (f64, f64, f64)) -> Vec<(f64, f64, f64)> {
    let mut p = start;
    let mut out = Vec::with_capacity(moves.len() + 1);
    for m in moves {
        out.push(p);
        p = (m.x.unwrap_or(p.0), m.y.unwrap_or(p.1), m.z.unwrap_or(p.2));
    }
    out.push(p);
    out
}

/// One piece of a setup cut as a puzzle.
#[derive(Debug, Clone, PartialEq)]
pub struct Piece {
    /// "A1": its row's letter from the bottom, its column's number from the left.
    pub name: String,
    pub column: usize,
    pub row: usize,
    /// The cell: the piece's share of the plane, bounded by the seams' joints.
    pub cell: Vec<Primitive>,
    /// The material the cut-out operations cut out for it: its loops, counter-clockwise.
    pub regions: Vec<Vec<Primitive>>,
}

/// A setup's pieces, worked out from its project (docs/specs/pieces.md §3–§6).
#[derive(Debug, Clone, PartialEq)]
pub struct PieceSet {
    pub layout: Layout,
    /// The pieces with material, in order: A1, A2, …, B1, …
    pub pieces: Vec<Piece>,
    /// The cut-out operations (indices in `job.operations`): outside profiles, which cut
    /// each piece's own loops instead of the outlines they select.
    pub cut_out: Vec<usize>,
}

impl PieceSet {
    pub fn piece(&self, name: &str) -> Option<&Piece> {
        self.pieces.iter().find(|p| p.name == name)
    }
}

/// A contour as a loop of lines and arcs.
fn contour_ring(contour: &Contour) -> Vec<Primitive> {
    pline_segments(&contour_pline(contour))
}

/// The pieces of `job`'s active setup, or None when it is one board. `contours` are the
/// drawing's. Refused when nothing cuts an outline out, when the joints are too small for
/// the cut-out tool, when no grid fits the board, or when a seam is shifted that doesn't
/// exist or two seams come too close.
pub fn piece_set(job: &Job, contours: &[Contour]) -> Result<Option<PieceSet>, ToolpathError> {
    let one = job.active_setup_id().map(|id| job.for_setup(id));
    let job = one.as_ref().unwrap_or(job);
    let Some(setup) = job.setups.get(job.active_setup_index()) else {
        return Ok(None);
    };
    let Some(pieces) = &setup.pieces else {
        return Ok(None);
    };
    let mut cut_out = Vec::new();
    let mut outers: Vec<Vec<Primitive>> = Vec::new();
    let mut holes: Vec<Vec<Primitive>> = Vec::new();
    let mut radius: f64 = 0.0;
    for (i, op) in job.operations.iter().enumerate() {
        let Operation::Profile(profile) = op else {
            continue;
        };
        if !job.cuts(i) || profile.side == Side::On {
            continue;
        }
        let label = job.label_message(chipload_job::OpRef::op(i));
        let selection = select_contours(contours, &op.select().resolved(&job.drawing), &label)
            .map_err(|e| e.at(i))?;
        let rings = selection.contours.iter().map(|c| contour_ring(c));
        if profile.side == Side::Outside {
            cut_out.push(i);
            outers.extend(rings);
            radius = radius.max(op_tool(job, op).map_err(|e| e.at(i))?.diameter / 2.0);
        } else {
            holes.extend(rings);
        }
    }
    if outers.is_empty() {
        return Err(ToolpathError::Job(Message::new(
            "toolpath-pieces-no-cut-out",
        )));
    }
    let shape = JointShape {
        joint: pieces.joint,
        size: pieces.size,
        spacing: pieces.spacing,
        round: radius + pieces.clearance,
    };
    if let Some(problem) = shape.problem() {
        return Err(ToolpathError::Job(problem));
    }
    let bounds = bounds_of(&outers);
    let margin = setup
        .stock
        .fit
        .unwrap_or(chipload_job::DEFAULT_PIECE_MARGIN);
    let shifts = (pieces.shift_x.as_slice(), pieces.shift_y.as_slice());
    let (columns, rows) = match pieces.grid {
        Some(g) => (g.columns, g.rows),
        None => auto_grid(
            bounds,
            (pieces.board.x, pieces.board.y),
            margin,
            shifts,
            &shape,
        )
        .ok_or_else(|| {
            ToolpathError::Job(
                Message::new("toolpath-pieces-board-too-small")
                    .arg("max", MAX_GRID)
                    .arg("x", Value::mm(pieces.board.x))
                    .arg("y", Value::mm(pieces.board.y)),
            )
        })?,
    };
    for (axis, count, list) in [("X", columns, shifts.0), ("Y", rows, shifts.1)] {
        if list.len() >= count as usize && !list.is_empty() {
            return Err(ToolpathError::Job(
                Message::new("toolpath-pieces-no-seam")
                    .arg("axis", axis)
                    .arg("seam", list.len())
                    .arg("seams", count.saturating_sub(1)),
            ));
        }
    }
    // Seams in order and apart, so no cell is too thin for its joints.
    let ((x0, y0), (x1, y1)) = bounds;
    for (lo, hi, count, list) in [(x0, x1, columns, shifts.0), (y0, y1, rows, shifts.1)] {
        let mut edges = vec![lo - shape.size];
        edges.extend(seam_positions(lo, hi, count, list));
        edges.push(hi + shape.size);
        if edges.windows(2).any(|w| w[1] - w[0] < 3.0 * shape.size) {
            return Err(ToolpathError::Job(
                Message::new("toolpath-pieces-seams-close")
                    .arg("least", Value::mm(3.0 * shape.size)),
            ));
        }
    }
    let mut material = outers.clone();
    material.extend(holes);
    let layout = layout(&material, bounds, (columns, rows), shifts, shape);
    let mut list = Vec::new();
    for row in 0..layout.rows() {
        for column in 0..layout.columns() {
            let cell = layout.cell(column, row);
            let regions = regions(&outers, &cell, pieces.clearance)?;
            if regions.is_empty() {
                continue;
            }
            list.push(Piece {
                name: Layout::name(column, row),
                column,
                row,
                cell,
                regions,
            });
        }
    }
    Ok(Some(PieceSet {
        layout,
        pieces: list,
        cut_out,
    }))
}

/// The box round loops of lines and arcs.
fn bounds_of(loops: &[Vec<Primitive>]) -> (Point, Point) {
    let mut lo = (f64::INFINITY, f64::INFINITY);
    let mut hi = (f64::NEG_INFINITY, f64::NEG_INFINITY);
    for p in loops.iter().flatten() {
        let (a, b) = p.bounds();
        lo = (lo.0.min(a.0), lo.1.min(a.1));
        hi = (hi.0.max(b.0), hi.1.max(b.1));
    }
    (lo, hi)
}

#[cfg(test)]
mod tests;
