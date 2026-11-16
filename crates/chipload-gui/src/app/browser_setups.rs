// SPDX-License-Identifier: GPL-3.0-or-later
//! The Browser's setups: each with its stock, work zero and parts (docs/specs/setups.md §7).

use eframe::egui::Id;

use super::browser::tool_changes;
use super::browser_ops::{OpAction, PartRows};
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

/// What a setup's rows ask for.
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
    /// A board surface first in this setup (docs/specs/surface-levels.md §5).
    FaceBoard(u64),
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
        let row = Row::new(Id::new("browser-setups"), 1, t!(ui, "browser-setups"))
            .icon_tinted(Icon::Rectangle, Tint::Board)
            .trailing(setups.len().to_string());
        let (open, folder_row) = folder(ui, row, true);
        let mut setup_action: Option<SetupAction> = None;
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
            });
        if open {
            for setup in &setups {
                let is_active = active_setup == Some(setup.id);
                let mine: Vec<chipload_job::Part> = parts
                    .iter()
                    .filter(|p| p.setup == setup.id)
                    .cloned()
                    .collect();
                let rows = SetupRows {
                    setups: &setups,
                    parts: mine.len(),
                    missing: missing_dowels.contains(&setup.id),
                    is_active,
                };
                let (setup_open, asked) = self.setup_row(ui, setup, rows);
                if asked.is_some() {
                    setup_action = asked;
                }
                if !setup_open {
                    continue;
                }
                if let Some(a) = self.stock_row(ui, setup.id) {
                    setup_action = Some(a);
                }
                if let Some(a) = self.work_zero_row(ui, setup.id) {
                    setup_action = Some(a);
                }
                // The setup's board surfaces, before its parts (surface-levels.md §5).
                for i in
                    (0..count).filter(|&i| board[i] && mine.iter().any(|p| p.id == op_parts[i]))
                {
                    let changes = tool_before.get(&i).map_or(&[][..], Vec::as_slice);
                    if let Some(a) = self.operation_row(ui, i, count, changes, 3) {
                        action = Some(a);
                    }
                }
                for part in &mine {
                    let k = parts.iter().position(|p| p.id == part.id).unwrap_or(0);
                    let rows = PartRows {
                        k,
                        parts: &parts,
                        active,
                        op_parts: &op_parts,
                        board: &board,
                        count,
                        tool_before: &tool_before,
                    };
                    if let Some(a) = self.part_rows(ui, part, rows) {
                        action = Some(a);
                    }
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
            if ui
                .add_enabled(several, egui::Button::new(t!(ui, "browser-delete")))
                .clicked()
            {
                action = Some(SetupAction::Remove(setup.id));
            }
        });
        (setup_open, action)
    }

    /// Do what a setup's row asked.
    fn apply_setup_action(&mut self, action: SetupAction) {
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
            SetupAction::FaceBoard(id) => {
                self.face_board(id);
            }
        }
    }

    /// Setup `id`'s board, its thickness and material beside it, a leaf under the setup
    /// above its work zero (the user, 2026-11-03). A click opens it, its setup made the
    /// active one first.
    pub(super) fn stock_row(&mut self, ui: &mut egui::Ui, id: u64) -> Option<SetupAction> {
        let edit = self.edit.as_ref()?;
        let job = edit.job.for_setup(id);
        let active = edit.job.active_setup_id() == Some(id);
        let summary = super::forms::stock_summary(ui, &job);
        let problems = if active {
            self.stock_problems().count()
        } else {
            0
        };
        let row = Row::new(Id::new(("browser-stock", id)), 3, t!(ui, "browser-stock"))
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
    pub(super) fn work_zero_row(&mut self, ui: &mut egui::Ui, id: u64) -> Option<SetupAction> {
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
            3,
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
