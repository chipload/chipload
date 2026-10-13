// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use chipload_job::{HoleMethod, Lead};
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
    assert!(app.generate_and_wait(), "{:?}", app.report);
    let dir = temp_dir("save");
    let project = dir.join("lower.chipload");
    let gcode_path = dir.join("lower.nc");
    assert!(app.save_project(&project));
    assert!(!app.dirty());
    assert!(app.export_gcode(&gcode_path));
    // The saved project is self-contained: run_job on it gives the exported G-code.
    let job = load_job(&project).unwrap();
    assert!(!job.drawing.shapes.is_empty());
    let again = chipload_pipeline::run_job(&job).unwrap();
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
fn the_simulation_is_on_from_the_start_and_shows_what_the_tools_leave() {
    // docs/specs/material-left-merge.md: the simulation, not a separate shading, shows the
    // material left, and the report has no "can't reach" lines any more.
    let mut app = lower_part();
    assert!(app.show_simulation);
    assert!(app.generate_and_wait(), "{:?}", app.report);
    let ctx = egui::Context::default();
    assert!(app.wait_for_simulation(&ctx));
    assert!(app.simulation.is_some());
    assert!(
        !app.report
            .iter()
            .any(|l| l.contains("can't reach material"))
    );
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
    // Three depth passes, retracting between rings, like the reported project.
    if let Some(Operation::Pocket(p)) = app.edit.as_mut().map(|e| &mut e.ops[0].op) {
        p.end_depth = 3.0;
        p.step_down = 1.0;
        p.keep_tool_down = false;
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
    assert!(pocket_total >= 6, "{pocket_total}");
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
        app.prompt,
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
    // The middles are drawn apart, and the status bar says how to use them.
    draw_canvas(&mut app);
    assert_eq!(app.scene.handles.len(), 3);
    assert_eq!(app.scene.middle_handles.len(), 3);
    assert!(app.polyline_selected());
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
    assert!(!app.polyline_selected());
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

    #[test]
    fn export_writes_the_setup_sheet_unless_turned_off() {
        // docs/specs/setup-sheet.md §4
        let dir = settings("sheet");
        let _ = std::fs::remove_file(dir.join("settings.json"));
        let mut app = lower_part();
        app.set_settings_dir(Some(dir.clone()));
        assert!(app.setup_sheet, "on by default");
        assert!(app.generate_and_wait(), "{:?}", app.report);
        let out = temp_dir("sheet-export");
        let project = out.join("lower.chipload");
        assert!(app.save_project(&project));
        assert!(app.export_gcode(&out.join("lower.nc")));
        let sheet = std::fs::read_to_string(out.join("lower-sheet.html")).unwrap();
        assert!(sheet.contains("Setup sheet: lower.chipload"));
        assert!(sheet.contains("1. lower.nc"));
        assert!(
            app.status
                .as_ref()
                .unwrap()
                .0
                .contains("setup sheet lower-sheet.html")
        );

        // Turned off, and kept so for next time.
        app.set_setup_sheet(false);
        assert!(app.export_gcode(&out.join("again.nc")));
        assert!(out.join("again.nc").exists());
        assert!(!out.join("again-sheet.html").exists());
        let mut again = ChiploadApp::default();
        again.set_settings_dir(Some(dir));
        assert!(!again.setup_sheet);
        std::fs::remove_dir_all(&out).unwrap();
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
    fn tools_are_exported_and_imported_after_a_preview() {
        // docs/specs/library-import-export.md §6
        use super::library_transfer::ImportChoice;
        let (mut old, dir) = app_with_library("export");
        let file = dir.join("backup.json");
        assert!(old.export_library(LibraryKind::Tools, &file));
        assert!(old.library_windows.message.is_some());

        let mut new = ChiploadApp::default();
        new.set_settings_dir(Some(settings("import-preview")));
        assert!(new.preview_library_import(LibraryKind::Tools, &file));
        let pending = new.library_windows.import.as_ref().unwrap();
        assert!(!pending_differs(pending));
        // Cancel changes nothing; Import adds the tool and its default.
        assert!(!new.finish_library_import(ImportChoice::Cancel));
        assert!(new.library.tools.tools.is_empty());
        assert!(new.preview_library_import(LibraryKind::Tools, &file));
        assert!(new.finish_library_import(ImportChoice::Keep));
        assert_eq!(new.library.tools.tools, [upcut()]);
        assert_eq!(
            new.library.tools.default_tool.as_deref(),
            Some("3 mm upcut")
        );
        let (kind, text) = new.library_windows.message.clone().unwrap();
        assert_eq!(kind, LibraryKind::Tools);
        assert!(text.contains("Added: 3 mm upcut"), "{text}");

        // A file with other values: kept, or replaced when asked.
        let mut wider = upcut();
        wider.diameter = 3.175;
        let other = dir.join("other.json");
        save_tools(
            &ToolLibrary {
                tools: vec![wider.clone()],
                ..ToolLibrary::default()
            },
            &other,
        )
        .unwrap();
        assert!(new.preview_library_import(LibraryKind::Tools, &other));
        assert!(pending_differs(
            new.library_windows.import.as_ref().unwrap()
        ));
        assert!(new.finish_library_import(ImportChoice::Keep));
        assert_eq!(new.library.tools.tools, [upcut()]);
        assert!(new.preview_library_import(LibraryKind::Tools, &other));
        assert!(new.finish_library_import(ImportChoice::Replace));
        assert_eq!(new.library.tools.tools, [wider]);

        // The wrong kind of file is refused with a hint.
        assert!(!new.preview_library_import(LibraryKind::Machines, &file));
        let error = new.library_windows.error.clone().unwrap();
        assert!(error.contains("import it as tools"), "{error}");
        assert!(new.library_windows.import.is_none());
    }

    fn pending_differs(pending: &super::library_transfer::PendingImport) -> bool {
        pending.lines().iter().any(|l| l.starts_with("Differs"))
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
        assert!(app.generate_and_wait(), "{:?}", app.report);

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
        assert!(app.generate_and_wait(), "{:?}", app.report);
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
    assert!(job.drawing.shapes.is_empty() && job.drawing.dxfs.is_empty());
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
    assert!(app.generate_and_wait(), "{:?}", app.report);

    // Unsaved changes: a new project asks first.
    app.ask_new();
    assert!(matches!(app.prompt, Some(Prompt::Discard(Pending::New))));
    app.prompt = None;

    // The first DXF has no units: they are asked for, then the drawing comes in and the
    // drawn circle stays.
    assert!(!app.update_from(&corpus("dxf/messy.dxf"), None));
    assert!(matches!(
        app.prompt,
        Some(Prompt::Units(_, UnitsFor::Update(None)))
    ));
    assert!(app.update_from(&corpus("dxf/messy.dxf"), Some(Units::Mm)));
    assert!(app.prompt.is_none());
    assert!(app.shape(1).is_some());
    assert!(app.contours.len() > 1);
    assert_eq!(app.update_labels().0, "Update drawing from DXF…");
    assert!(app.generate_and_wait(), "{:?}", app.report);
}

#[test]
fn copies_get_a_pitch_from_a_gap_and_are_drawn_faded() {
    // docs/specs/copies-grid.md §6
    let mut app = lower_part();
    assert!(app.generate_and_wait(), "{:?}", app.report);
    let one = app.scene.toolpath.clone().unwrap();
    assert!(one.ghost_feeds.is_empty());

    app.edit.as_mut().unwrap().settings.parts[0].copies = Some(chipload_job::Copies {
        columns: 2,
        rows: 2,
        dx: 0.0,
        dy: 0.0,
        order: chipload_job::CopyOrder::ByOperation,
    });
    app.set_copies_gap(1, 5.0);
    let copies = app.edit.as_ref().unwrap().settings.parts[0].copies.unwrap();
    assert!(copies.dx > 5.0 && copies.dy > 5.0, "{copies:?}");
    // Generating again: the copies are cut, the gap leaves them apart.
    app.edit.as_mut().unwrap().settings.stock.x = 1000.0;
    app.edit.as_mut().unwrap().settings.stock.y = 1000.0;
    app.commit();
    assert!(app.generate_and_wait(), "{:?}", app.report);
    let result = app.result.as_ref().unwrap();
    let copy_numbers: BTreeSet<u32> = result.op_spans.iter().map(|s| s.copy).collect();
    assert_eq!(copy_numbers, BTreeSet::from([1, 2, 3, 4]));
    // Copy 1 as before, the other three faded, and plunges marked on copy 1 only.
    let path = app.scene.toolpath.as_ref().unwrap();
    assert_eq!(path.feeds.len(), one.feeds.len());
    assert_eq!(path.ghost_feeds.len(), 3 * one.feeds.len());
    assert_eq!(path.starts.len(), one.starts.len());
    assert_eq!(
        app.edit.as_ref().unwrap().to_job().unwrap().parts[0].copies,
        Some(copies)
    );

    // An overlapping pitch is refused with the size it needs.
    app.edit.as_mut().unwrap().settings.parts[0]
        .copies
        .as_mut()
        .unwrap()
        .dx = 10.0;
    app.commit();
    assert!(!app.generate_and_wait());
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
    assert!(app.generate_and_wait(), "{:?}", app.report);
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
    assert!(app.wait_for_generation(), "{:?}", app.report);
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
    assert!(app.wait_for_generation(), "{:?}", app.report);
    assert!(app.edit.as_ref().unwrap().ops[1].op.starts().is_empty());
    app.undo();
    assert_eq!(app.edit.as_ref().unwrap().ops[1].op.starts().len(), 1);
}

#[test]
fn start_markers_are_only_for_the_focused_operation_in_top_view() {
    let mut app = lower_part();
    assert!(app.generate_and_wait(), "{:?}", app.report);
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
    assert!(app.generate_and_wait(), "{:?}", app.report);
    let before = app.result.as_ref().unwrap().program.moves.len();
    if let Operation::Profile(p) = &mut app.edit.as_mut().unwrap().ops[1].op {
        p.lead = Some(Lead { radius: 3.0 });
    }
    assert!(app.generate_and_wait(), "{:?}", app.report);
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
    assert!(app.generate_and_wait(), "{:?}", app.report);
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
    let drawing_only = crate::view::legend(&app.scene).len();
    assert!(app.generate_and_wait(), "{:?}", app.report);
    assert!(!app.toolpath_stale);
    let generated = crate::view::legend(&app.scene).len();
    assert!(
        generated > drawing_only,
        "the legend explains the toolpath too"
    );
    match &mut app.edit.as_mut().unwrap().ops[0].op {
        Operation::Pocket(p) => p.feed = 900.0,
        _ => panic!("operation 1 is a pocket"),
    }
    app.commit();
    assert!(app.toolpath_stale, "the old toolpath is noticed");
    // It stays, faded, until a new one replaces it; it can't be exported.
    assert!(app.scene.toolpath.as_ref().unwrap().stale);
    assert!(app.result.is_none());
    assert!(app.scene.start_markers.is_empty());
    assert!(app.generate_and_wait(), "{:?}", app.report);
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
    let placement =
        |app: &ChiploadApp| app.edit.as_ref().unwrap().settings.drawing.dxfs[0].placement;
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
    assert!(app.generate_and_wait(), "{:?}", app.report);
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

#[test]
fn a_follow_path_is_added_for_open_and_closed_shapes_and_generated() {
    // docs/specs/follow-path.md §7: the follow-path project, its operations replaced.
    let mut app = ChiploadApp::default();
    assert!(app.open(&corpus("jobs/v17/follow_path.chipload")));
    let edit = app.edit.as_mut().unwrap();
    edit.remove(1);
    edit.remove(0);
    let open: Vec<usize> = app
        .contours
        .iter()
        .filter(|c| !c.closed)
        .map(|c| c.id)
        .collect();
    let closed = app.contours.iter().find(|c| c.closed).unwrap().id;
    assert_eq!(open.len(), 2);
    let mut ids = open.clone();
    ids.push(closed);
    select(&mut app, &ids);
    let i = app.add_operation(OpKind::Path).unwrap();
    let edit = app.edit.as_ref().unwrap();
    let Operation::Path(p) = &edit.ops[i].op else {
        panic!("a follow path");
    };
    assert_eq!(p.name, "Follow path 1");
    assert_eq!(p.start_depth, 0.0);
    // The open shapes are in its geometry (a profile would leave them out).
    assert_eq!(p.select.shapes.as_ref().map(Vec::len), Some(3));
    assert!(app.generate_and_wait(), "{:?}", app.report);
    // Every shape is cut: the open paths' ends are reached.
    let moves = &app.result.as_ref().unwrap().program.moves;
    for end in [(50.0, 40.0), (150.0, 60.0), (60.0, 110.0), (140.0, 110.0)] {
        assert!(
            moves
                .iter()
                .any(|m| m.x == Some(end.0) && m.y == Some(end.1)),
            "{end:?} is never reached"
        );
    }
}

#[test]
fn holes_take_the_tools_method_and_generate() {
    // docs/specs/holes.md §7: the holes project, its operations replaced.
    let mut app = ChiploadApp::default();
    assert!(app.open(&corpus("jobs/v18/holes.chipload")));
    let edit = app.edit.as_mut().unwrap();
    edit.remove(1);
    edit.remove(0);
    let circles: Vec<usize> = app
        .contours
        .iter()
        .filter(|c| chipload_toolpath::holes::circle_of(c).is_some())
        .map(|c| c.id)
        .collect();
    assert_eq!(circles.len(), 4);
    // The two 8 mm holes, with the drill (tool 2).
    select(&mut app, &circles[..2]);
    let i = app.add_operation(OpKind::Holes).unwrap();
    let edit = app.edit.as_mut().unwrap();
    edit.ops[i].op.set_tool(2);
    crate::editor::fit_hole_method(&mut edit.ops[i].op, &edit.settings.tools);
    let Operation::Holes(h) = &edit.ops[i].op else {
        panic!("holes");
    };
    assert_eq!(h.name, "Holes 1");
    assert_eq!(h.method, HoleMethod::Drill { peck: None });
    // Back to the endmill: a bore with its defaults.
    edit.ops[i].op.set_tool(1);
    crate::editor::fit_hole_method(&mut edit.ops[i].op, &edit.settings.tools);
    let Operation::Holes(h) = &edit.ops[i].op else {
        panic!("holes");
    };
    assert!(matches!(h.method, HoleMethod::Bore { step_down, .. } if step_down == 3.0));
    assert!(app.generate_and_wait(), "{:?}", app.report);
    let moves = &app.result.as_ref().unwrap().program.moves;
    assert!(
        moves
            .iter()
            .any(|m| m.z == Some(-crate::editor::DEFAULT_DEPTH))
    );
}

#[test]
fn dogbones_are_set_on_a_profile_and_generated() {
    // docs/specs/dogbones.md §7: the slot of the dogbones project, as round corners first.
    let mut app = ChiploadApp::default();
    assert!(app.open(&corpus("jobs/v18/dogbones.chipload")));
    let edit = app.edit.as_mut().unwrap();
    edit.remove(2);
    edit.remove(0);
    let Operation::Profile(p) = &mut edit.ops[0].op else {
        panic!("the slot");
    };
    p.corners = None;
    assert!(app.generate_and_wait(), "{:?}", app.report);
    let round = app.result.as_ref().unwrap().program.moves.len();
    let edit = app.edit.as_mut().unwrap();
    let Operation::Profile(p) = &mut edit.ops[0].op else {
        panic!("the slot");
    };
    p.corners = Some(chipload_job::Corners::Dogbone {
        max_angle: chipload_job::DEFAULT_CORNER_ANGLE_RAD,
    });
    assert!(app.generate_and_wait(), "{:?}", app.report);
    // Two more moves per corner, at every depth pass.
    let moves = app.result.as_ref().unwrap().program.moves.len();
    assert_eq!(moves, round + 4 * 2 * 3);
}

#[test]
fn the_simulation_follows_the_toggle_and_the_toolpath() {
    // docs/specs/simulation.md §7 (GUI).
    let ctx = egui::Context::default();
    let mut app = ChiploadApp::default();
    assert!(app.open(&corpus("jobs/v18/holes.chipload")));
    // Switched off: nothing computed.
    app.set_show_simulation(false);
    assert!(app.generate_and_wait(), "{:?}", app.report);
    assert!(!app.simulating());
    assert!(app.scene.sim.is_none());
    app.set_show_simulation(true);
    assert!(app.wait_for_simulation(&ctx));
    let sim = app.scene.sim.as_ref().unwrap();
    assert!(sim.surface.columns * sim.surface.rows <= simulation::SURFACE_CELLS);
    assert!(sim.surface.columns > 1 && sim.surface.rows > 1);
    // The 3D surface: a vertex per cell, two triangles per quad.
    let view = View {
        tilt: 0.5,
        ..View::default()
    };
    let mesh = sim.surface.mesh(&view, false);
    assert_eq!(mesh.vertices.len(), sim.surface.columns * sim.surface.rows);
    assert_eq!(
        mesh.indices.len(),
        6 * (sim.surface.columns - 1) * (sim.surface.rows - 1)
    );
    assert!(
        app.report
            .lines
            .iter()
            .any(|l| l.text.starts_with("Simulation:"))
    );
    // A new toolpath replaces it; switching off drops it.
    assert!(app.generate_and_wait());
    assert!(app.scene.sim.is_none());
    assert!(app.wait_for_simulation(&ctx));
    assert!(app.scene.sim.is_some());
    app.set_show_simulation(false);
    assert!(app.scene.sim.is_none() && app.simulation.is_none());
}

#[test]
fn playback_steps_through_the_program_and_shows_the_stock_so_far() {
    // docs/specs/playback.md §8 (GUI).
    let ctx = egui::Context::default();
    let mut app = ChiploadApp::default();
    assert!(app.open(&corpus("jobs/v18/dogbones.chipload")));
    assert!(app.generate_and_wait(), "{:?}", app.report);
    let total = app.playback.as_ref().unwrap().total();
    assert!(total > 0.0);
    // At the end: the canvas as without playback.
    app.sync_playback(&ctx);
    assert!(app.scene.progress.is_none());
    // One mark per operation (numbered) and none for tool changes (one tool).
    let labels: Vec<String> = app
        .playback
        .as_ref()
        .unwrap()
        .marks
        .iter()
        .map(|m| m.1.clone())
        .collect();
    assert_eq!(labels, vec!["1", "2", "3"]);
    let pb = app.playback.as_mut().unwrap();
    pb.set_time(0.0);
    // Moves that take no time (the first rapids, before every axis is known) are done at 0.
    let at_start = pb.done();
    pb.step_move(true);
    assert!(pb.done() > at_start);
    pb.step_move(false);
    assert_eq!(pb.done(), at_start);
    // Marks: the first operation's is at 0; on to the second's, the third's, and back.
    pb.set_time(0.0);
    assert_eq!(pb.marks[0].0, 0.0);
    pb.step_mark(true);
    assert_eq!(pb.time, pb.marks[1].0);
    pb.step_mark(true);
    assert_eq!(pb.time, pb.marks[2].0);
    pb.step_mark(false);
    assert_eq!(pb.time, pb.marks[1].0);
    // Half way: progress with the tool marker.
    pb.set_time(total / 2.0);
    app.sync_playback(&ctx);
    let progress = app.scene.progress.clone().unwrap();
    assert!(progress.tool_at.is_some());
    assert_eq!(progress.tool.radius, 3.0);
    assert!(
        app.playback
            .as_ref()
            .unwrap()
            .readout(None)
            .contains(" of ")
    );
    // With the simulation: the stock half way, then the full image back at the end.
    app.set_show_simulation(true);
    assert!(app.wait_for_simulation(&ctx));
    app.sync_playback(&ctx);
    let done = app.playback.as_ref().unwrap().done();
    let half = app.scene.sim.as_ref().unwrap().surface.heights.clone();
    let numbers = app.simulation.as_ref().unwrap().numbers.clone();
    let expected = app
        .sim_history
        .as_ref()
        .unwrap()
        .after(done, &numbers)
        .heights;
    assert_eq!(half, expected);
    let full = app.sim_full_layer.as_ref().unwrap().surface.heights.clone();
    assert_ne!(half, full, "less is cut half way");
    let pb = app.playback.as_mut().unwrap();
    pb.set_time(pb.total());
    app.sync_playback(&ctx);
    assert!(app.scene.progress.is_none());
    assert_eq!(app.scene.sim.as_ref().unwrap().surface.heights, full);
    // Play runs to the end and stops.
    let pb = app.playback.as_mut().unwrap();
    let start = Instant::now();
    pb.play(start);
    assert_eq!(pb.time, 0.0, "play from the end starts over");
    pb.speed = 100.0;
    pb.tick(start + Duration::from_secs_f64(total / 100.0 + 1.0));
    assert!(pb.at_end() && !pb.playing());
    // A new generation of the same program keeps a time part way.
    pb.set_time(total / 3.0);
    assert!(app.generate_and_wait());
    assert_eq!(app.playback.as_ref().unwrap().time, total / 3.0);
}

#[test]
fn a_project_with_a_removed_strategy_opens_and_is_switched_to_rings() {
    // docs/decisions/0017-scope.md: it loads unchanged, is refused, and one change fixes it.
    let mut app = ChiploadApp::default();
    assert!(app.open(&corpus("jobs/v7/trochoidal.chipload")));
    let edit = app.edit.as_ref().unwrap();
    let job = edit.to_job().unwrap();
    let problems = chipload_pipeline::job_problems(&job);
    assert!(
        problems.iter().any(|p| p.path == "operations.0.strategy"),
        "{problems:?}"
    );
    assert!(!app.generate_and_wait());
    let edit = app.edit.as_mut().unwrap();
    let Operation::Pocket(p) = &mut edit.ops[0].op else {
        panic!("a pocket");
    };
    p.strategy = chipload_job::PocketStrategy::Rings {};
    assert!(app.generate_and_wait(), "{:?}", app.report);
}

#[test]
fn a_rest_pocket_generates_and_a_smaller_rest_tool_is_flagged_on_its_field() {
    // docs/specs/rest-machining.md §5, §6.
    let mut app = ChiploadApp::default();
    assert!(app.open(&corpus("jobs/v18/rest.chipload")));
    assert!(app.generate_and_wait(), "{:?}", app.report);
    let edit = app.edit.as_mut().unwrap();
    let Operation::Pocket(p) = &mut edit.ops[0].op else {
        panic!("a pocket");
    };
    p.rest = Some(chipload_job::Rest { after_tool: 2 });
    let job = edit.to_job().unwrap();
    let problems = chipload_pipeline::job_problems(&job);
    assert!(
        problems
            .iter()
            .any(|p| p.path == "operations.0.rest" && p.short.contains("needs a tool larger")),
        "{problems:?}"
    );
    assert!(!app.generate_and_wait());
}

#[test]
fn add_rest_pocket_inserts_a_rest_pocket_after_the_pocket_and_undo_removes_it() {
    // docs/specs/rest-machining.md §10.
    let mut app = ChiploadApp::default();
    assert!(app.open(&corpus("jobs/v18/rest.chipload")));
    app.edit.as_mut().unwrap().ops.truncate(1);
    app.commit();
    app.add_rest_pocket(0, 2);
    let edit = app.edit.as_ref().unwrap();
    assert_eq!(edit.ops.len(), 2);
    assert_eq!(app.focus, Focus::Op(1));
    let Operation::Pocket(p) = &edit.ops[1].op else {
        panic!("a pocket");
    };
    assert_eq!((p.tool, p.rest.map(|r| r.after_tool)), (2, Some(1)));
    assert!(app.generate_and_wait(), "{:?}", app.report);
    // Not for a larger tool: an error, nothing added.
    app.add_rest_pocket(1, 1);
    assert_eq!(app.edit.as_ref().unwrap().ops.len(), 2);
    app.undo();
    assert_eq!(app.edit.as_ref().unwrap().ops.len(), 1);
}

/// Run the keyboard shortcuts for one frame with `key` pressed.
fn press(app: &mut ChiploadApp, key: egui::Key) {
    press_with(app, egui::Modifiers::NONE, key);
}

fn press_with(app: &mut ChiploadApp, modifiers: egui::Modifiers, key: egui::Key) {
    let ctx = egui::Context::default();
    let input = egui::RawInput {
        events: vec![egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers,
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

/// Draw the canvas and the status bar for one frame.
fn draw_canvas(app: &mut ChiploadApp) {
    let ctx = egui::Context::default();
    let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
        app.status_bar(ui);
        app.canvas(ui);
    });
    output.textures_delta.clear();
}

/// Draw the left sidebar for one frame.
fn draw_sidebar(app: &mut ChiploadApp) {
    let ctx = egui::Context::default();
    let mut output = ctx.run_ui(egui::RawInput::default(), |ui| app.sidebar(ui));
    // The font atlas a first frame makes isn't painted anywhere here.
    output.textures_delta.clear();
}

#[test]
fn the_sidebar_shows_operations_and_shapes_apart() {
    // docs/specs/sidebar-sections.md
    let mut app = ChiploadApp::default();
    draw_sidebar(&mut app);
    let mut app = lower_part();
    draw_sidebar(&mut app);
    app.click(None, false);
    draw_sidebar(&mut app);
    // Every shape is in exactly one group of the Browser's Drawing folder.
    let groups = app.drawing_groups();
    let total: usize = groups.iter().map(|g| g.shapes).sum();
    let shapes = app.edit.as_ref().unwrap().settings.drawing.shapes.len();
    assert_eq!(total, shapes, "{groups:?}");
    let mut app = drawn_part();
    app.click(Some(contour_of(&app, 2)), false);
    draw_sidebar(&mut app);
    // The selection and the drawn shape's editor are in Properties
    // (docs/specs/browser-and-layout.md §4).
    draw_properties(&mut app);
    assert!(app.shape_draft.is_some());
}

// -- generating in the background (docs/specs/background-generation.md)

/// Change operation 1's feed and record it as an edit.
fn edit_feed(app: &mut ChiploadApp, feed: f64) {
    match &mut app.edit.as_mut().unwrap().ops[0].op {
        Operation::Pocket(p) => p.feed = feed,
        _ => panic!("operation 1 is a pocket"),
    }
    app.commit();
}

#[test]
fn a_result_for_a_project_that_changed_meanwhile_is_dropped() {
    let mut app = lower_part();
    assert!(app.generate());
    assert!(app.generating());
    // Changed but not yet recorded (a value being dragged): the run isn't cancelled, but
    // its result is for the project as it was.
    match &mut app.edit.as_mut().unwrap().ops[0].op {
        Operation::Pocket(p) => p.feed = 777.0,
        _ => panic!("operation 1 is a pocket"),
    }
    assert!(!app.wait_for_generation());
    assert!(app.result.is_none() && app.scene.toolpath.is_none());
    // An edit cancels the run in progress.
    assert!(app.generate());
    edit_feed(&mut app, 650.0);
    assert!(!app.generating());
}

#[test]
fn the_live_preview_generates_once_the_project_is_quiet() {
    let mut app = lower_part();
    assert!(app.generate_and_wait(), "{:?}", app.report);
    edit_feed(&mut app, 650.0);
    let changed = app.changed_at.unwrap();
    assert!(app.toolpath_stale && app.result.is_none());
    // Not yet: it waits for the rest of the quiet time.
    let wait = app.live_preview(changed + Duration::from_millis(200));
    assert_eq!(wait, Some(Duration::from_millis(300)));
    assert!(!app.generating());
    // Then it runs, quietly.
    assert_eq!(app.live_preview(changed + generation::QUIET_TIME), None);
    assert!(app.generating());
    assert!(app.wait_for_generation(), "{:?}", app.report);
    assert!(!app.toolpath_stale && app.result.is_some());
    assert!(!app.scene.toolpath.as_ref().unwrap().stale);
    // Switched off, an edit waits for Generate.
    app.auto_generate = false;
    edit_feed(&mut app, 700.0);
    assert_eq!(
        app.live_preview(Instant::now() + Duration::from_secs(5)),
        None
    );
    assert!(!app.generating() && app.toolpath_stale);
}

#[test]
fn the_live_preview_waits_while_the_project_has_problems() {
    // docs/specs/live-problems.md decision 4: the Problems strip says why.
    let mut app = lower_part();
    match &mut app.edit.as_mut().unwrap().ops[0].op {
        Operation::Pocket(p) => p.feed = 0.0,
        _ => panic!("operation 1 is a pocket"),
    }
    app.commit();
    let changed = app.changed_at.unwrap();
    assert_eq!(app.live_preview(changed + generation::QUIET_TIME), None);
    assert!(!app.generating());
    assert!(
        app.changed_at.is_none(),
        "not tried again until the next edit"
    );
    assert!(app.status.is_none(), "no red status-bar message");
    // Generate (F5) says it loudly.
    assert!(!app.generate());
    assert!(app.status.is_some());
}

#[test]
fn a_panic_while_generating_becomes_an_error_and_keeps_the_project() {
    let mut app = lower_part();
    let project = app.edit.clone();
    app.runner = |_, _| panic!("a geometry library fell over");
    assert!(!app.generate_and_wait());
    let text = format!("{:?}", app.report);
    assert!(text.contains("failed unexpectedly"), "{text}");
    assert!(text.contains("a geometry library fell over"), "{text}");
    assert_eq!(app.edit, project);
    assert!(!app.generating());
}

#[test]
fn cancel_stops_the_run_and_export_waits_for_it() {
    let mut app = lower_part();
    assert!(app.generate());
    app.export_when_ready();
    assert!(app.export_when_done);
    app.cancel_generation();
    assert!(!app.generating() && !app.export_when_done);
    assert!(app.result.is_none());
    let status = app.status.as_ref().map(|(text, _)| text.clone());
    assert_eq!(status.as_deref(), Some("Generation cancelled"));
}

// -- recovery copies (docs/specs/autosave-and-backup.md)

/// An app with its own settings folder (and so a recovery session).
fn with_settings(name: &str) -> (ChiploadApp, PathBuf) {
    let dir = temp_dir(name);
    let _ = std::fs::remove_dir_all(dir.join("recovery"));
    let mut app = ChiploadApp::default();
    app.set_settings_dir(Some(dir.clone()));
    (app, dir)
}

fn recovery_copies(dir: &Path) -> usize {
    std::fs::read_dir(dir.join("recovery"))
        .map(|d| {
            d.filter_map(Result::ok)
                .filter(|e| e.path().extension().is_some_and(|x| x == "chipload"))
                .count()
        })
        .unwrap_or(0)
}

#[test]
fn an_edit_is_copied_for_recovery_after_a_pause_and_saving_removes_it() {
    let (mut app, dir) = with_settings("recovery-edit");
    assert!(app.leftovers.is_empty());
    assert!(app.open(&corpus("jobs/v7/leads.chipload")));
    edit_feed(&mut app, 650.0);
    let due = app.recovery_due.unwrap();
    assert_eq!(
        app.recovery_tick(due + Duration::from_secs(1)),
        Some(Duration::from_secs(1))
    );
    assert_eq!(recovery_copies(&dir), 0, "not before the pause");
    assert_eq!(app.recovery_tick(due + recovery::RECOVERY_DELAY), None);
    assert_eq!(recovery_copies(&dir), 1);
    // An edit that leaves the project incomplete keeps the last complete copy.
    edit_feed(&mut app, 0.0);
    app.write_recovery();
    assert_eq!(recovery_copies(&dir), 1);
    edit_feed(&mut app, 650.0);
    let saved = dir.join("saved.chipload");
    assert!(app.save_project(&saved));
    assert_eq!(recovery_copies(&dir), 0, "saved: nothing to recover");
    // Opening another project drops the copy of discarded changes.
    edit_feed(&mut app, 700.0);
    app.write_recovery();
    assert_eq!(recovery_copies(&dir), 1);
    assert!(app.open(&corpus("jobs/v7/leads.chipload")));
    assert_eq!(recovery_copies(&dir), 0);
}

#[test]
fn work_left_by_a_crashed_window_is_offered_and_reopened_unsaved() {
    let (mut crashed, dir) = with_settings("recovery-crash");
    let project = dir.join("leads.chipload");
    std::fs::copy(corpus("jobs/v7/leads.chipload"), &project).unwrap();
    assert!(crashed.open(&project));
    edit_feed(&mut crashed, 650.0);
    crashed.write_recovery();
    // Not offered to another window while the first is still running.
    let mut other = ChiploadApp::default();
    other.set_settings_dir(Some(dir.clone()));
    assert!(other.leftovers.is_empty());
    drop(other);
    // The first window goes away without saving or clearing (its lock is released).
    crashed.recovery = None;
    let mut app = ChiploadApp::default();
    app.set_settings_dir(Some(dir.clone()));
    assert_eq!(app.leftovers.len(), 1);
    assert_eq!(
        app.leftovers[0].project_path.as_deref(),
        Some(project.as_path())
    );
    assert!(app.open_leftover(0));
    assert!(app.leftovers.is_empty());
    assert!(app.dirty(), "recovered work is unsaved");
    assert_eq!(app.job_path.as_deref(), Some(project.as_path()));
    match &app.edit.as_ref().unwrap().ops[0].op {
        Operation::Pocket(p) => assert_eq!(p.feed, 650.0),
        _ => panic!("operation 1 is a pocket"),
    }
    // The left-over is gone; this window's own copy holds the work now.
    assert_eq!(recovery_copies(&dir), 1);
    assert!(app.save_project(&project));
    assert_eq!(recovery_copies(&dir), 0);
    assert!(dir.join("leads.chipload.bak").exists());
}

#[test]
fn a_left_over_copy_can_be_deleted() {
    let (mut crashed, dir) = with_settings("recovery-delete");
    assert!(crashed.open(&corpus("jobs/v7/leads.chipload")));
    edit_feed(&mut crashed, 650.0);
    crashed.write_recovery();
    crashed.recovery = None;
    let mut app = ChiploadApp::default();
    app.set_settings_dir(Some(dir.clone()));
    assert_eq!(app.leftovers.len(), 1);
    app.delete_leftover(0);
    assert!(app.leftovers.is_empty());
    assert_eq!(recovery_copies(&dir), 0);
}

// -- post-processors (docs/specs/user-posts.md)

#[test]
fn a_user_post_is_chosen_copied_in_and_updated() {
    let (mut app, dir) = with_settings("user-post");
    let posts = dir.join("posts");
    std::fs::create_dir_all(&posts).unwrap();
    let config = "schema_version = 1\nname = \"Shop\"\ntemplate = \"shop.nc.j2\"\n\
                  spindle_dwell_s = 2\n";
    std::fs::write(posts.join("shop.toml"), config).unwrap();
    let template = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../chipload-post/templates/grbl.nc.j2"),
    )
    .unwrap();
    std::fs::write(posts.join("shop.nc.j2"), &template).unwrap();
    assert!(app.open(&corpus("jobs/v7/leads.chipload")));
    assert!(app.choose_project_post("shop"));
    let job = app.edit.as_ref().unwrap().to_job().unwrap();
    assert_eq!(job.machine.post, "shop");
    assert!(job.machine.custom_post.is_some());
    assert!(app.generate_and_wait(), "{:?}", app.report);
    let state = |app: &ChiploadApp| {
        let job = app.edit.as_ref().unwrap().to_job().unwrap();
        chipload_pipeline::posts::copy_state(&job, Some(&posts))
    };
    assert_eq!(state(&app), chipload_pipeline::posts::CopyState::Same);
    std::fs::write(
        posts.join("shop.nc.j2"),
        template.replace("Chipload", "Shop"),
    )
    .unwrap();
    assert_eq!(state(&app), chipload_pipeline::posts::CopyState::Changed);
    assert!(app.choose_project_post("shop"), "Update copies it again");
    assert_eq!(state(&app), chipload_pipeline::posts::CopyState::Same);
    assert!(!app.choose_project_post("nope"));
    assert!(app.choose_project_post("grbl"));
    let job = app.edit.as_ref().unwrap().to_job().unwrap();
    assert!(job.machine.custom_post.is_none());
}

// -- problems while editing (docs/specs/live-problems.md)

fn problem_paths(app: &ChiploadApp) -> Vec<String> {
    app.problems.iter().map(|p| p.path.clone()).collect()
}

#[test]
fn a_bad_value_is_flagged_at_its_field_once_the_edit_is_finished() {
    let mut app = lower_part();
    assert!(app.problems.is_empty(), "{:?}", app.problems);
    match &mut app.edit.as_mut().unwrap().ops[0].op {
        Operation::Pocket(p) => p.feed = 0.0,
        _ => panic!("operation 1 is a pocket"),
    }
    assert!(
        app.problems.is_empty(),
        "not while the value is being edited"
    );
    app.commit();
    assert_eq!(problem_paths(&app), ["operations.0.feed"]);
    let fields = app.form_problem_map(Focus::Op(0));
    assert_eq!(
        fields.get("feed").map(String::as_str),
        Some("Must be greater than 0, got 0")
    );
    assert!(app.form_problem_map(Focus::Op(1)).is_empty());
    assert!(app.form_problem_map(Focus::Job).is_empty());
    assert_eq!(
        problems::plain(&app.problems[0]),
        "Feed: must be greater than 0, got 0"
    );
    // The forms, cards and strip draw with it.
    app.focus = Focus::Op(0);
    draw_sidebar(&mut app);
    draw_properties(&mut app);
    // Fixed: gone.
    edit_feed(&mut app, 900.0);
    assert!(app.problems.is_empty(), "{:?}", app.problems);
}

#[test]
fn a_job_setting_problem_marks_the_job_and_undo_brings_it_back() {
    let mut app = lower_part();
    app.edit.as_mut().unwrap().settings.machine.safe_z = -2.0;
    app.commit();
    assert_eq!(problem_paths(&app), ["machine.safe_z"]);
    assert_eq!(app.job_problems().count(), 1);
    assert_eq!(
        problems::plain(&app.problems[0]),
        "Safe Z: must be above the stock top (Z 0)"
    );
    let fields = app.form_problem_map(Focus::Job);
    assert_eq!(
        fields.get("machine.safe_z").map(String::as_str),
        Some("Must be above the stock top (Z 0)")
    );
    app.focus = Focus::Job;
    draw_sidebar(&mut app);
    draw_properties(&mut app);
    app.undo();
    assert!(app.problems.is_empty());
    app.redo();
    assert_eq!(problem_paths(&app), ["machine.safe_z"]);
}

#[test]
fn an_operation_without_shapes_is_a_problem_of_the_operation() {
    let mut app = lower_part();
    app.edit.as_mut().unwrap().ops[1].op.select_mut().shapes = Some(Vec::new());
    app.commit();
    assert_eq!(problem_paths(&app), ["operations.1.select"]);
    assert_eq!(
        problems::plain(&app.problems[0]),
        "Geometry: no geometry: select contours, then Use selection"
    );
}

/// Draw the properties panel for one frame.
fn draw_properties(app: &mut ChiploadApp) {
    let ctx = egui::Context::default();
    let mut output = ctx.run_ui(egui::RawInput::default(), |ui| app.properties_panel(ui));
    output.textures_delta.clear();
}

// -- cycle time (docs/specs/cycle-time.md)

#[test]
fn the_report_and_the_cards_show_the_estimated_time() {
    let mut app = lower_part();
    assert!(app.generate_and_wait(), "{:?}", app.report);
    let text = format!("{:?}", app.report);
    assert!(text.contains("Estimated time: "), "{text}");
    let estimate = &app.result.as_ref().unwrap().estimate;
    assert_eq!(estimate.operations.len(), 2);
    draw_sidebar(&mut app);
    // Out of date: the times go with the result.
    edit_feed(&mut app, 650.0);
    assert!(app.result.is_none());
    draw_sidebar(&mut app);
}

mod keyboard_shortcuts {
    use super::super::shortcuts::{BINDINGS, OTHERS, keys_text, shortcut_rows, with_keys};
    use super::*;

    #[test]
    fn every_shortcut_is_listed_once_and_shift_ones_come_first() {
        // docs/specs/small-items.md §1
        let listed: Vec<String> = shortcut_rows()
            .into_iter()
            .flat_map(|(_, rows)| rows.into_iter().map(|(keys, _)| keys))
            .collect();
        for b in BINDINGS {
            assert!(listed.contains(&keys_text(&b.keys)), "{:?}", b.action);
        }
        for (i, a) in BINDINGS.iter().enumerate() {
            for later in &BINDINGS[i + 1..] {
                assert_ne!(a.keys, later.keys, "{:?} twice", a.action);
                // egui takes Ctrl+Shift+Z for Ctrl+Z too: the one with more modifiers first.
                let shadows = later.keys.logical_key == a.keys.logical_key
                    && a.keys.modifiers.contains(later.keys.modifiers)
                    && a.keys.modifiers != later.keys.modifiers;
                let shadowed = later.keys.logical_key == a.keys.logical_key
                    && later.keys.modifiers.contains(a.keys.modifiers)
                    && a.keys.modifiers != later.keys.modifiers;
                assert!(
                    shadows || !shadowed,
                    "{:?} must come before {:?}",
                    later.action,
                    a.action
                );
            }
        }
        // The keys handled outside the table are listed too.
        for keys in ["Delete", "Escape", "Enter, double-click", "Backspace"] {
            assert!(OTHERS.iter().any(|(_, k, _)| *k == keys), "{keys}");
        }
        assert_eq!(
            with_keys("Undo", super::super::shortcuts::Action::Undo),
            "Undo  (Ctrl+Z)"
        );
    }

    #[test]
    fn f1_opens_the_list_and_shortcuts_act() {
        let mut app = ChiploadApp::default();
        press(&mut app, egui::Key::F1);
        assert!(app.shortcuts_open);
        let mut app = lower_part();
        let before = app.edit.as_ref().unwrap().ops.len();
        press_with(&mut app, egui::Modifiers::COMMAND, egui::Key::Z);
        assert_eq!(
            app.edit.as_ref().unwrap().ops.len(),
            before - 1,
            "Ctrl+Z undid"
        );
        press_with(
            &mut app,
            egui::Modifiers::COMMAND.plus(egui::Modifiers::SHIFT),
            egui::Key::Z,
        );
        assert_eq!(
            app.edit.as_ref().unwrap().ops.len(),
            before,
            "Ctrl+Shift+Z redid, not undid"
        );
    }
}

#[test]
fn dropped_files_open_or_import_after_asking() {
    // docs/specs/small-items.md §2, drawing-sources.md §3
    let mut app = ChiploadApp::default();
    assert!(app.drop_files(&[corpus("dxf/messy_2.dxf")]));
    assert!(!app.contours.is_empty(), "a new project from the DXF");
    // Unsaved changes: ask first.
    assert!(app.dirty());
    assert!(app.drop_files(&[corpus("jobs/v11/two_tools.chipload")]));
    assert!(matches!(
        app.prompt,
        Some(Prompt::Discard(Pending::Open(_)))
    ));
    // Nothing else is taken.
    let mut fresh = ChiploadApp::default();
    assert!(!fresh.drop_files(&[PathBuf::from("notes.txt")]));
    assert!(!fresh.drop_files(&[
        corpus("jobs/v11/two_tools.chipload"),
        corpus("dxf/messy_2.dxf")
    ]));
    assert!(fresh.edit.is_none());
    assert!(fresh.drop_files(&[corpus("jobs/v11/two_tools.chipload")]));
    assert_eq!(fresh.edit.as_ref().unwrap().ops.len(), 2);
}

#[test]
fn dropped_dxfs_and_pictures_are_added_one_at_a_time() {
    // docs/specs/drawing-sources.md §3: two DXFs make a project of two.
    let mut app = ChiploadApp::default();
    assert!(app.drop_files(&[corpus("dxf/simple_part.dxf"), corpus("dxf/simple_part.dxf")]));
    let dxfs = |app: &ChiploadApp| app.edit.as_ref().unwrap().settings.drawing.dxfs.len();
    assert_eq!(dxfs(&app), 2);
    // A DXF without units asks, and is added once answered.
    assert!(app.drop_files(&[corpus("dxf/messy.dxf")]));
    assert!(matches!(app.prompt, Some(Prompt::Units(_, UnitsFor::Add))));
    assert!(app.add_dxf_from(&corpus("dxf/messy.dxf"), Some(Units::Mm)));
    assert_eq!(dxfs(&app), 3);
    // Pictures open their window in turn: the second waits for the first.
    assert!(app.drop_files(&[corpus("images/logo.png"), corpus("images/strokes.png")]));
    assert_eq!(app.image_dialog.as_ref().unwrap().draft.file, "logo.png");
    assert_eq!(app.drops.len(), 1);
    assert!(app.apply_image());
    app.next_drop();
    assert_eq!(app.image_dialog.as_ref().unwrap().draft.file, "strokes.png");
    assert!(app.drops.is_empty());
}

#[test]
fn measuring_changes_nothing_in_the_project() {
    // docs/specs/small-items.md §3
    let mut app = drawn_part();
    let before = app.edit.clone();
    press(&mut app, egui::Key::M);
    assert!(app.measure.is_some());
    app.measure_click((10.0, 10.0));
    app.measure_click((40.0, 50.0));
    assert_eq!(
        app.measure_status("").unwrap(),
        "L 50.000  ΔX 30.000  ΔY 40.000  53.1°  · click to measure again, Esc or M ends"
    );
    app.commit();
    assert_eq!(app.edit, before);
    assert!(app.undo.is_empty(), "no undo step");
    // Esc ends it; so does starting to draw.
    press(&mut app, egui::Key::Escape);
    assert!(app.measure.is_none());
    app.toggle_measure();
    app.start_drawing(Tool::Circle);
    assert!(app.measure.is_none() && app.sketch.is_some());
    // From 3D it goes to the top view.
    app.cancel_drawing();
    app.view_3d();
    app.toggle_measure();
    assert!(!app.three_d);
}

mod copy_paste_mirror {
    // docs/specs/small-items.md §4
    use super::*;
    use chipload_job::edit::Mirror;

    /// One frame with these input events through the shortcuts; returns what was put on the
    /// system clipboard.
    fn events(
        app: &mut ChiploadApp,
        events: Vec<egui::Event>,
        modifiers: egui::Modifiers,
    ) -> String {
        let ctx = egui::Context::default();
        let mut all = vec![egui::Event::ModifiersChanged(modifiers)];
        all.extend(events);
        let input = egui::RawInput {
            events: all,
            ..Default::default()
        };
        ctx.begin_pass(input);
        app.shortcuts(&ctx);
        let mut output = ctx.end_pass();
        output.textures_delta.clear();
        output
            .platform_output
            .commands
            .iter()
            .find_map(|c| match c {
                egui::OutputCommand::CopyText(text) => Some(text.clone()),
                _ => None,
            })
            .unwrap_or_default()
    }

    #[test]
    fn copy_and_paste_at_the_cursor_is_one_undo_step() {
        let mut app = drawn_part();
        let shapes = app.edit.as_ref().unwrap().settings.drawing.shapes.len();
        app.click(Some(contour_of(&app, 2)), false);
        // Ctrl+C reaches egui as a Copy event; the text goes to the system clipboard.
        let text = events(&mut app, vec![egui::Event::Copy], egui::Modifiers::COMMAND);
        assert!(text.starts_with("{\"chipload_shapes\":1"), "{text}");
        app.commit();
        app.cursor = Some((200.0, 150.0));
        // Ctrl+V: a Paste event with the clipboard's text.
        events(
            &mut app,
            vec![egui::Event::Paste(text)],
            egui::Modifiers::COMMAND,
        );
        app.commit();
        let drawing = &app.edit.as_ref().unwrap().settings.drawing;
        assert_eq!(drawing.shapes.len(), shapes + 1);
        let new = drawing.shapes.last().unwrap();
        let chipload_job::Geometry::Circle { center, .. } = new.geometry else {
            panic!("a circle was copied")
        };
        assert!(
            (center[0] - 200.0).abs() < 1e-6 && (center[1] - 150.0).abs() < 1e-6,
            "{center:?}"
        );
        assert_eq!(
            app.selection,
            CanvasSelection::Shapes([new.id].into_iter().collect()),
            "the pasted shapes are selected"
        );
        app.undo();
        assert_eq!(
            app.edit.as_ref().unwrap().settings.drawing.shapes.len(),
            shapes
        );
        // Other text on the clipboard pastes nothing.
        events(
            &mut app,
            vec![egui::Event::Paste("hello".into())],
            egui::Modifiers::COMMAND,
        );
        assert_eq!(
            app.edit.as_ref().unwrap().settings.drawing.shapes.len(),
            shapes
        );
        // The Edit menu pastes the last copy 10 mm up and right (no cursor on the canvas).
        app.cursor = None;
        assert!(app.paste(None));
        assert_eq!(
            app.edit.as_ref().unwrap().settings.drawing.shapes.len(),
            shapes + 1
        );
    }

    #[test]
    fn ctrl_shift_c_still_copies_the_selector() {
        let mut app = drawn_part();
        app.click(Some(contour_of(&app, 2)), false);
        let modifiers = egui::Modifiers::COMMAND.plus(egui::Modifiers::SHIFT);
        let text = events(&mut app, vec![egui::Event::Copy], modifiers);
        assert_eq!(text, app.selector_json().unwrap());
    }

    #[test]
    fn mirrored_shapes_keep_their_ids_and_operations_follow() {
        let mut app = drawn_part();
        let before = app.edit.clone().unwrap();
        app.click(Some(contour_of(&app, 1)), false);
        app.click(Some(contour_of(&app, 3)), true);
        assert!(app.mirror_selected(Mirror::LeftRight));
        app.commit();
        let after = app.edit.as_ref().unwrap();
        assert_ne!(after.settings.drawing, before.settings.drawing);
        let ids =
            |e: &EditJob| -> Vec<u64> { e.settings.drawing.shapes.iter().map(|s| s.id).collect() };
        assert_eq!(ids(after), ids(&before));
        assert_eq!(
            app.shape(2),
            before.settings.drawing.shapes.iter().find(|s| s.id == 2)
        );
        assert!(app.generate_and_wait(), "{:?}", app.report);
        app.undo();
        assert_eq!(
            app.edit.as_ref().unwrap().settings.drawing,
            before.settings.drawing
        );
    }

    #[test]
    fn imported_shapes_are_not_copied_or_mirrored() {
        let mut app = lower_part();
        select(&mut app, &[2]);
        assert_eq!(app.copy_selected_shapes(), None);
        assert!(!app.mirror_selected(Mirror::UpDown));
        assert!(app.status.as_ref().unwrap().0.contains("only drawn ones"));
    }
}

// -- chip load (docs/specs/small-items.md §5)

/// Every text the properties panel paints in one frame.
fn properties_text(app: &mut ChiploadApp) -> String {
    fn collect(shape: &egui::Shape, out: &mut String) {
        match shape {
            egui::Shape::Text(text) => {
                out.push_str(text.galley.text());
                out.push('\n');
            }
            egui::Shape::Vec(shapes) => shapes.iter().for_each(|s| collect(s, out)),
            _ => {}
        }
    }
    let ctx = egui::Context::default();
    let mut output = ctx.run_ui(egui::RawInput::default(), |ui| app.properties_panel(ui));
    output.textures_delta.clear();
    let mut text = String::new();
    for clipped in &output.shapes {
        collect(&clipped.shape, &mut text);
    }
    text
}

#[test]
fn the_feed_shows_the_chip_load_once_the_tool_has_flutes() {
    let mut app = lower_part();
    app.focus = Focus::Op(1);
    // The default tool has no flutes yet.
    let text = properties_text(&mut app);
    assert!(
        text.contains("set the tool's flutes to see the chip load"),
        "{text}"
    );
    app.edit.as_mut().unwrap().settings.tools[0].flutes = Some(2);
    let feed = chipload_pipeline::chip_load::cutting_feed(&app.edit.as_ref().unwrap().ops[1].op);
    let rpm = app.edit.as_ref().unwrap().settings.tools[0].rpm.unwrap();
    let text = properties_text(&mut app);
    let expected = format!("{:.3} mm/tooth", feed / (rpm * 2.0));
    assert!(text.contains(&expected), "{expected} in {text}");
}

// -- Images (docs/specs/image-trace.md §6)

#[test]
fn an_imported_image_is_traced_selected_as_a_whole_and_v_carved() {
    let mut app = ChiploadApp::default();
    assert!(app.import_image(&corpus("images/logo.png")));
    // No project was open: a new one was made for it.
    assert!(app.edit.is_some());
    let dialog = app.image_dialog.as_mut().unwrap();
    assert_eq!(dialog.draft.layer, "logo");
    dialog.draft.width = 80.0;
    assert!(app.apply_image());
    assert!(app.image_dialog.is_none());
    let job = &app.edit.as_ref().unwrap().settings;
    assert_eq!(job.drawing.images.len(), 1);
    assert_eq!(
        job.drawing
            .shapes
            .iter()
            .filter(|s| s.image() == Some(1))
            .count(),
        4
    );
    assert_eq!(
        app.selection,
        CanvasSelection::Source(chipload_core::chain::ShapeFrom::Image(1))
    );
    let index = app.add_operation(OpKind::Vcarve).unwrap();
    let op = &app.edit.as_ref().unwrap().ops[index].op;
    assert_eq!(op.select().image, Some(1), "the image itself");
    // One undo step takes the image away again (and one more the operation).
    app.undo();
    app.undo();
    assert!(
        app.edit
            .as_ref()
            .unwrap()
            .settings
            .drawing
            .images
            .is_empty()
    );
}

#[test]
fn clicked_traced_shapes_become_their_image_in_a_new_operation() {
    let mut app = ChiploadApp::default();
    assert!(app.import_image(&corpus("images/logo.png")));
    app.image_dialog.as_mut().unwrap().draft.width = 80.0;
    assert!(app.apply_image());
    let first = app.edit.as_ref().unwrap().settings.drawing.shapes[0].id;
    app.selection = CanvasSelection::Shapes([first].into());
    let index = app.add_operation(OpKind::Vcarve).unwrap();
    let select = app.edit.as_ref().unwrap().ops[index].op.select().clone();
    assert_eq!(
        (select.image, select.layer.as_deref(), select.shapes),
        (Some(1), None, None)
    );
}

#[test]
fn an_image_is_changed_and_removed() {
    let mut app = ChiploadApp::default();
    assert!(app.import_image(&corpus("images/logo.png")));
    app.image_dialog.as_mut().unwrap().draft.width = 80.0;
    assert!(app.apply_image());
    assert!(app.edit_image(1));
    app.image_dialog.as_mut().unwrap().draft.invert = true;
    assert!(app.apply_image());
    let job = &app.edit.as_ref().unwrap().settings;
    assert!(job.drawing.images[0].invert);
    assert_eq!(
        job.drawing
            .shapes
            .iter()
            .filter(|s| s.image() == Some(1))
            .count(),
        5
    );
    assert!(app.remove_image(1));
    let job = &app.edit.as_ref().unwrap().settings;
    assert!(job.drawing.images.is_empty() && job.drawing.shapes.is_empty());
    assert!(!app.edit_image(1));
}

#[test]
fn a_file_that_is_no_picture_is_refused() {
    let mut app = ChiploadApp::default();
    assert!(!app.import_image(&corpus("dxf/simple_part.dxf")));
    assert!(app.image_dialog.is_none());
}

#[test]
fn the_image_window_traces_its_preview_in_the_background() {
    let mut app = ChiploadApp::default();
    assert!(app.import_image(&corpus("images/logo.png")));
    let ctx = egui::Context::default();
    app.wait_for_image_preview(&ctx);
    let dialog = app.image_dialog.as_mut().unwrap();
    let Some(Ok(preview)) = &dialog.preview else {
        panic!("a preview");
    };
    assert_eq!(preview.trace.paths.len(), 4);
    assert_eq!(
        (preview.black_white.width, preview.black_white.height),
        (80, 40)
    );
    // A changed setting is traced again.
    dialog.draft.invert = true;
    app.wait_for_image_preview(&ctx);
    let Some(Ok(preview)) = &app.image_dialog.as_ref().unwrap().preview else {
        panic!("a preview");
    };
    assert!(preview.trace.black_at_border);
}

#[test]
fn the_image_window_previews_centre_lines_and_imports_them() {
    // docs/specs/image-arcs-centrelines.md §6.
    let mut app = ChiploadApp::default();
    assert!(app.import_image(&corpus("images/strokes.png")));
    let ctx = egui::Context::default();
    let dialog = app.image_dialog.as_mut().unwrap();
    assert!(dialog.draft.arcs, "new images fit arcs");
    dialog.draft.width = 60.0;
    dialog.draft.trace = chipload_job::ImageTrace::CentreLine;
    app.wait_for_image_preview(&ctx);
    let Some(Ok(preview)) = &app.image_dialog.as_ref().unwrap().preview else {
        panic!("a preview");
    };
    assert!(preview.trace.centre_lines);
    assert_eq!(preview.trace.paths.len(), 4);
    assert_eq!(preview.trace.dots, 1);
    assert!(app.apply_image());
    let job = &app.edit.as_ref().unwrap().settings;
    assert_eq!(
        job.drawing.images[0].trace,
        chipload_job::ImageTrace::CentreLine
    );
    assert_eq!(
        job.drawing.shapes.iter().filter(|s| !s.closed()).count(),
        3,
        "the T's strokes"
    );
}

#[test]
fn a_hidden_toolpath_leaves_the_legend_and_the_plunges() {
    // docs/specs/drawing-sources.md §5.
    let mut app = ChiploadApp::default();
    assert!(app.open(&corpus("jobs/v7/leads.chipload")));
    let drawing_only = crate::view::legend(&app.scene).len();
    assert!(app.generate_and_wait(), "{:?}", app.report);
    let plunge = app.scene.toolpath.as_ref().unwrap().starts[0].at;
    let at = app.view.to_screen(plunge);
    assert!(!app.scene.plunges_at(&app.view, at).is_empty());
    app.show_toolpath = false;
    app.scene.hide_toolpath = !app.show_toolpath;
    assert_eq!(crate::view::legend(&app.scene).len(), drawing_only);
    assert!(app.scene.plunges_at(&app.view, at).is_empty());
    // The toolpath itself stays: showing it again needs no new generation.
    assert!(app.scene.toolpath.is_some() && app.result.is_some());
}

// -- The Browser's Drawing folder (docs/specs/drawing-sources.md §4, browser-and-layout.md)

#[test]
fn the_drawing_panel_groups_dxfs_images_and_drawn_shapes() {
    let mut app = ChiploadApp::default();
    assert!(app.open(&corpus("jobs/v21/two_dxfs.chipload")));
    let groups = app.drawing_groups();
    let keys: Vec<&str> = groups.iter().map(|g| g.key.as_str()).collect();
    assert_eq!(keys, ["dxf:1", "dxf:2", "image:1"]);
    assert_eq!(groups[1].title, "part B");
    assert_eq!(
        groups[1].layers,
        [("CUT_OUTSIDE".to_string(), 1), ("HOLES".to_string(), 6)]
    );
    // A layer selected under a DXF makes an operation of that DXF's layer only.
    app.selection = CanvasSelection::Layer(
        "HOLES".into(),
        Some(chipload_core::chain::ShapeFrom::Dxf(2)),
    );
    let index = app.add_operation(OpKind::Profile).unwrap();
    let select = app.edit.as_ref().unwrap().ops[index].op.select().clone();
    assert_eq!(
        (select.layer.as_deref(), select.dxf),
        (Some("HOLES"), Some(2))
    );
    let both = app.op_contours(&app.edit.as_ref().unwrap().ops[0].op).len();
    let one = app
        .op_contours(&app.edit.as_ref().unwrap().ops[index].op)
        .len();
    assert!(one * 2 == both, "{one} of {both}");
    // A whole group is selected by its shapes.
    app.select_group("image:1");
    assert_eq!(app.selection.ids(&app.contours).len(), 4);
}

#[test]
fn hidden_groups_are_not_drawn_nor_clickable_but_still_cut() {
    let mut app = ChiploadApp::default();
    assert!(app.open(&corpus("jobs/v21/two_dxfs.chipload")));
    let all = app.scene.contours.len();
    app.toggle_hidden("dxf:2");
    let shown = app.scene.contours.len();
    assert_eq!(all - shown, app.hidden_contours.len());
    assert!(!app.hidden_contours.is_empty());
    assert!(
        app.scene
            .contours
            .iter()
            .all(|c| !app.hidden_contours.contains(&c.id))
    );
    // Operation 1 cuts the holes of both DXFs: it still does, and says so.
    let op = app.edit.as_ref().unwrap().ops[0].op.clone();
    assert!(app.hidden_cut(&op) > 0);
    assert!(app.generate_and_wait(), "{:?}", app.report);
    // A layer alone, then shown again; undo brings back what was hidden.
    app.toggle_hidden("dxf:2");
    assert_eq!(app.scene.contours.len(), all);
    app.toggle_hidden("dxf:1/layer:HOLES");
    assert!(app.scene.contours.len() < all);
    app.undo();
    assert_eq!(app.scene.contours.len(), all);
    // Removing a DXF removes its shapes, as one undo step.
    assert!(app.remove_dxf(2));
    assert_eq!(app.drawing_groups().len(), 2);
    app.undo();
    assert_eq!(app.drawing_groups().len(), 3);
}

// -- The Browser (docs/specs/browser-and-layout.md §3, §9)

#[test]
fn a_browser_click_selects_and_ctrl_click_adds() {
    use chipload_core::chain::ShapeFrom;
    let mut app = ChiploadApp::default();
    assert!(app.open(&corpus("jobs/v21/two_dxfs.chipload")));
    // A click on a DXF's row selects the DXF as a whole, and its row shows selected.
    let dxf = app.group_selection("dxf:1");
    assert_eq!(dxf, CanvasSelection::Source(ShapeFrom::Dxf(1)));
    app.select_row(dxf.clone(), false);
    assert_eq!(app.selection, dxf);
    let first = app.selection.ids(&app.contours);
    // Ctrl+click on the picture's row adds its contours.
    app.select_row(app.group_selection("image:1"), true);
    let both = app.selection.ids(&app.contours);
    assert_eq!(both.len(), first.len() + 4);
    assert!(first.is_subset(&both));
    // A plain click replaces it again.
    app.select_row(
        CanvasSelection::Layer("HOLES".into(), Some(ShapeFrom::Dxf(2))),
        false,
    );
    assert_eq!(app.selection.ids(&app.contours).len(), 3); // 6 shapes, 3 holes
    // The Browser draws with every folder open, with a selection and with an operation off.
    app.edit.as_mut().unwrap().ops[1].op.set_enabled(false);
    app.focus = Focus::Op(2);
    draw_sidebar(&mut app);
}

// -- The header (docs/specs/browser-and-layout.md §5)

/// Draw the top bar, the ribbon and the status bar for one frame.
fn draw_header(app: &mut ChiploadApp) {
    let ctx = egui::Context::default();
    let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
        app.top_bar(ui);
        app.ribbon(ui);
        app.status_bar(ui);
    });
    output.textures_delta.clear();
}

#[test]
fn the_header_draws_with_and_without_a_project() {
    let mut app = ChiploadApp::default();
    draw_header(&mut app);
    let mut app = lower_part();
    draw_header(&mut app);
    // With a selection (the operation buttons on), unsaved changes and a result.
    app.click(None, false);
    edit_feed(&mut app, 650.0);
    assert!(app.dirty());
    assert!(app.generate_and_wait(), "{:?}", app.report);
    draw_header(&mut app);
}

// -- parts (docs/specs/parts.md §8)

/// The two-part sample: part 1 with operations 1–3, part 2 ("part B") with operation 4.
fn two_parts() -> ChiploadApp {
    let mut app = ChiploadApp::default();
    assert!(app.open(&corpus("jobs/v22/two_parts.chipload")));
    app
}

fn op_parts(app: &ChiploadApp) -> Vec<u64> {
    app.edit
        .as_ref()
        .unwrap()
        .ops
        .iter()
        .map(|e| e.op.part())
        .collect()
}

#[test]
fn new_operations_go_into_the_active_part_whatever_is_open() {
    let mut app = two_parts();
    assert_eq!(
        app.edit.as_ref().unwrap().settings.active_part_id(),
        Some(2)
    );
    // An operation of part 1 is open; the new one still goes into part 2, at its end.
    app.focus = Focus::Op(0);
    app.select_row(app.group_selection("dxf:2"), false);
    let index = app.add_operation(OpKind::Profile).unwrap();
    assert_eq!(index, 4);
    assert_eq!(op_parts(&app), [1, 1, 1, 2, 2]);
    // Activating part 1: the next one goes after part 1's operations.
    app.activate_part(1);
    let index = app.add_operation(OpKind::Profile).unwrap();
    assert_eq!(index, 3);
    assert_eq!(op_parts(&app), [1, 1, 1, 1, 2, 2]);
    // It is kept with the project.
    assert_eq!(app.edit.as_ref().unwrap().settings.active_part, Some(1));
}

#[test]
fn operations_move_between_parts_and_the_list_stays_grouped() {
    let mut app = two_parts();
    // Dropped on part 2's row: to the end of part 2.
    app.move_operation_to_part(0, 2);
    assert_eq!(op_parts(&app), [1, 1, 2, 2]);
    assert_eq!(app.focus, Focus::Op(3));
    // Alt+↑ on part 2's first operation: it crosses into part 1.
    app.move_operation(2, 1);
    assert_eq!(op_parts(&app), [1, 1, 1, 2]);
    // Moving part 2 up takes its operation along.
    app.move_part(2, true);
    assert_eq!(op_parts(&app), [2, 1, 1, 1]);
    assert!(app.edit.as_ref().unwrap().to_job().is_ok());
}

#[test]
fn a_part_is_added_turned_off_and_deleted_with_its_operations_and_undo_brings_it_back() {
    let mut app = two_parts();
    let id = app.add_part().unwrap();
    assert_eq!(id, 3);
    assert_eq!(app.focus, Focus::Part(3));
    let names: Vec<String> = app
        .edit
        .as_ref()
        .unwrap()
        .settings
        .parts
        .iter()
        .map(|p| p.name.clone())
        .collect();
    assert_eq!(names, ["simple part", "part B", "Part 3"]);
    app.toggle_part(2);
    let job = app.edit.as_ref().unwrap().to_job().unwrap();
    assert!(!job.cuts(3));
    app.delete_part(2);
    assert_eq!(op_parts(&app), [1, 1, 1]);
    app.undo();
    assert_eq!(op_parts(&app), [1, 1, 1, 2]);
    // The last part stays.
    app.delete_part(3);
    app.delete_part(2);
    app.delete_part(1);
    assert_eq!(app.edit.as_ref().unwrap().settings.parts.len(), 1);
}

#[test]
fn a_parts_properties_edit_its_copies_and_the_browser_draws_the_parts() {
    let mut app = two_parts();
    app.focus = Focus::Part(1);
    draw_properties(&mut app);
    draw_sidebar(&mut app);
    // A problem in part 1's copies marks part 1's form.
    app.edit.as_mut().unwrap().settings.parts[0].copies = Some(chipload_job::Copies {
        columns: 0,
        rows: 1,
        dx: 10.0,
        dy: 10.0,
        order: chipload_job::CopyOrder::ByOperation,
    });
    app.commit();
    let fields = app.form_problem_map(Focus::Part(1));
    assert!(fields.contains_key("parts.0.copies.columns"), "{fields:?}");
    assert!(app.form_problem_map(Focus::Part(2)).is_empty());
    draw_properties(&mut app);
    draw_sidebar(&mut app);
}
