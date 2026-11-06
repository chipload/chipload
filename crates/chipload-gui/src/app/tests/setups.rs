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
