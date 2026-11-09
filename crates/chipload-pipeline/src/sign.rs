// SPDX-License-Identifier: GPL-3.0-or-later
//! A sign in one step (docs/specs/sign-maker.md §3, §6): the words, a frame that follows
//! them, and every operation that cuts the sign, in a part of its own, the sign grouped.
//! A shortcut over what exists, as `add_inlay` is: afterwards everything is ordinary
//! shapes and operations.

use chipload_core::chain::Contour;
use chipload_i18n::{Digits, Language, Message, Units, Value};
use chipload_job::library::ToolLibrary;
use chipload_job::{
    Job, Operation, Selector, Source, StockOrigin, TextAlign, TextAnchor, ToolType,
};

use crate::PipelineError;
use crate::fonts::FontSource;
use crate::frames::{CornerStyle, FrameKind, FrameSize, NewFrame, add_frame};
use crate::groups::{Members, group};
use crate::inlay::{self, InlayRequest};
use crate::new_ops::{NewRequest, OpKind, new_operations};
use crate::texts::{NewText, add_text};

/// A raised sign's background is cut this deep when not given, mm (decision 7).
pub const DEFAULT_RELIEF_MM: f64 = 3.0;
/// A border's groove width when not given, mm.
pub const DEFAULT_BORDER_WIDTH_MM: f64 = 2.0;
/// The board round the sign when not given, mm: room for the end mill and its tabs.
pub const DEFAULT_BOARD_MARGIN_MM: f64 = 15.0;
/// The board's thickness when not given, mm.
pub const DEFAULT_BOARD_THICKNESS_MM: f64 = 18.0;
/// The longest a sign's part and group name is, in characters.
const NAME_LENGTH: usize = 40;

/// How the words are cut (§3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignStyle {
    /// V-carved letters, grooved borders.
    Engraved,
    /// The background inside the innermost border cut down flat round the letters, which
    /// stand, with the border.
    Raised,
    /// The letters as a V-carve inlay (docs/specs/vcarve-inlay.md), the plug in a board of
    /// its own.
    Inlay,
}

impl SignStyle {
    pub const ALL: [SignStyle; 3] = [SignStyle::Engraved, SignStyle::Raised, SignStyle::Inlay];

    /// Its name on the command line.
    pub fn name(self) -> &'static str {
        match self {
            SignStyle::Engraved => "engraved",
            SignStyle::Raised => "raised",
            SignStyle::Inlay => "inlay",
        }
    }
}

/// The board the sign is cut from: the stock fitted round the sign.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SignBoard {
    pub thickness: f64,
    /// From the sign's outline to the board's edges, mm.
    pub margin: f64,
}

/// What *New sign…* asks (§3).
#[derive(Debug, Clone, PartialEq)]
pub struct SignRequest {
    pub words: String,
    pub font: String,
    /// The capital height, mm.
    pub height: f64,
    pub kind: FrameKind,
    /// The frame's corner size, mm; None: a tenth of its shorter side.
    pub corner: Option<f64>,
    /// From the words to the inner edge of the innermost border's groove, mm.
    pub margin: f64,
    pub borders: u32,
    pub gap: f64,
    pub border_corners: Option<CornerStyle>,
    pub double: bool,
    /// The borders' groove width at the surface, mm: their V-grooves' depth comes from it.
    pub border_width: f64,
    pub style: SignStyle,
    /// A raised sign's background depth, mm.
    pub relief: f64,
    /// A chamfer this wide at the surface round the outline with the V-bit, mm (§6).
    pub edge: Option<f64>,
    pub vbit: i64,
    pub end_mill: i64,
    /// The stock fitted round the sign; None: left as it is.
    pub board: Option<SignBoard>,
}

impl SignRequest {
    /// A sign of `words` with the defaults: FreeSans 30 mm, a rounded frame with one
    /// 2 mm border, engraved, no edge, an 18 mm board fitted round it.
    pub fn new(words: &str, vbit: i64, end_mill: i64) -> SignRequest {
        SignRequest {
            words: words.to_string(),
            font: crate::texts::DEFAULT_FONT.to_string(),
            height: 30.0,
            kind: FrameKind::Rounded,
            corner: None,
            margin: 10.0,
            borders: 1,
            gap: 5.0,
            border_corners: None,
            double: false,
            border_width: DEFAULT_BORDER_WIDTH_MM,
            style: SignStyle::Engraved,
            relief: DEFAULT_RELIEF_MM,
            edge: None,
            vbit,
            end_mill,
            board: Some(SignBoard {
                thickness: DEFAULT_BOARD_THICKNESS_MM,
                margin: DEFAULT_BOARD_MARGIN_MM,
            }),
        }
    }
}

/// What *New sign…* made.
#[derive(Debug, Clone, PartialEq)]
pub struct SignMade {
    /// The sign's part, now the active one.
    pub part: u64,
    pub text: u64,
    /// The frame's outline (the frame is named by it), and its border lines.
    pub outline: u64,
    pub borders: Vec<u64>,
    /// The group of the text and the frame.
    pub group: u64,
    /// The operations made, by index, in cutting order (an inlay's plug among them).
    pub operations: Vec<usize>,
    /// The inlay's plug setup.
    pub plug_setup: Option<u64>,
}

fn refused(message: Message) -> PipelineError {
    PipelineError::Job(message)
}

fn length(mm: f64) -> Value {
    Value::Length(mm, Digits::Shortest)
}

/// The sign's name: its first line, at most [`NAME_LENGTH`] characters.
fn sign_name(words: &str) -> String {
    let line = words.lines().find(|l| !l.trim().is_empty()).unwrap_or("");
    line.trim().chars().take(NAME_LENGTH).collect()
}

/// The tools a sign needs, or why they won't do.
fn check_tools(job: &Job, request: &SignRequest) -> Result<(), PipelineError> {
    let vbit = job
        .tool(request.vbit)
        .ok_or_else(|| refused(Message::new("pipeline-no-tool").arg("tool", request.vbit)))?;
    if vbit.kind != ToolType::Vbit || vbit.angle.is_none() {
        return Err(refused(
            Message::new("pipeline-sign-vbit").arg("tool", request.vbit),
        ));
    }
    let mill = job
        .tool(request.end_mill)
        .ok_or_else(|| refused(Message::new("pipeline-no-tool").arg("tool", request.end_mill)))?;
    if mill.kind != ToolType::Endmill {
        return Err(refused(
            Message::new("pipeline-sign-end-mill").arg("tool", request.end_mill),
        ));
    }
    Ok(())
}

/// What else must hold before anything is made.
fn check_values(request: &SignRequest) -> Result<(), PipelineError> {
    if sign_name(&request.words).is_empty() {
        return Err(refused(Message::new("pipeline-sign-words")));
    }
    if request.borders > 0 && !(request.border_width.is_finite() && request.border_width > 0.0) {
        return Err(refused(
            Message::new("pipeline-sign-border-width").arg("got", length(request.border_width)),
        ));
    }
    if request.style == SignStyle::Raised {
        if request.borders == 0 {
            return Err(refused(Message::new("pipeline-sign-raised-border")));
        }
        if !(request.relief.is_finite() && request.relief > 0.0) {
            return Err(refused(
                Message::new("pipeline-sign-relief").arg("got", length(request.relief)),
            ));
        }
    }
    if let Some(edge) = request.edge
        && !(edge.is_finite() && edge > 0.0)
    {
        return Err(refused(
            Message::new("pipeline-sign-edge").arg("got", length(edge)),
        ));
    }
    if let Some(board) = request.board
        && !(board.thickness.is_finite()
            && board.thickness > 0.0
            && board.margin.is_finite()
            && board.margin >= 0.0)
    {
        return Err(refused(Message::new("pipeline-sign-board")));
    }
    Ok(())
}

/// Make the sign (§3): the words centred on the stock in a new part named after them (the
/// active part when it is empty),
/// the frame round them following them, the sign grouped, and its operations in cutting
/// order with feeds from the library: the letters (V-carved, the background cleared round
/// them, or an inlay), the borders' V-grooves as deep as their width needs, the edge
/// chamfer, and the profile that cuts the sign out. With `board`, the stock is fitted
/// round the drawing. One step for the caller's undo.
pub fn add_sign(
    job: &Job,
    request: &SignRequest,
    fonts: &dyn FontSource,
    library: Option<&ToolLibrary>,
    lang: Language,
) -> Result<(Job, SignMade), PipelineError> {
    check_values(request)?;
    check_tools(job, request)?;
    let name = sign_name(&request.words);
    let ((x0, y0), (x1, y1)) = job.stock_bounds();
    let center = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);
    // An empty active part (a new project's) becomes the sign's; otherwise a new one.
    let empty = job
        .active_part
        .filter(|id| !job.operations.iter().any(|op| op.part() == *id));
    let (mut job, part) = match empty {
        Some(id) => {
            let mut out = job.clone();
            let free = crate::parts::free_name(&out, Some(&name)).ok();
            if let (Some(free), Some(p)) = (free, out.parts.iter_mut().find(|p| p.id == id)) {
                p.name = free;
            }
            (out, id)
        }
        None => crate::parts::add_part(job, Some(&name))
            .or_else(|_| crate::parts::add_part(job, None))?,
    };
    if let Some(board) = request.board {
        let stock = job.stock_mut();
        stock.thickness = board.thickness;
        stock.origin = StockOrigin::Drawing;
        stock.fit = Some(board.margin);
    }
    let mut new = NewText::new(&request.words, request.height);
    new.font = &request.font;
    (new.x, new.y) = center;
    new.anchor = TextAnchor::Center;
    new.align = TextAlign::Center;
    let (job, _) = add_text(&job, new, fonts)?;
    let text = job.drawing.texts.iter().map(|t| t.id).max().unwrap_or(0);
    let letters: Vec<u64> = job
        .drawing
        .shapes
        .iter()
        .filter(|s| s.text() == Some(text))
        .map(|s| s.id)
        .collect();
    let mut frame = NewFrame::new(
        request.kind,
        FrameSize::Around {
            ids: letters,
            margin: request.margin,
        },
    );
    frame.r = request.corner;
    frame.borders = request.borders;
    frame.gap = request.gap;
    frame.border_corners = request.border_corners;
    frame.double = request.double;
    frame.border_width = request.border_width;
    let framed = add_frame(&job, &frame)?;
    let outline = framed.new_ids[0];
    let borders = framed.new_ids[1..].to_vec();
    let members = Members {
        shapes: framed.new_ids.clone(),
        texts: vec![text],
    };
    let (mut job, group_id) = group(&framed.job, &members, Some(&name))?;
    let contours = crate::load_drawing(&job)?.contours;
    let mut operations = Vec::new();
    let mut plug_setup = None;
    let op_name = |id: &'static str| Message::new(id).render(lang, Units::Mm);
    let ctx = Ctx {
        contours: &contours,
        library,
        lang,
        part,
    };
    match request.style {
        SignStyle::Engraved => {
            let mut op = ctx.make(
                &job,
                OpKind::Vcarve,
                Selector::of(Source::Text(text)),
                request.vbit,
            )?;
            *op.name_mut() = op_name("pipeline-name-sign-letters");
            operations.push(ctx.append(&mut job, op));
        }
        SignStyle::Raised => {
            // Inside the innermost line, round the letters: the group without the outline
            // and the other border lines.
            let innermost = *borders.last().unwrap_or(&outline);
            let exclude: Vec<u64> = framed
                .new_ids
                .iter()
                .copied()
                .filter(|id| *id != innermost)
                .collect();
            let select = Selector {
                source: Some(Source::Group(group_id)),
                exclude,
                ..Selector::default()
            };
            let mut op = ctx.make(&job, OpKind::Pocket, select, request.end_mill)?;
            *op.name_mut() = op_name("pipeline-name-sign-background");
            if let Operation::Pocket(p) = &mut op {
                p.start_depth = 0.0;
                p.end_depth = request.relief;
                p.through = None;
            }
            operations.push(ctx.append(&mut job, op));
        }
        SignStyle::Inlay => {
            let inlay = InlayRequest {
                select: Selector::of(Source::Text(text)),
                tool: request.vbit,
                depth: inlay::DEFAULT_POCKET_DEPTH_MM,
                plug_start: inlay::DEFAULT_PLUG_START_MM,
                gap: inlay::DEFAULT_GLUE_GAP_MM,
                floor_tool: Some(request.end_mill),
                plug_thickness: inlay::DEFAULT_PLUG_THICKNESS_MM,
            };
            let (with, made) = inlay::add_inlay(&job, &inlay, &contours, library, lang)?;
            job = with;
            operations.push(made.pocket);
            plug_setup = Some(made.setup);
        }
    }
    let bit = job.tool(request.vbit).cloned();
    if !borders.is_empty() {
        let depth = bit
            .as_ref()
            .and_then(|b| b.vbit_depth_for(request.border_width))
            .ok_or_else(|| {
                refused(
                    Message::new("pipeline-sign-groove-wide")
                        .arg("width", length(request.border_width))
                        .arg("tool", request.vbit),
                )
            })?;
        let select = Selector {
            shapes: Some(borders.clone()),
            ..Selector::default()
        };
        let mut op = ctx.make(&job, OpKind::Path, select, request.vbit)?;
        *op.name_mut() = op_name("pipeline-name-sign-borders");
        set_depth(&mut op, depth);
        operations.push(ctx.append(&mut job, op));
    }
    let outline_only = Selector {
        shapes: Some(vec![outline]),
        ..Selector::default()
    };
    if let Some(edge) = request.edge {
        // Centred on the outline: the chamfer on the sign's side is half the groove.
        let depth = bit
            .as_ref()
            .and_then(|b| b.vbit_depth_for(2.0 * edge))
            .ok_or_else(|| {
                refused(
                    Message::new("pipeline-sign-edge-wide")
                        .arg("width", length(edge))
                        .arg("tool", request.vbit),
                )
            })?;
        let mut op = ctx.make(&job, OpKind::Path, outline_only.clone(), request.vbit)?;
        *op.name_mut() = op_name("pipeline-name-sign-edge");
        set_depth(&mut op, depth);
        operations.push(ctx.append(&mut job, op));
    }
    let mut op = ctx.make(&job, OpKind::Profile, outline_only, request.end_mill)?;
    *op.name_mut() = op_name("pipeline-name-sign-cut-out");
    operations.push(ctx.append(&mut job, op));
    // The indices as they end up: each insertion after an earlier one leaves it be, and
    // the inlay's plug goes in a later setup's part.
    let made_ops: Vec<String> = operations
        .iter()
        .map(|i| format!("operations.{i}"))
        .collect();
    let ours = |path: &str| {
        made_ops
            .iter()
            .any(|own| path == own || path.starts_with(&format!("{own}.")))
    };
    if let Some(problem) = job.validate().into_iter().find(|p| ours(&p.path)) {
        return Err(refused(problem.text_message));
    }
    job.active_part = Some(part);
    Ok((
        job,
        SignMade {
            part,
            text,
            outline,
            borders,
            group: group_id,
            operations,
            plug_setup,
        },
    ))
}

/// What every new operation of the sign is made with.
struct Ctx<'a> {
    contours: &'a [Contour],
    library: Option<&'a ToolLibrary>,
    lang: Language,
    part: u64,
}

impl Ctx<'_> {
    /// A new operation of `kind` on `select` with `tool`, the app's defaults and the
    /// library's feeds, in the sign's part.
    fn make(
        &self,
        job: &Job,
        kind: OpKind,
        select: Selector,
        tool: i64,
    ) -> Result<Operation, PipelineError> {
        let request = NewRequest {
            kind,
            select,
            tool: Some(tool),
        };
        let made = new_operations(job, &request, self.contours, self.library, self.lang);
        let mut op = made
            .ops
            .into_iter()
            .next()
            .map(|n| n.op)
            .ok_or_else(|| refused(Message::new("pipeline-sign-words")))?;
        op.set_part(self.part);
        Ok(op)
    }

    /// `op` after the sign's other operations; its index.
    fn append(&self, job: &mut Job, op: Operation) -> usize {
        let at = job.insert_index(self.part);
        job.operations.insert(at, op);
        at
    }
}

/// A follow path's depth: from the top to `depth`, in its passes.
fn set_depth(op: &mut Operation, depth: f64) {
    if let Operation::Path(p) = op {
        p.start_depth = 0.0;
        p.end_depth = depth;
        p.through = None;
        if p.step_down > depth {
            p.step_down = depth;
        }
    }
}
