// SPDX-License-Identifier: GPL-3.0-or-later
//! The Mirror tool (docs/specs/drawing-improvements.md §1): the selected drawn shapes
//! mirrored about a line of two clicks, or about a straight side clicked, as copies (halves
//! on the line joining their image) or in place.

use super::*;
use chipload_core::primitives::distance;
use chipload_core::tolerances::CHAIN_TOL_MM;
use chipload_draw::snap::SnapKind;

/// The Mirror tool's state: whether it keeps the originals (kept for the session) and the
/// first point of the line, once clicked.
#[derive(Debug, Clone, PartialEq)]
pub struct MirrorSettings {
    pub keep: bool,
    pub from: Option<Point>,
}

impl Default for MirrorSettings {
    fn default() -> Self {
        MirrorSettings {
            keep: true,
            from: None,
        }
    }
}

/// Where the cursor puts the mirror line.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum MirrorAim {
    /// A whole line: from the first point to the cursor, or a straight side under it.
    Line(Point, Point),
    /// The line's first point, before it is clicked.
    First(Point),
}

impl ChiploadApp {
    /// Where the cursor at `raw` (over contour `picked`) puts the mirror line: with the
    /// first point clicked, the line to the cursor (snapped, Shift: 15° steps); before it,
    /// the straight side of a drawn shape the cursor lies on, unless it snaps to one of the
    /// side's points, which starts a line of two clicks instead.
    pub(super) fn mirror_aim(
        &mut self,
        picked: Option<usize>,
        raw: Point,
        alt: bool,
        shift: bool,
    ) -> Option<MirrorAim> {
        if let Some(a) = self.canvas_tools.mirror.from {
            let b = if shift {
                let step = (!alt).then(|| grid_step(self.view.scale, self.prefs.units));
                crate::draw::angle_step(a, raw, step, self.prefs.units)
            } else {
                self.snap_world(raw, alt, false)
            };
            return (distance(a, b) > CHAIN_TOL_MM).then_some(MirrorAim::Line(a, b));
        }
        let p = self.snap_world(raw, alt, false);
        let on_a_point = self
            .drag
            .snapped
            .and_then(|(_, hit)| hit)
            .is_some_and(|hit| hit.kind != SnapKind::Nearest);
        if !on_a_point && let Some(side) = self.straight_side_under(picked, raw) {
            return Some(MirrorAim::Line(side.0, side.1));
        }
        Some(MirrorAim::First(p))
    }

    /// The straight side of the drawn shape under the cursor, not one being mirrored, when
    /// the cursor lies on it.
    fn straight_side_under(&self, picked: Option<usize>, at: Point) -> Option<(Point, Point)> {
        let id = self.drawn_under(picked, at)?;
        if self.selected_drawn().contains(&id) {
            return None;
        }
        let job = &self.edit.as_ref()?.job;
        let (p, q) = chipload_pipeline::shapes::straight_line_of(job, id, Some(at))?;
        let (p, q) = ((p[0], p[1]), (q[0], q[1]));
        let reach = crate::view::PICK_TOLERANCE_PX as f64 / self.view.scale;
        let line = chipload_core::primitives::Primitive::Line(chipload_core::primitives::Line {
            start: p,
            end: q,
        });
        let near = chipload_draw::intersect::nearest_on(&line, at);
        (distance(near, at) <= reach).then_some((p, q))
    }

    /// The preview: the selected shapes mirrored about the line the cursor offers, and the
    /// line itself.
    pub(super) fn mirror_preview(&self, aim: Option<MirrorAim>) -> Vec<Vec<Point>> {
        let Some(MirrorAim::Line(a, b)) = aim else {
            return Vec::new();
        };
        let Some(edit) = self.edit.as_ref() else {
            return Vec::new();
        };
        let ids = self.selected_drawn();
        let mut lines: Vec<Vec<Point>> = edit
            .job
            .drawing
            .shapes
            .iter()
            .filter(|s| ids.contains(&s.id))
            .filter_map(|s| chipload_job::edit::mirrored_about(&s.geometry, [a.0, a.1], [b.0, b.1]))
            .flat_map(|g| crate::draw::outline(&g))
            .collect();
        lines.push(vec![a, b]);
        lines
    }

    /// A click with the Mirror tool: with nothing selected it picks shapes (Ctrl adds);
    /// else it sets the line's first point, or mirrors about the line it completes.
    pub(super) fn mirror_click(
        &mut self,
        picked: Option<usize>,
        at: Point,
        toggle: bool,
        keys: (bool, bool),
    ) {
        let (alt, shift) = keys;
        let picks_shape = picked.is_some() && self.drawn_under(picked, at).is_some();
        if self.selected_drawn().is_empty() || (toggle && picks_shape) {
            self.click(picked, toggle);
            return;
        }
        match self.mirror_aim(picked, at, alt, shift) {
            Some(MirrorAim::Line(a, b)) => {
                if self.apply_mirror(a, b) {
                    self.cancel_modify();
                }
            }
            Some(MirrorAim::First(p)) => self.canvas_tools.mirror.from = Some(p),
            None => {}
        }
    }

    /// Mirror the selected drawn shapes about the line through `a` and `b`, as one undo
    /// step: copies (selected afterwards) or in place, as the status bar says.
    pub fn apply_mirror(&mut self, a: Point, b: Point) -> bool {
        let ids = self.selected_drawn();
        let Some(job) = self.job_with_operations() else {
            return false;
        };
        let keep = self.canvas_tools.mirror.keep;
        let result = chipload_pipeline::shapes::mirror_shapes_about(
            &job,
            &ids,
            [a.0, a.1],
            [b.0, b.1],
            keep,
        );
        self.canvas_tools.mirror.from = None;
        match result {
            Ok((edited, joined)) => {
                let changed = if keep { joined } else { ids };
                self.take_edit(Ok(edited), &changed)
            }
            Err(err) => self.take_edit(Err(err), &[]),
        }
    }

    /// While mirroring: *Keep the original* on the status bar.
    pub(super) fn mirror_fields(&mut self, ui: &mut egui::Ui) {
        if self.modify() != Some(super::modify::ModifyTool::Mirror) {
            return;
        }
        ui.checkbox(
            &mut self.canvas_tools.mirror.keep,
            t!(ui, "modify-mirror-keep"),
        )
        .on_hover_text(t!(ui, "modify-mirror-keep-hover"));
        ui.separator();
    }
}
