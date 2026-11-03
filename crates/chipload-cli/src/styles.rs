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
    /// Give a style another tool of your library (by name), for example after an import
    /// whose tool you don't have.
    Set {
        name: String,
        #[arg(long)]
        tool: String,
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
        StyleAction::Set { name, tool } => {
            let Some(entry) = tools.find(&tool) else {
                return out.fail(format!("no tool named \"{tool}\" in your library"));
            };
            update_library(&path, |library: &mut StyleLibrary| {
                let style = library.find_mut(&name)?;
                let Some(first) = style.tools.first_mut() else {
                    return Ok(format!("Style \"{name}\" names no tool."));
                };
                first.uid = entry.uid.clone();
                first.name = entry.name.clone();
                Ok(format!(
                    "Style \"{}\" cuts with \"{}\".",
                    style.name, entry.name
                ))
            })
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
