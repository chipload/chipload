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
use chipload_job::edit::{Handle, handles, try_drag};
use chipload_job::library::Theme;
use chipload_job::{
    Direction, Drawing, Geometry, Job, Operation, PocketStrategy, Selector, Side, Spindle,
    StockCorner, StockOrigin, Tabs, ToolType, Travel, Units, WorkZero, ZZero, load_job, save_job,
};
use chipload_pipeline::report::format_material_left;
use chipload_pipeline::shapes::{add_shape, delete_shape, set_shape};
use chipload_pipeline::{
    PipelineError, RunResult, empty_project, load_drawing, new_project, run_job, update_drawing,
    write_gcode_files,
};
use chipload_toolpath::left::LeftKind;
use chipload_toolpath::select::select_contours;
use eframe::egui::{
    self, Align, CentralPanel, ComboBox, DragValue, Grid, Key, KeyboardShortcut, Layout, MenuBar,
    Modifiers, Panel, RichText, ScrollArea, Sense, TextEdit, ViewportCommand,
};

use crate::draw::{Sketch, SnapPoint, Step, Tool, grid_step, snap_cursor, snap_points};
use crate::editor::{CanvasSelection, EditJob, OpKind, geometry_summary, shapes_of};
use crate::shape_form::{ShapeAction, ShapeDraft, imported_lines, shape_form};
use crate::view::{ContourShape, Highlight, LeftShape, Scene, StartMarker, ToolpathShape, View};

mod canvas;
mod dialogs;
mod drawing;
mod forms;
mod menu;
mod panels;
mod report;
#[cfg(test)]
mod tests;
mod user_library;

use forms::{job_form, op_form};
use report::{DrawingFacts, Report, Tone};
use user_library::LibraryWindows;
pub use user_library::{LibraryKind, SaveTarget};

const STATUS_SECONDS: u64 = 5;
const ERROR_STATUS_SECONDS: u64 = 10;
/// Extension of project files (docs/specs/project-file.md, D1).
pub const PROJECT_EXTENSION: &str = "chipload";

const NEW: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::N);
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
const TOP_VIEW: KeyboardShortcut = KeyboardShortcut::new(Modifiers::NONE, Key::T);
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
    New,
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
    /// The DXF being imported (or, with `true`, updating the drawing) declares no units.
    Units(PathBuf, bool),
    Path(PathPurpose, String),
    Discard(Pending),
    /// A name for the project's machine or tool in the user's library.
    LibraryName(SaveTarget, String),
}

/// A handle drag in progress: the shape as it was when the drag began, which every frame
/// of the drag starts from (chipload_job::edit::drag).
#[derive(Debug, Clone)]
struct HandleDrag {
    handle: Handle,
    layer: String,
    start: Geometry,
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
    /// The canvas is in 3D: left-drag orbits (docs/specs/3d-view.md).
    pub three_d: bool,
    /// The shape being drawn with the mouse, if any (docs/specs/drawing-tools.md).
    pub sketch: Option<Sketch>,
    /// The selected drawn shape's values as being edited.
    shape_draft: Option<ShapeDraft>,
    /// Points of the drawing the cursor snaps to (docs/specs/drawing-follow-ups.md).
    snap_points: Vec<SnapPoint>,
    /// Where the cursor snapped this frame, while drawing or dragging a handle.
    snapped: Option<(Point, Option<SnapPoint>)>,
    /// The handle being dragged, if any.
    handle_drag: Option<HandleDrag>,
    /// The start marker being dragged (index in `scene.start_markers`).
    start_drag: Option<usize>,
    /// The start marker right-clicked, for its context menu.
    context_start: Option<usize>,

    pub report: Report,
    /// The report panel is open (it opens by itself on warnings and errors).
    pub report_open: bool,
    /// The current drawing's facts, for the report's Drawing section.
    pub drawing_facts: Option<DrawingFacts>,
    /// The canvas legend is unfolded.
    pub legend_open: bool,
    /// The project changed since the last generation, which was cleared.
    pub toolpath_stale: bool,
    /// View > Theme, kept in the app settings.
    pub theme: Theme,
    /// The theme last handed to egui.
    applied_theme: Option<Theme>,
    status: Option<(String, Instant)>,
    cursor: Option<Point>,
    prompt: Option<Prompt>,
    title: String,
    allow_close: bool,
    about_open: bool,

    /// The user's settings folder (docs/specs/tool-library-and-machines.md); None in tests
    /// and when there is no home folder.
    settings_dir: Option<PathBuf>,
    /// The tool library and machine profiles as last read.
    library: chipload_pipeline::library::Library,
    library_windows: LibraryWindows,
    /// File > Open recent (docs/specs/open-recent.md), most recent first.
    pub recent: Vec<String>,
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
            three_d: false,
            sketch: None,
            shape_draft: None,
            snap_points: Vec::new(),
            snapped: None,
            handle_drag: None,
            start_drag: None,
            context_start: None,
            report: Report::info([
                "Import a DXF (Ctrl+I) to start a project, or open one (Ctrl+O).".into(),
            ]),
            report_open: false,
            drawing_facts: None,
            legend_open: true,
            toolpath_stale: false,
            theme: Theme::System,
            applied_theme: None,
            status: None,
            cursor: None,
            prompt: None,
            title: String::new(),
            allow_close: false,
            about_open: false,
            settings_dir: None,
            library: chipload_pipeline::library::Library::default(),
            library_windows: LibraryWindows::default(),
            recent: Vec::new(),
        }
    }
}

fn theme_preference(theme: Theme) -> egui::ThemePreference {
    match theme {
        Theme::System => egui::ThemePreference::System,
        Theme::Light => egui::ThemePreference::Light,
        Theme::Dark => egui::ThemePreference::Dark,
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

    /// Recompute contours and the drawing's facts if the project's drawing changed (open,
    /// import, update, undo).
    fn refresh_drawing(&mut self) {
        let Some(job) = self.edit.as_ref().map(|e| e.settings.clone()) else {
            self.drawing_facts = None;
            return;
        };
        if self.loaded_drawing.as_ref() == Some(&job.drawing) {
            return;
        }
        self.loaded_drawing = Some(job.drawing.clone());
        self.scene.stock = Some(job.stock.xy_bounds());
        match load_drawing(&job) {
            Ok(imported) => {
                self.contours = imported.contours;
                self.scene.contours = self.contours.iter().map(ContourShape::new).collect();
                self.snap_points = snap_points(&self.contours);
                self.drawing_facts = Some(DrawingFacts::new(&imported.report, &self.contours));
            }
            Err(err) => {
                self.contours.clear();
                self.scene.contours.clear();
                self.snap_points.clear();
                self.drawing_facts = Some(DrawingFacts::failed(err.to_string()));
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
        self.record_recent(path);
        let mut lines = vec![format!("Project: {}", file_name(path))];
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
        self.set_report(Report::info(lines));
        true
    }

    /// Start a new, unsaved project with no drawing (docs/specs/new-project.md).
    pub fn new_project(&mut self) {
        self.prompt = None;
        self.reload_library();
        let mut job = empty_project();
        let notes = chipload_pipeline::library::apply_library(&mut job, &self.library, None)
            .unwrap_or_default();
        self.start_editing(EditJob::from_job(&job), false, None);
        let mut lines = vec!["New project.".to_string()];
        lines.extend(notes.iter().map(|n| format!("Started with {n}.")));
        lines.extend(self.library.problems.iter().cloned());
        lines.push(
            "Draw shapes from the Draw menu, or bring in a drawing with File > Import DXF \
             into project. Check stock, machine and tool under Job settings."
                .into(),
        );
        self.set_report(Report::info(lines));
    }

    /// Import a DXF as a new, unsaved project. A DXF without units asks for them.
    pub fn import_dxf(&mut self, path: &Path, units: Option<Units>) -> bool {
        self.prompt = None;
        let imported = match shapes_from_dxf(path, units) {
            Ok(imported) => imported,
            Err(ImportError::UnitsMissing) => {
                self.prompt = Some(Prompt::Units(path.to_path_buf(), false));
                self.set_report(Report::info([format!(
                    "{} does not say which units it uses; choose mm or inch.",
                    file_name(path)
                )]));
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
        // The library gives starting values (the default machine and tool).
        self.reload_library();
        let mut job = new_project(imported);
        let notes = chipload_pipeline::library::apply_library(&mut job, &self.library, None)
            .unwrap_or_default();
        self.start_editing(EditJob::from_job(&job), false, None);
        let mut lines = vec![format!(
            "New project from {} ({shapes} shapes).",
            file_name(path)
        )];
        lines.extend(notes.iter().map(|n| format!("Started with {n}.")));
        lines.extend(self.library.problems.iter().cloned());
        lines.push(
            "Select contours, then New profile or New pocket. Check stock, machine and tool \
             under Job settings."
                .into(),
        );
        self.set_report(Report::info(lines));
        true
    }

    /// Replace the imported drawing with a new version of the DXF (§4 of the project spec),
    /// or bring in the first DXF of a project started empty. A DXF without units, when the
    /// project has none recorded either, asks for them.
    pub fn update_from(&mut self, dxf: &Path, units: Option<Units>) -> bool {
        self.prompt = None;
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
        match update_drawing(&job, dxf, units) {
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
                self.set_report(Report::info(report.lines));
                if !report.lost.is_empty() {
                    self.set_status(
                        "Some operations lost their geometry: choose it again".into(),
                        ERROR_STATUS_SECONDS,
                    );
                }
                true
            }
            Err(PipelineError::Import(ImportError::UnitsMissing)) => {
                self.prompt = Some(Prompt::Units(dxf.to_path_buf(), true));
                self.set_report(Report::info([format!(
                    "{} does not say which units it uses; choose mm or inch.",
                    file_name(dxf)
                )]));
                false
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
        let tool = self.edit.as_ref()?.next_tool_id();
        let cutting = self.tool_cutting(tool);
        let edit = self.edit.as_mut()?;
        let index = edit.add_with(kind, &self.selection, &self.contours, cutting.as_ref());
        self.focus = Focus::Op(index);
        self.commit();
        Some(index)
    }

    /// Set the copies' pitch so `gap` mm are left between their cuts (docs/specs/
    /// copies-grid.md §6). It plans the part once, so the project must be complete.
    /// Move the imported drawing (docs/specs/drawing-placement.md), as one undo step.
    pub fn place(&mut self, how: chipload_pipeline::placement::Move) -> bool {
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
        match chipload_pipeline::placement::place_drawing(&job, how) {
            Ok((placed, report)) => {
                let mut next = EditJob::from_job(&placed);
                // Keep "side not chosen" marks; placing only moves start points.
                for (entry, old) in next.ops.iter_mut().zip(&edit.ops) {
                    entry.side_chosen = old.side_chosen;
                }
                self.edit = Some(next);
                self.commit();
                self.refresh_drawing();
                self.set_report(Report::info(report.lines));
                true
            }
            Err(err) => {
                self.error(vec!["Could not move the drawing:".into(), err.to_string()]);
                false
            }
        }
    }

    pub fn set_copies_gap(&mut self, gap: f64) {
        self.commit();
        let Some(edit) = &self.edit else {
            return;
        };
        // The part alone: the grid being set may not be valid yet (a pitch of 0).
        let mut part = edit.clone();
        part.settings.copies = None;
        let job = match part.to_job() {
            Ok(job) => job,
            Err(problems) => {
                self.error(problems);
                return;
            }
        };
        let current = edit.settings.copies.map_or((0.0, 0.0), |c| (c.dx, c.dy));
        match chipload_pipeline::copies::pitch_for_gap(&job, gap, current) {
            Ok((dx, dy)) => {
                if let Some(copies) = self.edit.as_mut().and_then(|e| e.settings.copies.as_mut()) {
                    copies.dx = dx;
                    copies.dy = dy;
                }
                self.commit();
                self.set_status(
                    format!("Pitch {dx:.3} × {dy:.3} mm ({gap:.3} mm gap)"),
                    STATUS_SECONDS,
                );
            }
            Err(err) => self.error(vec![
                "Could not set the pitch from a gap: the part has to be planned first.".into(),
                err.to_string(),
            ]),
        }
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
        self.toolpath_stale = false;
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
                self.scene.start_markers = result
                    .ring_starts
                    .iter()
                    .map(|s| StartMarker::new(s.operation, s.ring.clone(), s.stored))
                    .collect();
                // Missed pieces on top.
                self.scene.left.sort_by_key(|l| l.kind == LeftKind::Missed);
                let mut report = Report::default();
                let lines: usize = result
                    .files
                    .iter()
                    .map(|f| f.gcode.matches('\n').count())
                    .sum();
                let files = match result.files.len() {
                    1 => String::new(),
                    n => format!(" in {n} files, one per tool group"),
                };
                report.push(
                    Tone::Good,
                    format!(
                        "Generated {lines} lines{files} for {}. Check passed.",
                        result.post.name
                    ),
                );
                for note in &result.notes {
                    report.push(Tone::Warning, note.clone());
                }
                for warning in &result.check_warnings {
                    report.push(Tone::Warning, format!("Check: {}", warning.message));
                }
                for line in format_material_left(&job, &result.material_left) {
                    report.push(Tone::Info, line);
                }
                report.push(Tone::Info, format!("Work zero: {}", job.work_zero_text()));
                self.set_report(report);
                self.result = Some(result);
                true
            }
            Err(PipelineError::CheckFailed { errors, program }) => {
                self.scene.toolpath = Some(ToolpathShape::new(&program, true, &[]));
                self.scene.tabs.clear();
                self.scene.left.clear();
                self.scene.start_markers.clear();
                let mut lines = vec!["G-code check FAILED. Nothing can be exported.".to_string()];
                lines.extend(errors);
                self.error(lines);
                false
            }
            Err(err) => {
                self.scene.toolpath = None;
                self.scene.tabs.clear();
                self.scene.left.clear();
                self.scene.start_markers.clear();
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
        match write_gcode_files(path, &result.files) {
            Ok(written) if written.len() > 1 => {
                let mut report = Report::default();
                report.push(
                    Tone::Good,
                    format!(
                        "Exported {} files: change the tool and set Z zero between them, \
                         keeping X and Y zero.",
                        written.len()
                    ),
                );
                for file in &result.files {
                    report.push(Tone::Info, file.describe(path));
                }
                self.set_report(report);
                self.set_status(
                    format!(
                        "Exported {} files next to {}",
                        written.len(),
                        path.display()
                    ),
                    STATUS_SECONDS,
                );
                true
            }
            Ok(_) => {
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
        self.record_recent(path);
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
            return vec!["Nothing selected.".into()];
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
        self.toolpath_stale = false;
        self.result = None;
        self.scene.toolpath = None;
        self.scene.tabs.clear();
        self.scene.left.clear();
        self.scene.start_markers.clear();
        self.start_drag = None;
        self.context_start = None;
    }

    fn toolpath_out_of_date(&mut self) {
        if self.result.is_some() || self.scene.toolpath.is_some() {
            self.invalidate();
            self.toolpath_stale = true;
        }
        self.scene.stock = self.edit.as_ref().map(|e| e.settings.stock.xy_bounds());
        self.refresh_drawing();
    }

    fn error(&mut self, lines: Vec<String>) {
        if let Some(first) = lines.first() {
            self.set_status(first.clone(), ERROR_STATUS_SECONDS);
        }
        self.set_report(Report::error(lines));
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
            Pending::New => self.new_project(),
            Pending::OpenDialog => self.open_dialog(),
            Pending::ImportDialog => self.import_dialog(),
            Pending::Open(path) => {
                self.open(&path);
            }
        }
    }

    /// The File menu's labels for bringing in a DXF: the first one into a project started
    /// empty, or a new version of it (docs/specs/new-project.md §3).
    pub fn update_labels(&self) -> (&'static str, &'static str) {
        let first = self
            .edit
            .as_ref()
            .is_some_and(|e| e.settings.drawing.source.is_none());
        if first {
            (
                "Import DXF into project…",
                "Import DXF into project from path…",
            )
        } else {
            ("Update drawing from DXF…", "Update drawing from path…")
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
        if self.applied_theme != Some(self.theme) {
            ctx.set_theme(theme_preference(self.theme));
            self.applied_theme = Some(self.theme);
        }
        self.shortcuts(&ctx);
        Panel::top("menu").show(ui, |ui| self.menu(ui));
        Panel::top("toolbar").show(ui, |ui| self.toolbar(ui));
        Panel::bottom("status").show(ui, |ui| self.status_bar(ui));
        self.report_panel(ui);
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
                ScrollArea::vertical()
                    .id_salt("properties-scroll")
                    .auto_shrink([false, true])
                    .show(ui, |ui| self.properties_panel(ui));
            });
        CentralPanel::no_frame().show(ui, |ui| {
            if self.edit.is_some() {
                self.canvas(ui);
            } else {
                self.start_screen(ui);
            }
        });
        self.prompts(&ctx);
        self.about(&ctx);
        self.library_windows_ui(&ctx);
        // One undo step per finished edit: not while dragging a value or typing.
        let busy = ctx.egui_is_using_pointer() || ctx.egui_wants_keyboard_input();
        if !busy {
            self.commit();
        }
        self.update_title(&ctx);
    }
}
