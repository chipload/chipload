// SPDX-License-Identifier: GPL-3.0-or-later
//! A narrow Properties panel, and folding the side panels for a larger canvas (user report
//! 2026-10-17: some pages kept the panel wide).
use super::*;

/// The width the Properties page in `app.focus` takes when laid out in `width`, after two
/// frames (grids size their columns from the frame before).
fn page_width(app: &mut ChiploadApp, width: f32) -> f32 {
    let ctx = egui::Context::default();
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1400.0, 2000.0),
        )),
        ..Default::default()
    };
    let mut taken = 0.0;
    for _ in 0..2 {
        let mut output = ctx.run_ui(input.clone(), |ui| {
            let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(width, 2000.0));
            let mut panel = ui.new_child(egui::UiBuilder::new().max_rect(rect));
            app.properties_panel(&mut panel);
            taken = panel.min_rect().width();
        });
        output.textures_delta.clear();
    }
    taken
}

#[test]
fn every_properties_page_fits_the_narrowest_panel() {
    let mut app = lower_part();
    app.edit
        .as_mut()
        .unwrap()
        .job
        .tools
        .push(chipload_job::Tool {
            id: 2,
            name: None,
            library_uid: None,
            kind: ToolType::Vbit,
            diameter: 12.7,
            flutes: Some(2),
            center_cutting: Some(true),
            rpm: Some(18000.0),
            angle: Some(60.0_f64.to_radians()),
            tip_diameter: Some(0.2),
        });
    app.commit();
    for kind in [OpKind::Vcarve, OpKind::Path, OpKind::Holes] {
        select(&mut app, &[4, 6]);
        app.add_operation(kind).unwrap();
    }
    app.selection = CanvasSelection::Nothing;
    app.add_operation(OpKind::Surface).unwrap();
    app.start_text();
    let text = app.place_text_at((20.0, 30.0)).unwrap();
    // Generated, so the pages show their results too (the holes find no circle: a problem
    // on their page, which must fit as well).
    app.generate_and_wait();
    select(&mut app, &[4, 6]);
    let edit = app.edit.as_ref().unwrap();
    let part = edit.job.parts[0].id;
    let mut pages = vec![
        Focus::Job,
        Focus::Part(part),
        Focus::Tools,
        Focus::Tool(1),
        Focus::Tool(2),
        Focus::Dxf(1),
        Focus::Text(text),
    ];
    pages.extend((0..edit.job.operations.len()).map(Focus::Op));
    // The panel's least width, less its frame's margins.
    let width = crate::app::layout::PROPERTIES_MIN_WIDTH - 16.0;
    let mut wide = Vec::new();
    for page in pages {
        app.focus = page;
        let taken = page_width(&mut app, width);
        if taken > width + 0.5 {
            wide.push(format!("{page:?}: {taken:.0} px"));
        }
    }
    assert!(wide.is_empty(), "wider than {width} px: {wide:?}");
}

#[test]
fn p_hides_every_panel_and_brings_them_back_as_they_were() {
    use crate::app::layout::Pane;
    let mut app = lower_part();
    let open = |app: &ChiploadApp| Pane::PANELS.map(|p| app.panels.is_open(p));
    assert_eq!(open(&app), [true; 4]);
    app.panels.close(Pane::Shapes);
    app.panels.toggle_hidden();
    assert_eq!(open(&app), [false; 4]);
    app.panels.toggle_hidden();
    assert_eq!(open(&app), [true, true, false, true], "as they were");
    // A panel asked for while hidden brings the others back too.
    app.panels.toggle_hidden();
    app.panels.show(Pane::Properties);
    assert_eq!(open(&app), [true, true, false, true]);
}

/// A narrow Report wraps its first line instead of cutting it off (the user, 2026-11-03:
/// the start hint ended in "(Ctr…").
#[test]
fn a_narrow_report_wraps_its_first_line() {
    let mut app = ChiploadApp::default();
    let first = app.report.lines[0].clone();
    let ctx = egui::Context::default();
    // The fonts main.rs sets: a bare context has none to measure with.
    let mut fonts = crate::fonts::ui_fonts();
    crate::fonts::add_text_fonts(&mut fonts);
    ctx.set_fonts(fonts);
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1400.0, 2000.0),
        )),
        ..Default::default()
    };
    let mut size = egui::Vec2::ZERO;
    let mut row = 0.0;
    for _ in 0..3 {
        let mut output = ctx.run_ui(input.clone(), |ui| {
            let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(260.0, 2000.0));
            let mut panel = ui.new_child(egui::UiBuilder::new().max_rect(rect));
            app.report_line(&mut panel, &first, true);
            size = panel.min_rect().size();
            row = panel.text_style_height(&egui::TextStyle::Body);
        });
        output.textures_delta.clear();
    }
    assert!(size.x <= 260.0, "{size:?}");
    // More than one line high: the text is all there, not cut off after one.
    assert!(size.y > 1.5 * row, "{size:?}, a line {row}");
}
