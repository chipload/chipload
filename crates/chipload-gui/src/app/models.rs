// SPDX-License-Identifier: GPL-3.0-or-later
//! 3D models in the app (docs/specs/stl-relief.md §9): File > Add 3D model…, the model's
//! properties, which draw its outline again as one undo step, and a relief of it.

use chipload_core::chain::ShapeFrom;
use chipload_job::Model;
use chipload_pipeline::models::{NewModel, ReadModel, add_model, read_model, remake, remove_model};
use chipload_pipeline::new_ops::OpKind;

use super::*;

/// The darkest a model's surface is shaded, as a share of its colour.
const MODEL_SHADE_LOW: f32 = 0.35;

/// The models' surfaces in 3D are about this many cells along their longer side.
const SURFACE_CELLS: usize = 160;

/// The models' faint surfaces in the 3D view (spec §9), kept until a model, the board's
/// top or the theme changes.
#[derive(Default)]
pub(super) struct ModelSurfaces {
    key: Option<(Vec<Model>, u64, bool)>,
    surfaces: Vec<crate::view::SimSurface>,
}

/// An STL file being read on a worker thread (docs/specs/relief-wizard.md §2).
pub(super) struct ModelReading {
    path: std::path::PathBuf,
    receiver: std::sync::mpsc::Receiver<Result<ReadModel, ReadFailed>>,
}

/// Why a model's file could not be added: the line to show, and the reader's reason.
type ReadFailed = (String, Option<chipload_i18n::Message>);

/// A new model's depth is at most this share of the board's thickness when its own
/// proportion would carve deeper (spec §14 step 4).
const NEW_MODEL_MOST_DEPTH_SHARE: f64 = 0.5;

impl ChiploadApp {
    /// File > Add 3D model…: pick an STL file and add it.
    pub(super) fn add_model_dialog(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter(t!(self, "models-stl-filter"), &["stl", "STL"])
            .pick_file()
        {
            self.add_model_from(&path);
        }
    }

    /// Start adding the STL file at `path` to the project (a new one if none is open): it
    /// is read on a worker thread, the window stays usable, and it is added when read
    /// ([`ChiploadApp::poll_model_reading`]). False when a file is being read already.
    pub fn add_model_from(&mut self, path: &std::path::Path) -> bool {
        if self.model_reading.is_some() {
            return false;
        }
        if self.edit.is_none() {
            self.new_project();
        }
        let (sender, receiver) = std::sync::mpsc::channel();
        let file = path.to_path_buf();
        let lang = crate::i18n::Lang::lang(self);
        let spawned = std::thread::Builder::new()
            .name("read model".into())
            .spawn(move || {
                let path = file.display().to_string();
                let read = std::fs::read(&file)
                    .map_err(|err| {
                        let error = err.to_string();
                        (
                            t!(
                                lang,
                                "models-cannot-read",
                                path = path.clone(),
                                error = error
                            ),
                            None,
                        )
                    })
                    .and_then(|bytes| {
                        let cannot = t!(lang, "models-cannot-add", path = path.clone());
                        match std::panic::catch_unwind(|| read_model(&bytes)) {
                            Ok(Ok(read)) => Ok(read),
                            Ok(Err(err)) => Err((cannot, Some(err.message()))),
                            Err(_) => Err((cannot, None)),
                        }
                    });
                let _ = sender.send(read);
            });
        if let Err(err) = spawned {
            self.error(vec![t!(
                self,
                "models-cannot-read",
                path = path.display().to_string(),
                error = err.to_string()
            )]);
            return false;
        }
        let name = file_name(path);
        self.set_status(t!(self, "models-reading", file = name), STATUS_SECONDS);
        self.model_reading = Some(ModelReading {
            path: path.to_path_buf(),
            receiver,
        });
        true
    }

    /// An STL file is being read.
    pub(super) fn reading_model(&self) -> bool {
        self.model_reading.is_some()
    }

    /// Add the model whose file has been read, if it has; true while still reading.
    pub(super) fn poll_model_reading(&mut self) -> bool {
        let Some(reading) = &self.model_reading else {
            return false;
        };
        let read = match reading.receiver.try_recv() {
            Ok(read) => read,
            Err(std::sync::mpsc::TryRecvError::Empty) => return true,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => Err((
                t!(
                    self,
                    "models-cannot-add",
                    path = reading.path.display().to_string()
                ),
                None,
            )),
        };
        self.finish_model_reading(read);
        false
    }

    /// Wait for the file being read, and add it, as the window does when it is read. True
    /// when a model was added.
    #[cfg(test)]
    pub fn wait_model_reading(&mut self) -> bool {
        let Some(reading) = &self.model_reading else {
            return false;
        };
        let read = reading.receiver.recv().unwrap_or_else(|_| {
            Err((
                t!(
                    self,
                    "models-cannot-add",
                    path = reading.path.display().to_string()
                ),
                None,
            ))
        });
        self.finish_model_reading(read)
    }

    fn finish_model_reading(&mut self, read: Result<ReadModel, ReadFailed>) -> bool {
        let Some(reading) = self.model_reading.take() else {
            return false;
        };
        self.status = None;
        let added = match read {
            Ok(read) => self.add_read_model(&reading.path, &read),
            Err((text, Some(why))) => {
                self.error_report(Report::failed(text, why));
                false
            }
            Err((text, None)) => {
                self.error(vec![text]);
                false
            }
        };
        // Dropped files waiting behind this one come in now.
        self.next_drop();
        added
    }

    /// Add the model read from `path`: half the board wide at its lower-left corner, as
    /// deep as its proportion gives but at most half the board, as one undo step; its
    /// properties open.
    fn add_read_model(&mut self, path: &std::path::Path, read: &ReadModel) -> bool {
        if self.edit.is_none() {
            self.new_project();
        }
        let Some(edit) = self.edit.as_ref() else {
            return false;
        };
        let file = path
            .file_name()
            .map(|f| f.to_string_lossy().into_owned())
            .unwrap_or_default();
        // Half the stock's width to start with, as a picture: shown and editable.
        let (width, _) = edit.job.stock_size();
        let thickness = edit.job.stock().thickness;
        let mut new = NewModel::new(&file);
        new.width = Some((width / 2.0).round().max(1.0));
        let added = add_model(&edit.job, read, new).and_then(|(job, report)| {
            let most = thickness * NEW_MODEL_MOST_DEPTH_SHARE;
            if report.placed[2] <= most {
                return Ok((job, report));
            }
            // Flatter than its own proportion: reliefs nearly always are (decision 4).
            let mut flatter = job;
            if let Some(model) = flatter
                .drawing
                .models
                .iter_mut()
                .find(|m| m.id == report.id)
            {
                model.depth = most;
                model.proportional = false;
            }
            remake(&flatter, report.id).map(|(job, again)| {
                let mut messages = report.messages;
                messages.extend(again.messages);
                (
                    job,
                    chipload_pipeline::models::ModelReport { messages, ..again },
                )
            })
        });
        match added {
            Ok((job, report)) => {
                if let Some(edit) = self.edit.as_mut() {
                    edit.job = job;
                }
                self.drawing_changed(&[]);
                self.selection = CanvasSelection::whole(ShapeFrom::Model(report.id));
                self.focus = Focus::Model(report.id);
                self.commit();
                self.set_report(Report::of_messages(report.messages));
                true
            }
            Err(err) => {
                self.error_report(Report::failed(
                    t!(self, "models-cannot-add", path = path.display().to_string()),
                    err.message(),
                ));
                false
            }
        }
    }

    /// Model `id` as its properties left it, its outline drawn again, as one undo step.
    pub(super) fn set_model(&mut self, id: u64, model: &Model) -> bool {
        let Some(edit) = self.edit.as_ref() else {
            return false;
        };
        let mut job = edit.job.clone();
        let Some(slot) = job.drawing.models.iter_mut().find(|m| m.id == id) else {
            return false;
        };
        *slot = model.clone();
        match remake(&job, id) {
            Ok((job, report)) => {
                if let Some(edit) = self.edit.as_mut() {
                    edit.job = job;
                }
                self.drawing_changed(&[]);
                self.selection = CanvasSelection::whole(ShapeFrom::Model(id));
                self.commit();
                self.set_report(Report::of_messages(report.messages));
                true
            }
            Err(err) => {
                self.error_report(Report::failed(
                    t!(self, "models-cannot-place"),
                    err.message(),
                ));
                false
            }
        }
    }

    /// Remove model `id` and its outline, as one undo step. Its reliefs stay, with a
    /// problem each until they get another model.
    pub fn remove_model(&mut self, id: u64) -> bool {
        let Some(edit) = self.edit.as_mut() else {
            return false;
        };
        match remove_model(&edit.job, id) {
            Ok(job) => {
                edit.job = job;
                self.drawing_changed(&[]);
                self.selection = CanvasSelection::Nothing;
                if self.focus == Focus::Model(id) {
                    self.focus = Focus::Job;
                }
                self.commit();
                self.set_status(t!(self, "models-removed", id = id), STATUS_SECONDS);
                true
            }
            Err(err) => {
                self.error_report(Report::of_error(err.message()));
                false
            }
        }
    }

    /// Whether the project has a 3D model, which a relief carves.
    pub(super) fn has_models(&self) -> bool {
        self.edit
            .as_ref()
            .is_some_and(|e| !e.job.drawing.models.is_empty())
    }

    /// A relief of model `id`, within its outline, in the active part.
    pub(super) fn new_relief(&mut self, id: u64) {
        self.enter_edit_mode();
        self.selection = CanvasSelection::whole(ShapeFrom::Model(id));
        self.add_operation(OpKind::Relief);
    }

    /// Give relief `i` a roughing pass cut with `tool` (docs/specs/stl-relief.md §14), in
    /// place of any it had, and open it. False, with the reason shown, when it can't.
    pub fn add_roughing(&mut self, i: usize, tool: i64) -> bool {
        let cutting = self.tool_cutting(tool);
        let Some(edit) = self.edit.as_mut() else {
            return false;
        };
        let tools = edit.job.tools.clone();
        let Some(Operation::Relief(relief)) = edit.job.operations.get_mut(i) else {
            return false;
        };
        let allowance = chipload_pipeline::DEFAULT_ROUGHING_ALLOWANCE_MM;
        match chipload_pipeline::roughing_of(relief, &tools, tool, allowance, cutting.as_ref()) {
            Ok(pass) => {
                relief.roughing = Some(pass);
                self.focus = Focus::RoughingPass(i);
                self.commit();
                true
            }
            Err(err) => {
                self.error(vec![t!(
                    self,
                    "models-could-not-add-roughing",
                    error = self.say(&err)
                )]);
                false
            }
        }
    }

    /// [`App::add_roughing`] with the library tool `name`, copied into the project first
    /// unless it is there already. If the pass can't be added, the tools stay as they were.
    pub fn add_roughing_from_library(&mut self, i: usize, name: &str) -> bool {
        let Some(before) = self.edit.as_ref().map(|e| e.job.tools.clone()) else {
            return false;
        };
        let Some(tool) = self.insert_library_tool(name) else {
            return false;
        };
        if self.add_roughing(i, tool) {
            return true;
        }
        if let Some(edit) = self.edit.as_mut() {
            edit.job.tools = before;
        }
        false
    }

    /// What relief `i`'s *Add roughing pass ▸* offers: the end mills and plain ball noses,
    /// the project's and then the library's not yet in it; or why there is none. None for
    /// any other operation.
    pub fn roughing_menu(&self, i: usize) -> Option<RestMenu> {
        let edit = self.edit.as_ref()?;
        let Operation::Relief(_) = &edit.job.operations.get(i)? else {
            return None;
        };
        // A tapered ball nose or a V-bit has a point too fine to rough with.
        let roughs = |kind: ToolType, angle: Option<f64>| {
            kind == ToolType::Endmill || (kind == ToolType::Ballnose && angle.is_none())
        };
        let tools = &edit.job.tools;
        let project: Vec<(i64, f64)> = tools
            .iter()
            .filter(|t| roughs(t.kind, t.angle))
            .map(|t| (t.id, t.diameter))
            .collect();
        let library: Vec<(String, f64)> = self
            .library()
            .map(|l| l.tools.tools.as_slice())
            .unwrap_or_default()
            .iter()
            .filter(|e| roughs(e.kind, e.angle))
            .filter(|e| {
                !tools
                    .iter()
                    .any(|t| t.library_uid.as_deref() == Some(e.uid.as_str()) && e.same_values(t))
            })
            .map(|e| (e.name.clone(), e.diameter))
            .collect();
        if project.is_empty() && library.is_empty() {
            return Some(RestMenu::Unavailable(t!(self, "models-no-roughing-tool")));
        }
        Some(RestMenu::Tools { project, library })
    }

    /// Relief `i`'s roughing pass, to change.
    pub(super) fn roughing_mut(&mut self, i: usize) -> Option<&mut chipload_job::ReliefRoughing> {
        match self.edit.as_mut()?.job.operations.get_mut(i) {
            Some(Operation::Relief(r)) => r.roughing.as_mut(),
            _ => None,
        }
    }

    /// Remove relief `i`'s roughing pass; the relief opens.
    pub fn remove_roughing(&mut self, i: usize) {
        if let Some(Some(Operation::Relief(r))) =
            self.edit.as_mut().map(|e| e.job.operations.get_mut(i))
            && r.roughing.take().is_some()
        {
            self.focus = Focus::Op(i);
            self.commit();
        }
    }

    /// Turn relief `i`'s roughing pass on or off.
    pub fn toggle_roughing(&mut self, i: usize) {
        if let Some(pass) = self.roughing_mut(i) {
            pass.enabled = !pass.enabled;
            self.commit();
        }
    }

    /// Keep the 3D view's model surfaces in step with the project's models.
    pub(super) fn sync_models(&mut self, dark: bool) {
        let models: Vec<Model> = self
            .edit
            .as_ref()
            .map(|e| e.job.drawing.models.clone())
            .unwrap_or_default();
        let top = self.scene.stock_z.map_or(0.0, |(top, _)| top);
        let key = Some((models.clone(), top.to_bits(), dark));
        if key != self.model_surfaces.key {
            let color = crate::style::op_color(OpKind::Relief, dark);
            let surfaces = match self.edit.as_ref() {
                Some(edit) => models
                    .iter()
                    .filter_map(|m| {
                        chipload_pipeline::models::top_surface(&edit.job, m.id, SURFACE_CELLS)
                            .ok()
                            .flatten()
                    })
                    .map(|grid| model_surface(&grid, top, color))
                    .collect(),
                None => Vec::new(),
            };
            self.model_surfaces = ModelSurfaces { key, surfaces };
        }
        self.scene.models.clone_from(&self.model_surfaces.surfaces);
    }
}

/// A model's top as a surface on the board whose top is at `top`: its sides go down to
/// its lowest point, cells with nothing over them are left out.
fn model_surface(
    grid: &chipload_pipeline::models::TopHeights,
    top: f64,
    color: egui::Color32,
) -> crate::view::SimSurface {
    let lowest = grid.heights.iter().flatten().copied().fold(0.0, f64::min);
    let bottom = (top + lowest) as f32;
    // A cell with nothing over it has its floor at its height: left out.
    let heights: Vec<f32> = grid
        .heights
        .iter()
        .map(|h| h.map_or(bottom, |h| (top + h) as f32))
        .collect();
    let floors = vec![bottom; heights.len()];
    // Lit from the upper left, as the simulation is, so its shape shows.
    let colors = chipload_pipeline::models::top_shades(grid)
        .into_iter()
        .map(|shade| color.gamma_multiply(MODEL_SHADE_LOW + (1.0 - MODEL_SHADE_LOW) * shade))
        .collect();
    crate::view::SimSurface {
        origin: grid.origin,
        cell: grid.cell,
        columns: grid.columns,
        rows: grid.rows,
        colors,
        heights,
        floors,
        wall: color.gamma_multiply(0.7),
        raw: false,
    }
}
