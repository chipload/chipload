// SPDX-License-Identifier: GPL-3.0-or-later
//! An operation in words: its kind, depths, entry and tabs.

use chipload_i18n::{Digits, Message, Value};
use chipload_job::{
    Corners, EdgeSide, Entry, FillPattern, HoleMethod, HolesOp, MouldEnds, MouldProfile, MouldSide,
    Operation, PocketOp, RampShape, Side, Tool, find_tool,
};

use super::files::direction_message;
use super::{Words, len, len_u, listed, num};

/// "Profile, outside, climb, leaves 0.2 mm" (a negative clearance).
pub(super) fn kind_text(op: &Operation, tools: &[Tool], w: Words) -> String {
    let stepover = |s: f64| Message::new("sheet-stepover").arg("percent", num(s * 100.0));
    let mut parts = match op {
        Operation::Profile(p) => {
            let side = match p.side {
                Side::Outside => "sheet-side-outside",
                Side::Inside => "sheet-side-inside",
                Side::On => "sheet-side-on",
            };
            vec![
                Message::new("sheet-kind-profile"),
                Message::new(side),
                direction_message(p.direction),
            ]
        }
        Operation::Pocket(p) => vec![
            Message::new("sheet-kind-pocket"),
            direction_message(p.direction),
            stepover(p.stepover),
        ],
        Operation::Vcarve(_) => vec![Message::new("sheet-kind-vcarve")],
        Operation::Path(_) => vec![Message::new("sheet-kind-path")],
        // "chamfer 2 mm wide, outside, climb" (docs/specs/chamfer.md §6).
        Operation::Chamfer(c) => vec![
            Message::new("sheet-kind-chamfer").arg("width", len(c.width)),
            Message::new(match c.side {
                EdgeSide::Outside => "sheet-side-outside",
                EdgeSide::Inside => "sheet-side-inside",
            }),
            direction_message(c.direction),
        ],
        // "flute, ramps 20 / 20 mm, straight" (docs/specs/fluting.md §8).
        Operation::Flute(f) => vec![
            Message::new("sheet-kind-flute")
                .arg("start", len(f.ramp_start))
                .arg("end", len(f.ramp_end)),
            Message::new(match f.ramp_shape {
                RampShape::Straight => "sheet-ramp-straight",
                RampShape::Curved => "sheet-ramp-curved",
            }),
        ],
        Operation::Moulding(m) => moulding_parts(m),
        // "Prism, climb" (docs/specs/prism.md §5).
        Operation::Prism(f) => vec![
            Message::new("sheet-kind-prism"),
            direction_message(f.direction),
        ],
        Operation::Pattern(f) => pattern_parts(f),
        Operation::Holes(h) => match h.method {
            HoleMethod::Drill { .. } => vec![Message::new("sheet-kind-holes-drilled")],
            HoleMethod::Bore { direction, .. } => vec![
                Message::new("sheet-kind-holes-bored"),
                direction_message(direction),
            ],
        },
        // A part surface's area and rings (docs/specs/surface-levels.md §7).
        Operation::Surface(f) if !f.select.is_empty() => vec![
            Message::new("sheet-kind-surface-part")
                .arg("margin", len_u(f.margin))
                .arg("percent", num(f.stepover * 100.0)),
        ],
        // docs/specs/surfacing.md §3, §5.
        Operation::Surface(f) => {
            let kind = Message::new("sheet-kind-surface").arg("percent", num(f.stepover * 100.0));
            vec![match tools.iter().find(|t| t.id == f.tool) {
                Some(t) => Message::new("sheet-kind-surface-past")
                    .arg("surface", kind)
                    .arg("distance", len_u(t.diameter / 2.0)),
                None => kind,
            }]
        }
        // docs/specs/stl-relief.md §5.
        Operation::Relief(r) => vec![
            Message::new(if r.zigzag {
                "sheet-kind-relief-zigzag"
            } else {
                "sheet-kind-relief-one-way"
            })
            .arg("model", r.model)
            .arg("angle", num(r.angle.to_degrees()))
            .arg("percent", num(r.stepover * 100.0)),
        ],
    };
    parts.extend(extra_parts(op, tools));
    listed(w, &parts)
}

/// A moulding's profile, side, ends, stepover and direction: "Moulding, cove R 8, outside,
/// climb" (docs/specs/moulding.md §9).
fn moulding_parts(m: &chipload_job::MouldingOp) -> Vec<Message> {
    let profile = match m.profile {
        MouldProfile::Cove { radius } => {
            Message::new("sheet-moulding-cove").arg("radius", len(radius))
        }
        MouldProfile::Roundover { radius } => {
            Message::new("sheet-moulding-roundover").arg("radius", len(radius))
        }
        MouldProfile::Ogee { width, depth } => Message::new("sheet-moulding-ogee")
            .arg("width", len(width))
            .arg("depth", len(depth)),
        MouldProfile::Bead { radius } => {
            Message::new("sheet-moulding-bead").arg("radius", len(radius))
        }
        MouldProfile::Flute { radius } => {
            Message::new("sheet-moulding-flute").arg("radius", len(radius))
        }
        MouldProfile::Shape { shape } => Message::new("sheet-moulding-shape").arg("shape", shape),
    };
    let side = Message::new(match m.side {
        MouldSide::Left => "sheet-moulding-left",
        MouldSide::Right => "sheet-moulding-right",
        MouldSide::Inside => "sheet-side-inside",
        MouldSide::Outside => "sheet-side-outside",
    });
    let mut parts = vec![profile, side];
    match m.ends {
        MouldEnds::Square => {}
        MouldEnds::Round => parts.push(Message::new("sheet-moulding-round-ends")),
        MouldEnds::Taper { length, .. } => {
            parts.push(Message::new("sheet-moulding-tapered-ends").arg("length", len(length)))
        }
    }
    parts.push(Message::new("sheet-stepover-mm").arg("stepover", len(m.stepover)));
    parts.push(direction_message(m.direction));
    parts
}

/// A pattern's kind, where it runs and its ramp: "pattern, cross-hatch 3 mm at 45°, inside
/// the edge" (docs/specs/fill-pattern.md §8).
fn pattern_parts(f: &chipload_job::PatternOp) -> Vec<Message> {
    let kind = pattern_kind(f.pattern);
    let mut parts = vec![
        kind,
        Message::new(if f.inside {
            "sheet-pattern-inside"
        } else {
            "sheet-pattern-on-line"
        }),
    ];
    if f.ramp > 0.0 {
        parts.push(Message::new("sheet-pattern-ramp").arg("ramp", len(f.ramp)));
    }
    parts
}

/// A fill pattern with its sizes and angles.
fn pattern_kind(pattern: FillPattern) -> Message {
    let deg = |a: f64| num(a.to_degrees());
    match pattern {
        FillPattern::Lines { angle, spacing } => Message::new("sheet-pattern-lines")
            .arg("spacing", len(spacing))
            .arg("angle", deg(angle)),
        FillPattern::Cross {
            angle,
            spacing,
            between,
        } => Message::new("sheet-pattern-cross")
            .arg("spacing", len(spacing))
            .arg("angle", deg(angle))
            .arg("between", deg(between)),
        FillPattern::Rings { spacing } => {
            Message::new("sheet-pattern-rings").arg("spacing", len(spacing))
        }
        FillPattern::Waves {
            angle,
            spacing,
            wavelength,
            height,
        } => Message::new("sheet-pattern-waves")
            .arg("spacing", len(spacing))
            .arg("angle", deg(angle))
            .arg("wavelength", len(wavelength))
            .arg("height", len(height)),
        FillPattern::Dashes {
            angle,
            spacing,
            length,
            gap,
            ..
        } => Message::new("sheet-pattern-dashes")
            .arg("spacing", len(spacing))
            .arg("angle", deg(angle))
            .arg("length", len(length))
            .arg("gap", len(gap)),
        FillPattern::Herringbone {
            angle,
            spacing,
            length,
            ..
        } => Message::new("sheet-pattern-herringbone")
            .arg("spacing", len(spacing))
            .arg("angle", deg(angle))
            .arg("length", len(length)),
        FillPattern::Scales {
            angle,
            spacing,
            radius,
            ..
        } => Message::new("sheet-pattern-scales")
            .arg("spacing", len(spacing))
            .arg("angle", deg(angle))
            .arg("radius", len(radius)),
        FillPattern::Sunburst {
            spacing,
            length,
            pitch,
            ..
        } => Message::new("sheet-pattern-sunburst")
            .arg("spacing", len(spacing))
            .arg("length", len(length))
            .arg("pitch", len(pitch)),
    }
}

/// What any kind may add: clearance, rest after a tool, bones in corners.
fn extra_parts(op: &Operation, tools: &[Tool]) -> Vec<Message> {
    let mut parts = Vec::new();
    // Positive clearance cuts into the material that is kept, negative leaves some
    // (docs/specs/depths-and-clearance.md §3).
    let clearance = op.clearance();
    if clearance > 0.0 {
        parts.push(Message::new("sheet-cuts-past").arg("distance", len_u(clearance)));
    } else if clearance < 0.0 {
        parts.push(Message::new("sheet-leaves").arg("distance", len_u(-clearance)));
    }
    // docs/specs/rest-machining.md §6.
    if let Operation::Pocket(PocketOp {
        rest: Some(rest), ..
    }) = op
    {
        parts.push(match find_tool(tools, rest.after_tool) {
            Some(tool) => Message::new("sheet-rest-after-size")
                .arg("tool", rest.after_tool)
                .arg("diameter", len_u(tool.diameter)),
            None => Message::new("sheet-rest-after").arg("tool", rest.after_tool),
        });
    }
    // docs/specs/dogbones.md §7.
    if let Some(corners) = op.corners() {
        let style = match corners {
            Corners::Dogbone { .. } => "sheet-dogbones",
            Corners::Tbone { .. } => "sheet-tbones",
        };
        parts.push(Message::new(style).arg("degrees", num(corners.max_angle().to_degrees())));
    }
    // docs/specs/dogbone-per-corner.md.
    let chosen = op.corner_choices().len();
    if chosen > 0 {
        parts.push(Message::new("sheet-corner-choices").arg("count", chosen));
    }
    parts
}

/// "0 to 6, 2 per pass", with a profile's finishing stock and skin.
pub(super) fn depth_text(op: &Operation, tools: &[Tool], w: Words) -> String {
    let (start, end) = op.depths(tools);
    let step = match op {
        Operation::Profile(p) => Some(p.step_down),
        Operation::Pocket(p) => Some(p.step_down),
        Operation::Vcarve(_) => None,
        Operation::Path(p) => Some(p.step_down),
        Operation::Chamfer(c) => c.step_down,
        Operation::Flute(f) => f.step_down,
        Operation::Pattern(f) => f.step_down,
        Operation::Prism(f) => f.step_down,
        Operation::Moulding(m) => m.step_down,
        Operation::Holes(_) => None,
        Operation::Surface(f) => Some(f.step_down),
        // In one pass along the model (docs/specs/stl-relief.md §5).
        Operation::Relief(_) => None,
    };
    let depths = |id: &'static str| {
        Message::new(id)
            .arg("start", len(start))
            .arg("end", len(end))
    };
    let mut parts = match op {
        // docs/specs/holes.md §6: a drill's depth is its point's.
        Operation::Holes(HolesOp {
            method: HoleMethod::Drill { peck },
            ..
        }) => vec![
            depths("sheet-depths-drill"),
            match peck {
                Some(peck) if *peck < end - start => {
                    Message::new("sheet-pecks").arg("peck", len_u(*peck))
                }
                _ => Message::new("sheet-one-go"),
            },
        ],
        Operation::Holes(HolesOp {
            method: HoleMethod::Bore { step_down, .. },
            ..
        }) => vec![
            depths("sheet-depths"),
            Message::new("sheet-per-turn").arg("step", len(*step_down)),
        ],
        // Its depth follows from the shapes: at most the cone or the max depth
        // (docs/specs/prism.md §4).
        Operation::Prism(_) => vec![
            depths("sheet-depths-at-most"),
            match step {
                Some(step) if step > 0.0 && step < end - start => {
                    Message::new("sheet-per-pass").arg("step", len(step))
                }
                _ => Message::new("sheet-one-pass"),
            },
        ],
        // docs/specs/v-carve.md decision 12: a number of passes of equal load.
        Operation::Vcarve(v) if v.passes > 1 => vec![
            depths("sheet-depths"),
            Message::new("sheet-passes").arg("count", v.passes),
        ],
        _ => vec![
            depths("sheet-depths"),
            match step {
                Some(step) if step > 0.0 && step < end - start => {
                    Message::new("sheet-per-pass").arg("step", len(step))
                }
                _ => Message::new("sheet-one-pass"),
            },
        ],
    };
    // A V-bit following a path: how wide its groove is (docs/specs/follow-path.md §5).
    if let Operation::Path(p) = op
        && let Some(width) = tools
            .iter()
            .find(|t| t.id == p.tool)
            .and_then(|t| chipload_toolpath::plan::groove_width_mm(t, end - start))
    {
        parts.push(Message::new("sheet-groove-width").arg("width", len(width)));
    }
    if let Operation::Profile(p) = op {
        // docs/specs/finish-and-skin.md §6: "finish 0.3 mm, skin 0.2 mm".
        if let Some(finish) = p.finish {
            parts.push(Message::new("sheet-finish").arg("stock", len_u(finish.stock)));
        }
        if let Some(skin) = p.skin {
            parts.push(Message::new("sheet-skin").arg("thickness", len_u(skin.thickness)));
        }
    }
    // docs/specs/pocket-finish.md §6: "finishing: walls 0.3 mm, floor 0.2 mm".
    if let Operation::Pocket(PocketOp {
        finish: Some(finish),
        ..
    }) = op
    {
        parts.push(
            Message::new("sheet-pocket-finish")
                .arg("wall", len_u(finish.wall))
                .arg("floor", len_u(finish.floor)),
        );
    }
    listed(w, &parts)
}

pub(super) fn entry_text(op: &Operation, w: Words) -> String {
    let degrees = |angle: f64| num(angle.to_degrees());
    if let Operation::Surface(_) = op {
        return w.say(&Message::new("sheet-entry-beside"));
    }
    if let Operation::Holes(h) = op {
        return w.say(&match h.method {
            HoleMethod::Drill { .. } => Message::new("sheet-entry-straight"),
            HoleMethod::Bore { ramp_angle, .. } => {
                Message::new("sheet-entry-helix").arg("degrees", degrees(ramp_angle))
            }
        });
    }
    let mut parts = vec![match op.entry() {
        Entry::Plunge {} => Message::new("sheet-entry-plunge"),
        Entry::Ramp { ramp_angle } => {
            Message::new("sheet-entry-ramp").arg("degrees", degrees(ramp_angle))
        }
        Entry::Helix { radius, ramp_angle } => Message::new("sheet-entry-helix-radius")
            .arg("radius", len(radius))
            .arg("degrees", degrees(ramp_angle)),
    }];
    if let Some(lead) = op.lead() {
        parts.push(Message::new("sheet-lead").arg("radius", len(lead.radius)));
    }
    listed(w, &parts)
}

pub(super) fn tabs_text(op: &Operation, w: Words) -> String {
    match op {
        Operation::Profile(p) => match &p.tabs {
            Some(t) if t.count > 0 => {
                let mut parts = vec![
                    Message::new("sheet-tabs")
                        .arg("count", t.count)
                        .arg("width", len(t.width))
                        .arg("height", len(t.height)),
                ];
                if !t.is_straight() {
                    let degrees = Value::Number(t.flank_angle.to_degrees(), Digits::Trim(1));
                    parts.push(Message::new("sheet-flanks").arg("degrees", degrees));
                }
                // Where they go (docs/specs/tabs-along-grain.md §4); evenly round says nothing.
                match t.along {
                    Some(chipload_job::Axis::X) => parts.push(Message::new("sheet-tabs-along-x")),
                    Some(chipload_job::Axis::Y) => parts.push(Message::new("sheet-tabs-along-y")),
                    None => {}
                }
                listed(w, &parts)
            }
            _ => w.say(&Message::new("sheet-tabs-none")),
        },
        _ => "-".into(),
    }
}
