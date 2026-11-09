// SPDX-License-Identifier: GPL-3.0-or-later
//! `chipload frame …`: frames for signs, drawn round shapes or to a size, with border lines
//! (docs/specs/shape-library.md §3, §7).

use std::path::PathBuf;
use std::process::ExitCode;

use chipload_job::Job;
use chipload_pipeline::frames::{
    BORDER_LAYER, CornerStyle, FRAME_LAYER, FrameChange, FrameKind, FrameSize, MOST_BORDERS,
    NewFrame, add_frame, detach_frame, kind_of, place_frame, set_frame,
};
use clap::{Args, Subcommand, ValueEnum};
use serde_json::json;

use crate::load;
use crate::out::Out;

#[derive(Subcommand)]
pub enum FrameAction {
    /// The built-in frames, and what their corner size means.
    Kinds,
    /// Draw a frame round shapes (--around, --around-text) or to a size (--width and
    /// --height), with border lines inside it; prints the new shapes' ids.
    Add(Box<AddFrame>),
    /// The frames that follow what they go round: their shapes and values.
    List {
        /// The project (.chipload).
        project: PathBuf,
    },
    /// Change a frame's values (ID: its outline's or a border's shape id); it is drawn
    /// again with them.
    Set(Box<SetFrame>),
    /// Let a frame go: its shapes stay as they are and follow nothing from now on.
    Detach {
        /// The project (.chipload).
        project: PathBuf,
        /// Its outline's or a border's shape id.
        id: u64,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

/// `frame set`'s options.
#[derive(Args)]
pub struct SetFrame {
    /// The project (.chipload).
    project: PathBuf,
    /// Its outline's or a border's shape id.
    id: u64,
    /// Another frame kind (`frame kinds`).
    #[arg(long, value_enum)]
    kind: Option<KindArg>,
    /// The corner size, mm.
    #[arg(value_parser = crate::lengths::length, long)]
    corner: Option<f64>,
    /// From what it goes round to the innermost line, mm.
    #[arg(value_parser = crate::lengths::length, long)]
    margin: Option<f64>,
    /// Border lines inside the outline, 0 to 3.
    #[arg(long)]
    borders: Option<u32>,
    /// From the outline to the first border and between borders, mm.
    #[arg(value_parser = crate::lengths::length, long)]
    gap: Option<f64>,
    /// A rectangular frame's corners: sharp, round, notch, chamfer or step.
    #[arg(long, value_enum)]
    corners: Option<CornersArg>,
    /// The borders' own corners, inside a rectangular frame.
    #[arg(long, value_enum)]
    border_corners: Option<CornersArg>,
    /// The borders follow the outline's shape again.
    #[arg(long, conflicts_with = "border_corners")]
    border_corners_off: bool,
    /// Each border two lines (true) or one (false).
    #[arg(long)]
    double: Option<bool>,
    /// A border's groove width at the surface, mm.
    #[arg(value_parser = crate::lengths::length, long)]
    border_width: Option<f64>,
    /// A frame drawn to a size: its outline's width, mm.
    #[arg(value_parser = crate::lengths::length, long)]
    width: Option<f64>,
    /// A frame drawn to a size: its outline's height, mm.
    #[arg(value_parser = crate::lengths::length, long)]
    height: Option<f64>,
    /// Where to write the result; default: the project itself.
    #[arg(short, long)]
    output: Option<PathBuf>,
}

/// `frame add`'s options.
#[derive(Args)]
pub struct AddFrame {
    /// The project (.chipload).
    project: PathBuf,
    /// Which frame (`frame kinds`); default rectangle, with --corners.
    #[arg(long, value_enum)]
    kind: Option<KindArg>,
    /// Round these shapes (ids as `inspect` lists them, without the S): the frame is
    /// centred on their box, --margin from it to the innermost line.
    #[arg(long, num_args = 1.., value_delimiter = ',')]
    around: Vec<u64>,
    /// Round these texts' letters (ids as `text list` shows them).
    #[arg(long, num_args = 1.., value_delimiter = ',')]
    around_text: Vec<u64>,
    /// Round these traced pictures' shapes (ids as `image list` shows them); the frame
    /// follows a picture traced again.
    #[arg(long, num_args = 1.., value_delimiter = ',')]
    around_image: Vec<u64>,
    /// From the shapes' box to the innermost line, mm (default 10).
    #[arg(value_parser = crate::lengths::length, long)]
    margin: Option<f64>,
    /// The outline's width, mm, when not round shapes.
    #[arg(value_parser = crate::lengths::length, long)]
    width: Option<f64>,
    /// The outline's height, mm, when not round shapes.
    #[arg(value_parser = crate::lengths::length, long)]
    height: Option<f64>,
    /// Its centre, X,Y in mm (default: the stock's centre), with --width and --height.
    #[arg(long, value_parser = parse_at, allow_hyphen_values = true)]
    at: Option<(f64, f64)>,
    /// The corner size, mm (default: a tenth of the shorter side; `frame kinds` says
    /// what it is for each frame).
    #[arg(value_parser = crate::lengths::length, long)]
    corner: Option<f64>,
    /// Border lines inside the outline, 0 to 3.
    #[arg(long, default_value_t = 0)]
    borders: u32,
    /// From the outline to the first border and between borders, mm.
    #[arg(value_parser = crate::lengths::length, long, default_value = "5")]
    gap: f64,
    /// A rectangular frame's corners: sharp, round, notch, chamfer or step (with
    /// --kind rectangle, or instead of it).
    #[arg(long, value_enum)]
    corners: Option<CornersArg>,
    /// The borders' own corners, inside a rectangular frame (default: the outline's
    /// shape).
    #[arg(long, value_enum)]
    border_corners: Option<CornersArg>,
    /// Each border two lines, twice --border-width apart.
    #[arg(long)]
    double: bool,
    /// A border's groove width at the surface, mm: the margin is measured to its inner
    /// edge (default 0: lines).
    #[arg(value_parser = crate::lengths::length, long, default_value = "0")]
    border_width: f64,
    /// The outline's layer.
    #[arg(long, default_value = FRAME_LAYER)]
    layer: String,
    /// The borders' layer.
    #[arg(long, default_value = BORDER_LAYER)]
    border_layer: String,
    /// Where to write the result; default: the project itself.
    #[arg(short, long)]
    output: Option<PathBuf>,
}

#[derive(Clone, Copy, ValueEnum)]
pub enum KindArg {
    Rectangle,
    Rounded,
    Notched,
    Chamfered,
    Stepped,
    Oval,
    Arched,
    DoubleArch,
    Gable,
    Shield,
    Plaque,
    Banner,
    Octagon,
}

#[derive(Clone, Copy, ValueEnum)]
pub enum CornersArg {
    Sharp,
    Round,
    Notch,
    Chamfer,
    Step,
}

impl From<CornersArg> for CornerStyle {
    fn from(c: CornersArg) -> Self {
        match c {
            CornersArg::Sharp => CornerStyle::Sharp,
            CornersArg::Round => CornerStyle::Round,
            CornersArg::Notch => CornerStyle::Notch,
            CornersArg::Chamfer => CornerStyle::Chamfer,
            CornersArg::Step => CornerStyle::Step,
        }
    }
}

/// The frame `kind` with rectangular corners `corners`: refused for a kind that isn't
/// rectangular.
fn with_corners(kind: FrameKind, corners: Option<CornersArg>) -> Result<FrameKind, String> {
    match corners {
        None => Ok(kind),
        Some(c) if kind.corner_style().is_some() => Ok(CornerStyle::from(c).kind()),
        Some(_) => Err(format!(
            "--corners is for the rectangular frames, not {}",
            kind.name()
        )),
    }
}

impl From<KindArg> for FrameKind {
    fn from(k: KindArg) -> Self {
        match k {
            KindArg::Rectangle => FrameKind::Rectangle,
            KindArg::Rounded => FrameKind::Rounded,
            KindArg::Notched => FrameKind::Notched,
            KindArg::Chamfered => FrameKind::Chamfered,
            KindArg::Oval => FrameKind::Oval,
            KindArg::Arched => FrameKind::Arched,
            KindArg::Plaque => FrameKind::Plaque,
            KindArg::Stepped => FrameKind::Stepped,
            KindArg::DoubleArch => FrameKind::DoubleArch,
            KindArg::Gable => FrameKind::Gable,
            KindArg::Shield => FrameKind::Shield,
            KindArg::Banner => FrameKind::Banner,
            KindArg::Octagon => FrameKind::Octagon,
        }
    }
}

fn parse_at(text: &str) -> Result<(f64, f64), String> {
    crate::lengths::point(text).map(|[x, y]| (x, y))
}

/// What the corner size is for each frame, for `frame kinds`.
fn corner_meaning(kind: FrameKind) -> &'static str {
    match kind {
        FrameKind::Rectangle => "sharp corners (no corner size)",
        FrameKind::Rounded => "corners rounded by the corner size",
        FrameKind::Notched => "corners cut in by quarter circles of the corner size",
        FrameKind::Chamfered => "corners cut off at 45° by the corner size",
        FrameKind::Oval => "an ellipse touching the four sides (no corner size)",
        FrameKind::Arched => "the top an arc rising the corner size above the top corners",
        FrameKind::Plaque => "the short sides bulging out by the corner size",
        FrameKind::Stepped => "corners cut in by squares of the corner size",
        FrameKind::Gable => "a pointed top rising the corner size above the top corners",
        FrameKind::DoubleArch => "top and bottom arcs rising the corner size beyond the corners",
        FrameKind::Shield => {
            "a straight top, the sides curving into a point the corner size below them"
        }
        FrameKind::Banner => "the short ends cut in as a V the corner size deep",
        FrameKind::Octagon => "corners cut off at 45° to sides as long as the shorter ones",
    }
}

pub fn frame(out: &mut Out, action: FrameAction) -> ExitCode {
    match action {
        FrameAction::Kinds => {
            for kind in FrameKind::ALL {
                out.line(format!("{:<10} {}", kind.name(), corner_meaning(kind)));
            }
            out.set(
                "kinds",
                json!(FrameKind::ALL.iter().map(|k| k.name()).collect::<Vec<_>>()),
            );
            out.line(format!(
                "corners (--corners, --border-corners): {}",
                CornerStyle::ALL.map(CornerStyle::name).join(", ")
            ));
            out.set(
                "corners",
                json!(CornerStyle::ALL.map(CornerStyle::name).to_vec()),
            );
            out.set("most_borders", json!(MOST_BORDERS));
            ExitCode::SUCCESS
        }
        FrameAction::Add(add) => {
            let AddFrame {
                project,
                kind,
                around,
                around_text,
                around_image,
                margin,
                width,
                height,
                at,
                corner,
                borders,
                gap,
                corners,
                border_corners,
                double,
                border_width,
                layer,
                border_layer,
                output,
            } = *add;
            let kind = match with_corners(kind.map_or(FrameKind::Rectangle, Into::into), corners) {
                Ok(kind) => kind,
                Err(err) => return out.fail(err),
            };
            let job = match load(out, &project) {
                Ok(job) => job,
                Err(code) => return code,
            };
            let size = match frame_size(
                &job,
                around,
                &around_text,
                &around_image,
                margin,
                width,
                height,
                at,
            ) {
                Ok(size) => size,
                Err(err) => return out.fail(err),
            };
            let frame = NewFrame {
                kind,
                r: corner,
                borders,
                gap,
                border_corners: border_corners.map(Into::into),
                double,
                border_width,
                size,
                layer,
                border_layer,
            };
            let place = match place_frame(&job, &frame) {
                Ok(place) => place,
                Err(err) => return out.fail(err),
            };
            let edited = match add_frame(&job, &frame) {
                Ok(edited) => edited,
                Err(err) => return out.fail(err),
            };
            let target = output.as_deref().unwrap_or(&project);
            if let Err(err) = crate::save_project(out, &edited.job, target) {
                return out.fail(err);
            }
            let names: Vec<String> = edited.new_ids.iter().map(|id| format!("S{id}")).collect();
            out.line(format!(
                "Added frame {}, {} × {} mm, centred at {}, {}: shapes {}.",
                frame.kind.name(),
                mm(place.width),
                mm(place.height),
                mm(place.center.0),
                mm(place.center.1),
                names.join(", ")
            ));
            for note in &edited.notes {
                out.line(note.english());
            }
            out.wrote(target, "");
            out.set("ids", json!(edited.new_ids));
            out.set("outline", json!(edited.new_ids[0]));
            out.set("borders", json!(edited.new_ids[1..]));
            out.set("width", json!(place.width));
            out.set("height", json!(place.height));
            out.set("center", json!([place.center.0, place.center.1]));
            out.set("corner", json!(place.r));
            ExitCode::SUCCESS
        }
        FrameAction::List { project } => {
            let job = match load(out, &project) {
                Ok(job) => job,
                Err(code) => return code,
            };
            list(out, &job);
            ExitCode::SUCCESS
        }
        FrameAction::Set(set) => {
            let SetFrame {
                project,
                id,
                kind,
                corner,
                margin,
                borders,
                gap,
                corners,
                border_corners,
                border_corners_off,
                double,
                border_width,
                width,
                height,
                output,
            } = *set;
            let job = match load(out, &project) {
                Ok(job) => job,
                Err(code) => return code,
            };
            let now = job.drawing.frame_of(id).map(|f| kind_of(f.kind));
            let kind = match (kind.map(FrameKind::from).or(now), corners) {
                (Some(k), c) => match with_corners(k, c) {
                    Ok(k) => Some(k),
                    Err(err) => return out.fail(err),
                },
                (None, _) => None,
            };
            let change = FrameChange {
                kind,
                corner,
                margin,
                borders,
                gap,
                border_corners: if border_corners_off {
                    Some(None)
                } else {
                    border_corners.map(|c| Some(c.into()))
                },
                double,
                border_width,
                width,
                height,
            };
            let edited = match set_frame(&job, id, &change) {
                Ok(edited) => edited,
                Err(err) => return out.fail(err),
            };
            let target = output.as_deref().unwrap_or(&project);
            if let Err(err) = crate::save_project(out, &edited.job, target) {
                return out.fail(err);
            }
            if let Some(frame) = edited.job.drawing.frame_of(id).or_else(|| {
                // Its border went: the frame is still named by its outline.
                job.drawing
                    .frame_of(id)
                    .and_then(|f| edited.job.drawing.frame_of(f.outline))
            }) {
                out.line(format!(
                    "Frame S{} is now {}, {} × {} mm.",
                    frame.outline,
                    kind_of(frame.kind).name(),
                    mm(frame.width),
                    mm(frame.height)
                ));
                out.set("outline", json!(frame.outline));
                out.set("borders", json!(frame.borders));
                out.set("width", json!(frame.width));
                out.set("height", json!(frame.height));
            }
            for note in &edited.notes {
                out.line(note.english());
            }
            out.wrote(target, "");
            out.set("ids", json!(edited.new_ids));
            ExitCode::SUCCESS
        }
        FrameAction::Detach {
            project,
            id,
            output,
        } => {
            let job = match load(out, &project) {
                Ok(job) => job,
                Err(code) => return code,
            };
            let detached = match detach_frame(&job, id) {
                Ok(job) => job,
                Err(err) => return out.fail(err),
            };
            let target = output.as_deref().unwrap_or(&project);
            if let Err(err) = crate::save_project(out, &detached, target) {
                return out.fail(err);
            }
            out.line(format!(
                "The frame with shape S{id} is plain shapes now and follows nothing."
            ));
            out.wrote(target, "");
            ExitCode::SUCCESS
        }
    }
}

/// `frame list`: one line per live frame, and the records under `frames`.
fn list(out: &mut Out, job: &Job) {
    if job.drawing.frames.is_empty() {
        out.line("No frames follow what they go round.");
    }
    for frame in &job.drawing.frames {
        let borders: Vec<String> = frame.borders.iter().map(|b| format!("S{b}")).collect();
        let around = match &frame.around {
            Some(a) => {
                let mut parts: Vec<String> = Vec::new();
                parts.extend(a.texts.iter().map(|t| format!("text {t}")));
                parts.extend(a.images.iter().map(|i| format!("picture {i}")));
                parts.extend(a.shapes.iter().map(|s| format!("S{s}")));
                format!("round {}, margin {} mm", parts.join(", "), mm(frame.margin))
            }
            None => "to a size".to_string(),
        };
        out.line(format!(
            "S{} {} {} × {} mm, corner {} mm, borders [{}] gap {} mm, {}",
            frame.outline,
            kind_of(frame.kind).name(),
            mm(frame.width),
            mm(frame.height),
            mm(frame.corner),
            borders.join(", "),
            mm(frame.gap),
            around
        ));
    }
    out.set("frames", json!(job.drawing.frames));
}

/// A length for a line of output: two decimals at most.
fn mm(value: f64) -> String {
    let text = format!("{value:.2}");
    text.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// Round shapes or to a size, never both.
#[allow(clippy::too_many_arguments)]
fn frame_size(
    job: &Job,
    around: Vec<u64>,
    around_text: &[u64],
    around_image: &[u64],
    margin: Option<f64>,
    width: Option<f64>,
    height: Option<f64>,
    at: Option<(f64, f64)>,
) -> Result<FrameSize, String> {
    let around = around_ids(job, around, around_text, around_image)?;
    match (around.is_empty(), width, height) {
        (false, None, None) if at.is_none() => Ok(FrameSize::Around {
            ids: around,
            margin: margin.unwrap_or(10.0),
        }),
        (true, Some(width), Some(height)) if margin.is_none() => Ok(FrameSize::Fixed {
            width,
            height,
            center: at,
        }),
        _ => Err(
            "give either --around, --around-text or --around-image (with --margin), or \
             --width and --height (with --at)"
                .to_string(),
        ),
    }
}

/// Shapes `ids`, with the letters of texts `texts` and the traced shapes of pictures
/// `images`; a text or picture that isn't there is refused.
pub(crate) fn around_ids(
    job: &Job,
    mut ids: Vec<u64>,
    texts: &[u64],
    images: &[u64],
) -> Result<Vec<u64>, String> {
    for &id in texts {
        if !job.drawing.texts.iter().any(|t| t.id == id) {
            return Err(format!("there is no text {id}"));
        }
        ids.extend(
            job.drawing
                .shapes
                .iter()
                .filter(|s| s.text() == Some(id))
                .map(|s| s.id),
        );
    }
    for &id in images {
        if !job.drawing.images.iter().any(|i| i.id == id) {
            return Err(format!("there is no picture {id}"));
        }
        ids.extend(
            job.drawing
                .shapes
                .iter()
                .filter(|s| s.image() == Some(id))
                .map(|s| s.id),
        );
    }
    Ok(ids)
}
