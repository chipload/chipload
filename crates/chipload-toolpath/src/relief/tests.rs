// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

/// A dome: a hemisphere of radius 20 at (0, 0), its top at 0, as triangles.
fn dome() -> Mesh {
    let (n, m) = (24, 48);
    let at = |i: usize, j: usize| {
        let polar = std::f64::consts::FRAC_PI_2 * i as f64 / n as f64;
        let az = std::f64::consts::TAU * j as f64 / m as f64;
        [
            20.0 * chipload_core::math::sin(polar) * chipload_core::math::cos(az),
            20.0 * chipload_core::math::sin(polar) * chipload_core::math::sin(az),
            20.0 * chipload_core::math::cos(polar) - 20.0,
        ]
    };
    let mut triangles = Vec::new();
    for i in 0..n {
        for j in 0..m {
            let (a, b, c, d) = (at(i, j), at(i, j + 1), at(i + 1, j), at(i + 1, j + 1));
            if i > 0 {
                triangles.push([a, c, b]);
            }
            triangles.push([b, c, d]);
        }
    }
    Mesh { triangles }
}

fn square(half: f64) -> Vec<Vec<Point>> {
    vec![vec![
        (-half, -half),
        (half, -half),
        (half, half),
        (-half, half),
    ]]
}

#[test]
fn a_dome_is_carved_without_cutting_into_it() {
    let mesh = dome();
    let index = DropMesh::new(&mesh, 6.0);
    let area = Area::Loops(square(22.0));
    for cutter in [
        Cutter::Ball { radius: 1.5 },
        Cutter::Flat { radius: 3.0 },
        Cutter::TaperedBall {
            radius: 3.175,
            ball: 0.5,
            half_angle: 10f64.to_radians(),
        },
    ] {
        let cut = ReliefCut {
            mesh: &index,
            cutter,
            top: 0.0,
            floor: -20.0,
            area: &area,
            ramp: None,
            step: 1.5,
            angle: 30f64.to_radians(),
            zigzag: true,
            safe_z: 5.0,
            feed: 1500.0,
            plunge_feed: 500.0,
            shift: (0.0, 0.0),
            progress: None,
        };
        let (moves, _) = relief_moves(&cut, (0.0, 0.0));
        assert!(moves.len() > 50, "{cutter:?}: {}", moves.len());
        assert_eq!(
            verify(&moves, &mesh, cutter, 0.0, (0.0, 0.0)),
            None,
            "{cutter:?}"
        );
        // The top of the dome is reached: some point near the middle stands at Z 0.
        let highest = moves
            .iter()
            .filter(|m| m.kind == MoveKind::Feed)
            .filter_map(|m| m.z)
            .fold(f64::NEG_INFINITY, f64::max);
        assert!(highest > -0.5, "{cutter:?}: {highest}");
    }
}

/// A dome with a rim (spec §16): a flat `foot` wide round its outline (the dome's circle
/// as a 48-gon), a 45° ramp up to the stock top.
fn rimmed(foot: f64, floor: f64) -> (Area, Ramp) {
    let circle: Vec<Point> = (0..48)
        .map(|k| {
            let a = std::f64::consts::TAU * k as f64 / 48.0;
            (
                20.0 * chipload_core::math::cos(a),
                20.0 * chipload_core::math::sin(a),
            )
        })
        .collect();
    let ramp = Ramp {
        loops: vec![circle.clone()],
        foot,
        tan: 1.0,
        floor,
        top: 0.0,
    };
    let area = Area::Grown {
        loops: vec![circle],
        reach: ramp.reach(),
    };
    (area, ramp)
}

#[test]
fn a_rim_is_cut_flat_then_ramped_up_to_the_top() {
    let mesh = dome();
    let ball = Cutter::Ball { radius: 1.0 };
    let index = DropMesh::new(&mesh, 2.0);
    let (area, ramp) = rimmed(3.0, -20.0);
    let cut = ReliefCut {
        mesh: &index,
        cutter: ball,
        top: 0.0,
        floor: -20.0,
        area: &area,
        ramp: Some(&ramp),
        step: 0.8,
        angle: 0.0,
        zigzag: true,
        safe_z: 5.0,
        feed: 1500.0,
        plunge_feed: 500.0,
        shift: (0.0, 0.0),
        progress: None,
    };
    let (moves, _) = relief_moves(&cut, (0.0, 0.0));
    assert_eq!(verify(&moves, &mesh, ball, 0.0, (0.0, 0.0)), None);
    assert_eq!(verify_ramp(&moves, &ramp, ball), None);
    // Every plunge goes down to the stock top: lines start where the ramp meets it.
    let plunges: Vec<f64> = moves
        .windows(2)
        .filter(|w| w[0].kind == MoveKind::Rapid && w[1].kind == MoveKind::Feed)
        .filter(|w| w[1].x.is_none() && w[1].y.is_none())
        .filter_map(|w| w[1].z)
        .collect();
    assert!(!plunges.is_empty());
    assert!(plunges.iter().all(|z| *z > -0.01), "{plunges:?}");
    // The flat at the floor: on the X axis between the dome (r 20) and the ramp's foot
    // (r 23), within the ball's radius of it, the tool stands at the floor.
    let feeds: Vec<(f64, f64, f64)> = positions(&moves)
        .into_iter()
        .map(|((x, y), z)| (x, y, z))
        .collect();
    let on_flat = feeds
        .iter()
        .filter(|(x, y, _)| y.abs() < 0.5 && (*x - 21.5).abs() < 0.4)
        .collect::<Vec<_>>();
    assert!(!on_flat.is_empty());
    assert!(
        on_flat.iter().all(|(_, _, z)| (z + 20.0).abs() < 0.02),
        "{on_flat:?}"
    );
    // Halfway up the ramp (10 mm out from its foot), a ball 1/cos45° − 1 above it.
    let up = feeds
        .iter()
        .filter(|(x, y, _)| y.abs() < 0.5 && (*x - 33.0).abs() < 0.2)
        .collect::<Vec<_>>();
    assert!(!up.is_empty());
    let rest = 2f64.sqrt() - 1.0;
    for (x, y, z) in up {
        let expected = ramp.surface((*x, *y)) + rest;
        assert!((z - expected).abs() < 0.02, "{x} {y} {z} {expected}");
    }
}

#[test]
fn a_rim_is_roughed_keeping_the_allowance_off_its_ramp() {
    let mesh = dome();
    let allowance = 0.5;
    let flat = Cutter::Flat { radius: 3.0 };
    let grown = flat.grown(allowance).unwrap();
    let index = DropMesh::new(&mesh, 2.0 * grown.radius());
    let (area, ramp) = rimmed(10.0, -20.0 + allowance);
    let ramp = Ramp {
        top: allowance,
        ..ramp
    };
    let cut = ReliefCut {
        mesh: &index,
        cutter: grown,
        top: allowance,
        floor: -20.0 + allowance,
        area: &area.shrunk(2.0).unwrap(),
        ramp: Some(&ramp),
        step: 2.4,
        angle: 0.0,
        zigzag: true,
        safe_z: 5.0,
        feed: 1500.0,
        plunge_feed: 500.0,
        shift: (0.0, 0.0),
        progress: None,
    };
    let (moves, _) = rough_moves(&cut, 0.0, 6.0, (0.0, 0.0));
    assert_eq!(verify(&moves, &mesh, grown, allowance, (0.0, 0.0)), None);
    assert_eq!(verify_ramp(&moves, &ramp, grown), None);
    let lowest = moves
        .iter()
        .filter(|m| m.kind == MoveKind::Feed)
        .filter_map(|m| m.z)
        .fold(f64::INFINITY, f64::min);
    assert!((lowest + 19.5).abs() < 1e-6, "{lowest}");
}

#[test]
fn the_check_finds_a_cut_into_the_model() {
    let mesh = dome();
    let into = vec![
        Move::rapid_z(5.0),
        Move::rapid_xy((0.0, 0.0)),
        Move::feed_z(-2.0, 500.0),
        Move::feed_xy((5.0, 0.0), 1500.0),
    ];
    let found = verify(&into, &mesh, Cutter::Ball { radius: 1.5 }, 0.0, (0.0, 0.0));
    assert!(found.is_some_and(|(depth, _)| depth > 1.0), "{found:?}");
}

#[test]
fn a_dome_is_roughed_keeping_the_allowance() {
    let mesh = dome();
    let area = Area::Loops(square(22.0));
    let allowance = 0.5;
    for tool in [Cutter::Flat { radius: 3.0 }, Cutter::Ball { radius: 3.0 }] {
        let grown = tool.grown(allowance).unwrap();
        let index = DropMesh::new(&mesh, 2.0 * grown.radius());
        let cut = ReliefCut {
            mesh: &index,
            cutter: grown,
            top: allowance,
            floor: -20.0 + allowance,
            area: &area,
            ramp: None,
            step: 2.4,
            angle: 0.0,
            zigzag: true,
            safe_z: 5.0,
            feed: 1500.0,
            plunge_feed: 500.0,
            shift: (0.0, 0.0),
            progress: None,
        };
        let (moves, _) = rough_moves(&cut, 0.0, 6.0, (0.0, 0.0));
        // The allowance is kept: the grown tool, raised by it, stays off the model.
        assert_eq!(
            verify(&moves, &mesh, grown, allowance, (0.0, 0.0)),
            None,
            "{tool:?}"
        );
        let zs: Vec<f64> = moves
            .iter()
            .filter(|m| m.kind == MoveKind::Feed)
            .filter_map(|m| m.z)
            .collect();
        let lowest = zs.iter().copied().fold(f64::INFINITY, f64::min);
        // Down to the floor plus the allowance round the dome, never below it.
        assert!((lowest - (-19.5)).abs() < 1e-6, "{tool:?}: {lowest}");
        // Each level is cut: some moves at -6, -12 and -18 (the flat floors round the
        // dome's sides).
        for level in [-6.0, -12.0, -18.0] {
            assert!(
                zs.iter().any(|z| (z - level).abs() < 1e-9),
                "{tool:?}: no moves at {level}"
            );
        }
        // Down level by level: the tool lifts between them, fewer times than lines.
        let lifts = moves
            .iter()
            .filter(|m| m.kind == MoveKind::Rapid && m.z == Some(5.0))
            .count();
        assert!(lifts < 40, "{tool:?}: {lifts} lifts");
    }
}

#[test]
fn levels_go_one_step_down_apart_to_the_bottom() {
    assert_eq!(levels(0.0, -10.0, 4.0), vec![-4.0, -8.0, -10.0]);
    assert_eq!(levels(0.0, -8.0, 4.0), vec![-4.0, -8.0]);
    assert_eq!(levels(2.0, -1.0, 5.0), vec![-1.0]);
    assert!(levels(0.0, 0.0, 4.0).is_empty());
    assert!(levels(0.0, 1.0, 4.0).is_empty());
}

#[test]
fn a_link_must_stay_inside_the_area() {
    // An L: the notch's corner at (5, 5).
    let l = vec![vec![
        (0.0, 0.0),
        (10.0, 0.0),
        (10.0, 5.0),
        (5.0, 5.0),
        (5.0, 10.0),
        (0.0, 10.0),
    ]];
    assert!(inside_loops_link(&l, (1.0, 1.0), (9.0, 4.0)));
    // Along an edge's ends.
    assert!(inside_loops_link(&l, (10.0, 1.0), (10.0, 4.0)));
    // Across the notch.
    assert!(!inside_loops_link(&l, (9.0, 4.0), (4.0, 9.0)));
    // Outside, between two ends on the boundary.
    assert!(!inside_loops_link(&l, (10.0, 5.0), (5.0, 10.0)));
}

#[test]
fn spans_follow_the_even_odd_rule() {
    let loops = vec![
        vec![(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)],
        vec![(3.0, 3.0), (7.0, 3.0), (7.0, 7.0), (3.0, 7.0)],
    ];
    assert_eq!(loop_spans(&loops, 5.0), vec![(0.0, 3.0), (7.0, 10.0)]);
    assert_eq!(loop_spans(&loops, 1.0), vec![(0.0, 10.0)]);
    assert!(loop_spans(&loops, 11.0).is_empty());
}

/// The lines done are reported as they finish, up to their share of the step, and a
/// stop skips the rest (docs/specs/background-generation.md §7).
#[test]
fn a_relief_reports_its_lines_and_stops_when_asked() {
    let mesh = dome();
    let index = DropMesh::new(&mesh, 3.0);
    let area = Area::Loops(square(22.0));
    let cancel = crate::cancel::Cancel::new();
    let cut = ReliefCut {
        mesh: &index,
        cutter: Cutter::Ball { radius: 1.5 },
        top: 0.0,
        floor: -20.0,
        area: &area,
        ramp: None,
        step: 1.0,
        angle: 0.0,
        zigzag: true,
        safe_z: 5.0,
        feed: 1500.0,
        plunge_feed: 500.0,
        shift: (0.0, 0.0),
        progress: Some(&cancel),
    };
    let (moves, _) = relief_moves(&cut, (0.0, 0.0));
    assert!(
        (cancel.progress() - LINES_SHARE).abs() < 1e-9,
        "{}",
        cancel.progress()
    );
    // The same lines as without a flag.
    let plain = ReliefCut {
        progress: None,
        ..cut
    };
    assert_eq!(relief_moves(&plain, (0.0, 0.0)).0, moves);
    // Stopped: no line is followed.
    let stopped = crate::cancel::Cancel::new();
    stopped.cancel();
    let cut = ReliefCut {
        progress: Some(&stopped),
        ..plain
    };
    let (moves, _) = relief_moves(&cut, (0.0, 0.0));
    assert!(
        moves
            .iter()
            .all(|m| m.kind == crate::model::MoveKind::Rapid),
        "{moves:?}"
    );
}
