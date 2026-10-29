// SPDX-License-Identifier: GPL-3.0-or-later
//! One part's G-code from its right-click menu (docs/specs/part-export.md).
use super::*;

/// What `run_job` gives for the project with only part `id` cut.
fn only(app: &ChiploadApp, id: u64) -> Vec<String> {
    let job = app.edit.as_ref().unwrap().to_job().unwrap();
    let (job, _) = chipload_pipeline::parts::only_parts(&job, &[id]).unwrap();
    let result = chipload_pipeline::run_job(&job).unwrap();
    result.files.iter().map(|f| f.gcode.clone()).collect()
}

#[test]
fn a_part_exports_alone_and_the_project_is_unchanged() {
    let mut app = two_parts();
    app.setup_sheet = true;
    let project = app.edit.clone();
    let dirty = app.dirty();
    let dir = temp_dir("part-export");
    // Part 2 ("part B") is switched off: exported all the same (decision 1).
    let edit = app.edit.as_mut().unwrap();
    edit.settings.parts[1].enabled = false;
    app.commit();
    assert_eq!(
        app.default_part_export_path(2, "part B")
            .file_name()
            .unwrap()
            .to_string_lossy(),
        "two_parts-part-B.nc"
    );
    let path = dir.join("b.nc");
    assert!(!app.export_part(2, &path), "planned first");
    assert!(app.generating());
    assert!(app.wait_for_generation(), "{:?}", app.report);
    assert_eq!(app.result_part, Some(2));
    let expected = only(&app, 2);
    assert_eq!(expected.len(), 1);
    assert_eq!(std::fs::read_to_string(&path).unwrap(), expected[0]);
    let sheet = std::fs::read_to_string(dir.join("b-sheet.html")).unwrap();
    assert!(sheet.contains("two_parts.chipload, part part B"), "{sheet}");
    // The part's toolpath is shown now: written at once.
    std::fs::remove_file(&path).unwrap();
    assert!(app.export_part(2, &path));
    assert_eq!(std::fs::read_to_string(&path).unwrap(), expected[0]);
    // Only the switch made above changed the project.
    app.edit.as_mut().unwrap().settings.parts[1].enabled = true;
    app.commit();
    assert_eq!(app.edit, project);
    assert_eq!(app.dirty(), dirty);
    std::fs::remove_dir_all(&dir).unwrap();
}
