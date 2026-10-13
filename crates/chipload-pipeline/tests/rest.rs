// SPDX-License-Identifier: GPL-3.0-or-later
//! "Add rest pocket" (docs/specs/rest-machining.md §10).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;

use chipload_job::library::Cutting;
use chipload_job::{Entry, Job, Lead, Operation, PocketOp, load_job};
use chipload_pipeline::{add_rest_pocket, rest_pocket_of, run_job};

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

#[test]
fn the_rest_pocket_copies_the_pocket_and_rests_after_its_tool() {
    let mut job = sample();
    job.operations.truncate(1);
    let mut rough = pocket(&job, 0);
    rough.starts = vec![[20.0, 20.0]];
    rough.lead = Some(Lead { radius: 3.0 });
    rough.clearance = 0.1;
    job.operations[0] = Operation::Pocket(rough.clone());
    let index = add_rest_pocket(&mut job, 1, 2, None).unwrap();
    assert_eq!(index, 1);
    let rest = pocket(&job, 1);
    assert_eq!(rest.name, "Rough (rest)");
    assert_eq!(rest.tool, 2);
    assert_eq!(rest.rest.unwrap().after_tool, 1);
    assert_eq!(rest.select, rough.select);
    assert_eq!(
        (rest.start_depth, rest.end_depth, rest.clearance),
        (rough.start_depth, rough.end_depth, rough.clearance)
    );
    assert_eq!(
        (rest.direction, rest.stepover),
        (rough.direction, rough.stepover)
    );
    assert_eq!((rest.feed, rest.step_down), (rough.feed, rough.step_down));
    assert!(rest.starts.is_empty() && rest.lead.is_none() && rest.corners.is_none());
    // It runs, without the "no earlier pocket" warning.
    let result = run_job(&job).unwrap();
    assert!(!result.notes.iter().any(|n| n.contains("no earlier pocket")));
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
    let rest = rest_pocket_of(&rough, &job.tools, 2, Some(&cutting)).unwrap();
    assert_eq!(
        (rest.feed, rest.plunge_feed, rest.step_down, rest.stepover),
        (900.0, 250.0, 1.0, 0.3)
    );
    // A helix sized for the large tool becomes the small tool's default.
    rough.entry = Entry::default_helix(12.0);
    let rest = rest_pocket_of(&rough, &job.tools, 2, None).unwrap();
    assert_eq!(rest.entry, Entry::default_helix(4.0));
    // A plunge with a tool that can't plunge goes down round a helix.
    let mut tools = job.tools.clone();
    tools[1].center_cutting = Some(false);
    rough.entry = Entry::Plunge {};
    let rest = rest_pocket_of(&rough, &tools, 2, None).unwrap();
    assert_eq!(rest.entry, Entry::default_helix(4.0));
    // An unnamed pocket.
    rough.name = String::new();
    assert_eq!(
        rest_pocket_of(&rough, &job.tools, 2, None).unwrap().name,
        "Rest"
    );
}

#[test]
fn refused_for_a_rest_pocket_a_larger_tool_or_not_a_pocket() {
    let mut job = sample();
    let error = |job: &mut Job, number: usize, tool: i64| {
        add_rest_pocket(job, number, tool, None)
            .unwrap_err()
            .to_string()
    };
    assert!(error(&mut job, 2, 1).contains("it is a rest pocket itself"));
    assert!(error(&mut job, 1, 1).contains("is not smaller than the pocket's tool"));
    assert!(error(&mut job, 1, 7).contains("the project has no tool 7"));
    assert!(error(&mut job, 3, 2).contains("there is no operation 3"));
    let mut drill = job.clone();
    drill.tools[1].kind = chipload_job::ToolType::Drill;
    assert!(error(&mut drill, 1, 2).contains("tool 2 is not an endmill"));
    assert_eq!(job.operations.len(), 2, "nothing added");
}
