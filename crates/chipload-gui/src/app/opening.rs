// SPDX-License-Identifier: GPL-3.0-or-later
//! Projects opened, created and imported; drawing files added and updated.

use std::path::{Path, PathBuf};

use chipload_dxf::{ImportError, shapes_from_file};
use chipload_job::{Units, load_job};
use chipload_pipeline::{PipelineError, empty_project, load_drawing, new_project};

use super::ChiploadApp;
use super::report::{DrawingFacts, Report};
use super::{
    ERROR_STATUS_SECONDS, Focus, Prompt, UnitsFor, file_name, ghost_contours, is_drawing_file,
    raw_schema_version, sheets,
};
use crate::draw::{Snapper, snapper};
use crate::editor::{CanvasSelection, EditJob};
use crate::i18n::t;
use crate::view::ContourShape;

impl ChiploadApp {
    /// A new project opens on its stock, where to start (the user, 2026-11-03: "When
    /// starting a new project we should open the stock setup first").
    fn open_stock(&mut self) {
        if self.edit.is_some() {
            self.focus = Focus::Stock;
            self.focus_thickness = true;
        }
    }

    /// Start editing `job` as a fresh document (no undo history).
    pub(super) fn start_editing(&mut self, job: EditJob, saved: bool, path: Option<PathBuf>) {
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
    pub(super) fn refresh_drawing(&mut self) {
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
                self.hidden_contours =
                    super::sources::hidden_contours(&job.drawing, &self.contours);
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
    pub(super) fn update_ghosts(&mut self) {
        // A part shown alone (docs/specs/parts-and-setups.md §8a): its board, no other
        // setup, every contour only another part cuts a ghost.
        if let Some((id, job)) = self
            .part_shown()
            .zip(self.edited_job())
            .and_then(|(id, job)| Some((id, chipload_pipeline::part_view::part_view(&job, id)?)))
        {
            self.scene.stock = Some(job.stock_bounds());
            self.scene.sheets.clear();
            self.scene.pieces.clear();
            self.scene.ghost_contours = ghost_contours(&job, &self.contours, |part| part == id);
            return;
        }
        let job = self.edited_job();
        self.scene.stock = job.as_ref().map(chipload_job::Job::stock_bounds);
        self.scene.sheets = job
            .as_ref()
            .map(|job| sheets(job, &self.contours))
            .unwrap_or_default();
        // The pieces of a setup cut in pieces (docs/specs/pieces.md §9); none while they
        // can't be worked out (the report says why once it generates).
        self.scene.pieces = job
            .as_ref()
            .and_then(|job| chipload_pipeline::pieces::piece_map_of(job, &self.contours).ok())
            .flatten()
            .map(|map| map.pieces)
            .unwrap_or_default();
        self.scene.ghost_contours = job
            .filter(|job| job.setups.len() > 1)
            .map(|job| {
                let active = job.active_setup_id();
                let here = |part| active.is_some_and(|s| job.part_on(part, s));
                ghost_contours(&job, &self.contours, here)
            })
            .unwrap_or_default();
    }

    // -- operations (also called directly by tests)

    /// Open a project, an older job, or a DXF or SVG (imported as a new project).
    pub fn open(&mut self, path: &Path) -> bool {
        if is_drawing_file(path)
            || super::dropping::dropped_kind(path) == Some(super::dropping::Dropped::Model)
        {
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
    pub(super) fn in_display_units(&self, job: chipload_job::Job) -> chipload_job::Job {
        match self.prefs.units {
            Units::Mm => job,
            Units::Inch => chipload_pipeline::with_inch_defaults(job),
        }
    }

    /// Start a new, unsaved project with no drawing (docs/specs/new-project.md), with what
    /// the last project was started with (docs/specs/new-project-dialog.md §4). *New
    /// project* in the window asks first ([`ChiploadApp::ask_new`]).
    pub fn new_project(&mut self) {
        self.reload_library();
        let start = self.remembered_start();
        self.create_project(empty_project(), &start, t!(self, "app-new-project"), None);
    }

    /// Make the new project `job` (empty, or an imported drawing) with `start`, and say what
    /// it started with; `next` is the report's last line (default: what to do next in an
    /// empty project).
    pub(super) fn create_project(
        &mut self,
        job: chipload_job::Job,
        start: &chipload_pipeline::library::ProjectStart,
        first: String,
        next: Option<String>,
    ) {
        self.dialogs.prompt = None;
        let mut job = self.in_display_units(job);
        let notes =
            match chipload_pipeline::library::start_project(&mut job, &self.user.library, start) {
                Ok(notes) => notes,
                // A choice the library no longer has: its defaults, and why.
                Err(err) => {
                    let mut notes = chipload_pipeline::library::apply_library(
                        &mut job,
                        &self.user.library,
                        None,
                    )
                    .unwrap_or_default();
                    notes.insert(0, err);
                    notes
                }
            };
        self.start_editing(EditJob::from_job(&job), false, None);
        self.open_stock();
        let mut lines = vec![first];
        lines.extend(
            notes
                .iter()
                .map(|n| t!(self, "app-started-with", what = self.say(n))),
        );
        lines.extend(self.user.library.problems.iter().map(|p| self.say(p)));
        lines.push(next.unwrap_or_else(|| t!(self, "app-new-project-next")));
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
    #[cfg(test)]
    pub fn import_dxf(&mut self, path: &Path, units: Option<Units>) -> bool {
        self.import_file(path, units, None)
    }

    /// [`ChiploadApp::import_dxf`] with an SVG's pixels per inch; an SVG without a real size
    /// and none given asks for them. Made at once with the remembered choices; the window
    /// asks first ([`ChiploadApp::ask_import_file`]).
    #[cfg(test)]
    pub fn import_file(&mut self, path: &Path, units: Option<Units>, px: Option<f64>) -> bool {
        let Some(imported) = self.read_new_drawing(path, units, px) else {
            return false;
        };
        self.reload_library();
        let start = self.remembered_start();
        self.create_from_import(path, imported, &start);
        true
    }

    /// Read a drawing for a new project; None after asking for its units or pixels per inch,
    /// or saying why it can't be read.
    pub(super) fn read_new_drawing(
        &mut self,
        path: &Path,
        units: Option<Units>,
        px: Option<f64>,
    ) -> Option<chipload_dxf::ImportedShapes> {
        self.dialogs.prompt = None;
        match shapes_from_file(path, units, px, 1.0) {
            Ok(imported) => Some(imported),
            Err(ImportError::ScaleMissing) => {
                self.ask_px_per_inch(path, UnitsFor::NewProject);
                None
            }
            Err(ImportError::UnitsMissing) => {
                self.dialogs.prompt = Some(Prompt::Units(path.to_path_buf(), UnitsFor::NewProject));
                self.set_report(Report::info([t!(
                    self,
                    "app-units-missing",
                    file = file_name(path)
                )]));
                None
            }
            Err(err) => {
                self.error_report(Report::failed(
                    t!(self, "app-cannot-import", file = file_name(path)),
                    err.message(),
                ));
                None
            }
        }
    }

    /// The new project from the drawing `imported` (read from `path`), with `start`.
    pub(super) fn create_from_import(
        &mut self,
        path: &Path,
        imported: chipload_dxf::ImportedShapes,
        start: &chipload_pipeline::library::ProjectStart,
    ) {
        let shapes = imported.shapes.len();
        let first = t!(
            self,
            "app-new-project-from",
            file = file_name(path),
            shapes = shapes
        );
        let next = t!(self, "app-import-next");
        self.create_project(new_project(imported), start, first, Some(next));
        // The styles' layer rules (docs/specs/fewer-clicks.md §4).
        self.layer_rules_on_import();
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
}
