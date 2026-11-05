// SPDX-License-Identifier: GPL-3.0-or-later
//! Two-sided work (docs/specs/two-sided.md): a back setup is the same board as its front,
//! turned over.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;

use chipload_job::{BackOf, Job, Register, StockOrigin, Turn, WorkZero, load_job};
use chipload_pipeline::setups::{
    BoardChange, add_back_side, move_part_to_setup, remove_setup, set_board,
};

/// Two parts: "simple part" (operations 1-3) and "part B" (operation 4), one setup.
fn sample() -> Job {
    load_job(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/corpus/jobs/v22/two_parts.chipload"),
    )
    .unwrap()
}

/// The sample with a back side, setup 2, holding part B.
fn with_back() -> Job {
    let (job, id, _) = add_back_side(&sample(), 1, Turn::LeftRight, None).unwrap();
    assert_eq!(id, 2);
    move_part_to_setup(&job, 2, 2).unwrap()
}

#[test]
fn a_back_side_shares_its_fronts_board_and_zero() {
    let job = with_back();
    let back = job.setup(2).unwrap();
    assert_eq!(back.name, "Setup 1 back");
    assert_eq!(
        back.back_of,
        Some(BackOf {
            setup: 1,
            turn: Turn::LeftRight,
            register: Register::Pins,
        })
    );
    assert_eq!(back.stock, job.setup(1).unwrap().stock);
    assert_eq!(back.work_zero, job.setup(1).unwrap().work_zero);
    // The front changes, the back follows wherever a project is read or changed.
    let mut changed = job.clone();
    changed.setups[0].stock.thickness = 18.0;
    changed.setups[0].work_zero = WorkZero::Point { x: 3.0, y: 4.0 };
    // Set on the back by hand: not its own, so it goes.
    changed.setups[1].stock.thickness = 5.0;
    changed.follow_thickness();
    assert_eq!(changed.setup(2).unwrap().stock.thickness, 18.0);
    assert_eq!(
        changed.setup(2).unwrap().work_zero,
        WorkZero::Point { x: 3.0, y: 4.0 }
    );
}

#[test]
fn a_board_that_fits_fits_both_sides() {
    let mut job = with_back();
    for setup in &mut job.setups {
        setup.stock.origin = StockOrigin::Drawing;
        setup.stock.fit = Some(10.0);
    }
    job.follow_thickness();
    let front = job.for_setup(1).stock_bounds();
    let back = job.for_setup(2).stock_bounds();
    assert_eq!(front, back, "one board");
    // Unlinked, each setup fits its own part, so the front's board is smaller.
    let own = set_board(
        &job,
        2,
        BoardChange {
            back_of: Some(None),
            ..BoardChange::default()
        },
    )
    .unwrap();
    let alone = own.for_setup(1).stock_bounds();
    let area = |((x0, y0), (x1, y1)): ((f64, f64), (f64, f64))| (x1 - x0) * (y1 - y0);
    assert!(area(alone) < area(front), "{alone:?} {front:?}");
}

#[test]
fn one_board_has_one_back_and_a_back_is_not_turned_again() {
    let job = with_back();
    // A second back of setup 1.
    assert!(add_back_side(&job, 1, Turn::TopBottom, None).is_err());
    // The back of the back.
    assert!(add_back_side(&job, 2, Turn::LeftRight, None).is_err());
    // A setup turned over onto itself.
    let itself = BoardChange {
        back_of: Some(Some(2)),
        ..BoardChange::default()
    };
    assert!(set_board(&job, 2, itself).is_err());
    // A turn for a setup that is not a back.
    let turn = BoardChange {
        turn: Some(Turn::TopBottom),
        ..BoardChange::default()
    };
    let err = set_board(&job, 1, turn).unwrap_err();
    assert!(err.to_string().contains("not the back"), "{err}");
    // A back's turn and way of locating change; its front stays.
    let changed = set_board(
        &job,
        2,
        BoardChange {
            turn: Some(Turn::TopBottom),
            register: Some(Register::Corner),
            ..BoardChange::default()
        },
    )
    .unwrap();
    assert_eq!(
        changed.setup(2).unwrap().back_of,
        Some(BackOf {
            setup: 1,
            turn: Turn::TopBottom,
            register: Register::Corner,
        })
    );
}

#[test]
fn removing_the_front_leaves_the_back_its_board() {
    let mut job = with_back();
    job.setups[0].stock.thickness = 15.0;
    job.follow_thickness();
    let (job, lines) = remove_setup(&job, 1, true).unwrap();
    assert!(
        lines.iter().any(|l| l.contains("board of its own")),
        "{lines:?}"
    );
    let back = job.setup(2).unwrap();
    assert_eq!(back.back_of, None);
    assert_eq!(back.stock.thickness, 15.0);
}

#[test]
fn a_back_side_is_saved_and_read_back() {
    let job = with_back();
    let dir = std::env::temp_dir().join(format!("chipload-two-sided-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("back.chipload");
    chipload_job::save_job(&job, &path).unwrap();
    let read = load_job(&path).unwrap();
    assert_eq!(
        read.setup(2).unwrap().back_of,
        job.setup(2).unwrap().back_of
    );
    assert_eq!(read.schema_version, 36);
    std::fs::remove_dir_all(&dir).unwrap();
}

// -- the cut direction on the back face (docs/specs/two-sided.md §4)

fn corpus_value(rel: &str) -> serde_json::Value {
    let text = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/corpus")
            .join(rel),
    )
    .unwrap();
    chipload_job::migrations::migrate(serde_json::from_str(&text).unwrap()).unwrap()
}

/// A pocket, an inside and an outside profile (op_add), plus a helical bore of the round
/// hole and a follow path round the slot, climb and conventional both: every operation
/// with a direction. Its shapes mirrored about `axis` (x: the line X = at) when given.
fn every_direction(mirror: Option<(Turn, f64)>) -> Job {
    use chipload_pipeline::new_ops::{NewRequest, OpKind, insert, new_operations};
    let mut data = corpus_value("jobs/v35/op_add.chipload");
    if let Some((turn, at)) = mirror {
        let flip = |p: &mut serde_json::Value| {
            let k = match turn {
                Turn::LeftRight => 0,
                Turn::TopBottom => 1,
            };
            p[k] = (2.0 * at - p[k].as_f64().unwrap()).into();
        };
        for shape in data["drawing"]["shapes"].as_array_mut().unwrap() {
            for segment in shape["segments"].as_array_mut().unwrap() {
                if let Some(line) = segment.get_mut("line") {
                    flip(&mut line["start"]);
                    flip(&mut line["end"]);
                } else {
                    let arc = &mut segment["arc"];
                    flip(&mut arc["center"]);
                    let (a, s) = (
                        arc["start_angle"].as_f64().unwrap(),
                        arc["sweep"].as_f64().unwrap(),
                    );
                    // x -> -x turns an angle t into pi - t, y -> -y into -t; the arc keeps
                    // its place in the path, so it runs the other way round.
                    arc["start_angle"] = match turn {
                        Turn::LeftRight => std::f64::consts::PI - a,
                        Turn::TopBottom => -a,
                    }
                    .into();
                    arc["sweep"] = (-s).into();
                }
            }
        }
    }
    // Conventional milling on the pocket, climb on the profiles.
    data["operations"][0]["direction"] = "conventional".into();
    let mut job = chipload_job::parse_job(data, "test").unwrap();
    let contours = chipload_pipeline::run_job(&job).unwrap().contours;
    for (kind, shape) in [(OpKind::Holes, 2), (OpKind::Path, 3)] {
        let request = NewRequest {
            kind,
            select: chipload_job::Selector {
                shapes: Some(vec![shape]),
                ..Default::default()
            },
            tool: None,
        };
        let made = new_operations(&job, &request, &contours, None, chipload_i18n::Language::En);
        insert(&mut job, made);
    }
    job.follow_thickness();
    assert!(job.validate().is_empty(), "{:?}", job.validate());
    job
}

/// Each run of cutting moves between rapids, by operation, as the signed area of the loop
/// it makes when closed by a straight line back to its start (mm², counter-clockwise
/// positive; arcs counted along their curve).
fn loops(
    program: &chipload_toolpath::model::Program,
    spans: &[chipload_toolpath::OpSpan],
) -> Vec<(usize, Vec<f64>)> {
    use chipload_toolpath::model::MoveKind;
    let mut out = Vec::new();
    for span in spans {
        let mut runs = Vec::new();
        let mut area = 0.0;
        let mut at: Option<(f64, f64)> = None;
        // Where the run being measured began.
        let mut from: Option<(f64, f64)> = None;
        let close = |area: f64, from: Option<(f64, f64)>, to: Option<(f64, f64)>| match (from, to) {
            (Some(a), Some(b)) => area + (b.0 * a.1 - a.0 * b.1) / 2.0,
            _ => area,
        };
        // Where the tool is before the span: the last XY set before it.
        for m in &program.moves[..span.moves.start] {
            if let (Some(x), Some(y)) = (m.x, m.y) {
                at = Some((x, y));
            }
        }
        for m in &program.moves[span.moves.clone()] {
            let to = match (at, m.x, m.y) {
                (Some(p), x, y) => (x.unwrap_or(p.0), y.unwrap_or(p.1)),
                (None, Some(x), Some(y)) => (x, y),
                _ => continue,
            };
            match (m.kind, at) {
                (MoveKind::Rapid, _) => {
                    let closed = close(area, from, at);
                    if closed != 0.0 {
                        runs.push(closed);
                    }
                    area = 0.0;
                    from = None;
                }
                (kind, Some(p)) => {
                    from = from.or(Some(p));
                    area += (p.0 * to.1 - to.0 * p.1) / 2.0;
                    if let (MoveKind::ArcCw | MoveKind::ArcCcw, Some(c)) = (kind, m.circle) {
                        let angle = |q: (f64, f64)| (q.1 - c.center.1).atan2(q.0 - c.center.0);
                        let tau = std::f64::consts::TAU;
                        let mut sweep = (angle(to) - angle(p)).rem_euclid(tau);
                        if sweep == 0.0 {
                            sweep = tau;
                        }
                        if kind == MoveKind::ArcCw {
                            sweep -= tau;
                        }
                        area += c.radius * c.radius / 2.0 * (sweep - sweep.sin());
                    }
                }
                _ => {}
            }
            at = Some(to);
        }
        let closed = close(area, from, at);
        if closed != 0.0 {
            runs.push(closed);
        }
        out.push((span.operation, runs));
    }
    out
}

/// The area cut each way by each operation: (counter-clockwise, clockwise) totals.
fn turning(loops: &[(usize, Vec<f64>)]) -> Vec<(usize, f64, f64)> {
    let mut ops: Vec<usize> = loops.iter().map(|(op, _)| *op).collect();
    ops.dedup();
    ops.into_iter()
        .map(|op| {
            let runs: Vec<f64> = loops
                .iter()
                .filter(|(o, _)| *o == op)
                .flat_map(|(_, r)| r.clone())
                .collect();
            let ccw: f64 = runs.iter().filter(|a| **a > 0.0).sum();
            let cw: f64 = runs.iter().filter(|a| **a < 0.0).sum();
            (op, ccw, cw)
        })
        .collect()
}

#[test]
fn climb_stays_climb_on_the_back_face() {
    for turn in [Turn::LeftRight, Turn::TopBottom] {
        let front = every_direction(None);
        // The same shapes cut from the back: the whole part in setup 2, the back of 1.
        let (two, back, _) = add_back_side(&front, 1, turn, None).unwrap();
        let mut two = two;
        for part in &mut two.parts {
            part.setup = back;
        }
        two.follow_thickness();
        let one = two.for_setup(back);
        let ((x0, y0), (x1, y1)) = one.stock_bounds();
        let axis = match turn {
            Turn::LeftRight => (x0 + x1) / 2.0,
            Turn::TopBottom => (y0 + y1) / 2.0,
        };
        let result = chipload_pipeline::run_job(&one).unwrap();
        let on_board = chipload_pipeline::on_the_board(&one, &result.program);
        // The shapes drawn mirrored by hand, cut from the front.
        let by_hand = every_direction(Some((turn, axis)));
        let expected = chipload_pipeline::run_job(&by_hand).unwrap();
        let got = turning(&loops(&on_board, &result.op_spans));
        let want = turning(&loops(&expected.program, &expected.op_spans));
        assert_eq!(got.len(), 5, "{turn:?}: every operation cut: {got:?}");
        assert_eq!(got.len(), want.len(), "{turn:?}");
        // The share of each operation's cut area that runs counter-clockwise: 1 or 0 for
        // a direction kept throughout. It must be the same on the back as by hand.
        let share = |(_, ccw, cw): &(usize, f64, f64)| ccw / (ccw - cw);
        assert_eq!(got.len(), 5, "{turn:?}: every operation cut: {got:?}");
        assert_eq!(got.len(), want.len(), "{turn:?}");
        for (g, w) in got.iter().zip(&want) {
            assert_eq!(g.0, w.0);
            assert!(g.1 - g.2 > 1.0, "{turn:?} operation {} cuts", g.0);
            // The same loops: the areas agree but for where each ring starts, which moves
            // its ramp's overlap (a follow path starts on a line on one side, an arc on the
            // other: 6 %).
            let total = |t: &(usize, f64, f64)| t.1 - t.2;
            assert!(
                (total(g) - total(w)).abs() < 0.1 * total(w),
                "{turn:?} operation {}: back {g:?}, by hand {w:?}",
                g.0
            );
            assert!(
                (share(g) - share(w)).abs() < 0.01,
                "{turn:?} operation {}: back {g:?}, by hand {w:?}",
                g.0
            );
        }
        // Not by chance: the program as planned, before the mirror, turns the other way.
        let planned = turning(&loops(&result.program, &result.op_spans));
        for (p, w) in planned.iter().zip(&want) {
            assert!(
                (share(p) - (1.0 - share(w))).abs() < 0.01,
                "{turn:?} operation {}: planned {p:?}",
                p.0
            );
        }
        // And the G-code is the program on the board: its spindle is the machine's, M3.
        let gcode: String = result.files.iter().map(|f| f.gcode.clone()).collect();
        assert!(gcode.contains("M3") && !gcode.contains("M4"), "{turn:?}");
    }
}

#[test]
fn the_back_is_as_thick_as_the_front_leaves_the_board() {
    // docs/specs/two-sided.md §5: the corpus sample surfaces its 10 mm board 0.5 mm on the
    // front, and cuts its outline through from the back.
    let job = load_job(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/corpus/jobs/v36/two_sided.chipload"),
    )
    .unwrap();
    assert_eq!(job.setup(1).unwrap().stock.thickness, 10.0);
    assert_eq!(job.setup(2).unwrap().stock.thickness, 9.5);
    // Through the 9.5 mm, 0.5 mm into the spoilboard.
    assert_eq!(job.operations[4].depths().1, 10.0);
    // Without the surfacing the back is the whole board again.
    let mut whole = job.clone();
    whole.operations.remove(0);
    whole.follow_thickness();
    assert_eq!(whole.setup(2).unwrap().stock.thickness, 10.0);
    assert_eq!(whole.operations[3].depths().1, 10.5);
    // The back's program, checked against its own 9.5 mm board, goes no deeper.
    let back = chipload_pipeline::run_job(&job.for_setup(2)).unwrap();
    let deepest = back
        .program
        .moves
        .iter()
        .filter_map(|m| m.z)
        .fold(f64::INFINITY, f64::min);
    assert!((deepest + 10.0).abs() < 1e-9, "{deepest}");
}
