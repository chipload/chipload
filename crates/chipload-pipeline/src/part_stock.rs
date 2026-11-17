// SPDX-License-Identifier: GPL-3.0-or-later
//! A part's stock (docs/specs/parts-and-setups.md §4): the material or thickness a part is
//! cut from, recorded on the part. Setting it moves nothing: a part on a board of another
//! stock gets a note, and its form offers the move, to a board that has it, or to a new
//! one made like its own (the user, 2026-11-17: boards are made or removed only when
//! asked).

use chipload_i18n::{Digits, Message, Value};
use chipload_job::{Job, Note, PartStock, Setup, Stock};

use crate::PipelineError;

fn refused(message: Message) -> PipelineError {
    PipelineError::Job(message)
}

/// What moving a part to its stock's board did.
#[derive(Debug, Clone, PartialEq)]
pub struct StockMove {
    /// The setup the part is cut on now.
    pub to: u64,
    /// That setup was made for it.
    pub made: bool,
}

impl StockMove {
    /// "Bottom is cut from 6 mm: Setup 2", or "Made Setup 2 (6 mm) for Bottom".
    pub fn message(&self, job: &Job, part: u64) -> Option<Message> {
        let name = job.part(part).map_or(String::new(), |p| p.name.clone());
        let board = job.setup(self.to)?;
        let id = if self.made {
            "pipeline-part-stock-made"
        } else {
            "pipeline-part-stock-moved"
        };
        Some(
            Message::new(id)
                .arg("part", name)
                .arg("board", board.name.clone())
                .arg("stock", board_message(&board.stock)),
        )
    }
}

/// Where a part would go to be cut from its stock.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StockBoard {
    /// This setup has it.
    Existing(u64),
    /// None has it: a new one, made like the part's board.
    New,
}

/// A stock as words: "6 mm", "Birch plywood" or "6 mm Birch plywood".
pub fn stock_message(stock: &PartStock) -> Message {
    let length = |mm: f64| Value::Length(mm, Digits::Shortest);
    match (&stock.material, stock.thickness) {
        (Some(m), Some(t)) => Message::new("pipeline-stock-both")
            .arg("thickness", length(t))
            .arg("material", m.name.clone()),
        (Some(m), None) => Message::new("pipeline-stock-material").arg("material", m.name.clone()),
        (None, Some(t)) => Message::new("pipeline-stock-thickness").arg("thickness", length(t)),
        (None, None) => Message::new("pipeline-stock-any"),
    }
}

/// A board's stock as words: its thickness, and its material when it has one.
pub fn board_message(stock: &Stock) -> Message {
    stock_message(&PartStock {
        material: stock.material.clone(),
        thickness: Some(stock.thickness),
    })
}

/// A board made on purpose, which a part's stock never moves a part onto or off: a front
/// or back of a two-sided board, or a board in pieces (§4).
fn fixed(job: &Job, setup: &Setup) -> bool {
    setup.back_of.is_some() || setup.pieces.is_some() || job.back_of(setup.id).is_some()
}

/// Part `id` cut from `stock` (None or empty: any board). Nothing moves (§4).
pub fn set_part_stock(job: &Job, id: u64, stock: Option<PartStock>) -> Result<Job, PipelineError> {
    let at = part_index(job, id)?;
    let mut job = job.clone();
    job.parts[at].stock = stock.filter(|s| !s.is_empty());
    checked(job)
}

fn part_index(job: &Job, id: u64) -> Result<usize, PipelineError> {
    job.parts
        .iter()
        .position(|p| p.id == id)
        .ok_or_else(|| refused(Message::new("pipeline-no-part").arg("id", id)))
}

/// Where part `id` would go to be cut from its stock: None when it asks for none, every
/// board it is on has it, it is placed nowhere, or it is on a two-sided board or one in
/// pieces (made on purpose, never changed for a part's stock).
pub fn stock_board(job: &Job, id: u64) -> Option<StockBoard> {
    let wanted = job.part(id)?.stock.as_ref()?;
    let holds = job.setups_of_part(id);
    let board = |h: &u64| job.setup(*h);
    if holds.is_empty()
        || holds
            .iter()
            .any(|h| board(h).is_some_and(|s| fixed(job, s)))
        || holds
            .iter()
            .all(|h| board(h).is_some_and(|b| wanted.fits(&b.stock)))
    {
        return None;
    }
    Some(
        job.setups
            .iter()
            .find(|b| !fixed(job, b) && wanted.fits(&b.stock))
            .map_or(StockBoard::New, |b| StockBoard::Existing(b.id)),
    )
}

/// Part `id` moved, every instance of it, to the board [`stock_board`] names, made like its
/// board when none has its stock; that board becomes the active one, the part's cuts
/// through the stock following it. Refused when there is nowhere to move it.
pub fn move_to_stock_board(job: &Job, id: u64) -> Result<(Job, StockMove), PipelineError> {
    part_index(job, id)?;
    let target = stock_board(job, id)
        .ok_or_else(|| refused(Message::new("pipeline-part-stock-nowhere").arg("id", id)))?;
    let mut job = job.clone();
    let (to, made) = match target {
        StockBoard::Existing(to) => (to, false),
        StockBoard::New => {
            let wanted = job
                .part(id)
                .and_then(|p| p.stock.clone())
                .unwrap_or_default();
            (make_board(&mut job, id, &wanted)?, true)
        }
    };
    job.move_part_to(id, to);
    job.active_setup = Some(to);
    job.follow_thickness();
    Ok((checked(job)?, StockMove { to, made }))
}

/// A new board for part `id`'s stock, made like the board it is cut from: its size or fit,
/// origin, work zero, Z zero and overcut, with the stock's thickness and material
/// (§4). Returns its id.
fn make_board(job: &mut Job, id: u64, wanted: &PartStock) -> Result<u64, PipelineError> {
    let home = job
        .home_setup(id)
        .and_then(|h| job.setup(h))
        .cloned()
        .ok_or_else(|| refused(Message::new("pipeline-no-setup").arg("id", 0)))?;
    let new = job.setups.iter().map(|s| s.id).max().unwrap_or(0) + 1;
    let mut stock = home.stock.clone();
    if let Some(t) = wanted.thickness {
        stock.thickness = t;
    }
    if let Some(m) = &wanted.material {
        stock.material = Some(m.clone());
    }
    job.setups.push(Setup {
        id: new,
        name: format!("Setup {}", job.setups.len() + 1),
        enabled: true,
        stock,
        work_zero: home.work_zero,
        back_of: None,
        pieces: None,
        nest: None,
        instances: Vec::new(),
        placed: None,
    });
    Ok(new)
}

fn checked(job: Job) -> Result<Job, PipelineError> {
    match job.validate().into_iter().next() {
        Some(problem) => Err(refused(problem.text_message)),
        None => Ok(job),
    }
}

/// For the active setup: each part on it whose stock this board isn't, placed there by
/// hand or the board changed since (§4). A note, not an error: the user may mean it.
pub fn stock_mismatch_notes(job: &Job) -> Vec<Note> {
    let Some(setup) = job.active_setup_id().and_then(|id| job.setup(id)) else {
        return Vec::new();
    };
    job.stock_mismatches(setup.id)
        .into_iter()
        .filter_map(|id| job.part(id))
        .filter_map(|part| {
            let wants = part.stock.as_ref()?;
            Some(Note::job(
                Message::new("pipeline-part-wrong-stock")
                    .arg("part", part.name.clone())
                    .arg("wants", stock_message(wants))
                    .arg("board", board_message(&setup.stock)),
            ))
        })
        .collect()
}
