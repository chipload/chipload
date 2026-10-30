// SPDX-License-Identifier: GPL-3.0-or-later
//! Texts in the app (docs/specs/text.md §5, §8).
use chipload_core::chain::ShapeFrom;

use super::*;

/// A new project with a 60° V-bit as tool 2, and a text placed at (20, 30).
fn with_text() -> (ChiploadApp, u64) {
    let mut app = ChiploadApp::default();
    app.new_project();
    app.edit
        .as_mut()
        .unwrap()
        .settings
        .tools
        .push(chipload_job::Tool {
            id: 2,
            name: None,
            library_uid: None,
            kind: ToolType::Vbit,
            diameter: 12.7,
            flutes: Some(2),
            center_cutting: Some(true),
            rpm: Some(18000.0),
            angle: Some(60.0_f64.to_radians()),
            tip_diameter: Some(0.2),
        });
    app.commit();
    app.start_text();
    assert!(app.placing_text());
    let id = app.place_text_at((20.0, 30.0)).unwrap();
    (app, id)
}

fn texts(app: &ChiploadApp) -> &[chipload_job::Text] {
    &app.edit.as_ref().unwrap().settings.drawing.texts
}

#[test]
fn draw_text_places_a_text_opened_and_selected_as_one_undo_step() {
    let (mut app, id) = with_text();
    assert!(!app.placing_text());
    assert_eq!(texts(&app).len(), 1);
    assert_eq!((texts(&app)[0].x, texts(&app)[0].y), (20.0, 30.0));
    assert_eq!(app.focus, Focus::Text(id));
    assert_eq!(app.selection, CanvasSelection::Source(ShapeFrom::Text(id)));
    assert!(app.focus_text_field, "the text field takes the keyboard");
    // Its letters are contours on the canvas.
    let letters = app
        .contours
        .iter()
        .filter(|c| {
            c.parts
                .iter()
                .all(|p| p.segment.from == Some(ShapeFrom::Text(id)))
        })
        .count();
    assert!(letters >= 4, "T, e, x, t: {letters}");
    // The form and the Browser draw (and the field takes focus once).
    draw_properties(&mut app);
    assert!(!app.focus_text_field);
    draw_sidebar(&mut app);
    app.undo();
    assert!(texts(&app).is_empty());
    assert!(
        app.contours
            .iter()
            .all(|c| c.parts.iter().all(|p| p.segment.from.is_none()))
    );
}

#[test]
fn changing_a_text_writes_its_letters_again_as_one_undo_step() {
    let (mut app, id) = with_text();
    let before = app.contours.len();
    let mut text = texts(&app)[0].clone();
    text.text = "CHIPLOAD".into();
    text.height = 20.0;
    text.rotation = 30.0;
    assert!(app.set_text(id, &text));
    assert_eq!(texts(&app)[0].text, "CHIPLOAD");
    assert!(app.contours.len() > before, "more letters");
    app.undo();
    assert_eq!(texts(&app)[0].text, "Text");
    assert_eq!(app.contours.len(), before);
}

#[test]
fn clicking_a_letter_selects_the_text_and_a_v_carve_cuts_it() {
    let (mut app, id) = with_text();
    app.selection = CanvasSelection::Nothing;
    let letter = app
        .contours
        .iter()
        .find(|c| {
            c.parts
                .iter()
                .all(|p| p.segment.from == Some(ShapeFrom::Text(id)))
        })
        .unwrap()
        .id;
    app.click(Some(letter), false);
    assert_eq!(app.selection, CanvasSelection::Source(ShapeFrom::Text(id)));
    // Its properties open, as from its Browser row (the user, 2026-10-30).
    assert_eq!(app.focus, Focus::Text(id));
    draw_properties(&mut app);
    // Ctrl+click picks letters, not the text: the selection shows.
    app.click(Some(letter), true);
    assert_eq!(app.focus, Focus::Selection);
    app.click(Some(letter), false);
    let index = app.add_operation(OpKind::Vcarve).unwrap();
    let op = &app.edit.as_ref().unwrap().ops[index].op;
    assert_eq!(op.select().text, Some(id));
    // Its form names the text, not "no geometry" (user report 2026-10-17).
    assert_eq!(
        crate::editor::geometry_summary(op, &app.contours, &crate::i18n::Language::En),
        format!("all of text {id}")
    );
    assert_eq!(op.tool(), 2, "the V-bit");
    assert!(app.generate_and_wait(), "{:?}", app.report);
    // Removing the text leaves the operation with nothing: a problem, not a silent cut.
    assert!(app.remove_text(id));
    assert!(texts(&app).is_empty());
    assert!(!app.problems.is_empty());
}

#[test]
fn dragging_a_texts_move_handle_moves_its_letters_as_one_undo_step() {
    use chipload_pipeline::box_handles::BoxHandle;
    let (mut app, id) = with_text();
    let handles = app.box_handle_points();
    assert_eq!(handles.len(), 6, "move, turn and four corners");
    let (_, center) = handles[0];
    let before = texts(&app)[0].clone();
    app.start_box_drag(BoxHandle::Move);
    app.drag_box_to((center.0 + 15.0, center.1 - 5.0), false);
    assert_eq!(
        texts(&app)[0],
        before,
        "nothing changes until the drag ends"
    );
    app.end_box_drag();
    let after = &texts(&app)[0];
    assert!((after.x - (before.x + 15.0)).abs() < 1e-9);
    assert!((after.y - (before.y - 5.0)).abs() < 1e-9);
    assert_eq!(app.selection, CanvasSelection::Source(ShapeFrom::Text(id)));
    app.undo();
    assert_eq!(texts(&app)[0], before);
}

#[test]
fn a_corner_drag_makes_a_text_taller() {
    use chipload_pipeline::box_handles::BoxHandle;
    let (mut app, _) = with_text();
    let handles = app.box_handle_points();
    // Upper-right corner (Size(2)) out by the box's own diagonal: twice the size.
    let (_, lower_left) = handles[2];
    let (_, upper_right) = handles[4];
    let to = (
        2.0 * upper_right.0 - lower_left.0,
        2.0 * upper_right.1 - lower_left.1,
    );
    app.start_box_drag(BoxHandle::Size(2));
    app.drag_box_to(to, false);
    app.end_box_drag();
    let text = &texts(&app)[0];
    assert!((text.height - 20.0).abs() < 1e-9, "{}", text.height);
    // The lower-left corner stays (the text's anchor is its corner).
    assert!((text.x - 20.0).abs() < 0.01 && (text.y - 30.0).abs() < 0.01);
}
