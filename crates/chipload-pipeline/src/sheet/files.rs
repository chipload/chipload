// SPDX-License-Identifier: GPL-3.0-or-later
//! Each G-code file: its tool, its operations in a table, what to watch for.

use std::fmt::Write as _;
use std::path::Path;

use chipload_core::tolerances::CHAIN_TOL_MM;
use chipload_i18n::Message;
use chipload_job::{
    Direction, HoleMethod, HolesOp, Job, OpRef, Operation, Spindle, Tool, ToolChange, ToolType,
};
use chipload_toolpath::ToolGroup;
use chipload_toolpath::model::{MoveKind, Program};

use super::operations::{depth_text, entry_text, kind_text, tabs_text};
use super::stock::{xy_zero_message, z_zero_message};
use super::{Words, escape, len, len_u, listed, num};
use crate::estimate::duration_message;
use crate::{GcodeFile, RunResult};

pub(super) fn file_name(file: &GcodeFile, output: &Path) -> String {
    let path = file.path(output);
    path.file_name().map_or_else(
        || path.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    )
}

/// The tool groups a file holds: its own one, or all of them in one file with M6.
fn file_groups(result: &RunResult, index: usize) -> Vec<(usize, &ToolGroup)> {
    if result.files.len() == 1 {
        result.groups.iter().enumerate().collect()
    } else {
        result
            .groups
            .get(index)
            .map(|g| vec![(index, g)])
            .unwrap_or_default()
    }
}

/// A time for reading, escaped: `12 min 30 s`.
fn duration(w: Words, seconds: f64) -> String {
    w.text(&duration_message(seconds))
}

pub(super) fn file_section(
    html: &mut String,
    job: &Job,
    result: &RunResult,
    index: usize,
    name: &str,
    several: bool,
    w: Words,
) {
    let groups = file_groups(result, index);
    let seconds: f64 = groups
        .iter()
        .filter_map(|(g, _)| result.estimate.groups.get(*g))
        .sum();
    let _ = writeln!(
        html,
        "<section class=\"file\">\n<h2>{}. {} <span class=\"time\">{}</span></h2>",
        index + 1,
        escape(name),
        duration(w, seconds)
    );
    let one_file_m6 =
        !several && groups.len() > 1 && job.machine.tool_change == ToolChange::Program;
    for (n, (g, group)) in groups.iter().enumerate() {
        let tool = &group.tool;
        let what = tool_text(tool, w);
        let todo = if index == 0 && n == 0 {
            Message::new("sheet-load-tool")
                .arg("tool", tool.id)
                .arg("what", what.as_str())
                .arg("xy", xy_zero_message(job))
                .arg("z", z_zero_message(job))
        } else if one_file_m6 {
            Message::new("sheet-tool-change-m6")
                .arg("tool", tool.id)
                .arg("what", what.as_str())
        } else {
            Message::new("sheet-change-tool")
                .arg("tool", tool.id)
                .arg("what", what.as_str())
                .arg("z", z_zero_message(job))
        };
        if one_file_m6 {
            let _ = writeln!(
                html,
                "<h3>T{} <span class=\"time\">{}</span></h3>",
                tool.id,
                duration(w, result.estimate.groups.get(*g).copied().unwrap_or(0.0))
            );
        }
        let _ = writeln!(html, "<p class=\"do\">{}</p>", w.text(&todo));
        let line = match &tool.name {
            Some(name) => Message::new("sheet-tool-named").arg("name", escape(name)),
            None => Message::new("sheet-tool"),
        };
        let line = line
            .arg("tool", tool.id)
            .arg("what", escape(&what))
            .arg("rpm", num(tool.rpm.unwrap_or(0.0)))
            .arg("spindle", spindle_message(job.machine.spindle));
        let _ = writeln!(html, "<p>{}</p>", w.html(&line));
        operations_table(html, job, result, &group.operations, w);
        if let Some(text) = deepest_cut_text(job, &result.program, group, w) {
            let _ = writeln!(html, "<p>{}</p>", escape(&text));
        }
    }
    html.push_str("</section>\n");
}

/// "6 mm end mill, 2 flutes", "60° V-bit, 12.7 mm, tip 0.2 mm".
fn tool_text(tool: &Tool, w: Words) -> String {
    let diameter = len_u(tool.diameter);
    let mut parts = match tool.kind {
        ToolType::Endmill => {
            let mut parts = vec![Message::new("sheet-end-mill").arg("diameter", diameter)];
            if let Some(r) = tool.corner_radius.filter(|r| *r > 0.0) {
                parts.push(Message::new("sheet-corner-radius").arg("radius", len_u(r)));
            }
            parts
        }
        ToolType::Drill => vec![Message::new("sheet-drill").arg("diameter", diameter)],
        ToolType::Ballnose => match (tool.angle, tool.tip_diameter) {
            (Some(angle), Some(ball)) => vec![
                Message::new("sheet-tapered-ball")
                    .arg("angle", num(angle.to_degrees()))
                    .arg("diameter", diameter),
                Message::new("sheet-ball").arg("diameter", len_u(ball)),
            ],
            _ => vec![Message::new("sheet-ball-nose").arg("diameter", diameter)],
        },
        ToolType::Vbit => {
            let mut parts = vec![match tool.angle {
                Some(angle) => Message::new("sheet-vbit-angle")
                    .arg("angle", num(angle.to_degrees()))
                    .arg("diameter", diameter),
                None => Message::new("sheet-vbit").arg("diameter", diameter),
            }];
            if let Some(tip) = tool.tip_diameter.filter(|t| *t > 0.0) {
                parts.push(Message::new("sheet-tip").arg("diameter", len_u(tip)));
            }
            parts
        }
    };
    if let Some(n) = tool.flutes {
        parts.push(Message::new("sheet-flutes").arg("count", n));
    }
    listed(w, &parts)
}

fn spindle_message(spindle: Spindle) -> Message {
    Message::new(match spindle {
        Spindle::Cw => "sheet-spindle-cw",
        Spindle::Ccw => "sheet-spindle-ccw",
    })
}

fn operations_table(
    html: &mut String,
    job: &Job,
    result: &RunResult,
    operations: &[OpRef],
    w: Words,
) {
    let heading = |id: &'static str| w.text(&Message::new(id));
    let _ = writeln!(
        html,
        "<div class=\"scroll\"><table class=\"ops\">\n<tr><th>#</th><th>{}</th>\
         <th>{}</th><th>{}</th><th>{}</th><th>{}</th><th>{}</th></tr>",
        heading("sheet-column-operation"),
        heading("sheet-column-depth"),
        heading("sheet-column-feeds"),
        heading("sheet-column-entry"),
        heading("sheet-column-tabs"),
        heading("sheet-column-time"),
    );
    // With two parts or more, a heading row where each part starts (docs/specs/parts.md §6).
    let mut part_shown: Option<u64> = None;
    for &at in operations {
        // A rest pass is the pocket it makes (docs/specs/rest-passes.md §6).
        let Some(op) = job.resolve(at) else {
            continue;
        };
        let op = &op;
        if job.parts.len() > 1
            && part_shown != Some(op.part())
            && let Some((number, part)) = job
                .parts
                .iter()
                .enumerate()
                .find(|(_, p)| p.id == op.part())
        {
            part_shown = Some(part.id);
            let row = match part.count() {
                1 => Message::new("sheet-part-row"),
                n => Message::new("sheet-part-row-copies").arg("count", n),
            };
            let row = row
                .arg("number", number + 1)
                .arg("name", part.name.as_str());
            let _ = writeln!(
                html,
                "<tr class=\"part\"><th colspan=\"7\">{}</th></tr>",
                w.text(&row)
            );
        }
        let name = match op.name() {
            "" => w.say(&Message::new("sheet-operation").arg("number", at.to_string())),
            name => name.to_string(),
        };
        let seconds = result
            .estimate
            .operations
            .iter()
            .find(|(r, _)| *r == at)
            .map_or(0.0, |(_, s)| *s);
        let (feed, plunge) = feeds(op);
        // A drill only goes down (docs/specs/holes.md §2): its one feed.
        let feeds_text = match op {
            Operation::Holes(HolesOp {
                method: HoleMethod::Drill { .. },
                ..
            }) => w.text(&Message::new("sheet-feed-down").arg("feed", len(plunge))),
            _ => format!(
                "{} / {}",
                crate::units::length_number(feed, w.units),
                crate::units::length_number(plunge, w.units)
            ),
        };
        let load = match crate::chip_load::chip_load(job, op) {
            known @ crate::chip_load::ChipLoad::Mm(_) => {
                format!("<br>{}", w.text(&known.message()))
            }
            _ => String::new(),
        };
        let _ = writeln!(
            html,
            "<tr><td>{}</td><td><strong>{}</strong><br>{}</td><td>{}</td><td>{}{load}</td>\
             <td>{}</td><td>{}</td><td>{}</td></tr>",
            at,
            escape(&name),
            escape(&kind_text(op, &job.tools, w)),
            escape(&depth_text(op, &job.tools, w)),
            feeds_text,
            escape(&entry_text(op, w)),
            escape(&tabs_text(op, w)),
            duration(w, seconds)
        );
    }
    html.push_str("</table></div>\n");
}

fn feeds(op: &Operation) -> (f64, f64) {
    (op.feed(), op.plunge_feed())
}

pub(super) fn direction_message(direction: Direction) -> Message {
    Message::new(match direction {
        Direction::Climb => "sheet-climb",
        Direction::Conventional => "sheet-conventional",
    })
}

/// "Cuts down to Z-12.3: 12.3 mm below the stock top, 0.3 mm into the spoilboard."
fn deepest_cut_text(job: &Job, program: &Program, group: &ToolGroup, w: Words) -> Option<String> {
    let moves = program.moves.get(group.moves.clone())?;
    let lowest = moves
        .iter()
        .filter(|m| m.kind != MoveKind::Rapid)
        .filter_map(|m| m.z)
        .fold(None, |low: Option<f64>, z| {
            Some(low.map_or(z, |l| l.min(z)))
        })?;
    let top = job.stock().top_z();
    let depth = top - lowest;
    let mut parts = vec![
        Message::new("sheet-cuts-down")
            .arg("z", len(lowest))
            .arg("depth", len_u(depth)),
    ];
    let into = depth - job.stock().thickness;
    if into > CHAIN_TOL_MM {
        parts.push(Message::new("sheet-into-spoilboard").arg("depth", len_u(into)));
    } else if into.abs() <= CHAIN_TOL_MM {
        parts.push(Message::new("sheet-through"));
    }
    let cut = listed(w, &parts);
    Some(w.say(&Message::new("sheet-sticks-out").arg("cut", cut)))
}

/// The notes and checker warnings, in the sheet's language and units
/// (docs/specs/inch-units.md §5.1).
pub(super) fn watch_out(html: &mut String, result: &RunResult, w: Words) {
    let mut items: Vec<String> = Vec::new();
    if let Some(note) = &result.post.note {
        // The post's note is its own words, as its config file writes them.
        items.push(
            w.say(
                &Message::new("sheet-post-note")
                    .arg("post", result.post.name.as_str())
                    .arg("note", note.as_str()),
            ),
        );
    }
    items.extend(result.notes.iter().map(|n| w.say(&n.message)));
    items.extend(
        result
            .check_warnings
            .iter()
            .map(|c| w.say(&Message::new("sheet-check").arg("finding", c.text_message.clone()))),
    );
    let _ = writeln!(
        html,
        "<section class=\"warn\">\n<h2>{}</h2>",
        w.text(&Message::new("sheet-watch-out"))
    );
    if items.is_empty() {
        let _ = writeln!(
            html,
            "<p>{}</p>",
            w.text(&Message::new("sheet-nothing-reported"))
        );
    } else {
        html.push_str("<ul>\n");
        for item in items {
            let _ = writeln!(html, "<li>{}</li>", escape(item.trim()));
        }
        html.push_str("</ul>\n");
    }
    html.push_str("</section>\n");
}
