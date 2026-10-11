// SPDX-License-Identifier: GPL-3.0-or-later
//! Placing the imported drawing (docs/specs/drawing-placement.md).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};

use chipload_dxf::contours_from_shapes;
use chipload_job::{Geometry, Job, Placement, Shape, WorkZero, parse_job};
use chipload_pipeline::placement::{Move, place_drawing};
use chipload_pipeline::{PipelineError, empty_project, run_job, update_drawing};
use serde_json::json;

/// A DXF (text, mm) with closed LWPOLYLINE squares: (handle, layer, x, y, size).
fn square_dxf(path: &Path, squares: &[(&str, &str, f64, f64, f64)]) {
    let mut text = String::from("0\nSECTION\n2\nHEADER\n9\n$INSUNITS\n70\n4\n0\nENDSEC\n");
    text += "0\nSECTION\n2\nENTITIES\n";
    for (handle, layer, x, y, size) in squares {
        text += &format!("0\nLWPOLYLINE\n5\n{handle}\n8\n{layer}\n90\n4\n70\n1\n");
        for (px, py) in [
            (*x, *y),
            (x + size, *y),
            (x + size, y + size),
            (*x, y + size),
        ] {
            text += &format!("10\n{px}\n20\n{py}\n");
        }
    }
    text += "0\nENDSEC\n0\nEOF\n";
    std::fs::write(path, text).unwrap();
}

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("chipload-place-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// The user's case: an empty 300 × 200 project with a drawn circle on the stock (shape 1),
/// then a DXF square of 50 mm drawn far away at (500, 400) imported into it (shape 2).
fn drawn_then_imported(name: &str) -> (Job, PathBuf, PathBuf) {
    let dir = temp_dir(name);
    let dxf = dir.join("part.dxf");
    square_dxf(&dxf, &[("A1", "CUT", 500.0, 400.0, 50.0)]);
    let mut job = empty_project();
    job.drawing.shapes.push(Shape {
        id: 1,
        layer: "DRAWN".into(),
        geometry: Geometry::Circle {
            center: [100.0, 100.0],
            radius: 20.0,
        },
    });
    job.drawing.next_shape_id = 2;
    let (job, _) = update_drawing(&job, &dxf, None).unwrap();
    (job, dxf, dir)
}

fn shape(job: &Job, id: u64) -> &Shape {
    job.drawing.shapes.iter().find(|s| s.id == id).unwrap()
}

/// Bounding box of one shape.
fn bbox(job: &Job, id: u64) -> ((f64, f64), (f64, f64)) {
    contours_from_shapes(std::slice::from_ref(shape(job, id)), None)
        .report
        .bbox
        .unwrap()
}

fn placement(job: &Job) -> Placement {
    job.drawing.source.as_ref().unwrap().placement
}

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

fn assert_box(job: &Job, id: u64, lo: (f64, f64), hi: (f64, f64)) {
    let ((x0, y0), (x1, y1)) = bbox(job, id);
    assert!(
        near(x0, lo.0) && near(y0, lo.1) && near(x1, hi.0) && near(y1, hi.1),
        "shape {id}: ({x0}, {y0})-({x1}, {y1}), expected {lo:?}-{hi:?}"
    );
}

/// Profile on `select` with `starts`, from the defaults of a new project's tool.
fn profile(select: serde_json::Value, starts: serde_json::Value) -> serde_json::Value {
    json!({
        "type": "profile", "name": "", "enabled": true, "select": select,
        "side": "outside", "direction": "climb", "tool": 1,
        "start_depth": 0.0, "end_depth": 3.0, "clearance": 0.0, "step_down": 3.0,
        "feed": 1000.0, "plunge_feed": 300.0, "tabs": null,
        "entry": {"type": "plunge"}, "starts": starts, "lead": null
    })
}

fn with_ops(job: &Job, ops: serde_json::Value) -> Job {
    let mut data = serde_json::to_value(job).unwrap();
    data["operations"] = ops;
    parse_job(data, "test").unwrap()
}

#[test]
fn a_first_import_is_where_the_dxf_has_it() {
    let (job, _, dir) = drawn_then_imported("first");
    assert_eq!(placement(&job), Placement::default());
    assert_box(&job, 2, (500.0, 400.0), (550.0, 450.0));
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn bottom_left_with_a_margin_puts_the_drawing_on_the_stock() {
    let (job, _, dir) = drawn_then_imported("bottom-left");
    let (placed, report) = place_drawing(&job, Move::BottomLeft { margin: 10.0 }).unwrap();
    assert_box(&placed, 2, (10.0, 10.0), (60.0, 60.0));
    assert_eq!(
        placement(&placed),
        Placement {
            dx: -490.0,
            dy: -390.0
        }
    );
    // The drawn circle stays, and the report says so.
    assert_eq!(shape(&placed, 1), shape(&job, 1));
    assert!(
        report.lines.iter().any(|l| l.contains("drawn in Chipload")),
        "{:?}",
        report.lines
    );
    // It was beyond the stock before and isn't now.
    assert!(!report.lines.iter().any(|l| l.contains("beyond the stock")));
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn centre_on_the_stock() {
    let (job, _, dir) = drawn_then_imported("centre");
    let (placed, _) = place_drawing(&job, Move::Center).unwrap();
    assert_box(&placed, 2, (125.0, 75.0), (175.0, 125.0));
    // A centred stock: its centre is the drawing origin.
    let mut centred = job.clone();
    centred.stock.origin = chipload_job::StockOrigin::Center;
    let (placed, _) = place_drawing(&centred, Move::Center).unwrap();
    assert_box(&placed, 2, (-25.0, -25.0), (25.0, 25.0));
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn by_adds_and_to_sets() {
    let (job, _, dir) = drawn_then_imported("by-to");
    let (once, _) = place_drawing(&job, Move::By(-400.0, -300.0)).unwrap();
    let (twice, report) = place_drawing(&once, Move::By(5.5, -2.25)).unwrap();
    assert_eq!(report.moved, (5.5, -2.25));
    assert_eq!(
        placement(&twice),
        Placement {
            dx: -394.5,
            dy: -302.25
        }
    );
    assert_box(&twice, 2, (105.5, 97.75), (155.5, 147.75));
    let (back, _) = place_drawing(&twice, Move::To(0.0, 0.0)).unwrap();
    assert_eq!(placement(&back), Placement::default());
    assert_box(&back, 2, (500.0, 400.0), (550.0, 450.0));
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn an_update_keeps_the_placement() {
    let (job, dxf, dir) = drawn_then_imported("update");
    // Two moves, whose sum may differ in the last bits from moving once.
    let (job, _) = place_drawing(&job, Move::By(-487.1, -391.3)).unwrap();
    let (job, _) = place_drawing(&job, Move::By(0.7, 0.9)).unwrap();
    let (same, report) = update_drawing(&job, &dxf, None).unwrap();
    assert_eq!(report.unchanged, [2], "{:?}", report.lines);
    assert!(report.changed.is_empty());
    assert_eq!(placement(&same), placement(&job));
    assert_box(&same, 2, (13.6, 9.6), (63.6, 59.6));
    assert!(
        report.lines.iter().any(|l| l.contains("Placed as before")),
        "{:?}",
        report.lines
    );
    // A new version of the DXF with one more square: both go to the placed position.
    square_dxf(
        &dxf,
        &[
            ("A1", "CUT", 500.0, 400.0, 50.0),
            ("A2", "CUT", 600.0, 400.0, 10.0),
        ],
    );
    let (grown, report) = update_drawing(&job, &dxf, None).unwrap();
    assert_eq!((report.unchanged, report.added), (vec![2], vec![3]));
    assert_box(&grown, 3, (113.6, 9.6), (123.6, 19.6));
    assert_eq!(shape(&grown, 1), shape(&job, 1));
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn start_points_on_the_imported_drawing_move_with_it() {
    let (job, _, dir) = drawn_then_imported("starts");
    let job = with_ops(
        &job,
        json!([
            profile(
                json!({"layer": null, "shapes": [2]}),
                json!([[525.0, 397.0]])
            ),
            profile(
                json!({"layer": "DRAWN", "shapes": null}),
                json!([[100.0, 77.0]])
            ),
            // Selects both: each point goes by its nearest shape.
            profile(
                json!({"layer": null, "shapes": [1, 2]}),
                json!([[553.0, 425.0], [123.0, 100.0]])
            ),
        ]),
    );
    let (placed, report) = place_drawing(&job, Move::BottomLeft { margin: 10.0 }).unwrap();
    let starts: Vec<Vec<[f64; 2]>> = placed
        .operations
        .iter()
        .map(|op| op.starts().to_vec())
        .collect();
    assert_eq!(
        starts,
        [
            vec![[35.0, 7.0]],
            vec![[100.0, 77.0]],
            vec![[63.0, 35.0], [123.0, 100.0]],
        ]
    );
    assert!(
        report.lines.iter().any(|l| l.contains("2 start point")),
        "{:?}",
        report.lines
    );
    // And the moved project still makes a program that passes the checker.
    let result = run_job(&placed).unwrap();
    assert!(!result.gcode.is_empty());
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_work_zero_at_a_point_stays_and_is_mentioned() {
    let (mut job, _, dir) = drawn_then_imported("work-zero");
    job.machine.work_zero = WorkZero::Point { x: 500.0, y: 400.0 };
    let (placed, report) = place_drawing(&job, Move::By(1.0, 1.0)).unwrap();
    assert_eq!(placed.machine.work_zero, job.machine.work_zero);
    assert!(
        report.lines.iter().any(|l| l.contains("work zero stays")),
        "{:?}",
        report.lines
    );
    std::fs::remove_dir_all(dir).unwrap();
}

fn job_error(
    result: Result<(Job, chipload_pipeline::placement::PlaceReport), PipelineError>,
) -> String {
    match result {
        Err(PipelineError::Job(text)) => text,
        other => panic!("expected a job error, got {other:?}"),
    }
}

#[test]
fn nothing_to_move_bad_values_and_no_move() {
    let empty = empty_project();
    assert!(job_error(place_drawing(&empty, Move::Center)).contains("no imported drawing"));
    let (job, _, dir) = drawn_then_imported("errors");
    assert!(job_error(place_drawing(&job, Move::By(f64::NAN, 0.0))).contains("can't move"));
    assert!(job_error(place_drawing(&job, Move::By(f64::INFINITY, 0.0))).contains("can't move"));
    assert!(job_error(place_drawing(&job, Move::BottomLeft { margin: -1.0 })).contains("margin"));
    let (same, report) = place_drawing(&job, Move::By(0.0, 0.0)).unwrap();
    assert_eq!(same, job);
    assert!(report.lines[0].contains("nothing moved"));
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_moved_drawing_cuts_the_same_program_moved() {
    let corpus = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/corpus");
    let job = chipload_job::load_job(&corpus.join("jobs/v7/simple_part_full.chipload")).unwrap();
    let (dx, dy) = (-20.0, 30.0);
    let (placed, _) = place_drawing(&job, Move::By(dx, dy)).unwrap();
    let before = run_job(&job).unwrap().program.moves;
    let after = run_job(&placed).unwrap().program.moves;
    assert_eq!(before.len(), after.len());
    for (a, b) in before.iter().zip(&after) {
        assert_eq!((a.kind, a.z, a.feed), (b.kind, b.z, b.feed));
        let shifted = |v: Option<f64>, d: f64| v.map(|v| v + d);
        for (x, y) in [(shifted(a.x, dx), b.x), (shifted(a.y, dy), b.y)] {
            match (x, y) {
                (Some(x), Some(y)) => assert!(near(x, y), "{x} vs {y}"),
                (x, y) => assert_eq!(x, y),
            }
        }
    }
}
