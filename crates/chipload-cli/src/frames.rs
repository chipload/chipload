// SPDX-License-Identifier: GPL-3.0-or-later
//! `chipload frame …`: frames for signs, drawn round shapes or to a size, with border lines
//! (docs/specs/shape-library.md §3, §7).

use std::path::PathBuf;
use std::process::ExitCode;

use chipload_job::{Job, save_job};
use chipload_pipeline::frames::{
    BORDER_LAYER, FRAME_LAYER, FrameKind, FrameSize, MOST_BORDERS, NewFrame, add_frame, place_frame,
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
}

/// `frame add`'s options.
#[derive(Args)]
pub struct AddFrame {
    /// The project (.chipload).
    project: PathBuf,
    /// Which frame (`frame kinds`).
    #[arg(long, value_enum)]
    kind: KindArg,
    /// Round these shapes (ids as `inspect` lists them, without the S): the frame is
    /// centred on their box, --margin from it to the innermost line.
    #[arg(long, num_args = 1.., value_delimiter = ',')]
    around: Vec<u64>,
    /// Round these texts' letters (ids as `text list` shows them).
    #[arg(long, num_args = 1.., value_delimiter = ',')]
    around_text: Vec<u64>,
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
    Oval,
    Arched,
    Plaque,
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
            out.set("most_borders", json!(MOST_BORDERS));
            ExitCode::SUCCESS
        }
        FrameAction::Add(add) => {
            let AddFrame {
                project,
                kind,
                around,
                around_text,
                margin,
                width,
                height,
                at,
                corner,
                borders,
                gap,
                layer,
                border_layer,
                output,
            } = *add;
            let job = match load(out, &project) {
                Ok(job) => job,
                Err(code) => return code,
            };
            let size = match frame_size(&job, around, &around_text, margin, width, height, at) {
                Ok(size) => size,
                Err(err) => return out.fail(err),
            };
            let frame = NewFrame {
                kind: kind.into(),
                r: corner,
                borders,
                gap,
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
            if let Err(err) = save_job(&edited.job, target) {
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
    }
}

/// A length for a line of output: two decimals at most.
fn mm(value: f64) -> String {
    let text = format!("{value:.2}");
    text.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// Round shapes or to a size, never both.
fn frame_size(
    job: &Job,
    mut around: Vec<u64>,
    around_text: &[u64],
    margin: Option<f64>,
    width: Option<f64>,
    height: Option<f64>,
    at: Option<(f64, f64)>,
) -> Result<FrameSize, String> {
    for &id in around_text {
        if !job.drawing.texts.iter().any(|t| t.id == id) {
            return Err(format!("there is no text {id}"));
        }
        around.extend(
            job.drawing
                .shapes
                .iter()
                .filter(|s| s.text() == Some(id))
                .map(|s| s.id),
        );
    }
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
            "give either --around or --around-text (with --margin), or --width and \
             --height (with --at)"
                .to_string(),
        ),
    }
}
