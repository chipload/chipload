// SPDX-License-Identifier: GPL-3.0-or-later
//! Align and distribute move whole contours and whole groups (docs/specs/arrange-whole.md).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};

use chipload_job::{Geometry, Job};
use chipload_pipeline::drawing_edit::{AlignTo, align_shapes, arrange_units, distribute_shapes};
use chipload_pipeline::explode::explode_dxf;
use chipload_pipeline::fonts::BuiltinOnly;
use chipload_pipeline::groups::{Members, group, group_box};
use chipload_pipeline::shapes::{add_shape, shapes_bounds};
use chipload_pipeline::texts::{NewText, add_text};
use chipload_pipeline::{empty_project, load_drawing};

fn corpus(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/corpus")
        .join(rel)
}

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-6
}

fn centre_x(b: ((f64, f64), (f64, f64))) -> f64 {
    (b.0.0 + b.1.0) / 2.0
}

/// Issue #13's drawing, exploded: the slot on layer "poche 3mm" is four open polylines,
/// S12 to S15, that chain into one closed contour; S47 a rectangle on the same layer.
fn exploded() -> Job {
    let shapes = chipload_dxf::shapes_from_dxf(&corpus("dxf/shared_edges.dxf"), None).unwrap();
    let job = chipload_pipeline::new_project(shapes);
    let id = job.drawing.dxfs[0].id;
    explode_dxf(&job, id, None).unwrap().0
}

/// The closed contours made of shapes `ids` alone.
fn closed_of(job: &Job, ids: &[u64]) -> usize {
    load_drawing(job)
        .unwrap()
        .contours
        .iter()
        .filter(|c| c.closed)
        .filter(|c| {
            c.parts
                .iter()
                .all(|p| p.segment.shape.is_some_and(|s| ids.contains(&s)))
        })
        .count()
}

#[test]
fn a_slot_of_four_pieces_is_aligned_whole() {
    let job = exploded();
    let slot = [12, 13, 14, 15];
    assert_eq!(closed_of(&job, &slot), 1, "the slot is one closed contour");
    let units = arrange_units(&job, &[12, 13, 14, 15, 47]).unwrap();
    assert_eq!(units.len(), 2);
    assert_eq!(units[0].shapes, slot);
    let out = align_shapes(&job, &[12, 13, 14, 15, 47], AlignTo::Centre, false).unwrap();
    assert_eq!(closed_of(&out, &slot), 1, "still one closed contour");
    let (s, r) = (
        shapes_bounds(&out, &slot).unwrap(),
        shapes_bounds(&out, &[47]).unwrap(),
    );
    assert!(near(centre_x(s), centre_x(r)), "{s:?} {r:?}");
    // The slot alone to the stock's middle, whole.
    let stock = align_shapes(&job, &slot, AlignTo::Middle, true).unwrap();
    assert_eq!(closed_of(&stock, &slot), 1);
    // The slot alone is one thing: nothing to line it up with.
    assert!(align_shapes(&job, &slot, AlignTo::Left, false).is_err());
}

/// A plate: a circle S1, a rectangle S2 and the text "HI", S2 and the text grouped.
fn plate() -> (Job, u64) {
    let job = empty_project();
    let (job, _) = add_shape(
        &job,
        "holes",
        Geometry::Circle {
            center: [10.0, 10.0],
            radius: 5.0,
        },
    )
    .unwrap();
    let (job, frame) = add_shape(
        &job,
        "frame",
        Geometry::Rectangle {
            center: [100.0, 60.0],
            length: 80.0,
            width: 40.0,
            angle: 0.0,
            corner_radius: 0.0,
        },
    )
    .unwrap();
    let mut new = NewText::new("HI", 10.0);
    new.x = 90.0;
    new.y = 55.0;
    let (job, text) = add_text(&job, new, &BuiltinOnly).unwrap();
    let members = Members::of(&job, &[frame], &[1]).unwrap();
    let (job, id) = group(&job, &members, None).unwrap();
    assert!(
        text.shapes
            .iter()
            .all(|s| job.drawing.group_shapes(id).contains(s))
    );
    (job, id)
}

#[test]
fn a_group_is_aligned_whole_with_its_text() {
    let (job, id) = plate();
    let members = job.drawing.group_shapes(id);
    let letters: Vec<u64> = members.iter().copied().filter(|&s| s != 2).collect();
    let text_before = shapes_bounds(&job, &letters).unwrap();
    let frame_before = shapes_bounds(&job, &[2]).unwrap();
    let mut ids = vec![1];
    ids.extend(&members);
    assert_eq!(arrange_units(&job, &ids).unwrap().len(), 2);
    let out = align_shapes(&job, &ids, AlignTo::Left, false).unwrap();
    // The group's box starts where the circle's does; the frame and its text moved alike.
    let group_after = group_box(&out, id).unwrap();
    assert!(near(group_after.0.0, 5.0), "{group_after:?}");
    let dx = shapes_bounds(&out, &[2]).unwrap().0.0 - frame_before.0.0;
    let text_after = shapes_bounds(&out, &letters).unwrap();
    assert!(near(text_after.0.0 - text_before.0.0, dx));
    assert!(near(out.drawing.texts[0].x - job.drawing.texts[0].x, dx));
    // The group alone, to the stock: whole.
    let stock = align_shapes(&job, &members, AlignTo::Centre, true).unwrap();
    let frame = shapes_bounds(&stock, &[2]).unwrap();
    let text = shapes_bounds(&stock, &letters).unwrap();
    assert!(near(
        text.0.0 - frame.0.0,
        text_before.0.0 - frame_before.0.0
    ));
    // One shape of the group picked alone moves alone; one letter alone is refused.
    let alone = align_shapes(&job, &[1, 2], AlignTo::Left, false).unwrap();
    assert!(near(shapes_bounds(&alone, &[2]).unwrap().0.0, 5.0));
    assert!(near(
        shapes_bounds(&alone, &letters).unwrap().0.0,
        text_before.0.0
    ));
    assert!(align_shapes(&job, &[1, letters[0]], AlignTo::Left, false).is_err());
}

#[test]
fn distributing_spaces_out_whole_contours() {
    let job = exploded();
    // The slot between two rectangles of the same layer: three things, not six.
    let ids = [45, 12, 13, 14, 15, 47];
    assert_eq!(arrange_units(&job, &ids).unwrap().len(), 3);
    let out = distribute_shapes(&job, &ids, false).unwrap();
    assert_eq!(closed_of(&out, &[12, 13, 14, 15]), 1);
    assert!(distribute_shapes(&job, &[12, 13, 14, 15, 47], true).is_err());
}
