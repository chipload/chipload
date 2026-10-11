// SPDX-License-Identifier: GPL-3.0-or-later
//! The property forms: one operation, and the job settings.

use super::user_library::{LibraryRows, library_rows};
use super::*;
use crate::editor::DEFAULT_STEP_DOWN;
use chipload_job::Placement;
use chipload_job::{Copies, CopyOrder, MAX_COPIES_PER_SIDE};
use chipload_job::{DEFAULT_RAMP_ANGLE_RAD, Entry, Lead, MAX_RAMP_ANGLE_RAD, Machine, Tool};
use chipload_pipeline::copies::order_name;
use chipload_pipeline::library::Library;
use chipload_pipeline::placement::{Move as PlaceMove, placement_text};

pub(super) fn mm(value: &mut f64) -> DragValue<'_> {
    DragValue::new(value)
        .speed(0.1)
        .range(0.0..=100_000.0)
        .suffix(" mm")
}

/// Start depth, end depth and clearance (docs/specs/depths-and-clearance.md).
fn depth_rows(
    ui: &mut egui::Ui,
    start: &mut f64,
    end: &mut f64,
    clearance: &mut f64,
    can_clear: bool,
) {
    ui.label("Start depth")
        .on_hover_text("Where the cut starts, down from the stock top");
    ui.add(mm(start));
    ui.end_row();
    ui.label("End depth")
        .on_hover_text("Where the cut ends, down from the stock top");
    ui.add(mm(end));
    ui.end_row();
    ui.label("Clearance").on_hover_text(
        "How far the cut moves into the material that is kept: positive makes a pocket \
         larger or a part smaller, negative leaves material. Less than the tool radius.",
    );
    ui.add_enabled(
        can_clear,
        DragValue::new(clearance)
            .speed(0.01)
            .range(-100_000.0..=100_000.0)
            .suffix(" mm"),
    )
    .on_disabled_hover_text("A profile on the line has no side to clear towards");
    ui.end_row();
}

/// Rings, trochoidal (loop radius, step; docs/specs/trochoidal.md) or adaptive (load;
/// docs/specs/adaptive.md). Switching fills defaults for the current tool, visibly.
fn strategy_rows(ui: &mut egui::Ui, index: usize, strategy: &mut PocketStrategy, diameter: f64) {
    ui.label("Strategy");
    let current = match strategy {
        PocketStrategy::Rings {} => "rings",
        PocketStrategy::Trochoidal { .. } => "trochoidal",
        PocketStrategy::Adaptive { .. } => "adaptive",
    };
    ComboBox::from_id_salt(("strategy", index))
        .selected_text(current)
        .show_ui(ui, |ui| {
            if ui.selectable_label(current == "rings", "rings").clicked() {
                *strategy = PocketStrategy::Rings {};
            }
            if ui
                .selectable_label(current == "trochoidal", "trochoidal")
                .on_hover_text(
                    "Small loops that take a thin bite each: for hard materials and deep cuts",
                )
                .clicked()
                && current != "trochoidal"
            {
                *strategy = PocketStrategy::Trochoidal {
                    loop_radius: 0.25 * diameter,
                    step: 0.1 * diameter,
                };
            }
            if ui
                .selectable_label(current == "adaptive", "adaptive")
                .on_hover_text(
                    "A path steered to cut about the same amount everywhere, corners too",
                )
                .clicked()
                && current != "adaptive"
            {
                *strategy = PocketStrategy::Adaptive { load: 0.1 };
            }
        });
    ui.end_row();
    match strategy {
        PocketStrategy::Rings {} => {}
        PocketStrategy::Trochoidal { loop_radius, step } => {
            ui.label("  loop radius")
                .on_hover_text("At most the tool radius");
            ui.add(mm(loop_radius));
            ui.end_row();
            ui.label("  step per loop")
                .on_hover_text("How far each loop moves on; at most the loop radius");
            ui.add(mm(step));
            ui.end_row();
        }
        PocketStrategy::Adaptive { load } => {
            ui.label("  load").on_hover_text(
                "The sideways cut to keep, as a fraction of the tool diameter (0 < load <= 0.5)",
            );
            ui.add(
                DragValue::new(load)
                    .speed(0.01)
                    .range(0.01..=0.5)
                    .suffix(" × D"),
            );
            ui.end_row();
        }
    }
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

/// Least width of a form's label column, so the sections' grids line up.
const LABEL_WIDTH: f32 = 130.0;

/// A collapsible section of a form: a bold title with a short summary beside it (seen when
/// folded), and its rows in a two-column grid. Its open state is kept per `key`.
pub(super) fn section<R>(
    ui: &mut egui::Ui,
    key: &str,
    title: &str,
    summary: &str,
    rows: impl FnOnce(&mut egui::Ui) -> R,
) -> Option<R> {
    let mut header = egui::text::LayoutJob::default();
    let style = ui.style().clone();
    RichText::new(title).strong().append_to(
        &mut header,
        &style,
        egui::FontSelection::Default,
        Align::Center,
    );
    if !summary.is_empty() {
        RichText::new(format!("   {summary}")).weak().append_to(
            &mut header,
            &style,
            egui::FontSelection::Default,
            Align::Center,
        );
    }
    egui::CollapsingHeader::new(header)
        .id_salt(("section", key))
        .default_open(true)
        .show(ui, |ui| {
            Grid::new(("section-grid", key))
                .num_columns(2)
                .min_col_width(LABEL_WIDTH)
                .show(ui, rows)
                .inner
        })
        .body_returned
}

/// A length for a summary: no trailing zeros.
fn len(value: f64) -> String {
    let text = format!("{value:.3}");
    text.trim_end_matches('0').trim_end_matches('.').to_string()
}

fn direction_name(direction: Direction) -> &'static str {
    match direction {
        Direction::Climb => "climb",
        Direction::Conventional => "conventional",
    }
}

fn entry_summary(entry: &Entry, lead: Option<&Lead>) -> String {
    let entry = match entry {
        Entry::Plunge {} => "straight plunge".to_string(),
        Entry::Ramp { ramp_angle } => format!("ramp {:.1}°", ramp_angle.to_degrees()),
        Entry::Helix { radius, ramp_angle } => format!(
            "helix r {} mm, {:.1}°",
            len(*radius),
            ramp_angle.to_degrees()
        ),
    };
    match lead {
        Some(lead) => format!("{entry}, lead {} mm", len(lead.radius)),
        None => entry,
    }
}

/// The selected operation's fields, in sections. Returns true if "Use selection" was
/// clicked.
pub(super) fn op_form(
    ui: &mut egui::Ui,
    index: usize,
    entry: &mut crate::editor::EditOp,
    contours: &[Contour],
    tool_diameter: f64,
) -> bool {
    let mut use_selection = false;
    let summary = geometry_summary(&entry.op, contours);
    let side_chosen = &mut entry.side_chosen;
    ui.heading(format!("Operation {}", index + 1));
    Grid::new(("op-form", index))
        .num_columns(2)
        // Past the sections' indent, so the values line up with theirs.
        .min_col_width(LABEL_WIDTH + ui.spacing().indent)
        .show(ui, |ui| {
            let name = entry.op.name_mut();
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
            if let Operation::Profile(p) = &mut entry.op {
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
            }
        });
    match &mut entry.op {
        Operation::Profile(p) => {
            let cut = format!(
                "{}–{} mm, steps of {} mm, {}",
                len(p.start_depth),
                len(p.end_depth),
                len(p.step_down),
                direction_name(p.direction)
            );
            section(ui, "op-cut", "Cut", &cut, |ui| {
                ui.label("Direction");
                direction_combo(ui, ("direction", index), &mut p.direction);
                ui.end_row();
                let can_clear = p.side != Side::On;
                if !can_clear {
                    p.clearance = 0.0;
                }
                depth_rows(
                    ui,
                    &mut p.start_depth,
                    &mut p.end_depth,
                    &mut p.clearance,
                    can_clear,
                );
                ui.label("Step down");
                ui.add(mm(&mut p.step_down));
                ui.end_row();
            });
            feeds_section(ui, &mut p.feed, &mut p.plunge_feed);
            let entry_text = entry_summary(&p.entry, p.lead.as_ref());
            section(ui, "op-entry", "Entry and lead", &entry_text, |ui| {
                entry_rows(ui, index, &mut p.entry, None);
                // On the line there is no waste side to put a lead on.
                let can_lead = p.side != Side::On || p.lead.is_some();
                lead_rows(ui, &mut p.lead, tool_diameter, can_lead);
            });
            let tabs_text = p.tabs.as_ref().map_or("none".to_string(), |t| {
                format!(
                    "{} × {} mm, {} mm high",
                    t.count,
                    len(t.width),
                    len(t.height)
                )
            });
            section(ui, "op-tabs", "Tabs", &tabs_text, |ui| {
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
            });
        }
        Operation::Pocket(p) => {
            let cut = format!(
                "{}–{} mm, steps of {} mm, {}",
                len(p.start_depth),
                len(p.end_depth),
                len(p.step_down),
                direction_name(p.direction)
            );
            section(ui, "op-cut", "Cut", &cut, |ui| {
                ui.label("Direction");
                direction_combo(ui, ("direction", index), &mut p.direction);
                ui.end_row();
                depth_rows(
                    ui,
                    &mut p.start_depth,
                    &mut p.end_depth,
                    &mut p.clearance,
                    true,
                );
                ui.label("Step down");
                ui.add(mm(&mut p.step_down));
                ui.end_row();
            });
            let strategy = match p.strategy {
                PocketStrategy::Rings {} => "rings",
                PocketStrategy::Trochoidal { .. } => "trochoidal",
                PocketStrategy::Adaptive { .. } => "adaptive",
            };
            let clearing = format!("{strategy}, stepover {:.2} × D", p.stepover);
            section(ui, "op-clearing", "Clearing", &clearing, |ui| {
                strategy_rows(ui, index, &mut p.strategy, tool_diameter);
                ui.label("Stepover");
                ui.add(
                    DragValue::new(&mut p.stepover)
                        .speed(0.01)
                        .range(0.01..=1.0)
                        .suffix(" × D"),
                );
                ui.end_row();
                ui.label("Links");
                ui.checkbox(&mut p.keep_tool_down, "keep the tool down where safe");
                ui.end_row();
            });
            feeds_section(ui, &mut p.feed, &mut p.plunge_feed);
            let entry_text = entry_summary(&p.entry, p.lead.as_ref());
            section(ui, "op-entry", "Entry and lead", &entry_text, |ui| {
                entry_rows(ui, index, &mut p.entry, Some(tool_diameter));
                lead_rows(ui, &mut p.lead, tool_diameter, true);
            });
        }
        Operation::Vcarve(v) => {
            let passes = v.step_down.map_or("one pass".to_string(), |s| {
                format!("steps of {} mm", len(s))
            });
            let cut = format!(
                "from {} mm, at most {} mm deep, {passes}",
                len(v.start_depth),
                len(v.end_depth)
            );
            section(ui, "op-cut", "Cut", &cut, |ui| {
                ui.label("Start depth").on_hover_text(
                    "The surface carved, down from the stock top (0: the top; more to carve \
                     into a pocket floor)",
                );
                ui.add(mm(&mut v.start_depth));
                ui.end_row();
                ui.label("Max depth").on_hover_text(
                    "The deepest the tip goes, from the stock top. Wider strokes are carved to \
                     it and reported",
                );
                ui.add(mm(&mut v.end_depth));
                ui.end_row();
                ui.label("Passes");
                let mut stepped = v.step_down.is_some();
                if ui.checkbox(&mut stepped, "in steps").changed() {
                    v.step_down = stepped.then_some(DEFAULT_STEP_DOWN);
                }
                ui.end_row();
                if let Some(step) = &mut v.step_down {
                    ui.label("  step down");
                    ui.add(mm(step));
                    ui.end_row();
                }
            });
            feeds_section(ui, &mut v.feed, &mut v.plunge_feed);
        }
    }
    use_selection
}

fn feeds_section(ui: &mut egui::Ui, cut: &mut f64, plunge: &mut f64) {
    let text = format!("{} / {} mm/min", len(*cut), len(*plunge));
    section(ui, "op-feeds", "Feeds", &text, |ui| {
        ui.label("Feed");
        ui.add(feed(cut));
        ui.end_row();
        ui.label("Plunge feed");
        ui.add(feed(plunge));
        ui.end_row();
    });
}

/// How the tool goes down into each pass (docs/specs/ramp-entry.md); angles are shown in
/// degrees and kept in radians. `helix`: the tool diameter, for a pocket, which may also go
/// down round a helix (docs/specs/helix-and-leads.md part C).
fn entry_rows(ui: &mut egui::Ui, index: usize, entry: &mut Entry, helix: Option<f64>) {
    ui.label("Entry").on_hover_text(
        "How the tool goes down into the material. A ramp goes down along the path and back, \
         a helix (pockets) round a small circle, so the tool needn't be centre-cutting; both \
         use the cutting feed.",
    );
    let ramp = Entry::Ramp {
        ramp_angle: DEFAULT_RAMP_ANGLE_RAD,
    };
    ComboBox::from_id_salt(("entry", index))
        .selected_text(match entry {
            Entry::Plunge {} => "straight plunge",
            Entry::Ramp { .. } => "ramp",
            Entry::Helix { .. } => "helix",
        })
        .show_ui(ui, |ui| {
            let is_ramp = matches!(entry, Entry::Ramp { .. });
            let is_helix = matches!(entry, Entry::Helix { .. });
            if ui
                .selectable_label(!is_ramp && !is_helix, "straight plunge")
                .clicked()
            {
                *entry = Entry::Plunge {};
            }
            if ui.selectable_label(is_ramp, "ramp").clicked() && !is_ramp {
                *entry = ramp;
            }
            if let Some(diameter) = helix
                && ui.selectable_label(is_helix, "helix").clicked()
                && !is_helix
            {
                *entry = Entry::default_helix(diameter);
            }
        });
    ui.end_row();
    if let Entry::Helix { radius, .. } = entry {
        ui.label("  helix radius").on_hover_text(
            "Of the circle the tool centre follows: less than the tool radius, so no core is \
             left standing",
        );
        ui.add(mm(radius));
        ui.end_row();
    }
    if let Entry::Ramp { ramp_angle } | Entry::Helix { ramp_angle, .. } = entry {
        ui.label("  ramp angle");
        let mut degrees = ramp_angle.to_degrees();
        if ui
            .add(
                DragValue::new(&mut degrees)
                    .speed(0.1)
                    .range(0.5..=MAX_RAMP_ANGLE_RAD.to_degrees())
                    .suffix("°"),
            )
            .changed()
        {
            *ramp_angle = degrees.to_radians();
        }
        ui.end_row();
    }
}

/// Lead-in and lead-out (docs/specs/helix-and-leads.md part B): on or off, and the radius,
/// the tool radius by default.
fn lead_rows(ui: &mut egui::Ui, lead: &mut Option<Lead>, tool_diameter: f64, enabled: bool) {
    ui.label("Lead").on_hover_text(
        "Arc onto the path from the waste and off it again, so the tool goes down in the \
         waste and leaves no mark on the part. A lead without room shrinks, or is left out \
         (the report says so).",
    );
    let mut on = lead.is_some();
    let response = ui
        .add_enabled(
            enabled,
            egui::Checkbox::new(&mut on, "lead-in and lead-out"),
        )
        .on_disabled_hover_text("A profile cut on the line has no waste side for a lead");
    if response.changed() {
        *lead = on.then_some(Lead {
            radius: tool_diameter / 2.0,
        });
    }
    ui.end_row();
    if let Some(lead) = lead {
        ui.label("  radius");
        ui.add(mm(&mut lead.radius));
        ui.end_row();
    }
}

/// Drawing, stock, machine and tool.
/// The G-code's X0 Y0: a choice, plus X and Y for a point. A point starts at `here`
/// (where the current zero is), so choosing it moves nothing.
pub(super) fn work_zero_rows(
    ui: &mut egui::Ui,
    salt: &str,
    work_zero: &mut WorkZero,
    here: (f64, f64),
) {
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
    let current = *work_zero;
    let selected = choices
        .iter()
        .find(|(choice, _)| *choice == current)
        .map_or("point", |(_, name)| *name);
    ui.label("Work zero X Y")
        .on_hover_text("The point that becomes X0 Y0 in the G-code. The geometry doesn't move.");
    ComboBox::from_id_salt((salt, "work_zero"))
        .selected_text(selected)
        .show_ui(ui, |ui| {
            for (choice, name) in choices {
                ui.selectable_value(work_zero, choice, name);
            }
            let (x, y) = here;
            let is_point = matches!(work_zero, WorkZero::Point { .. });
            if ui.selectable_label(is_point, "point").clicked() && !is_point {
                *work_zero = WorkZero::Point { x, y };
            }
        });
    ui.end_row();
    if let WorkZero::Point { x, y } = work_zero {
        ui.label("");
        ui.horizontal(|ui| {
            ui.add(mm(x).prefix("X "));
            ui.add(mm(y).prefix("Y "));
        });
        ui.end_row();
    }
}

/// What the job form asks of the app beyond editing values.
pub(super) enum JobRequest {
    /// docs/specs/tool-library-and-machines.md §8.
    Library(LibraryRequest),
    /// Set the copies' pitch so this many mm are left between their cuts.
    CopiesGap(f64),
    /// Move the imported drawing (docs/specs/drawing-placement.md).
    Place(PlaceMove),
}

/// Gap the *Set pitch* helper starts with, mm; only an input, never saved.
const DEFAULT_COPIES_GAP: f64 = 10.0;

pub(super) fn job_form(
    ui: &mut egui::Ui,
    edit: &mut EditJob,
    library: Option<&Library>,
) -> Option<JobRequest> {
    let mut request = None;
    let mut copies_request = None;
    let job = &mut edit.settings;
    ui.heading("Job settings");
    Grid::new("job-form")
        .num_columns(2)
        // Past the sections' indent, so the values line up with theirs.
        .min_col_width(LABEL_WIDTH + ui.spacing().indent)
        .show(ui, |ui| {
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
        });

    let stock = format!(
        "{} × {} × {} mm",
        len(job.stock.x),
        len(job.stock.y),
        len(job.stock.thickness)
    );
    section(ui, "job-stock", "Stock", &stock, |ui| {
        ui.label("Size X × Y");
        ui.horizontal(|ui| {
            ui.add(mm(&mut job.stock.x));
            ui.add(mm(&mut job.stock.y));
        });
        ui.end_row();
        ui.label("Thickness");
        ui.add(mm(&mut job.stock.thickness));
        ui.end_row();
        ui.label("Max depth below stock").on_hover_text(
            "How far a cut may go below the stock (into the spoilboard). Cuts within it are \
             warned about; deeper ones are refused.",
        );
        ui.add(mm(&mut job.stock.max_overcut));
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
        let here = job.work_zero_xy();
        work_zero_rows(ui, "job", &mut job.machine.work_zero, here);
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
    });

    let imported = job.drawing.shapes.iter().any(|s| !s.is_drawn());
    if let (true, Some(source)) = (imported, &job.drawing.source) {
        let placement = source.placement;
        let summary = placement_text(placement);
        section(ui, "job-placement", "Drawing position", &summary, |ui| {
            placement_rows(ui, placement, &mut copies_request);
        });
    }

    let copies = job.copies.map_or("one part".to_string(), |c| {
        format!("{} × {} = {}", c.columns, c.rows, c.count())
    });
    section(ui, "job-copies", "Copies", &copies, |ui| {
        copies_rows(ui, &mut job.copies, &mut copies_request);
    });

    let machine = format!(
        "{}, safe Z {} mm, {}",
        job.machine.post,
        len(job.machine.safe_z),
        match job.machine.spindle {
            Spindle::Cw => "M3",
            Spindle::Ccw => "M4",
        }
    );
    section(ui, "job-machine", "Machine", &machine, |ui| {
        machine_rows(ui, "job", &mut job.machine);
        library_rows(ui, &mut request, LibraryRows::machine(job, library));
    });

    if let Some(tool) = job.tools.first() {
        let title = format!("Tool {}", tool.id);
        let kind = match tool.kind {
            ToolType::Endmill => "endmill",
            ToolType::Drill => "drill",
            ToolType::Vbit => "v-bit",
        };
        let text = format!("{kind} Ø{} mm", len(tool.diameter));
        section(ui, "job-tool", &title, &text, |ui| {
            if let Some(tool) = job.tools.first() {
                library_rows(ui, &mut request, LibraryRows::tool(tool, library));
            }
            if let Some(tool) = job.tools.first_mut() {
                tool_rows(ui, "job", tool);
            }
        });
    }
    copies_request.or(request.map(JobRequest::Library))
}

/// Margin the *Bottom left* button starts with, mm; only an input, never saved.
const DEFAULT_PLACE_MARGIN: f64 = 10.0;

/// Moving the imported drawing (docs/specs/drawing-placement.md §5). The values typed here
/// are inputs only: nothing moves until a button is pressed, so each move is one undo step.
fn placement_rows(ui: &mut egui::Ui, placement: Placement, request: &mut Option<JobRequest>) {
    ui.label("From the DXF").on_hover_text(
        "Where the DXF's shapes are, compared with the DXF itself. Updates from the DXF \
         keep this. Shapes drawn in Chipload don't move.",
    );
    ui.horizontal(|ui| {
        let turned = match placement.quarter_turns {
            0 => String::new(),
            k => format!("turned {}°, ", 90 * u32::from(k)),
        };
        ui.label(format!(
            "{turned}moved X {}  Y {} mm",
            len(placement.dx),
            len(placement.dy)
        ));
        if placement != Placement::default()
            && ui
                .button("Reset")
                .on_hover_text("Back to where the DXF has it: no turn, no move.")
                .clicked()
        {
            *request = Some(JobRequest::Place(PlaceMove::Reset));
        }
    });
    ui.end_row();
    ui.label("  turn");
    ui.horizontal(|ui| {
        if ui
            .button("⟲ 90°")
            .on_hover_text("Counter-clockwise, about the drawing's centre.")
            .clicked()
        {
            *request = Some(JobRequest::Place(PlaceMove::Turn(1)));
        }
        if ui
            .button("⟳ 90°")
            .on_hover_text("Clockwise, about the drawing's centre.")
            .clicked()
        {
            *request = Some(JobRequest::Place(PlaceMove::Turn(-1)));
        }
    });
    ui.end_row();
    ui.label("  on the stock");
    ui.horizontal(|ui| {
        let id = egui::Id::new("place-margin");
        let mut margin = ui.data_mut(|d| *d.get_temp_mut_or(id, DEFAULT_PLACE_MARGIN));
        if ui
            .button("Bottom left")
            .on_hover_text("The drawing's lower-left corner this far in from the stock's.")
            .clicked()
        {
            *request = Some(JobRequest::Place(PlaceMove::BottomLeft { margin }));
        }
        ui.add(mm(&mut margin).prefix("margin "));
        ui.data_mut(|d| d.insert_temp(id, margin));
        if ui.button("Centre").clicked() {
            *request = Some(JobRequest::Place(PlaceMove::Center));
        }
    });
    ui.end_row();
    ui.label("  move by X Y");
    ui.horizontal(|ui| {
        let id = egui::Id::new("place-by");
        let (mut dx, mut dy) = ui.data_mut(|d| *d.get_temp_mut_or(id, (0.0, 0.0)));
        for value in [&mut dx, &mut dy] {
            ui.add(
                DragValue::new(value)
                    .speed(0.1)
                    .range(-100_000.0..=100_000.0)
                    .suffix(" mm"),
            );
        }
        ui.data_mut(|d| d.insert_temp(id, (dx, dy)));
        if ui.button("Move").clicked() {
            *request = Some(JobRequest::Place(PlaceMove::By(dx, dy)));
        }
    });
    ui.end_row();
}

/// The part cut several times in a grid (docs/specs/copies-grid.md).
fn copies_rows(ui: &mut egui::Ui, copies: &mut Option<Copies>, request: &mut Option<JobRequest>) {
    ui.label("Copies").on_hover_text(
        "Cut the part several times in a grid. The copies are drawn faded; only the drawing \
         itself can be selected.",
    );
    let mut on = copies.is_some();
    if ui.checkbox(&mut on, "several, in a grid").changed() {
        *copies = on.then_some(Copies {
            columns: 2,
            rows: 1,
            dx: 0.0,
            dy: 0.0,
            order: CopyOrder::ByOperation,
        });
        if on {
            *request = Some(JobRequest::CopiesGap(DEFAULT_COPIES_GAP));
        }
    }
    ui.end_row();
    let Some(grid) = copies else {
        return;
    };
    ui.label("  columns × rows");
    ui.horizontal(|ui| {
        let count = 1..=MAX_COPIES_PER_SIDE;
        ui.add(DragValue::new(&mut grid.columns).range(count.clone()));
        ui.add(DragValue::new(&mut grid.rows).range(count));
        ui.label(format!("= {}", grid.count()));
    });
    ui.end_row();
    ui.label("  pitch X Y")
        .on_hover_text("Copy to copy, mm. Negative grows to the left or down.");
    ui.horizontal(|ui| {
        for value in [&mut grid.dx, &mut grid.dy] {
            ui.add(
                DragValue::new(value)
                    .speed(0.1)
                    .range(-100_000.0..=100_000.0)
                    .suffix(" mm"),
            );
        }
    });
    ui.end_row();
    ui.label("  gap").on_hover_text(
        "Sets the pitch once so this much is left between the cuts of neighbouring copies. \
         The project keeps the pitch.",
    );
    ui.horizontal(|ui| {
        let id = egui::Id::new("copies-gap");
        let mut gap = ui.data_mut(|d| *d.get_temp_mut_or(id, DEFAULT_COPIES_GAP));
        ui.add(mm(&mut gap));
        ui.data_mut(|d| d.insert_temp(id, gap));
        if ui.button("Set pitch").clicked() {
            *request = Some(JobRequest::CopiesGap(gap));
        }
    });
    ui.end_row();
    ui.label("  order");
    ComboBox::from_id_salt("copies-order")
        .selected_text(order_name(grid.order))
        .show_ui(ui, |ui| {
            for order in [CopyOrder::ByOperation, CopyOrder::ByPart] {
                ui.selectable_value(&mut grid.order, order, order_name(order));
            }
        });
    ui.end_row();
}

/// Post, safe Z, spindle, max feed and travel (the work zero is with the stock).
pub(super) fn machine_rows(ui: &mut egui::Ui, salt: &str, machine: &mut Machine) {
    ui.label("Post");
    ComboBox::from_id_salt((salt, "post"))
        .selected_text(machine.post.clone())
        .show_ui(ui, |ui| {
            ui.selectable_value(&mut machine.post, "grbl".to_string(), "grbl");
        });
    ui.end_row();
    ui.label("Safe Z");
    ui.add(DragValue::new(&mut machine.safe_z).speed(0.1).suffix(" mm"));
    ui.end_row();
    ui.label("Spindle");
    ComboBox::from_id_salt((salt, "spindle"))
        .selected_text(match machine.spindle {
            Spindle::Cw => "cw (M3)",
            Spindle::Ccw => "ccw (M4)",
        })
        .show_ui(ui, |ui| {
            ui.selectable_value(&mut machine.spindle, Spindle::Cw, "cw (M3)");
            ui.selectable_value(&mut machine.spindle, Spindle::Ccw, "ccw (M4)");
        });
    ui.end_row();
    ui.label("Max feed");
    ui.horizontal(|ui| {
        let mut set = machine.max_feed.is_some();
        if ui.checkbox(&mut set, "").changed() {
            machine.max_feed = set.then_some(5000.0);
        }
        if let Some(max) = &mut machine.max_feed {
            ui.add(feed(max));
        }
    });
    ui.end_row();
    ui.label("Travel X Y Z");
    ui.horizontal(|ui| {
        let mut set = machine.travel.is_some();
        if ui.checkbox(&mut set, "").changed() {
            machine.travel = set.then_some(Travel {
                x: 800.0,
                y: 600.0,
                z: 100.0,
            });
        }
        if let Some(t) = &mut machine.travel {
            ui.add(mm(&mut t.x));
            ui.add(mm(&mut t.y));
            ui.add(mm(&mut t.z));
        }
    });
    ui.end_row();
}

/// Type, diameter, spindle speed and centre cutting.
pub(super) fn tool_rows(ui: &mut egui::Ui, salt: &str, tool: &mut Tool) {
    ui.label("Type");
    ComboBox::from_id_salt((salt, "tool-type"))
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
    ComboBox::from_id_salt((salt, "center-cutting"))
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
    if tool.kind == ToolType::Vbit {
        vbit_rows(ui, tool);
    }
}

/// A V-bit's included angle (degrees here, radians in the file) and flat tip
/// (docs/specs/v-carve.md §2). Both can be left unset; V-carving needs the angle.
fn vbit_rows(ui: &mut egui::Ui, tool: &mut Tool) {
    ui.label("Angle")
        .on_hover_text("The included angle of the V; V-carving needs it");
    ui.horizontal(|ui| {
        let mut set = tool.angle.is_some();
        if ui.checkbox(&mut set, "").changed() {
            tool.angle = set.then_some(60f64.to_radians());
        }
        if let Some(angle) = &mut tool.angle {
            let mut degrees = angle.to_degrees();
            if ui
                .add(
                    DragValue::new(&mut degrees)
                        .speed(0.5)
                        .range(1.0..=179.0)
                        .suffix("°"),
                )
                .changed()
            {
                *angle = degrees.to_radians();
            }
        } else {
            ui.weak("unknown");
        }
    });
    ui.end_row();
    ui.label("Tip")
        .on_hover_text("The diameter of the flat tip; not set is taken as pointed");
    ui.horizontal(|ui| {
        let mut set = tool.tip_diameter.is_some();
        if ui.checkbox(&mut set, "").changed() {
            tool.tip_diameter = set.then_some(0.0);
        }
        match &mut tool.tip_diameter {
            Some(tip) => {
                ui.add(mm(tip));
            }
            None => {
                ui.weak("pointed");
            }
        }
    });
    ui.end_row();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summaries_are_short() {
        assert_eq!(len(12.0), "12");
        assert_eq!(len(0.25), "0.25");
        assert_eq!(len(12.3456), "12.346");
        let ramp = Entry::Ramp {
            ramp_angle: 3f64.to_radians(),
        };
        assert_eq!(entry_summary(&ramp, None), "ramp 3.0°");
        let helix = Entry::Helix {
            radius: 1.5,
            ramp_angle: 3f64.to_radians(),
        };
        let lead = Lead { radius: 2.0 };
        assert_eq!(
            entry_summary(&helix, Some(&lead)),
            "helix r 1.5 mm, 3.0°, lead 2 mm"
        );
    }
}
