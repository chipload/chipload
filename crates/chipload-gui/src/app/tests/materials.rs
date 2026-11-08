// SPDX-License-Identifier: GPL-3.0-or-later
//! Materials in the window (docs/specs/materials.md, docs/specs/cutting-data.md §7).
use super::*;
use chipload_job::library::{
    Cutting, LibraryTool, ToolLibrary, save_tools, uid_for_name, update_tools,
};

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
    let tool = app.edit.as_ref().unwrap().job.tools[0].clone();
    let entry = LibraryTool::from_tool(TOOL, &uid_for_name(TOOL), &tool, Vec::new());
    let library = ToolLibrary {
        tools: vec![entry.clone()],
        ..ToolLibrary::default()
    };
    save_tools(&library, &dir.join("tools.json")).unwrap();
    app.set_settings_dir(Some(dir));
    let t = &mut app.edit.as_mut().unwrap().job.tools[0];
    t.name = Some(TOOL.into());
    t.library_uid = Some(entry.uid);
    app.commit();
    app
}

fn plywood() -> Cutting {
    Cutting::uniform(1500.0, 500.0, 2.5, 0.45)
}

/// Library tool `tool_uid`'s row for material `material` (a copy of the badge's tool when
/// the library hasn't got it), written to `tools.json` and read back.
fn set_row(app: &mut ChiploadApp, tool_uid: &str, material: &str, values: Cutting) {
    let dir = app.user.settings_dir.clone().unwrap();
    let tool = app.edit.as_ref().unwrap().job.tools[0].clone();
    update_tools(&chipload_pipeline::library::tools_path(&dir), |library| {
        if library.find_uid(tool_uid).is_none() {
            let name = format!("tool {tool_uid}");
            library
                .tools
                .push(LibraryTool::from_tool(&name, tool_uid, &tool, Vec::new()));
        }
        let entry = library
            .tools
            .iter_mut()
            .find(|t| t.uid == tool_uid)
            .unwrap();
        entry.set_row(Some(material), values);
        Ok(())
    })
    .unwrap();
    app.reload_library();
}

/// A material "Plywood" with the library tool's values for it; its uid.
fn add_plywood(app: &mut ChiploadApp) -> String {
    let uid = app.new_material().expect("a new material");
    let mut draft = app.user.library.materials.find_uid(&uid).unwrap().clone();
    draft.name = "Plywood".into();
    app.user
        .library_state
        .material_drafts
        .insert(uid.clone(), draft);
    assert!(app.save_material_draft(&uid));
    set_row(app, &uid_for_name(TOOL), &uid, plywood());
    uid
}

#[test]
fn choosing_a_material_fills_the_operations_in_one_undo_step() {
    let mut app = app_with_library("choose");
    let uid = add_plywood(&mut app);
    let before = app.edit.as_ref().unwrap().job.stock().thickness;
    assert!(app.use_material(Some(&uid)));
    let edit = app.edit.as_ref().unwrap();
    assert_eq!(edit.job.stock().thickness, before, "the stock stays");
    assert_eq!(edit.job.stock().material.as_ref().unwrap().name, "Plywood");
    assert!(
        edit.job
            .operations
            .iter()
            .all(|e| e.has_cutting(&plywood()))
    );
    assert!(edit.job.operations.iter().all(|e| e.material().is_some()));
    assert!(
        app.report.lines.len() >= 3,
        "chosen, operations: {:?}",
        app.report
    );
    app.undo();
    let edit = app.edit.as_ref().unwrap();
    assert_eq!(edit.job.stock().material, None);
    assert!(edit.job.operations.iter().all(|e| e.material().is_none()));
}

#[test]
fn a_new_project_starts_with_the_default_material() {
    let mut app = app_with_library("default");
    let uid = add_plywood(&mut app);
    assert!(app.make_default_material(&uid));
    // Renamed in its form: still the default.
    let mut draft = app.user.library.materials.find_uid(&uid).unwrap().clone();
    draft.name = "Birch ply".into();
    app.user
        .library_state
        .material_drafts
        .insert(uid.clone(), draft);
    assert!(app.save_material_draft(&uid));
    app.new_project();
    let edit = app.edit.as_ref().unwrap();
    assert_eq!(
        edit.job.stock().material.as_ref().unwrap().name,
        "Birch ply"
    );
}

#[test]
fn a_new_operation_takes_the_materials_values() {
    let mut app = app_with_library("new-op");
    let uid = add_plywood(&mut app);
    assert!(app.use_material(Some(&uid)));
    let outline = app.contours.first().unwrap().id;
    select(&mut app, &[outline]);
    let i = app.add_operation(OpKind::Profile).unwrap();
    let op = &app.edit.as_ref().unwrap().job.operations[i];
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
    let op =
        |h: &Harness<'_, ChiploadApp>| h.state().edit.as_ref().unwrap().job.operations[1].clone();
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
    assert_eq!(edit.job.stock().material, None, "the project's stays");
    assert_eq!(
        edit.job.operations[0].material(),
        None,
        "the other operation's too"
    );
    assert!(harness.query_by_label("From Plywood").is_some());
}

#[test]
fn a_new_tool_takes_the_operations_materials_values() {
    let mut app = app_with_library("tool-change");
    let uid = add_plywood(&mut app);
    assert!(app.use_material(Some(&uid)));
    // A second tool, from the library too, with its own values in Plywood.
    let second = Cutting {
        slot_feed: 900.0,
        feed: 900.0,
        ..plywood()
    };
    set_row(&mut app, "b000000000000002", &uid, second);
    let edit = app.edit.as_mut().unwrap();
    let mut tool = edit.job.tools[0].clone();
    tool.id = 2;
    tool.library_uid = Some("b000000000000002".into());
    edit.job.tools.push(tool);
    edit.job.operations[1].set_tool(2);
    app.follow_op_material(1);
    assert!(app.edit.as_ref().unwrap().job.operations[1].has_cutting(&second));
    // Back to tool 1: Plywood's values for it.
    app.edit.as_mut().unwrap().job.operations[1].set_tool(1);
    app.follow_op_material(1);
    assert!(app.edit.as_ref().unwrap().job.operations[1].has_cutting(&plywood()));
}

#[test]
fn an_operations_tuned_values_are_saved_to_the_material() {
    let mut app = app_with_library("save");
    let uid = add_plywood(&mut app);
    assert!(app.use_material(Some(&uid)));
    // Tuned on the machine: a slower feed for the outline.
    if let Operation::Profile(p) = &mut app.edit.as_mut().unwrap().job.operations[1] {
        p.feed = 1300.0;
    }
    app.commit();
    let values = |app: &ChiploadApp| {
        app.user
            .library
            .tools
            .find_uid(&uid_for_name(TOOL))
            .unwrap()
            .row(Some(&uid))
            .unwrap()
            .values
    };
    assert_eq!(values(&app).slot_feed, 1500.0);
    assert!(app.save_op_to_material(1));
    // A profile is a full-width cut: its feed is the row's slot feed; the clearing values
    // and the stepover stay the row's.
    assert_eq!(values(&app).slot_feed, 1300.0);
    assert_eq!((values(&app).feed, values(&app).stepover), (1500.0, 0.45));
}

#[test]
fn the_tool_shows_its_cutting_data_and_the_material_its_tools() {
    use egui_kittest::Harness;
    use egui_kittest::kittest::{NodeT, Queryable};
    let uid = std::cell::RefCell::new(String::new());
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1200.0, 900.0))
        .build_eframe(|_| {
            let mut app = app_with_library("tool-form");
            *uid.borrow_mut() = add_plywood(&mut app);
            app.focus =
                Focus::LibraryTool(super::super::library_view::uid_key(&uid_for_name(TOOL)));
            app
        });
    harness.run_steps(4);
    assert!(harness.query_by_label_contains("Cutting data").is_some());
    assert!(harness.query_by_label_contains("Plywood").is_some());
    assert!(harness.query_by_label_contains("slot 1500").is_some());
    // The material lists the tool, a link to it.
    let key = super::super::library_view::uid_key(&uid.borrow());
    harness.state_mut().focus = Focus::LibraryMaterial(key);
    harness.run_steps(4);
    // A link: found by its text among the window's labels.
    harness
        .get_all_by_role(egui::accesskit::Role::Label)
        .find(|n| n.accesskit_node().label().as_deref() == Some(TOOL))
        .expect("the tool's link")
        .click();
    harness.run_steps(4);
    assert!(matches!(harness.state().focus, Focus::LibraryTool(_)));
}

#[test]
fn a_pocket_has_its_slot_feed_and_slot_step_down() {
    use egui_kittest::Harness;
    use egui_kittest::kittest::Queryable;
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1200.0, 900.0))
        .build_eframe(|cc| {
            open_sections(&cc.egui_ctx);
            let mut app = app_with_library("pocket-form");
            let uid = add_plywood(&mut app);
            set_row(
                &mut app,
                &uid_for_name(TOOL),
                &uid,
                Cutting {
                    slot_feed: 1100.0,
                    slot_step_down: 1.5,
                    ..plywood()
                },
            );
            assert!(app.use_material(Some(&uid)));
            app.focus = Focus::Op(0);
            app
        });
    harness.run_steps(4);
    assert!(harness.query_by_label("Slot feed").is_some());
    assert!(harness.query_by_label("Slot step down").is_some());
    let Operation::Pocket(p) = &harness.state().edit.as_ref().unwrap().job.operations[0] else {
        panic!("operation 1 is a pocket");
    };
    assert_eq!((p.slot_feed, p.slot_step_down), (Some(1100.0), Some(1.5)));
    assert!(harness.query_by_label("From Plywood").is_some());
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
    let shots: [(&str, &str); 9] = [
        ("material-job-en", "job"),
        ("job-fit-en", "job-fit"),
        ("job-fit-de", "job-fit"),
        ("job-machine-en", "job-machine"),
        ("material-op-en", "op"),
        ("through-en", "through"),
        ("through-de", "through"),
        ("material-form-en", "form"),
        ("material-form-de", "form"),
    ];
    for (name, view) in shots {
        let language = if name.ends_with("-de") { "de" } else { "en" };
        let mut harness = Harness::builder()
            .with_size(egui::vec2(1200.0, 800.0))
            .wgpu()
            .build_eframe(move |cc| {
                {
                    let mut fonts = crate::fonts::ui_fonts();
                    crate::fonts::add_text_fonts(&mut fonts);
                    cc.egui_ctx.set_fonts(fonts);
                }
                crate::style::apply(&cc.egui_ctx);
                let mut app = app_with_library(&format!("pictures-{view}"));
                app.set_language(language);
                let uid = add_plywood(&mut app);
                assert!(app.use_material(Some(&uid)));
                if let Operation::Profile(p) = &mut app.edit.as_mut().unwrap().job.operations[1] {
                    p.feed = 1300.0;
                    if view == "through" {
                        p.through = Some(0.3);
                    }
                }
                app.commit();
                if view == "job-fit" {
                    let stock = app.edit.as_mut().unwrap().job.stock_mut();
                    stock.fit = Some(10.0);
                    stock.origin = chipload_job::StockOrigin::Drawing;
                    app.commit();
                }
                app.focus = match view {
                    "job" | "job-fit" | "job-machine" => Focus::Job,
                    "op" | "through" => Focus::Op(1),
                    _ => Focus::LibraryMaterial(super::super::library_view::uid_key(&uid)),
                };
                app
            });
        harness.run_steps(6);
        if view == "job-machine" {
            // The machine section opened, as a click on its header does.
            use egui_kittest::kittest::Queryable;
            harness.get_by_label_contains("safe Z").click();
            harness.run_steps(4);
        }
        let image = harness.render().unwrap();
        image.save(dir.join(format!("{name}.png"))).unwrap();
    }
}
