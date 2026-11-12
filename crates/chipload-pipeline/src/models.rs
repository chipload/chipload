// SPDX-License-Identifier: GPL-3.0-or-later
//! 3D models in a project (docs/specs/stl-relief.md §2–§4): add an STL file, place it on
//! the board, and keep its footprint outline, a closed `path` shape on its layer that names
//! the model, as the model changes. The CLI and the GUI both go through these.

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use chipload_core::primitives::Primitive;
use chipload_i18n::{Digits, Message, Value};
use chipload_job::{Geometry, Job, Model, ModelUp, Shape, Units};
use chipload_mesh::{Format, Mesh, MeshError};
use sha2::{Digest, Sha256};

use crate::PipelineError;

/// The footprint is traced in at most this many cells along its longer side: fine enough
/// for an area to cut, quick to trace.
const OUTLINE_CELLS: f64 = 1000.0;
/// ... and never in cells smaller than this, mm.
const OUTLINE_MIN_CELL_MM: f64 = 0.05;
/// How far the outline may be simplified from the traced shadow, in cells.
const OUTLINE_SMOOTHING_CELLS: f64 = 2.0;
/// The trace's settings: no blur, the edge midway between a covered cell and an empty one.
const OUTLINE_LEVEL: u8 = 128;
/// Over this much STL, adding a model warns that the project gets large (decision 3).
pub const LARGE_MODEL_BYTES: usize = 20 * 1024 * 1024;

impl From<MeshError> for PipelineError {
    fn from(e: MeshError) -> Self {
        PipelineError::Job(e.message())
    }
}

/// A model to add: its file name and bytes, the unit it is read in, which way is up, and
/// where it goes.
#[derive(Debug, Clone)]
pub struct NewModel<'a> {
    pub file: &'a str,
    pub bytes: &'a [u8],
    pub units: Units,
    pub up: ModelUp,
    /// None: named after the file.
    pub layer: Option<String>,
    pub x: f64,
    pub y: f64,
    /// None: its own size in its unit.
    pub width: Option<f64>,
    pub rotation: f64,
    /// None: in proportion to the width.
    pub depth: Option<f64>,
}

impl<'a> NewModel<'a> {
    /// Read in mm, Z up, its own size, at (0, 0).
    pub fn new(file: &'a str, bytes: &'a [u8]) -> Self {
        NewModel {
            file,
            bytes,
            units: Units::Mm,
            up: ModelUp::Z,
            layer: None,
            x: 0.0,
            y: 0.0,
            width: None,
            rotation: 0.0,
            depth: None,
        }
    }
}

/// What reading and placing a model gave.
#[derive(Debug, Clone, PartialEq)]
pub struct ModelReport {
    pub id: u64,
    pub triangles: usize,
    pub dropped: usize,
    pub open_edges: Option<usize>,
    /// The model's own size with its up axis up, in its unit read as mm: x, y, z.
    pub size: [f64; 3],
    /// Its size on the board, mm: width, height, depth.
    pub placed: [f64; 3],
    /// The footprint outline's shape ids.
    pub shapes: Vec<u64>,
    /// The report as messages, for the app; `lines` in English, for the command line.
    pub messages: Vec<Message>,
    pub lines: Vec<String>,
}

/// The STL read, under a guard: a panic inside the reader becomes an error.
fn read(bytes: &[u8]) -> Result<(Mesh, chipload_mesh::ReadReport), PipelineError> {
    match std::panic::catch_unwind(|| chipload_mesh::read_stl(bytes)) {
        Ok(read) => Ok(read?),
        Err(_) => Err(PipelineError::Job(Message::new(
            "pipeline-model-unreadable",
        ))),
    }
}

/// Model `id`.
pub fn find(job: &Job, id: u64) -> Result<&Model, PipelineError> {
    job.drawing
        .models
        .iter()
        .find(|m| m.id == id)
        .ok_or_else(|| PipelineError::Job(Message::new("pipeline-no-model").arg("id", id)))
}

/// Model `id`'s STL file.
pub fn model_bytes(model: &Model) -> Result<Vec<u8>, PipelineError> {
    STANDARD.decode(&model.data).map_err(|e| {
        PipelineError::Job(
            Message::new("pipeline-model-unreadable-data")
                .arg("id", model.id)
                .arg("error", Value::Text(e.to_string())),
        )
    })
}

fn relief_error(e: chipload_toolpath::relief::ReliefError) -> PipelineError {
    PipelineError::Job(e.message())
}

/// The mesh turned up, in mm as its unit reads, with its bounds.
fn turned(model: &Model) -> Result<(Mesh, [f64; 3], [f64; 3]), PipelineError> {
    let mesh = chipload_toolpath::relief::turned_mesh(model).map_err(relief_error)?;
    let (lo, hi) = mesh
        .bounds()
        .ok_or_else(|| PipelineError::Job(Message::new("mesh-error-empty")))?;
    Ok((mesh, lo, hi))
}

/// Model `id` as it lies on the board, in the drawing's coordinates, mm: scaled, its
/// footprint's lower-left corner at (x, y), turned about the footprint's centre, its top
/// at Z 0 (the stock top) and its lowest point at −depth (spec §3). What the drop cutter
/// cuts; the placement itself is the toolpath's, so planning and this agree.
pub fn placed_mesh(job: &Job, id: u64) -> Result<Mesh, PipelineError> {
    chipload_toolpath::relief::placed_mesh(find(job, id)?).map_err(relief_error)
}

/// Add a model and draw its footprint outline. Returns the project and what was read.
pub fn add_model(job: &Job, new: NewModel) -> Result<(Job, ModelReport), PipelineError> {
    let (_, read_report) = read(new.bytes)?;
    let id = job.drawing.models.iter().map(|m| m.id).max().unwrap_or(0) + 1;
    let name = std::path::Path::new(new.file).file_stem().map_or_else(
        || new.file.to_string(),
        |s| s.to_string_lossy().into_owned(),
    );
    let mut model = Model {
        id,
        name: name.clone(),
        file: new.file.to_string(),
        sha256: format!("{:x}", Sha256::digest(new.bytes)),
        data: STANDARD.encode(new.bytes),
        units: new.units,
        up: new.up,
        layer: new.layer.unwrap_or_else(|| default_layer(job, &name)),
        x: new.x,
        y: new.y,
        width: 1.0,
        rotation: new.rotation,
        depth: new.depth.unwrap_or(1.0),
        proportional: new.depth.is_none(),
    };
    let (_, lo, hi) = turned(&model)?;
    model.width = new.width.unwrap_or(hi[0] - lo[0]);
    let mut out = job.clone();
    out.drawing.models.push(model);
    let (out, mut report) = remake(&out, id)?;
    report.triangles = read_report.triangles;
    report.dropped = read_report.dropped;
    report.open_edges = read_report.open_edges;
    let read = Message::new(match read_report.format {
        Format::Binary => "pipeline-model-read-binary",
        Format::Text => "pipeline-model-read-text",
    })
    .arg("id", id)
    .arg("count", read_report.triangles)
    .arg("units", new.units.name());
    let mut messages = vec![with_size(read, report.size)];
    if read_report.dropped > 0 {
        messages.push(Message::new("pipeline-model-dropped").arg("count", read_report.dropped));
    }
    if let Some(open) = read_report.open_edges.filter(|n| *n > 0) {
        messages.push(Message::new("pipeline-model-open").arg("count", open));
    }
    if new.bytes.len() > LARGE_MODEL_BYTES {
        messages.push(Message::new("pipeline-model-large").arg("bytes", new.bytes.len()));
    }
    messages.append(&mut report.messages);
    report.lines = messages.iter().map(Message::english).collect();
    report.messages = messages;
    Ok((out, report))
}

/// `message` with a size's three lengths: `x`, `y`, `z`.
fn with_size(message: Message, size: [f64; 3]) -> Message {
    let length = |v: f64| Value::Length(v, Digits::Fixed(1));
    message
        .arg("x", length(size[0]))
        .arg("y", length(size[1]))
        .arg("z", length(size[2]))
}

/// A layer named after the model, made unique among the drawing's layers.
fn default_layer(job: &Job, name: &str) -> String {
    let base = if name.trim().is_empty() {
        "model"
    } else {
        name.trim()
    };
    let taken = |layer: &str| {
        job.drawing.shapes.iter().any(|s| s.layer == layer)
            || job.drawing.models.iter().any(|m| m.layer == layer)
    };
    if !taken(base) {
        return base.to_string();
    }
    (2..)
        .map(|k| format!("{base} {k}"))
        .find(|l| !taken(l))
        .unwrap_or_else(|| base.to_string())
}

/// Draw model `id`'s footprint outline again from its current values, replacing the old
/// one.
pub fn remake(job: &Job, id: u64) -> Result<(Job, ModelReport), PipelineError> {
    let k = job.drawing.models.iter().position(|m| m.id == id);
    let model = find(job, id)?;
    // A depth in proportion isn't known yet: any positive value passes the check.
    let mut checked = model.clone();
    if checked.proportional {
        checked.depth = 1.0;
    }
    if let Some(problem) = k.and_then(|k| checked.problems(k).into_iter().next()) {
        return Err(PipelineError::Job(problem.text_message));
    }
    let (mesh, lo, hi) = turned(model)?;
    let p = chipload_toolpath::relief::placement(model, &mesh).map_err(relief_error)?;
    let placed = chipload_toolpath::relief::placed(model, &mesh, &p);
    let outlines = footprint(&placed)?;
    let mut out = job.clone();
    // A depth in proportion is made again from the width.
    if let Some(m) = out.drawing.models.iter_mut().find(|m| m.id == id) {
        m.depth = p.depth;
    }
    out.follow_thickness();
    out.drawing.shapes.retain(|s| s.model() != Some(id));
    let mut ids = Vec::with_capacity(outlines.len());
    for pieces in &outlines {
        let shape_id = out.drawing.next_shape_id;
        out.drawing.next_shape_id += 1;
        ids.push(shape_id);
        out.drawing.shapes.push(Shape {
            construction: false,
            id: shape_id,
            layer: model.layer.clone(),
            name: None,
            geometry: Geometry::Path {
                closed: true,
                segments: crate::images::segments_of(pieces),
                origin: Some(chipload_job::Origin::Model(id)),
            },
        });
    }
    let size = [hi[0] - lo[0], hi[1] - lo[1], hi[2] - lo[2]];
    let placed_size = [p.width, p.height, p.depth];
    let messages = vec![with_size(
        Message::new("pipeline-model-placed")
            .arg("id", id)
            .arg("outlines", ids.len()),
        placed_size,
    )];
    Ok((
        out,
        ModelReport {
            id,
            triangles: mesh.triangles.len(),
            dropped: 0,
            open_edges: None,
            size,
            placed: placed_size,
            shapes: ids,
            lines: messages.iter().map(Message::english).collect(),
            messages,
        },
    ))
}

/// The footprint of a placed mesh as closed outlines, lines and arcs: its shadow seen from
/// above, traced as a black-and-white picture is (spec §4).
fn footprint(placed: &Mesh) -> Result<Vec<Vec<Primitive>>, PipelineError> {
    let (lo, hi) = placed
        .bounds()
        .ok_or_else(|| PipelineError::Job(Message::new("mesh-error-empty")))?;
    let longest = (hi[0] - lo[0]).max(hi[1] - lo[1]);
    let cell = (longest / OUTLINE_CELLS).max(OUTLINE_MIN_CELL_MM);
    let max_cells = ((OUTLINE_CELLS + 3.0) * (OUTLINE_CELLS + 3.0)) as usize;
    let shadow = placed
        .shadow(cell, max_cells)
        .ok_or_else(|| PipelineError::Job(Message::new("pipeline-model-no-outline")))?;
    // The picture's rows run from the top; the shadow's from the bottom.
    let mut grey = Vec::with_capacity(shadow.columns * shadow.rows);
    for row in (0..shadow.rows).rev() {
        for column in 0..shadow.columns {
            grey.push(if shadow.at(column, row) { 0.0 } else { 255.0 });
        }
    }
    let picture = chipload_image::Decoded {
        width: u32::try_from(shadow.columns).unwrap_or(u32::MAX),
        height: u32::try_from(shadow.rows).unwrap_or(u32::MAX),
        grey,
    };
    let settings = chipload_image::Settings {
        threshold: chipload_image::Threshold::Level(OUTLINE_LEVEL),
        invert: false,
        blur: 0.0,
        min_area: (4.0 * cell * cell).min(chipload_image::MAX_MIN_AREA_MM2),
        // Within two cells: the grid's stair steps smoothed away, the outline still within
        // a fraction of a millimetre of the shadow.
        smoothing: (OUTLINE_SMOOTHING_CELLS * cell).clamp(
            chipload_image::MIN_SMOOTHING_MM,
            chipload_image::MAX_SMOOTHING_MM,
        ),
        arcs: true,
    };
    let width = shadow.cell * shadow.columns as f64;
    let traced = chipload_image::trace(&picture, &settings, width)?;
    let (ox, oy) = shadow.origin;
    Ok(traced
        .pieces
        .into_iter()
        .map(|loop_| loop_.iter().map(|p| p.translated((ox, oy))).collect())
        .collect())
}

/// Remove model `id` and its outline.
pub fn remove_model(job: &Job, id: u64) -> Result<Job, PipelineError> {
    find(job, id)?;
    let mut out = job.clone();
    out.drawing.models.retain(|m| m.id != id);
    out.drawing.shapes.retain(|s| s.model() != Some(id));
    Ok(out)
}

/// Models whose values differ from `before`'s: their outlines drawn again (after `set`).
pub fn remake_changed(before: &Job, after: &Job) -> Result<(Job, Vec<ModelReport>), PipelineError> {
    let mut out = after.clone();
    let mut reports = Vec::new();
    for model in &after.drawing.models {
        let was = before.drawing.models.iter().find(|m| m.id == model.id);
        if was != Some(model) {
            let (next, report) = remake(&out, model.id)?;
            out = next;
            reports.push(report);
        }
    }
    Ok((out, reports))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A binary STL of a box `x` × `y` × `z` with its corner at the origin.
    pub(crate) fn box_stl(x: f64, y: f64, z: f64) -> Vec<u8> {
        let c = |i: usize| -> [f64; 3] {
            [
                if i & 1 == 1 { x } else { 0.0 },
                if i & 2 == 2 { y } else { 0.0 },
                if i & 4 == 4 { z } else { 0.0 },
            ]
        };
        let faces = [
            [0, 2, 3, 1],
            [4, 5, 7, 6],
            [0, 1, 5, 4],
            [2, 6, 7, 3],
            [0, 4, 6, 2],
            [1, 3, 7, 5],
        ];
        let mut tris = Vec::new();
        for f in faces {
            tris.push([c(f[0]), c(f[1]), c(f[2])]);
            tris.push([c(f[0]), c(f[2]), c(f[3])]);
        }
        let mut out = vec![0u8; 80];
        out.extend((tris.len() as u32).to_le_bytes());
        for t in tris {
            out.extend([0u8; 12]);
            for v in t {
                for k in v {
                    out.extend((k as f32).to_le_bytes());
                }
            }
            out.extend([0u8; 2]);
        }
        out
    }

    #[test]
    fn a_box_is_placed_scaled_and_outlined() {
        let job = crate::empty_project();
        let bytes = box_stl(40.0, 20.0, 10.0);
        let mut new = NewModel::new("box.stl", &bytes);
        new.x = 5.0;
        new.y = 7.0;
        new.width = Some(80.0);
        new.depth = Some(4.0);
        let (job, report) = add_model(&job, new).unwrap();
        assert_eq!(report.size, [40.0, 20.0, 10.0]);
        assert_eq!(report.placed, [80.0, 40.0, 4.0]);
        assert_eq!(report.shapes.len(), 1, "{report:?}");
        let mesh = placed_mesh(&job, 1).unwrap();
        let (lo, hi) = mesh.bounds().unwrap();
        assert_eq!((lo, hi), ([5.0, 7.0, -4.0], [85.0, 47.0, 0.0]));
        // The outline follows the box within a cell.
        let shape = &job.drawing.shapes[0];
        assert_eq!(shape.model(), Some(1));
        assert_eq!(shape.layer, "box");
        let contours = crate::load_drawing(&job).unwrap().contours;
        let points = contours[0].points();
        let x0 = points.iter().map(|p| p.0).fold(f64::INFINITY, f64::min);
        let y0 = points.iter().map(|p| p.1).fold(f64::INFINITY, f64::min);
        let x1 = points.iter().map(|p| p.0).fold(f64::NEG_INFINITY, f64::max);
        let y1 = points.iter().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max);
        let cell = 80.0 / OUTLINE_CELLS;
        for (got, want) in [(x0, 5.0), (y0, 7.0), (x1, 85.0), (y1, 47.0)] {
            assert!((got - want).abs() <= cell, "{got} {want}");
        }
        // Changed and drawn again: the old outline goes.
        let mut moved = job.clone();
        moved.drawing.models[0].rotation = 90.0;
        let (moved, reports) = remake_changed(&job, &moved).unwrap();
        assert_eq!(reports.len(), 1);
        assert_eq!(
            moved
                .drawing
                .shapes
                .iter()
                .filter(|s| s.model() == Some(1))
                .count(),
            1
        );
        let removed = remove_model(&moved, 1).unwrap();
        assert!(removed.drawing.models.is_empty() && removed.drawing.shapes.is_empty());
    }

    #[test]
    fn y_up_and_inches_and_proportional_depth() {
        let job = crate::empty_project();
        let bytes = box_stl(2.0, 1.0, 3.0);
        let mut new = NewModel::new("tall.stl", &bytes);
        new.up = ModelUp::Y;
        new.units = Units::Inch;
        let (job, report) = add_model(&job, new).unwrap();
        // Y up: the 1 inch side stands up, the 3 inch side lies along −Y.
        let near = |a: [f64; 3], b: [f64; 3]| a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-9);
        assert!(near(report.size, [50.8, 76.2, 25.4]), "{:?}", report.size);
        assert!(
            near(report.placed, [50.8, 76.2, 25.4]),
            "{:?}",
            report.placed
        );
        let (lo, hi) = placed_mesh(&job, 1).unwrap().bounds().unwrap();
        assert!((hi[2] - 0.0).abs() < 1e-9 && (lo[2] + 25.4).abs() < 1e-9);
    }

    #[test]
    fn a_file_that_is_not_stl_is_refused() {
        let job = crate::empty_project();
        let err = add_model(&job, NewModel::new("x.stl", b"not a model")).unwrap_err();
        assert!(err.to_string().contains("not an STL file"), "{err}");
    }
}
