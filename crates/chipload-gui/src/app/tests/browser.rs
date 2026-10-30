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

/// The tool change line follows the program: a floor pass is cut before its V-carve, so the
/// line above the V-carve names both tools in cut order (found 2026-10-30: it said only the
/// V-bit's, as if the floor pass's end mill came after).
#[test]
fn tool_change_lines_follow_the_cut_order() {
    use crate::app::browser::tool_changes;
    let job = chipload_job::load_job(&corpus("jobs/v33/vcarve_floor.chipload")).unwrap();
    // The floor pass with tool 2, then the V-carve with tool 1.
    assert_eq!(tool_changes(&job).get(&0), Some(&vec![2, 1]));
    // Without its floor pass, one tool and no line.
    let mut plain = job.clone();
    if let chipload_job::Operation::Vcarve(v) = &mut plain.operations[0] {
        v.floor = None;
    }
    assert!(tool_changes(&plain).is_empty());
    // Two parts with a tool each, in list order: one change, above the second.
    let two = chipload_job::load_job(&corpus("jobs/v22/two_parts.chipload")).unwrap();
    let changes = tool_changes(&two);
    assert!(changes.values().all(|c| c.len() == 1), "{changes:?}");
}
