// SPDX-License-Identifier: GPL-3.0-or-later
//! A part's instances in the window (docs/specs/instances.md §4): added from the part's
//! menu, drawn as numbered ghosts where they are cut, picked with a click, moved by a drag
//! and turned by a handle (quarter turns snap within 3°, Alt turns freely). A drag
//! changes the instance as it goes and is one undo step when it ends; Escape puts it back.
//! Every instance is alike, instance 1 too (docs/specs/parts-and-setups.md §6): its ghost
//! shows where it is cut once that isn't where its part is drawn, or while it is selected.
//! The setups hold every instance (docs/specs/parts-and-setups.md §2).

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
    /// A copy of this instance made as the drag began (Ctrl+drag): kept when dropped, gone
    /// on Escape.
    copy_of: Option<u32>,
}

impl ChiploadApp {
    /// Instance `id` of part `part` in the project being edited.
    pub(super) fn instance_value(&self, part: u64, id: u32) -> Option<Instance> {
        self.edit.as_ref()?.job.instance(part, id).copied()
    }

    /// Instance `id` of part `part` made `value`. Whether there was one.
    pub(super) fn set_instance_value(&mut self, part: u64, id: u32, value: Instance) -> bool {
        let Some(instance) = self
            .edit
            .as_mut()
            .and_then(|e| e.job.instance_mut(part, id))
        else {
            return false;
        };
        *instance = Instance { part, id, ..value };
        true
    }

    /// The number of part `part`'s first instance: on the active setup, else anywhere.
    pub(super) fn first_instance(&self, part: u64) -> Option<u32> {
        let job = &self.edit.as_ref()?.job;
        job.instances_here(part)
            .first()
            .map(|i| i.id)
            .or_else(|| job.instances_of(part).first().map(|(_, i)| i.id))
    }

    /// *Place…*: part `part`'s first instance selected, its ghost shown where it is cut and
    /// its form open, to move and turn it (docs/specs/part-placement.md §6).
    pub(super) fn place_part(&mut self, part: u64) {
        if let Some(id) = self.first_instance(part) {
            self.focus = Focus::Instance(part, id);
        }
    }

    /// Part `part` placed on setup `setup` where it is drawn, a new instance, selected, as one
    /// undo step (docs/specs/parts-and-setups.md §8: a part's row dropped on a setup, or
    /// *Place on setup ▸*).
    pub(super) fn place_part_on(&mut self, part: u64, setup: u64) {
        let Some(edit) = self.edit.as_mut() else {
            return;
        };
        let id = edit.job.next_instance_id(part);
        if edit.job.place_part(part, setup) {
            edit.job.follow_thickness();
            self.focus = Focus::Instance(part, id);
            self.commit();
        }
    }

    /// Instance `id` of part `part` cut on setup `setup`, keeping its number and placement,
    /// as one undo step (§8: its row dropped on a setup, or *Move to setup ▸*).
    pub(super) fn instance_to_setup(&mut self, part: u64, id: u32, setup: u64) {
        let Some(edit) = self.edit.as_mut() else {
            return;
        };
        match chipload_pipeline::instances::move_instance(&edit.job, part, id, setup) {
            Ok(mut moved) => {
                moved.follow_thickness();
                edit.job.setups = moved.setups;
                edit.job.operations = moved.operations;
                self.commit();
            }
            Err(err) => self.error(vec![t!(self, "app-setup-refused", error = err.to_string())]),
        }
    }

    /// Part `part`'s first instance cut where drawn again, as one undo step (*Reset
    /// placement*); its mirror and on or off stay.
    pub(super) fn reset_part_placement(&mut self, part: u64) {
        let Some(id) = self.first_instance(part) else {
            return;
        };
        if let Some(mut now) = self.instance_value(part, id)
            && (now.angle != 0.0 || now.at != [0.0, 0.0])
        {
            now.angle = 0.0;
            now.at = [0.0, 0.0];
            self.set_instance_value(part, id, now);
            self.commit();
        }
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
        let made = chipload_pipeline::instances::add_instance(&job, part, None, None, 0.0, mirror);
        if self.adopt_instance(part, made).is_some() {
            self.commit();
        }
    }

    /// A new instance like instance `id` of part `part` (turned, mirrored and on its setup),
    /// beside what the part cuts, selected, one undo step (docs/specs/canvas-handling.md
    /// §3). False when refused, the reason shown.
    pub(super) fn duplicate_instance(&mut self, part: u64, id: u32) -> bool {
        self.commit();
        let made = self.duplicated(part, id, None);
        let done = made.is_some();
        if done {
            self.commit();
        }
        done
    }

    /// Instance `id` of part `part` duplicated, moved by `at` or beside the others, and
    /// selected; not yet an undo step. Its number, or None when refused (the reason shown).
    fn duplicated(&mut self, part: u64, id: u32, at: Option<[f64; 2]>) -> Option<u32> {
        let job = match self.edit.as_ref()?.to_job() {
            Ok(job) => job,
            Err(problems) => {
                self.error_report(Report::of_problems(
                    &t!(self, "app-project-incomplete"),
                    &problems,
                ));
                return None;
            }
        };
        let made = chipload_pipeline::instances::duplicate_instance(&job, part, id, at);
        self.adopt_instance(part, made)
    }

    /// The setups' instances taken from `made` and its new instance selected; the reason
    /// shown when it was refused.
    fn adopt_instance(
        &mut self,
        part: u64,
        made: Result<(chipload_job::Job, u32), chipload_pipeline::PipelineError>,
    ) -> Option<u32> {
        match made {
            Ok((made, id)) => {
                let edit = self.edit.as_mut()?;
                for (setup, target) in made.setups.iter().zip(edit.job.setups.iter_mut()) {
                    target.instances = setup.instances.clone();
                }
                self.focus = Focus::Instance(part, id);
                Some(id)
            }
            Err(err) => {
                self.error_report(Report::failed(
                    t!(self, "app-instance-needs-plan"),
                    err.message(),
                ));
                None
            }
        }
    }

    /// A Ctrl+drag of instance `id` of part `part` begins at `at`: a copy is made where it
    /// is, and that copy is dragged; the original stays (docs/specs/canvas-handling.md §3).
    pub(super) fn start_instance_copy(&mut self, (part, id): (u64, u32), at: Point) -> bool {
        let centre = self
            .scene
            .instances
            .iter()
            .find(|g| g.part == part && g.id == id)
            .map(|g| g.centre);
        let (Some(original), Some(centre)) = (self.instance_value(part, id), centre) else {
            return false;
        };
        self.commit();
        let Some(copy) = self.duplicated(part, id, Some(original.at)) else {
            return false;
        };
        let Some(start) = self.instance_value(part, copy) else {
            return false;
        };
        self.drag.instance = Some(InstanceDrag {
            part,
            id: copy,
            grip: InstanceGrip::Move,
            start,
            grab: at,
            centre,
            copy_of: Some(id),
        });
        true
    }

    /// Instance `id` of part `part` removed, instance 1 as any other; the others keep their
    /// numbers. A part left with none is placed nowhere (docs/specs/parts-and-setups.md §2).
    pub(super) fn remove_instance(&mut self, part: u64, id: u32) {
        if let Some(edit) = self.edit.as_mut() {
            for setup in &mut edit.job.setups {
                setup.instances.retain(|i| i.part != part || i.id != id);
            }
            if self.focus == Focus::Instance(part, id) {
                self.focus = Focus::Part(part);
            }
            self.commit();
        }
    }

    /// Instance `id` turned on or off.
    pub(super) fn toggle_instance(&mut self, part: u64, id: u32) {
        if let Some(mut instance) = self.instance_value(part, id) {
            instance.enabled = !instance.enabled;
            self.set_instance_value(part, id, instance);
            self.commit();
        }
    }

    /// Instance `id` mirrored left to right, or back as the part is.
    pub(super) fn mirror_instance(&mut self, part: u64, id: u32) {
        if let Some(mut instance) = self.instance_value(part, id) {
            instance.mirror = match instance.mirror {
                None => Some(Mirror::LeftRight),
                Some(_) => None,
            };
            self.set_instance_value(part, id, instance);
            self.commit();
        }
    }

    /// Instance `id` turned a quarter, counter-clockwise (`left`) or clockwise, about its
    /// centre.
    pub(super) fn turn_instance(&mut self, part: u64, id: u32, left: bool) {
        if let Some(mut instance) = self.instance_value(part, id) {
            let quarter = std::f64::consts::FRAC_PI_2;
            let turn = if left { quarter } else { -quarter };
            instance.angle = tidy_angle(instance.angle + turn);
            self.set_instance_value(part, id, instance);
            self.commit();
        }
    }

    /// The ghosts of the active setup's parts' instances, where they are cut, for the view.
    pub(super) fn instance_ghosts(&self) -> Vec<crate::view::InstanceGhost> {
        let Some(edit) = &self.edit else {
            return Vec::new();
        };
        // Every instance on the active setup, those cut where drawn too: a setup's space shows
        // no drawn shapes (docs/specs/drawing-and-setup-spaces.md §4).
        let shown: Vec<(u64, u32)> = edit
            .job
            .parts
            .iter()
            .flat_map(|p| {
                edit.job
                    .instances_here(p.id)
                    .into_iter()
                    .map(|i| (p.id, i.id))
            })
            .collect();
        let here = |part: u64, id: u32| shown.contains(&(part, id));
        // A part cut once, where drawn, is just the part: no badge (parts-and-setups.md §2a).
        let alone = |part: u64| {
            let all = edit.job.instances_of(part);
            all.len() == 1 && all.iter().all(|(_, i)| i.is_drawn())
        };
        let scale = self.view.scale.max(f64::MIN_POSITIVE);
        // What a press at the pointer would take, lit up (decision 13 of the handles).
        let hot = self
            .cursor
            .filter(|_| !self.dragging_instance())
            .and_then(|c| self.instance_grip_at(self.view.to_screen(c)));
        chipload_pipeline::parts::instance_outlines_showing(&edit.job, &self.contours, &shown)
            .into_iter()
            .filter(|i| here(i.part, i.id))
            .map(|i| {
                let selected = self.focus == Focus::Instance(i.part, i.id);
                let angle = self.instance_value(i.part, i.id).map_or(0.0, |k| k.angle);
                let reach = i
                    .outlines
                    .iter()
                    .flat_map(|(points, _)| points)
                    .map(|p| math::hypot(p.0 - i.centre.0, p.1 - i.centre.1))
                    .fold(0.0, f64::max);
                let r = reach + TURN_HANDLE_GAP_PX / scale;
                let (s, c) = math::sin_cos(angle + std::f64::consts::FRAC_PI_2);
                let handle = (i.centre.0 + r * c, i.centre.1 + r * s);
                // Its badge above the top left of its box, outside the shapes: a turned
                // ghost's top point may be a far corner, next to a neighbour.
                let label_at = i
                    .outlines
                    .iter()
                    .flat_map(|(points, _)| points)
                    .fold(None, |top: Option<Point>, &p| match top {
                        Some(t) => Some((t.0.min(p.0), t.1.max(p.1))),
                        None => Some(p),
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
                    label: if alone(i.part) {
                        String::new()
                    } else {
                        format!("#{}", i.id)
                    },
                    label_at,
                    selected,
                    enabled: i.enabled,
                    centre: i.centre,
                    handle: selected.then_some(handle),
                    hot: hot == Some((i.part, i.id))
                        || self.dragged_instance() == Some((i.part, i.id)),
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
        if self.scene.hide_drawing || self.display.three_d {
            return None;
        }
        let reach = crate::view::TURN_HANDLE_PX.max(crate::view::PICK_TOLERANCE_PX) + 1.0;
        self.scene.instances.iter().find_map(|ghost| {
            let handle = ghost.handle?;
            (self.view.to_screen(handle).distance(pos) <= reach).then_some((ghost.part, ghost.id))
        })
    }

    /// The instance whose badge (its number, above its ghost) is under `pos`: the handle
    /// that moves it, taken before any shape under it.
    pub(super) fn instance_badge_at(&self, pos: egui::Pos2) -> Option<(u64, u32)> {
        if self.scene.hide_drawing || self.display.three_d {
            return None;
        }
        // The last drawn is on top.
        self.scene
            .instances
            .iter()
            .rev()
            .filter(|ghost| !ghost.label.is_empty())
            .find_map(|ghost| {
                crate::view::badge_rect(&self.view, ghost)
                    .expand(1.0)
                    .contains(pos)
                    .then_some((ghost.part, ghost.id))
            })
    }

    /// The instance a press at `pos` would take: its turn handle, its badge, or its outline
    /// where no shape lies under it (or it is the selected one).
    pub(super) fn instance_grip_at(&self, pos: egui::Pos2) -> Option<(u64, u32)> {
        self.instance_handle_at(pos)
            .or_else(|| self.instance_badge_at(pos))
            .or_else(|| {
                (!self.display.three_d)
                    .then(|| self.instance_at(pos))
                    .flatten()
                    .filter(|i| {
                        self.scene.pick(&self.view, pos).is_none()
                            || self.focus == Focus::Instance(i.0, i.1)
                    })
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
        let (Some(start), Some(centre)) = (self.instance_value(part, id), centre) else {
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
            copy_of: None,
        });
        true
    }

    pub(super) fn dragging_instance(&self) -> bool {
        self.drag.instance.is_some()
    }

    /// The instance being dragged, part and number.
    fn dragged_instance(&self) -> Option<(u64, u32)> {
        self.drag.instance.map(|d| (d.part, d.id))
    }

    /// The dragged instance with the pointer at `at`: moved by as much as the pointer, or
    /// turned about its centre as the pointer goes round it, a quarter turn within 3°
    /// snapping to it (`free`, Alt: as it is). With `shift` a move keeps to 15° steps from
    /// where it was grabbed (docs/specs/canvas-handling.md §1). Returns the label for the
    /// cursor.
    pub(super) fn drag_instance_to(
        &mut self,
        at: Point,
        free: bool,
        shift: bool,
    ) -> Option<String> {
        let drag = self.drag.instance?;
        let units = self.prefs.units;
        let mut instance = self.instance_value(drag.part, drag.id)?;
        let label = match drag.grip {
            InstanceGrip::Move => {
                let at = if shift {
                    crate::draw::angle_step(drag.grab, at, None, units)
                } else {
                    at
                };
                let (dx, dy) = (at.0 - drag.grab.0, at.1 - drag.grab.1);
                instance.at = [drag.start.at[0] + dx, drag.start.at[1] + dy];
                format!(
                    "X {}  Y {}",
                    crate::draw::fixed(instance.at[0], units),
                    crate::draw::fixed(instance.at[1], units)
                )
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
                format!("{:.1}°", angle.to_degrees())
            }
        };
        self.set_instance_value(drag.part, drag.id, instance);
        Some(label)
    }

    /// The drag ends: the instance stays where it was dropped, as one undo step.
    pub(super) fn end_instance_drag(&mut self) {
        let Some(drag) = self.drag.instance.take() else {
            return;
        };
        let now = self.instance_value(drag.part, drag.id);
        if drag.copy_of.is_some() || now.is_some_and(|now| now != drag.start) {
            self.commit();
        }
    }

    /// Escape during a drag: the instance goes back where it was. False when there was no
    /// drag.
    pub(super) fn cancel_instance_drag(&mut self) -> bool {
        let Some(drag) = self.drag.instance.take() else {
            return false;
        };
        if let Some(original) = drag.copy_of {
            // The copy goes; the original is selected again.
            if let Some(edit) = self.edit.as_mut() {
                for setup in &mut edit.job.setups {
                    setup
                        .instances
                        .retain(|i| i.part != drag.part || i.id != drag.id);
                }
            }
            self.focus = Focus::Instance(drag.part, original);
        } else {
            self.set_instance_value(drag.part, drag.id, drag.start);
        }
        true
    }
}

/// An angle in −π…π.
pub(super) fn tidy_angle(angle: f64) -> f64 {
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

impl ChiploadApp {
    /// The items of an instance's right-click menu, on the canvas and in the Browser
    /// (docs/specs/canvas-handling.md §3). Moving it to another setup is the Browser's.
    pub(super) fn instance_menu_items(
        &self,
        ui: &mut egui::Ui,
        part: u64,
        id: u32,
    ) -> Option<super::browser_ops::OpAction> {
        use super::browser_ops::OpAction;
        let mut action = None;
        if ui
            .button(t!(ui, "browser-instance-duplicate"))
            .on_hover_text(t!(ui, "browser-instance-duplicate-hover"))
            .clicked()
        {
            action = Some(OpAction::InstanceDuplicate(part, id));
        }
        if ui.button(t!(ui, "browser-instance-turn")).clicked() {
            action = Some(OpAction::InstanceTurn(part, id, true));
        }
        if ui.button(t!(ui, "browser-instance-turn-right")).clicked() {
            action = Some(OpAction::InstanceTurn(part, id, false));
        }
        if ui.button(t!(ui, "browser-instance-mirror")).clicked() {
            action = Some(OpAction::InstanceMirror(part, id));
        }
        let enabled = self.instance_value(part, id).is_some_and(|i| i.enabled);
        let toggle = if enabled {
            t!(ui, "browser-instance-off")
        } else {
            t!(ui, "browser-instance-on")
        };
        if ui.button(toggle).clicked() {
            action = Some(OpAction::InstanceToggle(part, id));
        }
        action
    }
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
