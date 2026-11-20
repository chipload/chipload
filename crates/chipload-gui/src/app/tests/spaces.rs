// SPDX-License-Identifier: GPL-3.0-or-later
//! The drawing space and a space per setup (docs/specs/drawing-and-setup-spaces.md): what is
//! selected picks the tab, each space shows its own things, each keeps its view.
use super::*;
use crate::app::display::Space;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;

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
