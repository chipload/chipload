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

/// The window as pictures, to look at: each language at the default size and English in a
/// narrow window, written to CHIPLOAD_PICTURES (a folder).
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
}
