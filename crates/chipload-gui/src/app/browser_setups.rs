// SPDX-License-Identifier: GPL-3.0-or-later
//! The Browser's setups: each with its stock, work zero and parts (docs/specs/setups.md §7).

use eframe::egui::Id;

use super::browser::tool_changes;
use super::browser_ops::{DraggedInstance, DraggedPart, InstanceLook, OpAction, PartRows};
use super::browser_row::{Row, folder, folder_open, toggle_folder};
use super::*;
use crate::icons::Icon;
use crate::style::Tint;

/// What a setup's row needs besides the setup.
#[derive(Clone, Copy)]
struct SetupRows<'a> {
    setups: &'a [chipload_job::Setup],
    /// How many parts it holds.
    parts: usize,
    /// A back whose front has no dowel holes (two-sided-wizard.md §3).
    missing: bool,
    is_active: bool,
}

/// What a board's rows need from the project ([`ChiploadApp::board_contents`]).
#[derive(Clone, Copy)]
struct BoardRows<'a> {
    parts: &'a [chipload_job::Part],
    active: Option<u64>,
    op_parts: &'a [u64],
    board: &'a [bool],
    count: usize,
    tool_before: &'a std::collections::BTreeMap<usize, Vec<i64>>,
}

/// What a setup's rows ask for.
#[derive(Debug, PartialEq)]
pub(super) enum SetupAction {
    Add,
    /// A back side for this setup (docs/specs/two-sided.md §3).
    AddBack(u64),
    Toggle(u64),
    Open(u64),
    Stock(u64),
    Zero(u64),
    NewPart(u64),
    Remove(u64),
    Export(u64),
    /// Every setup's programs and sheets at once (docs/specs/parts-and-setups.md §8).
    ExportAll,
    /// A board surface first in this setup (docs/specs/surface-levels.md §5).
    FaceBoard(u64),
    /// Nest its instances on its board (docs/specs/nesting.md §7).
    Nest(u64),
    NestAgain(u64),
    /// A part's row dropped on this setup: every instance of it moved there
    /// (docs/specs/parts-and-setups.md §8).
    MovePart(u64, u64),
    /// A part's row dropped on this setup with Ctrl (⌘): placed there too, a new instance.
    PlacePart(u64, u64),
    /// An instance's row dropped on this setup: cut there.
    MoveInstance(u64, u32, u64),
    /// List the setups even with one board (true), or go back to the simple view (false;
    /// docs/specs/parts-and-setups.md §2a).
    ShowSetups(bool),
}

/// What a part's row dropped on setup `setup` does (the user, 2026-11-17): it moves there,
/// every instance of it; with Ctrl (⌘ on a Mac) it is placed there too, a new instance.
pub(super) fn part_dropped(part: u64, setup: u64, copy: bool) -> SetupAction {
    if copy {
        SetupAction::PlacePart(part, setup)
    } else {
        SetupAction::MovePart(part, setup)
    }
}

impl ChiploadApp {
    /// Parts (docs/specs/parts.md §8): each part with its operations, in cutting order,
    /// with a line at each tool change.
    pub(super) fn operation_rows(&mut self, ui: &mut egui::Ui) {
        let Some(edit) = self.edit.as_ref() else {
            return;
        };
        let parts = edit.job.parts.clone();
        let active = edit.job.active_part_id();
        let count = edit.job.operations.len();
        let op_parts: Vec<u64> = edit.job.operations.iter().map(|e| e.part()).collect();
        let board: Vec<bool> = edit
            .job
            .operations
            .iter()
            .map(Operation::is_board_surface)
            .collect();
        let tool_before = self
            .edited_job()
            .map(|job| tool_changes(&job))
            .unwrap_or_default();
        let mut action: Option<OpAction> = None;
        // Setups, each with its zero and its parts as leaves (docs/specs/setups.md §7): the
        // active one in full colour, the others as ghosts.
        let setups = edit.job.setups.clone();
        // Backs put back by pins whose front has no dowel holes (two-sided-wizard.md §3).
        let missing_dowels: Vec<u64> = setups
            .iter()
            .filter(|s| edit.job.missing_dowels(s.id))
            .map(|s| s.id)
            .collect();
        let active_setup = edit.job.active_setup_id();
        let mut setup_action: Option<SetupAction> = None;
        let rows = BoardRows {
            parts: &parts,
            active,
            op_parts: &op_parts,
            board: &board,
            count,
            tool_before: &tool_before,
        };
        // One board: no Setups branch; its stock, zero and parts are the project's rows
        // (docs/specs/parts-and-setups.md §2a).
        let simple = !edit.job.lists_setups();
        if let ([setup], true) = (setups.as_slice(), simple) {
            let (a, b) = self.board_contents(ui, setup.id, 1, rows, true);
            setup_action = a.or(setup_action);
            action = b.or(action);
        } else {
            // The parts as definitions, each with its operations; their instances are under
            // the setups (docs/specs/parts-and-setups.md §8).
            let row = Row::new(Id::new("browser-parts"), 1, t!(ui, "browser-parts"))
                .icon(Icon::Part)
                .trailing(parts.len().to_string());
            let (parts_open, parts_row) = folder(ui, row, true);
            parts_row
                .response
                .on_hover_text(t!(ui, "browser-parts-hover"));
            if parts_open {
                for (k, part) in parts.iter().enumerate() {
                    let rows = PartRows {
                        k,
                        depth: 2,
                        parts: &parts,
                        active,
                        op_parts: &op_parts,
                        board: &board,
                        count,
                        tool_before: &tool_before,
                        simple: false,
                    };
                    if let Some(a) = self.part_rows(ui, part, rows) {
                        action = Some(a);
                    }
                }
            }
            let row = Row::new(Id::new("browser-setups"), 1, t!(ui, "browser-setups"))
                .icon_tinted(Icon::Rectangle, Tint::Board)
                .trailing(setups.len().to_string());
            let (open, folder_row) = folder(ui, row, true);
            folder_row
                .response
                .on_hover_text(t!(ui, "browser-setups-hover"))
                .context_menu(|ui| {
                    if ui
                        .button(t!(ui, "browser-add-setup"))
                        .on_hover_text(t!(ui, "browser-add-setup-hover"))
                        .clicked()
                    {
                        setup_action = Some(SetupAction::Add);
                    }
                    if setups.len() > 1
                        && ui
                            .button(t!(ui, "menu-export-all"))
                            .on_hover_text(t!(ui, "menu-export-all-hover"))
                            .clicked()
                    {
                        setup_action = Some(SetupAction::ExportAll);
                    }
                });
            if open {
                for setup in &setups {
                    let on: Vec<u64> = self
                        .edit
                        .as_ref()
                        .map(|e| e.job.parts_on(setup.id))
                        .unwrap_or_default();
                    let setup_rows = SetupRows {
                        setups: &setups,
                        parts: on.len(),
                        missing: missing_dowels.contains(&setup.id),
                        is_active: active_setup == Some(setup.id),
                    };
                    let (setup_open, asked) = self.setup_row(ui, setup, setup_rows);
                    if asked.is_some() {
                        setup_action = asked;
                    }
                    if !setup_open {
                        continue;
                    }
                    let (a, b) = self.board_contents(ui, setup.id, 3, rows, false);
                    setup_action = a.or(setup_action);
                    action = b.or(action);
                }
            }
        }
        if let Some(action) = setup_action {
            self.apply_setup_action(action);
        }
        if let Some(action) = action {
            self.apply_op_action(action);
        }
    }

    /// What is on board `id`, its rows at `depth`: its stock, work zero and board surfaces,
    /// then its parts with their operations (docs/specs/setups.md §7). Under its setup's row,
    /// or under the project with one board (docs/specs/parts-and-setups.md §2a).
    fn board_contents(
        &mut self,
        ui: &mut egui::Ui,
        id: u64,
        depth: usize,
        rows: BoardRows,
        simple: bool,
    ) -> (Option<SetupAction>, Option<OpAction>) {
        let BoardRows {
            parts,
            active,
            op_parts,
            board,
            count,
            tool_before,
        } = rows;
        let (mut setup_action, mut action) = (None, None);
        let on: Vec<u64> = self
            .edit
            .as_ref()
            .map(|e| e.job.parts_on(id))
            .unwrap_or_default();
        if let Some(a) = self.stock_row(ui, id, depth) {
            setup_action = Some(a);
        }
        if let Some(a) = self.work_zero_row(ui, id, depth) {
            setup_action = Some(a);
        }
        // The board surfaces, before its parts (surface-levels.md §5).
        for i in (0..count).filter(|&i| board[i] && on.contains(&op_parts[i])) {
            let changes = tool_before.get(&i).map_or(&[][..], Vec::as_slice);
            if let Some(a) = self.operation_row(ui, i, count, changes, depth) {
                action = Some(a);
            }
        }
        if !simple {
            // Its instances, by part in the project's order, each named with its part.
            let (setups, instances, active_setup) = self.edit.as_ref().map_or_else(
                || (Vec::new(), Vec::new(), None),
                |e| {
                    let setups: Vec<(u64, String)> = e
                        .job
                        .setups
                        .iter()
                        .map(|s| (s.id, s.name.clone()))
                        .collect();
                    let mine = e
                        .job
                        .setup(id)
                        .map(|s| s.instances.clone())
                        .unwrap_or_default();
                    (setups, mine, e.job.active_setup_id())
                },
            );
            let look = InstanceLook {
                in_active_setup: active_setup == Some(id),
                depth,
                named: true,
                setups: &setups,
            };
            for part in parts.iter().filter(|p| on.contains(&p.id)) {
                for instance in instances.iter().filter(|i| i.part == part.id) {
                    if let Some(a) = self.instance_row(ui, part, instance, look) {
                        action = Some(a);
                    }
                }
            }
            return (setup_action, action);
        }
        for (k, part) in parts.iter().enumerate().filter(|(_, p)| on.contains(&p.id)) {
            let rows = PartRows {
                k,
                depth,
                parts,
                active,
                op_parts,
                board,
                count,
                tool_before,
                simple,
            };
            if let Some(a) = self.part_rows(ui, part, rows) {
                action = Some(a);
            }
        }
        (setup_action, action)
    }

    /// One setup's row, with its menu: the active one in full colour, a back marked as the
    /// board turned over (docs/specs/two-sided.md §3). Returns whether it is open, and what
    /// was asked.
    fn setup_row(
        &mut self,
        ui: &mut egui::Ui,
        setup: &chipload_job::Setup,
        rows: SetupRows,
    ) -> (bool, Option<SetupAction>) {
        let SetupRows {
            setups,
            parts: mine_count,
            missing,
            is_active,
        } = rows;
        let mut action = None;
        let id = Id::new(("browser-setup", setup.id));
        let open_id = id.with("folder-open");
        let setup_open = folder_open(ui, open_id, true);
        let several = setups.len() > 1;
        // A back is marked as the board turned over (docs/specs/two-sided.md §3).
        let parts_text = t!(ui, "browser-setup-parts", count = mine_count);
        let trailing = match setup.back_of {
            Some(_) if missing => format!("⚠ ⟲ {parts_text}"),
            Some(_) => format!("⟲ {parts_text}"),
            None => parts_text,
        };
        let can_turn = setup.back_of.is_none()
            && !setups
                .iter()
                .any(|s| s.back_of.is_some_and(|b| b.setup == setup.id));
        let mut row = Row::new(id, 2, &setup.name)
            .folder(setup_open)
            .icon_tinted(Icon::Rectangle, Tint::Board)
            .trailing(trailing)
            .selected(self.focus == Focus::Setup(setup.id))
            .dim(!setup.enabled || !is_active)
            .strong(is_active && several);
        if several {
            row = row.tick(setup.enabled);
        }
        let out = row.show(ui);
        if out.toggled {
            toggle_folder(ui, open_id, true);
        }
        // A part or an instance dropped here (docs/specs/parts-and-setups.md §8).
        if let Some(part) = out.response.dnd_release_payload::<DraggedPart>() {
            let copy = ui.input(|i| i.modifiers.command);
            action = Some(part_dropped(part.0, setup.id, copy));
        } else if let Some(i) = out.response.dnd_release_payload::<DraggedInstance>() {
            action = Some(SetupAction::MoveInstance(i.0, i.1, setup.id));
        } else if out.response.dnd_hover_payload::<DraggedPart>().is_some()
            || out
                .response
                .dnd_hover_payload::<DraggedInstance>()
                .is_some()
        {
            ui.painter().rect_stroke(
                out.response.rect,
                0.0,
                egui::Stroke::new(2.0, ui.visuals().selection.stroke.color),
                egui::StrokeKind::Inside,
            );
        }
        if out.tick {
            action = Some(SetupAction::Toggle(setup.id));
        } else if out.response.clicked() {
            action = Some(SetupAction::Open(setup.id));
        }
        let mut hover = if is_active {
            t!(ui, "browser-setup-active-hover")
        } else {
            t!(ui, "browser-setup-hover")
        };
        if missing {
            hover = format!("⚠ {}\n\n{hover}", t!(ui, "browser-setup-no-dowels-hover"));
        }
        out.response.on_hover_text(hover).context_menu(|ui| {
            if ui
                .button(t!(ui, "browser-export-setup"))
                .on_hover_text(t!(ui, "browser-export-setup-hover"))
                .clicked()
            {
                action = Some(SetupAction::Export(setup.id));
            }
            ui.separator();
            if ui
                .button(t!(ui, "browser-face-board"))
                .on_hover_text(t!(ui, "browser-face-board-hover"))
                .clicked()
            {
                action = Some(SetupAction::FaceBoard(setup.id));
            }
            if ui
                .button(t!(ui, "nest-menu"))
                .on_hover_text(t!(ui, "nest-menu-hover"))
                .clicked()
            {
                action = Some(SetupAction::Nest(setup.id));
            }
            if self.was_nested(setup.id)
                && ui
                    .button(t!(ui, "nest-again"))
                    .on_hover_text(t!(ui, "nest-again-hover"))
                    .clicked()
            {
                action = Some(SetupAction::NestAgain(setup.id));
            }
            if ui.button(t!(ui, "browser-new-part")).clicked() {
                action = Some(SetupAction::NewPart(setup.id));
            }
            if ui
                .button(t!(ui, "browser-add-setup"))
                .on_hover_text(t!(ui, "browser-add-setup-hover"))
                .clicked()
            {
                action = Some(SetupAction::Add);
            }
            if ui
                .add_enabled(can_turn, egui::Button::new(t!(ui, "browser-add-back-side")))
                .on_hover_text(t!(ui, "browser-add-back-side-hover"))
                .clicked()
            {
                action = Some(SetupAction::AddBack(setup.id));
            }
            ui.separator();
            // The last board isn't deleted: it can go back to being the project's.
            if several {
                if ui.button(t!(ui, "browser-delete")).clicked() {
                    action = Some(SetupAction::Remove(setup.id));
                }
            } else if ui
                .add_enabled(
                    self.edit.as_ref().is_some_and(|e| e.job.is_simple()),
                    egui::Button::new(t!(ui, "browser-simple-view")),
                )
                .on_hover_text(t!(ui, "browser-simple-view-hover"))
                .on_disabled_hover_text(t!(ui, "browser-simple-view-not-hover"))
                .clicked()
            {
                action = Some(SetupAction::ShowSetups(false));
            }
        });
        (setup_open, action)
    }

    /// The project's menu when it has one board (docs/specs/parts-and-setups.md §2a): what
    /// the setup's row offers with several, its own settings (pieces, back side) included.
    pub(super) fn board_menu(&self, ui: &mut egui::Ui, id: u64) -> Option<SetupAction> {
        let mut action = None;
        if ui.button(t!(ui, "browser-new-part")).clicked() {
            action = Some(SetupAction::NewPart(id));
        }
        if ui
            .button(t!(ui, "browser-face-board"))
            .on_hover_text(t!(ui, "browser-face-board-hover"))
            .clicked()
        {
            action = Some(SetupAction::FaceBoard(id));
        }
        if ui
            .button(t!(ui, "nest-menu"))
            .on_hover_text(t!(ui, "nest-menu-hover"))
            .clicked()
        {
            action = Some(SetupAction::Nest(id));
        }
        if self.was_nested(id)
            && ui
                .button(t!(ui, "nest-again"))
                .on_hover_text(t!(ui, "nest-again-hover"))
                .clicked()
        {
            action = Some(SetupAction::NestAgain(id));
        }
        ui.separator();
        if ui
            .button(t!(ui, "browser-board-settings"))
            .on_hover_text(t!(ui, "browser-board-settings-hover"))
            .clicked()
        {
            action = Some(SetupAction::Open(id));
        }
        if ui
            .button(t!(ui, "browser-add-back-side"))
            .on_hover_text(t!(ui, "browser-add-back-side-hover"))
            .clicked()
        {
            action = Some(SetupAction::AddBack(id));
        }
        if ui
            .button(t!(ui, "browser-add-setup"))
            .on_hover_text(t!(ui, "browser-add-setup-hover"))
            .clicked()
        {
            action = Some(SetupAction::Add);
        }
        if ui
            .button(t!(ui, "browser-show-setups"))
            .on_hover_text(t!(ui, "browser-show-setups-hover"))
            .clicked()
        {
            action = Some(SetupAction::ShowSetups(true));
        }
        action
    }

    /// Do what a setup's row asked.
    pub(super) fn apply_setup_action(&mut self, action: SetupAction) {
        match action {
            SetupAction::Add => {
                self.add_setup();
            }
            SetupAction::AddBack(id) => self.start_two_sided_for(id),
            SetupAction::Toggle(id) => self.toggle_setup(id),
            SetupAction::Open(id) => {
                self.activate_setup(id);
                self.focus = Focus::Setup(id);
            }
            SetupAction::Stock(id) => {
                self.activate_setup(id);
                self.focus = Focus::Stock;
            }
            SetupAction::Zero(id) => {
                self.activate_setup(id);
                self.focus = Focus::WorkZero;
            }
            SetupAction::NewPart(id) => {
                self.activate_setup(id);
                self.add_part();
            }
            SetupAction::Remove(id) => self.remove_setup(id),
            SetupAction::Export(id) => self.ask_export_setup(id),
            SetupAction::ExportAll => self.ask_export_all(),
            SetupAction::FaceBoard(id) => {
                self.face_board(id);
            }
            SetupAction::Nest(id) => self.start_nest(Some(id)),
            SetupAction::NestAgain(id) => self.nest_again(id),
            SetupAction::MovePart(part, setup) => self.move_part_to_setup(part, setup),
            SetupAction::PlacePart(part, setup) => self.place_part_on(part, setup),
            SetupAction::MoveInstance(part, id, setup) => self.instance_to_setup(part, id, setup),
            SetupAction::ShowSetups(shown) => self.show_setups(shown),
        }
    }

    /// Setup `id`'s board, its thickness and material beside it, a leaf under the setup
    /// above its work zero (the user, 2026-11-03). A click opens it, its setup made the
    /// active one first.
    pub(super) fn stock_row(
        &mut self,
        ui: &mut egui::Ui,
        id: u64,
        depth: usize,
    ) -> Option<SetupAction> {
        let edit = self.edit.as_ref()?;
        let job = edit.job.for_setup(id);
        let active = edit.job.active_setup_id() == Some(id);
        let summary = super::forms::stock_summary(ui, &job);
        let problems = if active {
            self.stock_problems().count()
        } else {
            0
        };
        let row = Row::new(
            Id::new(("browser-stock", id)),
            depth,
            t!(ui, "browser-stock"),
        )
        .icon_tinted(Icon::Material, Tint::Board)
        .trailing(summary)
        .selected(active && self.focus == Focus::Stock)
        .dim(!active)
        .alert(problems > 0)
        .show(ui);
        row.response
            .on_hover_text(t!(ui, "browser-stock-hover"))
            .clicked()
            .then_some(SetupAction::Stock(id))
    }

    /// Setup `id`'s G-code zero, with where it is (docs/specs/work-zero-marker.md §2), a
    /// leaf under the setup (docs/specs/setups.md §7). A click opens it, its setup made the
    /// active one first.
    pub(super) fn work_zero_row(
        &mut self,
        ui: &mut egui::Ui,
        id: u64,
        depth: usize,
    ) -> Option<SetupAction> {
        let edit = self.edit.as_ref()?;
        let job = edit.job.for_setup(id);
        let active = edit.job.active_setup_id() == Some(id);
        let place = super::forms::work_zero_name(ui, &job.work_zero());
        let summary = super::forms::zero_summary(ui, &job);
        let problems = if active {
            self.zero_problems().count()
        } else {
            0
        };
        let row = Row::new(
            Id::new(("browser-work-zero", id)),
            depth,
            t!(ui, "browser-work-zero"),
        )
        .icon(Icon::WorkZero)
        .trailing(place)
        .selected(active && self.focus == Focus::WorkZero)
        .dim(!active)
        .alert(problems > 0)
        .show(ui);
        let hover = format!("{summary}\n{}", t!(ui, "browser-work-zero-hover"));
        row.response
            .on_hover_text(hover)
            .clicked()
            .then_some(SetupAction::Zero(id))
    }
}
