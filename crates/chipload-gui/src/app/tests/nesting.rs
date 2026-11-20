// SPDX-License-Identifier: GPL-3.0-or-later
//! The Nest window (docs/specs/nesting.md §7): opened on a setup, previewed on a worker
//! thread as ghosts on the board, applied as one undo step; Nest again with the kept
//! settings; a refusal shown in the window.
use super::*;
use egui_kittest::Harness;

/// The badge (three instances, one turned, one mirrored) on a 400 × 300 board.
fn badge() -> ChiploadApp {
    let mut app = ChiploadApp::default();
    assert!(app.open(&corpus("jobs/v40/nest_badge.chipload")));
    app
}

fn placements(app: &ChiploadApp) -> Vec<(f64, [f64; 2])> {
    let job = &app.edit.as_ref().unwrap().job;
    job.instances_of(1)
        .iter()
        .map(|(_, i)| (i.angle, i.at))
        .collect()
}

#[test]
fn a_nest_is_previewed_as_ghosts_and_applied_as_one_undo_step() {
    let mut app = badge();
    let before = placements(&app);
    app.start_nest(None);
    {
        let dialog = app.nest.as_mut().unwrap();
        assert_eq!(dialog.rows.len(), 1);
        assert_eq!(dialog.rows[0].count, 3);
        assert_eq!((dialog.gap, dialog.margin), (3.0, 10.0));
        assert!(!dialog.follows);
        dialog.rows[0].count = 5;
    }
    // The count changed after the first run began: run again for it.
    app.start_nest_run();
    app.wait_for_nest();
    let preview = app.nest.as_ref().unwrap().preview.as_ref().unwrap();
    let result = &preview.as_ref().unwrap().result;
    assert_eq!(result.placed.len(), 5);
    // The canvas shows the five where they would go, not the instances as they are.
    draw_canvas(&mut app);
    assert_eq!(app.scene.nest_preview.len(), 5);
    assert!(app.scene.nest_preview.iter().all(|(_, parked)| !parked));
    assert!(app.scene.instances.is_empty());
    assert!(app.scene.nest_board.is_some());
    // Nothing is changed until Apply.
    assert_eq!(placements(&app), before);
    let result = app
        .nest
        .as_mut()
        .unwrap()
        .preview
        .take()
        .unwrap()
        .unwrap()
        .result;
    app.apply_nest(result);
    assert!(app.nest.is_none());
    let job = &app.edit.as_ref().unwrap().job;
    assert_eq!(job.count(1), 5);
    assert!(job.setups[0].nest.is_some());
    draw_canvas(&mut app);
    assert!(app.scene.nest_preview.is_empty());
    // One undo step puts everything back.
    app.undo();
    assert_eq!(placements(&app), before);
    assert!(app.edit.as_ref().unwrap().job.setups[0].nest.is_none());
}

/// What no board can take (larger than an empty one) is parked beside the board; what
/// merely doesn't fit goes onto more boards (docs/specs/nest-overflow-and-all.md §1).
#[test]
fn what_no_board_takes_is_parked_beside_the_board_in_the_preview() {
    let mut app = badge();
    app.start_nest(None);
    {
        let dialog = app.nest.as_mut().unwrap();
        dialog.board = (40.0, 30.0);
        dialog.rows[0].count = 2;
    }
    app.start_nest_run();
    app.wait_for_nest();
    draw_canvas(&mut app);
    let parked = app.scene.nest_preview.iter().filter(|(_, p)| *p).count();
    assert!(parked > 0, "{:?}", app.scene.nest_preview.len());
    // Parked right of the board.
    let ((_, _), (x1, _)) = app.scene.nest_board.unwrap();
    for (outlines, is_parked) in &app.scene.nest_preview {
        if *is_parked {
            let left = outlines
                .iter()
                .flatten()
                .map(|p| p.0)
                .fold(f64::MAX, f64::min);
            assert!(left > x1, "{left} vs the board's right {x1}");
        }
    }
}

#[test]
fn nest_again_uses_the_kept_settings_without_the_window() {
    let mut app = badge();
    assert!(!app.was_nested(1));
    app.start_nest(None);
    app.nest.as_mut().unwrap().gap = 6.0;
    app.start_nest_run();
    app.wait_for_nest();
    let result = app
        .nest
        .as_mut()
        .unwrap()
        .preview
        .take()
        .unwrap()
        .unwrap()
        .result;
    app.apply_nest(result);
    assert!(app.was_nested(1));
    // Moved by hand, then nested again: back to the nest, with the 6 mm gap.
    let nested = placements(&app);
    app.edit
        .as_mut()
        .unwrap()
        .job
        .instance_mut(1, 1)
        .unwrap()
        .at = [5.0, 5.0];
    app.commit();
    app.nest_again(1);
    assert!(app.nest.is_some());
    app.wait_for_nest();
    assert!(app.nest.is_none(), "applied and closed");
    assert_eq!(placements(&app), nested);
    assert_eq!(
        app.edit.as_ref().unwrap().job.setups[0].nest.unwrap().gap,
        6.0
    );
}

#[test]
fn a_refusal_is_shown_in_the_window() {
    let mut app = badge();
    app.start_nest(None);
    app.nest.as_mut().unwrap().margin = 400.0;
    app.start_nest_run();
    app.wait_for_nest();
    match app.nest.as_ref().unwrap().preview.as_ref().unwrap() {
        Err(text) => assert!(text.contains("No room is left"), "{text}"),
        Ok(_) => panic!("a 400 mm margin leaves no room"),
    }
}

#[test]
fn the_window_draws_its_rows_and_buttons() {
    let mut h = Harness::builder()
        .with_size(egui::vec2(1200.0, 800.0))
        .build_eframe(|_| {
            let mut app = badge();
            app.start_nest(None);
            app.wait_for_nest();
            app
        });
    h.run_steps(4);
    use egui_kittest::kittest::Queryable;
    for label in [
        "Board", "Gap", "Margin", "Times", "Turns", "Apply", "Cancel",
    ] {
        assert!(h.query_by_label(label).is_some(), "{label}");
    }
    assert!(
        h.query_by_label_contains("3 of 3 placed").is_some(),
        "the summary"
    );
}

/// The window and its preview as drawn, for a look by eye: five badges nested, then a
/// board too small for them all, the rest onto more boards.
/// `CHIPLOAD_PICTURES=dir cargo test -p chipload-gui pictures -- --ignored`.
#[test]
#[ignore = "writes pictures; needs CHIPLOAD_PICTURES"]
fn nest_pictures() {
    let Ok(dir) = std::env::var("CHIPLOAD_PICTURES") else {
        panic!("set CHIPLOAD_PICTURES to the folder to write to");
    };
    let dir = std::path::PathBuf::from(dir);
    std::fs::create_dir_all(&dir).unwrap();
    for (name, board, count) in [
        ("nest", (400.0, 300.0), 5),
        ("nest-overflow", (200.0, 150.0), 6),
    ] {
        let mut h = Harness::builder()
            .with_size(egui::vec2(1280.0, 800.0))
            .wgpu()
            .build_eframe(move |cc| {
                let mut fonts = crate::fonts::ui_fonts();
                crate::fonts::add_text_fonts(&mut fonts);
                cc.egui_ctx.set_fonts(fonts);
                crate::style::apply(&cc.egui_ctx);
                let mut app = badge();
                app.start_nest(None);
                {
                    let dialog = app.nest.as_mut().unwrap();
                    dialog.board = board;
                    dialog.rows[0].count = count;
                }
                app.start_nest_run();
                app.wait_for_nest();
                app.fit_pending = true;
                app
            });
        h.run_steps(8);
        let image = h.render().unwrap();
        image.save(dir.join(format!("{name}.png"))).unwrap();
    }
}

/// docs/specs/parts-and-setups.md §9: what doesn't fit goes onto more boards, previewed
/// beside the first, applied as one undo step.
#[test]
fn what_doesnt_fit_goes_onto_more_boards_previewed_beside_the_first() {
    let mut app = badge();
    app.start_nest(None);
    {
        let dialog = app.nest.as_mut().unwrap();
        dialog.board = (200.0, 150.0);
        dialog.rows[0].count = 6;
    }
    app.start_nest_run();
    app.wait_for_nest();
    let result = match app.nest.as_ref().unwrap().preview.as_ref().unwrap() {
        Ok(p) => p.result.clone(),
        Err(e) => panic!("{e}"),
    };
    assert!(result.unplaced.is_empty(), "{:?}", result.unplaced);
    assert!(!result.overflow.is_empty());
    assert_eq!(result.placed_count(), 6);
    draw_canvas(&mut app);
    // Each extra board right of the first, none parked.
    let ((_, _), (x1, _)) = app.scene.nest_board.unwrap();
    assert_eq!(app.scene.nest_more_boards.len(), result.overflow.len());
    assert!(app.scene.nest_more_boards.iter().all(|b| b.0.0 > x1));
    assert_eq!(app.scene.nest_preview.len(), 6);
    assert!(app.scene.nest_preview.iter().all(|(_, parked)| !parked));
    let before = app.edit.as_ref().unwrap().job.setups.len();
    app.apply_nest(result.clone());
    assert_eq!(
        app.edit.as_ref().unwrap().job.setups.len(),
        before + result.overflow.len()
    );
    app.undo();
    assert_eq!(app.edit.as_ref().unwrap().job.setups.len(), before);
}

/// docs/specs/nest-overflow-and-all.md §1: what no board takes is switched off, marked in
/// the Browser, and its Report line opens it.
#[test]
fn what_no_board_takes_is_marked_and_its_report_line_opens_it() {
    let mut app = badge();
    app.start_nest(None);
    {
        let dialog = app.nest.as_mut().unwrap();
        dialog.board = (40.0, 30.0);
        dialog.rows[0].count = 2;
    }
    app.start_nest_run();
    app.wait_for_nest();
    let result = match app.nest.as_ref().unwrap().preview.as_ref().unwrap() {
        Ok(p) => p.result.clone(),
        Err(e) => panic!("{e}"),
    };
    assert!(!result.unplaced.is_empty());
    app.apply_nest(result.clone());
    for &(part, id) in &result.unplaced {
        assert!(app.nest_too_large.contains(&(part, id)));
        assert!(!app.instance_value(part, id).unwrap().enabled);
    }
    let (part, id) = result.unplaced[0];
    let line = app
        .report
        .lines
        .iter()
        .find(|l| l.target == Some(crate::app::report::Target::Instance(part, id)))
        .expect("a line for it");
    assert!(
        line.text.contains("larger than an empty board"),
        "{}",
        line.text
    );
    // The Browser draws it marked.
    draw_sidebar(&mut app);
}

/// docs/specs/nest-overflow-and-all.md §2: *Nest all setups again* pools the boards, as one
/// undo step; offered with two boards that can be nested.
#[test]
fn every_setup_is_nested_again_as_one_undo_step() {
    let mut app = badge();
    assert!(!app.can_nest_all(), "one board");
    let two = app.add_setup().unwrap();
    app.activate_setup(1);
    for x in [0.0, 300.0] {
        let job = app.edit.as_ref().unwrap().to_job().unwrap();
        let (made, _) = chipload_pipeline::instances::add_instance(
            &job,
            1,
            Some(two),
            Some([x, 0.0]),
            0.0,
            None,
        )
        .unwrap();
        app.edit.as_mut().unwrap().job = made;
        app.commit();
    }
    assert!(app.can_nest_all());
    let before = app.edit.as_ref().unwrap().job.clone();
    app.start_nest_all();
    app.wait_for_nest_all();
    let job = &app.edit.as_ref().unwrap().job;
    // Everything on the first board now; the second kept, empty.
    assert!(
        job.setup(two).unwrap().instances.is_empty(),
        "{:?}",
        job.setup(two)
    );
    assert_eq!(job.setups.len(), before.setups.len());
    assert!(app.report.iter().any(|l| l.contains("is empty now")));
    app.undo();
    assert_eq!(app.edit.as_ref().unwrap().job.setups, before.setups);
}
