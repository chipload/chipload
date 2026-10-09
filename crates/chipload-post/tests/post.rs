// SPDX-License-Identifier: GPL-3.0-or-later
//! Post output rules.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use chipload_check::{CheckLimits, check_gcode};
use chipload_job::Spindle;
use chipload_post::{load_builtin_post, parse_post_config, render_program};
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
        stock_min: (0.0, 0.0),
        stock_max: (600.0, 400.0),
        tool_radius: 3.0,
        travel: Some((800.0, 600.0, 100.0)),
    };
    let report = check_gcode(&text, &limits);
    assert!(report.ok(), "{:?}", report.errors);
    assert!(report.warnings.is_empty(), "{:?}", report.warnings);
}
