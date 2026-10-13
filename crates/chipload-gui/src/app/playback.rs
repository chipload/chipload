// SPDX-License-Identifier: GPL-3.0-or-later
//! The playback slider (docs/specs/playback.md): drag the program's progress back and forth
//! by time, see the moves done, the tool, and with the simulation on the stock as cut so
//! far.

use chipload_pipeline::estimate::{duration_text, move_seconds};
use chipload_toolpath::model::MoveKind;

use super::*;
use crate::view::{Point3, Progress, ToolLook};

/// The play speeds, × the estimated time (decision 7).
pub(super) const SPEEDS: [f64; 3] = [1.0, 10.0, 100.0];

/// A drill's point, as the simulation takes it (docs/specs/simulation.md decision 3).
const DRILL_POINT_RAD: f64 = chipload_pipeline::simulate::DRILL_POINT_RAD;

/// Where playback is in the last generated program.
pub struct Playback {
    /// When each move ends, seconds from the program's start.
    ends: Vec<f64>,
    /// Where the tool is after each move (None until every axis is known).
    positions: Vec<Option<Point3>>,
    /// The seconds shown.
    pub time: f64,
    pub speed: f64,
    /// Playing since this instant (the last frame's), if playing.
    playing: Option<Instant>,
    /// Slider marks: (time, label); operation starts (their number) and tool changes.
    pub marks: Vec<(f64, String)>,
    /// Where each operation's moves are: (moves, operation index, copy).
    spans: Vec<(std::ops::Range<usize>, usize, u32)>,
    /// The tool group of each move range: (moves, tool).
    tools: Vec<(std::ops::Range<usize>, chipload_job::Tool)>,
    kinds: Vec<(MoveKind, Option<f64>)>,
    /// The moves done the canvas shows the stock for; None: the end (the full image).
    pub(super) shown: Option<usize>,
}

impl Playback {
    pub fn new(job: &Job, result: &RunResult) -> Self {
        let mut t = 0.0;
        let ends: Vec<f64> = move_seconds(job, &result.program)
            .into_iter()
            .map(|s| {
                t += s;
                t
            })
            .collect();
        let mut at: (Option<f64>, Option<f64>, Option<f64>) = (None, None, None);
        let positions = result
            .program
            .moves
            .iter()
            .map(|m| {
                at = (m.x.or(at.0), m.y.or(at.1), m.z.or(at.2));
                match at {
                    (Some(x), Some(y), Some(z)) => Some((x, y, z)),
                    _ => None,
                }
            })
            .collect();
        let start_of = |k: usize| if k == 0 { 0.0 } else { ends[k - 1] };
        let mut marks: Vec<(f64, String)> = Vec::new();
        let mut last_op = None;
        for span in &result.op_spans {
            if span.moves.is_empty() {
                continue;
            }
            let label = if last_op == Some(span.operation) {
                String::new()
            } else {
                (span.operation + 1).to_string()
            };
            last_op = Some(span.operation);
            marks.push((start_of(span.moves.start), label));
        }
        for group in result.groups.iter().skip(1) {
            marks.push((start_of(group.moves.start), format!("T{}", group.tool.id)));
        }
        marks.sort_by(|a, b| a.0.total_cmp(&b.0));
        let total = ends.last().copied().unwrap_or(0.0);
        Playback {
            ends,
            positions,
            time: total,
            speed: SPEEDS[1],
            playing: None,
            marks,
            spans: result
                .op_spans
                .iter()
                .map(|s| (s.moves.clone(), s.operation, s.copy))
                .collect(),
            tools: result
                .groups
                .iter()
                .map(|g| (g.moves.clone(), g.tool.clone()))
                .collect(),
            kinds: result
                .program
                .moves
                .iter()
                .map(|m| (m.kind, m.feed))
                .collect(),
            shown: None,
        }
    }

    pub fn total(&self) -> f64 {
        self.ends.last().copied().unwrap_or(0.0)
    }

    pub fn at_end(&self) -> bool {
        self.time >= self.total()
    }

    /// How many moves are done at the time shown.
    pub fn done(&self) -> usize {
        self.ends.partition_point(|&e| e <= self.time)
    }

    /// The tool's point on the move under way, between where it started and where it ends.
    pub fn tool_at(&self) -> Option<Point3> {
        let k = self.done();
        let to = (*self.positions.get(k)?)?;
        let start = if k == 0 { 0.0 } else { self.ends[k - 1] };
        let span = self.ends[k] - start;
        let from = if k == 0 { None } else { self.positions[k - 1] };
        let Some(from) = from else {
            return Some(to);
        };
        let f = if span > 0.0 {
            ((self.time - start) / span).clamp(0.0, 1.0)
        } else {
            0.0
        };
        Some((
            from.0 + (to.0 - from.0) * f,
            from.1 + (to.1 - from.1) * f,
            from.2 + (to.2 - from.2) * f,
        ))
    }

    /// The move the tool is on (or the last one, at the end).
    fn current(&self) -> usize {
        self.done().min(self.ends.len().saturating_sub(1))
    }

    pub fn set_time(&mut self, time: f64) {
        self.time = time.clamp(0.0, self.total());
    }

    /// One move on (+1) or back (−1): to the end of the next move, or of the one before.
    pub fn step_move(&mut self, forward: bool) {
        let t = self.time;
        let target = if forward {
            self.ends.iter().copied().find(|&e| e > t)
        } else {
            self.ends.iter().rev().copied().find(|&e| e < t)
        };
        self.set_time(target.unwrap_or(if forward { self.total() } else { 0.0 }));
    }

    /// To the next (or the previous) operation's or tool's mark.
    pub fn step_mark(&mut self, forward: bool) {
        let t = self.time;
        let target = if forward {
            self.marks.iter().map(|m| m.0).find(|&m| m > t)
        } else {
            self.marks.iter().rev().map(|m| m.0).find(|&m| m < t)
        };
        self.set_time(target.unwrap_or(if forward { self.total() } else { 0.0 }));
    }

    pub fn playing(&self) -> bool {
        self.playing.is_some()
    }

    pub fn play(&mut self, now: Instant) {
        if self.at_end() {
            self.time = 0.0;
        }
        self.playing = Some(now);
    }

    pub fn pause(&mut self) {
        self.playing = None;
    }

    /// Move on by the time since the last frame × the speed; stops at the end.
    pub fn tick(&mut self, now: Instant) {
        let Some(last) = self.playing else {
            return;
        };
        let dt = now.saturating_duration_since(last).as_secs_f64();
        self.set_time(self.time + dt * self.speed);
        self.playing = if self.at_end() { None } else { Some(now) };
    }

    /// The tool cutting at the current move, as the marker draws it.
    pub fn tool_look(&self) -> ToolLook {
        let k = self.current();
        let tool = self.tools.iter().find(|(moves, _)| moves.contains(&k));
        let Some((_, tool)) = tool else {
            return ToolLook {
                radius: 1.0,
                cone: None,
            };
        };
        let radius = tool.diameter / 2.0;
        let cone = |angle: f64, tip: f64| Some((tip.min(radius), 1.0 / (angle / 2.0).tan()));
        ToolLook {
            radius,
            cone: match tool.kind {
                ToolType::Endmill => None,
                ToolType::Drill => cone(DRILL_POINT_RAD, 0.0),
                ToolType::Vbit => cone(
                    tool.angle.unwrap_or(std::f64::consts::FRAC_PI_2),
                    tool.tip_diameter.unwrap_or(0.0) / 2.0,
                ),
            },
        }
    }

    /// "12 min 30 s of 41 min · Operation 3 (Outline), copy 2 · X … Y … Z …, F 1800, cutting"
    pub fn readout(&self, job: Option<&Job>) -> String {
        let mut parts = vec![format!(
            "{} of {}",
            duration_text(self.time),
            duration_text(self.total())
        )];
        let k = self.current();
        if let Some((_, op, copy)) = self.spans.iter().find(|(moves, _, _)| moves.contains(&k)) {
            let name = job
                .and_then(|j| j.operations.get(*op))
                .map(|o| o.name())
                .filter(|n| !n.is_empty());
            let mut text = match name {
                Some(name) => format!("Operation {} ({name})", op + 1),
                None => format!("Operation {}", op + 1),
            };
            if *copy > 1 {
                text += &format!(", copy {copy}");
            }
            parts.push(text);
        }
        if let Some((x, y, z)) = self
            .tool_at()
            .or_else(|| self.positions.last().copied().flatten())
        {
            let mut text = format!("X {x:.3} Y {y:.3} Z {z:.3}");
            match self.kinds.get(k) {
                Some((MoveKind::Rapid, _)) => text += ", rapid",
                Some((_, Some(feed))) => text += &format!(", F {feed:.0}, cutting"),
                _ => {}
            }
            parts.push(text);
        }
        parts.join(" · ")
    }
}

impl ChiploadApp {
    /// A new program: playback starts at the end, or keeps its time if the program is
    /// long enough (docs/specs/playback.md §7).
    pub(super) fn reset_playback(&mut self) {
        let (Some(result), Some(job)) = (&self.result, &self.result_job) else {
            self.playback = None;
            self.scene.progress = None;
            return;
        };
        let mut next = Playback::new(job, result);
        if let Some(old) = &self.playback
            && !old.at_end()
            && old.time <= next.total()
        {
            next.time = old.time;
        }
        self.playback = Some(next);
    }

    /// Set up what the user guide's screenshots show (docs/specs/user-guide.md §3), from
    /// `CHIPLOAD_SCREENSHOT_SETUP`: comma-separated steps `generate`, `simulation`, `3d`,
    /// `half` (the playback half way), `notoolpath` (View > Show toolpath off), `image` (the
    /// first image's window, traced) and `drawing` (the last DXF hidden, the first one's
    /// placement window open), `select` (the first drawn shape selected, with operation 1
    /// open), `part` (the last part's properties). Only in builds made for screenshots.
    #[cfg(feature = "screenshots")]
    pub fn screenshot_setup(&mut self, ctx: &egui::Context, steps: &str) {
        let steps: Vec<&str> = steps.split(',').map(str::trim).collect();
        // The legend would cover part of the canvas.
        self.legend_open = false;
        // Only the shots that ask for it show the simulation (it is on at start).
        self.show_simulation = steps.contains(&"simulation");
        if steps.contains(&"generate") || steps.contains(&"simulation") {
            self.generate_and_wait();
        }
        if steps.contains(&"simulation") {
            self.wait_for_simulation(ctx);
        }
        if steps.contains(&"3d") {
            self.view_3d();
        }
        if steps.contains(&"notoolpath") {
            self.show_toolpath = false;
        }
        let dxfs: Vec<u64> = self
            .edit
            .as_ref()
            .map(|e| e.settings.drawing.dxfs.iter().map(|d| d.id).collect())
            .unwrap_or_default();
        if steps.contains(&"drawing")
            && let (Some(&first), Some(&last)) = (dxfs.first(), dxfs.last())
        {
            if last != first {
                self.toggle_hidden(&format!("dxf:{last}"));
            }
            self.place_open = Some(first);
        }
        let first_drawn = self.edit.as_ref().and_then(|e| {
            e.settings
                .drawing
                .shapes
                .iter()
                .find(|s| s.is_drawn())
                .map(|s| s.id)
        });
        if steps.contains(&"select")
            && let Some(id) = first_drawn
        {
            self.selection = CanvasSelection::Shapes(BTreeSet::from([id]));
            self.focus = Focus::Op(0);
        }
        let last_part = self
            .edit
            .as_ref()
            .and_then(|e| e.settings.parts.last().map(|p| p.id));
        if steps.contains(&"part")
            && let Some(id) = last_part
        {
            self.focus = Focus::Part(id);
        }
        if steps.contains(&"half")
            && let Some(playback) = &mut self.playback
        {
            playback.set_time(playback.total() / 2.0);
        }
        let first_image = self
            .edit
            .as_ref()
            .and_then(|e| e.settings.drawing.images.first().map(|i| i.id));
        if steps.contains(&"image")
            && let Some(id) = first_image
            && self.edit_image(id)
        {
            self.wait_for_image_preview(ctx);
        }
    }

    /// Put the playback's state into the scene: the progress, and the stock as cut so far
    /// (only when the moves done change).
    pub(super) fn sync_playback(&mut self, ctx: &egui::Context) {
        let Some(playback) = &mut self.playback else {
            self.scene.progress = None;
            return;
        };
        playback.tick(Instant::now());
        if playback.playing() {
            ctx.request_repaint();
        }
        if playback.at_end() {
            self.scene.progress = None;
            if playback.shown.take().is_some() {
                self.scene.sim.clone_from(&self.sim_full_layer);
            }
            return;
        }
        let done = playback.done();
        self.scene.progress = Some(Progress {
            done,
            tool_at: playback.tool_at(),
            tool: playback.tool_look(),
        });
        if playback.shown == Some(done) {
            return;
        }
        playback.shown = Some(done);
        self.show_stock_after(ctx, done);
    }

    /// The canvas's stock after `done` moves, from the simulation's history.
    fn show_stock_after(&mut self, ctx: &egui::Context, done: usize) {
        let (Some(history), Some(sim), Some(full)) =
            (&self.sim_history, &self.simulation, &self.sim_full_layer)
        else {
            return;
        };
        let stock = history.after(done, &sim.numbers);
        let image = simulation::image_of(&stock);
        let texture = match &mut self.sim_playback_texture {
            Some(texture) => {
                texture.set(image, egui::TextureOptions::LINEAR);
                texture.id()
            }
            None => {
                let texture =
                    ctx.load_texture("simulation-playback", image, egui::TextureOptions::LINEAR);
                let id = texture.id();
                self.sim_playback_texture = Some(texture);
                id
            }
        };
        self.scene.sim = Some(crate::view::SimLayer {
            texture,
            min: full.min,
            max: full.max,
            surface: simulation::surface_of(&stock),
        });
    }

    /// The bar under the canvas: play, speed, the slider with its marks, and where it is.
    pub(super) fn playback_bar(&mut self, ui: &mut egui::Ui) {
        let job = self.result_job.clone();
        let Some(playback) = &mut self.playback else {
            return;
        };
        let now = Instant::now();
        // Keys, when no text field has them (docs/specs/playback.md §1).
        if !ui.ctx().egui_wants_keyboard_input() {
            ui.input(|i| {
                let shift = i.modifiers.shift;
                if i.key_pressed(Key::ArrowRight) {
                    playback.pause();
                    if shift {
                        playback.step_mark(true);
                    } else {
                        playback.step_move(true);
                    }
                }
                if i.key_pressed(Key::ArrowLeft) {
                    playback.pause();
                    if shift {
                        playback.step_mark(false);
                    } else {
                        playback.step_move(false);
                    }
                }
                if i.key_pressed(Key::Home) {
                    playback.pause();
                    playback.set_time(0.0);
                }
                if i.key_pressed(Key::End) {
                    playback.pause();
                    playback.set_time(playback.total());
                }
            });
        }
        ui.horizontal(|ui| {
            let play = if playback.playing() { "⏸" } else { "▶" };
            if ui
                .button(play)
                .on_hover_text("Play the program at the chosen speed")
                .clicked()
            {
                if playback.playing() {
                    playback.pause();
                } else {
                    playback.play(now);
                }
            }
            ComboBox::from_id_salt("playback-speed")
                .width(56.0)
                .selected_text(format!("×{}", playback.speed))
                .show_ui(ui, |ui| {
                    for speed in SPEEDS {
                        if ui
                            .selectable_label(playback.speed == speed, format!("×{speed}"))
                            .clicked()
                        {
                            playback.speed = speed;
                        }
                    }
                });
            let width = ui.available_width().max(100.0);
            let (rect, response) =
                ui.allocate_exact_size(egui::vec2(width, 26.0), egui::Sense::click_and_drag());
            let total = playback.total().max(f64::MIN_POSITIVE);
            let x_of = |t: f64| rect.left() + (t / total) as f32 * rect.width();
            if (response.dragged() || response.clicked())
                && let Some(p) = response.interact_pointer_pos()
            {
                playback.pause();
                let f = f64::from(((p.x - rect.left()) / rect.width()).clamp(0.0, 1.0));
                playback.set_time(f * total);
            }
            let painter = ui.painter_at(rect.expand(2.0));
            let visuals = ui.visuals();
            let track_y = rect.bottom() - 7.0;
            painter.line_segment(
                [
                    egui::pos2(rect.left(), track_y),
                    egui::pos2(rect.right(), track_y),
                ],
                egui::Stroke::new(3.0, visuals.widgets.inactive.bg_fill),
            );
            let here = x_of(playback.time);
            painter.line_segment(
                [egui::pos2(rect.left(), track_y), egui::pos2(here, track_y)],
                egui::Stroke::new(3.0, visuals.selection.bg_fill),
            );
            for (t, label) in &playback.marks {
                let x = x_of(*t);
                painter.line_segment(
                    [egui::pos2(x, track_y - 5.0), egui::pos2(x, track_y + 4.0)],
                    egui::Stroke::new(1.0, visuals.text_color()),
                );
                if !label.is_empty() {
                    painter.text(
                        egui::pos2(x + 2.0, rect.top()),
                        egui::Align2::LEFT_TOP,
                        label,
                        egui::FontId::proportional(10.0),
                        visuals.weak_text_color(),
                    );
                }
            }
            painter.circle_filled(
                egui::pos2(here, track_y),
                6.0,
                visuals.selection.stroke.color,
            );
        });
        ui.label(RichText::new(playback.readout(job.as_ref())).small());
    }
}
