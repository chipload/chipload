// SPDX-License-Identifier: GPL-3.0-or-later
//! The user's tool library and machine profiles (docs/specs/tool-library-and-machines.md
//! §8, library-in-browser.md): the library rows of a project tool and the machine, and
//! copying values into and out of the project. The Browser rows and the entries' properties
//! are in library_view.rs. The rules are in `chipload_job::library`; this only shows and
//! edits values.

use std::collections::BTreeMap;

use chipload_job::library::{AppSettings, load_settings, update_settings};
use chipload_job::library::{
    Cutting, LibraryError, LibraryTool, Link, LinkEntry, MachineProfile, machine_link, same_name,
    update_machines, update_tools,
};
use chipload_job::{Machine, Tool, ToolChange};
use chipload_pipeline::library::{Library, machines_path, tools_path};
use chipload_pipeline::{DEFAULT_MAX_OVERCUT, DEFAULT_RPM, DEFAULT_SAFE_Z, DEFAULT_TOOL_DIAMETER};

use super::forms::{feed, machine_rows, mm, safe_z_row, tool_rows, work_zero_rows};
use super::library_transfer::PendingImport;
use super::library_view::uid_key;
use super::*;
use crate::i18n::Lang;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LibraryKind {
    Tools,
    Machines,
}

/// What a library row works on: the project's machine, or one of its tools (by id).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveTarget {
    Machine,
    Tool(i64),
}

impl SaveTarget {
    fn kind(self) -> LibraryKind {
        match self {
            SaveTarget::Machine => LibraryKind::Machines,
            SaveTarget::Tool(_) => LibraryKind::Tools,
        }
    }
}

/// What the job form asks of the library.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LibraryRequest {
    UseMachine(String),
    /// Take the linked profile's machine values again, keeping the job values.
    UpdateMachine,
    /// Give project tool `id` the values of the named library tool.
    UseTool(i64, String),
    /// Copy a library tool into the project as a tool of its own
    /// (docs/specs/tool-per-operation.md §8).
    AddTool(String),
    /// Ask for a name, then save the project's machine or tool into the library.
    Save(SaveTarget),
    /// Save the project's machine or tool into the entry it is linked to
    /// (docs/specs/library-in-browser.md §3).
    SaveLinked(SaveTarget),
    /// Show a library entry, by uid, in the Browser and Properties.
    Show(LibraryKind, String),
    /// The project's tools that differ from their library tools take its values
    /// (docs/specs/tool-updates.md §3).
    UpdateAllTools,
}

/// The library rows of a project tool or the machine: the entries to choose from, and how
/// the project's values stand to the entry they came from.
pub(super) struct LibraryRows {
    target: SaveTarget,
    /// None: no settings folder, so no library.
    names: Option<Vec<String>>,
    /// The link, and the linked entry's uid while it is in the library.
    link: Option<(Link, Option<String>)>,
}

impl LibraryRows {
    pub(super) fn machine(job: &Job, library: Option<&Library>) -> Self {
        let names = library.map(|l| l.machines.machines.iter().map(|m| m.name.clone()).collect());
        let link = library.map(|l| {
            let link = machine_link(&l.machines, job);
            let uid = job
                .machine
                .library_uid
                .clone()
                .filter(|_| link.entry_name().is_some());
            (link, uid)
        });
        LibraryRows {
            target: SaveTarget::Machine,
            names,
            link,
        }
    }

    /// A project tool's rows; `link`: how it stands to its library tool, a speed from the
    /// setup's material aside (`project_tool_link`).
    pub(super) fn tool(tool: &Tool, library: Option<&Library>, link: Option<Link>) -> Self {
        let names = library.map(|l| l.tools.tools.iter().map(|t| t.name.clone()).collect());
        let link = link.map(|link| {
            let uid = tool
                .library_uid
                .clone()
                .filter(|_| link.entry_name().is_some());
            (link, uid)
        });
        LibraryRows {
            target: SaveTarget::Tool(tool.id),
            names,
            link,
        }
    }

    fn what(&self) -> LinkEntry {
        match self.target {
            SaveTarget::Machine => LinkEntry::Profile,
            SaveTarget::Tool(_) => LinkEntry::Tool,
        }
    }
}

#[cfg(test)]
impl LibraryRows {
    /// The entry the dropdown shows, while it is in the library.
    pub(super) fn entry_name(&self) -> Option<String> {
        self.link
            .as_ref()
            .and_then(|(link, _)| link.entry_name().map(str::to_string))
    }

    pub(super) fn note_text(&self) -> Option<String> {
        let units = chipload_i18n::Units::Mm;
        self.link
            .as_ref()
            .map(|(link, _)| status_text(crate::i18n::Language::En, link, self.what(), units))
    }
}

fn target_salt(target: SaveTarget) -> i64 {
    match target {
        SaveTarget::Machine => -1,
        SaveTarget::Tool(id) => id,
    }
}

/// The library rows of the project's machine or a project tool
/// (docs/specs/linked-machine.md §3, decision 7): the entry in a dropdown (another one is
/// used when picked), one status line with only the buttons that apply, and the rarer
/// choices behind ⋯.
pub(super) fn library_rows(
    ui: &mut egui::Ui,
    request: &mut Option<LibraryRequest>,
    rows: LibraryRows,
) {
    let target = rows.target;
    let what = rows.what();
    let use_entry = |name: String| match target {
        SaveTarget::Machine => LibraryRequest::UseMachine(name),
        SaveTarget::Tool(id) => LibraryRequest::UseTool(id, name),
    };
    let (label, choose) = match target {
        SaveTarget::Machine => (
            t!(ui, "user-library-profile"),
            t!(ui, "user-library-use-profile"),
        ),
        SaveTarget::Tool(_) => (t!(ui, "user-library-tool"), t!(ui, "user-library-use-tool")),
    };
    let linked = rows.link.as_ref().and_then(|(_, uid)| uid.clone());
    let current = match rows.link.as_ref().map(|(link, _)| link) {
        Some(Link::Same(name) | Link::Differs(name, _)) => Some(name.clone()),
        _ => None,
    };
    ui.label(label);
    ui.horizontal(|ui| {
        let names = rows.names.clone().unwrap_or_default();
        let shown = current.clone().unwrap_or(choose);
        let combo = ui.add_enabled_ui(!names.is_empty(), |ui| {
            ComboBox::from_id_salt(("library", target_salt(target)))
                .selected_text(shown)
                .show_ui(ui, |ui| {
                    for name in names {
                        let on = current.as_deref() == Some(name.as_str());
                        if ui.selectable_label(on, &name).clicked() && !on {
                            *request = Some(use_entry(name));
                        }
                    }
                })
        });
        if rows.names.is_none() {
            combo.response.on_disabled_hover_text(t!(
                ui,
                "user-library-no-settings-folder-set",
                var = chipload_pipeline::library::CONFIG_DIR_VAR
            ));
            return;
        }
        if linked.is_some() {
            ui.menu_button("⋯", |ui| {
                if ui
                    .button(t!(ui, "user-library-save-as-new"))
                    .on_hover_text(t!(ui, "user-library-save-as-new-hover"))
                    .clicked()
                {
                    *request = Some(LibraryRequest::Save(target));
                    ui.close();
                }
                if let Some(uid) = &linked
                    && ui
                        .button(t!(ui, "user-library-show-in-library"))
                        .on_hover_text(t!(ui, "user-library-show-in-library-hover"))
                        .clicked()
                {
                    *request = Some(LibraryRequest::Show(target.kind(), uid.clone()));
                    ui.close();
                }
            })
            .response
            .on_hover_text(t!(ui, "user-library-more-hover"));
        }
    });
    ui.end_row();
    let Some((link, _)) = rows.link else {
        return;
    };
    ui.label("");
    ui.vertical(|ui| {
        let units = crate::units::display(ui.ctx()).into();
        let status = status_text(ui.lang(), &link, what, units);
        ui.add(egui::Label::new(egui::RichText::new(status).weak()).wrap());
        if matches!(link, Link::Same(_)) {
            return;
        }
        ui.horizontal_wrapped(|ui| match &link {
            Link::Same(_) => {}
            Link::Differs(name, _) => {
                if ui
                    .button(t!(ui, "user-library-update-from-library"))
                    .on_hover_text(t!(ui, "user-library-update-from-library-hover"))
                    .clicked()
                {
                    *request = Some(match target {
                        SaveTarget::Machine => LibraryRequest::UpdateMachine,
                        SaveTarget::Tool(_) => use_entry(name.clone()),
                    });
                }
                if ui
                    .button(t!(ui, "user-library-save-to-library"))
                    .on_hover_text(match target {
                        SaveTarget::Machine => t!(ui, "user-library-save-to-linked-profile-hover"),
                        SaveTarget::Tool(_) => t!(ui, "user-library-save-to-linked-tool-hover"),
                    })
                    .clicked()
                {
                    *request = Some(LibraryRequest::SaveLinked(target));
                }
            }
            Link::None | Link::Missing(_) => {
                if ui
                    .button(t!(ui, "user-library-save-to-library-named"))
                    .on_hover_text(t!(ui, "user-library-save-to-library-hover"))
                    .clicked()
                {
                    *request = Some(LibraryRequest::Save(target));
                }
            }
        });
    });
    ui.end_row();
}

/// The status line under the dropdown: the same as the library, how it differs, or not
/// from it (the dropdown already names the entry).
pub(super) fn status_text(
    lang: crate::i18n::Language,
    link: &Link,
    what: LinkEntry,
    units: chipload_i18n::Units,
) -> String {
    match link {
        Link::Same(_) => t!(lang, "user-library-same"),
        Link::Differs(_, changes) => {
            let changes: Vec<String> = changes.iter().map(|c| c.render(lang, units)).collect();
            t!(lang, "user-library-differs", changes = changes.join(", "))
        }
        Link::None | Link::Missing(_) => link.message(what).render(lang, units),
    }
}

/// The library's state in the app: entries being edited, and what import and export said.
#[derive(Debug, Clone, Default)]
pub(super) struct LibraryState {
    /// Tools being edited and not saved yet, by uid.
    pub(super) tool_drafts: BTreeMap<String, LibraryTool>,
    /// Machine profiles being edited and not saved yet, by uid.
    pub(super) machine_drafts: BTreeMap<String, MachineProfile>,
    /// Materials being edited and not saved yet, by uid (docs/specs/materials.md §4).
    pub(super) material_drafts: BTreeMap<String, chipload_job::materials::Material>,
    pub(super) error: Option<String>,
    /// What the last import or export did, shown in that folder's properties.
    pub(super) message: Option<(LibraryKind, String)>,
    /// A file read for import, waiting for Import or Cancel.
    pub(super) import: Option<PendingImport>,
    /// A Library folder to open in the Browser on its next frame.
    pub(super) reveal: Option<LibraryKind>,
    /// The facts of the post shown in Properties, by name (docs/specs/this-computer.md §4).
    pub(super) post_facts: Option<(String, Result<Vec<String>, String>)>,
}

/// "New tool", or "New tool 2" … if that is taken.
pub(super) fn unique_name(base: &str, taken: impl Fn(&str) -> bool) -> String {
    let mut name = base.to_string();
    let mut n = 2;
    while taken(&name) {
        name = format!("{base} {n}");
        n += 1;
    }
    name
}

/// Which library entry a save goes to.
enum Pick {
    Name(String),
    Uid(String),
}

/// A refusal in the app's own words, already in its language.
fn refused(text: String) -> LibraryError {
    LibraryError::Refused(chipload_i18n::Value::Text(text))
}

/// Where the library is kept, and why it couldn't be read.
pub(super) fn library_header(ui: &mut egui::Ui, path: Option<&Path>, problems: &[Message]) {
    match path {
        Some(path) => {
            ui.weak(t!(
                ui,
                "user-library-kept-in",
                path = path.display().to_string()
            ));
        }
        None => {
            ui.colored_label(
                ui.visuals().warn_fg_color,
                t!(
                    ui,
                    "user-library-no-settings-folder-use",
                    var = chipload_pipeline::library::CONFIG_DIR_VAR
                ),
            );
        }
    }
    let units = crate::units::display(ui.ctx()).into();
    for problem in problems {
        let text = problem.render(ui.lang(), units);
        ui.colored_label(ui.visuals().error_fg_color, text);
    }
}

pub(super) fn tool_entry_form(
    ui: &mut egui::Ui,
    entry: &mut LibraryTool,
    materials: &chipload_job::materials::MaterialLibrary,
) {
    Grid::new("library-tool").num_columns(2).show(ui, |ui| {
        ui.label(t!(ui, "user-library-name"));
        ui.text_edit_singleline(&mut entry.name);
        ui.end_row();
        ui.label(t!(ui, "user-library-tool-number"))
            .on_hover_text(t!(ui, "user-library-tool-number-hover"));
        ui.add(crate::units::number(ui, &mut entry.id).range(1..=9999));
        ui.end_row();
        let mut tool = entry.tool(entry.id);
        tool_rows(ui, "library", "", &mut tool);
        entry.kind = tool.kind;
        entry.diameter = tool.diameter;
        entry.rpm = tool.rpm;
        entry.center_cutting = tool.center_cutting;
        entry.angle = tool.angle;
        entry.tip_diameter = tool.tip_diameter;
        entry.flutes = tool.flutes;
    });
    cutting_data_rows(ui, entry, materials);
}

/// A library tool's cutting data (docs/specs/cutting-data.md §7): one folded row per
/// material, *Any material* first, each with its seven values, and *Add a material…*.
fn cutting_data_rows(
    ui: &mut egui::Ui,
    entry: &mut LibraryTool,
    materials: &chipload_job::materials::MaterialLibrary,
) {
    ui.add_space(4.0);
    ui.strong(t!(ui, "user-library-cutting-data"))
        .on_hover_text(t!(ui, "user-library-cutting-data-hover"));
    // Any material first, then the materials as the library lists them.
    entry.cutting_data.sort_by_key(|r| match &r.material_uid {
        None => 0,
        Some(uid) => {
            1 + materials
                .materials
                .iter()
                .position(|m| &m.uid == uid)
                .unwrap_or(usize::MAX - 1)
        }
    });
    let units = crate::units::display(ui.ctx());
    let unit = chipload_pipeline::units::length_unit(units);
    let feed_unit = chipload_pipeline::units::feed_unit(units);
    let mut remove = None;
    for (k, row) in entry.cutting_data.iter_mut().enumerate() {
        let name = match &row.material_uid {
            None => t!(ui, "user-library-any-material"),
            Some(uid) => materials
                .find_uid(uid)
                .map_or_else(|| t!(ui, "user-library-material-gone"), |m| m.name.clone()),
        };
        let c = &mut row.values;
        let summary = t!(
            ui,
            "user-library-row-summary",
            slot_feed = super::forms::feed_in(units, c.slot_feed),
            slot_step_down = super::forms::len_in(units, c.slot_step_down),
            feed = super::forms::feed_in(units, c.feed),
            step_down = super::forms::len_in(units, c.step_down),
            stepover = chipload_i18n::message::trimmed(c.stepover, 2),
            unit = unit,
            feed_unit = feed_unit
        );
        let key = format!("cutting-row-{k}");
        super::forms::section(ui, &key, &name, &summary, |ui| {
            ui.label(t!(ui, "user-library-row-speed"))
                .on_hover_text(t!(ui, "user-library-row-speed-hover"));
            ui.horizontal(|ui| {
                let mut own = c.rpm.is_none();
                if ui
                    .checkbox(&mut own, t!(ui, "user-library-row-own-speed"))
                    .changed()
                {
                    c.rpm = (!own).then(|| entry_rpm_or_default(c.rpm));
                }
                if let Some(rpm) = &mut c.rpm {
                    ui.add(
                        crate::units::number(ui, rpm)
                            .range(1.0..=100_000.0)
                            .suffix(" rpm"),
                    );
                }
            });
            ui.end_row();
            ui.label(t!(ui, "user-library-plunge-feed"));
            ui.add(feed(&mut c.plunge_feed));
            ui.end_row();
            ui.label(t!(ui, "user-library-slot-feed"))
                .on_hover_text(t!(ui, "user-library-slot-hover"));
            ui.add(feed(&mut c.slot_feed));
            ui.end_row();
            ui.label(t!(ui, "user-library-slot-step-down"))
                .on_hover_text(t!(ui, "user-library-slot-hover"));
            ui.add(mm(&mut c.slot_step_down));
            ui.end_row();
            ui.label(t!(ui, "user-library-clearing-feed"))
                .on_hover_text(t!(ui, "user-library-clearing-hover"));
            ui.add(feed(&mut c.feed));
            ui.end_row();
            ui.label(t!(ui, "user-library-clearing-step-down"))
                .on_hover_text(t!(ui, "user-library-clearing-hover"));
            ui.add(mm(&mut c.step_down));
            ui.end_row();
            ui.label(t!(ui, "user-library-stepover"));
            ui.add(
                crate::units::number(ui, &mut c.stepover)
                    .speed(0.01)
                    .range(0.01..=1.0)
                    .suffix(" × D"),
            );
            ui.end_row();
            ui.label("");
            if ui
                .button(t!(ui, "user-library-remove-row"))
                .on_hover_text(t!(ui, "user-library-remove-row-hover"))
                .clicked()
            {
                remove = Some(k);
            }
            ui.end_row();
        });
    }
    if let Some(k) = remove {
        entry.cutting_data.remove(k);
    }
    // A new row starts from the row for any material, else a new operation's values.
    let start = entry.cutting_for(None).unwrap_or(Cutting {
        rpm: None,
        plunge_feed: crate::editor::default_plunge_feed(entry.diameter),
        slot_feed: crate::editor::default_feed(entry.diameter),
        slot_step_down: crate::editor::DEFAULT_STEP_DOWN,
        feed: crate::editor::default_feed(entry.diameter),
        step_down: crate::editor::DEFAULT_STEP_DOWN,
        stepover: crate::editor::DEFAULT_STEPOVER,
    });
    let mut choices: Vec<(Option<String>, String)> = Vec::new();
    if entry.row(None).is_none() {
        choices.push((None, t!(ui, "user-library-any-material")));
    }
    for m in &materials.materials {
        if entry.row(Some(&m.uid)).is_none() {
            choices.push((Some(m.uid.clone()), m.name.clone()));
        }
    }
    if !choices.is_empty() {
        egui::ComboBox::from_id_salt("library-tool-add-row")
            .selected_text(t!(ui, "user-library-add-material"))
            .show_ui(ui, |ui| {
                for (uid, name) in choices {
                    if ui.selectable_label(false, name).clicked() {
                        entry.set_row(uid.as_deref(), start);
                    }
                }
            });
    }
}

/// A speed to start a row's own speed from: the one it had, else a common router's.
fn entry_rpm_or_default(rpm: Option<f64>) -> f64 {
    rpm.unwrap_or(DEFAULT_RPM)
}

pub(super) fn machine_entry_form(
    ui: &mut egui::Ui,
    profile: &mut MachineProfile,
    library: &Library,
) {
    Grid::new("library-machine").num_columns(2).show(ui, |ui| {
        ui.label(t!(ui, "user-library-name"));
        ui.text_edit_singleline(&mut profile.name);
        ui.end_row();
        let mut machine = profile.machine();
        // A profile only names its post; it is copied into a project when used.
        if let Some(post) = machine_rows(ui, "library", &mut machine, Some(library), None) {
            machine.post = post;
        }
        let safe_z = t!(ui, "user-library-safe-z");
        safe_z_row(ui, &safe_z, &mut machine);
        work_zero_rows(ui, "library", &mut profile.work_zero, (0.0, 0.0));
        take_machine_values(profile, machine);
        ui.label(t!(ui, "user-library-max-overcut"))
            .on_hover_text(t!(ui, "user-library-max-overcut-hover"));
        ui.add(mm(&mut profile.max_overcut));
        ui.end_row();
    });
}

/// The machine values edited in a profile's form, back into the profile. Every field is
/// named, so a new one can't be left out (the G-code units and tool changes were, seen
/// 2026-10-31: changing them in the library did nothing).
pub(super) fn take_machine_values(profile: &mut MachineProfile, machine: Machine) {
    let Machine {
        name: _,
        library_uid: _,
        post,
        gcode_units,
        safe_z,
        spindle,
        max_feed,
        travel,
        tool_change,
        custom_post: _,
    } = machine;
    profile.post = post;
    profile.gcode_units = gcode_units;
    profile.safe_z = safe_z;
    profile.spindle = spindle;
    profile.max_feed = max_feed;
    profile.travel = travel;
    profile.tool_change = tool_change;
}

/// What the last import or export of this window did.
pub(super) fn library_message(
    ui: &mut egui::Ui,
    message: Option<&(LibraryKind, String)>,
    kind: LibraryKind,
) {
    if let Some((_, text)) = message.filter(|(k, _)| *k == kind) {
        ui.separator();
        ui.label(text);
    }
}

pub(super) fn tool_summary(
    lang: &impl crate::i18n::Lang,
    t: &LibraryTool,
    units: chipload_job::Units,
) -> String {
    format!(
        "{} {}",
        super::forms::tool_kind_name(lang, t.kind),
        chipload_pipeline::units::length_text(t.diameter, units)
    )
}

pub(super) fn machine_summary(m: &MachineProfile, units: chipload_job::Units) -> String {
    let n = |mm: f64| chipload_pipeline::units::length_number(mm, units);
    let unit = chipload_pipeline::units::length_unit(units);
    match &m.travel {
        Some(t) => format!("{}, {} × {} × {} {unit}", m.post, n(t.x), n(t.y), n(t.z)),
        None => m.post.clone(),
    }
}

impl ChiploadApp {
    /// Where the library is kept (None in tests and without a home folder), and read it.
    pub fn set_settings_dir(&mut self, dir: Option<PathBuf>) {
        self.user.settings_dir = dir;
        self.text_fonts = super::text_fonts::TextFonts::new(self.user.settings_dir.as_deref());
        self.reload_library();
        let settings = self
            .user
            .settings_dir
            .as_deref()
            .map(chipload_pipeline::library::settings_path)
            .and_then(|path| load_settings(&path).ok())
            .unwrap_or_default();
        self.load_update_settings(&settings);
        self.prefs.recent = settings.recent_projects;
        self.prefs.theme = settings.theme;
        self.prefs.auto_generate = settings.auto_generate;
        self.prefs.setup_sheet = settings.setup_sheet;
        self.prefs.show_grid = settings.show_grid;
        self.prefs.show_rulers = settings.show_rulers;
        self.prefs.units = settings.units;
        self.show_in(crate::i18n::resolve(&settings.language));
        self.prefs.language_setting = settings.language;
        self.load_layout();
        self.start_recovery();
    }

    /// Settings > Units: used at once and kept in the settings for next time.
    pub fn set_units(&mut self, units: chipload_job::Units) {
        self.prefs.units = units;
        self.change_settings(|settings| settings.units = units);
    }

    /// Settings > Language: used at once and kept in the settings for next time.
    pub fn set_language(&mut self, setting: &str) {
        self.show_in(crate::i18n::resolve(setting));
        self.prefs.language_setting = setting.to_string();
        self.change_settings(|settings| settings.language = setting.to_string());
    }

    /// Show the app in `language`. The start screen's hint, written before the language
    /// was known, is written again in it.
    fn show_in(&mut self, language: crate::i18n::Language) {
        let hint = |l| Report::info([t!(l, "app-start-hint")]);
        if self.report == hint(self.prefs.language) {
            self.report = hint(language);
        }
        self.prefs.language = language;
    }

    /// Settings > Colour theme: used at once and kept in the settings for next time.
    pub fn set_theme(&mut self, theme: Theme) {
        self.prefs.theme = theme;
        self.change_settings(|settings| settings.theme = theme);
    }

    /// Put a project at the top of File > Open recent. A failure to write the settings
    /// only shows in the status bar: it never stops opening or saving.
    pub(super) fn record_recent(&mut self, path: &Path) {
        let Some(text) = std::path::absolute(path)
            .ok()
            .and_then(|p| p.to_str().map(str::to_string))
        else {
            return;
        };
        self.change_settings(|settings| settings.add_recent(&text));
    }

    /// Take a project off the recent list, or empty it (None).
    pub fn forget_recent(&mut self, path: Option<&str>) {
        self.change_settings(|settings| match path {
            Some(path) => settings.remove_recent(path),
            None => settings.recent_projects.clear(),
        });
    }

    pub(super) fn change_settings(&mut self, change: impl FnOnce(&mut AppSettings)) {
        let Some(dir) = self.user.settings_dir.clone() else {
            return;
        };
        let path = chipload_pipeline::library::settings_path(&dir);
        let result = update_settings(&path, |settings| {
            change(settings);
            Ok(settings.recent_projects.clone())
        });
        match result {
            Ok(recent) => self.prefs.recent = recent,
            Err(err) => {
                let status = t!(
                    self,
                    "user-library-settings-not-saved",
                    error = self.say(&err.message())
                );
                self.set_status(status, ERROR_STATUS_SECONDS);
            }
        }
    }

    /// Open a project from File > Open recent; one that is gone is reported and forgotten.
    pub fn open_recent(&mut self, path: &str) -> bool {
        let file = PathBuf::from(path);
        if !file.exists() {
            self.error(vec![t!(self, "user-library-recent-gone", path = path)]);
            self.forget_recent(Some(path));
            return false;
        }
        if self.guard(Pending::Open(file.clone())) {
            return self.open(&file);
        }
        false
    }

    /// Read the library again (it may have been changed by the CLI or another window).
    pub(super) fn reload_library(&mut self) {
        self.user.library_state.post_facts = None;
        self.user.library = self
            .user
            .settings_dir
            .as_deref()
            .map(Library::load)
            .unwrap_or_default();
        // An old library was just converted (docs/specs/cutting-data.md §8): the report says
        // what moved, once.
        if !self.user.library.converted.is_empty() {
            let lines = self.user.library.converted.clone();
            self.set_report(Report::of_messages(lines));
        }
    }

    /// The library, if there is a settings folder.
    pub(super) fn library(&self) -> Option<&Library> {
        self.user.settings_dir.as_ref().map(|_| &self.user.library)
    }

    /// Project tool `id`'s cutting values for the setup's material, from its library tool
    /// (docs/specs/styles.md §2.3).
    pub(super) fn tool_cutting(&self, id: i64) -> Option<Cutting> {
        let edit = self.edit.as_ref()?;
        chipload_pipeline::new_ops::tool_cutting(&edit.job, self.library().map(|l| &l.tools), id)
    }

    pub(super) fn library_request(&mut self, request: LibraryRequest) {
        match request {
            LibraryRequest::UseMachine(name) => {
                self.use_machine_profile(&name);
            }
            LibraryRequest::UpdateMachine => {
                self.update_machine_from_library();
            }
            LibraryRequest::UseTool(id, name) => {
                self.use_library_tool(id, &name);
            }
            LibraryRequest::AddTool(name) => {
                self.add_library_tool(&name);
            }
            LibraryRequest::Save(target) => {
                let current = self.edit.as_ref().and_then(|e| match target {
                    SaveTarget::Machine => e.job.machine.name.clone(),
                    SaveTarget::Tool(id) => e
                        .job
                        .tools
                        .iter()
                        .find(|t| t.id == id)
                        .and_then(|t| t.name.clone()),
                });
                self.dialogs.prompt =
                    Some(Prompt::LibraryName(target, current.unwrap_or_default()));
            }
            LibraryRequest::SaveLinked(target) => {
                self.save_to_linked(target);
            }
            LibraryRequest::Show(kind, uid) => self.show_library(kind, Some(&uid)),
            LibraryRequest::UpdateAllTools => {
                self.update_tools_from_library(None);
            }
        }
    }

    /// Open the Library in the Browser at `kind`, and show the entry `uid` (or the folder)
    /// in Properties (docs/specs/library-in-browser.md §6).
    pub fn show_library(&mut self, kind: LibraryKind, uid: Option<&str>) {
        self.reload_library();
        self.user.library_state.reveal = Some(kind);
        self.focus = match (kind, uid) {
            (LibraryKind::Tools, None) => Focus::LibraryTools,
            (LibraryKind::Machines, None) => Focus::LibraryMachines,
            (LibraryKind::Tools, Some(uid)) => Focus::LibraryTool(uid_key(uid)),
            (LibraryKind::Machines, Some(uid)) => Focus::LibraryMachine(uid_key(uid)),
        };
    }

    /// Make `name` the project's post-processor; one of the user's is read again and copied
    /// in (docs/specs/user-posts.md §6). One undo step.
    pub fn choose_project_post(&mut self, name: &str) -> bool {
        self.reload_library();
        let posts = self.user.library.posts.clone();
        let Some(edit) = self.edit.as_mut() else {
            return false;
        };
        if let Err(err) =
            chipload_pipeline::posts::choose_post(&mut edit.job, name, posts.as_deref())
        {
            let title = t!(self, "user-library-post-named", name = name);
            self.error_report(Report::failed(title, err.message()));
            return false;
        }
        self.commit();
        let status = t!(self, "user-library-post-chosen", name = name);
        self.set_status(status, STATUS_SECONDS);
        true
    }

    /// Copy a machine profile's machine values into the project, keeping its safe Z, work
    /// zero and max overcut (docs/specs/linked-machine.md §2; one undo step).
    pub fn use_machine_profile(&mut self, name: &str) -> bool {
        self.reload_library();
        let Some(profile) = self.user.library.machines.find(name).cloned() else {
            self.error(vec![t!(
                self,
                "user-library-no-machine-profile",
                name = name
            )]);
            return false;
        };
        let Some(edit) = self.edit.as_mut() else {
            return false;
        };
        let mut changes = profile.update_machine_values(&mut edit.job);
        changes.extend(chipload_pipeline::library::copy_profile_post(
            &mut edit.job,
            self.user.library.posts.as_deref(),
        ));
        self.commit();
        let what = t!(
            self,
            "user-library-machine-profile-named",
            name = &profile.name
        );
        self.report_changes(&what, changes);
        true
    }

    /// Take the linked profile's machine values again (post, spindle, max feed, tool
    /// changes, travel), keeping the project's safe Z, work zero and max depth. One undo
    /// step.
    pub fn update_machine_from_library(&mut self) -> bool {
        self.reload_library();
        let profile = self.edit.as_ref().and_then(|e| {
            let uid = e.job.machine.library_uid.as_deref()?;
            self.user.library.machines.find_uid(uid).cloned()
        });
        let Some(profile) = profile else {
            return false;
        };
        let Some(edit) = self.edit.as_mut() else {
            return false;
        };
        let mut changes = profile.update_machine_values(&mut edit.job);
        changes.extend(chipload_pipeline::library::copy_profile_post(
            &mut edit.job,
            self.user.library.posts.as_deref(),
        ));
        self.commit();
        let what = t!(
            self,
            "user-library-machine-profile-named",
            name = &profile.name
        );
        self.report_changes(&what, changes);
        true
    }

    /// Give project tool `id` a library tool's values; it keeps its id, so the operations
    /// that use it still do (one undo step).
    pub fn use_library_tool(&mut self, id: i64, name: &str) -> bool {
        self.reload_library();
        let Some(entry) = self.user.library.tools.find(name).cloned() else {
            self.error(vec![t!(self, "user-library-no-library-tool", name = name)]);
            return false;
        };
        let Some(edit) = self.edit.as_mut() else {
            return false;
        };
        let Some(before) = edit.job.tools.iter().find(|t| t.id == id).cloned() else {
            self.error(vec![t!(self, "user-library-no-project-tool", id = id)]);
            return false;
        };
        let changes = entry.changes(&before);
        // Updating from its own library tool keeps a speed from the setup's material
        // (docs/specs/tool-updates.md §4).
        let keep = chipload_pipeline::library::material_speed(
            &edit.job,
            &self.user.library.tools,
            &self.user.library.materials,
            &before,
        )
        .filter(|_| before.library_uid.as_deref() == Some(entry.uid.as_str()));
        let Some(tool) = edit.job.tools.iter_mut().find(|t| t.id == id) else {
            return false;
        };
        *tool = entry.tool(id);
        if let Some(rpm) = keep {
            tool.rpm = Some(rpm);
        }
        edit.job.fit_helixes();
        self.commit();
        let what = t!(self, "user-library-library-tool-named", name = &entry.name);
        self.report_changes(&what, changes);
        true
    }

    /// How project tool `tool` stands to its library tool, a speed from the setup's material
    /// aside (docs/specs/tool-updates.md §4). None without a library.
    pub(super) fn project_tool_link(&self, tool: &Tool) -> Option<Link> {
        let library = self.library()?;
        let job = &self.edit.as_ref()?.job;
        Some(chipload_pipeline::library::project_tool_link(
            job,
            &library.tools,
            &library.materials,
            tool,
        ))
    }

    /// The project's tools (all, or tool `only`) that differ from their library tools take
    /// the library's values, as one undo step; the Report says which
    /// (docs/specs/tool-updates.md §2, §3). Whether any changed.
    pub fn update_tools_from_library(&mut self, only: Option<i64>) -> bool {
        self.commit();
        self.reload_library();
        let Some(edit) = self.edit.as_ref() else {
            return false;
        };
        let (job, updated) = chipload_pipeline::library::update_from_library(
            &edit.job,
            &self.user.library.tools,
            &self.user.library.materials,
            only,
        );
        if updated.is_empty() {
            return false;
        }
        if let Some(edit) = self.edit.as_mut() {
            edit.adopt(job);
        }
        self.commit();
        let mut report = Report::info([t!(self, "user-library-tools-updated")]);
        for (id, changes) in updated {
            let text = self.changes_text(&changes);
            let line = t!(self, "user-library-tool-updated", tool = id, changes = text);
            report.push_target(Tone::Info, line, super::report::Target::Tool(id));
        }
        self.set_report(report);
        true
    }

    /// What differs, in one line: "diameter 6 mm -> 6.35 mm; rpm 18000 -> 16000".
    pub(super) fn changes_text(&self, changes: &[Message]) -> String {
        changes
            .iter()
            .map(|c| self.say(c))
            .collect::<Vec<_>>()
            .join("; ")
    }

    /// Lines for the Report when a project opens: each project tool that differs from its
    /// library tool, and *Update all from library* (docs/specs/tool-updates.md §3).
    pub(super) fn differing_tool_lines(&self, report: &mut Report) {
        let Some(edit) = self.edit.as_ref() else {
            return;
        };
        let mut any = false;
        for tool in &edit.job.tools {
            let Some(Link::Differs(name, changes)) = self.project_tool_link(tool) else {
                continue;
            };
            let line = t!(
                self,
                "user-library-tool-differs",
                tool = tool.id,
                name = name,
                changes = self.changes_text(&changes)
            );
            report.push_target(Tone::Warning, line, super::report::Target::Tool(tool.id));
            any = true;
        }
        if any {
            let line = t!(self, "user-library-update-all");
            report.push_target(Tone::Info, line, super::report::Target::UpdateTools);
        }
    }

    /// Copy a library tool into the project as a tool of its own, or find it there already
    /// (docs/specs/tool-per-operation.md §8). Returns its id in the project.
    pub fn add_library_tool(&mut self, name: &str) -> Option<i64> {
        let id = self.insert_library_tool(name)?;
        self.commit();
        Some(id)
    }

    /// Cut operation `op` with a library tool, copied into the project if it isn't there
    /// yet: one undo step. The operation keeps its feeds (decision 6).
    pub fn tool_from_library(&mut self, op: usize, name: &str) -> bool {
        let Some(id) = self.insert_library_tool(name) else {
            return false;
        };
        let Some(entry) = self
            .edit
            .as_mut()
            .and_then(|e| e.job.operations.get_mut(op))
        else {
            return false;
        };
        entry.set_tool(id);
        if let Some(edit) = &mut self.edit
            && let Some(entry) = edit.job.operations.get_mut(op)
        {
            crate::editor::fit_hole_method(entry, &edit.job.tools);
            entry.fit_helixes(&edit.job.tools);
        }
        self.commit();
        true
    }

    pub(super) fn insert_library_tool(&mut self, name: &str) -> Option<i64> {
        self.reload_library();
        let Some(entry) = self.user.library.tools.find(name).cloned() else {
            self.error(vec![t!(self, "user-library-no-library-tool", name = name)]);
            return None;
        };
        let edit = self.edit.as_mut()?;
        let used = chipload_job::library::use_tool(&mut edit.job, &entry);
        let text = format!(
            "\"{}\": {}",
            entry.name,
            self.say(&used.describe_message(entry.id))
        );
        self.set_status(text, STATUS_SECONDS);
        Some(used.id)
    }

    fn report_changes(&mut self, what: &str, changes: Vec<Message>) {
        let status = if changes.is_empty() {
            t!(self, "user-library-no-values-changed-status", what = what)
        } else {
            t!(
                self,
                "user-library-values-changed-status",
                what = what,
                count = changes.len()
            )
        };
        let mut report = Report::info([t!(self, "user-library-changes-heading", what = what)]);
        if changes.is_empty() {
            report.push(
                Tone::Info,
                format!("  {}", t!(self, "user-library-no-values-changed")),
            );
        }
        for change in changes {
            let line = Message::new("report-indented").arg("line", change);
            report.push_message(Tone::Info, line, None);
        }
        self.set_report(report);
        self.set_status(status, STATUS_SECONDS);
    }

    /// Save the project's machine or tool into the library under `name`: an entry of that
    /// name is replaced and keeps its uid, so projects linked to it stay linked; otherwise a
    /// new entry is added. The project is linked to it (docs/specs/library-in-browser.md §3).
    pub fn save_to_library(&mut self, target: SaveTarget, name: &str) -> bool {
        self.save_values(target, Pick::Name(name.trim().to_string()))
    }

    /// Save the project's machine or tool into the entry it is linked to, keeping the
    /// entry's name and uid.
    pub fn save_to_linked(&mut self, target: SaveTarget) -> bool {
        let uid = self.edit.as_ref().and_then(|e| match target {
            SaveTarget::Machine => e.job.machine.library_uid.clone(),
            SaveTarget::Tool(id) => e
                .job
                .tools
                .iter()
                .find(|t| t.id == id)
                .and_then(|t| t.library_uid.clone()),
        });
        match uid {
            Some(uid) => self.save_values(target, Pick::Uid(uid)),
            None => false,
        }
    }

    fn save_values(&mut self, target: SaveTarget, pick: Pick) -> bool {
        let (Some(dir), Some(edit)) = (self.user.settings_dir.clone(), self.edit.as_ref()) else {
            return false;
        };
        let settings = edit.job.clone();
        let lang = self.lang();
        // The entry's name and uid after saving.
        let result: Result<(String, String), LibraryError> = match target {
            SaveTarget::Machine => update_machines(&machines_path(&dir), |library| {
                let at = match &pick {
                    Pick::Name(name) => library.position(name),
                    Pick::Uid(uid) => library.machines.iter().position(|m| &m.uid == uid),
                };
                match (at, &pick) {
                    (Some(i), _) => {
                        let old = &library.machines[i];
                        let profile = MachineProfile::from_job(&old.name, &old.uid, &settings);
                        library.machines[i] = profile.clone();
                        Ok((profile.name, profile.uid))
                    }
                    (None, Pick::Name(name)) => {
                        let uid = library.new_uid(name);
                        library
                            .machines
                            .push(MachineProfile::from_job(name, &uid, &settings));
                        Ok((name.clone(), uid))
                    }
                    (None, Pick::Uid(_)) => Err(refused(t!(lang, "user-library-profile-gone"))),
                }
            }),
            SaveTarget::Tool(id) => match settings.tool(id) {
                Some(tool) => update_tools(&tools_path(&dir), |library| {
                    let at = match &pick {
                        Pick::Name(name) => library.position(name),
                        Pick::Uid(uid) => library.tools.iter().position(|t| &t.uid == uid),
                    };
                    match (at, &pick) {
                        // A project tool has no cutting values: keep the library's.
                        (Some(i), _) => {
                            let old = &library.tools[i];
                            let entry = LibraryTool::from_tool(
                                &old.name,
                                &old.uid,
                                tool,
                                old.cutting_data.clone(),
                            );
                            library.tools[i] = entry.clone();
                            Ok((entry.name, entry.uid))
                        }
                        (None, Pick::Name(name)) => {
                            let uid = library.new_uid(name);
                            library.tools.push(LibraryTool::from_tool(
                                name,
                                &uid,
                                tool,
                                Vec::new(),
                            ));
                            Ok((name.clone(), uid))
                        }
                        (None, Pick::Uid(_)) => {
                            Err(refused(t!(lang, "user-library-library-tool-gone")))
                        }
                    }
                }),
                None => Err(refused(t!(lang, "user-library-no-tool-to-save"))),
            },
        };
        self.reload_library();
        let (name, uid) = match result {
            Ok(saved) => saved,
            Err(err) => {
                let title = t!(self, "user-library-not-saved");
                self.error_report(Report::failed(title, err.message()));
                return false;
            }
        };
        if let Some(edit) = self.edit.as_mut() {
            match target {
                SaveTarget::Machine => {
                    edit.job.machine.name = Some(name.clone());
                    edit.job.machine.library_uid = Some(uid);
                }
                SaveTarget::Tool(id) => {
                    if let Some(tool) = edit.job.tools.iter_mut().find(|t| t.id == id) {
                        tool.name = Some(name.clone());
                        tool.library_uid = Some(uid);
                    }
                }
            }
        }
        self.commit();
        let status = t!(self, "user-library-saved", name = &name);
        self.set_status(status, STATUS_SECONDS);
        true
    }

    pub(super) fn library_name_prompt(
        &mut self,
        ctx: &egui::Context,
        target: SaveTarget,
        mut text: String,
    ) {
        let (title, taken) = match target.kind() {
            LibraryKind::Machines => (
                t!(ctx, "user-library-save-machine-title"),
                self.user
                    .library
                    .machines
                    .find(text.trim())
                    .map(|m| m.name.clone()),
            ),
            LibraryKind::Tools => (
                t!(ctx, "user-library-save-tool-title"),
                self.user
                    .library
                    .tools
                    .find(text.trim())
                    .map(|t| t.name.clone()),
            ),
        };
        let mut done = false;
        let mut save = false;
        egui::Window::new(title)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label(t!(ui, "user-library-name-in-library"));
                ui.add(TextEdit::singleline(&mut text).desired_width(320.0));
                if let Some(existing) = &taken {
                    ui.colored_label(
                        ui.visuals().warn_fg_color,
                        t!(ui, "user-library-replaces", name = existing),
                    );
                }
                ui.horizontal(|ui| {
                    let label = if taken.is_some() {
                        t!(ui, "user-library-replace")
                    } else {
                        t!(ui, "common-save")
                    };
                    if ui
                        .add_enabled(!text.trim().is_empty(), egui::Button::new(label))
                        .clicked()
                    {
                        save = true;
                        done = true;
                    }
                    if ui.button(t!(ui, "common-cancel")).clicked() {
                        done = true;
                    }
                });
            });
        self.dialogs.prompt = (!done).then(|| Prompt::LibraryName(target, text.clone()));
        if save {
            self.save_to_library(target, &text);
        }
    }

    /// Store a library tool edited in Properties: it replaces the tool with its uid, or is
    /// added. A name another tool has is refused.
    pub fn store_library_tool(&mut self, entry: LibraryTool) -> Result<(), String> {
        let dir = self
            .user
            .settings_dir
            .clone()
            .ok_or_else(|| t!(self, "user-library-no-settings-folder"))?;
        let lang = self.lang();
        let result = update_tools(&tools_path(&dir), |library| {
            if library
                .tools
                .iter()
                .any(|t| t.uid != entry.uid && same_name(&t.name, &entry.name))
            {
                return Err(refused(t!(
                    lang,
                    "user-library-tool-exists",
                    name = &entry.name
                )));
            }
            match library.tools.iter().position(|t| t.uid == entry.uid) {
                Some(i) => {
                    if library
                        .default_tool
                        .as_deref()
                        .is_some_and(|d| same_name(d, &library.tools[i].name))
                    {
                        library.default_tool = Some(entry.name.clone());
                    }
                    library.tools[i] = entry;
                }
                None => library.tools.push(entry),
            }
            Ok(())
        });
        self.reload_library();
        result.map_err(|e| self.say(&e.message()))
    }

    /// Store a machine profile edited in Properties, as `store_library_tool`.
    pub fn store_machine_profile(&mut self, profile: MachineProfile) -> Result<(), String> {
        let dir = self
            .user
            .settings_dir
            .clone()
            .ok_or_else(|| t!(self, "user-library-no-settings-folder"))?;
        let lang = self.lang();
        let result = update_machines(&machines_path(&dir), |library| {
            if library
                .machines
                .iter()
                .any(|m| m.uid != profile.uid && same_name(&m.name, &profile.name))
            {
                return Err(refused(t!(
                    lang,
                    "user-library-machine-exists",
                    name = &profile.name
                )));
            }
            match library.machines.iter().position(|m| m.uid == profile.uid) {
                Some(i) => {
                    if library
                        .default_machine
                        .as_deref()
                        .is_some_and(|d| same_name(d, &library.machines[i].name))
                    {
                        library.default_machine = Some(profile.name.clone());
                    }
                    library.machines[i] = profile;
                }
                None => library.machines.push(profile),
            }
            Ok(())
        });
        self.reload_library();
        result.map_err(|e| self.say(&e.message()))
    }

    /// A new library tool or machine, saved at once under a free name ("New tool 2") and
    /// shown in Properties. It starts from the open project's first tool or machine.
    pub fn new_library_entry(&mut self, kind: LibraryKind) -> Option<String> {
        self.reload_library();
        let result = match kind {
            LibraryKind::Tools => {
                let base = t!(self, "user-library-new-tool");
                let name = unique_name(&base, |n| self.user.library.tools.find(n).is_some());
                let uid = self.user.library.tools.new_uid(&name);
                let entry = match self.edit.as_ref().and_then(|e| e.job.tools.first()) {
                    Some(tool) => LibraryTool::from_tool(&name, &uid, tool, Vec::new()),
                    None => LibraryTool {
                        name,
                        uid: uid.clone(),
                        id: 1,
                        kind: ToolType::Endmill,
                        diameter: DEFAULT_TOOL_DIAMETER,
                        flutes: None,
                        center_cutting: None,
                        rpm: Some(DEFAULT_RPM),
                        cutting_data: Vec::new(),
                        angle: None,
                        tip_diameter: None,
                    },
                };
                self.store_library_tool(entry).map(|()| uid)
            }
            LibraryKind::Machines => {
                let base = t!(self, "user-library-new-machine");
                let name = unique_name(&base, |n| self.user.library.machines.find(n).is_some());
                let uid = self.user.library.machines.new_uid(&name);
                let profile = match self.edit.as_ref() {
                    Some(edit) => MachineProfile::from_job(&name, &uid, &edit.job),
                    None => MachineProfile {
                        name,
                        uid: uid.clone(),
                        post: "grbl".into(),
                        gcode_units: chipload_job::Units::Mm,
                        safe_z: DEFAULT_SAFE_Z,
                        spindle: Spindle::Cw,
                        max_feed: None,
                        travel: None,
                        // As a new project (docs/specs/work-zero-marker.md §7).
                        work_zero: WorkZero::Stock {
                            corner: chipload_job::StockCorner::BottomLeft,
                        },
                        max_overcut: DEFAULT_MAX_OVERCUT,
                        tool_change: ToolChange::Files,
                    },
                };
                self.store_machine_profile(profile).map(|()| uid)
            }
        };
        match result {
            Ok(uid) => {
                self.show_library(kind, Some(&uid));
                Some(uid)
            }
            Err(err) => {
                self.user.library_state.error = Some(err);
                None
            }
        }
    }

    pub(super) fn remove_from_library(
        &mut self,
        kind: LibraryKind,
        name: &str,
    ) -> Result<(), String> {
        let dir = self
            .user
            .settings_dir
            .clone()
            .ok_or_else(|| t!(self, "user-library-no-settings-folder"))?;
        let result = match kind {
            // Its rows of cutting data go with it.
            LibraryKind::Tools => update_tools(&tools_path(&dir), |library| {
                library.remove(name).map(|_| ())
            }),
            LibraryKind::Machines => update_machines(&machines_path(&dir), |library| {
                library.remove(name).map(|_| ())
            }),
        };
        self.reload_library();
        result.map_err(|e| self.say(&e.message()))
    }

    /// The tool or machine new projects start with.
    pub fn set_library_default(&mut self, kind: LibraryKind, name: &str) -> Result<(), String> {
        let dir = self
            .user
            .settings_dir
            .clone()
            .ok_or_else(|| t!(self, "user-library-no-settings-folder"))?;
        let result = match kind {
            LibraryKind::Tools => update_tools(&tools_path(&dir), |library| {
                library.default_tool = Some(name.to_string());
                Ok(())
            }),
            LibraryKind::Machines => update_machines(&machines_path(&dir), |library| {
                library.default_machine = Some(name.to_string());
                Ok(())
            }),
        };
        self.reload_library();
        result.map_err(|e| self.say(&e.message()))
    }
}

/// The user's settings folder and what the window read from it.
#[derive(Default)]
pub struct UserFiles {
    /// The user's settings folder (docs/specs/tool-library-and-machines.md); None in tests
    /// and when there is no home folder.
    pub(super) settings_dir: Option<PathBuf>,
    /// The tool library and machine profiles as last read.
    pub(super) library: chipload_pipeline::library::Library,
    pub(super) library_state: LibraryState,
}
