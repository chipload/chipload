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

#[test]
fn typed_values_draw_exact_shapes() {
    // docs/specs/drawing-input.md §2.
    let mut app = drawn_part();
    app.click(None, false);
    app.start_drawing(Tool::Polyline);
    let at = egui::pos2(0.0, 0.0);
    // The first point must be absolute.
    app.start_typing("@", at);
    app.typed.as_mut().unwrap().text = "@5,5".into();
    app.apply_typed((0.0, 0.0));
    assert!(app.typed.as_ref().unwrap().refused, "no last point yet");
    assert!(app.sketch.as_ref().unwrap().points.is_empty());
    // x,y, then a length towards the cursor, then length<angle, then back to the start.
    for (text, cursor) in [
        ("200,100", (0.0, 0.0)),
        ("30", (300.0, 100.0)),
        ("20<90", (0.0, 0.0)),
        ("200,100", (0.0, 0.0)),
    ] {
        app.start_typing(text, at);
        app.apply_typed(cursor);
    }
    assert!(app.sketch.is_none(), "typing the first point closes it");
    let id = app.selected_shape().unwrap();
    assert_eq!(
        app.shape(id).unwrap().geometry,
        Geometry::Polyline {
            closed: true,
            points: vec![[200.0, 100.0], [230.0, 100.0], [230.0, 120.0]],
            bulges: vec![0.0; 3],
        }
    );
    app.commit();
    // A circle: its typed distance is the radius.
    app.start_drawing(Tool::Circle);
    app.start_typing("50,50", at);
    app.apply_typed((0.0, 0.0));
    app.start_typing("12.5", at);
    app.apply_typed((60.0, 50.0));
    let id = app.selected_shape().unwrap();
    assert_eq!(
        app.shape(id).unwrap().geometry,
        Geometry::Circle {
            center: [50.0, 50.0],
            radius: 12.5
        }
    );
}

#[test]
fn enter_draws_with_the_last_tool_and_shift_holds_the_angle() {
    let mut app = drawn_part();
    app.view.scale = 10.0;
    assert_eq!(app.last_tool, None);
    app.start_drawing(Tool::Rectangle);
    app.cancel_drawing();
    assert_eq!(app.last_tool, Some(Tool::Rectangle));
    app.start_drawing(Tool::Polyline);
    app.draw_click((300.0, 300.0), false);
    // Shift: 15° steps from the last click, the length on the grid (1 mm at 10 px/mm).
    assert_eq!(
        app.drawing_point((310.4, 300.3), false, true),
        (310.0, 300.0)
    );
    assert_eq!(
        app.drawing_point((300.3, 287.6), false, true),
        (300.0, 288.0)
    );
    assert!(
        app.drag.snapped.unwrap().1.is_none(),
        "no snap while Shift is held"
    );
}

#[test]
fn a_polygon_is_its_centre_and_a_corner_with_the_sides_kept() {
    // docs/specs/drawing-input.md §5.
    let mut app = drawn_part();
    app.click(None, false);
    app.polygon_sides = 8;
    app.start_drawing(Tool::Polygon);
    assert_eq!(app.sketch.as_ref().unwrap().sides, 8);
    app.draw_click((200.0, 50.0), false);
    app.draw_click((200.0, 60.0), false);
    let id = app.selected_shape().unwrap();
    let Geometry::Polygon {
        center,
        radius,
        sides,
        angle,
    } = app.shape(id).unwrap().geometry
    else {
        panic!()
    };
    assert_eq!((center, radius, sides), ([200.0, 50.0], 10.0, 8));
    assert!((angle - std::f64::consts::FRAC_PI_2).abs() < 1e-12);
    // The next polygon starts with eight sides too.
    app.start_drawing(Tool::Polygon);
    assert_eq!(app.sketch.as_ref().unwrap().sides, 8);
}

#[test]
fn a_length_label_takes_a_typed_length() {
    // docs/specs/drawing-input.md §6: the triangle S3's first side, 30 mm, made 40.
    use chipload_job::edit::Dimension;
    let mut app = drawn_part();
    app.selection = CanvasSelection::Shapes([3].into());
    app.start_typing_dimension(Dimension::Segment(0), 30.0, egui::pos2(0.0, 0.0));
    assert_eq!(app.typed.as_ref().unwrap().text, "30");
    app.typed.as_mut().unwrap().text = "40".into();
    app.apply_typed((0.0, 0.0));
    assert!(app.typed.is_none());
    let Geometry::Polyline { points, .. } = &app.shape(3).unwrap().geometry else {
        panic!()
    };
    assert_eq!(points[..2], [[110.0, 65.0], [150.0, 65.0]]);
    // A length that can't be: the entry stays, refused.
    app.start_typing_dimension(Dimension::Segment(0), 40.0, egui::pos2(0.0, 0.0));
    app.typed.as_mut().unwrap().text = "0".into();
    app.apply_typed((0.0, 0.0));
    assert!(app.typed.as_ref().unwrap().refused);
}

#[test]
fn the_offset_tool_offsets_the_selection_to_the_cursor() {
    // docs/specs/drawing-editing.md §2: the circle S2 (r 12 at (75, 80)).
    use super::modify::ModifyTool;
    use chipload_pipeline::drawing_edit::OffsetSide;
    let mut app = drawn_part();
    app.view.scale = 10.0; // the grid is 1 mm
    app.start_modify(ModifyTool::Offset);
    // Nothing selected: a click on the circle selects it.
    let circle = contour_of(&app, 2);
    app.offset_click(Some(circle), (87.0, 80.0), false, false);
    assert_eq!(app.selected_drawn(), [2]);
    // 3.2 mm outside the circle: 3 mm on the grid.
    assert_eq!(
        app.offset_for_cursor((90.2, 80.0), false),
        Some((3.0, OffsetSide::Outside))
    );
    app.offset_click(None, (90.2, 80.0), false, false);
    assert_eq!(
        app.shape(4).unwrap().geometry,
        Geometry::Circle {
            center: [75.0, 80.0],
            radius: 15.0
        }
    );
    assert_eq!(app.selected_drawn(), [4], "the new shape is selected");
    app.commit();
    // Typed: 2 mm, inside (the cursor is inside the circle).
    app.start_typing("2", egui::pos2(0.0, 0.0));
    app.apply_typed((75.0, 80.0));
    assert!(app.typed.is_none());
    assert!(matches!(
        app.shape(5).unwrap().geometry,
        Geometry::Circle { radius, .. } if radius == 13.0
    ));
    app.cancel_modify();
    assert!(app.modify.is_none());
}

#[test]
fn the_corner_tools_round_or_cut_the_clicked_corner() {
    // docs/specs/drawing-editing.md §3: the triangle S3 (110, 65), (140, 65), (125, 95).
    use super::modify::ModifyTool;
    let mut app = drawn_part();
    app.view.scale = 10.0;
    app.start_modify(ModifyTool::Fillet);
    app.fillet_radius = 2.0;
    // A click on the triangle selects it; then a click by its first corner rounds it.
    app.modify_click(
        Some(contour_of(&app, 3)),
        (125.0, 65.0),
        false,
        (false, false),
    );
    assert_eq!(app.selected_drawn(), [3]);
    assert_eq!(app.corner_at((110.2, 65.1)).map(|c| c.1), Some(0));
    app.modify_click(None, (110.2, 65.1), false, (false, false));
    let Geometry::Polyline { points, bulges, .. } = &app.shape(3).unwrap().geometry else {
        panic!("a triangle with a rounded corner is a polyline")
    };
    assert_eq!(
        (points.len(), bulges.iter().filter(|&&b| b != 0.0).count()),
        (4, 1)
    );
    app.commit();
    // Chamfer, Shift: the two corners left are cut.
    app.start_modify(ModifyTool::Chamfer);
    app.chamfer_distance = 1.0;
    app.modify_click(None, (0.0, 0.0), false, (false, true));
    let Geometry::Polyline { points, .. } = &app.shape(3).unwrap().geometry else {
        panic!()
    };
    assert_eq!(points.len(), 6);
}

#[test]
fn trim_extend_and_break_work_on_the_shape_clicked() {
    // docs/specs/drawing-editing.md §4, on lines drawn away from the sample's shapes.
    use super::modify::ModifyTool;
    let mut app = drawn_part();
    let line = |app: &mut ChiploadApp, a: [f64; 2], b: [f64; 2]| {
        let id = app
            .add_drawn(Geometry::straight_polyline(false, vec![a, b]))
            .unwrap();
        app.commit();
        id
    };
    let a = line(&mut app, [200.0, 150.0], [240.0, 150.0]);
    let _wall = line(&mut app, [220.0, 140.0], [220.0, 160.0]);
    let short = line(&mut app, [200.0, 170.0], [210.0, 170.0]);
    let _far = line(&mut app, [230.0, 165.0], [230.0, 175.0]);
    let points = |app: &ChiploadApp, id: u64| match &app.shape(id).unwrap().geometry {
        Geometry::Polyline { points, .. } => points.clone(),
        other => panic!("{other:?}"),
    };
    // Trim: the part of A beyond the wall goes.
    app.start_modify(ModifyTool::Trim);
    let picked = Some(contour_of(&app, a));
    assert!(!app.on_shape_preview(picked, (230.0, 150.0)).0.is_empty());
    app.modify_click(picked, (230.0, 150.0), false, (false, false));
    assert_eq!(points(&app, a), [[200.0, 150.0], [220.0, 150.0]]);
    app.commit();
    // Extend: the short line's right end grows to the far line.
    app.start_modify(ModifyTool::Extend);
    app.modify_click(
        Some(contour_of(&app, short)),
        (209.0, 170.0),
        false,
        (false, false),
    );
    assert_eq!(points(&app, short), [[200.0, 170.0], [230.0, 170.0]]);
    app.commit();
    // Break: A splits in two at the point clicked; the first part keeps its id.
    app.start_modify(ModifyTool::Break);
    let before = app.edit.as_ref().unwrap().settings.drawing.shapes.len();
    app.modify_click(
        Some(contour_of(&app, a)),
        (210.0, 150.3),
        false,
        (false, false),
    );
    assert_eq!(points(&app, a), [[200.0, 150.0], [210.0, 150.0]]);
    assert_eq!(
        app.edit.as_ref().unwrap().settings.drawing.shapes.len(),
        before + 1
    );
}

#[test]
fn union_and_intersect_act_on_the_selection_and_subtract_on_the_shape_clicked() {
    // docs/specs/drawing-editing.md §5. S1 a rectangle round S2 (circle at 75,80 r 12).
    use super::modify::ModifyTool;
    let mut app = drawn_part();
    let circle = |app: &mut ChiploadApp, x: f64, r: f64| {
        let id = app
            .add_drawn(Geometry::Circle {
                center: [x, 80.0],
                radius: r,
            })
            .unwrap();
        app.commit();
        id
    };
    let kind = |app: &ChiploadApp, id: u64| app.shape(id).map(chipload_job::Shape::kind_name);
    // Union needs two shapes selected; S2 takes the joined outline, the other goes.
    let bump = circle(&mut app, 87.0, 5.0);
    app.selection = CanvasSelection::Shapes([2].into());
    assert!(!app.modify_ready(ModifyTool::Union));
    app.selection = CanvasSelection::Shapes([2, bump].into());
    assert!(app.modify_ready(ModifyTool::Union));
    app.start_modify(ModifyTool::Union);
    assert_eq!(kind(&app, 2), Some("polyline"));
    assert!(app.shape(bump).is_none());
    assert_eq!(app.modify, None);
    app.commit();
    // Subtract: the selected circle cut out of the rectangle clicked; the tool ends.
    let notch = circle(&mut app, 160.0, 10.0);
    app.selection = CanvasSelection::Shapes([notch].into());
    app.start_modify(ModifyTool::Subtract);
    let picked = Some(contour_of(&app, 1));
    assert!(!app.on_shape_preview(picked, (40.0, 60.0)).0.is_empty());
    app.modify_click(picked, (40.0, 60.0), false, (false, false));
    assert_eq!(kind(&app, 1), Some("polyline"));
    assert!(app.shape(notch).is_none());
    assert_eq!(app.modify, None);
}

#[test]
fn the_array_tools_preview_their_copies_and_create_them() {
    // docs/specs/drawing-editing.md §6: S2 a circle at 75,80.
    use super::modify::ModifyTool;
    let mut app = drawn_part();
    app.selection = CanvasSelection::Shapes([2].into());
    let count = |app: &ChiploadApp| app.edit.as_ref().unwrap().settings.drawing.shapes.len();
    // A grid: three columns, the preview shows the two copies.
    app.start_modify(ModifyTool::Array);
    app.array.columns = 3;
    app.array.dx = 30.0;
    assert_eq!(app.array_outlines().len(), 2);
    assert!(app.create_array());
    assert_eq!(count(&app), 5);
    assert_eq!(app.modify, None);
    app.commit();
    // Polar: a click sets the centre (snapped to the grid), four round it.
    app.selection = CanvasSelection::Shapes([2].into());
    app.start_modify(ModifyTool::PolarArray);
    app.modify_click(None, (100.0, 80.0), false, (false, false));
    assert_eq!(app.array.center, (100.0, 80.0));
    app.array.count = 4;
    assert_eq!(app.array_outlines().len(), 3);
    assert!(app.create_array());
    assert_eq!(count(&app), 8);
}

#[test]
fn the_selection_aligns_and_distributes_its_shapes() {
    // docs/specs/drawing-editing.md §7: S2 a circle at 75,80 r 12, S3 a triangle from
    // 110,65 to 140,95.
    use super::modify::Arrange;
    use chipload_pipeline::drawing_edit::AlignTo;
    let mut app = drawn_part();
    app.selection = CanvasSelection::Shapes([2, 3].into());
    assert!(app.arrange_selected(Arrange::Align(AlignTo::Bottom)));
    assert!(matches!(
        app.shape(2).unwrap().geometry,
        Geometry::Circle { center, .. } if center == [75.0, 77.0]
    ));
    app.commit();
    // Distributing needs three; one shape aligns to the stock only.
    assert!(!app.arrange_selected(Arrange::Distribute(true)));
    app.selection = CanvasSelection::Shapes([2].into());
    assert!(!app.arrange_selected(Arrange::Align(AlignTo::Left)));
    app.align_to_stock = true;
    assert!(app.arrange_selected(Arrange::Align(AlignTo::Left)));
}

#[test]
fn svgs_open_and_a_file_in_pixels_asks_for_its_pixels_per_inch() {
    // docs/specs/svg-import.md §7, §10.
    let mut app = ChiploadApp::default();
    assert!(app.open(&corpus("svg/badge_layers.svg")));
    assert_eq!(app.contours.len(), 3);
    // Dropped on the open project: a second drawing file, which asks first.
    assert!(app.drop_files(&[corpus("svg/pixels.svg")]));
    assert!(matches!(
        &app.dialogs.prompt,
        Some(Prompt::PxPerInch(_, UnitsFor::Add, typed)) if typed == "96"
    ));
    assert!(app.add_file_from(&corpus("svg/pixels.svg"), None, Some(96.0)));
    let files = &app.edit.as_ref().unwrap().settings.drawing.dxfs;
    assert_eq!(files.len(), 2);
    assert_eq!(files[1].px_per_inch, Some(96.0));
    // Colours as layers: the red plate, the blue hole.
    let layers: BTreeSet<&str> = app
        .edit
        .as_ref()
        .unwrap()
        .settings
        .drawing
        .shapes
        .iter()
        .filter(|s| s.dxf() == Some(2))
        .map(|s| s.layer.as_str())
        .collect();
    assert_eq!(layers, BTreeSet::from(["#0000ff", "#ff0000"]));
}

#[test]
fn the_drawing_is_exported_as_a_dxf_with_its_choices() {
    // docs/specs/dxf-export.md §A6. S1 rectangle, S2 circle, S3 triangle.
    let mut app = drawn_part();
    app.ask_export_dxf();
    assert_eq!(app.dialogs.prompt, Some(Prompt::ExportDxf));
    let dir = std::env::temp_dir().join("chipload-gui-export-dxf");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("part.dxf");
    let read_back = |path: &Path| {
        chipload_dxf::shapes_from_dxf_bytes(&std::fs::read(path).unwrap(), "x", None, 1.0)
            .unwrap()
            .shapes
            .len()
    };
    assert!(app.export_dxf(&path));
    assert_eq!(read_back(&path), 3);
    // Only the selection.
    app.selection = CanvasSelection::Shapes([2].into());
    app.dxf_export.selection = true;
    assert!(app.export_dxf(&path));
    assert_eq!(read_back(&path), 1);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn k_draws_construction_geometry_and_toggles_the_selection() {
    // docs/specs/dxf-export.md §B2.
    let mut app = drawn_part();
    app.start_drawing(Tool::Circle);
    app.toggle_construction();
    assert!(app.construction_mode);
    let id = app
        .add_drawn(Geometry::Circle {
            center: [10.0, 10.0],
            radius: 3.0,
        })
        .unwrap();
    assert!(app.shape(id).unwrap().construction);
    app.commit();
    // No tool, a shape selected: K makes it construction, and again ordinary.
    app.sketch = None;
    app.selection = CanvasSelection::Shapes([3].into());
    app.toggle_construction();
    assert!(app.shape(3).unwrap().construction);
    app.commit();
    app.toggle_construction();
    assert!(!app.shape(3).unwrap().construction);
}
