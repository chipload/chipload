// SPDX-License-Identifier: GPL-3.0-or-later
//! *Inlay…* (docs/specs/vcarve-inlay.md §4): a V-carve inlay of the selection, its V-bit,
//! depths, glue gap, floor end mill and plug board asked in a dialog with a drawing of the
//! two cuts. The pair itself is `chipload_pipeline::inlay`'s.

use chipload_job::{Tool, ToolType};
use chipload_pipeline::inlay::{
    DEFAULT_GLUE_GAP_MM, DEFAULT_PLUG_START_MM, DEFAULT_PLUG_THICKNESS_MM, DEFAULT_POCKET_DEPTH_MM,
    InlayRequest, add_inlay,
};
use egui::{Color32, Stroke, pos2, vec2};

use super::forms::tool_label;
use super::*;

/// What the inlay dialog holds while it is open.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct InlaySettings {
    /// The V-bit both are carved with: a project tool's id.
    pub tool: Option<i64>,
    /// The pocket's depth P, mm.
    pub depth: f64,
    /// The plug's start depth D, mm.
    pub plug_start: f64,
    /// The glue gap g, mm.
    pub gap: f64,
    /// The end mill of both floor passes, if any.
    pub floor: Option<i64>,
    /// The plug board's thickness, mm.
    pub thickness: f64,
}

impl InlaySettings {
    /// The defaults of decision 5: the project's first V-bit, and its smallest end mill for
    /// the floor passes.
    fn for_tools(tools: &[Tool]) -> Self {
        let vbit = tools
            .iter()
            .find(|t| t.kind == ToolType::Vbit && t.angle.is_some());
        let floor = tools
            .iter()
            .filter(|t| t.kind == ToolType::Endmill)
            .min_by(|a, b| a.diameter.total_cmp(&b.diameter));
        Self {
            tool: vbit.map(|t| t.id),
            depth: DEFAULT_POCKET_DEPTH_MM,
            plug_start: DEFAULT_PLUG_START_MM,
            gap: DEFAULT_GLUE_GAP_MM,
            floor: floor.map(|t| t.id),
            thickness: DEFAULT_PLUG_THICKNESS_MM,
        }
    }
}

impl ChiploadApp {
    /// *Inlay…*: the dialog for an inlay of the selection.
    pub fn start_inlay(&mut self) {
        let Some(edit) = self.edit.as_ref() else {
            return;
        };
        let settings = InlaySettings::for_tools(&edit.job.tools);
        self.dialogs.prompt = Some(Prompt::Inlay(settings));
    }

    /// Whether *Inlay…* can start: shapes are selected.
    pub(super) fn can_inlay(&self) -> bool {
        self.edit.is_some() && !self.selected_ids().is_empty()
    }

    /// Make the inlay of the selection, as one undo step: the pocket in the active part, the
    /// plug in a new setup (docs/specs/vcarve-inlay.md §4). False, with the reason in the
    /// report, when it is refused.
    pub(super) fn make_inlay(&mut self, settings: &InlaySettings) -> bool {
        self.commit();
        let Some(tool) = settings.tool else {
            self.error(vec![t!(self, "inlay-no-vbit")]);
            return false;
        };
        let selection = self.op_selection();
        let library = self.library().map(|l| l.tools.clone());
        let lang = self.prefs.language;
        let Some(edit) = self.edit.as_ref() else {
            return false;
        };
        let request = InlayRequest {
            select: selection.selector(&self.contours, false),
            tool,
            depth: settings.depth,
            plug_start: settings.plug_start,
            gap: settings.gap,
            floor_tool: settings.floor,
            plug_thickness: settings.thickness,
        };
        match add_inlay(&edit.job, &request, &self.contours, library.as_ref(), lang) {
            Ok((job, made)) => {
                if let Some(edit) = self.edit.as_mut() {
                    edit.adopt(job);
                }
                self.focus = Focus::Op(made.pocket);
                self.commit();
                self.set_report(Report::info([t!(
                    self,
                    "inlay-made",
                    pocket = made.pocket + 1,
                    plug = made.plug + 1,
                    setup = made.setup
                )]));
                true
            }
            Err(err) => {
                self.error_report(Report::failed(t!(self, "inlay-refused"), err.message()));
                false
            }
        }
    }

    /// The dialog: the V-bit and floor end mill, the four numbers, a drawing of the two
    /// cuts, *Make inlay*.
    pub(super) fn inlay_prompt(&mut self, ctx: &egui::Context, mut settings: InlaySettings) {
        let tools: Vec<Tool> = self
            .edit
            .as_ref()
            .map(|e| e.job.tools.clone())
            .unwrap_or_default();
        let units = self.prefs.units;
        let mut answer: Option<bool> = None;
        let modal = egui::Modal::new(egui::Id::new("inlay")).show(ctx, |ui| {
            ui.set_width(420.0);
            ui.label(crate::style::bold(
                ui,
                egui::RichText::new(t!(ui, "inlay-title")).heading(),
            ));
            ui.add_space(4.0);
            ui.label(t!(ui, "inlay-body"));
            ui.add_space(8.0);
            egui::Grid::new("inlay-grid")
                .num_columns(2)
                .spacing([12.0, 6.0])
                .show(ui, |ui| {
                    ui.label(t!(ui, "inlay-vbit"));
                    let none = t!(ui, "inlay-none");
                    let labels: Vec<(i64, String)> = tools
                        .iter()
                        .map(|t| (t.id, tool_label(ui, t, units)))
                        .collect();
                    let label_of = |id: i64| {
                        labels
                            .iter()
                            .find(|(k, _)| *k == id)
                            .map_or_else(String::new, |(_, l)| l.clone())
                    };
                    let shown = |id: Option<i64>| id.map_or_else(|| none.clone(), label_of);
                    egui::ComboBox::from_id_salt("inlay-vbit")
                        .selected_text(shown(settings.tool))
                        .show_ui(ui, |ui| {
                            for t in tools.iter().filter(|t| t.kind == ToolType::Vbit) {
                                ui.selectable_value(&mut settings.tool, Some(t.id), label_of(t.id));
                            }
                        });
                    ui.end_row();
                    ui.label(t!(ui, "inlay-depth"))
                        .on_hover_text(t!(ui, "inlay-depth-hover"));
                    ui.add(crate::units::length(&mut settings.depth).range(0.1..=100.0));
                    ui.end_row();
                    ui.label(t!(ui, "inlay-plug-start"))
                        .on_hover_text(t!(ui, "inlay-plug-start-hover"));
                    ui.add(crate::units::length(&mut settings.plug_start).range(0.1..=100.0));
                    ui.end_row();
                    ui.label(t!(ui, "inlay-gap"))
                        .on_hover_text(t!(ui, "inlay-gap-hover"));
                    ui.add(crate::units::length(&mut settings.gap).range(0.1..=50.0));
                    ui.end_row();
                    ui.label(t!(ui, "inlay-floor"))
                        .on_hover_text(t!(ui, "inlay-floor-hover"));
                    egui::ComboBox::from_id_salt("inlay-floor")
                        .selected_text(shown(settings.floor))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut settings.floor, None, none.clone());
                            for t in tools.iter().filter(|t| t.kind == ToolType::Endmill) {
                                ui.selectable_value(
                                    &mut settings.floor,
                                    Some(t.id),
                                    label_of(t.id),
                                );
                            }
                        });
                    ui.end_row();
                    ui.label(t!(ui, "inlay-thickness"))
                        .on_hover_text(t!(ui, "inlay-thickness-hover"));
                    ui.add(crate::units::length(&mut settings.thickness).range(1.0..=200.0));
                    ui.end_row();
                });
            ui.add_space(8.0);
            cross_section(ui, &settings);
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                let ready = settings.tool.is_some();
                if ui
                    .add_enabled(ready, egui::Button::new(t!(ui, "inlay-make")))
                    .on_disabled_hover_text(t!(ui, "inlay-no-vbit"))
                    .clicked()
                {
                    answer = Some(true);
                }
                if ui.button(t!(ui, "common-cancel")).clicked() {
                    answer = Some(false);
                }
            });
        });
        if modal.should_close() && answer.is_none() {
            answer = Some(false);
        }
        match answer {
            Some(true) => {
                self.dialogs.prompt = None;
                self.make_inlay(&settings);
            }
            Some(false) => self.dialogs.prompt = None,
            None => self.dialogs.prompt = Some(Prompt::Inlay(settings)),
        }
    }
}

/// The two cuts side by side, as a cross-section to scale in depth: the base with its
/// pocket P deep, and the plug turned over in it, its face D deep and its back g above the
/// base. A sketch for the numbers, not the artwork.
fn cross_section(ui: &mut egui::Ui, s: &InlaySettings) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 130.0), egui::Sense::hover());
    let painter = ui.painter_at(rect);
    let visuals = ui.visuals();
    let base = visuals.widgets.inactive.bg_fill;
    let plug = Color32::from_rgb(0x8a, 0x5a, 0x34);
    let line = Stroke::new(1.0, visuals.text_color());
    let text = visuals.text_color();
    // Depths to points: the deepest thing drawn fills most of the height.
    let deepest = s.depth.max(s.plug_start) + s.gap + 1.0;
    let scale = (rect.height() - 40.0) / deepest as f32;
    let surface = rect.top() + 18.0 + s.gap as f32 * scale;
    let z = |mm: f64| surface + mm as f32 * scale;
    let (left, right) = (rect.left() + 8.0, rect.right() - 110.0);
    let middle = (left + right) / 2.0;
    // The pocket's walls slope at 45° in the sketch.
    let half_top = (right - left) * 0.3;
    let wall = |depth: f64| half_top - depth as f32 * scale;
    let bottom = rect.bottom() - 4.0;
    // The base, the pocket cut out of it.
    let pocket_depth = s.depth;
    let pocket_half = wall(pocket_depth).max(4.0);
    painter.rect_filled(
        egui::Rect::from_min_max(pos2(left, z(0.0)), pos2(right, bottom)),
        0.0,
        base,
    );
    let pocket = vec![
        pos2(middle - half_top, z(0.0)),
        pos2(middle + half_top, z(0.0)),
        pos2(middle + pocket_half, z(pocket_depth)),
        pos2(middle - pocket_half, z(pocket_depth)),
    ];
    painter.add(egui::Shape::convex_polygon(
        pocket,
        visuals.window_fill,
        Stroke::NONE,
    ));
    painter.add(egui::Shape::line(
        vec![
            pos2(left, z(0.0)),
            pos2(middle - half_top, z(0.0)),
            pos2(middle - pocket_half, z(pocket_depth)),
            pos2(middle + pocket_half, z(pocket_depth)),
            pos2(middle + half_top, z(0.0)),
            pos2(right, z(0.0)),
        ],
        line,
    ));
    // The plug turned over: its back g above the surface, a slab, and its taper down to
    // its face, D deep.
    let face = s.plug_start;
    let face_half = wall(face).max(4.0);
    painter.rect(
        egui::Rect::from_min_max(
            pos2(middle - half_top - 30.0, z(-s.gap)),
            pos2(middle + half_top + 30.0, z(0.0)),
        ),
        0.0,
        plug,
        line,
        egui::StrokeKind::Middle,
    );
    painter.add(egui::Shape::convex_polygon(
        vec![
            pos2(middle - half_top, z(0.0)),
            pos2(middle + half_top, z(0.0)),
            pos2(middle + face_half, z(face)),
            pos2(middle - face_half, z(face)),
        ],
        plug,
        line,
    ));
    // The numbers, at their depths on the right.
    let font = egui::FontId::proportional(11.0);
    let mark = |mm: f64, label: String| {
        let y = z(mm);
        painter.line_segment([pos2(right + 4.0, y), pos2(right + 12.0, y)], line);
        painter.text(
            pos2(right + 16.0, y),
            egui::Align2::LEFT_CENTER,
            label,
            font.clone(),
            text,
        );
    };
    let len = |mm: f64| crate::units::text(ui.ctx(), mm);
    mark(-s.gap, t!(ui, "inlay-mark-gap", gap = len(s.gap)));
    mark(0.0, t!(ui, "inlay-mark-surface"));
    mark(face, t!(ui, "inlay-mark-face", depth = len(face)));
    mark(
        pocket_depth,
        t!(ui, "inlay-mark-pocket", depth = len(pocket_depth)),
    );
}
