// SPDX-License-Identifier: GPL-3.0-or-later
//! *Turn and cut…* (docs/specs/relief-wizard.md §4): the model seen while it is turned and
//! cut; OK changes it as one undo step, Cancel leaves it.

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;

use super::*;

fn window() -> Harness<'static, ChiploadApp> {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1400.0, 1000.0))
        .build_eframe(move |_| {
            let mut app = ChiploadApp::default();
            assert!(app.open(&corpus("jobs/v36/relief_dome.chipload")));
            app.open_turn_cut(1);
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

fn model(harness: &Harness<'_, ChiploadApp>) -> chipload_job::Model {
    harness.state().edit.as_ref().unwrap().job.drawing.models[0].clone()
}

#[test]
fn ok_turns_and_cuts_the_model_as_one_undo_step() {
    let mut harness = window();
    let before = model(&harness);
    // Turned a quarter and its top half kept, in the window.
    click(&mut harness, "⟲ 90°");
    let draft = super::super::turn_cut::draft_of(harness.state_mut()).unwrap();
    assert_eq!(draft.rotation, 90.0);
    draft.keep = 0.5;
    harness.run_steps(2);
    // Not applied until OK.
    assert_eq!(model(&harness), before);
    click(&mut harness, "OK");
    assert!(harness.state().dialogs.prompt.is_none());
    let after = model(&harness);
    assert_eq!((after.rotation, after.keep), (90.0, 0.5));
    harness.state_mut().undo();
    assert_eq!(model(&harness), before);
}

#[test]
fn cancel_leaves_the_model_and_faces_change_the_view() {
    let mut harness = window();
    let before = model(&harness);
    click(&mut harness, "−Z");
    assert_eq!(
        super::super::turn_cut::draft_of(harness.state_mut())
            .unwrap()
            .up,
        chipload_job::ModelUp::MinusZ
    );
    click(&mut harness, "Cancel");
    assert!(harness.state().dialogs.prompt.is_none());
    assert_eq!(model(&harness), before);
}

/// The window as a picture, to look at:
/// `CHIPLOAD_PICTURES=/some/dir cargo test -p chipload-gui turn_cut_picture -- --ignored`.
#[test]
#[ignore = "writes a picture; run by hand"]
fn turn_cut_picture() {
    let Ok(dir) = std::env::var("CHIPLOAD_PICTURES") else {
        return;
    };
    let dir = std::path::PathBuf::from(dir);
    std::fs::create_dir_all(&dir).unwrap();
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
            let file = std::env::var("CHIPLOAD_STL").ok();
            match file {
                Some(file) => {
                    app.new_project();
                    assert!(app.add_model_from(std::path::Path::new(&file)));
                    assert!(app.wait_model_reading());
                    if let Some(draft) = super::super::turn_cut::draft_of(&mut app) {
                        draft.keep = 0.5;
                    }
                }
                None => {
                    assert!(app.open(&corpus("jobs/v36/relief_dome.chipload")));
                    app.open_turn_cut(1);
                }
            }
            app
        });
    harness.run_steps(8);
    let image = harness.render().unwrap();
    image.save(dir.join("turn-cut.png")).unwrap();
}
