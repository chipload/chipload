// SPDX-License-Identifier: GPL-3.0-or-later
//! A part's instances in the window (docs/specs/instances.md §4): added from the part's
//! menu, drawn as numbered ghosts where they are cut, picked with a click, moved by a drag
//! and turned by a handle (quarter turns snap within 3°, Alt turns freely). A drag
//! changes the instance as it goes and is one undo step when it ends; Escape puts it back.

use chipload_core::{Point, math};
use chipload_job::{Instance, Mirror};

use super::*;

/// How near a quarter turn snaps a turn by the handle (decision 2).
const QUARTER_SNAP_DEG: f64 = 3.0;

/// How far the turn handle sits beyond the ghost's outlines, px.
const TURN_HANDLE_GAP_PX: f64 = 18.0;

/// What a drag of an instance does.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum InstanceGrip {
    /// Grabbed by its outline: it moves with the pointer.
    Move,
    /// Grabbed by its turn handle: it turns about its centre.
    Turn,
}

/// A drag of an instance in progress.
#[derive(Debug, Clone, Copy)]
pub(super) struct InstanceDrag {
    part: u64,
    id: u32,
    grip: InstanceGrip,
    /// The instance when the drag began.
    start: Instance,
    /// Where the pointer took hold, in drawing coordinates.
    grab: Point,
    /// The centre it turns about, where it was when the drag began.
    centre: Point,
}

impl ChiploadApp {
    /// The instance `id` of part `part` in the project being edited.
    fn instance_mut(&mut self, part: u64, id: u32) -> Option<&mut Instance> {
        self.edit
            .as_mut()?
            .job
            .parts
            .iter_mut()
            .find(|p| p.id == part)?
            .instances
            .iter_mut()
            .find(|i| i.id == id)
    }

    /// A new instance of part `part` beside what it cuts, mirrored left to right with
    /// `mirrored`, selected (docs/specs/instances.md §4). Refused, with the reason shown,
    /// when the part can't be planned.
    pub(super) fn add_instance(&mut self, part: u64, mirrored: bool) {
        self.commit();
        let Some(edit) = &self.edit else {
            return;
        };
        let job = match edit.to_job() {
            Ok(job) => job,
            Err(problems) => {
                self.error_report(Report::of_problems(
                    &t!(self, "app-project-incomplete"),
                    &problems,
                ));
                return;
            }
        };
        let mirror = mirrored.then_some(Mirror::LeftRight);
        match chipload_pipeline::instances::add_instance(&job, part, None, 0.0, mirror) {
            Ok((made, id)) => {
                let instances = made.part(part).map(|p| p.instances.clone());
                if let (Some(instances), Some(target)) = (
                    instances,
                    self.edit
                        .as_mut()
                        .and_then(|e| e.job.parts.iter_mut().find(|p| p.id == part)),
                ) {
                    target.instances = instances;
                    self.focus = Focus::Instance(part, id);
                    self.commit();
                }
            }
            Err(err) => {
                self.error_report(Report::failed(
                    t!(self, "app-instance-needs-plan"),
                    err.message(),
                ));
            }
        }
    }

    /// Instance `id` of part `part` removed; the others keep their numbers.
    pub(super) fn remove_instance(&mut self, part: u64, id: u32) {
        if let Some(p) = self
            .edit
            .as_mut()
            .and_then(|e| e.job.parts.iter_mut().find(|p| p.id == part))
        {
            p.instances.retain(|i| i.id != id);
            if self.focus == Focus::Instance(part, id) {
                self.focus = Focus::Part(part);
            }
            self.commit();
        }
    }

    /// Instance `id` turned on or off.
    pub(super) fn toggle_instance(&mut self, part: u64, id: u32) {
        if let Some(instance) = self.instance_mut(part, id) {
            instance.enabled = !instance.enabled;
            self.commit();
        }
    }

    /// Instance `id` mirrored left to right, or back as the part is.
    pub(super) fn mirror_instance(&mut self, part: u64, id: u32) {
        if let Some(instance) = self.instance_mut(part, id) {
            instance.mirror = match instance.mirror {
                None => Some(Mirror::LeftRight),
                Some(_) => None,
            };
            self.commit();
        }
    }

    /// Instance `id` turned a quarter counter-clockwise.
    pub(super) fn turn_instance(&mut self, part: u64, id: u32) {
        if let Some(instance) = self.instance_mut(part, id) {
            instance.angle = tidy_angle(instance.angle + std::f64::consts::FRAC_PI_2);
            self.commit();
        }
    }

    /// The ghosts of the active setup's parts' instances, where they are cut, for the view.
    pub(super) fn instance_ghosts(&self) -> Vec<crate::view::InstanceGhost> {
        let Some(edit) = &self.edit else {
            return Vec::new();
        };
        if edit.job.parts.iter().all(|p| p.instances.is_empty()) {
            return Vec::new();
        }
        let setup = edit.job.active_setup_id();
        let scale = self.view.scale.max(f64::MIN_POSITIVE);
        chipload_pipeline::parts::instance_outlines(&edit.job, &self.contours)
            .into_iter()
            .filter(|i| setup.is_none() || edit.job.part(i.part).map(|p| p.setup) == setup)
            .map(|i| {
                let selected = self.focus == Focus::Instance(i.part, i.id);
                let angle = edit
                    .job
                    .part(i.part)
                    .and_then(|p| p.instances.iter().find(|k| k.id == i.id))
                    .map_or(0.0, |k| k.angle);
                let reach = i
                    .outlines
                    .iter()
                    .flat_map(|(points, _)| points)
                    .map(|p| math::hypot(p.0 - i.centre.0, p.1 - i.centre.1))
                    .fold(0.0, f64::max);
                let r = reach + TURN_HANDLE_GAP_PX / scale;
                let (s, c) = math::sin_cos(angle + std::f64::consts::FRAC_PI_2);
                let handle = (i.centre.0 + r * c, i.centre.1 + r * s);
                let label_at = i
                    .outlines
                    .iter()
                    .flat_map(|(points, _)| points)
                    .fold(None, |top: Option<Point>, &p| match top {
                        Some(t) if t.1 > p.1 || (t.1 == p.1 && t.0 <= p.0) => Some(t),
                        _ => Some(p),
                    })
                    .unwrap_or(i.centre);
                crate::view::InstanceGhost {
                    part: i.part,
                    id: i.id,
                    // A closed outline drawn and picked all the way round.
                    outlines: i
                        .outlines
                        .into_iter()
                        .map(|(mut points, closed)| {
                            if let (true, Some(&first)) = (closed, points.first()) {
                                points.push(first);
                            }
                            points
                        })
                        .collect(),
                    label: format!("#{}", i.id),
                    label_at,
                    selected,
                    enabled: i.enabled,
                    centre: i.centre,
                    handle: selected.then_some(handle),
                }
            })
            .collect()
    }

    /// The instance whose ghost is under `pos`: the nearest outline within reach.
    pub(super) fn instance_at(&self, pos: egui::Pos2) -> Option<(u64, u32)> {
        if self.scene.hide_drawing {
            return None;
        }
        let view = self.view;
        self.scene
            .instances
            .iter()
            .filter_map(|ghost| {
                let d = ghost
                    .outlines
                    .iter()
                    .flat_map(|points| points.windows(2))
                    .map(|w| segment_distance(pos, view.to_screen(w[0]), view.to_screen(w[1])))
                    .fold(f32::INFINITY, f32::min);
                (d <= crate::view::PICK_TOLERANCE_PX).then_some((d, ghost.part, ghost.id))
            })
            .min_by(|a, b| a.0.total_cmp(&b.0))
            .map(|(_, part, id)| (part, id))
    }

    /// The selected instance, when `pos` is on its turn handle.
    pub(super) fn instance_handle_at(&self, pos: egui::Pos2) -> Option<(u64, u32)> {
        self.scene.instances.iter().find_map(|ghost| {
            let handle = ghost.handle?;
            (self.view.to_screen(handle).distance(pos) <= crate::view::PICK_TOLERANCE_PX)
                .then_some((ghost.part, ghost.id))
        })
    }

    /// A drag of instance `id` of part `part` begins at `at`, grabbed by `grip`. It selects
    /// the instance.
    pub(super) fn start_instance_drag(
        &mut self,
        (part, id): (u64, u32),
        grip: InstanceGrip,
        at: Point,
    ) -> bool {
        let centre = self
            .scene
            .instances
            .iter()
            .find(|g| g.part == part && g.id == id)
            .map(|g| g.centre);
        let (Some(start), Some(centre)) = (self.instance_mut(part, id).copied(), centre) else {
            return false;
        };
        self.focus = Focus::Instance(part, id);
        self.drag.instance = Some(InstanceDrag {
            part,
            id,
            grip,
            start,
            grab: at,
            centre,
        });
        true
    }

    pub(super) fn dragging_instance(&self) -> bool {
        self.drag.instance.is_some()
    }

    /// The dragged instance with the pointer at `at`: moved by as much as the pointer, or
    /// turned about its centre as the pointer goes round it, a quarter turn within 3°
    /// snapping to it (`free`, Alt: as it is). Returns the label for the cursor.
    pub(super) fn drag_instance_to(&mut self, at: Point, free: bool) -> Option<String> {
        let drag = self.drag.instance?;
        let units = self.prefs.units;
        let instance = self.instance_mut(drag.part, drag.id)?;
        match drag.grip {
            InstanceGrip::Move => {
                let (dx, dy) = (at.0 - drag.grab.0, at.1 - drag.grab.1);
                instance.at = [drag.start.at[0] + dx, drag.start.at[1] + dy];
                Some(format!(
                    "X {}  Y {}",
                    crate::draw::fixed(instance.at[0], units),
                    crate::draw::fixed(instance.at[1], units)
                ))
            }
            InstanceGrip::Turn => {
                let c = drag.centre;
                let from = math::atan2(drag.grab.1 - c.1, drag.grab.0 - c.0);
                let to = math::atan2(at.1 - c.1, at.0 - c.0);
                let turned = drag.start.angle + (to - from);
                let angle = if free { turned } else { snapped_turn(turned) };
                // Turned about its centre where it lies: the move keeps that centre still.
                let angle = tidy_angle(angle);
                instance.angle = angle;
                Some(format!("{:.1}°", angle.to_degrees()))
            }
        }
    }

    /// The drag ends: the instance stays where it was dropped, as one undo step.
    pub(super) fn end_instance_drag(&mut self) {
        let Some(drag) = self.drag.instance.take() else {
            return;
        };
        let now = self.instance_mut(drag.part, drag.id).copied();
        if now.is_some_and(|now| now != drag.start) {
            self.commit();
        }
    }

    /// Escape during a drag: the instance goes back where it was. False when there was no
    /// drag.
    pub(super) fn cancel_instance_drag(&mut self) -> bool {
        let Some(drag) = self.drag.instance.take() else {
            return false;
        };
        if let Some(instance) = self.instance_mut(drag.part, drag.id) {
            *instance = drag.start;
        }
        true
    }
}

/// An angle in −π…π.
fn tidy_angle(angle: f64) -> f64 {
    use std::f64::consts::{PI, TAU};
    let a = (angle + PI).rem_euclid(TAU) - PI;
    // −π and π are the same turn: keep the one written as 180°.
    if a <= -PI + chipload_core::tolerances::ANGLE_TOL_RAD {
        PI
    } else {
        a
    }
}

/// A turn within [`QUARTER_SNAP_DEG`] of a quarter turn is that quarter turn.
fn snapped_turn(angle: f64) -> f64 {
    let quarter = std::f64::consts::FRAC_PI_2;
    let nearest = (angle / quarter).round() * quarter;
    if (angle - nearest).abs() <= QUARTER_SNAP_DEG.to_radians() {
        nearest
    } else {
        angle
    }
}

/// The distance in px from `p` to the segment `a`–`b`.
fn segment_distance(p: egui::Pos2, a: egui::Pos2, b: egui::Pos2) -> f32 {
    let ab = b - a;
    let length = ab.length_sq();
    if length <= f32::EPSILON {
        return p.distance(a);
    }
    let t = ((p - a).dot(ab) / length).clamp(0.0, 1.0);
    p.distance(a + ab * t)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::{FRAC_PI_2, PI};

    #[test]
    fn a_turn_near_a_quarter_snaps_to_it() {
        assert_eq!(snapped_turn(FRAC_PI_2 + 2.9_f64.to_radians()), FRAC_PI_2);
        assert_eq!(snapped_turn(-FRAC_PI_2 - 2.0_f64.to_radians()), -FRAC_PI_2);
        let free = FRAC_PI_2 + 3.5_f64.to_radians();
        assert_eq!(snapped_turn(free), free);
    }

    #[test]
    fn angles_stay_between_minus_and_plus_half_a_turn() {
        assert!((tidy_angle(3.0 * FRAC_PI_2) + FRAC_PI_2).abs() < 1e-12);
        assert_eq!(tidy_angle(-PI), PI);
        assert!((tidy_angle(0.3) - 0.3).abs() < 1e-15);
    }
}
