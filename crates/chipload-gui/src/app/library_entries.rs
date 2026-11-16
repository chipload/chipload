// SPDX-License-Identifier: GPL-3.0-or-later
//! A library entry's form and summary: a tool with its cutting data per material, a machine
//! profile (docs/specs/tool-library-and-machines.md §8).

use chipload_job::Machine;
use chipload_job::library::{Cutting, LibraryTool, MachineProfile};
use chipload_pipeline::DEFAULT_RPM;
use chipload_pipeline::library::Library;

use super::forms::{feed, machine_rows, mm, safe_z_row, tool_rows, work_zero_rows};
use super::*;
use crate::i18n::Lang;

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
    crate::app::forms::card(ui, |ui| {
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
            entry.corner_radius = tool.corner_radius;
            entry.flutes = tool.flutes;
        })
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
    ui.label(crate::style::bold(ui, t!(ui, "user-library-cutting-data")))
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
    crate::app::forms::card(ui, |ui| {
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
        })
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
