// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

fn knobs() -> JointShape {
    JointShape {
        joint: Joint::Knobs,
        size: 30.0,
        spacing: 120.0,
        round: 3.2,
    }
}

fn fingers() -> JointShape {
    JointShape {
        joint: Joint::Fingers,
        size: 15.0,
        spacing: 60.0,
        round: 3.2,
    }
}

fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<Primitive> {
    let pts = [(x0, y0), (x1, y0), (x1, y1), (x0, y1)];
    (0..4)
        .map(|i| {
            Primitive::Line(Line {
                start: pts[i],
                end: pts[(i + 1) % 4],
            })
        })
        .collect()
}

fn close(a: Point, b: Point) -> bool {
    math::hypot(a.0 - b.0, a.1 - b.1) < 1e-9
}

/// The direction a primitive leaves its start and arrives at its end.
fn directions(p: &Primitive) -> (Point, Point) {
    match p {
        Primitive::Line(l) => {
            let len = math::hypot(l.end.0 - l.start.0, l.end.1 - l.start.1);
            let d = ((l.end.0 - l.start.0) / len, (l.end.1 - l.start.1) / len);
            (d, d)
        }
        Primitive::Arc(a) => {
            let s = a.sweep.signum();
            let at = |angle: f64| (-math::sin(angle) * s, math::cos(angle) * s);
            (at(a.start_angle), at(a.start_angle + a.sweep))
        }
    }
}

fn connected_and_smooth(path: &[Primitive], smooth: bool) {
    for w in path.windows(2) {
        assert!(close(w[0].end(), w[1].start()), "{w:?}");
        if smooth {
            let (_, out) = directions(&w[0]);
            let (into, _) = directions(&w[1]);
            assert!(close(out, into), "a corner between {w:?}");
        }
    }
}

fn area(ring: &[Primitive]) -> f64 {
    let pts: Vec<Point> = ring
        .iter()
        .flat_map(|p| {
            let mut v = p.points();
            v.pop();
            v
        })
        .collect();
    chipload_core::primitives::signed_area(&pts)
}

#[test]
fn a_knob_is_smooth_and_as_wide_as_its_size() {
    for vertical in [false, true] {
        for side in [1.0, -1.0] {
            let seam = Seam {
                vertical,
                at: 10.0,
                features: vec![Feature::Knob { at: 100.0, side }],
            };
            let path = seam.portion(0.0, 200.0, &knobs());
            assert_eq!(path.len(), 5, "{path:?}");
            connected_and_smooth(&path, true);
            let start = seam.xy((0.0, 0.0));
            let end = seam.xy((200.0, 0.0));
            assert!(close(path[0].start(), start) && close(path[4].end(), end));
            // The head is the size across and stands out on its side.
            let Primitive::Arc(head) = path[2] else {
                panic!("{path:?}")
            };
            assert!((head.radius - 15.0).abs() < 1e-9);
            let out = if vertical {
                head.center.0 - 10.0
            } else {
                head.center.1 - 10.0
            };
            assert!(out * side > 0.0, "{head:?}");
            // More than half a circle: the head is wider than the neck and locks.
            assert!(head.sweep.abs() > PI);
        }
    }
}

#[test]
fn a_knob_sticks_out_about_one_size_and_its_neck_is_narrower_than_its_head() {
    let shape = knobs();
    let k = shape.knob();
    assert!(
        (shape.protrusion() - 35.7).abs() < 0.1,
        "{}",
        shape.protrusion()
    );
    assert!((k.reach - k.fillet) * 2.0 < 2.0 * k.head);
    assert!((k.reach - k.head).abs() < 1e-9);
}

#[test]
fn fingers_have_rounded_corners_and_stay_in_their_room() {
    let shape = fingers();
    let seam = Seam {
        vertical: false,
        at: 0.0,
        features: vec![Feature::Fingers {
            from: 50.0,
            teeth: 3,
            width: 30.0,
        }],
    };
    let path = seam.portion(0.0, 250.0, &shape);
    connected_and_smooth(&path, true);
    let arcs: Vec<&Arc> = path
        .iter()
        .filter_map(|p| match p {
            Primitive::Arc(a) => Some(a),
            Primitive::Line(_) => None,
        })
        .collect();
    // Four corners per tooth, every one the tool's round.
    assert_eq!(arcs.len(), 12);
    assert!(arcs.iter().all(|a| (a.radius - 3.2).abs() < 1e-9));
    for p in &path {
        let ((_, lo), (_, hi)) = p.bounds();
        assert!(lo > -1e-9 && hi < 15.0 + 1e-9, "{p:?}");
    }
}

#[test]
fn stretches_follow_the_material_and_its_holes() {
    let outer = rect(0.0, 0.0, 100.0, 50.0);
    let hole = rect(40.0, 10.0, 60.0, 40.0);
    let loops = vec![outer, hole];
    assert_eq!(
        stretches(&loops, true, 50.0),
        vec![(0.0, 10.0), (40.0, 50.0)]
    );
    assert_eq!(stretches(&loops, true, 20.0), vec![(0.0, 50.0)]);
    assert!(stretches(&loops, true, 150.0).is_empty());
    assert_eq!(
        stretches(&loops, false, 25.0),
        vec![(0.0, 40.0), (60.0, 100.0)]
    );
}

#[test]
fn knobs_alternate_and_keep_away_from_crossings_and_edges() {
    let shape = knobs();
    let feats = features(&[(0.0, 600.0)], &[300.0], &shape);
    let ats: Vec<(f64, f64)> = feats
        .iter()
        .map(|f| match *f {
            Feature::Knob { at, side } => (at, side),
            Feature::Fingers { .. } => panic!(),
        })
        .collect();
    // Each half: from 30 to 240, two knobs 120 apart, centred.
    assert_eq!(
        ats,
        vec![(75.0, 1.0), (195.0, -1.0), (405.0, 1.0), (525.0, -1.0)]
    );
    // A stretch too short for a knob stays straight.
    assert!(features(&[(0.0, 50.0)], &[], &shape).is_empty());
}

#[test]
fn neighbouring_cells_share_their_seam_and_tile_the_plane() {
    for shape in [knobs(), fingers()] {
        let loops = vec![rect(0.0, 0.0, 900.0, 500.0)];
        let lay = layout(
            &loops,
            ((0.0, 0.0), (900.0, 500.0)),
            (3, 2),
            (&[], &[20.0]),
            shape,
        );
        assert_eq!((lay.columns(), lay.rows()), (3, 2));
        assert!(lay.vertical.iter().all(|s| !s.features.is_empty()));
        let mut total = 0.0;
        for c in 0..3 {
            for r in 0..2 {
                let ring = lay.cell(c, r);
                connected_and_smooth(&ring, false);
                assert!(close(ring[0].start(), ring.last().unwrap().end()));
                let a = area(&ring);
                assert!(a > 0.0, "cell {c},{r} not counter-clockwise");
                total += a;
            }
        }
        let w = lay.xs[3] - lay.xs[0];
        let h = lay.ys[2] - lay.ys[0];
        // The joints move area between cells, never add or lose any.
        assert!((total - w * h).abs() < 1.0, "{total} vs {}", w * h);
        // Cell A1's right edge is cell A2's left edge walked the other way.
        let a1 = lay.vertical[0].portion(lay.ys[0], lay.ys[1], &shape);
        let a2 = lay.cell(1, 0);
        let left = reversed(a2[a2.len() - a1.len()..].to_vec());
        for (p, q) in left.iter().zip(&a1) {
            assert!(
                close(p.start(), q.start()) && close(p.end(), q.end()),
                "{p:?} {q:?}"
            );
        }
    }
}

fn square(x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<Point> {
    vec![(x0, y0), (x1, y0), (x1, y1), (x0, y1)]
}

#[test]
fn a_cut_across_the_reach_is_kept_inside_and_entered_from_above() {
    let reach = square(0.0, 0.0, 100.0, 100.0);
    let mut t = Trimmer::new(5.0, 1.0);
    let moves = [
        Move::rapid_z(5.0),
        Move::rapid_xy((-50.0, 50.0)),
        Move::rapid_z(1.0),
        Move::feed_z(-3.0, 300.0),
        Move::feed_xy((150.0, 50.0), 1500.0),
        Move::rapid_z(5.0),
    ];
    t.add(&moves, (0.0, 0.0, 5.0), Some(&reach), 300.0).unwrap();
    t.finish();
    // In at the reach's edge mid-cut, along to its far edge, up.
    let expect = [
        Move::rapid_z(5.0),
        Move::rapid_xy((0.0, 50.0)),
        Move::rapid_z(1.0),
        Move::feed_z(-3.0, 300.0),
        Move::feed_xyz((100.0, 50.0), -3.0, 1500.0),
        Move::rapid_z(5.0),
    ];
    assert_eq!(t.moves, expect);
    assert_eq!(t.split_entries, 1);
}

#[test]
fn a_cut_that_leaves_and_comes_back_is_entered_again() {
    // A U-shaped reach: the cut runs out of it and back in.
    let reach = vec![
        (0.0, 0.0),
        (100.0, 0.0),
        (100.0, 100.0),
        (60.0, 100.0),
        (60.0, 20.0),
        (40.0, 20.0),
        (40.0, 100.0),
        (0.0, 100.0),
    ];
    let mut t = Trimmer::new(5.0, 1.0);
    let moves = [
        Move::rapid_xy((10.0, 50.0)),
        Move::rapid_z(1.0),
        Move::feed_z(-2.0, 300.0),
        Move::feed_xy((90.0, 50.0), 1000.0),
    ];
    t.add(&moves, (0.0, 0.0, 5.0), Some(&reach), 300.0).unwrap();
    let feeds: Vec<&Move> = t
        .moves
        .iter()
        .filter(|m| m.kind == MoveKind::Feed)
        .collect();
    // The first plunge, to x=40, a second plunge at x=60, to the end.
    assert_eq!(feeds.len(), 4, "{:?}", t.moves);
    assert_eq!(feeds[1].x, Some(40.0));
    assert_eq!(t.split_entries, 1);
    let ups = t
        .moves
        .iter()
        .filter(|m| m.kind == MoveKind::Rapid && m.z == Some(5.0))
        .count();
    assert_eq!(ups, 2, "{:?}", t.moves);
}

#[test]
fn a_piece_is_its_cell_in_the_outline_less_half_the_clearance() {
    let shape = knobs();
    let outline = rect(0.0, 0.0, 900.0, 500.0);
    let lay = layout(
        std::slice::from_ref(&outline),
        ((0.0, 0.0), (900.0, 500.0)),
        (2, 1),
        (&[], &[]),
        shape,
    );
    let left = regions(std::slice::from_ref(&outline), &lay.cell(0, 0), 0.2).unwrap();
    let right = regions(std::slice::from_ref(&outline), &lay.cell(1, 0), 0.2).unwrap();
    assert_eq!((left.len(), right.len()), (1, 1));
    let (a, b) = (area(&left[0]), area(&right[0]));
    // Together the whole board less a 0.2 mm gap along the seam (500 long plus the
    // knobs' extra length).
    let gap = 900.0 * 500.0 - a - b;
    assert!(gap > 0.2 * 500.0 && gap < 0.2 * 800.0, "{gap}");
    // Every point of the left piece's seam edge is 0.2 mm from the right piece.
    let right_pl = pline_of(&right[0]);
    let right_edge = pline_segments(&right_pl);
    for p in &left[0] {
        let m = p.midpoint();
        // The seam edge: near x = 450, off the outline's top and bottom.
        if m.0 > 400.0 && m.1 > 1.0 && m.1 < 499.0 {
            let d = right_edge
                .iter()
                .map(|s| crate::offset::point_distance(s, m))
                .fold(f64::INFINITY, f64::min);
            assert!((d - 0.2).abs() < 0.01, "{d} at {m:?}");
        }
    }
}

#[test]
fn names_go_by_row_letter_and_column_number() {
    assert_eq!(Layout::name(0, 0), "A1");
    assert_eq!(Layout::name(2, 1), "B3");
    assert_eq!(Layout::name(0, 26), "AA1");
}

#[test]
fn a_joint_too_small_for_the_tool_is_named() {
    let mut shape = knobs();
    assert!(shape.problem().is_none());
    shape.round = 6.5;
    assert!(shape.problem().is_some());
    assert!((shape.least_size() - 32.5).abs() < 1e-9);
    let mut f = fingers();
    assert!(f.problem().is_none());
    f.size = 6.0;
    assert!(f.problem().is_some());
}

#[test]
fn the_automatic_grid_is_the_fewest_that_fit() {
    let shape = knobs();
    // 900 wide on 600 boards: two columns of 450 plus a knob and two 10 mm margins.
    let grid = auto_grid(
        ((0.0, 0.0), (900.0, 500.0)),
        (600.0, 600.0),
        10.0,
        (&[], &[]),
        &shape,
    );
    assert_eq!(grid, Some((2, 1)));
    let grid = auto_grid(
        ((0.0, 0.0), (900.0, 500.0)),
        (400.0, 400.0),
        10.0,
        (&[], &[]),
        &shape,
    );
    assert_eq!(grid, Some((3, 2)));
    assert_eq!(
        auto_grid(
            ((0.0, 0.0), (900.0, 500.0)),
            (50.0, 50.0),
            10.0,
            (&[], &[]),
            &shape
        ),
        None
    );
}
