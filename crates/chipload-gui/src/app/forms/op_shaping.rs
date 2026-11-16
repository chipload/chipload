// SPDX-License-Identifier: GPL-3.0-or-later
//! The forms of holes, mouldings, surfaces and reliefs.

use super::*;

/// A Holes operation's cut, method and feeds (docs/specs/holes.md §7): a drill shows its
/// pecks and plunge feed, an endmill the helix.
pub(super) fn holes_sections(
    ui: &mut egui::Ui,
    index: usize,
    h: &mut HolesOp,
    tool: Option<&Tool>,
    stock: StockDepths,
) {
    let units = crate::units::display(ui.ctx());
    let len_unit = chipload_pipeline::units::length_unit(units);
    let feed_unit = chipload_pipeline::units::feed_unit(units);
    let method = match h.method {
        HoleMethod::Drill { peck: Some(peck) } => t!(
            ui,
            "forms-holes-drilled-pecks",
            peck = format!("{} {len_unit}", len_in(units, peck))
        ),
        HoleMethod::Drill { peck: None } => t!(ui, "forms-holes-drilled-once"),
        HoleMethod::Bore {
            step_down,
            direction,
            ..
        } => t!(
            ui,
            "forms-holes-bore",
            step = format!("{} {len_unit}", len_in(units, step_down)),
            direction = direction_name(ui, direction)
        ),
    };
    let cut = t!(
        ui,
        "forms-holes-cut-summary",
        start = len_in(units, h.start_depth),
        end = len_in(units, h.end_depth),
        unit = len_unit,
        method = method
    );
    folded_section(ui, "op-cut", &t!(ui, "forms-section-cut"), &cut, |ui| {
        ui.label(t!(ui, "forms-start-depth"))
            .on_hover_text(t!(ui, "forms-start-depth-hover"));
        flag(ui.add(mm(&mut h.start_depth)), ui, "start_depth");
        ui.end_row();
        let drill = matches!(h.method, HoleMethod::Drill { .. });
        let hover = if drill {
            t!(ui, "forms-holes-drill-end-depth-hover")
        } else {
            t!(ui, "forms-end-depth-hover")
        };
        end_depth_rows(ui, &mut h.end_depth, &mut h.through, stock, hover);
        match &mut h.method {
            HoleMethod::Drill { peck } => {
                ui.label(t!(ui, "forms-pecks"))
                    .on_hover_text(t!(ui, "forms-pecks-hover"));
                let mut on = peck.is_some();
                let check = ui.checkbox(&mut on, t!(ui, "forms-in-pecks"));
                if flag(check, ui, "method.peck").changed() {
                    // Pecks of one drill diameter, a common start (the review of 2026-11-02).
                    *peck = on.then_some(tool.map_or(DEFAULT_STEP_DOWN, |t| t.diameter));
                }
                ui.end_row();
                if let Some(peck) = peck {
                    ui.label(format!("  {}", t!(ui, "forms-peck")));
                    flag(ui.add(mm(peck)), ui, "method.peck");
                    ui.end_row();
                }
            }
            HoleMethod::Bore {
                step_down,
                ramp_angle,
                direction,
            } => {
                ui.label(t!(ui, "forms-direction"));
                direction_combo(ui, ("direction", index), direction);
                ui.end_row();
                ui.label(t!(ui, "forms-step-per-turn"))
                    .on_hover_text(t!(ui, "forms-step-per-turn-hover"));
                flag(ui.add(mm(step_down)), ui, "method.step_down");
                ui.end_row();
                ui.label(t!(ui, "forms-ramp-angle"))
                    .on_hover_text(t!(ui, "forms-bore-ramp-angle-hover"));
                let mut degrees = ramp_angle.to_degrees();
                let response = ui.add(
                    crate::units::number(ui, &mut degrees)
                        .speed(0.1)
                        .range(0.5..=MAX_RAMP_ANGLE_RAD.to_degrees())
                        .suffix("°"),
                );
                if flag(response, ui, "method.ramp_angle").changed() {
                    *ramp_angle = degrees.to_radians();
                }
                ui.end_row();
            }
        }
    });
    if matches!(h.method, HoleMethod::Drill { .. }) {
        // A drill only goes down: no cutting feed to show.
        let text = t!(
            ui,
            "forms-drill-feeds-summary",
            feed = feed_in(units, h.plunge_feed),
            unit = feed_unit
        );
        folded_section(
            ui,
            "op-feeds",
            &t!(ui, "forms-section-feeds"),
            &text,
            |ui| {
                ui.label(t!(ui, "forms-drill-feed"));
                flag(ui.add(feed(&mut h.plunge_feed)), ui, "plunge_feed");
                ui.end_row();
                if let Some(tool) = tool {
                    let load = chipload_pipeline::chip_load::chip_load_of(h.plunge_feed, tool);
                    chip_load_row(ui, load, units);
                }
            },
        );
    } else {
        feeds_section(ui, &mut h.feed, &mut h.plunge_feed, tool);
    }
}

/// A moulding's sections (docs/specs/moulding.md §9): the profile and its size, the side,
/// the ends; start depth, stepover, levels and direction; feeds.
pub(super) fn moulding_sections(
    ui: &mut egui::Ui,
    index: usize,
    m: &mut chipload_job::MouldingOp,
    tool: Option<&Tool>,
    contours: &[Contour],
) {
    use chipload_job::{MouldEnds, MouldProfile, MouldSide, RampShape};
    let units = crate::units::display(ui.ctx());
    let len_unit = chipload_pipeline::units::length_unit(units);
    let with_unit = |mm: f64| format!("{} {len_unit}", len_in(units, mm));
    let profile_name = |ui: &egui::Ui, p: &MouldProfile| match p {
        MouldProfile::Cove { .. } => t!(ui, "forms-moulding-cove"),
        MouldProfile::Roundover { .. } => t!(ui, "forms-moulding-roundover"),
        MouldProfile::Ogee { .. } => t!(ui, "forms-moulding-ogee"),
        MouldProfile::Bead { .. } => t!(ui, "forms-moulding-bead"),
        MouldProfile::Flute { .. } => t!(ui, "forms-moulding-flute"),
        MouldProfile::Shape { .. } => t!(ui, "forms-moulding-shape"),
    };
    let side_name = |ui: &egui::Ui, s: MouldSide| match s {
        MouldSide::Left => t!(ui, "forms-moulding-left"),
        MouldSide::Right => t!(ui, "forms-moulding-right"),
        MouldSide::Inside => t!(ui, "forms-side-inside"),
        MouldSide::Outside => t!(ui, "forms-side-outside"),
    };
    let summary = t!(
        ui,
        "forms-moulding-summary",
        profile = profile_name(ui, &m.profile),
        side = side_name(ui, m.side)
    );
    let title = t!(ui, "forms-section-profile");
    // Open at first: the profile is a moulding's first choice (the user, 2026-11-15).
    unfolded_section(ui, "op-moulding", &title, &summary, |ui| {
        ui.label(t!(ui, "forms-moulding-profile"));
        let size = m.profile.depth().unwrap_or(6.0);
        let choices = [
            MouldProfile::Cove { radius: size },
            MouldProfile::Roundover { radius: size },
            MouldProfile::Ogee {
                width: 2.0 * size,
                depth: size,
            },
            MouldProfile::Bead { radius: size },
            MouldProfile::Flute { radius: size },
            MouldProfile::Shape { shape: 0 },
        ];
        let current = m.profile;
        let combo = ComboBox::from_id_salt(("moulding-profile", index))
            .selected_text(profile_name(ui, &m.profile))
            .show_ui(ui, |ui| {
                for choice in choices {
                    let same =
                        std::mem::discriminant(&choice) == std::mem::discriminant(&m.profile);
                    // A drawn profile shows the shape it has, if it has one.
                    let shown = if same { current } else { choice };
                    let label = profile_name(ui, &choice);
                    let clicked = ui
                        .horizontal(|ui| {
                            let points = chipload_toolpath::moulding::preview(&shown, contours);
                            section_picture(ui, points.as_deref(), None, egui::vec2(44.0, 22.0));
                            ui.selectable_label(same, label).clicked()
                        })
                        .inner;
                    if clicked && !same {
                        m.profile = choice;
                    }
                }
            });
        flag(combo.response, ui, "profile");
        ui.end_row();
        match &mut m.profile {
            MouldProfile::Cove { radius }
            | MouldProfile::Roundover { radius }
            | MouldProfile::Bead { radius }
            | MouldProfile::Flute { radius } => {
                ui.label(t!(ui, "forms-pattern-radius"));
                flag(ui.add(mm(radius)), ui, "profile.radius");
                ui.end_row();
            }
            MouldProfile::Ogee { width, depth } => {
                ui.label(t!(ui, "forms-moulding-width"));
                flag(ui.add(mm(width)), ui, "profile.width");
                ui.end_row();
                ui.label(t!(ui, "forms-flute-depth"));
                flag(ui.add(mm(depth)), ui, "profile.depth");
                ui.end_row();
            }
            MouldProfile::Shape { shape } => {
                ui.label(t!(ui, "forms-moulding-shape-id"))
                    .on_hover_text(t!(ui, "forms-moulding-shape-hover"));
                let field = ui.add(egui::DragValue::new(shape).prefix("S").range(0..=u64::MAX));
                flag(field, ui, "profile.shape");
                ui.end_row();
            }
        }
        // The section, true to scale: the board with the profile cut into it, the path at
        // the left.
        ui.label("");
        let points = chipload_toolpath::moulding::preview(&m.profile, contours);
        let passes = tool.and_then(|t| t.cutter()).and_then(|cutter| {
            chipload_toolpath::moulding::preview_tool(&m.profile, contours, cutter, m.stepover)
        });
        section_picture(
            ui,
            points.as_deref(),
            passes.as_ref(),
            egui::vec2(170.0, 72.0),
        )
        .on_hover_text(t!(ui, "forms-moulding-preview-hover"));
        ui.end_row();
        ui.label(t!(ui, "forms-side"))
            .on_hover_text(t!(ui, "forms-moulding-side-hover"));
        let combo = ComboBox::from_id_salt(("moulding-side", index))
            .selected_text(side_name(ui, m.side))
            .show_ui(ui, |ui| {
                for side in [
                    MouldSide::Outside,
                    MouldSide::Inside,
                    MouldSide::Left,
                    MouldSide::Right,
                ] {
                    let label = side_name(ui, side);
                    if ui.selectable_label(m.side == side, label).clicked() {
                        m.side = side;
                    }
                }
            });
        flag(combo.response, ui, "side");
        ui.end_row();
        let ends_name = |ui: &egui::Ui, e: &MouldEnds| match e {
            MouldEnds::Square => t!(ui, "forms-moulding-square"),
            MouldEnds::Round => t!(ui, "forms-moulding-round"),
            MouldEnds::Taper { .. } => t!(ui, "forms-moulding-taper"),
        };
        ui.label(t!(ui, "forms-moulding-ends"))
            .on_hover_text(t!(ui, "forms-moulding-ends-hover"));
        let ends = [
            MouldEnds::Square,
            MouldEnds::Round,
            MouldEnds::Taper {
                length: 30.0,
                shape: RampShape::Straight,
            },
        ];
        let combo = ComboBox::from_id_salt(("moulding-ends", index))
            .selected_text(ends_name(ui, &m.ends))
            .show_ui(ui, |ui| {
                for choice in ends {
                    let same = std::mem::discriminant(&choice) == std::mem::discriminant(&m.ends);
                    let label = ends_name(ui, &choice);
                    if ui.selectable_label(same, label).clicked() && !same {
                        m.ends = choice;
                    }
                }
            });
        flag(combo.response, ui, "ends");
        ui.end_row();
        if let MouldEnds::Taper { length, shape } = &mut m.ends {
            ui.label(format!("  {}", t!(ui, "forms-moulding-taper-length")));
            flag(ui.add(mm(length)), ui, "ends.length");
            ui.end_row();
            ui.label(format!("  {}", t!(ui, "forms-flute-ramp")));
            let shape_name = |ui: &egui::Ui, s: RampShape| match s {
                RampShape::Straight => t!(ui, "forms-ramp-straight"),
                RampShape::Curved => t!(ui, "forms-ramp-curved"),
            };
            let combo = ComboBox::from_id_salt(("moulding-taper-shape", index))
                .selected_text(shape_name(ui, *shape))
                .show_ui(ui, |ui| {
                    for s in [RampShape::Straight, RampShape::Curved] {
                        let label = shape_name(ui, s);
                        if ui.selectable_label(*shape == s, label).clicked() {
                            *shape = s;
                        }
                    }
                });
            flag(combo.response, ui, "ends.shape");
            ui.end_row();
        }
    });
    let cut = t!(
        ui,
        "forms-moulding-cut-summary",
        stepover = with_unit(m.stepover),
        direction = direction_name(ui, m.direction)
    );
    folded_section(ui, "op-cut", &t!(ui, "forms-section-cut"), &cut, |ui| {
        ui.label(t!(ui, "forms-start-depth"));
        flag(ui.add(mm(&mut m.start_depth)), ui, "start_depth");
        ui.end_row();
        ui.label(t!(ui, "forms-moulding-stepover"))
            .on_hover_text(t!(ui, "forms-moulding-stepover-hover"));
        flag(ui.add(mm(&mut m.stepover)), ui, "stepover");
        ui.end_row();
        ui.label(t!(ui, "forms-step-down"));
        let mut levels = m.step_down.is_some();
        let tick = ui.checkbox(&mut levels, t!(ui, "forms-chamfer-passes"));
        if flag(tick, ui, "step_down").changed() {
            m.step_down = levels.then(|| tool.map_or(2.0, |t| (t.diameter / 3.0).max(0.5)));
        }
        ui.end_row();
        if let Some(step) = &mut m.step_down {
            ui.label(format!("  {}", t!(ui, "forms-step-down")));
            flag(ui.add(mm(step)), ui, "step_down");
            ui.end_row();
        }
        ui.label(t!(ui, "forms-direction"));
        direction_combo(ui, ("direction", index), &mut m.direction);
        ui.end_row();
    });
    feeds_section(ui, &mut m.feed, &mut m.plunge_feed, tool);
}

/// A small drawing of a moulding's section (docs/specs/moulding.md §9), `size` on screen:
/// the board in section with the profile cut into it, true to scale, a mark at the path;
/// with `passes`, the tool at its deepest pass and a dot at each pass's tip. `points` are
/// (distance across, Z) from `moulding::preview`; None draws a question mark (a drawn
/// profile that can't be read).
fn section_picture(
    ui: &mut egui::Ui,
    points: Option<&[(f64, f64)]>,
    passes: Option<&chipload_toolpath::moulding::PreviewTool>,
    size: egui::Vec2,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::hover());
    let painter = ui.painter_at(rect);
    let visuals = ui.visuals();
    // A wood tone, light or dark with the theme, so the board reads against the panel.
    let board = if visuals.dark_mode {
        egui::Color32::from_rgb(0x8a, 0x6a, 0x45)
    } else {
        egui::Color32::from_rgb(0xd9, 0xbc, 0x8c)
    };
    let line = visuals.text_color();
    let Some(points) = points.filter(|p| p.len() >= 2) else {
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "?",
            egui::FontId::proportional(size.y * 0.6),
            line,
        );
        return response;
    };
    let width = points.iter().map(|p| p.0).fold(0.0, f64::max).max(0.1);
    let depth = -points.iter().map(|p| p.1).fold(0.0, f64::min);
    // Room either side of the profile and below its deepest point, in mm.
    let pad = 0.25 * width.max(depth);
    // Room for the tool too, where it reaches past the profile.
    let (tool_lo, tool_hi) = passes.map_or((0.0, width), |p| {
        p.outline.iter().fold((0.0, width), |(lo, hi), q| {
            (f64::min(lo, q.0), f64::max(hi, q.0))
        })
    });
    let (x0, x1) = (tool_lo.min(0.0) - pad, tool_hi.max(width) + pad);
    let (z0, z1) = (-(depth + pad), pad * 0.5);
    let margin = 2.0;
    let scale = ((rect.width() - 2.0 * margin) as f64 / (x1 - x0))
        .min((rect.height() - 2.0 * margin) as f64 / (z1 - z0));
    let to_screen = |x: f64, z: f64| {
        egui::pos2(
            rect.left() + margin + ((x - x0) * scale) as f32,
            rect.top() + margin + ((z1 - z) * scale) as f32,
        )
    };
    // The board: an edge profile (below the surface at the path) has waste before the path.
    let edge = points.first().is_some_and(|p| p.1 < -0.01);
    let mut outline: Vec<egui::Pos2> = Vec::new();
    if edge {
        outline.push(to_screen(0.0, z0));
    } else {
        outline.push(to_screen(x0, z0));
        outline.push(to_screen(x0, 0.0));
    }
    outline.extend(points.iter().map(|&(x, z)| to_screen(x, z)));
    outline.push(to_screen(width, 0.0));
    outline.push(to_screen(x1, 0.0));
    outline.push(to_screen(x1, z0));
    // Filled in vertical strips under the outline (the profile can be concave), as one mesh
    // so no seams show between them.
    let bottom = to_screen(0.0, z0).y;
    let mut mesh = egui::epaint::Mesh::default();
    for w in outline.windows(2) {
        let (a, b) = (w[0], w[1]);
        let base = mesh.vertices.len() as u32;
        for p in [a, b, egui::pos2(b.x, bottom), egui::pos2(a.x, bottom)] {
            mesh.colored_vertex(p, board);
        }
        mesh.add_triangle(base, base + 1, base + 2);
        mesh.add_triangle(base, base + 2, base + 3);
    }
    painter.add(egui::Shape::mesh(mesh));
    painter.add(egui::Shape::line(outline, egui::Stroke::new(1.2, line)));
    // The tool at its deepest pass, and where each pass's tip goes.
    if let Some(p) = passes {
        let tool_color = visuals.selection.stroke.color;
        let outline: Vec<egui::Pos2> = p.outline.iter().map(|&(x, z)| to_screen(x, z)).collect();
        painter.add(egui::Shape::line(
            outline,
            egui::Stroke::new(1.2, tool_color),
        ));
        for &(x, z) in &p.tips {
            painter.circle_filled(to_screen(x, z), 1.6, tool_color);
        }
    }
    // The path: a short tick at the surface.
    let at = to_screen(0.0, 0.0);
    painter.line_segment(
        [at + egui::vec2(0.0, -4.0), at + egui::vec2(0.0, 2.0)],
        egui::Stroke::new(1.5, visuals.selection.stroke.color),
    );
    response
}

/// A relief's rim (docs/specs/stl-relief.md §16): none, or a flat round the outline or a
/// box, then a ramp up to the stock top.
fn rim_section(ui: &mut egui::Ui, rim: &mut Option<chipload_job::ReliefRim>) {
    use chipload_job::RimShape;
    let units = crate::units::display(ui.ctx());
    let len_unit = chipload_pipeline::units::length_unit(units);
    let summary = match rim {
        None => t!(ui, "forms-rim-none"),
        Some(r) => t!(
            ui,
            "forms-rim-summary",
            width = format!("{} {len_unit}", len_in(units, r.width)),
            shape = match r.shape {
                RimShape::Outline => t!(ui, "forms-rim-outline"),
                RimShape::Box => t!(ui, "forms-rim-box"),
            },
            angle = format!("{:.0}", r.ramp.to_degrees())
        ),
    };
    folded_section(ui, "op-rim", &t!(ui, "forms-section-rim"), &summary, |ui| {
        let mut on = rim.is_some();
        ui.label(t!(ui, "forms-section-rim"))
            .on_hover_text(t!(ui, "forms-rim-hover"));
        if ui.checkbox(&mut on, "").changed() {
            *rim = on.then_some(chipload_job::DEFAULT_RIM);
        }
        ui.end_row();
        let Some(r) = rim else {
            return;
        };
        ui.label(t!(ui, "forms-rim-shape"));
        let shape_name = |ui: &egui::Ui, shape: RimShape| match shape {
            RimShape::Outline => t!(ui, "forms-rim-outline"),
            RimShape::Box => t!(ui, "forms-rim-box"),
        };
        ComboBox::from_id_salt("rim-shape")
            .selected_text(shape_name(ui, r.shape))
            .show_ui(ui, |ui| {
                for shape in [RimShape::Outline, RimShape::Box] {
                    let name = shape_name(ui, shape);
                    ui.selectable_value(&mut r.shape, shape, name);
                }
            });
        ui.end_row();
        ui.label(t!(ui, "forms-rim-width"))
            .on_hover_text(t!(ui, "forms-rim-width-hover"));
        flag(ui.add(mm(&mut r.width)), ui, "rim.width");
        ui.end_row();
        if r.shape == RimShape::Box {
            ui.label(t!(ui, "forms-rim-corner"));
            flag(ui.add(mm(&mut r.corner_radius)), ui, "rim.corner_radius");
            ui.end_row();
        }
        if r.shape == RimShape::Outline {
            ui.label(t!(ui, "forms-rim-hull"))
                .on_hover_text(t!(ui, "forms-rim-hull-hover"));
            flag(super::hull_slider(ui, &mut r.hull), ui, "rim.hull");
            ui.end_row();
        }
        ui.label(t!(ui, "forms-rim-ramp"))
            .on_hover_text(t!(ui, "forms-rim-ramp-hover"));
        let mut degrees = r.ramp.to_degrees();
        let least = chipload_job::RIM_MIN_RAMP.to_degrees();
        let ramp = ui.add(
            crate::units::number(ui, &mut degrees)
                .speed(1.0)
                .range(least..=90.0)
                .suffix("°"),
        );
        if ramp.changed() {
            r.ramp = degrees.to_radians();
        }
        flag(ramp, ui, "rim.ramp");
        ui.end_row();
    });
}

/// A surface's sections: its cut and feeds (docs/specs/surfacing.md).
pub(super) fn surface_sections(
    ui: &mut egui::Ui,
    f: &mut chipload_job::SurfaceOp,
    op_tool: Option<&Tool>,
) {
    let units = crate::units::display(ui.ctx());
    let len_unit = chipload_pipeline::units::length_unit(units);
    let cut = t!(
        ui,
        "forms-surface-cut-summary",
        start = len_in(units, f.start_depth),
        end = len_in(units, f.end_depth),
        unit = len_unit,
        step = len_in(units, f.step_down),
        stepover = format!("{:.2}", f.stepover)
    );
    folded_section(ui, "op-cut", &t!(ui, "forms-section-cut"), &cut, |ui| {
        ui.label(t!(ui, "forms-start-depth"))
            .on_hover_text(t!(ui, "forms-start-depth-hover"));
        flag(ui.add(mm(&mut f.start_depth)), ui, "start_depth");
        ui.end_row();
        ui.label(t!(ui, "forms-end-depth"))
            .on_hover_text(t!(ui, "forms-surface-end-depth-hover"));
        flag(ui.add(mm(&mut f.end_depth)), ui, "end_depth");
        ui.end_row();
        ui.label(t!(ui, "forms-step-down"));
        flag(ui.add(mm(&mut f.step_down)), ui, "step_down");
        ui.end_row();
        ui.label(t!(ui, "forms-stepover"))
            .on_hover_text(t!(ui, "forms-surface-stepover-hover"));
        let (lo, hi) = chipload_job::SURFACE_STEPOVER;
        let stepover = ui.add(
            crate::units::number(ui, &mut f.stepover)
                .speed(0.01)
                .range(lo..=hi)
                .suffix(" × D"),
        );
        flag(stepover, ui, "stepover");
        ui.end_row();
    });
    feeds_section(ui, &mut f.feed, &mut f.plunge_feed, op_tool);
}

/// A relief's sections (docs/specs/stl-relief.md §5, §9): its cut, rim and feeds. Returns
/// the model's new depth when it was changed here: model id and depth, mm.
pub(super) fn relief_sections(
    ui: &mut egui::Ui,
    index: usize,
    r: &mut chipload_job::ReliefOp,
    op_tool: Option<&Tool>,
    tool_diameter: f64,
) -> Option<(u64, f64)> {
    let units = crate::units::display(ui.ctx());
    let len_unit = chipload_pipeline::units::length_unit(units);
    let mut relief_depth = None;
    // The plan's deep-relief warning, seen before generating (§16).
    if r.roughing.is_none() && r.end_depth > tool_diameter + chipload_core::tolerances::DEPTH_TOL_MM
    {
        ui.label(
            egui::RichText::new(t!(
                ui,
                "forms-relief-deep",
                depth = format!("{} {len_unit}", len_in(units, r.end_depth))
            ))
            .color(ui.visuals().warn_fg_color),
        );
    }
    let cut = t!(
        ui,
        "forms-relief-cut-summary",
        start = len_in(units, r.start_depth),
        end = len_in(units, r.end_depth),
        unit = len_unit,
        stepover = format!("{:.2}", r.stepover),
        angle = format!("{:.0}", r.angle.to_degrees())
    );
    folded_section(ui, "op-cut", &t!(ui, "forms-section-cut"), &cut, |ui| {
        ui.label(t!(ui, "forms-start-depth"))
            .on_hover_text(t!(ui, "forms-start-depth-hover"));
        flag(ui.add(mm(&mut r.start_depth)), ui, "start_depth");
        ui.end_row();
        // The model's depth: changed here, it is the model's that changes, once the
        // edit is finished (the user, 2026-11-14), for every relief of the model.
        ui.label(t!(ui, "forms-relief-depth"))
            .on_hover_text(t!(ui, "forms-relief-depth-hover"));
        let stored = r.end_depth - r.start_depth;
        let key = egui::Id::new(("relief-depth", index));
        let mut depth = ui
            .data(|d| d.get_temp::<(f64, f64)>(key))
            .filter(|(from, _)| *from == stored)
            .map_or(stored, |(_, draft)| draft);
        let response = ui.add(mm(&mut depth).range(0.01..=1_000.0).speed(0.1));
        if response.drag_stopped() || response.lost_focus() {
            ui.data_mut(|d| d.remove::<(f64, f64)>(key));
            if (depth - stored).abs() > chipload_core::tolerances::DEPTH_TOL_MM {
                relief_depth = Some((r.model, depth));
            }
        } else {
            ui.data_mut(|d| d.insert_temp(key, (stored, depth)));
        }
        flag(response, ui, "end_depth");
        ui.end_row();
        ui.label(t!(ui, "forms-stepover"))
            .on_hover_text(t!(ui, "forms-relief-stepover-hover"));
        let stepover = ui.add(
            crate::units::number(ui, &mut r.stepover)
                .speed(0.01)
                .range(0.01..=chipload_job::RELIEF_MAX_STEPOVER)
                .suffix(" × D"),
        );
        flag(stepover, ui, "stepover");
        ui.end_row();
        // A ball nose's ridges between lines (docs/specs/stl-relief.md §15).
        if let Some(ridge) =
            op_tool.and_then(|t| chipload_pipeline::new_ops::relief_ridge(t, r.stepover))
        {
            ui.label(t!(ui, "forms-relief-ridge"))
                .on_hover_text(t!(ui, "forms-relief-ridge-hover"));
            ui.weak(format!("{} {len_unit}", len_in(units, ridge)));
            ui.end_row();
        }
        ui.label(t!(ui, "forms-relief-angle"))
            .on_hover_text(t!(ui, "forms-relief-angle-hover"));
        let mut degrees = r.angle.to_degrees();
        if ui
            .add(
                crate::units::number(ui, &mut degrees)
                    .speed(1.0)
                    .range(-180.0..=180.0)
                    .suffix("°"),
            )
            .changed()
        {
            r.angle = degrees.to_radians();
        }
        ui.end_row();
        ui.label("");
        ui.checkbox(&mut r.zigzag, t!(ui, "forms-relief-zigzag"))
            .on_hover_text(t!(ui, "forms-relief-zigzag-hover"));
        ui.end_row();
    });
    rim_section(ui, &mut r.rim);
    feeds_section(ui, &mut r.feed, &mut r.plunge_feed, op_tool);
    relief_depth
}
