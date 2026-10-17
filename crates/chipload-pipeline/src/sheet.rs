// SPDX-License-Identifier: GPL-3.0-or-later
//! The setup sheet for the operator (docs/specs/setup-sheet.md): one HTML page, written next
//! to the G-code, saying what stock to load, where the zero is, which tool goes in for which
//! file and what to watch for. Made from the checked run, so it can't disagree with the
//! programs. ASCII only (names are written as character references) and no date, so the same
//! run gives the same sheet.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use chipload_core::Point;
use chipload_core::chain::Contour;
use chipload_core::text::trimmed;
use chipload_core::tolerances::CHAIN_TOL_MM;
use chipload_job::{
    Corners, Direction, Entry, HoleMethod, HolesOp, Job, OpRef, Operation, PocketOp, Side, Spindle,
    Tool, ToolChange, ToolType, Units, WorkZero, ZZero, find_tool,
};
use chipload_toolpath::ToolGroup;
use chipload_toolpath::model::{MoveKind, Program};
use chipload_toolpath::select::{select_contours, select_paths};

use crate::estimate::{duration_text, estimate_lines};
use crate::{GcodeFile, RunResult, copies, write_atomically};

/// What the sheet says about where it comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SheetInfo {
    /// The project's file name, or what the GUI calls an unsaved one.
    pub project: String,
    /// `chipload 0.13.0`.
    pub written_by: String,
    /// What lengths and feeds are written in (docs/specs/inch-units.md §5.1).
    pub units: Units,
}

/// `part-sheet.html` next to `part.nc` (§4, decision 7).
pub fn sheet_path(output: &Path) -> PathBuf {
    let stem = output
        .file_stem()
        .map_or(String::new(), |s| s.to_string_lossy().into_owned());
    output.with_file_name(format!("{stem}-sheet.html"))
}

/// Write the sheet of a run whose G-code went to `output`, next to it. Returns its path.
pub fn write_setup_sheet(
    job: &Job,
    result: &RunResult,
    output: &Path,
    info: &SheetInfo,
) -> std::io::Result<PathBuf> {
    let path = sheet_path(output);
    write_atomically(&path, &setup_sheet(job, result, output, info))?;
    Ok(path)
}

/// The sheet of a run whose G-code is (or will be) written to `output`, as HTML.
pub fn setup_sheet(job: &Job, result: &RunResult, output: &Path, info: &SheetInfo) -> String {
    let mut html = String::new();
    head(&mut html, info);
    header(&mut html, job, result, info);
    setup(&mut html, job, info.units);
    picture(&mut html, job, result);
    let names: Vec<String> = result.files.iter().map(|f| file_name(f, output)).collect();
    let several = result.files.len() > 1;
    for (i, name) in names.iter().enumerate() {
        file_section(&mut html, job, result, i, name, several, info.units);
    }
    watch_out(&mut html, result);
    let _ = writeln!(html, "<section class=\"total\">\n<h2>Time</h2>");
    for line in estimate_lines(job, &result.estimate, several) {
        let _ = writeln!(html, "<p>{}</p>", escape(line.trim_start()));
    }
    html.push_str("</section>\n</body>\n</html>\n");
    html
}

const STYLE: &str = "\
body { font-family: system-ui, -apple-system, \"Segoe UI\", sans-serif; color: #111; \
background: #fff; margin: 24px auto; max-width: 900px; padding: 0 16px; line-height: 1.4; }
h1 { font-size: 1.5em; margin: 0; }
h2 { font-size: 1.15em; margin: 1.2em 0 0.4em; border-bottom: 1px solid #999; }
h3 { font-size: 1em; margin: 0.8em 0 0.3em; }
.meta { color: #444; margin: 0.2em 0 0; }
table { border-collapse: collapse; width: 100%; font-size: 0.92em; }
th, td { text-align: left; vertical-align: top; padding: 3px 6px; border-bottom: 1px solid #ddd; }
th { font-weight: 600; }
table.facts th { width: 9em; }
.time { float: right; font-weight: normal; }
.do { font-weight: 600; margin: 0.4em 0; }
.legend span { margin-right: 1.2em; white-space: nowrap; }
figure { margin: 1em 0; }
svg.picture { width: 100%; max-height: 520px; border: 1px solid #ccc; }
.warn li { margin: 0.2em 0; }
.scroll { overflow-x: auto; }
.total p { margin: 0.15em 0; }
.total p + p { padding-left: 1em; }
@media print {
  body { margin: 0; max-width: none; font-size: 10pt; }
  svg.picture { max-height: 110mm; }
  section.file { break-inside: avoid; }
}
";

fn head(html: &mut String, info: &SheetInfo) {
    let _ = writeln!(
        html,
        "<!doctype html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n\
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n\
         <title>Setup sheet: {}</title>\n<style>\n{STYLE}</style>\n</head>\n<body>",
        escape(&info.project)
    );
}

fn header(html: &mut String, job: &Job, result: &RunResult, info: &SheetInfo) {
    let machine = match &job.machine.name {
        Some(name) => format!("{} &middot; ", escape(name)),
        None => String::new(),
    };
    let _ = writeln!(
        html,
        "<header>\n<h1>Setup sheet: {}</h1>\n<p class=\"meta\">{machine}post {} &middot; \
         written by {} &middot; lengths in {}, feeds in {}{}</p>\n</header>",
        escape(&info.project),
        escape(&result.post.name),
        escape(&info.written_by),
        crate::units::length_unit(info.units),
        crate::units::feed_unit(info.units),
        // The program's own units, when they aren't the sheet's (docs/specs/inch-units.md §3).
        match job.machine.gcode_units {
            chipload_job::Units::Mm => "",
            chipload_job::Units::Inch => " &middot; <strong>G-code in inches (G20)</strong>",
        }
    );
}

fn setup(html: &mut String, job: &Job, u: Units) {
    let stock = &job.stock;
    html.push_str("<section class=\"setup\">\n<h2>Stock and zero</h2>\n<table class=\"facts\">\n");
    let _ = writeln!(
        html,
        "<tr><th>Stock</th><td>{} &times; {}, {} thick</td></tr>",
        len(u, stock.x),
        len_u(u, stock.y),
        len_u(u, stock.thickness)
    );
    let _ = writeln!(
        html,
        "<tr><th>X0 Y0</th><td>{}</td></tr>",
        escape(&xy_zero_text_in(job, u))
    );
    let _ = writeln!(
        html,
        "<tr><th>Z0</th><td>{}</td></tr>",
        escape(z_zero_text(job))
    );
    if let Some(text) = copies::copies_text_in(job, u) {
        let _ = writeln!(html, "<tr><th>Copies</th><td>{}</td></tr>", escape(&text));
    }
    // Several parts: each with its copies (docs/specs/parts.md §6).
    for line in copies::parts_lines_in(job, u) {
        let _ = writeln!(html, "<tr><th>Part</th><td>{}</td></tr>", escape(&line));
    }
    // Several drawings: each with where it was put (docs/specs/drawing-sources.md §7).
    let d = &job.drawing;
    if d.dxfs.len() + d.images.len() + d.texts.len() > 1 {
        for dxf in &job.drawing.dxfs {
            let _ = writeln!(
                html,
                "<tr><th>DXF {}</th><td>{} ({}), {}</td></tr>",
                dxf.id,
                escape(&dxf.name),
                escape(&dxf.file),
                escape(&crate::placement::placement_text(dxf.placement))
            );
        }
        for image in &job.drawing.images {
            let _ = writeln!(
                html,
                "<tr><th>Image {}</th><td>{} on layer {}, {} wide at X{} Y{}</td></tr>",
                image.id,
                escape(&image.file),
                escape(&image.layer),
                len_u(u, image.width),
                len(u, image.x),
                len(u, image.y)
            );
        }
        for text in &job.drawing.texts {
            let _ = writeln!(
                html,
                "<tr><th>Text {}</th><td>{} in {} on layer {}, {} high at X{} Y{}</td></tr>",
                text.id,
                escape(&crate::texts::text_label(text)),
                escape(&text.font),
                escape(&text.layer),
                len_u(u, text.height),
                len(u, text.x),
                len(u, text.y)
            );
        }
    }
    html.push_str("</table>\n</section>\n");
}

/// Where X0 Y0 is, in words measured on the stock (§2.3).
pub fn xy_zero_text(job: &Job) -> String {
    xy_zero_text_in(job, Units::Mm)
}

/// As [`xy_zero_text`], its distances in `u`.
pub fn xy_zero_text_in(job: &Job, u: Units) -> String {
    if let WorkZero::Stock { corner } = job.machine.work_zero {
        return match corner {
            chipload_job::StockCorner::Center => "the centre of the stock".into(),
            corner => format!("the stock's {}", corner.name()),
        };
    }
    let ((x0, y0), (x1, y1)) = job.stock.xy_bounds();
    let (zx, zy) = job.work_zero_xy();
    let (dx, dy) = (zx - x0, zy - y0);
    let mut parts = Vec::new();
    if dx.abs() > CHAIN_TOL_MM {
        let side = if dx > 0.0 { "right of" } else { "left of" };
        parts.push(format!("{} {side}", len_u(u, dx.abs())));
    }
    if dy.abs() > CHAIN_TOL_MM {
        let side = if dy > 0.0 { "above" } else { "below" };
        parts.push(format!("{} {side}", len_u(u, dy.abs())));
    }
    let mut text = if parts.is_empty() {
        "the stock's bottom left corner".to_string()
    } else {
        format!("{} the stock's bottom left corner", parts.join(" and "))
    };
    let outside = zx < x0 - CHAIN_TOL_MM
        || zx > x1 + CHAIN_TOL_MM
        || zy < y0 - CHAIN_TOL_MM
        || zy > y1 + CHAIN_TOL_MM;
    if outside {
        text.push_str(" (outside the stock)");
    }
    text
}

fn z_zero_text(job: &Job) -> &'static str {
    match job.stock.z_zero {
        ZZero::Top => "the top of the stock",
        ZZero::Bottom => "the bottom of the stock (the spoilboard)",
    }
}

fn file_name(file: &GcodeFile, output: &Path) -> String {
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

fn file_section(
    html: &mut String,
    job: &Job,
    result: &RunResult,
    index: usize,
    name: &str,
    several: bool,
    u: Units,
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
        duration_text(seconds)
    );
    let one_file_m6 =
        !several && groups.len() > 1 && job.machine.tool_change == ToolChange::Program;
    for (n, (g, group)) in groups.iter().enumerate() {
        let tool = &group.tool;
        let what = tool_text(tool, u);
        let todo = if index == 0 && n == 0 {
            format!(
                "Load tool T{} ({what}). Set X0 Y0 at {} and Z0 on {}.",
                tool.id,
                xy_zero_text_in(job, u),
                z_zero_text(job)
            )
        } else if one_file_m6 {
            format!(
                "Tool change (M6) to T{} ({what}), as your controller handles it.",
                tool.id
            )
        } else {
            format!(
                "Change to tool T{} ({what}). Set Z0 again with this tool on {}; keep X0 Y0.",
                tool.id,
                z_zero_text(job)
            )
        };
        if one_file_m6 {
            let _ = writeln!(
                html,
                "<h3>T{} <span class=\"time\">{}</span></h3>",
                tool.id,
                duration_text(result.estimate.groups.get(*g).copied().unwrap_or(0.0))
            );
        }
        let _ = writeln!(html, "<p class=\"do\">{}</p>", escape(&todo));
        let _ = writeln!(
            html,
            "<p>Tool T{}: {}{}, {} rpm {}.</p>",
            tool.id,
            tool.name
                .as_ref()
                .map_or(String::new(), |n| format!("&ldquo;{}&rdquo;, ", escape(n))),
            escape(&what),
            num(tool.rpm.unwrap_or(0.0)),
            spindle_text(job.machine.spindle)
        );
        operations_table(html, job, result, &group.operations, u);
        if let Some(text) = deepest_cut_text(job, &result.program, group, u) {
            let _ = writeln!(html, "<p>{}</p>", escape(&text));
        }
    }
    html.push_str("</section>\n");
}

fn tool_text(tool: &Tool, u: Units) -> String {
    let flutes = tool.flutes.map_or(String::new(), |n| {
        format!(", {n} flute{}", if n == 1 { "" } else { "s" })
    });
    match tool.kind {
        ToolType::Endmill => format!("{} end mill{flutes}", len_u(u, tool.diameter)),
        ToolType::Drill => format!("{} drill{flutes}", len_u(u, tool.diameter)),
        ToolType::Vbit => {
            let angle = tool
                .angle
                .map_or(String::new(), |a| format!("{}\u{b0} ", num(a.to_degrees())));
            let tip = tool
                .tip_diameter
                .filter(|t| *t > 0.0)
                .map_or(String::new(), |t| format!(", tip {}", len_u(u, t)));
            format!("{angle}V-bit, {}{tip}{flutes}", len_u(u, tool.diameter))
        }
    }
}

fn spindle_text(spindle: Spindle) -> &'static str {
    match spindle {
        Spindle::Cw => "clockwise (M3)",
        Spindle::Ccw => "counter-clockwise (M4)",
    }
}

fn operations_table(
    html: &mut String,
    job: &Job,
    result: &RunResult,
    operations: &[OpRef],
    u: Units,
) {
    html.push_str(
        "<div class=\"scroll\"><table class=\"ops\">\n<tr><th>#</th><th>Operation</th>\
         <th>Depth</th><th>Feed / plunge</th><th>Entry</th><th>Tabs</th><th>Time</th></tr>\n",
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
            let copies = match part.count() {
                1 => String::new(),
                n => format!(", {n} copies"),
            };
            let _ = writeln!(
                html,
                "<tr class=\"part\"><th colspan=\"7\">Part {}: {}{copies}</th></tr>",
                number + 1,
                escape(&part.name)
            );
        }
        let name = match op.name() {
            "" => format!("Operation {at}"),
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
            }) => format!("{} down", len(u, plunge)),
            _ => format!("{} / {}", len(u, feed), len(u, plunge)),
        };
        let load = match crate::chip_load::chip_load(job, op) {
            known @ crate::chip_load::ChipLoad::Mm(_) => format!("<br>{}", known.text_in(u)),
            _ => String::new(),
        };
        let _ = writeln!(
            html,
            "<tr><td>{}</td><td><strong>{}</strong><br>{}</td><td>{}</td><td>{}{load}</td>\
             <td>{}</td><td>{}</td><td>{}</td></tr>",
            at,
            escape(&name),
            escape(&kind_text(op, &job.tools, u)),
            escape(&depth_text(op, u)),
            feeds_text,
            escape(&entry_text(op, u)),
            escape(&tabs_text(op, u)),
            duration_text(seconds)
        );
    }
    html.push_str("</table></div>\n");
}

fn feeds(op: &Operation) -> (f64, f64) {
    match op {
        Operation::Profile(p) => (p.feed, p.plunge_feed),
        Operation::Pocket(p) => (p.feed, p.plunge_feed),
        Operation::Vcarve(v) => (v.feed, v.plunge_feed),
        Operation::Path(p) => (p.feed, p.plunge_feed),
        Operation::Holes(h) => (h.feed, h.plunge_feed),
        Operation::Surface(f) => (f.feed, f.plunge_feed),
    }
}

fn direction_text(direction: Direction) -> &'static str {
    match direction {
        Direction::Climb => "climb",
        Direction::Conventional => "conventional",
    }
}

/// "Profile, outside, climb, leaves 0.2 mm" (a negative clearance).
fn kind_text(op: &Operation, tools: &[Tool], u: Units) -> String {
    let mut text = match op {
        Operation::Profile(p) => {
            let side = match p.side {
                Side::Outside => "outside",
                Side::Inside => "inside",
                Side::On => "on the line",
            };
            format!("Profile, {side}, {}", direction_text(p.direction))
        }
        Operation::Pocket(p) => {
            format!(
                "Pocket, rings, {}, stepover {} %",
                direction_text(p.direction),
                num(p.stepover * 100.0)
            )
        }
        Operation::Vcarve(_) => "V-carve".to_string(),
        Operation::Path(_) => "Follow path, on the line".to_string(),
        Operation::Holes(h) => match h.method {
            HoleMethod::Drill { .. } => "Holes, drilled".to_string(),
            HoleMethod::Bore { direction, .. } => {
                format!("Holes, helical bore, {}", direction_text(direction))
            }
        },
        // docs/specs/surfacing.md §3, §5.
        Operation::Surface(f) => {
            let past = tools
                .iter()
                .find(|t| t.id == f.tool)
                .map_or(String::new(), |t| {
                    format!(
                        "; runs {} past the stock ends: keep clamps clear",
                        len_u(u, t.diameter / 2.0)
                    )
                });
            format!(
                "Surface, the whole stock, back and forth (climb and conventional), \
                 stepover {} %{past}",
                num(f.stepover * 100.0)
            )
        }
    };
    // Positive clearance cuts into the material that is kept, negative leaves some
    // (docs/specs/depths-and-clearance.md §3).
    let clearance = op.clearance();
    if clearance > 0.0 {
        let _ = write!(text, ", cuts {} past the line", len_u(u, clearance));
    } else if clearance < 0.0 {
        let _ = write!(text, ", leaves {}", len_u(u, -clearance));
    }
    // docs/specs/rest-machining.md §6.
    if let Operation::Pocket(PocketOp {
        rest: Some(rest), ..
    }) = op
    {
        let _ = write!(text, ", rest after T{}", rest.after_tool);
        if let Some(tool) = find_tool(tools, rest.after_tool) {
            let _ = write!(text, " ({})", len_u(u, tool.diameter));
        }
    }
    // docs/specs/dogbones.md §7.
    if let Some(corners) = op.corners() {
        let style = match corners {
            Corners::Dogbone { .. } => "dogbones",
            Corners::Tbone { .. } => "T-bones",
        };
        let _ = write!(
            text,
            ", {style} at corners up to {}\u{b0}",
            num(corners.max_angle().to_degrees())
        );
    }
    text
}

/// "0 to 6, 2 per pass", with a profile's finishing stock and skin.
fn depth_text(op: &Operation, u: Units) -> String {
    let (start, end) = op.depths();
    let step = match op {
        Operation::Profile(p) => Some(p.step_down),
        Operation::Pocket(p) => Some(p.step_down),
        Operation::Vcarve(v) => v.step_down,
        Operation::Path(p) => Some(p.step_down),
        Operation::Holes(_) => None,
        Operation::Surface(f) => Some(f.step_down),
    };
    let passes = match op {
        // docs/specs/holes.md §6: a drill's depth is its point's.
        Operation::Holes(HolesOp {
            method: HoleMethod::Drill { peck },
            ..
        }) => {
            let pecks = match peck {
                Some(peck) if *peck < end - start => format!(", pecks {}", len_u(u, *peck)),
                _ => ", in one go".to_string(),
            };
            format!(" (the drill's point){pecks}")
        }
        Operation::Holes(HolesOp {
            method: HoleMethod::Bore { step_down, .. },
            ..
        }) => format!(", {} per turn", len(u, *step_down)),
        _ => match step {
            Some(step) if step > 0.0 && step < end - start => {
                format!(", {} per pass", len(u, step))
            }
            _ => ", one pass".to_string(),
        },
    };
    let mut text = format!("{} to {}{passes}", len(u, start), len(u, end));
    if let Operation::Profile(p) = op {
        // docs/specs/finish-and-skin.md §6: "finish 0.3 mm, skin 0.2 mm".
        if let Some(finish) = p.finish {
            let _ = write!(text, ", finish {}", len_u(u, finish.stock));
        }
        if let Some(skin) = p.skin {
            let _ = write!(text, ", skin {}", len_u(u, skin.thickness));
        }
    }
    text
}

fn entry_text(op: &Operation, u: Units) -> String {
    if let Operation::Surface(_) = op {
        return "down beside the stock".to_string();
    }
    if let Operation::Holes(h) = op {
        return match h.method {
            HoleMethod::Drill { .. } => "straight down".to_string(),
            HoleMethod::Bore { ramp_angle, .. } => {
                format!("helix {}\u{b0}", num(ramp_angle.to_degrees()))
            }
        };
    }
    let mut text = match op.entry() {
        Entry::Plunge {} => "plunge".to_string(),
        Entry::Ramp { ramp_angle } => format!("ramp {}\u{b0}", num(ramp_angle.to_degrees())),
        Entry::Helix { radius, ramp_angle } => format!(
            "helix r {}, {}\u{b0}",
            len(u, radius),
            num(ramp_angle.to_degrees())
        ),
    };
    if let Some(lead) = op.lead() {
        let _ = write!(text, ", lead r {}", len(u, lead.radius));
    }
    text
}

fn tabs_text(op: &Operation, u: Units) -> String {
    match op {
        Operation::Profile(p) => match &p.tabs {
            Some(t) if t.count > 0 => {
                let mut text = format!(
                    "{} \u{d7} {} wide, {} high",
                    t.count,
                    len(u, t.width),
                    len(u, t.height)
                );
                if !t.is_straight() {
                    let degrees = trimmed(t.flank_angle.to_degrees(), 1);
                    let _ = write!(text, ", flanks {degrees}\u{b0}");
                }
                text
            }
            _ => "none".into(),
        },
        _ => "-".into(),
    }
}

/// "Cuts down to Z-12.3: 12.3 mm below the stock top, 0.3 mm into the spoilboard."
fn deepest_cut_text(job: &Job, program: &Program, group: &ToolGroup, u: Units) -> Option<String> {
    let moves = program.moves.get(group.moves.clone())?;
    let lowest = moves
        .iter()
        .filter(|m| m.kind != MoveKind::Rapid)
        .filter_map(|m| m.z)
        .fold(None, |low: Option<f64>, z| {
            Some(low.map_or(z, |l| l.min(z)))
        })?;
    let top = job.stock.top_z();
    let depth = top - lowest;
    let mut text = format!(
        "Cuts down to Z{}: {} below the stock top",
        len(u, lowest),
        len_u(u, depth)
    );
    let into = depth - job.stock.thickness;
    if into > CHAIN_TOL_MM {
        let _ = write!(text, ", {} into the spoilboard", len_u(u, into));
    } else if into.abs() <= CHAIN_TOL_MM {
        text.push_str(", through the stock");
    }
    text.push_str(". Check that the tool sticks out far enough.");
    Some(text)
}

fn watch_out(html: &mut String, result: &RunResult) {
    let mut items: Vec<String> = Vec::new();
    if let Some(note) = &result.post.note {
        items.push(format!("Post {}: {note}", result.post.name));
    }
    items.extend(result.notes.iter().map(|n| n.text.clone()));
    items.extend(
        result
            .check_warnings
            .iter()
            .map(|w| format!("Check: {}", w.message)),
    );
    html.push_str("<section class=\"warn\">\n<h2>Watch out</h2>\n");
    if items.is_empty() {
        html.push_str("<p>Nothing was reported.</p>\n");
    } else {
        html.push_str("<ul>\n");
        for item in items {
            let _ = writeln!(html, "<li>{}</li>", escape(item.trim()));
        }
        html.push_str("</ul>\n");
    }
    html.push_str("</section>\n");
}

/// Colours and dashes of the files (or of the tools in one file), told apart in black and
/// white too (§5, decision 4).
const COLOURS: [&str; 6] = [
    "#1f5fbf", "#c0392b", "#1e8449", "#7d3c98", "#b9770e", "#117a8b",
];
const DASHES: [&str; 4] = ["", "6 3", "2 2", "9 3 2 3"];

/// The top view: stock, parts, each file's cutting moves, tabs and the X0 Y0 mark (§2.4).
fn picture(html: &mut String, job: &Job, result: &RunResult) {
    let ((sx0, sy0), (sx1, sy1)) = job.stock.xy_bounds();
    let mut bounds = Bounds::new((sx0, sy0));
    bounds.add((sx1, sy1));
    let zero = job.work_zero_xy();
    bounds.add(zero);
    for m in &result.program.moves {
        if let (Some(x), Some(y)) = (m.x, m.y) {
            bounds.add((x, y));
        }
    }
    let size = (bounds.x1 - bounds.x0).max(bounds.y1 - bounds.y0).max(1.0);
    let margin = size * 0.06;
    let _ = writeln!(
        html,
        "<figure>\n<svg class=\"picture\" xmlns=\"http://www.w3.org/2000/svg\" \
         viewBox=\"{} {} {} {}\" preserveAspectRatio=\"xMidYMid meet\">",
        svg_num(bounds.x0 - margin),
        svg_num(-(bounds.y1 + margin)),
        svg_num(bounds.x1 - bounds.x0 + 2.0 * margin),
        svg_num(bounds.y1 - bounds.y0 + 2.0 * margin)
    );
    let _ = writeln!(
        html,
        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"#f4efe6\" stroke=\"#555\" \
         stroke-width=\"1\" vector-effect=\"non-scaling-stroke\"/>",
        svg_num(sx0),
        svg_num(-sy1),
        svg_num(sx1 - sx0),
        svg_num(sy1 - sy0)
    );
    // The drawing, at every copy of every part: with one part the whole drawing as
    // before; with several, each part's copies of the contours its operations cut.
    let mut parts = String::new();
    for (contour, shifts) in contour_copies(job, &result.contours) {
        let points = contour.points();
        for (dx, dy) in shifts {
            let moved: Vec<Point> = points.iter().map(|p| (p.0 + dx, p.1 + dy)).collect();
            path_data(&mut parts, &moved, contour.closed);
        }
    }
    let _ = writeln!(
        html,
        "<path d=\"{}\" fill=\"none\" stroke=\"#999\" stroke-width=\"1\" \
         vector-effect=\"non-scaling-stroke\"/>",
        parts.trim_end()
    );
    for (i, group) in result.groups.iter().enumerate() {
        let d = cutting_path(&result.program, group);
        let dash = match DASHES[i % DASHES.len()] {
            "" => String::new(),
            dash => format!(" stroke-dasharray=\"{dash}\""),
        };
        let _ = writeln!(
            html,
            "<path d=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"1.3\"{dash} \
             stroke-linejoin=\"round\" vector-effect=\"non-scaling-stroke\"/>",
            d.trim_end(),
            COLOURS[i % COLOURS.len()]
        );
    }
    let mut tabs = String::new();
    for tab in &result.tabs {
        path_data(&mut tabs, tab, false);
    }
    if !tabs.is_empty() {
        let _ = writeln!(
            html,
            "<path d=\"{}\" fill=\"none\" stroke=\"#e67e22\" stroke-width=\"5\" \
             stroke-linecap=\"round\" vector-effect=\"non-scaling-stroke\"/>",
            tabs.trim_end()
        );
    }
    zero_mark(html, zero, size);
    html.push_str("</svg>\n<figcaption class=\"legend\">");
    let several = result.files.len() > 1;
    for (i, group) in result.groups.iter().enumerate() {
        let label = if several {
            format!("File {} (T{})", i + 1, group.tool.id)
        } else {
            format!("T{}", group.tool.id)
        };
        let dash = match DASHES[i % DASHES.len()] {
            "" => String::new(),
            dash => format!(" stroke-dasharray=\"{dash}\""),
        };
        let _ = write!(
            html,
            "<span><svg width=\"36\" height=\"10\"><line x1=\"0\" y1=\"5\" x2=\"36\" y2=\"5\" \
             stroke=\"{}\" stroke-width=\"2\"{dash}/></svg> {label}</span>",
            COLOURS[i % COLOURS.len()]
        );
    }
    if !result.tabs.is_empty() {
        html.push_str(
            "<span><svg width=\"36\" height=\"10\"><line x1=\"4\" y1=\"5\" x2=\"32\" y2=\"5\" \
             stroke=\"#e67e22\" stroke-width=\"5\" stroke-linecap=\"round\"/></svg> tabs</span>",
        );
    }
    html.push_str(
        "<span>Grey: the parts. Cutting moves only, no rapids.</span></figcaption>\n</figure>\n",
    );
}

/// The X0 Y0 mark: a circle with arrows along +X and +Y.
fn zero_mark(html: &mut String, (zx, zy): (f64, f64), size: f64) {
    let arrow = size * 0.08;
    let head = arrow * 0.25;
    let font = size * 0.03;
    let (x, y) = (zx, -zy);
    let _ = writeln!(
        html,
        "<g stroke=\"#000\" stroke-width=\"2\" vector-effect=\"non-scaling-stroke\" fill=\"none\">\
         <circle cx=\"{x0}\" cy=\"{y0}\" r=\"{r}\"/>\
         <path d=\"M{x0} {y0} H{xe} M{xh} {yu} L{xe} {y0} L{xh} {yd} \
         M{x0} {y0} V{ye} M{xl} {yh} L{x0} {ye} L{xr} {yh}\"/></g>\n\
         <g font-size=\"{font}\" font-family=\"sans-serif\" fill=\"#000\">\
         <text x=\"{tx}\" y=\"{ty}\">X+</text><text x=\"{ux}\" y=\"{uy}\">Y+</text>\
         <text x=\"{zx_}\" y=\"{zy_}\">X0 Y0</text></g>",
        x0 = svg_num(x),
        y0 = svg_num(y),
        r = svg_num(head * 0.6),
        xe = svg_num(x + arrow),
        xh = svg_num(x + arrow - head),
        yu = svg_num(y - head * 0.5),
        yd = svg_num(y + head * 0.5),
        ye = svg_num(y - arrow),
        yh = svg_num(y - arrow + head),
        xl = svg_num(x - head * 0.5),
        xr = svg_num(x + head * 0.5),
        font = svg_num(font),
        tx = svg_num(x + arrow + font * 0.3),
        ty = svg_num(y + font * 0.35),
        ux = svg_num(x - font * 0.5),
        uy = svg_num(y - arrow - font * 0.4),
        zx_ = svg_num(x + font * 0.4),
        zy_ = svg_num(y + font * 1.3),
    );
}

/// A group's cutting moves as path data: a new subpath after each rapid.
fn cutting_path(program: &Program, group: &ToolGroup) -> String {
    let mut d = String::new();
    let (mut x, mut y) = (None, None);
    let mut drawing = false;
    for (i, m) in program.moves.iter().enumerate() {
        let from = (x, y);
        x = m.x.or(x);
        y = m.y.or(y);
        if !group.moves.contains(&i) || m.kind == MoveKind::Rapid {
            drawing = false;
            continue;
        }
        let (Some(tx), Some(ty)) = (x, y) else {
            continue;
        };
        if (Some(tx), Some(ty)) == (from.0, from.1) {
            continue; // straight down or up: nothing to see from above
        }
        if !drawing {
            if let (Some(fx), Some(fy)) = from {
                let _ = write!(d, "M{} {} ", svg_num(fx), svg_num(-fy));
            } else {
                let _ = write!(d, "M{} {} ", svg_num(tx), svg_num(-ty));
            }
            drawing = true;
        }
        let _ = write!(d, "L{} {} ", svg_num(tx), svg_num(-ty));
    }
    d
}

/// Each contour with the shifts it is drawn at: every copy of the parts that cut it
/// (docs/specs/parts.md §6). With one part, every contour at every copy, as before parts.
/// A contour no part cuts is drawn once, where it is.
pub fn contour_copies<'a>(job: &Job, contours: &'a [Contour]) -> Vec<(&'a Contour, Vec<Point>)> {
    let shifts_of = |part: &chipload_job::Part| -> Vec<Point> {
        part.in_cut_order().into_iter().map(|(_, s)| s).collect()
    };
    if let [part] = job.parts.as_slice() {
        let shifts = shifts_of(part);
        return contours.iter().map(|c| (c, shifts.clone())).collect();
    }
    let mut cut_by: Vec<Vec<Point>> = vec![Vec::new(); contours.len()];
    for part in &job.parts {
        let mut ids: Vec<usize> = Vec::new();
        for op in job
            .operations
            .iter()
            .filter(|op| op.part() == part.id && op.selects_shapes())
        {
            let picked = match op {
                Operation::Path(_) | Operation::Holes(_) => select_paths(contours, op.select(), ""),
                _ => select_contours(contours, op.select(), ""),
            };
            if let Ok(selection) = picked {
                ids.extend(selection.contours.iter().map(|c| c.id));
            }
        }
        for (k, contour) in contours.iter().enumerate() {
            if ids.contains(&contour.id) {
                for shift in shifts_of(part) {
                    if !cut_by[k].contains(&shift) {
                        cut_by[k].push(shift);
                    }
                }
            }
        }
    }
    contours
        .iter()
        .zip(cut_by)
        .map(|(c, shifts)| {
            if shifts.is_empty() {
                (c, vec![(0.0, 0.0)])
            } else {
                (c, shifts)
            }
        })
        .collect()
}

fn path_data(d: &mut String, points: &[Point], closed: bool) {
    for (i, p) in points.iter().enumerate() {
        let _ = write!(
            d,
            "{}{} {} ",
            if i == 0 { "M" } else { "L" },
            svg_num(p.0),
            svg_num(-p.1)
        );
    }
    if closed && points.len() > 2 {
        d.push_str("Z ");
    }
}

struct Bounds {
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
}

impl Bounds {
    fn new((x, y): Point) -> Bounds {
        Bounds {
            x0: x,
            y0: y,
            x1: x,
            y1: y,
        }
    }

    fn add(&mut self, (x, y): Point) {
        if x.is_finite() && y.is_finite() {
            self.x0 = self.x0.min(x);
            self.y0 = self.y0.min(y);
            self.x1 = self.x1.max(x);
            self.y1 = self.y1.max(y);
        }
    }
}

/// A length for reading: at most 3 decimals, no trailing zeros, never `-0`.
fn num(value: f64) -> String {
    trimmed(value, 3)
}

/// A length (or feed) in `u`, the number only.
fn len(u: Units, mm: f64) -> String {
    crate::units::length_number(mm, u)
}

/// A length in `u` with its unit: `6 mm`, `0.25 in`.
fn len_u(u: Units, mm: f64) -> String {
    crate::units::length_text(mm, u)
}

/// A coordinate in the picture: 2 decimals are plenty on paper.
fn svg_num(value: f64) -> String {
    trimmed(value, 2)
}

/// Text for HTML: the five special characters and everything outside ASCII as character
/// references (the sheet is written as ASCII, like the G-code).
pub fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            c if c.is_ascii() && !c.is_ascii_control() => out.push(c),
            c if c == '\n' || c == '\t' => out.push(c),
            c => {
                let _ = write!(out, "&#x{:x};", c as u32);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_read_naturally() {
        assert_eq!(num(6.0), "6");
        assert_eq!(num(0.5), "0.5");
        assert_eq!(num(-0.0001), "0");
        assert_eq!(num(12.3456), "12.346");
        assert_eq!(num(3.0_f64.to_radians().to_degrees()), "3");
        assert_eq!(svg_num(-0.001), "0");
    }

    #[test]
    fn text_is_escaped_to_ascii() {
        assert_eq!(
            escape("<a & \"b\"> 'c' \u{e5}\u{b0}"),
            "&lt;a &amp; &quot;b&quot;&gt; &#39;c&#39; &#xe5;&#xb0;"
        );
    }

    #[test]
    fn the_sheet_sits_next_to_the_gcode() {
        assert_eq!(
            sheet_path(Path::new("out/part.nc")),
            Path::new("out/part-sheet.html")
        );
    }
}
