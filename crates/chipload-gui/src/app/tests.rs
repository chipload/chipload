// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use chipload_job::Lead;
use chipload_toolpath::model::MoveKind;

fn corpus(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/corpus")
        .join(rel)
}

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("chipload-gui-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn select(app: &mut ChiploadApp, ids: &[usize]) {
    app.selection = CanvasSelection::of_contours(&ids.iter().copied().collect(), &app.contours);
}

/// messy_2 imported: a pocket on the crossing bars, then the lower outline.
fn lower_part() -> ChiploadApp {
    let mut app = ChiploadApp::default();
    assert!(app.import_dxf(&corpus("dxf/messy_2.dxf"), None));
    select(&mut app, &[4, 6]);
    app.add_operation(OpKind::Pocket).unwrap();
    select(&mut app, &[2]);
    app.add_operation(OpKind::Profile).unwrap();
    app
}

fn first_cut(app: &ChiploadApp) -> (f64, f64) {
    let moves = &app.result.as_ref().unwrap().program.moves;
    let m = moves
        .iter()
        .find(|m| m.kind == MoveKind::Rapid && m.x.is_some())
        .unwrap();
    (m.x.unwrap(), m.y.unwrap())
}

#[test]
fn import_create_operations_generate_save_and_run_without_the_dxf() {
    let mut app = lower_part();
    assert!(app.dirty(), "a new project is unsaved");
    assert!(app.generate(), "{:?}", app.report);
    let dir = temp_dir("save");
    let project = dir.join("lower.chipload");
    let gcode_path = dir.join("lower.nc");
    assert!(app.save_project(&project));
    assert!(!app.dirty());
    assert!(app.export_gcode(&gcode_path));
    // The saved project is self-contained: run_job on it gives the exported G-code.
    let job = load_job(&project).unwrap();
    assert!(!job.drawing.shapes.is_empty());
    let again = run_job(&job).unwrap();
    assert_eq!(
        std::fs::read_to_string(&gcode_path).unwrap(),
        again.files[0].gcode
    );
    // Reopening it shows the same contours.
    let mut reopened = ChiploadApp::default();
    assert!(reopened.open(&project));
    assert_eq!(reopened.contours, app.contours);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn generating_shows_the_material_left_until_the_project_changes() {
    let mut app = lower_part();
    assert!(app.generate(), "{:?}", app.report);
    // Bar #4's four square corners. Bar #6's right end is rounded and its left corners
    // lie inside bar #4, which clears them. The outline (operation 1, outside) has no
    // inside corners.
    assert!(app.scene.left.iter().all(|l| l.operation == 0));
    assert_eq!(app.scene.left.len(), 4);
    assert!(
        app.report
            .iter()
            .any(|l| l.contains("the tool can't reach material in 4 places"))
    );
    app.toolpath_out_of_date();
    assert!(app.scene.left.is_empty());
}

// -- drawing (docs/specs/drawing-tools.md)

fn drawn_part() -> ChiploadApp {
    let mut app = ChiploadApp::default();
    assert!(app.open(&corpus("jobs/v5/drawn_part.chipload")));
    app
}

fn contour_of(app: &ChiploadApp, shape: u64) -> usize {
    app.contours
        .iter()
        .find(|c| c.parts.iter().any(|p| p.segment.shape == Some(shape)))
        .unwrap()
        .id
}

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
    assert!(app.generate(), "{:?}", app.report);
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
    assert!(app.generate(), "{:?}", app.report);
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
    assert!(!app.generate());
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
    // Three depth passes, retracting between rings, like the reported project.
    if let Some(Operation::Pocket(p)) = app.edit.as_mut().map(|e| &mut e.ops[0].op) {
        p.end_depth = 3.0;
        p.step_down = 1.0;
        p.keep_tool_down = false;
    }
    app.commit();
    assert!(app.generate(), "{:?}", app.report);
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
    assert!(pocket_total >= 6, "{pocket_total}");
    assert!(text.ends_with(&format!(" of {pocket_total}")), "{text}");
}

#[test]
fn reordering_changes_the_cut_order() {
    let mut app = lower_part();
    assert!(app.generate());
    let pocket_first = first_cut(&app);
    app.move_operation(1, 0);
    assert!(app.result.is_none(), "editing clears the toolpath until F5");
    assert!(app.generate());
    let profile_first = first_cut(&app);
    assert_ne!(pocket_first, profile_first);
    // The outline profile starts on the outline, far from the bars in the middle.
    assert!(profile_first.1.abs() > 40.0, "{profile_first:?}");
}

#[test]
fn disabled_operations_are_not_cut() {
    let mut app = lower_part();
    assert!(app.generate());
    let all = app.result.as_ref().unwrap().program.moves.len();
    if let Operation::Pocket(p) = &mut app.edit.as_mut().unwrap().ops[0].op {
        p.enabled = false;
    }
    app.commit();
    assert!(app.generate());
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
    assert!(!app.generate());
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
    assert!(!app.generate());
    app.undo();
    assert_eq!(app.edit, before);
    assert!(app.generate(), "{:?}", app.report);
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
    assert!(matches!(app.prompt, Some(Prompt::Units(_, false))));
    assert!(app.import_dxf(&corpus("dxf/messy.dxf"), Some(Units::Mm)));
    assert!(!app.contours.is_empty());
    let source = app.edit.as_ref().unwrap().settings.drawing.source.clone();
    assert_eq!(source.unwrap().units_from, chipload_job::UnitsFrom::User);
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
fn operation_numbers_are_read_from_messages() {
    assert_eq!(operation_index("Operation 3 (Outline): bad"), Some(2));
    assert_eq!(operation_index("Operation 1: x"), Some(0));
    assert_eq!(operation_index("no operation"), None);
}

// -- 3D view (docs/specs/3d-view.md)

#[test]
fn the_3d_view_can_be_entered_and_left_and_drawing_returns_to_top_view() {
    let mut app = drawn_part();
    app.view_3d();
    assert!(app.three_d && !app.view.is_top());
    app.view_top();
    assert!(!app.three_d && app.view.is_top());
    app.view_3d();
    app.start_drawing(Tool::Circle);
    assert!(
        !app.three_d && app.view.is_top(),
        "drawing works in top view only"
    );
    assert!(app.sketch.is_some());
}

#[test]
fn contours_can_be_picked_in_3d() {
    let mut app = drawn_part();
    app.view_3d();
    let circle = contour_of(&app, 2);
    let on_it = app
        .contours
        .iter()
        .find(|c| c.id == circle)
        .unwrap()
        .points()[0];
    let at = app.view.to_screen(on_it);
    assert_eq!(app.scene.pick(&app.view, at), Some(circle));
}

// -- handles and snapping (docs/specs/drawing-follow-ups.md)

#[test]
fn dragging_a_handle_edits_the_shape_in_one_undo_step() {
    let mut app = drawn_part();
    app.commit();
    // S3 is the triangle (110, 65) (140, 65) (125, 95).
    app.click(Some(contour_of(&app, 3)), false);
    assert_eq!(app.selected_shape(), Some(3));
    assert_eq!(app.shape_handles().len(), 3 + 3, "3 points, 3 middles");
    app.start_handle_drag(Handle::Point(2));
    app.drag_handle_to((125.0, 100.0), false);
    app.drag_handle_to((126.0, 105.0), false);
    // Onto its neighbour: refused, the last valid shape stays and the form says why.
    app.drag_handle_to((140.0, 65.0), false);
    let Geometry::Polyline { points, .. } = &app.shape(3).unwrap().geometry else {
        panic!("polyline");
    };
    assert_eq!(points[2], [126.0, 105.0]);
    let why = app.shape_draft.as_ref().unwrap().error.clone().unwrap();
    assert!(why.contains("on top of the previous point"), "{why}");
    app.end_handle_drag();
    assert!(app.shape_draft.as_ref().unwrap().error.is_none());
    app.commit();
    // One undo step for the whole drag.
    app.undo();
    let Geometry::Polyline { points, .. } = &app.shape(3).unwrap().geometry else {
        panic!("polyline");
    };
    assert_eq!(points[2], [125.0, 95.0]);

    // Shift on a middle bends the segment: 7.5 mm below the 30 mm bottom side.
    app.click(Some(contour_of(&app, 3)), false);
    app.start_handle_drag(Handle::Middle(0));
    app.drag_handle_to((125.0, 57.5), true);
    app.end_handle_drag();
    assert!(matches!(
        &app.shape(3).unwrap().geometry,
        Geometry::Polyline { bulges, .. } if bulges[0] == 0.5
    ));
}

#[test]
fn handles_only_in_top_view_and_only_for_drawn_shapes() {
    let mut app = drawn_part();
    app.click(Some(contour_of(&app, 1)), false);
    // The rectangle: centre, 4 corners, rotation.
    assert_eq!(app.shape_handles().len(), 6);
    app.view_3d();
    assert!(app.shape_handles().is_empty());
    app.view_top();
    app.start_drawing(Tool::Circle);
    assert!(app.shape_handles().is_empty(), "not while drawing");
    app.cancel_drawing();
    // Imported shapes have none.
    let mut imported = lower_part();
    select(&mut imported, &[2]);
    assert!(imported.shape_handles().is_empty());
}

#[test]
fn drawing_snaps_to_the_drawing_unless_alt_is_held() {
    let mut app = drawn_part();
    app.view.scale = 10.0; // 8 px = 0.8 mm
    // Near the triangle's corner (110, 65): it snaps there and says so.
    let p = app.snap_world((110.3, 65.4), false, false);
    assert_eq!(p, (110.0, 65.0));
    let (_, hit) = app.snapped.unwrap();
    assert!(hit.unwrap().name().starts_with("end of #"));
    // Alt: exactly the cursor.
    assert_eq!(app.snap_world((110.3, 65.4), true, false), (110.3, 65.4));
    // Far from everything: the grid (1 mm at 10 px/mm).
    assert_eq!(app.snap_world((20.3, 20.6), false, false), (20.0, 21.0));
    assert!(app.snapped.unwrap().1.is_none());
}

// -- tool library and machine profiles (docs/specs/tool-library-and-machines.md)

mod library {
    use super::*;
    use chipload_job::ToolChange;
    use chipload_job::library::{
        Cutting, LibraryTool, MachineLibrary, MachineProfile, ToolLibrary, load_machines,
        load_tools, save_machines, save_tools,
    };

    fn settings(name: &str) -> PathBuf {
        let dir = temp_dir(&format!("settings-{name}"));
        let _ = std::fs::remove_file(dir.join("tools.json"));
        let _ = std::fs::remove_file(dir.join("machines.json"));
        dir
    }

    fn upcut() -> LibraryTool {
        LibraryTool {
            name: "3 mm upcut".into(),
            id: 2,
            kind: ToolType::Endmill,
            diameter: 3.0,
            flutes: Some(2),
            center_cutting: Some(true),
            rpm: Some(20000.0),
            cutting: Some(Cutting {
                feed: 800.0,
                plunge_feed: 200.0,
                step_down: 0.75,
                stepover: 0.35,
            }),
            angle: None,
            tip_diameter: None,
        }
    }

    fn shapeoko() -> MachineProfile {
        MachineProfile {
            name: "Shapeoko".into(),
            post: "grbl".into(),
            safe_z: 12.0,
            spindle: Spindle::Cw,
            max_feed: Some(5000.0),
            travel: Some(Travel {
                x: 830.0,
                y: 830.0,
                z: 95.0,
            }),
            work_zero: WorkZero::Stock {
                corner: StockCorner::BottomLeft,
            },
            max_overcut: 0.3,
            tool_change: ToolChange::Files,
        }
    }

    /// An app with a library holding one tool and one machine, both the defaults.
    #[test]
    fn the_theme_is_kept_for_next_time() {
        let dir = settings("theme");
        let _ = std::fs::remove_file(dir.join("settings.json"));
        let mut app = ChiploadApp::default();
        app.set_settings_dir(Some(dir.clone()));
        assert_eq!(
            app.theme,
            Theme::System,
            "no settings yet: follow the system"
        );
        app.set_theme(Theme::Light);
        let mut again = ChiploadApp::default();
        again.set_settings_dir(Some(dir));
        assert_eq!(again.theme, Theme::Light);
    }

    fn app_with_library(name: &str) -> (ChiploadApp, PathBuf) {
        let dir = settings(name);
        save_tools(
            &ToolLibrary {
                default_tool: Some("3 mm upcut".into()),
                tools: vec![upcut()],
                ..ToolLibrary::default()
            },
            &dir.join("tools.json"),
        )
        .unwrap();
        save_machines(
            &MachineLibrary {
                default_machine: Some("Shapeoko".into()),
                machines: vec![shapeoko()],
                ..MachineLibrary::default()
            },
            &dir.join("machines.json"),
        )
        .unwrap();
        let mut app = ChiploadApp::default();
        app.set_settings_dir(Some(dir.clone()));
        (app, dir)
    }

    fn settings_of(app: &ChiploadApp) -> &Job {
        &app.edit.as_ref().unwrap().settings
    }

    #[test]
    fn without_a_settings_folder_nothing_changes() {
        let mut app = ChiploadApp::default();
        assert!(app.library().is_none());
        assert!(app.import_dxf(&corpus("dxf/messy_2.dxf"), None));
        let job = settings_of(&app);
        assert!(job.machine.name.is_none() && job.tools[0].name.is_none());
        assert_eq!(
            job.tools[0].diameter,
            chipload_pipeline::DEFAULT_TOOL_DIAMETER
        );
        assert!(!app.save_to_library(SaveTarget::Machine, "Mine"));
    }

    #[test]
    fn a_new_project_starts_with_the_default_machine_and_tool() {
        let (mut app, _) = app_with_library("import");
        assert!(app.import_dxf(&corpus("dxf/messy_2.dxf"), None));
        let job = settings_of(&app);
        assert_eq!(job.machine, shapeoko().machine());
        assert_eq!(job.stock.max_overcut, 0.3);
        assert_eq!(job.tools, [upcut().tool(2)]);
        assert!(
            app.report
                .iter()
                .any(|l| l == "Started with machine: Shapeoko (default profile).")
        );
        // A new operation takes the tool's cutting values (nothing above to copy).
        select(&mut app, &[4]);
        let i = app.add_operation(OpKind::Pocket).unwrap();
        let Operation::Pocket(p) = &app.edit.as_ref().unwrap().ops[i].op else {
            panic!()
        };
        assert_eq!((p.feed, p.step_down, p.stepover), (800.0, 0.75, 0.35));
        assert_eq!(p.tool, 2);
    }

    #[test]
    fn using_a_profile_and_a_library_tool_are_single_undo_steps() {
        // A project made without a library, then the library appears.
        let (_, dir) = app_with_library("use");
        let mut app = lower_part();
        app.set_settings_dir(Some(dir));
        let before = settings_of(&app).clone();

        assert!(app.use_machine_profile("shapeoko"));
        let job = settings_of(&app);
        assert_eq!(job.machine.safe_z, 12.0);
        assert_eq!(job.machine.name.as_deref(), Some("Shapeoko"));
        assert!(
            app.report.iter().any(|l| l == "  safe Z 5 mm -> 12 mm"),
            "{:?}",
            app.report
        );

        // The project's one tool takes the library values and keeps its id, so the
        // operations still use it.
        let id = before.tools[0].id;
        assert!(app.use_library_tool(id, "3 mm upcut"));
        let job = settings_of(&app);
        assert_eq!(job.tools, [upcut().tool(id)]);
        assert!(
            app.report.iter().any(|l| l == "  diameter 6 mm -> 3 mm"),
            "{:?}",
            app.report
        );
        assert!(app.generate(), "{:?}", app.report);

        app.undo();
        app.undo();
        assert_eq!(settings_of(&app), &before);
    }

    #[test]
    fn an_operation_takes_a_library_tool_of_its_own() {
        // docs/specs/tool-per-operation.md §8.
        let dir = settings("op-tool");
        save_tools(
            &ToolLibrary {
                tools: vec![upcut()],
                ..ToolLibrary::default()
            },
            &dir.join("tools.json"),
        )
        .unwrap();
        let mut app = lower_part();
        app.set_settings_dir(Some(dir.clone()));
        let before = settings_of(&app).clone();
        let ops_before: Vec<Operation> = app
            .edit
            .as_ref()
            .unwrap()
            .ops
            .iter()
            .map(|e| e.op.clone())
            .collect();
        // The pocket (operation 1) is cut with the 3 mm tool: copied in as tool 2, one
        // undo step, feeds kept.
        assert!(app.tool_from_library(0, "3 mm upcut"));
        let job = settings_of(&app);
        assert_eq!(job.tools.len(), 2);
        assert_eq!(job.tools[1], upcut().tool(2));
        let ops: Vec<Operation> = app
            .edit
            .as_ref()
            .unwrap()
            .ops
            .iter()
            .map(|e| e.op.clone())
            .collect();
        assert_eq!((ops[0].tool(), ops[1].tool()), (2, 1));
        let mut same = ops[0].clone();
        same.set_tool(ops_before[0].tool());
        assert_eq!(same, ops_before[0], "only the tool changed");
        assert!(app.generate(), "{:?}", app.report);
        let files = &app.result.as_ref().unwrap().files;
        assert_eq!(files.len(), 2);
        // Exported as one file per tool, named after the chosen file.
        let out = temp_dir("op-tool-export").join("part.nc");
        assert!(app.export_gcode(&out));
        assert!(out.with_file_name("part-1-T2.nc").exists());
        assert!(out.with_file_name("part-2-T1.nc").exists());
        // Again: the library tool is found in the project, not added twice.
        assert!(app.tool_from_library(1, "3 mm upcut"));
        assert_eq!(settings_of(&app).tools.len(), 2);
        // A new operation gets the tool of the last one.
        select(&mut app, &[2]);
        let new = app.add_operation(OpKind::Profile).unwrap();
        assert_eq!(app.edit.as_ref().unwrap().ops[new].op.tool(), 2);
        app.undo();
        app.undo();
        app.undo();
        assert_eq!(settings_of(&app), &before);
    }

    #[test]
    fn saving_to_the_library_names_the_project_values() {
        let dir = settings("save");
        let mut app = lower_part();
        app.set_settings_dir(Some(dir.clone()));
        assert!(app.save_to_library(SaveTarget::Machine, " Router "));
        assert!(app.save_to_library(SaveTarget::Tool(1), "6 mm"));
        let job = settings_of(&app);
        assert_eq!(job.machine.name.as_deref(), Some("Router"));
        assert_eq!(job.tools[0].name.as_deref(), Some("6 mm"));
        let machines = load_machines(&dir.join("machines.json")).unwrap();
        assert_eq!(
            machines.machines[0],
            MachineProfile::from_job("Router", job)
        );
        let tools = load_tools(&dir.join("tools.json")).unwrap();
        assert!(tools.tools[0].same_values(&job.tools[0]));

        // The project and the library agree: no "differs" note.
        let rows =
            |app: &ChiploadApp| user_library::LibraryRows::machine(settings_of(app), app.library());
        assert!(rows(&app).note_text().unwrap().starts_with("from profile"));
        app.edit.as_mut().unwrap().settings.machine.safe_z = 9.0;
        assert_eq!(
            rows(&app).note_text().unwrap(),
            "differs from profile \"Router\": safe Z 9 mm -> 5 mm"
        );
    }

    #[test]
    fn the_settings_windows_store_rename_and_refuse_duplicates() {
        let (mut app, dir) = app_with_library("windows");
        let renamed = LibraryTool {
            name: "3 mm".into(),
            ..upcut()
        };
        app.store_library_tool(Some("3 mm upcut"), renamed).unwrap();
        let tools = load_tools(&dir.join("tools.json")).unwrap();
        assert_eq!(tools.default_tool.as_deref(), Some("3 mm"));
        assert_eq!(app.library.tools.tools[0].name, "3 mm");

        let err = app
            .store_library_tool(
                None,
                LibraryTool {
                    name: "3 MM".into(),
                    ..upcut()
                },
            )
            .unwrap_err();
        assert!(err.contains("already a tool"), "{err}");
        let err = app
            .store_machine_profile(
                None,
                MachineProfile {
                    max_overcut: -1.0,
                    name: "Bad".into(),
                    ..shapeoko()
                },
            )
            .unwrap_err();
        assert!(err.contains("max_overcut"), "{err}");
        assert_eq!(
            load_machines(&dir.join("machines.json"))
                .unwrap()
                .machines
                .len(),
            1
        );
        app.set_library_default(LibraryKind::Machines, "Shapeoko")
            .unwrap();
    }

    #[test]
    fn a_broken_library_is_reported_and_the_project_still_starts() {
        let dir = settings("broken");
        std::fs::write(dir.join("machines.json"), "{\"schema_version\": 9}").unwrap();
        let mut app = ChiploadApp::default();
        app.set_settings_dir(Some(dir));
        assert!(app.import_dxf(&corpus("dxf/messy_2.dxf"), None));
        assert!(
            app.report
                .iter()
                .any(|l| l.contains("schema_version 9 is not supported")),
            "{:?}",
            app.report
        );
        assert!(settings_of(&app).machine.name.is_none());
    }
}

// -- File > Open recent (docs/specs/open-recent.md)

#[test]
fn opened_and_saved_projects_are_recent_and_missing_ones_are_forgotten() {
    let dir = temp_dir("recent");
    let settings = dir.join("settings");
    let _ = std::fs::remove_dir_all(&settings);
    let mut app = lower_part();
    app.set_settings_dir(Some(settings.clone()));
    assert!(app.recent.is_empty(), "an import isn't recent until saved");
    let first = dir.join("first.chipload");
    let second = dir.join("second.chipload");
    assert!(app.save_project(&first));
    assert!(app.save_project(&second));
    assert!(app.open(&first));
    let name = |p: &str| {
        Path::new(p)
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned()
    };
    let names: Vec<String> = app.recent.iter().map(|p| name(p)).collect();
    assert_eq!(names, ["first.chipload", "second.chipload"]);
    assert!(Path::new(&app.recent[0]).is_absolute());

    // A new window reads the same list.
    let mut other = ChiploadApp::default();
    other.set_settings_dir(Some(settings.clone()));
    assert_eq!(other.recent, app.recent);

    // Unsaved changes: opening a recent project asks first.
    app.edit.as_mut().unwrap().settings.stock.thickness = 12.0;
    let second_path = app.recent[1].clone();
    assert!(!app.open_recent(&second_path));
    assert!(matches!(
        app.prompt,
        Some(Prompt::Discard(Pending::Open(_)))
    ));
    app.prompt = None;
    app.saved.clone_from(&app.edit);

    std::fs::remove_file(&second).unwrap();
    assert!(!app.open_recent(&second_path));
    assert!(
        app.report[0].contains("no longer exists"),
        "{:?}",
        app.report
    );
    assert_eq!(app.recent.len(), 1);

    app.forget_recent(None);
    assert!(app.recent.is_empty());
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_new_project_is_drawn_in_and_gets_its_dxf_later() {
    // docs/specs/new-project.md
    let mut app = ChiploadApp::default();
    assert_eq!(app.update_labels().0, "Update drawing from DXF…");
    app.ask_new();
    let job = app.edit.as_ref().unwrap().settings.clone();
    assert!(job.drawing.shapes.is_empty() && job.drawing.source.is_none());
    assert_eq!((job.stock.x, job.stock.y), (300.0, 200.0));
    assert!(app.job_path.is_none() && app.contours.is_empty());
    assert_eq!(app.update_labels().0, "Import DXF into project…");
    assert_eq!(
        app.scene.fit_points(0.0).len(),
        10,
        "the axes and the stock box"
    );

    // Draw a circle and cut it out.
    app.start_drawing(Tool::Circle);
    app.draw_click((50.0, 50.0), false);
    app.draw_click((50.0, 60.0), false);
    app.commit();
    assert_eq!(app.contours.len(), 1);
    let id = app.contours[0].id;
    select(&mut app, &[id]);
    app.add_operation(OpKind::Profile).unwrap();
    assert!(app.generate(), "{:?}", app.report);

    // Unsaved changes: a new project asks first.
    app.ask_new();
    assert!(matches!(app.prompt, Some(Prompt::Discard(Pending::New))));
    app.prompt = None;

    // The first DXF has no units: they are asked for, then the drawing comes in and the
    // drawn circle stays.
    assert!(!app.update_from(&corpus("dxf/messy.dxf"), None));
    assert!(matches!(app.prompt, Some(Prompt::Units(_, true))));
    assert!(app.update_from(&corpus("dxf/messy.dxf"), Some(Units::Mm)));
    assert!(app.prompt.is_none());
    assert!(app.shape(1).is_some());
    assert!(app.contours.len() > 1);
    assert_eq!(app.update_labels().0, "Update drawing from DXF…");
    assert!(app.generate(), "{:?}", app.report);
}

#[test]
fn copies_get_a_pitch_from_a_gap_and_are_drawn_faded() {
    // docs/specs/copies-grid.md §6
    let mut app = lower_part();
    assert!(app.generate(), "{:?}", app.report);
    let one = app.scene.toolpath.clone().unwrap();
    assert!(one.ghost_feeds.is_empty());

    app.edit.as_mut().unwrap().settings.copies = Some(chipload_job::Copies {
        columns: 2,
        rows: 2,
        dx: 0.0,
        dy: 0.0,
        order: chipload_job::CopyOrder::ByOperation,
    });
    app.set_copies_gap(5.0);
    let copies = app.edit.as_ref().unwrap().settings.copies.unwrap();
    assert!(copies.dx > 5.0 && copies.dy > 5.0, "{copies:?}");
    // Generating again: the copies are cut, the gap leaves them apart.
    app.edit.as_mut().unwrap().settings.stock.x = 1000.0;
    app.edit.as_mut().unwrap().settings.stock.y = 1000.0;
    app.commit();
    assert!(app.generate(), "{:?}", app.report);
    let result = app.result.as_ref().unwrap();
    let copy_numbers: BTreeSet<u32> = result.op_spans.iter().map(|s| s.copy).collect();
    assert_eq!(copy_numbers, BTreeSet::from([1, 2, 3, 4]));
    // Copy 1 as before, the other three faded, and plunges marked on copy 1 only.
    let path = app.scene.toolpath.as_ref().unwrap();
    assert_eq!(path.feeds.len(), one.feeds.len());
    assert_eq!(path.ghost_feeds.len(), 3 * one.feeds.len());
    assert_eq!(path.starts.len(), one.starts.len());
    assert_eq!(
        app.edit.as_ref().unwrap().to_job().unwrap().copies,
        Some(copies)
    );

    // An overlapping pitch is refused with the size it needs.
    app.edit
        .as_mut()
        .unwrap()
        .settings
        .copies
        .as_mut()
        .unwrap()
        .dx = 10.0;
    app.commit();
    assert!(!app.generate());
    assert!(
        app.report.iter().any(|l| l.contains("Copies overlap")),
        "{:?}",
        app.report
    );
}

#[test]
fn a_start_marker_is_dragged_along_its_ring_reset_and_undone() {
    // docs/specs/helix-and-leads.md part A
    let mut app = lower_part();
    assert!(app.generate(), "{:?}", app.report);
    app.focus = Focus::Op(1);
    let marker = app
        .scene
        .start_markers
        .iter()
        .position(|m| m.operation == 1)
        .expect("the profile's ring has a marker");
    assert_eq!(app.scene.start_markers[marker].stored, None);
    let before = first_cut(&app);
    // The ring's point halfway round, as far as possible from the default start.
    let ring = app.scene.start_markers[marker].ring.clone();
    let far = ring[ring.len() / 2];
    // Dragging follows the ring, wherever the pointer is.
    app.drag_start_to(marker, (far.0 + 0.4, far.1 - 0.3));
    let on_ring = app.scene.start_markers[marker].at;
    let (_, _, d) = chipload_toolpath::start::nearest_on_ring(&ring, on_ring).unwrap();
    assert!(d < 1e-9);
    app.drop_start();
    let starts = app.edit.as_ref().unwrap().ops[1].op.starts().to_vec();
    assert_eq!(starts, vec![[on_ring.0, on_ring.1]]);
    assert!(app.result.is_some(), "generated again: {:?}", app.report);
    // The profile now starts there; the markers show it.
    let moved = app
        .scene
        .start_markers
        .iter()
        .find(|m| m.operation == 1)
        .unwrap();
    assert_eq!(moved.stored, Some(0));
    // On a chord of an arc the start goes onto the arc itself: within the arc tolerance.
    let tol = chipload_core::tolerances::ARC_APPROX_TOL_MM;
    assert!(
        (moved.at.0 - on_ring.0).hypot(moved.at.1 - on_ring.1) <= tol,
        "{:?} vs {on_ring:?}",
        moved.at
    );
    assert_eq!(first_cut(&app), before, "the pocket still comes first");

    // Reset removes the point; undo brings it back.
    let marker = app
        .scene
        .start_markers
        .iter()
        .position(|m| m.operation == 1)
        .unwrap();
    app.reset_start(marker);
    assert!(app.edit.as_ref().unwrap().ops[1].op.starts().is_empty());
    app.undo();
    assert_eq!(app.edit.as_ref().unwrap().ops[1].op.starts().len(), 1);
}

#[test]
fn start_markers_are_only_for_the_focused_operation_in_top_view() {
    let mut app = lower_part();
    assert!(app.generate(), "{:?}", app.report);
    let marker = &app.scene.start_markers[0];
    let pos = app.view.to_screen(marker.at);
    let operation = marker.operation;
    app.focus = Focus::Job;
    assert_eq!(app.start_marker_at(pos), None);
    app.focus = Focus::Op(operation);
    assert_eq!(app.start_marker_at(pos), Some(0));
    app.view_3d();
    assert_eq!(app.start_marker_at(pos), None);
    app.view_top();
    // A change to the project hides them until the next generation.
    app.toolpath_out_of_date();
    assert!(app.scene.start_markers.is_empty());
}

#[test]
fn a_lead_set_on_an_operation_is_generated_and_saved() {
    // docs/specs/helix-and-leads.md part B
    let mut app = lower_part();
    assert!(app.generate(), "{:?}", app.report);
    let before = app.result.as_ref().unwrap().program.moves.len();
    if let Operation::Profile(p) = &mut app.edit.as_mut().unwrap().ops[1].op {
        p.lead = Some(Lead { radius: 3.0 });
    }
    assert!(app.generate(), "{:?}", app.report);
    let after = app.result.as_ref().unwrap().program.moves.len();
    assert!(
        after > before,
        "the lead arcs add moves: {before} -> {after}"
    );
    // The marker shows the ring without its leads.
    let marker = app
        .scene
        .start_markers
        .iter()
        .find(|m| m.operation == 1)
        .unwrap();
    assert!(marker.ring.len() > 3);
    let dir = temp_dir("lead");
    let project = dir.join("lead.chipload");
    assert!(app.save_project(&project));
    let job = load_job(&project).unwrap();
    assert_eq!(job.operations[1].lead(), Some(Lead { radius: 3.0 }));
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_report_with_warnings_opens_and_links_to_its_operations() {
    let mut app = ChiploadApp::default();
    assert!(app.open(&corpus("jobs/v7/leads.chipload")));
    assert!(!app.report_open, "nothing to warn about yet");
    let facts = app.drawing_facts.clone().unwrap();
    assert_eq!((facts.contours, facts.issues), (3, 0));
    assert!(app.generate(), "{:?}", app.report);
    assert_eq!(app.report.lines[0].tone, Tone::Good);
    assert!(app.report_open, "warnings open the report");
    // "Operation 2 (Outline): cuts 0.300 mm below the stock bottom ..."
    let warning = app
        .report
        .lines
        .iter()
        .find(|l| l.tone == Tone::Warning)
        .unwrap();
    assert_eq!(warning.op, Some(1), "{warning:?}");
    // Closing a project clears the drawing's facts with it.
    app.new_project();
    assert_eq!(app.drawing_facts.clone().unwrap().contours, 0);
}

#[test]
fn an_edit_after_generating_marks_the_toolpath_out_of_date() {
    let mut app = ChiploadApp::default();
    assert!(app.open(&corpus("jobs/v7/leads.chipload")));
    let drawing_only = crate::view::legend(&app.scene, true).len();
    assert!(app.generate(), "{:?}", app.report);
    assert!(!app.toolpath_stale);
    let generated = crate::view::legend(&app.scene, true).len();
    assert!(
        generated > drawing_only,
        "the legend explains the toolpath too"
    );
    match &mut app.edit.as_mut().unwrap().ops[0].op {
        Operation::Pocket(p) => p.feed = 900.0,
        _ => panic!("operation 1 is a pocket"),
    }
    app.commit();
    assert!(app.toolpath_stale, "the cleared toolpath is noticed");
    assert_eq!(crate::view::legend(&app.scene, true).len(), drawing_only);
    assert!(app.generate(), "{:?}", app.report);
    assert!(!app.toolpath_stale);
    // A project opened afresh has nothing out of date.
    app.commit();
    assert!(app.open(&corpus("jobs/v7/leads.chipload")));
    assert!(!app.toolpath_stale);
}

/// Lowest X and Y of a contour.
fn contour_min(app: &ChiploadApp, id: usize) -> (f64, f64) {
    let contour = app.contours.iter().find(|c| c.id == id).unwrap();
    contour
        .points()
        .iter()
        .fold((f64::MAX, f64::MAX), |(x, y), p| (x.min(p.0), y.min(p.1)))
}

#[test]
fn the_imported_drawing_is_moved_onto_the_stock_and_back() {
    // docs/specs/drawing-placement.md §5: the user's case, drawn first, DXF later.
    let mut app = ChiploadApp::default();
    app.ask_new();
    app.start_drawing(Tool::Circle);
    app.draw_click((250.0, 150.0), false);
    app.draw_click((250.0, 160.0), false);
    app.commit();
    assert!(app.update_from(&corpus("dxf/simple_part.dxf"), None));
    let circle = |app: &ChiploadApp| app.shape(1).unwrap().geometry.clone();
    let before = circle(&app);
    let placement = |app: &ChiploadApp| {
        app.edit
            .as_ref()
            .unwrap()
            .settings
            .drawing
            .source
            .as_ref()
            .unwrap()
            .placement
    };
    assert_eq!(placement(&app), chipload_job::Placement::default());
    // simple_part's outline (the outer contour) is 50 .. 250 × 50 .. 170.
    let outline = |app: &ChiploadApp| {
        app.contours
            .iter()
            .find(|c| c.nesting_depth == Some(0) && c.layers() == ["CUT_OUTSIDE"])
            .unwrap()
            .id
    };
    let near = |a: (f64, f64), b: (f64, f64)| (a.0 - b.0).abs() < 1e-6 && (a.1 - b.1).abs() < 1e-6;
    assert!(near(contour_min(&app, outline(&app)), (50.0, 50.0)));

    assert!(app.place(chipload_pipeline::placement::Move::BottomLeft { margin: 10.0 }));
    assert!(near(contour_min(&app, outline(&app)), (10.0, 10.0)));
    assert_eq!(circle(&app), before, "the drawn circle stays");
    assert_eq!(
        placement(&app),
        chipload_job::Placement {
            scale: 1.0,
            quarter_turns: 0,
            dx: -40.0,
            dy: -40.0
        }
    );

    // One undo step brings it back; redo moves it again.
    app.undo();
    assert!(near(contour_min(&app, outline(&app)), (50.0, 50.0)));
    app.redo();
    assert!(near(contour_min(&app, outline(&app)), (10.0, 10.0)));
    assert!(app.place(chipload_pipeline::placement::Move::To(0.0, 0.0)));
    assert_eq!(placement(&app), chipload_job::Placement::default());

    // Turned in place (docs/specs/drawing-rotation.md §5): the 200 × 120 outline centred on
    // (150, 110) becomes 120 × 200; one undo step; Reset takes it back.
    assert!(app.place(chipload_pipeline::placement::Move::Turn(-1)));
    assert_eq!(placement(&app).quarter_turns, 3);
    assert!(near(contour_min(&app, outline(&app)), (90.0, 10.0)));
    assert_eq!(circle(&app), before, "the drawn circle stays");
    app.undo();
    assert_eq!(placement(&app).quarter_turns, 0);
    app.redo();
    assert!(app.place(chipload_pipeline::placement::Move::Reset));
    assert!(near(contour_min(&app, outline(&app)), (50.0, 50.0)));
    assert_eq!(placement(&app).quarter_turns, 0);

    // Scaled about its centre (docs/specs/drawing-scale.md §5): to a width, one undo step;
    // a bad factor is refused and reported; Reset takes it back.
    assert!(app.place(chipload_pipeline::placement::Move::Width(100.0)));
    assert_eq!(placement(&app).scale, 0.5);
    assert!(near(contour_min(&app, outline(&app)), (100.0, 80.0)));
    assert_eq!(circle(&app), before, "the drawn circle stays");
    let size = |app: &ChiploadApp| {
        chipload_pipeline::placement::imported_size(
            &app.edit.as_ref().unwrap().settings.drawing.shapes,
        )
        .unwrap()
    };
    assert!(near(size(&app), (100.0, 60.0)));
    app.undo();
    assert_eq!(placement(&app).scale, 1.0);
    app.redo();
    assert!(!app.place(chipload_pipeline::placement::Move::Scale(5000.0)));
    assert_eq!(placement(&app).scale, 0.5);
    assert!(app.place(chipload_pipeline::placement::Move::Scale(3.0)));
    assert!(near(size(&app), (300.0, 180.0)));
    assert!(app.place(chipload_pipeline::placement::Move::Reset));
    assert_eq!(placement(&app), chipload_job::Placement::default());

    // A project without an imported drawing has nothing to move.
    let mut app = ChiploadApp::default();
    app.ask_new();
    assert!(!app.place(chipload_pipeline::placement::Move::Center));
}

#[test]
fn a_vcarve_is_added_for_the_selection_and_generated() {
    // docs/specs/v-carve.md §8: the letters project, its operation replaced by a new one.
    let mut app = ChiploadApp::default();
    assert!(app.open(&corpus("jobs/v10/vcarve_letters.chipload")));
    app.edit.as_mut().unwrap().remove(0);
    select(&mut app, &[1, 2, 3, 4]);
    let i = app.add_operation(OpKind::Vcarve).unwrap();
    let edit = app.edit.as_ref().unwrap();
    let Operation::Vcarve(v) = &edit.ops[i].op else {
        panic!("a V-carve");
    };
    assert_eq!(v.name, "V-carve 1");
    assert_eq!((v.start_depth, v.step_down), (0.0, None));
    assert!(app.generate(), "{:?}", app.report);
    let moves = &app.result.as_ref().unwrap().program.moves;
    assert!(
        moves
            .iter()
            .any(|m| m.kind == MoveKind::Feed && m.x.is_some() && m.z.is_some()),
        "cuts in X, Y and Z at once"
    );
    // Duplicating keeps the kind and marks the name.
    let edit = app.edit.as_mut().unwrap();
    let copy = edit.duplicate(i).unwrap();
    assert_eq!(edit.ops[copy].op.name(), "V-carve 1 (copy)");
    assert!(matches!(edit.ops[copy].op, Operation::Vcarve(_)));
}

/// Run the keyboard shortcuts for one frame with `key` pressed.
fn press(app: &mut ChiploadApp, key: egui::Key) {
    let ctx = egui::Context::default();
    let input = egui::RawInput {
        events: vec![egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }],
        ..Default::default()
    };
    ctx.begin_pass(input);
    app.shortcuts(&ctx);
    // The font atlas a first frame makes isn't painted anywhere here.
    ctx.end_pass().textures_delta.clear();
}

#[test]
fn the_delete_key_deletes_the_selected_drawn_shapes() {
    let mut app = drawn_part();
    app.click(Some(contour_of(&app, 2)), false);
    app.click(Some(contour_of(&app, 3)), true);
    press(&mut app, egui::Key::Delete);
    assert!(app.shape(2).is_none() && app.shape(3).is_none());
    assert!(app.shape(1).is_some());
    assert_eq!(app.selection, CanvasSelection::Nothing);
    assert!(
        app.report
            .iter()
            .any(|l| l.contains("Deleted shapes S2, S3"))
    );
    // One undo step brings both back.
    app.commit();
    app.undo();
    assert!(app.shape(2).is_some() && app.shape(3).is_some());
    // Nothing selected: the key does nothing.
    press(&mut app, egui::Key::Delete);
    assert!(app.shape(1).is_some() && app.shape(2).is_some());
}

#[test]
fn the_delete_key_keeps_imported_shapes() {
    let mut app = lower_part();
    app.click(Some(4), false);
    let id = app.selected_shape().unwrap();
    press(&mut app, egui::Key::Delete);
    assert!(app.shape(id).is_some());
    assert!(
        app.report
            .iter()
            .any(|l| l.contains("Imported shapes can't be deleted"))
    );
}
