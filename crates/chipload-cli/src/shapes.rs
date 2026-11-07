// SPDX-License-Identifier: GPL-3.0-or-later
//! `chipload shape …`: drawn shapes added, changed, moved, edited (trim, offset, fillet, arrays,
//! …) and named.

use super::*;

#[derive(Subcommand)]
pub(crate) enum ShapeAction {
    /// The shape library: save, insert, list (docs/specs/shape-library.md §7).
    #[command(flatten)]
    Library(shape_library::LibraryCommand),
    /// Add a drawn shape; prints its id.
    Add {
        /// The project (.chipload).
        project: PathBuf,
        kind: KindArg,
        #[command(flatten)]
        values: ShapeValues,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Change a drawn shape's values or layer; it keeps its id.
    Set {
        /// The project (.chipload).
        project: PathBuf,
        /// The shape id (as `inspect` lists it, without the S).
        id: u64,
        #[command(flatten)]
        values: ShapeValues,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Remove a drawn shape. Operations that used it are flagged.
    #[command(alias = "delete")]
    Remove {
        /// The project (.chipload).
        project: PathBuf,
        id: u64,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Copy drawn shapes, moved by --dx and --dy, on their layers; prints the new ids.
    Copy {
        /// The project (.chipload).
        project: PathBuf,
        /// The shape ids (as `inspect` lists them, without the S).
        #[arg(required = true)]
        ids: Vec<u64>,
        /// How far to move the copies right, mm.
        #[arg(value_parser = crate::lengths::length, long, allow_hyphen_values = true)]
        dx: f64,
        /// How far to move the copies up, mm.
        #[arg(value_parser = crate::lengths::length, long, allow_hyphen_values = true)]
        dy: f64,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Mirror drawn shapes in place, about the middle of the box round them. They keep
    /// their ids, so the operations that cut them follow.
    Mirror {
        /// The project (.chipload).
        project: PathBuf,
        /// The shape ids (as `inspect` lists them, without the S).
        #[arg(required = true)]
        ids: Vec<u64>,
        /// x: flip the X coordinates (left becomes right); y: flip Y (up becomes down).
        #[arg(long)]
        axis: AxisArg,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Move drawn shapes by --dx and --dy; they keep their ids, so the operations that cut
    /// them follow (docs/specs/drawn-shapes-box.md).
    Move {
        /// The project (.chipload).
        project: PathBuf,
        /// The shape ids (as `inspect` lists them, without the S).
        #[arg(required = true)]
        ids: Vec<u64>,
        /// How far to move them right, mm.
        #[arg(value_parser = crate::lengths::length, long, allow_hyphen_values = true)]
        dx: f64,
        /// How far to move them up, mm.
        #[arg(value_parser = crate::lengths::length, long, allow_hyphen_values = true)]
        dy: f64,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Turn drawn shapes counter-clockwise about the middle of the box round them, or a
    /// point.
    Turn {
        /// The project (.chipload).
        project: PathBuf,
        /// The shape ids (as `inspect` lists them, without the S).
        #[arg(required = true)]
        ids: Vec<u64>,
        /// The angle, degrees, counter-clockwise; negative turns clockwise.
        #[arg(long, allow_hyphen_values = true)]
        angle_deg: f64,
        /// Turn about this point "X,Y" instead.
        #[arg(long, value_parser = crate::lengths::point, allow_hyphen_values = true)]
        about: Option<[f64; 2]>,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Scale drawn shapes evenly about the middle of the box round them, or a point.
    Scale {
        /// The project (.chipload).
        project: PathBuf,
        /// The shape ids (as `inspect` lists them, without the S).
        #[arg(required = true)]
        ids: Vec<u64>,
        /// The factor, greater than 0: 2 doubles them.
        #[arg(long)]
        factor: f64,
        /// Scale about this point "X,Y" instead.
        #[arg(long, value_parser = crate::lengths::point, allow_hyphen_values = true)]
        about: Option<[f64; 2]>,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Add the curve at a distance from drawn shapes, on their layers; the shapes stay
    /// (docs/specs/drawing-editing.md §2). Prints the new ids.
    Offset {
        /// The project (.chipload).
        project: PathBuf,
        /// The shape ids (as `inspect` lists them, without the S).
        #[arg(required = true)]
        ids: Vec<u64>,
        /// How far, mm (or with a unit).
        #[arg(value_parser = crate::lengths::length, long)]
        distance: f64,
        /// Which side: outside or inside of a closed shape, left or right of an open one
        /// (default: outside, or left).
        #[arg(long, value_enum)]
        side: Option<OffsetSideArg>,
        /// Round corners (the exact offset) or sharp ones (the sides extended to meet).
        #[arg(long, value_enum, default_value = "round")]
        corners: CornersArg,
        /// How many, each the distance further than the last.
        #[arg(long, default_value_t = 1)]
        count: u32,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Round corners of a drawn shape with arcs of a radius; it keeps its id
    /// (docs/specs/drawing-editing.md §3).
    Fillet {
        /// The project (.chipload).
        project: PathBuf,
        /// The shape id (as `inspect` lists it, without the S).
        id: u64,
        /// The radius, mm (or with a unit).
        #[arg(value_parser = crate::lengths::length, long)]
        radius: f64,
        /// Only this corner: its point's number, from 0 (default: every corner).
        #[arg(long)]
        corner: Option<usize>,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Trim a drawn shape: the piece of it at --at, between the nearest crossings with the
    /// shapes shown, goes; the rest keeps the id, a second piece is a new shape
    /// (docs/specs/drawing-editing.md §4).
    Trim {
        /// The project (.chipload).
        project: PathBuf,
        /// The shape id (as `inspect` lists it, without the S).
        id: u64,
        /// A point on (or near) the piece to remove, "X,Y".
        #[arg(long, value_parser = crate::lengths::point, allow_hyphen_values = true)]
        at: [f64; 2],
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Extend an open drawn shape's straight end along itself to the nearest shape shown
    /// ahead (docs/specs/drawing-editing.md §4).
    Extend {
        /// The project (.chipload).
        project: PathBuf,
        /// The shape id (as `inspect` lists it, without the S).
        id: u64,
        /// Which end.
        #[arg(long, value_enum)]
        end: EndArg,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Break a drawn shape at the point of it nearest --at: an open one becomes two, a
    /// closed one opens there (docs/specs/drawing-editing.md §4).
    Break {
        /// The project (.chipload).
        project: PathBuf,
        /// The shape id (as `inspect` lists it, without the S).
        id: u64,
        /// The point, "X,Y".
        #[arg(long, value_parser = crate::lengths::point, allow_hyphen_values = true)]
        at: [f64; 2],
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Join closed drawn shapes into one: everything any of them covers. The result takes
    /// the first shape's id; the others go (docs/specs/drawing-editing.md §5).
    Union {
        /// The project (.chipload).
        project: PathBuf,
        /// The shape ids, two or more (as `inspect` lists them, without the S).
        #[arg(required = true, num_args = 2..)]
        ids: Vec<u64>,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Cut closed drawn shapes out of another: BASE keeps its id, the cutters go
    /// (docs/specs/drawing-editing.md §5).
    Subtract {
        /// The project (.chipload).
        project: PathBuf,
        /// The shape cut (as `inspect` lists it, without the S).
        base: u64,
        /// The shapes cut out of it.
        #[arg(required = true)]
        ids: Vec<u64>,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// What closed drawn shapes have in common: the result takes the first shape's id; the
    /// others go (docs/specs/drawing-editing.md §5).
    Intersect {
        /// The project (.chipload).
        project: PathBuf,
        /// The shape ids, two or more (as `inspect` lists them, without the S).
        #[arg(required = true, num_args = 2..)]
        ids: Vec<u64>,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Copies of drawn shapes in a grid, on their layers; the originals stay
    /// (docs/specs/drawing-editing.md §6). Prints the new ids.
    Array {
        /// The project (.chipload).
        project: PathBuf,
        /// The shape ids (as `inspect` lists them, without the S).
        #[arg(required = true)]
        ids: Vec<u64>,
        /// Columns, the originals' included.
        #[arg(long, default_value_t = 1)]
        columns: u32,
        /// Rows, the originals' included.
        #[arg(long, default_value_t = 1)]
        rows: u32,
        /// From one column to the next, mm (or with a unit); negative goes left.
        #[arg(value_parser = crate::lengths::length, long, default_value = "0", allow_hyphen_values = true)]
        dx: f64,
        /// From one row to the next, mm (or with a unit); negative goes down.
        #[arg(value_parser = crate::lengths::length, long, default_value = "0", allow_hyphen_values = true)]
        dy: f64,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Copies of drawn shapes round a centre, on their layers; the originals stay
    /// (docs/specs/drawing-editing.md §6). Prints the new ids.
    ArrayPolar {
        /// The project (.chipload).
        project: PathBuf,
        /// The shape ids (as `inspect` lists them, without the S).
        #[arg(required = true)]
        ids: Vec<u64>,
        /// How many, the originals included.
        #[arg(long)]
        count: u32,
        /// The centre, "X,Y".
        #[arg(long, value_parser = crate::lengths::point, allow_hyphen_values = true)]
        center: [f64; 2],
        /// The sweep, degrees: 360 spreads them evenly round, less puts the first and last
        /// at its ends; negative goes clockwise.
        #[arg(long, default_value_t = 360.0, allow_hyphen_values = true)]
        angle_deg: f64,
        /// Move the copies round without turning them.
        #[arg(long)]
        no_rotate: bool,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Line up drawn shapes: each one's box side or middle with that of the box round
    /// them all, or the stock's; they move whole and keep their ids. The shapes of one
    /// contour, and a group named whole, line up as one (docs/specs/drawing-editing.md §7,
    /// arrange-whole.md).
    Align {
        /// The project (.chipload).
        project: PathBuf,
        /// The shape ids (as `inspect` lists them, without the S).
        #[arg(required = true)]
        ids: Vec<u64>,
        /// Which side or middle lines up.
        #[arg(long, value_enum)]
        to: AlignToArg,
        /// Line up with the stock instead (then one shape is enough).
        #[arg(long)]
        stock: bool,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Even out the gaps between drawn shapes' boxes, the first and last staying put; the
    /// shapes of one contour, and a group named whole, count as one
    /// (docs/specs/drawing-editing.md §7, arrange-whole.md).
    Distribute {
        /// The project (.chipload).
        project: PathBuf,
        /// The shape ids, three or more (as `inspect` lists them, without the S).
        #[arg(required = true, num_args = 3..)]
        ids: Vec<u64>,
        /// Along X.
        #[arg(
            long,
            conflicts_with = "vertical",
            required_unless_present = "vertical"
        )]
        horizontal: bool,
        /// Along Y.
        #[arg(long)]
        vertical: bool,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Cut corners of a drawn shape straight across, a distance back along each side; it
    /// keeps its id (docs/specs/drawing-editing.md §3).
    Chamfer {
        /// The project (.chipload).
        project: PathBuf,
        /// The shape id (as `inspect` lists it, without the S).
        id: u64,
        /// How far back along each side, mm (or with a unit).
        #[arg(value_parser = crate::lengths::length, long)]
        distance: f64,
        /// Only this corner: its point's number, from 0 (default: every corner).
        #[arg(long)]
        corner: Option<usize>,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Join open drawn polylines whose ends are closer than --gap into one, on the same
    /// layer; a chain that comes back to its start closes (docs/specs/explode-and-join.md).
    Join {
        /// The project (.chipload).
        project: PathBuf,
        /// The shape ids (as `inspect` lists them, without the S); none: every drawn shape.
        ids: Vec<u64>,
        /// Join ends closer than this, 0.001 to 1 mm.
        #[arg(long, value_parser = crate::lengths::length, default_value = "0.01")]
        gap: f64,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

#[derive(Clone, Copy, ValueEnum)]
pub(crate) enum AxisArg {
    X,
    Y,
}

#[derive(Clone, Copy, ValueEnum)]
pub(crate) enum OffsetSideArg {
    Outside,
    Inside,
    Left,
    Right,
}

#[derive(Clone, Copy, ValueEnum)]
pub(crate) enum AlignToArg {
    Left,
    #[value(alias = "center")]
    Centre,
    Right,
    Top,
    Middle,
    Bottom,
}

#[derive(Clone, Copy, ValueEnum)]
pub(crate) enum EndArg {
    Start,
    End,
}

#[derive(Clone, Copy, ValueEnum)]
pub(crate) enum CornersArg {
    Round,
    Sharp,
}

#[derive(Clone, Copy, ValueEnum)]
pub(crate) enum KindArg {
    Circle,
    Rectangle,
    Polyline,
    Polygon,
}

/// Defining values, all in mm; the angle in degrees.
#[derive(clap::Args)]
pub(crate) struct ShapeValues {
    /// Centre "X,Y" (circle, rectangle, polygon).
    #[arg(long, allow_hyphen_values = true)]
    center: Option<String>,
    /// A circle's radius; a polygon's, to its corners.
    #[arg(value_parser = crate::lengths::length, long)]
    radius: Option<f64>,
    /// A polygon's number of sides, 3 to 1000.
    #[arg(long)]
    sides: Option<u32>,
    /// A polygon's size across its flats (twice the radius to the middle of a side), in
    /// place of --radius.
    #[arg(value_parser = crate::lengths::length, long, conflicts_with = "radius")]
    across_flats: Option<f64>,
    /// Along the rectangle's own X axis.
    #[arg(value_parser = crate::lengths::length, long)]
    length: Option<f64>,
    /// Along the rectangle's own Y axis.
    #[arg(value_parser = crate::lengths::length, long)]
    width: Option<f64>,
    /// Rotation about the centre (a polygon: its first corner's angle from +X), degrees,
    /// counter-clockwise.
    #[arg(long, allow_hyphen_values = true)]
    angle_deg: Option<f64>,
    /// The radius of the rectangle's corners, at most half its shorter side (default 0:
    /// sharp).
    #[arg(value_parser = crate::lengths::length, long)]
    corner_radius: Option<f64>,
    /// Polyline points "X,Y X,Y ...".
    #[arg(long, allow_hyphen_values = true)]
    points: Option<String>,
    /// Polyline bulges "B B ...", one per segment: tan(sweep / 4) as in DXF, 0 straight,
    /// 1 a counter-clockwise half circle, -1 clockwise (default: all straight).
    #[arg(long, allow_hyphen_values = true)]
    bulges: Option<String>,
    /// The polyline doesn't return to its first point.
    #[arg(long, conflicts_with = "closed")]
    open: bool,
    /// The polyline returns to its first point (the default for new ones).
    #[arg(long)]
    closed: bool,
    #[arg(long)]
    layer: Option<String>,
    /// Its name in the Browser; "" takes the name away.
    #[arg(long)]
    name: Option<String>,
    /// Construction geometry: shown and snapped to, never cut (`--construction false`
    /// makes it an ordinary shape again).
    #[arg(long, num_args = 0..=1, default_missing_value = "true")]
    construction: Option<bool>,
}

/// "S2, S3".
pub(crate) fn shape_names(ids: &[u64]) -> String {
    ids.iter()
        .map(|id| format!("S{id}"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// `shape union`, `subtract` or `intersect`: the project and what to print; the new shapes'
/// ids and the number of operations flagged (they used a shape that is gone).
pub(crate) fn combined(
    job: &Job,
    op: Combine,
    ids: &[u64],
    new_ids: &mut Option<Vec<u64>>,
    flagged: &mut usize,
) -> Result<(Job, Vec<String>), PipelineError> {
    let edited = combine_shapes(job, op, ids)?;
    let gone: Vec<u64> = ids
        .iter()
        .copied()
        .filter(|id| !edited.job.drawing.shapes.iter().any(|s| s.id == *id))
        .collect();
    let mut lines = Vec::new();
    if !gone.is_empty() {
        lines.push(format!("Removed shapes {}.", shape_names(&gone)));
    }
    if !edited.new_ids.is_empty() {
        lines.push(format!("Added shapes {}.", shape_names(&edited.new_ids)));
    }
    *flagged = job
        .operations
        .iter()
        .filter(|op| {
            op.select()
                .shapes
                .as_ref()
                .is_some_and(|s| s.iter().any(|id| gone.contains(id)))
        })
        .count();
    lines.extend(edited.notes.iter().map(Message::english));
    *new_ids = Some(edited.new_ids);
    Ok((edited.job, lines))
}

/// `about`, or the middle of the box round shapes `ids`.
pub(crate) fn about_point(
    job: &Job,
    ids: &[u64],
    about: Option<[f64; 2]>,
) -> Result<(f64, f64), PipelineError> {
    match about {
        Some([x, y]) => Ok((x, y)),
        None => drawn_middle(job, ids),
    }
}

pub(crate) fn shape(out: &mut Out, action: ShapeAction) -> ExitCode {
    let action = match action {
        ShapeAction::Library(command) => return shape_library::run(out, command),
        other => other,
    };
    let (project, output) = match &action {
        // Answered above.
        ShapeAction::Library(_) => return ExitCode::FAILURE,
        ShapeAction::Add {
            project, output, ..
        }
        | ShapeAction::Set {
            project, output, ..
        }
        | ShapeAction::Remove {
            project, output, ..
        }
        | ShapeAction::Copy {
            project, output, ..
        }
        | ShapeAction::Mirror {
            project, output, ..
        }
        | ShapeAction::Join {
            project, output, ..
        }
        | ShapeAction::Offset {
            project, output, ..
        }
        | ShapeAction::Fillet {
            project, output, ..
        }
        | ShapeAction::Chamfer {
            project, output, ..
        }
        | ShapeAction::Trim {
            project, output, ..
        }
        | ShapeAction::Extend {
            project, output, ..
        }
        | ShapeAction::Break {
            project, output, ..
        }
        | ShapeAction::Union {
            project, output, ..
        }
        | ShapeAction::Align {
            project, output, ..
        }
        | ShapeAction::Distribute {
            project, output, ..
        }
        | ShapeAction::Array {
            project, output, ..
        }
        | ShapeAction::ArrayPolar {
            project, output, ..
        }
        | ShapeAction::Subtract {
            project, output, ..
        }
        | ShapeAction::Intersect {
            project, output, ..
        }
        | ShapeAction::Move {
            project, output, ..
        }
        | ShapeAction::Turn {
            project, output, ..
        }
        | ShapeAction::Scale {
            project, output, ..
        } => (project.clone(), output.clone()),
    };
    let job = match load(out, &project) {
        Ok(job) => job,
        Err(code) => return code,
    };
    let mut flagged_ops = 0;
    // The new shapes' ids (`shape add`, `shape copy`).
    let mut new_ids: Option<Vec<u64>> = None;
    // What `shape join` joined and removed.
    let mut joined: Option<(Vec<u64>, Vec<u64>)> = None;
    let changed = match action {
        ShapeAction::Library(_) => return ExitCode::FAILURE,
        ShapeAction::Add { kind, values, .. } => new_geometry(kind, &values)
            .map_err(|e| PipelineError::Job(foreign_text(e)))
            .and_then(|geometry| {
                let layer = values.layer.as_deref().unwrap_or("0");
                let (job, id) = add_shape(&job, layer, geometry)?;
                let job = match &values.name {
                    Some(name) => name_shape(&job, id, name)?,
                    None => job,
                };
                match values.construction {
                    Some(true) => set_construction(&job, &[id], true).map(|(job, _)| (job, id)),
                    _ => Ok((job, id)),
                }
            })
            .map(|(job, id)| {
                new_ids = Some(vec![id]);
                (job, vec![format!("Added shape S{id}.")])
            }),
        ShapeAction::Set { id, values, .. } => job
            .drawing
            .shapes
            .iter()
            .find(|s| s.id == id)
            .ok_or_else(|| chipload_pipeline::shapes::no_shape(id))
            .and_then(|shape| {
                let geometry = changed_geometry(&shape.geometry, &values)
                    .map_err(|e| PipelineError::Job(foreign_text(e)))?;
                let layer = values.layer.clone().unwrap_or_else(|| shape.layer.clone());
                let job = set_shape(&job, id, &layer, geometry)?;
                match &values.name {
                    Some(name) => name_shape(&job, id, name),
                    None => Ok(job),
                }
            })
            .and_then(|job| match values.construction {
                Some(on) => set_construction(&job, &[id], on),
                None => Ok((job, Vec::new())),
            })
            .map(|(job, notes)| {
                flagged_ops = notes.len();
                let mut lines = vec![format!("Changed shape S{id}.")];
                lines.extend(notes.iter().map(|n| format!("warning: {}", n.english())));
                (job, lines)
            }),
        ShapeAction::Remove { id, .. } => delete_shape(&job, id).map(|(job, flagged)| {
            flagged_ops = flagged.len();
            let mut lines = vec![format!("Deleted shape S{id}.")];
            lines.extend(flagged.into_iter().map(|l| format!("warning: {l}")));
            (job, lines)
        }),
        ShapeAction::Copy { ids, dx, dy, .. } => {
            copy_shapes(&job, &ids, (dx, dy)).map(|(job, new)| {
                let names: Vec<String> = new.iter().map(|id| format!("S{id}")).collect();
                new_ids = Some(new.clone());
                (job, vec![format!("Added shapes {}.", names.join(", "))])
            })
        }
        ShapeAction::Mirror { ids, axis, .. } => {
            let mirror = match axis {
                AxisArg::X => Mirror::LeftRight,
                AxisArg::Y => Mirror::UpDown,
            };
            mirror_shapes(&job, &ids, mirror).map(|job| {
                let names: Vec<String> = ids.iter().map(|id| format!("S{id}")).collect();
                (job, vec![format!("Mirrored shapes {}.", names.join(", "))])
            })
        }
        ShapeAction::Offset {
            ids,
            distance,
            side,
            corners,
            count,
            ..
        } => {
            let side = side.map(|s| match s {
                OffsetSideArg::Outside => OffsetSide::Outside,
                OffsetSideArg::Inside => OffsetSide::Inside,
                OffsetSideArg::Left => OffsetSide::Left,
                OffsetSideArg::Right => OffsetSide::Right,
            });
            let corners = match corners {
                CornersArg::Round => Corners::Round,
                CornersArg::Sharp => Corners::Sharp,
            };
            offset_shapes(&job, &ids, distance, side, corners, count).map(|edited| {
                new_ids = Some(edited.new_ids.clone());
                let mut lines = vec![format!("Added shapes {}.", shape_names(&edited.new_ids))];
                lines.extend(edited.notes.iter().map(Message::english));
                (edited.job, lines)
            })
        }
        ShapeAction::Fillet {
            id, radius, corner, ..
        } => cut_shape_corners(&job, id, corner, Cut::Fillet(radius))
            .map(|job| (job, vec![format!("Rounded the corners of S{id}.")])),
        ShapeAction::Chamfer {
            id,
            distance,
            corner,
            ..
        } => cut_shape_corners(&job, id, corner, Cut::Chamfer(distance))
            .map(|job| (job, vec![format!("Cut the corners of S{id}.")])),
        ShapeAction::Trim { id, at, .. } => trim_shape(&job, id, (at[0], at[1])).map(|edited| {
            new_ids = Some(edited.new_ids.clone());
            let mut lines = vec![format!("Trimmed S{id}.")];
            if !edited.new_ids.is_empty() {
                lines.push(format!("Added shapes {}.", shape_names(&edited.new_ids)));
            }
            lines.extend(edited.notes.iter().map(Message::english));
            (edited.job, lines)
        }),
        ShapeAction::Extend { id, end, .. } => {
            let end = match end {
                EndArg::Start => End::Start,
                EndArg::End => End::End,
            };
            extend_shape(&job, id, end).map(|edited| (edited.job, vec![format!("Extended S{id}.")]))
        }
        ShapeAction::Break { id, at, .. } => break_shape(&job, id, (at[0], at[1])).map(|edited| {
            new_ids = Some(edited.new_ids.clone());
            let mut lines = vec![format!("Broke S{id}.")];
            if !edited.new_ids.is_empty() {
                lines.push(format!("Added shapes {}.", shape_names(&edited.new_ids)));
            }
            lines.extend(edited.notes.iter().map(Message::english));
            (edited.job, lines)
        }),
        ShapeAction::Array {
            ids,
            columns,
            rows,
            dx,
            dy,
            ..
        } => array_grid(&job, &ids, columns, rows, (dx, dy)).map(|edited| {
            new_ids = Some(edited.new_ids.clone());
            let mut lines = vec![format!("Added shapes {}.", shape_names(&edited.new_ids))];
            lines.extend(edited.notes.iter().map(Message::english));
            (edited.job, lines)
        }),
        ShapeAction::ArrayPolar {
            ids,
            count,
            center,
            angle_deg,
            no_rotate,
            ..
        } => {
            let polar = Polar {
                count,
                center: (center[0], center[1]),
                angle_deg,
                rotate: !no_rotate,
            };
            array_polar(&job, &ids, polar).map(|edited| {
                new_ids = Some(edited.new_ids.clone());
                let mut lines = vec![format!("Added shapes {}.", shape_names(&edited.new_ids))];
                lines.extend(edited.notes.iter().map(Message::english));
                (edited.job, lines)
            })
        }
        ShapeAction::Align { ids, to, stock, .. } => {
            let to = match to {
                AlignToArg::Left => AlignTo::Left,
                AlignToArg::Centre => AlignTo::Centre,
                AlignToArg::Right => AlignTo::Right,
                AlignToArg::Top => AlignTo::Top,
                AlignToArg::Middle => AlignTo::Middle,
                AlignToArg::Bottom => AlignTo::Bottom,
            };
            align_shapes(&job, &ids, to, stock)
                .map(|job| (job, vec![format!("Aligned shapes {}.", shape_names(&ids))]))
        }
        ShapeAction::Distribute {
            ids, horizontal, ..
        } => distribute_shapes(&job, &ids, horizontal).map(|job| {
            (
                job,
                vec![format!("Distributed shapes {}.", shape_names(&ids))],
            )
        }),
        ShapeAction::Union { ids, .. } => {
            combined(&job, Combine::Union, &ids, &mut new_ids, &mut flagged_ops)
        }
        ShapeAction::Subtract { base, ids, .. } => {
            let all: Vec<u64> = std::iter::once(base).chain(ids).collect();
            combined(
                &job,
                Combine::Subtract,
                &all,
                &mut new_ids,
                &mut flagged_ops,
            )
        }
        ShapeAction::Intersect { ids, .. } => combined(
            &job,
            Combine::Intersect,
            &ids,
            &mut new_ids,
            &mut flagged_ops,
        ),
        ShapeAction::Move { ids, dx, dy, .. } => {
            transform_shapes(&job, &ids, (0.0, 0.0), (dx, dy), 1.0, 0.0).map(|job| {
                let text = format!("Moved shapes {}.", shape_names(&ids));
                (job, vec![text])
            })
        }
        ShapeAction::Turn {
            ids,
            angle_deg,
            about,
            ..
        } => about_point(&job, &ids, about).and_then(|c| {
            transform_shapes(&job, &ids, c, c, 1.0, angle_deg).map(|job| {
                let text = format!("Turned shapes {} by {angle_deg}°.", shape_names(&ids));
                (job, vec![text])
            })
        }),
        ShapeAction::Scale {
            ids, factor, about, ..
        } => about_point(&job, &ids, about).and_then(|c| {
            transform_shapes(&job, &ids, c, c, factor, 0.0).map(|job| {
                let text = format!("Scaled shapes {} ×{factor}.", shape_names(&ids));
                (job, vec![text])
            })
        }),
        ShapeAction::Join { ids, gap, .. } => join_shapes(&job, &ids, gap).map(|(job, report)| {
            joined = Some((report.shapes, report.removed));
            (job, report.lines)
        }),
    };
    let (job, lines) = match changed {
        Ok(done) => done,
        Err(err) => return out.fail(err),
    };
    let target = output.as_deref().unwrap_or(&project);
    if let Err(err) = save_job(&job, target) {
        return out.fail(err);
    }
    for line in lines {
        out.line(line.to_string());
    }
    out.wrote(target, "");
    if let Some(ids) = new_ids {
        out.set("ids", json!(ids));
    }
    if let Some((shapes, removed)) = joined {
        out.set("shapes", json!(shapes));
        out.set("removed", json!(removed));
    }
    if flagged_ops == 0 {
        ExitCode::SUCCESS
    } else {
        // As for `update`: written, but some operations can't run until fixed.
        ExitCode::from(EXIT_FAILED)
    }
}

/// "X,Y" in mm, each optionally with a unit (`1in,2in`).
pub(crate) fn point_arg(text: &str) -> Result<[f64; 2], String> {
    lengths::point(text)
}

pub(crate) fn points_arg(text: &str) -> Result<Vec<[f64; 2]>, String> {
    text.split_whitespace().map(point_arg).collect()
}

pub(crate) fn bulges_arg(text: &str) -> Result<Vec<f64>, String> {
    text.split_whitespace()
        .map(|b| {
            b.parse::<f64>()
                .map_err(|_| format!("expected a bulge as a number, got {b:?}"))
        })
        .collect()
}

pub(crate) fn need<T>(value: Option<T>, what: &str, kind: &str) -> Result<T, String> {
    value.ok_or_else(|| format!("a {kind} needs --{what}"))
}

pub(crate) fn new_geometry(kind: KindArg, v: &ShapeValues) -> Result<Geometry, String> {
    let center = v.center.as_deref().map(point_arg).transpose()?;
    if v.corner_radius.is_some() && !matches!(kind, KindArg::Rectangle) {
        return Err("--corner-radius is for a rectangle".into());
    }
    if (v.sides.is_some() || v.across_flats.is_some()) && !matches!(kind, KindArg::Polygon) {
        return Err("--sides and --across-flats are for a polygon".into());
    }
    Ok(match kind {
        KindArg::Circle => Geometry::Circle {
            center: need(center, "center", "circle")?,
            radius: need(v.radius, "radius", "circle")?,
        },
        KindArg::Rectangle => Geometry::Rectangle {
            center: need(center, "center", "rectangle")?,
            length: need(v.length, "length", "rectangle")?,
            width: need(v.width, "width", "rectangle")?,
            angle: v.angle_deg.unwrap_or(0.0).to_radians(),
            corner_radius: v.corner_radius.unwrap_or(0.0),
        },
        KindArg::Polyline => {
            let closed = !v.open;
            let points = points_arg(need(v.points.as_deref(), "points", "polyline")?)?;
            let bulges = match &v.bulges {
                Some(text) => bulges_arg(text)?,
                None => vec![0.0; polyline_segment_count(points.len(), closed)],
            };
            Geometry::Polyline {
                closed,
                points,
                bulges,
            }
        }
        KindArg::Polygon => {
            let sides = need(v.sides, "sides", "polygon")?;
            let radius = match v.across_flats {
                Some(flats) => polygon_radius_across_flats(flats, sides),
                None => need(v.radius, "radius", "polygon")?,
            };
            Geometry::Polygon {
                center: need(center, "center", "polygon")?,
                radius,
                sides,
                angle: v.angle_deg.unwrap_or(0.0).to_radians(),
            }
        }
    })
}

/// A polygon's radius to its corners from its size across the flats.
pub(crate) fn polygon_radius_across_flats(flats: f64, sides: u32) -> f64 {
    flats / 2.0 / (std::f64::consts::PI / f64::from(sides.max(3))).cos()
}

/// The shape's geometry with the given values replaced; values that don't belong to its
/// kind are refused rather than ignored.
pub(crate) fn changed_geometry(current: &Geometry, v: &ShapeValues) -> Result<Geometry, String> {
    let center = v.center.as_deref().map(point_arg).transpose()?;
    let not_for = |kind: &str, given: &[(&str, bool)]| -> Result<(), String> {
        match given.iter().find(|(_, set)| *set) {
            Some((name, _)) => Err(format!("--{name} doesn't apply to a {kind}")),
            None => Ok(()),
        }
    };
    let mut geometry = current.clone();
    match &mut geometry {
        Geometry::Path { .. } => {}
        Geometry::Circle { center: c, radius } => {
            not_for(
                "circle",
                &[
                    ("length", v.length.is_some()),
                    ("width", v.width.is_some()),
                    ("angle-deg", v.angle_deg.is_some()),
                    ("corner-radius", v.corner_radius.is_some()),
                    ("points", v.points.is_some()),
                    ("bulges", v.bulges.is_some()),
                    ("open", v.open),
                    ("closed", v.closed),
                    ("sides", v.sides.is_some()),
                    ("across-flats", v.across_flats.is_some()),
                ],
            )?;
            *c = center.unwrap_or(*c);
            *radius = v.radius.unwrap_or(*radius);
        }
        Geometry::Polygon {
            center: c,
            radius,
            sides,
            angle,
        } => {
            not_for(
                "polygon",
                &[
                    ("length", v.length.is_some()),
                    ("width", v.width.is_some()),
                    ("corner-radius", v.corner_radius.is_some()),
                    ("points", v.points.is_some()),
                    ("bulges", v.bulges.is_some()),
                    ("open", v.open),
                    ("closed", v.closed),
                ],
            )?;
            *c = center.unwrap_or(*c);
            *sides = v.sides.unwrap_or(*sides);
            *radius = match v.across_flats {
                Some(flats) => polygon_radius_across_flats(flats, *sides),
                None => v.radius.unwrap_or(*radius),
            };
            if let Some(degrees) = v.angle_deg {
                *angle = degrees.to_radians();
            }
        }
        Geometry::Rectangle {
            center: c,
            length,
            width,
            angle,
            corner_radius,
        } => {
            not_for(
                "rectangle",
                &[
                    ("radius", v.radius.is_some()),
                    ("points", v.points.is_some()),
                    ("bulges", v.bulges.is_some()),
                    ("open", v.open),
                    ("closed", v.closed),
                    ("sides", v.sides.is_some()),
                    ("across-flats", v.across_flats.is_some()),
                ],
            )?;
            *c = center.unwrap_or(*c);
            *length = v.length.unwrap_or(*length);
            *width = v.width.unwrap_or(*width);
            *corner_radius = v.corner_radius.unwrap_or(*corner_radius);
            if let Some(degrees) = v.angle_deg {
                *angle = degrees.to_radians();
            }
        }
        Geometry::Polyline {
            closed,
            points,
            bulges,
        } => {
            not_for(
                "polyline",
                &[
                    ("center", center.is_some()),
                    ("radius", v.radius.is_some()),
                    ("length", v.length.is_some()),
                    ("width", v.width.is_some()),
                    ("angle-deg", v.angle_deg.is_some()),
                    ("corner-radius", v.corner_radius.is_some()),
                    ("sides", v.sides.is_some()),
                    ("across-flats", v.across_flats.is_some()),
                ],
            )?;
            if let Some(text) = &v.points {
                *points = points_arg(text)?;
            }
            if v.open {
                *closed = false;
            } else if v.closed {
                *closed = true;
            }
            let count = polyline_segment_count(points.len(), *closed);
            if let Some(text) = &v.bulges {
                *bulges = bulges_arg(text)?;
            } else if bulges.len() != count {
                // The segments changed: straight ones stay straight, but which bend a curved
                // one should keep isn't ours to guess.
                if bulges.iter().any(|&b| b != 0.0) {
                    return Err(format!(
                        "the polyline now has {count} segments and some were curved: \
                         give --bulges too"
                    ));
                }
                *bulges = vec![0.0; count];
            }
        }
    }
    Ok(geometry)
}
