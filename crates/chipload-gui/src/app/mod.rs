// SPDX-License-Identifier: GPL-3.0-or-later
//! Main window: import a drawing into a project, edit its operations (from selected geometry,
//! in any order), generate on F5, save the project, export checked G-code.
//! See docs/specs/job-editor.md and docs/specs/project-file.md. Thin by design: geometry and
//! G-code decisions are calls into the pipeline; this file only edits a `Job` (a project)
//! and shows results.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use chipload_core::Point;
use chipload_core::chain::Contour;
use chipload_dxf::{ImportError, shapes_from_dxf};
use chipload_job::{
    Direction, Drawing, Geometry, Job, Operation, Selector, Side, Spindle, StockCorner,
    StockOrigin, Tabs, ToolType, Travel, Units, WorkZero, ZZero, load_job, save_job,
};
use chipload_pipeline::report::{format_contours, format_import_report, format_material_left};
use chipload_pipeline::shapes::{add_shape, delete_shape, set_shape};
use chipload_pipeline::{
    PipelineError, RunResult, load_drawing, new_project, run_job, update_drawing, write_atomically,
};
use chipload_toolpath::left::LeftKind;
use chipload_toolpath::select::select_contours;
use eframe::egui::{
    self, Align, CentralPanel, ComboBox, DragValue, Grid, Key, KeyboardShortcut, Layout, MenuBar,
    Modifiers, Panel, RichText, ScrollArea, Sense, TextEdit, ViewportCommand,
};

use crate::draw::{Sketch, Step, Tool, grid_step, snap};
use crate::editor::{CanvasSelection, EditJob, OpKind, geometry_summary, shapes_of};
use crate::shape_form::{ShapeAction, ShapeDraft, imported_lines, shape_form};
use crate::view::{ContourShape, Highlight, LeftShape, Scene, ToolpathShape, View};

mod canvas;
mod dialogs;
mod drawing;
mod forms;
mod menu;
mod panels;
#[cfg(test)]
mod tests;

use forms::{job_form, op_form};

const STATUS_SECONDS: u64 = 5;
const ERROR_STATUS_SECONDS: u64 = 10;
/// Extension of project files (docs/specs/project-file.md, D1).
pub const PROJECT_EXTENSION: &str = "chipload";

const OPEN: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::O);
const IMPORT: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::I);
const SAVE_AS: KeyboardShortcut =
    KeyboardShortcut::new(Modifiers::COMMAND.plus(Modifiers::SHIFT), Key::S);
const SAVE: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::S);
const EXPORT: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::E);
const GENERATE: KeyboardShortcut = KeyboardShortcut::new(Modifiers::NONE, Key::F5);
const REDO: KeyboardShortcut =
    KeyboardShortcut::new(Modifiers::COMMAND.plus(Modifiers::SHIFT), Key::Z);
const UNDO: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::Z);
const MOVE_UP: KeyboardShortcut = KeyboardShortcut::new(Modifiers::ALT, Key::ArrowUp);
const MOVE_DOWN: KeyboardShortcut = KeyboardShortcut::new(Modifiers::ALT, Key::ArrowDown);
const FIT: KeyboardShortcut = KeyboardShortcut::new(Modifiers::NONE, Key::F);
const COPY_SELECTOR: KeyboardShortcut =
    KeyboardShortcut::new(Modifiers::COMMAND.plus(Modifiers::SHIFT), Key::C);
const QUIT: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::Q);

/// What the properties panel shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Job,
    Op(usize),
}

/// Something that would throw away unsaved changes, waiting for confirmation.
#[derive(Debug, Clone, PartialEq)]
enum Pending {
    Close,
    OpenDialog,
    ImportDialog,
    Open(PathBuf),
}

/// What a typed-in path is for (for systems without a native file dialog).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PathPurpose {
    Open,
    Save,
    Export,
    Update,
}

/// A small modal asking for something before an action can continue.
#[derive(Debug, Clone, PartialEq)]
enum Prompt {
    /// The DXF being imported declares no units.
    Units(PathBuf),
    Path(PathPurpose, String),
    Discard(Pending),
}

pub struct ChiploadApp {
    /// The project being edited.
    pub edit: Option<EditJob>,
    /// Where it was opened from or last saved to.
    pub job_path: Option<PathBuf>,
    /// The project as last opened or saved; differs from `edit` when there are unsaved changes.
    saved: Option<EditJob>,
    /// The project as last recorded for undo.
    committed: Option<EditJob>,
    undo: Vec<Option<EditJob>>,
    redo: Vec<Option<EditJob>>,
    pub focus: Focus,

    /// The project's contours (from its embedded drawing), and the drawing they came from.
    pub contours: Vec<Contour>,
    loaded_drawing: Option<Drawing>,

    // The last generation.
    pub result: Option<RunResult>,
    /// Operations (indices) the last generation, validation or update complained about.
    pub error_ops: BTreeSet<usize>,

    // The canvas.
    pub scene: Scene,
    view: View,
    fit_pending: bool,
    pub selection: CanvasSelection,
    context_pick: Option<usize>,
    /// View > Show material left (docs/specs/material-left.md).
    pub show_material_left: bool,
    /// The shape being drawn with the mouse, if any (docs/specs/drawing-tools.md).
    pub sketch: Option<Sketch>,
    /// The selected drawn shape's values as being edited.
    shape_draft: Option<ShapeDraft>,

    pub report: Vec<String>,
    status: Option<(String, Instant)>,
    cursor: Option<Point>,
    prompt: Option<Prompt>,
    title: String,
    allow_close: bool,
    about_open: bool,
}

impl Default for ChiploadApp {
    fn default() -> Self {
        ChiploadApp {
            edit: None,
            job_path: None,
            saved: None,
            committed: None,
            undo: Vec::new(),
            redo: Vec::new(),
            focus: Focus::Job,
            contours: Vec::new(),
            loaded_drawing: None,
            result: None,
            error_ops: BTreeSet::new(),
            scene: Scene::default(),
            view: View::default(),
            fit_pending: false,
            selection: CanvasSelection::Nothing,
            context_pick: None,
            show_material_left: true,
            sketch: None,
            shape_draft: None,
            report: vec!["Import a DXF (Ctrl+I) to start a project, or open one (Ctrl+O).".into()],
            status: None,
            cursor: None,
            prompt: None,
            title: String::new(),
            allow_close: false,
            about_open: false,
        }
    }
}

fn file_name(path: &Path) -> String {
    path.file_name().map_or_else(
        || path.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    )
}

/// The `schema_version` a file was written with, before migration (for the save note).
fn raw_schema_version(path: &Path) -> Option<i64> {
    let text = std::fs::read_to_string(path).ok()?;
    let value: serde_json::Value = serde_json::from_str(&text).ok()?;
    value.get("schema_version")?.as_i64()
}

/// "Operation 3 (...)": the zero-based index of the operation a message is about.
fn operation_index(message: &str) -> Option<usize> {
    let rest = &message[message.find("Operation ")? + "Operation ".len()..];
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    digits.parse::<usize>().ok()?.checked_sub(1)
}

fn selector_text(selector: &Selector) -> String {
    if let Some(layer) = &selector.layer {
        return format!("{{\"layer\": \"{layer}\"}}");
    }
    if let Some(shapes) = selector.shapes.as_ref().filter(|s| !s.is_empty()) {
        let ids: Vec<String> = shapes.iter().map(u64::to_string).collect();
        return format!("{{\"shapes\": [{}]}}", ids.join(", "));
    }
    "{}".into()
}

fn is_dxf(path: &Path) -> bool {
    path.extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("dxf"))
}

impl ChiploadApp {
    // -- state

    /// The project has changes that aren't saved.
    pub fn dirty(&self) -> bool {
        self.edit.is_some() && self.edit != self.saved
    }

    /// Record the current project as one undo step if it changed since the last one. Called
    /// once the user has finished an edit (no drag or text entry in progress).
    pub fn commit(&mut self) {
        if self.edit != self.committed {
            self.undo.push(self.committed.take());
            self.redo.clear();
            self.committed.clone_from(&self.edit);
            self.toolpath_out_of_date();
        }
    }

    pub fn undo(&mut self) {
        self.commit();
        if let Some(previous) = self.undo.pop() {
            self.redo.push(self.edit.take());
            self.edit = previous;
            self.committed.clone_from(&self.edit);
            self.after_history_step();
        }
    }

    pub fn redo(&mut self) {
        if let Some(next) = self.redo.pop() {
            self.undo.push(self.edit.take());
            self.edit = next;
            self.committed.clone_from(&self.edit);
            self.after_history_step();
        }
    }

    fn after_history_step(&mut self) {
        self.shape_draft = None;
        self.sketch = None;
        let len = self.edit.as_ref().map_or(0, |e| e.ops.len());
        if let Focus::Op(i) = self.focus
            && i >= len
        {
            self.focus = if len == 0 {
                Focus::Job
            } else {
                Focus::Op(len - 1)
            };
        }
        self.toolpath_out_of_date();
    }

    /// Start editing `job` as a fresh document (no undo history).
    fn start_editing(&mut self, job: EditJob, saved: bool, path: Option<PathBuf>) {
        self.saved = saved.then(|| job.clone());
        self.committed = Some(job.clone());
        self.edit = Some(job);
        self.job_path = path;
        self.undo.clear();
        self.redo.clear();
        self.focus = Focus::Job;
        self.error_ops.clear();
        self.selection = CanvasSelection::Nothing;
        self.invalidate();
        self.loaded_drawing = None;
        self.refresh_drawing();
        self.fit_pending = true;
    }

    /// Recompute contours if the project's drawing changed (open, import, update, undo).
    /// Returns the import report lines when it did.
    fn refresh_drawing(&mut self) -> Option<Vec<String>> {
        let job = self.edit.as_ref()?.settings.clone();
        if self.loaded_drawing.as_ref() == Some(&job.drawing) {
            return None;
        }
        self.loaded_drawing = Some(job.drawing.clone());
        self.scene.stock = Some(job.stock.xy_bounds());
        match load_drawing(&job) {
            Ok(imported) => {
                self.contours = imported.contours;
                self.scene.contours = self.contours.iter().map(ContourShape::new).collect();
                let mut lines = format_import_report(&imported.report);
                lines.extend(format_contours(&self.contours));
                Some(lines)
            }
            Err(_) => {
                self.contours.clear();
                self.scene.contours.clear();
                None
            }
        }
    }

    // -- operations (also called directly by tests)

    /// Open a project, an older job, or a DXF (imported as a new project).
    pub fn open(&mut self, path: &Path) -> bool {
        if is_dxf(path) {
            self.import_dxf(path, None)
        } else {
            self.open_project(path)
        }
    }

    pub fn open_project(&mut self, path: &Path) -> bool {
        let job = match load_job(path) {
            Ok(job) => job,
            Err(err) => {
                self.error(vec![
                    format!("Cannot open {}:", file_name(path)),
                    err.to_string(),
                ]);
                return false;
            }
        };
        let version = raw_schema_version(path);
        self.start_editing(EditJob::from_job(&job), true, Some(path.to_path_buf()));
        let mut lines = vec![format!("Project: {}", file_name(path))];
        lines.extend(self.drawing_lines());
        if let Some(v) = version
            && v < chipload_job::CURRENT_VERSION
        {
            lines.insert(
                1,
                format!(
                    "This file is schema version {v}; saving writes version {}.",
                    chipload_job::CURRENT_VERSION
                ),
            );
        }
        self.report = lines;
        true
    }

    /// Import a DXF as a new, unsaved project. A DXF without units asks for them.
    pub fn import_dxf(&mut self, path: &Path, units: Option<Units>) -> bool {
        self.prompt = None;
        let imported = match shapes_from_dxf(path, units) {
            Ok(imported) => imported,
            Err(ImportError::UnitsMissing) => {
                self.prompt = Some(Prompt::Units(path.to_path_buf()));
                self.report = vec![format!(
                    "{} does not say which units it uses; choose mm or inch.",
                    file_name(path)
                )];
                return false;
            }
            Err(err) => {
                self.error(vec![
                    format!("Cannot import {}:", file_name(path)),
                    err.to_string(),
                ]);
                return false;
            }
        };
        let shapes = imported.shapes.len();
        self.start_editing(EditJob::from_job(&new_project(imported)), false, None);
        let mut lines = vec![
            format!("New project from {} ({shapes} shapes).", file_name(path)),
            "Select contours, then New profile or New pocket. Check stock, machine and tool \
             under Job settings."
                .into(),
            String::new(),
        ];
        lines.extend(self.drawing_lines());
        self.report = lines;
        true
    }

    fn drawing_lines(&self) -> Vec<String> {
        let Some(edit) = &self.edit else {
            return Vec::new();
        };
        match load_drawing(&edit.settings) {
            Ok(imported) => {
                let mut lines = format_import_report(&imported.report);
                lines.extend(format_contours(&imported.contours));
                lines
            }
            Err(err) => vec![err.to_string()],
        }
    }

    /// Replace the imported drawing with a new version of the DXF (§4 of the project spec).
    pub fn update_from(&mut self, dxf: &Path) -> bool {
        self.commit();
        let Some(edit) = &self.edit else {
            return false;
        };
        let job = match edit.to_job() {
            Ok(job) => job,
            Err(problems) => {
                self.error(problems);
                return false;
            }
        };
        match update_drawing(&job, dxf, None) {
            Ok((updated, report)) => {
                let mut next = EditJob::from_job(&updated);
                // Keep "side not chosen" marks; the update doesn't touch operations.
                for (entry, old) in next.ops.iter_mut().zip(&edit.ops) {
                    entry.side_chosen = old.side_chosen;
                }
                self.edit = Some(next);
                self.commit();
                self.refresh_drawing();
                self.error_ops = report.lost.iter().copied().collect();
                self.report = report.lines;
                if !report.lost.is_empty() {
                    self.set_status(
                        "Some operations lost their geometry: choose it again".into(),
                        ERROR_STATUS_SECONDS,
                    );
                }
                true
            }
            Err(err) => {
                self.error(vec![
                    "Could not update the drawing:".into(),
                    err.to_string(),
                ]);
                false
            }
        }
    }

    /// Add an operation for the current canvas selection and select it.
    pub fn add_operation(&mut self, kind: OpKind) -> Option<usize> {
        let edit = self.edit.as_mut()?;
        let index = edit.add(kind, &self.selection, &self.contours);
        self.focus = Focus::Op(index);
        self.commit();
        Some(index)
    }

    /// Give the selected operation the current canvas selection as its geometry.
    pub fn use_selection(&mut self) {
        if let (Focus::Op(i), Some(edit)) = (self.focus, self.edit.as_mut()) {
            edit.set_geometry(i, &self.selection, &self.contours);
            self.error_ops.remove(&i);
            self.commit();
        }
    }

    /// Move the operation at `from` to position `to` in the cut order.
    pub fn move_operation(&mut self, from: usize, to: usize) {
        if let Some(edit) = self.edit.as_mut()
            && from < edit.ops.len()
            && to < edit.ops.len()
        {
            edit.move_op(from, to);
            self.focus = Focus::Op(to);
            self.commit();
        }
    }

    pub fn duplicate_operation(&mut self) {
        if let (Focus::Op(i), Some(edit)) = (self.focus, self.edit.as_mut())
            && let Some(copy) = edit.duplicate(i)
        {
            self.focus = Focus::Op(copy);
            self.commit();
        }
    }

    pub fn delete_operation(&mut self) {
        if let (Focus::Op(i), Some(edit)) = (self.focus, self.edit.as_mut()) {
            edit.remove(i);
            let len = edit.ops.len();
            self.focus = if len == 0 {
                Focus::Job
            } else {
                Focus::Op(i.min(len - 1))
            };
            self.commit();
        }
    }

    pub fn generate(&mut self) -> bool {
        self.commit();
        let Some(edit) = &self.edit else {
            self.error(vec!["Import a DXF or open a project first.".into()]);
            return false;
        };
        let job = match edit.to_job() {
            Ok(job) => job,
            Err(problems) => {
                self.error_ops = problems.iter().filter_map(|p| operation_index(p)).collect();
                let mut lines = vec!["The project is not complete:".to_string()];
                lines.extend(problems.into_iter().map(|p| format!("  {p}")));
                self.error(lines);
                return false;
            }
        };
        match run_job(&job) {
            Ok(result) => {
                self.error_ops.clear();
                self.scene.toolpath =
                    Some(ToolpathShape::new(&result.program, false, &result.op_spans));
                self.scene.tabs.clone_from(&result.tabs);
                self.scene.left = result.material_left.iter().map(LeftShape::new).collect();
                // Missed pieces on top.
                self.scene.left.sort_by_key(|l| l.kind == LeftKind::Missed);
                let mut lines = vec![format!(
                    "Generated {} lines for {}. Check passed.",
                    result.gcode.matches('\n').count(),
                    result.post.name
                )];
                lines.extend(result.notes.iter().map(|n| format!("warning: {n}")));
                lines.extend(
                    result
                        .check_warnings
                        .iter()
                        .map(|w| format!("check warning: {}", w.message)),
                );
                lines.extend(format_material_left(&job, &result.material_left));
                lines.push(format!("Work zero: {}", job.work_zero_text()));
                lines.push(String::new());
                lines.extend(format_import_report(&result.import_report));
                lines.extend(format_contours(&result.contours));
                self.report = lines;
                self.result = Some(result);
                true
            }
            Err(PipelineError::CheckFailed { errors, program }) => {
                self.scene.toolpath = Some(ToolpathShape::new(&program, true, &[]));
                self.scene.tabs.clear();
                self.scene.left.clear();
                let mut lines = vec!["G-code check FAILED. Nothing can be exported.".to_string()];
                lines.extend(errors);
                self.error(lines);
                false
            }
            Err(err) => {
                self.scene.toolpath = None;
                self.scene.tabs.clear();
                self.scene.left.clear();
                let message = err.to_string();
                self.error_ops = operation_index(&message).into_iter().collect();
                self.error(vec!["Generation failed:".into(), message]);
                false
            }
        }
    }

    /// Write the checked G-code of the last generation.
    pub fn export_gcode(&mut self, path: &Path) -> bool {
        let Some(result) = &self.result else {
            self.error(vec!["Generate (F5) before exporting G-code.".into()]);
            return false;
        };
        match write_atomically(path, &result.gcode) {
            Ok(()) => {
                self.set_status(format!("Exported {}", path.display()), STATUS_SECONDS);
                true
            }
            Err(err) => {
                self.error(vec![
                    format!("Could not export {}:", path.display()),
                    err.to_string(),
                ]);
                false
            }
        }
    }

    /// Save the project (current schema, every field explicit).
    pub fn save_project(&mut self, path: &Path) -> bool {
        self.commit();
        let Some(edit) = &self.edit else {
            return false;
        };
        let job = match edit.to_job() {
            Ok(job) => job,
            Err(problems) => {
                let mut lines = vec!["The project can't be saved yet:".to_string()];
                lines.extend(problems.into_iter().map(|p| format!("  {p}")));
                self.error(lines);
                return false;
            }
        };
        if let Err(err) = save_job(&job, path) {
            self.error(vec![
                format!("Could not save {}:", path.display()),
                err.to_string(),
            ]);
            return false;
        }
        let saved = EditJob::from_job(&job);
        self.edit = Some(saved.clone());
        self.committed = Some(saved.clone());
        self.saved = Some(saved);
        self.job_path = Some(path.to_path_buf());
        self.set_status(format!("Saved {}", path.display()), STATUS_SECONDS);
        true
    }

    /// Click on the canvas: select one contour, Ctrl toggles, empty space clears.
    /// A click on the canvas: `picked` is the contour hit (if any); `toggle` (Ctrl) adds or
    /// removes it instead of replacing the selection. Selections are kept as shape ids.
    pub fn click(&mut self, picked: Option<usize>, toggle: bool) {
        let mut shapes: BTreeSet<u64> = match (&self.selection, toggle) {
            (CanvasSelection::Shapes(shapes), true) => shapes.clone(),
            (CanvasSelection::Layer(_), true) => {
                let ids = self.selection.ids(&self.contours);
                match CanvasSelection::of_contours(&ids, &self.contours) {
                    CanvasSelection::Shapes(shapes) => shapes,
                    _ => BTreeSet::new(),
                }
            }
            _ => BTreeSet::new(),
        };
        let hit: BTreeSet<u64> = picked
            .and_then(|id| self.contours.iter().find(|c| c.id == id))
            .map(shapes_of)
            .unwrap_or_default();
        if toggle && !hit.is_empty() && hit.is_subset(&shapes) {
            shapes.retain(|s| !hit.contains(s));
        } else {
            shapes.extend(hit);
        }
        self.selection = if shapes.is_empty() {
            CanvasSelection::Nothing
        } else {
            CanvasSelection::Shapes(shapes)
        };
    }

    pub fn select_layer(&mut self, layer: &str) {
        self.selection = CanvasSelection::Layer(layer.to_string());
    }

    /// A job selector for the canvas selection, e.g. {"shapes": [3, 4]}.
    pub fn selector_json(&self) -> Option<String> {
        let selector = self.selection.selector(&self.contours);
        let empty = selector.layer.is_none() && selector.shapes.as_ref().is_none_or(Vec::is_empty);
        (!empty).then(|| selector_text(&selector))
    }

    pub fn selection_lines(&self) -> Vec<String> {
        let ids = self.selection.ids(&self.contours);
        if ids.is_empty() {
            return vec![
                "Click a contour to select it; Ctrl+click adds or removes, Esc clears.".into(),
                "Right-click a contour to select its whole layer.".into(),
            ];
        }
        let what = match &self.selection {
            CanvasSelection::Layer(layer) => format!("layer {layer}"),
            _ => "contours".into(),
        };
        let list: Vec<String> = ids.iter().map(|i| format!("#{i}")).collect();
        let mut lines = vec![format!(
            "{} selected ({what}): {}",
            ids.len(),
            list.join(", ")
        )];
        let open: Vec<String> = self
            .contours
            .iter()
            .filter(|c| ids.contains(&c.id) && !c.closed)
            .map(|c| format!("#{}", c.id))
            .collect();
        if !open.is_empty() {
            lines.push(format!("Open (not cut): {}", open.join(", ")));
        }
        lines
    }

    // -- internals

    fn focused_contours(&self) -> BTreeSet<usize> {
        let (Focus::Op(i), Some(edit)) = (self.focus, &self.edit) else {
            return BTreeSet::new();
        };
        let Some(entry) = edit.ops.get(i) else {
            return BTreeSet::new();
        };
        select_contours(&self.contours, entry.op.select(), "Operation")
            .map(|s| s.contours.iter().map(|c| c.id).collect())
            .unwrap_or_default()
    }

    /// Inputs changed: the old toolpath and G-code no longer apply.
    fn invalidate(&mut self) {
        self.result = None;
        self.scene.toolpath = None;
        self.scene.tabs.clear();
        self.scene.left.clear();
    }

    fn toolpath_out_of_date(&mut self) {
        if self.result.is_some() || self.scene.toolpath.is_some() {
            self.invalidate();
            self.set_status(
                "Project changed: press F5 to generate again".into(),
                STATUS_SECONDS,
            );
        }
        self.scene.stock = self.edit.as_ref().map(|e| e.settings.stock.xy_bounds());
        self.refresh_drawing();
    }

    fn error(&mut self, lines: Vec<String>) {
        if let Some(first) = lines.first() {
            self.set_status(first.clone(), ERROR_STATUS_SECONDS);
        }
        self.report = lines;
    }

    fn set_status(&mut self, text: String, seconds: u64) {
        self.status = Some((text, Instant::now() + Duration::from_secs(seconds)));
    }

    fn can_generate(&self) -> bool {
        self.edit.is_some()
    }

    /// Proceed now, or ask first if unsaved changes would be lost.
    fn guard(&mut self, action: Pending) -> bool {
        if self.dirty() {
            self.prompt = Some(Prompt::Discard(action));
            false
        } else {
            true
        }
    }

    fn run_pending(&mut self, action: Pending, ctx: &egui::Context) {
        match action {
            Pending::Close => {
                self.allow_close = true;
                ctx.send_viewport_cmd(ViewportCommand::Close);
            }
            Pending::OpenDialog => self.open_dialog(),
            Pending::ImportDialog => self.import_dialog(),
            Pending::Open(path) => {
                self.open(&path);
            }
        }
    }

    fn default_project_path(&self) -> PathBuf {
        if let Some(path) = &self.job_path {
            return path.with_extension(PROJECT_EXTENSION);
        }
        self.edit
            .as_ref()
            .and_then(|e| e.settings.drawing.source.as_ref())
            .map(|s| Path::new(&s.file).with_extension(PROJECT_EXTENSION))
            .unwrap_or_else(|| PathBuf::from(format!("project.{PROJECT_EXTENSION}")))
    }

    fn default_export_path(&self) -> PathBuf {
        let ext = self.result.as_ref().map_or("nc".to_string(), |r| {
            r.post.file_extension.trim_start_matches('.').to_string()
        });
        self.default_project_path().with_extension(ext)
    }
}

impl eframe::App for ChiploadApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        if ctx.input(|i| i.viewport().close_requested()) && self.dirty() && !self.allow_close {
            ctx.send_viewport_cmd(ViewportCommand::CancelClose);
            self.prompt = Some(Prompt::Discard(Pending::Close));
        }
        self.shortcuts(&ctx);
        Panel::top("menu").show(ui, |ui| self.menu(ui));
        Panel::bottom("status").show(ui, |ui| self.status_bar(ui));
        Panel::left("operations")
            .resizable(true)
            .default_size(340.0)
            .show(ui, |ui| {
                ScrollArea::vertical()
                    .id_salt("operations-scroll")
                    .show(ui, |ui| self.operations_panel(ui));
            });
        Panel::right("side")
            .resizable(true)
            .default_size(440.0)
            .show(ui, |ui| {
                let height = ui.available_height() * 0.55;
                ScrollArea::vertical()
                    .id_salt("properties-scroll")
                    .max_height(height)
                    .show(ui, |ui| self.properties_panel(ui));
                ui.separator();
                self.report_panel(ui);
            });
        CentralPanel::no_frame().show(ui, |ui| self.canvas(ui));
        self.prompts(&ctx);
        self.about(&ctx);
        // One undo step per finished edit: not while dragging a value or typing.
        let busy = ctx.egui_is_using_pointer() || ctx.egui_wants_keyboard_input();
        if !busy {
            self.commit();
        }
        self.update_title(&ctx);
    }
}
