// SPDX-License-Identifier: GPL-3.0-or-later
//! The user guide's screenshots (docs/specs/user-guide.md §3), rendered headless with wgpu:
//! each opens a corpus project, sets up a view (`screenshot_setup`) and saves a frame. Run by
//! packaging/docs/screenshots.sh, which sets `CHIPLOAD_SHOTS_OUT` (the folder to write to),
//! `CHIPLOAD_SHOTS_LIBRARY` (a settings folder with a library, for `library.png`) and
//! `CHIPLOAD_SHOTS_LANGUAGE` (en unless set).
use super::*;

/// Name, project (from the repository's root), setup steps, window size.
const SHOTS: &[(&str, &str, &str, (f32, f32))] = &[
    (
        "toolpath",
        "tests/corpus/jobs/v18/dogbones.chipload",
        "generate",
        (1200.0, 800.0),
    ),
    (
        "simulation-3d",
        "tests/corpus/jobs/v18/dogbones.chipload",
        "simulation,3d",
        (1200.0, 800.0),
    ),
    (
        "playback",
        "tests/corpus/jobs/v18/dogbones.chipload",
        "simulation,half",
        (1200.0, 800.0),
    ),
    (
        "image",
        "tests/corpus/jobs/v19/image_vcarve.chipload",
        "image",
        (1200.0, 800.0),
    ),
    (
        "drawing",
        "tests/corpus/jobs/v21/two_dxfs.chipload",
        "drawing",
        (1200.0, 800.0),
    ),
    (
        "properties",
        "tests/corpus/jobs/v18/dogbones.chipload",
        "generate,select",
        (1200.0, 800.0),
    ),
    (
        "parts",
        "tests/corpus/jobs/v22/two_parts.chipload",
        "generate,part",
        (1200.0, 800.0),
    ),
    // The README's picture: a larger window, the simulated stock with the toolpaths over it,
    // the logo V-carved from a traced picture (the picture itself hidden), the outline open.
    (
        "showcase",
        "packaging/docs/showcase.chipload",
        "simulation,3d,noreport,noimages,op4",
        (1600.0, 950.0),
    ),
    // The Text page: a text and its properties.
    (
        "text",
        "tests/corpus/jobs/v27/text.chipload",
        "generate,select,text1",
        (1200.0, 800.0),
    ),
    // Drawing: a value typed while drawing a polyline (docs/specs/drawing-input.md §2).
    (
        "typing",
        "tests/corpus/jobs/v5/drawn_part.chipload",
        "typing",
        (1200.0, 800.0),
    ),
    // V-carving: a sign too wide for its V-carve, its floor cleared by a floor pass, whose
    // form is open (docs/specs/vcarve-wide-strokes.md).
    (
        "vcarve-floor",
        "packaging/docs/vcarve-floor.chipload",
        "simulation,noimages,floor",
        (1200.0, 800.0),
    ),
    // Setups: two boards, the second active in full colour, the first a ghost
    // (docs/specs/setups.md).
    (
        "setups",
        "tests/corpus/jobs/v34/two_setups.chipload",
        "setup2,generate",
        (1200.0, 800.0),
    ),
    // The library: a settings folder of its own with two tools and a machine.
    (
        "library",
        "tests/corpus/jobs/v22/two_parts.chipload",
        "library",
        (1200.0, 800.0),
    ),
];

#[test]
#[ignore = "writes the guide's pictures; run by packaging/docs/screenshots.sh"]
fn screenshots() {
    use egui_kittest::Harness;
    let Ok(out) = std::env::var("CHIPLOAD_SHOTS_OUT") else {
        panic!("set CHIPLOAD_SHOTS_OUT to the folder to write to");
    };
    let out = PathBuf::from(out);
    std::fs::create_dir_all(&out).unwrap();
    let language = std::env::var("CHIPLOAD_SHOTS_LANGUAGE").unwrap_or_else(|_| "en".into());
    let library = std::env::var("CHIPLOAD_SHOTS_LIBRARY")
        .ok()
        .map(PathBuf::from);
    // An empty settings folder: the pictures don't depend on this computer's library.
    let empty = temp_dir("shots-settings");
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    for &(name, project, steps, (width, height)) in SHOTS {
        let settings = match (name, &library) {
            ("library", Some(dir)) => dir.clone(),
            ("library", None) => panic!("set CHIPLOAD_SHOTS_LIBRARY for library.png"),
            _ => empty.clone(),
        };
        let project = root.join(project);
        let language = language.clone();
        let mut harness = Harness::builder()
            .with_size(egui::vec2(width, height))
            .wgpu()
            .build_eframe(move |cc| {
                if let Ok(mut fonts) = crate::fonts::system_fonts() {
                    crate::fonts::add_text_fonts(&mut fonts);
                    cc.egui_ctx.set_fonts(fonts);
                }
                crate::style::apply(&cc.egui_ctx);
                let mut app = ChiploadApp::default();
                app.set_settings_dir(Some(settings));
                app.set_language(&language);
                assert!(app.open(&project), "{}", project.display());
                app.screenshot_setup(&cc.egui_ctx, steps);
                app
            });
        harness.run_steps(10);
        let image = harness.render().unwrap();
        image.save(out.join(format!("{name}.png"))).unwrap();
        println!("{name}.png");
    }
}
