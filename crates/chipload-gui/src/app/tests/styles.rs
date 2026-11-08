// SPDX-License-Identifier: GPL-3.0-or-later
//! Styles in the window (docs/specs/styles.md §3).
use super::*;
use chipload_job::library::{LibraryTool, ToolLibrary, save_tools, uid_for_name};

const TOOL: &str = "6 mm 2F";

/// simple_part imported, with a settings folder whose library has its tool, the project's
/// tool linked to it. Contour 1 is the outline, 2-4 its holes.
fn app_with_library(name: &str) -> ChiploadApp {
    let dir = temp_dir(&format!("styles-{name}"));
    for file in [
        "tools.json",
        "machines.json",
        "materials.json",
        "styles.json",
    ] {
        let _ = std::fs::remove_file(dir.join(file));
    }
    let mut app = ChiploadApp::default();
    assert!(app.import_dxf(&corpus("dxf/simple_part.dxf"), None));
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

fn ops(app: &ChiploadApp) -> Vec<String> {
    app.edit
        .as_ref()
        .unwrap()
        .job
        .operations
        .iter()
        .map(|op| op.name().to_string())
        .collect()
}

#[test]
fn an_operation_saved_as_a_style_is_applied_to_a_selection_in_one_undo_step() {
    let mut app = app_with_library("apply");
    select(&mut app, &[1]);
    let i = app.add_operation(OpKind::Profile).unwrap();
    assert!(app.save_style(i, "Cut out"));
    assert_eq!(app.user.library.styles.styles.len(), 1);
    assert!(
        app.status
            .as_ref()
            .is_some_and(|(s, _)| s.contains("Cut out"))
    );
    // The whole part: its holes inside first, then the outline.
    select(&mut app, &[1, 2, 3, 4]);
    let uid = app.user.library.styles.styles[0].uid.clone();
    let first = app.apply_style(&uid).unwrap();
    assert_eq!(first, 1);
    assert_eq!(
        ops(&app),
        ["Profile 1", "Cut out (holes)", "Cut out (outline)"]
    );
    assert_eq!(app.focus, Focus::Op(1));
    app.undo();
    assert_eq!(ops(&app), ["Profile 1"]);
    // Saved again under its name: replaced, still one style.
    assert!(app.save_style(0, "cut out"));
    assert_eq!(app.user.library.styles.styles.len(), 1);
}

#[test]
fn a_tool_not_in_the_library_cant_be_saved_in_a_style() {
    let mut app = app_with_library("refused");
    app.edit.as_mut().unwrap().job.tools[0].library_uid = None;
    app.commit();
    select(&mut app, &[1]);
    let i = app.add_operation(OpKind::Profile).unwrap();
    assert!(!app.save_style(i, "Cut out"));
    assert!(app.user.library.styles.styles.is_empty());
    assert!(
        app.report
            .iter()
            .any(|l| l.contains("isn't in your library")),
        "{:?}",
        app.report
    );
}

#[test]
fn styles_show_in_the_browser_ribbon_and_properties() {
    use egui_kittest::Harness;
    use egui_kittest::kittest::Queryable;
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1400.0, 900.0))
        .build_eframe(|_| {
            let mut app = app_with_library("window");
            select(&mut app, &[1]);
            let i = app.add_operation(OpKind::Profile).unwrap();
            assert!(app.save_style(i, "Cut out"));
            app
        });
    harness.run_steps(4);
    assert!(
        harness.query_by_label("Style").is_some(),
        "the ribbon's button"
    );
    harness.state_mut().focus = Focus::LibraryStyles;
    harness.run_steps(4);
    assert!(
        harness
            .query_by_label_contains("1 style in your library")
            .is_some(),
        "the folder's properties"
    );
    // An operation's properties offer to save it.
    harness.state_mut().focus = Focus::Op(0);
    harness.run_steps(4);
    harness.get_by_label("Save as style…").click();
    harness.run_steps(4);
    assert!(matches!(
        harness.state().dialogs.prompt,
        Some(Prompt::StyleName(0, _))
    ));
    harness.state_mut().dialogs.prompt = None;
    // The style's form, and the Delete key.
    let uid = harness.state().user.library.styles.styles[0].uid.clone();
    let key = super::super::library_view::uid_key(&uid);
    harness.state_mut().focus = Focus::LibraryStyle(key);
    harness.run_steps(4);
    assert!(harness.query_by_label("Apply to the selection").is_some());
    harness.state_mut().delete_key();
    harness.run_steps(2);
    assert!(harness.state().user.library.styles.styles.is_empty());
    assert_eq!(harness.state().focus, Focus::LibraryStyles);
}

/// A style's form is its operation's form (docs/specs/styles.md §9, step 3b): a change is
/// a draft until Save, Apply waits for it, and Save changes the style, not the operations
/// made with it.
#[test]
fn a_style_is_edited_in_its_operations_form_and_saved() {
    use egui_kittest::Harness;
    use egui_kittest::kittest::{NodeT, Queryable};
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1400.0, 900.0))
        .build_eframe(|_| {
            let mut app = app_with_library("edit");
            select(&mut app, &[1]);
            let i = app.add_operation(OpKind::Profile).unwrap();
            assert!(app.save_style(i, "Cut out"));
            let uid = app.user.library.styles.styles[0].uid.clone();
            app.focus = Focus::LibraryStyle(super::super::library_view::uid_key(&uid));
            app
        });
    harness.run_steps(4);
    let uid = harness.state().user.library.styles.styles[0].uid.clone();
    // The operation's rows: its tool, its side as a style's, its sections (the pictures
    // show the tool by its name: a combo's text isn't its label).
    assert!(harness.query_by_label("Tool").is_some());
    assert!(harness.query_by_label("Side").is_some());
    assert!(harness.query_by_label_contains("Holding").is_some());
    assert!(
        harness.query_by_label("Use selection").is_none(),
        "no shapes"
    );
    // A change: Save offered, Apply waits.
    let draft = chipload_pipeline::styles::style_draft(
        &harness.state().user.library.styles.styles[0],
        &harness.state().user.library.tools,
    )
    .unwrap();
    let mut op = draft.operation.clone();
    let Operation::Profile(p) = &mut op else {
        panic!("a profile");
    };
    p.tabs.as_mut().unwrap().count = 6;
    harness
        .state_mut()
        .user
        .library_state
        .style_drafts
        .insert(uid.clone(), op);
    harness.run_steps(3);
    assert!(harness.query_by_label("not saved").is_some());
    assert!(
        harness
            .get_by_label("Apply to the selection")
            .accesskit_node()
            .is_disabled()
    );
    // The form's Save, right of the ribbon's.
    harness
        .query_all_by_label("Save")
        .max_by(|a, b| a.rect().left().total_cmp(&b.rect().left()))
        .expect("the form's Save")
        .click();
    harness.run_steps(3);
    let style = harness.state().user.library.styles.styles[0].clone();
    let Operation::Profile(saved) = style.operation().unwrap() else {
        panic!("a profile");
    };
    assert_eq!(saved.tabs.unwrap().count, 6);
    assert!(harness.state().user.library_state.style_drafts.is_empty());
    // The operation it was saved from keeps its four tabs.
    let Operation::Profile(made) = &harness.state().edit.as_ref().unwrap().job.operations[0] else {
        panic!("a profile");
    };
    assert_eq!(made.tabs.as_ref().unwrap().count, 4);
    // Read again from the file: the same.
    harness.state_mut().reload_library();
    let again = harness.state().user.library.styles.styles[0].clone();
    assert_eq!(again, style);
}

/// The styles in the window rendered to PNG files to look at, as `layout::pictures`:
/// `CHIPLOAD_PICTURES=/some/dir cargo test -p chipload-gui style_pictures -- --ignored`.
#[test]
#[ignore = "writes pictures; run by hand"]
fn style_pictures() {
    use egui_kittest::Harness;
    let Ok(dir) = std::env::var("CHIPLOAD_PICTURES") else {
        panic!("set CHIPLOAD_PICTURES to the folder to write to");
    };
    let dir = PathBuf::from(dir);
    std::fs::create_dir_all(&dir).unwrap();
    for language in ["en", "de", "ru"] {
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
                cc.egui_ctx.data_mut(|d| {
                    d.insert_persisted(egui::Id::new("browser-library").with("folder-open"), true);
                });
                let mut app = app_with_library(&format!("pictures-{language}"));
                app.set_language(language);
                select(&mut app, &[1]);
                let i = app.add_operation(OpKind::Profile).unwrap();
                assert!(app.save_style(i, "Cut out 10 mm"));
                select(&mut app, &[2]);
                let i = app.add_operation(OpKind::Pocket).unwrap();
                assert!(app.save_style(i, "Pocket 3 mm"));
                select(&mut app, &[1, 2, 3, 4]);
                let uid = app.user.library.styles.styles[0].uid.clone();
                app.focus = Focus::LibraryStyle(super::super::library_view::uid_key(&uid));
                app
            });
        harness.run_steps(6);
        let image = harness.render().unwrap();
        image
            .save(dir.join(format!("styles-{language}.png")))
            .unwrap();
    }
}

/// The canvas' right-click menu with styles, rendered to look at (as `style_pictures`).
#[test]
#[ignore = "writes pictures; run by hand"]
fn style_menu_pictures() {
    use egui_kittest::Harness;
    use egui_kittest::kittest::Queryable;
    let Ok(dir) = std::env::var("CHIPLOAD_PICTURES") else {
        panic!("set CHIPLOAD_PICTURES to the folder to write to");
    };
    let dir = PathBuf::from(dir);
    std::fs::create_dir_all(&dir).unwrap();
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
            let mut app = app_with_library("menu-pictures");
            select(&mut app, &[1]);
            let i = app.add_operation(OpKind::Profile).unwrap();
            assert!(app.save_style(i, "Cut out 10 mm"));
            select(&mut app, &[1, 2, 3, 4]);
            app.focus = Focus::Selection;
            app
        });
    harness.run_steps(6);
    // Inside the part's outline on the canvas, at the default size.
    let at = egui::pos2(470.0, 430.0);
    harness.hover_at(at);
    harness.event(egui::Event::PointerButton {
        pos: at,
        button: egui::PointerButton::Secondary,
        pressed: true,
        modifiers: egui::Modifiers::NONE,
    });
    harness.event(egui::Event::PointerButton {
        pos: at,
        button: egui::PointerButton::Secondary,
        pressed: false,
        modifiers: egui::Modifiers::NONE,
    });
    harness.run_steps(4);
    if let Some(item) = harness.query_by_label("Apply style") {
        item.hover();
    }
    harness.run_steps(4);
    let image = harness.render().unwrap();
    image.save(dir.join("style-menu.png")).unwrap();
}

/// A profile's and a pocket's properties rendered to look at (as `style_pictures`).
#[test]
#[ignore = "writes pictures; run by hand"]
fn operation_form_pictures() {
    use egui_kittest::Harness;
    let Ok(dir) = std::env::var("CHIPLOAD_PICTURES") else {
        panic!("set CHIPLOAD_PICTURES to the folder to write to");
    };
    let dir = PathBuf::from(dir);
    std::fs::create_dir_all(&dir).unwrap();
    for (name, kind) in [
        ("form-profile", OpKind::Profile),
        ("form-profile-open", OpKind::Profile),
        ("form-pocket", OpKind::Pocket),
    ] {
        let mut harness = Harness::builder()
            .with_size(egui::vec2(1200.0, 900.0))
            .wgpu()
            .build_eframe(move |cc| {
                {
                    let mut fonts = crate::fonts::ui_fonts();
                    crate::fonts::add_text_fonts(&mut fonts);
                    cc.egui_ctx.set_fonts(fonts);
                }
                crate::style::apply(&cc.egui_ctx);
                let mut app = app_with_library(&format!("{name}-pictures"));
                select(&mut app, &[if kind == OpKind::Pocket { 2 } else { 1 }]);
                let i = app.add_operation(kind).unwrap();
                app.focus = Focus::Op(i);
                app
            });
        harness.run_steps(6);
        if name.ends_with("-open") {
            use egui_kittest::kittest::Queryable;
            for section in ["Cut", "Holding"] {
                harness.get_by_label_contains(section).click();
                harness.run_steps(4);
            }
        }
        let image = harness.render().unwrap();
        image.save(dir.join(format!("{name}.png"))).unwrap();
    }
}
