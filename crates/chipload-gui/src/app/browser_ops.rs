// SPDX-License-Identifier: GPL-3.0-or-later
//! The Browser's setups, parts, instances, operations and their passes.

use eframe::egui::{Color32, Id, Stroke};

use super::browser::{
    Leaf, ToolPick, can_add, cannot_add, kind_color, new_operations, op_icon, tool_change,
    tool_menu,
};
use super::browser_row::{INDENT, PAD, Row, folder_open, toggle_folder};
use super::shortcuts::with_keys;
use super::*;
use crate::i18n::Lang;
use crate::icons::Icon;

/// What a part's rows need besides the part.
/// What a leaf under an operation needs from it: whether the operation is cut, its kind's
/// colour, and the leaf's depth.
#[derive(Clone, Copy)]
struct Under {
    cut: bool,
    color: Color32,
    depth: usize,
}

pub(super) struct PartRows<'a> {
    /// Its place among the parts.
    pub(super) k: usize,
    /// The part row's depth in the tree: under its setup, or under the project when the
    /// project has one board (docs/specs/parts-and-setups.md §2a). Its operations and
    /// instances are one deeper.
    pub(super) depth: usize,
    pub(super) parts: &'a [chipload_job::Part],
    pub(super) active: Option<u64>,
    pub(super) op_parts: &'a [u64],
    /// Which operations are board surfaces, listed under their setup, not their part
    /// (docs/specs/surface-levels.md §5).
    pub(super) board: &'a [bool],
    pub(super) count: usize,
    /// The tool changes shown above an operation's row ([`tool_changes`]).
    pub(super) tool_before: &'a std::collections::BTreeMap<usize, Vec<i64>>,
}

/// How an instance's row is drawn: whether its setup is the active one, its depth, named
/// with its part (under its setup), and the setups it can be moved to.
#[derive(Clone, Copy)]
pub(super) struct InstanceLook<'a> {
    pub(super) in_active_setup: bool,
    pub(super) depth: usize,
    pub(super) named: bool,
    pub(super) setups: &'a [(u64, String)],
}

/// A part's row dragged in the Browser: dropped on a setup, the part is placed there.
#[derive(Clone, Copy)]
pub(super) struct DraggedPart(pub u64);

/// An instance's row dragged in the Browser: dropped on a setup, it is cut there.
#[derive(Clone, Copy)]
pub(super) struct DraggedInstance(pub u64, pub u32);

/// What a click in the operation rows asks for, done after they are drawn.
pub(super) enum OpAction {
    Focus(usize),
    Toggle(usize),
    Move(usize, usize),
    Duplicate(usize),
    /// *Save as style…* (docs/specs/styles.md §3.1).
    SaveStyle(usize),
    Delete(usize),
    /// A rest pass for pocket i with this tool (docs/specs/rest-passes.md §6).
    AddRest(usize, i64),
    /// The same with a library tool, by name, copied into the project first.
    AddRestFromLibrary(usize, String),
    /// Pocket i's rest pass k: open it, turn it on or off, remove it.
    PassFocus(usize, usize),
    PassToggle(usize, usize),
    PassRemove(usize, usize),
    /// Rest pocket i becomes a rest pass of the pocket it copies.
    MakePass(usize),
    /// A floor pass for V-carve i with this tool, or a library tool by name
    /// (docs/specs/vcarve-wide-strokes.md §7); its floor pass: open, on or off, remove.
    AddFloor(usize, i64),
    AddFloorFromLibrary(usize, String),
    FloorFocus(usize),
    FloorToggle(usize),
    FloorRemove(usize),
    /// A roughing pass for relief i with this tool, or a library tool by name
    /// (docs/specs/stl-relief.md §14); its roughing pass: open, on or off, remove.
    AddRoughing(usize, i64),
    AddRoughingFromLibrary(usize, String),
    RoughingFocus(usize),
    RoughingToggle(usize),
    RoughingRemove(usize),
    /// A smoothing pass for relief i, with its roughing tool.
    AddSmoothing(usize),
    SmoothingFocus(usize),
    SmoothingToggle(usize),
    SmoothingRemove(usize),
    /// Into part `id`, at the end of its operations (dropped on the part's row).
    ToPart(usize, u64),
    PartFocus(u64),
    PartActivate(u64),
    PartExport(u64),
    PartToggle(u64),
    PartMove(u64, bool),
    PartDelete(u64),
    /// A new operation of this kind in part `id`: it becomes the active part first.
    NewIn(OpKind, u64),
    /// Part `0` into setup `1` (docs/specs/setups.md §7).
    PartToSetup(u64, u64),
    /// A new instance of part `0`, mirrored left to right with `1`
    /// (docs/specs/instances.md §4).
    InstanceAdd(u64, bool),
    /// Part `0`'s instance `1`: open it, turn it on or off, mirror it, turn it a quarter
    /// (left with `2`), duplicate it, remove it.
    InstanceFocus(u64, u32),
    InstanceToggle(u64, u32),
    InstanceMirror(u64, u32),
    InstanceTurn(u64, u32, bool),
    InstanceDuplicate(u64, u32),
    InstanceReset(u64, u32),
    InstanceRemove(u64, u32),
    /// Part `.0` placed on setup `.1` where it is drawn, a new instance
    /// (docs/specs/parts-and-setups.md §8).
    PartPlaceOn(u64, u64),
    /// Instance (`.0`, `.1`) cut on setup `.2`, keeping its number and placement.
    InstanceToSetup(u64, u32, u64),
    /// Part `0` itself placed: its instance 1's form; or put back where drawn
    /// (docs/specs/part-placement.md §6).
    PartPlace(u64),
    PartResetPlacement(u64),
}

impl ChiploadApp {
    /// Do what an operation's, part's or instance's row asked.
    pub(super) fn apply_op_action(&mut self, action: OpAction) {
        match action {
            OpAction::Focus(i) => self.focus = Focus::Op(i),
            OpAction::Toggle(i) => {
                if let Some(entry) = self.edit.as_mut().and_then(|e| e.job.operations.get_mut(i)) {
                    let enabled = entry.enabled();
                    entry.set_enabled(!enabled);
                }
            }
            OpAction::Move(from, to) => self.move_operation(from, to),
            OpAction::Duplicate(i) => {
                self.focus = Focus::Op(i);
                self.duplicate_operation();
            }
            OpAction::SaveStyle(i) => self.ask_style_name(i),
            OpAction::Delete(i) => {
                self.focus = Focus::Op(i);
                self.delete_operation();
            }
            OpAction::AddRest(i, tool) => {
                self.add_rest_pass(i, tool);
            }
            OpAction::AddRestFromLibrary(i, name) => {
                self.add_rest_pass_from_library(i, &name);
            }
            OpAction::PassFocus(i, k) => self.focus = Focus::RestPass(i, k),
            OpAction::PassToggle(i, k) => self.toggle_rest_pass(i, k),
            OpAction::PassRemove(i, k) => self.remove_rest_pass(i, k),
            OpAction::MakePass(i) => self.make_rest_pass(i),
            OpAction::AddFloor(i, tool) => {
                self.add_floor_pass(i, tool);
            }
            OpAction::AddFloorFromLibrary(i, name) => {
                self.add_floor_pass_from_library(i, &name);
            }
            OpAction::FloorFocus(i) => self.focus = Focus::FloorPass(i),
            OpAction::FloorToggle(i) => self.toggle_floor_pass(i),
            OpAction::FloorRemove(i) => self.remove_floor_pass(i),
            OpAction::AddRoughing(i, tool) => {
                self.add_roughing(i, tool);
            }
            OpAction::AddRoughingFromLibrary(i, name) => {
                self.add_roughing_from_library(i, &name);
            }
            OpAction::RoughingFocus(i) => self.focus = Focus::RoughingPass(i),
            OpAction::RoughingToggle(i) => self.toggle_roughing(i),
            OpAction::RoughingRemove(i) => self.remove_roughing(i),
            OpAction::AddSmoothing(i) => {
                self.add_smoothing(i);
            }
            OpAction::SmoothingFocus(i) => self.focus = Focus::SmoothingPass(i),
            OpAction::SmoothingToggle(i) => self.toggle_smoothing(i),
            OpAction::SmoothingRemove(i) => self.remove_smoothing(i),
            OpAction::ToPart(from, id) => self.move_operation_to_part(from, id),
            OpAction::PartFocus(id) => self.focus = Focus::Part(id),
            OpAction::PartActivate(id) => self.activate_part(id),
            OpAction::PartExport(id) => self.ask_export_part(id),
            OpAction::PartToggle(id) => self.toggle_part(id),
            OpAction::PartMove(id, up) => self.move_part(id, up),
            OpAction::PartDelete(id) => self.delete_part(id),
            OpAction::NewIn(kind, id) => {
                self.activate_part(id);
                self.add_operation(kind);
            }
            OpAction::PartToSetup(part, setup) => self.move_part_to_setup(part, setup),
            OpAction::InstanceAdd(part, mirrored) => self.add_instance(part, mirrored),
            OpAction::InstanceFocus(part, id) => self.focus = Focus::Instance(part, id),
            OpAction::InstanceToggle(part, id) => self.toggle_instance(part, id),
            OpAction::InstanceMirror(part, id) => self.mirror_instance(part, id),
            OpAction::InstanceTurn(part, id, left) => self.turn_instance(part, id, left),
            OpAction::InstanceDuplicate(part, id) => {
                self.duplicate_instance(part, id);
            }
            OpAction::InstanceReset(part, id) => self.reset_instance(part, id),
            OpAction::InstanceRemove(part, id) => self.remove_instance(part, id),
            OpAction::PartPlace(part) => self.place_part(part),
            OpAction::PartPlaceOn(part, setup) => self.place_part_on(part, setup),
            OpAction::InstanceToSetup(part, id, setup) => self.instance_to_setup(part, id, setup),
            OpAction::PartResetPlacement(part) => self.reset_part_placement(part),
        }
    }

    /// Part `part`'s row (the `k`th part) and, when it is open, its operations.
    pub(super) fn part_rows(
        &mut self,
        ui: &mut egui::Ui,
        part: &chipload_job::Part,
        rows: PartRows,
    ) -> Option<OpAction> {
        let PartRows {
            k,
            depth,
            parts,
            active,
            op_parts,
            board,
            count,
            tool_before,
        } = rows;
        let mine: Vec<usize> = (0..count)
            .filter(|i| op_parts[*i] == part.id && !board[*i])
            .collect();
        let flagged = mine.iter().any(|i| {
            self.problems.iter().any(|p| p.operation() == Some(*i)) || self.error_ops.contains(i)
        }) || !self.form_problem_map(Focus::Part(part.id)).is_empty();
        let is_active = active == Some(part.id);
        // Parts of another setup are ghosts too (docs/specs/setups.md §7).
        // Its placement: where it is cut, how often, whether elsewhere than drawn
        // (docs/specs/parts-and-setups.md §2).
        let (setups, own_setups, times, placed, instances) =
            self.edit
                .as_ref()
                .map_or((Vec::new(), Vec::new(), 1, false, Vec::new()), |e| {
                    let setups: Vec<(u64, String)> = e
                        .job
                        .setups
                        .iter()
                        .map(|s| (s.id, s.name.clone()))
                        .collect();
                    let instances: Vec<chipload_job::Instance> = e
                        .job
                        .instances_of(part.id)
                        .into_iter()
                        .map(|(_, i)| *i)
                        .collect();
                    (
                        setups,
                        e.job.setups_of_part(part.id),
                        e.job.count_all(part.id),
                        e.job.is_placed(part.id),
                        instances,
                    )
                });
        let mut trailing = Vec::new();
        if is_active && parts.len() > 1 {
            trailing.push(t!(ui, "browser-part-active"));
        }
        if instances.is_empty() {
            trailing.push(t!(ui, "browser-part-not-placed"));
        } else if times > 1 {
            trailing.push(format!("×{times}"));
        }
        if part.mirror.is_some() {
            trailing.push(t!(ui, "browser-part-mirrored"));
        }
        let id = Id::new(("browser-part", part.id));
        let open_id = id.with("folder-open");
        let open = folder_open(ui, open_id, true);
        let row = Row::new(id, depth, &part.name)
            .folder(open)
            .tick(part.enabled)
            .icon(Icon::Part)
            .trailing(trailing.join(" · "))
            .selected(self.focus == Focus::Part(part.id))
            .dim(!part.enabled)
            .alert(flagged)
            .strong(is_active && parts.len() > 1);
        // Dragged onto a setup, it moves there; with Ctrl it is placed there too
        // (docs/specs/parts-and-setups.md §8).
        let out = row.draggable().show(ui);
        out.response.dnd_set_drag_payload(DraggedPart(part.id));
        let mut action = None;
        if out.toggled {
            toggle_folder(ui, open_id, true);
        }
        if out.tick {
            action = Some(OpAction::PartToggle(part.id));
        } else if out.response.double_clicked() {
            action = Some(OpAction::PartActivate(part.id));
        } else if out.response.clicked() {
            action = Some(OpAction::PartFocus(part.id));
        }
        // A board surface belongs to its setup: no part takes it (surface-levels.md §2).
        let board_dragged = |from: usize| board.get(from).copied().unwrap_or(false);
        if let Some(from) = out.response.dnd_release_payload::<usize>() {
            action = Some(OpAction::ToPart(*from, part.id));
        } else if out
            .response
            .dnd_hover_payload::<usize>()
            .is_some_and(|from| !board_dragged(*from))
        {
            ui.painter().rect_stroke(
                out.response.rect,
                0.0,
                Stroke::new(2.0, ui.visuals().selection.stroke.color),
                egui::StrokeKind::Inside,
            );
        }
        let selected = !self.selected_ids().is_empty();
        let models = self.has_models();
        let hover = if is_active {
            t!(ui, "browser-part-active-hover")
        } else {
            t!(ui, "browser-part-hover")
        };
        let hover = format!("{hover}\n\n{}", t!(ui, "browser-part-drag-hover"));
        out.response.on_hover_text(hover).context_menu(|ui| {
            if ui
                .add_enabled(!is_active, egui::Button::new(t!(ui, "browser-activate")))
                .on_hover_text(t!(ui, "browser-activate-hover"))
                .clicked()
            {
                action = Some(OpAction::PartActivate(part.id));
            }
            if ui
                .button(t!(ui, "browser-export-part"))
                .on_hover_text(t!(ui, "browser-export-part-hover"))
                .clicked()
            {
                action = Some(OpAction::PartExport(part.id));
            }
            if ui
                .button(t!(ui, "browser-add-instance"))
                .on_hover_text(t!(ui, "browser-add-instance-hover"))
                .clicked()
            {
                action = Some(OpAction::InstanceAdd(part.id, false));
            }
            if ui
                .button(t!(ui, "browser-add-mirrored-instance"))
                .on_hover_text(t!(ui, "browser-add-mirrored-instance-hover"))
                .clicked()
            {
                action = Some(OpAction::InstanceAdd(part.id, true));
            }
            // Its first instance selected, to move and turn it.
            if ui
                .add_enabled(
                    !instances.is_empty(),
                    egui::Button::new(t!(ui, "browser-place-part")),
                )
                .on_hover_text(t!(ui, "browser-place-part-hover"))
                .clicked()
            {
                action = Some(OpAction::PartPlace(part.id));
            }
            if ui
                .add_enabled(placed, egui::Button::new(t!(ui, "browser-reset-placement")))
                .on_hover_text(t!(ui, "browser-reset-placement-hover"))
                .clicked()
            {
                action = Some(OpAction::PartResetPlacement(part.id));
            }
            if setups.len() > 1 {
                super::header::submenu(ui, t!(ui, "browser-place-on-setup"), |ui| {
                    for (id, name) in &setups {
                        if ui
                            .button(name)
                            .on_hover_text(t!(ui, "browser-place-on-setup-hover"))
                            .clicked()
                        {
                            action = Some(OpAction::PartPlaceOn(part.id, *id));
                        }
                    }
                });
                super::header::submenu(ui, t!(ui, "browser-move-to-setup"), |ui| {
                    for (id, name) in &setups {
                        if ui
                            .add_enabled(own_setups != [*id], egui::Button::new(name))
                            .clicked()
                        {
                            action = Some(OpAction::PartToSetup(part.id, *id));
                        }
                    }
                });
            }
            super::header::submenu(ui, t!(ui, "browser-new-operation"), |ui| {
                for (label, kind, hover) in new_operations(ui) {
                    let why = match kind {
                        OpKind::Relief => cannot_add(ui, kind),
                        _ => t!(ui, "browser-select-shapes-first"),
                    };
                    if ui
                        .add_enabled(can_add(kind, selected, models), egui::Button::new(label))
                        .on_hover_text(hover)
                        .on_disabled_hover_text(why)
                        .clicked()
                    {
                        action = Some(OpAction::NewIn(kind, part.id));
                    }
                }
            });
            ui.separator();
            if ui
                .add_enabled(k > 0, egui::Button::new(t!(ui, "browser-move-up")))
                .clicked()
            {
                action = Some(OpAction::PartMove(part.id, true));
            }
            if ui
                .add_enabled(
                    k + 1 < parts.len(),
                    egui::Button::new(t!(ui, "browser-move-down")),
                )
                .clicked()
            {
                action = Some(OpAction::PartMove(part.id, false));
            }
            ui.separator();
            let what = t!(ui, "browser-delete-part", count = mine.len());
            if ui
                .add_enabled(parts.len() > 1, egui::Button::new(what))
                .on_disabled_hover_text(t!(ui, "browser-delete-part-disabled-hover"))
                .clicked()
            {
                action = Some(OpAction::PartDelete(part.id));
            }
        });
        if !open {
            return action;
        }
        if mine.is_empty() {
            ui.horizontal(|ui| {
                ui.add_space(PAD + (depth + 2) as f32 * INDENT);
                ui.weak(if is_active {
                    t!(ui, "browser-part-empty-active")
                } else {
                    t!(ui, "browser-part-empty")
                });
            });
        }
        for i in mine {
            let changes = tool_before.get(&i).map_or(&[][..], Vec::as_slice);
            if let Some(a) = self.operation_row(ui, i, count, changes, depth + 1) {
                action = Some(a);
            }
        }
        // Instances are never under their part: a project with more than one cut of a part,
        // or one moved, lists its setups, each with its instances
        // (docs/specs/parts-and-setups.md §2a, §8).
        action
    }

    /// Instance `instance` of `part`'s row (docs/specs/instances.md §4): its number, on or
    /// off, how it is turned and mirrored; its menu mirrors, turns, moves it to another setup
    /// or removes it. Under its setup it is named with its part, and can be dragged onto
    /// another setup (docs/specs/parts-and-setups.md §8).
    pub(super) fn instance_row(
        &mut self,
        ui: &mut egui::Ui,
        part: &chipload_job::Part,
        instance: &chipload_job::Instance,
        look: InstanceLook,
    ) -> Option<OpAction> {
        let InstanceLook {
            in_active_setup,
            depth,
            named,
            setups,
        } = look;
        let (p, id) = (part.id, instance.id);
        let mut trailing = Vec::new();
        if instance.angle != 0.0 {
            trailing.push(format!("{:.1}°", instance.angle.to_degrees()));
        }
        if instance.mirror.is_some() {
            trailing.push(t!(ui, "browser-part-mirrored"));
        }
        let flagged = !self.form_problem_map(Focus::Instance(p, id)).is_empty();
        let label = if named {
            t!(
                ui,
                "browser-instance-of",
                part = part.name.as_str(),
                number = id
            )
        } else {
            t!(ui, "forms-instance-number", number = id)
        };
        let row = Row::new(Id::new(("browser-instance", p, id)), depth, label)
            .tick(instance.enabled)
            .icon(Icon::Array)
            .trailing(trailing.join(" · "))
            .selected(self.focus == Focus::Instance(p, id))
            .dim(!instance.enabled || !part.enabled || !in_active_setup)
            .alert(flagged);
        let several = setups.len() > 1;
        let out = if several { row.draggable() } else { row }.show(ui);
        if several {
            out.response.dnd_set_drag_payload(DraggedInstance(p, id));
        }
        let mut action = None;
        if out.tick {
            action = Some(OpAction::InstanceToggle(p, id));
        } else if out.response.clicked() {
            action = Some(OpAction::InstanceFocus(p, id));
        }
        let hover = t!(ui, "browser-instance-hover");
        out.response.on_hover_text(hover).context_menu(|ui| {
            if let Some(chosen) = self.instance_menu_items(ui, p, id) {
                action = Some(chosen);
            }
            if several {
                let here = self
                    .edit
                    .as_ref()
                    .and_then(|e| e.job.instances_of(p).into_iter().find(|(_, i)| i.id == id))
                    .map(|(s, _)| s);
                super::header::submenu(ui, t!(ui, "browser-move-to-setup"), |ui| {
                    for (setup, name) in setups {
                        if ui
                            .add_enabled(here != Some(*setup), egui::Button::new(name))
                            .clicked()
                        {
                            action = Some(OpAction::InstanceToSetup(p, id, *setup));
                        }
                    }
                });
            }
            ui.separator();
            if ui.button(t!(ui, "browser-instance-remove")).clicked() {
                action = Some(OpAction::InstanceRemove(p, id));
            }
        });
        action
    }

    /// Operation `i`'s row (and the tool change line above it). `changes`: the tools changed
    /// to above it ([`tool_changes`]).
    pub(super) fn operation_row(
        &mut self,
        ui: &mut egui::Ui,
        i: usize,
        count: usize,
        changes: &[i64],
        depth: usize,
    ) -> Option<OpAction> {
        let edit = self.edit.as_ref()?;
        let entry = edit.job.operations.get(i)?;
        let op = &entry;
        // An operation is cut when it and its part are on (docs/specs/parts.md §3).
        let cut =
            |op: &Operation| op.enabled() && edit.job.part(op.part()).is_some_and(|p| p.enabled);
        // A line marks each tool change (docs/specs/tool-per-operation.md §8).
        let is_cut = cut(op);
        if is_cut && !changes.is_empty() {
            tool_change(ui, changes);
        }
        let problem = self
            .problems
            .iter()
            .find(|p| p.operation() == Some(i))
            .map(|p| problems::plain(ui, self.prefs.units, p));
        let missing = edit.missing().iter().any(|p| p.operation() == Some(i));
        let flagged = missing || self.error_ops.contains(&i) || problem.is_some();
        let hidden = self.hidden_cut(op);
        // What a rest pass for this pocket can be cut with, or why it can't
        // (docs/specs/rest-passes.md §6).
        let units = crate::units::display(ui.ctx());
        let rest_menu = self.rest_menu(i, units);
        // Styles are kept in the settings folder (docs/specs/styles.md §4).
        let has_library = self.user.settings_dir.is_some();
        // A V-carve's floor pass, or what it can be cut with
        // (docs/specs/vcarve-wide-strokes.md §7).
        let floor = match op {
            Operation::Vcarve(v) => v.floor,
            _ => None,
        };
        let floor_menu = match floor {
            None => self.floor_menu(i),
            Some(_) => None,
        };
        // A relief's roughing pass, or what it can be cut with (docs/specs/stl-relief.md §14).
        let roughing = match op {
            Operation::Relief(r) => r.roughing,
            _ => None,
        };
        let smoothing = match op {
            Operation::Relief(r) => r.smoothing,
            _ => None,
        };
        let roughing_menu = match roughing {
            None => self.roughing_menu(i),
            Some(_) => None,
        };
        let name = match op.name() {
            "" => t!(ui, "browser-no-name"),
            name => name.to_string(),
        };
        let trailing = if hidden > 0 {
            let hidden = t!(ui, "browser-op-hidden", count = hidden);
            format!("{hidden} · T{}", op.tool())
        } else {
            format!("T{}", op.tool())
        };
        let hover = self.operation_hover(i, problem.as_deref(), hidden);
        // A rest pocket that copies an earlier pocket can become its rest pass (§5).
        let parent = match op {
            Operation::Pocket(p) if p.rest.is_some() => self.rest_pocket_parent(i),
            _ => None,
        };
        let passes: Vec<chipload_job::ClearingPass> = match op {
            Operation::Pocket(p) => p.rest_passes.clone(),
            _ => Vec::new(),
        };
        let dark = ui.visuals().dark_mode;
        // A board surface stays first in its setup (surface-levels.md §2): no dragging,
        // nothing dropped on it, no moving up or down.
        let board = op.is_board_surface();
        let row = Row::new(
            Id::new(("browser-op", i)),
            depth,
            format!("{}  {name}", i + 1),
        )
        .tick(op.enabled())
        .icon_colored(op_icon(op), kind_color(op, dark))
        .trailing(trailing)
        .selected(self.focus == Focus::Op(i))
        .dim(!is_cut)
        .alert(flagged);
        let row = if board { row } else { row.draggable() }.show(ui);
        let mut action = None;
        if row.tick {
            action = Some(OpAction::Toggle(i));
        } else if row.response.clicked() {
            action = Some(OpAction::Focus(i));
        }
        let dragged_board = |from: usize| edit.is_board_surface(from);
        if !board {
            row.response.dnd_set_drag_payload(i);
        }
        if board {
            // Not a drop target: whatever is dropped here would go before it.
        } else if let Some(from) = row.response.dnd_release_payload::<usize>() {
            action = Some(OpAction::Move(*from, i));
        } else if row
            .response
            .dnd_hover_payload::<usize>()
            .is_some_and(|from| !dragged_board(*from))
        {
            ui.painter().hline(
                row.response.rect.x_range(),
                row.response.rect.top(),
                Stroke::new(2.0, ui.visuals().selection.stroke.color),
            );
        }
        let response = row.response.on_hover_text(hover);
        response.context_menu(|ui| {
            if ui
                .add_enabled(
                    i > 0 && !board,
                    egui::Button::new(with_keys(
                        &t!(ui, "browser-move-up"),
                        shortcuts::Action::MoveUp,
                    )),
                )
                .clicked()
            {
                action = Some(OpAction::Move(i, i - 1));
            }
            if ui
                .add_enabled(
                    i + 1 < count && !board,
                    egui::Button::new(with_keys(
                        &t!(ui, "browser-move-down"),
                        shortcuts::Action::MoveDown,
                    )),
                )
                .clicked()
            {
                action = Some(OpAction::Move(i, i + 1));
            }
            if ui.button(t!(ui, "browser-duplicate")).clicked() {
                action = Some(OpAction::Duplicate(i));
            }
            if ui
                .add_enabled(has_library, egui::Button::new(t!(ui, "styles-save-as")))
                .on_hover_text(t!(ui, "styles-save-as-hover"))
                .clicked()
            {
                action = Some(OpAction::SaveStyle(i));
            }
            if let Some(menu) = &rest_menu {
                let label = t!(ui, "browser-add-rest-pass");
                let hover = t!(ui, "browser-add-rest-pass-hover");
                match tool_menu(ui, &label, &hover, menu, units) {
                    Some(ToolPick::Project(tool)) => action = Some(OpAction::AddRest(i, tool)),
                    Some(ToolPick::Library(name)) => {
                        action = Some(OpAction::AddRestFromLibrary(i, name));
                    }
                    None => {}
                }
            }
            if let Some(menu) = &floor_menu {
                let label = t!(ui, "browser-add-floor-pass");
                let hover = t!(ui, "browser-add-floor-pass-hover");
                match tool_menu(ui, &label, &hover, menu, units) {
                    Some(ToolPick::Project(tool)) => action = Some(OpAction::AddFloor(i, tool)),
                    Some(ToolPick::Library(name)) => {
                        action = Some(OpAction::AddFloorFromLibrary(i, name));
                    }
                    None => {}
                }
            }
            if let Some(menu) = &roughing_menu {
                let label = t!(ui, "browser-add-roughing");
                let hover = t!(ui, "browser-add-roughing-hover");
                match tool_menu(ui, &label, &hover, menu, units) {
                    Some(ToolPick::Project(tool)) => action = Some(OpAction::AddRoughing(i, tool)),
                    Some(ToolPick::Library(name)) => {
                        action = Some(OpAction::AddRoughingFromLibrary(i, name));
                    }
                    None => {}
                }
            }
            // A smoothing pass goes after a roughing pass (docs/specs/stl-relief.md §17):
            // offered on the relief too, greyed out until it has one (the user, 2026-11-14).
            if matches!(op, Operation::Relief(_)) && smoothing.is_none() {
                let hover = if roughing.is_some() {
                    t!(ui, "forms-add-smoothing-hover")
                } else {
                    t!(ui, "browser-add-smoothing-needs-roughing")
                };
                if ui
                    .add_enabled(
                        roughing.is_some(),
                        egui::Button::new(t!(ui, "forms-add-smoothing")),
                    )
                    .on_hover_text(hover)
                    .on_disabled_hover_text(t!(ui, "browser-add-smoothing-needs-roughing"))
                    .clicked()
                {
                    action = Some(OpAction::AddSmoothing(i));
                }
            }
            if let Some(parent) = parent
                && ui
                    .button(t!(
                        ui,
                        "browser-make-rest-pass",
                        number = (parent + 1).to_string()
                    ))
                    .on_hover_text(t!(ui, "browser-make-rest-pass-hover"))
                    .clicked()
            {
                action = Some(OpAction::MakePass(i));
            }
            ui.separator();
            if ui.button(t!(ui, "browser-delete")).clicked() {
                action = Some(OpAction::Delete(i));
            }
        });
        let under = Under {
            cut: is_cut,
            color: kind_color(op, dark),
            depth: depth + 1,
        };
        for (k, pass) in passes.iter().enumerate() {
            if let Some(a) = self.rest_pass_row(ui, i, k, pass, under) {
                action = Some(a);
            }
        }
        // Cut before the operation: a floor pass, or a roughing and a smoothing pass.
        if let Some(pass) = &floor
            && let Some(a) = self.pass_leaf(ui, i, Leaf::Floor, (pass.enabled, pass.tool), under)
        {
            action = Some(a);
        }
        if let Some(pass) = &roughing
            && let Some(a) = self.pass_leaf(ui, i, Leaf::Roughing, (pass.enabled, pass.tool), under)
        {
            action = Some(a);
        }
        if let Some(pass) = &smoothing
            && let Some(a) =
                self.pass_leaf(ui, i, Leaf::Smoothing, (pass.enabled, pass.tool), under)
        {
            action = Some(a);
        }
        action
    }

    /// Operation `i`'s one pass cut before it, a leaf under it: a V-carve's floor pass
    /// (docs/specs/vcarve-wide-strokes.md §7) or a relief's roughing pass
    /// (docs/specs/stl-relief.md §14). `pass`: whether it is on, and its tool.
    fn pass_leaf(
        &self,
        ui: &mut egui::Ui,
        i: usize,
        leaf: Leaf,
        (enabled, tool): (bool, i64),
        under: Under,
    ) -> Option<OpAction> {
        let Under {
            cut: op_cut,
            color,
            depth,
        } = under;
        let at = chipload_job::OpRef::pass(i, usize::from(matches!(leaf, Leaf::Smoothing)));
        let smoothed = matches!(
            self.edit.as_ref().and_then(|e| e.job.operations.get(i)),
            Some(Operation::Relief(r)) if r.smoothing.is_some()
        );
        let (field, focus, icon, name, hover) = match leaf {
            Leaf::Floor => (
                "floor",
                Focus::FloorPass(i),
                Icon::Pocket,
                t!(ui, "browser-floor"),
                t!(ui, "browser-floor-pass-hover"),
            ),
            Leaf::Roughing => (
                "roughing",
                Focus::RoughingPass(i),
                Icon::Relief,
                t!(ui, "browser-roughing"),
                t!(ui, "browser-roughing-hover"),
            ),
            Leaf::Smoothing => (
                "smoothing",
                Focus::SmoothingPass(i),
                Icon::Relief,
                t!(ui, "browser-smoothing"),
                t!(ui, "browser-smoothing-hover"),
            ),
        };
        let flagged = self
            .problems
            .iter()
            .any(|p| p.operation() == Some(i) && p.field().starts_with(field));
        let time = self
            .toolpath
            .result
            .as_ref()
            .and_then(|r| r.estimate.operations.iter().find(|(o, _)| *o == at))
            .map_or(String::new(), |(_, s)| {
                let time = chipload_pipeline::estimate::duration_message(*s);
                format!("{} · ", time.render(ui.lang(), self.prefs.units.into()))
            });
        let row = Row::new(
            Id::new(("browser-", field, i)),
            depth,
            format!("{at}  {name}"),
        )
        .tick(enabled)
        .icon_colored(icon, color)
        .trailing(format!("{time}T{tool}"))
        .selected(self.focus == focus)
        .dim(!(op_cut && enabled))
        .alert(flagged)
        .show(ui);
        let (toggle, open, remove) = match leaf {
            Leaf::Floor => (
                OpAction::FloorToggle(i),
                OpAction::FloorFocus(i),
                OpAction::FloorRemove(i),
            ),
            Leaf::Roughing => (
                OpAction::RoughingToggle(i),
                OpAction::RoughingFocus(i),
                OpAction::RoughingRemove(i),
            ),
            Leaf::Smoothing => (
                OpAction::SmoothingToggle(i),
                OpAction::SmoothingFocus(i),
                OpAction::SmoothingRemove(i),
            ),
        };
        let mut action = None;
        if row.tick {
            action = Some(toggle);
        } else if row.response.clicked() {
            action = Some(open);
        }
        row.response.on_hover_text(hover).context_menu(|ui| {
            if matches!(leaf, Leaf::Roughing)
                && !smoothed
                && ui
                    .button(t!(ui, "forms-add-smoothing"))
                    .on_hover_text(t!(ui, "forms-add-smoothing-hover"))
                    .clicked()
            {
                action = Some(OpAction::AddSmoothing(i));
            }
            if ui.button(t!(ui, "browser-remove")).clicked() {
                action = Some(remove);
            }
        });
        action
    }

    /// Pocket `i`'s rest pass `k`, a leaf under it (docs/specs/rest-passes.md §6).
    fn rest_pass_row(
        &self,
        ui: &mut egui::Ui,
        i: usize,
        k: usize,
        pass: &chipload_job::ClearingPass,
        under: Under,
    ) -> Option<OpAction> {
        let Under {
            cut: pocket_cut,
            color,
            depth,
        } = under;
        let at = chipload_job::OpRef::pass(i, k);
        let flagged = self.problems.iter().any(|p| {
            p.operation() == Some(i) && p.field().starts_with(&format!("rest_passes.{k}"))
        });
        let time = self
            .toolpath
            .result
            .as_ref()
            .and_then(|r| r.estimate.operations.iter().find(|(o, _)| *o == at))
            .map_or(String::new(), |(_, s)| {
                let time = chipload_pipeline::estimate::duration_message(*s);
                format!("{} · ", time.render(ui.lang(), self.prefs.units.into()))
            });
        let row = Row::new(
            Id::new(("browser-pass", i, k)),
            depth,
            format!("{at}  {}", t!(ui, "browser-rest")),
        )
        .tick(pass.enabled)
        .icon_colored(Icon::Pocket, color)
        .trailing(format!("{time}T{}", pass.tool))
        .selected(self.focus == Focus::RestPass(i, k))
        .dim(!(pocket_cut && pass.enabled))
        .alert(flagged)
        .show(ui);
        let mut action = None;
        if row.tick {
            action = Some(OpAction::PassToggle(i, k));
        } else if row.response.clicked() {
            action = Some(OpAction::PassFocus(i, k));
        }
        row.response
            .on_hover_text(t!(ui, "browser-rest-pass-hover"))
            .context_menu(|ui| {
                if ui.button(t!(ui, "browser-remove")).clicked() {
                    action = Some(OpAction::PassRemove(i, k));
                }
            });
        action
    }

    /// An operation row's hover text: its kind, tool, geometry, depths and time, then its
    /// first problem (already in the display units) and the hidden contours it cuts.
    fn operation_hover(&self, i: usize, problem: Option<&str>, hidden: usize) -> String {
        let Some(entry) = self.edit.as_ref().and_then(|e| e.job.operations.get(i)) else {
            return String::new();
        };
        let op = &entry;
        let kind = match op {
            Operation::Profile(_) => t!(self, "browser-profile"),
            Operation::Pocket(_) => t!(self, "browser-pocket"),
            Operation::Vcarve(_) => t!(self, "browser-vcarve"),
            Operation::Path(_) => t!(self, "browser-follow-path"),
            Operation::Chamfer(_) => t!(self, "browser-chamfer"),
            Operation::Flute(_) => t!(self, "browser-flute"),
            Operation::Pattern(_) => t!(self, "browser-pattern"),
            Operation::Prism(_) => t!(self, "browser-prism"),
            Operation::Moulding(_) => t!(self, "browser-moulding"),
            Operation::Holes(_) => t!(self, "browser-holes"),
            Operation::Surface(_) => t!(self, "browser-surface"),
            Operation::Relief(_) => t!(self, "browser-relief"),
        };
        let summary = geometry_summary(op, &self.contours, self);
        let tools = self.edit.as_ref().map_or(&[][..], |e| &e.job.tools[..]);
        let (start, end) = op.depths(tools);
        // docs/specs/rest-machining.md §6.
        let rest = match op {
            Operation::Pocket(p) => p.rest.map_or(String::new(), |r| {
                let rest = t!(
                    self,
                    "browser-op-rest-after",
                    tool = r.after_tool.to_string()
                );
                format!(", {rest}")
            }),
            _ => String::new(),
        };
        let time = self
            .toolpath
            .result
            .as_ref()
            .and_then(|r| {
                r.estimate
                    .operations
                    .iter()
                    .find(|(o, _)| *o == chipload_job::OpRef::op(i))
            })
            .map_or(String::new(), |(_, s)| {
                let time = chipload_pipeline::estimate::duration_message(*s);
                format!(" · {}", time.render(self.lang(), self.prefs.units.into()))
            });
        // The depths as Rust writes them in mm, 4 decimals in inches.
        let depth = |mm: f64| {
            chipload_i18n::message::length_number(
                mm,
                chipload_i18n::Digits::Shortest,
                self.prefs.units.into(),
            )
        };
        let first = format!(
            "{kind} · T{}{rest} · {summary} · {}–{} {}{time}",
            op.tool(),
            depth(start),
            depth(end),
            chipload_pipeline::units::length_unit(self.prefs.units)
        );
        let mut lines = vec![first];
        if !op.enabled() {
            lines.push(t!(self, "browser-op-not-cut"));
        }
        if let Some(problem) = problem {
            lines.push(format!("⚠ {problem}"));
        }
        if hidden > 0 {
            lines.push(t!(self, "browser-op-hidden-hover", count = hidden));
        }
        lines.push(t!(self, "browser-op-drag-hover"));
        lines.join("\n")
    }
}
