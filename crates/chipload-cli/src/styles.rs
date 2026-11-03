// SPDX-License-Identifier: GPL-3.0-or-later
//! `chipload style …`: your styles, operations saved by name and used again on shapes of a
//! drawing (docs/specs/styles.md §3).

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use chipload_job::library::{
    LibraryError, export, import_into, load_tools, read_import, update_library, write_export,
};
use chipload_job::styles::{Style, StyleLibrary, StyleSide, load_styles, style_of};
use chipload_job::{Operation, Side, save_job};
use chipload_pipeline::library::{styles_path, tools_path};
use clap::Subcommand;
use serde_json::json;

use crate::library::dir;
use crate::load;
use crate::ops::{Shapes, describe};
use crate::out::Out;

#[derive(Subcommand)]
pub enum StyleAction {
    /// List your styles and where the file is.
    List,
    /// A style: its kind, tool, depth and settings.
    Show { name: String },
    /// Save an operation of a project as a style. Its tools must be in your library; its
    /// shapes and its start points, tabs and corners placed by hand are not kept, nor its
    /// feeds when its tool has cutting data.
    Save {
        /// The project (.chipload).
        project: PathBuf,
        /// The operation, 1-based as listed.
        #[arg(long)]
        op: usize,
        /// The style's name.
        #[arg(long)]
        name: String,
        /// Put it in place of the style of that name.
        #[arg(long)]
        replace: bool,
    },
    /// Add the style's operation for shapes of a project, at the end of the active part: a
    /// profile style on an outline with its holes gives two, the holes first.
    Use {
        /// The project (.chipload).
        project: PathBuf,
        /// The style.
        name: String,
        #[command(flatten)]
        what: Shapes,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Rename a style.
    Rename { name: String, new: String },
    /// Change a style: values of its operation by path, as `chipload set` changes a
    /// project's (`end_depth=6`, `tabs.count=6`, `side=on`; `style show` lists them), and
    /// its tool by name from your library, for example after an import whose tool you
    /// don't have. Projects that used the style keep their operations.
    Set {
        name: String,
        /// PATH=VALUE, paths within the style's operation.
        values: Vec<String>,
        /// Another tool of your library, by name.
        #[arg(long)]
        tool: Option<String>,
    },
    /// Remove a style.
    Remove { name: String },
    /// Write styles to a file: a backup, or for another computer.
    Export {
        file: PathBuf,
        /// Only these (repeatable); default: all.
        #[arg(long = "name")]
        names: Vec<String>,
    },
    /// Add the styles of an exported file.
    Import {
        file: PathBuf,
        /// Overwrite yours where the file has another style under the same name.
        #[arg(long)]
        replace: bool,
    },
}

pub fn style(out: &mut Out, action: StyleAction) -> ExitCode {
    let dir = match dir() {
        Ok(dir) => dir,
        Err(err) => return out.fail(err),
    };
    let path = styles_path(&dir);
    let tools = match load_tools(&tools_path(&dir)) {
        Ok(tools) => tools,
        Err(err) => return out.fail(err),
    };
    // Read once through `load_styles`: an older file is upgraded and saved before anything
    // changes it.
    let loaded = match load_styles(&path) {
        Ok(loaded) => loaded,
        Err(err) => return out.fail(err),
    };
    for refused in &loaded.refused {
        out.err(format!("warning: {}", refused.english()));
    }
    let styles = loaded.library;
    let result = match action {
        StyleAction::List => return list(out, &path, &styles, &tools),
        StyleAction::Show { name } => {
            return match styles.get(&name) {
                Ok(style) => {
                    out.line(format!("{}: {}", style.name, summary(style, &tools)));
                    // Every value, by the path `style set` takes.
                    let mut lines = Vec::new();
                    value_lines("", &style.operation, &mut lines);
                    for line in lines {
                        out.line(format!("  {line}"));
                    }
                    out.set("style", json!(style));
                    ExitCode::SUCCESS
                }
                Err(err) => out.fail(err),
            };
        }
        StyleAction::Save {
            project,
            op,
            name,
            replace,
        } => {
            let job = match load(out, &project) {
                Ok(job) => job,
                Err(code) => return code,
            };
            let index = op.saturating_sub(1);
            style_of(&job, index, &name, &tools).and_then(|style| {
                let line = format!(
                    "Saved style \"{}\": {}.",
                    style.name,
                    summary(&style, &tools)
                );
                update_library(&path, |library: &mut StyleLibrary| {
                    library.put(style, replace)
                })?;
                Ok(line)
            })
        }
        StyleAction::Use {
            project,
            name,
            what,
            output,
        } => return use_style(out, &styles, &tools, &project, &name, what, output),
        StyleAction::Rename { name, new } => update_library(&path, |library: &mut StyleLibrary| {
            library.rename(&name, &new)?;
            Ok(format!("Renamed style \"{name}\" to \"{}\".", new.trim()))
        }),
        StyleAction::Set { name, values, tool } => {
            return set(out, &path, &styles, &tools, &name, &values, tool.as_deref());
        }
        StyleAction::Remove { name } => update_library(&path, |library: &mut StyleLibrary| {
            let removed = library.remove(&name)?;
            Ok(format!("Removed style \"{}\".", removed.name))
        }),
        StyleAction::Export { file, names } => {
            return match export(&styles, &names)
                .and_then(|lib| write_export(&lib, &file).map(|()| lib.styles.len()))
            {
                Ok(count) => {
                    out.line(format!("Exported styles: {count}"));
                    out.wrote(&file, "");
                    ExitCode::SUCCESS
                }
                Err(err) => out.fail(err),
            };
        }
        StyleAction::Import { file, replace } => {
            return import(out, &path, &file, replace, &tools);
        }
    };
    match result {
        Ok(line) => {
            out.line(line);
            out.wrote(&path, "");
            ExitCode::SUCCESS
        }
        Err(err) => out.fail(err),
    }
}

/// The paths a style's operation keeps but never uses: they come from the shapes and
/// the project it is used in, or are placed by hand on them.
const NOT_SET: [&str; 9] = [
    "type",
    "name",
    "part",
    "enabled",
    "material",
    "select",
    "tool",
    "starts",
    "corner_choices",
];

/// `style set` (docs/specs/styles.md §9, step 3b): the values set in the style's
/// operation, read back as an operation, and its tool from the library; written with the
/// problems it has listed, as `chipload set` does.
fn set(
    out: &mut Out,
    path: &Path,
    styles: &StyleLibrary,
    tools: &chipload_job::library::ToolLibrary,
    name: &str,
    values: &[String],
    tool: Option<&str>,
) -> ExitCode {
    if values.is_empty() && tool.is_none() {
        return out.fail("give PATH=VALUE or --tool NAME");
    }
    let style = match styles.get(name) {
        Ok(style) => style.clone(),
        Err(err) => return out.fail(err),
    };
    let mut changed = style.clone();
    let mut lines = Vec::new();
    for text in values {
        let (at, value) = match chipload_job::paths::assignment(text) {
            Ok(assignment) => assignment,
            Err(err) => return out.fail(err),
        };
        let first = at.split('.').next().unwrap_or_default();
        if NOT_SET.contains(&first) {
            let how = if first == "tool" {
                "give it with --tool NAME"
            } else {
                "it comes from where the style is used"
            };
            return out.fail(format!("{at} can't be set in a style: {how}"));
        }
        if let Err(err) = chipload_job::paths::set(&mut changed.operation, &at, value.clone()) {
            return out.fail(err);
        }
        lines.push(format!("Set {at} = {value}"));
    }
    let op = match changed.operation() {
        Ok(op) => op,
        Err(err) => return out.fail(err),
    };
    changed.side = chipload_job::styles::side_of(&op);
    if let Some(tool) = tool {
        let Some(entry) = tools.find(tool) else {
            return out.fail(format!("no tool named \"{tool}\" in your library"));
        };
        let Some(first) = changed.tools.first_mut() else {
            return out.fail(format!("style \"{}\" names no tool", style.name));
        };
        first.uid = entry.uid.clone();
        first.name = entry.name.clone();
        lines.push(format!(
            "Style \"{}\" cuts with \"{}\".",
            style.name, entry.name
        ));
    }
    let problems = match chipload_pipeline::styles::style_draft(&changed, tools) {
        Ok(draft) => chipload_pipeline::styles::style_problems(&draft.operation, &draft.tools),
        Err(err) => return out.fail(err),
    };
    let written = update_library(path, |library: &mut StyleLibrary| {
        *library.find_mut(name)? = changed.clone();
        Ok(())
    });
    if let Err(err) = written {
        return out.fail(err);
    }
    for line in lines {
        out.line(line);
    }
    for problem in &problems {
        out.line(format!("problem: {}: {}", problem.field(), problem.text));
    }
    out.set(
        "problems",
        json!(
            problems
                .iter()
                .map(|p| json!({"path": p.field(), "short": p.short, "text": p.text}))
                .collect::<Vec<_>>()
        ),
    );
    out.wrote(path, "");
    ExitCode::SUCCESS
}

/// `value` as "path = value" lines under `at`, the fields a style keeps but never uses
/// left out.
fn value_lines(at: &str, value: &serde_json::Value, lines: &mut Vec<String>) {
    match value {
        serde_json::Value::Object(map) => {
            for (key, inner) in map {
                if at.is_empty() && NOT_SET.contains(&key.as_str()) {
                    continue;
                }
                let path = if at.is_empty() {
                    key.clone()
                } else {
                    format!("{at}.{key}")
                };
                value_lines(&path, inner, lines);
            }
        }
        // Lists (a hole's steps, corner choices) as one value.
        _ => lines.push(format!("{at} = {value}")),
    }
}

fn list(
    out: &mut Out,
    path: &Path,
    styles: &StyleLibrary,
    tools: &chipload_job::library::ToolLibrary,
) -> ExitCode {
    out.line(format!(
        "Styles ({}): {}",
        styles.styles.len(),
        path.display()
    ));
    for style in &styles.styles {
        out.line(format!("  {}: {}", style.name, summary(style, tools)));
    }
    let names: Vec<&str> = styles.styles.iter().map(|s| s.name.as_str()).collect();
    out.set("styles", json!(names));
    out.set("file", json!(path.display().to_string()));
    ExitCode::SUCCESS
}

/// A style in a few words: "profile, by the selection, through + 0.5 mm, 4 tabs 8 × 3 mm,
/// tool \"6 mm 2F\"".
fn summary(style: &Style, tools: &chipload_job::library::ToolLibrary) -> String {
    let Ok(op) = style.operation() else {
        return "can't be read".to_string();
    };
    let mut parts = vec![kind_name(&op).to_string()];
    if let Operation::Profile(p) = &op {
        parts.push(match (style.side, p.side) {
            (Some(StyleSide::AsSaved), Side::On) => "on the line".to_string(),
            (Some(StyleSide::AsSaved), side) => format!("{side:?}").to_lowercase(),
            _ => "side by the selection".to_string(),
        });
    }
    let (start, end) = op.depths();
    let mm = chipload_core::text::mm;
    parts.push(match op.through() {
        Some(below) => format!("through + {} mm", mm(below)),
        None if start > 0.0 => format!("{} to {} mm deep", mm(start), mm(end)),
        None => format!("{} mm deep", mm(end)),
    });
    if let Operation::Profile(p) = &op
        && let Some(tabs) = &p.tabs
    {
        parts.push(format!(
            "{} tabs {} × {} mm",
            tabs.count,
            mm(tabs.width),
            mm(tabs.height)
        ));
    }
    if let Some(tool) = style.tool() {
        let missing = if tools.find_uid(&tool.uid).is_none() {
            " (not in your library)"
        } else {
            ""
        };
        parts.push(format!("tool \"{}\"{missing}", tool.name));
    }
    parts.join(", ")
}

fn kind_name(op: &Operation) -> &'static str {
    match op {
        Operation::Profile(_) => "profile",
        Operation::Pocket(_) => "pocket",
        Operation::Vcarve(_) => "V-carve",
        Operation::Path(_) => "follow path",
        Operation::Holes(_) => "holes",
        Operation::Surface(_) => "surface",
    }
}

fn use_style(
    out: &mut Out,
    styles: &StyleLibrary,
    tools: &chipload_job::library::ToolLibrary,
    project: &Path,
    name: &str,
    what: Shapes,
    output: Option<PathBuf>,
) -> ExitCode {
    let style = match styles.get(name) {
        Ok(style) => style,
        Err(err) => return out.fail(err),
    };
    let select = match what.selector() {
        Ok(select) => select,
        Err(why) => return out.fail(why),
    };
    let mut job = match load(out, project) {
        Ok(job) => job,
        Err(code) => return code,
    };
    let drawing = match chipload_pipeline::load_drawing(&job) {
        Ok(drawing) => drawing,
        Err(err) => return out.fail(err),
    };
    let made = match chipload_pipeline::styles::style_operations(
        &job,
        style,
        &select,
        &drawing.contours,
        tools,
        chipload_i18n::Language::En,
    ) {
        Ok(made) => made,
        Err(err) => return out.fail(err),
    };
    let added: Vec<String> = made
        .tools
        .iter()
        .map(|t| format!("Copied tool {} in from your library.", t.id))
        .collect();
    let notes: Vec<String> = made.made.messages.iter().map(|m| m.english()).collect();
    let count = made.made.ops.len();
    let unsided = made.made.ops.iter().any(|n| !n.side_chosen);
    let first = chipload_pipeline::styles::insert(&mut job, made);
    let target = output.as_deref().unwrap_or(project);
    if let Err(err) = save_job(&job, target) {
        return out.fail(err);
    }
    for line in added {
        out.line(line);
    }
    for (offset, op) in job.operations[first..first + count].iter().enumerate() {
        out.line(format!(
            "Added operation {}: {}.",
            first + offset + 1,
            describe(op)
        ));
    }
    if unsided {
        out.line(
            "The selection doesn't tell the profile's side: it cuts outside; set `side` if \
             that is wrong."
                .to_string(),
        );
    }
    for note in notes {
        out.line(format!("note: {note}"));
    }
    out.set(
        "operations",
        json!((first..first + count).collect::<Vec<usize>>()),
    );
    out.wrote(target, "");
    ExitCode::SUCCESS
}

fn import(
    out: &mut Out,
    path: &Path,
    file: &Path,
    replace: bool,
    tools: &chipload_job::library::ToolLibrary,
) -> ExitCode {
    let imported = read_import::<StyleLibrary>(file).and_then(|from| {
        let missing: Vec<String> = from
            .styles
            .iter()
            .filter_map(|s| {
                let tool = s.tool()?;
                tools.find_uid(&tool.uid).is_none().then(|| {
                    format!(
                        "style \"{}\": its tool \"{}\" isn't in your library; choose one with \
                         `style set \"{}\" --tool NAME`",
                        s.name, tool.name, s.name
                    )
                })
            })
            .collect();
        let plan = update_library(path, |library: &mut StyleLibrary| {
            Ok::<_, LibraryError>(import_into(library, &from, replace))
        })?;
        Ok((plan, missing))
    });
    match imported {
        Ok((plan, missing)) => {
            for line in plan.report("styles", replace) {
                out.line(line.to_string());
            }
            for line in missing {
                out.line(format!("note: {line}"));
            }
            let differing: Vec<&String> = plan.differing.iter().map(|(name, _)| name).collect();
            out.set("added", json!(plan.added));
            out.set("same", json!(plan.same));
            out.set("differing", json!(differing));
            out.set("replaced", json!(replace));
            out.wrote(path, "");
            ExitCode::SUCCESS
        }
        Err(err) => out.fail(err),
    }
}
