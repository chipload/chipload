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
                if let Ok(mut fonts) = crate::fonts::system_fonts() {
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
