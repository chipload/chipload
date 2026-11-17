// SPDX-License-Identifier: GPL-3.0-or-later
//! Sheets side by side (docs/specs/parts-and-setups.md §7): the active setup in place, every
//! other one a sheet in a row to its right, 50 mm apart, its bottom level with the active
//! board's, with its board, its instances' outlines and its name; a click makes it the
//! active one, the view keeping the point under the pointer.
use super::*;

fn two_setups() -> ChiploadApp {
    let mut app = ChiploadApp::default();
    assert!(app.open(&corpus("jobs/v34/two_setups.chipload")));
    app
}

fn close(a: (f64, f64), b: (f64, f64)) -> bool {
    (a.0 - b.0).abs() < 1e-6 && (a.1 - b.1).abs() < 1e-6
}

#[test]
fn the_other_setups_lie_beside_the_active_board_in_a_row() {
    let mut app = two_setups();
    app.add_setup().unwrap();
    app.activate_setup(1);
    let job = app.edited_job().unwrap();
    let ((_, bottom), (right, _)) = job.stock_bounds();
    let sheets = &app.scene.sheets;
    assert_eq!(sheets.iter().map(|s| s.setup).collect::<Vec<_>>(), [2, 3]);
    // The first 50 mm right of the board, the next 50 mm right of it; bottoms level.
    assert!((sheets[0].board.0.0 - (right + 50.0)).abs() < 1e-9);
    assert!((sheets[1].board.0.0 - (sheets[0].board.1.0 + 50.0)).abs() < 1e-9);
    for sheet in sheets {
        assert!((sheet.board.0.1 - bottom).abs() < 1e-9);
        // Its board is the setup's own, moved by its offset.
        let own = job.for_setup(sheet.setup).stock_bounds();
        let moved = (
            (own.0.0 + sheet.offset.0, own.0.1 + sheet.offset.1),
            (own.1.0 + sheet.offset.0, own.1.1 + sheet.offset.1),
        );
        assert!(close(moved.0, sheet.board.0) && close(moved.1, sheet.board.1));
        assert_eq!(sheet.name, job.setup(sheet.setup).unwrap().name);
    }
    // Setup 2's part B is drawn on its sheet, inside its board.
    let ((x0, y0), (x1, y1)) = sheets[0].board;
    assert!(!sheets[0].outlines.is_empty());
    let tol = 1e-6;
    assert!(
        sheets[0]
            .outlines
            .iter()
            .flatten()
            .all(|p| { p.0 >= x0 - tol && p.0 <= x1 + tol && p.1 >= y0 - tol && p.1 <= y1 + tol })
    );
    // A fit shows them.
    let fit = app.scene.fit_points(0.0);
    assert!(fit.iter().any(|p| p.0 >= sheets[1].board.1.0 - tol));
    draw_canvas(&mut app);
}

#[test]
fn a_click_on_a_sheet_makes_it_active_keeping_the_point_under_the_pointer() {
    let mut app = two_setups();
    draw_canvas(&mut app);
    let sheet = app.scene.sheets[0].clone();
    let ((x0, y0), (x1, y1)) = sheet.board;
    let middle = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);
    let pos = app.view.to_screen(middle);
    assert_eq!(app.sheet_under(pos), Some(2));
    // Not on the active board.
    let active = app.edited_job().unwrap().stock_bounds();
    let inside = (
        (active.0.0 + active.1.0) / 2.0,
        (active.0.1 + active.1.1) / 2.0,
    );
    assert_eq!(app.sheet_under(app.view.to_screen(inside)), None);
    app.activate_sheet(2, pos);
    let job = app.edited_job().unwrap();
    assert_eq!(job.active_setup_id(), Some(2));
    // Its board is in place now, the same point of it under the pointer.
    // (Through the screen's f32 points: within a thousandth of a millimetre.)
    let here = app.view.to_world(pos);
    let want = (middle.0 - sheet.offset.0, middle.1 - sheet.offset.1);
    assert!(
        (here.0 - want.0).abs() < 1e-3 && (here.1 - want.1).abs() < 1e-3,
        "{here:?} {want:?}"
    );
    // Setup 1 is the sheet now; nothing of the sheets is in the project.
    assert_eq!(
        app.scene.sheets.iter().map(|s| s.setup).collect::<Vec<_>>(),
        [1]
    );
    let saved = serde_json::to_string(&job).unwrap();
    assert!(!saved.contains("offset") && !saved.contains("sheet"));
}

#[test]
fn one_board_has_no_sheets() {
    let mut app = lower_part();
    assert!(app.scene.sheets.is_empty());
    draw_canvas(&mut app);
}

/// Two setups and a third, as drawn, for a look by eye.
/// `CHIPLOAD_PICTURES=dir cargo test -p chipload-gui sheets_pictures -- --ignored`.
#[test]
#[ignore = "writes pictures; needs CHIPLOAD_PICTURES"]
fn sheets_pictures() {
    let Ok(dir) = std::env::var("CHIPLOAD_PICTURES") else {
        panic!("set CHIPLOAD_PICTURES to the folder to write to");
    };
    let dir = std::path::PathBuf::from(dir);
    std::fs::create_dir_all(&dir).unwrap();
    let mut h = egui_kittest::Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .wgpu()
        .build_eframe(|cc| {
            let mut fonts = crate::fonts::ui_fonts();
            crate::fonts::add_text_fonts(&mut fonts);
            cc.egui_ctx.set_fonts(fonts);
            crate::style::apply(&cc.egui_ctx);
            let mut app = two_setups();
            app.add_setup().unwrap();
            app.activate_setup(1);
            app.fit_pending = true;
            app
        });
    h.run_steps(8);
    h.render().unwrap().save(dir.join("sheets.png")).unwrap();
}
