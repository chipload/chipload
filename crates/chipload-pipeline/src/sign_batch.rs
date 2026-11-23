// SPDX-License-Identifier: GPL-3.0-or-later
//! Signs from a list (docs/specs/batch-signs.md): the same sign for every line of a list,
//! laid out in a grid, in one part whose operations cut every sign at once, selecting by
//! the batch's layers, so the tools change once.

use chipload_core::tolerances::DEPTH_TOL_MM;
use chipload_i18n::{Language, Message, Units};
use chipload_job::library::ToolLibrary;
use chipload_job::{
    EdgeSide, Job, Operation, Selector, Source, StockOrigin, TextAlign, TextAnchor, Tool,
};

use crate::PipelineError;
use crate::fonts::FontSource;
use crate::frames::{FrameSize, add_frame};
use crate::groups::{Members, group};
use crate::new_ops::OpKind;
use crate::sign::{
    Ctx, SignRequest, SignStyle, check_size, check_tools, check_values, fits_size, fitted_height,
    framed_size, length, refused, round_words, set_depth, sign_name,
};
use crate::texts::{NewText, add_text};

/// Between two signs in the grid, and from a kept board's edge, mm: room for the end mill
/// and a little more.
pub const DEFAULT_BATCH_GAP_MM: f64 = 10.0;
/// The most signs one list makes.
pub const MOST_SIGNS: usize = 500;

/// What a list of signs is made from: the sign as the dialog has it (its words ignored),
/// the list (one sign per entry, `\n` or `|` for a second line), and the gap.
#[derive(Debug, Clone, PartialEq)]
pub struct BatchRequest {
    pub sign: SignRequest,
    pub list: Vec<String>,
    pub gap: f64,
}

/// What a list of signs made.
#[derive(Debug, Clone, PartialEq)]
pub struct BatchMade {
    /// The batch's part, now the active one.
    pub part: u64,
    /// Each sign's texts, outline and group, in the list's order.
    pub texts: Vec<Vec<u64>>,
    pub outlines: Vec<u64>,
    pub groups: Vec<u64>,
    /// Each sign's frame lines: its outline first, then its borders from the outside in.
    pub frames: Vec<Vec<u64>>,
    /// The operations made, by index, in cutting order.
    pub operations: Vec<usize>,
    /// The one letters' height every sign got, for a fixed size, mm.
    pub fitted: Option<f64>,
    /// The layers the operations select: letters, borders, outline.
    pub layers: [String; 3],
}

/// The list's signs as words: each entry trimmed, `|` starting a second line, empty
/// entries left out.
pub fn list_words(list: &[String]) -> Vec<String> {
    list.iter()
        .map(|entry| {
            entry
                .replace("\\n", "\n")
                .split(['|', '\n'])
                .map(str::trim)
                .filter(|l| !l.is_empty())
                .collect::<Vec<_>>()
                .join("\n")
        })
        .filter(|w| !w.is_empty())
        .collect()
}

/// Make the signs (spec §2): one part, every sign drawn in a grid with its frame
/// following its words and grouped, and the operations once: letters, borders and edge
/// with the V-bit, a raised sign's backgrounds and the cut-outs with the end mill.
pub fn add_sign_batch(
    job: &Job,
    batch: &BatchRequest,
    fonts: &dyn FontSource,
    library: Option<&ToolLibrary>,
    lang: Language,
) -> Result<(Job, BatchMade), PipelineError> {
    let (mut job, mut made) = draw_sign_batch(job, batch, fonts, lang)?;
    let base = SignRequest {
        words: String::new(),
        around: Vec::new(),
        ..batch.sign.clone()
    };
    made.operations = batch_operations(
        &mut job,
        &base,
        made.part,
        &made.layers,
        &made.groups,
        &made.frames,
        library,
        lang,
    )?;
    Ok((job, made))
}

/// The signs drawn, grouped, in their part, as `add_sign_batch` makes them, without the
/// operations: what the dialog's picture shows while the list is typed, quickly.
pub fn draw_sign_batch(
    job: &Job,
    batch: &BatchRequest,
    fonts: &dyn FontSource,
    lang: Language,
) -> Result<(Job, BatchMade), PipelineError> {
    let words = list_words(&batch.list);
    let Some(first) = words.first() else {
        return Err(refused(Message::new("pipeline-sign-list-empty")));
    };
    if words.len() > MOST_SIGNS {
        return Err(refused(
            Message::new("pipeline-sign-list-long")
                .arg("count", words.len())
                .arg("most", MOST_SIGNS),
        ));
    }
    if !(batch.gap.is_finite() && batch.gap >= 0.0) {
        return Err(refused(
            Message::new("pipeline-sign-list-gap").arg("got", length(batch.gap)),
        ));
    }
    let base = SignRequest {
        words: first.clone(),
        around: Vec::new(),
        ..batch.sign.clone()
    };
    check_values(&base)?;
    check_tools(job, &base)?;
    if base.style == SignStyle::Inlay {
        return Err(refused(Message::new("pipeline-sign-list-inlay")));
    }
    if base.round && base.kind != crate::frames::FrameKind::Oval {
        return Err(refused(Message::new("pipeline-sign-round-oval")));
    }
    let each = |w: &String| SignRequest {
        words: w.clone(),
        ..base.clone()
    };
    // One letters' height for every sign (decision 2): the largest that fits each name.
    let fitted = match base.size {
        Some(size) => {
            check_size(&base, size)?;
            // The longest first: a sign whose words fit at the least height found so far
            // can't lower it, so most need one try rather than a search each.
            let mut order: Vec<&String> = words.iter().collect();
            order.sort_by_key(|w| std::cmp::Reverse(w.lines().map(str::len).max()));
            let mut least = f64::INFINITY;
            for w in order {
                let request = each(w);
                if least.is_finite() && fits_size(&request, least, size, fonts).unwrap_or(false) {
                    continue;
                }
                least = least.min(fitted_height(&request, size, fonts)?);
            }
            Some(least)
        }
        None => None,
    };
    let height = fitted.unwrap_or(base.height);
    let sized = |w: &String| SignRequest { height, ..each(w) };
    // Each sign's outer size, for the grid.
    let sizes: Vec<(f64, f64)> = match base.size {
        Some(size) => vec![size; words.len()],
        None => words
            .iter()
            .map(|w| framed_size(&sized(w), height, fonts))
            .collect::<Result<_, _>>()?,
    };
    let (mut job, part, name) = batch_part(job, first, words.len())?;
    let layers = batch_layers(&name, lang);
    if let Some(board) = base.board {
        let stock = job.stock_mut();
        stock.thickness = board.thickness;
        stock.origin = StockOrigin::Drawing;
        stock.fit = Some(board.margin);
    }
    let centres = grid(&job, &sizes, batch.gap, base.board.is_none())?;
    let mut texts = Vec::new();
    let mut outlines = Vec::new();
    let mut groups = Vec::new();
    let mut innermost = Vec::new();
    for ((w, centre), size) in words.iter().zip(&centres).zip(&sizes) {
        let request = sized(w);
        let (with, made) = one_sign(&job, &request, *centre, *size, &layers, fonts)?;
        job = with;
        texts.push(made.texts);
        outlines.push(made.outline);
        innermost.push(made.frame);
        groups.push(made.group);
    }
    job.active_part = Some(part);
    Ok((
        job,
        BatchMade {
            part,
            texts,
            outlines,
            groups,
            frames: innermost,
            operations: Vec::new(),
            fitted,
            layers,
        },
    ))
}

/// The batch's part: the active part when it is empty (a new project's), else a new one,
/// named after the first sign and the count ("ANNA BERG +19"); with its name.
fn batch_part(job: &Job, first: &str, count: usize) -> Result<(Job, u64, String), PipelineError> {
    let wanted = match count {
        1 => sign_name(first),
        _ => format!("{} +{}", sign_name(first), count - 1),
    };
    let empty = job
        .active_part
        .filter(|id| !job.operations.iter().any(|op| op.part() == *id));
    let (job, part) = match empty {
        Some(id) => {
            let mut out = job.clone();
            let free = crate::parts::free_name(&out, Some(&wanted)).ok();
            if let (Some(free), Some(p)) = (free, out.parts.iter_mut().find(|p| p.id == id)) {
                p.name = free;
            }
            (out, id)
        }
        None => crate::parts::add_part(job, Some(&wanted))
            .or_else(|_| crate::parts::add_part(job, None))?,
    };
    let name = job
        .part(part)
        .map_or_else(|| wanted.clone(), |p| p.name.clone());
    Ok((job, part, name))
}

/// The batch's layers, named after its part: letters, borders, outline.
fn batch_layers(name: &str, lang: Language) -> [String; 3] {
    let layer = |id: &'static str| Message::new(id).arg("name", name).render(lang, Units::Mm);
    [
        layer("pipeline-sign-list-layer-letters"),
        layer("pipeline-sign-list-layer-borders"),
        layer("pipeline-sign-list-layer-outline"),
    ]
}

/// Each sign's centre in a grid of cells as large as the largest sign, `gap` apart (spec
/// §2): as near square as the count allows round the stock's centre when the board is
/// fitted; as many to a row as fit a kept board, from its top left, `gap` in from its
/// edges, refused with the size needed when they don't fit.
fn grid(
    job: &Job,
    sizes: &[(f64, f64)],
    gap: f64,
    kept: bool,
) -> Result<Vec<(f64, f64)>, PipelineError> {
    let n = sizes.len();
    let cell = sizes
        .iter()
        .fold((0.0_f64, 0.0_f64), |(w, h), s| (w.max(s.0), h.max(s.1)));
    let ((x0, y0), (x1, y1)) = job.stock_bounds();
    let columns = if kept {
        let room = x1 - x0 - 2.0 * gap;
        (((room + gap) / (cell.0 + gap)).floor() as usize).clamp(1, n)
    } else {
        ((n as f64).sqrt().ceil() as usize).max(1)
    };
    let rows = n.div_ceil(columns);
    let total = (
        columns as f64 * cell.0 + (columns as f64 - 1.0) * gap,
        rows as f64 * cell.1 + (rows as f64 - 1.0) * gap,
    );
    let (left, top) = if kept {
        let needed = (total.0 + 2.0 * gap, total.1 + 2.0 * gap);
        if needed.0 > x1 - x0 + DEPTH_TOL_MM || needed.1 > y1 - y0 + DEPTH_TOL_MM {
            return Err(refused(
                Message::new("pipeline-sign-list-board")
                    .arg("count", n)
                    .arg("width", length(needed.0))
                    .arg("height", length(needed.1)),
            ));
        }
        (x0 + gap, y1 - gap)
    } else {
        let centre = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);
        (centre.0 - total.0 / 2.0, centre.1 + total.1 / 2.0)
    };
    Ok((0..n)
        .map(|i| {
            let (col, row) = ((i % columns) as f64, (i / columns) as f64);
            (
                left + col * (cell.0 + gap) + cell.0 / 2.0,
                top - row * (cell.1 + gap) - cell.1 / 2.0,
            )
        })
        .collect())
}

/// One sign of the batch, as it was drawn.
struct OneSign {
    texts: Vec<u64>,
    outline: u64,
    /// Its frame's lines: the outline first, then the borders from the outside in.
    frame: Vec<u64>,
    group: u64,
}

/// One sign's words round `centre`, on the batch's letters layer, its frame on the
/// outline and borders layers (the size given, or round the words), and its group.
fn one_sign(
    job: &Job,
    request: &SignRequest,
    centre: (f64, f64),
    size: (f64, f64),
    layers: &[String; 3],
    fonts: &dyn FontSource,
) -> Result<(Job, OneSign), PipelineError> {
    let (job, texts) = if request.round {
        round_words(job, request, centre, &layers[0], fonts)?
    } else {
        let mut new = NewText::new(&request.words, request.height);
        new.font = &request.font;
        (new.x, new.y) = centre;
        new.anchor = TextAnchor::Center;
        new.align = TextAlign::Center;
        new.layer = &layers[0];
        let (job, report) = add_text(job, new, fonts)?;
        (job, vec![report.id])
    };
    let letters: Vec<u64> = job
        .drawing
        .shapes
        .iter()
        .filter(|s| s.text().is_some_and(|t| texts.contains(&t)))
        .map(|s| s.id)
        .collect();
    let frame_size = match request.size {
        Some(_) => FrameSize::Fixed {
            width: size.0,
            height: size.1,
            center: if request.round {
                Some(centre)
            } else {
                crate::shapes::drawn_box(&job, &letters)
                    .map(|((x0, y0), (x1, y1))| ((x0 + x1) / 2.0, (y0 + y1) / 2.0))
            },
        },
        None => FrameSize::Around {
            ids: letters,
            margin: request.margin,
        },
    };
    let mut frame = request.frame(frame_size);
    frame.layer.clone_from(&layers[2]);
    frame.border_layer.clone_from(&layers[1]);
    let framed = add_frame(&job, &frame)?;
    let members = Members {
        shapes: framed.new_ids.clone(),
        texts: texts.clone(),
    };
    let (job, group_id) = group(&framed.job, &members, Some(&sign_name(&request.words)))?;
    Ok((
        job,
        OneSign {
            texts,
            outline: framed.new_ids[0],
            frame: framed.new_ids,
            group: group_id,
        },
    ))
}

/// The batch's operations, grouped by tool so it changes once (spec §3): the letters
/// (V-carved, or a follow path for single-line words), the borders and the edge with the
/// V-bit; a raised sign's background, one pocket per sign, and the cut-outs with the end
/// mill. All but the backgrounds select by layer.
#[allow(clippy::too_many_arguments)]
fn batch_operations(
    job: &mut Job,
    request: &SignRequest,
    part: u64,
    layers: &[String; 3],
    groups: &[u64],
    frames: &[Vec<u64>],
    library: Option<&ToolLibrary>,
    lang: Language,
) -> Result<Vec<usize>, PipelineError> {
    let contours = crate::load_drawing(job)?.contours;
    let ctx = Ctx {
        contours: &contours,
        library,
        lang,
        part,
    };
    let op_name = |id: &'static str| Message::new(id).render(lang, Units::Mm);
    let on = |layer: &String| Selector {
        layer: Some(layer.clone()),
        ..Selector::default()
    };
    let mut operations = Vec::new();
    if request.style != SignStyle::Raised {
        let mut op = if request.single_line() {
            let mut op = ctx.make(job, OpKind::Path, on(&layers[0]), request.vbit)?;
            set_depth(&mut op, request.engrave_depth);
            op
        } else {
            ctx.make(job, OpKind::Vcarve, on(&layers[0]), request.vbit)?
        };
        *op.name_mut() = op_name("pipeline-name-sign-letters");
        operations.push(ctx.append(job, op));
    }
    let bit = job.tool(request.vbit).cloned();
    if request.borders > 0 {
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
        let mut op = ctx.make(job, OpKind::Path, on(&layers[1]), request.vbit)?;
        *op.name_mut() = op_name("pipeline-name-sign-borders");
        set_depth(&mut op, depth);
        operations.push(ctx.append(job, op));
    }
    if let Some(edge) = request.edge {
        let widest = bit.as_ref().and_then(Tool::widest_chamfer).unwrap_or(0.0);
        if edge > widest + DEPTH_TOL_MM {
            return Err(refused(
                Message::new("pipeline-sign-edge-wide")
                    .arg("width", length(edge))
                    .arg("tool", request.vbit)
                    .arg("widest", length(widest)),
            ));
        }
        let mut op = ctx.make(job, OpKind::Chamfer, on(&layers[2]), request.vbit)?;
        *op.name_mut() = op_name("pipeline-name-sign-edge");
        if let Operation::Chamfer(c) = &mut op {
            c.side = EdgeSide::Outside;
            c.width = edge;
            c.start_depth = 0.0;
        }
        operations.push(ctx.append(job, op));
    }
    if request.style == SignStyle::Raised {
        // Each sign's background: inside its innermost line, round its letters.
        for (group_id, frame) in groups.iter().zip(frames) {
            let innermost = *frame.last().unwrap_or(&frame[0]);
            let select = Selector {
                source: Some(Source::Group(*group_id)),
                exclude: frame
                    .iter()
                    .copied()
                    .filter(|id| *id != innermost)
                    .collect(),
                ..Selector::default()
            };
            let mut op = ctx.make(job, OpKind::Pocket, select, request.end_mill)?;
            *op.name_mut() = op_name("pipeline-name-sign-background");
            if let Operation::Pocket(p) = &mut op {
                p.start_depth = 0.0;
                p.end_depth = request.relief;
                p.through = None;
            }
            operations.push(ctx.append(job, op));
        }
    }
    let mut op = ctx.make(job, OpKind::Profile, on(&layers[2]), request.end_mill)?;
    *op.name_mut() = op_name("pipeline-name-sign-cut-out");
    operations.push(ctx.append(job, op));
    let made: Vec<String> = operations
        .iter()
        .map(|i| format!("operations.{i}"))
        .collect();
    let ours = |path: &str| {
        made.iter()
            .any(|own| path == own || path.starts_with(&format!("{own}.")))
    };
    if let Some(problem) = job.validate().into_iter().find(|p| ours(&p.path)) {
        return Err(refused(problem.text_message));
    }
    Ok(operations)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_list_is_one_sign_per_entry_with_bars_for_second_lines() {
        let list = vec![
            "ANNA BERG | ROOM 12".to_string(),
            "  ".to_string(),
            "BO LIND\\nROOM 14".to_string(),
            "CARL".to_string(),
        ];
        assert_eq!(
            list_words(&list),
            ["ANNA BERG\nROOM 12", "BO LIND\nROOM 14", "CARL"]
        );
    }
}
