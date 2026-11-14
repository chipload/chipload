// SPDX-License-Identifier: GPL-3.0-or-later
//! The new project dialog (docs/specs/new-project-dialog.md): the board, Z zero, the machine
//! and the tools, chosen before the project exists and remembered for the next one. The
//! pipeline makes the project (`library::start_project`); this only gathers the choices.

use std::path::PathBuf;
use std::sync::Arc;

use super::*;
use chipload_job::ZZero;
use chipload_job::library::NewProject;
use chipload_pipeline::library::{Board, Library, ProjectStart, tool_numbers};

/// What the new project starts from.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum Starting {
    Empty,
    /// A drawing already read, with its file.
    Import(PathBuf, Arc<chipload_dxf::ImportedShapes>),
}

/// The dialog's state while it is open.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct NewProjectDialog {
    starting: Starting,
    choice: NewProject,
    search: String,
}

/// A new project's choices when nothing is remembered (docs/specs/new-project-dialog.md
/// §4): the library's default machine, material and tool, the material's thickness (else
/// the default one, else `job`'s), the board fitted round the drawing as `job`'s is, Z zero
/// on top.
fn default_choice(library: &Library, job: &chipload_job::Job) -> NewProject {
    let stock = job.stock();
    let material = library.materials.default_entry();
    NewProject {
        machine: library.machines.default_entry().map(|m| m.uid.clone()),
        material: material.map(|m| m.uid.clone()),
        thickness: library.materials.thickness_for(material, stock.thickness),
        x: stock.x,
        y: stock.y,
        fit: Some(stock.fit.unwrap_or(chipload_pipeline::DEFAULT_STOCK_MARGIN)),
        z_zero: ZZero::Top,
        tools: library
            .tools
            .default_entry()
            .map(|t| t.uid.clone())
            .into_iter()
            .collect(),
    }
}

/// The remembered choices with what has left the library taken out: a machine or material
/// falls back to the default, a tool is no longer ticked (§4).
fn resolved(mut choice: NewProject, library: &Library) -> NewProject {
    if choice
        .machine
        .as_ref()
        .is_some_and(|uid| library.machines.find_uid(uid).is_none())
    {
        choice.machine = library.machines.default_entry().map(|m| m.uid.clone());
    }
    if choice
        .material
        .as_ref()
        .is_some_and(|uid| !library.materials.materials.iter().any(|m| m.uid == *uid))
    {
        choice.material = library.materials.default_entry().map(|m| m.uid.clone());
    }
    choice
        .tools
        .retain(|uid| library.tools.find_uid(uid).is_some());
    choice
}

/// The pipeline's start for `choice`: every choice made, the tools in the library's order.
/// With no tools in the library the project keeps Chipload's own end mill.
fn start_of(choice: &NewProject, library: &Library) -> ProjectStart {
    let tools = (!library.tools.tools.is_empty()).then(|| {
        library
            .tools
            .tools
            .iter()
            .filter(|t| choice.tools.contains(&t.uid))
            .map(|t| t.uid.clone())
            .collect()
    });
    ProjectStart {
        machine: choice.machine.clone(),
        material: Some(choice.material.clone()),
        thickness: Some(choice.thickness),
        board: Some(Board {
            x: choice.x,
            y: choice.y,
            fit: choice.fit,
        }),
        z_zero: Some(choice.z_zero),
        tools,
    }
}

impl ChiploadApp {
    /// The remembered choices as a start, for a project made without the dialog (a picture
    /// or model dropped on an empty window, the tests); none remembered: the library's
    /// defaults as before.
    pub(super) fn remembered_start(&self) -> ProjectStart {
        match &self.prefs.new_project {
            Some(choice) => start_of(
                &resolved(choice.clone(), &self.user.library),
                &self.user.library,
            ),
            None => ProjectStart::default(),
        }
    }

    /// *New project*: the dialog, for an empty project.
    pub(super) fn ask_new_project(&mut self) {
        self.open_new_project(Starting::Empty);
    }

    /// A drawing as a new project: read it (asking for its units or pixels per inch when it
    /// doesn't say), then the dialog.
    pub(super) fn ask_import_file(
        &mut self,
        path: &std::path::Path,
        units: Option<Units>,
        px: Option<f64>,
    ) -> bool {
        let Some(imported) = self.read_new_drawing(path, units, px) else {
            return false;
        };
        self.open_new_project(Starting::Import(path.to_path_buf(), Arc::new(imported)));
        true
    }

    fn open_new_project(&mut self, starting: Starting) {
        self.reload_library();
        let library = &self.user.library;
        let job = self.in_display_units(chipload_pipeline::empty_project());
        let choice = match &self.prefs.new_project {
            Some(remembered) => resolved(remembered.clone(), library),
            None => default_choice(library, &job),
        };
        self.dialogs.prompt = Some(Prompt::NewProject(Box::new(NewProjectDialog {
            starting,
            choice,
            search: String::new(),
        })));
    }

    pub(super) fn new_project_prompt(&mut self, ctx: &egui::Context, mut dialog: NewProjectDialog) {
        let mut answer: Option<bool> = None;
        let library = self.user.library.clone();
        let units = self.prefs.units;
        let modal = egui::Modal::new(egui::Id::new("new-project")).show(ctx, |ui| {
            ui.set_width(760.0_f32.min(ctx.content_rect().width() - 48.0));
            ui.label(crate::style::bold(
                ui,
                egui::RichText::new(t!(ui, "new-project-title")).heading(),
            ));
            ui.add_space(8.0);
            ui.horizontal_top(|ui| {
                ui.vertical(|ui| {
                    ui.set_width(340.0);
                    board_column(ui, &mut dialog.choice, &library);
                });
                // Space, not a separator: a vertical separator takes all the height there is,
                // and the dialog would grow a little every frame.
                ui.add_space(24.0);
                ui.vertical(|ui| {
                    tools_column(ui, &mut dialog, &library, units);
                });
            });
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button(t!(ui, "new-project-create")).clicked() {
                    answer = Some(true);
                }
                if ui.button(t!(ui, "common-cancel")).clicked() {
                    answer = Some(false);
                }
            });
            // Enter creates (but not while typing in the search).
            if ui.input(|i| i.key_pressed(egui::Key::Enter))
                && !ui.memory(|m| m.focused().is_some())
            {
                answer = Some(true);
            }
        });
        if modal.should_close() && answer.is_none() {
            answer = Some(false);
        }
        match answer {
            Some(true) => self.create_from_dialog(dialog),
            Some(false) => self.dialogs.prompt = None,
            None => self.dialogs.prompt = Some(Prompt::NewProject(Box::new(dialog))),
        }
    }

    /// *Create*: remember the choices for next time and make the project.
    fn create_from_dialog(&mut self, dialog: NewProjectDialog) {
        self.dialogs.prompt = None;
        let choice = dialog.choice.clone();
        self.prefs.new_project = Some(choice.clone());
        self.change_settings(|s| s.new_project = Some(choice));
        let start = start_of(&dialog.choice, &self.user.library);
        match dialog.starting {
            Starting::Empty => {
                let first = t!(self, "app-new-project");
                self.create_project(chipload_pipeline::empty_project(), &start, first, None);
            }
            Starting::Import(path, imported) => {
                let imported = Arc::try_unwrap(imported).unwrap_or_else(|a| (*a).clone());
                self.create_from_import(&path, imported, &start);
            }
        }
    }

    /// Tests: press *Create* in the open dialog, then take the next dropped file, as the
    /// window does on its next frame. False when the dialog isn't open.
    #[cfg(test)]
    pub(super) fn accept_new_project(&mut self) -> bool {
        let Some(Prompt::NewProject(dialog)) = self.dialogs.prompt.clone() else {
            return false;
        };
        self.create_from_dialog(*dialog);
        self.next_drop();
        true
    }

    /// Tests: press *Cancel*; false when the dialog isn't open.
    #[cfg(test)]
    pub(super) fn cancel_new_project(&mut self) -> bool {
        let open = self.new_project_open();
        if open {
            self.dialogs.prompt = None;
        }
        open
    }

    /// Tests: whether the dialog is open.
    #[cfg(test)]
    pub(super) fn new_project_open(&self) -> bool {
        matches!(self.dialogs.prompt, Some(Prompt::NewProject(_)))
    }

    /// Tests: the open dialog's choices, to change before [`Self::accept_new_project`].
    #[cfg(test)]
    pub(super) fn new_project_choice(&mut self) -> Option<&mut NewProject> {
        match self.dialogs.prompt.as_mut() {
            Some(Prompt::NewProject(dialog)) => Some(&mut dialog.choice),
            _ => None,
        }
    }
}

/// Material, thickness, board, Z zero and machine.
fn board_column(ui: &mut egui::Ui, choice: &mut NewProject, library: &Library) {
    egui::Grid::new("new-project-board")
        .num_columns(2)
        .spacing([12.0, 8.0])
        .show(ui, |ui| {
            ui.label(t!(ui, "new-project-material"));
            let none = t!(ui, "new-project-material-none");
            let name = |uid: &Option<String>| {
                uid.as_ref()
                    .and_then(|u| library.materials.materials.iter().find(|m| m.uid == *u))
                    .map_or(none.clone(), |m| m.name.clone())
            };
            let before = choice.material.clone();
            egui::ComboBox::from_id_salt("new-project-material")
                .selected_text(name(&choice.material))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut choice.material, None, none.clone());
                    for m in &library.materials.materials {
                        ui.selectable_value(&mut choice.material, Some(m.uid.clone()), &m.name);
                    }
                });
            // A material with a thickness of its own fills it in (§3).
            if choice.material != before
                && let Some(t) = choice
                    .material
                    .as_ref()
                    .and_then(|u| library.materials.materials.iter().find(|m| m.uid == *u))
                    .and_then(|m| m.thickness)
            {
                choice.thickness = t;
            }
            ui.end_row();
            ui.label(t!(ui, "new-project-thickness"));
            ui.add(crate::units::length(&mut choice.thickness).range(0.1..=1000.0));
            ui.end_row();
            ui.label(t!(ui, "new-project-board"));
            ui.vertical(|ui| {
                let mut fit = choice.fit.is_some();
                ui.horizontal(|ui| {
                    ui.radio_value(&mut fit, true, t!(ui, "new-project-fit"));
                    if let Some(margin) = &mut choice.fit {
                        ui.add(crate::units::length(margin).range(0.0..=1000.0));
                    }
                });
                ui.horizontal(|ui| {
                    ui.radio_value(&mut fit, false, t!(ui, "new-project-size"));
                    if choice.fit.is_none() {
                        ui.add(crate::units::length(&mut choice.x).range(1.0..=100_000.0));
                        ui.label("×");
                        ui.add(crate::units::length(&mut choice.y).range(1.0..=100_000.0));
                    }
                });
                if fit != choice.fit.is_some() {
                    choice.fit = fit.then_some(chipload_pipeline::DEFAULT_STOCK_MARGIN);
                }
            });
            ui.end_row();
            ui.label(t!(ui, "new-project-z-zero"))
                .on_hover_text(t!(ui, "new-project-z-zero-hover"));
            ui.horizontal(|ui| {
                ui.radio_value(&mut choice.z_zero, ZZero::Top, t!(ui, "new-project-z-top"));
                ui.radio_value(
                    &mut choice.z_zero,
                    ZZero::Bottom,
                    t!(ui, "new-project-z-spoilboard"),
                );
            });
            ui.end_row();
            ui.label(t!(ui, "new-project-machine"));
            ui.vertical(|ui| machine_rows(ui, choice, library));
            ui.end_row();
        });
}

fn machine_rows(ui: &mut egui::Ui, choice: &mut NewProject, library: &Library) {
    let profiles = &library.machines.machines;
    if profiles.is_empty() {
        ui.weak(t!(ui, "new-project-machine-none"));
        return;
    }
    let chosen = choice
        .machine
        .as_ref()
        .and_then(|u| library.machines.find_uid(u));
    egui::ComboBox::from_id_salt("new-project-machine")
        .selected_text(chosen.map_or(String::new(), |m| m.name.clone()))
        .show_ui(ui, |ui| {
            for m in profiles {
                ui.selectable_value(&mut choice.machine, Some(m.uid.clone()), &m.name);
            }
        });
    if let Some(m) = chosen {
        let units = match m.gcode_units {
            Units::Mm => "mm",
            Units::Inch => "inch",
        };
        let change = match m.tool_change {
            chipload_job::ToolChange::Files => t!(ui, "forms-tool-change-files"),
            chipload_job::ToolChange::Program => t!(ui, "forms-tool-change-program"),
        };
        ui.weak(t!(
            ui,
            "new-project-machine-summary",
            post = m.post.clone(),
            units = units,
            change = change
        ));
    }
}

/// The library's tools to tick, with their T numbers in the new project.
fn tools_column(ui: &mut egui::Ui, dialog: &mut NewProjectDialog, library: &Library, units: Units) {
    ui.horizontal(|ui| {
        ui.label(crate::style::bold(ui, t!(ui, "new-project-tools")));
        ui.add(
            egui::TextEdit::singleline(&mut dialog.search)
                .hint_text(t!(ui, "new-project-search"))
                .desired_width(160.0),
        );
    });
    let entries = &library.tools.tools;
    if entries.is_empty() {
        ui.weak(t!(ui, "new-project-no-library-tools"));
        return;
    }
    // Numbers as the project will have them: ticked tools in the library's order.
    let ticked: Vec<&chipload_job::library::LibraryTool> = entries
        .iter()
        .filter(|t| dialog.choice.tools.contains(&t.uid))
        .collect();
    let numbers = tool_numbers(ticked.iter().map(|t| t.id));
    let number_of = |uid: &str| ticked.iter().position(|t| t.uid == uid).map(|i| numbers[i]);
    let needle = dialog.search.trim().to_lowercase();
    egui::ScrollArea::vertical()
        .id_salt("new-project-tools")
        .max_height(300.0)
        .show(ui, |ui| {
            for entry in entries {
                let label = super::forms::tool_label(ui, &entry.tool(entry.id), units);
                let text = format!("{} {label}", entry.name);
                if !needle.is_empty() && !text.to_lowercase().contains(&needle) {
                    continue;
                }
                let mut on = dialog.choice.tools.contains(&entry.uid);
                let number =
                    number_of(&entry.uid).map_or(String::from("    "), |n| format!("T{n}"));
                let shown = format!("{number}  {}", entry.name);
                let kind = format!(
                    "{} Ø{}",
                    super::forms::tool_kind_name(ui, entry.kind),
                    crate::units::text(ui.ctx(), entry.diameter)
                );
                ui.horizontal(|ui| {
                    if ui.checkbox(&mut on, shown).on_hover_text(label).changed() {
                        if on {
                            dialog.choice.tools.push(entry.uid.clone());
                        } else {
                            dialog.choice.tools.retain(|u| *u != entry.uid);
                        }
                    }
                    ui.weak(kind);
                });
            }
        });
    ui.add_space(4.0);
    let list: Vec<String> = numbers.iter().map(|n| format!("T{n}")).collect();
    ui.weak(t!(
        ui,
        "new-project-tools-count",
        count = list.len(),
        list = list.join(", ")
    ));
}
