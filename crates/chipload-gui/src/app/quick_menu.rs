// SPDX-License-Identifier: GPL-3.0-or-later
//! Fewer clicks (docs/specs/fewer-clicks.md): the Q menu (§2): with shapes selected, Q
//! opens a small menu at the mouse that adds an operation for the selection by a letter, or
//! applies a style by a digit; its letters work only while it is open. And selecting by
//! kind (§3): *Select similar* and *Select inside* from the canvas' right-click menu.

use std::collections::BTreeSet;

use chipload_pipeline::new_ops::OpKind;
use chipload_pipeline::selection::{Unfit, unfit_for};

use super::styles::StyleAct;
use super::*;

/// The kinds the menu offers, with their keys, in its order.
pub(super) const KINDS: [(Key, &str, OpKind); 6] = [
    (Key::P, "P", OpKind::Profile),
    (Key::K, "K", OpKind::Pocket),
    (Key::H, "H", OpKind::Holes),
    (Key::V, "V", OpKind::Vcarve),
    (Key::F, "F", OpKind::Path),
    (Key::C, "C", OpKind::Chamfer),
];

/// The digit keys of the first nine styles.
const DIGITS: [Key; 9] = [
    Key::Num1,
    Key::Num2,
    Key::Num3,
    Key::Num4,
    Key::Num5,
    Key::Num6,
    Key::Num7,
    Key::Num8,
    Key::Num9,
];

/// What the menu was asked for.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum QuickChoice {
    Kind(OpKind),
    /// A style, by uid.
    Style(String),
}

impl ChiploadApp {
    /// *Select similar* on contour `id` (docs/specs/fewer-clicks.md §3): replaces the
    /// selection, or adds to it.
    pub fn select_similar(&mut self, id: usize, add: bool) {
        let found = chipload_pipeline::selection::similar(&self.contours, id);
        self.select_found(found, add);
    }

    /// *Select inside* on closed contour `id`: the closed shapes within it.
    pub fn select_inside(&mut self, id: usize, add: bool) {
        let found = chipload_pipeline::selection::inside(&self.contours, id).unwrap_or_default();
        self.select_found(found, add);
    }

    /// Whether contour `id` is closed, so *Select inside* is offered.
    pub(super) fn has_inside(&self, id: usize) -> bool {
        self.contours
            .iter()
            .any(|c| c.id == id && c.closed && c.invalid_reason.is_none())
    }

    fn select_found(&mut self, mut found: BTreeSet<usize>, add: bool) {
        if add {
            found.extend(self.selected_ids());
        }
        self.selection = CanvasSelection::of_contours(&found, &self.contours, &self.twins);
        self.focus = Focus::Selection;
    }

    /// Q: open the menu at the mouse, or say on the status bar that shapes come first.
    pub(super) fn open_quick_menu(&mut self, ctx: &egui::Context) {
        if self.edit.is_none() {
            return;
        }
        if self.selection == CanvasSelection::Nothing {
            self.set_status(t!(self, "quick-select-first"), STATUS_SECONDS);
            return;
        }
        let at = ctx
            .input(|i| i.pointer.latest_pos())
            .unwrap_or_else(|| ctx.content_rect().center());
        self.quick_menu = Some(at);
    }

    /// Whether the menu is open.
    pub(super) fn quick_menu_open(&self) -> bool {
        self.quick_menu.is_some()
    }

    /// Why the selection can't take an operation of `kind`, for the greyed entry's hover;
    /// None when it can.
    pub(super) fn quick_unfit(&self, kind: OpKind) -> Option<String> {
        let ids: BTreeSet<usize> = self
            .op_selection()
            .ids(&self.contours, self.edit.as_ref().map(|e| &e.job.drawing));
        unfit_for(kind, &ids, &self.contours).map(|why| match why {
            Unfit::Nothing => t!(self, "header-select-shapes-first"),
            Unfit::NoCircles => t!(self, "quick-needs-circles"),
            Unfit::NoClosed => t!(self, "quick-needs-closed"),
        })
    }

    /// The menu's keys while it is open: a letter or digit chooses, Escape closes. Every
    /// other shortcut waits until it is closed, so P is a profile here and not Panels.
    pub(super) fn quick_menu_keys(&mut self, ctx: &egui::Context) {
        if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
            self.quick_menu = None;
            return;
        }
        let mut chosen = None;
        for (key, _, kind) in KINDS {
            if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, key)) {
                chosen = Some(QuickChoice::Kind(kind));
            }
        }
        let styles: Vec<String> = self
            .user
            .library
            .styles
            .styles
            .iter()
            .map(|s| s.uid.clone())
            .collect();
        for (key, uid) in DIGITS.iter().zip(styles) {
            if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, *key)) {
                chosen = Some(QuickChoice::Style(uid));
            }
        }
        if let Some(choice) = chosen {
            self.quick_choose(choice);
        }
    }

    /// Do what the menu was asked for and close it. A kind the selection can't take is
    /// left alone, with the reason on the status bar.
    pub(super) fn quick_choose(&mut self, choice: QuickChoice) {
        match choice {
            QuickChoice::Kind(kind) => {
                if let Some(why) = self.quick_unfit(kind) {
                    self.set_status(why, STATUS_SECONDS);
                    return;
                }
                self.quick_menu = None;
                self.add_operation(kind);
            }
            QuickChoice::Style(uid) => {
                self.quick_menu = None;
                self.style_act(StyleAct::Apply(uid));
            }
        }
    }

    /// The menu, drawn over the canvas while open.
    pub(super) fn quick_menu_ui(&mut self, ui: &egui::Ui) {
        let Some(at) = self.quick_menu else {
            return;
        };
        if self.edit.is_none() || self.selection == CanvasSelection::Nothing {
            self.quick_menu = None;
            return;
        }
        let kinds: Vec<(String, &str, OpKind, Option<String>)> = {
            let labels = super::browser::new_operations(ui);
            KINDS
                .iter()
                .map(|(_, key, kind)| {
                    let label = labels
                        .iter()
                        .find(|(_, k, _)| k == kind)
                        .map(|(l, _, _)| l.clone())
                        .unwrap_or_default();
                    (label, *key, *kind, self.quick_unfit(*kind))
                })
                .collect()
        };
        let styles: Vec<(String, String)> = self
            .user
            .library
            .styles
            .styles
            .iter()
            .map(|s| (s.uid.clone(), s.name.clone()))
            .collect();
        let mut open = true;
        let mut chosen = None;
        egui::Popup::new(
            egui::Id::new("quick-menu"),
            ui.ctx().clone(),
            egui::PopupAnchor::Position(at),
            ui.layer_id(),
        )
        .kind(egui::PopupKind::Menu)
        .layout(egui::Layout::top_down_justified(egui::Align::Min))
        .open_bool(&mut open)
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .show(|ui| {
            ui.weak(t!(ui, "quick-title"));
            for (label, key, kind, unfit) in &kinds {
                let button = egui::Button::new(format!("{key}   {label}"));
                let response = ui.add_enabled(unfit.is_none(), button);
                let response = match unfit {
                    Some(why) => response.on_disabled_hover_text(why),
                    None => response,
                };
                if response.clicked() {
                    chosen = Some(QuickChoice::Kind(*kind));
                }
            }
            if !styles.is_empty() {
                ui.separator();
            }
            for (i, (uid, name)) in styles.iter().enumerate() {
                let text = match i {
                    0..9 => format!("{}   {name}", i + 1),
                    _ => format!("     {name}"),
                };
                if ui.button(text).clicked() {
                    chosen = Some(QuickChoice::Style(uid.clone()));
                }
            }
        });
        if !open {
            self.quick_menu = None;
        }
        if let Some(choice) = chosen {
            self.quick_choose(choice);
        }
    }
}
