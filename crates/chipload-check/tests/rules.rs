// SPDX-License-Identifier: GPL-3.0-or-later
//! One case per rule, with the exact messages users see.

// Integration tests may unwrap and panic: a failure is the test result.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use chipload_check::{CheckLimits, check_gcode};

fn limits() -> CheckLimits {
    CheckLimits {
        safe_z: 5.0,
        stock_top_z: 0.0,
        stock_bottom_z: -18.0,
        max_overcut: 0.0,
        stock_min: (0.0, 0.0),
        stock_max: (600.0, 400.0),
        tool_radius: 3.0,
        tool_radii: std::collections::BTreeMap::from([(1, 3.0), (2, 1.5)]),
        travel: Some((800.0, 600.0, 100.0)),
    }
}

const HEADER: &str = "G21 G90 G17 G94\nG0 Z5\nM3 S18000\nG4 P3\n";
const FOOTER: &str = "G0 Z5\nM5\nM30\n";
const GOOD_BODY: &str = "G0 X10 Y10\nG1 Z-6 F600\nG1 X50 F1800\nG1 Y50\nG1 X10\nG1 Y10\n";

fn good() -> String {
    format!("{HEADER}{GOOD_BODY}{FOOTER}")
}

fn errors(program: &str) -> Vec<String> {
    check_gcode(program, &limits())
        .errors
        .into_iter()
        .map(|e| e.message)
        .collect()
}

#[test]
fn good_program_passes() {
    let report = check_gcode(&good(), &limits());
    assert!(report.ok(), "{:?}", report.errors);
    assert!(report.warnings.is_empty());
}

#[test]
fn comments_are_ignored() {
    let program = format!(
        "(T1 D=6.000)\n{}",
        good().replace("G1 Y50", "G1 Y50 ; side 2")
    );
    assert!(errors(&program).is_empty());
}

#[test]
fn bad_programs_are_flagged() {
    let h = HEADER;
    let cases: Vec<(String, &str)> = vec![
        (
            "G21 G90\nG0 Z5\nG0 X10 Y10\nG1 Z-1 F100\n".into(),
            "spindle off",
        ),
        (
            format!("{}{GOOD_BODY}", h.replace("G21 ", "")),
            "before G21",
        ),
        (format!("{}{GOOD_BODY}", h.replace("G21", "G20")), "G20"),
        (
            format!("{}{GOOD_BODY}", h.replace(" G90", "")),
            "before G90",
        ),
        (format!("{h}G91\n"), "G91"),
        (format!("{h}G0 X10 Y10\nG1 Z-1\n"), "no feed"),
        (format!("{h}G0 X10 Y10\nG1 Z-1 F0\n"), "must be positive"),
        (
            format!("{h}G0 X10 Y10\nG1 Z-1 F100\nG0 X20\n"),
            "retract first",
        ),
        ("G21 G90\nG0 X10 Y10\n".into(), "retract first"),
        (format!("{h}G0 X10 Y10\nG0 Z-1\n"), "rapid into material"),
        (
            format!("{h}G0 X10 Y10\nG1 Z-18.5 F100\n"),
            "below the stock bottom",
        ),
        (
            format!("{h}G0 X10 Y10\nG1 Z-1 F100\nG2 X20 Y10 I4 J0\n"),
            "radius mismatch",
        ),
        (
            format!("{h}G0 X10 Y10\nG1 Z-1 F100\nG2 X20 Y10\n"),
            "without I/J",
        ),
        (format!("{h}G28\n"), "G28 is not supported"),
        (format!("{h}M8\n"), "M8 is not supported"),
        (format!("{h}G0 X10 Y10 X20\n"), "twice"),
        (format!("{h}G0 G1 X10\n"), "more than one motion"),
        (format!("{h}G0 X10 Y10 (unclosed\n"), "comment"),
        (format!("{h}G0 X10 Y10 @\n"), "Cannot parse"),
        (format!("{h}H1\n"), "Unsupported word H"),
        (format!("{}G0 Z10\n", good()), "after program end"),
        (
            format!("{h}G0 X0 Y0\nG0 X900\n{FOOTER}"),
            "exceeds machine travel",
        ),
    ];
    for (program, expected) in cases {
        let found = errors(&program);
        assert!(
            found
                .iter()
                .any(|e| e.to_lowercase().contains(&expected.to_lowercase())),
            "expected {expected:?} in {found:?}"
        );
    }
}

#[test]
fn valid_arc_passes() {
    let body = "G0 X10 Y10\nG1 Z-1 F100\nG2 X20 Y10 I5 J0\nG3 X10 Y10 I-5 J0\n";
    assert!(errors(&format!("{HEADER}{body}{FOOTER}")).is_empty());
}

#[test]
fn rapid_down_to_just_above_stock_is_allowed() {
    assert!(errors(&format!("{HEADER}G0 X10 Y10\nG0 Z1\nG1 Z-1 F100\n{FOOTER}")).is_empty());
}

#[test]
fn cutting_outside_stock_is_a_warning() {
    let program = format!("{HEADER}G0 X-20 Y10\nG1 Z-1 F100\nG1 X-30\n{FOOTER}");
    let report = check_gcode(&program, &limits());
    assert!(report.ok());
    assert!(
        report
            .warnings
            .iter()
            .any(|w| w.message.contains("outside the stock"))
    );
}

#[test]
fn missing_travel_is_a_warning() {
    let mut l = limits();
    l.travel = None;
    let report = check_gcode(&good(), &l);
    assert!(report.ok());
    assert!(
        report
            .warnings
            .iter()
            .any(|w| w.message.contains("travel not set"))
    );
}

#[test]
fn missing_program_end_is_a_warning() {
    let report = check_gcode(&format!("{HEADER}{GOOD_BODY}"), &limits());
    assert!(report.warnings.iter().any(|w| w.message.contains("M30")));
}

#[test]
fn exact_messages() {
    let found = errors(&format!("{HEADER}G0 X10 Y10\nG1 Z-1 F0\n"));
    assert!(
        found.contains(&"Feed F0 must be positive".to_string()),
        "{found:?}"
    );
    let found = errors(&format!("{HEADER}G0 X10 Y10\nG1 Z-18.5 F100\n"));
    assert!(
        found.contains(
            &"Z-18.500 is below the stock bottom (-18.000) by more than the allowed 0.000 mm"
                .to_string()
        )
    );
}

#[test]
fn below_the_stock_is_a_warning_within_the_overcut_and_an_error_beyond() {
    let mut allowed = limits();
    allowed.max_overcut = 0.5;
    let program = |z: &str| format!("{HEADER}G0 X10 Y10\nG1 Z{z} F100\n{FOOTER}");
    let within = check_gcode(&program("-18.3"), &allowed);
    assert!(within.ok(), "{:?}", within.errors);
    assert!(
        within.warnings.iter().any(|w| w
            .message
            .contains("1 moves go below the stock bottom (deepest Z-18.300")),
        "{:?}",
        within.warnings
    );
    let beyond = check_gcode(&program("-18.6"), &allowed);
    assert!(
        beyond
            .errors
            .iter()
            .any(|e| e.message.contains("by more than the allowed 0.500 mm")),
        "{:?}",
        beyond.errors
    );
}

// -- arcs (docs/specs/arc-output.md)

#[test]
fn arcs_with_i_j_pass_and_bad_ones_are_refused() {
    // From (20, 10) a half circle around (30, 10) to (40, 10), clockwise (over the top).
    let ok = format!("{HEADER}G0 X20 Y10\nG1 Z-1 F100\nG2 X40 Y10 I10 J0\n{FOOTER}");
    let report = check_gcode(&ok, &limits());
    assert!(report.ok(), "{:?}", report.errors);
    for (line, why) in [
        ("G2 X40 Y10 R10", "Unsupported word R"), // only I/J
        ("G2 X40 Y10", "without I/J"),
        ("G2 X20 Y10 I10 J0", "full circle in one move"),
        ("G2 X41 Y10 I10 J0", "radius mismatch"),
    ] {
        let found = errors(&format!(
            "{HEADER}G0 X20 Y10\nG1 Z-1 F100\n{line}\n{FOOTER}"
        ));
        assert!(found.iter().any(|e| e.contains(why)), "{line}: {found:?}");
    }
}

#[test]
fn helical_arcs_pass_and_are_held_to_the_same_limits() {
    // Down 1 mm over a half circle, then on round the other half to its start, 1 mm lower.
    let ok = format!(
        "{HEADER}G0 X20 Y10\nG1 Z-1 F100\nG2 X40 Y10 Z-2 I10 J0\nG2 X20 Y10 Z-3 I-10 J0\n{FOOTER}"
    );
    let report = check_gcode(&ok, &limits());
    assert!(report.ok(), "{:?}", report.errors);
    for (line, why) in [
        // Its radius must agree as for a flat arc.
        ("G2 X41 Y10 Z-2 I10 J0", "radius mismatch"),
        // Its end may not go deeper than the stock allows.
        ("G2 X40 Y10 Z-18.5 I10 J0", "below the stock bottom"),
        // Nor may it be a full circle in one move.
        ("G2 X20 Y10 Z-2 I10 J0", "full circle in one move"),
    ] {
        let found = errors(&format!(
            "{HEADER}G0 X20 Y10\nG1 Z-1 F100\n{line}\n{FOOTER}"
        ));
        assert!(found.iter().any(|e| e.contains(why)), "{line}: {found:?}");
    }
}

#[test]
fn an_arc_bulging_past_the_stock_is_seen_by_its_extent() {
    // Both ends inside the 600 x 400 stock (Y 395), but the arc over the top reaches Y 405.
    let mut tight = limits();
    tight.tool_radius = 0.0;
    let bulge = format!("{HEADER}G0 X290 Y395\nG1 Z-1 F100\nG2 X310 Y395 I10 J0\n{FOOTER}");
    let report = check_gcode(&bulge, &tight);
    assert!(
        report
            .warnings
            .iter()
            .any(|w| w.message.contains("outside the stock")),
        "{:?}",
        report.warnings
    );
    // The same arc the other way (G3, under its chord) stays inside.
    let under = bulge.replace("G2 X310", "G3 X310");
    let report = check_gcode(&under, &tight);
    assert!(
        !report
            .warnings
            .iter()
            .any(|w| w.message.contains("outside the stock")),
        "{:?}",
        report.warnings
    );
}

#[test]
fn each_tool_has_its_own_stock_margin() {
    // docs/specs/tool-per-operation.md §6: tool 1 is 6 mm, tool 2 is 3 mm.
    let project = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/corpus/jobs/v11/two_tools.chipload");
    let job = chipload_job::load_job(&project).unwrap();
    let largest = chipload_check::limits_from_job(&job);
    let small = chipload_check::limits_for_tool(&job, 2).unwrap();
    assert_eq!(largest.tool_radius, 3.0);
    assert_eq!(chipload_check::limits_for_tool(&job, 1).unwrap(), largest);
    assert_eq!(
        small,
        CheckLimits {
            tool_radius: 1.5,
            ..largest.clone()
        }
    );
    assert_eq!(chipload_check::limits_for_tool(&job, 9), None);
    // A cut 2 mm left of the stock: within 6 mm tool's margin, outside the 3 mm tool's.
    let x = largest.stock_min.0 - 2.0;
    let y = largest.stock_min.1 + 10.0;
    let program = format!(
        "{HEADER}G0 X{x} Y{y}\nG1 Z-1 F100\nG1 Y{}\n{FOOTER}",
        y + 10.0
    );
    let outside = |limits: &CheckLimits| {
        check_gcode(&program, limits)
            .warnings
            .iter()
            .any(|w| w.message.contains("outside the stock"))
    };
    assert!(!outside(&largest));
    assert!(outside(&small));
}

// -- tool changes (docs/specs/tool-per-operation.md §6)

/// Tool 1 (radius 3) cuts, then T2 M6 (radius 1.5) and it cuts again.
fn with_change(change: &str) -> String {
    format!(
        "G21 G90 G17 G94\nG0 Z5\nT1 M6\nM3 S18000\nG4 P3\n{GOOD_BODY}G0 Z5\n{change}\
         G0 X20 Y20\nG1 Z-3 F600\nG1 X40 F1800\n{FOOTER}"
    )
}

const CHANGE: &str = "M5\nT2 M6\nM3 S20000\nG4 P3\n";

#[test]
fn a_tool_change_with_the_spindle_stopped_at_safe_height_passes() {
    let report = check_gcode(&with_change(CHANGE), &limits());
    assert!(report.ok(), "{:?}", report.errors);
    // T alone changes nothing.
    let report = check_gcode(&format!("T2\n{}", good()), &limits());
    assert!(report.ok(), "{:?}", report.errors);
}

#[test]
fn bad_tool_changes_are_refused() {
    for (change, message) in [
        // Spindle still on.
        ("T2 M6\nM3 S20000\n", "Tool change (M6) with the spindle on"),
        // A tool the project doesn't have.
        (
            "M5\nT7 M6\nM3 S20000\n",
            "Tool change (M6) to T7, not a tool of the project",
        ),
        // Down at the stock.
        (
            "G0 Z1\nM5\nT2 M6\nM3 S20000\n",
            "Tool change (M6) below safe height",
        ),
        // Started again without a speed, or not at all.
        (
            "M5\nT2 M6\nM3\n",
            "Spindle started after a tool change without a speed (S)",
        ),
        ("M5\nT2 M6\n", "Cutting move with the spindle off"),
        ("M5\nT2.5 M6\nM3 S1\n", "T2.5 must be a whole tool number"),
    ] {
        let errors = errors(&with_change(change));
        assert!(
            errors.iter().any(|e| e.contains(message)),
            "{change:?}: expected {message:?} in {errors:?}"
        );
    }
    // M6 before any T.
    let errors = errors(&format!("G21 G90 G17 G94\nG0 Z5\nM6\n{}", good()));
    assert!(
        errors
            .iter()
            .any(|e| e == "Tool change (M6) without a tool number (T)"),
        "{errors:?}"
    );
}

#[test]
fn after_a_tool_change_the_stock_margin_is_the_new_tools() {
    // A cut 2 mm left of the stock: inside tool 1's margin (3), outside tool 2's (1.5).
    let program = |change: &str| {
        format!("{HEADER}{GOOD_BODY}G0 Z5\n{change}G0 X-2 Y20\nG1 Z-1 F600\nG1 Y40\n{FOOTER}")
    };
    let outside = |text: &str| {
        let report = check_gcode(text, &limits());
        assert!(report.ok(), "{:?}", report.errors);
        report
            .warnings
            .iter()
            .any(|w| w.message.contains("outside the stock"))
    };
    assert!(!outside(&program("")));
    assert!(outside(&program(CHANGE)));
}
