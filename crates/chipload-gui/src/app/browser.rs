// SPDX-License-Identifier: GPL-3.0-or-later
//! The Browser (docs/specs/browser-and-layout.md §2, §3): the project as one tree, with
//! Job settings, Tools, Drawing and Operations as folders. Rows are flat and one line: a
//! click selects, a double-click edits, a right-click offers the rest.

use eframe::egui::{Color32, Id, Response, Stroke, vec2};

use super::browser_row::{
    GAP, INDENT, PAD, ROW_HEIGHT, Row, folder, folder_open, title_line, toggle_folder,
};
use super::*;
use crate::i18n::Lang;
use crate::icons::Icon;
use crate::style::Tint;

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
        Operation::Chamfer(_) => Icon::EdgeChamfer,
        Operation::Flute(_) => Icon::Flute,
        Operation::Pattern(_) => Icon::Hatch,
        Operation::Prism(_) => Icon::Prism,
        Operation::Moulding(_) => Icon::Moulding,
        Operation::Holes(_) => Icon::Holes,
        Operation::Surface(_) => Icon::Surface,
        Operation::Relief(_) => Icon::Relief,
    }
}

/// The icon of a tool's kind.
pub(super) fn tool_icon(kind: ToolType) -> Icon {
    match kind {
        ToolType::Endmill => Icon::Endmill,
        ToolType::Drill => Icon::Drill,
        ToolType::Vbit => Icon::Vbit,
        ToolType::Ballnose => Icon::Ballnose,
    }
}

/// A tool picked from a tool menu: a project tool by id, or a library tool by name.
pub(super) enum ToolPick {
    Project(i64),
    Library(String),
}

/// A *▸* menu of end mills (`menu`): the project's, then the library's, each with its size;
/// or greyed out with why. The pick, if any.
pub(super) fn tool_menu(
    ui: &mut egui::Ui,
    label: &str,
    hover: &str,
    menu: &RestMenu,
    units: Units,
) -> Option<ToolPick> {
    let mut pick = None;
    match menu {
        RestMenu::Tools { project, library } => {
            let size = |d: f64| chipload_pipeline::units::length_text(d, units);
            super::header::submenu(ui, label, |ui| {
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
                        pick = Some(ToolPick::Project(*tool));
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
                            pick = Some(ToolPick::Library(name.clone()));
                        }
                    }
                }
            })
            .response
            .on_hover_text(hover);
        }
        RestMenu::Unavailable(why) => {
            ui.add_enabled(false, egui::Button::new(label))
                .on_disabled_hover_text(why);
        }
    }
    pick
}

/// A pass cut before its operation, shown as a leaf under it.
#[derive(Clone, Copy)]
pub(super) enum Leaf {
    Floor,
    Roughing,
    Smoothing,
}

/// The kinds of new operation: label, kind and what it does.
pub(super) fn new_operations(src: &impl crate::i18n::Lang) -> [(String, OpKind, String); 12] {
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
            t!(src, "browser-chamfer"),
            OpKind::Chamfer,
            t!(src, "browser-chamfer-hover"),
        ),
        (
            t!(src, "browser-flute"),
            OpKind::Flute,
            t!(src, "browser-flute-hover"),
        ),
        (
            t!(src, "browser-pattern"),
            OpKind::Pattern,
            t!(src, "browser-pattern-hover"),
        ),
        (
            t!(src, "browser-prism"),
            OpKind::Prism,
            t!(src, "browser-prism-hover"),
        ),
        (
            t!(src, "browser-moulding"),
            OpKind::Moulding,
            t!(src, "browser-moulding-hover"),
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
        (
            t!(src, "browser-relief"),
            OpKind::Relief,
            t!(src, "browser-relief-hover"),
        ),
    ]
}

/// Whether a new operation of `kind` can be made: most need shapes selected, a surface
/// nothing, a relief a model in the project (docs/specs/stl-relief.md §9).
pub(super) fn can_add(kind: OpKind, selected: bool, models: bool) -> bool {
    match kind {
        OpKind::Relief => models,
        _ => selected || !kind.needs_selection(),
    }
}

/// Why a new operation of `kind` can't be made yet, for its hover.
pub(super) fn cannot_add(src: &impl crate::i18n::Lang, kind: OpKind) -> String {
    match kind {
        OpKind::Relief => t!(src, "browser-relief-needs-model"),
        // The whole board is the setup's (docs/specs/surface-levels.md §5).
        OpKind::Surface => t!(src, "browser-surface-needs-shapes"),
        _ => t!(src, "header-select-shapes-first"),
    }
}

impl ChiploadApp {
    /// Left: the Browser.
    pub(super) fn sidebar(&mut self, ui: &mut egui::Ui) {
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
                    // The empty space below the rows: the project's menu
                    // (docs/specs/parts-and-setups.md §2a).
                    let rest = ui.available_size().max(egui::vec2(0.0, ROW_HEIGHT * 2.0));
                    let (_, empty) = ui.allocate_exact_size(rest, egui::Sense::click());
                    let mut asked = None;
                    empty.context_menu(|ui| asked = self.project_menu(ui));
                    if let Some(action) = asked {
                        self.apply_setup_action(action);
                    }
                } else {
                    ui.weak(t!(ui, "browser-no-project"));
                }
            });
    }

    fn browser_tree(&mut self, ui: &mut egui::Ui) {
        let name = self
            .file
            .path
            .as_deref()
            .map_or_else(|| t!(ui, "browser-new-project"), file_name);
        let root = Row::new(Id::new("browser-project"), 0, name).icon(Icon::Project);
        let (open, root) = folder(ui, root, true);
        // The project's menu: new part, new setup, … (docs/specs/parts-and-setups.md §2a).
        let mut asked = None;
        root.response
            .context_menu(|ui| asked = self.project_menu(ui));
        if let Some(action) = asked {
            self.apply_setup_action(action);
        }
        if !open {
            return;
        }
        let problems = self.settings_problems().count();
        let job = Row::new(Id::new("browser-job"), 1, t!(ui, "browser-job-settings"))
            .icon_tinted(Icon::Settings, Tint::Tools)
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
        let tools = edit.job.tools.clone();
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
            .icon_tinted(Icon::Tools, Tint::Tools)
            .trailing(tools.len().to_string())
            .selected(self.focus == Focus::Tools)
            .alert(any)
            .show(ui);
        if out.toggled {
            toggle_folder(ui, open_id, false);
        }
        let mut dropped = self.library_drop(ui, &out.response);
        // Every tool that differs from its library tool, updated (docs/specs/tool-updates.md
        // §3).
        let differ = tools.iter().any(|t| {
            matches!(
                self.project_tool_link(t),
                Some(chipload_job::library::Link::Differs(..))
            )
        });
        let mut update_all = false;
        out.response.context_menu(|ui| {
            if ui
                .add_enabled(differ, egui::Button::new(t!(ui, "user-library-update-all")))
                .on_hover_text(t!(ui, "user-library-update-all-hover"))
                .on_disabled_hover_text(t!(ui, "user-library-update-all-none"))
                .clicked()
            {
                update_all = true;
            }
        });
        if update_all {
            self.update_tools_from_library(None);
        }
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
                let link = self.project_tool_link(tool);
                let row = Row::new(
                    Id::new(("browser-tool", tool.id)),
                    2,
                    forms::tool_label(ui, tool, crate::units::display(ui.ctx())),
                )
                .icon(tool_icon(tool.kind))
                .trailing(link.as_ref().map(|l| link_note(ui, l)).unwrap_or_default())
                .selected(self.focus == Focus::Tool(tool.id))
                .alert(flagged(k))
                .show(ui);
                dropped = dropped.or(self.library_drop(ui, &row.response));
                let response = match &link {
                    Some(link) => row.response.on_hover_text(
                        link.message(chipload_job::library::LinkEntry::Tool)
                            .render(ui.lang(), self.prefs.units.into()),
                    ),
                    None => row.response,
                };
                if response.clicked() {
                    self.focus = Focus::Tool(tool.id);
                }
                let mut delete = false;
                response.context_menu(|ui| {
                    delete = tool_delete_item(ui, self.edit.as_ref(), tool.id);
                });
                if delete {
                    self.delete_project_tool(tool.id);
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

    /// Properties: the selection, the selected shape's editor or facts, and Copy as
    /// selector (docs/specs/browser-and-layout.md §4), above the open form when shown.
    pub(super) fn selection_section(&mut self, ui: &mut egui::Ui) {
        if self.selected_ids().is_empty() {
            return;
        }
        let icon = match &self.selection {
            CanvasSelection::Layer(..) => Icon::Layer,
            CanvasSelection::Whole {
                of: Source::Dxf(_), ..
            } => Icon::Dxf,
            CanvasSelection::Whole {
                of: Source::Image(_),
                ..
            } => Icon::Image,
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
                self.group_section(ui);
                self.array_form(ui);
                self.frame_form(ui);
                self.live_frame_ui(ui);
                self.selected_shape_ui(ui);
                self.selected_shapes_place_ui(ui);
                self.arrange_ui(ui);
                self.construction_ui(ui);
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

/// The tool changes shown above each operation's row, by operation index, in cut order
/// (docs/specs/tool-per-operation.md §8). Floor passes are cut before their run of V-carves
/// and rest passes after their run of pockets, though their rows are leaves below: where
/// rows are cut out of the Browser's order, the line above the first row of the run says
/// its tools in cut order, "tool change → T3 → T1", the first one too.
pub(super) fn tool_changes(job: &chipload_job::Job) -> std::collections::BTreeMap<usize, Vec<i64>> {
    let list = job.cut_list();
    let tools: Vec<Option<i64>> = list
        .iter()
        .map(|&at| job.resolve(at).map(|op| op.tool()))
        .collect();
    let mut last_at = std::collections::BTreeMap::new();
    for (k, at) in list.iter().enumerate() {
        last_at.insert(at.operation, k);
    }
    let mut changes = std::collections::BTreeMap::new();
    let mut last: Option<i64> = None;
    let mut k = 0;
    while k < list.len() {
        // A run: from this row's first item to the last item of any row cut within it.
        let first = list[k].operation;
        let mut end = last_at.get(&first).copied().unwrap_or(k);
        let start = k;
        let mut here = Vec::new();
        while k <= end && k < list.len() {
            end = end.max(last_at.get(&list[k].operation).copied().unwrap_or(k));
            if let Some(tool) = tools[k] {
                if last.is_some_and(|t| t != tool) {
                    here.push(tool);
                }
                last = Some(tool);
            }
            k += 1;
        }
        let first_tool = tools[start..k].iter().flatten().next().copied();
        if k - start > 1
            && !here.is_empty()
            && let Some(tool) = first_tool
            && here.first() != Some(&tool)
        {
            here.insert(0, tool);
        }
        if !here.is_empty() {
            changes.insert(first, here);
        }
    }
    changes
}

/// "tool change → T2", or "tool change → T3 → T1" for a run cut out of the list's order: a
/// thin line with the new tools, between operation rows.
pub(super) fn tool_change(ui: &mut egui::Ui, tools: &[i64]) {
    let tool = tools
        .iter()
        .map(i64::to_string)
        .collect::<Vec<_>>()
        .join(" → T");
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 16.0), Sense::hover());
    let painter = ui.painter_at(rect);
    let color = ui.visuals().weak_text_color();
    let left = rect.left() + PAD + 4.0 * INDENT + INDENT;
    let galley =
        egui::WidgetText::from(RichText::new(t!(ui, "browser-tool-change", tool = tool)).small())
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
    for level in 1..=4 {
        let x = rect.left() + PAD + (level as f32 - 0.5) * INDENT;
        painter.vline(x.round() + 0.5, rect.y_range(), guide);
    }
    if line_from < rect.right() - PAD {
        painter.hline(line_from..=rect.right() - PAD, rect.center().y, guide);
    }
}

/// The kind's colour: pockets and profiles told apart at a glance.
pub(super) fn kind_color(op: &Operation, dark: bool) -> Color32 {
    crate::style::op_color(chipload_pipeline::styles::kind_of(op), dark)
}

/// *Delete* in a project tool's menu, refused (with why) while an operation uses it. Whether
/// it was clicked.
fn tool_delete_item(ui: &mut egui::Ui, edit: Option<&EditJob>, id: i64) -> bool {
    let users = edit.map(|e| forms::tool_users(e, id)).unwrap_or_default();
    let button = ui.add_enabled(users.is_empty(), egui::Button::new(t!(ui, "common-delete")));
    if users.is_empty() {
        button.clicked()
    } else {
        button.on_disabled_hover_text(t!(
            ui,
            "forms-remove-tool-disabled",
            operations = users.join(", ")
        ));
        false
    }
}

impl ChiploadApp {
    /// Remove project tool `id`, as one undo step; refused, in the status bar, while an
    /// operation or a pass cuts with it. Whether it was removed.
    pub(super) fn delete_project_tool(&mut self, id: i64) -> bool {
        let Some(edit) = self.edit.as_ref() else {
            return false;
        };
        let users = forms::tool_users(edit, id);
        if !users.is_empty() {
            let text = t!(
                self,
                "forms-remove-tool-disabled",
                operations = users.join(", ")
            );
            self.set_status(text, ERROR_STATUS_SECONDS);
            return false;
        }
        let Some(edit) = self.edit.as_mut() else {
            return false;
        };
        let before = edit.job.tools.len();
        edit.job.tools.retain(|t| t.id != id);
        if edit.job.tools.len() == before {
            return false;
        }
        self.commit();
        if self.focus == Focus::Tool(id) {
            self.focus = Focus::Tools;
        }
        true
    }
}
