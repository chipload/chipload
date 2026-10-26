// SPDX-License-Identifier: GPL-3.0-or-later
//! Handles on a selected picture's or text's box, or on the box round selected drawn
//! shapes: move, turn, size (docs/specs/box-handles.md, drawn-shapes-box.md §2). The box
//! maths is `chipload_pipeline::box_handles`; a drag shows the box and sets the picture,
//! text or shapes when it ends.

use chipload_core::Point;
use chipload_core::chain::ShapeFrom;
use chipload_job::Text;
use chipload_pipeline::box_handles::{
    BoxHandle, PlacedBox, box_handles, drag_box, image_box, image_in_box, text_box, text_in_box,
};

use super::*;

/// How far beyond the box's side its turn handle sits, in screen pixels.
const TURN_HANDLE_PX: f64 = 24.0;
/// How far the box round drawn shapes stands off them, in screen pixels, so its corners
/// never sit on a polyline's own point handles.
const SHAPES_BOX_PAD_PX: f64 = 10.0;

/// What a box belongs to.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum BoxOf {
    /// A picture or a text.
    Source(ShapeFrom),
    /// Drawn shapes, by id.
    Shapes(Vec<u64>),
}

/// What a box's size was worked out from, so it is worked out again only when that changes.
#[derive(Debug, Clone, PartialEq)]
enum SizeKey {
    /// A picture by id and contents: its pixels' width and height.
    Image(u64, String),
    /// A text as it is: its letters' box, mm.
    Text(Box<Text>),
}

/// The last box size worked out (decoding a picture or lettering a text every frame would
/// be slow).
#[derive(Debug, Clone, Default)]
pub(super) struct BoxSize(Option<(SizeKey, (f64, f64))>);

/// A box handle drag in progress.
#[derive(Debug, Clone)]
pub(super) struct BoxDrag {
    of: BoxOf,
    handle: BoxHandle,
    /// The box when the drag began, which every frame starts from.
    start: PlacedBox,
    /// Where the drag has it now.
    now: PlacedBox,
    /// Alt was held: no steps.
    free: bool,
    /// How far the box stands off the shapes in it, mm (0 for a picture or text).
    pad: f64,
}

impl ChiploadApp {
    /// The selected picture or text, or drawn shapes, and its box: in top view, while the
    /// project can be drawn on and nothing is being drawn, measured or placed.
    pub(super) fn selected_box(&mut self) -> Option<(BoxOf, PlacedBox)> {
        if self.three_d
            || self.sketch.is_some()
            || self.measure.is_some()
            || self.placing_text()
            || !self.can_draw()
            || !self.show_drawing()
        {
            return None;
        }
        if let CanvasSelection::Shapes(_) = self.selection {
            return self.shapes_box();
        }
        let CanvasSelection::Source(from) = self.selection else {
            return None;
        };
        let drawing = &self.edit.as_ref()?.settings.drawing;
        match from {
            ShapeFrom::Image(id) => {
                let image = drawing.images.iter().find(|i| i.id == id)?.clone();
                let key = SizeKey::Image(id, image.sha256.clone());
                let (w, h) = self.box_size.get(key, || {
                    let picture = chipload_pipeline::images::picture_bytes(&image)
                        .and_then(|b| chipload_pipeline::images::decode_picture(&b))
                        .ok()?;
                    Some((f64::from(picture.width), f64::from(picture.height)))
                })?;
                (w > 0.0).then(|| (BoxOf::Source(from), image_box(&image, h / w)))
            }
            ShapeFrom::Text(id) => {
                let text = drawing.texts.iter().find(|t| t.id == id)?.clone();
                let key = SizeKey::Text(Box::new(text.clone()));
                let size = self
                    .box_size
                    .get(key, || chipload_pipeline::texts::text_size(&text).ok())?;
                Some((BoxOf::Source(from), text_box(&text, size)))
            }
            ShapeFrom::Dxf(_) => None,
        }
    }

    /// The box round the selected drawn shapes, standing off them: all drawn, and not a
    /// single circle or rectangle, which have their own move and turn handles.
    fn shapes_box(&self) -> Option<(BoxOf, PlacedBox)> {
        let (ids, imported) = self.deletable_shapes();
        if imported || ids.is_empty() {
            return None;
        }
        if let [only] = ids[..]
            && self.shape(only).is_some_and(|s| {
                matches!(
                    s.geometry,
                    Geometry::Circle { .. } | Geometry::Rectangle { .. }
                )
            })
        {
            return None;
        }
        let job = &self.edit.as_ref()?.settings;
        let ((x0, y0), (x1, y1)) = chipload_pipeline::shapes::drawn_box(job, &ids)?;
        let pad = SHAPES_BOX_PAD_PX / self.view.scale;
        Some((
            BoxOf::Shapes(ids),
            PlacedBox {
                corner: (x0 - pad, y0 - pad),
                size: (x1 - x0 + 2.0 * pad, y1 - y0 + 2.0 * pad),
                rotation: 0.0,
            },
        ))
    }

    /// The handles to show: of the box being dragged, else of the selected one.
    pub(super) fn box_handle_points(&mut self) -> Vec<(BoxHandle, Point)> {
        let reach = TURN_HANDLE_PX / self.view.scale;
        if let Some(drag) = &self.drag.boxed {
            return box_handles(&drag.now, reach);
        }
        self.selected_box()
            .map(|(_, b)| box_handles(&b, reach))
            .unwrap_or_default()
    }

    /// The box's outline and a line out to its turn handle, dashed while its handles show.
    pub(super) fn box_outline(&mut self) -> Vec<Vec<Point>> {
        let b = match &self.drag.boxed {
            Some(drag) => drag.now,
            None => match self.selected_box() {
                Some((_, b)) => b,
                None => return Vec::new(),
            },
        };
        let c = b.corners();
        let edge = ((c[1].0 + c[2].0) / 2.0, (c[1].1 + c[2].1) / 2.0);
        let turn = box_handles(&b, TURN_HANDLE_PX / self.view.scale)
            .into_iter()
            .find(|(h, _)| *h == BoxHandle::Turn)
            .map_or(edge, |(_, p)| p);
        let mut lines = vec![vec![c[0], c[1], c[2], c[3], c[0]], vec![edge, turn]];
        // Drawn shapes follow the box while it is dragged.
        if let Some(BoxDrag {
            of: BoxOf::Shapes(ids),
            start,
            now,
            pad,
            ..
        }) = &self.drag.boxed
            && now != start
            && let Some(edit) = self.edit.as_ref()
        {
            let (from, to, scale, degrees) = box_transform(start, now, *pad);
            for shape in edit
                .settings
                .drawing
                .shapes
                .iter()
                .filter(|s| ids.contains(&s.id))
            {
                if let Some(moved) = chipload_job::edit::transformed(
                    &shape.geometry,
                    [from.0, from.1],
                    [to.0, to.1],
                    scale,
                    degrees,
                ) {
                    lines.push(outline(&moved));
                }
            }
        }
        lines
    }

    pub fn start_box_drag(&mut self, handle: BoxHandle) {
        if let Some((of, start)) = self.selected_box() {
            let pad = match of {
                BoxOf::Shapes(_) => SHAPES_BOX_PAD_PX / self.view.scale,
                BoxOf::Source(_) => 0.0,
            };
            self.drag.boxed = Some(BoxDrag {
                of,
                handle,
                start,
                now: start,
                free: false,
                pad,
            });
        }
    }

    /// The dragged box with its handle at `to`. A drag that gives no box (too small) leaves
    /// it as it last was; the status bar says why.
    pub fn drag_box_to(&mut self, to: Point, free: bool) {
        let Some(drag) = self.drag.boxed.as_mut() else {
            return;
        };
        drag.free = free;
        let dragged = match drag.handle {
            BoxHandle::Size(k) if drag.pad > 0.0 => size_shapes_box(&drag.start, drag.pad, k, to),
            handle => drag_box(&drag.start, handle, to, free),
        };
        match dragged {
            Ok(b) => drag.now = b,
            Err(why) => {
                let text = t!(self, "box-drag-not-that-size", error = self.say(&why));
                self.set_status(text, STATUS_SECONDS);
            }
        }
    }

    /// The drag ends: the picture or text is set to the box, as one undo step.
    pub fn end_box_drag(&mut self) {
        let Some(drag) = self.drag.boxed.take() else {
            return;
        };
        self.scene.snap_mark = None;
        if drag.now == drag.start {
            return;
        }
        if let BoxOf::Shapes(ids) = &drag.of {
            let (from, to, scale, degrees) = box_transform(&drag.start, &drag.now, drag.pad);
            self.transform_shapes(ids, from, to, scale, degrees);
            return;
        }
        let Some(edit) = self.edit.as_ref() else {
            return;
        };
        let drawing = &edit.settings.drawing;
        let BoxOf::Source(from) = drag.of else {
            return;
        };
        match from {
            ShapeFrom::Image(id) => {
                let Some(image) = drawing.images.iter().find(|i| i.id == id) else {
                    return;
                };
                let (x, y, width, rotation) = image_in_box(&drag.now, drag.free);
                let place = super::forms::ImagePlace {
                    width,
                    x,
                    y,
                    rotation,
                    layer: image.layer.clone(),
                };
                self.set_image_place(id, &place);
            }
            ShapeFrom::Text(id) => {
                let Some(text) = drawing.texts.iter().find(|t| t.id == id).cloned() else {
                    return;
                };
                let moved = text_in_box(&text, drag.start.size, &drag.now, drag.free);
                self.set_text(id, &moved);
            }
            ShapeFrom::Dxf(_) => {}
        }
    }

    /// Drawn shapes `ids` moved, turned and scaled (`transform_shapes`), as one undo step;
    /// they stay selected. A refusal goes to the report.
    pub fn transform_shapes(
        &mut self,
        ids: &[u64],
        from: Point,
        to: Point,
        scale: f64,
        degrees: f64,
    ) -> bool {
        let Some(edit) = self.edit.as_mut() else {
            return false;
        };
        // With the operations, so their start points and corner choices follow.
        let mut job = edit.settings.clone();
        job.operations = edit.ops.iter().map(|e| e.op.clone()).collect();
        match chipload_pipeline::shapes::transform_shapes(&job, ids, from, to, scale, degrees) {
            Ok(job) => {
                edit.settings.drawing = job.drawing;
                for (entry, op) in edit.ops.iter_mut().zip(job.operations) {
                    entry.op = op;
                }
                self.shape_draft = None;
                self.drawing_changed(ids);
                true
            }
            Err(err) => {
                self.error_report(super::report::Report::failed(
                    t!(self, "box-drag-cannot-move-shapes"),
                    err.message(),
                ));
                false
            }
        }
    }

    /// A box handle is being dragged.
    pub(super) fn dragging_box(&self) -> bool {
        self.drag.boxed.is_some()
    }
}

impl BoxSize {
    /// The size for `key`, worked out by `make` when the key is new.
    fn get(
        &mut self,
        key: SizeKey,
        make: impl FnOnce() -> Option<(f64, f64)>,
    ) -> Option<(f64, f64)> {
        if let Some((known, size)) = &self.0
            && *known == key
        {
            return Some(*size);
        }
        let size = make()?;
        self.0 = Some((key, size));
        Some(size)
    }
}

/// What turns box `start` into `now`: from its centre to the new centre, the scale, the
/// turn in degrees. The scale is the shapes' own: of the box less its `pad` on each side,
/// along its longer side (one side may be 0, for a straight line).
fn box_transform(start: &PlacedBox, now: &PlacedBox, pad: f64) -> (Point, Point, f64, f64) {
    let inner = |b: &PlacedBox| (b.size.0 - 2.0 * pad, b.size.1 - 2.0 * pad);
    let (was, is) = (inner(start), inner(now));
    let scale = if was.0 >= was.1 && was.0 > 0.0 {
        is.0 / was.0
    } else if was.1 > 0.0 {
        is.1 / was.1
    } else {
        1.0
    };
    (
        start.center(),
        now.center(),
        scale,
        now.rotation - start.rotation,
    )
}

/// The box round drawn shapes after corner `k` of `start` is dragged to `to`: the shapes'
/// own box (`start` less `pad`) scales evenly about its opposite corner, by how far `to`
/// (less the handle's stand-off) reaches along its diagonal; the pad stays as it was. Not
/// below 1 mm along the shapes' longer side.
fn size_shapes_box(start: &PlacedBox, pad: f64, k: usize, to: Point) -> Result<PlacedBox, Message> {
    if k >= 4 {
        return Err(Message::new("pipeline-box-no-corner").arg("corner", k));
    }
    let (x0, y0) = (start.corner.0 + pad, start.corner.1 + pad);
    let (w, h) = (start.size.0 - 2.0 * pad, start.size.1 - 2.0 * pad);
    let corners = [(x0, y0), (x0 + w, y0), (x0 + w, y0 + h), (x0, y0 + h)];
    let (grabbed, opposite) = (corners[k], corners[(k + 2) % 4]);
    // The handle sits `pad` out from the shapes' corner, along both axes.
    let out = (
        (grabbed.0 - opposite.0).signum() * pad,
        (grabbed.1 - opposite.1).signum() * pad,
    );
    let at = (to.0 - out.0, to.1 - out.1);
    let diagonal = (grabbed.0 - opposite.0, grabbed.1 - opposite.1);
    let length2 = diagonal.0 * diagonal.0 + diagonal.1 * diagonal.1;
    if length2 == 0.0 {
        return Err(Message::new("pipeline-box-no-size"));
    }
    let factor = ((at.0 - opposite.0) * diagonal.0 + (at.1 - opposite.1) * diagonal.1) / length2;
    let min = chipload_pipeline::box_handles::MIN_SIZE_MM;
    if factor * w.max(h) < min {
        return Err(Message::new("pipeline-box-too-small").arg(
            "min",
            chipload_i18n::Value::Length(min, chipload_i18n::Digits::Shortest),
        ));
    }
    let (nw, nh) = (w * factor, h * factor);
    let lower_left = (
        opposite.0.min(opposite.0 + diagonal.0 * factor),
        opposite.1.min(opposite.1 + diagonal.1 * factor),
    );
    Ok(PlacedBox {
        corner: (lower_left.0 - pad, lower_left.1 - pad),
        size: (nw + 2.0 * pad, nh + 2.0 * pad),
        rotation: 0.0,
    })
}

/// A drawn shape's outline as points, its arcs in short straight pieces, for the preview.
fn outline(geometry: &Geometry) -> Vec<Point> {
    let shape = chipload_job::Shape {
        id: 0,
        layer: String::new(),
        name: None,
        geometry: geometry.clone(),
    };
    let mut points = Vec::new();
    for segment in shape.segments() {
        let prim = segment.to_primitive();
        match prim {
            chipload_core::primitives::Primitive::Line(line) => {
                points.push(line.start);
                points.push(line.end);
            }
            chipload_core::primitives::Primitive::Arc(arc) => {
                let steps = ((arc.sweep.abs() / std::f64::consts::TAU) * 64.0)
                    .ceil()
                    .max(2.0);
                for k in 0..=steps as usize {
                    let t = arc.start_angle + arc.sweep * k as f64 / steps;
                    points.push(arc.point_at(t));
                }
            }
        }
    }
    points
}
