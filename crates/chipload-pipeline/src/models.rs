// SPDX-License-Identifier: GPL-3.0-or-later
//! 3D models in a project (docs/specs/stl-relief.md §2–§4): add an STL file, place it on
//! the board, and keep its footprint outline, a closed `path` shape on its layer that names
//! the model, as the model changes. The CLI and the GUI both go through these.

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use chipload_core::primitives::Primitive;
use chipload_i18n::{Digits, Message, Value};
use chipload_job::{Geometry, Job, Model, ModelData, ModelUp, Shape, Units};
use chipload_mesh::{Format, MeshError};
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

/// A model's file read (`read_model`), ready to add: the slow part of adding a model, which
/// the window does on a worker thread (docs/specs/relief-wizard.md §2).
#[derive(Debug, Clone)]
pub struct ReadModel {
    /// The file as the project keeps it, with its mesh already read.
    pub data: ModelData,
    pub sha256: String,
    pub report: chipload_mesh::ReadReport,
    /// The file's size in bytes.
    pub size: usize,
    /// A picture read as a depth map (docs/specs/picture-relief.md): its size in pixels.
    /// None: an STL file.
    pub picture: Option<(u32, u32)>,
}

/// A new picture model's width when none is given, mm.
pub const DEFAULT_PICTURE_WIDTH_MM: f64 = 100.0;
/// A new picture model's depth when none is given, mm.
pub const DEFAULT_PICTURE_DEPTH_MM: f64 = 4.0;

/// Whether `bytes` begin as a picture does (PNG, JPEG, BMP, or an HEIC photo, refused
/// later with advice): a model file that is a picture is a depth map.
fn looks_like_picture(bytes: &[u8]) -> bool {
    bytes.starts_with(b"\x89PNG")
        || bytes.starts_with(&[0xFF, 0xD8, 0xFF])
        || bytes.starts_with(b"BM")
        || bytes.get(4..8) == Some(b"ftyp".as_slice())
}

/// Read a model file's `bytes` for adding: an STL file, or a picture read as a depth map
/// (docs/specs/picture-relief.md §2); checked, parsed once, encoded for the project.
pub fn read_model(bytes: &[u8]) -> Result<ReadModel, PipelineError> {
    let sha256 = format!("{:x}", Sha256::digest(bytes));
    if looks_like_picture(bytes) {
        let picture = chipload_image::decode(bytes)?;
        let pixels = (picture.width, picture.height);
        return Ok(ReadModel {
            data: chipload_toolpath::relief::picture_data(STANDARD.encode(bytes), picture),
            sha256,
            report: chipload_mesh::ReadReport {
                format: Format::Binary,
                triangles: 0,
                dropped: 0,
                open_edges: None,
            },
            size: bytes.len(),
            picture: Some(pixels),
        });
    }
    let (mesh, report) = read(bytes)?;
    Ok(ReadModel {
        data: chipload_toolpath::relief::model_data(STANDARD.encode(bytes), mesh),
        sha256,
        report,
        size: bytes.len(),
        picture: None,
    })
}

/// A model to add: its file name, the unit it is read in, which way is up, and where it
/// goes.
#[derive(Debug, Clone)]
pub struct NewModel<'a> {
    pub file: &'a str,
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
    /// The share of its height kept from the top (docs/specs/relief-wizard.md §5).
    pub keep: f64,
    /// A picture's settings (docs/specs/picture-relief.md §3); None: the defaults. Not
    /// used for an STL.
    pub picture: Option<chipload_job::PictureDepth>,
}

impl<'a> NewModel<'a> {
    /// Read in mm, Z up, its own size, at (0, 0).
    pub fn new(file: &'a str) -> Self {
        NewModel {
            file,
            units: Units::Mm,
            up: ModelUp::Z,
            layer: None,
            x: 0.0,
            y: 0.0,
            width: None,
            rotation: 0.0,
            depth: None,
            keep: 1.0,
            picture: None,
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
    STANDARD.decode(model.data.as_str()).map_err(|e| {
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

/// Model `id`'s light copy as it lies on the board, placed as [`placed_mesh`] places the
/// full one: for showing it, never for cutting (docs/specs/relief-wizard.md §3).
pub fn placed_light_mesh(job: &Job, id: u64) -> Result<Mesh, PipelineError> {
    chipload_toolpath::relief::placed_light_mesh(find(job, id)?).map_err(relief_error)
}

/// What the *Turn and cut…* window shows of a model (docs/specs/relief-wizard.md §4), from
/// its light copy: turned up, turned on the board about its middle, and split at its cut
/// level, in mm, unscaled.
#[derive(Debug, Clone, PartialEq)]
pub struct TurnView {
    /// The part kept, above the cut level.
    pub kept: Mesh,
    /// The part cut away, below it; empty when the model is kept whole.
    pub cut_away: Mesh,
    /// The cut level's Z; the model's lowest Z when it is kept whole.
    pub level: f64,
    /// The whole model's box, lowest and highest corners.
    pub bounds: ([f64; 3], [f64; 3]),
}

/// [`TurnView`] of `model`, which need not be in a project yet: a draft being turned.
pub fn turn_view(model: &Model) -> Result<TurnView, PipelineError> {
    let (light, level) = chipload_toolpath::relief::turned_light(model).map_err(relief_error)?;
    let (lo, hi) = light
        .bounds()
        .ok_or_else(|| PipelineError::Job(Message::new("mesh-error-empty")))?;
    // Turned on the board as it will lie, about the box's middle.
    let (cx, cy) = ((lo[0] + hi[0]) / 2.0, (lo[1] + hi[1]) / 2.0);
    let (sin, cos) = model.rotation.to_radians().sin_cos();
    let turned = light.mapped(|v| {
        let (u, w) = (v[0] - cx, v[1] - cy);
        [cx + u * cos - w * sin, cy + u * sin + w * cos, v[2]]
    });
    let bounds = turned
        .bounds()
        .ok_or_else(|| PipelineError::Job(Message::new("mesh-error-empty")))?;
    Ok(match level {
        Some(level) => TurnView {
            kept: turned.above(level),
            cut_away: turned.below(level),
            level,
            bounds,
        },
        None => TurnView {
            kept: turned,
            cut_away: Mesh::default(),
            level: bounds.0[2],
            bounds,
        },
    })
}

/// The relief `model` gives, seen from above, from its light copy: its top heights over
/// about `along` cells along the longer side (Z from 0 down to −depth), and its size on the
/// board (width, height, depth). Placed by the light copy's own box, so the size is within
/// a light cell of the real one: for the live preview only (docs/specs/relief-wizard.md §4).
pub fn relief_preview(
    model: &Model,
    along: usize,
) -> Result<Option<(TopHeights, [f64; 3])>, PipelineError> {
    let (light, level) = chipload_toolpath::relief::turned_light(model).map_err(relief_error)?;
    let kept = match level {
        Some(level) => light.above(level),
        None => light,
    };
    let Some((lo, hi)) = kept.bounds() else {
        return Ok(None);
    };
    let p = chipload_toolpath::relief::placement_in(model, lo, hi).map_err(relief_error)?;
    let placed = chipload_toolpath::relief::placed(model, &kept, &p);
    Ok(placed
        .top_heights(along)
        .map(|grid| (grid, [p.width, p.height, p.depth])))
}

/// Model `id` as it lies on the board, in the drawing's coordinates, mm: scaled, its
/// footprint's lower-left corner at (x, y), turned about the footprint's centre, its top
/// at Z 0 (the stock top) and its lowest point at −depth (spec §3). What the drop cutter
/// cuts; the placement itself is the toolpath's, so planning and this agree.
pub fn placed_mesh(job: &Job, id: u64) -> Result<Mesh, PipelineError> {
    chipload_toolpath::relief::placed_mesh(find(job, id)?).map_err(relief_error)
}

pub use chipload_mesh::{Mesh, TopHeights};

/// How much steeper a model's shading takes its slopes than they are (`top_shades`).
const SHADE_STEEPEN: f64 = 4.0;

/// The height over the width of `model`'s footprint before it is turned: its box on the
/// canvas (spec §9).
pub fn footprint_aspect(model: &Model) -> Result<f64, PipelineError> {
    let mesh = chipload_toolpath::relief::turned_mesh(model).map_err(relief_error)?;
    let p = chipload_toolpath::relief::placement(model, &mesh).map_err(relief_error)?;
    Ok(p.height / p.width)
}

/// Model `id`'s top surface as placed, about `along` cells along its longer side, Z from
/// the stock top (0) down: what the window shows of it in 3D (spec §9), from its light copy
/// (docs/specs/relief-wizard.md §3).
pub fn top_surface(job: &Job, id: u64, along: usize) -> Result<Option<TopHeights>, PipelineError> {
    Ok(placed_light_mesh(job, id)?.top_heights(along))
}

/// How bright each cell of a model's top looks, 0 to 1, lit low from the upper left: the
/// surface's slope from its neighbours, steepened [`SHADE_STEEPEN`] times so a flat relief
/// still shows its shape (spec §9). Cells with nothing over them are 1.
pub fn top_shades(grid: &TopHeights) -> Vec<f32> {
    let (columns, rows) = (grid.columns, grid.rows);
    let at = |i: usize, j: usize| grid.heights[j * columns + i];
    // The light's direction, toward the light: up and to the upper left.
    let light = {
        let (x, y, z) = (-1.0f64, 1.0, 1.0);
        let n = (x * x + y * y + z * z).sqrt();
        (x / n, y / n, z / n)
    };
    let mut shades = Vec::with_capacity(columns * rows);
    for j in 0..rows {
        for i in 0..columns {
            let Some(h) = at(i, j) else {
                shades.push(1.0);
                continue;
            };
            // The slope by the neighbours that have a surface, this cell where not.
            let side = |a: Option<f64>| a.unwrap_or(h);
            let (left, right) = (
                side(i.checked_sub(1).and_then(|i| at(i, j))),
                side((i + 1 < columns).then(|| at(i + 1, j)).flatten()),
            );
            let (down, up) = (
                side(j.checked_sub(1).and_then(|j| at(i, j))),
                side((j + 1 < rows).then(|| at(i, j + 1)).flatten()),
            );
            let dx = SHADE_STEEPEN * (right - left) / (2.0 * grid.cell);
            let dy = SHADE_STEEPEN * (up - down) / (2.0 * grid.cell);
            let n = (dx * dx + dy * dy + 1.0).sqrt();
            let lit = (-dx * light.0 - dy * light.1 + light.2) / n;
            shades.push(lit.clamp(0.0, 1.0) as f32);
        }
    }
    shades
}

/// Add a model and draw its footprint outline. Returns the project and what was read.
pub fn add_model(
    job: &Job,
    read: &ReadModel,
    new: NewModel,
) -> Result<(Job, ModelReport), PipelineError> {
    let read_report = &read.report;
    let id = job.drawing.models.iter().map(|m| m.id).max().unwrap_or(0) + 1;
    let name = std::path::Path::new(new.file).file_stem().map_or_else(
        || new.file.to_string(),
        |s| s.to_string_lossy().into_owned(),
    );
    let picture = read.picture.is_some();
    let mut model = Model {
        id,
        name: name.clone(),
        file: new.file.to_string(),
        sha256: read.sha256.clone(),
        data: read.data.clone(),
        units: new.units,
        up: new.up,
        layer: new.layer.unwrap_or_else(|| default_layer(job, &name)),
        x: new.x,
        y: new.y,
        width: 1.0,
        rotation: new.rotation,
        depth: new.depth.unwrap_or(1.0),
        proportional: new.depth.is_none(),
        keep: new.keep,
        picture: None,
    };
    if picture {
        // Seen from above, sized by the width given, as deep as asked
        // (docs/specs/picture-relief.md §2).
        model.picture = Some(new.picture.unwrap_or(chipload_job::DEFAULT_PICTURE_DEPTH));
        model.units = Units::Mm;
        model.up = ModelUp::Z;
        model.width = new.width.unwrap_or(DEFAULT_PICTURE_WIDTH_MM);
        model.depth = new.depth.unwrap_or(DEFAULT_PICTURE_DEPTH_MM);
        model.proportional = false;
    } else {
        let (lo, hi) = chipload_toolpath::relief::turned_bounds(&model).map_err(relief_error)?;
        model.width = new.width.unwrap_or(hi[0] - lo[0]);
    }
    let mut out = job.clone();
    out.drawing.models.push(model);
    let (out, mut report) = remake(&out, id)?;
    report.triangles = read_report.triangles;
    report.dropped = read_report.dropped;
    report.open_edges = read_report.open_edges;
    let read_message = match read.picture {
        Some((w, h)) => Message::new("pipeline-model-read-picture")
            .arg("id", id)
            .arg("width", w)
            .arg("height", h),
        None => with_size(
            Message::new(match read_report.format {
                Format::Binary => "pipeline-model-read-binary",
                Format::Text => "pipeline-model-read-text",
            })
            .arg("id", id)
            .arg("count", read_report.triangles)
            .arg("units", new.units.name()),
            report.size,
        ),
    };
    let mut messages = vec![read_message];
    if read_report.dropped > 0 {
        messages.push(Message::new("pipeline-model-dropped").arg("count", read_report.dropped));
    }
    if let Some(open) = read_report.open_edges.filter(|n| *n > 0) {
        messages.push(Message::new("pipeline-model-open").arg("count", open));
    }
    if read.size > LARGE_MODEL_BYTES {
        messages.push(Message::new("pipeline-model-large").arg("bytes", read.size));
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
    // The outline keeps its shape ids, in order, so an operation or group that names them
    // with other shapes (a pocket round the model, the outline an island) still finds them
    // after the model is moved or sized; a new loop gets a new id, and a loop that is gone
    // takes its id with it, which the operation then reports as missing (the user,
    // 2026-11-13).
    let mut old_ids = job
        .drawing
        .shapes
        .iter()
        .filter(|s| s.model() == Some(id))
        .map(|s| s.id)
        .collect::<Vec<_>>()
        .into_iter();
    out.drawing.shapes.retain(|s| s.model() != Some(id));
    let mut ids = Vec::with_capacity(outlines.len());
    for pieces in &outlines {
        let shape_id = match old_ids.next() {
            Some(old) => old,
            None => {
                let new = out.drawing.next_shape_id;
                out.drawing.next_shape_id += 1;
                new
            }
        };
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
    trace_cells(
        (shadow.columns, shadow.rows),
        shadow.cell,
        shadow.origin,
        |column, row| shadow.at(column, row),
    )
}

/// Filled cells of a grid, `size` (columns, rows) of `cell` mm from `origin` (the lower-left
/// corner), row 0 the lowest, traced as a black-and-white picture is: closed outlines, lines
/// and arcs, in mm.
fn trace_cells(
    (columns, rows): (usize, usize),
    cell: f64,
    (ox, oy): (f64, f64),
    filled: impl Fn(usize, usize) -> bool,
) -> Result<Vec<Vec<Primitive>>, PipelineError> {
    // The picture's rows run from the top; the grid's from the bottom.
    let mut grey = Vec::with_capacity(columns * rows);
    for row in (0..rows).rev() {
        for column in 0..columns {
            grey.push(if filled(column, row) { 0.0 } else { 255.0 });
        }
    }
    let picture = chipload_image::Decoded {
        width: u32::try_from(columns).unwrap_or(u32::MAX),
        height: u32::try_from(rows).unwrap_or(u32::MAX),
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
    let width = cell * columns as f64;
    let traced = chipload_image::trace(&picture, &settings, width)?;
    Ok(traced
        .pieces
        .into_iter()
        .map(|loop_| loop_.iter().map(|p| p.translated((ox, oy))).collect())
        .collect())
}

/// The relief preview `grid`'s shadow filled out as an outline rim with `hull` fills it
/// (docs/specs/stl-relief.md §16.7), for the Relief wizard's board picture: which cells lie
/// inside, row by row from the lowest, as `grid.heights`. The shadow itself at `hull` 0.
pub fn fuller_shadow(grid: &TopHeights, hull: f64) -> Result<Vec<bool>, PipelineError> {
    let filled = |column: usize, row: usize| {
        grid.heights
            .get(row * grid.columns + column)
            .is_some_and(Option::is_some)
    };
    let mut cells: Vec<bool> = (0..grid.rows)
        .flat_map(|row| (0..grid.columns).map(move |column| (column, row)))
        .map(|(column, row)| filled(column, row))
        .collect();
    if hull <= 0.0 {
        return Ok(cells);
    }
    let loops: Vec<Vec<chipload_core::Point>> =
        trace_cells((grid.columns, grid.rows), grid.cell, grid.origin, filled)?
            .iter()
            .map(|pieces| {
                let mut points: Vec<chipload_core::Point> = Vec::new();
                for piece in pieces {
                    for p in piece.points() {
                        if points.last() != Some(&p) {
                            points.push(p);
                        }
                    }
                }
                points
            })
            .collect();
    // The outer loops: inside an even number of others.
    let outer: Vec<Vec<chipload_core::Point>> = loops
        .iter()
        .enumerate()
        .filter(|&(i, l)| {
            l.first().is_some_and(|&p| {
                let around = loops
                    .iter()
                    .enumerate()
                    .filter(|&(j, other)| j != i && chipload_toolpath::relief::encloses(other, p))
                    .count();
                around % 2 == 0
            })
        })
        .map(|(_, l)| l.clone())
        .collect();
    let fuller = chipload_toolpath::fuller::fuller(&outer, hull).map_err(PipelineError::from)?;
    let area = chipload_toolpath::rim::Area::Loops(fuller);
    for row in 0..grid.rows {
        let y = grid.origin.1 + (row as f64 + 0.5) * grid.cell;
        for (x0, x1) in area.spans(y) {
            for column in 0..grid.columns {
                let x = grid.origin.0 + (column as f64 + 0.5) * grid.cell;
                if (x0..=x1).contains(&x) {
                    cells[row * grid.columns + column] = true;
                }
            }
        }
    }
    Ok(cells)
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

    /// The corpus's soft grey dome on black, 120 × 80 pixels.
    fn depth_dome() -> Vec<u8> {
        std::fs::read(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../tests/corpus/images/depth_dome.png"),
        )
        .unwrap()
    }

    #[test]
    fn a_picture_is_a_depth_map_its_background_left_out() {
        let read = read_model(&depth_dome()).unwrap();
        assert_eq!(read.picture, Some((120, 80)));
        let mut new = NewModel::new("depth_dome.png");
        new.width = Some(90.0);
        new.depth = Some(5.0);
        new.picture = Some(chipload_job::PictureDepth {
            background: 0.1,
            ..chipload_job::DEFAULT_PICTURE_DEPTH
        });
        let (job, report) = add_model(&crate::empty_project(), &read, new).unwrap();
        let model = &job.drawing.models[0];
        assert!(model.picture.is_some() && !model.proportional);
        assert_eq!(model.up, ModelUp::Z);
        // The dome's ellipse, 100 × 64 of the picture's pixels: 90 mm wide, 57.6 tall.
        let (lo, hi) = placed_mesh(&job, model.id).unwrap().bounds().unwrap();
        assert!((hi[0] - lo[0] - 90.0).abs() < 0.01, "{lo:?} {hi:?}");
        assert!((hi[1] - lo[1] - 57.6).abs() < 1.5, "{lo:?} {hi:?}");
        assert!(hi[2].abs() < 1e-9 && (lo[2] + 5.0).abs() < 1e-9);
        assert_eq!(report.shapes.len(), 1);
        // Smoothing more is a new surface: the remembered one is for the old settings.
        let before = placed_mesh(&job, model.id).unwrap();
        let mut softer = job.clone();
        if let Some(p) = softer.drawing.models[0].picture.as_mut() {
            p.smoothing = 3.0;
        }
        assert_ne!(placed_mesh(&softer, model.id).unwrap(), before);
        // The picture's settings are checked.
        let mut bad = job.clone();
        if let Some(p) = bad.drawing.models[0].picture.as_mut() {
            p.gamma = 9.0;
        }
        assert!(remake(&bad, model.id).is_err());
    }

    #[test]
    fn an_iphone_photo_is_refused_with_advice() {
        let mut heic = vec![0, 0, 0, 24];
        heic.extend_from_slice(b"ftypheic");
        heic.extend_from_slice(&[0; 40]);
        let err = read_model(&heic).unwrap_err().to_string();
        assert!(err.contains("HEIC") && err.contains("PNG"), "{err}");
    }

    #[test]
    fn the_wizards_shadow_fills_out_between_two_blocks() {
        // Two blocks of 10 × 10 cells, 10 cells apart, 1 mm cells.
        let (columns, rows) = (40, 14);
        let heights = (0..rows)
            .flat_map(|row| (0..columns).map(move |c| (c, row)))
            .map(|(c, row)| {
                let block =
                    (2..12).contains(&row) && ((2..12).contains(&c) || (22..32).contains(&c));
                block.then_some(0.0)
            })
            .collect();
        let grid = TopHeights {
            origin: (0.0, 0.0),
            cell: 1.0,
            columns,
            rows,
            heights,
        };
        let at = |cells: &[bool], c: usize, row: usize| cells[row * columns + c];
        let plain = fuller_shadow(&grid, 0.0).unwrap();
        assert!(at(&plain, 5, 5) && !at(&plain, 17, 7));
        let full = fuller_shadow(&grid, 1.0).unwrap();
        assert!(at(&full, 5, 5) && at(&full, 17, 7));
        // Never past the blocks' box.
        assert!(!at(&full, 17, 0) && !at(&full, 36, 7));
    }

    #[test]
    fn a_box_is_placed_scaled_and_outlined() {
        let job = crate::empty_project();
        let bytes = box_stl(40.0, 20.0, 10.0);
        let mut new = NewModel::new("box.stl");
        new.x = 5.0;
        new.y = 7.0;
        new.width = Some(80.0);
        new.depth = Some(4.0);
        let (job, report) = add_model(&job, &read_model(&bytes).unwrap(), new).unwrap();
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
        let mut new = NewModel::new("tall.stl");
        new.up = ModelUp::Y;
        new.units = Units::Inch;
        let (job, report) = add_model(&job, &read_model(&bytes).unwrap(), new).unwrap();
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

    /// A binary STL of `tris`.
    fn stl_of(tris: &[[[f64; 3]; 3]]) -> Vec<u8> {
        let mut out = vec![0u8; 80];
        out.extend((tris.len() as u32).to_le_bytes());
        for t in tris {
            out.extend([0u8; 12]);
            for v in t {
                for k in v {
                    out.extend((*k as f32).to_le_bytes());
                }
            }
            out.extend([0u8; 2]);
        }
        out
    }

    #[test]
    fn a_model_cut_at_half_keeps_its_front_its_outline_and_scales_what_is_kept() {
        // An octahedron 20 wide and 20 tall, widest at its middle: its top half is a
        // pyramid as wide as the whole.
        let (x, mx, y, my, z, mz) = (
            [10.0, 0.0, 0.0],
            [-10.0, 0.0, 0.0],
            [0.0, 10.0, 0.0],
            [0.0, -10.0, 0.0],
            [0.0, 0.0, 10.0],
            [0.0, 0.0, -10.0],
        );
        let tris = [
            [x, y, z],
            [y, mx, z],
            [mx, my, z],
            [my, x, z],
            [y, x, mz],
            [mx, y, mz],
            [my, mx, mz],
            [x, my, mz],
        ];
        let bytes = stl_of(&tris);
        let job = crate::empty_project();
        let mut new = NewModel::new("octa.stl");
        new.width = Some(40.0);
        new.keep = 0.5;
        let (job, report) = add_model(&job, &read_model(&bytes).unwrap(), new).unwrap();
        // The kept half scaled to 40 wide: in proportion, 20 deep (not 40).
        assert_eq!(report.placed, [40.0, 40.0, 20.0]);
        let (lo, hi) = placed_mesh(&job, 1).unwrap().bounds().unwrap();
        assert_eq!((lo, hi), ([0.0, 0.0, -20.0], [40.0, 40.0, 0.0]));
        // The light copy lies as the full mesh does.
        let light = placed_light_mesh(&job, 1).unwrap().bounds().unwrap();
        assert_eq!(light, (lo, hi));
        // The outline is the whole width: the cut went through the widest part.
        let contours = crate::load_drawing(&job).unwrap().contours;
        let x1 = contours[0]
            .points()
            .iter()
            .map(|p| p.0)
            .fold(f64::NEG_INFINITY, f64::max);
        assert!((x1 - 40.0).abs() <= 40.0 / OUTLINE_CELLS, "{x1}");
        // A typed depth stays when the cut changes.
        let mut deeper = job.clone();
        deeper.drawing.models[0].depth = 6.0;
        deeper.drawing.models[0].proportional = false;
        deeper.drawing.models[0].keep = 0.25;
        let (deeper, report) = remake(&deeper, 1).unwrap();
        assert_eq!(report.placed[2], 6.0);
        let (lo, _) = placed_mesh(&deeper, 1).unwrap().bounds().unwrap();
        assert!((lo[2] + 6.0).abs() < 1e-9);
        // Nothing but 0 < keep <= 1.
        let mut wrong = job.clone();
        wrong.drawing.models[0].keep = 0.0;
        assert!(remake(&wrong, 1).is_err());
    }

    #[test]
    fn a_file_that_is_not_stl_is_refused() {
        let err = read_model(b"not a model").unwrap_err();
        assert!(err.to_string().contains("not an STL file"), "{err}");
    }
}
