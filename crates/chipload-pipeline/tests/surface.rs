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
fn a_new_surface_goes_first_in_the_active_part_with_the_widest_endmill() {
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
    // In part B: before its outline.
    job.active_part = Some(job.parts[1].id);
    let index = add_surface(&mut job, Some(1), Some(0.3), None).unwrap();
    assert_eq!(index, 4);
    assert_eq!(job.operations[4].part(), job.parts[1].id);
    // Refused: a V-bit, a tool not in the project, and no endmill at all.
    let err = add_surface(&mut job, Some(2), None, None).unwrap_err();
    assert!(err.to_string().contains("needs an endmill"), "{err}");
    let err = add_surface(&mut job, Some(9), None, None).unwrap_err();
    assert!(err.to_string().contains("no tool 9"), "{err}");
    job.tools.retain(|t| t.kind != ToolType::Endmill);
    let err = add_surface(&mut job, None, None, None).unwrap_err();
    assert!(err.to_string().contains("no endmill"), "{err}");
}
