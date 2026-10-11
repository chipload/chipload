// SPDX-License-Identifier: GPL-3.0-or-later
//! Post output rules.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use chipload_check::{CheckLimits, check_gcode};
use chipload_job::{Spindle, Tool, ToolChange, ToolType};
use chipload_post::{
    ToolPart, load_builtin_post, parse_post_config, render_program, render_with_tool_changes,
};
use chipload_toolpath::model::{Move, Program};

fn program(spindle: Spindle) -> Program {
    Program {
        tool_id: 1,
        tool_diameter: 6.0,
        rpm: 18000.0,
        spindle,
        safe_z: 5.0,
        moves: vec![
            Move::rapid_z(5.0),
            Move::rapid_xy((10.0, 10.0)),
            Move::feed_z(-1.0, 300.0),
            Move::feed_xy((50.0, 10.0), 1000.0),
            Move::feed_xy((50.0, 40.0), 1000.0),
            Move::feed_xy((10.0, 40.0), 1000.0),
            Move::feed_xy((10.0, 10.0), 1000.0),
            Move::rapid_z(5.0),
        ],
    }
}

#[test]
fn output_is_deterministic() {
    let post = load_builtin_post("grbl").unwrap();
    let a = render_program(&program(Spindle::Cw), &post).unwrap();
    let b = render_program(&program(Spindle::Cw), &post).unwrap();
    assert_eq!(a, b);
}

#[test]
fn ccw_spindle_uses_m4() {
    let post = load_builtin_post("grbl").unwrap();
    let text = render_program(&program(Spindle::Ccw), &post).unwrap();
    assert!(text.lines().any(|l| l == "M4 S18000"), "{text}");
    assert!(!text.lines().any(|l| l.starts_with("M3 ")), "{text}");
}

#[test]
fn zero_dwell_is_omitted() {
    let config =
        "schema_version = 1\nname = \"T\"\ntemplate = \"grbl.nc.j2\"\nspindle_dwell_s = 0\n";
    let post = parse_post_config(config).unwrap();
    let text = render_program(&program(Spindle::Cw), &post).unwrap();
    assert!(!text.contains("G4"), "{text}");
}

#[test]
fn rendered_program_passes_the_checker() {
    let post = load_builtin_post("grbl").unwrap();
    let text = render_program(&program(Spindle::Cw), &post).unwrap();
    let limits = CheckLimits {
        safe_z: 5.0,
        stock_top_z: 0.0,
        stock_bottom_z: -18.0,
        max_overcut: 0.0,
        stock_min: (0.0, 0.0),
        stock_max: (600.0, 400.0),
        tool_radius: 3.0,
        tool_radii: std::collections::BTreeMap::from([(1, 3.0), (2, 1.5)]),
        travel: Some((800.0, 600.0, 100.0)),
    };
    let report = check_gcode(&text, &limits);
    assert!(report.ok(), "{:?}", report.errors);
    assert!(report.warnings.is_empty(), "{:?}", report.warnings);
}

#[test]
fn a_post_without_tool_changes_refuses_one_file_with_m6() {
    // docs/specs/tool-per-operation.md §3.2: a config that doesn't list "program" (missing
    // means files only) is never asked to write M6.
    let mut post = load_builtin_post("grbl").unwrap();
    assert_eq!(post.tool_changes, [ToolChange::Files, ToolChange::Program]);
    let text = "schema_version = 1\nname = \"Old\"\ntemplate = \"grbl.nc.j2\"\n\
                spindle_dwell_s = 0\n";
    let old = parse_post_config(text).unwrap();
    assert_eq!(old.tool_changes, [ToolChange::Files]);
    let tool = |id: i64, diameter: f64| Tool {
        id,
        name: None,
        kind: ToolType::Endmill,
        diameter,
        flutes: None,
        center_cutting: None,
        rpm: Some(18000.0),
        angle: None,
        tip_diameter: None,
    };
    let parts = [
        ToolPart {
            program: program(Spindle::Cw),
            tool: tool(1, 6.0),
            operations: vec![0],
        },
        ToolPart {
            program: program(Spindle::Cw),
            tool: tool(2, 3.0),
            operations: vec![1],
        },
    ];
    let err = render_with_tool_changes(&parts, &old, None).unwrap_err();
    assert!(
        err.to_string()
            .contains("can't write tool changes in one file"),
        "{err}"
    );
    post.arcs = false;
    let text = render_with_tool_changes(&parts, &post, None).unwrap();
    assert_eq!(text.matches(" M6\n").count(), 2, "{text}");
}
