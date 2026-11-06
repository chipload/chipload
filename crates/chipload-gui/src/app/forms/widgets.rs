// SPDX-License-Identifier: GPL-3.0-or-later
//! The forms' shared pieces: number fields, widths, collapsing sections, position and
//! rotation cells, reset buttons, and how lengths and feeds are written.

use super::*;
use eframe::egui::Color32;

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
    // Inside a card: its margins and an operation's stripe left out, for every form, so
    // label columns worked out before a card starts fit in it.
    let width = ui.available_width() - CARD_MARGINS;
    ui.data_mut(|d| {
        d.insert_temp(egui::Id::new(FORM_WIDTH_ID), width);
        // A new frame of the panel: no heading or stripe yet.
        d.remove::<String>(egui::Id::new(HEADING_ID));
        d.remove::<Color32>(egui::Id::new(STRIPE_ID));
    });
}

/// Where the Properties panel's heading is kept this frame, so a form's first section of
/// the same name doesn't say it again (docs/specs/look-and-feel.md decision 14).
pub(in crate::app) const HEADING_ID: &str = "form-heading";
/// Where an operation's colour is kept for the stripe down its cards (decision 15).
const STRIPE_ID: &str = "form-stripe";
/// A choice box's or text field's width in a card (decision 16), less in a narrow panel.
const FIELD_WIDTH: f32 = 110.0;
/// The least width of a number field in a card: wider for a longer number.
const NUMBER_WIDTH: f32 = 72.0;
/// The stripe down an operation's cards, points.
const STRIPE: f32 = 3.0;
/// A section's indent inside its card, points.
const CARD_INDENT: f32 = 6.0;
/// The room before a section's title, for its arrow: egui's indent.
const HEADER_INDENT: f32 = 18.0;
/// A card's margins left and right, with the stripe.
const CARD_MARGINS: f32 = 8.0 + STRIPE + 8.0;

/// Fields side by side in a card's value cell: `n` of them share its width, so they fit
/// beside each other (decision 16). Called inside the row's `horizontal`, `extra` points
/// left for what else is on the row.
pub(in crate::app) fn share_row(ui: &mut egui::Ui, n: usize, extra: f32) {
    let gap = ui.spacing().item_spacing.x;
    let room = field_width(ui, f32::INFINITY) - extra - gap * (n.saturating_sub(1)) as f32;
    let each = (room / n.max(1) as f32).max(40.0);
    let spacing = ui.spacing_mut();
    spacing.interact_size.x = spacing.interact_size.x.min(each);
}

/// The heading the panel shows this frame.
pub(in crate::app) fn set_heading(ui: &egui::Ui, text: &str) {
    ui.data_mut(|d| d.insert_temp(egui::Id::new(HEADING_ID), text.to_owned()));
}

/// Give the cards drawn after this, this frame, a stripe of `color`: an operation's.
pub(in crate::app) fn set_card_stripe(ui: &egui::Ui, color: Color32) {
    ui.data_mut(|d| d.insert_temp(egui::Id::new(STRIPE_ID), color));
}

/// A card: a form's section on a raised shade, with an operation's stripe down its left
/// edge when one is set (docs/specs/look-and-feel.md §5). Its fields are one width.
pub(in crate::app) fn card<R>(ui: &mut egui::Ui, contents: impl FnOnce(&mut egui::Ui) -> R) -> R {
    let palette = crate::style::palette(ui.visuals());
    let stripe = ui.data(|d| d.get_temp::<Color32>(egui::Id::new(STRIPE_ID)));
    let left = 8 + if stripe.is_some() { STRIPE as i8 } else { 0 };
    let shown = egui::Frame::new()
        .fill(palette.card)
        .corner_radius(crate::style::CARD_ROUNDING)
        .inner_margin(egui::Margin {
            left,
            right: 8,
            top: 5,
            bottom: 6,
        })
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            // The card shows what belongs together: its rows need little indent.
            ui.spacing_mut().indent = CARD_INDENT;
            let width = field_width(ui, FIELD_WIDTH);
            // Room for a button beside a number in a narrow panel.
            let number = NUMBER_WIDTH.min(0.6 * field_width(ui, f32::INFINITY));
            let spacing = ui.spacing_mut();
            spacing.interact_size.x = number;
            spacing.combo_width = width;
            spacing.text_edit_width = width;
            contents(ui)
        });
    if let Some(color) = stripe {
        let rect = shown.response.rect;
        let radius = f32::from(crate::style::CARD_ROUNDING);
        let band =
            egui::Rect::from_min_max(rect.min, egui::pos2(rect.left() + STRIPE, rect.bottom()));
        ui.painter().rect_filled(
            band,
            egui::CornerRadius {
                nw: radius as u8,
                sw: radius as u8,
                ne: 0,
                se: 0,
            },
            color,
        );
    }
    ui.add_space(6.0);
    shown.inner
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

/// In the egui memory: folded sections start open instead (tests that read their fields).
pub(in crate::app) const OPEN_SECTIONS: &str = "open-sections";

/// A [`section`] that starts folded, its summary beside its title: an operation's sections,
/// so its properties open on what matters (the user, 2026-11-03: "the user feels
/// overwhelmed with the information"). Opened, it stays open; one with a problem starts
/// open.
pub(in crate::app) fn folded_section<R>(
    ui: &mut egui::Ui,
    key: &str,
    title: &str,
    summary: &str,
    rows: impl FnOnce(&mut egui::Ui) -> R,
) -> Option<R> {
    let marked = problems::section_has_problem(ui, key);
    // A section with a problem opens, so the problem is never folded away.
    let unfolded = ui.data(|d| d.get_temp::<bool>(egui::Id::new(OPEN_SECTIONS)));
    let look = SectionLook {
        open: marked || unfolded == Some(true),
        labels: label_width(ui),
    };
    section_shown(ui, key, title, summary, marked, look, rows)
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
    card(ui, |ui| {
        section_in_card(ui, key, title, summary, marked, look, rows)
    })
}

/// A section's header and rows, in its card. A section named as the panel's heading shows
/// its rows alone, always open: the heading names it (decision 14).
fn section_in_card<R>(
    ui: &mut egui::Ui,
    key: &str,
    title: &str,
    summary: &str,
    marked: bool,
    look: SectionLook,
    rows: impl FnOnce(&mut egui::Ui) -> R,
) -> Option<R> {
    let heading = ui.data(|d| d.get_temp::<String>(egui::Id::new(HEADING_ID)));
    if heading.as_deref() == Some(title) && !marked {
        labels_weak(ui);
        let shown = Grid::new(("section-grid", key))
            .num_columns(2)
            .min_col_width(look.labels + ui.spacing().indent)
            .show(ui, rows);
        return Some(shown.inner);
    }
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
    // The header's arrow keeps egui's room before the title; the rows under it only the
    // card's small indent.
    let indent = ui.spacing().indent;
    ui.spacing_mut().indent = HEADER_INDENT;
    let shown = egui::CollapsingHeader::new(header)
        .id_salt(("section", key))
        .default_open(look.open)
        .show_unindented(ui, |ui| {
            ui.spacing_mut().indent = indent;
            labels_weak(ui);
            egui::Frame::new()
                .inner_margin(egui::Margin {
                    left: indent as i8,
                    ..egui::Margin::ZERO
                })
                .show(ui, |ui| {
                    Grid::new(("section-grid", key))
                        .num_columns(2)
                        .min_col_width(look.labels)
                        .show(ui, rows)
                        .inner
                })
                .inner
        })
        .body_returned;
    ui.spacing_mut().indent = indent;
    shown
}

/// A form's labels in the weak colour, so its values (in their fields, the text of active
/// widgets) and the sections' bold titles stand out (the user, 2026-11-03: "all the font
/// colors and styling is a little same, so it hard to see what is important").
pub(in crate::app) fn labels_weak(ui: &mut egui::Ui) {
    let weak = ui.visuals().weak_text_color();
    ui.visuals_mut().widgets.noninteractive.fg_stroke.color = weak;
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
pub(in crate::app) fn len_in(units: Units, mm: f64) -> String {
    chipload_pipeline::units::length_number(mm, units)
}

/// A feed in mm/min as a number in `units` per minute, for reading.
pub(in crate::app) fn feed_in(units: Units, mm_per_min: f64) -> String {
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
        share_row(ui, 2, 30.0);
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
        share_row(ui, 1, 100.0);
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
