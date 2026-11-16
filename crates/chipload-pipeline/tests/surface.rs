// SPDX-License-Identifier: GPL-3.0-or-later
//! Surfacing (docs/specs/surfacing.md): the whole stock, once, flat at the end depth.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;

use chipload_job::{Job, Operation, SurfaceOp, ToolType, load_job};
use chipload_pipeline::surface::{add_surface, surface_tool};
use chipload_pipeline::{Cancel, job_problems, run_job};

/// A surface of the whole 480 × 220 stock (25 mm endmill, 1 mm in two depths) first in part
/// 1, then two parts, part B with two copies.
fn sample() -> Job {
    load_job(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/corpus/jobs/v29/surface.chipload"),
    )
    .unwrap()
}

fn surface(job: &mut Job) -> &mut SurfaceOp {
    match &mut job.operations[0] {
        Operation::Surface(f) => f,
        _ => panic!("the sample starts with its surface"),
    }
}

/// The sample with only its surface cut.
fn surface_only() -> Job {
    let mut job = sample();
    for op in job.operations.iter_mut().skip(1) {
        op.set_enabled(false);
    }
    job
}

#[test]
fn the_whole_stock_comes_out_flat_at_the_end_depth() {
    let job = surface_only();
    let result = run_job(&job).unwrap();
    let sim = chipload_pipeline::simulate::simulate(
        &job,
        &result.program,
        &result.groups,
        Some(1.0),
        &Cancel::new(),
    )
    .unwrap();
    let floor = sim.top - 1.0;
    let mut high = 0;
    for j in 0..sim.rows {
        for i in 0..sim.columns {
            // Cells wholly on the stock (the last row and column may stick out).
            let (x, y) = (i as f64 + 1.0, j as f64 + 1.0);
            if x <= sim.size.0 && y <= sim.size.1 && sim.height(i, j) > floor + 1e-3 {
                high += 1;
            }
        }
    }
    assert_eq!(high, 0, "cells left above the end depth");
    // Nothing deeper either.
    let deepest = sim.numbers.deepest.unwrap();
    assert!((deepest - floor).abs() < 1e-3, "{deepest}");
    // Every cutting move is on the stock grown by the tool radius: no checker warning.
    assert!(
        !result
            .check_warnings
            .iter()
            .any(|w| w.message.contains("outside the stock")),
        "{:?}",
        result.check_warnings
    );
}

#[test]
fn the_stock_is_faced_once_whatever_the_copies() {
    // The surface in part B, which has two copies: it is still cut once, and its pass over
    // the whole stock doesn't count as the copies overlapping.
    let mut job = sample();
    let b = job.parts[1].id;
    job.operations[0].set_part(b);
    let surface = job.operations.remove(0);
    job.operations.insert(3, surface);
    let result = run_job(&job).unwrap();
    let spans: Vec<u32> = result
        .op_spans
        .iter()
        .filter(|s| s.operation == 3)
        .map(|s| s.copy)
        .collect();
    assert_eq!(spans, [1]);
    // The outline of part B, after it, is cut on both copies.
    let outline: Vec<u32> = result
        .op_spans
        .iter()
        .filter(|s| s.operation == 4)
        .map(|s| s.copy)
        .collect();
    assert_eq!(outline, [1, 2]);
}

#[test]
fn a_surface_after_other_cuts_of_its_part_is_warned_about() {
    let mut job = sample();
    let surface = job.operations.remove(0);
    job.operations.insert(2, surface);
    let result = run_job(&job).unwrap();
    let note = result
        .notes
        .iter()
        .find(|n| n.text.contains("faces the stock after"))
        .unwrap_or_else(|| panic!("{:?}", result.notes));
    assert!(
        note.text.contains("after operations 1 and 2 have cut it"),
        "{}",
        note.text
    );
    // First, as in the sample: no such note.
    let result = run_job(&sample()).unwrap();
    assert!(
        !result
            .notes
            .iter()
            .any(|n| n.text.contains("faces the stock"))
    );
}

#[test]
fn problems_name_their_field() {
    let has = |job: &Job, path: &str, text: &str| {
        let problems = job_problems(job);
        assert!(
            problems
                .iter()
                .any(|p| p.path == path && p.text.contains(text)),
            "{path}: {problems:?}"
        );
    };
    let mut job = sample();
    surface(&mut job).stepover = 0.95;
    has(&job, "operations.0.stepover", "must be from 0.1 to 0.9");
    let mut job = sample();
    surface(&mut job).end_depth = 12.5;
    has(
        &job,
        "operations.0.end_depth",
        "can't go below the stock bottom",
    );
    let mut job = sample();
    surface(&mut job).tool = 2;
    has(&job, "operations.0.tool", "a surface needs an endmill");
    // No shapes needed: the sample has none on its surface and no selector problem.
    let job = sample();
    assert!(
        !job_problems(&job)
            .iter()
            .any(|p| p.path.starts_with("operations.0.select")),
        "{:?}",
        job_problems(&job)
    );
}

#[test]
fn a_new_board_surface_goes_first_in_the_setup_with_the_widest_endmill() {
    let mut job = sample();
    job.operations.remove(0);
    job.active_part = Some(job.parts[0].id);
    assert_eq!(surface_tool(&job), Some(3));
    let index = add_surface(&mut job, None, None, None).unwrap();
    assert_eq!(index, 0);
    let Operation::Surface(f) = &job.operations[0] else {
        panic!()
    };
    assert_eq!(
        (f.tool, f.end_depth, f.step_down, f.stepover),
        (3, 0.5, 0.5, 0.75)
    );
    assert_eq!(f.feed, chipload_pipeline::chip_load::default_feed(25.0));
    run_job(&job).unwrap();
    // With part B active: still first in the setup, in its first part
    // (docs/specs/surface-levels.md §2).
    job.active_part = Some(job.parts[1].id);
    let index = add_surface(&mut job, Some(1), Some(0.3), None).unwrap();
    assert_eq!(index, 0);
    assert_eq!(job.operations[0].part(), job.parts[0].id);
    assert!(job.operations[0].is_board_surface());
    // Refused: a V-bit, a tool not in the project, and no endmill at all.
    let err = add_surface(&mut job, Some(2), None, None).unwrap_err();
    assert!(err.to_string().contains("needs an endmill"), "{err}");
    let err = add_surface(&mut job, Some(9), None, None).unwrap_err();
    assert!(err.to_string().contains("no tool 9"), "{err}");
    job.tools.retain(|t| t.kind != ToolType::Endmill);
    let err = add_surface(&mut job, None, None, None).unwrap_err();
    assert!(err.to_string().contains("no endmill"), "{err}");
}

// -- A part surface (docs/specs/surface-levels.md §3)

/// The sample with only a part surface of part B's outline (shape 8, a 6 mm endmill, 0.5 mm
/// in one pass, a 3 mm margin) cut, part B cut twice (its instance 2 at 0, 82).
fn part_surface_only() -> (Job, usize) {
    let mut job = sample();
    for op in job.operations.iter_mut() {
        op.set_enabled(false);
    }
    let index = job.insert_index(2);
    job.operations.insert(
        index,
        Operation::Surface(SurfaceOp {
            name: "Face B".into(),
            enabled: true,
            part: 2,
            select: chipload_job::Selector {
                shapes: Some(vec![8]),
                ..Default::default()
            },
            margin: 3.0,
            material: None,
            tool: 1,
            start_depth: 0.0,
            end_depth: 0.5,
            step_down: 0.5,
            stepover: 0.5,
            feed: 1500.0,
            plunge_feed: 500.0,
        }),
    );
    assert!(job.validate().is_empty(), "{:?}", job.validate());
    (job, index)
}

#[test]
fn a_part_surface_faces_its_shapes_under_each_instance_and_nothing_else() {
    let (job, index) = part_surface_only();
    // run_job refuses a program the independent checker doesn't pass.
    let result = run_job(&job).unwrap();
    let sim = chipload_pipeline::simulate::simulate(
        &job,
        &result.program,
        &result.groups,
        Some(1.0),
        &Cancel::new(),
    )
    .unwrap();
    let height = |x: f64, y: f64| {
        let i = ((x - sim.origin.0) / sim.cell) as usize;
        let j = ((y - sim.origin.1) / sim.cell) as usize;
        sim.height(i, j)
    };
    let ((x0, y0), (x1, y1)) = job.part_box(2).unwrap();
    let floor = sim.top - 0.5;
    let at = job.parts[1].instances[0].at;
    // Each instance's area: faced, flat at 0.5 mm, inside the outline and 1 mm past the
    // middle of each side (the margin is 3 mm); the outline's own corners are rounded.
    let (cx, cy) = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);
    let mut points = vec![
        (cx, y0 - 1.0),
        (cx, y1 + 1.0),
        (x0 - 1.0, cy),
        (x1 + 1.0, cy),
    ];
    let mut x = x0 + 12.0;
    while x <= x1 - 12.0 {
        let mut y = y0 + 12.0;
        while y <= y1 - 12.0 {
            points.push((x, y));
            y += 2.0;
        }
        x += 2.0;
    }
    for (dx, dy) in [(0.0, 0.0), (at[0], at[1])] {
        for &(x, y) in &points {
            let h = height(x + dx, y + dy);
            assert!((h - floor).abs() < 1e-3, "({x}, {y}) + ({dx}, {dy}): {h}");
        }
    }
    // Part 1's area, and the board beyond the margin: untouched.
    let ((a0, b0), (a1, b1)) = job.part_box(1).unwrap();
    for (x, y) in [
        (a0 + 5.0, b0 + 5.0),
        ((a0 + a1) / 2.0, (b0 + b1) / 2.0),
        (a1 - 5.0, b1 - 5.0),
    ] {
        assert_eq!(height(x, y), sim.top, "part 1 at ({x}, {y})");
    }
    assert_eq!(
        height(x1 + 6.0, (y0 + y1) / 2.0),
        sim.top,
        "past the margin"
    );
    // Never a plunge: every move going down also moves across (a helix or a ramp).
    let mut last: (Option<f64>, Option<f64>, Option<f64>) = (None, None, None);
    for span in result.op_spans.iter().filter(|s| s.operation == index) {
        for m in &result.program.moves[span.moves.clone()] {
            let now = (m.x.or(last.0), m.y.or(last.1), m.z.or(last.2));
            if m.kind == chipload_toolpath::model::MoveKind::Feed
                && let (Some(z0), Some(z1)) = (last.2, now.2)
                && z1 < z0 - 1e-9
                && z1 < sim.top
            {
                assert!(now.0 != last.0 || now.1 != last.1, "a plunge to Z{z1}");
            }
            last = now;
        }
    }
}

#[test]
fn a_part_surface_needs_an_endmill_and_faces_over_holes() {
    let (mut job, index) = part_surface_only();
    // Its tool must be an endmill, as a pocket's.
    if let Operation::Surface(f) = &mut job.operations[index] {
        f.tool = 2;
    }
    let err = run_job(&job).unwrap_err().to_string();
    assert!(err.contains("endmill"), "{err}");
    // A hole selected with its outline is faced over, not left standing as a pocket's
    // island would be.
    let (mut both, index) = part_surface_only();
    if let Operation::Surface(f) = &mut both.operations[index] {
        f.select.shapes = Some(vec![8, 9]);
    }
    let result = run_job(&both).unwrap();
    let sim = chipload_pipeline::simulate::simulate(
        &both,
        &result.program,
        &result.groups,
        Some(1.0),
        &Cancel::new(),
    )
    .unwrap();
    let ((hx0, hy0), (hx1, hy1)) = both.drawing.extents_of(|s| s.id == 9).unwrap();
    let (hx, hy) = ((hx0 + hx1) / 2.0, (hy0 + hy1) / 2.0);
    let (i, j) = ((hx - sim.origin.0) as usize, (hy - sim.origin.1) as usize);
    assert!(
        (sim.height(i, j) - (sim.top - 0.5)).abs() < 1e-3,
        "the hole at ({hx}, {hy})"
    );
}

#[test]
fn two_board_surfaces_in_one_setup_are_warned_about() {
    // docs/specs/surface-levels.md decision 4
    let mut job = sample();
    let again = job.operations[0].clone();
    job.operations.insert(1, again);
    let result = run_job(&job).unwrap();
    assert!(
        result.notes.iter().any(|n| n
            .text
            .contains("faces the whole board again after operation 1")),
        "{:?}",
        result.notes
    );
    // One board surface and a part surface: no such warning.
    let (part, _) = part_surface_only();
    let mut both = part;
    both.operations[0].set_enabled(true);
    let result = run_job(&both).unwrap();
    assert!(
        !result
            .notes
            .iter()
            .any(|n| n.text.contains("whole board again")),
        "{:?}",
        result.notes
    );
}
