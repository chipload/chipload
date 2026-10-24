// SPDX-License-Identifier: GPL-3.0-or-later
//! Import and export of the tool library and machine profiles
//! (docs/specs/library-import-export.md §6): the buttons' dialogs, the import preview and
//! its buttons. The rules are in `chipload_job::library`.

use chipload_job::library::{
    Collection, ImportPlan, MachineLibrary, ToolLibrary, export, import_into, import_plan,
    read_import, update_library, write_export,
};
use chipload_pipeline::library::{machines_path, tools_path, unknown_posts};

use super::user_library::LibraryKind;
use super::*;

/// A file read for import, waiting for the user's choice.
#[derive(Debug, Clone)]
pub(super) struct PendingImport {
    kind: LibraryKind,
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
    kind: LibraryKind,
) -> Option<ImportChoice> {
    let pending = pending.filter(|p| p.kind == kind)?;
    let mut choice = None;
    ui.separator();
    ui.strong(t!(
        ui,
        "library-transfer-import-from",
        file = pending.file.display().to_string()
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

fn list_name(kind: LibraryKind) -> &'static str {
    match kind {
        LibraryKind::Tools => ToolLibrary::LIST,
        LibraryKind::Machines => MachineLibrary::LIST,
    }
}

fn plan_for<L: Collection>(library: &L, file: &L) -> (ImportPlan, Vec<Message>) {
    let plan = import_plan(library, file);
    let lines = plan.preview_messages(L::LIST);
    (plan, lines)
}

impl ChiploadApp {
    /// Export… : all entries of the library to a file the user picks.
    pub(super) fn export_library_dialog(&mut self, kind: LibraryKind) {
        let suggested = match kind {
            LibraryKind::Tools => "chipload-tools.json",
            LibraryKind::Machines => "chipload-machines.json",
        };
        let filter = t!(self, "library-transfer-library-files");
        if let Some(file) = super::dialogs::save_dialog(Path::new(suggested), &filter, "json") {
            self.export_library(kind, &file);
        }
    }

    /// Import… : read a file the user picks and show what importing it would do.
    pub(super) fn import_library_dialog(&mut self, kind: LibraryKind) {
        if let Some(file) = rfd::FileDialog::new()
            .add_filter(t!(self, "library-transfer-library-files"), &["json", "bak"])
            .pick_file()
        {
            self.preview_library_import(kind, &file);
        }
    }

    /// Write every entry of the library to `file`; the result shows in the window.
    pub fn export_library(&mut self, kind: LibraryKind, file: &Path) -> bool {
        self.library_state.error = None;
        self.reload_library();
        let written = match kind {
            LibraryKind::Tools => export(&self.library.tools, &[])
                .and_then(|out| write_export(&out, file).map(|()| out.tools.len())),
            LibraryKind::Machines => export(&self.library.machines, &[])
                .and_then(|out| write_export(&out, file).map(|()| out.machines.len())),
        };
        match written {
            Ok(count) => {
                let file = file.display().to_string();
                let text = match kind {
                    LibraryKind::Tools => {
                        t!(
                            self,
                            "library-transfer-exported-tools",
                            count = count,
                            file = file
                        )
                    }
                    LibraryKind::Machines => {
                        t!(
                            self,
                            "library-transfer-exported-machines",
                            count = count,
                            file = file
                        )
                    }
                };
                self.library_state.message = Some((kind, text));
                true
            }
            Err(err) => {
                self.library_state.error = Some(self.say(&err.message()));
                false
            }
        }
    }

    /// Read `file` and keep it waiting with what importing it would do.
    pub fn preview_library_import(&mut self, kind: LibraryKind, file: &Path) -> bool {
        self.library_state.error = None;
        self.library_state.message = None;
        self.library_state.import = None;
        self.reload_library();
        let read = match kind {
            LibraryKind::Tools => read_import::<ToolLibrary>(file).map(|from| {
                let (plan, lines) = plan_for(&self.library.tools, &from);
                (ImportFile::Tools(from), plan, lines)
            }),
            LibraryKind::Machines => read_import::<MachineLibrary>(file).map(|from| {
                let (plan, mut lines) = plan_for(&self.library.machines, &from);
                lines.extend(unknown_posts(&from, self.library.posts.as_deref()));
                (ImportFile::Machines(from), plan, lines)
            }),
        };
        match read {
            Ok((entries, plan, lines)) => {
                self.library_state.import = Some(PendingImport {
                    kind,
                    file: file.to_path_buf(),
                    entries,
                    lines,
                    differing: !plan.differing.is_empty(),
                });
                true
            }
            Err(err) => {
                self.library_state.error = Some(self.say(&err.message()));
                false
            }
        }
    }

    /// Import the waiting file (`Keep` or `Replace`), or drop it (`Cancel`).
    pub(super) fn finish_library_import(&mut self, choice: ImportChoice) -> bool {
        let Some(pending) = self.library_state.import.take() else {
            return false;
        };
        let replace = match choice {
            ImportChoice::Cancel => return false,
            ImportChoice::Keep => false,
            ImportChoice::Replace => true,
        };
        let Some(dir) = self.settings_dir.clone() else {
            self.library_state.error = Some(t!(self, "library-transfer-no-settings-folder"));
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
        };
        self.reload_library();
        match done {
            Ok(plan) => {
                let lines: Vec<String> = plan
                    .report_messages(list_name(pending.kind), replace)
                    .iter()
                    .map(|line| self.say(line))
                    .collect();
                self.library_state.message = Some((pending.kind, lines.join("\n")));
                true
            }
            Err(err) => {
                self.library_state.error = Some(self.say(&err.message()));
                false
            }
        }
    }
}
