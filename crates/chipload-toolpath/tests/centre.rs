// SPDX-License-Identifier: GPL-3.0-or-later
//! Centre lines of strokes (docs/specs/image-arcs-centrelines.md §3, §8).

use std::f64::consts::TAU;

use chipload_core::Point;
use chipload_core::primitives::distance;
use chipload_toolpath::centre::centre_lines;

fn circle(r: f64, n: usize, ccw: bool) -> Vec<Point> {
    let mut ring: Vec<Point> = (0..n)
        .map(|k| {
            let t = TAU * k as f64 / n as f64;
            (r * t.cos(), r * t.sin())
        })
        .collect();
    if !ccw {
        ring.reverse();
    }
    ring
}

fn length(points: &[Point]) -> f64 {
    points.windows(2).map(|w| distance(w[0], w[1])).sum()
}

#[test]
fn a_square_ended_bar_is_one_path_without_corner_branches() {
    let bar = vec![(0.0, 0.0), (20.0, 0.0), (20.0, 2.0), (0.0, 2.0)];
    let found = centre_lines(&[bar], 0.05).unwrap();
    assert_eq!(found.paths.len(), 1, "{:?}", found.paths);
    let path = &found.paths[0];
    assert!(!path.closed);
    for &(_, y) in &path.points {
        assert!((y - 1.0).abs() < 1e-6, "{y}");
    }
    // It ends a half-width short of each square end.
    assert!(
        (length(&path.points) - 18.0).abs() < 0.05,
        "{}",
        length(&path.points)
    );
    let (low, high) = found.widths.unwrap();
    assert!((high - 2.0).abs() < 0.01, "{low} {high}");
    assert_eq!(found.dots, 0);
}

#[test]
fn a_t_is_three_paths_meeting_at_one_point() {
    let t = vec![
        (0.0, 15.0),
        (0.0, 13.0),
        (9.0, 13.0),
        (9.0, 0.0),
        (11.0, 0.0),
        (11.0, 13.0),
        (20.0, 13.0),
        (20.0, 15.0),
    ];
    let found = centre_lines(&[t], 0.05).unwrap();
    assert_eq!(found.paths.len(), 3, "{:?}", found.paths);
    let ends: Vec<Point> = found
        .paths
        .iter()
        .flat_map(|p| [p.points[0], p.points[p.points.len() - 1]])
        .collect();
    // One point is an end of all three.
    let shared = ends
        .iter()
        .filter(|&&e| ends.iter().filter(|&&f| distance(e, f) < 1e-9).count() == 3)
        .count();
    assert_eq!(shared, 3, "{ends:?}");
}

#[test]
fn an_o_is_one_closed_path_of_its_stroke_width() {
    let found = centre_lines(&[circle(10.0, 400, true), circle(8.0, 320, false)], 0.05).unwrap();
    assert_eq!(found.paths.len(), 1, "{:?}", found.paths.len());
    let path = &found.paths[0];
    assert!(path.closed);
    for &p in &path.points {
        assert!((distance(p, (0.0, 0.0)) - 9.0).abs() < 0.02, "{p:?}");
    }
    let (low, high) = found.widths.unwrap();
    assert!(low > 1.95 && high < 2.05, "{low} {high}");
}

#[test]
fn a_dot_is_counted_not_traced() {
    let found = centre_lines(&[circle(0.3, 40, true)], 0.05).unwrap();
    assert!(found.paths.is_empty(), "{:?}", found.paths);
    assert_eq!(found.dots, 1);
    assert_eq!(found.widths, None);
}
