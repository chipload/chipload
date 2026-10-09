// SPDX-License-Identifier: GPL-3.0-or-later
//! Main window: open a drawing, create and edit a job (operations from selected geometry, in
//! any order), generate on F5, save the job, export checked G-code.
//! See docs/specs/job-editor.md. Thin by design: geometry and G-code decisions are calls into
//! the pipeline; this file only edits a `Job` and shows results.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use chipload_core::Point;
use chipload_core::chain::Contour;
use chipload_dxf::{ImportError, import_dxf};
use chipload_job::{
    Direction, Operation, Selector, Side, Spindle, StockOrigin, Tabs, ToolType, Travel, Units,
    ZZero, load_job, save_job,
};
use chipload_pipeline::report::{format_contours, format_import_report};
use chipload_pipeline::{PipelineError, RunResult, drawing_sha256, run_job, write_atomically};
use chipload_toolpath::select::select_contours;
use eframe::egui::{
    self, Align, CentralPanel, ComboBox, DragValue, Grid, Key, KeyboardShortcut, Layout, MenuBar,
    Modifiers, Panel, RichText, ScrollArea, Sense, TextEdit, ViewportCommand,
};

use crate::editor::{CanvasSelection, EditJob, OpKind, geometry_summary};
use crate::view::{ContourShape, Scene, ToolpathShape, View};

const STATUS_SECONDS: u64 = 5;
const ERROR_STATUS_SECONDS: u64 = 10;

const NEW_JOB: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::N);
const OPEN_DXF: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::O);
const OPEN_JOB: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::J);
const SAVE_JOB_AS: KeyboardShortcut =
    KeyboardShortcut::new(Modifiers::COMMAND.plus(Modifiers::SHIFT), Key::S);
const SAVE_JOB: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::S);
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

/// Something that would throw away unsaved job changes, waiting for confirmation.
#[derive(Debug, Clone, PartialEq)]
enum Pending {
    Close,
    NewJob,
    OpenJobDialog,
    OpenJob(PathBuf),
}

/// A small modal asking for something before an action can continue.
#[derive(Debug, Clone, PartialEq)]
enum Prompt {
    /// The DXF declares no units and no job is open.
    Units,
    /// Type or paste a path (for systems without a native file dialog).
    OpenPath(String),
    SaveJobPath(String),
    ExportPath(String),
    Discard(Pending),
}

pub struct ChiploadApp {
    // The drawing.
    pub dxf_path: Option<PathBuf>,
    pub contours: Vec<Contour>,
    drawing_units: Option<Units>,
    drawing_bbox: Option<(Point, Point)>,
    /// Units the user chose for a unitless DXF (used while no job says otherwise).
    dxf_units: Option<Units>,

    // The job being edited.
    pub edit: Option<EditJob>,
    pub job_path: Option<PathBuf>,
    /// The job as last opened or saved; differs from `edit` when there are unsaved changes.
    saved: Option<EditJob>,
    /// The job as last recorded for undo.
    committed: Option<EditJob>,
    undo: Vec<Option<EditJob>>,
    redo: Vec<Option<EditJob>>,
    pub focus: Focus,

    // The last generation.
    pub result: Option<RunResult>,
    /// Operations (indices) the last generation or validation complained about.
    pub error_ops: BTreeSet<usize>,

    // The canvas.
    pub scene: Scene,
    view: View,
    fit_pending: bool,
    pub selection: CanvasSelection,
    context_pick: Option<usize>,

    pub report: Vec<String>,
    status: Option<(String, Instant)>,
    cursor: Option<Point>,
    prompt: Option<Prompt>,
    title: String,
    allow_close: bool,
}

impl Default for ChiploadApp {
    fn default() -> Self {
        ChiploadApp {
            dxf_path: None,
            contours: Vec::new(),
            drawing_units: None,
            drawing_bbox: None,
            dxf_units: None,
            edit: None,
            job_path: None,
            saved: None,
            committed: None,
            undo: Vec::new(),
            redo: Vec::new(),
            focus: Focus::Job,
            result: None,
            error_ops: BTreeSet::new(),
            scene: Scene::default(),
            view: View::default(),
            fit_pending: false,
            selection: CanvasSelection::Nothing,
            context_pick: None,
            report: vec![
                "Open a drawing (Ctrl+O), then create a job (Ctrl+N) or open one (Ctrl+J).".into(),
            ],
            status: None,
            cursor: None,
            prompt: None,
            title: String::new(),
            allow_close: false,
        }
    }
}

fn file_name(path: &Path) -> String {
    path.file_name().map_or_else(
        || path.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    )
}

/// The `schema_version` a job file was written with, before migration (for the save note).
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

fn selector_of(op: &Operation) -> &Selector {
    match op {
        Operation::Profile(p) => &p.select,
        Operation::Pocket(p) => &p.select,
        Operation::Drill(d) => &d.select,
    }
}

fn selector_text(selector: &Selector) -> String {
    match (&selector.layer, &selector.handles) {
        (Some(layer), _) => format!("{{\"layer\": \"{layer}\"}}"),
        (None, Some(handles)) => {
            let quoted: Vec<String> = handles.iter().map(|h| format!("\"{h}\"")).collect();
            format!("{{\"handles\": [{}]}}", quoted.join(", "))
        }
        (None, None) => "{}".into(),
    }
}

impl ChiploadApp {
    // -- state

    /// The job has changes that aren't saved.
    pub fn dirty(&self) -> bool {
        self.edit.is_some() && self.edit != self.saved
    }

    /// Record the current job as one undo step if it changed since the last one. Called once
    /// the user has finished an edit (no drag or text entry in progress).
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
    fn start_editing(&mut self, job: Option<EditJob>, saved: bool) {
        self.saved = if saved { job.clone() } else { None };
        self.committed.clone_from(&job);
        self.edit = job;
        self.undo.clear();
        self.redo.clear();
        self.focus = Focus::Job;
        self.error_ops.clear();
        self.scene.stock = self.edit.as_ref().map(|e| e.settings.stock.xy_bounds());
        self.invalidate();
    }

    // -- operations (also called directly by tests)

    /// Show a drawing. Units come from the job if one is open, else from the file, else
    /// from the user. Never guessed. An open job is kept.
    pub fn open_dxf(&mut self, path: &Path) -> bool {
        self.dxf_path = Some(path.to_path_buf());
        self.dxf_units = None;
        self.invalidate();
        let shown = self.show_drawing();
        if shown
            && let Some(edit) = &self.edit
            && !edit.settings.source.file.is_empty()
            && edit.settings.source.file != file_name(path)
        {
            self.report.insert(
                0,
                format!(
                    "Note: the job was made for {}, this drawing is {}.",
                    edit.settings.source.file,
                    file_name(path)
                ),
            );
        }
        shown
    }

    /// Open a job. If no drawing is open, its `source.file` next to the job is opened too.
    pub fn open_job(&mut self, path: &Path) -> bool {
        let job = match load_job(path) {
            Ok(job) => job,
            Err(err) => {
                self.error(vec![
                    format!("Cannot open job {}:", file_name(path)),
                    err.to_string(),
                ]);
                return false;
            }
        };
        let version = raw_schema_version(path);
        self.start_editing(Some(EditJob::from_job(&job)), true);
        self.job_path = Some(path.to_path_buf());
        let drawing = path.with_file_name(&job.source.file);
        if self.dxf_path.is_none() && drawing.is_file() {
            self.open_dxf(&drawing);
        } else if self.dxf_path.is_some() {
            self.show_drawing();
        } else {
            self.report = vec![
                format!("Job: {}", file_name(path)),
                format!(
                    "Its drawing {} is not next to it; open the drawing (Ctrl+O).",
                    job.source.file
                ),
            ];
        }
        if let Some(v) = version
            && v < chipload_job::CURRENT_VERSION
        {
            self.report.insert(
                0,
                format!(
                    "This job file is schema version {v}; saving writes version {}.",
                    chipload_job::CURRENT_VERSION
                ),
            );
        }
        self.fit_pending = true;
        true
    }

    /// A new job for the open drawing (docs/specs/job-editor.md §6).
    pub fn new_job(&mut self) -> bool {
        let (Some(path), Some(units)) = (&self.dxf_path, self.drawing_units) else {
            self.error(vec!["Open a drawing first (Ctrl+O).".into()]);
            return false;
        };
        let job = EditJob::for_drawing(&file_name(path), units, self.drawing_bbox);
        self.start_editing(Some(job), false);
        self.job_path = None;
        self.fit_pending = true;
        self.report = vec![
            "New job. Select contours, then New profile or New pocket.".into(),
            "Check the stock, machine and tool under Job settings.".into(),
        ];
        true
    }

    /// The user answered the units question for a unitless DXF.
    pub fn choose_units(&mut self, units: Units) -> bool {
        self.prompt = None;
        self.dxf_units = Some(units);
        self.show_drawing()
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
        let (Some(edit), Some(dxf)) = (&self.edit, &self.dxf_path) else {
            self.error(vec![
                "Open a drawing and create or open a job before generating.".into(),
            ]);
            return false;
        };
        let job = match edit.to_job() {
            Ok(job) => job,
            Err(problems) => {
                self.error_ops = problems.iter().filter_map(|p| operation_index(p)).collect();
                let mut lines = vec!["The job is not complete:".to_string()];
                lines.extend(problems.into_iter().map(|p| format!("  {p}")));
                self.error(lines);
                return false;
            }
        };
        match run_job(&job, dxf) {
            Ok(result) => {
                self.error_ops.clear();
                self.contours.clone_from(&result.contours);
                self.scene.contours = result.contours.iter().map(ContourShape::new).collect();
                self.scene.stock = Some(job.stock.xy_bounds());
                self.scene.toolpath = Some(ToolpathShape::new(&result.program, false));
                self.scene.tabs.clone_from(&result.tabs);
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
                lines.push(String::new());
                lines.extend(format_import_report(&result.import_report));
                lines.extend(format_contours(&result.contours));
                self.report = lines;
                self.result = Some(result);
                true
            }
            Err(PipelineError::CheckFailed { errors, program }) => {
                self.scene.toolpath = Some(ToolpathShape::new(&program, true));
                self.scene.tabs.clear();
                let mut lines = vec!["G-code check FAILED. Nothing can be exported.".to_string()];
                lines.extend(errors);
                self.error(lines);
                false
            }
            Err(err) => {
                self.scene.toolpath = None;
                self.scene.tabs.clear();
                let message = err.to_string();
                self.error_ops = operation_index(&message).into_iter().collect();
                let mut lines = vec!["Generation failed:".into(), message];
                if matches!(&err, PipelineError::Job(m) if m.contains("sha256")) {
                    lines.push(
                        "If you changed the drawing on purpose, use Job > Re-lock to drawing."
                            .into(),
                    );
                }
                self.error(lines);
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

    /// Save the job (current schema). A job using handle selectors is locked to the drawing
    /// (`source.sha256`) if it isn't yet.
    pub fn save_job(&mut self, path: &Path) -> bool {
        self.commit();
        let Some(edit) = &self.edit else {
            return false;
        };
        let mut job = match edit.to_job() {
            Ok(job) => job,
            Err(problems) => {
                let mut lines = vec!["The job can't be saved yet:".to_string()];
                lines.extend(problems.into_iter().map(|p| format!("  {p}")));
                self.error(lines);
                return false;
            }
        };
        let mut locked_now = false;
        if edit.uses_handles()
            && job.source.sha256.is_none()
            && let Some(dxf) = &self.dxf_path
        {
            match drawing_sha256(dxf) {
                Ok(sha) => {
                    job.source.sha256 = Some(sha);
                    locked_now = true;
                }
                Err(err) => {
                    self.error(vec![
                        format!("Cannot read {} to lock the job to it:", dxf.display()),
                        err.to_string(),
                    ]);
                    return false;
                }
            }
        }
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
        let mut message = format!("Saved {}", path.display());
        if locked_now {
            message.push_str(" (locked to this drawing: it uses handle selectors)");
        }
        self.set_status(message, STATUS_SECONDS);
        true
    }

    /// Lock the job to the open drawing's current contents (after a deliberate change).
    pub fn relock(&mut self) {
        let Some(dxf) = self.dxf_path.clone() else {
            return;
        };
        match drawing_sha256(&dxf) {
            Ok(sha) => {
                if let Some(edit) = self.edit.as_mut() {
                    edit.settings.source.sha256 = Some(sha);
                    edit.settings.source.file = file_name(&dxf);
                    self.commit();
                }
            }
            Err(err) => self.error(vec![
                format!("Cannot read {}:", dxf.display()),
                err.to_string(),
            ]),
        }
    }

    pub fn unlock(&mut self) {
        if let Some(edit) = self.edit.as_mut() {
            edit.settings.source.sha256 = None;
            self.commit();
        }
    }

    /// Click on the canvas: select one contour, Ctrl toggles, empty space clears.
    pub fn click(&mut self, picked: Option<usize>, toggle: bool) {
        let mut ids = match (&self.selection, toggle) {
            (CanvasSelection::Contours(ids), true) => ids.clone(),
            (CanvasSelection::Layer(_), true) => self.selection.ids(&self.contours),
            _ => BTreeSet::new(),
        };
        match (picked, toggle) {
            (Some(id), true) => {
                if !ids.remove(&id) {
                    ids.insert(id);
                }
            }
            (Some(id), false) => {
                ids.insert(id);
            }
            (None, true) => {}
            (None, false) => ids.clear(),
        }
        self.selection = if ids.is_empty() {
            CanvasSelection::Nothing
        } else {
            CanvasSelection::Contours(ids)
        };
    }

    pub fn select_layer(&mut self, layer: &str) {
        self.selection = CanvasSelection::Layer(layer.to_string());
    }

    /// A job selector for the canvas selection, e.g. {"handles": ["5C", "5D"]}.
    pub fn selector_json(&self) -> Option<String> {
        let selector = self.selection.selector(&self.contours);
        let empty = selector.layer.is_none() && selector.handles.as_ref().is_none_or(Vec::is_empty);
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
        select_contours(&self.contours, selector_of(&entry.op), "Operation")
            .map(|s| s.contours.iter().map(|c| c.id).collect())
            .unwrap_or_default()
    }

    fn show_drawing(&mut self) -> bool {
        let Some(path) = self.dxf_path.clone() else {
            return false;
        };
        let units = self
            .edit
            .as_ref()
            .map(|e| e.settings.source.units)
            .or(self.dxf_units);
        let imported = match import_dxf(&path, units) {
            Ok(imported) => imported,
            Err(ImportError::UnitsMissing) => {
                self.prompt = Some(Prompt::Units);
                self.report = vec![format!(
                    "{} does not say which units it uses; choose mm or inch.",
                    file_name(&path)
                )];
                return false;
            }
            Err(err) => {
                self.contours.clear();
                self.scene.contours.clear();
                self.error(vec![
                    format!("Cannot open {}:", file_name(&path)),
                    err.to_string(),
                ]);
                return false;
            }
        };
        self.contours = imported.contours;
        self.drawing_units = Some(imported.report.units);
        self.drawing_bbox = imported.report.bbox;
        self.selection = CanvasSelection::Nothing;
        self.scene.contours = self.contours.iter().map(ContourShape::new).collect();
        self.fit_pending = true;
        let mut lines = vec![format!("Drawing: {}", file_name(&path))];
        lines.extend(format_import_report(&imported.report));
        lines.extend(format_contours(&self.contours));
        self.report = lines;
        true
    }

    /// Inputs changed: the old toolpath and G-code no longer apply.
    fn invalidate(&mut self) {
        self.result = None;
        self.scene.toolpath = None;
        self.scene.tabs.clear();
    }

    fn toolpath_out_of_date(&mut self) {
        if self.result.is_some() || self.scene.toolpath.is_some() {
            self.invalidate();
            self.set_status(
                "Job changed: press F5 to generate again".into(),
                STATUS_SECONDS,
            );
        }
        self.scene.stock = self.edit.as_ref().map(|e| e.settings.stock.xy_bounds());
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
        self.edit.is_some() && self.dxf_path.is_some()
    }

    /// Proceed now, or ask first if unsaved job changes would be lost.
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
            Pending::NewJob => {
                self.new_job();
            }
            Pending::OpenJobDialog => self.open_job_dialog(),
            Pending::OpenJob(path) => {
                self.open_job(&path);
            }
        }
    }

    fn default_job_path(&self) -> PathBuf {
        self.job_path.clone().unwrap_or_else(|| {
            self.dxf_path
                .as_ref()
                .map_or_else(|| PathBuf::from("job.json"), |d| d.with_extension("json"))
        })
    }

    fn default_export_path(&self) -> PathBuf {
        let ext = self.result.as_ref().map_or("nc".to_string(), |r| {
            r.post.file_extension.trim_start_matches('.').to_string()
        });
        self.dxf_path
            .as_ref()
            .map_or_else(|| PathBuf::from("out.nc"), |d| d.with_extension(ext))
    }

    // -- dialogs

    fn ask_open_dxf(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("DXF drawings", &["dxf"])
            .pick_file()
        {
            self.open_dxf(&path);
        }
    }

    fn ask_open_job(&mut self) {
        if self.guard(Pending::OpenJobDialog) {
            self.open_job_dialog();
        }
    }

    fn open_job_dialog(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Chipload jobs", &["json"])
            .pick_file()
        {
            self.open_job(&path);
        }
    }

    fn ask_new_job(&mut self) {
        if self.guard(Pending::NewJob) {
            self.new_job();
        }
    }

    fn ask_save_job(&mut self, save_as: bool) {
        if self.edit.is_none() {
            return;
        }
        if !save_as && let Some(path) = self.job_path.clone() {
            self.save_job(&path);
            return;
        }
        let suggested = self.default_job_path();
        if let Some(path) = save_dialog(&suggested, "Chipload jobs", "json") {
            self.save_job(&path);
        }
    }

    fn ask_export(&mut self) {
        if self.result.is_none() {
            self.error(vec!["Generate (F5) before exporting G-code.".into()]);
            return;
        }
        let suggested = self.default_export_path();
        if let Some(path) = save_dialog(&suggested, "G-code", "nc") {
            self.export_gcode(&path);
        }
    }

    fn copy_selector(&mut self, ctx: &egui::Context) {
        if let Some(text) = self.selector_json() {
            ctx.copy_text(text);
            self.set_status("Selector copied to the clipboard".into(), STATUS_SECONDS);
        }
    }

    // -- user interface

    fn shortcuts(&mut self, ctx: &egui::Context) {
        // Not while typing in a text field.
        if ctx.egui_wants_keyboard_input() {
            return;
        }
        let pressed = |s: &KeyboardShortcut| ctx.input_mut(|i| i.consume_shortcut(s));
        // Shift variants first: they also contain the plain shortcut.
        if pressed(&COPY_SELECTOR) {
            self.copy_selector(ctx);
        }
        if pressed(&SAVE_JOB_AS) {
            self.ask_save_job(true);
        }
        if pressed(&REDO) {
            self.redo();
        }
        if pressed(&SAVE_JOB) {
            self.ask_save_job(false);
        }
        if pressed(&UNDO) {
            self.undo();
        }
        if pressed(&NEW_JOB) {
            self.ask_new_job();
        }
        if pressed(&OPEN_DXF) {
            self.ask_open_dxf();
        }
        if pressed(&OPEN_JOB) {
            self.ask_open_job();
        }
        if pressed(&EXPORT) {
            self.ask_export();
        }
        if pressed(&GENERATE) && self.can_generate() {
            self.generate();
        }
        if let Focus::Op(i) = self.focus {
            if pressed(&MOVE_UP) && i > 0 {
                self.move_operation(i, i - 1);
            }
            if pressed(&MOVE_DOWN) {
                self.move_operation(i, i + 1);
            }
        }
        if pressed(&FIT) {
            self.fit_pending = true;
        }
        if pressed(&QUIT) {
            ctx.send_viewport_cmd(ViewportCommand::Close);
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
            self.selection = CanvasSelection::Nothing;
        }
    }

    fn menu(&mut self, ui: &mut egui::Ui) {
        let has_job = self.edit.is_some();
        let has_result = self.result.is_some();
        MenuBar::new().ui(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("Open drawing…  (Ctrl+O)").clicked() {
                    self.ask_open_dxf();
                }
                if ui.button("Open job…  (Ctrl+J)").clicked() {
                    self.ask_open_job();
                }
                if ui.button("Open path…").clicked() {
                    self.prompt = Some(Prompt::OpenPath(String::new()));
                }
                ui.separator();
                if ui
                    .add_enabled(
                        self.dxf_path.is_some(),
                        egui::Button::new("New job  (Ctrl+N)"),
                    )
                    .clicked()
                {
                    self.ask_new_job();
                }
                if ui
                    .add_enabled(has_job, egui::Button::new("Save job  (Ctrl+S)"))
                    .clicked()
                {
                    self.ask_save_job(false);
                }
                if ui
                    .add_enabled(has_job, egui::Button::new("Save job as…  (Ctrl+Shift+S)"))
                    .clicked()
                {
                    self.ask_save_job(true);
                }
                if ui
                    .add_enabled(has_job, egui::Button::new("Save job to path…"))
                    .clicked()
                {
                    self.prompt = Some(Prompt::SaveJobPath(
                        self.default_job_path().display().to_string(),
                    ));
                }
                ui.separator();
                if ui
                    .add_enabled(has_result, egui::Button::new("Export G-code…  (Ctrl+E)"))
                    .clicked()
                {
                    self.ask_export();
                }
                if ui
                    .add_enabled(has_result, egui::Button::new("Export G-code to path…"))
                    .clicked()
                {
                    self.prompt = Some(Prompt::ExportPath(
                        self.default_export_path().display().to_string(),
                    ));
                }
                ui.separator();
                if ui.button("Quit  (Ctrl+Q)").clicked() {
                    ui.ctx().send_viewport_cmd(ViewportCommand::Close);
                }
            });
            ui.menu_button("Edit", |ui| {
                if ui
                    .add_enabled(!self.undo.is_empty(), egui::Button::new("Undo  (Ctrl+Z)"))
                    .clicked()
                {
                    self.undo();
                }
                if ui
                    .add_enabled(
                        !self.redo.is_empty(),
                        egui::Button::new("Redo  (Ctrl+Shift+Z)"),
                    )
                    .clicked()
                {
                    self.redo();
                }
                ui.separator();
                if ui
                    .add_enabled(
                        self.selector_json().is_some(),
                        egui::Button::new("Copy as selector  (Ctrl+Shift+C)"),
                    )
                    .clicked()
                {
                    self.copy_selector(ui.ctx());
                }
            });
            ui.menu_button("Job", |ui| {
                if ui
                    .add_enabled(self.can_generate(), egui::Button::new("Generate  (F5)"))
                    .clicked()
                {
                    self.generate();
                }
                ui.separator();
                let locked = self
                    .edit
                    .as_ref()
                    .is_some_and(|e| e.settings.source.sha256.is_some());
                if ui
                    .add_enabled(
                        has_job && self.dxf_path.is_some(),
                        egui::Button::new("Re-lock to drawing"),
                    )
                    .clicked()
                {
                    self.relock();
                }
                if ui
                    .add_enabled(locked, egui::Button::new("Unlock from drawing"))
                    .clicked()
                {
                    self.unlock();
                }
            });
            ui.menu_button("View", |ui| {
                if ui.button("Fit view  (F)").clicked() {
                    self.fit_pending = true;
                }
            });
            ui.separator();
            if ui.button("Open drawing").clicked() {
                self.ask_open_dxf();
            }
            if ui.button("Open job").clicked() {
                self.ask_open_job();
            }
            if ui
                .add_enabled(has_job, egui::Button::new("Save job"))
                .clicked()
            {
                self.ask_save_job(false);
            }
            if ui
                .add_enabled(self.can_generate(), egui::Button::new("Generate (F5)"))
                .clicked()
            {
                self.generate();
            }
            if ui
                .add_enabled(has_result, egui::Button::new("Export G-code"))
                .clicked()
            {
                self.ask_export();
            }
            if ui.button("Fit").clicked() {
                self.fit_pending = true;
            }
        });
    }

    fn status_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if let Some((text, until)) = &self.status {
                if Instant::now() < *until {
                    ui.label(text);
                    ui.ctx().request_repaint_after(*until - Instant::now());
                } else {
                    self.status = None;
                }
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if let Some((x, y)) = self.cursor {
                    ui.monospace(format!("X {x:.3}  Y {y:.3} mm"));
                }
            });
        });
    }

    /// Left: operations in cut order, the canvas selection, layers.
    fn operations_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("Job");
        if self.edit.is_none() {
            ui.label("No job open.");
            if ui
                .add_enabled(
                    self.dxf_path.is_some(),
                    egui::Button::new("New job from drawing"),
                )
                .clicked()
            {
                self.ask_new_job();
            }
            if ui.button("Open job…").clicked() {
                self.ask_open_job();
            }
        } else {
            self.operation_list(ui);
        }
        ui.separator();
        ui.heading("Selection");
        for line in self.selection_lines() {
            ui.label(line);
        }
        ui.horizontal(|ui| {
            let has_job = self.edit.is_some();
            if ui
                .add_enabled(has_job, egui::Button::new("New profile"))
                .on_hover_text("A profile operation for the selected contours")
                .clicked()
            {
                self.add_operation(OpKind::Profile);
            }
            if ui
                .add_enabled(has_job, egui::Button::new("New pocket"))
                .on_hover_text("A pocket operation for the selected contours")
                .clicked()
            {
                self.add_operation(OpKind::Pocket);
            }
            if ui
                .add_enabled(
                    self.selector_json().is_some(),
                    egui::Button::new("Copy as selector"),
                )
                .clicked()
            {
                self.copy_selector(ui.ctx());
            }
        });
        ui.collapsing("Layers", |ui| {
            let mut layers: Vec<(String, usize)> = Vec::new();
            for c in &self.contours {
                for layer in c.layers() {
                    match layers.iter_mut().find(|(l, _)| *l == layer) {
                        Some((_, n)) => *n += 1,
                        None => layers.push((layer, 1)),
                    }
                }
            }
            layers.sort();
            for (layer, count) in layers {
                if ui
                    .button(format!("Select layer {layer} ({count})"))
                    .clicked()
                {
                    self.select_layer(&layer);
                }
            }
        });
    }

    fn operation_list(&mut self, ui: &mut egui::Ui) {
        let Some(edit) = self.edit.as_mut() else {
            return;
        };
        if ui
            .selectable_label(
                self.focus == Focus::Job,
                "Job settings (stock, machine, tool)",
            )
            .clicked()
        {
            self.focus = Focus::Job;
        }
        let missing: BTreeSet<usize> = edit.missing().into_iter().map(|(i, _)| i).collect();
        let mut moved: Option<(usize, usize)> = None;
        let count = edit.ops.len();
        for i in 0..count {
            let entry = &mut edit.ops[i];
            let summary = geometry_summary(&entry.op, &self.contours);
            let kind = match &entry.op {
                Operation::Profile(_) => "Profile",
                Operation::Pocket(_) => "Pocket",
                Operation::Drill(_) => "Drill",
            };
            let flagged = missing.contains(&i) || self.error_ops.contains(&i);
            let row = ui.horizontal(|ui| {
                ui.dnd_drag_source(egui::Id::new(("operation", i)), i, |ui| {
                    ui.label("≡")
                        .on_hover_text("Drag onto another operation to move it there");
                });
                let mut enabled = entry.op.enabled();
                if ui
                    .checkbox(&mut enabled, "")
                    .on_hover_text("Enabled")
                    .changed()
                {
                    match &mut entry.op {
                        Operation::Profile(p) => p.enabled = enabled,
                        Operation::Pocket(p) => p.enabled = enabled,
                        Operation::Drill(d) => d.enabled = enabled,
                    }
                }
                let mut text = RichText::new(format!(
                    "{}. {kind}: {}  [{summary}]",
                    i + 1,
                    entry.op.name()
                ));
                if !enabled {
                    text = text.weak();
                }
                if flagged {
                    text = text.color(egui::Color32::from_rgb(0xf2, 0x8b, 0x82));
                }
                if ui
                    .selectable_label(self.focus == Focus::Op(i), text)
                    .clicked()
                {
                    self.focus = Focus::Op(i);
                }
            });
            if let Some(from) = row.response.dnd_release_payload::<usize>() {
                moved = Some((*from, i));
            } else if row.response.dnd_hover_payload::<usize>().is_some() {
                ui.painter().hline(
                    row.response.rect.x_range(),
                    row.response.rect.top(),
                    egui::Stroke::new(2.0, ui.visuals().selection.stroke.color),
                );
            }
        }
        let focused = match self.focus {
            Focus::Op(i) if i < count => Some(i),
            _ => None,
        };
        ui.horizontal(|ui| {
            if ui
                .add_enabled(focused.is_some_and(|i| i > 0), egui::Button::new("↑"))
                .on_hover_text("Move up (Alt+Up)")
                .clicked()
                && let Some(i) = focused
            {
                moved = Some((i, i - 1));
            }
            if ui
                .add_enabled(
                    focused.is_some_and(|i| i + 1 < count),
                    egui::Button::new("↓"),
                )
                .on_hover_text("Move down (Alt+Down)")
                .clicked()
                && let Some(i) = focused
            {
                moved = Some((i, i + 1));
            }
            if ui
                .add_enabled(focused.is_some(), egui::Button::new("Duplicate"))
                .clicked()
            {
                self.duplicate_operation();
            }
            if ui
                .add_enabled(focused.is_some(), egui::Button::new("Delete"))
                .clicked()
            {
                self.delete_operation();
            }
        });
        if let Some((from, to)) = moved {
            self.move_operation(from, to);
        }
    }

    /// Right: the selected operation's fields, or the job settings.
    fn properties_panel(&mut self, ui: &mut egui::Ui) {
        let Some(edit) = self.edit.as_mut() else {
            ui.label("No job open.");
            return;
        };
        match self.focus {
            Focus::Job => job_form(ui, edit),
            Focus::Op(i) => {
                let Some(entry) = edit.ops.get_mut(i) else {
                    return;
                };
                if op_form(ui, i, entry, &self.contours) {
                    self.use_selection();
                }
            }
        }
    }

    fn report_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("Report");
        ScrollArea::both()
            .id_salt("report")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.add(
                    egui::Label::new(RichText::new(self.report.join("\n")).monospace()).extend(),
                );
            });
    }

    fn canvas(&mut self, ui: &mut egui::Ui) {
        let (response, painter) = ui.allocate_painter(ui.available_size(), Sense::click_and_drag());
        let rect = response.rect;
        if self.fit_pending
            && let Some(bounds) = self.scene.bounds()
        {
            self.view.fit(bounds, rect);
            self.fit_pending = false;
        }
        if response.dragged() {
            self.view.pan(response.drag_delta());
        }
        if let Some(pos) = response.hover_pos() {
            let (scroll, zoom) = ui.input(|i| (i.smooth_scroll_delta.y, i.zoom_delta()));
            let factor = f64::from(zoom) * (f64::from(scroll) * 0.0015).exp();
            if (factor - 1.0).abs() > 1e-6 {
                self.view.zoom_at(pos, factor);
            }
            self.cursor = Some(self.view.to_world(pos));
        } else {
            self.cursor = None;
        }
        if response.clicked()
            && let Some(pos) = response.interact_pointer_pos()
        {
            let toggle = ui.input(|i| i.modifiers.command || i.modifiers.ctrl);
            let picked = self.scene.pick(&self.view, pos);
            self.click(picked, toggle);
        }
        if response.secondary_clicked() {
            self.context_pick = response
                .interact_pointer_pos()
                .and_then(|pos| self.scene.pick(&self.view, pos));
        }
        let picked = self
            .context_pick
            .and_then(|id| self.contours.iter().find(|c| c.id == id));
        let layers: Vec<String> = picked.map(Contour::layers).unwrap_or_default();
        let picked_id = picked.map(|c| c.id);
        response.context_menu(|ui| match picked_id {
            Some(id) => {
                if ui.button(format!("Select #{id}")).clicked() {
                    self.click(Some(id), false);
                    ui.close();
                }
                for layer in &layers {
                    if ui.button(format!("Select layer {layer}")).clicked() {
                        self.select_layer(layer);
                        ui.close();
                    }
                }
            }
            None => {
                ui.label("Right-click a contour to select its layer.");
            }
        });
        let selected = self.selection.ids(&self.contours);
        let operation = self.focused_contours();
        self.scene
            .paint(&painter, &self.view, &selected, &operation);
    }

    fn prompts(&mut self, ctx: &egui::Context) {
        let Some(prompt) = self.prompt.clone() else {
            return;
        };
        match prompt {
            Prompt::Units => {
                egui::Window::new("Drawing units")
                    .collapsible(false)
                    .resizable(false)
                    .show(ctx, |ui| {
                        ui.label("This DXF does not say which units it uses.");
                        ui.label("What units was it drawn in?");
                        ui.horizontal(|ui| {
                            if ui.button("mm").clicked() {
                                self.choose_units(Units::Mm);
                            }
                            if ui.button("inch").clicked() {
                                self.choose_units(Units::Inch);
                            }
                            if ui.button("Cancel").clicked() {
                                self.prompt = None;
                                self.error(vec![
                                    "Drawing not opened: its units are unknown.".into(),
                                ]);
                            }
                        });
                    });
            }
            Prompt::Discard(action) => {
                let mut answer: Option<bool> = None;
                egui::Window::new("Unsaved changes")
                    .collapsible(false)
                    .resizable(false)
                    .show(ctx, |ui| {
                        ui.label("The job has unsaved changes. Discard them?");
                        ui.horizontal(|ui| {
                            if ui.button("Discard").clicked() {
                                answer = Some(true);
                            }
                            if ui.button("Cancel").clicked() {
                                answer = Some(false);
                            }
                        });
                    });
                match answer {
                    Some(true) => {
                        self.prompt = None;
                        self.saved.clone_from(&self.edit); // nothing left to protect
                        self.run_pending(action, ctx);
                    }
                    Some(false) => self.prompt = None,
                    None => {}
                }
            }
            Prompt::OpenPath(text) => {
                self.path_prompt(
                    ctx,
                    "Open path",
                    "Path to a .dxf drawing or a .json job:",
                    text,
                );
            }
            Prompt::SaveJobPath(text) => {
                self.path_prompt(ctx, "Save job", "Save the job to:", text);
            }
            Prompt::ExportPath(text) => {
                self.path_prompt(ctx, "Export G-code", "Export the checked G-code to:", text);
            }
        }
    }

    fn path_prompt(&mut self, ctx: &egui::Context, title: &str, label: &str, mut text: String) {
        let mut done = false;
        let mut chosen: Option<PathBuf> = None;
        egui::Window::new(title).collapsible(false).show(ctx, |ui| {
            ui.label(label);
            ui.add(TextEdit::singleline(&mut text).desired_width(460.0));
            ui.horizontal(|ui| {
                if ui.button("OK").clicked() {
                    chosen = Some(PathBuf::from(text.trim()));
                    done = true;
                }
                if ui.button("Cancel").clicked() {
                    done = true;
                }
            });
        });
        let kind = self.prompt.clone();
        self.prompt = if done {
            None
        } else {
            kind.clone().map(|p| match p {
                Prompt::OpenPath(_) => Prompt::OpenPath(text),
                Prompt::SaveJobPath(_) => Prompt::SaveJobPath(text),
                Prompt::ExportPath(_) => Prompt::ExportPath(text),
                other => other,
            })
        };
        let (Some(path), Some(kind)) = (chosen, kind) else {
            return;
        };
        match kind {
            Prompt::OpenPath(_) => {
                let is_job = path
                    .extension()
                    .is_some_and(|e| e.eq_ignore_ascii_case("json"));
                if is_job {
                    if self.guard(Pending::OpenJob(path.clone())) {
                        self.open_job(&path);
                    }
                } else {
                    self.open_dxf(&path);
                }
            }
            Prompt::SaveJobPath(_) => {
                self.save_job(&path);
            }
            Prompt::ExportPath(_) => {
                self.export_gcode(&path);
            }
            _ => {}
        }
    }

    fn update_title(&mut self, ctx: &egui::Context) {
        let mut parts = vec!["Chipload".to_string()];
        if let Some(dxf) = &self.dxf_path {
            parts.push(file_name(dxf));
        }
        if self.edit.is_some() {
            let name = self
                .job_path
                .as_deref()
                .map_or("new job".to_string(), file_name);
            parts.push(if self.dirty() {
                format!("{name} *")
            } else {
                name
            });
        }
        let title = parts.join(" — ");
        if title != self.title {
            ctx.send_viewport_cmd(ViewportCommand::Title(title.clone()));
            self.title = title;
        }
    }
}

fn save_dialog(suggested: &Path, filter: &str, ext: &str) -> Option<PathBuf> {
    let mut dialog = rfd::FileDialog::new().add_filter(filter, &[ext]);
    if let Some(name) = suggested.file_name() {
        dialog = dialog.set_file_name(name.to_string_lossy());
    }
    if let Some(dir) = suggested.parent().filter(|d| !d.as_os_str().is_empty()) {
        dialog = dialog.set_directory(dir);
    }
    dialog.save_file()
}

fn mm(value: &mut f64) -> DragValue<'_> {
    DragValue::new(value)
        .speed(0.1)
        .range(0.0..=100_000.0)
        .suffix(" mm")
}

fn feed(value: &mut f64) -> DragValue<'_> {
    DragValue::new(value)
        .speed(10.0)
        .range(0.0..=100_000.0)
        .suffix(" mm/min")
}

fn direction_combo(ui: &mut egui::Ui, id: (&str, usize), direction: &mut Direction) {
    ComboBox::from_id_salt(id)
        .selected_text(match direction {
            Direction::Climb => "climb",
            Direction::Conventional => "conventional",
        })
        .show_ui(ui, |ui| {
            ui.selectable_value(direction, Direction::Climb, "climb");
            ui.selectable_value(direction, Direction::Conventional, "conventional");
        });
}

/// The selected operation's fields. Returns true if "Use selection" was clicked.
fn op_form(
    ui: &mut egui::Ui,
    index: usize,
    entry: &mut crate::editor::EditOp,
    contours: &[Contour],
) -> bool {
    let mut use_selection = false;
    let summary = geometry_summary(&entry.op, contours);
    let side_chosen = &mut entry.side_chosen;
    ui.heading(format!("Operation {}", index + 1));
    Grid::new(("op-form", index)).num_columns(2).show(ui, |ui| {
        let name = match &mut entry.op {
            Operation::Profile(p) => &mut p.name,
            Operation::Pocket(p) => &mut p.name,
            Operation::Drill(d) => &mut d.name,
        };
        ui.label("Name");
        ui.add(TextEdit::singleline(name).desired_width(200.0));
        ui.end_row();
        ui.label("Geometry");
        ui.horizontal(|ui| {
            ui.label(&summary);
            if ui
                .button("Use selection")
                .on_hover_text("Replace with the contours selected on the canvas")
                .clicked()
            {
                use_selection = true;
            }
        });
        ui.end_row();
        match &mut entry.op {
            Operation::Profile(p) => {
                ui.label("Side");
                let text = if *side_chosen {
                    match p.side {
                        Side::Inside => "inside",
                        Side::Outside => "outside",
                        Side::On => "on",
                    }
                } else {
                    "— choose —"
                };
                ComboBox::from_id_salt(("side", index))
                    .selected_text(text)
                    .show_ui(ui, |ui| {
                        for (side, label) in [
                            (Side::Outside, "outside"),
                            (Side::Inside, "inside"),
                            (Side::On, "on"),
                        ] {
                            if ui
                                .selectable_label(*side_chosen && p.side == side, label)
                                .clicked()
                            {
                                p.side = side;
                                *side_chosen = true;
                            }
                        }
                    });
                ui.end_row();
                ui.label("Direction");
                direction_combo(ui, ("direction", index), &mut p.direction);
                ui.end_row();
                ui.label("Depth");
                ui.add(mm(&mut p.depth));
                ui.end_row();
                ui.label("Step down");
                ui.add(mm(&mut p.step_down));
                ui.end_row();
                ui.label("Feed");
                ui.add(feed(&mut p.feed));
                ui.end_row();
                ui.label("Plunge feed");
                ui.add(feed(&mut p.plunge_feed));
                ui.end_row();
                ui.label("Tabs");
                let mut has_tabs = p.tabs.is_some();
                if ui.checkbox(&mut has_tabs, "holding tabs").changed() {
                    p.tabs = has_tabs.then_some(Tabs {
                        count: 4,
                        width: 8.0,
                        height: 3.0,
                    });
                }
                ui.end_row();
                if let Some(tabs) = &mut p.tabs {
                    ui.label("  count");
                    ui.add(DragValue::new(&mut tabs.count).range(1..=100));
                    ui.end_row();
                    ui.label("  width");
                    ui.add(mm(&mut tabs.width));
                    ui.end_row();
                    ui.label("  height");
                    ui.add(mm(&mut tabs.height));
                    ui.end_row();
                }
            }
            Operation::Pocket(p) => {
                ui.label("Direction");
                direction_combo(ui, ("direction", index), &mut p.direction);
                ui.end_row();
                ui.label("Depth");
                ui.add(mm(&mut p.depth));
                ui.end_row();
                ui.label("Step down");
                ui.add(mm(&mut p.step_down));
                ui.end_row();
                ui.label("Stepover");
                ui.add(
                    DragValue::new(&mut p.stepover)
                        .speed(0.01)
                        .range(0.01..=1.0)
                        .suffix(" × D"),
                );
                ui.end_row();
                ui.label("Feed");
                ui.add(feed(&mut p.feed));
                ui.end_row();
                ui.label("Plunge feed");
                ui.add(feed(&mut p.plunge_feed));
                ui.end_row();
                ui.label("Links");
                ui.checkbox(&mut p.keep_tool_down, "keep the tool down where safe");
                ui.end_row();
            }
            Operation::Drill(_) => {
                ui.label("Drilling is not implemented yet.");
                ui.end_row();
            }
        }
    });
    use_selection
}

/// Stock, machine and tool.
fn job_form(ui: &mut egui::Ui, edit: &mut EditJob) {
    let job = &mut edit.settings;
    ui.heading("Job settings");
    Grid::new("job-form").num_columns(2).show(ui, |ui| {
        ui.label("Drawing");
        ui.label(format!(
            "{} ({})",
            job.source.file,
            match job.source.units {
                Units::Mm => "mm",
                Units::Inch => "inch",
            }
        ));
        ui.end_row();
        ui.label("Locked");
        ui.label(match &job.source.sha256 {
            Some(sha) => format!("to sha256 {}…", &sha[..sha.len().min(12)]),
            None => "no (Job > Re-lock to drawing)".into(),
        });
        ui.end_row();

        ui.strong("Stock");
        ui.end_row();
        ui.label("Size X × Y");
        ui.horizontal(|ui| {
            ui.add(mm(&mut job.stock.x));
            ui.add(mm(&mut job.stock.y));
        });
        ui.end_row();
        ui.label("Thickness");
        ui.add(mm(&mut job.stock.thickness));
        ui.end_row();
        ui.label("Origin");
        ComboBox::from_id_salt("origin")
            .selected_text(match job.stock.origin {
                StockOrigin::BottomLeft => "bottom left",
                StockOrigin::Center => "center",
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(
                    &mut job.stock.origin,
                    StockOrigin::BottomLeft,
                    "bottom left",
                );
                ui.selectable_value(&mut job.stock.origin, StockOrigin::Center, "center");
            });
        ui.end_row();
        ui.label("Z zero");
        ComboBox::from_id_salt("z_zero")
            .selected_text(match job.stock.z_zero {
                ZZero::Top => "stock top",
                ZZero::Bottom => "stock bottom",
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut job.stock.z_zero, ZZero::Top, "stock top");
                ui.selectable_value(&mut job.stock.z_zero, ZZero::Bottom, "stock bottom");
            });
        ui.end_row();

        ui.strong("Machine");
        ui.end_row();
        ui.label("Post");
        ComboBox::from_id_salt("post")
            .selected_text(job.machine.post.clone())
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut job.machine.post, "grbl".to_string(), "grbl");
            });
        ui.end_row();
        ui.label("Safe Z");
        ui.add(
            DragValue::new(&mut job.machine.safe_z)
                .speed(0.1)
                .suffix(" mm"),
        );
        ui.end_row();
        ui.label("Spindle");
        ComboBox::from_id_salt("spindle")
            .selected_text(match job.machine.spindle {
                Spindle::Cw => "cw (M3)",
                Spindle::Ccw => "ccw (M4)",
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut job.machine.spindle, Spindle::Cw, "cw (M3)");
                ui.selectable_value(&mut job.machine.spindle, Spindle::Ccw, "ccw (M4)");
            });
        ui.end_row();
        ui.label("Max feed");
        ui.horizontal(|ui| {
            let mut set = job.machine.max_feed.is_some();
            if ui.checkbox(&mut set, "").changed() {
                job.machine.max_feed = set.then_some(5000.0);
            }
            if let Some(max) = &mut job.machine.max_feed {
                ui.add(feed(max));
            }
        });
        ui.end_row();
        ui.label("Travel X Y Z");
        ui.horizontal(|ui| {
            let mut set = job.machine.travel.is_some();
            if ui.checkbox(&mut set, "").changed() {
                job.machine.travel = set.then_some(Travel {
                    x: 800.0,
                    y: 600.0,
                    z: 100.0,
                });
            }
            if let Some(t) = &mut job.machine.travel {
                ui.add(mm(&mut t.x));
                ui.add(mm(&mut t.y));
                ui.add(mm(&mut t.z));
            }
        });
        ui.end_row();

        if let Some(tool) = job.tools.first_mut() {
            ui.strong(format!("Tool {}", tool.id));
            ui.end_row();
            ui.label("Type");
            ComboBox::from_id_salt("tool-type")
                .selected_text(match tool.kind {
                    ToolType::Endmill => "endmill",
                    ToolType::Drill => "drill",
                    ToolType::Vbit => "v-bit",
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut tool.kind, ToolType::Endmill, "endmill");
                    ui.selectable_value(&mut tool.kind, ToolType::Drill, "drill");
                    ui.selectable_value(&mut tool.kind, ToolType::Vbit, "v-bit");
                });
            ui.end_row();
            ui.label("Diameter");
            ui.add(mm(&mut tool.diameter));
            ui.end_row();
            ui.label("Spindle speed");
            let mut rpm = tool.rpm.unwrap_or(0.0);
            if ui
                .add(
                    DragValue::new(&mut rpm)
                        .speed(100.0)
                        .range(0.0..=100_000.0)
                        .suffix(" rpm"),
                )
                .changed()
            {
                tool.rpm = (rpm > 0.0).then_some(rpm);
            }
            ui.end_row();
            ui.label("Centre cutting");
            ComboBox::from_id_salt("center-cutting")
                .selected_text(match tool.center_cutting {
                    Some(true) => "yes",
                    Some(false) => "no",
                    None => "unknown",
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut tool.center_cutting, Some(true), "yes");
                    ui.selectable_value(&mut tool.center_cutting, Some(false), "no");
                    ui.selectable_value(&mut tool.center_cutting, None, "unknown");
                });
            ui.end_row();
        }
    });
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
        // One undo step per finished edit: not while dragging a value or typing.
        let busy = ctx.egui_is_using_pointer() || ctx.egui_wants_keyboard_input();
        if !busy {
            self.commit();
        }
        self.update_title(&ctx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chipload_toolpath::model::MoveKind;

    fn corpus(rel: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/corpus")
            .join(rel)
    }

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("chipload-gui-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn select(app: &mut ChiploadApp, ids: &[usize]) {
        app.selection = CanvasSelection::Contours(ids.iter().copied().collect());
    }

    /// messy_2 with a new job: a pocket on the crossing bars, then the lower outline.
    fn lower_part() -> ChiploadApp {
        let mut app = ChiploadApp::default();
        assert!(app.open_dxf(&corpus("dxf/messy_2.dxf")));
        assert!(app.new_job());
        select(&mut app, &[4, 6]);
        app.add_operation(OpKind::Pocket).unwrap();
        select(&mut app, &[2]);
        app.add_operation(OpKind::Profile).unwrap();
        app
    }

    fn first_cut(app: &ChiploadApp) -> (f64, f64) {
        let moves = &app.result.as_ref().unwrap().program.moves;
        let m = moves
            .iter()
            .find(|m| m.kind == MoveKind::Rapid && m.x.is_some())
            .unwrap();
        (m.x.unwrap(), m.y.unwrap())
    }

    #[test]
    fn create_operations_generate_save_and_run_from_the_cli_side() {
        let mut app = lower_part();
        assert!(app.dirty(), "a new job is unsaved");
        assert!(app.generate(), "{:?}", app.report);
        let dir = temp_dir("save");
        let job_path = dir.join("lower.json");
        let gcode_path = dir.join("lower.nc");
        assert!(app.save_job(&job_path));
        assert!(!app.dirty());
        assert!(app.export_gcode(&gcode_path));
        // The saved job is an ordinary job: run_job on it gives the exported G-code.
        let job = load_job(&job_path).unwrap();
        let again = run_job(&job, &corpus("dxf/messy_2.dxf")).unwrap();
        assert_eq!(std::fs::read_to_string(&gcode_path).unwrap(), again.gcode);
        // Handle selectors: the job was locked to the drawing.
        assert!(job.source.sha256.is_some());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn reordering_changes_the_cut_order() {
        let mut app = lower_part();
        assert!(app.generate());
        let pocket_first = first_cut(&app);
        app.move_operation(1, 0);
        assert!(app.result.is_none(), "editing clears the toolpath until F5");
        assert!(app.generate());
        let profile_first = first_cut(&app);
        assert_ne!(pocket_first, profile_first);
        // The outline profile starts on the outline, far from the bars in the middle.
        assert!(profile_first.1.abs() > 40.0, "{profile_first:?}");
    }

    #[test]
    fn disabled_operations_are_not_cut() {
        let mut app = lower_part();
        assert!(app.generate());
        let all = app.result.as_ref().unwrap().program.moves.len();
        if let Operation::Pocket(p) = &mut app.edit.as_mut().unwrap().ops[0].op {
            p.enabled = false;
        }
        app.commit();
        assert!(app.generate());
        assert!(app.result.as_ref().unwrap().program.moves.len() < all);
    }

    #[test]
    fn undo_and_redo_restore_earlier_jobs() {
        let mut app = lower_part();
        assert_eq!(app.edit.as_ref().unwrap().ops.len(), 2);
        app.undo();
        assert_eq!(app.edit.as_ref().unwrap().ops.len(), 1);
        app.undo();
        assert_eq!(app.edit.as_ref().unwrap().ops.len(), 0);
        app.redo();
        app.redo();
        assert_eq!(app.edit.as_ref().unwrap().ops.len(), 2);
        // A field edit is one step.
        if let Operation::Profile(p) = &mut app.edit.as_mut().unwrap().ops[1].op {
            p.depth = 5.0;
        }
        app.commit();
        app.undo();
        let Operation::Profile(p) = &app.edit.as_ref().unwrap().ops[1].op else {
            panic!("expected the profile");
        };
        assert_eq!(p.depth, 1.0);
    }

    #[test]
    fn incomplete_jobs_say_what_is_missing() {
        let mut app = ChiploadApp::default();
        app.open_dxf(&corpus("dxf/messy_2.dxf"));
        app.new_job();
        select(&mut app, &[1, 2]); // a hole and an outer: side must be chosen
        app.add_operation(OpKind::Profile).unwrap();
        assert!(!app.generate());
        assert!(
            app.report
                .iter()
                .any(|l| l.contains("choose the profile side"))
        );
        assert_eq!(app.error_ops, BTreeSet::from([0]));
        let dir = temp_dir("incomplete");
        assert!(!app.save_job(&dir.join("job.json")));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn layer_selection_becomes_a_layer_selector() {
        let mut app = ChiploadApp::default();
        app.open_dxf(&corpus("dxf/messy_2.dxf"));
        app.new_job();
        app.select_layer("lower");
        assert_eq!(
            app.selector_json().as_deref(),
            Some("{\"layer\": \"lower\"}")
        );
        app.add_operation(OpKind::Pocket).unwrap();
        assert!(!app.edit.as_ref().unwrap().uses_handles());
        assert!(
            app.selection_lines()[1].contains("#7"),
            "the open centreline is named"
        );
    }

    #[test]
    fn a_job_opens_its_drawing_and_a_changed_drawing_is_refused() {
        let dir = temp_dir("lock");
        let dxf = dir.join("messy_2.dxf");
        std::fs::copy(corpus("dxf/messy_2.dxf"), &dxf).unwrap();
        let mut app = ChiploadApp::default();
        app.open_dxf(&dxf);
        app.new_job();
        select(&mut app, &[4]);
        app.add_operation(OpKind::Pocket).unwrap();
        let job_path = dir.join("job.json");
        assert!(app.save_job(&job_path));

        let mut fresh = ChiploadApp::default();
        assert!(fresh.open_job(&job_path));
        assert_eq!(
            fresh.dxf_path.as_deref(),
            Some(dxf.as_path()),
            "opened next to the job"
        );
        assert!(fresh.generate());

        // Change the drawing: the lock refuses it until re-locked.
        let mut text = std::fs::read_to_string(&dxf).unwrap();
        text.push_str("\n999\nchanged\n");
        std::fs::write(&dxf, text).unwrap();
        assert!(!fresh.generate());
        assert!(fresh.report.iter().any(|l| l.contains("Re-lock")));
        fresh.relock();
        assert!(fresh.generate(), "{:?}", fresh.report);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn old_job_versions_are_saved_as_current() {
        let mut app = ChiploadApp::default();
        app.open_dxf(&corpus("dxf/simple_part.dxf"));
        assert!(app.open_job(&corpus("jobs/v1/example_job.json")));
        assert!(
            app.report[0].contains("schema version 1"),
            "{:?}",
            app.report
        );
        let dir = temp_dir("old");
        let path = dir.join("job.json");
        assert!(app.save_job(&path));
        assert_eq!(
            raw_schema_version(&path),
            Some(chipload_job::CURRENT_VERSION)
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn unitless_drawing_asks_for_units() {
        let mut app = ChiploadApp::default();
        assert!(!app.open_dxf(&corpus("dxf/messy.dxf")));
        assert_eq!(app.prompt, Some(Prompt::Units));
        assert!(app.choose_units(Units::Mm));
        assert!(!app.contours.is_empty());
        assert!(app.new_job());
        assert_eq!(app.edit.as_ref().unwrap().settings.source.units, Units::Mm);
    }

    #[test]
    fn clicks_select_and_toggle() {
        let mut app = ChiploadApp::default();
        app.open_dxf(&corpus("dxf/messy_2.dxf"));
        assert_eq!(app.selector_json(), None);
        app.click(Some(4), false);
        app.click(Some(6), true);
        assert_eq!(app.selection.ids(&app.contours), BTreeSet::from([4, 6]));
        app.click(Some(6), true);
        assert_eq!(app.selection.ids(&app.contours), BTreeSet::from([4]));
        app.click(None, false);
        assert_eq!(app.selection, CanvasSelection::Nothing);
    }

    #[test]
    fn operation_numbers_are_read_from_messages() {
        assert_eq!(operation_index("Operation 3 (Outline): bad"), Some(2));
        assert_eq!(operation_index("Operation 1: x"), Some(0));
        assert_eq!(operation_index("no operation"), None);
    }
}
