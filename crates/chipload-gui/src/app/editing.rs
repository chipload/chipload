// SPDX-License-Identifier: GPL-3.0-or-later
//! Operations and parts: added, placed, moved, duplicated, turned on and off.

use std::collections::BTreeSet;

use chipload_core::Point;

use super::ChiploadApp;
use super::report::Report;
use super::{Focus, STATUS_SECONDS};
use crate::editor::{CanvasSelection, OpKind};
use crate::i18n::t;

impl ChiploadApp {
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
        // A surface without shapes is the whole board: the setup's
        // (docs/specs/surface-levels.md §5).
        if kind == OpKind::Surface && matches!(&selection, CanvasSelection::Nothing)
            || matches!(&selection, CanvasSelection::Shapes(ids) if ids.is_empty())
        {
            let setup = self.edit.as_ref()?.job.active_setup_id().unwrap_or(1);
            return self.face_board(setup);
        }
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

    /// A board surface first in setup `setup`, with the widest endmill and its library
    /// values for the setup's material, selected, as one undo step
    /// (docs/specs/surface-levels.md §5). Refused, with the reason, without an endmill.
    pub fn face_board(&mut self, setup: u64) -> Option<usize> {
        self.commit();
        let tool = chipload_pipeline::surface::surface_tool(&self.edit.as_ref()?.job);
        let cutting = tool.and_then(|id| self.tool_cutting(id));
        let edit = self.edit.as_mut()?;
        match chipload_pipeline::surface::add_board_surface(
            &mut edit.job,
            setup,
            None,
            None,
            cutting.as_ref(),
        ) {
            Ok(index) => {
                self.activate_setup(setup);
                self.focus = Focus::Op(index);
                self.commit();
                Some(index)
            }
            Err(err) => {
                self.error_report(Report::failed(
                    t!(self, "app-face-board-failed"),
                    err.message(),
                ));
                None
            }
        }
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

    /// Part `id`'s instances made `grid`'s (docs/specs/instances.md §5), at its pitch, or
    /// with `gap`, at the pitch that leaves that much between neighbours' cuts.
    pub fn set_copies_grid(&mut self, id: u64, mut grid: chipload_job::Copies, gap: Option<f64>) {
        self.commit();
        let Some(edit) = &self.edit else {
            return;
        };
        if let Some(gap) = gap {
            // The part alone, without its instances, to measure one cut.
            let mut part = edit.clone();
            for p in &mut part.job.parts {
                p.instances.clear();
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
            let current = (grid.dx, grid.dy);
            match chipload_pipeline::copies::pitch_for_gap(&job, id, gap, current) {
                Ok((dx, dy)) => (grid.dx, grid.dy) = (dx, dy),
                Err(err) => {
                    self.error_report(Report::failed(
                        t!(self, "app-pitch-needs-plan"),
                        err.message(),
                    ));
                    return;
                }
            }
        }
        let problems = grid.problems("grid");
        if !problems.is_empty() {
            self.error_report(Report::of_problems(
                &t!(self, "app-project-incomplete"),
                &problems,
            ));
            return;
        }
        if let Some(part) = self
            .edit
            .as_mut()
            .and_then(|e| e.job.parts.iter_mut().find(|p| p.id == id))
        {
            part.instances = grid.instances();
        }
        self.commit();
        let (units, unit) = (
            self.prefs.units,
            chipload_pipeline::units::length_unit(self.prefs.units),
        );
        let text = t!(
            self,
            "app-pitch-set",
            dx = crate::draw::fixed(grid.dx, units),
            dy = format!("{} {unit}", crate::draw::fixed(grid.dy, units)),
            gap = format!("{} {unit}", crate::draw::fixed(gap.unwrap_or(0.0), units))
        );
        self.set_status(text, STATUS_SECONDS);
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
        let Some(edit) = self.edit.as_mut() else {
            return;
        };
        let board = edit.is_board_surface(from) || edit.is_board_surface(to);
        if edit.move_op(from, to) {
            self.focus = Focus::Op(to);
            self.commit();
        } else if board {
            self.set_status(t!(self, "app-board-surface-stays"), STATUS_SECONDS);
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
        let Some(edit) = self.edit.as_mut() else {
            return;
        };
        let board = edit.is_board_surface(from);
        if let Some(at) = edit.move_op_to_part(from, id) {
            self.focus = Focus::Op(at);
            self.commit();
        } else if board {
            self.set_status(t!(self, "app-board-surface-stays"), STATUS_SECONDS);
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
}
