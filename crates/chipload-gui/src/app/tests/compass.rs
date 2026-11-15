// SPDX-License-Identifier: GPL-3.0-or-later
//! The Compass (docs/specs/compass.md): moving and turning the selection, placing the
//! Compass with Space, the right drag's menu, exact values, the keys.
use super::super::compass::CompassPart;
use super::super::modify::ModifyTool;
use super::*;
use egui_kittest::kittest::Queryable;

/// drawn_part with the triangle S3 (110, 65), (140, 65), (125, 95) selected and the
/// Compass shown on it, the view 10 px to the mm.
fn on_triangle() -> ChiploadApp {
    let mut app = drawn_part();
    app.view.scale = 10.0;
    app.click(Some(contour_of(&app, 3)), false);
    app.toggle_compass();
    app
}

fn triangle(app: &ChiploadApp) -> Vec<[f64; 2]> {
    match &app.shape(3).unwrap().geometry {
        Geometry::Polyline { points, .. } => points.clone(),
        other => panic!("{other:?}"),
    }
}

fn near(a: [f64; 2], b: [f64; 2]) -> bool {
    (a[0] - b[0]).abs() < 1e-6 && (a[1] - b[1]).abs() < 1e-6
}

/// On screen, `dx`, `dy` pixels from the Compass's centre.
fn beside(app: &mut ChiploadApp, dx: f32, dy: f32) -> egui::Pos2 {
    let (centre, _, _) = app.compass_now().unwrap();
    app.view.to_screen(centre) + egui::vec2(dx, dy)
}

#[test]
fn it_shows_on_the_middle_of_the_selection_and_an_arrow_moves_along_its_axis() {
    let mut app = on_triangle();
    let (centre, angle, unlocked) = app.compass_now().unwrap();
    assert_eq!((centre, angle, unlocked), ((125.0, 80.0), 0.0, false));
    // Dragged along X, the cursor off the axis too: only X changes.
    let at = beside(&mut app, 45.0, 0.0);
    assert_eq!(app.compass_part_at(at), Some(CompassPart::X));
    assert!(app.start_compass_drag(CompassPart::X, at, false));
    app.drag_compass_to((129.5 + 23.25, 84.0), true);
    app.end_compass_drag(None);
    let was = [[110.0, 65.0], [140.0, 65.0], [125.0, 95.0]];
    let moved: Vec<[f64; 2]> = was.iter().map(|p| [p[0] + 23.25, p[1]]).collect();
    assert!(triangle(&app).iter().zip(&moved).all(|(a, b)| near(*a, *b)));
    // The Compass went with it, and the shape kept its id and its selection.
    assert_eq!(app.compass_now().unwrap().0, (148.25, 80.0));
    assert_eq!(app.selection, CanvasSelection::Shapes([3].into()));
    // One undo step.
    app.commit();
    app.undo();
    assert!(triangle(&app).iter().zip(&was).all(|(a, b)| near(*a, *b)));
}

#[test]
fn the_ring_turns_about_the_centre_in_15_degree_steps() {
    let mut app = on_triangle();
    let at = beside(&mut app, 18.0, 0.0);
    assert_eq!(app.compass_part_at(at), Some(CompassPart::Ring));
    assert!(app.start_compass_drag(CompassPart::Ring, at, false));
    // Grabbed to the right of the centre, let go nearly straight above it: 88° → 90°.
    let up = (
        125.0 + 10.0 * chipload_core::math::cos(88f64.to_radians()),
        80.0 + 10.0 * chipload_core::math::sin(88f64.to_radians()),
    );
    app.drag_compass_to(up, false);
    assert_eq!(app.compass_readout().as_deref(), Some("90.0°"));
    app.end_compass_drag(None);
    // (110, 65) is (−15, −15) from the centre: turned 90°, (15, −15).
    assert!(
        near(triangle(&app)[0], [140.0, 65.0]),
        "{:?}",
        triangle(&app)
    );
    let (centre, angle, _) = app.compass_now().unwrap();
    assert_eq!((centre, angle), ((125.0, 80.0), 90.0));
}

#[test]
fn the_centre_snaps_the_selection_onto_another_shapes_point() {
    let mut app = on_triangle();
    let at = beside(&mut app, 0.0, 0.0);
    assert_eq!(app.compass_part_at(at), Some(CompassPart::Centre));
    assert!(app.start_compass_drag(CompassPart::Centre, at, false));
    // Near the circle S2's centre (75, 80): snapped onto it.
    app.drag_compass_to((75.2, 80.15), false);
    app.end_compass_drag(None);
    assert!(
        near(triangle(&app)[0], [60.0, 65.0]),
        "{:?}",
        triangle(&app)
    );
}

#[test]
fn a_typed_value_moves_exactly() {
    let mut app = on_triangle();
    let at = beside(&mut app, 0.0, -45.0);
    app.start_typing_compass(CompassPart::Y, at);
    app.canvas_tools.typed.as_mut().unwrap().text = "12.5".into();
    app.apply_typed((0.0, 0.0));
    assert!(app.canvas_tools.typed.is_none());
    assert!(near(triangle(&app)[0], [110.0, 77.5]));
    // The centre takes ΔX and ΔY; something unreadable stays open.
    let at = beside(&mut app, 0.0, 0.0);
    app.start_typing_compass(CompassPart::Centre, at);
    app.canvas_tools.typed.as_mut().unwrap().text = "nonsense".into();
    app.apply_typed((0.0, 0.0));
    assert!(app.canvas_tools.typed.as_ref().is_some_and(|t| t.refused));
    app.canvas_tools.typed.as_mut().unwrap().text = "-10 5".into();
    app.apply_typed((0.0, 0.0));
    assert!(near(triangle(&app)[0], [100.0, 82.5]));
}

#[test]
fn a_new_selection_starts_it_afresh_and_none_hides_it() {
    let mut app = on_triangle();
    let at = beside(&mut app, 45.0, 0.0);
    app.start_compass_drag(CompassPart::X, at, false);
    app.drag_compass_to((140.0, 80.0), true);
    app.end_compass_drag(None);
    app.click(Some(contour_of(&app, 2)), false);
    assert_eq!(app.compass_now().unwrap().0, (75.0, 80.0));
    app.selection = CanvasSelection::Nothing;
    assert!(app.compass_now().is_none());
    assert!(!app.compass_shown());
}

/// A window on `app`, the Compass's centre at `centre` on screen once laid out.
fn window(app: ChiploadApp) -> egui_kittest::Harness<'static, ChiploadApp> {
    let mut app = Some(app);
    let mut harness = egui_kittest::Harness::builder()
        .with_size(egui::vec2(1200.0, 800.0))
        .build_eframe(move |cc| {
            cc.egui_ctx.set_fonts(crate::fonts::ui_fonts());
            app.take().unwrap()
        });
    harness.run_steps(2);
    harness.ctx.memory_mut(|m| m.stop_text_input());
    harness.run_steps(1);
    harness
}

fn centre_on_screen(harness: &mut egui_kittest::Harness<'_, ChiploadApp>) -> egui::Pos2 {
    let app = harness.state_mut();
    let (centre, _, _) = app.compass_now().unwrap();
    app.view.to_screen(centre)
}

/// A drag with `button` from `from` to `to` on screen, in steps as a hand would.
fn drag_with(
    harness: &mut egui_kittest::Harness<'_, ChiploadApp>,
    button: egui::PointerButton,
    from: egui::Pos2,
    to: egui::Pos2,
) {
    let press = |pos, pressed| egui::Event::PointerButton {
        pos,
        button,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    harness.hover_at(from);
    harness.run_steps(2);
    harness.event(press(from, true));
    harness.run_steps(2);
    for k in 1..=5 {
        harness.hover_at(from + (to - from) * (k as f32 / 5.0));
        harness.run_steps(2);
    }
    harness.event(press(to, false));
    harness.run_steps(3);
}

#[test]
fn q_shows_it_space_unlocks_it_to_be_put_on_a_point() {
    let mut app = drawn_part();
    app.click(Some(contour_of(&app, 3)), false);
    let mut harness = window(app);
    harness.key_press(egui::Key::Q);
    harness.run_steps(2);
    assert!(harness.state().compass_shown());
    // A tap of Space unlocks it.
    harness.key_down(egui::Key::Space);
    harness.run_steps(1);
    harness.key_up(egui::Key::Space);
    harness.run_steps(2);
    assert!(harness.state_mut().compass_now().unwrap().2, "unlocked");
    // Unlocked, its centre goes onto the triangle's own corner (110, 65); nothing moves.
    let before = triangle(harness.state());
    let from = centre_on_screen(&mut harness);
    let corner = harness.state().view.to_screen((110.0, 65.0)) + egui::vec2(2.0, -1.0);
    drag_with(&mut harness, egui::PointerButton::Primary, from, corner);
    assert_eq!(triangle(harness.state()), before);
    assert_eq!(harness.state_mut().compass_now().unwrap().0, (110.0, 65.0));
    // Locked again, the next drag is measured from there: the corner onto the circle's
    // centre (75, 80).
    harness.key_down(egui::Key::Space);
    harness.run_steps(1);
    harness.key_up(egui::Key::Space);
    harness.run_steps(2);
    assert!(!harness.state_mut().compass_now().unwrap().2, "locked");
    let from = centre_on_screen(&mut harness);
    let circle = harness.state().view.to_screen((75.0, 80.0)) + egui::vec2(1.0, 1.0);
    drag_with(&mut harness, egui::PointerButton::Primary, from, circle);
    assert!(
        near(triangle(harness.state())[0], [75.0, 80.0]),
        "{:?}",
        triangle(harness.state())
    );
    // Escape hides it, keeping the selection; Q shows it again.
    harness.key_press(egui::Key::Escape);
    harness.run_steps(2);
    assert!(!harness.state().compass_shown());
    assert_eq!(
        harness.state().selection,
        CanvasSelection::Shapes([3].into())
    );
}

#[test]
fn a_right_drag_asks_and_copies_or_opens_an_array() {
    let mut app = drawn_part();
    app.click(Some(contour_of(&app, 3)), false);
    let mut harness = window(app);
    harness.key_press(egui::Key::Q);
    harness.run_steps(2);
    let shapes = |h: &egui_kittest::Harness<'_, ChiploadApp>| {
        h.state().edit.as_ref().unwrap().job.drawing.shapes.len()
    };
    let before = shapes(&harness);
    // Right-dragged along X: Copy here.
    let centre = centre_on_screen(&mut harness);
    let arm = centre + egui::vec2(45.0, 0.0);
    drag_with(
        &mut harness,
        egui::PointerButton::Secondary,
        arm,
        arm + egui::vec2(120.0, 0.0),
    );
    harness.get_by_label("Copy here").click();
    harness.run_steps(2);
    assert_eq!(shapes(&harness), before + 1);
    let copy = match &harness.state().selection {
        CanvasSelection::Shapes(ids) => *ids.iter().next().unwrap(),
        other => panic!("{other:?}"),
    };
    assert_ne!(copy, 3, "the copy is selected");
    assert!(harness.state().compass_shown());
    // Right-dragged on the ring: Circular array… opens the polar array with the angle and
    // the Compass's centre.
    let centre = centre_on_screen(&mut harness);
    let world = harness.state_mut().compass_now().unwrap().0;
    let ring = centre + egui::vec2(18.0, 0.0);
    drag_with(
        &mut harness,
        egui::PointerButton::Secondary,
        ring,
        centre + egui::vec2(0.0, -60.0),
    );
    harness.get_by_label("Circular array…").click();
    harness.run_steps(2);
    let app = harness.state();
    assert_eq!(app.modify(), Some(ModifyTool::PolarArray));
    assert_eq!(app.canvas_tools.array.angle_deg, 90.0);
    assert_eq!(app.canvas_tools.array.count, 2);
    let c = app.canvas_tools.array.center;
    assert!((c.0 - world.0).abs() < 1e-6 && (c.1 - world.1).abs() < 1e-6);
}

#[test]
fn a_whole_text_turns_about_the_compass() {
    use chipload_core::chain::ShapeFrom;
    let mut app = ChiploadApp::default();
    app.new_project();
    app.view.scale = 10.0;
    app.start_text();
    let id = app.place_text_at((20.0, 30.0)).unwrap();
    app.selection = CanvasSelection::whole(ShapeFrom::Text(id));
    app.toggle_compass();
    let text = |app: &ChiploadApp| app.edit.as_ref().unwrap().job.drawing.texts[0].clone();
    let before = text(&app);
    let at = beside(&mut app, 18.0, 0.0);
    assert!(app.start_compass_drag(CompassPart::Ring, at, false));
    let (centre, _, _) = app.compass_now().unwrap();
    app.drag_compass_to((centre.0, centre.1 + 10.0), false);
    app.end_compass_drag(None);
    let after = text(&app);
    assert!(
        (after.rotation - (before.rotation + 90.0)).abs() < 1e-9,
        "{after:?}"
    );
    assert_eq!(app.selection, CanvasSelection::whole(ShapeFrom::Text(id)));
    // The Compass stays on its centre: the box's middle didn't move.
    let (still, _, _) = app.compass_now().unwrap();
    assert!((still.0 - centre.0).abs() < 1e-6 && (still.1 - centre.1).abs() < 1e-6);
}

/// A click with `button` at `pos` on screen, not moving.
fn click_with(
    harness: &mut egui_kittest::Harness<'_, ChiploadApp>,
    button: egui::PointerButton,
    pos: egui::Pos2,
) {
    let press = |pressed| egui::Event::PointerButton {
        pos,
        button,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    harness.hover_at(pos);
    harness.run_steps(2);
    harness.event(press(true));
    harness.run_steps(1);
    harness.event(press(false));
    harness.run_steps(3);
}

#[test]
fn a_right_click_on_the_centre_moves_it_to_a_clicked_point() {
    // docs/specs/compass.md §15: the Compass's own menu, not the canvas's.
    let mut app = drawn_part();
    app.click(Some(contour_of(&app, 3)), false);
    let mut harness = window(app);
    harness.key_press(egui::Key::Q);
    harness.run_steps(2);
    let centre = centre_on_screen(&mut harness);
    click_with(&mut harness, egui::PointerButton::Secondary, centre);
    assert!(harness.state().compass_pick_menu_open());
    assert!(
        !egui::Popup::is_any_open(&harness.ctx),
        "the canvas's menu stays shut"
    );
    harness.get_by_label("To point").click();
    harness.run_steps(2);
    assert!(harness.state().compass_picking());
    // The circle S2's centre (75, 80), clicked a hair off: snapped onto it, the triangle's
    // middle (125, 80) moved there.
    let circle = harness.state().view.to_screen((75.0, 80.0)) + egui::vec2(1.0, 1.0);
    click_with(&mut harness, egui::PointerButton::Primary, circle);
    assert!(!harness.state().compass_picking());
    assert!(
        near(triangle(harness.state())[0], [60.0, 65.0]),
        "{:?}",
        triangle(harness.state())
    );
    assert_eq!(
        harness.state().selection,
        CanvasSelection::Shapes([3].into()),
        "the selection stays"
    );
    // Escape cancels a pick, moving nothing.
    let centre = centre_on_screen(&mut harness);
    click_with(&mut harness, egui::PointerButton::Secondary, centre);
    harness.get_by_label("To point").click();
    harness.run_steps(2);
    let before = triangle(harness.state());
    harness.key_press(egui::Key::Escape);
    harness.run_steps(2);
    assert!(!harness.state().compass_picking());
    assert!(
        harness.state().compass_shown(),
        "the first Escape only cancels the pick"
    );
    assert_eq!(triangle(harness.state()), before);
}

#[test]
fn an_arrow_moves_only_along_its_axis_to_the_point() {
    let mut app = on_triangle();
    app.start_compass_pick(CompassPart::X, super::super::compass::PickBy::Point);
    // Alt: no snapping; (100, 90) is 25 left of the centre (125, 80) and 10 above it.
    let at = app.view.to_screen((100.0, 90.0));
    app.click_compass_pick(at, true);
    assert!(
        near(triangle(&app)[0], [85.0, 65.0]),
        "{:?}",
        triangle(&app)
    );
    assert_eq!(app.compass_now().unwrap().0, (100.0, 80.0));
}

#[test]
fn to_centre_point_takes_the_clicked_shapes_centre() {
    let mut app = on_triangle();
    app.start_compass_pick(CompassPart::Y, super::super::compass::PickBy::Centre);
    // Off any shape: it says so and waits.
    let empty = app.view.to_screen((200.0, 200.0));
    app.click_compass_pick(empty, false);
    assert!(app.compass_picking());
    // On the circle S2's line: its centre (75, 80), the same height as the Compass, so the
    // Y arrow moves nothing; on the X arrow, 50 left.
    let r = match &app.shape(2).unwrap().geometry {
        Geometry::Circle { radius, .. } => *radius,
        other => panic!("{other:?}"),
    };
    let on_line = app.view.to_screen((75.0 + r, 80.0));
    app.click_compass_pick(on_line, false);
    assert!(!app.compass_picking());
    assert!(near(triangle(&app)[0], [110.0, 65.0]));
    app.start_compass_pick(CompassPart::X, super::super::compass::PickBy::Centre);
    app.click_compass_pick(on_line, false);
    assert!(
        near(triangle(&app)[0], [60.0, 65.0]),
        "{:?}",
        triangle(&app)
    );
}

#[test]
fn unlocked_a_pick_moves_the_compass_alone_the_same_way() {
    let mut app = drawn_part();
    app.click(Some(contour_of(&app, 3)), false);
    let mut harness = window(app);
    harness.key_press(egui::Key::Q);
    harness.run_steps(2);
    harness.key_down(egui::Key::Space);
    harness.run_steps(1);
    harness.key_up(egui::Key::Space);
    harness.run_steps(2);
    assert!(harness.state_mut().compass_now().unwrap().2, "unlocked");
    let before = triangle(harness.state());
    // Right-click the X arrow: To point, then the circle's centre (75, 80) 10 px up: along
    // X only, the Compass goes from (125, 80) to (75, 80).
    let arm = centre_on_screen(&mut harness) + egui::vec2(45.0, 0.0);
    click_with(&mut harness, egui::PointerButton::Secondary, arm);
    assert!(harness.state().compass_pick_menu_open());
    harness.get_by_label("To point").click();
    harness.run_steps(2);
    let above = harness.state().view.to_screen((75.0, 80.0)) + egui::vec2(0.0, -10.0);
    harness.state_mut().click_compass_pick(above, true);
    let (centre, angle, unlocked) = harness.state_mut().compass_now().unwrap();
    assert!(unlocked);
    assert!(
        (centre.0 - 75.0).abs() < 1e-4 && (centre.1 - 80.0).abs() < 1e-4,
        "{centre:?}"
    );
    assert_eq!(angle, 0.0, "not turned");
    assert_eq!(triangle(harness.state()), before, "the selection stays");
    // The centre: onto the triangle's own corner (110, 65), in X and Y.
    harness
        .state_mut()
        .start_compass_pick(CompassPart::Centre, super::super::compass::PickBy::Point);
    let corner = harness.state().view.to_screen((110.0, 65.0)) + egui::vec2(1.0, 1.0);
    harness.state_mut().click_compass_pick(corner, false);
    assert_eq!(harness.state_mut().compass_now().unwrap().0, (110.0, 65.0));
    assert_eq!(triangle(harness.state()), before);
}
