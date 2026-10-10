// SPDX-License-Identifier: GPL-3.0-or-later
//! The tool library and machine profiles (docs/specs/tool-library-and-machines.md).

// Integration tests may unwrap and panic: a failure is the test result.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};

use chipload_job::library::{AppSettings, RECENT_MAX, load_settings, update_settings};
use chipload_job::library::{
    Cutting, LibraryError, LibraryTool, MachineLibrary, MachineProfile, ToolLibrary, UsedTool,
    load_machines, load_tools, save_machines, save_tools, update_tools, use_tool,
};
use chipload_job::{Job, Spindle, StockCorner, ToolType, Travel, WorkZero, load_job};

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
        post: "grbl".into(),
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
        "\"schema_version\": 1",
        "\"center_cutting\": true",
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
        (r#"{"schema_version": 2, "tools": []}"#, "2"),
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
                assert_eq!(current, 1);
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
fn a_broken_library_is_never_overwritten() {
    let dir = temp_dir("broken");
    let path = dir.join("tools.json");
    let newer = r#"{"schema_version": 2, "tools": [], "from_the_future": true}"#;
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
        entry.changes(&tool),
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
    let changes = profile.changes(&job);
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

    assert_eq!(profile.apply(&mut job), changes);
    assert_eq!(job.machine.name.as_deref(), Some("Shapeoko"));
    assert_eq!(job.stock.max_overcut, 0.3);
    assert!(profile.changes(&job).is_empty());
    // A profile saved from the project gives the project back.
    assert_eq!(MachineProfile::from_job("Shapeoko", &job), profile);
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
    assert!(text.contains("\"schema_version\": 1"), "{text}");

    std::fs::write(
        &path,
        r#"{"schema_version": 1, "recent_projects": ["/a", "/a"]}"#,
    )
    .unwrap();
    let err = load_settings(&path).unwrap_err().to_string();
    assert!(err.contains("listed twice"), "{err}");
    std::fs::write(&path, r#"{"schema_version": 2, "recent_projects": []}"#).unwrap();
    assert!(matches!(
        load_settings(&path).unwrap_err(),
        LibraryError::UnsupportedVersion { .. }
    ));
}
