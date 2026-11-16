// SPDX-License-Identifier: GPL-3.0-or-later
//! A shape command's change: the project edited, then written with its report.

use super::*;

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

/// `shape similar` and `shape inside`: the ids of the shapes like shape `id`, or inside it.
fn select_by_kind(out: &mut Out, project: &Path, id: u64, inside: bool) -> ExitCode {
    use chipload_pipeline::selection;
    let job = match load(out, project) {
        Ok(job) => job,
        Err(code) => return code,
    };
    let contours = match load_drawing(&job) {
        Ok(drawing) => drawing.contours,
        Err(err) => return out.fail(err),
    };
    let Some(contour) = selection::contour_of_shape(&contours, id) else {
        return out.fail(format!("there is no shape S{id} in the drawing"));
    };
    let found = if inside {
        match selection::inside(&contours, contour) {
            Some(found) => found,
            None => return out.fail(format!("S{id} is not a closed shape: it has no inside")),
        }
    } else {
        selection::similar(&contours, contour)
    };
    let shapes: Vec<u64> = selection::shapes_of(&contours, &found)
        .into_iter()
        .collect();
    let list: Vec<String> = shapes.iter().map(u64::to_string).collect();
    out.set("shapes", json!(shapes));
    if list.is_empty() {
        out.line(format!("Nothing inside S{id}."));
    } else {
        out.line(list.join(","));
    }
    ExitCode::SUCCESS
}

pub(crate) fn shape(out: &mut Out, action: ShapeAction) -> ExitCode {
    let action = match action {
        ShapeAction::Library(command) => return shape_library::run(out, command),
        ShapeAction::Similar { project, id } => return select_by_kind(out, &project, id, false),
        ShapeAction::Inside { project, id } => return select_by_kind(out, &project, id, true),
        other => other,
    };
    let Some((project, output)) = action.files() else {
        return ExitCode::FAILURE;
    };
    let job = match load(out, &project) {
        Ok(job) => job,
        Err(code) => return code,
    };
    let mut report = ShapeReport::default();
    let (job, lines) = match change_shapes(&job, action, &mut report) {
        Ok(done) => done,
        Err(err) => return out.fail(err),
    };
    let target = output.as_deref().unwrap_or(&project);
    write_shapes(out, &job, &lines, target, report)
}

/// What a shape command found besides the changed project, for its report.
#[derive(Default)]
struct ShapeReport {
    /// Operations the change left with a problem.
    flagged_ops: usize,
    /// The new shapes' ids (`shape add`, `shape copy`).
    new_ids: Option<Vec<u64>>,
    /// What `shape join` joined and removed.
    joined: Option<(Vec<u64>, Vec<u64>)>,
}

impl ShapeAction {
    /// The project a changing command reads, and where it writes (None: the project itself);
    /// None for the commands that only answer.
    fn files(&self) -> Option<(PathBuf, Option<PathBuf>)> {
        match self {
            ShapeAction::Library(_) | ShapeAction::Similar { .. } | ShapeAction::Inside { .. } => {
                None
            }
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
            | ShapeAction::ArrayLinear {
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
            } => Some((project.clone(), output.clone())),
        }
    }
}

/// The project with the command's change, and the lines to say.
fn change_shapes(
    job: &Job,
    action: ShapeAction,
    report: &mut ShapeReport,
) -> Result<(Job, Vec<String>), PipelineError> {
    match action {
        ShapeAction::Library(_) | ShapeAction::Similar { .. } | ShapeAction::Inside { .. } => {
            Err(PipelineError::Job(foreign_text(
                "this shape command doesn't change the project",
            )))
        }
        ShapeAction::Add { kind, values, .. } => new_geometry(kind, &values)
            .map_err(|e| PipelineError::Job(foreign_text(e)))
            .and_then(|geometry| {
                let layer = values.layer.as_deref().unwrap_or("0");
                let (job, id) = add_shape(job, layer, geometry)?;
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
                report.new_ids = Some(vec![id]);
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
                let job = set_shape(job, id, &layer, geometry)?;
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
                report.flagged_ops = notes.len();
                let mut lines = vec![format!("Changed shape S{id}.")];
                lines.extend(notes.iter().map(|n| format!("warning: {}", n.english())));
                (job, lines)
            }),
        ShapeAction::Remove { id, .. } => delete_shape(job, id).map(|(job, flagged)| {
            report.flagged_ops = flagged.len();
            let mut lines = vec![format!("Deleted shape S{id}.")];
            lines.extend(flagged.into_iter().map(|l| format!("warning: {l}")));
            (job, lines)
        }),
        ShapeAction::Copy { ids, dx, dy, .. } => {
            copy_shapes(job, &ids, (dx, dy)).map(|(job, new)| {
                let names: Vec<String> = new.iter().map(|id| format!("S{id}")).collect();
                report.new_ids = Some(new.clone());
                (job, vec![format!("Added shapes {}.", names.join(", "))])
            })
        }
        ShapeAction::Mirror { ids, axis, .. } => {
            let mirror = match axis {
                AxisArg::X => Mirror::LeftRight,
                AxisArg::Y => Mirror::UpDown,
            };
            mirror_shapes(job, &ids, mirror).map(|job| {
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
            offset_shapes(job, &ids, distance, side, corners, count).map(|edited| {
                report.new_ids = Some(edited.new_ids.clone());
                let mut lines = vec![format!("Added shapes {}.", shape_names(&edited.new_ids))];
                lines.extend(edited.notes.iter().map(Message::english));
                (edited.job, lines)
            })
        }
        ShapeAction::Fillet {
            id, radius, corner, ..
        } => cut_shape_corners(job, id, corner, Cut::Fillet(radius))
            .map(|job| (job, vec![format!("Rounded the corners of S{id}.")])),
        ShapeAction::Chamfer {
            id,
            distance,
            corner,
            ..
        } => cut_shape_corners(job, id, corner, Cut::Chamfer(distance))
            .map(|job| (job, vec![format!("Cut the corners of S{id}.")])),
        ShapeAction::Trim { id, at, .. } => trim_shape(job, id, (at[0], at[1])).map(|edited| {
            report.new_ids = Some(edited.new_ids.clone());
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
            extend_shape(job, id, end).map(|edited| (edited.job, vec![format!("Extended S{id}.")]))
        }
        ShapeAction::Break { id, at, .. } => break_shape(job, id, (at[0], at[1])).map(|edited| {
            report.new_ids = Some(edited.new_ids.clone());
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
        } => array_grid(job, &ids, columns, rows, (dx, dy)).map(|edited| {
            report.new_ids = Some(edited.new_ids.clone());
            let mut lines = vec![format!("Added shapes {}.", shape_names(&edited.new_ids))];
            lines.extend(edited.notes.iter().map(Message::english));
            (edited.job, lines)
        }),
        ShapeAction::ArrayLinear {
            ids, count, step, ..
        } => array_linear(job, &ids, count, (step[0], step[1])).map(|edited| {
            report.new_ids = Some(edited.new_ids.clone());
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
            array_polar(job, &ids, polar).map(|edited| {
                report.new_ids = Some(edited.new_ids.clone());
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
            let with = if stock {
                AlignWith::Stock
            } else {
                AlignWith::First
            };
            align_shapes(job, &ids, to, with)
                .map(|job| (job, vec![format!("Aligned shapes {}.", shape_names(&ids))]))
        }
        ShapeAction::Distribute {
            ids, horizontal, ..
        } => distribute_shapes(job, &ids, horizontal).map(|job| {
            (
                job,
                vec![format!("Distributed shapes {}.", shape_names(&ids))],
            )
        }),
        ShapeAction::Union { ids, .. } => combined(
            job,
            Combine::Union,
            &ids,
            &mut report.new_ids,
            &mut report.flagged_ops,
        ),
        ShapeAction::Subtract { base, ids, .. } => {
            let all: Vec<u64> = std::iter::once(base).chain(ids).collect();
            combined(
                job,
                Combine::Subtract,
                &all,
                &mut report.new_ids,
                &mut report.flagged_ops,
            )
        }
        ShapeAction::Intersect { ids, .. } => combined(
            job,
            Combine::Intersect,
            &ids,
            &mut report.new_ids,
            &mut report.flagged_ops,
        ),
        ShapeAction::Move { ids, dx, dy, .. } => {
            transform_shapes(job, &ids, (0.0, 0.0), (dx, dy), 1.0, 0.0).map(|job| {
                let text = format!("Moved shapes {}.", shape_names(&ids));
                (job, vec![text])
            })
        }
        ShapeAction::Turn {
            ids,
            angle_deg,
            about,
            ..
        } => about_point(job, &ids, about).and_then(|c| {
            transform_shapes(job, &ids, c, c, 1.0, angle_deg).map(|job| {
                let text = format!("Turned shapes {} by {angle_deg}°.", shape_names(&ids));
                (job, vec![text])
            })
        }),
        ShapeAction::Scale {
            ids, factor, about, ..
        } => about_point(job, &ids, about).and_then(|c| {
            transform_shapes(job, &ids, c, c, factor, 0.0).map(|job| {
                let text = format!("Scaled shapes {} ×{factor}.", shape_names(&ids));
                (job, vec![text])
            })
        }),
        ShapeAction::Join { ids, gap, .. } => join_shapes(job, &ids, gap).map(|(job, join)| {
            report.joined = Some((join.shapes, join.removed));
            (job, join.lines)
        }),
    }
}

/// Write the changed project and report what changed; operations left with a problem make
/// the exit code a failure (as for `update`).
fn write_shapes(
    out: &mut Out,
    job: &Job,
    lines: &[String],
    target: &Path,
    report: ShapeReport,
) -> ExitCode {
    if let Err(err) = crate::save_project(out, job, target) {
        return out.fail(err);
    }
    for line in lines {
        out.line(line.to_string());
    }
    out.wrote(target, "");
    if let Some(ids) = report.new_ids {
        out.set("ids", json!(ids));
    }
    if let Some((shapes, removed)) = report.joined {
        out.set("shapes", json!(shapes));
        out.set("removed", json!(removed));
    }
    if report.flagged_ops == 0 {
        ExitCode::SUCCESS
    } else {
        // As for `update`: written, but some operations can't run until fixed.
        ExitCode::from(EXIT_FAILED)
    }
}
