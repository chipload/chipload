// SPDX-License-Identifier: GPL-3.0-or-later
//! Material-removal simulation (docs/specs/simulation.md §7).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use chipload_job::migrations::migrate;
use chipload_job::{CURRENT_VERSION, Job, Spindle, parse_job};
use chipload_pipeline::Cancel;
use chipload_pipeline::simulate::{PRINT_PALETTE, Simulation, simulate};
use chipload_toolpath::ToolGroup;
use chipload_toolpath::model::{Move, Program};
use serde_json::{Value, json};

const CELL: f64 = 0.1;

/// A 60 × 40 × 10 mm stock, Z0 on top, with one tool.
fn job(tool: Value) -> Job {
    parse_job(
        migrate(json!({
            "schema_version": CURRENT_VERSION,
            "drawing": {"shapes": [], "next_shape_id": 1, "source": null},
            "stock": {"x": 60, "y": 40, "thickness": 10, "origin": "bottom_left",
                      "max_overcut": 1.0},
            "machine": {"post": "grbl", "safe_z": 5, "spindle": "cw", "tool_change": "files"},
            "tools": [tool],
            "operations": [{"type": "profile", "select": {"layer": "L"}, "side": "outside",
                "direction": "climb", "tool": 1, "start_depth": 0, "end_depth": 2,
                "clearance": 0, "step_down": 2, "feed": 1000, "plunge_feed": 300,
                "entry": {"type": "plunge"}}],
        }))
        .unwrap(),
        "test.json",
    )
    .unwrap()
}

fn endmill(diameter: f64) -> Value {
    json!({"id": 1, "type": "endmill", "diameter": diameter, "rpm": 18000})
}

fn run(job: &Job, moves: Vec<Move>) -> Simulation {
    let program = Program {
        tool_id: 1,
        tool_diameter: job.tools[0].diameter,
        rpm: 18000.0,
        spindle: Spindle::Cw,
        safe_z: 5.0,
        moves,
    };
    let groups = vec![ToolGroup {
        tool: job.tools[0].clone(),
        operations: vec![0],
        moves: 0..program.moves.len(),
    }];
    simulate(job, &program, &groups, Some(CELL), &Cancel::new()).unwrap()
}

/// The height at the cell whose centre is nearest to (x, y).
fn at(sim: &Simulation, x: f64, y: f64) -> f64 {
    let i = ((x - sim.origin.0) / sim.cell).floor() as usize;
    let j = ((y - sim.origin.1) / sim.cell).floor() as usize;
    sim.height(i, j)
}

fn plunge(x: f64, y: f64, z: f64) -> Vec<Move> {
    vec![
        Move::rapid_z(5.0),
        Move::rapid_xy((x, y)),
        Move::feed_z(z, 300.0),
        Move::rapid_z(5.0),
    ]
}

#[test]
fn a_plunge_makes_a_round_hole_of_the_tool() {
    let sim = run(&job(endmill(6.0)), plunge(20.0, 20.0, -3.0));
    assert_eq!(at(&sim, 20.0, 20.0), -3.0);
    assert_eq!(at(&sim, 22.9, 20.0), -3.0);
    // Outside the tool's radius plus a cell: untouched.
    assert_eq!(at(&sim, 23.2, 20.0), 0.0);
    assert_eq!(at(&sim, 20.0, 16.8), 0.0);
    // πr²·depth = 84.8 mm³, within a cell's width round the edge.
    let volume = sim.numbers.removed_cm3 * 1000.0;
    let ring = 2.0 * std::f64::consts::PI * 3.0 * CELL * 3.0;
    assert!((volume - 84.823).abs() < ring, "{volume}");
    assert_eq!(sim.numbers.deepest, Some(-3.0));
    assert_eq!(sim.numbers.below_bottom, None);
}

#[test]
fn a_slot_has_the_tools_width_and_a_flat_floor() {
    let mut moves = plunge(10.0, 20.0, -2.0);
    moves.insert(3, Move::feed_xy((50.0, 20.0), 1000.0));
    let sim = run(&job(endmill(6.0)), moves);
    for x in [10.0, 25.0, 40.0, 50.0] {
        assert_eq!(at(&sim, x, 20.0), -2.0, "{x}");
        assert_eq!(at(&sim, x, 22.95), -2.0);
        assert_eq!(at(&sim, x, 23.1), 0.0);
    }
}

#[test]
fn a_ramp_leaves_a_sloping_floor() {
    let moves = vec![
        Move::rapid_z(5.0),
        Move::rapid_xy((10.0, 20.0)),
        Move::feed_z(0.0, 300.0),
        Move::feed_xyz((50.0, 20.0), -4.0, 1000.0),
        Move::rapid_z(5.0),
    ];
    let sim = run(&job(endmill(2.0)), moves);
    // Under the tool's front edge the lowest point is where the edge passed lowest: the
    // floor at x is the ramp's Z at x + r (the tool covers x until its centre is r past it).
    for x in [20.05, 30.05, 40.05] {
        let want = -4.0 * ((x + 1.0 - 10.0) / 40.0);
        assert!(
            (at(&sim, x, 20.0) - want).abs() < 0.02,
            "{x}: {}",
            at(&sim, x, 20.0)
        );
    }
}

#[test]
fn a_v_bit_groove_is_as_wide_as_its_cone() {
    // 90° V-bit, 2 mm deep: 4 mm wide at the top.
    let vbit = json!({"id": 1, "type": "vbit", "diameter": 12.7, "angle": std::f64::consts::FRAC_PI_2,
                      "rpm": 18000});
    let mut moves = plunge(10.0, 20.0, -2.0);
    moves.insert(3, Move::feed_xy((50.0, 20.0), 1000.0));
    let sim = run(&job(vbit), moves);
    assert!((at(&sim, 30.0, 20.05) - -1.95).abs() < 0.01);
    assert!(at(&sim, 30.0, 21.95) < 0.0);
    assert_eq!(at(&sim, 30.0, 22.05), 0.0);
}

#[test]
fn a_drill_leaves_a_cone_at_the_bottom() {
    let drill = json!({"id": 1, "type": "drill", "diameter": 8, "rpm": 6000});
    let sim = run(&job(drill), plunge(20.0, 20.0, -6.0));
    assert!((at(&sim, 20.05, 20.05) - -6.0).abs() < 0.1);
    // 118°: the side rises cot(59°) = 0.6 mm per mm out.
    let slope = 1.0 / 59.0f64.to_radians().tan();
    let h = at(&sim, 23.05, 20.05);
    let want = -6.0 + slope * 3.05;
    assert!((h - want).abs() < 0.1, "{h} vs {want}");
    assert!(sim.numbers.drill_point_assumed);
    assert!(sim.lines().iter().any(|l| l.contains("118° point")));
}

#[test]
fn rapids_cut_nothing_and_cells_only_go_down() {
    let moves = vec![
        Move::rapid_z(5.0),
        Move::rapid_xy((20.0, 20.0)),
        Move::rapid_z(-1.0),
        Move::rapid_xy((40.0, 20.0)),
    ];
    let sim = run(&job(endmill(6.0)), moves);
    assert!(sim.heights.iter().all(|&h| h == 0.0));
    assert_eq!(sim.numbers.deepest, None);
    // Deep, then shallow over the same place: the deep cut stays.
    let mut moves = plunge(20.0, 20.0, -3.0);
    moves.extend(plunge(20.0, 20.0, -1.0));
    let sim = run(&job(endmill(6.0)), moves);
    assert_eq!(at(&sim, 20.0, 20.0), -3.0);
}

#[test]
fn a_cut_below_the_stock_is_reported() {
    let sim = run(&job(endmill(6.0)), plunge(20.0, 20.0, -10.5));
    assert!((sim.numbers.below_bottom.unwrap() - 0.5).abs() < 1e-9);
    assert!(
        sim.lines()
            .iter()
            .any(|l| l.contains("0.500 mm below the stock bottom"))
    );
}

#[test]
fn the_same_input_gives_the_same_heights_and_image() {
    let mut moves = plunge(10.0, 20.0, -2.0);
    moves.insert(3, Move::feed_xyz((50.0, 30.0), -3.0, 1000.0));
    let a = run(&job(endmill(6.0)), moves.clone());
    let b = run(&job(endmill(6.0)), moves);
    assert_eq!(a, b);
    assert_eq!(a.heights_hash(), b.heights_hash());
    assert_eq!(a.png(PRINT_PALETTE).unwrap(), b.png(PRINT_PALETTE).unwrap());
    // One pixel per cell.
    assert_eq!(a.shaded(PRINT_PALETTE).len(), 600 * 400 * 4);
}

#[test]
fn a_cell_size_out_of_range_or_too_fine_is_refused() {
    let job = job(endmill(6.0));
    let program = Program {
        tool_id: 1,
        tool_diameter: 6.0,
        rpm: 18000.0,
        spindle: Spindle::Cw,
        safe_z: 5.0,
        moves: Vec::new(),
    };
    for cell in [0.01, 20.0] {
        let err = simulate(&job, &program, &[], Some(cell), &Cancel::new()).unwrap_err();
        assert!(
            err.to_string().contains("cell size must be between"),
            "{err}"
        );
    }
    let mut big = job.clone();
    big.stock.x = 2000.0;
    big.stock.y = 1000.0;
    let err = simulate(&big, &program, &[], Some(0.1), &Cancel::new()).unwrap_err();
    assert!(err.to_string().contains("use a larger cell"), "{err}");
}
