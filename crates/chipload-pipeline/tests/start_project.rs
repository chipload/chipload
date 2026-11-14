// SPDX-License-Identifier: GPL-3.0-or-later
//! A new project started from the library and the new project dialog's choices
//! (docs/specs/new-project-dialog.md §2-§5).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use chipload_job::library::{LibraryTool, uid_for_name};
use chipload_job::materials::Material;
use chipload_job::{StockOrigin, ToolType, ZZero};
use chipload_pipeline::empty_project;
use chipload_pipeline::library::{
    Board, Library, ProjectStart, apply_library, start_project, tool_numbers,
};

fn tool(name: &str, id: i64, kind: ToolType, diameter: f64) -> LibraryTool {
    LibraryTool {
        name: name.into(),
        uid: uid_for_name(name),
        id,
        kind,
        diameter,
        flutes: Some(2),
        center_cutting: Some(true),
        rpm: Some(18000.0),
        cutting_data: Vec::new(),
        angle: (kind == ToolType::Vbit).then(|| 60f64.to_radians()),
        tip_diameter: None,
        corner_radius: None,
    }
}

fn library() -> Library {
    let mut library = Library::default();
    library.tools.tools = vec![
        tool("6 mm", 1, ToolType::Endmill, 6.0),
        tool("V 60", 1, ToolType::Vbit, 12.7),
        tool("3 mm", 3, ToolType::Endmill, 3.0),
    ];
    library.tools.default_tool = Some("3 mm".into());
    library.materials.materials = vec![
        Material {
            name: "Plywood".into(),
            uid: uid_for_name("Plywood"),
            thickness: Some(18.0),
        },
        Material {
            name: "MDF".into(),
            uid: uid_for_name("MDF"),
            thickness: None,
        },
    ];
    library.materials.default_thickness = Some(12.0);
    library
}

#[test]
fn tools_keep_their_numbers_and_a_clash_takes_the_next_free_one() {
    assert_eq!(tool_numbers([1, 1, 3, 0, 2]), [1, 2, 3, 4, 5]);
    let mut job = empty_project();
    let start = ProjectStart {
        tools: Some(vec!["6 mm".into(), uid_for_name("V 60"), "3 mm".into()]),
        ..ProjectStart::default()
    };
    start_project(&mut job, &library(), &start).unwrap();
    let got: Vec<(i64, &str)> = job
        .tools
        .iter()
        .map(|t| (t.id, t.name.as_deref().unwrap()))
        .collect();
    assert_eq!(got, [(1, "6 mm"), (2, "V 60"), (3, "3 mm")]);
    assert!(job.tools.iter().all(|t| t.library_uid.is_some()));
}

#[test]
fn no_tools_is_allowed_and_said() {
    let mut job = empty_project();
    let start = ProjectStart {
        tools: Some(Vec::new()),
        ..ProjectStart::default()
    };
    let notes = start_project(&mut job, &library(), &start).unwrap();
    assert!(job.tools.is_empty());
    assert!(notes.iter().any(|n| n.english().contains("no tools")));
}

#[test]
fn every_choice_is_applied() {
    let mut job = empty_project();
    let start = ProjectStart {
        material: Some(Some("MDF".into())),
        thickness: Some(15.0),
        board: Some(Board {
            x: 600.0,
            y: 400.0,
            fit: None,
        }),
        z_zero: Some(ZZero::Bottom),
        ..ProjectStart::default()
    };
    start_project(&mut job, &library(), &start).unwrap();
    let stock = job.stock();
    assert_eq!(stock.material.as_ref().unwrap().name, "MDF");
    assert_eq!(stock.thickness, 15.0);
    assert_eq!((stock.x, stock.y, stock.fit), (600.0, 400.0, None));
    assert_eq!(stock.origin, StockOrigin::Drawing);
    assert_eq!(stock.z_zero, ZZero::Bottom);
}

#[test]
fn a_material_with_a_thickness_gives_it_and_one_without_the_default() {
    let thickness = |material: &str| {
        let mut job = empty_project();
        let start = ProjectStart {
            material: Some(Some(material.into())),
            ..ProjectStart::default()
        };
        start_project(&mut job, &library(), &start).unwrap();
        job.stock().thickness
    };
    assert_eq!(thickness("Plywood"), 18.0);
    assert_eq!(thickness("MDF"), 12.0);
    let lib = library();
    assert_eq!(lib.materials.thickness_for(None, 10.0), 12.0);
    let mut bare = library();
    bare.materials.default_thickness = None;
    assert_eq!(bare.materials.thickness_for(None, 10.0), 10.0);
}

#[test]
fn unknown_names_are_refused() {
    for start in [
        ProjectStart {
            machine: Some("Nope".into()),
            ..ProjectStart::default()
        },
        ProjectStart {
            material: Some(Some("Nope".into())),
            ..ProjectStart::default()
        },
        ProjectStart {
            tools: Some(vec!["Nope".into()]),
            ..ProjectStart::default()
        },
    ] {
        let err = start_project(&mut empty_project(), &library(), &start).unwrap_err();
        assert!(err.english().contains("Nope"), "{}", err.english());
    }
}

#[test]
fn without_choices_the_library_defaults_apply_as_before() {
    let (mut a, mut b) = (empty_project(), empty_project());
    let notes = apply_library(&mut a, &library(), None).unwrap();
    start_project(&mut b, &library(), &ProjectStart::default()).unwrap();
    assert_eq!(a, b);
    // The default tool, numbered as in the library; the default thickness.
    assert_eq!(a.tools.len(), 1);
    assert_eq!(a.tools[0].id, 3);
    assert_eq!(a.stock().thickness, 12.0);
    assert!(notes.iter().any(|n| n.english().contains("(default tool)")));
}
