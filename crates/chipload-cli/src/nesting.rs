// SPDX-License-Identifier: GPL-3.0-or-later
//! `chipload nest` (docs/specs/nesting.md §8): a setup's instances placed on its board.

use std::path::PathBuf;
use std::process::ExitCode;

use chipload_job::Turns;
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
    /// Report what would be placed without writing the project.
    #[arg(long)]
    dry_run: bool,
    /// Where to write the result; default: the project itself.
    #[arg(short, long)]
    output: Option<PathBuf>,
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
    for put in &nested.placed {
        let part = nested.job.part(put.part).map_or("", |p| p.name.as_str());
        let turned = if put.angle == 0.0 {
            String::new()
        } else {
            format!(", turned {}°", degrees_text(put.angle))
        };
        out.line(format!(
            "  Part {} \"{part}\" instance {}: at {}, {} mm{turned}",
            put.part,
            put.instance,
            mm3(put.at[0]),
            mm3(put.at[1])
        ));
    }
    let notes: Vec<String> = nested.notes().iter().map(|m| m.english()).collect();
    for note in &notes {
        out.line(format!("Note: {note}"));
    }
    let share = if nested.length > 0.0 {
        (100.0 * nested.used / nested.length).round()
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
    out.set("used", json!(nested.used));
    out.set("length", json!(nested.length));
    out.set("used_pct", json!(share));
    out.set("board_fixed", json!(nested.board_fixed));
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
