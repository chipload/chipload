// SPDX-License-Identifier: GPL-3.0-or-later
//! `chipload machine …`: the user's machine profiles, and the project's machine.

use super::*;
use crate::load;
use crate::out::Out;
use chipload_i18n::Message;
use chipload_job::library::{
    LibraryError, LinkEntry, MachineLibrary, MachineProfile, load_machines, machine_link,
    update_machines,
};
use chipload_job::{Spindle, StockCorner, ToolChange, Travel, WorkZero};
use chipload_pipeline::library::{copy_profile_post, machines_path, unknown_posts};
use chipload_pipeline::posts::{post_exists, posts_dir};
use chipload_pipeline::{DEFAULT_MAX_OVERCUT, DEFAULT_SAFE_Z};
use clap::{Subcommand, ValueEnum};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

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
    /// Copy a profile's machine values into a project, keeping its safe Z, work zero and
    /// max overcut; lists what changed.
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
    #[arg(value_parser = crate::lengths::length, long, allow_hyphen_values = true)]
    safe_z: Option<f64>,
    #[arg(long)]
    spindle: Option<SpindleArg>,
    #[arg(value_parser = crate::lengths::feed, long)]
    max_feed: Option<f64>,
    /// Travel "X,Y,Z".
    #[arg(long)]
    travel: Option<String>,
    /// drawing-origin, stock-center, stock-bottom-left, stock-bottom-right,
    /// stock-top-left, stock-top-right, or a point "X,Y".
    #[arg(long, allow_hyphen_values = true)]
    work_zero: Option<String>,
    /// How far below the stock a cut may go (into the spoilboard).
    #[arg(value_parser = crate::lengths::length, long)]
    max_overcut: Option<f64>,
    /// With several tools: one G-code file per tool, or one file with M6 tool changes
    /// that your sender handles.
    #[arg(long)]
    tool_change: Option<ToolChangeArg>,
    /// The units the G-code is written in: mm (G21) or inch (G20). Everything else stays
    /// in mm.
    #[arg(long)]
    gcode_units: Option<crate::UnitsArg>,
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
                    .iter()
                    .map(Message::english)
                    .collect()
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
                gcode_units: chipload_job::Units::Mm,
                safe_z: DEFAULT_SAFE_Z,
                spindle: Spindle::Cw,
                max_feed: None,
                travel: None,
                // As a new project (docs/specs/work-zero-marker.md §7).
                work_zero: WorkZero::Stock {
                    corner: chipload_job::StockCorner::BottomLeft,
                },
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
            if let Err(err) = crate::save_project(out, &job, &project) {
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
            out.line(format!("Machine: {}", link.text(LinkEntry::Profile)));
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
    if let Some(units) = values.gcode_units {
        profile.gcode_units = units.value();
    }
    Ok(())
}

fn travel_arg(text: &str) -> Result<Travel, String> {
    let numbers: Option<Vec<f64>> = text
        .split(',')
        .map(|n| crate::lengths::length(n).ok())
        .collect();
    match numbers.as_deref() {
        Some(&[x, y, z]) => Ok(Travel { x, y, z }),
        _ => Err(format!(
            "expected travel as X,Y,Z (mm, or with a unit), got {text:?}"
        )),
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
            "  {}{}: post {}, G-code in {}, safe Z {} mm, spindle {spindle}, max overcut {} mm, \
             tool changes {}",
            m.name,
            mark(is_default),
            m.post,
            m.gcode_units.name(),
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
    // The machine values only: the project keeps its safe Z, work zero and max overcut.
    let mut changes = profile.update_machine_values(&mut job);
    changes.extend(copy_profile_post(&mut job, Some(posts)));
    let target = output.unwrap_or(project);
    if let Err(err) = crate::save_project(out, &job, target) {
        return out.fail(err);
    }
    out.line(format!("Machine \"{}\":", profile.name));
    if changes.is_empty() {
        out.line("  no values changed");
    }
    for change in changes {
        out.line(format!("  {}", change.english()));
    }
    out.wrote(target, "");
    ExitCode::SUCCESS
}

// -- post-processors (docs/specs/user-posts.md §6)
