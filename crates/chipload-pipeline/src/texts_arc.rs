// SPDX-License-Identifier: GPL-3.0-or-later
//! A text on an arc (docs/specs/text-on-arc.md §3): its letters, laid out straight by the
//! font, each moved and turned whole onto the circle, so they stay exact lines and arcs.

use std::f64::consts::{FRAC_PI_2, TAU};

use chipload_core::math;
use chipload_core::primitives::Primitive;
use chipload_i18n::{Digits, Message, Value};
use chipload_job::{ArcSide, Text, TextAlign, TextArc};
use chipload_text::Lettering;

use crate::PipelineError;

/// The outlines of `straight` (a text's letters on straight lines, the box at (0, 0))
/// placed on `arc` round the text's position, each with its letter. Refused when a line
/// is longer than its circle or would reach the centre.
pub(crate) fn on_arc(
    text: &Text,
    arc: &TextArc,
    straight: &Lettering,
) -> Result<(Vec<Vec<Primitive>>, Vec<usize>), PipelineError> {
    let h = text.height;
    let step = text.line_spacing * h;
    // Each line's circle: the first on the radius, the others the way they're read.
    let radius = |n: usize| match arc.side {
        ArcSide::Top => arc.radius - n as f64 * step,
        ArcSide::Bottom => arc.radius + n as f64 * step,
    };
    for (n, &(start, end)) in straight.spans.iter().enumerate() {
        let r = radius(n);
        if r < h / 2.0 {
            return Err(PipelineError::Job(refusal(
                "pipeline-text-arc-centre",
                text,
                n,
            )));
        }
        if end - start > TAU * r {
            return Err(PipelineError::Job(
                refusal("pipeline-text-arc-long", text, n)
                    .arg("round", Value::Length(TAU * r, Digits::Fixed(1))),
            ));
        }
    }
    // Where along each line the angle is: its start, middle or end.
    let reference = |n: usize| {
        let (start, end) = straight.spans.get(n).copied().unwrap_or((0.0, 0.0));
        match text.align {
            TextAlign::Left => start,
            TextAlign::Center => (start + end) / 2.0,
            TextAlign::Right => end,
        }
    };
    let angle = arc.angle.to_radians();
    let mut outlines = Vec::with_capacity(straight.outlines.len());
    let mut letters = Vec::with_capacity(straight.outlines.len());
    for (outline, &letter) in straight.outlines.iter().zip(&straight.letter) {
        let Some(&(n, middle)) = straight.places.get(letter) else {
            continue;
        };
        let r = radius(n);
        let base = straight.baselines.get(n).copied().unwrap_or(0.0);
        let along = (middle - reference(n)) / r;
        // Top: read clockwise, letters pointing out; bottom: counter-clockwise, upright.
        let (phi, turn) = match arc.side {
            ArcSide::Top => (angle - along, angle - along - FRAC_PI_2),
            ArcSide::Bottom => (angle + along, angle + along + FRAC_PI_2),
        };
        let (sin, cos) = math::sin_cos(phi);
        let to = (text.x + r * cos, text.y + r * sin);
        outlines.push(
            outline
                .iter()
                .map(|p| {
                    p.translated((-middle, -(base + h / 2.0)))
                        .rotated((0.0, 0.0), turn)
                        .translated(to)
                })
                .collect(),
        );
        letters.push(letter);
    }
    Ok((outlines, letters))
}

/// A refusal naming the text and its line (counted from 1).
fn refusal(id: &'static str, text: &Text, line: usize) -> Message {
    Message::new(id)
        .arg("text", crate::texts::text_label(text))
        .arg("line", line + 1)
}
