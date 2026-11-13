// SPDX-License-Identifier: GPL-3.0-or-later
//! 3D models in the window (docs/specs/stl-relief.md §9).
use super::*;

/// The dome sample's STL, written to a file of its own.
fn dome_stl(name: &str) -> PathBuf {
    let job = chipload_job::load_job(&corpus("jobs/v36/relief_dome.chipload")).unwrap();
    let bytes = chipload_pipeline::models::model_bytes(&job.drawing.models[0]).unwrap();
    let path = temp_dir(name).join("dome.stl");
    std::fs::write(&path, bytes).unwrap();
    path
}

#[test]
fn a_model_is_added_placed_carved_and_removed() {
    let path = dome_stl("models-add");
    let mut app = ChiploadApp::default();
    app.new_project();
    let before = app.edit.as_ref().unwrap().job.clone();
    assert!(app.add_model_from(&path));
    assert!(app.wait_model_reading());
    let job = app.edit.as_ref().unwrap().job.clone();
    let model = job.drawing.models[0].clone();
    // Half the board wide, and no deeper than half the board.
    let (width, _) = before.stock_size();
    assert_eq!(model.width, (width / 2.0).round());
    assert!(
        model.depth <= before.stock().thickness / 2.0 + 1e-9,
        "{}",
        model.depth
    );
    assert_eq!(app.focus, Focus::Model(model.id));
    // Its outline is a group of its own in the Browser, its row opens its properties.
    let groups = app.drawing_groups();
    let group = groups
        .iter()
        .find(|g| g.key == format!("model:{}", model.id));
    assert!(group.is_some_and(|g| g.shapes > 0), "{groups:?}");
    draw_properties(&mut app);
    draw_sidebar(&mut app);
    // A new width draws the outline again, as the pipeline does, as one undo step.
    let mut wider = model.clone();
    wider.width = 60.0;
    assert!(app.set_model(model.id, &wider));
    let changed = app.edit.as_ref().unwrap().job.clone();
    let (expected, _) = chipload_pipeline::models::remake(
        &{
            let mut j = job.clone();
            j.drawing.models[0] = wider.clone();
            j
        },
        model.id,
    )
    .unwrap();
    assert_eq!(changed.drawing.shapes, expected.drawing.shapes);
    app.undo();
    assert_eq!(
        app.edit.as_ref().unwrap().job.drawing.models[0].width,
        model.width
    );
    // New relief: a Relief of the model, within its outline.
    app.new_relief(model.id);
    let ops = &app.edit.as_ref().unwrap().job.operations;
    assert!(
        matches!(ops.last(), Some(chipload_job::Operation::Relief(r)) if r.model == model.id),
        "{ops:?}"
    );
    // Removing it takes the outline; the relief stays, with a problem.
    assert!(app.remove_model(model.id));
    let job = &app.edit.as_ref().unwrap().job;
    assert!(job.drawing.models.is_empty());
    assert!(job.drawing.shapes.iter().all(|s| s.model().is_none()));
    // The relief stays open, with its problem.
    assert_eq!(app.focus, Focus::Op(0));
    assert!(
        job.validate()
            .iter()
            .any(|p| p.path == "operations.0.model"),
        "no problem for the relief's model"
    );
}

#[test]
fn an_stl_dropped_on_the_window_is_added() {
    let path = dome_stl("models-drop");
    let mut app = ChiploadApp::default();
    assert!(app.drop_files(&[path]));
    assert!(app.wait_model_reading());
    assert_eq!(app.edit.as_ref().unwrap().job.drawing.models.len(), 1);
}

#[test]
fn opening_or_importing_an_stl_starts_a_project_with_it() {
    let path = dome_stl("models-open");
    let mut app = ChiploadApp::default();
    assert!(app.open(&path));
    assert!(app.wait_model_reading());
    assert_eq!(app.edit.as_ref().unwrap().job.drawing.models.len(), 1);
    // Import starts a new project even with one open: not a second model in the first.
    assert!(app.import_any(&path));
    assert!(app.wait_model_reading());
    let job = &app.edit.as_ref().unwrap().job;
    assert_eq!(job.drawing.models.len(), 1);
}

/// A model's rows and properties, rendered to look at:
/// `CHIPLOAD_PICTURES=/some/dir cargo test -p chipload-gui model_pictures -- --ignored`.
#[test]
#[ignore = "writes pictures; run by hand"]
fn model_pictures() {
    use egui_kittest::Harness;
    let Ok(dir) = std::env::var("CHIPLOAD_PICTURES") else {
        panic!("set CHIPLOAD_PICTURES to the folder to write to");
    };
    let dir = PathBuf::from(dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dome_stl("models-pictures");
    for (name, language) in [("model-en", "en"), ("model-de", "de")] {
        let path = path.clone();
        let mut harness = Harness::builder()
            .with_size(egui::vec2(1280.0, 800.0))
            .wgpu()
            .build_eframe(move |cc| {
                open_sections(&cc.egui_ctx);
                {
                    let mut fonts = crate::fonts::ui_fonts();
                    crate::fonts::add_text_fonts(&mut fonts);
                    cc.egui_ctx.set_fonts(fonts);
                }
                crate::style::apply(&cc.egui_ctx);
                let mut app = ChiploadApp::default();
                app.set_language(language);
                app.add_model_from(&path);
                app.wait_model_reading();
                app
            });
        harness.run_steps(6);
        let image = harness.render().unwrap();
        image.save(dir.join(format!("{name}.png"))).unwrap();
    }
    // A relief with its roughing pass open.
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .wgpu()
        .build_eframe(move |cc| {
            open_sections(&cc.egui_ctx);
            {
                let mut fonts = crate::fonts::ui_fonts();
                crate::fonts::add_text_fonts(&mut fonts);
                cc.egui_ctx.set_fonts(fonts);
            }
            crate::style::apply(&cc.egui_ctx);
            let mut app = ChiploadApp::default();
            app.open(&corpus("jobs/v36/relief_dome.chipload"));
            app.add_roughing(0, 1);
            app
        });
    harness.run_steps(6);
    let image = harness.render().unwrap();
    image.save(dir.join("roughing-en.png")).unwrap();
    // The model in 3D, faintly, with its outline.
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .wgpu()
        .build_eframe(move |cc| {
            {
                let mut fonts = crate::fonts::ui_fonts();
                crate::fonts::add_text_fonts(&mut fonts);
                cc.egui_ctx.set_fonts(fonts);
            }
            crate::style::apply(&cc.egui_ctx);
            let mut app = ChiploadApp::default();
            app.open(&corpus("jobs/v36/relief_dome.chipload"));
            app.view_3d();
            app
        });
    harness.run_steps(8);
    let image = harness.render().unwrap();
    image.save(dir.join("model-3d.png")).unwrap();
}

#[test]
fn a_relief_needs_a_model_and_can_be_roughed_first() {
    use chipload_pipeline::new_ops::OpKind;
    let mut app = ChiploadApp::default();
    app.new_project();
    assert!(!crate::app::browser::can_add(
        OpKind::Relief,
        true,
        app.has_models()
    ));
    let mut app = ChiploadApp::default();
    assert!(app.open(&corpus("jobs/v36/relief_dome.chipload")));
    assert!(crate::app::browser::can_add(
        OpKind::Relief,
        false,
        app.has_models()
    ));
    // The 6 mm end mill is offered, the ball nose too (a plain one).
    match app.roughing_menu(0) {
        Some(RestMenu::Tools { project, .. }) => {
            assert_eq!(
                project.iter().map(|(id, _)| *id).collect::<Vec<_>>(),
                vec![1, 2]
            );
        }
        other => panic!("{other:?}"),
    }
    assert!(app.add_roughing(0, 1));
    assert_eq!(app.focus, Focus::RoughingPass(0));
    let rough = |app: &ChiploadApp| match &app.edit.as_ref().unwrap().job.operations[0] {
        chipload_job::Operation::Relief(r) => r.roughing,
        _ => None,
    };
    assert_eq!(rough(&app).map(|r| (r.tool, r.allowance)), Some((1, 0.5)));
    draw_properties(&mut app);
    draw_sidebar(&mut app);
    app.toggle_roughing(0);
    assert_eq!(rough(&app).map(|r| r.enabled), Some(false));
    app.remove_roughing(0);
    assert_eq!(rough(&app), None);
    assert_eq!(app.focus, Focus::Op(0));
    app.undo();
    assert!(rough(&app).is_some());
}

#[test]
fn a_models_handles_move_turn_and_size_it() {
    use chipload_pipeline::box_handles::BoxHandle;
    let mut app = ChiploadApp::default();
    assert!(app.open(&corpus("jobs/v36/relief_dome.chipload")));
    app.selection = CanvasSelection::whole(chipload_core::chain::ShapeFrom::Model(1));
    let handles = app.box_handle_points();
    assert_eq!(handles.len(), 6);
    let model = |app: &ChiploadApp| app.edit.as_ref().unwrap().job.drawing.models[0].clone();
    let before = model(&app);
    // Moved by its centre: 5 mm right.
    let (_, center) = handles[0];
    app.start_box_drag(BoxHandle::Move);
    app.drag_box_to((center.0 + 5.0, center.1), false);
    app.end_box_drag();
    let moved = model(&app);
    assert!((moved.x - (before.x + 5.0)).abs() < 1e-6, "{moved:?}");
    assert_eq!(moved.width, before.width);
    // Turned: 80° stepped to 75°, about its centre.
    let center = app.box_handle_points()[0].1;
    app.start_box_drag(BoxHandle::Turn);
    let a = 80f64.to_radians();
    app.drag_box_to(
        (center.0 + 30.0 * a.cos(), center.1 + 30.0 * a.sin()),
        false,
    );
    app.end_box_drag();
    assert_eq!(model(&app).rotation, 75.0);
    // Its outline went with it, and each drag was one undo step.
    app.undo();
    assert_eq!(model(&app).rotation, before.rotation);
}

#[test]
fn a_pocket_round_a_model_keeps_its_outline_as_an_island_when_the_model_moves() {
    use chipload_pipeline::new_ops::OpKind;
    let mut app = ChiploadApp::default();
    assert!(app.open(&corpus("jobs/v36/relief_dome.chipload")));
    // A square round the dome (X, Y 10 .. 50), drawn as in the window.
    app.start_drawing(Tool::Rectangle);
    app.draw_click((2.0, 2.0), false);
    app.draw_click((58.0, 58.0), false);
    app.commit();
    let id_of = |app: &ChiploadApp, model: bool| {
        app.contours
            .iter()
            .find(|c| {
                c.parts
                    .iter()
                    .all(|p| p.segment.from == Some(chipload_core::chain::ShapeFrom::Model(1)))
                    == model
            })
            .unwrap()
            .id
    };
    // The outline clicked, the square added with Ctrl: a pocket of both.
    app.click(Some(id_of(&app, true)), false);
    app.click(Some(id_of(&app, false)), true);
    let index = app.add_operation(OpKind::Pocket).unwrap();
    let cut = |app: &ChiploadApp| {
        let op = &app.edit.as_ref().unwrap().job.operations[index];
        app.op_contours(op)
    };
    assert_eq!(cut(&app).len(), 2);
    // Moved and made smaller: the pocket still has the outline as its island.
    let mut model = app.edit.as_ref().unwrap().job.drawing.models[0].clone();
    model.x += 3.0;
    model.width = 30.0;
    assert!(app.set_model(1, &model));
    assert_eq!(
        cut(&app),
        [id_of(&app, true), id_of(&app, false)]
            .into_iter()
            .collect()
    );
}

#[test]
fn two_stls_dropped_are_read_one_after_the_other() {
    let path = dome_stl("models-drop-two");
    let mut app = ChiploadApp::default();
    assert!(app.drop_files(&[path.clone(), path]));
    // The first is being read; the second waits for it.
    assert!(app.reading_model());
    assert!(app.wait_model_reading());
    assert!(app.reading_model());
    assert!(app.wait_model_reading());
    assert!(!app.reading_model());
    assert_eq!(app.edit.as_ref().unwrap().job.drawing.models.len(), 2);
}
