// SPDX-License-Identifier: GPL-3.0-or-later
//! *New sign…* (docs/specs/sign-maker.md §3): the dialog shows the sign and makes it, the
//! words, the frame and the operations, as one undo step.

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;

use super::*;
use crate::app::wizard_tools::WizardTool;

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
    assert_eq!(settings.vbit, Some(WizardTool::Project(2)));
    assert_eq!(settings.end_mill, Some(WizardTool::Project(1)));
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
        corner_radius: None,
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
    assert_eq!(
        settings.vbit,
        Some(WizardTool::Library(uid_for_name("V 60")))
    );
    assert_eq!(
        settings.end_mill,
        Some(WizardTool::Library(uid_for_name("6 mm upcut")))
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

#[test]
fn a_sign_of_a_fixed_size_shows_its_letters_height_and_is_that_size() {
    // docs/specs/sign-size-and-start.md §1.
    let mut harness = window();
    harness.state_mut().start_sign();
    harness.run_steps(2);
    let Some(Prompt::Sign(mut settings)) = harness.state().dialogs.prompt.clone() else {
        panic!("the dialog");
    };
    settings.words = "SMITH".into();
    settings.fixed = true;
    settings.size = (300.0, 100.0);
    harness.state_mut().dialogs.prompt = Some(Prompt::Sign(settings));
    harness.run_steps(3);
    // The letters' height found shows instead of the height field.
    assert!(harness.get_all_by_label("Letters").count() > 0);
    assert!(harness.get_all_by_label("Outer size").count() > 0);
    click(&mut harness, "Make sign");
    let job = &harness.state().edit.as_ref().unwrap().job;
    let outline = job.drawing.frames[0].outline;
    let ((x0, y0), (x1, y1)) = chipload_pipeline::shapes::drawn_box(job, &[outline]).unwrap();
    assert!(((x1 - x0) - 300.0).abs() < 1e-6 && ((y1 - y0) - 100.0).abs() < 1e-6);
    let height = job.drawing.texts[0].height;
    // One word: the height limits it, 100 less the margin, gap and border each side.
    assert!(height > 40.0 && height < 80.0, "{height}");
}

#[test]
fn new_sign_on_the_start_page_makes_a_project_and_opens_the_dialog() {
    // docs/specs/sign-size-and-start.md §2.
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1400.0, 1000.0))
        .build_eframe(|_| ChiploadApp::default());
    harness.run_steps(2);
    assert!(harness.state().edit.is_none());
    click(&mut harness, "New sign…");
    let app = harness.state();
    assert!(app.edit.is_some(), "a new project");
    assert!(
        matches!(app.dialogs.prompt, Some(Prompt::Sign(_))),
        "the sign's dialog"
    );
    assert!(app.edit.as_ref().unwrap().job.drawing.shapes.is_empty());
}

#[test]
fn a_single_line_font_shows_the_engrave_depth_and_makes_a_follow_path() {
    // docs/specs/single-line-text.md §2.
    let mut harness = window();
    harness.state_mut().start_sign();
    harness.run_steps(2);
    let Some(Prompt::Sign(mut settings)) = harness.state().dialogs.prompt.clone() else {
        panic!("the dialog");
    };
    settings.words = "SHED".into();
    settings.font = "Relief SingleLine".into();
    settings.engrave_depth = 0.7;
    harness.state_mut().dialogs.prompt = Some(Prompt::Sign(settings));
    harness.run_steps(3);
    assert!(harness.get_all_by_label("Engrave depth").count() > 0);
    click(&mut harness, "Make sign");
    let job = &harness.state().edit.as_ref().unwrap().job;
    let Operation::Path(letters) = &job.operations[0] else {
        panic!("a follow path");
    };
    assert_eq!(letters.end_depth, 0.7);
}

#[test]
fn signs_from_a_list_are_made_in_one_part_and_the_preview_counts_them() {
    // docs/specs/batch-signs.md §2.
    let mut harness = window();
    harness.state_mut().start_sign();
    harness.run_steps(2);
    let Some(Prompt::Sign(mut settings)) = harness.state().dialogs.prompt.clone() else {
        panic!("the dialog");
    };
    settings.from_list = true;
    settings.fixed = true;
    settings.size = (160.0, 50.0);
    settings.list = "ANNA | ROOM 1\nBO\nCARL\n".into();
    harness.state_mut().dialogs.prompt = Some(Prompt::Sign(settings));
    harness.run_steps(3);
    assert!(harness.get_all_by_label("3 signs, in a grid").count() > 0);
    click(&mut harness, "Make sign");
    let job = &harness.state().edit.as_ref().unwrap().job;
    assert_eq!(job.drawing.frames.len(), 3);
    assert_eq!(job.drawing.texts.len(), 3);
    let names: Vec<&str> = job.operations.iter().map(|op| op.name()).collect();
    assert_eq!(names, ["Letters", "Borders", "Cut out"]);
}

#[test]
fn new_with_a_project_open_offers_the_start_screens_choices() {
    // The user, 2026-11-23: New shows what the start screen does, not the board at once.
    let mut harness = window();
    harness.state_mut().ask_new_choice();
    harness.run_steps(2);
    assert!(matches!(
        harness.state().dialogs.prompt,
        Some(Prompt::Start)
    ));
    click(&mut harness, "Cancel");
    assert!(harness.state().dialogs.prompt.is_none());
    assert!(harness.state().edit.is_some(), "the project stays");
    harness.state_mut().ask_new_choice();
    harness.run_steps(2);
    click(&mut harness, "New sign…");
    // The project has unsaved changes: asked first, then the sign, not the board.
    assert!(matches!(
        harness.state().dialogs.prompt,
        Some(Prompt::Discard(Pending::NewSign))
    ));
    click(&mut harness, "Don't save");
    assert!(matches!(
        harness.state().dialogs.prompt,
        Some(Prompt::Sign(_))
    ));
    harness.state_mut().dialogs.prompt = None;
    harness.state_mut().ask_new_choice();
    harness.run_steps(2);
    click(&mut harness, "New project");
    // The sign's new project isn't saved yet: asked first, then the board.
    assert!(matches!(
        harness.state().dialogs.prompt,
        Some(Prompt::Discard(Pending::New))
    ));
    click(&mut harness, "Don't save");
    assert!(matches!(
        harness.state().dialogs.prompt,
        Some(Prompt::NewProject(_))
    ));
}
