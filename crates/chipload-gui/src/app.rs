// SPDX-License-Identifier: GPL-3.0-or-later
//! Main window: open a DXF and a job, generate, inspect the result, save checked G-code.
//! Thin by design: everything that decides geometry or G-code
//! is a call into the pipeline.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use chipload_core::Point;
use chipload_core::chain::Contour;
use chipload_dxf::{ImportError, import_dxf};
use chipload_job::{Job, Selector, Units, load_job};
use chipload_pipeline::report::{format_contours, format_import_report};
use chipload_pipeline::{PipelineError, RunResult, run_job, write_atomically};
use chipload_toolpath::select::select_contours;
use eframe::egui::{
    self, CentralPanel, Key, KeyboardShortcut, MenuBar, Modifiers, Panel, RichText, ScrollArea,
    Sense, TextEdit, ViewportCommand,
};

use crate::view::{ContourShape, Kind, Scene, ToolpathShape, View};

const STATUS_SECONDS: u64 = 5;
const ERROR_STATUS_SECONDS: u64 = 10;
const OPEN_DXF: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::O);
const OPEN_JOB: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::J);
const GENERATE: KeyboardShortcut = KeyboardShortcut::new(Modifiers::NONE, Key::F5);
const SAVE: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::S);
const FIT: KeyboardShortcut = KeyboardShortcut::new(Modifiers::NONE, Key::F);
const COPY_SELECTOR: KeyboardShortcut =
    KeyboardShortcut::new(Modifiers::COMMAND.plus(Modifiers::SHIFT), Key::C);
const QUIT: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::Q);

/// A small modal asking for something before an action can continue.
#[derive(Debug, Clone, PartialEq)]
enum Prompt {
    /// The DXF declares no units and no job is open.
    Units,
    /// Type or paste a path (for systems without a native file dialog).
    OpenPath(String),
    SavePath(String),
}

pub struct ChiploadApp {
    pub dxf_path: Option<PathBuf>,
    pub job_path: Option<PathBuf>,
    pub job: Option<Job>,
    pub result: Option<RunResult>,
    /// What the canvas shows.
    pub contours: Vec<Contour>,
    /// Units the user chose for a unitless DXF (used while no job says otherwise).
    dxf_units: Option<Units>,
    pub scene: Scene,
    view: View,
    fit_pending: bool,
    pub selected: BTreeSet<usize>,
    pub report: Vec<String>,
    status: Option<(String, Instant)>,
    cursor: Option<Point>,
    prompt: Option<Prompt>,
    title: String,
}

impl Default for ChiploadApp {
    fn default() -> Self {
        ChiploadApp {
            dxf_path: None,
            job_path: None,
            job: None,
            result: None,
            contours: Vec::new(),
            dxf_units: None,
            scene: Scene::default(),
            view: View::default(),
            fit_pending: false,
            selected: BTreeSet::new(),
            report: vec![
                "Open a DXF and a job (File menu, or chipload-gui part.dxf job.json).".into(),
            ],
            status: None,
            cursor: None,
            prompt: None,
            title: String::new(),
        }
    }
}

fn file_name(path: &Path) -> String {
    path.file_name().map_or_else(
        || path.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    )
}

/// Handles of the contours' entities, in contour order, without repeats.
fn handles(contours: &[&Contour]) -> Vec<String> {
    let mut seen: Vec<String> = Vec::new();
    for c in contours {
        for part in &c.parts {
            if !seen.contains(&part.segment.handle) {
                seen.push(part.segment.handle.clone());
            }
        }
    }
    seen
}

fn selector_text(handles: &[String]) -> String {
    let quoted: Vec<String> = handles.iter().map(|h| format!("\"{h}\"")).collect();
    format!("{{\"handles\": [{}]}}", quoted.join(", "))
}

impl ChiploadApp {
    // -- operations (also called directly by tests)

    /// Show a drawing. Units come from the job if one is open, else from the file, else
    /// from the user. Never guessed.
    pub fn open_dxf(&mut self, path: &Path) -> bool {
        self.dxf_path = Some(path.to_path_buf());
        self.dxf_units = None;
        self.invalidate();
        self.show_drawing()
    }

    pub fn open_job(&mut self, path: &Path) -> bool {
        let job = match load_job(path) {
            Ok(job) => job,
            Err(err) => {
                self.error(vec![
                    format!("Cannot open job {}:", file_name(path)),
                    err.to_string(),
                ]);
                return false;
            }
        };
        self.scene.stock = Some(job.stock.xy_bounds());
        self.job = Some(job);
        self.job_path = Some(path.to_path_buf());
        self.invalidate();
        if self.dxf_path.is_some() {
            self.show_drawing();
        } else {
            self.report = vec![
                format!("Job: {}", file_name(path)),
                "Open a DXF to continue.".into(),
            ];
        }
        self.fit_pending = true;
        true
    }

    /// The user answered the units question for a unitless DXF.
    pub fn choose_units(&mut self, units: Units) -> bool {
        self.prompt = None;
        self.dxf_units = Some(units);
        self.show_drawing()
    }

    pub fn generate(&mut self) -> bool {
        let (Some(job), Some(dxf)) = (&self.job, &self.dxf_path) else {
            self.error(vec!["Open both a DXF and a job before generating.".into()]);
            return false;
        };
        match run_job(job, dxf) {
            Ok(result) => {
                self.contours.clone_from(&result.contours);
                self.scene.contours = result.contours.iter().map(ContourShape::new).collect();
                self.scene.toolpath = Some(ToolpathShape::new(&result.program, false));
                self.scene.tabs.clone_from(&result.tabs);
                let mut lines = vec![format!(
                    "Generated {} lines for {}. Check passed.",
                    result.gcode.matches('\n').count(),
                    result.post.name
                )];
                lines.extend(result.notes.iter().map(|n| format!("warning: {n}")));
                lines.extend(
                    result
                        .check_warnings
                        .iter()
                        .map(|w| format!("check warning: {}", w.message)),
                );
                lines.push(String::new());
                lines.extend(format_import_report(&result.import_report));
                lines.extend(format_contours(&result.contours));
                self.report = lines;
                self.result = Some(result);
                true
            }
            Err(PipelineError::CheckFailed { errors, program }) => {
                self.scene.toolpath = Some(ToolpathShape::new(&program, true));
                self.scene.tabs.clear();
                let mut lines = vec!["G-code check FAILED. Nothing can be saved.".to_string()];
                lines.extend(errors);
                self.error(lines);
                false
            }
            Err(err) => {
                self.scene.toolpath = None;
                self.scene.tabs.clear();
                self.error(vec!["Generation failed:".into(), err.to_string()]);
                false
            }
        }
    }

    pub fn save_gcode(&mut self, path: &Path) -> bool {
        let Some(result) = &self.result else {
            return false;
        };
        match write_atomically(path, &result.gcode) {
            Ok(()) => {
                self.set_status(format!("Saved {}", path.display()), STATUS_SECONDS);
                true
            }
            Err(err) => {
                self.error(vec![
                    format!("Could not save {}:", path.display()),
                    err.to_string(),
                ]);
                false
            }
        }
    }

    /// A job selector for the selected contours, e.g. {"handles": ["5C", "5D"]}.
    pub fn selector_json(&self) -> Option<String> {
        let selected = self.selected_contours();
        let handles = handles(&selected);
        (!handles.is_empty()).then(|| selector_text(&handles))
    }

    /// Click on the canvas: select one contour, Ctrl toggles, empty space clears.
    pub fn click(&mut self, picked: Option<usize>, toggle: bool) {
        match (picked, toggle) {
            (Some(id), true) => {
                if !self.selected.remove(&id) {
                    self.selected.insert(id);
                }
            }
            (Some(id), false) => {
                self.selected = BTreeSet::from([id]);
            }
            (None, true) => {}
            (None, false) => self.selected.clear(),
        }
    }

    pub fn selection_lines(&self) -> Vec<String> {
        let selected = self.selected_contours();
        if selected.is_empty() {
            return vec![
                "Click a contour to select it.".into(),
                "Ctrl+click adds or removes, Esc clears.".into(),
            ];
        }
        let mut lines = vec![format!("{} selected:", selected.len())];
        for c in &selected {
            lines.push(format!(
                "  #{:<4} {:<8} layer {:<12} handles {}",
                c.id,
                Kind::of(c).name(),
                c.layers().join(", "),
                handles(&[c]).join(", ")
            ));
        }
        let handles = handles(&selected);
        lines.push(String::new());
        lines.push(format!("Selector: {}", selector_text(&handles)));
        lines.push(String::new());
        lines.extend(self.selector_preview(&handles));
        lines
    }

    // -- internals

    fn selected_contours(&self) -> Vec<&Contour> {
        self.contours
            .iter()
            .filter(|c| self.selected.contains(&c.id))
            .collect()
    }

    /// What a job would actually cut with this selector, using the toolpath crate's own
    /// matching.
    fn selector_preview(&self, handles: &[String]) -> Vec<String> {
        let selector = Selector {
            layer: None,
            handles: Some(handles.to_vec()),
        };
        let selection = match select_contours(&self.contours, &selector, "Selector") {
            Ok(selection) => selection,
            Err(err) => return vec![format!("As a job selector: {err}")],
        };
        let ids = |list: &[usize]| {
            list.iter()
                .map(|i| format!("#{i}"))
                .collect::<Vec<_>>()
                .join(", ")
        };
        let matched: Vec<usize> = selection.contours.iter().map(|c| c.id).collect();
        let extra: Vec<usize> = matched
            .iter()
            .copied()
            .filter(|id| !self.selected.contains(id))
            .collect();
        let mut lines = vec![format!("As a job selector it cuts: {}", ids(&matched))];
        if !extra.is_empty() {
            lines.push(format!(
                "  Note: also {}, which share a handle (shapes from one block INSERT share its \
                 handle)",
                ids(&extra)
            ));
        }
        lines.extend(selection.notes.iter().map(|n| format!("  {n}")));
        lines
    }

    fn show_drawing(&mut self) -> bool {
        let Some(path) = self.dxf_path.clone() else {
            return false;
        };
        let units = self.job.as_ref().map(|j| j.source.units).or(self.dxf_units);
        let imported = match import_dxf(&path, units) {
            Ok(imported) => imported,
            Err(ImportError::UnitsMissing) => {
                self.prompt = Some(Prompt::Units);
                self.report = vec![format!(
                    "{} does not say which units it uses; choose mm or inch.",
                    file_name(&path)
                )];
                return false;
            }
            Err(err) => {
                self.contours.clear();
                self.scene.contours.clear();
                self.error(vec![
                    format!("Cannot open {}:", file_name(&path)),
                    err.to_string(),
                ]);
                return false;
            }
        };
        self.contours = imported.contours;
        self.selected.clear();
        self.scene.contours = self.contours.iter().map(ContourShape::new).collect();
        self.fit_pending = true;
        let mut lines = vec![format!("Drawing: {}", file_name(&path))];
        lines.extend(format_import_report(&imported.report));
        lines.extend(format_contours(&self.contours));
        self.report = lines;
        true
    }

    /// Inputs changed: the old toolpath and G-code no longer apply.
    fn invalidate(&mut self) {
        self.result = None;
        self.scene.toolpath = None;
        self.scene.tabs.clear();
    }

    fn error(&mut self, lines: Vec<String>) {
        if let Some(first) = lines.first() {
            self.set_status(first.clone(), ERROR_STATUS_SECONDS);
        }
        self.report = lines;
    }

    fn set_status(&mut self, text: String, seconds: u64) {
        self.status = Some((text, Instant::now() + Duration::from_secs(seconds)));
    }

    fn can_generate(&self) -> bool {
        self.job.is_some() && self.dxf_path.is_some()
    }

    fn suggested_output(&self) -> String {
        match (&self.dxf_path, &self.result) {
            (Some(dxf), Some(result)) => dxf
                .with_extension(result.post.file_extension.trim_start_matches('.'))
                .display()
                .to_string(),
            _ => String::new(),
        }
    }

    // -- user interface

    fn ask_open_dxf(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("DXF drawings", &["dxf"])
            .pick_file()
        {
            self.open_dxf(&path);
        }
    }

    fn ask_open_job(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Chipload jobs", &["json"])
            .pick_file()
        {
            self.open_job(&path);
        }
    }

    fn ask_save(&mut self) {
        if self.result.is_none() {
            return;
        }
        let suggested = PathBuf::from(self.suggested_output());
        let mut dialog = rfd::FileDialog::new();
        if let Some(name) = suggested.file_name() {
            dialog = dialog.set_file_name(name.to_string_lossy());
        }
        if let Some(dir) = suggested.parent() {
            dialog = dialog.set_directory(dir);
        }
        if let Some(path) = dialog.save_file() {
            self.save_gcode(&path);
        }
    }

    fn copy_selector(&mut self, ctx: &egui::Context) {
        if let Some(text) = self.selector_json() {
            ctx.copy_text(text);
            self.set_status("Selector copied to the clipboard".into(), STATUS_SECONDS);
        }
    }

    fn shortcuts(&mut self, ctx: &egui::Context) {
        // Not while typing in a text field.
        if ctx.egui_wants_keyboard_input() {
            return;
        }
        let pressed = |s: &KeyboardShortcut| ctx.input_mut(|i| i.consume_shortcut(s));
        if pressed(&COPY_SELECTOR) {
            self.copy_selector(ctx);
        }
        if pressed(&OPEN_DXF) {
            self.ask_open_dxf();
        }
        if pressed(&OPEN_JOB) {
            self.ask_open_job();
        }
        if pressed(&GENERATE) && self.can_generate() {
            self.generate();
        }
        if pressed(&SAVE) {
            self.ask_save();
        }
        if pressed(&FIT) {
            self.fit_pending = true;
        }
        if pressed(&QUIT) {
            ctx.send_viewport_cmd(ViewportCommand::Close);
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
            self.selected.clear();
        }
    }

    fn menu(&mut self, ui: &mut egui::Ui) {
        MenuBar::new().ui(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("Open DXF…  (Ctrl+O)").clicked() {
                    self.ask_open_dxf();
                }
                if ui.button("Open Job…  (Ctrl+J)").clicked() {
                    self.ask_open_job();
                }
                if ui.button("Open path…").clicked() {
                    self.prompt = Some(Prompt::OpenPath(String::new()));
                }
                ui.separator();
                let can_save = self.result.is_some();
                if ui
                    .add_enabled(can_save, egui::Button::new("Save G-code…  (Ctrl+S)"))
                    .clicked()
                {
                    self.ask_save();
                }
                if ui
                    .add_enabled(can_save, egui::Button::new("Save G-code to path…"))
                    .clicked()
                {
                    self.prompt = Some(Prompt::SavePath(self.suggested_output()));
                }
                ui.separator();
                if ui.button("Quit  (Ctrl+Q)").clicked() {
                    ui.ctx().send_viewport_cmd(ViewportCommand::Close);
                }
            });
            ui.menu_button("Edit", |ui| {
                let enabled = !self.selected.is_empty();
                if ui
                    .add_enabled(
                        enabled,
                        egui::Button::new("Copy as Selector  (Ctrl+Shift+C)"),
                    )
                    .clicked()
                {
                    self.copy_selector(ui.ctx());
                }
            });
            ui.menu_button("Toolpath", |ui| {
                if ui
                    .add_enabled(self.can_generate(), egui::Button::new("Generate  (F5)"))
                    .clicked()
                {
                    self.generate();
                }
            });
            ui.menu_button("View", |ui| {
                if ui.button("Fit View  (F)").clicked() {
                    self.fit_pending = true;
                }
            });
            ui.separator();
            if ui.button("Open DXF").clicked() {
                self.ask_open_dxf();
            }
            if ui.button("Open Job").clicked() {
                self.ask_open_job();
            }
            if ui
                .add_enabled(self.can_generate(), egui::Button::new("Generate"))
                .clicked()
            {
                self.generate();
            }
            if ui
                .add_enabled(self.result.is_some(), egui::Button::new("Save G-code"))
                .clicked()
            {
                self.ask_save();
            }
            if ui.button("Fit").clicked() {
                self.fit_pending = true;
            }
        });
    }

    fn status_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if let Some((text, until)) = &self.status {
                if Instant::now() < *until {
                    ui.label(text);
                    ui.ctx().request_repaint_after(*until - Instant::now());
                } else {
                    self.status = None;
                }
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if let Some((x, y)) = self.cursor {
                    ui.monospace(format!("X {x:.3}  Y {y:.3} mm"));
                }
            });
        });
    }

    fn side_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("Report");
        let height = (ui.available_height() * 0.6).max(120.0);
        ScrollArea::both()
            .id_salt("report")
            .max_height(height)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.add(
                    egui::Label::new(RichText::new(self.report.join("\n")).monospace()).extend(),
                );
            });
        ui.separator();
        ui.heading("Selection");
        let lines = self.selection_lines();
        ScrollArea::both()
            .id_salt("selection")
            .max_height(ui.available_height() - 40.0)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.add(egui::Label::new(RichText::new(lines.join("\n")).monospace()).extend());
            });
        if ui
            .add_enabled(
                !self.selected.is_empty(),
                egui::Button::new("Copy as selector"),
            )
            .clicked()
        {
            self.copy_selector(ui.ctx());
        }
    }

    fn canvas(&mut self, ui: &mut egui::Ui) {
        let (response, painter) = ui.allocate_painter(ui.available_size(), Sense::click_and_drag());
        let rect = response.rect;
        if self.fit_pending
            && let Some(bounds) = self.scene.bounds()
        {
            self.view.fit(bounds, rect);
            self.fit_pending = false;
        }
        if response.dragged() {
            self.view.pan(response.drag_delta());
        }
        if let Some(pos) = response.hover_pos() {
            let (scroll, zoom) = ui.input(|i| (i.smooth_scroll_delta.y, i.zoom_delta()));
            let factor = f64::from(zoom) * (f64::from(scroll) * 0.0015).exp();
            if (factor - 1.0).abs() > 1e-6 {
                self.view.zoom_at(pos, factor);
            }
            self.cursor = Some(self.view.to_world(pos));
        } else {
            self.cursor = None;
        }
        if response.clicked()
            && let Some(pos) = response.interact_pointer_pos()
        {
            let toggle = ui.input(|i| i.modifiers.command || i.modifiers.ctrl);
            let picked = self.scene.pick(&self.view, pos);
            self.click(picked, toggle);
        }
        self.scene.paint(&painter, &self.view, &self.selected);
    }

    fn prompts(&mut self, ctx: &egui::Context) {
        let Some(prompt) = self.prompt.clone() else {
            return;
        };
        match prompt {
            Prompt::Units => {
                egui::Window::new("Drawing units")
                    .collapsible(false)
                    .resizable(false)
                    .show(ctx, |ui| {
                        ui.label("This DXF does not say which units it uses.");
                        ui.label("What units was it drawn in?");
                        ui.horizontal(|ui| {
                            if ui.button("mm").clicked() {
                                self.choose_units(Units::Mm);
                            }
                            if ui.button("inch").clicked() {
                                self.choose_units(Units::Inch);
                            }
                            if ui.button("Cancel").clicked() {
                                self.prompt = None;
                                self.error(vec![
                                    "Drawing not opened: its units are unknown.".into(),
                                ]);
                            }
                        });
                    });
            }
            Prompt::OpenPath(mut text) => {
                let mut keep = true;
                egui::Window::new("Open path")
                    .collapsible(false)
                    .show(ctx, |ui| {
                        ui.label("Path to a .dxf drawing or a .json job:");
                        ui.add(TextEdit::singleline(&mut text).desired_width(420.0));
                        ui.horizontal(|ui| {
                            let path = PathBuf::from(text.trim());
                            if ui.button("Open as DXF").clicked() {
                                self.open_dxf(&path);
                                keep = false;
                            }
                            if ui.button("Open as job").clicked() {
                                self.open_job(&path);
                                keep = false;
                            }
                            if ui.button("Cancel").clicked() {
                                keep = false;
                            }
                        });
                    });
                // Opening may itself ask for units; only replace this prompt if it didn't.
                if keep {
                    self.prompt = Some(Prompt::OpenPath(text));
                } else if self.prompt == Some(Prompt::OpenPath(text)) {
                    self.prompt = None;
                }
            }
            Prompt::SavePath(mut text) => {
                let mut keep = true;
                egui::Window::new("Save G-code")
                    .collapsible(false)
                    .show(ctx, |ui| {
                        ui.label("Save the checked G-code to:");
                        ui.add(TextEdit::singleline(&mut text).desired_width(420.0));
                        ui.horizontal(|ui| {
                            if ui.button("Save").clicked() {
                                self.save_gcode(&PathBuf::from(text.trim()));
                                keep = false;
                            }
                            if ui.button("Cancel").clicked() {
                                keep = false;
                            }
                        });
                    });
                self.prompt = keep.then_some(Prompt::SavePath(text));
            }
        }
    }

    fn update_title(&mut self, ctx: &egui::Context) {
        let mut parts = vec!["Chipload".to_string()];
        parts.extend(
            [&self.dxf_path, &self.job_path]
                .into_iter()
                .flatten()
                .map(|p| file_name(p)),
        );
        let title = parts.join(" — ");
        if title != self.title {
            ctx.send_viewport_cmd(ViewportCommand::Title(title.clone()));
            self.title = title;
        }
    }
}

impl eframe::App for ChiploadApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.shortcuts(&ctx);
        Panel::top("menu").show(ui, |ui| self.menu(ui));
        Panel::bottom("status").show(ui, |ui| self.status_bar(ui));
        Panel::right("side")
            .resizable(true)
            .default_size(440.0)
            .show(ui, |ui| self.side_panel(ui));
        CentralPanel::no_frame().show(ui, |ui| self.canvas(ui));
        self.prompts(&ctx);
        self.update_title(&ctx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn corpus(rel: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/corpus")
            .join(rel)
    }

    #[test]
    fn open_generate_and_save_the_checked_program() {
        let mut app = ChiploadApp::default();
        assert!(!app.generate(), "needs both inputs");
        assert!(app.open_dxf(&corpus("dxf/messy_2.dxf")));
        assert_eq!(app.contours.len(), 22);
        assert!(app.open_job(&corpus("jobs/v3/messy_2_lower_tabs.json")));
        assert!(app.generate());
        assert!(app.report[0].ends_with("Check passed."), "{:?}", app.report);
        let toolpath = app.scene.toolpath.as_ref().unwrap();
        assert!(!toolpath.rejected && !toolpath.feeds.is_empty());
        assert_eq!(app.scene.tabs.len(), 4);

        let dir = std::env::temp_dir().join(format!("chipload-gui-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let out = dir.join("out.nc");
        assert!(app.save_gcode(&out));
        let saved = std::fs::read_to_string(&out).unwrap();
        std::fs::remove_dir_all(&dir).unwrap();
        // Exactly what the pipeline (and `chipload run`) produces.
        let job = load_job(&corpus("jobs/v3/messy_2_lower_tabs.json")).unwrap();
        let expected = run_job(&job, &corpus("dxf/messy_2.dxf")).unwrap().gcode;
        assert_eq!(saved, expected);
    }

    #[test]
    fn a_new_drawing_invalidates_the_toolpath() {
        let mut app = ChiploadApp::default();
        app.open_job(&corpus("jobs/v3/simple_part_tabs.json"));
        app.open_dxf(&corpus("dxf/simple_part.dxf"));
        assert!(app.generate());
        app.open_dxf(&corpus("dxf/simple_part.dxf"));
        assert!(app.result.is_none() && app.scene.toolpath.is_none());
        assert!(!app.save_gcode(Path::new("/nonexistent/never.nc")));
    }

    #[test]
    fn unitless_drawing_asks_for_units() {
        let mut app = ChiploadApp::default();
        assert!(!app.open_dxf(&corpus("dxf/messy.dxf")));
        assert_eq!(app.prompt, Some(Prompt::Units));
        assert!(app.contours.is_empty());
        assert!(app.choose_units(Units::Mm));
        assert!(!app.contours.is_empty());
        assert!(
            app.report
                .iter()
                .any(|l| l.contains("using job units") || l.contains("from job"))
        );
    }

    #[test]
    fn selection_and_selector() {
        let mut app = ChiploadApp::default();
        app.open_dxf(&corpus("dxf/messy_2.dxf"));
        assert_eq!(app.selector_json(), None);
        app.click(Some(4), false);
        app.click(Some(6), true);
        assert_eq!(app.selected, BTreeSet::from([4, 6]));
        let json = app.selector_json().unwrap();
        assert!(json.starts_with("{\"handles\": [\""), "{json}");
        let lines = app.selection_lines();
        assert!(lines[0] == "2 selected:");
        assert!(
            lines
                .iter()
                .any(|l| l == "As a job selector it cuts: #4, #6"),
            "{lines:?}"
        );
        app.click(Some(6), true);
        assert_eq!(app.selected, BTreeSet::from([4]));
        app.click(None, false);
        assert!(app.selected.is_empty());
    }

    #[test]
    fn a_failed_job_reports_without_a_toolpath() {
        let mut app = ChiploadApp::default();
        app.open_dxf(&corpus("dxf/messy_2.dxf"));
        assert!(!app.open_job(Path::new("/nonexistent/job.json")));
        assert!(app.report[0].starts_with("Cannot open job"));
    }
}
