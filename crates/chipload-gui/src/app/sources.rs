// SPDX-License-Identifier: GPL-3.0-or-later
//! The Drawing panel (docs/specs/drawing-sources.md §4): the project's DXFs with their
//! layers, its images and its drawn shapes by kind, each to select, hide, and for DXFs and
//! images to place, update, edit or remove.

use std::collections::BTreeMap;

use chipload_core::chain::ShapeFrom;
use chipload_job::Drawing;

use super::forms::{JobRequest, placement_rows};
use super::*;

/// The placement window's width, px.
const PLACE_WINDOW_WIDTH: f32 = 540.0;

/// One group of the Drawing panel.
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
            ("drawn:rectangle", "Drawn: rectangles"),
            ("drawn:circle", "Drawn: circles"),
            ("drawn:polyline", "Drawn: polylines"),
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

    /// Select a group: a DXF or image as a whole (so an operation made from it follows its
    /// updates), drawn shapes by their ids.
    pub fn select_group(&mut self, key: &str) {
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
            self.selection = CanvasSelection::Source(source);
            return;
        }
        let Some(edit) = &self.edit else {
            return;
        };
        let shapes: BTreeSet<u64> = edit
            .settings
            .drawing
            .shapes
            .iter()
            .filter(|s| Drawing::group_key(s) == key)
            .map(|s| s.id)
            .collect();
        self.selection = CanvasSelection::Shapes(shapes);
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
                if self.place_open == Some(id) {
                    self.place_open = None;
                }
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

    /// The groups as a tree: each with its eye (shown or hidden), its count and its
    /// buttons; a DXF's layers inside it.
    pub(super) fn drawing_tree(&mut self, ui: &mut egui::Ui) {
        let groups = self.drawing_groups();
        if groups.is_empty() {
            ui.weak("No shapes yet.");
            return;
        }
        let mut action: Option<PanelAction> = None;
        for group in &groups {
            let id = ui.make_persistent_id(("drawing-group", &group.key));
            let hidden = self.is_hidden_key(&group.key);
            // A DXF opens on its layers: they are what is selected and hidden most.
            let state = egui::collapsing_header::CollapsingState::load_with_default_open(
                ui.ctx(),
                id,
                true,
            );
            let header = |ui: &mut egui::Ui, action: &mut Option<PanelAction>| {
                eye(ui, hidden, &group.key, action);
                let title = RichText::new(&group.title);
                ui.label(if hidden { title.weak() } else { title.strong() });
                ui.weak(count_text(group.shapes));
                if ui
                    .small_button("Select")
                    .on_hover_text("Select every shape of it")
                    .clicked()
                {
                    *action = Some(PanelAction::SelectGroup(group.key.clone()));
                }
                if group.kind == GroupKind::Drawn {
                    return;
                }
                ui.menu_button("⋯", |ui| match group.kind {
                    GroupKind::Dxf(dxf) => {
                        if ui
                            .button("Place…")
                            .on_hover_text("Move, turn or scale this DXF")
                            .clicked()
                        {
                            *action = Some(PanelAction::Place(dxf));
                            ui.close();
                        }
                        if ui.button("Update from a DXF…").clicked() {
                            *action = Some(PanelAction::Update(dxf));
                            ui.close();
                        }
                        if ui.button("Update from a path…").clicked() {
                            *action = Some(PanelAction::UpdatePath(dxf));
                            ui.close();
                        }
                        ui.separator();
                        if ui
                            .button("Remove")
                            .on_hover_text("Remove this DXF and its shapes")
                            .clicked()
                        {
                            *action = Some(PanelAction::RemoveDxf(dxf));
                            ui.close();
                        }
                    }
                    GroupKind::Image(image) => {
                        if ui
                            .button("Edit…")
                            .on_hover_text("Change its size, place and tracing")
                            .clicked()
                        {
                            *action = Some(PanelAction::EditImage(image));
                            ui.close();
                        }
                        ui.separator();
                        if ui
                            .button("Remove")
                            .on_hover_text("Remove the picture and its shapes")
                            .clicked()
                        {
                            *action = Some(PanelAction::RemoveImage(image));
                            ui.close();
                        }
                    }
                    GroupKind::Drawn => {}
                });
            };
            if group.layers.is_empty() {
                ui.horizontal(|ui| {
                    ui.add_space(ui.spacing().indent);
                    header(ui, &mut action);
                });
                continue;
            }
            state
                .show_header(ui, |ui| header(ui, &mut action))
                .body(|ui| {
                    let from = match group.kind {
                        GroupKind::Dxf(dxf) => Some(ShapeFrom::Dxf(dxf)),
                        GroupKind::Image(image) => Some(ShapeFrom::Image(image)),
                        GroupKind::Drawn => None,
                    };
                    for (layer, count) in &group.layers {
                        let key = Drawing::layer_key(&group.key, layer);
                        let layer_hidden = hidden || self.is_hidden_key(&key);
                        ui.horizontal(|ui| {
                            eye(ui, self.is_hidden_key(&key), &key, &mut action);
                            let name = RichText::new(layer);
                            ui.label(if layer_hidden { name.weak() } else { name });
                            ui.weak(count_text(*count));
                            if ui
                                .small_button("Select")
                                .on_hover_text(
                                    "Select this layer of this DXF: an operation made from it \
                                     cuts only this DXF's layer",
                                )
                                .clicked()
                            {
                                action = Some(PanelAction::SelectLayer(layer.clone(), from));
                            }
                        });
                    }
                });
        }
        match action {
            Some(PanelAction::Toggle(key)) => self.toggle_hidden(&key),
            Some(PanelAction::SelectGroup(key)) => self.select_group(&key),
            Some(PanelAction::SelectLayer(layer, from)) => {
                self.selection = CanvasSelection::Layer(layer, from);
            }
            Some(PanelAction::Place(dxf)) => self.place_open = Some(dxf),
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
                self.edit_image(image);
            }
            Some(PanelAction::RemoveImage(image)) => {
                self.remove_image(image);
            }
            None => {}
        }
    }

    /// The placement window of the DXF whose *Place…* was pressed.
    pub(super) fn place_window(&mut self, ctx: &egui::Context) {
        let Some(dxf) = self.place_open else {
            return;
        };
        let Some(edit) = &self.edit else {
            self.place_open = None;
            return;
        };
        let drawing = &edit.settings.drawing;
        let Some(source) = drawing.dxfs.iter().find(|d| d.id == dxf).cloned() else {
            self.place_open = None;
            return;
        };
        let size = chipload_pipeline::placement::dxf_size(&drawing.shapes, dxf);
        let mut open = true;
        let mut request: Option<JobRequest> = None;
        egui::Window::new(format!("Place {}", source.name))
            .id(egui::Id::new("place-window"))
            .open(&mut open)
            .resizable(false)
            .collapsible(false)
            .fade_in(false)
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(ctx.content_rect().center())
            .show(ctx, |ui| {
                // Wide enough on its first frame for the longest row and its Reset button.
                ui.set_min_width(PLACE_WINDOW_WIDTH);
                egui::Grid::new("place-grid").num_columns(2).show(ui, |ui| {
                    placement_rows(ui, dxf, source.placement, size, &mut request);
                });
            });
        if !open {
            self.place_open = None;
        }
        if let Some(JobRequest::Place(id, how)) = request {
            self.place_dxf(Some(id), how);
        }
    }
}

/// The eye: checked is shown. A click asks to toggle `key`.
fn eye(ui: &mut egui::Ui, hidden: bool, key: &str, action: &mut Option<PanelAction>) {
    let mut shown = !hidden;
    if ui
        .checkbox(&mut shown, "")
        .on_hover_text(if hidden {
            "Hidden on the canvas (still cut): show it"
        } else {
            "Hide it on the canvas; it is still cut"
        })
        .changed()
    {
        *action = Some(PanelAction::Toggle(key.to_string()));
    }
}

fn count_text(n: usize) -> String {
    format!("{n} {}", if n == 1 { "shape" } else { "shapes" })
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
