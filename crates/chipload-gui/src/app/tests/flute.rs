// SPDX-License-Identifier: GPL-3.0-or-later
//! The flute in the window (docs/specs/fluting.md §8): S then L adds one for open shapes,
//! closed ones grey it, and its properties show the ramps and the width at the top.

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;

use super::*;

/// The golden flute project, its operations removed.
fn flute_app() -> ChiploadApp {
    let mut app = ChiploadApp::default();
    assert!(app.open(&corpus("jobs/v36/flute.chipload")));
    let edit = app.edit.as_mut().unwrap();
    while !edit.job.operations.is_empty() {
        edit.remove(0);
    }
    app
}

#[test]
fn s_then_l_adds_a_flute_with_the_ball_nose_and_it_generates() {
    let mut app = flute_app();
    let open: Vec<usize> = app.contours.iter().map(|c| c.id).collect();
    select(&mut app, &open);
    assert!(app.quick_unfit(OpKind::Flute).is_none());
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1200.0, 800.0))
        .build_eframe(move |cc| {
            cc.egui_ctx.set_fonts(crate::fonts::ui_fonts());
            app
        });
    harness.run_steps(2);
    harness.ctx.memory_mut(|m| m.stop_text_input());
    harness.run_steps(1);
    harness.key_press(egui::Key::S);
    harness.run_steps(2);
    harness.key_press(egui::Key::L);
    harness.run_steps(2);
    let app = harness.state_mut();
    let Operation::Flute(f) = app.edit.as_ref().unwrap().job.operations[0].clone() else {
        panic!("a flute");
    };
    assert_eq!(f.name, "Flute 1");
    assert_eq!(f.tool, 1, "the ball nose");
    assert_eq!(f.end_depth, 3.0, "half-round: the ball's radius");
    assert_eq!((f.ramp_start, f.ramp_end), (20.0, 20.0));
    assert_eq!(f.select.shapes.as_ref().map(Vec::len), Some(3));
    assert!(app.generate_and_wait(), "{:?}", app.report);
}

#[test]
fn closed_shapes_alone_grey_it() {
    let mut app = flute_app();
    let edit = app.edit.as_mut().unwrap();
    edit.job.drawing.shapes.push(
        serde_json::from_value(serde_json::json!({"id": 9, "layer": "drawn",
            "kind": "circle", "center": [200.0, 100.0], "radius": 10.0}))
        .unwrap(),
    );
    app.refresh_drawing();
    let circle = app.contours.iter().find(|c| c.closed).unwrap().id;
    select(&mut app, &[circle]);
    let why = app.quick_unfit(OpKind::Flute).unwrap();
    assert!(why.contains("open shape"), "{why}");
}

#[test]
fn the_form_shows_the_ramps_and_the_width() {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1200.0, 900.0))
        .build_eframe(|cc| {
            open_sections(&cc.egui_ctx);
            let mut app = ChiploadApp::default();
            assert!(app.open(&corpus("jobs/v36/flute.chipload")));
            app.focus = Focus::Op(0);
            app
        });
    harness.run_steps(4);
    harness.get_by_label("Start ramp");
    harness.get_by_label("End ramp");
    harness.get_by_label("6 mm wide at the top");
    harness.get_by_label("Cut   3 mm deep, ramps 25 mm / 25 mm, straight");
}

/// The window with the golden flutes selected, the first flute's form and the S menu open,
/// rendered (wgpu) to `CHIPLOAD_SHOTS_OUT`; looked at by hand.
#[test]
#[ignore = "a picture for looking at the flute"]
fn flute_picture() {
    let mut app = ChiploadApp::default();
    assert!(app.open(&corpus("jobs/v36/flute.chipload")));
    let open: Vec<usize> = app.contours.iter().map(|c| c.id).collect();
    select(&mut app, &open);
    app.focus = Focus::Op(0);
    let mut app = Some(app);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1400.0, 900.0))
        .wgpu()
        .build_eframe(move |cc| {
            cc.egui_ctx.set_fonts(crate::fonts::ui_fonts());
            crate::style::apply(&cc.egui_ctx);
            open_sections(&cc.egui_ctx);
            app.take().unwrap()
        });
    harness.run_steps(4);
    harness.ctx.memory_mut(|m| m.stop_text_input());
    harness.state_mut().quick_menu = Some(egui::pos2(700.0, 450.0));
    harness.run_steps(4);
    let out = std::env::var("CHIPLOAD_SHOTS_OUT").unwrap_or_else(|_| ".".into());
    harness
        .render()
        .unwrap()
        .save(std::path::PathBuf::from(out).join("flute.png"))
        .unwrap();
}
