// SPDX-License-Identifier: GPL-3.0-or-later
//! The new project dialog (docs/specs/new-project-dialog.md): it opens for New project,
//! creates with the choices, remembers them for the next project, and Cancel keeps the
//! open project.

use chipload_job::ZZero;
use chipload_job::library::{LibraryTool, ToolLibrary, save_library, save_tools, uid_for_name};
use chipload_job::materials::{Material, MaterialLibrary};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;

use super::*;

fn tool(name: &str, id: i64, kind: ToolType, diameter: f64) -> LibraryTool {
    LibraryTool {
        name: name.into(),
        uid: uid_for_name(name),
        id,
        kind,
        diameter,
        flutes: Some(2),
        center_cutting: Some(true),
        rpm: Some(18000.0),
        cutting_data: Vec::new(),
        angle: (kind == ToolType::Vbit).then(|| 60f64.to_radians()),
        tip_diameter: None,
        corner_radius: None,
    }
}

/// A settings folder with three tools (6 mm the default) and plywood 18 mm as the default
/// material.
fn settings(name: &str) -> PathBuf {
    let dir = temp_dir(name);
    let tools = ToolLibrary {
        tools: vec![
            tool("6 mm", 1, ToolType::Endmill, 6.0),
            tool("V 60", 1, ToolType::Vbit, 12.7),
            tool("3 mm", 3, ToolType::Endmill, 3.0),
        ],
        default_tool: Some("6 mm".into()),
        ..ToolLibrary::default()
    };
    save_tools(&tools, &chipload_pipeline::library::tools_path(&dir)).unwrap();
    let materials = MaterialLibrary {
        materials: vec![Material {
            name: "Plywood".into(),
            uid: uid_for_name("Plywood"),
            thickness: Some(18.0),
        }],
        default_material: Some("Plywood".into()),
        ..MaterialLibrary::default()
    };
    save_library(
        &materials,
        &chipload_pipeline::library::materials_path(&dir),
    )
    .unwrap();
    dir
}

fn window(dir: PathBuf) -> Harness<'static, ChiploadApp> {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1400.0, 900.0))
        .build_eframe(move |cc| {
            cc.egui_ctx.set_fonts(crate::fonts::ui_fonts());
            let mut app = ChiploadApp::default();
            app.set_settings_dir(Some(dir.clone()));
            app
        });
    harness.run_steps(2);
    harness
}

#[test]
fn new_project_asks_then_remembers_every_choice() {
    let dir = settings("new-project-dialog");
    let mut harness = window(dir.clone());
    harness.state_mut().ask_new();
    harness.run_steps(3);
    assert!(harness.state().edit.is_none(), "nothing made before Create");
    assert!(harness.state().new_project_open());
    // The first time: the library's defaults, the material's thickness.
    let choice = harness.state_mut().new_project_choice().unwrap().clone();
    assert_eq!(choice.material, Some(uid_for_name("Plywood")));
    assert_eq!(choice.thickness, 18.0);
    assert_eq!(choice.tools, [uid_for_name("6 mm")]);
    assert_eq!(choice.z_zero, ZZero::Top);
    // Choose: 12 mm, Z zero on the spoilboard, the V-bit too, a fixed size.
    {
        let choice = harness.state_mut().new_project_choice().unwrap();
        choice.thickness = 12.0;
        choice.z_zero = ZZero::Bottom;
        choice.tools.push(uid_for_name("V 60"));
        choice.fit = None;
        choice.x = 500.0;
        choice.y = 250.0;
    }
    harness.run_steps(2);
    harness.get_by_label("Create").click();
    harness.run_steps(3);
    let job = harness.state().edit.as_ref().unwrap().job.clone();
    assert_eq!(job.stock().thickness, 12.0);
    assert_eq!(job.stock().z_zero, ZZero::Bottom);
    assert_eq!(
        (job.stock().x, job.stock().y, job.stock().fit),
        (500.0, 250.0, None)
    );
    let tools: Vec<(i64, &str)> = job
        .tools
        .iter()
        .map(|t| (t.id, t.name.as_deref().unwrap()))
        .collect();
    assert_eq!(tools, [(1, "6 mm"), (2, "V 60")]);
    // Remembered, in the settings file too.
    let saved =
        chipload_job::library::load_settings(&chipload_pipeline::library::settings_path(&dir))
            .unwrap();
    assert_eq!(saved.new_project.as_ref().unwrap().thickness, 12.0);
    // The next New project starts from them.
    harness.state_mut().ask_new_project();
    harness.run_steps(2);
    let next = harness.state_mut().new_project_choice().unwrap().clone();
    assert_eq!(next.thickness, 12.0);
    assert_eq!(next.z_zero, ZZero::Bottom);
    assert_eq!(next.tools.len(), 2);
    // Cancel keeps the open project.
    assert!(harness.state_mut().cancel_new_project());
    assert!(harness.state().dialogs.prompt.is_none());
    assert_eq!(
        harness.state().edit.as_ref().unwrap().job.stock().thickness,
        12.0
    );
}

#[test]
fn a_tool_that_left_the_library_is_no_longer_ticked() {
    let dir = settings("new-project-gone");
    let mut harness = window(dir.clone());
    harness.state_mut().prefs.new_project = Some(chipload_job::library::NewProject {
        machine: Some("0000000000000000".into()),
        material: Some("0000000000000000".into()),
        thickness: 9.0,
        x: 100.0,
        y: 100.0,
        fit: Some(5.0),
        z_zero: ZZero::Top,
        tools: vec![uid_for_name("3 mm"), "0000000000000000".into()],
    });
    harness.state_mut().ask_new_project();
    harness.run_steps(2);
    let choice = harness.state_mut().new_project_choice().unwrap().clone();
    assert_eq!(choice.tools, [uid_for_name("3 mm")]);
    // The material falls back to the default; no machines: none.
    assert_eq!(choice.material, Some(uid_for_name("Plywood")));
    assert_eq!(choice.machine, None);
    assert_eq!(choice.thickness, 9.0);
}

#[test]
fn the_dialog_opens_in_every_language() {
    let dir = settings("new-project-languages");
    for language in crate::i18n::Language::ALL {
        let mut harness = Harness::builder()
            .with_size(egui::vec2(900.0, 700.0))
            .build_eframe({
                let dir = dir.clone();
                move |_| {
                    let mut app = ChiploadApp::default();
                    app.set_settings_dir(Some(dir.clone()));
                    app
                }
            });
        harness.state_mut().set_language(language.code());
        harness.state_mut().ask_new_project();
        harness.run_steps(4);
        assert!(
            harness.state().new_project_open(),
            "{language:?}: the dialog is open"
        );
    }
}

#[test]
#[ignore = "a picture for looking at the dialog"]
fn dialog_picture() {
    let dir = settings("new-project-picture");
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1400.0, 900.0))
        .wgpu()
        .build_eframe(move |cc| {
            cc.egui_ctx.set_fonts(crate::fonts::ui_fonts());
            crate::style::apply(&cc.egui_ctx);
            let mut app = ChiploadApp::default();
            app.set_settings_dir(Some(dir.clone()));
            app.set_language("en");
            app
        });
    harness.state_mut().ask_new_project();
    harness.run_steps(6);
    let out = std::env::var("CHIPLOAD_SHOTS_OUT").unwrap_or_else(|_| ".".into());
    harness
        .render()
        .unwrap()
        .save(PathBuf::from(out).join("new-project.png"))
        .unwrap();
}
