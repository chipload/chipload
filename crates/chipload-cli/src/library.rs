// SPDX-License-Identifier: GPL-3.0-or-later
//! `chipload tool …` and `chipload machine …`: the user's tool library and machine profiles
//! (docs/specs/tool-library-and-machines.md §7).

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use chipload_job::library::{
    Cutting, LibraryError, LibraryTool, MachineProfile, load_machines, load_tools, update_machines,
    update_tools, use_tool,
};
use chipload_job::{Spindle, StockCorner, ToolType, Travel, WorkZero, save_job};
use chipload_pipeline::library::{CONFIG_DIR_VAR, machines_path, settings_dir, tools_path};
use chipload_pipeline::{DEFAULT_MAX_OVERCUT, DEFAULT_SAFE_Z};
use clap::{Subcommand, ValueEnum};

use crate::{fail, load};

#[derive(Subcommand)]
pub enum ToolAction {
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
    /// Save a project's tool into the library.
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
    /// Save a project's machine values as a profile.
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

pub fn tool(action: ToolAction) -> ExitCode {
    let dir = match dir() {
        Ok(dir) => dir,
        Err(err) => return fail(err),
    };
    let path = tools_path(&dir);
    let result = match action {
        ToolAction::List => return list_tools(&path),
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
        } => return use_library_tool(&path, &name, &project, output.as_deref()),
        ToolAction::Save {
            project,
            tool,
            name,
            replace,
        } => {
            let job = match load(&project) {
                Ok(job) => job,
                Err(code) => return code,
            };
            let Some(found) = job.tools.iter().find(|t| t.id == tool) else {
                return fail(format!("the project has no tool {tool}"));
            };
            let entry = LibraryTool::from_tool(&name, found, None);
            update_tools(&path, |library| {
                // Cutting values aren't in a project tool: keep the library's.
                let at = library.position(&name);
                match at {
                    Some(i) if replace => {
                        let cutting = library.tools[i].cutting;
                        library.tools[i] = LibraryTool { cutting, ..entry };
                        Ok(format!(
                            "Replaced library tool \"{name}\" with tool {tool}."
                        ))
                    }
                    Some(_) => Err(refused(format!(
                        "the library already has a tool named \"{name}\" (add --replace)"
                    ))),
                    None => {
                        library.tools.push(entry);
                        Ok(format!("Saved tool {tool} as \"{name}\"."))
                    }
                }
            })
        }
    };
    finish(result, &path)
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

fn finish(result: Result<String, LibraryError>, path: &Path) -> ExitCode {
    match result {
        Ok(line) => {
            println!("{line}");
            println!("Wrote {}", path.display());
            ExitCode::SUCCESS
        }
        Err(err) => fail(err),
    }
}

fn mark(is_default: bool) -> &'static str {
    if is_default { " (default)" } else { "" }
}

fn optional<T: std::fmt::Display>(label: &str, value: Option<T>, unit: &str) -> String {
    value.map_or_else(String::new, |v| format!(", {label} {v}{unit}"))
}

fn list_tools(path: &Path) -> ExitCode {
    let library = match load_tools(path) {
        Ok(library) => library,
        Err(err) => return fail(err),
    };
    println!("Tool library: {}", path.display());
    if library.tools.is_empty() {
        println!("  (no tools)");
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
        println!("{line}");
    }
    ExitCode::SUCCESS
}

fn use_library_tool(path: &Path, name: &str, project: &Path, output: Option<&Path>) -> ExitCode {
    let library = match load_tools(path) {
        Ok(library) => library,
        Err(err) => return fail(err),
    };
    let Some(entry) = library.find(name) else {
        return fail(format!("no tool named \"{name}\" in {}", path.display()));
    };
    let mut job = match load(project) {
        Ok(job) => job,
        Err(code) => return code,
    };
    let used = use_tool(&mut job, entry);
    let target = output.unwrap_or(project);
    if let Err(err) = save_job(&job, target) {
        return fail(err);
    }
    println!("\"{}\": {}", entry.name, used.describe(entry.id));
    println!("Wrote {}", target.display());
    ExitCode::SUCCESS
}

pub fn machine(action: MachineAction) -> ExitCode {
    let dir = match dir() {
        Ok(dir) => dir,
        Err(err) => return fail(err),
    };
    let path = machines_path(&dir);
    let result = match action {
        MachineAction::List => return list_machines(&path),
        MachineAction::Add { name, values } => update_machines(&path, |library| {
            if library.find(&name).is_some() {
                return Err(refused(format!(
                    "there is already a machine named \"{name}\" (use `machine set`)"
                )));
            }
            let mut profile = MachineProfile {
                name: name.clone(),
                post: "grbl".into(),
                safe_z: DEFAULT_SAFE_Z,
                spindle: Spindle::Cw,
                max_feed: None,
                travel: None,
                work_zero: WorkZero::DrawingOrigin {},
                max_overcut: DEFAULT_MAX_OVERCUT,
            };
            set_machine_values(&mut profile, &values)?;
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
            set_machine_values(&mut library.machines[i], &values)?;
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
        } => return use_machine(&path, &name, &project, output.as_deref()),
        MachineAction::Save {
            project,
            name,
            replace,
        } => {
            let job = match load(&project) {
                Ok(job) => job,
                Err(code) => return code,
            };
            let profile = MachineProfile::from_job(&name, &job);
            update_machines(&path, |library| match library.position(&name) {
                Some(i) if replace => {
                    library.machines[i] = profile;
                    Ok(format!(
                        "Replaced machine \"{name}\" with the project's values."
                    ))
                }
                Some(_) => Err(refused(format!(
                    "there is already a machine named \"{name}\" (add --replace)"
                ))),
                None => {
                    library.machines.push(profile);
                    Ok(format!("Saved the project's machine as \"{name}\"."))
                }
            })
        }
    };
    finish(result, &path)
}

fn set_machine_values(
    profile: &mut MachineProfile,
    values: &MachineValues,
) -> Result<(), LibraryError> {
    if let Some(post) = &values.post {
        chipload_post::load_builtin_post(post).map_err(|e| refused(e.to_string()))?;
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

fn list_machines(path: &Path) -> ExitCode {
    let library = match load_machines(path) {
        Ok(library) => library,
        Err(err) => return fail(err),
    };
    println!("Machine profiles: {}", path.display());
    if library.machines.is_empty() {
        println!("  (no machines)");
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
            "  {}{}: post {}, safe Z {} mm, spindle {spindle}, max overcut {} mm",
            m.name,
            mark(is_default),
            m.post,
            m.safe_z,
            m.max_overcut
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
        println!("{line}");
    }
    ExitCode::SUCCESS
}

fn use_machine(path: &Path, name: &str, project: &Path, output: Option<&Path>) -> ExitCode {
    let library = match load_machines(path) {
        Ok(library) => library,
        Err(err) => return fail(err),
    };
    let Some(profile) = library.find(name) else {
        return fail(format!("no machine named \"{name}\" in {}", path.display()));
    };
    let mut job = match load(project) {
        Ok(job) => job,
        Err(code) => return code,
    };
    let changes = profile.apply(&mut job);
    let target = output.unwrap_or(project);
    if let Err(err) = save_job(&job, target) {
        return fail(err);
    }
    println!("Machine \"{}\":", profile.name);
    if changes.is_empty() {
        println!("  no values changed");
    }
    for change in changes {
        println!("  {change}");
    }
    println!("Wrote {}", target.display());
    ExitCode::SUCCESS
}
