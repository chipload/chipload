// SPDX-License-Identifier: GPL-3.0-or-later
//! Tabs placed by hand on the canvas (docs/specs/tabs-by-hand.md §5).
use super::*;

/// A drawn rectangle cut outside (operation 2) with 4 tabs, generated, the profile focused.
fn tabbed() -> ChiploadApp {
    let mut app = ChiploadApp::default();
    assert!(app.open(&corpus("jobs/v26/straight_tabs.chipload")));
    assert!(app.generate_and_wait(), "{:?}", app.report);
    app.focus = Focus::Op(1);
    app
}

fn markers(app: &ChiploadApp) -> Vec<crate::view::TabMarker> {
    app.scene
        .tab_markers
        .iter()
        .filter(|m| m.tab.operation == 1)
        .copied()
        .collect()
}

fn points(app: &ChiploadApp) -> Vec<[f64; 2]> {
    match &app.edit.as_ref().unwrap().ops[1].op {
        Operation::Profile(p) => p.tabs.as_ref().unwrap().at.clone(),
        _ => panic!("operation 2 is the profile"),
    }
}

/// The marker index in `scene.tab_markers` of the operation's `k`-th tab.
fn index_of(app: &ChiploadApp, k: usize) -> usize {
    app.scene
        .tab_markers
        .iter()
        .position(|m| m.tab == markers(app)[k].tab)
        .unwrap()
}

#[test]
fn dragging_a_tab_moves_it_alone_and_undo_puts_it_back() {
    let mut app = tabbed();
    let before = markers(&app);
    assert_eq!(before.len(), 4);
    assert!(before.iter().all(|m| !m.tab.by_hand));
    let pos = app.view.to_screen(before[0].at);
    let k = app
        .tab_marker_at(pos)
        .expect("the first tab is under its marker");
    // Dragged 10 mm along: the marker follows the ring.
    let ring = app
        .scene
        .start_markers
        .iter()
        .find(|m| m.operation == 1)
        .unwrap()
        .ring
        .clone();
    let to = (before[0].at.0 + 10.0, before[0].at.1 + 10.0);
    app.drag_tab_to(k, to);
    let at = app.scene.tab_markers[k].at;
    let (_, _, d) = chipload_toolpath::start::nearest_on_ring(&ring, at).unwrap();
    assert!(d < 1e-9);
    app.drop_tab();
    assert!(app.wait_for_generation(), "{:?}", app.report);
    let after = markers(&app);
    assert_eq!(after.len(), 4);
    assert!(after.iter().all(|m| m.tab.by_hand));
    assert!(
        after
            .iter()
            .any(|m| chipload_core::primitives::distance(m.at, at) < 1e-6)
    );
    for m in &before[1..] {
        assert!(
            after
                .iter()
                .any(|n| chipload_core::primitives::distance(n.at, m.at) < 1e-6)
        );
    }
    app.undo();
    assert!(points(&app).is_empty());
}

#[test]
fn removing_spacing_evenly_and_adding_from_the_canvas() {
    let mut app = tabbed();
    let k = index_of(&app, 2);
    app.remove_tab(k);
    assert!(app.wait_for_generation(), "{:?}", app.report);
    assert_eq!(markers(&app).len(), 3);
    assert_eq!(points(&app).len(), 3);

    app.even_tabs(index_of(&app, 0));
    assert!(app.wait_for_generation(), "{:?}", app.report);
    assert!(points(&app).is_empty());
    assert_eq!(markers(&app).len(), 4);

    // Right-clicking the ring halfway between two tabs adds one there.
    let (a, b) = (markers(&app)[0].at, markers(&app)[1].at);
    let ring = app
        .scene
        .start_markers
        .iter()
        .find(|m| m.operation == 1)
        .unwrap()
        .ring
        .clone();
    let (_, mid, _) =
        chipload_toolpath::start::nearest_on_ring(&ring, ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0))
            .unwrap();
    let (r, at) = app
        .tab_ring_at(app.view.to_screen(mid))
        .expect("on the ring");
    app.add_tab(r, at);
    assert!(app.wait_for_generation(), "{:?}", app.report);
    assert_eq!(markers(&app).len(), 5);
}

#[test]
fn tab_markers_are_only_for_the_focused_profile_in_top_view() {
    let mut app = tabbed();
    let pos = app.view.to_screen(markers(&app)[0].at);
    assert!(app.tab_marker_at(pos).is_some());
    app.focus = Focus::Op(0);
    assert_eq!(app.tab_marker_at(pos), None);
    app.focus = Focus::Op(1);
    app.view_3d();
    assert_eq!(app.tab_marker_at(pos), None);
    app.view_top();
    app.toolpath_out_of_date();
    assert!(app.scene.tab_markers.is_empty());
}

/// A real mouse drag of a tab: the profile stays focused afterwards, so the next tab can
/// be dragged straight away (the user, 2026-10-29).
#[test]
fn the_profile_stays_selected_after_a_tab_is_dragged() {
    use egui_kittest::Harness;
    let mut h = Harness::builder()
        .with_size(egui::vec2(1200.0, 800.0))
        .build_eframe(|_| tabbed());
    h.run_steps(6);
    let at = markers(h.state())[0].at;
    let from = h.state().view.to_screen(at);
    let to = h.state().view.to_screen((at.0 + 10.0, at.1 + 10.0));
    h.hover_at(from);
    h.run_steps(2);
    h.drag_at(from);
    h.run_steps(2);
    for k in 1..=5 {
        h.hover_at(from + (to - from) * (k as f32 / 5.0));
        h.run_steps(2);
    }
    h.drop_at(to);
    h.run_steps(3);
    h.state_mut().wait_for_generation();
    h.run_steps(3);
    assert!(!points(h.state()).is_empty(), "the tab was moved");
    assert_eq!(h.state().focus, Focus::Op(1));
}
