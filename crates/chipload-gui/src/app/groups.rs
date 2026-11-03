// SPDX-License-Identifier: GPL-3.0-or-later
//! Groups of drawn shapes and texts in the window (docs/specs/shape-groups.md §5): picked
//! whole with a click, as a picture or a DXF is; made, ungrouped, added to and taken from;
//! moved, turned and scaled as one. The rules are the pipeline's
//! (`chipload_pipeline::groups`); this takes the selection and shows the result.

use std::collections::BTreeSet;

use chipload_core::Point;
use chipload_job::{Geometry, Job};
use chipload_pipeline::groups::{
    Members, add_to_group, delete_group, group, remove_from_group, rename_group, transform_group,
    transformed_segment, ungroup,
};

use super::*;

impl ChiploadApp {
    /// The group contour `id` is wholly in, if any.
    pub(super) fn group_of_contour(&self, id: usize) -> Option<u64> {
        let drawing = &self.edit.as_ref()?.job.drawing;
        let contour = self.contours.iter().find(|c| c.id == id)?;
        let mut groups = shapes_of(contour).into_iter().map(|s| drawing.group_of(s));
        let first = groups.next()??;
        groups.all(|g| g == Some(first)).then_some(first)
    }

    /// Contour `id` selected alone, as a double-click on a grouped shape or a DXF's contour
    /// picks it (docs/specs/shape-groups.md decision 3, dxf-whole-on-canvas.md decision 1).
    pub fn pick_alone(&mut self, id: usize) {
        if let Some(contour) = self.contours.iter().find(|c| c.id == id) {
            self.selection = CanvasSelection::Shapes(shapes_of(contour));
            self.focus = Focus::Selection;
        }
    }

    /// Whether contour `id` is a DXF's, all of it.
    pub(super) fn dxf_contour(&self, id: usize) -> bool {
        self.contours.iter().find(|c| c.id == id).is_some_and(|c| {
            let from = c.parts.first().and_then(|p| p.segment.from);
            matches!(from, Some(chipload_core::chain::ShapeFrom::Dxf(_)))
                && c.parts.iter().all(|p| p.segment.from == from)
        })
    }

    /// The selected group, if a group is what is selected.
    pub(super) fn selected_group(&self) -> Option<u64> {
        match self.selection.source() {
            Some(Source::Group(id)) => Some(id),
            _ => None,
        }
    }

    /// Group `id`'s name; empty when there is no such group.
    pub(super) fn group_name(&self, id: u64) -> String {
        self.edit
            .as_ref()
            .and_then(|e| e.job.drawing.groups.iter().find(|g| g.id == id))
            .map_or_else(String::new, |g| g.name.clone())
    }

    /// What the selection is made of, as group members: drawn shapes and texts.
    fn selected_members(&self) -> Option<Members> {
        let edit = self.edit.as_ref()?;
        Members::of(&edit.job, &self.framed_shape_ids(), &[]).ok()
    }

    /// Whether *Group* can be used: two or more things selected, all drawn shapes or
    /// texts; or one free thing with a group.
    pub(super) fn can_group(&self) -> bool {
        let Some(members) = self.selected_members() else {
            return false;
        };
        let Some(edit) = self.edit.as_ref() else {
            return false;
        };
        let groups: BTreeSet<Option<u64>> = members
            .shapes
            .iter()
            .map(|s| edit.job.drawing.group_of(*s))
            .chain(
                members
                    .texts
                    .iter()
                    .map(|t| edit.job.drawing.group_of_text(*t)),
            )
            .collect();
        // Already all in one group: nothing to do.
        let all_in_one = groups.len() == 1 && !groups.contains(&None);
        members.shapes.len() + members.texts.len() > 1 && !all_in_one
    }

    /// The project as it is now.
    fn current_job(&self) -> Option<Job> {
        self.edit.as_ref().map(|e| e.job.clone())
    }

    /// A changed project taken in: drawing and operations, as one undo step.
    fn take_job(&mut self, job: Job) {
        let Some(edit) = self.edit.as_mut() else {
            return;
        };
        edit.adopt(job);
        self.shape_draft = None;
        self.refresh_drawing();
    }

    fn group_failed(&mut self, err: &PipelineError) {
        let head = t!(self, "groups-failed");
        self.error_report(Report::failed(head, err.message()));
    }

    /// *Group* (Ctrl+G): the selection becomes a group, joins the one group in it, or
    /// merges the groups in it (decision 9). The group is selected.
    pub fn group_selected(&mut self) -> bool {
        let (Some(job), Some(members)) = (self.current_job(), self.selected_members()) else {
            return false;
        };
        match group(&job, &members, None) {
            Ok((job, id)) => {
                self.take_job(job);
                self.selection = CanvasSelection::whole(Source::Group(id));
                self.focus = Focus::Selection;
                true
            }
            Err(err) => {
                self.group_failed(&err);
                false
            }
        }
    }

    /// *Ungroup* (Ctrl+Shift+G): the selected group's shapes stay, free and selected.
    pub fn ungroup_selected(&mut self) -> bool {
        let (Some(job), Some(id)) = (self.current_job(), self.selected_group()) else {
            return false;
        };
        let shapes: BTreeSet<u64> = job.drawing.group_shapes(id).into_iter().collect();
        match ungroup(&job, id) {
            Ok(job) => {
                self.take_job(job);
                self.selection = CanvasSelection::Shapes(shapes);
                true
            }
            Err(err) => {
                self.group_failed(&err);
                false
            }
        }
    }

    /// *Remove from group*: the selected shapes (picked inside their group) go free.
    pub fn remove_selected_from_group(&mut self) -> bool {
        let (Some(job), Some(members)) = (self.current_job(), self.selected_members()) else {
            return false;
        };
        match remove_from_group(&job, &members) {
            Ok(job) => {
                self.take_job(job);
                true
            }
            Err(err) => {
                self.group_failed(&err);
                false
            }
        }
    }

    /// *Add to group ▸*: the selection joins group `id`.
    pub fn add_selected_to_group(&mut self, id: u64) -> bool {
        let (Some(job), Some(members)) = (self.current_job(), self.selected_members()) else {
            return false;
        };
        match add_to_group(&job, id, &members) {
            Ok(job) => {
                self.take_job(job);
                self.selection = CanvasSelection::whole(Source::Group(id));
                true
            }
            Err(err) => {
                self.group_failed(&err);
                false
            }
        }
    }

    /// Delete the selected group with its shapes and texts (Del).
    pub fn delete_selected_group(&mut self) -> bool {
        let (Some(job), Some(id)) = (self.current_job(), self.selected_group()) else {
            return false;
        };
        match delete_group(&job, id) {
            Ok(job) => {
                self.take_job(job);
                self.selection = CanvasSelection::Nothing;
                true
            }
            Err(err) => {
                self.group_failed(&err);
                false
            }
        }
    }

    /// Rename group `id`.
    pub fn rename_group(&mut self, id: u64, name: &str) -> bool {
        let Some(job) = self.current_job() else {
            return false;
        };
        match rename_group(&job, id, name) {
            Ok(job) => {
                self.take_job(job);
                true
            }
            Err(err) => {
                self.group_failed(&err);
                false
            }
        }
    }

    /// Group `id` moved, turned and scaled, as one undo step; it stays selected.
    pub fn transform_group(
        &mut self,
        id: u64,
        from: Point,
        to: Point,
        scale: f64,
        degrees: f64,
    ) -> bool {
        let Some(job) = self.current_job() else {
            return false;
        };
        let left_out = match &self.selection {
            CanvasSelection::Whole { left_out, .. } => left_out.clone(),
            _ => BTreeSet::new(),
        };
        match transform_group(&job, id, from, to, scale, degrees) {
            Ok(job) => {
                self.take_job(job);
                self.selection = CanvasSelection::Whole {
                    of: Source::Group(id),
                    left_out,
                };
                true
            }
            Err(err) => {
                self.group_failed(&err);
                false
            }
        }
    }

    /// The outlines of shapes `ids` (a group's, a DXF's) moved, turned and scaled as a drag
    /// has them, for its preview.
    pub(super) fn moved_outlines(
        &self,
        ids: &[u64],
        from: Point,
        to: Point,
        scale: f64,
        degrees: f64,
    ) -> Vec<Vec<Point>> {
        let Some(edit) = self.edit.as_ref() else {
            return Vec::new();
        };
        let drawing = &edit.job.drawing;
        let (from, to) = ([from.0, from.1], [to.0, to.1]);
        drawing
            .shapes
            .iter()
            .filter(|s| ids.contains(&s.id))
            .filter_map(|s| match &s.geometry {
                Geometry::Path { segments, .. } => {
                    let mut geometry = s.geometry.clone();
                    if let Geometry::Path {
                        segments: moved, ..
                    } = &mut geometry
                    {
                        *moved = segments
                            .iter()
                            .map(|seg| transformed_segment(seg, from, to, scale, degrees))
                            .collect();
                    }
                    Some(geometry)
                }
                other => chipload_job::edit::transformed(other, from, to, scale, degrees),
            })
            .flat_map(|g| crate::draw::outline(&g))
            .collect()
    }

    /// The selection's items of the canvas's right-click menu and of DRAW ▾: Group,
    /// Ungroup, Add to group ▸, Remove from group (docs/specs/shape-groups.md §5), and Save
    /// to shape library… (shape-library.md §5).
    pub(super) fn selection_menu(&mut self, ui: &mut egui::Ui) {
        use super::shortcuts::{Action, with_keys};
        if ui
            .add_enabled(
                self.can_group(),
                egui::Button::new(with_keys(&t!(ui, "groups-group"), Action::Group)),
            )
            .on_hover_text(t!(ui, "groups-group-hover"))
            .clicked()
        {
            self.group_selected();
            ui.close();
        }
        if ui
            .add_enabled(
                self.selected_group().is_some(),
                egui::Button::new(with_keys(&t!(ui, "groups-ungroup"), Action::Ungroup)),
            )
            .clicked()
        {
            self.ungroup_selected();
            ui.close();
        }
        let groups: Vec<(u64, String)> = self
            .edit
            .as_ref()
            .map(|e| {
                e.job
                    .drawing
                    .groups
                    .iter()
                    .map(|g| (g.id, g.name.clone()))
                    .collect()
            })
            .unwrap_or_default();
        let free = self.selected_group().is_none() && self.selected_members().is_some();
        ui.add_enabled_ui(free && !groups.is_empty(), |ui| {
            super::header::submenu(ui, t!(ui, "groups-add-to"), |ui| {
                for (id, name) in &groups {
                    if ui.button(name).clicked() {
                        self.add_selected_to_group(*id);
                        ui.close();
                    }
                }
            });
        });
        let inside = self.selected_group().is_none()
            && self.selected_members().is_some_and(|m| {
                self.edit.as_ref().is_some_and(|e| {
                    m.shapes
                        .iter()
                        .any(|s| e.job.drawing.group_of(*s).is_some())
                        || m.texts
                            .iter()
                            .any(|t| e.job.drawing.group_of_text(*t).is_some())
                })
            });
        if ui
            .add_enabled(inside, egui::Button::new(t!(ui, "groups-remove-from")))
            .on_hover_text(t!(ui, "groups-remove-from-hover"))
            .clicked()
        {
            self.remove_selected_from_group();
            ui.close();
        }
        ui.separator();
        if ui
            .add_enabled(
                self.user.settings_dir.is_some(),
                egui::Button::new(t!(ui, "shape-library-save-menu")),
            )
            .clicked()
        {
            let group = self.shape_library_group();
            self.ask_library_save(group);
            ui.close();
        }
        // A whole DXF, picture or text: explode it (explode-pictures-and-texts.md §6).
        if let CanvasSelection::Whole { of, .. } = &self.selection
            && !matches!(of, Source::Group(_))
        {
            let of = *of;
            let hover = match of {
                Source::Image(_) => t!(ui, "sources-explode-image-hover"),
                Source::Text(_) => t!(ui, "sources-explode-text-hover"),
                Source::Dxf(_) | Source::Group(_) => t!(ui, "sources-explode-hover"),
            };
            if ui
                .button(t!(ui, "sources-explode"))
                .on_hover_text(hover)
                .clicked()
            {
                self.ask_explode(of);
                ui.close();
            }
        }
    }

    /// Under Selection, a group's name, what is in it, and Ungroup.
    pub(super) fn group_section(&mut self, ui: &mut egui::Ui) {
        let Some(id) = self.selected_group() else {
            return;
        };
        let Some(group) = self
            .edit
            .as_ref()
            .and_then(|e| e.job.drawing.groups.iter().find(|g| g.id == id))
            .cloned()
        else {
            return;
        };
        egui::Grid::new("group-section")
            .num_columns(2)
            .show(ui, |ui| {
                ui.label(t!(ui, "groups-name"));
                let mut name = group.name.clone();
                let response = ui.text_edit_singleline(&mut name);
                if response.lost_focus() && name.trim() != group.name && !name.trim().is_empty() {
                    self.rename_group(id, &name);
                }
                ui.end_row();
                ui.label(t!(ui, "groups-holds"));
                ui.label(t!(
                    ui,
                    "groups-holds-what",
                    shapes = group.shapes.len(),
                    texts = group.texts.len()
                ));
                ui.end_row();
            });
        ui.horizontal(|ui| {
            if ui
                .button(t!(ui, "groups-ungroup"))
                .on_hover_text(t!(ui, "groups-ungroup-hover"))
                .clicked()
            {
                self.ungroup_selected();
            }
        });
        ui.separator();
    }
}
