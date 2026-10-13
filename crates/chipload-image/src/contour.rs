// SPDX-License-Identifier: GPL-3.0-or-later
//! Marching squares (spec §3.1–3.2): the line where the blurred picture crosses the level,
//! between pixel centres, as closed loops with black on their left.
//!
//! Each cell of four neighbouring samples is walked counter-clockwise. Along an edge the
//! walk either leaves black ("exit") or enters it ("enter"), and a cell's piece of edge line
//! runs from an exit to an enter, which keeps black on its left. A cell crossed four times
//! (a saddle) joins exits to the following enters when its centre is black, and to the
//! preceding ones when it's white. Every crossing is computed once per edge, in one fixed
//! direction, so the two cells sharing an edge meet in exactly the same point, and each
//! crossing starts one piece and ends one: the pieces chain into closed loops that never
//! cross.

use std::collections::BTreeMap;

use chipload_core::Point;
use chipload_core::tolerances::TRACE_EDGE_MARGIN;

use crate::field::Level;

/// Edge ids: the horizontal edge from sample (i, j) to (i + 1, j) is `2k`, the vertical one
/// from (i, j) to (i, j + 1) is `2k + 1`, with k = j · columns + i.
fn horizontal(level: &Level, i: usize, j: usize) -> u64 {
    2 * (j * level.columns + i) as u64
}

fn vertical(level: &Level, i: usize, j: usize) -> u64 {
    2 * (j * level.columns + i) as u64 + 1
}

/// The loops in mm, `pixel` mm apart, the picture's lower-left corner at (0, 0).
pub(crate) fn loops(level: &Level, pixel: f64) -> Vec<Vec<Point>> {
    let mut next: BTreeMap<u64, u64> = BTreeMap::new();
    for j in 0..level.rows - 1 {
        for i in 0..level.columns - 1 {
            cell(level, i, j, &mut next);
        }
    }
    let mut loops = Vec::new();
    while let Some((&start, _)) = next.first_key_value() {
        let mut ring = Vec::new();
        let mut edge = start;
        while let Some(to) = next.remove(&edge) {
            ring.push(crossing(level, edge, pixel));
            edge = to;
        }
        loops.push(ring);
    }
    loops
}

/// The pieces of edge line in the cell whose lower-left sample is (i, j), as exit → enter.
fn cell(level: &Level, i: usize, j: usize, next: &mut BTreeMap<u64, u64>) {
    // Corners counter-clockwise from the lower left, and the edge from each to the next.
    let corners = [(i, j), (i + 1, j), (i + 1, j + 1), (i, j + 1)];
    let edges = [
        horizontal(level, i, j),
        vertical(level, i + 1, j),
        horizontal(level, i, j + 1),
        vertical(level, i, j),
    ];
    let black = corners.map(|(x, y)| level.is_black(x, y));
    // (edge, true for an exit) in walking order.
    let crossings: Vec<(u64, bool)> = (0..4)
        .filter(|&k| black[k] != black[(k + 1) % 4])
        .map(|k| (edges[k], black[k]))
        .collect();
    match crossings.as_slice() {
        [] => {}
        [(a, a_exit), (b, _)] => {
            let (from, to) = if *a_exit { (*a, *b) } else { (*b, *a) };
            next.insert(from, to);
        }
        four => {
            let centre: f64 = corners.iter().map(|&(x, y)| level.at(x, y)).sum::<f64>() / 4.0;
            let step = if centre < level.level { 1 } else { 3 };
            for (k, &(edge, exit)) in four.iter().enumerate() {
                if exit {
                    next.insert(edge, four[(k + step) % 4].0);
                }
            }
        }
    }
}

/// Where the edge line crosses `edge`, in mm: interpolated between its two samples, kept a
/// hair away from both.
fn crossing(level: &Level, edge: u64, pixel: f64) -> Point {
    let k = (edge / 2) as usize;
    let (i, j) = (k % level.columns, k / level.columns);
    let (di, dj) = if edge.is_multiple_of(2) {
        (1, 0)
    } else {
        (0, 1)
    };
    let (v0, v1) = (level.at(i, j), level.at(i + di, j + dj));
    let t = ((level.level - v0) / (v1 - v0)).clamp(TRACE_EDGE_MARGIN, 1.0 - TRACE_EDGE_MARGIN);
    let x = i as f64 + t * di as f64;
    let y = j as f64 + t * dj as f64;
    // Sample (1, 1) is the centre of the lower-left pixel, half a pixel in from the corner.
    ((x - 0.5) * pixel, (y - 0.5) * pixel)
}
