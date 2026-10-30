// SPDX-License-Identifier: GPL-3.0-or-later
//! A V-carve's floor pass (docs/specs/vcarve-wide-strokes.md §4, §8).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;

use chipload_job::{Job, OpRef, Operation, Tool, ToolType, load_job};
use chipload_pipeline::{add_floor_pass, fitting_end_mills, job_problems, run_job};

/// A traced logo (a ring and a framed hole) V-carved 3.5 mm deep with a 60° bit: too wide,
/// so it gets walls; and a 3 mm end mill, tool 2.
fn sample() -> Job {
    let mut job = load_job(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/corpus/jobs/v19/image_vcarve.chipload"),
    )
    .unwrap();
    job.tools.push(Tool {
        id: 2,
        name: Some("3 mm flat".into()),
        kind: ToolType::Endmill,
        diameter: 3.0,
        angle: None,
        tip_diameter: None,
        ..job.tools[0].clone()
    });
    job
}

fn with_floor() -> Job {
    let mut job = sample();
    assert_eq!(add_floor_pass(&mut job, 1, 2, None), Ok(OpRef::pass(0, 0)));
    job
}

#[test]
fn the_floor_pass_is_cut_first_as_a_pocket_kept_off_the_walls() {
    let job = with_floor();
    assert_eq!(job.cut_list(), [OpRef::pass(0, 0), OpRef::op(0)]);
    assert!(job.is_floor_pass(OpRef::pass(0, 0)));
    assert_eq!(job.label(OpRef::pass(0, 0)), "Operation 1.1 (Logo, floor)");
    let Some(Operation::Pocket(pocket)) = job.resolve(OpRef::pass(0, 0)) else {
        panic!("a pocket");
    };
    // 3.5 mm deep with a 60° bit: the walls meet the floor 3.5 × tan 30° in.
    let inset = 3.5 * (30.0f64).to_radians().tan();
    assert!(
        (pocket.clearance + inset).abs() < 1e-9,
        "{}",
        pocket.clearance
    );
    assert_eq!((pocket.start_depth, pocket.end_depth), (0.0, 3.5));
    assert_eq!(pocket.tool, 2);
    // Its feeds are the V-carve's without library values.
    let Operation::Vcarve(vcarve) = &job.operations[0] else {
        panic!("a V-carve");
    };
    let floor = vcarve.floor.unwrap();
    assert_eq!(
        (floor.feed, floor.plunge_feed),
        (vcarve.feed, vcarve.plunge_feed)
    );
}

#[test]
fn a_run_with_a_floor_pass_is_checked_and_cleans_up_after_it() {
    let before = run_job(&sample()).unwrap();
    let after = run_job(&with_floor()).unwrap();
    // Two tools: the end mill first, then the V-bit.
    assert_eq!(after.groups[0].tool.id, 2);
    assert_eq!(after.groups[0].operations, [OpRef::pass(0, 0)]);
    assert_eq!(after.groups[1].operations, [OpRef::op(0)]);
    // No floor-left note any more; the framed hole's narrow floor gets a hint.
    let notes: Vec<String> = after.notes.iter().map(|n| n.text.clone()).collect();
    assert!(
        notes.iter().all(|n| !n.contains("Add a floor pass")),
        "{notes:?}"
    );
    assert!(
        notes
            .iter()
            .any(|n| n.contains("of the floor is cleared; one of at most")),
        "{notes:?}"
    );
    assert_eq!(after.floor_hints.len(), 1);
    assert!(after.floor_hints[0].1 < 3.0);
    // The V-bit cleans along the centre line where the end mill can't reach: more V moves.
    let vbit = |r: &chipload_pipeline::RunResult| {
        r.op_spans
            .iter()
            .filter(|s| s.pass.is_none())
            .map(|s| s.moves.len())
            .sum::<usize>()
    };
    assert!(vbit(&after) > vbit(&before));
}

#[test]
fn a_floor_pass_cuts_with_an_end_mill() {
    let mut job = sample();
    let refused = add_floor_pass(&mut job, 1, 1, None).unwrap_err();
    assert!(refused.to_string().contains("not an end mill"), "{refused}");
    // Set by hand anyway: a problem on its tool's field.
    let mut job = with_floor();
    if let Operation::Vcarve(v) = &mut job.operations[0] {
        v.floor.as_mut().unwrap().tool = 1;
    }
    let problems = job_problems(&job);
    assert!(
        problems
            .iter()
            .any(|p| p.path == "operations.0.floor.tool" && p.text.contains("end mill")),
        "{problems:?}"
    );
}

#[test]
fn a_vcarve_without_a_floor_pass_is_saved_without_the_field() {
    let text = serde_json::to_string(&sample()).unwrap();
    assert!(!text.contains("\"floor\""));
    let text = serde_json::to_string(&with_floor()).unwrap();
    assert!(text.contains("\"floor\""));
}

#[test]
fn library_end_mills_that_fit_nearest_first() {
    let tool = |name: &str, kind: &str, diameter: f64| {
        serde_json::from_value::<chipload_job::library::LibraryTool>(serde_json::json!({
            "name": name, "uid": name, "id": 1, "type": kind, "diameter": diameter,
        }))
        .unwrap()
    };
    let library = [
        tool("1 mm", "endmill", 1.0),
        tool("V", "vbit", 2.0),
        tool("3 mm", "endmill", 3.0),
        tool("2 mm", "endmill", 2.0),
    ];
    let names: Vec<&str> = fitting_end_mills(&library, 2.5)
        .iter()
        .map(|t| t.name.as_str())
        .collect();
    assert_eq!(names, ["2 mm", "1 mm"]);
}
