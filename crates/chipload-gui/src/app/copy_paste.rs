// SPDX-License-Identifier: GPL-3.0-or-later
//! Copy, paste and mirror drawn shapes (docs/specs/small-items.md §4). The rules are the
//! pipeline's (`chipload_pipeline::shapes`); this only takes the selection and shows the
//! result.

use chipload_job::edit::Mirror;
use chipload_pipeline::shapes::{
    clipboard_text, mirror_shapes, paste_shapes, shapes_from_clipboard, shapes_to_copy,
};

use super::*;

impl ChiploadApp {
    /// Keep the selected drawn shapes for pasting. Returns the text for the system
    /// clipboard (egui only reports Ctrl+V when that holds text).
    pub fn copy_selected_shapes(&mut self) -> Option<String> {
        // A whole group copies its drawn shapes, and pastes as a group.
        let group = self.selected_group();
        let (ids, imported) = match group {
            Some(id) => {
                let drawn = self.edit.as_ref().map(|e| {
                    e.job
                        .drawing
                        .groups
                        .iter()
                        .find(|g| g.id == id)
                        .map(|g| g.shapes.clone())
                        .unwrap_or_default()
                });
                (drawn.unwrap_or_default(), false)
            }
            None => self.deletable_shapes(),
        };
        if ids.is_empty() {
            let why = if imported {
                t!(self, "copy-paste-imported-no-copy")
            } else {
                t!(self, "copy-paste-select-to-copy")
            };
            self.set_status(why, STATUS_SECONDS);
            return None;
        }
        let edit = self.edit.as_ref()?;
        match shapes_to_copy(&edit.job, &ids) {
            Ok(shapes) => {
                let text = clipboard_text(&shapes);
                let n = shapes.len();
                self.clipboard.group = group.is_some() && n > 1;
                self.clipboard.shapes = shapes;
                let status = t!(self, "copy-paste-copied", count = n);
                self.set_status(status, STATUS_SECONDS);
                Some(text)
            }
            Err(err) => {
                self.error_report(Report::of_error(err.message()));
                None
            }
        }
    }

    /// Paste: the shapes in `text` (the system clipboard) if it holds some, else (no text)
    /// the ones copied last. Centred on the cursor when it is over the canvas in top view.
    pub fn paste(&mut self, text: Option<&str>) -> bool {
        let shapes = match text {
            Some(text) => match shapes_from_clipboard(text) {
                Some(shapes) => shapes,
                None => {
                    self.set_status(t!(self, "copy-paste-no-shapes"), STATUS_SECONDS);
                    return false;
                }
            },
            None => self.clipboard.shapes.clone(),
        };
        let as_group = text.is_none() && self.clipboard.group;
        if shapes.is_empty() {
            self.set_status(t!(self, "copy-paste-copy-first"), STATUS_SECONDS);
            return false;
        }
        let at = self.cursor.filter(|_| !self.three_d);
        let Some(edit) = self.edit.as_mut() else {
            return false;
        };
        match paste_shapes(&edit.job, &shapes, at) {
            Ok((job, ids)) => {
                edit.adopt(job);
                self.drawing_changed(&ids);
                if as_group {
                    self.group_selected();
                }
                let names: Vec<String> = ids.iter().map(|id| format!("S{id}")).collect();
                let text = t!(self, "copy-paste-pasted", shapes = names.join(", "));
                self.set_status(text, STATUS_SECONDS);
                true
            }
            Err(err) => {
                self.error_report(Report::failed(
                    t!(self, "copy-paste-cannot-paste"),
                    err.message(),
                ));
                false
            }
        }
    }

    /// Mirror the selected drawn shapes in place, about the middle of the box round them.
    pub fn mirror_selected(&mut self, mirror: Mirror) -> bool {
        let (ids, imported) = self.deletable_shapes();
        if ids.is_empty() {
            let why = if imported {
                t!(self, "copy-paste-imported-no-mirror")
            } else {
                t!(self, "copy-paste-select-to-mirror")
            };
            self.set_status(why, STATUS_SECONDS);
            return false;
        }
        let Some(edit) = self.edit.as_mut() else {
            return false;
        };
        match mirror_shapes(&edit.job, &ids, mirror) {
            Ok(job) => {
                edit.adopt(job);
                self.shape_draft = None;
                self.drawing_changed(&ids);
                let text = if imported {
                    t!(self, "copy-paste-mirrored-drawn")
                } else {
                    t!(self, "copy-paste-mirrored")
                };
                self.set_status(text, STATUS_SECONDS);
                true
            }
            Err(err) => {
                self.error_report(Report::failed(
                    t!(self, "copy-paste-cannot-mirror"),
                    err.message(),
                ));
                false
            }
        }
    }

    /// Ctrl+C and Ctrl+V reach egui as clipboard events, not keys: Ctrl+Shift+C copies the
    /// selector, Ctrl+C the shapes, Ctrl+V pastes what the clipboard holds.
    pub(super) fn clipboard_events(&mut self, ctx: &egui::Context) {
        let (events, shift) = ctx.input(|i| (i.events.clone(), i.modifiers.shift));
        for event in events {
            match event {
                egui::Event::Copy if shift => self.copy_selector(ctx),
                egui::Event::Copy => {
                    if let Some(text) = self.copy_selected_shapes() {
                        ctx.copy_text(text);
                    }
                }
                egui::Event::Paste(text) => {
                    self.paste(Some(&text));
                }
                _ => {}
            }
        }
    }
}

/// The drawn shapes copied last.
#[derive(Default)]
pub struct Clipboard {
    /// The drawn shapes copied last (layer, geometry), for Ctrl+V.
    pub(super) shapes: Vec<(String, Geometry)>,
    /// The shapes copied were a whole group: they paste as a new group
    /// (docs/specs/shape-groups.md, decision 6).
    pub(super) group: bool,
}
