// SPDX-License-Identifier: GPL-3.0-or-later
//! Dogbones chosen per corner on the canvas (docs/specs/dogbone-per-corner.md §6).
use super::*;
use chipload_job::{Bone, Corners, DEFAULT_CORNER_ANGLE_RAD, Wall};

/// The user's drawing (2026-10-26) with a pocket on contour #30: a 9 mm slot with a
/// 3.167 mm step at each end, cut with the 6 mm tool.
fn slot_pocket() -> ChiploadApp {
    let mut app = ChiploadApp::default();
    assert!(app.import_dxf(&corpus("dxf/shared_edges.dxf"), None));
    select(&mut app, &[30]);
    app.add_operation(OpKind::Pocket).unwrap();
    app.focus = Focus::Op(0);
    app
}

fn set_dogbones(app: &mut ChiploadApp) {
    if let Operation::Pocket(p) = &mut app.edit.as_mut().unwrap().ops[0].op {
        p.corners = Some(Corners::Dogbone {
            max_angle: DEFAULT_CORNER_ANGLE_RAD,
        });
    }
    app.commit();
}

/// The steps' feet, where no 6 mm bone fits but a T-bone along the step.
fn step_corners(app: &ChiploadApp) -> Vec<[f64; 2]> {
    app.scene
        .corner_markers
        .iter()
        .filter(|m| (m.at.1 - 301.0).abs() < 1e-6)
        .map(|m| [m.at.0, m.at.1])
        .collect()
}

#[test]
fn a_round_pockets_corners_show_and_can_each_get_a_bone() {
    let mut app = slot_pocket();
    app.update_corner_markers();
    let markers = &app.scene.corner_markers;
    assert_eq!(markers.len(), 6, "{markers:?}");
    assert!(markers.iter().all(|m| m.notch.is_none() && !m.chosen));

    // Start round and pick: the step corners get T-bones along the step.
    for at in step_corners(&app) {
        app.choose_corner(0, at, Some((Bone::Tbone, Some(Wall::Shorter))));
    }
    assert!(app.wait_for_generation(), "{:?}", app.report);
    app.update_corner_markers();
    let chosen: Vec<_> = app
        .scene
        .corner_markers
        .iter()
        .filter(|m| m.chosen)
        .collect();
    assert_eq!(chosen.len(), 2);
    for m in chosen {
        // The tool's circle at the tip: on the long edge's line, 3 mm along from the corner.
        let (centre, r) = m.notch.unwrap();
        assert!((r - 3.0).abs() < 1e-9);
        assert!((centre.1 - 301.0).abs() < 1e-6, "{centre:?}");
        assert!(((centre.0 - m.at.0).abs() - 3.0).abs() < 1e-6, "{centre:?}");
    }

    // Each choice is one undo step.
    app.undo();
    assert_eq!(
        app.edit.as_ref().unwrap().ops[0].op.corner_choices().len(),
        1
    );
}

#[test]
fn the_users_pocket_generates_with_its_steps_chosen() {
    let mut app = slot_pocket();
    set_dogbones(&mut app);
    assert!(!app.generate_and_wait());
    assert!(
        app.report
            .iter()
            .any(|l| l.contains("another wall of its own contour #30")),
        "{:?}",
        app.report
    );
    app.update_corner_markers();
    assert!(app.scene.corner_markers.iter().all(|m| m.notch.is_some()));
    for at in step_corners(&app) {
        app.choose_corner(0, at, Some((Bone::None, None)));
    }
    assert!(app.wait_for_generation(), "{:?}", app.report);
    assert!(app.result.is_some(), "{:?}", app.report);
}

#[test]
fn corner_markers_are_only_for_the_focused_operation_in_top_view() {
    let mut app = slot_pocket();
    app.update_corner_markers();
    let at = app.scene.corner_markers[0].at;
    let pos = app.view.to_screen(at);
    assert_eq!(app.corner_marker_at(pos), Some(0));
    app.focus = Focus::Job;
    app.update_corner_markers();
    assert!(app.scene.corner_markers.is_empty());
    app.focus = Focus::Op(0);
    app.view_3d();
    app.update_corner_markers();
    assert!(app.scene.corner_markers.is_empty());
}

#[test]
fn moving_a_drawn_shape_takes_its_corner_choices_and_start_points() {
    // docs/specs/dogbone-per-corner.md §4, decision 6
    let mut app = drawn_part();
    {
        let op = &mut app.edit.as_mut().unwrap().ops[0].op;
        op.starts_mut().unwrap().push([125.0, 66.0]);
        op.corner_choices_mut()
            .unwrap()
            .push(chipload_job::CornerChoice {
                at: [125.0, 95.0],
                bone: Bone::Dogbone,
                along: None,
            });
    }
    app.commit();
    // The triangle (S3) moves by (10, -5); the circle (S2) stays.
    assert!(app.transform_shapes(&[3], (0.0, 0.0), (10.0, -5.0), 1.0, 0.0));
    let op = &app.edit.as_ref().unwrap().ops[0].op;
    assert_eq!(op.starts(), &[[135.0, 61.0]]);
    assert_eq!(op.corner_choices()[0].at, [135.0, 90.0]);
}
