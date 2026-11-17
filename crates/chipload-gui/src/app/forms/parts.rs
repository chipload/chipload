// SPDX-License-Identifier: GPL-3.0-or-later
//! A part's and an instance's forms, and a grid of copies.

use super::*;

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
    // Its instances, on every board (docs/specs/parts-and-setups.md §2).
    let count = edit.job.count_all(id);
    let has_instances = edit.job.instances_of(id).len() > 1;
    let mut clear = false;
    let part = &mut edit.job.parts[index];
    super::browser_row::title_line(
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
    crate::app::forms::card(ui, |ui| {
        Grid::new(("part-form", id))
            .num_columns(2)
            .min_col_width(label_width(ui) + ui.spacing().indent)
            .show(ui, |ui| {
                ui.label(t!(ui, "forms-name"));
                let response = ui.add(
                    TextEdit::singleline(&mut part.name).desired_width(field_width(ui, 200.0)),
                );
                flag(response, ui, &at("name"));
                ui.end_row();
                ui.label(t!(ui, "forms-part-cut"))
                    .on_hover_text(t!(ui, "forms-part-cut-hover"));
                ui.checkbox(&mut part.enabled, t!(ui, "forms-part-cut-check"));
                ui.end_row();
                mirror_row(ui, &mut part.mirror, id);
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
            })
    });
    let copies = if has_instances {
        count.to_string()
    } else {
        t!(ui, "forms-copies-one")
    };
    section(ui, "part-copies", &t!(ui, "forms-copies"), &copies, |ui| {
        let instances = (count, has_instances, &mut clear);
        copies_rows(ui, part, instances, &mut request, id);
    });
    if clear {
        // Cut once: its first instance on its board stays.
        let active = edit.job.active_setup;
        edit.job.active_setup = edit.job.home_setup(id).or(active);
        let first = edit.job.instances_here(id).first().map(|i| **i);
        edit.job.set_instances_here(id, first.into_iter().collect());
        edit.job.active_setup = active;
    }
    if several {
        ui.add_space(4.0);
        ui.weak(t!(ui, "forms-parts-note"));
    }
    request
}

/// A part cut mirrored about its box's centre line, or as drawn
/// (docs/specs/mirror-part.md §5).
fn mirror_row(ui: &mut egui::Ui, mirror: &mut Option<chipload_job::Mirror>, part: u64) {
    use chipload_job::Mirror;
    let name = |ui: &egui::Ui, mirror: Option<Mirror>| match mirror {
        None => t!(ui, "forms-part-mirror-none"),
        Some(Mirror::LeftRight) => t!(ui, "forms-setup-turn-left-right"),
        Some(Mirror::TopBottom) => t!(ui, "forms-setup-turn-top-bottom"),
    };
    ui.label(t!(ui, "forms-part-mirror"))
        .on_hover_text(t!(ui, "forms-part-mirror-hover"));
    ComboBox::from_id_salt(("part-mirror", part))
        .selected_text(name(ui, *mirror))
        .show_ui(ui, |ui| {
            for choice in [None, Some(Mirror::LeftRight), Some(Mirror::TopBottom)] {
                ui.selectable_value(mirror, choice, name(ui, choice));
            }
        });
    ui.end_row();
}

/// Part `part`'s instance `id` (docs/specs/instances.md §4): cut or not, its mirror, its
/// turn and its move, each a field. Instance 1 is the part itself
/// (docs/specs/part-placement.md §6): its turn and move only, the part's own fields.
pub(in crate::app) fn instance_form(
    ui: &mut egui::Ui,
    edit: &mut EditJob,
    part: u64,
    id: u32,
) -> Option<JobRequest> {
    use chipload_job::{Instance, Mirror};
    let own = edit.job.part(part)?;
    let name = own.name.clone();
    let enabled = own.enabled;
    // The value edited, and where its problems are reported: in its setup
    // (docs/specs/parts-and-setups.md §5).
    let (j, k) = edit.job.setups.iter().enumerate().find_map(|(j, s)| {
        s.instances
            .iter()
            .position(|i| i.part == part && i.id == id)
            .map(|k| (j, k))
    })?;
    let mut instance: Instance = edit.job.setups[j].instances[k];
    if id == 1 {
        // The part itself is on with its part.
        instance.enabled = enabled;
    }
    // Problems are keyed by field, without the setup's place.
    let base = format!("instances.{k}");
    super::browser_row::title_line(
        ui,
        crate::icons::Icon::Array,
        None,
        &format!("{} · {name}", t!(ui, "forms-instance-number", number = id)),
    );
    let at = |field: &str| format!("{base}.{field}");
    crate::app::forms::card(ui, |ui| {
        Grid::new(("instance-form", part, id))
            .num_columns(2)
            .min_col_width(label_width(ui) + ui.spacing().indent)
            .show(ui, |ui| {
                if id != 1 {
                    ui.label(t!(ui, "forms-part-cut"))
                        .on_hover_text(t!(ui, "forms-instance-cut-hover"));
                    ui.checkbox(&mut instance.enabled, t!(ui, "forms-instance-cut-check"));
                    ui.end_row();
                    let mirror_name = |ui: &egui::Ui, mirror: Option<Mirror>| match mirror {
                        None => t!(ui, "forms-part-mirror-none"),
                        Some(Mirror::LeftRight) => t!(ui, "forms-setup-turn-left-right"),
                        Some(Mirror::TopBottom) => t!(ui, "forms-setup-turn-top-bottom"),
                    };
                    ui.label(t!(ui, "forms-part-mirror"))
                        .on_hover_text(t!(ui, "forms-instance-mirror-hover"));
                    ComboBox::from_id_salt(("instance-mirror", part, id))
                        .selected_text(mirror_name(ui, instance.mirror))
                        .show_ui(ui, |ui| {
                            for choice in [None, Some(Mirror::LeftRight), Some(Mirror::TopBottom)] {
                                ui.selectable_value(
                                    &mut instance.mirror,
                                    choice,
                                    mirror_name(ui, choice),
                                );
                            }
                        });
                    ui.end_row();
                }
                ui.label(t!(ui, "forms-rotation"))
                    .on_hover_text(t!(ui, "forms-instance-angle-hover"));
                ui.horizontal(|ui| {
                    let current = instance.angle.to_degrees();
                    let salt = egui::Id::new(("instance-angle", part, id));
                    if let Some(degrees) = finished_number(ui, salt, current, |ui, v| {
                        ui.add(
                            crate::units::number(ui, v)
                                .speed(1.0)
                                .suffix("°")
                                .max_decimals(3),
                        )
                    }) {
                        instance.angle = degrees.to_radians();
                    }
                    if let Some(by) = quarter_turn_buttons(ui) {
                        let turned = (current + by + 180.0).rem_euclid(360.0) - 180.0;
                        instance.angle = turned.to_radians();
                    }
                });
                ui.end_row();
                for axis in [0, 1] {
                    let text = match axis {
                        0 => t!(ui, "forms-instance-move-x"),
                        _ => t!(ui, "forms-instance-move-y"),
                    };
                    ui.label(text)
                        .on_hover_text(t!(ui, "forms-instance-move-hover"));
                    let response = ui.add(mm_field(&mut instance.at[axis]));
                    flag(response, ui, &at(&format!("at.{axis}")));
                    ui.end_row();
                }
            })
    });
    ui.add_space(4.0);
    let target = &mut edit.job.setups[j].instances[k];
    if id == 1 {
        ui.weak(t!(ui, "forms-instance-one-note"));
        target.angle = instance.angle;
        target.at = instance.at;
    } else {
        ui.weak(t!(ui, "forms-instance-note"));
        *target = instance;
    }
    None
}

/// A part's instances (docs/specs/instances.md §4): how many, their cut order, and a grid
/// that makes them: columns × rows at a pitch, or at a gap between the cuts.
/// `instances`: how many times it is cut, whether it has instances besides one, and set
/// when they are to go.
pub(super) fn copies_rows(
    ui: &mut egui::Ui,
    part: &mut chipload_job::Part,
    instances: (u32, bool, &mut bool),
    request: &mut Option<JobRequest>,
    id: u64,
) {
    let (count, several, clear) = instances;
    ui.label(t!(ui, "forms-copies"))
        .on_hover_text(t!(ui, "forms-copies-hover"));
    ui.horizontal(|ui| {
        ui.label(count.to_string());
        if several && ui.button(t!(ui, "forms-instances-none")).clicked() {
            *clear = true;
        }
    });
    ui.end_row();
    if several {
        ui.label(format!("  {}", t!(ui, "forms-order")));
        ComboBox::from_id_salt(("copies-order", id))
            .selected_text(order_name(ui, part.order))
            .show_ui(ui, |ui| {
                for order in [InstanceOrder::ByOperation, InstanceOrder::ByInstance] {
                    let name = order_name(ui, order);
                    ui.selectable_value(&mut part.order, order, name);
                }
            });
        ui.end_row();
    }
    // The grid being set up, kept in the window's memory: it makes instances when asked.
    let key = egui::Id::new(("copies-grid", id));
    let mut grid = ui.data_mut(|d| {
        *d.get_temp_mut_or(
            key,
            Copies {
                columns: 2,
                rows: 1,
                dx: 0.0,
                dy: 0.0,
            },
        )
    });
    let count = 1..=MAX_COPIES_PER_SIDE;
    let columns = t!(ui, "forms-grid-columns");
    let rows = t!(ui, "forms-grid-rows");
    for (label, value) in [(columns, &mut grid.columns), (rows, &mut grid.rows)] {
        ui.label(format!("  {label}"));
        ui.add(crate::units::number(ui, value).range(count.clone()));
        ui.end_row();
    }
    let pitch_x = t!(ui, "forms-pitch-x");
    let pitch_y = t!(ui, "forms-pitch-y");
    let hover = t!(
        ui,
        "units-text-pitch-hover",
        unit = crate::units::unit(ui.ctx())
    );
    for (label, value) in [(pitch_x, &mut grid.dx), (pitch_y, &mut grid.dy)] {
        ui.label(format!("  {label}")).on_hover_text(&hover);
        ui.add(
            crate::units::length(value)
                .speed(0.1)
                .range(-100_000.0..=100_000.0),
        );
        ui.end_row();
    }
    ui.label("");
    if ui.button(t!(ui, "forms-instances-grid")).clicked() {
        *request = Some(JobRequest::CopiesGrid(id, grid, None));
    }
    ui.end_row();
    ui.label(format!("  {}", t!(ui, "forms-gap")))
        .on_hover_text(t!(ui, "forms-gap-hover"));
    let gap_key = egui::Id::new(("copies-gap", id));
    let mut gap = ui.data_mut(|d| *d.get_temp_mut_or(gap_key, DEFAULT_COPIES_GAP));
    ui.add(mm(&mut gap));
    ui.data_mut(|d| d.insert_temp(gap_key, gap));
    ui.end_row();
    ui.label("");
    if ui.button(t!(ui, "forms-instances-grid")).clicked() {
        *request = Some(JobRequest::CopiesGrid(id, grid, Some(gap)));
    }
    ui.end_row();
    ui.data_mut(|d| d.insert_temp(key, grid));
}

/// How the copies of a part are cut, in the app's language.
pub(super) fn order_name(ui: &egui::Ui, order: InstanceOrder) -> String {
    let units = crate::units::display(ui.ctx()).into();
    chipload_pipeline::copies::order_message(order).render(ui.lang(), units)
}
