// SPDX-License-Identifier: GPL-3.0-or-later
//! The shape library (docs/specs/shape-library.md §4, §5.2): drawn shapes and texts saved
//! under a name in the settings folder's `shapes/`, one file each, in groups that are its
//! folders; DXF and SVG files put there are listed too. Saved from a project's selection,
//! inserted into any project with new ids.

use std::path::{Path, PathBuf};

use chipload_job::edit::{transformed, transformed_point};
use chipload_job::explode::path_to_drawn;
use chipload_job::library::LibraryError;
use chipload_job::shape_library::{
    SHAPE_EXTENSION, SHAPES_DIR, SHAPES_VERSION, ShapeEntry, load_shape_entry, name_problem,
    save_shape_entry,
};
use chipload_job::{ArcSegment, Geometry, Job, LineSegment, PathSegment, Shape, Text};

use crate::PipelineError;
use chipload_i18n::Message;

/// The folder deleted entries are moved to, inside `shapes/`: never listed, so a delete
/// can be undone by hand.
pub const DELETED_DIR: &str = ".deleted";
/// How deep folders are looked into, and how many files are listed at most: a library is
/// a few hundred entries; a folder of thousands of files put there by mistake mustn't
/// hang the window.
const MAX_DEPTH: usize = 6;
const MAX_ITEMS: usize = 5000;

/// The library's folder in the settings folder `settings`.
pub fn shapes_dir(settings: &Path) -> PathBuf {
    settings.join(SHAPES_DIR)
}

/// What an item of the library is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ItemKind {
    /// A `.chipload-shapes` entry.
    Entry,
    /// A DXF file put in the folder, inserted through the importer.
    Dxf,
    /// An SVG file, likewise.
    Svg,
}

/// One item of the library.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Item {
    /// Its group (a folder, `/` between levels), None at the top.
    pub group: Option<String>,
    /// Its name: the file's name without the extension.
    pub name: String,
    pub kind: ItemKind,
    pub path: PathBuf,
}

impl Item {
    /// The name with its group, as the command line takes it: `Signs/House number`.
    pub fn full_name(&self) -> String {
        full_name(self.group.as_deref(), &self.name)
    }
}

fn full_name(group: Option<&str>, name: &str) -> String {
    match group {
        Some(group) => format!("{group}/{name}"),
        None => name.to_string(),
    }
}

/// The library's groups and items, sorted by group (the top first), then name.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Listing {
    /// Every group, the empty ones too.
    pub groups: Vec<String>,
    pub items: Vec<Item>,
}

/// List the library in `dir` (`shapes/`). A missing folder is an empty library.
pub fn list(dir: &Path) -> Result<Listing, PipelineError> {
    let mut listing = Listing::default();
    if dir.is_dir() {
        walk(dir, None, 0, &mut listing)?;
    }
    listing.groups.sort();
    listing.items.sort();
    Ok(listing)
}

fn walk(
    folder: &Path,
    group: Option<&str>,
    depth: usize,
    listing: &mut Listing,
) -> Result<(), PipelineError> {
    let entries = std::fs::read_dir(folder).map_err(|e| io_error(folder, e))?;
    for entry in entries {
        let entry = entry.map_err(|e| io_error(folder, e))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        // Hidden files and folders (the deleted entries among them) are not the library's.
        if name.starts_with('.') || listing.items.len() >= MAX_ITEMS {
            continue;
        }
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        let path = entry.path();
        if kind.is_dir() {
            if depth < MAX_DEPTH {
                let inner = full_name(group, &name);
                listing.groups.push(inner.clone());
                walk(&path, Some(&inner), depth + 1, listing)?;
            }
        } else if let Some((stem, kind)) = item_kind(&name) {
            listing.items.push(Item {
                group: group.map(str::to_string),
                name: stem.to_string(),
                kind,
                path,
            });
        }
    }
    Ok(())
}

/// A file name's stem and kind, None for a file that isn't an item.
fn item_kind(file: &str) -> Option<(&str, ItemKind)> {
    let (stem, extension) = file.rsplit_once('.')?;
    let kind = if extension == SHAPE_EXTENSION {
        ItemKind::Entry
    } else if extension.eq_ignore_ascii_case("dxf") {
        ItemKind::Dxf
    } else if extension.eq_ignore_ascii_case("svg") || extension.eq_ignore_ascii_case("svgz") {
        ItemKind::Svg
    } else {
        return None;
    };
    (!stem.is_empty()).then_some((stem, kind))
}

/// The item named `name` (`Group/Name` or `Name`): an entry before a DXF or SVG of the
/// same name.
pub fn find(dir: &Path, name: &str) -> Result<Item, PipelineError> {
    let (group, name) = split_name(name);
    list(dir)?
        .items
        .into_iter()
        .find(|i| i.group.as_deref() == group && i.name == name)
        .ok_or_else(|| {
            refused(Message::new("pipeline-shapes-no-item").arg("name", full_name(group, name)))
        })
}

/// `Group/Name` into its group and name, at the last `/`.
fn split_name(name: &str) -> (Option<&str>, &str) {
    match name.rsplit_once('/') {
        Some((group, name)) if !group.is_empty() => (Some(group), name),
        _ => (None, name.trim_start_matches('/')),
    }
}

/// The folder of `group` in `dir`, each level's name checked.
fn group_dir(dir: &Path, group: Option<&str>) -> Result<PathBuf, PipelineError> {
    let mut folder = dir.to_path_buf();
    for level in group.into_iter().flat_map(|g| g.split('/')) {
        check_name(level)?;
        folder.push(level);
    }
    Ok(folder)
}

fn check_name(name: &str) -> Result<(), PipelineError> {
    match name_problem(name) {
        Some(e) => Err(library_error(e)),
        None => Ok(()),
    }
}

/// The file an entry `name` (`Group/Name`) is saved in.
pub fn entry_path(dir: &Path, name: &str) -> Result<PathBuf, PipelineError> {
    let (group, name) = split_name(name);
    check_name(name)?;
    Ok(group_dir(dir, group)?.join(format!("{name}.{SHAPE_EXTENSION}")))
}

/// Read an item as an entry: a DXF or SVG through the importer, its shapes as drawn ones
/// (a DXF without units is refused, as on import).
pub fn read(item: &Item) -> Result<ShapeEntry, PipelineError> {
    match item.kind {
        ItemKind::Entry => load_shape_entry(&item.path).map_err(library_error),
        ItemKind::Dxf | ItemKind::Svg => {
            let imported = chipload_dxf::shapes_from_file(&item.path, None, None, 1.0)?;
            let mut shapes = Vec::new();
            for shape in &imported.shapes {
                for geometry in drawn_geometry(shape) {
                    shapes.push(Shape {
                        construction: false,
                        id: shapes.len() as u64 + 1,
                        layer: shape.layer.clone(),
                        name: None,
                        geometry,
                    });
                }
            }
            if shapes.is_empty() {
                return Err(refused(
                    Message::new("pipeline-shapes-file-empty").arg("name", item.full_name()),
                ));
            }
            Ok(ShapeEntry {
                schema_version: SHAPES_VERSION,
                written_by: None,
                shapes,
                texts: Vec::new(),
            })
        }
    }
}

/// A shape's geometry as drawn shapes: drawn ones as they are, an imported path as
/// polylines and circles (`path_to_drawn`).
fn drawn_geometry(shape: &Shape) -> Vec<Geometry> {
    match &shape.geometry {
        Geometry::Path { segments, .. } => path_to_drawn(segments),
        other => vec![other.clone()],
    }
}

/// The entry the selection makes (§4.3): drawn shapes as they are (name and construction
/// kept), imported ones as drawn ones, and every text a selected letter belongs to, or
/// that `texts` names, whole with its letters. Ids from 1.
pub fn entry_from(job: &Job, shapes: &[u64], texts: &[u64]) -> Result<ShapeEntry, PipelineError> {
    let mut text_ids: Vec<u64> = texts.to_vec();
    for shape in job.drawing.shapes.iter().filter(|s| shapes.contains(&s.id)) {
        if let Some(text) = shape.text()
            && !text_ids.contains(&text)
        {
            text_ids.push(text);
        }
    }
    let mut entry = ShapeEntry {
        schema_version: SHAPES_VERSION,
        written_by: None,
        shapes: Vec::new(),
        texts: Vec::new(),
    };
    for shape in &job.drawing.shapes {
        if shape.text().is_some() || !shapes.contains(&shape.id) {
            continue;
        }
        for geometry in drawn_geometry(shape) {
            entry.shapes.push(Shape {
                construction: shape.construction,
                id: entry.shapes.len() as u64 + 1,
                layer: shape.layer.clone(),
                name: shape.name.clone(),
                geometry,
            });
        }
    }
    for text in job
        .drawing
        .texts
        .iter()
        .filter(|t| text_ids.contains(&t.id))
    {
        let id = entry.texts.len() as u64 + 1;
        entry.texts.push(Text { id, ..text.clone() });
        for letter in job
            .drawing
            .shapes
            .iter()
            .filter(|s| s.text() == Some(text.id))
        {
            let mut letter = letter.clone();
            letter.id = entry.shapes.len() as u64 + 1;
            if let Geometry::Path { text, .. } = &mut letter.geometry {
                *text = Some(id);
            }
            entry.shapes.push(letter);
        }
    }
    if entry.shapes.is_empty() {
        return Err(refused(Message::new("pipeline-shapes-nothing-to-save")));
    }
    Ok(entry)
}

/// Save `entry` as `name` (`Group/Name`; the group's folder is made). An entry of that
/// name is replaced only with `replace`, and kept as `.bak`. Returns the file.
pub fn save(
    dir: &Path,
    name: &str,
    entry: &ShapeEntry,
    replace: bool,
) -> Result<PathBuf, PipelineError> {
    let path = entry_path(dir, name)?;
    if path.exists() && !replace {
        return Err(refused(
            Message::new("pipeline-shapes-exists").arg("name", name.to_string()),
        ));
    }
    save_shape_entry(entry, &path).map_err(library_error)?;
    Ok(path)
}

/// How big an entry is inserted.
#[derive(Debug, Clone, PartialEq)]
pub enum InsertSize {
    /// As it was saved.
    AsSaved,
    /// Scaled by this.
    Scale(f64),
    /// Scaled to this width (mm); the height follows.
    Width(f64),
    /// Scaled to this height (mm); the width follows.
    Height(f64),
    /// Scaled so that its box fits round the box of shapes `ids` plus `margin` on every
    /// side, centred on them (§4.4).
    Around { ids: Vec<u64>, margin: f64 },
}

/// What inserting gave: the project, and the new shapes and texts.
#[derive(Debug, Clone, PartialEq)]
pub struct Inserted {
    pub job: Job,
    pub shapes: Vec<u64>,
    pub texts: Vec<u64>,
}

/// Insert `entry` (§4.4): the middle of its box at `at` (for `Around`, the middle of the
/// shapes it goes round; else the stock's middle when None), scaled as `size` says, every
/// shape on `layer` if given. Shapes and texts get new ids; texts stay texts, their letters
/// moved with them, so a text in a font this computer lacks keeps its letters.
pub fn insert(
    job: &Job,
    entry: &ShapeEntry,
    at: Option<(f64, f64)>,
    size: &InsertSize,
    layer: Option<&str>,
) -> Result<Inserted, PipelineError> {
    let ((x0, y0), (x1, y1)) =
        entry_box(entry).ok_or_else(|| refused(Message::new("pipeline-shapes-no-extent")))?;
    let (width, height) = (x1 - x0, y1 - y0);
    let from = [(x0 + x1) / 2.0, (y0 + y1) / 2.0];
    let positive = |v: f64| v.is_finite() && v > 0.0;
    let (scale, around) = match size {
        InsertSize::AsSaved => (1.0, None),
        InsertSize::Scale(s) => (*s, None),
        InsertSize::Width(w) if positive(width) => (w / width, None),
        InsertSize::Height(h) if positive(height) => (h / height, None),
        InsertSize::Width(_) | InsertSize::Height(_) => {
            return Err(refused(Message::new("pipeline-shapes-flat")));
        }
        InsertSize::Around { ids, margin } => {
            let ((a0, b0), (a1, b1)) = crate::shapes::drawn_box(job, ids)
                .ok_or_else(|| refused(Message::new("pipeline-shapes-no-extent")))?;
            let need = |inner: f64, outer: f64| {
                if positive(outer) {
                    (inner + 2.0 * margin) / outer
                } else {
                    0.0
                }
            };
            let scale = need(a1 - a0, width).max(need(b1 - b0, height));
            (scale, Some(((a0 + a1) / 2.0, (b0 + b1) / 2.0)))
        }
    };
    if !positive(scale) {
        return Err(refused(Message::new("pipeline-place-scale").arg(
            "got",
            chipload_i18n::Value::Number(scale, chipload_i18n::Digits::Shortest),
        )));
    }
    let to = around.or(at).unwrap_or_else(|| stock_middle(job));
    let to = [to.0, to.1];
    let mut out = job.clone();
    let mut texts = Vec::new();
    let first_text = job.drawing.texts.iter().map(|t| t.id).max().unwrap_or(0) + 1;
    let mut text_ids = std::collections::BTreeMap::new();
    for (next_text, text) in (first_text..).zip(&entry.texts) {
        let [x, y] = transformed_point([text.x, text.y], from, to, scale, 0.0);
        text_ids.insert(text.id, next_text);
        out.drawing.texts.push(Text {
            id: next_text,
            x,
            y,
            height: text.height * scale,
            spacing: text.spacing * scale,
            layer: layer.map_or_else(|| text.layer.clone(), str::to_string),
            ..text.clone()
        });
        texts.push(next_text);
    }
    let mut shapes = Vec::new();
    for shape in &entry.shapes {
        let geometry = match &shape.geometry {
            Geometry::Path {
                closed,
                segments,
                text,
                ..
            } => Geometry::Path {
                dxf_handle: None,
                dxf_type: None,
                closed: *closed,
                segments: segments
                    .iter()
                    .map(|s| scaled_segment(s, from, to, scale))
                    .collect(),
                image: None,
                dxf: None,
                text: text.and_then(|t| text_ids.get(&t).copied()),
            },
            other => transformed(other, from, to, scale, 0.0)
                .ok_or_else(|| refused(Message::new("pipeline-shapes-no-extent")))?,
        };
        let id = out.drawing.next_shape_id;
        out.drawing.next_shape_id += 1;
        out.drawing.shapes.push(Shape {
            construction: shape.construction,
            id,
            layer: layer.map_or_else(|| shape.layer.clone(), str::to_string),
            name: shape.name.clone(),
            geometry,
        });
        shapes.push(id);
    }
    crate::refuse_problems(&out.drawing.validate())?;
    Ok(Inserted {
        job: out,
        shapes,
        texts,
    })
}

/// A letter's segment moved and scaled about `from` to `to`.
fn scaled_segment(segment: &PathSegment, from: [f64; 2], to: [f64; 2], scale: f64) -> PathSegment {
    let at = |p: [f64; 2]| transformed_point(p, from, to, scale, 0.0);
    match segment {
        PathSegment::Line(l) => PathSegment::Line(LineSegment {
            start: at(l.start),
            end: at(l.end),
        }),
        PathSegment::Arc(a) => PathSegment::Arc(ArcSegment {
            center: at(a.center),
            radius: a.radius * scale,
            ..*a
        }),
    }
}

/// The middle of the stock, where an entry goes without a place.
fn stock_middle(job: &Job) -> (f64, f64) {
    let ((x0, y0), (x1, y1)) = job.stock_bounds();
    ((x0 + x1) / 2.0, (y0 + y1) / 2.0)
}

/// The box round an entry's shapes, mm.
pub fn entry_box(entry: &ShapeEntry) -> Option<((f64, f64), (f64, f64))> {
    entry
        .shapes
        .iter()
        .flat_map(Shape::segments)
        .map(|s| s.to_primitive().bounds())
        .reduce(|(lo, hi), (a, b)| {
            (
                (lo.0.min(a.0), lo.1.min(a.1)),
                (hi.0.max(b.0), hi.1.max(b.1)),
            )
        })
}

/// An entry's shapes as lines for its picture (§5.1): each shape a list of points, arcs
/// in short chords, in the entry's own mm.
pub fn preview(entry: &ShapeEntry) -> Vec<Vec<(f64, f64)>> {
    entry
        .shapes
        .iter()
        .filter(|s| !s.construction)
        .map(|shape| {
            let mut points: Vec<(f64, f64)> = Vec::new();
            for segment in shape.segments() {
                let chords = segment.to_primitive().points();
                let skip = usize::from(!points.is_empty());
                points.extend(chords.into_iter().skip(skip));
            }
            points
        })
        .collect()
}

/// Rename an item (`from` and `to` as `Group/Name`): it may move to another group too. A
/// DXF or SVG keeps its extension. An item of the new name is never replaced.
pub fn rename(dir: &Path, from: &str, to: &str) -> Result<PathBuf, PipelineError> {
    let item = find(dir, from)?;
    let (group, name) = split_name(to);
    check_name(name)?;
    let folder = group_dir(dir, group)?;
    let extension = item
        .path
        .extension()
        .map(|e| e.to_string_lossy().into_owned())
        .unwrap_or_default();
    let target = folder.join(format!("{name}.{extension}"));
    if target == item.path {
        return Ok(target);
    }
    if target.exists() || find(dir, to).is_ok() {
        return Err(refused(
            Message::new("pipeline-shapes-exists").arg("name", to.to_string()),
        ));
    }
    std::fs::create_dir_all(&folder).map_err(|e| io_error(&folder, e))?;
    std::fs::rename(&item.path, &target).map_err(|e| io_error(&item.path, e))?;
    Ok(target)
}

/// Move an item into `group` (None: the top), keeping its name.
pub fn move_to(dir: &Path, name: &str, group: Option<&str>) -> Result<PathBuf, PipelineError> {
    let item = find(dir, name)?;
    rename(dir, name, &full_name(group, &item.name))
}

/// Delete an item: its file goes to `shapes/.deleted/`, where it can be taken back by
/// hand (a file of the same name there before is replaced).
pub fn remove(dir: &Path, name: &str) -> Result<PathBuf, PipelineError> {
    let item = find(dir, name)?;
    let deleted = dir.join(DELETED_DIR);
    std::fs::create_dir_all(&deleted).map_err(|e| io_error(&deleted, e))?;
    let file = item
        .path
        .file_name()
        .map(|f| f.to_string_lossy().into_owned())
        .unwrap_or_default();
    let target = match &item.group {
        Some(group) => deleted.join(format!("{} - {file}", group.replace('/', " - "))),
        None => deleted.join(file),
    };
    std::fs::rename(&item.path, &target).map_err(|e| io_error(&item.path, e))?;
    Ok(target)
}

/// Make an empty group.
pub fn add_group(dir: &Path, group: &str) -> Result<PathBuf, PipelineError> {
    let folder = group_dir(dir, Some(group))?;
    if folder.exists() {
        return Err(refused(
            Message::new("pipeline-shapes-group-exists").arg("name", group.to_string()),
        ));
    }
    std::fs::create_dir_all(&folder).map_err(|e| io_error(&folder, e))?;
    Ok(folder)
}

/// Rename a group (its folder), with what is in it.
pub fn rename_group(dir: &Path, from: &str, to: &str) -> Result<PathBuf, PipelineError> {
    let old = existing_group(dir, from)?;
    let new = group_dir(dir, Some(to))?;
    if new.exists() {
        return Err(refused(
            Message::new("pipeline-shapes-group-exists").arg("name", to.to_string()),
        ));
    }
    if let Some(parent) = new.parent() {
        std::fs::create_dir_all(parent).map_err(|e| io_error(parent, e))?;
    }
    std::fs::rename(&old, &new).map_err(|e| io_error(&old, e))?;
    Ok(new)
}

/// Delete an empty group: one with no items and no groups in it. Anything else in its
/// folder (backups, other files) goes to `shapes/.deleted/` with it.
pub fn remove_group(dir: &Path, group: &str) -> Result<PathBuf, PipelineError> {
    let folder = existing_group(dir, group)?;
    let listing = list(dir)?;
    let inside = |g: &Option<String>| {
        g.as_deref()
            .is_some_and(|g| g == group || g.starts_with(&format!("{group}/")))
    };
    if listing.items.iter().any(|i| inside(&i.group))
        || listing
            .groups
            .iter()
            .any(|g| g.starts_with(&format!("{group}/")))
    {
        return Err(refused(
            Message::new("pipeline-shapes-group-not-empty").arg("name", group.to_string()),
        ));
    }
    let deleted = dir.join(DELETED_DIR);
    std::fs::create_dir_all(&deleted).map_err(|e| io_error(&deleted, e))?;
    let target = deleted.join(group.replace('/', " - "));
    if target.exists() {
        std::fs::remove_dir_all(&target).map_err(|e| io_error(&target, e))?;
    }
    std::fs::rename(&folder, &target).map_err(|e| io_error(&folder, e))?;
    Ok(target)
}

fn existing_group(dir: &Path, group: &str) -> Result<PathBuf, PipelineError> {
    let folder = group_dir(dir, Some(group))?;
    if folder.is_dir() {
        Ok(folder)
    } else {
        Err(refused(
            Message::new("pipeline-shapes-no-group").arg("name", group.to_string()),
        ))
    }
}

fn refused(message: Message) -> PipelineError {
    PipelineError::Job(message)
}

fn library_error(e: LibraryError) -> PipelineError {
    PipelineError::Job(e.message())
}

fn io_error(path: &Path, e: std::io::Error) -> PipelineError {
    refused(
        Message::new("pipeline-shapes-file-error")
            .arg("path", path.display().to_string())
            .arg("error", e.to_string()),
    )
}
