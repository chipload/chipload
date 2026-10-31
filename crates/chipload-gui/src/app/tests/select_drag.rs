// SPDX-License-Identifier: GPL-3.0-or-later
//! Box selection and moving drawn shapes with the mouse (docs/specs/drawing-input.md §3).
use super::*;

fn selected(app: &ChiploadApp) -> BTreeSet<u64> {
    match &app.selection {
        CanvasSelection::Shapes(s) => s.clone(),
        CanvasSelection::Nothing => BTreeSet::new(),
        other => panic!("{other:?}"),
    }
}

fn box_select(app: &mut ChiploadApp, from: Point, to: Point, add: bool, toggle: bool) {
    app.start_select_drag(None, from, add, false);
    app.drag_select_to(to);
    app.end_select_drag(add, toggle);
}

#[test]
fn a_box_selects_what_is_inside_or_what_it_touches() {
    // Circle S2 at (75, 80) r 12, triangle S3 (110, 65), (140, 65), (125, 95), both inside
    // the turned rectangle S1.
    let mut app = drawn_part();
    assert!(app.drag_selects());
    // Left to right round the circle: only the circle.
    box_select(&mut app, (60.0, 60.0), (90.0, 100.0), false, false);
    assert_eq!(selected(&app), [2].into());
    // Right to left over the triangle's left side only: it touches the triangle.
    box_select(&mut app, (118.0, 70.0), (100.0, 75.0), false, false);
    assert_eq!(selected(&app), [3].into());
    // The same box left to right catches nothing, and clears the selection.
    box_select(&mut app, (100.0, 70.0), (118.0, 75.0), false, false);
    assert!(selected(&app).is_empty());
    // Shift adds, Ctrl toggles.
    box_select(&mut app, (60.0, 60.0), (90.0, 100.0), false, false);
    box_select(&mut app, (105.0, 60.0), (145.0, 100.0), true, false);
    assert_eq!(selected(&app), [2, 3].into());
    box_select(&mut app, (60.0, 60.0), (90.0, 100.0), false, true);
    assert_eq!(selected(&app), [3].into());
    // Round everything.
    box_select(&mut app, (0.0, 0.0), (300.0, 200.0), false, false);
    assert_eq!(selected(&app), [1, 2, 3].into());
}

#[test]
fn dragging_a_drawn_shape_moves_it_keeping_its_id() {
    let mut app = drawn_part();
    app.view.scale = 10.0;
    app.click(None, false);
    let circle = contour_of(&app, 2);
    // Grab the circle's right quadrant (87, 80) and drop it on the grid at (100, 50).
    app.start_select_drag(Some(circle), (87.1, 80.2), false, false);
    assert_eq!(
        selected(&app),
        [2].into(),
        "an unselected shape is selected first"
    );
    app.drag_move_to((100.3, 49.8), false, false);
    app.end_move_drag();
    assert_eq!(
        app.shape(2).unwrap().geometry,
        Geometry::Circle {
            center: [88.0, 50.0],
            radius: 12.0
        }
    );
    app.commit();
    // With both drawn shapes selected, dragging one moves both; Shift holds the move to
    // 15° steps (straight up here).
    box_select(&mut app, (60.0, 30.0), (145.0, 100.0), false, false);
    assert_eq!(selected(&app), [2, 3].into());
    let before = app.shape(3).unwrap().geometry.clone();
    app.start_select_drag(Some(contour_of(&app, 3)), (140.0, 65.0), false, false);
    app.drag_move_to((140.4, 75.2), false, true);
    app.end_move_drag();
    assert_eq!(
        app.shape(2).unwrap().geometry,
        Geometry::Circle {
            center: [88.0, 60.0],
            radius: 12.0
        }
    );
    let Geometry::Polyline { points, .. } = &app.shape(3).unwrap().geometry else {
        panic!()
    };
    let Geometry::Polyline { points: was, .. } = &before else {
        panic!()
    };
    assert_eq!(points[0], [was[0][0], was[0][1] + 10.0]);
    // Ids are kept, so the selection still holds them.
    assert_eq!(selected(&app), [2, 3].into());
}

#[test]
fn a_drag_on_an_imported_contour_selects_with_a_box() {
    let mut app = lower_part();
    let picked = Some(app.contours[0].id);
    app.start_select_drag(picked, (0.0, 0.0), false, false);
    assert!(app.drag.moving.is_none());
    assert!(app.drag.select_box.is_some());
}

mod modes {
    //! Edit and Simulate (docs/specs/edit-and-simulate.md §2), with the mouse in the window.
    use super::*;
    use egui_kittest::Harness;
    use egui_kittest::kittest::Queryable;

    fn harness() -> Harness<'static, ChiploadApp> {
        let mut harness = Harness::builder()
            .with_size(egui::vec2(1200.0, 800.0))
            .build_eframe(|_| {
                let mut app = ChiploadApp::default();
                assert!(app.open(&corpus("jobs/v5/drawn_part.chipload")));
                app
            });
        harness.run_steps(6);
        harness
    }

    fn drag(h: &mut Harness<'_, ChiploadApp>, from: (f64, f64), to: (f64, f64)) {
        let (from, to) = (h.state().view.to_screen(from), h.state().view.to_screen(to));
        h.hover_at(from);
        h.run_steps(2);
        h.drag_at(from);
        h.run_steps(2);
        for k in 1..=5 {
            h.hover_at(from + (to - from) * (k as f32 / 5.0));
            h.run_steps(2);
        }
        h.drop_at(to);
        h.run_steps(3);
    }

    #[test]
    fn simulate_is_a_viewer_and_edit_selects_with_a_box_again() {
        let mut h = harness();
        assert!(!h.state().show_simulation, "projects open in Edit");
        // F6: Simulate, which makes the toolpath and its simulation.
        h.key_press(egui::Key::F6);
        h.run_steps(2);
        assert!(h.state().show_simulation);
        assert!(h.state_mut().wait_for_generation() || h.state().result.is_some());
        let ctx = egui::Context::default();
        h.state_mut().wait_for_simulation(&ctx);
        assert!(h.state().sim.simulation.is_some());
        // In Simulate a drag pans: nothing is selected.
        h.state_mut().selection = CanvasSelection::Nothing;
        let before = h.state().view.to_screen((0.0, 0.0));
        drag(&mut h, (40.0, 40.0), (160.0, 120.0));
        assert_eq!(h.state().selection, CanvasSelection::Nothing);
        assert_ne!(h.state().view.to_screen((0.0, 0.0)), before, "panned");
        // F6 again: Edit, where the same drag selects with a box (the user's report).
        h.key_press(egui::Key::F6);
        h.run_steps(2);
        assert!(!h.state().show_simulation);
        h.state_mut().fit_pending = true;
        h.run_steps(3);
        drag(&mut h, (40.0, 40.0), (160.0, 120.0));
        assert_eq!(selected(h.state()), [2, 3].into());
    }

    /// The bar's Edit | Simulate switch (decision 10, the user 2026-10-31: the Simulate
    /// button "does not stand out"): each half sets its mode, and clicking the mode's own
    /// half again keeps it.
    #[test]
    fn the_switch_sets_each_mode() {
        let mut h = harness();
        h.get_by_label("Simulate").click();
        h.run_steps(2);
        assert!(h.state().show_simulation);
        // While it simulates the button reads "Simulate…": wait until it is done.
        assert!(h.state_mut().wait_for_generation() || h.state().result.is_some());
        let ctx = egui::Context::default();
        h.state_mut().wait_for_simulation(&ctx);
        h.run_steps(2);
        h.get_by_label("Simulate").click();
        h.run_steps(2);
        assert!(
            h.state().show_simulation,
            "Simulate's own half keeps Simulate"
        );
        h.get_by_label("Edit").click();
        h.run_steps(2);
        assert!(!h.state().show_simulation);
        h.get_by_label("Edit").click();
        h.run_steps(2);
        assert!(!h.state().show_simulation, "Edit's own half keeps Edit");
    }

    #[test]
    fn a_drawing_tool_or_a_browser_selection_goes_back_to_edit() {
        let mut app = drawn_part();
        app.set_show_simulation(true);
        app.start_drawing(Tool::Circle);
        assert!(!app.show_simulation);
        app.sketch = None;
        app.set_show_simulation(true);
        app.set_show_drawing(false);
        app.select_row(CanvasSelection::Layer("drawn".into(), None), false);
        assert!(!app.show_simulation);
        assert!(app.show_drawing(), "Edit always shows the drawing");
    }
}
