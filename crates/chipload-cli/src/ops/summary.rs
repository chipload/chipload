// SPDX-License-Identifier: GPL-3.0-or-later
//! One operation described in a line, as `op list` and the report show it.

use chipload_job::{Operation, RampShape};

/// What `op add` added, in a few words: "\"Profile 1\", outside, through to 12.300 mm, with
/// tabs, tool 1".
pub(crate) fn describe(op: &Operation, tools: &[chipload_job::Tool]) -> String {
    let mut parts = vec![format!("\"{}\"", op.name())];
    if let Operation::Chamfer(c) = op {
        parts.push(format!(
            "{} mm wide, {}",
            chipload_core::text::mm(c.width),
            match c.side {
                chipload_job::EdgeSide::Outside => "outside",
                chipload_job::EdgeSide::Inside => "inside",
            }
        ));
    }
    if let Operation::Moulding(m) = op {
        let mm = chipload_core::text::mm;
        parts.push(match m.profile {
            chipload_job::MouldProfile::Cove { radius } => format!("cove R {}", mm(radius)),
            chipload_job::MouldProfile::Roundover { radius } => {
                format!("round-over R {}", mm(radius))
            }
            chipload_job::MouldProfile::Ogee { width, depth } => {
                format!("ogee {} × {} mm", mm(width), mm(depth))
            }
            chipload_job::MouldProfile::Bead { radius } => format!("bead R {}", mm(radius)),
            chipload_job::MouldProfile::Flute { radius } => format!("flute R {}", mm(radius)),
            chipload_job::MouldProfile::Shape { shape } => format!("drawn profile S{shape}"),
        });
        parts.push(
            match m.side {
                chipload_job::MouldSide::Left => "left",
                chipload_job::MouldSide::Right => "right",
                chipload_job::MouldSide::Inside => "inside",
                chipload_job::MouldSide::Outside => "outside",
            }
            .to_string(),
        );
        match m.ends {
            chipload_job::MouldEnds::Square => {}
            chipload_job::MouldEnds::Round => parts.push("round ends".to_string()),
            chipload_job::MouldEnds::Taper { length, .. } => {
                parts.push(format!("ends tapered over {} mm", mm(length)));
            }
        }
        parts.push(format!("stepover {} mm", mm(m.stepover)));
    }
    if let Operation::Prism(f) = op {
        parts.push(match f.max_depth {
            Some(max) => format!("edges at most {} mm deep", chipload_core::text::mm(max)),
            None => "as deep as the shapes need".to_string(),
        });
    }
    if let Operation::Pattern(f) = op {
        let mm = chipload_core::text::mm;
        let deg = |a: f64| chipload_core::text::mm(a.to_degrees());
        parts.push(match f.pattern {
            chipload_job::FillPattern::Lines { angle, spacing } => {
                format!("lines {} mm apart at {}°", mm(spacing), deg(angle))
            }
            chipload_job::FillPattern::Cross {
                angle,
                spacing,
                between,
            } => format!(
                "cross-hatch {} mm apart at {}°, {}° between",
                mm(spacing),
                deg(angle),
                deg(between)
            ),
            chipload_job::FillPattern::Rings { spacing } => {
                format!("rings {} mm apart", mm(spacing))
            }
            chipload_job::FillPattern::Waves {
                angle,
                spacing,
                wavelength,
                height,
            } => format!(
                "waves {} mm apart at {}°, {} mm long, {} mm high",
                mm(spacing),
                deg(angle),
                mm(wavelength),
                mm(height)
            ),
            chipload_job::FillPattern::Dashes {
                angle,
                spacing,
                length,
                gap,
                ..
            } => format!(
                "dashes {} mm long, {} mm gaps, rows {} mm apart at {}°",
                mm(length),
                mm(gap),
                mm(spacing),
                deg(angle)
            ),
            chipload_job::FillPattern::Herringbone {
                angle,
                spacing,
                length,
                ..
            } => format!(
                "herringbone {} mm strokes {} mm apart at {}°",
                mm(length),
                mm(spacing),
                deg(angle)
            ),
            chipload_job::FillPattern::Scales {
                angle,
                spacing,
                radius,
                ..
            } => format!(
                "scales of radius {} mm, rows {} mm apart at {}°",
                mm(radius),
                mm(spacing),
                deg(angle)
            ),
            chipload_job::FillPattern::Sunburst {
                spacing,
                length,
                pitch,
                ..
            } => format!(
                "sunburst {} mm strokes, rings {} mm apart, {} mm round",
                mm(length),
                mm(spacing),
                mm(pitch)
            ),
        });
        if let chipload_job::FillPattern::Dashes { whole: false, .. }
        | chipload_job::FillPattern::Herringbone { whole: false, .. }
        | chipload_job::FillPattern::Scales { whole: false, .. }
        | chipload_job::FillPattern::Sunburst { whole: false, .. } = f.pattern
        {
            parts.push("clipped".to_string());
        }
        parts.push(if f.inside { "inside" } else { "on the line" }.to_string());
    }
    if let Operation::Flute(f) = op {
        parts.push(format!(
            "ramps {} / {} mm, {}",
            chipload_core::text::mm(f.ramp_start),
            chipload_core::text::mm(f.ramp_end),
            match f.ramp_shape {
                RampShape::Straight => "straight",
                RampShape::Curved => "curved",
            }
        ));
    }
    if let Operation::Profile(p) = op {
        parts.push(
            match p.side {
                chipload_job::Side::Inside => "inside",
                chipload_job::Side::Outside => "outside",
                chipload_job::Side::On => "on the line",
            }
            .to_string(),
        );
    }
    let (_, end) = op.depths(tools);
    let depth = chipload_core::text::mm(end);
    parts.push(match op.through() {
        Some(_) => format!("through to {depth} mm"),
        None => format!("{depth} mm deep"),
    });
    if let Operation::Profile(p) = op
        && let Some(tabs) = &p.tabs
    {
        parts.push(format!("{} tabs per contour", tabs.count));
    }
    parts.push(format!("tool {}", op.tool()));
    parts.join(", ")
}
