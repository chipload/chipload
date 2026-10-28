// SPDX-License-Identifier: GPL-3.0-or-later
//! File > Export drawing as DXF (docs/specs/dxf-export.md §A6): the choices, then the file.
//! The DXF is made and checked by `chipload_pipeline::dxfs::export_drawing`.

use chipload_pipeline::dxfs::{ExportOptions, export_drawing};

use super::*;

/// What the export writes, kept for the session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DxfExportChoice {
    /// In inches (default: as the display shows lengths).
    pub inch: bool,
    /// Hidden groups and layers too.
    pub hidden: bool,
    /// Only the selected shapes.
    pub selection: bool,
}

impl ChiploadApp {
    /// Open the export's choices, inch as the display is.
    pub(super) fn ask_export_dxf(&mut self) {
        self.dxf_export.inch = self.units == chipload_job::Units::Inch;
        self.dxf_export.selection = false;
        self.dialogs.prompt = Some(Prompt::ExportDxf);
    }

    /// The shapes selected on the canvas, when the export is to write only those.
    fn selected_shape_ids(&self) -> Vec<u64> {
        match &self.selection {
            CanvasSelection::Shapes(ids) => ids.iter().copied().collect(),
            _ => Vec::new(),
        }
    }

    /// Write the drawing to `path` as the choices say; the report on the status bar.
    pub fn export_dxf(&mut self, path: &Path) -> bool {
        let Some(edit) = &self.edit else {
            return false;
        };
        let choice = self.dxf_export;
        let options = ExportOptions {
            inch: choice.inch,
            hidden: choice.hidden,
            layers: Vec::new(),
            shapes: if choice.selection {
                self.selected_shape_ids()
            } else {
                Vec::new()
            },
        };
        let (out, notes) = match export_drawing(&edit.settings, &options) {
            Ok(done) => done,
            Err(err) => {
                self.error(vec![
                    t!(self, "app-could-not-export-dxf"),
                    err.message().render(self.language, self.units.into()),
                ]);
                return false;
            }
        };
        let text = String::from_utf8_lossy(&out.bytes).into_owned();
        if let Err(err) = chipload_pipeline::write_atomically(path, &text) {
            // The file system's own words.
            self.error(vec![
                t!(
                    self,
                    "app-could-not-export",
                    path = path.display().to_string()
                ),
                err.to_string(),
            ]);
            return false;
        }
        let units = self.units.into();
        let mut status: Vec<String> = notes
            .iter()
            .map(|n| n.render(self.language, units))
            .collect();
        status.push(t!(
            self,
            "app-dxf-exported",
            path = path.display().to_string()
        ));
        self.set_status(status.join(" "), STATUS_SECONDS);
        true
    }

    /// The export's choices: inches, hidden shapes, only the selection; then a file.
    pub(super) fn export_dxf_prompt(&mut self, ctx: &egui::Context) {
        let mut open = true;
        let mut action = None;
        let has_selection = !self.selected_shape_ids().is_empty();
        egui::Window::new(t!(ctx, "dialogs-export-dxf-title"))
            .id(egui::Id::new("export-dxf"))
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .show(ctx, |ui| {
                ui.checkbox(&mut self.dxf_export.inch, t!(ui, "dialogs-export-dxf-inch"));
                ui.checkbox(
                    &mut self.dxf_export.hidden,
                    t!(ui, "dialogs-export-dxf-hidden"),
                );
                ui.add_enabled(
                    has_selection,
                    egui::Checkbox::new(
                        &mut self.dxf_export.selection,
                        t!(ui, "dialogs-export-dxf-selection"),
                    ),
                );
                ui.horizontal(|ui| {
                    if ui.button(t!(ui, "dialogs-export-dxf-save")).clicked() {
                        action = Some(false);
                    }
                    if ui.button(t!(ui, "dialogs-export-dxf-path")).clicked() {
                        action = Some(true);
                    }
                    if ui.button(t!(ui, "common-cancel")).clicked() {
                        self.dialogs.prompt = None;
                    }
                });
            });
        if !open {
            self.dialogs.prompt = None;
        }
        let suggested = self.default_project_path().with_extension("dxf");
        match action {
            Some(false) => {
                self.dialogs.prompt = None;
                if let Some(path) =
                    dialogs::save_dialog(&suggested, &t!(ctx, "dialogs-filter-dxf-file"), "dxf")
                {
                    self.export_dxf(&path);
                }
            }
            Some(true) => {
                self.dialogs.prompt = Some(Prompt::Path(
                    PathPurpose::ExportDxf,
                    suggested.display().to_string(),
                ));
            }
            None => {}
        }
    }
}
