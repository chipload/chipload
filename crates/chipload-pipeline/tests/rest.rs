// SPDX-License-Identifier: GPL-3.0-or-later
//! Rest passes under their pocket (docs/specs/rest-passes.md), and the standalone rest pocket
//! (docs/specs/rest-machining.md).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;

use chipload_job::library::Cutting;
use chipload_job::{Entry, Job, OpRef, Operation, PocketOp, load_job};
use chipload_pipeline::{
    add_rest_pass, job_problems, make_rest_pass, rest_pass_of, rest_pocket_parent, run_job,
};

/// A 12 mm pocket ("Rough", tool 1) and a 4 mm rest pocket after it ("Rest", tool 2).
fn sample() -> Job {
    load_job(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/corpus/jobs/v18/rest.chipload"),
    )
    .unwrap()
}

fn pocket(job: &Job, index: usize) -> PocketOp {
    match &job.operations[index] {
        Operation::Pocket(p) => p.clone(),
        _ => panic!("a pocket"),
    }
}

/// The sample with its rest pocket made a rest pass of the first.
fn with_pass() -> Job {
    let mut job = sample();
    assert_eq!(make_rest_pass(&mut job, 1), Some(OpRef::pass(0, 0)));
    job
}

#[test]
fn a_rest_pocket_made_a_pass_cuts_the_same_moves() {
    let before = run_job(&sample()).unwrap();
    let job = with_pass();
    assert_eq!(job.operations.len(), 1);
    assert_eq!(pocket(&job, 0).rest_passes.len(), 1);
    let after = run_job(&job).unwrap();
    assert_eq!(after.program.moves, before.program.moves);
    // Its moves are the pass's, and its time is its own.
    assert!(
        after
            .op_spans
            .iter()
            .any(|s| s.operation == 0 && s.pass == Some(0))
    );
    assert!(
        after
            .estimate
            .operations
            .iter()
            .any(|(r, _)| *r == OpRef::pass(0, 0))
    );
    // The sample writes one file with a tool change: the change names it as 1.1.
    assert_eq!(after.groups[1].operations, [OpRef::pass(0, 0)]);
    assert_eq!(after.files[0].operations, [OpRef::op(0), OpRef::pass(0, 0)]);
    assert!(
        after.files[0].gcode.contains("operations 1.1"),
        "{}",
        after.files[0].gcode
    );
    // One file per tool: the second file is the pass's.
    let mut files = job.clone();
    files.machine.tool_change = chipload_job::ToolChange::Files;
    let result = run_job(&files).unwrap();
    assert!(
        result.files[1].gcode.contains("(Operations 1.1)"),
        "{}",
        result.files[1].gcode
    );
}

#[test]
fn a_pass_follows_its_pocket() {
    let mut job = with_pass();
    let Operation::Pocket(p) = &mut job.operations[0] else {
        panic!()
    };
    p.end_depth = 4.0;
    p.clearance = 0.1;
    let Some(Operation::Pocket(pass)) = job.resolve(OpRef::pass(0, 0)) else {
        panic!()
    };
    assert_eq!((pass.end_depth, pass.clearance, pass.tool), (4.0, 0.1, 2));
    assert_eq!(pass.rest.unwrap().after_tool, 1);
    assert!(pass.starts.is_empty() && pass.lead.is_none() && pass.corners.is_none());
    // The pocket off: its pass isn't cut either.
    let Operation::Pocket(p) = &mut job.operations[0] else {
        panic!()
    };
    p.enabled = false;
    assert!(job.cut_list().is_empty());
}

#[test]
fn library_cutting_values_and_a_fitting_entry() {
    let job = sample();
    let mut rough = pocket(&job, 0);
    let cutting = Cutting {
        feed: 900.0,
        plunge_feed: 250.0,
        step_down: 1.0,
        stepover: 0.3,
    };
    let pass = rest_pass_of(&rough, &job.tools, 2, Some(&cutting)).unwrap();
    assert_eq!(
        (pass.feed, pass.plunge_feed, pass.step_down, pass.stepover),
        (900.0, 250.0, 1.0, 0.3)
    );
    // Without library values, the pocket's.
    let pass = rest_pass_of(&rough, &job.tools, 2, None).unwrap();
    assert_eq!((pass.feed, pass.step_down), (rough.feed, rough.step_down));
    // A helix sized for the large tool becomes the small tool's default.
    rough.entry = Entry::default_helix(12.0);
    let pass = rest_pass_of(&rough, &job.tools, 2, None).unwrap();
    assert_eq!(pass.entry, Entry::default_helix(4.0));
    // A plunge with a tool that can't plunge goes down round a helix.
    let mut tools = job.tools.clone();
    tools[1].center_cutting = Some(false);
    rough.entry = Entry::Plunge {};
    let pass = rest_pass_of(&rough, &tools, 2, None).unwrap();
    assert_eq!(pass.entry, Entry::default_helix(4.0));
}

#[test]
fn a_chain_of_passes_each_rests_after_the_one_before() {
    let mut job = sample();
    job.operations.truncate(1);
    let mut small = job.tools[1].clone();
    small.id = 3;
    small.diameter = 2.0;
    job.tools.push(small);
    assert_eq!(
        add_rest_pass(&mut job, 1, 2, None).unwrap(),
        OpRef::pass(0, 0)
    );
    assert_eq!(
        add_rest_pass(&mut job, 1, 3, None).unwrap(),
        OpRef::pass(0, 1)
    );
    let rest = |k| match job.resolve(OpRef::pass(0, k)) {
        Some(Operation::Pocket(p)) => p.rest.unwrap().after_tool,
        _ => panic!(),
    };
    assert_eq!((rest(0), rest(1)), (1, 2));
    assert_eq!(
        job.cut_list(),
        [OpRef::op(0), OpRef::pass(0, 0), OpRef::pass(0, 1)]
    );
    // A third with the 4 mm tool again is refused: not smaller than the 2 mm before it.
    let err = add_rest_pass(&mut job, 1, 2, None).unwrap_err().to_string();
    assert!(err.contains("is not smaller than tool 3"), "{err}");
    run_job(&job).unwrap();
}

#[test]
fn passes_wait_for_the_run_of_pockets_and_the_outline_stays_last() {
    // Pocket A (tool 1, a pass with tool 2), Pocket B (tool 1, a pass with tool 2), then the
    // outline with tool 1: A, B, A's pass, B's pass, outline (docs/specs/rest-passes.md §3).
    let mut job = sample();
    job.operations.truncate(1);
    let a = pocket(&job, 0);
    job.operations.push(Operation::Pocket(PocketOp {
        name: "B".into(),
        ..a.clone()
    }));
    // The outline: a profile from another corpus project, round this drawing's outer shape.
    let dogbones = load_job(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/corpus/jobs/v18/dogbones.chipload"),
    )
    .unwrap();
    let Some(Operation::Profile(mut outline)) = dogbones.operations.last().cloned() else {
        panic!("the dogbones sample ends with its outline")
    };
    outline.part = a.part;
    outline.tool = 1;
    outline.select = chipload_job::Selector {
        shapes: Some(vec![1]),
        ..Default::default()
    };
    outline.corners = None;
    outline.end_depth = 6.0;
    job.operations.push(Operation::Profile(outline));
    add_rest_pass(&mut job, 1, 2, None).unwrap();
    add_rest_pass(&mut job, 2, 2, None).unwrap();
    assert_eq!(
        job.cut_list(),
        [
            OpRef::op(0),
            OpRef::op(1),
            OpRef::pass(0, 0),
            OpRef::pass(1, 0),
            OpRef::op(2)
        ]
    );
    let result = run_job(&job).unwrap();
    // Tool 1 for both pockets, tool 2 for both passes, tool 1 again for the outline: two
    // tool changes, where cutting each pass after its pocket would take four.
    let tools: Vec<i64> = result.groups.iter().map(|g| g.tool.id).collect();
    assert_eq!(tools, [1, 2, 1]);
    assert_eq!(
        result.groups[1].operations,
        [OpRef::pass(0, 0), OpRef::pass(1, 0)]
    );
}

#[test]
fn problems_for_a_pass_name_its_field() {
    let mut job = with_pass();
    let Operation::Pocket(p) = &mut job.operations[0] else {
        panic!()
    };
    p.rest_passes[0].tool = 1;
    let problems = job_problems(&job);
    let found = problems
        .iter()
        .find(|p| p.path == "operations.0.rest_passes.0.tool")
        .unwrap_or_else(|| panic!("{problems:?}"));
    assert!(
        found.text.contains("Operation 1.1 (Rough, rest)"),
        "{}",
        found.text
    );
    assert!(
        found.text.contains("must be smaller than tool 1"),
        "{}",
        found.text
    );
    let Operation::Pocket(p) = &mut job.operations[0] else {
        panic!()
    };
    p.rest_passes[0].tool = 2;
    p.rest_passes[0].stepover = 1.5;
    assert!(
        job_problems(&job)
            .iter()
            .any(|p| p.path == "operations.0.rest_passes.0.stepover")
    );
}

#[test]
fn refused_for_dogbones_a_larger_tool_or_not_a_pocket() {
    let mut job = sample();
    let error = |job: &mut Job, number: usize, tool: i64| {
        add_rest_pass(job, number, tool, None)
            .unwrap_err()
            .to_string()
    };
    assert!(error(&mut job, 1, 1).contains("is not smaller than tool 1"));
    assert!(error(&mut job, 1, 7).contains("the project has no tool 7"));
    assert!(error(&mut job, 3, 2).contains("there is no operation 3"));
    let mut drill = job.clone();
    drill.tools[1].kind = chipload_job::ToolType::Drill;
    assert!(error(&mut drill, 1, 2).contains("tool 2 is not an endmill"));
    let mut dogbones = job.clone();
    let Operation::Pocket(p) = &mut dogbones.operations[0] else {
        panic!()
    };
    p.corners = Some(chipload_job::Corners::Dogbone {
        max_angle: chipload_job::DEFAULT_CORNER_ANGLE_RAD,
    });
    assert!(error(&mut dogbones, 1, 2).contains("dogbones"));
    assert!(pocket(&job, 0).rest_passes.is_empty(), "nothing added");
}

#[test]
fn only_a_rest_pocket_that_copies_an_earlier_pocket_can_be_made_a_pass() {
    let job = sample();
    assert_eq!(rest_pocket_parent(&job, 1), Some(0));
    assert_eq!(rest_pocket_parent(&job, 0), None, "not a rest pocket");
    let mut other = sample();
    let Operation::Pocket(p) = &mut other.operations[1] else {
        panic!()
    };
    p.end_depth = 3.0;
    assert_eq!(rest_pocket_parent(&other, 1), None, "another depth");
    assert_eq!(make_rest_pass(&mut other, 1), None);
    assert_eq!(other.operations.len(), 2);
}

#[test]
fn a_pocket_without_passes_is_saved_without_the_field() {
    let job = sample();
    let text = serde_json::to_string(&job).unwrap();
    assert!(!text.contains("rest_passes"));
    let text = serde_json::to_string(&with_pass()).unwrap();
    assert!(text.contains("\"rest_passes\""));
}
