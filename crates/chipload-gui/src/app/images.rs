// SPDX-License-Identifier: GPL-3.0-or-later
//! File > Import image… and changing an image (docs/specs/image-trace.md §6): a window with
//! the picture as black and white (its centre lines over it), and the settings. The
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
type PictureKey = (u64, String, f64, f64, f64, f64);

/// The textures of the project's pictures, kept until its images change.
#[derive(Default)]
pub struct PictureTextures {
    keys: Vec<PictureKey>,
    layers: Vec<(crate::view::PictureLayer, egui::TextureHandle)>,
}

/// The longest side of the preview's texture, px: larger pictures are shown scaled down.
const MAX_TEXTURE_PX: usize = 2048;
/// The preview's largest zoom: a picture pixel then spans many screen pixels.
const MAX_ZOOM: f32 = 32.0;
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
    zoom: PreviewZoom,
}

/// What part of the picture the preview shows: `zoom` 1 is all of it, centred on `centre`
/// in texture coordinates (0..1, y down).
#[derive(Clone, Copy, Debug, PartialEq)]
struct PreviewZoom {
    zoom: f32,
    centre: Pos2,
}

impl Default for PreviewZoom {
    fn default() -> Self {
        PreviewZoom {
            zoom: 1.0,
            centre: Pos2::new(0.5, 0.5),
        }
    }
}

impl PreviewZoom {
    /// The part of the texture shown, inside 0..1.
    fn uv(&self) -> Rect {
        let half = 0.5 / self.zoom;
        Rect::from_center_size(self.centre, Vec2::splat(2.0 * half))
    }

    /// Zoom by `factor` keeping the texture point `at` where it is on screen.
    fn zoom_at(&mut self, at: Pos2, factor: f32) {
        let zoom = (self.zoom * factor).clamp(1.0, MAX_ZOOM);
        let kept = zoom / self.zoom;
        self.zoom = zoom;
        self.centre = at + (self.centre - at) / kept;
        self.keep_inside();
    }

    /// Move the view by `delta` in texture coordinates.
    fn pan(&mut self, delta: Vec2) {
        self.centre -= delta;
        self.keep_inside();
    }

    /// The shown part stays on the picture.
    fn keep_inside(&mut self) {
        let half = 0.5 / self.zoom;
        self.centre.x = self.centre.x.clamp(half, 1.0 - half);
        self.centre.y = self.centre.y.clamp(half, 1.0 - half);
    }
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
            zoom: PreviewZoom::default(),
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

/// How the black parts are shown: near black, so the picture reads as it will be traced.
const FIELD_GREY: u8 = 40;

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
            gray.push(if black { FIELD_GREY } else { 255 });
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
            rotation: 0.0,
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
                // The window changes the conversion settings only: the size, place and
                // layer are the project's, set in the picture's properties
                // (docs/specs/properties-for-tools-and-dxfs.md §3b).
                if let Some(image) = job.drawing.images.iter_mut().find(|i| i.id == *id) {
                    *image = Image {
                        data: image.data.clone(),
                        sha256: image.sha256.clone(),
                        layer: image.layer.clone(),
                        x: image.x,
                        y: image.y,
                        width: image.width,
                        rotation: image.rotation,
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
                // Its size and place are in its properties, right after importing.
                self.focus = super::Focus::Image(report.id);
                self.commit();
                let mut lines = report.lines.clone();
                lines.push(
                    "The picture is selected: V-carve (ribbon) carves it. Set its size and \
                     place in Properties."
                        .to_string(),
                );
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

    /// A picture's new size, place and layer from its properties, traced again, as one
    /// undo step (docs/specs/properties-for-tools-and-dxfs.md §3b).
    pub(super) fn set_image_place(&mut self, id: u64, place: &super::forms::ImagePlace) -> bool {
        let Some(edit) = self.edit.as_ref() else {
            return false;
        };
        let mut job = edit.settings.clone();
        let Some(image) = job.drawing.images.iter_mut().find(|i| i.id == id) else {
            return false;
        };
        image.width = place.width;
        (image.x, image.y) = (place.x, place.y);
        image.rotation = place.rotation;
        image.layer.clone_from(&place.layer);
        match retrace(&job, id) {
            Ok((job, report)) => {
                if let Some(edit) = self.edit.as_mut() {
                    edit.settings = job;
                }
                self.drawing_changed(&[]);
                self.selection = super::CanvasSelection::Source(
                    chipload_core::chain::ShapeFrom::Image(report.id),
                );
                self.commit();
                self.set_report(Report::info(report.lines));
                true
            }
            Err(err) => {
                self.error(vec![
                    "Could not trace the picture at that size:".into(),
                    err.to_string(),
                ]);
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

    /// A selection of shapes all traced from one image, as that image minus the contours
    /// not selected: an operation selects the image, never its shapes, whose ids change each
    /// time it is traced again (docs/specs/image-trace.md §4, leave-out-shapes.md §5).
    pub(super) fn traced_selection(&self) -> Option<super::CanvasSelection> {
        let edit = self.edit.as_ref()?;
        let super::CanvasSelection::Shapes(ids) = &self.selection else {
            return None;
        };
        let mut images = std::collections::BTreeSet::new();
        for id in ids {
            let shape = edit.settings.drawing.shapes.iter().find(|s| s.id == *id)?;
            images.insert(shape.image()?);
        }
        let image = match images.len() {
            1 => images.into_iter().next()?,
            _ => return None,
        };
        let from = chipload_core::chain::ShapeFrom::Image(image);
        let selected = self.selection.ids(&self.contours);
        let left_out: std::collections::BTreeSet<u64> = self
            .contours
            .iter()
            .filter(|c| c.parts.iter().all(|p| p.segment.from == Some(from)))
            .filter(|c| !selected.contains(&c.id))
            .flat_map(super::shapes_of)
            .collect();
        Some(if left_out.is_empty() {
            super::CanvasSelection::Source(from)
        } else {
            super::CanvasSelection::Except(from, left_out)
        })
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
            .map(|i| (i.id, i.sha256.clone(), i.x, i.y, i.width, i.rotation))
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
                // Turned about its centre, as its traced shapes are.
                let (cx, cy) = (image.x + image.width / 2.0, image.y + height / 2.0);
                let (hw, hh) = (image.width / 2.0, height / 2.0);
                let corners = [[-hw, -hh], [hw, -hh], [hw, hh], [-hw, hh]].map(|p| {
                    let q = chipload_job::rotated(p, image.rotation);
                    (cx + q[0], cy + q[1])
                });
                let layer = crate::view::PictureLayer {
                    texture: texture.id(),
                    corners,
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
            ImageTarget::Existing(_) => format!("Trace {}", dialog.draft.file),
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
                        let picked = match &dialog.preview {
                            Some(Ok(p)) => Some(p.trace.threshold),
                            _ => None,
                        };
                        settings_form(ui, &mut dialog.draft, picked);
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

/// The black-and-white picture. Outlines are not drawn: they follow the black parts and
/// would hide the detail the threshold is set by. Centre lines are, as they are the result.
/// Scrolling zooms at the pointer, dragging pans, a double click shows all of it.
fn picture_view(ui: &mut egui::Ui, dialog: &mut ImageDialog) {
    let (w, h) = picture_size(dialog);
    let aspect = f64::from(h) / f64::from(w);
    let scale = (PREVIEW_SIZE.x / w as f32).min(PREVIEW_SIZE.y / h as f32);
    let size = Vec2::new(w as f32 * scale, h as f32 * scale);
    ui.vertical(|ui| {
        let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click_and_drag());
        zoom_input(ui, &response, rect, &mut dialog.zoom);
        let uv = dialog.zoom.uv();
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0.0, Color32::from_gray(128));
        if let Some(texture) = &dialog.texture {
            painter.image(texture.id(), rect, uv, Color32::WHITE);
        }
        if let Some(Ok(preview)) = &dialog.preview
            && dialog.shown.as_ref().map(|image| image.trace) == Some(ImageTrace::CentreLine)
        {
            // Lines are in mm from the picture's lower-left corner.
            let width = preview.trace.width;
            let height = width * aspect;
            let to_screen = |(x, y): (f64, f64)| {
                let u = (x / width) as f32;
                let v = 1.0 - (y / height) as f32;
                Pos2::new(
                    rect.left() + (u - uv.left()) / uv.width() * rect.width(),
                    rect.top() + (v - uv.top()) / uv.height() * rect.height(),
                )
            };
            let stroke = Stroke::new(1.0, Color32::from_rgb(230, 110, 20));
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
        let note = if dialog.zoom.zoom > 1.0 {
            format!(
                "Zoom {:.0}% · drag to pan · double-click to fit",
                dialog.zoom.zoom * 100.0
            )
        } else {
            "Scroll to zoom in".to_owned()
        };
        ui.weak(note);
    });
}

/// Scroll (or pinch) zooms at the pointer, a drag pans, a double click fits.
fn zoom_input(ui: &egui::Ui, response: &egui::Response, rect: Rect, zoom: &mut PreviewZoom) {
    let to_uv = |pos: Pos2, zoom: &PreviewZoom| {
        let uv = zoom.uv();
        Pos2::new(
            uv.left() + (pos.x - rect.left()) / rect.width() * uv.width(),
            uv.top() + (pos.y - rect.top()) / rect.height() * uv.height(),
        )
    };
    if response.double_clicked() {
        *zoom = PreviewZoom::default();
        return;
    }
    if response.dragged() {
        let uv = zoom.uv();
        let delta = response.drag_delta();
        zoom.pan(Vec2::new(
            delta.x / rect.width() * uv.width(),
            delta.y / rect.height() * uv.height(),
        ));
    }
    if let Some(pos) = response.hover_pos() {
        let (scroll, pinch) = ui.input(|i| (i.smooth_scroll_delta.y, i.zoom_delta()));
        let factor = pinch * (scroll * 0.0015).exp();
        if factor != 1.0 {
            let at = to_uv(pos, zoom);
            zoom.zoom_at(at, factor);
        }
    }
}

/// The settings of spec §2, the size and the place. `picked`: the threshold the last
/// preview used, which the slider shows (and starts from) while *auto* is on.
fn settings_form(ui: &mut egui::Ui, draft: &mut Image, picked: Option<u8>) {
    egui::Grid::new("image-settings")
        .num_columns(2)
        .show(ui, |ui| {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zooming_keeps_the_point_under_the_pointer() {
        let mut zoom = PreviewZoom::default();
        let at = Pos2::new(0.25, 0.75);
        // Where `at` is within the shown part, before and after.
        let place = |z: &PreviewZoom| {
            let uv = z.uv();
            (
                (at.x - uv.left()) / uv.width(),
                (at.y - uv.top()) / uv.height(),
            )
        };
        let before = place(&zoom);
        zoom.zoom_at(at, 4.0);
        let after = place(&zoom);
        assert_eq!(zoom.zoom, 4.0);
        assert!((before.0 - after.0).abs() < 1e-5 && (before.1 - after.1).abs() < 1e-5);
    }

    #[test]
    fn the_view_stays_on_the_picture() {
        let mut zoom = PreviewZoom::default();
        zoom.zoom_at(Pos2::new(0.0, 0.0), 2.0);
        zoom.pan(Vec2::new(5.0, 5.0));
        let uv = zoom.uv();
        assert!(uv.left() >= 0.0 && uv.top() >= 0.0 && uv.right() <= 1.0 && uv.bottom() <= 1.0);
        zoom.zoom_at(Pos2::new(0.5, 0.5), 1000.0);
        assert_eq!(zoom.zoom, MAX_ZOOM);
        zoom.zoom_at(Pos2::new(0.5, 0.5), 0.0001);
        assert_eq!(
            zoom.uv(),
            Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0))
        );
    }
}
