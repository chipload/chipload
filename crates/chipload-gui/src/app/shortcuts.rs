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
}

pub(super) struct Binding {
    pub action: Action,
    pub keys: KeyboardShortcut,
    pub group: &'static str,
    pub what: &'static str,
}

const fn bind(
    action: Action,
    modifiers: Modifiers,
    key: Key,
    group: &'static str,
    what: &'static str,
) -> Binding {
    Binding {
        action,
        keys: KeyboardShortcut::new(modifiers, key),
        group,
        what,
    }
}

const CTRL: Modifiers = Modifiers::COMMAND;
const CTRL_SHIFT: Modifiers = Modifiers::COMMAND.plus(Modifiers::SHIFT);

/// The shortcuts, in the order they are tried: one with Shift before the same without, as
/// egui would take Ctrl+Shift+Z for Ctrl+Z too.
pub(super) const BINDINGS: &[Binding] = &[
    bind(
        Action::CopySelector,
        CTRL_SHIFT,
        Key::C,
        "Edit",
        "Copy the selection as a selector (JSON)",
    ),
    bind(Action::SaveAs, CTRL_SHIFT, Key::S, "File", "Save as"),
    bind(Action::Redo, CTRL_SHIFT, Key::Z, "Edit", "Redo"),
    bind(
        Action::Copy,
        CTRL,
        Key::C,
        "Edit",
        "Copy the selected drawn shapes",
    ),
    bind(
        Action::Paste,
        CTRL,
        Key::V,
        "Edit",
        "Paste them (at the cursor, over the canvas)",
    ),
    bind(Action::New, CTRL, Key::N, "File", "New project"),
    bind(Action::Open, CTRL, Key::O, "File", "Open a project"),
    bind(
        Action::Import,
        CTRL,
        Key::I,
        "File",
        "New project from a DXF",
    ),
    bind(Action::Save, CTRL, Key::S, "File", "Save"),
    bind(
        Action::Export,
        CTRL,
        Key::E,
        "File",
        "Export G-code (once generated)",
    ),
    bind(Action::Quit, CTRL, Key::Q, "File", "Quit"),
    bind(Action::Undo, CTRL, Key::Z, "Edit", "Undo"),
    bind(
        Action::MoveUp,
        Modifiers::ALT,
        Key::ArrowUp,
        "Edit",
        "Move the operation up",
    ),
    bind(
        Action::MoveDown,
        Modifiers::ALT,
        Key::ArrowDown,
        "Edit",
        "Move the operation down",
    ),
    bind(
        Action::Generate,
        Modifiers::NONE,
        Key::F5,
        "Generate",
        "Generate the toolpath and G-code",
    ),
    bind(Action::Fit, Modifiers::NONE, Key::F, "View", "Fit the view"),
    bind(Action::TopView, Modifiers::NONE, Key::T, "View", "Top view"),
    bind(
        Action::Grid,
        Modifiers::NONE,
        Key::G,
        "View",
        "Show or hide the grid",
    ),
    bind(
        Action::Measure,
        Modifiers::NONE,
        Key::M,
        "View",
        "Measure: two clicks give the distance and angle",
    ),
    bind(
        Action::Panels,
        Modifiers::NONE,
        Key::P,
        "View",
        "Fold or open the side panels, for a larger canvas",
    ),
    bind(
        Action::Shortcuts,
        Modifiers::NONE,
        Key::F1,
        "Help",
        "This list",
    ),
];

/// Keys and mouse actions that aren't shortcuts of a menu item: (group, keys, what).
pub(super) const OTHERS: &[(&str, &str, &str)] = &[
    ("Canvas", "Click", "Select a contour"),
    (
        "Canvas",
        "Ctrl+click",
        "Add a contour to the selection, or take it out",
    ),
    (
        "Canvas",
        "Right-click",
        "Select a contour or its layer; reset a moved ring start",
    ),
    ("Canvas", "Scroll", "Zoom at the cursor"),
    ("Canvas", "Drag", "Pan (top view); orbit (3D view)"),
    (
        "Canvas",
        "Right or middle drag, Shift+drag",
        "Pan (3D view)",
    ),
    ("Canvas", "Delete", "Delete the selected drawn shapes"),
    ("Canvas", "Escape", "Clear the selection"),
    (
        "Canvas",
        "Drag a ring's start",
        "Move where the ring starts",
    ),
    (
        "Canvas",
        "Drop a file",
        "Open a .chipload project, or start one from a .dxf",
    ),
    (
        "Drawing",
        "Click",
        "Place a point (snaps to ends, centres and middles)",
    ),
    ("Drawing", "Alt", "Hold: don't snap"),
    ("Drawing", "Enter, double-click", "Finish a polyline"),
    ("Drawing", "Backspace", "Take back the last point"),
    ("Drawing", "Escape", "Stop drawing"),
    (
        "Drawing",
        "Drag a handle",
        "Move a point, the centre or a corner; turn a rectangle",
    ),
    ("Drawing", "Drag a segment's middle", "Insert a point there"),
    (
        "Drawing",
        "Shift+drag a segment's middle",
        "Bend the segment into an arc",
    ),
];

/// The groups, in the order the window shows them.
const GROUPS: [&str; 7] = [
    "File", "Edit", "Generate", "View", "Help", "Canvas", "Drawing",
];

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
            Action::Shortcuts => self.dialogs.shortcuts_open = true,
        }
    }

    /// Help → Keyboard shortcuts.
    pub(super) fn shortcuts_window(&mut self, ctx: &egui::Context) {
        let mut open = self.dialogs.shortcuts_open;
        egui::Window::new("Keyboard shortcuts")
            .open(&mut open)
            .default_size([520.0, 560.0])
            .show(ctx, |ui| {
                ScrollArea::vertical().show(ui, |ui| {
                    for (group, rows) in shortcut_rows() {
                        ui.add_space(4.0);
                        ui.strong(group);
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
pub(super) fn shortcut_rows() -> Vec<(&'static str, Vec<(String, &'static str)>)> {
    GROUPS
        .iter()
        .map(|&group| {
            let mut rows: Vec<(String, &str)> = BINDINGS
                .iter()
                .filter(|b| b.group == group)
                .map(|b| (keys_text(&b.keys), b.what))
                .collect();
            rows.extend(
                OTHERS
                    .iter()
                    .filter(|(g, _, _)| *g == group)
                    .map(|(_, keys, what)| (keys.to_string(), *what)),
            );
            (group, rows)
        })
        .filter(|(_, rows)| !rows.is_empty())
        .collect()
}
