// SPDX-License-Identifier: GPL-3.0-or-later
//! Frames for signs (docs/specs/shape-library.md §3): a frame's outline and border lines
//! added to a project as drawn shapes, round shapes already there or to a size. The
//! geometry is `chipload_draw::frames`'; the borders are offsets (`drawing_edit`).

use chipload_core::tolerances::CHAIN_TOL_MM;
use chipload_draw::frames::{self, FrameError};
use chipload_i18n::{Digits, Message, Value};
use chipload_job::{Frame, FrameAround, FrameCorners, FrameShape, Geometry, Job, Shape};

use crate::PipelineError;
use crate::drawing_edit::{
    Corners, Edited, OffsetSide, layer_cutters, offset_geometry, polyline_of,
};
use crate::shapes::{add_shape, shapes_bounds};
pub use chipload_draw::frames::{CornerStyle, FrameKind};

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
    /// The borders' own corner style, for a rectangular frame; None: the outline's shape.
    pub border_corners: Option<CornerStyle>,
    /// Each border two lines, twice `border_width` apart.
    pub double: bool,
    /// A border's groove width at the surface (mm); 0: lines as drawn.
    pub border_width: f64,
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
            border_corners: None,
            double: false,
            border_width: 0.0,
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

/// A frame's border lines: how many borders, how far apart, their corners, single or
/// double, and their grooves' width (docs/specs/sign-maker.md §5).
#[derive(Debug, Clone, Copy, PartialEq)]
struct BorderLines {
    count: u32,
    gap: f64,
    corners: Option<CornerStyle>,
    double: bool,
    width: f64,
}

impl BorderLines {
    fn of_new(frame: &NewFrame) -> BorderLines {
        BorderLines {
            count: frame.borders,
            gap: frame.gap,
            corners: frame.border_corners,
            double: frame.double,
            width: frame.border_width,
        }
    }

    fn of_record(frame: &Frame) -> BorderLines {
        let per = if frame.double { 2 } else { 1 };
        BorderLines {
            count: u32::try_from(frame.borders.len() / per).unwrap_or(u32::MAX),
            gap: frame.gap,
            corners: frame.border_corners.map(style_of),
            double: frame.double,
            width: frame.border_width,
        }
    }

    /// From a double border's first line to its second.
    fn pair(&self) -> f64 {
        if self.double { 2.0 * self.width } else { 0.0 }
    }

    /// How far in from the outline each line is, outside in.
    fn lines(&self) -> Vec<f64> {
        let mut out = Vec::new();
        for k in 1..=self.count {
            let d = f64::from(k) * self.gap + f64::from(k - 1) * self.pair();
            out.push(d);
            if self.double {
                out.push(d + self.pair());
            }
        }
        out
    }

    /// From the outline to the inner edge of the innermost groove: where the margin is
    /// measured from.
    fn inset(&self) -> f64 {
        match self.lines().last() {
            Some(d) => d + self.width / 2.0,
            None => 0.0,
        }
    }

    /// Why these borders can't be, for a `kind` frame.
    fn problem(&self, kind: FrameKind) -> Option<Message> {
        if self.count > MOST_BORDERS {
            return Some(
                Message::new("pipeline-frame-borders")
                    .arg("most", MOST_BORDERS)
                    .arg("got", self.count),
            );
        }
        if self.count == 0 {
            return None;
        }
        if !(self.gap.is_finite() && self.gap > 0.0) {
            return Some(Message::new("pipeline-frame-gap").arg("got", length(self.gap)));
        }
        if !(self.width.is_finite() && self.width >= 0.0) {
            return Some(
                Message::new("pipeline-frame-border-width").arg("got", length(self.width)),
            );
        }
        if self.double && self.width <= 0.0 {
            return Some(Message::new("pipeline-frame-double-width"));
        }
        if self.corners.is_some() && kind.corner_style().is_none() {
            return Some(Message::new("pipeline-frame-border-corners"));
        }
        None
    }
}

/// A corner style as the record keeps it, and back.
pub fn style_of(corners: FrameCorners) -> CornerStyle {
    match corners {
        FrameCorners::Sharp => CornerStyle::Sharp,
        FrameCorners::Round => CornerStyle::Round,
        FrameCorners::Notch => CornerStyle::Notch,
        FrameCorners::Chamfer => CornerStyle::Chamfer,
        FrameCorners::Step => CornerStyle::Step,
    }
}

pub fn corners_of(style: CornerStyle) -> FrameCorners {
    match style {
        CornerStyle::Sharp => FrameCorners::Sharp,
        CornerStyle::Round => FrameCorners::Round,
        CornerStyle::Notch => FrameCorners::Notch,
        CornerStyle::Chamfer => FrameCorners::Chamfer,
        CornerStyle::Step => FrameCorners::Step,
    }
}

fn refused(message: Message) -> PipelineError {
    PipelineError::Job(message)
}

fn length(mm: f64) -> Value {
    Value::Length(mm, Digits::Shortest)
}

/// The frame's centre, size and corner size.
pub fn place_frame(job: &Job, frame: &NewFrame) -> Result<FramePlace, PipelineError> {
    let kind = frame.kind;
    let borders = BorderLines::of_new(frame);
    if let Some(problem) = borders.problem(kind) {
        return Err(refused(problem));
    }
    let inset = borders.inset();
    let place = match &frame.size {
        FrameSize::Around { ids, margin } => place_round(job, kind, ids, *margin, inset, frame.r)?,
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
            FramePlace {
                center,
                width: *width,
                height: *height,
                r,
            }
        }
    };
    frames::outline(kind, place.center, place.width, place.height, place.r).map_err(why)?;
    Ok(place)
}

/// A `kind` frame round shapes `ids`, `margin` from their box to a line `inset` inside the
/// outline, centred on them; `r` None: the default for that size.
fn place_round(
    job: &Job,
    kind: FrameKind,
    ids: &[u64],
    margin: f64,
    inset: f64,
    r: Option<f64>,
) -> Result<FramePlace, PipelineError> {
    if !(margin.is_finite() && margin >= 0.0) {
        return Err(refused(
            Message::new("pipeline-frame-margin").arg("got", length(margin)),
        ));
    }
    let ((x0, y0), (x1, y1)) =
        shapes_bounds(job, ids).ok_or_else(|| refused(Message::new("pipeline-frame-nothing")))?;
    let inner = (x1 - x0 + 2.0 * margin, y1 - y0 + 2.0 * margin);
    let r = r.unwrap_or_else(|| kind.default_r(inner.0 + 2.0 * inset, inner.1 + 2.0 * inset));
    let (width, height) = frames::fit_round(kind, inner, inset, r).map_err(why)?;
    Ok(FramePlace {
        center: ((x0 + x1) / 2.0, (y0 + y1) / 2.0),
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
    let square = (place.width - place.height).abs() < CHAIN_TOL_MM;
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
            radii: Vec::new(),
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

fn borders_fit(count: u32, gap: f64) -> PipelineError {
    refused(
        Message::new("pipeline-frame-borders-fit")
            .arg("count", count)
            .arg("gap", length(gap)),
    )
}

/// The frame's outline, then its border lines, outside in: offsets of the outline, or
/// rectangles of their own corner style.
fn frame_geometries(
    kind: FrameKind,
    place: &FramePlace,
    borders: &BorderLines,
) -> Result<Vec<Geometry>, PipelineError> {
    let outline = outline_geometry(kind, place)?;
    let shape = Shape {
        id: 0,
        layer: String::new(),
        name: None,
        geometry: outline.clone(),
        construction: false,
    };
    let fit = || borders_fit(borders.count, borders.gap);
    let mut out = vec![outline];
    for d in borders.lines() {
        let line = match borders.corners {
            Some(style) => {
                let (width, height) = (place.width - 2.0 * d, place.height - 2.0 * d);
                if width <= CHAIN_TOL_MM || height <= CHAIN_TOL_MM {
                    return Err(fit());
                }
                let inner = style.kind();
                let inner_place = FramePlace {
                    center: place.center,
                    width,
                    height,
                    r: place.r.min(inner.largest_r(width, height)),
                };
                outline_geometry(inner, &inner_place).map_err(|_| fit())?
            }
            None => {
                let lines = offset_geometry(&shape, d, OffsetSide::Inside, Corners::Round)
                    .map_err(|_| fit())?;
                // An inward offset across a waist gives more than one line: not a border.
                let [line] = <[Geometry; 1]>::try_from(lines).map_err(|_| fit())?;
                line
            }
        };
        out.push(line);
    }
    Ok(out)
}

/// What a frame round shapes `ids` goes round, kept by text and picture so that it still
/// does when their letters and traced shapes are made again: a text or picture all of whose
/// shapes are among `ids`. Some of them only, and other shapes, by id.
fn around_of(job: &Job, ids: &[u64]) -> FrameAround {
    let whole = |from: chipload_core::chain::ShapeFrom| {
        job.drawing
            .shapes
            .iter()
            .filter(|s| s.source() == Some(from))
            .all(|s| ids.contains(&s.id))
    };
    let mut around = FrameAround::default();
    for shape in job.drawing.shapes.iter().filter(|s| ids.contains(&s.id)) {
        match (shape.text(), shape.image(), shape.source()) {
            (Some(text), _, Some(from)) if whole(from) => {
                if !around.texts.contains(&text) {
                    around.texts.push(text);
                }
            }
            (_, Some(image), Some(from)) if whole(from) => {
                if !around.images.contains(&image) {
                    around.images.push(image);
                }
            }
            _ => around.shapes.push(shape.id),
        }
    }
    around
}

/// The shapes a frame goes round now: its texts' letters, its pictures' shapes and its
/// other shapes, those still in the drawing.
pub fn around_ids(job: &Job, around: &FrameAround) -> Vec<u64> {
    job.drawing
        .shapes
        .iter()
        .filter(|s| {
            s.text().is_some_and(|t| around.texts.contains(&t))
                || s.image().is_some_and(|i| around.images.contains(&i))
                || around.shapes.contains(&s.id)
        })
        .map(|s| s.id)
        .collect()
}

/// The frame kind of a frame record, and back.
pub fn kind_of(shape: FrameShape) -> FrameKind {
    match shape {
        FrameShape::Rectangle => FrameKind::Rectangle,
        FrameShape::Rounded => FrameKind::Rounded,
        FrameShape::Notched => FrameKind::Notched,
        FrameShape::Chamfered => FrameKind::Chamfered,
        FrameShape::Oval => FrameKind::Oval,
        FrameShape::Arched => FrameKind::Arched,
        FrameShape::Plaque => FrameKind::Plaque,
        FrameShape::Stepped => FrameKind::Stepped,
        FrameShape::Gable => FrameKind::Gable,
        FrameShape::DoubleArch => FrameKind::DoubleArch,
        FrameShape::Shield => FrameKind::Shield,
        FrameShape::Banner => FrameKind::Banner,
        FrameShape::Octagon => FrameKind::Octagon,
    }
}

pub fn shape_of(kind: FrameKind) -> FrameShape {
    match kind {
        FrameKind::Rectangle => FrameShape::Rectangle,
        FrameKind::Rounded => FrameShape::Rounded,
        FrameKind::Notched => FrameShape::Notched,
        FrameKind::Chamfered => FrameShape::Chamfered,
        FrameKind::Oval => FrameShape::Oval,
        FrameKind::Arched => FrameShape::Arched,
        FrameKind::Plaque => FrameShape::Plaque,
        FrameKind::Stepped => FrameShape::Stepped,
        FrameKind::Gable => FrameShape::Gable,
        FrameKind::DoubleArch => FrameShape::DoubleArch,
        FrameKind::Shield => FrameShape::Shield,
        FrameKind::Banner => FrameShape::Banner,
        FrameKind::Octagon => FrameShape::Octagon,
    }
}

/// Add the frame: its outline on `layer`, then its borders on `border_layer`, outside in,
/// and its record, so that it follows what it goes round (docs/specs/sign-maker.md §4).
/// `new_ids` holds the outline's id first.
pub fn add_frame(job: &Job, frame: &NewFrame) -> Result<Edited, PipelineError> {
    let place = place_frame(job, frame)?;
    let geometries = frame_geometries(frame.kind, &place, &BorderLines::of_new(frame))?;
    let mut out = job.clone();
    let mut new_ids = Vec::new();
    for (i, geometry) in geometries.into_iter().enumerate() {
        let layer = if i == 0 {
            &frame.layer
        } else {
            &frame.border_layer
        };
        let (next, id) = add_shape(&out, layer, geometry)?;
        out = next;
        new_ids.push(id);
    }
    let (around, margin) = match &frame.size {
        FrameSize::Around { ids, margin } => (Some(around_of(job, ids)), *margin),
        FrameSize::Fixed { .. } => (None, 0.0),
    };
    out.drawing.frames.push(Frame {
        outline: new_ids[0],
        borders: new_ids[1..].to_vec(),
        kind: shape_of(frame.kind),
        corner: place.r,
        gap: frame.gap,
        border_corners: frame.border_corners.map(corners_of),
        double: frame.double,
        border_width: frame.border_width,
        around,
        margin,
        center: [place.center.0, place.center.1],
        width: place.width,
        height: place.height,
    });
    crate::refuse_problems(&out.drawing.validate())?;
    let mut notes = Vec::new();
    notes.extend(layer_cutters(&out, &frame.layer, 1));
    if new_ids.len() > 1 {
        notes.extend(layer_cutters(&out, &frame.border_layer, new_ids.len() - 1));
    }
    Ok(Edited {
        job: out,
        new_ids,
        notes,
        joined: None,
    })
}

// -- frames that follow (docs/specs/sign-maker.md §4)

/// Where a frame record says it was last drawn.
fn last_place(frame: &Frame) -> FramePlace {
    FramePlace {
        center: (frame.center[0], frame.center[1]),
        width: frame.width,
        height: frame.height,
        r: frame.corner,
    }
}

/// Where a frame goes now: round what it goes round, or where it is.
fn wanted_place(job: &Job, frame: &Frame) -> Result<FramePlace, Message> {
    let Some(around) = &frame.around else {
        return Ok(last_place(frame));
    };
    let ids = around_ids(job, around);
    if ids.is_empty() {
        return Err(Message::new("pipeline-frame-follows-nothing").arg("id", frame.outline));
    }
    let inset = BorderLines::of_record(frame).inset();
    place_round(
        job,
        kind_of(frame.kind),
        &ids,
        frame.margin,
        inset,
        Some(frame.corner),
    )
    .map_err(|e| {
        Message::new("pipeline-frame-follows-not")
            .arg("id", frame.outline)
            .arg("why", e.message())
    })
}

fn same_place(a: &FramePlace, b: &FramePlace) -> bool {
    let close = |x: f64, y: f64| (x - y).abs() <= CHAIN_TOL_MM;
    close(a.center.0, b.center.0)
        && close(a.center.1, b.center.1)
        && close(a.width, b.width)
        && close(a.height, b.height)
        && close(a.r, b.r)
}

/// Whether two geometries are the same within the chaining tolerance.
fn same_geometry(a: &Geometry, b: &Geometry) -> bool {
    let close = |x: f64, y: f64| (x - y).abs() <= CHAIN_TOL_MM;
    let all_close =
        |a: &[f64], b: &[f64]| a.len() == b.len() && a.iter().zip(b).all(|(x, y)| close(*x, *y));
    match (a, b) {
        (
            Geometry::Rectangle {
                center: c1,
                length: l1,
                width: w1,
                angle: a1,
                corner_radius: r1,
                ..
            },
            Geometry::Rectangle {
                center: c2,
                length: l2,
                width: w2,
                angle: a2,
                corner_radius: r2,
                ..
            },
        ) => all_close(
            &[c1[0], c1[1], *l1, *w1, *a1, *r1],
            &[c2[0], c2[1], *l2, *w2, *a2, *r2],
        ),
        (
            Geometry::Circle {
                center: c1,
                radius: r1,
            },
            Geometry::Circle {
                center: c2,
                radius: r2,
            },
        ) => all_close(&[c1[0], c1[1], *r1], &[c2[0], c2[1], *r2]),
        (
            Geometry::Polyline {
                closed: k1,
                points: p1,
                bulges: b1,
                ..
            },
            Geometry::Polyline {
                closed: k2,
                points: p2,
                bulges: b2,
                ..
            },
        ) => {
            let flat = |p: &[[f64; 2]]| p.iter().flatten().copied().collect::<Vec<f64>>();
            k1 == k2 && all_close(&flat(p1), &flat(p2)) && all_close(b1, b2)
        }
        _ => a == b,
    }
}

/// Draw every frame that follows something again round it, after any edit: called by the
/// window when it records an undo step and by the command line before it saves. A frame
/// whose shapes were edited by hand, or lost, or that follows nothing any more, is let go
/// of: its shapes stay as they are, and a note says so. Changes nothing for a frame whose
/// texts and shapes didn't change.
pub fn follow_frames(job: Job) -> (Job, Vec<Message>) {
    if job.drawing.frames.is_empty() {
        return (job, Vec::new());
    }
    let mut out = job;
    let mut notes = Vec::new();
    let frames = std::mem::take(&mut out.drawing.frames);
    let mut kept = Vec::new();
    for frame in frames {
        match follow(&mut out, frame) {
            Ok(frame) => kept.push(frame),
            Err(note) => notes.push(note),
        }
    }
    out.drawing.frames = kept;
    (out, notes)
}

/// One frame: drawn again where it now goes, or why it is let go of.
fn follow(job: &mut Job, mut frame: Frame) -> Result<Frame, Message> {
    let mut now = Vec::new();
    for id in frame.shapes() {
        match job.drawing.shapes.iter().find(|s| s.id == id) {
            Some(shape) if shape.is_drawn() => now.push(shape.geometry.clone()),
            _ => {
                return Err(Message::new("pipeline-frame-lost")
                    .arg("id", frame.outline)
                    .arg("shape", id));
            }
        }
    }
    let kind = kind_of(frame.kind);
    let borders = BorderLines::of_record(&frame);
    let edited = || Message::new("pipeline-frame-edited").arg("id", frame.outline);
    let last = frame_geometries(kind, &last_place(&frame), &borders).map_err(|_| edited())?;
    let place = wanted_place(job, &frame)?;
    let fresh = if same_place(&place, &last_place(&frame)) {
        last.clone()
    } else {
        frame_geometries(kind, &place, &borders).map_err(|e| {
            Message::new("pipeline-frame-follows-not")
                .arg("id", frame.outline)
                .arg("why", e.message())
        })?
    };
    let matches = |drawn: &[Geometry]| {
        now.len() == drawn.len() && now.iter().zip(drawn).all(|(a, b)| same_geometry(a, b))
    };
    // As last drawn: draw it where it goes. Already there: moved with what it goes round.
    if !matches(&last) && !matches(&fresh) {
        return Err(edited());
    }
    let ids: Vec<u64> = frame.shapes().collect();
    for (id, geometry) in ids.into_iter().zip(fresh) {
        if let Some(shape) = job.drawing.shapes.iter_mut().find(|s| s.id == id) {
            shape.geometry = geometry;
        }
    }
    frame.center = [place.center.0, place.center.1];
    frame.width = place.width;
    frame.height = place.height;
    Ok(frame)
}

/// New values for a live frame (`chipload frame set`); None keeps a value.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FrameChange {
    pub kind: Option<FrameKind>,
    pub corner: Option<f64>,
    pub margin: Option<f64>,
    pub borders: Option<u32>,
    pub gap: Option<f64>,
    /// The borders' own corner style; Some(None): the outline's shape again.
    pub border_corners: Option<Option<CornerStyle>>,
    pub double: Option<bool>,
    pub border_width: Option<f64>,
    /// A frame drawn to a size: its new width and height.
    pub width: Option<f64>,
    pub height: Option<f64>,
}

/// The live frame with shape `id` (its outline or a border).
fn live_frame(job: &Job, id: u64) -> Result<&Frame, PipelineError> {
    job.drawing
        .frame_of(id)
        .ok_or_else(|| refused(Message::new("pipeline-frame-not-live").arg("id", id)))
}

/// Change a live frame's values and draw it again. Border lines added go on the layer of
/// its first border (or `border`); those taken away are deleted, and operations that cut
/// them are flagged as for any deleted shape.
pub fn set_frame(job: &Job, id: u64, change: &FrameChange) -> Result<Edited, PipelineError> {
    let old = live_frame(job, id)?.clone();
    let mut frame = old.clone();
    let kind = change.kind.unwrap_or(kind_of(old.kind));
    frame.kind = shape_of(kind);
    if let Some(corner) = change.corner {
        frame.corner = corner;
    }
    if let Some(margin) = change.margin {
        frame.margin = margin;
    }
    if let Some(gap) = change.gap {
        frame.gap = gap;
    }
    match change.border_corners {
        Some(corners) => frame.border_corners = corners.map(corners_of),
        // Another kind that isn't rectangular: its borders follow its shape.
        None if kind.corner_style().is_none() => frame.border_corners = None,
        None => {}
    }
    if let Some(double) = change.double {
        frame.double = double;
    }
    if let Some(width) = change.border_width {
        frame.border_width = width;
    }
    if frame.around.is_some() && (change.width.is_some() || change.height.is_some()) {
        return Err(refused(
            Message::new("pipeline-frame-size-follows").arg("id", old.outline),
        ));
    }
    let borders = change.borders.unwrap_or(BorderLines::of_record(&old).count);
    // As a new frame would be, checked the same way.
    let size = match &frame.around {
        Some(around) => FrameSize::Around {
            ids: around_ids(job, around),
            margin: frame.margin,
        },
        None => FrameSize::Fixed {
            width: change.width.unwrap_or(frame.width),
            height: change.height.unwrap_or(frame.height),
            center: Some((frame.center[0], frame.center[1])),
        },
    };
    let new = NewFrame {
        kind,
        r: Some(frame.corner),
        borders,
        gap: frame.gap,
        border_corners: frame.border_corners.map(style_of),
        double: frame.double,
        border_width: frame.border_width,
        size,
        layer: FRAME_LAYER.to_string(),
        border_layer: BORDER_LAYER.to_string(),
    };
    let place = place_frame(job, &new)?;
    let geometries = frame_geometries(kind, &place, &BorderLines::of_new(&new))?;
    let mut out = job.clone();
    let border_layer = old
        .borders
        .first()
        .and_then(|b| out.drawing.shapes.iter().find(|s| s.id == *b))
        .map_or_else(|| BORDER_LAYER.to_string(), |s| s.layer.clone());
    let mut ids: Vec<u64> = old.shapes().collect();
    let mut new_ids = Vec::new();
    // Fewer borders: the innermost go; operations that cut them are flagged by the
    // project's problems, as for any deleted shape.
    while ids.len() > geometries.len() {
        if let Some(gone) = ids.pop() {
            out = crate::shapes::delete_shape(&out, gone)?.0;
        }
    }
    for (i, geometry) in geometries.into_iter().enumerate() {
        if let Some(&id) = ids.get(i) {
            if let Some(shape) = out.drawing.shapes.iter_mut().find(|s| s.id == id) {
                shape.geometry = geometry;
            }
        } else {
            let (next, id) = add_shape(&out, &border_layer, geometry)?;
            out = next;
            ids.push(id);
            new_ids.push(id);
        }
    }
    // New border lines join the frame's group, so a grouped sign stays whole.
    if let Some(group) = out.drawing.group_of(old.outline)
        && let Some(g) = out.drawing.groups.iter_mut().find(|g| g.id == group)
    {
        g.shapes.extend(new_ids.iter().copied());
    }
    frame.outline = ids[0];
    frame.borders = ids[1..].to_vec();
    frame.center = [place.center.0, place.center.1];
    frame.width = place.width;
    frame.height = place.height;
    frame.corner = place.r;
    if let Some(record) = out
        .drawing
        .frames
        .iter_mut()
        .find(|f| f.outline == old.outline)
    {
        *record = frame;
    }
    crate::refuse_problems(&out.drawing.validate())?;
    let mut notes = Vec::new();
    if !new_ids.is_empty() {
        notes.extend(layer_cutters(&out, &border_layer, new_ids.len()));
    }
    Ok(Edited {
        job: out,
        new_ids,
        notes,
        joined: None,
    })
}

/// Let a live frame go on purpose: its shapes stay as they are, and follow nothing.
pub fn detach_frame(job: &Job, id: u64) -> Result<Job, PipelineError> {
    let outline = live_frame(job, id)?.outline;
    let mut out = job.clone();
    out.drawing.frames.retain(|f| f.outline != outline);
    Ok(out)
}

#[cfg(test)]
mod tests;
