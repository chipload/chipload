// SPDX-License-Identifier: GPL-3.0-or-later
//! The property forms: one operation, and the job settings.

use super::*;

pub(super) fn mm(value: &mut f64) -> DragValue<'_> {
    DragValue::new(value)
        .speed(0.1)
        .range(0.0..=100_000.0)
        .suffix(" mm")
}

pub(super) fn feed(value: &mut f64) -> DragValue<'_> {
    DragValue::new(value)
        .speed(10.0)
        .range(0.0..=100_000.0)
        .suffix(" mm/min")
}

pub(super) fn direction_combo(ui: &mut egui::Ui, id: (&str, usize), direction: &mut Direction) {
    ComboBox::from_id_salt(id)
        .selected_text(match direction {
            Direction::Climb => "climb",
            Direction::Conventional => "conventional",
        })
        .show_ui(ui, |ui| {
            ui.selectable_value(direction, Direction::Climb, "climb");
            ui.selectable_value(direction, Direction::Conventional, "conventional");
        });
}

/// The selected operation's fields. Returns true if "Use selection" was clicked.
pub(super) fn op_form(
    ui: &mut egui::Ui,
    index: usize,
    entry: &mut crate::editor::EditOp,
    contours: &[Contour],
) -> bool {
    let mut use_selection = false;
    let summary = geometry_summary(&entry.op, contours);
    let side_chosen = &mut entry.side_chosen;
    ui.heading(format!("Operation {}", index + 1));
    Grid::new(("op-form", index)).num_columns(2).show(ui, |ui| {
        let name = match &mut entry.op {
            Operation::Profile(p) => &mut p.name,
            Operation::Pocket(p) => &mut p.name,
        };
        ui.label("Name");
        ui.add(TextEdit::singleline(name).desired_width(200.0));
        ui.end_row();
        ui.label("Geometry");
        ui.horizontal(|ui| {
            ui.label(&summary);
            if ui
                .button("Use selection")
                .on_hover_text("Replace with the contours selected on the canvas")
                .clicked()
            {
                use_selection = true;
            }
        });
        ui.end_row();
        match &mut entry.op {
            Operation::Profile(p) => {
                ui.label("Side");
                let text = if *side_chosen {
                    match p.side {
                        Side::Inside => "inside",
                        Side::Outside => "outside",
                        Side::On => "on",
                    }
                } else {
                    "— choose —"
                };
                ComboBox::from_id_salt(("side", index))
                    .selected_text(text)
                    .show_ui(ui, |ui| {
                        for (side, label) in [
                            (Side::Outside, "outside"),
                            (Side::Inside, "inside"),
                            (Side::On, "on"),
                        ] {
                            if ui
                                .selectable_label(*side_chosen && p.side == side, label)
                                .clicked()
                            {
                                p.side = side;
                                *side_chosen = true;
                            }
                        }
                    });
                ui.end_row();
                ui.label("Direction");
                direction_combo(ui, ("direction", index), &mut p.direction);
                ui.end_row();
                ui.label("Depth");
                ui.add(mm(&mut p.depth));
                ui.end_row();
                ui.label("Step down");
                ui.add(mm(&mut p.step_down));
                ui.end_row();
                ui.label("Feed");
                ui.add(feed(&mut p.feed));
                ui.end_row();
                ui.label("Plunge feed");
                ui.add(feed(&mut p.plunge_feed));
                ui.end_row();
                ui.label("Tabs");
                let mut has_tabs = p.tabs.is_some();
                if ui.checkbox(&mut has_tabs, "holding tabs").changed() {
                    p.tabs = has_tabs.then_some(Tabs {
                        count: 4,
                        width: 8.0,
                        height: 3.0,
                    });
                }
                ui.end_row();
                if let Some(tabs) = &mut p.tabs {
                    ui.label("  count");
                    ui.add(DragValue::new(&mut tabs.count).range(1..=100));
                    ui.end_row();
                    ui.label("  width");
                    ui.add(mm(&mut tabs.width));
                    ui.end_row();
                    ui.label("  height");
                    ui.add(mm(&mut tabs.height));
                    ui.end_row();
                }
            }
            Operation::Pocket(p) => {
                ui.label("Direction");
                direction_combo(ui, ("direction", index), &mut p.direction);
                ui.end_row();
                ui.label("Depth");
                ui.add(mm(&mut p.depth));
                ui.end_row();
                ui.label("Step down");
                ui.add(mm(&mut p.step_down));
                ui.end_row();
                ui.label("Stepover");
                ui.add(
                    DragValue::new(&mut p.stepover)
                        .speed(0.01)
                        .range(0.01..=1.0)
                        .suffix(" × D"),
                );
                ui.end_row();
                ui.label("Feed");
                ui.add(feed(&mut p.feed));
                ui.end_row();
                ui.label("Plunge feed");
                ui.add(feed(&mut p.plunge_feed));
                ui.end_row();
                ui.label("Links");
                ui.checkbox(&mut p.keep_tool_down, "keep the tool down where safe");
                ui.end_row();
            }
        }
    });
    use_selection
}

/// Drawing, stock, machine and tool.
/// The G-code's X0 Y0: a choice, plus X and Y for a point.
pub(super) fn work_zero_rows(ui: &mut egui::Ui, job: &mut Job) {
    let choices = [
        (WorkZero::DrawingOrigin {}, "drawing origin"),
        (
            WorkZero::Stock {
                corner: StockCorner::Center,
            },
            "stock centre",
        ),
        (
            WorkZero::Stock {
                corner: StockCorner::BottomLeft,
            },
            "stock bottom left",
        ),
        (
            WorkZero::Stock {
                corner: StockCorner::BottomRight,
            },
            "stock bottom right",
        ),
        (
            WorkZero::Stock {
                corner: StockCorner::TopLeft,
            },
            "stock top left",
        ),
        (
            WorkZero::Stock {
                corner: StockCorner::TopRight,
            },
            "stock top right",
        ),
    ];
    let current = job.machine.work_zero;
    let selected = choices
        .iter()
        .find(|(choice, _)| *choice == current)
        .map_or("point", |(_, name)| *name);
    ui.label("Work zero X Y")
        .on_hover_text("The point that becomes X0 Y0 in the G-code. The geometry doesn't move.");
    ComboBox::from_id_salt("work_zero")
        .selected_text(selected)
        .show_ui(ui, |ui| {
            for (choice, name) in choices {
                ui.selectable_value(&mut job.machine.work_zero, choice, name);
            }
            // A point starts where the current zero is, so choosing it moves nothing.
            let (x, y) = job.work_zero_xy();
            let is_point = matches!(job.machine.work_zero, WorkZero::Point { .. });
            if ui.selectable_label(is_point, "point").clicked() && !is_point {
                job.machine.work_zero = WorkZero::Point { x, y };
            }
        });
    ui.end_row();
    if let WorkZero::Point { x, y } = &mut job.machine.work_zero {
        ui.label("");
        ui.horizontal(|ui| {
            ui.add(mm(x).prefix("X "));
            ui.add(mm(y).prefix("Y "));
        });
        ui.end_row();
    }
}

pub(super) fn job_form(ui: &mut egui::Ui, edit: &mut EditJob) {
    let job = &mut edit.settings;
    ui.heading("Job settings");
    Grid::new("job-form").num_columns(2).show(ui, |ui| {
        ui.label("Drawing");
        match &job.drawing.source {
            Some(source) => {
                ui.label(format!(
                    "{} shapes from {} ({})",
                    job.drawing.shapes.len(),
                    source.file,
                    match source.units {
                        Units::Mm => "mm",
                        Units::Inch => "inch, converted to mm",
                    }
                ));
            }
            None => {
                ui.label(format!("{} shapes", job.drawing.shapes.len()));
            }
        }
        ui.end_row();

        ui.strong("Stock");
        ui.end_row();
        ui.label("Size X × Y");
        ui.horizontal(|ui| {
            ui.add(mm(&mut job.stock.x));
            ui.add(mm(&mut job.stock.y));
        });
        ui.end_row();
        ui.label("Thickness");
        ui.add(mm(&mut job.stock.thickness));
        ui.end_row();
        // Where the stock lies relative to the drawing (file: stock.origin). Not the G-code
        // zero: that's the work zero below (docs/specs/work-zero.md).
        ui.label("Stock position");
        ComboBox::from_id_salt("origin")
            .selected_text(match job.stock.origin {
                StockOrigin::BottomLeft => "corner at drawing origin",
                StockOrigin::Center => "centred on drawing origin",
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(
                    &mut job.stock.origin,
                    StockOrigin::BottomLeft,
                    "corner at drawing origin",
                );
                ui.selectable_value(
                    &mut job.stock.origin,
                    StockOrigin::Center,
                    "centred on drawing origin",
                );
            });
        ui.end_row();
        work_zero_rows(ui, job);
        ui.label("Z zero");
        ComboBox::from_id_salt("z_zero")
            .selected_text(match job.stock.z_zero {
                ZZero::Top => "stock top",
                ZZero::Bottom => "stock bottom",
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut job.stock.z_zero, ZZero::Top, "stock top");
                ui.selectable_value(&mut job.stock.z_zero, ZZero::Bottom, "stock bottom");
            });
        ui.end_row();

        ui.strong("Machine");
        ui.end_row();
        ui.label("Post");
        ComboBox::from_id_salt("post")
            .selected_text(job.machine.post.clone())
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut job.machine.post, "grbl".to_string(), "grbl");
            });
        ui.end_row();
        ui.label("Safe Z");
        ui.add(
            DragValue::new(&mut job.machine.safe_z)
                .speed(0.1)
                .suffix(" mm"),
        );
        ui.end_row();
        ui.label("Spindle");
        ComboBox::from_id_salt("spindle")
            .selected_text(match job.machine.spindle {
                Spindle::Cw => "cw (M3)",
                Spindle::Ccw => "ccw (M4)",
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut job.machine.spindle, Spindle::Cw, "cw (M3)");
                ui.selectable_value(&mut job.machine.spindle, Spindle::Ccw, "ccw (M4)");
            });
        ui.end_row();
        ui.label("Max feed");
        ui.horizontal(|ui| {
            let mut set = job.machine.max_feed.is_some();
            if ui.checkbox(&mut set, "").changed() {
                job.machine.max_feed = set.then_some(5000.0);
            }
            if let Some(max) = &mut job.machine.max_feed {
                ui.add(feed(max));
            }
        });
        ui.end_row();
        ui.label("Travel X Y Z");
        ui.horizontal(|ui| {
            let mut set = job.machine.travel.is_some();
            if ui.checkbox(&mut set, "").changed() {
                job.machine.travel = set.then_some(Travel {
                    x: 800.0,
                    y: 600.0,
                    z: 100.0,
                });
            }
            if let Some(t) = &mut job.machine.travel {
                ui.add(mm(&mut t.x));
                ui.add(mm(&mut t.y));
                ui.add(mm(&mut t.z));
            }
        });
        ui.end_row();

        if let Some(tool) = job.tools.first_mut() {
            ui.strong(format!("Tool {}", tool.id));
            ui.end_row();
            ui.label("Type");
            ComboBox::from_id_salt("tool-type")
                .selected_text(match tool.kind {
                    ToolType::Endmill => "endmill",
                    ToolType::Drill => "drill",
                    ToolType::Vbit => "v-bit",
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut tool.kind, ToolType::Endmill, "endmill");
                    ui.selectable_value(&mut tool.kind, ToolType::Drill, "drill");
                    ui.selectable_value(&mut tool.kind, ToolType::Vbit, "v-bit");
                });
            ui.end_row();
            ui.label("Diameter");
            ui.add(mm(&mut tool.diameter));
            ui.end_row();
            ui.label("Spindle speed");
            let mut rpm = tool.rpm.unwrap_or(0.0);
            if ui
                .add(
                    DragValue::new(&mut rpm)
                        .speed(100.0)
                        .range(0.0..=100_000.0)
                        .suffix(" rpm"),
                )
                .changed()
            {
                tool.rpm = (rpm > 0.0).then_some(rpm);
            }
            ui.end_row();
            ui.label("Centre cutting");
            ComboBox::from_id_salt("center-cutting")
                .selected_text(match tool.center_cutting {
                    Some(true) => "yes",
                    Some(false) => "no",
                    None => "unknown",
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut tool.center_cutting, Some(true), "yes");
                    ui.selectable_value(&mut tool.center_cutting, Some(false), "no");
                    ui.selectable_value(&mut tool.center_cutting, None, "unknown");
                });
            ui.end_row();
        }
    });
}
