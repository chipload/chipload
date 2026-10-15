// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

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
        app.dialogs.prompt,
        Some(Prompt::Discard(Pending::Open(_)))
    ));
    app.dialogs.prompt = None;
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
    assert!(matches!(
        app.dialogs.prompt,
        Some(Prompt::Discard(Pending::New))
    ));
    app.dialogs.prompt = None;

    // The first DXF has no units: they are asked for, then the drawing comes in and the
    // drawn circle stays.
    assert!(!app.update_from(&corpus("dxf/messy.dxf"), None));
    assert!(matches!(
        app.dialogs.prompt,
        Some(Prompt::Units(_, UnitsFor::Update(None)))
    ));
    assert!(app.update_from(&corpus("dxf/messy.dxf"), Some(Units::Mm)));
    assert!(app.dialogs.prompt.is_none());
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
    if let Operation::Pocket(p) = &mut app.edit.as_mut().unwrap().ops[0].op {
        p.lead = Some(Lead { radius: 3.0 });
    }
    assert!(app.generate_and_wait(), "{:?}", app.report);
    let after = app.result.as_ref().unwrap().program.moves.len();
    assert!(
        after > before,
        "the lead arcs add moves: {before} -> {after}"
    );
    // The marker shows the wall ring without its leads.
    let marker = app
        .scene
        .start_markers
        .iter()
        .find(|m| m.operation == 0)
        .unwrap();
    assert!(marker.ring.len() > 3);
    let dir = temp_dir("lead");
    let project = dir.join("lead.chipload");
    assert!(app.save_project(&project));
    let job = load_job(&project).unwrap();
    assert_eq!(job.operations[0].lead(), Some(Lead { radius: 3.0 }));
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
            rotation: 0.0,
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
    assert!(app.place(chipload_pipeline::placement::Move::Rotate(-90.0)));
    assert_eq!(placement(&app).rotation, 270.0);
    assert!(near(contour_min(&app, outline(&app)), (90.0, 10.0)));
    assert_eq!(circle(&app), before, "the drawn circle stays");
    app.undo();
    assert_eq!(placement(&app).rotation, 0.0);
    app.redo();
    assert!(app.place(chipload_pipeline::placement::Move::Reset));
    assert!(near(contour_min(&app, outline(&app)), (50.0, 50.0)));
    assert_eq!(placement(&app).rotation, 0.0);

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
    // The legs add moves on every lap, and the tool turns straight back at each of the
    // four tips (the toolpath tests count the moves per lap).
    let moves = &app.result.as_ref().unwrap().program.moves;
    assert!(
        moves.len() > round + 4 * 2 * 2,
        "{round} -> {}",
        moves.len()
    );
    let mut at: Vec<(f64, f64)> = Vec::new();
    let mut here = (f64::NAN, f64::NAN);
    for m in moves {
        here = (m.x.unwrap_or(here.0), m.y.unwrap_or(here.1));
        if m.kind == MoveKind::Rapid {
            at.push((f64::NAN, f64::NAN));
        } else if m.x.is_some() || m.y.is_some() {
            at.push(here);
        }
    }
    let mut tips: Vec<(f64, f64)> = Vec::new();
    for w in at.windows(3) {
        let (u, v) = (
            (w[1].0 - w[0].0, w[1].1 - w[0].1),
            (w[2].0 - w[1].0, w[2].1 - w[1].1),
        );
        let (lu, lv) = (u.0.hypot(u.1), v.0.hypot(v.1));
        let back = lu > 1e-6 && lv > 1e-6 && (u.0 * v.0 + u.1 * v.1) / (lu * lv) < -1.0 + 1e-9;
        if back
            && !tips
                .iter()
                .any(|t| (t.0 - w[1].0).hypot(t.1 - w[1].1) < 1e-6)
        {
            tips.push(w[1]);
        }
    }
    assert_eq!(tips.len(), 4, "{tips:?}");
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
    assert!(app.scene.sim.is_none() && app.sim.simulation.is_none());
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
    let (done, along) = {
        let pb = app.playback.as_ref().unwrap();
        (pb.done(), pb.along())
    };
    let half = app.scene.sim.as_ref().unwrap().surface.heights.clone();
    let numbers = app.sim.simulation.as_ref().unwrap().numbers.clone();
    // The stock part of the way along the move under way.
    let expected = app
        .sim
        .history
        .as_ref()
        .unwrap()
        .at(done, along, &numbers)
        .heights;
    assert_eq!(half, expected);
    let full = app.sim.full_layer.as_ref().unwrap().surface.heights.clone();
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
fn a_project_with_a_removed_strategy_opens_as_a_rings_pocket() {
    // Since schema 26 the strategy is gone: the pocket is cut as rings, nothing to switch.
    let mut app = ChiploadApp::default();
    assert!(app.open(&corpus("jobs/v7/trochoidal.chipload")));
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
