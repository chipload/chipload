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
use chipload_job::edit::{Handle, SegmentDrag, handles, try_drag};
use chipload_job::library::Theme;
use chipload_job::{
    Direction, Drawing, Geometry, Job, Operation, Selector, Side, Source, Spindle, StockCorner,
    StockOrigin, Tabs, ToolType, Travel, Units, WorkZero, ZZero, load_job,
};
use chipload_pipeline::shapes::{add_shape, delete_shape, set_shape};
use chipload_pipeline::{PipelineError, RunResult};
use eframe::egui::{
    self, Align, CentralPanel, ComboBox, Grid, Key, Layout, Modifiers, Panel, RichText, ScrollArea,
    Sense, TextEdit, ViewportCommand,
};

use crate::draw::{Cursor, Sketch, SnapPoint, Snapper, Step, Tool, grid_step, snap_cursor};
use crate::editor::{CanvasSelection, EditJob, OpKind, geometry_summary, shapes_of, with_twins};
use crate::i18n::{Lang, t};
use crate::shape_form::{ShapeAction, ShapeDraft, imported_lines, shape_form};
use crate::view::{Highlight, Scene, StartMarker, ToolpathShape, View};
use chipload_i18n::Message;

mod app_settings;
mod box_drag;
mod browser;
mod browser_ops;
mod browser_row;
mod browser_setups;
mod canvas;
mod canvas_handles;
mod canvas_menu;
mod canvas_paint;
mod canvas_tools;
use canvas_tools::ActiveTool;
mod compass;
mod computer;
mod copy_paste;
mod corners;
mod dialogs;
mod disk;
mod display;
mod drawing;
mod dropping;
mod dxf_export;
mod editing;
mod export_all;
mod forms;
mod frames;
mod generation;
mod groups;
pub(crate) mod header;
mod history;
mod images;
mod inlay;
mod instances;
mod layout;
mod library_entries;
mod library_store;
mod library_transfer;
mod library_view;
mod materials;
mod measure;
mod menu;
mod models;
mod modify;
mod navigation;
mod nesting;
mod new_project;
mod opening;
mod panels;
mod passes;
mod playback;
mod prefs;
mod problems;
mod quick_menu;
mod recovery;
mod relief_wizard;
mod report;
mod ribbon;
mod rulers;
mod saving;
mod select_drag;
mod selection;
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
    /// Part `0`'s instance `1` (docs/specs/instances.md §4).
    Instance(u64, u32),
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
    /// *New project*: the board, machine and tools (docs/specs/new-project-dialog.md).
    NewProject(Box<new_project::NewProjectDialog>),
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
    /// An instance being moved or turned (docs/specs/instances.md §4).
    instance: Option<instances::InstanceDrag>,
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
    /// Where the Q menu is open (docs/specs/fewer-clicks.md §2).
    quick_menu: Option<egui::Pos2>,
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
    /// The Nest window, or a Nest again running (docs/specs/nesting.md §7).
    nest: Option<Box<nesting::NestDialog>>,
    /// *Export all setups…* running (docs/specs/parts-and-setups.md §8).
    export_all: Option<export_all::ExportAllRun>,
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
    /// The open relief's rim edges, kept until what they are made from changes.
    rim_preview: models::RimPreview,
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
            quick_menu: None,
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
            nest: None,
            export_all: None,
            drops: std::collections::VecDeque::new(),
            hidden_contours: BTreeSet::new(),
            drawing_hidden: None,
            picture_textures: images::PictureTextures::default(),
            model_surfaces: models::ModelSurfaces::default(),
            rim_preview: models::RimPreview::default(),
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
/// What *Add 3D model…* takes: STL files and pictures read as depth maps
/// (docs/specs/picture-relief.md §5).
pub(super) const MODEL_FILES: &[&str] = &[
    "stl", "STL", "png", "PNG", "jpg", "JPG", "jpeg", "JPEG", "bmp", "BMP",
];
/// Pictures read as depth maps.
pub(super) const DEPTH_MAP_FILES: &[&str] =
    &["png", "PNG", "jpg", "JPG", "jpeg", "JPEG", "bmp", "BMP"];

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
            Pending::New => self.ask_new_project(),
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
        self.follow_focus();
        self.prompts(&ctx);
        self.about(&ctx);
        self.shortcuts_window(&ctx);
        self.file_drops(&ctx);
        self.next_drop();
        self.image_window(&ctx);
        self.nest_window(&ctx);
        self.poll_export_all(&ctx);
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

/// How far apart the sheets lie, and the first from the active board, mm
/// (docs/specs/parts-and-setups.md §7).
const SHEET_GAP_MM: f64 = 50.0;

/// The setups that aren't active, as sheets in a row to the right of the active board, in
/// their order (a back after its front), their bottoms level with its bottom: each its
/// board and its instances' outlines, moved there for the view only
/// (docs/specs/parts-and-setups.md §7). None with one setup.
fn sheets(
    job: &chipload_job::Job,
    contours: &[chipload_core::chain::Contour],
) -> Vec<crate::view::Sheet> {
    let active = job.active_setup_id();
    let ((_, bottom), (right, _)) = job.stock_bounds();
    let mut x = right + SHEET_GAP_MM;
    let mut out = Vec::new();
    for setup in job.setups.iter().filter(|s| Some(s.id) != active) {
        let one = job.for_setup(setup.id);
        let ((x0, y0), (x1, y1)) = one.stock_bounds();
        let (dx, dy) = (x - x0, bottom - y0);
        let shift = |p: &chipload_core::Point| (p.0 + dx, p.1 + dy);
        let outlines = chipload_pipeline::parts::setup_outlines(&one, contours, setup.id)
            .into_iter()
            .map(|(points, closed)| {
                let mut moved: Vec<chipload_core::Point> = points.iter().map(shift).collect();
                if let (true, Some(&first)) = (closed, moved.first()) {
                    moved.push(first);
                }
                moved
            })
            .collect();
        out.push(crate::view::Sheet {
            setup: setup.id,
            name: setup.name.clone(),
            board: ((x0 + dx, y0 + dy), (x1 + dx, y1 + dy)),
            outlines,
            offset: (dx, dy),
        });
        x += x1 - x0 + SHEET_GAP_MM;
    }
    out
}

/// The contours only operations of parts not shown cut (every shape of them picked by one,
/// none by a part `here`): drawn as ghosts (docs/specs/setups.md §5; a part shown alone,
/// parts-and-setups.md §8a).
fn ghost_contours(
    job: &chipload_job::Job,
    contours: &[chipload_core::chain::Contour],
    here: impl Fn(u64) -> bool,
) -> BTreeSet<usize> {
    let picks = |mine: bool, shape: &chipload_job::Shape| {
        job.operations.iter().any(|op| {
            here(op.part()) == mine
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
