// SPDX-License-Identifier: GPL-3.0-or-later
//! Materials in the window (docs/specs/materials.md).
use super::*;
use chipload_job::library::{Cutting, LibraryTool, ToolLibrary, save_tools, uid_for_name};

const TOOL: &str = "6 mm 2-flute";

/// The app with a settings folder whose library has the badge project's tool, and the badge
/// project open with its tool linked to it.
fn app_with_library(name: &str) -> ChiploadApp {
    let dir = temp_dir(&format!("materials-{name}"));
    for file in ["tools.json", "machines.json", "materials.json"] {
        let _ = std::fs::remove_file(dir.join(file));
    }
    let mut app = ChiploadApp::default();
    assert!(app.open(&corpus("jobs/v31/svg_badge.chipload")));
    let tool = app.edit.as_ref().unwrap().settings.tools[0].clone();
    let entry = LibraryTool::from_tool(TOOL, &uid_for_name(TOOL), &tool, None);
    let library = ToolLibrary {
        tools: vec![entry.clone()],
        ..ToolLibrary::default()
    };
    save_tools(&library, &dir.join("tools.json")).unwrap();
    app.set_settings_dir(Some(dir));
    let t = &mut app.edit.as_mut().unwrap().settings.tools[0];
    t.name = Some(TOOL.into());
    t.library_uid = Some(entry.uid);
    app.commit();
    app
}

fn plywood() -> Cutting {
    Cutting {
        feed: 1500.0,
        plunge_feed: 500.0,
        step_down: 2.5,
        stepover: 0.45,
    }
}

/// A material "Plywood" with values for the library tool; its uid.
fn add_plywood(app: &mut ChiploadApp) -> String {
    let uid = app.new_material().expect("a new material");
    let mut draft = app.library.materials.find_uid(&uid).unwrap().clone();
    draft.name = "Plywood".into();
    draft.set_values(&uid_for_name(TOOL), plywood());
    app.library_state.material_drafts.insert(uid.clone(), draft);
    assert!(app.save_material_draft(&uid));
    uid
}

#[test]
fn choosing_a_material_fills_the_operations_in_one_undo_step() {
    let mut app = app_with_library("choose");
    let uid = add_plywood(&mut app);
    let before = app.edit.as_ref().unwrap().settings.stock.thickness;
    assert!(app.use_material(Some(&uid)));
    let edit = app.edit.as_ref().unwrap();
    assert_eq!(edit.settings.stock.thickness, before, "the stock stays");
    assert_eq!(
        edit.settings.stock.material.as_ref().unwrap().name,
        "Plywood"
    );
    assert!(edit.ops.iter().all(|e| e.op.has_cutting(&plywood())));
    assert!(edit.ops.iter().all(|e| e.op.material().is_some()));
    assert!(
        app.report.lines.len() >= 3,
        "chosen, operations: {:?}",
        app.report
    );
    app.undo();
    let edit = app.edit.as_ref().unwrap();
    assert_eq!(edit.settings.stock.material, None);
    assert!(edit.ops.iter().all(|e| e.op.material().is_none()));
}

#[test]
fn a_new_operation_takes_the_materials_values() {
    let mut app = app_with_library("new-op");
    let uid = add_plywood(&mut app);
    assert!(app.use_material(Some(&uid)));
    let outline = app.contours.first().unwrap().id;
    select(&mut app, &[outline]);
    let i = app.add_operation(OpKind::Profile).unwrap();
    let op = &app.edit.as_ref().unwrap().ops[i].op;
    assert!(op.has_cutting(&plywood()));
    assert_eq!(op.material().unwrap().name, "Plywood");
}

#[test]
fn an_operation_chooses_its_own_material_in_its_form() {
    use egui_kittest::Harness;
    use egui_kittest::kittest::{NodeT, Queryable};
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1200.0, 900.0))
        .build_eframe(|_| {
            let mut app = app_with_library("op-form");
            add_plywood(&mut app);
            app.focus = Focus::Op(1);
            app
        });
    harness.run_steps(4);
    let op = |h: &Harness<'_, ChiploadApp>| h.state().edit.as_ref().unwrap().ops[1].op.clone();
    assert_eq!(op(&harness).material(), None);
    // The material's list, showing None.
    harness
        .query_all_by_role(egui::accesskit::Role::ComboBox)
        .find(|n| n.accesskit_node().value().as_deref() == Some("None"))
        .expect("the material's list")
        .click();
    harness.run_steps(3);
    harness.get_by_label("Plywood").click();
    harness.run_steps(4);
    let chosen = op(&harness);
    assert_eq!(chosen.material().unwrap().name, "Plywood");
    assert!(chosen.has_cutting(&plywood()));
    let edit = harness.state().edit.as_ref().unwrap();
    assert_eq!(edit.settings.stock.material, None, "the project's stays");
    assert_eq!(edit.ops[0].op.material(), None, "the other operation's too");
    assert!(harness.query_by_label("From Plywood").is_some());
}

#[test]
fn a_new_tool_takes_the_operations_materials_values() {
    let mut app = app_with_library("tool-change");
    let uid = add_plywood(&mut app);
    assert!(app.use_material(Some(&uid)));
    // A second tool, from the library too, with its own values in Plywood.
    let second = Cutting {
        feed: 900.0,
        ..plywood()
    };
    let mut draft = app.library.materials.find_uid(&uid).unwrap().clone();
    draft.set_values("b000000000000002", second);
    app.library_state.material_drafts.insert(uid.clone(), draft);
    assert!(app.save_material_draft(&uid));
    let edit = app.edit.as_mut().unwrap();
    let mut tool = edit.settings.tools[0].clone();
    tool.id = 2;
    tool.library_uid = Some("b000000000000002".into());
    edit.settings.tools.push(tool);
    edit.ops[1].op.set_tool(2);
    app.follow_op_material(1);
    assert!(app.edit.as_ref().unwrap().ops[1].op.has_cutting(&second));
    // Back to tool 1: Plywood's values for it.
    app.edit.as_mut().unwrap().ops[1].op.set_tool(1);
    app.follow_op_material(1);
    assert!(app.edit.as_ref().unwrap().ops[1].op.has_cutting(&plywood()));
}

#[test]
fn an_operations_tuned_values_are_saved_to_the_material() {
    let mut app = app_with_library("save");
    let uid = add_plywood(&mut app);
    assert!(app.use_material(Some(&uid)));
    // Tuned on the machine: a slower feed for the outline.
    if let Operation::Profile(p) = &mut app.edit.as_mut().unwrap().ops[1].op {
        p.feed = 1300.0;
    }
    app.commit();
    let values = |app: &ChiploadApp| {
        app.library
            .materials
            .find_uid(&uid)
            .unwrap()
            .values(&uid_for_name(TOOL))
            .unwrap()
    };
    assert_eq!(values(&app).feed, 1500.0);
    assert!(app.save_op_to_material(1));
    assert_eq!(values(&app).feed, 1300.0);
    // The pocket's own stepover stays the material's, the profile has none.
    assert_eq!(values(&app).stepover, 0.45);
}

#[test]
fn removing_a_library_tool_removes_its_values_from_the_materials() {
    let mut app = app_with_library("forget");
    let uid = add_plywood(&mut app);
    app.remove_from_library(super::super::user_library::LibraryKind::Tools, TOOL)
        .unwrap();
    assert!(
        app.library
            .materials
            .find_uid(&uid)
            .unwrap()
            .tools
            .is_empty()
    );
}

/// The material views rendered to PNG files to look at, as `layout::pictures`:
/// `CHIPLOAD_PICTURES=/some/dir cargo test -p chipload-gui material_pictures -- --ignored`.
#[test]
#[ignore = "writes pictures; run by hand"]
fn material_pictures() {
    use egui_kittest::Harness;
    let Ok(dir) = std::env::var("CHIPLOAD_PICTURES") else {
        panic!("set CHIPLOAD_PICTURES to the folder to write to");
    };
    let dir = PathBuf::from(dir);
    std::fs::create_dir_all(&dir).unwrap();
    let shots: [(&str, &str); 4] = [
        ("material-job-en", "job"),
        ("material-op-en", "op"),
        ("material-form-en", "form"),
        ("material-form-de", "form"),
    ];
    for (name, view) in shots {
        let language = if name.ends_with("-de") { "de" } else { "en" };
        let mut harness = Harness::builder()
            .with_size(egui::vec2(1200.0, 800.0))
            .wgpu()
            .build_eframe(move |cc| {
                if let Ok(mut fonts) = crate::fonts::system_fonts() {
                    crate::fonts::add_text_fonts(&mut fonts);
                    cc.egui_ctx.set_fonts(fonts);
                }
                crate::style::apply(&cc.egui_ctx);
                let mut app = app_with_library(&format!("pictures-{view}"));
                app.set_language(language);
                let uid = add_plywood(&mut app);
                assert!(app.use_material(Some(&uid)));
                if let Operation::Profile(p) = &mut app.edit.as_mut().unwrap().ops[1].op {
                    p.feed = 1300.0;
                }
                app.commit();
                app.focus = match view {
                    "job" => Focus::Job,
                    "op" => Focus::Op(1),
                    _ => Focus::LibraryMaterial(super::super::library_view::uid_key(&uid)),
                };
                app
            });
        harness.run_steps(6);
        let image = harness.render().unwrap();
        image.save(dir.join(format!("{name}.png"))).unwrap();
    }
}
