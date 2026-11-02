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
pub(super) const SHAPES_BOX_PAD_PX: f64 = 10.0;

/// What a box belongs to.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum BoxOf {
    /// A picture, a text, or a group with its texts (docs/specs/shape-groups.md §2).
    Whole(Source),
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
    /// A picture or text dragged by its own lines rather than the Move handle: the box's
    /// centre from the point grabbed, mm.
    offset: Point,
    /// Carried onto the shape library: saved there, not moved.
    pub carried: bool,
}

impl ChiploadApp {
    /// The selected picture or text, or drawn shapes, and its box: in top view, while the
    /// project can be drawn on and nothing is being drawn, measured or placed.
    pub(super) fn selected_box(&mut self) -> Option<(BoxOf, PlacedBox)> {
        if self.display.three_d
            || self.sketch().is_some()
            || self.measure().is_some()
            || self.placing_text()
            || !self.can_draw()
            || !self.show_drawing()
        {
            return None;
        }
        if let CanvasSelection::Shapes(_) = self.selection {
            return self.shapes_box();
        }
        if let Some(id) = self.selected_group() {
            // Turned as the group was (docs/specs/dxf-whole-on-canvas.md §6).
            let drawing = &self.edit.as_ref()?.job.drawing;
            let rotation = drawing.groups.iter().find(|g| g.id == id)?.rotation;
            let ids: BTreeSet<u64> = drawing.group_shapes(id).into_iter().collect();
            let pad = SHAPES_BOX_PAD_PX / self.view.scale;
            let placed = self.turned_box_of(
                |_, shape| shape.is_some_and(|s| ids.contains(&s)),
                rotation,
                pad,
            )?;
            return Some((BoxOf::Whole(Source::Group(id)), placed));
        }
        let CanvasSelection::Whole { of, left_out } = &self.selection else {
            return None;
        };
        let from = of.shape_from().filter(|_| left_out.is_empty())?;
        let drawing = &self.edit.as_ref()?.job.drawing;
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
                (w > 0.0).then(|| (BoxOf::Whole(from.into()), image_box(&image, h / w)))
            }
            ShapeFrom::Text(id) => {
                let text = drawing.texts.iter().find(|t| t.id == id)?.clone();
                let key = SizeKey::Text(Box::new(text.clone()));
                let fonts = self.font_source();
                let job = self.edit.as_ref().map(|e| &e.job)?;
                let size = self.box_size.get(key, || {
                    chipload_pipeline::texts::text_size(job, &text, &*fonts).ok()
                })?;
                Some((BoxOf::Whole(from.into()), text_box(&text, size)))
            }
            ShapeFrom::Dxf(id) => {
                // Turned as its placement turns it.
                let rotation = drawing.dxfs.iter().find(|d| d.id == id)?.placement.rotation;
                let pad = SHAPES_BOX_PAD_PX / self.view.scale;
                let placed =
                    self.turned_box_of(|f, _| f == Some(ShapeFrom::Dxf(id)), rotation, pad)?;
                Some((BoxOf::Whole(from.into()), placed))
            }
        }
    }

    /// DXF `id`'s shapes.
    fn dxf_shapes(&self, id: u64) -> Vec<u64> {
        let Some(edit) = self.edit.as_ref() else {
            return Vec::new();
        };
        let shapes = edit.job.drawing.shapes.iter();
        shapes
            .filter(|s| s.dxf() == Some(id))
            .map(|s| s.id)
            .collect()
    }

    /// The box round the contours' segments that `take` takes (by where they come from and
    /// their shape), in axes turned `degrees`, standing `pad` mm off.
    fn turned_box_of(
        &self,
        take: impl Fn(Option<ShapeFrom>, Option<u64>) -> bool,
        degrees: f64,
        pad: f64,
    ) -> Option<PlacedBox> {
        let prims: Vec<_> = self
            .contours
            .iter()
            .flat_map(|c| &c.parts)
            .filter(|p| take(p.segment.from, p.segment.shape))
            .map(|p| p.segment.prim)
            .collect();
        chipload_pipeline::box_handles::turned_box(&prims, degrees, pad)
    }

    /// The box round the selected drawn shapes, standing off them: all drawn, and not a
    /// single circle, rectangle or polygon, which have their own move and turn handles.
    fn shapes_box(&self) -> Option<(BoxOf, PlacedBox)> {
        let (ids, imported) = self.deletable_shapes();
        if imported || ids.is_empty() {
            return None;
        }
        if let [only] = ids[..]
            && self.shape(only).is_some_and(|s| {
                matches!(
                    s.geometry,
                    Geometry::Circle { .. } | Geometry::Rectangle { .. } | Geometry::Polygon { .. }
                )
            })
        {
            return None;
        }
        let job = &self.edit.as_ref()?.job;
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
        // A group follows the box while it is dragged, its letters too.
        if let Some(BoxDrag {
            of: BoxOf::Whole(Source::Group(id)),
            start,
            now,
            pad,
            ..
        }) = &self.drag.boxed
            && now != start
        {
            let (from, to, scale, degrees) = box_transform(start, now, *pad);
            let ids = self.edit.as_ref().map(|e| e.job.drawing.group_shapes(*id));
            lines.extend(self.moved_outlines(&ids.unwrap_or_default(), from, to, scale, degrees));
        }
        // So does a DXF (docs/specs/dxf-whole-on-canvas.md §2).
        if let Some(BoxDrag {
            of: BoxOf::Whole(Source::Dxf(id)),
            start,
            now,
            pad,
            ..
        }) = &self.drag.boxed
            && now != start
        {
            let (from, to, scale, degrees) = box_transform(start, now, *pad);
            lines.extend(self.moved_outlines(&self.dxf_shapes(*id), from, to, scale, degrees));
        }
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
                .job
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
        self.start_box_drag_at(handle, (0.0, 0.0));
    }

    /// A drag starting at `at` (drawing coordinates) on the contour `picked`, a line of the
    /// selected picture or text: it moves the picture or text whole, as its Move handle
    /// does, grabbed where it was (or carries it onto the shape library). False when that
    /// isn't what is picked.
    pub fn start_whole_drag(&mut self, picked: Option<usize>, at: Point) -> bool {
        let Some(picked) = picked else {
            return false;
        };
        if !matches!(self.selection, CanvasSelection::Whole { .. })
            || !self.selected_ids().contains(&picked)
        {
            return false;
        }
        let Some((BoxOf::Whole(Source::Image(_) | Source::Text(_) | Source::Dxf(_)), placed)) =
            self.selected_box()
        else {
            return false;
        };
        // Grabbed where it snaps, as drawn shapes are: that point lands where the cursor snaps.
        let at = self.snap_world(at, false, false);
        let (cx, cy) = placed.center();
        self.start_box_drag_at(BoxHandle::Move, (cx - at.0, cy - at.1));
        self.dragging_box()
    }

    fn start_box_drag_at(&mut self, handle: BoxHandle, offset: Point) {
        if let Some((of, start)) = self.selected_box() {
            let pad = match of {
                BoxOf::Shapes(_) | BoxOf::Whole(Source::Group(_) | Source::Dxf(_)) => {
                    SHAPES_BOX_PAD_PX / self.view.scale
                }
                BoxOf::Whole(_) => 0.0,
            };
            self.drag.boxed = Some(BoxDrag {
                of,
                handle,
                start,
                now: start,
                free: false,
                pad,
                offset,
                carried: false,
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
        let to = (to.0 + drag.offset.0, to.1 + drag.offset.1);
        let dragged = match drag.handle {
            BoxHandle::Size(k) if drag.pad > 0.0 => {
                turned_size_shapes_box(&drag.start, drag.pad, k, to)
            }
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
        if let BoxOf::Whole(Source::Group(id)) = drag.of {
            let (from, to, scale, degrees) = box_transform(&drag.start, &drag.now, drag.pad);
            self.transform_group(id, from, to, scale, degrees);
            return;
        }
        if let BoxOf::Whole(Source::Dxf(id)) = drag.of {
            let (from, to, scale, degrees) = box_transform(&drag.start, &drag.now, drag.pad);
            self.drag_dxf(id, from, to, scale, degrees);
            return;
        }
        let Some(edit) = self.edit.as_ref() else {
            return;
        };
        let drawing = &edit.job.drawing;
        let BoxOf::Whole(of) = drag.of else {
            return;
        };
        match of {
            Source::Image(id) => {
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
            Source::Text(id) => {
                let Some(text) = drawing.texts.iter().find(|t| t.id == id).cloned() else {
                    return;
                };
                let moved = text_in_box(&text, drag.start.size, &drag.now, drag.free);
                self.set_text(id, &moved);
            }
            Source::Dxf(_) | Source::Group(_) => {}
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
        let job = edit.job.clone();
        match chipload_pipeline::shapes::transform_shapes(&job, ids, from, to, scale, degrees) {
            Ok(job) => {
                edit.adopt(job);
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

    /// The dragged picture or text carried onto the shape library, or back off it: while
    /// carried it stays where it was.
    pub(super) fn carry_box(&mut self, carried: bool) {
        if let Some(drag) = self.drag.boxed.as_mut()
            && drag.handle == BoxHandle::Move
        {
            drag.carried = carried;
            if carried {
                drag.now = drag.start;
            }
        }
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

/// [`size_shapes_box`] for a box turned as a group's or a DXF's is: the handle's point taken
/// into the box's own axes, the box sized there, and turned back about the same centre.
fn turned_size_shapes_box(
    start: &PlacedBox,
    pad: f64,
    k: usize,
    to: Point,
) -> Result<PlacedBox, Message> {
    if start.rotation == 0.0 {
        return size_shapes_box(start, pad, k, to);
    }
    let c = start.center();
    let turn = |p: Point, degrees: f64| {
        let q = chipload_job::rotated([p.0 - c.0, p.1 - c.1], degrees);
        (q[0] + c.0, q[1] + c.1)
    };
    let upright = PlacedBox {
        rotation: 0.0,
        ..*start
    };
    let sized = size_shapes_box(&upright, pad, k, turn(to, -start.rotation))?;
    let centre = turn(sized.center(), start.rotation);
    Ok(PlacedBox {
        corner: (centre.0 - sized.size.0 / 2.0, centre.1 - sized.size.1 / 2.0),
        size: sized.size,
        rotation: start.rotation,
    })
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
        construction: false,
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
