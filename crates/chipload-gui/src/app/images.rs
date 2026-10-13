// SPDX-License-Identifier: GPL-3.0-or-later
//! File > Import image… and changing an image (docs/specs/image-trace.md §6): a window with
//! the picture as black and white, its traced outlines over it, and the settings. The
//! preview is traced in the background as the settings change; *Import* (or *Apply*) puts
//! the shapes into the project through the pipeline.

use std::sync::{Arc, mpsc};

use chipload_job::{Image, ImageThreshold, ImageTrace};
use chipload_pipeline::images::{
    Decoded, NewImage, Preview, add_image, decode_picture, new_layer, picture_bytes, preview,
    retrace,
};
use eframe::egui::{self, Color32, ColorImage, Pos2, Rect, Stroke, TextureOptions, Vec2};

use super::{ChiploadApp, STATUS_SECONDS};
use crate::app::report::Report;

/// What a picture's texture was made from: the image's id, picture, place and width.
type PictureKey = (u64, String, f64, f64, f64);

/// The textures of the project's pictures, kept until its images change.
#[derive(Default)]
pub struct PictureTextures {
    keys: Vec<PictureKey>,
    layers: Vec<(crate::view::PictureLayer, egui::TextureHandle)>,
}

/// The longest side of the preview's texture, px: larger pictures are shown scaled down.
const MAX_TEXTURE_PX: usize = 1024;
/// The settings column's width.
const SETTINGS_WIDTH: f32 = 330.0;
/// The preview's largest size on screen.
const PREVIEW_SIZE: Vec2 = Vec2::new(520.0, 380.0);

/// The picture an open image window is about.
#[derive(Clone)]
pub enum ImageTarget {
    /// A picture being imported: its file name and bytes.
    New { file: String, bytes: Arc<Vec<u8>> },
    /// An image already in the project.
    Existing(u64),
}

pub struct ImageDialog {
    pub target: ImageTarget,
    /// The settings as being edited (`data` is unused here).
    pub draft: Image,
    picture: Arc<Decoded>,
    /// The last preview, for the settings in `shown`.
    pub preview: Option<Result<Preview, String>>,
    shown: Option<Image>,
    texture: Option<egui::TextureHandle>,
    /// The preview being traced, for the settings in `asked`.
    pending: Option<mpsc::Receiver<Result<Preview, String>>>,
    asked: Option<Image>,
}

impl ImageDialog {
    fn new(target: ImageTarget, draft: Image, picture: Decoded) -> Self {
        ImageDialog {
            target,
            draft,
            picture: Arc::new(picture),
            preview: None,
            shown: None,
            texture: None,
            pending: None,
            asked: None,
        }
    }

    /// Start tracing the draft if it changed and nothing is being traced; take a finished
    /// preview. True while one is being traced.
    fn step(&mut self, ctx: &egui::Context) -> bool {
        if let Some(rx) = &self.pending {
            match rx.try_recv() {
                Ok(result) => {
                    self.texture = result
                        .as_ref()
                        .ok()
                        .map(|p| ctx.load_texture("image-preview", texture_of(p), nearest()));
                    self.preview = Some(result);
                    self.shown = self.asked.take();
                    self.pending = None;
                }
                Err(mpsc::TryRecvError::Empty) => return true,
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.preview = Some(Err("tracing the picture failed".into()));
                    self.shown = self.asked.take();
                    self.pending = None;
                }
            }
        }
        if self.shown.as_ref() != Some(&self.draft) {
            let (tx, rx) = mpsc::channel();
            let (picture, draft) = (Arc::clone(&self.picture), self.draft.clone());
            std::thread::spawn(move || {
                let result = preview(&picture, &draft).map_err(|e| e.to_string());
                let _ = tx.send(result);
            });
            self.pending = Some(rx);
            self.asked = Some(self.draft.clone());
            return true;
        }
        false
    }

    /// The preview is for the settings as they are now, and traced without an error.
    fn ready(&self) -> bool {
        self.pending.is_none()
            && self.shown.as_ref() == Some(&self.draft)
            && matches!(self.preview, Some(Ok(_)))
    }
}

/// The black-and-white picture as a texture, scaled down to `MAX_TEXTURE_PX`.
fn texture_of(p: &Preview) -> ColorImage {
    let bw = &p.black_white;
    let (w, h) = (bw.width as usize, bw.height as usize);
    let step = w.max(h).div_ceil(MAX_TEXTURE_PX).max(1);
    let (tw, th) = (w.div_ceil(step), h.div_ceil(step));
    let mut gray = Vec::with_capacity(tw * th);
    for row in 0..th {
        for column in 0..tw {
            let black = bw.black[(row * step).min(h - 1) * w + (column * step).min(w - 1)];
            gray.push(if black { 0 } else { 255 });
        }
    }
    ColorImage::from_gray([tw, th], &gray)
}

/// The picture in grey, scaled down to `MAX_TEXTURE_PX`.
fn grey_texture(picture: &Decoded) -> ColorImage {
    let (w, h) = (picture.width as usize, picture.height as usize);
    let step = w.max(h).div_ceil(MAX_TEXTURE_PX).max(1);
    let (tw, th) = (w.div_ceil(step), h.div_ceil(step));
    let mut gray = Vec::with_capacity(tw * th);
    for row in 0..th {
        for column in 0..tw {
            let v = picture.grey[(row * step).min(h - 1) * w + (column * step).min(w - 1)];
            gray.push(v.round().clamp(0.0, 255.0) as u8);
        }
    }
    ColorImage::from_gray([tw, th], &gray)
}

fn nearest() -> TextureOptions {
    TextureOptions::NEAREST
}

impl ChiploadApp {
    /// File > Import image…: pick a picture and open the image window for it.
    pub(super) fn import_image_dialog(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Pictures", &["png", "jpg", "jpeg", "bmp"])
            .pick_file()
        {
            self.import_image(&path);
        }
    }

    /// Open the image window for the picture at `path`, to import it into the project
    /// (a new one if none is open).
    pub fn import_image(&mut self, path: &std::path::Path) -> bool {
        let bytes = match std::fs::read(path) {
            Ok(bytes) => bytes,
            Err(err) => {
                self.error(vec![format!("Cannot read {}: {err}", path.display())]);
                return false;
            }
        };
        let picture = match decode_picture(&bytes) {
            Ok(picture) => picture,
            Err(err) => {
                self.error(vec![
                    format!("Cannot import {}:", path.display()),
                    err.to_string(),
                ]);
                return false;
            }
        };
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
        // Half the stock's width to start with, shown and editable: never a guess from the
        // picture's DPI (spec §3.5).
        let width = (edit.settings.stock.x / 2.0).round().max(1.0);
        let defaults = NewImage::new(&file, &bytes, width);
        let draft = Image {
            id: 0,
            file: file.clone(),
            sha256: String::new(),
            data: String::new(),
            layer: new_layer(&edit.settings, &file),
            x: 0.0,
            y: 0.0,
            width,
            threshold: defaults.threshold,
            invert: defaults.invert,
            blur: defaults.blur,
            min_area: defaults.min_area,
            smoothing: defaults.smoothing,
            trace: defaults.trace,
            arcs: defaults.arcs,
        };
        let target = ImageTarget::New {
            file,
            bytes: Arc::new(bytes),
        };
        self.image_dialog = Some(ImageDialog::new(target, draft, picture));
        true
    }

    /// Open the image window for image `id` of the project.
    pub fn edit_image(&mut self, id: u64) -> bool {
        let Some(image) = self
            .edit
            .as_ref()
            .and_then(|e| e.settings.drawing.images.iter().find(|i| i.id == id))
            .cloned()
        else {
            return false;
        };
        let picture = match picture_bytes(&image).and_then(|b| decode_picture(&b)) {
            Ok(picture) => picture,
            Err(err) => {
                self.error(vec![err.to_string()]);
                return false;
            }
        };
        self.image_dialog = Some(ImageDialog::new(ImageTarget::Existing(id), image, picture));
        true
    }

    /// Put the window's image into the project: import it, or trace it again with its new
    /// settings. One undo step; its layer is selected, ready for + V-carve.
    pub fn apply_image(&mut self) -> bool {
        let Some(dialog) = self.image_dialog.as_ref() else {
            return false;
        };
        let Some(edit) = self.edit.as_ref() else {
            return false;
        };
        let draft = dialog.draft.clone();
        let result = match &dialog.target {
            ImageTarget::New { file, bytes } => {
                let mut new = NewImage::new(file, bytes, draft.width);
                new.layer = Some(draft.layer.clone());
                (new.x, new.y) = (draft.x, draft.y);
                new.threshold = draft.threshold;
                new.invert = draft.invert;
                new.blur = draft.blur;
                new.min_area = draft.min_area;
                new.smoothing = draft.smoothing;
                new.trace = draft.trace;
                new.arcs = draft.arcs;
                add_image(&edit.settings, new)
            }
            ImageTarget::Existing(id) => {
                let mut job = edit.settings.clone();
                if let Some(image) = job.drawing.images.iter_mut().find(|i| i.id == *id) {
                    let (data, sha256) = (image.data.clone(), image.sha256.clone());
                    *image = Image {
                        data,
                        sha256,
                        ..draft.clone()
                    };
                }
                retrace(&job, *id)
            }
        };
        match result {
            Ok((job, report)) => {
                if let Some(edit) = self.edit.as_mut() {
                    edit.settings = job;
                }
                self.image_dialog = None;
                self.drawing_changed(&[]);
                self.selection = super::CanvasSelection::Source(
                    chipload_core::chain::ShapeFrom::Image(report.id),
                );
                self.commit();
                let mut lines = report.lines.clone();
                lines.push(format!(
                    "Layer {} is selected: + V-carve carves it.",
                    draft.layer
                ));
                self.set_report(Report::info(lines));
                self.set_status(format!("Traced {}", draft.file), STATUS_SECONDS);
                true
            }
            Err(err) => {
                if let Some(dialog) = self.image_dialog.as_mut() {
                    dialog.preview = Some(Err(err.to_string()));
                }
                false
            }
        }
    }

    /// Wait until the image window's preview is traced (screenshots and tests).
    #[cfg(any(test, feature = "screenshots"))]
    pub fn wait_for_image_preview(&mut self, ctx: &egui::Context) {
        while let Some(dialog) = self.image_dialog.as_mut() {
            if !dialog.step(ctx) {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    }

    /// Remove image `id` and its shapes, as one undo step.
    pub fn remove_image(&mut self, id: u64) -> bool {
        let Some(edit) = self.edit.as_mut() else {
            return false;
        };
        match chipload_pipeline::images::remove_image(&edit.settings, id) {
            Ok(job) => {
                edit.settings = job;
                self.drawing_changed(&[]);
                self.selection = super::CanvasSelection::Nothing;
                self.commit();
                self.set_status(format!("Removed image {id}"), STATUS_SECONDS);
                true
            }
            Err(err) => {
                self.error(vec![err.to_string()]);
                false
            }
        }
    }

    /// The image every selected shape was traced from, if they all come from one: an
    /// operation selects the image, never its shapes, whose ids change each time it is
    /// traced again (docs/specs/image-trace.md §4).
    pub(super) fn traced_image(&self) -> Option<chipload_core::chain::ShapeFrom> {
        let edit = self.edit.as_ref()?;
        let super::CanvasSelection::Shapes(ids) = &self.selection else {
            return None;
        };
        let mut images = std::collections::BTreeSet::new();
        for id in ids {
            let shape = edit.settings.drawing.shapes.iter().find(|s| s.id == *id)?;
            images.insert(shape.image()?);
        }
        match images.len() {
            1 => images
                .into_iter()
                .next()
                .map(chipload_core::chain::ShapeFrom::Image),
            _ => None,
        }
    }

    /// Keep the canvas's pictures in step with the project's images (View > Show images).
    pub(super) fn sync_pictures(&mut self, ctx: &egui::Context) {
        let images: Vec<Image> = self
            .edit
            .as_ref()
            .map(|e| e.settings.drawing.images.clone())
            .unwrap_or_default();
        let keys: Vec<PictureKey> = images
            .iter()
            .map(|i| (i.id, i.sha256.clone(), i.x, i.y, i.width))
            .collect();
        if keys != self.picture_textures.keys {
            let mut layers = Vec::new();
            for image in &images {
                let Ok(picture) = picture_bytes(image).and_then(|b| decode_picture(&b)) else {
                    continue;
                };
                let height = image.width * f64::from(picture.height) / f64::from(picture.width);
                let texture = ctx.load_texture(
                    format!("picture-{}", image.id),
                    grey_texture(&picture),
                    TextureOptions::LINEAR,
                );
                let layer = crate::view::PictureLayer {
                    texture: texture.id(),
                    min: (image.x, image.y),
                    max: (image.x + image.width, image.y + height),
                };
                layers.push((layer, texture));
            }
            self.picture_textures = PictureTextures { keys, layers };
        }
        self.scene.pictures = if self.show_images {
            self.picture_textures
                .layers
                .iter()
                .map(|(layer, _)| layer.clone())
                .collect()
        } else {
            Vec::new()
        };
    }

    /// The image window, when open.
    pub(super) fn image_window(&mut self, ctx: &egui::Context) {
        let Some(dialog) = self.image_dialog.as_mut() else {
            return;
        };
        if dialog.step(ctx) {
            ctx.request_repaint_after(std::time::Duration::from_millis(30));
        }
        let title = match &dialog.target {
            ImageTarget::New { file, .. } => format!("Import image: {file}"),
            ImageTarget::Existing(id) => format!("Image {id}: {}", dialog.draft.file),
        };
        let mut open = true;
        let mut apply = false;
        egui::Window::new(title)
            .id(egui::Id::new("image-window"))
            .open(&mut open)
            .resizable(false)
            .fade_in(false)
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(ctx.content_rect().center())
            .show(ctx, |ui| {
                ui.horizontal_top(|ui| {
                    picture_view(ui, dialog);
                    ui.vertical(|ui| {
                        ui.set_min_width(SETTINGS_WIDTH);
                        let size = picture_size(dialog);
                        let picked = match &dialog.preview {
                            Some(Ok(p)) => Some(p.trace.threshold),
                            _ => None,
                        };
                        settings_form(ui, &mut dialog.draft, size, picked);
                        ui.separator();
                        summary(ui, dialog);
                        ui.add_space(6.0);
                        let label = match dialog.target {
                            ImageTarget::New { .. } => "Import",
                            ImageTarget::Existing(_) => "Apply",
                        };
                        apply = ui
                            .add_enabled(dialog.ready(), egui::Button::new(label))
                            .clicked();
                    });
                });
            });
        if apply {
            self.apply_image();
        } else if !open {
            self.image_dialog = None;
        }
    }
}

/// The picture's size in pixels.
fn picture_size(dialog: &ImageDialog) -> (u32, u32) {
    (dialog.picture.width, dialog.picture.height)
}

/// The black-and-white picture with the traced outlines over it.
fn picture_view(ui: &mut egui::Ui, dialog: &ImageDialog) {
    let (w, h) = picture_size(dialog);
    let aspect = f64::from(h) / f64::from(w);
    let scale = (PREVIEW_SIZE.x / w as f32).min(PREVIEW_SIZE.y / h as f32);
    let size = Vec2::new(w as f32 * scale, h as f32 * scale);
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 0.0, Color32::from_gray(128));
    if let Some(texture) = &dialog.texture {
        let uv = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
        painter.image(texture.id(), rect, uv, Color32::WHITE);
    }
    let Some(Ok(preview)) = &dialog.preview else {
        return;
    };
    // Loops are in mm from the picture's lower-left corner.
    let width = preview.trace.width;
    let height = width * aspect;
    let to_screen = |(x, y): (f64, f64)| {
        Pos2::new(
            rect.left() + (x / width) as f32 * rect.width(),
            rect.bottom() - (y / height) as f32 * rect.height(),
        )
    };
    let stroke = Stroke::new(1.5, Color32::from_rgb(230, 110, 20));
    for path in &preview.trace.paths {
        let mut points: Vec<Pos2> = path.points.iter().map(|&p| to_screen(p)).collect();
        if path.closed
            && let Some(&first) = points.first()
        {
            points.push(first);
        }
        painter.add(egui::Shape::line(points, stroke));
    }
}

/// The settings of spec §2, the size and the place. `picked`: the threshold the last
/// preview used, which the slider shows (and starts from) while *auto* is on.
fn settings_form(ui: &mut egui::Ui, draft: &mut Image, (w, h): (u32, u32), picked: Option<u8>) {
    egui::Grid::new("image-settings")
        .num_columns(2)
        .show(ui, |ui| {
            ui.label("Width");
            ui.add(
                egui::DragValue::new(&mut draft.width)
                    .range(0.1..=10_000.0)
                    .speed(0.5)
                    .suffix(" mm"),
            );
            ui.end_row();
            ui.label("Height");
            ui.label(format!(
                "{:.1} mm",
                draft.width * f64::from(h) / f64::from(w)
            ));
            ui.end_row();
            ui.label("Lower-left corner");
            ui.horizontal(|ui| {
                ui.add(
                    egui::DragValue::new(&mut draft.x)
                        .speed(0.5)
                        .prefix("X ")
                        .suffix(" mm"),
                );
                ui.add(
                    egui::DragValue::new(&mut draft.y)
                        .speed(0.5)
                        .prefix("Y ")
                        .suffix(" mm"),
                );
            });
            ui.end_row();
            ui.label("Layer");
            ui.text_edit_singleline(&mut draft.layer);
            ui.end_row();
            ui.label("Threshold").on_hover_text(
                "Grey values below it are black (0 black, 255 white). Auto picks the value that \
                 best separates the dark and light parts.",
            );
            ui.horizontal(|ui| {
                let mut auto = matches!(draft.threshold, ImageThreshold::Auto(_));
                let mut level = match draft.threshold {
                    ImageThreshold::Level(t) => t,
                    ImageThreshold::Auto(_) => picked.unwrap_or(128),
                };
                ui.checkbox(&mut auto, "auto");
                ui.add_enabled(!auto, egui::Slider::new(&mut level, 1..=254));
                draft.threshold = if auto {
                    ImageThreshold::AUTO
                } else {
                    ImageThreshold::Level(level)
                };
            });
            ui.end_row();
            ui.label("Invert");
            ui.checkbox(&mut draft.invert, "carve the light parts");
            ui.end_row();
            ui.label("Blur")
                .on_hover_text("Smooths ragged edges and noise before the threshold.");
            ui.add(egui::Slider::new(&mut draft.blur, 0.0..=10.0).suffix(" px"));
            ui.end_row();
            ui.label("Smallest piece")
                .on_hover_text("Black or white specks smaller than this are dropped.");
            ui.add(
                egui::DragValue::new(&mut draft.min_area)
                    .range(0.0..=100.0)
                    .speed(0.05)
                    .suffix(" mm²"),
            );
            ui.end_row();
            ui.label("Smoothing").on_hover_text(
                "How far a traced edge may be straightened away from the picture's edge.",
            );
            ui.add(
                egui::DragValue::new(&mut draft.smoothing)
                    .range(0.01..=1.0)
                    .speed(0.005)
                    .suffix(" mm"),
            );
            ui.end_row();
            ui.label("Trace").on_hover_text(
                "Outlines round the black parts, for a V-carve; or the centre lines of black \
                 strokes (handwriting, line art), for a follow path.",
            );
            egui::ComboBox::from_id_salt("image-trace")
                .selected_text(trace_name(draft.trace))
                .show_ui(ui, |ui| {
                    for trace in [ImageTrace::Outline, ImageTrace::CentreLine] {
                        ui.selectable_value(&mut draft.trace, trace, trace_name(trace));
                    }
                });
            ui.end_row();
            ui.label("Arcs").on_hover_text(
                "Fit arcs as well as lines: smaller programs and smoother motion for profiles, \
                 pockets and follow paths.",
            );
            ui.checkbox(&mut draft.arcs, "fit arcs");
            ui.end_row();
        });
}

fn trace_name(trace: ImageTrace) -> &'static str {
    match trace {
        ImageTrace::Outline => "Outlines (V-carve)",
        ImageTrace::CentreLine => "Centre lines (follow path)",
    }
}

/// What the preview traced, or why it couldn't.
fn summary(ui: &mut egui::Ui, dialog: &ImageDialog) {
    match &dialog.preview {
        None => {
            ui.label("Tracing…");
        }
        Some(Err(err)) => {
            ui.colored_label(ui.visuals().error_fg_color, err);
        }
        Some(Ok(p)) => {
            let t = &p.trace;
            let picked = if matches!(dialog.draft.threshold, ImageThreshold::Auto(_)) {
                format!(" (threshold {} picked)", t.threshold)
            } else {
                String::new()
            };
            let count = t.paths.len();
            let what = match (t.centre_lines, count == 1) {
                (false, true) => "shape",
                (false, false) => "shapes",
                (true, true) => "centre line",
                (true, false) => "centre lines",
            };
            ui.label(format!("{count} {what}{picked}"));
            if let Some((narrow, wide)) = t.widths {
                ui.label(format!("Strokes {narrow:.2} to {wide:.2} mm wide"));
            }
            if t.dots > 0 {
                ui.label(format!("{} dots left out", t.dots));
            }
            if t.without_arcs > 0 {
                ui.label(format!("{} kept as lines", t.without_arcs));
            }
            if t.dropped > 0 {
                ui.label(format!("{} specks dropped", t.dropped));
            }
            if t.black_at_border {
                ui.label("Black touches the edge: invert if it's the background.");
            }
        }
    }
}
