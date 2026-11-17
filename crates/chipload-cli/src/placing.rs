// SPDX-License-Identifier: GPL-3.0-or-later
//! `chipload place`, `zero` and `copies`: where the drawing, a DXF, a picture or a text sits,
//! the work zero, and a part's grid of copies.

use super::*;

#[derive(Clone, Copy, PartialEq, ValueEnum)]
pub(crate) enum AlignArg {
    /// Bottom-left corner on the stock's, --margin mm in.
    BottomLeft,
    /// Centre on the stock's centre.
    Center,
}

#[derive(Clone, Copy, ValueEnum)]
pub(crate) enum CornerArg {
    BottomLeft,
    BottomRight,
    TopLeft,
    TopRight,
    #[value(alias = "centre")]
    Center,
}

#[derive(Clone, Copy, ValueEnum)]
pub(crate) enum ZArg {
    Top,
    Bottom,
}

#[derive(Clone, Copy, ValueEnum)]
pub(crate) enum OrderArg {
    /// Each operation on every instance, then the next operation.
    ByOperation,
    /// Every operation on one instance, then the next (was `by-copy`, `by-part`).
    #[value(alias = "by-copy", alias = "by-part")]
    ByInstance,
}

impl OrderArg {
    pub(crate) fn value(self) -> InstanceOrder {
        match self {
            OrderArg::ByOperation => InstanceOrder::ByOperation,
            OrderArg::ByInstance => InstanceOrder::ByInstance,
        }
    }
}

/// What `chipload copies` was asked to change.
pub(crate) struct CopiesArgs {
    pub(crate) part: Option<u64>,
    /// The setup whose instances of the part make the grid; default: the board the part
    /// is cut from.
    pub(crate) setup: Option<u64>,
    pub(crate) grid: Option<String>,
    pub(crate) pitch: Option<String>,
    pub(crate) gap: Option<f64>,
    pub(crate) order: Option<OrderArg>,
    pub(crate) off: bool,
}

pub(crate) struct ZeroArgs {
    pub(crate) stock: Option<CornerArg>,
    pub(crate) drawing_origin: bool,
    pub(crate) at: Option<String>,
    pub(crate) z: Option<ZArg>,
}

/// `zero`: set the work zero, or print where it is (docs/specs/work-zero-marker.md §5).
pub(crate) fn zero(
    out: &mut Out,
    project: &Path,
    args: ZeroArgs,
    output: Option<&Path>,
) -> ExitCode {
    let mut job = match load_job(project) {
        Ok(job) => job,
        Err(err) => return out.fail(err),
    };
    let before = (job.work_zero(), job.stock().z_zero);
    let original = job.clone();
    if let Some(corner) = args.stock {
        let corner = match corner {
            CornerArg::BottomLeft => StockCorner::BottomLeft,
            CornerArg::BottomRight => StockCorner::BottomRight,
            CornerArg::TopLeft => StockCorner::TopLeft,
            CornerArg::TopRight => StockCorner::TopRight,
            CornerArg::Center => StockCorner::Center,
        };
        *job.work_zero_mut() = WorkZero::Stock { corner };
    }
    if args.drawing_origin {
        *job.work_zero_mut() = WorkZero::DrawingOrigin {};
    }
    if let Some(at) = &args.at {
        match point_arg(at) {
            Ok([x, y]) => *job.work_zero_mut() = WorkZero::Point { x, y },
            Err(err) => return out.fail(format!("--at: {err}")),
        }
    }
    if let Some(z) = args.z {
        job.stock_mut().z_zero = match z {
            ZArg::Top => ZZero::Top,
            ZArg::Bottom => ZZero::Bottom,
        };
    }
    let (x, y) = job.work_zero_xy();
    out.set(
        "work_zero",
        serde_json::to_value(job.work_zero()).unwrap_or_default(),
    );
    out.set(
        "z_zero",
        serde_json::to_value(job.stock().z_zero).unwrap_or_default(),
    );
    out.set("x", json!(x));
    out.set("y", json!(y));
    out.line(format!("Work zero: {}", job.work_zero_text()));
    if (job.work_zero(), job.stock().z_zero) == before && output.is_none() {
        return ExitCode::SUCCESS;
    }
    // Set by hand: a board made for a part's stock is the user's now.
    job.keep_made_flags(&original);
    let problems = job.validate();
    if !problems.is_empty() {
        return out.fail(chipload_pipeline::joined(&problems));
    }
    let target = output.unwrap_or(project);
    write_project(out, &job, target)
}

pub(crate) fn copies(
    out: &mut Out,
    project: &Path,
    args: CopiesArgs,
    output: Option<&Path>,
) -> ExitCode {
    let mut job = match load_job(project) {
        Ok(job) => job,
        Err(err) => return out.fail(err),
    };
    let part = match (args.part, job.parts.as_slice()) {
        (Some(id), _) => id,
        (None, [only]) => only.id,
        (None, _) => {
            return out.fail("the project has several parts: give --part ID (see `part list`)");
        }
    };
    let Some(at) = job.parts.iter().position(|p| p.id == part) else {
        return out.fail(format!("no part {part} in the project (see `part list`)"));
    };
    let setup = match args.setup {
        Some(id) if job.setup(id).is_none() => {
            return out.fail(format!("no setup {id} in the project (see `setup list`)"));
        }
        Some(id) => Some(id),
        None => job.home_setup(part).or_else(|| job.active_setup_id()),
    };
    match changed_copies(&job, part, args) {
        Ok((grid, order)) => {
            // The grid is made on its board; the active setup stays as it was.
            let active = job.active_setup;
            job.active_setup = setup;
            match grid {
                Some(GridChange::Grid(copies)) => job.set_grid_here(part, &copies),
                Some(GridChange::Off) => {
                    let own = job.instances_here(part).first().map(|i| **i);
                    job.set_instances_here(part, own.into_iter().collect());
                }
                None => {}
            }
            job.active_setup = active;
            if let Some(order) = order {
                job.parts[at].order = order;
            }
        }
        Err(err) => return out.fail(err),
    }
    let problems = job.validate();
    if !problems.is_empty() {
        return out.fail(chipload_pipeline::joined(&problems));
    }
    let target = output.unwrap_or(project);
    if let Err(err) = crate::save_project(out, &job, target) {
        return out.fail(err);
    }
    let shown = setup.map_or_else(|| job.clone(), |id| job.for_setup(id));
    if shown.instances_here(part).len() < 2 {
        out.line("Instances: none (cut once)");
    } else {
        out.line(format!(
            "Instances: {}",
            instances_text(&shown, &job.parts[at])
        ));
    }
    warn_overlap(out, &job, part);
    out.wrote(target, "");
    ExitCode::SUCCESS
}

/// Overlap is refused when the project is run: with `part` cut more than once, say so now.
pub(crate) fn warn_overlap(out: &mut Out, job: &Job, part: u64) {
    if job.count_all(part) > 1
        && let Ok(drawing) = load_drawing(job)
        && let Err(err) = plan_job(job, &drawing.contours)
        && matches!(err.cause(), ToolpathError::Overlap(_))
    {
        out.err(format!("warning: {err}"));
    }
}

/// What `copies` does to a part's instances on its board.
pub(crate) enum GridChange {
    /// The part's instances there but its first made this grid.
    Grid(Copies),
    /// Only its first instance there left: cut once.
    Off,
}

/// What `copies` makes of a part (docs/specs/instances.md §5): with --grid, the grid's
/// instances in place of the part's; with --order, its cut order; with --off, none.
pub(crate) fn changed_copies(
    job: &Job,
    part: u64,
    args: CopiesArgs,
) -> Result<(Option<GridChange>, Option<InstanceOrder>), String> {
    if args.off {
        return Ok((Some(GridChange::Off), None));
    }
    let order = args.order.map(OrderArg::value);
    let Some(grid) = &args.grid else {
        if args.pitch.is_some() || args.gap.is_some() {
            return Err("a grid's spacing goes with its --grid COLUMNSxROWS".into());
        }
        if order.is_none() {
            return Err("give --grid (and --pitch or --gap), --order or --off".into());
        }
        return Ok((None, order));
    };
    let (columns, rows) = crate::grid_counts(grid)
        .ok_or_else(|| format!("--grid wants columns x rows like 3x2, got \"{grid}\""))?;
    let mut copies = Copies {
        columns,
        rows,
        dx: 0.0,
        dy: 0.0,
    };
    if let Some(pitch) = &args.pitch {
        [copies.dx, copies.dy] = point_arg(pitch).map_err(|e| format!("--pitch: {e}"))?;
    }
    if let Some(gap) = args.gap {
        (copies.dx, copies.dy) =
            pitch_for_gap(job, part, gap, (1.0, 1.0)).map_err(|e| e.to_string())?;
    } else if args.pitch.is_none() && copies.count() > 1 {
        return Err("give the spacing with --pitch DX,DY or --gap MM".into());
    }
    let problems = copies.problems("--grid");
    if let Some(problem) = problems.first() {
        return Err(problem.to_string());
    }
    Ok((Some(GridChange::Grid(copies)), order))
}

/// What `chipload place` was asked to do.
pub(crate) struct PlaceArgs {
    pub(crate) by: Option<String>,
    pub(crate) to: Option<String>,
    pub(crate) at: Option<String>,
    pub(crate) align: Option<AlignArg>,
    pub(crate) margin: Option<f64>,
    pub(crate) rotate: Option<f64>,
    pub(crate) scale: Option<f64>,
    pub(crate) width: Option<f64>,
    pub(crate) height: Option<f64>,
    pub(crate) reset: bool,
    pub(crate) dxf: Option<u64>,
    pub(crate) image: Option<u64>,
    pub(crate) text: Option<u64>,
}

pub(crate) fn place(
    out: &mut Out,
    project: &Path,
    args: PlaceArgs,
    output: Option<&Path>,
) -> ExitCode {
    if let Some(id) = args.image {
        return place_image(out, project, id, args, output);
    }
    if let Some(id) = args.text {
        return place_text(out, project, id, args, output);
    }
    let job = match load_job(project) {
        Ok(job) => job,
        Err(err) => return out.fail(err),
    };
    let which = args.dxf;
    let steps = match place_moves(args) {
        Ok(steps) => steps,
        Err(err) => return out.fail(err),
    };
    if steps.is_empty() {
        let source = match which {
            Some(id) => job.drawing.dxfs.iter().find(|d| d.id == id),
            None if job.drawing.dxfs.len() == 1 => job.drawing.dxfs.first(),
            None if job.drawing.dxfs.len() > 1 => {
                return out.fail(format!(
                    "the project has {} DXFs ({}): say which with --dxf",
                    job.drawing.dxfs.len(),
                    dxf_list(&job.drawing.dxfs)
                ));
            }
            None => None,
        };
        return match source {
            Some(source) => {
                out.line(format!("Drawing: {}", placement_text(source.placement)));
                let size = dxf_size(&job.drawing.shapes, source.id);
                if let Some((w, h)) = size {
                    out.line(format!("Size: {w:.3} × {h:.3} mm"));
                }
                out.set("placement", json!(source.placement));
                out.set("size", json!(size.map(|(w, h)| [w, h])));
                ExitCode::SUCCESS
            }
            None => out.fail("the project has no imported drawing"),
        };
    }
    let mut placed = job;
    let mut lines = Vec::new();
    for how in steps {
        let done = match which {
            Some(id) => place_dxf(&placed, id, how),
            None => place_drawing(&placed, how),
        };
        match done {
            Ok((next, report)) => {
                placed = next;
                lines.extend(report.lines);
            }
            Err(err) => return out.fail(err),
        }
    }
    let target = output.unwrap_or(project);
    if let Err(err) = crate::save_project(out, &placed, target) {
        return out.fail(err);
    }
    for line in &lines {
        out.line(line.to_string());
    }
    out.wrote(target, "");
    ExitCode::SUCCESS
}

/// `place --image`: a picture's corner, turn and width, as `image set` sets them
/// (docs/specs/direct-placement.md).
pub(crate) fn place_image(
    out: &mut Out,
    project: &Path,
    id: u64,
    args: PlaceArgs,
    output: Option<&Path>,
) -> ExitCode {
    let others = args.by.is_some()
        || args.to.is_some()
        || args.align.is_some()
        || args.margin.is_some()
        || args.scale.is_some()
        || args.height.is_some()
        || args.reset;
    if others {
        return out.fail("with --image, place takes --at, --rotate and --width (as `image set`)");
    }
    let at = match args.at.as_deref().map(point_arg) {
        Some(Ok(p)) => Some((p[0], p[1])),
        Some(Err(err)) => return out.fail(err),
        None => None,
    };
    if at.is_none() && args.rotate.is_none() && args.width.is_none() {
        return out.fail("with --image, say what to change: --at, --rotate or --width");
    }
    images::place_image(out, project, id, at, args.rotate, args.width, output)
}

/// `place --text`: a text's corner, turn and capital height, as `text set` sets them.
pub(crate) fn place_text(
    out: &mut Out,
    project: &Path,
    id: u64,
    args: PlaceArgs,
    output: Option<&Path>,
) -> ExitCode {
    let others = args.by.is_some()
        || args.to.is_some()
        || args.align.is_some()
        || args.margin.is_some()
        || args.scale.is_some()
        || args.width.is_some()
        || args.reset;
    if others {
        return out.fail("with --text, place takes --at, --rotate and --height (as `text set`)");
    }
    let at = match args.at.as_deref().map(point_arg) {
        Some(Ok(p)) => Some((p[0], p[1])),
        Some(Err(err)) => return out.fail(err),
        None => None,
    };
    if at.is_none() && args.rotate.is_none() && args.height.is_none() {
        return out.fail("with --text, say what to change: --at, --rotate or --height");
    }
    texts::place_text(out, project, id, at, args.rotate, args.height, output)
}

/// The moves `place` was asked for, in order: a turn, a scale, then a move or an alignment.
/// Empty: just print the placement.
pub(crate) fn place_moves(args: PlaceArgs) -> Result<Vec<PlaceMove>, String> {
    if args.reset {
        return Ok(vec![PlaceMove::Reset]);
    }
    let mut steps = Vec::new();
    if let Some(degrees) = args.rotate {
        if !degrees.is_finite() {
            return Err(format!("--rotate takes degrees, got {degrees}"));
        }
        steps.push(PlaceMove::Rotate(degrees));
    }
    if let Some(factor) = args.scale {
        steps.push(PlaceMove::Scale(factor));
    }
    if let Some(width) = args.width {
        steps.push(PlaceMove::Width(width));
    }
    if let Some(height) = args.height {
        steps.push(PlaceMove::Height(height));
    }
    steps.extend(place_move(args.by, args.to, args.align, args.margin)?);
    if let Some(at) = args.at {
        let [x, y] = point_arg(&at).map_err(|e| format!("--at: {e}"))?;
        steps.push(PlaceMove::Corner(x, y));
    }
    Ok(steps)
}

/// The move or alignment `place` was asked for, if any.
pub(crate) fn place_move(
    by: Option<String>,
    to: Option<String>,
    align: Option<AlignArg>,
    margin: Option<f64>,
) -> Result<Option<PlaceMove>, String> {
    if margin.is_some() && align != Some(AlignArg::BottomLeft) {
        return Err("--margin goes with --align bottom-left".into());
    }
    if let Some(by) = by {
        let [dx, dy] = point_arg(&by).map_err(|e| format!("--by: {e}"))?;
        return Ok(Some(PlaceMove::By(dx, dy)));
    }
    if let Some(to) = to {
        let [dx, dy] = point_arg(&to).map_err(|e| format!("--to: {e}"))?;
        return Ok(Some(PlaceMove::To(dx, dy)));
    }
    Ok(align.map(|align| match align {
        AlignArg::BottomLeft => PlaceMove::BottomLeft {
            margin: margin.unwrap_or(0.0),
        },
        AlignArg::Center => PlaceMove::Center,
    }))
}
