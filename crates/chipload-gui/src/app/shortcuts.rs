// SPDX-License-Identifier: GPL-3.0-or-later
//! Every keyboard shortcut in one table: the key handling, the menu labels and Help →
//! Keyboard shortcuts all read it, so the help can't fall out of date
//! (docs/specs/small-items.md §1).

use eframe::egui::{Key, KeyboardShortcut, ModifierNames, Modifiers};

use super::*;

/// What a shortcut does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Action {
    New,
    Open,
    Import,
    Save,
    SaveAs,
    Export,
    Quit,
    Undo,
    Redo,
    CopySelector,
    Copy,
    Paste,
    Generate,
    MoveUp,
    MoveDown,
    Fit,
    TopView,
    Grid,
    Measure,
    Panels,
    Shortcuts,
    DrawPolyline,
    DrawCircle,
    DrawRectangle,
    DrawArc,
    DrawSlot,
    DrawPolygon,
    Offset,
}

/// The shortcut window's groups, in the order it shows them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum Group {
    File,
    Edit,
    Generate,
    View,
    Help,
    Canvas,
    Drawing,
}

impl Group {
    const ALL: [Group; 7] = [
        Group::File,
        Group::Edit,
        Group::Generate,
        Group::View,
        Group::Help,
        Group::Canvas,
        Group::Drawing,
    ];

    /// The group's heading in the shortcut window.
    pub(super) fn name(self, src: &impl crate::i18n::Lang) -> String {
        match self {
            Group::File => t!(src, "shortcuts-group-file"),
            Group::Edit => t!(src, "shortcuts-group-edit"),
            Group::Generate => t!(src, "shortcuts-group-generate"),
            Group::View => t!(src, "shortcuts-group-view"),
            Group::Help => t!(src, "shortcuts-group-help"),
            Group::Canvas => t!(src, "shortcuts-group-canvas"),
            Group::Drawing => t!(src, "shortcuts-group-drawing"),
        }
    }
}

pub(super) struct Binding {
    pub action: Action,
    pub keys: KeyboardShortcut,
    pub group: Group,
}

const fn bind(action: Action, modifiers: Modifiers, key: Key, group: Group) -> Binding {
    Binding {
        action,
        keys: KeyboardShortcut::new(modifiers, key),
        group,
    }
}

const CTRL: Modifiers = Modifiers::COMMAND;
const CTRL_SHIFT: Modifiers = Modifiers::COMMAND.plus(Modifiers::SHIFT);

/// The shortcuts, in the order they are tried: one with Shift before the same without, as
/// egui would take Ctrl+Shift+Z for Ctrl+Z too.
pub(super) const BINDINGS: &[Binding] = &[
    bind(Action::CopySelector, CTRL_SHIFT, Key::C, Group::Edit),
    bind(Action::SaveAs, CTRL_SHIFT, Key::S, Group::File),
    bind(Action::Redo, CTRL_SHIFT, Key::Z, Group::Edit),
    bind(Action::Copy, CTRL, Key::C, Group::Edit),
    bind(Action::Paste, CTRL, Key::V, Group::Edit),
    bind(Action::New, CTRL, Key::N, Group::File),
    bind(Action::Open, CTRL, Key::O, Group::File),
    bind(Action::Import, CTRL, Key::I, Group::File),
    bind(Action::Save, CTRL, Key::S, Group::File),
    bind(Action::Export, CTRL, Key::E, Group::File),
    bind(Action::Quit, CTRL, Key::Q, Group::File),
    bind(Action::Undo, CTRL, Key::Z, Group::Edit),
    bind(Action::MoveUp, Modifiers::ALT, Key::ArrowUp, Group::Edit),
    bind(
        Action::MoveDown,
        Modifiers::ALT,
        Key::ArrowDown,
        Group::Edit,
    ),
    bind(Action::Generate, Modifiers::NONE, Key::F5, Group::Generate),
    bind(Action::Fit, Modifiers::NONE, Key::F, Group::View),
    bind(Action::TopView, Modifiers::NONE, Key::T, Group::View),
    bind(Action::Grid, Modifiers::NONE, Key::G, Group::View),
    bind(Action::Measure, Modifiers::NONE, Key::M, Group::View),
    bind(Action::Panels, Modifiers::NONE, Key::P, Group::View),
    bind(Action::Shortcuts, Modifiers::NONE, Key::F1, Group::Help),
    bind(
        Action::DrawPolyline,
        Modifiers::NONE,
        Key::L,
        Group::Drawing,
    ),
    bind(Action::DrawCircle, Modifiers::NONE, Key::C, Group::Drawing),
    bind(
        Action::DrawRectangle,
        Modifiers::NONE,
        Key::R,
        Group::Drawing,
    ),
    bind(Action::DrawArc, Modifiers::NONE, Key::A, Group::Drawing),
    bind(Action::DrawSlot, Modifiers::NONE, Key::S, Group::Drawing),
    bind(Action::DrawPolygon, Modifiers::NONE, Key::N, Group::Drawing),
    bind(Action::Offset, Modifiers::NONE, Key::O, Group::Drawing),
];

/// What a shortcut does, as the shortcut window says it.
pub(super) fn what(action: Action, src: &impl crate::i18n::Lang) -> String {
    match action {
        Action::CopySelector => t!(src, "shortcuts-copy-selector"),
        Action::SaveAs => t!(src, "shortcuts-save-as"),
        Action::Redo => t!(src, "shortcuts-redo"),
        Action::Copy => t!(src, "shortcuts-copy"),
        Action::Paste => t!(src, "shortcuts-paste"),
        Action::New => t!(src, "shortcuts-new"),
        Action::Open => t!(src, "shortcuts-open"),
        Action::Import => t!(src, "shortcuts-import"),
        Action::Save => t!(src, "shortcuts-save"),
        Action::Export => t!(src, "shortcuts-export"),
        Action::Quit => t!(src, "shortcuts-quit"),
        Action::Undo => t!(src, "shortcuts-undo"),
        Action::MoveUp => t!(src, "shortcuts-move-up"),
        Action::MoveDown => t!(src, "shortcuts-move-down"),
        Action::Generate => t!(src, "shortcuts-generate"),
        Action::Fit => t!(src, "shortcuts-fit"),
        Action::TopView => t!(src, "shortcuts-top-view"),
        Action::Grid => t!(src, "shortcuts-grid"),
        Action::Measure => t!(src, "shortcuts-measure"),
        Action::Panels => t!(src, "shortcuts-panels"),
        Action::Shortcuts => t!(src, "shortcuts-this-list"),
        Action::DrawPolyline => t!(src, "shortcuts-draw-polyline"),
        Action::DrawCircle => t!(src, "shortcuts-draw-circle"),
        Action::DrawRectangle => t!(src, "shortcuts-draw-rectangle"),
        Action::DrawArc => t!(src, "shortcuts-draw-arc"),
        Action::DrawSlot => t!(src, "shortcuts-draw-slot"),
        Action::DrawPolygon => t!(src, "shortcuts-draw-polygon"),
        Action::Offset => t!(src, "shortcuts-offset"),
    }
}

/// Keys and mouse actions that aren't shortcuts of a menu item: (group, keys, what).
pub(super) fn others(src: &impl crate::i18n::Lang) -> Vec<(Group, String, String)> {
    use Group::{Canvas, Drawing};
    vec![
        (
            Canvas,
            t!(src, "shortcuts-click"),
            t!(src, "shortcuts-click-select"),
        ),
        (
            Canvas,
            t!(src, "shortcuts-ctrl-click"),
            t!(src, "shortcuts-ctrl-click-what"),
        ),
        (
            Canvas,
            t!(src, "shortcuts-right-click"),
            t!(src, "shortcuts-right-click-what"),
        ),
        (
            Canvas,
            t!(src, "shortcuts-scroll"),
            t!(src, "shortcuts-scroll-what"),
        ),
        (
            Canvas,
            t!(src, "shortcuts-drag"),
            t!(src, "shortcuts-drag-what"),
        ),
        (
            Canvas,
            t!(src, "shortcuts-drag-shape"),
            t!(src, "shortcuts-drag-shape-what"),
        ),
        (
            Canvas,
            t!(src, "shortcuts-pan-drag"),
            t!(src, "shortcuts-pan-drag-what"),
        ),
        (
            Canvas,
            t!(src, "shortcuts-orbit-drag"),
            t!(src, "shortcuts-orbit-drag-what"),
        ),
        (
            Canvas,
            t!(src, "shortcuts-delete"),
            t!(src, "shortcuts-delete-what"),
        ),
        (
            Canvas,
            t!(src, "shortcuts-escape"),
            t!(src, "shortcuts-escape-clear"),
        ),
        (
            Canvas,
            t!(src, "shortcuts-drag-start"),
            t!(src, "shortcuts-drag-start-what"),
        ),
        (
            Canvas,
            t!(src, "shortcuts-drop"),
            t!(src, "shortcuts-drop-what"),
        ),
        (
            Drawing,
            t!(src, "shortcuts-click"),
            t!(src, "shortcuts-click-point"),
        ),
        (
            Drawing,
            t!(src, "shortcuts-type-value"),
            t!(src, "shortcuts-type-value-what"),
        ),
        (
            Drawing,
            t!(src, "shortcuts-shift"),
            t!(src, "shortcuts-shift-what"),
        ),
        (
            Drawing,
            t!(src, "shortcuts-alt"),
            t!(src, "shortcuts-alt-what"),
        ),
        (
            Drawing,
            t!(src, "shortcuts-enter"),
            t!(src, "shortcuts-enter-what"),
        ),
        (
            Drawing,
            t!(src, "shortcuts-backspace"),
            t!(src, "shortcuts-backspace-what"),
        ),
        (
            Drawing,
            t!(src, "shortcuts-escape"),
            t!(src, "shortcuts-escape-stop"),
        ),
        (
            Drawing,
            t!(src, "shortcuts-drag-handle"),
            t!(src, "shortcuts-drag-handle-what"),
        ),
        (
            Drawing,
            t!(src, "shortcuts-drag-middle"),
            t!(src, "shortcuts-drag-middle-what"),
        ),
        (
            Drawing,
            t!(src, "shortcuts-shift-drag-middle"),
            t!(src, "shortcuts-shift-drag-middle-what"),
        ),
    ]
}

pub(super) fn binding(action: Action) -> Option<&'static Binding> {
    BINDINGS.iter().find(|b| b.action == action)
}

/// "Ctrl+Shift+Z".
pub(super) fn keys_text(keys: &KeyboardShortcut) -> String {
    keys.format(&ModifierNames::NAMES, false)
}

/// A menu item's label with its shortcut: "Undo  (Ctrl+Z)".
pub(super) fn with_keys(label: &str, action: Action) -> String {
    match binding(action) {
        Some(b) => format!("{label}  ({})", keys_text(&b.keys)),
        None => label.to_string(),
    }
}

/// A hover text with the shortcut: "Save the project (Ctrl+S)".
pub(super) fn hover_keys(text: &str, action: Action) -> String {
    match binding(action) {
        Some(b) => format!("{text} ({})", keys_text(&b.keys)),
        None => text.to_string(),
    }
}

impl ChiploadApp {
    /// Do what a shortcut asks.
    pub(super) fn act(&mut self, action: Action, ctx: &egui::Context) {
        match action {
            Action::New => self.ask_new(),
            Action::Open => self.ask_open(),
            Action::Import => self.ask_import(),
            Action::Save => {
                self.ask_save(false);
            }
            Action::SaveAs => {
                self.ask_save(true);
            }
            Action::Export => self.export_when_ready(),
            Action::Quit => ctx.send_viewport_cmd(ViewportCommand::Close),
            Action::Undo => self.undo(),
            Action::Redo => self.redo(),
            Action::CopySelector => self.copy_selector(ctx),
            Action::Copy => {
                if let Some(text) = self.copy_selected_shapes() {
                    ctx.copy_text(text);
                }
            }
            Action::Paste => {
                self.paste(None);
            }
            Action::Generate => {
                if self.can_generate() {
                    self.generate();
                }
            }
            Action::MoveUp => {
                if let Focus::Op(i) = self.focus
                    && i > 0
                {
                    self.move_operation(i, i - 1);
                }
            }
            Action::MoveDown => {
                if let Focus::Op(i) = self.focus {
                    self.move_operation(i, i + 1);
                }
            }
            Action::Fit => self.fit_pending = true,
            Action::TopView => self.view_top(),
            Action::Grid => self.set_show_grid(!self.show_grid),
            Action::Measure => self.toggle_measure(),
            Action::Panels => self.toggle_panels(),
            Action::DrawPolyline => self.start_drawing(Tool::Polyline),
            Action::DrawCircle => self.start_drawing(Tool::Circle),
            Action::DrawRectangle => self.start_drawing(Tool::Rectangle),
            Action::DrawArc => self.start_drawing(Tool::Arc),
            Action::DrawSlot => self.start_drawing(Tool::Slot),
            Action::DrawPolygon => self.start_drawing(Tool::Polygon),
            Action::Offset => self.start_modify(super::modify::ModifyTool::Offset),
            Action::Shortcuts => self.dialogs.shortcuts_open = true,
        }
    }

    /// Help → Keyboard shortcuts.
    pub(super) fn shortcuts_window(&mut self, ctx: &egui::Context) {
        let mut open = self.dialogs.shortcuts_open;
        egui::Window::new(t!(ctx, "shortcuts-window"))
            .open(&mut open)
            .default_size([520.0, 560.0])
            .show(ctx, |ui| {
                ScrollArea::vertical().show(ui, |ui| {
                    for (group, rows) in shortcut_rows(ui) {
                        ui.add_space(4.0);
                        ui.strong(group.name(ui));
                        egui::Grid::new(("shortcuts", group))
                            .num_columns(2)
                            .spacing([16.0, 2.0])
                            .striped(true)
                            .show(ui, |ui| {
                                for (keys, what) in rows {
                                    ui.monospace(keys);
                                    ui.label(what);
                                    ui.end_row();
                                }
                            });
                    }
                });
            });
        self.dialogs.shortcuts_open = open;
    }
}

/// What the window lists: each group with its (keys, what) rows.
pub(super) fn shortcut_rows(src: &impl crate::i18n::Lang) -> Vec<(Group, Vec<(String, String)>)> {
    let others = others(src);
    Group::ALL
        .iter()
        .map(|&group| {
            let mut rows: Vec<(String, String)> = BINDINGS
                .iter()
                .filter(|b| b.group == group)
                .map(|b| (keys_text(&b.keys), what(b.action, src)))
                .collect();
            rows.extend(
                others
                    .iter()
                    .filter(|(g, _, _)| *g == group)
                    .map(|(_, keys, what)| (keys.clone(), what.clone())),
            );
            (group, rows)
        })
        .filter(|(_, rows)| !rows.is_empty())
        .collect()
}
