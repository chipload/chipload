// SPDX-License-Identifier: GPL-3.0-or-later
//! `chipload tool …`, `chipload machine …` and new projects from the library
//! (docs/specs/tool-library-and-machines.md).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;

fn corpus(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/corpus")
        .join(rel)
}

/// A fresh folder for one test, used as the settings folder and for its projects.
fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "chipload-cli-library-{name}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn chipload(settings: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_chipload"))
        .env("CHIPLOAD_CONFIG_DIR", settings)
        .args(args)
        .output()
        .unwrap()
}

/// Runs and expects success; returns stdout.
fn ok(settings: &Path, args: &[&str]) -> String {
    let out = chipload(settings, args);
    assert!(
        out.status.success(),
        "{args:?}: {}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap()
}

/// Runs and expects failure; returns stderr.
fn refused(settings: &Path, args: &[&str]) -> String {
    let out = chipload(settings, args);
    assert_eq!(out.status.code(), Some(1), "{args:?} should fail");
    String::from_utf8(out.stderr).unwrap()
}

fn path(p: &Path) -> &str {
    p.to_str().unwrap()
}

fn json(p: &Path) -> Value {
    serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap()
}

fn add_upcut(settings: &Path) {
    ok(
        settings,
        &[
            "tool",
            "add",
            "6 mm upcut",
            "--diameter",
            "6",
            "--flutes",
            "2",
            "--center-cutting",
            "yes",
            "--rpm",
            "18000",
            "--feed",
            "1200",
            "--plunge-feed",
            "300",
            "--step-down",
            "2",
            "--stepover",
            "0.4",
        ],
    );
}

fn add_shapeoko(settings: &Path) {
    ok(
        settings,
        &[
            "machine",
            "add",
            "Shapeoko",
            "--safe-z",
            "10",
            "--max-feed",
            "5000",
            "--travel",
            "830,830,95",
            "--work-zero",
            "stock-bottom-left",
            "--max-overcut",
            "0.3",
        ],
    );
}

#[test]
fn tools_are_added_listed_changed_and_removed() {
    let dir = temp_dir("tools");
    let listed = ok(&dir, &["tool", "list"]);
    assert!(listed.contains("(no tools)"), "{listed}");
    assert!(!dir.join("tools.json").exists(), "listing must not write");

    add_upcut(&dir);
    ok(&dir, &["tool", "default", "6 MM UPCUT"]);
    let listed = ok(&dir, &["tool", "list"]);
    assert!(listed.contains(path(&dir.join("tools.json"))), "{listed}");
    assert!(
        listed.contains(
            "6 mm upcut (default): id 1, endmill 6 mm, flutes 2, centre cutting yes, rpm 18000; \
             feed 1200 plunge 300 mm/min, step down 2 mm, stepover 0.4"
        ),
        "{listed}"
    );

    // A changed cutting value keeps the rest of the set; a rename keeps the default.
    ok(
        &dir,
        &[
            "tool",
            "set",
            "6 mm upcut",
            "--feed",
            "1500",
            "--rename",
            "6 mm",
        ],
    );
    let file = json(&dir.join("tools.json"));
    assert_eq!(file["default_tool"], "6 mm");
    assert_eq!(file["tools"][0]["cutting"]["feed"], 1500.0);
    assert_eq!(file["tools"][0]["cutting"]["stepover"], 0.4);
    assert_eq!(file["schema_version"], 2);

    let err = refused(&dir, &["tool", "add", "6 MM", "--diameter", "3"]);
    assert!(err.contains("already has a tool named"), "{err}");
    let err = refused(&dir, &["tool", "add", "3 mm"]);
    assert!(err.contains("needs --diameter"), "{err}");
    let err = refused(
        &dir,
        &["tool", "add", "3 mm", "--diameter", "3", "--feed", "900"],
    );
    assert!(err.contains("missing --plunge-feed"), "{err}");
    let err = refused(&dir, &["tool", "add", "bad", "--diameter", "0"]);
    assert!(err.contains("diameter: must be greater than 0"), "{err}");

    ok(&dir, &["tool", "remove", "6 mm"]);
    let file = json(&dir.join("tools.json"));
    assert_eq!(file["tools"].as_array().unwrap().len(), 0);
    assert!(file["default_tool"].is_null());
    assert!(dir.join("tools.json.bak").exists());
}

#[test]
fn machines_are_added_listed_and_checked() {
    let dir = temp_dir("machines");
    add_shapeoko(&dir);
    ok(&dir, &["machine", "default", "shapeoko"]);
    let listed = ok(&dir, &["machine", "list"]);
    assert!(
        listed.contains(
            "Shapeoko (default): post grbl, safe Z 10 mm, spindle cw, max overcut 0.3 mm, \
             tool changes files, max feed 5000 mm/min, travel X830 Y830 Z95 mm, work zero stock bottom left corner"
        ),
        "{listed}"
    );
    let err = refused(&dir, &["machine", "add", "Other", "--post", "nonesuch"]);
    assert!(err.contains("nonesuch"), "{err}");
    let err = refused(&dir, &["machine", "add", "Other", "--travel", "1,2"]);
    assert!(err.contains("expected travel as X,Y,Z"), "{err}");
    let err = refused(&dir, &["machine", "add", "Other", "--work-zero", "middle"]);
    assert!(err.contains("expected drawing-origin"), "{err}");
    ok(
        &dir,
        &[
            "machine",
            "add",
            "Laser",
            "--work-zero",
            "-10,5",
            "--spindle",
            "ccw",
        ],
    );
    let file = json(&dir.join("machines.json"));
    assert_eq!(file["machines"][1]["work_zero"]["at"], "point");
    assert_eq!(file["machines"][1]["work_zero"]["x"], -10.0);
    assert_eq!(file["machines"][1]["spindle"], "ccw");
    assert_eq!(file["machines"][1]["safe_z"], 5.0);
}

#[test]
fn a_new_project_starts_from_the_default_machine_and_tool() {
    let dir = temp_dir("import");
    let dxf = corpus("dxf/simple_part.dxf");

    // No library: the built-in values, no names.
    let plain = dir.join("plain.chipload");
    let out = ok(&dir, &["import", path(&dxf), "-o", path(&plain)]);
    assert!(!out.contains("machine:"), "{out}");
    let job = json(&plain);
    assert!(job["machine"]["name"].is_null());
    assert_eq!(job["machine"]["post"], "grbl");
    assert!(job["tools"][0]["name"].is_null());
    assert!(!dir.join("tools.json").exists() && !dir.join("machines.json").exists());

    add_upcut(&dir);
    add_shapeoko(&dir);
    ok(&dir, &["tool", "default", "6 mm upcut"]);
    ok(&dir, &["machine", "default", "Shapeoko"]);
    let project = dir.join("part.chipload");
    let out = ok(&dir, &["import", path(&dxf), "-o", path(&project)]);
    assert!(out.contains("machine: Shapeoko (default profile)"), "{out}");
    assert!(out.contains("tool 1: 6 mm upcut (default tool)"), "{out}");
    let job = json(&project);
    assert_eq!(job["machine"]["name"], "Shapeoko");
    assert_eq!(job["machine"]["safe_z"], 10.0);
    assert_eq!(job["machine"]["travel"]["x"], 830.0);
    assert_eq!(job["machine"]["work_zero"]["corner"], "bottom_left");
    assert_eq!(job["stock"]["max_overcut"], 0.3);
    assert_eq!(job["tools"].as_array().unwrap().len(), 1);
    assert_eq!(job["tools"][0]["name"], "6 mm upcut");
    assert_eq!(job["tools"][0]["center_cutting"], true);

    // Opening a project never reads the library: a broken one changes nothing.
    std::fs::write(dir.join("machines.json"), "{broken").unwrap();
    ok(&dir, &["inspect", path(&project)]);

    let err = refused(
        &dir.join("elsewhere"),
        &[
            "import",
            path(&dxf),
            "-o",
            path(&project),
            "--machine",
            "Nope",
        ],
    );
    assert!(err.contains("no machine profile named \"Nope\""), "{err}");
}

#[test]
fn a_broken_library_warns_on_import_and_is_never_overwritten() {
    let dir = temp_dir("broken");
    let newer = r#"{"schema_version": 3, "tools": []}"#;
    std::fs::write(dir.join("tools.json"), newer).unwrap();
    let project = dir.join("part.chipload");
    let out = chipload(
        &dir,
        &[
            "import",
            path(&corpus("dxf/simple_part.dxf")),
            "-o",
            path(&project),
        ],
    );
    assert!(out.status.success());
    let err = String::from_utf8(out.stderr).unwrap();
    assert!(
        err.contains("warning:") && err.contains("schema_version 3 is not supported"),
        "{err}"
    );
    assert!(err.contains("using built-in defaults"), "{err}");
    assert!(json(&project)["tools"][0]["name"].is_null());

    let err = refused(&dir, &["tool", "add", "3 mm", "--diameter", "3"]);
    assert!(err.contains("schema_version 3 is not supported"), "{err}");
    assert_eq!(
        std::fs::read_to_string(dir.join("tools.json")).unwrap(),
        newer
    );
}

#[test]
fn library_tools_and_machines_go_into_projects_and_back() {
    let dir = temp_dir("use");
    let project = dir.join("part.chipload");
    std::fs::copy(corpus("jobs/v7/ramp_entry.chipload"), &project).unwrap();
    let before = json(&project);
    let ids: Vec<i64> = before["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["id"].as_i64().unwrap())
        .collect();
    assert!(ids.contains(&1), "the corpus project has tool 1: {ids:?}");

    add_upcut(&dir);
    let out = ok(&dir, &["tool", "use", "6 mm upcut", path(&project)]);
    let next = ids.iter().max().unwrap() + 1;
    assert!(
        out.contains(&format!("tool {next} (added as {next}; tool 1 was taken)")),
        "{out}"
    );
    let out = ok(&dir, &["tool", "use", "6 mm upcut", path(&project)]);
    assert!(out.contains("(already in the project)"), "{out}");

    add_shapeoko(&dir);
    let out = ok(&dir, &["machine", "use", "Shapeoko", path(&project)]);
    assert!(out.contains("  safe Z 5 mm -> 10 mm"), "{out}");
    assert!(
        out.contains("  work zero drawing origin -> stock bottom left corner"),
        "{out}"
    );
    let out = ok(&dir, &["machine", "use", "Shapeoko", path(&project)]);
    assert!(out.contains("no values changed"), "{out}");

    // Back into the library: an existing name needs --replace.
    let err = refused(
        &dir,
        &["machine", "save", path(&project), "--name", "Shapeoko"],
    );
    assert!(err.contains("add --replace"), "{err}");
    ok(&dir, &["machine", "save", path(&project), "--name", "Copy"]);
    ok(
        &dir,
        &[
            "tool",
            "save",
            path(&project),
            "--tool",
            "1",
            "--name",
            "Tool one",
        ],
    );
    ok(
        &dir,
        &[
            "tool",
            "save",
            path(&project),
            "--tool",
            "1",
            "--name",
            "6 mm upcut",
            "--replace",
        ],
    );
    let tools = json(&dir.join("tools.json"));
    // The replaced entry keeps its cutting values: a project tool has none.
    assert_eq!(tools["tools"][0]["cutting"]["feed"], 1200.0);
    assert_eq!(tools["tools"][1]["name"], "Tool one");
    let machines = json(&dir.join("machines.json"));
    assert_eq!(machines["machines"][1]["name"], "Copy");
    assert_eq!(machines["machines"][1]["safe_z"], 10.0);

    // The project still runs: the copies are complete project values.
    let gcode = dir.join("out.nc");
    ok(&dir, &["run", path(&project), "-o", path(&gcode)]);
}

#[test]
fn an_empty_project_starts_from_a_named_machine() {
    let dir = temp_dir("new");
    add_upcut(&dir);
    add_shapeoko(&dir);
    ok(&dir, &["tool", "default", "6 mm upcut"]);
    let project = dir.join("empty.chipload");
    let out = ok(
        &dir,
        &["new", "-o", path(&project), "--machine", "shapeoko"],
    );
    assert!(out.contains("machine: Shapeoko (profile)"), "{out}");
    assert!(out.contains("tool 1: 6 mm upcut (default tool)"), "{out}");
    let job = json(&project);
    assert_eq!(job["machine"]["name"], "Shapeoko");
    assert_eq!(job["tools"][0]["name"], "6 mm upcut");
    assert!(job["drawing"]["shapes"].as_array().unwrap().is_empty());
    let err = refused(
        &dir,
        &[
            "new",
            "-o",
            path(&dir.join("x.chipload")),
            "--machine",
            "Nope",
        ],
    );
    assert!(err.contains("no machine profile named \"Nope\""), "{err}");
    assert!(!dir.join("x.chipload").exists());
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_vbit_gets_its_angle_and_tip_and_carves_letters() {
    // docs/specs/v-carve.md §7
    let dir = temp_dir("vbit");
    ok(
        &dir,
        &[
            "tool",
            "add",
            "V 60",
            "--type",
            "vbit",
            "--diameter",
            "12.7",
            "--angle",
            "60",
            "--tip",
            "0.2",
            "--rpm",
            "18000",
        ],
    );
    let listed = ok(&dir, &["tool", "list"]);
    assert!(
        listed.contains("V 60: id 1, v-bit 12.7 mm, rpm 18000, angle 60.0°, tip 0.2 mm"),
        "{listed}"
    );
    let file = json(&dir.join("tools.json"));
    let angle = file["tools"][0]["angle"].as_f64().unwrap();
    assert!(
        (angle - std::f64::consts::FRAC_PI_3).abs() < 1e-12,
        "radians in the file"
    );

    // The letters project V-carves and passes the independent checker.
    let nc = dir.join("letters.nc");
    let out = ok(
        &dir,
        &[
            "run",
            path(&corpus("jobs/v10/vcarve_letters.chipload")),
            "-o",
            path(&nc),
        ],
    );
    assert!(out.contains("Wrote"), "{out}");
    let gcode = std::fs::read_to_string(&nc).unwrap();
    assert!(
        gcode
            .lines()
            .any(|l| l.starts_with("G1 X") && l.contains(" Z-"))
    );
}

#[test]
fn tools_and_machines_are_exported_and_imported_on_another_computer() {
    // docs/specs/library-import-export.md
    let old = temp_dir("export-old");
    let new = temp_dir("export-new");
    let backup = temp_dir("export-files");
    add_upcut(&old);
    ok(&old, &["tool", "add", "3 mm", "--diameter", "3"]);
    ok(&old, &["tool", "default", "6 mm upcut"]);
    add_shapeoko(&old);
    let tools_file = backup.join("tools-backup.json");
    let machines_file = backup.join("machines-backup.json");
    let out = ok(&old, &["tool", "export", path(&tools_file)]);
    assert!(out.contains("Exported tools: 2"), "{out}");
    ok(&old, &["machine", "export", path(&machines_file)]);

    let out = ok(&new, &["tool", "import", path(&tools_file)]);
    assert!(out.contains("Added: 6 mm upcut, 3 mm"), "{out}");
    assert!(out.contains("Default: 6 mm upcut"), "{out}");
    let out = ok(&new, &["machine", "import", path(&machines_file)]);
    assert!(out.contains("Added: Shapeoko"), "{out}");
    let strip = |mut v: Value| {
        v.as_object_mut().unwrap().remove("written_by");
        v
    };
    for file in ["tools.json", "machines.json"] {
        assert_eq!(strip(json(&new.join(file))), strip(json(&old.join(file))));
    }

    // Again: nothing new. A changed tool is kept until --replace.
    ok(&new, &["tool", "set", "3 mm", "--diameter", "3.175"]);
    let out = ok(&new, &["tool", "import", path(&tools_file)]);
    assert!(out.contains("Already in the library: 6 mm upcut"), "{out}");
    assert!(
        out.contains("Kept yours: 3 mm (file: diameter 3.175 mm -> 3 mm)"),
        "{out}"
    );
    assert!(out.contains("--replace"), "{out}");
    assert_eq!(json(&new.join("tools.json"))["tools"][1]["diameter"], 3.175);
    let out = ok(&new, &["tool", "import", path(&tools_file), "--replace"]);
    assert!(out.contains("Replaced: 3 mm"), "{out}");
    assert_eq!(json(&new.join("tools.json"))["tools"][1]["diameter"], 3.0);

    // By name; unknown names, missing files and the wrong kind are refused.
    let one = backup.join("one.json");
    ok(&old, &["tool", "export", path(&one), "--name", "3 MM"]);
    assert_eq!(json(&one)["tools"].as_array().unwrap().len(), 1);
    assert!(json(&one)["default_tool"].is_null());
    let err = refused(&old, &["tool", "export", path(&one), "--name", "2 mm"]);
    assert!(err.contains("no tool named \"2 mm\""), "{err}");
    let err = refused(
        &new,
        &["tool", "import", path(&backup.join("nothing.json"))],
    );
    assert!(err.contains("nothing.json"), "{err}");
    let err = refused(&new, &["tool", "import", path(&machines_file)]);
    assert!(err.contains("import it as machines"), "{err}");
}

#[test]
fn an_imported_machine_with_an_unknown_post_is_kept_with_a_note() {
    let settings = temp_dir("import-post");
    let file = settings.join("other.json");
    std::fs::write(
        &file,
        r#"{"schema_version": 2, "default_machine": null,
            "machines": [{"name": "Laser", "post": "my-laser", "safe_z": 5.0, "spindle": "cw",
                          "max_feed": null, "travel": null, "work_zero": {"at": "drawing_origin"},
                          "max_overcut": 0.2, "tool_change": "files"}]}"#,
    )
    .unwrap();
    let out = ok(&settings, &["machine", "import", path(&file)]);
    assert!(out.contains("Added: Laser"), "{out}");
    assert!(out.contains("no post-processor \"my-laser\""), "{out}");
}

// -- post-processors (docs/specs/user-posts.md)

/// A user post `name` in the settings folder: the GRBL template with its own first line.
fn add_user_post(settings: &Path, name: &str, first_line: &str) {
    let dir = settings.join("posts");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join(format!("{name}.toml")),
        format!(
            "schema_version = 1\nname = \"{name}\"\ntemplate = \"{name}.nc.j2\"\n\
             spindle_dwell_s = 2\narcs = true\n"
        ),
    )
    .unwrap();
    let grbl = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../chipload-post/templates/grbl.nc.j2"),
    )
    .unwrap();
    let template = grbl.replace("(Chipload - {{ post_name }})", first_line);
    std::fs::write(dir.join(format!("{name}.nc.j2")), template).unwrap();
}

#[test]
fn a_user_post_is_listed_copied_into_the_project_and_kept_there() {
    let settings = temp_dir("posts");
    add_user_post(&settings, "shop", "(Shop router)");
    std::fs::write(settings.join("posts/broken.toml"), "schema_version = 1\n").unwrap();
    let listed = ok(&settings, &["post", "list"]);
    for line in [
        "  grbl (built in)",
        "  linuxcnc (built in)",
        "  mach3 (built in)",
        "  shop (yours)",
    ] {
        assert!(listed.lines().any(|l| l == line), "{line}\n{listed}");
    }
    assert!(listed.contains("broken (yours) can't be used"), "{listed}");

    let project = settings.join("part.chipload");
    std::fs::copy(corpus("jobs/v7/helix.chipload"), &project).unwrap();
    let said = ok(
        &settings,
        &["post", "use", path(&project), "--post", "shop"],
    );
    assert!(said.contains("shop (yours, copied in)"), "{said}");
    assert_eq!(json(&project)["machine"]["post"], "shop");
    assert!(
        json(&project)["machine"]["custom_post"]["template"]
            .as_str()
            .unwrap()
            .starts_with("(Shop router)")
    );
    let out = settings.join("part.nc");
    ok(&settings, &["run", path(&project), "-o", path(&out)]);
    let first = std::fs::read_to_string(&out).unwrap();
    assert!(first.starts_with("(Shop router)\n"), "{first}");

    // The file changes: the project keeps its copy until `post use` again.
    add_user_post(&settings, "shop", "(Shop router, new)");
    ok(&settings, &["run", path(&project), "-o", path(&out)]);
    assert_eq!(std::fs::read_to_string(&out).unwrap(), first);
    let said = ok(
        &settings,
        &["post", "use", path(&project), "--post", "shop"],
    );
    assert!(said.contains("copy updated"), "{said}");
    ok(&settings, &["run", path(&project), "-o", path(&out)]);
    assert!(
        std::fs::read_to_string(&out)
            .unwrap()
            .starts_with("(Shop router, new)\n")
    );

    // Gone from the folder: the project still renders from its copy.
    std::fs::remove_dir_all(settings.join("posts")).unwrap();
    ok(&settings, &["run", path(&project), "-o", path(&out)]);

    // Back to a built-in: the copy goes.
    ok(
        &settings,
        &["post", "use", path(&project), "--post", "linuxcnc"],
    );
    assert_eq!(json(&project)["machine"]["custom_post"], Value::Null);
    let err = refused(
        &settings,
        &["post", "use", path(&project), "--post", "nope"],
    );
    assert!(err.contains("nope"), "{err}");
}

#[test]
fn a_machine_profile_can_name_a_user_post_which_is_copied_when_used() {
    let settings = temp_dir("profile-post");
    let err = refused(&settings, &["machine", "add", "Shop", "--post", "shop"]);
    assert!(err.contains("no post-processor \"shop\""), "{err}");
    add_user_post(&settings, "shop", "(Shop router)");
    ok(&settings, &["machine", "add", "Shop", "--post", "shop"]);
    let project = settings.join("part.chipload");
    std::fs::copy(corpus("jobs/v7/helix.chipload"), &project).unwrap();
    ok(&settings, &["machine", "use", "Shop", path(&project)]);
    assert_eq!(json(&project)["machine"]["post"], "shop");
    assert!(json(&project)["machine"]["custom_post"].is_object());
}
