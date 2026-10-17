// SPDX-License-Identifier: GPL-3.0-or-later
//! Merging overlapping outlines (docs/specs/more-fonts.md §4).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use chipload_core::Point;
use chipload_core::primitives::{Arc, Line, Primitive};
use chipload_toolpath::merge::merge_overlaps;

/// A rectangle as a loop of lines, counter-clockwise (or clockwise for a hole).
fn rect(x0: f64, y0: f64, x1: f64, y1: f64, ccw: bool) -> Vec<Primitive> {
    let mut points: Vec<Point> = vec![(x0, y0), (x1, y0), (x1, y1), (x0, y1)];
    if !ccw {
        points.reverse();
    }
    (0..4)
        .map(|i| {
            Primitive::Line(Line {
                start: points[i],
                end: points[(i + 1) % 4],
            })
        })
        .collect()
}

/// A circle as one arc.
fn circle(c: Point, r: f64, ccw: bool) -> Vec<Primitive> {
    vec![Primitive::Arc(Arc {
        center: c,
        radius: r,
        start_angle: 0.0,
        sweep: if ccw {
            std::f64::consts::TAU
        } else {
            -std::f64::consts::TAU
        },
    })]
}

/// Twice the signed area enclosed by a loop of lines and arcs.
fn area(prims: &[Primitive]) -> f64 {
    prims
        .iter()
        .map(|p| match p {
            Primitive::Line(l) => (l.start.0 * l.end.1 - l.end.0 * l.start.1) / 2.0,
            Primitive::Arc(a) => {
                let (s, e) = (p.start(), p.end());
                let chord = (s.0 * e.1 - e.0 * s.1) / 2.0;
                let segment = a.radius * a.radius / 2.0 * (a.sweep - a.sweep.sin());
                chord + segment
            }
        })
        .sum()
}

fn total(loops: &[(Vec<Primitive>, usize)]) -> f64 {
    loops.iter().map(|(l, _)| area(l)).sum()
}

#[test]
fn loops_that_overlap_nothing_come_back_unchanged() {
    // An "o" (outline and its hole) and a bar beside it.
    let loops = vec![
        circle((0.0, 0.0), 5.0, true),
        circle((0.0, 0.0), 3.0, false),
        rect(10.0, -5.0, 12.0, 5.0, true),
    ];
    let merged = merge_overlaps(&loops, &[0, 0, 1]).unwrap();
    let back: Vec<Vec<Primitive>> = merged.iter().map(|(l, _)| l.clone()).collect();
    assert_eq!(back, loops);
    assert_eq!(
        merged.iter().map(|(_, g)| *g).collect::<Vec<_>>(),
        [0, 0, 1]
    );
}

#[test]
fn two_overlapping_squares_become_one_outline() {
    let loops = vec![
        rect(0.0, 0.0, 10.0, 10.0, true),
        rect(5.0, 5.0, 15.0, 15.0, true),
    ];
    let merged = merge_overlaps(&loops, &[0, 1]).unwrap();
    assert_eq!(merged.len(), 1);
    assert!((total(&merged) - 175.0).abs() < 1e-6, "{}", total(&merged));
}

#[test]
fn a_stroke_over_a_hole_fills_only_what_it_covers() {
    // An "o" with a bar through its middle: the hole keeps its two uncovered parts.
    let loops = vec![
        circle((0.0, 0.0), 5.0, true),
        circle((0.0, 0.0), 3.0, false),
        rect(-6.0, -0.5, 6.0, 0.5, true),
    ];
    let merged = merge_overlaps(&loops, &[0, 0, 1]).unwrap();
    let outlines = merged.iter().filter(|(l, _)| area(l) > 0.0).count();
    let holes = merged.iter().filter(|(l, _)| area(l) < 0.0).count();
    assert_eq!((outlines, holes), (1, 2), "{merged:?}");
    // The disc, less what's left of the hole, plus the bar's ends outside the disc. A
    // strip |y| < h across a circle of radius r covers 2 (h √(r² − h²) + r² asin(h/r)).
    let pi = std::f64::consts::PI;
    let strip = |r: f64| 2.0 * (0.5 * (r * r - 0.25f64).sqrt() + r * r * (0.5 / r).asin());
    let expected = 25.0 * pi - (9.0 * pi - strip(3.0)) + (12.0 - strip(5.0));
    assert!(
        (total(&merged) - expected).abs() < 0.01,
        "{} vs {expected}",
        total(&merged)
    );
}

#[test]
fn letters_overlapping_another_letters_hole_keep_it_filled() {
    // A dot inside the hole of an "o", from another letter: it stays a separate piece.
    let loops = vec![
        circle((0.0, 0.0), 5.0, true),
        circle((0.0, 0.0), 3.0, false),
        circle((0.0, 0.0), 1.0, true),
    ];
    let merged = merge_overlaps(&loops, &[0, 0, 1]).unwrap();
    let pi = std::f64::consts::PI;
    assert!((total(&merged) - (25.0 - 9.0 + 1.0) * pi).abs() < 0.01);
    assert_eq!(merged.len(), 3);
}

#[test]
fn letters_joined_around_a_gap_leave_the_gap_open() {
    // Letters that overlap round a gap neither fills: the gap stays a hole.
    // A U drawn as three overlapping strokes, and a lid over its tips from another letter.
    let loops = vec![
        rect(0.0, 0.0, 6.0, 2.0, true),
        rect(0.0, 0.0, 2.0, 10.0, true),
        rect(4.0, 0.0, 6.0, 10.0, true),
        rect(-1.0, 9.0, 7.0, 11.0, true),
    ];
    let merged = merge_overlaps(&loops, &[0, 0, 0, 1]).unwrap();
    let holes = merged.iter().filter(|(l, _)| area(l) < 0.0).count();
    assert_eq!(holes, 1, "{merged:?}");
    // The U (12 + 20 + 20, less 8 of overlap) and the lid (16, less 4 over the U).
    assert!((total(&merged) - 56.0).abs() < 1e-6, "{}", total(&merged));
}

#[test]
fn touching_letters_stay_apart() {
    let loops = vec![
        rect(0.0, 0.0, 10.0, 10.0, true),
        rect(10.0, 10.0, 20.0, 20.0, true),
    ];
    let merged = merge_overlaps(&loops, &[0, 1]).unwrap();
    assert_eq!(merged.len(), 2);
}
