// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

#[test]
fn the_drawing_panel_groups_dxfs_images_and_drawn_shapes() {
    let mut app = ChiploadApp::default();
    assert!(app.open(&corpus("jobs/v21/two_dxfs.chipload")));
    let groups = app.drawing_groups();
    let keys: Vec<&str> = groups.iter().map(|g| g.key.as_str()).collect();
    assert_eq!(keys, ["dxf:1", "dxf:2", "image:1"]);
    assert_eq!(groups[1].title, "part B");
    assert_eq!(
        groups[1].layers,
        [("CUT_OUTSIDE".to_string(), 1), ("HOLES".to_string(), 6)]
    );
    // A layer selected under a DXF makes an operation of that DXF's layer only.
    app.selection = CanvasSelection::Layer(
        "HOLES".into(),
        Some(chipload_core::chain::ShapeFrom::Dxf(2)),
    );
    let index = app.add_operation(OpKind::Profile).unwrap();
    let select = app.edit.as_ref().unwrap().ops[index].op.select().clone();
    assert_eq!(
        (select.layer.as_deref(), select.dxf),
        (Some("HOLES"), Some(2))
    );
    let both = app.op_contours(&app.edit.as_ref().unwrap().ops[0].op).len();
    let one = app
        .op_contours(&app.edit.as_ref().unwrap().ops[index].op)
        .len();
    assert!(one * 2 == both, "{one} of {both}");
    // A whole group is selected by its shapes.
    app.select_group("image:1");
    assert_eq!(app.selection.ids(&app.contours).len(), 4);
}

#[test]
fn hidden_groups_are_not_drawn_nor_clickable_but_still_cut() {
    let mut app = ChiploadApp::default();
    assert!(app.open(&corpus("jobs/v21/two_dxfs.chipload")));
    let all = app.scene.contours.len();
    app.toggle_hidden("dxf:2");
    let shown = app.scene.contours.len();
    assert_eq!(all - shown, app.hidden_contours.len());
    assert!(!app.hidden_contours.is_empty());
    assert!(
        app.scene
            .contours
            .iter()
            .all(|c| !app.hidden_contours.contains(&c.id))
    );
    // Operation 1 cuts the holes of both DXFs: it still does, and says so.
    let op = app.edit.as_ref().unwrap().ops[0].op.clone();
    assert!(app.hidden_cut(&op) > 0);
    assert!(app.generate_and_wait(), "{:?}", app.report);
    // A layer alone, then shown again; undo brings back what was hidden.
    app.toggle_hidden("dxf:2");
    assert_eq!(app.scene.contours.len(), all);
    app.toggle_hidden("dxf:1/layer:HOLES");
    assert!(app.scene.contours.len() < all);
    app.undo();
    assert_eq!(app.scene.contours.len(), all);
    // Removing a DXF removes its shapes, as one undo step.
    assert!(app.remove_dxf(2));
    assert_eq!(app.drawing_groups().len(), 2);
    app.undo();
    assert_eq!(app.drawing_groups().len(), 3);
}
