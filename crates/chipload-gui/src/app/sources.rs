// SPDX-License-Identifier: GPL-3.0-or-later
//! The Browser's Drawing folder (docs/specs/drawing-sources.md §4, browser-and-layout.md
//! §2): the project's DXFs with their layers, its images and its drawn shapes by kind, each
//! to select, hide, and for DXFs and images to place, update, edit or remove.

use std::collections::BTreeMap;

use chipload_core::chain::ShapeFrom;
use chipload_job::Drawing;

use super::browser::Row;
use super::*;
use crate::icons::Icon;

/// One group of the Browser's Drawing folder.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Group {
    /// Its `drawing.hidden` key: `dxf:1`, `image:2`, `drawn:circle`, …
    pub key: String,
    pub title: String,
    pub kind: GroupKind,
    pub shapes: usize,
    /// Its layers with their shape counts (DXFs only; an image has one layer).
    pub layers: Vec<(String, usize)>,
    /// Drawn shapes only: per layer, each shape's id and name, in the drawing's order
    /// (docs/specs/drawn-shapes-box.md §4).
    pub members: Vec<(String, Vec<Member>)>,
}

/// A drawn shape in a group: its id and name.
pub(super) type Member = (u64, Option<String>);

/// Groups with more shapes than this start closed.
const OPEN_GROUP_MOST: usize = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum GroupKind {
    Dxf(u64),
    Image(u64),
    Text(u64),
    Drawn,
}

/// A project changed by exploding or joining, with its report, or why not.
type ShapesChange = Result<
    (chipload_job::Job, chipload_pipeline::explode::ShapesReport),
    chipload_pipeline::PipelineError,
>;

/// What a click in the panel asks for, done after drawing it.
enum PanelAction {
    Toggle(String),
    SelectGroup(String),
    SelectLayer(String, Option<ShapeFrom>),
    Place(u64),
    Update(u64),
    UpdatePath(u64),
    RemoveDxf(u64),
    ExplodeDxf(u64),
    EditImage(u64),
    RemoveImage(u64),
    RemoveText(u64),
    /// A drawn shape's row: select it (or the shapes of a layer), rename, copy, delete it.
    SelectShapes(Vec<u64>),
    RenameShape(u64),
    CopyShape(u64),
    DeleteShape(u64),
    AddDxf,
    AddImage,
    AddText,
}

impl ChiploadApp {
    /// The panel's groups, in the project's order: DXFs, images, then drawn shapes.
    pub(super) fn drawing_groups(&self) -> Vec<Group> {
        let Some(edit) = &self.edit else {
            return Vec::new();
        };
        let drawing = &edit.settings.drawing;
        let mut counts: BTreeMap<String, (usize, BTreeMap<String, usize>)> = BTreeMap::new();
        for shape in &drawing.shapes {
            let entry = counts.entry(Drawing::group_key(shape)).or_default();
            entry.0 += 1;
            *entry.1.entry(shape.layer.clone()).or_default() += 1;
        }
        let mut groups = Vec::new();
        let mut push = |key: String, title: String, kind: GroupKind| {
            let (shapes, layers) = counts.get(&key).cloned().unwrap_or_default();
            if shapes == 0 && kind == GroupKind::Drawn {
                return;
            }
            let layers = match kind {
                GroupKind::Dxf(_) => layers.into_iter().collect(),
                _ => Vec::new(),
            };
            let mut members: Vec<(String, Vec<Member>)> = Vec::new();
            if kind == GroupKind::Drawn && key.starts_with("drawn:") {
                let mut by_layer: BTreeMap<String, Vec<Member>> = BTreeMap::new();
                for shape in drawing
                    .shapes
                    .iter()
                    .filter(|s| Drawing::group_key(s) == key)
                {
                    by_layer
                        .entry(shape.layer.clone())
                        .or_default()
                        .push((shape.id, shape.name.clone()));
                }
                members = by_layer.into_iter().collect();
            }
            groups.push(Group {
                key,
                title,
                kind,
                shapes,
                layers,
                members,
            });
        };
        for dxf in &drawing.dxfs {
            push(
                format!("dxf:{}", dxf.id),
                dxf.name.clone(),
                GroupKind::Dxf(dxf.id),
            );
        }
        for image in &drawing.images {
            push(
                format!("image:{}", image.id),
                t!(
                    self,
                    "sources-image-title",
                    file = image.file.as_str(),
                    id = image.id
                ),
                GroupKind::Image(image.id),
            );
        }
        for text in &drawing.texts {
            push(
                format!("text:{}", text.id),
                t!(
                    self,
                    "sources-text-title",
                    text = chipload_pipeline::texts::text_label(text),
                    id = text.id
                ),
                GroupKind::Text(text.id),
            );
        }
        for (key, title) in [
            ("drawn:rectangle", t!(self, "sources-rectangles")),
            ("drawn:circle", t!(self, "sources-circles")),
            ("drawn:polyline", t!(self, "sources-polylines")),
            ("paths", t!(self, "sources-other-paths")),
        ] {
            push(key.into(), title, GroupKind::Drawn);
        }
        groups
    }

    /// Whether a group or layer key is hidden.
    pub(super) fn is_hidden_key(&self, key: &str) -> bool {
        self.edit
            .as_ref()
            .is_some_and(|e| e.settings.drawing.hidden.iter().any(|h| h == key))
    }

    /// Hide or show a group or layer, as one undo step.
    pub fn toggle_hidden(&mut self, key: &str) {
        let Some(edit) = self.edit.as_mut() else {
            return;
        };
        let hidden = &mut edit.settings.drawing.hidden;
        if let Some(at) = hidden.iter().position(|h| h == key) {
            hidden.remove(at);
        } else {
            hidden.push(key.to_string());
        }
        self.commit();
        self.refresh_drawing();
    }

    /// Select a group: a DXF or image as a whole, drawn shapes by their ids.
    #[cfg(test)]
    pub fn select_group(&mut self, key: &str) {
        self.selection = self.group_selection(key);
    }

    /// Remove DXF `id` and its shapes, as one undo step.
    pub fn remove_dxf(&mut self, id: u64) -> bool {
        self.commit();
        let Some(edit) = &self.edit else {
            return false;
        };
        let job = match edit.to_job() {
            Ok(job) => job,
            Err(problems) => {
                self.error_report(Report::of_problems(
                    &t!(self, "sources-project-incomplete"),
                    &problems,
                ));
                return false;
            }
        };
        match chipload_pipeline::dxfs::remove_dxf(&job, id) {
            Ok((removed, lines)) => {
                let mut next = EditJob::from_job(&removed);
                for (entry, old) in next.ops.iter_mut().zip(&edit.ops) {
                    entry.side_chosen = old.side_chosen;
                }
                self.edit = Some(next);
                self.commit();
                self.refresh_drawing();
                self.set_report(Report::of_messages(lines));
                true
            }
            Err(err) => {
                self.error_report(Report::failed(
                    t!(self, "sources-remove-dxf-failed"),
                    err.message(),
                ));
                false
            }
        }
    }

    /// Ask how to explode DXF `id` (docs/specs/explode-and-join.md §5).
    pub(super) fn ask_explode(&mut self, dxf: u64) {
        self.dialogs.prompt = Some(Prompt::Explode {
            dxf,
            join: false,
            gap: chipload_job::explode::DEFAULT_JOIN_GAP_MM,
        });
    }

    /// Ask for the gap to join the selected drawn shapes, or every drawn shape when none
    /// is selected.
    pub(super) fn ask_join(&mut self) {
        let (ids, _) = self.deletable_shapes();
        self.dialogs.prompt = Some(Prompt::Join {
            ids,
            gap: chipload_job::explode::DEFAULT_JOIN_GAP_MM,
        });
    }

    /// Explode DXF `id` into drawn shapes, then join ends closer than `join` mm if given
    /// (docs/specs/explode-and-join.md), as one undo step. The new shapes are selected.
    pub fn explode_dxf(&mut self, id: u64, join: Option<f64>) -> bool {
        let done = self.changed_job(|job| chipload_pipeline::explode::explode_dxf(job, id, join));
        let failed = t!(self, "sources-explode-failed");
        let exploded = self.after_shapes_change(done, failed);
        if exploded && self.focus == Focus::Dxf(id) {
            self.focus = Focus::Job;
        }
        exploded
    }

    /// Join drawn shapes `ids` (none: every drawn shape) whose ends are closer than `gap` mm,
    /// as one undo step. The joined shapes are selected.
    pub fn join_shapes(&mut self, ids: &[u64], gap: f64) -> bool {
        let done = self.changed_job(|job| chipload_pipeline::explode::join_shapes(job, ids, gap));
        let failed = t!(self, "sources-join-failed");
        self.after_shapes_change(done, failed)
    }

    /// The project as `change` leaves it, or why not; None without a complete project (the
    /// report says why).
    fn changed_job(
        &mut self,
        change: impl FnOnce(&chipload_job::Job) -> ShapesChange,
    ) -> Option<ShapesChange> {
        self.commit();
        let edit = self.edit.as_ref()?;
        match edit.to_job() {
            Ok(job) => Some(change(&job)),
            Err(problems) => {
                self.error_report(Report::of_problems(
                    &t!(self, "sources-project-incomplete"),
                    &problems,
                ));
                None
            }
        }
    }

    /// Take the changed project as one undo step and show its report, or the error under
    /// `failed` (a text id).
    fn after_shapes_change(&mut self, done: Option<ShapesChange>, failed: String) -> bool {
        let Some(edit) = &self.edit else {
            return false;
        };
        match done {
            Some(Ok((job, report))) => {
                let mut next = EditJob::from_job(&job);
                for (entry, old) in next.ops.iter_mut().zip(&edit.ops) {
                    entry.side_chosen = old.side_chosen;
                }
                self.edit = Some(next);
                self.commit();
                self.drawing_changed(&report.shapes);
                self.set_report(Report::of_messages(report.messages));
                true
            }
            Some(Err(err)) => {
                self.error_report(Report::failed(failed, err.message()));
                false
            }
            None => false,
        }
    }

    /// The selection a group's row makes: a DXF or image as a whole (so an operation made
    /// from it follows its updates and retracing), drawn shapes by their ids.
    pub(super) fn group_selection(&self, key: &str) -> CanvasSelection {
        let source = key
            .strip_prefix("dxf:")
            .and_then(|id| id.parse().ok())
            .map(ShapeFrom::Dxf)
            .or_else(|| {
                key.strip_prefix("image:")
                    .and_then(|id| id.parse().ok())
                    .map(ShapeFrom::Image)
            })
            .or_else(|| {
                key.strip_prefix("text:")
                    .and_then(|id| id.parse().ok())
                    .map(ShapeFrom::Text)
            });
        if let Some(source) = source {
            return CanvasSelection::Source(source);
        }
        let Some(edit) = &self.edit else {
            return CanvasSelection::Nothing;
        };
        let shapes: BTreeSet<u64> = edit
            .settings
            .drawing
            .shapes
            .iter()
            .filter(|s| Drawing::group_key(s) == key)
            .map(|s| s.id)
            .collect();
        CanvasSelection::Shapes(shapes)
    }

    /// Select what a row stands for; with Ctrl, add its contours to the selection.
    pub(super) fn select_row(&mut self, selection: CanvasSelection, add: bool) {
        if !add || self.selection == CanvasSelection::Nothing {
            self.selection = selection;
            return;
        }
        let mut ids = self.selection.ids(&self.contours);
        ids.extend(selection.ids(&self.contours));
        self.selection = CanvasSelection::of_contours(&ids, &self.contours);
    }

    /// Drawing: each DXF with its layers, each image, and the drawn shapes by kind
    /// (docs/specs/browser-and-layout.md §2), with an eye to hide them.
    pub(super) fn drawing_rows(&mut self, ui: &mut egui::Ui) {
        let groups = self.drawing_groups();
        let shapes: usize = groups.iter().map(|g| g.shapes).sum();
        let row = Row::new(
            egui::Id::new("browser-drawing"),
            1,
            t!(ui, "sources-drawing"),
        )
        .icon(Icon::Drawing)
        .trailing(shapes.to_string());
        let (open, folder_row) = super::browser::folder(ui, row, true);
        let mut action: Option<PanelAction> = None;
        folder_row
            .response
            .on_hover_text(t!(ui, "sources-drawing-hover"))
            .context_menu(|ui| {
                let adding = self.dxf_count() > 0;
                if ui
                    .add_enabled(adding, egui::Button::new(t!(ui, "sources-add-dxf")))
                    .on_hover_text(t!(ui, "sources-add-dxf-hover"))
                    .on_disabled_hover_text(t!(ui, "sources-add-dxf-disabled-hover"))
                    .clicked()
                {
                    action = Some(PanelAction::AddDxf);
                }
                if ui
                    .button(t!(ui, "sources-import-image"))
                    .on_hover_text(t!(ui, "sources-import-image-hover"))
                    .clicked()
                {
                    action = Some(PanelAction::AddImage);
                }
                if ui
                    .button(t!(ui, "sources-add-text"))
                    .on_hover_text(t!(ui, "sources-add-text-hover"))
                    .clicked()
                {
                    action = Some(PanelAction::AddText);
                }
            });
        if open {
            if groups.is_empty() {
                ui.horizontal(|ui| {
                    ui.add_space(4.0 + 3.0 * 14.0);
                    ui.weak(t!(ui, "sources-no-shapes"));
                });
            }
            for group in &groups {
                self.group_rows(ui, group, &mut action);
            }
        }
        let add = ui.input(|i| i.modifiers.command);
        match action {
            Some(PanelAction::Toggle(key)) => self.toggle_hidden(&key),
            Some(PanelAction::SelectGroup(key)) => {
                let selection = self.group_selection(&key);
                self.select_row(selection, add);
                // A DXF or picture shows its properties too
                // (docs/specs/properties-for-tools-and-dxfs.md §3, §3b).
                if let Some(focus) = source_focus(&key) {
                    self.focus = focus;
                }
            }
            Some(PanelAction::SelectLayer(layer, from)) => {
                self.select_row(CanvasSelection::Layer(layer, from), add);
                if let Some(ShapeFrom::Dxf(dxf)) = from {
                    self.focus = Focus::Dxf(dxf);
                }
            }
            Some(PanelAction::Place(dxf)) => self.focus = Focus::Dxf(dxf),
            Some(PanelAction::Update(dxf)) => {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter(t!(ui, "sources-dxf-filter"), &["dxf"])
                    .pick_file()
                {
                    self.update_dxf_from(Some(dxf), &path, None);
                }
            }
            Some(PanelAction::UpdatePath(dxf)) => {
                self.dialogs.prompt =
                    Some(Prompt::Path(PathPurpose::UpdateDxf(dxf), String::new()));
            }
            Some(PanelAction::RemoveDxf(dxf)) => {
                self.remove_dxf(dxf);
            }
            Some(PanelAction::ExplodeDxf(dxf)) => self.ask_explode(dxf),
            Some(PanelAction::EditImage(image)) => {
                self.focus = Focus::Image(image);
                self.edit_image(image);
            }
            Some(PanelAction::RemoveImage(image)) => {
                self.remove_image(image);
            }
            Some(PanelAction::RemoveText(text)) => {
                self.remove_text(text);
            }
            Some(PanelAction::SelectShapes(ids)) => {
                self.select_row(CanvasSelection::Shapes(ids.into_iter().collect()), add);
            }
            Some(PanelAction::RenameShape(id)) => {
                self.selection = CanvasSelection::Shapes(BTreeSet::from([id]));
                self.grab_shape_name = true;
            }
            Some(PanelAction::CopyShape(id)) => {
                self.selection = CanvasSelection::Shapes(BTreeSet::from([id]));
                if let Some(text) = self.copy_selected_shapes() {
                    ui.ctx().copy_text(text);
                }
            }
            Some(PanelAction::DeleteShape(id)) => {
                self.selection = CanvasSelection::Shapes(BTreeSet::from([id]));
                self.delete_selected_shapes();
            }
            Some(PanelAction::AddDxf) => self.ask_add_dxf(),
            Some(PanelAction::AddImage) => self.import_image_dialog(),
            Some(PanelAction::AddText) => self.start_text(),
            None => {}
        }
    }

    /// One group's row, and a DXF's layers under it when it is open.
    fn group_rows(&self, ui: &mut egui::Ui, group: &Group, action: &mut Option<PanelAction>) {
        let hidden = self.is_hidden_key(&group.key);
        let icon = match (group.kind, group.key.as_str()) {
            (GroupKind::Dxf(_), _) => Icon::Dxf,
            (GroupKind::Image(_), _) => Icon::Image,
            (GroupKind::Text(_), _) => Icon::Text,
            (GroupKind::Drawn, "drawn:circle") => Icon::Circle,
            (GroupKind::Drawn, "drawn:rectangle") => Icon::Rectangle,
            (GroupKind::Drawn, "drawn:polyline") => Icon::Polyline,
            (GroupKind::Drawn, _) => Icon::Paths,
        };
        let id = egui::Id::new(("browser-group", &group.key));
        let open_id = id.with("folder-open");
        let has_layers = !group.layers.is_empty() || !group.members.is_empty();
        let open_first = group.members.is_empty() || group.shapes <= OPEN_GROUP_MOST;
        let open = has_layers && super::browser::folder_open(ui, open_id, open_first);
        let mut row = Row::new(id, 2, &group.title)
            .eye(!hidden)
            .icon(icon)
            .trailing(group.shapes.to_string())
            .dim(hidden)
            .selected(match (&self.selection, self.group_selection(&group.key)) {
                // Some of it left out: still its row (docs/specs/leave-out-shapes.md §5).
                (CanvasSelection::Except(from, _), CanvasSelection::Source(source)) => {
                    *from == source
                }
                (selection, whole) => *selection == whole,
            });
        if has_layers {
            row = row.folder(open);
        }
        let out = row.show(ui);
        if out.toggled {
            super::browser::toggle_folder(ui, open_id, open_first);
        }
        if out.eye {
            *action = Some(PanelAction::Toggle(group.key.clone()));
        } else if out.response.double_clicked() {
            match group.kind {
                GroupKind::Dxf(dxf) => *action = Some(PanelAction::Place(dxf)),
                GroupKind::Image(image) => *action = Some(PanelAction::EditImage(image)),
                GroupKind::Text(_) | GroupKind::Drawn => {}
            }
        } else if out.response.clicked() {
            *action = Some(PanelAction::SelectGroup(group.key.clone()));
        }
        let hover = match group.kind {
            GroupKind::Dxf(_) => t!(ui, "sources-dxf-hover"),
            GroupKind::Image(_) => t!(ui, "sources-image-hover"),
            GroupKind::Text(_) => t!(ui, "sources-text-hover"),
            GroupKind::Drawn => t!(ui, "sources-drawn-hover"),
        };
        out.response.on_hover_text(hover).context_menu(|ui| {
            if ui.button(t!(ui, "sources-select")).clicked() {
                *action = Some(PanelAction::SelectGroup(group.key.clone()));
            }
            if ui
                .button(if hidden {
                    t!(ui, "sources-show")
                } else {
                    t!(ui, "sources-hide")
                })
                .clicked()
            {
                *action = Some(PanelAction::Toggle(group.key.clone()));
            }
            match group.kind {
                GroupKind::Dxf(dxf) => {
                    ui.separator();
                    if ui
                        .button(t!(ui, "sources-place"))
                        .on_hover_text(t!(ui, "sources-place-hover"))
                        .clicked()
                    {
                        *action = Some(PanelAction::Place(dxf));
                    }
                    if ui.button(t!(ui, "sources-update-from-dxf")).clicked() {
                        *action = Some(PanelAction::Update(dxf));
                    }
                    if ui.button(t!(ui, "sources-update-from-path")).clicked() {
                        *action = Some(PanelAction::UpdatePath(dxf));
                    }
                    if ui
                        .button(t!(ui, "sources-explode"))
                        .on_hover_text(t!(ui, "sources-explode-hover"))
                        .clicked()
                    {
                        *action = Some(PanelAction::ExplodeDxf(dxf));
                    }
                    ui.separator();
                    if ui
                        .button(t!(ui, "common-remove"))
                        .on_hover_text(t!(ui, "sources-remove-dxf-hover"))
                        .clicked()
                    {
                        *action = Some(PanelAction::RemoveDxf(dxf));
                    }
                }
                GroupKind::Image(image) => {
                    ui.separator();
                    if ui
                        .button(t!(ui, "sources-trace-settings"))
                        .on_hover_text(t!(ui, "sources-trace-settings-hover"))
                        .clicked()
                    {
                        *action = Some(PanelAction::EditImage(image));
                    }
                    ui.separator();
                    if ui
                        .button(t!(ui, "common-remove"))
                        .on_hover_text(t!(ui, "sources-remove-image-hover"))
                        .clicked()
                    {
                        *action = Some(PanelAction::RemoveImage(image));
                    }
                }
                GroupKind::Text(text) => {
                    ui.separator();
                    if ui
                        .button(t!(ui, "common-remove"))
                        .on_hover_text(t!(ui, "sources-remove-text-hover"))
                        .clicked()
                    {
                        *action = Some(PanelAction::RemoveText(text));
                    }
                }
                GroupKind::Drawn => {}
            }
        });
        if !open {
            return;
        }
        if !group.members.is_empty() {
            self.drawn_member_rows(ui, group, icon, hidden, action);
            return;
        }
        let from = match group.kind {
            GroupKind::Dxf(dxf) => Some(ShapeFrom::Dxf(dxf)),
            GroupKind::Image(image) => Some(ShapeFrom::Image(image)),
            GroupKind::Text(text) => Some(ShapeFrom::Text(text)),
            GroupKind::Drawn => None,
        };
        for (layer, count) in &group.layers {
            let key = Drawing::layer_key(&group.key, layer);
            let layer_hidden = self.is_hidden_key(&key);
            let selection = CanvasSelection::Layer(layer.clone(), from);
            let out = Row::new(egui::Id::new(("browser-layer", &key)), 3, layer)
                .eye(!layer_hidden)
                .icon(Icon::Layer)
                .trailing(count.to_string())
                .dim(hidden || layer_hidden)
                .selected(self.selection == selection)
                .show(ui);
            if out.eye {
                *action = Some(PanelAction::Toggle(key.clone()));
            } else if out.response.clicked() {
                *action = Some(PanelAction::SelectLayer(layer.clone(), from));
            }
            out.response
                .on_hover_text(t!(ui, "sources-layer-hover"))
                .context_menu(|ui| {
                    if ui.button(t!(ui, "sources-select")).clicked() {
                        *action = Some(PanelAction::SelectLayer(layer.clone(), from));
                    }
                    if ui
                        .button(if layer_hidden {
                            t!(ui, "sources-show")
                        } else {
                            t!(ui, "sources-hide")
                        })
                        .clicked()
                    {
                        *action = Some(PanelAction::Toggle(key.clone()));
                    }
                });
        }
    }
}

impl ChiploadApp {
    /// A drawn group's shapes: a row each, under a folder per layer when it has several
    /// (docs/specs/drawn-shapes-box.md §4).
    fn drawn_member_rows(
        &self,
        ui: &mut egui::Ui,
        group: &Group,
        icon: Icon,
        hidden: bool,
        action: &mut Option<PanelAction>,
    ) {
        if let [(_, shapes)] = &group.members[..] {
            for (id, name) in shapes {
                self.shape_row(ui, 3, *id, name.as_deref(), icon, hidden, action);
            }
            return;
        }
        for (layer, shapes) in &group.members {
            let key = Drawing::layer_key(&group.key, layer);
            let layer_hidden = self.is_hidden_key(&key);
            let ids: Vec<u64> = shapes.iter().map(|(id, _)| *id).collect();
            let id = egui::Id::new(("browser-drawn-layer", &key));
            let open_id = id.with("folder-open");
            let open_first = shapes.len() <= OPEN_GROUP_MOST;
            let open = super::browser::folder_open(ui, open_id, open_first);
            let selected = matches!(&self.selection, CanvasSelection::Shapes(s)
                if s.len() == ids.len() && ids.iter().all(|i| s.contains(i)));
            let out = Row::new(id, 3, layer)
                .eye(!layer_hidden)
                .icon(Icon::Layer)
                .trailing(shapes.len().to_string())
                .dim(hidden || layer_hidden)
                .selected(selected)
                .folder(open)
                .show(ui);
            if out.toggled {
                super::browser::toggle_folder(ui, open_id, open_first);
            }
            if out.eye {
                *action = Some(PanelAction::Toggle(key.clone()));
            } else if out.response.clicked() {
                *action = Some(PanelAction::SelectShapes(ids.clone()));
            }
            out.response
                .on_hover_text(t!(ui, "sources-drawn-layer-hover"))
                .context_menu(|ui| {
                    if ui.button(t!(ui, "sources-select")).clicked() {
                        *action = Some(PanelAction::SelectShapes(ids.clone()));
                    }
                    if ui
                        .button(if layer_hidden {
                            t!(ui, "sources-show")
                        } else {
                            t!(ui, "sources-hide")
                        })
                        .clicked()
                    {
                        *action = Some(PanelAction::Toggle(key.clone()));
                    }
                });
            if open {
                for (id, name) in shapes {
                    let dim = hidden || layer_hidden;
                    self.shape_row(ui, 4, *id, name.as_deref(), icon, dim, action);
                }
            }
        }
    }

    /// One drawn shape's row: its name (its id beside it), or its id.
    #[allow(clippy::too_many_arguments)]
    fn shape_row(
        &self,
        ui: &mut egui::Ui,
        depth: usize,
        id: u64,
        name: Option<&str>,
        icon: Icon,
        dim: bool,
        action: &mut Option<PanelAction>,
    ) {
        let label = format!("S{id}");
        let mut row = Row::new(
            egui::Id::new(("browser-shape", id)),
            depth,
            name.unwrap_or(&label),
        )
        .icon(icon)
        .dim(dim)
        .selected(matches!(&self.selection, CanvasSelection::Shapes(s) if s.contains(&id)));
        if name.is_some() {
            row = row.trailing(label);
        }
        let out = row.show(ui);
        if out.response.clicked() {
            *action = Some(PanelAction::SelectShapes(vec![id]));
        }
        out.response
            .on_hover_text(t!(ui, "sources-shape-hover"))
            .context_menu(|ui| {
                if ui.button(t!(ui, "sources-select")).clicked() {
                    *action = Some(PanelAction::SelectShapes(vec![id]));
                }
                if ui.button(t!(ui, "sources-rename")).clicked() {
                    *action = Some(PanelAction::RenameShape(id));
                }
                if ui.button(t!(ui, "sources-copy-shape")).clicked() {
                    *action = Some(PanelAction::CopyShape(id));
                }
                ui.separator();
                if ui.button(t!(ui, "sources-delete-shape")).clicked() {
                    *action = Some(PanelAction::DeleteShape(id));
                }
            });
    }
}

/// The properties a group's row shows: a DXF's, a picture's or a text's.
fn source_focus(key: &str) -> Option<Focus> {
    if let Some(id) = key.strip_prefix("dxf:").and_then(|id| id.parse().ok()) {
        return Some(Focus::Dxf(id));
    }
    if let Some(id) = key.strip_prefix("image:").and_then(|id| id.parse().ok()) {
        return Some(Focus::Image(id));
    }
    key.strip_prefix("text:")
        .and_then(|id| id.parse().ok())
        .map(Focus::Text)
}

/// The ids of the contours hidden in `drawing`: those whose every shape is hidden.
pub(super) fn hidden_contours(drawing: &Drawing, contours: &[Contour]) -> BTreeSet<usize> {
    if drawing.hidden.is_empty() {
        return BTreeSet::new();
    }
    let hidden: BTreeSet<u64> = drawing
        .shapes
        .iter()
        .filter(|s| drawing.is_hidden(s))
        .map(|s| s.id)
        .collect();
    contours
        .iter()
        .filter(|c| {
            c.parts
                .iter()
                .all(|p| p.segment.shape.is_some_and(|s| hidden.contains(&s)))
        })
        .map(|c| c.id)
        .collect()
}
