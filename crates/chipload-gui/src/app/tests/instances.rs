// SPDX-License-Identifier: GPL-3.0-or-later
//! A part's instances in the window (docs/specs/instances.md §4): added from the part's
//! menu, drawn as numbered ghosts, picked with a click, moved by a drag, turned by their
//! handle, changed in their form and the Browser.
use super::*;
use chipload_job::{Instance, Mirror};
use egui_kittest::Harness;
use std::f64::consts::FRAC_PI_2;

fn harness() -> Harness<'static, ChiploadApp> {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1200.0, 800.0))
        .build_eframe(|_| {
            let mut app = ChiploadApp::default();
            assert!(app.open(&corpus("jobs/v5/drawn_part.chipload")));
            app.add_instance(1, false);
            app.focus = Focus::Job;
            app.fit_pending = true;
            app
        });
    harness.run_steps(6);
    harness
}

fn instance(app: &ChiploadApp) -> Instance {
    app.edit.as_ref().unwrap().job.parts[0].instances[0]
}

fn screen(h: &Harness<'_, ChiploadApp>, p: (f64, f64)) -> egui::Pos2 {
    h.state().view.to_screen(p)
}

/// A point on the ghost's first outline, halfway along its first line.
fn on_ghost(h: &Harness<'_, ChiploadApp>) -> (f64, f64) {
    let ghost = &h.state().scene.instances[0];
    let (a, b) = (ghost.outlines[0][0], ghost.outlines[0][1]);
    ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0)
}

fn click_at(h: &mut Harness<'_, ChiploadApp>, pos: egui::Pos2) {
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
    }
    h.run_steps(3);
}

/// Press at `from`, move to `to` in steps, and let go there unless `escape` is pressed
/// first.
fn drag(h: &mut Harness<'_, ChiploadApp>, from: egui::Pos2, to: egui::Pos2, escape: bool) {
    h.hover_at(from);
    h.run_steps(2);
    h.drag_at(from);
    h.run_steps(2);
    for k in 1..=5 {
        h.hover_at(from + (to - from) * (k as f32 / 5.0));
        h.run_steps(2);
    }
    if escape {
        h.key_press(egui::Key::Escape);
        h.run_steps(2);
    }
    h.drop_at(to);
    h.run_steps(3);
}

#[test]
fn an_instance_is_added_beside_the_part_drawn_numbered_and_undone() {
    let mut app = drawn_part();
    app.add_instance(1, false);
    let job = &app.edit.as_ref().unwrap().job;
    assert_eq!(job.parts[0].instances.len(), 1);
    assert_eq!(app.focus, Focus::Instance(1, 2));
    draw_canvas(&mut app);
    let ghost = &app.scene.instances[0];
    assert_eq!(ghost.label, "#2");
    assert!(ghost.selected && ghost.handle.is_some());
    // Beside the part: right of everything it draws.
    let right = app
        .contours
        .iter()
        .flat_map(|c| c.points())
        .map(|p| p.0)
        .fold(f64::MIN, f64::max);
    let left = ghost
        .outlines
        .iter()
        .flatten()
        .map(|p| p.0)
        .fold(f64::MAX, f64::min);
    assert!(left > right, "{left} vs {right}");
    // A mirrored one goes beside both.
    app.add_instance(1, true);
    let job = &app.edit.as_ref().unwrap().job;
    assert_eq!(job.parts[0].instances[1].mirror, Some(Mirror::LeftRight));
    assert_eq!(app.focus, Focus::Instance(1, 3));
    // Each is one undo step.
    app.undo();
    app.undo();
    assert!(app.edit.as_ref().unwrap().job.parts[0].instances.is_empty());
    draw_canvas(&mut app);
    assert!(app.scene.instances.is_empty());
}

#[test]
fn an_instance_is_picked_moved_and_turned_on_the_canvas() {
    let mut h = harness();
    let start = instance(h.state());
    assert_ne!(h.state().focus, Focus::Instance(1, 2));
    // A click on its ghost selects it.
    let on = screen(&h, on_ghost(&h));
    click_at(&mut h, on);
    assert_eq!(h.state().focus, Focus::Instance(1, 2));
    // Dragged by its outline: moved as far as the pointer, one undo step.
    let to = on + egui::vec2(40.0, -20.0);
    drag(&mut h, on, to, false);
    let moved = instance(h.state());
    let scale = h.state().view.scale;
    assert!(
        (moved.at[0] - start.at[0] - 40.0 / scale).abs() < 1.0 / scale,
        "{moved:?}"
    );
    assert!(
        (moved.at[1] - start.at[1] - 20.0 / scale).abs() < 1.0 / scale,
        "{moved:?}"
    );
    h.state_mut().undo();
    assert_eq!(instance(h.state()), start);
    // Escape during a drag: back where it was.
    h.run_steps(3);
    let on = screen(&h, on_ghost(&h));
    drag(&mut h, on, on + egui::vec2(40.0, 0.0), true);
    assert_eq!(instance(h.state()), start, "escape");
    // Its handle turns it: a little past a quarter turn snaps to the quarter.
    h.state_mut().focus = Focus::Instance(1, 2);
    h.run_steps(3);
    let ghost = h.state().scene.instances[0].clone();
    let (centre, handle) = (ghost.centre, ghost.handle.unwrap());
    let r = ((handle.0 - centre.0).powi(2) + (handle.1 - centre.1).powi(2)).sqrt();
    // From straight up round to 2° past straight left.
    let a = std::f64::consts::PI + 2.0_f64.to_radians();
    let target = (
        centre.0 + r * chipload_core::math::cos(a),
        centre.1 + r * chipload_core::math::sin(a),
    );
    let (from, to) = (screen(&h, handle), screen(&h, target));
    drag(&mut h, from, to, false);
    let turned = instance(h.state());
    assert_eq!(turned.angle, FRAC_PI_2, "{turned:?}");
    assert_eq!(turned.at, start.at, "turned about its centre, not moved");
}

#[test]
fn an_instance_is_changed_from_the_browser_menu_and_removed_with_delete() {
    let mut app = drawn_part();
    app.add_instance(1, false);
    draw_sidebar(&mut app);
    draw_properties(&mut app);
    app.mirror_instance(1, 2);
    app.turn_instance(1, 2);
    app.turn_instance(1, 2);
    app.toggle_instance(1, 2);
    let i = instance(&app);
    assert_eq!(i.mirror, Some(Mirror::LeftRight));
    assert!((i.angle - std::f64::consts::PI).abs() < 1e-12, "{i:?}");
    assert!(!i.enabled);
    // The part's form and the Browser still draw; the instance is drawn dashed.
    draw_canvas(&mut app);
    assert!(!app.scene.instances[0].enabled);
    draw_sidebar(&mut app);
    draw_properties(&mut app);
    app.focus = Focus::Instance(1, 2);
    app.delete_key();
    assert!(app.edit.as_ref().unwrap().job.parts[0].instances.is_empty());
    assert_eq!(app.focus, Focus::Part(1));
}

#[test]
fn a_problem_in_an_instance_marks_its_form() {
    let mut app = drawn_part();
    app.add_instance(1, false);
    app.edit.as_mut().unwrap().job.parts[0].instances[0].at[0] = f64::NAN;
    app.commit();
    let fields = app.form_problem_map(Focus::Instance(1, 2));
    assert!(
        fields.contains_key("parts.0.instances.0.at.0"),
        "{fields:?}"
    );
    draw_properties(&mut app);
}
