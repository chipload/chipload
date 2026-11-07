// SPDX-License-Identifier: GPL-3.0-or-later
//! The last section of an operation's, pass's or part's properties: its time, lengths and
//! lifts from the last run (docs/specs/operation-statistics.md §3.1,
//! docs/specs/part-statistics.md §3.1).

use super::*;
use chipload_pipeline::estimate::{Counts, duration_message};

/// What the Statistics section can show.
pub(in crate::app) enum Statistics {
    /// From the last run; `copies`: how many times its part is cut; `operations`: for a
    /// part, how many of its operations and passes were counted.
    Counted {
        counts: Counts,
        copies: u32,
        operations: Option<usize>,
    },
    /// A run is under way.
    Generating,
    /// No run since the last change, or none yet.
    OutOfDate,
    /// The last run failed.
    Failed,
    /// It isn't cut: off, or its part or setup is.
    Off,
    /// A part without operations.
    Empty,
}

/// The Statistics section, folded to the time; `id` keeps an operation's folding apart from
/// a part's.
pub(in crate::app) fn statistics_section(ui: &mut egui::Ui, id: &str, statistics: &Statistics) {
    let units: chipload_i18n::Units = crate::units::display(ui.ctx()).into();
    let lang = ui.lang();
    let duration = |seconds: f64| duration_message(seconds).render(lang, units);
    let summary = match statistics {
        Statistics::Counted { counts, .. } => duration(counts.seconds()),
        Statistics::Generating => t!(ui, "forms-statistics-generating"),
        Statistics::OutOfDate => t!(ui, "forms-statistics-out-of-date"),
        Statistics::Failed => t!(ui, "forms-statistics-not-generated"),
        Statistics::Off => t!(ui, "forms-statistics-off-summary"),
        Statistics::Empty => t!(ui, "forms-statistics-empty-summary"),
    };
    let title = t!(ui, "forms-section-statistics");
    folded_section(ui, id, &title, &summary, |ui| {
        let note = match statistics {
            Statistics::Counted {
                counts,
                copies,
                operations,
            } => {
                counted_rows(ui, counts, units);
                if let Some(operations) = operations {
                    ui.label(t!(ui, "forms-operations"))
                        .on_hover_text(t!(ui, "forms-statistics-operations-hover"));
                    ui.label(operations.to_string());
                    ui.end_row();
                }
                if *copies > 1 {
                    ui.label("");
                    ui.label(t!(ui, "forms-statistics-copies", copies = *copies));
                    ui.end_row();
                }
                return;
            }
            Statistics::Generating => t!(ui, "forms-statistics-generating"),
            Statistics::OutOfDate => t!(ui, "forms-statistics-generate"),
            Statistics::Failed => t!(ui, "forms-statistics-failed"),
            Statistics::Off => t!(ui, "forms-statistics-off"),
            Statistics::Empty => t!(ui, "forms-statistics-empty"),
        };
        ui.label("");
        wrapped_cell(ui, |ui| ui.label(note));
        ui.end_row();
    });
}

fn counted_rows(ui: &mut egui::Ui, counts: &Counts, units: chipload_i18n::Units) {
    let lang = ui.lang();
    let duration = |seconds: f64| duration_message(seconds).render(lang, units);
    let distance = |mm: f64| chipload_i18n::message::distance(mm, units);
    // The total, then each kind of move with its length and its time (the user,
    // 2026-11-06: "add the time for each category after the length").
    ui.label(t!(ui, "forms-statistics-time"));
    ui.label(duration(counts.seconds()));
    ui.end_row();
    let rows = [
        (
            t!(ui, "forms-statistics-cutting"),
            t!(ui, "forms-statistics-cutting-hover"),
            counts.cutting_mm,
            counts.cutting_s - counts.plunge_s,
        ),
        (
            t!(ui, "forms-statistics-plunging"),
            t!(ui, "forms-statistics-plunging-hover"),
            counts.plunge_mm,
            counts.plunge_s,
        ),
        (
            t!(ui, "forms-statistics-rapids"),
            t!(ui, "forms-statistics-rapids-hover"),
            counts.rapid_mm,
            counts.rapid_s,
        ),
    ];
    for (label, hover, mm, seconds) in rows {
        ui.label(label).on_hover_text(hover);
        ui.label(t!(
            ui,
            "forms-statistics-length-time",
            length = distance(mm),
            time = duration(seconds)
        ));
        ui.end_row();
    }
    ui.label(t!(ui, "forms-statistics-lifts"))
        .on_hover_text(t!(ui, "forms-statistics-lifts-hover"));
    ui.label(counts.lifts.to_string());
    ui.end_row();
}
