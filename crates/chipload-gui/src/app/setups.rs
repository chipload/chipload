// SPDX-License-Identifier: GPL-3.0-or-later
//! Setups in the window (docs/specs/setups.md §7): added, made active, turned on and off,
//! renamed and removed; parts moved between them. The pipeline's rules
//! (`chipload_pipeline::setups`) decide; the result goes back into the edited project.

use chipload_job::Job;
use chipload_pipeline::PipelineError;

use super::{ChiploadApp, Focus};
#[cfg(test)]
use crate::i18n::Lang;
use crate::i18n::t;

impl ChiploadApp {
    /// Apply `change` to the edited project; on success its setups, parts and active ones
    /// go back into it as one undo step, else the reason is shown.
    fn change_setups(&mut self, change: impl FnOnce(&Job) -> Result<Job, PipelineError>) -> bool {
        let Some(job) = self.edited_job() else {
            return false;
        };
        match change(&job) {
            Ok(changed) => {
                if let Some(edit) = self.edit.as_mut() {
                    edit.job.setups = changed.setups;
                    edit.job.active_setup = changed.active_setup;
                    edit.job.parts = changed.parts;
                    edit.job.active_part = changed.active_part;
                    // Depths through the stock that followed a new board (the same
                    // operations, so their side marks stay right).
                    if changed.operations.len() == edit.job.operations.len() {
                        edit.job.operations = changed.operations;
                    }
                }
                self.commit();
                true
            }
            Err(err) => {
                self.error(vec![t!(self, "app-setup-refused", error = err.to_string())]);
                false
            }
        }
    }

    /// The setup whose sheet is under screen point `pos` (docs/specs/parts-and-setups.md
    /// §7), in the top view.
    pub(super) fn sheet_under(&self, pos: eframe::egui::Pos2) -> Option<u64> {
        if !self.view.is_top() {
            return None;
        }
        let p = self.view.to_world(pos);
        self.scene.sheet_at(p).map(|s| s.setup)
    }

    /// Setup `id`'s sheet clicked at `pos`: the setup becomes the active one and its board
    /// moves into place, the view keeping the point under the pointer there (§7).
    pub(super) fn activate_sheet(&mut self, id: u64, _pos: eframe::egui::Pos2) {
        // The boards stay in their row: the view follows (`activate_setup`).
        self.activate_setup(id);
        self.show_part(None);
    }

    /// Part `id` cut from `stock`, None for any board (docs/specs/parts-and-setups.md §4):
    /// recorded as one undo step; nothing moves.
    pub fn set_part_stock(&mut self, id: u64, stock: Option<chipload_job::PartStock>) {
        let Some(edit) = self.edit.as_ref() else {
            return;
        };
        match chipload_pipeline::part_stock::set_part_stock(&edit.job, id, stock) {
            Ok(job) => {
                if let Some(edit) = self.edit.as_mut() {
                    edit.job = job;
                }
                self.commit();
            }
            Err(err) => {
                self.error(vec![t!(self, "app-setup-refused", error = err.to_string())]);
            }
        }
    }

    /// Part `id` moved to a setup of its stock, made like its board when none has it
    /// (docs/specs/parts-and-setups.md §4): one undo step, the status saying where.
    pub fn move_to_stock_board(&mut self, id: u64) {
        let Some(edit) = self.edit.as_ref() else {
            return;
        };
        match chipload_pipeline::part_stock::move_to_stock_board(&edit.job, id) {
            Ok((job, moved)) => {
                let text = moved
                    .message(&job, id)
                    .map(|m| m.render(self.prefs.language, self.prefs.units.into()));
                if let Some(edit) = self.edit.as_mut() {
                    edit.job = job;
                }
                self.commit();
                if let Some(text) = text {
                    self.set_status(text, super::STATUS_SECONDS);
                }
            }
            Err(err) => {
                self.error(vec![t!(self, "app-setup-refused", error = err.to_string())]);
            }
        }
    }

    /// A new, empty setup, a board like the active one's; it becomes active and its form
    /// opens. Parts are placed on it from the Browser.
    pub fn add_setup(&mut self) -> Option<u64> {
        let mut added = None;
        self.change_setups(|job| {
            chipload_pipeline::setups::add_setup(job, None, None).map(|(job, id)| {
                added = Some(id);
                job
            })
        });
        if let Some(id) = added {
            self.focus = Focus::Setup(id);
        }
        added
    }

    /// A new setup that is the back of setup `front`, the same board turned over left to
    /// right (docs/specs/two-sided.md §3); it and its part become active, its form opens.
    /// The window asks with *Two-sided…* (two-sided-wizard.md §4); tests add one directly.
    #[cfg(test)]
    pub fn add_back_side(&mut self, front: u64) -> Option<u64> {
        let mut added = None;
        self.change_setups(|job| {
            chipload_pipeline::setups::add_back_side(
                job,
                front,
                chipload_job::Turn::LeftRight,
                None,
            )
            .map(|(job, id, _)| {
                added = Some(id);
                job
            })
        });
        if let Some(id) = added {
            self.focus = Focus::Setup(id);
            self.display.turned = true;
        }
        added
    }

    /// Change setup `id`'s board: the back of another, its own, how it turns, how it is
    /// put back. Its board and zero follow the front's at once.
    pub fn set_board(&mut self, id: u64, change: chipload_pipeline::setups::BoardChange) {
        self.change_setups(|job| chipload_pipeline::setups::set_board(job, id, change));
        if let Some(edit) = self.edit.as_mut() {
            edit.job.follow_thickness();
        }
    }

    /// Dowel holes for back setup `id`: two circles drawn on the turn axis and a holes
    /// operation first in the front (docs/specs/two-sided.md §6), one undo step. The window
    /// asks with *Two-sided…*; tests add them directly.
    #[cfg(test)]
    pub fn add_dowels(&mut self, id: u64) {
        let Some(job) = self.edited_job() else {
            return;
        };
        let dowels = chipload_pipeline::dowels::Dowels::default();
        match chipload_pipeline::dowels::add_dowels(&job, id, dowels, self.lang()) {
            Ok((changed, at)) => {
                if let Some(edit) = self.edit.as_mut() {
                    edit.job = changed;
                }
                self.commit();
                self.focus = Focus::Op(at);
            }
            Err(err) => {
                self.error(vec![t!(self, "app-setup-refused", error = err.to_string())]);
            }
        }
    }

    /// Remove back setup `id`'s dowel holes: the front's operation and its circles, one
    /// undo step (docs/specs/two-sided-wizard.md §3).
    pub fn remove_dowels(&mut self, id: u64) {
        let Some(job) = self.edited_job() else {
            return;
        };
        match chipload_pipeline::dowels::remove_dowels(&job, id) {
            Ok(changed) => {
                if let Some(edit) = self.edit.as_mut() {
                    edit.job = changed;
                }
                self.commit();
                self.refresh_drawing();
            }
            Err(err) => {
                self.error(vec![t!(self, "app-setup-refused", error = err.to_string())]);
            }
        }
    }

    /// Place back setup `id`'s dowel holes again where they go now, one undo step
    /// (docs/specs/two-sided-wizard.md §5).
    pub fn place_dowels_again(&mut self, id: u64) {
        let Some(job) = self.edited_job() else {
            return;
        };
        match chipload_pipeline::dowels::place_dowels_again(&job, id) {
            Ok(changed) => {
                if let Some(edit) = self.edit.as_mut() {
                    edit.job = changed;
                }
                self.commit();
                self.refresh_drawing();
                self.set_report(super::Report::info([t!(self, "app-dowels-placed")]));
            }
            Err(err) => {
                self.error(vec![t!(self, "app-setup-refused", error = err.to_string())]);
            }
        }
    }

    /// Make setup `id` the one shown, simulated and previewed. A back is simulated turned
    /// over, back face up; a front face up (docs/specs/two-sided.md §10.2).
    pub fn activate_setup(&mut self, id: u64) {
        // Where its board is drawn now, in the row: the view moves by as much, so every
        // board stays where it is on screen (the user, 2026-11-20).
        let moved = self
            .edit
            .as_ref()
            .and_then(|e| super::row_offsets(&e.job).into_iter().find(|r| r.0 == id))
            .map(|r| r.1);
        if self.change_setups(|job| chipload_pipeline::setups::activate_setup(job, id)) {
            if let Some(by) = moved {
                self.shift_setups_view(by);
            }
            self.display.turned = self
                .edit
                .as_ref()
                .and_then(|e| e.job.setup(id))
                .is_some_and(|s| s.back_of.is_some());
        }
    }

    /// The setups' view moved with their row by `by` (drawing mm): what was at a point is now
    /// `by` short of it, so the boards stay on screen. The view itself while a setup's space
    /// is shown, else the one kept for it.
    fn shift_setups_view(&mut self, by: (f64, f64)) {
        use super::display::Space;
        if self.display.three_d {
            return;
        }
        let shift = |view: &mut crate::view::View| {
            let at = view.to_screen((0.0, 0.0));
            view.keep_at((-by.0, -by.1), at);
        };
        if self.space() == Space::Setup {
            shift(&mut self.view);
        } else if let Some(&(scale, offset)) = self.display.views.get(&Space::Setup) {
            let mut kept = self.view;
            kept.scale = scale;
            kept.offset = offset;
            shift(&mut kept);
            self.display
                .views
                .insert(Space::Setup, (kept.scale, kept.offset));
        }
    }

    /// Turn setup `id` on or off.
    pub fn toggle_setup(&mut self, id: u64) {
        let enabled = self
            .edit
            .as_ref()
            .and_then(|e| e.job.setup(id))
            .map(|s| !s.enabled);
        self.change_setups(|job| chipload_pipeline::setups::set_setup(job, id, None, enabled));
    }

    /// Move part `part` into setup `id`.
    pub fn move_part_to_setup(&mut self, part: u64, id: u64) {
        self.change_setups(|job| chipload_pipeline::setups::move_part_to_setup(job, part, id));
    }

    /// Remove setup `id` and its parts, when none of them has operations: those are moved
    /// or deleted first, so nothing is lost by accident. The last setup stays.
    pub fn remove_setup(&mut self, id: u64) {
        let has_operations = self.edit.as_ref().is_some_and(|e| {
            e.job
                .operations
                .iter()
                .any(|entry| e.job.setups_of_part(entry.part()) == [id])
        });
        if has_operations {
            self.error(vec![t!(self, "app-setup-has-operations")]);
            return;
        }
        if self.change_setups(|job| {
            chipload_pipeline::setups::remove_setup(job, id, true).map(|(job, _)| job)
        }) {
            self.focus = Focus::Job;
        }
    }
}
