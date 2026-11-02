// SPDX-License-Identifier: GPL-3.0-or-later
//! The shape library (docs/specs/shape-library.md §4, §5.2).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};

use chipload_job::Geometry;
use chipload_job::shape_library::{SHAPES_VERSION, load_shape_entry};
use chipload_pipeline::empty_project;
use chipload_pipeline::fonts::BuiltinOnly;
use chipload_pipeline::shape_library::{
    InsertSize, ItemKind, add_group, entry_box, entry_from, find, insert, list, move_to, preview,
    read, remove, remove_group, rename, rename_group, save,
};
use chipload_pipeline::shapes::{add_shape, drawn_box};
use chipload_pipeline::texts::{NewText, add_text};

/// An empty folder of its own for each test.
fn folder(test: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "chipload-shape-library-{}-{test}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn corpus(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/corpus")
        .join(rel)
}

/// A project with a circle (S1) and a rectangle (S2) round it, and the text "HI" (T1).
fn plate() -> chipload_job::Job {
    let job = empty_project();
    let (job, _) = add_shape(
        &job,
        "holes",
        Geometry::Circle {
            center: [50.0, 40.0],
            radius: 5.0,
        },
    )
    .unwrap();
    let (job, _) = add_shape(
        &job,
        "frame",
        Geometry::Rectangle {
            center: [50.0, 40.0],
            length: 80.0,
            width: 40.0,
            angle: 0.0,
            corner_radius: 4.0,
        },
    )
    .unwrap();
    let mut new = NewText::new("HI", 10.0);
    new.x = 30.0;
    new.y = 30.0;
    add_text(&job, new, &BuiltinOnly).unwrap().0
}

fn size(b: ((f64, f64), (f64, f64))) -> (f64, f64) {
    (b.1.0 - b.0.0, b.1.1 - b.0.1)
}

#[test]
fn a_selection_is_saved_and_inserted_with_new_ids_where_asked() {
    let dir = folder("save-insert");
    let job = plate();
    let letters: Vec<u64> = job
        .drawing
        .shapes
        .iter()
        .filter(|s| s.text() == Some(1))
        .map(|s| s.id)
        .collect();
    // A letter selected takes its whole text with it.
    let entry = entry_from(&job, &[1, 2, letters[0]], &[]).unwrap();
    assert_eq!(entry.texts.len(), 1);
    assert_eq!(entry.shapes.len(), 2 + letters.len());
    save(&dir, "Plate", &entry, false).unwrap();
    let listing = list(&dir).unwrap();
    assert_eq!(listing.items.len(), 1);
    assert_eq!(listing.items[0].kind, ItemKind::Entry);
    let entry = read(&find(&dir, "Plate").unwrap()).unwrap();
    // The entry's box is the rectangle's.
    let (w, h) = size(entry_box(&entry).unwrap());
    assert!((w - 80.0).abs() < 1e-9 && (h - 40.0).abs() < 1e-9);

    // Into the same project, its middle at (200, 100).
    let done = insert(
        &job,
        &entry,
        "Plate",
        Some((200.0, 100.0)),
        &InsertSize::AsSaved,
        None,
    )
    .unwrap();
    assert_eq!(done.texts, [2]);
    let first_new = job.drawing.next_shape_id;
    assert_eq!(done.shapes[0], first_new);
    assert_eq!(done.shapes.len(), entry.shapes.len());
    let moved = drawn_box(&done.job, &done.shapes).unwrap();
    assert!((moved.0.0 - 160.0).abs() < 1e-9 && (moved.0.1 - 80.0).abs() < 1e-9);
    // The text stays a text, its letters moved with it and naming it.
    let text = &done.job.drawing.texts[1];
    assert_eq!((text.x, text.y), (180.0, 90.0));
    let new_letters = done
        .job
        .drawing
        .shapes
        .iter()
        .filter(|s| s.text() == Some(2));
    assert_eq!(new_letters.count(), letters.len());
    // Nothing that was there changed.
    assert_eq!(
        done.job.drawing.shapes[..job.drawing.shapes.len()],
        job.drawing.shapes[..]
    );
    // Layers kept, or one for all.
    assert!(
        done.job
            .drawing
            .shapes
            .iter()
            .any(|s| s.layer == "holes" && s.id >= first_new)
    );
    let one = insert(
        &job,
        &entry,
        "Plate",
        None,
        &InsertSize::AsSaved,
        Some("sign"),
    )
    .unwrap();
    assert!(one.shapes.iter().all(|id| {
        one.job
            .drawing
            .shapes
            .iter()
            .find(|s| s.id == *id)
            .unwrap()
            .layer
            == "sign"
    }));
    assert_eq!(one.job.drawing.texts[1].layer, "sign");
}

#[test]
fn an_entry_is_scaled_to_a_size_or_round_the_selection() {
    let dir = folder("scale");
    let job = plate();
    save(
        &dir,
        "Plate",
        &entry_from(&job, &[1, 2], &[1]).unwrap(),
        false,
    )
    .unwrap();
    let entry = read(&find(&dir, "Plate").unwrap()).unwrap();
    let wide = insert(
        &job,
        &entry,
        "Plate",
        Some((0.0, 0.0)),
        &InsertSize::Width(160.0),
        None,
    )
    .unwrap();
    let (w, h) = size(drawn_box(&wide.job, &wide.shapes).unwrap());
    assert!((w - 160.0).abs() < 1e-9 && (h - 80.0).abs() < 1e-9);
    // Text heights scale with it.
    assert!((wide.job.drawing.texts[1].height - 20.0).abs() < 1e-9);
    // Round the circle (10 × 10) with 5 mm: a 20 × 20 box. The plate's 80 × 40 fits it
    // by its height, at half size, centred on the circle.
    let around = InsertSize::Around {
        ids: vec![1],
        margin: 5.0,
    };
    let fitted = insert(&job, &entry, "Plate", None, &around, None).unwrap();
    let ((x0, y0), (x1, y1)) = drawn_box(&fitted.job, &fitted.shapes).unwrap();
    assert!((y1 - y0 - 20.0).abs() < 1e-9 && (x1 - x0 - 40.0).abs() < 1e-9);
    assert!(((x0 + x1) / 2.0 - 50.0).abs() < 1e-9 && ((y0 + y1) / 2.0 - 40.0).abs() < 1e-9);
}

#[test]
fn a_text_in_a_font_this_computer_lacks_keeps_its_letters() {
    let dir = folder("font");
    let job = plate();
    let mut entry = entry_from(&job, &[], &[1]).unwrap();
    entry.texts[0].font = "Some Font Not Here".into();
    save(&dir, "Hi", &entry, false).unwrap();
    let entry = read(&find(&dir, "Hi").unwrap()).unwrap();
    let done = insert(
        &empty_project(),
        &entry,
        "Plate",
        None,
        &InsertSize::Scale(2.0),
        None,
    )
    .unwrap();
    assert_eq!(done.job.drawing.texts[0].font, "Some Font Not Here");
    assert_eq!(done.shapes.len(), entry.shapes.len());
    assert!(done.job.drawing.shapes.iter().all(|s| s.text() == Some(1)));
}

#[test]
fn names_groups_and_deleting() {
    let dir = folder("groups");
    let job = plate();
    let entry = entry_from(&job, &[1], &[]).unwrap();
    // Bad names are refused, an existing one only replaced when asked (kept as .bak).
    for bad in ["", " x", ".x", "a:b", "x.", "CON", "com1"] {
        assert!(save(&dir, bad, &entry, false).is_err(), "{bad:?}");
    }
    save(&dir, "Signs/Plate", &entry, false).unwrap();
    assert!(save(&dir, "Signs/Plate", &entry, false).is_err());
    save(&dir, "Signs/Plate", &entry, true).unwrap();
    assert!(dir.join("Signs/Plate.chipload-shapes.bak").is_file());
    add_group(&dir, "Empty").unwrap();
    assert!(add_group(&dir, "Empty").is_err());
    let listing = list(&dir).unwrap();
    assert_eq!(listing.groups, ["Empty", "Signs"]);
    assert_eq!(listing.items[0].full_name(), "Signs/Plate");

    // Moved out of its group and back, renamed.
    move_to(&dir, "Signs/Plate", None).unwrap();
    assert_eq!(list(&dir).unwrap().items[0].full_name(), "Plate");
    rename(&dir, "Plate", "Empty/Door").unwrap();
    assert_eq!(list(&dir).unwrap().items[0].full_name(), "Empty/Door");
    save(&dir, "Door", &entry, false).unwrap();
    assert!(
        rename(&dir, "Door", "Empty/Door").is_err(),
        "never replaced"
    );

    // A group with an entry can't be deleted; renamed, its entries go with it.
    assert!(remove_group(&dir, "Empty").is_err());
    rename_group(&dir, "Empty", "Doors").unwrap();
    assert!(find(&dir, "Doors/Door").is_ok());
    // Deleting moves the file to .deleted, which isn't listed.
    let gone = remove(&dir, "Doors/Door").unwrap();
    assert!(gone.is_file() && gone.starts_with(dir.join(".deleted")));
    remove_group(&dir, "Doors").unwrap();
    remove_group(&dir, "Signs").unwrap();
    let listing = list(&dir).unwrap();
    assert_eq!(listing.groups, Vec::<String>::new());
    assert_eq!(listing.items.len(), 1, "only Door at the top");
}

#[test]
fn a_dxf_in_the_folder_is_listed_and_read_as_drawn_shapes() {
    let dir = folder("dxf");
    std::fs::create_dir_all(dir.join("Parts")).unwrap();
    std::fs::copy(
        corpus("dxf/simple_part.dxf"),
        dir.join("Parts/simple_part.dxf"),
    )
    .unwrap();
    let item = find(&dir, "Parts/simple_part").unwrap();
    assert_eq!(item.kind, ItemKind::Dxf);
    let entry = read(&item).unwrap();
    assert!(!entry.shapes.is_empty());
    assert!(entry.shapes.iter().all(|s| s.is_drawn()));
    let done = insert(
        &empty_project(),
        &entry,
        "Plate",
        None,
        &InsertSize::AsSaved,
        None,
    )
    .unwrap();
    assert_eq!(done.shapes.len(), entry.shapes.len());
    assert!(!preview(&entry).is_empty());
}

#[test]
fn the_first_version_still_loads_and_a_newer_one_is_refused() {
    let entry = load_shape_entry(&corpus("shapes/v1/plate.chipload-shapes")).unwrap();
    assert_eq!(entry.schema_version, SHAPES_VERSION);
    assert_eq!(entry.texts.len(), 1);
    let done = insert(
        &empty_project(),
        &entry,
        "Plate",
        None,
        &InsertSize::AsSaved,
        None,
    )
    .unwrap();
    assert_eq!(done.shapes.len(), entry.shapes.len());

    let dir = folder("newer");
    let text = std::fs::read_to_string(corpus("shapes/v1/plate.chipload-shapes")).unwrap();
    let newer = text.replacen("\"schema_version\": 1", "\"schema_version\": 99", 1);
    std::fs::write(dir.join("new.chipload-shapes"), newer).unwrap();
    let err = read(&find(&dir, "new").unwrap()).unwrap_err().to_string();
    assert!(err.contains("99"), "{err}");
}
