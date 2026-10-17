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
        .settings
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
    let part = edit.settings.parts[0].id;
    let mut pages = vec![
        Focus::Job,
        Focus::Part(part),
        Focus::Tools,
        Focus::Tool(1),
        Focus::Tool(2),
        Focus::Dxf(1),
        Focus::Text(text),
    ];
    pages.extend((0..edit.ops.len()).map(Focus::Op));
    // The panel's least width, less its frame's margins.
    let width = PROPERTIES_MIN_WIDTH - 16.0;
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
fn p_folds_both_side_panels_and_opens_them_again() {
    let mut app = lower_part();
    assert!(app.browser_open && app.properties_open);
    app.toggle_panels();
    assert!(!app.browser_open && !app.properties_open);
    app.toggle_panels();
    assert!(app.browser_open && app.properties_open);
    // One folded: P folds the other too.
    app.properties_open = false;
    app.toggle_panels();
    assert!(!app.browser_open && !app.properties_open);
}
