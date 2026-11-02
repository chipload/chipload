// SPDX-License-Identifier: GPL-3.0-or-later
//! The panels docked round the canvas (docs/specs/dockable-panels.md).

use super::*;
use crate::app::layout::{Pane, Panels};

#[test]
fn the_default_layout_is_todays() {
    let panels = Panels::default();
    for pane in Pane::PANELS {
        assert!(panels.is_open(pane), "{pane:?}");
    }
    assert!(panels.together(Pane::Properties, Pane::Shapes));
    assert!(panels.is_shown(Pane::Properties) && !panels.is_shown(Pane::Shapes));
    assert!(panels.is_shown(Pane::Browser));
    assert!(
        !panels.is_shown(Pane::Report),
        "folded until it has something to say"
    );
    assert!(!panels.together(Pane::Canvas, Pane::Browser));
}

#[test]
fn a_closed_panel_opens_again_where_it_goes() {
    let mut panels = Panels::default();
    panels.close(Pane::Shapes);
    assert!(!panels.is_open(Pane::Shapes));
    panels.show(Pane::Shapes);
    assert!(
        panels.together(Pane::Properties, Pane::Shapes),
        "back with Properties"
    );
    assert!(panels.is_shown(Pane::Shapes));
    // Both closed: Properties comes back on its own, then Shapes joins it.
    panels.close(Pane::Shapes);
    panels.close(Pane::Properties);
    panels.show(Pane::Properties);
    panels.show(Pane::Shapes);
    assert!(panels.together(Pane::Properties, Pane::Shapes));
    // The canvas can't be closed.
    panels.close(Pane::Canvas);
    assert!(panels.is_shown(Pane::Canvas));
}

#[test]
fn a_panel_dropped_on_the_canvas_docks_beside_it() {
    let mut panels = Panels::default();
    panels.drop_on_canvas(Pane::Browser);
    assert!(panels.is_open(Pane::Browser));
    assert!(!panels.together(Pane::Canvas, Pane::Browser));
    assert!(panels.is_shown(Pane::Canvas) && panels.is_shown(Pane::Browser));
}

#[test]
fn reset_layout_brings_the_default_back() {
    let mut app = ChiploadApp::default();
    app.panels.close(Pane::Browser);
    app.panels.show(Pane::Shapes);
    app.panels.toggle_hidden();
    app.panels.reset();
    assert!(!app.panels.hidden());
    assert!(app.panels.is_shown(Pane::Browser) && app.panels.is_shown(Pane::Properties));
}

#[test]
fn the_dock_is_drawn_with_every_panel_and_the_canvas() {
    let mut harness = egui_kittest::Harness::builder()
        .with_size(egui::vec2(1200.0, 800.0))
        .build_eframe(|_| {
            let mut app = ChiploadApp::default();
            assert!(app.open(&corpus("jobs/v7/leads.chipload")));
            app
        });
    harness.run_steps(3);
    use egui_kittest::kittest::Queryable;
    for title in ["Browser", "Properties", "Shapes", "Report"] {
        harness.get_by_label(title);
    }
}
