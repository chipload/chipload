// SPDX-License-Identifier: GPL-3.0-or-later
//! `chipload material …`: the user's materials, each with their own feeds per library tool,
//! and choosing one for a project (docs/specs/materials.md).

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use chipload_job::library::{
    Cutting, LibraryError, ToolLibrary, export, import_into, load_library, load_tools, read_import,
    update_library, write_export,
};
use chipload_job::materials::{Material, MaterialLibrary};
use chipload_job::save_job;
use chipload_pipeline::library::{materials_path, tools_path};
use chipload_pipeline::materials::{use_material, values_to_save};
use clap::Subcommand;

use crate::library::{dir, refused};
use crate::load;
use crate::out::Out;

#[derive(Subcommand)]
pub enum MaterialAction {
    /// List the materials and where the file is.
    List,
    /// A material's thickness and its values per tool.
    Show { name: String },
    /// Add a material, with no values yet.
    Add {
        name: String,
        /// The board it usually comes as (any length unit; a plain number is mm).
        #[arg(value_parser = crate::lengths::length, long)]
        thickness: Option<f64>,
    },
    /// Change a material: its name, thickness, or one library tool's values.
    Set {
        name: String,
        #[arg(long)]
        rename: Option<String>,
        #[arg(value_parser = crate::lengths::length, long)]
        thickness: Option<f64>,
        /// No thickness: choosing the material keeps the project's.
        #[arg(long, conflicts_with = "thickness")]
        no_thickness: bool,
        /// The library tool whose values these are.
        #[arg(long)]
        tool: Option<String>,
        /// mm/min. The four values come as a set the first time for a tool.
        #[arg(value_parser = crate::lengths::feed, long, requires = "tool")]
        feed: Option<f64>,
        #[arg(value_parser = crate::lengths::feed, long, requires = "tool")]
        plunge_feed: Option<f64>,
        #[arg(value_parser = crate::lengths::length, long, requires = "tool")]
        step_down: Option<f64>,
        /// Fraction of the tool diameter (pockets).
        #[arg(long, requires = "tool")]
        stepover: Option<f64>,
    },
    /// Remove a material, or with --tool only that tool's values.
    Remove {
        name: String,
        #[arg(long)]
        tool: Option<String>,
    },
    /// Choose the project's material: the stock takes its thickness, the operations its
    /// values for their tools.
    Use {
        /// The project (.chipload).
        project: PathBuf,
        /// The material; omit with --none.
        #[arg(required_unless_present = "none")]
        name: Option<String>,
        /// No material (the feeds stay as they are).
        #[arg(long, conflicts_with = "name")]
        none: bool,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Save an operation's feeds, step down and stepover as its tool's values in a material.
    Save {
        /// The project (.chipload).
        project: PathBuf,
        /// The operation, 1-based as listed.
        #[arg(long)]
        op: usize,
        /// The material; default: the project's.
        #[arg(long)]
        material: Option<String>,
    },
    /// Write the materials to a file: a backup, or for another computer.
    Export {
        file: PathBuf,
        /// Only these (repeatable); default: all.
        #[arg(long = "name")]
        names: Vec<String>,
    },
    /// Add the materials of an exported file.
    Import {
        file: PathBuf,
        /// Overwrite yours where the file has other values under the same name.
        #[arg(long)]
        replace: bool,
    },
}

pub fn material(out: &mut Out, action: MaterialAction) -> ExitCode {
    let dir = match dir() {
        Ok(dir) => dir,
        Err(err) => return out.fail(err),
    };
    let path = materials_path(&dir);
    let tools = match load_tools(&tools_path(&dir)) {
        Ok(tools) => tools,
        Err(err) => return out.fail(err),
    };
    let result = match action {
        MaterialAction::List => return list(out, &path, &tools),
        MaterialAction::Show { name } => return show(out, &path, &tools, &name),
        MaterialAction::Add { name, thickness } => {
            update_library(&path, |library: &mut MaterialLibrary| {
                let added = library.add(&name, thickness)?;
                Ok(format!("Added material \"{}\".", added.name))
            })
        }
        MaterialAction::Set {
            name,
            rename,
            thickness,
            no_thickness,
            tool,
            feed,
            plunge_feed,
            step_down,
            stepover,
        } => update_library(&path, |library: &mut MaterialLibrary| {
            if let Some(new) = rename.as_deref()
                && library
                    .position(new)
                    .is_some_and(|i| Some(i) != library.position(&name))
            {
                return Err(refused(format!(
                    "a material named \"{new}\" already exists"
                )));
            }
            let material = library.find_mut(&name)?;
            if let Some(new) = rename {
                material.name = new.trim().to_string();
            }
            if let Some(t) = thickness {
                material.thickness = Some(t);
            }
            if no_thickness {
                material.thickness = None;
            }
            if let Some(tool) = tool {
                let entry = tools
                    .find(&tool)
                    .ok_or_else(|| refused(format!("no tool named \"{tool}\" in your library")))?;
                let old = material.values(&entry.uid);
                let pick = |new: Option<f64>, old: Option<f64>, flag: &str| {
                    new.or(old).ok_or_else(|| {
                        refused(format!(
                            "a tool's values come as a set the first time: give --feed, \
                             --plunge-feed, --step-down and --stepover (missing --{flag})"
                        ))
                    })
                };
                let values = Cutting {
                    feed: pick(feed, old.map(|c| c.feed), "feed")?,
                    plunge_feed: pick(plunge_feed, old.map(|c| c.plunge_feed), "plunge-feed")?,
                    step_down: pick(step_down, old.map(|c| c.step_down), "step-down")?,
                    stepover: pick(stepover, old.map(|c| c.stepover), "stepover")?,
                };
                material.set_values(&entry.uid, values);
            }
            Ok(format!("Changed material \"{}\".", material.name))
        }),
        MaterialAction::Remove { name, tool } => {
            update_library(&path, |library: &mut MaterialLibrary| match tool {
                None => {
                    let removed = library.remove(&name)?;
                    Ok(format!("Removed material \"{}\".", removed.name))
                }
                Some(tool) => {
                    let uid = tools.find(&tool).map(|t| t.uid.clone()).ok_or_else(|| {
                        refused(format!("no tool named \"{tool}\" in your library"))
                    })?;
                    let material = library.find_mut(&name)?;
                    if !material.remove_values(&uid) {
                        return Err(refused(format!(
                            "\"{}\" has no values for tool \"{tool}\"",
                            material.name
                        )));
                    }
                    Ok(format!(
                        "Removed tool \"{tool}\"'s values from \"{}\".",
                        material.name
                    ))
                }
            })
        }
        MaterialAction::Use {
            project,
            name,
            none: _,
            output,
        } => return use_in_project(out, &path, &project, name.as_deref(), output),
        MaterialAction::Save {
            project,
            op,
            material,
        } => return save_from_op(out, &path, &tools, &project, op, material.as_deref()),
        MaterialAction::Export { file, names } => {
            let exported =
                load_library::<MaterialLibrary>(&path).and_then(|library| export(&library, &names));
            return match exported
                .and_then(|lib| write_export(&lib, &file).map(|()| lib.materials.len()))
            {
                Ok(count) => {
                    out.line(format!("Exported materials: {count}"));
                    out.wrote(&file, "");
                    ExitCode::SUCCESS
                }
                Err(err) => out.fail(err),
            };
        }
        MaterialAction::Import { file, replace } => {
            return import(out, &path, &tools, &file, replace);
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

/// A tool's name in the library, or its uid when the library no longer has it.
fn tool_name(tools: &ToolLibrary, uid: &str) -> String {
    tools.find_uid(uid).map_or_else(
        || format!("(not in your library: {uid})"),
        |t| t.name.clone(),
    )
}

fn values_text(c: &Cutting) -> String {
    format!(
        "feed {} plunge {} mm/min, step down {} mm, stepover {}",
        c.feed, c.plunge_feed, c.step_down, c.stepover
    )
}

fn heading(material: &Material) -> String {
    let thickness = material
        .thickness
        .map_or_else(String::new, |t| format!(", {t} mm"));
    format!(
        "{}{thickness}: {} tool(s) (uid {})",
        material.name,
        material.tools.len(),
        material.uid
    )
}

fn list(out: &mut Out, path: &Path, tools: &ToolLibrary) -> ExitCode {
    let library = match load_library::<MaterialLibrary>(path) {
        Ok(library) => library,
        Err(err) => return out.fail(err),
    };
    out.line(format!("Materials: {}", path.display()));
    out.set("path", serde_json::json!(path.display().to_string()));
    out.set("materials", serde_json::json!(library.materials));
    if library.materials.is_empty() {
        out.line("  (no materials)");
    }
    for material in &library.materials {
        out.line(format!("  {}", heading(material)));
        for t in &material.tools {
            out.line(format!(
                "    {}: {}",
                tool_name(tools, &t.tool_uid),
                values_text(&t.cutting())
            ));
        }
    }
    ExitCode::SUCCESS
}

fn show(out: &mut Out, path: &Path, tools: &ToolLibrary, name: &str) -> ExitCode {
    let library = match load_library::<MaterialLibrary>(path) {
        Ok(library) => library,
        Err(err) => return out.fail(err),
    };
    let Some(material) = library.find(name) else {
        return out.fail(format!("no material named \"{name}\""));
    };
    out.line(heading(material));
    out.set("material", serde_json::json!(material));
    if material.tools.is_empty() {
        out.line("  (no tool values yet)");
    }
    for t in &material.tools {
        out.line(format!(
            "  {}: {}",
            tool_name(tools, &t.tool_uid),
            values_text(&t.cutting())
        ));
    }
    ExitCode::SUCCESS
}

fn use_in_project(
    out: &mut Out,
    path: &Path,
    project: &Path,
    name: Option<&str>,
    output: Option<PathBuf>,
) -> ExitCode {
    let library = match load_library::<MaterialLibrary>(path) {
        Ok(library) => library,
        Err(err) => return out.fail(err),
    };
    let material = match name {
        Some(name) => match library.find(name) {
            Some(m) => Some(m),
            None => return out.fail(format!("no material named \"{name}\"")),
        },
        None => None,
    };
    let job = match load(out, project) {
        Ok(job) => job,
        Err(code) => return code,
    };
    let (job, messages) = use_material(&job, material);
    let target = output.unwrap_or_else(|| project.to_path_buf());
    if let Err(err) = save_job(&job, &target) {
        return out.fail(err);
    }
    let lines: Vec<String> = messages.iter().map(|m| m.english()).collect();
    for line in &lines {
        out.line(line.clone());
    }
    out.set("messages", serde_json::json!(lines));
    out.wrote(&target, "");
    ExitCode::SUCCESS
}

fn save_from_op(
    out: &mut Out,
    path: &Path,
    tools: &ToolLibrary,
    project: &Path,
    op: usize,
    material: Option<&str>,
) -> ExitCode {
    let job = match load(out, project) {
        Ok(job) => job,
        Err(code) => return code,
    };
    let saved = update_library(path, |library: &mut MaterialLibrary| {
        let name = match material {
            Some(name) => name.to_string(),
            None => job
                .stock
                .material
                .as_ref()
                .and_then(|m| library.find_uid(&m.library_uid))
                .map(|m| m.name.clone())
                .ok_or_else(|| {
                    refused(
                        "the project has no material from your library: give --material NAME"
                            .into(),
                    )
                })?,
        };
        let target = library.find_mut(&name)?;
        let (uid, values) = values_to_save(&job, op, tools, target)
            .map_err(|e| LibraryError::Refused(e.message().into()))?;
        target.set_values(&uid, values);
        Ok(format!(
            "Saved operation {op}'s values for tool \"{}\" in \"{}\": {}.",
            tool_name(tools, &uid),
            target.name,
            values_text(&values)
        ))
    });
    match saved {
        Ok(line) => {
            out.line(line);
            out.wrote(path, "");
            ExitCode::SUCCESS
        }
        Err(err) => out.fail(err),
    }
}

/// `material import` (as `tool import`), naming values whose tool isn't in this library.
fn import(out: &mut Out, path: &Path, tools: &ToolLibrary, file: &Path, replace: bool) -> ExitCode {
    let imported = read_import::<MaterialLibrary>(file).and_then(|from| {
        let plan = update_library(path, |library: &mut MaterialLibrary| {
            Ok(import_into(library, &from, replace))
        })?;
        Ok((plan, from))
    });
    match imported {
        Ok((plan, from)) => {
            for line in plan.report("materials", replace) {
                out.line(line.to_string());
            }
            for material in &from.materials {
                let missing = material
                    .tools
                    .iter()
                    .filter(|t| tools.find_uid(&t.tool_uid).is_none())
                    .count();
                if missing > 0 {
                    out.line(format!(
                        "\"{}\": values for {missing} tool(s) that aren't in your tool library \
                         (import those tools too)",
                        material.name
                    ));
                }
            }
            if !replace && !plan.differing.is_empty() {
                out.line("Use --replace to take the file's values.");
            }
            let differing: Vec<&String> = plan.differing.iter().map(|(name, _)| name).collect();
            out.set("added", serde_json::json!(plan.added));
            out.set("same", serde_json::json!(plan.same));
            out.set("differing", serde_json::json!(differing));
            out.set("replaced", serde_json::json!(replace));
            out.wrote(path, "");
            ExitCode::SUCCESS
        }
        Err(err) => out.fail(err),
    }
}
