// SPDX-License-Identifier: GPL-3.0-or-later
//! `chipload setup …`: a project's setups, each one board clamped once with its stock, its
//! work zero and the parts cut from it, run as a program of its own (docs/specs/setups.md
//! §8). The board's values are set by path: `set P setups.1.stock.thickness=6`.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use chipload_i18n::Language;
use chipload_job::{Register, Turn};
use chipload_pipeline::dowels::{
    DEFAULT_PIN_DEPTH, DEFAULT_PIN_DIAMETER, Dowels, add_dowels, place_dowels_again, remove_dowels,
};
use chipload_pipeline::setups::{
    BoardChange, activate_setup, add_back_side, add_setup, move_part_to_setup, remove_setup,
    set_board, set_setup,
};
use chipload_pipeline::two_sided::{PinsRequest, TwoSidedRequest, make_two_sided};
use clap::Subcommand;
use serde_json::{Value, json};

use crate::out::Out;
use crate::{load, write_project};

#[derive(Subcommand)]
pub enum SetupAction {
    /// The project's setups: id, name, on or off, board, work zero and parts.
    List {
        /// The project (.chipload).
        project: PathBuf,
    },
    /// Add a setup at the end, a board like the active one's, with a first part of its own;
    /// both become active.
    Add {
        /// The project (.chipload).
        project: PathBuf,
        /// Its name; default: "Setup N".
        #[arg(long)]
        name: Option<String>,
        /// Its board's thickness; default: the active setup's.
        #[arg(long, value_parser = crate::lengths::length, conflicts_with = "back_of")]
        thickness: Option<f64>,
        /// The back of this setup: the same board turned over, its cuts mirrored
        /// (docs/specs/two-sided.md). Its name defaults to "<that setup> back".
        #[arg(long)]
        back_of: Option<u64>,
        /// How the board is turned over: left-right (default) or top-bottom.
        #[arg(long, value_parser = turn, requires = "back_of")]
        turn: Option<Turn>,
        /// How the turned board is put back: pins (default) or corner.
        #[arg(long, value_parser = register, requires = "back_of")]
        register: Option<Register>,
        /// Dowel holes for pins of this diameter too (8 mm without a value), in the same
        /// step (docs/specs/two-sided-wizard.md §6).
        #[arg(long, value_parser = crate::lengths::length, num_args = 0..=1,
            default_missing_value = "8", requires = "back_of", conflicts_with = "register")]
        pins: Option<f64>,
        /// How far the dowel holes go into the spoilboard; default 8 mm.
        #[arg(long, value_parser = crate::lengths::length, requires = "pins")]
        pin_depth: Option<f64>,
        /// Set the front's work zero to the first pin's centre, which the back shares.
        #[arg(long, requires = "pins")]
        zero: bool,
        /// The project tool that cuts the dowel holes; default: a drill of their size, else
        /// the largest end mill that bores them.
        #[arg(long, requires = "pins")]
        pin_tool: Option<i64>,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Rename a setup, or turn it off or on (a setup that is off isn't cut).
    Set {
        /// The project (.chipload).
        project: PathBuf,
        /// The setup's id (as `setup list` shows it).
        id: u64,
        /// Its new name.
        #[arg(long)]
        name: Option<String>,
        /// Leave the setup and its parts out.
        #[arg(long, conflicts_with = "on")]
        off: bool,
        /// Cut the setup again.
        #[arg(long)]
        on: bool,
        /// Make it the back of this setup: the same board turned over.
        #[arg(long, conflicts_with = "own_board")]
        back_of: Option<u64>,
        /// Give a back a board of its own again, as it is now.
        #[arg(long)]
        own_board: bool,
        /// How a back is turned over: left-right or top-bottom.
        #[arg(long, value_parser = turn)]
        turn: Option<Turn>,
        /// How a turned board is located: pins (dowel pins on the turn axis) or corner.
        #[arg(long, value_parser = register)]
        register: Option<Register>,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Dowel holes for a back setup (docs/specs/two-sided.md §6): two circles on the line
    /// the board turns about, in the margin beyond the shapes, cut first with the front,
    /// through the board and into the spoilboard for the pins.
    Dowels {
        /// The project (.chipload).
        project: PathBuf,
        /// The back setup's id.
        #[arg(long)]
        setup: u64,
        /// The pins' diameter; default 8 mm (a 6 mm end mill bores it).
        #[arg(long, value_parser = crate::lengths::length)]
        diameter: Option<f64>,
        /// How far the holes go into the spoilboard; default 8 mm.
        #[arg(long, value_parser = crate::lengths::length)]
        depth: Option<f64>,
        /// Set the front's work zero to the first pin's centre, which the back shares.
        #[arg(long)]
        zero: bool,
        /// The project tool that cuts them; default: a drill of their size, else the largest
        /// end mill that bores them.
        #[arg(long)]
        tool: Option<i64>,
        /// Remove the dowel holes instead: the front's operation and its circles.
        #[arg(long, conflicts_with_all = ["diameter", "depth", "zero", "tool", "place"])]
        remove: bool,
        /// Place the dowel holes there are again, clear of both sides' cuts as they are
        /// now; a work zero at the first pin moves with it. Not once the front is cut: its
        /// holes are drilled.
        #[arg(long, conflicts_with_all = ["diameter", "depth", "zero", "tool"])]
        place: bool,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Make a setup the active one: shown, simulated, where new parts go.
    Activate {
        /// The project (.chipload).
        project: PathBuf,
        /// The setup's id.
        id: u64,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Move a part into a setup: it is cut from that board, with its zero.
    Part {
        /// The project (.chipload).
        project: PathBuf,
        /// The part's id (as `part list` shows it).
        part: u64,
        /// The setup's id.
        #[arg(long)]
        to: u64,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Remove a setup. Refused while it has parts, unless --with-parts.
    Remove {
        /// The project (.chipload).
        project: PathBuf,
        /// The setup's id.
        id: u64,
        /// Remove its parts and their operations with it.
        #[arg(long)]
        with_parts: bool,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

pub fn setup(out: &mut Out, action: SetupAction) -> ExitCode {
    let (project, output) = match &action {
        SetupAction::List { project } => return list(out, project),
        SetupAction::Add {
            project, output, ..
        }
        | SetupAction::Set {
            project, output, ..
        }
        | SetupAction::Activate {
            project, output, ..
        }
        | SetupAction::Dowels {
            project, output, ..
        }
        | SetupAction::Part {
            project, output, ..
        }
        | SetupAction::Remove {
            project, output, ..
        } => (project.clone(), output.clone()),
    };
    let job = match load(out, &project) {
        Ok(job) => job,
        Err(code) => return code,
    };
    let changed = match action {
        SetupAction::List { .. } => return ExitCode::SUCCESS,
        SetupAction::Add {
            back_of: Some(front),
            turn,
            pins: Some(diameter),
            pin_depth,
            zero,
            pin_tool,
            ..
        } => {
            let request = TwoSidedRequest {
                front,
                turn: turn.unwrap_or(Turn::LeftRight),
                register: Register::Pins,
                pins: Some(PinsRequest {
                    diameter,
                    depth: pin_depth.unwrap_or(DEFAULT_PIN_DEPTH),
                    zero,
                    tool: pin_tool,
                }),
            };
            make_two_sided(&job, &request, Language::En).map(|(job, made)| {
                tight_warning(out, &job, made.back, diameter);
                let at = made.dowels.map_or(0, |i| i + 1);
                out.line(format!(
                    "Added setup {}, the back of setup {front}, and operation {at}: dowel \
                     holes; now the active setup",
                    made.back
                ));
                out.set("setup", json!(made.back));
                out.set("operation", json!(at));
                job
            })
        }
        SetupAction::Add {
            name,
            back_of: Some(front),
            turn,
            register,
            ..
        } => add_back_side(
            &job,
            front,
            turn.unwrap_or(Turn::LeftRight),
            name.as_deref(),
        )
        .and_then(|(job, id, part)| {
            let job = match register {
                Some(register) => set_board(
                    &job,
                    id,
                    BoardChange {
                        register: Some(register),
                        ..BoardChange::default()
                    },
                )?,
                None => job,
            };
            out.line(format!(
                "Added setup {id}, the back of setup {front}, with part {part}, now the \
                     active setup and part"
            ));
            out.set("setup", json!(id));
            out.set("part", json!(part));
            Ok(job)
        }),
        SetupAction::Add {
            name, thickness, ..
        } => add_setup(&job, name.as_deref(), thickness).map(|(job, id, part)| {
            out.line(format!(
                "Added setup {id} with part {part}, now the active setup and part"
            ));
            out.set("setup", json!(id));
            out.set("part", json!(part));
            job
        }),
        SetupAction::Set {
            id,
            name,
            off,
            on,
            back_of,
            own_board,
            turn,
            register,
            ..
        } => {
            let enabled = match (off, on) {
                (true, _) => Some(false),
                (_, true) => Some(true),
                _ => None,
            };
            let change = BoardChange {
                back_of: if own_board {
                    Some(None)
                } else {
                    back_of.map(Some)
                },
                turn,
                register,
            };
            set_setup(&job, id, name.as_deref(), enabled).and_then(|job| {
                if change == BoardChange::default() {
                    Ok(job)
                } else {
                    set_board(&job, id, change)
                }
            })
        }
        SetupAction::Activate { id, .. } => activate_setup(&job, id),
        SetupAction::Dowels {
            setup,
            remove: true,
            ..
        } => remove_dowels(&job, setup).inspect(|_| out.line("Removed the dowel holes")),
        SetupAction::Dowels {
            setup, place: true, ..
        } => place_dowels_again(&job, setup).inspect(|job| {
            out.line("Placed the dowel holes again");
            let diameter = job
                .dowels_of(
                    job.setup(setup)
                        .and_then(|s| s.back_of)
                        .map_or(0, |b| b.setup),
                )
                .and_then(|at| dowel_diameter(job, at));
            if let Some(diameter) = diameter {
                tight_warning(out, job, setup, diameter);
            }
        }),
        SetupAction::Dowels {
            setup,
            diameter,
            depth,
            zero,
            tool,
            ..
        } => {
            let dowels = Dowels {
                diameter: diameter.unwrap_or(DEFAULT_PIN_DIAMETER),
                depth: depth.unwrap_or(DEFAULT_PIN_DEPTH),
                zero,
                tool,
            };
            add_dowels(&job, setup, dowels, Language::En).map(|(job, at)| {
                tight_warning(out, &job, setup, dowels.diameter);
                out.line(format!("Added operation {}: dowel holes", at + 1));
                out.set("operation", json!(at + 1));
                job
            })
        }
        SetupAction::Part { part, to, .. } => move_part_to_setup(&job, part, to),
        SetupAction::Remove { id, with_parts, .. } => {
            remove_setup(&job, id, with_parts).map(|(job, lines)| {
                for line in lines {
                    out.line(line);
                }
                job
            })
        }
    };
    match changed {
        Ok(job) => write_project(out, &job, output.as_deref().unwrap_or(&project)),
        Err(err) => out.fail(err),
    }
}

/// Dowel holes on a board of its own size without their room: made, with a warning
/// (docs/specs/two-sided.md §6). A board fitted round the drawing grows round them.
fn tight_warning(out: &mut Out, job: &chipload_job::Job, back: u64, diameter: f64) {
    let tight = chipload_pipeline::dowels::dowel_layout(job, back, diameter)
        .ok()
        .and_then(|layout| layout.tight());
    if let Some(message) = tight {
        out.line(format!("warning: {}", message.english()));
    }
}

/// The diameter of operation `at`'s dowel circles.
fn dowel_diameter(job: &chipload_job::Job, at: usize) -> Option<f64> {
    let select = job.operations.get(at)?.select().resolved(&job.drawing);
    job.drawing.shapes.iter().find_map(|s| match s.geometry {
        chipload_job::Geometry::Circle { radius, .. } if select.may_pick(s) => Some(2.0 * radius),
        _ => None,
    })
}

fn list(out: &mut Out, project: &Path) -> ExitCode {
    let job = match load(out, project) {
        Ok(job) => job,
        Err(code) => return code,
    };
    let active = job.active_setup_id();
    let mut rows = Vec::new();
    for setup in &job.setups {
        let one = job.for_setup(setup.id);
        let (x, y) = one.stock_size();
        let parts: Vec<u64> = job
            .parts
            .iter()
            .filter(|p| p.setup == setup.id)
            .map(|p| p.id)
            .collect();
        let numbers: Vec<String> = parts.iter().map(u64::to_string).collect();
        let mm = chipload_core::text::mm;
        // A back put back by pins: its front's dowel holes, or a warning that there are none.
        let dowels = setup
            .back_of
            .filter(|b| b.register == chipload_job::Register::Pins)
            .map(|b| job.dowels_of(b.setup));
        let link = setup.back_of.map_or(String::new(), |b| {
            let holes = match dowels {
                Some(Some(i)) if !job.missing_dowels(setup.id) => {
                    format!(", dowel holes: operation {}", i + 1)
                }
                Some(_) => ", no dowel holes (warning)".to_string(),
                None => String::new(),
            };
            format!(
                ", back of {}, {}, {}{holes}",
                b.setup,
                b.turn.name(),
                b.register.name()
            )
        });
        out.line(format!(
            "{} {}{}{}{link}: board {} × {} × {} mm; zero {}; parts {}",
            setup.id,
            setup.name,
            if Some(setup.id) == active {
                " (active)"
            } else {
                ""
            },
            if setup.enabled { "" } else { ", off" },
            mm(x),
            mm(y),
            mm(setup.stock.thickness),
            one.work_zero_text(),
            if numbers.is_empty() {
                "none".to_string()
            } else {
                numbers.join(", ")
            }
        ));
        rows.push(json!({
            "id": setup.id,
            "name": setup.name,
            "enabled": setup.enabled,
            "active": Some(setup.id) == active,
            "board": [x, y, setup.stock.thickness],
            "work_zero": setup.work_zero,
            "back_of": setup.back_of,
            "dowels": dowels.flatten().map(|i| i + 1),
            "missing_dowels": job.missing_dowels(setup.id),
            "parts": parts,
        }));
    }
    out.set("setups", Value::Array(rows));
    ExitCode::SUCCESS
}

/// A turn as the command line writes it.
fn turn(text: &str) -> Result<Turn, String> {
    match text {
        "left-right" => Ok(Turn::LeftRight),
        "top-bottom" => Ok(Turn::TopBottom),
        _ => Err("left-right or top-bottom".into()),
    }
}

/// A way of locating a turned board as the command line writes it.
fn register(text: &str) -> Result<Register, String> {
    match text {
        "pins" => Ok(Register::Pins),
        "corner" => Ok(Register::Corner),
        _ => Err("pins or corner".into()),
    }
}
