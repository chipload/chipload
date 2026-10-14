// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

#[test]
fn the_report_and_the_cards_show_the_estimated_time() {
    let mut app = lower_part();
    assert!(app.generate_and_wait(), "{:?}", app.report);
    let text = format!("{:?}", app.report);
    assert!(text.contains("Estimated time: "), "{text}");
    let estimate = &app.result.as_ref().unwrap().estimate;
    assert_eq!(estimate.operations.len(), 2);
    draw_sidebar(&mut app);
    // Out of date: the times go with the result.
    edit_feed(&mut app, 650.0);
    assert!(app.result.is_none());
    draw_sidebar(&mut app);
}

mod keyboard_shortcuts {
    use super::super::shortcuts::{BINDINGS, OTHERS, keys_text, shortcut_rows, with_keys};
    use super::*;

    #[test]
    fn every_shortcut_is_listed_once_and_shift_ones_come_first() {
        // docs/specs/small-items.md §1
        let listed: Vec<String> = shortcut_rows()
            .into_iter()
            .flat_map(|(_, rows)| rows.into_iter().map(|(keys, _)| keys))
            .collect();
        for b in BINDINGS {
            assert!(listed.contains(&keys_text(&b.keys)), "{:?}", b.action);
        }
        for (i, a) in BINDINGS.iter().enumerate() {
            for later in &BINDINGS[i + 1..] {
                assert_ne!(a.keys, later.keys, "{:?} twice", a.action);
                // egui takes Ctrl+Shift+Z for Ctrl+Z too: the one with more modifiers first.
                let shadows = later.keys.logical_key == a.keys.logical_key
                    && a.keys.modifiers.contains(later.keys.modifiers)
                    && a.keys.modifiers != later.keys.modifiers;
                let shadowed = later.keys.logical_key == a.keys.logical_key
                    && later.keys.modifiers.contains(a.keys.modifiers)
                    && a.keys.modifiers != later.keys.modifiers;
                assert!(
                    shadows || !shadowed,
                    "{:?} must come before {:?}",
                    later.action,
                    a.action
                );
            }
        }
        // The keys handled outside the table are listed too.
        for keys in ["Delete", "Escape", "Enter, double-click", "Backspace"] {
            assert!(OTHERS.iter().any(|(_, k, _)| *k == keys), "{keys}");
        }
        assert_eq!(
            with_keys("Undo", super::super::shortcuts::Action::Undo),
            "Undo  (Ctrl+Z)"
        );
    }

    #[test]
    fn f1_opens_the_list_and_shortcuts_act() {
        let mut app = ChiploadApp::default();
        press(&mut app, egui::Key::F1);
        assert!(app.dialogs.shortcuts_open);
        let mut app = lower_part();
        let before = app.edit.as_ref().unwrap().ops.len();
        press_with(&mut app, egui::Modifiers::COMMAND, egui::Key::Z);
        assert_eq!(
            app.edit.as_ref().unwrap().ops.len(),
            before - 1,
            "Ctrl+Z undid"
        );
        press_with(
            &mut app,
            egui::Modifiers::COMMAND.plus(egui::Modifiers::SHIFT),
            egui::Key::Z,
        );
        assert_eq!(
            app.edit.as_ref().unwrap().ops.len(),
            before,
            "Ctrl+Shift+Z redid, not undid"
        );
    }
}

#[test]
fn dropped_files_open_or_import_after_asking() {
    // docs/specs/small-items.md §2, drawing-sources.md §3
    let mut app = ChiploadApp::default();
    assert!(app.drop_files(&[corpus("dxf/messy_2.dxf")]));
    assert!(!app.contours.is_empty(), "a new project from the DXF");
    // Unsaved changes: ask first.
    assert!(app.dirty());
    assert!(app.drop_files(&[corpus("jobs/v11/two_tools.chipload")]));
    assert!(matches!(
        app.dialogs.prompt,
        Some(Prompt::Discard(Pending::Open(_)))
    ));
    // Nothing else is taken.
    let mut fresh = ChiploadApp::default();
    assert!(!fresh.drop_files(&[PathBuf::from("notes.txt")]));
    assert!(!fresh.drop_files(&[
        corpus("jobs/v11/two_tools.chipload"),
        corpus("dxf/messy_2.dxf")
    ]));
    assert!(fresh.edit.is_none());
    assert!(fresh.drop_files(&[corpus("jobs/v11/two_tools.chipload")]));
    assert_eq!(fresh.edit.as_ref().unwrap().ops.len(), 2);
}

#[test]
fn dropped_dxfs_and_pictures_are_added_one_at_a_time() {
    // docs/specs/drawing-sources.md §3: two DXFs make a project of two.
    let mut app = ChiploadApp::default();
    assert!(app.drop_files(&[corpus("dxf/simple_part.dxf"), corpus("dxf/simple_part.dxf")]));
    let dxfs = |app: &ChiploadApp| app.edit.as_ref().unwrap().settings.drawing.dxfs.len();
    assert_eq!(dxfs(&app), 2);
    // A DXF without units asks, and is added once answered.
    assert!(app.drop_files(&[corpus("dxf/messy.dxf")]));
    assert!(matches!(
        app.dialogs.prompt,
        Some(Prompt::Units(_, UnitsFor::Add))
    ));
    assert!(app.add_dxf_from(&corpus("dxf/messy.dxf"), Some(Units::Mm)));
    assert_eq!(dxfs(&app), 3);
    // Pictures open their window in turn: the second waits for the first.
    assert!(app.drop_files(&[corpus("images/logo.png"), corpus("images/strokes.png")]));
    assert_eq!(app.image_dialog.as_ref().unwrap().draft.file, "logo.png");
    assert_eq!(app.drops.len(), 1);
    assert!(app.apply_image());
    app.next_drop();
    assert_eq!(app.image_dialog.as_ref().unwrap().draft.file, "strokes.png");
    assert!(app.drops.is_empty());
}

#[test]
fn measuring_changes_nothing_in_the_project() {
    // docs/specs/small-items.md §3
    let mut app = drawn_part();
    let before = app.edit.clone();
    press(&mut app, egui::Key::M);
    assert!(app.measure.is_some());
    app.measure_click((10.0, 10.0));
    app.measure_click((40.0, 50.0));
    assert_eq!(
        app.measure_status("").unwrap(),
        "L 50.000  ΔX 30.000  ΔY 40.000  53.1°  · click to measure again, Esc or M ends"
    );
    app.commit();
    assert_eq!(app.edit, before);
    assert!(app.undo.is_empty(), "no undo step");
    // Esc ends it; so does starting to draw.
    press(&mut app, egui::Key::Escape);
    assert!(app.measure.is_none());
    app.toggle_measure();
    app.start_drawing(Tool::Circle);
    assert!(app.measure.is_none() && app.sketch.is_some());
    // From 3D it goes to the top view.
    app.cancel_drawing();
    app.view_3d();
    app.toggle_measure();
    assert!(!app.three_d);
}

mod copy_paste_mirror {
    // docs/specs/small-items.md §4
    use super::*;
    use chipload_job::edit::Mirror;

    /// One frame with these input events through the shortcuts; returns what was put on the
    /// system clipboard.
    fn events(
        app: &mut ChiploadApp,
        events: Vec<egui::Event>,
        modifiers: egui::Modifiers,
    ) -> String {
        let ctx = egui::Context::default();
        let mut all = vec![egui::Event::ModifiersChanged(modifiers)];
        all.extend(events);
        let input = egui::RawInput {
            events: all,
            ..Default::default()
        };
        ctx.begin_pass(input);
        app.shortcuts(&ctx);
        let mut output = ctx.end_pass();
        output.textures_delta.clear();
        output
            .platform_output
            .commands
            .iter()
            .find_map(|c| match c {
                egui::OutputCommand::CopyText(text) => Some(text.clone()),
                _ => None,
            })
            .unwrap_or_default()
    }

    #[test]
    fn copy_and_paste_at_the_cursor_is_one_undo_step() {
        let mut app = drawn_part();
        let shapes = app.edit.as_ref().unwrap().settings.drawing.shapes.len();
        app.click(Some(contour_of(&app, 2)), false);
        // Ctrl+C reaches egui as a Copy event; the text goes to the system clipboard.
        let text = events(&mut app, vec![egui::Event::Copy], egui::Modifiers::COMMAND);
        assert!(text.starts_with("{\"chipload_shapes\":1"), "{text}");
        app.commit();
        app.cursor = Some((200.0, 150.0));
        // Ctrl+V: a Paste event with the clipboard's text.
        events(
            &mut app,
            vec![egui::Event::Paste(text)],
            egui::Modifiers::COMMAND,
        );
        app.commit();
        let drawing = &app.edit.as_ref().unwrap().settings.drawing;
        assert_eq!(drawing.shapes.len(), shapes + 1);
        let new = drawing.shapes.last().unwrap();
        let chipload_job::Geometry::Circle { center, .. } = new.geometry else {
            panic!("a circle was copied")
        };
        assert!(
            (center[0] - 200.0).abs() < 1e-6 && (center[1] - 150.0).abs() < 1e-6,
            "{center:?}"
        );
        assert_eq!(
            app.selection,
            CanvasSelection::Shapes([new.id].into_iter().collect()),
            "the pasted shapes are selected"
        );
        app.undo();
        assert_eq!(
            app.edit.as_ref().unwrap().settings.drawing.shapes.len(),
            shapes
        );
        // Other text on the clipboard pastes nothing.
        events(
            &mut app,
            vec![egui::Event::Paste("hello".into())],
            egui::Modifiers::COMMAND,
        );
        assert_eq!(
            app.edit.as_ref().unwrap().settings.drawing.shapes.len(),
            shapes
        );
        // The Edit menu pastes the last copy 10 mm up and right (no cursor on the canvas).
        app.cursor = None;
        assert!(app.paste(None));
        assert_eq!(
            app.edit.as_ref().unwrap().settings.drawing.shapes.len(),
            shapes + 1
        );
    }

    #[test]
    fn ctrl_shift_c_still_copies_the_selector() {
        let mut app = drawn_part();
        app.click(Some(contour_of(&app, 2)), false);
        let modifiers = egui::Modifiers::COMMAND.plus(egui::Modifiers::SHIFT);
        let text = events(&mut app, vec![egui::Event::Copy], modifiers);
        assert_eq!(text, app.selector_json().unwrap());
    }

    #[test]
    fn mirrored_shapes_keep_their_ids_and_operations_follow() {
        let mut app = drawn_part();
        let before = app.edit.clone().unwrap();
        app.click(Some(contour_of(&app, 1)), false);
        app.click(Some(contour_of(&app, 3)), true);
        assert!(app.mirror_selected(Mirror::LeftRight));
        app.commit();
        let after = app.edit.as_ref().unwrap();
        assert_ne!(after.settings.drawing, before.settings.drawing);
        let ids =
            |e: &EditJob| -> Vec<u64> { e.settings.drawing.shapes.iter().map(|s| s.id).collect() };
        assert_eq!(ids(after), ids(&before));
        assert_eq!(
            app.shape(2),
            before.settings.drawing.shapes.iter().find(|s| s.id == 2)
        );
        assert!(app.generate_and_wait(), "{:?}", app.report);
        app.undo();
        assert_eq!(
            app.edit.as_ref().unwrap().settings.drawing,
            before.settings.drawing
        );
    }

    #[test]
    fn imported_shapes_are_not_copied_or_mirrored() {
        let mut app = lower_part();
        select(&mut app, &[2]);
        assert_eq!(app.copy_selected_shapes(), None);
        assert!(!app.mirror_selected(Mirror::UpDown));
        assert!(app.status.as_ref().unwrap().0.contains("only drawn ones"));
    }
}
