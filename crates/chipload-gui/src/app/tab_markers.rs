// SPDX-License-Identifier: GPL-3.0-or-later
//! Tabs placed by hand on the canvas (docs/specs/tabs-by-hand.md §5): the focused profile's
//! tabs as markers to drag along their ring, and right-click to add, remove or space a
//! ring's tabs evenly again. Each change is one undo step and generates again.

use super::*;
use chipload_core::order::Ordered;
use chipload_pipeline::tabs::{
    Ring, with_ring_even, with_tab_added, with_tab_moved, with_tab_removed,
};
use chipload_toolpath::plan::TabCentre;

use crate::view::TabMarker;

impl ChiploadApp {
    /// The markers for the last generation's tabs.
    pub(super) fn show_tabs(&mut self, centres: &[TabCentre]) {
        self.scene.tab_markers = centres
            .iter()
            .map(|&tab| TabMarker { tab, at: tab.at })
            .collect();
    }

    /// The focused profile, when it has tabs and its tabs are shown.
    fn tab_operation(&self) -> Option<usize> {
        let Focus::Op(i) = self.focus else {
            return None;
        };
        if self.three_d || self.sketch.is_some() || self.scene.hide_toolpath {
            return None;
        }
        let edit = self.edit.as_ref()?;
        match &edit.ops.get(i)?.op {
            Operation::Profile(p) if p.tabs.is_some() => Some(i),
            _ => None,
        }
    }

    /// The focused profile's tab marker at `pos`.
    pub(super) fn tab_marker_at(&self, pos: egui::Pos2) -> Option<usize> {
        let operation = self.tab_operation()?;
        self.scene
            .tab_markers
            .iter()
            .enumerate()
            .filter(|(_, m)| m.tab.operation == operation)
            .map(|(i, m)| (i, self.view.to_screen(m.at).distance(pos)))
            .filter(|&(_, d)| d <= crate::view::PICK_TOLERANCE_PX + 2.0)
            .min_by_key(|&(_, d)| Ordered(f64::from(d)))
            .map(|(i, _)| i)
    }

    /// The rings of operation `operation`, in planning order (as its tabs number them).
    fn tab_rings(&self, operation: usize) -> Vec<Ring> {
        self.scene
            .start_markers
            .iter()
            .filter(|m| m.operation == operation)
            .map(|m| m.ring.clone())
            .collect()
    }

    /// The focused profile's ring under `pos`, and the point on it there.
    pub(super) fn tab_ring_at(&self, pos: egui::Pos2) -> Option<(usize, Point)> {
        let operation = self.tab_operation()?;
        let world = self.view.to_world(pos);
        self.tab_rings(operation)
            .iter()
            .enumerate()
            .filter_map(|(k, ring)| {
                let (_, at, _) = chipload_toolpath::start::nearest_on_ring(ring, world)?;
                let d = self.view.to_screen(at).distance(pos);
                (d <= crate::view::PICK_TOLERANCE_PX).then_some((k, at, d))
            })
            .min_by_key(|&(_, _, d)| Ordered(f64::from(d)))
            .map(|(k, at, _)| (k, at))
    }

    /// Move tab marker `marker` along its ring, towards `to` (drawing coordinates).
    pub fn drag_tab_to(&mut self, marker: usize, to: Point) {
        self.drag.tab = Some(marker);
        let Some(m) = self.scene.tab_markers.get(marker).copied() else {
            return;
        };
        let rings = self.tab_rings(m.tab.operation);
        if let Some((_, at, _)) = rings
            .get(m.tab.ring)
            .and_then(|ring| chipload_toolpath::start::nearest_on_ring(ring, to))
            && let Some(m) = self.scene.tab_markers.get_mut(marker)
        {
            m.at = at;
        }
    }

    /// Store where the dragged tab was left, keeping its ring's other tabs where they are.
    pub fn drop_tab(&mut self) {
        let Some(marker) = self.drag.tab.take() else {
            return;
        };
        let Some(m) = self.scene.tab_markers.get(marker).copied() else {
            return;
        };
        if chipload_core::primitives::distance(m.at, m.tab.at)
            <= chipload_core::tolerances::CHAIN_TOL_MM
        {
            return;
        }
        let tabs = self.op_tabs(m.tab.operation);
        self.change_tabs(m.tab.operation, |points| {
            with_tab_moved(points, &tabs, &m.tab, [m.at.0, m.at.1]);
        });
    }

    /// Remove tab marker `marker`'s tab.
    pub fn remove_tab(&mut self, marker: usize) {
        let Some(m) = self.scene.tab_markers.get(marker).copied() else {
            return;
        };
        let tabs = self.op_tabs(m.tab.operation);
        self.change_tabs(m.tab.operation, |points| {
            with_tab_removed(points, &tabs, &m.tab);
        });
    }

    /// Space the tabs of marker `marker`'s ring evenly again.
    pub fn even_tabs(&mut self, marker: usize) {
        let Some(m) = self.scene.tab_markers.get(marker).copied() else {
            return;
        };
        let rings = self.tab_rings(m.tab.operation);
        self.change_tabs(m.tab.operation, |points| {
            with_ring_even(points, &rings, m.tab.ring);
        });
    }

    /// Add a tab to the focused profile's ring `ring` at `at`.
    pub fn add_tab(&mut self, ring: usize, at: Point) {
        let Some(operation) = self.tab_operation() else {
            return;
        };
        let tabs = self.op_tabs(operation);
        self.change_tabs(operation, |points| {
            with_tab_added(points, &tabs, ring, [at.0, at.1]);
        });
    }

    /// The last generation's tabs of operation `operation`, as planned.
    fn op_tabs(&self, operation: usize) -> Vec<TabCentre> {
        self.scene
            .tab_markers
            .iter()
            .filter(|m| m.tab.operation == operation)
            .map(|m| m.tab)
            .collect()
    }

    /// Change operation `operation`'s tabs placed by hand, as one undo step, and generate
    /// again.
    fn change_tabs(&mut self, operation: usize, change: impl FnOnce(&mut Vec<[f64; 2]>)) {
        let Some(points) = self
            .edit
            .as_mut()
            .and_then(|e| e.ops.get_mut(operation))
            .and_then(|entry| entry.op.tab_points_mut())
        else {
            return;
        };
        change(points);
        self.commit();
        self.undo_if_refused.clone_from(&self.edit);
        self.generate();
    }
}
