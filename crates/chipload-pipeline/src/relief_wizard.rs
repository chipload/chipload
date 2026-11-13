// SPDX-License-Identifier: GPL-3.0-or-later
//! A relief in one step (docs/specs/relief-wizard.md §6): a 3D model turned, cut and sized,
//! its relief with an optional roughing pass, and what is round it (a rim and ramp round its
//! outline or round its box, or a cut-out; docs/specs/stl-relief.md §16), in a part of its
//! own and in cutting order. Afterwards they are ordinary things, edited as any others.

use chipload_core::chain::Contour;
use chipload_i18n::{Digits, Language, Message, Value};
use chipload_job::library::ToolLibrary;
use chipload_job::{
    DEFAULT_RIM, Job, ModelUp, Operation, RIM_MIN_RAMP, ReliefRim, RimShape, Selector, Source,
    ToolType, Units,
};

use crate::PipelineError;
use crate::models::{NewModel, ReadModel, add_model, remake};
use crate::new_ops::{NewRequest, OpKind, new_operations};

/// The rim's flat reaches this far round the model by default, mm.
pub const DEFAULT_MARGIN_MM: f64 = DEFAULT_RIM.width;

/// The rim's flat by default (docs/specs/stl-relief.md §16.6): wide enough for the roughing
/// tool `rough` (a project tool and its allowance) to reach the floor between the model and
/// the ramp. See [`rim_width_for`].
pub fn rim_width(job: &Job, rough: Option<(i64, f64)>) -> f64 {
    match rough {
        Some((tool, allowance)) => rim_width_for(job.tool(tool).map(|t| t.diameter), allowance),
        None => DEFAULT_MARGIN_MM,
    }
}

/// The rim's flat by default for a roughing tool `diameter` wide (None: no roughing) leaving
/// `allowance`: its diameter and the allowance on both sides, rounded up to a whole mm, and
/// at least [`DEFAULT_MARGIN_MM`].
pub fn rim_width_for(diameter: Option<f64>, allowance: f64) -> f64 {
    let needed = diameter
        .map(|d| d + 2.0 * allowance.max(0.0))
        .filter(|w| w.is_finite())
        .map_or(0.0, |w| {
            (w - chipload_core::tolerances::CHAIN_TOL_MM).ceil()
        });
    needed.max(DEFAULT_MARGIN_MM)
}

/// What goes round the relief (§6 step 4; docs/specs/stl-relief.md §16).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Around {
    /// A flat round the model's outline, then a ramp up to the stock top.
    #[default]
    Rim,
    /// The same round the rectangle round the model.
    Box,
    /// No rim: a profile outside the outline, through the board, with tabs, cut after the
    /// relief.
    CutOut,
}

impl Around {
    pub fn name(self) -> &'static str {
        match self {
            Around::Rim => "rim",
            Around::Box => "box",
            Around::CutOut => "cut-out",
        }
    }
}

/// The model the relief carves: a file just read, or one in the project already.
#[derive(Debug, Clone)]
pub enum ReliefModel<'a> {
    New {
        file: &'a str,
        read: &'a ReadModel,
        units: Units,
    },
    Existing(u64),
}

/// What *Relief…* asks, in its order (§6).
#[derive(Debug, Clone, PartialEq)]
pub struct ReliefRequest {
    /// Step 2: which way is up, the turn on the board (degrees), the share kept.
    pub up: ModelUp,
    pub rotation: f64,
    pub keep: f64,
    /// Step 3: its width, and its depth (None: in proportion to the width), mm; where
    /// its footprint's lower-left corner goes (None: centred on the board).
    pub width: f64,
    pub depth: Option<f64>,
    pub at: Option<(f64, f64)>,
    /// How far below the board's surface the model's top lies, mm: the relief's start
    /// depth, for an unplaned board. The roughing still starts at the surface.
    pub sink: f64,
    /// Step 4.
    pub around: Around,
    /// The rim's flat round the outline or the model's box, mm; a box's corner radius, mm;
    /// the ramp from horizontal, radians.
    pub margin: f64,
    pub corner: f64,
    pub ramp: f64,
    /// Step 5: the finishing ball nose; the roughing end mill and its allowance; the end
    /// mill for the cut-out (None: the roughing one).
    pub tool: i64,
    pub rough: Option<(i64, f64)>,
    /// A smoothing pass after the roughing, with its tool (docs/specs/stl-relief.md §17).
    pub smooth: bool,
    pub end_mill: Option<i64>,
}

impl ReliefRequest {
    /// A relief `width` wide, Z up, whole, centred, the default rim round its outline,
    /// finished with `tool` and not roughed.
    pub fn new(width: f64, tool: i64) -> ReliefRequest {
        ReliefRequest {
            up: ModelUp::Z,
            rotation: 0.0,
            keep: 1.0,
            width,
            depth: None,
            at: None,
            sink: 0.0,
            around: Around::Rim,
            margin: DEFAULT_MARGIN_MM,
            corner: 0.0,
            ramp: DEFAULT_RIM.ramp,
            tool,
            rough: None,
            smooth: false,
            end_mill: None,
        }
    }

    /// The end mill the cut-out is cut with.
    pub fn around_mill(&self) -> Option<i64> {
        self.end_mill.or(self.rough.map(|(tool, _)| tool))
    }

    /// The relief's rim: none for a cut-out.
    pub fn rim(&self) -> Option<ReliefRim> {
        let shape = match self.around {
            Around::Rim => RimShape::Outline,
            Around::Box => RimShape::Box,
            Around::CutOut => return None,
        };
        Some(ReliefRim {
            shape,
            width: self.margin,
            ramp: self.ramp,
            corner_radius: if shape == RimShape::Box {
                self.corner
            } else {
                0.0
            },
        })
    }
}

/// What *Relief…* made.
#[derive(Debug, Clone, PartialEq)]
pub struct ReliefMade {
    /// Its part, now the active one.
    pub part: u64,
    pub model: u64,
    /// The operations made, by index, in cutting order.
    pub operations: Vec<usize>,
    /// The relief's operation among them.
    pub relief: usize,
}

fn refused(message: Message) -> PipelineError {
    PipelineError::Job(message)
}

/// The tools the request names, of the kinds its steps need, or why they won't do.
fn check(job: &Job, request: &ReliefRequest) -> Result<(), PipelineError> {
    let kind = |id: i64| {
        job.tool(id)
            .map(|t| t.kind)
            .ok_or_else(|| refused(Message::new("pipeline-no-tool").arg("tool", id)))
    };
    if kind(request.tool)? != ToolType::Ballnose {
        return Err(refused(
            Message::new("pipeline-relief-ball").arg("tool", request.tool),
        ));
    }
    if let Some((tool, _)) = request.rough {
        kind(tool)?;
    }
    if !(request.sink.is_finite() && request.sink >= 0.0) {
        return Err(refused(
            Message::new("pipeline-relief-sink")
                .arg("got", Value::Length(request.sink, Digits::Shortest)),
        ));
    }
    if request.around == Around::CutOut {
        let mill = request
            .around_mill()
            .ok_or_else(|| refused(Message::new("pipeline-relief-needs-end-mill")))?;
        if kind(mill)? != ToolType::Endmill {
            return Err(refused(
                Message::new("pipeline-relief-end-mill").arg("tool", mill),
            ));
        }
    }
    let rim_fits = request.rim().is_none_or(|rim| {
        rim.width.is_finite()
            && rim.width >= 0.0
            && (RIM_MIN_RAMP..=std::f64::consts::FRAC_PI_2).contains(&rim.ramp)
            && rim.corner_radius.is_finite()
            && (0.0..=rim.width).contains(&rim.corner_radius)
    });
    if !rim_fits {
        return Err(refused(
            Message::new("pipeline-relief-margin")
                .arg("margin", Value::Length(request.margin, Digits::Shortest)),
        ));
    }
    Ok(())
}

/// A new operation of `kind` on `select` with `tool`, the app's defaults and the library's
/// feeds, in `part`.
fn make(
    job: &Job,
    contours: &[Contour],
    library: Option<&ToolLibrary>,
    lang: Language,
    (kind, select, tool, part): (OpKind, Selector, i64, u64),
) -> Result<Operation, PipelineError> {
    let request = NewRequest {
        kind,
        select,
        tool: Some(tool),
    };
    let mut op = new_operations(job, &request, contours, library, lang)
        .ops
        .into_iter()
        .next()
        .map(|n| n.op)
        .ok_or_else(|| refused(Message::new("pipeline-relief-nothing")))?;
    op.set_part(part);
    Ok(op)
}

/// `op` after its part's other operations; its index.
fn append(job: &mut Job, part: u64, op: Operation) -> usize {
    let at = job.insert_index(part);
    job.operations.insert(at, op);
    at
}

/// Make the relief `request` asks for of `model` in `job`, as one change.
pub fn add_relief(
    job: &Job,
    model: ReliefModel<'_>,
    request: &ReliefRequest,
    library: Option<&ToolLibrary>,
    lang: Language,
) -> Result<(Job, ReliefMade), PipelineError> {
    check(job, request)?;
    // The model, turned, cut and sized, at the drawing's origin for now.
    let (job, id) = match model {
        ReliefModel::New { file, read, units } => {
            let mut new = NewModel::new(file);
            new.units = units;
            new.up = request.up;
            new.rotation = request.rotation;
            new.keep = request.keep;
            new.width = Some(request.width);
            new.depth = request.depth;
            let (job, report) = add_model(job, read, new)?;
            (job, report.id)
        }
        ReliefModel::Existing(id) => {
            let mut out = job.clone();
            let m = out
                .drawing
                .models
                .iter_mut()
                .find(|m| m.id == id)
                .ok_or_else(|| refused(Message::new("pipeline-no-model").arg("id", id)))?;
            m.up = request.up;
            m.rotation = request.rotation;
            m.keep = request.keep;
            m.width = request.width;
            match request.depth {
                Some(depth) => {
                    m.depth = depth;
                    m.proportional = false;
                }
                None => m.proportional = true,
            }
            let (out, _) = remake(&out, id)?;
            (out, id)
        }
    };
    // Where it lies: at the corner asked for, or centred on the board.
    let mut job = {
        let size = crate::models::placed_mesh(&job, id)?
            .bounds()
            .map(|(lo, hi)| (hi[0] - lo[0], hi[1] - lo[1]))
            .unwrap_or((request.width, request.width));
        let at = request.at.unwrap_or_else(|| {
            let ((x0, y0), (x1, y1)) = job.stock_bounds();
            ((x0 + x1 - size.0) / 2.0, (y0 + y1 - size.1) / 2.0)
        });
        let mut out = job.clone();
        if let Some(m) = out.drawing.models.iter_mut().find(|m| m.id == id) {
            (m.x, m.y) = at;
        }
        remake(&out, id)?.0
    };
    let name = crate::models::find(&job, id)?.name.clone();
    // An empty active part (a new project's) becomes the relief's; otherwise a new one.
    let empty = job
        .active_part
        .filter(|p| !job.operations.iter().any(|op| op.part() == *p));
    let part = match empty {
        Some(p) => {
            let free = crate::parts::free_name(&job, Some(&name)).ok();
            if let (Some(free), Some(it)) = (free, job.parts.iter_mut().find(|it| it.id == p)) {
                it.name = free;
            }
            p
        }
        None => {
            let (out, p) = crate::parts::add_part(&job, Some(&name))
                .or_else(|_| crate::parts::add_part(&job, None))?;
            job = out;
            p
        }
    };
    let contours = crate::load_drawing(&job)?.contours;
    let op_name =
        |message: &'static str| Message::new(message).render(lang, chipload_i18n::Units::Mm);
    let mut operations = Vec::new();
    // The relief, with its roughing pass.
    let op = make(
        &job,
        &contours,
        library,
        lang,
        (
            OpKind::Relief,
            Selector::of(Source::Model(id)),
            request.tool,
            part,
        ),
    )?;
    let mut op = op;
    if let Operation::Relief(r) = &mut op {
        r.rim = request.rim();
        // Sunk below the surface: the model's depth below its start.
        r.end_depth += request.sink - r.start_depth;
        r.start_depth = request.sink;
    }
    let relief = append(&mut job, part, op);
    operations.push(relief);
    if let Some((tool, allowance)) = request.rough {
        let cutting = crate::new_ops::tool_cutting(&job, library, tool);
        crate::add_roughing_pass(&mut job, relief + 1, tool, allowance, cutting.as_ref())?;
        if request.smooth {
            crate::add_smoothing_pass(&mut job, relief + 1, None, None)?;
        }
    }
    // The cut-out last: outside the outline, through, with tabs.
    if let (Around::CutOut, Some(mill)) = (request.around, request.around_mill()) {
        let mut op = make(
            &job,
            &contours,
            library,
            lang,
            (OpKind::Profile, Selector::of(Source::Model(id)), mill, part),
        )?;
        *op.name_mut() = op_name("pipeline-name-sign-cut-out");
        operations.push(append(&mut job, part, op));
    }
    job.active_part = Some(part);
    Ok((
        job,
        ReliefMade {
            part,
            model: id,
            operations,
            relief,
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// The dome sample: model 1, tools 1 (a 6 mm end mill) and 2 (a ball nose).
    fn dome() -> Job {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/corpus/jobs/v36/relief_dome.chipload");
        chipload_job::load_job(&path).unwrap()
    }

    fn made(job: &Job, request: &ReliefRequest) -> (Job, ReliefMade) {
        add_relief(job, ReliefModel::Existing(1), request, None, Language::En).unwrap()
    }

    #[test]
    fn a_box_rim_goes_on_the_relief_and_the_board_grows_round_it() {
        let job = dome();
        let mut request = ReliefRequest::new(30.0, 2);
        request.around = Around::Box;
        request.margin = 8.0;
        request.corner = 4.0;
        request.ramp = 60f64.to_radians();
        request.rough = Some((1, 0.5));
        request.smooth = true;
        request.keep = 0.6;
        request.depth = Some(6.0);
        let (out, made) = made(&job, &request);
        // In a part of its own, now active: the relief alone, no pocket, no shape added.
        assert_ne!(made.part, job.parts[0].id);
        assert_eq!(out.active_part, Some(made.part));
        assert_eq!(made.operations, vec![made.relief]);
        assert_eq!(out.drawing.shapes.len(), job.drawing.shapes.len());
        // The relief: the model, the ball nose, roughed with the end mill, its rim a box.
        let Operation::Relief(relief) = &out.operations[made.relief] else {
            unreachable!()
        };
        assert_eq!(relief.part, made.part);
        assert_eq!((relief.model, relief.tool, relief.end_depth), (1, 2, 6.0));
        assert_eq!(
            relief.roughing.map(|r| (r.tool, r.allowance)),
            Some((1, 0.5))
        );
        // Smoothed with the roughing end mill, across its lines, 15 %, the same allowance.
        let smooth = relief.smoothing.unwrap();
        assert_eq!(
            (smooth.tool, smooth.allowance, smooth.stepover),
            (1, 0.5, 0.15)
        );
        assert!((smooth.angle - std::f64::consts::FRAC_PI_2).abs() < 1e-12);
        assert_eq!(
            out.cut_list()
                .into_iter()
                .filter(|r| r.operation == made.relief)
                .map(|r| r.to_string())
                .collect::<Vec<_>>(),
            [1, 2, 0].map(|k| match k {
                0 => format!("{}", made.relief + 1),
                k => format!("{}.{k}", made.relief + 1),
            })
        );
        assert_eq!(
            relief.rim,
            Some(ReliefRim {
                shape: RimShape::Box,
                width: 8.0,
                ramp: 60f64.to_radians(),
                corner_radius: 4.0,
            })
        );
        // The model turned, cut and sized as asked, centred on the board: the fitted board
        // grows round the rim's box, centred on the outline, which is traced within a
        // fraction of a millimetre of the model.
        let model = &out.drawing.models[0];
        assert_eq!((model.width, model.keep, model.depth), (30.0, 0.6, 6.0));
        let ((x0, y0), (x1, y1)) = out.stock_bounds();
        let (lo, hi) = crate::models::placed_mesh(&out, 1)
            .unwrap()
            .bounds()
            .unwrap();
        let middle = ((lo[0] + hi[0]) / 2.0, (lo[1] + hi[1]) / 2.0);
        let board = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);
        assert!((middle.0 - board.0).abs() < 0.2 && (middle.1 - board.1).abs() < 0.2);
        // The board holds the flat and the ramp: 30 mm, 8 mm flat and 6/tan 60° each side,
        // and the board's own 10 mm margin.
        let ramp = 6.0 / 60f64.to_radians().tan();
        assert!(
            x1 - x0 > 30.0 + 2.0 * (8.0 + ramp + 10.0) - 0.5,
            "{}",
            x1 - x0
        );
        // It runs, its G-code checked, and the board holds the ramp: no note it is cut off.
        let result = crate::run_job(&out).unwrap();
        assert!(!result.files.is_empty());
        let notes: Vec<String> = result.notes.iter().map(|n| n.message.english()).collect();
        assert!(
            !notes.iter().any(|n| n.contains("past the board")),
            "{notes:?}"
        );
    }

    #[test]
    fn the_default_flat_is_wide_enough_for_the_roughing_tool() {
        let job = dome();
        // Tool 1 is a 6 mm end mill: 6 + 2 × 0.2 = 6.4, rounded up to 7.
        assert_eq!(rim_width(&job, Some((1, 0.2))), 7.0);
        assert_eq!(rim_width(&job, Some((1, 0.5))), 7.0);
        assert_eq!(rim_width(&job, Some((1, 0.6))), 8.0);
        // Without roughing, or with a smaller tool: 5 mm, never less.
        assert_eq!(rim_width(&job, None), DEFAULT_MARGIN_MM);
        assert_eq!(rim_width_for(Some(3.0), 0.1), DEFAULT_MARGIN_MM);
        assert_eq!(rim_width_for(Some(7.0), 0.0), 7.0);
        assert_eq!(rim_width_for(None, 0.2), DEFAULT_MARGIN_MM);
    }

    #[test]
    fn a_flat_narrower_than_the_roughing_tool_is_noted() {
        let job = dome();
        let notes = |margin: f64| {
            let mut request = ReliefRequest::new(30.0, 2);
            request.rough = Some((1, 0.2));
            request.margin = margin;
            request.depth = Some(4.0);
            let (out, _) = made(&job, &request);
            let result = crate::run_job(&out).unwrap();
            result
                .notes
                .iter()
                .map(|n| n.message.english())
                .collect::<Vec<String>>()
        };
        let narrow = notes(5.0);
        assert!(
            narrow
                .iter()
                .any(|n| n.contains("narrower than the roughing tool")),
            "{narrow:?}"
        );
        let wide = notes(rim_width(&job, Some((1, 0.2))));
        assert!(
            !wide
                .iter()
                .any(|n| n.contains("narrower than the roughing tool")),
            "{wide:?}"
        );
    }

    /// The stock left by the end mill's passes alone (the relief's roughing, and smoothing
    /// if any), and by the whole program, on the same grid.
    fn before_and_after_the_finish(
        job: &Job,
        relief: usize,
    ) -> (crate::simulate::Simulation, crate::simulate::Simulation) {
        let result = crate::run_job(job).unwrap();
        let finish = chipload_job::OpRef::op(relief);
        let cancel = crate::Cancel::new();
        let all =
            crate::simulate::simulate(job, &result.program, &result.groups, Some(0.25), &cancel)
                .unwrap();
        let (mut program, groups) = (result.program.clone(), result.groups.clone());
        let at = groups
            .iter()
            .find(|g| g.operations.contains(&finish))
            .unwrap()
            .moves
            .start;
        program.moves.truncate(at);
        let before: Vec<_> = groups.into_iter().filter(|g| g.moves.end <= at).collect();
        let rough = crate::simulate::simulate(job, &program, &before, Some(0.25), &cancel).unwrap();
        (rough, all)
    }

    /// How much the end mill leaves above the finished surface on the rim's ramp, mm.
    fn left_on_the_ramp(job: &Job, relief: usize) -> f64 {
        let (rough, done) = before_and_after_the_finish(job, relief);
        let (lo, hi) = crate::models::placed_mesh(job, 1)
            .unwrap()
            .bounds()
            .unwrap();
        let mut most: f64 = 0.0;
        for j in 0..done.rows {
            for i in 0..done.columns {
                let x = done.origin.0 + (i as f64 + 0.5) * done.cell;
                let y = done.origin.1 + (j as f64 + 0.5) * done.cell;
                let h = done.height(i, j);
                // The ramp's four straight sides: off the flat round the model's box (8 mm),
                // beside the box, not at the top or the floor. Its corners are cones a
                // flat end mill can't reach into at all.
                let off = (lo[0] - 8.0 - x)
                    .max(x - hi[0] - 8.0)
                    .max(lo[1] - 8.0 - y)
                    .max(y - hi[1] - 8.0);
                let beside = (lo[0]..=hi[0]).contains(&x) || (lo[1]..=hi[1]).contains(&y);
                if beside && off > 0.5 && h < done.top - 0.5 && h > done.top - 5.5 {
                    most = most.max(rough.height(i, j) - h);
                }
            }
        }
        most
    }

    #[test]
    fn a_smoothing_pass_takes_the_roughing_steps_off_a_box_ramp() {
        let job = dome();
        let mut request = ReliefRequest::new(30.0, 2);
        request.around = Around::Box;
        request.margin = 8.0;
        request.rough = Some((1, 0.2));
        request.depth = Some(6.0);
        let (roughed, made_one) = made(&job, &request);
        request.smooth = true;
        let (smoothed, made_two) = made(&job, &request);
        let steps = left_on_the_ramp(&roughed, made_one.relief);
        let smooth = left_on_the_ramp(&smoothed, made_two.relief);
        // 40 % of 6 mm across a 45° ramp: steps up to 2.4 mm, and the allowance (0.28 mm
        // upright on 45°); smoothed along them, the allowance and the 0.25 mm grid's own
        // error on the slope.
        assert!(steps > 2.2, "{steps}");
        assert!(smooth < 0.8, "{smooth}");
    }

    #[test]
    fn a_sunken_relief_is_roughed_from_the_surface_and_its_ramp_meets_it() {
        let job = dome();
        let mut request = ReliefRequest::new(30.0, 2);
        request.rough = Some((1, 0.2));
        request.depth = Some(4.0);
        request.sink = 1.0;
        request.margin = 7.0;
        let (out, made) = made(&job, &request);
        let Operation::Relief(relief) = &out.operations[made.relief] else {
            unreachable!()
        };
        assert_eq!((relief.start_depth, relief.end_depth), (1.0, 5.0));
        let (rough, done) = before_and_after_the_finish(&out, made.relief);
        let (lo, hi) = crate::models::placed_mesh(&out, 1)
            .unwrap()
            .bounds()
            .unwrap();
        let cell = |sim: &crate::simulate::Simulation, x: f64, y: f64| {
            let i = ((x - sim.origin.0) / sim.cell) as usize;
            let j = ((y - sim.origin.1) / sim.cell) as usize;
            sim.height(i, j)
        };
        // The dome's top, 1 mm down: the roughing took the board's top layer off it, from
        // the surface, leaving the allowance; the finish cut it to the model.
        let middle = ((lo[0] + hi[0]) / 2.0, (lo[1] + hi[1]) / 2.0);
        let roughed = cell(&rough, middle.0, middle.1);
        assert!(roughed < -0.7 && roughed > -1.0, "{roughed}");
        let finished = cell(&done, middle.0, middle.1);
        assert!((finished + 1.0).abs() < 0.1, "{finished}");
        // Out past the flat (7 mm) and the ramp (5 mm deep at 45°): the surface, untouched.
        let out_x = hi[0] + 7.0 + 5.0 + 3.0;
        assert_eq!(cell(&done, out_x, middle.1), done.top);
        // Halfway up the ramp: halfway between the floor and the surface.
        let half = cell(&done, hi[0] + 7.0 + 2.5, middle.1);
        assert!((half - (done.top - 2.5)).abs() < 0.3, "{half}");
        // Above the surface (a start depth below 0) is refused.
        request.sink = -1.0;
        assert!(add_relief(&job, ReliefModel::Existing(1), &request, None, Language::En).is_err());
    }

    #[test]
    fn a_rim_round_the_outline_is_the_default() {
        let job = dome();
        let (out, made) = made(&job, &ReliefRequest::new(30.0, 2));
        let Operation::Relief(relief) = &out.operations[made.relief] else {
            unreachable!()
        };
        assert_eq!(relief.rim, Some(DEFAULT_RIM));
        // A ramp beyond 90° or below 10°, or a corner wider than the flat: refused.
        let mut steep = ReliefRequest::new(30.0, 2);
        steep.ramp = 95f64.to_radians();
        assert!(add_relief(&job, ReliefModel::Existing(1), &steep, None, Language::En).is_err());
        let mut corner = ReliefRequest::new(30.0, 2);
        corner.around = Around::Box;
        corner.corner = 6.0;
        assert!(add_relief(&job, ReliefModel::Existing(1), &corner, None, Language::En).is_err());
    }

    #[test]
    fn a_cut_out_comes_last_and_the_tools_must_fit_their_steps() {
        let job = dome();
        let mut request = ReliefRequest::new(30.0, 2);
        request.around = Around::CutOut;
        request.end_mill = Some(1);
        let (out, made) = made(&job, &request);
        let last = &out.operations[*made.operations.last().unwrap()];
        let Operation::Profile(profile) = last else {
            panic!("{last:?}")
        };
        assert_eq!(profile.side, chipload_job::Side::Outside);
        assert!(profile.through.is_some() && profile.tabs.is_some());
        // A finishing tool that isn't a ball nose, a cut-out without an end mill, or with
        // a ball nose: refused.
        let wrong = ReliefRequest::new(30.0, 1);
        assert!(add_relief(&job, ReliefModel::Existing(1), &wrong, None, Language::En).is_err());
        let mut no_mill = ReliefRequest::new(30.0, 2);
        no_mill.around = Around::CutOut;
        assert!(add_relief(&job, ReliefModel::Existing(1), &no_mill, None, Language::En).is_err());
        no_mill.end_mill = Some(2);
        assert!(add_relief(&job, ReliefModel::Existing(1), &no_mill, None, Language::En).is_err());
    }
}
