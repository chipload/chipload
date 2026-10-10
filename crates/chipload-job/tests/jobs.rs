// SPDX-License-Identifier: GPL-3.0-or-later
//! Job (project) loading, migration and saving against the corpus.

// Integration tests may unwrap and panic: a failure is the test result.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};

use chipload_job::migrations::migrate;
use chipload_job::{CURRENT_VERSION, Direction, JobError, Operation, load_job, parse_job};
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
    assert!(pocket.keep_tool_down);
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
    data["drawing"] = json!({"shapes": shapes, "next_shape_id": next, "source": null});
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
         "width": 40.0, "angle": 0.5235987755982988},
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
    assert_eq!(data["drawing"]["shapes"], drawn_shapes());
    let again = parse_job(migrate(data).unwrap(), "p").unwrap();
    assert_eq!(again.drawing, job.drawing);
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
    refused(
        json!({"id": 1, "layer": "0", "kind": "rectangle", "center": [0.0, 0.0],
               "length": 10.0, "width": 0.0, "angle": 0.0}),
        "width: must be a positive number",
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

// -- pocket strategy (docs/specs/trochoidal.md)

#[test]
fn pocket_strategy_defaults_to_rings_and_trochoidal_values_are_checked() {
    // Operation 0 of the lower part is a pocket without a strategy: rings, written explicitly.
    let job = parse_job(lower_job(), "p").unwrap();
    let Operation::Pocket(pocket) = &job.operations[0] else {
        panic!("expected a pocket");
    };
    assert_eq!(pocket.strategy, chipload_job::PocketStrategy::Rings {});
    let saved: Value = serde_json::from_str(&chipload_job::dump_job(&job).unwrap()).unwrap();
    assert_eq!(saved["operations"][0]["strategy"], json!({"type": "rings"}));

    let with = |strategy: Value| {
        let mut data = lower_job();
        data["operations"][0]["strategy"] = strategy;
        parse_job(data, "p")
    };
    let adaptive = json!({"type": "adaptive", "load": 0.1});
    let job = with(adaptive.clone()).unwrap();
    let saved: Value = serde_json::from_str(&chipload_job::dump_job(&job).unwrap()).unwrap();
    assert_eq!(saved["operations"][0]["strategy"], adaptive);
    let trochoidal = json!({"type": "trochoidal", "loop_radius": 1.5, "step": 0.6});
    let job = with(trochoidal.clone()).unwrap();
    let saved: Value = serde_json::from_str(&chipload_job::dump_job(&job).unwrap()).unwrap();
    assert_eq!(saved["operations"][0]["strategy"], trochoidal);
    for (bad, why) in [
        (json!({"type": "trochoidal", "step": 0.6}), "loop_radius"),
        (
            json!({"type": "trochoidal", "loop_radius": 1.0, "step": 1.5}),
            "0 < step <= loop_radius",
        ),
        (
            json!({"type": "trochoidal", "loop_radius": 0.0, "step": 0.0}),
            "0 < step <= loop_radius",
        ),
        (json!({"type": "zigzag"}), "unknown variant"),
        (json!({"type": "adaptive", "load": 0.6}), "0 < load <= 0.5"),
        (json!({"type": "adaptive", "load": 0.0}), "0 < load <= 0.5"),
        (json!({"type": "adaptive"}), "missing field `load`"),
        (json!({"type": "rings", "step": 1}), "unknown field"),
    ] {
        let err = with(bad.clone()).unwrap_err().to_string();
        assert!(err.contains(why), "{bad}: {err}");
    }
}
