// SPDX-License-Identifier: GPL-3.0-or-later
//! Dowel holes for turning a board over (docs/specs/two-sided.md §6): two drawn circles on
//! the line the board turns about, one at each end in the margin beyond both sides'
//! shapes, cut first with the front, through the board and into the spoilboard for the
//! pins. On the turned board each hole lands on itself, so a pin through both locates it.
//! The holes go beyond where the tools cut, not just beyond the shapes: an outside profile
//! reaches a tool's diameter past its outline, and on the back the pins stand there.

use chipload_core::Point;
use chipload_core::tolerances::CHAIN_TOL_MM;
use chipload_i18n::{Digits, Language, Message, Units, Value};
use chipload_job::{Geometry, Job, Note, Operation, Selector, StockOrigin, Turn, WorkZero};
use chipload_toolpath::OpSpan;
use chipload_toolpath::model::{MoveKind, Program};

use crate::PipelineError;
use crate::new_ops::{NewRequest, OpKind, new_operations};
use crate::shapes::add_shape;

/// The pins a new pair of dowel holes is for: 8 mm, which a 6 mm end mill bores (a bore
/// needs 1.2 × the tool), 8 mm into the spoilboard.
pub const DEFAULT_PIN_DIAMETER: f64 = 8.0;
pub const DEFAULT_PIN_DEPTH: f64 = 8.0;

/// Room left beside a hole in the margin, each side, mm.
const PIN_ROOM: f64 = 1.0;

/// The layer dowel circles are drawn on.
pub const DOWEL_LAYER: &str = "dowels";

/// The dowel holes asked for.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Dowels {
    pub diameter: f64,
    /// How far into the spoilboard, mm.
    pub depth: f64,
    /// Set the front's work zero to the first pin's centre, which the back shares.
    pub zero: bool,
}

impl Default for Dowels {
    fn default() -> Self {
        Dowels {
            diameter: DEFAULT_PIN_DIAMETER,
            depth: DEFAULT_PIN_DEPTH,
            zero: false,
        }
    }
}

fn refused(message: Message) -> PipelineError {
    PipelineError::Job(message)
}

/// The centres of the two holes for the back setup `back`: on its turn axis, each centred
/// in the margin between where both sides cut (and their shapes) and the board's edge, the
/// lower (left-right) or the left (top-bottom) first. Refused when the margin can't hold the
/// hole, saying how wide it needs to be.
pub fn dowel_centres(job: &Job, back: u64, diameter: f64) -> Result<[Point; 2], PipelineError> {
    let Some((front, turn)) = job
        .setup(back)
        .and_then(|s| s.back_of)
        .map(|b| (b.setup, b.turn))
    else {
        return Err(refused(
            Message::new("pipeline-setup-not-a-back").arg("id", back),
        ));
    };
    let one = job.for_setup(front);
    let ((x0, y0), (x1, y1)) = one.stock_bounds();
    let Some(shapes) = one.board_extents() else {
        return Err(refused(Message::new("pipeline-dowels-no-shapes")));
    };
    let mut reach = shapes;
    for side in [front, back] {
        if let Some(cut) = cut_extents(&job.for_setup(side))? {
            reach = union(reach, cut);
        }
    }
    let ((px0, py0), (px1, py1)) = reach;
    let (gaps, centres) = match turn {
        Turn::LeftRight => {
            let x = (x0 + x1) / 2.0;
            (
                [py0 - y0, y1 - py1],
                [(x, (y0 + py0) / 2.0), (x, (py1 + y1) / 2.0)],
            )
        }
        Turn::TopBottom => {
            let y = (y0 + y1) / 2.0;
            (
                [px0 - x0, x1 - px1],
                [((x0 + px0) / 2.0, y), ((px1 + x1) / 2.0, y)],
            )
        }
    };
    let needed = diameter + 2.0 * PIN_ROOM;
    let narrowest = gaps[0].min(gaps[1]);
    // A margin exactly as wide as needed holds the hole.
    if narrowest + CHAIN_TOL_MM < needed {
        return Err(refused(no_room(&one, narrowest, needed)));
    }
    Ok(centres)
}

/// Why the holes don't fit: the room there is and the room a hole needs, and for a board
/// fitted round the drawing the margin that would hold them, rounded up to 0.1 mm (the
/// margin grows the room on both ends alike).
fn no_room(front: &Job, room: f64, needed: f64) -> Message {
    let length = |v: f64| Value::Length(v, Digits::Fixed(1));
    let message = |id: &'static str| {
        Message::new(id)
            .arg("room", length(room.max(0.0)))
            .arg("needed", length(needed))
    };
    match (front.stock().origin, front.stock().fit) {
        (StockOrigin::Drawing, Some(margin)) => {
            let wider = ((margin + needed - room) * 10.0 - CHAIN_TOL_MM).ceil() / 10.0;
            message("pipeline-dowels-no-room-margin").arg("margin", length(wider))
        }
        _ => message("pipeline-dowels-no-room"),
    }
}

/// A (min, max) box in the drawing's coordinates.
type Box2 = ((f64, f64), (f64, f64));

fn union(a: Box2, b: Box2) -> Box2 {
    (
        (a.0.0.min(b.0.0), a.0.1.min(b.0.1)),
        (a.1.0.max(b.1.0), a.1.1.max(b.1.1)),
    )
}

/// Where a setup's tools cut below its stock's top, the tools' own width included, in the
/// drawing's coordinates (a back's as planned, before it is mirrored), dowel holes and
/// surface passes left out. None when nothing is cut.
fn cut_extents(side: &Job) -> Result<Option<Box2>, PipelineError> {
    // A side with nothing to cut yet (a new back) has nothing to plan.
    if !(0..side.operations.len()).any(|i| side.cuts(i)) {
        return Ok(None);
    }
    let contours = crate::load_drawing(side)?.contours;
    let plan = chipload_toolpath::plan_job(side, &contours)?;
    let mut extents: Option<Box2> = None;
    for cut in cuts(side, &plan.program, &plan.op_spans) {
        let ((ax, ay), (bx, by)) = cut.bounds();
        let r = cut.radius;
        let grown = ((ax - r, ay - r), (bx + r, by + r));
        extents = Some(extents.map_or(grown, |e| union(e, grown)));
    }
    Ok(extents)
}

/// One move of a planned program that cuts below the stock's top.
struct Cut {
    operation: usize,
    from: Point,
    to: Point,
    /// An arc's centre, radius and whether it turns clockwise.
    arc: Option<(Point, f64, bool)>,
    /// The tool's radius.
    radius: f64,
}

impl Cut {
    /// The (min, max) corners of the path, an arc's bulge included.
    fn bounds(&self) -> Box2 {
        let mut points = vec![self.from, self.to];
        if let Some((c, r, _)) = self.arc {
            for (dx, dy) in [(r, 0.0), (0.0, r), (-r, 0.0), (0.0, -r)] {
                let p = (c.0 + dx, c.1 + dy);
                if self.sweeps(p) {
                    points.push(p);
                }
            }
        }
        let lo = points.iter().fold((f64::INFINITY, f64::INFINITY), |a, p| {
            (a.0.min(p.0), a.1.min(p.1))
        });
        let hi = points
            .iter()
            .fold((f64::NEG_INFINITY, f64::NEG_INFINITY), |a, p| {
                (a.0.max(p.0), a.1.max(p.1))
            });
        (lo, hi)
    }

    /// Whether the direction from an arc's centre to `p` lies within its sweep.
    fn sweeps(&self, p: Point) -> bool {
        let Some((c, _, clockwise)) = self.arc else {
            return false;
        };
        let angle = |q: Point| (q.1 - c.1).atan2(q.0 - c.0);
        let along = |a: f64| {
            let d = if clockwise {
                angle(self.from) - a
            } else {
                a - angle(self.from)
            };
            d.rem_euclid(std::f64::consts::TAU)
        };
        along(angle(p)) <= along(angle(self.to))
    }

    /// The shortest distance from `p` to the path.
    fn distance(&self, p: Point) -> f64 {
        let to_end = |q: Point| (p.0 - q.0).hypot(p.1 - q.1);
        let ends = to_end(self.from).min(to_end(self.to));
        match self.arc {
            Some((c, r, _)) if self.sweeps(p) => ((p.0 - c.0).hypot(p.1 - c.1) - r).abs(),
            Some(_) => ends,
            None => {
                let (dx, dy) = (self.to.0 - self.from.0, self.to.1 - self.from.1);
                let length2 = dx * dx + dy * dy;
                if length2 <= CHAIN_TOL_MM * CHAIN_TOL_MM {
                    return ends;
                }
                let t = (((p.0 - self.from.0) * dx + (p.1 - self.from.1) * dy) / length2)
                    .clamp(0.0, 1.0);
                to_end((self.from.0 + t * dx, self.from.1 + t * dy))
            }
        }
    }
}

/// Every cutting move of `program` below the stock's top, in cut order, but for those of
/// dowel holes and surface passes.
fn cuts(job: &Job, program: &Program, spans: &[OpSpan]) -> Vec<Cut> {
    let top = job.stock().top_z();
    let mut out = Vec::new();
    let (mut x, mut y, mut z) = (None, None, None);
    for (i, m) in program.moves.iter().enumerate() {
        let before = (x, y, z);
        x = m.x.or(x);
        y = m.y.or(y);
        z = m.z.or(z);
        if m.kind == MoveKind::Rapid {
            continue;
        }
        let Some(span) = spans.iter().find(|s| s.moves.contains(&i)) else {
            continue;
        };
        // A surface pass faces the board shallowly, edge to edge, before the holes are made.
        let counts = |op: &Operation| !op.is_dowels() && !matches!(op, Operation::Surface(_));
        if !job.operations.get(span.operation).is_some_and(counts) {
            continue;
        }
        let (Some(x0), Some(y0), Some(z0), Some(x1), Some(y1), Some(z1)) =
            (before.0, before.1, before.2, x, y, z)
        else {
            continue;
        };
        if z0.min(z1) >= top - CHAIN_TOL_MM {
            continue;
        }
        let arc = match (m.kind, m.circle) {
            (MoveKind::ArcCw, Some(c)) => Some((c.center, c.radius, true)),
            (MoveKind::ArcCcw, Some(c)) => Some((c.center, c.radius, false)),
            _ => None,
        };
        out.push(Cut {
            operation: span.operation,
            from: (x0, y0),
            to: (x1, y1),
            arc,
            radius: span.tool_radius,
        });
    }
    out
}

/// The front's operations that cut into its dowel holes after they are made, one note each
/// (docs/specs/two-sided.md §6): the hole is no longer round, and its pin won't sit firmly.
/// A cut wholly inside a hole only makes it again. Nothing for a setup without dowel holes.
pub fn broken_hole_notes(job: &Job, program: &Program, spans: &[OpSpan]) -> Vec<Note> {
    let holes: Vec<(Point, f64)> = job
        .operations
        .iter()
        .enumerate()
        .filter(|(i, op)| op.is_dowels() && job.cuts(*i))
        .flat_map(|(_, op)| {
            let select = op.select().resolved(&job.drawing);
            job.drawing
                .shapes
                .iter()
                .filter(move |s| !s.construction && select.may_pick(s))
                .filter_map(|s| match s.geometry {
                    Geometry::Circle { center, radius } => Some(((center[0], center[1]), radius)),
                    _ => None,
                })
                .collect::<Vec<_>>()
        })
        .collect();
    let Some(made) = spans
        .iter()
        .filter(|s| {
            job.operations
                .get(s.operation)
                .is_some_and(Operation::is_dowels)
        })
        .map(|s| s.moves.end)
        .min()
    else {
        return Vec::new();
    };
    let after: Vec<OpSpan> = spans
        .iter()
        .filter(|s| s.moves.start >= made)
        .cloned()
        .collect();
    let mut broken: Vec<usize> = Vec::new();
    for cut in cuts(job, program, &after) {
        if broken.contains(&cut.operation) {
            continue;
        }
        let breaks = holes.iter().any(|&(c, r)| {
            let touches = cut.distance(c) < r + cut.radius - CHAIN_TOL_MM;
            let inside = |p: Point| (p.0 - c.0).hypot(p.1 - c.1) + cut.radius <= r + CHAIN_TOL_MM;
            let wholly = match cut.arc {
                None => inside(cut.from) && inside(cut.to),
                Some((ac, ar, _)) => {
                    (ac.0 - c.0).hypot(ac.1 - c.1) + ar + cut.radius <= r + CHAIN_TOL_MM
                }
            };
            touches && !wholly
        });
        if breaks {
            broken.push(cut.operation);
        }
    }
    broken.sort_unstable();
    broken
        .into_iter()
        .map(|i| Note::at(i, Message::new("pipeline-dowels-broken")))
        .collect()
}

/// Add dowel holes for the back setup `back`: two circles on layer "dowels" and a holes
/// operation, named in `lang`, first in its front's first part (after a surface pass),
/// through the board and
/// `depth` into the spoilboard (a drill of the size when the project has one, else a bore).
/// Refused when the front has dowel holes already. Returns the operation's index.
pub fn add_dowels(
    job: &Job,
    back: u64,
    dowels: Dowels,
    lang: Language,
) -> Result<(Job, usize), PipelineError> {
    let centres = dowel_centres(job, back, dowels.diameter)?;
    let front = job
        .setup(back)
        .and_then(|s| s.back_of)
        .map_or(0, |b| b.setup);
    let in_front = |op: &Operation| job.setup_of(op).is_some_and(|s| s.id == front);
    if let Some(i) = job
        .operations
        .iter()
        .position(|op| op.is_dowels() && in_front(op))
    {
        return Err(refused(
            Message::new("pipeline-dowels-there").arg("number", i + 1),
        ));
    }
    let Some(part) = job.parts.iter().find(|p| p.setup == front).map(|p| p.id) else {
        return Err(refused(Message::new("pipeline-no-setup").arg("id", front)));
    };
    let mut out = job.clone();
    let mut ids = Vec::new();
    for (x, y) in centres {
        let geometry = Geometry::Circle {
            center: [x, y],
            radius: dowels.diameter / 2.0,
        };
        let (next, id) = add_shape(&out, DOWEL_LAYER, geometry)?;
        out = next;
        ids.push(id);
    }
    // Made as a new holes operation in the front's part, so it takes the project's drill
    // of the size, or a bore, with their feeds.
    let mut there = out.clone();
    there.active_setup = Some(front);
    there.active_part = Some(part);
    let contours = crate::load_drawing(&there)?.contours;
    let request = NewRequest {
        kind: OpKind::Holes,
        select: Selector {
            shapes: Some(ids),
            ..Selector::default()
        },
        tool: None,
    };
    let made = new_operations(&there, &request, &contours, None, lang);
    let Some(mut op) = made.ops.into_iter().next().map(|n| n.op) else {
        return Err(refused(Message::new("pipeline-dowels-no-shapes")));
    };
    if let Operation::Holes(h) = &mut op {
        h.dowels = true;
        h.through = Some(dowels.depth);
        h.name = Message::new("pipeline-name-dowels").render(lang, Units::Mm);
        h.part = part;
    }
    // First in the part, after a surface pass that faces the board.
    let at = out
        .operations
        .iter()
        .position(|e| e.part() == part && !matches!(e, Operation::Surface(_)))
        .unwrap_or_else(|| out.insert_index(part));
    out.operations.insert(at, op);
    if dowels.zero {
        let (x, y) = centres[0];
        if let Some(setup) = out.setups.iter_mut().find(|s| s.id == front) {
            setup.work_zero = WorkZero::Point { x, y };
        }
    }
    out.follow_thickness();
    match out.validate().into_iter().next() {
        Some(problem) => Err(refused(problem.text_message)),
        None => Ok((out, at)),
    }
}
