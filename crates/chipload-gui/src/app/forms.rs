// SPDX-License-Identifier: GPL-3.0-or-later
//! The property forms: one operation, and the job settings.

use super::problems::{self, flag};
use super::user_library::{LibraryRequest, LibraryRows, library_rows};
use super::*;
use crate::editor::DEFAULT_STEP_DOWN;
use chipload_core::text::mm as len;
use chipload_job::library::{Cutting, MachineProfile};
use chipload_job::{Copies, CopyOrder, Corners, DEFAULT_CORNER_ANGLE_RAD, MAX_COPIES_PER_SIDE};
use chipload_job::{
    DEFAULT_RAMP_ANGLE_RAD, Entry, Lead, MAX_RAMP_ANGLE_RAD, Machine, Tool, rest_refusal,
};
use chipload_job::{
    Finish, HoleMethod, HolesOp, ProfileOp, Rest, STRAIGHT_FLANK_RAD, Skin, ToolChange,
};
use chipload_job::{ImageTrace, Placement, Units};
use chipload_pipeline::copies::order_name;
use chipload_pipeline::library::Library;
use chipload_pipeline::placement::Move as PlaceMove;

pub(super) fn mm(value: &mut f64) -> DragValue<'_> {
    DragValue::new(value)
        .speed(0.1)
        .range(0.0..=100_000.0)
        .suffix(" mm")
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
    ui.label("Start depth")
        .on_hover_text("Where the cut starts, down from the stock top");
    flag(ui.add(mm(start)), ui, "start_depth");
    ui.end_row();
    ui.label("End depth")
        .on_hover_text("Where the cut ends, down from the stock top");
    ui.horizontal(|ui| {
        flag(ui.add(mm(end)), ui, "end_depth");
        if ui
            .small_button("Through")
            .on_hover_text(format!(
                "Cut through the stock: {} mm, its thickness plus the allowed depth below it \
                 (Job settings > Stock)",
                len(through)
            ))
            .clicked()
        {
            *end = through;
        }
    });
    ui.end_row();
    ui.label("Oversize").on_hover_text(
        "How much more than drawn is cut, mm (clearance in the file): positive makes a \
         pocket larger or a part smaller, negative leaves stock on for a later cut. Less \
         than the tool radius. Other CAM programs call the opposite \"stock to leave\".",
    );
    let response = ui
        .add_enabled(
            can_clear,
            DragValue::new(clearance)
                .speed(0.01)
                .range(-100_000.0..=100_000.0)
                .suffix(" mm"),
        )
        .on_disabled_hover_text("A profile on the line has no side to cut more on");
    flag(response, ui, "clearance");
    ui.end_row();
}

pub(super) fn feed(value: &mut f64) -> DragValue<'_> {
    DragValue::new(value)
        .speed(10.0)
        .range(0.0..=100_000.0)
        .suffix(" mm/min")
}

pub(super) fn direction_combo(ui: &mut egui::Ui, id: (&str, usize), direction: &mut Direction) {
    ComboBox::from_id_salt(id)
        .selected_text(match direction {
            Direction::Climb => "climb",
            Direction::Conventional => "conventional",
        })
        .show_ui(ui, |ui| {
            ui.selectable_value(direction, Direction::Climb, "climb");
            ui.selectable_value(direction, Direction::Conventional, "conventional");
        });
}

/// Least width of a form's label column, so the sections' grids line up.
const LABEL_WIDTH: f32 = 130.0;

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
    egui::CollapsingHeader::new(header)
        .id_salt(("section", key))
        .default_open(true)
        .show(ui, |ui| {
            Grid::new(("section-grid", key))
                .num_columns(2)
                .min_col_width(LABEL_WIDTH)
                .show(ui, rows)
                .inner
        })
        .body_returned
}

/// The tabs' flank angle in degrees, and how far the feet reach (docs/specs/
/// profile-spiral.md §4).
fn flank_row(ui: &mut egui::Ui, tabs: &mut Tabs) {
    ui.label("  flank angle").on_hover_text(
        "How steeply the tab's sides slope down from its flat top. 90° is a straight tab: \
         the tool goes straight up and down. Below that the tool follows the slope, so it \
         never plunges after a tab. Going down a flank is a ramp: with a tool that isn't \
         centre-cutting, no steeper than the ramp angle.",
    );
    ui.horizontal(|ui| {
        let mut value = tabs.flank_angle.to_degrees();
        let response = ui.add(
            DragValue::new(&mut value)
                .speed(0.5)
                .range(0.5..=90.0)
                .suffix("°"),
        );
        if response.changed() {
            tabs.flank_angle = value.to_radians().min(STRAIGHT_FLANK_RAD);
        }
        flag(response, ui, "tabs.flank_angle");
        if tabs.is_straight() {
            ui.weak("straight");
        } else if tabs.height > 0.0 {
            let foot = tabs.height / tabs.flank_angle.tan();
            ui.weak(format!("feet {} mm each side", len(foot)));
        }
    });
    ui.end_row();
}

/// Degrees for a summary: one decimal, no trailing zero.
fn degrees(radians: f64) -> String {
    let text = format!("{:.1}", radians.to_degrees());
    text.trim_end_matches('0').trim_end_matches('.').to_string()
}

fn direction_name(direction: Direction) -> &'static str {
    match direction {
        Direction::Climb => "climb",
        Direction::Conventional => "conventional",
    }
}

fn entry_summary(entry: &Entry, lead: Option<&Lead>) -> String {
    let entry = match entry {
        Entry::Plunge {} => "straight plunge".to_string(),
        Entry::Ramp { ramp_angle } => format!("ramp {:.1}°", ramp_angle.to_degrees()),
        Entry::Helix { radius, ramp_angle } => format!(
            "helix r {} mm, {:.1}°",
            len(*radius),
            ramp_angle.to_degrees()
        ),
    };
    match lead {
        Some(lead) => format!("{entry}, lead {} mm", len(lead.radius)),
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
    let summary = geometry_summary(&entry.op, contours);
    let side_chosen = &mut entry.side_chosen;
    let kind = match &entry.op {
        Operation::Profile(_) => "Profile",
        Operation::Pocket(_) => "Pocket",
        Operation::Vcarve(_) => "V-carve",
        Operation::Path(_) => "Follow path",
        Operation::Holes(_) => "Holes",
    };
    let color = super::browser::kind_color(&entry.op, ui.visuals().dark_mode);
    super::browser::title_line(
        ui,
        super::browser::op_icon(&entry.op),
        Some(color),
        &format!("{kind} · Operation {}", index + 1),
    );
    Grid::new(("op-form", index))
        .num_columns(2)
        // Past the sections' indent, so the values line up with theirs.
        .min_col_width(LABEL_WIDTH + ui.spacing().indent)
        .show(ui, |ui| {
            let name = entry.op.name_mut();
            ui.label("Name");
            ui.add(TextEdit::singleline(name).desired_width(200.0));
            ui.end_row();
            ui.label("Geometry");
            let geometry = ui.horizontal(|ui| {
                ui.label(&summary);
                if ui
                    .button("Use selection")
                    .on_hover_text("Replace with the contours selected on the canvas")
                    .clicked()
                {
                    use_selection = true;
                }
            });
            flag(geometry.response, ui, "select");
            ui.end_row();
            tool_row(ui, index, &mut entry.op, &choice, &mut request);
            if let Operation::Profile(p) = &mut entry.op {
                ui.label("Side");
                let text = if *side_chosen {
                    match p.side {
                        Side::Inside => "inside",
                        Side::Outside => "outside",
                        Side::On => "on",
                    }
                } else {
                    "— choose —"
                };
                let side = ComboBox::from_id_salt(("side", index))
                    .selected_text(text)
                    .show_ui(ui, |ui| {
                        for (side, label) in [
                            (Side::Outside, "outside"),
                            (Side::Inside, "inside"),
                            (Side::On, "on"),
                        ] {
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
            let cut = format!(
                "{}–{} mm, steps of {} mm, {}, spiral ramp {}°",
                len(p.start_depth),
                len(p.end_depth),
                len(p.step_down),
                direction_name(p.direction),
                degrees(p.ramp_angle)
            );
            section(ui, "op-profile-cut", "Cut", &cut, |ui| {
                ui.label("Direction");
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
                ui.label("Step down");
                flag(ui.add(mm(&mut p.step_down)), ui, "step_down");
                ui.end_row();
                ramp_angle_row(ui, &mut p.ramp_angle);
                corner_rows(ui, index, &mut p.corners, can_clear);
            });
            feeds_section(ui, &mut p.feed, &mut p.plunge_feed, op_tool);
            holding_section(ui, p, op_tool);
            finish_section(ui, p);
        }
        Operation::Pocket(p) => {
            let cut = format!(
                "{}–{} mm, steps of {} mm, {}",
                len(p.start_depth),
                len(p.end_depth),
                len(p.step_down),
                direction_name(p.direction)
            );
            section(ui, "op-cut", "Cut", &cut, |ui| {
                ui.label("Direction");
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
                ui.label("Step down");
                flag(ui.add(mm(&mut p.step_down)), ui, "step_down");
                ui.end_row();
                corner_rows(ui, index, &mut p.corners, true);
            });
            let mut clearing = format!("rings, stepover {:.2} × D", p.stepover);
            if let Some(rest) = p.rest {
                clearing += &format!(", rest after T{}", rest.after_tool);
            }
            section(ui, "op-clearing", "Clearing", &clearing, |ui| {
                rest_row(ui, index, &mut p.rest, choice.tools, op_tool);
                ui.label("Stepover").on_hover_text(
                    "How far each ring moves on, as a fraction of the tool diameter",
                );
                let stepover = ui.add(
                    DragValue::new(&mut p.stepover)
                        .speed(0.01)
                        .range(0.01..=1.0)
                        .suffix(" × D"),
                );
                flag(stepover, ui, "stepover");
                ui.end_row();
            });
            feeds_section(ui, &mut p.feed, &mut p.plunge_feed, op_tool);
            let entry_text = entry_summary(&p.entry, p.lead.as_ref());
            section(ui, "op-entry", "Entry and lead", &entry_text, |ui| {
                entry_rows(ui, index, &mut p.entry, Some(tool_diameter));
                lead_rows(ui, &mut p.lead, tool_diameter);
            });
        }
        Operation::Vcarve(v) => {
            let passes = v.step_down.map_or("one pass".to_string(), |s| {
                format!("steps of {} mm", len(s))
            });
            let cut = format!(
                "from {} mm, at most {} mm deep, {passes}",
                len(v.start_depth),
                len(v.end_depth)
            );
            section(ui, "op-cut", "Cut", &cut, |ui| {
                ui.label("Start depth").on_hover_text(
                    "The surface carved, down from the stock top (0: the top; more to carve \
                     into a pocket floor)",
                );
                flag(ui.add(mm(&mut v.start_depth)), ui, "start_depth");
                ui.end_row();
                ui.label("End depth").on_hover_text(
                    "The deepest the tip goes, from the stock top (end_depth in the file). \
                     Wider strokes are carved to it and reported",
                );
                flag(ui.add(mm(&mut v.end_depth)), ui, "end_depth");
                ui.end_row();
                ui.label("Passes");
                let mut stepped = v.step_down.is_some();
                if ui.checkbox(&mut stepped, "in steps").changed() {
                    v.step_down = stepped.then_some(DEFAULT_STEP_DOWN);
                }
                ui.end_row();
                if let Some(step) = &mut v.step_down {
                    ui.label("  step down");
                    flag(ui.add(mm(step)), ui, "step_down");
                    ui.end_row();
                }
            });
            feeds_section(ui, &mut v.feed, &mut v.plunge_feed, op_tool);
        }
        Operation::Path(p) => {
            let cut = format!(
                "{}–{} mm, steps of {} mm, on the line",
                len(p.start_depth),
                len(p.end_depth),
                len(p.step_down)
            );
            section(ui, "op-cut", "Cut", &cut, |ui| {
                ui.label("Start depth")
                    .on_hover_text("Where the cut starts, down from the stock top");
                flag(ui.add(mm(&mut p.start_depth)), ui, "start_depth");
                ui.end_row();
                ui.label("End depth")
                    .on_hover_text("Where the cut ends, down from the stock top");
                flag(ui.add(mm(&mut p.end_depth)), ui, "end_depth");
                ui.end_row();
                ui.label("Step down");
                flag(ui.add(mm(&mut p.step_down)), ui, "step_down");
                ui.end_row();
            });
            feeds_section(ui, &mut p.feed, &mut p.plunge_feed, op_tool);
            let entry_text = entry_summary(&p.entry, None);
            section(ui, "op-entry", "Entry", &entry_text, |ui| {
                entry_rows(ui, index, &mut p.entry, None);
            });
        }
        Operation::Holes(h) => holes_sections(ui, index, h, op_tool),
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
    let current = op.tool();
    let selected = choice
        .tools
        .iter()
        .find(|t| t.id == current)
        .map_or(format!("T{current} (not in the project)"), tool_label);
    ui.label("Tool");
    let combo = ComboBox::from_id_salt(("op-tool", index))
        .selected_text(selected)
        .width(240.0)
        .show_ui(ui, |ui| {
            for tool in choice.tools {
                if ui
                    .selectable_label(tool.id == current, tool_label(tool))
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
                ui.weak("From your library:");
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
            .button("Use the tool's cutting values")
            .on_hover_text(format!(
                "From the library: feed {} / plunge {} mm/min, step down {} mm, stepover {}",
                len(cutting.feed),
                len(cutting.plunge_feed),
                len(cutting.step_down),
                cutting.stepover
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
    }
}

/// The feeds, and the chip load next to the feed (docs/specs/small-items.md §5) when the
/// operation's tool is known.
fn feeds_section(ui: &mut egui::Ui, cut: &mut f64, plunge: &mut f64, tool: Option<&Tool>) {
    let text = format!("{} / {} mm/min", len(*cut), len(*plunge));
    section(ui, "op-feeds", "Feeds", &text, |ui| {
        ui.label("Feed");
        ui.horizontal(|ui| {
            flag(ui.add(feed(cut)), ui, "feed");
            if let Some(tool) = tool {
                let load = chipload_pipeline::chip_load::chip_load_of(*cut, tool);
                ui.label(RichText::new(load.text()).weak())
                    .on_hover_text(chipload_pipeline::chip_load::CHIP_LOAD_HINT);
            }
        });
        ui.end_row();
        ui.label("Plunge feed");
        flag(ui.add(feed(plunge)), ui, "plunge_feed");
        ui.end_row();
    });
}

/// How the tool goes down into each pass (docs/specs/ramp-entry.md); angles are shown in
/// degrees and kept in radians. `helix`: the tool diameter, for a pocket, which may also go
/// down round a helix (docs/specs/helix-and-leads.md part C).
fn entry_rows(ui: &mut egui::Ui, index: usize, entry: &mut Entry, helix: Option<f64>) {
    ui.label("Entry").on_hover_text(
        "How the tool goes down into the material. A ramp goes down along the path (a \
         profile spirals down round its ring, a follow path goes down as it goes back and \
         forth, a pocket ramps out and back), a helix (pockets) round a small circle, so the \
         tool needn't be centre-cutting; both use the cutting feed.",
    );
    let ramp = Entry::Ramp {
        ramp_angle: DEFAULT_RAMP_ANGLE_RAD,
    };
    let combo = ComboBox::from_id_salt(("entry", index))
        .selected_text(match entry {
            Entry::Plunge {} => "straight plunge",
            Entry::Ramp { .. } => "ramp",
            Entry::Helix { .. } => "helix",
        })
        .show_ui(ui, |ui| {
            let is_ramp = matches!(entry, Entry::Ramp { .. });
            let is_helix = matches!(entry, Entry::Helix { .. });
            if ui
                .selectable_label(!is_ramp && !is_helix, "straight plunge")
                .clicked()
            {
                *entry = Entry::Plunge {};
            }
            if ui.selectable_label(is_ramp, "ramp").clicked() && !is_ramp {
                *entry = ramp;
            }
            if let Some(diameter) = helix
                && ui.selectable_label(is_helix, "helix").clicked()
                && !is_helix
            {
                *entry = Entry::default_helix(diameter);
            }
        });
    flag(combo.response, ui, "entry");
    ui.end_row();
    if let Entry::Helix { radius, .. } = entry {
        ui.label("  helix radius").on_hover_text(
            "Of the circle the tool centre follows: less than the tool radius, so no core is \
             left standing",
        );
        flag(ui.add(mm(radius)), ui, "entry");
        ui.end_row();
    }
    if let Entry::Ramp { ramp_angle } | Entry::Helix { ramp_angle, .. } = entry {
        ui.label("  ramp angle");
        let mut degrees = ramp_angle.to_degrees();
        if ui
            .add(
                DragValue::new(&mut degrees)
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
    ui.label("Ramp angle").on_hover_text(
        "The tool spirals down along the contour, never steeper than this and never more \
         than one step down per lap: no plunge into the part. Each pass goes down at this \
         angle, then runs level; a steeper angle makes the ramp shorter.",
    );
    let mut value = ramp_angle.to_degrees();
    let response = ui.add(
        DragValue::new(&mut value)
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
    ui.label("Rest after").on_hover_text(
        "Clear only what a larger tool leaves in these shapes: the corners it leaves round \
         and the slots it doesn't fit in. Cut the pocket with that tool first",
    );
    let name = |t: &Tool| format!("T{} · {} mm endmill", t.id, len(t.diameter));
    let selected = match rest {
        None => "none".to_string(),
        Some(r) => tools
            .iter()
            .find(|t| t.id == r.after_tool)
            .map_or(format!("T{} (not in the project)", r.after_tool), name),
    };
    let response = ComboBox::from_id_salt(("rest", index))
        .selected_text(selected)
        .show_ui(ui, |ui| {
            if ui.selectable_label(rest.is_none(), "none").clicked() {
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
    ui.label("Corners").on_hover_text(
        "Inside corners a round tool leaves round: a dogbone goes into the corner along its \
         middle, a T-bone along the longer wall (the notch is in the shorter one), so a \
         square part fits all the way in",
    );
    let angle = corners.map_or(DEFAULT_CORNER_ANGLE_RAD, |c| c.max_angle());
    let name = match corners {
        None => "round",
        Some(Corners::Dogbone { .. }) => "dogbones",
        Some(Corners::Tbone { .. }) => "T-bones",
    };
    let combo = ui.add_enabled_ui(allowed || corners.is_some(), |ui| {
        ComboBox::from_id_salt(("corners", index))
            .selected_text(name)
            .show_ui(ui, |ui| {
                for (choice, label) in [
                    (None, "round"),
                    (Some(Corners::Dogbone { max_angle: angle }), "dogbones"),
                    (Some(Corners::Tbone { max_angle: angle }), "T-bones"),
                ] {
                    if ui.selectable_label(name == label, label).clicked() {
                        *corners = choice;
                    }
                }
            })
    });
    let response = combo
        .response
        .on_disabled_hover_text("A profile cut on the line has no inside corners");
    flag(response, ui, "corners");
    ui.end_row();
    if let Some(Corners::Dogbone { max_angle } | Corners::Tbone { max_angle }) = corners {
        ui.label("  up to").on_hover_text(
            "Only corners this sharp or sharper (90° corners drawn slightly off still count)",
        );
        let mut degrees = max_angle.to_degrees();
        let response = ui.add(
            DragValue::new(&mut degrees)
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
    let method = match h.method {
        HoleMethod::Drill { peck: Some(peck) } => format!("drilled, pecks {} mm", len(peck)),
        HoleMethod::Drill { peck: None } => "drilled in one go".to_string(),
        HoleMethod::Bore {
            step_down,
            direction,
            ..
        } => format!(
            "helical bore, {} mm per turn, {}",
            len(step_down),
            direction_name(direction)
        ),
    };
    let cut = format!("{}–{} mm, {method}", len(h.start_depth), len(h.end_depth));
    section(ui, "op-cut", "Cut", &cut, |ui| {
        ui.label("Start depth")
            .on_hover_text("Where the cut starts, down from the stock top");
        flag(ui.add(mm(&mut h.start_depth)), ui, "start_depth");
        ui.end_row();
        let drill = matches!(h.method, HoleMethod::Drill { .. });
        ui.label("End depth").on_hover_text(if drill {
            "Where the drill's point ends, down from the stock top: the full diameter stops \
             about 0.3 × the drill's diameter higher"
        } else {
            "Where the cut ends, down from the stock top"
        });
        flag(ui.add(mm(&mut h.end_depth)), ui, "end_depth");
        ui.end_row();
        match &mut h.method {
            HoleMethod::Drill { peck } => {
                ui.label("Pecks").on_hover_text(
                    "Drill this deep at a time, rising out of the hole in between to clear \
                     the chips",
                );
                let mut on = peck.is_some();
                if flag(ui.checkbox(&mut on, "in pecks"), ui, "method.peck").changed() {
                    *peck = on.then_some(DEFAULT_STEP_DOWN);
                }
                ui.end_row();
                if let Some(peck) = peck {
                    ui.label("  peck");
                    flag(ui.add(mm(peck)), ui, "method.peck");
                    ui.end_row();
                }
            }
            HoleMethod::Bore {
                step_down,
                ramp_angle,
                direction,
            } => {
                ui.label("Direction");
                direction_combo(ui, ("direction", index), direction);
                ui.end_row();
                ui.label("Step per turn")
                    .on_hover_text("The most the helix goes down in one turn");
                flag(ui.add(mm(step_down)), ui, "method.step_down");
                ui.end_row();
                ui.label("Ramp angle")
                    .on_hover_text("The helix is never steeper than this");
                let mut degrees = ramp_angle.to_degrees();
                let response = ui.add(
                    DragValue::new(&mut degrees)
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
        let text = format!("{} mm/min down", len(h.plunge_feed));
        section(ui, "op-feeds", "Feeds", &text, |ui| {
            ui.label("Drill feed");
            ui.horizontal(|ui| {
                flag(ui.add(feed(&mut h.plunge_feed)), ui, "plunge_feed");
                if let Some(tool) = tool {
                    let load = chipload_pipeline::chip_load::chip_load_of(h.plunge_feed, tool);
                    ui.label(RichText::new(load.text()).weak())
                        .on_hover_text(chipload_pipeline::chip_load::CHIP_LOAD_HINT);
                }
            });
            ui.end_row();
        });
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

    fn name(self) -> &'static str {
        match self {
            Holding::None => "none",
            Holding::Tabs => "tabs",
            Holding::Skin => "onion skin",
            Holding::SkinAndTabs => "onion skin and tabs",
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
    let holding = Holding::of(p);
    let mut summary = holding.name().to_string();
    if let Some(t) = &p.tabs {
        summary += &format!(
            ": {} × {} mm, {} mm high",
            t.count,
            len(t.width),
            len(t.height)
        );
    }
    if let Some(skin) = p.skin {
        summary += &format!(", skin {} mm", len(skin.thickness));
    }
    section(ui, "op-holding", "Holding", &summary, |ui| {
        ui.label("Hold the part").on_hover_text(
            "How a part cut out stays in place.\n\
             Tabs: bridges left at the bottom of the cut; the part stays attached until you \
             cut it free.\n\
             Onion skin: a thin skin left all round during the roughing, then a last lap \
             after every ring of the operation cuts through it. The part is free during that \
             lap: add tabs if the parts must not move then.",
        );
        let mut chosen = holding;
        ComboBox::from_id_salt("holding")
            .selected_text(holding.name())
            .show_ui(ui, |ui| {
                for choice in Holding::ALL {
                    ui.selectable_value(&mut chosen, choice, choice.name());
                }
            });
        if chosen != holding {
            chosen.apply(p, op_tool.and_then(|t| t.center_cutting));
        }
        ui.end_row();
        if let Some(tabs) = &mut p.tabs {
            ui.label("  tab count");
            flag(
                ui.add(DragValue::new(&mut tabs.count).range(1..=100)),
                ui,
                "tabs.count",
            );
            ui.end_row();
            ui.label("  width");
            flag(ui.add(mm(&mut tabs.width)), ui, "tabs.width");
            ui.end_row();
            ui.label("  height");
            flag(ui.add(mm(&mut tabs.height)), ui, "tabs.height");
            ui.end_row();
            flank_row(ui, tabs);
        }
        if let Some(skin) = &mut p.skin {
            ui.label("  skin thickness").on_hover_text(
                "Left above the end depth until the last lap: at most one step down, and \
                 thinner than the tabs",
            );
            flag(ui.add(mm(&mut skin.thickness)), ui, "skin.thickness");
            ui.end_row();
        }
    });
}

/// A profile's finishing pass (docs/specs/finish-and-skin.md).
fn finish_section(ui: &mut egui::Ui, p: &mut ProfileOp) {
    let summary = p.finish.map_or("none".to_string(), |f| {
        format!("{} mm left, then one lap", len(f.stock))
    });
    section(ui, "op-finish", "Finishing pass", &summary, |ui| {
        ui.label("Finishing pass").on_hover_text(
            "Rough with this much left on the wall, then cut it away in one light lap at \
             full depth: a cleaner wall without step marks. The lap goes down in the \
             roughing's slot, never on the finished wall.",
        );
        let mut on = p.finish.is_some();
        let response = ui
            .add_enabled(
                p.side != Side::On || on,
                egui::Checkbox::new(&mut on, "finishing lap"),
            )
            .on_disabled_hover_text("A profile cut on the line has no wall to finish");
        if flag(response, ui, "finish").changed() {
            p.finish = on.then_some(Finish { stock: 0.3 });
        }
        ui.end_row();
        if let Some(finish) = &mut p.finish {
            ui.label("  stock");
            flag(ui.add(mm(&mut finish.stock)), ui, "finish.stock");
            ui.end_row();
        }
    });
}

/// A pocket's lead-in and lead-out (docs/specs/helix-and-leads.md part B): on or off, and
/// the radius, the tool radius by default.
fn lead_rows(ui: &mut egui::Ui, lead: &mut Option<Lead>, tool_diameter: f64) {
    ui.label("Lead").on_hover_text(
        "Arc onto the wall from the cleared pocket and off it again, so the tool never \
         stops against the wall. A lead without room shrinks, or is left out (the report \
         says so).",
    );
    let mut on = lead.is_some();
    let response = ui.checkbox(&mut on, "lead-in and lead-out");
    let response = flag(response, ui, "lead");
    if response.changed() {
        *lead = on.then_some(Lead {
            radius: tool_diameter / 2.0,
        });
    }
    ui.end_row();
    if let Some(lead) = lead {
        ui.label("  radius");
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
        (WorkZero::DrawingOrigin {}, "drawing origin"),
        (
            WorkZero::Stock {
                corner: StockCorner::Center,
            },
            "stock centre",
        ),
        (
            WorkZero::Stock {
                corner: StockCorner::BottomLeft,
            },
            "stock bottom left",
        ),
        (
            WorkZero::Stock {
                corner: StockCorner::BottomRight,
            },
            "stock bottom right",
        ),
        (
            WorkZero::Stock {
                corner: StockCorner::TopLeft,
            },
            "stock top left",
        ),
        (
            WorkZero::Stock {
                corner: StockCorner::TopRight,
            },
            "stock top right",
        ),
    ];
    let current = *work_zero;
    let selected = choices
        .iter()
        .find(|(choice, _)| *choice == current)
        .map_or("point", |(_, name)| *name);
    ui.label("Work zero X Y")
        .on_hover_text("The point that becomes X0 Y0 in the G-code. The geometry doesn't move.");
    ComboBox::from_id_salt((salt, "work_zero"))
        .selected_text(selected)
        .show_ui(ui, |ui| {
            for (choice, name) in choices {
                ui.selectable_value(work_zero, choice, name);
            }
            let (x, y) = here;
            let is_point = matches!(work_zero, WorkZero::Point { .. });
            if ui.selectable_label(is_point, "point").clicked() && !is_point {
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
        .min_col_width(LABEL_WIDTH + ui.spacing().indent)
        .show(ui, |ui| {
            ui.label("File");
            ui.label(&source.file);
            ui.end_row();
            ui.label("Units");
            ui.label(match source.units {
                Units::Mm => "mm",
                Units::Inch => "inch (converted to mm)",
            });
            ui.end_row();
            ui.label("Shapes");
            ui.label(shapes.to_string());
            ui.end_row();
        });
    let summary = size.map_or(String::new(), |(w, h)| {
        format!("{} × {} mm", len(w), len(h))
    });
    section(
        ui,
        "dxf-place",
        "Position, rotation and scale",
        &summary,
        |ui| {
            placement_rows(ui, id, source.placement, bbox, &mut request);
        },
    );
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        if ui
            .button("Update from a DXF…")
            .on_hover_text("A new version of the drawing: operations keep their layers")
            .clicked()
        {
            request = Some(JobRequest::UpdateDxf(id));
        }
        if ui
            .button("Remove")
            .on_hover_text("Remove this DXF and its shapes")
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
        .min_col_width(LABEL_WIDTH + ui.spacing().indent)
        .show(ui, |ui| {
            if let Some((w, h)) = pixels {
                ui.label("Picture");
                ui.label(format!("{w} × {h} pixels"));
                ui.end_row();
            }
            ui.label("Traced as");
            ui.label(match image.trace {
                ImageTrace::Outline => "outlines",
                ImageTrace::CentreLine => "centre lines",
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
            ui.label("Position")
                .on_hover_text("The lower-left corner of the picture's box, mm");
            ui.horizontal(|ui| {
                let (mut cx, mut cy) = (x0, y0);
                for (value, axis) in [(&mut cx, "X "), (&mut cy, "Y ")] {
                    let response = ui.add(mm_field(value).prefix(axis));
                    finished |= response.drag_stopped() || response.lost_focus();
                }
                // Moving the box's corner moves the picture's own corner as much.
                draft.x += cx - x0;
                draft.y += cy - y0;
                if reset_button(ui, (x0, y0) != (0.0, 0.0), "The box's corner at X0 Y0") {
                    draft.x -= x0;
                    draft.y -= y0;
                    finished = true;
                }
            });
            ui.end_row();
            ui.label("Rotation").on_hover_text(
                "Degrees counter-clockwise, any angle, about the picture's centre. The picture \
                 is traced as it is, then its shapes are turned.",
            );
            ui.horizontal(|ui| {
                // Turning keeps the picture's centre where it is: x, y stay.
                let response = ui.add(
                    DragValue::new(&mut draft.rotation)
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
                    .on_hover_text("90° counter-clockwise")
                    .clicked()
                {
                    turn(90.0);
                }
                if ui
                    .small_button("⟳ 90")
                    .on_hover_text("90° clockwise")
                    .clicked()
                {
                    turn(-90.0);
                }
                if reset_button(ui, draft.rotation != 0.0, "As the picture is (0°)") {
                    draft.rotation = 0.0;
                    finished = true;
                }
                draft.rotation = draft.rotation.rem_euclid(360.0);
                if draft.rotation >= 360.0 {
                    draft.rotation = 0.0;
                }
            });
            ui.end_row();
            ui.label("Width")
                .on_hover_text("The picture's width before it is turned; the height follows");
            ui.horizontal(|ui| {
                let response = ui.add(
                    DragValue::new(&mut draft.width)
                        .range(0.1..=10_000.0)
                        .speed(0.5)
                        .suffix(" mm"),
                );
                finished |= response.drag_stopped() || response.lost_focus();
                if pixels.is_some() {
                    ui.weak(format!("height {} mm", len(draft.width * aspect)));
                }
            });
            ui.end_row();
            ui.label("Layer")
                .on_hover_text("The layer its shapes are on, for operations that select it");
            let response = ui.add(TextEdit::singleline(&mut draft.layer).desired_width(160.0));
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
            .button("Trace settings…")
            .on_hover_text("Threshold, smoothing, outlines or centre lines, with a preview")
            .clicked()
        {
            request = Some(JobRequest::TraceSettings(id));
        }
        if ui
            .button("Remove")
            .on_hover_text("Remove the picture and its shapes")
            .clicked()
        {
            request = Some(JobRequest::RemoveImage(id));
        }
    });
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
    let mut request = None;
    let mut copies_request = None;
    let job = &mut edit.settings;
    super::browser::title_line(ui, crate::icons::Icon::Settings, None, "Job settings");

    let stock = format!(
        "{} × {} × {} mm",
        len(job.stock.x),
        len(job.stock.y),
        len(job.stock.thickness)
    );
    section(ui, "job-stock", "Stock", &stock, |ui| {
        ui.label("Size X × Y");
        ui.horizontal(|ui| {
            flag(ui.add(mm(&mut job.stock.x)), ui, "stock.x");
            flag(ui.add(mm(&mut job.stock.y)), ui, "stock.y");
        });
        ui.end_row();
        ui.label("Thickness");
        flag(ui.add(mm(&mut job.stock.thickness)), ui, "stock.thickness");
        ui.end_row();
        ui.label("Max depth below stock").on_hover_text(
            "How far a cut may go below the stock (into the spoilboard). Cuts within it are \
             warned about; deeper ones are refused.",
        );
        flag(
            ui.add(mm(&mut job.stock.max_overcut)),
            ui,
            "stock.max_overcut",
        );
        ui.end_row();
        // Where the stock lies relative to the drawing (file: stock.origin). Not the G-code
        // zero: that's the work zero below (docs/specs/work-zero.md).
        ui.label("Stock position");
        ComboBox::from_id_salt("origin")
            .selected_text(match job.stock.origin {
                StockOrigin::BottomLeft => "corner at drawing origin",
                StockOrigin::Center => "centred on drawing origin",
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(
                    &mut job.stock.origin,
                    StockOrigin::BottomLeft,
                    "corner at drawing origin",
                );
                ui.selectable_value(
                    &mut job.stock.origin,
                    StockOrigin::Center,
                    "centred on drawing origin",
                );
            });
        ui.end_row();
        let here = job.work_zero_xy();
        work_zero_rows(ui, "job", &mut job.machine.work_zero, here);
        ui.label("Z zero");
        ComboBox::from_id_salt("z_zero")
            .selected_text(match job.stock.z_zero {
                ZZero::Top => "stock top",
                ZZero::Bottom => "stock bottom",
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut job.stock.z_zero, ZZero::Top, "stock top");
                ui.selectable_value(&mut job.stock.z_zero, ZZero::Bottom, "stock bottom");
            });
        ui.end_row();
    });

    let machine = format!(
        "{}, safe Z {} mm, {}",
        job.machine.post,
        len(job.machine.safe_z),
        match job.machine.spindle {
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
    section(ui, "job-machine", "Machine", &machine, |ui| {
        match &profile {
            Some(p) if same && !editing => {
                ui.label("From machine");
                ui.horizontal(|ui| {
                    ui.strong(&p.name);
                    if ui
                        .button("Edit here")
                        .on_hover_text(
                            "Change the machine's values for this project only; it stays \
                             linked, and shows what differs",
                        )
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
        safe_z_row(ui, "Safe Z (this job)", &mut job.machine);
        post_copy_row(ui, &job.machine, library, &mut copies_request);
        library_rows(ui, &mut request, LibraryRows::machine(job, library));
    });

    copies_request.or(request.map(JobRequest::Library))
}

/// "T2 endmill Ø3 mm "3 mm downcut"": a project tool as lists show it.
pub(super) fn tool_label(tool: &Tool) -> String {
    let name = tool
        .name
        .as_deref()
        .map_or(String::new(), |n| format!(" \"{n}\""));
    format!(
        "T{} {} Ø{} mm{name}",
        tool.id,
        tool.kind.name(),
        len(tool.diameter)
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
    super::browser::title_line(ui, crate::icons::Icon::Tools, None, "Tools");
    ui.horizontal(|ui| {
        let names: Vec<String> = library
            .map(|l| l.tools.tools.iter().map(|t| t.name.clone()).collect())
            .unwrap_or_default();
        ui.add_enabled_ui(!names.is_empty(), |ui| {
            ComboBox::from_id_salt("add-library-tool")
                .selected_text("Add from library…")
                .show_ui(ui, |ui| {
                    for name in names {
                        if ui.selectable_label(false, &name).clicked() {
                            request = Some(LibraryRequest::AddTool(name));
                        }
                    }
                });
        });
        if ui
            .button("New tool")
            .on_hover_text("A 6 mm endmill to edit; operations pick it in their Tool row")
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
            0 => "not used".to_string(),
            _ => format!("operations {}", users.join(", ")),
        };
        let title = tool_label(&edit.settings.tools[index]);
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
    super::browser::title_line(ui, icon, None, &tool_label(&edit.settings.tools[index]));
    ui.weak(match users.len() {
        0 => "Not used by an operation".to_string(),
        _ => format!("Cuts operations {}", users.join(", ")),
    });
    let at = format!("tools.{index}");
    let tool = &mut edit.settings.tools[index];
    let mut removed = false;
    Grid::new(("tool-form", id))
        .num_columns(2)
        .min_col_width(LABEL_WIDTH)
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
    let button = ui.add_enabled(users.is_empty(), egui::Button::new("Remove tool"));
    let mut remove = false;
    if users.is_empty() {
        remove = button.clicked();
    } else {
        button.on_disabled_hover_text(format!(
            "Used by operations {}; give them another tool first",
            users.join(", ")
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
    field: impl FnOnce(&mut f64) -> DragValue<'_>,
) -> Option<f64> {
    let mut value = ui.data(|d| d.get_temp::<f64>(id)).unwrap_or(current);
    let response = ui.add(field(&mut value));
    if response.has_focus() || response.dragged() {
        ui.data_mut(|d| d.insert_temp(id, value));
        return None;
    }
    ui.data_mut(|d| d.remove::<f64>(id));
    let done = response.lost_focus() || response.drag_stopped() || response.changed();
    (done && value != current).then_some(value)
}

/// A position field: mm, half a millimetre per pixel dragged.
fn mm_field(value: &mut f64) -> DragValue<'_> {
    DragValue::new(value).speed(0.5).suffix(" mm")
}

/// A small reset button after a row; true when clicked.
fn reset_button(ui: &mut egui::Ui, enabled: bool, hover: &str) -> bool {
    super::header::icon_button(ui, crate::icons::Icon::Undo, "Reset", enabled, false, 18.0)
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
    let mut place = |how: PlaceMove| *request = Some(JobRequest::Place(dxf, how));
    let salt = |what: &str| egui::Id::new(("dxf-place", dxf, what.to_string()));
    if let Some(((x0, y0), _)) = bbox {
        ui.label("Position").on_hover_text(
            "The lower-left corner of the drawing's box, mm. Shapes drawn in Chipload and \
             other DXFs don't move.",
        );
        ui.horizontal(|ui| {
            if let Some(x) = finished_number(ui, salt("x"), x0, |v| mm_field(v).prefix("X ")) {
                place(PlaceMove::Corner(x, y0));
            }
            if let Some(y) = finished_number(ui, salt("y"), y0, |v| mm_field(v).prefix("Y ")) {
                place(PlaceMove::Corner(x0, y));
            }
            let moved = (placement.dx, placement.dy) != (0.0, 0.0);
            if reset_button(
                ui,
                moved,
                "Back to where the DXF has it (turn and scale stay)",
            ) {
                place(PlaceMove::To(0.0, 0.0));
            }
        });
        ui.end_row();
        ui.label("");
        ui.horizontal(|ui| {
            if ui
                .small_button("Bottom left")
                .on_hover_text(format!(
                    "The drawing's corner {PLACE_MARGIN} mm in from the stock's lower-left corner"
                ))
                .clicked()
            {
                place(PlaceMove::BottomLeft {
                    margin: PLACE_MARGIN,
                });
            }
            if ui
                .small_button("Centre")
                .on_hover_text("The drawing's centre on the stock's centre")
                .clicked()
            {
                place(PlaceMove::Center);
            }
        });
        ui.end_row();
    }
    ui.label("Rotation").on_hover_text(
        "Degrees counter-clockwise from the DXF as drawn, any angle, about the drawing's centre",
    );
    ui.horizontal(|ui| {
        let current = placement.rotation;
        if let Some(degrees) = finished_number(ui, salt("rotation"), current, |v| {
            DragValue::new(v).speed(1.0).suffix("°").max_decimals(3)
        }) {
            place(PlaceMove::Rotate(degrees - current));
        }
        if ui
            .small_button("⟲ 90")
            .on_hover_text("90° counter-clockwise")
            .clicked()
        {
            place(PlaceMove::Rotate(90.0));
        }
        if ui
            .small_button("⟳ 90")
            .on_hover_text("90° clockwise")
            .clicked()
        {
            place(PlaceMove::Rotate(-90.0));
        }
        if reset_button(ui, current != 0.0, "Back to the DXF's own angle (0°)") {
            place(PlaceMove::Rotate(-current));
        }
    });
    ui.end_row();
    ui.label("Scale").on_hover_text(
        "From the DXF as drawn, about the drawing's centre. Tool sizes, depths and tabs \
         don't scale.",
    );
    ui.horizontal(|ui| {
        let current = placement.scale;
        if let Some(scale) = finished_number(ui, salt("scale"), current, |v| {
            DragValue::new(v)
                .speed(0.01)
                .range(Placement::MIN_SCALE..=Placement::MAX_SCALE)
                .max_decimals(6)
                .prefix("× ")
        }) {
            place(PlaceMove::Scale(scale / current));
        }
        if reset_button(ui, current != 1.0, "Back to the DXF's own size (× 1)") {
            place(PlaceMove::Scale(1.0 / current));
        }
    });
    ui.end_row();
    if let Some(((x0, y0), (x1, y1))) = bbox {
        ui.label("Size");
        ui.label(format!("{} × {} mm", len(x1 - x0), len(y1 - y0)));
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
        &format!("Part {} · {}", index + 1, part.name),
    );
    let at = |field: &str| format!("parts.{index}.{field}");
    Grid::new(("part-form", id))
        .num_columns(2)
        .min_col_width(LABEL_WIDTH + ui.spacing().indent)
        .show(ui, |ui| {
            ui.label("Name");
            let response = ui.add(TextEdit::singleline(&mut part.name).desired_width(200.0));
            flag(response, ui, &at("name"));
            ui.end_row();
            ui.label("Cut")
                .on_hover_text("Off: the part and its operations stay in the project, not cut");
            ui.checkbox(&mut part.enabled, "this part and its operations");
            ui.end_row();
            ui.label("Active").on_hover_text(
                "New operations go into the active part. Right-click a part to activate it",
            );
            if active {
                ui.label("yes: new operations go here");
            } else if ui.button("Activate").clicked() {
                request = Some(JobRequest::Activate(id));
            }
            ui.end_row();
            ui.label("Operations");
            ui.label(if numbers.is_empty() {
                "none yet".to_string()
            } else {
                numbers.join(", ")
            });
            ui.end_row();
        });
    let copies = part.copies.map_or("one".to_string(), |c| {
        format!("{} × {} = {}", c.columns, c.rows, c.count())
    });
    section(ui, "part-copies", "Copies", &copies, |ui| {
        copies_rows(ui, &mut part.copies, &mut request, &at("copies"), id);
    });
    if several {
        ui.add_space(4.0);
        ui.weak("Parts are cut one after the other, each with its copies.");
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
    ui.label("Copies").on_hover_text(
        "Cut the part several times in a grid. The copies are drawn faded; only the drawing \
         itself can be selected.",
    );
    let mut on = copies.is_some();
    if ui.checkbox(&mut on, "several, in a grid").changed() {
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
    ui.label("  columns × rows");
    ui.horizontal(|ui| {
        let count = 1..=MAX_COPIES_PER_SIDE;
        flag(
            ui.add(DragValue::new(&mut grid.columns).range(count.clone())),
            ui,
            &format!("{at}.columns"),
        );
        flag(
            ui.add(DragValue::new(&mut grid.rows).range(count)),
            ui,
            &format!("{at}.rows"),
        );
        ui.label(format!("= {}", grid.count()));
    });
    ui.end_row();
    ui.label("  pitch X Y")
        .on_hover_text("Copy to copy, mm. Negative grows to the left or down.");
    ui.horizontal(|ui| {
        for (name, value) in [("dx", &mut grid.dx), ("dy", &mut grid.dy)] {
            let response = ui.add(
                DragValue::new(value)
                    .speed(0.1)
                    .range(-100_000.0..=100_000.0)
                    .suffix(" mm"),
            );
            flag(response, ui, &format!("{at}.{name}"));
        }
    });
    ui.end_row();
    ui.label("  gap").on_hover_text(
        "Sets the pitch once so this much is left between the cuts of neighbouring copies. \
         The project keeps the pitch.",
    );
    ui.horizontal(|ui| {
        let id = egui::Id::new(("copies-gap", part));
        let mut gap = ui.data_mut(|d| *d.get_temp_mut_or(id, DEFAULT_COPIES_GAP));
        ui.add(mm(&mut gap));
        ui.data_mut(|d| d.insert_temp(id, gap));
        if ui.button("Set pitch").clicked() {
            *request = Some(JobRequest::CopiesGap(part, gap));
        }
    });
    ui.end_row();
    ui.label("  order");
    ComboBox::from_id_salt("copies-order")
        .selected_text(order_name(grid.order))
        .show_ui(ui, |ui| {
            for order in [CopyOrder::ByOperation, CopyOrder::ByCopy] {
                ui.selectable_value(&mut grid.order, order, order_name(order));
            }
        });
    ui.end_row();
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
    ui.label("Post");
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
                    let label = format!("{name} (yours)");
                    let response = ui.add_enabled(
                        read.is_ok(),
                        egui::Button::selectable(machine.post == *name, label),
                    );
                    if let Err(problem) = read {
                        response.on_disabled_hover_text(problem.as_str());
                    } else if response.clicked() {
                        picked = Some(name.clone());
                    }
                }
            })
            .response
            .on_hover_text(match library.and_then(|l| l.posts.as_deref()) {
                Some(dir) => format!("Your own post-processors go in {}", dir.display()),
                None => "The built-in post-processors".into(),
            });
        if let Some(from) = from
            && from.post != machine.post
            && revert_mark(ui, profile, &from.post)
        {
            picked = Some(from.post.clone());
        }
    });
    ui.end_row();
    ui.label("Spindle");
    ui.horizontal(|ui| {
        ComboBox::from_id_salt((salt, "spindle"))
            .selected_text(spindle_text(machine.spindle))
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut machine.spindle, Spindle::Cw, "cw (M3)");
                ui.selectable_value(&mut machine.spindle, Spindle::Ccw, "ccw (M4)");
            });
        if let Some(from) = from
            && from.spindle != machine.spindle
            && revert_mark(ui, profile, spindle_text(from.spindle))
        {
            machine.spindle = from.spindle;
        }
    });
    ui.end_row();
    ui.label("Max feed");
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
            && revert_mark(ui, profile, &max_feed_text(from.max_feed))
        {
            machine.max_feed = from.max_feed;
        }
    });
    ui.end_row();
    ui.label("Tool changes").on_hover_text(
        "With several tools: one G-code file per tool (change the tool and set Z zero \
         between files), or one file with M6 tool changes that your sender handles \
         (GRBL itself refuses M6)",
    );
    ui.horizontal(|ui| {
        ComboBox::from_id_salt((salt, "tool-change"))
            .selected_text(tool_change_name(machine.tool_change))
            .show_ui(ui, |ui| {
                for change in [ToolChange::Files, ToolChange::Program] {
                    ui.selectable_value(&mut machine.tool_change, change, tool_change_name(change));
                }
            });
        if let Some(from) = from
            && from.tool_change != machine.tool_change
            && revert_mark(ui, profile, tool_change_name(from.tool_change))
        {
            machine.tool_change = from.tool_change;
        }
    });
    ui.end_row();
    ui.label("Travel X Y Z");
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
            && revert_mark(ui, profile, &travel_text(from.travel.as_ref()))
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
        ("Post", machine.post.clone()),
        ("Spindle", spindle_text(machine.spindle).to_string()),
        ("Max feed", max_feed_text(machine.max_feed)),
        (
            "Tool changes",
            tool_change_name(machine.tool_change).to_string(),
        ),
        ("Travel X Y Z", travel_text(machine.travel.as_ref())),
    ] {
        ui.label(label);
        ui.weak(value);
        ui.end_row();
    }
}

/// Safe Z: a job value, editable also while the machine is linked.
pub(super) fn safe_z_row(ui: &mut egui::Ui, label: &str, machine: &mut Machine) {
    ui.label(label)
        .on_hover_text("The height for rapid moves sideways, above the stock and its clamps");
    flag(
        ui.add(DragValue::new(&mut machine.safe_z).speed(0.1).suffix(" mm")),
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
        .on_hover_text(format!("\"{name}\" has {value}: click to take it"))
        .clicked()
}

fn spindle_text(spindle: Spindle) -> &'static str {
    match spindle {
        Spindle::Cw => "cw (M3)",
        Spindle::Ccw => "ccw (M4)",
    }
}

fn max_feed_text(max: Option<f64>) -> String {
    max.map_or("not set".into(), |f| format!("{} mm/min", len(f)))
}

fn travel_text(travel: Option<&Travel>) -> String {
    travel.map_or("not checked".into(), |t| {
        format!("{} × {} × {} mm", len(t.x), len(t.y), len(t.z))
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
            ui.label(RichText::new("Yours, copied into the project").weak());
        }
        Some(Ok(_)) => {
            ui.horizontal(|ui| {
                ui.colored_label(ui.visuals().warn_fg_color, "Your post file has changed");
                if ui
                    .button("Update")
                    .on_hover_text("Copy the post as it is now into the project")
                    .clicked()
                {
                    *request = Some(JobRequest::Post(machine.post.clone()));
                }
            });
        }
        _ => {
            ui.label(
                RichText::new("Yours; the project keeps its copy (the file isn't there)").weak(),
            );
        }
    }
    ui.end_row();
}

fn tool_change_name(change: ToolChange) -> &'static str {
    match change {
        ToolChange::Files => "one file per tool",
        ToolChange::Program => "one file with M6 (your sender handles the change)",
    }
}

/// Type, diameter, spindle speed and centre cutting.
/// `at`: the tool's path in the project (`tools.0`), for its problems; empty elsewhere.
pub(super) fn tool_rows(ui: &mut egui::Ui, salt: &str, at: &str, tool: &mut Tool) {
    ui.label("Type");
    ComboBox::from_id_salt((salt, "tool-type"))
        .selected_text(tool.kind.name())
        .show_ui(ui, |ui| {
            for kind in [ToolType::Endmill, ToolType::Drill, ToolType::Vbit] {
                ui.selectable_value(&mut tool.kind, kind, kind.name());
            }
        });
    ui.end_row();
    ui.label("Diameter");
    flag(
        ui.add(mm(&mut tool.diameter)),
        ui,
        &format!("{at}.diameter"),
    );
    ui.end_row();
    ui.label("Spindle speed");
    let mut rpm = tool.rpm.unwrap_or(0.0);
    let response = ui.add(
        DragValue::new(&mut rpm)
            .speed(100.0)
            .range(0.0..=100_000.0)
            .suffix(" rpm"),
    );
    if flag(response, ui, &format!("{at}.rpm")).changed() {
        tool.rpm = (rpm > 0.0).then_some(rpm);
    }
    ui.end_row();
    ui.label("Centre cutting");
    ComboBox::from_id_salt((salt, "center-cutting"))
        .selected_text(match tool.center_cutting {
            Some(true) => "yes",
            Some(false) => "no",
            None => "unknown",
        })
        .show_ui(ui, |ui| {
            ui.selectable_value(&mut tool.center_cutting, Some(true), "yes");
            ui.selectable_value(&mut tool.center_cutting, Some(false), "no");
            ui.selectable_value(&mut tool.center_cutting, None, "unknown");
        });
    ui.end_row();
    if tool.kind == ToolType::Vbit {
        vbit_rows(ui, at, tool);
    }
}

/// A V-bit's included angle (degrees here, radians in the file) and flat tip
/// (docs/specs/v-carve.md §2). Both can be left unset; V-carving needs the angle.
fn vbit_rows(ui: &mut egui::Ui, at: &str, tool: &mut Tool) {
    ui.label("Angle")
        .on_hover_text("The included angle of the V; V-carving needs it");
    let angle_row = ui.horizontal(|ui| {
        let mut set = tool.angle.is_some();
        if ui.checkbox(&mut set, "").changed() {
            tool.angle = set.then_some(60f64.to_radians());
        }
        if let Some(angle) = &mut tool.angle {
            let mut degrees = angle.to_degrees();
            if ui
                .add(
                    DragValue::new(&mut degrees)
                        .speed(0.5)
                        .range(1.0..=179.0)
                        .suffix("°"),
                )
                .changed()
            {
                *angle = degrees.to_radians();
            }
        } else {
            ui.weak("unknown");
        }
    });
    flag(angle_row.response, ui, &format!("{at}.angle"));
    ui.end_row();
    ui.label("Tip")
        .on_hover_text("The diameter of the flat tip; not set is taken as pointed");
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
                ui.weak("pointed");
            }
        }
    });
    ui.end_row();
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
        assert_eq!(len(12.0), "12");
        assert_eq!(len(0.25), "0.25");
        assert_eq!(len(12.3456), "12.346");
        let ramp = Entry::Ramp {
            ramp_angle: 3f64.to_radians(),
        };
        assert_eq!(entry_summary(&ramp, None), "ramp 3.0°");
        let helix = Entry::Helix {
            radius: 1.5,
            ramp_angle: 3f64.to_radians(),
        };
        let lead = Lead { radius: 2.0 };
        assert_eq!(
            entry_summary(&helix, Some(&lead)),
            "helix r 1.5 mm, 3.0°, lead 2 mm"
        );
    }
}
