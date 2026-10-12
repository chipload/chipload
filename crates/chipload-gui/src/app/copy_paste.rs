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
        let (ids, imported) = self.deletable_shapes();
        if ids.is_empty() {
            let why = if imported {
                "Imported shapes can't be copied; only drawn ones"
            } else {
                "Select drawn shapes to copy"
            };
            self.set_status(why.into(), STATUS_SECONDS);
            return None;
        }
        let edit = self.edit.as_ref()?;
        match shapes_to_copy(&edit.settings, &ids) {
            Ok(shapes) => {
                let text = clipboard_text(&shapes);
                let n = shapes.len();
                self.shape_clipboard = shapes;
                let s = if n == 1 { "" } else { "s" };
                self.set_status(format!("Copied {n} shape{s}"), STATUS_SECONDS);
                Some(text)
            }
            Err(err) => {
                self.error(vec![err.to_string()]);
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
                    self.set_status("The clipboard holds no shapes".into(), STATUS_SECONDS);
                    return false;
                }
            },
            None => self.shape_clipboard.clone(),
        };
        if shapes.is_empty() {
            self.set_status("Copy drawn shapes first (Ctrl+C)".into(), STATUS_SECONDS);
            return false;
        }
        let at = self.cursor.filter(|_| !self.three_d);
        let Some(edit) = self.edit.as_mut() else {
            return false;
        };
        match paste_shapes(&edit.settings, &shapes, at) {
            Ok((job, ids)) => {
                edit.settings.drawing = job.drawing;
                self.drawing_changed(&ids);
                let names: Vec<String> = ids.iter().map(|id| format!("S{id}")).collect();
                self.set_status(format!("Pasted {}", names.join(", ")), STATUS_SECONDS);
                true
            }
            Err(err) => {
                self.error(vec!["Cannot paste:".into(), err.to_string()]);
                false
            }
        }
    }

    /// Mirror the selected drawn shapes in place, about the middle of the box round them.
    pub fn mirror_selected(&mut self, mirror: Mirror) -> bool {
        let (ids, imported) = self.deletable_shapes();
        if ids.is_empty() {
            let why = if imported {
                "Imported shapes can't be mirrored; only drawn ones"
            } else {
                "Select drawn shapes to mirror"
            };
            self.set_status(why.into(), STATUS_SECONDS);
            return false;
        }
        let Some(edit) = self.edit.as_mut() else {
            return false;
        };
        match mirror_shapes(&edit.settings, &ids, mirror) {
            Ok(job) => {
                edit.settings.drawing = job.drawing;
                self.shape_draft = None;
                self.drawing_changed(&ids);
                let mut note = String::new();
                if imported {
                    note = " (imported shapes in the selection stay as they are)".into();
                }
                self.set_status(format!("Mirrored{note}"), STATUS_SECONDS);
                true
            }
            Err(err) => {
                self.error(vec!["Cannot mirror:".into(), err.to_string()]);
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
