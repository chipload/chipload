// SPDX-License-Identifier: GPL-3.0-or-later
//! The forms of V-carves, follow paths, chamfers, flutes, prisms and fill patterns.

use super::*;

/// A chamfer's waste side (docs/specs/chamfer.md §2): around the shapes or within them.
pub(super) fn chamfer_side_row(ui: &mut egui::Ui, index: usize, c: &mut chipload_job::ChamferOp) {
    use chipload_job::EdgeSide;
    let name = |ui: &egui::Ui, side: EdgeSide| match side {
        EdgeSide::Outside => t!(ui, "forms-side-outside"),
        EdgeSide::Inside => t!(ui, "forms-side-inside"),
    };
    ui.label(t!(ui, "forms-side"))
        .on_hover_text(t!(ui, "forms-chamfer-side-hover"));
    let combo = ComboBox::from_id_salt(("chamfer-side", index))
        .selected_text(name(ui, c.side))
        .show_ui(ui, |ui| {
            for side in [EdgeSide::Outside, EdgeSide::Inside] {
                let label = name(ui, side);
                if ui.selectable_label(c.side == side, label).clicked() {
                    c.side = side;
                }
            }
        });
    flag(combo.response, ui, "side");
    ui.end_row();
}

/// A chamfer's sections (docs/specs/chamfer.md §6): its width, with the depth it makes with
/// the V-bit beneath, start depth, passes and direction; feeds; entry.
pub(super) fn chamfer_sections(
    ui: &mut egui::Ui,
    index: usize,
    c: &mut chipload_job::ChamferOp,
    tool: Option<&Tool>,
) {
    let units = crate::units::display(ui.ctx());
    let len_unit = chipload_pipeline::units::length_unit(units);
    let with_unit = |mm: f64| format!("{} {len_unit}", len_in(units, mm));
    let depth = tool.and_then(|t| c.depth_with(t));
    let summary = t!(
        ui,
        "forms-chamfer-cut-summary",
        width = with_unit(c.width),
        depth = depth.map_or_else(|| "?".to_string(), with_unit),
        direction = direction_name(ui, c.direction)
    );
    folded_section(ui, "op-cut", &t!(ui, "forms-section-cut"), &summary, |ui| {
        ui.label(t!(ui, "forms-chamfer-width"))
            .on_hover_text(t!(ui, "forms-chamfer-width-hover"));
        flag(ui.add(mm(&mut c.width)), ui, "width");
        ui.end_row();
        ui.label(t!(ui, "forms-chamfer-depth"));
        match (depth, tool.and_then(|t| t.angle)) {
            (Some(depth), Some(angle)) => ui.label(t!(
                ui,
                "forms-chamfer-depth-value",
                depth = with_unit(depth),
                angle = format!("{:.0}", angle.to_degrees())
            )),
            _ => ui.label(t!(ui, "forms-chamfer-needs-vbit")),
        };
        ui.end_row();
        ui.label(t!(ui, "forms-start-depth"))
            .on_hover_text(t!(ui, "forms-chamfer-start-hover"));
        flag(ui.add(mm(&mut c.start_depth)), ui, "start_depth");
        ui.end_row();
        ui.label(t!(ui, "forms-step-down"))
            .on_hover_text(t!(ui, "forms-chamfer-passes-hover"));
        let mut passes = c.step_down.is_some();
        let tick = ui.checkbox(&mut passes, t!(ui, "forms-chamfer-passes"));
        if flag(tick, ui, "step_down").changed() {
            c.step_down = passes.then(|| depth.map_or(c.width, |d| d / 2.0).max(0.1));
        }
        ui.end_row();
        if let Some(step) = &mut c.step_down {
            ui.label(format!("  {}", t!(ui, "forms-step-down")));
            flag(ui.add(mm(step)), ui, "step_down");
            ui.end_row();
        }
        ui.label(t!(ui, "forms-direction"));
        direction_combo(ui, ("direction", index), &mut c.direction);
        ui.end_row();
    });
    feeds_section(ui, &mut c.feed, &mut c.plunge_feed, tool);
    let entry_text = entry_summary(ui.lang(), &c.entry, None, units);
    let title = t!(ui, "forms-section-entry");
    folded_section(ui, "op-entry", &title, &entry_text, |ui| {
        entry_rows(ui, index, &mut c.entry, None);
    });
}

/// A flute's sections (docs/specs/fluting.md §8): its depths, with the width at the top
/// beneath, passes, ramps; feeds. Its ramps are its entry.
pub(super) fn flute_sections(
    ui: &mut egui::Ui,
    f: &mut chipload_job::FluteOp,
    tool: Option<&Tool>,
) {
    use chipload_job::RampShape;
    let units = crate::units::display(ui.ctx());
    let len_unit = chipload_pipeline::units::length_unit(units);
    let with_unit = |mm: f64| format!("{} {len_unit}", len_in(units, mm));
    let shape_name = |ui: &egui::Ui, shape: RampShape| match shape {
        RampShape::Straight => t!(ui, "forms-ramp-straight"),
        RampShape::Curved => t!(ui, "forms-ramp-curved"),
    };
    let summary = t!(
        ui,
        "forms-flute-cut-summary",
        depth = with_unit(f.end_depth - f.start_depth),
        start = with_unit(f.ramp_start),
        end = with_unit(f.ramp_end),
        shape = shape_name(ui, f.ramp_shape)
    );
    folded_section(ui, "op-cut", &t!(ui, "forms-section-cut"), &summary, |ui| {
        ui.label(t!(ui, "forms-start-depth"))
            .on_hover_text(t!(ui, "forms-flute-start-hover"));
        flag(ui.add(mm(&mut f.start_depth)), ui, "start_depth");
        ui.end_row();
        ui.label(t!(ui, "forms-flute-depth"))
            .on_hover_text(t!(ui, "forms-flute-depth-hover"));
        flag(ui.add(mm(&mut f.end_depth)), ui, "end_depth");
        ui.end_row();
        let depth = f.end_depth - f.start_depth;
        if let Some(tool) = tool
            && let Some(width) = chipload_toolpath::flute::flute_width_mm(tool, depth)
        {
            ui.label("");
            ui.label(t!(ui, "forms-flute-width-value", width = with_unit(width)));
            ui.end_row();
        }
        ui.label(t!(ui, "forms-step-down"))
            .on_hover_text(t!(ui, "forms-flute-passes-hover"));
        let mut passes = f.step_down.is_some();
        let tick = ui.checkbox(&mut passes, t!(ui, "forms-chamfer-passes"));
        if flag(tick, ui, "step_down").changed() {
            f.step_down = passes.then(|| (depth / 2.0).max(0.1));
        }
        ui.end_row();
        if let Some(step) = &mut f.step_down {
            ui.label(format!("  {}", t!(ui, "forms-step-down")));
            flag(ui.add(mm(step)), ui, "step_down");
            ui.end_row();
        }
        ui.label(t!(ui, "forms-flute-ramp-start"))
            .on_hover_text(t!(ui, "forms-flute-ramp-start-hover"));
        flag(ui.add(mm(&mut f.ramp_start)), ui, "ramp_start");
        ui.end_row();
        ui.label(t!(ui, "forms-flute-ramp-end"))
            .on_hover_text(t!(ui, "forms-flute-ramp-end-hover"));
        flag(ui.add(mm(&mut f.ramp_end)), ui, "ramp_end");
        ui.end_row();
        ui.label(t!(ui, "forms-flute-ramp"))
            .on_hover_text(t!(ui, "forms-flute-ramp-hover"));
        let combo = ComboBox::from_id_salt("flute-ramp-shape")
            .selected_text(shape_name(ui, f.ramp_shape))
            .show_ui(ui, |ui| {
                for shape in [RampShape::Straight, RampShape::Curved] {
                    let label = shape_name(ui, shape);
                    if ui.selectable_label(f.ramp_shape == shape, label).clicked() {
                        f.ramp_shape = shape;
                    }
                }
            });
        flag(combo.response, ui, "ramp_shape");
        ui.end_row();
    });
    feeds_section(ui, &mut f.feed, &mut f.plunge_feed, tool);
}

/// A prism's sections (docs/specs/prism.md §5): start depth, max depth (with how deep the
/// bit's cone reaches), passes and direction; feeds.
pub(super) fn prism_sections(
    ui: &mut egui::Ui,
    index: usize,
    f: &mut chipload_job::PrismOp,
    tool: Option<&Tool>,
) {
    let units = crate::units::display(ui.ctx());
    let len_unit = chipload_pipeline::units::length_unit(units);
    let with_unit = |mm: f64| format!("{} {len_unit}", len_in(units, mm));
    let deepest = f.deepest(tool);
    let summary = t!(
        ui,
        "forms-prism-cut-summary",
        depth = with_unit(deepest),
        direction = direction_name(ui, f.direction)
    );
    folded_section(ui, "op-cut", &t!(ui, "forms-section-cut"), &summary, |ui| {
        ui.label(t!(ui, "forms-start-depth"))
            .on_hover_text(t!(ui, "forms-prism-start-hover"));
        flag(ui.add(mm(&mut f.start_depth)), ui, "start_depth");
        ui.end_row();
        ui.label(t!(ui, "forms-prism-max-depth"))
            .on_hover_text(t!(ui, "forms-prism-max-depth-hover"));
        let mut capped = f.max_depth.is_some();
        let tick = ui.checkbox(&mut capped, t!(ui, "forms-prism-flat-top"));
        if flag(tick, ui, "max_depth").changed() {
            f.max_depth = capped.then(|| (deepest / 2.0).max(0.5));
        }
        ui.end_row();
        if let Some(max) = &mut f.max_depth {
            ui.label(format!("  {}", t!(ui, "forms-prism-max-depth")));
            flag(ui.add(mm(max)), ui, "max_depth");
            ui.end_row();
        }
        ui.label("");
        match tool.and_then(|t| t.angle) {
            Some(angle) if tool.is_some_and(|t| t.kind == chipload_job::ToolType::Vbit) => ui
                .label(t!(
                    ui,
                    "forms-prism-deepest-value",
                    depth = with_unit(deepest),
                    angle = format!("{:.0}", angle.to_degrees())
                )),
            _ => ui.label(t!(ui, "forms-chamfer-needs-vbit")),
        };
        ui.end_row();
        ui.label(t!(ui, "forms-step-down"));
        let mut passes = f.step_down.is_some();
        let tick = ui.checkbox(&mut passes, t!(ui, "forms-chamfer-passes"));
        if flag(tick, ui, "step_down").changed() {
            f.step_down = passes.then(|| (deepest / 2.0).max(0.1));
        }
        ui.end_row();
        if let Some(step) = &mut f.step_down {
            ui.label(format!("  {}", t!(ui, "forms-step-down")));
            flag(ui.add(mm(step)), ui, "step_down");
            ui.end_row();
        }
        ui.label(t!(ui, "forms-direction"));
        direction_combo(ui, ("direction", index), &mut f.direction);
        ui.end_row();
    });
    feeds_section(ui, &mut f.feed, &mut f.plunge_feed, tool);
}

/// An angle in degrees for a radian `value`, -180° to 180°.
fn degrees_field(ui: &mut egui::Ui, value: &mut f64, field: &str) {
    let mut degrees = value.to_degrees();
    let response = ui.add(
        crate::units::number(ui, &mut degrees)
            .speed(1.0)
            .range(-180.0..=180.0)
            .suffix("°"),
    );
    if flag(response, ui, field).changed() {
        *value = degrees.to_radians();
    }
}

/// A fill pattern's sections (docs/specs/fill-pattern.md §8): the pattern and its settings,
/// inside the edge; the depths with the groove's width beneath, passes, ends; feeds.
pub(super) fn pattern_sections(
    ui: &mut egui::Ui,
    f: &mut chipload_job::PatternOp,
    tool: Option<&Tool>,
) {
    use chipload_job::FillPattern;
    let units = crate::units::display(ui.ctx());
    let len_unit = chipload_pipeline::units::length_unit(units);
    let with_unit = |mm: f64| format!("{} {len_unit}", len_in(units, mm));
    use chipload_pipeline::new_ops::PatternKind;
    let kind_name = |ui: &egui::Ui, kind: PatternKind| match kind {
        PatternKind::Lines => t!(ui, "forms-pattern-lines"),
        PatternKind::Cross => t!(ui, "forms-pattern-cross"),
        PatternKind::Rings => t!(ui, "forms-pattern-rings"),
        PatternKind::Waves => t!(ui, "forms-pattern-waves"),
        PatternKind::Dashes => t!(ui, "forms-pattern-dashes"),
        PatternKind::Herringbone => t!(ui, "forms-pattern-herringbone"),
        PatternKind::Scales => t!(ui, "forms-pattern-scales"),
        PatternKind::Sunburst => t!(ui, "forms-pattern-sunburst"),
    };
    let current = PatternKind::of(&f.pattern);
    let summary = t!(
        ui,
        "forms-pattern-summary",
        kind = kind_name(ui, current),
        spacing = with_unit(f.pattern.spacing())
    );
    let title = t!(ui, "forms-section-pattern");
    folded_section(ui, "op-pattern", &title, &summary, |ui| {
        ui.label(t!(ui, "forms-pattern"));
        let mut chosen = None;
        let combo = ComboBox::from_id_salt("pattern-kind")
            .selected_text(kind_name(ui, current))
            .show_ui(ui, |ui| {
                for kind in PatternKind::ALL {
                    let label = kind_name(ui, kind);
                    if ui.selectable_label(kind == current, label).clicked() && kind != current {
                        chosen = Some(kind);
                    }
                }
            });
        if let Some(kind) = chosen {
            // The new kind's sizes from the groove; a stroke pattern fluted (fill-pattern.md
            // §14).
            *f = chipload_pipeline::new_ops::switched_pattern(f, kind, tool);
        }
        flag(combo.response, ui, "pattern");
        ui.end_row();
        let spacing_row = |ui: &mut egui::Ui, spacing: &mut f64| {
            ui.label(t!(ui, "forms-pattern-spacing"))
                .on_hover_text(t!(ui, "forms-pattern-spacing-hover"));
            flag(ui.add(mm(spacing)), ui, "pattern.spacing");
            ui.end_row();
        };
        let length_row = |ui: &mut egui::Ui, length: &mut f64| {
            ui.label(t!(ui, "forms-pattern-length"));
            flag(ui.add(mm(length)), ui, "pattern.length");
            ui.end_row();
        };
        let whole_row = |ui: &mut egui::Ui, whole: &mut bool| {
            ui.label(t!(ui, "forms-pattern-whole"))
                .on_hover_text(t!(ui, "forms-pattern-whole-hover"));
            let tick = ui.checkbox(whole, "");
            flag(tick, ui, "pattern.whole");
            ui.end_row();
        };
        let angle_row = |ui: &mut egui::Ui, angle: &mut f64| {
            ui.label(t!(ui, "forms-pattern-angle"))
                .on_hover_text(t!(ui, "forms-pattern-angle-hover"));
            degrees_field(ui, angle, "pattern.angle");
            ui.end_row();
        };
        match &mut f.pattern {
            FillPattern::Lines { angle, spacing } => {
                angle_row(ui, angle);
                spacing_row(ui, spacing);
            }
            FillPattern::Cross {
                angle,
                spacing,
                between,
            } => {
                angle_row(ui, angle);
                spacing_row(ui, spacing);
                ui.label(t!(ui, "forms-pattern-between"))
                    .on_hover_text(t!(ui, "forms-pattern-between-hover"));
                degrees_field(ui, between, "pattern.between");
                ui.end_row();
            }
            FillPattern::Rings { spacing } => spacing_row(ui, spacing),
            FillPattern::Waves {
                angle,
                spacing,
                wavelength,
                height,
            } => {
                angle_row(ui, angle);
                spacing_row(ui, spacing);
                ui.label(t!(ui, "forms-pattern-wavelength"));
                flag(ui.add(mm(wavelength)), ui, "pattern.wavelength");
                ui.end_row();
                ui.label(t!(ui, "forms-pattern-height"))
                    .on_hover_text(t!(ui, "forms-pattern-height-hover"));
                flag(ui.add(mm(height)), ui, "pattern.height");
                ui.end_row();
            }
            FillPattern::Dashes {
                angle,
                spacing,
                length,
                gap,
                whole,
            } => {
                angle_row(ui, angle);
                spacing_row(ui, spacing);
                length_row(ui, length);
                ui.label(t!(ui, "forms-pattern-gap"));
                flag(ui.add(mm(gap)), ui, "pattern.gap");
                ui.end_row();
                whole_row(ui, whole);
            }
            FillPattern::Herringbone {
                angle,
                spacing,
                length,
                whole,
            } => {
                angle_row(ui, angle);
                spacing_row(ui, spacing);
                length_row(ui, length);
                whole_row(ui, whole);
            }
            FillPattern::Scales {
                angle,
                spacing,
                radius,
                whole,
            } => {
                angle_row(ui, angle);
                spacing_row(ui, spacing);
                ui.label(t!(ui, "forms-pattern-radius"));
                flag(ui.add(mm(radius)), ui, "pattern.radius");
                ui.end_row();
                whole_row(ui, whole);
            }
            FillPattern::Sunburst {
                spacing,
                length,
                pitch,
                whole,
            } => {
                spacing_row(ui, spacing);
                length_row(ui, length);
                ui.label(t!(ui, "forms-pattern-pitch"))
                    .on_hover_text(t!(ui, "forms-pattern-pitch-hover"));
                flag(ui.add(mm(pitch)), ui, "pattern.pitch");
                ui.end_row();
                whole_row(ui, whole);
            }
        }
        ui.label(t!(ui, "forms-pattern-inside"))
            .on_hover_text(t!(ui, "forms-pattern-inside-hover"));
        let tick = ui.checkbox(&mut f.inside, "");
        flag(tick, ui, "inside");
        ui.end_row();
    });
    let depth = f.end_depth - f.start_depth;
    let cut = t!(ui, "forms-pattern-cut-summary", depth = with_unit(depth));
    folded_section(ui, "op-cut", &t!(ui, "forms-section-cut"), &cut, |ui| {
        ui.label(t!(ui, "forms-start-depth"));
        flag(ui.add(mm(&mut f.start_depth)), ui, "start_depth");
        ui.end_row();
        ui.label(t!(ui, "forms-flute-depth"));
        flag(ui.add(mm(&mut f.end_depth)), ui, "end_depth");
        ui.end_row();
        if let Some(tool) = tool
            && let Some(width) = chipload_toolpath::flute::flute_width_mm(tool, depth)
        {
            ui.label("");
            ui.label(t!(
                ui,
                "forms-pattern-width-value",
                width = with_unit(width)
            ));
            ui.end_row();
        }
        ui.label(t!(ui, "forms-step-down"));
        let mut passes = f.step_down.is_some();
        let tick = ui.checkbox(&mut passes, t!(ui, "forms-chamfer-passes"));
        if flag(tick, ui, "step_down").changed() {
            f.step_down = passes.then(|| (depth / 2.0).max(0.1));
        }
        ui.end_row();
        if let Some(step) = &mut f.step_down {
            ui.label(format!("  {}", t!(ui, "forms-step-down")));
            flag(ui.add(mm(step)), ui, "step_down");
            ui.end_row();
        }
        ui.label(t!(ui, "forms-pattern-ends"))
            .on_hover_text(t!(ui, "forms-pattern-ends-hover"));
        flag(ui.add(mm(&mut f.ramp)), ui, "ramp");
        ui.end_row();
    });
    feeds_section(ui, &mut f.feed, &mut f.plunge_feed, tool);
}

/// A V-carve's depth passes (docs/specs/vcarve-step-down.md §3): equal load, a number of
/// passes (v-carve.md decision 12), or an ordinary step down; their depths beneath.
pub(super) fn vcarve_pass_rows(
    ui: &mut egui::Ui,
    index: usize,
    v: &mut chipload_job::VcarveOp,
    depths: &[f64],
    units: Units,
) {
    let len_unit = chipload_pipeline::units::length_unit(units);
    ui.label(t!(ui, "forms-depth-passes"))
        .on_hover_text(t!(ui, "forms-depth-passes-hover"));
    let equal_load = t!(ui, "forms-depth-passes-equal-load");
    let step_down = t!(ui, "forms-step-down");
    let chosen = if v.step_down.is_some() {
        step_down.clone()
    } else {
        equal_load.clone()
    };
    ComboBox::from_id_salt(("vcarve-depth-passes", index))
        .selected_text(chosen)
        .show_ui(ui, |ui| {
            if ui
                .selectable_label(v.step_down.is_none(), equal_load)
                .clicked()
            {
                v.step_down = None;
            }
            if ui
                .selectable_label(v.step_down.is_some(), step_down)
                .clicked()
                && v.step_down.is_none()
            {
                // The first pass stays as deep as it was.
                let first = depths.first().map_or(v.end_depth, |d| d - v.start_depth);
                v.step_down = Some((first * 10.0).round().max(1.0) / 10.0);
            }
        });
    ui.end_row();
    match &mut v.step_down {
        Some(step) => {
            ui.label(t!(ui, "forms-step-down"))
                .on_hover_text(t!(ui, "forms-vcarve-step-down-hover"));
            flag(ui.add(mm(step)), ui, "step_down");
        }
        None => {
            ui.label(t!(ui, "forms-passes"))
                .on_hover_text(t!(ui, "forms-vcarve-passes-hover"));
            let most = chipload_job::MAX_VCARVE_PASSES;
            flag(
                ui.add(crate::units::number(ui, &mut v.passes).range(1..=most)),
                ui,
                "passes",
            );
        }
    }
    ui.end_row();
    if depths.len() > 1 {
        let depths: Vec<String> = depths.iter().map(|d| len_in(units, *d)).collect();
        ui.label("");
        ui.add(
            egui::Label::new(
                RichText::new(t!(
                    ui,
                    "forms-vcarve-pass-depths",
                    depths = format!("{} {len_unit}", depths.join(", "))
                ))
                .weak(),
            )
            .wrap(),
        );
        ui.end_row();
    }
}

/// With a V-bit: the groove's width at the surface, which sets the end depth when changed
/// (docs/specs/sign-maker.md §5: a border is a groove of a width). Shown only, through the
/// stock.
fn groove_width_row(ui: &mut egui::Ui, tool: Option<&Tool>, end_depth: &mut f64, through: bool) {
    let Some(tool) = tool else {
        return;
    };
    let Some(mut width) = tool.vbit_width_at(*end_depth) else {
        return;
    };
    ui.label(t!(ui, "forms-groove-width"))
        .on_hover_text(t!(ui, "forms-groove-width-hover"));
    let field = ui.add_enabled(!through, crate::units::length(&mut width));
    if field.changed()
        && let Some(depth) = tool.vbit_depth_for(width.min(tool.diameter))
    {
        *end_depth = depth;
    }
    ui.end_row();
}

/// A V-carve's sections: its cut and passes, an inlay's plug, feeds.
pub(super) fn vcarve_sections(
    ui: &mut egui::Ui,
    index: usize,
    v: &mut chipload_job::VcarveOp,
    op_tool: Option<&Tool>,
) {
    let units = crate::units::display(ui.ctx());
    let len_unit = chipload_pipeline::units::length_unit(units);
    let start = format!("{} {len_unit}", len_in(units, v.start_depth));
    let end = format!("{} {len_unit}", len_in(units, v.end_depth));
    let depths = chipload_pipeline::vcarve_pass_depths(v);
    let cut = match depths.len() {
        0 | 1 => t!(ui, "forms-vcarve-cut-one-pass", start = start, end = end),
        count => t!(
            ui,
            "forms-vcarve-cut-passes",
            start = start,
            end = end,
            count = count
        ),
    };
    folded_section(ui, "op-cut", &t!(ui, "forms-section-cut"), &cut, |ui| {
        ui.label(t!(ui, "forms-start-depth"))
            .on_hover_text(t!(ui, "forms-vcarve-start-depth-hover"));
        flag(ui.add(mm(&mut v.start_depth)), ui, "start_depth");
        ui.end_row();
        ui.label(t!(ui, "forms-end-depth"))
            .on_hover_text(t!(ui, "forms-vcarve-end-depth-hover"));
        flag(ui.add(mm(&mut v.end_depth)), ui, "end_depth");
        ui.end_row();
        vcarve_pass_rows(ui, index, v, &depths, units);
        // Corners where the outline bends less than this get no branch: the bit
        // doesn't rise into every kink (docs/specs/vcarve-smooth.md §3).
        ui.label(t!(ui, "forms-ignore-kinks"))
            .on_hover_text(t!(ui, "forms-ignore-kinks-hover"));
        flag(
            ui.add(
                crate::units::number(ui, &mut v.ignore_kinks_deg)
                    .range(0.0..=chipload_job::MAX_IGNORE_KINKS_DEG)
                    .speed(1.0)
                    .max_decimals(1)
                    .suffix("°"),
            ),
            ui,
            "ignore_kinks_deg",
        );
        ui.end_row();
    });
    // An inlay's plug (docs/specs/vcarve-inlay.md §4): its start depth and its glue
    // gap, the end depth less the start depth; setting the gap moves the end depth.
    if let Some(plug) = v.plug.as_mut() {
        let mut gap = v.end_depth - plug.start_depth;
        let summary = t!(
            ui,
            "forms-plug-summary",
            start = format!("{} {len_unit}", len_in(units, plug.start_depth)),
            gap = format!("{} {len_unit}", len_in(units, gap))
        );
        let title = t!(ui, "forms-section-plug");
        folded_section(ui, "op-plug", &title, &summary, |ui| {
            ui.label(t!(ui, "forms-plug-start"))
                .on_hover_text(t!(ui, "forms-plug-start-hover"));
            flag(ui.add(mm(&mut plug.start_depth)), ui, "plug.start_depth");
            ui.end_row();
            ui.label(t!(ui, "forms-plug-gap"))
                .on_hover_text(t!(ui, "forms-plug-gap-hover"));
            if ui.add(mm(&mut gap)).changed() {
                v.end_depth = plug.start_depth + gap;
            }
            ui.end_row();
        });
    }
    feeds_section(ui, &mut v.feed, &mut v.plunge_feed, op_tool);
}

/// A follow path's sections: its cut, feeds, entry.
pub(super) fn path_sections(
    ui: &mut egui::Ui,
    index: usize,
    p: &mut chipload_job::PathOp,
    stock: StockDepths,
    op_tool: Option<&Tool>,
) {
    let units = crate::units::display(ui.ctx());
    let len_unit = chipload_pipeline::units::length_unit(units);
    let cut = t!(
        ui,
        "forms-path-cut-summary",
        start = len_in(units, p.start_depth),
        end = len_in(units, p.end_depth),
        unit = len_unit,
        step = len_in(units, p.step_down)
    );
    folded_section(ui, "op-cut", &t!(ui, "forms-section-cut"), &cut, |ui| {
        ui.label(t!(ui, "forms-start-depth"))
            .on_hover_text(t!(ui, "forms-start-depth-hover"));
        flag(ui.add(mm(&mut p.start_depth)), ui, "start_depth");
        ui.end_row();
        let hover = t!(ui, "forms-end-depth-hover");
        end_depth_rows(ui, &mut p.end_depth, &mut p.through, stock, hover);
        groove_width_row(ui, op_tool, &mut p.end_depth, p.through.is_some());
        ui.label(t!(ui, "forms-step-down"));
        flag(ui.add(mm(&mut p.step_down)), ui, "step_down");
        ui.end_row();
    });
    feeds_section(ui, &mut p.feed, &mut p.plunge_feed, op_tool);
    let entry_text = entry_summary(ui.lang(), &p.entry, None, units);
    let title = t!(ui, "forms-section-entry");
    folded_section(ui, "op-entry", &title, &entry_text, |ui| {
        entry_rows(ui, index, &mut p.entry, None);
    });
}
