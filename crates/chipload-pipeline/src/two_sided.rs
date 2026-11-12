// SPDX-License-Identifier: GPL-3.0-or-later
//! A back side with its dowel pins in one step (docs/specs/two-sided-wizard.md §4): what
//! *Two-sided…* and `setup add --back-of … --pins` make, and the picture the wizard draws.

use chipload_i18n::Language;
use chipload_job::{Job, Register, Turn};

use crate::PipelineError;
use crate::dowels::{DowelLayout, Dowels, add_dowels, dowel_layout, remove_dowels};
use crate::setups::{BoardChange, activate_setup, add_back_side, set_board};

/// The pins of a back put back by dowel pins.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PinsRequest {
    pub diameter: f64,
    /// How far into the spoilboard, mm.
    pub depth: f64,
    /// Set the front's work zero to the first pin's centre.
    pub zero: bool,
    /// The project tool that cuts them; None: `dowels::pin_tool`'s choice.
    pub tool: Option<i64>,
}

/// A back side for setup `front`, or the one it has, changed.
#[derive(Debug, Clone, PartialEq)]
pub struct TwoSidedRequest {
    pub front: u64,
    pub turn: Turn,
    pub register: Register,
    /// With `Register::Pins`: the dowel holes, made again if there are some.
    pub pins: Option<PinsRequest>,
}

/// What [`make_two_sided`] made.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TwoSidedMade {
    /// The back setup.
    pub back: u64,
    /// Whether it is new.
    pub added: bool,
    /// The dowel holes operation, when made.
    pub dowels: Option<usize>,
}

/// The project with `request`'s board, back and way of putting it back, before its pins:
/// the back's id and whether it is new.
fn with_back(job: &Job, request: &TwoSidedRequest) -> Result<(Job, u64, bool), PipelineError> {
    let mut job = job.clone();
    let change = BoardChange {
        turn: Some(request.turn),
        register: Some(request.register),
        ..BoardChange::default()
    };
    match job.back_of(request.front).map(|s| s.id) {
        Some(back) => {
            // Dowel holes made again, or off a turn axis that moves, go first.
            let turned = job
                .setup(back)
                .and_then(|s| s.back_of)
                .is_some_and(|b| b.turn != request.turn);
            if job.dowels_of(request.front).is_some() && (turned || request.pins.is_some()) {
                job = remove_dowels(&job, back)?;
            }
            Ok((set_board(&job, back, change)?, back, false))
        }
        None => {
            let (job, back, _) = add_back_side(&job, request.front, request.turn, None)?;
            Ok((set_board(&job, back, change)?, back, true))
        }
    }
}

/// Make the back side `request` asks for, or change the one there is: its turn, how it is
/// put back, its dowel holes (made again when there were some) and the front's work zero at
/// the first pin; the back becomes the active setup.
pub fn make_two_sided(
    job: &Job,
    request: &TwoSidedRequest,
    lang: Language,
) -> Result<(Job, TwoSidedMade), PipelineError> {
    let (mut out, back, added) = with_back(job, request)?;
    let mut dowels = None;
    if let (Register::Pins, Some(pins)) = (request.register, request.pins) {
        let made = Dowels {
            diameter: pins.diameter,
            depth: pins.depth,
            zero: pins.zero,
            tool: pins.tool,
        };
        let (with, at) = add_dowels(&out, back, made, lang)?;
        out = with;
        dowels = Some(at);
    }
    let out = activate_setup(&out, back)?;
    Ok((
        out,
        TwoSidedMade {
            back,
            added,
            dowels,
        },
    ))
}

/// Where the pins of `request` would go, for the wizard's picture: the board, where both
/// sides cut, the turn axis and the holes, fitting or not. Its pin diameter is the
/// request's, or the default.
pub fn two_sided_layout(
    job: &Job,
    request: &TwoSidedRequest,
) -> Result<DowelLayout, PipelineError> {
    let (out, back, _) = with_back(job, request)?;
    let diameter = request
        .pins
        .map_or(crate::dowels::DEFAULT_PIN_DIAMETER, |p| p.diameter);
    dowel_layout(&out, back, diameter)
}
