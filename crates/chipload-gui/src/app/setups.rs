// SPDX-License-Identifier: GPL-3.0-or-later
//! Setups in the window (docs/specs/setups.md §7): added, made active, turned on and off,
//! renamed and removed; parts moved between them. The pipeline's rules
//! (`chipload_pipeline::setups`) decide; the result goes back into the edited project.

use chipload_job::Job;
use chipload_pipeline::PipelineError;

use super::{ChiploadApp, Focus};
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
                    edit.settings.setups = changed.setups;
                    edit.settings.active_setup = changed.active_setup;
                    edit.settings.parts = changed.parts;
                    edit.settings.active_part = changed.active_part;
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

    /// Make setup `id` the one shown, simulated and previewed.
    pub fn activate_setup(&mut self, id: u64) {
        self.change_setups(|job| chipload_pipeline::setups::activate_setup(job, id));
    }

    /// Turn setup `id` on or off.
    pub fn toggle_setup(&mut self, id: u64) {
        let enabled = self
            .edit
            .as_ref()
            .and_then(|e| e.settings.setup(id))
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
            e.ops.iter().any(|entry| {
                e.settings
                    .part(entry.op.part())
                    .is_some_and(|p| p.setup == id)
            })
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
