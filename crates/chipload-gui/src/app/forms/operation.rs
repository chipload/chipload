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
    let mut request = OpFormRequest::default();
    let mut use_selection = false;
    // For the chip load next to the feed.
    let op_tool: Option<Tool> = choice.tools.iter().find(|t| t.id == op.tool()).cloned();
    let op_tool = op_tool.as_ref();
    let tool_diameter = op_tool.map_or(0.0, |t| t.diameter);
    let summary = geometry_summary(&*op, contours, ui);
    let kind = kind_name(ui, op);
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
                use_selection = geometry_row(ui, &summary, &choice);
                tool_row(ui, index, &mut *op, &choice, &mut request);
                if let Operation::Chamfer(c) = &mut *op {
                    chamfer_side_row(ui, index, c);
                }
                if let Operation::Profile(p) = &mut *op {
                    profile_side_row(ui, index, p, side_chosen);
                }
            });
    });
    match &mut *op {
        Operation::Profile(p) => profile_sections(ui, index, p, stock, op_tool),
        Operation::Pocket(p) => pocket_sections(ui, index, p, choice.tools, stock, op_tool),
        Operation::Vcarve(v) => vcarve_sections(ui, index, v, op_tool),
        Operation::Path(p) => path_sections(ui, index, p, stock, op_tool),
        Operation::Chamfer(c) => chamfer_sections(ui, index, c, op_tool),
        Operation::Flute(f) => flute_sections(ui, f, op_tool),
        Operation::Pattern(f) => pattern_sections(ui, f, op_tool),
        Operation::Prism(f) => prism_sections(ui, index, f, op_tool),
        Operation::Moulding(m) => moulding_sections(ui, index, m, op_tool, contours),
        Operation::Holes(h) => holes_sections(ui, index, h, op_tool, stock),
        Operation::Surface(f) => surface_sections(ui, f, op_tool),
        Operation::Relief(r) => {
            request.relief_depth = relief_sections(ui, index, r, op_tool, tool_diameter);
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

/// The operation's kind in the form's title.
fn kind_name(ui: &egui::Ui, op: &Operation) -> String {
    match op {
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
    }
}

/// A profile's side: outside, inside or on the line; "choose" until the user has chosen.
fn profile_side_row(ui: &mut egui::Ui, index: usize, p: &mut ProfileOp, side_chosen: &mut bool) {
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

/// What the operation cuts, with *Use selection*; true when that was clicked.
fn geometry_row(ui: &mut egui::Ui, summary: &str, choice: &ToolChoice<'_>) -> bool {
    let mut clicked = false;
    let geometry = wrapped_cell(ui, |ui| {
        ui.label(summary);
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
            clicked = true;
        }
    });
    flag(geometry.response, ui, "select");
    ui.end_row();
    clicked
}
