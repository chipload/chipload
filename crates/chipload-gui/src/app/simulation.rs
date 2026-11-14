// SPDX-License-Identifier: GPL-3.0-or-later
//! The material-removal simulation in the canvas (docs/specs/simulation.md §4): computed on
//! a worker thread after each generation while it is switched on, shown as an image in top
//! view and a surface in 3D.

use std::panic::AssertUnwindSafe;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, TryRecvError, channel};

use chipload_pipeline::Cancel;
use chipload_pipeline::simulate::{
    DISPLAY_CELLS, History, Palette, Simulation, simulate_with_history,
};

use super::*;
use crate::view::{SimLayer, SimSurface};

/// The 3D surface's budget, cells (§4): the display grid's, which the playback's history
/// uses too.
pub(super) const SURFACE_CELLS: usize = DISPLAY_CELLS;

/// The canvas's colours: the stock top a muted wood, the deepest cut dark, the spoilboard
/// warning red (the canvas is dark in both themes).
const CANVAS_PALETTE: Palette = Palette {
    top: [150, 132, 104],
    deep: [52, 42, 32],
    below: [190, 70, 60],
};

type Outcome = Result<Result<(Simulation, Option<History>), PipelineError>, String>;

/// A simulation in progress.
pub(super) struct SimRun {
    cancel: Arc<Cancel>,
    receiver: Receiver<Outcome>,
    /// The generation it is for: its result is shown only while that one is.
    serial: u64,
}

impl ChiploadApp {
    /// Simulate (on) or Edit (off) (docs/specs/edit-and-simulate.md §2, §3): the view
    /// bar's Simulate, View > Simulate, F6. Leaving Simulate keeps the simulation, hidden,
    /// for coming back; entering it computes one when there is none, and makes a toolpath
    /// first when there is none either.
    pub fn set_show_simulation(&mut self, on: bool) {
        if on == self.display.simulate {
            return;
        }
        // Each mode keeps its own view: this one's is put away, the other's comes back.
        let leaving = super::display::ModeView {
            three_d: self.display.three_d,
            toolpath: self.display.toolpath,
            drawing: self.show_drawing(),
            images: self.display.images,
        };
        let coming = self.display.other.replace(leaving).unwrap_or(leaving);
        self.display.simulate = on;
        self.display.toolpath = coming.toolpath;
        self.display.images = coming.images;
        if coming.three_d != self.display.three_d {
            if coming.three_d {
                self.view_3d();
            } else {
                self.view_top();
            }
        }
        if let Some(playback) = &mut self.playback {
            playback.shown = None; // redrawn from the history when shown again
        }
        if !on {
            self.stop_simulation();
            self.scene.sim = None;
            // Edit always shows the drawing: its shapes are what is worked on.
            self.drawing_hidden = None;
            return;
        }
        self.set_show_drawing(coming.drawing);
        if self.sim.simulation.is_some() {
            self.scene.sim.clone_from(&self.sim.full_layer);
        } else if self.toolpath.result.is_some() {
            self.start_simulation();
        } else if self.toolpath.running.is_none() && self.edit.is_some() {
            self.start_generation(false);
        }
    }

    /// Back to Edit: a drawing or editing tool, placing text or a Browser selection
    /// (docs/specs/edit-and-simulate.md decision 5).
    pub(super) fn enter_edit_mode(&mut self) {
        self.set_show_simulation(false);
    }

    /// How far the simulation being computed has got, 0 to 1; None when none is.
    pub fn simulation_progress(&self) -> Option<f64> {
        self.sim.run.as_ref().map(|r| r.cancel.progress())
    }

    /// A simulation is being computed.
    pub fn simulating(&self) -> bool {
        self.sim.run.is_some()
    }

    /// Simulate the last generation's program on a worker thread, if switched on.
    pub(super) fn start_simulation(&mut self) {
        self.stop_simulation();
        if !self.display.simulate {
            return;
        }
        let (Some(result), Some(job)) = (&self.toolpath.result, &self.toolpath.job) else {
            return;
        };
        let (job, program, groups) = (job.clone(), result.program.clone(), result.groups.clone());
        let (sender, receiver) = channel();
        let cancel = Arc::new(Cancel::new());
        let flag = Arc::clone(&cancel);
        let lang = crate::i18n::Lang::lang(self);
        let spawned = std::thread::Builder::new()
            .name("simulate".into())
            .spawn(move || {
                let outcome = std::panic::catch_unwind(AssertUnwindSafe(|| {
                    simulate_with_history(&job, &program, &groups, None, &flag)
                }))
                .map_err(|_| t!(lang, "simulation-stopped-unexpectedly"));
                let _ = sender.send(outcome);
            });
        if let Err(err) = spawned {
            self.report.push(
                Tone::Warning,
                t!(self, "simulation-could-not-start", error = err.to_string()),
            );
            return;
        }
        self.sim.run = Some(SimRun {
            cancel,
            receiver,
            serial: self.sim.serial,
        });
    }

    pub(super) fn stop_simulation(&mut self) {
        if let Some(run) = self.sim.run.take() {
            run.cancel.cancel();
        }
    }

    /// No simulation shown or running.
    pub(super) fn drop_simulation(&mut self) {
        self.stop_simulation();
        self.scene.sim = None;
        self.sim.texture = None;
        self.sim.playback_texture = None;
        self.sim.full_layer = None;
        self.sim.history = None;
        self.sim.simulation = None;
        if let Some(playback) = &mut self.playback {
            playback.shown = None;
        }
    }

    /// Show the worker's result if it has arrived. Returns whether one is still running.
    pub(super) fn poll_simulation(&mut self, ctx: &egui::Context) -> bool {
        let Some(run) = &self.sim.run else {
            return false;
        };
        let outcome = match run.receiver.try_recv() {
            Ok(outcome) => outcome,
            Err(TryRecvError::Empty) => return true,
            Err(TryRecvError::Disconnected) => Err(t!(self, "simulation-stopped-unexpectedly")),
        };
        let serial = run.serial;
        self.sim.run = None;
        self.finish_simulation(ctx, serial, outcome);
        false
    }

    /// Wait for the simulation in progress and show it (tests, screenshots).
    #[cfg(test)]
    pub fn wait_for_simulation(&mut self, ctx: &egui::Context) -> bool {
        let Some(run) = self.sim.run.take() else {
            return false;
        };
        let outcome = run
            .receiver
            .recv()
            .unwrap_or_else(|_| Err(t!(self, "simulation-stopped-unexpectedly")));
        self.finish_simulation(ctx, run.serial, outcome);
        self.sim.simulation.is_some()
    }

    fn finish_simulation(&mut self, ctx: &egui::Context, serial: u64, outcome: Outcome) {
        if serial != self.sim.serial || !self.display.simulate {
            return; // made for an older toolpath, or switched off meanwhile
        }
        let (sim, history) = match outcome {
            Ok(Ok(done)) => done,
            Ok(Err(PipelineError::Toolpath(chipload_toolpath::ToolpathError::Cancelled))) => {
                return;
            }
            Ok(Err(err)) => {
                self.report.push(
                    Tone::Warning,
                    t!(self, "simulation-failed", error = self.say(&err.message())),
                );
                return;
            }
            Err(panic) => {
                self.report
                    .push(Tone::Warning, t!(self, "simulation-failed", error = panic));
                return;
            }
        };
        self.show_simulation_layer(ctx, &sim);
        for line in sim.messages() {
            self.report.push_message(Tone::Info, line, None);
        }
        if history.is_none() {
            self.report.push(
                Tone::Info,
                format!("  {}", t!(self, "simulation-too-long-to-play")),
            );
        }
        self.sim.history = history;
        if let Some(playback) = &mut self.playback {
            playback.shown = None; // redrawn from the history on the next frame
        }
        self.sim.simulation = Some(sim);
    }

    /// The finished simulation as the canvas shows it: the top view's picture and the 3D
    /// surface, turned over when the board is (docs/specs/two-sided.md §10.2).
    fn show_simulation_layer(&mut self, ctx: &egui::Context, sim: &Simulation) {
        let turn = self.shown_turn();
        let side = ctx.input(|i| i.max_texture_side);
        let image = sim.coarsened(sim.factor_for(usize::MAX, side));
        let texture = ctx.load_texture(
            "simulation",
            image_of(&image, turn),
            egui::TextureOptions::LINEAR,
        );
        let surface = surface_of(
            &sim.coarsened(sim.factor_for(SURFACE_CELLS, usize::MAX)),
            turn,
        );
        let max = (
            sim.origin.0 + sim.columns as f64 * sim.cell,
            sim.origin.1 + sim.rows as f64 * sim.cell,
        );
        let layer = SimLayer {
            texture: texture.id(),
            min: sim.origin,
            max,
            surface,
        };
        self.scene.sim = Some(layer.clone());
        self.sim.full_layer = Some(layer);
        self.sim.texture = Some(texture);
    }

    /// How the board the canvas shows is turned over, if it is: for a setup that has a
    /// back or is one, when *Turn the board over* is on (docs/specs/two-sided.md §10.2). In
    /// Simulate only: the shapes are edited as drawn, from the front (decision 8).
    pub(super) fn shown_turn(&self) -> Option<chipload_job::Turn> {
        if !self.display.turned || !self.display.simulate {
            return None;
        }
        board_turn(&self.edit.as_ref()?.job)
    }

    /// Turn the board over, or back: the view, the simulation and playback with it.
    pub fn toggle_turned(&mut self, ctx: &egui::Context) {
        self.display.turned = !self.display.turned;
        if let Some(sim) = self.sim.simulation.take() {
            self.show_simulation_layer(ctx, &sim);
            self.sim.simulation = Some(sim);
        }
        if let Some(playback) = &mut self.playback {
            playback.shown = None;
        }
    }
}

/// How the active setup's board turns over: its own turn for a back, its back's for a
/// front with one; None for a board with one side.
pub(super) fn board_turn(job: &chipload_job::Job) -> Option<chipload_job::Turn> {
    let id = job.active_setup_id()?;
    job.setup(id)?
        .back_of
        .or_else(|| job.back_of(id)?.back_of)
        .map(|b| b.turn)
}

/// A simulation's shaded image for a texture; with `turn`, of the board turned over.
pub(super) fn image_of(sim: &Simulation, turn: Option<chipload_job::Turn>) -> egui::ColorImage {
    let turned = turn.map(|t| sim.turned_over(t));
    let sim = turned.as_ref().unwrap_or(sim);
    egui::ColorImage::from_rgba_unmultiplied([sim.columns, sim.rows], &sim.shaded(CANVAS_PALETTE))
}

/// The surface of a (coarse) simulation: its heights and shaded colours; with `turn`, of
/// the board turned over, its points already as it lies.
pub(super) fn surface_of(sim: &Simulation, turn: Option<chipload_job::Turn>) -> SimSurface {
    let turned = turn.map(|t| sim.turned_over(t));
    let sim = turned.as_ref().unwrap_or(sim);
    let pixels = sim.shaded(CANVAS_PALETTE);
    // `shaded` runs from the top row (highest Y) down; the surface from the bottom row up.
    let mut colors = Vec::with_capacity(sim.columns * sim.rows);
    for j in 0..sim.rows {
        let row = sim.rows - 1 - j;
        for i in 0..sim.columns {
            let k = (row * sim.columns + i) * 4;
            colors.push(egui::Color32::from_rgb(
                pixels[k],
                pixels[k + 1],
                pixels[k + 2],
            ));
        }
    }
    let [r, g, b] = CANVAS_PALETTE.top;
    SimSurface {
        origin: sim.origin,
        cell: sim.cell,
        columns: sim.columns,
        rows: sim.rows,
        heights: sim.heights.clone(),
        floors: sim.floors.clone(),
        colors,
        // The board's sides a shade darker than its top.
        wall: egui::Color32::from_rgb(r, g, b).gamma_multiply(0.7),
        raw: turn.is_some(),
    }
}
