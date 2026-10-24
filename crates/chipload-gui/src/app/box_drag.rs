// SPDX-License-Identifier: GPL-3.0-or-later
//! Handles on a selected picture's or text's box: move, turn, size
//! (docs/specs/box-handles.md). The box maths is `chipload_pipeline::box_handles`; a drag
//! shows the box and sets the picture or text when it ends.

use chipload_core::Point;
use chipload_core::chain::ShapeFrom;
use chipload_job::Text;
use chipload_pipeline::box_handles::{
    BoxHandle, PlacedBox, box_handles, drag_box, image_box, image_in_box, text_box, text_in_box,
};

use super::*;

/// How far beyond the box's side its turn handle sits, in screen pixels.
const TURN_HANDLE_PX: f64 = 24.0;

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
    from: ShapeFrom,
    handle: BoxHandle,
    /// The box when the drag began, which every frame starts from.
    start: PlacedBox,
    /// Where the drag has it now.
    now: PlacedBox,
    /// Alt was held: no steps.
    free: bool,
}

impl ChiploadApp {
    /// The selected picture or text and its box: a whole one selected, in top view, while
    /// the project can be drawn on and nothing is being drawn, measured or placed.
    pub(super) fn selected_box(&mut self) -> Option<(ShapeFrom, PlacedBox)> {
        if self.three_d
            || self.sketch.is_some()
            || self.measure.is_some()
            || self.placing_text()
            || !self.can_draw()
            || !self.show_drawing()
        {
            return None;
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
                (w > 0.0).then(|| (from, image_box(&image, h / w)))
            }
            ShapeFrom::Text(id) => {
                let text = drawing.texts.iter().find(|t| t.id == id)?.clone();
                let key = SizeKey::Text(Box::new(text.clone()));
                let size = self
                    .box_size
                    .get(key, || chipload_pipeline::texts::text_size(&text).ok())?;
                Some((from, text_box(&text, size)))
            }
            ShapeFrom::Dxf(_) => None,
        }
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
        vec![vec![c[0], c[1], c[2], c[3], c[0]], vec![edge, turn]]
    }

    pub fn start_box_drag(&mut self, handle: BoxHandle) {
        if let Some((from, start)) = self.selected_box() {
            self.drag.boxed = Some(BoxDrag {
                from,
                handle,
                start,
                now: start,
                free: false,
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
        match drag_box(&drag.start, drag.handle, to, free) {
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
        let Some(edit) = self.edit.as_ref() else {
            return;
        };
        let drawing = &edit.settings.drawing;
        match drag.from {
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
