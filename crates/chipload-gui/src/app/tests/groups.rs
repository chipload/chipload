// SPDX-License-Identifier: GPL-3.0-or-later
//! Groups in the window (docs/specs/shape-groups.md §5): picked whole, Ctrl+click leaves
//! out, double-click picks one shape, Group and Ungroup, moved whole, deleted.

use super::*;

/// The golden sign: text 1 (S1-S4), the frame S5, its border S6.
fn sign() -> ChiploadApp {
    let mut app = ChiploadApp::default();
    assert!(app.open(&corpus("jobs/v35/sign_frame.chipload")));
    app
}

/// The contour shape `id` is in.
fn contour_of(app: &ChiploadApp, id: u64) -> usize {
    app.contours
        .iter()
        .find(|c| shapes_of(c).contains(&id))
        .unwrap()
        .id
}

fn drawing(app: &ChiploadApp) -> &chipload_job::Drawing {
    &app.edit.as_ref().unwrap().job.drawing
}

/// Frame, border and text grouped with Ctrl+G.
fn grouped() -> ChiploadApp {
    let mut app = sign();
    app.selection = CanvasSelection::Shapes([1, 5, 6].into());
    assert!(app.can_group());
    assert!(app.group_selected());
    app.commit();
    app
}

#[test]
fn group_makes_one_and_a_click_picks_it_whole() {
    let mut app = grouped();
    let group = &drawing(&app).groups[0];
    assert_eq!((&group.shapes, &group.texts), (&vec![5, 6], &vec![1]));
    // A click on the border, or on a letter, picks the whole group.
    for shape in [6, 3] {
        app.selection = CanvasSelection::Nothing;
        app.click(Some(contour_of(&app, shape)), false);
        assert_eq!(app.selected_group(), Some(1), "S{shape}");
        assert_eq!(app.selected_ids().len(), app.contours.len());
    }
    // Already one group: nothing to group.
    assert!(!app.can_group());
}

#[test]
fn ctrl_click_leaves_a_shape_out_of_the_selection_not_the_group() {
    let mut app = grouped();
    app.click(Some(contour_of(&app, 5)), false);
    app.click(Some(contour_of(&app, 6)), true);
    let CanvasSelection::Whole { left_out, .. } = &app.selection else {
        panic!("{:?}", app.selection);
    };
    assert_eq!(left_out, &BTreeSet::from([6]));
    let select = app.selection.selector(&app.contours, false);
    assert_eq!((select.group(), select.exclude), (Some(1), vec![6]));
    assert_eq!(drawing(&app).groups[0].shapes, [5, 6], "still in the group");
    // And back in.
    app.click(Some(contour_of(&app, 6)), true);
    assert!(
        matches!(&app.selection, CanvasSelection::Whole { left_out, .. } if left_out.is_empty())
    );
}

#[test]
fn ungroup_and_remove_from_group() {
    let mut app = grouped();
    app.click(Some(contour_of(&app, 5)), false);
    assert!(app.ungroup_selected());
    assert!(drawing(&app).groups.is_empty());
    assert!(matches!(app.selection, CanvasSelection::Shapes(_)));
    // Undo brings the group back.
    app.commit();
    app.undo();
    assert_eq!(drawing(&app).groups.len(), 1);
    // One shape picked alone inside the group, taken out of it.
    app.selection = CanvasSelection::Shapes([6].into());
    assert!(app.remove_selected_from_group());
    assert_eq!(drawing(&app).groups[0].shapes, [5]);
}

#[test]
fn a_group_moves_whole_with_its_text_and_del_deletes_it() {
    let mut app = grouped();
    app.click(Some(contour_of(&app, 5)), false);
    let before = drawing(&app).texts[0].clone();
    assert!(app.transform_group(1, (0.0, 0.0), (10.0, 5.0), 1.0, 0.0));
    let after = &drawing(&app).texts[0];
    assert_eq!((after.x, after.y), (before.x + 10.0, before.y + 5.0));
    assert_eq!(app.selected_group(), Some(1), "still selected");
    app.commit();
    app.delete_key();
    assert!(drawing(&app).shapes.is_empty() && drawing(&app).texts.is_empty());
    assert!(drawing(&app).groups.is_empty());
}

#[test]
fn an_operation_made_from_a_group_names_it() {
    let mut app = grouped();
    app.click(Some(contour_of(&app, 5)), false);
    let select = app.selection.selector(&app.contours, false);
    assert_eq!(select.group(), Some(1));
    assert_eq!(select.shapes, None);
}

#[test]
fn copying_a_group_pastes_a_new_group() {
    let mut app = grouped();
    // A group of drawn shapes only: take the text out first.
    app.selection = CanvasSelection::whole(chipload_core::chain::ShapeFrom::Text(1));
    assert!(app.remove_selected_from_group());
    app.click(Some(contour_of(&app, 5)), false);
    assert!(app.copy_selected_shapes().is_some());
    assert!(app.paste(None));
    assert_eq!(drawing(&app).groups.len(), 2);
    assert_eq!(app.selected_group(), Some(2));
}

#[test]
fn dragging_a_shape_of_the_selected_group_moves_the_group() {
    let mut app = grouped();
    app.click(Some(contour_of(&app, 5)), false);
    let text_x = drawing(&app).texts[0].x;
    let picked = Some(contour_of(&app, 5));
    // Free (Alt): no snapping, so it lands exactly.
    app.start_select_drag(picked, (100.0, 50.0), false, true);
    app.drag_move_to((120.0, 50.0), true, false);
    app.end_move_drag();
    assert!((drawing(&app).texts[0].x - (text_x + 20.0)).abs() < 1e-9);
    assert_eq!(app.selected_group(), Some(1));
}

#[test]
fn a_double_click_picks_one_shape_inside() {
    use egui_kittest::Harness;
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1200.0, 800.0))
        // Frames close enough together for two clicks to be a double-click.
        .with_step_dt(0.05)
        .build_eframe(|_| {
            let mut app = grouped();
            app.selection = CanvasSelection::Nothing;
            app
        });
    harness.run_steps(4);
    // The frame's left side (S5: 132.3 wide round (150, 80)).
    let at = harness
        .state()
        .view
        .to_screen((150.0 - 132.314 / 2.0, 80.0));
    harness.hover_at(at);
    harness.run_steps(1);
    for _ in 0..2 {
        for pressed in [true, false] {
            harness.event(egui::Event::PointerButton {
                pos: at,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            });
            harness.run_steps(1);
        }
    }
    harness.run_steps(2);
    assert_eq!(
        harness.state().selection,
        CanvasSelection::Shapes([5].into()),
        "the frame alone"
    );
}

#[test]
fn a_group_is_aligned_whole() {
    // docs/specs/arrange-whole.md: the group picked whole aligns to the stock as one, its
    // text with it; picked with another shape, the two line up, the group whole.
    use super::modify::Arrange;
    use chipload_pipeline::drawing_edit::AlignTo;
    let mut app = grouped();
    let shapes = |app: &ChiploadApp, ids: &[u64]| {
        chipload_pipeline::shapes::shapes_bounds(&app.edit.as_ref().unwrap().job, ids).unwrap()
    };
    let (frame, letters) = (shapes(&app, &[5]), shapes(&app, &[1, 2, 3, 4]));
    app.click(Some(contour_of(&app, 5)), false);
    assert_eq!(app.selected_group(), Some(1));
    app.canvas_tools.align_to_stock = true;
    assert!(app.arrange_selected(Arrange::Align(AlignTo::Left)));
    app.commit();
    let (frame2, letters2) = (shapes(&app, &[5]), shapes(&app, &[1, 2, 3, 4]));
    let moved = frame2.0.0 - frame.0.0;
    assert!(
        (letters2.0.0 - letters.0.0 - moved).abs() < 1e-6,
        "the text went along"
    );
    assert!((frame2.0.1 - frame.0.1).abs() < 1e-9);
    // Two of its own shapes are no longer two things to line up: the group is one.
    app.canvas_tools.align_to_stock = false;
    assert!(!app.arrange_selected(Arrange::Align(AlignTo::Right)));
}
