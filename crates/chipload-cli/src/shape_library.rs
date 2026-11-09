// SPDX-License-Identifier: GPL-3.0-or-later
//! `chipload shape save`, `shape insert` and `shape library …`: the shape library in the
//! settings folder's `shapes/` (docs/specs/shape-library.md §4, §5.2, §7).

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use chipload_pipeline::shape_library::{
    InsertSize, ItemKind, add_group, entry_from, find, insert, list, move_to, read, remove,
    remove_group, rename, rename_group, save, shapes_dir,
};
use clap::{Args, Subcommand};
use serde_json::json;

use crate::load;
use crate::out::Out;

/// The library's commands among `shape`'s.
#[derive(Subcommand)]
pub enum LibraryCommand {
    /// Save shapes and texts to the shape library under a name.
    Save(Box<SaveShapes>),
    /// Insert an entry of the shape library into a project; prints the new ids.
    Insert(Box<InsertShapes>),
    /// The shape library: list (the default), rename, move, remove, groups.
    Library {
        #[command(subcommand)]
        action: Option<LibraryAction>,
    },
}

pub fn run(out: &mut Out, command: LibraryCommand) -> ExitCode {
    match command {
        LibraryCommand::Save(args) => save_shapes(out, *args),
        LibraryCommand::Insert(args) => insert_shapes(out, *args),
        LibraryCommand::Library { action } => library(out, action),
    }
}

/// `shape save`'s options.
#[derive(Args)]
pub struct SaveShapes {
    /// The project (.chipload).
    project: PathBuf,
    /// The shape ids (as `inspect` lists them, without the S); a letter saves its whole
    /// text.
    ids: Vec<u64>,
    /// Texts to save, with their letters (ids as `text list` shows them).
    #[arg(long, num_args = 1.., value_delimiter = ',')]
    texts: Vec<u64>,
    /// The entry's name, with its group: "Signs/House number".
    #[arg(long)]
    name: String,
    /// Replace an entry of that name (it is kept as .bak).
    #[arg(long)]
    replace: bool,
}

/// `shape insert`'s options.
#[derive(Args)]
pub struct InsertShapes {
    /// The project (.chipload).
    project: PathBuf,
    /// The entry's name, with its group: "Signs/House number".
    name: String,
    /// Where the middle of its box goes, X,Y in mm (default: the stock's middle).
    #[arg(long, value_parser = parse_at, allow_hyphen_values = true)]
    at: Option<(f64, f64)>,
    /// Scaled to fit round these shapes, --margin from their box (ids without the S).
    #[arg(long, num_args = 1.., value_delimiter = ',')]
    around: Vec<u64>,
    /// With --around: from the shapes' box to the entry's, mm.
    #[arg(value_parser = crate::lengths::length, long, default_value = "10")]
    margin: f64,
    /// Scaled to this width, mm; the height follows.
    #[arg(value_parser = crate::lengths::length, long)]
    width: Option<f64>,
    /// Scaled to this height, mm; the width follows.
    #[arg(value_parser = crate::lengths::length, long)]
    height: Option<f64>,
    /// Scaled by this.
    #[arg(long)]
    scale: Option<f64>,
    /// Every shape and text on this layer (default: the layers they were saved on).
    #[arg(long)]
    layer: Option<String>,
    /// Where to write the result; default: the project itself.
    #[arg(short, long)]
    output: Option<PathBuf>,
}

#[derive(Subcommand)]
pub enum LibraryAction {
    /// The entries and groups (the default).
    List,
    /// Where the library's folder is.
    Path,
    /// Rename an entry; a name with another group moves it there.
    Rename { from: String, to: String },
    /// Move an entry into a group ("" for none).
    Move { name: String, group: String },
    /// Delete an entry: its file goes to the folder's .deleted.
    Remove { name: String },
    /// Groups of entries: folders in the library's folder.
    Group {
        #[command(subcommand)]
        action: GroupAction,
    },
}

#[derive(Subcommand)]
pub enum GroupAction {
    /// Make an empty group.
    Add { name: String },
    /// Rename a group, with its entries.
    Rename { from: String, to: String },
    /// Delete an empty group.
    Remove { name: String },
}

fn parse_at(text: &str) -> Result<(f64, f64), String> {
    crate::lengths::point(text).map(|[x, y]| (x, y))
}

fn folder() -> Result<PathBuf, String> {
    crate::library::dir().map(|dir| shapes_dir(&dir))
}

fn save_shapes(out: &mut Out, args: SaveShapes) -> ExitCode {
    let dir = match folder() {
        Ok(dir) => dir,
        Err(err) => return out.fail(err),
    };
    let job = match load(out, &args.project) {
        Ok(job) => job,
        Err(code) => return code,
    };
    let entry = match entry_from(&job, &args.ids, &args.texts) {
        Ok(entry) => entry,
        Err(err) => return out.fail(err),
    };
    match save(&dir, &args.name, &entry, args.replace) {
        Ok(path) => {
            out.line(format!(
                "Saved \"{}\": {}, {}.",
                args.name,
                count(entry.shapes.len(), "shape"),
                count(entry.texts.len(), "text")
            ));
            out.wrote(&path, "");
            out.set("name", json!(args.name));
            out.set("shapes", json!(entry.shapes.len()));
            out.set("texts", json!(entry.texts.len()));
            ExitCode::SUCCESS
        }
        Err(err) => out.fail(err),
    }
}

fn insert_shapes(out: &mut Out, args: InsertShapes) -> ExitCode {
    let dir = match folder() {
        Ok(dir) => dir,
        Err(err) => return out.fail(err),
    };
    let job = match load(out, &args.project) {
        Ok(job) => job,
        Err(code) => return code,
    };
    let size = match (&args.around[..], args.width, args.height, args.scale) {
        ([], None, None, None) => InsertSize::AsSaved,
        ([], Some(w), None, None) => InsertSize::Width(w),
        ([], None, Some(h), None) => InsertSize::Height(h),
        ([], None, None, Some(s)) => InsertSize::Scale(s),
        (ids, None, None, None) if args.at.is_none() => InsertSize::Around {
            ids: ids.to_vec(),
            margin: args.margin,
        },
        _ => {
            return out.fail("give one of --around (without --at), --width, --height or --scale");
        }
    };
    let entry = match find(&dir, &args.name).and_then(|item| read(&item)) {
        Ok(entry) => entry,
        Err(err) => return out.fail(err),
    };
    let done = match insert(
        &job,
        &entry,
        &args.name,
        args.at,
        &size,
        args.layer.as_deref(),
    ) {
        Ok(done) => done,
        Err(err) => return out.fail(err),
    };
    let target = args.output.as_deref().unwrap_or(&args.project);
    if let Err(err) = crate::save_project(out, &done.job, target) {
        return out.fail(err);
    }
    let names: Vec<String> = done.shapes.iter().map(|id| format!("S{id}")).collect();
    out.line(format!(
        "Inserted \"{}\": shapes {}{}.",
        args.name,
        names.join(", "),
        if done.texts.is_empty() {
            String::new()
        } else {
            let texts: Vec<String> = done.texts.iter().map(|id| format!("T{id}")).collect();
            format!(", texts {}", texts.join(", "))
        }
    ));
    out.wrote(target, "");
    out.set("ids", json!(done.shapes));
    out.set("texts", json!(done.texts));
    ExitCode::SUCCESS
}

fn library(out: &mut Out, action: Option<LibraryAction>) -> ExitCode {
    let dir = match folder() {
        Ok(dir) => dir,
        Err(err) => return out.fail(err),
    };
    let done = match action.unwrap_or(LibraryAction::List) {
        LibraryAction::List => return show(out, &dir),
        LibraryAction::Path => {
            out.line(dir.display().to_string());
            out.set("path", json!(dir.display().to_string()));
            return ExitCode::SUCCESS;
        }
        LibraryAction::Rename { from, to } => {
            rename(&dir, &from, &to).map(|_| format!("Renamed \"{from}\" to \"{to}\"."))
        }
        LibraryAction::Move { name, group } => {
            let group = (!group.is_empty()).then_some(group.as_str());
            move_to(&dir, &name, group).map(|_| format!("Moved \"{name}\"."))
        }
        LibraryAction::Remove { name } => {
            remove(&dir, &name).map(|to| format!("Deleted \"{name}\" (kept in {}).", to.display()))
        }
        LibraryAction::Group { action } => match action {
            GroupAction::Add { name } => {
                add_group(&dir, &name).map(|_| format!("Added the group \"{name}\"."))
            }
            GroupAction::Rename { from, to } => rename_group(&dir, &from, &to)
                .map(|_| format!("Renamed the group \"{from}\" to \"{to}\".")),
            GroupAction::Remove { name } => {
                remove_group(&dir, &name).map(|_| format!("Deleted the group \"{name}\"."))
            }
        },
    };
    match done {
        Ok(line) => {
            out.line(line);
            ExitCode::SUCCESS
        }
        Err(err) => out.fail(err),
    }
}

fn show(out: &mut Out, dir: &Path) -> ExitCode {
    let listing = match list(dir) {
        Ok(listing) => listing,
        Err(err) => return out.fail(err),
    };
    out.line(format!("Shape library: {}", dir.display()));
    if listing.items.is_empty() && listing.groups.is_empty() {
        out.line("  (empty)");
    }
    let mut items = Vec::new();
    let mut group = None;
    for item in &listing.items {
        if item.group != group {
            group = item.group.clone();
            out.line(format!("{}/", group.as_deref().unwrap_or_default()));
        }
        let (kind, problem) = match item.kind {
            ItemKind::Entry => ("entry", read(item).err()),
            ItemKind::Dxf => ("dxf", None),
            ItemKind::Svg => ("svg", None),
        };
        let indent = if item.group.is_some() { "  " } else { "" };
        match &problem {
            Some(err) => out.line(format!("{indent}{}  (can't be read: {err})", item.name)),
            None if kind == "entry" => out.line(format!("{indent}{}", item.name)),
            None => out.line(format!("{indent}{}  ({kind})", item.name)),
        }
        items.push(json!({
            "name": item.full_name(),
            "group": item.group,
            "kind": kind,
            "error": problem.map(|e| e.to_string()),
        }));
    }
    let empty: Vec<&String> = listing
        .groups
        .iter()
        .filter(|g| !listing.items.iter().any(|i| i.group.as_ref() == Some(g)))
        .collect();
    for group in &empty {
        out.line(format!("{group}/  (empty)"));
    }
    out.set("path", json!(dir.display().to_string()));
    out.set("groups", json!(listing.groups));
    out.set("items", json!(items));
    ExitCode::SUCCESS
}

/// "1 shape", "3 shapes".
fn count(n: usize, what: &str) -> String {
    if n == 1 {
        format!("1 {what}")
    } else {
        format!("{n} {what}s")
    }
}
