// SPDX-License-Identifier: GPL-3.0-or-later
//! A setup's form: its name, a back side and its dowels, pieces of a puzzle of boards.

use super::*;

/// Setup `id` (docs/specs/setups.md §7): its name, on/off and parts. Its board and its work
/// zero are the items beneath it in the Browser (the user, 2026-11-03). Another setup's
/// form offers to make it active.
pub(in crate::app) fn setup_form(
    ui: &mut egui::Ui,
    edit: &mut EditJob,
    id: u64,
) -> Option<JobRequest> {
    let mut request = None;
    let index = edit.job.setups.iter().position(|s| s.id == id)?;
    let active = edit.job.active_setup_id() == Some(id);
    let several = edit.job.setups.len() > 1;
    let parts: Vec<String> = edit
        .job
        .parts
        .iter()
        .filter(|p| edit.job.part_on(p.id, id))
        .map(|p| p.name.clone())
        .collect();
    let labels = job_label_width(ui);
    let setup = &mut edit.job.setups[index];
    super::browser_row::title_line(
        ui,
        crate::icons::Icon::Rectangle,
        None,
        &format!("{} · {}", t!(ui, "forms-setup"), setup.name),
    );
    setup_name_rows(ui, setup, index, id, several, &parts, labels);
    if several && let Some(change) = board_rows(ui, &edit.job, id, labels) {
        request = Some(change);
    }
    pieces_section(ui, &mut edit.job, index);
    if !active {
        ui.add_space(6.0);
        if ui.button(t!(ui, "forms-setup-make-active")).clicked() {
            request = Some(JobRequest::ActivateSetup(id));
        }
    }
    if several {
        ui.add_space(6.0);
        if ui.button(t!(ui, "forms-remove-setup")).clicked() {
            request = Some(JobRequest::RemoveSetup(id));
        }
    }
    request
}

/// The boards a new setup in pieces starts with when the machine's travel isn't known, mm.
const PIECES_BOARD: (f64, f64) = (600.0, 400.0);

/// A setup cut in pieces (docs/specs/pieces.md §9): on or off, the boards, the grid, the
/// joint with its size, spacing and clearance, and with a grid given, each seam's shift.
fn pieces_section(ui: &mut egui::Ui, job: &mut chipload_job::Job, index: usize) {
    use chipload_job::{Joint, PieceBoard, PieceGrid, Pieces};
    let units = crate::units::display(ui.ctx());
    let travel = job.machine.travel.as_ref().map(|t| (t.x, t.y));
    let setup = &mut job.setups[index];
    let at = |field: &str| format!("setups.{index}.pieces.{field}");
    let joint_name = |ui: &egui::Ui, joint: Joint| match joint {
        Joint::Knobs => t!(ui, "forms-pieces-knobs"),
        Joint::Fingers => t!(ui, "forms-pieces-fingers"),
    };
    let summary = match &setup.pieces {
        Some(p) => format!(
            "{} {} {}",
            joint_name(ui, p.joint),
            len_in(units, p.size),
            chipload_pipeline::units::length_unit(units)
        ),
        None => t!(ui, "forms-pieces-off"),
    };
    ui.add_space(6.0);
    section(
        ui,
        &format!("setup-pieces-{index}"),
        &t!(ui, "forms-section-pieces"),
        &summary,
        |ui| {
            let mut on = setup.pieces.is_some();
            ui.label("");
            if ui
                .checkbox(&mut on, t!(ui, "forms-pieces-on"))
                .on_hover_text(t!(ui, "forms-pieces-on-hover"))
                .changed()
            {
                setup.pieces = on.then(|| {
                    let (x, y) = travel.unwrap_or(PIECES_BOARD);
                    Pieces::new(PieceBoard { x, y }, Joint::Knobs)
                });
            }
            ui.end_row();
            let Some(p) = setup.pieces.as_mut() else {
                return;
            };
            ui.label(t!(ui, "forms-pieces-board"))
                .on_hover_text(t!(ui, "forms-pieces-board-hover"));
            ui.horizontal(|ui| {
                flag(ui.add(mm(&mut p.board.x)), ui, &at("board.x"));
                ui.label("×");
                flag(ui.add(mm(&mut p.board.y)), ui, &at("board.y"));
            });
            ui.end_row();
            ui.label(t!(ui, "forms-pieces-grid"));
            ui.horizontal(|ui| {
                let mut auto = p.grid.is_none();
                if ui
                    .checkbox(&mut auto, t!(ui, "forms-pieces-grid-auto"))
                    .changed()
                {
                    p.grid = (!auto).then_some(PieceGrid {
                        columns: 2,
                        rows: 1,
                    });
                    p.shift_x.clear();
                    p.shift_y.clear();
                }
                if let Some(g) = &mut p.grid {
                    ui.add(egui::DragValue::new(&mut g.columns).range(1..=50));
                    ui.label("×");
                    ui.add(egui::DragValue::new(&mut g.rows).range(1..=50));
                }
            });
            ui.end_row();
            ui.label(t!(ui, "forms-pieces-joint"));
            ComboBox::from_id_salt(("setup-pieces-joint", index))
                .selected_text(joint_name(ui, p.joint))
                .show_ui(ui, |ui| {
                    for joint in [Joint::Knobs, Joint::Fingers] {
                        if ui
                            .selectable_label(p.joint == joint, joint_name(ui, joint))
                            .clicked()
                            && p.joint != joint
                        {
                            p.joint = joint;
                            (p.size, p.spacing) = joint.defaults();
                        }
                    }
                });
            ui.end_row();
            ui.label(t!(ui, "forms-pieces-size"))
                .on_hover_text(t!(ui, "forms-pieces-size-hover"));
            flag(ui.add(mm(&mut p.size)), ui, &at("size"));
            ui.end_row();
            ui.label(t!(ui, "forms-pieces-spacing"))
                .on_hover_text(t!(ui, "forms-pieces-spacing-hover"));
            flag(ui.add(mm(&mut p.spacing)), ui, &at("spacing"));
            ui.end_row();
            ui.label(t!(ui, "forms-pieces-clearance"))
                .on_hover_text(t!(ui, "forms-pieces-clearance-hover"));
            flag(ui.add(mm(&mut p.clearance)), ui, &at("clearance"));
            ui.end_row();
            // Each inner seam's shift, with a grid given: its seams are known.
            if let Some(g) = p.grid {
                for (axis, count, shifts) in [
                    ("x", g.columns, &mut p.shift_x),
                    ("y", g.rows, &mut p.shift_y),
                ] {
                    let seams = count.saturating_sub(1) as usize;
                    shifts.resize(seams.max(shifts.len()), 0.0);
                    shifts.truncate(seams);
                    for (k, shift) in shifts.iter_mut().enumerate() {
                        let label = if axis == "x" {
                            t!(ui, "forms-pieces-shift-x", seam = k + 1)
                        } else {
                            t!(ui, "forms-pieces-shift-y", seam = k + 1)
                        };
                        ui.label(label)
                            .on_hover_text(t!(ui, "forms-pieces-shift-hover"));
                        let field = crate::units::length(shift)
                            .speed(1.0)
                            .range(-100_000.0..=100_000.0);
                        flag(ui.add(field), ui, &at(&format!("shift_{axis}")));
                        ui.end_row();
                    }
                    while shifts.last() == Some(&0.0) {
                        shifts.pop();
                    }
                }
            }
        },
    );
}

/// A setup's board (docs/specs/two-sided.md §3): its own, or the back of another setup,
/// then how a back turns over and is put back, and its dowel holes.
fn board_rows(
    ui: &mut egui::Ui,
    job: &chipload_job::Job,
    id: u64,
    labels: f32,
) -> Option<JobRequest> {
    use chipload_job::{Register, Turn};
    use chipload_pipeline::setups::BoardChange;
    let setup = job.setup(id)?;
    let back = setup.back_of;
    // Fronts it may turn over: another setup, not a back, with no other back.
    let fronts: Vec<(u64, String)> = job
        .setups
        .iter()
        .filter(|s| {
            s.id != id
                && s.back_of.is_none()
                && job.back_of(s.id).is_none_or(|b| b.id == id)
                && job.back_of(id).is_none()
        })
        .map(|s| (s.id, s.name.clone()))
        .collect();
    let name_of = |front: u64| job.setup(front).map_or(String::new(), |s| s.name.clone());
    let mut request = None;
    ui.add_space(6.0);
    Grid::new(("setup-board", id))
        .num_columns(2)
        .min_col_width(labels + ui.spacing().indent)
        .show(ui, |ui| {
            ui.label(t!(ui, "forms-setup-board"))
                .on_hover_text(t!(ui, "forms-setup-board-hover"));
            let shown = match back {
                Some(b) => t!(
                    ui,
                    "forms-setup-board-back",
                    setup = name_of(b.setup).as_str()
                ),
                None => t!(ui, "forms-setup-board-own"),
            };
            ComboBox::from_id_salt(("setup-board-pick", id))
                .selected_text(shown)
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_label(back.is_none(), t!(ui, "forms-setup-board-own"))
                        .clicked()
                        && back.is_some()
                    {
                        let change = BoardChange {
                            back_of: Some(None),
                            ..BoardChange::default()
                        };
                        request = Some(JobRequest::SetBoard(id, change));
                    }
                    for (front, name) in &fronts {
                        let chosen = back.is_some_and(|b| b.setup == *front);
                        let label = t!(ui, "forms-setup-board-back", setup = name.as_str());
                        if ui.selectable_label(chosen, label).clicked() && !chosen {
                            let change = BoardChange {
                                back_of: Some(Some(*front)),
                                ..BoardChange::default()
                            };
                            request = Some(JobRequest::SetBoard(id, change));
                        }
                    }
                });
            ui.end_row();
            let Some(b) = back else {
                // A front: its back side and the dowel holes for it (two-sided-wizard.md §3).
                if let Some(asked) = back_side_row(ui, job, id) {
                    request = Some(asked);
                }
                return;
            };
            ui.label(t!(ui, "forms-setup-turn"))
                .on_hover_text(t!(ui, "forms-setup-turn-hover"));
            let turn_name = |ui: &egui::Ui, turn: Turn| match turn {
                Turn::LeftRight => t!(ui, "forms-setup-turn-left-right"),
                Turn::TopBottom => t!(ui, "forms-setup-turn-top-bottom"),
            };
            ComboBox::from_id_salt(("setup-turn", id))
                .selected_text(turn_name(ui, b.turn))
                .show_ui(ui, |ui| {
                    for turn in [Turn::LeftRight, Turn::TopBottom] {
                        if ui
                            .selectable_label(b.turn == turn, turn_name(ui, turn))
                            .clicked()
                            && b.turn != turn
                        {
                            let change = BoardChange {
                                turn: Some(turn),
                                ..BoardChange::default()
                            };
                            request = Some(JobRequest::SetBoard(id, change));
                        }
                    }
                });
            ui.end_row();
            ui.label(t!(ui, "forms-setup-register"))
                .on_hover_text(t!(ui, "forms-setup-register-hover"));
            let register_name = |ui: &egui::Ui, register: Register| match register {
                Register::Pins => t!(ui, "forms-setup-register-pins"),
                Register::Corner => t!(ui, "forms-setup-register-corner"),
            };
            ComboBox::from_id_salt(("setup-register", id))
                .selected_text(register_name(ui, b.register))
                .show_ui(ui, |ui| {
                    for register in [Register::Pins, Register::Corner] {
                        if ui
                            .selectable_label(b.register == register, register_name(ui, register))
                            .clicked()
                            && b.register != register
                        {
                            let change = BoardChange {
                                register: Some(register),
                                ..BoardChange::default()
                            };
                            request = Some(JobRequest::SetBoard(id, change));
                        }
                    }
                });
            ui.end_row();
            if b.register == Register::Pins {
                ui.label(t!(ui, "forms-setup-dowels"));
                if let Some(asked) = dowels_cell(ui, job, id, b.setup, true) {
                    request = Some(asked);
                }
                ui.end_row();
            }
        });
    request
}

/// A front setup's *Back side* row: *None* with *Add the back side*, or its back as a link
/// with the state of its dowel holes.
fn back_side_row(ui: &mut egui::Ui, job: &chipload_job::Job, front: u64) -> Option<JobRequest> {
    let mut request = None;
    ui.label(t!(ui, "forms-setup-back-side"))
        .on_hover_text(t!(ui, "forms-setup-back-side-hover"));
    match job.back_of(front) {
        None => {
            ui.horizontal(|ui| {
                ui.label(t!(ui, "forms-setup-back-none"));
                if ui
                    .button(t!(ui, "forms-setup-add-back"))
                    .on_hover_text(t!(ui, "forms-setup-add-back-hover"))
                    .clicked()
                {
                    request = Some(JobRequest::AddBack(front));
                }
            });
            ui.end_row();
        }
        Some(back) => {
            let mut name = back.name.clone();
            if job.missing_dowels(back.id) {
                name.push_str(" ⚠");
            }
            if ui.link(name).clicked() {
                request = Some(JobRequest::OpenSetup(back.id));
            }
            ui.end_row();
            if back
                .back_of
                .is_some_and(|b| b.register == chipload_job::Register::Pins)
            {
                ui.label(t!(ui, "forms-setup-dowels"));
                if let Some(asked) = dowels_cell(ui, job, back.id, front, false) {
                    request = Some(asked);
                }
                ui.end_row();
            }
        }
    }
    request
}

/// The dowel holes of back `back`, whose front is `front`: the operation as a link (with
/// *Remove* in the back's own form), or *None yet* in the warning colour with *Add dowel
/// holes* (docs/specs/two-sided-wizard.md §3).
fn dowels_cell(
    ui: &mut egui::Ui,
    job: &chipload_job::Job,
    back: u64,
    front: u64,
    with_remove: bool,
) -> Option<JobRequest> {
    let mut request = None;
    let front_name = job.setup(front).map_or(String::new(), |s| s.name.clone());
    ui.horizontal(|ui| match job.dowels_of(front) {
        Some(i) => {
            let mut text = t!(ui, "forms-setup-dowels-there", number = i + 1);
            if job.missing_dowels(back) {
                text.push_str(" ⚠");
            }
            if ui
                .link(text)
                .on_hover_text(t!(ui, "forms-setup-dowels-open-hover"))
                .clicked()
            {
                request = Some(JobRequest::OpenOperation(i));
            }
            if with_remove {
                if ui
                    .button(t!(ui, "forms-setup-place-dowels"))
                    .on_hover_text(t!(ui, "forms-setup-place-dowels-hover"))
                    .clicked()
                {
                    request = Some(JobRequest::PlaceDowels(back));
                }
                if ui
                    .button(t!(ui, "common-remove"))
                    .on_hover_text(t!(ui, "forms-setup-remove-dowels-hover"))
                    .clicked()
                {
                    request = Some(JobRequest::RemoveDowels(back));
                }
            }
        }
        None => {
            ui.colored_label(
                ui.visuals().warn_fg_color,
                t!(ui, "forms-setup-dowels-none"),
            )
            .on_hover_text(t!(
                ui,
                "forms-setup-dowels-none-hover",
                front = front_name.as_str()
            ));
            if ui
                .button(t!(ui, "forms-setup-add-dowels"))
                .on_hover_text(t!(
                    ui,
                    "forms-setup-add-dowels-hover",
                    front = front_name.as_str()
                ))
                .clicked()
            {
                request = Some(JobRequest::AddDowels(back));
            }
        }
    });
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
    crate::app::forms::card(ui, |ui| {
        Grid::new(("setup-form", id))
            .num_columns(2)
            .min_col_width(labels + ui.spacing().indent)
            .show(ui, |ui| {
                ui.label(t!(ui, "forms-name"));
                let response = ui.add(
                    TextEdit::singleline(&mut setup.name).desired_width(field_width(ui, 200.0)),
                );
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
            })
    });
}
