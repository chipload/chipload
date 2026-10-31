// SPDX-License-Identifier: GPL-3.0-or-later
//! Texts in the app (docs/specs/text.md §5): Draw > Text, then a click where it starts; its
//! properties make its letters again, as one undo step.

use chipload_core::Point;
use chipload_core::chain::ShapeFrom;
use chipload_job::Text;
use chipload_pipeline::texts::{NewText, add_text, remake, remove_text};

use super::*;

/// A new text's capital height, mm.
pub(super) const NEW_TEXT_HEIGHT_MM: f64 = 10.0;
/// What a new text says until it is typed over.
pub(super) const NEW_TEXT: &str = "Text";

impl ChiploadApp {
    /// Draw > Text: the next click on the canvas places a text there.
    pub fn start_text(&mut self) {
        if !self.can_draw() {
            return;
        }
        if self.three_d {
            self.view_top(); // placed by a click in top view, as drawing is
        }
        self.enter_edit_mode();
        self.sketch = None;
        self.measure = None;
        self.placing_text = true;
        self.set_status(t!(self, "texts-place-hint"), STATUS_SECONDS);
    }

    /// Placing a text: the next canvas click places it.
    pub fn placing_text(&self) -> bool {
        self.placing_text
    }

    /// A new text with its lower-left corner at `at`, selected and open in Properties with
    /// its text field ready to type into, as one undo step. Returns its id.
    pub fn place_text_at(&mut self, at: Point) -> Option<u64> {
        self.placing_text = false;
        self.commit();
        let edit = self.edit.as_ref()?;
        let mut new = NewText::new(NEW_TEXT, NEW_TEXT_HEIGHT_MM);
        (new.x, new.y) = at;
        match add_text(&edit.settings, new, &*self.font_source()) {
            Ok((job, report)) => {
                if let Some(edit) = self.edit.as_mut() {
                    edit.settings = job;
                }
                self.drawing_changed(&[]);
                self.selection = CanvasSelection::Source(ShapeFrom::Text(report.id));
                self.focus = Focus::Text(report.id);
                self.focus_text_field = true;
                self.commit();
                let mut lines = Report::of_messages(report.messages.clone());
                lines.push(Tone::Info, t!(self, "texts-added-next"));
                self.set_report(lines);
                Some(report.id)
            }
            Err(err) => {
                self.error_report(Report::failed(
                    t!(self, "texts-write-failed"),
                    err.message(),
                ));
                None
            }
        }
    }

    /// Text `id` as its properties left it, written again, as one undo step.
    pub(super) fn set_text(&mut self, id: u64, text: &Text) -> bool {
        let Some(edit) = self.edit.as_ref() else {
            return false;
        };
        let mut job = edit.settings.clone();
        let Some(slot) = job.drawing.texts.iter_mut().find(|t| t.id == id) else {
            return false;
        };
        *slot = Text { id, ..text.clone() };
        match remake(&edit.settings, &job, id, &*self.font_source()) {
            Ok((job, report)) => {
                if let Some(edit) = self.edit.as_mut() {
                    edit.settings = job;
                }
                self.drawing_changed(&[]);
                self.selection = CanvasSelection::Source(ShapeFrom::Text(id));
                self.commit();
                self.set_report(Report::of_messages(report.messages));
                true
            }
            Err(err) => {
                self.error_report(Report::failed(
                    t!(self, "texts-write-failed"),
                    err.message(),
                ));
                false
            }
        }
    }

    /// Remove text `id` and its letters, as one undo step.
    pub fn remove_text(&mut self, id: u64) -> bool {
        let Some(edit) = self.edit.as_mut() else {
            return false;
        };
        match remove_text(&edit.settings, id) {
            Ok(job) => {
                edit.settings = job;
                self.drawing_changed(&[]);
                self.selection = CanvasSelection::Nothing;
                if self.focus == Focus::Text(id) {
                    self.focus = Focus::Job;
                }
                self.commit();
                self.set_status(t!(self, "texts-removed", id = id), STATUS_SECONDS);
                true
            }
            Err(err) => {
                self.error_report(Report::of_error(err.message()));
                false
            }
        }
    }
}
