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
            scale: 1.0,
            quarter_turns: 0,
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
    let (twice, _) = place_drawing(&once, Move::By(5.5, -2.25)).unwrap();
    assert_eq!(
        placement(&twice),
        Placement {
            scale: 1.0,
            quarter_turns: 0,
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
fn arcs_rounded_differently_on_another_platform_are_unchanged() {
    // A project saved on Linux and updated on Windows: radius and sweep of a bulged
    // polyline's arcs come from trigonometry and may differ in the last bits.
    let mut job = simple_part();
    let mut nudged = 0;
    for shape in &mut job.drawing.shapes {
        if let Geometry::Path { segments, .. } = &mut shape.geometry {
            for segment in segments {
                if let chipload_job::PathSegment::Arc(arc) = segment {
                    arc.radius = f64::from_bits(arc.radius.to_bits() + 2);
                    arc.sweep = f64::from_bits(arc.sweep.to_bits() + 2);
                    nudged += 1;
                }
            }
        }
    }
    assert!(nudged > 0, "simple_part has arcs");
    let corpus = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/corpus");
    let (_, report) = update_drawing(&job, &corpus.join("dxf/simple_part.dxf"), None).unwrap();
    assert_eq!(report.unchanged.len(), 7, "{:?}", report.lines);
    assert!(report.changed.is_empty(), "{:?}", report.lines);
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
    assert!(!result.files[0].gcode.is_empty());
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

// -- turning in 90° steps (docs/specs/drawing-rotation.md)

/// simple_part (an outline with arcs, a slot, a circle and a rectangle hole), frozen at v7
/// with an inside and an outside profile.
fn simple_part() -> Job {
    let corpus = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/corpus");
    chipload_job::load_job(&corpus.join("jobs/v7/simple_part_full.chipload")).unwrap()
}

fn drawing_box(job: &Job) -> ((f64, f64), (f64, f64)) {
    chipload_pipeline::load_drawing(job)
        .unwrap()
        .report
        .bbox
        .unwrap()
}

/// Every segment of every shape equal within float noise (angles modulo a full turn).
fn assert_same_shapes(a: &Job, b: &Job) {
    use chipload_job::PathSegment;
    let close = |p: [f64; 2], q: [f64; 2]| near(p[0], q[0]) && near(p[1], q[1]);
    for (x, y) in a.drawing.shapes.iter().zip(&b.drawing.shapes) {
        for (s, t) in x.segments().iter().zip(y.segments()) {
            let same = match (s, &t) {
                (PathSegment::Line(s), PathSegment::Line(t)) => {
                    close(s.start, t.start) && close(s.end, t.end)
                }
                (PathSegment::Arc(s), PathSegment::Arc(t)) => {
                    let turn = (s.start_angle - t.start_angle).rem_euclid(std::f64::consts::TAU);
                    close(s.center, t.center)
                        && s.radius == t.radius
                        && s.sweep == t.sweep
                        && (turn < 1e-9 || std::f64::consts::TAU - turn < 1e-9)
                }
                _ => false,
            };
            assert!(same, "shape {}: {s:?} vs {t:?}", x.id);
        }
    }
}

#[test]
fn a_turn_keeps_the_drawing_centred_where_it_was() {
    let job = simple_part();
    // 50 .. 250 × 50 .. 170: centre (150, 110).
    let (turned, report) = place_drawing(&job, Move::Turn(1)).unwrap();
    let ((x0, y0), (x1, y1)) = drawing_box(&turned);
    assert!(near(x0, 90.0) && near(x1, 210.0) && near(y0, 10.0) && near(y1, 210.0));
    assert_eq!(placement(&turned).quarter_turns, 1);
    assert!(
        report.lines[0].starts_with("Turned the imported drawing 90° counter-clockwise; it is now turned 90°, moved X260.000 Y-40.000 from the DXF"),
        "{:?}",
        report.lines
    );
    // Points turn exactly: every line end is (-y, x) plus the same move.
    let (dx, dy) = (placement(&turned).dx, placement(&turned).dy);
    for (a, b) in job.drawing.shapes.iter().zip(&turned.drawing.shapes) {
        for (s, t) in a.segments().iter().zip(b.segments()) {
            if let (chipload_job::PathSegment::Line(s), chipload_job::PathSegment::Line(t)) =
                (s, &t)
            {
                assert_eq!(t.start, [-s.start[1] + dx, s.start[0] + dy]);
            }
        }
    }
    // Clockwise is three counter-clockwise turns; half a turn twice is none.
    let (cw, report) = place_drawing(&job, Move::Turn(-1)).unwrap();
    assert!(
        report.lines[0].contains("90° clockwise"),
        "{:?}",
        report.lines
    );
    assert_eq!(placement(&cw).quarter_turns, 3);
    let (half, _) = place_drawing(&job, Move::Turn(2)).unwrap();
    let (back, _) = place_drawing(&half, Move::Turn(2)).unwrap();
    assert_eq!(placement(&back).quarter_turns, 0);
    assert_same_shapes(&back, &job);
    let (same, report) = place_drawing(&job, Move::Turn(4)).unwrap();
    assert_eq!(same, job);
    assert!(report.lines[0].contains("nothing moved"));
}

#[test]
fn four_turns_bring_it_back() {
    let job = simple_part();
    let mut turned = job.clone();
    for _ in 0..4 {
        turned = place_drawing(&turned, Move::Turn(1)).unwrap().0;
    }
    let p = placement(&turned);
    assert_eq!(p.quarter_turns, 0);
    assert!(near(p.dx, 0.0) && near(p.dy, 0.0), "{p:?}");
    assert_same_shapes(&turned, &job);
}

#[test]
fn a_turned_part_keeps_its_holes_and_cuts_the_same_amount() {
    let job = simple_part();
    let (turned, _) = place_drawing(&job, Move::Turn(1)).unwrap();
    let depths = |job: &Job| {
        let mut d: Vec<Option<usize>> = chipload_pipeline::load_drawing(job)
            .unwrap()
            .contours
            .iter()
            .map(|c| c.nesting_depth)
            .collect();
        d.sort();
        d
    };
    assert_eq!(depths(&turned), depths(&job), "outer and holes stay");
    let before = run_job(&job).unwrap();
    let after = run_job(&turned).unwrap();
    // Each operation cuts the same path length, turned (the order of holes may differ).
    // Along the circle where a move has one, so a ring started at another point of an arc
    // (cut into other chords) measures the same.
    use chipload_toolpath::model::MoveKind;
    let along = |m: &chipload_toolpath::model::Move, a: (f64, f64), b: (f64, f64)| -> f64 {
        let chord = (b.0 - a.0).hypot(b.1 - a.1);
        let Some(c) = m.circle else { return chord };
        let turn =
            (b.1 - c.center.1).atan2(b.0 - c.center.0) - (a.1 - c.center.1).atan2(a.0 - c.center.0);
        match m.kind {
            MoveKind::ArcCcw => c.radius * turn.rem_euclid(std::f64::consts::TAU),
            MoveKind::ArcCw => c.radius * (-turn).rem_euclid(std::f64::consts::TAU),
            _ => 2.0 * c.radius * (chord / (2.0 * c.radius)).min(1.0).asin(),
        }
    };
    // Feed lengths per operation, following the position through moves that give only Z.
    let length = |r: &chipload_pipeline::RunResult, op: usize| -> f64 {
        let in_op: Vec<bool> = (0..r.program.moves.len())
            .map(|i| {
                r.op_spans
                    .iter()
                    .any(|s| s.operation == op && s.moves.contains(&i))
            })
            .collect();
        let (mut x, mut y, mut total) = (0.0_f64, 0.0_f64, 0.0);
        for (i, m) in r.program.moves.iter().enumerate() {
            let (nx, ny) = (m.x.unwrap_or(x), m.y.unwrap_or(y));
            if in_op[i] && m.kind != MoveKind::Rapid {
                total += along(m, (x, y), (nx, ny));
            }
            (x, y) = (nx, ny);
        }
        total
    };
    for op in 0..job.operations.len() {
        let (a, b) = (length(&before, op), length(&after, op));
        assert!((a - b).abs() < 1e-6, "operation {op}: {a} vs {b}");
    }
}

#[test]
fn start_points_turn_with_it_and_updates_keep_the_turn() {
    let (job, dxf, dir) = drawn_then_imported("turn");
    // The square 500 .. 550 × 400 .. 450, centre (525, 425); a start on its bottom edge.
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
        ]),
    );
    let (turned, _) = place_drawing(&job, Move::Turn(1)).unwrap();
    // Bottom edge turns to the right edge: (525, 397) -> (553, 425).
    assert_eq!(turned.operations[0].starts(), [[553.0, 425.0]]);
    assert_eq!(turned.operations[1].starts(), [[100.0, 77.0]]);
    assert_eq!(shape(&turned, 1), shape(&job, 1));
    let (turned, _) = place_drawing(&turned, Move::BottomLeft { margin: 10.0 }).unwrap();
    let (same, report) = update_drawing(&turned, &dxf, None).unwrap();
    assert_eq!(report.unchanged, [2], "{:?}", report.lines);
    assert_eq!(placement(&same), placement(&turned));
    assert_box(&same, 2, (10.0, 10.0), (60.0, 60.0));
    assert!(
        report.lines.iter().any(|l| l.contains("turned 90°")),
        "{:?}",
        report.lines
    );
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn reset_undoes_turns_and_moves() {
    let job = simple_part();
    let (moved, _) = place_drawing(&job, Move::Turn(-1)).unwrap();
    let (moved, _) = place_drawing(&moved, Move::BottomLeft { margin: 7.5 }).unwrap();
    let (moved, _) = place_drawing(&moved, Move::Turn(2)).unwrap();
    let (back, report) = place_drawing(&moved, Move::Reset).unwrap();
    assert!(
        report.lines[0].starts_with("Put the imported drawing back"),
        "{:?}",
        report.lines
    );
    let p = placement(&back);
    assert_eq!(p.quarter_turns, 0);
    assert!(near(p.dx, 0.0) && near(p.dy, 0.0), "{p:?}");
    assert_same_shapes(&back, &job);
}

// -- scaling (docs/specs/drawing-scale.md)

/// Radii of every arc, in shape order.
fn radii(job: &Job) -> Vec<f64> {
    job.drawing
        .shapes
        .iter()
        .flat_map(|s| s.segments())
        .filter_map(|s| match s {
            chipload_job::PathSegment::Arc(a) => Some(a.radius),
            chipload_job::PathSegment::Line(_) => None,
        })
        .collect()
}

fn nesting(job: &Job) -> Vec<Option<usize>> {
    let mut d: Vec<Option<usize>> = chipload_pipeline::load_drawing(job)
        .unwrap()
        .contours
        .iter()
        .map(|c| c.nesting_depth)
        .collect();
    d.sort();
    d
}

#[test]
fn scaling_grows_the_drawing_about_its_centre() {
    let job = simple_part();
    // 50 .. 250 × 50 .. 170: centre (150, 110), 200 × 120.
    let (scaled, report) = place_drawing(&job, Move::Scale(2.0)).unwrap();
    let ((x0, y0), (x1, y1)) = drawing_box(&scaled);
    assert!(
        near(x0, -50.0) && near(x1, 350.0) && near(y0, -10.0) && near(y1, 230.0),
        "({x0}, {y0})-({x1}, {y1})"
    );
    assert_eq!(
        placement(&scaled),
        Placement {
            scale: 2.0,
            quarter_turns: 0,
            dx: -150.0,
            dy: -110.0
        }
    );
    assert_eq!(
        report.lines[0],
        "Scaled the imported drawing ×2 (now 400.000 × 240.000 mm); it is now scaled ×2, \
         moved X-150.000 Y-110.000 from the DXF."
    );
    assert!(
        report
            .lines
            .iter()
            .any(|l| l.contains("extends beyond the stock")),
        "{:?}",
        report.lines
    );
    // Radii double, angles stay, outer and holes stay.
    let doubled: Vec<f64> = radii(&job).iter().map(|r| r * 2.0).collect();
    assert_eq!(radii(&scaled), doubled);
    for (a, b) in job.drawing.shapes.iter().zip(&scaled.drawing.shapes) {
        for (s, t) in a.segments().iter().zip(b.segments()) {
            if let (chipload_job::PathSegment::Arc(s), chipload_job::PathSegment::Arc(t)) = (s, &t)
            {
                assert_eq!((s.start_angle, s.sweep), (t.start_angle, t.sweep));
            }
        }
    }
    assert_eq!(nesting(&scaled), nesting(&job));
    // Half of that is back where it was: powers of two scale exactly.
    let (back, _) = place_drawing(&scaled, Move::Scale(0.5)).unwrap();
    assert_eq!(placement(&back), Placement::default());
    assert_same_shapes(&back, &job);
    // No spline in this part, no units-like factor: no notes about them.
    assert!(
        !report
            .lines
            .iter()
            .any(|l| l.contains("splines") || l.contains("units")),
        "{:?}",
        report.lines
    );
}

#[test]
fn to_a_width_or_a_height() {
    let job = simple_part();
    let (wide, report) = place_drawing(&job, Move::Width(100.0)).unwrap();
    let ((x0, y0), (x1, y1)) = drawing_box(&wide);
    assert!(near(x1 - x0, 100.0) && near(y1 - y0, 60.0));
    assert!(near((x0 + x1) / 2.0, 150.0) && near((y0 + y1) / 2.0, 110.0));
    assert!(report.lines[0].contains("×0.5 (now 100.000 × 60.000 mm)"));
    let (high, _) = place_drawing(&wide, Move::Height(36.0)).unwrap();
    let ((x0, y0), (x1, y1)) = drawing_box(&high);
    assert!(near(x1 - x0, 60.0) && near(y1 - y0, 36.0));
    assert!(near(placement(&high).scale, 0.3));
    assert!(
        chipload_pipeline::placement::placement_text(placement(&high)).starts_with("scaled ×0.3,")
    );
}

#[test]
fn bad_scales_are_refused() {
    let job = simple_part();
    for how in [
        Move::Scale(0.0),
        Move::Scale(-1.0),
        Move::Scale(f64::NAN),
        Move::Scale(f64::INFINITY),
    ] {
        assert!(
            job_error(place_drawing(&job, how)).contains("greater than 0"),
            "{how:?}"
        );
    }
    assert!(job_error(place_drawing(&job, Move::Width(0.0))).contains("width must be"));
    assert!(job_error(place_drawing(&job, Move::Height(-3.0))).contains("height must be"));
    assert!(
        job_error(place_drawing(&job, Move::Scale(2000.0)))
            .contains("×2000 from the DXF; it must stay between ×0.001 and ×1000")
    );
    // The range is for the total: two steps can't get round it.
    let (small, _) = place_drawing(&job, Move::Scale(0.01)).unwrap();
    assert!(job_error(place_drawing(&small, Move::Scale(0.01))).contains("×0.0001"));
    let (same, report) = place_drawing(&job, Move::Scale(1.0)).unwrap();
    assert_eq!(same, job);
    assert!(report.lines[0].contains("nothing moved"));
    // A drawing with no height can't be scaled to a height.
    let dir = temp_dir("flat");
    let dxf = dir.join("line.dxf");
    std::fs::write(
        &dxf,
        "0\nSECTION\n2\nHEADER\n9\n$INSUNITS\n70\n4\n0\nENDSEC\n0\nSECTION\n2\nENTITIES\n\
         0\nLINE\n5\nA1\n8\nCUT\n10\n10\n20\n10\n11\n60\n21\n10\n0\nENDSEC\n0\nEOF\n",
    )
    .unwrap();
    let (flat, _) = update_drawing(&empty_project(), &dxf, None).unwrap();
    assert!(job_error(place_drawing(&flat, Move::Height(5.0))).contains("no height"));
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn reset_puts_the_scale_back_to_exactly_one() {
    let job = simple_part();
    let (moved, _) = place_drawing(&job, Move::Scale(3.0)).unwrap();
    let (moved, _) = place_drawing(&moved, Move::Turn(1)).unwrap();
    let (moved, _) = place_drawing(&moved, Move::BottomLeft { margin: 7.5 }).unwrap();
    let (back, report) = place_drawing(&moved, Move::Reset).unwrap();
    assert_eq!(placement(&back), Placement::default());
    assert!(
        report.lines[0].ends_with("it is now as in the DXF."),
        "{:?}",
        report.lines
    );
    let ((x0, y0), (x1, y1)) = drawing_box(&back);
    assert!(
        (x0 - 50.0).abs() < 1e-9 && (x1 - 250.0).abs() < 1e-9,
        "({x0}, {y0})-({x1}, {y1})"
    );
    assert!((y0 - 50.0).abs() < 1e-9 && (y1 - 170.0).abs() < 1e-9);
}

#[test]
fn turning_and_scaling_about_the_centre_commute() {
    let job = simple_part();
    let (a, _) = place_drawing(&job, Move::Turn(1)).unwrap();
    let (a, _) = place_drawing(&a, Move::Scale(1.5)).unwrap();
    let (b, _) = place_drawing(&job, Move::Scale(1.5)).unwrap();
    let (b, _) = place_drawing(&b, Move::Turn(1)).unwrap();
    let (pa, pb) = (placement(&a), placement(&b));
    assert_eq!((pa.scale, pa.quarter_turns), (pb.scale, pb.quarter_turns));
    assert!(near(pa.dx, pb.dx) && near(pa.dy, pb.dy), "{pa:?} vs {pb:?}");
    assert_same_shapes(&a, &b);
}

#[test]
fn a_scaled_part_cuts_like_the_part_drawn_that_size() {
    let dir = temp_dir("scaled-cut");
    let small = dir.join("small.dxf");
    let large = dir.join("large.dxf");
    // A 20 mm square round a 10 mm one, centre (110, 60); ×2 is 40 round 20.
    square_dxf(
        &small,
        &[
            ("A1", "CUT", 100.0, 50.0, 20.0),
            ("A2", "CUT", 105.0, 55.0, 10.0),
        ],
    );
    square_dxf(
        &large,
        &[
            ("A1", "CUT", 90.0, 40.0, 40.0),
            ("A2", "CUT", 100.0, 50.0, 20.0),
        ],
    );
    let ops = json!([profile(json!({"layer": "CUT", "shapes": null}), json!([]))]);
    let (job, _) = update_drawing(&empty_project(), &small, None).unwrap();
    let (scaled, _) = place_drawing(&with_ops(&job, ops.clone()), Move::Scale(2.0)).unwrap();
    let (drawn, _) = update_drawing(&empty_project(), &large, None).unwrap();
    let drawn = with_ops(&drawn, ops);
    let a = run_job(&scaled).unwrap().program.moves;
    let b = run_job(&drawn).unwrap().program.moves;
    assert_eq!(a.len(), b.len());
    for (m, n) in a.iter().zip(&b) {
        assert_eq!((m.kind, m.z, m.feed), (n.kind, n.z, n.feed));
        for (x, y) in [(m.x, n.x), (m.y, n.y)] {
            match (x, y) {
                (Some(x), Some(y)) => assert!(near(x, y), "{x} vs {y}"),
                (x, y) => assert_eq!(x, y),
            }
        }
    }
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn start_points_scale_with_it_and_updates_keep_the_scale() {
    let (job, dxf, dir) = drawn_then_imported("scale");
    // The square 500 .. 550 × 400 .. 450, centre (525, 425); a start 3 mm below it.
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
        ]),
    );
    let (scaled, report) = place_drawing(&job, Move::Scale(1.5)).unwrap();
    assert_eq!(scaled.operations[0].starts(), [[525.0, 383.0]]);
    assert_eq!(scaled.operations[1].starts(), [[100.0, 77.0]]);
    assert_eq!(shape(&scaled, 1), shape(&job, 1));
    assert!(
        report
            .lines
            .iter()
            .any(|l| l.contains("drawn in Chipload stay"))
    );
    let (moved, _) = place_drawing(&scaled, Move::Turn(1)).unwrap();
    let (moved, _) = place_drawing(&moved, Move::BottomLeft { margin: 10.0 }).unwrap();
    let (same, report) = update_drawing(&moved, &dxf, None).unwrap();
    assert_eq!(report.unchanged, [2], "{:?}", report.lines);
    assert_eq!(placement(&same), placement(&moved));
    assert_box(&same, 2, (10.0, 10.0), (85.0, 85.0));
    assert!(
        report
            .lines
            .iter()
            .any(|l| l.contains("scaled ×1.5, turned 90°")),
        "{:?}",
        report.lines
    );
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_factor_like_inches_points_at_the_units() {
    let (job, _, dir) = drawn_then_imported("inch");
    for factor in [25.4, 1.0 / 25.4] {
        let (_, report) = place_drawing(&job, Move::Scale(factor)).unwrap();
        assert!(
            report
                .lines
                .iter()
                .any(|l| l.contains("set them on update instead (--units)")),
            "{factor}: {:?}",
            report.lines
        );
    }
    let (_, report) = place_drawing(&job, Move::Scale(25.0)).unwrap();
    assert!(!report.lines.iter().any(|l| l.contains("--units")));
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_gap_that_opens_when_scaled_up_is_reported() {
    let dir = temp_dir("gap");
    let dxf = dir.join("gap.dxf");
    // An open polyline whose ends are 0.0008 mm apart: joined at ×1 (0.001 mm), open at ×10.
    let mut text = String::from("0\nSECTION\n2\nHEADER\n9\n$INSUNITS\n70\n4\n0\nENDSEC\n");
    text += "0\nSECTION\n2\nENTITIES\n0\nLWPOLYLINE\n5\nA1\n8\nCUT\n90\n5\n70\n0\n";
    for (x, y) in [
        (0.0, 0.0008),
        (0.0, 10.0),
        (10.0, 10.0),
        (10.0, 0.0),
        (0.0, 0.0),
    ] {
        text += &format!("10\n{x}\n20\n{y}\n");
    }
    text += "0\nENDSEC\n0\nEOF\n";
    std::fs::write(&dxf, text).unwrap();
    let (job, _) = update_drawing(&empty_project(), &dxf, None).unwrap();
    let (_, report) = place_drawing(&job, Move::Scale(10.0)).unwrap();
    assert!(
        report
            .lines
            .iter()
            .any(|l| l.starts_with("1 closed contour(s) before, 0 now")),
        "{:?}",
        report.lines
    );
    let (_, report) = place_drawing(&job, Move::Scale(1.2)).unwrap();
    assert!(!report.lines.iter().any(|l| l.contains("closed contour")));
    std::fs::remove_dir_all(dir).unwrap();
}

/// A quadratic B-spline from (0, 0) over (50, 100) to (100, 0) (a parabola, mm), as
/// control points and knots.
fn spline_dxf(path: &Path) {
    let mut text = String::from("0\nSECTION\n2\nHEADER\n9\n$INSUNITS\n70\n4\n0\nENDSEC\n");
    text += "0\nSECTION\n2\nENTITIES\n0\nSPLINE\n5\nB1\n8\nCUT\n100\nAcDbEntity\n100\nAcDbSpline\n";
    text += "70\n8\n71\n2\n72\n6\n73\n3\n74\n0\n";
    for k in [0.0, 0.0, 0.0, 1.0, 1.0, 1.0] {
        text += &format!("40\n{k}\n");
    }
    for (x, y) in [(0.0, 0.0), (50.0, 100.0), (100.0, 0.0)] {
        text += &format!("10\n{x}\n20\n{y}\n30\n0.0\n");
    }
    text += "0\nENDSEC\n0\nEOF\n";
    std::fs::write(path, text).unwrap();
}

/// How far `p` (in DXF mm) is from the parabola of `spline_dxf`, sampled finely.
fn off_the_spline(p: [f64; 2]) -> f64 {
    let at = |t: f64| [100.0 * t, 200.0 * t * (1.0 - t)];
    let n = 20_000;
    (0..n)
        .map(|i| {
            let (a, b) = (at(i as f64 / n as f64), at((i + 1) as f64 / n as f64));
            let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
            let t =
                (((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / (dx * dx + dy * dy)).clamp(0.0, 1.0);
            (p[0] - a[0] - t * dx).hypot(p[1] - a[1] - t * dy)
        })
        .fold(f64::INFINITY, f64::min)
}

/// The largest distance, in placed mm, of the spline shape's vertices and chord midpoints
/// from the scaled curve.
fn spline_error(job: &Job) -> f64 {
    let p = placement(job);
    assert_eq!(p.quarter_turns, 0);
    let to_dxf = |q: [f64; 2]| [(q[0] - p.dx) / p.scale, (q[1] - p.dy) / p.scale];
    let mut worst: f64 = 0.0;
    for segment in job.drawing.shapes[0].segments() {
        let chipload_job::PathSegment::Line(l) = segment else {
            panic!("a spline is approximated by lines");
        };
        let mid = [(l.start[0] + l.end[0]) / 2.0, (l.start[1] + l.end[1]) / 2.0];
        for q in [l.start, mid, l.end] {
            worst = worst.max(off_the_spline(to_dxf(q)) * p.scale);
        }
    }
    worst
}

#[test]
fn splines_scaled_up_are_noted_and_updates_approximate_them_at_the_new_size() {
    let dir = temp_dir("spline");
    let dxf = dir.join("spline.dxf");
    spline_dxf(&dxf);
    let (job, _) = update_drawing(&empty_project(), &dxf, None).unwrap();
    assert_eq!(job.drawing.shapes[0].dxf_type(), Some("SPLINE"));
    assert!(spline_error(&job) <= 0.0101, "{}", spline_error(&job));
    let (scaled, report) = place_drawing(&job, Move::Scale(10.0)).unwrap();
    assert!(
        report.lines.iter().any(|l| l
            == "1 shape(s) from splines or ellipses were approximated within 0.010 mm at the \
                size before; scaled ×10, they may be up to 0.100 mm off their curves. Update \
                from the DXF to approximate them again at this size."),
        "{:?}",
        report.lines
    );
    assert!(spline_error(&scaled) > 0.02, "{}", spline_error(&scaled));
    let (updated, update) = update_drawing(&scaled, &dxf, None).unwrap();
    assert_eq!(placement(&updated), placement(&scaled));
    let error = spline_error(&updated);
    assert!(error <= 0.0101, "{error}");
    assert!(updated.drawing.shapes[0].segments().len() > scaled.drawing.shapes[0].segments().len());
    // The shape is different now (finer): an update reports it changed, not unchanged.
    assert!(update.unchanged.is_empty(), "{:?}", update.lines);
    // Scaling down needs no note.
    let (_, report) = place_drawing(&job, Move::Scale(0.5)).unwrap();
    assert!(!report.lines.iter().any(|l| l.contains("splines")));
    std::fs::remove_dir_all(dir).unwrap();
}

mod steps {
    use super::*;
    use proptest::prelude::*;

    fn a_move() -> impl Strategy<Value = Move> {
        prop_oneof![
            (0.2_f64..5.0).prop_map(Move::Scale),
            (-3_i32..=3).prop_map(Move::Turn),
            ((-200.0_f64..200.0), (-200.0_f64..200.0)).prop_map(|(x, y)| Move::By(x, y)),
            (20.0_f64..400.0).prop_map(Move::Width),
            Just(Move::Center),
        ]
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(48))]

        /// Placing step by step gives what placing the DXF once with the final placement
        /// gives: an update reports every shape unchanged.
        #[test]
        fn step_by_step_is_the_same_as_once(moves in prop::collection::vec(a_move(), 1..6)) {
            let corpus = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/corpus");
            let mut job = simple_part();
            for how in moves {
                match place_drawing(&job, how) {
                    Ok((next, _)) => job = next,
                    // Out of the scale range: that step is refused and nothing changes.
                    Err(PipelineError::Job(m)) if m.contains("must stay between") => {}
                    Err(err) => panic!("{how:?}: {err}"),
                }
            }
            let (again, report) =
                update_drawing(&job, &corpus.join("dxf/simple_part.dxf"), None).unwrap();
            prop_assert_eq!(placement(&again), placement(&job));
            let imported = job.drawing.shapes.iter().filter(|s| !s.is_drawn()).count();
            prop_assert_eq!(report.unchanged.len(), imported, "{:?}", report.lines);
        }
    }
}
