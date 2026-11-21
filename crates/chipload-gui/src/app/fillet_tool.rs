// SPDX-License-Identifier: GPL-3.0-or-later
//! The Fillet and Chamfer tools on the shape under the cursor, with no selection first, and
//! between two shapes: a click on each, on the part to keep, with the result previewed
//! before the second (docs/specs/fillet-between-shapes.md §5, decision 7).

use super::modify::ModifyTool;
use super::*;
use chipload_pipeline::drawing_edit::{Between, Edited, fillet_between, shape_corners};

impl ChiploadApp {
    /// The corner the tool works on at `at`, within reach on screen: the selected drawn
    /// shape's first, else the nearest of any drawn shape shown.
    pub fn corner_at(&self, at: Point) -> Option<(u64, usize, Point)> {
        let job = &self.edit.as_ref()?.job;
        let reach = f64::from(crate::view::PICK_TOLERANCE_PX) * 1.5 / self.view.scale;
        let fillet = self.modify() == Some(ModifyTool::Fillet);
        let nearest = |shapes: &mut dyn Iterator<Item = &chipload_job::Shape>| {
            shapes
                .flat_map(|s| {
                    shape_corners(s, fillet)
                        .into_iter()
                        .map(move |(k, p)| (s.id, k, p))
                })
                .map(|(id, k, p)| (id, k, p, chipload_core::primitives::distance(p, at)))
                .filter(|c| c.3 <= reach)
                .min_by(|a, b| a.3.total_cmp(&b.3))
                .map(|(id, k, p, _)| (id, k, p))
        };
        if let [id] = self.selected_drawn()[..]
            && let Some(found) = nearest(&mut job.drawing.shapes.iter().filter(|s| s.id == id))
        {
            return Some(found);
        }
        nearest(
            &mut job
                .drawing
                .shapes
                .iter()
                .filter(|s| s.is_drawn() && !job.drawing.is_hidden(s)),
        )
    }

    /// The shape Shift+click does every corner of: the selected drawn shape, else the one
    /// whose corner is at the cursor.
    pub(super) fn all_corners_shape(&self, at: Point) -> Option<u64> {
        match self.selected_drawn()[..] {
            [id] => Some(id),
            _ => self.corner_at(at).map(|(id, _, _)| id),
        }
    }

    /// The open drawn polyline (a line or an arc too) of contour `picked` nearest `at`.
    fn open_drawn_under(&self, picked: Option<usize>, at: Point) -> Option<u64> {
        let id = self.drawn_under(picked, at)?;
        matches!(
            self.shape(id)?.geometry,
            Geometry::Polyline { closed: false, .. }
        )
        .then_some(id)
    }

    /// What joins two shapes with the tool in use.
    fn between_how(&self) -> Option<Between> {
        match self.modify()? {
            ModifyTool::Fillet => Some(Between::Fillet(self.canvas_tools.fillet_radius)),
            ModifyTool::Chamfer => Some(Between::Chamfer(self.canvas_tools.chamfer_distance)),
            _ => None,
        }
    }

    /// The first shape picked joined to shape `id`, kept at `at`; None without a first
    /// pick, or on the first shape itself.
    fn between_result(&self, id: u64, at: Point) -> Option<Result<Edited, PipelineError>> {
        let (first, from) = self.canvas_tools.between?;
        if first == id {
            return None;
        }
        let job = &self.edit.as_ref()?.job;
        Some(fillet_between(
            job,
            (first, from),
            (id, at),
            self.between_how()?,
        ))
    }

    /// The corner tool's preview at `at` and the point it marks: the round between the
    /// first shape picked and the one under the cursor (or the first marked alone, the
    /// reason kept for the status bar); else the corner under the cursor.
    pub(super) fn corner_tool_preview(
        &mut self,
        picked: Option<usize>,
        at: Point,
        all: bool,
    ) -> (Vec<Vec<Point>>, Option<Point>) {
        self.canvas_tools.between_why = None;
        if let Some((first, from)) = self.canvas_tools.between {
            let second = self.open_drawn_under(picked, at);
            let result = second.and_then(|id| self.between_result(id, at));
            return match result {
                Some(Ok(edited)) => {
                    let mut ids = vec![first];
                    ids.extend(second);
                    ids.extend(&edited.new_ids);
                    ids.extend(edited.joined);
                    let lines = edited
                        .job
                        .drawing
                        .shapes
                        .iter()
                        .filter(|s| ids.contains(&s.id))
                        .flat_map(|s| crate::draw::outline(&s.geometry))
                        .collect();
                    (lines, None)
                }
                Some(Err(err)) => {
                    let units = self.prefs.units.into();
                    self.canvas_tools.between_why =
                        Some(err.message().render(self.prefs.language, units));
                    (self.outline_of(first), Some(from))
                }
                None => (self.outline_of(first), Some(from)),
            };
        }
        let mark = self.corner_at(at).map(|(_, _, p)| p);
        (self.corner_preview(at, all), mark)
    }

    /// Shape `id`'s outline.
    fn outline_of(&self, id: u64) -> Vec<Vec<Point>> {
        self.shape(id)
            .map(|s| crate::draw::outline(&s.geometry))
            .unwrap_or_default()
    }

    /// A click with Fillet or Chamfer: Shift on a shape does every corner; by a corner it
    /// rounds or cuts it; on an open shape it picks the first of two, or with one picked
    /// joins them (one undo step); on empty space it forgets the first, and with nothing
    /// picked or selected ends the tool, as Trim does.
    pub(super) fn corner_click(&mut self, picked: Option<usize>, at: Point, all: bool) {
        if self.canvas_tools.between.is_none() {
            if all && let Some(id) = self.all_corners_shape(at) {
                self.apply_corner_cut(id, None);
                return;
            }
            if let Some((id, k, _)) = self.corner_at(at) {
                self.apply_corner_cut(id, Some(k));
                return;
            }
        }
        if let Some(id) = self.open_drawn_under(picked, at) {
            match self.canvas_tools.between {
                Some((first, _)) if first != id => {
                    let result = self.between_result(id, at);
                    if let Some(result) = result
                        && self.take_edit(result, &[first, id])
                    {
                        self.canvas_tools.between = None;
                        self.canvas_tools.between_why = None;
                    }
                }
                _ => self.canvas_tools.between = Some((id, at)),
            }
            return;
        }
        if picked.is_none() {
            if self.forget_between_pick() {
                return;
            }
            if self.selection == CanvasSelection::Nothing {
                self.cancel_modify();
            } else {
                self.click(None, false);
            }
        } else if self.drawn_under(picked, at).is_some() {
            // A closed shape: selected, so its corners come first.
            self.click(picked, false);
        }
    }

    /// Forget the first shape picked for a fillet between two; whether there was one.
    pub fn forget_between_pick(&mut self) -> bool {
        self.canvas_tools.between_why = None;
        self.canvas_tools.between.take().is_some()
    }
}
