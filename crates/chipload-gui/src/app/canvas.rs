// SPDX-License-Identifier: GPL-3.0-or-later
//! The drawing canvas: input, painting and hover texts.

use super::canvas_paint::DimensionLabel;
use super::*;
use chipload_core::math;
use chipload_core::order::Ordered;

impl ChiploadApp {
    /// `pos` lies on the active stock's outline, in the top view, within the pick
    /// tolerance.
    fn on_stock_edge(&self, pos: egui::Pos2) -> bool {
        let Some(edit) = self.edit.as_ref().filter(|_| self.view.is_top()) else {
            return false;
        };
        let ((x0, y0), (x1, y1)) = edit.job.stock_bounds();
        let board =
            egui::Rect::from_two_pos(self.view.to_screen((x0, y0)), self.view.to_screen((x1, y1)));
        let near = crate::view::PICK_TOLERANCE_PX;
        let outside = board.expand(near).contains(pos);
        let inside = board.shrink(near).contains(pos);
        outside && !inside
    }

    /// Switch to the 3D view, starting from a three-quarter view (docs/specs/3d-view.md).
    pub fn view_3d(&mut self) {
        if !self.display.three_d {
            self.display.three_d = true;
            // Every canvas tool works in top view only.
            self.end_tool();
            let pivot = self.orbit_pivot();
            self.view
                .orient(-30f64.to_radians(), 55f64.to_radians(), pivot);
        }
    }

    /// Back to looking straight down, as the 2D view always did.
    pub fn view_top(&mut self) {
        self.display.three_d = false;
        let pivot = self.orbit_pivot();
        self.view.orient(0.0, 0.0, pivot);
    }

    /// Mirrored parts where they are cut, faded (docs/specs/mirror-part.md §5); their
    /// instances are ghosts of their own.
    fn mirrored_outlines(&self) -> Vec<Vec<Point>> {
        match &self.edit {
            Some(edit) if edit.job.parts.iter().any(|p| p.mirror.is_some()) => {
                chipload_pipeline::parts::mirrored_outlines(&edit.job, &self.contours)
            }
            _ => Vec::new(),
        }
    }

    /// What the 3D view turns around: the stock's centre, else the drawing origin.
    /// Orbit by a drag in 3D; with View > Reverse left-right orbit, a left-right drag moves
    /// the view round the model instead, as before issue #16.
    pub(super) fn orbit_by(&mut self, mut delta: egui::Vec2) {
        if self.prefs.reverse_orbit {
            delta.x = -delta.x;
        }
        let pivot = self.orbit_pivot();
        self.view.orbit(delta, pivot);
    }

    pub(super) fn orbit_pivot(&self) -> Point {
        self.scene.stock.map_or((0.0, 0.0), |((x0, y0), (x1, y1))| {
            ((x0 + x1) / 2.0, (y0 + y1) / 2.0)
        })
    }

    /// The points a pending fit shows, None without a fit or with nothing to show yet.
    fn fit_pending_points(&self) -> Option<Vec<crate::view::Point3>> {
        if !self.fit_pending {
            return None;
        }
        let points = self
            .fit_only
            .clone()
            .unwrap_or_else(|| self.scene.fit_points(self.view.top_z));
        (!points.is_empty()).then_some(points)
    }

    /// Where a fit puts things: not under the rulers (top view only, as they are).
    fn fit_rect(&self, rect: egui::Rect) -> egui::Rect {
        if self.prefs.show_rulers && self.view.is_top() {
            let ruler = super::rulers::RULER;
            egui::Rect::from_min_max(rect.min + egui::vec2(ruler, ruler), rect.max)
        } else {
            rect
        }
    }

    /// The canvas follows a change of its size (docs/specs/canvas-resize.md): a view as a fit
    /// left it is fitted again; any other keeps showing the area it showed. What it is meant
    /// to show is taken again whenever the view changes (a pan, a zoom, a turn, a fit).
    fn keep_view(&mut self, rect: egui::Rect, fitted: Option<Vec<crate::view::Point3>>) {
        if let Some(points) = fitted {
            self.view_kept = Some(KeptView {
                rect,
                view: self.view,
                aim: Aim::Fit(points),
            });
            return;
        }
        let kept = match self.view_kept.take() {
            // Changed since the last frame (or the first): it now shows what it shows.
            Some(kept) if kept.view == self.view => kept,
            other => {
                let at = other.map_or(rect, |k| k.rect);
                KeptView {
                    rect: at,
                    view: self.view,
                    aim: Aim::Area(self.view.visible_area(at)),
                }
            }
        };
        if kept.rect != rect {
            match &kept.aim {
                Aim::Fit(points) => {
                    let free = self.fit_rect(rect);
                    self.view.fit(points, free);
                }
                Aim::Area(area) => self.view.show_area(*area, rect),
            }
        }
        self.view_kept = Some(KeptView {
            rect,
            view: self.view,
            aim: kept.aim,
        });
    }

    /// The line a two-sided board turns about, across its board, with its label; None for a
    /// board with one side or with *Show the turn axis* off (two-sided-wizard.md §2).
    fn turn_axis_line(&self, ui: &egui::Ui) -> Option<(Point, Point, String)> {
        if !self.display.turn_axis {
            return None;
        }
        let job = &self.edit.as_ref()?.job;
        let turn = super::simulation::board_turn(job)?;
        let ((x0, y0), (x1, y1)) = job.stock_bounds();
        let (a, b) = match turn {
            chipload_job::Turn::LeftRight => {
                let x = (x0 + x1) / 2.0;
                ((x, y0), (x, y1))
            }
            chipload_job::Turn::TopBottom => {
                let y = (y0 + y1) / 2.0;
                ((x0, y), (x1, y))
            }
        };
        Some((a, b, t!(ui, "canvas-turn-axis")))
    }

    pub(super) fn canvas(&mut self, ui: &mut egui::Ui) {
        self.prepare_scene(ui);
        let (response, painter) = ui.allocate_painter(ui.available_size(), Sense::click_and_drag());
        let rect = response.rect;
        // The selected shape's lengths, laid out before clicks are handled so a click on one
        // opens its entry (docs/specs/drawing-input.md §6).
        let labels = self.dimension_labels(ui, &painter);
        self.fit_view(rect);
        let keys = Keys::of(ui);
        self.update_corner_markers();
        // Simulate is a viewer: no handles to drag (docs/specs/edit-and-simulate.md §2).
        let editing = !self.display.simulate;
        // The Compass hides the selection's own handles while it shows (compass.md §3).
        self.compass_space(ui.ctx());
        let compass_on = editing && self.compass_now().is_some();
        let grips = self.grips(editing, compass_on);
        // The work zero's handle, over everything else (docs/specs/work-zero-marker.md §3).
        let zero_handle = self.zero_handle();
        self.scene.zero_handle = zero_handle.map(|p| (p, self.focus == Focus::WorkZero));
        self.scene.compass = if editing {
            self.compass_shape(response.hover_pos())
        } else {
            None
        };
        if editing {
            self.start_canvas_drag(ui, &response, keys, &grips);
        }
        if response.drag_stopped() {
            self.end_canvas_drag(ui, keys);
        }
        self.drag.snapped = None;
        self.scene.snap_mark = None;
        let zero_label = self.drag_canvas(ui, &response, keys);
        self.zoom_and_hover(ui, &response);
        let mut readout = zero_label;
        if readout.is_none()
            && let (Some(text), Some(at)) = (self.compass_readout(), self.cursor)
        {
            readout = Some((at, text));
        }
        if self.sketch().is_some() {
            readout = self.sketch_input(&response, keys);
        } else if self.modify().is_some() {
            if let Some(offset) = self.modify_input(&response, keys) {
                readout = Some(offset);
            }
        } else if self.placing_text() {
            self.text_input(&response, keys);
        } else if self.measure().is_some() {
            self.measure_input(&response, keys);
        } else {
            self.select_input(ui, &response, keys, &grips, &labels, editing);
        }
        if !compass_on {
            let outline = self.box_outline();
            self.scene.preview.extend(outline);
        }
        let moved = self.move_outlines();
        self.scene.preview.extend(moved);
        let compassed = self.compass_outlines();
        self.scene.preview.extend(compassed);
        self.compass_menu(ui.ctx());
        self.compass_pick_menu(ui.ctx());
        self.drop_library_entry(ui, &response, editing);
        if editing {
            self.canvas_context_menu(&response);
        }
        self.canvas_hover_text(ui, &response);
        self.paint_canvas(ui, &painter, rect, readout, &labels);
    }

    /// The scene's settings for this frame, and the heights for the 3D view.
    fn prepare_scene(&mut self, ui: &egui::Ui) {
        self.keep_drawing_in_view();
        self.scene.hide_toolpath = !self.display.toolpath;
        self.scene.hide_drawing = !self.show_drawing();
        self.scene.grid = self.prefs.show_grid;
        self.scene.inch_grid = self.prefs.units == chipload_job::Units::Inch;
        self.scene.palette = crate::view::Palette::of(ui.visuals().dark_mode);
        self.scene.work_zero = self.shown_zero_xy();
        self.scene.turn_axis = self.turn_axis_line(ui);
        // Each part's instances, as ghosts where they are cut (docs/specs/instances.md §4):
        // the contours its operations cut; the part itself is the drawing.
        self.scene.instances = self.instance_ghosts();
        // A nest's preview stands in for the instances while it is shown
        // (docs/specs/nesting.md §7).
        let (nest, board) = self.nest_scene();
        if !nest.is_empty() {
            self.scene.instances.clear();
        }
        self.scene.nest_preview = nest;
        self.scene.nest_board = board;
        self.scene.elsewhere = self.placed_elsewhere();
        self.scene.mirrored = self.mirrored_outlines();
        self.sync_rim_preview(ui.visuals().dark_mode);
        // Heights for the 3D view: 2D items sit on the stock top (docs/specs/3d-view.md). A
        // back is shown on its front's board, in its heights, and a turned board with its
        // other face up (docs/specs/two-sided.md §10.2).
        if let Some(edit) = &self.edit {
            let stock = &edit.job.stock();
            let (top, bottom) = chipload_pipeline::front_board_z(&edit.job)
                .unwrap_or((stock.top_z(), stock.top_z() - stock.thickness));
            let turn = self.shown_turn();
            self.view.top_z = if turn.is_some() { bottom } else { top };
            self.view.turned = turn.map(|turn| {
                let ((x0, y0), (x1, y1)) = edit.job.stock_bounds();
                let left_right = turn == chipload_job::Turn::LeftRight;
                crate::view::Turned {
                    left_right,
                    at: if left_right {
                        (x0 + x1) / 2.0
                    } else {
                        (y0 + y1) / 2.0
                    },
                    z_sum: top + bottom,
                }
            });
            self.scene.stock_z = Some((top, bottom));
        }
    }

    /// Fit the view when asked, and keep it on the drawing as the canvas changes size.
    fn fit_view(&mut self, rect: egui::Rect) {
        let fitted = self.fit_pending_points();
        if let Some(points) = &fitted {
            self.view.fit(points, self.fit_rect(rect));
            self.fit_pending = false;
            self.fit_only = None;
        }
        self.keep_view(rect, fitted);
    }

    /// The handles that can be grabbed this frame: the selected shape's, and a selected
    /// picture's or text's box (docs/specs/box-handles.md). Shown in the scene too.
    fn grips(&mut self, editing: bool, compass_on: bool) -> Grips {
        let shape_handles = if editing && self.show_drawing() && !compass_on {
            self.shape_handles()
        } else {
            Vec::new()
        };
        let (middles, others): (Vec<_>, Vec<_>) = shape_handles
            .iter()
            .partition(|(h, _)| matches!(h, Handle::Middle(_)));
        self.scene.handles = others.into_iter().map(|&(_, p)| p).collect();
        self.scene.middle_handles = middles.into_iter().map(|&(_, p)| p).collect();
        // A selected picture's or text's box handles (docs/specs/box-handles.md).
        let box_handles = if editing && !compass_on {
            self.box_handle_points()
        } else {
            Vec::new()
        };
        self.scene
            .handles
            .extend(box_handles.iter().map(|&(_, p)| p));
        Grips {
            shape: shape_handles,
            boxes: box_handles,
        }
    }

    /// A drag starting on the canvas: the Compass, the work zero, an instance, a handle, a
    /// marker, or a box selection or move.
    fn start_canvas_drag(
        &mut self,
        ui: &egui::Ui,
        response: &egui::Response,
        keys: Keys,
        grips: &Grips,
    ) {
        let Keys {
            alt, shift, space, ..
        } = keys;
        let view = self.view;
        let on_handle = |pos: egui::Pos2| grips.shape_at(&view, pos);
        let on_box_handle = |pos: egui::Pos2| grips.box_at(&view, pos);
        // A right drag on the Compass asks what to do when let go (compass.md §6).
        if response.drag_started_by(egui::PointerButton::Secondary)
            && let Some(origin) = ui.input(|i| i.pointer.press_origin())
            && let Some(part) = self.compass_part_at(origin)
        {
            self.start_compass_drag(part, origin, true);
        }
        if response.drag_started_by(egui::PointerButton::Primary)
            && !self.compass_picking()
            && let Some(origin) = ui.input(|i| i.pointer.press_origin())
        {
            let flat = !self.display.three_d;
            let at = self.view.to_world(origin);
            if self.on_zero_handle(origin) && self.start_zero_drag() {
                // Dragged below.
            } else if let Some(part) = self.compass_part_at(origin) {
                // The Compass is drawn over everything, an instance's ghost too.
                self.start_compass_drag(part, origin, false);
            } else if let Some(instance) = self.instance_handle_at(origin).filter(|_| flat) {
                self.start_instance_drag(instance, super::instances::InstanceGrip::Turn, at);
            } else if let Some(instance) = self.instance_badge_at(origin).filter(|_| flat) {
                // Its badge, above it, outside the shapes: always the instance.
                self.start_instance_drag(instance, super::instances::InstanceGrip::Move, at);
            } else if let Some(instance) = self.instance_at(origin).filter(|i| {
                // Over the shapes, only the selected one: instance 1 where it is drawn lies
                // on them (docs/specs/part-placement.md §6).
                flat && (self.scene.pick(&self.view, origin).is_none()
                    || self.focus == Focus::Instance(i.0, i.1))
            }) {
                self.start_instance_drag(instance, super::instances::InstanceGrip::Move, at);
            } else if let Some(handle) = on_handle(origin) {
                self.start_handle_drag(handle);
            } else if let Some(handle) = on_box_handle(origin) {
                self.start_box_drag(handle);
            } else if let Some((i, at)) = self.polyline_segment_at(origin) {
                // A selected polyline's line: that segment moves (the user, 2026-10-28).
                self.start_segment_drag(i, at);
            } else {
                self.drag.start = self.start_marker_at(origin);
                if self.drag.start.is_none() {
                    self.drag.tab = self.tab_marker_at(origin);
                }
                // Space + drag pans; otherwise a drag selects with a box or moves drawn
                // shapes (docs/specs/drawing-input.md §3).
                if self.drag.start.is_none()
                    && self.drag.tab.is_none()
                    && !space
                    && self.drag_selects()
                {
                    let picked = self.scene.pick(&self.view, origin);
                    let at = self.view.to_world(origin);
                    if !self.start_whole_drag(picked, at) {
                        self.start_select_drag(picked, at, shift, alt);
                    }
                }
            }
        }
    }

    /// A drag let go: whatever it carried ends, or is dropped on the shape library.
    fn end_canvas_drag(&mut self, ui: &egui::Ui, keys: Keys) {
        let Keys { shift, ctrl, .. } = keys;
        let at = ui.input(|i| i.pointer.latest_pos());
        self.end_compass_drag(at);
        // Selected shapes carried onto the shape library: saved there, not moved.
        let box_carried = self.drag.boxed.as_ref().is_some_and(|b| b.carried);
        if box_carried || self.drag.moving.as_ref().is_some_and(|m| m.carried) {
            self.drag.moving = None;
            self.drag.boxed = None;
            if let Some(pos) = ui.input(|i| i.pointer.latest_pos()) {
                self.drop_selection_on_library(pos);
            }
        }
        self.end_zero_drag();
        self.end_instance_drag();
        self.end_handle_drag();
        self.end_box_drag();
        self.end_move_drag();
        self.end_select_drag(shift, ctrl);
        self.drop_start();
        self.drop_tab();
    }

    /// A drag going on: what it moves follows the pointer, else the view pans or orbits.
    /// Returns the work zero's or an instance's readout while one is dragged.
    fn drag_canvas(
        &mut self,
        ui: &egui::Ui,
        response: &egui::Response,
        keys: Keys,
    ) -> Option<(Point, String)> {
        let Keys {
            alt,
            shift,
            ctrl,
            space,
        } = keys;
        let mut zero_label = None;
        // Picking a point for the Compass (compass.md §15): the preview follows the pointer.
        if self.compass_picking()
            && let Some(pos) = response.hover_pos()
        {
            self.hover_compass_pick(pos, alt);
        }
        if self.dragging_compass() {
            if response.dragged()
                && let Some(pos) = response.interact_pointer_pos()
            {
                self.drag_compass_to(self.view.to_world(pos), alt);
            }
        } else if self.dragging_zero() {
            if response.dragged()
                && let Some(pos) = response.interact_pointer_pos()
            {
                let at = self.view.to_world(pos);
                zero_label = self.drag_zero_to(at, alt).map(|text| (at, text));
            }
        } else if self.dragging_instance() {
            if response.dragged()
                && let Some(pos) = response.interact_pointer_pos()
            {
                let at = self.view.to_world(pos);
                zero_label = self.drag_instance_to(at, alt).map(|text| (at, text));
            }
        } else if let Some(marker) = self.drag.start {
            if response.dragged()
                && let Some(pos) = response.interact_pointer_pos()
            {
                self.drag_start_to(marker, self.view.to_world(pos));
            }
        } else if let Some(marker) = self.drag.tab {
            if response.dragged()
                && let Some(pos) = response.interact_pointer_pos()
            {
                self.drag_tab_to(marker, self.view.to_world(pos));
            }
        } else if self.drag.handle.is_some() {
            if response.dragged()
                && let Some(pos) = response.interact_pointer_pos()
            {
                let to = self.snap_world(self.view.to_world(pos), alt, true);
                // Ctrl: a new point; Shift: bend; plain: move the segment.
                let segment = if ctrl {
                    SegmentDrag::Insert
                } else if shift {
                    SegmentDrag::Bend
                } else {
                    SegmentDrag::Move
                };
                self.drag_handle_to(to, segment);
            }
        } else if self.dragging_box() {
            if response.dragged()
                && let Some(pos) = response.interact_pointer_pos()
            {
                // A picture or text grabbed by its lines can be carried onto the shape
                // library, as drawn shapes are.
                let carried = self.over_library(pos);
                self.carry_box(carried);
                if self.drag.boxed.as_ref().is_some_and(|b| b.carried) {
                    self.paint_carried(ui.ctx(), pos);
                } else {
                    let to = self.snap_world(self.view.to_world(pos), alt, true);
                    self.drag_box_to(to, alt);
                }
            }
        } else if self.drag.moving.is_some() {
            if response.dragged()
                && let Some(pos) = response.interact_pointer_pos()
            {
                let carried = self.over_library(pos);
                if let Some(moving) = self.drag.moving.as_mut() {
                    moving.carried = carried;
                    if carried {
                        moving.to = moving.grab;
                    }
                }
                if carried {
                    self.paint_carried(ui.ctx(), pos);
                } else {
                    self.drag_move_to(self.view.to_world(pos), alt, shift);
                }
            }
        } else if self.drag.select_box.is_some() {
            if response.dragged()
                && let Some(pos) = response.interact_pointer_pos()
            {
                self.drag_select_to(self.view.to_world(pos));
            }
        } else if response.dragged() {
            // In 3D, left-drag orbits; right-drag or Shift+drag pans. In top view a drag
            // that doesn't select or move pans: right, middle, Space + left, or any drag
            // while a tool is active.
            let pan = !self.display.three_d
                || response.dragged_by(egui::PointerButton::Secondary)
                || response.dragged_by(egui::PointerButton::Middle)
                || shift
                || space;
            if pan {
                self.view.pan(response.drag_delta());
            } else {
                self.orbit_by(response.drag_delta());
            }
        }
        zero_label
    }

    /// The wheel zooms at the pointer; the cursor follows it.
    fn zoom_and_hover(&mut self, ui: &egui::Ui, response: &egui::Response) {
        if let Some(pos) = response.hover_pos() {
            let (scroll, zoom) = ui.input(|i| (i.smooth_scroll_delta.y, i.zoom_delta()));
            let factor = f64::from(zoom) * math::exp(f64::from(scroll) * 0.0015);
            if (factor - 1.0).abs() > 1e-6 {
                self.view.zoom_at(pos, factor);
            }
            self.cursor = Some(self.view.to_world(pos));
            // An instance's badge and turn handle show they can be taken.
            if self.dragging_instance() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
            } else if self.instance_handle_at(pos).is_some()
                || self.instance_badge_at(pos).is_some()
            {
                ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
            }
            // Another setup's sheet lights up: a click makes it the active one
            // (docs/specs/parts-and-setups.md §7).
            self.scene.hot_sheet = self.sheet_under(pos);
            if self.scene.hot_sheet.is_some() && !self.dragging_instance() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
        } else {
            self.cursor = None;
            self.scene.hot_sheet = None;
        }
    }

    /// Drawing a shape: its preview at the cursor, clicks add points. Returns the readout.
    fn sketch_input(&mut self, response: &egui::Response, keys: Keys) -> Option<(Point, String)> {
        let Keys { alt, shift, .. } = keys;
        self.scene.measure = None;
        let at = self.cursor.map(|c| self.drawing_point(c, alt, shift));
        // A value being typed shows where it would land (also with the pointer off the
        // canvas: then a plain distance has no direction).
        let anchor = self.sketch().and_then(Sketch::anchor);
        let at = match at.or(anchor) {
            Some(c) if self.canvas_tools.typed.is_some() => Some(self.typed_target(c).unwrap_or(c)),
            _ => at,
        };
        if response.double_clicked() {
            self.finish_drawing();
        } else if response.clicked()
            && let Some(pos) = response.interact_pointer_pos()
        {
            let first = self.sketch().and_then(|s| s.points.first().copied());
            let on_first = first.is_some_and(|f| {
                self.view.to_screen(f).distance(pos) <= crate::view::PICK_TOLERANCE_PX
            });
            let p = self.drawing_point(self.view.to_world(pos), alt, shift);
            self.draw_click(p, on_first);
        }
        self.scene.preview = match (&self.sketch(), at) {
            (Some(sketch), Some(at)) => sketch.preview(at),
            _ => Vec::new(),
        };
        let readout = match (&self.sketch(), at) {
            (Some(sketch), Some(at)) => sketch.readout(at, self.prefs.units).map(|text| (at, text)),
            _ => None,
        };
        self.scene.sketch_points = self.sketch().map(|s| s.points.clone()).unwrap_or_default();
        readout
    }

    /// An editing tool (docs/specs/drawing-editing.md): the preview at the cursor; a click
    /// selects a shape, or applies the tool. Returns an offset's readout.
    fn modify_input(&mut self, response: &egui::Response, keys: Keys) -> Option<(Point, String)> {
        let mut readout = None;
        let Keys {
            alt, shift, ctrl, ..
        } = keys;
        self.scene.measure = None;
        self.scene.sketch_points.clear();
        let free = alt;
        if self.modify() == Some(super::modify::ModifyTool::Offset) {
            let target = self.cursor.and_then(|c| self.offset_for_cursor(c, free));
            self.scene.preview = match target {
                Some((d, side)) => self.offset_outlines(d, side),
                None => Vec::new(),
            };
            if let (Some((d, _)), Some(at)) = (target, self.cursor) {
                readout = Some((at, format!("d {}", crate::draw::fixed(d, self.prefs.units))));
            }
        } else if matches!(
            self.modify(),
            Some(
                super::modify::ModifyTool::Array
                    | super::modify::ModifyTool::PolarArray
                    | super::modify::ModifyTool::LinearArray
            )
        ) {
            // An array: its copies, and a polar array's centre marked.
            self.scene.preview = self.array_outlines();
            self.scene.snap_mark = (self.modify() == Some(super::modify::ModifyTool::PolarArray))
                .then_some(self.canvas_tools.array.center);
        } else if self.modify() == Some(super::modify::ModifyTool::Frame) {
            self.scene.preview = self.frame_outlines();
        } else if self
            .modify()
            .is_some_and(super::modify::ModifyTool::on_shape_under_cursor)
        {
            // Trim, extend, break: on the shape under the cursor.
            let hover = response.hover_pos();
            let picked = hover.and_then(|pos| self.scene.pick(&self.view, pos));
            let (lines, mark) = match (picked, self.cursor) {
                (Some(_), Some(at)) => self.on_shape_preview(picked, at),
                _ => (Vec::new(), None),
            };
            self.scene.preview = lines;
            self.scene.snap_mark = mark;
        } else {
            // A corner tool: the corner under the cursor marked, the result previewed.
            self.scene.snap_mark = self
                .cursor
                .and_then(|c| self.corner_at(c))
                .map(|(_, _, p)| p);
            self.scene.preview = self
                .cursor
                .map(|c| self.corner_preview(c, shift))
                .unwrap_or_default();
        }
        if response.clicked()
            && let Some(pos) = response.interact_pointer_pos()
        {
            let picked = self.scene.pick(&self.view, pos);
            self.modify_click(picked, self.view.to_world(pos), ctrl, (free, shift));
        }
        readout
    }

    /// Placing a text: a click puts it there.
    fn text_input(&mut self, response: &egui::Response, keys: Keys) {
        let Keys { alt, .. } = keys;
        self.scene.preview.clear();
        self.scene.sketch_points.clear();
        self.scene.measure = None;
        if response.clicked()
            && let Some(pos) = response.interact_pointer_pos()
        {
            let at = self.snap_world(self.view.to_world(pos), alt, false);
            self.place_text_at(at);
        }
    }

    /// Measuring: clicks set the points, the scene shows the measure.
    fn measure_input(&mut self, response: &egui::Response, keys: Keys) {
        let Keys { alt, .. } = keys;
        self.scene.preview.clear();
        self.scene.sketch_points.clear();
        let at = self.cursor.map(|c| self.snap_world(c, alt, false));
        if response.clicked()
            && let Some(pos) = response.interact_pointer_pos()
        {
            let p = self.snap_world(self.view.to_world(pos), alt, false);
            self.measure_click(p);
        }
        self.scene.measure = self.measure_scene(at);
    }

    /// No tool: a click picks a shape, a marker, a label, the work zero, the Compass or the
    /// stock.
    fn select_input(
        &mut self,
        ui: &egui::Ui,
        response: &egui::Response,
        keys: Keys,
        grips: &Grips,
        labels: &[DimensionLabel],
        editing: bool,
    ) {
        let Keys { alt, .. } = keys;
        let view = self.view;
        let on_handle = |pos: egui::Pos2| grips.shape_at(&view, pos);
        let on_box_handle = |pos: egui::Pos2| grips.box_at(&view, pos);
        // Built afresh each frame: the move and Compass previews are added after this.
        self.scene.preview.clear();
        self.scene.sketch_points.clear();
        self.scene.measure = None;
        // A corner marker opens its menu (docs/specs/dogbone-per-corner.md §6).
        let corner = (editing && response.clicked())
            .then(|| response.interact_pointer_pos())
            .flatten()
            .filter(|&pos| on_handle(pos).is_none() && on_box_handle(pos).is_none())
            .and_then(|pos| self.corner_marker_at(pos));
        let label = (editing && response.clicked())
            .then(|| response.interact_pointer_pos())
            .flatten()
            .and_then(|pos| labels.iter().find(|l| l.rect.contains(pos)));
        // An operation's number on the toolpath opens the operation.
        let op_label = response
            .clicked()
            .then(|| response.interact_pointer_pos())
            .flatten()
            .filter(|&pos| {
                on_handle(pos).is_none()
                    && on_box_handle(pos).is_none()
                    && self.start_marker_at(pos).is_none()
                    && self.tab_marker_at(pos).is_none()
            })
            .and_then(|pos| self.scene.op_label_at(&self.view, pos));
        let zero_clicked = (editing && response.clicked())
            .then(|| response.interact_pointer_pos())
            .flatten()
            .is_some_and(|pos| self.on_zero_handle(pos));
        // Picking a point for the Compass: the click is the point (compass.md §15).
        let picked = (editing && response.clicked() && self.compass_picking())
            .then(|| response.interact_pointer_pos())
            .flatten();
        if let Some(pos) = picked {
            self.click_compass_pick(pos, alt);
        }
        // A click on the Compass keeps the selection; a double-click types a value.
        let compass_hit = (editing && (response.clicked() || response.double_clicked()))
            .then(|| response.interact_pointer_pos())
            .flatten()
            .and_then(|pos| self.compass_part_at(pos).map(|part| (part, pos)));
        if picked.is_some() {
            // Taken above.
        } else if let Some((part, pos)) = compass_hit {
            if response.double_clicked() {
                self.start_typing_compass(part, pos);
            }
        } else if zero_clicked {
            self.focus = Focus::WorkZero;
        } else if let Some(label) = label {
            self.start_typing_dimension(label.dimension, label.value, label.rect.left_top());
        } else if let Some(k) = corner {
            self.open_corner_menu(k);
        } else if let Some(op) = op_label {
            self.focus = Focus::Op(op);
        } else if editing
            && response.clicked()
            && let Some(pos) = response.interact_pointer_pos()
            && on_handle(pos).is_none()
            && on_box_handle(pos).is_none()
            && self.start_marker_at(pos).is_none()
            && self.tab_marker_at(pos).is_none()
            && self.show_drawing()
        {
            let toggle = ui.input(|i| i.modifiers.command || i.modifiers.ctrl);
            let picked = self.scene.pick(&self.view, pos);
            // A double-click on a grouped shape, or a DXF's contour, picks it alone, to
            // work on it (docs/specs/shape-groups.md, decision 3; dxf-whole-on-canvas.md
            // decision 1).
            let inside = response
                .double_clicked()
                .then_some(picked)
                .flatten()
                .filter(|id| self.group_of_contour(*id).is_some() || self.dxf_contour(*id));
            let badge = self.instance_badge_at(pos);
            let instance = picked.is_none().then(|| self.instance_at(pos)).flatten();
            if let Some((part, id)) = badge {
                // An instance's badge is taken before any shape under it.
                self.focus = Focus::Instance(part, id);
            } else if let Some(contour) = inside {
                self.pick_alone(contour);
            } else if let Some((part, id)) = instance {
                self.focus = Focus::Instance(part, id);
            } else if picked.is_none()
                && !toggle
                && let Some(setup) = self.sheet_under(pos)
            {
                // Another setup's sheet: it becomes the active one, moved into place.
                self.activate_sheet(setup, pos);
            } else if picked.is_none() && !toggle && self.on_stock_edge(pos) {
                // The board's edge, where no shape is, opens its stock (the user,
                // 2026-11-03).
                self.focus = Focus::Stock;
            } else {
                self.click(picked, toggle);
            }
        }
    }

    /// An entry of the shape library dragged over the canvas: where it would land; let go,
    /// it is inserted there (docs/specs/shape-library.md §5.1).
    fn drop_library_entry(&mut self, ui: &egui::Ui, response: &egui::Response, editing: bool) {
        let rect = response.rect;
        self.shape_library.view_middle = Some(self.view.to_world(rect.center()));
        if editing
            && self.view.is_top()
            && let Some(pos) = ui
                .input(|i| i.pointer.hover_pos())
                .filter(|p| rect.contains(*p))
            && let Some(at) = self.entry_drag_preview(ui.ctx(), pos)
            && let Some(dropped) =
                response.dnd_release_payload::<super::shape_library::DraggedEntry>()
        {
            self.insert_entry(
                &dropped.0,
                Some(at),
                chipload_pipeline::shape_library::InsertSize::AsSaved,
            );
        }
    }
}

/// What the canvas view is meant to show, kept to show it again in a canvas of another
/// size (docs/specs/canvas-resize.md).
#[derive(Debug, Clone)]
pub(super) struct KeptView {
    /// The canvas as last drawn.
    rect: egui::Rect,
    /// The view as last drawn: another one means it was panned, zoomed or turned since.
    view: crate::view::View,
    aim: Aim,
}

#[derive(Debug, Clone)]
enum Aim {
    /// As a fit left it: these points, fitted again.
    Fit(Vec<crate::view::Point3>),
    /// This area, in camera units, all of it shown.
    Area(crate::view::Area),
}

/// The keys held while the canvas is used.
#[derive(Debug, Clone, Copy)]
struct Keys {
    alt: bool,
    shift: bool,
    ctrl: bool,
    space: bool,
}

impl Keys {
    fn of(ui: &egui::Ui) -> Keys {
        ui.input(|i| Keys {
            alt: i.modifiers.alt,
            shift: i.modifiers.shift,
            ctrl: i.modifiers.command || i.modifiers.ctrl,
            space: i.key_down(egui::Key::Space),
        })
    }
}

/// The handles the canvas can grab this frame.
struct Grips {
    /// The selected shape's.
    shape: Vec<(Handle, Point)>,
    /// A selected picture's or text's box.
    boxes: Vec<(chipload_pipeline::box_handles::BoxHandle, Point)>,
}

impl Grips {
    fn shape_at(&self, view: &crate::view::View, pos: egui::Pos2) -> Option<Handle> {
        nearest(view, self.shape.iter().map(|&(_, p)| p), pos).map(|i| self.shape[i].0)
    }

    fn box_at(
        &self,
        view: &crate::view::View,
        pos: egui::Pos2,
    ) -> Option<chipload_pipeline::box_handles::BoxHandle> {
        nearest(view, self.boxes.iter().map(|&(_, p)| p), pos).map(|i| self.boxes[i].0)
    }
}

/// The index of the point nearest `pos` on screen, within the pick tolerance.
fn nearest(
    view: &crate::view::View,
    points: impl Iterator<Item = Point>,
    pos: egui::Pos2,
) -> Option<usize> {
    points
        .enumerate()
        .map(|(i, p)| (i, view.to_screen(p).distance(pos)))
        .filter(|&(_, d)| d <= crate::view::PICK_TOLERANCE_PX)
        .min_by_key(|&(_, d)| Ordered(f64::from(d)))
        .map(|(i, _)| i)
}
