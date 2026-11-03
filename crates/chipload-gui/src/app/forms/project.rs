// SPDX-License-Identifier: GPL-3.0-or-later
//! The project's forms: the job, its stock and material, work zero, setups and parts.

use super::*;

/// Drawing, stock, machine and tool.
/// The G-code's X0 Y0: a choice, plus X and Y for a point. A point starts at `here`
/// (where the current zero is), so choosing it moves nothing.
pub(in crate::app) fn work_zero_rows(
    ui: &mut egui::Ui,
    salt: &str,
    work_zero: &mut WorkZero,
    here: (f64, f64),
) {
    let choices = [
        (
            WorkZero::DrawingOrigin {},
            t!(ui, "forms-zero-drawing-origin"),
        ),
        (
            WorkZero::Stock {
                corner: StockCorner::Center,
            },
            t!(ui, "forms-zero-stock-centre"),
        ),
        (
            WorkZero::Stock {
                corner: StockCorner::BottomLeft,
            },
            t!(ui, "forms-zero-stock-bottom-left"),
        ),
        (
            WorkZero::Stock {
                corner: StockCorner::BottomRight,
            },
            t!(ui, "forms-zero-stock-bottom-right"),
        ),
        (
            WorkZero::Stock {
                corner: StockCorner::TopLeft,
            },
            t!(ui, "forms-zero-stock-top-left"),
        ),
        (
            WorkZero::Stock {
                corner: StockCorner::TopRight,
            },
            t!(ui, "forms-zero-stock-top-right"),
        ),
    ];
    let current = *work_zero;
    let selected = choices
        .iter()
        .find(|(choice, _)| *choice == current)
        .map_or(t!(ui, "forms-zero-point"), |(_, name)| name.clone());
    ui.label(t!(ui, "forms-work-zero"))
        .on_hover_text(t!(ui, "forms-work-zero-hover"));
    ComboBox::from_id_salt((salt, "work_zero"))
        .selected_text(selected)
        .show_ui(ui, |ui| {
            for (choice, name) in choices {
                ui.selectable_value(work_zero, choice, name);
            }
            let (x, y) = here;
            let is_point = matches!(work_zero, WorkZero::Point { .. });
            if ui
                .selectable_label(is_point, t!(ui, "forms-zero-point"))
                .clicked()
                && !is_point
            {
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

/// Where X0 Y0 is, as the work zero list names it.
pub(in crate::app) fn work_zero_name(ui: &impl Lang, work_zero: &WorkZero) -> String {
    match work_zero {
        WorkZero::DrawingOrigin {} => t!(ui, "forms-zero-drawing-origin"),
        WorkZero::Stock { corner } => match corner {
            StockCorner::Center => t!(ui, "forms-zero-stock-centre"),
            StockCorner::BottomLeft => t!(ui, "forms-zero-stock-bottom-left"),
            StockCorner::BottomRight => t!(ui, "forms-zero-stock-bottom-right"),
            StockCorner::TopLeft => t!(ui, "forms-zero-stock-top-left"),
            StockCorner::TopRight => t!(ui, "forms-zero-stock-top-right"),
        },
        WorkZero::Point { .. } => t!(ui, "forms-zero-point"),
    }
}

/// Gap the *Set pitch* helper starts with, mm; only an input, never saved.
pub(super) const DEFAULT_COPIES_GAP: f64 = 10.0;

/// The job settings: the machine and its post, what every setup shares. A setup's board,
/// material and zero are its own (docs/specs/setups.md §7); the setup sheet is in
/// Preferences (docs/specs/this-computer.md §3).
pub(in crate::app) fn job_form(
    ui: &mut egui::Ui,
    edit: &mut EditJob,
    library: Option<&Library>,
) -> Option<JobRequest> {
    let mut request = None;
    let mut copies_request = None;
    let title = t!(ui, "forms-job-settings");
    super::browser::title_line(ui, crate::icons::Icon::Settings, None, &title);
    let labels = job_label_width(ui);
    let job = &mut edit.job;
    machine_section(
        ui,
        job,
        library,
        SectionLook { open: true, labels },
        &mut request,
        &mut copies_request,
    );
    copies_request.or(request.map(JobRequest::Library))
}

/// One label column for every Job settings section: as wide as the widest label, within
/// the panel's share for labels, so the fields line up down the panel.
pub(super) fn job_label_width(ui: &egui::Ui) -> f32 {
    let keys = [
        "forms-thickness",
        "forms-stock-size",
        "forms-work-zero",
        "forms-z-zero",
        "forms-material",
        "user-library-profile",
        "user-library-tool",
        "forms-from-profile",
        "forms-safe-z-job",
        "forms-max-overcut",
        "forms-post",
        "forms-gcode-units",
        "forms-spindle",
        "forms-max-feed",
        "forms-tool-changes",
        "forms-travel",
    ];
    let font = egui::TextStyle::Body.resolve(ui.style());
    let widest = keys
        .iter()
        .map(|key| {
            let text = crate::i18n::text(ui, key, &[]);
            ui.fonts_mut(|f| {
                f.layout_no_wrap(text, font.clone(), egui::Color32::WHITE)
                    .size()
                    .x
            })
        })
        .fold(0.0_f32, f32::max);
    let most = (0.45 * form_width(ui)).max(NARROW_WIDTH);
    widest.ceil().clamp(label_width(ui), most)
}

/// The stock: its thickness first, then its size, fitting the drawing or fixed
/// (docs/specs/stock-around-drawing.md).
/// `board`: the extents of the drawing the board is placed on, its setup's own shapes
/// ([`chipload_job::Job::board_extents`]).
/// Set in the egui memory to put the keyboard in the stock's thickness the next time it is
/// drawn: a new project's first field.
pub(in crate::app) const FOCUS_THICKNESS: &str = "focus-thickness";

pub(super) fn stock_section(
    ui: &mut egui::Ui,
    job: &mut chipload_job::Job,
    board: Option<((f64, f64), (f64, f64))>,
    look: SectionLook,
) {
    let units = crate::units::display(ui.ctx());
    let len_unit = chipload_pipeline::units::length_unit(units);
    let (size_x, size_y) = job.stock_size_around(board);
    let summary = format!(
        "{} × {} × {} {len_unit}",
        len_in(units, size_x),
        len_in(units, size_y),
        len_in(units, job.stock().thickness)
    );
    let drawing = board.map(|((x0, y0), (x1, y1))| {
        format!(
            "{} × {} {len_unit}",
            len_in(units, x1 - x0),
            len_in(units, y1 - y0)
        )
    });
    section_look(
        ui,
        "job-stock",
        &t!(ui, "forms-section-stock"),
        &summary,
        look,
        |ui| {
            ui.label(t!(ui, "forms-thickness"));
            let thickness = ui.add(mm(&mut job.stock_mut().thickness));
            // A new project: the thickness is typed first (the user, 2026-11-03).
            if ui.data_mut(|d| d.remove_temp::<bool>(egui::Id::new(FOCUS_THICKNESS))) == Some(true)
            {
                thickness.request_focus();
            }
            flag(thickness, ui, "stock.thickness");
            ui.end_row();
            // The board's own: a setup's (docs/specs/setups.md §7).
            ui.label(t!(ui, "forms-max-overcut"))
                .on_hover_text(t!(ui, "forms-max-overcut-hover"));
            flag(
                ui.add(mm(&mut job.stock_mut().max_overcut)),
                ui,
                "stock.max_overcut",
            );
            ui.end_row();
            // One choice of two: fit the drawing (the usual one, the user 2026-10-28), or
            // a fixed size for a board you want to use.
            let fits = job.stock().fit.is_some();
            ui.label(t!(ui, "forms-stock-size"));
            ui.horizontal_wrapped(|ui| {
                if ui
                    .radio(fits, t!(ui, "forms-stock-fit"))
                    .on_hover_text(t!(ui, "forms-stock-fit-hover"))
                    .clicked()
                    && !fits
                {
                    job.stock_mut().origin = StockOrigin::Drawing;
                    job.stock_mut().fit = Some(chipload_pipeline::DEFAULT_STOCK_MARGIN);
                }
                if let Some(size) = &drawing {
                    ui.weak(format!("({size})"))
                        .on_hover_text(t!(ui, "forms-drawing-size-hover"));
                }
            });
            ui.end_row();
            if let Some(mut margin) = job.stock().fit {
                ui.label("");
                ui.horizontal(|ui| {
                    ui.add_space(ui.spacing().indent);
                    ui.label(t!(ui, "forms-stock-margin"))
                        .on_hover_text(t!(ui, "forms-stock-fits-hover"));
                    if ui.add(mm(&mut margin)).changed() {
                        job.stock_mut().fit = Some(margin);
                    }
                });
                ui.end_row();
            }
            ui.label("");
            if ui
                .radio(!fits, t!(ui, "forms-stock-fixed-size"))
                .on_hover_text(t!(ui, "forms-stock-fixed-size-hover"))
                .clicked()
                && fits
            {
                // Keep the size it had, now fixed, still centred on the outline.
                (job.stock_mut().x, job.stock_mut().y) = (size_x, size_y);
                job.stock_mut().fit = None;
                job.stock_mut().origin = StockOrigin::Drawing;
            }
            ui.end_row();
            if job.stock().fit.is_none() {
                ui.label("");
                ui.horizontal(|ui| {
                    ui.add_space(ui.spacing().indent);
                    flag(ui.add(mm(&mut job.stock_mut().x)), ui, "stock.x");
                    ui.label("×");
                    flag(ui.add(mm(&mut job.stock_mut().y)), ui, "stock.y");
                });
                ui.end_row();
                // A stock is centred on the outline (the user, 2026-10-28). An older
                // project's stock at the drawing's origin keeps its place until changed
                // here, so its stock and a work zero on it never move unseen.
                if job.stock().origin != StockOrigin::Drawing {
                    ui.label("");
                    ui.horizontal(|ui| {
                        ui.add_space(ui.spacing().indent);
                        stock_position_combo(ui, &mut job.stock_mut().origin);
                    });
                    ui.end_row();
                }
                if let Some(size) = &drawing {
                    ui.label("");
                    ui.horizontal(|ui| {
                        ui.add_space(ui.spacing().indent);
                        ui.weak(t!(ui, "forms-drawing-size-note", size = size.as_str()))
                            .on_hover_text(t!(ui, "forms-drawing-size-hover"));
                    });
                    ui.end_row();
                }
            }
        },
    );
}

/// Where an older project's fixed-size stock lies relative to the drawing (file:
/// stock.origin). Not the G-code zero: that's the Zero section (docs/specs/work-zero.md).
pub(super) fn stock_position_combo(ui: &mut egui::Ui, origin: &mut StockOrigin) {
    let name = |ui: &egui::Ui, origin: StockOrigin| match origin {
        StockOrigin::BottomLeft => t!(ui, "forms-stock-corner-at-origin"),
        StockOrigin::Center => t!(ui, "forms-stock-centred-on-origin"),
        StockOrigin::Drawing => t!(ui, "forms-stock-centred-on-drawing"),
    };
    ComboBox::from_id_salt("origin")
        .selected_text(name(ui, *origin))
        .show_ui(ui, |ui| {
            for choice in [
                StockOrigin::BottomLeft,
                StockOrigin::Center,
                StockOrigin::Drawing,
            ] {
                ui.selectable_value(origin, choice, name(ui, choice));
            }
        })
        .response
        .on_hover_text(t!(ui, "forms-stock-position"));
}

/// Where X0 Y0 and Z0 are, in a line: "X0 Y0 stock bottom left · Z0 stock top".
pub(in crate::app) fn zero_summary(ui: &egui::Ui, job: &chipload_job::Job) -> String {
    t!(
        ui,
        "forms-zero-summary",
        xy = work_zero_name(ui, &job.work_zero()),
        z = z_zero_name(ui, job.stock().z_zero)
    )
}

pub(super) fn z_zero_name(ui: &egui::Ui, z: ZZero) -> String {
    match z {
        ZZero::Top => t!(ui, "forms-z-zero-top"),
        ZZero::Bottom => t!(ui, "forms-z-zero-bottom"),
    }
}

/// The work zero's properties (docs/specs/work-zero-marker.md §4): the stock's corners and
/// centre to click, the drawing origin, where it is, and Z zero.
pub(in crate::app) fn work_zero_form(ui: &mut egui::Ui, edit: &mut EditJob) {
    let title = t!(ui, "browser-work-zero");
    super::browser::title_line(ui, crate::icons::Icon::WorkZero, None, &title);
    let (x, y) = edit.work_zero_xy();
    let ((x0, y0), (x1, y1)) = edit.job.stock_bounds();
    let job = &mut edit.job;
    let chosen = job.work_zero();
    let mut zero = chosen;
    Grid::new("work-zero-form")
        .num_columns(2)
        .min_col_width(label_width(ui))
        .show(ui, |ui| {
            ui.label(t!(ui, "forms-zero-on-stock"))
                .on_hover_text(t!(ui, "forms-zero-on-stock-hover"));
            if let Some(corner) = stock_corner_picker(ui, (x1 - x0, y1 - y0), chosen) {
                zero = WorkZero::Stock { corner };
            }
            ui.end_row();
            ui.label("");
            let origin = matches!(chosen, WorkZero::DrawingOrigin {});
            if ui
                .selectable_label(origin, t!(ui, "forms-zero-drawing-origin-button"))
                .on_hover_text(t!(ui, "forms-zero-drawing-origin-hover"))
                .clicked()
            {
                zero = WorkZero::DrawingOrigin {};
            }
            ui.end_row();
            ui.label(t!(ui, "forms-zero-at"))
                .on_hover_text(t!(ui, "forms-zero-at-hover"));
            let (mut px, mut py) = (x, y);
            let typed = ui.horizontal(|ui| {
                let a = flag(
                    ui.add(crate::units::length(&mut px).prefix("X ").speed(0.1)),
                    ui,
                    "work_zero",
                );
                let b = ui.add(crate::units::length(&mut py).prefix("Y ").speed(0.1));
                a.changed() || b.changed()
            });
            if typed.inner {
                zero = WorkZero::Point { x: px, y: py };
            }
            ui.end_row();
            ui.label("");
            ui.weak(work_zero_name(ui, &chosen));
            ui.end_row();
            ui.label(t!(ui, "forms-z-zero"));
            ComboBox::from_id_salt("z_zero")
                .selected_text(z_zero_name(ui, job.stock().z_zero))
                .show_ui(ui, |ui| {
                    for z in [ZZero::Top, ZZero::Bottom] {
                        ui.selectable_value(&mut job.stock_mut().z_zero, z, z_zero_name(ui, z));
                    }
                });
            ui.end_row();
        });
    ui.add_space(4.0);
    ui.add(egui::Label::new(egui::RichText::new(t!(ui, "forms-zero-note")).weak()).wrap());
    *job.work_zero_mut() = zero;
}

/// A small drawing of the stock, in its proportions, with a dot at each corner and the
/// centre to click; the chosen one filled. Returns the corner clicked.
pub(super) fn stock_corner_picker(
    ui: &mut egui::Ui,
    (w, h): (f64, f64),
    chosen: WorkZero,
) -> Option<StockCorner> {
    const LONG_SIDE: f32 = 96.0;
    const SHORT_SIDE_MIN: f32 = 36.0;
    const DOT: f32 = 6.0;
    let ratio = if w > 0.0 && h > 0.0 {
        (h / w) as f32
    } else {
        0.6
    };
    let (pw, ph) = if ratio <= 1.0 {
        (LONG_SIDE, (LONG_SIDE * ratio).max(SHORT_SIDE_MIN))
    } else {
        ((LONG_SIDE / ratio).max(SHORT_SIDE_MIN), LONG_SIDE)
    };
    let margin = DOT + 2.0;
    let size = egui::vec2(pw + 2.0 * margin, ph + 2.0 * margin);
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    let board = rect.shrink(margin);
    let visuals = ui.visuals().clone();
    let painter = ui.painter_at(rect);
    painter.rect_stroke(
        board,
        0.0,
        egui::Stroke::new(1.0, visuals.weak_text_color()),
        egui::StrokeKind::Middle,
    );
    let mut clicked = None;
    for corner in StockCorner::ALL {
        let at = match corner {
            StockCorner::BottomLeft => board.left_bottom(),
            StockCorner::BottomRight => board.right_bottom(),
            StockCorner::TopLeft => board.left_top(),
            StockCorner::TopRight => board.right_top(),
            StockCorner::Center => board.center(),
        };
        let name = work_zero_name(ui, &WorkZero::Stock { corner });
        let on = chosen == WorkZero::Stock { corner };
        let hit = egui::Rect::from_center_size(at, egui::vec2(2.0 * margin, 2.0 * margin));
        let response = ui
            .interact(
                hit,
                ui.id().with(("zero-corner", corner.name())),
                egui::Sense::click(),
            )
            .on_hover_text(&name);
        response.widget_info(|| {
            egui::WidgetInfo::selected(egui::WidgetType::RadioButton, true, on, &name)
        });
        let color = if on || response.hovered() {
            visuals.selection.bg_fill
        } else {
            visuals.widgets.inactive.bg_fill
        };
        painter.circle_filled(at, DOT, color);
        painter.circle_stroke(at, DOT, visuals.widgets.inactive.fg_stroke);
        if response.clicked() {
            clicked = Some(corner);
        }
    }
    clicked
}

/// The project's material: every operation and those added later take it
/// (docs/specs/materials.md §2). Each operation can choose its own under its tool.
pub(super) fn material_section(
    ui: &mut egui::Ui,
    job: &chipload_job::Job,
    library: &Library,
    look: SectionLook,
) -> Option<JobRequest> {
    let materials = &library.materials;
    let chosen = job.stock().material.clone();
    let known = chosen
        .as_ref()
        .and_then(|m| materials.find_uid(&m.library_uid));
    let shown = match (&chosen, known) {
        (None, _) => t!(ui, "forms-material-none"),
        (Some(_), Some(m)) => m.name.clone(),
        (Some(m), None) => t!(ui, "forms-material-not-in-library", name = m.name.as_str()),
    };
    let mut request = None;
    section_look(
        ui,
        "job-material",
        &t!(ui, "forms-material"),
        &shown,
        look,
        |ui| {
            ui.label(t!(ui, "forms-material"))
                .on_hover_text(t!(ui, "forms-material-hover"));
            ComboBox::from_id_salt("material")
                .selected_text(shown.clone())
                .width(field_width(ui, 160.0))
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_label(chosen.is_none(), t!(ui, "forms-material-none"))
                        .clicked()
                        && chosen.is_some()
                    {
                        request = Some(JobRequest::UseMaterial(None));
                    }
                    for m in &materials.materials {
                        let on = chosen.as_ref().is_some_and(|c| c.library_uid == m.uid);
                        if ui.selectable_label(on, &m.name).clicked() && !on {
                            request = Some(JobRequest::UseMaterial(Some(m.uid.clone())));
                        }
                    }
                });
            ui.end_row();
            ui.label("");
            // What the label column, wider than a plain form's, leaves of the panel.
            let spacing = ui.spacing();
            let width =
                form_width(ui) - look.labels - spacing.indent - 2.0 * spacing.item_spacing.x;
            ui.scope(|ui| {
                ui.set_max_width(width.max(NARROW_WIDTH));
                ui.add(
                    egui::Label::new(RichText::new(t!(ui, "forms-material-each-op")).weak()).wrap(),
                );
            });
            ui.end_row();
        },
    );
    request
}

/// A part's properties (docs/specs/parts.md §8): its name, on or off, whether it is the
/// active part, and its copies.
/// Setup `id` (docs/specs/setups.md §7): its board (stock and material), then its name,
/// on/off and parts; its work zero is the Browser item beneath it. They
/// edit the active setup, so another setup's form offers to make it active first.
pub(in crate::app) fn setup_form(
    ui: &mut egui::Ui,
    edit: &mut EditJob,
    id: u64,
    library: Option<&Library>,
) -> Option<JobRequest> {
    let mut request = None;
    let index = edit.job.setups.iter().position(|s| s.id == id)?;
    let active = edit.job.active_setup_id() == Some(id);
    let several = edit.job.setups.len() > 1;
    let parts: Vec<String> = edit
        .job
        .parts
        .iter()
        .filter(|p| p.setup == id)
        .map(|p| p.name.clone())
        .collect();
    let labels = job_label_width(ui);
    let open = SectionLook { open: true, labels };
    {
        let setup = &mut edit.job.setups[index];
        super::browser::title_line(
            ui,
            crate::icons::Icon::Rectangle,
            None,
            &format!("{} · {}", t!(ui, "forms-setup"), setup.name),
        );
    }
    if !active {
        setup_name_rows(
            ui,
            &mut edit.job.setups[index],
            index,
            id,
            several,
            &parts,
            labels,
        );
        ui.add_space(6.0);
        if ui.button(t!(ui, "forms-setup-make-active")).clicked() {
            request = Some(JobRequest::ActivateSetup(id));
        }
        return request;
    }
    // Taken before the stock is borrowed for its fields.
    let board = edit.job.board_extents();
    let job = &mut edit.job;
    // The board first: its thickness and material are what a new project needs before
    // anything else (the user, 2026-11-03).
    stock_section(ui, job, board, open);
    if let Some(library) = library
        && let Some(material) = material_section(ui, job, library, open)
    {
        request = Some(material);
    }
    ui.add_space(4.0);
    setup_name_rows(
        ui,
        &mut job.setups[index],
        index,
        id,
        several,
        &parts,
        labels,
    );
    // The zero is set on its own item under the setup in the Browser, which shows
    // where it is (the user, 2026-11-03: not twice).

    if several {
        ui.add_space(6.0);
        if ui.button(t!(ui, "forms-remove-setup")).clicked() {
            request = Some(JobRequest::RemoveSetup(id));
        }
    }
    request
}

/// A setup's name, whether it is cut (with several), and its parts.
fn setup_name_rows(
    ui: &mut egui::Ui,
    setup: &mut chipload_job::Setup,
    index: usize,
    id: u64,
    several: bool,
    parts: &[String],
    labels: f32,
) {
    let at = |field: &str| format!("setups.{index}.{field}");
    Grid::new(("setup-form", id))
        .num_columns(2)
        .min_col_width(labels + ui.spacing().indent)
        .show(ui, |ui| {
            ui.label(t!(ui, "forms-name"));
            let response =
                ui.add(TextEdit::singleline(&mut setup.name).desired_width(field_width(ui, 200.0)));
            flag(response, ui, &at("name"));
            ui.end_row();
            if several {
                ui.label(t!(ui, "forms-setup-cut"))
                    .on_hover_text(t!(ui, "forms-setup-cut-hover"));
                ui.checkbox(&mut setup.enabled, t!(ui, "forms-setup-cut-check"));
                ui.end_row();
            }
            ui.label(t!(ui, "forms-setup-parts"));
            ui.label(if parts.is_empty() {
                t!(ui, "forms-operations-none")
            } else {
                parts.join(", ")
            });
            ui.end_row();
        });
}

pub(in crate::app) fn part_form(
    ui: &mut egui::Ui,
    edit: &mut EditJob,
    id: u64,
) -> Option<JobRequest> {
    let mut request = None;
    let active = edit.job.active_part_id() == Some(id);
    let numbers: Vec<String> = (0..edit.job.operations.len())
        .filter(|i| edit.job.operations[*i].part() == id)
        .map(|i| (i + 1).to_string())
        .collect();
    let index = edit.job.parts.iter().position(|p| p.id == id)?;
    let several = edit.job.parts.len() > 1;
    let part = &mut edit.job.parts[index];
    super::browser::title_line(
        ui,
        crate::icons::Icon::Part,
        None,
        &format!(
            "{} · {}",
            t!(ui, "forms-part-number", number = index + 1),
            part.name
        ),
    );
    let at = |field: &str| format!("parts.{index}.{field}");
    Grid::new(("part-form", id))
        .num_columns(2)
        .min_col_width(label_width(ui) + ui.spacing().indent)
        .show(ui, |ui| {
            ui.label(t!(ui, "forms-name"));
            let response =
                ui.add(TextEdit::singleline(&mut part.name).desired_width(field_width(ui, 200.0)));
            flag(response, ui, &at("name"));
            ui.end_row();
            ui.label(t!(ui, "forms-part-cut"))
                .on_hover_text(t!(ui, "forms-part-cut-hover"));
            ui.checkbox(&mut part.enabled, t!(ui, "forms-part-cut-check"));
            ui.end_row();
            ui.label(t!(ui, "forms-active"))
                .on_hover_text(t!(ui, "forms-active-hover"));
            if active {
                ui.label(t!(ui, "forms-active-yes"));
            } else if ui.button(t!(ui, "forms-activate")).clicked() {
                request = Some(JobRequest::Activate(id));
            }
            ui.end_row();
            ui.label(t!(ui, "forms-operations"));
            ui.label(if numbers.is_empty() {
                t!(ui, "forms-operations-none")
            } else {
                numbers.join(", ")
            });
            ui.end_row();
        });
    let copies = part.copies.map_or(t!(ui, "forms-copies-one"), |c| {
        format!("{} × {} = {}", c.columns, c.rows, c.count())
    });
    section(ui, "part-copies", &t!(ui, "forms-copies"), &copies, |ui| {
        copies_rows(ui, &mut part.copies, &mut request, &at("copies"), id);
    });
    if several {
        ui.add_space(4.0);
        ui.weak(t!(ui, "forms-parts-note"));
    }
    request
}

/// A part cut several times in a grid (docs/specs/copies-grid.md); `at` is the copies'
/// path, for the problems at its fields.
pub(super) fn copies_rows(
    ui: &mut egui::Ui,
    copies: &mut Option<Copies>,
    request: &mut Option<JobRequest>,
    at: &str,
    part: u64,
) {
    ui.label(t!(ui, "forms-copies"))
        .on_hover_text(t!(ui, "forms-copies-hover"));
    let mut on = copies.is_some();
    if ui.checkbox(&mut on, t!(ui, "forms-copies-grid")).changed() {
        *copies = on.then_some(Copies {
            columns: 2,
            rows: 1,
            dx: 0.0,
            dy: 0.0,
            order: CopyOrder::ByOperation,
        });
        if on {
            *request = Some(JobRequest::CopiesGap(part, DEFAULT_COPIES_GAP));
        }
    }
    ui.end_row();
    let Some(grid) = copies else {
        return;
    };
    ui.label(format!("  {}", t!(ui, "forms-columns-rows")));
    ui.horizontal(|ui| {
        let count = 1..=MAX_COPIES_PER_SIDE;
        flag(
            ui.add(crate::units::number(ui, &mut grid.columns).range(count.clone())),
            ui,
            &format!("{at}.columns"),
        );
        flag(
            ui.add(crate::units::number(ui, &mut grid.rows).range(count)),
            ui,
            &format!("{at}.rows"),
        );
        ui.label(format!("= {}", grid.count()));
    });
    ui.end_row();
    ui.label(format!("  {}", t!(ui, "forms-pitch")))
        .on_hover_text(t!(
            ui,
            "units-text-pitch-hover",
            unit = crate::units::unit(ui.ctx())
        ));
    ui.horizontal(|ui| {
        for (name, value) in [("dx", &mut grid.dx), ("dy", &mut grid.dy)] {
            let response = ui.add(
                crate::units::length(value)
                    .speed(0.1)
                    .range(-100_000.0..=100_000.0),
            );
            flag(response, ui, &format!("{at}.{name}"));
        }
    });
    ui.end_row();
    ui.label(format!("  {}", t!(ui, "forms-gap")))
        .on_hover_text(t!(ui, "forms-gap-hover"));
    ui.horizontal(|ui| {
        let id = egui::Id::new(("copies-gap", part));
        let mut gap = ui.data_mut(|d| *d.get_temp_mut_or(id, DEFAULT_COPIES_GAP));
        ui.add(mm(&mut gap));
        ui.data_mut(|d| d.insert_temp(id, gap));
        if ui.button(t!(ui, "forms-set-pitch")).clicked() {
            *request = Some(JobRequest::CopiesGap(part, gap));
        }
    });
    ui.end_row();
    ui.label(format!("  {}", t!(ui, "forms-order")));
    ComboBox::from_id_salt("copies-order")
        .selected_text(order_name(ui, grid.order))
        .show_ui(ui, |ui| {
            for order in [CopyOrder::ByOperation, CopyOrder::ByCopy] {
                let name = order_name(ui, order);
                ui.selectable_value(&mut grid.order, order, name);
            }
        });
    ui.end_row();
}

/// How the copies of a part are cut, in the app's language.
pub(super) fn order_name(ui: &egui::Ui, order: CopyOrder) -> String {
    let units = crate::units::display(ui.ctx()).into();
    chipload_pipeline::copies::order_message(order).render(ui.lang(), units)
}
