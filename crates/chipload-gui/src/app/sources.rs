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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum GroupKind {
    Dxf(u64),
    Image(u64),
    Drawn,
}

/// What a click in the panel asks for, done after drawing it.
enum PanelAction {
    Toggle(String),
    SelectGroup(String),
    SelectLayer(String, Option<ShapeFrom>),
    Place(u64),
    Update(u64),
    UpdatePath(u64),
    RemoveDxf(u64),
    EditImage(u64),
    RemoveImage(u64),
    AddDxf,
    AddImage,
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
            groups.push(Group {
                key,
                title,
                kind,
                shapes,
                layers,
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
                format!("{} (image {})", image.file, image.id),
                GroupKind::Image(image.id),
            );
        }
        for (key, title) in [
            ("drawn:rectangle", "Rectangles"),
            ("drawn:circle", "Circles"),
            ("drawn:polyline", "Polylines"),
            ("paths", "Other paths"),
        ] {
            push(key.into(), title.into(), GroupKind::Drawn);
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
                self.error(problems);
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
                self.set_report(Report::info(lines));
                true
            }
            Err(err) => {
                self.error(vec!["Could not remove the DXF:".into(), err.to_string()]);
                false
            }
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
        let row = Row::new(egui::Id::new("browser-drawing"), 1, "Drawing")
            .icon(Icon::Drawing)
            .trailing(shapes.to_string());
        let (open, folder_row) = super::browser::folder(ui, row, true);
        let mut action: Option<PanelAction> = None;
        folder_row
            .response
            .on_hover_text("DXFs, pictures and drawn shapes; right-click to add one")
            .context_menu(|ui| {
                let adding = self.dxf_count() > 0;
                if ui
                    .add_enabled(adding, egui::Button::new("Add DXF…"))
                    .on_hover_text("Another part: a DXF placed and updated on its own")
                    .on_disabled_hover_text("File > Import DXF starts a project from one")
                    .clicked()
                {
                    action = Some(PanelAction::AddDxf);
                }
                if ui
                    .button("Import image…")
                    .on_hover_text("Trace a picture into shapes, for V-carving")
                    .clicked()
                {
                    action = Some(PanelAction::AddImage);
                }
            });
        if open {
            if groups.is_empty() {
                ui.horizontal(|ui| {
                    ui.add_space(4.0 + 3.0 * 14.0);
                    ui.weak("No shapes yet.");
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
                    .add_filter("DXF drawings", &["dxf"])
                    .pick_file()
                {
                    self.update_dxf_from(Some(dxf), &path, None);
                }
            }
            Some(PanelAction::UpdatePath(dxf)) => {
                self.prompt = Some(Prompt::Path(PathPurpose::UpdateDxf(dxf), String::new()));
            }
            Some(PanelAction::RemoveDxf(dxf)) => {
                self.remove_dxf(dxf);
            }
            Some(PanelAction::EditImage(image)) => {
                self.focus = Focus::Image(image);
                self.edit_image(image);
            }
            Some(PanelAction::RemoveImage(image)) => {
                self.remove_image(image);
            }
            Some(PanelAction::AddDxf) => self.ask_add_dxf(),
            Some(PanelAction::AddImage) => self.import_image_dialog(),
            None => {}
        }
    }

    /// One group's row, and a DXF's layers under it when it is open.
    fn group_rows(&self, ui: &mut egui::Ui, group: &Group, action: &mut Option<PanelAction>) {
        let hidden = self.is_hidden_key(&group.key);
        let icon = match (group.kind, group.key.as_str()) {
            (GroupKind::Dxf(_), _) => Icon::Dxf,
            (GroupKind::Image(_), _) => Icon::Image,
            (GroupKind::Drawn, "drawn:circle") => Icon::Circle,
            (GroupKind::Drawn, "drawn:rectangle") => Icon::Rectangle,
            (GroupKind::Drawn, "drawn:polyline") => Icon::Polyline,
            (GroupKind::Drawn, _) => Icon::Paths,
        };
        let id = egui::Id::new(("browser-group", &group.key));
        let open_id = id.with("folder-open");
        let has_layers = !group.layers.is_empty();
        let open = has_layers && super::browser::folder_open(ui, open_id, true);
        let mut row = Row::new(id, 2, &group.title)
            .eye(!hidden)
            .icon(icon)
            .trailing(group.shapes.to_string())
            .dim(hidden)
            .selected(self.selection == self.group_selection(&group.key));
        if has_layers {
            row = row.folder(open);
        }
        let out = row.show(ui);
        if out.toggled {
            super::browser::toggle_folder(ui, open_id, true);
        }
        if out.eye {
            *action = Some(PanelAction::Toggle(group.key.clone()));
        } else if out.response.double_clicked() {
            match group.kind {
                GroupKind::Dxf(dxf) => *action = Some(PanelAction::Place(dxf)),
                GroupKind::Image(image) => *action = Some(PanelAction::EditImage(image)),
                GroupKind::Drawn => {}
            }
        } else if out.response.clicked() {
            *action = Some(PanelAction::SelectGroup(group.key.clone()));
        }
        let hover = match group.kind {
            GroupKind::Dxf(_) => {
                "Click selects the whole DXF: an operation made from it follows its updates. \
                 Its position and scale show in Properties; right-click for more"
            }
            GroupKind::Image(_) => {
                "Click selects the whole picture: an operation made from it follows its \
                 retracing. Its size and place show in Properties; double-click for its trace settings"
            }
            GroupKind::Drawn => "Click selects these shapes; Ctrl+click adds them",
        };
        out.response.on_hover_text(hover).context_menu(|ui| {
            if ui.button("Select").clicked() {
                *action = Some(PanelAction::SelectGroup(group.key.clone()));
            }
            if ui.button(if hidden { "Show" } else { "Hide" }).clicked() {
                *action = Some(PanelAction::Toggle(group.key.clone()));
            }
            match group.kind {
                GroupKind::Dxf(dxf) => {
                    ui.separator();
                    if ui
                        .button("Place…")
                        .on_hover_text("Move, turn or scale this DXF, in its properties")
                        .clicked()
                    {
                        *action = Some(PanelAction::Place(dxf));
                    }
                    if ui.button("Update from a DXF…").clicked() {
                        *action = Some(PanelAction::Update(dxf));
                    }
                    if ui.button("Update from a path…").clicked() {
                        *action = Some(PanelAction::UpdatePath(dxf));
                    }
                    ui.separator();
                    if ui
                        .button("Remove")
                        .on_hover_text("Remove this DXF and its shapes")
                        .clicked()
                    {
                        *action = Some(PanelAction::RemoveDxf(dxf));
                    }
                }
                GroupKind::Image(image) => {
                    ui.separator();
                    if ui
                        .button("Trace settings…")
                        .on_hover_text("Threshold, smoothing, outlines or centre lines; its size and place are in its properties")
                        .clicked()
                    {
                        *action = Some(PanelAction::EditImage(image));
                    }
                    ui.separator();
                    if ui
                        .button("Remove")
                        .on_hover_text("Remove the picture and its shapes")
                        .clicked()
                    {
                        *action = Some(PanelAction::RemoveImage(image));
                    }
                }
                GroupKind::Drawn => {}
            }
        });
        if !open {
            return;
        }
        let from = match group.kind {
            GroupKind::Dxf(dxf) => Some(ShapeFrom::Dxf(dxf)),
            GroupKind::Image(image) => Some(ShapeFrom::Image(image)),
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
                .on_hover_text(
                    "Click selects this layer of this DXF: an operation made from it cuts only \
                     this DXF's layer",
                )
                .context_menu(|ui| {
                    if ui.button("Select").clicked() {
                        *action = Some(PanelAction::SelectLayer(layer.clone(), from));
                    }
                    if ui
                        .button(if layer_hidden { "Show" } else { "Hide" })
                        .clicked()
                    {
                        *action = Some(PanelAction::Toggle(key.clone()));
                    }
                });
        }
    }
}

/// The properties a group's row shows: a DXF's or a picture's.
fn source_focus(key: &str) -> Option<Focus> {
    if let Some(id) = key.strip_prefix("dxf:").and_then(|id| id.parse().ok()) {
        return Some(Focus::Dxf(id));
    }
    key.strip_prefix("image:")
        .and_then(|id| id.parse().ok())
        .map(Focus::Image)
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
