// SPDX-License-Identifier: GPL-3.0-or-later
//! The tool lists of the dialogs that make a whole thing (*New sign…*, *Inlay…*): the
//! project's tools, then the library's not in it yet, copied into the project only when the
//! thing is made (the user, 2026-11-10).

use chipload_job::library::{LibraryTool, ToolLibrary};
use chipload_job::{Job, Tool, ToolType};

use super::forms::tool_label;
use super::*;

/// A tool chosen in such a dialog: one of the project's, or one of the library's.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum WizardTool {
    Project(i64),
    /// A library tool by its uid.
    Library(String),
}

impl WizardTool {
    /// Its project tool number, a library tool copied into `job` first (linked by uid; a
    /// project tool linked to it with the same values is used instead). None when the
    /// library has no such tool.
    pub(super) fn take(&self, job: &mut Job, library: Option<&ToolLibrary>) -> Option<i64> {
        match self {
            WizardTool::Project(id) => Some(*id),
            WizardTool::Library(uid) => {
                let entry = library?.tools.iter().find(|t| t.uid == *uid)?;
                Some(chipload_job::library::use_tool(job, entry).id)
            }
        }
    }
}

/// The kind of tool a list offers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ToolKind {
    /// A V-bit with its angle.
    Vbit,
    EndMill,
}

impl ToolKind {
    pub(super) fn fits(self, kind: ToolType, angle: Option<f64>) -> bool {
        match self {
            ToolKind::Vbit => kind == ToolType::Vbit && angle.is_some(),
            ToolKind::EndMill => kind == ToolType::Endmill,
        }
    }
}

/// The library's tools of kind `want` not in the project yet (a project tool linked to one
/// is listed as the project's).
pub(super) fn library_choices<'a>(
    tools: &[Tool],
    library: &'a ToolLibrary,
    want: ToolKind,
) -> Vec<&'a LibraryTool> {
    library
        .tools
        .iter()
        .filter(|t| want.fits(t.kind, t.angle))
        .filter(|t| {
            !tools
                .iter()
                .any(|p| p.library_uid.as_deref() == Some(t.uid.as_str()))
        })
        .collect()
}

/// The tools a dialog offers: the project's, then the library's not in it yet.
pub(super) struct ToolOffer<'a> {
    pub tools: &'a [Tool],
    pub library: Option<&'a ToolLibrary>,
    pub units: Units,
}

/// A tool's list: *None* first when `optional`, the project's tools of kind `want`, then
/// under *From your library:* the library's (marked +).
pub(super) fn tool_combo(
    ui: &mut egui::Ui,
    salt: &str,
    chosen: &mut Option<WizardTool>,
    offer: &ToolOffer<'_>,
    want: ToolKind,
    optional: bool,
) {
    let choices = offer
        .library
        .map(|l| library_choices(offer.tools, l, want))
        .unwrap_or_default();
    let shown = match chosen {
        Some(WizardTool::Project(id)) => offer
            .tools
            .iter()
            .find(|t| t.id == *id)
            .map(|t| tool_label(ui, t, offer.units)),
        Some(WizardTool::Library(uid)) => choices
            .iter()
            .find(|t| t.uid == *uid)
            .map(|t| format!("+ {}", t.name)),
        None => None,
    }
    .unwrap_or_else(|| t!(ui, "inlay-none"));
    egui::ComboBox::from_id_salt(salt)
        .selected_text(shown)
        .show_ui(ui, |ui| {
            if optional {
                ui.selectable_value(chosen, None, t!(ui, "inlay-none"));
            }
            for t in offer.tools.iter().filter(|t| want.fits(t.kind, t.angle)) {
                let label = tool_label(ui, t, offer.units);
                ui.selectable_value(chosen, Some(WizardTool::Project(t.id)), label);
            }
            if !choices.is_empty() {
                ui.separator();
                ui.weak(t!(ui, "forms-from-library"));
                for t in &choices {
                    let pick = Some(WizardTool::Library(t.uid.clone()));
                    ui.selectable_value(chosen, pick, format!("+ {}", t.name))
                        .on_hover_text(t!(ui, "wizard-library-tool-hover"));
                }
            }
        });
}
