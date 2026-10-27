// SPDX-License-Identifier: GPL-3.0-or-later
//! Job (project) loading, migration and saving against the corpus.

// Integration tests may unwrap and panic: a failure is the test result.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};

use chipload_job::migrations::migrate;
use chipload_job::{
    CURRENT_VERSION, Direction, HoleMethod, ImageThreshold, ImageTrace, JobError, Operation,
    ToolType, load_job, parse_job,
};
use serde_json::{Value, json};

fn corpus() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/corpus/jobs")
}

/// Every frozen project, in every version directory.
fn all_corpus_jobs() -> Vec<PathBuf> {
    let mut files = Vec::new();
    for dir in std::fs::read_dir(corpus()).unwrap() {
        for file in std::fs::read_dir(dir.unwrap().path()).unwrap() {
            let path = file.unwrap().path();
            if path.extension().is_some_and(|e| e == "chipload") {
                files.push(path);
            }
        }
    }
    files.sort();
    files
}

fn raw(rel: &str) -> Value {
    serde_json::from_str(&std::fs::read_to_string(corpus().join(rel)).unwrap()).unwrap()
}

#[test]
fn every_frozen_project_loads_as_the_current_version() {
    let files = all_corpus_jobs();
    assert!(files.len() >= 6, "expected the corpus, found {files:?}");
    for path in files {
        let job = load_job(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        assert_eq!(job.schema_version, CURRENT_VERSION, "{}", path.display());
    }
}

#[test]
fn missing_newer_and_malformed_versions_are_refused() {
    for data in [
        json!({}),
        json!({"schema_version": CURRENT_VERSION + 1}),
        json!({"schema_version": 0}),
        json!({"schema_version": true}),
        json!({"schema_version": "4"}),
        json!({"schema_version": 4.0}),
    ] {
        let result = migrate(data.clone());
        assert!(
            matches!(result, Err(JobError::UnsupportedVersion { .. })),
            "{data} should be refused"
        );
    }
}

#[test]
fn jobs_from_before_projects_are_refused_with_advice() {
    for version in 1..=3 {
        let err = migrate(json!({"schema_version": version})).unwrap_err();
        assert!(matches!(err, JobError::TooOld { found } if found == version));
        assert!(err.to_string().contains("Chipload 0.3.0"), "{err}");
    }
}

#[test]
fn v5_to_v6_drops_the_unused_legacy_keys() {
    // A v5 project writes `external` and `handles` as null: they go away silently.
    let v5 = raw("v5/drawn_part.chipload");
    assert!(v5["drawing"].get("external").is_some());
    let current = migrate(v5).unwrap();
    assert_eq!(current["schema_version"], json!(CURRENT_VERSION));
    assert!(current["drawing"].get("external").is_none());
    for op in current["operations"].as_array().unwrap() {
        assert!(op["select"].get("handles").is_none());
    }
}

#[test]
fn v5_files_that_need_removed_features_are_refused() {
    let refused = |change: &dyn Fn(&mut Value), why: &str| {
        let mut v5 = raw("v5/drawn_part.chipload");
        change(&mut v5);
        let err = migrate(v5).unwrap_err();
        assert!(
            matches!(err, JobError::CannotMigrate { from: 5, .. }),
            "{err}"
        );
        assert!(err.to_string().contains(why), "{why}: {err}");
    };
    refused(
        &|d| d["drawing"]["external"] = json!({"file": "a.dxf", "units": "mm"}),
        "not embedded",
    );
    refused(
        &|d| d["operations"][0]["select"]["handles"] = json!(["5C"]),
        "selects DXF handles",
    );
    refused(
        &|d| {
            d["operations"]
                .as_array_mut()
                .unwrap()
                .push(json!({"type": "drill", "select": {"layer": "x"}, "tool": 1}));
        },
        "operation 3 is a drill operation",
    );
}

/// The messy_2 lower part (a pocket, then a profile), as current-version data.
fn lower_job() -> Value {
    migrate(raw("v4/messy_2_lower.chipload")).unwrap()
}

#[test]
fn pockets_cannot_have_tabs() {
    let mut data = lower_job();
    data["operations"][0]["tabs"] = json!({"count": 4, "width": 8, "height": 3});
    let err = parse_job(data, "job.json").unwrap_err();
    assert!(err.to_string().contains("tabs"), "{err}");
}

#[test]
fn unknown_fields_are_refused() {
    let mut data = lower_job();
    data["stock"]["colour"] = json!("oak");
    assert!(parse_job(data, "job.json").is_err());
    // Removed in v6: refused in current files, not ignored.
    let mut data = lower_job();
    data["operations"][0]["select"]["handles"] = json!(null);
    assert!(parse_job(data, "job.json").is_err());
}

#[test]
fn value_constraints_are_checked() {
    let cases: [(&str, Value); 5] = [
        ("/stock/thickness", json!(0)),
        ("/operations/0/stepover", json!(1.5)),
        ("/operations/0/end_depth", json!(-1)),
        ("/tools/0/diameter", json!(0)),
        ("/operations/1/select", json!({})),
    ];
    for (pointer, bad) in cases {
        let mut data = lower_job();
        *data.pointer_mut(pointer).unwrap() = bad;
        assert!(
            parse_job(data, "job.json").is_err(),
            "{pointer} should be refused"
        );
    }
}

#[test]
fn pocket_fields_parse() {
    let job = parse_job(lower_job(), "job.json").unwrap();
    let Operation::Pocket(pocket) = &job.operations[0] else {
        panic!("expected a pocket")
    };
    assert_eq!(pocket.direction, Direction::Climb);
    assert_eq!(job.tools[0].center_cutting, Some(true));
}

// -- saving

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("chipload-job-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn round_trip_keeps_the_job() {
    let mut original = load_job(&corpus().join("v4/simple_part_full.chipload")).unwrap();
    let dir = temp_dir("round-trip");
    let path = dir.join("job.chipload");
    chipload_job::save_job(&original, &path).unwrap();
    let mut reloaded = load_job(&path).unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
    original.written_by = None;
    reloaded.written_by = None;
    assert_eq!(reloaded, original);
}

#[test]
fn saving_over_a_project_keeps_the_previous_one_as_bak() {
    // docs/specs/autosave-and-backup.md §3
    let mut job = load_job(&corpus().join("v4/simple_part_full.chipload")).unwrap();
    let dir = temp_dir("bak");
    let path = dir.join("part.chipload");
    let bak = dir.join("part.chipload.bak");
    let _ = std::fs::remove_file(&bak);
    chipload_job::save_job(&job, &path).unwrap();
    assert!(!bak.exists(), "a first save makes no backup");
    let first = std::fs::read(&path).unwrap();
    job.stock.thickness += 1.0;
    chipload_job::save_job(&job, &path).unwrap();
    assert_eq!(std::fs::read(&bak).unwrap(), first);
    assert_ne!(std::fs::read(&path).unwrap(), first);

    // A backup that can't be written stops the save; the project is untouched.
    let blocked = dir.join("blocked.chipload");
    chipload_job::save_job(&job, &blocked).unwrap();
    let before = std::fs::read(&blocked).unwrap();
    std::fs::create_dir_all(dir.join("blocked.chipload.bak")).unwrap();
    job.stock.thickness += 1.0;
    let err = chipload_job::save_job(&job, &blocked)
        .unwrap_err()
        .to_string();
    assert!(err.contains(".bak"), "{err}");
    assert_eq!(std::fs::read(&blocked).unwrap(), before);
    assert!(!dir.join("blocked.chipload.tmp").exists());
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn every_corpus_job_survives_save_and_load() {
    for path in all_corpus_jobs() {
        let job = load_job(&path).unwrap();
        let text = chipload_job::dump_job(&job).unwrap();
        let data: Value = serde_json::from_str(&text).unwrap();
        let again = parse_job(migrate(data).unwrap(), "saved").unwrap();
        assert_eq!(again.drawing, job.drawing, "{}", path.display());
        assert_eq!(again.operations, job.operations, "{}", path.display());
    }
}

#[test]
fn old_version_is_saved_as_current() {
    let job = load_job(&corpus().join("v4/example_job.chipload")).unwrap();
    let data: Value = serde_json::from_str(&chipload_job::dump_job(&job).unwrap()).unwrap();
    assert_eq!(data["schema_version"], json!(CURRENT_VERSION));
    assert!(
        data["written_by"]
            .as_str()
            .unwrap()
            .starts_with("chipload ")
    );
}

#[test]
fn defaults_are_written_explicitly() {
    let job = load_job(&corpus().join("v4/example_job.chipload")).unwrap();
    let data: Value = serde_json::from_str(&chipload_job::dump_job(&job).unwrap()).unwrap();
    assert_eq!(data["stock"]["origin"], json!("bottom_left"));
    assert_eq!(data["stock"]["z_zero"], json!("top"));
    assert_eq!(
        data["machine"]["work_zero"],
        json!({"at": "drawing_origin"})
    );
    assert!(
        data["machine"]
            .as_object()
            .unwrap()
            .contains_key("max_feed")
    );
}

#[test]
fn output_is_deterministic_ascii() {
    let mut job = load_job(&corpus().join("v4/simple_part_full.chipload")).unwrap();
    if let Operation::Profile(op) = &mut job.operations[0] {
        op.name = "Hål ⌀6 😀".into();
    }
    let text = chipload_job::dump_job(&job).unwrap();
    assert_eq!(text, chipload_job::dump_job(&job).unwrap());
    assert!(text.ends_with("}\n") && text.is_ascii());
    // Non-ASCII escaped as \uXXXX (UTF-16 surrogates for the emoji).
    assert!(
        text.contains(r#""H\u00e5l \u23006 \ud83d\ude00""#),
        "{text}"
    );
    let back: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(back["operations"][0]["name"], json!("Hål ⌀6 😀"));
}

#[test]
fn operations_are_enabled_unless_the_file_says_otherwise() {
    for path in all_corpus_jobs() {
        let job = load_job(&path).unwrap();
        assert!(
            job.operations.iter().all(Operation::enabled),
            "{}",
            path.display()
        );
        let data: Value = serde_json::from_str(&chipload_job::dump_job(&job).unwrap()).unwrap();
        for op in data["operations"].as_array().unwrap() {
            assert_eq!(op["enabled"], json!(true), "written explicitly");
        }
    }
    let mut data = lower_job();
    data["operations"][1]["enabled"] = json!(false);
    let job = parse_job(data, "test").unwrap();
    assert!(!job.operations[1].enabled());
}

// -- projects (docs/specs/project-file.md)

fn project(shapes: Value, next: u64, select: Value) -> Value {
    let mut data = lower_job();
    data["drawing"] = json!({"shapes": shapes, "next_shape_id": next, "dxfs": []});
    data["operations"] = json!([data["operations"][1].clone()]);
    data["operations"][0]["select"] = select;
    data
}

fn square_shape(id: u64) -> Value {
    json!({"id": id, "layer": "L", "kind": "path", "closed": true, "segments": [
        {"line": {"start": [0.0, 0.0], "end": [10.0, 0.0]}},
        {"line": {"start": [10.0, 0.0], "end": [10.0, 10.0]}},
        {"line": {"start": [10.0, 10.0], "end": [0.0, 0.0]}},
    ]})
}

#[test]
fn shapes_round_trip_exactly() {
    let arc = json!({"id": 1, "layer": "L", "dxf_handle": "5C", "dxf_type": "CIRCLE",
        "kind": "path", "closed": true, "segments": [
        {"arc": {"center": [0.1, -1e-300], "radius": 1.0 / 3.0, "start_angle": 0.0,
                 "sweep": std::f64::consts::TAU}}]});
    let job = parse_job(project(json!([arc]), 2, json!({"shapes": [1]})), "p").unwrap();
    let text = chipload_job::dump_job(&job).unwrap();
    let again = parse_job(migrate(serde_json::from_str(&text).unwrap()).unwrap(), "p").unwrap();
    assert_eq!(again.drawing, job.drawing);
}

#[test]
fn drawing_rules_are_enforced() {
    let refused = |data: Value, what: &str| {
        let err = parse_job(data, "p").unwrap_err().to_string();
        assert!(err.contains(what), "{what}: {err}");
    };
    let ok = project(json!([square_shape(1)]), 2, json!({"shapes": [1]}));
    parse_job(ok, "p").unwrap();
    refused(
        project(json!([square_shape(1)]), 1, json!({"shapes": [1]})),
        "next_shape_id",
    );
    refused(
        project(
            json!([square_shape(1), square_shape(1)]),
            3,
            json!({"layer": "L"}),
        ),
        "used twice",
    );
    refused(
        project(json!([square_shape(0)]), 3, json!({"layer": "L"})),
        "between 1",
    );
    refused(
        project(json!([square_shape(1)]), 2, json!({"handles": ["5C"]})),
        "unknown field `handles`",
    );
    let mut bad_arc = square_shape(1);
    bad_arc["segments"] =
        json!([{"arc": {"center": [0, 0], "radius": 0.0, "start_angle": 0, "sweep": 1}}]);
    refused(
        project(json!([bad_arc]), 2, json!({"layer": "L"})),
        "radii positive",
    );
    let mut unknown = square_shape(1);
    unknown["kind"] = json!("spiral");
    refused(
        project(json!([unknown]), 2, json!({"layer": "L"})),
        "unknown variant",
    );
}

#[test]
fn every_f64_survives_save_and_load_exactly() {
    // Deterministic pseudo-random bit patterns plus hard cases (subnormals, near-halfway).
    let mut state: u64 = 0x9e37_79b9_7f4a_7c15;
    let mut values: Vec<f64> = vec![
        0.1,
        1.0 / 3.0,
        -85.4901456079,
        2.225_073_858_507_201e-308,
        5e-324,
        f64::MAX,
        9_007_199_254_740_993.0,
        0.300_000_000_000_000_04,
    ];
    for _ in 0..1_000_000 {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        let v = f64::from_bits(state);
        if v.is_finite() {
            values.push(v);
        }
    }
    let text = serde_json::to_string(&values).unwrap();
    let back: Vec<f64> = serde_json::from_str(&text).unwrap();
    let wrong = values
        .iter()
        .zip(&back)
        .filter(|(a, b)| a.to_bits() != b.to_bits())
        .count();
    assert_eq!(wrong, 0, "{wrong} values changed through JSON");
}

// -- drawn shapes (docs/specs/drawing-tools.md)

fn drawn_shapes() -> Value {
    json!([
        {"id": 1, "layer": "0", "kind": "circle", "center": [50.0, 30.0], "radius": 12.5},
        {"id": 2, "layer": "0", "kind": "rectangle", "center": [0.0, 0.0], "length": 80.0,
         "width": 40.0, "angle": 0.5235987755982988, "corner_radius": 5.0},
        {"id": 3, "layer": "0", "kind": "polyline", "closed": true,
         "points": [[0.0, 0.0], [40.0, 0.0], [20.0, 30.0]]},
    ])
}

#[test]
fn drawn_shapes_round_trip_exactly_and_are_written_flat() {
    let job = parse_job(project(drawn_shapes(), 4, json!({"layer": "0"})), "p").unwrap();
    let text = chipload_job::dump_job(&job).unwrap();
    let data: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(data["schema_version"], CURRENT_VERSION);
    // Written with every field, so a polyline without bulges gets them (all straight).
    let mut written = drawn_shapes();
    written[2]["bulges"] = json!([0.0, 0.0, 0.0]);
    assert_eq!(data["drawing"]["shapes"], written);
    let again = parse_job(migrate(data).unwrap(), "p").unwrap();
    assert_eq!(again.drawing, job.drawing);
}

// -- polyline bulges (docs/specs/drawing-follow-ups.md section 3)

fn polyline(closed: bool, points: Value, bulges: Option<Value>) -> Value {
    let mut shape = json!({"id": 1, "layer": "0", "kind": "polyline", "closed": closed,
                           "points": points});
    if let Some(b) = bulges {
        shape["bulges"] = b;
    }
    project(json!([shape]), 2, json!({"layer": "0"}))
}

#[test]
fn polyline_bulges_round_trip_and_are_checked() {
    let triangle = json!([[0.0, 0.0], [40.0, 0.0], [20.0, 30.0]]);
    let job = parse_job(
        polyline(true, triangle.clone(), Some(json!([0.0, 0.4, -1.0]))),
        "p",
    )
    .unwrap();
    let text = chipload_job::dump_job(&job).unwrap();
    let data: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(
        data["drawing"]["shapes"][0]["bulges"],
        json!([0.0, 0.4, -1.0])
    );
    assert_eq!(parse_job(data, "p").unwrap().drawing, job.drawing);
    // Open: one fewer than the points.
    assert!(
        parse_job(
            polyline(false, triangle.clone(), Some(json!([0.2, 0.0]))),
            "p"
        )
        .is_ok()
    );

    let refused = |data: Value, what: &str| {
        let err = parse_job(data, "p").unwrap_err().to_string();
        assert!(err.contains(what), "{what}: {err}");
    };
    refused(
        polyline(true, triangle.clone(), Some(json!([0.0, 1.5, 0.0]))),
        "bulges.1: must be between -1 and 1",
    );
    refused(
        polyline(true, triangle.clone(), Some(json!([0.0, 0.0]))),
        "bulges: one per segment, 3 for this polyline, got 2",
    );
    refused(
        polyline(false, triangle, Some(json!([0.0, 0.0, 0.0]))),
        "bulges: one per segment, 2 for this polyline, got 3",
    );
    refused(
        project(
            json!([{"id": 1, "layer": "0", "kind": "circle", "center": [0.0, 0.0],
                    "radius": 1.0, "bulges": [0.0]}]),
            2,
            json!({"layer": "0"}),
        ),
        "field `bulges` is not allowed for a circle",
    );
}

#[test]
fn drawn_shape_values_are_checked() {
    let refused = |shape: Value, what: &str| {
        let data = project(json!([shape]), 2, json!({"layer": "0"}));
        let err = parse_job(data, "p").unwrap_err().to_string();
        assert!(err.contains(what), "{what}: {err}");
    };
    let circle = |radius: Value| json!({"id": 1, "layer": "0", "kind": "circle", "center": [0.0, 0.0], "radius": radius});
    refused(circle(json!(0.0)), "radius: must be a positive number");
    refused(circle(json!(-1.0)), "radius: must be a positive number");
    refused(
        json!({"id": 1, "layer": "0", "kind": "circle", "center": [0.0, 0.0]}),
        "a circle needs `radius`",
    );
    refused(
        json!({"id": 1, "layer": "0", "kind": "circle", "center": [0.0, 0.0], "radius": 1.0,
               "closed": true}),
        "field `closed` is not allowed for a circle",
    );
    let rectangle = |width: f64, corner_radius: f64| json!({"id": 1, "layer": "0", "kind": "rectangle", "center": [0.0, 0.0], "length": 10.0, "width": width, "angle": 0.0, "corner_radius": corner_radius});
    refused(rectangle(0.0, 0.0), "width: must be a positive number");
    refused(
        rectangle(4.0, 2.5),
        "corner_radius: at most 2 mm, half the shorter side, got 2.5",
    );
    refused(rectangle(4.0, -1.0), "corner_radius: must be 0 or more");
    refused(
        json!({"id": 1, "layer": "0", "kind": "rectangle", "center": [0.0, 0.0],
               "length": 10.0, "width": 4.0, "angle": 0.0}),
        "a rectangle needs `corner_radius`",
    );
    let polyline = |closed: bool, points: Value| json!({"id": 1, "layer": "0", "kind": "polyline", "closed": closed, "points": points});
    refused(
        polyline(true, json!([[0.0, 0.0], [1.0, 0.0]])),
        "a closed polyline needs at least 3 points",
    );
    refused(
        polyline(false, json!([[0.0, 0.0]])),
        "an open polyline needs at least 2 points",
    );
    refused(
        polyline(false, json!([[0.0, 0.0], [0.0, 0.0005], [5.0, 0.0]])),
        "points.1: is on top of the previous point",
    );
    // An open polyline with two points is fine.
    let ok = project(
        json!([polyline(false, json!([[0.0, 0.0], [5.0, 0.0]]))]),
        2,
        json!({"layer": "0"}),
    );
    parse_job(ok, "p").unwrap();
}

#[test]
fn a_file_from_a_newer_chipload_is_refused_by_its_version() {
    let mut data = project(drawn_shapes(), 4, json!({"layer": "0"}));
    data["schema_version"] = json!(CURRENT_VERSION + 1);
    let err = migrate(data).unwrap_err().to_string();
    let expected = format!(
        "schema_version {} is not supported: this version of Chipload reads 4 to {CURRENT_VERSION}",
        CURRENT_VERSION + 1
    );
    assert!(err.contains(&expected), "{err}");
    let err = migrate(json!({})).unwrap_err().to_string();
    assert!(
        err.starts_with("Job schema_version missing is not supported"),
        "{err}"
    );
    let err = migrate(json!({"schema_version": "4"}))
        .unwrap_err()
        .to_string();
    assert!(err.starts_with(r#"Job schema_version "4" is not"#), "{err}");
}

#[test]
fn work_zero_round_trips_and_is_checked() {
    let with_zero = |zero: Value| {
        let mut data = project(json!([square_shape(1)]), 2, json!({"shapes": [1]}));
        data["machine"]["work_zero"] = zero;
        data
    };
    for zero in [
        json!({"at": "drawing_origin"}),
        json!({"at": "stock", "corner": "center"}),
        json!({"at": "stock", "corner": "top_left"}),
        json!({"at": "point", "x": 25.5, "y": -1e-3}),
    ] {
        let job = parse_job(with_zero(zero.clone()), "p").unwrap();
        let data: Value = serde_json::from_str(&chipload_job::dump_job(&job).unwrap()).unwrap();
        assert_eq!(data["machine"]["work_zero"], zero);
    }
    // Missing: the drawing origin, written explicitly on save.
    let mut data = project(json!([square_shape(1)]), 2, json!({"shapes": [1]}));
    data["machine"].as_object_mut().unwrap().remove("work_zero");
    let job = parse_job(data, "p").unwrap();
    assert_eq!(
        job.machine.work_zero,
        chipload_job::WorkZero::DrawingOrigin {}
    );
    let saved: Value = serde_json::from_str(&chipload_job::dump_job(&job).unwrap()).unwrap();
    assert_eq!(
        saved["machine"]["work_zero"],
        json!({"at": "drawing_origin"})
    );
    for bad in [
        json!({"at": "somewhere"}),
        json!({"at": "stock", "corner": "middle"}),
        json!({"at": "stock"}),
        json!({"at": "point", "x": 1.0}),
        json!({"at": "drawing_origin", "x": 1.0}),
    ] {
        assert!(parse_job(with_zero(bad.clone()), "p").is_err(), "{bad}");
    }
}

// -- start/end depth, clearance, overcut (docs/specs/depths-and-clearance.md)

#[test]
fn v6_to_v7_keeps_machining_as_it_was() {
    let v6 = raw("v6/drawn_part.chipload");
    let depths: Vec<Value> = v6["operations"]
        .as_array()
        .unwrap()
        .iter()
        .map(|op| op["depth"].clone())
        .collect();
    let current = migrate(v6).unwrap();
    assert_eq!(current["stock"]["max_overcut"], json!(0.0));
    for (op, depth) in current["operations"].as_array().unwrap().iter().zip(depths) {
        assert!(op.get("depth").is_none());
        assert_eq!(op["end_depth"], depth);
        assert_eq!(op["start_depth"], json!(0.0));
        assert_eq!(op["clearance"], json!(0.0));
    }
}

#[test]
fn depth_and_clearance_rules_are_checked() {
    let refused = |change: &dyn Fn(&mut Value), why: &str| {
        let mut data = lower_job();
        change(&mut data);
        let err = parse_job(data, "p").unwrap_err().to_string();
        assert!(err.contains(why), "{why}: {err}");
    };
    // Operation 1 is the outline profile (outside), operation 0 the pocket.
    refused(
        &|d| d["operations"][1]["start_depth"] = json!(-0.5),
        "start_depth: must be 0 or more",
    );
    refused(
        &|d| d["operations"][1]["start_depth"] = d["operations"][1]["end_depth"].clone(),
        "start_depth: must be less than end_depth",
    );
    refused(
        &|d| d["stock"]["max_overcut"] = json!(-1),
        "stock.max_overcut: must be 0 or more",
    );
    refused(
        &|d| {
            d["operations"][1]["side"] = json!("on");
            d["operations"][1]["clearance"] = json!(0.2);
        },
        "a profile on the line has no side",
    );
    // Allowed: a start depth, a negative clearance, a clearance on a pocket.
    let mut data = lower_job();
    data["operations"][1]["start_depth"] = json!(2.0);
    data["operations"][1]["clearance"] = json!(-0.4);
    data["operations"][0]["clearance"] = json!(0.25);
    parse_job(data, "p").unwrap();
}

// -- profiles always spiral (docs/specs/profile-always-spiral.md)

#[test]
fn v27_turns_a_profiles_entry_into_a_ramp_angle_and_drops_its_lead() {
    let v26 = raw("v26/straight_tabs.chipload");
    let plunged = &v26["operations"][1];
    assert_eq!(plunged["entry"], json!({"type": "plunge"}));
    assert!(plunged.get("lead").is_some());
    let current = migrate(v26.clone()).unwrap();
    assert_eq!(current["schema_version"], json!(30));
    let op = &current["operations"][1];
    assert_eq!(op["type"], json!("profile"));
    assert!(op.get("entry").is_none() && op.get("lead").is_none());
    // A plunge gets the default angle, 3°; its straight tabs stay straight.
    assert_eq!(
        op["ramp_angle"],
        json!(chipload_job::DEFAULT_RAMP_ANGLE_RAD)
    );
    assert_eq!(
        op["tabs"]["flank_angle"],
        json!(std::f64::consts::FRAC_PI_2)
    );
    // The pocket keeps its helix and its lead.
    let pocket = &current["operations"][0];
    assert_eq!(pocket["entry"]["type"], json!("helix"));
    assert!(pocket["lead"].is_object());
    let job = parse_job(current, "p").unwrap();
    let Operation::Profile(p) = &job.operations[1] else {
        panic!()
    };
    assert_eq!(p.ramp_angle, chipload_job::DEFAULT_RAMP_ANGLE_RAD);
    // A ramp keeps its angle.
    let v16 = raw("v16/profile_spiral.chipload");
    let angle = v16["operations"][1]["entry"]["ramp_angle"].clone();
    let current = migrate(v16).unwrap();
    assert_eq!(current["operations"][1]["ramp_angle"], angle);
    // A current file with the old fields on a profile is refused; the angle has its rule.
    for (field, value) in [
        ("entry", json!({"type": "plunge"})),
        ("lead", json!({"radius": 3.0})),
    ] {
        let mut data = lower_job();
        data["operations"][1][field] = value;
        assert!(parse_job(data, "p").is_err(), "{field} is refused");
    }
    let mut data = lower_job();
    data["operations"][1]["ramp_angle"] = json!(1.0);
    let err = parse_job(data, "p").unwrap_err().to_string();
    assert!(err.contains("operations.1.ramp_angle"), "{err}");
}

// -- the clearing settings, gone since v26 (docs/specs/pocket.md as built 2026-10-15)

#[test]
fn v26_drops_the_clearing_settings_and_removed_strategies_become_rings_pockets() {
    // A v7 trochoidal pocket and a v12 adaptive one load as plain pockets.
    for sample in ["v7/trochoidal.chipload", "v12/adaptive.chipload"] {
        let current = migrate(raw(sample)).unwrap();
        let op = &current["operations"][0];
        assert_eq!(op["type"], json!("pocket"), "{sample}");
        for gone in ["strategy", "keep_tool_down", "round_corners"] {
            assert!(op.get(gone).is_none(), "{sample}: {gone} should be gone");
        }
        let job = parse_job(current, "p").unwrap_or_else(|e| panic!("{sample}: {e}"));
        assert!(
            matches!(job.operations[0], Operation::Pocket(_)),
            "{sample}"
        );
    }
    // Written back without them; a current file that still has one is refused.
    let job = parse_job(lower_job(), "p").unwrap();
    let saved: Value = serde_json::from_str(&chipload_job::dump_job(&job).unwrap()).unwrap();
    for gone in ["strategy", "keep_tool_down", "round_corners"] {
        assert!(saved["operations"][0].get(gone).is_none(), "{gone}");
        let mut data = lower_job();
        data["operations"][0][gone] = json!({"type": "rings"});
        assert!(parse_job(data, "p").is_err(), "{gone} is refused");
    }
}

// -- copies in a grid (docs/specs/copies-grid.md)

#[test]
fn copies_round_trip_default_to_one_part_and_are_checked() {
    let with_copies = |copies: Value| {
        let mut data = project(json!([square_shape(1)]), 2, json!({"shapes": [1]}));
        data["parts"][0]["copies"] = copies;
        data
    };
    // None: one copy, written explicitly as null.
    let data = project(json!([square_shape(1)]), 2, json!({"shapes": [1]}));
    let job = parse_job(data, "p").unwrap();
    assert_eq!(job.parts[0].copies, None);
    let saved: Value = serde_json::from_str(&chipload_job::dump_job(&job).unwrap()).unwrap();
    assert_eq!(saved["parts"][0]["copies"], Value::Null);

    // A missing order is by operation, written explicitly.
    let job = parse_job(
        with_copies(json!({"columns": 3, "rows": 2, "dx": 120.5, "dy": -80})),
        "p",
    )
    .unwrap();
    let saved: Value = serde_json::from_str(&chipload_job::dump_job(&job).unwrap()).unwrap();
    assert_eq!(
        saved["parts"][0]["copies"],
        json!({"columns": 3, "rows": 2, "dx": 120.5, "dy": -80.0, "order": "by_operation"})
    );
    let by_copy = json!({"columns": 1, "rows": 1, "dx": 0.0, "dy": 0.0, "order": "by_copy"});
    let job = parse_job(with_copies(by_copy.clone()), "p").unwrap();
    let saved: Value = serde_json::from_str(&chipload_job::dump_job(&job).unwrap()).unwrap();
    assert_eq!(saved["parts"][0]["copies"], by_copy);
    // Renamed in schema 22 (docs/specs/parts.md §4): the old name is refused in a current file.
    let by_part = json!({"columns": 1, "rows": 1, "dx": 0.0, "dy": 0.0, "order": "by_part"});
    assert!(parse_job(with_copies(by_part), "p").is_err());

    // Copies numbered row by row, cut in a serpentine.
    let grid = job.parts[0].copies.map(|mut c| {
        c.columns = 3;
        c.rows = 2;
        c.dx = 10.0;
        c.dy = 20.0;
        c
    });
    let order = grid.unwrap().in_cut_order();
    assert_eq!(
        order,
        [
            (1, (0.0, 0.0)),
            (2, (10.0, 0.0)),
            (3, (20.0, 0.0)),
            (6, (20.0, 20.0)),
            (5, (10.0, 20.0)),
            (4, (0.0, 20.0)),
        ]
    );

    for bad in [
        json!({"columns": 0, "rows": 1, "dx": 10, "dy": 10}),
        json!({"columns": 101, "rows": 1, "dx": 10, "dy": 10}),
        json!({"columns": 40, "rows": 26, "dx": 10, "dy": 10}),
        json!({"columns": 2, "rows": 1, "dx": 0, "dy": 10}),
        json!({"columns": 1, "rows": 2, "dx": 10, "dy": 0}),
        json!({"columns": -1, "rows": 1, "dx": 10, "dy": 10}),
        json!({"columns": 2, "rows": 1, "dx": 10, "dy": 10, "order": "random"}),
        json!({"columns": 2, "rows": 1, "dx": 10, "dy": 10, "gap": 3}),
    ] {
        assert!(parse_job(with_copies(bad.clone()), "p").is_err(), "{bad}");
    }
    // At the limits.
    for good in [
        json!({"columns": 100, "rows": 10, "dx": 10, "dy": 10}),
        json!({"columns": 2, "rows": 1, "dx": 10, "dy": 0}),
    ] {
        assert!(parse_job(with_copies(good.clone()), "p").is_ok(), "{good}");
    }
}

// -- parts (docs/specs/parts.md)

/// docs/specs/library-in-browser.md §2: a named tool or machine is linked to the library
/// entry of that name, by the uid both get from the name.
#[test]
fn v22_to_v23_links_named_tools_and_the_machine_by_the_uid_of_their_name() {
    let mut v22 = raw("v22/two_parts.chipload");
    v22["tools"][0]["name"] = json!("6 mm Upcut");
    v22["machine"]["name"] = json!("Shapeoko");
    let job = parse_job(migrate(v22.clone()).unwrap(), "p").unwrap();
    let uid = chipload_job::library::uid_for_name;
    assert_eq!(job.tools[0].library_uid, Some(uid("6 mm upcut")));
    assert_eq!(job.machine.library_uid, Some(uid("shapeoko")));
    // Each named tool gets the uid of its name; one without a name is not from the library.
    for tool in &job.tools {
        assert_eq!(tool.library_uid, tool.name.as_deref().map(uid));
    }
    // Nothing else changes.
    let mut current = migrate(v22.clone()).unwrap();
    without_rotation(&mut current);
    for tool in current["tools"].as_array_mut().unwrap() {
        tool.as_object_mut().unwrap().remove("library_uid");
    }
    current["machine"]
        .as_object_mut()
        .unwrap()
        .remove("library_uid");
    v22["schema_version"] = json!(30);
    v22["machine"]["gcode_units"] = json!("mm");
    assert_eq!(stripped(&current), stripped(&v22));
}

#[test]
fn v21_to_v22_puts_every_operation_in_one_part_with_the_copies() {
    let mut v21 = lower_job();
    // As a v21 file: no parts, copies at the top, the old order name.
    let map = v21.as_object_mut().unwrap();
    map.remove("parts");
    map.remove("active_part");
    for op in map["operations"].as_array_mut().unwrap() {
        op.as_object_mut().unwrap().remove("part");
    }
    map.insert(
        "copies".into(),
        json!({"columns": 2, "rows": 1, "dx": 50, "dy": 0, "order": "by_part"}),
    );
    map.insert("schema_version".into(), json!(21));
    let job = parse_job(migrate(v21).unwrap(), "p").unwrap();
    assert_eq!(job.parts.len(), 1);
    let part = &job.parts[0];
    assert_eq!(
        (part.id, part.name.as_str(), part.enabled),
        (1, "Part 1", true)
    );
    let copies = part.copies.unwrap();
    assert_eq!(
        (copies.columns, copies.order),
        (2, chipload_job::CopyOrder::ByCopy)
    );
    assert!(job.operations.iter().all(|op| op.part() == 1));
    assert_eq!(job.active_part, None);
    assert_eq!(job.active_part_id(), Some(1));
}

#[test]
fn part_rules_are_enforced() {
    let refused = |data: Value, why: &str| {
        let err = parse_job(data.clone(), "p").unwrap_err().to_string();
        assert!(err.contains(why), "{why}: {err}");
    };
    let two = || {
        let mut data = lower_job();
        data["parts"] = json!([
            {"id": 1, "name": "lid", "enabled": true, "copies": null},
            {"id": 2, "name": "box", "enabled": true, "copies": null},
        ]);
        data["operations"][1]["part"] = json!(2);
        data
    };
    parse_job(two(), "p").unwrap();
    let mut data = two();
    data["parts"] = json!([]);
    refused(data, "parts: a project needs at least one part");
    let mut data = two();
    data["parts"][1]["id"] = json!(1);
    refused(data, "parts.1.id: part 1 is listed twice");
    let mut data = two();
    data["parts"][1]["name"] = json!(" LID");
    refused(data, "parts.1.name: another part is named");
    let mut data = two();
    data["parts"][1]["name"] = json!("  ");
    refused(data, "parts.1.name: must not be empty");
    let mut data = two();
    data["operations"][1]["part"] = json!(7);
    refused(data, "operations.1.part: no part 7 in parts");
    // Out of the parts' order: part 2's operation before part 1's.
    let mut data = two();
    data["operations"][0]["part"] = json!(2);
    data["operations"][1]["part"] = json!(1);
    refused(
        data,
        "operations.1.part: part 1's operations must all come before part 2's",
    );
    let mut data = two();
    data["parts"][0]["copies"] = json!({"columns": 0, "rows": 1, "dx": 10, "dy": 10});
    refused(data, "parts.0.copies.columns");
    // A part without operations is fine; an active part that doesn't exist means the first.
    let mut data = two();
    data["operations"][1]["part"] = json!(1);
    data["active_part"] = json!(9);
    let job = parse_job(data, "p").unwrap();
    assert_eq!(job.active_part_id(), Some(1));
    // A part that is off: its operations aren't cut.
    let mut data = two();
    data["parts"][1]["enabled"] = json!(false);
    let job = parse_job(data, "p").unwrap();
    assert!(job.cuts(0) && !job.cuts(1));
    assert_eq!(job.insert_index(1), 1);
    assert_eq!(job.insert_index(2), 2);
}

// -- placement of the imported drawing (docs/specs/drawing-placement.md)

#[test]
fn v7_to_v8_places_an_imported_drawing_where_the_dxf_has_it() {
    let v7 = raw("v7/simple_part_full.chipload");
    let shapes = v7["drawing"]["shapes"].clone();
    let current = migrate(v7).unwrap();
    assert_eq!(
        current["drawing"]["dxfs"][0]["placement"],
        json!({"scale": 1.0, "rotation": 0.0, "dx": 0.0, "dy": 0.0})
    );
    assert_eq!(
        without_dxf_ids(&current)["drawing"]["shapes"],
        shapes,
        "nothing moves"
    );
    // Without an imported drawing there is nothing to place.
    let drawn = migrate(raw("v7/leads.chipload")).unwrap();
    assert_eq!(drawn["drawing"]["dxfs"], json!([]));
    load_job(&corpus().join("v7/simple_part_full.chipload")).unwrap();
}

#[test]
fn a_v8_drawing_source_needs_a_finite_placement() {
    let good = migrate(raw("v7/simple_part_full.chipload")).unwrap();
    let mut missing = good.clone();
    missing["drawing"]["dxfs"][0]
        .as_object_mut()
        .unwrap()
        .remove("placement");
    assert!(parse_job(missing, "p").is_err());
    let mut stray = good.clone();
    stray["drawing"]["dxfs"][0]["placement"]["angle"] = json!(0.0);
    assert!(parse_job(stray, "p").is_err());
    let mut moved = good;
    moved["drawing"]["dxfs"][0]["placement"] =
        json!({"scale": 1.0, "rotation": 0.0, "dx": -12.5, "dy": 3.0});
    let mut job = parse_job(moved, "p").unwrap();
    let source = job.drawing.dxfs.first_mut().unwrap();
    assert_eq!(
        source.placement,
        chipload_job::Placement {
            scale: 1.0,
            rotation: 0.0,
            dx: -12.5,
            dy: 3.0
        }
    );
    // JSON can't hold a non-finite number, but a job built in code can.
    source.placement.dy = f64::NAN;
    assert!(
        job.validate()
            .iter()
            .any(|p| p.path.starts_with("drawing.dxfs.0.placement"))
    );
}

// -- turning the imported drawing (docs/specs/drawing-rotation.md)

#[test]
fn v8_to_v9_gives_a_placement_no_turn() {
    let v8 = raw("v8/placed_part.chipload");
    let shapes = v8["drawing"]["shapes"].clone();
    let current = migrate(v8).unwrap();
    assert_eq!(
        current["drawing"]["dxfs"][0]["placement"],
        json!({"scale": 1.0, "rotation": 0.0, "dx": -20.0, "dy": 30.0})
    );
    assert_eq!(
        without_dxf_ids(&current)["drawing"]["shapes"],
        shapes,
        "nothing moves"
    );
}

#[test]
fn rotations_are_at_least_0_and_under_360() {
    // docs/specs/direct-placement.md §3.
    let good = migrate(raw("v8/placed_part.chipload")).unwrap();
    for (degrees, ok) in [
        (json!(0.0), true),
        (json!(37.5), true),
        (json!(359.9), true),
        (json!(360.0), false),
        (json!(-1.0), false),
    ] {
        let mut data = good.clone();
        data["drawing"]["dxfs"][0]["placement"]["rotation"] = degrees.clone();
        assert_eq!(parse_job(data, "p").is_ok(), ok, "{degrees}");
    }
}

/// docs/specs/direct-placement.md §3: quarter turns become degrees, images get rotation 0,
/// and nothing else changes.
#[test]
fn v23_to_v24_makes_quarter_turns_degrees() {
    let v23 = raw("v23/turned_parts.chipload");
    let mut current = migrate(v23.clone()).unwrap();
    assert_eq!(current["schema_version"], json!(30));
    without_gcode_units(&mut current);
    let dxfs = current["drawing"]["dxfs"].as_array_mut().unwrap();
    for (dxf, before) in dxfs
        .iter_mut()
        .zip(v23["drawing"]["dxfs"].as_array().unwrap())
    {
        let placement = dxf["placement"].as_object_mut().unwrap();
        let turns = before["placement"]["quarter_turns"].as_f64().unwrap();
        assert_eq!(placement.remove("rotation"), Some(json!(90.0 * turns)));
        placement.insert(
            "quarter_turns".into(),
            before["placement"]["quarter_turns"].clone(),
        );
    }
    for image in current["drawing"]["images"].as_array_mut().unwrap() {
        assert_eq!(
            image.as_object_mut().unwrap().remove("rotation"),
            Some(json!(0.0))
        );
    }
    without_corner_radius(&mut current);
    current["schema_version"] = json!(23);
    assert_eq!(stripped(&current), stripped(&v23));
}

/// docs/specs/rectangle-corner-radius.md §3: every rectangle gets corner radius 0, and
/// nothing else changes.
#[test]
fn v24_to_v25_gives_rectangles_sharp_corners() {
    let v24 = raw("v24/rectangle_part.chipload");
    let mut current = migrate(v24.clone()).unwrap();
    assert_eq!(current["schema_version"], json!(30));
    assert_eq!(current["drawing"]["shapes"][0]["corner_radius"], json!(0.0));
    let job = parse_job(current.clone(), "p").unwrap();
    assert!(matches!(
        job.drawing.shapes[0].geometry,
        chipload_job::Geometry::Rectangle { corner_radius, .. } if corner_radius == 0.0
    ));
    without_gcode_units(&mut current);
    without_corner_radius(&mut current);
    current["schema_version"] = json!(24);
    assert_eq!(stripped(&current), stripped(&v24));
}

// -- V-carving (docs/specs/v-carve.md)

#[test]
fn v9_to_v10_changes_nothing_but_the_version() {
    let v9 = raw("v9/turned_part.chipload");
    let mut current = migrate(v9.clone()).unwrap();
    assert_eq!(current["schema_version"], json!(30));
    without_gcode_units(&mut current);
    without_images(&mut current);
    without_custom_post(&mut current);
    without_flank_angle(&mut current);
    without_clearing_settings(&mut current);
    // Only v10 -> v11 and v11 -> v12 add something: the scale and the tool change setting
    // (v12 -> v13 only touches adaptive pockets, and this has none).
    current["schema_version"] = json!(9);
    current["drawing"]["source"]["placement"]
        .as_object_mut()
        .unwrap()
        .remove("scale");
    current["machine"]
        .as_object_mut()
        .unwrap()
        .remove("tool_change");
    assert_eq!(stripped(&current), stripped(&v9));
    let job = parse_job(migrate(v9).unwrap(), "p").unwrap();
    assert!(
        job.tools
            .iter()
            .all(|t| t.angle.is_none() && t.tip_diameter.is_none())
    );
}

/// A v-bit and a V-carve operation, as current-version data.
fn vcarve_job() -> Value {
    let mut data = lower_job();
    data["tools"][0]["type"] = json!("vbit");
    data["tools"][0]["angle"] = json!(std::f64::consts::FRAC_PI_3);
    data["tools"][0]["tip_diameter"] = json!(0.2);
    data["operations"] = json!([{
        "type": "vcarve", "part": 1, "name": "Letters", "select": {"layer": "TEXT"}, "tool": 1,
        "start_depth": 0.0, "end_depth": 3.0, "step_down": null,
        "feed": 1200.0, "plunge_feed": 400.0
    }]);
    data
}

#[test]
fn a_vcarve_operation_loads() {
    let job = parse_job(vcarve_job(), "p").unwrap();
    let Operation::Vcarve(v) = &job.operations[0] else {
        panic!("a vcarve operation");
    };
    assert_eq!((v.start_depth, v.end_depth, v.step_down), (0.0, 3.0, None));
    assert_eq!(job.tools[0].tip_diameter, Some(0.2));
    // The optional fields may be left out.
    let mut data = vcarve_job();
    let op = data["operations"][0].as_object_mut().unwrap();
    op.remove("step_down");
    op.remove("name");
    assert!(parse_job(data, "p").is_ok());
}

#[test]
fn vbit_angle_tip_and_vcarve_values_are_checked() {
    let cases = [
        ("/tools/0/angle", json!(0.0), "angle"),
        ("/tools/0/angle", json!(std::f64::consts::PI), "angle"),
        ("/tools/0/tip_diameter", json!(-0.1), "tip_diameter"),
        ("/tools/0/tip_diameter", json!(1000.0), "tip_diameter"),
        ("/operations/0/step_down", json!(0.0), "step_down"),
        ("/operations/0/feed", json!(0.0), "feed"),
        ("/operations/0/end_depth", json!(0.0), "end_depth"),
    ];
    for (pointer, value, field) in cases {
        let mut data = vcarve_job();
        *data.pointer_mut(pointer).unwrap() = value.clone();
        let err = parse_job(data, "p").unwrap_err().to_string();
        assert!(err.contains(field), "{pointer} = {value}: {err}");
    }
    let mut data = vcarve_job();
    data["operations"][0]["side"] = json!("inside");
    assert!(parse_job(data, "p").is_err(), "a V-carve has no side");
}

// -- scaling the imported drawing (docs/specs/drawing-scale.md)

#[test]
fn v10_to_v11_gives_a_placement_scale_1() {
    let v10 = raw("v10/vcarve_letters.chipload");
    let mut current = migrate(v10.clone()).unwrap();
    assert_eq!(current["schema_version"], json!(30));
    without_gcode_units(&mut current);
    without_images(&mut current);
    without_custom_post(&mut current);
    without_flank_angle(&mut current);
    without_clearing_settings(&mut current);
    assert_eq!(
        current["drawing"]["source"]["placement"]["scale"],
        json!(1.0)
    );
    current["schema_version"] = json!(10);
    current["drawing"]["source"]["placement"]
        .as_object_mut()
        .unwrap()
        .remove("scale");
    current["machine"]
        .as_object_mut()
        .unwrap()
        .remove("tool_change");
    assert_eq!(stripped(&current), stripped(&v10), "nothing else changes");
    let job = parse_job(migrate(v10).unwrap(), "p").unwrap();
    assert_eq!(
        job.drawing.dxfs[0].placement,
        chipload_job::Placement::default()
    );
}

#[test]
fn the_scale_is_required_and_in_range() {
    let good = migrate(raw("v10/vcarve_letters.chipload")).unwrap();
    for (scale, ok) in [
        (json!(1.0), true),
        (json!(0.001), true),
        (json!(1000.0), true),
        (json!(25.4), true),
        (json!(0.0), false),
        (json!(-2.0), false),
        (json!(0.0009), false),
        (json!(1000.5), false),
        (json!("2"), false),
        (Value::Null, false),
    ] {
        let mut data = good.clone();
        data["drawing"]["dxfs"][0]["placement"]["scale"] = scale.clone();
        assert_eq!(parse_job(data, "p").is_ok(), ok, "{scale}");
    }
    let mut missing = good;
    missing["drawing"]["dxfs"][0]["placement"]
        .as_object_mut()
        .unwrap()
        .remove("scale");
    assert!(parse_job(missing, "p").is_err());
}

// -- a tool per operation (docs/specs/tool-per-operation.md)

#[test]
fn v11_to_v12_gives_the_machine_one_file_per_tool() {
    for sample in ["v11/scaled_part.chipload", "v11/two_tools.chipload"] {
        let v11 = raw(sample);
        let mut current = migrate(v11.clone()).unwrap();
        assert_eq!(current["schema_version"], json!(30));
        without_gcode_units(&mut current);
        without_images(&mut current);
        without_custom_post(&mut current);
        without_flank_angle(&mut current);
        without_clearing_settings(&mut current);
        assert_eq!(current["machine"]["tool_change"], json!("files"));
        current["schema_version"] = json!(11);
        current["machine"]
            .as_object_mut()
            .unwrap()
            .remove("tool_change");
        assert_eq!(
            stripped(&current),
            stripped(&v11),
            "{sample}: nothing else changes"
        );
        let job = parse_job(migrate(v11).unwrap(), "p").unwrap();
        assert_eq!(job.machine.tool_change, chipload_job::ToolChange::Files);
    }
}

#[test]
fn the_tool_change_setting_is_required_and_known() {
    let good = migrate(raw("v11/two_tools.chipload")).unwrap();
    for (value, ok) in [
        (json!("files"), true),
        (json!("program"), true),
        (json!("m6"), false),
        (json!("Files"), false),
        (Value::Null, false),
    ] {
        let mut data = good.clone();
        data["machine"]["tool_change"] = value.clone();
        assert_eq!(parse_job(data, "p").is_ok(), ok, "{value}");
    }
    let mut missing = good;
    missing["machine"]
        .as_object_mut()
        .unwrap()
        .remove("tool_change");
    assert!(parse_job(missing, "p").is_err());
}

// -- adaptive clearing cuts the stepover (docs/specs/adaptive-fronts.md §5)

#[test]
fn v12_to_v13_makes_the_adaptive_load_the_stepover() {
    let v12 = raw("v12/adaptive.chipload");
    assert_eq!(v12["operations"][0]["strategy"]["load"], json!(0.1));
    assert_eq!(v12["operations"][0]["stepover"], json!(0.4));
    let mut current = migrate(v12.clone()).unwrap();
    assert_eq!(current["schema_version"], json!(30));
    without_gcode_units(&mut current);
    without_images(&mut current);
    without_custom_post(&mut current);
    without_flank_angle(&mut current);
    without_clearing_settings(&mut current);
    // The cut stays as wide as it was: 0.1 × D, not the 0.4 adaptive never used. The
    // strategy itself is gone since v26.
    assert_eq!(current["operations"][0]["stepover"], json!(0.1));
    current["schema_version"] = json!(12);
    current["operations"][0]["stepover"] = json!(0.4);
    assert_eq!(stripped(&current), stripped(&v12), "nothing else changes");
    let job = parse_job(migrate(v12).unwrap(), "p").unwrap();
    let Operation::Pocket(pocket) = &job.operations[0] else {
        panic!("a pocket");
    };
    assert_eq!(pocket.stepover, 0.1);

    // Other pockets keep their stepover; an adaptive without a load can't be carried on.
    let rings = raw("v12/two_tools_program.chipload");
    let mut migrated = migrate(rings.clone()).unwrap();
    without_custom_post(&mut migrated);
    without_flank_angle(&mut migrated);
    without_clearing_settings(&mut migrated);
    for op in migrated["operations"].as_array_mut().unwrap() {
        op.as_object_mut().unwrap().remove("part");
    }
    assert_eq!(
        stripped(&migrated)["operations"],
        stripped(&rings)["operations"]
    );
    let mut broken = raw("v12/adaptive.chipload");
    broken["operations"][0]["strategy"]
        .as_object_mut()
        .unwrap()
        .remove("load");
    let err = migrate(broken).unwrap_err().to_string();
    assert!(err.contains("adaptive without a numeric load"), "{err}");
}

/// Undo v15 -> v16 (`flank_angle` on tabs), to compare with an older sample.
/// Undo v27 -> v28: the machine's G-code units, mm in every older file
/// (docs/specs/inch-units.md §4.1).
fn without_gcode_units(data: &mut Value) {
    let machine = data["machine"].as_object_mut().unwrap();
    assert_eq!(machine.remove("gcode_units"), Some(json!("mm")));
    data["schema_version"] = json!(27);
}

fn without_flank_angle(data: &mut Value) {
    if let Some(ops) = data["operations"].as_array_mut() {
        for op in ops.iter_mut() {
            if let Some(tabs) = op.get_mut("tabs").and_then(Value::as_object_mut) {
                tabs.remove("flank_angle");
            }
        }
    }
}

/// Undo v14 -> v15 (`custom_post` on the machine), to compare with an older sample.
/// v18 -> v19 adds an empty image list (docs/specs/image-trace.md §4): take it out, so a
/// test of an older step sees only what that step did.
/// Undo v18 -> v19 (`images`) and every later step, to compare with an older sample.
fn without_images(data: &mut Value) {
    without_dxfs(data);
    let drawing = data["drawing"].as_object_mut().unwrap();
    assert_eq!(drawing.remove("images"), Some(json!([])));
}

/// The data with its shapes' `dxf` (added by v20 -> v21) left out.
fn without_dxf_ids(data: &Value) -> Value {
    let mut data = data.clone();
    for shape in data["drawing"]["shapes"].as_array_mut().unwrap() {
        shape.as_object_mut().unwrap().remove("dxf");
    }
    data
}

/// Undo v21 -> v22: the one part's copies back at the top (`by_copy` back to `by_part`),
/// operations without `part`.
/// Undo v22 -> v23: the library links, each the uid of the name or null.
/// Undo v23 -> v24: rotation in degrees back to quarter turns, images without rotation.
fn without_rotation(data: &mut Value) {
    without_corner_radius(data);
    let drawing = data["drawing"].as_object_mut().unwrap();
    if let Some(dxfs) = drawing.get_mut("dxfs").and_then(Value::as_array_mut) {
        for dxf in dxfs {
            let placement = dxf["placement"].as_object_mut().unwrap();
            let degrees = placement.remove("rotation").unwrap().as_f64().unwrap();
            placement.insert("quarter_turns".into(), json!((degrees / 90.0) as u64));
        }
    }
    if let Some(images) = drawing.get_mut("images").and_then(Value::as_array_mut) {
        for image in images {
            assert_eq!(
                image.as_object_mut().unwrap().remove("rotation"),
                Some(json!(0.0))
            );
        }
    }
}

/// Undo v24 -> v25: rectangles without their corner radius, 0 for all of them.
fn without_corner_radius(data: &mut Value) {
    let Some(shapes) = data["drawing"]
        .get_mut("shapes")
        .and_then(Value::as_array_mut)
    else {
        return;
    };
    for shape in shapes {
        if shape["kind"] == json!("rectangle") {
            assert_eq!(
                shape.as_object_mut().unwrap().remove("corner_radius"),
                Some(json!(0.0))
            );
        }
    }
}

fn without_links(data: &mut Value) {
    without_rotation(data);
    let unlink = |entry: &mut Value| {
        let entry = entry.as_object_mut().unwrap();
        let expected = match entry.get("name").and_then(Value::as_str) {
            Some(name) => json!(chipload_job::library::uid_for_name(name)),
            None => Value::Null,
        };
        assert_eq!(entry.remove("library_uid"), Some(expected));
    };
    let map = data.as_object_mut().unwrap();
    for tool in map["tools"].as_array_mut().unwrap() {
        unlink(tool);
    }
    unlink(&mut map["machine"]);
}

fn without_parts(data: &mut Value) {
    without_links(data);
    let map = data.as_object_mut().unwrap();
    assert_eq!(map.remove("active_part"), Some(Value::Null));
    let mut parts = map.remove("parts").unwrap();
    let parts = parts.as_array_mut().unwrap();
    assert_eq!(parts.len(), 1);
    let mut copies = parts[0]["copies"].take();
    if copies["order"] == json!("by_copy") {
        copies["order"] = json!("by_part");
    }
    map.insert("copies".into(), copies);
    for op in map["operations"].as_array_mut().unwrap() {
        assert_eq!(op.as_object_mut().unwrap().remove("part"), Some(json!(1)));
    }
}

/// Undo v20 -> v21: DXF 1 back to `source`, shapes without `dxf`, nothing hidden.
fn without_dxfs(data: &mut Value) {
    without_parts(data);
    let drawing = data["drawing"].as_object_mut().unwrap();
    assert_eq!(drawing.remove("hidden"), Some(json!([])));
    let mut dxfs = drawing.remove("dxfs").unwrap();
    let source = match dxfs.as_array_mut().unwrap().pop() {
        Some(mut source) => {
            let source_map = source.as_object_mut().unwrap();
            assert_eq!(source_map.remove("id"), Some(json!(1)));
            source_map.remove("name").unwrap();
            source
        }
        None => Value::Null,
    };
    drawing.insert("source".into(), source);
    for shape in drawing["shapes"].as_array_mut().unwrap() {
        shape.as_object_mut().unwrap().remove("dxf");
    }
}

fn without_custom_post(data: &mut Value) {
    if let Some(machine) = data["machine"].as_object_mut() {
        machine.remove("custom_post");
    }
}

/// Take the clearing settings (`keep_tool_down` since v3, `strategy` since v7,
/// `round_corners` v14 to v25) off a sample's pockets, to compare with the current
/// version, which has none (v26).
fn without_clearing_settings(data: &mut Value) {
    if let Some(ops) = data["operations"].as_array_mut() {
        for op in ops.iter_mut().filter_map(Value::as_object_mut) {
            if op.get("type").and_then(Value::as_str) == Some("pocket") {
                op.remove("keep_tool_down");
                op.remove("strategy");
                op.remove("round_corners");
            }
        }
    }
}

/// Take a profile's `entry` and `lead` (v26 and before) off a sample, to compare with the
/// current version, which has `ramp_angle` instead (v27).
fn without_profile_entry(data: &mut Value) {
    if let Some(ops) = data["operations"].as_array_mut() {
        for op in ops.iter_mut().filter_map(Value::as_object_mut) {
            if op.get("type").and_then(Value::as_str) == Some("profile") {
                op.remove("entry");
                op.remove("lead");
            }
        }
    }
}

/// Undo v26 -> v27 (`ramp_angle` on profiles), to compare with an older sample.
fn without_ramp_angle(data: &mut Value) {
    if let Some(ops) = data["operations"].as_array_mut() {
        for op in ops.iter_mut().filter_map(Value::as_object_mut) {
            if op.get("type").and_then(Value::as_str) == Some("profile") {
                op.remove("ramp_angle");
            }
        }
    }
}

/// A sample without its clearing settings and its profiles' entry, lead and ramp angle,
/// to compare an old version with the current one.
fn stripped(data: &Value) -> Value {
    let mut out = data.clone();
    without_clearing_settings(&mut out);
    without_profile_entry(&mut out);
    without_ramp_angle(&mut out);
    out
}

// -- rounded ring corners (docs/specs/pocket-round-corners.md)

#[test]
fn v13_to_v14_added_round_corners_which_v26_took_off_again() {
    let v13 = raw("v13/helix_rings.chipload");
    let mut current = migrate(v13.clone()).unwrap();
    assert_eq!(current["schema_version"], json!(30));
    without_gcode_units(&mut current);
    without_images(&mut current);
    assert!(current["operations"][0].get("round_corners").is_none());
    current["schema_version"] = json!(13);
    without_custom_post(&mut current);
    without_flank_angle(&mut current);
    assert_eq!(stripped(&current), stripped(&v13), "nothing else changes");
    let job = parse_job(migrate(v13).unwrap(), "p").unwrap();
    assert!(matches!(job.operations[0], Operation::Pocket(_)));
}

// -- user post-processors (docs/specs/user-posts.md)

#[test]
fn v14_to_v15_keeps_the_built_in_post() {
    let v14 = raw("v14/round_corners.chipload");
    let mut current = migrate(v14.clone()).unwrap();
    assert_eq!(current["schema_version"], json!(30));
    without_gcode_units(&mut current);
    without_images(&mut current);
    assert_eq!(current["machine"]["custom_post"], Value::Null);
    current["schema_version"] = json!(14);
    without_custom_post(&mut current);
    without_flank_angle(&mut current);
    assert_eq!(stripped(&current), stripped(&v14), "nothing else changes");
    let job = parse_job(migrate(v14).unwrap(), "p").unwrap();
    assert_eq!(job.machine.post, "grbl");
    assert!(job.machine.custom_post.is_none());
}

#[test]
fn a_copied_post_round_trips_and_stray_fields_are_refused() {
    let mut data = migrate(raw("v14/round_corners.chipload")).unwrap();
    data["machine"]["post"] = json!("mine");
    data["machine"]["custom_post"] = json!({"config": "schema_version = 1\n", "template": "M30\n"});
    let job = parse_job(data.clone(), "p").unwrap();
    let post = job.machine.custom_post.clone().unwrap();
    assert_eq!(post.template, "M30\n");
    let dir = temp_dir("custom-post");
    let path = dir.join("p.chipload");
    chipload_job::save_job(&job, &path).unwrap();
    assert_eq!(load_job(&path).unwrap().machine.custom_post, Some(post));
    std::fs::remove_dir_all(&dir).unwrap();
    data["machine"]["custom_post"]["extra"] = json!(1);
    assert!(parse_job(data, "p").is_err());
}

// -- problems with their fields (docs/specs/live-problems.md)

#[test]
fn every_value_rule_names_its_field() {
    let mut job = load_job(&corpus().join("v14/round_corners.chipload")).unwrap();
    job.stock.x = 0.0;
    job.stock.thickness = -1.0;
    job.stock.max_overcut = -1.0;
    job.tools[0].diameter = 0.0;
    if let Operation::Pocket(p) = &mut job.operations[0] {
        p.feed = 0.0;
        p.stepover = 0.0;
        p.start_depth = 9.0;
    }
    let problems = job.validate();
    assert!(problems.len() >= 7, "{problems:?}");
    for problem in &problems {
        assert!(!problem.path.is_empty(), "{problem:?}");
        assert_eq!(problem.text, format!("{}: {}", problem.path, problem.short));
    }
    let paths: Vec<&str> = problems.iter().map(|p| p.path.as_str()).collect();
    for path in [
        "stock.x",
        "stock.thickness",
        "stock.max_overcut",
        "tools.0.diameter",
        "operations.0.feed",
        "operations.0.stepover",
        "operations.0.start_depth",
    ] {
        assert!(paths.contains(&path), "{path} in {paths:?}");
    }
}

// -- ramped tabs (docs/specs/profile-spiral.md)

#[test]
fn v15_to_v16_keeps_plunged_tabs_straight() {
    let v15 = raw("v15/straight_tabs.chipload");
    let mut current = migrate(v15.clone()).unwrap();
    assert_eq!(current["schema_version"], json!(30));
    without_gcode_units(&mut current);
    without_images(&mut current);
    assert_eq!(
        current["operations"][1]["tabs"]["flank_angle"],
        json!(std::f64::consts::FRAC_PI_2)
    );
    current["schema_version"] = json!(15);
    without_flank_angle(&mut current);
    assert_eq!(stripped(&current), stripped(&v15), "nothing else changes");
    let job = parse_job(migrate(v15).unwrap(), "p").unwrap();
    let Operation::Profile(profile) = &job.operations[1] else {
        panic!("a profile");
    };
    assert!(profile.tabs.as_ref().unwrap().is_straight());
}

#[test]
fn v15_to_v16_gives_ramped_tabs_the_default_flank() {
    // The tool isn't centre-cutting: the flank is the ramp angle (3°).
    let v15 = raw("v15/linuxcnc_helix.chipload");
    let current = migrate(v15.clone()).unwrap();
    let ramp_angle = v15["operations"][1]["entry"]["ramp_angle"].clone();
    assert_eq!(current["operations"][1]["tabs"]["flank_angle"], ramp_angle);
    // A tool that can plunge (or isn't marked): 30°.
    for marked in [json!(true), Value::Null] {
        let mut data = v15.clone();
        data["tools"][0]["center_cutting"] = marked;
        let current = migrate(data).unwrap();
        assert_eq!(
            current["operations"][1]["tabs"]["flank_angle"],
            json!(std::f64::consts::PI / 6.0)
        );
    }
    // Pockets have no tabs and get nothing.
    assert!(
        current["operations"][0]
            .get("tabs")
            .is_none_or(Value::is_null)
    );
    // Required from v16 on.
    let mut missing = current;
    missing["operations"][1]["tabs"]
        .as_object_mut()
        .unwrap()
        .remove("flank_angle");
    assert!(parse_job(missing, "p").is_err());
}

#[test]
fn a_flank_angle_must_be_more_than_0_and_at_most_90_degrees() {
    let data = migrate(raw("v15/straight_tabs.chipload")).unwrap();
    for (angle, ok) in [
        (0.0, false),
        (-0.1, false),
        (0.01, true),
        (std::f64::consts::FRAC_PI_2, true),
        (90f64.to_radians(), true),
        (1.6, false),
    ] {
        let mut data = data.clone();
        data["operations"][1]["tabs"]["flank_angle"] = json!(angle);
        let problems = parse_job(data, "p").map(|job| job.validate());
        let refused = match problems {
            Ok(problems) => problems
                .iter()
                .any(|p| p.path == "operations.1.tabs.flank_angle"),
            Err(_) => true,
        };
        assert_eq!(refused, !ok, "{angle}");
    }
}

#[test]
fn new_tabs_get_a_flank_that_suits_the_tool() {
    use chipload_job::{DEFAULT_RAMPED_FLANK_RAD, Tabs};
    assert_eq!(
        Tabs::default_flank(0.05, Some(true)),
        DEFAULT_RAMPED_FLANK_RAD
    );
    assert_eq!(Tabs::default_flank(0.05, None), DEFAULT_RAMPED_FLANK_RAD);
    // Going down a flank is a ramp: no steeper than the ramp for a tool that can't plunge.
    assert_eq!(Tabs::default_flank(0.05, Some(false)), 0.05);
}

// -- follow path (docs/specs/follow-path.md)

#[test]
fn v16_to_v17_changes_only_the_version() {
    let v16 = raw("v16/profile_spiral.chipload");
    let mut current = migrate(v16.clone()).unwrap();
    assert_eq!(current["schema_version"], json!(30));
    without_gcode_units(&mut current);
    without_images(&mut current);
    current["schema_version"] = json!(16);
    assert_eq!(stripped(&current), stripped(&v16), "nothing else changes");
}

#[test]
fn a_follow_path_round_trips() {
    let job = load_job(&corpus().join("v17/follow_path.chipload")).unwrap();
    let Operation::Path(path) = &job.operations[0] else {
        panic!("a follow path");
    };
    assert_eq!(path.select.shapes, Some(vec![4, 2]));
    let dir = temp_dir("follow-path");
    let file = dir.join("p.chipload");
    chipload_job::save_job(&job, &file).unwrap();
    assert_eq!(
        unversioned(load_job(&file).unwrap()),
        unversioned(job.clone())
    );
    std::fs::remove_dir_all(&dir).unwrap();
    // A side or tabs mean nothing on the line: refused as unknown fields.
    let mut data = migrate(raw("v17/follow_path.chipload")).unwrap();
    data["operations"][0]["side"] = json!("outside");
    assert!(parse_job(data, "p").is_err());
}

// -- holes (docs/specs/holes.md)

#[test]
fn v17_to_v18_changes_only_the_version() {
    for sample in ["v17/follow_path.chipload", "v17/finish_skin.chipload"] {
        let v17 = raw(sample);
        let mut current = migrate(v17.clone()).unwrap();
        assert_eq!(current["schema_version"], json!(30));
        without_gcode_units(&mut current);
        without_images(&mut current);
        current["schema_version"] = json!(17);
        assert_eq!(
            stripped(&current),
            stripped(&v17),
            "{sample}: nothing else changes"
        );
    }
}

#[test]
fn a_holes_operation_round_trips_and_its_values_are_checked() {
    let job = load_job(&corpus().join("v18/holes.chipload")).unwrap();
    let Operation::Holes(drilled) = &job.operations[0] else {
        panic!("holes");
    };
    assert_eq!(drilled.method, HoleMethod::Drill { peck: Some(4.0) });
    let dir = temp_dir("holes");
    let file = dir.join("h.chipload");
    chipload_job::save_job(&job, &file).unwrap();
    assert_eq!(
        unversioned(load_job(&file).unwrap()),
        unversioned(job.clone())
    );
    std::fs::remove_dir_all(&dir).unwrap();
    // A hole has no side or entry: refused as unknown fields.
    let mut data = migrate(raw("v18/holes.chipload")).unwrap();
    data["operations"][0]["entry"] = json!({"type": "plunge"});
    assert!(parse_job(data, "h").is_err());
    // Value rules name their field.
    let mut data = migrate(raw("v18/holes.chipload")).unwrap();
    data["operations"][0]["method"]["peck"] = json!(0.0);
    data["operations"][1]["method"]["step_down"] = json!(-1.0);
    data["operations"][1]["method"]["ramp_angle"] = json!(1.0);
    let err = parse_job(data, "h").unwrap_err().to_string();
    for field in [
        "operations.0.method.peck",
        "operations.1.method.step_down",
        "operations.1.method.ramp_angle",
    ] {
        assert!(err.contains(field), "{field}: {err}");
    }
}

#[test]
fn a_tool_gets_its_hole_method() {
    assert_eq!(
        HoleMethod::default_for(ToolType::Drill, 8.0),
        Some(HoleMethod::Drill { peck: None })
    );
    let Some(HoleMethod::Bore { step_down, .. }) = HoleMethod::default_for(ToolType::Endmill, 6.0)
    else {
        panic!("a bore");
    };
    assert_eq!(step_down, 3.0);
    assert_eq!(HoleMethod::default_for(ToolType::Vbit, 6.0), None);
}

// -- dogbones (docs/specs/dogbones.md)

#[test]
fn corners_round_trip_and_their_values_are_checked() {
    let job = load_job(&corpus().join("v18/dogbones.chipload")).unwrap();
    assert!(matches!(
        job.operations[1].corners(),
        Some(chipload_job::Corners::Tbone { .. })
    ));
    let dir = temp_dir("dogbones");
    let file = dir.join("d.chipload");
    chipload_job::save_job(&job, &file).unwrap();
    assert_eq!(
        unversioned(load_job(&file).unwrap()),
        unversioned(job.clone())
    );
    std::fs::remove_dir_all(&dir).unwrap();
    // A profile without the field loads as round corners.
    let mut data = migrate(raw("v18/dogbones.chipload")).unwrap();
    data["operations"][1]
        .as_object_mut()
        .unwrap()
        .remove("corners");
    let loaded = parse_job(data, "d").unwrap();
    assert_eq!(loaded.operations[1].corners(), None);
    // Out of range, and on the line.
    let mut data = migrate(raw("v18/dogbones.chipload")).unwrap();
    data["operations"][0]["corners"]["max_angle"] = json!(4.0);
    data["operations"][1]["side"] = json!("on");
    data["operations"][1]["clearance"] = json!(0.0);
    let err = parse_job(data, "d").unwrap_err().to_string();
    assert!(err.contains("operations.0.corners.max_angle"), "{err}");
    assert!(
        err.contains("operations.1.corners: a profile on the line has no inside corners"),
        "{err}"
    );
}

/// A job without `written_by`, which saving sets to this version: a frozen sample saved
/// again differs only there.
fn unversioned(mut job: chipload_job::Job) -> chipload_job::Job {
    job.written_by = None;
    job
}

#[test]
fn v18_to_v19_adds_an_empty_image_list() {
    for sample in [
        "v18/holes.chipload",
        "v18/dogbones.chipload",
        "v18/rest.chipload",
    ] {
        let v18 = raw(sample);
        let mut current = migrate(v18.clone()).unwrap();
        assert_eq!(current["schema_version"], json!(30));
        without_gcode_units(&mut current);
        without_images(&mut current);
        current["schema_version"] = json!(18);
        assert_eq!(
            stripped(&current),
            stripped(&v18),
            "{sample}: nothing else changes"
        );
    }
}

/// docs/specs/image-arcs-centrelines.md §4: images stay outlines without arcs, as traced
/// before; nothing else changes.
#[test]
fn v19_to_v20_keeps_images_as_outlines_without_arcs() {
    let v19 = raw("v19/image_vcarve.chipload");
    let mut current = migrate(v19.clone()).unwrap();
    assert_eq!(current["schema_version"], json!(30));
    without_gcode_units(&mut current);
    without_dxfs(&mut current);
    let images = current["drawing"]["images"].as_array_mut().unwrap();
    assert!(!images.is_empty());
    for image in images.iter_mut() {
        let image = image.as_object_mut().unwrap();
        assert_eq!(image.remove("trace"), Some(json!("outline")));
        assert_eq!(image.remove("arcs"), Some(json!(false)));
    }
    current["schema_version"] = json!(19);
    assert_eq!(stripped(&current), stripped(&v19), "nothing else changes");
}

/// docs/specs/drawing-sources.md §2: the DXF becomes DXF 1 of a list, named after its file,
/// and its shapes name it; nothing is hidden; nothing else changes.
#[test]
fn v20_to_v21_makes_the_dxf_the_first_of_a_list() {
    let v20 = raw("v20/scaled_part.chipload");
    let mut current = migrate(v20.clone()).unwrap();
    assert_eq!(current["schema_version"], json!(30));
    without_gcode_units(&mut current);
    let dxf = &current["drawing"]["dxfs"][0];
    assert_eq!(
        (&dxf["id"], &dxf["name"]),
        (&json!(1), &json!("simple_part"))
    );
    // v24 makes the quarter turn degrees.
    let mut placement = v20["drawing"]["source"]["placement"].clone();
    let turns = placement["quarter_turns"].as_f64().unwrap();
    let placement = placement.as_object_mut().unwrap();
    placement.remove("quarter_turns");
    placement.insert("rotation".into(), json!(90.0 * turns));
    assert_eq!(dxf["placement"], json!(placement));
    assert!(
        current["drawing"]["shapes"]
            .as_array()
            .unwrap()
            .iter()
            .all(|s| s["dxf"] == json!(1))
    );
    without_dxfs(&mut current);
    current["schema_version"] = json!(20);
    assert_eq!(stripped(&current), stripped(&v20), "nothing else changes");
    // Without a DXF: an empty list, and traced shapes name no DXF.
    let images = migrate(raw("v20/image_centre_line.chipload")).unwrap();
    assert_eq!(images["drawing"]["dxfs"], json!([]));
    assert!(
        images["drawing"]["shapes"]
            .as_array()
            .unwrap()
            .iter()
            .all(|s| s.get("dxf").is_none())
    );
}

#[test]
fn selectors_and_shapes_name_dxfs_that_exist() {
    let mut data = migrate(raw("v20/scaled_part.chipload")).unwrap();
    data["operations"][0]["select"]["dxf"] = json!(1);
    let job = parse_job(data.clone(), "p").unwrap();
    assert_eq!(job.operations[0].select().dxf, Some(1));
    let mut bad = data.clone();
    bad["operations"][0]["select"]["dxf"] = json!(7);
    bad["drawing"]["shapes"][0]["dxf"] = json!(9);
    bad["drawing"]["dxfs"][0]["name"] = json!(" ");
    let text = parse_job(bad, "p").unwrap_err().to_string();
    for wanted in [
        "operations.0.select.dxf: no DXF 7",
        "dxf: no DXF 9",
        "drawing.dxfs.0.name",
    ] {
        assert!(text.contains(wanted), "{wanted}: {text}");
    }
    let mut both = data;
    both["operations"][0]["select"]["image"] = json!(1);
    assert!(parse_job(both, "p").is_err());
}

#[test]
fn images_and_their_traced_shapes_are_checked() {
    let mut data = migrate(raw("v18/holes.chipload")).unwrap();
    let image = json!({
        "id": 1, "file": "logo.png", "sha256": "00", "data": "iVBORw0KGgo=",
        "layer": "logo", "x": 0.0, "y": 0.0, "width": 100.0,
        "threshold": "auto", "invert": false, "blur": 1.0, "min_area": 0.5, "smoothing": 0.05,
        "trace": "centre_line", "arcs": true, "rotation": 0.0
    });
    data["drawing"]["images"] = json!([image]);
    let job = parse_job(data.clone(), "p").unwrap();
    assert_eq!(job.drawing.images[0].threshold, ImageThreshold::AUTO);
    assert_eq!(job.drawing.images[0].trace, ImageTrace::CentreLine);
    let mut unknown = data.clone();
    unknown["drawing"]["images"][0]["trace"] = json!("skeleton");
    assert!(parse_job(unknown, "p").is_err());
    let mut bad = data.clone();
    bad["drawing"]["images"][0]["threshold"] = json!(255);
    bad["drawing"]["images"][0]["smoothing"] = json!(0.0);
    let text = parse_job(bad, "p").unwrap_err().to_string();
    assert!(text.contains("drawing.images.0.threshold"), "{text}");
    assert!(text.contains("drawing.images.0.smoothing"), "{text}");
    let mut orphan = data.clone();
    let next = orphan["drawing"]["next_shape_id"].as_u64().unwrap();
    orphan["drawing"]["shapes"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "id": next, "layer": "logo", "kind": "path", "dxf_handle": null, "dxf_type": null,
            "image": 7, "closed": true, "segments": [
                {"line": {"start": [0.0, 0.0], "end": [1.0, 0.0]}},
                {"line": {"start": [1.0, 0.0], "end": [0.0, 1.0]}},
                {"line": {"start": [0.0, 1.0], "end": [0.0, 0.0]}}
            ]
        }));
    orphan["drawing"]["next_shape_id"] = json!(next + 1);
    let text = parse_job(orphan, "p").unwrap_err().to_string();
    assert!(text.contains("no image 7"), "{text}");
    let mut level = data;
    level["drawing"]["images"][0]["threshold"] = json!(140);
    let job = parse_job(level, "p").unwrap();
    assert_eq!(job.drawing.images[0].threshold, ImageThreshold::Level(140));
}

// -- shapes left out of a whole DXF or image (docs/specs/leave-out-shapes.md §2)

#[test]
fn shapes_can_be_left_out_of_a_whole_source_only() {
    let mut data = project(json!([square_shape(1)]), 2, json!({"shapes": [1]}));
    data["operations"][0]["select"] = json!({"layer": "L", "exclude": [1]});
    let err = parse_job(data, "p").unwrap_err().to_string();
    assert!(
        err.contains("only be left out of a whole DXF, image or text"),
        "{err}"
    );
}

// -- inches (docs/specs/inch-units.md §4.1)

#[test]
fn v27_to_v28_says_the_gcode_is_in_mm_and_changes_nothing_else() {
    // Migrated to the current version (29 and 30 change only the version,
    // docs/specs/surfacing.md, drawing-input.md §5).
    let v27 = raw("v27/text.chipload");
    assert!(v27["machine"].get("gcode_units").is_none());
    let mut current = migrate(v27.clone()).unwrap();
    assert_eq!(current["schema_version"], json!(30));
    let job = parse_job(current.clone(), "p").unwrap();
    assert_eq!(job.machine.gcode_units, chipload_job::Units::Mm);
    without_gcode_units(&mut current);
    assert_eq!(current, v27);
}

#[test]
fn a_machine_without_gcode_units_is_refused_at_28() {
    let mut data = migrate(raw("v27/text.chipload")).unwrap();
    data["machine"]
        .as_object_mut()
        .unwrap()
        .remove("gcode_units");
    assert!(parse_job(data, "p").is_err());
}

// -- surfacing (docs/specs/surfacing.md §9)

#[test]
fn v28_to_v29_changes_only_the_version() {
    let v28 = raw("v28/rest_passes.chipload");
    let mut current = migrate(v28.clone()).unwrap();
    assert_eq!(current["schema_version"], json!(30));
    current["schema_version"] = json!(28);
    assert_eq!(current, v28);
}

#[test]
fn v29_to_v30_changes_only_the_version() {
    for sample in ["v29/surface.chipload", "v29/script_text.chipload"] {
        let v29 = raw(sample);
        let mut current = migrate(v29.clone()).unwrap();
        assert_eq!(current["schema_version"], json!(30));
        current["schema_version"] = json!(29);
        assert_eq!(current, v29, "{sample}");
    }
}

/// A polygon (docs/specs/drawing-input.md §5): saved with its values, its segments
/// counter-clockwise from the first corner, and refused when its values can't be.
#[test]
fn a_polygon_is_saved_loaded_and_checked() {
    use chipload_job::{Geometry, Shape};
    let mut data = raw("v29/surface.chipload");
    data["schema_version"] = json!(30);
    data["drawing"]["shapes"] = json!([{"id": 1, "layer": "0", "kind": "polygon",
        "center": [50.0, 30.0], "radius": 20.0, "sides": 6, "angle": 0.0}]);
    data["drawing"]["next_shape_id"] = json!(2);
    let job = parse_job(data.clone(), "p").unwrap();
    let shape: &Shape = &job.drawing.shapes[0];
    assert_eq!(
        shape.geometry,
        Geometry::Polygon {
            center: [50.0, 30.0],
            radius: 20.0,
            sides: 6,
            angle: 0.0
        }
    );
    let segments = shape.segments();
    assert_eq!(segments.len(), 6);
    let chipload_job::PathSegment::Line(first) = &segments[0] else {
        panic!("straight sides")
    };
    assert_eq!(first.start, [70.0, 30.0]);
    // Counter-clockwise: the second corner is up and to the left of the first.
    assert!(first.end[1] > 30.0 && first.end[0] < 70.0);
    // Saved as loaded.
    let saved = serde_json::to_value(&job).unwrap();
    assert_eq!(saved["drawing"]["shapes"][0], data["drawing"]["shapes"][0]);
    // Two sides, a radius of 0, a field of another kind: refused.
    for (field, value) in [
        ("sides", json!(2)),
        ("radius", json!(0.0)),
        ("length", json!(5.0)),
    ] {
        let mut bad = data.clone();
        bad["drawing"]["shapes"][0][field] = value;
        assert!(parse_job(bad, "p").is_err(), "{field}");
    }
}

#[test]
fn a_surface_is_saved_and_loaded_without_a_selector() {
    let job = load_job(&corpus().join("v29/surface.chipload")).unwrap();
    let chipload_job::Operation::Surface(surface) = &job.operations[0] else {
        panic!("the sample starts with its surface");
    };
    assert_eq!((surface.tool, surface.end_depth), (3, 1.0));
    assert!(job.operations[0].select().is_empty());
    let text = serde_json::to_value(&job).unwrap();
    assert!(text["operations"][0].get("select").is_none());
    // A selector isn't one of its fields: refused, as any unknown field.
    let mut data = raw("v29/surface.chipload");
    data["operations"][0]["select"] = json!({"layer": "L"});
    let err = parse_job(data, "p").unwrap_err().to_string();
    assert!(err.contains("select"), "{err}");
}

/// A drawn shape's name is kept, and left out of the file when there is none; an empty
/// one, or one on an imported shape, is refused (docs/specs/drawn-shapes-box.md §4).
#[test]
fn a_drawn_shapes_name_round_trips() {
    let dir = temp_dir("shape-name");
    let mut job = load_job(&corpus().join("v5/drawn_part.chipload")).unwrap();
    job.drawing.shapes[2].name = Some("Lid".into());
    let path = dir.join("named.chipload");
    chipload_job::save_job(&job, &path).unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    assert_eq!(text.matches("\"name\": \"Lid\"").count(), 1, "{text}");
    let back = load_job(&path).unwrap();
    assert_eq!(back.drawing.shapes[2].name.as_deref(), Some("Lid"));
    assert_eq!(back.drawing.shapes[0].name, None);

    job.drawing.shapes[2].name = Some("  ".into());
    let problems = job.drawing.validate();
    assert!(
        problems.iter().any(|p| p.path == "drawing.shapes.2.name"),
        "{problems:?}"
    );

    let mut raw = raw("v5/drawn_part.chipload");
    raw["drawing"]["shapes"][0]["name"] = "box".into();
    let mut imported = raw.clone();
    imported["drawing"]["shapes"][0]["kind"] = "path".into();
    let path = dir.join("raw.chipload");
    std::fs::write(&path, raw.to_string()).unwrap();
    assert!(load_job(&path).is_ok());
    std::fs::write(&path, imported.to_string()).unwrap();
    assert!(load_job(&path).is_err());
}
