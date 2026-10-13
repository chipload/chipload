// SPDX-License-Identifier: GPL-3.0-or-later
//! Material-removal simulation (docs/specs/simulation.md §7).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use chipload_job::migrations::migrate;
use chipload_job::{Job, Spindle, parse_job};
use chipload_pipeline::Cancel;
use chipload_pipeline::simulate::{
    DISPLAY_CELLS, History, PRINT_PALETTE, Simulation, simulate, simulate_with_history,
};
use chipload_toolpath::ToolGroup;
use chipload_toolpath::model::{Move, Program};
use serde_json::{Value, json};

const CELL: f64 = 0.1;

/// A 60 × 40 × 10 mm stock, Z0 on top, with one tool.
fn job(tool: Value) -> Job {
    parse_job(
        migrate(json!({
            "schema_version": 21, // written as v21 JSON: migrated into one part
            "drawing": {"shapes": [], "next_shape_id": 1, "dxfs": []},
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
        comments: Vec::new(),
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
        comments: Vec::new(),
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

// -- history for playback (docs/specs/playback.md §8)

/// A program of plunges, slots and ramps at various depths, from a seed.
fn wandering(seed: u64) -> Vec<Move> {
    let mut state = seed;
    let mut next = || {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((state >> 11) as f64) / ((1u64 << 53) as f64)
    };
    let mut moves = vec![Move::rapid_z(5.0), Move::rapid_xy((30.0, 20.0))];
    for _ in 0..40 {
        let p = (5.0 + next() * 50.0, 5.0 + next() * 30.0);
        let z = -next() * 6.0;
        match (next() * 3.0) as u32 {
            0 => moves.push(Move::feed_xyz(p, z, 1000.0)),
            1 => moves.push(Move::feed_xy(p, 1000.0)),
            _ => {
                moves.push(Move::rapid_z(5.0));
                moves.push(Move::rapid_xy(p));
                moves.push(Move::feed_z(z, 300.0));
            }
        }
    }
    moves.push(Move::rapid_z(5.0));
    moves
}

fn with_history(job: &Job, moves: &[Move], cell: f64) -> (Simulation, History) {
    let (program, groups) = program_of(job, moves.to_vec());
    let (sim, history) =
        simulate_with_history(job, &program, &groups, Some(cell), &Cancel::new()).unwrap();
    (sim, history.unwrap())
}

fn program_of(job: &Job, moves: Vec<Move>) -> (Program, Vec<ToolGroup>) {
    let program = Program {
        tool_id: 1,
        tool_diameter: job.tools[0].diameter,
        rpm: 18000.0,
        spindle: Spindle::Cw,
        safe_z: 5.0,
        moves,
        comments: Vec::new(),
    };
    let groups = vec![ToolGroup {
        tool: job.tools[0].clone(),
        operations: vec![0],
        moves: 0..program.moves.len(),
    }];
    (program, groups)
}

#[test]
fn the_history_gives_the_stock_after_any_move() {
    let vbit = json!({"id": 1, "type": "vbit", "diameter": 6, "angle": 1.0, "rpm": 18000});
    for (tool, cell) in [(endmill(4.0), 0.1), (endmill(4.0), 0.05), (vbit, 0.1)] {
        let job = job(tool);
        let moves = wandering(7);
        let (sim, history) = with_history(&job, &moves, cell);
        let factor = sim.factor_for(DISPLAY_CELLS, usize::MAX);
        assert_eq!((history.columns, history.rows), {
            let c = sim.coarsened(factor);
            (c.columns, c.rows)
        });
        for done in [0, 1, 5, 17, 40, 77, moves.len()] {
            let done = done.min(moves.len());
            let (program, groups) = program_of(&job, moves[..done].to_vec());
            let fresh = simulate(&job, &program, &groups, Some(cell), &Cancel::new())
                .unwrap()
                .coarsened(factor);
            assert_eq!(
                history.heights_after(done),
                fresh.heights,
                "cell {cell}, after {done} moves"
            );
        }
        // At the end: the display grid of the full simulation.
        assert_eq!(
            history.heights_after(moves.len()),
            sim.coarsened(factor).heights
        );
        // Each cell's entries in move order, going down.
        for c in 0..history.columns * history.rows {
            let run = history.cell_entries(c);
            for w in run.windows(2) {
                assert!(w[0].0 < w[1].0 && w[0].1 > w[1].1, "{w:?}");
            }
        }
        assert!(!history.is_empty());
    }
}

/// The per-frame work while dragging the playback slider (docs/specs/playback.md §4), on a
/// 600 × 400 mm sheet with a long program. Printed, not asserted (machines differ): run
/// with `cargo test --release -p chipload-pipeline --test simulate -- --ignored --nocapture`.
#[test]
#[ignore]
fn playback_frame_timing() {
    let mut job = job(endmill(6.0));
    job.stock.x = 600.0;
    job.stock.y = 400.0;
    // Rows of cuts over the whole sheet in three depth passes: about 10 000 moves.
    let mut moves = vec![Move::rapid_z(5.0), Move::rapid_xy((10.0, 10.0))];
    for pass in 1..=3 {
        moves.push(Move::feed_z(-2.0 * f64::from(pass), 300.0));
        for row in 0..80 {
            let y = 10.0 + f64::from(row) * 4.8;
            for step in 0..42 {
                let x = if row % 2 == 0 {
                    10.0 + f64::from(step) * 14.0
                } else {
                    584.0 - f64::from(step) * 14.0
                };
                moves.push(Move::feed_xy((x, y), 2000.0));
            }
        }
    }
    let (program, groups) = program_of(&job, moves);
    let started = std::time::Instant::now();
    let (sim, history) =
        simulate_with_history(&job, &program, &groups, None, &Cancel::new()).unwrap();
    let history = history.unwrap();
    println!(
        "{} moves; simulation with history: {:.0} ms; cells {} ({} × {}); display {} × {}; \
         {} history entries ({:.0} MB)",
        program.moves.len(),
        started.elapsed().as_secs_f64() * 1000.0,
        sim.cell,
        sim.columns,
        sim.rows,
        history.columns,
        history.rows,
        history.len(),
        history.len() as f64 * 8.0 / 1e6
    );
    let frames = 20;
    let started = std::time::Instant::now();
    for f in 0..frames {
        let stock = history.after(program.moves.len() * f / frames, &sim.numbers);
        let pixels = stock.shaded(PRINT_PALETTE);
        assert!(!pixels.is_empty());
    }
    println!(
        "per frame (heights + shading): {:.1} ms",
        started.elapsed().as_secs_f64() * 1000.0 / frames as f64
    );
}

/// A 40 × 30 mm rectangle pocketed 2 mm deep with a 12 mm endmill, then, with `rest`,
/// rest machined with a 3 mm one (docs/specs/rest-machining.md §8).
fn rest_job(rest: bool) -> Job {
    let pocket = |tool: i64, rest: Value| {
        json!({"type": "pocket", "select": {"layer": "L"}, "tool": tool,
            "start_depth": 0, "end_depth": 2, "clearance": 0, "step_down": 2,
            "stepover": 0.4, "direction": "climb", "feed": 1000, "plunge_feed": 300,
            "strategy": {"type": "rings"}, "entry": {"type": "plunge"},
            "round_corners": false, "corners": null, "rest": rest})
    };
    let mut operations = vec![pocket(1, Value::Null)];
    if rest {
        operations.push(pocket(2, json!({"after_tool": 1})));
    }
    parse_job(
        migrate(json!({
            "schema_version": 21, // written as v21 JSON: migrated into one part
            "drawing": {"shapes": [{"id": 1, "layer": "L", "kind": "rectangle",
                "center": [30, 20], "length": 40, "width": 30, "angle": 0}],
                "next_shape_id": 2, "dxfs": []},
            "stock": {"x": 60, "y": 40, "thickness": 10, "origin": "bottom_left",
                      "max_overcut": 1.0},
            "machine": {"post": "grbl", "safe_z": 5, "spindle": "cw", "tool_change": "program"},
            "tools": [
                {"id": 1, "diameter": 12.0, "rpm": 18000, "center_cutting": true},
                {"id": 2, "diameter": 3.0, "rpm": 18000, "center_cutting": true},
            ],
            "operations": operations,
        }))
        .unwrap(),
        "test.json",
    )
    .unwrap()
}

/// Whether the cell centre (x, y) lies where a tool of radius `r` can't reach in the
/// pocket's corners (the fillet of radius r a square corner keeps), with `slack` mm round it.
fn in_corner_fillet(x: f64, y: f64, r: f64, slack: f64) -> bool {
    let (x0, x1, y0, y1) = (10.0, 50.0, 5.0, 35.0);
    [(x0, y0), (x1, y0), (x1, y1), (x0, y1)]
        .into_iter()
        .any(|(cx, cy)| {
            let (dx, dy) = ((x - cx).abs(), (y - cy).abs());
            if dx >= r + slack || dy >= r + slack {
                return false;
            }
            // Outside the circle the tool's edge follows in the corner.
            (dx - r).hypot(dy - r) > r - slack
        })
}

#[test]
fn rest_machining_cuts_the_floor_into_the_small_tool_s_corners() {
    let simulated = |rest: bool| {
        let job = rest_job(rest);
        let result = chipload_pipeline::run_job(&job).unwrap();
        simulate(
            &job,
            &result.program,
            &result.groups,
            Some(CELL),
            &Cancel::new(),
        )
        .unwrap()
    };
    let with = simulated(true);
    let without = simulated(false);
    let mut standing_before = 0;
    for j in 0..with.rows {
        for i in 0..with.columns {
            let x = with.origin.0 + (i as f64 + 0.5) * CELL;
            let y = with.origin.1 + (j as f64 + 0.5) * CELL;
            if !(10.0..50.0).contains(&x) || !(5.0..35.0).contains(&y) {
                continue;
            }
            if in_corner_fillet(x, y, 1.5, 2.0 * CELL) {
                continue;
            }
            assert_eq!(with.height(i, j), -2.0, "({x:.2}, {y:.2}) is not cut");
            if without.height(i, j) > -2.0 {
                standing_before += 1;
                assert!(in_corner_fillet(x, y, 6.0, 2.0 * CELL), "({x:.2}, {y:.2})");
            }
        }
    }
    // The 12 mm tool alone leaves its corners: 4 · r²(1 − π/4) ≈ 30.9 mm², less the
    // 3 mm tool's, in cells of 0.01 mm².
    assert!(standing_before > 2500, "{standing_before} cells");
}
