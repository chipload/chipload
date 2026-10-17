// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

mod library {
    use super::*;
    use chipload_job::ToolChange;
    use chipload_job::library::{
        Cutting, LibraryTool, MachineLibrary, MachineProfile, ToolLibrary, load_machines,
        load_tools, save_machines, save_tools, uid_for_name,
    };

    fn settings(name: &str) -> PathBuf {
        let dir = temp_dir(&format!("settings-{name}"));
        let _ = std::fs::remove_file(dir.join("tools.json"));
        let _ = std::fs::remove_file(dir.join("machines.json"));
        dir
    }

    fn upcut() -> LibraryTool {
        LibraryTool {
            name: "3 mm upcut".into(),
            uid: uid_for_name("3 mm upcut"),
            id: 2,
            kind: ToolType::Endmill,
            diameter: 3.0,
            flutes: Some(2),
            center_cutting: Some(true),
            rpm: Some(20000.0),
            cutting: Some(Cutting {
                feed: 800.0,
                plunge_feed: 200.0,
                step_down: 0.75,
                stepover: 0.35,
            }),
            angle: None,
            tip_diameter: None,
        }
    }

    fn shapeoko() -> MachineProfile {
        MachineProfile {
            name: "Shapeoko".into(),
            uid: uid_for_name("Shapeoko"),
            post: "grbl".into(),
            gcode_units: chipload_job::Units::Mm,
            safe_z: 12.0,
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

    /// An app with a library holding one tool and one machine, both the defaults.
    #[test]
    fn the_theme_is_kept_for_next_time() {
        let dir = settings("theme");
        let _ = std::fs::remove_file(dir.join("settings.json"));
        let mut app = ChiploadApp::default();
        app.set_settings_dir(Some(dir.clone()));
        assert_eq!(
            app.theme,
            Theme::System,
            "no settings yet: follow the system"
        );
        app.set_theme(Theme::Light);
        let mut again = ChiploadApp::default();
        again.set_settings_dir(Some(dir));
        assert_eq!(again.theme, Theme::Light);
    }

    #[test]
    fn export_writes_the_setup_sheet_unless_turned_off() {
        // docs/specs/setup-sheet.md §4
        let dir = settings("sheet");
        let _ = std::fs::remove_file(dir.join("settings.json"));
        let mut app = lower_part();
        app.set_settings_dir(Some(dir.clone()));
        assert!(app.setup_sheet, "on by default");
        assert!(app.generate_and_wait(), "{:?}", app.report);
        let out = temp_dir("sheet-export");
        let project = out.join("lower.chipload");
        assert!(app.save_project(&project));
        assert!(app.export_gcode(&out.join("lower.nc")));
        let sheet = std::fs::read_to_string(out.join("lower-sheet.html")).unwrap();
        assert!(sheet.contains("Setup sheet: lower.chipload"));
        assert!(sheet.contains("1. lower.nc"));
        assert!(
            app.status
                .as_ref()
                .unwrap()
                .0
                .contains("setup sheet lower-sheet.html")
        );

        // Turned off, and kept so for next time.
        app.set_setup_sheet(false);
        assert!(app.export_gcode(&out.join("again.nc")));
        assert!(out.join("again.nc").exists());
        assert!(!out.join("again-sheet.html").exists());
        let mut again = ChiploadApp::default();
        again.set_settings_dir(Some(dir));
        assert!(!again.setup_sheet);
        std::fs::remove_dir_all(&out).unwrap();
    }

    fn app_with_library(name: &str) -> (ChiploadApp, PathBuf) {
        let dir = settings(name);
        save_tools(
            &ToolLibrary {
                default_tool: Some("3 mm upcut".into()),
                tools: vec![upcut()],
                ..ToolLibrary::default()
            },
            &dir.join("tools.json"),
        )
        .unwrap();
        save_machines(
            &MachineLibrary {
                default_machine: Some("Shapeoko".into()),
                machines: vec![shapeoko()],
                ..MachineLibrary::default()
            },
            &dir.join("machines.json"),
        )
        .unwrap();
        let mut app = ChiploadApp::default();
        app.set_settings_dir(Some(dir.clone()));
        (app, dir)
    }

    fn settings_of(app: &ChiploadApp) -> &Job {
        &app.edit.as_ref().unwrap().settings
    }

    #[test]
    fn tools_are_exported_and_imported_after_a_preview() {
        // docs/specs/library-import-export.md §6
        use super::library_transfer::ImportChoice;
        let (mut old, dir) = app_with_library("export");
        let file = dir.join("backup.json");
        assert!(old.export_library(LibraryKind::Tools, &file));
        assert!(old.library_state.message.is_some());

        let mut new = ChiploadApp::default();
        new.set_settings_dir(Some(settings("import-preview")));
        assert!(new.preview_library_import(LibraryKind::Tools, &file));
        let pending = new.library_state.import.as_ref().unwrap();
        assert!(!pending_differs(pending));
        // Cancel changes nothing; Import adds the tool and its default.
        assert!(!new.finish_library_import(ImportChoice::Cancel));
        assert!(new.library.tools.tools.is_empty());
        assert!(new.preview_library_import(LibraryKind::Tools, &file));
        assert!(new.finish_library_import(ImportChoice::Keep));
        assert_eq!(new.library.tools.tools, [upcut()]);
        assert_eq!(
            new.library.tools.default_tool.as_deref(),
            Some("3 mm upcut")
        );
        let (kind, text) = new.library_state.message.clone().unwrap();
        assert_eq!(kind, LibraryKind::Tools);
        assert!(text.contains("Added: 3 mm upcut"), "{text}");

        // A file with other values: kept, or replaced when asked.
        let mut wider = upcut();
        wider.diameter = 3.175;
        let other = dir.join("other.json");
        save_tools(
            &ToolLibrary {
                tools: vec![wider.clone()],
                ..ToolLibrary::default()
            },
            &other,
        )
        .unwrap();
        assert!(new.preview_library_import(LibraryKind::Tools, &other));
        assert!(pending_differs(new.library_state.import.as_ref().unwrap()));
        assert!(new.finish_library_import(ImportChoice::Keep));
        assert_eq!(new.library.tools.tools, [upcut()]);
        assert!(new.preview_library_import(LibraryKind::Tools, &other));
        assert!(new.finish_library_import(ImportChoice::Replace));
        assert_eq!(new.library.tools.tools, [wider]);

        // The wrong kind of file is refused with a hint.
        assert!(!new.preview_library_import(LibraryKind::Machines, &file));
        let error = new.library_state.error.clone().unwrap();
        assert!(error.contains("import it as tools"), "{error}");
        assert!(new.library_state.import.is_none());
    }

    fn pending_differs(pending: &super::library_transfer::PendingImport) -> bool {
        pending.lines().iter().any(|l| l.starts_with("Differs"))
    }

    #[test]
    fn without_a_settings_folder_nothing_changes() {
        let mut app = ChiploadApp::default();
        assert!(app.library().is_none());
        assert!(app.import_dxf(&corpus("dxf/messy_2.dxf"), None));
        let job = settings_of(&app);
        assert!(job.machine.name.is_none() && job.tools[0].name.is_none());
        assert_eq!(
            job.tools[0].diameter,
            chipload_pipeline::DEFAULT_TOOL_DIAMETER
        );
        assert!(!app.save_to_library(SaveTarget::Machine, "Mine"));
    }

    #[test]
    fn a_new_project_starts_with_the_default_machine_and_tool() {
        let (mut app, _) = app_with_library("import");
        assert!(app.import_dxf(&corpus("dxf/messy_2.dxf"), None));
        let job = settings_of(&app);
        assert_eq!(job.machine, shapeoko().machine());
        assert_eq!(job.stock.max_overcut, 0.3);
        assert_eq!(job.tools, [upcut().tool(2)]);
        assert!(
            app.report
                .iter()
                .any(|l| l == "Started with machine: Shapeoko (default profile).")
        );
        // A new operation takes the tool's cutting values (nothing above to copy).
        select(&mut app, &[4]);
        let i = app.add_operation(OpKind::Pocket).unwrap();
        let Operation::Pocket(p) = &app.edit.as_ref().unwrap().ops[i].op else {
            panic!()
        };
        assert_eq!((p.feed, p.step_down, p.stepover), (800.0, 0.75, 0.35));
        assert_eq!(p.tool, 2);
    }

    #[test]
    fn using_a_profile_and_a_library_tool_are_single_undo_steps() {
        // A project made without a library, then the library appears.
        let (_, dir) = app_with_library("use");
        let mut app = lower_part();
        app.set_settings_dir(Some(dir));
        let before = settings_of(&app).clone();

        assert!(app.use_machine_profile("shapeoko"));
        let job = settings_of(&app);
        assert_eq!(job.machine.safe_z, 12.0);
        assert_eq!(job.machine.name.as_deref(), Some("Shapeoko"));
        assert!(
            app.report.iter().any(|l| l == "  safe Z 5 mm -> 12 mm"),
            "{:?}",
            app.report
        );

        // The project's one tool takes the library values and keeps its id, so the
        // operations still use it.
        let id = before.tools[0].id;
        assert!(app.use_library_tool(id, "3 mm upcut"));
        let job = settings_of(&app);
        assert_eq!(job.tools, [upcut().tool(id)]);
        assert!(
            app.report.iter().any(|l| l == "  diameter 6 mm -> 3 mm"),
            "{:?}",
            app.report
        );
        assert!(app.generate_and_wait(), "{:?}", app.report);

        app.undo();
        app.undo();
        assert_eq!(settings_of(&app), &before);
    }

    #[test]
    fn an_operation_takes_a_library_tool_of_its_own() {
        // docs/specs/tool-per-operation.md §8.
        let dir = settings("op-tool");
        save_tools(
            &ToolLibrary {
                tools: vec![upcut()],
                ..ToolLibrary::default()
            },
            &dir.join("tools.json"),
        )
        .unwrap();
        let mut app = lower_part();
        app.set_settings_dir(Some(dir.clone()));
        let before = settings_of(&app).clone();
        let ops_before: Vec<Operation> = app
            .edit
            .as_ref()
            .unwrap()
            .ops
            .iter()
            .map(|e| e.op.clone())
            .collect();
        // The pocket (operation 1) is cut with the 3 mm tool: copied in as tool 2, one
        // undo step, feeds kept.
        assert!(app.tool_from_library(0, "3 mm upcut"));
        let job = settings_of(&app);
        assert_eq!(job.tools.len(), 2);
        assert_eq!(job.tools[1], upcut().tool(2));
        let ops: Vec<Operation> = app
            .edit
            .as_ref()
            .unwrap()
            .ops
            .iter()
            .map(|e| e.op.clone())
            .collect();
        assert_eq!((ops[0].tool(), ops[1].tool()), (2, 1));
        let mut same = ops[0].clone();
        same.set_tool(ops_before[0].tool());
        assert_eq!(same, ops_before[0], "only the tool changed");
        assert!(app.generate_and_wait(), "{:?}", app.report);
        let files = &app.result.as_ref().unwrap().files;
        assert_eq!(files.len(), 2);
        // Exported as one file per tool, named after the chosen file.
        let out = temp_dir("op-tool-export").join("part.nc");
        assert!(app.export_gcode(&out));
        assert!(out.with_file_name("part-1-T2.nc").exists());
        assert!(out.with_file_name("part-2-T1.nc").exists());
        // Again: the library tool is found in the project, not added twice.
        assert!(app.tool_from_library(1, "3 mm upcut"));
        assert_eq!(settings_of(&app).tools.len(), 2);
        // A new operation gets the tool of the last one.
        select(&mut app, &[2]);
        let new = app.add_operation(OpKind::Profile).unwrap();
        assert_eq!(app.edit.as_ref().unwrap().ops[new].op.tool(), 2);
        app.undo();
        app.undo();
        app.undo();
        assert_eq!(settings_of(&app), &before);
    }

    #[test]
    fn saving_to_the_library_names_the_project_values() {
        let dir = settings("save");
        let mut app = lower_part();
        app.set_settings_dir(Some(dir.clone()));
        assert!(app.save_to_library(SaveTarget::Machine, " Router "));
        assert!(app.save_to_library(SaveTarget::Tool(1), "6 mm"));
        let job = settings_of(&app);
        assert_eq!(job.machine.name.as_deref(), Some("Router"));
        assert_eq!(job.tools[0].name.as_deref(), Some("6 mm"));
        let machines = load_machines(&dir.join("machines.json")).unwrap();
        assert_eq!(
            machines.machines[0],
            MachineProfile::from_job("Router", &uid_for_name("Router"), job)
        );
        let tools = load_tools(&dir.join("tools.json")).unwrap();
        assert!(tools.tools[0].same_values(&job.tools[0]));

        // The project and the library agree: no "differs" note.
        let rows =
            |app: &ChiploadApp| user_library::LibraryRows::machine(settings_of(app), app.library());
        assert!(rows(&app).note_text().unwrap().starts_with("from profile"));
        // A job value doesn't count; a machine value does.
        app.edit.as_mut().unwrap().settings.machine.safe_z = 9.0;
        assert!(rows(&app).note_text().unwrap().starts_with("from profile"));
        app.edit.as_mut().unwrap().settings.machine.max_feed = Some(1234.0);
        assert!(
            rows(&app)
                .note_text()
                .unwrap()
                .starts_with("differs from profile \"Router\": max feed 1234 mm/min ->"),
            "{:?}",
            rows(&app).note_text()
        );
        // Update from library takes the machine values back and keeps the job's.
        assert!(app.update_machine_from_library());
        let machine = &settings_of(&app).machine;
        assert_eq!(machine.safe_z, 9.0);
        assert_ne!(machine.max_feed, Some(1234.0));
        assert!(rows(&app).note_text().unwrap().starts_with("from profile"));
    }

    #[test]
    fn library_entries_store_rename_and_refuse_duplicates() {
        let (mut app, dir) = app_with_library("windows");
        let renamed = LibraryTool {
            name: "3 mm".into(),
            ..upcut()
        };
        app.store_library_tool(renamed).unwrap();
        let tools = load_tools(&dir.join("tools.json")).unwrap();
        assert_eq!(tools.default_tool.as_deref(), Some("3 mm"));
        assert_eq!(app.library.tools.tools[0].name, "3 mm");

        let err = app
            .store_library_tool(LibraryTool {
                name: "3 MM".into(),
                uid: uid_for_name("another"),
                ..upcut()
            })
            .unwrap_err();
        assert!(err.contains("already a tool"), "{err}");
        let err = app
            .store_machine_profile(MachineProfile {
                max_overcut: -1.0,
                name: "Bad".into(),
                uid: uid_for_name("Bad"),
                ..shapeoko()
            })
            .unwrap_err();
        assert!(err.contains("max_overcut"), "{err}");
        assert_eq!(
            load_machines(&dir.join("machines.json"))
                .unwrap()
                .machines
                .len(),
            1
        );
        app.set_library_default(LibraryKind::Machines, "Shapeoko")
            .unwrap();
    }

    /// docs/specs/this-computer.md: the posts are listed, built in first, and Preferences
    /// keeps its settings.
    #[test]
    fn this_computer_lists_posts_and_keeps_preferences() {
        let (mut app, dir) = app_with_library("computer");
        let posts = app.post_list();
        assert_eq!(posts[0], ("grbl".to_string(), true, None));
        assert!(posts.iter().all(|(_, builtin, _)| *builtin));
        assert_eq!(app.post_problems(), 0);
        app.set_setup_sheet(false);
        let mut again = ChiploadApp::default();
        again.set_settings_dir(Some(dir));
        assert!(!again.setup_sheet);
        again.focus = Focus::Post(0);
        assert!(again.focus.is_library());
    }

    /// docs/specs/library-in-browser.md §5, §9.
    #[test]
    fn a_library_tool_is_edited_as_a_draft_and_saved() {
        let (mut app, dir) = app_with_library("draft");
        let uid = uid_for_name("3 mm upcut");
        app.show_library(LibraryKind::Tools, Some(&uid));
        assert_eq!(app.focus, Focus::LibraryTool(library_view::uid_key(&uid)));
        let mut draft = upcut();
        draft.diameter = 3.175;
        app.library_state.tool_drafts.insert(uid.clone(), draft);
        // Not written until saved.
        assert_eq!(
            load_tools(&dir.join("tools.json")).unwrap().tools[0].diameter,
            3.0
        );
        assert!(app.save_library_draft(LibraryKind::Tools, &uid));
        assert_eq!(
            load_tools(&dir.join("tools.json")).unwrap().tools[0].diameter,
            3.175
        );
        assert!(app.library_state.tool_drafts.is_empty());
        // A new entry is saved at once under a free name, and shown.
        let new = app.new_library_entry(LibraryKind::Tools).unwrap();
        assert_eq!(app.library.tools.find_uid(&new).unwrap().name, "New tool");
        assert_eq!(app.focus, Focus::LibraryTool(library_view::uid_key(&new)));
        let new = app.new_library_entry(LibraryKind::Machines).unwrap();
        assert_eq!(
            app.library.machines.find_uid(&new).unwrap().name,
            "New machine"
        );
    }

    #[test]
    fn a_library_tool_added_to_the_project_is_linked_and_updates() {
        let (mut app, _dir) = app_with_library("link");
        assert!(app.import_dxf(&corpus("dxf/messy_2.dxf"), None));
        let uid = uid_for_name("3 mm upcut");
        let id = app.add_library_tool_uid(&uid).unwrap();
        assert_eq!(app.focus, Focus::Tool(id));
        let tool = settings_of(&app)
            .tools
            .iter()
            .find(|t| t.id == id)
            .unwrap()
            .clone();
        assert_eq!(tool.library_uid.as_deref(), Some(uid.as_str()));
        let rows = |app: &ChiploadApp| {
            let tool = settings_of(app).tools.iter().find(|t| t.id == id).unwrap();
            user_library::LibraryRows::tool(tool, app.library()).note_text()
        };
        assert_eq!(rows(&app).unwrap(), "from library tool \"3 mm upcut\"");
        // Renamed and changed in the library: still linked, and it says what differs.
        let mut entry = upcut();
        entry.name = "Upcut 3".into();
        entry.rpm = Some(18000.0);
        app.store_library_tool(entry).unwrap();
        assert_eq!(
            rows(&app).unwrap(),
            "differs from library tool \"Upcut 3\": rpm 20000 -> 18000"
        );
        assert!(app.use_library_tool(id, "Upcut 3"));
        assert_eq!(rows(&app).unwrap(), "from library tool \"Upcut 3\"");
        // Saved back: the linked entry takes the project's values, keeping its name.
        app.edit
            .as_mut()
            .unwrap()
            .settings
            .tools
            .iter_mut()
            .find(|t| t.id == id)
            .unwrap()
            .rpm = Some(16000.0);
        assert!(app.save_to_linked(SaveTarget::Tool(id)));
        assert_eq!(app.library.tools.find_uid(&uid).unwrap().rpm, Some(16000.0));
        assert_eq!(app.library.tools.find_uid(&uid).unwrap().name, "Upcut 3");
        // Gone from the library.
        app.remove_from_library(LibraryKind::Tools, "Upcut 3")
            .unwrap();
        assert_eq!(
            rows(&app).unwrap(),
            "library tool \"Upcut 3\" is not in your library"
        );
    }

    #[test]
    fn a_broken_library_is_reported_and_the_project_still_starts() {
        let dir = settings("broken");
        std::fs::write(dir.join("machines.json"), "{\"schema_version\": 9}").unwrap();
        let mut app = ChiploadApp::default();
        app.set_settings_dir(Some(dir));
        assert!(app.import_dxf(&corpus("dxf/messy_2.dxf"), None));
        assert!(
            app.report
                .iter()
                .any(|l| l.contains("schema_version 9 is not supported")),
            "{:?}",
            app.report
        );
        assert!(settings_of(&app).machine.name.is_none());
    }
}
