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
    assert!(
        text.contains("set the tool's flutes to see the chip load"),
        "{text}"
    );
    app.edit.as_mut().unwrap().settings.tools[0].flutes = Some(2);
    let feed = chipload_pipeline::chip_load::cutting_feed(&app.edit.as_ref().unwrap().ops[1].op);
    let rpm = app.edit.as_ref().unwrap().settings.tools[0].rpm.unwrap();
    let text = properties_text(&mut app);
    let expected = format!("{:.3} mm/tooth", feed / (rpm * 2.0));
    assert!(text.contains(&expected), "{expected} in {text}");
}
