// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

#[test]
fn each_kind_is_drawn_by_clicks_and_selected() {
    let mut app = drawn_part();
    app.click(None, false);
    app.start_drawing(Tool::Circle);
    app.draw_click((200.0, 50.0), false);
    app.draw_click((203.0, 54.0), false);
    assert!(app.sketch.is_none());
    assert_eq!(app.selected_shape(), Some(4));
    assert_eq!(
        app.shape(4).unwrap().geometry,
        Geometry::Circle {
            center: [200.0, 50.0],
            radius: 5.0
        }
    );
    assert_eq!(app.shape(4).unwrap().layer, "0");
    app.commit(); // as the UI loop does after every finished edit

    app.start_drawing(Tool::Rectangle);
    app.draw_click((220.0, 10.0), false);
    app.draw_click((250.0, 30.0), false);
    assert!(matches!(
        app.shape(5).unwrap().geometry,
        Geometry::Rectangle {
            length: 30.0,
            width: 20.0,
            ..
        }
    ));
    app.commit();

    app.start_drawing(Tool::Polyline);
    for p in [(200.0, 100.0), (230.0, 100.0), (215.0, 120.0)] {
        app.draw_click(p, false);
    }
    app.draw_click((200.0, 100.0), true); // the first point: closes it
    assert!(matches!(
        app.shape(6).unwrap().geometry,
        Geometry::Polyline { closed: true, .. }
    ));
    app.commit();

    app.start_drawing(Tool::Polyline);
    app.draw_click((260.0, 100.0), false);
    app.draw_click((280.0, 100.0), false);
    app.finish_drawing();
    assert!(matches!(
        app.shape(7).unwrap().geometry,
        Geometry::Polyline { closed: false, .. }
    ));
    // A closed drawn shape is its own contour; the open one is an open contour.
    let contour = |id| {
        app.contours
            .iter()
            .find(|c| c.id == contour_of(&app, id))
            .unwrap()
    };
    assert!(contour(6).closed && !contour(7).closed);

    // Every addition is an undo step.
    app.commit();
    app.undo();
    assert!(app.shape(7).is_none() && app.shape(6).is_some());
    app.redo();
    assert!(app.shape(7).is_some());
}

#[test]
fn escape_cancels_and_a_new_shape_takes_the_selections_layer() {
    let mut app = drawn_part();
    app.start_drawing(Tool::Circle);
    app.draw_click((10.0, 10.0), false);
    app.cancel_drawing();
    assert!(app.sketch.is_none());
    assert_eq!(app.edit.as_ref().unwrap().settings.drawing.shapes.len(), 3);
    // The circle S2 is on layer "drawn" and selected: the new shape goes there too.
    app.click(Some(contour_of(&app, 2)), false);
    app.start_drawing(Tool::Circle);
    app.draw_click((10.0, 10.0), false);
    app.draw_click((12.0, 10.0), false);
    assert_eq!(app.shape(4).unwrap().layer, "drawn");
}

#[test]
fn editing_the_selected_shape_moves_its_contour_and_operations_follow() {
    let mut app = drawn_part();
    assert!(app.generate_and_wait(), "{:?}", app.report);
    let before = first_cut(&app);
    // S2 is the circle the first operation (Holes) cuts first.
    app.click(Some(contour_of(&app, 2)), false);
    assert_eq!(app.selected_shape(), Some(2));
    let moved = Geometry::Circle {
        center: [70.0, 80.0],
        radius: 12.0,
    };
    app.set_selected_shape("drawn", moved.clone()).unwrap();
    assert_eq!(app.shape(2).unwrap().geometry, moved);
    // Still selected, though contours were recomputed.
    assert_eq!(app.selected_shape(), Some(2));
    // Refused values leave the project as it was.
    let err = app
        .set_selected_shape(
            "drawn",
            Geometry::Circle {
                center: [70.0, 80.0],
                radius: 0.0,
            },
        )
        .unwrap_err();
    assert!(err.contains("radius"), "{err}");
    assert_eq!(app.shape(2).unwrap().geometry, moved);
    app.commit();
    assert!(app.generate_and_wait(), "{:?}", app.report);
    assert!((first_cut(&app).0 - (before.0 - 5.0)).abs() < 1e-9);
}

#[test]
fn deleting_a_shape_flags_the_operations_using_it() {
    let mut app = drawn_part();
    app.click(Some(contour_of(&app, 2)), false);
    app.delete_selected_shape();
    assert!(app.shape(2).is_none());
    assert_eq!(app.error_ops, BTreeSet::from([0]));
    assert!(
        app.report
            .iter()
            .any(|l| l.contains("shape S2 no longer exists"))
    );
    app.commit();
    assert!(!app.generate_and_wait());
    app.undo();
    assert!(app.shape(2).is_some());
}

#[test]
fn the_selection_stays_on_its_shape_when_contours_are_renumbered() {
    let mut app = drawn_part();
    // Delete the rectangle S1 (contour #1): the triangle S3 becomes contour #2.
    app.click(Some(contour_of(&app, 1)), false);
    app.delete_selected_shape();
    app.commit();
    assert_eq!(contour_of(&app, 3), 2);
    app.click(Some(2), false);
    assert_eq!(app.selected_shape(), Some(3));
    // Undo brings S1 back as #1 and the triangle is #3 again. Contour #2 is now the
    // circle; a selection kept as contour numbers would have jumped to it.
    app.undo();
    assert_eq!(contour_of(&app, 3), 3);
    assert_eq!(app.selected_shape(), Some(3));
    assert_eq!(app.selection.ids(&app.contours), BTreeSet::from([3]));
}

#[test]
fn a_selected_shape_that_is_deleted_drops_out_of_the_selection() {
    let mut app = drawn_part();
    app.click(Some(contour_of(&app, 2)), false);
    app.click(Some(contour_of(&app, 3)), true);
    // Delete S3 behind the selection's back (as Update from DXF could).
    let edit = app.edit.as_mut().unwrap();
    edit.settings.drawing.shapes.retain(|s| s.id != 3);
    app.commit();
    app.refresh_drawing();
    assert_eq!(
        app.selection.ids(&app.contours),
        BTreeSet::from([contour_of(&app, 2)])
    );
}

#[test]
fn imported_shapes_are_not_editable() {
    let mut app = lower_part();
    app.click(Some(4), false);
    let id = app.selected_shape().unwrap();
    assert!(!app.shape(id).unwrap().is_drawn());
    let err = app
        .set_selected_shape(
            "x",
            Geometry::Circle {
                center: [0.0, 0.0],
                radius: 1.0,
            },
        )
        .unwrap_err();
    assert!(err.contains("imported from the DXF"), "{err}");
}

#[test]
fn plunges_are_counted_per_operation() {
    // The 35 x 24 mm pocket #5 needs several rings.
    let mut app = ChiploadApp::default();
    assert!(app.import_dxf(&corpus("dxf/messy_2.dxf"), None));
    select(&mut app, &[5]);
    app.add_operation(OpKind::Pocket).unwrap();
    select(&mut app, &[2]);
    app.add_operation(OpKind::Profile).unwrap();
    // Three depth passes, like the reported project.
    if let Some(Operation::Pocket(p)) = app.edit.as_mut().map(|e| &mut e.ops[0].op) {
        p.end_depth = 3.0;
        p.step_down = 1.0;
    }
    app.commit();
    assert!(app.generate_and_wait(), "{:?}", app.report);
    let path = app.scene.toolpath.clone().unwrap();
    // Two operations, many plunges (every depth pass, every ring change).
    let ops: BTreeSet<Option<usize>> = path.starts.iter().map(|s| s.operation).collect();
    assert_eq!(ops, BTreeSet::from([Some(0), Some(1)]));
    assert!(path.starts.len() > 2);
    // The pocket starts every depth at the same point: hovering it lists all of them.
    let first = path.starts[0].at;
    let same: Vec<usize> = (0..path.starts.len())
        .filter(|&i| path.starts[i].at == first)
        .collect();
    let pocket_total = path
        .starts
        .iter()
        .filter(|s| s.operation == Some(0))
        .count();
    let text = app.plunge_text(&same);
    assert!(
        text.starts_with("Operation 1 (Pocket 1): plunges 1, "),
        "{text}"
    );
    // One plunge per depth pass: the rings are linked at depth.
    assert_eq!(pocket_total, 3, "{pocket_total}");
    assert!(text.ends_with(&format!(" of {pocket_total}")), "{text}");
}

#[test]
fn reordering_changes_the_cut_order() {
    let mut app = lower_part();
    assert!(app.generate_and_wait());
    let pocket_first = first_cut(&app);
    app.move_operation(1, 0);
    assert!(app.result.is_none(), "editing clears the toolpath until F5");
    assert!(app.generate_and_wait());
    let profile_first = first_cut(&app);
    assert_ne!(pocket_first, profile_first);
    // The outline profile starts on the outline, far from the bars in the middle.
    assert!(profile_first.1.abs() > 40.0, "{profile_first:?}");
}

#[test]
fn disabled_operations_are_not_cut() {
    let mut app = lower_part();
    assert!(app.generate_and_wait());
    let all = app.result.as_ref().unwrap().program.moves.len();
    if let Operation::Pocket(p) = &mut app.edit.as_mut().unwrap().ops[0].op {
        p.enabled = false;
    }
    app.commit();
    assert!(app.generate_and_wait());
    assert!(app.result.as_ref().unwrap().program.moves.len() < all);
}

#[test]
fn undo_and_redo_restore_earlier_projects() {
    let mut app = lower_part();
    assert_eq!(app.edit.as_ref().unwrap().ops.len(), 2);
    app.undo();
    assert_eq!(app.edit.as_ref().unwrap().ops.len(), 1);
    app.undo();
    assert_eq!(app.edit.as_ref().unwrap().ops.len(), 0);
    app.redo();
    app.redo();
    assert_eq!(app.edit.as_ref().unwrap().ops.len(), 2);
    // A field edit is one step.
    if let Operation::Profile(p) = &mut app.edit.as_mut().unwrap().ops[1].op {
        p.end_depth = 5.0;
    }
    app.commit();
    app.undo();
    let Operation::Profile(p) = &app.edit.as_ref().unwrap().ops[1].op else {
        panic!("expected the profile");
    };
    assert_eq!(p.end_depth, 1.0);
}

#[test]
fn incomplete_projects_say_what_is_missing() {
    let mut app = ChiploadApp::default();
    app.import_dxf(&corpus("dxf/messy_2.dxf"), None);
    select(&mut app, &[1, 2]); // a hole and an outer: side must be chosen
    app.add_operation(OpKind::Profile).unwrap();
    assert!(!app.generate_and_wait());
    assert!(
        app.report
            .iter()
            .any(|l| l.contains("choose the profile side"))
    );
    assert_eq!(app.error_ops, BTreeSet::from([0]));
    let dir = temp_dir("incomplete");
    assert!(!app.save_project(&dir.join("p.chipload")));
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn selections_become_shape_or_layer_selectors() {
    let mut app = ChiploadApp::default();
    app.import_dxf(&corpus("dxf/messy_2.dxf"), None);
    app.select_layer("lower");
    assert_eq!(
        app.selector_json().as_deref(),
        Some("{\"layer\": \"lower\"}")
    );
    assert!(
        app.selection_lines()[1].contains("#7"),
        "the open centreline is named"
    );
    select(&mut app, &[4]);
    let json = app.selector_json().unwrap();
    assert!(json.starts_with("{\"shapes\": ["), "{json}");
}

#[test]
fn a_job_from_before_projects_is_refused_with_advice() {
    let dir = temp_dir("old-job");
    let job = dir.join("lower.json");
    std::fs::write(&job, r#"{"schema_version": 3}"#).unwrap();
    let mut app = ChiploadApp::default();
    assert!(!app.open(&job));
    assert!(app.edit.is_none());
    assert!(
        app.report.iter().any(|l| l.contains("Chipload 0.3.0")),
        "{:?}",
        app.report
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn updating_the_drawing_keeps_operations_and_can_be_undone() {
    let mut app = lower_part();
    let before = app.edit.clone();
    assert!(app.update_from(&corpus("dxf/messy_2.dxf"), None));
    assert!(app.report[0].contains("22 unchanged"), "{:?}", app.report);
    assert_eq!(app.edit, before, "same drawing: nothing changes");
    // A different drawing: the operations' shapes are gone, so they are flagged.
    assert!(app.update_from(&corpus("dxf/simple_part.dxf"), None));
    assert_eq!(app.error_ops, BTreeSet::from([0, 1]));
    assert!(!app.generate_and_wait());
    app.undo();
    assert_eq!(app.edit, before);
    assert!(app.generate_and_wait(), "{:?}", app.report);
}

#[test]
fn old_job_versions_are_saved_as_current_projects() {
    let mut app = ChiploadApp::default();
    let dir = temp_dir("old");
    std::fs::copy(
        corpus("jobs/v4/example_job.chipload"),
        dir.join("example.chipload"),
    )
    .unwrap();
    assert!(app.open(&dir.join("example.chipload")));
    let path = dir.join("again.chipload");
    assert!(app.save_project(&path));
    assert_eq!(
        raw_schema_version(&path),
        Some(chipload_job::CURRENT_VERSION)
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn unitless_drawing_asks_for_units() {
    let mut app = ChiploadApp::default();
    assert!(!app.open(&corpus("dxf/messy.dxf")));
    assert!(matches!(
        app.dialogs.prompt,
        Some(Prompt::Units(_, UnitsFor::NewProject))
    ));
    assert!(app.import_dxf(&corpus("dxf/messy.dxf"), Some(Units::Mm)));
    assert!(!app.contours.is_empty());
    let source = app.edit.as_ref().unwrap().settings.drawing.dxfs[0].clone();
    assert_eq!(source.units_from, chipload_job::UnitsFrom::User);
}

#[test]
fn clicks_select_and_toggle() {
    let mut app = ChiploadApp::default();
    app.import_dxf(&corpus("dxf/messy_2.dxf"), None);
    assert_eq!(app.selector_json(), None);
    app.click(Some(4), false);
    app.click(Some(6), true);
    assert_eq!(app.selection.ids(&app.contours), BTreeSet::from([4, 6]));
    app.click(Some(6), true);
    assert_eq!(app.selection.ids(&app.contours), BTreeSet::from([4]));
    app.click(None, false);
    assert_eq!(app.selection, CanvasSelection::Nothing);
}
