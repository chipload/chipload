// SPDX-License-Identifier: GPL-3.0-or-later
//! `chipload tool …`: the user's tool library, and project tools linked to it.

use super::*;
use crate::load;
use crate::out::Out;
use chipload_i18n::Message;
use chipload_job::ToolType;
use chipload_job::library::{
    Cutting, LibraryError, LibraryTool, Link, LinkEntry, ToolLibrary, load_library, load_tools,
    update_tools, use_tool,
};
use chipload_job::materials::MaterialLibrary;
use chipload_pipeline::library::tools_path;
use clap::{Subcommand, ValueEnum};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

#[derive(Subcommand)]
pub enum ToolAction {
    /// Write the tools to a file: a backup, or for another computer.
    Export {
        file: PathBuf,
        /// Only these (repeatable); default: all.
        #[arg(long = "name")]
        names: Vec<String>,
    },
    /// Add the tools of an exported file (or another computer's library file).
    Import {
        file: PathBuf,
        /// Overwrite yours where the file has other values under the same name.
        #[arg(long)]
        replace: bool,
    },
    /// List the library's tools and where the library is.
    List,
    /// Add a tool to the library.
    Add {
        name: String,
        #[command(flatten)]
        values: ToolValues,
    },
    /// Change a library tool.
    Set {
        name: String,
        #[command(flatten)]
        values: ToolValues,
        /// A new name.
        #[arg(long)]
        rename: Option<String>,
    },
    /// Remove a tool from the library.
    Remove { name: String },
    /// The tool new projects start with.
    Default { name: String },
    /// Set or remove a row of a tool's cutting data: its values for a material, or for any
    /// material (docs/specs/cutting-data.md).
    Cutting {
        name: String,
        /// The material's name, or `any`: the row used without a material, or for a material
        /// the tool has no row for.
        #[arg(long)]
        material: String,
        #[command(flatten)]
        values: RowValues,
        /// Remove the row instead.
        #[arg(long)]
        remove: bool,
    },
    /// Copy a library tool into a project; prints its id there.
    Use {
        name: String,
        /// The project (.chipload).
        project: PathBuf,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// How each project tool stands to the library tool it came from: same, differs (with
    /// the differences), missing, or none. A speed from the setup's material is no
    /// difference.
    Compare {
        /// The project (.chipload).
        project: PathBuf,
    },
    /// The project's tools that differ from their library tools take the library's values;
    /// a speed from the setup's material stays.
    Update {
        /// The project (.chipload).
        project: PathBuf,
        /// Only this tool (its id in the project).
        #[arg(long)]
        tool: Option<i64>,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Save a project's tool into the library; the project's tool is linked to it.
    Save {
        /// The project (.chipload).
        project: PathBuf,
        /// The tool id in the project.
        #[arg(long)]
        tool: i64,
        /// Its name in the library.
        #[arg(long)]
        name: String,
        /// Overwrite a library tool of that name.
        #[arg(long)]
        replace: bool,
    },
}

/// Tool values: lengths in mm, feeds in mm/min.
#[derive(clap::Args)]
pub struct ToolValues {
    #[arg(long = "type")]
    kind: Option<TypeArg>,
    #[arg(value_parser = crate::lengths::length, long)]
    diameter: Option<f64>,
    #[arg(long)]
    flutes: Option<i64>,
    #[arg(long)]
    center_cutting: Option<YesNo>,
    #[arg(long)]
    rpm: Option<f64>,
    /// V-bits: the included angle, degrees. Tapered ball noses: the taper's.
    #[arg(long)]
    angle: Option<f64>,
    /// V-bits: the diameter of the flat tip, mm (0: pointed). Tapered ball noses: the
    /// ball's diameter at the tip.
    #[arg(value_parser = crate::lengths::length, long)]
    tip: Option<f64>,
    /// End mills: the radius of its corners, a bull nose (0: square corners).
    #[arg(value_parser = crate::lengths::length, long)]
    corner_radius: Option<f64>,
    /// The id it gets in a project where that id is free (default 1).
    #[arg(long)]
    id: Option<i64>,
}

/// A row of a tool's cutting data (docs/specs/cutting-data.md §2): lengths in mm, feeds in
/// mm/min. A new row needs every value but the speed; a row there keeps what isn't given.
#[derive(clap::Args)]
pub struct RowValues {
    /// Spindle speed; default: the tool's own.
    #[arg(long)]
    rpm: Option<f64>,
    #[arg(value_parser = crate::lengths::feed, long)]
    plunge_feed: Option<f64>,
    /// Full-width cuts: profiles, paths, V-carves, a pocket's first ring.
    #[arg(value_parser = crate::lengths::feed, long)]
    slot_feed: Option<f64>,
    #[arg(value_parser = crate::lengths::length, long)]
    slot_step_down: Option<f64>,
    /// Cuts at a stepover: a pocket's rings, a surface.
    #[arg(value_parser = crate::lengths::feed, long)]
    feed: Option<f64>,
    #[arg(value_parser = crate::lengths::length, long)]
    step_down: Option<f64>,
    /// Fraction of the tool diameter.
    #[arg(long)]
    stepover: Option<f64>,
}

#[derive(Clone, Copy, ValueEnum)]
enum TypeArg {
    Endmill,
    Drill,
    Vbit,
    /// A ball nose; with --angle and --tip, a tapered one.
    Ballnose,
}

#[derive(Clone, Copy, ValueEnum)]
enum YesNo {
    Yes,
    No,
}

pub fn tool(out: &mut Out, action: ToolAction) -> ExitCode {
    let dir = match dir() {
        Ok(dir) => dir,
        Err(err) => return out.fail(err),
    };
    let path = tools_path(&dir);
    let result = match action {
        ToolAction::List => return list_tools(out, &path),
        ToolAction::Export { file, names } => {
            return export_file::<ToolLibrary>(out, &path, &file, &names);
        }
        ToolAction::Import { file, replace } => {
            return import_file::<ToolLibrary>(out, &path, &file, replace, |_| Vec::new());
        }
        ToolAction::Add { name, values } => update_tools(&path, |library| {
            if library.find(&name).is_some() {
                return Err(refused(format!(
                    "the library already has a tool named \"{name}\" (use `tool set`)"
                )));
            }
            let diameter = values
                .diameter
                .ok_or_else(|| refused("a new tool needs --diameter".into()))?;
            let mut entry = LibraryTool {
                name: name.clone(),
                uid: library.new_uid(&name),
                id: 1,
                kind: ToolType::Endmill,
                diameter,
                flutes: None,
                center_cutting: None,
                rpm: None,
                cutting_data: Vec::new(),
                angle: None,
                tip_diameter: None,
                corner_radius: None,
            };
            set_tool_values(&mut entry, &values)?;
            library.tools.push(entry);
            Ok(format!("Added tool \"{name}\"."))
        }),
        ToolAction::Set {
            name,
            values,
            rename,
        } => update_tools(&path, |library| {
            let i = library
                .position(&name)
                .ok_or_else(|| refused(format!("no tool named \"{name}\"")))?;
            set_tool_values(&mut library.tools[i], &values)?;
            if let Some(new) = rename {
                rename_default(&mut library.default_tool, &name, &new);
                library.tools[i].name = new;
            }
            Ok(format!("Changed tool \"{}\".", library.tools[i].name))
        }),
        ToolAction::Remove { name } => update_tools(&path, |library| library.remove(&name))
            .map(|removed| format!("Removed tool \"{}\".", removed.name)),
        ToolAction::Cutting {
            name,
            material,
            values,
            remove,
        } => set_cutting_row(&dir, &path, &name, &material, &values, remove),
        ToolAction::Default { name } => update_tools(&path, |library| {
            let entry = library
                .find(&name)
                .ok_or_else(|| refused(format!("no tool named \"{name}\"")))?;
            let name = entry.name.clone();
            library.default_tool = Some(name.clone());
            Ok(format!("New projects start with tool \"{name}\"."))
        }),
        ToolAction::Use {
            name,
            project,
            output,
        } => return use_library_tool(out, &path, &name, &project, output.as_deref()),
        ToolAction::Save {
            project,
            tool,
            name,
            replace,
        } => {
            let mut job = match load(out, &project) {
                Ok(job) => job,
                Err(code) => return code,
            };
            let Some(found) = job.tool(tool).cloned() else {
                return out.fail(format!("the project has no tool {tool}"));
            };
            let saved = update_tools(&path, |library| {
                // Cutting values aren't in a project tool: keep the library's. A replaced
                // tool keeps its uid, so other projects stay linked to it.
                match library.position(&name) {
                    Some(i) if replace => {
                        let old = &library.tools[i];
                        let entry = LibraryTool::from_tool(
                            &old.name,
                            &old.uid,
                            &found,
                            old.cutting_data.clone(),
                        );
                        library.tools[i] = entry.clone();
                        Ok((
                            entry,
                            format!("Replaced library tool \"{name}\" with tool {tool}."),
                        ))
                    }
                    Some(_) => Err(refused(format!(
                        "the library already has a tool named \"{name}\" (add --replace)"
                    ))),
                    None => {
                        let entry = LibraryTool::from_tool(
                            &name,
                            &library.new_uid(&name),
                            &found,
                            Vec::new(),
                        );
                        library.tools.push(entry.clone());
                        Ok((entry, format!("Saved tool {tool} as \"{name}\".")))
                    }
                }
            });
            let (entry, line) = match saved {
                Ok(saved) => saved,
                Err(err) => return out.fail(err),
            };
            // The project's tool is now linked to the entry.
            if let Some(t) = job.tools.iter_mut().find(|t| t.id == tool) {
                t.name = Some(entry.name.clone());
                t.library_uid = Some(entry.uid.clone());
            }
            if let Err(err) = crate::save_project(out, &job, &project) {
                return out.fail(err);
            }
            out.wrote(&project, "");
            Ok(line)
        }
        ToolAction::Compare { project } => return compare_tools(out, &path, &project),
        ToolAction::Update {
            project,
            tool,
            output,
        } => return update_tools_in_project(out, &path, &project, tool, output.as_deref()),
    };
    finish(out, result, &path)
}

/// `tool compare` (docs/specs/library-in-browser.md §7).
/// The materials beside the tool library at `path`, to tell a speed from the material
/// (docs/specs/tool-updates.md §4); none when there is no file to read.
fn materials_beside(path: &Path) -> MaterialLibrary {
    path.parent()
        .and_then(|dir| load_library(&chipload_pipeline::library::materials_path(dir)).ok())
        .unwrap_or_default()
}

fn compare_tools(out: &mut Out, path: &Path, project: &Path) -> ExitCode {
    let library = match load_tools(path) {
        Ok(library) => library,
        Err(err) => return out.fail(err),
    };
    let materials = materials_beside(path);
    let job = match load(out, project) {
        Ok(job) => job,
        Err(code) => return code,
    };
    let mut rows = Vec::new();
    for tool in &job.tools {
        let link = chipload_pipeline::library::project_tool_link(&job, &library, &materials, tool);
        out.line(format!("  T{}: {}", tool.id, link.text(LinkEntry::Tool)));
        rows.push(link_json(&link, serde_json::json!({ "id": tool.id })));
    }
    out.set("tools", serde_json::Value::Array(rows));
    ExitCode::SUCCESS
}

/// `tool update` (docs/specs/tool-updates.md §5): the project's tools that differ from their
/// library tools take their values, a speed from the material kept.
fn update_tools_in_project(
    out: &mut Out,
    path: &Path,
    project: &Path,
    only: Option<i64>,
    output: Option<&Path>,
) -> ExitCode {
    let library = match load_tools(path) {
        Ok(library) => library,
        Err(err) => return out.fail(err),
    };
    let materials = materials_beside(path);
    let job = match load(out, project) {
        Ok(job) => job,
        Err(code) => return code,
    };
    if let Some(id) = only
        && job.tool(id).is_none()
    {
        return out.fail(format!("the project has no tool {id}"));
    }
    let (job, updated) =
        chipload_pipeline::library::update_from_library(&job, &library, &materials, only);
    let mut rows = Vec::new();
    for (id, changes) in &updated {
        let text: Vec<String> = changes.iter().map(Message::english).collect();
        out.line(format!("Updated T{id}: {}", text.join("; ")));
        rows.push(serde_json::json!({ "id": id, "changes": text }));
    }
    out.set("updated", serde_json::Value::Array(rows));
    if updated.is_empty() {
        out.line("Every tool from the library has its values: nothing to update.");
        return ExitCode::SUCCESS;
    }
    let target = output.unwrap_or(project);
    if let Err(err) = crate::save_project(out, &job, target) {
        return out.fail(err);
    }
    out.wrote(target, "");
    ExitCode::SUCCESS
}

/// A link as JSON: `state`, the entry's `name` and the `changes`, added to `row`.
pub(super) fn link_json(link: &Link, mut row: serde_json::Value) -> serde_json::Value {
    let (name, changes) = match link {
        Link::None => (None, Vec::new()),
        Link::Missing(name) => (name.clone(), Vec::new()),
        Link::Same(name) => (Some(name.clone()), Vec::new()),
        Link::Differs(name, changes) => (
            Some(name.clone()),
            changes.iter().map(Message::english).collect(),
        ),
    };
    if let Some(map) = row.as_object_mut() {
        map.insert("state".into(), serde_json::json!(link.state()));
        map.insert("name".into(), serde_json::json!(name));
        map.insert("changes".into(), serde_json::json!(changes));
    }
    row
}

fn set_tool_values(entry: &mut LibraryTool, values: &ToolValues) -> Result<(), LibraryError> {
    if let Some(kind) = values.kind {
        entry.kind = match kind {
            TypeArg::Endmill => ToolType::Endmill,
            TypeArg::Drill => ToolType::Drill,
            TypeArg::Vbit => ToolType::Vbit,
            TypeArg::Ballnose => ToolType::Ballnose,
        };
    }
    if let Some(d) = values.diameter {
        entry.diameter = d;
    }
    if let Some(f) = values.flutes {
        entry.flutes = Some(f);
    }
    if let Some(c) = values.center_cutting {
        entry.center_cutting = Some(matches!(c, YesNo::Yes));
    }
    if let Some(rpm) = values.rpm {
        entry.rpm = Some(rpm);
    }
    // Degrees on the command line, radians in the file.
    if let Some(angle) = values.angle {
        entry.angle = Some(angle.to_radians());
    }
    if let Some(tip) = values.tip {
        entry.tip_diameter = Some(tip);
    }
    if let Some(corner) = values.corner_radius {
        entry.corner_radius = (corner != 0.0).then_some(corner);
    }
    if let Some(id) = values.id {
        if id < 1 {
            return Err(refused(format!("--id must be 1 or more, got {id}")));
        }
        entry.id = id;
    }
    Ok(())
}

/// `tool cutting`: set or remove tool `name`'s row for `material` (a name, or `any`).
fn set_cutting_row(
    dir: &Path,
    path: &Path,
    name: &str,
    material: &str,
    values: &RowValues,
    remove: bool,
) -> Result<String, LibraryError> {
    let (uid, shown) = if material.eq_ignore_ascii_case("any") {
        (None, "any material".to_string())
    } else {
        let materials: MaterialLibrary =
            load_library(&chipload_pipeline::library::materials_path(dir))?;
        let found = materials
            .find(material)
            .ok_or_else(|| refused(format!("no material named \"{material}\"")))?;
        (Some(found.uid.clone()), found.name.clone())
    };
    update_tools(path, |library| {
        let i = library
            .position(name)
            .ok_or_else(|| refused(format!("no tool named \"{name}\"")))?;
        let entry = &mut library.tools[i];
        if remove {
            return if entry.remove_row(uid.as_deref()) {
                Ok(format!("Removed {shown} from \"{}\".", entry.name))
            } else {
                Err(refused(format!(
                    "\"{}\" has no row for {shown}",
                    entry.name
                )))
            };
        }
        let old = entry.row(uid.as_deref()).map(|r| r.values);
        let mut missing = Vec::new();
        let mut pick = |new: Option<f64>, old: Option<f64>, flag: &'static str| {
            new.or(old).unwrap_or_else(|| {
                missing.push(format!("--{flag}"));
                0.0
            })
        };
        let row = Cutting {
            rpm: values.rpm.or(old.and_then(|c| c.rpm)),
            plunge_feed: pick(
                values.plunge_feed,
                old.map(|c| c.plunge_feed),
                "plunge-feed",
            ),
            slot_feed: pick(values.slot_feed, old.map(|c| c.slot_feed), "slot-feed"),
            slot_step_down: pick(
                values.slot_step_down,
                old.map(|c| c.slot_step_down),
                "slot-step-down",
            ),
            feed: pick(values.feed, old.map(|c| c.feed), "feed"),
            step_down: pick(values.step_down, old.map(|c| c.step_down), "step-down"),
            stepover: pick(values.stepover, old.map(|c| c.stepover), "stepover"),
        };
        if !missing.is_empty() {
            return Err(refused(format!(
                "a new row needs every value but --rpm (missing {})",
                missing.join(", ")
            )));
        }
        entry.set_row(uid.as_deref(), row);
        Ok(format!("Set {shown} for \"{}\".", entry.name))
    })
}

/// A row's values in one line: "18000 rpm, plunge 500, slot 1500 / 2.5, clearing 1800 / 5 ×
/// 0.4" (mm/min and mm).
pub(crate) fn row_text(c: &Cutting) -> String {
    let rpm = c.rpm.map_or_else(String::new, |r| format!("{r} rpm, "));
    format!(
        "{rpm}plunge {} mm/min, slot {} mm/min / {} mm, clearing {} mm/min / {} mm × {}",
        c.plunge_feed, c.slot_feed, c.slot_step_down, c.feed, c.step_down, c.stepover
    )
}

fn list_tools(out: &mut Out, path: &Path) -> ExitCode {
    // Names for the rows' materials; a library without materials names them by uid.
    let materials: MaterialLibrary = path
        .parent()
        .and_then(|dir| load_library(&chipload_pipeline::library::materials_path(dir)).ok())
        .unwrap_or_default();
    let library = match load_tools(path) {
        Ok(library) => library,
        Err(err) => return out.fail(err),
    };
    out.line(format!("Tool library: {}", path.display()));
    out.set("path", serde_json::json!(path.display().to_string()));
    out.set("default", serde_json::json!(library.default_tool));
    out.set("tools", serde_json::json!(library.tools));
    if library.tools.is_empty() {
        out.line("  (no tools)");
    }
    for t in &library.tools {
        let is_default = library
            .default_tool
            .as_deref()
            .is_some_and(|d| chipload_job::library::same_name(d, &t.name));
        let kind = t.kind.name();
        let mut line = format!(
            "  {}{}: T{}, {kind} {} mm",
            t.name,
            mark(is_default),
            t.id,
            t.diameter
        );
        line.push_str(&optional("flutes", t.flutes, ""));
        line.push_str(&optional(
            "centre cutting",
            t.center_cutting.map(|c| if c { "yes" } else { "no" }),
            "",
        ));
        line.push_str(&optional("rpm", t.rpm, ""));
        line.push_str(&optional(
            "angle",
            t.angle.map(|a| format!("{:.1}", a.to_degrees())),
            "°",
        ));
        line.push_str(&optional("tip", t.tip_diameter, " mm"));
        line.push_str(&optional("corner radius", t.corner_radius, " mm"));
        line.push_str(&format!(" (uid {})", t.uid));
        out.line(line.to_string());
        for row in &t.cutting_data {
            let material = match &row.material_uid {
                None => "any material".to_string(),
                Some(uid) => materials
                    .find_uid(uid)
                    .map_or_else(|| format!("material {uid}"), |m| m.name.clone()),
            };
            out.line(format!("    {material}: {}", row_text(&row.values)));
        }
    }
    ExitCode::SUCCESS
}

fn use_library_tool(
    out: &mut Out,
    path: &Path,
    name: &str,
    project: &Path,
    output: Option<&Path>,
) -> ExitCode {
    let library = match load_tools(path) {
        Ok(library) => library,
        Err(err) => return out.fail(err),
    };
    let Some(entry) = library.find(name) else {
        return out.fail(format!("no tool named \"{name}\" in {}", path.display()));
    };
    let mut job = match load(out, project) {
        Ok(job) => job,
        Err(code) => return code,
    };
    let used = use_tool(&mut job, entry);
    let target = output.unwrap_or(project);
    if let Err(err) = crate::save_project(out, &job, target) {
        return out.fail(err);
    }
    out.line(format!("\"{}\": {}", entry.name, used.describe(entry.id)));
    out.set("id", serde_json::json!(used.id));
    out.set("added", serde_json::json!(used.added));
    out.wrote(target, "");
    ExitCode::SUCCESS
}
