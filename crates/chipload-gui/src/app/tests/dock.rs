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

// -- the layout kept in layout.json (docs/specs/dockable-panels.md §5)

/// A window on `dir`'s settings, drawn a few frames.
fn window(dir: &std::path::Path) -> egui_kittest::Harness<'static, ChiploadApp> {
    let dir = dir.to_path_buf();
    let mut harness = egui_kittest::Harness::builder()
        .with_size(egui::vec2(1200.0, 800.0))
        .build_eframe(move |_| {
            let mut app = ChiploadApp::default();
            app.set_settings_dir(Some(dir.clone()));
            app
        });
    harness.run_steps(3);
    harness
}

fn fresh(name: &str) -> std::path::PathBuf {
    let dir = temp_dir(name);
    let _ = std::fs::remove_file(dir.join(chipload_job::layout::LAYOUT_FILE));
    dir
}

#[test]
fn a_layout_is_described_and_built_back_the_same() {
    let mut panels = Panels::default();
    panels.close(Pane::Browser);
    panels.show(Pane::Shapes);
    panels.show(Pane::Report);
    let layout = panels.layout();
    assert!(layout.problems().is_empty(), "{:?}", layout.problems());
    let again = Panels::from_layout(&layout);
    assert_eq!(again.layout(), layout);
    assert!(!again.is_open(Pane::Browser));
    assert!(again.is_shown(Pane::Shapes) && again.is_shown(Pane::Report));
    // The default layout, described, is the format's default.
    assert_eq!(
        Panels::default().layout().root,
        chipload_job::layout::WindowLayout::default().root
    );
}

#[test]
fn the_layout_is_kept_for_the_next_start() {
    let dir = fresh("layout-kept");
    let path = dir.join(chipload_job::layout::LAYOUT_FILE);
    let mut first = window(&dir);
    assert!(!path.exists(), "nothing written while nothing changed");
    first.state_mut().panels.close(Pane::Browser);
    first.run_steps(2);
    assert!(path.exists());
    let kept = first.state().panels.layout();
    drop(first);
    let second = window(&dir);
    assert_eq!(second.state().panels.layout(), kept);
    assert!(!second.state().panels.is_open(Pane::Browser));
    // Hidden with P: the layout to come back is what is kept.
    let mut third = window(&dir);
    third.state_mut().panels.toggle_hidden();
    third.run_steps(2);
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains("properties"), "{text}");
}

#[test]
fn a_layout_that_cant_be_read_gives_the_default_and_stays_until_changed() {
    let dir = fresh("layout-newer");
    let path = dir.join(chipload_job::layout::LAYOUT_FILE);
    let newer = r#"{"schema_version": 99, "root": "canvas"}"#;
    std::fs::write(&path, newer).unwrap();
    let mut harness = window(&dir);
    assert!(
        harness.state().panels.is_shown(Pane::Browser),
        "the default layout"
    );
    let status = harness
        .state()
        .status
        .as_ref()
        .map(|(text, _)| text.clone())
        .unwrap_or_default();
    assert!(status.contains("couldn't be read"), "{status}");
    harness.run_steps(3);
    assert_eq!(std::fs::read_to_string(&path).unwrap(), newer, "kept");
    harness.state_mut().panels.close(Pane::Report);
    harness.run_steps(2);
    assert_ne!(
        std::fs::read_to_string(&path).unwrap(),
        newer,
        "changed: written"
    );
}

#[test]
fn the_panels_have_square_corners_in_both_themes() {
    // Rounded corners showed the dark behind the dock in the light theme (issue #14).
    let ctx = egui::Context::default();
    crate::style::apply(&ctx);
    for theme in [egui::Theme::Dark, egui::Theme::Light] {
        let style = crate::app::layout::dock_style(&ctx.style_of(theme));
        assert_eq!(style.tab_bar.corner_radius, egui::CornerRadius::ZERO);
        assert_eq!(style.tab.tab_body.corner_radius, egui::CornerRadius::ZERO);
    }
}
