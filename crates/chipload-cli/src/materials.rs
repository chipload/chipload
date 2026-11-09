// SPDX-License-Identifier: GPL-3.0-or-later
//! `chipload material …`: the user's materials, names whose values are the tools' rows of
//! cutting data, and choosing one for a project (docs/specs/materials.md,
//! docs/specs/cutting-data.md §7).

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use chipload_job::library::{
    LibraryError, ToolLibrary, export, import_into, load_library, load_tools, read_import,
    update_library, update_tools, write_export,
};
use chipload_job::materials::{Material, MaterialLibrary};
use chipload_pipeline::library::{materials_path, tools_path};
use chipload_pipeline::materials::{use_material, use_material_for_op, values_to_save};
use clap::Subcommand;

use crate::library::{dir, refused, row_text};
use crate::load;
use crate::out::Out;

#[derive(Subcommand)]
pub enum MaterialAction {
    /// List the materials and where the file is.
    List,
    /// A material and the tools with values for it (their rows, `tool cutting`).
    Show { name: String },
    /// Add a material; its values are the tools' rows for it (`tool cutting`).
    Add { name: String },
    /// Rename a material.
    Set {
        name: String,
        #[arg(long)]
        rename: String,
    },
    /// The material new projects start with (--none: none).
    Default {
        /// The material; omit with --none.
        #[arg(required_unless_present = "none")]
        name: Option<String>,
        #[arg(long, conflicts_with = "name")]
        none: bool,
    },
    /// The stock thickness new projects start with (--none: 10 mm).
    Thickness {
        /// The thickness (a plain number is mm); omit with --none.
        #[arg(required_unless_present = "none", value_parser = crate::lengths::length)]
        thickness: Option<f64>,
        #[arg(long, conflicts_with = "thickness")]
        none: bool,
    },
    /// Remove a material, and every tool's row for it.
    Remove { name: String },
    /// Choose the material for every operation and those added later: they take its values
    /// for their tools. The stock stays as it is.
    Use {
        /// The project (.chipload).
        project: PathBuf,
        /// The material; omit with --none.
        #[arg(required_unless_present = "none")]
        name: Option<String>,
        /// No material (the feeds stay as they are).
        #[arg(long, conflicts_with = "name")]
        none: bool,
        /// Only this operation, 1-based as listed; the project's material stays.
        #[arg(long)]
        op: Option<usize>,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Save an operation's values into its tool's row for a material.
    Save {
        /// The project (.chipload).
        project: PathBuf,
        /// The operation, 1-based as listed.
        #[arg(long)]
        op: usize,
        /// The material; default: the operation's, else the project's.
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
        MaterialAction::Add { name } => update_library(&path, |library: &mut MaterialLibrary| {
            let added = library.add(&name)?;
            Ok(format!("Added material \"{}\".", added.name))
        }),
        MaterialAction::Set { name, rename } => {
            update_library(&path, |library: &mut MaterialLibrary| {
                let new = rename.trim();
                if library
                    .position(new)
                    .is_some_and(|i| Some(i) != library.position(&name))
                {
                    return Err(refused(format!(
                        "a material named \"{new}\" already exists"
                    )));
                }
                library.rename(&name, new);
                let material = library.find_mut(&name)?;
                material.name = new.to_string();
                Ok(format!("Changed material \"{}\".", material.name))
            })
        }
        MaterialAction::Default { name, none: _ } => {
            update_library(&path, |library: &mut MaterialLibrary| match name {
                Some(name) => {
                    let found = library
                        .find(&name)
                        .map(|m| m.name.clone())
                        .ok_or_else(|| refused(format!("no material named \"{name}\"")))?;
                    library.default_material = Some(found.clone());
                    Ok(format!("New projects start with material \"{found}\"."))
                }
                None => {
                    library.default_material = None;
                    Ok("New projects start with no material.".to_string())
                }
            })
        }
        MaterialAction::Thickness { thickness, none: _ } => {
            update_library(&path, |library: &mut MaterialLibrary| {
                library.default_thickness = thickness;
                Ok(match thickness {
                    Some(t) => format!(
                        "New projects start with stock {} mm thick.",
                        chipload_core::text::mm(t)
                    ),
                    None => "New projects start with stock 10 mm thick.".to_string(),
                })
            })
        }
        MaterialAction::Remove { name } => update_library(
            &path,
            |library: &mut MaterialLibrary| library.remove(&name),
        )
        .and_then(|removed| {
            let mut line = format!("Removed material \"{}\".", removed.name);
            let names = chipload_pipeline::library::forget_material_in_tools(&dir, &removed.uid)?;
            if !names.is_empty() {
                line.push_str(&format!(
                    " Its rows were removed from: {}.",
                    names.join(", ")
                ));
            }
            Ok(line)
        }),
        MaterialAction::Use {
            project,
            name,
            none: _,
            op,
            output,
        } => return use_in_project(out, &path, &tools, &project, name.as_deref(), op, output),
        MaterialAction::Save {
            project,
            op,
            material,
        } => {
            let tools_file = tools_path(&dir);
            return save_from_op(out, &path, &tools_file, &project, op, material.as_deref());
        }
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
            return import(out, &path, &file, replace);
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

/// The tools with a row for `material`, and the row's values, one line each.
fn rows_of(tools: &ToolLibrary, material: &Material) -> Vec<(String, String)> {
    tools
        .tools
        .iter()
        .filter_map(|t| {
            t.row(Some(&material.uid))
                .map(|row| (t.name.clone(), row_text(&row.values)))
        })
        .collect()
}

fn list(out: &mut Out, path: &Path, tools: &ToolLibrary) -> ExitCode {
    let library = match load_library::<MaterialLibrary>(path) {
        Ok(library) => library,
        Err(err) => return out.fail(err),
    };
    out.line(format!("Materials: {}", path.display()));
    out.set("path", serde_json::json!(path.display().to_string()));
    out.set("materials", serde_json::json!(library.materials));
    out.set("default", serde_json::json!(library.default_material));
    if library.materials.is_empty() {
        out.line("  (no materials)");
    }
    for material in &library.materials {
        let is_default = library
            .default_material
            .as_deref()
            .is_some_and(|d| chipload_job::library::same_name(d, &material.name));
        let mark = if is_default { " (default)" } else { "" };
        let rows = rows_of(tools, material);
        out.line(format!(
            "  {}{mark}: {} tool(s) (uid {})",
            material.name,
            rows.len(),
            material.uid
        ));
        for (tool, values) in rows {
            out.line(format!("    {tool}: {values}"));
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
    let rows = rows_of(tools, material);
    out.line(format!(
        "{}: {} tool(s) (uid {})",
        material.name,
        rows.len(),
        material.uid
    ));
    out.set("material", serde_json::json!(material));
    out.set(
        "tools",
        serde_json::json!(rows.iter().map(|(tool, _)| tool).collect::<Vec<_>>()),
    );
    if rows.is_empty() {
        out.line("  (no tool has values for it yet: `tool cutting`)");
    }
    for (tool, values) in rows {
        out.line(format!("  {tool}: {values}"));
    }
    ExitCode::SUCCESS
}

fn use_in_project(
    out: &mut Out,
    path: &Path,
    tools: &ToolLibrary,
    project: &Path,
    name: Option<&str>,
    op: Option<usize>,
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
    let (job, messages) = match op {
        None => use_material(&job, tools, material),
        Some(0) => return out.fail("operations are numbered from 1"),
        Some(number) => match use_material_for_op(&job, tools, number - 1, material) {
            Ok(changed) => changed,
            Err(err) => return out.fail(err),
        },
    };
    let target = output.unwrap_or_else(|| project.to_path_buf());
    if let Err(err) = crate::save_project(out, &job, &target) {
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
    tools_file: &Path,
    project: &Path,
    op: usize,
    material: Option<&str>,
) -> ExitCode {
    let job = match load(out, project) {
        Ok(job) => job,
        Err(code) => return code,
    };
    let library = match load_library::<MaterialLibrary>(path) {
        Ok(library) => library,
        Err(err) => return out.fail(err),
    };
    let target = match material {
        Some(name) => library.find(name),
        None => op
            .checked_sub(1)
            .and_then(|i| job.operations.get(i))
            .and_then(|o| o.material())
            .or(job.stock().material.as_ref())
            .and_then(|m| library.find_uid(&m.library_uid)),
    };
    let Some(target) = target else {
        return out.fail(match material {
            Some(name) => format!("no material named \"{name}\""),
            None => "neither the operation nor the project has a material from your library: \
                     give --material NAME"
                .to_string(),
        });
    };
    let saved = update_tools(tools_file, |tools| {
        let (uid, values) = values_to_save(&job, op, tools, target)
            .map_err(|e| LibraryError::Refused(e.message().into()))?;
        let entry = tools
            .tools
            .iter_mut()
            .find(|t| t.uid == uid)
            .ok_or_else(|| refused(format!("no library tool with uid {uid}")))?;
        entry.set_row(Some(&target.uid), values);
        Ok(format!(
            "Saved operation {op}'s values as \"{}\"'s row for \"{}\": {}.",
            entry.name,
            target.name,
            row_text(&values)
        ))
    });
    match saved {
        Ok(line) => {
            out.line(line);
            out.wrote(tools_file, "");
            ExitCode::SUCCESS
        }
        Err(err) => out.fail(err),
    }
}

/// `material import` (as `tool import`): names only, the values come with the tools.
fn import(out: &mut Out, path: &Path, file: &Path, replace: bool) -> ExitCode {
    let imported = read_import::<MaterialLibrary>(file).and_then(|from| {
        update_library(path, |library: &mut MaterialLibrary| {
            Ok(import_into(library, &from, replace))
        })
    });
    match imported {
        Ok(plan) => {
            for line in plan.report("materials", replace) {
                out.line(line.to_string());
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

/// After operation `index`'s tool changed: it takes its material's values for the new
/// tool. The material's name when the feeds changed. Without a materials file to read,
/// the feeds stay.
pub fn follow_op_material(job: &mut chipload_job::Job, index: usize) -> Option<String> {
    job.operations.get(index)?.material()?;
    let dir = dir().ok()?;
    let library = load_library::<MaterialLibrary>(&materials_path(&dir)).ok()?;
    let tools = load_tools(&tools_path(&dir)).ok()?;
    let name = chipload_pipeline::materials::op_material(job, &library, index)?
        .name
        .clone();
    chipload_pipeline::materials::follow_material(job, &library, &tools, index).then_some(name)
}
