// SPDX-License-Identifier: GPL-3.0-or-later
//! The setup sheet for the operator (docs/specs/setup-sheet.md): one HTML page, written next
//! to the G-code, saying what stock to load, where the zero is, which tool goes in for which
//! file and what to watch for. Made from the checked run, so it can't disagree with the
//! programs. ASCII only (names are written as character references) and no date, so the same
//! run gives the same sheet. Its words are texts of `sheet.ftl`, in the sheet's language
//! (docs/specs/translations.md step 3); names, numbers and G-code are never translated.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use chipload_core::text::trimmed;
use chipload_i18n::{Digits, Language, Message, Value};
use chipload_job::{Job, Units};

use crate::estimate::{Lengths, estimate_lines};
use crate::{RunResult, write_atomically};

mod files;
mod operations;
mod picture;
mod pieces;
mod stock;
#[cfg(test)]
mod tests;

use files::{file_name, file_section, watch_out};
pub use picture::contour_copies;
use picture::picture;
pub use pieces::{PieceSheet, pieces_sheet};
use stock::{nested, setup, turning};
pub use stock::{xy_zero_message, xy_zero_text};

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

/// [`write_setup_sheet`], or for a setup in pieces its one sheet for all its pieces
/// (docs/specs/pieces.md §8), from the pieces' own programs that `result` carries: the
/// window's export, whose result is the pieces put together. Returns its path.
pub fn write_sheet_for(
    job: &Job,
    result: &RunResult,
    output: &Path,
    info: &SheetInfo,
) -> std::io::Result<PathBuf> {
    if result.pieces.is_empty() {
        return write_setup_sheet(job, result, output, info);
    }
    let one = job
        .active_setup_id()
        .map_or_else(|| job.clone(), |id| job.for_setup(id));
    // The map is the pieces' outlines alone: no planning.
    let map = crate::pieces::piece_map(&one)
        .map_err(|e| std::io::Error::other(e.to_string()))?
        .ok_or_else(|| std::io::Error::other("not in pieces"))?;
    let mut runs = result.pieces.clone();
    for run in &mut runs {
        for file in &mut run.result.files {
            let tail = file.suffix.take().unwrap_or_default();
            let lead = match one.active_setup_id().filter(|_| one.setups.len() > 1) {
                Some(id) => format!("-S{id}-{}", run.name),
                None => format!("-{}", run.name),
            };
            file.suffix = Some(format!("{lead}{tail}"));
        }
    }
    let pieces: Vec<PieceSheet> = runs
        .iter()
        .map(|r| PieceSheet {
            name: &r.name,
            job: &r.job,
            result: &r.result,
        })
        .collect();
    let path = setup_sheet_path(&one, output);
    write_atomically(&path, &pieces_sheet(&one, &map, &pieces, output, info))?;
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
    nested(&mut html, job, result, w);
    turning(&mut html, job, w);
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
    for line in estimate_lines(job, &result.estimate, several, Lengths::Without) {
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
