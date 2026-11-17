// SPDX-License-Identifier: GPL-3.0-or-later
//! A part's stock in the window (docs/specs/parts-and-setups.md §4): set in the part's form,
//! recorded, nothing moved; on a board of another stock the form says so and offers the
//! move, one undo step, the status saying where.
use super::*;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;

fn thick(mm: f64) -> Option<chipload_job::PartStock> {
    Some(chipload_job::PartStock {
        material: None,
        thickness: Some(mm),
    })
}

/// part B cut through, whatever the board, its stock 6 mm on the 12 mm board.
fn asking_for_six() -> ChiploadApp {
    let mut app = two_parts();
    if let Some(t) = app.edit.as_mut().unwrap().job.operations[3].through_mut() {
        *t = Some(0.0);
    }
    app.commit();
    app.focus = Focus::Part(2);
    draw_properties(&mut app);
    app.set_part_stock(2, thick(6.0));
    app
}

#[test]
fn a_parts_thickness_is_recorded_and_the_move_is_offered() {
    let mut app = asking_for_six();
    // Recorded, nothing made or moved (the user, 2026-11-17).
    let job = &app.edit.as_ref().unwrap().job;
    assert_eq!(job.setups.len(), 1);
    assert_eq!(job.part(2).unwrap().stock, thick(6.0));
    draw_properties(&mut app);
    // Moved when asked: one undo step, the status saying where.
    app.move_to_stock_board(2);
    let job = &app.edit.as_ref().unwrap().job;
    assert_eq!(job.setups.len(), 2);
    assert_eq!(job.setups[1].stock.thickness, 6.0);
    assert_eq!(job.setups_of_part(2), [job.setups[1].id]);
    let status = app
        .status
        .as_ref()
        .map(|(s, _)| s.clone())
        .unwrap_or_default();
    assert!(
        status.contains("Made Setup 2 (6 mm) for part B"),
        "{status}"
    );
    assert!(browser_rows(&mut app).iter().any(|(_, l)| l == "Setups"));
    assert!(app.generate_and_wait(), "{:?}", app.report);
    // Back one step: one board, the stock still set.
    app.undo();
    let job = &app.edit.as_ref().unwrap().job;
    assert_eq!(job.setups.len(), 1);
    assert_eq!(job.part(2).unwrap().stock, thick(6.0));
}

#[test]
fn the_forms_button_moves_the_part() {
    let mut h = Harness::builder()
        .with_size(egui::vec2(1400.0, 900.0))
        .build_eframe(|_| asking_for_six());
    h.run_steps(4);
    assert!(
        h.query_by_label("This part's board isn't of its stock")
            .is_some()
    );
    h.get_by_label("Move to a new 6 mm setup").click();
    h.run_steps(4);
    let job = &h.state().edit.as_ref().unwrap().job;
    assert_eq!(job.setups.len(), 2);
    assert_eq!(job.setups_of_part(2), [job.setups[1].id]);
    // On its stock now: no offer.
    assert!(
        h.query_by_label("This part's board isn't of its stock")
            .is_none()
    );
}
