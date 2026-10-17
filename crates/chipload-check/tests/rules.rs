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
        units: chipload_job::Units::Mm,
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
        (format!("{h}M10\n"), "M10 is not supported"),
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

// -- setup codes of LinuxCNC and Mach programs (docs/specs/user-posts.md §4)

#[test]
fn setup_codes_coolant_and_percent_lines_pass() {
    let program = format!(
        "%\n(LinuxCNC)\nG21 G90 G17 G94 G40 G49 G80 G54 G64 P0.01 G91.1\nG61\nM8\n{}\
         {GOOD_BODY}M9\n{FOOTER}%\n",
        &HEADER["G21 G90 G17 G94\n".len()..]
    );
    let report = check_gcode(&program, &limits());
    assert!(report.ok(), "{:?}", report.errors);
    assert!(report.warnings.is_empty(), "{:?}", report.warnings);
}

#[test]
fn codes_that_move_the_tool_off_the_checked_path_stay_refused() {
    let cases = [
        (
            "G64",
            "G64 without P lets the controller round corners without limit (use P at most 0.05)",
        ),
        (
            "G64 P0.1",
            "G64 P0.1 lets the controller leave the path by more than 0.05 mm",
        ),
        ("G4 G64 P0.01", "G64 and G4 on one line: P is ambiguous"),
        ("G90.1", "G90.1 (absolute arc centres) is not supported"),
        ("G55", "G55 is not supported"),
        ("G43", "G43 is not supported"),
        ("G41", "G41 is not supported"),
        ("G28", "G28 is not supported"),
        ("G53", "G53 is not supported"),
        ("M2", "M2 is not supported"),
    ];
    for (line, message) in cases {
        let program = format!("{HEADER}{line}\n{GOOD_BODY}{FOOTER}");
        assert_eq!(errors(&program), [message], "{line}");
    }
    // `%` only alone on a line.
    let program = format!("{HEADER}G0 X1 %\n{GOOD_BODY}{FOOTER}");
    assert_eq!(errors(&program), [r#"Cannot parse "%""#]);
}

// -- rapids back into a drilled hole (docs/specs/holes.md §6)

/// Drilled to Z-4 at (10, 10), then out to the safe height.
const PECKED: &str = "G0 X10 Y10\nG1 Z-4 F300\nG0 Z5\n";

#[test]
fn a_peck_may_rapid_back_into_its_hole() {
    let program = format!("{HEADER}{PECKED}G0 Z-3.5\nG1 Z-8 F300\nG0 Z5\nG0 Z-7.5\n{FOOTER}");
    assert!(errors(&program).is_empty(), "{:?}", errors(&program));
}

#[test]
fn only_back_into_the_hole_and_not_too_deep() {
    let cases = [
        // Deeper than 0.5 mm above the bottom.
        format!("{HEADER}{PECKED}G0 Z-3.6\n{FOOTER}"),
        // Another hole: the tool moved in XY.
        format!("{HEADER}{PECKED}G0 X30\nG0 Z-3.5\n{FOOTER}"),
        // Back to the same place after an XY move: forgotten.
        format!("{HEADER}{PECKED}G0 X30\nG0 X10\nG0 Z-3.5\n{FOOTER}"),
        // The spindle stopped and started again.
        format!("{HEADER}{PECKED}M5\nM3 S18000\nG0 Z-3.5\n{FOOTER}"),
        // A cut that went sideways is not a hole.
        format!("{HEADER}G0 X10 Y10\nG1 Z-4 F300\nG1 X12\nG1 X10\nG0 Z5\nG0 Z-3.5\n{FOOTER}"),
    ];
    for program in cases {
        let found = errors(&program);
        assert!(
            found.iter().any(|e| e.contains("rapid into material")),
            "{program}\n{found:?}"
        );
    }
}

#[test]
fn a_new_tool_forgets_the_hole() {
    let program = format!("{HEADER}{PECKED}M5\nT2 M6\nM3 S18000\nG0 Z-3.5\n{FOOTER}");
    let found = errors(&program);
    assert!(
        found.iter().any(|e| e.contains("rapid into material")),
        "{found:?}"
    );
}

// -- inches (docs/specs/inch-units.md §4.3)

fn inch_limits() -> CheckLimits {
    CheckLimits {
        units: chipload_job::Units::Inch,
        ..limits()
    }
}

/// `good()` written in inches, rounded to 4 decimals as the posts write it.
fn good_in_inches() -> String {
    let inch = |mm: f64| format!("{:.4}", mm / 25.4);
    format!(
        "G20 G90 G17 G94\nG0 Z{z}\nM3 S18000\nG4 P3\nG0 X{a} Y{a}\nG1 Z{d} F{f}\nG1 X{b} F{g}\n\
         G1 Y{b}\nG1 X{a}\nG1 Y{a}\nG0 Z{z}\nM5\nM30\n",
        z = inch(5.0),
        a = inch(10.0),
        b = inch(50.0),
        d = inch(-6.0),
        f = inch(600.0),
        g = inch(1800.0),
    )
}

#[test]
fn an_inch_program_is_checked_in_mm() {
    let report = check_gcode(&good_in_inches(), &inch_limits());
    assert!(report.ok(), "{:?}", report.errors);
    // Its depths are seen in mm: 0.5" (12.7 mm) into 18 mm stock is fine, into 10 mm
    // stock it is not.
    let thin = CheckLimits {
        stock_bottom_z: -10.0,
        ..inch_limits()
    };
    let deep = good_in_inches().replace("Z-0.2362", "Z-0.5000");
    assert!(check_gcode(&deep, &inch_limits()).ok());
    let report = check_gcode(&deep, &thin);
    assert!(
        report
            .errors
            .iter()
            .any(|e| e.message.contains("below the stock bottom")),
        "{:?}",
        report.errors
    );
}

#[test]
fn an_inch_program_rounded_just_below_safe_height_passes() {
    // 4 decimals: 5.0 mm safe height could come out up to 0.00127 mm low.
    let limits = CheckLimits {
        safe_z: 5.001,
        ..inch_limits()
    };
    let program = good_in_inches();
    assert!(program.contains("Z0.1969"));
    let report = check_gcode(&program.replace("G0 X", "G0 Z0.1969\nG0 X"), &limits);
    assert!(report.ok(), "{:?}", report.errors);
}

#[test]
fn units_that_differ_from_the_machine_or_change_are_errors() {
    // An inch machine given a mm program, and the other way round.
    let report = check_gcode(&good(), &inch_limits());
    assert!(
        report.errors.iter().any(|e| e
            .message
            .contains("G21 (mm) but the machine's G-code units are inch")),
        "{:?}",
        report.errors
    );
    let report = check_gcode(&good_in_inches(), &limits());
    assert!(
        report.errors.iter().any(|e| e
            .message
            .contains("G20 (inch) but the machine's G-code units are mm")),
        "{:?}",
        report.errors
    );
    // A change of units part way.
    let changed = good_in_inches().replace("G1 Y", "G21\nG1 Y");
    let report = check_gcode(&changed, &inch_limits());
    assert!(
        report
            .errors
            .iter()
            .any(|e| e.message.contains("changes the units to mm")),
        "{:?}",
        report.errors
    );
    // Motion before either.
    let report = check_gcode("G90\nG0 Z5\nM30\n", &inch_limits());
    assert!(
        report
            .errors
            .iter()
            .any(|e| e.message.contains("Motion before G21 (mm) or G20 (inch)")),
        "{:?}",
        report.errors
    );
}
