// SPDX-License-Identifier: GPL-3.0-or-later
//! File dialogs, typed-path prompts, the About window and the window title.

use super::*;

/// Where the user guide is published; None until the repository is public
/// (docs/specs/user-guide.md §5).
const USER_GUIDE_URL: Option<&str> = None;

impl ChiploadApp {
    /// Help > User guide (docs/specs/user-guide.md §6): the published guide once there is
    /// one; until then, where to find it.
    pub(super) fn open_user_guide(&mut self, ctx: &egui::Context) {
        match USER_GUIDE_URL {
            Some(url) => ctx.open_url(egui::OpenUrl::new_tab(url)),
            None => self.set_report(super::report::Report::info([
                "The user guide comes with each release as chipload-guide-<version>.zip \
                 (open index.html). From the source: packaging/docs/build-book.sh."
                    .to_string(),
            ])),
        }
    }

    // -- dialogs

    pub(super) fn open_dialog(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Chipload projects", &[PROJECT_EXTENSION, "json"])
            .add_filter("DXF drawings", &["dxf"])
            .pick_file()
        {
            self.open(&path);
        }
    }

    pub(super) fn import_dialog(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("DXF drawings", &["dxf"])
            .pick_file()
        {
            self.import_dxf(&path, None);
        }
    }

    pub(super) fn ask_new(&mut self) {
        if self.guard(Pending::New) {
            self.new_project();
        }
    }

    pub(super) fn ask_open(&mut self) {
        if self.guard(Pending::OpenDialog) {
            self.open_dialog();
        }
    }

    pub(super) fn ask_import(&mut self) {
        if self.guard(Pending::ImportDialog) {
            self.import_dialog();
        }
    }

    pub(super) fn ask_update(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("DXF drawings", &["dxf"])
            .pick_file()
        {
            self.update_from(&path, None);
        }
    }

    /// File > Add DXF to project… (docs/specs/drawing-sources.md §2).
    pub(super) fn ask_add_dxf(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("DXF drawings", &["dxf"])
            .pick_file()
        {
            self.add_dxf_from(&path, None);
        }
    }

    /// Save the project, asking where when it has no file yet (or `save_as`). True when it
    /// was saved: not when the dialog was cancelled or saving failed.
    pub(super) fn ask_save(&mut self, save_as: bool) -> bool {
        if self.edit.is_none() {
            return false;
        }
        let current = self
            .job_path
            .clone()
            .filter(|p| p.extension().is_some_and(|e| e == PROJECT_EXTENSION));
        if !save_as && let Some(path) = current {
            return self.save_project(&path);
        }
        let suggested = self.default_project_path();
        match save_dialog(&suggested, "Chipload projects", PROJECT_EXTENSION) {
            Some(path) => self.save_project(&path),
            None => false,
        }
    }

    pub(super) fn ask_export(&mut self) {
        if self.result.is_none() {
            self.error(vec!["Generate (F5) before exporting G-code.".into()]);
            return;
        }
        let suggested = self.default_export_path();
        if let Some(path) = save_dialog(&suggested, "G-code", "nc") {
            self.export_gcode(&path);
        }
    }

    pub(super) fn copy_selector(&mut self, ctx: &egui::Context) {
        if let Some(text) = self.selector_json() {
            ctx.copy_text(text);
            self.set_status("Selector copied to the clipboard".into(), STATUS_SECONDS);
        }
    }

    pub(super) fn about(&mut self, ctx: &egui::Context) {
        let mut open = self.about_open;
        egui::Window::new("About Chipload")
            .open(&mut open)
            .default_size([640.0, 520.0])
            .show(ctx, |ui| {
                ui.label(chipload_pipeline::notices::short_notice());
                if cfg!(target_os = "linux") {
                    ui.label(
                        "The Linux AppImage also contains libxkbcommon and libxcb-xkb \
                         (MIT-style licences); their copyright files are in \
                         usr/share/doc/chipload/bundled inside the AppImage.",
                    );
                }
                ui.separator();
                egui::CollapsingHeader::new("GNU General Public License v3.0").show(ui, |ui| {
                    ScrollArea::vertical()
                        .id_salt("license-text")
                        .max_height(300.0)
                        .show(ui, |ui| {
                            ui.monospace(chipload_pipeline::notices::LICENSE);
                        });
                });
                egui::CollapsingHeader::new("Third-party licences")
                    .default_open(true)
                    .show(ui, |ui| {
                        ScrollArea::vertical()
                            .id_salt("third-party-text")
                            .max_height(300.0)
                            .show(ui, |ui| {
                                ui.monospace(chipload_pipeline::notices::THIRD_PARTY);
                            });
                    });
            });
        self.about_open = open;
    }

    pub(super) fn prompts(&mut self, ctx: &egui::Context) {
        let Some(prompt) = self.prompt.clone() else {
            return;
        };
        match prompt {
            Prompt::Units(path, purpose) => {
                egui::Window::new("Drawing units")
                    .collapsible(false)
                    .resizable(false)
                    .show(ctx, |ui| {
                        ui.label(format!(
                            "{} does not say which units it uses.",
                            file_name(&path)
                        ));
                        ui.label("What units was it drawn in?");
                        ui.horizontal(|ui| {
                            for (label, units) in [("mm", Units::Mm), ("inch", Units::Inch)] {
                                if ui.button(label).clicked() {
                                    match purpose {
                                        UnitsFor::NewProject => {
                                            self.import_dxf(&path, Some(units));
                                        }
                                        UnitsFor::Update(id) => {
                                            self.update_dxf_from(id, &path, Some(units));
                                        }
                                        UnitsFor::Add => {
                                            self.add_dxf_from(&path, Some(units));
                                        }
                                    }
                                }
                            }
                            if ui.button("Cancel").clicked() {
                                self.prompt = None;
                                self.error(vec![
                                    "Drawing not imported: its units are unknown.".into(),
                                ]);
                            }
                        });
                    });
            }
            Prompt::Discard(action) => {
                // In the middle of the window, over a dimmed app, so it can't be missed.
                let mut answer: Option<Unsaved> = None;
                let modal = egui::Modal::new(egui::Id::new("unsaved-changes")).show(ctx, |ui| {
                    ui.set_width(360.0);
                    ui.heading("Unsaved changes");
                    ui.add_space(4.0);
                    ui.label("The project has changes that aren't saved. Save them first?");
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        let accent = ui.visuals().selection.bg_fill;
                        let on_accent = ui.visuals().selection.stroke.color;
                        if ui
                            .add(
                                egui::Button::new(RichText::new("Save").color(on_accent))
                                    .fill(accent),
                            )
                            .on_hover_text("Save, then go on")
                            .clicked()
                        {
                            answer = Some(Unsaved::Save);
                        }
                        if ui
                            .button("Don't save")
                            .on_hover_text("Throw the changes away and go on")
                            .clicked()
                        {
                            answer = Some(Unsaved::Discard);
                        }
                        if ui.button("Cancel").clicked() {
                            answer = Some(Unsaved::Cancel);
                        }
                    });
                });
                if modal.should_close() && answer.is_none() {
                    answer = Some(Unsaved::Cancel);
                }
                match answer {
                    Some(Unsaved::Save) => {
                        self.prompt = None;
                        // Cancelled or failed: stay, with the changes (an error says why).
                        if self.ask_save(false) {
                            self.run_pending(action, ctx);
                        }
                    }
                    Some(Unsaved::Discard) => {
                        self.prompt = None;
                        self.saved.clone_from(&self.edit); // nothing left to protect
                        self.run_pending(action, ctx);
                    }
                    Some(Unsaved::Cancel) => self.prompt = None,
                    None => {}
                }
            }
            Prompt::Path(purpose, text) => self.path_prompt(ctx, purpose, text),
            Prompt::LibraryName(kind, text) => self.library_name_prompt(ctx, kind, text),
        }
    }

    pub(super) fn path_prompt(
        &mut self,
        ctx: &egui::Context,
        purpose: PathPurpose,
        mut text: String,
    ) {
        let (title, label) = match purpose {
            PathPurpose::Open => ("Open path", "A .chipload project or a .dxf:"),
            PathPurpose::Save => ("Save project", "Save the project to:"),
            PathPurpose::Export => ("Export G-code", "Export the checked G-code to:"),
            PathPurpose::Update => ("Update drawing", "The DXF:"),
            PathPurpose::Image => ("Import image", "A picture (PNG, JPEG or BMP):"),
            PathPurpose::AddDxf => ("Add DXF", "The DXF to add to the project:"),
            PathPurpose::UpdateDxf(_) => ("Update DXF", "The new version of the DXF:"),
        };
        let mut done = false;
        let mut chosen: Option<PathBuf> = None;
        egui::Window::new(title).collapsible(false).show(ctx, |ui| {
            ui.label(label);
            ui.add(TextEdit::singleline(&mut text).desired_width(460.0));
            ui.horizontal(|ui| {
                if ui.button("OK").clicked() {
                    chosen = Some(PathBuf::from(text.trim()));
                    done = true;
                }
                if ui.button("Cancel").clicked() {
                    done = true;
                }
            });
        });
        self.prompt = (!done).then_some(Prompt::Path(purpose, text));
        let Some(path) = chosen else {
            return;
        };
        match purpose {
            PathPurpose::Open => {
                if self.guard(Pending::Open(path.clone())) {
                    self.open(&path);
                }
            }
            PathPurpose::Save => {
                self.save_project(&path);
            }
            PathPurpose::Export => {
                self.export_gcode(&path);
            }
            PathPurpose::Update => {
                self.update_from(&path, None);
            }
            PathPurpose::Image => {
                self.import_image(&path);
            }
            PathPurpose::AddDxf => {
                self.add_dxf_from(&path, None);
            }
            PathPurpose::UpdateDxf(id) => {
                self.update_dxf_from(Some(id), &path, None);
            }
        }
    }

    pub(super) fn update_title(&mut self, ctx: &egui::Context) {
        let mut parts = vec!["Chipload".to_string()];
        if self.edit.is_some() {
            let name = self
                .job_path
                .as_deref()
                .map_or("new project".to_string(), file_name);
            parts.push(if self.dirty() {
                format!("{name} *")
            } else {
                name
            });
        }
        let title = parts.join(" — ");
        if title != self.title {
            ctx.send_viewport_cmd(ViewportCommand::Title(title.clone()));
            self.title = title;
        }
    }
}

/// What the unsaved-changes question was answered.
enum Unsaved {
    Save,
    Discard,
    Cancel,
}

pub(super) fn save_dialog(suggested: &Path, filter: &str, ext: &str) -> Option<PathBuf> {
    let mut dialog = rfd::FileDialog::new().add_filter(filter, &[ext]);
    if let Some(name) = suggested.file_name() {
        dialog = dialog.set_file_name(name.to_string_lossy());
    }
    if let Some(dir) = suggested.parent().filter(|d| !d.as_os_str().is_empty()) {
        dialog = dialog.set_directory(dir);
    }
    dialog.save_file()
}
