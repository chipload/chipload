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

    /// A new setup, a board like the active one's, with a part of its own; both become
    /// active, and its form opens.
    pub fn add_setup(&mut self) -> Option<u64> {
        let mut added = None;
        self.change_setups(|job| {
            chipload_pipeline::setups::add_setup(job, None, None).map(|(job, id, _)| {
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
        if self.change_setups(|job| chipload_pipeline::setups::activate_setup(job, id)) {
            self.display.turned = self
                .edit
                .as_ref()
                .and_then(|e| e.job.setup(id))
                .is_some_and(|s| s.back_of.is_some());
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
                .any(|entry| e.job.part(entry.part()).is_some_and(|p| p.setup == id))
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
