// SPDX-License-Identifier: GPL-3.0-or-later
//! Drawing and editing shapes (docs/specs/drawing-tools.md).

use super::*;

impl ChiploadApp {
    // -- drawing (docs/specs/drawing-tools.md)

    /// Whether shapes can be drawn: a project is open.
    pub fn can_draw(&self) -> bool {
        self.edit.is_some()
    }

    pub fn start_drawing(&mut self, tool: Tool) {
        if self.can_draw() {
            if self.three_d {
                self.view_top(); // drawing works in top view only
            }
            self.measure = None;
            self.placing_text = false;
            self.sketch = Some(Sketch::new(tool));
        }
    }

    pub fn cancel_drawing(&mut self) {
        self.sketch = None;
    }

    /// A click while drawing, at `p` (already on the grid). `on_first`: the click hit the
    /// polyline's first point on screen.
    pub fn draw_click(&mut self, p: Point, on_first: bool) {
        let Some(sketch) = self.sketch.as_mut() else {
            return;
        };
        let step = sketch.click(p, on_first);
        self.after_sketch_step(step);
    }

    /// Enter or double-click: finish a polyline open.
    pub fn finish_drawing(&mut self) {
        let Some(sketch) = self.sketch.as_mut() else {
            return;
        };
        let step = sketch.finish();
        self.after_sketch_step(step);
    }

    pub(super) fn after_sketch_step(&mut self, step: Step) {
        match step {
            Step::Continue => {}
            Step::Ignored(why) => {
                if let Some(why) = why {
                    self.set_status(why.text(self), STATUS_SECONDS);
                }
            }
            Step::Done(geometry) => {
                self.sketch = None;
                self.add_drawn(geometry);
            }
        }
    }

    /// Add a drawn shape to the project and select it.
    pub fn add_drawn(&mut self, geometry: Geometry) -> Option<u64> {
        let layer = self.default_layer();
        let edit = self.edit.as_mut()?;
        match add_shape(&edit.settings, &layer, geometry) {
            Ok((job, id)) => {
                edit.settings = job;
                self.drawing_changed(&[id]);
                let kind = match self.shape(id).map(|s| &s.geometry) {
                    Some(Geometry::Circle { .. }) => t!(self, "drawing-kind-circle"),
                    Some(Geometry::Rectangle { .. }) => t!(self, "drawing-kind-rectangle"),
                    Some(Geometry::Polyline { .. }) => t!(self, "drawing-kind-polyline"),
                    Some(Geometry::Path { .. }) | None => t!(self, "drawing-shape"),
                };
                self.set_status(
                    t!(self, "drawing-added", kind = kind, id = id, layer = layer),
                    STATUS_SECONDS,
                );
                Some(id)
            }
            Err(err) => {
                self.error_report(Report::failed(
                    t!(self, "drawing-add-failed"),
                    err.message(),
                ));
                None
            }
        }
    }

    /// A new shape's layer: the selection's, if it is all on one layer; else 0.
    pub(super) fn default_layer(&self) -> String {
        if let CanvasSelection::Layer(layer, _) = &self.selection {
            return layer.clone();
        }
        let ids = self.selection.ids(&self.contours);
        let layers: BTreeSet<String> = self
            .contours
            .iter()
            .filter(|c| ids.contains(&c.id))
            .flat_map(Contour::layers)
            .collect();
        match layers.len() {
            1 => layers.into_iter().next().unwrap_or_else(|| "0".into()),
            _ => "0".into(),
        }
    }

    pub fn shape(&self, id: u64) -> Option<&chipload_job::Shape> {
        self.edit
            .as_ref()?
            .settings
            .drawing
            .shapes
            .iter()
            .find(|s| s.id == id)
    }

    /// The one shape the canvas selection consists of, if it is exactly one.
    pub fn selected_shape(&self) -> Option<u64> {
        if !matches!(self.selection, CanvasSelection::Shapes(_)) {
            return None;
        }
        let ids = self.selection.ids(&self.contours);
        let mut contours = self.contours.iter().filter(|c| ids.contains(&c.id));
        let (Some(only), None) = (contours.next(), contours.next()) else {
            return None;
        };
        let shapes = shapes_of(only);
        match shapes.len() {
            1 => shapes.into_iter().next(),
            _ => None,
        }
    }

    /// Apply new values to the selected drawn shape. On failure the project is unchanged
    /// and the reason is returned (shown under the form).
    pub fn set_selected_shape(&mut self, layer: &str, geometry: Geometry) -> Result<(), String> {
        let lang = crate::i18n::Lang::lang(self);
        let units = self.units.into();
        let id = self
            .selected_shape()
            .ok_or_else(|| t!(lang, "drawing-no-shape-selected"))?;
        let edit = self
            .edit
            .as_mut()
            .ok_or_else(|| t!(lang, "drawing-no-project"))?;
        let job = set_shape(&edit.settings, id, layer, geometry)
            .map_err(|e| e.message().render(lang, units))?;
        edit.settings = job;
        self.drawing_changed(&[id]);
        Ok(())
    }

    /// The drawn shapes in the selection (the ones Delete removes), and whether imported
    /// ones are selected too.
    pub fn deletable_shapes(&self) -> (Vec<u64>, bool) {
        let CanvasSelection::Shapes(ids) = &self.selection else {
            return (Vec::new(), false);
        };
        let Some(edit) = self.edit.as_ref() else {
            return (Vec::new(), false);
        };
        let mut drawn = Vec::new();
        let mut imported = false;
        for shape in edit
            .settings
            .drawing
            .shapes
            .iter()
            .filter(|s| ids.contains(&s.id))
        {
            if shape.is_drawn() {
                drawn.push(shape.id);
            } else {
                imported = true;
            }
        }
        (drawn, imported)
    }

    /// Delete the selected drawn shape.
    pub fn delete_selected_shape(&mut self) {
        self.delete_selected_shapes();
    }

    /// Delete every drawn shape in the selection (the Delete key). Operations that used one
    /// are flagged. Imported shapes stay: they change only with the DXF.
    pub fn delete_selected_shapes(&mut self) {
        let (ids, imported) = self.deletable_shapes();
        if ids.is_empty() {
            if imported {
                self.error(vec![t!(self, "drawing-imported-not-deletable")]);
            }
            return;
        }
        let Some(edit) = self.edit.as_mut() else {
            return;
        };
        let mut job: Job = edit.settings.clone();
        job.operations = edit.ops.iter().map(|e| e.op.clone()).collect();
        let mut flagged = Vec::new();
        for &id in &ids {
            match delete_shape(&job, id) {
                Ok((out, lines)) => {
                    job = out;
                    flagged.extend(lines);
                }
                Err(err) => {
                    self.error_report(Report::of_error(err.message()));
                    return;
                }
            }
        }
        edit.settings.drawing = job.drawing;
        self.error_ops = edit
            .ops
            .iter()
            .enumerate()
            .filter(|(_, e)| {
                e.op.select()
                    .shapes
                    .as_ref()
                    .is_some_and(|used| used.iter().any(|u| ids.contains(u)))
            })
            .map(|(i, _)| i)
            .collect();
        self.selection = CanvasSelection::Nothing;
        self.shape_draft = None;
        self.drawing_changed(&[]);
        let names: Vec<String> = ids.iter().map(|id| format!("S{id}")).collect();
        let names = names.join(", ");
        let mut lines = vec![t!(
            self,
            "drawing-deleted-report",
            count = ids.len(),
            shapes = names.as_str()
        )];
        if imported {
            lines.push(t!(self, "drawing-imported-kept"));
        }
        lines.extend(flagged);
        self.set_report(Report::info(lines));
        self.set_status(
            t!(
                self,
                "drawing-deleted-status",
                count = ids.len(),
                shapes = names
            ),
            STATUS_SECONDS,
        );
    }

    /// The drawing changed: recompute contours now, so the canvas follows every edit, and
    /// select `shapes` if given. (The selection is kept as shape ids, so contour numbers
    /// changing doesn't disturb it.)
    pub(super) fn drawing_changed(&mut self, shapes: &[u64]) {
        self.refresh_drawing();
        if !shapes.is_empty() {
            self.selection = CanvasSelection::Shapes(shapes.iter().copied().collect());
        }
    }
}
