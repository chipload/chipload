// SPDX-License-Identifier: GPL-3.0-or-later
//! The side panels: operations, selection (with the shape editor), properties.

use super::forms::{JobRequest, OpFormRequest, ToolChoice};
use std::collections::BTreeMap;

use super::*;

/// The Shapes section's share of the sidebar's height at first.
const SHAPES_SHARE: f32 = 0.4;
/// Neither section is dragged smaller than this, in points.
const SECTION_MIN_HEIGHT: f32 = 120.0;

/// A section's title on a shaded bar across the sidebar.
fn section_header(ui: &mut egui::Ui, title: &str) {
    egui::Frame::new()
        .fill(ui.visuals().widgets.inactive.bg_fill)
        .inner_margin(egui::Margin::symmetric(6, 4))
        .corner_radius(2.0)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.strong(title);
        });
    ui.add_space(2.0);
}

impl ChiploadApp {
    /// The selection section's shape part: the editor for a drawn shape, facts for an
    /// imported one.
    pub(super) fn selected_shape_ui(&mut self, ui: &mut egui::Ui) {
        let Some(id) = self.selected_shape() else {
            self.shape_draft = None;
            return;
        };
        let Some(shape) = self.shape(id).cloned() else {
            return;
        };
        if !shape.is_drawn() {
            for line in imported_lines(&shape) {
                ui.label(line);
            }
            return;
        }
        // Start from the stored shape unless the user is fixing values that were refused.
        let keep = self
            .shape_draft
            .as_ref()
            .is_some_and(|d| d.id == id && d.error.is_some());
        if !keep {
            self.shape_draft = Some(ShapeDraft::of(&shape));
        }
        let Some(mut draft) = self.shape_draft.take() else {
            return;
        };
        let action = ui
            .add_enabled_ui(self.can_draw(), |ui| shape_form(ui, &mut draft))
            .inner;
        if action == ShapeAction::Delete {
            self.delete_selected_shape();
            return;
        }
        if draft.differs_from(&shape) {
            draft.error = self
                .set_selected_shape(&draft.layer.clone(), draft.geometry.clone())
                .err();
        }
        self.shape_draft = Some(draft);
    }

    /// Left: the job row, the operations, and below a draggable split the shapes
    /// (docs/specs/sidebar-sections.md).
    pub(super) fn sidebar(&mut self, ui: &mut egui::Ui) {
        if self.edit.is_none() {
            ui.heading("Project");
            ui.weak("No project open.");
            return;
        }
        let job_problems = self.job_problems().count();
        let clicked = ui
            .horizontal(|ui| {
                let clicked = ui
                    .selectable_label(
                        self.focus == Focus::Job,
                        "⚙ Job settings (stock, machine, tools)",
                    )
                    .clicked();
                if job_problems > 0 {
                    ui.colored_label(ui.visuals().error_fg_color, format!("● {job_problems}"))
                        .on_hover_text("Problems in the job settings");
                }
                clicked
            })
            .inner;
        if clicked {
            self.focus = Focus::Job;
        }
        ui.add_space(4.0);
        let height = ui.available_height();
        Panel::bottom("shapes")
            .resizable(true)
            .default_size(height * SHAPES_SHARE)
            .min_size(SECTION_MIN_HEIGHT)
            .max_size((height - SECTION_MIN_HEIGHT).max(SECTION_MIN_HEIGHT))
            .frame(egui::Frame::NONE)
            .show(ui, |ui| {
                ui.add_space(6.0);
                section_header(ui, "Shapes");
                ScrollArea::vertical()
                    .id_salt("shapes-scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| self.shapes_section(ui));
            });
        let count = self.edit.as_ref().map_or(0, |e| e.ops.len());
        section_header(ui, &format!("Operations ({count})"));
        self.new_operation_buttons(ui);
        ScrollArea::vertical()
            .id_salt("operations-scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| self.operation_list(ui));
    }

    /// + Profile, + Pocket, + V-carve, + Follow path, + Holes: for the selected shapes.
    fn new_operation_buttons(&mut self, ui: &mut egui::Ui) {
        let selected = !self.selection.ids(&self.contours).is_empty();
        ui.horizontal_wrapped(|ui| {
            for (label, kind, hover) in [
                (
                    "+ Profile",
                    OpKind::Profile,
                    "A profile operation for the selected contours",
                ),
                (
                    "+ Pocket",
                    OpKind::Pocket,
                    "A pocket operation for the selected contours",
                ),
                (
                    "+ V-carve",
                    OpKind::Vcarve,
                    "A V-bit along the centre line of the selected closed contours, as deep \
                     as makes the cut as wide as the shape",
                ),
                (
                    "+ Follow path",
                    OpKind::Path,
                    "The tool on the line of the selected shapes, open or closed: engraving, \
                     grooves and score lines",
                ),
                (
                    "+ Holes",
                    OpKind::Holes,
                    "The selected circles: drilled with a drill, or bored with a helix with an \
                     endmill",
                ),
            ] {
                if ui
                    .add_enabled(selected, egui::Button::new(label))
                    .on_hover_text(hover)
                    .on_disabled_hover_text("Select shapes first")
                    .clicked()
                {
                    self.add_operation(kind);
                }
            }
        });
        ui.add_space(2.0);
    }

    /// The selection, the selected shape's editor or facts, and the layers.
    fn shapes_section(&mut self, ui: &mut egui::Ui) {
        let nothing = self.selection.ids(&self.contours).is_empty();
        for line in self.selection_lines() {
            if nothing {
                ui.weak(line);
            } else {
                ui.label(line);
            }
        }
        self.selected_shape_ui(ui);
        if ui
            .add_enabled(
                self.selector_json().is_some(),
                egui::Button::new("Copy as selector"),
            )
            .on_hover_text("The selection as JSON, for an operation in a project file")
            .clicked()
        {
            self.copy_selector(ui.ctx());
        }
        ui.add_space(4.0);
        ui.strong("Layers");
        let layers = self.layer_counts();
        if layers.is_empty() {
            ui.weak("No shapes yet.");
        }
        Grid::new("layer-list")
            .num_columns(3)
            .striped(true)
            .show(ui, |ui| {
                for (layer, count) in layers {
                    ui.label(&layer);
                    ui.weak(format!(
                        "{count} {}",
                        if count == 1 { "shape" } else { "shapes" }
                    ));
                    if ui
                        .small_button("Select")
                        .on_hover_text("Select every shape on this layer")
                        .clicked()
                    {
                        self.select_layer(&layer);
                    }
                    ui.end_row();
                }
            });
    }

    /// Every layer with the number of shapes on it, sorted by name.
    pub(super) fn layer_counts(&self) -> Vec<(String, usize)> {
        let mut layers: BTreeMap<String, usize> = BTreeMap::new();
        for c in &self.contours {
            for layer in c.layers() {
                *layers.entry(layer).or_default() += 1;
            }
        }
        layers.into_iter().collect()
    }

    pub(super) fn operation_list(&mut self, ui: &mut egui::Ui) {
        // Each operation's first problem, for its card (docs/specs/live-problems.md §3).
        let first_problem = |i: usize| {
            self.problems
                .iter()
                .find(|p| p.operation() == Some(i))
                .map(problems::plain)
        };
        // Each operation's time from the last generation (docs/specs/cycle-time.md §4).
        let op_time = |i: usize| {
            self.result.as_ref().and_then(|r| {
                r.estimate
                    .operations
                    .iter()
                    .find(|(op, _)| *op == i)
                    .map(|(_, s)| chipload_pipeline::estimate::duration_text(*s))
            })
        };
        let card_times: Vec<Option<String>> = (0..self.edit.as_ref().map_or(0, |e| e.ops.len()))
            .map(op_time)
            .collect();
        let card_problems: Vec<Option<String>> = (0..self.edit.as_ref().map_or(0, |e| e.ops.len()))
            .map(first_problem)
            .collect();
        let Some(edit) = self.edit.as_mut() else {
            return;
        };
        let missing: BTreeSet<usize> = edit.missing().into_iter().map(|(i, _)| i).collect();
        let mut action: Option<CardAction> = None;
        let count = edit.ops.len();
        // The tool of the enabled operation before: a line marks each tool change
        // (docs/specs/tool-per-operation.md §8).
        let mut last_tool: Option<i64> = None;
        for i in 0..count {
            let entry = &mut edit.ops[i];
            if entry.op.enabled() {
                let tool = entry.op.tool();
                if last_tool.is_some_and(|t| t != tool) {
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::Separator::default()
                                .horizontal()
                                .shrink(0.0)
                                .spacing(4.0),
                        );
                    });
                    ui.weak(RichText::new(format!("tool change → T{tool}")).small());
                }
                last_tool = Some(tool);
            }
            let problem = card_problems.get(i).cloned().flatten();
            let flagged = missing.contains(&i) || self.error_ops.contains(&i) || problem.is_some();
            let selected = self.focus == Focus::Op(i);
            let time = card_times.get(i).cloned().flatten();
            let state = CardState {
                selected,
                flagged,
                problem,
                time,
            };
            let card = op_card(ui, i, entry, &self.contours, state);
            let response = card.response;
            if response.clicked() {
                action = Some(CardAction::Focus(i));
            }
            response.context_menu(|ui| {
                if ui
                    .add_enabled(i > 0, egui::Button::new("Move up"))
                    .clicked()
                {
                    action = Some(CardAction::Move(i, i - 1));
                }
                if ui
                    .add_enabled(i + 1 < count, egui::Button::new("Move down"))
                    .clicked()
                {
                    action = Some(CardAction::Move(i, i + 1));
                }
                if ui.button("Duplicate").clicked() {
                    action = Some(CardAction::Duplicate(i));
                }
                if ui.button("Delete").clicked() {
                    action = Some(CardAction::Delete(i));
                }
            });
            if let Some(from) = response.dnd_release_payload::<usize>() {
                action = Some(CardAction::Move(*from, i));
            } else if response.dnd_hover_payload::<usize>().is_some() {
                ui.painter().hline(
                    response.rect.x_range(),
                    response.rect.top() - 1.0,
                    egui::Stroke::new(2.0, ui.visuals().selection.stroke.color),
                );
            }
        }
        let focused = match self.focus {
            Focus::Op(i) if i < count => Some(i),
            _ => None,
        };
        ui.horizontal(|ui| {
            if ui
                .add_enabled(focused.is_some_and(|i| i > 0), egui::Button::new("↑"))
                .on_hover_text(shortcuts::hover_keys("Move up", shortcuts::Action::MoveUp))
                .clicked()
                && let Some(i) = focused
            {
                action = Some(CardAction::Move(i, i - 1));
            }
            if ui
                .add_enabled(
                    focused.is_some_and(|i| i + 1 < count),
                    egui::Button::new("↓"),
                )
                .on_hover_text(shortcuts::hover_keys(
                    "Move down",
                    shortcuts::Action::MoveDown,
                ))
                .clicked()
                && let Some(i) = focused
            {
                action = Some(CardAction::Move(i, i + 1));
            }
            if ui
                .add_enabled(focused.is_some(), egui::Button::new("Duplicate"))
                .clicked()
                && let Some(i) = focused
            {
                action = Some(CardAction::Duplicate(i));
            }
            if ui
                .add_enabled(focused.is_some(), egui::Button::new("Delete"))
                .clicked()
                && let Some(i) = focused
            {
                action = Some(CardAction::Delete(i));
            }
        });
        match action {
            Some(CardAction::Focus(i)) => self.focus = Focus::Op(i),
            Some(CardAction::Move(from, to)) => self.move_operation(from, to),
            Some(CardAction::Duplicate(i)) => {
                self.focus = Focus::Op(i);
                self.duplicate_operation();
            }
            Some(CardAction::Delete(i)) => {
                self.focus = Focus::Op(i);
                self.delete_operation();
            }
            None => {}
        }
    }

    /// Right: the selected operation's fields, or the project settings.
    pub(super) fn properties_panel(&mut self, ui: &mut egui::Ui) {
        if self.edit.is_none() {
            ui.label("No project open.");
            return;
        }
        self.problems_strip(ui);
        problems::set_form_problems(ui, self.form_problem_map(self.focus));
        let Some(edit) = self.edit.as_mut() else {
            return;
        };
        let focus = self.focus;
        let contours = &self.contours;
        let library = self.settings_dir.as_ref().map(|_| &self.library);
        let mut op_request = OpFormRequest::default();
        let mut request = None;
        let cutting = match focus {
            Focus::Op(i) => edit
                .ops
                .get(i)
                .and_then(|e| self_tool_cutting(library, &edit.settings.tools, e.op.tool())),
            Focus::Job => None,
        };
        ui.scope(|ui| match focus {
            Focus::Job => request = job_form(ui, edit, library),
            Focus::Op(i) => {
                let choice = ToolChoice {
                    tools: &edit.settings.tools,
                    library: library
                        .map(|l| l.tools.tools.iter().map(|t| t.name.clone()).collect()),
                    cutting,
                };
                if let Some(entry) = edit.ops.get_mut(i) {
                    op_request = op_form(ui, i, entry, contours, choice);
                }
            }
        });
        // The map was for this form only (library windows draw the same rows).
        problems::set_form_problems(ui, std::collections::BTreeMap::new());
        if op_request.use_selection {
            self.use_selection();
        }
        if let (Some(name), Focus::Op(i)) = (op_request.tool_from_library, focus) {
            self.tool_from_library(i, &name);
        }
        match request {
            Some(JobRequest::Library(request)) => self.library_request(request),
            Some(JobRequest::CopiesGap(gap)) => self.set_copies_gap(gap),
            Some(JobRequest::Post(name)) => {
                self.choose_project_post(&name);
            }
            Some(JobRequest::Place(how)) => {
                self.place(how);
            }
            None => {}
        }
    }
}

/// A project tool's cutting values from the library: only when the tool came from the
/// library and still has its values (as `ChiploadApp::tool_cutting`).
fn self_tool_cutting(
    library: Option<&chipload_pipeline::library::Library>,
    tools: &[chipload_job::Tool],
    id: i64,
) -> Option<chipload_job::library::Cutting> {
    let tool = tools.iter().find(|t| t.id == id)?;
    let entry = library?.tools.find(tool.name.as_deref()?)?;
    if entry.same_values(tool) {
        entry.cutting
    } else {
        None
    }
}

impl ChiploadApp {
    /// With no project open, the canvas offers the ways to start one, and recent projects.
    pub(super) fn start_screen(&mut self, ui: &mut egui::Ui) {
        // The canvas's own background, so the window doesn't change colour on opening.
        ui.painter()
            .rect_filled(ui.max_rect(), 0.0, crate::view::BACKGROUND);
        ui.add_space((ui.available_height() * 0.18).max(16.0));
        ui.vertical_centered(|ui| {
            egui::Frame::new()
                .fill(ui.visuals().panel_fill)
                .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
                .corner_radius(8.0)
                .inner_margin(24.0)
                .show(ui, |ui| {
                    ui.set_max_width(START_WIDTH);
                    ui.heading(RichText::new("Chipload").size(28.0).strong());
                    ui.weak("2.5D CAM: drawings in, checked G-code out");
                    ui.add_space(16.0);
                    let wide = egui::vec2(START_WIDTH, 32.0);
                    if ui
                        .add(egui::Button::new("New project").min_size(wide))
                        .on_hover_text("An empty project: draw shapes or import a DXF later")
                        .clicked()
                    {
                        self.ask_new();
                    }
                    if ui
                        .add(egui::Button::new("Import DXF…").min_size(wide))
                        .on_hover_text("A new project from a drawing (Ctrl+I)")
                        .clicked()
                    {
                        self.ask_import();
                    }
                    if ui
                        .add(egui::Button::new("Open project…").min_size(wide))
                        .on_hover_text("Ctrl+O")
                        .clicked()
                    {
                        self.ask_open();
                    }
                    let recent: Vec<String> =
                        self.recent.iter().take(START_RECENT).cloned().collect();
                    if !recent.is_empty() {
                        ui.add_space(16.0);
                        ui.label(RichText::new("Recent").strong());
                        for path in &recent {
                            let name = file_name(Path::new(path));
                            if ui
                                .add(egui::Button::new(name).frame(false))
                                .on_hover_text(path)
                                .clicked()
                            {
                                self.open_recent(path);
                            }
                        }
                    }
                    ui.add_space(16.0);
                    ui.label(
                        RichText::new(
                            "⚠ Machines are dangerous. Check every program in a G-code viewer \
                             and with an air cut before cutting material.",
                        )
                        .small()
                        .color(ui.visuals().warn_fg_color),
                    );
                });
        });
    }
}

/// Width of the start screen's card.
const START_WIDTH: f32 = 320.0;
/// Recent projects shown on the start screen (all are under File > Open recent).
const START_RECENT: usize = 6;

/// What a click in the operation list asks for, done after the list is drawn.
enum CardAction {
    Focus(usize),
    Move(usize, usize),
    Duplicate(usize),
    Delete(usize),
}

/// The kind's tag colour: pockets and profiles told apart at a glance.
fn kind_color(op: &Operation, dark: bool) -> egui::Color32 {
    match (op, dark) {
        (Operation::Profile(_), true) => egui::Color32::from_rgb(0x6f, 0xc8, 0xbd),
        (Operation::Profile(_), false) => egui::Color32::from_rgb(0x00, 0x79, 0x6b),
        (Operation::Pocket(_), true) => egui::Color32::from_rgb(0xc3, 0xa6, 0xf0),
        (Operation::Pocket(_), false) => egui::Color32::from_rgb(0x6a, 0x3d, 0xb8),
        (Operation::Vcarve(_), true) => egui::Color32::from_rgb(0xf0, 0xb4, 0x6a),
        (Operation::Vcarve(_), false) => egui::Color32::from_rgb(0xa8, 0x55, 0x00),
        (Operation::Path(_), true) => egui::Color32::from_rgb(0x8f, 0xbc, 0xf5),
        (Operation::Path(_), false) => egui::Color32::from_rgb(0x1f, 0x5c, 0xb0),
        (Operation::Holes(_), true) => egui::Color32::from_rgb(0xf0, 0x9a, 0xb4),
        (Operation::Holes(_), false) => egui::Color32::from_rgb(0xa8, 0x1e, 0x55),
    }
}

/// One operation as a card: drag handle, enabled box, number, kind and name; below, its
/// geometry and depths. The whole card is clickable (selects it) and right-clickable.
/// How an operation card shows: selected, marked, its first problem and its time.
struct CardState {
    selected: bool,
    flagged: bool,
    problem: Option<String>,
    time: Option<String>,
}

fn op_card(
    ui: &mut egui::Ui,
    i: usize,
    entry: &mut crate::editor::EditOp,
    contours: &[Contour],
    state: CardState,
) -> egui::InnerResponse<()> {
    let CardState {
        selected,
        flagged,
        problem,
        time,
    } = state;
    let summary = geometry_summary(&entry.op, contours);
    let (start, end) = entry.op.depths();
    let kind = match &entry.op {
        Operation::Profile(_) => "PROFILE",
        Operation::Pocket(_) => "POCKET",
        Operation::Vcarve(_) => "V-CARVE",
        Operation::Path(_) => "PATH",
        Operation::Holes(_) => "HOLES",
    };
    ui.scope_builder(
        egui::UiBuilder::new()
            .id_salt(("op-card", i))
            .sense(Sense::click()),
        |ui| {
            let hovered = ui.response().hovered();
            let visuals = ui.visuals().clone();
            let fill = if selected {
                visuals.selection.bg_fill
            } else if hovered {
                crate::style::card_hover(&visuals)
            } else {
                visuals.faint_bg_color
            };
            let text_color = if selected {
                visuals.selection.stroke.color
            } else {
                visuals.text_color()
            };
            egui::Frame::new()
                .fill(fill)
                .corner_radius(4.0)
                .inner_margin(egui::Margin::symmetric(6, 4))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.dnd_drag_source(egui::Id::new(("operation", i)), i, |ui| {
                            ui.label(RichText::new("≡").color(text_color))
                                .on_hover_text("Drag onto another operation to move it there");
                        });
                        let mut enabled = entry.op.enabled();
                        if ui
                            .add(egui::Checkbox::without_text(&mut enabled))
                            .on_hover_text("Enabled: cut this operation")
                            .changed()
                        {
                            entry.op.set_enabled(enabled);
                        }
                        ui.label(RichText::new(format!("{}", i + 1)).color(text_color));
                        let tag = if selected {
                            text_color
                        } else {
                            kind_color(&entry.op, visuals.dark_mode)
                        };
                        ui.label(RichText::new(kind).small().strong().color(tag));
                        let name = match entry.op.name() {
                            "" => "(no name)".to_string(),
                            name => name.to_string(),
                        };
                        let mut title = RichText::new(name).strong().color(text_color);
                        if flagged {
                            title = RichText::new(format!("⚠ {}", entry.op.name()))
                                .strong()
                                .color(visuals.error_fg_color);
                        }
                        if !enabled {
                            title = title.weak();
                        }
                        ui.add(egui::Label::new(title).truncate().selectable(false));
                    });
                    let tool = entry.op.tool();
                    let time = time.map_or(String::new(), |t| format!("  ·  {t}"));
                    // docs/specs/rest-machining.md §6.
                    let rest = match &entry.op {
                        Operation::Pocket(p) => p
                            .rest
                            .map_or(String::new(), |r| format!(" rest after T{}", r.after_tool)),
                        _ => String::new(),
                    };
                    let mut detail = RichText::new(format!(
                        "T{tool}{rest}  ·  {summary}  ·  {start}–{end} mm{time}"
                    ))
                    .small();
                    detail = if selected {
                        detail.color(text_color)
                    } else {
                        detail.weak()
                    };
                    ui.horizontal(|ui| {
                        ui.add_space(22.0);
                        ui.add(egui::Label::new(detail).truncate().selectable(false));
                    });
                    if let Some(problem) = problem {
                        let text = RichText::new(problem).small().color(visuals.error_fg_color);
                        ui.horizontal(|ui| {
                            ui.add_space(22.0);
                            ui.add(egui::Label::new(text).truncate().selectable(false));
                        });
                    }
                });
        },
    )
}
