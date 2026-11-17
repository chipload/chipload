// SPDX-License-Identifier: GPL-3.0-or-later
//! A part's stock in the window (docs/specs/parts-and-setups.md §4): set in the part's form,
//! the part moves to a board of that stock as one undo step, the status saying where; the
//! Browser shows the boards once there are two.
use super::*;

fn thick(mm: f64) -> Option<chipload_job::PartStock> {
    Some(chipload_job::PartStock {
        material: None,
        thickness: Some(mm),
    })
}

#[test]
fn a_parts_thickness_moves_it_to_a_board_of_its_own_as_one_undo_step() {
    let mut app = two_parts();
    // Its outline cut through, whatever the board.
    if let Some(t) = app.edit.as_mut().unwrap().job.operations[3].through_mut() {
        *t = Some(0.0);
    }
    app.commit();
    assert_eq!(app.edit.as_ref().unwrap().job.setups.len(), 1);
    let listed = browser_rows(&mut app).iter().any(|(_, l)| l == "Setups");
    // The form draws its stock section, "Any board".
    app.focus = Focus::Part(2);
    draw_properties(&mut app);
    app.set_part_stock(2, thick(6.0));
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
    // The Browser shows both boards now.
    assert!(browser_rows(&mut app).iter().any(|(_, l)| l == "Setups"));
    draw_properties(&mut app);
    // Generated: its setup's program.
    assert!(app.generate_and_wait(), "{:?}", app.report);
    // One undo step back: one board, the part on it, no stock.
    app.undo();
    let job = &app.edit.as_ref().unwrap().job;
    assert_eq!(job.setups.len(), 1);
    assert!(job.part(2).unwrap().stock.is_none());
    assert_eq!(
        browser_rows(&mut app).iter().any(|(_, l)| l == "Setups"),
        listed
    );
}

#[test]
fn a_made_board_changed_in_the_window_is_the_users() {
    let mut app = two_parts();
    if let Some(t) = app.edit.as_mut().unwrap().job.operations[3].through_mut() {
        *t = Some(0.0);
    }
    app.commit();
    app.set_part_stock(2, thick(6.0));
    let made = app.edit.as_ref().unwrap().job.setups[1].id;
    app.edit.as_mut().unwrap().job.setups[1].name = "Bottoms".into();
    app.commit();
    assert!(!app.edit.as_ref().unwrap().job.setups[1].made_for_stock);
    // Back to any board: the part stays, the board is kept.
    app.set_part_stock(2, None);
    let job = &app.edit.as_ref().unwrap().job;
    assert_eq!(job.setups.len(), 2);
    assert_eq!(job.setups_of_part(2), [made]);
}
