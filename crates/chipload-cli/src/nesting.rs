// SPDX-License-Identifier: GPL-3.0-or-later
//! `chipload nest` (docs/specs/nesting.md §8): a setup's instances placed on its board.

use std::path::PathBuf;
use std::process::ExitCode;

use chipload_job::{NestGoal, Turns};
use chipload_pipeline::nest::{NestRequest, nest_setup};
use chipload_toolpath::Cancel;
use serde_json::json;

use crate::instances::{degrees_text, mm3};
use crate::out::Out;
use crate::project_io::{load, save_project};

/// Nest a setup: every instance of its parts that is on, the part itself included, placed
/// on its board as close as they fit without cutting into each other, what doesn't fit
/// switched off (docs/specs/nesting.md). Only placements change; the drawing stays.
#[derive(clap::Args)]
pub struct NestArgs {
    /// The project (.chipload).
    project: PathBuf,
    /// The setup (as `setup list` shows it); default: the active one.
    #[arg(long)]
    setup: Option<u64>,
    /// The board's size "X×Y", such as 1220x610 (any length unit; plain numbers are mm).
    /// Needed when the board follows the drawing: it becomes this size, its corner at the
    /// drawing's origin.
    #[arg(long, value_parser = crate::commands::board_size)]
    board: Option<(f64, f64)>,
    /// The least distance between what two instances cut (default: as last nested, else 3
    /// mm).
    #[arg(long, value_parser = crate::lengths::length)]
    gap: Option<f64>,
    /// The room left at the board's edge (default: as last nested, else 10 mm).
    #[arg(long, value_parser = crate::lengths::length)]
    margin: Option<f64>,
    /// How parts may be turned: none, half, quarter or every-15, for every part; "ID=TURNS"
    /// for one part, such as 2=none (wood grain). Kept in the project. Default: as set,
    /// else quarter.
    #[arg(long, value_parser = turns_arg)]
    turns: Vec<TurnsArg>,
    /// How many times part ID is cut, "ID=N", such as 2=4: instances added or removed
    /// before nesting (never the part itself).
    #[arg(long, value_parser = count_arg)]
    count: Vec<(u64, u32)>,
    /// What doesn't fit always goes onto more boards: the next setup with the same stock,
    /// else a new one like this one (docs/specs/nest-overflow-and-all.md §1). Kept for
    /// scripts: `new-boards` is what happens anyway; `off` is refused.
    #[arg(long, value_enum)]
    overflow: Option<Overflow>,
    /// What the nest makes as small as it can: `compact` (the smallest box round the
    /// parts), `low` (the lowest top edge: a strip across the top left free) or `left` (the
    /// leftmost right edge: a strip down the right left free). Kept in the project.
    /// Default: as last nested, else compact (docs/specs/nesting-search.md).
    #[arg(long, value_enum)]
    goal: Option<GoalArg>,
    /// Report what would be placed without writing the project.
    #[arg(long)]
    dry_run: bool,
    /// Where to write the result; default: the project itself.
    #[arg(short, long)]
    output: Option<PathBuf>,
}

/// `--goal`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum GoalArg {
    /// The smallest box round the parts.
    Compact,
    /// The lowest top edge.
    Low,
    /// The leftmost right edge.
    Left,
}

impl From<GoalArg> for NestGoal {
    fn from(goal: GoalArg) -> NestGoal {
        match goal {
            GoalArg::Compact => NestGoal::Compact,
            GoalArg::Low => NestGoal::Low,
            GoalArg::Left => NestGoal::Left,
        }
    }
}

/// `--overflow`, kept for scripts written before what doesn't fit always went onto more
/// boards.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum Overflow {
    /// No longer offered: refused with the reason.
    Off,
    /// What happens anyway.
    NewBoards,
}

/// `--turns`: for every part, or for one.
#[derive(Debug, Clone, Copy)]
pub struct TurnsArg {
    part: Option<u64>,
    turns: Turns,
}

/// "none", "half", "quarter", "every-15" (also "every_15", "15").
pub(crate) fn turns_value(text: &str) -> Result<Turns, String> {
    match text.trim().to_ascii_lowercase().as_str() {
        "none" => Ok(Turns::None),
        "half" => Ok(Turns::Half),
        "quarter" => Ok(Turns::Quarter),
        "every-15" | "every_15" | "15" => Ok(Turns::Every15),
        other => Err(format!(
            "expected none, half, quarter or every-15, got {other:?}"
        )),
    }
}

fn turns_arg(text: &str) -> Result<TurnsArg, String> {
    match text.split_once('=') {
        Some((part, turns)) => Ok(TurnsArg {
            part: Some(part_id(part)?),
            turns: turns_value(turns)?,
        }),
        None => Ok(TurnsArg {
            part: None,
            turns: turns_value(text)?,
        }),
    }
}

fn count_arg(text: &str) -> Result<(u64, u32), String> {
    let (part, count) = text
        .split_once('=')
        .ok_or_else(|| format!("expected ID=N, such as 2=4, got {text:?}"))?;
    let count = count
        .trim()
        .parse()
        .map_err(|_| format!("expected a whole number of times, got {count:?}"))?;
    Ok((part_id(part)?, count))
}

fn part_id(text: &str) -> Result<u64, String> {
    text.trim()
        .parse()
        .map_err(|_| format!("expected a part's id, got {text:?}"))
}

pub(crate) fn nest(out: &mut Out, args: NestArgs) -> ExitCode {
    if args.overflow == Some(Overflow::Off) {
        return out.fail(
            "--overflow off: what doesn't fit goes onto more boards; remove a setup you \
             don't want",
        );
    }
    let job = match load(out, &args.project) {
        Ok(job) => job,
        Err(code) => return code,
    };
    let request = NestRequest {
        setup: args.setup,
        board: args.board,
        gap: args.gap,
        margin: args.margin,
        turns: args
            .turns
            .iter()
            .filter(|t| t.part.is_none())
            .map(|t| t.turns)
            .next_back(),
        part_turns: args
            .turns
            .iter()
            .filter_map(|t| t.part.map(|p| (p, t.turns)))
            .collect(),
        counts: args.count.clone(),
        goal: args.goal.map(NestGoal::from),
    };
    let nested = match nest_setup(&job, &request, &Cancel::new()) {
        Ok(nested) => nested,
        Err(err) => return out.fail(err),
    };
    let setup = nested.job.setup(nested.setup);
    let name = setup.map_or(String::new(), |s| s.name.clone());
    out.line(format!(
        "Nested setup {} \"{name}\": {}",
        nested.setup,
        nested.summary().english()
    ));
    let put_line = |put: &chipload_pipeline::nest::NestPut| {
        let part = nested.job.part(put.part).map_or("", |p| p.name.as_str());
        let turned = if put.angle == 0.0 {
            String::new()
        } else {
            format!(", turned {}°", degrees_text(put.angle))
        };
        format!(
            "  Part {} \"{part}\" instance {}: at {}, {} mm{turned}",
            put.part,
            put.instance,
            mm3(put.at[0]),
            mm3(put.at[1])
        )
    };
    for put in &nested.placed {
        out.line(put_line(put));
    }
    for board in &nested.overflow {
        let name = nested
            .job
            .setup(board.setup)
            .map_or("", |s| s.name.as_str());
        let made = if board.made { ", made" } else { "" };
        out.line(format!("Setup {} \"{name}\"{made}:", board.setup));
        for put in &board.placed {
            out.line(put_line(put));
        }
    }
    let notes: Vec<String> = nested.notes().iter().map(|m| m.english()).collect();
    for note in &notes {
        out.line(format!("Note: {note}"));
    }
    let area = nested.usable.0 * nested.usable.1;
    let share = if area > 0.0 {
        (100.0 * nested.extent.0 * nested.extent.1 / area).round()
    } else {
        0.0
    };
    out.set("setup", json!(nested.setup));
    out.set(
        "placed",
        json!(
            nested
                .placed
                .iter()
                .map(|p| json!({"part": p.part, "instance": p.instance, "angle": p.angle, "at": p.at}))
                .collect::<Vec<_>>()
        ),
    );
    out.set(
        "unplaced",
        json!(
            nested
                .unplaced
                .iter()
                .map(|&(part, instance)| json!({"part": part, "instance": instance}))
                .collect::<Vec<_>>()
        ),
    );
    out.set(
        "overflow",
        json!(
            nested
                .overflow
                .iter()
                .map(|b| json!({
                    "setup": b.setup,
                    "made": b.made,
                    "placed": b.placed.iter().map(|p| json!({"part": p.part, "instance": p.instance, "angle": p.angle, "at": p.at})).collect::<Vec<_>>(),
                }))
                .collect::<Vec<_>>()
        ),
    );
    let goal = match nested.goal {
        NestGoal::Compact => "compact",
        NestGoal::Low => "low",
        NestGoal::Left => "left",
    };
    out.set("goal", json!(goal));
    out.set("extent", json!([nested.extent.0, nested.extent.1]));
    out.set("usable", json!([nested.usable.0, nested.usable.1]));
    out.set("used_pct", json!(share));
    out.set("board_fixed", json!(nested.board_fixed));
    out.set("unchanged", json!(nested.unchanged));
    out.set(
        "travel",
        json!(
            nested
                .travel
                .iter()
                .map(|t| json!({"axis": t.axis.to_string(), "board": t.board, "travel": t.travel}))
                .collect::<Vec<_>>()
        ),
    );
    if args.dry_run {
        out.line("Not written (--dry-run)");
        return ExitCode::SUCCESS;
    }
    let target = args.output.as_deref().unwrap_or(&args.project);
    if let Err(err) = save_project(out, &nested.job, target) {
        return out.fail(err);
    }
    out.wrote(target, "");
    ExitCode::SUCCESS
}
