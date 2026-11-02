// SPDX-License-Identifier: GPL-3.0-or-later
//! A DXF or SVG selected whole on the canvas, moved, turned and scaled by its box, carried
//! onto the shape library; a group's box turned as the group is
//! (docs/specs/dxf-whole-on-canvas.md).

use chipload_pipeline::box_handles::BoxHandle;
use chipload_pipeline::placement::Move;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;

use super::*;

fn window(project: &str, settings: Option<PathBuf>) -> Harness<'static, ChiploadApp> {
    let project = corpus(project);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1400.0, 900.0))
        .build_eframe(move |_| {
            let mut app = ChiploadApp::default();
            assert!(app.open(&project));
            app.user.settings_dir = settings;
            app
        });
    harness.run_steps(4);
    harness
}

/// A point on DXF `id`'s first contour, and that contour.
fn on_dxf(app: &ChiploadApp, id: u64) -> (usize, Point) {
    let contour = app
        .contours
        .iter()
        .find(|c| {
            c.parts
                .iter()
                .all(|p| p.segment.from == Some(chipload_core::chain::ShapeFrom::Dxf(id)))
        })
        .unwrap();
    (contour.id, contour.parts[0].segment.prim.start())
}

fn dxf_placement(app: &ChiploadApp) -> chipload_job::Placement {
    app.edit.as_ref().unwrap().job.drawing.dxfs[0].placement
}

/// `harness`'s pointer dragged from `from` to `to` (screen), in steps as a hand would.
fn drag(harness: &mut Harness<'_, ChiploadApp>, from: egui::Pos2, to: egui::Pos2) {
    harness.hover_at(from);
    harness.run_steps(2);
    harness.drag_at(from);
    harness.run_steps(2);
    for k in 1..=5 {
        harness.hover_at(from + (to - from) * (k as f32 / 5.0));
        harness.run_steps(2);
    }
    harness.drop_at(to);
    harness.run_steps(4);
}

#[test]
fn a_click_selects_the_whole_dxf_and_its_box_turns_with_its_placement() {
    let mut app = ChiploadApp::default();
    assert!(app.open(&corpus("jobs/v20/scaled_part.chipload")));
    let id = app.edit.as_ref().unwrap().job.drawing.dxfs[0].id;
    // The sample's DXF is turned already (90°): 30° more.
    assert!(app.place_dxf(Some(id), Move::Rotate(30.0)));
    let turn = dxf_placement(&app).rotation;
    assert!((turn - 120.0).abs() < 1e-9, "{turn}");
    let (contour, _) = on_dxf(&app, id);
    app.click(Some(contour), false);
    assert!(matches!(
        app.selection,
        CanvasSelection::Whole {
            of: Source::Dxf(_),
            ..
        }
    ));
    let (of, placed) = app.selected_box().unwrap();
    assert!(matches!(
        of,
        super::super::box_drag::BoxOf::Whole(Source::Dxf(_))
    ));
    assert!((placed.rotation - turn).abs() < 1e-9, "{placed:?}");
    // A double-click picks the contour alone.
    app.pick_alone(contour);
    assert!(matches!(app.selection, CanvasSelection::Shapes(_)));
}

#[test]
fn a_dxf_dragged_by_its_line_moves_and_one_undo_takes_it_back() {
    let mut harness = window("jobs/v20/scaled_part.chipload", None);
    let id = harness.state().edit.as_ref().unwrap().job.drawing.dxfs[0].id;
    let (contour, on) = on_dxf(harness.state(), id);
    harness.state_mut().click(Some(contour), false);
    harness.run_steps(2);
    let before = dxf_placement(harness.state());
    let scale = harness.state().view.scale as f32;
    let from = harness.state().view.to_screen(on);
    drag(
        &mut harness,
        from,
        from + egui::vec2(20.0 * scale, -10.0 * scale),
    );
    let after = dxf_placement(harness.state());
    // The grabbed point lands on the grid point nearest the cursor.
    let (dx, dy) = (after.dx - before.dx, after.dy - before.dy);
    assert!(
        (dx - 20.0).abs() <= 2.5 && (dy - 10.0).abs() <= 2.5,
        "({dx}, {dy})"
    );
    assert!(matches!(
        harness.state().selection,
        CanvasSelection::Whole {
            of: Source::Dxf(_),
            ..
        }
    ));
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    harness.run_steps(3);
    assert_eq!(dxf_placement(harness.state()), before);
}

#[test]
fn a_dxf_turns_by_its_handle_and_scales_by_a_corner_of_its_turned_box() {
    let mut app = ChiploadApp::default();
    assert!(app.open(&corpus("jobs/v20/scaled_part.chipload")));
    let id = app.edit.as_ref().unwrap().job.drawing.dxfs[0].id;
    let turned_before = dxf_placement(&app).rotation;
    let (contour, _) = on_dxf(&app, id);
    app.click(Some(contour), false);
    // The turn handle (on the box's own +X side) dragged a quarter turn further round.
    let (_, b) = app.selected_box().unwrap();
    let c = b.center();
    let ahead = (b.rotation + 90.0).to_radians();
    app.start_box_drag(BoxHandle::Turn);
    app.drag_box_to(
        (c.0 + 100.0 * ahead.cos(), c.1 + 100.0 * ahead.sin()),
        false,
    );
    app.end_box_drag();
    let p = dxf_placement(&app);
    assert!(
        ((p.rotation - turned_before).rem_euclid(360.0) - 90.0).abs() < 1e-9,
        "{p:?}"
    );
    let (_, turned) = app.selected_box().unwrap();
    assert!(
        (turned.rotation - p.rotation).abs() < 1e-9,
        "the box stays turned"
    );
    // Its upper-right corner dragged out along the diagonal: the lower-left stays put.
    let corners = turned.corners();
    let (opposite, grabbed) = (corners[0], corners[2]);
    let to = (
        grabbed.0 + (grabbed.0 - opposite.0) * 0.5,
        grabbed.1 + (grabbed.1 - opposite.1) * 0.5,
    );
    app.start_box_drag(BoxHandle::Size(2));
    app.drag_box_to(to, true);
    app.end_box_drag();
    let (_, sized) = app.selected_box().unwrap();
    let still = sized.corners()[0];
    assert!(
        (still.0 - opposite.0).abs() < 0.01 && (still.1 - opposite.1).abs() < 0.01,
        "{still:?} moved from {opposite:?}"
    );
    // Scaled as its box: the shapes' own size (the box less its stand-off) grew alike.
    let pad = super::super::box_drag::SHAPES_BOX_PAD_PX / app.view.scale;
    let grew = (sized.size.0 - 2.0 * pad) / (turned.size.0 - 2.0 * pad);
    assert!((dxf_placement(&app).scale / p.scale - grew).abs() < 1e-6);
    assert!(grew > 1.4, "{grew}");
}

#[test]
fn a_dxf_carried_onto_the_shapes_tab_is_saved_not_moved() {
    let dir = std::env::temp_dir().join(format!("chipload-gui-dxf-carry-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let mut harness = window("jobs/v20/scaled_part.chipload", Some(dir));
    let id = harness.state().edit.as_ref().unwrap().job.drawing.dxfs[0].id;
    let (contour, on) = on_dxf(harness.state(), id);
    harness.state_mut().click(Some(contour), false);
    harness.run_steps(2);
    let before = dxf_placement(harness.state());
    let rect = harness.get_by_label("Shapes").rect();
    let from = harness.state().view.to_screen(on);
    drag(
        &mut harness,
        from,
        egui::pos2(rect.min.x + 6.0, rect.center().y),
    );
    assert_eq!(dxf_placement(harness.state()), before, "not moved");
    assert!(matches!(
        harness.state().shape_library.asking,
        Some(super::super::shape_library::Asking::Save { .. })
    ));
}

#[test]
fn an_svg_is_selected_whole_too() {
    let mut app = ChiploadApp::default();
    assert!(app.open(&corpus("jobs/v31/svg_badge.chipload")));
    let id = app.edit.as_ref().unwrap().job.drawing.dxfs[0].id;
    let (contour, _) = on_dxf(&app, id);
    app.click(Some(contour), false);
    assert_eq!(app.selection.source(), Some(Source::Dxf(id)));
    assert!(app.selected_box().is_some());
}

#[test]
fn a_turned_group_keeps_its_box_turned() {
    let mut app = ChiploadApp::default();
    assert!(app.open(&corpus("jobs/v35/sign_frame.chipload")));
    app.selection = CanvasSelection::Shapes([1, 5, 6].into());
    assert!(app.group_selected());
    let (_, b) = app.selected_box().unwrap();
    assert_eq!(b.rotation, 0.0);
    let c = b.center();
    app.start_box_drag(BoxHandle::Turn);
    // 30° up from the +X side (the handle turns in 15° steps).
    let t = 30f64.to_radians();
    app.drag_box_to((c.0 + 100.0 * t.cos(), c.1 + 100.0 * t.sin()), false);
    app.end_box_drag();
    let group = &app.edit.as_ref().unwrap().job.drawing.groups[0];
    assert!((group.rotation - 30.0).abs() < 1e-9, "{}", group.rotation);
    let (_, turned) = app.selected_box().unwrap();
    assert!((turned.rotation - 30.0).abs() < 1e-9);
    // The same size as before it turned: the box turned with the group.
    assert!((turned.size.0 - b.size.0).abs() < 1e-6 && (turned.size.1 - b.size.1).abs() < 1e-6);
}
