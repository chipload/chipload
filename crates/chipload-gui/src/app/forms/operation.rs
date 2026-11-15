// SPDX-License-Identifier: GPL-3.0-or-later
//! An operation's form, and its rest and floor passes'.

use super::*;
use chipload_core::math;

/// The stock's thickness and how far below it a cut may go (`stock.max_overcut`), for an
/// end depth that cuts through the stock.
#[derive(Debug, Clone, Copy)]
pub(in crate::app) struct StockDepths {
    pub thickness: f64,
    pub max_overcut: f64,
}

/// An end depth with its Through box (docs/specs/job-settings-layout.md §5): ticked, the
/// depth is the stock's thickness plus how far below it the cut goes, shown but not typed.
pub(super) fn end_depth_rows(
    ui: &mut egui::Ui,
    end: &mut f64,
    through: &mut Option<f64>,
    stock: StockDepths,
    hover: String,
) {
    ui.label(t!(ui, "forms-end-depth")).on_hover_text(hover);
    wrapped_cell(ui, |ui| {
        if let Some(below) = through {
            *end = stock.thickness + *below;
        }
        flag(ui.add_enabled(through.is_none(), mm(end)), ui, "end_depth");
        let mut on = through.is_some();
        let check = ui
            .checkbox(&mut on, t!(ui, "forms-through"))
            .on_hover_text(t!(ui, "forms-through-hover"));
        if flag(check, ui, "through").changed() {
            *through = on.then_some(stock.max_overcut);
        }
    });
    ui.end_row();
    if let Some(below) = through {
        let units = crate::units::display(ui.ctx());
        let len_unit = chipload_pipeline::units::length_unit(units);
        ui.label(t!(ui, "forms-through-below")).on_hover_text(t!(
            ui,
            "forms-through-below-hover",
            depth = format!("{} {len_unit}", len_in(units, stock.max_overcut))
        ));
        flag(ui.add(mm(below)), ui, "through");
        *end = stock.thickness + *below;
        ui.end_row();
    }
}

/// Start depth, end depth (with its Through box) and clearance
/// (docs/specs/depths-and-clearance.md).
#[allow(clippy::too_many_arguments)]
pub(super) fn depth_rows(
    ui: &mut egui::Ui,
    start: &mut f64,
    end: &mut f64,
    through: &mut Option<f64>,
    clearance: &mut f64,
    can_clear: bool,
    stock: StockDepths,
) {
    ui.label(t!(ui, "forms-start-depth"))
        .on_hover_text(t!(ui, "forms-start-depth-hover"));
    flag(ui.add(mm(start)), ui, "start_depth");
    ui.end_row();
    let hover = t!(ui, "forms-end-depth-hover");
    end_depth_rows(ui, end, through, stock, hover);
    ui.label(t!(ui, "forms-oversize")).on_hover_text(t!(
        ui,
        "units-text-oversize-hover",
        unit = crate::units::unit(ui.ctx())
    ));
    let response = ui
        .add_enabled(
            can_clear,
            crate::units::length(clearance)
                .speed(0.01)
                .range(-100_000.0..=100_000.0),
        )
        .on_disabled_hover_text(t!(ui, "forms-oversize-disabled"));
    flag(response, ui, "clearance");
    ui.end_row();
}

/// An operation's chip load as a row of its own under the feed, with where typical values
/// lie on hover, in the app's language and display units. When it can't be worked out, a
/// short note says which tool value is missing; the hover says where to set it (the
/// user, 2026-10-29: the long sentence beside the feed ran over the next row).
pub(super) fn chip_load_row(
    ui: &mut egui::Ui,
    load: chipload_pipeline::chip_load::ChipLoad,
    units: Units,
) {
    use chipload_pipeline::chip_load::ChipLoad;
    let (lang, units) = (ui.lang(), units.into());
    let hint = chipload_pipeline::chip_load::chip_load_hint().render(lang, units);
    ui.label(t!(ui, "forms-chip-load"))
        .on_hover_text(hint.clone());
    let short = match load {
        ChipLoad::Mm(_) | ChipLoad::NoTool => load.message().render(lang, units),
        ChipLoad::NoFlutes => t!(ui, "forms-chip-load-no-flutes"),
        ChipLoad::NoRpm => t!(ui, "forms-chip-load-no-rpm"),
    };
    let hover = match load {
        ChipLoad::NoFlutes | ChipLoad::NoRpm => t!(ui, "forms-chip-load-missing-hover"),
        _ => hint,
    };
    ui.label(RichText::new(short).weak()).on_hover_text(hover);
    ui.end_row();
}

/// The tabs' flank angle in degrees, and how far the feet reach (docs/specs/
/// profile-spiral.md §4).
pub(super) fn flank_row(ui: &mut egui::Ui, tabs: &mut Tabs) {
    let units = crate::units::display(ui.ctx());
    let len_unit = chipload_pipeline::units::length_unit(units);
    ui.label(format!("  {}", t!(ui, "forms-flank-angle")))
        .on_hover_text(t!(ui, "forms-flank-angle-hover"));
    ui.horizontal(|ui| {
        let mut value = tabs.flank_angle.to_degrees();
        let response = ui.add(
            crate::units::number(ui, &mut value)
                .speed(0.5)
                .range(0.5..=90.0)
                .suffix("°"),
        );
        if response.changed() {
            tabs.flank_angle = value.to_radians().min(STRAIGHT_FLANK_RAD);
        }
        flag(response, ui, "tabs.flank_angle");
        if tabs.is_straight() {
            ui.weak(t!(ui, "forms-flank-straight"));
        } else if tabs.height > 0.0 {
            let foot = tabs.height / math::tan(tabs.flank_angle);
            let length = format!("{} {len_unit}", len_in(units, foot));
            ui.weak(t!(ui, "forms-flank-feet", length = length));
        }
    });
    ui.end_row();
}

pub(super) fn entry_summary(
    lang: Language,
    entry: &Entry,
    lead: Option<&Lead>,
    units: Units,
) -> String {
    let len_unit = chipload_pipeline::units::length_unit(units);
    let entry = match entry {
        Entry::Plunge {} => t!(lang, "forms-entry-plunge"),
        Entry::Ramp { ramp_angle } => t!(
            lang,
            "forms-entry-ramp-summary",
            angle = format!("{:.1}°", ramp_angle.to_degrees())
        ),
        Entry::Helix { radius, ramp_angle } => t!(
            lang,
            "forms-entry-helix-summary",
            radius = format!("{} {len_unit}", len_in(units, *radius)),
            angle = format!("{:.1}°", ramp_angle.to_degrees())
        ),
    };
    match lead {
        Some(lead) => t!(
            lang,
            "forms-entry-with-lead",
            entry = entry,
            radius = format!("{} {len_unit}", len_in(units, lead.radius))
        ),
        None => entry,
    }
}

/// What the operation form asks of the app beyond editing values.
#[derive(Debug, Default, PartialEq)]
pub(in crate::app) struct OpFormRequest {
    /// "Use selection" was clicked.
    pub use_selection: bool,
    /// Copy this library tool into the project and cut this operation with it.
    pub tool_from_library: Option<String>,
    /// Save the operation's values for its tool in its material.
    pub save_to_material: bool,
    /// The tool was changed: the operation takes its material's values for the new one.
    pub tool_changed: bool,
    /// A relief's depth was changed: model `id` gets this depth (mm).
    pub relief_depth: Option<(u64, f64)>,
}

/// What the operation form needs to know about tools: the project's, the library's names
/// (None: no library), and the operation's tool's cutting values from the library.
pub(in crate::app) struct ToolChoice<'a> {
    /// How many contours are selected on the canvas, and which (for *Use selection*).
    pub selected: usize,
    pub selection_text: &'a str,
    pub tools: &'a [Tool],
    pub library: Option<Vec<String>>,
    pub cutting: Option<Cutting>,
    /// The library's materials, each with its values for the operation's tool
    /// (docs/specs/materials.md §2). None: no library.
    pub materials: Option<Vec<(MaterialRef, Option<Cutting>)>>,
    /// A style's operation, not a project's (docs/specs/styles.md §9, step 3b): no title,
    /// name or shapes; the tools are the library's, by name; a profile's side is by the
    /// selection or on the line.
    pub style: bool,
}

/// The selected operation's fields, in sections.
/// `stock`: for an end depth that cuts through the stock.
pub(in crate::app) fn op_form(
    ui: &mut egui::Ui,
    index: usize,
    op: &mut Operation,
    side_chosen: &mut bool,
    contours: &[Contour],
    choice: ToolChoice<'_>,
    stock: StockDepths,
) -> OpFormRequest {
    let units = crate::units::display(ui.ctx());
    let len_unit = chipload_pipeline::units::length_unit(units);
    let mut request = OpFormRequest::default();
    let mut use_selection = false;
    let tool_diameter = choice
        .tools
        .iter()
        .find(|t| t.id == op.tool())
        .map_or(0.0, |t| t.diameter);
    // For the chip load next to the feed.
    let op_tool: Option<Tool> = choice.tools.iter().find(|t| t.id == op.tool()).cloned();
    let op_tool = op_tool.as_ref();
    let summary = geometry_summary(&*op, contours, ui);
    let kind = match &*op {
        Operation::Profile(_) => t!(ui, "forms-kind-profile"),
        Operation::Pocket(_) => t!(ui, "forms-kind-pocket"),
        Operation::Vcarve(_) => t!(ui, "forms-kind-vcarve"),
        Operation::Path(_) => t!(ui, "forms-kind-path"),
        Operation::Chamfer(_) => t!(ui, "forms-kind-chamfer"),
        Operation::Flute(_) => t!(ui, "forms-kind-flute"),
        Operation::Pattern(_) => t!(ui, "forms-kind-pattern"),
        Operation::Prism(_) => t!(ui, "forms-kind-prism"),
        Operation::Moulding(_) => t!(ui, "forms-kind-moulding"),
        Operation::Holes(_) => t!(ui, "forms-kind-holes"),
        Operation::Surface(_) => t!(ui, "forms-kind-surface"),
        Operation::Relief(_) => t!(ui, "forms-kind-relief"),
    };
    let surface = !op.selects_shapes();
    let color = super::browser::kind_color(&*op, ui.visuals().dark_mode);
    if !choice.style {
        super::browser::title_line(
            ui,
            super::browser::op_icon(&*op),
            Some(color),
            &format!(
                "{kind} · {}",
                t!(ui, "forms-operation-number", number = index + 1)
            ),
        );
    }
    // Its cards have its colour down their edge (docs/specs/look-and-feel.md §5).
    set_card_stripe(ui, color);
    card(ui, |ui| {
        labels_weak(ui);
        Grid::new(("op-form", index))
            .num_columns(2)
            // Past the sections' indent, so the values line up with theirs.
            .min_col_width(label_width(ui) + ui.spacing().indent)
            .show(ui, |ui| {
                if choice.style {
                    // A style's name is its own, its shapes the selection it is used on.
                    tool_row(ui, index, &mut *op, &choice, &mut request);
                    if let Operation::Profile(p) = &mut *op {
                        style_side_row(ui, index, p);
                    }
                    return;
                }
                let name = op.name_mut();
                ui.label(t!(ui, "forms-name"));
                ui.add(TextEdit::singleline(name).desired_width(field_width(ui, 200.0)));
                ui.end_row();
                ui.label(t!(ui, "forms-geometry"));
                if surface {
                    // docs/specs/surfacing.md §2.
                    ui.label(t!(ui, "forms-whole-stock"))
                        .on_hover_text(t!(ui, "forms-whole-stock-hover"));
                    ui.end_row();
                    tool_row(ui, index, &mut *op, &choice, &mut request);
                    return;
                }
                let geometry = wrapped_cell(ui, |ui| {
                    ui.label(&summary);
                    let label = match choice.selected {
                        0 => t!(ui, "forms-use-selection"),
                        n => t!(ui, "forms-use-selection-count", count = n),
                    };
                    if ui
                        .add_enabled(choice.selected > 0, egui::Button::new(label))
                        .on_hover_text(format!(
                            "{}\n{}",
                            t!(ui, "forms-use-selection-hover"),
                            choice.selection_text
                        ))
                        .on_disabled_hover_text(t!(ui, "forms-use-selection-disabled"))
                        .clicked()
                    {
                        use_selection = true;
                    }
                });
                flag(geometry.response, ui, "select");
                ui.end_row();
                tool_row(ui, index, &mut *op, &choice, &mut request);
                if let Operation::Chamfer(c) = &mut *op {
                    chamfer_side_row(ui, index, c);
                }
                if let Operation::Profile(p) = &mut *op {
                    ui.label(t!(ui, "forms-side"));
                    let side_name = |ui: &egui::Ui, side: Side| match side {
                        Side::Inside => t!(ui, "forms-side-inside"),
                        Side::Outside => t!(ui, "forms-side-outside"),
                        Side::On => t!(ui, "forms-side-on"),
                    };
                    let text = if *side_chosen {
                        side_name(ui, p.side)
                    } else {
                        t!(ui, "forms-side-choose")
                    };
                    let side = ComboBox::from_id_salt(("side", index))
                        .selected_text(text)
                        .show_ui(ui, |ui| {
                            for side in [Side::Outside, Side::Inside, Side::On] {
                                let label = side_name(ui, side);
                                if ui
                                    .selectable_label(*side_chosen && p.side == side, label)
                                    .clicked()
                                {
                                    p.side = side;
                                    *side_chosen = true;
                                }
                            }
                        });
                    flag(side.response, ui, "side");
                    ui.end_row();
                }
            });
    });
    match &mut *op {
        Operation::Profile(p) => {
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
        Operation::Pocket(p) => {
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
                rest_row(ui, index, &mut p.rest, choice.tools, op_tool);
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
        Operation::Vcarve(v) => {
            let start = format!("{} {len_unit}", len_in(units, v.start_depth));
            let end = format!("{} {len_unit}", len_in(units, v.end_depth));
            let depths = chipload_pipeline::vcarve_pass_depths(v);
            let cut = match depths.len() {
                0 | 1 => t!(ui, "forms-vcarve-cut-one-pass", start = start, end = end),
                count => t!(
                    ui,
                    "forms-vcarve-cut-passes",
                    start = start,
                    end = end,
                    count = count
                ),
            };
            folded_section(ui, "op-cut", &t!(ui, "forms-section-cut"), &cut, |ui| {
                ui.label(t!(ui, "forms-start-depth"))
                    .on_hover_text(t!(ui, "forms-vcarve-start-depth-hover"));
                flag(ui.add(mm(&mut v.start_depth)), ui, "start_depth");
                ui.end_row();
                ui.label(t!(ui, "forms-end-depth"))
                    .on_hover_text(t!(ui, "forms-vcarve-end-depth-hover"));
                flag(ui.add(mm(&mut v.end_depth)), ui, "end_depth");
                ui.end_row();
                vcarve_pass_rows(ui, index, v, &depths, units);
                // Corners where the outline bends less than this get no branch: the bit
                // doesn't rise into every kink (docs/specs/vcarve-smooth.md §3).
                ui.label(t!(ui, "forms-ignore-kinks"))
                    .on_hover_text(t!(ui, "forms-ignore-kinks-hover"));
                flag(
                    ui.add(
                        crate::units::number(ui, &mut v.ignore_kinks_deg)
                            .range(0.0..=chipload_job::MAX_IGNORE_KINKS_DEG)
                            .speed(1.0)
                            .max_decimals(1)
                            .suffix("°"),
                    ),
                    ui,
                    "ignore_kinks_deg",
                );
                ui.end_row();
            });
            // An inlay's plug (docs/specs/vcarve-inlay.md §4): its start depth and its glue
            // gap, the end depth less the start depth; setting the gap moves the end depth.
            if let Some(plug) = v.plug.as_mut() {
                let mut gap = v.end_depth - plug.start_depth;
                let summary = t!(
                    ui,
                    "forms-plug-summary",
                    start = format!("{} {len_unit}", len_in(units, plug.start_depth)),
                    gap = format!("{} {len_unit}", len_in(units, gap))
                );
                let title = t!(ui, "forms-section-plug");
                folded_section(ui, "op-plug", &title, &summary, |ui| {
                    ui.label(t!(ui, "forms-plug-start"))
                        .on_hover_text(t!(ui, "forms-plug-start-hover"));
                    flag(ui.add(mm(&mut plug.start_depth)), ui, "plug.start_depth");
                    ui.end_row();
                    ui.label(t!(ui, "forms-plug-gap"))
                        .on_hover_text(t!(ui, "forms-plug-gap-hover"));
                    if ui.add(mm(&mut gap)).changed() {
                        v.end_depth = plug.start_depth + gap;
                    }
                    ui.end_row();
                });
            }
            feeds_section(ui, &mut v.feed, &mut v.plunge_feed, op_tool);
        }
        Operation::Path(p) => {
            let cut = t!(
                ui,
                "forms-path-cut-summary",
                start = len_in(units, p.start_depth),
                end = len_in(units, p.end_depth),
                unit = len_unit,
                step = len_in(units, p.step_down)
            );
            folded_section(ui, "op-cut", &t!(ui, "forms-section-cut"), &cut, |ui| {
                ui.label(t!(ui, "forms-start-depth"))
                    .on_hover_text(t!(ui, "forms-start-depth-hover"));
                flag(ui.add(mm(&mut p.start_depth)), ui, "start_depth");
                ui.end_row();
                let hover = t!(ui, "forms-end-depth-hover");
                end_depth_rows(ui, &mut p.end_depth, &mut p.through, stock, hover);
                groove_width_row(ui, op_tool, &mut p.end_depth, p.through.is_some());
                ui.label(t!(ui, "forms-step-down"));
                flag(ui.add(mm(&mut p.step_down)), ui, "step_down");
                ui.end_row();
            });
            feeds_section(ui, &mut p.feed, &mut p.plunge_feed, op_tool);
            let entry_text = entry_summary(ui.lang(), &p.entry, None, units);
            let title = t!(ui, "forms-section-entry");
            folded_section(ui, "op-entry", &title, &entry_text, |ui| {
                entry_rows(ui, index, &mut p.entry, None);
            });
        }
        Operation::Chamfer(c) => chamfer_sections(ui, index, c, op_tool),
        Operation::Flute(f) => flute_sections(ui, f, op_tool),
        Operation::Pattern(f) => pattern_sections(ui, f, op_tool),
        Operation::Prism(f) => prism_sections(ui, index, f, op_tool),
        Operation::Moulding(m) => moulding_sections(ui, index, m, op_tool, contours),
        Operation::Holes(h) => holes_sections(ui, index, h, op_tool, stock),
        Operation::Surface(f) => {
            let cut = t!(
                ui,
                "forms-surface-cut-summary",
                start = len_in(units, f.start_depth),
                end = len_in(units, f.end_depth),
                unit = len_unit,
                step = len_in(units, f.step_down),
                stepover = format!("{:.2}", f.stepover)
            );
            folded_section(ui, "op-cut", &t!(ui, "forms-section-cut"), &cut, |ui| {
                ui.label(t!(ui, "forms-start-depth"))
                    .on_hover_text(t!(ui, "forms-start-depth-hover"));
                flag(ui.add(mm(&mut f.start_depth)), ui, "start_depth");
                ui.end_row();
                ui.label(t!(ui, "forms-end-depth"))
                    .on_hover_text(t!(ui, "forms-surface-end-depth-hover"));
                flag(ui.add(mm(&mut f.end_depth)), ui, "end_depth");
                ui.end_row();
                ui.label(t!(ui, "forms-step-down"));
                flag(ui.add(mm(&mut f.step_down)), ui, "step_down");
                ui.end_row();
                ui.label(t!(ui, "forms-stepover"))
                    .on_hover_text(t!(ui, "forms-surface-stepover-hover"));
                let (lo, hi) = chipload_job::SURFACE_STEPOVER;
                let stepover = ui.add(
                    crate::units::number(ui, &mut f.stepover)
                        .speed(0.01)
                        .range(lo..=hi)
                        .suffix(" × D"),
                );
                flag(stepover, ui, "stepover");
                ui.end_row();
            });
            feeds_section(ui, &mut f.feed, &mut f.plunge_feed, op_tool);
        }
        // docs/specs/stl-relief.md §5, §9.
        Operation::Relief(r) => {
            // The plan's deep-relief warning, seen before generating (§16).
            if r.roughing.is_none()
                && r.end_depth > tool_diameter + chipload_core::tolerances::DEPTH_TOL_MM
            {
                ui.label(
                    egui::RichText::new(t!(
                        ui,
                        "forms-relief-deep",
                        depth = format!("{} {len_unit}", len_in(units, r.end_depth))
                    ))
                    .color(ui.visuals().warn_fg_color),
                );
            }
            let cut = t!(
                ui,
                "forms-relief-cut-summary",
                start = len_in(units, r.start_depth),
                end = len_in(units, r.end_depth),
                unit = len_unit,
                stepover = format!("{:.2}", r.stepover),
                angle = format!("{:.0}", r.angle.to_degrees())
            );
            folded_section(ui, "op-cut", &t!(ui, "forms-section-cut"), &cut, |ui| {
                ui.label(t!(ui, "forms-start-depth"))
                    .on_hover_text(t!(ui, "forms-start-depth-hover"));
                flag(ui.add(mm(&mut r.start_depth)), ui, "start_depth");
                ui.end_row();
                // The model's depth: changed here, it is the model's that changes, once the
                // edit is finished (the user, 2026-11-14), for every relief of the model.
                ui.label(t!(ui, "forms-relief-depth"))
                    .on_hover_text(t!(ui, "forms-relief-depth-hover"));
                let stored = r.end_depth - r.start_depth;
                let key = egui::Id::new(("relief-depth", index));
                let mut depth = ui
                    .data(|d| d.get_temp::<(f64, f64)>(key))
                    .filter(|(from, _)| *from == stored)
                    .map_or(stored, |(_, draft)| draft);
                let response = ui.add(mm(&mut depth).range(0.01..=1_000.0).speed(0.1));
                if response.drag_stopped() || response.lost_focus() {
                    ui.data_mut(|d| d.remove::<(f64, f64)>(key));
                    if (depth - stored).abs() > chipload_core::tolerances::DEPTH_TOL_MM {
                        request.relief_depth = Some((r.model, depth));
                    }
                } else {
                    ui.data_mut(|d| d.insert_temp(key, (stored, depth)));
                }
                flag(response, ui, "end_depth");
                ui.end_row();
                ui.label(t!(ui, "forms-stepover"))
                    .on_hover_text(t!(ui, "forms-relief-stepover-hover"));
                let stepover = ui.add(
                    crate::units::number(ui, &mut r.stepover)
                        .speed(0.01)
                        .range(0.01..=chipload_job::RELIEF_MAX_STEPOVER)
                        .suffix(" × D"),
                );
                flag(stepover, ui, "stepover");
                ui.end_row();
                // A ball nose's ridges between lines (docs/specs/stl-relief.md §15).
                if let Some(ridge) =
                    op_tool.and_then(|t| chipload_pipeline::new_ops::relief_ridge(t, r.stepover))
                {
                    ui.label(t!(ui, "forms-relief-ridge"))
                        .on_hover_text(t!(ui, "forms-relief-ridge-hover"));
                    ui.weak(format!("{} {len_unit}", len_in(units, ridge)));
                    ui.end_row();
                }
                ui.label(t!(ui, "forms-relief-angle"))
                    .on_hover_text(t!(ui, "forms-relief-angle-hover"));
                let mut degrees = r.angle.to_degrees();
                if ui
                    .add(
                        crate::units::number(ui, &mut degrees)
                            .speed(1.0)
                            .range(-180.0..=180.0)
                            .suffix("°"),
                    )
                    .changed()
                {
                    r.angle = degrees.to_radians();
                }
                ui.end_row();
                ui.label("");
                ui.checkbox(&mut r.zigzag, t!(ui, "forms-relief-zigzag"))
                    .on_hover_text(t!(ui, "forms-relief-zigzag-hover"));
                ui.end_row();
            });
            rim_section(ui, &mut r.rim);
            feeds_section(ui, &mut r.feed, &mut r.plunge_feed, op_tool);
        }
    }
    request.use_selection = use_selection;
    request
}

/// A profile style's side: by the selection (outlines outside, holes inside) or on the
/// line (docs/specs/styles.md §2.1). The operation keeps it as its side: on, or outside.
fn style_side_row(ui: &mut egui::Ui, index: usize, p: &mut chipload_job::ProfileOp) {
    let on = p.side == Side::On;
    let name = |ui: &egui::Ui, on: bool| {
        if on {
            t!(ui, "forms-side-on")
        } else {
            t!(ui, "styles-side-selection")
        }
    };
    ui.label(t!(ui, "forms-side"));
    let combo = ComboBox::from_id_salt(("style-side", index))
        .selected_text(name(ui, on))
        .show_ui(ui, |ui| {
            if ui.selectable_label(!on, name(ui, false)).clicked() && on {
                p.side = Side::Outside;
            }
            if ui.selectable_label(on, name(ui, true)).clicked() {
                p.side = Side::On;
            }
        });
    flag(
        combo.response.on_hover_text(t!(ui, "styles-side-hover")),
        ui,
        "side",
    );
    ui.end_row();
}

/// The operation's tool: one of the project's, or one copied in from the library
/// (docs/specs/tool-per-operation.md §8). Changing it keeps the feeds; when the tool's
/// library cutting values differ from the operation's, a button applies them.
pub(super) fn tool_row(
    ui: &mut egui::Ui,
    index: usize,
    op: &mut Operation,
    choice: &ToolChoice<'_>,
    request: &mut OpFormRequest,
) {
    let units = crate::units::display(ui.ctx());
    let len_unit = chipload_pipeline::units::length_unit(units);
    let feed_unit = chipload_pipeline::units::feed_unit(units);
    let current = op.tool();
    // A style's tools are the library's, known by name (their numbers are only places).
    let label = |ui: &egui::Ui, tool: &Tool| match (&tool.name, choice.style) {
        (Some(name), true) => name.clone(),
        _ => tool_label(ui, tool, units),
    };
    let missing = if choice.style {
        t!(ui, "styles-tool-not-in-library")
    } else {
        t!(ui, "forms-tool-not-in-project", tool = current)
    };
    let selected = choice
        .tools
        .iter()
        .find(|t| t.id == current)
        .map_or(missing, |t| label(ui, t));
    ui.label(t!(ui, "forms-tool"));
    let combo = ComboBox::from_id_salt(("op-tool", index))
        .selected_text(selected)
        .width(field_width(ui, 240.0))
        .truncate()
        .show_ui(ui, |ui| {
            for tool in choice.tools {
                if ui
                    .selectable_label(tool.id == current, label(ui, tool))
                    .clicked()
                {
                    if tool.id != current {
                        request.tool_changed = true;
                    }
                    op.set_tool(tool.id);
                    crate::editor::fit_hole_method(op, choice.tools);
                    op.fit_helixes(choice.tools);
                }
            }
            if let Some(names) = &choice.library
                && !names.is_empty()
            {
                ui.separator();
                ui.weak(t!(ui, "forms-from-library"));
                for name in names {
                    if ui.selectable_label(false, format!("+ {name}")).clicked() {
                        request.tool_from_library = Some(name.clone());
                    }
                }
            }
        });
    flag(combo.response, ui, "tool");
    ui.end_row();
    let mut material_values = None;
    if let Some(materials) = &choice.materials
        && (!materials.is_empty() || op.material().is_some())
    {
        let runs = choice
            .tools
            .iter()
            .find(|t| t.id == op.tool())
            .and_then(|t| t.rpm);
        material_values = material_rows(ui, index, op, materials, runs, request);
    }
    // The tool's own values, when the operation's material has none for it.
    if let Some(cutting) = choice.cutting
        && material_values.is_none()
        && !op.has_cutting(&cutting)
    {
        ui.label("");
        if ui
            .button(t!(ui, "forms-use-cutting-values"))
            .on_hover_text(t!(
                ui,
                "forms-use-cutting-values-hover",
                feed = feed_in(units, cutting.feed),
                plunge = feed_in(units, cutting.plunge_feed),
                feed_unit = feed_unit,
                step_down = format!("{} {len_unit}", len_in(units, cutting.step_down)),
                stepover = cutting.stepover.to_string()
            ))
            .clicked()
        {
            op.apply_cutting(&cutting);
        }
        ui.end_row();
    }
}

/// The operation's material: a list of the library's (docs/specs/materials.md §2), then
/// where its feeds stand against it: from it, changed (with *Use* and *Save*), or not in it
/// yet (with *Save*). The buttons have a row of their own, so a narrow panel doesn't wrap
/// them over the next row. The material's values for the tool, if it has some.
pub(super) fn material_rows(
    ui: &mut egui::Ui,
    index: usize,
    op: &mut Operation,
    materials: &[(MaterialRef, Option<Cutting>)],
    tool_rpm: Option<f64>,
    request: &mut OpFormRequest,
) -> Option<Cutting> {
    let current = op.material().cloned();
    let known = current.as_ref().and_then(|c| {
        materials
            .iter()
            .find(|(m, _)| m.library_uid == c.library_uid)
    });
    let shown = match (&current, known) {
        (None, _) => t!(ui, "forms-material-none"),
        (Some(_), Some((m, _))) => m.name.clone(),
        (Some(m), None) => t!(ui, "forms-material-not-in-library", name = m.name.as_str()),
    };
    ui.label(t!(ui, "forms-material"))
        .on_hover_text(t!(ui, "forms-op-material-hover"));
    ComboBox::from_id_salt(("op-material", index))
        .selected_text(shown)
        .width(field_width(ui, 240.0))
        .truncate()
        .show_ui(ui, |ui| {
            if ui
                .selectable_label(current.is_none(), t!(ui, "forms-material-none"))
                .clicked()
            {
                op.set_material(None);
            }
            for (m, values) in materials {
                let on = current
                    .as_ref()
                    .is_some_and(|c| c.library_uid == m.library_uid);
                let label = match values {
                    Some(_) => m.name.clone(),
                    None => t!(ui, "forms-material-no-values", name = m.name.as_str()),
                };
                if ui.selectable_label(on, label).clicked() {
                    op.set_material(Some(m.clone()));
                    if let Some(c) = values {
                        op.apply_cutting(c);
                    }
                }
            }
        });
    ui.end_row();
    let (m, values) = known?;
    let name = m.name.as_str();
    ui.label("");
    let (use_values, save) = match values {
        Some(c) if op.has_cutting(c) => {
            ui.weak(t!(ui, "forms-material-from", name = name))
                .on_hover_text(t!(ui, "forms-material-from-hover"));
            (None, false)
        }
        Some(c) => {
            ui.label(t!(ui, "forms-material-changed", name = name));
            (Some(*c), true)
        }
        None => {
            ui.weak(t!(ui, "forms-material-not-in", name = name));
            (None, true)
        }
    };
    ui.end_row();
    // Its values are for another speed than the tool runs at (docs/specs/cutting-data.md
    // §6): the chip load shown is at the tool's.
    if let (Some(row), Some(runs)) = (values.and_then(|c| c.rpm), tool_rpm)
        && row != runs
    {
        ui.label("");
        wrapped_cell(ui, |ui| {
            ui.weak(t!(
                ui,
                "forms-material-speed-differs",
                name = name,
                rpm = chipload_i18n::message::trimmed(row, 0),
                runs = chipload_i18n::message::trimmed(runs, 0)
            ))
        });
        ui.end_row();
    }
    // A row each: a grid cell that wraps overlaps the row under it.
    if let Some(c) = use_values {
        ui.label("");
        if ui
            .button(t!(ui, "forms-material-use", name = name))
            .on_hover_text(t!(ui, "forms-material-use-hover"))
            .clicked()
        {
            op.apply_cutting(&c);
        }
        ui.end_row();
    }
    if save {
        ui.label("");
        if ui
            .button(t!(ui, "forms-material-save", name = name))
            .on_hover_text(t!(ui, "forms-material-save-hover"))
            .clicked()
        {
            request.save_to_material = true;
        }
        ui.end_row();
    }
    *values
}

/// The feeds, and the chip load next to the feed (docs/specs/small-items.md §5) when the
/// operation's tool is known.
pub(super) fn feeds_section(
    ui: &mut egui::Ui,
    cut: &mut f64,
    plunge: &mut f64,
    tool: Option<&Tool>,
) {
    feeds_section_with(ui, cut, plunge, None, tool);
}

/// A pocket's full-width rings' step down (docs/specs/cutting-data.md §4): shown as the step
/// down while it has none of its own, set once edited.
pub(super) fn slot_step_row(ui: &mut egui::Ui, slot: &mut Option<f64>, step_down: f64) {
    ui.label(t!(ui, "forms-slot-step-down"))
        .on_hover_text(t!(ui, "forms-slot-step-down-hover"));
    let mut value = slot.unwrap_or(step_down);
    let field = ui.add(mm(&mut value));
    if field.changed() {
        *slot = Some(value);
    }
    flag(field, ui, "slot_step_down");
    ui.end_row();
}

/// The feeds, with a pocket's full-width rings' feed (`slot`, shown as the feed while it has
/// none of its own) under the feed.
pub(super) fn feeds_section_with(
    ui: &mut egui::Ui,
    cut: &mut f64,
    plunge: &mut f64,
    slot: Option<&mut Option<f64>>,
    tool: Option<&Tool>,
) {
    let units = crate::units::display(ui.ctx());
    let feed_unit = chipload_pipeline::units::feed_unit(units);
    let text = format!(
        "{} / {} {feed_unit}",
        feed_in(units, *cut),
        feed_in(units, *plunge)
    );
    folded_section(
        ui,
        "op-feeds",
        &t!(ui, "forms-section-feeds"),
        &text,
        |ui| {
            ui.label(t!(ui, "forms-feed"));
            flag(ui.add(feed(cut)), ui, "feed");
            ui.end_row();
            if let Some(tool) = tool {
                let load = chipload_pipeline::chip_load::chip_load_of(*cut, tool);
                chip_load_row(ui, load, units);
            }
            if let Some(slot) = slot {
                ui.label(t!(ui, "forms-slot-feed"))
                    .on_hover_text(t!(ui, "forms-slot-feed-hover"));
                let mut value = slot.unwrap_or(*cut);
                let field = ui.add(feed(&mut value));
                if field.changed() {
                    *slot = Some(value);
                }
                flag(field, ui, "slot_feed");
                ui.end_row();
                if let Some(tool) = tool {
                    let load = chipload_pipeline::chip_load::chip_load_of(value, tool);
                    chip_load_row(ui, load, units);
                }
            }
            ui.label(t!(ui, "forms-plunge-feed"));
            flag(ui.add(feed(plunge)), ui, "plunge_feed");
            ui.end_row();
        },
    );
}

/// How the tool goes down into each pass (docs/specs/ramp-entry.md); angles are shown in
/// degrees and kept in radians. `helix`: the tool diameter, for a pocket, which may also go
/// down round a helix (docs/specs/helix-and-leads.md part C).
pub(super) fn entry_rows(ui: &mut egui::Ui, index: usize, entry: &mut Entry, helix: Option<f64>) {
    ui.label(t!(ui, "forms-entry"))
        .on_hover_text(t!(ui, "forms-entry-hover"));
    let ramp = Entry::Ramp {
        ramp_angle: DEFAULT_RAMP_ANGLE_RAD,
    };
    let combo = ComboBox::from_id_salt(("entry", index))
        .selected_text(match entry {
            Entry::Plunge {} => t!(ui, "forms-entry-plunge"),
            Entry::Ramp { .. } => t!(ui, "forms-entry-ramp"),
            Entry::Helix { .. } => t!(ui, "forms-entry-helix"),
        })
        .show_ui(ui, |ui| {
            let is_ramp = matches!(entry, Entry::Ramp { .. });
            let is_helix = matches!(entry, Entry::Helix { .. });
            if ui
                .selectable_label(!is_ramp && !is_helix, t!(ui, "forms-entry-plunge"))
                .clicked()
            {
                *entry = Entry::Plunge {};
            }
            if ui
                .selectable_label(is_ramp, t!(ui, "forms-entry-ramp"))
                .clicked()
                && !is_ramp
            {
                *entry = ramp;
            }
            if let Some(diameter) = helix
                && ui
                    .selectable_label(is_helix, t!(ui, "forms-entry-helix"))
                    .clicked()
                && !is_helix
            {
                *entry = Entry::default_helix(diameter);
            }
        });
    flag(combo.response, ui, "entry");
    ui.end_row();
    if let Entry::Helix { radius, .. } = entry {
        ui.label(format!("  {}", t!(ui, "forms-helix-radius")))
            .on_hover_text(t!(ui, "forms-helix-radius-hover"));
        flag(ui.add(mm(radius)), ui, "entry");
        ui.end_row();
    }
    if let Entry::Ramp { ramp_angle } | Entry::Helix { ramp_angle, .. } = entry {
        ui.label(format!("  {}", t!(ui, "forms-ramp-angle-sub")));
        let mut degrees = ramp_angle.to_degrees();
        if ui
            .add(
                crate::units::number(ui, &mut degrees)
                    .speed(0.1)
                    .range(0.5..=MAX_RAMP_ANGLE_RAD.to_degrees())
                    .suffix("°"),
            )
            .changed()
        {
            *ramp_angle = degrees.to_radians();
        }
        ui.end_row();
    }
}

/// A profile's way down (docs/specs/profile-always-spiral.md): it always spirals down along
/// the contour; only the angle is set, after the step down.
pub(super) fn ramp_angle_row(ui: &mut egui::Ui, ramp_angle: &mut f64) {
    ui.label(t!(ui, "forms-ramp-angle"))
        .on_hover_text(t!(ui, "forms-ramp-angle-hover"));
    let mut value = ramp_angle.to_degrees();
    let response = ui.add(
        crate::units::number(ui, &mut value)
            .speed(0.1)
            .range(0.5..=MAX_RAMP_ANGLE_RAD.to_degrees())
            .suffix("°"),
    );
    if flag(response, ui, "ramp_angle").changed() {
        *ramp_angle = value.to_radians();
    }
    ui.end_row();
}

/// Rest machining (docs/specs/rest-machining.md §6): none, or one of the project's endmills
/// larger than the pocket's tool.
pub(super) fn rest_row(
    ui: &mut egui::Ui,
    index: usize,
    rest: &mut Option<Rest>,
    tools: &[Tool],
    tool: Option<&Tool>,
) {
    let units = crate::units::display(ui.ctx());
    let len_unit = chipload_pipeline::units::length_unit(units);
    ui.label(t!(ui, "forms-rest-after"))
        .on_hover_text(t!(ui, "forms-rest-after-hover"));
    let lang = ui.lang();
    let name = |t: &Tool| {
        let diameter = format!("{} {len_unit}", len_in(units, t.diameter));
        format!(
            "T{} · {}",
            t.id,
            t!(lang, "forms-rest-endmill", diameter = diameter)
        )
    };
    let selected = match rest {
        None => t!(ui, "forms-rest-none"),
        Some(r) => tools.iter().find(|t| t.id == r.after_tool).map_or(
            t!(ui, "forms-tool-not-in-project", tool = r.after_tool),
            name,
        ),
    };
    let response = ComboBox::from_id_salt(("rest", index))
        .selected_text(selected)
        .show_ui(ui, |ui| {
            if ui
                .selectable_label(rest.is_none(), t!(ui, "forms-rest-none"))
                .clicked()
            {
                *rest = None;
            }
            for t in rest_choices(tools, tool) {
                let chosen = rest.is_some_and(|r| r.after_tool == t.id);
                if ui.selectable_label(chosen, name(t)).clicked() {
                    *rest = Some(Rest { after_tool: t.id });
                }
            }
        })
        .response;
    flag(response, ui, "rest");
    ui.end_row();
}

/// The tools a pocket cutting with `small` can rest machine after: the project's endmills
/// larger than it (the project's rule, docs/specs/rest-machining.md §5).
pub(super) fn rest_choices<'a>(
    tools: &'a [Tool],
    small: Option<&'a Tool>,
) -> impl Iterator<Item = &'a Tool> {
    tools
        .iter()
        .filter(move |t| small.is_some_and(|small| rest_refusal(small, t).is_none()))
}

/// Square inside corners: dogbones or T-bones (docs/specs/dogbones.md §7), and how many
/// corners have their own choice, with *Reset* (docs/specs/dogbone-per-corner.md §6).
/// `allowed`: false for a profile on the line, which has no inside corners to clear.
pub(super) fn corner_rows(
    ui: &mut egui::Ui,
    index: usize,
    corners: &mut Option<Corners>,
    chosen: &mut Vec<chipload_job::CornerChoice>,
    allowed: bool,
) {
    ui.label(t!(ui, "forms-corners"))
        .on_hover_text(t!(ui, "forms-corners-hover"));
    let angle = corners.map_or(DEFAULT_CORNER_ANGLE_RAD, |c| c.max_angle());
    // Which kind of corners: none, dogbones or T-bones.
    let kind = |c: Option<Corners>| match c {
        None => 0,
        Some(Corners::Dogbone { .. }) => 1,
        Some(Corners::Tbone { .. }) => 2,
    };
    let choices = [
        (None, t!(ui, "forms-corners-round")),
        (
            Some(Corners::Dogbone { max_angle: angle }),
            t!(ui, "forms-corners-dogbones"),
        ),
        (
            Some(Corners::Tbone { max_angle: angle }),
            t!(ui, "forms-corners-tbones"),
        ),
    ];
    let current = kind(*corners);
    let name = choices
        .iter()
        .find(|(choice, _)| kind(*choice) == current)
        .map_or(String::new(), |(_, label)| label.clone());
    let combo = ui.add_enabled_ui(allowed || corners.is_some(), |ui| {
        ComboBox::from_id_salt(("corners", index))
            .selected_text(name)
            .show_ui(ui, |ui| {
                for (choice, label) in choices {
                    if ui
                        .selectable_label(kind(choice) == current, label)
                        .clicked()
                    {
                        *corners = choice;
                    }
                }
            })
    });
    let response = combo
        .response
        .on_disabled_hover_text(t!(ui, "forms-corners-disabled"));
    flag(response, ui, "corners");
    ui.end_row();
    if let Some(Corners::Dogbone { max_angle } | Corners::Tbone { max_angle }) = corners {
        ui.label(format!("  {}", t!(ui, "forms-corners-up-to")))
            .on_hover_text(t!(ui, "forms-corners-up-to-hover"));
        let mut degrees = max_angle.to_degrees();
        let response = ui.add(
            crate::units::number(ui, &mut degrees)
                .speed(1.0)
                .range(1.0..=179.0)
                .suffix("°"),
        );
        if flag(response, ui, "corners.max_angle").changed() {
            *max_angle = degrees.to_radians();
        }
        ui.end_row();
    }
    if !chosen.is_empty() {
        ui.label("");
        ui.horizontal(|ui| {
            let label = ui
                .label(t!(ui, "forms-corners-chosen", count = chosen.len()))
                .on_hover_text(t!(ui, "forms-corners-chosen-hover"));
            flag(label, ui, "corner_choices");
            if ui
                .button(t!(ui, "forms-corners-reset"))
                .on_hover_text(t!(ui, "forms-corners-reset-hover"))
                .clicked()
            {
                chosen.clear();
            }
        });
        ui.end_row();
    }
}

/// A Holes operation's cut, method and feeds (docs/specs/holes.md §7): a drill shows its
/// pecks and plunge feed, an endmill the helix.
pub(super) fn holes_sections(
    ui: &mut egui::Ui,
    index: usize,
    h: &mut HolesOp,
    tool: Option<&Tool>,
    stock: StockDepths,
) {
    let units = crate::units::display(ui.ctx());
    let len_unit = chipload_pipeline::units::length_unit(units);
    let feed_unit = chipload_pipeline::units::feed_unit(units);
    let method = match h.method {
        HoleMethod::Drill { peck: Some(peck) } => t!(
            ui,
            "forms-holes-drilled-pecks",
            peck = format!("{} {len_unit}", len_in(units, peck))
        ),
        HoleMethod::Drill { peck: None } => t!(ui, "forms-holes-drilled-once"),
        HoleMethod::Bore {
            step_down,
            direction,
            ..
        } => t!(
            ui,
            "forms-holes-bore",
            step = format!("{} {len_unit}", len_in(units, step_down)),
            direction = direction_name(ui, direction)
        ),
    };
    let cut = t!(
        ui,
        "forms-holes-cut-summary",
        start = len_in(units, h.start_depth),
        end = len_in(units, h.end_depth),
        unit = len_unit,
        method = method
    );
    folded_section(ui, "op-cut", &t!(ui, "forms-section-cut"), &cut, |ui| {
        ui.label(t!(ui, "forms-start-depth"))
            .on_hover_text(t!(ui, "forms-start-depth-hover"));
        flag(ui.add(mm(&mut h.start_depth)), ui, "start_depth");
        ui.end_row();
        let drill = matches!(h.method, HoleMethod::Drill { .. });
        let hover = if drill {
            t!(ui, "forms-holes-drill-end-depth-hover")
        } else {
            t!(ui, "forms-end-depth-hover")
        };
        end_depth_rows(ui, &mut h.end_depth, &mut h.through, stock, hover);
        match &mut h.method {
            HoleMethod::Drill { peck } => {
                ui.label(t!(ui, "forms-pecks"))
                    .on_hover_text(t!(ui, "forms-pecks-hover"));
                let mut on = peck.is_some();
                let check = ui.checkbox(&mut on, t!(ui, "forms-in-pecks"));
                if flag(check, ui, "method.peck").changed() {
                    // Pecks of one drill diameter, a common start (the review of 2026-11-02).
                    *peck = on.then_some(tool.map_or(DEFAULT_STEP_DOWN, |t| t.diameter));
                }
                ui.end_row();
                if let Some(peck) = peck {
                    ui.label(format!("  {}", t!(ui, "forms-peck")));
                    flag(ui.add(mm(peck)), ui, "method.peck");
                    ui.end_row();
                }
            }
            HoleMethod::Bore {
                step_down,
                ramp_angle,
                direction,
            } => {
                ui.label(t!(ui, "forms-direction"));
                direction_combo(ui, ("direction", index), direction);
                ui.end_row();
                ui.label(t!(ui, "forms-step-per-turn"))
                    .on_hover_text(t!(ui, "forms-step-per-turn-hover"));
                flag(ui.add(mm(step_down)), ui, "method.step_down");
                ui.end_row();
                ui.label(t!(ui, "forms-ramp-angle"))
                    .on_hover_text(t!(ui, "forms-bore-ramp-angle-hover"));
                let mut degrees = ramp_angle.to_degrees();
                let response = ui.add(
                    crate::units::number(ui, &mut degrees)
                        .speed(0.1)
                        .range(0.5..=MAX_RAMP_ANGLE_RAD.to_degrees())
                        .suffix("°"),
                );
                if flag(response, ui, "method.ramp_angle").changed() {
                    *ramp_angle = degrees.to_radians();
                }
                ui.end_row();
            }
        }
    });
    if matches!(h.method, HoleMethod::Drill { .. }) {
        // A drill only goes down: no cutting feed to show.
        let text = t!(
            ui,
            "forms-drill-feeds-summary",
            feed = feed_in(units, h.plunge_feed),
            unit = feed_unit
        );
        folded_section(
            ui,
            "op-feeds",
            &t!(ui, "forms-section-feeds"),
            &text,
            |ui| {
                ui.label(t!(ui, "forms-drill-feed"));
                flag(ui.add(feed(&mut h.plunge_feed)), ui, "plunge_feed");
                ui.end_row();
                if let Some(tool) = tool {
                    let load = chipload_pipeline::chip_load::chip_load_of(h.plunge_feed, tool);
                    chip_load_row(ui, load, units);
                }
            },
        );
    } else {
        feeds_section(ui, &mut h.feed, &mut h.plunge_feed, tool);
    }
}

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

/// A chamfer's waste side (docs/specs/chamfer.md §2): around the shapes or within them.
fn chamfer_side_row(ui: &mut egui::Ui, index: usize, c: &mut chipload_job::ChamferOp) {
    use chipload_job::EdgeSide;
    let name = |ui: &egui::Ui, side: EdgeSide| match side {
        EdgeSide::Outside => t!(ui, "forms-side-outside"),
        EdgeSide::Inside => t!(ui, "forms-side-inside"),
    };
    ui.label(t!(ui, "forms-side"))
        .on_hover_text(t!(ui, "forms-chamfer-side-hover"));
    let combo = ComboBox::from_id_salt(("chamfer-side", index))
        .selected_text(name(ui, c.side))
        .show_ui(ui, |ui| {
            for side in [EdgeSide::Outside, EdgeSide::Inside] {
                let label = name(ui, side);
                if ui.selectable_label(c.side == side, label).clicked() {
                    c.side = side;
                }
            }
        });
    flag(combo.response, ui, "side");
    ui.end_row();
}

/// A chamfer's sections (docs/specs/chamfer.md §6): its width, with the depth it makes with
/// the V-bit beneath, start depth, passes and direction; feeds; entry.
fn chamfer_sections(
    ui: &mut egui::Ui,
    index: usize,
    c: &mut chipload_job::ChamferOp,
    tool: Option<&Tool>,
) {
    let units = crate::units::display(ui.ctx());
    let len_unit = chipload_pipeline::units::length_unit(units);
    let with_unit = |mm: f64| format!("{} {len_unit}", len_in(units, mm));
    let depth = tool.and_then(|t| c.depth_with(t));
    let summary = t!(
        ui,
        "forms-chamfer-cut-summary",
        width = with_unit(c.width),
        depth = depth.map_or_else(|| "?".to_string(), with_unit),
        direction = direction_name(ui, c.direction)
    );
    folded_section(ui, "op-cut", &t!(ui, "forms-section-cut"), &summary, |ui| {
        ui.label(t!(ui, "forms-chamfer-width"))
            .on_hover_text(t!(ui, "forms-chamfer-width-hover"));
        flag(ui.add(mm(&mut c.width)), ui, "width");
        ui.end_row();
        ui.label(t!(ui, "forms-chamfer-depth"));
        match (depth, tool.and_then(|t| t.angle)) {
            (Some(depth), Some(angle)) => ui.label(t!(
                ui,
                "forms-chamfer-depth-value",
                depth = with_unit(depth),
                angle = format!("{:.0}", angle.to_degrees())
            )),
            _ => ui.label(t!(ui, "forms-chamfer-needs-vbit")),
        };
        ui.end_row();
        ui.label(t!(ui, "forms-start-depth"))
            .on_hover_text(t!(ui, "forms-chamfer-start-hover"));
        flag(ui.add(mm(&mut c.start_depth)), ui, "start_depth");
        ui.end_row();
        ui.label(t!(ui, "forms-step-down"))
            .on_hover_text(t!(ui, "forms-chamfer-passes-hover"));
        let mut passes = c.step_down.is_some();
        let tick = ui.checkbox(&mut passes, t!(ui, "forms-chamfer-passes"));
        if flag(tick, ui, "step_down").changed() {
            c.step_down = passes.then(|| depth.map_or(c.width, |d| d / 2.0).max(0.1));
        }
        ui.end_row();
        if let Some(step) = &mut c.step_down {
            ui.label(format!("  {}", t!(ui, "forms-step-down")));
            flag(ui.add(mm(step)), ui, "step_down");
            ui.end_row();
        }
        ui.label(t!(ui, "forms-direction"));
        direction_combo(ui, ("direction", index), &mut c.direction);
        ui.end_row();
    });
    feeds_section(ui, &mut c.feed, &mut c.plunge_feed, tool);
    let entry_text = entry_summary(ui.lang(), &c.entry, None, units);
    let title = t!(ui, "forms-section-entry");
    folded_section(ui, "op-entry", &title, &entry_text, |ui| {
        entry_rows(ui, index, &mut c.entry, None);
    });
}

/// A flute's sections (docs/specs/fluting.md §8): its depths, with the width at the top
/// beneath, passes, ramps; feeds. Its ramps are its entry.
fn flute_sections(ui: &mut egui::Ui, f: &mut chipload_job::FluteOp, tool: Option<&Tool>) {
    use chipload_job::RampShape;
    let units = crate::units::display(ui.ctx());
    let len_unit = chipload_pipeline::units::length_unit(units);
    let with_unit = |mm: f64| format!("{} {len_unit}", len_in(units, mm));
    let shape_name = |ui: &egui::Ui, shape: RampShape| match shape {
        RampShape::Straight => t!(ui, "forms-ramp-straight"),
        RampShape::Curved => t!(ui, "forms-ramp-curved"),
    };
    let summary = t!(
        ui,
        "forms-flute-cut-summary",
        depth = with_unit(f.end_depth - f.start_depth),
        start = with_unit(f.ramp_start),
        end = with_unit(f.ramp_end),
        shape = shape_name(ui, f.ramp_shape)
    );
    folded_section(ui, "op-cut", &t!(ui, "forms-section-cut"), &summary, |ui| {
        ui.label(t!(ui, "forms-start-depth"))
            .on_hover_text(t!(ui, "forms-flute-start-hover"));
        flag(ui.add(mm(&mut f.start_depth)), ui, "start_depth");
        ui.end_row();
        ui.label(t!(ui, "forms-flute-depth"))
            .on_hover_text(t!(ui, "forms-flute-depth-hover"));
        flag(ui.add(mm(&mut f.end_depth)), ui, "end_depth");
        ui.end_row();
        let depth = f.end_depth - f.start_depth;
        if let Some(tool) = tool
            && let Some(width) = chipload_toolpath::flute::flute_width_mm(tool, depth)
        {
            ui.label("");
            ui.label(t!(ui, "forms-flute-width-value", width = with_unit(width)));
            ui.end_row();
        }
        ui.label(t!(ui, "forms-step-down"))
            .on_hover_text(t!(ui, "forms-flute-passes-hover"));
        let mut passes = f.step_down.is_some();
        let tick = ui.checkbox(&mut passes, t!(ui, "forms-chamfer-passes"));
        if flag(tick, ui, "step_down").changed() {
            f.step_down = passes.then(|| (depth / 2.0).max(0.1));
        }
        ui.end_row();
        if let Some(step) = &mut f.step_down {
            ui.label(format!("  {}", t!(ui, "forms-step-down")));
            flag(ui.add(mm(step)), ui, "step_down");
            ui.end_row();
        }
        ui.label(t!(ui, "forms-flute-ramp-start"))
            .on_hover_text(t!(ui, "forms-flute-ramp-start-hover"));
        flag(ui.add(mm(&mut f.ramp_start)), ui, "ramp_start");
        ui.end_row();
        ui.label(t!(ui, "forms-flute-ramp-end"))
            .on_hover_text(t!(ui, "forms-flute-ramp-end-hover"));
        flag(ui.add(mm(&mut f.ramp_end)), ui, "ramp_end");
        ui.end_row();
        ui.label(t!(ui, "forms-flute-ramp"))
            .on_hover_text(t!(ui, "forms-flute-ramp-hover"));
        let combo = ComboBox::from_id_salt("flute-ramp-shape")
            .selected_text(shape_name(ui, f.ramp_shape))
            .show_ui(ui, |ui| {
                for shape in [RampShape::Straight, RampShape::Curved] {
                    let label = shape_name(ui, shape);
                    if ui.selectable_label(f.ramp_shape == shape, label).clicked() {
                        f.ramp_shape = shape;
                    }
                }
            });
        flag(combo.response, ui, "ramp_shape");
        ui.end_row();
    });
    feeds_section(ui, &mut f.feed, &mut f.plunge_feed, tool);
}

/// A moulding's sections (docs/specs/moulding.md §9): the profile and its size, the side,
/// the ends; start depth, stepover, levels and direction; feeds.
fn moulding_sections(
    ui: &mut egui::Ui,
    index: usize,
    m: &mut chipload_job::MouldingOp,
    tool: Option<&Tool>,
    contours: &[Contour],
) {
    use chipload_job::{MouldEnds, MouldProfile, MouldSide, RampShape};
    let units = crate::units::display(ui.ctx());
    let len_unit = chipload_pipeline::units::length_unit(units);
    let with_unit = |mm: f64| format!("{} {len_unit}", len_in(units, mm));
    let profile_name = |ui: &egui::Ui, p: &MouldProfile| match p {
        MouldProfile::Cove { .. } => t!(ui, "forms-moulding-cove"),
        MouldProfile::Roundover { .. } => t!(ui, "forms-moulding-roundover"),
        MouldProfile::Ogee { .. } => t!(ui, "forms-moulding-ogee"),
        MouldProfile::Bead { .. } => t!(ui, "forms-moulding-bead"),
        MouldProfile::Flute { .. } => t!(ui, "forms-moulding-flute"),
        MouldProfile::Shape { .. } => t!(ui, "forms-moulding-shape"),
    };
    let side_name = |ui: &egui::Ui, s: MouldSide| match s {
        MouldSide::Left => t!(ui, "forms-moulding-left"),
        MouldSide::Right => t!(ui, "forms-moulding-right"),
        MouldSide::Inside => t!(ui, "forms-side-inside"),
        MouldSide::Outside => t!(ui, "forms-side-outside"),
    };
    let summary = t!(
        ui,
        "forms-moulding-summary",
        profile = profile_name(ui, &m.profile),
        side = side_name(ui, m.side)
    );
    let title = t!(ui, "forms-section-profile");
    folded_section(ui, "op-moulding", &title, &summary, |ui| {
        ui.label(t!(ui, "forms-moulding-profile"));
        let size = m.profile.depth().unwrap_or(6.0);
        let choices = [
            MouldProfile::Cove { radius: size },
            MouldProfile::Roundover { radius: size },
            MouldProfile::Ogee {
                width: 2.0 * size,
                depth: size,
            },
            MouldProfile::Bead { radius: size },
            MouldProfile::Flute { radius: size },
            MouldProfile::Shape { shape: 0 },
        ];
        let current = m.profile;
        let combo = ComboBox::from_id_salt(("moulding-profile", index))
            .selected_text(profile_name(ui, &m.profile))
            .show_ui(ui, |ui| {
                for choice in choices {
                    let same =
                        std::mem::discriminant(&choice) == std::mem::discriminant(&m.profile);
                    // A drawn profile shows the shape it has, if it has one.
                    let shown = if same { current } else { choice };
                    let label = profile_name(ui, &choice);
                    let clicked = ui
                        .horizontal(|ui| {
                            let points = chipload_toolpath::moulding::preview(&shown, contours);
                            section_picture(ui, points.as_deref(), None, egui::vec2(44.0, 22.0));
                            ui.selectable_label(same, label).clicked()
                        })
                        .inner;
                    if clicked && !same {
                        m.profile = choice;
                    }
                }
            });
        flag(combo.response, ui, "profile");
        ui.end_row();
        match &mut m.profile {
            MouldProfile::Cove { radius }
            | MouldProfile::Roundover { radius }
            | MouldProfile::Bead { radius }
            | MouldProfile::Flute { radius } => {
                ui.label(t!(ui, "forms-pattern-radius"));
                flag(ui.add(mm(radius)), ui, "profile.radius");
                ui.end_row();
            }
            MouldProfile::Ogee { width, depth } => {
                ui.label(t!(ui, "forms-moulding-width"));
                flag(ui.add(mm(width)), ui, "profile.width");
                ui.end_row();
                ui.label(t!(ui, "forms-flute-depth"));
                flag(ui.add(mm(depth)), ui, "profile.depth");
                ui.end_row();
            }
            MouldProfile::Shape { shape } => {
                ui.label(t!(ui, "forms-moulding-shape-id"))
                    .on_hover_text(t!(ui, "forms-moulding-shape-hover"));
                let field = ui.add(egui::DragValue::new(shape).prefix("S").range(0..=u64::MAX));
                flag(field, ui, "profile.shape");
                ui.end_row();
            }
        }
        // The section, true to scale: the board with the profile cut into it, the path at
        // the left.
        ui.label("");
        let points = chipload_toolpath::moulding::preview(&m.profile, contours);
        let passes = tool.and_then(|t| t.cutter()).and_then(|cutter| {
            chipload_toolpath::moulding::preview_tool(&m.profile, contours, cutter, m.stepover)
        });
        section_picture(
            ui,
            points.as_deref(),
            passes.as_ref(),
            egui::vec2(170.0, 72.0),
        )
        .on_hover_text(t!(ui, "forms-moulding-preview-hover"));
        ui.end_row();
        ui.label(t!(ui, "forms-side"))
            .on_hover_text(t!(ui, "forms-moulding-side-hover"));
        let combo = ComboBox::from_id_salt(("moulding-side", index))
            .selected_text(side_name(ui, m.side))
            .show_ui(ui, |ui| {
                for side in [
                    MouldSide::Outside,
                    MouldSide::Inside,
                    MouldSide::Left,
                    MouldSide::Right,
                ] {
                    let label = side_name(ui, side);
                    if ui.selectable_label(m.side == side, label).clicked() {
                        m.side = side;
                    }
                }
            });
        flag(combo.response, ui, "side");
        ui.end_row();
        let ends_name = |ui: &egui::Ui, e: &MouldEnds| match e {
            MouldEnds::Square => t!(ui, "forms-moulding-square"),
            MouldEnds::Round => t!(ui, "forms-moulding-round"),
            MouldEnds::Taper { .. } => t!(ui, "forms-moulding-taper"),
        };
        ui.label(t!(ui, "forms-moulding-ends"))
            .on_hover_text(t!(ui, "forms-moulding-ends-hover"));
        let ends = [
            MouldEnds::Square,
            MouldEnds::Round,
            MouldEnds::Taper {
                length: 30.0,
                shape: RampShape::Straight,
            },
        ];
        let combo = ComboBox::from_id_salt(("moulding-ends", index))
            .selected_text(ends_name(ui, &m.ends))
            .show_ui(ui, |ui| {
                for choice in ends {
                    let same = std::mem::discriminant(&choice) == std::mem::discriminant(&m.ends);
                    let label = ends_name(ui, &choice);
                    if ui.selectable_label(same, label).clicked() && !same {
                        m.ends = choice;
                    }
                }
            });
        flag(combo.response, ui, "ends");
        ui.end_row();
        if let MouldEnds::Taper { length, shape } = &mut m.ends {
            ui.label(format!("  {}", t!(ui, "forms-moulding-taper-length")));
            flag(ui.add(mm(length)), ui, "ends.length");
            ui.end_row();
            ui.label(format!("  {}", t!(ui, "forms-flute-ramp")));
            let shape_name = |ui: &egui::Ui, s: RampShape| match s {
                RampShape::Straight => t!(ui, "forms-ramp-straight"),
                RampShape::Curved => t!(ui, "forms-ramp-curved"),
            };
            let combo = ComboBox::from_id_salt(("moulding-taper-shape", index))
                .selected_text(shape_name(ui, *shape))
                .show_ui(ui, |ui| {
                    for s in [RampShape::Straight, RampShape::Curved] {
                        let label = shape_name(ui, s);
                        if ui.selectable_label(*shape == s, label).clicked() {
                            *shape = s;
                        }
                    }
                });
            flag(combo.response, ui, "ends.shape");
            ui.end_row();
        }
    });
    let cut = t!(
        ui,
        "forms-moulding-cut-summary",
        stepover = with_unit(m.stepover),
        direction = direction_name(ui, m.direction)
    );
    folded_section(ui, "op-cut", &t!(ui, "forms-section-cut"), &cut, |ui| {
        ui.label(t!(ui, "forms-start-depth"));
        flag(ui.add(mm(&mut m.start_depth)), ui, "start_depth");
        ui.end_row();
        ui.label(t!(ui, "forms-moulding-stepover"))
            .on_hover_text(t!(ui, "forms-moulding-stepover-hover"));
        flag(ui.add(mm(&mut m.stepover)), ui, "stepover");
        ui.end_row();
        ui.label(t!(ui, "forms-step-down"));
        let mut levels = m.step_down.is_some();
        let tick = ui.checkbox(&mut levels, t!(ui, "forms-chamfer-passes"));
        if flag(tick, ui, "step_down").changed() {
            m.step_down = levels.then(|| tool.map_or(2.0, |t| (t.diameter / 3.0).max(0.5)));
        }
        ui.end_row();
        if let Some(step) = &mut m.step_down {
            ui.label(format!("  {}", t!(ui, "forms-step-down")));
            flag(ui.add(mm(step)), ui, "step_down");
            ui.end_row();
        }
        ui.label(t!(ui, "forms-direction"));
        direction_combo(ui, ("direction", index), &mut m.direction);
        ui.end_row();
    });
    feeds_section(ui, &mut m.feed, &mut m.plunge_feed, tool);
}

/// A small drawing of a moulding's section (docs/specs/moulding.md §9), `size` on screen:
/// the board in section with the profile cut into it, true to scale, a mark at the path;
/// with `passes`, the tool at its deepest pass and a dot at each pass's tip. `points` are
/// (distance across, Z) from `moulding::preview`; None draws a question mark (a drawn
/// profile that can't be read).
fn section_picture(
    ui: &mut egui::Ui,
    points: Option<&[(f64, f64)]>,
    passes: Option<&chipload_toolpath::moulding::PreviewTool>,
    size: egui::Vec2,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::hover());
    let painter = ui.painter_at(rect);
    let visuals = ui.visuals();
    // A wood tone, light or dark with the theme, so the board reads against the panel.
    let board = if visuals.dark_mode {
        egui::Color32::from_rgb(0x8a, 0x6a, 0x45)
    } else {
        egui::Color32::from_rgb(0xd9, 0xbc, 0x8c)
    };
    let line = visuals.text_color();
    let Some(points) = points.filter(|p| p.len() >= 2) else {
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "?",
            egui::FontId::proportional(size.y * 0.6),
            line,
        );
        return response;
    };
    let width = points.iter().map(|p| p.0).fold(0.0, f64::max).max(0.1);
    let depth = -points.iter().map(|p| p.1).fold(0.0, f64::min);
    // Room either side of the profile and below its deepest point, in mm.
    let pad = 0.25 * width.max(depth);
    // Room for the tool too, where it reaches past the profile.
    let (tool_lo, tool_hi) = passes.map_or((0.0, width), |p| {
        p.outline.iter().fold((0.0, width), |(lo, hi), q| {
            (f64::min(lo, q.0), f64::max(hi, q.0))
        })
    });
    let (x0, x1) = (tool_lo.min(0.0) - pad, tool_hi.max(width) + pad);
    let (z0, z1) = (-(depth + pad), pad * 0.5);
    let margin = 2.0;
    let scale = ((rect.width() - 2.0 * margin) as f64 / (x1 - x0))
        .min((rect.height() - 2.0 * margin) as f64 / (z1 - z0));
    let to_screen = |x: f64, z: f64| {
        egui::pos2(
            rect.left() + margin + ((x - x0) * scale) as f32,
            rect.top() + margin + ((z1 - z) * scale) as f32,
        )
    };
    // The board: an edge profile (below the surface at the path) has waste before the path.
    let edge = points.first().is_some_and(|p| p.1 < -0.01);
    let mut outline: Vec<egui::Pos2> = Vec::new();
    if edge {
        outline.push(to_screen(0.0, z0));
    } else {
        outline.push(to_screen(x0, z0));
        outline.push(to_screen(x0, 0.0));
    }
    outline.extend(points.iter().map(|&(x, z)| to_screen(x, z)));
    outline.push(to_screen(width, 0.0));
    outline.push(to_screen(x1, 0.0));
    outline.push(to_screen(x1, z0));
    // Filled in vertical strips under the outline (the profile can be concave), as one mesh
    // so no seams show between them.
    let bottom = to_screen(0.0, z0).y;
    let mut mesh = egui::epaint::Mesh::default();
    for w in outline.windows(2) {
        let (a, b) = (w[0], w[1]);
        let base = mesh.vertices.len() as u32;
        for p in [a, b, egui::pos2(b.x, bottom), egui::pos2(a.x, bottom)] {
            mesh.colored_vertex(p, board);
        }
        mesh.add_triangle(base, base + 1, base + 2);
        mesh.add_triangle(base, base + 2, base + 3);
    }
    painter.add(egui::Shape::mesh(mesh));
    painter.add(egui::Shape::line(outline, egui::Stroke::new(1.2, line)));
    // The tool at its deepest pass, and where each pass's tip goes.
    if let Some(p) = passes {
        let tool_color = visuals.selection.stroke.color;
        let outline: Vec<egui::Pos2> = p.outline.iter().map(|&(x, z)| to_screen(x, z)).collect();
        painter.add(egui::Shape::line(
            outline,
            egui::Stroke::new(1.2, tool_color),
        ));
        for &(x, z) in &p.tips {
            painter.circle_filled(to_screen(x, z), 1.6, tool_color);
        }
    }
    // The path: a short tick at the surface.
    let at = to_screen(0.0, 0.0);
    painter.line_segment(
        [at + egui::vec2(0.0, -4.0), at + egui::vec2(0.0, 2.0)],
        egui::Stroke::new(1.5, visuals.selection.stroke.color),
    );
    response
}

/// A prism's sections (docs/specs/prism.md §5): start depth, max depth (with how deep the
/// bit's cone reaches), passes and direction; feeds.
fn prism_sections(
    ui: &mut egui::Ui,
    index: usize,
    f: &mut chipload_job::PrismOp,
    tool: Option<&Tool>,
) {
    let units = crate::units::display(ui.ctx());
    let len_unit = chipload_pipeline::units::length_unit(units);
    let with_unit = |mm: f64| format!("{} {len_unit}", len_in(units, mm));
    let deepest = f.deepest(tool);
    let summary = t!(
        ui,
        "forms-prism-cut-summary",
        depth = with_unit(deepest),
        direction = direction_name(ui, f.direction)
    );
    folded_section(ui, "op-cut", &t!(ui, "forms-section-cut"), &summary, |ui| {
        ui.label(t!(ui, "forms-start-depth"))
            .on_hover_text(t!(ui, "forms-prism-start-hover"));
        flag(ui.add(mm(&mut f.start_depth)), ui, "start_depth");
        ui.end_row();
        ui.label(t!(ui, "forms-prism-max-depth"))
            .on_hover_text(t!(ui, "forms-prism-max-depth-hover"));
        let mut capped = f.max_depth.is_some();
        let tick = ui.checkbox(&mut capped, t!(ui, "forms-prism-flat-top"));
        if flag(tick, ui, "max_depth").changed() {
            f.max_depth = capped.then(|| (deepest / 2.0).max(0.5));
        }
        ui.end_row();
        if let Some(max) = &mut f.max_depth {
            ui.label(format!("  {}", t!(ui, "forms-prism-max-depth")));
            flag(ui.add(mm(max)), ui, "max_depth");
            ui.end_row();
        }
        ui.label("");
        match tool.and_then(|t| t.angle) {
            Some(angle) if tool.is_some_and(|t| t.kind == chipload_job::ToolType::Vbit) => ui
                .label(t!(
                    ui,
                    "forms-prism-deepest-value",
                    depth = with_unit(deepest),
                    angle = format!("{:.0}", angle.to_degrees())
                )),
            _ => ui.label(t!(ui, "forms-chamfer-needs-vbit")),
        };
        ui.end_row();
        ui.label(t!(ui, "forms-step-down"));
        let mut passes = f.step_down.is_some();
        let tick = ui.checkbox(&mut passes, t!(ui, "forms-chamfer-passes"));
        if flag(tick, ui, "step_down").changed() {
            f.step_down = passes.then(|| (deepest / 2.0).max(0.1));
        }
        ui.end_row();
        if let Some(step) = &mut f.step_down {
            ui.label(format!("  {}", t!(ui, "forms-step-down")));
            flag(ui.add(mm(step)), ui, "step_down");
            ui.end_row();
        }
        ui.label(t!(ui, "forms-direction"));
        direction_combo(ui, ("direction", index), &mut f.direction);
        ui.end_row();
    });
    feeds_section(ui, &mut f.feed, &mut f.plunge_feed, tool);
}

/// An angle in degrees for a radian `value`, -180° to 180°.
fn degrees_field(ui: &mut egui::Ui, value: &mut f64, field: &str) {
    let mut degrees = value.to_degrees();
    let response = ui.add(
        crate::units::number(ui, &mut degrees)
            .speed(1.0)
            .range(-180.0..=180.0)
            .suffix("°"),
    );
    if flag(response, ui, field).changed() {
        *value = degrees.to_radians();
    }
}

/// A fill pattern's sections (docs/specs/fill-pattern.md §8): the pattern and its settings,
/// inside the edge; the depths with the groove's width beneath, passes, ends; feeds.
fn pattern_sections(ui: &mut egui::Ui, f: &mut chipload_job::PatternOp, tool: Option<&Tool>) {
    use chipload_job::FillPattern;
    let units = crate::units::display(ui.ctx());
    let len_unit = chipload_pipeline::units::length_unit(units);
    let with_unit = |mm: f64| format!("{} {len_unit}", len_in(units, mm));
    use chipload_pipeline::new_ops::PatternKind;
    let kind_name = |ui: &egui::Ui, kind: PatternKind| match kind {
        PatternKind::Lines => t!(ui, "forms-pattern-lines"),
        PatternKind::Cross => t!(ui, "forms-pattern-cross"),
        PatternKind::Rings => t!(ui, "forms-pattern-rings"),
        PatternKind::Waves => t!(ui, "forms-pattern-waves"),
        PatternKind::Dashes => t!(ui, "forms-pattern-dashes"),
        PatternKind::Herringbone => t!(ui, "forms-pattern-herringbone"),
        PatternKind::Scales => t!(ui, "forms-pattern-scales"),
        PatternKind::Sunburst => t!(ui, "forms-pattern-sunburst"),
    };
    let current = PatternKind::of(&f.pattern);
    let summary = t!(
        ui,
        "forms-pattern-summary",
        kind = kind_name(ui, current),
        spacing = with_unit(f.pattern.spacing())
    );
    let title = t!(ui, "forms-section-pattern");
    folded_section(ui, "op-pattern", &title, &summary, |ui| {
        ui.label(t!(ui, "forms-pattern"));
        let mut chosen = None;
        let combo = ComboBox::from_id_salt("pattern-kind")
            .selected_text(kind_name(ui, current))
            .show_ui(ui, |ui| {
                for kind in PatternKind::ALL {
                    let label = kind_name(ui, kind);
                    if ui.selectable_label(kind == current, label).clicked() && kind != current {
                        chosen = Some(kind);
                    }
                }
            });
        if let Some(kind) = chosen {
            // The new kind's sizes from the groove; a stroke pattern fluted (fill-pattern.md
            // §14).
            *f = chipload_pipeline::new_ops::switched_pattern(f, kind, tool);
        }
        flag(combo.response, ui, "pattern");
        ui.end_row();
        let spacing_row = |ui: &mut egui::Ui, spacing: &mut f64| {
            ui.label(t!(ui, "forms-pattern-spacing"))
                .on_hover_text(t!(ui, "forms-pattern-spacing-hover"));
            flag(ui.add(mm(spacing)), ui, "pattern.spacing");
            ui.end_row();
        };
        let length_row = |ui: &mut egui::Ui, length: &mut f64| {
            ui.label(t!(ui, "forms-pattern-length"));
            flag(ui.add(mm(length)), ui, "pattern.length");
            ui.end_row();
        };
        let whole_row = |ui: &mut egui::Ui, whole: &mut bool| {
            ui.label(t!(ui, "forms-pattern-whole"))
                .on_hover_text(t!(ui, "forms-pattern-whole-hover"));
            let tick = ui.checkbox(whole, "");
            flag(tick, ui, "pattern.whole");
            ui.end_row();
        };
        let angle_row = |ui: &mut egui::Ui, angle: &mut f64| {
            ui.label(t!(ui, "forms-pattern-angle"))
                .on_hover_text(t!(ui, "forms-pattern-angle-hover"));
            degrees_field(ui, angle, "pattern.angle");
            ui.end_row();
        };
        match &mut f.pattern {
            FillPattern::Lines { angle, spacing } => {
                angle_row(ui, angle);
                spacing_row(ui, spacing);
            }
            FillPattern::Cross {
                angle,
                spacing,
                between,
            } => {
                angle_row(ui, angle);
                spacing_row(ui, spacing);
                ui.label(t!(ui, "forms-pattern-between"))
                    .on_hover_text(t!(ui, "forms-pattern-between-hover"));
                degrees_field(ui, between, "pattern.between");
                ui.end_row();
            }
            FillPattern::Rings { spacing } => spacing_row(ui, spacing),
            FillPattern::Waves {
                angle,
                spacing,
                wavelength,
                height,
            } => {
                angle_row(ui, angle);
                spacing_row(ui, spacing);
                ui.label(t!(ui, "forms-pattern-wavelength"));
                flag(ui.add(mm(wavelength)), ui, "pattern.wavelength");
                ui.end_row();
                ui.label(t!(ui, "forms-pattern-height"))
                    .on_hover_text(t!(ui, "forms-pattern-height-hover"));
                flag(ui.add(mm(height)), ui, "pattern.height");
                ui.end_row();
            }
            FillPattern::Dashes {
                angle,
                spacing,
                length,
                gap,
                whole,
            } => {
                angle_row(ui, angle);
                spacing_row(ui, spacing);
                length_row(ui, length);
                ui.label(t!(ui, "forms-pattern-gap"));
                flag(ui.add(mm(gap)), ui, "pattern.gap");
                ui.end_row();
                whole_row(ui, whole);
            }
            FillPattern::Herringbone {
                angle,
                spacing,
                length,
                whole,
            } => {
                angle_row(ui, angle);
                spacing_row(ui, spacing);
                length_row(ui, length);
                whole_row(ui, whole);
            }
            FillPattern::Scales {
                angle,
                spacing,
                radius,
                whole,
            } => {
                angle_row(ui, angle);
                spacing_row(ui, spacing);
                ui.label(t!(ui, "forms-pattern-radius"));
                flag(ui.add(mm(radius)), ui, "pattern.radius");
                ui.end_row();
                whole_row(ui, whole);
            }
            FillPattern::Sunburst {
                spacing,
                length,
                pitch,
                whole,
            } => {
                spacing_row(ui, spacing);
                length_row(ui, length);
                ui.label(t!(ui, "forms-pattern-pitch"))
                    .on_hover_text(t!(ui, "forms-pattern-pitch-hover"));
                flag(ui.add(mm(pitch)), ui, "pattern.pitch");
                ui.end_row();
                whole_row(ui, whole);
            }
        }
        ui.label(t!(ui, "forms-pattern-inside"))
            .on_hover_text(t!(ui, "forms-pattern-inside-hover"));
        let tick = ui.checkbox(&mut f.inside, "");
        flag(tick, ui, "inside");
        ui.end_row();
    });
    let depth = f.end_depth - f.start_depth;
    let cut = t!(ui, "forms-pattern-cut-summary", depth = with_unit(depth));
    folded_section(ui, "op-cut", &t!(ui, "forms-section-cut"), &cut, |ui| {
        ui.label(t!(ui, "forms-start-depth"));
        flag(ui.add(mm(&mut f.start_depth)), ui, "start_depth");
        ui.end_row();
        ui.label(t!(ui, "forms-flute-depth"));
        flag(ui.add(mm(&mut f.end_depth)), ui, "end_depth");
        ui.end_row();
        if let Some(tool) = tool
            && let Some(width) = chipload_toolpath::flute::flute_width_mm(tool, depth)
        {
            ui.label("");
            ui.label(t!(
                ui,
                "forms-pattern-width-value",
                width = with_unit(width)
            ));
            ui.end_row();
        }
        ui.label(t!(ui, "forms-step-down"));
        let mut passes = f.step_down.is_some();
        let tick = ui.checkbox(&mut passes, t!(ui, "forms-chamfer-passes"));
        if flag(tick, ui, "step_down").changed() {
            f.step_down = passes.then(|| (depth / 2.0).max(0.1));
        }
        ui.end_row();
        if let Some(step) = &mut f.step_down {
            ui.label(format!("  {}", t!(ui, "forms-step-down")));
            flag(ui.add(mm(step)), ui, "step_down");
            ui.end_row();
        }
        ui.label(t!(ui, "forms-pattern-ends"))
            .on_hover_text(t!(ui, "forms-pattern-ends-hover"));
        flag(ui.add(mm(&mut f.ramp)), ui, "ramp");
        ui.end_row();
    });
    feeds_section(ui, &mut f.feed, &mut f.plunge_feed, tool);
}

/// A pocket's lead-in and lead-out (docs/specs/helix-and-leads.md part B): on or off, and
/// the radius, the tool radius by default.
pub(super) fn lead_rows(ui: &mut egui::Ui, lead: &mut Option<Lead>, tool_diameter: f64) {
    ui.label(t!(ui, "forms-lead"))
        .on_hover_text(t!(ui, "forms-lead-hover"));
    let mut on = lead.is_some();
    let response = ui.checkbox(&mut on, t!(ui, "forms-lead-in-out"));
    let response = flag(response, ui, "lead");
    if response.changed() {
        *lead = on.then_some(Lead {
            radius: tool_diameter / 2.0,
        });
    }
    ui.end_row();
    if let Some(lead) = lead {
        ui.label(format!("  {}", t!(ui, "forms-lead-radius")));
        flag(ui.add(mm(&mut lead.radius)), ui, "lead");
        ui.end_row();
    }
}

/// A V-carve's depth passes (docs/specs/vcarve-step-down.md §3): equal load, a number of
/// passes (v-carve.md decision 12), or an ordinary step down; their depths beneath.
pub(super) fn vcarve_pass_rows(
    ui: &mut egui::Ui,
    index: usize,
    v: &mut chipload_job::VcarveOp,
    depths: &[f64],
    units: Units,
) {
    let len_unit = chipload_pipeline::units::length_unit(units);
    ui.label(t!(ui, "forms-depth-passes"))
        .on_hover_text(t!(ui, "forms-depth-passes-hover"));
    let equal_load = t!(ui, "forms-depth-passes-equal-load");
    let step_down = t!(ui, "forms-step-down");
    let chosen = if v.step_down.is_some() {
        step_down.clone()
    } else {
        equal_load.clone()
    };
    ComboBox::from_id_salt(("vcarve-depth-passes", index))
        .selected_text(chosen)
        .show_ui(ui, |ui| {
            if ui
                .selectable_label(v.step_down.is_none(), equal_load)
                .clicked()
            {
                v.step_down = None;
            }
            if ui
                .selectable_label(v.step_down.is_some(), step_down)
                .clicked()
                && v.step_down.is_none()
            {
                // The first pass stays as deep as it was.
                let first = depths.first().map_or(v.end_depth, |d| d - v.start_depth);
                v.step_down = Some((first * 10.0).round().max(1.0) / 10.0);
            }
        });
    ui.end_row();
    match &mut v.step_down {
        Some(step) => {
            ui.label(t!(ui, "forms-step-down"))
                .on_hover_text(t!(ui, "forms-vcarve-step-down-hover"));
            flag(ui.add(mm(step)), ui, "step_down");
        }
        None => {
            ui.label(t!(ui, "forms-passes"))
                .on_hover_text(t!(ui, "forms-vcarve-passes-hover"));
            let most = chipload_job::MAX_VCARVE_PASSES;
            flag(
                ui.add(crate::units::number(ui, &mut v.passes).range(1..=most)),
                ui,
                "passes",
            );
        }
    }
    ui.end_row();
    if depths.len() > 1 {
        let depths: Vec<String> = depths.iter().map(|d| len_in(units, *d)).collect();
        ui.label("");
        ui.add(
            egui::Label::new(
                RichText::new(t!(
                    ui,
                    "forms-vcarve-pass-depths",
                    depths = format!("{} {len_unit}", depths.join(", "))
                ))
                .weak(),
            )
            .wrap(),
        );
        ui.end_row();
    }
}

/// With a V-bit: the groove's width at the surface, which sets the end depth when changed
/// (docs/specs/sign-maker.md §5: a border is a groove of a width). Shown only, through the
/// stock.
fn groove_width_row(ui: &mut egui::Ui, tool: Option<&Tool>, end_depth: &mut f64, through: bool) {
    let Some(tool) = tool else {
        return;
    };
    let Some(mut width) = tool.vbit_width_at(*end_depth) else {
        return;
    };
    ui.label(t!(ui, "forms-groove-width"))
        .on_hover_text(t!(ui, "forms-groove-width-hover"));
    let field = ui.add_enabled(!through, crate::units::length(&mut width));
    if field.changed()
        && let Some(depth) = tool.vbit_depth_for(width.min(tool.diameter))
    {
        *end_depth = depth;
    }
    ui.end_row();
}

/// A relief's rim (docs/specs/stl-relief.md §16): none, or a flat round the outline or a
/// box, then a ramp up to the stock top.
fn rim_section(ui: &mut egui::Ui, rim: &mut Option<chipload_job::ReliefRim>) {
    use chipload_job::RimShape;
    let units = crate::units::display(ui.ctx());
    let len_unit = chipload_pipeline::units::length_unit(units);
    let summary = match rim {
        None => t!(ui, "forms-rim-none"),
        Some(r) => t!(
            ui,
            "forms-rim-summary",
            width = format!("{} {len_unit}", len_in(units, r.width)),
            shape = match r.shape {
                RimShape::Outline => t!(ui, "forms-rim-outline"),
                RimShape::Box => t!(ui, "forms-rim-box"),
            },
            angle = format!("{:.0}", r.ramp.to_degrees())
        ),
    };
    folded_section(ui, "op-rim", &t!(ui, "forms-section-rim"), &summary, |ui| {
        let mut on = rim.is_some();
        ui.label(t!(ui, "forms-section-rim"))
            .on_hover_text(t!(ui, "forms-rim-hover"));
        if ui.checkbox(&mut on, "").changed() {
            *rim = on.then_some(chipload_job::DEFAULT_RIM);
        }
        ui.end_row();
        let Some(r) = rim else {
            return;
        };
        ui.label(t!(ui, "forms-rim-shape"));
        let shape_name = |ui: &egui::Ui, shape: RimShape| match shape {
            RimShape::Outline => t!(ui, "forms-rim-outline"),
            RimShape::Box => t!(ui, "forms-rim-box"),
        };
        ComboBox::from_id_salt("rim-shape")
            .selected_text(shape_name(ui, r.shape))
            .show_ui(ui, |ui| {
                for shape in [RimShape::Outline, RimShape::Box] {
                    let name = shape_name(ui, shape);
                    ui.selectable_value(&mut r.shape, shape, name);
                }
            });
        ui.end_row();
        ui.label(t!(ui, "forms-rim-width"))
            .on_hover_text(t!(ui, "forms-rim-width-hover"));
        flag(ui.add(mm(&mut r.width)), ui, "rim.width");
        ui.end_row();
        if r.shape == RimShape::Box {
            ui.label(t!(ui, "forms-rim-corner"));
            flag(ui.add(mm(&mut r.corner_radius)), ui, "rim.corner_radius");
            ui.end_row();
        }
        if r.shape == RimShape::Outline {
            ui.label(t!(ui, "forms-rim-hull"))
                .on_hover_text(t!(ui, "forms-rim-hull-hover"));
            flag(super::hull_slider(ui, &mut r.hull), ui, "rim.hull");
            ui.end_row();
        }
        ui.label(t!(ui, "forms-rim-ramp"))
            .on_hover_text(t!(ui, "forms-rim-ramp-hover"));
        let mut degrees = r.ramp.to_degrees();
        let least = chipload_job::RIM_MIN_RAMP.to_degrees();
        let ramp = ui.add(
            crate::units::number(ui, &mut degrees)
                .speed(1.0)
                .range(least..=90.0)
                .suffix("°"),
        );
        if ramp.changed() {
            r.ramp = degrees.to_radians();
        }
        flag(ramp, ui, "rim.ramp");
        ui.end_row();
    });
}
