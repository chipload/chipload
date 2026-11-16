// SPDX-License-Identifier: GPL-3.0-or-later
//! Picking on the canvas, and which contours an operation or the focus shows.

use std::collections::BTreeSet;

use chipload_job::{Operation, Source};
use chipload_toolpath::select::{select_contours, select_paths};

use super::ChiploadApp;
use super::{Focus, selector_text};
use crate::editor::{CanvasSelection, shapes_of, with_twins};
use crate::i18n::t;

impl ChiploadApp {
    /// Click on the canvas: select one contour, Ctrl toggles, empty space clears.
    /// A click on the canvas: `picked` is the contour hit (if any); `toggle` (Ctrl) adds or
    /// removes it instead of replacing the selection. Selections are kept as shape ids.
    pub fn click(&mut self, picked: Option<usize>, toggle: bool) {
        // Picking on the canvas shows what is picked, and only that (the user, 2026-10-28).
        // The selection stays when something else is opened, for an operation's Use selection.
        if picked.is_some() {
            self.focus = Focus::Selection;
        }
        // A grouped shape selects its whole group (docs/specs/shape-groups.md §2, decision 10).
        if let Some(group) = picked.and_then(|id| self.group_of_contour(id))
            && !toggle
        {
            self.selection = CanvasSelection::whole(Source::Group(group));
            return;
        }
        // A traced contour selects its whole picture, a letter its whole text: their shapes
        // get new ids each time the picture is traced again or the text written again, so
        // operations select the picture or text, not its shapes (docs/specs/image-trace.md
        // §4, decision 6; user report 2026-10-13; text.md §5). A DXF's contour selects the
        // whole DXF too, to be moved by its box; a double-click picks the contour alone
        // (dxf-whole-on-canvas.md §2).
        let traced = picked
            .and_then(|id| self.contours.iter().find(|c| c.id == id))
            .and_then(|c| match c.parts.first().and_then(|p| p.segment.from) {
                Some(from) if c.parts.iter().all(|p| p.segment.from == Some(from)) => Some(from),
                _ => None,
            });
        if let (Some(from), false) = (traced, toggle) {
            self.selection = CanvasSelection::whole(from);
            // A letter opens its text, as the text's Browser row does: its words, font and
            // height to change (the user, 2026-10-30). The text stays selected for an
            // operation's Use selection.
            if let chipload_core::chain::ShapeFrom::Text(id) = from {
                self.focus = Focus::Text(id);
            }
            return;
        }
        // Ctrl+click on a contour of the selected DXF, picture, text or group leaves it out,
        // or puts it back (docs/specs/leave-out-shapes.md §5, shape-groups.md decision 10).
        if toggle
            && let CanvasSelection::Whole { of, left_out } = &self.selection
            && let Some(contour) = picked.and_then(|id| self.contours.iter().find(|c| c.id == id))
            && CanvasSelection::whole(*of)
                .ids(&self.contours, self.edit.as_ref().map(|e| &e.job.drawing))
                .contains(&contour.id)
        {
            let mut left_out = left_out.clone();
            let hit = with_twins(shapes_of(contour), &self.twins);
            if hit.is_subset(&left_out) {
                left_out.retain(|s| !hit.contains(s));
            } else {
                left_out.extend(hit);
            }
            self.selection = CanvasSelection::Whole { of: *of, left_out };
            return;
        }
        let mut shapes: BTreeSet<u64> = match (&self.selection, toggle) {
            (CanvasSelection::Shapes(shapes), true) => shapes.clone(),
            (CanvasSelection::Layer(..) | CanvasSelection::Whole { .. }, true) => {
                let ids = self.selected_ids();
                match CanvasSelection::of_contours(&ids, &self.contours, &self.twins) {
                    CanvasSelection::Shapes(shapes) => shapes,
                    _ => BTreeSet::new(),
                }
            }
            _ => BTreeSet::new(),
        };
        let hit: BTreeSet<u64> = picked
            .and_then(|id| self.contours.iter().find(|c| c.id == id))
            .map(|c| with_twins(shapes_of(c), &self.twins))
            .unwrap_or_default();
        if toggle && !hit.is_empty() && hit.is_subset(&shapes) {
            shapes.retain(|s| !hit.contains(s));
        } else {
            shapes.extend(hit);
        }
        self.selection = if shapes.is_empty() {
            CanvasSelection::Nothing
        } else {
            CanvasSelection::Shapes(shapes)
        };
    }

    #[cfg(test)]
    pub fn select_layer(&mut self, layer: &str) {
        self.selection = CanvasSelection::Layer(layer.to_string(), None);
    }

    /// A job selector for the canvas selection, e.g. {"shapes": [3, 4]}.
    pub fn selector_json(&self) -> Option<String> {
        let selector = self.op_selection().selector(&self.contours, false);
        (!selector.is_empty()).then(|| selector_text(&selector))
    }

    pub fn selection_lines(&self) -> Vec<String> {
        let ids = self.selected_ids();
        if ids.is_empty() {
            return vec![t!(self, "app-nothing-selected")];
        }
        let what = match &self.selection {
            CanvasSelection::Layer(layer, _) => t!(self, "app-selection-layer", layer = layer),
            CanvasSelection::Whole { of, left_out } => {
                let count = CanvasSelection::whole(*of)
                    .ids(&self.contours, self.edit.as_ref().map(|e| &e.job.drawing))
                    .len()
                    - ids.len();
                match (of.shape_from(), left_out.is_empty()) {
                    (Some(from), true) => self.source_text(from),
                    (Some(from), false) => t!(
                        self,
                        "app-selection-left-out",
                        source = self.source_text(from),
                        count = count
                    ),
                    (None, true) => t!(self, "groups-selection", name = self.group_name(of.id())),
                    (None, false) => t!(
                        self,
                        "groups-selection-left-out",
                        name = self.group_name(of.id()),
                        count = count
                    ),
                }
            }
            _ => t!(self, "app-selection-contours"),
        };
        let list: Vec<String> = ids.iter().map(|i| format!("#{i}")).collect();
        let mut lines = vec![t!(
            self,
            "app-selected",
            count = ids.len(),
            what = what,
            list = list.join(", ")
        )];
        let open: Vec<String> = self
            .contours
            .iter()
            .filter(|c| ids.contains(&c.id) && !c.closed)
            .map(|c| format!("#{}", c.id))
            .collect();
        if !open.is_empty() {
            lines.push(t!(self, "app-open-not-cut", list = open.join(", ")));
        }
        lines
    }

    /// What the selection's shapes come from: a DXF, an image or a text.
    fn source_text(&self, from: chipload_core::chain::ShapeFrom) -> String {
        match from {
            chipload_core::chain::ShapeFrom::Dxf(id) => t!(self, "app-selection-dxf", id = id),
            chipload_core::chain::ShapeFrom::Image(id) => {
                t!(self, "app-selection-image", id = id)
            }
            chipload_core::chain::ShapeFrom::Text(id) => t!(self, "app-selection-text", id = id),
            chipload_core::chain::ShapeFrom::Model(id) => {
                t!(self, "app-selection-model", id = id)
            }
        }
    }

    // -- internals

    pub(super) fn focused_contours(&self) -> BTreeSet<usize> {
        let (
            Focus::Op(i)
            | Focus::RestPass(i, _)
            | Focus::FloorPass(i)
            | Focus::RoughingPass(i)
            | Focus::SmoothingPass(i),
            Some(edit),
        ) = (self.focus, &self.edit)
        else {
            return BTreeSet::new();
        };
        edit.job
            .operations
            .get(i)
            .map(|entry| self.op_contours(entry))
            .unwrap_or_default()
    }

    /// The contours an operation cuts, as planning selects them.
    pub(super) fn op_contours(&self, op: &Operation) -> BTreeSet<usize> {
        if !op.selects_shapes() {
            return BTreeSet::new();
        }
        let select = match &self.edit {
            Some(edit) => op.select().resolved(&edit.job.drawing),
            None => op.select().clone(),
        };
        let selected = match op {
            Operation::Path(_) | Operation::Holes(_) => {
                select_paths(&self.contours, &select, &op.label_message(0))
            }
            _ => select_contours(&self.contours, &select, &op.label_message(0)),
        };
        // Holes cut circles only: other shapes stay unhighlighted (docs/specs/holes.md §7).
        let holes = matches!(op, Operation::Holes(_));
        selected
            .map(|s| {
                s.contours
                    .iter()
                    .filter(|c| !holes || chipload_toolpath::holes::circle_of(c).is_some())
                    .map(|c| c.id)
                    .collect()
            })
            .unwrap_or_default()
    }

    /// How many of an operation's contours are hidden (docs/specs/drawing-sources.md §4).
    pub(super) fn hidden_cut(&self, op: &Operation) -> usize {
        if self.hidden_contours.is_empty() {
            return 0;
        }
        self.op_contours(op)
            .intersection(&self.hidden_contours)
            .count()
    }

    /// The open contours an enabled follow path or flute cuts (docs/specs/follow-path.md §7).
    pub(super) fn followed_contours(&self) -> BTreeSet<usize> {
        let Some(edit) = &self.edit else {
            return BTreeSet::new();
        };
        edit.job
            .operations
            .iter()
            .filter(|e| {
                e.enabled()
                    && matches!(
                        e,
                        Operation::Path(_) | Operation::Flute(_) | Operation::Moulding(_)
                    )
            })
            .filter_map(|e| {
                let select = e.select().resolved(&edit.job.drawing);
                select_paths(&self.contours, &select, &e.label_message(0)).ok()
            })
            .flat_map(|s| {
                s.contours
                    .iter()
                    .filter(|c| !c.closed)
                    .map(|c| c.id)
                    .collect::<Vec<_>>()
            })
            .collect()
    }
}
