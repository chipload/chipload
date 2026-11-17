// SPDX-License-Identifier: GPL-3.0-or-later
//! Nesting a setup's instances on its board (docs/specs/nesting.md): each part planned
//! once, its footprints at each allowed turn, placed bottom-left, and the result checked as
//! any project is before it is returned. Only placements change, and what doesn't fit is
//! switched off (decision 10).

use chipload_core::Point;
use chipload_core::tolerances::PLACEMENT_ROUNDING_MM;
use chipload_i18n::{Digits, Message, Value};
use chipload_job::{
    Instance, Job, Mirror, Nest, Operation, StockCorner, StockOrigin, Turns, WorkZero,
};
use chipload_toolpath::nest::{Footprint, Item, Sheet, Sweep, cell_size, fill};
use chipload_toolpath::plan::OpSpan;
use chipload_toolpath::{Cancel, plan_job};

use crate::{PipelineError, load_drawing};

/// The gap between what two instances cut, mm, until the setup has been nested
/// (decision 4).
pub const NEST_GAP_MM: f64 = 3.0;

/// The room left at the board's edge, mm, until the setup has been nested (decision 4).
pub const NEST_MARGIN_MM: f64 = 10.0;

/// What to nest and how (docs/specs/nesting.md §7, §8). Anything left None is what the
/// project has: the setup's kept settings, else the defaults.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct NestRequest {
    /// The setup; None: the active one.
    pub setup: Option<u64>,
    /// The board's size, mm: needed when the board follows the drawing (§5).
    pub board: Option<(f64, f64)>,
    pub gap: Option<f64>,
    pub margin: Option<f64>,
    /// Every part's turns, kept in the file.
    pub turns: Option<Turns>,
    /// One part's turns, after `turns`.
    pub part_turns: Vec<(u64, Turns)>,
    /// How many times a part is cut (§6): instances added or removed first.
    pub counts: Vec<(u64, u32)>,
}

/// Where one instance went.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NestPut {
    pub part: u64,
    pub instance: u32,
    pub angle: f64,
    pub at: [f64; 2],
}

/// A board side longer than the machine travels (§5): only `travel` of it is used.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TravelLimit {
    pub axis: char,
    pub board: f64,
    pub travel: f64,
}

/// A nest: the project with its placements, checked; what went where; what didn't fit and
/// was switched off; how much of the board's length it uses.
#[derive(Debug, Clone)]
pub struct NestResult {
    pub job: Job,
    pub setup: u64,
    pub placed: Vec<NestPut>,
    pub unplaced: Vec<(u64, u32)>,
    /// How far along the usable area's longer side it reaches, and that side's length, mm.
    pub used: f64,
    pub length: f64,
    /// The board followed the drawing and was given a fixed size (§5).
    pub board_fixed: bool,
    pub travel: Vec<TravelLimit>,
}

impl NestResult {
    /// How many instances there were to place.
    pub fn count(&self) -> usize {
        self.placed.len() + self.unplaced.len()
    }

    /// "8 of 8 placed, the board used to 742 mm of 1220 mm (61 %)".
    pub fn summary(&self) -> Message {
        let share = if self.length > 0.0 {
            (100.0 * self.used / self.length).round()
        } else {
            0.0
        };
        Message::new("pipeline-nest-done")
            .arg("placed", self.placed.len())
            .arg("count", self.count())
            .arg("used", Value::Length(self.used, Digits::Fixed(0)))
            .arg("length", Value::Length(self.length, Digits::Fixed(0)))
            .arg("share", share as u64)
    }

    /// The notes beside the summary: the board made a fixed size, the travel's limits,
    /// each instance switched off.
    pub fn notes(&self) -> Vec<Message> {
        let mut notes = Vec::new();
        if self.board_fixed {
            let (width, height) = self
                .job
                .setup(self.setup)
                .map_or((0.0, 0.0), |s| (s.stock.x, s.stock.y));
            notes.push(
                Message::new("pipeline-nest-board-fixed")
                    .arg("width", Value::LengthNumber(width, Digits::Shortest))
                    .arg("height", Value::Length(height, Digits::Shortest)),
            );
        }
        for limit in &self.travel {
            notes.push(
                Message::new("pipeline-nest-travel")
                    .arg("axis", limit.axis.to_string())
                    .arg("board", Value::Length(limit.board, Digits::Shortest))
                    .arg("travel", Value::Length(limit.travel, Digits::Shortest)),
            );
        }
        for &(part, instance) in &self.unplaced {
            let name = self.job.part(part).map_or("", |p| p.name.as_str());
            notes.push(
                Message::new("pipeline-nest-unplaced")
                    .arg("part", name)
                    .arg("instance", instance),
            );
        }
        notes
    }
}

impl NestResult {
    /// The project as a preview shows it (docs/specs/nesting.md §7): what didn't fit turned
    /// back on and parked in a row right of the board, unturned, its box [`PARK_GAP_MM`]
    /// from the board and from the one before. Only for showing: never planned or saved.
    pub fn preview_job(&self) -> Job {
        let mut job = self.job.clone();
        let one = job.for_setup(self.setup);
        let ((_, y0), (x1, _)) = one.stock_bounds();
        let mut x = x1 + PARK_GAP_MM;
        for &(part, number) in &self.unplaced {
            let Some(((bx0, by0), (bx1, _))) = job.part_box(part) else {
                continue;
            };
            let at = [x - bx0, y0 - by0];
            x += bx1 - bx0 + PARK_GAP_MM;
            let Some(p) = job.parts.iter_mut().find(|p| p.id == part) else {
                continue;
            };
            if number == 1 {
                p.enabled = true;
                p.angle = 0.0;
                p.at = at;
            } else if let Some(i) = p.instances.iter_mut().find(|i| i.id == number) {
                i.enabled = true;
                i.angle = 0.0;
                i.at = at;
            }
        }
        job
    }
}

/// How far apart parked instances lie in a preview, and from the board, mm.
pub const PARK_GAP_MM: f64 = 20.0;

fn refused(id: &'static str) -> PipelineError {
    PipelineError::Job(Message::new(id))
}

/// Nest setup `request.setup` (docs/specs/nesting.md §4): its enabled parts' instances
/// placed on its board, what doesn't fit switched off, the result planned and checked in
/// full before it is returned. Refused for a setup in pieces or with a back, one with no
/// part on, or a board that follows the drawing without a size given.
pub fn nest_setup(
    job: &Job,
    request: &NestRequest,
    cancel: &Cancel,
) -> Result<NestResult, PipelineError> {
    let id = request
        .setup
        .or_else(|| job.active_setup_id())
        .ok_or_else(|| refused("pipeline-no-setup-cut"))?;
    let setup = job
        .setup(id)
        .ok_or_else(|| PipelineError::Job(Message::new("pipeline-no-setup").arg("id", id)))?;
    if setup.pieces.is_some() {
        return Err(refused("pipeline-nest-pieces"));
    }
    if setup.back_of.is_some() || job.back_of(id).is_some() {
        return Err(refused("pipeline-nest-two-sided"));
    }
    let mut job = job.clone();
    let gap = request
        .gap
        .or(setup.nest.map(|n| n.gap))
        .unwrap_or(NEST_GAP_MM);
    let margin = request
        .margin
        .or(setup.nest.map(|n| n.margin))
        .unwrap_or(NEST_MARGIN_MM);
    let index = job.setups.iter().position(|s| s.id == id).unwrap_or(0);
    job.setups[index].nest = Some(Nest { gap, margin });
    let board_fixed = fix_board(&mut job, index, request.board)?;
    set_turns(&mut job, id, request)?;
    set_counts(&mut job, id, &request.counts)?;
    let mut job = checked(job)?;
    let parts: Vec<u64> = job
        .parts
        .iter()
        .filter(|p| p.setup == id && p.enabled)
        .map(|p| p.id)
        .collect();
    if parts.is_empty() {
        return Err(refused("pipeline-nest-no-parts"));
    }
    let one = job.for_setup(id);
    let (low, high, travel) = usable_area(&one, margin);
    let size = (high.0 - low.0, high.1 - low.1);
    if !(size.0 > 0.0 && size.1 > 0.0) {
        return Err(PipelineError::Job(
            Message::new("pipeline-nest-no-room")
                .arg("margin", Value::Length(margin, Digits::Shortest)),
        ));
    }
    let cell = cell_size(size.0, size.1);
    let drawing = load_drawing(&one)?;
    let mut keep_out = Vec::new();
    let mut items: Vec<(u64, u32, Item)> = Vec::new();
    let mut sheet_parts = Vec::new();
    for &part in &parts {
        cancel.check()?;
        let planned = plan_part(&one, part, &drawing.contours)?;
        keep_out.extend(planned.dowels.clone());
        sheet_parts.push((part, planned));
    }
    let sheet = Sheet::new(low, high, cell, &keep_out).ok_or_else(|| {
        PipelineError::Job(
            Message::new("pipeline-nest-no-room")
                .arg("margin", Value::Length(margin, Digits::Shortest)),
        )
    })?;
    for (part, planned) in &sheet_parts {
        let Some(p) = one.part(*part) else { continue };
        let angles = p.turns.unwrap_or_default().angles();
        // One item per mirror, cloned for each instance that lies so.
        let mut made: Vec<(Option<Mirror>, Item)> = Vec::new();
        for (number, mirror) in instances_of(p) {
            let item = match made.iter().find(|(m, _)| *m == mirror) {
                Some((_, item)) => item.clone(),
                None => {
                    let base = planned
                        .sweeps
                        .iter()
                        .find(|(m, _)| *m == mirror)
                        .map(|(_, s)| s)
                        .ok_or_else(|| refused("pipeline-part-cuts-nothing"))?;
                    let item = lies(&sheet, base, planned.centre, &angles, gap)?;
                    made.push((mirror, item.clone()));
                    item
                }
            };
            items.push((*part, number, item));
        }
    }
    let plain: Vec<Item> = items.iter().map(|(_, _, item)| item.clone()).collect();
    let layout = fill(&sheet, &plain, cancel)?;
    let mut placed = Vec::new();
    for put in &layout.placed {
        let Some((part, number, item)) = items.get(put.item) else {
            continue;
        };
        let Some(lie) = item.lies.get(put.lie) else {
            continue;
        };
        let f = &lie.footprint;
        let at = [tidy(put.low.0 - f.low.0), tidy(put.low.1 - f.low.1)];
        placed.push(NestPut {
            part: *part,
            instance: *number,
            angle: lie.angle,
            at,
        });
    }
    let mut unplaced: Vec<(u64, u32)> = layout
        .unplaced
        .iter()
        .filter_map(|&k| items.get(k).map(|(part, number, _)| (*part, *number)))
        .collect();
    keep_parts_on(&one, &mut placed, &mut unplaced);
    for put in &placed {
        place(&mut job, put.part, put.instance, put.angle, put.at);
    }
    for &(part, number) in &unplaced {
        switch_off(&mut job, part, number);
    }
    let job = checked(job)?;
    if !placed.is_empty() {
        verify(&job, id)?;
    }
    let length = if size.0 >= size.1 { size.0 } else { size.1 };
    Ok(NestResult {
        job,
        setup: id,
        placed,
        unplaced,
        used: layout.used,
        length,
        board_fixed,
        travel,
    })
}

fn checked(job: Job) -> Result<Job, PipelineError> {
    match job.validate().into_iter().next() {
        Some(problem) => Err(PipelineError::Job(problem.text_message)),
        None => Ok(job),
    }
}

/// A board that follows the drawing (fitted to it or centred on it) gets the fixed size
/// asked for, its bottom-left corner at the drawing's origin (§5): the drawing never moves,
/// and every instance is placed anew. A fixed board takes a size asked for. Whether it
/// was following the drawing.
fn fix_board(
    job: &mut Job,
    index: usize,
    board: Option<(f64, f64)>,
) -> Result<bool, PipelineError> {
    let Some(setup) = job.setups.get_mut(index) else {
        return Ok(false);
    };
    let follows = setup.stock.fit.is_some() || setup.stock.origin == StockOrigin::Drawing;
    match (follows, board) {
        (true, None) => Err(refused("pipeline-nest-needs-board")),
        (true, Some((x, y))) => {
            setup.stock.fit = None;
            setup.stock.origin = StockOrigin::BottomLeft;
            setup.stock.x = x;
            setup.stock.y = y;
            Ok(true)
        }
        (false, Some((x, y))) => {
            setup.stock.x = x;
            setup.stock.y = y;
            Ok(false)
        }
        (false, None) => Ok(false),
    }
}

/// The turns asked for, kept on the setup's parts.
fn set_turns(job: &mut Job, setup: u64, request: &NestRequest) -> Result<(), PipelineError> {
    if let Some(turns) = request.turns {
        for part in job.parts.iter_mut().filter(|p| p.setup == setup) {
            part.turns = Some(turns);
        }
    }
    for &(id, turns) in &request.part_turns {
        let part =
            job.parts.iter_mut().find(|p| p.id == id).ok_or_else(|| {
                PipelineError::Job(Message::new("pipeline-no-part").arg("id", id))
            })?;
        part.turns = Some(turns);
    }
    Ok(())
}

/// Each part asked for cut `count` times (§6): unmirrored instances added, or the
/// highest-numbered enabled ones removed; never instance 1. Disabled instances stay.
fn set_counts(job: &mut Job, setup: u64, counts: &[(u64, u32)]) -> Result<(), PipelineError> {
    for &(id, count) in counts {
        let part = job
            .parts
            .iter_mut()
            .find(|p| p.id == id && p.setup == setup)
            .ok_or_else(|| PipelineError::Job(Message::new("pipeline-no-part").arg("id", id)))?;
        if count == 0 {
            return Err(PipelineError::Job(
                Message::new("pipeline-nest-count").arg("part", part.name.as_str()),
            ));
        }
        while part.count() < count {
            let next = part.next_instance_id();
            part.instances.push(Instance {
                id: next,
                enabled: true,
                mirror: None,
                angle: 0.0,
                at: [0.0, 0.0],
            });
        }
        while part.count() > count {
            let Some(last) = part.instances.iter().rposition(|i| i.enabled) else {
                break;
            };
            part.instances.remove(last);
        }
    }
    Ok(())
}

/// The usable area (§5): the board less the margin, within the machine's travel measured
/// from the side work zero is on, and each side the travel shortened.
fn usable_area(one: &Job, margin: f64) -> (Point, Point, Vec<TravelLimit>) {
    let ((x0, y0), (x1, y1)) = one.stock_bounds();
    let (mut low, mut high) = ((x0 + margin, y0 + margin), (x1 - margin, y1 - margin));
    let mut limits = Vec::new();
    if let Some(travel) = &one.machine.travel {
        let (from_right, from_top) = match one.work_zero() {
            WorkZero::Stock { corner } => (
                matches!(corner, StockCorner::BottomRight | StockCorner::TopRight),
                matches!(corner, StockCorner::TopLeft | StockCorner::TopRight),
            ),
            _ => (false, false),
        };
        for (axis, lo, hi, side, far, reach) in [
            ('X', &mut low.0, &mut high.0, (x0, x1), from_right, travel.x),
            ('Y', &mut low.1, &mut high.1, (y0, y1), from_top, travel.y),
        ] {
            let board = side.1 - side.0;
            if board <= reach {
                continue;
            }
            limits.push(TravelLimit {
                axis,
                board,
                travel: reach,
            });
            if far {
                *lo = lo.max(side.1 - reach);
            } else {
                *hi = hi.min(side.0 + reach);
            }
        }
    }
    (low, high, limits)
}

/// A part planned alone, as drawn, once for each mirror its instances lie in: its sweep
/// per mirror, its centre, and the dowel holes it drills (keep-outs, never placed).
struct PlannedPart {
    centre: Point,
    sweeps: Vec<(Option<Mirror>, Sweep)>,
    dowels: Vec<Sweep>,
}

/// The mirrors part `part`'s instances lie in, instance 1 first (unmirrored: the part's own
/// mirror is its definition's), each with its number.
fn instances_of(part: &chipload_job::Part) -> Vec<(u32, Option<Mirror>)> {
    let mut out = vec![(1, None)];
    out.extend(
        part.instances
            .iter()
            .filter(|i| i.enabled)
            .map(|i| (i.id, i.mirror)),
    );
    out
}

/// Plan part `part` alone (§3): instance 1 where drawn, and one instance for each other
/// mirror its instances need, set far to the right so nothing meets; each one's sweep moved
/// back. Board surfaces and dowel holes are no part of a footprint.
fn plan_part(
    one: &Job,
    part: u64,
    contours: &[chipload_core::chain::Contour],
) -> Result<PlannedPart, PipelineError> {
    let mut alone = one.clone();
    let centre = alone.part_centre(part).unwrap_or((0.0, 0.0));
    let width = alone
        .part_box(part)
        .map_or(0.0, |((x0, _), (x1, _))| x1 - x0);
    let pitch = 2.0 * width + 1000.0;
    let mut mirrors: Vec<Option<Mirror>> = vec![None];
    for p in &mut alone.parts {
        p.enabled = p.id == part;
        if p.id != part {
            continue;
        }
        for (_, mirror) in instances_of(p) {
            if !mirrors.contains(&mirror) {
                mirrors.push(mirror);
            }
        }
        p.angle = 0.0;
        p.at = [0.0, 0.0];
        p.instances = mirrors
            .iter()
            .enumerate()
            .skip(1)
            .map(|(k, &mirror)| Instance {
                id: k as u32 + 1,
                enabled: true,
                mirror,
                angle: 0.0,
                at: [pitch * k as f64, 0.0],
            })
            .collect();
    }
    let plan = plan_job(&alone, contours)?;
    let operation = |s: &OpSpan| alone.operations.get(s.operation);
    let cut = |s: &OpSpan| operation(s).is_some_and(|op| !op.is_board_surface() && !op.is_dowels());
    let mut sweeps = Vec::new();
    for (k, &mirror) in mirrors.iter().enumerate() {
        let sweep = Sweep::from_moves(&plan.program.moves, &plan.op_spans, k as u32 + 1, cut)
            .moved((-pitch * k as f64, 0.0));
        if sweep.capsules.is_empty() {
            return Err(refused("pipeline-part-cuts-nothing"));
        }
        sweeps.push((mirror, sweep));
    }
    let dowels = Sweep::from_moves(&plan.program.moves, &plan.op_spans, 1, |s| {
        operation(s).is_some_and(Operation::is_dowels)
    });
    Ok(PlannedPart {
        centre,
        sweeps,
        dowels: if dowels.capsules.is_empty() {
            Vec::new()
        } else {
            vec![dowels]
        },
    })
}

/// An item: `base` turned about `centre` by each angle, its footprint on the sheet.
fn lies(
    sheet: &Sheet,
    base: &Sweep,
    centre: Point,
    angles: &[f64],
    gap: f64,
) -> Result<Item, PipelineError> {
    let lies = angles
        .iter()
        .map(|&angle| {
            let footprint = Footprint::new(&base.turned(angle, centre), sheet.cell)
                .ok_or_else(|| refused("pipeline-part-cuts-nothing"))?;
            Ok(sheet.lie(angle, footprint, gap))
        })
        .collect::<Result<Vec<_>, PipelineError>>()?;
    Ok(Item { lies })
}

/// Instance `number` of part `part` placed: turned by `angle`, moved by `at`; instance 1
/// is the part's own placement.
fn place(job: &mut Job, part: u64, number: u32, angle: f64, at: [f64; 2]) {
    let Some(p) = job.parts.iter_mut().find(|p| p.id == part) else {
        return;
    };
    if number == 1 {
        p.angle = angle;
        p.at = at;
    } else if let Some(i) = p.instances.iter_mut().find(|i| i.id == number) {
        i.angle = angle;
        i.at = at;
    }
}

/// A part whose instance 1 didn't fit while an unmirrored instance of it did: instance 1
/// takes that place and the other is switched off instead, so the part stays on and is
/// cut as many times (switching instance 1 off switches off the whole part).
fn keep_parts_on(one: &Job, placed: &mut [NestPut], unplaced: &mut [(u64, u32)]) {
    for left in unplaced.iter_mut() {
        if left.1 != 1 {
            continue;
        }
        let part = left.0;
        let unmirrored = |n: u32| {
            one.part(part)
                .and_then(|p| p.instances.iter().find(|i| i.id == n))
                .is_some_and(|i| i.mirror.is_none())
        };
        if let Some(put) = placed
            .iter_mut()
            .filter(|p| p.part == part && unmirrored(p.instance))
            .max_by_key(|p| p.instance)
        {
            left.1 = put.instance;
            put.instance = 1;
        }
    }
    unplaced.sort_unstable();
}

/// What didn't fit switched off (decision 10): the instance, or for instance 1 the whole
/// part. It keeps its place.
fn switch_off(job: &mut Job, part: u64, number: u32) {
    let Some(p) = job.parts.iter_mut().find(|p| p.id == part) else {
        return;
    };
    if number == 1 {
        p.enabled = false;
    } else if let Some(i) = p.instances.iter_mut().find(|i| i.id == number) {
        i.enabled = false;
    }
}

/// The nest planned and checked in full, as a run of the setup is (§4.6): a nest that
/// fails here is a bug, and nothing is applied.
fn verify(job: &Job, setup: u64) -> Result<(), PipelineError> {
    crate::run_setups(job, Some(setup))
        .map(|_| ())
        .map_err(|e| {
            PipelineError::Job(
                Message::new("pipeline-nest-not-verified").arg("reason", e.message()),
            )
        })
}

/// A placement without the last bits a turn leaves, never a negative zero.
fn tidy(mm: f64) -> f64 {
    let step = PLACEMENT_ROUNDING_MM;
    (mm / step).round() * step + 0.0
}
