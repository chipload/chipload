// SPDX-License-Identifier: GPL-3.0-or-later
//! G-code exported (all, a part, a setup) and the project saved.

use std::path::Path;

use chipload_job::save_job;
use chipload_pipeline::sheet::{SheetInfo, write_sheet_for};
use chipload_pipeline::write_gcode_files;

use super::ChiploadApp;
use super::report::{Report, Tone};
use super::{STATUS_SECONDS, file_name};
use crate::editor::EditJob;
use crate::i18n::{Lang, t};

impl ChiploadApp {
    /// Write the checked G-code of the last generation, and its setup sheet when the
    /// setting asks for one (docs/specs/setup-sheet.md §4).
    pub fn export_gcode(&mut self, path: &Path) -> bool {
        if self.toolpath.result.is_none() {
            self.error(vec![t!(self, "app-generate-before-export")]);
            return false;
        }
        // Never a preview of one part: every part is planned first, then written there
        // (docs/specs/active-part-preview.md §2).
        if self.toolpath.part.is_some() {
            if self.start_generation_of(true, true) {
                self.toolpath.export_path_when_done = Some(path.to_path_buf());
                self.set_status(t!(self, "app-planning-then-export"), STATUS_SECONDS);
            }
            return false;
        }
        self.write_export(path, Vec::new())
    }

    /// Write the G-code of part `id` alone, as if every other part were switched off, and
    /// its setup sheet (docs/specs/part-export.md §4): now if the toolpath shown is exactly
    /// that part's, else once it is planned. Returns whether it was written now.
    pub fn export_part(&mut self, id: u64, path: &Path) -> bool {
        let Some(name) = self
            .edit
            .as_ref()
            .and_then(|e| e.job.part(id))
            .map(|p| p.name.clone())
        else {
            return false;
        };
        let current = self.toolpath.result.is_some()
            && !self.toolpath.stale
            && self.toolpath.running.is_none();
        if current && self.toolpath.part == Some(id) {
            return self.write_export(path, vec![name]);
        }
        if self.start_generation_scoped(true, Some(id)) {
            self.toolpath.export_path_when_done = None;
            self.toolpath.export_when_done = false;
            self.toolpath.export_part_when_done = Some((id, path.to_path_buf()));
            let status = t!(self, "app-planning-part-then-export", name = name);
            self.set_status(status, STATUS_SECONDS);
        }
        false
    }

    /// Write the G-code of setup `id` and its setup sheet (docs/specs/setups.md §7), named
    /// `-S<id>` when there are several: the setup is made the active one, then written now
    /// if the toolpath shown is all of it, else once it is planned. Returns whether it was
    /// written now.
    pub fn export_setup(&mut self, id: u64, path: &Path) -> bool {
        if self.edit.as_ref().and_then(|e| e.job.setup(id)).is_none() {
            return false;
        }
        if self.edit.as_ref().and_then(|e| e.job.active_setup_id()) != Some(id) {
            self.activate_setup(id);
        }
        let shown = self
            .toolpath
            .job
            .as_ref()
            .is_some_and(|job| job.active_setup_id() == Some(id));
        let current = self.toolpath.result.is_some()
            && !self.toolpath.stale
            && self.toolpath.running.is_none()
            && self.toolpath.part.is_none()
            && shown;
        if current {
            return self.write_export(path, Vec::new());
        }
        if self.start_generation_of(true, true) {
            self.toolpath.export_when_done = false;
            self.toolpath.export_part_when_done = None;
            self.toolpath.export_path_when_done = Some(path.to_path_buf());
            self.set_status(t!(self, "app-planning-then-export"), STATUS_SECONDS);
        }
        false
    }

    /// Write the last generation's G-code files, and its setup sheet when the setting asks
    /// for one; `parts` names the parts when only those were planned.
    fn write_export(&mut self, path: &Path, parts: Vec<String>) -> bool {
        let Some(result) = &self.toolpath.result else {
            self.error(vec![t!(self, "app-generate-before-export")]);
            return false;
        };
        // With several setups the result is the active one's: its files say so
        // (`-S2-1-T1`), as `run` names them (docs/specs/setups.md §4).
        let setup = self
            .toolpath
            .job
            .as_ref()
            .filter(|job| job.setups.len() > 1)
            .and_then(|job| job.active_setup_id());
        let named;
        let result = match setup {
            Some(id) => {
                let mut copy = result.clone();
                for file in &mut copy.files {
                    let tail = file.suffix.take().unwrap_or_default();
                    file.suffix = Some(format!("-S{id}{tail}"));
                }
                named = copy;
                &named
            }
            None => result,
        };
        let written = match write_gcode_files(path, &result.files) {
            Ok(written) => written,
            Err(err) => {
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
        };
        let sheet = match (&self.toolpath.job, self.prefs.setup_sheet) {
            (Some(job), true) => {
                // In the app's language and display units (docs/specs/translations.md
                // step 3).
                let info = SheetInfo {
                    project: self
                        .file
                        .path
                        .as_deref()
                        .map_or_else(|| t!(self, "export-untitled-project"), file_name),
                    written_by: format!("chipload {}", env!("CARGO_PKG_VERSION")),
                    units: self.prefs.units,
                    language: self.lang(),
                    parts,
                };
                match write_sheet_for(job, result, path, &info) {
                    Ok(sheet) => Some(sheet),
                    Err(err) => {
                        // The file system's own words.
                        self.error(vec![
                            t!(
                                self,
                                "app-sheet-failed",
                                path = chipload_pipeline::sheet::setup_sheet_path(job, path)
                                    .display()
                                    .to_string()
                            ),
                            err.to_string(),
                        ]);
                        return false;
                    }
                }
            }
            _ => None,
        };
        let shown = path.display().to_string();
        let setup_note = setup.and_then(|id| {
            let job = self.toolpath.job.as_ref()?;
            let name = job.setup(id)?.name.clone();
            Some(t!(self, "app-exported-setup", name = name))
        });
        if written.len() > 1 {
            let mut report = Report::default();
            report.push(
                Tone::Good,
                t!(self, "app-exported-n-files", count = written.len()),
            );
            for file in &result.files {
                report.push_message(Tone::Info, file.describe_message(path), None);
            }
            if let Some(sheet) = &sheet {
                report.push(
                    Tone::Info,
                    t!(
                        self,
                        "app-sheet-for-operator",
                        path = sheet.display().to_string()
                    ),
                );
            }
            self.set_report(report);
            let count = written.len();
            let text = match &sheet {
                Some(sheet) => t!(
                    self,
                    "app-exported-files-sheet",
                    count = count,
                    sheet = file_name(sheet),
                    path = shown
                ),
                None => t!(self, "app-exported-files", count = count, path = shown),
            };
            self.set_status(text, STATUS_SECONDS);
        } else {
            let text = match &sheet {
                Some(sheet) => t!(
                    self,
                    "app-exported-sheet",
                    path = shown,
                    sheet = file_name(sheet)
                ),
                None => t!(self, "app-exported", path = shown),
            };
            self.set_status(text, STATUS_SECONDS);
        }
        if let Some(note) = setup_note
            && let Some((text, _)) = &self.status
        {
            let text = format!("{text} {note}");
            self.set_status(text, STATUS_SECONDS);
        }
        true
    }

    /// Save the project (current schema, every field explicit).
    pub fn save_project(&mut self, path: &Path) -> bool {
        self.commit();
        let Some(edit) = &self.edit else {
            return false;
        };
        let job = match edit.to_job() {
            Ok(job) => job,
            Err(problems) => {
                self.error_report(Report::of_problems(
                    &t!(self, "app-cannot-save-yet"),
                    &problems,
                ));
                return false;
            }
        };
        if let Err(err) = save_job(&job, path) {
            self.error_report(Report::failed(
                t!(
                    self,
                    "app-could-not-save",
                    path = path.display().to_string()
                ),
                err.message(),
            ));
            return false;
        }
        let saved = EditJob::from_job(&job);
        self.edit = Some(saved.clone());
        self.history.committed = Some(saved.clone());
        self.file.saved = Some(saved);
        self.file.path = Some(path.to_path_buf());
        self.remember_disk();
        self.clear_recovery();
        let text = t!(self, "app-saved", path = path.display().to_string());
        self.set_status(text, STATUS_SECONDS);
        self.record_recent(path);
        true
    }
}
