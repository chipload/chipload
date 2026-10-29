// SPDX-License-Identifier: GPL-3.0-or-later
//! Shapes trimmed by a line, then deselected and the tool ended (the user, 2026-10-29).
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

/// A slot cut in half by a line, the line's ends trimmed too, then a click in open space
/// (the user, 2026-10-29: the pieces stayed two polylines, and open space didn't end the
/// tool).
#[test]
fn a_trimmed_slot_can_be_deselected() {
    let mut h = Harness::builder()
        .with_size(egui::vec2(1200.0, 800.0))
        .build_eframe(|_| {
            let mut app = drawn_part();
            app.add_drawn(Geometry::Rectangle {
                center: [300.0, 100.0],
                length: 80.0,
                width: 20.0,
                angle: 0.0,
                corner_radius: 10.0,
            })
            .unwrap();
            app.commit();
            app.add_drawn(Geometry::straight_polyline(
                false,
                vec![[300.0, 70.0], [300.0, 130.0]],
            ))
            .unwrap();
            app.commit();
            app.selection = CanvasSelection::Nothing;
            app.fit_pending = true;
            app
        });
    h.run_steps(6);
    h.state_mut().start_modify(ModifyTool::Trim);
    h.run_steps(2);
    let click = |h: &mut Harness<'_, ChiploadApp>, at: (f64, f64)| {
        let p = h.state().view.to_screen(at);
        h.hover_at(p);
        h.run_steps(2);
        h.drag_at(p);
        h.run_steps(1);
        h.drop_at(p);
        h.run_steps(4);
    };
    // The slot's right half, then the line's two ends outside it.
    click(&mut h, (340.0, 100.0));
    click(&mut h, (300.0, 75.0));
    click(&mut h, (300.0, 125.0));
    // The half slot and the line between its ends are one closed polyline now.
    let edit = h.state().edit.as_ref().unwrap();
    let drawn: Vec<&chipload_job::Shape> = edit
        .settings
        .drawing
        .shapes
        .iter()
        .filter(|s| s.is_drawn() && s.id >= 4)
        .collect();
    assert_eq!(drawn.len(), 1, "{drawn:?}");
    assert!(
        matches!(drawn[0].geometry, Geometry::Polyline { closed: true, .. }),
        "{drawn:?}"
    );
    assert_eq!(
        h.state().selection,
        CanvasSelection::Shapes([drawn[0].id].into())
    );
    // A click in open space clears the selection, the next ends the tool.
    click(&mut h, (300.0, 160.0));
    assert!(h.state().modify.is_some(), "still trimming");
    assert_eq!(h.state().selection, CanvasSelection::Nothing);
    click(&mut h, (300.0, 160.0));
    assert!(h.state().modify.is_none(), "the tool ended");
}
