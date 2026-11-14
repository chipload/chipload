// SPDX-License-Identifier: GPL-3.0-or-later
//! Ctrl+E like Save: the first export asks, then the same file is written again without
//! asking (docs/specs/fewer-clicks.md §5).
use super::*;

#[test]
fn the_export_file_is_remembered_per_project_and_written_again() {
    let settings = temp_dir("export-again-settings");
    let dir = temp_dir("export-again");
    let mut app = lower_part();
    app.set_settings_dir(Some(settings.clone()));
    let project_file = dir.join("lower.chipload");
    assert!(app.save_project(&project_file));
    assert!(app.generate_and_wait(), "{:?}", app.report);
    // Nothing remembered yet: Ctrl+E would ask.
    assert_eq!(app.remembered_export(), None);
    let path = dir.join("cut.nc");
    assert!(app.export_and_remember(&path));
    assert_eq!(app.remembered_export(), Some(path.clone()));
    // Kept in the settings, by the project's path.
    let saved =
        chipload_job::library::load_settings(&chipload_pipeline::library::settings_path(&settings))
            .unwrap();
    let project = app.file.path.as_ref().unwrap().display().to_string();
    assert_eq!(
        saved.export_of(&project),
        Some(path.display().to_string().as_str())
    );
    assert_eq!(saved.recent_projects[0], project);
    // The next Ctrl+E writes the same file without a dialog.
    std::fs::remove_file(&path).unwrap();
    app.export_when_ready();
    assert!(path.exists(), "written again without asking");
    // Another window with the same settings remembers it too.
    let mut other = ChiploadApp::default();
    other.set_settings_dir(Some(settings.clone()));
    assert!(other.open(&project_file));
    assert_eq!(other.remembered_export(), Some(path.clone()));
    // A folder that is gone: asks again.
    std::fs::remove_dir_all(&dir).unwrap();
    assert_eq!(app.remembered_export(), None);
    std::fs::remove_dir_all(&settings).unwrap();
}

#[test]
fn an_untitled_project_remembers_nothing() {
    let dir = temp_dir("export-untitled");
    let mut app = lower_part();
    assert!(app.file.path.is_none());
    assert!(app.generate_and_wait(), "{:?}", app.report);
    assert!(app.export_and_remember(&dir.join("a.nc")));
    assert_eq!(app.remembered_export(), None);
    assert!(app.prefs.exports.is_empty());
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn settings_keep_exports_only_for_recent_projects() {
    let mut settings = chipload_job::library::AppSettings::default();
    settings.set_export("/p/a.chipload", "/p/a.nc");
    assert_eq!(settings.export_of("/p/a.chipload"), Some("/p/a.nc"));
    assert!(settings.validate().is_empty());
    // Opened again: still remembered.
    settings.add_recent("/p/a.chipload");
    assert_eq!(settings.export_of("/p/a.chipload"), Some("/p/a.nc"));
    settings.remove_recent("/p/a.chipload");
    assert_eq!(settings.export_of("/p/a.chipload"), None);
    // Pushed off the end of the recent list: forgotten as well.
    settings.set_export("/p/old.chipload", "/p/old.nc");
    for i in 0..chipload_job::library::RECENT_MAX {
        settings.add_recent(&format!("/p/{i}.chipload"));
    }
    assert!(settings.exports.is_empty());
    // A hand-edited file naming a project not in the list is refused.
    settings
        .exports
        .insert("/p/nope.chipload".into(), "/p/nope.nc".into());
    assert_eq!(settings.validate().len(), 1);
}
