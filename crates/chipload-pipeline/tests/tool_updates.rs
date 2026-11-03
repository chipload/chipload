// SPDX-License-Identifier: GPL-3.0-or-later
//! Project tools against their library tools (docs/specs/tool-updates.md).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;

use chipload_job::library::{Cutting, LibraryTool, Link, ToolLibrary};
use chipload_job::materials::MaterialLibrary;
use chipload_job::{Job, load_job};
use chipload_pipeline::library::{project_tool_link, update_from_library};

const UID: &str = "a1b2c3d4e5f60718";

/// The badge project, its tool 1 linked to library tool UID, and the library with it.
fn setup() -> (Job, ToolLibrary, MaterialLibrary) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/corpus/jobs/v31/svg_badge.chipload");
    let mut job = load_job(&path).unwrap();
    job.tools[0].library_uid = Some(UID.into());
    job.tools[0].name = Some("6 mm".into());
    let mut tools = ToolLibrary::default();
    tools.tools.push(LibraryTool::from_tool(
        "6 mm",
        UID,
        &job.tools[0],
        Vec::new(),
    ));
    (job, tools, MaterialLibrary::default())
}

#[test]
fn a_differing_tool_is_updated_and_the_others_are_left() {
    let (job, mut tools, materials) = setup();
    assert!(matches!(
        project_tool_link(&job, &tools, &materials, &job.tools[0]),
        Link::Same(_)
    ));
    tools.tools[0].diameter = 6.35;
    let Link::Differs(_, changes) = project_tool_link(&job, &tools, &materials, &job.tools[0])
    else {
        panic!("differs");
    };
    assert_eq!(changes.len(), 1);
    let (out, updated) = update_from_library(&job, &tools, &materials, None);
    assert_eq!(out.tools[0].diameter, 6.35);
    assert_eq!(updated.len(), 1);
    assert_eq!(updated[0].0, job.tools[0].id);
    // Again: nothing to update. Another tool only: nothing either.
    assert!(
        update_from_library(&out, &tools, &materials, None)
            .1
            .is_empty()
    );
    assert!(
        update_from_library(&job, &tools, &materials, Some(99))
            .1
            .is_empty()
    );
}

#[test]
fn a_speed_from_the_setups_material_is_no_difference_and_stays() {
    let (job, mut tools, mut materials) = setup();
    materials.add("Plywood").unwrap();
    let plywood = materials.materials[0].clone();
    tools.tools[0].set_row(
        Some(&plywood.uid),
        Cutting {
            rpm: Some(21000.0),
            ..Cutting::uniform(1500.0, 500.0, 2.5, 0.4)
        },
    );
    let (job, _) = chipload_pipeline::materials::use_material(&job, &tools, Some(&plywood));
    assert_eq!(job.tools[0].rpm, Some(21000.0));
    assert!(matches!(
        project_tool_link(&job, &tools, &materials, &job.tools[0]),
        Link::Same(_)
    ));
    // Another difference: updated, and the material's speed kept.
    tools.tools[0].flutes = Some(3);
    let (out, updated) = update_from_library(&job, &tools, &materials, None);
    assert_eq!(updated.len(), 1);
    assert_eq!(
        (out.tools[0].flutes, out.tools[0].rpm),
        (Some(3), Some(21000.0))
    );
    // A speed set by hand is a difference.
    let mut by_hand = job.clone();
    by_hand.tools[0].rpm = Some(12000.0);
    assert!(matches!(
        project_tool_link(&by_hand, &tools, &materials, &by_hand.tools[0]),
        Link::Differs(..)
    ));
}
