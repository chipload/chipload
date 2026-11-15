// SPDX-License-Identifier: GPL-3.0-or-later
//! The Compass (docs/specs/compass.md): one handle on the selection, two arrows, a ring and
//! a centre. Locked, a drag moves the selection along an arrow, turns it about the centre,
//! or moves it freely by the centre (§4); unlocked with a tap of Space, a drag places the
//! Compass itself (§5); a right drag asks to move, copy or make an array (§6); a right-click
//! on the centre or an arrow moves it to a point clicked next (§15). What it
//! does to the drawing goes through the same pipeline calls as the box handles and the
//! array tools; the only maths here is where the handle sits on screen.

use chipload_core::Point;
use chipload_core::math;
use chipload_core::primitives::Primitive;
use chipload_pipeline::box_handles::PlacedBox;
use chipload_pipeline::drawing_edit::{Edited, Polar, array_polar};

use super::box_drag::BoxOf;
use super::modify::ModifyTool;
use super::*;

use crate::view::{
    COMPASS_ARM_END_PX as ARM_END_PX, COMPASS_ARM_START_PX as ARM_START_PX,
    COMPASS_CENTRE_PX as CENTRE_PX, COMPASS_RING_PX as RING_PX,
};

/// How near the ring a press takes it, in screen pixels.
const RING_REACH_PX: f32 = 5.0;
/// How near an arrow's line a press takes it, in screen pixels.
const ARM_REACH_PX: f32 = 6.0;
/// The ring turns in these steps; with Alt in the fine ones (degrees).
const TURN_STEP_DEG: f64 = 15.0;
const FINE_TURN_STEP_DEG: f64 = 0.1;
/// A Space press with the pointer moved less than this, button down, is a tap.
const TAP_PX: f32 = 3.0;

/// A part of the Compass.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompassPart {
    X,
    Y,
    Ring,
    Centre,
}

impl CompassPart {
    /// Its index in the scene, for the highlight: X, Y, ring, centre.
    fn index(self) -> usize {
        match self {
            CompassPart::X => 0,
            CompassPart::Y => 1,
            CompassPart::Ring => 2,
            CompassPart::Centre => 3,
        }
    }
}

/// What a drag does: a move by (dx, dy) mm, or a turn by degrees counter-clockwise.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum Motion {
    Move(Point),
    Turn(f64),
}

impl Motion {
    fn is_none(self) -> bool {
        match self {
            Motion::Move((dx, dy)) => dx == 0.0 && dy == 0.0,
            Motion::Turn(degrees) => degrees == 0.0,
        }
    }

    /// The Compass after it: its centre and its axes' angle (degrees).
    fn applied(self, centre: Point, angle: f64) -> (Point, f64) {
        match self {
            Motion::Move((dx, dy)) => ((centre.0 + dx, centre.1 + dy), angle),
            Motion::Turn(degrees) => (centre, angle + degrees),
        }
    }

    /// As `transform_shapes` takes it: from, to, degrees about `to`.
    fn transform(self, centre: Point) -> (Point, Point, f64) {
        match self {
            Motion::Move((dx, dy)) => (centre, (centre.0 + dx, centre.1 + dy), 0.0),
            Motion::Turn(degrees) => (centre, centre, degrees),
        }
    }
}

/// What the Compass moves.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum Target {
    /// Drawn shapes, by id: everything (§2).
    Shapes(Vec<u64>),
    /// A group selected whole: moved and turned with its texts.
    Group(u64),
    /// A whole DXF or SVG, by its placement.
    Dxf(u64),
    /// A picture, text or 3D model, by its box.
    Placed(Source, PlacedBox),
}

/// A drag of the Compass in progress.
#[derive(Debug, Clone)]
pub(super) struct CompassDrag {
    part: CompassPart,
    /// With the right button: ask what to do on release (§6).
    right: bool,
    /// The Compass itself is being placed (§5).
    unlocked: bool,
    /// The Compass when the drag began.
    centre: Point,
    angle: f64,
    /// Where the drag began, mm.
    grab: Point,
    motion: Motion,
    target: Target,
}

/// The menu after a right drag (§6): what it would do, where it shows.
#[derive(Debug, Clone)]
pub(super) struct CompassMenu {
    part: CompassPart,
    motion: Motion,
    centre: Point,
    angle: f64,
    at: egui::Pos2,
    target: Target,
}

/// What the right drag's menu was asked for.
enum Choice {
    Here,
    Copy,
    Array,
    Cancel,
}

/// What a pick takes from the click (§15): the snapped point, or a shape's centre.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PickBy {
    Point,
    Centre,
}

/// *To point* or *To centre point* in progress (§15): the next click picks the point.
#[derive(Debug, Clone)]
pub(super) struct CompassPick {
    part: CompassPart,
    by: PickBy,
    /// The Compass is being placed, not the selection moved.
    unlocked: bool,
    centre: Point,
    angle: f64,
    target: Target,
    /// Where the point under the pointer would take it.
    motion: Motion,
}

/// The Compass's state; nothing of it goes into the project (decision 9).
#[derive(Debug, Clone, Default)]
pub struct Compass {
    shown: bool,
    unlocked: bool,
    /// Where it sits, mm; None: the middle of the selection.
    centre: Option<Point>,
    /// Its X axis's angle, degrees counter-clockwise.
    angle: f64,
    /// The selection it was set up for: another selection starts it afresh.
    selection: Option<CanvasSelection>,
    pub(super) drag: Option<CompassDrag>,
    pub(super) menu: Option<CompassMenu>,
    /// The menu after a right-click on the centre or an arrow (§15): the part, where.
    pick_menu: Option<(CompassPart, egui::Pos2)>,
    /// A point being picked (§15).
    pick: Option<CompassPick>,
    /// Space is held: where the pointer was, and whether a button was down while it moved.
    space: Option<(egui::Pos2, bool)>,
}

impl ChiploadApp {
    /// Q: show the Compass on the selection, or hide it.
    pub fn toggle_compass(&mut self) {
        if self.canvas_tools.compass.shown {
            self.hide_compass();
            return;
        }
        if self.edit.is_none() {
            return;
        }
        self.enter_edit_mode();
        if self.display.three_d {
            self.view_top();
        }
        if self.tool_active() {
            self.end_tool();
        }
        if self.compass_target().is_none() {
            self.set_status(t!(self, "compass-select-first"), STATUS_SECONDS);
            return;
        }
        self.canvas_tools.compass = Compass {
            shown: true,
            selection: Some(self.selection.clone()),
            ..Compass::default()
        };
    }

    /// Hide the Compass; whether it was shown.
    pub fn hide_compass(&mut self) -> bool {
        let shown = self.canvas_tools.compass.shown;
        self.canvas_tools.compass = Compass::default();
        shown
    }

    /// Whether the Compass could show on the selection now (for its menu item and button).
    pub(super) fn can_show_compass(&mut self) -> bool {
        self.edit.is_some() && !self.display.three_d && self.compass_target().is_some()
    }

    /// Whether the Compass shows.
    pub(super) fn compass_shown(&self) -> bool {
        self.canvas_tools.compass.shown
    }

    /// Close the right drag's menu, the right-click's menu or a pick in progress, changing
    /// nothing; whether one was open.
    pub(super) fn close_compass_menu(&mut self) -> bool {
        let compass = &mut self.canvas_tools.compass;
        let open = compass.menu.is_some() || compass.pick_menu.is_some() || compass.pick.is_some();
        compass.menu = None;
        compass.pick_menu = None;
        compass.pick = None;
        open
    }

    /// What the Compass would move, and the middle of it: drawn shapes, a group, a whole
    /// DXF, picture, text or model. None for anything else, or while it can't show.
    fn compass_target(&mut self) -> Option<(Target, Point)> {
        if let CanvasSelection::Shapes(_) = self.selection {
            let (ids, imported) = self.deletable_shapes();
            if imported || ids.is_empty() {
                return None;
            }
            let job = &self.edit.as_ref()?.job;
            let ((x0, y0), (x1, y1)) = chipload_pipeline::shapes::drawn_box(job, &ids)?;
            return Some((Target::Shapes(ids), ((x0 + x1) / 2.0, (y0 + y1) / 2.0)));
        }
        let (of, placed) = self.selected_box()?;
        let target = match of {
            BoxOf::Shapes(ids) => Target::Shapes(ids),
            BoxOf::Whole(Source::Group(id)) => Target::Group(id),
            BoxOf::Whole(Source::Dxf(id)) => Target::Dxf(id),
            BoxOf::Whole(source) => Target::Placed(source, placed),
        };
        Some((target, placed.center()))
    }

    /// The Compass as it shows now: its centre, its axes' angle and whether it is unlocked;
    /// with a drag or the right drag's menu, where they have it. None while hidden, while
    /// a tool, the 3D view or Simulate is on, or when the selection can't be moved. A new
    /// selection starts it afresh; an empty one hides it.
    pub(super) fn compass_now(&mut self) -> Option<(Point, f64, bool)> {
        if !self.canvas_tools.compass.shown {
            return None;
        }
        if self.canvas_tools.compass.selection.as_ref() != Some(&self.selection) {
            if self.selection == CanvasSelection::Nothing {
                self.hide_compass();
                return None;
            }
            self.canvas_tools.compass = Compass {
                shown: true,
                selection: Some(self.selection.clone()),
                ..Compass::default()
            };
        }
        if self.display.simulate || self.display.three_d || self.tool_active() {
            return None;
        }
        let compass = &self.canvas_tools.compass;
        let unlocked = compass.unlocked;
        if let Some(drag) = &compass.drag {
            let (centre, angle) = drag.motion.applied(drag.centre, drag.angle);
            return Some((centre, angle, drag.unlocked));
        }
        if let Some(menu) = &compass.menu {
            let (centre, angle) = menu.motion.applied(menu.centre, menu.angle);
            return Some((centre, angle, false));
        }
        if let Some(pick) = &compass.pick {
            let (centre, angle) = pick.motion.applied(pick.centre, pick.angle);
            return Some((centre, angle, pick.unlocked));
        }
        let (stored, angle) = (compass.centre, compass.angle);
        let (_, middle) = self.compass_target()?;
        Some((stored.unwrap_or(middle), angle, unlocked))
    }

    /// The part of the Compass at `pos` on screen, if it shows.
    pub(super) fn compass_part_at(&mut self, pos: egui::Pos2) -> Option<CompassPart> {
        let (centre, angle, _) = self.compass_now()?;
        part_at(self.view.to_screen(centre), angle, pos)
    }

    /// The Compass for the scene, with the part under the pointer at `hover` highlighted.
    pub(super) fn compass_shape(
        &mut self,
        hover: Option<egui::Pos2>,
    ) -> Option<crate::view::CompassShape> {
        let (centre, angle, unlocked) = self.compass_now()?;
        let dragged = self.canvas_tools.compass.drag.as_ref().map(|d| d.part);
        let hot = dragged
            .or_else(|| hover.and_then(|pos| part_at(self.view.to_screen(centre), angle, pos)))
            .map(CompassPart::index);
        Some(crate::view::CompassShape {
            centre,
            angle_deg: angle,
            unlocked,
            hot,
        })
    }

    /// A drag begins on `part` at `origin` on screen; `right`: with the right button, which
    /// only a locked Compass takes. Whether it began.
    pub(super) fn start_compass_drag(
        &mut self,
        part: CompassPart,
        origin: egui::Pos2,
        right: bool,
    ) -> bool {
        let Some((centre, angle, unlocked)) = self.compass_now() else {
            return false;
        };
        if right && unlocked {
            return false;
        }
        let Some((target, _)) = self.compass_target() else {
            return false;
        };
        self.canvas_tools.compass.menu = None;
        self.canvas_tools.compass.drag = Some(CompassDrag {
            part,
            right,
            unlocked,
            centre,
            angle,
            grab: self.view.to_world(origin),
            motion: match part {
                CompassPart::Ring => Motion::Turn(0.0),
                _ => Motion::Move((0.0, 0.0)),
            },
            target,
        });
        true
    }

    /// Whether the Compass is being dragged.
    pub(super) fn dragging_compass(&self) -> bool {
        self.canvas_tools.compass.drag.is_some()
    }

    /// The drag with the cursor at `cursor` (mm); `free`: Alt, no snapping.
    pub(super) fn drag_compass_to(&mut self, cursor: Point, free: bool) {
        let Some(drag) = self.canvas_tools.compass.drag.clone() else {
            return;
        };
        let centre = drag.centre;
        let motion = match (drag.part, drag.unlocked) {
            // Free, the centre on a snap point (§4); unlocked, the selection's own points
            // count too, so it can sit on one of its corners (§5).
            (CompassPart::Centre, unlocked) => {
                let skip = if unlocked {
                    BTreeSet::new()
                } else {
                    self.own_contours(&drag.target)
                };
                let (p, _) = self.snap_skipping(cursor, free, &skip);
                Motion::Move((p.0 - centre.0, p.1 - centre.1))
            }
            // Along the arrow: to a snap point's projection, else in the grid's steps.
            (part @ (CompassPart::X | CompassPart::Y), false) => {
                let u = axis(drag.angle, part);
                let skip = self.own_contours(&drag.target);
                let (p, hit) = self.snap_skipping(cursor, free, &skip);
                let along = if hit {
                    dot((p.0 - centre.0, p.1 - centre.1), u)
                } else {
                    self.scene.snap_mark = None;
                    let raw = dot((cursor.0 - drag.grab.0, cursor.1 - drag.grab.1), u);
                    if free {
                        raw
                    } else {
                        let step = grid_step(self.view.scale, self.prefs.units);
                        (raw / step).round() * step
                    }
                };
                Motion::Move((u.0 * along, u.1 * along))
            }
            (CompassPart::Ring, _) => {
                self.scene.snap_mark = None;
                let turned = degrees_of((cursor.0 - centre.0, cursor.1 - centre.1))
                    - degrees_of((drag.grab.0 - centre.0, drag.grab.1 - centre.1));
                Motion::Turn(stepped(normalised(turned), free))
            }
            // Unlocked, an arrow points at the cursor, or along a line under it (§5).
            (part @ (CompassPart::X | CompassPart::Y), true) => {
                self.scene.snap_mark = None;
                let offset = if part == CompassPart::Y { 90.0 } else { 0.0 };
                let raw = degrees_of((cursor.0 - centre.0, cursor.1 - centre.1)) - offset;
                let to = match self.line_direction_near(cursor) {
                    Some(line) => nearest_turn(line, raw),
                    None => stepped(raw, free),
                };
                Motion::Turn(normalised(to - drag.angle))
            }
        };
        if let Some(drag) = self.canvas_tools.compass.drag.as_mut() {
            drag.motion = motion;
        }
    }

    /// The drag ends with the pointer at `at`: unlocked, the Compass stays where it was
    /// put; locked, the selection is moved or turned as one undo step, or after a right
    /// drag the menu opens.
    pub(super) fn end_compass_drag(&mut self, at: Option<egui::Pos2>) {
        let Some(drag) = self.canvas_tools.compass.drag.take() else {
            return;
        };
        self.scene.snap_mark = None;
        if drag.motion.is_none() {
            return;
        }
        if drag.unlocked {
            let (centre, angle) = drag.motion.applied(drag.centre, drag.angle);
            let compass = &mut self.canvas_tools.compass;
            compass.centre = Some(centre);
            compass.angle = angle;
            return;
        }
        if drag.right {
            self.canvas_tools.compass.menu = Some(CompassMenu {
                part: drag.part,
                motion: drag.motion,
                centre: drag.centre,
                angle: drag.angle,
                at: at.unwrap_or(egui::Pos2::ZERO),
                target: drag.target,
            });
            return;
        }
        self.compass_apply(&drag.target, drag.centre, drag.angle, drag.motion);
    }

    /// Move or turn `target` by `motion` from the Compass at `centre`, `angle`, as one
    /// undo step; the Compass goes with it.
    fn compass_apply(&mut self, target: &Target, centre: Point, angle: f64, motion: Motion) {
        let (from, to, degrees) = motion.transform(centre);
        let done = match target {
            Target::Shapes(ids) => self.transform_shapes(ids, from, to, 1.0, degrees),
            Target::Group(id) => self.transform_group(*id, from, to, 1.0, degrees),
            Target::Dxf(id) => self.drag_dxf(*id, from, to, 1.0, degrees),
            Target::Placed(source, start) => {
                let now = moved_box(start, from, to, degrees);
                self.place_whole(*source, start, &now, true);
                true
            }
        };
        if done {
            self.compass_follows(motion, centre, angle);
        }
    }

    /// The Compass after `motion` from `centre`, `angle`, on the selection as it now is.
    fn compass_follows(&mut self, motion: Motion, centre: Point, angle: f64) {
        let (centre, angle) = motion.applied(centre, angle);
        let selection = self.selection.clone();
        let compass = &mut self.canvas_tools.compass;
        compass.centre = Some(centre);
        compass.angle = angle;
        compass.selection = Some(selection);
    }

    /// The contours of what the Compass moves, left out of snapping so it never snaps to
    /// itself.
    fn own_contours(&self, target: &Target) -> BTreeSet<usize> {
        let ids: BTreeSet<u64> = match target {
            Target::Shapes(ids) => ids.iter().copied().collect(),
            Target::Group(id) => self
                .edit
                .as_ref()
                .map(|e| e.job.drawing.group_shapes(*id).into_iter().collect())
                .unwrap_or_default(),
            Target::Dxf(id) => self.dxf_shapes(*id).into_iter().collect(),
            Target::Placed(..) => return self.selected_ids(),
        };
        self.contours
            .iter()
            .filter(|c| !shapes_of(c).is_disjoint(&ids))
            .map(|c| c.id)
            .collect()
    }

    /// The direction of a straight line within reach of `at`, degrees.
    fn line_direction_near(&self, at: Point) -> Option<f64> {
        let reach = f64::from(crate::view::PICK_TOLERANCE_PX) / self.view.scale;
        self.contours
            .iter()
            .flat_map(|c| &c.parts)
            .filter_map(|p| match p.segment.prim {
                Primitive::Line(line) => Some((line.start, line.end)),
                Primitive::Arc(_) => None,
            })
            .map(|(a, b)| (to_segment(at, a, b), degrees_of((b.0 - a.0, b.1 - a.1))))
            .filter(|(d, _)| *d <= reach)
            .min_by(|x, y| x.0.total_cmp(&y.0))
            .map(|(_, direction)| direction)
    }

    /// The outlines of what is moved, where the drag or the open menu has it.
    pub(super) fn compass_outlines(&self) -> Vec<Vec<Point>> {
        let compass = &self.canvas_tools.compass;
        let (target, centre, motion) = match (&compass.drag, &compass.menu, &compass.pick) {
            (Some(drag), _, _) if !drag.unlocked => (&drag.target, drag.centre, drag.motion),
            (None, Some(menu), _) => (&menu.target, menu.centre, menu.motion),
            (None, None, Some(pick)) if !pick.unlocked => (&pick.target, pick.centre, pick.motion),
            _ => return Vec::new(),
        };
        if motion.is_none() {
            return Vec::new();
        }
        let (from, to, degrees) = motion.transform(centre);
        match target {
            Target::Shapes(ids) => self.moved_outlines(ids, from, to, 1.0, degrees),
            Target::Group(id) => {
                let ids = self
                    .edit
                    .as_ref()
                    .map(|e| e.job.drawing.group_shapes(*id))
                    .unwrap_or_default();
                self.moved_outlines(&ids, from, to, 1.0, degrees)
            }
            Target::Dxf(id) => self.moved_outlines(&self.dxf_shapes(*id), from, to, 1.0, degrees),
            Target::Placed(_, start) => {
                let c = moved_box(start, from, to, degrees).corners();
                vec![vec![c[0], c[1], c[2], c[3], c[0]]]
            }
        }
    }

    /// The value a locked drag has, for the label at the cursor.
    pub(super) fn compass_readout(&self) -> Option<String> {
        let drag = self
            .canvas_tools
            .compass
            .drag
            .as_ref()
            .filter(|d| !d.unlocked)?;
        let units = self.prefs.units;
        Some(match (drag.part, drag.motion) {
            (_, Motion::Turn(degrees)) => format!("{degrees:.1}°"),
            (part @ (CompassPart::X | CompassPart::Y), Motion::Move(d)) => {
                let along = dot(d, axis(drag.angle, part));
                let name = if part == CompassPart::X { "X" } else { "Y" };
                format!("{name} {}", crate::draw::fixed(along, units))
            }
            (_, Motion::Move((dx, dy))) => format!(
                "ΔX {}  ΔY {}",
                crate::draw::fixed(dx, units),
                crate::draw::fixed(dy, units)
            ),
        })
    }

    /// A double-click on `part` at `pos`: an entry for its exact value (§4).
    pub(super) fn start_typing_compass(&mut self, part: CompassPart, pos: egui::Pos2) {
        if self.compass_now().is_none_or(|(_, _, unlocked)| unlocked) {
            return;
        }
        self.canvas_tools.typed = Some(super::drawing::Typed {
            text: String::new(),
            at: pos + egui::vec2(16.0, 16.0),
            refused: false,
            focus: true,
            target: super::drawing::TypedTarget::Compass(part),
        });
    }

    /// Enter in the Compass's entry: a distance along an arrow, an angle for the ring, or
    /// "ΔX ΔY" for the centre. A value that can't be read leaves the entry open, in red.
    pub(super) fn apply_typed_compass(&mut self, part: CompassPart) {
        let text = self
            .canvas_tools
            .typed
            .as_ref()
            .map(|t| t.text.clone())
            .unwrap_or_default();
        let comma = crate::i18n::Lang::lang(self).decimal_comma();
        let units = self.prefs.units;
        let length = |s: &str| {
            chipload_pipeline::units::parse_length(&crate::units::decimal_point(s, comma), units)
        };
        let target = self.compass_target();
        let (Some((target, _)), Some((centre, angle, _))) = (target, self.compass_now()) else {
            self.canvas_tools.typed = None;
            return;
        };
        let motion = match part {
            CompassPart::X | CompassPart::Y => length(text.trim()).map(|d| {
                let u = axis(angle, part);
                Motion::Move((u.0 * d, u.1 * d))
            }),
            CompassPart::Ring => crate::units::decimal_point(text.trim(), comma)
                .trim_end_matches('°')
                .parse::<f64>()
                .ok()
                .filter(|d| d.is_finite())
                .map(Motion::Turn),
            CompassPart::Centre => {
                let parts: Vec<&str> = text
                    .split(|c: char| c == ';' || c.is_whitespace() || (c == ',' && !comma))
                    .filter(|s| !s.is_empty())
                    .collect();
                match parts[..] {
                    [x, y] => length(x).zip(length(y)).map(Motion::Move),
                    _ => None,
                }
            }
        };
        match motion {
            Some(motion) => {
                self.canvas_tools.typed = None;
                if !motion.is_none() {
                    self.compass_apply(&target, centre, angle, motion);
                }
            }
            None => {
                if let Some(typed) = self.canvas_tools.typed.as_mut() {
                    typed.refused = true;
                    typed.focus = true;
                }
                self.set_status(t!(self, "compass-typed-unreadable"), STATUS_SECONDS);
            }
        }
    }

    /// Space while the Compass shows (§5): a tap, pressed and let go without dragging,
    /// unlocks or locks it; held while dragging it pans, as everywhere.
    pub(super) fn compass_space(&mut self, ctx: &egui::Context) {
        if !self.canvas_tools.compass.shown || ctx.egui_wants_keyboard_input() {
            self.canvas_tools.compass.space = None;
            return;
        }
        let (pressed, released, pos, down) = ctx.input(|i| {
            (
                i.key_pressed(Key::Space),
                i.key_released(Key::Space),
                i.pointer.latest_pos().unwrap_or_default(),
                i.pointer.any_down(),
            )
        });
        let compass = &mut self.canvas_tools.compass;
        if pressed && compass.space.is_none() {
            compass.space = Some((pos, false));
        }
        if let Some((at, dragged)) = compass.space.as_mut()
            && down
            && at.distance(pos) > TAP_PX
        {
            *dragged = true;
        }
        if released && let Some((_, dragged)) = compass.space.take() {
            if dragged || compass.drag.is_some() {
                return;
            }
            compass.unlocked = !compass.unlocked;
            let text = if compass.unlocked {
                t!(self, "compass-unlocked")
            } else {
                t!(self, "compass-locked")
            };
            self.set_status(text, STATUS_SECONDS);
        }
    }

    /// The menu after a right drag (§6), at the pointer: move or turn here, copy here, an
    /// array, or cancel. A click elsewhere cancels.
    pub(super) fn compass_menu(&mut self, ctx: &egui::Context) {
        let Some(menu) = self.canvas_tools.compass.menu.clone() else {
            return;
        };
        let copies = matches!(menu.target, Target::Shapes(_));
        let turn = menu.part == CompassPart::Ring;
        let mut choice = None;
        let area = egui::Area::new(egui::Id::new("compass-menu"))
            .fixed_pos(menu.at)
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                egui::Frame::menu(ui.style()).show(ui, |ui| {
                    let here = if turn {
                        t!(ui, "compass-turn-here")
                    } else {
                        t!(ui, "compass-move-here")
                    };
                    if ui.button(here).clicked() {
                        choice = Some(Choice::Here);
                    }
                    let why = t!(ui, "compass-copies-drawn-only");
                    if ui
                        .add_enabled(copies, egui::Button::new(t!(ui, "compass-copy-here")))
                        .on_disabled_hover_text(&why)
                        .clicked()
                    {
                        choice = Some(Choice::Copy);
                    }
                    let array = if turn {
                        t!(ui, "compass-circular-array")
                    } else {
                        t!(ui, "compass-linear-array")
                    };
                    if ui
                        .add_enabled(copies, egui::Button::new(array))
                        .on_disabled_hover_text(&why)
                        .clicked()
                    {
                        choice = Some(Choice::Array);
                    }
                    ui.separator();
                    if ui.button(t!(ui, "common-cancel")).clicked() {
                        choice = Some(Choice::Cancel);
                    }
                });
            });
        if choice.is_none() && area.response.clicked_elsewhere() {
            choice = Some(Choice::Cancel);
        }
        let Some(choice) = choice else {
            return;
        };
        self.canvas_tools.compass.menu = None;
        match choice {
            Choice::Here => self.compass_apply(&menu.target, menu.centre, menu.angle, menu.motion),
            Choice::Copy => self.compass_copy(&menu),
            Choice::Array => self.compass_array(&menu),
            Choice::Cancel => {}
        }
    }

    /// *Copy here*: a copy where the drag ended, as one undo step; the copy becomes the
    /// selection, the Compass on it.
    fn compass_copy(&mut self, menu: &CompassMenu) {
        let Target::Shapes(ids) = &menu.target else {
            return;
        };
        let Some(job) = self.job_with_operations() else {
            return;
        };
        let result = match menu.motion {
            Motion::Move(by) => {
                chipload_pipeline::shapes::copy_shapes(&job, ids, by).map(|(job, new_ids)| Edited {
                    job,
                    new_ids,
                    notes: Vec::new(),
                    joined: None,
                })
            }
            Motion::Turn(degrees) => array_polar(
                &job,
                ids,
                Polar {
                    count: 2,
                    center: menu.centre,
                    angle_deg: degrees,
                    rotate: true,
                },
            ),
        };
        if self.take_edit(result, &[]) {
            self.compass_follows(menu.motion, menu.centre, menu.angle);
        }
    }

    /// *Linear array…* or *Circular array…*: the array tool's form, filled from the drag
    /// (two at first, the dragged step or angle, the Compass's centre).
    fn compass_array(&mut self, menu: &CompassMenu) {
        let array = &mut self.canvas_tools.array;
        let tool = match menu.motion {
            Motion::Move(step) => {
                array.line_count = 2;
                array.step = step;
                ModifyTool::LinearArray
            }
            Motion::Turn(degrees) => {
                array.count = 2;
                array.center = menu.centre;
                array.angle_deg = degrees;
                array.rotate = true;
                ModifyTool::PolarArray
            }
        };
        self.start_modify(tool);
    }

    /// A right-click on `part` at `at` on screen: the centre or an arrow opens the menu of
    /// §15 (the ring has none). Whether it opened, so the canvas's own menu stays shut.
    pub(super) fn open_compass_pick_menu(&mut self, part: CompassPart, at: egui::Pos2) -> bool {
        if part == CompassPart::Ring || self.compass_now().is_none() {
            return false;
        }
        let compass = &mut self.canvas_tools.compass;
        compass.menu = None;
        compass.pick = None;
        compass.pick_menu = Some((part, at));
        true
    }

    /// Whether the right-click's menu is open.
    pub(super) fn compass_pick_menu_open(&self) -> bool {
        self.canvas_tools.compass.pick_menu.is_some()
    }

    /// The right-click's menu (§15): *To point*, *To centre point*, or cancel. A click
    /// elsewhere cancels.
    pub(super) fn compass_pick_menu(&mut self, ctx: &egui::Context) {
        let Some((part, at)) = self.canvas_tools.compass.pick_menu else {
            return;
        };
        let mut choice: Option<Option<PickBy>> = None;
        let area = egui::Area::new(egui::Id::new("compass-pick-menu"))
            .fixed_pos(at)
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                egui::Frame::menu(ui.style()).show(ui, |ui| {
                    if ui
                        .button(t!(ui, "compass-to-point"))
                        .on_hover_text(t!(ui, "compass-to-point-hover"))
                        .clicked()
                    {
                        choice = Some(Some(PickBy::Point));
                    }
                    if ui
                        .button(t!(ui, "compass-to-centre"))
                        .on_hover_text(t!(ui, "compass-to-centre-hover"))
                        .clicked()
                    {
                        choice = Some(Some(PickBy::Centre));
                    }
                    ui.separator();
                    if ui.button(t!(ui, "common-cancel")).clicked() {
                        choice = Some(None);
                    }
                });
            });
        if choice.is_none() && area.response.clicked_elsewhere() {
            choice = Some(None);
        }
        let Some(choice) = choice else {
            return;
        };
        self.canvas_tools.compass.pick_menu = None;
        if let Some(by) = choice {
            self.start_compass_pick(part, by);
        }
    }

    /// *To point* or *To centre point* on `part`: the next click picks the point.
    pub(super) fn start_compass_pick(&mut self, part: CompassPart, by: PickBy) {
        let Some((centre, angle, unlocked)) = self.compass_now() else {
            return;
        };
        let Some((target, _)) = self.compass_target() else {
            return;
        };
        self.canvas_tools.compass.pick = Some(CompassPick {
            part,
            by,
            unlocked,
            centre,
            angle,
            target,
            motion: Motion::Move((0.0, 0.0)),
        });
    }

    /// Whether a point is being picked.
    pub(super) fn compass_picking(&self) -> bool {
        self.canvas_tools.compass.pick.is_some()
    }

    /// The point a pick takes at `pos` on screen: the snapped point (Alt: none), or the
    /// centre of the shape there (an arc's or a circle's centre, else its box's middle).
    fn picked_point(&mut self, pick: &CompassPick, pos: egui::Pos2, free: bool) -> Option<Point> {
        match pick.by {
            PickBy::Point => {
                // Locked, the selection's own points are left out, as in a drag; unlocked,
                // the Compass can go onto one of them.
                let skip = if pick.unlocked {
                    BTreeSet::new()
                } else {
                    self.own_contours(&pick.target)
                };
                Some(self.snap_skipping(self.view.to_world(pos), free, &skip).0)
            }
            PickBy::Centre => {
                let id = self.scene.pick(&self.view, pos)?;
                let centre = self
                    .contours
                    .iter()
                    .find(|c| c.id == id)
                    .map(chipload_pipeline::selection::contour_centre)?;
                self.scene.snap_mark = Some(centre);
                Some(centre)
            }
        }
    }

    /// Where `point` takes the Compass of `pick`: the centre there in X and Y, an arrow
    /// along its axis only (to where the point lies across it). Locked, the selection goes
    /// with it; unlocked, the Compass alone moves, the same way.
    fn pick_motion(pick: &CompassPick, point: Point) -> Motion {
        let d = (point.0 - pick.centre.0, point.1 - pick.centre.1);
        match pick.part {
            CompassPart::Centre => Motion::Move(d),
            part @ (CompassPart::X | CompassPart::Y) => {
                let u = axis(pick.angle, part);
                let along = dot(d, u);
                Motion::Move((u.0 * along, u.1 * along))
            }
            CompassPart::Ring => Motion::Move((0.0, 0.0)),
        }
    }

    /// The pointer at `pos` while picking: the preview follows the point there.
    pub(super) fn hover_compass_pick(&mut self, pos: egui::Pos2, free: bool) {
        let Some(pick) = self.canvas_tools.compass.pick.clone() else {
            return;
        };
        let motion = match self.picked_point(&pick, pos, free) {
            Some(point) => Self::pick_motion(&pick, point),
            None => Motion::Move((0.0, 0.0)),
        };
        if let Some(p) = self.canvas_tools.compass.pick.as_mut() {
            p.motion = motion;
        }
    }

    /// A click at `pos` while picking: the move (one undo step) or, unlocked, the Compass
    /// alone moved there. *To centre point* off any shape says so and waits for another click.
    pub(super) fn click_compass_pick(&mut self, pos: egui::Pos2, free: bool) {
        let Some(pick) = self.canvas_tools.compass.pick.clone() else {
            return;
        };
        let Some(point) = self.picked_point(&pick, pos, free) else {
            self.set_status(t!(self, "compass-no-centre-here"), STATUS_SECONDS);
            return;
        };
        self.scene.snap_mark = None;
        self.canvas_tools.compass.pick = None;
        let motion = Self::pick_motion(&pick, point);
        if motion.is_none() {
            return;
        }
        if pick.unlocked {
            let (centre, angle) = motion.applied(pick.centre, pick.angle);
            let compass = &mut self.canvas_tools.compass;
            compass.centre = Some(centre);
            compass.angle = angle;
            return;
        }
        self.compass_apply(&pick.target, pick.centre, pick.angle, motion);
    }

    /// The status bar while the Compass shows: what a drag on the part under the pointer
    /// at `hover` does.
    pub(super) fn compass_status(&mut self, hover: Option<egui::Pos2>) -> Option<String> {
        if let Some(pick) = &self.canvas_tools.compass.pick {
            return Some(match (pick.by, pick.part) {
                (PickBy::Point, CompassPart::Centre) => t!(self, "compass-status-pick-point"),
                (PickBy::Point, _) => t!(self, "compass-status-pick-point-axis"),
                (PickBy::Centre, CompassPart::Centre) => t!(self, "compass-status-pick-centre"),
                (PickBy::Centre, _) => t!(self, "compass-status-pick-centre-axis"),
            });
        }
        let (centre, angle, unlocked) = self.compass_now()?;
        let part = hover.and_then(|pos| part_at(self.view.to_screen(centre), angle, pos));
        Some(match (unlocked, part) {
            (true, Some(CompassPart::Centre)) => t!(self, "compass-status-place"),
            (true, Some(_)) => t!(self, "compass-status-aim"),
            (true, None) => t!(self, "compass-unlocked"),
            (false, Some(CompassPart::X | CompassPart::Y)) => t!(self, "compass-status-arrow"),
            (false, Some(CompassPart::Ring)) => t!(self, "compass-status-ring"),
            (false, Some(CompassPart::Centre)) => t!(self, "compass-status-centre"),
            (false, None) => t!(self, "compass-status"),
        })
    }
}

/// The part of a Compass drawn at `centre` on screen, its axes at `angle_deg`, at `pos`.
fn part_at(centre: egui::Pos2, angle_deg: f64, pos: egui::Pos2) -> Option<CompassPart> {
    let d = pos - centre;
    if d.x.abs() <= CENTRE_PX + 2.0 && d.y.abs() <= CENTRE_PX + 2.0 {
        return Some(CompassPart::Centre);
    }
    if (d.length() - RING_PX).abs() <= RING_REACH_PX {
        return Some(CompassPart::Ring);
    }
    for (part, degrees) in [
        (CompassPart::X, angle_deg),
        (CompassPart::Y, angle_deg + 90.0),
    ] {
        let u = screen_axis(degrees);
        let along = d.dot(u);
        let across = (d.x * u.y - d.y * u.x).abs();
        if (ARM_START_PX - 4.0..=ARM_END_PX + 8.0).contains(&along) && across <= ARM_REACH_PX {
            return Some(part);
        }
    }
    None
}

/// The direction `degrees` counter-clockwise on screen, where Y points down.
pub(super) fn screen_axis(degrees: f64) -> egui::Vec2 {
    let (sin, cos) = math::sin_cos(degrees.to_radians());
    egui::vec2(cos as f32, -(sin as f32))
}

/// The unit vector of `part`'s arrow with the axes at `angle` degrees, in drawing axes.
fn axis(angle: f64, part: CompassPart) -> Point {
    let degrees = if part == CompassPart::Y {
        angle + 90.0
    } else {
        angle
    };
    let (sin, cos) = math::sin_cos(degrees.to_radians());
    (cos, sin)
}

fn dot(a: Point, b: Point) -> f64 {
    a.0 * b.0 + a.1 * b.1
}

/// The direction of `v`, degrees counter-clockwise from X.
fn degrees_of(v: Point) -> f64 {
    math::atan2(v.1, v.0).to_degrees()
}

/// `degrees` brought within (−180, 180].
fn normalised(degrees: f64) -> f64 {
    let mut d = degrees % 360.0;
    if d <= -180.0 {
        d += 360.0;
    }
    if d > 180.0 {
        d -= 360.0;
    }
    d
}

/// `degrees` in the ring's steps: 15°, or 0.1° when `free`.
fn stepped(degrees: f64, free: bool) -> f64 {
    let step = if free {
        FINE_TURN_STEP_DEG
    } else {
        TURN_STEP_DEG
    };
    (degrees / step).round() * step
}

/// Of the directions along a line at `line` degrees (it and its opposite), the one nearest
/// `raw`.
fn nearest_turn(line: f64, raw: f64) -> f64 {
    let forward = raw + normalised(line - raw);
    let backward = raw + normalised(line + 180.0 - raw);
    if (forward - raw).abs() <= (backward - raw).abs() {
        forward
    } else {
        backward
    }
}

/// The distance from `p` to the segment from `a` to `b`.
fn to_segment(p: Point, a: Point, b: Point) -> f64 {
    let ab = (b.0 - a.0, b.1 - a.1);
    let length2 = dot(ab, ab);
    let t = if length2 > 0.0 {
        (dot((p.0 - a.0, p.1 - a.1), ab) / length2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    math::hypot(p.0 - (a.0 + t * ab.0), p.1 - (a.1 + t * ab.1))
}

/// Box `start` moved and turned as `transform_shapes` moves points: its centre goes to
/// `to + turn(centre − from)`, and it turns by `degrees`.
fn moved_box(start: &PlacedBox, from: Point, to: Point, degrees: f64) -> PlacedBox {
    let c = start.center();
    let moved = chipload_job::edit::transformed_point(
        [c.0, c.1],
        [from.0, from.1],
        [to.0, to.1],
        1.0,
        degrees,
    );
    PlacedBox {
        corner: (moved[0] - start.size.0 / 2.0, moved[1] - start.size.1 / 2.0),
        size: start.size,
        rotation: start.rotation + degrees,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_parts_are_found_where_they_are_drawn() {
        let c = egui::pos2(100.0, 100.0);
        assert_eq!(part_at(c, 0.0, c), Some(CompassPart::Centre));
        assert_eq!(
            part_at(c, 0.0, c + egui::vec2(RING_PX, 0.0)),
            Some(CompassPart::Ring)
        );
        assert_eq!(
            part_at(c, 0.0, c + egui::vec2(50.0, 0.0)),
            Some(CompassPart::X)
        );
        // Y up on screen is minus.
        assert_eq!(
            part_at(c, 0.0, c + egui::vec2(0.0, -50.0)),
            Some(CompassPart::Y)
        );
        // Turned 90°: X points up.
        assert_eq!(
            part_at(c, 90.0, c + egui::vec2(0.0, -50.0)),
            Some(CompassPart::X)
        );
        assert_eq!(part_at(c, 0.0, c + egui::vec2(-50.0, 0.0)), None);
    }

    #[test]
    fn turns_step_and_stay_within_half_a_turn() {
        assert_eq!(normalised(190.0), -170.0);
        assert_eq!(normalised(-190.0), 170.0);
        assert_eq!(stepped(22.0, false), 15.0);
        assert!((stepped(22.04, true) - 22.0).abs() < 1e-9);
        // A line at 10° seen from 170° is the line's other way.
        assert!((nearest_turn(10.0, 170.0) - 190.0).abs() < 1e-9);
        assert!((nearest_turn(10.0, 20.0) - 10.0).abs() < 1e-9);
    }

    #[test]
    fn a_turned_box_goes_round_the_centre() {
        let start = PlacedBox {
            corner: (10.0, -5.0),
            size: (20.0, 10.0),
            rotation: 0.0,
        };
        let now = moved_box(&start, (0.0, 0.0), (0.0, 0.0), 90.0);
        let c = now.center();
        assert!(
            (c.0 - 0.0).abs() < 1e-9 && (c.1 - 20.0).abs() < 1e-9,
            "{c:?}"
        );
        assert_eq!(now.rotation, 90.0);
    }
}
