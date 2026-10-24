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
                    t!(ui, "browser-eye-hide-hover")
                } else {
                    t!(ui, "browser-eye-show-hover")
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
                    t!(ui, "browser-tick-on-hover")
                } else {
                    t!(ui, "browser-tick-off-hover")
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
        // Ends in "…" in a narrow panel.
        ui.add(egui::Label::new(egui::RichText::new(text).heading()).truncate())
            .on_hover_text(text);
    });
    ui.add_space(2.0);
}

/// A project tool's link to the library, short, for its row's right end.
fn link_note(ui: &egui::Ui, link: &chipload_job::library::Link) -> String {
    use chipload_job::library::Link;
    match link {
        Link::None => String::new(),
        Link::Same(name) => format!("← {name}"),
        Link::Differs(name, _) => format!("← {}", t!(ui, "browser-link-changed", name = name)),
        Link::Missing(_) => t!(ui, "browser-link-missing"),
    }
}

/// The icon of an operation's kind.
pub(super) fn op_icon(op: &Operation) -> Icon {
    match op {
        Operation::Profile(_) => Icon::Profile,
        Operation::Pocket(_) => Icon::Pocket,
        Operation::Vcarve(_) => Icon::Vcarve,
        Operation::Path(_) => Icon::FollowPath,
        Operation::Holes(_) => Icon::Holes,
        Operation::Surface(_) => Icon::Surface,
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

/// What a part's rows need besides the part.
struct PartRows<'a> {
    /// Its place among the parts.
    k: usize,
    parts: &'a [chipload_job::Part],
    active: Option<u64>,
    op_parts: &'a [u64],
    count: usize,
    /// The tool cut just before each operation, in cut order.
    tool_before: &'a std::collections::BTreeMap<usize, i64>,
}

/// What a click in the operation rows asks for, done after they are drawn.
enum OpAction {
    Focus(usize),
    Toggle(usize),
    Move(usize, usize),
    Duplicate(usize),
    Delete(usize),
    /// A rest pass for pocket i with this tool (docs/specs/rest-passes.md §6).
    AddRest(usize, i64),
    /// The same with a library tool, by name, copied into the project first.
    AddRestFromLibrary(usize, String),
    /// Pocket i's rest pass k: open it, turn it on or off, remove it.
    PassFocus(usize, usize),
    PassToggle(usize, usize),
    PassRemove(usize, usize),
    /// Rest pocket i becomes a rest pass of the pocket it copies.
    MakePass(usize),
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
pub(super) fn new_operations(src: &impl crate::i18n::Lang) -> [(String, OpKind, String); 6] {
    [
        (
            t!(src, "browser-profile"),
            OpKind::Profile,
            t!(src, "browser-profile-hover"),
        ),
        (
            t!(src, "browser-pocket"),
            OpKind::Pocket,
            t!(src, "browser-pocket-hover"),
        ),
        (
            t!(src, "browser-vcarve"),
            OpKind::Vcarve,
            t!(src, "browser-vcarve-hover"),
        ),
        (
            t!(src, "browser-follow-path"),
            OpKind::Path,
            t!(src, "browser-follow-path-hover"),
        ),
        (
            t!(src, "browser-holes"),
            OpKind::Holes,
            t!(src, "browser-holes-hover"),
        ),
        (
            t!(src, "browser-surface"),
            OpKind::Surface,
            t!(src, "browser-surface-hover"),
        ),
    ]
}

impl ChiploadApp {
    /// Left: the Browser.
    pub(super) fn sidebar(&mut self, ui: &mut egui::Ui) {
        let browser = t!(ui, "browser-caption");
        caption(ui, &browser);
        ScrollArea::vertical()
            .id_salt("browser-scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                // The user's library, above the project (docs/specs/library-in-browser.md §4).
                self.library_tree(ui);
                ui.add_space(ROW_HEIGHT / 2.0);
                if self.edit.is_some() {
                    self.browser_tree(ui);
                } else {
                    ui.weak(t!(ui, "browser-no-project"));
                }
            });
    }

    fn browser_tree(&mut self, ui: &mut egui::Ui) {
        let name = self
            .job_path
            .as_deref()
            .map_or_else(|| t!(ui, "browser-new-project"), file_name);
        let root = Row::new(Id::new("browser-project"), 0, name).icon(Icon::Project);
        let (open, _) = folder(ui, root, true);
        if !open {
            return;
        }
        let problems = self.settings_problems().count();
        let job = Row::new(Id::new("browser-job"), 1, t!(ui, "browser-job-settings"))
            .icon(Icon::Settings)
            .selected(self.focus == Focus::Job)
            .alert(problems > 0)
            .show(ui);
        let job = if problems > 0 {
            job.response.on_hover_text(t!(
                ui,
                "browser-job-settings-problems-hover",
                count = problems
            ))
        } else {
            job.response
                .on_hover_text(t!(ui, "browser-job-settings-hover"))
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
        let marked: Vec<bool> = (0..tools.len())
            .map(|k| {
                let prefix = format!("tools.{k}.");
                self.job_problems().any(|p| p.field().starts_with(&prefix))
            })
            .collect();
        let flagged = |k: usize| marked.get(k).copied().unwrap_or(false);
        let any = marked.iter().any(|m| *m);
        let id = Id::new("browser-tools");
        let open_id = id.with("folder-open");
        let open = folder_open(ui, open_id, false);
        let out = Row::new(id, 1, t!(ui, "browser-tools"))
            .folder(open)
            .icon(Icon::Tools)
            .trailing(tools.len().to_string())
            .selected(self.focus == Focus::Tools)
            .alert(any)
            .show(ui);
        if out.toggled {
            toggle_folder(ui, open_id, false);
        }
        let mut dropped = self.library_drop(ui, &out.response);
        // A click shows every tool, to add one or compare them
        // (docs/specs/properties-for-tools-and-dxfs.md §2).
        if out
            .response
            .on_hover_text(t!(ui, "browser-tools-hover"))
            .clicked()
        {
            self.focus = Focus::Tools;
        }
        if open {
            for (k, tool) in tools.iter().enumerate() {
                let link = self
                    .library()
                    .map(|l| chipload_job::library::tool_link(&l.tools, tool));
                let row = Row::new(
                    Id::new(("browser-tool", tool.id)),
                    2,
                    forms::tool_label(tool, crate::units::display(ui.ctx())),
                )
                .icon(tool_icon(tool.kind))
                .trailing(link.as_ref().map(|l| link_note(ui, l)).unwrap_or_default())
                .selected(self.focus == Focus::Tool(tool.id))
                .alert(flagged(k))
                .show(ui);
                dropped = dropped.or(self.library_drop(ui, &row.response));
                let response = match &link {
                    Some(link) => row.response.on_hover_text(link.text("library tool")),
                    None => row.response,
                };
                if response.clicked() {
                    self.focus = Focus::Tool(tool.id);
                }
            }
        }
        if let Some(uid) = dropped {
            self.add_library_tool_uid(&uid);
        }
    }

    /// A library tool dropped on `response` (the project's Tools or one of its tools): its
    /// uid. While one is dragged over it, the row is outlined.
    fn library_drop(&self, ui: &egui::Ui, response: &Response) -> Option<String> {
        use super::library_view::DraggedTool;
        if let Some(tool) = response.dnd_release_payload::<DraggedTool>() {
            return Some(tool.0.clone());
        }
        if response.dnd_hover_payload::<DraggedTool>().is_some() {
            ui.painter().rect_stroke(
                response.rect,
                0.0,
                Stroke::new(1.5, ui.visuals().selection.stroke.color),
                egui::StrokeKind::Inside,
            );
        }
        None
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
        let row = Row::new(Id::new("browser-parts"), 1, t!(ui, "browser-parts"))
            .icon(Icon::Parts)
            .trailing(parts.len().to_string());
        let (open, folder_row) = folder(ui, row, true);
        let mut action: Option<OpAction> = None;
        // The tool cut just before each operation, in cut order: rest passes wait for their
        // run of pockets (docs/specs/rest-passes.md §3), so the list's order isn't enough.
        let tool_before: std::collections::BTreeMap<usize, i64> = self
            .edited_job()
            .map(|job| {
                let mut before = std::collections::BTreeMap::new();
                let mut last: Option<i64> = None;
                for at in job.cut_list() {
                    let tool = job.resolve(at).map(|op| op.tool());
                    if at.pass.is_none()
                        && let Some(t) = last
                    {
                        before.insert(at.operation, t);
                    }
                    last = tool.or(last);
                }
                before
            })
            .unwrap_or_default();
        folder_row
            .response
            .on_hover_text(t!(ui, "browser-parts-hover"))
            .context_menu(|ui| {
                if ui.button(t!(ui, "browser-new-part")).clicked() {
                    action = Some(OpAction::NewPart);
                }
            });
        if open {
            for (k, part) in parts.iter().enumerate() {
                let rows = PartRows {
                    k,
                    parts: &parts,
                    active,
                    op_parts: &op_parts,
                    count,
                    tool_before: &tool_before,
                };
                if let Some(a) = self.part_rows(ui, part, rows) {
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
            Some(OpAction::AddRest(i, tool)) => {
                self.add_rest_pass(i, tool);
            }
            Some(OpAction::AddRestFromLibrary(i, name)) => {
                self.add_rest_pass_from_library(i, &name);
            }
            Some(OpAction::PassFocus(i, k)) => self.focus = Focus::RestPass(i, k),
            Some(OpAction::PassToggle(i, k)) => self.toggle_rest_pass(i, k),
            Some(OpAction::PassRemove(i, k)) => self.remove_rest_pass(i, k),
            Some(OpAction::MakePass(i)) => self.make_rest_pass(i),
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
    fn part_rows(
        &mut self,
        ui: &mut egui::Ui,
        part: &chipload_job::Part,
        rows: PartRows,
    ) -> Option<OpAction> {
        let PartRows {
            k,
            parts,
            active,
            op_parts,
            count,
            tool_before,
        } = rows;
        let mine: Vec<usize> = (0..count).filter(|i| op_parts[*i] == part.id).collect();
        let flagged = mine.iter().any(|i| {
            self.problems.iter().any(|p| p.operation() == Some(*i)) || self.error_ops.contains(i)
        }) || !self.form_problem_map(Focus::Part(part.id)).is_empty();
        let is_active = active == Some(part.id);
        let mut trailing = Vec::new();
        if is_active && parts.len() > 1 {
            trailing.push(t!(ui, "browser-part-active"));
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
            t!(ui, "browser-part-active-hover")
        } else {
            t!(ui, "browser-part-hover")
        };
        out.response.on_hover_text(hover).context_menu(|ui| {
            if ui
                .add_enabled(!is_active, egui::Button::new(t!(ui, "browser-activate")))
                .on_hover_text(t!(ui, "browser-activate-hover"))
                .clicked()
            {
                action = Some(OpAction::PartActivate(part.id));
            }
            ui.menu_button(t!(ui, "browser-new-operation"), |ui| {
                for (label, kind, hover) in new_operations(ui) {
                    if ui
                        .add_enabled(
                            selected || !kind.needs_selection(),
                            egui::Button::new(label),
                        )
                        .on_hover_text(hover)
                        .on_disabled_hover_text(t!(ui, "browser-select-shapes-first"))
                        .clicked()
                    {
                        action = Some(OpAction::NewIn(kind, part.id));
                    }
                }
            });
            ui.separator();
            if ui
                .add_enabled(k > 0, egui::Button::new(t!(ui, "browser-move-up")))
                .clicked()
            {
                action = Some(OpAction::PartMove(part.id, true));
            }
            if ui
                .add_enabled(
                    k + 1 < parts.len(),
                    egui::Button::new(t!(ui, "browser-move-down")),
                )
                .clicked()
            {
                action = Some(OpAction::PartMove(part.id, false));
            }
            ui.separator();
            let what = t!(ui, "browser-delete-part", count = mine.len());
            if ui
                .add_enabled(parts.len() > 1, egui::Button::new(what))
                .on_disabled_hover_text(t!(ui, "browser-delete-part-disabled-hover"))
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
                    t!(ui, "browser-part-empty-active")
                } else {
                    t!(ui, "browser-part-empty")
                });
            });
        }
        for i in mine {
            if let Some(a) = self.operation_row(ui, i, count, tool_before.get(&i).copied()) {
                action = Some(a);
            }
        }
        action
    }

    /// Operation `i`'s row (and the tool change line above it). `previous`: the tool cut
    /// just before it, in cut order.
    fn operation_row(
        &mut self,
        ui: &mut egui::Ui,
        i: usize,
        count: usize,
        previous: Option<i64>,
    ) -> Option<OpAction> {
        let edit = self.edit.as_ref()?;
        let entry = edit.ops.get(i)?;
        let op = &entry.op;
        // An operation is cut when it and its part are on (docs/specs/parts.md §3).
        let cut = |op: &Operation| {
            op.enabled() && edit.settings.part(op.part()).is_some_and(|p| p.enabled)
        };
        // A line marks each tool change (docs/specs/tool-per-operation.md §8).
        let is_cut = cut(op);
        if is_cut && previous.is_some_and(|t| t != op.tool()) {
            tool_change(ui, op.tool());
        }
        let problem = self
            .problems
            .iter()
            .find(|p| p.operation() == Some(i))
            .map(|p| problems::plain(ui, p));
        let missing = edit.missing().iter().any(|p| p.operation() == Some(i));
        let flagged = missing || self.error_ops.contains(&i) || problem.is_some();
        let hidden = self.hidden_cut(op);
        // What a rest pass for this pocket can be cut with, or why it can't
        // (docs/specs/rest-passes.md §6).
        let units = crate::units::display(ui.ctx());
        let rest_menu = self.rest_menu(i, units);
        let name = match op.name() {
            "" => t!(ui, "browser-no-name"),
            name => name.to_string(),
        };
        let trailing = if hidden > 0 {
            let hidden = t!(ui, "browser-op-hidden", count = hidden);
            format!("{hidden} · T{}", op.tool())
        } else {
            format!("T{}", op.tool())
        };
        let hover = crate::units::message(
            ui.ctx(),
            &self.operation_hover(i, problem.as_deref(), hidden),
        );
        // A rest pocket that copies an earlier pocket can become its rest pass (§5).
        let parent = match op {
            Operation::Pocket(p) if p.rest.is_some() => self.rest_pocket_parent(i),
            _ => None,
        };
        let passes: Vec<chipload_job::RestPass> = match op {
            Operation::Pocket(p) => p.rest_passes.clone(),
            _ => Vec::new(),
        };
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
                    egui::Button::new(with_keys(
                        &t!(ui, "browser-move-up"),
                        shortcuts::Action::MoveUp,
                    )),
                )
                .clicked()
            {
                action = Some(OpAction::Move(i, i - 1));
            }
            if ui
                .add_enabled(
                    i + 1 < count,
                    egui::Button::new(with_keys(
                        &t!(ui, "browser-move-down"),
                        shortcuts::Action::MoveDown,
                    )),
                )
                .clicked()
            {
                action = Some(OpAction::Move(i, i + 1));
            }
            if ui.button(t!(ui, "browser-duplicate")).clicked() {
                action = Some(OpAction::Duplicate(i));
            }
            match &rest_menu {
                Some(RestMenu::Tools { project, library }) => {
                    let size = |d: f64| chipload_pipeline::units::length_text(d, units);
                    ui.menu_button(t!(ui, "browser-add-rest-pass"), |ui| {
                        for (tool, diameter) in project {
                            if ui
                                .button(t!(
                                    ui,
                                    "browser-rest-project-tool",
                                    tool = tool.to_string(),
                                    size = size(*diameter)
                                ))
                                .clicked()
                            {
                                action = Some(OpAction::AddRest(i, *tool));
                            }
                        }
                        if !library.is_empty() {
                            if !project.is_empty() {
                                ui.separator();
                            }
                            ui.weak(t!(ui, "browser-from-library"));
                            for (name, diameter) in library {
                                if ui
                                    .button(t!(
                                        ui,
                                        "browser-rest-library-tool",
                                        name = name,
                                        size = size(*diameter)
                                    ))
                                    .on_hover_text(t!(ui, "browser-copied-into-project-hover"))
                                    .clicked()
                                {
                                    action = Some(OpAction::AddRestFromLibrary(i, name.clone()));
                                }
                            }
                        }
                    })
                    .response
                    .on_hover_text(t!(ui, "browser-add-rest-pass-hover"));
                }
                Some(RestMenu::Unavailable(why)) => {
                    ui.add_enabled(false, egui::Button::new(t!(ui, "browser-add-rest-pass")))
                        .on_disabled_hover_text(why);
                }
                None => {}
            }
            if let Some(parent) = parent
                && ui
                    .button(t!(
                        ui,
                        "browser-make-rest-pass",
                        number = (parent + 1).to_string()
                    ))
                    .on_hover_text(t!(ui, "browser-make-rest-pass-hover"))
                    .clicked()
            {
                action = Some(OpAction::MakePass(i));
            }
            ui.separator();
            if ui.button(t!(ui, "browser-delete")).clicked() {
                action = Some(OpAction::Delete(i));
            }
        });
        let color = kind_color(op, dark);
        for (k, pass) in passes.iter().enumerate() {
            if let Some(a) = self.rest_pass_row(ui, i, k, pass, is_cut, color) {
                action = Some(a);
            }
        }
        action
    }

    /// Pocket `i`'s rest pass `k`, a leaf under it (docs/specs/rest-passes.md §6).
    fn rest_pass_row(
        &self,
        ui: &mut egui::Ui,
        i: usize,
        k: usize,
        pass: &chipload_job::RestPass,
        pocket_cut: bool,
        color: Color32,
    ) -> Option<OpAction> {
        let at = chipload_job::OpRef::pass(i, k);
        let flagged = self.problems.iter().any(|p| {
            p.operation() == Some(i) && p.field().starts_with(&format!("rest_passes.{k}"))
        });
        let time = self
            .result
            .as_ref()
            .and_then(|r| r.estimate.operations.iter().find(|(o, _)| *o == at))
            .map_or(String::new(), |(_, s)| {
                format!("{} · ", chipload_pipeline::estimate::duration_text(*s))
            });
        let row = Row::new(
            Id::new(("browser-pass", i, k)),
            4,
            format!("{at}  {}", t!(ui, "browser-rest")),
        )
        .tick(pass.enabled)
        .icon_colored(Icon::Pocket, color)
        .trailing(format!("{time}T{}", pass.tool))
        .selected(self.focus == Focus::RestPass(i, k))
        .dim(!(pocket_cut && pass.enabled))
        .alert(flagged)
        .show(ui);
        let mut action = None;
        if row.tick {
            action = Some(OpAction::PassToggle(i, k));
        } else if row.response.clicked() {
            action = Some(OpAction::PassFocus(i, k));
        }
        row.response
            .on_hover_text(t!(ui, "browser-rest-pass-hover"))
            .context_menu(|ui| {
                if ui.button(t!(ui, "browser-remove")).clicked() {
                    action = Some(OpAction::PassRemove(i, k));
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
            Operation::Profile(_) => t!(self, "browser-profile"),
            Operation::Pocket(_) => t!(self, "browser-pocket"),
            Operation::Vcarve(_) => t!(self, "browser-vcarve"),
            Operation::Path(_) => t!(self, "browser-follow-path"),
            Operation::Holes(_) => t!(self, "browser-holes"),
            Operation::Surface(_) => t!(self, "browser-surface"),
        };
        let summary = geometry_summary(op, &self.contours, self);
        let (start, end) = op.depths();
        // docs/specs/rest-machining.md §6.
        let rest = match op {
            Operation::Pocket(p) => p.rest.map_or(String::new(), |r| {
                let rest = t!(
                    self,
                    "browser-op-rest-after",
                    tool = r.after_tool.to_string()
                );
                format!(", {rest}")
            }),
            _ => String::new(),
        };
        let time = self
            .result
            .as_ref()
            .and_then(|r| {
                r.estimate
                    .operations
                    .iter()
                    .find(|(o, _)| *o == chipload_job::OpRef::op(i))
            })
            .map_or(String::new(), |(_, s)| {
                format!(" · {}", chipload_pipeline::estimate::duration_text(*s))
            });
        let mut lines = vec![format!(
            "{kind} · T{}{rest} · {summary} · {start}–{end} mm{time}",
            op.tool()
        )];
        if !op.enabled() {
            lines.push(t!(self, "browser-op-not-cut"));
        }
        if let Some(problem) = problem {
            lines.push(format!("⚠ {problem}"));
        }
        if hidden > 0 {
            lines.push(t!(self, "browser-op-hidden-hover", count = hidden));
        }
        lines.push(t!(self, "browser-op-drag-hover"));
        lines.join("\n")
    }

    /// Whether Properties shows the Selection section: while something is selected, except
    /// while an operation is open (its *Use selection* counts the selection, so the form
    /// never moves) or the open DXF, picture or text is itself what is selected.
    pub(super) fn selection_shown(&self) -> bool {
        use chipload_core::chain::ShapeFrom;
        if self.selection.ids(&self.contours).is_empty() {
            return false;
        }
        let open = match self.focus {
            Focus::Op(_) | Focus::RestPass(..) => return false,
            Focus::Dxf(id) => ShapeFrom::Dxf(id),
            Focus::Image(id) => ShapeFrom::Image(id),
            Focus::Text(id) => ShapeFrom::Text(id),
            _ => return true,
        };
        self.selection != CanvasSelection::Source(open)
    }

    /// Properties: the selection, the selected shape's editor or facts, and Copy as
    /// selector (docs/specs/browser-and-layout.md §4), above the open form when shown.
    pub(super) fn selection_section(&mut self, ui: &mut egui::Ui) {
        if self.selection.ids(&self.contours).is_empty() {
            return;
        }
        let icon = match &self.selection {
            CanvasSelection::Layer(..) => Icon::Layer,
            CanvasSelection::Source(chipload_core::chain::ShapeFrom::Dxf(_)) => Icon::Dxf,
            CanvasSelection::Source(chipload_core::chain::ShapeFrom::Image(_))
            | CanvasSelection::Except(chipload_core::chain::ShapeFrom::Image(_), _) => Icon::Image,
            CanvasSelection::Except(chipload_core::chain::ShapeFrom::Dxf(_), _) => Icon::Dxf,
            _ => Icon::Polyline,
        };
        let id = ui.make_persistent_id("properties-selection");
        egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, true)
            .show_header(ui, |ui| {
                let title = t!(ui, "browser-selection");
                title_line(ui, icon, None, &title);
            })
            .body(|ui| {
                for line in self.selection_lines() {
                    ui.label(line);
                }
                self.selected_shape_ui(ui);
                if ui
                    .add_enabled(
                        self.selector_json().is_some(),
                        egui::Button::new(t!(ui, "browser-copy-as-selector")),
                    )
                    .on_hover_text(t!(ui, "browser-copy-as-selector-hover"))
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
    let galley = egui::WidgetText::from(
        RichText::new(t!(ui, "browser-tool-change", tool = tool.to_string())).small(),
    )
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
        (Operation::Surface(_), true) => Color32::from_rgb(0xb8, 0xc4, 0x8a),
        (Operation::Surface(_), false) => Color32::from_rgb(0x5a, 0x6b, 0x1e),
    }
}
