// SPDX-License-Identifier: GPL-3.0-or-later
//! The user guide's screenshots (docs/specs/user-guide.md §3), rendered headless with wgpu:
//! each opens a corpus project, sets up a view (`screenshot_setup`) and saves a frame. Run by
//! packaging/docs/screenshots.sh, which sets `CHIPLOAD_SHOTS_OUT` (the folder to write to),
//! `CHIPLOAD_SHOTS_LIBRARY` (a settings folder with a library, for `library.png` and
//! `machine.png`) and
//! `CHIPLOAD_SHOTS_LANGUAGE` (en unless set) and `CHIPLOAD_SHOTS_THEME` (dark unless
//! `light`).
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
    // A 3D model in the 3D view, its properties open (docs/book/src/drawings/models.md).
    (
        "model",
        "tests/corpus/jobs/v36/relief_dome.chipload",
        "model,3d",
        (1200.0, 800.0),
    ),
    // *Turn and cut…* (docs/book/src/drawings/models.md).
    (
        "turn-cut",
        "tests/corpus/jobs/v36/relief_dome.chipload",
        "turn-cut",
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
    // The README's second picture: a word in a script font V-carved with a 25° V-bit.
    (
        "showcase-text",
        "packaging/docs/showcase-text.chipload",
        "simulation,3d,noreport,notoolpath,nodrawing,nopanels",
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
    // V-carving in passes of equal load: the V-carve open, its depths under *Passes*
    // (docs/specs/v-carve.md decision 12).
    (
        "vcarve-passes",
        "tests/corpus/jobs/v34/vcarve_step_down.chipload",
        "generate,noimages,op",
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
    // Signs: Draw > Frame… round a text, the frame's preview and its values
    // (docs/specs/shape-library.md §5).
    (
        "frame",
        "tests/corpus/jobs/v27/text.chipload",
        "frame",
        (1200.0, 800.0),
    ),
    // Signs: New sign… filled in (docs/specs/sign-maker.md §3).
    (
        "new-sign",
        "tests/corpus/jobs/v27/text.chipload",
        "new-sign,noreport",
        (1200.0, 1000.0),
    ),
    // Signs: the golden sign (framed letters V-carved, the border grooved, cut out),
    // simulated.
    (
        "sign",
        "tests/corpus/jobs/v35/sign_frame.chipload",
        "simulation,3d,notoolpath,noreport",
        (1200.0, 800.0),
    ),
    // The shape library's tab beside the canvas, the sign selected (docs/specs/
    // shape-library.md §5.1).
    (
        "shape-library",
        "tests/corpus/jobs/v35/sign_frame.chipload",
        "shapes,noreport",
        (1200.0, 800.0),
    ),
    // The library: a settings folder of its own with two tools and a machine.
    (
        "library",
        "tests/corpus/jobs/v22/two_parts.chipload",
        "library",
        (1200.0, 800.0),
    ),
    // Machine profiles: the project's machine from the library's profile, its values greyed
    // under *From the profile* (docs/specs/linked-machine.md decision 7).
    (
        "machine",
        "tests/corpus/jobs/v22/two_parts.chipload",
        "machine",
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
    let theme = match std::env::var("CHIPLOAD_SHOTS_THEME").as_deref() {
        Ok("light") => chipload_job::library::Theme::Light,
        _ => chipload_job::library::Theme::Dark,
    };
    let library = std::env::var("CHIPLOAD_SHOTS_LIBRARY")
        .ok()
        .map(PathBuf::from);
    // An empty settings folder: the pictures don't depend on this computer's library.
    let empty = temp_dir("shots-settings");
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    for &(name, project, steps, (width, height)) in SHOTS {
        let settings = match (name, &library) {
            ("library" | "machine", Some(dir)) => dir.clone(),
            ("library" | "machine", None) => {
                panic!("set CHIPLOAD_SHOTS_LIBRARY for {name}.png")
            }
            // Its own folder, so the entries it saves are in no other picture.
            ("shape-library", _) => temp_dir("shots-shapes"),
            _ => empty.clone(),
        };
        let project = root.join(project);
        let language = language.clone();
        let mut harness = Harness::builder()
            .with_size(egui::vec2(width, height))
            .wgpu()
            .build_eframe(move |cc| {
                {
                    let mut fonts = crate::fonts::ui_fonts();
                    crate::fonts::add_text_fonts(&mut fonts);
                    cc.egui_ctx.set_fonts(fonts);
                }
                crate::style::apply(&cc.egui_ctx);
                let mut app = ChiploadApp::default();
                app.set_settings_dir(Some(settings));
                // The guide's pictures show the work, not the question about new versions.
                app.updates.bar_closed = true;
                app.set_language(&language);
                app.prefs.theme = theme;
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

/// The README's relief showcase (the user, 2026-11-14): a 3DBenchy relief, simulated, in
/// 3D (`showcase-relief.png`), and its playback as frames for a GIF (`relief-frames/`).
/// The project embeds the model (15 MB), so it stays out of the repository: its path comes
/// from `CHIPLOAD_RELIEF_SHOWCASE`. Run by packaging/docs/relief-showcase.sh.
/// The playback's frames at `(k / count)^FRAME_EASE` of its time.
const FRAME_EASE: f64 = 1.6;

#[test]
#[ignore = "writes the README's relief pictures; run by packaging/docs/relief-showcase.sh"]
fn relief_showcase() {
    use egui_kittest::Harness;
    let (Ok(out), Ok(project)) = (
        std::env::var("CHIPLOAD_SHOTS_OUT"),
        std::env::var("CHIPLOAD_RELIEF_SHOWCASE"),
    ) else {
        panic!("set CHIPLOAD_SHOTS_OUT and CHIPLOAD_RELIEF_SHOWCASE");
    };
    let out = PathBuf::from(out);
    let frames = out.join("relief-frames");
    std::fs::create_dir_all(&frames).unwrap();
    let count: usize = std::env::var("CHIPLOAD_RELIEF_FRAMES")
        .ok()
        .and_then(|n| n.parse().ok())
        .unwrap_or(48);
    let empty = temp_dir("relief-showcase-settings");
    let open = move |size: (f32, f32), project: PathBuf, settings: PathBuf| {
        Harness::builder()
            .with_size(egui::vec2(size.0, size.1))
            .wgpu()
            .build_eframe(move |cc| {
                {
                    let mut fonts = crate::fonts::ui_fonts();
                    crate::fonts::add_text_fonts(&mut fonts);
                    cc.egui_ctx.set_fonts(fonts);
                }
                crate::style::apply(&cc.egui_ctx);
                let mut app = ChiploadApp::default();
                app.set_settings_dir(Some(settings));
                app.updates.bar_closed = true;
                app.set_language("en");
                assert!(app.open(&project), "{}", project.display());
                app.screenshot_setup(
                    &cc.egui_ctx,
                    "simulation,3d,noreport,notoolpath,nodrawing,nopanels",
                );
                app
            })
    };
    let project = PathBuf::from(project);
    let mut harness = open((1600.0, 950.0), project.clone(), empty.clone());
    harness.run_steps(10);
    let image = harness.render().unwrap();
    image.save(out.join("showcase-relief.png")).unwrap();
    println!("showcase-relief.png");
    // The playback, the tool shown cutting: frames closer together at the start, where
    // the roughing takes the board down fast, than in the long finish.
    let mut harness = open((960.0, 600.0), project, empty);
    harness.run_steps(4);
    let total = harness.state().playback.as_ref().map_or(0.0, |p| p.total());
    assert!(
        total > 0.0,
        "no playback: the program may be too long to play"
    );
    for k in 0..=count {
        if let Some(playback) = harness.state_mut().playback.as_mut() {
            playback.set_time(total * (k as f64 / count as f64).powf(FRAME_EASE));
        }
        harness.run_steps(3);
        let image = harness.render().unwrap();
        image.save(frames.join(format!("{k:03}.png"))).unwrap();
    }
    println!("relief-frames/ ({} frames)", count + 1);
}
