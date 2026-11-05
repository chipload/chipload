// SPDX-License-Identifier: GPL-3.0-or-later
//! Dowel holes for turning a board over (docs/specs/two-sided.md §6): two drawn circles on
//! the line the board turns about, one at each end in the margin beyond both sides'
//! shapes, cut first with the front, through the board and into the spoilboard for the
//! pins. On the turned board each hole lands on itself, so a pin through both locates it.

use chipload_core::Point;
use chipload_core::tolerances::CHAIN_TOL_MM;
use chipload_i18n::{Digits, Language, Message, Units, Value};
use chipload_job::{Geometry, Job, Operation, Selector, Turn, WorkZero};

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
/// in the margin between both sides' shapes and the board's edge, the lower (left-right)
/// or the left (top-bottom) first. Refused when the margin can't hold the hole.
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
    let Some(((px0, py0), (px1, py1))) = one.board_extents() else {
        return Err(refused(Message::new("pipeline-dowels-no-shapes")));
    };
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
    // The default 10 mm margin holds the default 8 mm pins exactly.
    if narrowest + CHAIN_TOL_MM < needed {
        let length = |v: f64| Value::Length(v, Digits::Fixed(1));
        return Err(refused(
            Message::new("pipeline-dowels-no-room")
                .arg("margin", length(narrowest.max(0.0)))
                .arg("needed", length(needed)),
        ));
    }
    Ok(centres)
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
