// SPDX-License-Identifier: GPL-3.0-or-later
//! Editing tools on drawn shapes (docs/specs/drawing-editing.md): the tool being used, its
//! preview and its clicks. The geometry is `chipload_pipeline::drawing_edit`.

use chipload_pipeline::drawing_edit::{Corners, OffsetSide, offset_at, offset_shapes};

use super::*;

/// An editing tool.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModifyTool {
    /// Offset the selected shapes to the cursor's side (§2).
    Offset,
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

    /// A click while the offset tool is active: on a drawn shape that isn't selected, it
    /// selects it (Ctrl: adds it); anywhere else, with shapes selected, it offsets them to
    /// the cursor.
    pub fn offset_click(&mut self, picked: Option<usize>, at: Point, toggle: bool, free: bool) {
        let ids = self.selection.ids(&self.contours);
        let drawn = picked
            .and_then(|id| self.contours.iter().find(|c| c.id == id))
            .is_some_and(|c| {
                let shapes = shapes_of(c);
                !shapes.is_empty()
                    && shapes
                        .iter()
                        .all(|&s| self.shape(s).is_some_and(chipload_job::Shape::is_drawn))
            });
        if drawn && picked.is_some_and(|p| !ids.contains(&p)) {
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
        })
    }

    /// While the offset tool is active: its corners and how many, on the status bar.
    pub(super) fn offset_fields(&mut self, ui: &mut egui::Ui) {
        if self.modify != Some(ModifyTool::Offset) {
            return;
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
