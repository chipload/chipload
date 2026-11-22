// SPDX-License-Identifier: GPL-3.0-or-later
//! A sign in one step (docs/specs/sign-maker.md §3, §6): the words, a frame that follows
//! them, and every operation that cuts the sign, in a part of its own, the sign grouped.
//! A shortcut over what exists, as `add_inlay` is: afterwards everything is ordinary
//! shapes and operations.

use chipload_core::chain::Contour;
use chipload_core::tolerances::DEPTH_TOL_MM;
use chipload_i18n::{Digits, Language, Message, Units, Value};
use chipload_job::library::ToolLibrary;
use chipload_job::{
    ArcSide, EdgeSide, Job, Operation, Selector, Source, StockOrigin, TextAlign, TextAnchor,
    TextArc, Tool, ToolType,
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
/// How deep single-line words are engraved by default, mm.
pub const DEFAULT_ENGRAVE_DEPTH_MM: f64 = 1.0;
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
    /// Existing shapes to make the sign round (a traced picture, a text, drawn or DXF
    /// shapes), by id, instead of new words; empty: the words.
    pub around: Vec<u64>,
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
    /// Round words, with the round frame: the first line along the top of a circle, the
    /// second along its bottom, the rest straight in the middle
    /// (docs/specs/text-on-arc.md §6).
    pub round: bool,
    /// The frame's outer size, width and height (mm), instead of following the words: the
    /// letters are then as high as fit, and `height` is ignored
    /// (docs/specs/sign-size-and-start.md §1). Round shapes, the frame is that size round
    /// them.
    pub size: Option<(f64, f64)>,
    /// With a single-line font, how deep the follow path engraves the words, mm
    /// (docs/specs/single-line-text.md §2).
    pub engrave_depth: f64,
}

impl SignRequest {
    /// A sign of `words` with the defaults: FreeSans 30 mm, a rounded frame with one
    /// 2 mm border, engraved, no edge, an 18 mm board fitted round it.
    pub fn new(words: &str, vbit: i64, end_mill: i64) -> SignRequest {
        SignRequest {
            words: words.to_string(),
            around: Vec::new(),
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
            round: false,
            size: None,
            engrave_depth: DEFAULT_ENGRAVE_DEPTH_MM,
        }
    }

    /// New words in a single-line font: engraved along their strokes.
    pub(crate) fn single_line(&self) -> bool {
        self.around.is_empty()
            && chipload_text::builtin(&self.font)
                .is_some_and(chipload_text::BuiltinFont::single_line)
    }

    /// The frame as asked for, of `size`.
    pub(crate) fn frame(&self, size: FrameSize) -> NewFrame {
        let mut frame = NewFrame::new(self.kind, size);
        frame.r = self.corner;
        frame.borders = self.borders;
        frame.gap = self.gap;
        frame.border_corners = self.border_corners;
        frame.double = self.double;
        frame.border_width = self.border_width;
        frame
    }
}

/// What a sign goes round: its shapes, the selector its letters' operation cuts them
/// with, the group members they make, and the sign's name.
#[derive(Debug, Clone)]
struct Artwork {
    ids: Vec<u64>,
    select: Selector,
    members: Members,
    /// Every shape can be in the sign's group (texts and drawn shapes; a picture's or a
    /// DXF's can't).
    groupable: bool,
    name: String,
}

impl Artwork {
    /// Text `text`'s letters.
    fn of_text(job: &Job, text: u64, name: &str) -> Artwork {
        Artwork {
            ids: job
                .drawing
                .shapes
                .iter()
                .filter(|s| s.text() == Some(text))
                .map(|s| s.id)
                .collect(),
            select: Selector::of(Source::Text(text)),
            members: Members {
                shapes: Vec::new(),
                texts: vec![text],
            },
            groupable: true,
            name: name.to_string(),
        }
    }

    /// Texts `texts`' letters; the selector is set once the group is made.
    fn of_texts(job: &Job, texts: &[u64], name: &str) -> Artwork {
        Artwork {
            ids: job
                .drawing
                .shapes
                .iter()
                .filter(|s| s.text().is_some_and(|t| texts.contains(&t)))
                .map(|s| s.id)
                .collect(),
            select: Selector::default(),
            members: Members {
                shapes: Vec::new(),
                texts: texts.to_vec(),
            },
            groupable: true,
            name: name.to_string(),
        }
    }

    /// Shapes `ids`, which must be in the drawing.
    fn of(job: &Job, ids: &[u64], lang: Language) -> Result<Artwork, PipelineError> {
        let shapes: Vec<&chipload_job::Shape> = ids
            .iter()
            .map(|id| {
                job.drawing
                    .shapes
                    .iter()
                    .find(|s| s.id == *id)
                    .ok_or_else(|| crate::shapes::no_shape(*id))
            })
            .collect::<Result<_, _>>()?;
        // All of one picture, text or DXF: cut as that source, the shapes not chosen left
        // out (a traced picture's shapes can't be chosen by id, and it may be traced
        // again); otherwise by id.
        let first = shapes.first().and_then(|s| s.source());
        let one_source = first.filter(|from| shapes.iter().all(|s| s.source() == Some(*from)));
        let select = match one_source {
            Some(from) => Selector {
                source: Some(Source::from(from)),
                exclude: job
                    .drawing
                    .shapes
                    .iter()
                    .filter(|s| s.source() == Some(from) && !ids.contains(&s.id))
                    .map(|s| s.id)
                    .collect(),
                ..Selector::default()
            },
            None => Selector {
                shapes: Some(ids.to_vec()),
                ..Selector::default()
            },
        };
        let mut members = Members::default();
        let mut groupable = true;
        for shape in &shapes {
            if let Some(text) = shape.text() {
                if !members.texts.contains(&text) {
                    members.texts.push(text);
                }
            } else if shape.is_drawn() {
                members.shapes.push(shape.id);
            } else {
                groupable = false;
            }
        }
        let name = match one_source {
            Some(chipload_core::chain::ShapeFrom::Image(id)) => job
                .drawing
                .images
                .iter()
                .find(|i| i.id == id)
                .map(|i| file_stem(&i.file)),
            Some(chipload_core::chain::ShapeFrom::Text(id)) => job
                .drawing
                .texts
                .iter()
                .find(|t| t.id == id)
                .map(|t| sign_name(&t.text)),
            Some(chipload_core::chain::ShapeFrom::Dxf(id)) => job
                .drawing
                .dxfs
                .iter()
                .find(|d| d.id == id)
                .map(|d| d.name.clone()),
            _ => None,
        }
        .filter(|n| !n.trim().is_empty())
        .unwrap_or_else(|| Message::new("pipeline-name-sign").render(lang, Units::Mm));
        Ok(Artwork {
            ids: ids.to_vec(),
            select,
            members,
            groupable,
            name,
        })
    }
}

/// How far round its circle a round sign's longer line goes at most, degrees: the rest of
/// the ring stays clear between the top and bottom words.
const ROUND_WORDS_SPAN_DEG: f64 = 150.0;

/// A round sign's words (docs/specs/text-on-arc.md §6): the first line along the top of a
/// circle round `center`, the second along its bottom, on one radius, the rest straight in
/// the middle. The radius is the least that keeps each arc line within
/// [`ROUND_WORDS_SPAN_DEG`] and the middle clear of the ring. Returns the texts' ids.
pub(crate) fn round_words(
    job: &Job,
    request: &SignRequest,
    center: (f64, f64),
    layer: &str,
    fonts: &dyn FontSource,
) -> Result<(Job, Vec<u64>), PipelineError> {
    let lines: Vec<&str> = request
        .words
        .split('\n')
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    let h = request.height;
    let straight = |words: &str| -> Result<(f64, f64), PipelineError> {
        let mut new = NewText::new(words, h);
        new.font = &request.font;
        new.align = TextAlign::Center;
        let (_, report) = add_text(&crate::empty_project(), new, fonts)?;
        Ok((report.width, report.height))
    };
    let span = ROUND_WORDS_SPAN_DEG.to_radians();
    let mut radius = 2.0 * h;
    for line in lines.iter().take(2) {
        radius = radius.max(straight(line)?.0 / span);
    }
    let middle = lines
        .get(2..)
        .map(|rest| rest.join("\n"))
        .filter(|m| !m.is_empty());
    if let Some(middle) = &middle {
        let (w, m) = straight(middle)?;
        radius = radius.max(chipload_core::math::hypot(w / 2.0, m / 2.0) + h);
    }
    let mut job = job.clone();
    let mut texts = Vec::new();
    let mut add = |job: &mut Job, words: &str, arc: Option<TextArc>| -> Result<(), PipelineError> {
        let mut new = NewText::new(words, h);
        new.font = &request.font;
        (new.x, new.y) = center;
        new.anchor = TextAnchor::Center;
        new.align = TextAlign::Center;
        new.arc = arc;
        new.layer = layer;
        let (with, report) = add_text(job, new, fonts)?;
        *job = with;
        texts.push(report.id);
        Ok(())
    };
    for (line, side) in lines.iter().zip([ArcSide::Top, ArcSide::Bottom]) {
        let arc = TextArc {
            radius,
            side,
            angle: side.default_angle(),
        };
        add(&mut job, line, Some(arc))?;
    }
    if let Some(middle) = &middle {
        add(&mut job, middle, None)?;
    }
    Ok((job, texts))
}

/// The lowest letters' height tried for a sign of a fixed size, mm.
const FIT_MIN_HEIGHT_MM: f64 = 1.0;
/// How close the search for the letters' height gets before rounding down, mm.
const FIT_STEP_MM: f64 = 0.05;

/// A fixed size that can be a sign's: positive, and round for round words.
pub(crate) fn check_size(
    request: &SignRequest,
    (width, height): (f64, f64),
) -> Result<(), PipelineError> {
    if !(width.is_finite() && width > 0.0 && height.is_finite() && height > 0.0) {
        return Err(refused(
            Message::new("pipeline-sign-size")
                .arg("width", length(width))
                .arg("height", length(height)),
        ));
    }
    if request.round && (width - height).abs() > chipload_core::tolerances::CHAIN_TOL_MM {
        return Err(refused(Message::new("pipeline-sign-round-size")));
    }
    Ok(())
}

/// The largest letters' height, to 0.1 mm, whose sign fits `size` (spec §1): halving
/// between [`FIT_MIN_HEIGHT_MM`] and the size, each try a sign of its own in an empty
/// project with its frame round the words, compared with the size. A sign grows with its
/// letters, so the search needs no formula for each frame.
pub(crate) fn fitted_height(
    request: &SignRequest,
    size: (f64, f64),
    fonts: &dyn FontSource,
) -> Result<f64, PipelineError> {
    check_size(request, size)?;
    let slack = chipload_core::tolerances::CHAIN_TOL_MM;
    let fits = |height: f64| -> Result<bool, PipelineError> {
        let (w, h) = framed_size(request, height, fonts)?;
        Ok(w <= size.0 + slack && h <= size.1 + slack)
    };
    let too_small = || {
        refused(
            Message::new("pipeline-sign-size-small")
                .arg("width", length(size.0))
                .arg("height", length(size.1)),
        )
    };
    if !fits(FIT_MIN_HEIGHT_MM).map_err(|_| too_small())? {
        return Err(too_small());
    }
    let mut low = FIT_MIN_HEIGHT_MM;
    let mut high = size.0.max(size.1).min(chipload_job::TEXT_MAX_HEIGHT_MM);
    if fits(high).unwrap_or(false) {
        return Ok(high);
    }
    while high - low > FIT_STEP_MM {
        let middle = (low + high) / 2.0;
        if fits(middle).unwrap_or(false) {
            low = middle;
        } else {
            high = middle;
        }
    }
    let rounded = ((low * 10.0).floor() / 10.0).max(FIT_MIN_HEIGHT_MM);
    Ok(rounded)
}

/// The outer size of the frame round `request`'s words at letters `height`, mm.
pub(crate) fn framed_size(
    request: &SignRequest,
    height: f64,
    fonts: &dyn FontSource,
) -> Result<(f64, f64), PipelineError> {
    let tried = SignRequest {
        height,
        ..request.clone()
    };
    let scratch = crate::empty_project();
    let (job, ids) = if tried.round {
        let (job, texts) = round_words(
            &scratch,
            &tried,
            (0.0, 0.0),
            crate::texts::DEFAULT_LAYER,
            fonts,
        )?;
        let ids: Vec<u64> = job
            .drawing
            .shapes
            .iter()
            .filter(|s| s.text().is_some_and(|t| texts.contains(&t)))
            .map(|s| s.id)
            .collect();
        (job, ids)
    } else {
        let mut new = NewText::new(&tried.words, height);
        new.font = &tried.font;
        new.anchor = TextAnchor::Center;
        new.align = TextAlign::Center;
        let (job, _) = add_text(&scratch, new, fonts)?;
        let ids = job.drawing.shapes.iter().map(|s| s.id).collect();
        (job, ids)
    };
    let framed = add_frame(
        &job,
        &tried.frame(FrameSize::Around {
            ids,
            margin: tried.margin,
        }),
    )?;
    let outline = framed.new_ids.first().copied().unwrap_or(0);
    let ((x0, y0), (x1, y1)) = crate::shapes::drawn_box(&framed.job, &[outline])
        .ok_or_else(|| refused(Message::new("pipeline-shapes-no-extent")))?;
    Ok((x1 - x0, y1 - y0))
}

/// A file's name without its folders and extension.
fn file_stem(file: &str) -> String {
    std::path::Path::new(file)
        .file_stem()
        .map_or_else(|| file.to_string(), |s| s.to_string_lossy().into_owned())
}

/// What *New sign…* made.
#[derive(Debug, Clone, PartialEq)]
pub struct SignMade {
    /// The sign's part, now the active one.
    pub part: u64,
    /// The words' text; None for a sign round existing shapes.
    pub text: Option<u64>,
    /// The frame's outline (the frame is named by it), and its border lines.
    pub outline: u64,
    pub borders: Vec<u64>,
    /// The group of the text and the frame.
    pub group: u64,
    /// The operations made, by index, in cutting order (an inlay's plug among them).
    pub operations: Vec<usize>,
    /// The inlay's plug setup.
    pub plug_setup: Option<u64>,
    /// The letters' height found for a sign of a fixed size, mm.
    pub fitted: Option<f64>,
}

pub(crate) fn refused(message: Message) -> PipelineError {
    PipelineError::Job(message)
}

pub(crate) fn length(mm: f64) -> Value {
    Value::Length(mm, Digits::Shortest)
}

/// The sign's name: its first line, at most [`NAME_LENGTH`] characters.
pub(crate) fn sign_name(words: &str) -> String {
    let line = words.lines().find(|l| !l.trim().is_empty()).unwrap_or("");
    line.trim().chars().take(NAME_LENGTH).collect()
}

/// The tools a sign needs, or why they won't do.
pub(crate) fn check_tools(job: &Job, request: &SignRequest) -> Result<(), PipelineError> {
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
pub(crate) fn check_values(request: &SignRequest) -> Result<(), PipelineError> {
    if request.around.is_empty() && sign_name(&request.words).is_empty() {
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
    if request.single_line() {
        if request.style != SignStyle::Engraved {
            return Err(refused(Message::new("pipeline-sign-single-line-style")));
        }
        if !(request.engrave_depth.is_finite() && request.engrave_depth > 0.0) {
            return Err(refused(
                Message::new("pipeline-sign-engrave-depth")
                    .arg("got", length(request.engrave_depth)),
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
    if request.round && (request.kind != FrameKind::Oval || !request.around.is_empty()) {
        return Err(refused(Message::new("pipeline-sign-round-oval")));
    }
    // A fixed size: the letters as high as fit (docs/specs/sign-size-and-start.md §1).
    let fitted = match request.size {
        Some(size) if request.around.is_empty() => Some(fitted_height(request, size, fonts)?),
        Some(size) => {
            check_size(request, size)?;
            None
        }
        None => None,
    };
    let sized;
    let request = match fitted {
        Some(height) => {
            sized = SignRequest {
                height,
                ..request.clone()
            };
            &sized
        }
        None => request,
    };
    let art = match request.around.is_empty() {
        true => None,
        false => Some(Artwork::of(job, &request.around, lang)?),
    };
    if request.style == SignStyle::Raised && art.as_ref().is_some_and(|a| !a.groupable) {
        return Err(refused(Message::new("pipeline-sign-raised-imported")));
    }
    let name = match &art {
        Some(art) => art.name.clone(),
        None => sign_name(&request.words),
    };
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
    // The words, centred on the stock; or the shapes already there.
    let (job, text, mut art) = match art {
        Some(art) => (job, None, art),
        None if request.round => {
            let (job, texts) =
                round_words(&job, request, center, crate::texts::DEFAULT_LAYER, fonts)?;
            let art = Artwork::of_texts(&job, &texts, &name);
            (job, texts.first().copied(), art)
        }
        None => {
            let mut new = NewText::new(&request.words, request.height);
            new.font = &request.font;
            (new.x, new.y) = center;
            new.anchor = TextAnchor::Center;
            new.align = TextAlign::Center;
            let (job, _) = add_text(&job, new, fonts)?;
            let text = job.drawing.texts.iter().map(|t| t.id).max().unwrap_or(0);
            let art = Artwork::of_text(&job, text, &name);
            (job, Some(text), art)
        }
    };
    let size = match request.size {
        // Centred on the round words' ring, or on the letters' (or shapes') box.
        Some((width, height)) => FrameSize::Fixed {
            width,
            height,
            center: if request.round {
                Some(center)
            } else {
                crate::shapes::drawn_box(&job, &art.ids)
                    .map(|((x0, y0), (x1, y1))| ((x0 + x1) / 2.0, (y0 + y1) / 2.0))
            },
        },
        None => FrameSize::Around {
            ids: art.ids.clone(),
            margin: request.margin,
        },
    };
    let framed = add_frame(&job, &request.frame(size))?;
    let outline = framed.new_ids[0];
    let borders = framed.new_ids[1..].to_vec();
    let mut members = art.members.clone();
    members.shapes.extend(framed.new_ids.iter().copied());
    let (mut job, group_id) = group(&framed.job, &members, Some(&name))?;
    // Round words are several texts: cut as the group without the frame's lines.
    if request.round {
        art.select = Selector {
            source: Some(Source::Group(group_id)),
            exclude: framed.new_ids.clone(),
            ..Selector::default()
        };
    }
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
        // Single-line words: one pass of the V-bit down each stroke.
        SignStyle::Engraved if request.single_line() => {
            let mut op = ctx.make(&job, OpKind::Path, art.select.clone(), request.vbit)?;
            set_depth(&mut op, request.engrave_depth);
            *op.name_mut() = op_name("pipeline-name-sign-letters");
            operations.push(ctx.append(&mut job, op));
        }
        SignStyle::Engraved => {
            let mut op = ctx.make(&job, OpKind::Vcarve, art.select.clone(), request.vbit)?;
            *op.name_mut() = op_name(if text.is_some() {
                "pipeline-name-sign-letters"
            } else {
                "pipeline-name-sign-carving"
            });
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
                select: art.select.clone(),
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
        // A Chamfer operation: the bit's flank on the outline, as wide as asked
        // (docs/specs/chamfer.md §6).
        let widest = bit.as_ref().and_then(Tool::widest_chamfer).unwrap_or(0.0);
        if edge > widest + DEPTH_TOL_MM {
            return Err(refused(
                Message::new("pipeline-sign-edge-wide")
                    .arg("width", length(edge))
                    .arg("tool", request.vbit)
                    .arg("widest", length(widest)),
            ));
        }
        let mut op = ctx.make(&job, OpKind::Chamfer, outline_only.clone(), request.vbit)?;
        *op.name_mut() = op_name("pipeline-name-sign-edge");
        if let Operation::Chamfer(c) = &mut op {
            c.side = EdgeSide::Outside;
            c.width = edge;
            c.start_depth = 0.0;
        }
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
            fitted,
        },
    ))
}

/// What every new operation of the sign is made with.
pub(crate) struct Ctx<'a> {
    pub(crate) contours: &'a [Contour],
    pub(crate) library: Option<&'a ToolLibrary>,
    pub(crate) lang: Language,
    pub(crate) part: u64,
}

impl Ctx<'_> {
    /// A new operation of `kind` on `select` with `tool`, the app's defaults and the
    /// library's feeds, in the sign's part.
    pub(crate) fn make(
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
    pub(crate) fn append(&self, job: &mut Job, op: Operation) -> usize {
        let at = job.insert_index(self.part);
        job.operations.insert(at, op);
        at
    }
}

/// A follow path's depth: from the top to `depth`, in its passes.
pub(crate) fn set_depth(op: &mut Operation, depth: f64) {
    if let Operation::Path(p) = op {
        p.start_depth = 0.0;
        p.end_depth = depth;
        p.through = None;
        if p.step_down > depth {
            p.step_down = depth;
        }
    }
}
