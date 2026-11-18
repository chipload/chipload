// SPDX-License-Identifier: GPL-3.0-or-later
//! Handles on the canvas: a shape's points dragged, snapping, start markers.

use super::*;
use chipload_core::order::Ordered;

/// How far beyond a rectangle's end its rotation handle sits, in screen pixels.
const ROTATE_HANDLE_PX: f64 = 24.0;

impl ChiploadApp {
    /// Handles of the selected shape: one drawn shape, in top view, not while drawing.
    pub(super) fn shape_handles(&self) -> Vec<(Handle, Point)> {
        if self.display.three_d || self.sketch().is_some() || !self.can_draw() {
            return Vec::new();
        }
        let Some(shape) = self.selected_shape().and_then(|id| self.shape(id)) else {
            return Vec::new();
        };
        // The rotation handle sits a fixed distance on screen beyond the rectangle.
        let reach = ROTATE_HANDLE_PX / self.view.scale;
        handles(&shape.geometry, reach)
            .into_iter()
            .map(|(h, p)| (h, (p[0], p[1])))
            .collect()
    }

    /// A drawn polyline is selected and its handles show: the status bar says how to edit it.
    pub(super) fn polyline_selected(&self) -> bool {
        !self.shape_handles().is_empty()
            && self
                .selected_shape()
                .and_then(|id| self.shape(id))
                .is_some_and(|s| matches!(s.geometry, chipload_job::Geometry::Polyline { .. }))
    }

    /// Where the cursor at `at` lands (snap point, else grid; exact with Alt), remembered
    /// for the marker and the status bar. `dragging`: skip the selected shape's own points.
    pub(super) fn snap_world(&mut self, at: Point, free: bool, dragging: bool) -> Point {
        // A dragged box doesn't snap to its own letters or traced shapes.
        let own_box = (dragging && self.dragging_box()).then(|| self.selected_ids());
        let moving: Option<BTreeSet<u64>> = self
            .drag
            .moving
            .as_ref()
            .filter(|_| dragging)
            .map(|m| m.ids.iter().copied().collect());
        let skip: BTreeSet<usize> = match self.selected_shape().filter(|_| dragging) {
            _ if moving.is_some() => self
                .contours
                .iter()
                .filter(|c| {
                    moving
                        .as_ref()
                        .is_some_and(|ids| !shapes_of(c).is_disjoint(ids))
                })
                .map(|c| c.id)
                .collect(),
            Some(id) => self
                .contours
                .iter()
                .filter(|c| shapes_of(c).contains(&id))
                .map(|c| c.id)
                .collect(),
            None => own_box
                .map(|ids| ids.into_iter().collect())
                .unwrap_or_default(),
        };
        self.snap_skipping(at, free, &skip).0
    }

    /// Where the cursor at `at` lands, leaving out the contours `skip` (snap point, else
    /// grid; exact with `free`), remembered for the marker and the status bar; and whether
    /// it took a snap point.
    pub(super) fn snap_skipping(
        &mut self,
        at: Point,
        free: bool,
        skip: &BTreeSet<usize>,
    ) -> (Point, bool) {
        let step = grid_step(self.view.scale, self.prefs.units);
        let cursor = Cursor {
            at,
            from: self.last_point(),
            skip,
        };
        let (p, hit) = snap_cursor(
            &cursor,
            &self.drag.snapper,
            self.view.scale,
            step,
            self.prefs.units,
            free,
        );
        self.drag.snapped = Some((p, hit));
        self.scene.snap_mark = hit.map(|h| h.at);
        (p, hit.is_some())
    }

    /// Where a drawing click or the cursor lands: snapped (else the grid), or with Shift
    /// held to 15° steps from the last click (docs/specs/drawing-input.md §2).
    pub(super) fn drawing_point(&mut self, raw: Point, alt: bool, shift: bool) -> Point {
        let anchor = self.sketch().and_then(Sketch::anchor);
        match anchor.filter(|_| shift) {
            Some(anchor) => {
                let step = (!alt).then(|| grid_step(self.view.scale, self.prefs.units));
                let p = crate::draw::angle_step(anchor, raw, step, self.prefs.units);
                self.drag.snapped = Some((p, None));
                self.scene.snap_mark = None;
                p
            }
            None => self.snap_world(raw, alt, false),
        }
    }

    /// The last point of the tool being used, for perpendicular and tangent snaps: a
    /// polyline's last click, or a measurement's first point.
    pub(super) fn last_point(&self) -> Option<Point> {
        if let Some(sketch) = &self.sketch() {
            return sketch.last_point();
        }
        if self.modify() == Some(super::modify::ModifyTool::Mirror) {
            return self.canvas_tools.mirror.from;
        }
        self.measure()
            .and_then(|m| m.start.filter(|_| m.end.is_none()))
    }

    pub fn start_handle_drag(&mut self, handle: Handle) {
        self.start_grab(handle, (0.0, 0.0));
    }

    /// Drag the selected polyline's segment `i`, grabbed at `at` (drawing coordinates)
    /// anywhere along it.
    pub fn start_segment_drag(&mut self, i: usize, at: Point) {
        let Some(shape) = self.selected_shape().and_then(|id| self.shape(id)) else {
            return;
        };
        let Some(&(_, middle)) = handles(&shape.geometry, 0.0)
            .iter()
            .find(|(h, _)| *h == Handle::Middle(i))
        else {
            return;
        };
        self.start_grab(Handle::Middle(i), (middle[0] - at.0, middle[1] - at.1));
    }

    fn start_grab(&mut self, handle: Handle, grab: Point) {
        let Some(shape) = self.selected_shape().and_then(|id| self.shape(id)) else {
            return;
        };
        self.drag.handle = Some(HandleDrag {
            handle,
            layer: shape.layer.clone(),
            start: shape.geometry.clone(),
            grab,
        });
    }

    /// The segment of the selected polyline (with its handles showing) under screen point
    /// `pos`, and where on it in drawing coordinates.
    pub(super) fn polyline_segment_at(&self, pos: egui::Pos2) -> Option<(usize, Point)> {
        if !self.polyline_selected() {
            return None;
        }
        let shape = self.selected_shape().and_then(|id| self.shape(id))?;
        let chipload_job::Geometry::Polyline {
            closed,
            points,
            bulges,
        } = &shape.geometry
        else {
            return None;
        };
        let n = points.len();
        let at = self.view.to_world(pos);
        (0..chipload_job::polyline_segment_count(n, *closed))
            .filter_map(|i| {
                let (a, b) = (points[i], points[(i + 1) % n]);
                let (a, b) = ((a[0], a[1]), (b[0], b[1]));
                let bulge = bulges.get(i).copied().unwrap_or(0.0);
                let segment = match chipload_core::primitives::Arc::from_bulge(a, b, bulge) {
                    Some(arc) => chipload_core::primitives::Primitive::Arc(arc),
                    None => chipload_core::primitives::Primitive::Line(
                        chipload_core::primitives::Line { start: a, end: b },
                    ),
                };
                let near = chipload_draw::intersect::nearest_on(&segment, at);
                let d = self.view.to_screen(near).distance(pos);
                (d <= crate::view::PICK_TOLERANCE_PX).then_some((d, i))
            })
            .min_by_key(|&(d, i)| (Ordered(f64::from(d)), i))
            .map(|(_, i)| (i, at))
    }

    /// Move the dragged handle with the pointer at `to`. For a polyline segment, `segment`
    /// says whether it moves, gets a new point or bends. A position that would make the
    /// shape invalid leaves it as it was last valid; the form says why.
    pub fn drag_handle_to(&mut self, to: Point, segment: SegmentDrag) {
        let Some(drag) = self.drag.handle.clone() else {
            return;
        };
        // A new point or a bend goes through the pointer; a moved segment keeps where it
        // was grabbed under the pointer.
        let to = match segment {
            SegmentDrag::Move => (to.0 + drag.grab.0, to.1 + drag.grab.1),
            SegmentDrag::Insert | SegmentDrag::Bend => to,
        };
        let refused = match try_drag(&drag.start, drag.handle, [to.0, to.1], segment) {
            Ok(geometry) => self.set_selected_shape(&drag.layer, geometry).err(),
            Err(why) => Some(why),
        };
        let shape = self.selected_shape().and_then(|id| self.shape(id)).cloned();
        if let Some(shape) = shape {
            let mut draft = ShapeDraft::of(&shape);
            draft.error = refused;
            self.shape_draft = Some(draft);
        }
    }

    pub fn end_handle_drag(&mut self) {
        if self.drag.handle.take().is_some()
            && let Some(draft) = self.shape_draft.as_mut()
        {
            draft.error = None;
        }
        self.scene.snap_mark = None;
    }

    // -- start points (docs/specs/helix-and-leads.md part A)

    /// The focused operation's start marker at `pos`, in top view.
    pub(super) fn start_marker_at(&self, pos: egui::Pos2) -> Option<usize> {
        let Focus::Op(operation) = self.focus else {
            return None;
        };
        if self.display.three_d || self.sketch().is_some() || self.scene.hide_toolpath {
            return None;
        }
        self.scene
            .start_markers
            .iter()
            .enumerate()
            .filter(|(_, m)| m.operation == operation)
            .map(|(i, m)| (i, self.view.to_screen(m.at).distance(pos)))
            .filter(|&(_, d)| d <= crate::view::PICK_TOLERANCE_PX + 2.0)
            .min_by_key(|&(_, d)| Ordered(f64::from(d)))
            .map(|(i, _)| i)
    }

    /// Move a start marker along its ring, towards `to` (drawing coordinates).
    pub fn drag_start_to(&mut self, marker: usize, to: Point) {
        self.drag.start = Some(marker);
        if let Some(m) = self.scene.start_markers.get_mut(marker) {
            m.move_to(to);
        }
    }

    /// Store where the dragged marker was left as the ring's start point, and generate
    /// again so the toolpath starts there.
    pub fn drop_start(&mut self) {
        let Some(marker) = self.drag.start.take() else {
            return;
        };
        let Some(m) = self.scene.start_markers.get(marker).cloned() else {
            return;
        };
        let Some(op) = self
            .edit
            .as_mut()
            .and_then(|e| e.job.operations.get_mut(m.operation))
        else {
            return;
        };
        let point = [m.at.0, m.at.1];
        let Some(starts) = op.starts_mut() else {
            return;
        };
        match m.stored.filter(|&k| k < starts.len()) {
            Some(k) => starts[k] = point,
            None => starts.push(point),
        }
        self.commit();
        self.toolpath.undo_if_refused.clone_from(&self.edit);
        self.generate();
    }

    /// Forget the point that set this marker's ring start, and generate again.
    pub fn reset_start(&mut self, marker: usize) {
        let Some(m) = self.scene.start_markers.get(marker).cloned() else {
            return;
        };
        let (Some(k), Some(op)) = (
            m.stored,
            self.edit
                .as_mut()
                .and_then(|e| e.job.operations.get_mut(m.operation)),
        ) else {
            return;
        };
        let Some(starts) = op.starts_mut() else {
            return;
        };
        if k < starts.len() {
            starts.remove(k);
        }
        self.commit();
        self.generate();
    }
}
