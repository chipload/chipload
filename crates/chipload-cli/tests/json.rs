// SPDX-License-Identifier: GPL-3.0-or-later
//! `--json` on every command, and `get`, `set`, `check-project`, `--set`
//! (docs/specs/scripting.md §3, §4).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{Value, json};

fn corpus(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/corpus")
        .join(rel)
}

struct Run {
    code: Option<i32>,
    stdout: String,
    stderr: String,
}

/// Runs in `dir`, with `dir/settings` as the settings folder.
fn chipload(dir: &Path, args: &[&str]) -> Run {
    let out = Command::new(env!("CARGO_BIN_EXE_chipload"))
        .env("CHIPLOAD_CONFIG_DIR", dir.join("settings"))
        .current_dir(dir)
        .args(args)
        .output()
        .unwrap();
    Run {
        code: out.status.code(),
        stdout: String::from_utf8(out.stdout).unwrap(),
        stderr: String::from_utf8(out.stderr).unwrap(),
    }
}

/// The one JSON object a `--json` command prints, with the envelope checked.
fn json_of(run: &Run, args: &[&str]) -> Value {
    assert!(run.stderr.is_empty(), "{args:?}: stderr {}", run.stderr);
    assert_eq!(run.stdout.lines().count(), 1, "{args:?}: {}", run.stdout);
    let value: Value = serde_json::from_str(&run.stdout).unwrap();
    assert_eq!(value["chipload_json"], json!(1), "{args:?}");
    let ok = value["ok"].as_bool().unwrap();
    assert_eq!(ok, run.code == Some(0), "{args:?}: {value}");
    assert!(
        value["notes"].is_array() && value["written"].is_array(),
        "{args:?}"
    );
    assert_eq!(value.get("error").is_some(), !ok, "{args:?}: {value}");
    value
}

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("chipload-json-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn every_command_prints_one_json_object() {
    let dir = temp_dir("all");
    std::fs::copy(corpus("jobs/v18/rest.chipload"), dir.join("rest.chipload")).unwrap();
    std::fs::copy(corpus("dxf/simple_part.dxf"), dir.join("part.dxf")).unwrap();
    std::fs::copy(corpus("images/logo.png"), dir.join("logo.png")).unwrap();
    let commands: &[&[&str]] = &[
        &["inspect", "part.dxf"],
        &["import", "part.dxf", "--units", "mm", "-o", "part.chipload"],
        &["update", "part.chipload", "--from", "part.dxf"],
        &["new", "-o", "empty.chipload"],
        &["plan", "rest.chipload"],
        &["run", "rest.chipload", "-o", "rest.nc"],
        &["sheet", "rest.chipload", "-o", "sheet.html"],
        &["simulate", "rest.chipload", "--no-image"],
        &["check", "rest.nc", "--job", "rest.chipload"],
        &["op", "tool", "rest.chipload", "--op", "2", "--tool", "2"],
        &[
            "op",
            "rest",
            "rest.chipload",
            "--op",
            "2",
            "--after-tool",
            "1",
        ],
        &[
            "shape",
            "add",
            "empty.chipload",
            "circle",
            "--center",
            "10,10",
            "--radius",
            "5",
        ],
        &[
            "shape",
            "copy",
            "empty.chipload",
            "1",
            "--dx",
            "20",
            "--dy",
            "0",
        ],
        &["new", "-o", "img.chipload"],
        &["image", "add", "img.chipload", "logo.png", "--width", "80"],
        &["image", "set", "img.chipload", "1", "--threshold", "100"],
        &["image", "list", "img.chipload"],
        &["image", "preview", "img.chipload", "1", "-o", "bw.png"],
        &["set", "img.chipload", "drawing.images.0.blur=2"],
        &["image", "remove", "img.chipload", "1"],
        &["copies", "rest.chipload", "--grid", "2x1", "--gap", "10"],
        &["place", "part.chipload"],
        &["get", "rest.chipload", "stock"],
        &["set", "rest.chipload", "operations.0.name=First"],
        &["check-project", "rest.chipload"],
        &["tool", "add", "6 mm", "--diameter", "6", "--rpm", "18000"],
        &["tool", "list"],
        &["tool", "use", "6 mm", "rest.chipload"],
        &["tool", "export", "tools.json"],
        &["tool", "import", "tools.json"],
        &["machine", "list"],
        &["post", "list"],
        &["licenses"],
    ];
    for args in commands {
        let mut all = args.to_vec();
        all.push("--json");
        let run = chipload(&dir, &all);
        let value = json_of(&run, &all);
        assert_eq!(value["ok"], json!(true), "{all:?}: {value}");
    }
    // Each command's own keys (a sample).
    let keys = |args: &[&str]| {
        let mut all = args.to_vec();
        all.push("--json");
        json_of(&chipload(&dir, &all), &all)
    };
    let run = keys(&["run", "rest.chipload", "-o", "again.nc", "--no-sheet"]);
    assert_eq!(run["files"][0]["tool"], json!(1));
    assert!(run["estimate"]["total_s"].as_f64().unwrap() > 0.0);
    assert_eq!(run["sheet"], Value::Null);
    let written = run["written"][0].as_str().unwrap();
    assert!(Path::new(written).is_absolute() && written.ends_with("again.nc"));
    let plan = keys(&["plan", "rest.chipload"]);
    assert!(plan["program"]["moves"].is_array());
    assert!(
        plan["notes"].as_array().unwrap().is_empty() || plan["notes"][0].is_string(),
        "{}",
        plan["notes"]
    );
    let added = keys(&[
        "shape",
        "add",
        "empty.chipload",
        "circle",
        "--center",
        "50,10",
        "--radius",
        "5",
    ]);
    assert_eq!(added["ids"], json!([3]));
    let simulated = keys(&["simulate", "rest.chipload", "--no-image"]);
    assert!(simulated["numbers"]["removed_cm3"].as_f64().unwrap() > 1.0);
    assert_eq!(simulated["image"], Value::Null);
    let got = keys(&["get", "rest.chipload", "operations.0.name"]);
    assert_eq!(got["value"], json!("First"));
    let listed = keys(&["tool", "list"]);
    assert_eq!(listed["tools"][0]["name"], json!("6 mm"));
    let used = keys(&["tool", "use", "6 mm", "rest.chipload"]);
    assert_eq!(used["added"], json!(false));
    let updated = keys(&["update", "part.chipload", "--from", "part.dxf"]);
    assert!(!updated["unchanged"].as_array().unwrap().is_empty());
    // The flag before the command works too.
    let run = chipload(&dir, &["--json", "get", "rest.chipload", "stock.thickness"]);
    assert_eq!(json_of(&run, &[])["value"], json!(12.0));
    // `render` gives the G-code under `gcode`.
    let part = keys(&[
        "plan",
        "part.chipload",
        "--set",
        "operations.0={\"type\": \"profile\", \"part\": 1, \"select\": {\"layer\": null, \"shapes\": [1]}, \"side\": \"outside\", \"direction\": \"climb\", \"tool\": 1, \"start_depth\": 0, \"end_depth\": 2, \"clearance\": 0, \"step_down\": 2, \"feed\": 1000, \"plunge_feed\": 300}",
    ]);
    std::fs::write(dir.join("program.json"), part["program"].to_string()).unwrap();
    let rendered = keys(&["render", "program.json", "--post", "grbl"]);
    assert!(rendered["gcode"].as_str().unwrap().contains("G21"));
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn failures_give_the_error_and_the_problems_with_their_paths() {
    let dir = temp_dir("fail");
    std::fs::copy(corpus("jobs/v18/rest.chipload"), dir.join("rest.chipload")).unwrap();
    // A rest tool smaller than the pocket's: a problem at operations.0.rest.
    let set = chipload(
        &dir,
        &[
            "set",
            "rest.chipload",
            "operations.0.rest={\"after_tool\": 2}",
            "--json",
        ],
    );
    let value = json_of(&set, &[]);
    assert_eq!(value["problems"][0]["path"], json!("operations.0.rest"));
    for args in [
        &["plan", "rest.chipload", "--json"][..],
        &["run", "rest.chipload", "-o", "x.nc", "--json"],
        &["check-project", "rest.chipload", "--json"],
    ] {
        let run = chipload(&dir, args);
        assert_eq!(run.code, Some(1), "{args:?}");
        let value = json_of(&run, args);
        let error = &value["error"];
        assert!(
            error["message"]
                .as_str()
                .unwrap()
                .contains("needs a tool larger"),
            "{args:?}: {error}"
        );
        assert_eq!(
            error["problems"][0]["path"],
            json!("operations.0.rest"),
            "{args:?}"
        );
    }
    assert!(!dir.join("x.nc").exists());
    // A file that doesn't load.
    let run = chipload(&dir, &["plan", "missing.chipload", "--json"]);
    let value = json_of(&run, &[]);
    assert!(
        value["error"]["message"]
            .as_str()
            .unwrap()
            .contains("missing.chipload")
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn set_writes_only_a_project_that_loads() {
    let dir = temp_dir("set");
    let project = dir.join("rest.chipload");
    std::fs::copy(corpus("jobs/v18/rest.chipload"), &project).unwrap();
    let before = std::fs::read_to_string(&project).unwrap();
    for (values, message) in [
        (vec!["stock.thikness=3"], "unknown field `thikness`"),
        (
            vec!["operations.5.name=x"],
            "index 5 is past the end (there are 2)",
        ),
        (vec!["schema_version=17"], "schema_version can't be set"),
        (vec!["stock.thickness"], "is not path=value"),
        // The second fails, so the first isn't written either.
        (
            vec!["stock.thickness=20", "operations.0.direction=sideways"],
            "sideways",
        ),
    ] {
        let mut args = vec!["set", "rest.chipload"];
        args.extend(values.iter().copied());
        let run = chipload(&dir, &args);
        assert_eq!(run.code, Some(1), "{values:?}");
        assert!(run.stderr.contains(message), "{values:?}: {}", run.stderr);
        assert_eq!(std::fs::read_to_string(&project).unwrap(), before);
    }
    // JSON values, text values, null, appending an operation.
    let run = chipload(
        &dir,
        &[
            "set",
            "rest.chipload",
            "stock.thickness=18",
            "operations.1.name=Corners",
            "operations.1.rest=null",
            "-o",
            "out.chipload",
        ],
    );
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    assert_eq!(std::fs::read_to_string(&project).unwrap(), before);
    let get = |path: &str| {
        let run = chipload(&dir, &["get", "out.chipload", path, "--json"]);
        json_of(&run, &[])["value"].clone()
    };
    assert_eq!(get("stock.thickness"), json!(18.0));
    assert_eq!(get("operations.1.name"), json!("Corners"));
    assert_eq!(get("operations.1.rest"), Value::Null);
    let first = serde_json::to_string(&get("operations.0")).unwrap();
    let run = chipload(
        &dir,
        &["set", "out.chipload", &format!("operations.2={first}")],
    );
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    assert_eq!(get("operations.2.name"), json!("Rough"));
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn set_for_one_run_leaves_the_project_alone() {
    let dir = temp_dir("set-run");
    let project = dir.join("rest.chipload");
    std::fs::copy(corpus("jobs/v18/rest.chipload"), &project).unwrap();
    let before = std::fs::read_to_string(&project).unwrap();
    let total = |extra: &[&str]| {
        let mut args = vec!["plan", "rest.chipload", "--json"];
        args.extend_from_slice(extra);
        let run = chipload(&dir, &args);
        json_of(&run, &args)["estimate"]["total_s"]
            .as_f64()
            .unwrap()
    };
    let normal = total(&[]);
    let faster = total(&[
        "--set",
        "operations.0.feed=3000",
        "--set",
        "operations.1.feed=3000",
    ]);
    assert!(faster < normal, "{faster} vs {normal}");
    let run = chipload(
        &dir,
        &[
            "run",
            "rest.chipload",
            "-o",
            "thick.nc",
            "--set",
            "stock.thickness=18",
        ],
    );
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    assert_eq!(std::fs::read_to_string(&project).unwrap(), before);
    // A bad --set refuses the run.
    let run = chipload(
        &dir,
        &[
            "run",
            "rest.chipload",
            "-o",
            "x.nc",
            "--set",
            "stock.x=wide",
        ],
    );
    assert_eq!(run.code, Some(1));
    assert!(!dir.join("x.nc").exists());
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn text_output_is_unchanged_without_the_flag() {
    let dir = temp_dir("text");
    std::fs::copy(corpus("jobs/v18/rest.chipload"), dir.join("rest.chipload")).unwrap();
    let run = chipload(&dir, &["get", "rest.chipload", "operations.1.rest"]);
    assert_eq!(run.stdout, "{\n  \"after_tool\": 1\n}\n");
    let run = chipload(&dir, &["check-project", "rest.chipload"]);
    assert_eq!(run.stdout, "No problems.\n");
    let run = chipload(&dir, &["schema", "tools", "--json"]);
    let schema: Value = serde_json::from_str(&run.stdout).unwrap();
    assert_eq!(schema["title"], json!("Chipload tool library"));
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn an_image_traces_again_when_set_changes_it() {
    // docs/specs/image-trace.md §5.
    let dir = temp_dir("image");
    std::fs::copy(corpus("images/logo.png"), dir.join("logo.png")).unwrap();
    let ok = |args: &[&str]| {
        let mut all = args.to_vec();
        all.push("--json");
        let value = json_of(&chipload(&dir, &all), &all);
        assert_eq!(value["ok"], json!(true), "{all:?}: {value}");
        value
    };
    ok(&["new", "-o", "p.chipload"]);
    let added = ok(&["image", "add", "p.chipload", "logo.png", "--width", "80"]);
    assert_eq!(added["image"]["shapes"].as_array().unwrap().len(), 4);
    let set = ok(&["set", "p.chipload", "drawing.images.0.invert=true"]);
    assert_eq!(set["image"]["black_at_border"], json!(true), "{set}");
    let listed = ok(&["image", "list", "p.chipload"]);
    assert_eq!(listed["images"][0]["invert"], json!(true));
    assert_eq!(listed["images"][0]["shapes"], json!(5));
    // A threshold out of range is refused before anything is written.
    let run = chipload(
        &dir,
        &["image", "set", "p.chipload", "1", "--threshold", "255"],
    );
    assert_ne!(run.code, Some(0));
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn an_image_traces_into_centre_lines_and_back() {
    // docs/specs/image-arcs-centrelines.md §6.
    let dir = temp_dir("centre");
    std::fs::copy(corpus("images/strokes.png"), dir.join("strokes.png")).unwrap();
    let ok = |args: &[&str]| {
        let mut all = args.to_vec();
        all.push("--json");
        let value = json_of(&chipload(&dir, &all), &all);
        assert_eq!(value["ok"], json!(true), "{all:?}: {value}");
        value
    };
    ok(&["new", "-o", "p.chipload"]);
    let added = ok(&[
        "image",
        "add",
        "p.chipload",
        "strokes.png",
        "--width",
        "60",
        "--trace",
        "centre-line",
    ]);
    assert_eq!(added["image"]["centre_lines"], json!(true), "{added}");
    assert_eq!(added["image"]["dots"], json!(1));
    assert_eq!(added["image"]["stroke_widths"].as_array().unwrap().len(), 2);
    assert_eq!(added["image"]["shapes"].as_array().unwrap().len(), 4);
    let listed = ok(&["image", "list", "p.chipload"]);
    assert_eq!(listed["images"][0]["trace"], json!("centre_line"));
    assert_eq!(listed["images"][0]["arcs"], json!(true));
    let back = ok(&[
        "image",
        "set",
        "p.chipload",
        "1",
        "--trace",
        "outline",
        "--arcs",
        "false",
    ]);
    assert_eq!(back["image"]["centre_lines"], json!(false), "{back}");
    let run = chipload(
        &dir,
        &["image", "set", "p.chipload", "1", "--trace", "skeleton"],
    );
    assert_ne!(run.code, Some(0));
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn dxfs_are_added_listed_placed_updated_and_removed() {
    // docs/specs/drawing-sources.md §2.
    let dir = temp_dir("dxfs");
    std::fs::copy(corpus("dxf/simple_part.dxf"), dir.join("part.dxf")).unwrap();
    let ok = |args: &[&str]| {
        let mut all = args.to_vec();
        all.push("--json");
        let value = json_of(&chipload(&dir, &all), &all);
        assert_eq!(value["ok"], json!(true), "{all:?}: {value}");
        value
    };
    ok(&["import", "part.dxf", "-o", "p.chipload"]);
    let added = ok(&["dxf", "add", "p.chipload", "part.dxf", "--name", "lid"]);
    assert_eq!(added["dxf"], json!(2));
    assert_eq!(added["added"].as_array().unwrap().len(), 7);
    let listed = ok(&["dxf", "list", "p.chipload"]);
    assert_eq!(listed["dxfs"][1]["name"], json!("lid"));
    assert_eq!(listed["dxfs"][1]["shapes"], json!(7));
    // With two DXFs, place and update need --dxf.
    let run = chipload(&dir, &["place", "p.chipload", "--by", "10,0"]);
    assert_ne!(run.code, Some(0));
    ok(&["place", "p.chipload", "--dxf", "2", "--by", "300,0"]);
    let listed = ok(&["dxf", "list", "p.chipload"]);
    assert_eq!(listed["dxfs"][1]["placement"]["dx"], json!(300.0));
    assert_eq!(listed["dxfs"][0]["placement"]["dx"], json!(0.0));
    let updated = ok(&["update", "p.chipload", "--from", "part.dxf", "--dxf", "2"]);
    assert_eq!(
        updated["unchanged"].as_array().unwrap().len(),
        7,
        "{updated}"
    );
    ok(&["dxf", "remove", "p.chipload", "1"]);
    let listed = ok(&["dxf", "list", "p.chipload"]);
    assert_eq!(listed["dxfs"].as_array().unwrap().len(), 1);
    assert_eq!(listed["dxfs"][0]["id"], json!(2));
    let run = chipload(&dir, &["dxf", "remove", "p.chipload", "1"]);
    assert_ne!(run.code, Some(0));
    std::fs::remove_dir_all(&dir).unwrap();
}
