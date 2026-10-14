// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

/// Draw the top bar, the ribbon and the status bar for one frame.
fn draw_header(app: &mut ChiploadApp) {
    let ctx = egui::Context::default();
    let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
        app.top_bar(ui);
        app.ribbon(ui);
        app.status_bar(ui);
    });
    output.textures_delta.clear();
}

#[test]
fn the_header_draws_with_and_without_a_project() {
    let mut app = ChiploadApp::default();
    draw_header(&mut app);
    let mut app = lower_part();
    draw_header(&mut app);
    // With a selection (the operation buttons on), unsaved changes and a result.
    app.click(None, false);
    edit_feed(&mut app, 650.0);
    assert!(app.dirty());
    assert!(app.generate_and_wait(), "{:?}", app.report);
    draw_header(&mut app);
}
