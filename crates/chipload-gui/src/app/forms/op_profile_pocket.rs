// SPDX-License-Identifier: GPL-3.0-or-later
//! The forms of profiles and pockets: holding the part, finishing passes.

use super::*;

/// How a profile holds its part (docs/specs/holding.md): none, tabs, an onion skin, or
/// both.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::app) enum Holding {
    None,
    Tabs,
    Skin,
    SkinAndTabs,
}

impl Holding {
    pub(in crate::app) const ALL: [Holding; 4] = [
        Holding::None,
        Holding::Tabs,
        Holding::Skin,
        Holding::SkinAndTabs,
    ];

    pub(in crate::app) fn of(p: &ProfileOp) -> Holding {
        match (p.tabs.is_some(), p.skin.is_some()) {
            (false, false) => Holding::None,
            (true, false) => Holding::Tabs,
            (false, true) => Holding::Skin,
            (true, true) => Holding::SkinAndTabs,
        }
    }

    fn name(self, ui: &egui::Ui) -> String {
        match self {
            Holding::None => t!(ui, "forms-holding-none"),
            Holding::Tabs => t!(ui, "forms-holding-tabs"),
            Holding::Skin => t!(ui, "forms-holding-skin"),
            Holding::SkinAndTabs => t!(ui, "forms-holding-skin-tabs"),
        }
    }

    fn has_tabs(self) -> bool {
        matches!(self, Holding::Tabs | Holding::SkinAndTabs)
    }

    fn has_skin(self) -> bool {
        matches!(self, Holding::Skin | Holding::SkinAndTabs)
    }

    /// Set `p` to this choice, keeping the tabs or skin it already has where they stay.
    pub(in crate::app) fn apply(self, p: &mut ProfileOp, center_cutting: Option<bool>) {
        if !self.has_tabs() {
            p.tabs = None;
        } else if p.tabs.is_none() {
            p.tabs = Some(Tabs {
                count: 4,
                width: 8.0,
                height: 3.0,
                flank_angle: Tabs::default_flank(p.ramp_angle, center_cutting),
                at: Vec::new(),
                along: None,
            });
        }
        if !self.has_skin() {
            p.skin = None;
        } else if p.skin.is_none() {
            p.skin = Some(Skin {
                thickness: p.step_down.min(0.3),
            });
        }
    }
}

/// A profile's tabs and onion skin as one choice, with the chosen kind's rows
/// (docs/specs/holding.md §2).
pub(super) fn holding_section(ui: &mut egui::Ui, p: &mut ProfileOp, op_tool: Option<&Tool>) {
    let units = crate::units::display(ui.ctx());
    let len_unit = chipload_pipeline::units::length_unit(units);
    let holding = Holding::of(p);
    let mut summary = holding.name(ui);
    if let Some(tabs) = &p.tabs {
        summary = t!(
            ui,
            "forms-holding-summary-tabs",
            holding = summary,
            count = tabs.count,
            width = len_in(units, tabs.width),
            unit = len_unit,
            height = len_in(units, tabs.height)
        );
    }
    if let Some(skin) = p.skin {
        summary = t!(
            ui,
            "forms-holding-summary-skin",
            summary = summary,
            thickness = format!("{} {len_unit}", len_in(units, skin.thickness))
        );
    }
    folded_section(
        ui,
        "op-holding",
        &t!(ui, "forms-section-holding"),
        &summary,
        |ui| {
            ui.label(t!(ui, "forms-hold-the-part"))
                .on_hover_text(t!(ui, "forms-hold-the-part-hover"));
            let mut chosen = holding;
            ComboBox::from_id_salt("holding")
                .selected_text(holding.name(ui))
                .show_ui(ui, |ui| {
                    for choice in Holding::ALL {
                        ui.selectable_value(&mut chosen, choice, choice.name(ui));
                    }
                });
            if chosen != holding {
                chosen.apply(p, op_tool.and_then(|t| t.center_cutting));
            }
            ui.end_row();
            if let Some(tabs) = &mut p.tabs {
                ui.label(format!("  {}", t!(ui, "forms-tab-count")));
                flag(
                    ui.add(crate::units::number(ui, &mut tabs.count).range(1..=100)),
                    ui,
                    "tabs.count",
                );
                ui.end_row();
                // docs/specs/tabs-along-grain.md §2.
                ui.label(format!("  {}", t!(ui, "forms-tab-placement")))
                    .on_hover_text(t!(ui, "forms-tab-placement-hover"));
                let placement = |ui: &egui::Ui, along: Option<chipload_job::Axis>| match along {
                    None => t!(ui, "forms-tab-around"),
                    Some(chipload_job::Axis::X) => t!(ui, "forms-tab-along-x"),
                    Some(chipload_job::Axis::Y) => t!(ui, "forms-tab-along-y"),
                };
                let hover = |ui: &egui::Ui, along: Option<chipload_job::Axis>| match along {
                    None => t!(ui, "forms-tab-around-hover"),
                    Some(chipload_job::Axis::X) => t!(ui, "forms-tab-along-x-hover"),
                    Some(chipload_job::Axis::Y) => t!(ui, "forms-tab-along-y-hover"),
                };
                let combo = ComboBox::from_id_salt("tab-placement")
                    .selected_text(placement(ui, tabs.along))
                    .show_ui(ui, |ui| {
                        for choice in [
                            None,
                            Some(chipload_job::Axis::X),
                            Some(chipload_job::Axis::Y),
                        ] {
                            ui.selectable_value(&mut tabs.along, choice, placement(ui, choice))
                                .on_hover_text(hover(ui, choice));
                        }
                    });
                flag(combo.response, ui, "tabs.along");
                ui.end_row();
                ui.label(format!("  {}", t!(ui, "forms-tab-width")));
                flag(ui.add(mm(&mut tabs.width)), ui, "tabs.width");
                ui.end_row();
                ui.label(format!("  {}", t!(ui, "forms-tab-height")));
                flag(ui.add(mm(&mut tabs.height)), ui, "tabs.height");
                ui.end_row();
                flank_row(ui, tabs);
                // docs/specs/tabs-by-hand.md §5.
                if !tabs.at.is_empty() {
                    ui.label("");
                    ui.horizontal(|ui| {
                        ui.label(t!(ui, "forms-tabs-placed", count = tabs.at.len()))
                            .on_hover_text(t!(ui, "forms-tabs-placed-hover"));
                        if ui
                            .button(t!(ui, "forms-tabs-reset"))
                            .on_hover_text(t!(ui, "forms-tabs-reset-hover"))
                            .clicked()
                        {
                            tabs.at.clear();
                        }
                    });
                    ui.end_row();
                }
            }
            if let Some(skin) = &mut p.skin {
                ui.label(format!("  {}", t!(ui, "forms-skin-thickness")))
                    .on_hover_text(t!(ui, "forms-skin-thickness-hover"));
                flag(ui.add(mm(&mut skin.thickness)), ui, "skin.thickness");
                ui.end_row();
            }
        },
    );
}

/// A profile's finishing pass (docs/specs/finish-and-skin.md).
pub(super) fn finish_section(ui: &mut egui::Ui, p: &mut ProfileOp) {
    let units = crate::units::display(ui.ctx());
    let len_unit = chipload_pipeline::units::length_unit(units);
    let summary = match p.finish {
        None => t!(ui, "forms-finish-none"),
        Some(f) => t!(
            ui,
            "forms-finish-summary",
            stock = format!("{} {len_unit}", len_in(units, f.stock))
        ),
    };
    let title = t!(ui, "forms-finishing-pass");
    folded_section(ui, "op-finish", &title, &summary, |ui| {
        ui.label(t!(ui, "forms-finishing-pass"))
            .on_hover_text(t!(ui, "forms-finishing-pass-hover"));
        let mut on = p.finish.is_some();
        let response = ui
            .add_enabled(
                p.side != Side::On || on,
                egui::Checkbox::new(&mut on, t!(ui, "forms-finishing-lap")),
            )
            .on_disabled_hover_text(t!(ui, "forms-finishing-disabled"));
        if flag(response, ui, "finish").changed() {
            p.finish = on.then_some(Finish { stock: 0.3 });
        }
        ui.end_row();
        if let Some(finish) = &mut p.finish {
            ui.label(format!("  {}", t!(ui, "forms-finish-stock")));
            flag(ui.add(mm(&mut finish.stock)), ui, "finish.stock");
            ui.end_row();
        }
    });
}

/// A pocket's finishing pass (docs/specs/pocket-finish.md §6): a tick, then the wall and
/// floor allowances. Ticking it starts at wall 0.3 mm and floor 0.2 mm (none on the floor of
/// a pocket cut through).
pub(super) fn pocket_finish_section(ui: &mut egui::Ui, p: &mut chipload_job::PocketOp) {
    let units = crate::units::display(ui.ctx());
    let len_unit = chipload_pipeline::units::length_unit(units);
    let summary = match p.finish {
        None => t!(ui, "forms-finish-none"),
        Some(f) => t!(
            ui,
            "forms-pocket-finish-summary",
            wall = format!("{} {len_unit}", len_in(units, f.wall)),
            floor = format!("{} {len_unit}", len_in(units, f.floor))
        ),
    };
    let title = t!(ui, "forms-finishing-pass");
    folded_section(ui, "op-pocket-finish", &title, &summary, |ui| {
        ui.label(t!(ui, "forms-finishing-pass"))
            .on_hover_text(t!(ui, "forms-pocket-finish-hover"));
        let mut on = p.finish.is_some();
        let response = ui
            .add_enabled(
                p.rest.is_none() || on,
                egui::Checkbox::new(&mut on, t!(ui, "forms-pocket-finish-tick")),
            )
            .on_disabled_hover_text(t!(ui, "forms-pocket-finish-rest"));
        if flag(response, ui, "finish").changed() {
            p.finish = on.then(|| {
                let mut finish = chipload_job::PocketFinish::DEFAULT;
                if p.through.is_some() {
                    finish.floor = 0.0;
                }
                finish
            });
        }
        ui.end_row();
        if let Some(finish) = &mut p.finish {
            ui.label(format!("  {}", t!(ui, "forms-pocket-finish-wall")))
                .on_hover_text(t!(ui, "forms-pocket-finish-wall-hover"));
            flag(ui.add(mm(&mut finish.wall)), ui, "finish.wall");
            ui.end_row();
            ui.label(format!("  {}", t!(ui, "forms-pocket-finish-floor")))
                .on_hover_text(t!(ui, "forms-pocket-finish-floor-hover"));
            flag(ui.add(mm(&mut finish.floor)), ui, "finish.floor");
            ui.end_row();
        }
    });
}

/// A profile's sections: its cut, feeds, how the part is held, finishing.
pub(super) fn profile_sections(
    ui: &mut egui::Ui,
    index: usize,
    p: &mut ProfileOp,
    stock: StockDepths,
    op_tool: Option<&Tool>,
) {
    let units = crate::units::display(ui.ctx());
    let len_unit = chipload_pipeline::units::length_unit(units);
    let cut = t!(
        ui,
        "forms-profile-cut-summary",
        start = len_in(units, p.start_depth),
        end = len_in(units, p.end_depth),
        unit = len_unit,
        step = len_in(units, p.step_down),
        direction = direction_name(ui, p.direction),
        ramp = degrees(p.ramp_angle)
    );
    folded_section(
        ui,
        "op-profile-cut",
        &t!(ui, "forms-section-cut"),
        &cut,
        |ui| {
            ui.label(t!(ui, "forms-direction"));
            direction_combo(ui, ("direction", index), &mut p.direction);
            ui.end_row();
            let can_clear = p.side != Side::On;
            if !can_clear {
                p.clearance = 0.0;
            }
            depth_rows(
                ui,
                &mut p.start_depth,
                &mut p.end_depth,
                &mut p.through,
                &mut p.clearance,
                can_clear,
                stock,
            );
            ui.label(t!(ui, "forms-step-down"));
            flag(ui.add(mm(&mut p.step_down)), ui, "step_down");
            ui.end_row();
            ramp_angle_row(ui, &mut p.ramp_angle);
            corner_rows(ui, index, &mut p.corners, &mut p.corner_choices, can_clear);
        },
    );
    feeds_section(ui, &mut p.feed, &mut p.plunge_feed, op_tool);
    holding_section(ui, p, op_tool);
    finish_section(ui, p);
}

/// A pocket's sections: its cut, clearing, feeds, entry and lead, finishing.
pub(super) fn pocket_sections(
    ui: &mut egui::Ui,
    index: usize,
    p: &mut chipload_job::PocketOp,
    tools: &[Tool],
    stock: StockDepths,
    op_tool: Option<&Tool>,
) {
    let units = crate::units::display(ui.ctx());
    let len_unit = chipload_pipeline::units::length_unit(units);
    let tool_diameter = op_tool.map_or(0.0, |t| t.diameter);
    let cut = t!(
        ui,
        "forms-pocket-cut-summary",
        start = len_in(units, p.start_depth),
        end = len_in(units, p.end_depth),
        unit = len_unit,
        step = len_in(units, p.step_down),
        direction = direction_name(ui, p.direction)
    );
    folded_section(ui, "op-cut", &t!(ui, "forms-section-cut"), &cut, |ui| {
        ui.label(t!(ui, "forms-direction"));
        direction_combo(ui, ("direction", index), &mut p.direction);
        ui.end_row();
        depth_rows(
            ui,
            &mut p.start_depth,
            &mut p.end_depth,
            &mut p.through,
            &mut p.clearance,
            true,
            stock,
        );
        ui.label(t!(ui, "forms-step-down"))
            .on_hover_text(t!(ui, "forms-pocket-step-down-hover"));
        flag(ui.add(mm(&mut p.step_down)), ui, "step_down");
        ui.end_row();
        slot_step_row(ui, &mut p.slot_step_down, p.step_down);
        corner_rows(ui, index, &mut p.corners, &mut p.corner_choices, true);
    });
    let stepover = format!("{:.2}", p.stepover);
    let clearing = match p.rest {
        None => t!(ui, "forms-clearing-summary", stepover = stepover),
        Some(rest) => t!(
            ui,
            "forms-clearing-summary-rest",
            stepover = stepover,
            tool = rest.after_tool
        ),
    };
    let title = t!(ui, "forms-section-clearing");
    folded_section(ui, "op-clearing", &title, &clearing, |ui| {
        rest_row(ui, index, &mut p.rest, tools, op_tool);
        ui.label(t!(ui, "forms-stepover"))
            .on_hover_text(t!(ui, "forms-stepover-hover"));
        let stepover = ui.add(
            crate::units::number(ui, &mut p.stepover)
                .speed(0.01)
                .range(0.01..=1.0)
                .suffix(" × D"),
        );
        flag(stepover, ui, "stepover");
        ui.end_row();
    });
    feeds_section_with(
        ui,
        &mut p.feed,
        &mut p.plunge_feed,
        Some(&mut p.slot_feed),
        op_tool,
    );
    let entry_text = entry_summary(ui.lang(), &p.entry, p.lead.as_ref(), units);
    let title = t!(ui, "forms-section-entry-lead");
    folded_section(ui, "op-entry", &title, &entry_text, |ui| {
        entry_rows(ui, index, &mut p.entry, Some(tool_diameter));
        lead_rows(ui, &mut p.lead, tool_diameter);
    });
    pocket_finish_section(ui, p);
}
