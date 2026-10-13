// SPDX-License-Identifier: GPL-3.0-or-later
//! Job-level rules of plan_job.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use chipload_core::chain::Contour;
use chipload_job::migrations::migrate;
use chipload_job::{Job, parse_job};
use chipload_toolpath::{ToolpathError, plan_job};
use serde_json::{Value, json};

fn job(operations: Value) -> Job {
    parse_job(
        migrate(json!({
            "schema_version": 21, // written as v21 JSON: migrated into one part
            // plan_job gets its contours directly; the drawing isn't used here.
            "drawing": {"shapes": [], "next_shape_id": 1, "dxfs": []},
            "stock": {"x": 300, "y": 200, "thickness": 18, "origin": "bottom_left",
                      "max_overcut": 0.5},
            "machine": {"post": "grbl", "safe_z": 5, "spindle": "cw", "tool_change": "files"},
            "tools": [{"id": 1, "diameter": 6, "rpm": 18000}],
            "operations": operations,
        }))
        .unwrap(),
        "test.json",
    )
    .unwrap()
}

fn profile(side: &str, tabs: Value) -> Value {
    json!([{"type": "profile", "select": {"layer": "L"}, "side": side, "direction": "climb",
            "tool": 1, "start_depth": 0, "end_depth": 12, "clearance": 0, "step_down": 6, "feed": 1500, "plunge_feed": 500,
            "tabs": tabs}])
}

fn part() -> Vec<Contour> {
    vec![common::polygon(&[
        (10.0, 10.0),
        (110.0, 10.0),
        (110.0, 70.0),
        (10.0, 70.0),
    ])]
}

#[test]
fn tabs_lift_on_the_last_pass_and_are_reported_for_display() {
    let tabs =
        json!({"count": 4, "width": 8, "height": 3, "flank_angle": std::f64::consts::FRAC_PI_2});
    let plan = plan_job(&job(profile("outside", tabs)), &part()).unwrap();
    assert_eq!(plan.tabs.len(), 4);
    let lifts = plan
        .program
        .moves
        .iter()
        .filter(|m| m.z == Some(-9.0))
        .count();
    assert_eq!(lifts, 4);
    // Only the reminder that a straight plunge needs a centre-cutting tool.
    assert_eq!(plan.notes.len(), 1, "{:?}", plan.notes);
    assert!(plan.notes[0].contains("plunges straight down"));
}

#[test]
fn tab_as_high_as_the_cut_is_an_error() {
    let tabs =
        json!({"count": 4, "width": 8, "height": 12, "flank_angle": std::f64::consts::FRAC_PI_2});
    let err = plan_job(&job(profile("outside", tabs)), &part()).unwrap_err();
    assert!(
        matches!(&err, ToolpathError::Job(m) if m.contains("tab height 12")),
        "{err}"
    );
}

#[test]
fn tabs_on_an_inside_profile_are_allowed_with_a_warning() {
    let tabs =
        json!({"count": 2, "width": 8, "height": 3, "flank_angle": std::f64::consts::FRAC_PI_2});
    let plan = plan_job(&job(profile("inside", tabs)), &part()).unwrap();
    assert!(
        plan.notes
            .iter()
            .any(|n| n.contains("hold the cut-out slug"))
    );
}

#[test]
fn too_many_tabs_names_the_contour() {
    let tabs =
        json!({"count": 40, "width": 8, "height": 3, "flank_angle": std::f64::consts::FRAC_PI_2});
    let err = plan_job(&job(profile("outside", tabs)), &part()).unwrap_err();
    assert!(
        matches!(&err, ToolpathError::Geometry(m) if m.starts_with("Operation 1, contour #1:")),
        "{err}"
    );
}

#[test]
fn each_operation_owns_a_run_of_moves_in_cut_order() {
    // Three operations, the middle one disabled: the spans keep the list's numbering.
    let mut ops = profile("inside", Value::Null);
    let mut disabled = ops[0].clone();
    disabled["enabled"] = json!(false);
    let mut outside = ops[0].clone();
    outside["side"] = json!("outside");
    ops = json!([ops[0], disabled, outside]);
    let plan = plan_job(&job(ops), &part()).unwrap();
    let ops: Vec<usize> = plan.op_spans.iter().map(|s| s.operation).collect();
    assert_eq!(ops, [0, 2]);
    assert_eq!(plan.op_spans[0].moves.start, 0);
    assert_eq!(plan.op_spans[0].moves.end, plan.op_spans[1].moves.start);
    assert_eq!(plan.op_spans[1].moves.end, plan.program.moves.len());
}

// -- depths and clearance (docs/specs/depths-and-clearance.md)

fn profile_with(side: &str, changes: Value) -> Value {
    let mut ops = profile(side, Value::Null);
    for (k, v) in changes.as_object().unwrap() {
        ops[0][k] = v.clone();
    }
    ops
}

/// (min x, min y, max x, max y) of the XY positions the program visits.
fn xy_extent(plan: &chipload_toolpath::Plan) -> (f64, f64, f64, f64) {
    let (mut x, mut y) = (0.0, 0.0);
    let mut out = (
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    );
    for m in &plan.program.moves {
        x = m.x.unwrap_or(x);
        y = m.y.unwrap_or(y);
        if m.x.is_some() || m.y.is_some() {
            out = (out.0.min(x), out.1.min(y), out.2.max(x), out.3.max(y));
        }
    }
    out
}

#[test]
fn clearance_moves_a_profile_into_the_part() {
    // The part is 10..110 x 10..70; tool radius 3.
    let outside = |c: f64| {
        xy_extent(
            &plan_job(
                &job(profile_with("outside", json!({"clearance": c}))),
                &part(),
            )
            .unwrap(),
        )
    };
    assert_eq!(outside(0.0), (7.0, 7.0, 113.0, 73.0));
    assert_eq!(outside(0.5), (7.5, 7.5, 112.5, 72.5)); // smaller part
    assert_eq!(outside(-0.5), (6.5, 6.5, 113.5, 73.5)); // stock left on it
    let inside = |c: f64| {
        xy_extent(
            &plan_job(
                &job(profile_with("inside", json!({"clearance": c}))),
                &part(),
            )
            .unwrap(),
        )
    };
    assert_eq!(inside(0.5), (12.5, 12.5, 107.5, 67.5)); // larger hole
    let err = plan_job(
        &job(profile_with("outside", json!({"clearance": 3}))),
        &part(),
    )
    .unwrap_err();
    assert!(
        err.to_string().contains("less than the tool radius 3"),
        "{err}"
    );
}

#[test]
fn passes_run_between_start_and_end_depth() {
    let ops = profile_with(
        "outside",
        json!({"start_depth": 4, "end_depth": 12, "step_down": 4}),
    );
    let plan = plan_job(&job(ops), &part()).unwrap();
    let mut cut_z: Vec<f64> = plan
        .program
        .moves
        .iter()
        .filter_map(|m| m.z)
        .filter(|&z| z < 0.0)
        .collect();
    cut_z.dedup();
    assert_eq!(cut_z, [-8.0, -12.0]);
}

#[test]
fn tabs_must_be_lower_than_the_cut_between_the_depths() {
    let tabs =
        json!({"count": 4, "width": 8, "height": 3, "flank_angle": std::f64::consts::FRAC_PI_2});
    let cut = |start: f64| {
        plan_job(
            &job(profile_with(
                "outside",
                json!({"start_depth": start, "tabs": tabs.clone()}),
            )),
            &part(),
        )
    };
    assert!(cut(8.0).is_ok()); // 4 mm cut, 3 mm tabs
    let err = cut(9.0).unwrap_err(); // 3 mm cut
    assert!(
        err.to_string()
            .contains("tab height 3 must be less than the cut depth 3"),
        "{err}"
    );
}

#[test]
fn cuts_below_the_stock_warn_within_the_overcut_and_fail_beyond() {
    // Stock 18 mm thick, max_overcut 0.5.
    let plan = plan_job(
        &job(profile_with("outside", json!({"end_depth": 18.3}))),
        &part(),
    )
    .unwrap();
    assert!(
        plan.notes
            .iter()
            .any(|n| n.contains("cuts 0.300 mm below the stock")),
        "{:?}",
        plan.notes
    );
    let err = plan_job(
        &job(profile_with("outside", json!({"end_depth": 18.6}))),
        &part(),
    )
    .unwrap_err();
    assert!(
        err.to_string().contains("more than the allowed 0.5 mm"),
        "{err}"
    );
    let flush = plan_job(
        &job(profile_with("outside", json!({"end_depth": 18}))),
        &part(),
    )
    .unwrap();
    assert!(!flush.notes.iter().any(|n| n.contains("below the stock")));
}

// -- ramp entry (docs/specs/ramp-entry.md)

const THREE_DEG: f64 = 3.0 * std::f64::consts::PI / 180.0;

fn not_centre_cutting(operations: Value) -> Job {
    let mut job = job(operations);
    job.tools[0].center_cutting = Some(false);
    job
}

/// Straight descents only reach depths already cut (or the start depth); every move that
/// goes down sideways is no steeper than the ramp angle. Returns how many ramp moves.
fn assert_ramps_only(moves: &[chipload_toolpath::model::Move], top: f64, angle: f64) -> usize {
    use chipload_toolpath::model::MoveKind;
    let (mut x, mut y, mut z) = (0.0, 0.0, f64::INFINITY);
    let mut deepest = top;
    let mut ramps = 0;
    for m in moves {
        let (nx, ny, nz) = (m.x.unwrap_or(x), m.y.unwrap_or(y), m.z.unwrap_or(z));
        let run = (nx - x).hypot(ny - y);
        if m.kind != MoveKind::Rapid && nz < z - 1e-9 {
            if run == 0.0 {
                assert!(
                    nz >= deepest - 1e-9,
                    "straight plunge to {nz} below {deepest}"
                );
            } else {
                assert!((z - nz) / run <= angle.tan() + 1e-9, "ramp too steep");
                ramps += 1;
            }
        }
        deepest = deepest.min(nz);
        (x, y, z) = (nx, ny, nz);
    }
    ramps
}

#[test]
fn a_profile_with_a_tool_that_cant_plunge_is_refused_unless_it_ramps() {
    let plunge = not_centre_cutting(profile("outside", Value::Null));
    let err = plan_job(&plunge, &part()).unwrap_err().to_string();
    assert!(
        err.contains("not centre-cutting") && err.contains("ramp"),
        "{err}"
    );

    // Ramped in, with tabs whose flanks are no steeper than the ramp.
    let mut ops = profile(
        "outside",
        json!({"count": 4, "width": 8, "height": 3, "flank_angle": THREE_DEG}),
    );
    ops[0]["entry"] = json!({"type": "ramp", "ramp_angle": THREE_DEG});
    let plan = plan_job(&not_centre_cutting(ops.clone()), &part()).unwrap();
    assert!(
        plan.notes.iter().all(|n| !n.contains("plunge")),
        "{:?}",
        plan.notes
    );
    // A spiral down (one lap per pass here), the level lap and the flanks: every descent
    // is within the ramp angle, and the tool still goes over the four tab tops.
    let ramps = assert_ramps_only(&plan.program.moves, 0.0, THREE_DEG);
    assert!(ramps >= 2 * 4, "{ramps} ramp moves");
    assert!(plan.program.moves.iter().any(|m| m.z == Some(-9.0)));

    // Straight (90°) tabs would plunge after each tab: refused for this tool, a note for a
    // tool that isn't marked.
    ops[0]["tabs"]["flank_angle"] = json!(std::f64::consts::FRAC_PI_2);
    let err = plan_job(&not_centre_cutting(ops.clone()), &part())
        .unwrap_err()
        .to_string();
    assert!(
        err.contains("not centre-cutting") && err.contains("flanks (90.0°)"),
        "{err}"
    );
    let plan = plan_job(&job(ops), &part()).unwrap();
    assert!(
        plan.notes
            .iter()
            .any(|n| n.contains("steeper than the ramp angle")),
        "{:?}",
        plan.notes
    );
}

#[test]
fn pockets_ramp_in_with_a_tool_that_cant_plunge() {
    let pocket_part = vec![common::polygon(&[
        (0.0, 0.0),
        (60.0, 0.0),
        (60.0, 40.0),
        (0.0, 40.0),
    ])];
    {
        let strategy = json!({"type": "rings"});
        let ops = json!([{"type": "pocket", "round_corners": false, "select": {"layer": "L"}, "tool": 1,
            "start_depth": 1, "end_depth": 7, "clearance": 0, "step_down": 3,
            "stepover": 0.4, "direction": "climb", "feed": 1500, "plunge_feed": 500,
            "strategy": strategy, "entry": {"type": "ramp", "ramp_angle": THREE_DEG}}]);
        let plan = plan_job(&not_centre_cutting(ops), &pocket_part)
            .unwrap_or_else(|e| panic!("{strategy}: {e}"));
        // Ramps start at the start depth (Z -1): only that is taken as cut.
        let ramps = assert_ramps_only(&plan.program.moves, -1.0, THREE_DEG);
        assert!(ramps > 0, "{strategy}");
    }
}

#[test]
fn a_ramp_steeper_than_30_degrees_is_refused() {
    let mut ops = profile("outside", Value::Null);
    ops[0]["entry"] = json!({"type": "ramp", "ramp_angle": 0.6});
    let err = parse_job(
        migrate(json!({
            "schema_version": 21, // written as v21 JSON: migrated into one part
            "drawing": {"shapes": [], "next_shape_id": 1, "dxfs": []},
            "stock": {"x": 300, "y": 200, "thickness": 18, "max_overcut": 0.5},
            "machine": {"post": "grbl", "safe_z": 5, "spindle": "cw", "tool_change": "files"},
            "tools": [{"id": 1, "diameter": 6, "rpm": 18000}],
            "operations": ops,
        }))
        .unwrap(),
        "test.json",
    )
    .unwrap_err()
    .to_string();
    assert!(
        err.contains("entry.ramp_angle: must be more than 0 and at most 30"),
        "{err}"
    );
}

// -- copies in a grid (docs/specs/copies-grid.md)

fn with_copies(ops: Value, copies: Value) -> Job {
    let mut job = job(ops);
    job.parts[0].copies = serde_json::from_value(copies).unwrap();
    job
}

/// The inside then the outside profile of the part.
fn two_profiles() -> Value {
    let inside = profile("inside", Value::Null)[0].clone();
    let outside = profile("outside", Value::Null)[0].clone();
    json!([inside, outside])
}

/// XY extent of the feed moves of the spans for `copy`.
fn extent_of(plan: &chipload_toolpath::Plan, copy: u32) -> ((f64, f64), (f64, f64)) {
    chipload_toolpath::plan::cut_extent(&plan.program.moves, &plan.op_spans, Some(copy)).unwrap()
}

#[test]
fn one_copy_is_the_part_as_before() {
    let plain = plan_job(&job(two_profiles()), &part()).unwrap();
    let grid = json!({"columns": 1, "rows": 1, "dx": 0, "dy": 0});
    let one = plan_job(&with_copies(two_profiles(), grid), &part()).unwrap();
    assert_eq!(one.program, plain.program);
    assert_eq!(one.notes, plain.notes);
}

#[test]
fn copies_are_cut_operation_by_operation_in_a_serpentine() {
    let grid = json!({"columns": 2, "rows": 2, "dx": 120, "dy": 80});
    let plan = plan_job(&with_copies(two_profiles(), grid), &part()).unwrap();
    let order: Vec<(usize, u32)> = plan
        .op_spans
        .iter()
        .map(|s| (s.operation, s.copy))
        .collect();
    assert_eq!(
        order,
        [
            (0, 1),
            (0, 2),
            (0, 4),
            (0, 3),
            (1, 1),
            (1, 2),
            (1, 4),
            (1, 3)
        ]
    );
    // Every copy cuts the original moved by its pitch.
    let ((x0, y0), (x1, y1)) = extent_of(&plan, 1);
    for (copy, (dx, dy)) in [(2, (120.0, 0.0)), (3, (0.0, 80.0)), (4, (120.0, 80.0))] {
        let ((a0, b0), (a1, b1)) = extent_of(&plan, copy);
        for (got, want) in [(a0, x0 + dx), (b0, y0 + dy), (a1, x1 + dx), (b1, y1 + dy)] {
            assert!((got - want).abs() < 1e-9, "copy {copy}: {got} vs {want}");
        }
    }
    // The straight-plunge reminder once per operation, not per copy.
    let reminders = plan
        .notes
        .iter()
        .filter(|n| n.contains("plunges straight down"))
        .count();
    assert_eq!(reminders, 2, "{:?}", plan.notes);
}

#[test]
fn copies_can_be_cut_part_by_part() {
    let grid = json!({"columns": 3, "rows": 1, "dx": -120, "dy": 0, "order": "by_copy"});
    let plan = plan_job(&with_copies(two_profiles(), grid), &part()).unwrap();
    let order: Vec<(usize, u32)> = plan
        .op_spans
        .iter()
        .map(|s| (s.operation, s.copy))
        .collect();
    assert_eq!(order, [(0, 1), (1, 1), (0, 2), (1, 2), (0, 3), (1, 3)]);
    // A negative pitch grows to the left.
    let ((x0, _), _) = extent_of(&plan, 3);
    assert!(x0 < -200.0, "{x0}");
}

#[test]
fn overlapping_copies_are_refused_at_the_boundary() {
    // The outside profile of the 100 x 60 part with a 6 mm tool cuts 112 x 72 mm.
    let ok = |columns, rows, dx, dy| {
        let grid = json!({"columns": columns, "rows": rows, "dx": dx, "dy": dy});
        plan_job(&with_copies(two_profiles(), grid), &part())
    };
    assert!(ok(2, 1, 112.0, 0.0).is_ok());
    let err = ok(2, 1, 111.9, 0.0).unwrap_err();
    assert!(
        err.to_string()
            .contains("Copies overlap: with columns the pitch dx must be at least 112.000 mm"),
        "{err}"
    );
    assert!(ok(1, 2, 0.0, 72.0).is_ok());
    let err = ok(1, 2, 0.0, -71.0).unwrap_err();
    assert!(
        err.to_string()
            .contains("pitch dy must be at least 72.000 mm"),
        "{err}"
    );
    // One column: dx doesn't matter.
    assert!(ok(1, 3, 5.0, 100.0).is_ok());
}

#[test]
fn tabs_whose_feet_meet_get_a_note() {
    // 3° flanks on 3 mm tabs: 57 mm feet each side, more than the gaps between four tabs
    // on this 368 mm ring (docs/specs/profile-spiral.md §4).
    let mut ops = profile(
        "outside",
        json!({"count": 4, "width": 8, "height": 3, "flank_angle": THREE_DEG}),
    );
    ops[0]["entry"] = json!({"type": "ramp", "ramp_angle": THREE_DEG});
    let plan = plan_job(&job(ops.clone()), &part()).unwrap();
    assert!(
        plan.notes
            .iter()
            .any(|n| n.contains("tabs 1 and 2, 2 and 3, 3 and 4, 4 and 1 meet below their top")),
        "{:?}",
        plan.notes
    );
    // At 30° the feet are 5.2 mm: no note.
    ops[0]["tabs"]["flank_angle"] = json!(std::f64::consts::PI / 6.0);
    let plan = plan_job(&job(ops), &part()).unwrap();
    assert!(
        plan.notes.iter().all(|n| !n.contains("meet")),
        "{:?}",
        plan.notes
    );
}

#[test]
fn removed_pocket_strategies_are_refused_until_switched_to_rings() {
    // docs/decisions/0017-scope.md: the project still loads; generating is refused, with
    // the strategy field named, until the pocket is switched to rings.
    let part = vec![common::polygon(&[
        (0.0, 0.0),
        (60.0, 0.0),
        (60.0, 40.0),
        (0.0, 40.0),
    ])];
    for (strategy, name) in [
        (
            json!({"type": "trochoidal", "loop_radius": 1.5, "step": 0.6}),
            "trochoidal",
        ),
        (json!({"type": "adaptive"}), "adaptive"),
    ] {
        let ops = json!([{"type": "pocket", "round_corners": false, "select": {"layer": "L"},
            "tool": 1, "start_depth": 0, "end_depth": 3, "clearance": 0, "step_down": 3,
            "stepover": 0.4, "direction": "climb", "feed": 1500, "plunge_feed": 500,
            "strategy": strategy}]);
        let mut job = job(ops);
        let err = plan_job(&job, &part).unwrap_err().to_string();
        assert!(
            err.contains(&format!(
                "Operation 1: {name} clearing was removed in Chipload 0.14; switch the pocket \
                 to rings"
            )),
            "{err}"
        );
        let problems = chipload_toolpath::job_problems(&job);
        assert!(problems.iter().any(|p| p.path == "operations.0.strategy"));
        let chipload_job::Operation::Pocket(pocket) = &mut job.operations[0] else {
            panic!("a pocket");
        };
        pocket.strategy = chipload_job::PocketStrategy::Rings {};
        assert!(plan_job(&job, &part).is_ok());
    }
}

// -- parts (docs/specs/parts.md)

fn on_layer(mut contour: Contour, layer: &str) -> Contour {
    for part in &mut contour.parts {
        part.segment.layer = layer.into();
    }
    contour
}

/// A square on layer A cut by part 1 and one on layer B cut by part 2 ("B"), both with
/// tool 1; part 2 with `copies`. Each square is 40 mm, cut outside with a 6 mm tool: 46 mm.
fn two_parts(b_at: f64, a_copies: Value, b_copies: Value) -> (Job, Vec<Contour>) {
    let mut ops = profile("outside", Value::Null);
    ops[0]["select"] = json!({"layer": "A"});
    let mut b = ops[0].clone();
    b["select"] = json!({"layer": "B"});
    ops.as_array_mut().unwrap().push(b);
    let mut job = job(ops);
    job.parts[0].copies = serde_json::from_value(a_copies).unwrap();
    job.parts.push(chipload_job::Part {
        id: 2,
        name: "B".into(),
        enabled: true,
        copies: serde_json::from_value(b_copies).unwrap(),
    });
    job.operations[1].set_part(2);
    assert!(job.validate().is_empty(), "{:?}", job.validate());
    let contours = vec![
        on_layer(common::rect(1, 0.0, 0.0, 40.0, 40.0), "A"),
        on_layer(common::rect(2, b_at, 0.0, b_at + 40.0, 40.0), "B"),
    ];
    (job, contours)
}

#[test]
fn parts_are_cut_one_after_the_other_each_with_its_copies() {
    let grid = |dx: f64| json!({"columns": 2, "rows": 1, "dx": dx, "dy": 0});
    let (job, contours) = two_parts(100.0, grid(-60.0), grid(60.0));
    let plan = plan_job(&job, &contours).unwrap();
    let order: Vec<(usize, u32)> = plan
        .op_spans
        .iter()
        .map(|s| (s.operation, s.copy))
        .collect();
    // Part 1 with both its copies, then part 2 with both of its: one tool, one group.
    assert_eq!(order, [(0, 1), (0, 2), (1, 1), (1, 2)]);
    assert_eq!(plan.groups.len(), 1);
    // Each part's copies are its own: part 1 grows left, part 2 right.
    let ((x0, _), _) = extent_of(&plan, 2);
    assert!(x0 < -50.0, "{x0}");
    // A comment where each part starts.
    assert_eq!(
        plan.program.comments,
        ["Part 1: Part 1, 2 copies", "Part 2: B, 2 copies"]
    );
    let marked: Vec<usize> = plan
        .program
        .moves
        .iter()
        .enumerate()
        .filter(|(_, m)| m.comment.is_some())
        .map(|(i, _)| i)
        .collect();
    assert_eq!(
        marked,
        [plan.op_spans[0].moves.start, plan.op_spans[2].moves.start]
    );
}

#[test]
fn a_part_that_is_off_is_left_out_and_one_part_has_no_comment() {
    let (mut job, contours) = two_parts(100.0, Value::Null, Value::Null);
    job.parts[1].enabled = false;
    let plan = plan_job(&job, &contours).unwrap();
    let operations: Vec<usize> = plan.op_spans.iter().map(|s| s.operation).collect();
    assert_eq!(operations, [0]);
    // Two parts, one off: still a comment for the one cut.
    assert_eq!(plan.program.comments, ["Part 1: Part 1"]);
    // One part: no comments at all, as before parts.
    let plain = plan_job(&job_of_one_part(), &part()).unwrap();
    assert!(plain.program.comments.is_empty());
}

fn job_of_one_part() -> Job {
    job(profile("outside", Value::Null))
}

#[test]
fn a_copy_may_not_cut_into_another_part_but_the_parts_as_drawn_may_overlap() {
    // The area is the tool centre's path widened by the tool radius: part 1 cuts -6..46;
    // part 2, drawn at 200, cuts 194..246, and its copy 2 194 + dx.
    let refused = |dx: f64| {
        let grid = json!({"columns": 2, "rows": 1, "dx": dx, "dy": 0});
        let (job, contours) = two_parts(200.0, Value::Null, grid);
        plan_job(&job, &contours).err().map(|e| e.to_string())
    };
    // Touching: copy 2 cuts 46..98.
    assert_eq!(refused(-148.0), None);
    let err = refused(-148.1).unwrap();
    assert!(
        err.contains("Part \"B\" copy 2 overlaps part \"Part 1\" copy 1"),
        "{err}"
    );
    // Drawn on top of each other, without copies: allowed (an engraving over a cut-out).
    let (job, contours) = two_parts(20.0, Value::Null, Value::Null);
    assert!(plan_job(&job, &contours).is_ok());
}
