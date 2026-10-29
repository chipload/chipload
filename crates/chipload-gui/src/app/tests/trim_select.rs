// SPDX-License-Identifier: GPL-3.0-or-later
//! A circle trimmed by a line, then deselected (the user, 2026-10-29).
use super::modify::ModifyTool;
use super::*;
use chipload_job::Geometry;
use egui_kittest::Harness;

#[test]
fn a_trimmed_circle_can_be_deselected() {
    let mut h = Harness::builder()
        .with_size(egui::vec2(1200.0, 800.0))
        .build_eframe(|_| {
            let mut app = drawn_part();
            let circle = app
                .add_drawn(Geometry::Circle {
                    center: [300.0, 100.0],
                    radius: 20.0,
                })
                .unwrap();
            app.commit();
            app.add_drawn(Geometry::straight_polyline(
                false,
                vec![[270.0, 100.0], [330.0, 100.0]],
            ))
            .unwrap();
            app.commit();
            app.selection = CanvasSelection::Nothing;
            let _ = circle;
            app.fit_pending = true;
            app
        });
    h.run_steps(6);
    h.state_mut().start_modify(ModifyTool::Trim);
    h.run_steps(2);
    // Click the circle's top half: it goes.
    let top = h.state().view.to_screen((300.0, 120.0));
    h.hover_at(top);
    h.run_steps(2);
    h.drag_at(top);
    h.run_steps(1);
    h.drop_at(top);
    h.run_steps(4);
    assert_eq!(h.state().selection, CanvasSelection::Shapes([4].into()));
    // Still trimming: a click on empty space clears the selection.
    let empty = h.state().view.to_screen((300.0, 160.0));
    h.hover_at(empty);
    h.run_steps(2);
    h.drag_at(empty);
    h.run_steps(1);
    h.drop_at(empty);
    h.run_steps(3);
    assert!(h.state().modify.is_some(), "still trimming");
    assert_eq!(h.state().selection, CanvasSelection::Nothing);
}
