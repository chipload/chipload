// SPDX-License-Identifier: GPL-3.0-or-later
//! Groups of drawn shapes and texts (docs/specs/shape-groups.md).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;

use chipload_job::{Geometry, Job, Selector, load_job};
use chipload_pipeline::fonts::BuiltinOnly;
use chipload_pipeline::groups::{
    Members, add_to_group, delete_group, group, group_box, remove_from_group, rename_group,
    transform_group, ungroup,
};
use chipload_pipeline::run_job;
use chipload_pipeline::shapes::{add_shape, delete_shape};

/// The golden sign: text 1 ("SIGN", S1-S4), the frame S5 on layer frame, its border S6.
/// Operations: V-carve the text layer, follow the border, profile the frame.
fn sign() -> Job {
    load_job(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/corpus/jobs/v35/sign_frame.chipload"),
    )
    .unwrap()
}

fn members(job: &Job, shapes: &[u64], texts: &[u64]) -> Members {
    Members::of(job, shapes, texts).unwrap()
}

fn gcode(job: &Job) -> String {
    run_job(job)
        .unwrap()
        .files
        .iter()
        .map(|f| f.gcode.clone())
        .collect()
}

#[test]
fn a_group_is_made_named_and_round_trips_through_the_file() {
    let job = sign();
    // A letter stands for its whole text.
    let m = members(&job, &[5, 6, 2], &[]);
    assert_eq!((m.shapes.clone(), m.texts.clone()), (vec![5, 6], vec![1]));
    let (job, id) = group(&job, &m, None).unwrap();
    assert_eq!(id, 1);
    let g = &job.drawing.groups[0];
    assert_eq!(
        (g.name.as_str(), &g.shapes, &g.texts),
        ("Group 1", &vec![5, 6], &vec![1])
    );
    assert_eq!(job.drawing.group_shapes(1), [1, 2, 3, 4, 5, 6]);
    assert_eq!(job.drawing.group_of(3), Some(1));
    let job = rename_group(&job, 1, "Sign").unwrap();
    let text = chipload_job::dump_job(&job).unwrap();
    let back = chipload_job::parse_job(serde_json::from_str(&text).unwrap(), "p").unwrap();
    assert_eq!(back.drawing.groups, job.drawing.groups);
    // Without groups, nothing is written.
    let plain = chipload_job::dump_job(&sign()).unwrap();
    assert!(!plain.contains("\"groups\""));
}

#[test]
fn grouping_cuts_nothing_differently() {
    let job = sign();
    let (grouped, _) = group(&job, &members(&job, &[5, 6], &[1]), None).unwrap();
    assert_eq!(gcode(&grouped), gcode(&job));
}

#[test]
fn group_adds_to_one_group_and_merges_several() {
    let job = sign();
    let (job, a) = group(&job, &members(&job, &[5], &[]), None).unwrap();
    let (job, b) = group(&job, &members(&job, &[6], &[]), Some("Border")).unwrap();
    assert_eq!((a, b), (1, 2));
    // An operation made from group 2.
    let mut job = job;
    *job.operations[1].select_mut().unwrap() = Selector {
        group: Some(b),
        ..Selector::default()
    };
    // One group in the selection: the text joins it.
    let (job, id) = group(&job, &members(&job, &[5], &[1]), None).unwrap();
    assert_eq!(id, a);
    assert_eq!(job.drawing.groups.len(), 2);
    // Two groups: merged into the first; group 2's operation follows it.
    let (job, id) = group(&job, &members(&job, &[5, 6], &[]), None).unwrap();
    assert_eq!(id, a);
    assert_eq!(job.drawing.groups.len(), 1);
    assert_eq!(job.drawing.groups[0].shapes, [5, 6]);
    assert_eq!(job.operations[1].select().group, Some(a));
}

#[test]
fn only_drawn_shapes_and_texts_and_each_in_one_group() {
    let job = chipload_job::load_job(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/corpus/jobs/v19/image_vcarve.chipload"),
    )
    .unwrap();
    let err = Members::of(&job, &[1], &[]).unwrap_err().to_string();
    assert!(err.contains("drawn shapes and texts"), "{err}");
    // A shape in two groups is refused when read.
    let mut job = sign();
    job.drawing.groups = vec![
        chipload_job::ShapeGroup {
            id: 1,
            name: "A".into(),
            shapes: vec![5],
            texts: vec![],
        },
        chipload_job::ShapeGroup {
            id: 2,
            name: "B".into(),
            shapes: vec![5],
            texts: vec![],
        },
    ];
    let problems: Vec<String> = job
        .drawing
        .validate()
        .iter()
        .map(|p| p.text.clone())
        .collect();
    assert!(
        problems.iter().any(|p| p.contains("two groups")),
        "{problems:?}"
    );
}

#[test]
fn deleting_members_and_groups() {
    let job = sign();
    let (job, id) = group(&job, &members(&job, &[5, 6], &[]), None).unwrap();
    let (job, _) = delete_shape(&job, 6).unwrap();
    assert_eq!(job.drawing.groups[0].shapes, [5]);
    let (job, _) = delete_shape(&job, 5).unwrap();
    assert!(job.drawing.groups.is_empty(), "an empty group goes");
    let job = sign();
    let (job, id2) = group(&job, &members(&job, &[5, 6], &[1]), None).unwrap();
    assert_eq!(id, id2);
    let gone = delete_group(&job, id2).unwrap();
    assert!(gone.drawing.texts.is_empty());
    assert!(gone.drawing.shapes.is_empty());
    assert!(gone.drawing.groups.is_empty());
}

#[test]
fn an_operation_made_from_a_group_follows_it() {
    let job = sign();
    let (mut job, id) = group(&job, &members(&job, &[5], &[]), None).unwrap();
    // No tabs: the small circle added below has no room for them.
    if let chipload_job::Operation::Profile(p) = &mut job.operations[2] {
        p.tabs = None;
    }
    // Profile the group instead of the frame layer.
    *job.operations[2].select_mut().unwrap() = Selector {
        group: Some(id),
        ..Selector::default()
    };
    let mut layer = job.clone();
    layer.operations[2].select_mut().unwrap().group = None;
    layer.operations[2].select_mut().unwrap().layer = Some("frame".into());
    assert_eq!(
        gcode(&job),
        gcode(&layer),
        "the group's one shape is the frame"
    );
    // A circle drawn and added to the group is cut too; taken out again, it isn't.
    let (with_hole, hole) = add_shape(
        &job,
        "frame",
        Geometry::Circle {
            center: [100.0, 60.0],
            radius: 3.0,
        },
    )
    .unwrap();
    let added = add_to_group(&with_hole, id, &members(&with_hole, &[hole], &[])).unwrap();
    let cut = run_job(&added).unwrap();
    let free = run_job(&with_hole).unwrap();
    assert!(cut.program.moves.len() > free.program.moves.len());
    let taken = remove_from_group(&added, &members(&added, &[hole], &[])).unwrap();
    assert_eq!(gcode(&taken), gcode(&with_hole));
    // Left out with exclude: as without it.
    let mut left_out = added.clone();
    left_out.operations[2].select_mut().unwrap().exclude = Some(vec![hole]);
    assert_eq!(gcode(&left_out), gcode(&with_hole));
    // Ungrouped: the operation names the shapes, and cuts the same.
    let free_again = ungroup(&added, id).unwrap();
    let select = free_again.operations[2].select();
    assert_eq!(
        (select.group, select.shapes.clone()),
        (None, Some(vec![5, hole]))
    );
    assert_eq!(gcode(&free_again), gcode(&added));
}

#[test]
fn ungrouping_a_cut_group_with_texts_is_refused() {
    let job = sign();
    let (mut job, id) = group(&job, &members(&job, &[5], &[1]), None).unwrap();
    *job.operations[2].select_mut().unwrap() = Selector {
        group: Some(id),
        ..Selector::default()
    };
    let err = ungroup(&job, id).unwrap_err().to_string();
    assert!(err.contains("has texts"), "{err}");
    // Not cut: ungrouped freely.
    job.operations[2].select_mut().unwrap().group = None;
    job.operations[2].select_mut().unwrap().layer = Some("frame".into());
    assert!(ungroup(&job, id).unwrap().drawing.groups.is_empty());
}

#[test]
fn a_group_moves_turns_and_scales_with_its_texts() {
    let job = sign();
    let (job, id) = group(&job, &members(&job, &[5, 6], &[1]), None).unwrap();
    let ((x0, y0), (x1, y1)) = group_box(&job, id).unwrap();
    let middle = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);
    let moved = transform_group(&job, id, middle, (300.0, 200.0), 2.0, 90.0).unwrap();
    let ((a0, b0), (a1, b1)) = group_box(&moved, id).unwrap();
    // Turned a quarter and doubled: width and height swap and double.
    assert!(((a1 - a0) - 2.0 * (y1 - y0)).abs() < 1e-6, "{a0} {a1}");
    assert!(((b1 - b0) - 2.0 * (x1 - x0)).abs() < 1e-6);
    assert!(((a0 + a1) / 2.0 - 300.0).abs() < 1e-6 && ((b0 + b1) / 2.0 - 200.0).abs() < 1e-6);
    let text = &moved.drawing.texts[0];
    assert!((text.height - 60.0).abs() < 1e-9 && (text.rotation - 90.0).abs() < 1e-9);
    // The letters are those the font gives for the new text.
    let (remade, _) =
        chipload_pipeline::texts::remake(&moved, &moved, text.id, &BuiltinOnly).unwrap();
    // At twice the size the font's arcs are fitted again: compare where the letters are.
    let letters_box = |j: &Job| {
        let ids: Vec<u64> = j
            .drawing
            .shapes
            .iter()
            .filter(|s| s.text() == Some(1))
            .map(|s| s.id)
            .collect();
        chipload_pipeline::shapes::drawn_box(j, &ids).unwrap()
    };
    let (ours, theirs) = (letters_box(&moved), letters_box(&remade));
    for (a, b) in [(ours.0, theirs.0), (ours.1, theirs.1)] {
        assert!(
            (a.0 - b.0).abs() < 0.05 && (a.1 - b.1).abs() < 0.05,
            "{ours:?} {theirs:?}"
        );
    }
}

#[test]
fn a_library_entry_of_several_shapes_comes_in_as_a_group() {
    use chipload_pipeline::shape_library::{InsertSize, entry_from, insert};
    let job = sign();
    let entry = entry_from(&job, &[5, 6], &[1]).unwrap();
    let done = insert(
        &job,
        &entry,
        "Signs/Welcome",
        None,
        &InsertSize::AsSaved,
        None,
    )
    .unwrap();
    let id = done.group.expect("a group");
    let g = done.job.drawing.groups.iter().find(|g| g.id == id).unwrap();
    assert_eq!(g.name, "Welcome");
    assert_eq!(g.texts, done.texts);
    // One shape: no group.
    let one = entry_from(&job, &[5], &[]).unwrap();
    let done = insert(&job, &one, "Frame", None, &InsertSize::AsSaved, None).unwrap();
    assert_eq!(done.group, None);
}
