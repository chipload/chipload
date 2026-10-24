// SPDX-License-Identifier: GPL-3.0-or-later
//! The tool library and machine profiles (docs/specs/tool-library-and-machines.md).

// Integration tests may unwrap and panic: a failure is the test result.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};

use chipload_i18n::Message;
use chipload_job::ToolChange;
use chipload_job::library::{AppSettings, RECENT_MAX, Theme, load_settings, update_settings};
use chipload_job::library::{
    Cutting, LibraryError, LibraryTool, MachineLibrary, MachineProfile, ToolLibrary, UsedTool,
    export, import_into, import_plan, load_machines, load_tools, read_import, save_machines,
    save_tools, update_tools, use_tool, write_export,
};
use chipload_job::library::{Link, LinkEntry, fresh_uid, machine_link, tool_link, uid_for_name};
use chipload_job::{Job, Spindle, StockCorner, ToolType, Travel, WorkZero, load_job};

/// Messages as the command line writes them.
fn english(messages: &[Message]) -> Vec<String> {
    messages.iter().map(Message::english).collect()
}

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("chipload-library-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn project() -> Job {
    let corpus = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/corpus/jobs");
    load_job(&corpus.join("v7/adaptive.chipload")).unwrap()
}

fn upcut() -> LibraryTool {
    LibraryTool {
        name: "6 mm upcut".into(),
        uid: uid_for_name("6 mm upcut"),
        id: 1,
        kind: ToolType::Endmill,
        diameter: 6.0,
        flutes: Some(2),
        center_cutting: Some(true),
        rpm: Some(18000.0),
        cutting: Some(Cutting {
            feed: 1200.0,
            plunge_feed: 300.0,
            step_down: 2.0,
            stepover: 0.4,
        }),
        angle: None,
        tip_diameter: None,
    }
}

fn tools() -> ToolLibrary {
    ToolLibrary {
        default_tool: Some("6 MM UPCUT".into()),
        tools: vec![upcut()],
        ..ToolLibrary::default()
    }
}

fn shapeoko() -> MachineProfile {
    MachineProfile {
        name: "Shapeoko".into(),
        uid: uid_for_name("Shapeoko"),
        post: "grbl".into(),
        gcode_units: chipload_job::Units::Mm,
        safe_z: 10.0,
        spindle: Spindle::Cw,
        max_feed: Some(5000.0),
        travel: Some(Travel {
            x: 830.0,
            y: 830.0,
            z: 95.0,
        }),
        work_zero: WorkZero::Stock {
            corner: StockCorner::BottomLeft,
        },
        max_overcut: 0.3,
        tool_change: ToolChange::Files,
    }
}

fn machines() -> MachineLibrary {
    MachineLibrary {
        default_machine: Some("Shapeoko".into()),
        machines: vec![shapeoko()],
        ..MachineLibrary::default()
    }
}

#[test]
fn a_missing_file_is_an_empty_library() {
    let dir = temp_dir("missing");
    assert_eq!(
        load_tools(&dir.join("tools.json")).unwrap(),
        ToolLibrary::default()
    );
    assert_eq!(
        load_machines(&dir.join("machines.json")).unwrap(),
        MachineLibrary::default()
    );
}

#[test]
fn both_files_round_trip_with_every_field_written() {
    let dir = temp_dir("round-trip").join("not yet there");
    let tools_path = dir.join("tools.json");
    let machines_path = dir.join("machines.json");
    save_tools(&tools(), &tools_path).unwrap();
    save_machines(&machines(), &machines_path).unwrap();

    let mut loaded = load_tools(&tools_path).unwrap();
    assert!(
        loaded
            .written_by
            .as_deref()
            .unwrap()
            .starts_with("chipload ")
    );
    loaded.written_by = None;
    assert_eq!(loaded, tools());
    let mut loaded = load_machines(&machines_path).unwrap();
    loaded.written_by = None;
    assert_eq!(loaded, machines());

    let text = std::fs::read_to_string(&tools_path).unwrap();
    for field in [
        "\"schema_version\": 3",
        "\"uid\": \"",
        "\"center_cutting\": true",
        "\"angle\": null",
        "\"stepover\": 0.4",
    ] {
        assert!(text.contains(field), "{field} in {text}");
    }
    let text = std::fs::read_to_string(&machines_path).unwrap();
    assert!(text.contains("\"max_overcut\": 0.3"), "{text}");
    assert!(text.contains("\"corner\": \"bottom_left\""), "{text}");
    assert!(text.ends_with("}\n"));
}

#[test]
fn saving_keeps_the_previous_file_as_bak() {
    let dir = temp_dir("bak");
    let path = dir.join("tools.json");
    save_tools(&tools(), &path).unwrap();
    let first = std::fs::read_to_string(&path).unwrap();
    let mut changed = tools();
    changed.tools[0].diameter = 8.0;
    save_tools(&changed, &path).unwrap();
    assert_eq!(
        std::fs::read_to_string(dir.join("tools.json.bak")).unwrap(),
        first
    );
    assert_eq!(load_tools(&path).unwrap().tools[0].diameter, 8.0);
    assert!(!dir.join("tools.json.tmp").exists());
}

#[test]
fn a_failed_save_leaves_the_old_file_intact() {
    let dir = temp_dir("failed-save");
    let path = dir.join("tools.json");
    save_tools(&tools(), &path).unwrap();
    let before = std::fs::read_to_string(&path).unwrap();
    // The temporary file can't be written where a folder of that name is.
    std::fs::create_dir(dir.join("tools.json.tmp")).unwrap();
    let mut changed = tools();
    changed.tools[0].diameter = 8.0;
    let err = save_tools(&changed, &path).unwrap_err();
    assert!(matches!(err, LibraryError::Unwritable { .. }), "{err}");
    assert_eq!(std::fs::read_to_string(&path).unwrap(), before);
}

#[test]
fn unknown_versions_and_broken_files_are_refused() {
    let dir = temp_dir("versions");
    let path = dir.join("tools.json");
    for (text, found) in [
        (r#"{"schema_version": 4, "tools": []}"#, "4"),
        (r#"{"schema_version": 0, "tools": []}"#, "0"),
        (r#"{"tools": []}"#, "(missing)"),
        (r#"{"schema_version": "1", "tools": []}"#, "\"1\""),
    ] {
        std::fs::write(&path, text).unwrap();
        match load_tools(&path).unwrap_err() {
            LibraryError::UnsupportedVersion {
                found: f, current, ..
            } => {
                assert_eq!(f, found);
                assert_eq!(current, 3);
            }
            other => panic!("{text}: {other}"),
        }
    }
    std::fs::write(&path, "{not json").unwrap();
    assert!(matches!(
        load_tools(&path).unwrap_err(),
        LibraryError::Unreadable { .. }
    ));
    std::fs::write(&path, r#"{"schema_version": 1, "tools": [], "colour": 1}"#).unwrap();
    assert!(matches!(
        load_tools(&path).unwrap_err(),
        LibraryError::Invalid { .. }
    ));
}

#[test]
fn a_version_1_library_loads_with_v_bits_of_unknown_angle() {
    // docs/specs/v-carve.md §9: tools 1 -> 2 adds nothing; the angle is never guessed.
    let dir = temp_dir("tools-v1");
    let path = dir.join("tools.json");
    let v1 = r#"{"schema_version": 1, "written_by": "chipload 0.10.1", "default_tool": null,
        "tools": [{"name": "V 60", "id": 3, "type": "vbit", "diameter": 12.0, "flutes": 2,
                   "center_cutting": null, "rpm": 18000.0, "cutting": null}]}"#;
    std::fs::write(&path, v1).unwrap();
    let library = load_tools(&path).unwrap();
    assert_eq!(library.schema_version, 3);
    let tool = &library.tools[0];
    assert_eq!(tool.kind, ToolType::Vbit);
    assert_eq!((tool.angle, tool.tip_diameter), (None, None));
}

#[test]
fn a_version_1_machine_library_writes_one_file_per_tool() {
    // docs/specs/tool-per-operation.md §3.3: machines 1 -> 2 adds the tool change setting.
    let dir = temp_dir("machines-v1");
    let path = dir.join("machines.json");
    let v1 = r#"{"schema_version": 1, "written_by": "chipload 0.11.0",
        "default_machine": "Shapeoko",
        "machines": [{"name": "Shapeoko", "post": "grbl", "safe_z": 10.0, "spindle": "cw",
                      "max_feed": 5000.0, "travel": {"x": 830.0, "y": 830.0, "z": 95.0},
                      "work_zero": {"at": "stock", "corner": "bottom_left"},
                      "max_overcut": 0.3}]}"#;
    std::fs::write(&path, v1).unwrap();
    let library = load_machines(&path).unwrap();
    assert_eq!(library.schema_version, 4);
    assert_eq!(library.machines, [shapeoko()]);
    assert_eq!(library.machines[0].tool_change, ToolChange::Files);
    // Version 2 needs it.
    let v2 = v1.replace("\"schema_version\": 1", "\"schema_version\": 2");
    std::fs::write(&path, v2).unwrap();
    assert!(load_machines(&path).is_err());
}

#[test]
fn a_broken_library_is_never_overwritten() {
    let dir = temp_dir("broken");
    let path = dir.join("tools.json");
    let newer = r#"{"schema_version": 4, "tools": [], "from_the_future": true}"#;
    std::fs::write(&path, newer).unwrap();
    let err = update_tools(&path, |library| {
        library.tools.push(upcut());
        Ok(())
    })
    .unwrap_err();
    assert!(
        matches!(err, LibraryError::UnsupportedVersion { .. }),
        "{err}"
    );
    assert_eq!(std::fs::read_to_string(&path).unwrap(), newer);
}

#[test]
fn update_changes_the_file_as_it_is_now() {
    let dir = temp_dir("update");
    let path = dir.join("tools.json");
    save_tools(&tools(), &path).unwrap();
    // Another window adds a tool after this one loaded the library.
    let mut other = load_tools(&path).unwrap();
    other.tools.push(LibraryTool {
        name: "3 mm".into(),
        uid: uid_for_name("3 mm"),
        diameter: 3.0,
        cutting: None,
        ..upcut()
    });
    save_tools(&other, &path).unwrap();
    update_tools(&path, |library| {
        library.tools[0].rpm = Some(20000.0);
        Ok(())
    })
    .unwrap();
    let now = load_tools(&path).unwrap();
    assert_eq!(now.tools.len(), 2);
    assert_eq!(now.tools[0].rpm, Some(20000.0));
}

#[test]
fn library_rules() {
    let problems = |library: &ToolLibrary| library.validate().join("; ");
    assert_eq!(problems(&tools()), "");

    let mut twice = tools();
    twice.tools.push(LibraryTool {
        name: "6 MM Upcut".into(),
        ..upcut()
    });
    assert!(problems(&twice).contains("tools.1.name: \"6 MM Upcut\" is used twice"));

    let mut bad = tools();
    bad.tools[0].name = " padded".into();
    bad.tools[0].diameter = 0.0;
    bad.tools[0].cutting = Some(Cutting {
        stepover: 1.5,
        feed: -1.0,
        ..upcut().cutting.unwrap()
    });
    bad.default_tool = Some("nothing".into());
    let text = problems(&bad);
    for expected in [
        "tools.0.name: must not start or end with spaces",
        "tools.0.diameter: must be greater than 0, got 0",
        "tools.0.cutting.feed: must be greater than 0, got -1",
        "tools.0.cutting.stepover: must be at most 1",
        "default_tool: \"nothing\" is not in tools",
    ] {
        assert!(text.contains(expected), "{expected} in {text}");
    }
    let mut empty = tools();
    empty.tools[0].name = String::new();
    assert!(problems(&empty).contains("tools.0.name: must not be empty"));

    let mut bad = machines();
    bad.machines[0].max_overcut = -1.0;
    bad.machines[0].max_feed = Some(0.0);
    bad.machines[0].travel = Some(Travel {
        x: 0.0,
        y: 1.0,
        z: 1.0,
    });
    bad.machines[0].post = " ".into();
    let text = bad.validate().join("; ");
    for expected in [
        "machines.0.max_overcut: must be 0 or more, got -1",
        "machines.0.max_feed: must be greater than 0, got 0",
        "machines.0.travel.x: must be greater than 0, got 0",
        "machines.0.post: must name a post-processor",
    ] {
        assert!(text.contains(expected), "{expected} in {text}");
    }
}

#[test]
fn names_are_found_ignoring_case_and_removing_the_default_clears_it() {
    let mut library = tools();
    assert_eq!(library.default_entry().unwrap().name, "6 mm upcut");
    assert!(library.find("6 MM UPCUT").is_some());
    library.remove("6 mm UPCUT").unwrap();
    assert!(library.tools.is_empty() && library.default_tool.is_none());
    assert_eq!(
        library.remove("6 mm upcut").unwrap_err().to_string(),
        "no tool named \"6 mm upcut\""
    );
}

#[test]
fn using_a_library_tool_reuses_keeps_or_renumbers_ids() {
    let mut job = project();
    job.tools.clear();
    // Id 1 is free: kept.
    let used = use_tool(&mut job, &upcut());
    assert_eq!(used, UsedTool { id: 1, added: true });
    assert_eq!(job.tools[0], upcut().tool(1));
    assert_eq!(job.tools[0].name.as_deref(), Some("6 mm upcut"));
    // The same tool again: reused.
    assert_eq!(
        use_tool(&mut job, &upcut()),
        UsedTool {
            id: 1,
            added: false
        }
    );
    assert_eq!(used.describe(1), "tool 1 (added)");
    // Changed values under the same name: a new copy, and id 1 is taken.
    let mut bigger = upcut();
    bigger.diameter = 8.0;
    let used = use_tool(&mut job, &bigger);
    assert_eq!(used, UsedTool { id: 2, added: true });
    assert_eq!(used.describe(1), "tool 2 (added as 2; tool 1 was taken)");
    assert_eq!(job.tools.len(), 2);
}

#[test]
fn a_library_tool_lists_what_it_changes() {
    let entry = upcut();
    let mut tool = entry.tool(4);
    assert!(entry.changes(&tool).is_empty());
    tool.name = None;
    assert!(entry.changes(&tool).is_empty(), "the name is only a note");
    tool.diameter = 8.0;
    tool.center_cutting = None;
    tool.rpm = None;
    assert_eq!(
        english(&entry.changes(&tool)),
        [
            "diameter 8 mm -> 6 mm",
            "centre cutting unknown -> yes",
            "rpm none -> 18000"
        ]
    );
}

#[test]
fn a_machine_profile_lists_what_it_changes() {
    let mut job = project();
    job.machine.max_feed = None;
    job.machine.travel = None;
    let profile = shapeoko();
    let messages = profile.changes(&job);
    let changes = english(&messages);
    assert!(
        changes.contains(&"travel none -> X830 Y830 Z95 mm".to_string()),
        "{changes:?}"
    );
    assert!(
        changes.contains(&"max feed none -> 5000 mm/min".to_string()),
        "{changes:?}"
    );
    assert!(
        changes.contains(&"max overcut 0.5 mm -> 0.3 mm".to_string()),
        "{changes:?}"
    );

    assert_eq!(profile.apply(&mut job), messages);
    assert_eq!(job.machine.name.as_deref(), Some("Shapeoko"));
    assert_eq!(job.stock.max_overcut, 0.3);
    assert!(profile.changes(&job).is_empty());
    // A profile saved from the project gives the project back.
    assert_eq!(
        MachineProfile::from_job("Shapeoko", &uid_for_name("Shapeoko"), &job),
        profile
    );
}

#[test]
fn names_in_a_project_round_trip_and_old_files_have_none() {
    let mut job = project();
    assert!(job.machine.name.is_none() && job.tools.iter().all(|t| t.name.is_none()));
    shapeoko().apply(&mut job);
    use_tool(&mut job, &upcut());
    let text = chipload_job::dump_job(&job).unwrap();
    assert!(text.contains("\"name\": \"Shapeoko\""), "{text}");
    let data = serde_json::from_str(&text).unwrap();
    let again = chipload_job::parse_job(data, "saved").unwrap();
    assert_eq!(again.machine, job.machine);
    assert_eq!(again.tools, job.tools);
}

#[test]
fn recent_projects_keep_order_without_duplicates_up_to_the_limit() {
    let dir = temp_dir("recent");
    let path = dir.join("settings.json");
    assert_eq!(load_settings(&path).unwrap(), AppSettings::default());
    for i in 0..12 {
        update_settings(&path, |s| {
            s.add_recent(&format!("/p/{i}.chipload"));
            Ok(())
        })
        .unwrap();
    }
    update_settings(&path, |s| {
        s.add_recent("/p/5.chipload");
        Ok(())
    })
    .unwrap();
    let settings = load_settings(&path).unwrap();
    assert_eq!(settings.recent_projects.len(), RECENT_MAX);
    assert_eq!(settings.recent_projects[0], "/p/5.chipload");
    assert_eq!(settings.recent_projects[1], "/p/11.chipload");
    assert_eq!(
        settings
            .recent_projects
            .iter()
            .filter(|p| *p == "/p/5.chipload")
            .count(),
        1
    );
    assert!(
        !settings
            .recent_projects
            .contains(&"/p/1.chipload".to_string())
    );
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains("\"schema_version\": 3"), "{text}");

    std::fs::write(
        &path,
        r#"{"schema_version": 1, "recent_projects": ["/a", "/a"]}"#,
    )
    .unwrap();
    let err = load_settings(&path).unwrap_err().to_string();
    assert!(err.contains("listed twice"), "{err}");
    std::fs::write(
        &path,
        r#"{"schema_version": 4, "recent_projects": [], "theme": "system", "auto_generate": true}"#,
    )
    .unwrap();
    assert!(matches!(
        load_settings(&path).unwrap_err(),
        LibraryError::UnsupportedVersion { .. }
    ));
}

/// The frozen settings of each older version (never edited; add a new one instead).
fn settings_sample(version: i64) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(format!(
        "../../tests/corpus/settings/v{version}/settings.json"
    ))
}

#[test]
fn version_1_settings_migrate_to_follow_the_system_theme() {
    let dir = temp_dir("settings-v1");
    let path = dir.join("settings.json");
    std::fs::copy(settings_sample(1), &path).unwrap();
    let settings = load_settings(&path).unwrap();
    assert_eq!(settings.theme, Theme::System);
    assert_eq!(
        settings.recent_projects,
        ["/home/ann/cnc/sign.chipload", "/home/ann/cnc/box.chipload"]
    );
    // Saving writes the current version, every field explicit.
    update_settings(&path, |s| {
        s.theme = Theme::Dark;
        Ok(())
    })
    .unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains("\"schema_version\": 3"), "{text}");
    assert!(text.contains("\"theme\": \"dark\""), "{text}");
    assert!(text.contains("\"auto_generate\": true"), "{text}");
    assert!(text.contains("\"setup_sheet\": true"), "{text}");
    assert_eq!(load_settings(&path).unwrap().theme, Theme::Dark);
}

#[test]
fn settings_without_the_setup_sheet_write_one() {
    // docs/specs/setup-sheet.md §4: an optional field, on when missing.
    let dir = temp_dir("settings-sheet");
    let path = dir.join("settings.json");
    let base =
        r#""schema_version": 3, "recent_projects": [], "theme": "system", "auto_generate": true"#;
    std::fs::write(&path, format!("{{{base}}}")).unwrap();
    assert!(load_settings(&path).unwrap().setup_sheet);
    std::fs::write(&path, format!("{{{base}, \"setup_sheet\": false}}")).unwrap();
    assert!(!load_settings(&path).unwrap().setup_sheet);
}

#[test]
fn settings_without_the_grid_show_it() {
    // docs/specs/browser-and-layout.md §6b: an optional field, on when missing.
    let dir = temp_dir("settings-grid");
    let path = dir.join("settings.json");
    let base =
        r#""schema_version": 3, "recent_projects": [], "theme": "system", "auto_generate": true"#;
    std::fs::write(&path, format!("{{{base}}}")).unwrap();
    assert!(load_settings(&path).unwrap().show_grid);
    std::fs::write(&path, format!("{{{base}, \"show_grid\": false}}")).unwrap();
    assert!(!load_settings(&path).unwrap().show_grid);
}

#[test]
fn version_2_settings_migrate_to_update_the_toolpath_by_itself() {
    let dir = temp_dir("settings-v2");
    let path = dir.join("settings.json");
    std::fs::copy(settings_sample(2), &path).unwrap();
    let settings = load_settings(&path).unwrap();
    assert!(settings.auto_generate);
    assert_eq!(settings.theme, Theme::Dark);
    assert_eq!(settings.recent_projects, ["/home/ann/cnc/sign.chipload"]);
}

#[test]
fn settings_newer_than_known_or_without_a_theme_are_refused() {
    let dir = temp_dir("settings-refused");
    let path = dir.join("settings.json");
    std::fs::write(
        &path,
        r#"{"schema_version": 4, "recent_projects": [], "theme": "system", "auto_generate": true}"#,
    )
    .unwrap();
    let err = load_settings(&path).unwrap_err().to_string();
    assert!(err.contains("schema_version 4 is not supported"), "{err}");
    // Version 2 needs its theme: a missing field is not guessed.
    std::fs::write(&path, r#"{"schema_version": 2, "recent_projects": []}"#).unwrap();
    assert!(load_settings(&path).is_err());
    // Version 3 needs its auto_generate.
    std::fs::write(
        &path,
        r#"{"schema_version": 3, "recent_projects": [], "theme": "system"}"#,
    )
    .unwrap();
    assert!(load_settings(&path).is_err());
}

// Import and export (docs/specs/library-import-export.md).

fn downcut() -> LibraryTool {
    LibraryTool {
        name: "3 mm downcut".into(),
        uid: uid_for_name("3 mm downcut"),
        id: 2,
        diameter: 3.0,
        cutting: None,
        ..upcut()
    }
}

#[test]
fn export_then_import_into_an_empty_library_gives_the_same_library() {
    let dir = temp_dir("export-round-trip");
    let mut library = tools();
    library.tools.push(downcut());
    let path = dir.join("backup.json");
    write_export(&export(&library, &[]).unwrap(), &path).unwrap();
    let mut empty = ToolLibrary::default();
    let plan = import_into(
        &mut empty,
        &read_import::<ToolLibrary>(&path).unwrap(),
        false,
    );
    assert_eq!(plan.added, ["6 mm upcut", "3 mm downcut"]);
    assert_eq!(plan.default.as_deref(), Some("6 MM UPCUT"));
    // The default takes the entry's spelling.
    assert_eq!(empty.default_tool.as_deref(), Some("6 mm upcut"));
    assert_eq!(empty.tools, library.tools);

    let path = dir.join("machines.json");
    write_export(&export(&machines(), &[]).unwrap(), &path).unwrap();
    let mut empty = MachineLibrary::default();
    import_into(&mut empty, &read_import(&path).unwrap(), false);
    assert_eq!(empty, machines());
    // An export is not a library save: no backup next to it.
    write_export(&export(&machines(), &[]).unwrap(), &path).unwrap();
    assert!(!dir.join("machines.json.bak").exists());
}

#[test]
fn export_by_name_keeps_the_default_only_when_exported() {
    let mut library = tools();
    library.tools.push(downcut());
    let some = export(&library, &["3 MM DOWNCUT".into()]).unwrap();
    assert_eq!(some.tools, [downcut()]);
    assert_eq!(some.default_tool, None);
    let some = export(&library, &["6 mm upcut".into(), "6 MM UPCUT".into()]).unwrap();
    assert_eq!(some.tools, [upcut()]);
    assert!(some.default_tool.is_some());
    let err = export(&library, &["2 mm".into()]).unwrap_err();
    assert!(err.to_string().contains("no tool named \"2 mm\""), "{err}");
}

#[test]
fn import_adds_new_keeps_or_replaces_differing_and_skips_same() {
    let mut mine = tools();
    mine.tools.push(downcut());
    let mut theirs = ToolLibrary::default();
    let mut changed = downcut();
    changed.name = "3 MM DOWNCUT".into();
    changed.diameter = 3.175;
    let mut same = upcut();
    same.name = "6 MM Upcut".into();
    let mut new = downcut();
    new.name = "1 mm".into();
    new.uid = uid_for_name("1 mm");
    theirs.tools = vec![new.clone(), changed, same];
    theirs.default_tool = Some("1 mm".into());

    let mut kept = mine.clone();
    let plan = import_into(&mut kept, &theirs, false);
    assert_eq!(plan.added, ["1 mm"]);
    assert_eq!(plan.same, ["6 mm upcut"]);
    assert_eq!(plan.differing.len(), 1);
    assert_eq!(plan.differing[0].0, "3 mm downcut");
    assert_eq!(english(&plan.differing[0].1), ["diameter 3 mm -> 3.175 mm"]);
    // The library has a default: it stays.
    assert_eq!(plan.default, None);
    assert_eq!(kept.default_tool, mine.default_tool);
    assert_eq!(kept.tools, [upcut(), downcut(), new.clone()]);

    let mut replaced = mine.clone();
    assert_eq!(import_into(&mut replaced, &theirs, true), plan);
    let mut wider = downcut();
    wider.diameter = 3.175;
    assert_eq!(replaced.tools, [upcut(), wider, new]);
}

#[test]
fn differing_tools_and_machines_say_what_differs() {
    let mut other = upcut();
    other.id = 4;
    other.cutting = None;
    let plan = import_plan(
        &tools(),
        &ToolLibrary {
            tools: vec![other],
            ..ToolLibrary::default()
        },
    );
    assert_eq!(plan.differing[0].1.len(), 2, "{plan:?}");
    assert_eq!(plan.differing[0].1[0].english(), "id 1 -> 4");
    assert!(
        plan.differing[0].1[1]
            .english()
            .starts_with("cutting values feed 1200")
    );

    let mut other = shapeoko();
    other.safe_z = 5.0;
    other.max_overcut = 0.5;
    let plan = import_plan(
        &machines(),
        &MachineLibrary {
            machines: vec![other],
            ..MachineLibrary::default()
        },
    );
    assert_eq!(
        english(&plan.differing[0].1),
        ["safe Z 10 mm -> 5 mm", "max overcut 0.3 mm -> 0.5 mm"]
    );
}

#[test]
fn import_refuses_missing_newer_broken_and_wrong_kind_files() {
    let dir = temp_dir("import-refused");
    let missing = read_import::<ToolLibrary>(&dir.join("nothing.json")).unwrap_err();
    assert!(
        matches!(missing, LibraryError::Unreadable { .. }),
        "{missing}"
    );

    let newer = dir.join("newer.json");
    std::fs::write(&newer, r#"{"schema_version": 99, "tools": []}"#).unwrap();
    let err = read_import::<ToolLibrary>(&newer).unwrap_err();
    assert!(
        matches!(err, LibraryError::UnsupportedVersion { .. }),
        "{err}"
    );

    let broken = dir.join("broken.json");
    let mut twice = tools();
    twice.tools.push(upcut());
    twice.default_tool = None;
    std::fs::write(&broken, serde_json::to_string(&twice).unwrap()).unwrap();
    let err = read_import::<ToolLibrary>(&broken).unwrap_err();
    assert!(err.to_string().contains("used twice"), "{err}");

    let machines_file = dir.join("machines.json");
    save_machines(&machines(), &machines_file).unwrap();
    let err = read_import::<ToolLibrary>(&machines_file).unwrap_err();
    assert!(err.to_string().contains("import it as machines"), "{err}");
    let tools_file = dir.join("tools.json");
    save_tools(&tools(), &tools_file).unwrap();
    let err = read_import::<MachineLibrary>(&tools_file).unwrap_err();
    assert!(err.to_string().contains("import it as tools"), "{err}");
}

#[test]
fn a_version_1_file_imports_migrated() {
    let dir = temp_dir("import-v1");
    let path = dir.join("old.json");
    let v1 = r#"{"schema_version": 1, "written_by": "chipload 0.10.1", "default_machine": null,
        "machines": [{"name": "Old", "post": "grbl", "safe_z": 5.0, "spindle": "cw",
                      "max_feed": null, "travel": null, "work_zero": {"at": "drawing_origin"},
                      "max_overcut": 0.2}]}"#;
    std::fs::write(&path, v1).unwrap();
    let file = read_import::<MachineLibrary>(&path).unwrap();
    let mut library = MachineLibrary::default();
    import_into(&mut library, &file, false);
    assert_eq!(library.machines[0].tool_change, ToolChange::Files);
}

// Recovery notes (docs/specs/autosave-and-backup.md §2).

#[test]
fn a_recovery_note_round_trips_and_odd_ones_are_refused() {
    use chipload_job::library::{RecoveryNote, load_recovery_note, save_recovery_note};
    let dir = temp_dir("recovery-note");
    let path = dir.join("note.json");
    assert!(
        load_recovery_note(&path).is_err(),
        "a missing note is an error"
    );
    let note = RecoveryNote {
        project_path: Some("/home/ann/cnc/sign.chipload".into()),
        ..RecoveryNote::default()
    };
    save_recovery_note(&note, &path).unwrap();
    let read = load_recovery_note(&path).unwrap();
    assert_eq!(read.schema_version, 1);
    assert!(read.written_by.unwrap().starts_with("chipload "));
    assert_eq!(read.project_path, note.project_path);
    std::fs::write(&path, r#"{"schema_version": 2, "project_path": null}"#).unwrap();
    assert!(matches!(
        load_recovery_note(&path).unwrap_err(),
        LibraryError::UnsupportedVersion { .. }
    ));
    std::fs::write(&path, r#"{"schema_version": 1}"#).unwrap();
    assert!(
        load_recovery_note(&path).is_err(),
        "project_path is required"
    );
    std::fs::write(&path, "{\"schema_version\": 1, \"project_path\"").unwrap();
    assert!(load_recovery_note(&path).is_err());
}

// docs/specs/library-in-browser.md §2, §3.

#[test]
fn a_uid_comes_from_the_name_and_skips_taken_ones() {
    // FNV-1a 64 of the lower-cased name: frozen, migrations depend on it.
    assert_eq!(uid_for_name(""), "cbf29ce484222325");
    assert_eq!(uid_for_name("a"), "af63dc4c8601ec8c");
    assert_eq!(uid_for_name("6 MM Upcut"), uid_for_name("6 mm upcut"));
    let first = uid_for_name("x");
    let taken = [first.clone()];
    let next = fresh_uid("x", taken.iter().map(String::as_str));
    assert_eq!(next, uid_for_name("x#2"));
    assert_eq!(
        fresh_uid("y", taken.iter().map(String::as_str)),
        uid_for_name("y")
    );
}

#[test]
fn old_libraries_get_the_uid_of_each_name() {
    let dir = temp_dir("uids");
    let tools = dir.join("tools.json");
    std::fs::write(
        &tools,
        r#"{"schema_version": 2, "tools": [
            {"name": "6 mm upcut", "id": 1, "type": "endmill", "diameter": 6.0},
            {"name": "V 60", "id": 2, "type": "vbit", "diameter": 12.0}]}"#,
    )
    .unwrap();
    let library = load_tools(&tools).unwrap();
    assert_eq!(library.tools[0].uid, uid_for_name("6 mm upcut"));
    assert_eq!(library.tools[1].uid, uid_for_name("V 60"));
    let machines = dir.join("machines.json");
    std::fs::write(
        &machines,
        r#"{"schema_version": 2, "machines": [{"name": "Shapeoko", "post": "grbl",
            "safe_z": 10.0, "spindle": "cw", "work_zero": {"at": "drawing_origin"},
            "max_overcut": 0.3, "tool_change": "files"}]}"#,
    )
    .unwrap();
    let library = load_machines(&machines).unwrap();
    assert_eq!(library.machines[0].uid, uid_for_name("Shapeoko"));
}

#[test]
fn bad_or_twice_used_uids_are_refused() {
    let mut library = tools();
    let mut other = upcut();
    other.name = "Other".into();
    library.tools.push(other);
    assert!(
        library
            .validate()
            .iter()
            .any(|p| p.starts_with("tools.1.uid:") && p.contains("used twice")),
        "{:?}",
        library.validate()
    );
    library.tools[1].uid = "XYZ".into();
    assert!(
        library
            .validate()
            .iter()
            .any(|p| p.contains("16 lowercase hex digits"))
    );
}

#[test]
fn a_project_tool_links_by_uid_and_survives_a_rename() {
    let mut job = project();
    job.tools.clear();
    let mut library = tools();
    let tool = upcut().tool(1);
    job.tools.push(tool.clone());
    assert_eq!(tool_link(&library, &tool), Link::Same("6 mm upcut".into()));
    // Renamed in the library: still linked, by its new name.
    library.tools[0].name = "Upcut 6".into();
    assert_eq!(tool_link(&library, &tool), Link::Same("Upcut 6".into()));
    // Other values in the library.
    library.tools[0].diameter = 8.0;
    let Link::Differs(name, changes) = tool_link(&library, &tool) else {
        panic!("not differing");
    };
    assert_eq!(name, "Upcut 6");
    assert_eq!(english(&changes), ["diameter 6 mm -> 8 mm"]);
    assert_eq!(
        tool_link(&library, &tool).text(LinkEntry::Tool),
        "differs from library tool \"Upcut 6\": diameter 6 mm -> 8 mm"
    );
    // Gone from the library.
    library.tools.clear();
    assert_eq!(
        tool_link(&library, &tool),
        Link::Missing(Some("6 mm upcut".into()))
    );
    // Never from the library.
    let mut own = tool;
    own.library_uid = None;
    assert_eq!(tool_link(&library, &own), Link::None);
}

#[test]
fn the_machine_links_by_uid() {
    let mut job = project();
    let mut library = machines();
    assert_eq!(machine_link(&library, &job), Link::None);
    shapeoko().apply(&mut job);
    assert_eq!(machine_link(&library, &job), Link::Same("Shapeoko".into()));
    // The job values (safe Z, work zero, max overcut) never count
    // (docs/specs/linked-machine.md §2).
    job.machine.safe_z = 20.0;
    job.machine.work_zero = WorkZero::DrawingOrigin {};
    job.stock.max_overcut = 0.1;
    assert_eq!(machine_link(&library, &job), Link::Same("Shapeoko".into()));
    library.machines[0].spindle = Spindle::Ccw;
    let Link::Differs(name, changes) = machine_link(&library, &job) else {
        panic!("not differing");
    };
    assert_eq!(name, "Shapeoko");
    assert_eq!(english(&changes), ["spindle cw -> ccw"]);
    library.machines.clear();
    assert_eq!(machine_link(&library, &job).state(), "missing");
}

#[test]
fn import_keeps_uids_and_gives_a_clashing_one_a_new_uid() {
    let mut library = tools();
    // A file tool of another name with the same uid: added with a fresh uid.
    let mut clash = upcut();
    clash.name = "Clash".into();
    // A file tool of the same name with another uid and other values: replaced, keeping
    // the library's uid.
    let mut same = upcut();
    same.uid = uid_for_name("somewhere else");
    same.rpm = Some(12000.0);
    let file = ToolLibrary {
        tools: vec![clash, same],
        ..ToolLibrary::default()
    };
    import_into(&mut library, &file, true);
    assert_eq!(library.tools.len(), 2);
    assert_eq!(library.tools[0].uid, uid_for_name("6 mm upcut"));
    assert_eq!(library.tools[0].rpm, Some(12000.0));
    assert_eq!(library.tools[1].name, "Clash");
    assert_ne!(library.tools[1].uid, library.tools[0].uid);
    assert!(library.validate().is_empty(), "{:?}", library.validate());
}
