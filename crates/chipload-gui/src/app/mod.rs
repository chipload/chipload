// SPDX-License-Identifier: GPL-3.0-or-later
//! Main window: import a drawing into a project, edit its operations (from selected geometry,
//! in any order), generate on F5, save the project, export checked G-code.
//! See docs/specs/job-editor.md and docs/specs/project-file.md. Thin by design: geometry and
//! G-code decisions are calls into the pipeline; this file only edits a `Job` (a project)
//! and shows results.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use chipload_core::Point;
use chipload_core::chain::Contour;
use chipload_dxf::{ImportError, shapes_from_file};
use chipload_job::edit::{Handle, SegmentDrag, handles, try_drag};
use chipload_job::library::Theme;
use chipload_job::{
    Direction, Drawing, Geometry, Job, Operation, Selector, Side, Source, Spindle, StockCorner,
    StockOrigin, Tabs, ToolType, Travel, Units, WorkZero, ZZero, load_job, save_job,
};
use chipload_pipeline::shapes::{add_shape, delete_shape, set_shape};
use chipload_pipeline::sheet::{SheetInfo, write_setup_sheet};
use chipload_pipeline::{
    PipelineError, RunResult, empty_project, load_drawing, new_project, write_gcode_files,
};
use chipload_toolpath::select::{select_contours, select_paths};
use eframe::egui::{
    self, Align, CentralPanel, ComboBox, Grid, Key, Layout, Modifiers, Panel, RichText, ScrollArea,
    Sense, TextEdit, ViewportCommand,
};

use crate::draw::{
    Cursor, Sketch, SnapPoint, Snapper, Step, Tool, grid_step, snap_cursor, snapper,
};
use crate::editor::{CanvasSelection, EditJob, OpKind, geometry_summary, shapes_of, with_twins};
use crate::i18n::{Lang, t};
use crate::shape_form::{ShapeAction, ShapeDraft, imported_lines, shape_form};
use crate::view::{ContourShape, Highlight, Scene, StartMarker, ToolpathShape, View};
use chipload_i18n::Message;

mod box_drag;
mod browser;
mod canvas;
mod canvas_tools;
use canvas_tools::ActiveTool;
mod computer;
mod copy_paste;
mod corners;
mod dialogs;
mod disk;
mod display;
mod drawing;
mod dropping;
mod dxf_export;
mod forms;
mod frames;
mod generation;
mod groups;
pub(crate) mod header;
mod history;
mod images;
mod inlay;
mod layout;
mod library_transfer;
mod library_view;
mod materials;
mod measure;
mod menu;
mod models;
mod modify;
mod navigation;
mod panels;
mod playback;
mod prefs;
mod problems;
mod recovery;
mod relief_wizard;
mod report;
mod rulers;
mod select_drag;
mod setups;
mod shape_library;
mod shortcuts;
mod sign;
mod simulation;
mod sources;
mod styles;
mod tab_markers;
#[cfg(test)]
mod tests;
mod text_fonts;
mod texts;
mod turn_cut;
mod two_sided;
mod update_notice;
mod user_library;
mod wizard_tools;
mod zero_drag;

use forms::{job_form, op_form};
use report::{DrawingFacts, Report, Tone};
pub use user_library::{LibraryKind, SaveTarget};

const STATUS_SECONDS: u64 = 5;
const ERROR_STATUS_SECONDS: u64 = 10;
/// Extension of project files (docs/specs/project-file.md, D1).
pub const PROJECT_EXTENSION: &str = "chipload";

/// What a pocket's *Add rest pass ▸* (docs/specs/rest-passes.md §6) or a V-carve's *Add
/// floor pass ▸* (docs/specs/vcarve-wide-strokes.md §7) offers.
#[derive(Debug, Clone, PartialEq)]
pub enum RestMenu {
    /// Endmills that fit: the project's (id, diameter mm), then the library's not yet in the
    /// project (name, diameter mm).
    Tools {
        project: Vec<(i64, f64)>,
        library: Vec<(String, f64)>,
    },
    /// Shown greyed out, with why.
    Unavailable(String),
}

/// What the properties panel shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Job,
    /// The active setup's board: its stock and material, an item beside its work zero
    /// (the user, 2026-11-03).
    Stock,
    /// The G-code's zero, selected in the Browser or the view
    /// (docs/specs/work-zero-marker.md).
    WorkZero,
    /// What is picked on the canvas, alone (the user, 2026-10-28: one thing at a time in
    /// Properties): its values, or how to pick something.
    Selection,
    Op(usize),
    /// Pocket `0`'s rest pass `1` (docs/specs/rest-passes.md §6).
    RestPass(usize, usize),
    /// V-carve `0`'s floor pass (docs/specs/vcarve-wide-strokes.md §7).
    FloorPass(usize),
    /// Relief `0`'s roughing pass (docs/specs/stl-relief.md §14).
    RoughingPass(usize),
    /// Relief `0`'s smoothing pass (docs/specs/stl-relief.md §17).
    SmoothingPass(usize),
    /// A part, by id (docs/specs/parts.md §8).
    Part(u64),
    /// A setup, by id (docs/specs/setups.md §7).
    Setup(u64),
    /// The project's tools; one tool, by id; an imported DXF; a picture
    /// (docs/specs/properties-for-tools-and-dxfs.md).
    Tools,
    Tool(i64),
    Dxf(u64),
    Image(u64),
    /// A 3D model, by id (docs/specs/stl-relief.md §9).
    Model(u64),
    /// A text, by id (docs/specs/text.md §5).
    Text(u64),
    /// The user's library in the Browser (docs/specs/library-in-browser.md §4): its tools
    /// folder, one tool by uid (as a number), its machines folder, one machine profile.
    LibraryTools,
    LibraryTool(u64),
    LibraryMachines,
    LibraryMachine(u64),
    /// The user's materials, and one by its uid as a number (docs/specs/materials.md §4).
    LibraryMaterials,
    LibraryMaterial(u64),
    /// The user's styles, and one by its uid as a number (docs/specs/styles.md §3.3).
    LibraryStyles,
    LibraryStyle(u64),
    /// The app's preferences, the post-processors, and one post by its place in the list
    /// (docs/specs/this-computer.md).
    Preferences,
    Posts,
    Post(u32),
}

impl Focus {
    /// Whether it shows something of the user's library, which needs no project.
    pub fn is_library(self) -> bool {
        matches!(
            self,
            Focus::LibraryTools
                | Focus::LibraryTool(_)
                | Focus::LibraryMachines
                | Focus::LibraryMachine(_)
                | Focus::LibraryMaterials
                | Focus::LibraryMaterial(_)
                | Focus::LibraryStyles
                | Focus::LibraryStyle(_)
                | Focus::Preferences
                | Focus::Posts
                | Focus::Post(_)
        )
    }
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
    /// An STL model for the project (File > Add 3D model from path…).
    Model,
    /// A new version of the project's DXF with this id (the Browser's Update…).
    UpdateDxf(u64),
    /// The drawing as a DXF (File > Export drawing as DXF).
    ExportDxf,
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
    /// File > Export drawing as DXF: its choices (docs/specs/dxf-export.md §A6).
    ExportDxf,
    /// The SVG gives no real size: its pixels per inch, as typed so far
    /// (docs/specs/svg-import.md §7).
    PxPerInch(PathBuf, UnitsFor, String),
    Path(PathPurpose, String),
    Discard(Pending),
    /// A name for the project's machine or tool in the user's library.
    LibraryName(SaveTarget, String),
    /// *Save as style…*: a name for operation `0` as a style (docs/specs/styles.md §3.1).
    StyleName(usize, String),
    /// The project's file changed on disk while there are unsaved changes
    /// (docs/specs/reload-changed-file.md §4); the file's version is in `disk`.
    ChangedOnDisk,
    /// Explode a DXF, picture or text into drawn shapes, then (a DXF) join ends closer than
    /// `gap` mm if `join` (docs/specs/explode-and-join.md §5,
    /// explode-pictures-and-texts.md §6).
    Explode {
        of: Source,
        join: bool,
        gap: f64,
    },
    /// Join the drawn shapes `ids` (none: every drawn shape) whose ends are closer than
    /// `gap` mm.
    Join {
        ids: Vec<u64>,
        gap: f64,
    },
    /// *Inlay…*: the inlay's values so far (docs/specs/vcarve-inlay.md §4).
    Inlay(inlay::InlaySettings),
    /// *New sign…*: the sign's values so far (docs/specs/sign-maker.md §3).
    Sign(Box<sign::SignSettings>),
    /// *Two-sided…* (docs/specs/two-sided-wizard.md §4).
    TwoSided(Box<two_sided::TwoSidedSettings>),
    /// *Turn and cut…*: a model turned and cut while seen (docs/specs/relief-wizard.md §4).
    TurnCut(Box<turn_cut::TurnCut>),
    /// *Relief…*: a relief in one step, its answers so far (docs/specs/relief-wizard.md §6).
    Relief(Box<relief_wizard::ReliefWizard>),
}

/// A handle drag in progress: the shape as it was when the drag began, which every frame
/// of the drag starts from (chipload_job::edit::drag).
#[derive(Debug, Clone)]
struct HandleDrag {
    handle: Handle,
    layer: String,
    start: Geometry,
    /// From the point grabbed to the handle, mm: zero on a handle, the way to the
    /// segment's middle when a polyline's line is grabbed elsewhere.
    grab: Point,
}

/// The material-removal simulation of the last result, and what playback made of it.
#[derive(Default)]
pub struct Sim {
    run: Option<simulation::SimRun>,
    /// The last simulation, and its image's texture (kept alive while shown).
    pub simulation: Option<chipload_pipeline::simulate::Simulation>,
    texture: Option<egui::TextureHandle>,
    /// The texture of the stock part way through playback (docs/specs/playback.md).
    playback_texture: Option<egui::TextureHandle>,
    /// The simulation's layer at the end of the program, shown when playback is there.
    full_layer: Option<crate::view::SimLayer>,
    /// Every display cell's history, for playback; None: too long, or no simulation.
    history: Option<chipload_pipeline::simulate::History>,
    /// Counts the results shown: a simulation is for one of them.
    serial: u64,
}

/// What the pointer is dragging on the canvas, and where it snaps.
#[derive(Default)]
struct Drag {
    /// The segments the cursor snaps to (docs/specs/drawing-input.md §1).
    snapper: Snapper,
    /// Where the cursor snapped this frame, while drawing or dragging a handle.
    snapped: Option<(Point, Option<SnapPoint>)>,
    /// The handle being dragged, if any.
    handle: Option<HandleDrag>,
    /// Drawn shapes being moved with the mouse (docs/specs/drawing-input.md §3).
    moving: Option<select_drag::MoveDrag>,
    /// A selection box being dragged: where it started and its far corner.
    select_box: Option<(Point, Point)>,
    /// The picture's or text's box handle being dragged, if any.
    boxed: Option<box_drag::BoxDrag>,
    /// The work zero being dragged (docs/specs/work-zero-marker.md §3).
    zero: Option<zero_drag::ZeroDrag>,
    /// The start marker being dragged (index in `scene.start_markers`).
    start: Option<usize>,
    /// The start marker right-clicked, for its context menu.
    context_start: Option<usize>,
    /// The tab marker being dragged (index in `scene.tab_markers`).
    tab: Option<usize>,
    /// The tab marker right-clicked, for its context menu.
    context_tab: Option<usize>,
    /// The focused profile's ring right-clicked (its index and the point), to add a tab.
    context_ring: Option<(usize, Point)>,
}

/// The small windows over the app.
#[derive(Default)]
struct Dialogs {
    prompt: Option<Prompt>,
    about_open: bool,
    /// Help → Keyboard shortcuts is open.
    shortcuts_open: bool,
}

pub struct ChiploadApp {
    /// The project being edited.
    pub edit: Option<EditJob>,
    /// The project's file: where it is, what was last saved there, and changes other
    /// programs make to it.
    pub file: disk::ProjectFile,
    /// Undo and redo.
    pub history: history::History,
    pub focus: Focus,

    /// The project's contours (from its embedded drawing), and the drawing they came from.
    pub contours: Vec<Contour>,
    /// Shapes left out as duplicates -> the shapes they repeat (`ImportResult::twins`).
    twins: BTreeMap<u64, BTreeSet<u64>>,
    loaded_drawing: Option<Drawing>,

    /// The toolpath: the last result, whether it is out of date, the run in progress and what waits for it.
    pub toolpath: generation::Toolpath,
    /// Operations (indices) the last generation, validation or update complained about.
    pub error_ops: BTreeSet<usize>,
    /// What is wrong with the project as last edited (docs/specs/live-problems.md).
    pub problems: Vec<chipload_job::Problem>,

    // The canvas.
    pub scene: Scene,
    view: View,
    /// What the view is meant to show, to show again when the canvas changes size.
    view_kept: Option<canvas::KeptView>,
    /// The shapes picked first, which Align keeps in place.
    align_first: modify::FirstPicked,
    fit_pending: bool,
    /// A new project: its thickness field takes the keyboard when the setup is drawn.
    focus_thickness: bool,
    /// What the pending fit shows instead of everything (a screenshot's close-up).
    fit_only: Option<Vec<crate::view::Point3>>,
    pub selection: CanvasSelection,
    /// The focused operation's corners and an open corner menu (docs/specs/dogbone-per-corner.md).
    corners: corners::CornerMarkers,
    context_pick: Option<usize>,
    /// The panels round the canvas: which are open, which tab the right one shows.
    pub panels: layout::Panels,
    shape_library: shape_library::ShapeLibraryView,
    /// The material-removal simulation of the last result (docs/specs/simulation.md).
    pub sim: Sim,
    /// The playback slider's state, once a toolpath has been generated.
    pub playback: Option<playback::Playback>,
    /// What the canvas shows: 3D or top view, Simulate or Edit, pictures, toolpath, parts, legend.
    pub display: display::Display,
    /// The canvas tools: the one in use, a value being typed, previews, and the values the
    /// tools keep for the session.
    pub canvas_tools: canvas_tools::CanvasTools,
    /// File > Export drawing as DXF's choices, kept for the session.
    pub dxf_export: dxf_export::DxfExportChoice,
    /// A text was just placed: its text field takes the keyboard once.
    pub(super) focus_text_field: bool,
    /// The drawn shapes copied last, for Ctrl+V.
    pub clipboard: copy_paste::Clipboard,
    /// The selected drawn shape's values as being edited.
    shape_draft: Option<ShapeDraft>,
    /// The shape's name field takes the keyboard next frame (Browser: Rename…).
    grab_shape_name: bool,
    /// What the pointer is dragging on the canvas, and where it snaps.
    drag: Drag,
    /// The selected picture's or text's box size, worked out once (box handles).
    box_size: box_drag::BoxSize,

    pub report: Report,
    /// The current drawing's facts, for the report's Drawing section.
    pub drawing_facts: Option<DrawingFacts>,
    /// The preferences kept in the app settings, and the language and theme they resolve to.
    pub prefs: prefs::Prefs,
    /// Recovery copies: this window's, when it is next written, and those other windows left.
    pub recovery: recovery::Recovery,
    /// The update notice: the user's answer, the daily check, a newer release
    /// (docs/specs/update-notice.md).
    pub updates: crate::updates::Updates,
    /// The computer's fonts for texts (docs/specs/system-fonts.md §6).
    pub text_fonts: text_fonts::TextFonts,
    status: Option<(String, Instant)>,
    cursor: Option<Point>,
    dialogs: Dialogs,
    title: String,
    allow_close: bool,

    /// The user's settings folder and the tool library and machine profiles read from it.
    pub user: user_library::UserFiles,
    /// The image window, while open (docs/specs/image-trace.md §6).
    pub image_dialog: Option<images::ImageDialog>,
    /// Dropped files still to bring in, one at a time (docs/specs/drawing-sources.md §3).
    pub(super) drops: std::collections::VecDeque<PathBuf>,
    /// Contours not drawn because their group or layer is hidden (drawing-sources.md §4).
    pub(super) hidden_contours: BTreeSet<usize>,
    /// View > Show drawing off: the selection when it was switched off. Selecting something
    /// else or drawing a shape shows the drawing again (docs/specs/view-and-settings.md §2).
    pub(super) drawing_hidden: Option<CanvasSelection>,
    /// The pictures' textures, for the images they were made from.
    picture_textures: images::PictureTextures,
    /// The models' faint surfaces in 3D, and what they were made from.
    model_surfaces: models::ModelSurfaces,
    /// An STL file being read on a worker thread, to be added when read
    /// (docs/specs/relief-wizard.md §2).
    model_reading: Option<models::ModelReading>,
}

impl Default for ChiploadApp {
    fn default() -> Self {
        ChiploadApp {
            edit: None,
            file: disk::ProjectFile::default(),
            history: history::History::default(),
            focus: Focus::Job,
            contours: Vec::new(),
            twins: BTreeMap::new(),
            loaded_drawing: None,
            toolpath: generation::Toolpath::default(),
            error_ops: BTreeSet::new(),
            problems: Vec::new(),
            scene: Scene::default(),
            view: View::default(),
            view_kept: None,
            align_first: modify::FirstPicked::default(),
            fit_pending: false,
            focus_thickness: false,
            fit_only: None,
            selection: CanvasSelection::Nothing,
            corners: corners::CornerMarkers::default(),
            context_pick: None,
            // On at start: it is how the canvas shows what the tools leave
            // (docs/specs/material-left-merge.md, decision 2).
            panels: layout::Panels::default(),
            shape_library: shape_library::ShapeLibraryView::default(),
            sim: Sim::default(),
            playback: None,
            display: display::Display::default(),
            canvas_tools: canvas_tools::CanvasTools::default(),
            dxf_export: dxf_export::DxfExportChoice::default(),
            focus_text_field: false,
            clipboard: copy_paste::Clipboard::default(),
            shape_draft: None,
            grab_shape_name: false,
            drag: Drag::default(),
            box_size: box_drag::BoxSize::default(),
            report: Report::info([t!(crate::i18n::Language::En, "app-start-hint")]),
            drawing_facts: None,
            prefs: prefs::Prefs::default(),
            updates: crate::updates::Updates::default(),
            text_fonts: text_fonts::TextFonts::default(),
            recovery: recovery::Recovery::default(),
            status: None,
            cursor: None,
            dialogs: Dialogs::default(),
            title: String::new(),
            allow_close: false,
            user: user_library::UserFiles::default(),
            image_dialog: None,
            drops: std::collections::VecDeque::new(),
            hidden_contours: BTreeSet::new(),
            drawing_hidden: None,
            picture_textures: images::PictureTextures::default(),
            model_surfaces: models::ModelSurfaces::default(),
            model_reading: None,
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

fn selector_text(selector: &Selector) -> String {
    let list = |ids: &[u64]| {
        let ids: Vec<String> = ids.iter().map(u64::to_string).collect();
        format!("[{}]", ids.join(", "))
    };
    let mut fields = Vec::new();
    if let Some(layer) = &selector.layer {
        // A JSON string, quotes and backslashes escaped.
        let text = serde_json::to_string(layer).unwrap_or_default();
        fields.push(format!("\"layer\": {text}"));
    }
    if let Some(shapes) = selector.shapes.as_ref().filter(|s| !s.is_empty()) {
        fields.push(format!("\"shapes\": {}", list(shapes)));
    }
    if let Some(source) = selector.source {
        fields.push(format!("\"{}\": {}", source.key(), source.id()));
    }
    if !selector.exclude.is_empty() {
        fields.push(format!("\"exclude\": {}", list(&selector.exclude)));
    }
    format!("{{{}}}", fields.join(", "))
}

/// The extensions of the drawing files the file dialogs offer (docs/specs/svg-import.md §10).
pub(super) const DRAWING_FILES: &[&str] = &["dxf", "svg", "svgz"];

/// What File > Import… and Open… take besides projects: drawings and STL models.
pub(super) const IMPORT_FILES: &[&str] = &["dxf", "svg", "svgz", "stl", "STL"];

/// A DXF or an SVG, by its extension.
fn is_drawing_file(path: &Path) -> bool {
    path.extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("dxf"))
        || chipload_dxf::is_svg(path)
}

impl ChiploadApp {
    // -- state

    /// The project has changes that aren't saved.
    pub fn dirty(&self) -> bool {
        self.edit.is_some() && self.edit != self.file.saved
    }

    /// A new project opens on its stock, where to start (the user, 2026-11-03: "When
    /// starting a new project we should open the stock setup first").
    fn open_stock(&mut self) {
        if self.edit.is_some() {
            self.focus = Focus::Stock;
            self.focus_thickness = true;
        }
    }

    /// Start editing `job` as a fresh document (no undo history).
    fn start_editing(&mut self, job: EditJob, saved: bool, path: Option<PathBuf>) {
        // The previous project was saved or its changes deliberately dropped.
        self.clear_recovery();
        self.file.saved = saved.then(|| job.clone());
        self.history.committed = Some(job.clone());
        self.edit = Some(job);
        self.file.path = path;
        self.remember_disk();
        self.history.undo.clear();
        self.history.redo.clear();
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
        let Some(job) = self.edit.as_ref().map(|e| e.job.clone()) else {
            self.drawing_facts = None;
            return;
        };
        if self.loaded_drawing.as_ref() == Some(&job.drawing) {
            return;
        }
        self.loaded_drawing = Some(job.drawing.clone());
        self.corners.forget();
        match load_drawing(&job) {
            Ok(imported) => {
                self.contours = imported.contours;
                self.twins = imported.twins;
                // Hidden groups and layers aren't drawn, so they can't be clicked either.
                self.hidden_contours = sources::hidden_contours(&job.drawing, &self.contours);
                self.scene.contours = self
                    .contours
                    .iter()
                    .filter(|c| !self.hidden_contours.contains(&c.id))
                    .map(ContourShape::new)
                    .collect();
                self.drag.snapper = snapper(&self.contours);
                self.drawing_facts = Some(DrawingFacts::new(&imported.report, &self.contours));
            }
            Err(err) => {
                self.contours.clear();
                self.twins.clear();
                self.hidden_contours.clear();
                self.scene.contours.clear();
                self.drag.snapper = Snapper::default();
                self.drawing_facts = Some(DrawingFacts::failed(err.message()));
            }
        }
        self.update_ghosts();
    }

    /// The active setup's board, and the other setups' boards and contours as ghosts
    /// (docs/specs/setups.md §5), from the edited project with its operations: a board
    /// that fits fits its own setup's parts.
    fn update_ghosts(&mut self) {
        let job = self.edited_job();
        self.scene.stock = job.as_ref().map(chipload_job::Job::stock_bounds);
        self.scene.ghost_boards = job.as_ref().map(ghost_boards).unwrap_or_default();
        self.scene.ghost_contours = job
            .map(|job| ghost_contours(&job, &self.contours))
            .unwrap_or_default();
    }

    // -- operations (also called directly by tests)

    /// Open a project, an older job, or a DXF or SVG (imported as a new project).
    pub fn open(&mut self, path: &Path) -> bool {
        if is_drawing_file(path) || dropping::dropped_kind(path) == Some(dropping::Dropped::Model) {
            self.import_any(path)
        } else {
            self.open_project(path)
        }
    }

    pub fn open_project(&mut self, path: &Path) -> bool {
        let job = match load_job(path) {
            Ok(job) => job,
            Err(err) => {
                self.error_report(Report::failed(
                    t!(self, "app-cannot-open", file = file_name(path)),
                    err.message(),
                ));
                return false;
            }
        };
        let version = raw_schema_version(path);
        self.start_editing(EditJob::from_job(&job), true, Some(path.to_path_buf()));
        self.record_recent(path);
        let mut lines = vec![t!(self, "app-project", file = file_name(path))];
        if let Some(v) = version
            && v < chipload_job::CURRENT_VERSION
        {
            lines.insert(
                1,
                t!(
                    self,
                    "app-old-schema",
                    version = v,
                    current = chipload_job::CURRENT_VERSION
                ),
            );
        }
        let mut report = Report::info(lines);
        self.differing_tool_lines(&mut report);
        self.set_report(report);
        true
    }

    /// A new project's starting values: tidy inch ones when lengths are shown in inches
    /// (docs/specs/inch-units.md §5.2).
    fn in_display_units(&self, job: chipload_job::Job) -> chipload_job::Job {
        match self.prefs.units {
            Units::Mm => job,
            Units::Inch => chipload_pipeline::with_inch_defaults(job),
        }
    }

    /// Start a new, unsaved project with no drawing (docs/specs/new-project.md).
    pub fn new_project(&mut self) {
        self.dialogs.prompt = None;
        self.reload_library();
        let mut job = self.in_display_units(empty_project());
        let notes = chipload_pipeline::library::apply_library(&mut job, &self.user.library, None)
            .unwrap_or_default();
        self.start_editing(EditJob::from_job(&job), false, None);
        self.open_stock();
        let mut lines = vec![t!(self, "app-new-project")];
        lines.extend(
            notes
                .iter()
                .map(|n| t!(self, "app-started-with", what = self.say(n))),
        );
        lines.extend(self.user.library.problems.iter().map(|p| self.say(p)));
        lines.push(t!(self, "app-new-project-next"));
        self.set_report(Report::info(lines));
    }

    /// Ask for an SVG's pixels per inch, 96 offered first (docs/specs/svg-import.md §7).
    fn ask_px_per_inch(&mut self, path: &Path, purpose: UnitsFor) {
        self.dialogs.prompt = Some(Prompt::PxPerInch(path.to_path_buf(), purpose, "96".into()));
        self.set_report(Report::info([t!(
            self,
            "app-px-missing",
            file = file_name(path)
        )]));
    }

    /// Import a DXF or an SVG as a new, unsaved project. A DXF without units asks for them.
    pub fn import_dxf(&mut self, path: &Path, units: Option<Units>) -> bool {
        self.import_file(path, units, None)
    }

    /// [`ChiploadApp::import_dxf`] with an SVG's pixels per inch; an SVG without a real size
    /// and none given asks for them.
    pub fn import_file(&mut self, path: &Path, units: Option<Units>, px: Option<f64>) -> bool {
        self.dialogs.prompt = None;
        let imported = match shapes_from_file(path, units, px, 1.0) {
            Ok(imported) => imported,
            Err(ImportError::ScaleMissing) => {
                self.ask_px_per_inch(path, UnitsFor::NewProject);
                return false;
            }
            Err(ImportError::UnitsMissing) => {
                self.dialogs.prompt = Some(Prompt::Units(path.to_path_buf(), UnitsFor::NewProject));
                self.set_report(Report::info([t!(
                    self,
                    "app-units-missing",
                    file = file_name(path)
                )]));
                return false;
            }
            Err(err) => {
                self.error_report(Report::failed(
                    t!(self, "app-cannot-import", file = file_name(path)),
                    err.message(),
                ));
                return false;
            }
        };
        let shapes = imported.shapes.len();
        // The library gives starting values (the default machine and tool).
        self.reload_library();
        let mut job = self.in_display_units(new_project(imported));
        let notes = chipload_pipeline::library::apply_library(&mut job, &self.user.library, None)
            .unwrap_or_default();
        self.start_editing(EditJob::from_job(&job), false, None);
        self.open_stock();
        let mut lines = vec![t!(
            self,
            "app-new-project-from",
            file = file_name(path),
            shapes = shapes
        )];
        lines.extend(
            notes
                .iter()
                .map(|n| t!(self, "app-started-with", what = self.say(n))),
        );
        lines.extend(self.user.library.problems.iter().map(|p| self.say(p)));
        lines.push(t!(self, "app-import-next"));
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
        self.update_file_from(id, dxf, units, None)
    }

    /// [`ChiploadApp::update_dxf_from`] with an SVG's pixels per inch (None: as chosen
    /// before, or ask).
    pub fn update_file_from(
        &mut self,
        id: Option<u64>,
        dxf: &Path,
        units: Option<Units>,
        px: Option<f64>,
    ) -> bool {
        self.dialogs.prompt = None;
        self.commit();
        let Some(edit) = &self.edit else {
            return false;
        };
        let job = match edit.to_job() {
            Ok(job) => job,
            Err(problems) => {
                self.error_report(Report::of_problems(
                    &t!(self, "app-project-incomplete"),
                    &problems,
                ));
                return false;
            }
        };
        let updated = match id {
            Some(id) => chipload_pipeline::dxfs::update_drawing_file(&job, id, dxf, units, px),
            None => chipload_pipeline::update_drawing_with(&job, dxf, units, px),
        };
        match updated {
            Ok((updated, report)) => {
                // The "side not chosen" marks stay: operations aren't added or removed.
                if let Some(edit) = self.edit.as_mut() {
                    edit.adopt(updated);
                }
                self.commit();
                self.refresh_drawing();
                self.error_ops = report.lost.iter().copied().collect();
                self.set_report(Report::of_messages(report.messages));
                if !report.lost.is_empty() {
                    self.set_status(t!(self, "app-lost-geometry"), ERROR_STATUS_SECONDS);
                }
                true
            }
            Err(PipelineError::Import(ImportError::ScaleMissing)) => {
                self.ask_px_per_inch(dxf, UnitsFor::Update(id));
                false
            }
            Err(PipelineError::Import(ImportError::UnitsMissing)) => {
                self.dialogs.prompt = Some(Prompt::Units(dxf.to_path_buf(), UnitsFor::Update(id)));
                self.set_report(Report::info([t!(
                    self,
                    "app-units-missing",
                    file = file_name(dxf)
                )]));
                false
            }
            Err(err) => {
                self.error_report(Report::failed(
                    t!(self, "app-could-not-update-drawing"),
                    err.message(),
                ));
                false
            }
        }
    }

    /// Add `dxf` to the project as another DXF, where its file has it, as one undo step
    /// (docs/specs/drawing-sources.md §2). A DXF without units asks for them.
    pub fn add_dxf_from(&mut self, dxf: &Path, units: Option<Units>) -> bool {
        self.add_file_from(dxf, units, None)
    }

    /// [`ChiploadApp::add_dxf_from`] for a DXF or an SVG, with an SVG's pixels per inch.
    pub fn add_file_from(&mut self, dxf: &Path, units: Option<Units>, px: Option<f64>) -> bool {
        self.dialogs.prompt = None;
        self.commit();
        let Some(edit) = &self.edit else {
            return false;
        };
        let job = match edit.to_job() {
            Ok(job) => job,
            Err(problems) => {
                self.error_report(Report::of_problems(
                    &t!(self, "app-project-incomplete"),
                    &problems,
                ));
                return false;
            }
        };
        match chipload_pipeline::dxfs::add_drawing_file(&job, dxf, units, px, None) {
            Ok((added, report)) => {
                // The "side not chosen" marks stay: operations aren't added or removed.
                if let Some(edit) = self.edit.as_mut() {
                    edit.adopt(added);
                }
                self.commit();
                self.refresh_drawing();
                self.set_report(Report::of_messages(report.messages));
                true
            }
            Err(PipelineError::Import(ImportError::ScaleMissing)) => {
                self.ask_px_per_inch(dxf, UnitsFor::Add);
                false
            }
            Err(PipelineError::Import(ImportError::UnitsMissing)) => {
                self.dialogs.prompt = Some(Prompt::Units(dxf.to_path_buf(), UnitsFor::Add));
                self.set_report(Report::info([t!(
                    self,
                    "app-units-missing",
                    file = file_name(dxf)
                )]));
                false
            }
            Err(err) => {
                self.error_report(Report::failed(
                    t!(self, "app-could-not-add", file = file_name(dxf)),
                    err.message(),
                ));
                false
            }
        }
    }

    /// The canvas selection as an operation takes it: shapes traced from one image become
    /// the image, minus what isn't selected, since their ids change each time it is traced
    /// again (docs/specs/image-trace.md §4, leave-out-shapes.md §5). Used by a new
    /// operation, *Use selection* and *Copy as selector* alike.
    pub(super) fn op_selection(&self) -> CanvasSelection {
        self.traced_selection()
            .unwrap_or_else(|| self.selection.clone())
    }

    /// The contour ids the canvas selection covers.
    pub fn selected_ids(&self) -> BTreeSet<usize> {
        self.selection
            .ids(&self.contours, self.edit.as_ref().map(|e| &e.job.drawing))
    }

    /// Add the operations of `kind` for the current canvas selection (one, or two for a
    /// profile of outlines with their holes; docs/specs/styles.md §2) and select the first.
    pub fn add_operation(&mut self, kind: OpKind) -> Option<usize> {
        let selection = self.op_selection();
        let library = self.library().map(|l| l.tools.clone());
        let lang = self.prefs.language;
        let edit = self.edit.as_mut()?;
        let (index, messages) =
            edit.add_with(kind, &selection, &self.contours, library.as_ref(), &lang);
        self.focus = Focus::Op(index);
        self.commit();
        if !messages.is_empty() {
            self.set_report(Report::of_messages(messages));
        }
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
                self.error_report(Report::of_problems(
                    &t!(self, "app-project-incomplete"),
                    &problems,
                ));
                return false;
            }
        };
        let placed = match dxf {
            Some(id) => chipload_pipeline::placement::place_dxf(&job, id, how),
            None => chipload_pipeline::placement::place_drawing(&job, how),
        };
        match placed {
            Ok((placed, report)) => {
                // The "side not chosen" marks stay: placing only moves start points.
                if let Some(edit) = self.edit.as_mut() {
                    edit.adopt(placed);
                }
                self.commit();
                self.refresh_drawing();
                self.set_report(Report::of_messages(report.messages));
                true
            }
            Err(err) => {
                self.error_report(Report::failed(
                    t!(self, "app-could-not-move-drawing"),
                    err.message(),
                ));
                false
            }
        }
    }

    /// DXF `id`'s box dragged on the canvas: scaled and turned about its centre `from`,
    /// which goes to `to` (docs/specs/dxf-whole-on-canvas.md §2), as one undo step. It stays
    /// selected; the report is left as it was, a drag says what it did by where the DXF is.
    pub fn drag_dxf(&mut self, id: u64, from: Point, to: Point, scale: f64, degrees: f64) -> bool {
        self.commit();
        let Some(edit) = &self.edit else {
            return false;
        };
        let job = match edit.to_job() {
            Ok(job) => job,
            Err(problems) => {
                self.error_report(Report::of_problems(
                    &t!(self, "app-project-incomplete"),
                    &problems,
                ));
                return false;
            }
        };
        match chipload_pipeline::placement::drag_dxf(&job, id, from, to, scale, degrees) {
            Ok((placed, _)) => {
                if let Some(edit) = self.edit.as_mut() {
                    edit.adopt(placed);
                }
                self.commit();
                self.refresh_drawing();
                true
            }
            Err(err) => {
                self.error_report(Report::failed(
                    t!(self, "app-could-not-move-drawing"),
                    err.message(),
                ));
                false
            }
        }
    }

    pub fn set_copies_gap(&mut self, id: u64, gap: f64) {
        self.commit();
        let Some(edit) = &self.edit else {
            return;
        };
        // The part alone: the grid being set may not be valid yet (a pitch of 0).
        let mut part = edit.clone();
        for p in &mut part.job.parts {
            p.copies = None;
        }
        let job = match part.to_job() {
            Ok(job) => job,
            Err(problems) => {
                self.error_report(Report::of_problems(
                    &t!(self, "app-project-incomplete"),
                    &problems,
                ));
                return;
            }
        };
        let current = edit
            .job
            .part(id)
            .and_then(|p| p.copies)
            .map_or((0.0, 0.0), |c| (c.dx, c.dy));
        match chipload_pipeline::copies::pitch_for_gap(&job, id, gap, current) {
            Ok((dx, dy)) => {
                let copies = self.edit.as_mut().and_then(|e| {
                    e.job
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
                let (units, unit) = (
                    self.prefs.units,
                    chipload_pipeline::units::length_unit(self.prefs.units),
                );
                let text = t!(
                    self,
                    "app-pitch-set",
                    dx = crate::draw::fixed(dx, units),
                    dy = format!("{} {unit}", crate::draw::fixed(dy, units)),
                    gap = format!("{} {unit}", crate::draw::fixed(gap, units))
                );
                self.set_status(text, STATUS_SECONDS);
            }
            Err(err) => self.error_report(Report::failed(
                t!(self, "app-pitch-needs-plan"),
                err.message(),
            )),
        }
    }

    /// Give the selected operation the current canvas selection as its geometry.
    pub fn use_selection(&mut self) {
        let selection = self.op_selection();
        if let (Focus::Op(i), Some(edit)) = (self.focus, self.edit.as_mut()) {
            edit.set_geometry(i, &selection, &self.contours);
            self.error_ops.remove(&i);
            self.commit();
        }
    }

    /// Move the operation at `from` to position `to` in the cut order.
    pub fn move_operation(&mut self, from: usize, to: usize) {
        if let Some(edit) = self.edit.as_mut()
            && from < edit.job.operations.len()
            && to < edit.job.operations.len()
        {
            edit.move_op(from, to);
            self.focus = Focus::Op(to);
            self.commit();
        }
    }

    // -- parts (docs/specs/parts.md §8)

    /// A new, empty part at the end: it becomes active and opens in the properties.
    pub fn add_part(&mut self) -> Option<u64> {
        let id = self.edit.as_mut()?.add_part();
        self.focus = Focus::Part(id);
        self.commit();
        Some(id)
    }

    /// Make part `id` the one new operations go into.
    pub fn activate_part(&mut self, id: u64) {
        if let Some(edit) = self.edit.as_mut()
            && let Some(setup) = edit.job.part(id).map(|p| p.setup)
        {
            edit.job.active_part = Some(id);
            // Its setup is the one shown (docs/specs/setups.md §7).
            edit.job.active_setup = Some(setup);
            self.commit();
        }
    }

    /// Delete part `id` with its operations, as one undo step; the last part stays.
    pub fn delete_part(&mut self, id: u64) {
        let Some(edit) = self.edit.as_mut() else {
            return;
        };
        let name = edit
            .job
            .part(id)
            .map(|p| p.name.clone())
            .unwrap_or_default();
        match edit.delete_part(id) {
            Some(ops) => {
                self.focus = Focus::Job;
                self.commit();
                let text = t!(self, "app-deleted-part", count = ops, name = name);
                self.set_status(text, STATUS_SECONDS);
            }
            None => self.set_status(t!(self, "app-part-needed"), STATUS_SECONDS),
        }
    }

    /// Move part `id` one place up or down, with its operations.
    pub fn move_part(&mut self, id: u64, up: bool) {
        if let Some(edit) = self.edit.as_mut() {
            edit.move_part(id, up);
            self.focus = Focus::Part(id);
            self.commit();
        }
    }

    /// Move operation `from` to the end of part `id`.
    pub fn move_operation_to_part(&mut self, from: usize, id: u64) {
        if let Some(at) = self.edit.as_mut().and_then(|e| e.move_op_to_part(from, id)) {
            self.focus = Focus::Op(at);
            self.commit();
        }
    }

    /// Turn part `id` on or off.
    pub fn toggle_part(&mut self, id: u64) {
        if let Some(part) = self
            .edit
            .as_mut()
            .and_then(|e| e.job.parts.iter_mut().find(|p| p.id == id))
        {
            part.enabled = !part.enabled;
            self.commit();
        }
    }

    pub fn duplicate_operation(&mut self) {
        if let (Focus::Op(i), Some(edit)) = (self.focus, self.edit.as_mut())
            && let Some(copy) = edit.duplicate(i, &self.prefs.language)
        {
            self.focus = Focus::Op(copy);
            self.commit();
        }
    }

    /// Add a rest pass to pocket `i`, cut with `tool` (docs/specs/rest-passes.md §6), and
    /// open it. False, with the reason shown, when it can't be added.
    pub fn add_rest_pass(&mut self, i: usize, tool: i64) -> bool {
        let cutting = self.tool_cutting(tool);
        let Some(edit) = self.edit.as_mut() else {
            return false;
        };
        let tools = edit.job.tools.clone();
        let Some(Operation::Pocket(pocket)) = edit.job.operations.get_mut(i) else {
            return false;
        };
        match chipload_pipeline::rest_pass_of(pocket, &tools, tool, cutting.as_ref()) {
            Ok(pass) => {
                pocket.rest_passes.push(pass);
                self.focus = Focus::RestPass(i, pocket.rest_passes.len() - 1);
                self.commit();
                true
            }
            Err(err) => {
                self.error(vec![t!(
                    self,
                    "app-could-not-add-rest-pass",
                    error = self.say(&err)
                )]);
                false
            }
        }
    }

    /// Add a rest pass to pocket `i` cut with the library tool `name`, copied into the
    /// project first unless it is there already: one undo step (docs/specs/rest-passes.md
    /// §6). If the pass can't be added, the project's tools stay as they were.
    pub fn add_rest_pass_from_library(&mut self, i: usize, name: &str) -> bool {
        let Some(before) = self.edit.as_ref().map(|e| e.job.tools.clone()) else {
            return false;
        };
        let Some(tool) = self.insert_library_tool(name) else {
            return false;
        };
        if self.add_rest_pass(i, tool) {
            return true;
        }
        if let Some(edit) = self.edit.as_mut() {
            edit.job.tools = before;
        }
        false
    }

    /// What pocket `i`'s *Add rest pass ▸* offers: the endmills smaller than the last tool
    /// of its chain, the project's and then the library's, or why there are none
    /// (docs/specs/rest-passes.md §6). None for any other operation.
    pub fn rest_menu(&self, i: usize, units: Units) -> Option<RestMenu> {
        let edit = self.edit.as_ref()?;
        let Operation::Pocket(pocket) = &edit.job.operations.get(i)? else {
            return None;
        };
        if chipload_job::has_bones(pocket.corners, &pocket.corner_choices) {
            return Some(RestMenu::Unavailable(t!(self, "app-rest-with-corners")));
        }
        let tools = &edit.job.tools;
        let last = pocket.rest_passes.last().map_or(pocket.tool, |r| r.tool);
        let Some(diameter) = tools.iter().find(|t| t.id == last).map(|t| t.diameter) else {
            return Some(RestMenu::Unavailable(t!(
                self,
                "app-rest-tool-missing",
                tool = format!("T{last}")
            )));
        };
        let project: Vec<(i64, f64)> = tools
            .iter()
            .filter(|t| t.kind == ToolType::Endmill && t.diameter < diameter)
            .map(|t| (t.id, t.diameter))
            .collect();
        // A library tool already copied in with the same values is listed as the project's.
        let library: Vec<(String, f64)> = self
            .library()
            .map(|l| l.tools.tools.as_slice())
            .unwrap_or_default()
            .iter()
            .filter(|e| e.kind == ToolType::Endmill && e.diameter < diameter)
            .filter(|e| {
                !tools
                    .iter()
                    .any(|t| t.library_uid.as_deref() == Some(e.uid.as_str()) && e.same_values(t))
            })
            .map(|e| (e.name.clone(), e.diameter))
            .collect();
        if project.is_empty() && library.is_empty() {
            return Some(RestMenu::Unavailable(t!(
                self,
                "app-rest-no-smaller",
                tool = format!("T{last}"),
                diameter = chipload_pipeline::units::length_text(diameter, units)
            )));
        }
        Some(RestMenu::Tools { project, library })
    }

    /// Pocket `i`'s rest passes, to change.
    pub(super) fn rest_passes_mut(
        &mut self,
        i: usize,
    ) -> Option<&mut Vec<chipload_job::ClearingPass>> {
        match self.edit.as_mut()?.job.operations.get_mut(i) {
            Some(Operation::Pocket(p)) => Some(&mut p.rest_passes),
            _ => None,
        }
    }

    /// Remove pocket `i`'s rest pass `k`; its pocket opens.
    pub fn remove_rest_pass(&mut self, i: usize, k: usize) {
        if let Some(passes) = self.rest_passes_mut(i)
            && k < passes.len()
        {
            passes.remove(k);
            self.focus = Focus::Op(i);
            self.commit();
        }
    }

    /// Turn pocket `i`'s rest pass `k` on or off.
    pub fn toggle_rest_pass(&mut self, i: usize, k: usize) {
        if let Some(pass) = self.rest_passes_mut(i).and_then(|p| p.get_mut(k)) {
            pass.enabled = !pass.enabled;
            self.commit();
        }
    }

    /// Give V-carve `i` a floor pass cut with `tool` (docs/specs/vcarve-wide-strokes.md §7),
    /// in place of any it had, and open it. False, with the reason shown, when it can't.
    pub fn add_floor_pass(&mut self, i: usize, tool: i64) -> bool {
        let cutting = self.tool_cutting(tool);
        let Some(edit) = self.edit.as_mut() else {
            return false;
        };
        let tools = edit.job.tools.clone();
        let Some(Operation::Vcarve(vcarve)) = edit.job.operations.get_mut(i) else {
            return false;
        };
        match chipload_pipeline::floor_pass_of(vcarve, &tools, tool, cutting.as_ref()) {
            Ok(pass) => {
                vcarve.floor = Some(pass);
                self.focus = Focus::FloorPass(i);
                self.commit();
                true
            }
            Err(err) => {
                self.error(vec![t!(
                    self,
                    "app-could-not-add-floor-pass",
                    error = self.say(&err)
                )]);
                false
            }
        }
    }

    /// [`App::add_floor_pass`] with the library tool `name`, copied into the project first
    /// unless it is there already. If the pass can't be added, the tools stay as they were.
    pub fn add_floor_pass_from_library(&mut self, i: usize, name: &str) -> bool {
        let Some(before) = self.edit.as_ref().map(|e| e.job.tools.clone()) else {
            return false;
        };
        let Some(tool) = self.insert_library_tool(name) else {
            return false;
        };
        if self.add_floor_pass(i, tool) {
            return true;
        }
        if let Some(edit) = self.edit.as_mut() {
            edit.job.tools = before;
        }
        false
    }

    /// What V-carve `i`'s *Add floor pass ▸* offers: every end mill, the project's and then
    /// the library's not yet in it; or why there is none. None for any other operation.
    pub fn floor_menu(&self, i: usize) -> Option<RestMenu> {
        let edit = self.edit.as_ref()?;
        let Operation::Vcarve(_) = &edit.job.operations.get(i)? else {
            return None;
        };
        let tools = &edit.job.tools;
        let project: Vec<(i64, f64)> = tools
            .iter()
            .filter(|t| t.kind == ToolType::Endmill)
            .map(|t| (t.id, t.diameter))
            .collect();
        let library: Vec<(String, f64)> = self
            .library()
            .map(|l| l.tools.tools.as_slice())
            .unwrap_or_default()
            .iter()
            .filter(|e| e.kind == ToolType::Endmill)
            .filter(|e| {
                !tools
                    .iter()
                    .any(|t| t.library_uid.as_deref() == Some(e.uid.as_str()) && e.same_values(t))
            })
            .map(|e| (e.name.clone(), e.diameter))
            .collect();
        if project.is_empty() && library.is_empty() {
            return Some(RestMenu::Unavailable(t!(self, "app-floor-no-endmill")));
        }
        Some(RestMenu::Tools { project, library })
    }

    /// V-carve `i`'s floor pass, to change.
    pub(super) fn floor_pass_mut(&mut self, i: usize) -> Option<&mut chipload_job::ClearingPass> {
        match self.edit.as_mut()?.job.operations.get_mut(i) {
            Some(Operation::Vcarve(v)) => v.floor.as_mut(),
            _ => None,
        }
    }

    /// Remove V-carve `i`'s floor pass; the V-carve opens.
    pub fn remove_floor_pass(&mut self, i: usize) {
        if let Some(Some(Operation::Vcarve(v))) =
            self.edit.as_mut().map(|e| e.job.operations.get_mut(i))
            && v.floor.take().is_some()
        {
            self.focus = Focus::Op(i);
            self.commit();
        }
    }

    /// Turn V-carve `i`'s floor pass on or off.
    pub fn toggle_floor_pass(&mut self, i: usize) {
        if let Some(pass) = self.floor_pass_mut(i) {
            pass.enabled = !pass.enabled;
            self.commit();
        }
    }

    /// For V-carve `i`'s floor pass form: the largest end mill that would clear its whole
    /// floor, mm, and the library's end mills that do, by name, nearest first
    /// (docs/specs/vcarve-wide-strokes.md §4.1). None when it clears it all.
    pub(super) fn floor_hint(&self, i: usize) -> Option<(f64, Vec<(String, f64)>)> {
        let (_, largest) = self
            .toolpath
            .result
            .as_ref()?
            .floor_hints
            .iter()
            .find(|(op, _)| *op == i)?;
        let library = self
            .library()
            .map(|l| l.tools.tools.as_slice())
            .unwrap_or_default();
        let fitting = chipload_pipeline::fitting_end_mills(library, *largest)
            .into_iter()
            .map(|t| (t.name.clone(), t.diameter))
            .collect();
        Some((*largest, fitting))
    }

    /// The project as edited, its operations included, for the pipeline's job-level rules.
    pub(super) fn edited_job(&self) -> Option<chipload_job::Job> {
        let edit = self.edit.as_ref()?;
        let job = edit.job.clone();
        Some(job)
    }

    /// The pocket that rest pocket `i` copies, which it can become a rest pass of
    /// (docs/specs/rest-passes.md §5).
    pub(super) fn rest_pocket_parent(&self, i: usize) -> Option<usize> {
        chipload_pipeline::rest_pocket_parent(&self.edited_job()?, i)
    }

    /// *Make it a rest pass*: rest pocket `i` becomes the last rest pass of the pocket it
    /// copies, which opens on it.
    pub fn make_rest_pass(&mut self, i: usize) {
        let Some(parent) = self.rest_pocket_parent(i) else {
            return;
        };
        let Some(edit) = self.edit.as_mut() else {
            return;
        };
        if !matches!(edit.job.operations.get(i), Some(Operation::Pocket(_))) {
            return;
        }
        let Some(Operation::Pocket(rest)) = edit.remove(i) else {
            return;
        };
        if let Some(Operation::Pocket(pocket)) = edit.job.operations.get_mut(parent) {
            pocket.rest_passes.push(chipload_job::ClearingPass {
                tool: rest.tool,
                feed: rest.feed,
                plunge_feed: rest.plunge_feed,
                step_down: rest.step_down,
                stepover: rest.stepover,
                slot_feed: rest.slot_feed,
                slot_step_down: rest.slot_step_down,
                entry: rest.entry,
                enabled: rest.enabled,
            });
            self.focus = Focus::RestPass(parent, pocket.rest_passes.len() - 1);
            self.commit();
            let text = t!(self, "app-now-rest-pass", op = i + 1, parent = parent + 1);
            self.set_status(text, STATUS_SECONDS);
        }
    }

    pub fn delete_operation(&mut self) {
        if let (Focus::Op(i), Some(edit)) = (self.focus, self.edit.as_mut()) {
            edit.remove(i);
            let len = edit.job.operations.len();
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
        if self.toolpath.result.is_none() {
            self.error(vec![t!(self, "app-generate-before-export")]);
            return false;
        }
        // Never a preview of one part: every part is planned first, then written there
        // (docs/specs/active-part-preview.md §2).
        if self.toolpath.part.is_some() {
            if self.start_generation_of(true, true) {
                self.toolpath.export_path_when_done = Some(path.to_path_buf());
                self.set_status(t!(self, "app-planning-then-export"), STATUS_SECONDS);
            }
            return false;
        }
        self.write_export(path, Vec::new())
    }

    /// Write the G-code of part `id` alone, as if every other part were switched off, and
    /// its setup sheet (docs/specs/part-export.md §4): now if the toolpath shown is exactly
    /// that part's, else once it is planned. Returns whether it was written now.
    pub fn export_part(&mut self, id: u64, path: &Path) -> bool {
        let Some(name) = self
            .edit
            .as_ref()
            .and_then(|e| e.job.part(id))
            .map(|p| p.name.clone())
        else {
            return false;
        };
        let current = self.toolpath.result.is_some()
            && !self.toolpath.stale
            && self.toolpath.running.is_none();
        if current && self.toolpath.part == Some(id) {
            return self.write_export(path, vec![name]);
        }
        if self.start_generation_scoped(true, Some(id)) {
            self.toolpath.export_path_when_done = None;
            self.toolpath.export_when_done = false;
            self.toolpath.export_part_when_done = Some((id, path.to_path_buf()));
            let status = t!(self, "app-planning-part-then-export", name = name);
            self.set_status(status, STATUS_SECONDS);
        }
        false
    }

    /// Write the G-code of setup `id` and its setup sheet (docs/specs/setups.md §7), named
    /// `-S<id>` when there are several: the setup is made the active one, then written now
    /// if the toolpath shown is all of it, else once it is planned. Returns whether it was
    /// written now.
    pub fn export_setup(&mut self, id: u64, path: &Path) -> bool {
        if self.edit.as_ref().and_then(|e| e.job.setup(id)).is_none() {
            return false;
        }
        if self.edit.as_ref().and_then(|e| e.job.active_setup_id()) != Some(id) {
            self.activate_setup(id);
        }
        let shown = self
            .toolpath
            .job
            .as_ref()
            .is_some_and(|job| job.active_setup_id() == Some(id));
        let current = self.toolpath.result.is_some()
            && !self.toolpath.stale
            && self.toolpath.running.is_none()
            && self.toolpath.part.is_none()
            && shown;
        if current {
            return self.write_export(path, Vec::new());
        }
        if self.start_generation_of(true, true) {
            self.toolpath.export_when_done = false;
            self.toolpath.export_part_when_done = None;
            self.toolpath.export_path_when_done = Some(path.to_path_buf());
            self.set_status(t!(self, "app-planning-then-export"), STATUS_SECONDS);
        }
        false
    }

    /// Write the last generation's G-code files, and its setup sheet when the setting asks
    /// for one; `parts` names the parts when only those were planned.
    fn write_export(&mut self, path: &Path, parts: Vec<String>) -> bool {
        let Some(result) = &self.toolpath.result else {
            self.error(vec![t!(self, "app-generate-before-export")]);
            return false;
        };
        // With several setups the result is the active one's: its files say so
        // (`-S2-1-T1`), as `run` names them (docs/specs/setups.md §4).
        let setup = self
            .toolpath
            .job
            .as_ref()
            .filter(|job| job.setups.len() > 1)
            .and_then(|job| job.active_setup_id());
        let named;
        let result = match setup {
            Some(id) => {
                let mut copy = result.clone();
                for file in &mut copy.files {
                    let tail = file.suffix.take().unwrap_or_default();
                    file.suffix = Some(format!("-S{id}{tail}"));
                }
                named = copy;
                &named
            }
            None => result,
        };
        let written = match write_gcode_files(path, &result.files) {
            Ok(written) => written,
            Err(err) => {
                // The file system's own words.
                self.error(vec![
                    t!(
                        self,
                        "app-could-not-export",
                        path = path.display().to_string()
                    ),
                    err.to_string(),
                ]);
                return false;
            }
        };
        let sheet = match (&self.toolpath.job, self.prefs.setup_sheet) {
            (Some(job), true) => {
                // In the app's language and display units (docs/specs/translations.md
                // step 3).
                let info = SheetInfo {
                    project: self
                        .file
                        .path
                        .as_deref()
                        .map_or_else(|| t!(self, "export-untitled-project"), file_name),
                    written_by: format!("chipload {}", env!("CARGO_PKG_VERSION")),
                    units: self.prefs.units,
                    language: self.lang(),
                    parts,
                };
                match write_setup_sheet(job, result, path, &info) {
                    Ok(sheet) => Some(sheet),
                    Err(err) => {
                        // The file system's own words.
                        self.error(vec![
                            t!(
                                self,
                                "app-sheet-failed",
                                path = chipload_pipeline::sheet::setup_sheet_path(job, path)
                                    .display()
                                    .to_string()
                            ),
                            err.to_string(),
                        ]);
                        return false;
                    }
                }
            }
            _ => None,
        };
        let shown = path.display().to_string();
        let setup_note = setup.and_then(|id| {
            let job = self.toolpath.job.as_ref()?;
            let name = job.setup(id)?.name.clone();
            Some(t!(self, "app-exported-setup", name = name))
        });
        if written.len() > 1 {
            let mut report = Report::default();
            report.push(
                Tone::Good,
                t!(self, "app-exported-n-files", count = written.len()),
            );
            for file in &result.files {
                report.push_message(Tone::Info, file.describe_message(path), None);
            }
            if let Some(sheet) = &sheet {
                report.push(
                    Tone::Info,
                    t!(
                        self,
                        "app-sheet-for-operator",
                        path = sheet.display().to_string()
                    ),
                );
            }
            self.set_report(report);
            let count = written.len();
            let text = match &sheet {
                Some(sheet) => t!(
                    self,
                    "app-exported-files-sheet",
                    count = count,
                    sheet = file_name(sheet),
                    path = shown
                ),
                None => t!(self, "app-exported-files", count = count, path = shown),
            };
            self.set_status(text, STATUS_SECONDS);
        } else {
            let text = match &sheet {
                Some(sheet) => t!(
                    self,
                    "app-exported-sheet",
                    path = shown,
                    sheet = file_name(sheet)
                ),
                None => t!(self, "app-exported", path = shown),
            };
            self.set_status(text, STATUS_SECONDS);
        }
        if let Some(note) = setup_note
            && let Some((text, _)) = &self.status
        {
            let text = format!("{text} {note}");
            self.set_status(text, STATUS_SECONDS);
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
                self.error_report(Report::of_problems(
                    &t!(self, "app-cannot-save-yet"),
                    &problems,
                ));
                return false;
            }
        };
        if let Err(err) = save_job(&job, path) {
            self.error_report(Report::failed(
                t!(
                    self,
                    "app-could-not-save",
                    path = path.display().to_string()
                ),
                err.message(),
            ));
            return false;
        }
        let saved = EditJob::from_job(&job);
        self.edit = Some(saved.clone());
        self.history.committed = Some(saved.clone());
        self.file.saved = Some(saved);
        self.file.path = Some(path.to_path_buf());
        self.remember_disk();
        self.clear_recovery();
        let text = t!(self, "app-saved", path = path.display().to_string());
        self.set_status(text, STATUS_SECONDS);
        self.record_recent(path);
        true
    }

    /// Click on the canvas: select one contour, Ctrl toggles, empty space clears.
    /// A click on the canvas: `picked` is the contour hit (if any); `toggle` (Ctrl) adds or
    /// removes it instead of replacing the selection. Selections are kept as shape ids.
    pub fn click(&mut self, picked: Option<usize>, toggle: bool) {
        // Picking on the canvas shows what is picked, and only that (the user, 2026-10-28).
        // The selection stays when something else is opened, for an operation's Use selection.
        if picked.is_some() {
            self.focus = Focus::Selection;
        }
        // A grouped shape selects its whole group (docs/specs/shape-groups.md §2, decision 10).
        if let Some(group) = picked.and_then(|id| self.group_of_contour(id))
            && !toggle
        {
            self.selection = CanvasSelection::whole(Source::Group(group));
            return;
        }
        // A traced contour selects its whole picture, a letter its whole text: their shapes
        // get new ids each time the picture is traced again or the text written again, so
        // operations select the picture or text, not its shapes (docs/specs/image-trace.md
        // §4, decision 6; user report 2026-10-13; text.md §5). A DXF's contour selects the
        // whole DXF too, to be moved by its box; a double-click picks the contour alone
        // (dxf-whole-on-canvas.md §2).
        let traced = picked
            .and_then(|id| self.contours.iter().find(|c| c.id == id))
            .and_then(|c| match c.parts.first().and_then(|p| p.segment.from) {
                Some(from) if c.parts.iter().all(|p| p.segment.from == Some(from)) => Some(from),
                _ => None,
            });
        if let (Some(from), false) = (traced, toggle) {
            self.selection = CanvasSelection::whole(from);
            // A letter opens its text, as the text's Browser row does: its words, font and
            // height to change (the user, 2026-10-30). The text stays selected for an
            // operation's Use selection.
            if let chipload_core::chain::ShapeFrom::Text(id) = from {
                self.focus = Focus::Text(id);
            }
            return;
        }
        // Ctrl+click on a contour of the selected DXF, picture, text or group leaves it out,
        // or puts it back (docs/specs/leave-out-shapes.md §5, shape-groups.md decision 10).
        if toggle
            && let CanvasSelection::Whole { of, left_out } = &self.selection
            && let Some(contour) = picked.and_then(|id| self.contours.iter().find(|c| c.id == id))
            && CanvasSelection::whole(*of)
                .ids(&self.contours, self.edit.as_ref().map(|e| &e.job.drawing))
                .contains(&contour.id)
        {
            let mut left_out = left_out.clone();
            let hit = with_twins(shapes_of(contour), &self.twins);
            if hit.is_subset(&left_out) {
                left_out.retain(|s| !hit.contains(s));
            } else {
                left_out.extend(hit);
            }
            self.selection = CanvasSelection::Whole { of: *of, left_out };
            return;
        }
        let mut shapes: BTreeSet<u64> = match (&self.selection, toggle) {
            (CanvasSelection::Shapes(shapes), true) => shapes.clone(),
            (CanvasSelection::Layer(..) | CanvasSelection::Whole { .. }, true) => {
                let ids = self.selected_ids();
                match CanvasSelection::of_contours(&ids, &self.contours, &self.twins) {
                    CanvasSelection::Shapes(shapes) => shapes,
                    _ => BTreeSet::new(),
                }
            }
            _ => BTreeSet::new(),
        };
        let hit: BTreeSet<u64> = picked
            .and_then(|id| self.contours.iter().find(|c| c.id == id))
            .map(|c| with_twins(shapes_of(c), &self.twins))
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
        let selector = self.op_selection().selector(&self.contours, false);
        (!selector.is_empty()).then(|| selector_text(&selector))
    }

    pub fn selection_lines(&self) -> Vec<String> {
        let ids = self.selected_ids();
        if ids.is_empty() {
            return vec![t!(self, "app-nothing-selected")];
        }
        let what = match &self.selection {
            CanvasSelection::Layer(layer, _) => t!(self, "app-selection-layer", layer = layer),
            CanvasSelection::Whole { of, left_out } => {
                let count = CanvasSelection::whole(*of)
                    .ids(&self.contours, self.edit.as_ref().map(|e| &e.job.drawing))
                    .len()
                    - ids.len();
                match (of.shape_from(), left_out.is_empty()) {
                    (Some(from), true) => self.source_text(from),
                    (Some(from), false) => t!(
                        self,
                        "app-selection-left-out",
                        source = self.source_text(from),
                        count = count
                    ),
                    (None, true) => t!(self, "groups-selection", name = self.group_name(of.id())),
                    (None, false) => t!(
                        self,
                        "groups-selection-left-out",
                        name = self.group_name(of.id()),
                        count = count
                    ),
                }
            }
            _ => t!(self, "app-selection-contours"),
        };
        let list: Vec<String> = ids.iter().map(|i| format!("#{i}")).collect();
        let mut lines = vec![t!(
            self,
            "app-selected",
            count = ids.len(),
            what = what,
            list = list.join(", ")
        )];
        let open: Vec<String> = self
            .contours
            .iter()
            .filter(|c| ids.contains(&c.id) && !c.closed)
            .map(|c| format!("#{}", c.id))
            .collect();
        if !open.is_empty() {
            lines.push(t!(self, "app-open-not-cut", list = open.join(", ")));
        }
        lines
    }

    /// What the selection's shapes come from: a DXF, an image or a text.
    fn source_text(&self, from: chipload_core::chain::ShapeFrom) -> String {
        match from {
            chipload_core::chain::ShapeFrom::Dxf(id) => t!(self, "app-selection-dxf", id = id),
            chipload_core::chain::ShapeFrom::Image(id) => {
                t!(self, "app-selection-image", id = id)
            }
            chipload_core::chain::ShapeFrom::Text(id) => t!(self, "app-selection-text", id = id),
            chipload_core::chain::ShapeFrom::Model(id) => {
                t!(self, "app-selection-model", id = id)
            }
        }
    }

    // -- internals

    fn focused_contours(&self) -> BTreeSet<usize> {
        let (
            Focus::Op(i)
            | Focus::RestPass(i, _)
            | Focus::FloorPass(i)
            | Focus::RoughingPass(i)
            | Focus::SmoothingPass(i),
            Some(edit),
        ) = (self.focus, &self.edit)
        else {
            return BTreeSet::new();
        };
        edit.job
            .operations
            .get(i)
            .map(|entry| self.op_contours(entry))
            .unwrap_or_default()
    }

    /// The contours an operation cuts, as planning selects them.
    pub(super) fn op_contours(&self, op: &Operation) -> BTreeSet<usize> {
        if !op.selects_shapes() {
            return BTreeSet::new();
        }
        let select = match &self.edit {
            Some(edit) => op.select().resolved(&edit.job.drawing),
            None => op.select().clone(),
        };
        let selected = match op {
            Operation::Path(_) | Operation::Holes(_) => {
                select_paths(&self.contours, &select, &op.label_message(0))
            }
            _ => select_contours(&self.contours, &select, &op.label_message(0)),
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
        edit.job
            .operations
            .iter()
            .filter(|e| e.enabled() && matches!(e, Operation::Path(_)))
            .filter_map(|e| {
                let select = e.select().resolved(&edit.job.drawing);
                select_paths(&self.contours, &select, &e.label_message(0)).ok()
            })
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
        self.toolpath.shown_inputs = None;
        self.toolpath.changed_at = None;
        self.toolpath.export_when_done = false;
        self.toolpath.stale = false;
        self.toolpath.result = None;
        self.clear_toolpath();
        self.drag.start = None;
        self.drag.context_start = None;
    }

    /// The project changed: the toolpath shown is kept, faded, until a new one replaces it
    /// (docs/specs/background-generation.md decision 4); a run in progress is cancelled.
    fn toolpath_out_of_date(&mut self) {
        self.refresh_problems();
        self.stop_generation();
        self.toolpath.shown_inputs = None;
        self.toolpath.changed_at = Some(Instant::now());
        self.recovery.due = self.toolpath.changed_at;
        if self.toolpath.result.is_some() || self.scene.toolpath.is_some() {
            self.toolpath.result = None;
            if let Some(path) = &mut self.scene.toolpath {
                path.stale = true;
            }
            self.scene.tabs.clear();
            self.scene.start_markers.clear();
            self.scene.tab_markers.clear();
            self.corners.unreached.clear();
            self.drag.start = None;
            self.drag.context_start = None;
            self.drag.tab = None;
            self.drag.context_tab = None;
            self.drag.context_ring = None;
            self.toolpath.stale = true;
        }
        self.refresh_drawing();
        self.update_ghosts();
    }

    fn error(&mut self, lines: Vec<String>) {
        self.error_report(Report::error(lines));
    }

    /// A failure: shown in the report, its first line in the status bar too.
    fn error_report(&mut self, report: Report) {
        if let Some(first) = report.lines.first() {
            let text = match &first.message {
                Some(message) => self.say(message),
                None => first.text.clone(),
            };
            self.set_status(text, ERROR_STATUS_SECONDS);
        }
        self.set_report(report);
    }

    fn set_status(&mut self, text: String, seconds: u64) {
        self.status = Some((text, Instant::now() + Duration::from_secs(seconds)));
    }

    /// A project with operations in its active setup: one without has nothing to generate
    /// (the user, 2026-11-03: "We should not generate if there are no operations").
    fn can_generate(&self) -> bool {
        self.edit
            .as_ref()
            .is_some_and(|e| generation::active_setup_has_operations(&e.job))
    }

    /// Proceed now, or ask first if unsaved changes would be lost.
    fn guard(&mut self, action: Pending) -> bool {
        if self.dirty() {
            self.dialogs.prompt = Some(Prompt::Discard(action));
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
    pub fn update_labels(&self) -> (String, String) {
        let first = self
            .edit
            .as_ref()
            .is_some_and(|e| e.job.drawing.dxfs.is_empty());
        if first {
            (
                t!(self, "app-import-dxf-into-project"),
                t!(self, "app-import-dxf-into-project-path"),
            )
        } else {
            (
                t!(self, "app-update-drawing-from-dxf"),
                t!(self, "app-update-drawing-from-path"),
            )
        }
    }

    /// How many DXFs the open project has.
    pub(super) fn dxf_count(&self) -> usize {
        self.edit.as_ref().map_or(0, |e| e.job.drawing.dxfs.len())
    }

    fn default_project_path(&self) -> PathBuf {
        if let Some(path) = &self.file.path {
            return path.with_extension(PROJECT_EXTENSION);
        }
        self.edit
            .as_ref()
            .and_then(|e| e.job.drawing.dxfs.first())
            .map(|s| Path::new(&s.file).with_extension(PROJECT_EXTENSION))
            .unwrap_or_else(|| PathBuf::from(format!("project.{PROJECT_EXTENSION}")))
    }

    fn default_export_path(&self) -> PathBuf {
        let ext = self.toolpath.result.as_ref().map_or("nc".to_string(), |r| {
            r.post.file_extension.trim_start_matches('.').to_string()
        });
        self.default_project_path().with_extension(ext)
    }

    /// The project's name and the part's: `box-lid.nc` (docs/specs/part-export.md §6.3).
    pub(super) fn default_part_export_path(&self, id: u64, name: &str) -> PathBuf {
        let general = self.default_export_path();
        let stem = general
            .file_stem()
            .map_or_else(String::new, |s| s.to_string_lossy().into_owned());
        let mut part = file_safe(name);
        if part.is_empty() {
            part = format!("part-{id}");
        }
        let file = match general.extension() {
            Some(ext) => format!("{stem}-{part}.{}", ext.to_string_lossy()),
            None => format!("{stem}-{part}"),
        };
        general.with_file_name(file)
    }
}

/// A name fit for a file name: letters, digits, `-` and `_` kept, spaces made `-`, the
/// rest left out.
fn file_safe(name: &str) -> String {
    let mut out = String::new();
    for c in name.trim().chars() {
        if c.is_alphanumeric() || c == '-' || c == '_' {
            out.push(c);
        } else if c.is_whitespace() && !out.ends_with('-') {
            out.push('-');
        }
    }
    out.trim_matches('-').to_string()
}

impl eframe::App for ChiploadApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        if ctx.input(|i| i.viewport().close_requested()) {
            if self.dirty() && !self.allow_close {
                ctx.send_viewport_cmd(ViewportCommand::CancelClose);
                self.dialogs.prompt = Some(Prompt::Discard(Pending::Close));
            } else if !self.dirty() {
                self.clear_recovery(); // nothing unsaved: no copy to leave behind
            }
        }
        crate::units::set_display(&ctx, self.prefs.units);
        crate::i18n::set_language(&ctx, self.prefs.language);
        if self.prefs.applied_theme != Some(self.prefs.theme) {
            ctx.set_theme(theme_preference(self.prefs.theme));
            self.prefs.applied_theme = Some(self.prefs.theme);
        }
        // Esc lets go of an entry dragged out of the shape library.
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            egui::DragAndDrop::clear_payload(&ctx);
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
        // The ribbon and the status bar are chrome, a step below the panels
        // (docs/specs/look-and-feel.md §3).
        let chrome = |ui: &egui::Ui, margin: egui::Margin| {
            egui::Frame::new()
                .fill(crate::style::palette(ui.visuals()).chrome)
                .inner_margin(margin)
        };
        Panel::top("ribbon")
            .frame(chrome(ui, egui::Margin::symmetric(8, 2)))
            .show(ui, |ui| self.ribbon(ui));
        Panel::bottom("status")
            .frame(chrome(ui, egui::Margin::symmetric(8, 2)))
            .show(ui, |ui| self.status_bar(ui));
        CentralPanel::no_frame().show(ui, |ui| self.dock_area(ui));
        self.follow_properties();
        self.prompts(&ctx);
        self.about(&ctx);
        self.shortcuts_window(&ctx);
        self.file_drops(&ctx);
        self.next_drop();
        self.image_window(&ctx);
        self.sync_pictures(&ctx);
        self.sync_models(ui.visuals().dark_mode);
        self.recovery_window(&ctx);
        // One undo step per finished edit: not while dragging a value or typing.
        let busy = ctx.egui_is_using_pointer() || ctx.egui_wants_keyboard_input();
        if !busy {
            self.commit();
        }
        if self.toolpath.running.is_some() {
            self.poll_generation();
            ctx.request_repaint_after(generation::POLL);
        }
        if self.poll_simulation(&ctx) {
            ctx.request_repaint_after(generation::POLL);
        }
        self.poll_updates(&ctx);
        self.poll_text_fonts(&ctx);
        if self.poll_model_reading() {
            ctx.request_repaint_after(generation::POLL);
        }
        self.track_first_picked();
        if !busy
            && self.drag.handle.is_none()
            && let Some(wait) = self.live_preview(Instant::now())
        {
            ctx.request_repaint_after(wait);
        }
        if !busy && let Some(wait) = self.recovery_tick(Instant::now()) {
            ctx.request_repaint_after(wait);
        }
        if !busy && let Some(wait) = self.watch_disk(Instant::now()) {
            ctx.request_repaint_after(wait);
        }
        self.update_title(&ctx);
    }
}

impl crate::i18n::Lang for ChiploadApp {
    fn lang(&self) -> crate::i18n::Language {
        self.prefs.language
    }
}

impl ChiploadApp {
    /// A message from the library crates (an error, a report line) in the app's language
    /// and display units (docs/specs/translations.md §7).
    pub(crate) fn say(&self, message: &chipload_i18n::Message) -> String {
        use crate::i18n::Lang;
        message.render(self.lang(), self.prefs.units.into())
    }
}

/// The boards of the setups that aren't active, for the view's ghosts
/// (docs/specs/setups.md §5): none with one setup.
fn ghost_boards(job: &chipload_job::Job) -> Vec<(chipload_core::Point, chipload_core::Point)> {
    let active = job.active_setup_id();
    job.setups
        .iter()
        .filter(|s| Some(s.id) != active)
        .map(|s| job.for_setup(s.id).stock_bounds())
        .collect()
}

/// The contours only another setup's operations cut (every shape of them picked by one,
/// none by the active setup's): drawn as ghosts (docs/specs/setups.md §5). None with one
/// setup.
fn ghost_contours(
    job: &chipload_job::Job,
    contours: &[chipload_core::chain::Contour],
) -> BTreeSet<usize> {
    if job.setups.len() < 2 {
        return BTreeSet::new();
    }
    let active = job.active_setup_id();
    let picks = |mine: bool, shape: &chipload_job::Shape| {
        job.operations.iter().any(|op| {
            let here = job.part_of(op).map(|p| p.setup) == active;
            here == mine
                && op.selects_shapes()
                && op.select().resolved(&job.drawing).may_pick(shape)
        })
    };
    let shape = |id: u64| job.drawing.shapes.iter().find(|s| s.id == id);
    contours
        .iter()
        .filter(|c| {
            let shapes: Vec<&chipload_job::Shape> = c
                .parts
                .iter()
                .filter_map(|p| p.segment.shape.and_then(shape))
                .collect();
            !shapes.is_empty()
                && shapes.iter().all(|s| picks(false, s))
                && !shapes.iter().any(|s| picks(true, s))
        })
        .map(|c| c.id)
        .collect()
}
