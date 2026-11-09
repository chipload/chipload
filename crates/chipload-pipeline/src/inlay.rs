// SPDX-License-Identifier: GPL-3.0-or-later
//! V-carve inlays (docs/specs/vcarve-inlay.md): a pocket V-carved in the base board and a
//! plug V-carved, mirrored, in a board of its own, made as a pair.

use chipload_core::chain::Contour;
use chipload_i18n::{Language, Message, Units};
use chipload_job::library::ToolLibrary;
use chipload_job::{Job, Mirror, Operation, Plug, Selector, StockOrigin, ToolType};

use crate::PipelineError;
use crate::new_ops::{NewRequest, OpKind, new_operations, tool_cutting};
use crate::parts::free_name;

/// The pocket's depth when not given, mm (docs/specs/vcarve-inlay.md decision 5).
pub const DEFAULT_POCKET_DEPTH_MM: f64 = 5.0;
/// The plug's start depth when not given, mm: 2 mm of space under its face.
pub const DEFAULT_PLUG_START_MM: f64 = 3.0;
/// The glue gap when not given, mm.
pub const DEFAULT_GLUE_GAP_MM: f64 = 1.5;
/// The plug board's thickness when not given, mm.
pub const DEFAULT_PLUG_THICKNESS_MM: f64 = 10.0;
/// The margin the plug board is fitted with round the artwork, mm (decision 3).
pub const PLUG_BOARD_MARGIN_MM: f64 = 10.0;

/// What an inlay is made of (docs/specs/vcarve-inlay.md §4).
#[derive(Debug, Clone, PartialEq)]
pub struct InlayRequest {
    /// The artwork: closed shapes.
    pub select: Selector,
    /// The V-bit both are carved with.
    pub tool: i64,
    /// The pocket's depth, mm.
    pub depth: f64,
    /// The plug's start depth D, mm.
    pub plug_start: f64,
    /// The glue gap g, mm: the plug's end depth is D + g.
    pub gap: f64,
    /// An end mill for both floor passes.
    pub floor_tool: Option<i64>,
    /// The plug board's thickness, mm.
    pub plug_thickness: f64,
}

/// Where the pair went.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InlayMade {
    /// The pocket's index in the operations, in the active part.
    pub pocket: usize,
    /// The plug's index.
    pub plug: usize,
    /// The plug board's setup.
    pub setup: u64,
    /// The plug's part, mirrored left to right.
    pub part: u64,
}

fn refused(message: Message) -> PipelineError {
    PipelineError::Job(message)
}

/// The pocket in the active part and, in a new setup "Inlay plug" with a part of its own
/// mirrored left to right, the plug (docs/specs/vcarve-inlay.md §4). The plug board fits the
/// artwork with a margin of [`PLUG_BOARD_MARGIN_MM`], `plug_thickness` thick, its work zero
/// as the base's. Feeds come from the library as for any new V-carve. The active setup and
/// part stay as they were. Refused for a tool that isn't a V-bit with an angle, and for
/// anything the project's checks refuse.
pub fn add_inlay(
    job: &Job,
    request: &InlayRequest,
    contours: &[Contour],
    library: Option<&ToolLibrary>,
    lang: Language,
) -> Result<(Job, InlayMade), PipelineError> {
    let bit = job
        .tool(request.tool)
        .ok_or_else(|| refused(Message::new("pipeline-no-tool").arg("tool", request.tool)))?;
    if bit.kind != ToolType::Vbit || bit.angle.is_none() {
        return Err(refused(
            Message::new("pipeline-inlay-vbit").arg("tool", request.tool),
        ));
    }
    let mut job = job.clone();
    let (setup_before, part_before) = (job.active_setup, job.active_part);
    let new = NewRequest {
        kind: OpKind::Vcarve,
        select: request.select.clone(),
        tool: Some(request.tool),
    };
    let made = new_operations(&job, &new, contours, library, lang);
    let Some(Operation::Vcarve(mut pocket)) = made.ops.into_iter().next().map(|n| n.op) else {
        return Err(refused(
            Message::new("pipeline-inlay-vbit").arg("tool", request.tool),
        ));
    };
    let name = |id: &'static str| Message::new(id).render(lang, Units::Mm);
    pocket.name = name("pipeline-name-inlay-pocket");
    pocket.start_depth = 0.0;
    pocket.end_depth = request.depth;
    // The pocket and the plug meet in every corner (docs/specs/vcarve-inlay.md decision 6).
    pocket.ignore_kinks_deg = 0.0;
    pocket.plug = None;
    if let Some(floor) = request.floor_tool {
        let cutting = tool_cutting(&job, library, floor);
        let pass =
            crate::floor_pass_of(&pocket, &job.tools, floor, cutting.as_ref()).map_err(refused)?;
        pocket.floor = Some(pass);
    }
    let mut plug = pocket.clone();
    plug.name = name("pipeline-name-inlay-plug");
    plug.end_depth = request.plug_start + request.gap;
    plug.plug = Some(Plug {
        start_depth: request.plug_start,
    });
    if let (Some(floor), Some(pass)) = (request.floor_tool, plug.floor.as_mut()) {
        // The floor pass's steps for the plug's own depth, when they are the V-carve's.
        let cutting = tool_cutting(&job, library, floor);
        if cutting.is_none() {
            pass.step_down = plug.end_depth / f64::from(plug.passes.max(1));
        }
    }
    let at = job.insert_index(pocket.part);
    job.operations.insert(at, Operation::Vcarve(pocket));
    let (mut job, setup, part) = crate::setups::add_setup(
        &job,
        Some(&name("pipeline-setup-inlay-plug")),
        Some(request.plug_thickness),
    )?;
    let stock = job.stock_mut();
    stock.origin = StockOrigin::Drawing;
    stock.fit = Some(PLUG_BOARD_MARGIN_MM);
    // "Inlay plug", or "Inlay plug 2" and on for a second inlay.
    let wanted = name("pipeline-name-inlay-plug");
    let part_name = std::iter::once(wanted.clone())
        .chain((2..).map(|n| format!("{wanted} {n}")))
        .find(|candidate| free_name(&job, Some(candidate)).is_ok())
        .unwrap_or(wanted);
    if let Some(p) = job.parts.iter_mut().find(|p| p.id == part) {
        p.name = part_name;
        p.mirror = Some(Mirror::LeftRight);
    }
    plug.part = part;
    let plug_at = job.insert_index(part);
    job.operations.insert(plug_at, Operation::Vcarve(plug));
    job.active_setup = setup_before;
    job.active_part = part_before;
    if let Some(problem) = job.validate().into_iter().next() {
        return Err(refused(problem.text_message));
    }
    Ok((
        job,
        InlayMade {
            pocket: at,
            plug: plug_at,
            setup,
            part,
        },
    ))
}
