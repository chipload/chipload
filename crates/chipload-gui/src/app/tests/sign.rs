// SPDX-License-Identifier: GPL-3.0-or-later
//! *New sign…* (docs/specs/sign-maker.md §3): the dialog shows the sign and makes it, the
//! words, the frame and the operations, as one undo step.

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;

use super::*;
use crate::app::sign::SignTool;

/// The corpus's tools (1 an end mill, 2 a V-bit) in an empty project.
fn window() -> Harness<'static, ChiploadApp> {
    let project = corpus("jobs/v27/text.chipload");
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1400.0, 1000.0))
        .build_eframe(move |_| {
            let mut app = ChiploadApp::default();
            assert!(app.open(&project));
            let edit = app.edit.as_mut().unwrap();
            edit.job.operations.clear();
            edit.job.drawing = chipload_pipeline::empty_project().drawing;
            app.commit();
            app
        });
    harness.run_steps(4);
    harness
}

fn click(harness: &mut Harness<'_, ChiploadApp>, label: &str) {
    let pos = harness
        .get_all_by_label(label)
        .last()
        .unwrap_or_else(|| panic!("{label}"))
        .rect()
        .center();
    harness.hover_at(pos);
    harness.run_steps(1);
    for pressed in [true, false] {
        harness.event(egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
        harness.run_steps(1);
    }
    harness.run_steps(2);
}

#[test]
fn new_sign_makes_the_whole_sign_as_one_undo_step() {
    let mut harness = window();
    harness.state_mut().start_sign();
    harness.run_steps(2);
    let Some(Prompt::Sign(settings)) = harness.state().dialogs.prompt.clone() else {
        panic!("the dialog");
    };
    // The project's V-bit and end mill are chosen; the board is fitted (nothing drawn).
    assert_eq!(settings.vbit, Some(SignTool::Project(2)));
    assert_eq!(settings.end_mill, Some(SignTool::Project(1)));
    assert!(settings.fit_board);
    let mut settings = settings;
    settings.words = "OPEN".into();
    settings.edge = true;
    harness.state_mut().dialogs.prompt = Some(Prompt::Sign(settings));
    harness.run_steps(3);
    assert!(harness.get_all_by_label("Make sign").count() > 0);
    click(&mut harness, "Make sign");
    assert!(harness.state().dialogs.prompt.is_none());
    let job = &harness.state().edit.as_ref().unwrap().job;
    let names: Vec<&str> = job.operations.iter().map(|op| op.name()).collect();
    assert_eq!(names, ["Letters", "Borders", "Edge chamfer", "Cut out"]);
    assert_eq!(job.drawing.frames.len(), 1);
    harness.state_mut().undo();
    let job = &harness.state().edit.as_ref().unwrap().job;
    assert!(job.operations.is_empty() && job.drawing.shapes.is_empty());
}

#[test]
fn new_sign_with_a_picture_selected_goes_round_it() {
    let project = corpus("jobs/v19/image_vcarve.chipload");
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1400.0, 1000.0))
        .build_eframe(move |_| {
            let mut app = ChiploadApp::default();
            assert!(app.open(&project));
            let tools = chipload_job::load_job(&corpus("jobs/v27/text.chipload"))
                .unwrap()
                .tools;
            let edit = app.edit.as_mut().unwrap();
            edit.job.operations.clear();
            edit.job.tools = tools;
            app.commit();
            app
        });
    harness.run_steps(4);
    harness.state_mut().selection =
        CanvasSelection::whole(chipload_core::chain::ShapeFrom::Image(1));
    harness.state_mut().start_sign();
    harness.run_steps(3);
    let Some(Prompt::Sign(settings)) = harness.state().dialogs.prompt.clone() else {
        panic!("the dialog");
    };
    assert_eq!(settings.around, vec![1, 2, 3, 4]);
    assert!(harness.get_all_by_label("Round the selection").count() > 0);
    click(&mut harness, "Make sign");
    let job = &harness.state().edit.as_ref().unwrap().job;
    let names: Vec<&str> = job.operations.iter().map(|op| op.name()).collect();
    assert_eq!(names, ["Carving", "Borders", "Cut out"]);
    assert_eq!(
        job.drawing.frames[0].around.as_ref().unwrap().images,
        vec![1]
    );
}

#[test]
fn the_sign_takes_the_font_last_picked_and_its_group_lists_the_words() {
    let mut harness = window();
    harness.state_mut().text_fonts.recent = vec!["FreeMono".into()];
    harness.state_mut().start_sign();
    harness.run_steps(2);
    let Some(Prompt::Sign(mut settings)) = harness.state().dialogs.prompt.clone() else {
        panic!("the dialog");
    };
    // The text tool's font list: the recently used fonts first (the user, 2026-11-10).
    assert_eq!(settings.font, "FreeMono");
    settings.words = "OPEN".into();
    harness.state_mut().dialogs.prompt = Some(Prompt::Sign(settings));
    harness.run_steps(3);
    click(&mut harness, "Make sign");
    let app = harness.state();
    let job = &app.edit.as_ref().unwrap().job;
    assert_eq!(job.drawing.texts[0].font, "FreeMono");
    // The Browser lists the words in the sign's group, beside the frame, and not again
    // under Drawing.
    let groups = app.drawing_groups();
    assert!(groups.iter().all(|g| !g.key.starts_with("text:")));
    let sign = groups
        .iter()
        .find(|g| g.key.starts_with("group:"))
        .expect("the sign's group");
    assert_eq!(sign.texts.len(), 1);
    assert_eq!(
        sign.texts[0].key,
        format!("text:{}", job.drawing.texts[0].id)
    );
    let framed: usize = sign.members.iter().map(|(_, m)| m.len()).sum();
    assert_eq!(framed, job.drawing.frames[0].shapes().count());
    assert_eq!(sign.shapes, framed + sign.texts[0].shapes);
}

#[test]
fn a_library_tool_is_copied_in_only_when_the_sign_is_made() {
    use chipload_job::library::{LibraryTool, ToolLibrary, save_tools, uid_for_name};
    let tool = |name: &str, kind, diameter, angle| LibraryTool {
        name: name.into(),
        uid: uid_for_name(name),
        id: 7,
        kind,
        diameter,
        flutes: Some(2),
        center_cutting: Some(true),
        rpm: Some(18000.0),
        cutting_data: Vec::new(),
        angle,
        tip_diameter: None,
    };
    let dir = temp_dir("sign-library");
    let library = ToolLibrary {
        tools: vec![
            tool("V 60", ToolType::Vbit, 12.7, Some(60f64.to_radians())),
            tool("6 mm upcut", ToolType::Endmill, 6.0, None),
        ],
        ..ToolLibrary::default()
    };
    save_tools(&library, &chipload_pipeline::library::tools_path(&dir)).unwrap();
    let mut harness = window();
    harness.state_mut().set_settings_dir(Some(dir));
    harness.state_mut().edit.as_mut().unwrap().job.tools.clear();
    harness.state_mut().commit();
    // A project without tools starts on the library's (the user, 2026-11-10).
    harness.state_mut().start_sign();
    harness.run_steps(2);
    let Some(Prompt::Sign(mut settings)) = harness.state().dialogs.prompt.clone() else {
        panic!("the dialog");
    };
    assert_eq!(settings.vbit, Some(SignTool::Library(uid_for_name("V 60"))));
    assert_eq!(
        settings.end_mill,
        Some(SignTool::Library(uid_for_name("6 mm upcut")))
    );
    settings.words = "OPEN".into();
    harness.state_mut().dialogs.prompt = Some(Prompt::Sign(settings.clone()));
    harness.run_steps(3);
    // The dialog shows the sign without touching the project.
    assert!(harness.state().edit.as_ref().unwrap().job.tools.is_empty());
    assert!(harness.get_all_by_label("Make sign").count() > 0);
    click(&mut harness, "Make sign");
    let job = &harness.state().edit.as_ref().unwrap().job;
    let names: Vec<Option<&str>> = job.tools.iter().map(|t| t.name.as_deref()).collect();
    assert_eq!(names.len(), 2, "{names:?}");
    assert!(job.tools.iter().all(|t| t.library_uid.is_some()));
    assert_eq!(job.drawing.frames.len(), 1);
    // One undo step takes the sign and its tools away.
    harness.state_mut().undo();
    assert!(harness.state().edit.as_ref().unwrap().job.tools.is_empty());
}
