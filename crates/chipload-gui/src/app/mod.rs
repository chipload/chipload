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
use chipload_pipeline::shapes::{add_shape, delete_shape, set_shape};
use chipload_pipeline::sheet::{SheetInfo, sheet_path, write_setup_sheet};
use chipload_pipeline::{
    PipelineError, RunResult, empty_project, load_drawing, new_project, update_drawing,
    write_gcode_files,
};
use chipload_toolpath::select::{select_contours, select_paths};
use eframe::egui::{
    self, Align, CentralPanel, ComboBox, DragValue, Grid, Key, Layout, Modifiers, Panel, RichText,
    ScrollArea, Sense, TextEdit, ViewportCommand,
};

use crate::draw::{Sketch, SnapPoint, Step, Tool, grid_step, snap_cursor, snap_points};
use crate::editor::{CanvasSelection, EditJob, OpKind, geometry_summary, shapes_of};
use crate::shape_form::{ShapeAction, ShapeDraft, imported_lines, shape_form};
use crate::view::{ContourShape, Highlight, Scene, StartMarker, ToolpathShape, View};

mod browser;
mod canvas;
mod copy_paste;
mod dialogs;
mod drawing;
mod dropping;
mod forms;
mod generation;
mod header;
mod images;
mod library_transfer;
mod measure;
mod menu;
mod navigation;
mod panels;
mod playback;
mod problems;
mod recovery;
mod report;
mod shortcuts;
mod simulation;
mod sources;
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
    /// A picture to trace (File > Import image from path…).
    Image,
    /// Another DXF for the project (File > Add DXF from path…).
    AddDxf,
    /// A new version of the project's DXF with this id (the Browser's Update…).
    UpdateDxf(u64),
}

/// What a DXF without units was being brought in for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum UnitsFor {
    /// A new project.
    NewProject,
    /// A new version of the project's DXF with this id (None: its only one).
    Update(Option<u64>),
    /// Another DXF in the project (docs/specs/drawing-sources.md §2).
    Add,
}

/// A small modal asking for something before an action can continue.
#[derive(Debug, Clone, PartialEq)]
enum Prompt {
    /// The DXF declares no units: for what it was being brought in.
    Units(PathBuf, UnitsFor),
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
    /// The job `result` was made from (for the setup sheet).
    result_job: Option<Job>,
    /// Operations (indices) the last generation, validation or update complained about.
    pub error_ops: BTreeSet<usize>,
    /// What is wrong with the project as last edited (docs/specs/live-problems.md).
    pub problems: Vec<chipload_job::Problem>,

    // The canvas.
    pub scene: Scene,
    view: View,
    fit_pending: bool,
    pub selection: CanvasSelection,
    context_pick: Option<usize>,
    /// View > Show simulation (docs/specs/simulation.md §4).
    pub show_simulation: bool,
    sim_run: Option<simulation::SimRun>,
    /// The last simulation, and its image's texture (kept alive while shown).
    pub simulation: Option<chipload_pipeline::simulate::Simulation>,
    sim_texture: Option<egui::TextureHandle>,
    /// The texture of the stock part way through playback (docs/specs/playback.md).
    sim_playback_texture: Option<egui::TextureHandle>,
    /// The simulation's layer at the end of the program, shown when playback is there.
    sim_full_layer: Option<crate::view::SimLayer>,
    /// Every display cell's history, for playback; None: too long, or no simulation.
    sim_history: Option<chipload_pipeline::simulate::History>,
    /// The playback slider's state, once a toolpath has been generated.
    pub playback: Option<playback::Playback>,
    /// Counts the results shown: a simulation is for one of them.
    result_serial: u64,
    /// The canvas is in 3D: left-drag orbits (docs/specs/3d-view.md).
    pub three_d: bool,
    /// The shape being drawn with the mouse, if any (docs/specs/drawing-tools.md).
    pub sketch: Option<Sketch>,
    /// View → Measure, while on (docs/specs/small-items.md §3).
    pub measure: Option<measure::Measure>,
    /// The drawn shapes copied last (layer, geometry), for Ctrl+V.
    shape_clipboard: Vec<(String, Geometry)>,
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
    /// The project changed since the last generation, which is shown faded.
    pub toolpath_stale: bool,
    /// The run in progress (docs/specs/background-generation.md).
    generation: Option<generation::Generation>,
    /// What plans a project (a stand-in in tests).
    runner: generation::Runner,
    /// View > Update toolpath automatically, kept in the app settings.
    pub auto_generate: bool,
    /// Settings > Write a setup sheet with the G-code (docs/specs/setup-sheet.md §4).
    pub setup_sheet: bool,
    /// View → Show grid (kept in the app settings).
    pub show_grid: bool,
    /// When the project last changed, while the live preview hasn't generated it yet.
    changed_at: Option<Instant>,
    /// Export G-code once the run in progress has finished (Ctrl+E while generating).
    export_when_done: bool,
    /// This window's recovery copy (docs/specs/autosave-and-backup.md); None without a
    /// settings folder.
    recovery: Option<chipload_pipeline::recovery::Session>,
    /// When the project last changed, while its recovery copy isn't written yet.
    recovery_due: Option<Instant>,
    /// Recovery copies left by windows that closed without saving, offered at start.
    pub leftovers: Vec<chipload_pipeline::recovery::Leftover>,
    /// Settings > Colour theme, kept in the app settings.
    pub theme: Theme,
    /// The theme last handed to egui.
    applied_theme: Option<Theme>,
    status: Option<(String, Instant)>,
    cursor: Option<Point>,
    prompt: Option<Prompt>,
    title: String,
    allow_close: bool,
    about_open: bool,
    /// Help → Keyboard shortcuts is open.
    shortcuts_open: bool,

    /// The user's settings folder (docs/specs/tool-library-and-machines.md); None in tests
    /// and when there is no home folder.
    settings_dir: Option<PathBuf>,
    /// The tool library and machine profiles as last read.
    library: chipload_pipeline::library::Library,
    library_windows: LibraryWindows,
    /// File > Open recent (docs/specs/open-recent.md), most recent first.
    pub recent: Vec<String>,
    /// The image window, while open (docs/specs/image-trace.md §6).
    pub image_dialog: Option<images::ImageDialog>,
    /// Dropped files still to bring in, one at a time (docs/specs/drawing-sources.md §3).
    pub(super) drops: std::collections::VecDeque<PathBuf>,
    /// The DXF whose placement window is open (the Browser's Place…).
    pub(super) place_open: Option<u64>,
    /// Contours not drawn because their group or layer is hidden (drawing-sources.md §4).
    pub(super) hidden_contours: BTreeSet<usize>,
    /// View > Show images: the pictures faintly under the drawing.
    pub show_images: bool,
    /// View → Show toolpath (docs/specs/drawing-sources.md §5).
    pub show_toolpath: bool,
    /// The pictures' textures, for the images they were made from.
    picture_textures: images::PictureTextures,
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
            result_job: None,
            error_ops: BTreeSet::new(),
            problems: Vec::new(),
            scene: Scene::default(),
            view: View::default(),
            fit_pending: false,
            selection: CanvasSelection::Nothing,
            context_pick: None,
            // On at start: it is how the canvas shows what the tools leave
            // (docs/specs/material-left-merge.md, decision 2).
            show_simulation: true,
            sim_run: None,
            simulation: None,
            sim_texture: None,
            sim_playback_texture: None,
            sim_full_layer: None,
            sim_history: None,
            playback: None,
            result_serial: 0,
            three_d: false,
            sketch: None,
            measure: None,
            shape_clipboard: Vec::new(),
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
            generation: None,
            runner: generation::RUNNER,
            auto_generate: true,
            setup_sheet: true,
            show_grid: true,
            changed_at: None,
            export_when_done: false,
            recovery: None,
            recovery_due: None,
            leftovers: Vec::new(),
            theme: Theme::System,
            applied_theme: None,
            status: None,
            cursor: None,
            prompt: None,
            title: String::new(),
            allow_close: false,
            about_open: false,
            shortcuts_open: false,
            settings_dir: None,
            library: chipload_pipeline::library::Library::default(),
            library_windows: LibraryWindows::default(),
            recent: Vec::new(),
            image_dialog: None,
            drops: std::collections::VecDeque::new(),
            place_open: None,
            hidden_contours: BTreeSet::new(),
            show_images: true,
            show_toolpath: true,
            picture_textures: images::PictureTextures::default(),
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
        // The previous project was saved or its changes deliberately dropped.
        self.clear_recovery();
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
        self.refresh_problems();
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
                // Hidden groups and layers aren't drawn, so they can't be clicked either.
                self.hidden_contours = sources::hidden_contours(&job.drawing, &self.contours);
                self.scene.contours = self
                    .contours
                    .iter()
                    .filter(|c| !self.hidden_contours.contains(&c.id))
                    .map(ContourShape::new)
                    .collect();
                self.snap_points = snap_points(&self.contours);
                self.drawing_facts = Some(DrawingFacts::new(&imported.report, &self.contours));
            }
            Err(err) => {
                self.contours.clear();
                self.hidden_contours.clear();
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
                self.prompt = Some(Prompt::Units(path.to_path_buf(), UnitsFor::NewProject));
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
        self.update_dxf_from(None, dxf, units)
    }

    /// [`ChiploadApp::update_from`] for DXF `id` of several (None: the project's only one).
    pub fn update_dxf_from(&mut self, id: Option<u64>, dxf: &Path, units: Option<Units>) -> bool {
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
        let updated = match id {
            Some(id) => chipload_pipeline::dxfs::update_dxf(&job, id, dxf, units),
            None => update_drawing(&job, dxf, units),
        };
        match updated {
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
                self.prompt = Some(Prompt::Units(dxf.to_path_buf(), UnitsFor::Update(id)));
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

    /// Add `dxf` to the project as another DXF, where its file has it, as one undo step
    /// (docs/specs/drawing-sources.md §2). A DXF without units asks for them.
    pub fn add_dxf_from(&mut self, dxf: &Path, units: Option<Units>) -> bool {
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
        match chipload_pipeline::dxfs::add_dxf(&job, dxf, units) {
            Ok((added, report)) => {
                let mut next = EditJob::from_job(&added);
                for (entry, old) in next.ops.iter_mut().zip(&edit.ops) {
                    entry.side_chosen = old.side_chosen;
                }
                self.edit = Some(next);
                self.commit();
                self.refresh_drawing();
                self.set_report(Report::info(report.lines));
                true
            }
            Err(PipelineError::Import(ImportError::UnitsMissing)) => {
                self.prompt = Some(Prompt::Units(dxf.to_path_buf(), UnitsFor::Add));
                self.set_report(Report::info([format!(
                    "{} does not say which units it uses; choose mm or inch.",
                    file_name(dxf)
                )]));
                false
            }
            Err(err) => {
                self.error(vec![
                    format!("Could not add {}:", file_name(dxf)),
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
        // Shapes traced from one image are selected by its layer: their ids change each time
        // it is traced again (docs/specs/image-trace.md §4).
        let selection = self
            .traced_image()
            .map_or_else(|| self.selection.clone(), CanvasSelection::Source);
        let edit = self.edit.as_mut()?;
        let index = edit.add_with(kind, &selection, &self.contours, cutting.as_ref());
        self.focus = Focus::Op(index);
        self.commit();
        Some(index)
    }

    /// Set the copies' pitch so `gap` mm are left between their cuts (docs/specs/
    /// copies-grid.md §6). It plans the part once, so the project must be complete.
    /// Move the project's one DXF (docs/specs/drawing-placement.md), as one undo step.
    #[cfg(test)]
    pub fn place(&mut self, how: chipload_pipeline::placement::Move) -> bool {
        self.place_dxf(None, how)
    }

    /// Move DXF `dxf` (None: the project's one DXF), as one undo step
    /// (docs/specs/drawing-sources.md §2).
    pub fn place_dxf(&mut self, dxf: Option<u64>, how: chipload_pipeline::placement::Move) -> bool {
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
        let placed = match dxf {
            Some(id) => chipload_pipeline::placement::place_dxf(&job, id, how),
            None => chipload_pipeline::placement::place_drawing(&job, how),
        };
        match placed {
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
        let Some(id) = edit.settings.active_part_id() else {
            return;
        };
        let mut part = edit.clone();
        for p in &mut part.settings.parts {
            p.copies = None;
        }
        let job = match part.to_job() {
            Ok(job) => job,
            Err(problems) => {
                self.error(problems);
                return;
            }
        };
        let current = edit
            .settings
            .part(id)
            .and_then(|p| p.copies)
            .map_or((0.0, 0.0), |c| (c.dx, c.dy));
        match chipload_pipeline::copies::pitch_for_gap(&job, id, gap, current) {
            Ok((dx, dy)) => {
                let copies = self.edit.as_mut().and_then(|e| {
                    e.settings
                        .parts
                        .iter_mut()
                        .find(|p| p.id == id)
                        .and_then(|p| p.copies.as_mut())
                });
                if let Some(copies) = copies {
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

    /// Add a rest pocket after pocket `i`, cut with `tool` (docs/specs/rest-machining.md
    /// §10), and select it.
    pub fn add_rest_pocket(&mut self, i: usize, tool: i64) {
        let cutting = self.tool_cutting(tool);
        let Some(edit) = self.edit.as_mut() else {
            return;
        };
        let Some(Operation::Pocket(pocket)) = edit.ops.get(i).map(|e| &e.op) else {
            return;
        };
        match chipload_pipeline::rest_pocket_of(
            pocket,
            &edit.settings.tools,
            tool,
            cutting.as_ref(),
        ) {
            Ok(rest) => {
                edit.ops.insert(
                    i + 1,
                    crate::editor::EditOp {
                        op: Operation::Pocket(rest),
                        side_chosen: true,
                    },
                );
                self.focus = Focus::Op(i + 1);
                self.commit();
            }
            Err(err) => self.error(vec![format!("Could not add a rest pocket: {err}")]),
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

    /// Write the checked G-code of the last generation, and its setup sheet when the
    /// setting asks for one (docs/specs/setup-sheet.md §4).
    pub fn export_gcode(&mut self, path: &Path) -> bool {
        let Some(result) = &self.result else {
            self.error(vec!["Generate (F5) before exporting G-code.".into()]);
            return false;
        };
        let written = match write_gcode_files(path, &result.files) {
            Ok(written) => written,
            Err(err) => {
                self.error(vec![
                    format!("Could not export {}:", path.display()),
                    err.to_string(),
                ]);
                return false;
            }
        };
        let sheet = match (&self.result_job, self.setup_sheet) {
            (Some(job), true) => {
                let info = SheetInfo {
                    project: self
                        .job_path
                        .as_deref()
                        .map_or_else(|| "Untitled project".into(), file_name),
                    written_by: format!("chipload {}", env!("CARGO_PKG_VERSION")),
                };
                match write_setup_sheet(job, result, path, &info) {
                    Ok(sheet) => Some(sheet),
                    Err(err) => {
                        self.error(vec![
                            format!(
                                "Exported the G-code, but could not write the setup sheet {}:",
                                sheet_path(path).display()
                            ),
                            err.to_string(),
                        ]);
                        return false;
                    }
                }
            }
            _ => None,
        };
        let and_sheet = match &sheet {
            Some(sheet) => format!(" and the setup sheet {}", file_name(sheet)),
            None => String::new(),
        };
        if written.len() > 1 {
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
            if let Some(sheet) = &sheet {
                report.push(
                    Tone::Info,
                    format!("Setup sheet for the operator: {}", sheet.display()),
                );
            }
            self.set_report(report);
            self.set_status(
                format!(
                    "Exported {} files{and_sheet} next to {}",
                    written.len(),
                    path.display()
                ),
                STATUS_SECONDS,
            );
        } else {
            self.set_status(
                format!("Exported {}{and_sheet}", path.display()),
                STATUS_SECONDS,
            );
        }
        true
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
        self.clear_recovery();
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
            (CanvasSelection::Layer(..) | CanvasSelection::Source(_), true) => {
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

    #[cfg(test)]
    pub fn select_layer(&mut self, layer: &str) {
        self.selection = CanvasSelection::Layer(layer.to_string(), None);
    }

    /// A job selector for the canvas selection, e.g. {"shapes": [3, 4]}.
    pub fn selector_json(&self) -> Option<String> {
        let selector = self.selection.selector(&self.contours, false);
        let empty = selector.layer.is_none() && selector.shapes.as_ref().is_none_or(Vec::is_empty);
        (!empty).then(|| selector_text(&selector))
    }

    pub fn selection_lines(&self) -> Vec<String> {
        let ids = self.selection.ids(&self.contours);
        if ids.is_empty() {
            return vec!["Nothing selected.".into()];
        }
        let what = match &self.selection {
            CanvasSelection::Layer(layer, _) => format!("layer {layer}"),
            CanvasSelection::Source(chipload_core::chain::ShapeFrom::Dxf(id)) => {
                format!("DXF {id}")
            }
            CanvasSelection::Source(chipload_core::chain::ShapeFrom::Image(id)) => {
                format!("image {id}")
            }
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
        edit.ops
            .get(i)
            .map(|entry| self.op_contours(&entry.op))
            .unwrap_or_default()
    }

    /// The contours an operation cuts, as planning selects them.
    pub(super) fn op_contours(&self, op: &Operation) -> BTreeSet<usize> {
        let selected = match op {
            Operation::Path(_) | Operation::Holes(_) => {
                select_paths(&self.contours, op.select(), "Operation")
            }
            _ => select_contours(&self.contours, op.select(), "Operation"),
        };
        // Holes cut circles only: other shapes stay unhighlighted (docs/specs/holes.md §7).
        let holes = matches!(op, Operation::Holes(_));
        selected
            .map(|s| {
                s.contours
                    .iter()
                    .filter(|c| !holes || chipload_toolpath::holes::circle_of(c).is_some())
                    .map(|c| c.id)
                    .collect()
            })
            .unwrap_or_default()
    }

    /// How many of an operation's contours are hidden (docs/specs/drawing-sources.md §4).
    pub(super) fn hidden_cut(&self, op: &Operation) -> usize {
        if self.hidden_contours.is_empty() {
            return 0;
        }
        self.op_contours(op)
            .intersection(&self.hidden_contours)
            .count()
    }

    /// The open contours an enabled follow path cuts (docs/specs/follow-path.md §7).
    fn followed_contours(&self) -> BTreeSet<usize> {
        let Some(edit) = &self.edit else {
            return BTreeSet::new();
        };
        edit.ops
            .iter()
            .filter(|e| e.op.enabled() && matches!(e.op, Operation::Path(_)))
            .filter_map(|e| select_paths(&self.contours, e.op.select(), "Operation").ok())
            .flat_map(|s| {
                s.contours
                    .iter()
                    .filter(|c| !c.closed)
                    .map(|c| c.id)
                    .collect::<Vec<_>>()
            })
            .collect()
    }

    /// Inputs changed: the old toolpath and G-code no longer apply.
    fn invalidate(&mut self) {
        self.stop_generation();
        self.changed_at = None;
        self.export_when_done = false;
        self.toolpath_stale = false;
        self.result = None;
        self.scene.toolpath = None;
        self.scene.tabs.clear();
        self.scene.start_markers.clear();
        self.start_drag = None;
        self.context_start = None;
    }

    /// The project changed: the toolpath shown is kept, faded, until a new one replaces it
    /// (docs/specs/background-generation.md decision 4); a run in progress is cancelled.
    fn toolpath_out_of_date(&mut self) {
        self.refresh_problems();
        self.stop_generation();
        self.changed_at = Some(Instant::now());
        self.recovery_due = self.changed_at;
        if self.result.is_some() || self.scene.toolpath.is_some() {
            self.result = None;
            if let Some(path) = &mut self.scene.toolpath {
                path.stale = true;
            }
            self.scene.tabs.clear();
            self.scene.start_markers.clear();
            self.start_drag = None;
            self.context_start = None;
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
                self.clear_recovery();
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
            .is_some_and(|e| e.settings.drawing.dxfs.is_empty());
        if first {
            (
                "Import DXF into project…",
                "Import DXF into project from path…",
            )
        } else {
            ("Update drawing from DXF…", "Update drawing from path…")
        }
    }

    /// How many DXFs the open project has.
    pub(super) fn dxf_count(&self) -> usize {
        self.edit
            .as_ref()
            .map_or(0, |e| e.settings.drawing.dxfs.len())
    }

    fn default_project_path(&self) -> PathBuf {
        if let Some(path) = &self.job_path {
            return path.with_extension(PROJECT_EXTENSION);
        }
        self.edit
            .as_ref()
            .and_then(|e| e.settings.drawing.dxfs.first())
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
        if ctx.input(|i| i.viewport().close_requested()) {
            if self.dirty() && !self.allow_close {
                ctx.send_viewport_cmd(ViewportCommand::CancelClose);
                self.prompt = Some(Prompt::Discard(Pending::Close));
            } else if !self.dirty() {
                self.clear_recovery(); // nothing unsaved: no copy to leave behind
            }
        }
        if self.applied_theme != Some(self.theme) {
            ctx.set_theme(theme_preference(self.theme));
            self.applied_theme = Some(self.theme);
        }
        self.shortcuts(&ctx);
        Panel::top("top-bar")
            .exact_size(header::TOP_BAR_HEIGHT)
            .frame(
                egui::Frame::new()
                    .fill(crate::style::top_bar_fill(ui.visuals()))
                    .inner_margin(egui::Margin::symmetric(6, 0)),
            )
            .show(ui, |ui| self.top_bar(ui));
        Panel::top("ribbon").show(ui, |ui| self.ribbon(ui));
        Panel::bottom("status").show(ui, |ui| self.status_bar(ui));
        self.report_panel(ui);
        Panel::left("operations")
            .resizable(true)
            .default_size(340.0)
            .show(ui, |ui| self.sidebar(ui));
        Panel::right("side")
            .resizable(true)
            .default_size(440.0)
            .show(ui, |ui| {
                browser::caption(ui, "Properties");
                ScrollArea::vertical()
                    .id_salt("properties-scroll")
                    .auto_shrink([false, true])
                    .show(ui, |ui| self.properties_panel(ui));
            });
        CentralPanel::no_frame().show(ui, |ui| {
            if self.edit.is_some() && self.playback.is_some() {
                Panel::bottom("playback").show(ui, |ui| self.playback_bar(ui));
                self.sync_playback(&ctx);
            }
            if self.edit.is_some() {
                self.canvas(ui);
            } else {
                self.start_screen(ui);
            }
        });
        self.prompts(&ctx);
        self.about(&ctx);
        self.shortcuts_window(&ctx);
        self.file_drops(&ctx);
        self.next_drop();
        self.library_windows_ui(&ctx);
        self.image_window(&ctx);
        self.place_window(&ctx);
        self.sync_pictures(&ctx);
        self.recovery_window(&ctx);
        // One undo step per finished edit: not while dragging a value or typing.
        let busy = ctx.egui_is_using_pointer() || ctx.egui_wants_keyboard_input();
        if !busy {
            self.commit();
        }
        if self.generation.is_some() {
            self.poll_generation();
            ctx.request_repaint_after(generation::POLL);
        }
        if self.poll_simulation(&ctx) {
            ctx.request_repaint_after(generation::POLL);
        }
        if !busy
            && self.handle_drag.is_none()
            && let Some(wait) = self.live_preview(Instant::now())
        {
            ctx.request_repaint_after(wait);
        }
        if !busy && let Some(wait) = self.recovery_tick(Instant::now()) {
            ctx.request_repaint_after(wait);
        }
        self.update_title(&ctx);
    }
}
