// SPDX-License-Identifier: GPL-3.0-or-later
//! Where the user's tool library and machine profiles live, and new projects started from
//! them (docs/specs/tool-library-and-machines.md §2, §5). Planning and G-code never read
//! the library: a project carries copies of what it uses.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use chipload_job::Job;
use chipload_job::library::{
    MACHINES_FILE, MachineLibrary, SETTINGS_FILE, TOOLS_FILE, ToolLibrary, load_machines,
    load_tools,
};

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

pub fn settings_path(dir: &Path) -> PathBuf {
    dir.join(SETTINGS_FILE)
}

/// The library as read for starting values. A file that can't be read is left empty and
/// named in `problems`: a broken library never stops anything (§6).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Library {
    pub tools: ToolLibrary,
    pub machines: MachineLibrary,
    pub problems: Vec<String>,
    /// Where the user's post-processors are (docs/specs/user-posts.md); None: no settings
    /// folder.
    pub posts: Option<PathBuf>,
    /// The user's post-processors as read, by name, with why one can't be used.
    pub user_posts: Vec<(String, Result<chipload_post::UserPost, String>)>,
}

impl Library {
    pub fn load(dir: &Path) -> Library {
        let mut problems = Vec::new();
        let tools = load_tools(&tools_path(dir)).unwrap_or_else(|err| {
            problems.push(format!("{err} (using built-in defaults)"));
            ToolLibrary::default()
        });
        let machines = load_machines(&machines_path(dir)).unwrap_or_else(|err| {
            problems.push(format!("{err} (using built-in defaults)"));
            MachineLibrary::default()
        });
        let posts = crate::posts::posts_dir(dir);
        let user_posts = chipload_post::list_user_posts(&posts)
            .into_iter()
            .map(|(name, read)| (name, read.map_err(|e| e.to_string())))
            .collect();
        Library {
            tools,
            machines,
            problems,
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
) -> Result<Vec<String>, String> {
    let mut notes = Vec::new();
    let profile = match machine {
        Some(name) => Some((
            library
                .machines
                .find(name)
                .ok_or_else(|| format!("no machine profile named \"{name}\""))?,
            "profile",
        )),
        None => library
            .machines
            .default_entry()
            .map(|profile| (profile, "default profile")),
    };
    if let Some((profile, how)) = profile {
        profile.apply(job);
        notes.push(format!("machine: {} ({how})", profile.name));
        notes.extend(copy_profile_post(job, library.posts.as_deref()));
    }
    if let Some(entry) = library.tools.default_entry() {
        let tool = entry.tool(entry.id.max(1));
        notes.push(format!("tool {}: {} (default tool)", tool.id, entry.name));
        job.tools = vec![tool];
    }
    Ok(notes)
}

/// After a machine profile is applied: copy its post in if it is a user post. A post that
/// can't be read is named in the returned note; the project keeps the name and can't make
/// G-code until the post is there (or another is chosen).
pub fn copy_profile_post(job: &mut Job, posts: Option<&Path>) -> Option<String> {
    let name = job.machine.post.clone();
    if chipload_post::builtin_posts().contains(&name.as_str()) {
        return None;
    }
    crate::posts::choose_post(job, &name, posts)
        .err()
        .map(|err| format!("post-processor \"{name}\": {err}"))
}

/// Notes for imported machine profiles whose post-processor this Chipload doesn't have,
/// built in or in `posts` (docs/specs/library-import-export.md §4): imported anyway,
/// refused only when used.
pub fn unknown_posts(file: &MachineLibrary, posts: Option<&Path>) -> Vec<String> {
    file.machines
        .iter()
        .filter(|m| !crate::posts::post_exists(&m.post, posts))
        .map(|m| {
            format!(
                "Machine \"{}\": this Chipload has no post-processor \"{}\"; projects using \
                 it can't make G-code until it has",
                m.name, m.post
            )
        })
        .collect()
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
