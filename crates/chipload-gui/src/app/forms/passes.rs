// SPDX-License-Identifier: GPL-3.0-or-later
//! The forms of a pocket's rest passes and a V-carve's floor pass, which share their rows.

use super::*;

/// A clearing pass's own rows, for a rest pass and a floor pass alike: on or off, its end
/// mill (one that `fits`), then `after_tool`'s rows, its step down, stepover, feeds and
/// entry. `salt` keeps its widgets apart from another pass's.
pub(super) fn clearing_pass_rows(
    ui: &mut egui::Ui,
    salt: impl std::hash::Hash + std::fmt::Debug + Copy,
    entry_salt: usize,
    pass: &mut ClearingPass,
    tools: &[Tool],
    fits: impl Fn(&Tool) -> bool,
    after_tool: impl FnOnce(&mut egui::Ui),
) {
    let units = crate::units::display(ui.ctx());
    ui.label(t!(ui, "forms-cut-it"));
    ui.checkbox(&mut pass.enabled, "")
        .on_hover_text(t!(ui, "forms-cut-it-hover"));
    ui.end_row();
    ui.label(t!(ui, "forms-tool"));
    let lang = ui.lang();
    let label = |id: i64| {
        tools
            .iter()
            .find(|t| t.id == id)
            .map_or(t!(lang, "forms-tool-not-in-project", tool = id), |t| {
                tool_label(ui, t, units)
            })
    };
    let combo = ComboBox::from_id_salt((salt, "tool"))
        .selected_text(label(pass.tool))
        .show_ui(ui, |ui| {
            for t in tools.iter().filter(|t| fits(t)) {
                ui.selectable_value(&mut pass.tool, t.id, tool_label(ui, t, units));
            }
        });
    flag(combo.response, ui, "tool");
    ui.end_row();
    after_tool(ui);
    ui.label(t!(ui, "forms-step-down"));
    flag(ui.add(mm(&mut pass.step_down)), ui, "step_down");
    ui.end_row();
    slot_step_row(ui, &mut pass.slot_step_down, pass.step_down);
    ui.label(t!(ui, "forms-stepover"));
    let stepover = ui.add(
        crate::units::number(ui, &mut pass.stepover)
            .speed(0.01)
            .range(0.01..=1.0)
            .suffix(" × D"),
    );
    flag(stepover, ui, "stepover");
    ui.end_row();
    ui.label(t!(ui, "forms-feed"));
    flag(ui.add(feed(&mut pass.feed)), ui, "feed");
    ui.end_row();
    ui.label(t!(ui, "forms-slot-feed"))
        .on_hover_text(t!(ui, "forms-slot-feed-hover"));
    let mut slot_feed = pass.slot_feed.unwrap_or(pass.feed);
    let field = ui.add(feed(&mut slot_feed));
    if field.changed() {
        pass.slot_feed = Some(slot_feed);
    }
    flag(field, ui, "slot_feed");
    ui.end_row();
    ui.label(t!(ui, "forms-plunge-feed"));
    flag(ui.add(feed(&mut pass.plunge_feed)), ui, "plunge_feed");
    ui.end_row();
    let diameter = tools.iter().find(|t| t.id == pass.tool).map(|t| t.diameter);
    entry_rows(ui, entry_salt, &mut pass.entry, diameter);
}

/// Pocket `i`'s rest pass `k` (docs/specs/rest-passes.md §6): its own values, and what it
/// takes from its pocket as facts. None when there is no such pass (after an undo).
pub(in crate::app) fn rest_pass_form(
    ui: &mut egui::Ui,
    edit: &mut EditJob,
    i: usize,
    k: usize,
) -> Option<JobRequest> {
    let units = crate::units::display(ui.ctx());
    let len_unit = chipload_pipeline::units::length_unit(units);
    let tools = edit.job.tools.clone();
    let Some(Operation::Pocket(pocket)) = edit.job.operations.get_mut(i) else {
        return None;
    };
    let before = match k {
        0 => pocket.tool,
        _ => pocket.rest_passes.get(k - 1)?.tool,
    };
    let before_diameter = tools.iter().find(|t| t.id == before).map(|t| t.diameter);
    let pocket_name = match pocket.name.as_str() {
        "" => t!(ui, "forms-rest-pass-of", number = i + 1),
        name => t!(ui, "forms-rest-pass-of-named", number = i + 1, name = name),
    };
    let facts = t!(
        ui,
        "forms-rest-pass-facts",
        start = len_in(units, pocket.start_depth),
        end = len_in(units, pocket.end_depth),
        unit = len_unit,
        clearance = len_in(units, pocket.clearance),
        direction = direction_name(ui, pocket.direction)
    );
    let pass = pocket.rest_passes.get_mut(k)?;
    let at = chipload_job::OpRef::pass(i, k);
    let mut request = None;
    // A pass clears like a pocket: its colour (docs/specs/look-and-feel.md §4).
    let color = crate::style::op_color(
        chipload_pipeline::new_ops::OpKind::Pocket,
        ui.visuals().dark_mode,
    );
    set_card_stripe(ui, color);
    super::browser::title_line(
        ui,
        crate::icons::Icon::Pocket,
        Some(color),
        &format!(
            "{} · {}",
            t!(ui, "forms-rest-pass"),
            t!(ui, "forms-operation-number", number = at.to_string())
        ),
    );
    crate::app::forms::card(ui, |ui| {
        Grid::new(("rest-pass", i, k))
            .num_columns(2)
            .min_col_width(label_width(ui) + ui.spacing().indent)
            .show(ui, |ui| {
                ui.label(t!(ui, "forms-rest-of"))
                    .on_hover_text(t!(ui, "forms-rest-of-hover"));
                ui.horizontal(|ui| {
                    ui.label(&pocket_name);
                    if ui.small_button(t!(ui, "forms-open")).clicked() {
                        request = Some(JobRequest::OpenOperation(i));
                    }
                });
                ui.end_row();
                ui.label(t!(ui, "forms-from-pocket"));
                ui.weak(&facts);
                ui.end_row();
                ui.label(t!(ui, "forms-after-tool"))
                    .on_hover_text(t!(ui, "forms-after-tool-hover"));
                ui.weak(format!("T{before}"));
                ui.end_row();
                // Smaller than the tool before it (docs/specs/rest-passes.md §2).
                let fits = |t: &Tool| {
                    t.kind == ToolType::Endmill && before_diameter.is_some_and(|d| t.diameter < d)
                };
                let salt = ("rest-pass", i, k);
                clearing_pass_rows(ui, salt, 1000 + i * 100 + k, pass, &tools, fits, |_| {});
            })
    });
    ui.add_space(6.0);
    ui.weak(t!(ui, "forms-rest-pass-note"));
    ui.add_space(6.0);
    if ui.button(t!(ui, "forms-remove-rest-pass")).clicked() {
        request = Some(JobRequest::RemoveRestPass(i, k));
    }
    request
}

/// V-carve `i`'s floor pass (docs/specs/vcarve-wide-strokes.md §7): its own values, what it
/// takes from its V-carve as facts, and, when its end mill leaves part of the floor, the
/// largest that would clear it all and the library's that do (`hint`). None when there is
/// no floor pass (after an undo).
pub(in crate::app) fn floor_pass_form(
    ui: &mut egui::Ui,
    edit: &mut EditJob,
    i: usize,
    hint: Option<&(f64, Vec<(String, f64)>)>,
) -> Option<JobRequest> {
    let units = crate::units::display(ui.ctx());
    let len_unit = chipload_pipeline::units::length_unit(units);
    let tools = edit.job.tools.clone();
    let Some(Operation::Vcarve(vcarve)) = edit.job.operations.get_mut(i) else {
        return None;
    };
    let vcarve_name = match vcarve.name.as_str() {
        "" => t!(ui, "forms-rest-pass-of", number = i + 1),
        name => t!(ui, "forms-rest-pass-of-named", number = i + 1, name = name),
    };
    let inset = tools
        .iter()
        .find(|t| t.id == vcarve.tool)
        .and_then(|bit| vcarve.floor_inset(bit))
        .unwrap_or(0.0);
    let facts = t!(
        ui,
        "forms-floor-pass-facts",
        start = len_in(units, vcarve.start_depth),
        end = len_in(units, vcarve.end_depth),
        unit = len_unit,
        inset = len_in(units, inset)
    );
    let pass = vcarve.floor.as_mut()?;
    let at = chipload_job::OpRef::pass(i, 0);
    let mut request = None;
    // A pass clears like a pocket: its colour (docs/specs/look-and-feel.md §4).
    let color = crate::style::op_color(
        chipload_pipeline::new_ops::OpKind::Pocket,
        ui.visuals().dark_mode,
    );
    set_card_stripe(ui, color);
    super::browser::title_line(
        ui,
        crate::icons::Icon::Pocket,
        Some(color),
        &format!(
            "{} · {}",
            t!(ui, "forms-floor-pass"),
            t!(ui, "forms-operation-number", number = at.to_string())
        ),
    );
    crate::app::forms::card(ui, |ui| {
        Grid::new(("floor-pass", i))
            .num_columns(2)
            .min_col_width(label_width(ui) + ui.spacing().indent)
            .show(ui, |ui| {
                ui.label(t!(ui, "forms-floor-of"))
                    .on_hover_text(t!(ui, "forms-floor-of-hover"));
                ui.horizontal(|ui| {
                    ui.label(&vcarve_name);
                    if ui.small_button(t!(ui, "forms-open")).clicked() {
                        request = Some(JobRequest::OpenOperation(i));
                    }
                });
                ui.end_row();
                ui.label(t!(ui, "forms-from-vcarve"));
                ui.weak(&facts);
                ui.end_row();
                let fits = |t: &Tool| t.kind == ToolType::Endmill;
                // When the end mill leaves part of the floor: the largest that clears it all.
                let hint_rows = |ui: &mut egui::Ui| {
                    if let Some((largest, library)) = hint {
                        ui.label("");
                        ui.vertical(|ui| {
                            let size = |d: f64| chipload_pipeline::units::length_text(d, units);
                            ui.weak(t!(ui, "forms-floor-hint", largest = size(*largest)));
                            if !library.is_empty() {
                                let names: Vec<String> = library
                                    .iter()
                                    .map(|(name, d)| format!("{name} ({})", size(*d)))
                                    .collect();
                                ui.weak(t!(
                                    ui,
                                    "forms-floor-hint-library",
                                    tools = names.join(", ")
                                ));
                            }
                        });
                        ui.end_row();
                    }
                };
                clearing_pass_rows(
                    ui,
                    ("floor-pass", i, 0),
                    2000 + i,
                    pass,
                    &tools,
                    fits,
                    hint_rows,
                );
            })
    });
    ui.add_space(6.0);
    ui.weak(t!(ui, "forms-floor-pass-note"));
    ui.add_space(6.0);
    if ui.button(t!(ui, "forms-remove-floor-pass")).clicked() {
        request = Some(JobRequest::RemoveFloorPass(i));
    }
    request
}

/// Relief `i`'s roughing pass (docs/specs/stl-relief.md §14): its tool, levels, lines,
/// allowance and feeds; the relief's model, area, depths and angle as facts. None when
/// there is no such pass (after an undo).
pub(in crate::app) fn roughing_pass_form(
    ui: &mut egui::Ui,
    edit: &mut EditJob,
    i: usize,
) -> Option<JobRequest> {
    let units = crate::units::display(ui.ctx());
    let len_unit = chipload_pipeline::units::length_unit(units);
    let tools = edit.job.tools.clone();
    let Some(Operation::Relief(relief)) = edit.job.operations.get_mut(i) else {
        return None;
    };
    let relief_name = match relief.name.as_str() {
        "" => t!(ui, "forms-rest-pass-of", number = i + 1),
        name => t!(ui, "forms-rest-pass-of-named", number = i + 1, name = name),
    };
    let facts = t!(
        ui,
        "forms-roughing-facts",
        start = len_in(units, relief.start_depth),
        end = len_in(units, relief.end_depth),
        unit = len_unit,
        angle = format!("{:.0}", relief.angle.to_degrees())
    );
    let pass = relief.roughing.as_mut()?;
    let at = chipload_job::OpRef::pass(i, 0);
    let mut request = None;
    let color = crate::style::op_color(
        chipload_pipeline::new_ops::OpKind::Relief,
        ui.visuals().dark_mode,
    );
    set_card_stripe(ui, color);
    super::browser::title_line(
        ui,
        crate::icons::Icon::Relief,
        Some(color),
        &format!(
            "{} · {}",
            t!(ui, "forms-roughing-pass"),
            t!(ui, "forms-operation-number", number = at.to_string())
        ),
    );
    // Tools a roughing takes: an end mill or a plain ball nose.
    let fits = |t: &Tool| {
        t.kind == ToolType::Endmill || (t.kind == ToolType::Ballnose && t.angle.is_none())
    };
    crate::app::forms::card(ui, |ui| {
        Grid::new(("roughing-pass", i))
            .num_columns(2)
            .min_col_width(label_width(ui) + ui.spacing().indent)
            .show(ui, |ui| {
                ui.label(t!(ui, "forms-roughing-of"))
                    .on_hover_text(t!(ui, "forms-roughing-of-hover"));
                ui.horizontal(|ui| {
                    ui.label(&relief_name);
                    if ui.small_button(t!(ui, "forms-open")).clicked() {
                        request = Some(JobRequest::OpenOperation(i));
                    }
                });
                ui.end_row();
                ui.label(t!(ui, "forms-from-relief"));
                ui.weak(&facts);
                ui.end_row();
                ui.label(t!(ui, "forms-cut-it"));
                ui.checkbox(&mut pass.enabled, "")
                    .on_hover_text(t!(ui, "forms-cut-it-hover"));
                ui.end_row();
                ui.label(t!(ui, "forms-tool"));
                let lang = ui.lang();
                let label = |id: i64| {
                    tools
                        .iter()
                        .find(|t| t.id == id)
                        .map_or(t!(lang, "forms-tool-not-in-project", tool = id), |t| {
                            tool_label(ui, t, units)
                        })
                };
                let combo = ComboBox::from_id_salt(("roughing-pass", i, "tool"))
                    .selected_text(label(pass.tool))
                    .show_ui(ui, |ui| {
                        for t in tools.iter().filter(|t| fits(t)) {
                            ui.selectable_value(&mut pass.tool, t.id, tool_label(ui, t, units));
                        }
                    });
                flag(combo.response, ui, "tool");
                ui.end_row();
                ui.label(t!(ui, "forms-step-down"))
                    .on_hover_text(t!(ui, "forms-roughing-step-down-hover"));
                flag(ui.add(mm(&mut pass.step_down)), ui, "step_down");
                ui.end_row();
                ui.label(t!(ui, "forms-stepover"));
                let stepover = ui.add(
                    crate::units::number(ui, &mut pass.stepover)
                        .speed(0.01)
                        .range(0.01..=chipload_job::RELIEF_MAX_STEPOVER)
                        .suffix(" × D"),
                );
                flag(stepover, ui, "stepover");
                ui.end_row();
                ui.label(t!(ui, "forms-roughing-allowance"))
                    .on_hover_text(t!(ui, "forms-roughing-allowance-hover"));
                flag(ui.add(mm(&mut pass.allowance)), ui, "allowance");
                ui.end_row();
                ui.label(t!(ui, "forms-feed"));
                flag(ui.add(feed(&mut pass.feed)), ui, "feed");
                ui.end_row();
                ui.label(t!(ui, "forms-plunge-feed"));
                flag(ui.add(feed(&mut pass.plunge_feed)), ui, "plunge_feed");
                ui.end_row();
            })
    });
    ui.add_space(6.0);
    ui.weak(t!(ui, "forms-roughing-note"));
    ui.add_space(6.0);
    if ui.button(t!(ui, "forms-remove-roughing")).clicked() {
        request = Some(JobRequest::RemoveRoughingPass(i));
    }
    request
}
