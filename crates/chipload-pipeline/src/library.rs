// SPDX-License-Identifier: GPL-3.0-or-later
//! Where the user's tool library and machine profiles live, and new projects started from
//! them (docs/specs/tool-library-and-machines.md §2, §5). Planning and G-code never read
//! the library: a project carries copies of what it uses.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use chipload_i18n::Message;
use chipload_job::Job;
use chipload_job::library::{
    LibraryError, MACHINES_FILE, MachineLibrary, SETTINGS_FILE, TOOLS_FILE, ToolLibrary,
    load_library, load_machines, load_tools, update_library,
};
use chipload_job::materials::{MATERIALS_FILE, MaterialLibrary, move_material_values};

/// Overrides the settings folder (tests, a portable install, a shared workshop folder).
pub const CONFIG_DIR_VAR: &str = "CHIPLOAD_CONFIG_DIR";

/// The user's settings folder, from the environment; None if it doesn't say (no home).
pub fn settings_dir() -> Option<PathBuf> {
    settings_dir_from(std::env::consts::OS, |key| std::env::var_os(key))
}

/// `settings_dir` for a given OS name (as `std::env::consts::OS`) and environment.
pub fn settings_dir_from(os: &str, var: impl Fn(&str) -> Option<OsString>) -> Option<PathBuf> {
    let absolute = |key: &str| {
        var(key)
            .map(PathBuf::from)
            .filter(|path| path.is_absolute())
    };
    if let Some(dir) = absolute(CONFIG_DIR_VAR) {
        return Some(dir);
    }
    let base = match os {
        "windows" => absolute("APPDATA"),
        "macos" => absolute("HOME").map(|home| home.join("Library").join("Application Support")),
        _ => absolute("XDG_CONFIG_HOME").or_else(|| absolute("HOME").map(|h| h.join(".config"))),
    };
    base.map(|dir| dir.join("chipload"))
}

pub fn tools_path(dir: &Path) -> PathBuf {
    dir.join(TOOLS_FILE)
}

pub fn machines_path(dir: &Path) -> PathBuf {
    dir.join(MACHINES_FILE)
}

pub fn materials_path(dir: &Path) -> PathBuf {
    dir.join(MATERIALS_FILE)
}

/// The user's styles in the settings folder `dir` (docs/specs/styles.md §4).
pub fn styles_path(dir: &Path) -> PathBuf {
    dir.join(chipload_job::styles::STYLES_FILE)
}

/// The folder of fonts for texts beside the installed ones (docs/specs/system-fonts.md §2).
pub fn fonts_path(dir: &Path) -> PathBuf {
    dir.join("fonts")
}

/// A material that was removed: its rows go from every tool in settings folder `dir`
/// (docs/specs/cutting-data.md §2). Returns the names of the tools that had one; the file is
/// only written when one had.
pub fn forget_material_in_tools(dir: &Path, uid: &str) -> Result<Vec<String>, LibraryError> {
    let path = tools_path(dir);
    let library = load_tools(&path)?;
    if library.tools.iter().all(|t| t.row(Some(uid)).is_none()) {
        return Ok(Vec::new());
    }
    update_library(&path, |library: &mut ToolLibrary| {
        Ok(library.forget_material(uid))
    })
}

/// Convert the library in settings folder `dir` once, when its `materials.json` is older
/// than version 3 (docs/specs/cutting-data.md §8): the materials' values move into the
/// tools as rows ([`move_material_values`], pure), then both files are written in their
/// current versions, each keeping the one before as `.bak`. A line saying so, then one
/// message per tool given rows and per value dropped; nothing for a library that is
/// current, missing or can't be read (loading it says why).
pub fn convert_library(dir: &Path) -> Result<Vec<Message>, LibraryError> {
    let Ok(text) = std::fs::read_to_string(materials_path(dir)) else {
        return Ok(Vec::new());
    };
    let Ok(old) = serde_json::from_str::<serde_json::Value>(&text) else {
        return Ok(Vec::new());
    };
    let version = old
        .get("schema_version")
        .and_then(serde_json::Value::as_i64);
    if !version.is_some_and(|v| v < 3) {
        return Ok(Vec::new());
    }
    // The tools first: a material file still old after a failure moves its values again,
    // and rows already there are kept.
    let moved = update_library(&tools_path(dir), |tools: &mut ToolLibrary| {
        Ok(move_material_values(tools, &old))
    })?;
    update_library(&materials_path(dir), |_: &mut MaterialLibrary| Ok(()))?;
    let mut messages = vec![Message::new("pipeline-library-converted")];
    messages.extend(moved);
    Ok(messages)
}

pub fn settings_path(dir: &Path) -> PathBuf {
    dir.join(SETTINGS_FILE)
}

/// The library as read for starting values. A file that can't be read is left empty and
/// named in `problems`: a broken library never stops anything (§6).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Library {
    pub tools: ToolLibrary,
    pub machines: MachineLibrary,
    /// The user's materials (docs/specs/materials.md).
    pub materials: MaterialLibrary,
    /// The user's styles (docs/specs/styles.md); one that can't be read is left out and
    /// named in `problems`.
    pub styles: chipload_job::styles::StyleLibrary,
    /// Why a file couldn't be read, as messages.
    pub problems: Vec<Message>,
    /// What converting an old library did (docs/specs/cutting-data.md §8); empty but the
    /// once it happens.
    pub converted: Vec<Message>,
    /// Where the user's post-processors are (docs/specs/user-posts.md); None: no settings
    /// folder.
    pub posts: Option<PathBuf>,
    /// The user's post-processors as read, by name, with why one can't be used.
    pub user_posts: Vec<(String, Result<chipload_post::UserPost, Message>)>,
}

impl Library {
    pub fn load(dir: &Path) -> Library {
        let mut problems = Vec::new();
        let converted = convert_library(dir).unwrap_or_else(|err| {
            problems.push(Message::new("pipeline-library-defaults").arg("error", err.message()));
            Vec::new()
        });
        let defaults = |err: LibraryError| {
            Message::new("pipeline-library-defaults").arg("error", err.message())
        };
        let tools = load_tools(&tools_path(dir)).unwrap_or_else(|err| {
            problems.push(defaults(err));
            ToolLibrary::default()
        });
        let machines = load_machines(&machines_path(dir)).unwrap_or_else(|err| {
            problems.push(defaults(err));
            MachineLibrary::default()
        });
        let materials = load_library(&materials_path(dir)).unwrap_or_else(|err| {
            problems.push(defaults(err));
            MaterialLibrary::default()
        });
        let styles = match chipload_job::styles::load_styles(&styles_path(dir)) {
            Ok(loaded) => {
                problems.extend(loaded.refused);
                loaded.library
            }
            Err(err) => {
                problems.push(defaults(err));
                chipload_job::styles::StyleLibrary::default()
            }
        };
        let posts = crate::posts::posts_dir(dir);
        let user_posts = chipload_post::list_user_posts(&posts)
            .into_iter()
            .map(|(name, read)| (name, read.map_err(|e| e.message())))
            .collect();
        Library {
            tools,
            machines,
            materials,
            styles,
            problems,
            converted,
            posts: Some(posts),
            user_posts,
        }
    }
}

/// Give a new project the library's starting values: the named machine profile (else the
/// default one) and the default tool. Returns what was used, one line each; a project
/// with no library keeps its built-in values and gets no lines.
pub fn apply_library(
    job: &mut Job,
    library: &Library,
    machine: Option<&str>,
) -> Result<Vec<Message>, Message> {
    let mut notes = Vec::new();
    let profile = match machine {
        Some(name) => Some((
            library
                .machines
                .find(name)
                .ok_or_else(|| Message::new("pipeline-library-no-profile").arg("name", name))?,
            "profile",
        )),
        None => library
            .machines
            .default_entry()
            .map(|profile| (profile, "default")),
    };
    if let Some((profile, how)) = profile {
        profile.apply(job);
        notes.push(
            Message::new("pipeline-library-machine")
                .arg("name", &profile.name)
                .arg("how", how),
        );
        notes.extend(copy_profile_post(job, library.posts.as_deref()));
    }
    if let Some(entry) = library.tools.default_entry() {
        let tool = entry.tool(entry.id.max(1));
        notes.push(
            Message::new("pipeline-library-tool")
                .arg("tool", tool.id)
                .arg("name", &entry.name),
        );
        job.tools = vec![tool];
    }
    // The default material, last: the operations an import made take its values for
    // their tools (docs/specs/job-settings-layout.md §3).
    if let Some(material) = library.materials.default_entry() {
        let (with, _) = crate::materials::use_material(job, &library.tools, Some(material));
        *job = with;
        notes.push(Message::new("pipeline-library-material").arg("name", &material.name));
    }
    Ok(notes)
}

/// After a machine profile is applied: copy its post in if it is a user post. A post that
/// can't be read is named in the returned note; the project keeps the name and can't make
/// G-code until the post is there (or another is chosen).
pub fn copy_profile_post(job: &mut Job, posts: Option<&Path>) -> Option<Message> {
    let name = job.machine.post.clone();
    if chipload_post::builtin_posts().contains(&name.as_str()) {
        return None;
    }
    crate::posts::choose_post(job, &name, posts)
        .err()
        .map(|err| {
            Message::new("pipeline-library-post-unusable")
                .arg("name", name)
                .arg("error", err.message())
        })
}

/// Notes for imported machine profiles whose post-processor this Chipload doesn't have,
/// built in or in `posts` (docs/specs/library-import-export.md §4): imported anyway,
/// refused only when used.
pub fn unknown_posts(file: &MachineLibrary, posts: Option<&Path>) -> Vec<Message> {
    file.machines
        .iter()
        .filter(|m| !crate::posts::post_exists(&m.post, posts))
        .map(|m| {
            Message::new("pipeline-library-unknown-post")
                .arg("name", &m.name)
                .arg("post", &m.post)
        })
        .collect()
}

/// The speed project tool `tool` has from its setup's material: its row's speed for the
/// project's material, when the tool runs at it (docs/specs/tool-updates.md §4).
pub fn material_speed(
    job: &Job,
    tools: &ToolLibrary,
    materials: &MaterialLibrary,
    tool: &chipload_job::Tool,
) -> Option<f64> {
    let material = crate::materials::project_material(job, materials)?;
    let rpm = crate::materials::tool_values(job, tools, Some(material), tool.id)?.rpm?;
    (tool.rpm == Some(rpm)).then_some(rpm)
}

/// How project tool `tool` stands to its library tool, a speed from the material aside
/// (docs/specs/tool-updates.md §4).
pub fn project_tool_link(
    job: &Job,
    tools: &ToolLibrary,
    materials: &MaterialLibrary,
    tool: &chipload_job::Tool,
) -> chipload_job::library::Link {
    let mut compared = tool.clone();
    if material_speed(job, tools, materials, tool).is_some()
        && let Some(entry) = tools.find_uid(tool.library_uid.as_deref().unwrap_or(""))
    {
        compared.rpm = entry.rpm;
    }
    chipload_job::library::tool_link(tools, &compared)
}

/// The project with its tools that differ from their library tools (all, or tool `only`)
/// given the library tools' values, keeping a speed from the material
/// (docs/specs/tool-updates.md §3, §4). Per tool updated, its id and what changed.
pub fn update_from_library(
    job: &Job,
    tools: &ToolLibrary,
    materials: &MaterialLibrary,
    only: Option<i64>,
) -> (Job, Vec<(i64, Vec<Message>)>) {
    let mut out = job.clone();
    let mut updated = Vec::new();
    for tool in &job.tools {
        if only.is_some_and(|id| id != tool.id) {
            continue;
        }
        let chipload_job::library::Link::Differs(_, changes) =
            project_tool_link(job, tools, materials, tool)
        else {
            continue;
        };
        let Some(entry) = tools.find_uid(tool.library_uid.as_deref().unwrap_or("")) else {
            continue;
        };
        let mut new = entry.tool(tool.id);
        if let Some(rpm) = material_speed(job, tools, materials, tool) {
            new.rpm = Some(rpm);
        }
        if let Some(slot) = out.tools.iter_mut().find(|t| t.id == tool.id) {
            *slot = new;
        }
        updated.push((tool.id, changes));
    }
    out.fit_helixes();
    (out, updated)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<OsString> {
        let pairs: Vec<(String, String)> = pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        move |key| {
            pairs
                .iter()
                .find(|(k, _)| k == key)
                .map(|(_, v)| OsString::from(v))
        }
    }

    /// An absolute path on the machine running the test: on Windows a path needs a drive
    /// to be absolute (and `/` separates there too), elsewhere it is as written.
    fn abs(path: &str) -> String {
        if cfg!(windows) {
            format!("C:{path}")
        } else {
            path.to_string()
        }
    }

    #[test]
    fn the_settings_folder_per_os() {
        let ann = abs("/home/ann");
        let home = [("HOME", ann.as_str())];
        assert_eq!(
            settings_dir_from("linux", env(&home)),
            Some(PathBuf::from(abs("/home/ann/.config/chipload")))
        );
        assert_eq!(
            settings_dir_from(
                "linux",
                env(&[
                    ("HOME", &abs("/home/ann")),
                    ("XDG_CONFIG_HOME", &abs("/cfg"))
                ])
            ),
            Some(PathBuf::from(abs("/cfg/chipload")))
        );
        // A relative XDG_CONFIG_HOME is ignored, as the XDG spec says.
        assert_eq!(
            settings_dir_from(
                "linux",
                env(&[("HOME", &abs("/home/ann")), ("XDG_CONFIG_HOME", "cfg")])
            ),
            Some(PathBuf::from(abs("/home/ann/.config/chipload")))
        );
        assert_eq!(
            settings_dir_from("macos", env(&home)),
            Some(PathBuf::from(abs(
                "/home/ann/Library/Application Support/chipload"
            )))
        );
        assert_eq!(settings_dir_from("linux", env(&[])), None);
        assert_eq!(
            settings_dir_from(
                "linux",
                env(&[
                    ("HOME", &abs("/home/ann")),
                    (CONFIG_DIR_VAR, &abs("/shared/cnc"))
                ])
            ),
            Some(PathBuf::from(abs("/shared/cnc")))
        );
    }

    #[cfg(windows)]
    #[test]
    fn the_settings_folder_on_windows() {
        assert_eq!(
            settings_dir_from(
                "windows",
                env(&[("APPDATA", r"C:\Users\ann\AppData\Roaming")])
            ),
            Some(PathBuf::from(r"C:\Users\ann\AppData\Roaming\chipload"))
        );
    }
}
