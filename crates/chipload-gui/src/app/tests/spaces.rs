// SPDX-License-Identifier: GPL-3.0-or-later
//! The drawing space and a space per setup (docs/specs/drawing-and-setup-spaces.md): what is
//! selected picks the tab, each space shows its own things, each keeps its view.
use super::*;
use crate::app::display::Space;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use std::f64::consts::FRAC_PI_2;

fn select(app: &mut ChiploadApp, focus: Focus) {
    app.focus = focus;
    app.follow_focus();
}

#[test]
fn what_is_selected_picks_the_space() {
    let mut app = drawn_part();
    // A project opens on its drawing.
    assert_eq!(app.space(), Space::Drawing);
    app.add_instance(1, false);
    // An instance: its setup's space.
    app.follow_focus();
    assert_eq!(app.space(), Space::Setup);
    select(&mut app, Focus::Part(1));
    assert_eq!(app.space(), Space::Drawing);
    select(&mut app, Focus::Stock);
    assert_eq!(app.space(), Space::Setup);
    // The project and the tools leave it as it is.
    select(&mut app, Focus::Job);
    assert_eq!(app.space(), Space::Setup);
    select(&mut app, Focus::Tools);
    assert_eq!(app.space(), Space::Setup);
    // A shape picked: the drawing.
    app.click(Some(contour_of(&app, 3)), false);
    assert_eq!(app.space(), Space::Drawing);
}

#[test]
fn each_space_shows_its_own_things() {
    let mut app = drawn_part();
    app.add_instance(1, false);
    select(&mut app, Focus::Part(1));
    draw_canvas(&mut app);
    // The drawing: the shapes, picked; no instance; the board faintly (cut where drawn).
    assert!(!app.scene.shapes_hidden && app.scene.instances.is_empty());
    assert!(app.scene.stock.is_some() && app.scene.stock_faint);
    let on_shape = app.view.to_screen(app.contours[0].points()[0]);
    assert!(app.scene.pick(&app.view, on_shape).is_some());
    // The setup's: every instance, #1 where drawn too; no shape picked or handled.
    show_setup(&mut app);
    app.click(None, false);
    draw_canvas(&mut app);
    assert!(app.scene.shapes_hidden && !app.scene.stock_faint);
    let ids: Vec<u32> = app.scene.instances.iter().map(|g| g.id).collect();
    assert_eq!(ids, [1, 2]);
    assert!(app.scene.instances.iter().all(|g| !g.label.is_empty()));
    assert!(app.scene.pick(&app.view, on_shape).is_none());
    assert!(app.shape_handles().is_empty());
}

#[test]
fn each_space_keeps_its_view_and_drawing_keys_go_to_the_drawing() {
    let mut app = drawn_part();
    app.add_instance(1, false);
    select(&mut app, Focus::Part(1));
    // As a frame's fit leaves them.
    app.fit_pending = false;
    app.view.scale = 3.0;
    app.view.offset = (10.0, 20.0);
    show_setup(&mut app);
    app.fit_pending = false;
    app.view.scale = 1.5;
    select(&mut app, Focus::Part(1));
    assert_eq!((app.view.scale, app.view.offset), (3.0, (10.0, 20.0)));
    show_setup(&mut app);
    assert_eq!(app.view.scale, 1.5);
    // A drawing tool from a setup's space: the drawing, its part selected.
    app.start_drawing(Tool::Circle);
    assert_eq!(app.space(), Space::Drawing);
    assert_eq!(app.focus, Focus::Part(1));
    assert!(app.sketch().is_some());
}

fn window() -> Harness<'static, ChiploadApp> {
    let mut h = Harness::builder()
        .with_size(egui::vec2(1200.0, 800.0))
        // Frames close enough together for two clicks to be a double-click.
        .with_step_dt(0.05)
        .build_eframe(|_| {
            let mut app = drawn_part();
            app.add_instance(1, false);
            app.focus = Focus::Part(1);
            app.fit_pending = true;
            app
        });
    h.run_steps(6);
    h
}

fn click(h: &mut Harness<'_, ChiploadApp>, at: egui::Pos2, double: bool) {
    h.hover_at(at);
    h.run_steps(1);
    for _ in 0..if double { 2 } else { 1 } {
        for pressed in [true, false] {
            h.event(egui::Event::PointerButton {
                pos: at,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            });
            h.run_steps(1);
        }
    }
    h.run_steps(2);
}

#[test]
fn the_tabs_switch_spaces_and_a_double_click_opens_an_instances_part() {
    let mut h = window();
    assert_eq!(h.state().space(), Space::Drawing);
    // The setup's tab (the canvas's, right of the Browser's row of that name).
    let tab = h
        .get_all_by_label("Setup 1")
        .map(|n| n.rect().center())
        .max_by(|a, b| a.x.total_cmp(&b.x))
        .unwrap();
    click(&mut h, tab, false);
    assert_eq!(h.state().space(), Space::Setup);
    assert_eq!(h.state().focus, Focus::Setup(1));
    // Its view fitted, the first time it is shown.
    h.run_steps(10);
    // Double-click instance 2's outline: its part, in the drawing.
    let ghost = h
        .state()
        .scene
        .instances
        .iter()
        .find(|g| g.id == 2)
        .unwrap()
        .clone();
    let (a, b) = (ghost.outlines[0][0], ghost.outlines[0][1]);
    let on = h
        .state()
        .view
        .to_screen(((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0));
    click(&mut h, on, true);
    assert_eq!(h.state().focus, Focus::Part(1));
    assert_eq!(h.state().space(), Space::Drawing);
    // *Drawing* is a tab too (clicks apart, not a double-click).
    h.run_steps(10);
    click(&mut h, tab, false);
    h.run_steps(10);
    // The tab beside it, not the Browser's *Drawing* folder.
    let drawing = h
        .get_all_by_label("Drawing")
        .map(|n| n.rect().center())
        .find(|p| (p.y - tab.y).abs() < 4.0)
        .unwrap();
    click(&mut h, drawing, false);
    assert_eq!(h.state().space(), Space::Drawing);
}

/// Press on `from`, move to `to` in steps (holding `hold` steps at `pause` on the way), and
/// let go there unless Escape is pressed first.
fn drag_row(
    h: &mut Harness<'_, ChiploadApp>,
    from: egui::Pos2,
    pause: Option<(egui::Pos2, usize)>,
    to: egui::Pos2,
    escape: bool,
) {
    h.hover_at(from);
    h.run_steps(2);
    h.drag_at(from);
    h.run_steps(2);
    let mut path = vec![];
    if let Some((at, hold)) = pause {
        for k in 1..=4 {
            path.push(from + (at - from) * (k as f32 / 4.0));
        }
        for _ in 0..hold {
            path.push(at);
        }
    }
    let start = path.last().copied().unwrap_or(from);
    for k in 1..=5 {
        path.push(start + (to - start) * (k as f32 / 5.0));
    }
    for p in path {
        h.hover_at(p);
        h.run_steps(1);
    }
    if escape {
        h.key_press(egui::Key::Escape);
        h.run_steps(2);
    }
    h.drop_at(to);
    h.run_steps(4);
}

/// The Browser's row of the part (left of anything else of that name).
fn part_row(h: &Harness<'_, ChiploadApp>) -> egui::Pos2 {
    h.get_all_by_label("Part 1")
        .map(|n| n.rect().center())
        .min_by(|a, b| a.x.total_cmp(&b.x))
        .unwrap()
}

fn setup_window() -> Harness<'static, ChiploadApp> {
    let mut h = Harness::builder()
        .with_size(egui::vec2(1200.0, 800.0))
        .with_step_dt(0.05)
        .build_eframe(|_| {
            let mut app = drawn_part();
            app.focus = Focus::Setup(1);
            app.fit_pending = true;
            app
        });
    h.run_steps(8);
    h
}

#[test]
fn a_part_dragged_onto_the_board_becomes_an_instance_there() {
    let mut h = setup_window();
    let target = (240.0, 170.0);
    let to = h.state().view.to_screen(target);
    let from = part_row(&h);
    drag_row(&mut h, from, None, to, false);
    let job = &h.state().edit.as_ref().unwrap().job;
    let made = others(job, 1);
    assert_eq!(made.len(), 1, "a new instance");
    let new = made[0];
    assert_eq!((new.angle, new.mirror), (0.0, None));
    // The part's middle under the pointer, on the grid's step.
    let centre = job.part_centre(1).unwrap();
    let (x, y) = (centre.0 + new.at[0], centre.1 + new.at[1]);
    assert!(
        (x - target.0).abs() <= 5.0 && (y - target.1).abs() <= 5.0,
        "{x}, {y}"
    );
    assert_eq!(h.state().focus, Focus::Instance(1, new.id));
    // One undo step.
    h.state_mut().undo();
    assert!(others(&h.state().edit.as_ref().unwrap().job, 1).is_empty());
    // Escape during the drag drops nothing.
    h.run_steps(4);
    let from = part_row(&h);
    drag_row(&mut h, from, None, to, true);
    assert!(others(&h.state().edit.as_ref().unwrap().job, 1).is_empty());
}

#[test]
fn a_part_held_over_a_setups_tab_opens_it_to_be_dropped() {
    let mut h = setup_window();
    h.state_mut().focus = Focus::Part(1);
    h.run_steps(8);
    assert_eq!(h.state().space(), Space::Drawing);
    let tab = h
        .get_all_by_label("Setup 1")
        .map(|n| n.rect().center())
        .max_by(|a, b| a.x.total_cmp(&b.x))
        .unwrap();
    let from = part_row(&h);
    let to = h.state().view.to_screen((240.0, 170.0));
    // Held over the tab for 0.75 s: it opens, and the drop lands on its board.
    drag_row(&mut h, from, Some((tab, 15)), to, false);
    assert_eq!(h.state().space(), Space::Setup);
    assert_eq!(others(&h.state().edit.as_ref().unwrap().job, 1).len(), 1);
}

#[test]
fn a_setup_in_pieces_refuses_a_second_instance_of_a_part() {
    let mut app = ChiploadApp::default();
    assert!(app.open(&corpus("jobs/v37/pieces.chipload")));
    let job = &app.edit.as_ref().unwrap().job;
    let part = job.parts[0].id;
    let before = job.instances_of(part).len();
    assert!(!app.drop_instance(part, None, [500.0, 0.0]));
    assert_eq!(
        app.edit.as_ref().unwrap().job.instances_of(part).len(),
        before
    );
    let why: Vec<&String> = app.report.iter().collect();
    assert!(why.iter().any(|l| l.contains("piece")), "{why:?}");
}

#[test]
fn reset_to_where_drawn_from_the_instances_menu() {
    let mut h = setup_window();
    h.state_mut().add_instance(1, true);
    h.state_mut().turn_instance(1, 2, true);
    h.run_steps(4);
    let ghost = h
        .state()
        .scene
        .instances
        .iter()
        .find(|g| g.id == 2)
        .unwrap()
        .clone();
    let (a, b) = (ghost.outlines[0][0], ghost.outlines[0][1]);
    let on = h
        .state()
        .view
        .to_screen(((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0));
    h.hover_at(on);
    h.run_steps(1);
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton {
            pos: on,
            button: egui::PointerButton::Secondary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
        h.run_steps(1);
    }
    h.run_steps(2);
    let item = h.get_by_label("Reset to where drawn").rect().center();
    click(&mut h, item, false);
    let now = h.state().instance_value(1, 2).unwrap();
    assert_eq!((now.at, now.angle), ([0.0, 0.0], 0.0));
    assert!(now.mirror.is_some(), "the mirror stays");
    // One undo step.
    h.state_mut().undo();
    assert_eq!(h.state().instance_value(1, 2).unwrap().angle, FRAC_PI_2);
}

#[test]
fn a_right_click_on_the_board_gives_the_setups_menu() {
    let mut h = setup_window();
    let right_click = |h: &mut Harness<'_, ChiploadApp>| {
        // Empty board, away from the part.
        let at = h.state().view.to_screen((250.0, 200.0));
        h.hover_at(at);
        h.run_steps(1);
        for pressed in [true, false] {
            h.event(egui::Event::PointerButton {
                pos: at,
                button: egui::PointerButton::Secondary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            });
            h.run_steps(1);
        }
        h.run_steps(2);
    };
    right_click(&mut h);
    let parts = h.state().edit.as_ref().unwrap().job.parts.len();
    let item = h.get_by_label("New part").rect().center();
    click(&mut h, item, false);
    assert_eq!(h.state().edit.as_ref().unwrap().job.parts.len(), parts + 1);
    // Nest… from the same menu opens the nest window for this setup.
    h.state_mut().focus = Focus::Setup(1);
    h.run_steps(10);
    right_click(&mut h);
    let item = h.get_by_label("Nest…").rect().center();
    click(&mut h, item, false);
    assert!(h.state().nest.is_some());
}

#[test]
fn the_boards_stay_where_they_are_when_another_setup_is_chosen() {
    let mut app = ChiploadApp::default();
    assert!(app.open(&corpus("jobs/v34/two_setups.chipload")));
    app.activate_setup(1);
    show_setup(&mut app);
    app.fit_pending = false;
    draw_canvas(&mut app);
    let on_screen = |app: &ChiploadApp, board: (Point, Point)| {
        (app.view.to_screen(board.0), app.view.to_screen(board.1))
    };
    let one = on_screen(&app, app.scene.stock.unwrap());
    let two = on_screen(&app, app.scene.sheets[0].board);
    // Setup 2 chosen from its tab: it is the active board now, setup 1 a sheet, and both
    // lie where they lay on screen.
    app.click_space_tab(Some(2));
    draw_canvas(&mut app);
    assert_eq!(app.edit.as_ref().unwrap().job.active_setup_id(), Some(2));
    let close = |a: (egui::Pos2, egui::Pos2), b: (egui::Pos2, egui::Pos2)| {
        a.0.distance(b.0) < 0.01 && a.1.distance(b.1) < 0.01
    };
    assert!(close(on_screen(&app, app.scene.stock.unwrap()), two));
    let sheet = app.scene.sheets.iter().find(|s| s.setup == 1).unwrap();
    assert!(close(on_screen(&app, sheet.board), one));
    // In the setups' order, left to right, whichever is active.
    assert!(sheet.board.1.0 < app.scene.stock.unwrap().0.0);
    // And back from the Browser: the same.
    app.activate_setup(1);
    draw_canvas(&mut app);
    assert!(close(on_screen(&app, app.scene.stock.unwrap()), one));
}
