// SPDX-License-Identifier: GPL-3.0-or-later
//! The Scale tool (docs/specs/scale-and-stretch.md §2): two points on the drawing and the
//! length their distance should have; the selection scales by new ÷ now, the first point
//! staying. It scales whatever the box's corner handles scale: drawn shapes, a text, a
//! picture, a group or a whole DXF.

use super::box_drag::BoxOf;
use super::*;
use chipload_core::primitives::distance;
use chipload_core::tolerances::CHAIN_TOL_MM;
use chipload_pipeline::box_handles::PlacedBox;

/// The Scale tool's points, once clicked.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ScaleSettings {
    /// The point that stays.
    pub first: Option<Point>,
    /// The other end of the length measured.
    pub second: Option<Point>,
}

/// `placed` scaled by `factor` about `at`.
fn scaled_box(placed: &PlacedBox, at: Point, factor: f64) -> PlacedBox {
    let (cx, cy) = placed.center();
    let center = (at.0 + factor * (cx - at.0), at.1 + factor * (cy - at.1));
    let size = (placed.size.0 * factor, placed.size.1 * factor);
    PlacedBox {
        corner: (center.0 - size.0 / 2.0, center.1 - size.1 / 2.0),
        size,
        rotation: placed.rotation,
    }
}

impl ChiploadApp {
    /// What the tool scales: the selected drawn shapes (one circle too, which has no box),
    /// else what the box's corner handles scale, with its box.
    fn scale_target(&mut self) -> Option<(BoxOf, Option<PlacedBox>)> {
        let ids = self.selected_drawn();
        if !ids.is_empty() && matches!(self.selection, CanvasSelection::Shapes(_)) {
            return Some((BoxOf::Shapes(ids), None));
        }
        self.selected_box().map(|(of, placed)| (of, Some(placed)))
    }

    /// The point a click or the cursor at `raw` gives: snapped (Alt: free); after the first
    /// point, Shift keeps the line from it to 15° steps.
    pub(super) fn scale_point(&mut self, raw: Point, alt: bool, shift: bool) -> Point {
        match self.canvas_tools.scale.first {
            Some(a) if shift => {
                let step = (!alt).then(|| grid_step(self.view.scale, self.prefs.units));
                crate::draw::angle_step(a, raw, step, self.prefs.units)
            }
            _ => self.snap_world(raw, alt, false),
        }
    }

    /// The factor the cursor at `p` gives, once both points are clicked: its distance from
    /// the first point over the distance measured.
    fn scale_factor_at(&self, p: Point) -> Option<f64> {
        let (a, b) = (
            self.canvas_tools.scale.first?,
            self.canvas_tools.scale.second?,
        );
        let now = distance(a, b);
        let new = distance(a, p);
        (now > CHAIN_TOL_MM && new > CHAIN_TOL_MM).then(|| new / now)
    }

    /// The preview: the line measured, the line to the cursor, and the selection scaled by
    /// the cursor's distance (drawn shapes as they would be, anything else as its box).
    pub(super) fn scale_preview(&mut self, cursor: Option<Point>) -> Vec<Vec<Point>> {
        let mut lines = Vec::new();
        let settings = self.canvas_tools.scale.clone();
        let (Some(a), Some(p)) = (settings.first, cursor) else {
            return lines;
        };
        let Some(b) = settings.second else {
            lines.push(vec![a, p]);
            return lines;
        };
        lines.push(vec![a, b]);
        lines.push(vec![a, p]);
        let Some(factor) = self.scale_factor_at(p) else {
            return lines;
        };
        match self.scale_target() {
            Some((BoxOf::Shapes(ids), _)) => {
                if let Some(edit) = self.edit.as_ref() {
                    lines.extend(
                        edit.job
                            .drawing
                            .shapes
                            .iter()
                            .filter(|s| ids.contains(&s.id))
                            .filter_map(|s| {
                                chipload_job::edit::transformed(
                                    &s.geometry,
                                    [a.0, a.1],
                                    [a.0, a.1],
                                    factor,
                                    0.0,
                                )
                            })
                            .flat_map(|g| crate::draw::outline(&g)),
                    );
                }
            }
            Some((_, Some(placed))) => {
                let mut corners = scaled_box(&placed, a, factor).corners().to_vec();
                corners.push(corners[0]);
                lines.push(corners);
            }
            _ => {}
        }
        lines
    }

    /// A click with the Scale tool: with nothing to scale selected it picks (Ctrl adds);
    /// else it sets the first point, then the second (opening the new length's entry), then
    /// scales to the third point's distance from the first.
    pub(super) fn scale_click(
        &mut self,
        picked: Option<usize>,
        at: Point,
        toggle: bool,
        keys: (bool, bool),
    ) {
        let (alt, shift) = keys;
        let first = self.canvas_tools.scale.first;
        if first.is_none() && (self.scale_target().is_none() || (toggle && picked.is_some())) {
            self.click(picked, toggle);
            return;
        }
        let p = self.scale_point(at, alt, shift);
        let Some(a) = first else {
            self.canvas_tools.scale.first = Some(p);
            return;
        };
        let Some(_) = self.canvas_tools.scale.second else {
            if distance(a, p) <= CHAIN_TOL_MM {
                return;
            }
            self.canvas_tools.scale.second = Some(p);
            self.open_scale_entry(distance(a, p));
            return;
        };
        if self.apply_scale(distance(a, p)) {
            self.cancel_modify();
        }
    }

    /// The new length's entry, beside the second point, holding the length measured.
    fn open_scale_entry(&mut self, now: f64) {
        let at = self
            .canvas_tools
            .scale
            .second
            .map(|p| self.view.to_screen(p) + egui::vec2(16.0, 16.0))
            .unwrap_or_default();
        self.canvas_tools.typed = Some(super::drawing::Typed {
            text: chipload_pipeline::units::length_number(now, self.prefs.units),
            at,
            refused: false,
            focus: true,
            target: super::drawing::TypedTarget::ScaleLength,
        });
    }

    /// Enter in the new length's entry.
    pub(super) fn apply_typed_scale(&mut self) {
        let text = self
            .canvas_tools
            .typed
            .as_ref()
            .map(|t| t.text.clone())
            .unwrap_or_default();
        let text =
            crate::units::decimal_point(&text, crate::i18n::Lang::lang(self).decimal_comma());
        let applied = match chipload_pipeline::units::parse_length(&text, self.prefs.units) {
            Some(length) => self.apply_scale(length),
            None => {
                self.set_status(
                    crate::draw::EntryError::Unreadable.text(self),
                    STATUS_SECONDS,
                );
                false
            }
        };
        if applied {
            self.canvas_tools.typed = None;
            self.cancel_modify();
        } else if let Some(typed) = self.canvas_tools.typed.as_mut() {
            typed.refused = true;
            typed.focus = true;
        }
    }

    /// Scale the selection so the distance measured becomes `length`, the first point
    /// staying, as one undo step.
    pub fn apply_scale(&mut self, length: f64) -> bool {
        let settings = self.canvas_tools.scale.clone();
        let (Some(a), Some(b)) = (settings.first, settings.second) else {
            return false;
        };
        let factor =
            match chipload_pipeline::scaling::reference_factor([a.0, a.1], [b.0, b.1], length) {
                Ok(factor) => factor,
                Err(err) => {
                    let text = err
                        .message()
                        .render(self.prefs.language, self.prefs.units.into());
                    self.set_status(text, STATUS_SECONDS);
                    return false;
                }
            };
        let Some((of, placed)) = self.scale_target() else {
            return false;
        };
        let done = match (of, placed) {
            (BoxOf::Shapes(ids), _) => self.transform_shapes(&ids, a, a, factor, 0.0),
            (BoxOf::Whole(Source::Group(id)), _) => self.transform_group(id, a, a, factor, 0.0),
            (BoxOf::Whole(Source::Dxf(id)), _) => self.drag_dxf(id, a, a, factor, 0.0),
            (BoxOf::Whole(of), Some(placed)) => {
                let now = scaled_box(&placed, a, factor);
                self.place_whole(of, &placed, &now, true);
                true
            }
            (BoxOf::Whole(_), None) => false,
        };
        if done {
            self.canvas_tools.scale = ScaleSettings::default();
            let text = t!(
                self,
                "modify-scale-done",
                factor = chipload_pipeline::placement::factor_text(factor)
            );
            self.set_status(text, STATUS_SECONDS);
        }
        done
    }

    /// Esc while scaling: back one step. False when there is no step to go back.
    pub(super) fn scale_back(&mut self) -> bool {
        if self.modify() != Some(super::modify::ModifyTool::Scale) {
            return false;
        }
        let scale = &mut self.canvas_tools.scale;
        if scale.second.take().is_some() {
            self.canvas_tools.typed = None;
            return true;
        }
        scale.first.take().is_some()
    }
}
