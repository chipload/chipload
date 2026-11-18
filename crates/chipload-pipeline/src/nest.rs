// SPDX-License-Identifier: GPL-3.0-or-later
//! Nesting a setup's instances on its board (docs/specs/nesting.md): each part planned
//! once, its footprints at each allowed turn, placed bottom-left, and the result checked as
//! any project is before it is returned. Only placements change, and what doesn't fit is
//! switched off (decision 10).

use chipload_core::Point;
use chipload_core::tolerances::{ANGLE_TOL_RAD, PLACEMENT_ROUNDING_MM};
use chipload_i18n::{Digits, Message, Value};
use chipload_job::{
    Instance, Job, Mirror, Nest, NestGoal, Operation, StockCorner, StockOrigin, Turns, WorkZero,
};
use chipload_toolpath::nest::{Footprint, Goal, Item, Now, Sheet, Sweep, cell_size, fill};
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
    /// What doesn't fit goes onto more boards instead of being switched off
    /// (docs/specs/parts-and-setups.md §9).
    pub overflow: bool,
    /// What the nest makes as small as it can (docs/specs/nesting-search.md §2), kept in
    /// the file.
    pub goal: Option<NestGoal>,
}

/// A board that took what didn't fit on the one before (docs/specs/parts-and-setups.md §9).
#[derive(Debug, Clone, PartialEq)]
pub struct OverflowBoard {
    pub setup: u64,
    /// Made for it, like the board nested first.
    pub made: bool,
    pub placed: Vec<NestPut>,
}

/// The most boards one nest adds or fills after the first.
pub const MAX_OVERFLOW_BOARDS: usize = 50;

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
    /// The box round what was placed on this board from the usable area's lower-left
    /// corner, and the usable area's size, mm (docs/specs/nesting-search.md §7).
    pub extent: (f64, f64),
    pub usable: (f64, f64),
    pub goal: NestGoal,
    /// The board followed the drawing and was given a fixed size (§5).
    pub board_fixed: bool,
    pub travel: Vec<TravelLimit>,
    /// The boards that took what didn't fit, in order; empty without overflow.
    pub overflow: Vec<OverflowBoard>,
    /// Nested again and nothing better found (docs/specs/nesting-search.md §5): the board
    /// is as it was, but for what doesn't fit there, switched off.
    pub unchanged: bool,
}

impl NestResult {
    /// How many instances there were to place.
    pub fn count(&self) -> usize {
        self.placed_count() + self.unplaced.len()
    }

    /// How many were placed, on every board.
    pub fn placed_count(&self) -> usize {
        self.placed.len() + self.overflow.iter().map(|b| b.placed.len()).sum::<usize>()
    }

    /// "8 of 8 placed, in 742 × 380 mm of the board's 1200 × 590 mm (40 %)": the box's
    /// share of the usable area; or that nothing better was found.
    pub fn summary(&self) -> Message {
        if self.unchanged {
            return Message::new("pipeline-nest-no-better");
        }
        let area = self.usable.0 * self.usable.1;
        let share = if area > 0.0 {
            (100.0 * self.extent.0 * self.extent.1 / area).round()
        } else {
            0.0
        };
        Message::new("pipeline-nest-done")
            .arg("placed", self.placed_count())
            .arg("count", self.count())
            .arg(
                "width",
                Value::LengthNumber(self.extent.0, Digits::Fixed(0)),
            )
            .arg("height", Value::Length(self.extent.1, Digits::Fixed(0)))
            .arg(
                "board_width",
                Value::LengthNumber(self.usable.0, Digits::Fixed(0)),
            )
            .arg(
                "board_height",
                Value::Length(self.usable.1, Digits::Fixed(0)),
            )
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
        for board in &self.overflow {
            let name = self.job.setup(board.setup).map_or("", |s| s.name.as_str());
            let id = if board.made {
                "pipeline-nest-overflow-made"
            } else {
                "pipeline-nest-overflow"
            };
            notes.push(
                Message::new(id)
                    .arg("count", board.placed.len())
                    .arg("board", name),
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
        let ((_, _), (x1, _)) = self.job.for_setup(self.setup).stock_bounds();
        self.preview_job_from(x1 + PARK_GAP_MM)
    }

    /// [`NestResult::preview_job`], parking from `x` on: right of the boards shown beside
    /// the first.
    pub fn preview_job_from(&self, x: f64) -> Job {
        let mut job = self.job.clone();
        let one = job.for_setup(self.setup);
        let ((_, y0), (_, _)) = one.stock_bounds();
        let mut x = x;
        for &(part, number) in &self.unplaced {
            let Some(((bx0, by0), (bx1, _))) = job.part_box(part) else {
                continue;
            };
            let at = [x - bx0, y0 - by0];
            x += bx1 - bx0 + PARK_GAP_MM;
            if let Some(i) = job.instance_mut(part, number) {
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
/// part on, or a board that follows the drawing without a size given. With `overflow`,
/// what doesn't fit goes onto more boards ([`overflow`]).
pub fn nest_setup(
    job: &Job,
    request: &NestRequest,
    cancel: &Cancel,
) -> Result<NestResult, PipelineError> {
    let first = nest_one(job, request, false, cancel)?;
    if !request.overflow || first.unplaced.is_empty() {
        return Ok(first);
    }
    overflow(first, cancel)
}

/// One board nested, with its parts whose stock isn't its own left out with overflow
/// (they go to a board of their stock, §4). A board nested before is searched from as it
/// is, unless `afresh` (a board taking what didn't fit on another).
fn nest_one(
    job: &Job,
    request: &NestRequest,
    afresh: bool,
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
    // Nested before: the board as it is is one of the search's starts, and is kept unless
    // the search does better (nesting-search.md §5).
    // A board that takes what didn't fit on another is nested afresh.
    let again = setup.nest.is_some() && !afresh;
    let mut job = job.clone();
    let gap = request
        .gap
        .or(setup.nest.map(|n| n.gap))
        .unwrap_or(NEST_GAP_MM);
    let margin = request
        .margin
        .or(setup.nest.map(|n| n.margin))
        .unwrap_or(NEST_MARGIN_MM);
    let goal = request
        .goal
        .or(setup.nest.map(|n| n.goal))
        .unwrap_or_default();
    let index = job.setups.iter().position(|s| s.id == id).unwrap_or(0);
    job.setups[index].nest = Some(Nest { gap, margin, goal });
    let board_fixed = fix_board(&mut job, index, request.board)?;
    set_turns(&mut job, id, request)?;
    set_counts(&mut job, id, &request.counts)?;
    let mut job = checked(job)?;
    board_checked(&job, id)?;
    let all: Vec<u64> = job
        .parts
        .iter()
        .filter(|p| p.enabled && job.part_on(p.id, id))
        .map(|p| p.id)
        .collect();
    if all.is_empty() {
        return Err(refused("pipeline-nest-no-parts"));
    }
    // With overflow, a part that asks for another stock goes to a board of it (§9).
    let mismatched = if request.overflow {
        job.stock_mismatches(id)
    } else {
        Vec::new()
    };
    let parts: Vec<u64> = all
        .iter()
        .copied()
        .filter(|p| !mismatched.contains(p))
        .collect();
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
        for (number, mirror) in instances_of(&one, *part) {
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
    let now: Vec<Option<Now>> = items
        .iter()
        .map(|(part, number, item)| lying_now(&one, *part, *number, item))
        .collect();
    let filled = fill(
        &sheet,
        &plain,
        placer_goal(goal),
        again.then_some(&now[..]),
        cancel,
    )?;
    let unchanged = !filled.better;
    let layout = filled.layout;
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
    for &part in &mismatched {
        unplaced.extend(
            one.instances_here(part)
                .into_iter()
                .filter(|i| i.enabled)
                .map(|i| (part, i.id)),
        );
    }
    // In part and number order, as reported.
    unplaced.sort_unstable();
    // Unchanged, the placements stay exactly as they were: nothing is written back.
    if !unchanged {
        for put in &placed {
            place(&mut job, put.part, put.instance, put.angle, put.at);
        }
    }
    for &(part, number) in &unplaced {
        switch_off(&mut job, part, number);
    }
    let job = checked(job)?;
    if !placed.is_empty() {
        verify(&job, id)?;
    }
    Ok(NestResult {
        job,
        setup: id,
        placed,
        unplaced,
        extent: layout.extent,
        usable: size,
        goal,
        board_fixed,
        travel,
        overflow: Vec::new(),
        unchanged,
    })
}

/// The placer's goal for the file's.
fn placer_goal(goal: NestGoal) -> Goal {
    match goal {
        NestGoal::Compact => Goal::Compact,
        NestGoal::Low => Goal::Low,
        NestGoal::Left => Goal::Left,
    }
}

/// Where instance `number` of part `part` lies now, as one of `item`'s lies (the one of its
/// turn) and its footprint's low corner; None when it is off or turned otherwise than its
/// part's turns allow.
fn lying_now(one: &Job, part: u64, number: u32, item: &Item) -> Option<Now> {
    let instance = one
        .instances_here(part)
        .into_iter()
        .find(|i| i.id == number && i.enabled)?;
    let tau = std::f64::consts::TAU;
    let lie = item.lies.iter().position(|lie| {
        let d = (lie.angle - instance.angle).rem_euclid(tau);
        d <= ANGLE_TOL_RAD || tau - d <= ANGLE_TOL_RAD
    })?;
    let f = &item.lies.get(lie)?.footprint;
    Some(Now {
        lie,
        low: (instance.at[0] + f.low.0, instance.at[1] + f.low.1),
    })
}

/// The stock what part `part` is cut from on a board like setup `like`'s: that board's,
/// with the part's own thickness and material where it asks for them (§4).
fn wanted_stock(job: &Job, like: u64, part: u64) -> Option<chipload_job::Stock> {
    let mut stock = job.setup(like)?.stock.clone();
    if let Some(wanted) = job.part(part).and_then(|p| p.stock.as_ref()) {
        if let Some(t) = wanted.thickness {
            stock.thickness = t;
        }
        if let Some(m) = &wanted.material {
            stock.material = Some(m.clone());
        }
    }
    Some(stock)
}

/// The same board for a part: thickness within the depth tolerance, the same material.
fn same_stock(a: &chipload_job::Stock, b: &chipload_job::Stock) -> bool {
    (a.thickness - b.thickness).abs() <= chipload_core::tolerances::DEPTH_TOL_MM
        && a.material.as_ref().map(|m| &m.library_uid)
            == b.material.as_ref().map(|m| &m.library_uid)
}

/// What didn't fit on the first board, onto more (docs/specs/parts-and-setups.md §9): by
/// the stock each asks for, onto the next setup with that stock and a size of its own (not
/// two-sided, not in pieces, not tried yet), else onto a new one made like the first; each
/// nested in turn, what doesn't fit there going on. Instances keep their numbers. What
/// doesn't fit even an empty board goes back to the first board, switched off.
fn overflow(first: NestResult, cancel: &Cancel) -> Result<NestResult, PipelineError> {
    let start = first.setup;
    let mut job = first.job.clone();
    let nest = job.setup(start).and_then(|s| s.nest);
    let mut left = first.unplaced.clone();
    let mut tried = vec![start];
    let mut boards: Vec<OverflowBoard> = Vec::new();
    let mut too_large: Vec<(u64, u32)> = Vec::new();
    while let Some(&(lead, _)) = left.first() {
        cancel.check()?;
        if boards.len() >= MAX_OVERFLOW_BOARDS {
            break;
        }
        let wanted = wanted_stock(&job, start, lead).ok_or_else(|| {
            PipelineError::Job(Message::new("pipeline-no-setup").arg("id", start))
        })?;
        let group: Vec<(u64, u32)> = left
            .iter()
            .copied()
            .filter(|(p, _)| wanted_stock(&job, start, *p).is_some_and(|s| same_stock(&s, &wanted)))
            .collect();
        left.retain(|i| !group.contains(i));
        let existing = job
            .setups
            .iter()
            .find(|s| {
                !tried.contains(&s.id)
                    && s.back_of.is_none()
                    && s.pieces.is_none()
                    && job.back_of(s.id).is_none()
                    && s.stock.fit.is_none()
                    && s.stock.origin != StockOrigin::Drawing
                    && same_stock(&s.stock, &wanted)
            })
            .map(|s| s.id);
        let (target, made) = match existing {
            Some(id) => (id, false),
            None => (make_board_like(&mut job, start, wanted)?, true),
        };
        tried.push(target);
        for &(part, number) in &group {
            move_instance_to(&mut job, part, number, target);
        }
        // Cuts through the stock go through the board they are on now.
        job.follow_thickness();
        let request = NestRequest {
            setup: Some(target),
            gap: nest.map(|n| n.gap),
            margin: nest.map(|n| n.margin),
            goal: nest.map(|n| n.goal),
            overflow: true,
            ..NestRequest::default()
        };
        let next = nest_one(&job, &request, true, cancel)?;
        let took = next
            .placed
            .iter()
            .any(|p| group.contains(&(p.part, p.instance)));
        if made && !took {
            // Larger than an empty board: back where it was, switched off.
            for &(part, number) in &group {
                move_instance_to(&mut job, part, number, start);
                switch_off(&mut job, part, number);
            }
            job.setups.retain(|s| s.id != target);
            job.follow_thickness();
            too_large.extend(group);
            continue;
        }
        job = next.job;
        left.extend(next.unplaced.iter().copied());
        boards.push(OverflowBoard {
            setup: target,
            made,
            placed: next.placed,
        });
    }
    let mut unplaced = too_large;
    unplaced.extend(left);
    unplaced.sort_unstable();
    let job = checked(job)?;
    Ok(NestResult {
        job,
        unplaced,
        unchanged: first.unchanged && boards.is_empty(),
        overflow: boards,
        ..first
    })
}

/// A new setup like `like`: its board (fixed by the nest) with `stock`, its work zero and
/// nest settings; named as a new setup is. Returns its id.
fn make_board_like(
    job: &mut Job,
    like: u64,
    stock: chipload_job::Stock,
) -> Result<u64, PipelineError> {
    let model = job
        .setup(like)
        .cloned()
        .ok_or_else(|| PipelineError::Job(Message::new("pipeline-no-setup").arg("id", like)))?;
    let id = job.setups.iter().map(|s| s.id).max().unwrap_or(0) + 1;
    // Boards the user asked for: listed as setups from now on.
    job.setups_shown = true;
    job.setups.push(chipload_job::Setup {
        id,
        name: format!("Setup {}", job.setups.len() + 1),
        enabled: true,
        stock,
        work_zero: model.work_zero,
        back_of: None,
        pieces: None,
        nest: model.nest,
        instances: Vec::new(),
        placed: None,
    });
    Ok(id)
}

/// Instance `number` of part `part` onto setup `to`, on again, keeping its number and place.
fn move_instance_to(job: &mut Job, part: u64, number: u32, to: u64) {
    let mut moved = None;
    for s in &mut job.setups {
        if let Some(k) = s
            .instances
            .iter()
            .position(|i| i.part == part && i.id == number)
        {
            moved = Some(s.instances.remove(k));
        }
    }
    if let (Some(mut instance), Some(s)) = (moved, job.setup_mut(to)) {
        instance.enabled = true;
        s.instances.push(instance);
    }
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

/// The tool diameter by which a board surface on setup `setup` (one of an enabled part on
/// it, itself on) runs past the board's ends along its longer side: the facing goes from
/// beside the board to beside it (docs/specs/surfacing.md). None without one.
pub fn surface_overhang(job: &Job, setup: u64) -> Option<f64> {
    job.operations
        .iter()
        .filter(|op| op.is_board_surface() && op.enabled())
        .filter(|op| {
            job.part(op.part())
                .is_some_and(|p| p.enabled && job.part_on(p.id, setup))
        })
        .filter_map(|op| job.tool(op.tool()).map(|t| t.diameter))
        .reduce(f64::max)
}

/// The board the Nest window starts from for setup `setup` when its board follows the
/// drawing: the machine's travel, less a board surface's overhang along the longer side so
/// the facing stays within the travel; without a travel, the board's size now.
pub fn default_board(job: &Job, setup: u64) -> (f64, f64) {
    let Some(travel) = &job.machine.travel else {
        return job.for_setup(setup).stock_size();
    };
    let (x, y) = (travel.x, travel.y);
    let Some(d) = surface_overhang(job, setup) else {
        return (x, y);
    };
    // The facing runs along the longer side; a square along X.
    if x >= y {
        ((x - d).max(0.0), y)
    } else {
        (x, (y - d).max(0.0))
    }
}

/// The board itself checked before anything is placed (docs/specs/nesting.md §5): a board
/// surface that would run past the machine's travel is refused by name; and the setup's
/// board work (surfaces, dowel holes) alone must pass the checks, so that a failure after
/// nesting is the nest's.
fn board_checked(job: &Job, setup: u64) -> Result<(), PipelineError> {
    let one = job.for_setup(setup);
    if let (Some(d), Some(travel)) = (surface_overhang(&one, setup), &one.machine.travel) {
        let (x, y) = one.stock_size();
        let (axis, reach, limit) = if x >= y {
            ('X', x + d, travel.x)
        } else {
            ('Y', y + d, travel.y)
        };
        if reach > limit + PLACEMENT_ROUNDING_MM {
            return Err(PipelineError::Job(
                Message::new("pipeline-nest-surface-travel")
                    .arg("reach", Value::Length(reach, Digits::Shortest))
                    .arg("axis", Value::Text(axis.to_string()))
                    .arg("travel", Value::Length(limit, Digits::Shortest)),
            ));
        }
    }
    let mut board = one;
    let mut any = false;
    for op in &mut board.operations {
        if op.is_board_surface() || op.is_dowels() {
            any |= op.enabled();
        } else {
            op.set_enabled(false);
        }
    }
    if !any {
        return Ok(());
    }
    crate::run_setups(&board, Some(setup))
        .map(|_| ())
        .map_err(|e| {
            PipelineError::Job(Message::new("pipeline-nest-board-fails").arg("reason", e.message()))
        })
}

/// The turns asked for, kept on the setup's parts.
fn set_turns(job: &mut Job, setup: u64, request: &NestRequest) -> Result<(), PipelineError> {
    if let Some(turns) = request.turns {
        let on = job.parts_on(setup);
        for part in job.parts.iter_mut().filter(|p| on.contains(&p.id)) {
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
/// highest-numbered enabled ones removed. Disabled instances stay.
fn set_counts(job: &mut Job, setup: u64, counts: &[(u64, u32)]) -> Result<(), PipelineError> {
    for &(id, count) in counts {
        let part = job
            .part(id)
            .filter(|_| job.part_on(id, setup))
            .ok_or_else(|| PipelineError::Job(Message::new("pipeline-no-part").arg("id", id)))?;
        if count == 0 {
            return Err(PipelineError::Job(
                Message::new("pipeline-nest-count").arg("part", part.name.as_str()),
            ));
        }
        let here = |job: &Job| -> usize {
            job.setup(setup).map_or(0, |s| {
                s.instances
                    .iter()
                    .filter(|i| i.part == id && i.enabled)
                    .count()
            })
        };
        while here(job) < count as usize {
            let next = job.next_instance_id(id);
            if let Some(s) = job.setup_mut(setup) {
                s.instances.push(Instance::drawn(id, next));
            }
        }
        while here(job) > count as usize {
            let Some(s) = job.setup_mut(setup) else {
                break;
            };
            let Some(last) = s.instances.iter().rposition(|i| i.part == id && i.enabled) else {
                break;
            };
            s.instances.remove(last);
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

/// The mirrors part `part`'s instances that are on lie in, on the active setup, each with
/// its number (the part's own mirror is its definition's).
fn instances_of(one: &Job, part: u64) -> Vec<(u32, Option<Mirror>)> {
    one.instances_here(part)
        .into_iter()
        .filter(|i| i.enabled)
        .map(|i| (i.id, i.mirror))
        .collect()
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
    for (_, mirror) in instances_of(one, part) {
        if !mirrors.contains(&mirror) {
            mirrors.push(mirror);
        }
    }
    for p in &mut alone.parts {
        p.enabled = p.id == part;
    }
    let planned = mirrors
        .iter()
        .enumerate()
        .map(|(k, &mirror)| Instance {
            part,
            id: k as u32 + 1,
            enabled: true,
            mirror,
            angle: 0.0,
            at: [pitch * k as f64, 0.0],
        })
        .collect();
    alone.set_instances_here(part, planned);
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
    if let Some(i) = job.instance_mut(part, number) {
        i.angle = angle;
        i.at = at;
    }
}

/// What didn't fit switched off (decision 10): the instance, instance 1 as any other
/// (docs/specs/parts-and-setups.md §6). It keeps its place.
fn switch_off(job: &mut Job, part: u64, number: u32) {
    if let Some(i) = job.instance_mut(part, number) {
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
