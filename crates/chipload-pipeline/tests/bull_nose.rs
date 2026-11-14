// SPDX-License-Identifier: GPL-3.0-or-later
//! End mills with a corner radius in 2.5D operations (docs/specs/corner-radius.md §4.2,
//! §4.3): an end mill's path, two warnings, and the true shape in the simulation.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;

use chipload_job::{Job, Operation, load_job};
use chipload_pipeline::simulate::simulate;
use chipload_pipeline::{Cancel, RunResult, job_problems, run_job};

/// A 12 mm pocket ("Rough", tool 1, a 12 mm end mill) and a 4 mm rest pocket after it.
fn sample() -> Job {
    load_job(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/corpus/jobs/v18/rest.chipload"),
    )
    .unwrap()
}

fn bull(mut job: Job, corner: f64) -> Job {
    job.tools[0].corner_radius = Some(corner);
    job
}

fn notes(result: &RunResult) -> Vec<String> {
    result.notes.iter().map(|n| n.text.clone()).collect()
}

fn removed(job: &Job, result: &RunResult) -> f64 {
    let sim = simulate(
        job,
        &result.program,
        &result.groups,
        Some(1.0),
        &Cancel::new(),
    )
    .unwrap();
    sim.heights.iter().map(|h| -f64::from(*h)).sum()
}

#[test]
fn a_bull_nose_pocket_cuts_an_end_mills_path_and_leaves_its_fillets() {
    let square = sample();
    let round = bull(sample(), 2.0);
    let (a, b) = (run_job(&square).unwrap(), run_job(&round).unwrap());
    assert_eq!(a.program.moves, b.program.moves);
    // Its stepover is within the 8 mm flat: no ridge warning.
    assert!(
        notes(&b).iter().all(|n| !n.contains("flat bottom")),
        "{:?}",
        notes(&b)
    );
    // The round corners leave material at the foot of the walls.
    let (cut_square, cut_round) = (removed(&square, &a), removed(&round, &b));
    assert!(cut_round < cut_square, "{cut_round} vs {cut_square}");
}

#[test]
fn a_stepover_wider_than_the_flat_warns_of_ridges() {
    let mut job = bull(sample(), 2.0);
    let Operation::Pocket(p) = &mut job.operations[0] else {
        panic!("a pocket");
    };
    // 0.9 × 12 = 10.8 mm against an 8 mm flat: 1.4 mm on each side of the band, on a 2 mm
    // corner, stands 2 − √(4 − 1.96) ≈ 0.572 mm.
    p.stepover = 0.9;
    let result = run_job(&job).unwrap();
    let found = notes(&result);
    assert!(
        found
            .iter()
            .any(|n| n.contains("flat bottom (8 mm)") && n.contains("0.572 mm")),
        "{found:?}"
    );
}

#[test]
fn a_through_cut_short_of_the_corner_warns() {
    let mut job = bull(sample(), 2.0);
    let thickness = job.stock().thickness;
    let Operation::Pocket(p) = &mut job.operations[0] else {
        panic!("a pocket");
    };
    p.through = Some(0.5);
    p.end_depth = thickness + 0.5;
    let result = run_job(&job).unwrap();
    assert!(
        notes(&result).iter().any(|n| n.contains("fillet")),
        "{:?}",
        notes(&result)
    );
    // Deep enough: no warning.
    let Operation::Pocket(p) = &mut job.operations[0] else {
        panic!("a pocket");
    };
    p.through = Some(2.0);
    p.end_depth = thickness + 2.0;
    job.stock_mut().max_overcut = 2.0;
    let result = run_job(&job).unwrap();
    assert!(
        notes(&result).iter().all(|n| !n.contains("fillet")),
        "{:?}",
        notes(&result)
    );
}

#[test]
fn a_corner_radius_is_checked() {
    let at = |job: &Job| {
        job_problems(job)
            .into_iter()
            .map(|p| p.path)
            .collect::<Vec<_>>()
    };
    assert!(at(&bull(sample(), 2.0)).is_empty());
    // Half the 12 mm diameter is a ball nose.
    assert_eq!(at(&bull(sample(), 6.0)), ["tools.0.corner_radius"]);
    assert_eq!(at(&bull(sample(), -1.0)), ["tools.0.corner_radius"]);
    // Only on an end mill.
    let mut drill = bull(sample(), 1.0);
    drill.tools[0].kind = chipload_job::ToolType::Drill;
    assert!(at(&drill).contains(&"tools.0.corner_radius".to_string()));
}
