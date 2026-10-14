// SPDX-License-Identifier: GPL-3.0-or-later
//! `chipload tool …` and `chipload machine …`: the user's tool library and machine profiles
//! (docs/specs/tool-library-and-machines.md §7).

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use chipload_job::library::{
    Collection, Cutting, LibraryError, LibraryTool, Link, MachineLibrary, MachineProfile,
    ToolLibrary, export, import_into, load_library, load_machines, load_tools, machine_link,
    read_import, tool_link, update_library, update_machines, update_tools, use_tool, write_export,
};
use chipload_job::{Spindle, StockCorner, ToolChange, ToolType, Travel, WorkZero, save_job};
use chipload_pipeline::library::{
    CONFIG_DIR_VAR, copy_profile_post, machines_path, settings_dir, tools_path, unknown_posts,
};
use chipload_pipeline::posts::{
    CopyState, choose_post, copy_state, post_choices, post_exists, post_facts, posts_dir,
};
use chipload_pipeline::{DEFAULT_MAX_OVERCUT, DEFAULT_SAFE_Z};
use clap::{Subcommand, ValueEnum};

use crate::load;
use crate::out::Out;

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
    /// the differences), missing, or none.
    Compare {
        /// The project (.chipload).
        project: PathBuf,
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
    #[arg(long)]
    diameter: Option<f64>,
    #[arg(long)]
    flutes: Option<i64>,
    #[arg(long)]
    center_cutting: Option<YesNo>,
    #[arg(long)]
    rpm: Option<f64>,
    /// V-bits: the included angle, degrees.
    #[arg(long)]
    angle: Option<f64>,
    /// V-bits: the diameter of the flat tip, mm (0: pointed).
    #[arg(long)]
    tip: Option<f64>,
    /// The id it gets in a project where that id is free (default 1).
    #[arg(long)]
    id: Option<i64>,
    /// Cutting values for new operations: give all four the first time.
    #[arg(long)]
    feed: Option<f64>,
    #[arg(long)]
    plunge_feed: Option<f64>,
    #[arg(long)]
    step_down: Option<f64>,
    /// Fraction of the tool diameter (pockets).
    #[arg(long)]
    stepover: Option<f64>,
}

#[derive(Clone, Copy, ValueEnum)]
enum TypeArg {
    Endmill,
    Drill,
    Vbit,
}

#[derive(Clone, Copy, ValueEnum)]
enum YesNo {
    Yes,
    No,
}

#[derive(Subcommand)]
pub enum MachineAction {
    /// Write the machine profiles to a file: a backup, or for another computer.
    Export {
        file: PathBuf,
        /// Only these (repeatable); default: all.
        #[arg(long = "name")]
        names: Vec<String>,
    },
    /// Add the machine profiles of an exported file (or another computer's library file).
    Import {
        file: PathBuf,
        /// Overwrite yours where the file has other values under the same name.
        #[arg(long)]
        replace: bool,
    },
    /// List the machine profiles and where they are kept.
    List,
    /// Add a machine profile. Values not given are the built-in project defaults.
    Add {
        name: String,
        #[command(flatten)]
        values: MachineValues,
    },
    /// Change a machine profile.
    Set {
        name: String,
        #[command(flatten)]
        values: MachineValues,
        /// A new name.
        #[arg(long)]
        rename: Option<String>,
    },
    /// Remove a machine profile.
    Remove { name: String },
    /// The machine new projects start with.
    Default { name: String },
    /// Copy a profile's values into a project; lists what changed.
    Use {
        name: String,
        /// The project (.chipload).
        project: PathBuf,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// How the project's machine stands to the profile it came from: same, differs (with
    /// the differences), missing, or none.
    Compare {
        /// The project (.chipload).
        project: PathBuf,
    },
    /// Save a project's machine values as a profile; the project is linked to it.
    Save {
        /// The project (.chipload).
        project: PathBuf,
        /// The profile's name.
        #[arg(long)]
        name: String,
        /// Overwrite a profile of that name.
        #[arg(long)]
        replace: bool,
    },
}

/// Machine values: lengths in mm, feeds in mm/min.
#[derive(clap::Args)]
pub struct MachineValues {
    /// Built-in post-processor.
    #[arg(long)]
    post: Option<String>,
    #[arg(long, allow_hyphen_values = true)]
    safe_z: Option<f64>,
    #[arg(long)]
    spindle: Option<SpindleArg>,
    #[arg(long)]
    max_feed: Option<f64>,
    /// Travel "X,Y,Z".
    #[arg(long)]
    travel: Option<String>,
    /// drawing-origin, stock-center, stock-bottom-left, stock-bottom-right,
    /// stock-top-left, stock-top-right, or a point "X,Y".
    #[arg(long, allow_hyphen_values = true)]
    work_zero: Option<String>,
    /// How far below the stock a cut may go (into the spoilboard).
    #[arg(long)]
    max_overcut: Option<f64>,
    /// With several tools: one G-code file per tool, or one file with M6 tool changes
    /// that your sender handles.
    #[arg(long)]
    tool_change: Option<ToolChangeArg>,
}

/// docs/specs/tool-per-operation.md §3.
#[derive(Clone, Copy, ValueEnum)]
pub enum ToolChangeArg {
    /// One G-code file per tool group.
    Files,
    /// One file with `T<n> M6` at each tool change.
    Program,
}

impl ToolChangeArg {
    pub fn value(self) -> ToolChange {
        match self {
            ToolChangeArg::Files => ToolChange::Files,
            ToolChangeArg::Program => ToolChange::Program,
        }
    }
}

#[derive(Clone, Copy, ValueEnum)]
enum SpindleArg {
    Cw,
    Ccw,
}

/// The settings folder, or an error naming the variable that sets it.
fn dir() -> Result<PathBuf, String> {
    settings_dir().ok_or_else(|| {
        format!("can't find your settings folder (no home folder); set {CONFIG_DIR_VAR}")
    })
}

fn refused(text: String) -> LibraryError {
    LibraryError::Refused(text)
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
                cutting: None,
                angle: None,
                tip_diameter: None,
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
        ToolAction::Remove { name } => update_tools(&path, |library| {
            let removed = library.remove(&name)?;
            Ok(format!("Removed tool \"{}\".", removed.name))
        }),
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
            let Some(found) = job.tools.iter().find(|t| t.id == tool).cloned() else {
                return out.fail(format!("the project has no tool {tool}"));
            };
            let saved = update_tools(&path, |library| {
                // Cutting values aren't in a project tool: keep the library's. A replaced
                // tool keeps its uid, so other projects stay linked to it.
                match library.position(&name) {
                    Some(i) if replace => {
                        let old = &library.tools[i];
                        let entry =
                            LibraryTool::from_tool(&old.name, &old.uid, &found, old.cutting);
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
                        let entry =
                            LibraryTool::from_tool(&name, &library.new_uid(&name), &found, None);
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
            if let Err(err) = save_job(&job, &project) {
                return out.fail(err);
            }
            out.wrote(&project, "");
            Ok(line)
        }
        ToolAction::Compare { project } => return compare_tools(out, &path, &project),
    };
    finish(out, result, &path)
}

/// `tool compare` (docs/specs/library-in-browser.md §7).
fn compare_tools(out: &mut Out, path: &Path, project: &Path) -> ExitCode {
    let library = match load_tools(path) {
        Ok(library) => library,
        Err(err) => return out.fail(err),
    };
    let job = match load(out, project) {
        Ok(job) => job,
        Err(code) => return code,
    };
    let mut rows = Vec::new();
    for tool in &job.tools {
        let link = tool_link(&library, tool);
        out.line(format!("  T{}: {}", tool.id, link.text("library tool")));
        rows.push(link_json(&link, serde_json::json!({ "id": tool.id })));
    }
    out.set("tools", serde_json::Value::Array(rows));
    ExitCode::SUCCESS
}

/// A link as JSON: `state`, the entry's `name` and the `changes`, added to `row`.
fn link_json(link: &Link, mut row: serde_json::Value) -> serde_json::Value {
    let (name, changes) = match link {
        Link::None => (None, Vec::new()),
        Link::Missing(name) => (name.clone(), Vec::new()),
        Link::Same(name) => (Some(name.clone()), Vec::new()),
        Link::Differs(name, changes) => (Some(name.clone()), changes.clone()),
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
    if let Some(id) = values.id {
        if id < 1 {
            return Err(refused(format!("--id must be 1 or more, got {id}")));
        }
        entry.id = id;
    }
    let given = [
        values.feed,
        values.plunge_feed,
        values.step_down,
        values.stepover,
    ];
    if given.iter().any(Option::is_some) {
        let old = entry.cutting;
        let pick = |new: Option<f64>, old: Option<f64>, flag: &str| {
            new.or(old).ok_or_else(|| {
                refused(format!(
                    "cutting values come as a set: give --feed, --plunge-feed, --step-down \
                     and --stepover (missing --{flag})"
                ))
            })
        };
        entry.cutting = Some(Cutting {
            feed: pick(values.feed, old.map(|c| c.feed), "feed")?,
            plunge_feed: pick(
                values.plunge_feed,
                old.map(|c| c.plunge_feed),
                "plunge-feed",
            )?,
            step_down: pick(values.step_down, old.map(|c| c.step_down), "step-down")?,
            stepover: pick(values.stepover, old.map(|c| c.stepover), "stepover")?,
        });
    }
    Ok(())
}

fn rename_default(default: &mut Option<String>, old: &str, new: &str) {
    if default
        .as_deref()
        .is_some_and(|d| chipload_job::library::same_name(d, old))
    {
        *default = Some(new.to_string());
    }
}

/// `tool export`, `machine export` (docs/specs/library-import-export.md §3).
fn export_file<L: Collection>(
    out: &mut Out,
    path: &Path,
    file: &Path,
    names: &[String],
) -> ExitCode {
    let exported = load_library::<L>(path).and_then(|library| export(&library, names));
    match exported.and_then(|out| write_export(&out, file).map(|()| out.entries().len())) {
        Ok(count) => {
            out.line(format!("Exported {}: {count}", L::LIST));
            out.wrote(file, "");
            ExitCode::SUCCESS
        }
        Err(err) => out.fail(err),
    }
}

/// `tool import`, `machine import` (§4): one save of the library, then the report.
fn import_file<L: Collection>(
    out: &mut Out,
    path: &Path,
    file: &Path,
    replace: bool,
    notes: impl Fn(&L) -> Vec<String>,
) -> ExitCode {
    let imported = read_import::<L>(file).and_then(|from| {
        let plan = update_library(path, |library: &mut L| {
            Ok(import_into(library, &from, replace))
        })?;
        Ok((plan, notes(&from)))
    });
    match imported {
        Ok((plan, notes)) => {
            for line in plan.report(L::LIST, replace).iter().chain(&notes) {
                out.line(line.to_string());
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

fn finish(out: &mut Out, result: Result<String, LibraryError>, path: &Path) -> ExitCode {
    match result {
        Ok(line) => {
            out.line(line.to_string());
            out.wrote(path, "");
            ExitCode::SUCCESS
        }
        Err(err) => out.fail(err),
    }
}

fn mark(is_default: bool) -> &'static str {
    if is_default { " (default)" } else { "" }
}

fn optional<T: std::fmt::Display>(label: &str, value: Option<T>, unit: &str) -> String {
    value.map_or_else(String::new, |v| format!(", {label} {v}{unit}"))
}

fn list_tools(out: &mut Out, path: &Path) -> ExitCode {
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
        let kind = match t.kind {
            ToolType::Endmill => "endmill",
            ToolType::Drill => "drill",
            ToolType::Vbit => "v-bit",
        };
        let mut line = format!(
            "  {}{}: id {}, {kind} {} mm",
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
        if let Some(c) = t.cutting {
            line.push_str(&format!(
                "; feed {} plunge {} mm/min, step down {} mm, stepover {}",
                c.feed, c.plunge_feed, c.step_down, c.stepover
            ));
        }
        line.push_str(&format!(" (uid {})", t.uid));
        out.line(line.to_string());
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
    if let Err(err) = save_job(&job, target) {
        return out.fail(err);
    }
    out.line(format!("\"{}\": {}", entry.name, used.describe(entry.id)));
    out.set("id", serde_json::json!(used.id));
    out.set("added", serde_json::json!(used.added));
    out.wrote(target, "");
    ExitCode::SUCCESS
}

pub fn machine(out: &mut Out, action: MachineAction) -> ExitCode {
    let dir = match dir() {
        Ok(dir) => dir,
        Err(err) => return out.fail(err),
    };
    let path = machines_path(&dir);
    let posts = posts_dir(&dir);
    let result = match action {
        MachineAction::List => return list_machines(out, &path),
        MachineAction::Export { file, names } => {
            return export_file::<MachineLibrary>(out, &path, &file, &names);
        }
        MachineAction::Import { file, replace } => {
            return import_file::<MachineLibrary>(out, &path, &file, replace, |from| {
                unknown_posts(from, Some(&posts))
            });
        }
        MachineAction::Add { name, values } => update_machines(&path, |library| {
            if library.find(&name).is_some() {
                return Err(refused(format!(
                    "there is already a machine named \"{name}\" (use `machine set`)"
                )));
            }
            let mut profile = MachineProfile {
                name: name.clone(),
                uid: library.new_uid(&name),
                post: "grbl".into(),
                safe_z: DEFAULT_SAFE_Z,
                spindle: Spindle::Cw,
                max_feed: None,
                travel: None,
                work_zero: WorkZero::DrawingOrigin {},
                max_overcut: DEFAULT_MAX_OVERCUT,
                tool_change: ToolChange::Files,
            };
            set_machine_values(&mut profile, &values, &posts)?;
            library.machines.push(profile);
            Ok(format!("Added machine \"{name}\"."))
        }),
        MachineAction::Set {
            name,
            values,
            rename,
        } => update_machines(&path, |library| {
            let i = library
                .position(&name)
                .ok_or_else(|| refused(format!("no machine named \"{name}\"")))?;
            set_machine_values(&mut library.machines[i], &values, &posts)?;
            if let Some(new) = rename {
                rename_default(&mut library.default_machine, &name, &new);
                library.machines[i].name = new;
            }
            Ok(format!("Changed machine \"{}\".", library.machines[i].name))
        }),
        MachineAction::Remove { name } => update_machines(&path, |library| {
            let removed = library.remove(&name)?;
            Ok(format!("Removed machine \"{}\".", removed.name))
        }),
        MachineAction::Default { name } => update_machines(&path, |library| {
            let profile = library
                .find(&name)
                .ok_or_else(|| refused(format!("no machine named \"{name}\"")))?;
            let name = profile.name.clone();
            library.default_machine = Some(name.clone());
            Ok(format!("New projects start with machine \"{name}\"."))
        }),
        MachineAction::Use {
            name,
            project,
            output,
        } => return use_machine(out, &path, &posts, &name, &project, output.as_deref()),
        MachineAction::Save {
            project,
            name,
            replace,
        } => {
            let mut job = match load(out, &project) {
                Ok(job) => job,
                Err(code) => return code,
            };
            // A replaced profile keeps its name and uid, so other projects stay linked.
            let saved = update_machines(&path, |library| match library.position(&name) {
                Some(i) if replace => {
                    let old = &library.machines[i];
                    let profile = MachineProfile::from_job(&old.name, &old.uid, &job);
                    library.machines[i] = profile.clone();
                    Ok((
                        profile,
                        format!("Replaced machine \"{name}\" with the project's values."),
                    ))
                }
                Some(_) => Err(refused(format!(
                    "there is already a machine named \"{name}\" (add --replace)"
                ))),
                None => {
                    let profile = MachineProfile::from_job(&name, &library.new_uid(&name), &job);
                    library.machines.push(profile.clone());
                    Ok((
                        profile,
                        format!("Saved the project's machine as \"{name}\"."),
                    ))
                }
            });
            let (profile, line) = match saved {
                Ok(saved) => saved,
                Err(err) => return out.fail(err),
            };
            job.machine.name = Some(profile.name.clone());
            job.machine.library_uid = Some(profile.uid.clone());
            if let Err(err) = save_job(&job, &project) {
                return out.fail(err);
            }
            out.wrote(&project, "");
            Ok(line)
        }
        MachineAction::Compare { project } => {
            let library = match load_machines(&path) {
                Ok(library) => library,
                Err(err) => return out.fail(err),
            };
            let job = match load(out, &project) {
                Ok(job) => job,
                Err(code) => return code,
            };
            let link = machine_link(&library, &job);
            out.line(format!("Machine: {}", link.text("profile")));
            out.set("machine", link_json(&link, serde_json::json!({})));
            return ExitCode::SUCCESS;
        }
    };
    finish(out, result, &path)
}

fn set_machine_values(
    profile: &mut MachineProfile,
    values: &MachineValues,
    posts: &Path,
) -> Result<(), LibraryError> {
    if let Some(post) = &values.post {
        if !post_exists(post, Some(posts)) {
            return Err(refused(format!(
                "no post-processor \"{post}\" (built in or in {}; see `chipload post list`)",
                posts.display()
            )));
        }
        profile.post = post.clone();
    }
    if let Some(z) = values.safe_z {
        profile.safe_z = z;
    }
    if let Some(spindle) = values.spindle {
        profile.spindle = match spindle {
            SpindleArg::Cw => Spindle::Cw,
            SpindleArg::Ccw => Spindle::Ccw,
        };
    }
    if let Some(feed) = values.max_feed {
        profile.max_feed = Some(feed);
    }
    if let Some(text) = &values.travel {
        profile.travel = Some(travel_arg(text).map_err(refused)?);
    }
    if let Some(text) = &values.work_zero {
        profile.work_zero = work_zero_arg(text).map_err(refused)?;
    }
    if let Some(overcut) = values.max_overcut {
        profile.max_overcut = overcut;
    }
    if let Some(change) = values.tool_change {
        profile.tool_change = change.value();
    }
    Ok(())
}

fn travel_arg(text: &str) -> Result<Travel, String> {
    let numbers: Option<Vec<f64>> = text.split(',').map(|n| n.trim().parse().ok()).collect();
    match numbers.as_deref() {
        Some(&[x, y, z]) => Ok(Travel { x, y, z }),
        _ => Err(format!("expected travel as X,Y,Z, got {text:?}")),
    }
}

fn work_zero_arg(text: &str) -> Result<WorkZero, String> {
    let corner = match text {
        "drawing-origin" => return Ok(WorkZero::DrawingOrigin {}),
        "stock-center" => StockCorner::Center,
        "stock-bottom-left" => StockCorner::BottomLeft,
        "stock-bottom-right" => StockCorner::BottomRight,
        "stock-top-left" => StockCorner::TopLeft,
        "stock-top-right" => StockCorner::TopRight,
        point => {
            let [x, y] = crate::point_arg(point).map_err(|_| {
                format!(
                    "expected drawing-origin, stock-center, stock-bottom-left, \
                     stock-bottom-right, stock-top-left, stock-top-right or a point X,Y; \
                     got {point:?}"
                )
            })?;
            return Ok(WorkZero::Point { x, y });
        }
    };
    Ok(WorkZero::Stock { corner })
}

fn list_machines(out: &mut Out, path: &Path) -> ExitCode {
    let library = match load_machines(path) {
        Ok(library) => library,
        Err(err) => return out.fail(err),
    };
    out.line(format!("Machine profiles: {}", path.display()));
    out.set("path", serde_json::json!(path.display().to_string()));
    out.set("default", serde_json::json!(library.default_machine));
    out.set("machines", serde_json::json!(library.machines));
    if library.machines.is_empty() {
        out.line("  (no machines)");
    }
    for m in &library.machines {
        let is_default = library
            .default_machine
            .as_deref()
            .is_some_and(|d| chipload_job::library::same_name(d, &m.name));
        let spindle = match m.spindle {
            Spindle::Cw => "cw",
            Spindle::Ccw => "ccw",
        };
        let mut line = format!(
            "  {}{}: post {}, safe Z {} mm, spindle {spindle}, max overcut {} mm, tool changes {}",
            m.name,
            mark(is_default),
            m.post,
            m.safe_z,
            m.max_overcut,
            m.tool_change.name()
        );
        line.push_str(&optional("max feed", m.max_feed, " mm/min"));
        if let Some(t) = &m.travel {
            line.push_str(&format!(", travel X{} Y{} Z{} mm", t.x, t.y, t.z));
        }
        line.push_str(&format!(
            ", work zero {}",
            match m.work_zero {
                WorkZero::DrawingOrigin {} => "drawing origin".to_string(),
                WorkZero::Stock { corner } => format!("stock {}", corner.name()),
                WorkZero::Point { x, y } => format!("point X{x} Y{y}"),
            }
        ));
        line.push_str(&format!(" (uid {})", m.uid));
        out.line(line.to_string());
    }
    ExitCode::SUCCESS
}

fn use_machine(
    out: &mut Out,
    path: &Path,
    posts: &Path,
    name: &str,
    project: &Path,
    output: Option<&Path>,
) -> ExitCode {
    let library = match load_machines(path) {
        Ok(library) => library,
        Err(err) => return out.fail(err),
    };
    let Some(profile) = library.find(name) else {
        return out.fail(format!("no machine named \"{name}\" in {}", path.display()));
    };
    let mut job = match load(out, project) {
        Ok(job) => job,
        Err(code) => return code,
    };
    let mut changes = profile.apply(&mut job);
    changes.extend(copy_profile_post(&mut job, Some(posts)));
    let target = output.unwrap_or(project);
    if let Err(err) = save_job(&job, target) {
        return out.fail(err);
    }
    out.line(format!("Machine \"{}\":", profile.name));
    if changes.is_empty() {
        out.line("  no values changed");
    }
    for change in changes {
        out.line(format!("  {change}"));
    }
    out.wrote(target, "");
    ExitCode::SUCCESS
}

// -- post-processors (docs/specs/user-posts.md §6)

#[derive(Subcommand)]
pub enum PostAction {
    /// List the built-in post-processors and yours, and where yours go.
    List,
    /// What a post-processor writes: its name, files, extension, decimals, arcs, tool change
    /// modes and spindle dwell.
    Show {
        /// The post's name (`chipload post list`).
        name: String,
    },
    /// Make a post-processor the project's; one of yours is copied into the project (run
    /// again after changing it, to update the copy).
    Use {
        /// The project (.chipload).
        project: PathBuf,
        /// The post's name (`chipload post list`).
        #[arg(long)]
        post: String,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

pub fn post(out: &mut Out, action: PostAction) -> ExitCode {
    let dir = match dir() {
        Ok(dir) => dir,
        Err(err) => return out.fail(err),
    };
    let posts = posts_dir(&dir);
    match action {
        PostAction::Show { name } => match post_facts(&name, Some(&posts)) {
            Ok(lines) => {
                out.line(format!("Post-processor \"{name}\":"));
                for line in &lines {
                    out.line(format!("  {line}"));
                }
                out.set("name", serde_json::json!(name));
                out.set("facts", serde_json::json!(lines));
                ExitCode::SUCCESS
            }
            Err(err) => out.fail(err),
        },
        PostAction::List => {
            out.line(format!("Your post-processors: {}", posts.display()));
            let choices = post_choices(Some(&posts));
            out.set("path", serde_json::json!(posts.display().to_string()));
            out.set(
                "posts",
                serde_json::json!(
                    choices
                        .iter()
                        .map(|c| serde_json::json!({
                            "name": c.name,
                            "builtin": c.builtin,
                            "problem": c.problem,
                        }))
                        .collect::<Vec<_>>()
                ),
            );
            for choice in choices {
                let kind = if choice.builtin { "built in" } else { "yours" };
                match &choice.problem {
                    None => out.line(format!("  {} ({kind})", choice.name)),
                    Some(problem) => {
                        out.line(format!(
                            "  {} ({kind}) can't be used: {problem}",
                            choice.name
                        ));
                    }
                }
            }
            ExitCode::SUCCESS
        }
        PostAction::Use {
            project,
            post,
            output,
        } => {
            let mut job = match load(out, &project) {
                Ok(job) => job,
                Err(code) => return code,
            };
            let before = copy_state(&job, Some(&posts));
            let was = job.machine.post.clone();
            if let Err(err) = choose_post(&mut job, &post, Some(&posts)) {
                return out.fail(err);
            }
            let target = output.as_deref().unwrap_or(&project);
            if let Err(err) = save_job(&job, target) {
                return out.fail(err);
            }
            match (&job.machine.custom_post, was == post, before) {
                (None, _, _) => out.line(format!("Post-processor: {post} (built in)")),
                (Some(_), true, CopyState::Same) => {
                    out.line(format!(
                        "Post-processor: {post} (yours; the copy was up to date)"
                    ));
                }
                (Some(_), true, _) => {
                    out.line(format!("Post-processor: {post} (yours; copy updated)"))
                }
                (Some(_), false, _) => {
                    out.line(format!("Post-processor: {post} (yours, copied in)"))
                }
            }
            out.wrote(target, "");
            ExitCode::SUCCESS
        }
    }
}
