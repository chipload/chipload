// SPDX-License-Identifier: GPL-3.0-or-later
//! The window's layout, run headless with egui_kittest: the app as the window shows it,
//! measured through AccessKit, in each language. `pictures` renders it to PNG files to look
//! at (with wgpu, no display needed):
//!
//! ```text
//! CHIPLOAD_PICTURES=/some/dir cargo test -p chipload-gui pictures -- --ignored
//! ```

use egui_kittest::kittest::Queryable;
use egui_kittest::{Harness, HarnessBuilder};

use super::*;

/// The default window, as main.rs opens it.
const WINDOW: egui::Vec2 = egui::vec2(1200.0, 800.0);

/// The six languages, by their setting.
const LANGUAGES: [&str; 6] = ["en", "de", "fr", "es", "it", "sv"];

/// The ribbon's last large button (View > Measure) and Generate, as named in `language`.
fn ribbon_ends(language: &str) -> (String, String) {
    let lang = crate::i18n::resolve(language);
    (t!(lang, "header-measure"), t!(lang, "header-generate"))
}

/// The app with a project open, in `language`, set up as main.rs sets it up (the system's
/// fonts, Chipload's style), a few frames on so that what depends on the last frame has
/// settled.
fn app_harness(
    builder: HarnessBuilder<ChiploadApp>,
    language: &str,
) -> Harness<'static, ChiploadApp> {
    let language = language.to_string();
    let mut harness = builder.build_eframe(move |cc| {
        if let Ok(mut fonts) = crate::fonts::system_fonts() {
            crate::fonts::add_text_fonts(&mut fonts);
            cc.egui_ctx.set_fonts(fonts);
        }
        crate::style::apply(&cc.egui_ctx);
        let mut app = ChiploadApp::default();
        app.set_language(&language);
        assert!(app.open(&corpus("jobs/v5/drawn_part.chipload")));
        app
    });
    harness.run_steps(6);
    harness
}

fn harness(language: &str, size: egui::Vec2) -> Harness<'static, ChiploadApp> {
    app_harness(Harness::builder().with_size(size), language)
}

/// A widget's rectangle by its label.
fn rect_of(harness: &Harness<'_, ChiploadApp>, label: &str) -> egui::Rect {
    harness.get_by_label(label).rect()
}

/// Whether the ribbon's large buttons show their names (not their icons alone).
fn names_shown(harness: &Harness<'_, ChiploadApp>, button: &str) -> bool {
    rect_of(harness, button).width() > super::super::header::RIBBON_COMPACT_WIDTH + 0.5
}

#[test]
fn the_ribbon_fits_the_default_window_in_every_language() {
    for language in LANGUAGES {
        let (last, generate) = ribbon_ends(language);
        let harness = harness(language, WINDOW);
        let end = rect_of(&harness, &last).right();
        let action = rect_of(&harness, &generate).left();
        assert!(
            end <= action,
            "{language}: the ribbon's last button ends at {end} but {generate} starts at {action}"
        );
    }
}

#[test]
fn the_ribbon_shows_names_where_they_fit_and_icons_alone_where_not() {
    // English fits with names; German needs icons alone at the default size.
    assert!(names_shown(&harness("en", WINDOW), "Measure"));
    let mut german = harness("de", WINDOW);
    assert!(!names_shown(&german, "Messen"));
    // A wider window shows the names again.
    german.set_size(egui::vec2(1600.0, 800.0));
    german.run_steps(6);
    assert!(names_shown(&german, "Messen"));
    // A narrow window: icons alone, in English too.
    assert!(!names_shown(
        &harness("en", egui::vec2(900.0, 700.0)),
        "Measure"
    ));
}

/// The window as pictures, to look at: each language at the default size, English in a
/// narrow window and with two shapes selected, written to CHIPLOAD_PICTURES (a folder).
#[test]
#[ignore = "writes pictures to look at; run with CHIPLOAD_PICTURES=DIR -- --ignored"]
fn pictures() {
    let Ok(dir) = std::env::var("CHIPLOAD_PICTURES") else {
        panic!("set CHIPLOAD_PICTURES to the folder to write to");
    };
    let dir = std::path::PathBuf::from(dir);
    std::fs::create_dir_all(&dir).unwrap();
    let mut shots: Vec<(String, &str, egui::Vec2)> = LANGUAGES
        .iter()
        .map(|language| (format!("window-{language}"), *language, WINDOW))
        .collect();
    shots.push(("window-en-narrow".into(), "en", egui::vec2(900.0, 700.0)));
    for (name, language, size) in shots {
        let mut harness = app_harness(Harness::builder().with_size(size).wgpu(), language);
        let image = harness.render().unwrap();
        image.save(dir.join(format!("{name}.png"))).unwrap();
    }
    // Two drawn shapes selected: the Selection section with its tools.
    let mut harness = app_harness(Harness::builder().with_size(WINDOW).wgpu(), "en");
    harness.state_mut().selection = CanvasSelection::Shapes([2, 3].into());
    harness.run_steps(4);
    let image = harness.render().unwrap();
    image.save(dir.join("window-en-selection.png")).unwrap();
}

/// One thing at a time (the user, 2026-10-28): a drawn shape's row in the Browser shows the
/// shape, not the form that was open.
#[test]
fn a_shape_row_in_the_browser_shows_the_shape_alone() {
    let mut harness = harness("en", WINDOW);
    harness.state_mut().focus = Focus::Job;
    harness.run_steps(2);
    harness.get_by_label("S2").click();
    harness.run_steps(3);
    assert_eq!(harness.state().focus, Focus::Selection);
    assert_eq!(harness.state().selected_shape(), Some(2));
}

/// Clicking an operation's number on the toolpath opens that operation (the user,
/// 2026-10-28), even with the drawing under it.
#[test]
fn clicking_an_operations_number_on_the_toolpath_opens_it() {
    let mut harness = harness("en", WINDOW);
    assert!(harness.state_mut().generate_and_wait());
    harness.state_mut().focus = Focus::Job;
    // Folded: in this narrow canvas the open legend stands on the navigation bar, over
    // the number.
    harness.state_mut().legend_open = false;
    harness.run_steps(3);
    let labels = harness
        .state()
        .scene
        .toolpath
        .as_ref()
        .expect("generated")
        .op_labels();
    let &(at, op) = labels.last().expect("an operation is numbered");
    // On the number, up and right of the circle.
    let pos = harness.state().view.to_screen(at) + egui::vec2(11.0, -11.0);
    harness.hover_at(pos);
    harness.run_steps(1);
    harness.event(egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: egui::Modifiers::NONE,
    });
    harness.run_steps(1);
    harness.event(egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: egui::Modifiers::NONE,
    });
    harness.run_steps(3);
    assert_eq!(harness.state().focus, Focus::Op(op));
}

/// Whether the stock's position list shows: one of its two places at the drawing origin.
fn position_shown(harness: &Harness<'_, ChiploadApp>) -> bool {
    use egui_kittest::kittest::NodeT;
    harness
        .query_all_by_role(egui::accesskit::Role::ComboBox)
        .filter_map(|n| n.accesskit_node().value())
        .any(|v| v == "corner at drawing origin" || v == "centred on drawing origin")
}

/// The stock's size is one choice of two (docs/specs/job-settings-layout.md §2): the
/// outline plus a margin, or a fixed size, both centred on the outline (the user,
/// 2026-10-28). Fixed keeps the size it had. An older project's stock at the drawing origin
/// keeps its place, with its position shown, until centred.
#[test]
fn the_stock_size_switches_between_outline_and_margin_and_a_fixed_size() {
    let mut harness = harness("en", WINDOW);
    harness.state_mut().focus = Focus::Job;
    harness.run_steps(3);
    let settings = |h: &Harness<'_, ChiploadApp>| h.state().edit.as_ref().unwrap().settings.clone();
    let old = settings(&harness);
    assert_eq!(old.stock.fit, None, "the sample's stock is fixed");
    assert_ne!(old.stock.origin, chipload_job::StockOrigin::Drawing);
    assert!(position_shown(&harness));
    harness.get_by_label("Outline + margin").click();
    harness.run_steps(3);
    let fitted = settings(&harness);
    assert_eq!(
        fitted.stock.fit,
        Some(chipload_pipeline::DEFAULT_STOCK_MARGIN)
    );
    assert_eq!(fitted.stock.origin, chipload_job::StockOrigin::Drawing);
    harness.get_by_label("Fixed size, centred").click();
    harness.run_steps(3);
    let fixed = settings(&harness);
    assert_eq!(fixed.stock.fit, None);
    assert_eq!(fixed.stock.origin, chipload_job::StockOrigin::Drawing);
    assert_eq!(fixed.stock_size(), fitted.stock_size());
    assert_eq!(fixed.stock_bounds(), fitted.stock_bounds());
    assert!(!position_shown(&harness), "centred: no position to choose");
}

/// An array tool can always be finished (the user, 2026-10-28): its Create and Cancel are
/// under Selection whatever was open, and its ribbon button ends it again.
#[test]
fn an_array_tool_shows_create_and_cancel_and_ends() {
    use crate::app::modify::ModifyTool;
    let mut harness = harness("en", WINDOW);
    harness.get_by_label("S2").click();
    harness.run_steps(3);
    harness.state_mut().focus = Focus::Job;
    harness.state_mut().start_modify(ModifyTool::PolarArray);
    harness.run_steps(3);
    harness.get_by_label("Cancel").click();
    harness.run_steps(3);
    assert_eq!(harness.state().modify, None);

    let shapes = harness
        .state()
        .edit
        .as_ref()
        .unwrap()
        .settings
        .drawing
        .shapes
        .len();
    harness.state_mut().start_modify(ModifyTool::Array);
    harness.run_steps(3);
    harness.get_by_label("Create").click();
    harness.run_steps(3);
    assert_eq!(harness.state().modify, None);
    assert!(
        harness
            .state()
            .edit
            .as_ref()
            .unwrap()
            .settings
            .drawing
            .shapes
            .len()
            > shapes
    );

    harness.state_mut().start_modify(ModifyTool::Array);
    harness.run_steps(3);
    harness.get_by_label("Array").click();
    harness.run_steps(3);
    assert_eq!(harness.state().modify, None, "its ribbon button ends it");
}

/// A project started empty takes a DXF or SVG as well (the user, 2026-10-28): *Add DXF* is
/// on, and adds the file beside the drawn shapes.
#[test]
fn a_project_started_empty_can_add_a_drawing_file() {
    use egui_kittest::kittest::NodeT;
    let mut harness = harness("en", WINDOW);
    harness.state_mut().new_project();
    harness.run_steps(3);
    assert!(
        !harness
            .get_by_label("Add DXF")
            .accesskit_node()
            .is_disabled()
    );
    assert!(
        harness
            .state_mut()
            .add_file_from(&corpus("dxf/simple_part.dxf"), None, None)
    );
    let settings = &harness.state().edit.as_ref().unwrap().settings;
    assert_eq!(settings.drawing.dxfs.len(), 1);
    assert!(!settings.drawing.shapes.is_empty());
}

/// Job settings in the order a job is set up (docs/specs/job-settings-layout.md §2): the
/// zero reads from its section's summary line, and the machine, set once, starts closed.
#[test]
fn job_settings_show_the_zero_and_keep_the_machine_closed() {
    let mut harness = harness("en", WINDOW);
    harness.state_mut().focus = Focus::Job;
    harness.run_steps(3);
    assert!(
        harness
            .query_by_label_contains("X0 Y0 drawing origin · Z0 stock top")
            .is_some()
    );
    assert!(harness.query_by_label("Thickness").is_some());
    assert!(harness.query_by_label("Post").is_none(), "machine closed");
    harness.get_by_label_contains("safe Z").click();
    harness.run_steps(3);
    assert!(harness.query_by_label("Post").is_some());
    // The zero's line opens its own properties (docs/specs/work-zero-marker.md §4).
    harness
        .get_by_label("X0 Y0 drawing origin · Z0 stock top")
        .click();
    harness.run_steps(3);
    assert_eq!(harness.state().focus, Focus::WorkZero);
}

/// The legend and the navigation bar never overlap: in a narrow canvas the legend stands on
/// the bar, in a wide one the bar keeps to its right (seen 2026-10-28 in German at
/// 1200 × 800: the bar covered the legend's rows).
#[test]
fn the_legend_stays_clear_of_the_navigation_bar() {
    let nav_top = |h: &Harness<'_, ChiploadApp>| {
        h.ctx
            .data(|d| d.get_temp::<f32>(egui::Id::new(super::super::navigation::NAV_TOP)))
    };
    let mut narrow = harness("de", WINDOW);
    narrow.run_steps(4);
    let top = nav_top(&narrow).expect("no room beside the legend");
    // The legend's last row, "Rohteil": the leftmost of that name (Properties has one too).
    let last = narrow
        .query_all_by_label("Rohteil")
        .map(|n| n.rect())
        .min_by(|a, b| a.left().total_cmp(&b.left()))
        .expect("the legend's stock row");
    assert!(last.bottom() <= top, "{last:?} over the bar at {top}");
    let mut wide = harness("de", egui::vec2(1900.0, 800.0));
    wide.run_steps(4);
    assert_eq!(nav_top(&wide), None, "beside the legend");
}
