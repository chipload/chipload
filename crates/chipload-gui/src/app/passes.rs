// SPDX-License-Identifier: GPL-3.0-or-later
//! Rest and floor passes added, changed and removed; operations duplicated and deleted.

use chipload_job::{Operation, ToolType, Units};

use super::ChiploadApp;
use super::{Focus, RestMenu, STATUS_SECONDS};
use crate::i18n::t;

impl ChiploadApp {
    pub fn duplicate_operation(&mut self) {
        if let (Focus::Op(i), Some(edit)) = (self.focus, self.edit.as_mut())
            && let Some(copy) = edit.duplicate(i, &self.prefs.language)
        {
            self.focus = Focus::Op(copy);
            self.commit();
        }
    }

    /// Add a rest pass to pocket `i`, cut with `tool` (docs/specs/rest-passes.md §6), and
    /// open it. False, with the reason shown, when it can't be added.
    pub fn add_rest_pass(&mut self, i: usize, tool: i64) -> bool {
        let cutting = self.tool_cutting(tool);
        let Some(edit) = self.edit.as_mut() else {
            return false;
        };
        let tools = edit.job.tools.clone();
        let Some(Operation::Pocket(pocket)) = edit.job.operations.get_mut(i) else {
            return false;
        };
        match chipload_pipeline::rest_pass_of(pocket, &tools, tool, cutting.as_ref()) {
            Ok(pass) => {
                pocket.rest_passes.push(pass);
                self.focus = Focus::RestPass(i, pocket.rest_passes.len() - 1);
                self.commit();
                true
            }
            Err(err) => {
                self.error(vec![t!(
                    self,
                    "app-could-not-add-rest-pass",
                    error = self.say(&err)
                )]);
                false
            }
        }
    }

    /// Add a rest pass to pocket `i` cut with the library tool `name`, copied into the
    /// project first unless it is there already: one undo step (docs/specs/rest-passes.md
    /// §6). If the pass can't be added, the project's tools stay as they were.
    pub fn add_rest_pass_from_library(&mut self, i: usize, name: &str) -> bool {
        let Some(before) = self.edit.as_ref().map(|e| e.job.tools.clone()) else {
            return false;
        };
        let Some(tool) = self.insert_library_tool(name) else {
            return false;
        };
        if self.add_rest_pass(i, tool) {
            return true;
        }
        if let Some(edit) = self.edit.as_mut() {
            edit.job.tools = before;
        }
        false
    }

    /// What pocket `i`'s *Add rest pass ▸* offers: the endmills smaller than the last tool
    /// of its chain, the project's and then the library's, or why there are none
    /// (docs/specs/rest-passes.md §6). None for any other operation.
    pub fn rest_menu(&self, i: usize, units: Units) -> Option<RestMenu> {
        let edit = self.edit.as_ref()?;
        let Operation::Pocket(pocket) = &edit.job.operations.get(i)? else {
            return None;
        };
        if chipload_job::has_bones(pocket.corners, &pocket.corner_choices) {
            return Some(RestMenu::Unavailable(t!(self, "app-rest-with-corners")));
        }
        let tools = &edit.job.tools;
        let last = pocket.rest_passes.last().map_or(pocket.tool, |r| r.tool);
        let Some(diameter) = tools.iter().find(|t| t.id == last).map(|t| t.diameter) else {
            return Some(RestMenu::Unavailable(t!(
                self,
                "app-rest-tool-missing",
                tool = format!("T{last}")
            )));
        };
        let project: Vec<(i64, f64)> = tools
            .iter()
            .filter(|t| t.kind == ToolType::Endmill && t.diameter < diameter)
            .map(|t| (t.id, t.diameter))
            .collect();
        // A library tool already copied in with the same values is listed as the project's.
        let library: Vec<(String, f64)> = self
            .library()
            .map(|l| l.tools.tools.as_slice())
            .unwrap_or_default()
            .iter()
            .filter(|e| e.kind == ToolType::Endmill && e.diameter < diameter)
            .filter(|e| {
                !tools
                    .iter()
                    .any(|t| t.library_uid.as_deref() == Some(e.uid.as_str()) && e.same_values(t))
            })
            .map(|e| (e.name.clone(), e.diameter))
            .collect();
        if project.is_empty() && library.is_empty() {
            return Some(RestMenu::Unavailable(t!(
                self,
                "app-rest-no-smaller",
                tool = format!("T{last}"),
                diameter = chipload_pipeline::units::length_text(diameter, units)
            )));
        }
        Some(RestMenu::Tools { project, library })
    }

    /// Pocket `i`'s rest passes, to change.
    pub(super) fn rest_passes_mut(
        &mut self,
        i: usize,
    ) -> Option<&mut Vec<chipload_job::ClearingPass>> {
        match self.edit.as_mut()?.job.operations.get_mut(i) {
            Some(Operation::Pocket(p)) => Some(&mut p.rest_passes),
            _ => None,
        }
    }

    /// Remove pocket `i`'s rest pass `k`; its pocket opens.
    pub fn remove_rest_pass(&mut self, i: usize, k: usize) {
        if let Some(passes) = self.rest_passes_mut(i)
            && k < passes.len()
        {
            passes.remove(k);
            self.focus = Focus::Op(i);
            self.commit();
        }
    }

    /// Turn pocket `i`'s rest pass `k` on or off.
    pub fn toggle_rest_pass(&mut self, i: usize, k: usize) {
        if let Some(pass) = self.rest_passes_mut(i).and_then(|p| p.get_mut(k)) {
            pass.enabled = !pass.enabled;
            self.commit();
        }
    }

    /// Give V-carve `i` a floor pass cut with `tool` (docs/specs/vcarve-wide-strokes.md §7),
    /// in place of any it had, and open it. False, with the reason shown, when it can't.
    pub fn add_floor_pass(&mut self, i: usize, tool: i64) -> bool {
        let cutting = self.tool_cutting(tool);
        let Some(edit) = self.edit.as_mut() else {
            return false;
        };
        let tools = edit.job.tools.clone();
        let Some(Operation::Vcarve(vcarve)) = edit.job.operations.get_mut(i) else {
            return false;
        };
        match chipload_pipeline::floor_pass_of(vcarve, &tools, tool, cutting.as_ref()) {
            Ok(pass) => {
                vcarve.floor = Some(pass);
                self.focus = Focus::FloorPass(i);
                self.commit();
                true
            }
            Err(err) => {
                self.error(vec![t!(
                    self,
                    "app-could-not-add-floor-pass",
                    error = self.say(&err)
                )]);
                false
            }
        }
    }

    /// [`App::add_floor_pass`] with the library tool `name`, copied into the project first
    /// unless it is there already. If the pass can't be added, the tools stay as they were.
    pub fn add_floor_pass_from_library(&mut self, i: usize, name: &str) -> bool {
        let Some(before) = self.edit.as_ref().map(|e| e.job.tools.clone()) else {
            return false;
        };
        let Some(tool) = self.insert_library_tool(name) else {
            return false;
        };
        if self.add_floor_pass(i, tool) {
            return true;
        }
        if let Some(edit) = self.edit.as_mut() {
            edit.job.tools = before;
        }
        false
    }

    /// What V-carve `i`'s *Add floor pass ▸* offers: every end mill, the project's and then
    /// the library's not yet in it; or why there is none. None for any other operation.
    pub fn floor_menu(&self, i: usize) -> Option<RestMenu> {
        let edit = self.edit.as_ref()?;
        let Operation::Vcarve(_) = &edit.job.operations.get(i)? else {
            return None;
        };
        let tools = &edit.job.tools;
        let project: Vec<(i64, f64)> = tools
            .iter()
            .filter(|t| t.kind == ToolType::Endmill)
            .map(|t| (t.id, t.diameter))
            .collect();
        let library: Vec<(String, f64)> = self
            .library()
            .map(|l| l.tools.tools.as_slice())
            .unwrap_or_default()
            .iter()
            .filter(|e| e.kind == ToolType::Endmill)
            .filter(|e| {
                !tools
                    .iter()
                    .any(|t| t.library_uid.as_deref() == Some(e.uid.as_str()) && e.same_values(t))
            })
            .map(|e| (e.name.clone(), e.diameter))
            .collect();
        if project.is_empty() && library.is_empty() {
            return Some(RestMenu::Unavailable(t!(self, "app-floor-no-endmill")));
        }
        Some(RestMenu::Tools { project, library })
    }

    /// V-carve `i`'s floor pass, to change.
    pub(super) fn floor_pass_mut(&mut self, i: usize) -> Option<&mut chipload_job::ClearingPass> {
        match self.edit.as_mut()?.job.operations.get_mut(i) {
            Some(Operation::Vcarve(v)) => v.floor.as_mut(),
            _ => None,
        }
    }

    /// Remove V-carve `i`'s floor pass; the V-carve opens.
    pub fn remove_floor_pass(&mut self, i: usize) {
        if let Some(Some(Operation::Vcarve(v))) =
            self.edit.as_mut().map(|e| e.job.operations.get_mut(i))
            && v.floor.take().is_some()
        {
            self.focus = Focus::Op(i);
            self.commit();
        }
    }

    /// Turn V-carve `i`'s floor pass on or off.
    pub fn toggle_floor_pass(&mut self, i: usize) {
        if let Some(pass) = self.floor_pass_mut(i) {
            pass.enabled = !pass.enabled;
            self.commit();
        }
    }

    /// For V-carve `i`'s floor pass form: the largest end mill that would clear its whole
    /// floor, mm, and the library's end mills that do, by name, nearest first
    /// (docs/specs/vcarve-wide-strokes.md §4.1). None when it clears it all.
    pub(super) fn floor_hint(&self, i: usize) -> Option<(f64, Vec<(String, f64)>)> {
        let (_, largest) = self
            .toolpath
            .result
            .as_ref()?
            .floor_hints
            .iter()
            .find(|(op, _)| *op == i)?;
        let library = self
            .library()
            .map(|l| l.tools.tools.as_slice())
            .unwrap_or_default();
        let fitting = chipload_pipeline::fitting_end_mills(library, *largest)
            .into_iter()
            .map(|t| (t.name.clone(), t.diameter))
            .collect();
        Some((*largest, fitting))
    }

    /// The project as edited, its operations included, for the pipeline's job-level rules.
    pub(super) fn edited_job(&self) -> Option<chipload_job::Job> {
        let edit = self.edit.as_ref()?;
        let job = edit.job.clone();
        Some(job)
    }

    /// The pocket that rest pocket `i` copies, which it can become a rest pass of
    /// (docs/specs/rest-passes.md §5).
    pub(super) fn rest_pocket_parent(&self, i: usize) -> Option<usize> {
        chipload_pipeline::rest_pocket_parent(&self.edited_job()?, i)
    }

    /// *Make it a rest pass*: rest pocket `i` becomes the last rest pass of the pocket it
    /// copies, which opens on it.
    pub fn make_rest_pass(&mut self, i: usize) {
        let Some(parent) = self.rest_pocket_parent(i) else {
            return;
        };
        let Some(edit) = self.edit.as_mut() else {
            return;
        };
        if !matches!(edit.job.operations.get(i), Some(Operation::Pocket(_))) {
            return;
        }
        let Some(Operation::Pocket(rest)) = edit.remove(i) else {
            return;
        };
        if let Some(Operation::Pocket(pocket)) = edit.job.operations.get_mut(parent) {
            pocket.rest_passes.push(chipload_job::ClearingPass {
                tool: rest.tool,
                feed: rest.feed,
                plunge_feed: rest.plunge_feed,
                step_down: rest.step_down,
                stepover: rest.stepover,
                slot_feed: rest.slot_feed,
                slot_step_down: rest.slot_step_down,
                entry: rest.entry,
                enabled: rest.enabled,
            });
            self.focus = Focus::RestPass(parent, pocket.rest_passes.len() - 1);
            self.commit();
            let text = t!(self, "app-now-rest-pass", op = i + 1, parent = parent + 1);
            self.set_status(text, STATUS_SECONDS);
        }
    }

    pub fn delete_operation(&mut self) {
        if let (Focus::Op(i), Some(edit)) = (self.focus, self.edit.as_mut()) {
            edit.remove(i);
            let len = edit.job.operations.len();
            self.focus = if len == 0 {
                Focus::Job
            } else {
                Focus::Op(i.min(len - 1))
            };
            self.commit();
        }
    }
}
