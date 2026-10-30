// SPDX-License-Identifier: GPL-3.0-or-later
//! The work zero in the view (docs/specs/work-zero-marker.md §3): a handle at the foot of
//! the axes, picked with a click and, once selected, dragged. A drop near a stock corner,
//! its centre or the drawing origin takes that (`Job::work_zero_at`); elsewhere a point.
//! The zero is set when the drag ends, as one undo step.

use chipload_core::Point;
use chipload_job::WorkZero;

use super::*;

/// A drag of the work zero in progress.
#[derive(Debug, Clone, Copy)]
pub(super) struct ZeroDrag {
    /// The zero when the drag began.
    start: WorkZero,
    /// Where the drag has it now.
    now: WorkZero,
    /// Escape was pressed: the rest of the drag does nothing.
    cancelled: bool,
}

impl ChiploadApp {
    /// The zero as shown: the dragged one while a drag is on.
    fn shown_zero(&self) -> Option<WorkZero> {
        let edit = self.edit.as_ref()?;
        Some(match self.drag.zero {
            Some(drag) if !drag.cancelled => drag.now,
            _ => edit.settings.work_zero(),
        })
    }

    /// Where the axes are drawn: the zero, or where a drag has it.
    pub(super) fn shown_zero_xy(&self) -> Point {
        match (self.shown_zero(), self.edit.as_ref()) {
            (Some(zero), Some(edit)) => edit.work_zero_point(zero),
            _ => (0.0, 0.0),
        }
    }

    /// Where the zero's handle is: in top view, while nothing is drawn, edited, measured or
    /// placed.
    pub(super) fn zero_handle(&self) -> Option<Point> {
        if self.three_d
            || self.show_simulation
            || self.sketch.is_some()
            || self.modify.is_some()
            || self.measure.is_some()
            || self.placing_text()
        {
            return None;
        }
        let zero = self.shown_zero()?;
        Some(self.edit.as_ref()?.work_zero_point(zero))
    }

    /// The pointer at `pos` is on the zero's handle.
    pub(super) fn on_zero_handle(&self, pos: egui::Pos2) -> bool {
        self.zero_handle()
            .is_some_and(|p| self.view.to_screen(p).distance(pos) <= crate::view::PICK_TOLERANCE_PX)
    }

    /// A drag begins on the handle: only when the zero is selected, so a drag meant for a
    /// shape under it never moves it (decision 2).
    pub(super) fn start_zero_drag(&mut self) -> bool {
        if self.focus != Focus::WorkZero {
            return false;
        }
        let Some(zero) = self.edit.as_ref().map(|e| e.settings.work_zero()) else {
            return false;
        };
        self.drag.zero = Some(ZeroDrag {
            start: zero,
            now: zero,
            cancelled: false,
        });
        true
    }

    pub(super) fn dragging_zero(&self) -> bool {
        self.drag.zero.is_some()
    }

    /// The dragged zero with the pointer at `at`: a stock corner, its centre or the drawing
    /// origin within reach, else the point snapped as the cursor snaps. `free` (Alt): the
    /// point as it is. Returns the label for the cursor.
    pub(super) fn drag_zero_to(&mut self, at: Point, free: bool) -> Option<String> {
        let drag = self.drag.zero.filter(|d| !d.cancelled)?;
        let reach = f64::from(crate::view::PICK_TOLERANCE_PX) / self.view.scale;
        let named = match (free, self.edit.as_ref()) {
            (false, Some(edit)) => {
                Some(edit.work_zero_at(at, reach)).filter(|z| !matches!(z, WorkZero::Point { .. }))
            }
            _ => None,
        };
        let now = match named {
            Some(zero) => {
                self.scene.snap_mark = self.edit.as_ref().map(|e| e.work_zero_point(zero));
                zero
            }
            None => {
                let (x, y) = if free {
                    at
                } else {
                    self.snap_world(at, false, false)
                };
                WorkZero::Point { x, y }
            }
        };
        self.drag.zero = Some(ZeroDrag { now, ..drag });
        Some(match now {
            WorkZero::Point { x, y } => format!(
                "X {}  Y {}",
                crate::draw::fixed(x, self.units),
                crate::draw::fixed(y, self.units)
            ),
            zero => super::forms::work_zero_name(self, &zero),
        })
    }

    /// The drag ends: the zero is set where it was dropped, as one undo step.
    pub(super) fn end_zero_drag(&mut self) {
        let Some(drag) = self.drag.zero.take() else {
            return;
        };
        self.scene.snap_mark = None;
        if drag.cancelled || drag.now == drag.start {
            return;
        }
        if let Some(edit) = self.edit.as_mut() {
            *edit.settings.work_zero_mut() = drag.now;
            self.commit();
        }
    }

    /// Escape during a drag: the zero stays where it was. False when there was no drag.
    pub(super) fn cancel_zero_drag(&mut self) -> bool {
        match self.drag.zero.as_mut() {
            Some(drag) if !drag.cancelled => {
                drag.cancelled = true;
                self.scene.snap_mark = None;
                true
            }
            _ => false,
        }
    }
}
