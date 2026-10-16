// SPDX-License-Identifier: GPL-3.0-or-later
//! Several DXFs in one project (docs/specs/drawing-sources.md §2, §8).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};

use chipload_job::{Job, Note, load_job};
use chipload_pipeline::dxfs::{add_dxf, remove_dxf, update_dxf};
use chipload_pipeline::placement::{Move, place_drawing, place_dxf};
use chipload_pipeline::{run_job, update_drawing};

fn corpus(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/corpus")
        .join(rel)
}

/// The scaled part (one DXF, a profile of layer HOLES and one of CUT_OUTSIDE), with the same
/// DXF added a second time.
fn two() -> (Job, Job) {
    let one = load_job(&corpus("jobs/v20/scaled_part.chipload")).unwrap();
    let (two, _) = add_dxf(&one, &corpus("dxf/simple_part.dxf"), None).unwrap();
    (one, two)
}

#[test]
fn adding_a_dxf_never_changes_what_an_operation_cuts() {
    let one = load_job(&corpus("jobs/v20/scaled_part.chipload")).unwrap();
    let (two, report) = add_dxf(&one, &corpus("dxf/simple_part.dxf"), None).unwrap();
    // Both operations select a layer the new DXF has too: they stay on the first.
    assert!(
        report.lines.iter().any(|l| l
            == "Operations 1, 2 keep cutting simple_part only: the new DXF has their layers too."),
        "{:?}",
        report.lines
    );
    assert!(two.operations.iter().all(|op| op.select().dxf == Some(1)));
    assert!(!report.lines.iter().any(|l| l.contains("is in")));
    let (two, _) = place_dxf(&two, 2, Move::By(400.0, 0.0)).unwrap();
    assert_eq!(
        run_job(&two).unwrap().files[0].gcode,
        run_job(&one).unwrap().files[0].gcode
    );
}

fn shapes_of(job: &Job, dxf: u64) -> Vec<chipload_job::Shape> {
    job.drawing
        .shapes
        .iter()
        .filter(|s| s.dxf() == Some(dxf))
        .cloned()
        .collect()
}

#[test]
fn a_second_dxf_is_added_where_its_file_has_it() {
    let (one, two) = two();
    assert_eq!(two.drawing.dxfs.len(), 2);
    let added = &two.drawing.dxfs[1];
    assert_eq!((added.id, added.name.as_str()), (2, "simple_part 2"));
    assert_eq!(added.placement, chipload_job::Placement::default());
    let new = shapes_of(&two, 2);
    assert_eq!(new.len(), 7);
    let first_new = one.drawing.next_shape_id;
    assert!(new.iter().all(|s| s.id >= first_new));
    assert_eq!(
        shapes_of(&two, 1),
        shapes_of(&one, 1),
        "the first DXF is untouched"
    );
    assert!(two.validate().is_empty(), "{:?}", two.validate());
}

#[test]
fn a_layer_alone_in_two_dxfs_is_noted_and_a_dxf_selector_limits_it() {
    let (one, two) = two();
    // Apart, as parts are: on top of each other, one part's holes would lie inside the
    // other's outline, which changes their nesting and so their order.
    let (mut two, _) = place_dxf(&two, 2, Move::By(400.0, 0.0)).unwrap();
    // As a hand-written file could have them: by layer alone.
    for op in &mut two.operations {
        op.select_mut().dxf = None;
    }
    let result = run_job(&two).unwrap();
    let clashes: Vec<&Note> = result
        .notes
        .iter()
        .filter(|n| n.contains("is in simple_part and simple_part 2"))
        .collect();
    assert_eq!(clashes.len(), 2, "{:?}", result.notes);
    // Limited to DXF 1, the operations cut exactly what they cut before.
    for op in &mut two.operations {
        op.select_mut().dxf = Some(1);
    }
    let limited = run_job(&two).unwrap();
    assert!(
        !limited.notes.iter().any(|n| n.contains("is in")),
        "{:?}",
        limited.notes
    );
    let before = run_job(&one).unwrap();
    assert_eq!(limited.files[0].gcode, before.files[0].gcode);
    // Limited to DXF 2, they cut the other part.
    for op in &mut two.operations {
        op.select_mut().dxf = Some(2);
    }
    let other = run_job(&two).unwrap();
    assert_ne!(other.files[0].gcode, before.files[0].gcode);
}

#[test]
fn placing_or_updating_one_dxf_leaves_the_other() {
    let (_, two) = two();
    let (moved, report) = place_dxf(&two, 2, Move::By(10.0, -5.0)).unwrap();
    assert_eq!(shapes_of(&moved, 1), shapes_of(&two, 1));
    assert_ne!(shapes_of(&moved, 2), shapes_of(&two, 2));
    assert_eq!(moved.drawing.dxfs[1].placement.dx, 10.0);
    assert!(
        report.lines[0].starts_with("Moved simple_part 2 by"),
        "{:?}",
        report.lines
    );
    let err = place_drawing(&two, Move::Center).unwrap_err().to_string();
    assert!(err.contains("2 DXFs"), "{err}");
    // Updated from the same file, where it was put: nothing changes, ids stay.
    let (updated, report) = update_dxf(&moved, 2, &corpus("dxf/simple_part.dxf"), None).unwrap();
    assert_eq!(report.unchanged.len(), 7, "{:?}", report.lines);
    assert_eq!(updated.drawing.shapes, moved.drawing.shapes);
    assert_eq!(updated.drawing.dxfs[1].name, "simple_part 2");
    let err = update_drawing(&two, &corpus("dxf/simple_part.dxf"), None)
        .unwrap_err()
        .to_string();
    assert!(err.contains("say which"), "{err}");
}

#[test]
fn removing_a_dxf_removes_its_shapes_and_names_the_operations_that_used_it() {
    let (one, mut two) = two();
    let lost = shapes_of(&two, 2)[0].id;
    two.operations[0].select_mut().shapes = Some(vec![lost]);
    two.operations[0].select_mut().layer = None;
    two.operations[1].select_mut().dxf = Some(2);
    two.drawing.hidden = vec!["dxf:2".into(), "dxf:2/layer:HOLES".into(), "dxf:1".into()];
    let (removed, lines) = remove_dxf(&two, 2).unwrap();
    assert!(shapes_of(&removed, 2).is_empty());
    assert_eq!(shapes_of(&removed, 1), shapes_of(&one, 1));
    assert_eq!(removed.drawing.hidden, vec!["dxf:1".to_string()]);
    assert!(
        lines.iter().any(|l| l.starts_with("Operation 1")),
        "{lines:?}"
    );
    assert!(
        lines.iter().any(|l| l.starts_with("Operation 2")),
        "{lines:?}"
    );
    let problems = removed.validate();
    assert!(
        problems
            .iter()
            .any(|p| p.text.contains("select.dxf: no DXF 2")),
        "{problems:?}"
    );
    assert!(remove_dxf(&removed, 2).is_err());
}

#[test]
fn a_selector_may_name_a_whole_dxf() {
    // An outline profile of all of DXF 2: its outline and its holes, whatever their layers.
    let (_, two) = two();
    let (mut two, _) = place_dxf(&two, 2, Move::By(400.0, 0.0)).unwrap();
    let select = two.operations[1].select_mut();
    *select = chipload_job::Selector {
        dxf: Some(2),
        ..Default::default()
    };
    assert!(two.validate().is_empty(), "{:?}", two.validate());
    let result = run_job(&two).unwrap();
    assert!(!result.files[0].gcode.is_empty());
    // Nothing at all is still refused.
    *two.operations[1].select_mut() = chipload_job::Selector::default();
    assert!(
        two.validate()
            .iter()
            .any(|p| p.text.contains("needs a DXF, an image, a text, a layer or shapes"))
    );
}
