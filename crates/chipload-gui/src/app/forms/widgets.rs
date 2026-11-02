// SPDX-License-Identifier: GPL-3.0-or-later
//! The forms' shared pieces: number fields, widths, collapsing sections, position and
//! rotation cells, reset buttons, and how lengths and feeds are written.

use super::*;

pub(in crate::app) fn mm(value: &mut f64) -> crate::units::Field<'_> {
    crate::units::length(value)
        .speed(0.1)
        .range(0.0..=100_000.0)
}

pub(in crate::app) fn feed(value: &mut f64) -> crate::units::Field<'_> {
    crate::units::feed(value).speed(10.0).range(0.0..=100_000.0)
}

pub(in crate::app) fn direction_combo(
    ui: &mut egui::Ui,
    id: (&str, usize),
    direction: &mut Direction,
) {
    ComboBox::from_id_salt(id)
        .selected_text(direction_name(ui, *direction))
        .show_ui(ui, |ui| {
            for choice in [Direction::Climb, Direction::Conventional] {
                ui.selectable_value(direction, choice, direction_name(ui, choice));
            }
        });
}

/// Least width of a form's label column, so the sections' grids line up; a narrow panel
/// gives it less, down to [`NARROW_WIDTH`].
pub(super) const LABEL_WIDTH: f32 = 130.0;

/// The least a label column or a field shrinks to in a narrow panel.
pub(super) const NARROW_WIDTH: f32 = 80.0;

/// Where the panel's width is kept for [`label_width`] (`set_form_width`).
pub(super) const FORM_WIDTH_ID: &str = "form-width";

/// The width the Properties forms get this frame: their label columns follow it, so the
/// panel can be made narrow (user report 2026-10-17) and every grid still lines up.
pub(in crate::app) fn set_form_width(ui: &egui::Ui) {
    let width = ui.available_width();
    ui.data_mut(|d| d.insert_temp(egui::Id::new(FORM_WIDTH_ID), width));
}

/// The forms' width this frame (`set_form_width`); unbounded outside the panel.
pub(super) fn form_width(ui: &egui::Ui) -> f32 {
    ui.data(|d| d.get_temp::<f32>(egui::Id::new(FORM_WIDTH_ID)))
        .unwrap_or(f32::INFINITY)
}

/// A form's label column: [`LABEL_WIDTH`], or 40 % of a narrower panel.
pub(super) fn label_width(ui: &egui::Ui) -> f32 {
    (0.4 * form_width(ui)).clamp(NARROW_WIDTH, LABEL_WIDTH)
}

/// A field's width: `wanted`, or what the label column (with a section's indent) leaves
/// of a narrow panel. Worked out from the panel, as a grid's cell doesn't know its own.
pub(in crate::app) fn field_width(ui: &egui::Ui, wanted: f32) -> f32 {
    let spacing = ui.spacing();
    let left = form_width(ui) - label_width(ui) - spacing.indent - 2.0 * spacing.item_spacing.x;
    wanted.min(left).max(NARROW_WIDTH)
}

/// A collapsible section of a form: a bold title with a short summary beside it (seen when
/// folded), and its rows in a two-column grid. Its open state is kept per `key`.
pub(in crate::app) fn section<R>(
    ui: &mut egui::Ui,
    key: &str,
    title: &str,
    summary: &str,
    rows: impl FnOnce(&mut egui::Ui) -> R,
) -> Option<R> {
    let marked = problems::section_has_problem(ui, key);
    section_marked(ui, key, title, summary, marked, rows)
}

/// How a section opens and how wide its label column is.
#[derive(Clone, Copy)]
pub(super) struct SectionLook {
    /// Open the first time it is shown.
    pub(super) open: bool,
    /// The least width of the label column.
    pub(super) labels: f32,
}

/// A [`section`] that opens as `look` says, with its label column at least `look.labels`
/// wide: sections that share one line up their fields.
pub(super) fn section_look<R>(
    ui: &mut egui::Ui,
    key: &str,
    title: &str,
    summary: &str,
    look: SectionLook,
    rows: impl FnOnce(&mut egui::Ui) -> R,
) -> Option<R> {
    let marked = problems::section_has_problem(ui, key);
    section_shown(ui, key, title, summary, marked, look, rows)
}

/// A section whose header has a red dot when `marked`: a field in it has a problem
/// (docs/specs/live-problems.md §3).
pub(super) fn section_marked<R>(
    ui: &mut egui::Ui,
    key: &str,
    title: &str,
    summary: &str,
    marked: bool,
    rows: impl FnOnce(&mut egui::Ui) -> R,
) -> Option<R> {
    let look = SectionLook {
        open: true,
        labels: label_width(ui),
    };
    section_shown(ui, key, title, summary, marked, look, rows)
}

pub(super) fn section_shown<R>(
    ui: &mut egui::Ui,
    key: &str,
    title: &str,
    summary: &str,
    marked: bool,
    look: SectionLook,
    rows: impl FnOnce(&mut egui::Ui) -> R,
) -> Option<R> {
    let mut header = egui::text::LayoutJob::default();
    let style = ui.style().clone();
    if marked {
        RichText::new("● ")
            .color(ui.visuals().error_fg_color)
            .append_to(
                &mut header,
                &style,
                egui::FontSelection::Default,
                Align::Center,
            );
    }
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
    // A long summary ends in "…" in a narrow panel (past the arrow and a section's indent).
    // Laid out here: the header lays text out again unwrapped, all but a finished galley.
    header.wrap = egui::text::TextWrapping {
        max_width: (form_width(ui) - 2.0 * ui.spacing().indent).max(NARROW_WIDTH),
        max_rows: 1,
        break_anywhere: true,
        overflow_character: Some('…'),
    };
    let header = ui.fonts_mut(|f| f.layout_job(header));
    egui::CollapsingHeader::new(header)
        .id_salt(("section", key))
        .default_open(look.open)
        .show(ui, |ui| {
            Grid::new(("section-grid", key))
                .num_columns(2)
                .min_col_width(look.labels)
                .show(ui, rows)
                .inner
        })
        .body_returned
}

/// A grid's value cell whose row wraps within the panel: a field with a button or a note
/// beside it, which goes under it in a narrow panel.
pub(super) fn wrapped_cell<R>(
    ui: &mut egui::Ui,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::InnerResponse<R> {
    let width = field_width(ui, f32::INFINITY);
    ui.scope(|ui| {
        ui.set_max_width(width);
        ui.horizontal_wrapped(add_contents).inner
    })
}

/// Degrees for a summary: one decimal, no trailing zero.
pub(super) fn degrees(radians: f64) -> String {
    let text = format!("{:.1}", radians.to_degrees());
    text.trim_end_matches('0').trim_end_matches('.').to_string()
}

pub(super) fn direction_name(ui: &egui::Ui, direction: Direction) -> String {
    match direction {
        Direction::Climb => t!(ui, "forms-climb"),
        Direction::Conventional => t!(ui, "forms-conventional"),
    }
}

/// A number field whose value is sent once the edit is finished (Enter, Tab, a click
/// elsewhere or a drag released): the new value, if it differs from `current`. While it is
/// edited, the typed value is kept between frames.
pub(super) fn finished_number(
    ui: &mut egui::Ui,
    id: egui::Id,
    current: f64,
    field: impl FnOnce(&mut egui::Ui, &mut f64) -> egui::Response,
) -> Option<f64> {
    let mut value = ui.data(|d| d.get_temp::<f64>(id)).unwrap_or(current);
    let response = field(ui, &mut value);
    if response.has_focus() || response.dragged() {
        ui.data_mut(|d| d.insert_temp(id, value));
        return None;
    }
    ui.data_mut(|d| d.remove::<f64>(id));
    let done = response.lost_focus() || response.drag_stopped() || response.changed();
    (done && value != current).then_some(value)
}

/// A position field: half a millimetre per pixel dragged.
pub(super) fn mm_field(value: &mut f64) -> crate::units::Field<'_> {
    crate::units::length(value).speed(0.5)
}

/// A length (or feed) in mm as a number in `units`, for reading.
pub(super) fn len_in(units: Units, mm: f64) -> String {
    chipload_pipeline::units::length_number(mm, units)
}

/// A feed in mm/min as a number in `units` per minute, for reading.
pub(super) fn feed_in(units: Units, mm_per_min: f64) -> String {
    chipload_pipeline::units::feed_number(mm_per_min, units)
}

/// A small reset button after a row; true when clicked.
/// X and Y of what is shown at `shown` (a box's corner, or a text's centre) and a reset
/// button that moves it to 0, 0; the drafted position `at` moves as much as the shown point.
/// Returns whether an edit finished.
pub(super) fn position_cells(
    ui: &mut egui::Ui,
    shown: (f64, f64),
    at: (&mut f64, &mut f64),
    reset_hover: &str,
) -> bool {
    let (x0, y0) = shown;
    let (x, y) = at;
    let mut finished = false;
    ui.horizontal(|ui| {
        let (mut cx, mut cy) = (x0, y0);
        for (value, axis) in [(&mut cx, "X "), (&mut cy, "Y ")] {
            let response = ui.add(mm_field(value).prefix(axis));
            finished |= response.drag_stopped() || response.lost_focus();
        }
        *x += cx - x0;
        *y += cy - y0;
        if reset_button(ui, (x0, y0) != (0.0, 0.0), reset_hover) {
            *x -= x0;
            *y -= y0;
            finished = true;
        }
    });
    finished
}

/// A drafted rotation, degrees counter-clockwise: the number, quarter turns either way and a
/// reset button; kept in [0, 360). Returns whether an edit finished.
pub(super) fn rotation_cells(ui: &mut egui::Ui, rotation: &mut f64, reset_hover: &str) -> bool {
    let mut finished = false;
    ui.horizontal(|ui| {
        let response = ui.add(
            crate::units::number(ui, rotation)
                .speed(1.0)
                .suffix("°")
                .max_decimals(3),
        );
        finished |= response.drag_stopped() || response.lost_focus();
        if let Some(by) = quarter_turn_buttons(ui) {
            *rotation += by;
            finished = true;
        }
        if reset_button(ui, *rotation != 0.0, reset_hover) {
            *rotation = 0.0;
            finished = true;
        }
        *rotation = rotation.rem_euclid(360.0);
        if *rotation >= 360.0 {
            *rotation = 0.0;
        }
    });
    finished
}

/// ⟲ 90 and ⟳ 90: the quarter turn clicked, degrees counter-clockwise.
pub(super) fn quarter_turn_buttons(ui: &mut egui::Ui) -> Option<f64> {
    let ccw = ui
        .small_button("⟲ 90")
        .on_hover_text(t!(ui, "forms-turn-ccw-hover"))
        .clicked();
    let cw = ui
        .small_button("⟳ 90")
        .on_hover_text(t!(ui, "forms-turn-cw-hover"))
        .clicked();
    match (ccw, cw) {
        (true, _) => Some(90.0),
        (_, true) => Some(-90.0),
        _ => None,
    }
}

pub(super) fn reset_button(ui: &mut egui::Ui, enabled: bool, hover: &str) -> bool {
    let label = t!(ui, "forms-reset");
    super::header::icon_button(ui, crate::icons::Icon::Undo, &label, enabled, false, 18.0)
        .on_hover_text(hover)
        .clicked()
}
