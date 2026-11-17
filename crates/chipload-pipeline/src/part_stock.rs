// SPDX-License-Identifier: GPL-3.0-or-later
//! A part's stock picks its board (docs/specs/parts-and-setups.md §4, decision 11): setting
//! the material or thickness a part is cut from moves every instance of it to a board that
//! has it, made like the board it leaves when there is none; a board made so is removed
//! again once it is empty. A part placed by hand on a board of another stock stays there,
//! with a note.

use chipload_i18n::{Digits, Message, Value};
use chipload_job::{Job, Note, PartStock, Setup, Stock};

use crate::PipelineError;

fn refused(message: Message) -> PipelineError {
    PipelineError::Job(message)
}

/// What setting a part's stock did to the boards.
#[derive(Debug, Clone, PartialEq)]
pub struct StockMove {
    /// The setup the part is cut on now.
    pub to: u64,
    /// That setup was made for it.
    pub made: bool,
    /// The setups removed, left empty, by name.
    pub removed: Vec<String>,
}

impl StockMove {
    /// "Bottom is cut from 6 mm: Setup 2", or "Made Setup 2 (6 mm) for Bottom"; then each
    /// board removed.
    pub fn messages(&self, job: &Job, part: u64) -> Vec<Message> {
        let name = job.part(part).map_or(String::new(), |p| p.name.clone());
        let Some(board) = job.setup(self.to) else {
            return Vec::new();
        };
        let id = if self.made {
            "pipeline-part-stock-made"
        } else {
            "pipeline-part-stock-moved"
        };
        let mut out = vec![
            Message::new(id)
                .arg("part", name)
                .arg("board", board.name.clone())
                .arg("stock", board_message(&board.stock)),
        ];
        out.extend(
            self.removed
                .iter()
                .map(|r| Message::new("pipeline-part-stock-removed").arg("board", r.clone())),
        );
        out
    }
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

/// Part `id` cut from `stock` (None or empty: any board), moved with every instance of it to
/// a board that has it when its board hasn't: the first such board, else a new one made like
/// the board it leaves. Cleared, a part on boards made for a stock goes back to the first
/// board that wasn't. A board made for a stock and left empty is removed. A part on a
/// two-sided board or one in pieces, or placed nowhere, isn't moved. The board it goes to
/// becomes the active one.
pub fn set_part_stock(
    job: &Job,
    id: u64,
    stock: Option<PartStock>,
) -> Result<(Job, Option<StockMove>), PipelineError> {
    let at = job
        .parts
        .iter()
        .position(|p| p.id == id)
        .ok_or_else(|| refused(Message::new("pipeline-no-part").arg("id", id)))?;
    let mut job = job.clone();
    let stock = stock.filter(|s| !s.is_empty());
    job.parts[at].stock = stock.clone();
    let holds = job.setups_of_part(id);
    let on_fixed = holds
        .iter()
        .any(|h| job.setup(*h).is_some_and(|s| fixed(&job, s)));
    if holds.is_empty() || on_fixed {
        return Ok((checked(job)?, None));
    }
    let board = |h: &u64| job.setup(*h);
    let target = match &stock {
        Some(wanted)
            if holds
                .iter()
                .all(|h| board(h).is_some_and(|b| wanted.fits(&b.stock))) =>
        {
            None
        }
        Some(wanted) => Some(
            job.setups
                .iter()
                .find(|b| !fixed(&job, b) && wanted.fits(&b.stock))
                .map(|b| (b.id, false)),
        ),
        None if holds
            .iter()
            .all(|h| board(h).is_some_and(|b| b.made_for_stock)) =>
        {
            Some(
                job.setups
                    .iter()
                    .find(|b| !b.made_for_stock && !fixed(&job, b))
                    .map(|b| (b.id, false)),
            )
        }
        None => None,
    };
    let Some(target) = target else {
        return Ok((checked(job)?, None));
    };
    let (to, made) = match (target, &stock) {
        (Some(found), _) => found,
        (None, Some(wanted)) => (make_board(&mut job, id, wanted)?, true),
        // Cleared, with no board of its own to go back to: it stays.
        (None, None) => return Ok((checked(job)?, None)),
    };
    job.move_part_to(id, to);
    let removed = remove_empty_made(&mut job, to);
    job.active_setup = Some(to);
    // Its cuts through the stock go through its new board.
    job.follow_thickness();
    Ok((checked(job)?, Some(StockMove { to, made, removed })))
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
        made_for_stock: true,
        placed: None,
    });
    Ok(new)
}

/// The boards made for a stock that nothing is cut from any more, but `keep`, removed;
/// their names.
fn remove_empty_made(job: &mut Job, keep: u64) -> Vec<String> {
    let gone: Vec<(u64, String)> = job
        .setups
        .iter()
        .filter(|s| s.made_for_stock && s.instances.is_empty() && s.id != keep)
        .map(|s| (s.id, s.name.clone()))
        .collect();
    job.setups
        .retain(|s| !gone.iter().any(|(id, _)| *id == s.id));
    gone.into_iter().map(|(_, name)| name).collect()
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
