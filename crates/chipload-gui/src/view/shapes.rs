// SPDX-License-Identifier: GPL-3.0-or-later
//! What the canvas draws for contours and toolpaths, picked and highlighted.

use std::collections::BTreeSet;

use chipload_core::Point;
use chipload_core::chain::Contour;
use chipload_core::math;
use chipload_toolpath::OpSpan;
use chipload_toolpath::model::{MoveKind, Program};

use super::{ARROW_SPACING_MM, Point3};

/// How a contour is shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Outer,
    Hole,
    Open,
    Invalid,
    /// Construction geometry: shown and snapped to, never cut.
    Construction,
}

impl Kind {
    pub fn of(contour: &Contour) -> Kind {
        if contour.is_construction() {
            Kind::Construction
        } else if !contour.closed {
            Kind::Open
        } else if contour.invalid_reason.is_some() {
            Kind::Invalid
        } else if contour.nesting_depth.unwrap_or(0) % 2 == 1 {
            Kind::Hole
        } else {
            Kind::Outer
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Kind::Outer => "outer",
            Kind::Hole => "hole",
            Kind::Open => "open",
            Kind::Invalid => "INVALID",
            Kind::Construction => "construction",
        }
    }
}

/// A contour ready to draw: its id, kind and polyline (closed ones repeat the start).
#[derive(Debug, Clone, PartialEq)]
pub struct ContourShape {
    pub id: usize,
    pub kind: Kind,
    pub points: Vec<Point>,
}

impl ContourShape {
    pub fn new(contour: &Contour) -> Self {
        let mut points = contour.points();
        if contour.closed
            && let Some(&first) = points.first()
        {
            points.push(first);
        }
        ContourShape {
            id: contour.id,
            kind: Kind::of(contour),
            points,
        }
    }
}

/// Where the tool goes down to cut: the end of a rapid move to a new XY position.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Plunge {
    pub at: Point,
    /// Index in `job.operations` (None for a rejected program shown without them).
    pub operation: Option<usize>,
}

/// A program as drawable pieces. Feeds solid, rapids dashed, plunge points (each operation
/// numbered once), arrows in the cutting direction.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ToolpathShape {
    /// Runs of consecutive XY feed moves.
    pub feeds: Vec<Vec<Point>>,
    /// The same for the copies of the part (docs/specs/copies-grid.md), drawn faded.
    pub ghost_feeds: Vec<Vec<Point>>,
    pub rapids: Vec<(Point, Point)>,
    /// Where each rapid ends, in cut order.
    pub starts: Vec<Plunge>,
    /// (position, direction angle in radians, CCW from +X)
    pub arrows: Vec<(Point, f64)>,
    /// Every move in 3D for the 3D view, plunges and retracts included: (from, to, is a
    /// cutting move).
    pub lines3: Vec<(Point3, Point3, bool)>,
    /// The move each of `lines3` is (index in the program), for playback.
    pub line_moves: Vec<usize>,
    pub rejected: bool,
    /// The project changed since: drawn faded until a new one replaces it.
    pub stale: bool,
}

impl ToolpathShape {
    /// `spans` says which moves each operation made (empty: unknown).
    pub fn new(program: &Program, rejected: bool, spans: &[OpSpan]) -> Self {
        let mut shape = ToolpathShape {
            rejected,
            ..Default::default()
        };
        let mut here: Option<Point> = None;
        let (mut x, mut y): (Option<f64>, Option<f64>) = (None, None);
        let mut since_arrow = ARROW_SPACING_MM; // so the first cut of every ring gets an arrow
        let mut run: Vec<Point> = Vec::new();
        let mut ghost = false;
        for (index, m) in program.moves.iter().enumerate() {
            let span = spans.iter().find(|s| s.moves.contains(&index));
            // Moves of a copy: faded, without plunge markers or arrows.
            let in_copy = span.is_some_and(|s| s.copy != 1);
            if in_copy != ghost {
                shape.flush(&mut run, ghost);
                ghost = in_copy;
            }
            let (nx, ny) = (m.x.or(x), m.y.or(y));
            let (Some(px), Some(py)) = (nx, ny) else {
                (x, y) = (nx, ny);
                continue;
            };
            let next = (px, py);
            if here == Some(next) {
                continue; // Z only (plunge, retract, tab lift): nothing to draw in plan view
            }
            match m.kind {
                MoveKind::Rapid => {
                    if let Some(from) = here {
                        shape.rapids.push((from, next));
                    }
                    if !ghost {
                        shape.starts.push(Plunge {
                            at: next,
                            operation: span.map(|s| s.operation),
                        });
                    }
                    since_arrow = ARROW_SPACING_MM;
                    shape.flush(&mut run, ghost);
                }
                // The canvas shows the planned program, whose arcs are still chords; an arc
                // move (never shown today) is drawn as its chord.
                MoveKind::Feed | MoveKind::ArcCw | MoveKind::ArcCcw => {
                    if let Some(from) = here {
                        if run.last() != Some(&from) {
                            shape.flush(&mut run, ghost);
                            run = vec![from];
                        }
                        run.push(next);
                        let (dx, dy) = (next.0 - from.0, next.1 - from.1);
                        if since_arrow >= ARROW_SPACING_MM && !ghost {
                            let mid = ((from.0 + next.0) / 2.0, (from.1 + next.1) / 2.0);
                            shape.arrows.push((mid, math::atan2(dy, dx)));
                            since_arrow = 0.0;
                        }
                        since_arrow += math::hypot(dx, dy);
                    }
                }
            }
            here = Some(next);
            (x, y) = (nx, ny);
        }
        shape.flush(&mut run, ghost);
        (shape.lines3, shape.line_moves) = lines_3d(program);
        shape
    }

    /// Each operation's first plunge, where its number is shown: (position, index in
    /// `job.operations`), in cut order.
    pub fn op_labels(&self) -> Vec<(Point, usize)> {
        let mut labelled: BTreeSet<usize> = BTreeSet::new();
        self.starts
            .iter()
            .filter_map(|p| p.operation.map(|op| (p.at, op)))
            .filter(|&(_, op)| labelled.insert(op))
            .collect()
    }

    /// End a run of feed moves: kept if it draws anything, then started afresh.
    fn flush(&mut self, run: &mut Vec<Point>, ghost: bool) {
        let done = std::mem::take(run);
        if done.len() > 1 {
            if ghost {
                self.ghost_feeds.push(done);
            } else {
                self.feeds.push(done);
            }
        }
    }

    /// The Z range of the cutting moves: (highest, lowest).
    pub fn cut_z_range(&self) -> Option<(f64, f64)> {
        self.lines3
            .iter()
            .filter(|l| l.2)
            .flat_map(|(a, b, _)| [a.2, b.2])
            .fold(None, |range, z| match range {
                None => Some((z, z)),
                Some((hi, lo)) => Some((f64::max(hi, z), f64::min(lo, z))),
            })
    }
}

/// What to emphasise when painting.
#[derive(Debug, Clone, Default)]
pub struct Highlight {
    /// The canvas selection.
    pub selected: BTreeSet<usize>,
    /// Contours of the operation selected in the operations list (drawn wider, under the
    /// selection colour).
    pub operation_contours: BTreeSet<usize>,
    /// That operation's index.
    pub operation: Option<usize>,
    /// Open contours that a follow path cuts: drawn solid, not dashed as "not cut".
    pub followed: BTreeSet<usize>,
}

/// Closed loops, each with its line width.
pub type RimLines = Vec<(Vec<Point>, f32)>;

/// Every move as a 3D segment, once the tool's position is known on all three axes.
type Lines3 = (Vec<(Point3, Point3, bool)>, Vec<usize>);

fn lines_3d(program: &Program) -> Lines3 {
    let mut lines = Vec::new();
    let mut moves = Vec::new();
    let (mut x, mut y, mut z): (Option<f64>, Option<f64>, Option<f64>) = (None, None, None);
    for (index, m) in program.moves.iter().enumerate() {
        let from = match (x, y, z) {
            (Some(x), Some(y), Some(z)) => Some((x, y, z)),
            _ => None,
        };
        (x, y, z) = (m.x.or(x), m.y.or(y), m.z.or(z));
        if let (Some(from), Some(x), Some(y), Some(z)) = (from, x, y, z)
            && from != (x, y, z)
        {
            lines.push((from, (x, y, z), m.kind != MoveKind::Rapid));
            moves.push(index);
        }
    }
    (lines, moves)
}
