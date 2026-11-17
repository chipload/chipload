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
    others(&app.edit.as_ref().unwrap().job, 1)[0]
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
    assert_eq!(others(job, 1).len(), 1);
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
    assert_eq!(others(job, 1)[1].mirror, Some(Mirror::LeftRight));
    assert_eq!(app.focus, Focus::Instance(1, 3));
    // Each is one undo step.
    app.undo();
    app.undo();
    assert!(others(&app.edit.as_ref().unwrap().job, 1).is_empty());
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
    assert!(others(&app.edit.as_ref().unwrap().job, 1).is_empty());
    assert_eq!(app.focus, Focus::Part(1));
}

#[test]
fn a_problem_in_an_instance_marks_its_form() {
    let mut app = drawn_part();
    app.add_instance(1, false);
    app.edit
        .as_mut()
        .unwrap()
        .job
        .instance_mut(1, 2)
        .unwrap()
        .at[0] = f64::NAN;
    app.commit();
    let fields = app.form_problem_map(Focus::Instance(1, 2));
    assert!(fields.contains_key("instances.1.at.0"), "{fields:?}");
    draw_properties(&mut app);
}

// -- The part itself placed (docs/specs/part-placement.md §6)

fn own(app: &ChiploadApp) -> ([f64; 2], f64) {
    let own = app.edit.as_ref().unwrap().job.instance(1, 1).unwrap();
    (own.at, own.angle)
}

#[test]
fn the_part_itself_is_placed_shown_where_it_is_cut_and_reset() {
    let mut app = drawn_part();
    draw_canvas(&mut app);
    assert!(app.scene.instances.is_empty() && app.scene.elsewhere.is_empty());
    // *Place…* opens its form: instance 1.
    app.place_part(1);
    assert_eq!(app.focus, Focus::Instance(1, 1));
    draw_properties(&mut app);
    let mut placed = app.instance_value(1, 1).unwrap();
    placed.at = [300.0, 0.0];
    assert!(app.set_instance_value(1, 1, placed));
    app.commit();
    assert_eq!(own(&app), ([300.0, 0.0], 0.0));
    // A ghost #1 where it is cut, and a dashed box round its shapes, not cut there.
    draw_canvas(&mut app);
    let ghost = app.scene.instances.iter().find(|g| g.id == 1).unwrap();
    assert!(ghost.label == "#1" && ghost.selected);
    let ((x0, _), _) = app.edit.as_ref().unwrap().job.part_box(1).unwrap();
    assert!(ghost.outlines.iter().flatten().all(|p| p.0 > x0 + 250.0));
    assert_eq!(app.scene.elsewhere.len(), 1);
    assert!(app.scene.elsewhere[0].1.ends_with("not cut here"));
    // Turned a quarter from its menu.
    app.turn_instance(1, 1);
    assert_eq!(own(&app).1, FRAC_PI_2);
    // *Reset placement*: back where drawn, as one undo step.
    app.reset_part_placement(1);
    assert_eq!(own(&app), ([0.0, 0.0], 0.0));
    draw_canvas(&mut app);
    // Still shown while selected, but cut where drawn: no "not cut here".
    assert!(app.scene.elsewhere.is_empty());
    assert!(app.scene.instances.iter().any(|g| g.id == 1));
    // Selected elsewhere: cut where drawn, it is its shapes, no ghost.
    app.focus = Focus::Part(1);
    draw_canvas(&mut app);
    assert!(app.scene.instances.is_empty());
    app.undo();
    assert_eq!(own(&app), ([300.0, 0.0], FRAC_PI_2));
    // Like any instance (docs/specs/parts-and-setups.md §6): mirrored, off alone, removed.
    app.mirror_instance(1, 1);
    assert!(app.instance_value(1, 1).unwrap().mirror.is_some());
    app.toggle_instance(1, 1);
    let job = &app.edit.as_ref().unwrap().job;
    assert!(job.parts[0].enabled && !job.instance(1, 1).unwrap().enabled);
    app.remove_instance(1, 1);
    let job = &app.edit.as_ref().unwrap().job;
    assert!(job.instances_of(1).is_empty());
    // Placed nowhere: not cut, listed as such.
    assert!(!job.cuts(0));
    draw_sidebar(&mut app);
}

/// A part cut twice lists the setups: its instances under *Setup 1*, named with the part,
/// not under the part (docs/specs/parts-and-setups.md §2a).
#[test]
fn the_instances_are_listed_under_their_setup() {
    let mut h = harness();
    h.state_mut().focus = Focus::Part(1);
    h.run_steps(4);
    use egui_kittest::kittest::Queryable;
    assert!(h.query_by_label("Part 1 #1").is_some());
    assert!(h.query_by_label("Part 1 #2").is_some());
    assert!(h.query_by_label("Instance 1").is_none());
}

#[test]
fn the_compass_moves_and_turns_an_instance() {
    let mut app = drawn_part();
    app.add_instance(1, false);
    draw_canvas(&mut app);
    let start = instance(&app);
    let centre = app.scene.instances[0].centre;
    // Q on the selected instance: the Compass sits on its centre.
    app.toggle_compass();
    let (at, angle, _) = app.compass_now().unwrap();
    assert_eq!((at, angle), (centre, 0.0));
    // The ring a quarter turn: turned about the Compass's centre, which is its own.
    let grab = app.view.to_screen((centre.0 + 20.0, centre.1));
    assert!(app.start_compass_drag(super::super::compass::CompassPart::Ring, grab, false));
    app.drag_compass_to((centre.0, centre.1 + 20.0), false);
    app.end_compass_drag(None);
    let turned = instance(&app);
    assert!((turned.angle - FRAC_PI_2).abs() < 1e-12, "{turned:?}");
    assert!(
        (turned.at[0] - start.at[0]).abs() < 1e-9 && (turned.at[1] - start.at[1]).abs() < 1e-9,
        "turned in place: {turned:?} vs {start:?}"
    );
    // A right drag (copy, array) is not for instances.
    assert!(!app.start_compass_drag(super::super::compass::CompassPart::X, grab, true));
    app.undo();
    assert_eq!(instance(&app), start);
}

/// A Compass drag on an instance in the window: the Compass takes the press over the ghost
/// under it, the project stays as it is until the drop (so nothing is planned again
/// mid-drag), and the preview is drawn afresh each frame, never piled up.
#[test]
fn a_compass_drag_on_an_instance_previews_without_trails() {
    use super::super::compass::CompassPart;
    for (part, offset) in [
        (
            CompassPart::Ring,
            egui::vec2(crate::view::COMPASS_RING_PX, 0.0),
        ),
        (CompassPart::Y, egui::vec2(0.0, -45.0)),
    ] {
        let mut h = harness();
        h.state_mut().focus = Focus::Instance(1, 2);
        h.run_steps(3);
        h.state_mut().toggle_compass();
        h.run_steps(3);
        let start = instance(h.state());
        let (c, _, _) = h.state_mut().compass_now().unwrap();
        let from = h.state().view.to_screen(c) + offset;
        assert_eq!(h.state_mut().compass_part_at(from), Some(part));
        h.hover_at(from);
        h.run_steps(2);
        h.drag_at(from);
        h.run_steps(2);
        let mut sizes = Vec::new();
        for k in 1..=6 {
            h.hover_at(from + egui::vec2(8.0 * k as f32, 8.0 * k as f32));
            h.run_steps(2);
            let app = h.state();
            assert!(
                app.dragging_compass() && !app.dragging_instance(),
                "{part:?}"
            );
            assert_eq!(instance(app), start, "{part:?}: unchanged until the drop");
            sizes.push(app.scene.preview.len());
        }
        assert!(sizes[0] > 0, "{part:?}: {sizes:?}");
        assert!(sizes.iter().all(|n| *n == sizes[0]), "{part:?}: {sizes:?}");
        h.drop_at(from + egui::vec2(48.0, 48.0));
        h.run_steps(3);
        assert_ne!(instance(h.state()), start, "{part:?}: moved on the drop");
        assert!(h.state().scene.preview.is_empty(), "{part:?}");
    }
}

/// *Place…* on a part cut once where drawn (docs/specs/part-placement.md §6): the setups
/// listed, its row under *Setup 1* and ghost #1 where it is drawn, selected with its turn handle; Q shows the
/// Compass on it; a drag on its outline moves it, though the part's shapes lie under it.
#[test]
fn the_part_itself_is_shown_to_place_without_instances() {
    let mut h = Harness::builder()
        .with_size(egui::vec2(1200.0, 800.0))
        .build_eframe(|_| {
            let mut app = ChiploadApp::default();
            assert!(app.open(&corpus("jobs/v5/drawn_part.chipload")));
            app.fit_pending = true;
            app
        });
    h.run_steps(6);
    assert!(h.state().scene.instances.is_empty());
    h.state_mut().place_part(1);
    h.run_steps(4);
    {
        use egui_kittest::kittest::Queryable;
        assert!(h.query_by_label("Part 1 #1").is_some());
    }
    let ghost = h.state().scene.instances[0].clone();
    assert!(ghost.id == 1 && ghost.selected && ghost.handle.is_some());
    // Where the part is drawn: on its shapes.
    let ((x0, y0), (x1, y1)) = h.state().edit.as_ref().unwrap().job.part_box(1).unwrap();
    let (cx, cy) = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);
    assert!((ghost.centre.0 - cx).abs() < 1e-9 && (ghost.centre.1 - cy).abs() < 1e-9);
    // Q: the Compass on it.
    h.state_mut().toggle_compass();
    h.run_steps(2);
    assert_eq!(h.state_mut().compass_now().map(|c| c.0), Some(ghost.centre));
    h.state_mut().hide_compass();
    h.run_steps(2);
    // Dragged by its outline, over the shapes: it moves; the shapes stay.
    let shapes = h.state().edit.as_ref().unwrap().job.drawing.shapes.clone();
    let on = screen(&h, on_ghost(&h));
    drag(&mut h, on, on + egui::vec2(60.0, 0.0), false);
    let (at, _) = own(h.state());
    let scale = h.state().view.scale;
    assert!((at[0] - 60.0 / scale).abs() < 1.0 / scale, "{at:?}");
    assert_eq!(h.state().edit.as_ref().unwrap().job.drawing.shapes, shapes);
}

/// The user's finding (2026-11-17): no clear handles. Each ghost has a badge, its number
/// with a move mark above its top, outside the shapes: hovered it lights up, dragged it
/// moves the instance, unselected too, and instance 1 though the part's shapes lie there.
#[test]
fn an_instance_is_moved_by_its_badge() {
    let mut h = harness();
    let badge = |h: &Harness<'_, ChiploadApp>, id: u32| {
        let state = h.state();
        let ghost = state.scene.instances.iter().find(|g| g.id == id).unwrap();
        crate::view::badge_rect(&state.view, ghost).center()
    };
    assert_eq!(h.state().focus, Focus::Job, "not selected");
    // Hovered: lit up, nothing else.
    h.hover_at(badge(&h, 2));
    h.run_steps(3);
    let ghost = h.state().scene.instances[0].clone();
    assert!(ghost.hot && !ghost.selected, "{ghost:?}");
    // Dragged by it: moved and selected.
    let start = instance(h.state());
    let from = badge(&h, 2);
    drag(&mut h, from, from + egui::vec2(50.0, -30.0), false);
    let moved = instance(h.state());
    let scale = h.state().view.scale;
    assert!(
        (moved.at[0] - start.at[0] - 50.0 / scale).abs() < 1.0 / scale,
        "{moved:?}"
    );
    assert!(
        (moved.at[1] - start.at[1] - 30.0 / scale).abs() < 1.0 / scale,
        "{moved:?}"
    );
    assert_eq!(h.state().focus, Focus::Instance(1, 2));
    // Instance 1 selected with *Place…*: its ghost on the part as drawn, its badge takes
    // it, and the shapes stay.
    h.state_mut().place_part(1);
    h.run_steps(3);
    let shapes = h.state().edit.as_ref().unwrap().job.drawing.shapes.clone();
    let from = badge(&h, 1);
    drag(&mut h, from, from + egui::vec2(-40.0, 0.0), false);
    let (at, _) = own(h.state());
    assert!((at[0] + 40.0 / scale).abs() < 1.0 / scale, "{at:?}");
    assert_eq!(h.state().edit.as_ref().unwrap().job.drawing.shapes, shapes);
    assert_eq!(h.state().focus, Focus::Instance(1, 1));
}

/// The handles as drawn, for a look by eye: instance 1 shown, instance 2 selected with its
/// turn handle, instance 3 hovered.
/// `CHIPLOAD_PICTURES=dir cargo test -p chipload-gui pictures -- --ignored`.
#[test]
#[ignore = "writes pictures; needs CHIPLOAD_PICTURES"]
fn instance_handles_pictures() {
    let Ok(dir) = std::env::var("CHIPLOAD_PICTURES") else {
        panic!("set CHIPLOAD_PICTURES to the folder to write to");
    };
    let dir = std::path::PathBuf::from(dir);
    std::fs::create_dir_all(&dir).unwrap();
    let mut h = Harness::builder()
        .with_size(egui::vec2(1200.0, 800.0))
        .wgpu()
        .build_eframe(|cc| {
            let mut fonts = crate::fonts::ui_fonts();
            crate::fonts::add_text_fonts(&mut fonts);
            cc.egui_ctx.set_fonts(fonts);
            crate::style::apply(&cc.egui_ctx);
            let mut app = ChiploadApp::default();
            assert!(app.open(&corpus("jobs/v5/drawn_part.chipload")));
            app.add_instance(1, false);
            app.add_instance(1, true);
            app.place_part(1);
            app.focus = Focus::Instance(1, 2);
            app.fit_pending = true;
            app
        });
    h.run_steps(6);
    let hover = {
        let state = h.state();
        let ghost = state.scene.instances.iter().find(|g| g.id == 3).unwrap();
        crate::view::badge_rect(&state.view, ghost).center()
    };
    h.hover_at(hover);
    h.run_steps(4);
    let image = h.render().unwrap();
    image.save(dir.join("instance-handles.png")).unwrap();
}
