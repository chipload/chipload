// SPDX-License-Identifier: GPL-3.0-or-later
//! Setups in the window (docs/specs/setups.md §7): the Browser, the active setup in full
//! colour and the others as ghosts, a setup's form, and export by setup.
use super::*;

#[test]
fn a_setup_is_made_active_shown_with_the_others_as_ghosts_and_exported() {
    let mut app = ChiploadApp::default();
    assert!(app.open(&corpus("jobs/v34/two_setups.chipload")));
    let job = app.edited_job().unwrap();
    assert_eq!(job.active_setup_id(), Some(1));
    // Setup 2's board is a ghost, and part B's shapes too.
    assert_eq!(app.scene.ghost_boards.len(), 1);
    assert!(!app.scene.ghost_contours.is_empty());
    draw_sidebar(&mut app);
    // Its row makes setup 2 the active one: its board, its part.
    app.activate_setup(2);
    app.focus = Focus::Setup(2);
    let job = app.edited_job().unwrap();
    assert_eq!(job.active_setup_id(), Some(2));
    assert_eq!(job.part(job.active_part_id().unwrap()).unwrap().setup, 2);
    assert_eq!(job.stock().thickness, 6.0);
    draw_properties(&mut app);
    draw_sidebar(&mut app);
    // Generated: only setup 2's operation, and exported with its name.
    assert!(app.generate_and_wait(), "{:?}", app.report);
    let ops: Vec<usize> = app
        .toolpath
        .result
        .as_ref()
        .unwrap()
        .op_spans
        .iter()
        .map(|s| s.operation)
        .collect();
    assert!(ops.iter().all(|&i| i == 3), "{ops:?}");
    let dir = temp_dir("setup-export");
    assert!(app.export_gcode(&dir.join("box.nc")));
    assert!(dir.join("box-S2.nc").exists());
    // A part moves to the other setup; one with operations can't be removed.
    app.move_part_to_setup(2, 1);
    assert_eq!(app.edited_job().unwrap().part(2).unwrap().setup, 1);
    app.remove_setup(1);
    assert_eq!(app.edited_job().unwrap().setups.len(), 2);
    // A new setup: active, with a part of its own; removed again, empty.
    let added = app.add_setup().unwrap();
    assert_eq!(app.focus, Focus::Setup(added));
    app.remove_setup(added);
    assert_eq!(app.edited_job().unwrap().setups.len(), 2);
    app.undo();
    assert_eq!(app.edited_job().unwrap().setups.len(), 3);
}

#[test]
fn a_setup_exports_from_its_right_click_whichever_is_shown() {
    let mut app = ChiploadApp::default();
    assert!(app.open(&corpus("jobs/v34/two_setups.chipload")));
    assert!(app.generate_and_wait(), "{:?}", app.report);
    let dir = temp_dir("setup-export-menu");
    // Setup 2 isn't shown: it becomes active, is planned, then written.
    assert!(!app.export_setup(2, &dir.join("box.nc")), "planned first");
    assert_eq!(app.edited_job().unwrap().active_setup_id(), Some(2));
    assert!(app.wait_for_generation(), "{:?}", app.report);
    assert!(dir.join("box-S2.nc").exists());
    assert!(!dir.join("box-S1-1-T1.nc").exists());
    // Shown and current: written at once.
    assert!(app.export_setup(2, &dir.join("again.nc")));
    assert!(dir.join("again-S2.nc").exists());
}

/// The forms and the zero's handle place a board that fits its parts around its own
/// setup's shapes, as the program does (found 2026-10-30, when the window kept the
/// operations apart from the project: the size line showed the whole drawing's).
#[test]
fn a_fitting_board_and_its_zero_are_placed_around_their_own_parts() {
    let mut app = ChiploadApp::default();
    assert!(app.open(&corpus("jobs/v34/two_setups.chipload")));
    app.activate_setup(2);
    let edit = app.edit.as_mut().unwrap();
    for setup in &mut edit.job.setups {
        setup.stock.origin = chipload_job::StockOrigin::Drawing;
        setup.stock.fit = Some(10.0);
        setup.work_zero = chipload_job::WorkZero::Stock {
            corner: chipload_job::StockCorner::BottomLeft,
        };
    }
    let job = app.edited_job().unwrap();
    let edit = app.edit.as_ref().unwrap();
    assert_eq!(edit.job.board_extents(), job.board_extents());
    assert_ne!(
        edit.job.board_extents(),
        job.drawing.extents(),
        "its own parts, not all"
    );
    assert_eq!(edit.work_zero_xy(), job.work_zero_xy());
    app.focus = Focus::Setup(2);
    draw_properties(&mut app);
    app.focus = Focus::Stock;
    draw_properties(&mut app);
    app.focus = Focus::WorkZero;
    draw_properties(&mut app);
}

/// A new project, empty or imported, opens on its stock (the user, 2026-11-03).
#[test]
fn a_new_project_opens_its_stock() {
    let mut app = ChiploadApp::default();
    app.new_project();
    assert_eq!(app.focus, Focus::Stock);
    let mut app = ChiploadApp::default();
    assert!(app.import_dxf(&corpus("dxf/messy_2.dxf"), None));
    assert_eq!(app.focus, Focus::Stock);
    // An opened project opens on the job, as before.
    assert!(app.open(&corpus("jobs/v31/svg_badge.chipload")));
    assert_eq!(app.focus, Focus::Job);
}

/// A new project's first view rendered to PNG files to look at:
/// `CHIPLOAD_PICTURES=/some/dir cargo test -p chipload-gui new_project_pictures -- --ignored`.
#[test]
#[ignore = "writes pictures; run by hand"]
fn new_project_pictures() {
    use egui_kittest::Harness;
    let Ok(dir) = std::env::var("CHIPLOAD_PICTURES") else {
        panic!("set CHIPLOAD_PICTURES to the folder to write to");
    };
    let dir = PathBuf::from(dir);
    std::fs::create_dir_all(&dir).unwrap();
    for (name, import) in [("new-empty", false), ("new-import", true)] {
        let mut harness = Harness::builder()
            .with_size(egui::vec2(1200.0, 800.0))
            .wgpu()
            .build_eframe(move |cc| {
                {
                    let mut fonts = crate::fonts::ui_fonts();
                    crate::fonts::add_text_fonts(&mut fonts);
                    cc.egui_ctx.set_fonts(fonts);
                }
                crate::style::apply(&cc.egui_ctx);
                let mut app = ChiploadApp::default();
                if import {
                    assert!(app.import_dxf(&corpus("dxf/simple_part.dxf"), None));
                } else {
                    app.new_project();
                }
                app
            });
        harness.run_steps(6);
        let image = harness.render().unwrap();
        image.save(dir.join(format!("{name}.png"))).unwrap();
    }
}

#[test]
fn a_back_side_is_added_turned_given_dowels_and_shares_its_board() {
    // docs/specs/two-sided.md §3, §6.
    use chipload_job::{Register, Turn};
    use chipload_pipeline::setups::BoardChange;
    let mut app = ChiploadApp::default();
    assert!(app.open(&corpus("jobs/v34/two_setups.chipload")));
    // The back of setup 1 goes right after it, active, its form open.
    let back = app.add_back_side(1).unwrap();
    let job = app.edited_job().unwrap();
    let ids: Vec<u64> = job.setups.iter().map(|s| s.id).collect();
    assert_eq!(ids, [1, back, 2]);
    assert_eq!(app.focus, Focus::Setup(back));
    assert_eq!(job.active_setup_id(), Some(back));
    assert_eq!(job.setup(back).unwrap().name, "12 mm sides back");
    assert_eq!(job.stock(), &job.setup(1).unwrap().stock);
    draw_properties(&mut app);
    draw_sidebar(&mut app);
    // Turned the other way, put back by a corner: the form's choices.
    app.set_board(
        back,
        BoardChange {
            turn: Some(Turn::TopBottom),
            register: Some(Register::Corner),
            ..BoardChange::default()
        },
    );
    let link = app
        .edited_job()
        .unwrap()
        .setup(back)
        .unwrap()
        .back_of
        .unwrap();
    assert_eq!(
        (link.turn, link.register),
        (Turn::TopBottom, Register::Corner)
    );
    // The back's stock and zero are read only: the zero's handle doesn't move.
    app.focus = Focus::Stock;
    draw_properties(&mut app);
    app.focus = Focus::WorkZero;
    draw_properties(&mut app);
    assert!(!app.start_zero_drag());
    // The board of its own size leaves no room beyond the cuts for dowel holes: made
    // anyway, the operator's choice (the user, 2026-11-12), then taken back.
    app.add_dowels(back);
    assert!(
        app.edited_job()
            .unwrap()
            .operations
            .iter()
            .any(|op| op.is_dowels()),
        "{:?}",
        app.report
    );
    app.undo();
    // On a board fitted round the parts with room for them: dowel holes, first in the
    // front, one undo step.
    let edit = app.edit.as_mut().unwrap();
    for setup in &mut edit.job.setups {
        setup.stock.origin = chipload_job::StockOrigin::Drawing;
        setup.stock.fit = Some(20.0);
    }
    edit.job.follow_thickness();
    app.commit();
    app.add_dowels(back);
    let job = app.edited_job().unwrap();
    let dowels = job
        .operations
        .iter()
        .position(|op| op.is_dowels())
        .unwrap_or_else(|| panic!("{:?}", app.report));
    assert_eq!(app.focus, Focus::Op(dowels));
    assert_eq!(job.setup_of(&job.operations[dowels]).unwrap().id, 1);
    app.undo();
    assert!(
        !app.edited_job()
            .unwrap()
            .operations
            .iter()
            .any(|op| op.is_dowels())
    );
    // Its own board again: it keeps the board it had.
    app.set_board(
        back,
        BoardChange {
            back_of: Some(None),
            ..BoardChange::default()
        },
    );
    let job = app.edited_job().unwrap();
    assert_eq!(job.setup(back).unwrap().back_of, None);
    assert_eq!(job.setup(back).unwrap().stock, job.setup(1).unwrap().stock);
}

#[test]
fn a_back_is_simulated_turned_over_and_edited_from_the_front() {
    // docs/specs/two-sided.md §10.2, decision 8.
    let mut app = ChiploadApp::default();
    assert!(app.open(&corpus("jobs/v36/two_sided.chipload")));
    app.activate_setup(1);
    assert!(!app.display.turned, "a front face up");
    app.activate_setup(2);
    assert!(app.display.turned, "a back turned over");
    // Edited from the front, as drawn; simulated turned.
    assert_eq!(app.shown_turn(), None);
    app.display.simulate = true;
    assert_eq!(app.shown_turn(), Some(chipload_job::Turn::LeftRight));
    let ctx = egui::Context::default();
    assert!(app.generate_and_wait(), "{:?}", app.report);
    app.start_simulation();
    assert!(app.wait_for_simulation(&ctx));
    let raw = |app: &ChiploadApp| app.scene.sim.as_ref().unwrap().surface.raw;
    assert!(raw(&app), "the turned surface is drawn as it lies");
    app.toggle_turned(&ctx);
    assert!(!raw(&app));
    // A one-sided board has nothing to turn.
    let mut one = ChiploadApp::default();
    assert!(one.open(&corpus("jobs/v34/two_setups.chipload")));
    one.display.simulate = true;
    one.display.turned = true;
    assert_eq!(one.shown_turn(), None);
}

/// A new setup has nothing to cut yet: it isn't planned, and no error says every operation
/// is off (found by the user, 2026-11-06, adding a setup to a project of one).
#[test]
fn a_new_setup_without_operations_is_not_an_error() {
    let mut app = ChiploadApp::default();
    assert!(app.open(&corpus("jobs/v31/svg_badge.chipload")));
    assert!(app.generate_and_wait(), "{:?}", app.report);
    let added = app.add_setup().unwrap();
    assert_eq!(app.edited_job().unwrap().active_setup_id(), Some(added));
    assert!(!app.start_generation(true), "nothing to plan");
    let disabled = t!(app, "toolpath-all-disabled");
    assert!(
        app.report.lines.iter().all(|l| l.text != disabled
            && l.message
                .as_ref()
                .is_none_or(|m| m.id != "toolpath-all-disabled")),
        "{:?}",
        app.report
    );
    // Back on the first setup, it is planned as before.
    app.activate_setup(1);
    assert!(app.generate_and_wait(), "{:?}", app.report);
}

#[test]
fn the_turn_axis_shows_and_missing_dowels_are_warned_in_both_forms() {
    // docs/specs/two-sided-wizard.md §2, §3.
    use egui_kittest::Harness;
    use egui_kittest::kittest::{NodeT, Queryable};
    let project = corpus("jobs/v36/two_sided.chipload");
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1400.0, 900.0))
        .build_eframe(move |_| {
            let mut app = ChiploadApp::default();
            assert!(app.open(&project));
            app
        });
    harness.run_steps(4);
    // The turn axis, across the board's middle (the sample turns left to right).
    let job = harness.state().edited_job().unwrap();
    let ((x0, y0), (x1, y1)) = job.stock_bounds();
    let mid = (x0 + x1) / 2.0;
    let axis = harness
        .state()
        .scene
        .turn_axis
        .clone()
        .expect("the turn axis");
    assert_eq!((axis.0, axis.1), ((mid, y0), (mid, y1)));
    assert!(axis.2.contains("Turn axis"));
    harness.state_mut().display.turn_axis = false;
    harness.run_steps(2);
    assert!(harness.state().scene.turn_axis.is_none());
    // The front's form: its back side, and the dowel holes as a link.
    harness.state_mut().focus = Focus::Setup(1);
    harness.run_steps(3);
    assert!(harness.query_by_label("Back side").is_some());
    let back_name = job.setup(2).unwrap().name.clone();
    assert!(harness.query_by_label(&back_name).is_some());
    // Without dowel holes: None yet, in both forms.
    harness.state_mut().remove_dowels(2);
    harness.run_steps(3);
    assert!(
        !harness
            .state()
            .edited_job()
            .unwrap()
            .operations
            .iter()
            .any(|op| op.is_dowels())
    );
    assert!(harness.query_by_label("None yet").is_some());
    // A link: found by its text among the window's labels; it opens the back's form.
    let warned = format!("{back_name} ⚠");
    harness
        .get_all_by_role(egui::accesskit::Role::Label)
        .find(|n| n.accesskit_node().label().as_deref() == Some(warned.as_str()))
        .expect("the back's link, warned")
        .click();
    harness.run_steps(3);
    assert_eq!(harness.state().focus, Focus::Setup(2));
    harness.state_mut().focus = Focus::Setup(2);
    harness.run_steps(3);
    assert!(harness.query_by_label("None yet").is_some());
    assert!(harness.query_by_label("Add dowel holes").is_some());
    // One undo brings them back.
    harness.state_mut().undo();
    harness.run_steps(2);
    assert!(
        harness
            .state()
            .edited_job()
            .unwrap()
            .operations
            .iter()
            .any(|op| op.is_dowels())
    );
}
