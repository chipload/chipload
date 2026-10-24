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
            None => self.set_report(super::report::Report::info([t!(
                self,
                "dialogs-user-guide"
            )])),
        }
    }

    // -- dialogs

    pub(super) fn open_dialog(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter(
                t!(self, "dialogs-filter-projects"),
                &[PROJECT_EXTENSION, "json"],
            )
            .add_filter(t!(self, "dialogs-filter-dxf"), &["dxf"])
            .pick_file()
        {
            self.open(&path);
        }
    }

    pub(super) fn import_dialog(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter(t!(self, "dialogs-filter-dxf"), &["dxf"])
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
            .add_filter(t!(self, "dialogs-filter-dxf"), &["dxf"])
            .pick_file()
        {
            self.update_from(&path, None);
        }
    }

    /// File > Add DXF to project… (docs/specs/drawing-sources.md §2).
    pub(super) fn ask_add_dxf(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter(t!(self, "dialogs-filter-dxf"), &["dxf"])
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
        let filter = t!(self, "dialogs-filter-projects");
        match save_dialog(&suggested, &filter, PROJECT_EXTENSION) {
            Some(path) => self.save_project(&path),
            None => false,
        }
    }

    pub(super) fn ask_export(&mut self) {
        if self.result.is_none() {
            self.error(vec![t!(self, "app-generate-before-export")]);
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
            self.set_status(t!(self, "dialogs-selector-copied"), STATUS_SECONDS);
        }
    }

    pub(super) fn about(&mut self, ctx: &egui::Context) {
        let mut open = self.dialogs.about_open;
        egui::Window::new(t!(ctx, "dialogs-about-title"))
            .id(egui::Id::new("about-chipload"))
            .open(&mut open)
            .default_size([640.0, 520.0])
            .show(ctx, |ui| {
                ui.label(chipload_pipeline::notices::short_notice());
                if cfg!(target_os = "linux") {
                    ui.label(t!(ui, "dialogs-about-appimage"));
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
                egui::CollapsingHeader::new(t!(ui, "dialogs-third-party"))
                    .id_salt("third-party-licences")
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
        self.dialogs.about_open = open;
    }

    pub(super) fn prompts(&mut self, ctx: &egui::Context) {
        let Some(prompt) = self.dialogs.prompt.clone() else {
            return;
        };
        match prompt {
            Prompt::Units(path, purpose) => {
                egui::Window::new(t!(ctx, "dialogs-units-title"))
                    .id(egui::Id::new("drawing-units"))
                    .collapsible(false)
                    .resizable(false)
                    .show(ctx, |ui| {
                        ui.label(t!(ui, "dialogs-units-unknown", file = file_name(&path)));
                        ui.label(t!(ui, "dialogs-units-question"));
                        ui.horizontal(|ui| {
                            let inch = t!(ui, "dialogs-inch");
                            for (label, units) in [("mm", Units::Mm), (inch.as_str(), Units::Inch)]
                            {
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
                            if ui.button(t!(ui, "common-cancel")).clicked() {
                                self.dialogs.prompt = None;
                                self.error(vec![t!(ui, "dialogs-units-cancelled")]);
                            }
                        });
                    });
            }
            Prompt::Discard(action) => {
                // In the middle of the window, over a dimmed app, so it can't be missed.
                let mut answer: Option<Unsaved> = None;
                let modal = egui::Modal::new(egui::Id::new("unsaved-changes")).show(ctx, |ui| {
                    ui.set_width(360.0);
                    ui.heading(t!(ui, "dialogs-unsaved-title"));
                    ui.add_space(4.0);
                    ui.label(t!(ui, "dialogs-unsaved-body"));
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        let accent = ui.visuals().selection.bg_fill;
                        let on_accent = ui.visuals().selection.stroke.color;
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new(t!(ui, "common-save")).color(on_accent),
                                )
                                .fill(accent),
                            )
                            .on_hover_text(t!(ui, "dialogs-save-hover"))
                            .clicked()
                        {
                            answer = Some(Unsaved::Save);
                        }
                        if ui
                            .button(t!(ui, "dialogs-dont-save"))
                            .on_hover_text(t!(ui, "dialogs-dont-save-hover"))
                            .clicked()
                        {
                            answer = Some(Unsaved::Discard);
                        }
                        if ui.button(t!(ui, "common-cancel")).clicked() {
                            answer = Some(Unsaved::Cancel);
                        }
                    });
                });
                if modal.should_close() && answer.is_none() {
                    answer = Some(Unsaved::Cancel);
                }
                match answer {
                    Some(Unsaved::Save) => {
                        self.dialogs.prompt = None;
                        // Cancelled or failed: stay, with the changes (an error says why).
                        if self.ask_save(false) {
                            self.run_pending(action, ctx);
                        }
                    }
                    Some(Unsaved::Discard) => {
                        self.dialogs.prompt = None;
                        self.saved.clone_from(&self.edit); // nothing left to protect
                        self.run_pending(action, ctx);
                    }
                    Some(Unsaved::Cancel) => self.dialogs.prompt = None,
                    None => {}
                }
            }
            Prompt::ChangedOnDisk => self.changed_on_disk_prompt(ctx),
            Prompt::Path(purpose, text) => self.path_prompt(ctx, purpose, text),
            Prompt::LibraryName(kind, text) => self.library_name_prompt(ctx, kind, text),
        }
    }

    /// The project's file changed while there are unsaved changes
    /// (docs/specs/reload-changed-file.md §4): in the middle, as for unsaved changes.
    fn changed_on_disk_prompt(&mut self, ctx: &egui::Context) {
        let name = self.job_path.as_deref().map(file_name).unwrap_or_default();
        let mut load: Option<bool> = None;
        let modal = egui::Modal::new(egui::Id::new("changed-on-disk")).show(ctx, |ui| {
            ui.set_width(380.0);
            ui.heading(t!(ui, "dialogs-changed-title", file = name.as_str()));
            ui.add_space(4.0);
            ui.label(t!(ui, "dialogs-changed-body"));
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                let accent = ui.visuals().selection.bg_fill;
                let on_accent = ui.visuals().selection.stroke.color;
                let load_it = RichText::new(t!(ui, "dialogs-load-it")).color(on_accent);
                if ui
                    .add(egui::Button::new(load_it).fill(accent))
                    .on_hover_text(t!(ui, "dialogs-load-it-hover"))
                    .clicked()
                {
                    load = Some(true);
                }
                if ui
                    .button(t!(ui, "dialogs-keep-mine"))
                    .on_hover_text(t!(ui, "dialogs-keep-mine-hover"))
                    .clicked()
                {
                    load = Some(false);
                }
            });
        });
        if modal.should_close() && load.is_none() {
            load = Some(false);
        }
        match load {
            Some(true) => self.load_from_disk(),
            Some(false) => self.keep_mine(),
            None => {}
        }
    }

    pub(super) fn path_prompt(
        &mut self,
        ctx: &egui::Context,
        purpose: PathPurpose,
        mut text: String,
    ) {
        let (title, label) = match purpose {
            PathPurpose::Open => (
                t!(ctx, "dialogs-path-open-title"),
                t!(ctx, "dialogs-path-open"),
            ),
            PathPurpose::Save => (
                t!(ctx, "dialogs-path-save-title"),
                t!(ctx, "dialogs-path-save"),
            ),
            PathPurpose::Export => (
                t!(ctx, "dialogs-path-export-title"),
                t!(ctx, "dialogs-path-export"),
            ),
            PathPurpose::Update => (
                t!(ctx, "dialogs-path-update-title"),
                t!(ctx, "dialogs-path-update"),
            ),
            PathPurpose::Image => (
                t!(ctx, "dialogs-path-image-title"),
                t!(ctx, "dialogs-path-image"),
            ),
            PathPurpose::AddDxf => (
                t!(ctx, "dialogs-path-add-dxf-title"),
                t!(ctx, "dialogs-path-add-dxf"),
            ),
            PathPurpose::UpdateDxf(_) => (
                t!(ctx, "dialogs-path-update-dxf-title"),
                t!(ctx, "dialogs-path-update-dxf"),
            ),
        };
        let mut done = false;
        let mut chosen: Option<PathBuf> = None;
        egui::Window::new(title).collapsible(false).show(ctx, |ui| {
            ui.label(label);
            ui.add(TextEdit::singleline(&mut text).desired_width(460.0));
            ui.horizontal(|ui| {
                if ui.button(t!(ui, "common-ok")).clicked() {
                    chosen = Some(PathBuf::from(text.trim()));
                    done = true;
                }
                if ui.button(t!(ui, "common-cancel")).clicked() {
                    done = true;
                }
            });
        });
        self.dialogs.prompt = (!done).then_some(Prompt::Path(purpose, text));
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
                .map_or_else(|| t!(self, "dialogs-title-new-project"), file_name);
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
