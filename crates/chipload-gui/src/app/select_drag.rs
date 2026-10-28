// SPDX-License-Identifier: GPL-3.0-or-later
//! A left drag in top view: selecting with a box, or moving drawn shapes with the mouse
//! (docs/specs/drawing-input.md §3). Which contours a box catches is
//! `chipload_draw::select`; a move is `transform_shapes`, as the box handles' is.

use chipload_core::Point;
use chipload_core::chain::ShapeFrom;
use chipload_draw::select::{BoxMode, caught};

use super::*;

/// Drawn shapes being moved with the mouse: the point grabbed, and where it is now.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct MoveDrag {
    pub ids: Vec<u64>,
    pub grab: Point,
    pub to: Point,
}

impl ChiploadApp {
    /// Whether a left drag selects or moves rather than pans: top view, a project with its
    /// drawing shown, no tool active.
    pub(super) fn drag_selects(&self) -> bool {
        !self.three_d
            && self.sketch.is_none()
            && self.measure.is_none()
            && self.modify.is_none()
            && !self.placing_text()
            && self.can_draw()
            && self.show_drawing()
    }

    /// A left drag starts at `at` (drawing coordinates) over the contour `picked`: drawn
    /// shapes there move, else a selection box opens. `add`: Shift was held (an unselected
    /// shape is added to the selection before it moves). `free`: Alt, no snapping.
    pub fn start_select_drag(&mut self, picked: Option<usize>, at: Point, add: bool, free: bool) {
        match self.shapes_to_move(picked, add) {
            Some(ids) => {
                let grab = self.snap_world(at, free, false);
                self.drag.moving = Some(MoveDrag {
                    ids,
                    grab,
                    to: grab,
                });
            }
            None => self.drag.select_box = Some((at, at)),
        }
    }

    /// The drawn shapes a drag starting on `picked` moves: the selection, if the contour is
    /// in it, else the contour's own shapes (selected first). None when the contour isn't
    /// drawn (a DXF's, a picture's, a text's).
    fn shapes_to_move(&mut self, picked: Option<usize>, add: bool) -> Option<Vec<u64>> {
        let contour = self.contours.iter().find(|c| Some(c.id) == picked)?;
        let id = contour.id;
        let hit = shapes_of(contour);
        let drawn = !hit.is_empty()
            && hit
                .iter()
                .all(|&s| self.shape(s).is_some_and(chipload_job::Shape::is_drawn));
        if !drawn {
            return None;
        }
        if !self.selection.ids(&self.contours).contains(&id) {
            self.click(picked, add);
        }
        let (ids, imported) = self.deletable_shapes();
        if imported || ids.is_empty() {
            Some(hit.into_iter().collect())
        } else {
            Some(ids)
        }
    }

    /// The moved shapes' grabbed point to the cursor at `cursor`: snapped (their own points
    /// left out), or with Shift held to 15° steps from where it was grabbed.
    pub fn drag_move_to(&mut self, cursor: Point, free: bool, shift: bool) {
        let Some(grab) = self.drag.moving.as_ref().map(|m| m.grab) else {
            return;
        };
        let to = if shift {
            let step = (!free).then(|| grid_step(self.view.scale, self.units));
            let p = crate::draw::angle_step(grab, cursor, step, self.units);
            self.drag.snapped = Some((p, None));
            self.scene.snap_mark = None;
            p
        } else {
            self.snap_world(cursor, free, true)
        };
        if let Some(moving) = self.drag.moving.as_mut() {
            moving.to = to;
        }
    }

    /// The move ends: the shapes are moved, as one undo step, keeping their ids.
    pub fn end_move_drag(&mut self) {
        let Some(moving) = self.drag.moving.take() else {
            return;
        };
        self.scene.snap_mark = None;
        if moving.to != moving.grab {
            self.transform_shapes(&moving.ids, moving.grab, moving.to, 1.0, 0.0);
        }
    }

    /// The outlines of the shapes being moved, where they would land.
    pub(super) fn move_outlines(&self) -> Vec<Vec<Point>> {
        let (Some(moving), Some(edit)) = (&self.drag.moving, &self.edit) else {
            return Vec::new();
        };
        if moving.to == moving.grab {
            return Vec::new();
        }
        let (from, to) = ([moving.grab.0, moving.grab.1], [moving.to.0, moving.to.1]);
        edit.settings
            .drawing
            .shapes
            .iter()
            .filter(|s| moving.ids.contains(&s.id))
            .filter_map(|s| chipload_job::edit::transformed(&s.geometry, from, to, 1.0, 0.0))
            .flat_map(|g| crate::draw::outline(&g))
            .collect()
    }

    /// The selection box's far corner follows the cursor at `at`.
    pub fn drag_select_to(&mut self, at: Point) {
        if let Some((_, end)) = self.drag.select_box.as_mut() {
            *end = at;
        }
    }

    /// The selection box being dragged: its corners and how it catches.
    pub(super) fn select_box(&self) -> Option<(Point, Point, BoxMode)> {
        let (from, to) = self.drag.select_box?;
        Some((from, to, BoxMode::of_drag(from, to)))
    }

    /// The box is let go: the contours it catches become the selection; with `add` (Shift)
    /// they are added to it, with `toggle` (Ctrl) each is added or taken out. Contours of
    /// a picture or text count only when the box catches nothing else and replaces the
    /// selection: then it selects the whole picture or text, as a click does.
    pub fn end_select_drag(&mut self, add: bool, toggle: bool) {
        // What the box picks is what Properties shows (one thing at a time).
        self.focus = Focus::Selection;
        let Some((from, to, mode)) = self.select_box() else {
            return;
        };
        self.drag.select_box = None;
        let mut plain = BTreeSet::new();
        let mut sources = BTreeSet::new();
        for contour in self
            .contours
            .iter()
            .filter(|c| !self.hidden_contours.contains(&c.id))
        {
            let segments: Vec<_> = contour.parts.iter().map(|p| p.segment.prim).collect();
            if !caught(&segments, from, to, mode) {
                continue;
            }
            match whole_source(contour) {
                Some(source) => {
                    sources.insert(source);
                }
                None => {
                    plain.insert(contour.id);
                }
            }
        }
        if plain.is_empty()
            && !add
            && !toggle
            && let [only] = sources.iter().copied().collect::<Vec<_>>()[..]
        {
            self.selection = CanvasSelection::Source(only);
            return;
        }
        let hit = shapes_in(&plain, &self.contours);
        let mut shapes = if add || toggle {
            shapes_in(&self.selection.ids(&self.contours), &self.contours)
        } else {
            BTreeSet::new()
        };
        if toggle {
            for s in hit {
                if !shapes.remove(&s) {
                    shapes.insert(s);
                }
            }
        } else {
            shapes.extend(hit);
        }
        self.selection = if shapes.is_empty() {
            CanvasSelection::Nothing
        } else {
            CanvasSelection::Shapes(shapes)
        };
    }
}

/// The picture or text a contour is wholly made of, if any.
fn whole_source(contour: &Contour) -> Option<ShapeFrom> {
    let from = contour.parts.first()?.segment.from?;
    let whole = contour.parts.iter().all(|p| p.segment.from == Some(from));
    (whole && matches!(from, ShapeFrom::Image(_) | ShapeFrom::Text(_))).then_some(from)
}

/// The shapes of these contours.
fn shapes_in(ids: &BTreeSet<usize>, contours: &[Contour]) -> BTreeSet<u64> {
    match CanvasSelection::of_contours(ids, contours) {
        CanvasSelection::Shapes(shapes) => shapes,
        _ => BTreeSet::new(),
    }
}
