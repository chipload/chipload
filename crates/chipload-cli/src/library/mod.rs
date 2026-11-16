// SPDX-License-Identifier: GPL-3.0-or-later
//! `chipload tool …`, `chipload machine …` and `chipload post …`: the user's tool library,
//! machine profiles and post-processors (docs/specs/tool-library-and-machines.md §7).

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use chipload_job::library::{
    Collection, LibraryError, export, import_into, load_library, read_import, update_library,
    write_export,
};
use chipload_pipeline::library::{CONFIG_DIR_VAR, settings_dir};

use crate::out::Out;

mod machines;
mod posts;
mod tools;

pub use machines::*;
pub use posts::*;
pub use tools::*;

/// The settings folder, or an error naming the variable that sets it.
pub(crate) fn dir() -> Result<PathBuf, String> {
    let dir = settings_dir().ok_or_else(|| {
        format!("can't find your settings folder (no home folder); set {CONFIG_DIR_VAR}")
    })?;
    // An old library is converted once, before any command reads it
    // (docs/specs/cutting-data.md §8).
    chipload_pipeline::library::convert_library(&dir).map_err(|err| err.to_string())?;
    Ok(dir)
}

pub(crate) fn refused(text: String) -> LibraryError {
    LibraryError::Refused(chipload_i18n::Value::Text(text))
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
