// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

/// Every text the properties panel paints in one frame.
fn properties_text(app: &mut ChiploadApp) -> String {
    fn collect(shape: &egui::Shape, out: &mut String) {
        match shape {
            egui::Shape::Text(text) => {
                out.push_str(text.galley.text());
                out.push('\n');
            }
            egui::Shape::Vec(shapes) => shapes.iter().for_each(|s| collect(s, out)),
            _ => {}
        }
    }
    let ctx = egui::Context::default();
    let mut output = ctx.run_ui(egui::RawInput::default(), |ui| app.properties_panel(ui));
    output.textures_delta.clear();
    let mut text = String::new();
    for clipped in &output.shapes {
        collect(&clipped.shape, &mut text);
    }
    text
}

#[test]
fn the_feed_shows_the_chip_load_once_the_tool_has_flutes() {
    let mut app = lower_part();
    app.focus = Focus::Op(1);
    // The default tool has no flutes yet.
    let text = properties_text(&mut app);
    assert!(text.contains("Chip load\nflutes not set"), "{text}");
    app.edit.as_mut().unwrap().settings.tools[0].flutes = Some(2);
    let feed = chipload_pipeline::chip_load::cutting_feed(&app.edit.as_ref().unwrap().ops[1].op);
    let rpm = app.edit.as_ref().unwrap().settings.tools[0].rpm.unwrap();
    let text = properties_text(&mut app);
    let expected = format!("{:.3} mm/tooth", feed / (rpm * 2.0));
    assert!(text.contains(&expected), "{expected} in {text}");
}

/// A project's tool shows its flutes in Properties, where they can be set (the user,
/// 2026-10-29: only the library had them).
#[test]
fn a_project_tool_shows_its_flutes() {
    let mut app = lower_part();
    let id = app.edit.as_ref().unwrap().settings.tools[0].id;
    app.focus = Focus::Tool(id);
    assert!(properties_text(&mut app).contains("Flutes"));
}

/// An operation's feeds with the chip load row, rendered to look at:
/// `CHIPLOAD_PICTURES=/some/dir cargo test -p chipload-gui feeds_pictures -- --ignored`.
#[test]
#[ignore = "writes pictures; run by hand"]
fn feeds_pictures() {
    use egui_kittest::Harness;
    let Ok(dir) = std::env::var("CHIPLOAD_PICTURES") else {
        panic!("set CHIPLOAD_PICTURES to the folder to write to");
    };
    let dir = PathBuf::from(dir);
    std::fs::create_dir_all(&dir).unwrap();
    for (name, language, width, flutes) in [
        ("feeds-en", "en", 1200.0, None),
        ("feeds-de", "de", 1200.0, Some(2)),
        ("feeds-sv-narrow", "sv", 900.0, None),
    ] {
        let mut harness = Harness::builder()
            .with_size(egui::vec2(width, 800.0))
            .wgpu()
            .build_eframe(move |cc| {
                if let Ok(mut fonts) = crate::fonts::system_fonts() {
                    crate::fonts::add_text_fonts(&mut fonts);
                    cc.egui_ctx.set_fonts(fonts);
                }
                crate::style::apply(&cc.egui_ctx);
                let mut app = lower_part();
                app.set_language(language);
                app.edit.as_mut().unwrap().settings.tools[0].flutes = flutes;
                app.commit();
                app.focus = Focus::Op(1);
                app
            });
        harness.run_steps(6);
        let image = harness.render().unwrap();
        image.save(dir.join(format!("{name}.png"))).unwrap();
    }
}
