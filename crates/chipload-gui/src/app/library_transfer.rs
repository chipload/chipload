// SPDX-License-Identifier: GPL-3.0-or-later
//! Import and export of the tool library, machine profiles and materials
//! (docs/specs/library-import-export.md §6, docs/specs/materials.md): the buttons' dialogs,
//! the import preview and its buttons. The rules are in `chipload_job::library`.

use chipload_job::library::{
    Collection, ImportPlan, MachineLibrary, ToolLibrary, export, import_into, import_plan,
    read_import, update_library, write_export,
};
use chipload_job::materials::MaterialLibrary;
use chipload_pipeline::library::{machines_path, materials_path, tools_path, unknown_posts};

use super::user_library::LibraryKind;
use super::*;

/// A library file with an import and an export in the window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transfer {
    Tools,
    Machines,
    Materials,
}

impl From<LibraryKind> for Transfer {
    fn from(kind: LibraryKind) -> Self {
        match kind {
            LibraryKind::Tools => Transfer::Tools,
            LibraryKind::Machines => Transfer::Machines,
        }
    }
}

/// A file read for import, waiting for the user's choice.
#[derive(Debug, Clone)]
pub(super) struct PendingImport {
    kind: Transfer,
    file: PathBuf,
    entries: ImportFile,
    /// What the import would do, and notes (unknown post-processors).
    lines: Vec<Message>,
    differing: bool,
}

#[derive(Debug, Clone)]
enum ImportFile {
    Tools(ToolLibrary),
    Machines(MachineLibrary),
    Materials(MaterialLibrary),
}

impl PendingImport {
    /// What the import would do, as messages.
    pub(super) fn lines(&self) -> &[Message] {
        &self.lines
    }
}

/// What was clicked in the import preview.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ImportChoice {
    Keep,
    Replace,
    Cancel,
}

/// The import preview of a Settings window, if a file of its kind waits.
pub(super) fn import_preview(
    ui: &mut egui::Ui,
    pending: Option<&PendingImport>,
    kind: Transfer,
) -> Option<ImportChoice> {
    let pending = pending.filter(|p| p.kind == kind)?;
    let mut choice = None;
    ui.separator();
    ui.label(crate::style::bold(
        ui,
        t!(
            ui,
            "library-transfer-import-from",
            file = pending.file.display().to_string()
        ),
    ));
    let units = crate::units::display(ui.ctx()).into();
    for line in pending.lines() {
        ui.label(line.render(ui.lang(), units));
    }
    ui.horizontal(|ui| {
        if ui
            .button(t!(ui, "library-transfer-import"))
            .on_hover_text(t!(ui, "library-transfer-import-hover"))
            .clicked()
        {
            choice = Some(ImportChoice::Keep);
        }
        if pending.differing
            && ui
                .button(t!(ui, "library-transfer-import-replace"))
                .on_hover_text(t!(ui, "library-transfer-import-replace-hover"))
                .clicked()
        {
            choice = Some(ImportChoice::Replace);
        }
        if ui.button(t!(ui, "common-cancel")).clicked() {
            choice = Some(ImportChoice::Cancel);
        }
    });
    choice
}

fn list_name(kind: Transfer) -> &'static str {
    match kind {
        Transfer::Tools => ToolLibrary::LIST,
        Transfer::Machines => MachineLibrary::LIST,
        Transfer::Materials => MaterialLibrary::LIST,
    }
}

fn plan_for<L: Collection>(library: &L, file: &L) -> (ImportPlan, Vec<Message>) {
    let plan = import_plan(library, file);
    let lines = plan.preview_messages(L::LIST);
    (plan, lines)
}

impl ChiploadApp {
    /// Export… : all entries of the library to a file the user picks.
    pub(super) fn export_library_dialog(&mut self, kind: Transfer) {
        let suggested = match kind {
            Transfer::Tools => "chipload-tools.json",
            Transfer::Machines => "chipload-machines.json",
            Transfer::Materials => "chipload-materials.json",
        };
        let filter = t!(self, "library-transfer-library-files");
        if let Some(file) = super::dialogs::save_dialog(Path::new(suggested), &filter, "json") {
            self.export_library(kind, &file);
        }
    }

    /// Import… : read a file the user picks and show what importing it would do.
    pub(super) fn import_library_dialog(&mut self, kind: Transfer) {
        if let Some(file) = rfd::FileDialog::new()
            .add_filter(t!(self, "library-transfer-library-files"), &["json", "bak"])
            .pick_file()
        {
            self.preview_library_import(kind, &file);
        }
    }

    /// Write every entry of the library to `file`; the result shows in the window.
    pub fn export_library(&mut self, kind: impl Into<Transfer>, file: &Path) -> bool {
        let kind = kind.into();
        self.user.library_state.error = None;
        self.reload_library();
        let written = match kind {
            Transfer::Tools => export(&self.user.library.tools, &[])
                .and_then(|out| write_export(&out, file).map(|()| out.tools.len())),
            Transfer::Machines => export(&self.user.library.machines, &[])
                .and_then(|out| write_export(&out, file).map(|()| out.machines.len())),
            Transfer::Materials => export(&self.user.library.materials, &[])
                .and_then(|out| write_export(&out, file).map(|()| out.materials.len())),
        };
        match written {
            Ok(count) => {
                let file = file.display().to_string();
                let text = match kind {
                    Transfer::Tools => {
                        t!(
                            self,
                            "library-transfer-exported-tools",
                            count = count,
                            file = file
                        )
                    }
                    Transfer::Machines => {
                        t!(
                            self,
                            "library-transfer-exported-machines",
                            count = count,
                            file = file
                        )
                    }
                    Transfer::Materials => {
                        t!(
                            self,
                            "library-transfer-exported-materials",
                            count = count,
                            file = file
                        )
                    }
                };
                self.user.library_state.message = Some((kind, text));
                true
            }
            Err(err) => {
                self.user.library_state.error = Some(self.say(&err.message()));
                false
            }
        }
    }

    /// Read `file` and keep it waiting with what importing it would do.
    pub fn preview_library_import(&mut self, kind: impl Into<Transfer>, file: &Path) -> bool {
        let kind = kind.into();
        self.user.library_state.error = None;
        self.user.library_state.message = None;
        self.user.library_state.import = None;
        self.reload_library();
        let read = match kind {
            Transfer::Tools => read_import::<ToolLibrary>(file).map(|from| {
                let (plan, lines) = plan_for(&self.user.library.tools, &from);
                (ImportFile::Tools(from), plan, lines)
            }),
            Transfer::Machines => read_import::<MachineLibrary>(file).map(|from| {
                let (plan, mut lines) = plan_for(&self.user.library.machines, &from);
                lines.extend(unknown_posts(&from, self.user.library.posts.as_deref()));
                (ImportFile::Machines(from), plan, lines)
            }),
            Transfer::Materials => read_import::<MaterialLibrary>(file).map(|from| {
                let (plan, lines) = plan_for(&self.user.library.materials, &from);
                (ImportFile::Materials(from), plan, lines)
            }),
        };
        match read {
            Ok((entries, plan, lines)) => {
                self.user.library_state.import = Some(PendingImport {
                    kind,
                    file: file.to_path_buf(),
                    entries,
                    lines,
                    differing: !plan.differing.is_empty(),
                });
                true
            }
            Err(err) => {
                self.user.library_state.error = Some(self.say(&err.message()));
                false
            }
        }
    }

    /// Import the waiting file (`Keep` or `Replace`), or drop it (`Cancel`).
    pub(super) fn finish_library_import(&mut self, choice: ImportChoice) -> bool {
        let Some(pending) = self.user.library_state.import.take() else {
            return false;
        };
        let replace = match choice {
            ImportChoice::Cancel => return false,
            ImportChoice::Keep => false,
            ImportChoice::Replace => true,
        };
        let Some(dir) = self.user.settings_dir.clone() else {
            self.user.library_state.error = Some(t!(self, "library-transfer-no-settings-folder"));
            return false;
        };
        let done = match &pending.entries {
            ImportFile::Tools(from) => update_library(&tools_path(&dir), |l: &mut ToolLibrary| {
                Ok(import_into(l, from, replace))
            }),
            ImportFile::Machines(from) => {
                update_library(&machines_path(&dir), |l: &mut MachineLibrary| {
                    Ok(import_into(l, from, replace))
                })
            }
            ImportFile::Materials(from) => {
                update_library(&materials_path(&dir), |l: &mut MaterialLibrary| {
                    Ok(import_into(l, from, replace))
                })
            }
        };
        self.reload_library();
        match done {
            Ok(plan) => {
                let lines: Vec<String> = plan
                    .report_messages(list_name(pending.kind), replace)
                    .iter()
                    .map(|line| self.say(line))
                    .collect();
                self.user.library_state.message = Some((pending.kind, lines.join("\n")));
                true
            }
            Err(err) => {
                self.user.library_state.error = Some(self.say(&err.message()));
                false
            }
        }
    }
}
