// SPDX-License-Identifier: GPL-3.0-or-later
//! The machine's and the tools' forms.

use super::*;

/// The machine: closed by default, as it is set once. The library rows come first, then
/// safe Z and the depth allowed below the stock, then the machine's values
/// (docs/specs/linked-machine.md §3).
pub(super) fn machine_section(
    ui: &mut egui::Ui,
    job: &mut chipload_job::Job,
    library: Option<&Library>,
    look: SectionLook,
    request: &mut Option<LibraryRequest>,
    copies_request: &mut Option<JobRequest>,
) {
    let units = crate::units::display(ui.ctx());
    let len_unit = chipload_pipeline::units::length_unit(units);
    let profile = library.and_then(|l| {
        let uid = job.machine.library_uid.as_deref()?;
        l.machines.find_uid(uid).cloned()
    });
    let mut summary = t!(
        ui,
        "forms-machine-summary",
        post = job.machine.post.as_str(),
        safe_z = format!("{} {len_unit}", len_in(units, job.machine.safe_z)),
        spindle = match job.machine.spindle {
            Spindle::Cw => "M3",
            Spindle::Ccw => "M4",
        }
    );
    if let Some(p) = &profile {
        summary = format!("{} · {summary}", p.name);
    }
    let same = profile
        .as_ref()
        .is_some_and(|p| p.machine_changes(&job.machine).is_empty());
    let edit_here = egui::Id::new("job-machine-edit-here");
    let editing = ui.data(|d| d.get_temp::<bool>(edit_here).unwrap_or(false));
    section_look(
        ui,
        "job-machine",
        &t!(ui, "forms-section-machine"),
        &summary,
        look,
        |ui| {
            library_rows(ui, request, LibraryRows::machine(job, library));
            safe_z_row(ui, &t!(ui, "forms-safe-z-job"), &mut job.machine);
            // The profile's machine values, greyed, under a heading of their own
            // (docs/specs/linked-machine.md decision 7).
            let linked = profile.is_some() && same && !editing;
            if linked {
                ui.add_space(4.0);
                ui.end_row();
                ui.strong(t!(ui, "forms-from-profile"));
                if ui
                    .button(t!(ui, "forms-edit-here"))
                    .on_hover_text(t!(ui, "forms-edit-here-hover"))
                    .clicked()
                {
                    ui.data_mut(|d| d.insert_temp(edit_here, true));
                }
                ui.end_row();
            }
            if linked {
                machine_text_rows(ui, &job.machine);
            } else if let Some(post) =
                machine_rows(ui, "job", &mut job.machine, library, profile.as_ref())
            {
                *copies_request = Some(JobRequest::Post(post));
            }
            post_copy_row(ui, &job.machine, library, copies_request);
        },
    );
}

/// "T2 endmill Ø3 mm "3 mm downcut"": a project tool as lists show it.
pub(in crate::app) fn tool_label(
    lang: &impl crate::i18n::Lang,
    tool: &Tool,
    units: Units,
) -> String {
    let len_unit = chipload_pipeline::units::length_unit(units);
    let name = tool
        .name
        .as_deref()
        .map_or(String::new(), |n| format!(" \"{n}\""));
    format!(
        "T{} {} Ø{} {len_unit}{name}",
        tool.id,
        tool_kind_name(lang, tool.kind),
        len_in(units, tool.diameter)
    )
}

/// The operations (1-based numbers) that cut with tool `id`.
pub(in crate::app) fn tool_users(edit: &EditJob, id: i64) -> Vec<String> {
    let mut users = Vec::new();
    for (i, op) in edit.job.operations.iter().enumerate() {
        let at = chipload_job::OpRef::op(i);
        // A rest pass rests after a tool too: a pocket's own, or the pass before.
        let rests_after = match op {
            Operation::Pocket(p) => p.rest.is_some_and(|r| r.after_tool == id),
            _ => false,
        };
        if op.tool() == id || rests_after {
            users.push(at.to_string());
        }
        match op {
            Operation::Pocket(p) => {
                for (k, pass) in p.rest_passes.iter().enumerate() {
                    if pass.tool == id {
                        users.push(chipload_job::OpRef::pass(i, k).to_string());
                    }
                }
            }
            Operation::Vcarve(v) if v.floor.is_some_and(|f| f.tool == id) => {
                users.push(chipload_job::OpRef::pass(i, 0).to_string());
            }
            _ => {}
        }
    }
    users
}

/// The project's tools, each in a section of its own, and buttons to add one
/// (docs/specs/tool-per-operation.md §8, properties-for-tools-and-dxfs.md §2). A tool an
/// operation uses can't be removed.
pub(in crate::app) fn tools_form(
    ui: &mut egui::Ui,
    edit: &mut EditJob,
    library: Option<&Library>,
) -> Option<JobRequest> {
    let mut request = None;
    let title = t!(ui, "forms-tools");
    super::browser::title_line(ui, crate::icons::Icon::Tools, None, &title);
    ui.horizontal(|ui| {
        let names: Vec<String> = library
            .map(|l| l.tools.tools.iter().map(|t| t.name.clone()).collect())
            .unwrap_or_default();
        ui.add_enabled_ui(!names.is_empty(), |ui| {
            ComboBox::from_id_salt("add-library-tool")
                .selected_text(t!(ui, "forms-add-from-library"))
                .show_ui(ui, |ui| {
                    for name in names {
                        if ui.selectable_label(false, &name).clicked() {
                            request = Some(LibraryRequest::AddTool(name));
                        }
                    }
                });
        });
        if ui
            .button(t!(ui, "forms-new-tool"))
            .on_hover_text(t!(
                ui,
                "units-text-new-tool-hover",
                diameter = crate::units::text(ui.ctx(), chipload_pipeline::DEFAULT_TOOL_DIAMETER)
            ))
            .clicked()
        {
            let job = &mut edit.job;
            let id = job.tools.iter().map(|t| t.id).max().unwrap_or(0).max(0) + 1;
            job.tools.push(Tool {
                id,
                name: None,
                library_uid: None,
                kind: ToolType::Endmill,
                diameter: chipload_pipeline::DEFAULT_TOOL_DIAMETER,
                flutes: None,
                center_cutting: None,
                rpm: Some(chipload_pipeline::DEFAULT_RPM),
                angle: None,
                tip_diameter: None,
            });
        }
    });
    ui.add_space(4.0);
    let mut remove = None;
    for index in 0..edit.job.tools.len() {
        let id = edit.job.tools[index].id;
        let users = tool_users(edit, id);
        let summary = match users.len() {
            0 => t!(ui, "forms-tool-not-used"),
            _ => t!(ui, "forms-tool-used-by", operations = users.join(", ")),
        };
        let title = tool_label(ui, &edit.job.tools[index], crate::units::display(ui.ctx()));
        let at = format!("tools.{index}");
        let marked = problems::has_problem_under(ui, &at);
        let tool = &mut edit.job.tools[index];
        section_marked(
            ui,
            &format!("job-tool-{id}"),
            &title,
            &summary,
            marked,
            |ui| {
                if tool_body(ui, tool, &at, &users, library, &mut request) {
                    remove = Some(index);
                }
            },
        );
    }
    if let Some(index) = remove {
        edit.job.tools.remove(index);
    }
    request.map(JobRequest::Library)
}

/// One tool alone (docs/specs/properties-for-tools-and-dxfs.md §2).
pub(in crate::app) fn tool_form(
    ui: &mut egui::Ui,
    edit: &mut EditJob,
    library: Option<&Library>,
    id: i64,
) -> Option<JobRequest> {
    let index = edit.job.tools.iter().position(|t| t.id == id)?;
    let users = tool_users(edit, id);
    let mut request = None;
    let icon = match edit.job.tools[index].kind {
        ToolType::Endmill => crate::icons::Icon::Endmill,
        ToolType::Drill => crate::icons::Icon::Drill,
        ToolType::Vbit => crate::icons::Icon::Vbit,
    };
    super::browser::title_line(
        ui,
        icon,
        None,
        &tool_label(ui, &edit.job.tools[index], crate::units::display(ui.ctx())),
    );
    ui.weak(match users.len() {
        0 => t!(ui, "forms-tool-not-used-long"),
        _ => t!(ui, "forms-tool-cuts", operations = users.join(", ")),
    });
    let at = format!("tools.{index}");
    let tool = &mut edit.job.tools[index];
    let mut removed = false;
    Grid::new(("tool-form", id))
        .num_columns(2)
        .min_col_width(label_width(ui))
        .show(ui, |ui| {
            removed = tool_body(ui, tool, &at, &users, library, &mut request);
        });
    if removed {
        edit.job.tools.remove(index);
        return Some(JobRequest::ToolRemoved);
    }
    request.map(JobRequest::Library)
}

/// A tool's rows: its library line, its values and *Remove tool*. True when *Remove* was
/// pressed.
pub(super) fn tool_body(
    ui: &mut egui::Ui,
    tool: &mut Tool,
    at: &str,
    users: &[String],
    library: Option<&Library>,
    request: &mut Option<LibraryRequest>,
) -> bool {
    let salt = format!("job-tool-{}", tool.id);
    library_rows(ui, request, LibraryRows::tool(tool, library));
    tool_rows(ui, &salt, at, tool);
    ui.label("");
    let button = ui.add_enabled(
        users.is_empty(),
        egui::Button::new(t!(ui, "forms-remove-tool")),
    );
    let mut remove = false;
    if users.is_empty() {
        remove = button.clicked();
    } else {
        button.on_disabled_hover_text(t!(
            ui,
            "forms-remove-tool-disabled",
            operations = users.join(", ")
        ));
    }
    ui.end_row();
    remove
}

/// A tool's type, in the app's language: `endmill`, `drill`, `V-bit`.
pub(in crate::app) fn tool_kind_name(lang: &impl crate::i18n::Lang, kind: ToolType) -> String {
    match kind {
        ToolType::Endmill => t!(lang, "names-tool-endmill"),
        ToolType::Drill => t!(lang, "names-tool-drill"),
        ToolType::Vbit => t!(lang, "names-tool-vbit"),
    }
}

/// The machine values: post, spindle, max feed, tool changes and travel (safe Z and the
/// work zero are the job's: `safe_z_row`, `work_zero_rows`). Returns the post picked, if
/// another one was: the caller sets it (a user post is copied into a project, a profile
/// only names it). With `profile`, a row that differs from it gets a button that takes the
/// profile's value (docs/specs/linked-machine.md §3).
pub(in crate::app) fn machine_rows(
    ui: &mut egui::Ui,
    salt: &str,
    machine: &mut Machine,
    library: Option<&Library>,
    profile: Option<&MachineProfile>,
) -> Option<String> {
    let mut picked = None;
    let from = profile.map(MachineProfile::machine);
    let from = from.as_ref();
    ui.label(t!(ui, "forms-post"));
    ui.horizontal(|ui| {
        ComboBox::from_id_salt((salt, "post"))
            .selected_text(machine.post.clone())
            .show_ui(ui, |ui| {
                for name in chipload_pipeline::posts::builtin_posts() {
                    if ui.selectable_label(machine.post == name, name).clicked() {
                        picked = Some(name.to_string());
                    }
                }
                let yours = library.map_or(&[][..], |l| l.user_posts.as_slice());
                if !yours.is_empty() {
                    ui.separator();
                }
                for (name, read) in yours {
                    let label = t!(ui, "forms-post-yours", name = name.as_str());
                    let response = ui.add_enabled(
                        read.is_ok(),
                        egui::Button::selectable(machine.post == *name, label),
                    );
                    if let Err(problem) = read {
                        response.on_disabled_hover_text(
                            problem.render(ui.lang(), crate::units::display(ui.ctx()).into()),
                        );
                    } else if response.clicked() {
                        picked = Some(name.clone());
                    }
                }
            })
            .response
            .on_hover_text(match library.and_then(|l| l.posts.as_deref()) {
                Some(dir) => t!(
                    ui,
                    "forms-post-folder-hover",
                    folder = dir.display().to_string()
                ),
                None => t!(ui, "forms-post-builtin-hover"),
            });
        if let Some(from) = from
            && from.post != machine.post
            && revert_mark(ui, profile, &from.post)
        {
            picked = Some(from.post.clone());
        }
    });
    ui.end_row();
    ui.label(t!(ui, "forms-gcode-units"))
        .on_hover_text(t!(ui, "forms-gcode-units-hover"));
    ui.horizontal(|ui| {
        let row = ComboBox::from_id_salt((salt, "gcode-units"))
            .selected_text(gcode_units_text(ui, machine.gcode_units))
            .show_ui(ui, |ui| {
                for units in [Units::Mm, Units::Inch] {
                    let text = gcode_units_text(ui, units);
                    ui.selectable_value(&mut machine.gcode_units, units, text);
                }
            });
        flag(row.response, ui, "machine.gcode_units");
        if let Some(from) = from
            && from.gcode_units != machine.gcode_units
            && revert_mark(ui, profile, &gcode_units_text(ui, from.gcode_units))
        {
            machine.gcode_units = from.gcode_units;
        }
    });
    ui.end_row();
    ui.label(t!(ui, "forms-spindle"));
    ui.horizontal(|ui| {
        ComboBox::from_id_salt((salt, "spindle"))
            .selected_text(spindle_text(ui, machine.spindle))
            .show_ui(ui, |ui| {
                for spindle in [Spindle::Cw, Spindle::Ccw] {
                    ui.selectable_value(&mut machine.spindle, spindle, spindle_text(ui, spindle));
                }
            });
        if let Some(from) = from
            && from.spindle != machine.spindle
            && revert_mark(ui, profile, &spindle_text(ui, from.spindle))
        {
            machine.spindle = from.spindle;
        }
    });
    ui.end_row();
    ui.label(t!(ui, "forms-max-feed"));
    ui.horizontal(|ui| {
        let mut set = machine.max_feed.is_some();
        if ui.checkbox(&mut set, "").changed() {
            machine.max_feed = set.then_some(5000.0);
        }
        if let Some(max) = &mut machine.max_feed {
            ui.add(feed(max));
        }
        if let Some(from) = from
            && from.max_feed != machine.max_feed
            && revert_mark(ui, profile, &max_feed_text(ui, from.max_feed))
        {
            machine.max_feed = from.max_feed;
        }
    });
    ui.end_row();
    ui.label(t!(ui, "forms-tool-changes"))
        .on_hover_text(t!(ui, "forms-tool-changes-hover"));
    ui.horizontal(|ui| {
        ComboBox::from_id_salt((salt, "tool-change"))
            .selected_text(tool_change_name(ui, machine.tool_change))
            .show_ui(ui, |ui| {
                for change in [ToolChange::Files, ToolChange::Program] {
                    let name = tool_change_name(ui, change);
                    ui.selectable_value(&mut machine.tool_change, change, name);
                }
            });
        if let Some(from) = from
            && from.tool_change != machine.tool_change
            && revert_mark(ui, profile, &tool_change_name(ui, from.tool_change))
        {
            machine.tool_change = from.tool_change;
        }
    });
    ui.end_row();
    ui.label(t!(ui, "forms-travel"));
    ui.horizontal(|ui| {
        let mut set = machine.travel.is_some();
        if ui.checkbox(&mut set, "").changed() {
            machine.travel = set.then_some(Travel {
                x: 800.0,
                y: 600.0,
                z: 100.0,
            });
        }
        if let Some(from) = from
            && from.travel != machine.travel
            && revert_mark(ui, profile, &travel_text(ui, from.travel.as_ref()))
        {
            machine.travel = from.travel.clone();
        }
    });
    ui.end_row();
    // The three fields on a row of their own: beside the box they run past a narrow
    // panel's edge.
    if let Some(t) = &mut machine.travel {
        ui.label("");
        ui.horizontal(|ui| {
            flag(ui.add(mm(&mut t.x)), ui, "machine.travel.x");
            flag(ui.add(mm(&mut t.y)), ui, "machine.travel.y");
            flag(ui.add(mm(&mut t.z)), ui, "machine.travel.z");
        });
        ui.end_row();
    }
    picked
}

/// The machine values as plain text, for a machine that is the same as its profile
/// (docs/specs/linked-machine.md §3).
pub(super) fn machine_text_rows(ui: &mut egui::Ui, machine: &Machine) {
    for (label, value) in [
        (t!(ui, "forms-post"), machine.post.clone()),
        (
            t!(ui, "forms-gcode-units"),
            gcode_units_text(ui, machine.gcode_units),
        ),
        (t!(ui, "forms-spindle"), spindle_text(ui, machine.spindle)),
        (
            t!(ui, "forms-max-feed"),
            max_feed_text(ui, machine.max_feed),
        ),
        (
            t!(ui, "forms-tool-changes"),
            tool_change_name(ui, machine.tool_change),
        ),
        (
            t!(ui, "forms-travel"),
            travel_text(ui, machine.travel.as_ref()),
        ),
    ] {
        ui.label(label);
        ui.weak(value);
        ui.end_row();
    }
}

/// "mm (G21)" or "inch (G20)".
pub(super) fn gcode_units_text(ui: &egui::Ui, units: Units) -> String {
    match units {
        Units::Mm => t!(ui, "forms-gcode-mm"),
        Units::Inch => t!(ui, "forms-gcode-inch"),
    }
}

/// Safe Z: a job value, editable also while the machine is linked.
pub(in crate::app) fn safe_z_row(ui: &mut egui::Ui, label: &str, machine: &mut Machine) {
    ui.label(label).on_hover_text(t!(ui, "forms-safe-z-hover"));
    flag(
        ui.add(crate::units::length(&mut machine.safe_z).speed(0.1)),
        ui,
        "machine.safe_z",
    );
    ui.end_row();
}

/// A small button after a value that differs from the profile's: hover shows the
/// profile's value, a click asks to take it.
pub(super) fn revert_mark(
    ui: &mut egui::Ui,
    profile: Option<&MachineProfile>,
    value: &str,
) -> bool {
    let name = profile.map_or("", |p| p.name.as_str());
    super::header::icon_button(ui, crate::icons::Icon::Undo, "", true, false, 18.0)
        .on_hover_text(t!(ui, "forms-revert-hover", name = name, value = value))
        .clicked()
}

pub(super) fn spindle_text(ui: &egui::Ui, spindle: Spindle) -> String {
    match spindle {
        Spindle::Cw => t!(ui, "forms-spindle-cw"),
        Spindle::Ccw => t!(ui, "forms-spindle-ccw"),
    }
}

pub(super) fn max_feed_text(ui: &egui::Ui, max: Option<f64>) -> String {
    let units = crate::units::display(ui.ctx());
    let feed_unit = chipload_pipeline::units::feed_unit(units);
    max.map_or(t!(ui, "forms-max-feed-not-set"), |f| {
        format!("{} {feed_unit}", feed_in(units, f))
    })
}

pub(super) fn travel_text(ui: &egui::Ui, travel: Option<&Travel>) -> String {
    let units = crate::units::display(ui.ctx());
    let len_unit = chipload_pipeline::units::length_unit(units);
    travel.map_or(t!(ui, "forms-travel-not-checked"), |t| {
        format!(
            "{} × {} × {} {len_unit}",
            len_in(units, t.x),
            len_in(units, t.y),
            len_in(units, t.z)
        )
    })
}

/// For a user post copied into the project: whether the file it came from has changed,
/// with Update (docs/specs/user-posts.md §2).
pub(super) fn post_copy_row(
    ui: &mut egui::Ui,
    machine: &Machine,
    library: Option<&Library>,
    request: &mut Option<JobRequest>,
) {
    let Some(copy) = &machine.custom_post else {
        return;
    };
    let file = library.and_then(|l| l.user_posts.iter().find(|(name, _)| *name == machine.post));
    ui.label("");
    match file.map(|(_, read)| read) {
        Some(Ok(read)) if read.config == copy.config && read.template == copy.template => {
            ui.label(RichText::new(t!(ui, "forms-post-yours-copied")).weak());
        }
        Some(Ok(_)) => {
            ui.horizontal(|ui| {
                ui.colored_label(ui.visuals().warn_fg_color, t!(ui, "forms-post-changed"));
                if ui
                    .button(t!(ui, "forms-post-update"))
                    .on_hover_text(t!(ui, "forms-post-update-hover"))
                    .clicked()
                {
                    *request = Some(JobRequest::Post(machine.post.clone()));
                }
            });
        }
        _ => {
            ui.label(RichText::new(t!(ui, "forms-post-file-missing")).weak());
        }
    }
    ui.end_row();
}

pub(super) fn tool_change_name(ui: &egui::Ui, change: ToolChange) -> String {
    match change {
        ToolChange::Files => t!(ui, "forms-tool-change-files"),
        ToolChange::Program => t!(ui, "forms-tool-change-program"),
    }
}

/// Type, diameter, spindle speed, flutes and centre cutting.
/// `at`: the tool's path in the project (`tools.0`), for its problems; empty elsewhere.
pub(in crate::app) fn tool_rows(ui: &mut egui::Ui, salt: &str, at: &str, tool: &mut Tool) {
    ui.label(t!(ui, "forms-tool-type"));
    ComboBox::from_id_salt((salt, "tool-type"))
        .selected_text(tool_kind_name(ui, tool.kind))
        .show_ui(ui, |ui| {
            for kind in [ToolType::Endmill, ToolType::Drill, ToolType::Vbit] {
                let name = tool_kind_name(ui, kind);
                ui.selectable_value(&mut tool.kind, kind, name);
            }
        });
    ui.end_row();
    ui.label(t!(ui, "forms-diameter"));
    flag(
        ui.add(mm(&mut tool.diameter)),
        ui,
        &format!("{at}.diameter"),
    );
    ui.end_row();
    ui.label(t!(ui, "forms-spindle-speed"));
    let mut rpm = tool.rpm.unwrap_or(0.0);
    let response = ui.add(
        crate::units::number(ui, &mut rpm)
            .speed(100.0)
            .range(0.0..=100_000.0)
            .suffix(" rpm"),
    );
    if flag(response, ui, &format!("{at}.rpm")).changed() {
        tool.rpm = (rpm > 0.0).then_some(rpm);
    }
    ui.end_row();
    // Ticked: the number of cutting edges, for the chip load; unticked: unknown.
    ui.label(t!(ui, "forms-flutes"))
        .on_hover_text(t!(ui, "forms-flutes-hover"));
    ui.horizontal(|ui| {
        let mut set = tool.flutes.is_some();
        if ui.checkbox(&mut set, "").changed() {
            tool.flutes = set.then_some(2);
        }
        if let Some(flutes) = &mut tool.flutes {
            flag(
                ui.add(crate::units::number(ui, flutes).range(1..=16)),
                ui,
                &format!("{at}.flutes"),
            );
        }
    });
    ui.end_row();
    ui.label(t!(ui, "forms-centre-cutting"));
    let cutting_name = |ui: &egui::Ui, cutting: Option<bool>| match cutting {
        Some(true) => t!(ui, "forms-yes"),
        Some(false) => t!(ui, "forms-no"),
        None => t!(ui, "forms-unknown"),
    };
    ComboBox::from_id_salt((salt, "center-cutting"))
        .selected_text(cutting_name(ui, tool.center_cutting))
        .show_ui(ui, |ui| {
            for cutting in [Some(true), Some(false), None] {
                let name = cutting_name(ui, cutting);
                ui.selectable_value(&mut tool.center_cutting, cutting, name);
            }
        });
    ui.end_row();
    if tool.kind == ToolType::Vbit {
        vbit_rows(ui, at, tool);
    }
}

/// A V-bit's included angle (degrees here, radians in the file) and flat tip
/// (docs/specs/v-carve.md §2). Both can be left unset; V-carving needs the angle.
pub(super) fn vbit_rows(ui: &mut egui::Ui, at: &str, tool: &mut Tool) {
    ui.label(t!(ui, "forms-angle"))
        .on_hover_text(t!(ui, "forms-angle-hover"));
    let angle_row = ui.horizontal(|ui| {
        let mut set = tool.angle.is_some();
        if ui.checkbox(&mut set, "").changed() {
            tool.angle = set.then_some(60f64.to_radians());
        }
        if let Some(angle) = &mut tool.angle {
            let mut degrees = angle.to_degrees();
            if ui
                .add(
                    crate::units::number(ui, &mut degrees)
                        .speed(0.5)
                        .range(1.0..=179.0)
                        .suffix("°"),
                )
                .changed()
            {
                *angle = degrees.to_radians();
            }
        } else {
            ui.weak(t!(ui, "forms-unknown"));
        }
    });
    flag(angle_row.response, ui, &format!("{at}.angle"));
    ui.end_row();
    ui.label(t!(ui, "forms-tip"))
        .on_hover_text(t!(ui, "forms-tip-hover"));
    ui.horizontal(|ui| {
        let mut set = tool.tip_diameter.is_some();
        if ui.checkbox(&mut set, "").changed() {
            tool.tip_diameter = set.then_some(0.0);
        }
        match &mut tool.tip_diameter {
            Some(tip) => {
                flag(ui.add(mm(tip)), ui, &format!("{at}.tip_diameter"));
            }
            None => {
                ui.weak(t!(ui, "forms-pointed"));
            }
        }
    });
    ui.end_row();
}
