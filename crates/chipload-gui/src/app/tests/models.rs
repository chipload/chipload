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
    assert_eq!(app.edit.as_ref().unwrap().job.drawing.models.len(), 1);
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
                app
            });
        harness.run_steps(6);
        let image = harness.render().unwrap();
        image.save(dir.join(format!("{name}.png"))).unwrap();
    }
}
