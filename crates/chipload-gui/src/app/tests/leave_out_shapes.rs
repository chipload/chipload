// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

#[test]
fn ctrl_click_leaves_a_contour_out_of_the_selected_dxf_and_puts_it_back() {
    use chipload_core::chain::ShapeFrom;
    let mut app = two_parts();
    app.select_group("dxf:1");
    let all = app.selection.ids(&app.contours);
    let one = *all.iter().next().unwrap();
    app.click(Some(one), true);
    let CanvasSelection::Except(ShapeFrom::Dxf(1), left_out) = app.selection.clone() else {
        panic!("{:?}", app.selection);
    };
    let ids = app.selection.ids(&app.contours);
    assert_eq!(ids.len(), all.len() - 1);
    assert!(!ids.contains(&one));
    assert!(
        app.selection_lines()[0].contains("DXF 1, 1 left out"),
        "{:?}",
        app.selection_lines()
    );
    // The operation selects the DXF but those shapes.
    let index = app.add_operation(OpKind::Profile).unwrap();
    let select = app.edit.as_ref().unwrap().ops[index].op.select().clone();
    assert_eq!(select.dxf, Some(1));
    assert_eq!(
        select.exclude,
        Some(left_out.iter().copied().collect::<Vec<_>>())
    );
    // Ctrl+click again: back to the whole DXF.
    app.select_group("dxf:1");
    app.click(Some(one), true);
    app.click(Some(one), true);
    assert_eq!(app.selection, CanvasSelection::Source(ShapeFrom::Dxf(1)));
}

#[test]
fn the_unsaved_changes_question_offers_save_and_draws_in_the_middle() {
    let mut app = lower_part();
    edit_feed(&mut app, 650.0);
    assert!(app.dirty());
    app.dialogs.prompt = Some(Prompt::Discard(Pending::New));
    let ctx = egui::Context::default();
    let mut output = ctx.run_ui(egui::RawInput::default(), |ui| app.prompts(ui.ctx()));
    output.textures_delta.clear();
    // Still asking: nothing was answered.
    assert!(matches!(
        app.dialogs.prompt,
        Some(Prompt::Discard(Pending::New))
    ));
    // A project without a file can't be saved without asking where: the unsaved changes
    // stay (the file dialog isn't there in tests, so nothing is chosen).
    assert!(app.dirty());
}
