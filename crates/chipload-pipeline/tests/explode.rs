// SPDX-License-Identifier: GPL-3.0-or-later
//! Exploding a DXF into drawn shapes and joining close ends
//! (docs/specs/explode-and-join.md §7).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};

use chipload_job::{Geometry, Job, Selector, Shape, load_job};
use chipload_pipeline::dxfs::add_dxf;
use chipload_pipeline::explode::{explode_dxf, join_shapes};
use chipload_pipeline::run_job;

fn corpus(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/corpus")
        .join(rel)
}

/// Every corpus project, by its path relative to the corpus.
fn projects() -> Vec<String> {
    let mut out = Vec::new();
    let mut todo = vec![corpus("jobs")];
    while let Some(dir) = todo.pop() {
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                todo.push(path);
            } else if path.extension().is_some_and(|e| e == "chipload") {
                let rel = path.strip_prefix(corpus("")).unwrap();
                out.push(rel.to_string_lossy().replace('\\', "/"));
            }
        }
    }
    out.sort();
    out
}

fn gcode(job: &Job) -> Vec<String> {
    run_job(job)
        .unwrap()
        .files
        .into_iter()
        .map(|f| f.gcode)
        .collect()
}

/// The same G-code before and after exploding, unless a circle's start moved; with `join`,
/// also joined with that gap. An SVG's arcs are fitted, and come back from the exploded
/// polylines' bulges a few billionths of a mm off: a ring that is nearly a circle may then
/// start elsewhere on it, so a project with an SVG only has to still give checked G-code.
fn keeps_the_gcode(join: Option<f64>) {
    let mut exploded = 0;
    let mut differ = Vec::new();
    for rel in projects() {
        let job = load_job(&corpus(&rel)).unwrap();
        if job.drawing.dxfs.is_empty() {
            continue;
        }
        let Ok(before) = run_job(&job).map(|r| r.files) else {
            continue;
        };
        let mut after = job.clone();
        let mut restarted = job
            .drawing
            .dxfs
            .iter()
            .any(|d| d.format == chipload_job::DrawingFormat::Svg);
        for id in job.drawing.dxfs.iter().map(|d| d.id) {
            let (out, report) = explode_dxf(&after, id, join).unwrap();
            restarted |= report.lines.iter().any(|l| l.contains("right-hand point"));
            after = out;
        }
        assert!(after.drawing.dxfs.is_empty(), "{rel}");
        assert!(
            after.drawing.shapes.iter().all(|s| s.dxf().is_none()),
            "{rel}"
        );
        let before: Vec<String> = before.into_iter().map(|f| f.gcode).collect();
        let after = gcode(&after);
        // A circle that started elsewhere starts at angle 0 now, as drawn circles do: the
        // same cut from another point (the report says so).
        if after != before && !restarted {
            let first = before
                .iter()
                .zip(&after)
                .flat_map(|(b, a)| b.lines().zip(a.lines()))
                .find(|(b, a)| b != a);
            differ.push(format!("{rel}: {first:?}"));
        }
        exploded += 1;
    }
    assert!(exploded > 10, "only {exploded} projects with a DXF");
    assert!(differ.is_empty(), "G-code changed for {differ:?}");
}

#[test]
fn exploding_keeps_the_gcode() {
    keeps_the_gcode(None);
}

/// Joining ends that already meet makes one polyline of a contour drawn in pieces.
#[test]
fn exploding_and_joining_ends_that_meet_keeps_the_gcode() {
    keeps_the_gcode(Some(chipload_core::tolerances::CHAIN_TOL_MM));
}

#[test]
fn selectors_of_the_dxf_become_its_shapes() {
    let one = load_job(&corpus("jobs/v20/scaled_part.chipload")).unwrap();
    let (two, _) = add_dxf(&one, &corpus("dxf/simple_part.dxf"), None).unwrap();
    // Both operations are limited to DXF 1 now: layer HOLES and CUT_OUTSIDE of it.
    assert!(two.operations.iter().all(|op| op.select().dxf == Some(1)));
    let (out, report) = explode_dxf(&two, 1, None).unwrap();
    for op in &out.operations {
        let select = op.select();
        let ids = select.shapes.clone().unwrap();
        assert!(!ids.is_empty());
        assert_eq!(
            select,
            &Selector {
                shapes: Some(ids.clone()),
                ..Selector::default()
            }
        );
        // None of the second DXF's shapes, which have the same layers.
        for id in ids {
            let shape = out.drawing.shapes.iter().find(|s| s.id == id).unwrap();
            assert!(shape.is_drawn());
        }
    }
    assert_eq!(out.drawing.dxfs.len(), 1);
    assert_eq!(out.drawing.dxfs[0].id, 2);
    assert!(
        report.lines[0].starts_with("Exploded DXF 1 "),
        "{:?}",
        report.lines
    );
    assert!(
        report
            .lines
            .iter()
            .any(|l| l.starts_with("Operation 1") && l.contains("now selects shapes S")),
        "{:?}",
        report.lines
    );
    // The DXF's hidden keys go with it.
    let mut hidden = two.clone();
    hidden.drawing.hidden = vec!["dxf:1".into(), "dxf:1/layer:HOLES".into(), "dxf:2".into()];
    let (out, _) = explode_dxf(&hidden, 1, None).unwrap();
    assert_eq!(out.drawing.hidden, vec!["dxf:2".to_string()]);
}

#[test]
fn an_unknown_dxf_and_a_wrong_gap_are_refused() {
    let job = load_job(&corpus("jobs/v20/scaled_part.chipload")).unwrap();
    assert!(explode_dxf(&job, 9, None).is_err());
    let err = explode_dxf(&job, 1, Some(2.0)).unwrap_err();
    assert_eq!(
        err.to_string(),
        "The gap must be 0.001 mm to 1 mm, got 2 mm."
    );
    assert!(join_shapes(&job, &[], 0.0).is_err());
    let imported = job.drawing.shapes[0].id;
    assert!(join_shapes(&job, &[imported], 0.01).is_err());
}

fn polyline(id: u64, layer: &str, points: Vec<[f64; 2]>) -> Shape {
    Shape {
        id,
        layer: layer.into(),
        name: None,
        geometry: Geometry::straight_polyline(false, points),
    }
}

#[test]
fn joining_closes_a_square_drawn_in_four_lines_and_operations_follow() {
    let mut job = load_job(&corpus("jobs/v20/scaled_part.chipload")).unwrap();
    let first = job.drawing.next_shape_id;
    let ids: Vec<u64> = (first..first + 4).collect();
    job.drawing.shapes.extend([
        polyline(ids[0], "sq", vec![[0.0, 0.0], [10.0, 0.0]]),
        polyline(ids[1], "sq", vec![[10.0, 0.004], [10.0, 10.0]]),
        polyline(ids[2], "sq", vec![[10.0, 10.0], [0.0, 10.0]]),
        polyline(ids[3], "sq", vec![[0.0, 10.0], [0.0, 0.006]]),
    ]);
    job.drawing.next_shape_id += 4;
    job.operations[0].select_mut().unwrap().shapes = Some(vec![ids[2], ids[3]]);
    job.operations[0].select_mut().unwrap().layer = None;
    job.operations[0].select_mut().unwrap().dxf = None;

    let (unchanged, report) = join_shapes(&job, &[], 0.005).unwrap();
    assert_eq!(
        report.lines[0], "Joined 4 shapes into 1, 0 of them closed; widest gap closed 0.004 mm.",
        "{:?}",
        report.lines
    );
    assert!(report.lines.iter().any(|l| l == "2 ends stay open."));
    assert!(
        report
            .lines
            .iter()
            .any(|l| l.contains("the nearest other end is 0.006 mm away")),
        "{:?}",
        report.lines
    );
    assert_eq!(unchanged.drawing.shapes.len(), job.drawing.shapes.len() - 3);

    let (out, report) = join_shapes(&job, &ids, 0.01).unwrap();
    assert_eq!(
        report.lines[0],
        "Joined 4 shapes into 1, 1 of them closed; widest gap closed 0.006 mm."
    );
    assert_eq!(report.shapes, vec![ids[0]]);
    assert_eq!(report.removed, ids[1..].to_vec());
    let square = out.drawing.shapes.iter().find(|s| s.id == ids[0]).unwrap();
    assert!(
        matches!(&square.geometry, Geometry::Polyline { closed: true, points, .. } if points.len() == 4)
    );
    assert_eq!(out.operations[0].select().shapes, Some(vec![ids[0]]));
}

mod join_properties {
    use chipload_job::Geometry;
    use chipload_job::explode::{JoinPiece, join};
    use proptest::prelude::*;

    /// Open polylines with ends on a coarse grid, nudged by up to 0.02 mm, so that some
    /// meet within the gap, some branch and some stay apart.
    fn pieces() -> impl Strategy<Value = Vec<Geometry>> {
        let point = (0..4i32, 0..4i32, -0.02..0.02f64, -0.02..0.02f64)
            .prop_map(|(x, y, dx, dy)| [f64::from(x) * 10.0 + dx, f64::from(y) * 10.0 + dy]);
        let piece =
            (prop::collection::vec(point, 2..4), -1.0..1.0f64).prop_map(|(points, bulge)| {
                let n = points.len() - 1;
                Geometry::Polyline {
                    closed: false,
                    points,
                    bulges: (0..n).map(|i| if i == 0 { bulge } else { 0.0 }).collect(),
                }
            });
        prop::collection::vec(piece, 1..10).prop_map(|all| {
            all.into_iter()
                .filter(|g| chipload_job::shape_problems(g).is_empty())
                .collect()
        })
    }

    fn values(g: &Geometry) -> (&[[f64; 2]], usize) {
        match g {
            Geometry::Polyline { points, bulges, .. } => (points, bulges.len()),
            _ => panic!("{g:?}"),
        }
    }

    proptest! {
        /// Joining moves no point by more than half the gap and keeps every segment (a
        /// one-segment piece closed on itself is split in two), and the result is valid.
        #[test]
        fn joining_moves_points_by_at_most_half_the_gap(shapes in pieces(), gap in 0.001..0.05f64) {
            let given: Vec<JoinPiece> = shapes
                .iter()
                .enumerate()
                .map(|(i, g)| JoinPiece { id: i as u64 + 1, layer: "0", geometry: g })
                .collect();
            let outcome = join(&given, gap);
            for joined in &outcome.joined {
                prop_assert!(chipload_job::shape_problems(&joined.geometry).is_empty());
                let originals: Vec<[f64; 2]> = joined
                    .pieces
                    .iter()
                    .flat_map(|id| values(&shapes[*id as usize - 1]).0.to_vec())
                    .collect();
                let segments: usize = joined
                    .pieces
                    .iter()
                    .map(|id| values(&shapes[*id as usize - 1]).1)
                    .sum();
                let (points, count) = values(&joined.geometry);
                let split = joined.pieces.len() == 1 && segments == 1;
                prop_assert!(count == segments || (split && count == 2));
                for p in points {
                    let nearest = originals
                        .iter()
                        .map(|q| (p[0] - q[0]).hypot(p[1] - q[1]))
                        .fold(f64::INFINITY, f64::min);
                    prop_assert!(split || nearest <= gap / 2.0 + 1e-12, "{p:?} moved {nearest}");
                }
            }
            prop_assert_eq!(join(&given, gap), outcome);
        }
    }
}

#[test]
fn a_slot_whose_end_is_a_hair_over_a_half_circle_explodes_whole() {
    // The user's drawing (2026-10-25): a bent slot's end arc, 195.59° to 15.59°, was dropped.
    let shapes = chipload_dxf::shapes_from_dxf(&corpus("dxf/shared_edges.dxf"), None).unwrap();
    let job = chipload_pipeline::new_project(shapes);
    let closed = |job: &Job| {
        chipload_pipeline::load_drawing(job)
            .unwrap()
            .contours
            .iter()
            .filter(|c| c.closed)
            .count()
    };
    let before = closed(&job);
    let (out, report) = explode_dxf(&job, 1, None).unwrap();
    assert!(
        !report.lines.iter().any(|l| l.contains("is gone")),
        "{:?}",
        report.lines
    );
    assert_eq!(closed(&out), before);
}
