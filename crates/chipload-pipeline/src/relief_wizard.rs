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
            around: Around::Rim,
            margin: DEFAULT_MARGIN_MM,
            corner: 0.0,
            ramp: DEFAULT_RIM.ramp,
            tool,
            rough: None,
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
    }
    let relief = append(&mut job, part, op);
    operations.push(relief);
    if let Some((tool, allowance)) = request.rough {
        let cutting = crate::new_ops::tool_cutting(&job, library, tool);
        crate::add_roughing_pass(&mut job, relief + 1, tool, allowance, cutting.as_ref())?;
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
