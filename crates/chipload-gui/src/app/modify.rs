// SPDX-License-Identifier: GPL-3.0-or-later
//! Editing tools on drawn shapes (docs/specs/drawing-editing.md): the tool being used, its
//! preview and its clicks. The geometry is `chipload_pipeline::drawing_edit`.

use chipload_pipeline::PipelineError;
use chipload_pipeline::drawing_edit::{
    Corners, Cut, Edited, OffsetSide, break_shape, cut_shape_corners, extend_shape, nearer_end,
    nearest_drawn, nearest_point, offset_at, offset_shapes, shape_corners, trim_preview,
    trim_shape,
};

use super::*;

/// An editing tool.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModifyTool {
    /// Offset the selected shapes to the cursor's side (§2).
    Offset,
    /// Round a corner of the selected shape (Shift: every corner) (§3).
    Fillet,
    /// Cut a corner of the selected shape straight across (Shift: every corner) (§3).
    Chamfer,
    /// Remove the piece of a shape under the cursor, between crossings (§4).
    Trim,
    /// Grow an open shape's straight end to the next shape ahead (§4).
    Extend,
    /// Split a shape at the point under the cursor (§4).
    Break,
}

impl ModifyTool {
    /// The tools that work on the shape under the cursor, with no selection first.
    pub fn on_shape_under_cursor(self) -> bool {
        matches!(
            self,
            ModifyTool::Trim | ModifyTool::Extend | ModifyTool::Break
        )
    }
}

/// The offset the cursor asks for, and its preview, kept until either changes.
#[derive(Debug, Clone, Default)]
pub(super) struct OffsetPreview {
    key: Option<(Vec<u64>, u64, OffsetSide, Corners, u32)>,
    pub lines: Vec<Vec<Point>>,
}

impl ChiploadApp {
    pub fn start_modify(&mut self, tool: ModifyTool) {
        if !self.can_draw() {
            return;
        }
        if self.three_d {
            self.view_top();
        }
        self.sketch = None;
        self.measure = None;
        self.placing_text = false;
        self.typed = None;
        self.modify = Some(tool);
    }

    pub fn cancel_modify(&mut self) {
        self.modify = None;
        self.typed = None;
        self.offset_preview = OffsetPreview::default();
    }

    /// The drawn shapes selected (none when imported ones are among them).
    pub(super) fn selected_drawn(&self) -> Vec<u64> {
        let (ids, imported) = self.deletable_shapes();
        if imported { Vec::new() } else { ids }
    }

    /// The offset the cursor at `at` asks for: its distance from the nearest selected
    /// shape, on the grid unless `free` (Alt), and the side it is on.
    pub fn offset_for_cursor(&self, at: Point, free: bool) -> Option<(f64, OffsetSide)> {
        let job = &self.edit.as_ref()?.settings;
        let (d, side) = offset_at(job, &self.selected_drawn(), at)?;
        let d = if free {
            d
        } else {
            let step = grid_step(self.view.scale, self.units);
            crate::draw::snap((d, 0.0), step, self.units).0
        };
        (d > 0.0).then_some((d, side))
    }

    /// The outlines the offset would add, for the preview.
    pub(super) fn offset_outlines(&mut self, d: f64, side: OffsetSide) -> Vec<Vec<Point>> {
        let ids = self.selected_drawn();
        let key = (
            ids.clone(),
            d.to_bits(),
            side,
            self.offset_corners,
            self.offset_count,
        );
        if self.offset_preview.key.as_ref() != Some(&key) {
            let lines = self
                .edit
                .as_ref()
                .and_then(|e| {
                    offset_shapes(
                        &e.settings,
                        &ids,
                        d,
                        Some(side),
                        self.offset_corners,
                        self.offset_count,
                    )
                    .ok()
                })
                .map(|edited| {
                    edited
                        .job
                        .drawing
                        .shapes
                        .iter()
                        .filter(|s| edited.new_ids.contains(&s.id))
                        .flat_map(|s| crate::draw::outline(&s.geometry))
                        .collect()
                })
                .unwrap_or_default();
            self.offset_preview = OffsetPreview {
                key: Some(key),
                lines,
            };
        }
        self.offset_preview.lines.clone()
    }

    /// Offset the selected shapes by `d` to `side`, as one undo step; the new shapes are
    /// selected, and the status bar says which operations now cut them.
    pub fn apply_offset(&mut self, d: f64, side: OffsetSide) -> bool {
        let ids = self.selected_drawn();
        let Some(edit) = self.edit.as_mut() else {
            return false;
        };
        let mut job = edit.settings.clone();
        job.operations = edit.ops.iter().map(|e| e.op.clone()).collect();
        let result = offset_shapes(
            &job,
            &ids,
            d,
            Some(side),
            self.offset_corners,
            self.offset_count,
        );
        match result {
            Ok(edited) => {
                edit.settings.drawing = edited.job.drawing;
                self.drawing_changed(&edited.new_ids);
                self.selection = CanvasSelection::Shapes(edited.new_ids.iter().copied().collect());
                let units = self.units.into();
                let mut text = t!(
                    self,
                    "modify-offset-done",
                    count = edited.new_ids.len(),
                    distance = chipload_pipeline::units::length_text(d, self.units)
                );
                for note in &edited.notes {
                    text.push(' ');
                    text.push_str(&note.render(self.language, units));
                }
                self.set_status(text, STATUS_SECONDS);
                true
            }
            Err(err) => {
                let text = err.message().render(self.language, self.units.into());
                self.set_status(text, STATUS_SECONDS);
                false
            }
        }
    }

    /// Enter on a typed offset distance: to the side the cursor at `cursor` is on.
    pub(super) fn apply_typed_offset(&mut self, cursor: Point) {
        let text = self
            .typed
            .as_ref()
            .map(|t| t.text.clone())
            .unwrap_or_default();
        let value = chipload_pipeline::units::parse_length(&text, self.units);
        let side = self.offset_for_cursor(cursor, true).map(|(_, side)| side);
        let applied = match (value, side) {
            (Some(d), Some(side)) => self.apply_offset(d, side),
            _ => {
                self.set_status(
                    crate::draw::EntryError::Unreadable.text(self),
                    STATUS_SECONDS,
                );
                false
            }
        };
        if applied {
            self.typed = None;
        } else if let Some(typed) = self.typed.as_mut() {
            typed.refused = true;
            typed.focus = true;
        }
    }

    /// The selected drawn shape's corner nearest the cursor at `at`, within reach on screen.
    pub fn corner_at(&self, at: Point) -> Option<(u64, usize, Point)> {
        let [id] = self.selected_drawn()[..] else {
            return None;
        };
        let shape = self.shape(id)?;
        let reach = f64::from(crate::view::PICK_TOLERANCE_PX) * 1.5 / self.view.scale;
        shape_corners(shape)
            .into_iter()
            .map(|(k, p)| (k, p, chipload_core::primitives::distance(p, at)))
            .filter(|&(_, _, d)| d <= reach)
            .min_by(|a, b| a.2.total_cmp(&b.2))
            .map(|(k, p, _)| (id, k, p))
    }

    /// The cut the fillet or chamfer tool makes.
    fn corner_cut(&self) -> Option<Cut> {
        match self.modify? {
            ModifyTool::Fillet => Some(Cut::Fillet(self.fillet_radius)),
            ModifyTool::Chamfer => Some(Cut::Chamfer(self.chamfer_distance)),
            _ => None,
        }
    }

    /// The selected shape as the corner tool would leave it, for the preview: the corner
    /// at the cursor, or with `all` every corner.
    pub(super) fn corner_preview(&self, at: Point, all: bool) -> Vec<Vec<Point>> {
        let (Some(cut), Some(edit)) = (self.corner_cut(), self.edit.as_ref()) else {
            return Vec::new();
        };
        let target = if all {
            match self.selected_drawn()[..] {
                [id] => Some((id, None)),
                _ => None,
            }
        } else {
            self.corner_at(at).map(|(id, k, _)| (id, Some(k)))
        };
        let Some((id, corner)) = target else {
            return Vec::new();
        };
        cut_shape_corners(&edit.settings, id, corner, cut)
            .ok()
            .and_then(|job| job.drawing.shapes.into_iter().find(|s| s.id == id))
            .map(|s| crate::draw::outline(&s.geometry))
            .unwrap_or_default()
    }

    /// Fillet or chamfer corner `corner` (None: every corner) of shape `id`, as one undo step.
    pub fn apply_corner_cut(&mut self, id: u64, corner: Option<usize>) -> bool {
        let Some(cut) = self.corner_cut() else {
            return false;
        };
        let Some(edit) = self.edit.as_mut() else {
            return false;
        };
        let mut job = edit.settings.clone();
        job.operations = edit.ops.iter().map(|e| e.op.clone()).collect();
        match cut_shape_corners(&job, id, corner, cut) {
            Ok(job) => {
                edit.settings.drawing = job.drawing;
                for (entry, op) in edit.ops.iter_mut().zip(job.operations) {
                    entry.op = op;
                }
                self.shape_draft = None;
                self.drawing_changed(&[id]);
                true
            }
            Err(err) => {
                let text = err.message().render(self.language, self.units.into());
                self.set_status(text, STATUS_SECONDS);
                false
            }
        }
    }

    /// A click while an editing tool is active: on a drawn shape that isn't selected it
    /// selects it (Ctrl: adds it); otherwise the tool works at the cursor. `all` (Shift):
    /// a corner tool does every corner.
    pub fn modify_click(
        &mut self,
        picked: Option<usize>,
        at: Point,
        toggle: bool,
        keys: (bool, bool),
    ) {
        let (free, all) = keys;
        match self.modify {
            Some(ModifyTool::Offset) => self.offset_click(picked, at, toggle, free),
            Some(ModifyTool::Fillet | ModifyTool::Chamfer) => {
                if self.picks_unselected_drawn(picked) {
                    self.click(picked, toggle);
                } else if all && let [id] = self.selected_drawn()[..] {
                    self.apply_corner_cut(id, None);
                } else if let Some((id, k, _)) = self.corner_at(at) {
                    self.apply_corner_cut(id, Some(k));
                }
            }
            Some(tool @ (ModifyTool::Trim | ModifyTool::Extend | ModifyTool::Break)) => {
                if let Some(id) = self.drawn_under(picked, at) {
                    self.apply_on_shape(tool, id, at);
                }
            }
            None => {}
        }
    }

    /// The drawn shape of contour `picked` nearest `at` (a contour may join several).
    pub fn drawn_under(&self, picked: Option<usize>, at: Point) -> Option<u64> {
        let contour = self.contours.iter().find(|c| Some(c.id) == picked)?;
        let ids: Vec<u64> = shapes_of(contour).into_iter().collect();
        nearest_drawn(&self.edit.as_ref()?.settings, &ids, at)
    }

    /// The project with the operations, as the editing operations take it (so points on a
    /// shape follow it).
    fn job_with_operations(&self) -> Option<chipload_job::Job> {
        let edit = self.edit.as_ref()?;
        let mut job = edit.settings.clone();
        job.operations = edit.ops.iter().map(|e| e.op.clone()).collect();
        Some(job)
    }

    /// Take an editing operation's result into the project, as one undo step: the drawing,
    /// the operations' points that followed, the changed and new shapes. Refused: the
    /// reason on the status bar.
    fn take_edit(&mut self, result: Result<Edited, PipelineError>, changed: &[u64]) -> bool {
        match result {
            Ok(edited) => {
                let Some(edit) = self.edit.as_mut() else {
                    return false;
                };
                edit.settings.drawing = edited.job.drawing;
                for (entry, op) in edit.ops.iter_mut().zip(edited.job.operations) {
                    entry.op = op;
                }
                let mut ids = changed.to_vec();
                ids.extend(&edited.new_ids);
                self.shape_draft = None;
                self.drawing_changed(&ids);
                let units = self.units.into();
                let notes: Vec<String> = edited
                    .notes
                    .iter()
                    .map(|n| n.render(self.language, units))
                    .collect();
                if !notes.is_empty() {
                    self.set_status(notes.join(" "), STATUS_SECONDS);
                }
                true
            }
            Err(err) => {
                let text = err.message().render(self.language, self.units.into());
                self.set_status(text, STATUS_SECONDS);
                false
            }
        }
    }

    /// Trim, extend or break drawn shape `id` at `at`, as one undo step.
    pub fn apply_on_shape(&mut self, tool: ModifyTool, id: u64, at: Point) -> bool {
        let Some(job) = self.job_with_operations() else {
            return false;
        };
        let result = match tool {
            ModifyTool::Trim => trim_shape(&job, id, at),
            ModifyTool::Break => break_shape(&job, id, at),
            ModifyTool::Extend => match nearer_end(&job, id, at) {
                Some(end) => extend_shape(&job, id, end),
                None => {
                    self.set_status(t!(self, "modify-extend-closed"), STATUS_SECONDS);
                    return false;
                }
            },
            _ => return false,
        };
        self.take_edit(result, &[id])
    }

    /// What trim, extend or break would do at `at` on the shape of contour `picked`: the
    /// trimmed piece, the extended shape, or the break point marked.
    pub(super) fn on_shape_preview(
        &self,
        picked: Option<usize>,
        at: Point,
    ) -> (Vec<Vec<Point>>, Option<Point>) {
        let (Some(tool), Some(id), Some(job)) = (
            self.modify,
            self.drawn_under(picked, at),
            self.edit.as_ref(),
        ) else {
            return (Vec::new(), None);
        };
        let job = &job.settings;
        match tool {
            ModifyTool::Trim => (
                trim_preview(job, id, at)
                    .map(|g| crate::draw::outline(&g))
                    .unwrap_or_default(),
                None,
            ),
            ModifyTool::Extend => {
                let lines = nearer_end(job, id, at)
                    .and_then(|end| extend_shape(job, id, end).ok())
                    .and_then(|edited| edited.job.drawing.shapes.into_iter().find(|s| s.id == id))
                    .map(|s| crate::draw::outline(&s.geometry))
                    .unwrap_or_default();
                (lines, None)
            }
            ModifyTool::Break => (Vec::new(), nearest_point(job, id, at)),
            _ => (Vec::new(), None),
        }
    }

    /// Whether contour `picked` is a drawn shape outside the selection.
    fn picks_unselected_drawn(&self, picked: Option<usize>) -> bool {
        let ids = self.selection.ids(&self.contours);
        picked.is_some_and(|p| !ids.contains(&p))
            && picked
                .and_then(|id| self.contours.iter().find(|c| c.id == id))
                .is_some_and(|c| {
                    let shapes = shapes_of(c);
                    !shapes.is_empty()
                        && shapes
                            .iter()
                            .all(|&s| self.shape(s).is_some_and(chipload_job::Shape::is_drawn))
                })
    }

    /// A click while the offset tool is active: on a drawn shape that isn't selected, it
    /// selects it (Ctrl: adds it); anywhere else, with shapes selected, it offsets them to
    /// the cursor.
    pub fn offset_click(&mut self, picked: Option<usize>, at: Point, toggle: bool, free: bool) {
        if self.picks_unselected_drawn(picked) {
            self.click(picked, toggle);
            return;
        }
        if let Some((d, side)) = self.offset_for_cursor(at, free) {
            self.apply_offset(d, side);
        }
    }

    /// The status bar while an editing tool is active.
    pub(super) fn modify_status(&self, at: Option<Point>, free: bool) -> Option<String> {
        let tool = self.modify?;
        Some(match tool {
            ModifyTool::Offset if self.selected_drawn().is_empty() => {
                t!(self, "modify-offset-pick")
            }
            ModifyTool::Offset => match at.and_then(|p| self.offset_for_cursor(p, free)) {
                Some((d, side)) => t!(
                    self,
                    "modify-offset-status",
                    distance = chipload_pipeline::units::length_text(d, self.units),
                    side = side_word(self, side)
                ),
                None => t!(self, "modify-offset-pick"),
            },
            ModifyTool::Fillet | ModifyTool::Chamfer if self.selected_drawn().len() != 1 => {
                t!(self, "modify-corner-pick")
            }
            ModifyTool::Fillet => t!(self, "modify-fillet-status"),
            ModifyTool::Chamfer => t!(self, "modify-chamfer-status"),
            ModifyTool::Trim => t!(self, "modify-trim-status"),
            ModifyTool::Extend => t!(self, "modify-extend-status"),
            ModifyTool::Break => t!(self, "modify-break-status"),
        })
    }

    /// While the offset tool is active: its corners and how many, on the status bar.
    pub(super) fn offset_fields(&mut self, ui: &mut egui::Ui) {
        match self.modify {
            Some(ModifyTool::Fillet) => {
                ui.label(t!(ui, "modify-fillet-radius"));
                ui.add(crate::units::length(&mut self.fillet_radius).range(0.001..=f64::MAX));
                ui.separator();
                return;
            }
            Some(ModifyTool::Chamfer) => {
                ui.label(t!(ui, "modify-chamfer-distance"));
                ui.add(crate::units::length(&mut self.chamfer_distance).range(0.001..=f64::MAX));
                ui.separator();
                return;
            }
            Some(ModifyTool::Offset) => {}
            _ => return,
        }
        egui::ComboBox::from_id_salt("offset-corners")
            .selected_text(match self.offset_corners {
                Corners::Round => t!(ui, "modify-corners-round"),
                Corners::Sharp => t!(ui, "modify-corners-sharp"),
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(
                    &mut self.offset_corners,
                    Corners::Round,
                    t!(ui, "modify-corners-round"),
                );
                ui.selectable_value(
                    &mut self.offset_corners,
                    Corners::Sharp,
                    t!(ui, "modify-corners-sharp"),
                );
            })
            .response
            .on_hover_text(t!(ui, "modify-corners-hover"));
        ui.label(t!(ui, "modify-offset-count"));
        ui.add(egui::DragValue::new(&mut self.offset_count).range(1..=100));
        ui.separator();
    }
}

fn side_word(lang: &impl crate::i18n::Lang, side: OffsetSide) -> String {
    match side {
        OffsetSide::Outside => t!(lang, "modify-side-outside"),
        OffsetSide::Inside => t!(lang, "modify-side-inside"),
        OffsetSide::Left => t!(lang, "modify-side-left"),
        OffsetSide::Right => t!(lang, "modify-side-right"),
    }
}
