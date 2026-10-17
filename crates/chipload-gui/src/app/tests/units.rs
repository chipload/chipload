// SPDX-License-Identifier: GPL-3.0-or-later
//! Display units in the app (docs/specs/inch-units.md §5).
use chipload_job::Units;

use super::*;

/// Every text Properties paints in one frame, with lengths shown in `units`.
fn properties_in(app: &mut ChiploadApp, units: Units) -> String {
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
    let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
        crate::units::set_display(ui.ctx(), units);
        app.properties_panel(ui);
    });
    output.textures_delta.clear();
    let mut text = String::new();
    for clipped in &output.shapes {
        collect(&clipped.shape, &mut text);
    }
    text
}

/// A number followed by mm: a length shown in mm.
fn mm_lengths(text: &str) -> Vec<String> {
    let words: Vec<&str> = text.split_whitespace().collect();
    words
        .windows(2)
        .filter(|w| {
            (w[1] == "mm" || w[1].starts_with("mm,") || w[1].starts_with("mm/"))
                && w[0]
                    .trim_start_matches(['X', 'Y', '-'])
                    .parse::<f64>()
                    .is_ok()
        })
        .map(|w| w.join(" "))
        .collect()
}

#[test]
fn with_inches_every_form_shows_inches() {
    let mut seen_in_mm = 0;
    for project in [
        "jobs/v18/holes.chipload",
        "jobs/v18/dogbones.chipload",
        "jobs/v17/finish_skin.chipload",
        "jobs/v10/vcarve_letters.chipload",
        "jobs/v22/two_parts.chipload",
        "jobs/v27/text.chipload",
        "jobs/v7/leads.chipload",
    ] {
        let mut app = ChiploadApp::default();
        assert!(app.open(&corpus(project)), "{project}");
        let settings = &app.edit.as_ref().unwrap().settings;
        let mut focuses = vec![Focus::Job, Focus::Tools];
        focuses.extend(settings.tools.iter().map(|t| Focus::Tool(t.id)));
        focuses.extend(settings.drawing.dxfs.iter().map(|d| Focus::Dxf(d.id)));
        focuses.extend(settings.drawing.images.iter().map(|i| Focus::Image(i.id)));
        focuses.extend(settings.drawing.texts.iter().map(|t| Focus::Text(t.id)));
        focuses.extend(settings.parts.iter().map(|p| Focus::Part(p.id)));
        focuses.extend((0..app.edit.as_ref().unwrap().ops.len()).map(Focus::Op));
        for focus in focuses {
            app.focus = focus;
            let mm = properties_in(&mut app, Units::Mm);
            let inch = properties_in(&mut app, Units::Inch);
            seen_in_mm += mm_lengths(&mm).len();
            let left = mm_lengths(&inch);
            assert!(left.is_empty(), "{project} {focus:?}: {left:?}\n{inch}");
            // Where mm shows a length, inches show one too.
            if !mm_lengths(&mm).is_empty() || mm.contains(" mm") {
                assert!(
                    inch.contains(" in"),
                    "{project} {focus:?}: no inches\n{inch}"
                );
            }
        }
    }
    // The check sees lengths: the same forms in mm show many.
    assert!(seen_in_mm > 50, "{seen_in_mm} lengths in mm");
}

#[test]
fn a_length_field_shows_inches_and_keeps_mm() {
    let ctx = egui::Context::default();
    let mut value = 6.35;
    let mut text = String::new();
    let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
        crate::units::set_display(ui.ctx(), Units::Inch);
        ui.add(crate::units::length(&mut value));
        ui.add(crate::units::feed(&mut 1270.0));
    });
    output.textures_delta.clear();
    for clipped in &output.shapes {
        if let egui::Shape::Text(t) = &clipped.shape {
            text.push_str(t.galley.text());
            text.push('\n');
        }
    }
    // The number and its unit are drawn as two texts.
    let words: Vec<&str> = text.split_whitespace().collect();
    assert!(words.windows(2).any(|w| w == ["0.2500", "in"]), "{text}");
    assert!(words.windows(2).any(|w| w == ["50.00", "in/min"]), "{text}");
    assert_eq!(value, 6.35, "showing it changes nothing");
}

#[test]
fn a_new_project_in_inches_starts_from_tidy_inch_values() {
    let mut app = ChiploadApp {
        units: Units::Inch,
        ..Default::default()
    };
    app.new_project();
    let job = &app.edit.as_ref().unwrap().settings;
    let inches = |mm: f64| mm / 25.4;
    assert!((inches(job.stock.x) - 24.0).abs() < 1e-12);
    assert!((inches(job.stock.y) - 12.0).abs() < 1e-12);
    assert!((inches(job.stock.thickness) - 0.75).abs() < 1e-12);
    assert!((inches(job.machine.safe_z) - 0.2).abs() < 1e-12);
    assert!((inches(job.tools[0].diameter) - 0.25).abs() < 1e-12);
    // The G-code stays mm until the machine says otherwise.
    assert_eq!(job.machine.gcode_units, Units::Mm);
    // In mm, nothing changes.
    let mut app = ChiploadApp::default();
    app.new_project();
    assert_eq!(app.edit.as_ref().unwrap().settings.stock.thickness, 10.0);
}

#[test]
fn with_inches_the_report_shows_its_notes_and_checks_in_inches() {
    // docs/specs/inch-units.md §5.1: a project with a cut below the stock (a note) and moves
    // below its bottom (a checker warning), both written with mm lengths by the library.
    let mut app = ChiploadApp::default();
    assert!(app.open(&corpus("jobs/v18/dogbones.chipload")));
    assert!(app.generate_and_wait(), "{:?}", app.report);
    assert!(
        app.report.iter().any(|l| l.contains("mm below the stock")),
        "{:?}",
        app.report
    );
    app.report_open = true;
    let painted = |units: Units, app: &mut ChiploadApp| {
        let ctx = egui::Context::default();
        let mut text = String::new();
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            crate::units::set_display(ui.ctx(), units);
            app.report_panel(ui);
        });
        output.textures_delta.clear();
        for clipped in &output.shapes {
            if let egui::Shape::Text(t) = &clipped.shape {
                text.push_str(t.galley.text());
                text.push('\n');
            }
        }
        text
    };
    let mm = painted(Units::Mm, &mut app);
    assert!(mm.contains("mm below the stock"), "{mm}");
    let inch = painted(Units::Inch, &mut app);
    assert!(inch.contains("in below the stock"), "{inch}");
    assert_eq!(mm_lengths(&inch), Vec::<String>::new(), "{inch}");
}
