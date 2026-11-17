// SPDX-License-Identifier: GPL-3.0-or-later
//! A part's and an instance's forms, and a grid of copies.

use super::*;

pub(in crate::app) fn part_form(
    ui: &mut egui::Ui,
    edit: &mut EditJob,
    id: u64,
    library: Option<&Library>,
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
    // Its instances by board, once there are several (docs/specs/parts-and-setups.md §8).
    let boards: Vec<(String, Vec<u32>)> = if edit.job.setups.len() > 1 {
        edit.job
            .setups
            .iter()
            .map(|s| {
                let numbers = s
                    .instances
                    .iter()
                    .filter(|i| i.part == id)
                    .map(|i| i.id)
                    .collect();
                (s.name.clone(), numbers)
            })
            .collect()
    } else {
        Vec::new()
    };
    // The board it is cut from, for a thickness newly ticked.
    let board = edit
        .job
        .home_setup(id)
        .and_then(|s| edit.job.setup(s))
        .map_or(0.0, |s| s.stock.thickness);
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
    if !boards.is_empty() {
        let on: Vec<&(String, Vec<u32>)> = boards.iter().filter(|(_, n)| !n.is_empty()).collect();
        let summary = match on.as_slice() {
            [] => t!(ui, "forms-part-boards-none"),
            [(name, _)] => name.clone(),
            more => t!(ui, "forms-part-boards-count", count = more.len()),
        };
        section(
            ui,
            "part-boards",
            &t!(ui, "forms-part-boards"),
            &summary,
            |ui| {
                for (name, numbers) in &boards {
                    ui.label(name);
                    let text: Vec<String> = numbers.iter().map(|n| format!("#{n}")).collect();
                    if text.is_empty() {
                        ui.weak("–");
                    } else {
                        ui.label(text.join(", "));
                    }
                    ui.end_row();
                }
            },
        );
    }
    let materials = library.map(|l| &l.materials);
    if let Some(asked) = stock_section(ui, id, part.stock.as_ref(), board, materials) {
        request = Some(asked);
    }
    // On a board of another stock: the move offered, never made unasked (§4).
    if let Some(asked) = stock_move_offer(ui, &edit.job, id) {
        request = Some(asked);
    }
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

/// The stock part `id` is cut from (docs/specs/parts-and-setups.md §4), folded, "Any
/// board" until set: a library material and a thickness, each "any" when not chosen. A
/// change is recorded, nothing moves; a thickness being dragged waits until it is let go.
fn stock_section(
    ui: &mut egui::Ui,
    id: u64,
    current: Option<&chipload_job::PartStock>,
    board: f64,
    materials: Option<&chipload_job::materials::MaterialLibrary>,
) -> Option<JobRequest> {
    let units = crate::units::display(ui.ctx()).into();
    let summary = current.map_or_else(
        || t!(ui, "forms-part-stock-any"),
        |s| chipload_pipeline::part_stock::stock_message(s).render(ui.lang(), units),
    );
    let mut wanted = current.cloned().unwrap_or_default();
    let mut changed = false;
    section(
        ui,
        "part-stock",
        &t!(ui, "forms-part-stock"),
        &summary,
        |ui| {
            ui.label(t!(ui, "forms-material"))
                .on_hover_text(t!(ui, "forms-part-stock-hover"));
            let shown = wanted.material.as_ref().map_or_else(
                || t!(ui, "forms-part-stock-any-material"),
                |m| m.name.clone(),
            );
            ComboBox::from_id_salt(("part-material", id))
                .selected_text(shown)
                .width(field_width(ui, 160.0))
                .show_ui(ui, |ui| {
                    let any = t!(ui, "forms-part-stock-any-material");
                    if ui
                        .selectable_label(wanted.material.is_none(), any)
                        .clicked()
                        && wanted.material.is_some()
                    {
                        wanted.material = None;
                        changed = true;
                    }
                    for m in materials.map_or(&[][..], |l| l.materials.as_slice()) {
                        let on = wanted
                            .material
                            .as_ref()
                            .is_some_and(|c| c.library_uid == m.uid);
                        if ui.selectable_label(on, &m.name).clicked() && !on {
                            wanted.material = Some(chipload_pipeline::materials::material_ref(m));
                            changed = true;
                        }
                    }
                });
            ui.end_row();
            ui.label(t!(ui, "forms-thickness"))
                .on_hover_text(t!(ui, "forms-part-stock-thickness-hover"));
            ui.horizontal(|ui| {
                let mut ticked = wanted.thickness.is_some();
                if ui
                    .checkbox(&mut ticked, "")
                    .on_hover_text(t!(ui, "forms-part-stock-thickness-hover"))
                    .changed()
                {
                    wanted.thickness = ticked.then_some(board);
                    changed = true;
                }
                if let Some(thickness) = wanted.thickness.as_mut() {
                    let response = ui.add(crate::units::length(thickness).range(0.01..=10_000.0));
                    if response.drag_stopped() || (response.changed() && !response.dragged()) {
                        changed = true;
                    }
                }
            });
            ui.end_row();
        },
    );
    changed.then(|| JobRequest::PartStock(id, (!wanted.is_empty()).then_some(wanted)))
}

/// When part `id` is on a board of another stock than its own: a note and the button that
/// moves it to a setup of its stock, or to a new one (docs/specs/parts-and-setups.md §4).
fn stock_move_offer(ui: &mut egui::Ui, job: &chipload_job::Job, id: u64) -> Option<JobRequest> {
    use chipload_pipeline::part_stock::{StockBoard, board_message, stock_board, stock_message};
    let target = stock_board(job, id)?;
    let wanted = job.part(id)?.stock.as_ref()?;
    let units = crate::units::display(ui.ctx()).into();
    let label = match target {
        StockBoard::Existing(setup) => {
            let board = job.setup(setup)?;
            t!(
                ui,
                "forms-part-stock-move-to",
                setup = board.name.as_str(),
                stock = board_message(&board.stock).render(ui.lang(), units)
            )
        }
        StockBoard::New => t!(
            ui,
            "forms-part-stock-move-new",
            stock = stock_message(wanted).render(ui.lang(), units)
        ),
    };
    ui.add_space(2.0);
    ui.label(
        egui::RichText::new(t!(ui, "forms-part-stock-elsewhere")).color(ui.visuals().warn_fg_color),
    );
    ui.button(label)
        .on_hover_text(t!(ui, "forms-part-stock-move-hover"))
        .clicked()
        .then_some(JobRequest::MoveToStockBoard(id))
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
    // The value edited, and where its problems are reported: in its setup
    // (docs/specs/parts-and-setups.md §5).
    let (j, k) = edit.job.setups.iter().enumerate().find_map(|(j, s)| {
        s.instances
            .iter()
            .position(|i| i.part == part && i.id == id)
            .map(|k| (j, k))
    })?;
    let mut instance: Instance = edit.job.setups[j].instances[k];
    // Its board, named once there are several (docs/specs/parts-and-setups.md §8).
    let board = (edit.job.setups.len() > 1).then(|| edit.job.setups[j].name.clone());
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
                if let Some(board) = &board {
                    ui.label(t!(ui, "forms-instance-setup"));
                    ui.label(board);
                    ui.end_row();
                }
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
    ui.weak(t!(ui, "forms-instance-note"));
    *target = instance;
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
