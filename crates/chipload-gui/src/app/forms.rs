// SPDX-License-Identifier: GPL-3.0-or-later
//! The property forms: one operation, and the job settings.

use super::problems::{self, flag};
use super::user_library::{LibraryRequest, LibraryRows, library_rows};
use super::*;
use crate::editor::DEFAULT_STEP_DOWN;
use crate::i18n::{Lang, Language};
use chipload_job::library::{Cutting, MachineProfile};
use chipload_job::{Copies, CopyOrder, Corners, DEFAULT_CORNER_ANGLE_RAD, MAX_COPIES_PER_SIDE};
use chipload_job::{
    DEFAULT_RAMP_ANGLE_RAD, Entry, Lead, MAX_RAMP_ANGLE_RAD, Machine, Tool, rest_refusal,
};
use chipload_job::{
    Finish, HoleMethod, HolesOp, ProfileOp, Rest, STRAIGHT_FLANK_RAD, Skin, ToolChange,
};
use chipload_job::{ImageTrace, Placement, Units};
use chipload_pipeline::library::Library;
use chipload_pipeline::placement::Move as PlaceMove;

pub(super) fn mm(value: &mut f64) -> crate::units::Field<'_> {
    crate::units::length(value)
        .speed(0.1)
        .range(0.0..=100_000.0)
}

/// Start depth, end depth and clearance (docs/specs/depths-and-clearance.md). `through`:
/// the end depth that cuts through the stock (its thickness plus the allowed overcut).
fn depth_rows(
    ui: &mut egui::Ui,
    start: &mut f64,
    end: &mut f64,
    clearance: &mut f64,
    can_clear: bool,
    through: f64,
) {
    let units = crate::units::display(ui.ctx());
    let len_unit = chipload_pipeline::units::length_unit(units);
    ui.label(t!(ui, "forms-start-depth"))
        .on_hover_text(t!(ui, "forms-start-depth-hover"));
    flag(ui.add(mm(start)), ui, "start_depth");
    ui.end_row();
    ui.label(t!(ui, "forms-end-depth"))
        .on_hover_text(t!(ui, "forms-end-depth-hover"));
    wrapped_cell(ui, |ui| {
        flag(ui.add(mm(end)), ui, "end_depth");
        if ui
            .small_button(t!(ui, "forms-through"))
            .on_hover_text(t!(
                ui,
                "forms-through-hover",
                depth = format!("{} {len_unit}", len_in(units, through))
            ))
            .clicked()
        {
            *end = through;
        }
    });
    ui.end_row();
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

pub(super) fn feed(value: &mut f64) -> crate::units::Field<'_> {
    crate::units::feed(value).speed(10.0).range(0.0..=100_000.0)
}

pub(super) fn direction_combo(ui: &mut egui::Ui, id: (&str, usize), direction: &mut Direction) {
    ComboBox::from_id_salt(id)
        .selected_text(direction_name(ui, *direction))
        .show_ui(ui, |ui| {
            for choice in [Direction::Climb, Direction::Conventional] {
                ui.selectable_value(direction, choice, direction_name(ui, choice));
            }
        });
}

/// Least width of a form's label column, so the sections' grids line up; a narrow panel
/// gives it less, down to [`NARROW_WIDTH`].
const LABEL_WIDTH: f32 = 130.0;

/// The least a label column or a field shrinks to in a narrow panel.
const NARROW_WIDTH: f32 = 80.0;

/// Where the panel's width is kept for [`label_width`] (`set_form_width`).
const FORM_WIDTH_ID: &str = "form-width";

/// The width the Properties forms get this frame: their label columns follow it, so the
/// panel can be made narrow (user report 2026-10-17) and every grid still lines up.
pub(super) fn set_form_width(ui: &egui::Ui) {
    let width = ui.available_width();
    ui.data_mut(|d| d.insert_temp(egui::Id::new(FORM_WIDTH_ID), width));
}

/// The forms' width this frame (`set_form_width`); unbounded outside the panel.
fn form_width(ui: &egui::Ui) -> f32 {
    ui.data(|d| d.get_temp::<f32>(egui::Id::new(FORM_WIDTH_ID)))
        .unwrap_or(f32::INFINITY)
}

/// A form's label column: [`LABEL_WIDTH`], or 40 % of a narrower panel.
fn label_width(ui: &egui::Ui) -> f32 {
    (0.4 * form_width(ui)).clamp(NARROW_WIDTH, LABEL_WIDTH)
}

/// A field's width: `wanted`, or what the label column (with a section's indent) leaves
/// of a narrow panel. Worked out from the panel, as a grid's cell doesn't know its own.
pub(super) fn field_width(ui: &egui::Ui, wanted: f32) -> f32 {
    let spacing = ui.spacing();
    let left = form_width(ui) - label_width(ui) - spacing.indent - 2.0 * spacing.item_spacing.x;
    wanted.min(left).max(NARROW_WIDTH)
}

/// A collapsible section of a form: a bold title with a short summary beside it (seen when
/// folded), and its rows in a two-column grid. Its open state is kept per `key`.
pub(super) fn section<R>(
    ui: &mut egui::Ui,
    key: &str,
    title: &str,
    summary: &str,
    rows: impl FnOnce(&mut egui::Ui) -> R,
) -> Option<R> {
    let marked = problems::section_has_problem(ui, key);
    section_marked(ui, key, title, summary, marked, rows)
}

/// A section whose header has a red dot when `marked`: a field in it has a problem
/// (docs/specs/live-problems.md §3).
fn section_marked<R>(
    ui: &mut egui::Ui,
    key: &str,
    title: &str,
    summary: &str,
    marked: bool,
    rows: impl FnOnce(&mut egui::Ui) -> R,
) -> Option<R> {
    let mut header = egui::text::LayoutJob::default();
    let style = ui.style().clone();
    if marked {
        RichText::new("● ")
            .color(ui.visuals().error_fg_color)
            .append_to(
                &mut header,
                &style,
                egui::FontSelection::Default,
                Align::Center,
            );
    }
    RichText::new(title).strong().append_to(
        &mut header,
        &style,
        egui::FontSelection::Default,
        Align::Center,
    );
    if !summary.is_empty() {
        RichText::new(format!("   {summary}")).weak().append_to(
            &mut header,
            &style,
            egui::FontSelection::Default,
            Align::Center,
        );
    }
    // A long summary ends in "…" in a narrow panel (past the arrow and a section's indent).
    // Laid out here: the header lays text out again unwrapped, all but a finished galley.
    header.wrap = egui::text::TextWrapping {
        max_width: (form_width(ui) - 2.0 * ui.spacing().indent).max(NARROW_WIDTH),
        max_rows: 1,
        break_anywhere: true,
        overflow_character: Some('…'),
    };
    let header = ui.fonts_mut(|f| f.layout_job(header));
    egui::CollapsingHeader::new(header)
        .id_salt(("section", key))
        .default_open(true)
        .show(ui, |ui| {
            Grid::new(("section-grid", key))
                .num_columns(2)
                .min_col_width(label_width(ui))
                .show(ui, rows)
                .inner
        })
        .body_returned
}

/// A grid's value cell whose row wraps within the panel: a field with a button or a note
/// beside it, which goes under it in a narrow panel.
fn wrapped_cell<R>(
    ui: &mut egui::Ui,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::InnerResponse<R> {
    let width = field_width(ui, f32::INFINITY);
    ui.scope(|ui| {
        ui.set_max_width(width);
        ui.horizontal_wrapped(add_contents).inner
    })
}

/// An operation's chip load beside its feed, with where typical values lie on hover, in
/// the app's language and display units.
fn chip_load_label(ui: &mut egui::Ui, load: chipload_pipeline::chip_load::ChipLoad, units: Units) {
    let (lang, units) = (ui.lang(), units.into());
    let hint = chipload_pipeline::chip_load::chip_load_hint().render(lang, units);
    ui.label(RichText::new(load.message().render(lang, units)).weak())
        .on_hover_text(hint);
}

/// The tabs' flank angle in degrees, and how far the feet reach (docs/specs/
/// profile-spiral.md §4).
fn flank_row(ui: &mut egui::Ui, tabs: &mut Tabs) {
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
            let foot = tabs.height / tabs.flank_angle.tan();
            let length = format!("{} {len_unit}", len_in(units, foot));
            ui.weak(t!(ui, "forms-flank-feet", length = length));
        }
    });
    ui.end_row();
}

/// Degrees for a summary: one decimal, no trailing zero.
fn degrees(radians: f64) -> String {
    let text = format!("{:.1}", radians.to_degrees());
    text.trim_end_matches('0').trim_end_matches('.').to_string()
}

fn direction_name(ui: &egui::Ui, direction: Direction) -> String {
    match direction {
        Direction::Climb => t!(ui, "forms-climb"),
        Direction::Conventional => t!(ui, "forms-conventional"),
    }
}

fn entry_summary(lang: Language, entry: &Entry, lead: Option<&Lead>, units: Units) -> String {
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
pub(super) struct OpFormRequest {
    /// "Use selection" was clicked.
    pub use_selection: bool,
    /// Copy this library tool into the project and cut this operation with it.
    pub tool_from_library: Option<String>,
}

/// What the operation form needs to know about tools: the project's, the library's names
/// (None: no library), and the operation's tool's cutting values from the library.
pub(super) struct ToolChoice<'a> {
    /// How many contours are selected on the canvas, and which (for *Use selection*).
    pub selected: usize,
    pub selection_text: &'a str,
    pub tools: &'a [Tool],
    pub library: Option<Vec<String>>,
    pub cutting: Option<Cutting>,
}

/// The selected operation's fields, in sections.
/// `through`: the end depth that cuts through the stock, for the Through button.
pub(super) fn op_form(
    ui: &mut egui::Ui,
    index: usize,
    entry: &mut crate::editor::EditOp,
    contours: &[Contour],
    choice: ToolChoice<'_>,
    through: f64,
) -> OpFormRequest {
    let units = crate::units::display(ui.ctx());
    let len_unit = chipload_pipeline::units::length_unit(units);
    let mut request = OpFormRequest::default();
    let mut use_selection = false;
    let tool_diameter = choice
        .tools
        .iter()
        .find(|t| t.id == entry.op.tool())
        .map_or(0.0, |t| t.diameter);
    // For the chip load next to the feed.
    let op_tool: Option<Tool> = choice
        .tools
        .iter()
        .find(|t| t.id == entry.op.tool())
        .cloned();
    let op_tool = op_tool.as_ref();
    let summary = geometry_summary(&entry.op, contours, ui);
    let side_chosen = &mut entry.side_chosen;
    let kind = match &entry.op {
        Operation::Profile(_) => t!(ui, "forms-kind-profile"),
        Operation::Pocket(_) => t!(ui, "forms-kind-pocket"),
        Operation::Vcarve(_) => t!(ui, "forms-kind-vcarve"),
        Operation::Path(_) => t!(ui, "forms-kind-path"),
        Operation::Holes(_) => t!(ui, "forms-kind-holes"),
        Operation::Surface(_) => t!(ui, "forms-kind-surface"),
    };
    let surface = !entry.op.selects_shapes();
    let color = super::browser::kind_color(&entry.op, ui.visuals().dark_mode);
    super::browser::title_line(
        ui,
        super::browser::op_icon(&entry.op),
        Some(color),
        &format!(
            "{kind} · {}",
            t!(ui, "forms-operation-number", number = index + 1)
        ),
    );
    Grid::new(("op-form", index))
        .num_columns(2)
        // Past the sections' indent, so the values line up with theirs.
        .min_col_width(label_width(ui) + ui.spacing().indent)
        .show(ui, |ui| {
            let name = entry.op.name_mut();
            ui.label(t!(ui, "forms-name"));
            ui.add(TextEdit::singleline(name).desired_width(field_width(ui, 200.0)));
            ui.end_row();
            ui.label(t!(ui, "forms-geometry"));
            if surface {
                // docs/specs/surfacing.md §2.
                ui.label(t!(ui, "forms-whole-stock"))
                    .on_hover_text(t!(ui, "forms-whole-stock-hover"));
                ui.end_row();
                tool_row(ui, index, &mut entry.op, &choice, &mut request);
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
            tool_row(ui, index, &mut entry.op, &choice, &mut request);
            if let Operation::Profile(p) = &mut entry.op {
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
    match &mut entry.op {
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
            section(
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
                        &mut p.clearance,
                        can_clear,
                        through,
                    );
                    ui.label(t!(ui, "forms-step-down"));
                    flag(ui.add(mm(&mut p.step_down)), ui, "step_down");
                    ui.end_row();
                    ramp_angle_row(ui, &mut p.ramp_angle);
                    corner_rows(ui, index, &mut p.corners, can_clear);
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
            section(ui, "op-cut", &t!(ui, "forms-section-cut"), &cut, |ui| {
                ui.label(t!(ui, "forms-direction"));
                direction_combo(ui, ("direction", index), &mut p.direction);
                ui.end_row();
                depth_rows(
                    ui,
                    &mut p.start_depth,
                    &mut p.end_depth,
                    &mut p.clearance,
                    true,
                    through,
                );
                ui.label(t!(ui, "forms-step-down"));
                flag(ui.add(mm(&mut p.step_down)), ui, "step_down");
                ui.end_row();
                corner_rows(ui, index, &mut p.corners, true);
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
            section(ui, "op-clearing", &title, &clearing, |ui| {
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
            feeds_section(ui, &mut p.feed, &mut p.plunge_feed, op_tool);
            let entry_text = entry_summary(ui.lang(), &p.entry, p.lead.as_ref(), units);
            let title = t!(ui, "forms-section-entry-lead");
            section(ui, "op-entry", &title, &entry_text, |ui| {
                entry_rows(ui, index, &mut p.entry, Some(tool_diameter));
                lead_rows(ui, &mut p.lead, tool_diameter);
            });
        }
        Operation::Vcarve(v) => {
            let start = format!("{} {len_unit}", len_in(units, v.start_depth));
            let end = format!("{} {len_unit}", len_in(units, v.end_depth));
            let cut = match v.step_down {
                None => t!(ui, "forms-vcarve-cut-one-pass", start = start, end = end),
                Some(s) => t!(
                    ui,
                    "forms-vcarve-cut-steps",
                    start = start,
                    end = end,
                    step = format!("{} {len_unit}", len_in(units, s))
                ),
            };
            section(ui, "op-cut", &t!(ui, "forms-section-cut"), &cut, |ui| {
                ui.label(t!(ui, "forms-start-depth"))
                    .on_hover_text(t!(ui, "forms-vcarve-start-depth-hover"));
                flag(ui.add(mm(&mut v.start_depth)), ui, "start_depth");
                ui.end_row();
                ui.label(t!(ui, "forms-end-depth"))
                    .on_hover_text(t!(ui, "forms-vcarve-end-depth-hover"));
                flag(ui.add(mm(&mut v.end_depth)), ui, "end_depth");
                ui.end_row();
                ui.label(t!(ui, "forms-passes"));
                let mut stepped = v.step_down.is_some();
                if ui
                    .checkbox(&mut stepped, t!(ui, "forms-in-steps"))
                    .changed()
                {
                    v.step_down = stepped.then_some(DEFAULT_STEP_DOWN);
                }
                ui.end_row();
                if let Some(step) = &mut v.step_down {
                    ui.label(format!("  {}", t!(ui, "forms-step-down-sub")));
                    flag(ui.add(mm(step)), ui, "step_down");
                    ui.end_row();
                }
            });
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
            section(ui, "op-cut", &t!(ui, "forms-section-cut"), &cut, |ui| {
                ui.label(t!(ui, "forms-start-depth"))
                    .on_hover_text(t!(ui, "forms-start-depth-hover"));
                flag(ui.add(mm(&mut p.start_depth)), ui, "start_depth");
                ui.end_row();
                ui.label(t!(ui, "forms-end-depth"))
                    .on_hover_text(t!(ui, "forms-end-depth-hover"));
                flag(ui.add(mm(&mut p.end_depth)), ui, "end_depth");
                ui.end_row();
                ui.label(t!(ui, "forms-step-down"));
                flag(ui.add(mm(&mut p.step_down)), ui, "step_down");
                ui.end_row();
            });
            feeds_section(ui, &mut p.feed, &mut p.plunge_feed, op_tool);
            let entry_text = entry_summary(ui.lang(), &p.entry, None, units);
            let title = t!(ui, "forms-section-entry");
            section(ui, "op-entry", &title, &entry_text, |ui| {
                entry_rows(ui, index, &mut p.entry, None);
            });
        }
        Operation::Holes(h) => holes_sections(ui, index, h, op_tool),
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
            section(ui, "op-cut", &t!(ui, "forms-section-cut"), &cut, |ui| {
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
    }
    request.use_selection = use_selection;
    request
}

/// The operation's tool: one of the project's, or one copied in from the library
/// (docs/specs/tool-per-operation.md §8). Changing it keeps the feeds; when the tool's
/// library cutting values differ from the operation's, a button applies them.
fn tool_row(
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
    let selected = choice
        .tools
        .iter()
        .find(|t| t.id == current)
        .map_or(t!(ui, "forms-tool-not-in-project", tool = current), |t| {
            tool_label(ui, t, units)
        });
    ui.label(t!(ui, "forms-tool"));
    let combo = ComboBox::from_id_salt(("op-tool", index))
        .selected_text(selected)
        .width(field_width(ui, 240.0))
        .truncate()
        .show_ui(ui, |ui| {
            for tool in choice.tools {
                if ui
                    .selectable_label(tool.id == current, tool_label(ui, tool, units))
                    .clicked()
                {
                    op.set_tool(tool.id);
                    crate::editor::fit_hole_method(op, choice.tools);
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
    if let Some(cutting) = choice.cutting
        && !has_cutting_values(op, &cutting)
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
            apply_cutting(op, &cutting);
        }
        ui.end_row();
    }
}

/// Whether the operation already cuts with these feeds, step-down and (pockets) stepover.
fn has_cutting_values(op: &Operation, c: &Cutting) -> bool {
    match op {
        Operation::Profile(p) => {
            (p.feed, p.plunge_feed, p.step_down) == (c.feed, c.plunge_feed, c.step_down)
        }
        Operation::Pocket(p) => {
            (p.feed, p.plunge_feed, p.step_down, p.stepover)
                == (c.feed, c.plunge_feed, c.step_down, c.stepover)
        }
        Operation::Vcarve(v) => {
            (v.feed, v.plunge_feed, v.step_down) == (c.feed, c.plunge_feed, Some(c.step_down))
        }
        Operation::Path(p) => {
            (p.feed, p.plunge_feed, p.step_down) == (c.feed, c.plunge_feed, c.step_down)
        }
        // A hole has no step down in the library's sense: only the feeds.
        Operation::Holes(h) => (h.feed, h.plunge_feed) == (c.feed, c.plunge_feed),
        // A surface keeps its own stepover: the library's is a pocket's.
        Operation::Surface(f) => {
            (f.feed, f.plunge_feed, f.step_down) == (c.feed, c.plunge_feed, c.step_down)
        }
    }
}

fn apply_cutting(op: &mut Operation, c: &Cutting) {
    match op {
        Operation::Profile(p) => {
            (p.feed, p.plunge_feed, p.step_down) = (c.feed, c.plunge_feed, c.step_down);
        }
        Operation::Pocket(p) => {
            (p.feed, p.plunge_feed, p.step_down, p.stepover) =
                (c.feed, c.plunge_feed, c.step_down, c.stepover);
        }
        Operation::Vcarve(v) => {
            (v.feed, v.plunge_feed, v.step_down) = (c.feed, c.plunge_feed, Some(c.step_down));
        }
        Operation::Path(p) => {
            (p.feed, p.plunge_feed, p.step_down) = (c.feed, c.plunge_feed, c.step_down);
        }
        Operation::Holes(h) => (h.feed, h.plunge_feed) = (c.feed, c.plunge_feed),
        Operation::Surface(f) => {
            (f.feed, f.plunge_feed, f.step_down) = (c.feed, c.plunge_feed, c.step_down);
        }
    }
}

/// The feeds, and the chip load next to the feed (docs/specs/small-items.md §5) when the
/// operation's tool is known.
fn feeds_section(ui: &mut egui::Ui, cut: &mut f64, plunge: &mut f64, tool: Option<&Tool>) {
    let units = crate::units::display(ui.ctx());
    let feed_unit = chipload_pipeline::units::feed_unit(units);
    let text = format!(
        "{} / {} {feed_unit}",
        feed_in(units, *cut),
        feed_in(units, *plunge)
    );
    section(
        ui,
        "op-feeds",
        &t!(ui, "forms-section-feeds"),
        &text,
        |ui| {
            ui.label(t!(ui, "forms-feed"));
            // The chip load goes under the feed when the panel is narrow.
            wrapped_cell(ui, |ui| {
                flag(ui.add(feed(cut)), ui, "feed");
                if let Some(tool) = tool {
                    let load = chipload_pipeline::chip_load::chip_load_of(*cut, tool);
                    chip_load_label(ui, load, units);
                }
            });
            ui.end_row();
            ui.label(t!(ui, "forms-plunge-feed"));
            flag(ui.add(feed(plunge)), ui, "plunge_feed");
            ui.end_row();
        },
    );
}

/// How the tool goes down into each pass (docs/specs/ramp-entry.md); angles are shown in
/// degrees and kept in radians. `helix`: the tool diameter, for a pocket, which may also go
/// down round a helix (docs/specs/helix-and-leads.md part C).
fn entry_rows(ui: &mut egui::Ui, index: usize, entry: &mut Entry, helix: Option<f64>) {
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
fn ramp_angle_row(ui: &mut egui::Ui, ramp_angle: &mut f64) {
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
fn rest_row(
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
fn rest_choices<'a>(tools: &'a [Tool], small: Option<&'a Tool>) -> impl Iterator<Item = &'a Tool> {
    tools
        .iter()
        .filter(move |t| small.is_some_and(|small| rest_refusal(small, t).is_none()))
}

/// Square inside corners: dogbones or T-bones (docs/specs/dogbones.md §7). `allowed`: false
/// for a profile on the line, which has no inside corners to clear.
fn corner_rows(ui: &mut egui::Ui, index: usize, corners: &mut Option<Corners>, allowed: bool) {
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
}

/// A Holes operation's cut, method and feeds (docs/specs/holes.md §7): a drill shows its
/// pecks and plunge feed, an endmill the helix.
fn holes_sections(ui: &mut egui::Ui, index: usize, h: &mut HolesOp, tool: Option<&Tool>) {
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
    section(ui, "op-cut", &t!(ui, "forms-section-cut"), &cut, |ui| {
        ui.label(t!(ui, "forms-start-depth"))
            .on_hover_text(t!(ui, "forms-start-depth-hover"));
        flag(ui.add(mm(&mut h.start_depth)), ui, "start_depth");
        ui.end_row();
        let drill = matches!(h.method, HoleMethod::Drill { .. });
        ui.label(t!(ui, "forms-end-depth")).on_hover_text(if drill {
            t!(ui, "forms-holes-drill-end-depth-hover")
        } else {
            t!(ui, "forms-end-depth-hover")
        });
        flag(ui.add(mm(&mut h.end_depth)), ui, "end_depth");
        ui.end_row();
        match &mut h.method {
            HoleMethod::Drill { peck } => {
                ui.label(t!(ui, "forms-pecks"))
                    .on_hover_text(t!(ui, "forms-pecks-hover"));
                let mut on = peck.is_some();
                let check = ui.checkbox(&mut on, t!(ui, "forms-in-pecks"));
                if flag(check, ui, "method.peck").changed() {
                    *peck = on.then_some(DEFAULT_STEP_DOWN);
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
        section(
            ui,
            "op-feeds",
            &t!(ui, "forms-section-feeds"),
            &text,
            |ui| {
                ui.label(t!(ui, "forms-drill-feed"));
                wrapped_cell(ui, |ui| {
                    flag(ui.add(feed(&mut h.plunge_feed)), ui, "plunge_feed");
                    if let Some(tool) = tool {
                        let load = chipload_pipeline::chip_load::chip_load_of(h.plunge_feed, tool);
                        chip_load_label(ui, load, units);
                    }
                });
                ui.end_row();
            },
        );
    } else {
        feeds_section(ui, &mut h.feed, &mut h.plunge_feed, tool);
    }
}

/// How a profile holds its part (docs/specs/holding.md): none, tabs, an onion skin, or
/// both.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Holding {
    None,
    Tabs,
    Skin,
    SkinAndTabs,
}

impl Holding {
    pub(super) const ALL: [Holding; 4] = [
        Holding::None,
        Holding::Tabs,
        Holding::Skin,
        Holding::SkinAndTabs,
    ];

    pub(super) fn of(p: &ProfileOp) -> Holding {
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
    pub(super) fn apply(self, p: &mut ProfileOp, center_cutting: Option<bool>) {
        if !self.has_tabs() {
            p.tabs = None;
        } else if p.tabs.is_none() {
            p.tabs = Some(Tabs {
                count: 4,
                width: 8.0,
                height: 3.0,
                flank_angle: Tabs::default_flank(p.ramp_angle, center_cutting),
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
fn holding_section(ui: &mut egui::Ui, p: &mut ProfileOp, op_tool: Option<&Tool>) {
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
    section(
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
                ui.label(format!("  {}", t!(ui, "forms-tab-width")));
                flag(ui.add(mm(&mut tabs.width)), ui, "tabs.width");
                ui.end_row();
                ui.label(format!("  {}", t!(ui, "forms-tab-height")));
                flag(ui.add(mm(&mut tabs.height)), ui, "tabs.height");
                ui.end_row();
                flank_row(ui, tabs);
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
fn finish_section(ui: &mut egui::Ui, p: &mut ProfileOp) {
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
    section(ui, "op-finish", &title, &summary, |ui| {
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

/// A pocket's lead-in and lead-out (docs/specs/helix-and-leads.md part B): on or off, and
/// the radius, the tool radius by default.
fn lead_rows(ui: &mut egui::Ui, lead: &mut Option<Lead>, tool_diameter: f64) {
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

/// Drawing, stock, machine and tool.
/// The G-code's X0 Y0: a choice, plus X and Y for a point. A point starts at `here`
/// (where the current zero is), so choosing it moves nothing.
pub(super) fn work_zero_rows(
    ui: &mut egui::Ui,
    salt: &str,
    work_zero: &mut WorkZero,
    here: (f64, f64),
) {
    let choices = [
        (
            WorkZero::DrawingOrigin {},
            t!(ui, "forms-zero-drawing-origin"),
        ),
        (
            WorkZero::Stock {
                corner: StockCorner::Center,
            },
            t!(ui, "forms-zero-stock-centre"),
        ),
        (
            WorkZero::Stock {
                corner: StockCorner::BottomLeft,
            },
            t!(ui, "forms-zero-stock-bottom-left"),
        ),
        (
            WorkZero::Stock {
                corner: StockCorner::BottomRight,
            },
            t!(ui, "forms-zero-stock-bottom-right"),
        ),
        (
            WorkZero::Stock {
                corner: StockCorner::TopLeft,
            },
            t!(ui, "forms-zero-stock-top-left"),
        ),
        (
            WorkZero::Stock {
                corner: StockCorner::TopRight,
            },
            t!(ui, "forms-zero-stock-top-right"),
        ),
    ];
    let current = *work_zero;
    let selected = choices
        .iter()
        .find(|(choice, _)| *choice == current)
        .map_or(t!(ui, "forms-zero-point"), |(_, name)| name.clone());
    ui.label(t!(ui, "forms-work-zero"))
        .on_hover_text(t!(ui, "forms-work-zero-hover"));
    ComboBox::from_id_salt((salt, "work_zero"))
        .selected_text(selected)
        .show_ui(ui, |ui| {
            for (choice, name) in choices {
                ui.selectable_value(work_zero, choice, name);
            }
            let (x, y) = here;
            let is_point = matches!(work_zero, WorkZero::Point { .. });
            if ui
                .selectable_label(is_point, t!(ui, "forms-zero-point"))
                .clicked()
                && !is_point
            {
                *work_zero = WorkZero::Point { x, y };
            }
        });
    ui.end_row();
    if let WorkZero::Point { x, y } = work_zero {
        ui.label("");
        ui.horizontal(|ui| {
            ui.add(mm(x).prefix("X "));
            ui.add(mm(y).prefix("Y "));
        });
        ui.end_row();
    }
}

/// What the job form asks of the app beyond editing values.
pub(super) enum JobRequest {
    /// docs/specs/tool-library-and-machines.md §8.
    Library(LibraryRequest),
    /// Open operation i (a rest pass's pocket).
    OpenOperation(usize),
    /// Remove pocket i's rest pass k (docs/specs/rest-passes.md §6).
    RemoveRestPass(usize, usize),
    /// Set part `id`'s pitch so this many mm are left between its copies' cuts
    /// (docs/specs/copies-grid.md decision 2).
    CopiesGap(u64, f64),
    /// Make part `id` the active part (docs/specs/parts.md §8).
    Activate(u64),
    /// The open tool was removed: show the tools instead.
    ToolRemoved,
    /// Write a setup sheet with exported G-code, or not (an app setting).
    /// DXF `id`: update it from a file, or remove it (docs/specs/properties-for-tools-and-
    /// dxfs.md §3).
    UpdateDxf(u64),
    RemoveDxf(u64),
    /// Picture `id`: its window, a new size, place and layer, or removed (§3b).
    TraceSettings(u64),
    ImagePlace(u64, ImagePlace),
    RemoveImage(u64),
    /// Text `id` as its properties left it, or removed (docs/specs/text.md §5).
    TextChange(u64, Box<chipload_job::Text>),
    RemoveText(u64),
    /// Move DXF `id` (docs/specs/drawing-placement.md, drawing-sources.md §2).
    Place(u64, PlaceMove),
    /// Make this the project's post-processor; one of the user's is copied in (again:
    /// updated) (docs/specs/user-posts.md).
    Post(String),
}

/// A picture's size, place and layer, as edited in its properties.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct ImagePlace {
    pub width: f64,
    /// The picture's lower-left corner before it is turned (the file's `x`, `y`).
    pub x: f64,
    pub y: f64,
    pub rotation: f64,
    pub layer: String,
}

/// An imported DXF's properties (docs/specs/properties-for-tools-and-dxfs.md §3): what it
/// is, its placement, and buttons to update or remove it.
pub(super) fn dxf_form(ui: &mut egui::Ui, edit: &EditJob, id: u64) -> Option<JobRequest> {
    let units = crate::units::display(ui.ctx());
    let len_unit = chipload_pipeline::units::length_unit(units);
    let drawing = &edit.settings.drawing;
    let source = drawing.dxfs.iter().find(|d| d.id == id)?;
    let bbox = chipload_pipeline::placement::dxf_box(&drawing.shapes, id);
    let size = bbox.map(|((x0, y0), (x1, y1))| (x1 - x0, y1 - y0));
    let shapes = drawing
        .shapes
        .iter()
        .filter(|s| s.dxf() == Some(id))
        .count();
    let mut request = None;
    super::browser::title_line(ui, crate::icons::Icon::Dxf, None, &source.name);
    Grid::new(("dxf-facts", id))
        .num_columns(2)
        .min_col_width(label_width(ui) + ui.spacing().indent)
        .show(ui, |ui| {
            ui.label(t!(ui, "forms-file"));
            ui.label(&source.file);
            ui.end_row();
            ui.label(t!(ui, "forms-units"));
            ui.label(match source.units {
                Units::Mm => "mm".to_string(),
                Units::Inch => t!(ui, "forms-dxf-inch"),
            });
            ui.end_row();
            ui.label(t!(ui, "forms-shapes"));
            ui.label(shapes.to_string());
            ui.end_row();
        });
    let summary = size.map_or(String::new(), |(w, h)| {
        format!("{} × {} {len_unit}", len_in(units, w), len_in(units, h))
    });
    let title = t!(ui, "forms-section-placement");
    section(ui, "dxf-place", &title, &summary, |ui| {
        placement_rows(ui, id, source.placement, bbox, &mut request);
    });
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        if ui
            .button(t!(ui, "forms-update-dxf"))
            .on_hover_text(t!(ui, "forms-update-dxf-hover"))
            .clicked()
        {
            request = Some(JobRequest::UpdateDxf(id));
        }
        if ui
            .button(t!(ui, "forms-remove"))
            .on_hover_text(t!(ui, "forms-remove-dxf-hover"))
            .clicked()
        {
            request = Some(JobRequest::RemoveDxf(id));
        }
    });
    request
}

/// A picture's properties (§3b): what it is, then its width, corner and layer. Each change
/// applies when the edit is finished, as one undo step, and traces the picture again.
pub(super) fn image_form(ui: &mut egui::Ui, edit: &EditJob, id: u64) -> Option<JobRequest> {
    let units = crate::units::display(ui.ctx());
    let len_unit = chipload_pipeline::units::length_unit(units);
    let image = edit.settings.drawing.images.iter().find(|i| i.id == id)?;
    let mut request = None;
    super::browser::title_line(ui, crate::icons::Icon::Image, None, &image.file);
    // The fields edit a copy kept between frames; it is sent once the edit is finished.
    let key = egui::Id::new(("image-place", id));
    let stored = ImagePlace {
        width: image.width,
        x: image.x,
        y: image.y,
        rotation: image.rotation,
        layer: image.layer.clone(),
    };
    let mut draft: ImagePlace = ui
        .data(|d| d.get_temp::<(ImagePlace, ImagePlace)>(key))
        .filter(|(from, _)| *from == stored)
        .map_or_else(|| stored.clone(), |(_, draft)| draft);
    // The picture's size in pixels, decoded once per picture (not every frame).
    let size_key = egui::Id::new(("image-pixels", id, image.sha256.clone()));
    let pixels: Option<(u32, u32)> = match ui.data(|d| d.get_temp::<Option<(u32, u32)>>(size_key)) {
        Some(known) => known,
        None => {
            let decoded = chipload_pipeline::images::picture_bytes(image)
                .ok()
                .and_then(|bytes| chipload_pipeline::images::decode_picture(&bytes).ok())
                .map(|p| (p.width, p.height));
            ui.data_mut(|d| d.insert_temp(size_key, decoded));
            decoded
        }
    };
    let mut finished = false;
    Grid::new(("image-form", id))
        .num_columns(2)
        .min_col_width(label_width(ui) + ui.spacing().indent)
        .show(ui, |ui| {
            if let Some((w, h)) = pixels {
                ui.label(t!(ui, "forms-picture"));
                ui.label(t!(ui, "forms-picture-pixels", width = w, height = h));
                ui.end_row();
            }
            ui.label(t!(ui, "forms-traced-as"));
            ui.label(match image.trace {
                ImageTrace::Outline => t!(ui, "forms-traced-outlines"),
                ImageTrace::CentreLine => t!(ui, "forms-traced-centre-lines"),
            });
            ui.end_row();
            // The picture's height follows from its width; its box is turned with it
            // (docs/specs/direct-placement.md §2).
            let aspect = pixels
                .filter(|(w, _)| *w > 0)
                .map_or(1.0, |(w, h)| f64::from(h) / f64::from(w));
            let turned = |d: &ImagePlace| {
                chipload_pipeline::images::turned_box(
                    d.x,
                    d.y,
                    d.width,
                    d.width * aspect,
                    d.rotation,
                )
            };
            let ((x0, y0), _) = turned(&draft);
            ui.label(t!(ui, "forms-position")).on_hover_text(t!(
                ui,
                "units-text-picture-position-hover",
                unit = crate::units::unit(ui.ctx())
            ));
            ui.horizontal(|ui| {
                let (mut cx, mut cy) = (x0, y0);
                for (value, axis) in [(&mut cx, "X "), (&mut cy, "Y ")] {
                    let response = ui.add(mm_field(value).prefix(axis));
                    finished |= response.drag_stopped() || response.lost_focus();
                }
                // Moving the box's corner moves the picture's own corner as much.
                draft.x += cx - x0;
                draft.y += cy - y0;
                let hover = t!(ui, "forms-picture-reset-position");
                if reset_button(ui, (x0, y0) != (0.0, 0.0), &hover) {
                    draft.x -= x0;
                    draft.y -= y0;
                    finished = true;
                }
            });
            ui.end_row();
            ui.label(t!(ui, "forms-rotation"))
                .on_hover_text(t!(ui, "forms-picture-rotation-hover"));
            ui.horizontal(|ui| {
                // Turning keeps the picture's centre where it is: x, y stay.
                let response = ui.add(
                    crate::units::number(ui, &mut draft.rotation)
                        .speed(1.0)
                        .suffix("°")
                        .max_decimals(3),
                );
                finished |= response.drag_stopped() || response.lost_focus();
                let mut turn = |by: f64| {
                    draft.rotation += by;
                    finished = true;
                };
                if ui
                    .small_button("⟲ 90")
                    .on_hover_text(t!(ui, "forms-turn-ccw-hover"))
                    .clicked()
                {
                    turn(90.0);
                }
                if ui
                    .small_button("⟳ 90")
                    .on_hover_text(t!(ui, "forms-turn-cw-hover"))
                    .clicked()
                {
                    turn(-90.0);
                }
                let hover = t!(ui, "forms-picture-reset-rotation");
                if reset_button(ui, draft.rotation != 0.0, &hover) {
                    draft.rotation = 0.0;
                    finished = true;
                }
                draft.rotation = draft.rotation.rem_euclid(360.0);
                if draft.rotation >= 360.0 {
                    draft.rotation = 0.0;
                }
            });
            ui.end_row();
            ui.label(t!(ui, "forms-width"))
                .on_hover_text(t!(ui, "forms-picture-width-hover"));
            ui.horizontal(|ui| {
                let response = ui.add(
                    crate::units::length(&mut draft.width)
                        .range(0.1..=10_000.0)
                        .speed(0.5),
                );
                finished |= response.drag_stopped() || response.lost_focus();
                if pixels.is_some() {
                    let height = format!("{} {len_unit}", len_in(units, draft.width * aspect));
                    ui.weak(t!(ui, "forms-picture-height", height = height));
                }
            });
            ui.end_row();
            ui.label(t!(ui, "forms-layer"))
                .on_hover_text(t!(ui, "forms-picture-layer-hover"));
            let response = ui
                .add(TextEdit::singleline(&mut draft.layer).desired_width(field_width(ui, 160.0)));
            finished |= response.lost_focus();
            ui.end_row();
        });
    if finished && draft != stored {
        request = Some(JobRequest::ImagePlace(id, draft.clone()));
    }
    ui.data_mut(|d| d.insert_temp(key, (stored, draft)));
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        if ui
            .button(t!(ui, "forms-trace-settings"))
            .on_hover_text(t!(ui, "forms-trace-settings-hover"))
            .clicked()
        {
            request = Some(JobRequest::TraceSettings(id));
        }
        if ui
            .button(t!(ui, "forms-remove"))
            .on_hover_text(t!(ui, "forms-remove-picture-hover"))
            .clicked()
        {
            request = Some(JobRequest::RemoveImage(id));
        }
    });
    request
}

/// A text's properties (docs/specs/text.md §5): what it says, its font and size, its place
/// and turn, how its lines line up, its layer. Each change applies when the edit is
/// finished, as one undo step, and writes the letters again. `grab`: the text field takes
/// the keyboard (a text just placed).
pub(super) fn text_form(
    ui: &mut egui::Ui,
    edit: &EditJob,
    id: u64,
    grab: bool,
) -> Option<JobRequest> {
    let units = crate::units::display(ui.ctx());
    let len_unit = chipload_pipeline::units::length_unit(units);
    use chipload_job::{Text, TextAlign, TextAnchor};
    let text = edit.settings.drawing.texts.iter().find(|t| t.id == id)?;
    let mut request = None;
    super::browser::title_line(
        ui,
        crate::icons::Icon::Text,
        None,
        &chipload_pipeline::texts::text_label(text),
    );
    // The fields edit a copy kept between frames; it is sent once the edit is finished.
    let key = egui::Id::new(("text-draft", id));
    let stored = text.clone();
    let mut draft: Text = ui
        .data(|d| d.get_temp::<(Text, Text)>(key))
        .filter(|(from, _)| *from == stored)
        .map_or_else(|| stored.clone(), |(_, draft)| draft);
    // The box's size before its turn, worked out once per version of the text.
    let size_key = egui::Id::new(("text-size", id));
    let size = match ui
        .data(|d| d.get_temp::<(Text, Option<(f64, f64)>)>(size_key))
        .filter(|(t, _)| *t == stored)
    {
        Some((_, size)) => size,
        None => {
            let size = chipload_pipeline::texts::text_size(&stored).ok();
            ui.data_mut(|d| d.insert_temp(size_key, (stored.clone(), size)));
            size
        }
    };
    let mut finished = false;
    Grid::new(("text-form", id))
        .num_columns(2)
        .min_col_width(label_width(ui) + ui.spacing().indent)
        .show(ui, |ui| {
            ui.label(t!(ui, "forms-text"))
                .on_hover_text(t!(ui, "forms-text-hover"));
            let response = ui.add(
                TextEdit::multiline(&mut draft.text)
                    .desired_rows(2)
                    .desired_width(field_width(ui, 220.0)),
            );
            if grab {
                response.request_focus();
            }
            finished |= response.lost_focus();
            ui.end_row();
            ui.label(t!(ui, "forms-font"))
                .on_hover_text(t!(ui, "forms-font-hover"));
            let before = draft.font.clone();
            egui::ComboBox::from_id_salt(("text-font", id))
                .selected_text(draft.font.as_str())
                .height(480.0)
                .show_ui(ui, |ui| font_list(ui, &mut draft.font));
            finished |= draft.font != before;
            ui.end_row();
            ui.label(t!(ui, "forms-height"))
                .on_hover_text(t!(ui, "forms-text-height-hover"));
            let response = ui.add(
                crate::units::length(&mut draft.height)
                    .range(0.1..=chipload_job::TEXT_MAX_HEIGHT_MM)
                    .speed(0.1),
            );
            finished |= response.drag_stopped() || response.lost_focus();
            ui.end_row();
            let (w, h) = size.unwrap_or((0.0, 0.0));
            ui.label(t!(ui, "forms-anchor"))
                .on_hover_text(t!(ui, "forms-anchor-hover"));
            let before = draft.anchor;
            let mut anchor = draft.anchor;
            egui::ComboBox::from_id_salt(("text-anchor", id))
                .selected_text(match anchor {
                    TextAnchor::Corner => t!(ui, "forms-anchor-corner"),
                    TextAnchor::Center => t!(ui, "forms-anchor-centre"),
                })
                .show_ui(ui, |ui| {
                    let corner = t!(ui, "forms-anchor-corner");
                    ui.selectable_value(&mut anchor, TextAnchor::Corner, corner);
                    let centre = t!(ui, "forms-anchor-centre");
                    ui.selectable_value(&mut anchor, TextAnchor::Center, centre);
                });
            if anchor != before {
                // The text stays where it is: only what Position means changes.
                draft = chipload_pipeline::texts::with_anchor(&draft, anchor, (w, h));
                finished = true;
            }
            ui.end_row();
            // Shown as the box's lower-left corner on the stock (after the turn), or its
            // centre, which the turn keeps in place.
            let shown = |d: &Text| match d.anchor {
                TextAnchor::Center => (d.x, d.y),
                TextAnchor::Corner => {
                    chipload_pipeline::images::turned_box(d.x, d.y, w, h, d.rotation).0
                }
            };
            let (x0, y0) = shown(&draft);
            ui.label(t!(ui, "forms-position"))
                .on_hover_text(match draft.anchor {
                    TextAnchor::Corner => t!(
                        ui,
                        "units-text-text-position-corner-hover",
                        unit = crate::units::unit(ui.ctx())
                    ),
                    TextAnchor::Center => t!(
                        ui,
                        "units-text-text-position-centre-hover",
                        unit = crate::units::unit(ui.ctx())
                    ),
                });
            ui.horizontal(|ui| {
                let (mut cx, mut cy) = (x0, y0);
                for (value, axis) in [(&mut cx, "X "), (&mut cy, "Y ")] {
                    let response = ui.add(mm_field(value).prefix(axis));
                    finished |= response.drag_stopped() || response.lost_focus();
                }
                // Moving what is shown moves the text's own position as much.
                draft.x += cx - x0;
                draft.y += cy - y0;
                let hover = t!(ui, "forms-text-reset-position");
                if reset_button(ui, (x0, y0) != (0.0, 0.0), &hover) {
                    draft.x -= x0;
                    draft.y -= y0;
                    finished = true;
                }
            });
            ui.end_row();
            ui.label(t!(ui, "forms-rotation"))
                .on_hover_text(t!(ui, "forms-text-rotation-hover"));
            ui.horizontal(|ui| {
                let response = ui.add(
                    crate::units::number(ui, &mut draft.rotation)
                        .speed(1.0)
                        .suffix("°")
                        .max_decimals(3),
                );
                finished |= response.drag_stopped() || response.lost_focus();
                let mut turn = |by: f64| {
                    draft.rotation += by;
                    finished = true;
                };
                if ui
                    .small_button("⟲ 90")
                    .on_hover_text(t!(ui, "forms-turn-ccw-hover"))
                    .clicked()
                {
                    turn(90.0);
                }
                if ui
                    .small_button("⟳ 90")
                    .on_hover_text(t!(ui, "forms-turn-cw-hover"))
                    .clicked()
                {
                    turn(-90.0);
                }
                let hover = t!(ui, "forms-text-reset-rotation");
                if reset_button(ui, draft.rotation != 0.0, &hover) {
                    draft.rotation = 0.0;
                    finished = true;
                }
                draft.rotation = draft.rotation.rem_euclid(360.0);
                if draft.rotation >= 360.0 {
                    draft.rotation = 0.0;
                }
            });
            ui.end_row();
            ui.label(t!(ui, "forms-align"))
                .on_hover_text(t!(ui, "forms-align-hover"));
            let before = draft.align;
            let align_name = |ui: &egui::Ui, align: TextAlign| match align {
                TextAlign::Left => t!(ui, "forms-align-left"),
                TextAlign::Center => t!(ui, "forms-align-centre"),
                TextAlign::Right => t!(ui, "forms-align-right"),
            };
            egui::ComboBox::from_id_salt(("text-align", id))
                .selected_text(align_name(ui, draft.align))
                .show_ui(ui, |ui| {
                    for align in [TextAlign::Left, TextAlign::Center, TextAlign::Right] {
                        ui.selectable_value(&mut draft.align, align, align_name(ui, align));
                    }
                });
            finished |= draft.align != before;
            ui.end_row();
            ui.label(t!(ui, "forms-letter-spacing"))
                .on_hover_text(t!(ui, "forms-letter-spacing-hover"));
            let most = chipload_job::TEXT_MAX_SPACING_HEIGHTS * draft.height;
            let response = ui.add(
                crate::units::length(&mut draft.spacing)
                    .range(-most..=most)
                    .speed(0.05),
            );
            finished |= response.drag_stopped() || response.lost_focus();
            ui.end_row();
            ui.label(t!(ui, "forms-line-spacing"))
                .on_hover_text(t!(ui, "forms-line-spacing-hover"));
            let response = ui.add(
                crate::units::number(ui, &mut draft.line_spacing)
                    .range(0.1..=chipload_job::TEXT_MAX_LINE_SPACING)
                    .speed(0.01)
                    .suffix(format!(" {}", t!(ui, "forms-line-spacing-suffix"))),
            );
            finished |= response.drag_stopped() || response.lost_focus();
            ui.end_row();
            ui.label(t!(ui, "forms-layer"))
                .on_hover_text(t!(ui, "forms-text-layer-hover"));
            let response = ui
                .add(TextEdit::singleline(&mut draft.layer).desired_width(field_width(ui, 160.0)));
            finished |= response.lost_focus();
            ui.end_row();
            if let Some((w, h)) = size {
                ui.label(t!(ui, "forms-size"));
                ui.weak(format!(
                    "{} × {} {len_unit}",
                    len_in(units, w),
                    len_in(units, h)
                ));
                ui.end_row();
            }
        });
    if finished && draft != stored {
        request = Some(JobRequest::TextChange(id, Box::new(draft.clone())));
    }
    ui.data_mut(|d| d.insert_temp(key, (stored, draft)));
    ui.add_space(4.0);
    if ui
        .button(t!(ui, "forms-remove"))
        .on_hover_text(t!(ui, "forms-remove-text-hover"))
        .clicked()
    {
        request = Some(JobRequest::RemoveText(id));
    }
    request
}

/// Gap the *Set pitch* helper starts with, mm; only an input, never saved.
const DEFAULT_COPIES_GAP: f64 = 10.0;

/// The job settings: stock, machine and post. The setup sheet is in Preferences
/// (docs/specs/this-computer.md §3).
pub(super) fn job_form(
    ui: &mut egui::Ui,
    edit: &mut EditJob,
    library: Option<&Library>,
) -> Option<JobRequest> {
    let units = crate::units::display(ui.ctx());
    let len_unit = chipload_pipeline::units::length_unit(units);
    let mut request = None;
    let mut copies_request = None;
    let job = &mut edit.settings;
    let title = t!(ui, "forms-job-settings");
    super::browser::title_line(ui, crate::icons::Icon::Settings, None, &title);

    let stock = format!(
        "{} × {} × {} {len_unit}",
        len_in(units, job.stock.x),
        len_in(units, job.stock.y),
        len_in(units, job.stock.thickness)
    );
    section(
        ui,
        "job-stock",
        &t!(ui, "forms-section-stock"),
        &stock,
        |ui| {
            ui.label(t!(ui, "forms-stock-size"));
            ui.horizontal(|ui| {
                flag(ui.add(mm(&mut job.stock.x)), ui, "stock.x");
                flag(ui.add(mm(&mut job.stock.y)), ui, "stock.y");
            });
            ui.end_row();
            ui.label(t!(ui, "forms-thickness"));
            flag(ui.add(mm(&mut job.stock.thickness)), ui, "stock.thickness");
            ui.end_row();
            ui.label(t!(ui, "forms-max-overcut"))
                .on_hover_text(t!(ui, "forms-max-overcut-hover"));
            flag(
                ui.add(mm(&mut job.stock.max_overcut)),
                ui,
                "stock.max_overcut",
            );
            ui.end_row();
            // Where the stock lies relative to the drawing (file: stock.origin). Not the G-code
            // zero: that's the work zero below (docs/specs/work-zero.md).
            ui.label(t!(ui, "forms-stock-position"));
            let origin_name = |ui: &egui::Ui, origin: StockOrigin| match origin {
                StockOrigin::BottomLeft => t!(ui, "forms-stock-corner-at-origin"),
                StockOrigin::Center => t!(ui, "forms-stock-centred-on-origin"),
            };
            ComboBox::from_id_salt("origin")
                .selected_text(origin_name(ui, job.stock.origin))
                .show_ui(ui, |ui| {
                    for origin in [StockOrigin::BottomLeft, StockOrigin::Center] {
                        ui.selectable_value(&mut job.stock.origin, origin, origin_name(ui, origin));
                    }
                });
            ui.end_row();
            let here = job.work_zero_xy();
            work_zero_rows(ui, "job", &mut job.machine.work_zero, here);
            ui.label(t!(ui, "forms-z-zero"));
            let z_name = |ui: &egui::Ui, z: ZZero| match z {
                ZZero::Top => t!(ui, "forms-z-zero-top"),
                ZZero::Bottom => t!(ui, "forms-z-zero-bottom"),
            };
            ComboBox::from_id_salt("z_zero")
                .selected_text(z_name(ui, job.stock.z_zero))
                .show_ui(ui, |ui| {
                    for z in [ZZero::Top, ZZero::Bottom] {
                        ui.selectable_value(&mut job.stock.z_zero, z, z_name(ui, z));
                    }
                });
            ui.end_row();
        },
    );

    let machine = t!(
        ui,
        "forms-machine-summary",
        post = job.machine.post.as_str(),
        safe_z = format!("{} {len_unit}", len_in(units, job.machine.safe_z)),
        spindle = match job.machine.spindle {
            Spindle::Cw => "M3",
            Spindle::Ccw => "M4",
        }
    );
    // A machine linked to a profile in the library, and whether it is the same
    // (docs/specs/linked-machine.md §3).
    let profile = library.and_then(|l| {
        let uid = job.machine.library_uid.as_deref()?;
        l.machines.find_uid(uid).cloned()
    });
    let same = profile
        .as_ref()
        .is_some_and(|p| p.machine_changes(&job.machine).is_empty());
    let edit_here = egui::Id::new("job-machine-edit-here");
    let editing = ui.data(|d| d.get_temp::<bool>(edit_here).unwrap_or(false));
    section(
        ui,
        "job-machine",
        &t!(ui, "forms-section-machine"),
        &machine,
        |ui| {
            match &profile {
                Some(p) if same && !editing => {
                    ui.label(t!(ui, "forms-from-machine"));
                    ui.horizontal(|ui| {
                        ui.strong(&p.name);
                        if ui
                            .button(t!(ui, "forms-edit-here"))
                            .on_hover_text(t!(ui, "forms-edit-here-hover"))
                            .clicked()
                        {
                            ui.data_mut(|d| d.insert_temp(edit_here, true));
                        }
                    });
                    ui.end_row();
                    machine_text_rows(ui, &job.machine);
                }
                _ => {
                    if let Some(post) =
                        machine_rows(ui, "job", &mut job.machine, library, profile.as_ref())
                    {
                        copies_request = Some(JobRequest::Post(post));
                    }
                }
            }
            safe_z_row(ui, &t!(ui, "forms-safe-z-job"), &mut job.machine);
            post_copy_row(ui, &job.machine, library, &mut copies_request);
            library_rows(ui, &mut request, LibraryRows::machine(job, library));
        },
    );

    copies_request.or(request.map(JobRequest::Library))
}

/// "T2 endmill Ø3 mm "3 mm downcut"": a project tool as lists show it.
pub(super) fn tool_label(lang: &impl crate::i18n::Lang, tool: &Tool, units: Units) -> String {
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
fn tool_users(edit: &EditJob, id: i64) -> Vec<String> {
    edit.ops
        .iter()
        .enumerate()
        .filter(|(_, e)| e.op.tool() == id)
        .map(|(i, _)| (i + 1).to_string())
        .collect()
}

/// The project's tools, each in a section of its own, and buttons to add one
/// (docs/specs/tool-per-operation.md §8, properties-for-tools-and-dxfs.md §2). A tool an
/// operation uses can't be removed.
pub(super) fn tools_form(
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
            let job = &mut edit.settings;
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
    for index in 0..edit.settings.tools.len() {
        let id = edit.settings.tools[index].id;
        let users = tool_users(edit, id);
        let summary = match users.len() {
            0 => t!(ui, "forms-tool-not-used"),
            _ => t!(ui, "forms-tool-used-by", operations = users.join(", ")),
        };
        let title = tool_label(
            ui,
            &edit.settings.tools[index],
            crate::units::display(ui.ctx()),
        );
        let at = format!("tools.{index}");
        let marked = problems::has_problem_under(ui, &at);
        let tool = &mut edit.settings.tools[index];
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
        edit.settings.tools.remove(index);
    }
    request.map(JobRequest::Library)
}

/// One tool alone (docs/specs/properties-for-tools-and-dxfs.md §2).
pub(super) fn tool_form(
    ui: &mut egui::Ui,
    edit: &mut EditJob,
    library: Option<&Library>,
    id: i64,
) -> Option<JobRequest> {
    let index = edit.settings.tools.iter().position(|t| t.id == id)?;
    let users = tool_users(edit, id);
    let mut request = None;
    let icon = match edit.settings.tools[index].kind {
        ToolType::Endmill => crate::icons::Icon::Endmill,
        ToolType::Drill => crate::icons::Icon::Drill,
        ToolType::Vbit => crate::icons::Icon::Vbit,
    };
    super::browser::title_line(
        ui,
        icon,
        None,
        &tool_label(
            ui,
            &edit.settings.tools[index],
            crate::units::display(ui.ctx()),
        ),
    );
    ui.weak(match users.len() {
        0 => t!(ui, "forms-tool-not-used-long"),
        _ => t!(ui, "forms-tool-cuts", operations = users.join(", ")),
    });
    let at = format!("tools.{index}");
    let tool = &mut edit.settings.tools[index];
    let mut removed = false;
    Grid::new(("tool-form", id))
        .num_columns(2)
        .min_col_width(label_width(ui))
        .show(ui, |ui| {
            removed = tool_body(ui, tool, &at, &users, library, &mut request);
        });
    if removed {
        edit.settings.tools.remove(index);
        return Some(JobRequest::ToolRemoved);
    }
    request.map(JobRequest::Library)
}

/// A tool's rows: its library line, its values and *Remove tool*. True when *Remove* was
/// pressed.
fn tool_body(
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

/// Margin the *Bottom left* button leaves, mm.
const PLACE_MARGIN: f64 = 10.0;

/// A number field whose value is sent once the edit is finished (Enter, Tab, a click
/// elsewhere or a drag released): the new value, if it differs from `current`. While it is
/// edited, the typed value is kept between frames.
fn finished_number(
    ui: &mut egui::Ui,
    id: egui::Id,
    current: f64,
    field: impl FnOnce(&mut egui::Ui, &mut f64) -> egui::Response,
) -> Option<f64> {
    let mut value = ui.data(|d| d.get_temp::<f64>(id)).unwrap_or(current);
    let response = field(ui, &mut value);
    if response.has_focus() || response.dragged() {
        ui.data_mut(|d| d.insert_temp(id, value));
        return None;
    }
    ui.data_mut(|d| d.remove::<f64>(id));
    let done = response.lost_focus() || response.drag_stopped() || response.changed();
    (done && value != current).then_some(value)
}

/// A position field: half a millimetre per pixel dragged.
fn mm_field(value: &mut f64) -> crate::units::Field<'_> {
    crate::units::length(value).speed(0.5)
}

/// A length (or feed) in mm as a number in `units`, for reading.
fn len_in(units: Units, mm: f64) -> String {
    chipload_pipeline::units::length_number(mm, units)
}

/// A feed in mm/min as a number in `units` per minute, for reading.
fn feed_in(units: Units, mm_per_min: f64) -> String {
    chipload_pipeline::units::feed_number(mm_per_min, units)
}

/// A small reset button after a row; true when clicked.
fn reset_button(ui: &mut egui::Ui, enabled: bool, hover: &str) -> bool {
    let label = t!(ui, "forms-reset");
    super::header::icon_button(ui, crate::icons::Icon::Undo, &label, enabled, false, 18.0)
        .on_hover_text(hover)
        .clicked()
}

/// A DXF's position, rotation and scale (docs/specs/direct-placement.md §2). Position is
/// the lower-left corner of its shapes' box; rotation and scale are from the DXF as drawn
/// and change about the box's centre. Each change is one undo step.
pub(super) fn placement_rows(
    ui: &mut egui::Ui,
    dxf: u64,
    placement: Placement,
    bbox: Option<((f64, f64), (f64, f64))>,
    request: &mut Option<JobRequest>,
) {
    let units = crate::units::display(ui.ctx());
    let len_unit = chipload_pipeline::units::length_unit(units);
    let mut place = |how: PlaceMove| *request = Some(JobRequest::Place(dxf, how));
    let salt = |what: &str| egui::Id::new(("dxf-place", dxf, what.to_string()));
    if let Some(((x0, y0), _)) = bbox {
        ui.label(t!(ui, "forms-position")).on_hover_text(t!(
            ui,
            "units-text-dxf-position-hover",
            unit = crate::units::unit(ui.ctx())
        ));
        ui.horizontal(|ui| {
            if let Some(x) =
                finished_number(ui, salt("x"), x0, |ui, v| ui.add(mm_field(v).prefix("X ")))
            {
                place(PlaceMove::Corner(x, y0));
            }
            if let Some(y) =
                finished_number(ui, salt("y"), y0, |ui, v| ui.add(mm_field(v).prefix("Y ")))
            {
                place(PlaceMove::Corner(x0, y));
            }
            let moved = (placement.dx, placement.dy) != (0.0, 0.0);
            let hover = t!(ui, "forms-dxf-reset-position");
            if reset_button(ui, moved, &hover) {
                place(PlaceMove::To(0.0, 0.0));
            }
        });
        ui.end_row();
        ui.label("");
        ui.horizontal(|ui| {
            if ui
                .small_button(t!(ui, "forms-bottom-left"))
                .on_hover_text(t!(
                    ui,
                    "forms-bottom-left-hover",
                    margin = format!("{PLACE_MARGIN} mm")
                ))
                .clicked()
            {
                place(PlaceMove::BottomLeft {
                    margin: PLACE_MARGIN,
                });
            }
            if ui
                .small_button(t!(ui, "forms-centre"))
                .on_hover_text(t!(ui, "forms-centre-hover"))
                .clicked()
            {
                place(PlaceMove::Center);
            }
        });
        ui.end_row();
    }
    ui.label(t!(ui, "forms-rotation"))
        .on_hover_text(t!(ui, "forms-dxf-rotation-hover"));
    ui.horizontal(|ui| {
        let current = placement.rotation;
        if let Some(degrees) = finished_number(ui, salt("rotation"), current, |ui, v| {
            ui.add(
                crate::units::number(ui, v)
                    .speed(1.0)
                    .suffix("°")
                    .max_decimals(3),
            )
        }) {
            place(PlaceMove::Rotate(degrees - current));
        }
        if ui
            .small_button("⟲ 90")
            .on_hover_text(t!(ui, "forms-turn-ccw-hover"))
            .clicked()
        {
            place(PlaceMove::Rotate(90.0));
        }
        if ui
            .small_button("⟳ 90")
            .on_hover_text(t!(ui, "forms-turn-cw-hover"))
            .clicked()
        {
            place(PlaceMove::Rotate(-90.0));
        }
        let hover = t!(ui, "forms-dxf-reset-rotation");
        if reset_button(ui, current != 0.0, &hover) {
            place(PlaceMove::Rotate(-current));
        }
    });
    ui.end_row();
    ui.label(t!(ui, "forms-scale"))
        .on_hover_text(t!(ui, "forms-scale-hover"));
    ui.horizontal(|ui| {
        let current = placement.scale;
        if let Some(scale) = finished_number(ui, salt("scale"), current, |ui, v| {
            ui.add(
                crate::units::number(ui, v)
                    .speed(0.01)
                    .range(Placement::MIN_SCALE..=Placement::MAX_SCALE)
                    .max_decimals(6)
                    .prefix("× "),
            )
        }) {
            place(PlaceMove::Scale(scale / current));
        }
        let hover = t!(ui, "forms-dxf-reset-scale");
        if reset_button(ui, current != 1.0, &hover) {
            place(PlaceMove::Scale(1.0 / current));
        }
    });
    ui.end_row();
    if let Some(((x0, y0), (x1, y1))) = bbox {
        ui.label(t!(ui, "forms-size"));
        ui.label(format!(
            "{} × {} {len_unit}",
            len_in(units, x1 - x0),
            len_in(units, y1 - y0)
        ));
        ui.end_row();
    }
}

/// A part's properties (docs/specs/parts.md §8): its name, on or off, whether it is the
/// active part, and its copies.
pub(super) fn part_form(ui: &mut egui::Ui, edit: &mut EditJob, id: u64) -> Option<JobRequest> {
    let mut request = None;
    let active = edit.settings.active_part_id() == Some(id);
    let numbers: Vec<String> = (0..edit.ops.len())
        .filter(|i| edit.ops[*i].op.part() == id)
        .map(|i| (i + 1).to_string())
        .collect();
    let index = edit.settings.parts.iter().position(|p| p.id == id)?;
    let several = edit.settings.parts.len() > 1;
    let part = &mut edit.settings.parts[index];
    super::browser::title_line(
        ui,
        crate::icons::Icon::Part,
        None,
        &format!(
            "{} · {}",
            t!(ui, "forms-part-number", number = index + 1),
            part.name
        ),
    );
    let at = |field: &str| format!("parts.{index}.{field}");
    Grid::new(("part-form", id))
        .num_columns(2)
        .min_col_width(label_width(ui) + ui.spacing().indent)
        .show(ui, |ui| {
            ui.label(t!(ui, "forms-name"));
            let response =
                ui.add(TextEdit::singleline(&mut part.name).desired_width(field_width(ui, 200.0)));
            flag(response, ui, &at("name"));
            ui.end_row();
            ui.label(t!(ui, "forms-part-cut"))
                .on_hover_text(t!(ui, "forms-part-cut-hover"));
            ui.checkbox(&mut part.enabled, t!(ui, "forms-part-cut-check"));
            ui.end_row();
            ui.label(t!(ui, "forms-active"))
                .on_hover_text(t!(ui, "forms-active-hover"));
            if active {
                ui.label(t!(ui, "forms-active-yes"));
            } else if ui.button(t!(ui, "forms-activate")).clicked() {
                request = Some(JobRequest::Activate(id));
            }
            ui.end_row();
            ui.label(t!(ui, "forms-operations"));
            ui.label(if numbers.is_empty() {
                t!(ui, "forms-operations-none")
            } else {
                numbers.join(", ")
            });
            ui.end_row();
        });
    let copies = part.copies.map_or(t!(ui, "forms-copies-one"), |c| {
        format!("{} × {} = {}", c.columns, c.rows, c.count())
    });
    section(ui, "part-copies", &t!(ui, "forms-copies"), &copies, |ui| {
        copies_rows(ui, &mut part.copies, &mut request, &at("copies"), id);
    });
    if several {
        ui.add_space(4.0);
        ui.weak(t!(ui, "forms-parts-note"));
    }
    request
}

/// A part cut several times in a grid (docs/specs/copies-grid.md); `at` is the copies'
/// path, for the problems at its fields.
fn copies_rows(
    ui: &mut egui::Ui,
    copies: &mut Option<Copies>,
    request: &mut Option<JobRequest>,
    at: &str,
    part: u64,
) {
    ui.label(t!(ui, "forms-copies"))
        .on_hover_text(t!(ui, "forms-copies-hover"));
    let mut on = copies.is_some();
    if ui.checkbox(&mut on, t!(ui, "forms-copies-grid")).changed() {
        *copies = on.then_some(Copies {
            columns: 2,
            rows: 1,
            dx: 0.0,
            dy: 0.0,
            order: CopyOrder::ByOperation,
        });
        if on {
            *request = Some(JobRequest::CopiesGap(part, DEFAULT_COPIES_GAP));
        }
    }
    ui.end_row();
    let Some(grid) = copies else {
        return;
    };
    ui.label(format!("  {}", t!(ui, "forms-columns-rows")));
    ui.horizontal(|ui| {
        let count = 1..=MAX_COPIES_PER_SIDE;
        flag(
            ui.add(crate::units::number(ui, &mut grid.columns).range(count.clone())),
            ui,
            &format!("{at}.columns"),
        );
        flag(
            ui.add(crate::units::number(ui, &mut grid.rows).range(count)),
            ui,
            &format!("{at}.rows"),
        );
        ui.label(format!("= {}", grid.count()));
    });
    ui.end_row();
    ui.label(format!("  {}", t!(ui, "forms-pitch")))
        .on_hover_text(t!(
            ui,
            "units-text-pitch-hover",
            unit = crate::units::unit(ui.ctx())
        ));
    ui.horizontal(|ui| {
        for (name, value) in [("dx", &mut grid.dx), ("dy", &mut grid.dy)] {
            let response = ui.add(
                crate::units::length(value)
                    .speed(0.1)
                    .range(-100_000.0..=100_000.0),
            );
            flag(response, ui, &format!("{at}.{name}"));
        }
    });
    ui.end_row();
    ui.label(format!("  {}", t!(ui, "forms-gap")))
        .on_hover_text(t!(ui, "forms-gap-hover"));
    ui.horizontal(|ui| {
        let id = egui::Id::new(("copies-gap", part));
        let mut gap = ui.data_mut(|d| *d.get_temp_mut_or(id, DEFAULT_COPIES_GAP));
        ui.add(mm(&mut gap));
        ui.data_mut(|d| d.insert_temp(id, gap));
        if ui.button(t!(ui, "forms-set-pitch")).clicked() {
            *request = Some(JobRequest::CopiesGap(part, gap));
        }
    });
    ui.end_row();
    ui.label(format!("  {}", t!(ui, "forms-order")));
    ComboBox::from_id_salt("copies-order")
        .selected_text(order_name(ui, grid.order))
        .show_ui(ui, |ui| {
            for order in [CopyOrder::ByOperation, CopyOrder::ByCopy] {
                let name = order_name(ui, order);
                ui.selectable_value(&mut grid.order, order, name);
            }
        });
    ui.end_row();
}

/// A tool's type, in the app's language: `endmill`, `drill`, `V-bit`.
pub(super) fn tool_kind_name(lang: &impl crate::i18n::Lang, kind: ToolType) -> String {
    match kind {
        ToolType::Endmill => t!(lang, "names-tool-endmill"),
        ToolType::Drill => t!(lang, "names-tool-drill"),
        ToolType::Vbit => t!(lang, "names-tool-vbit"),
    }
}

/// How the copies of a part are cut, in the app's language.
fn order_name(ui: &egui::Ui, order: CopyOrder) -> String {
    let units = crate::units::display(ui.ctx()).into();
    chipload_pipeline::copies::order_message(order).render(ui.lang(), units)
}

/// The machine values: post, spindle, max feed, tool changes and travel (safe Z and the
/// work zero are the job's: `safe_z_row`, `work_zero_rows`). Returns the post picked, if
/// another one was: the caller sets it (a user post is copied into a project, a profile
/// only names it). With `profile`, a row that differs from it gets a button that takes the
/// profile's value (docs/specs/linked-machine.md §3).
pub(super) fn machine_rows(
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
        if let Some(t) = &mut machine.travel {
            flag(ui.add(mm(&mut t.x)), ui, "machine.travel.x");
            flag(ui.add(mm(&mut t.y)), ui, "machine.travel.y");
            flag(ui.add(mm(&mut t.z)), ui, "machine.travel.z");
        }
        if let Some(from) = from
            && from.travel != machine.travel
            && revert_mark(ui, profile, &travel_text(ui, from.travel.as_ref()))
        {
            machine.travel = from.travel.clone();
        }
    });
    ui.end_row();
    picked
}

/// The machine values as plain text, for a machine that is the same as its profile
/// (docs/specs/linked-machine.md §3).
fn machine_text_rows(ui: &mut egui::Ui, machine: &Machine) {
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
fn gcode_units_text(ui: &egui::Ui, units: Units) -> String {
    match units {
        Units::Mm => t!(ui, "forms-gcode-mm"),
        Units::Inch => t!(ui, "forms-gcode-inch"),
    }
}

/// Safe Z: a job value, editable also while the machine is linked.
pub(super) fn safe_z_row(ui: &mut egui::Ui, label: &str, machine: &mut Machine) {
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
fn revert_mark(ui: &mut egui::Ui, profile: Option<&MachineProfile>, value: &str) -> bool {
    let name = profile.map_or("", |p| p.name.as_str());
    super::header::icon_button(ui, crate::icons::Icon::Undo, "", true, false, 18.0)
        .on_hover_text(t!(ui, "forms-revert-hover", name = name, value = value))
        .clicked()
}

fn spindle_text(ui: &egui::Ui, spindle: Spindle) -> String {
    match spindle {
        Spindle::Cw => t!(ui, "forms-spindle-cw"),
        Spindle::Ccw => t!(ui, "forms-spindle-ccw"),
    }
}

fn max_feed_text(ui: &egui::Ui, max: Option<f64>) -> String {
    let units = crate::units::display(ui.ctx());
    let feed_unit = chipload_pipeline::units::feed_unit(units);
    max.map_or(t!(ui, "forms-max-feed-not-set"), |f| {
        format!("{} {feed_unit}", feed_in(units, f))
    })
}

fn travel_text(ui: &egui::Ui, travel: Option<&Travel>) -> String {
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
fn post_copy_row(
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

fn tool_change_name(ui: &egui::Ui, change: ToolChange) -> String {
    match change {
        ToolChange::Files => t!(ui, "forms-tool-change-files"),
        ToolChange::Program => t!(ui, "forms-tool-change-program"),
    }
}

/// Type, diameter, spindle speed and centre cutting.
/// `at`: the tool's path in the project (`tools.0`), for its problems; empty elsewhere.
pub(super) fn tool_rows(ui: &mut egui::Ui, salt: &str, at: &str, tool: &mut Tool) {
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
fn vbit_rows(ui: &mut egui::Ui, at: &str, tool: &mut Tool) {
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

/// Pocket `i`'s rest pass `k` (docs/specs/rest-passes.md §6): its own values, and what it
/// takes from its pocket as facts. None when there is no such pass (after an undo).
pub(super) fn rest_pass_form(
    ui: &mut egui::Ui,
    edit: &mut EditJob,
    i: usize,
    k: usize,
) -> Option<JobRequest> {
    let units = crate::units::display(ui.ctx());
    let len_unit = chipload_pipeline::units::length_unit(units);
    let tools = edit.settings.tools.clone();
    let Some(Operation::Pocket(pocket)) = edit.ops.get_mut(i).map(|e| &mut e.op) else {
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
    super::browser::title_line(
        ui,
        crate::icons::Icon::Pocket,
        None,
        &format!(
            "{} · {}",
            t!(ui, "forms-rest-pass"),
            t!(ui, "forms-operation-number", number = at.to_string())
        ),
    );
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
            let combo = ComboBox::from_id_salt(("rest-pass-tool", i, k))
                .selected_text(label(pass.tool))
                .show_ui(ui, |ui| {
                    for t in tools.iter().filter(|t| {
                        t.kind == ToolType::Endmill
                            && before_diameter.is_some_and(|d| t.diameter < d)
                    }) {
                        ui.selectable_value(&mut pass.tool, t.id, tool_label(ui, t, units));
                    }
                });
            flag(combo.response, ui, "tool");
            ui.end_row();
            ui.label(t!(ui, "forms-step-down"));
            flag(ui.add(mm(&mut pass.step_down)), ui, "step_down");
            ui.end_row();
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
            ui.label(t!(ui, "forms-plunge-feed"));
            flag(ui.add(feed(&mut pass.plunge_feed)), ui, "plunge_feed");
            ui.end_row();
            let diameter = tools.iter().find(|t| t.id == pass.tool).map(|t| t.diameter);
            entry_rows(ui, 1000 + i * 100 + k, &mut pass.entry, diameter);
        });
    ui.add_space(6.0);
    ui.weak(t!(ui, "forms-rest-pass-note"));
    ui.add_space(6.0);
    if ui.button(t!(ui, "forms-remove-rest-pass")).clicked() {
        request = Some(JobRequest::RemoveRestPass(i, k));
    }
    request
}

/// The built-in fonts by style, each name in its own face where the app loaded it
/// (docs/specs/more-fonts.md §5).
fn font_list(ui: &mut egui::Ui, chosen: &mut String) {
    use chipload_text::{FONTS, Style};
    let loaded = ui.ctx().fonts(|f| f.families());
    for (style, heading) in [
        (Style::Plain, t!(ui, "forms-font-plain")),
        (Style::Script, t!(ui, "forms-font-script")),
        (Style::Display, t!(ui, "forms-font-display")),
    ] {
        ui.label(RichText::new(heading).small().weak());
        for font in FONTS.iter().filter(|f| f.style == style) {
            let family = crate::fonts::text_font_family(font.name);
            let mut text = RichText::new(font.name).size(18.0);
            if loaded.contains(&family) {
                text = text.family(family);
            }
            ui.selectable_value(chosen, font.name.to_string(), text);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rest_offers_only_larger_endmills() {
        // docs/specs/rest-machining.md §6.
        let tool = |id: i64, kind: ToolType, diameter: f64| Tool {
            id,
            name: None,
            library_uid: None,
            kind,
            diameter,
            flutes: None,
            center_cutting: None,
            rpm: None,
            angle: None,
            tip_diameter: None,
        };
        let tools = [
            tool(1, ToolType::Endmill, 12.0),
            tool(2, ToolType::Endmill, 3.0),
            tool(3, ToolType::Drill, 8.0),
            tool(4, ToolType::Vbit, 12.7),
            tool(5, ToolType::Endmill, 6.0),
        ];
        let ids = |d: f64| {
            rest_choices(&tools, Some(&tool(9, ToolType::Endmill, d)))
                .map(|t| t.id)
                .collect::<Vec<_>>()
        };
        assert_eq!(ids(3.0), [1, 5]);
        assert_eq!(ids(6.0), [1]);
        assert!(ids(12.0).is_empty());
        assert!(rest_choices(&tools, None).next().is_none());
    }

    #[test]
    fn summaries_are_short() {
        assert_eq!(len_in(Units::Mm, 12.0), "12");
        assert_eq!(len_in(Units::Mm, 0.25), "0.25");
        assert_eq!(len_in(Units::Mm, 12.3456), "12.346");
        assert_eq!(len_in(Units::Inch, 6.35), "0.25");
        let ramp = Entry::Ramp {
            ramp_angle: 3f64.to_radians(),
        };
        let en = Language::En;
        assert_eq!(entry_summary(en, &ramp, None, Units::Mm), "ramp 3.0°");
        let helix = Entry::Helix {
            radius: 1.5,
            ramp_angle: 3f64.to_radians(),
        };
        let lead = Lead { radius: 2.0 };
        assert_eq!(
            entry_summary(en, &helix, Some(&lead), Units::Mm),
            "helix r 1.5 mm, 3.0°, lead 2 mm"
        );
        assert_eq!(
            entry_summary(en, &helix, Some(&lead), Units::Inch),
            "helix r 0.0591 in, 3.0°, lead 0.0787 in"
        );
    }
}
