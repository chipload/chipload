// SPDX-License-Identifier: GPL-3.0-or-later
//! The setup sheet for the operator (docs/specs/setup-sheet.md): one HTML page, written next
//! to the G-code, saying what stock to load, where the zero is, which tool goes in for which
//! file and what to watch for. Made from the checked run, so it can't disagree with the
//! programs. ASCII only (names are written as character references) and no date, so the same
//! run gives the same sheet. Its words are texts of `sheet.ftl`, in the sheet's language
//! (docs/specs/translations.md step 3); names, numbers and G-code are never translated.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use chipload_core::Point;
use chipload_core::chain::Contour;
use chipload_core::text::trimmed;
use chipload_core::tolerances::CHAIN_TOL_MM;
use chipload_i18n::{Digits, Language, Message, Value};
use chipload_job::{
    Corners, Direction, Entry, HoleMethod, HolesOp, Job, OpRef, Operation, PocketOp, Side, Spindle,
    StockCorner, Tool, ToolChange, ToolType, Units, WorkZero, ZZero, find_tool,
};
use chipload_toolpath::ToolGroup;
use chipload_toolpath::model::{MoveKind, Program};
use chipload_toolpath::select::{select_contours, select_paths};

use crate::estimate::{duration_message, estimate_lines};
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
    /// What its words are written in.
    pub language: Language,
    /// The parts' names when only some of the project's parts were exported
    /// (docs/specs/part-export.md §4); empty for the whole project.
    pub parts: Vec<String>,
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
    let path = setup_sheet_path(job, output);
    write_atomically(&path, &setup_sheet(job, result, output, info))?;
    Ok(path)
}

/// Where the sheet of `job`'s run to `output` goes: `part-sheet.html`, or with several
/// setups the active one's, `part-S2-sheet.html` (docs/specs/setups.md §4).
pub fn setup_sheet_path(job: &Job, output: &Path) -> PathBuf {
    match job.active_setup_id().filter(|_| job.setups.len() > 1) {
        Some(id) => {
            let stem = output
                .file_stem()
                .map_or(String::new(), |s| s.to_string_lossy().into_owned());
            output.with_file_name(format!("{stem}-S{id}-sheet.html"))
        }
        None => sheet_path(output),
    }
}

/// The sheet's language and units, which its texts are written in.
#[derive(Debug, Clone, Copy)]
struct Words {
    lang: Language,
    units: Units,
}

impl Words {
    /// A text, as plain words.
    fn say(self, message: &Message) -> String {
        message.render(self.lang, self.units.into())
    }

    /// A text as HTML: escaped.
    fn text(self, message: &Message) -> String {
        escape(&self.say(message))
    }

    /// A text that is HTML itself (it holds an entity such as `&times;`), its arguments
    /// escaped by the caller: only what is outside ASCII becomes a reference.
    fn html(self, message: &Message) -> String {
        ascii(&self.say(message))
    }
}

/// Plain words joined with `, `: "Profile, outside, climb".
fn listed(w: Words, parts: &[Message]) -> String {
    let parts: Vec<String> = parts.iter().map(|m| w.say(m)).collect();
    parts.join(", ")
}

/// The sheet of a run whose G-code is (or will be) written to `output`, as HTML.
pub fn setup_sheet(job: &Job, result: &RunResult, output: &Path, info: &SheetInfo) -> String {
    let w = Words {
        lang: info.language,
        units: info.units,
    };
    let mut html = String::new();
    head(&mut html, job, info, w);
    header(&mut html, job, result, info, w);
    setup(&mut html, job, w);
    picture(&mut html, job, result, w);
    let names: Vec<String> = result.files.iter().map(|f| file_name(f, output)).collect();
    let several = result.files.len() > 1;
    for (i, name) in names.iter().enumerate() {
        file_section(&mut html, job, result, i, name, several, w);
    }
    watch_out(&mut html, result, w);
    let _ = writeln!(
        html,
        "<section class=\"total\">\n<h2>{}</h2>",
        w.text(&Message::new("sheet-time"))
    );
    // The estimate's lines in the sheet's units, as everything else on it.
    for line in estimate_lines(job, &result.estimate, several) {
        let text = line.message.render(w.lang, w.units.into());
        let _ = writeln!(html, "<p>{}</p>", escape(text.trim_start()));
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

/// "Setup sheet: part.chipload", escaped.
/// With some parts only: "Setup sheet: box.chipload, part "lid"".
/// With several setups, the active one's name after it: "Setup sheet: box.chipload, setup
/// "6 mm bottom"" (docs/specs/setups.md §4).
fn title(job: &Job, info: &SheetInfo, w: Words) -> String {
    let base = if info.parts.is_empty() {
        w.text(&Message::new("sheet-title").arg("project", info.project.as_str()))
    } else {
        let names = info.parts.join(", ");
        w.text(
            &Message::new("sheet-title-parts")
                .arg("project", info.project.as_str())
                .arg("parts", names)
                .arg("count", info.parts.len()),
        )
    };
    match job.active_setup_id().and_then(|id| job.setup(id)) {
        Some(setup) if job.setups.len() > 1 => format!(
            "{base}, {}",
            w.text(&Message::new("sheet-setup").arg("name", setup.name.as_str()))
        ),
        _ => base,
    }
}

fn head(html: &mut String, job: &Job, info: &SheetInfo, w: Words) {
    let _ = writeln!(
        html,
        "<!doctype html>\n<html lang=\"{}\">\n<head>\n<meta charset=\"utf-8\">\n\
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n\
         <title>{}</title>\n<style>\n{STYLE}</style>\n</head>\n<body>",
        info.language.code(),
        title(job, info, w)
    );
}

fn header(html: &mut String, job: &Job, result: &RunResult, info: &SheetInfo, w: Words) {
    let mut meta = Vec::new();
    if let Some(name) = &job.machine.name {
        meta.push(escape(name));
    }
    meta.push(w.text(&Message::new("sheet-post").arg("post", result.post.name.as_str())));
    meta.push(w.text(&Message::new("sheet-written-by").arg("by", info.written_by.as_str())));
    meta.push(
        w.text(
            &Message::new("sheet-units")
                .arg("length", crate::units::length_unit(w.units))
                .arg("feed", crate::units::feed_unit(w.units)),
        ),
    );
    // The program's own units, when they aren't the sheet's (docs/specs/inch-units.md §3).
    if job.machine.gcode_units == Units::Inch {
        meta.push(format!(
            "<strong>{}</strong>",
            w.text(&Message::new("sheet-gcode-inch"))
        ));
    }
    let _ = writeln!(
        html,
        "<header>\n<h1>{}</h1>\n<p class=\"meta\">{}</p>\n</header>",
        title(job, info, w),
        meta.join(" &middot; ")
    );
}

/// A row of the facts table: its heading and value, both HTML.
fn fact(html: &mut String, heading: &str, value: &str) {
    let _ = writeln!(html, "<tr><th>{heading}</th><td>{value}</td></tr>");
}

fn setup(html: &mut String, job: &Job, w: Words) {
    let stock = &job.stock();
    let _ = writeln!(
        html,
        "<section class=\"setup\">\n<h2>{}</h2>\n<table class=\"facts\">",
        w.text(&Message::new("sheet-stock-and-zero"))
    );
    fact(
        html,
        &w.text(&Message::new("sheet-stock")),
        &w.html(
            &Message::new("sheet-stock-size")
                .arg("width", len(job.stock_size().0))
                .arg("length", len_u(job.stock_size().1))
                .arg("thickness", len_u(stock.thickness)),
        ),
    );
    fact(html, "X0 Y0", &w.text(&xy_zero_message(job)));
    fact(html, "Z0", &w.text(&z_zero_message(job)));
    if let Some(copies) = copies::copies_message(job, w.units) {
        fact(
            html,
            &w.text(&Message::new("sheet-copies")),
            &w.text(&copies),
        );
    }
    // Several parts: each with its copies (docs/specs/parts.md §6).
    for line in copies::parts_messages(job, w.units) {
        fact(html, &w.text(&Message::new("sheet-part")), &w.text(&line));
    }
    // Several drawings: each with where it was put (docs/specs/drawing-sources.md §7).
    let d = &job.drawing;
    if d.dxfs.len() + d.images.len() + d.texts.len() > 1 {
        for dxf in &job.drawing.dxfs {
            fact(
                html,
                &w.text(&Message::new("sheet-dxf").arg("id", dxf.id)),
                &w.text(
                    &Message::new("sheet-dxf-placed")
                        .arg("name", dxf.name.as_str())
                        .arg("file", dxf.file.as_str())
                        .arg(
                            "placement",
                            crate::placement::placement_message(dxf.placement),
                        ),
                ),
            );
        }
        for image in &job.drawing.images {
            fact(
                html,
                &w.text(&Message::new("sheet-image").arg("id", image.id)),
                &w.text(
                    &Message::new("sheet-image-placed")
                        .arg("file", image.file.as_str())
                        .arg("layer", image.layer.as_str())
                        .arg("width", len_u(image.width))
                        .arg("x", len(image.x))
                        .arg("y", len(image.y)),
                ),
            );
        }
        for text in &job.drawing.texts {
            fact(
                html,
                &w.text(&Message::new("sheet-text").arg("id", text.id)),
                &w.text(
                    &Message::new("sheet-text-placed")
                        .arg("text", crate::texts::text_label(text))
                        .arg("font", text.font.as_str())
                        .arg("layer", text.layer.as_str())
                        .arg("height", len_u(text.height))
                        .arg("x", len(text.x))
                        .arg("y", len(text.y)),
                ),
            );
        }
    }
    html.push_str("</table>\n</section>\n");
}

/// Where X0 Y0 is, in words measured on the stock (§2.3), in English and mm.
pub fn xy_zero_text(job: &Job) -> String {
    xy_zero_message(job).english()
}

/// [`xy_zero_text`] as a message.
pub fn xy_zero_message(job: &Job) -> Message {
    let corner = |corner: StockCorner| {
        Message::new(match corner {
            StockCorner::Center => "sheet-zero-centre",
            StockCorner::BottomLeft => "sheet-zero-bottom-left",
            StockCorner::BottomRight => "sheet-zero-bottom-right",
            StockCorner::TopLeft => "sheet-zero-top-left",
            StockCorner::TopRight => "sheet-zero-top-right",
        })
    };
    if let WorkZero::Stock { corner: at } = job.work_zero() {
        return corner(at);
    }
    let ((x0, y0), (x1, y1)) = job.stock_bounds();
    let (zx, zy) = job.work_zero_xy();
    let (dx, dy) = (zx - x0, zy - y0);
    let mut parts = Vec::new();
    if dx.abs() > CHAIN_TOL_MM {
        let side = if dx > 0.0 {
            "sheet-zero-right-of"
        } else {
            "sheet-zero-left-of"
        };
        parts.push(Message::new(side).arg("distance", len_u(dx.abs())));
    }
    if dy.abs() > CHAIN_TOL_MM {
        let side = if dy > 0.0 {
            "sheet-zero-above"
        } else {
            "sheet-zero-below"
        };
        parts.push(Message::new(side).arg("distance", len_u(dy.abs())));
    }
    let offset = parts.into_iter().reduce(|first, second| {
        Message::new("sheet-zero-and")
            .arg("first", first)
            .arg("second", second)
    });
    let text = match offset {
        None => corner(StockCorner::BottomLeft),
        Some(offset) => Message::new("sheet-zero-from-corner").arg("offset", offset),
    };
    let outside = zx < x0 - CHAIN_TOL_MM
        || zx > x1 + CHAIN_TOL_MM
        || zy < y0 - CHAIN_TOL_MM
        || zy > y1 + CHAIN_TOL_MM;
    if outside {
        Message::new("sheet-zero-outside").arg("zero", text)
    } else {
        text
    }
}

fn z_zero_message(job: &Job) -> Message {
    Message::new(match job.stock().z_zero {
        ZZero::Top => "sheet-z-top",
        ZZero::Bottom => "sheet-z-bottom",
    })
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

/// A time for reading, escaped: `12 min 30 s`.
fn duration(w: Words, seconds: f64) -> String {
    w.text(&duration_message(seconds))
}

fn file_section(
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
        ToolType::Endmill => vec![Message::new("sheet-end-mill").arg("diameter", diameter)],
        ToolType::Drill => vec![Message::new("sheet-drill").arg("diameter", diameter)],
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
            escape(&depth_text(op, w)),
            feeds_text,
            escape(&entry_text(op, w)),
            escape(&tabs_text(op, w)),
            duration(w, seconds)
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

fn direction_message(direction: Direction) -> Message {
    Message::new(match direction {
        Direction::Climb => "sheet-climb",
        Direction::Conventional => "sheet-conventional",
    })
}

/// "Profile, outside, climb, leaves 0.2 mm" (a negative clearance).
fn kind_text(op: &Operation, tools: &[Tool], w: Words) -> String {
    let stepover = |s: f64| Message::new("sheet-stepover").arg("percent", num(s * 100.0));
    let mut parts = match op {
        Operation::Profile(p) => {
            let side = match p.side {
                Side::Outside => "sheet-side-outside",
                Side::Inside => "sheet-side-inside",
                Side::On => "sheet-side-on",
            };
            vec![
                Message::new("sheet-kind-profile"),
                Message::new(side),
                direction_message(p.direction),
            ]
        }
        Operation::Pocket(p) => vec![
            Message::new("sheet-kind-pocket"),
            direction_message(p.direction),
            stepover(p.stepover),
        ],
        Operation::Vcarve(_) => vec![Message::new("sheet-kind-vcarve")],
        Operation::Path(_) => vec![Message::new("sheet-kind-path")],
        Operation::Holes(h) => match h.method {
            HoleMethod::Drill { .. } => vec![Message::new("sheet-kind-holes-drilled")],
            HoleMethod::Bore { direction, .. } => vec![
                Message::new("sheet-kind-holes-bored"),
                direction_message(direction),
            ],
        },
        // docs/specs/surfacing.md §3, §5.
        Operation::Surface(f) => {
            let kind = Message::new("sheet-kind-surface").arg("percent", num(f.stepover * 100.0));
            vec![match tools.iter().find(|t| t.id == f.tool) {
                Some(t) => Message::new("sheet-kind-surface-past")
                    .arg("surface", kind)
                    .arg("distance", len_u(t.diameter / 2.0)),
                None => kind,
            }]
        }
    };
    // Positive clearance cuts into the material that is kept, negative leaves some
    // (docs/specs/depths-and-clearance.md §3).
    let clearance = op.clearance();
    if clearance > 0.0 {
        parts.push(Message::new("sheet-cuts-past").arg("distance", len_u(clearance)));
    } else if clearance < 0.0 {
        parts.push(Message::new("sheet-leaves").arg("distance", len_u(-clearance)));
    }
    // docs/specs/rest-machining.md §6.
    if let Operation::Pocket(PocketOp {
        rest: Some(rest), ..
    }) = op
    {
        parts.push(match find_tool(tools, rest.after_tool) {
            Some(tool) => Message::new("sheet-rest-after-size")
                .arg("tool", rest.after_tool)
                .arg("diameter", len_u(tool.diameter)),
            None => Message::new("sheet-rest-after").arg("tool", rest.after_tool),
        });
    }
    // docs/specs/dogbones.md §7.
    if let Some(corners) = op.corners() {
        let style = match corners {
            Corners::Dogbone { .. } => "sheet-dogbones",
            Corners::Tbone { .. } => "sheet-tbones",
        };
        parts.push(Message::new(style).arg("degrees", num(corners.max_angle().to_degrees())));
    }
    // docs/specs/dogbone-per-corner.md.
    let chosen = op.corner_choices().len();
    if chosen > 0 {
        parts.push(Message::new("sheet-corner-choices").arg("count", chosen));
    }
    listed(w, &parts)
}

/// "0 to 6, 2 per pass", with a profile's finishing stock and skin.
fn depth_text(op: &Operation, w: Words) -> String {
    let (start, end) = op.depths();
    let step = match op {
        Operation::Profile(p) => Some(p.step_down),
        Operation::Pocket(p) => Some(p.step_down),
        Operation::Vcarve(_) => None,
        Operation::Path(p) => Some(p.step_down),
        Operation::Holes(_) => None,
        Operation::Surface(f) => Some(f.step_down),
    };
    let depths = |id: &'static str| {
        Message::new(id)
            .arg("start", len(start))
            .arg("end", len(end))
    };
    let mut parts = match op {
        // docs/specs/holes.md §6: a drill's depth is its point's.
        Operation::Holes(HolesOp {
            method: HoleMethod::Drill { peck },
            ..
        }) => vec![
            depths("sheet-depths-drill"),
            match peck {
                Some(peck) if *peck < end - start => {
                    Message::new("sheet-pecks").arg("peck", len_u(*peck))
                }
                _ => Message::new("sheet-one-go"),
            },
        ],
        Operation::Holes(HolesOp {
            method: HoleMethod::Bore { step_down, .. },
            ..
        }) => vec![
            depths("sheet-depths"),
            Message::new("sheet-per-turn").arg("step", len(*step_down)),
        ],
        // docs/specs/v-carve.md decision 12: a number of passes of equal load.
        Operation::Vcarve(v) if v.passes > 1 => vec![
            depths("sheet-depths"),
            Message::new("sheet-passes").arg("count", v.passes),
        ],
        _ => vec![
            depths("sheet-depths"),
            match step {
                Some(step) if step > 0.0 && step < end - start => {
                    Message::new("sheet-per-pass").arg("step", len(step))
                }
                _ => Message::new("sheet-one-pass"),
            },
        ],
    };
    if let Operation::Profile(p) = op {
        // docs/specs/finish-and-skin.md §6: "finish 0.3 mm, skin 0.2 mm".
        if let Some(finish) = p.finish {
            parts.push(Message::new("sheet-finish").arg("stock", len_u(finish.stock)));
        }
        if let Some(skin) = p.skin {
            parts.push(Message::new("sheet-skin").arg("thickness", len_u(skin.thickness)));
        }
    }
    listed(w, &parts)
}

fn entry_text(op: &Operation, w: Words) -> String {
    let degrees = |angle: f64| num(angle.to_degrees());
    if let Operation::Surface(_) = op {
        return w.say(&Message::new("sheet-entry-beside"));
    }
    if let Operation::Holes(h) = op {
        return w.say(&match h.method {
            HoleMethod::Drill { .. } => Message::new("sheet-entry-straight"),
            HoleMethod::Bore { ramp_angle, .. } => {
                Message::new("sheet-entry-helix").arg("degrees", degrees(ramp_angle))
            }
        });
    }
    let mut parts = vec![match op.entry() {
        Entry::Plunge {} => Message::new("sheet-entry-plunge"),
        Entry::Ramp { ramp_angle } => {
            Message::new("sheet-entry-ramp").arg("degrees", degrees(ramp_angle))
        }
        Entry::Helix { radius, ramp_angle } => Message::new("sheet-entry-helix-radius")
            .arg("radius", len(radius))
            .arg("degrees", degrees(ramp_angle)),
    }];
    if let Some(lead) = op.lead() {
        parts.push(Message::new("sheet-lead").arg("radius", len(lead.radius)));
    }
    listed(w, &parts)
}

fn tabs_text(op: &Operation, w: Words) -> String {
    match op {
        Operation::Profile(p) => match &p.tabs {
            Some(t) if t.count > 0 => {
                let mut parts = vec![
                    Message::new("sheet-tabs")
                        .arg("count", t.count)
                        .arg("width", len(t.width))
                        .arg("height", len(t.height)),
                ];
                if !t.is_straight() {
                    let degrees = Value::Number(t.flank_angle.to_degrees(), Digits::Trim(1));
                    parts.push(Message::new("sheet-flanks").arg("degrees", degrees));
                }
                listed(w, &parts)
            }
            _ => w.say(&Message::new("sheet-tabs-none")),
        },
        _ => "-".into(),
    }
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
fn watch_out(html: &mut String, result: &RunResult, w: Words) {
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

/// Colours and dashes of the files (or of the tools in one file), told apart in black and
/// white too (§5, decision 4).
const COLOURS: [&str; 6] = [
    "#1f5fbf", "#c0392b", "#1e8449", "#7d3c98", "#b9770e", "#117a8b",
];
const DASHES: [&str; 4] = ["", "6 3", "2 2", "9 3 2 3"];

/// The top view: stock, parts, each file's cutting moves, tabs and the X0 Y0 mark (§2.4).
fn picture(html: &mut String, job: &Job, result: &RunResult, w: Words) {
    let ((sx0, sy0), (sx1, sy1)) = job.stock_bounds();
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
            w.text(
                &Message::new("sheet-legend-file")
                    .arg("number", i + 1)
                    .arg("tool", group.tool.id),
            )
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
        let _ = write!(
            html,
            "<span><svg width=\"36\" height=\"10\"><line x1=\"4\" y1=\"5\" x2=\"32\" y2=\"5\" \
             stroke=\"#e67e22\" stroke-width=\"5\" stroke-linecap=\"round\"/></svg> {}</span>",
            w.text(&Message::new("sheet-legend-tabs"))
        );
    }
    let _ = writeln!(
        html,
        "<span>{}</span></figcaption>\n</figure>",
        w.text(&Message::new("sheet-legend-grey"))
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
                Operation::Path(_) | Operation::Holes(_) => {
                    select_paths(contours, op.select(), &op.label_message(0))
                }
                _ => select_contours(contours, op.select(), &op.label_message(0)),
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

/// A number for reading: at most 3 decimals, no trailing zeros, never `-0`.
fn num(value: f64) -> Value {
    Value::Number(value, Digits::Trim(3))
}

/// A length (or feed) in the sheet's units, the number only.
fn len(mm: f64) -> Value {
    Value::LengthNumber(mm, Digits::Trim(3))
}

/// A length in the sheet's units, with its unit: `6 mm`, `0.25 in`.
fn len_u(mm: f64) -> Value {
    Value::mm(mm)
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

/// HTML with everything outside ASCII as character references: a translated text that
/// holds HTML itself.
fn ascii(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    for c in html.chars() {
        if c.is_ascii() {
            out.push(c);
        } else {
            let _ = write!(out, "&#x{:x};", c as u32);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_read_naturally() {
        let w = Words {
            lang: Language::En,
            units: Units::Mm,
        };
        let read = |value: Value| w.say(&Message::new("sheet-stepover").arg("percent", value));
        assert_eq!(read(num(6.0)), "stepover 6 %");
        assert_eq!(read(num(0.5)), "stepover 0.5 %");
        assert_eq!(read(num(-0.0001)), "stepover 0 %");
        assert_eq!(read(num(12.3456)), "stepover 12.346 %");
        assert_eq!(read(num(3.0_f64.to_radians().to_degrees())), "stepover 3 %");
        assert_eq!(svg_num(-0.001), "0");
    }

    #[test]
    fn html_texts_keep_their_markup_and_write_the_rest_as_ascii() {
        assert_eq!(ascii("6 &times; 4 \u{fc}"), "6 &times; 4 &#xfc;");
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
