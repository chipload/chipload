// SPDX-License-Identifier: GPL-3.0-or-later
//! `chipload sign add`: a sign in one step, the words, a frame that follows them and every
//! operation that cuts it (docs/specs/sign-maker.md §3, §7).

use std::path::PathBuf;
use std::process::ExitCode;

use chipload_pipeline::frames::FrameKind;
use chipload_pipeline::library::{Library, settings_dir};
use chipload_pipeline::sign::{
    DEFAULT_BOARD_MARGIN_MM, DEFAULT_BOARD_THICKNESS_MM, DEFAULT_BORDER_WIDTH_MM,
    DEFAULT_RELIEF_MM, SignBoard, SignRequest, SignStyle, add_sign,
};
use clap::{Args, Subcommand, ValueEnum};
use serde_json::json;

use crate::frames::{CornersArg, KindArg};
use crate::load;
use crate::out::Out;

#[derive(Subcommand)]
pub enum SignAction {
    /// Make a sign: the words centred on the board, a frame round them that follows them,
    /// the sign grouped in a part of its own, and its operations: the letters, the
    /// borders' grooves, the edge chamfer, and the profile that cuts it out.
    Add(Box<AddSign>),
}

/// `sign add`'s options.
#[derive(Args)]
pub struct AddSign {
    /// The project (.chipload).
    project: PathBuf,
    /// What it says; "\n" starts a new line. Leave it out to make the sign round shapes
    /// already drawn (--around, --around-text, --around-image).
    words: Option<String>,
    /// Round these shapes instead of new words (ids as `inspect` lists them).
    #[arg(long, num_args = 1.., value_delimiter = ',')]
    around: Vec<u64>,
    /// Round these texts' letters.
    #[arg(long, num_args = 1.., value_delimiter = ',')]
    around_text: Vec<u64>,
    /// Round these traced pictures (ids as `image list` shows them): carved as the
    /// picture, the frame following it when it is traced again.
    #[arg(long, num_args = 1.., value_delimiter = ',')]
    around_image: Vec<u64>,
    /// The font (`chipload text fonts`).
    #[arg(long)]
    font: Option<String>,
    /// The capital height, mm.
    #[arg(value_parser = crate::lengths::length, long, default_value = "30")]
    height: f64,
    /// The frame (`frame kinds`).
    #[arg(long, value_enum, default_value = "rounded")]
    frame: KindArg,
    /// A rectangular frame's corners: sharp, round, notch, chamfer or step.
    #[arg(long, value_enum)]
    corners: Option<CornersArg>,
    /// The frame's corner size, mm (default: a tenth of its shorter side).
    #[arg(value_parser = crate::lengths::length, long)]
    corner: Option<f64>,
    /// From the words to the inner edge of the innermost border's groove, mm.
    #[arg(value_parser = crate::lengths::length, long, default_value = "10")]
    margin: f64,
    /// Border lines inside the outline, 0 to 3.
    #[arg(long, default_value_t = 1)]
    borders: u32,
    /// From the outline to the first border and between borders, mm.
    #[arg(value_parser = crate::lengths::length, long, default_value = "5")]
    gap: f64,
    /// The borders' own corners, inside a rectangular frame.
    #[arg(long, value_enum)]
    border_corners: Option<CornersArg>,
    /// Each border two grooves.
    #[arg(long)]
    double: bool,
    /// The borders' groove width at the surface, mm: their depth comes from the V-bit.
    #[arg(value_parser = crate::lengths::length, long)]
    border_width: Option<f64>,
    /// engraved (V-carved letters), raised (the background cleared round them) or inlay.
    #[arg(long, value_enum, default_value = "engraved")]
    style: StyleArg,
    /// A raised sign's background depth, mm.
    #[arg(value_parser = crate::lengths::length, long)]
    relief: Option<f64>,
    /// The outline's edge: `none`, or `chamfer:WIDTH` (mm) cut with the V-bit.
    #[arg(long, value_parser = parse_edge, default_value = "none")]
    edge: Edge,
    /// The V-bit for the letters and grooves (a project tool id).
    #[arg(long)]
    vbit: i64,
    /// The end mill that cuts the sign out (and clears a raised background).
    #[arg(long)]
    end_mill: i64,
    /// The board's thickness, mm; the board is fitted round the sign.
    #[arg(value_parser = crate::lengths::length, long)]
    thickness: Option<f64>,
    /// From the sign's outline to the board's edges, mm.
    #[arg(value_parser = crate::lengths::length, long)]
    board_margin: Option<f64>,
    /// Leave the stock as it is instead of fitting it round the sign.
    #[arg(long, conflicts_with_all = ["thickness", "board_margin"])]
    keep_stock: bool,
    /// Where to write the result; default: the project itself.
    #[arg(short, long)]
    output: Option<PathBuf>,
}

#[derive(Clone, Copy, ValueEnum)]
pub enum StyleArg {
    Engraved,
    Raised,
    Inlay,
}

impl From<StyleArg> for SignStyle {
    fn from(s: StyleArg) -> Self {
        match s {
            StyleArg::Engraved => SignStyle::Engraved,
            StyleArg::Raised => SignStyle::Raised,
            StyleArg::Inlay => SignStyle::Inlay,
        }
    }
}

/// The outline's edge.
#[derive(Clone, Copy, Debug)]
pub enum Edge {
    None,
    Chamfer(f64),
}

fn parse_edge(text: &str) -> Result<Edge, String> {
    let text = text.trim();
    if text.eq_ignore_ascii_case("none") {
        return Ok(Edge::None);
    }
    match text.split_once(':') {
        Some((kind, width)) if kind.trim().eq_ignore_ascii_case("chamfer") => {
            crate::lengths::length(width).map(Edge::Chamfer)
        }
        _ => Err(format!(
            "`{text}`: give none, or chamfer:WIDTH (round-overs need a round-over bit, which \
             tools don't describe yet)"
        )),
    }
}

pub fn sign(out: &mut Out, action: SignAction) -> ExitCode {
    match action {
        SignAction::Add(add) => add_one(out, *add),
    }
}

fn add_one(out: &mut Out, add: AddSign) -> ExitCode {
    let job = match load(out, &add.project) {
        Ok(job) => job,
        Err(code) => return code,
    };
    let kind = FrameKind::from(add.frame);
    let kind = match add.corners {
        None => kind,
        Some(c) if kind.corner_style().is_some() => {
            chipload_pipeline::frames::CornerStyle::from(c).kind()
        }
        Some(_) => {
            return out.fail(format!(
                "--corners is for the rectangular frames, not {}",
                kind.name()
            ));
        }
    };
    let around = match crate::frames::around_ids(
        &job,
        add.around.clone(),
        &add.around_text,
        &add.around_image,
    ) {
        Ok(ids) => ids,
        Err(err) => return out.fail(err),
    };
    let words = match (&add.words, around.is_empty()) {
        (Some(words), true) => words.clone(),
        (None, false) => String::new(),
        _ => {
            return out.fail(
                "give the words, or --around, --around-text or --around-image: not both, \
                 nor neither",
            );
        }
    };
    let mut request = SignRequest::new(&words, add.vbit, add.end_mill);
    request.around = around;
    if let Some(font) = add.font {
        request.font = font;
    }
    request.height = add.height;
    request.kind = kind;
    request.corner = add.corner;
    request.margin = add.margin;
    request.borders = add.borders;
    request.gap = add.gap;
    request.border_corners = add.border_corners.map(Into::into);
    request.double = add.double;
    request.border_width = add.border_width.unwrap_or(DEFAULT_BORDER_WIDTH_MM);
    request.style = add.style.into();
    request.relief = add.relief.unwrap_or(DEFAULT_RELIEF_MM);
    request.edge = match add.edge {
        Edge::None => None,
        Edge::Chamfer(width) => Some(width),
    };
    request.board = (!add.keep_stock).then(|| SignBoard {
        thickness: add.thickness.unwrap_or(DEFAULT_BOARD_THICKNESS_MM),
        margin: add.board_margin.unwrap_or(DEFAULT_BOARD_MARGIN_MM),
    });
    let library = settings_dir()
        .map(|dir| Library::load(&dir))
        .unwrap_or_default();
    let (job, made) = match add_sign(
        &job,
        &request,
        &crate::texts::computer_fonts(),
        Some(&library.tools),
        chipload_i18n::Language::En,
    ) {
        Ok(done) => done,
        Err(err) => return out.fail(err),
    };
    let target = add.output.as_deref().unwrap_or(&add.project);
    if let Err(err) = crate::save_project(out, &job, target) {
        return out.fail(err);
    }
    let part = job.part(made.part).map_or("", |p| p.name.as_str());
    let what = made
        .text
        .map_or_else(|| "the shapes".to_string(), |t| format!("text {t}"));
    out.line(format!(
        "Made the sign \"{part}\" ({}), {what} in a {} frame S{}, in part {}:",
        request.style.name(),
        kind.name(),
        made.outline,
        made.part
    ));
    for &i in &made.operations {
        let op = &job.operations[i];
        out.line(format!("  operation {}: {}", i + 1, op.name()));
    }
    if let Some(setup) = made.plug_setup {
        out.line(format!(
            "The inlay's plug is in setup {setup}, a board of its own."
        ));
    }
    out.wrote(target, "");
    out.set("part", json!(made.part));
    out.set("text", json!(made.text));
    out.set("outline", json!(made.outline));
    out.set("borders", json!(made.borders));
    out.set("group", json!(made.group));
    out.set(
        "operations",
        json!(made.operations.iter().map(|i| i + 1).collect::<Vec<_>>()),
    );
    out.set("plug_setup", json!(made.plug_setup));
    ExitCode::SUCCESS
}
