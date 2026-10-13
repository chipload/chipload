// SPDX-License-Identifier: GPL-3.0-or-later
//! The Browser (docs/specs/browser-and-layout.md §2, §3): the project as one tree, with
//! Job settings, Tools, Drawing and Operations as folders. Rows are flat and one line: a
//! click selects, a double-click edits, a right-click offers the rest.

use eframe::egui::{Color32, Id, Rect, Response, Stroke, WidgetInfo, WidgetType, vec2};

use super::shortcuts::with_keys;
use super::*;
use crate::icons::{self, Icon};

/// A row's height, points.
pub(super) const ROW_HEIGHT: f32 = 22.0;
/// Indent per level of the tree, points.
const INDENT: f32 = 14.0;
/// An icon's square, points.
const ICON: f32 = 14.0;
/// Room between a row's parts, points.
const GAP: f32 = 4.0;
/// Room at the row's left and right ends, points.
const PAD: f32 = 4.0;

/// One row of the tree, built with its parts and shown with `show`.
pub(super) struct Row {
    id: Id,
    depth: usize,
    open: Option<bool>,
    eye: Option<bool>,
    tick: Option<bool>,
    icon: Option<(Icon, Option<Color32>)>,
    label: String,
    trailing: String,
    selected: bool,
    dim: bool,
    alert: bool,
    drag: bool,
    strong: bool,
}

/// What happened to a row this frame.
pub(super) struct RowResponse {
    /// The whole row: clicks, double-clicks, the context menu, hover text.
    pub response: Response,
    /// Its triangle was clicked (or the folder row itself, for a plain folder).
    pub toggled: bool,
    /// Its eye was clicked.
    pub eye: bool,
    /// Its tick was clicked.
    pub tick: bool,
}

impl Row {
    pub(super) fn new(id: Id, depth: usize, label: impl Into<String>) -> Self {
        Self {
            id,
            depth,
            open: None,
            eye: None,
            tick: None,
            icon: None,
            label: label.into(),
            trailing: String::new(),
            selected: false,
            dim: false,
            alert: false,
            drag: false,
            strong: false,
        }
    }

    /// The label in bold: the active part.
    pub(super) fn strong(mut self, strong: bool) -> Self {
        self.strong = strong;
        self
    }

    /// A folder: open or closed, with a triangle.
    pub(super) fn folder(mut self, open: bool) -> Self {
        self.open = Some(open);
        self
    }

    /// An eye: shown or hidden on the canvas.
    pub(super) fn eye(mut self, shown: bool) -> Self {
        self.eye = Some(shown);
        self
    }

    /// A tick: an operation that is cut or not.
    pub(super) fn tick(mut self, on: bool) -> Self {
        self.tick = Some(on);
        self
    }

    pub(super) fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some((icon, None));
        self
    }

    pub(super) fn icon_colored(mut self, icon: Icon, color: Color32) -> Self {
        self.icon = Some((icon, Some(color)));
        self
    }

    /// Weak text at the row's right end: a count, a tool.
    pub(super) fn trailing(mut self, text: impl Into<String>) -> Self {
        self.trailing = text.into();
        self
    }

    pub(super) fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// Dimmed: hidden, or an operation that isn't cut.
    pub(super) fn dim(mut self, dim: bool) -> Self {
        self.dim = dim;
        self
    }

    /// A problem: the name in the error colour and a dot at the right end.
    pub(super) fn alert(mut self, alert: bool) -> Self {
        self.alert = alert;
        self
    }

    /// The row can be dragged (operations, to reorder them).
    pub(super) fn draggable(mut self) -> Self {
        self.drag = true;
        self
    }

    pub(super) fn show(self, ui: &mut egui::Ui) -> RowResponse {
        let width = ui.available_width();
        let (rect, _) = ui.allocate_exact_size(vec2(width, ROW_HEIGHT), Sense::hover());
        let sense = if self.drag {
            Sense::click_and_drag()
        } else {
            Sense::click()
        };
        let response = ui.interact(rect, self.id, sense);
        let label = self.label.clone();
        let selected = self.selected;
        response.widget_info(|| {
            WidgetInfo::selected(WidgetType::SelectableLabel, true, selected, &label)
        });
        let visuals = ui.visuals().clone();
        let painter = ui.painter_at(rect);
        if self.selected {
            painter.rect_filled(rect, 0.0, visuals.selection.bg_fill);
        } else if response.hovered() {
            painter.rect_filled(rect, 0.0, crate::style::card_hover(&visuals));
        }
        let text_color = if self.selected {
            visuals.selection.stroke.color
        } else if self.alert {
            visuals.error_fg_color
        } else if self.dim {
            visuals.weak_text_color()
        } else {
            visuals.text_color()
        };
        let line_color = if self.selected {
            text_color
        } else {
            visuals.weak_text_color()
        };
        // Indent guides: a thin line under each open folder above.
        let guide = Stroke::new(1.0, visuals.widgets.noninteractive.bg_stroke.color);
        for level in 1..=self.depth {
            let x = rect.left() + PAD + (level as f32 - 0.5) * INDENT;
            painter.vline(x.round() + 0.5, rect.y_range(), guide);
        }
        let mut x = rect.left() + PAD + self.depth as f32 * INDENT;
        let slot =
            |x: f32, w: f32| Rect::from_min_size(egui::pos2(x, rect.top()), vec2(w, rect.height()));
        let mut toggled = false;
        if let Some(open) = self.open {
            let area = slot(x, INDENT);
            let hit = ui.interact(area, self.id.with("open"), Sense::click());
            icons::chevron(&painter, area, open, line_color);
            toggled = hit.clicked();
        }
        x += INDENT;
        let mut eye = false;
        if let Some(shown) = self.eye {
            let area = slot(x, ICON);
            let hit = ui
                .interact(area, self.id.with("eye"), Sense::click())
                .on_hover_text(if shown {
                    "Hide it on the canvas; it is still cut"
                } else {
                    "Hidden on the canvas (still cut): show it"
                });
            let color = if hit.hovered() {
                text_color
            } else {
                line_color
            };
            let icon = if shown { Icon::Eye } else { Icon::EyeOff };
            icons::paint(
                &painter,
                area.shrink2(vec2(0.0, (ROW_HEIGHT - ICON) / 2.0)),
                icon,
                color,
            );
            eye = hit.clicked();
            x += ICON + GAP;
        }
        let mut tick = false;
        if let Some(on) = self.tick {
            let area = slot(x, ICON);
            let hit = ui
                .interact(area, self.id.with("tick"), Sense::click())
                .on_hover_text(if on {
                    "Cut: untick to leave it out of the program"
                } else {
                    "Not cut: tick to cut it"
                });
            let fill = if self.selected {
                visuals.selection.bg_fill
            } else {
                visuals.extreme_bg_color
            };
            icons::tick_box(&painter, area, on, line_color, fill);
            tick = hit.clicked();
            x += ICON + GAP;
        }
        if let Some((icon, color)) = self.icon {
            let area = Rect::from_center_size(
                egui::pos2(x + ICON / 2.0, rect.center().y),
                vec2(ICON, ICON),
            );
            let color = match color {
                Some(c) if !self.selected && !self.dim => c,
                _ => line_color,
            };
            icons::paint(&painter, area, icon, color);
            x += ICON + GAP + 2.0;
        }
        // The scroll area's bar may cover the row's right end: stop short of it.
        let mut right = rect.right().min(ui.clip_rect().right()) - PAD;
        if self.alert {
            let dot = egui::pos2(right - 4.0, rect.center().y);
            painter.circle_filled(dot, 3.5, visuals.error_fg_color);
            right -= 8.0 + GAP;
        }
        if !self.trailing.is_empty() {
            let galley = egui::WidgetText::from(RichText::new(&self.trailing).small()).into_galley(
                ui,
                Some(egui::TextWrapMode::Extend),
                f32::INFINITY,
                egui::TextStyle::Small,
            );
            let at = egui::pos2(
                right - galley.size().x,
                rect.center().y - galley.size().y / 2.0,
            );
            painter.galley(at, galley.clone(), line_color);
            right -= galley.size().x + GAP * 2.0;
        }
        let mut label = RichText::new(&self.label);
        if self.strong {
            label = label.strong();
        }
        let galley = egui::WidgetText::from(label).into_galley(
            ui,
            Some(egui::TextWrapMode::Truncate),
            (right - x).max(0.0),
            egui::TextStyle::Body,
        );
        painter.galley(
            egui::pos2(x, rect.center().y - galley.size().y / 2.0),
            galley,
            text_color,
        );
        // A plain folder (no other click meaning) opens and closes on a click anywhere.
        RowResponse {
            response,
            toggled,
            eye,
            tick,
        }
    }
}

/// Whether the folder `id` is open (remembered between frames and sessions).
pub(super) fn folder_open(ui: &egui::Ui, id: Id, default: bool) -> bool {
    ui.data_mut(|d| *d.get_persisted_mut_or(id, default))
}

/// Open or close the folder `id`.
pub(super) fn toggle_folder(ui: &egui::Ui, id: Id, default: bool) {
    ui.data_mut(|d| {
        let open = d.get_persisted_mut_or(id, default);
        *open = !*open;
    });
}

/// A folder row: its triangle or a click on it opens and closes it. Returns whether it is
/// open after this frame, and the row's response.
pub(super) fn folder(ui: &mut egui::Ui, row: Row, default: bool) -> (bool, RowResponse) {
    let id = row.id.with("folder-open");
    let open = folder_open(ui, id, default);
    let out = row.folder(open).show(ui);
    if out.toggled || out.response.clicked() {
        toggle_folder(ui, id, default);
        return (!open, out);
    }
    (open, out)
}

/// A small capital caption over a panel: "BROWSER", "PROPERTIES".
pub(super) fn caption(ui: &mut egui::Ui, text: &str) {
    ui.add_space(2.0);
    ui.label(
        RichText::new(text.to_uppercase())
            .small()
            .color(ui.visuals().weak_text_color()),
    );
    ui.add_space(2.0);
}

/// The Properties panel's title: an icon and what the panel shows.
pub(super) fn title_line(ui: &mut egui::Ui, icon: Icon, color: Option<Color32>, text: &str) {
    ui.horizontal(|ui| {
        let size = ui.text_style_height(&egui::TextStyle::Heading);
        let (rect, _) = ui.allocate_exact_size(vec2(size, size), Sense::hover());
        let color = color.unwrap_or_else(|| ui.visuals().text_color());
        icons::paint(ui.painter(), rect.shrink(1.0), icon, color);
        ui.heading(text);
    });
    ui.add_space(2.0);
}

/// The icon of an operation's kind.
pub(super) fn op_icon(op: &Operation) -> Icon {
    match op {
        Operation::Profile(_) => Icon::Profile,
        Operation::Pocket(_) => Icon::Pocket,
        Operation::Vcarve(_) => Icon::Vcarve,
        Operation::Path(_) => Icon::FollowPath,
        Operation::Holes(_) => Icon::Holes,
    }
}

/// The icon of a tool's kind.
pub(super) fn tool_icon(kind: ToolType) -> Icon {
    match kind {
        ToolType::Endmill => Icon::Endmill,
        ToolType::Drill => Icon::Drill,
        ToolType::Vbit => Icon::Vbit,
    }
}

/// What a click in the operation rows asks for, done after they are drawn.
enum OpAction {
    Focus(usize),
    Toggle(usize),
    Move(usize, usize),
    Duplicate(usize),
    Delete(usize),
    /// A rest pocket for pocket i with this tool (docs/specs/rest-machining.md §10).
    AddRest(usize, i64),
    /// Into part `id`, at the end of its operations (dropped on the part's row).
    ToPart(usize, u64),
    PartFocus(u64),
    PartActivate(u64),
    PartToggle(u64),
    PartMove(u64, bool),
    PartDelete(u64),
    /// A new operation of this kind in part `id`: it becomes the active part first.
    NewIn(OpKind, u64),
    NewPart,
}

/// The kinds of new operation: label, kind and what it does.
pub(super) const NEW_OPERATIONS: [(&str, OpKind, &str); 5] = [
    (
        "Profile",
        OpKind::Profile,
        "A profile operation for the selected contours",
    ),
    (
        "Pocket",
        OpKind::Pocket,
        "A pocket operation for the selected contours",
    ),
    (
        "V-carve",
        OpKind::Vcarve,
        "A V-bit along the centre line of the selected closed contours, as deep as makes the \
         cut as wide as the shape",
    ),
    (
        "Follow path",
        OpKind::Path,
        "The tool on the line of the selected shapes, open or closed: engraving, grooves and \
         score lines",
    ),
    (
        "Holes",
        OpKind::Holes,
        "The selected circles: drilled with a drill, or bored with a helix with an endmill",
    ),
];

impl ChiploadApp {
    /// Left: the Browser.
    pub(super) fn sidebar(&mut self, ui: &mut egui::Ui) {
        caption(ui, "Browser");
        if self.edit.is_none() {
            ui.weak("No project open.");
            return;
        }
        ScrollArea::vertical()
            .id_salt("browser-scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                self.browser_tree(ui);
            });
    }

    fn browser_tree(&mut self, ui: &mut egui::Ui) {
        let name = self
            .job_path
            .as_deref()
            .map_or("New project".to_string(), file_name);
        let root = Row::new(Id::new("browser-project"), 0, name).icon(Icon::Project);
        let (open, _) = folder(ui, root, true);
        if !open {
            return;
        }
        let problems = self.job_problems().count();
        let job = Row::new(Id::new("browser-job"), 1, "Job settings")
            .icon(Icon::Settings)
            .selected(self.focus == Focus::Job)
            .alert(problems > 0)
            .show(ui);
        let job = if problems > 0 {
            job.response.on_hover_text(format!(
                "Stock, machine, post-processor and tools. {problems} {}",
                if problems == 1 { "problem" } else { "problems" }
            ))
        } else {
            job.response
                .on_hover_text("Stock, machine, post-processor and tools")
        };
        if job.clicked() {
            self.focus = Focus::Job;
        }
        self.tool_rows(ui);
        self.drawing_rows(ui);
        self.operation_rows(ui);
    }

    /// Tools: the project's tools; a click opens the job settings, where they are edited.
    fn tool_rows(&mut self, ui: &mut egui::Ui) {
        let Some(edit) = &self.edit else {
            return;
        };
        let tools = edit.settings.tools.clone();
        let row = Row::new(Id::new("browser-tools"), 1, "Tools")
            .icon(Icon::Tools)
            .trailing(tools.len().to_string());
        let (open, _) = folder(ui, row, false);
        if !open {
            return;
        }
        for tool in &tools {
            let row = Row::new(
                Id::new(("browser-tool", tool.id)),
                2,
                forms::tool_label(tool),
            )
            .icon(tool_icon(tool.kind))
            .show(ui);
            if row
                .response
                .on_hover_text("Edited in the job settings")
                .clicked()
            {
                self.focus = Focus::Job;
            }
        }
    }

    /// Parts (docs/specs/parts.md §8): each part with its operations, in cutting order,
    /// with a line at each tool change.
    fn operation_rows(&mut self, ui: &mut egui::Ui) {
        let Some(edit) = self.edit.as_ref() else {
            return;
        };
        let parts = edit.settings.parts.clone();
        let active = edit.settings.active_part_id();
        let count = edit.ops.len();
        let op_parts: Vec<u64> = edit.ops.iter().map(|e| e.op.part()).collect();
        let row = Row::new(Id::new("browser-parts"), 1, "Parts")
            .icon(Icon::Parts)
            .trailing(parts.len().to_string());
        let (open, folder_row) = folder(ui, row, true);
        let mut action: Option<OpAction> = None;
        folder_row
            .response
            .on_hover_text(
                "Each part is cut with its copies before the next; right-click for a new part",
            )
            .context_menu(|ui| {
                if ui.button("New part").clicked() {
                    action = Some(OpAction::NewPart);
                }
            });
        if open {
            for (k, part) in parts.iter().enumerate() {
                if let Some(a) = self.part_rows(ui, part, k, &parts, active, &op_parts, count) {
                    action = Some(a);
                }
            }
        }
        match action {
            Some(OpAction::Focus(i)) => self.focus = Focus::Op(i),
            Some(OpAction::Toggle(i)) => {
                if let Some(entry) = self.edit.as_mut().and_then(|e| e.ops.get_mut(i)) {
                    let enabled = entry.op.enabled();
                    entry.op.set_enabled(!enabled);
                }
            }
            Some(OpAction::Move(from, to)) => self.move_operation(from, to),
            Some(OpAction::Duplicate(i)) => {
                self.focus = Focus::Op(i);
                self.duplicate_operation();
            }
            Some(OpAction::Delete(i)) => {
                self.focus = Focus::Op(i);
                self.delete_operation();
            }
            Some(OpAction::AddRest(i, tool)) => self.add_rest_pocket(i, tool),
            Some(OpAction::ToPart(from, id)) => self.move_operation_to_part(from, id),
            Some(OpAction::PartFocus(id)) => self.focus = Focus::Part(id),
            Some(OpAction::PartActivate(id)) => self.activate_part(id),
            Some(OpAction::PartToggle(id)) => self.toggle_part(id),
            Some(OpAction::PartMove(id, up)) => self.move_part(id, up),
            Some(OpAction::PartDelete(id)) => self.delete_part(id),
            Some(OpAction::NewIn(kind, id)) => {
                self.activate_part(id);
                self.add_operation(kind);
            }
            Some(OpAction::NewPart) => {
                self.add_part();
            }
            None => {}
        }
    }

    /// Part `part`'s row (the `k`th part) and, when it is open, its operations.
    #[allow(clippy::too_many_arguments)]
    fn part_rows(
        &mut self,
        ui: &mut egui::Ui,
        part: &chipload_job::Part,
        k: usize,
        parts: &[chipload_job::Part],
        active: Option<u64>,
        op_parts: &[u64],
        count: usize,
    ) -> Option<OpAction> {
        let mine: Vec<usize> = (0..count).filter(|i| op_parts[*i] == part.id).collect();
        let flagged = mine.iter().any(|i| {
            self.problems.iter().any(|p| p.operation() == Some(*i)) || self.error_ops.contains(i)
        }) || !self.form_problem_map(Focus::Part(part.id)).is_empty();
        let is_active = active == Some(part.id);
        let mut trailing = Vec::new();
        if is_active && parts.len() > 1 {
            trailing.push("active".to_string());
        }
        if part.count() > 1 {
            trailing.push(format!("×{}", part.count()));
        }
        let id = Id::new(("browser-part", part.id));
        let open_id = id.with("folder-open");
        let open = folder_open(ui, open_id, true);
        let out = Row::new(id, 2, &part.name)
            .folder(open)
            .tick(part.enabled)
            .icon(Icon::Part)
            .trailing(trailing.join(" · "))
            .selected(self.focus == Focus::Part(part.id))
            .dim(!part.enabled)
            .alert(flagged)
            .strong(is_active && parts.len() > 1)
            .show(ui);
        let mut action = None;
        if out.toggled {
            toggle_folder(ui, open_id, true);
        }
        if out.tick {
            action = Some(OpAction::PartToggle(part.id));
        } else if out.response.double_clicked() {
            action = Some(OpAction::PartActivate(part.id));
        } else if out.response.clicked() {
            action = Some(OpAction::PartFocus(part.id));
        }
        if let Some(from) = out.response.dnd_release_payload::<usize>() {
            action = Some(OpAction::ToPart(*from, part.id));
        } else if out.response.dnd_hover_payload::<usize>().is_some() {
            ui.painter().rect_stroke(
                out.response.rect,
                0.0,
                Stroke::new(2.0, ui.visuals().selection.stroke.color),
                egui::StrokeKind::Inside,
            );
        }
        let selected = !self.selection.ids(&self.contours).is_empty();
        let hover = if is_active {
            "The active part: new operations go here. Click for its copies; right-click for more"
        } else {
            "Click for its copies; double-click to make it the active part; right-click for more"
        };
        out.response.on_hover_text(hover).context_menu(|ui| {
            if ui
                .add_enabled(!is_active, egui::Button::new("Activate"))
                .on_hover_text("New operations go into the active part")
                .clicked()
            {
                action = Some(OpAction::PartActivate(part.id));
            }
            ui.menu_button("New operation", |ui| {
                for (label, kind, hover) in NEW_OPERATIONS {
                    if ui
                        .add_enabled(selected, egui::Button::new(label))
                        .on_hover_text(hover)
                        .on_disabled_hover_text("Select shapes first")
                        .clicked()
                    {
                        action = Some(OpAction::NewIn(kind, part.id));
                    }
                }
            });
            ui.separator();
            if ui
                .add_enabled(k > 0, egui::Button::new("Move up"))
                .clicked()
            {
                action = Some(OpAction::PartMove(part.id, true));
            }
            if ui
                .add_enabled(k + 1 < parts.len(), egui::Button::new("Move down"))
                .clicked()
            {
                action = Some(OpAction::PartMove(part.id, false));
            }
            ui.separator();
            let what = match mine.len() {
                0 => "Delete part".to_string(),
                1 => "Delete part and its operation".to_string(),
                n => format!("Delete part and its {n} operations"),
            };
            if ui
                .add_enabled(parts.len() > 1, egui::Button::new(what))
                .on_disabled_hover_text("A project needs at least one part")
                .clicked()
            {
                action = Some(OpAction::PartDelete(part.id));
            }
        });
        if !open {
            return action;
        }
        if mine.is_empty() {
            ui.horizontal(|ui| {
                ui.add_space(PAD + 4.0 * INDENT);
                ui.weak(if is_active {
                    "Select shapes, then add an operation"
                } else {
                    "No operations: drag one here"
                });
            });
        }
        for i in mine {
            if let Some(a) = self.operation_row(ui, i, count) {
                action = Some(a);
            }
        }
        action
    }

    /// Operation `i`'s row (and the tool change line above it).
    fn operation_row(&mut self, ui: &mut egui::Ui, i: usize, count: usize) -> Option<OpAction> {
        let edit = self.edit.as_ref()?;
        let entry = edit.ops.get(i)?;
        let op = &entry.op;
        // An operation is cut when it and its part are on (docs/specs/parts.md §3).
        let cut = |op: &Operation| {
            op.enabled() && edit.settings.part(op.part()).is_some_and(|p| p.enabled)
        };
        // A line marks each tool change (docs/specs/tool-per-operation.md §8).
        let previous = edit.ops[..i]
            .iter()
            .rev()
            .find(|e| cut(&e.op))
            .map(|e| e.op.tool());
        let is_cut = cut(op);
        if is_cut && previous.is_some_and(|t| t != op.tool()) {
            tool_change(ui, op.tool());
        }
        let problem = self
            .problems
            .iter()
            .find(|p| p.operation() == Some(i))
            .map(problems::plain);
        let missing = edit.missing().iter().any(|(m, _)| *m == i);
        let flagged = missing || self.error_ops.contains(&i) || problem.is_some();
        let hidden = self.hidden_cut(op);
        // Smaller endmills a rest pocket for this pocket can use
        // (docs/specs/rest-machining.md §10).
        let rest_tools: Vec<(i64, f64)> = match op {
            Operation::Pocket(p) if p.rest.is_none() => {
                let diameter = edit
                    .settings
                    .tools
                    .iter()
                    .find(|t| t.id == p.tool)
                    .map(|t| t.diameter);
                edit.settings
                    .tools
                    .iter()
                    .filter(|t| {
                        t.kind == ToolType::Endmill && diameter.is_some_and(|d| t.diameter < d)
                    })
                    .map(|t| (t.id, t.diameter))
                    .collect()
            }
            _ => Vec::new(),
        };
        let name = match op.name() {
            "" => "(no name)",
            name => name,
        };
        let trailing = if hidden > 0 {
            format!("{hidden} hidden · T{}", op.tool())
        } else {
            format!("T{}", op.tool())
        };
        let hover = self.operation_hover(i, problem.as_deref(), hidden);
        let dark = ui.visuals().dark_mode;
        let row = Row::new(Id::new(("browser-op", i)), 3, format!("{}  {name}", i + 1))
            .tick(op.enabled())
            .icon_colored(op_icon(op), kind_color(op, dark))
            .trailing(trailing)
            .selected(self.focus == Focus::Op(i))
            .dim(!is_cut)
            .alert(flagged)
            .draggable()
            .show(ui);
        let mut action = None;
        if row.tick {
            action = Some(OpAction::Toggle(i));
        } else if row.response.clicked() {
            action = Some(OpAction::Focus(i));
        }
        row.response.dnd_set_drag_payload(i);
        if let Some(from) = row.response.dnd_release_payload::<usize>() {
            action = Some(OpAction::Move(*from, i));
        } else if row.response.dnd_hover_payload::<usize>().is_some() {
            ui.painter().hline(
                row.response.rect.x_range(),
                row.response.rect.top(),
                Stroke::new(2.0, ui.visuals().selection.stroke.color),
            );
        }
        let response = row.response.on_hover_text(hover);
        response.context_menu(|ui| {
            if ui
                .add_enabled(
                    i > 0,
                    egui::Button::new(with_keys("Move up", shortcuts::Action::MoveUp)),
                )
                .clicked()
            {
                action = Some(OpAction::Move(i, i - 1));
            }
            if ui
                .add_enabled(
                    i + 1 < count,
                    egui::Button::new(with_keys("Move down", shortcuts::Action::MoveDown)),
                )
                .clicked()
            {
                action = Some(OpAction::Move(i, i + 1));
            }
            if ui.button("Duplicate").clicked() {
                action = Some(OpAction::Duplicate(i));
            }
            if !rest_tools.is_empty() {
                ui.menu_button("Add rest pocket", |ui| {
                    for (tool, diameter) in &rest_tools {
                        let label = format!("T{tool} · {diameter} mm endmill");
                        if ui.button(label).clicked() {
                            action = Some(OpAction::AddRest(i, *tool));
                        }
                    }
                })
                .response
                .on_hover_text(
                    "A pocket after this one with the same shapes and depths, cut with a \
                     smaller tool, that clears only what this pocket's tool leaves",
                );
            }
            ui.separator();
            if ui.button("Delete").clicked() {
                action = Some(OpAction::Delete(i));
            }
        });
        action
    }

    /// An operation row's hover text: its kind, tool, geometry, depths and time, then its
    /// first problem and the hidden contours it cuts.
    fn operation_hover(&self, i: usize, problem: Option<&str>, hidden: usize) -> String {
        let Some(entry) = self.edit.as_ref().and_then(|e| e.ops.get(i)) else {
            return String::new();
        };
        let op = &entry.op;
        let kind = match op {
            Operation::Profile(_) => "Profile",
            Operation::Pocket(_) => "Pocket",
            Operation::Vcarve(_) => "V-carve",
            Operation::Path(_) => "Follow path",
            Operation::Holes(_) => "Holes",
        };
        let summary = geometry_summary(op, &self.contours);
        let (start, end) = op.depths();
        // docs/specs/rest-machining.md §6.
        let rest = match op {
            Operation::Pocket(p) => p
                .rest
                .map_or(String::new(), |r| format!(", rest after T{}", r.after_tool)),
            _ => String::new(),
        };
        let time = self
            .result
            .as_ref()
            .and_then(|r| r.estimate.operations.iter().find(|(o, _)| *o == i))
            .map_or(String::new(), |(_, s)| {
                format!(" · {}", chipload_pipeline::estimate::duration_text(*s))
            });
        let mut lines = vec![format!(
            "{kind} · T{}{rest} · {summary} · {start}–{end} mm{time}",
            op.tool()
        )];
        if !op.enabled() {
            lines.push("Not cut (ticked off)".into());
        }
        if let Some(problem) = problem {
            lines.push(format!("⚠ {problem}"));
        }
        if hidden > 0 {
            lines.push(format!(
                "Cuts {hidden} hidden {}: show them in Drawing to see them",
                if hidden == 1 { "contour" } else { "contours" }
            ));
        }
        lines.push("Drag to reorder; right-click for more".into());
        lines.join("\n")
    }

    /// Properties: the selection, the selected shape's editor or facts, and Copy as
    /// selector (docs/specs/browser-and-layout.md §4). Only while something is selected.
    pub(super) fn selection_section(&mut self, ui: &mut egui::Ui) {
        if self.selection.ids(&self.contours).is_empty() {
            return;
        }
        let icon = match &self.selection {
            CanvasSelection::Layer(..) => Icon::Layer,
            CanvasSelection::Source(chipload_core::chain::ShapeFrom::Dxf(_)) => Icon::Dxf,
            CanvasSelection::Source(chipload_core::chain::ShapeFrom::Image(_)) => Icon::Image,
            _ => Icon::Polyline,
        };
        let id = ui.make_persistent_id("properties-selection");
        egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, true)
            .show_header(ui, |ui| title_line(ui, icon, None, "Selection"))
            .body(|ui| {
                for line in self.selection_lines() {
                    ui.label(line);
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
            });
    }
}

/// "tool change → T2": a thin line with the new tool, between operation rows.
fn tool_change(ui: &mut egui::Ui, tool: i64) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 16.0), Sense::hover());
    let painter = ui.painter_at(rect);
    let color = ui.visuals().weak_text_color();
    let left = rect.left() + PAD + 3.0 * INDENT + INDENT;
    let galley = egui::WidgetText::from(RichText::new(format!("tool change → T{tool}")).small())
        .into_galley(
            ui,
            Some(egui::TextWrapMode::Extend),
            f32::INFINITY,
            egui::TextStyle::Small,
        );
    let text_at = egui::pos2(left, rect.center().y - galley.size().y / 2.0);
    let line_from = left + galley.size().x + GAP * 2.0;
    painter.galley(text_at, galley, color);
    let guide = Stroke::new(1.0, ui.visuals().widgets.noninteractive.bg_stroke.color);
    for level in 1..=3 {
        let x = rect.left() + PAD + (level as f32 - 0.5) * INDENT;
        painter.vline(x.round() + 0.5, rect.y_range(), guide);
    }
    if line_from < rect.right() - PAD {
        painter.hline(line_from..=rect.right() - PAD, rect.center().y, guide);
    }
}

/// The kind's colour: pockets and profiles told apart at a glance.
pub(super) fn kind_color(op: &Operation, dark: bool) -> Color32 {
    match (op, dark) {
        (Operation::Profile(_), true) => Color32::from_rgb(0x6f, 0xc8, 0xbd),
        (Operation::Profile(_), false) => Color32::from_rgb(0x00, 0x79, 0x6b),
        (Operation::Pocket(_), true) => Color32::from_rgb(0xc3, 0xa6, 0xf0),
        (Operation::Pocket(_), false) => Color32::from_rgb(0x6a, 0x3d, 0xb8),
        (Operation::Vcarve(_), true) => Color32::from_rgb(0xf0, 0xb4, 0x6a),
        (Operation::Vcarve(_), false) => Color32::from_rgb(0xa8, 0x55, 0x00),
        (Operation::Path(_), true) => Color32::from_rgb(0x8f, 0xbc, 0xf5),
        (Operation::Path(_), false) => Color32::from_rgb(0x1f, 0x5c, 0xb0),
        (Operation::Holes(_), true) => Color32::from_rgb(0xf0, 0x9a, 0xb4),
        (Operation::Holes(_), false) => Color32::from_rgb(0xa8, 0x1e, 0x55),
    }
}
