// SPDX-License-Identifier: GPL-3.0-or-later
//! Frames for signs (docs/specs/shape-library.md §3): a frame's outline and border lines
//! added to a project as drawn shapes, round shapes already there or to a size. The
//! geometry is `chipload_draw::frames`'; the borders are offsets (`drawing_edit`).

use chipload_draw::frames::{self, FrameError};
use chipload_i18n::{Digits, Message, Value};
use chipload_job::{Geometry, Job};

use crate::PipelineError;
use crate::drawing_edit::{Corners, Edited, OffsetSide, layer_cutters, offset_shapes, polyline_of};
use crate::shapes::{add_shape, shapes_bounds};
pub use chipload_draw::frames::FrameKind;

/// The most border lines a frame has.
pub const MOST_BORDERS: u32 = 3;
/// The layers a frame's outline and its borders go on when none is chosen (decision 5).
pub const FRAME_LAYER: &str = "frame";
pub const BORDER_LAYER: &str = "border";

/// How big a frame is and where it goes.
#[derive(Debug, Clone, PartialEq)]
pub enum FrameSize {
    /// Round shapes `ids`, `margin` (mm) from their box to the innermost line, centred on
    /// them.
    Around { ids: Vec<u64>, margin: f64 },
    /// The outline `width` × `height` (mm), centred on `center`, or on the stock.
    Fixed {
        width: f64,
        height: f64,
        center: Option<(f64, f64)>,
    },
}

/// A frame to add.
#[derive(Debug, Clone, PartialEq)]
pub struct NewFrame {
    pub kind: FrameKind,
    /// The corner size (mm); None: a tenth of the shorter side.
    pub r: Option<f64>,
    /// Border lines inside the outline, 0 to [`MOST_BORDERS`].
    pub borders: u32,
    /// Between the outline and the first border, and between borders (mm).
    pub gap: f64,
    pub size: FrameSize,
    pub layer: String,
    pub border_layer: String,
}

impl NewFrame {
    /// A frame of `kind` and `size` with the defaults: no borders, a 5 mm gap, the default
    /// layers.
    pub fn new(kind: FrameKind, size: FrameSize) -> NewFrame {
        NewFrame {
            kind,
            r: None,
            borders: 0,
            gap: 5.0,
            size,
            layer: FRAME_LAYER.to_string(),
            border_layer: BORDER_LAYER.to_string(),
        }
    }
}

/// Where a frame goes and its size, before it is added (for the window's preview and its
/// fields).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FramePlace {
    pub center: (f64, f64),
    pub width: f64,
    pub height: f64,
    pub r: f64,
}

fn refused(message: Message) -> PipelineError {
    PipelineError::Job(message)
}

fn length(mm: f64) -> Value {
    Value::Length(mm, Digits::Shortest)
}

/// The frame's centre, size and corner size.
pub fn place_frame(job: &Job, frame: &NewFrame) -> Result<FramePlace, PipelineError> {
    if frame.borders > MOST_BORDERS {
        return Err(refused(
            Message::new("pipeline-frame-borders")
                .arg("most", MOST_BORDERS)
                .arg("got", frame.borders),
        ));
    }
    if frame.borders > 0 && !(frame.gap.is_finite() && frame.gap > 0.0) {
        return Err(refused(
            Message::new("pipeline-frame-gap").arg("got", length(frame.gap)),
        ));
    }
    let inset = f64::from(frame.borders) * frame.gap;
    let kind = frame.kind;
    let (center, width, height, r) = match &frame.size {
        FrameSize::Around { ids, margin } => {
            if !(margin.is_finite() && *margin >= 0.0) {
                return Err(refused(
                    Message::new("pipeline-frame-margin").arg("got", length(*margin)),
                ));
            }
            let ((x0, y0), (x1, y1)) = shapes_bounds(job, ids)
                .ok_or_else(|| refused(Message::new("pipeline-frame-nothing")))?;
            let inner = (x1 - x0 + 2.0 * margin, y1 - y0 + 2.0 * margin);
            let r = frame
                .r
                .unwrap_or_else(|| kind.default_r(inner.0 + 2.0 * inset, inner.1 + 2.0 * inset));
            let (w, h) = frames::fit_round(kind, inner, inset, r).map_err(why)?;
            (((x0 + x1) / 2.0, (y0 + y1) / 2.0), w, h, r)
        }
        FrameSize::Fixed {
            width,
            height,
            center,
        } => {
            let center = center.unwrap_or_else(|| {
                let ((x0, y0), (x1, y1)) = job.stock_bounds();
                ((x0 + x1) / 2.0, (y0 + y1) / 2.0)
            });
            let r = frame.r.unwrap_or_else(|| kind.default_r(*width, *height));
            (center, *width, *height, r)
        }
    };
    frames::outline(kind, center, width, height, r).map_err(why)?;
    Ok(FramePlace {
        center,
        width,
        height,
        r,
    })
}

fn why(error: FrameError) -> PipelineError {
    refused(match error {
        FrameError::BadSize => Message::new("pipeline-frame-size"),
        FrameError::BadR { largest } => {
            Message::new("pipeline-frame-r").arg("largest", length(largest))
        }
        FrameError::NoFit => Message::new("pipeline-frame-no-fit"),
    })
}

/// The frame's outline as a drawn shape's geometry: a rectangle for the rectangular ones,
/// a circle for a round oval, a closed polyline otherwise.
fn outline_geometry(kind: FrameKind, place: &FramePlace) -> Result<Geometry, PipelineError> {
    let center = [place.center.0, place.center.1];
    let square = (place.width - place.height).abs() < chipload_core::tolerances::CHAIN_TOL_MM;
    Ok(match kind {
        FrameKind::Rectangle | FrameKind::Rounded => Geometry::Rectangle {
            center,
            length: place.width,
            width: place.height,
            angle: 0.0,
            corner_radius: if kind == FrameKind::Rounded {
                place.r
            } else {
                0.0
            },
        },
        FrameKind::Oval if square => Geometry::Circle {
            center,
            radius: place.width / 2.0,
        },
        _ => {
            let chain = frames::outline(kind, place.center, place.width, place.height, place.r)
                .map_err(why)?;
            polyline_of(&chain)
        }
    })
}

/// Add the frame: its outline on `layer`, then its borders on `border_layer`, outside in.
/// `new_ids` holds the outline's id first.
pub fn add_frame(job: &Job, frame: &NewFrame) -> Result<Edited, PipelineError> {
    let place = place_frame(job, frame)?;
    let geometry = outline_geometry(frame.kind, &place)?;
    let (mut out, outline) = add_shape(job, &frame.layer, geometry)?;
    let mut new_ids = vec![outline];
    if frame.borders > 0 {
        let borders = offset_shapes(
            &out,
            &[outline],
            frame.gap,
            Some(OffsetSide::Inside),
            Corners::Round,
            frame.borders,
        )
        .map_err(|_| {
            refused(
                Message::new("pipeline-frame-borders-fit")
                    .arg("count", frame.borders)
                    .arg("gap", length(frame.gap)),
            )
        })?;
        // An inward offset across a waist gives more than one line: not a border.
        if borders.new_ids.len() != frame.borders as usize {
            return Err(refused(
                Message::new("pipeline-frame-borders-fit")
                    .arg("count", frame.borders)
                    .arg("gap", length(frame.gap)),
            ));
        }
        out = borders.job;
        for shape in &mut out.drawing.shapes {
            if borders.new_ids.contains(&shape.id) {
                shape.layer = frame.border_layer.clone();
            }
        }
        new_ids.extend(borders.new_ids);
    }
    crate::refuse_problems(&out.drawing.validate())?;
    let mut notes = Vec::new();
    notes.extend(layer_cutters(&out, &frame.layer, 1));
    if frame.borders > 0 {
        notes.extend(layer_cutters(
            &out,
            &frame.border_layer,
            frame.borders as usize,
        ));
    }
    Ok(Edited {
        job: out,
        new_ids,
        notes,
        joined: None,
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;
    use crate::empty_project;
    use crate::texts::{NewText, add_text};

    fn project_with_text() -> (Job, Vec<u64>) {
        let mut new = NewText::new("SIGN", 30.0);
        (new.x, new.y) = (150.0, 100.0);
        let job = add_text(&empty_project(), new, &crate::fonts::BuiltinOnly)
            .unwrap()
            .0;
        let ids = job
            .drawing
            .shapes
            .iter()
            .filter(|s| s.text() == Some(1))
            .map(|s| s.id)
            .collect();
        (job, ids)
    }

    #[test]
    fn a_frame_round_a_text_is_centred_with_the_margin_inside_its_border() {
        let (job, letters) = project_with_text();
        let ((x0, y0), (x1, y1)) = shapes_bounds(&job, &letters).unwrap();
        let mut frame = NewFrame::new(
            FrameKind::Rounded,
            FrameSize::Around {
                ids: letters.clone(),
                margin: 10.0,
            },
        );
        frame.borders = 1;
        frame.r = Some(4.0);
        let edited = add_frame(&job, &frame).unwrap();
        assert_eq!(edited.new_ids.len(), 2);
        let shape = |id| {
            edited
                .job
                .drawing
                .shapes
                .iter()
                .find(|s| s.id == id)
                .unwrap()
                .clone()
        };
        let outline = shape(edited.new_ids[0]);
        let border = shape(edited.new_ids[1]);
        assert_eq!(outline.layer, "frame");
        assert_eq!(border.layer, "border");
        let Geometry::Rectangle {
            center,
            length,
            width,
            ..
        } = border.geometry
        else {
            panic!("{:?}", border.geometry);
        };
        assert!((center[0] - (x0 + x1) / 2.0).abs() < 1e-9);
        assert!((center[1] - (y0 + y1) / 2.0).abs() < 1e-9);
        // The border is the text's box plus the margin, its corners not cutting in.
        assert!((length - (x1 - x0 + 20.0)).abs() < 1e-9, "{length}");
        assert!((width - (y1 - y0 + 20.0)).abs() < 1e-9, "{width}");
        let Geometry::Rectangle { length: outer, .. } = outline.geometry else {
            panic!();
        };
        assert!((outer - length - 10.0).abs() < 1e-9);
    }

    #[test]
    fn a_frame_to_a_size_sits_on_the_stock_and_borders_must_fit() {
        let job = empty_project();
        let ((x0, y0), (x1, y1)) = job.stock_bounds();
        let size = FrameSize::Fixed {
            width: 200.0,
            height: 100.0,
            center: None,
        };
        let mut frame = NewFrame::new(FrameKind::Oval, size.clone());
        frame.borders = 2;
        let edited = add_frame(&job, &frame).unwrap();
        let ((a0, b0), (a1, b1)) = shapes_bounds(&edited.job, &edited.new_ids[..1]).unwrap();
        assert!(((a0 + a1) - (x0 + x1)).abs() < 0.02 && ((b0 + b1) - (y0 + y1)).abs() < 0.02);
        assert!((a1 - a0 - 200.0).abs() < 0.02);

        frame.gap = 30.0;
        let err = add_frame(&job, &frame).unwrap_err();
        assert!(err.to_string().contains("2 borders 30 mm apart"), "{err}");
        frame.borders = 4;
        assert!(add_frame(&job, &frame).is_err());

        let mut round = NewFrame::new(
            FrameKind::Oval,
            FrameSize::Fixed {
                width: 80.0,
                height: 80.0,
                center: Some((10.0, 10.0)),
            },
        );
        round.borders = 1;
        let edited = add_frame(&job, &round).unwrap();
        assert!(
            edited
                .job
                .drawing
                .shapes
                .iter()
                .all(|s| matches!(s.geometry, Geometry::Circle { .. }))
        );

        let mut big = NewFrame::new(FrameKind::Notched, size);
        big.r = Some(80.0);
        let err = add_frame(&job, &big).unwrap_err();
        assert!(err.to_string().contains("at most 50 mm"), "{err}");
    }

    #[test]
    fn nothing_to_frame_is_refused() {
        let job = empty_project();
        let frame = NewFrame::new(
            FrameKind::Rectangle,
            FrameSize::Around {
                ids: vec![99],
                margin: 5.0,
            },
        );
        assert!(add_frame(&job, &frame).is_err());
    }
}
