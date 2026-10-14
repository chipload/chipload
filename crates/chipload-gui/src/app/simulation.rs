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
    /// View → Show stock (simulation), and the canvas bar's Stock.
    pub fn set_show_simulation(&mut self, on: bool) {
        self.show_simulation = on;
        if on {
            self.start_simulation();
        } else {
            self.drop_simulation();
        }
    }

    /// A simulation is being computed.
    pub fn simulating(&self) -> bool {
        self.sim.run.is_some()
    }

    /// Simulate the last generation's program on a worker thread, if switched on.
    pub(super) fn start_simulation(&mut self) {
        self.stop_simulation();
        if !self.show_simulation {
            return;
        }
        let (Some(result), Some(job)) = (&self.result, &self.result_job) else {
            return;
        };
        let (job, program, groups) = (job.clone(), result.program.clone(), result.groups.clone());
        let (sender, receiver) = channel();
        let cancel = Arc::new(Cancel::new());
        let flag = Arc::clone(&cancel);
        let spawned = std::thread::Builder::new()
            .name("simulate".into())
            .spawn(move || {
                let outcome = std::panic::catch_unwind(AssertUnwindSafe(|| {
                    simulate_with_history(&job, &program, &groups, None, &flag)
                }))
                .map_err(|_| "the simulation stopped unexpectedly".to_string());
                let _ = sender.send(outcome);
            });
        if let Err(err) = spawned {
            self.report
                .push(Tone::Warning, format!("Simulation could not start: {err}"));
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
            Err(TryRecvError::Disconnected) => Err("the simulation stopped unexpectedly".into()),
        };
        let serial = run.serial;
        self.sim.run = None;
        self.finish_simulation(ctx, serial, outcome);
        false
    }

    /// Wait for the simulation in progress and show it (tests, screenshots).
    #[cfg(any(test, feature = "screenshots"))]
    pub fn wait_for_simulation(&mut self, ctx: &egui::Context) -> bool {
        let Some(run) = self.sim.run.take() else {
            return false;
        };
        let outcome = run
            .receiver
            .recv()
            .unwrap_or_else(|_| Err("the simulation stopped unexpectedly".into()));
        self.finish_simulation(ctx, run.serial, outcome);
        self.sim.simulation.is_some()
    }

    fn finish_simulation(&mut self, ctx: &egui::Context, serial: u64, outcome: Outcome) {
        if serial != self.sim.serial || !self.show_simulation {
            return; // made for an older toolpath, or switched off meanwhile
        }
        let (sim, history) = match outcome {
            Ok(Ok(done)) => done,
            Ok(Err(PipelineError::Toolpath(chipload_toolpath::ToolpathError::Cancelled))) => {
                return;
            }
            Ok(Err(err)) => {
                self.report
                    .push(Tone::Warning, format!("Simulation failed: {err}"));
                return;
            }
            Err(panic) => {
                self.report
                    .push(Tone::Warning, format!("Simulation failed: {panic}"));
                return;
            }
        };
        let side = ctx.input(|i| i.max_texture_side);
        let image = sim.coarsened(sim.factor_for(usize::MAX, side));
        let texture =
            ctx.load_texture("simulation", image_of(&image), egui::TextureOptions::LINEAR);
        let surface = surface_of(&sim.coarsened(sim.factor_for(SURFACE_CELLS, usize::MAX)));
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
        for line in sim.lines() {
            self.report.push(Tone::Info, line);
        }
        if history.is_none() {
            self.report.push(
                Tone::Info,
                "  The program is too long to play back the stock; the slider shows the \
                 toolpath only"
                    .into(),
            );
        }
        self.sim.history = history;
        if let Some(playback) = &mut self.playback {
            playback.shown = None; // redrawn from the history on the next frame
        }
        self.sim.texture = Some(texture);
        self.sim.simulation = Some(sim);
    }
}

/// A simulation's shaded image for a texture.
pub(super) fn image_of(sim: &Simulation) -> egui::ColorImage {
    egui::ColorImage::from_rgba_unmultiplied([sim.columns, sim.rows], &sim.shaded(CANVAS_PALETTE))
}

/// The surface of a (coarse) simulation: its heights and shaded colours.
pub(super) fn surface_of(sim: &Simulation) -> SimSurface {
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
    SimSurface {
        origin: sim.origin,
        cell: sim.cell,
        columns: sim.columns,
        rows: sim.rows,
        heights: sim.heights.clone(),
        colors,
    }
}
