// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

#[test]
fn a_browser_click_selects_and_ctrl_click_adds() {
    use chipload_core::chain::ShapeFrom;
    let mut app = ChiploadApp::default();
    assert!(app.open(&corpus("jobs/v21/two_dxfs.chipload")));
    // A click on a DXF's row selects the DXF as a whole, and its row shows selected.
    let dxf = app.group_selection("dxf:1");
    assert_eq!(dxf, CanvasSelection::Source(ShapeFrom::Dxf(1)));
    app.select_row(dxf.clone(), false);
    assert_eq!(app.selection, dxf);
    let first = app.selection.ids(&app.contours);
    // Ctrl+click on the picture's row adds its contours.
    app.select_row(app.group_selection("image:1"), true);
    let both = app.selection.ids(&app.contours);
    assert_eq!(both.len(), first.len() + 4);
    assert!(first.is_subset(&both));
    // A plain click replaces it again.
    app.select_row(
        CanvasSelection::Layer("HOLES".into(), Some(ShapeFrom::Dxf(2))),
        false,
    );
    assert_eq!(app.selection.ids(&app.contours).len(), 3); // 6 shapes, 3 holes
    // The Browser draws with every folder open, with a selection and with an operation off.
    app.edit.as_mut().unwrap().ops[1].op.set_enabled(false);
    app.focus = Focus::Op(2);
    draw_sidebar(&mut app);
}
