// SPDX-License-Identifier: GPL-3.0-or-later
//! Undo and redo: the project as last recorded, and the steps either way.

use super::*;

/// Undo and redo: the project as last recorded for undo, and the steps back and forward.
#[derive(Default)]
pub struct History {
    /// The project as last recorded for undo.
    pub(super) committed: Option<EditJob>,
    pub(super) undo: Vec<Option<EditJob>>,
    pub(super) redo: Vec<Option<EditJob>>,
}

impl ChiploadApp {
    /// Record the current project as one undo step if it changed since the last one. Called
    /// once the user has finished an edit (no drag or text entry in progress).
    pub fn commit(&mut self) {
        if self.edit != self.history.committed {
            self.follow_frames();
            self.history.undo.push(self.history.committed.take());
            self.history.redo.clear();
            self.history.committed.clone_from(&self.edit);
            // A shape no operation selects changes nothing that is cut: the toolpath
            // stands, and nothing is generated again (docs/specs/edit-and-simulate.md §5).
            if self.edit_keeps_cut() {
                self.refresh_problems();
                self.recovery.due = Some(Instant::now());
            } else {
                self.toolpath_out_of_date();
            }
        }
    }

    /// Frames that follow what they go round drawn again round it, as part of the edit
    /// being recorded (docs/specs/sign-maker.md §4); a note for each one let go of.
    fn follow_frames(&mut self) {
        let Some(edit) = self.edit.as_mut() else {
            return;
        };
        if edit.job.drawing.frames.is_empty() {
            return;
        }
        let (job, notes) = chipload_pipeline::frames::follow_frames(edit.job.clone());
        if job.drawing != edit.job.drawing {
            // Only the drawing changes: the operations and their marks stay.
            edit.job.drawing = job.drawing;
            self.refresh_drawing();
        }
        if !notes.is_empty() {
            let units = self.prefs.units.into();
            let text: Vec<String> = notes
                .iter()
                .map(|n| n.render(self.prefs.language, units))
                .collect();
            self.set_status(text.join(" "), STATUS_SECONDS);
        }
    }

    pub fn undo(&mut self) {
        self.commit();
        if let Some(previous) = self.history.undo.pop() {
            self.history.redo.push(self.edit.take());
            self.edit = previous;
            self.history.committed.clone_from(&self.edit);
            self.after_history_step();
        }
    }

    /// Take back the last edit for good (no redo): a drop the planner refused.
    pub(super) fn take_back_edit(&mut self) {
        if let Some(previous) = self.history.undo.pop() {
            self.edit = previous;
            self.history.committed.clone_from(&self.edit);
            self.after_history_step();
        }
    }

    pub fn redo(&mut self) {
        if let Some(next) = self.history.redo.pop() {
            self.history.undo.push(self.edit.take());
            self.edit = next;
            self.history.committed.clone_from(&self.edit);
            self.after_history_step();
        }
    }

    pub(super) fn after_history_step(&mut self) {
        self.shape_draft = None;
        // Undo or redo ends drawing; another tool stays for the next click.
        self.cancel_drawing();
        let len = self.edit.as_ref().map_or(0, |e| e.job.operations.len());
        if let Focus::Op(i) = self.focus
            && i >= len
        {
            self.focus = if len == 0 {
                Focus::Job
            } else {
                Focus::Op(len - 1)
            };
        }
        if self.edit_keeps_cut() {
            self.refresh_problems();
        } else {
            self.toolpath_out_of_date();
        }
    }
}
