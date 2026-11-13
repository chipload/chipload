// SPDX-License-Identifier: GPL-3.0-or-later
//! *Turn and cut…* (docs/specs/relief-wizard.md §4): a model's face up, its quarter turns
//! on the board, its cut level, width and depth, chosen while seeing the model in 3D and the
//! relief it gives, both from its light copy. OK applies it all as one undo step.

use chipload_job::{Model, ModelUp};
use chipload_pipeline::models::{TurnView, relief_preview, top_shades, turn_view};

use super::*;
use crate::mesh_view::{self, Camera, Painted, Plate, Scene};

/// The relief preview is about this many cells along its longer side.
const PREVIEW_CELLS: usize = 200;
/// Each view's size, points.
const VIEW: egui::Vec2 = egui::vec2(340.0, 280.0);
/// The darkest shade of the relief preview, as a share of its colour.
const PREVIEW_SHADE_LOW: f32 = 0.3;
/// How much of the preview's light comes from height, the rest from slope.
const HEIGHT_SHARE: f32 = 0.5;
/// The preview's sizes are rounded to this, mm.
const SIZE_STEP_MM: f64 = 0.1;
/// The plate under the model reaches this share of its size beyond it.
const PLATE_MARGIN: f64 = 0.15;

/// The window's state: the model as it was and as it is being changed, and what is shown.
#[derive(Clone)]
pub(super) struct TurnCut {
    id: u64,
    stored: Model,
    pub(super) draft: Model,
    views: ModelViews,
}

/// The relief's picture on the board: its lower-left corner and size, mm.
#[derive(Debug, Clone, Copy)]
pub(super) struct PlacedPicture {
    pub texture: egui::TextureId,
    pub corner: (f64, f64),
    pub size: (f64, f64),
}

/// The model in 3D and the relief it gives, for a draft model: made again when the draft
/// changes. Shared by *Turn and cut…* and the Relief wizard.
#[derive(Clone, Default)]
pub(super) struct ModelViews {
    camera: Camera,
    painted: Painted,
    /// The draft the views were made for, and a number that changes with them.
    shown: Option<Model>,
    version: u64,
    view: Option<Result<TurnView, String>>,
    preview: Option<(
        egui::TextureHandle,
        [f64; 3],
        chipload_pipeline::models::TopHeights,
    )>,
}

// Two states are the same window when they change the same model the same way; what is
// shown follows from that.
impl PartialEq for TurnCut {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id && self.stored == other.stored && self.draft == other.draft
    }
}

impl std::fmt::Debug for TurnCut {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "TurnCut(model {})", self.id)
    }
}

impl TurnCut {
    fn new(id: u64, model: &Model) -> Self {
        TurnCut {
            id,
            stored: model.clone(),
            draft: model.clone(),
            views: ModelViews::default(),
        }
    }
}

impl ModelViews {
    /// The views made again when `draft` has changed.
    pub(super) fn refresh(&mut self, ctx: &egui::Context, color: egui::Color32, draft: &Model) {
        if self.shown.as_ref() == Some(draft) {
            return;
        }
        let turned = self.shown.as_ref().is_none_or(|s| {
            s.up != draft.up || s.rotation != draft.rotation || s.keep != draft.keep
        });
        if turned {
            self.view = Some(turn_view(draft).map_err(|e| e.message().english()));
            self.version += 1;
        }
        self.preview = relief_preview(draft, PREVIEW_CELLS)
            .ok()
            .flatten()
            .map(|(grid, size)| (preview_texture(ctx, &grid, color), size, grid));
        self.shown = Some(draft.clone());
    }

    /// The model could be shown: turned and cut, it has something left.
    pub(super) fn ready(&self) -> bool {
        matches!(self.view, Some(Ok(_)))
    }

    /// The relief's size on the board from the preview: width, height, depth.
    pub(super) fn size(&self) -> Option<[f64; 3]> {
        self.preview.as_ref().map(|(_, size, _)| *size)
    }

    /// The relief's picture and where it lies on the board.
    pub(super) fn placed_picture(&self) -> Option<PlacedPicture> {
        self.preview
            .as_ref()
            .map(|(texture, _, grid)| PlacedPicture {
                texture: texture.id(),
                corner: grid.origin,
                size: (
                    grid.cell * grid.columns as f64,
                    grid.cell * grid.rows as f64,
                ),
            })
    }
}

/// The relief seen from above as a picture: each cell lit by its slope and lighter the
/// higher it is, so even a shallow relief shows its shape; empty cells clear.
fn preview_texture(
    ctx: &egui::Context,
    grid: &chipload_pipeline::models::TopHeights,
    color: egui::Color32,
) -> egui::TextureHandle {
    let shades = top_shades(grid);
    let (w, h) = (grid.columns, grid.rows);
    let lowest = grid.heights.iter().flatten().copied().fold(0.0, f64::min);
    let height = |z: f64| {
        if lowest < 0.0 {
            (1.0 - z / lowest) as f32
        } else {
            1.0
        }
    };
    let mut pixels = vec![egui::Color32::TRANSPARENT; w * h];
    // Row 0 of the grid is the lowest; a picture's is the top.
    for row in 0..h {
        for column in 0..w {
            let k = row * w + column;
            if let Some(z) = grid.heights[k] {
                let lit = HEIGHT_SHARE * height(z) + (1.0 - HEIGHT_SHARE) * shades[k];
                let shade = PREVIEW_SHADE_LOW + (1.0 - PREVIEW_SHADE_LOW) * lit;
                let scaled = |c: u8| (f32::from(c) * shade) as u8;
                pixels[(h - 1 - row) * w + column] = egui::Color32::from_rgb(
                    scaled(color.r()),
                    scaled(color.g()),
                    scaled(color.b()),
                );
            }
        }
    }
    let image = egui::ColorImage::new([w, h], pixels);
    ctx.load_texture("turn-cut-preview", image, egui::TextureOptions::LINEAR)
}

impl ChiploadApp {
    /// Open *Turn and cut…* for model `id`.
    pub(super) fn open_turn_cut(&mut self, id: u64) {
        let Some(model) = self
            .edit
            .as_ref()
            .and_then(|e| e.job.drawing.models.iter().find(|m| m.id == id))
        else {
            return;
        };
        self.dialogs.prompt = Some(Prompt::TurnCut(Box::new(TurnCut::new(id, model))));
    }

    pub(super) fn turn_cut_prompt(&mut self, ctx: &egui::Context, mut state: Box<TurnCut>) {
        let dark = ctx.global_style().visuals.dark_mode;
        let color = crate::style::op_color(chipload_pipeline::new_ops::OpKind::Relief, dark);
        state.views.refresh(ctx, color, &state.draft);
        let mut answer: Option<bool> = None;
        let modal = hung_modal("turn-cut").show(ctx, |ui| {
            ui.label(crate::style::bold(
                ui,
                egui::RichText::new(t!(ui, "turn-cut-title")).heading(),
            ));
            ui.add_space(4.0);
            ui.label(t!(ui, "turn-cut-body"));
            ui.add_space(8.0);
            scrolled(ui, |ui| {
                state.views.show(ui, color);
                ui.add_space(8.0);
                egui::Grid::new("turn-cut-grid")
                    .num_columns(2)
                    .spacing([12.0, 6.0])
                    .show(ui, |ui| {
                        turn_fields(ui, &mut state.draft);
                        size_fields(ui, &mut state.draft);
                    });
            });
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                let ready = state.views.ready();
                if ui
                    .add_enabled(ready, egui::Button::new(t!(ui, "common-ok")))
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
                if state.draft != state.stored {
                    self.set_model(state.id, &state.draft);
                }
            }
            Some(false) => self.dialogs.prompt = None,
            None => self.dialogs.prompt = Some(Prompt::TurnCut(state)),
        }
    }
}

impl ModelViews {
    /// The two views side by side: the model in 3D, the relief from above, each with its
    /// caption; or why the model can't be shown.
    pub(super) fn show(&mut self, ui: &mut egui::Ui, color: egui::Color32) {
        let state = self;
        let view = match &state.view {
            Some(Ok(view)) => view,
            Some(Err(why)) => {
                ui.colored_label(ui.visuals().warn_fg_color, why);
                return;
            }
            None => return,
        };
        let grey = ui.visuals().weak_text_color().gamma_multiply(0.5);
        let plate = crate::view::Palette::of(ui.visuals().dark_mode)
            .stock
            .gamma_multiply(0.6);
        let (lo, hi) = view.bounds;
        let (mx, my) = (
            (hi[0] - lo[0]) * PLATE_MARGIN,
            (hi[1] - lo[1]) * PLATE_MARGIN,
        );
        let scene = Scene {
            layers: vec![(&view.kept, color), (&view.cut_away, grey)],
            plate: Some(Plate {
                z: view.level,
                lo: [lo[0] - mx, lo[1] - my],
                hi: [hi[0] + mx, hi[1] + my],
                color: plate,
            }),
            bounds: view.bounds,
        };
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.weak(t!(ui, "turn-cut-model"));
                mesh_view::show(
                    ui,
                    VIEW,
                    &mut state.camera,
                    &mut state.painted,
                    state.version,
                    &scene,
                )
                .on_hover_text(t!(ui, "turn-cut-model-hover"));
            });
            ui.vertical(|ui| {
                ui.weak(t!(ui, "turn-cut-relief"));
                let (rect, _) = ui.allocate_exact_size(VIEW, egui::Sense::hover());
                ui.painter().rect_filled(
                    rect,
                    4.0,
                    crate::view::Palette::of(ui.visuals().dark_mode).stock,
                );
                if let Some((texture, size, _)) = &state.preview {
                    // Fitted to the view, its proportions kept.
                    let aspect = (size[1] / size[0].max(1e-9)) as f32;
                    let fit = (rect.width() * 0.9).min(rect.height() * 0.9 / aspect.max(1e-6));
                    let shown =
                        egui::Rect::from_center_size(rect.center(), egui::vec2(fit, fit * aspect));
                    ui.painter().image(
                        texture.id(),
                        shown,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        egui::Color32::WHITE,
                    );
                    let units = ui.ctx();
                    let text = |mm: f64| {
                        crate::units::text(units, (mm / SIZE_STEP_MM).round() * SIZE_STEP_MM)
                    };
                    ui.label(t!(
                        ui,
                        "turn-cut-size",
                        width = text(size[0]).as_str(),
                        height = text(size[1]).as_str(),
                        depth = text(size[2]).as_str()
                    ));
                }
            });
        });
    }
}

/// Face up, the quarter turns and the cut level: rows of a two-column grid.
pub(super) fn turn_fields(ui: &mut egui::Ui, draft: &mut Model) {
    ui.label(t!(ui, "turn-cut-face-up"))
        .on_hover_text(t!(ui, "forms-model-up-hover"));
    ui.horizontal(|ui| {
        for up in [
            ModelUp::Z,
            ModelUp::MinusZ,
            ModelUp::Y,
            ModelUp::MinusY,
            ModelUp::X,
            ModelUp::MinusX,
        ] {
            let name = up.name().to_uppercase().replace('-', "−");
            ui.selectable_value(&mut draft.up, up, name);
        }
    });
    ui.end_row();
    ui.label(t!(ui, "turn-cut-turn"));
    ui.horizontal(|ui| {
        if ui
            .button("⟲ 90°")
            .on_hover_text(t!(ui, "turn-cut-turn-left"))
            .clicked()
        {
            draft.rotation = quarter(draft.rotation + 90.0);
        }
        if ui
            .button("⟳ 90°")
            .on_hover_text(t!(ui, "turn-cut-turn-right"))
            .clicked()
        {
            draft.rotation = quarter(draft.rotation - 90.0);
        }
        ui.weak(format!("{}°", draft.rotation));
    });
    ui.end_row();
    ui.label(t!(ui, "forms-model-keep"))
        .on_hover_text(t!(ui, "forms-model-keep-hover"));
    let mut percent = draft.keep * 100.0;
    if ui
        .add(
            egui::Slider::new(&mut percent, 1.0..=100.0)
                .max_decimals(0)
                .suffix(" %"),
        )
        .changed()
    {
        draft.keep = (percent / 100.0).clamp(0.01, 1.0);
    }
    ui.end_row();
}

/// Width and depth with *Keep proportions*: rows of a two-column grid.
pub(super) fn size_fields(ui: &mut egui::Ui, draft: &mut Model) {
    ui.label(t!(ui, "forms-width"))
        .on_hover_text(t!(ui, "forms-model-width-hover"));
    ui.add(
        crate::units::length(&mut draft.width)
            .range(0.1..=10_000.0)
            .speed(0.5),
    );
    ui.end_row();
    ui.label(t!(ui, "forms-model-depth"))
        .on_hover_text(t!(ui, "forms-model-depth-hover"));
    ui.horizontal(|ui| {
        ui.add_enabled(
            !draft.proportional,
            crate::units::length(&mut draft.depth)
                .range(0.01..=1_000.0)
                .speed(0.1),
        );
        ui.checkbox(&mut draft.proportional, t!(ui, "forms-model-proportional"))
            .on_hover_text(t!(ui, "forms-model-proportional-hover"));
    });
    ui.end_row();
}

/// An angle stepped by quarters into 0 ≤ a < 360.
fn quarter(degrees: f64) -> f64 {
    let a = degrees.rem_euclid(360.0);
    if a >= 360.0 { 0.0 } else { a }
}

#[cfg(test)]
pub(super) fn draft_of(app: &mut ChiploadApp) -> Option<&mut Model> {
    match &mut app.dialogs.prompt {
        Some(Prompt::TurnCut(state)) => Some(&mut state.draft),
        _ => None,
    }
}

/// A tall dialog hangs this far below the window's top, points.
const HUNG_TOP: f32 = 40.0;
/// Height a tall dialog needs besides its scrolled content: title, a line or two, buttons
/// and the window's margins, points.
const HUNG_CHROME: f32 = 170.0;
/// Its content scrolls in no less than this, points, however short the window.
const MIN_CONTENT: f32 = 120.0;

/// A dialog too tall to centre (the relief wizard, Turn and cut): hung from near the top,
/// it grows downward only, so its buttons stay put while its content settles.
pub(super) fn hung_modal(id: &str) -> egui::Modal {
    let id = egui::Id::new(id);
    let area =
        egui::Modal::default_area(id).anchor(egui::Align2::CENTER_TOP, egui::vec2(0.0, HUNG_TOP));
    egui::Modal::new(id).area(area)
}

/// A hung dialog's content, scrolling in a short window so the buttons below it always show
/// (found by the user: on a laptop screen Next fell off the wizard's bottom).
pub(super) fn scrolled<R>(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    let most = (ui.ctx().content_rect().height() - HUNG_TOP - HUNG_CHROME).max(MIN_CONTENT);
    egui::ScrollArea::vertical()
        .max_height(most)
        .min_scrolled_height(most)
        .auto_shrink([true, true])
        .show(ui, add)
        .inner
}
