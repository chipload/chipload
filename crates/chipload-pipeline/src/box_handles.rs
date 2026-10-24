// SPDX-License-Identifier: GPL-3.0-or-later
//! Handles for a picture's or a text's box on the canvas: move, turn and size
//! (docs/specs/box-handles.md). Pure functions; the GUI shows the handles and applies the
//! result through the same setters as Properties.

use chipload_core::Point;
use chipload_i18n::{Digits, Message, Value};
use chipload_job::{Image, Text, TextAnchor, rotated};

/// The smallest box side a size drag gives, mm.
pub const MIN_SIZE_MM: f64 = 1.0;
/// A turn drag's step, degrees; and with snapping off.
pub const TURN_STEP_DEGREES: f64 = 15.0;
pub const FREE_TURN_STEP_DEGREES: f64 = 0.1;
/// A size drag rounds a picture's width or a text's height to this, mm (snapping on).
pub const SIZE_STEP_MM: f64 = 0.1;

/// A box before its turn: its lower-left corner and size, turned `rotation` degrees
/// counter-clockwise about its centre.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlacedBox {
    pub corner: Point,
    pub size: (f64, f64),
    pub rotation: f64,
}

/// A handle of a box.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoxHandle {
    /// At the centre: moves the box.
    Move,
    /// Beyond the middle of the box's own +X side: turns it about its centre.
    Turn,
    /// Corner k, counter-clockwise from the lower-left before the turn: scales the box
    /// evenly, the opposite corner staying put.
    Size(usize),
}

impl PlacedBox {
    pub fn center(&self) -> Point {
        (
            self.corner.0 + self.size.0 / 2.0,
            self.corner.1 + self.size.1 / 2.0,
        )
    }

    /// A point given in the box's own axes from its centre, on the drawing.
    fn at(&self, local: [f64; 2]) -> Point {
        let (cx, cy) = self.center();
        let p = rotated(local, self.rotation);
        (cx + p[0], cy + p[1])
    }

    /// Its four corners, turned: lower-left, lower-right, upper-right, upper-left.
    pub fn corners(&self) -> [Point; 4] {
        let (hw, hh) = (self.size.0 / 2.0, self.size.1 / 2.0);
        [[-hw, -hh], [hw, -hh], [hw, hh], [-hw, hh]].map(|p| self.at(p))
    }
}

/// The handles of `b` and where they are; the turn handle sits `turn_reach` mm beyond the
/// box (the GUI passes a fixed distance on screen).
pub fn box_handles(b: &PlacedBox, turn_reach: f64) -> Vec<(BoxHandle, Point)> {
    let mut out = vec![
        (BoxHandle::Move, b.center()),
        (BoxHandle::Turn, b.at([b.size.0 / 2.0 + turn_reach, 0.0])),
    ];
    out.extend(
        b.corners()
            .into_iter()
            .enumerate()
            .map(|(k, p)| (BoxHandle::Size(k), p)),
    );
    out
}

/// The box after dragging `handle` of `start` to `to`. `free`: no steps (Alt). `Err` says
/// why the drag gives no box.
pub fn drag_box(
    start: &PlacedBox,
    handle: BoxHandle,
    to: Point,
    free: bool,
) -> Result<PlacedBox, Message> {
    match handle {
        BoxHandle::Move => Ok(PlacedBox {
            corner: (to.0 - start.size.0 / 2.0, to.1 - start.size.1 / 2.0),
            ..*start
        }),
        BoxHandle::Turn => {
            let (cx, cy) = start.center();
            let (dx, dy) = (to.0 - cx, to.1 - cy);
            if dx == 0.0 && dy == 0.0 {
                return Err(Message::new("pipeline-box-turn-centre"));
            }
            let step = if free {
                FREE_TURN_STEP_DEGREES
            } else {
                TURN_STEP_DEGREES
            };
            let degrees = (dy.atan2(dx).to_degrees() / step).round() * step;
            Ok(PlacedBox {
                rotation: normalised(degrees),
                ..*start
            })
        }
        BoxHandle::Size(k) if k < 4 => {
            let corners = start.corners();
            let opposite = corners[(k + 2) % 4];
            let diagonal = (corners[k].0 - opposite.0, corners[k].1 - opposite.1);
            let length2 = diagonal.0 * diagonal.0 + diagonal.1 * diagonal.1;
            if length2 == 0.0 {
                return Err(Message::new("pipeline-box-no-size"));
            }
            let factor =
                ((to.0 - opposite.0) * diagonal.0 + (to.1 - opposite.1) * diagonal.1) / length2;
            let size = (start.size.0 * factor, start.size.1 * factor);
            if size.0.min(size.1) < MIN_SIZE_MM {
                return Err(Message::new("pipeline-box-too-small")
                    .arg("min", Value::Length(MIN_SIZE_MM, Digits::Shortest)));
            }
            // The centre moves along the diagonal, so the opposite corner stays.
            let center = (
                opposite.0 + diagonal.0 * factor / 2.0,
                opposite.1 + diagonal.1 * factor / 2.0,
            );
            Ok(PlacedBox {
                corner: (center.0 - size.0 / 2.0, center.1 - size.1 / 2.0),
                size,
                rotation: start.rotation,
            })
        }
        BoxHandle::Size(k) => Err(Message::new("pipeline-box-no-corner").arg("corner", k)),
    }
}

/// Degrees in 0 to <360.
fn normalised(degrees: f64) -> f64 {
    let d = degrees.rem_euclid(360.0);
    if d >= 360.0 { 0.0 } else { d }
}

/// `value` rounded to `step`, unless `free`.
fn stepped(value: f64, step: f64, free: bool) -> f64 {
    if free {
        value
    } else {
        (value / step).round() * step
    }
}

/// A picture's box; `aspect` is its height over its width (from its pixels).
pub fn image_box(image: &Image, aspect: f64) -> PlacedBox {
    PlacedBox {
        corner: (image.x, image.y),
        size: (image.width, image.width * aspect),
        rotation: image.rotation,
    }
}

/// `image` placed in box `b`: its width rounded to 0.1 mm unless `free`, the box scaled
/// about its centre to match. Returns (x, y, width, rotation).
pub fn image_in_box(b: &PlacedBox, free: bool) -> (f64, f64, f64, f64) {
    let width = stepped(b.size.0, SIZE_STEP_MM, free);
    let height = if b.size.0 > 0.0 {
        b.size.1 * width / b.size.0
    } else {
        b.size.1
    };
    let (cx, cy) = b.center();
    (cx - width / 2.0, cy - height / 2.0, width, b.rotation)
}

/// A text's box, `size` being its letters' box (`texts::text_size`).
pub fn text_box(text: &Text, size: (f64, f64)) -> PlacedBox {
    PlacedBox {
        corner: crate::texts::box_corner(text, size),
        size,
        rotation: text.rotation,
    }
}

/// `text` placed in box `b` (from its box of `size`): its capital height scaled with the
/// box and rounded to 0.1 mm unless `free`, its anchor kept. The letters' new box is
/// assumed to scale with the height; it is centred where the dragged box is.
pub fn text_in_box(text: &Text, size: (f64, f64), b: &PlacedBox, free: bool) -> Text {
    // Moved or turned only: the height stays exactly.
    let height = if b.size == size || size.1 <= 0.0 {
        text.height
    } else {
        stepped(text.height * b.size.1 / size.1, SIZE_STEP_MM, free)
    };
    let scaled = if text.height > 0.0 {
        height / text.height
    } else {
        1.0
    };
    let new_size = (size.0 * scaled, size.1 * scaled);
    let (cx, cy) = b.center();
    let (x, y) = match text.anchor {
        TextAnchor::Corner => (cx - new_size.0 / 2.0, cy - new_size.1 / 2.0),
        TextAnchor::Center => (cx, cy),
    };
    Text {
        x,
        y,
        height,
        rotation: b.rotation,
        ..text.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: Point, b: Point) -> bool {
        (a.0 - b.0).abs() < 1e-9 && (a.1 - b.1).abs() < 1e-9
    }

    fn sample() -> PlacedBox {
        PlacedBox {
            corner: (10.0, 20.0),
            size: (40.0, 10.0),
            rotation: 30.0,
        }
    }

    #[test]
    fn moving_puts_the_centre_at_the_cursor() {
        let moved = drag_box(&sample(), BoxHandle::Move, (100.0, 50.0), false).unwrap();
        assert!(close(moved.center(), (100.0, 50.0)));
        assert_eq!((moved.size, moved.rotation), ((40.0, 10.0), 30.0));
    }

    #[test]
    fn turning_follows_the_cursor_in_steps() {
        let b = sample();
        let (cx, cy) = b.center();
        let a = 50f64.to_radians();
        let to = (cx + 10.0 * a.cos(), cy + 10.0 * a.sin());
        assert_eq!(
            drag_box(&b, BoxHandle::Turn, to, false).unwrap().rotation,
            45.0
        );
        let free = drag_box(&b, BoxHandle::Turn, to, true).unwrap().rotation;
        assert!((free - 50.0).abs() < 1e-9);
        let below = (cx + 1.0, cy - 1.0);
        assert_eq!(
            drag_box(&b, BoxHandle::Turn, below, false)
                .unwrap()
                .rotation,
            315.0
        );
        assert!(drag_box(&b, BoxHandle::Turn, (cx, cy), false).is_err());
    }

    #[test]
    fn a_corner_scales_evenly_and_keeps_the_opposite_corner() {
        let b = sample();
        let corners = b.corners();
        // Dragging the upper-right corner out to twice the diagonal.
        let to = (
            2.0 * corners[2].0 - corners[0].0,
            2.0 * corners[2].1 - corners[0].1,
        );
        let scaled = drag_box(&b, BoxHandle::Size(2), to, false).unwrap();
        assert!((scaled.size.0 - 80.0).abs() < 1e-9 && (scaled.size.1 - 20.0).abs() < 1e-9);
        assert!(close(scaled.corners()[0], corners[0]));
        assert!(close(scaled.corners()[2], to));
        assert_eq!(scaled.rotation, 30.0);
    }

    #[test]
    fn too_small_is_refused() {
        let b = sample();
        let opposite = b.corners()[0];
        assert!(drag_box(&b, BoxHandle::Size(2), opposite, false).is_err());
        assert!(drag_box(&b, BoxHandle::Size(4), (0.0, 0.0), false).is_err());
    }

    #[test]
    fn a_picture_in_a_box_rounds_its_width_about_the_centre() {
        let b = PlacedBox {
            corner: (0.0, 0.0),
            size: (50.04, 25.02),
            rotation: 10.0,
        };
        let (x, y, width, rotation) = image_in_box(&b, false);
        assert!((width - 50.0).abs() < 1e-9);
        assert!((x - 0.02).abs() < 1e-9 && (y - 0.01).abs() < 1e-9);
        assert_eq!(rotation, 10.0);
    }

    #[test]
    fn a_centred_text_keeps_its_anchor_at_the_new_centre() {
        let text = Text {
            id: 1,
            text: "A".into(),
            font: "FreeSans".into(),
            height: 10.0,
            x: 5.0,
            y: 5.0,
            anchor: TextAnchor::Center,
            rotation: 0.0,
            align: chipload_job::TextAlign::Left,
            spacing: 0.0,
            line_spacing: 1.6,
            layer: "text".into(),
        };
        let size = (8.0, 10.0);
        let b = text_box(&text, size);
        assert!(close(b.center(), (5.0, 5.0)));
        let grown = drag_box(&b, BoxHandle::Size(2), (17.0, 20.0), false).unwrap();
        let after = text_in_box(&text, size, &grown, false);
        assert!((after.height - 20.0).abs() < 1e-9);
        assert_eq!(after.anchor, TextAnchor::Center);
        assert!(close((after.x, after.y), grown.center()));
        let moved = drag_box(&b, BoxHandle::Move, (50.0, 60.0), false).unwrap();
        let after = text_in_box(&text, size, &moved, false);
        assert_eq!((after.x, after.y, after.height), (50.0, 60.0, 10.0));
    }
}
