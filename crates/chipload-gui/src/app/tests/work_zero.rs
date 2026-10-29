// SPDX-License-Identifier: GPL-3.0-or-later
//! The work zero in the Browser, the view and Properties (docs/specs/work-zero-marker.md).
use super::*;
use chipload_job::{StockCorner, WorkZero};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;

fn harness() -> Harness<'static, ChiploadApp> {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1200.0, 800.0))
        .build_eframe(|_| {
            let mut app = ChiploadApp::default();
            assert!(app.open(&corpus("jobs/v5/drawn_part.chipload")));
            app
        });
    harness.run_steps(6);
    harness
}

fn zero(h: &Harness<'_, ChiploadApp>) -> WorkZero {
    h.state().edit.as_ref().unwrap().settings.machine.work_zero
}

/// A drawing point on the screen.
fn screen(h: &Harness<'_, ChiploadApp>, p: (f64, f64)) -> egui::Pos2 {
    h.state().view.to_screen(p)
}

/// Press at `from`, move to `to` in steps, and let go there unless `escape` is pressed
/// first.
fn drag(h: &mut Harness<'_, ChiploadApp>, from: egui::Pos2, to: egui::Pos2, escape: bool) {
    h.hover_at(from);
    h.run_steps(2);
    h.drag_at(from);
    h.run_steps(2);
    for k in 1..=5 {
        h.hover_at(from + (to - from) * (k as f32 / 5.0));
        h.run_steps(2);
    }
    if escape {
        h.key_press(egui::Key::Escape);
        h.run_steps(2);
    }
    h.drop_at(to);
    h.run_steps(3);
}

#[test]
fn the_work_zero_is_selected_in_the_browser_and_dragged_onto_a_stock_corner() {
    let mut h = harness();
    let start = zero(&h);
    let corner = |h: &Harness<'_, ChiploadApp>, c: StockCorner| {
        h.state().edit.as_ref().unwrap().settings.stock_corner_xy(c)
    };
    let here = h.state().edit.as_ref().unwrap().settings.work_zero_xy();
    let target = corner(&h, StockCorner::TopRight);
    // Not selected: a drag from the handle doesn't move it.
    let (from, to) = (screen(&h, here), screen(&h, target));
    drag(&mut h, from, to, false);
    assert_eq!(zero(&h), start, "not selected");
    // Selected in the Browser, dropped near the corner: that corner.
    h.state_mut().focus = Focus::Job;
    h.run_steps(2);
    // The Browser's row: the leftmost of the two (Job settings names it too).
    h.query_all_by_label("Work zero")
        .min_by(|a, b| a.rect().left().total_cmp(&b.rect().left()))
        .expect("the Browser row")
        .click();
    h.run_steps(3);
    assert_eq!(h.state().focus, Focus::WorkZero);
    let near = screen(&h, target) + egui::vec2(3.0, -2.0);
    let from = screen(&h, here);
    drag(&mut h, from, near, false);
    assert_eq!(
        zero(&h),
        WorkZero::Stock {
            corner: StockCorner::TopRight
        }
    );
    // One undo step.
    h.state_mut().undo();
    assert_eq!(zero(&h), start);
    // Escape during a drag: it stays.
    drag(&mut h, from, near, true);
    assert_eq!(zero(&h), start, "escape");
}

#[test]
fn the_work_zero_dropped_away_from_corners_is_a_point_and_alt_leaves_it_free() {
    let mut app = ChiploadApp::default();
    assert!(app.open(&corpus("jobs/v5/drawn_part.chipload")));
    app.focus = Focus::Job;
    assert!(!app.start_zero_drag(), "only once selected");
    app.focus = Focus::WorkZero;
    let ((x0, y0), _) = app.edit.as_ref().unwrap().settings.stock_bounds();
    // Free: exactly where the pointer is.
    let at = (x0 + 12.345, y0 + 7.89);
    assert!(app.start_zero_drag());
    let label = app.drag_zero_to(at, true).unwrap();
    assert!(label.starts_with("X "), "{label}");
    app.end_zero_drag();
    assert_eq!(zero_of(&app), WorkZero::Point { x: at.0, y: at.1 });
    // Near a corner, at this zoom: the corner.
    let reach = f64::from(crate::view::PICK_TOLERANCE_PX) / app.view.scale;
    assert!(app.start_zero_drag());
    let label = app
        .drag_zero_to((x0 + 0.5 * reach, y0 + 0.5 * reach), false)
        .unwrap();
    assert_eq!(label, "stock bottom left");
    app.end_zero_drag();
    assert_eq!(
        zero_of(&app),
        WorkZero::Stock {
            corner: StockCorner::BottomLeft
        }
    );
}

fn zero_of(app: &ChiploadApp) -> WorkZero {
    app.edit.as_ref().unwrap().settings.machine.work_zero
}

#[test]
fn the_work_zero_properties_set_a_stock_corner_or_the_drawing_origin() {
    let mut h = harness();
    h.state_mut().focus = Focus::WorkZero;
    h.run_steps(3);
    h.get_by_label("stock centre").click();
    h.run_steps(3);
    assert_eq!(
        zero(&h),
        WorkZero::Stock {
            corner: StockCorner::Center
        }
    );
    assert!(h.query_by_label("Z zero").is_some());
    h.get_by_label("Drawing origin").click();
    h.run_steps(3);
    assert_eq!(zero(&h), WorkZero::DrawingOrigin {});
}

/// The work zero selected, rendered to PNG files to look at, as `layout::pictures`:
/// `CHIPLOAD_PICTURES=/some/dir cargo test -p chipload-gui work_zero_pictures -- --ignored`.
#[test]
#[ignore = "writes pictures; run by hand"]
fn work_zero_pictures() {
    let Ok(dir) = std::env::var("CHIPLOAD_PICTURES") else {
        panic!("set CHIPLOAD_PICTURES to the folder to write to");
    };
    let dir = PathBuf::from(dir);
    std::fs::create_dir_all(&dir).unwrap();
    for (name, language, width) in [
        ("zero-en", "en", 1200.0),
        ("zero-de", "de", 1200.0),
        ("zero-sv-narrow", "sv", 900.0),
        ("zero-job-en", "en", 1200.0),
        ("zero-centre-en", "en", 1200.0),
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
                let mut app = ChiploadApp::default();
                app.set_language(language);
                assert!(app.open(&corpus("jobs/v31/svg_badge.chipload")));
                // Away from the drawing origin, which then shows as ghost axes.
                let corner = if name == "zero-centre-en" {
                    StockCorner::Center
                } else {
                    StockCorner::BottomLeft
                };
                app.edit.as_mut().unwrap().settings.machine.work_zero = WorkZero::Stock { corner };
                app.commit();
                app.focus = if name.starts_with("zero-job") {
                    Focus::Job
                } else {
                    Focus::WorkZero
                };
                app
            });
        harness.run_steps(6);
        let image = harness.render().unwrap();
        image.save(dir.join(format!("{name}.png"))).unwrap();
    }
}
