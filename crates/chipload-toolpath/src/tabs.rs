// SPDX-License-Identifier: GPL-3.0-or-later
//! Holding tabs on profile rings. See docs/specs/tabs.md.

use std::f64::consts::TAU;

use chipload_core::Point;
use chipload_core::primitives::distance;
use chipload_core::tolerances::{CHAIN_TOL_MM, DEPTH_TOL_MM};

use crate::ToolpathError;
use crate::model::{Move, MoveKind};

/// A tab gap turning more than this (15°) in total is "on a corner".
pub const TAB_MAX_TURN_RAD: f64 = 15.0 * std::f64::consts::PI / 180.0;
/// Resolution when sliding a tab off a corner.
pub const TAB_SLIDE_STEP_MM: f64 = 0.25;

#[derive(Debug, Clone, PartialEq)]
pub struct TabLayout {
    /// (start, end) arc length of each lift on the ring.
    pub spans: Vec<(f64, f64)>,
    /// Ring length.
    pub length: f64,
    /// Tabs that could not be moved off a corner or curve.
    pub on_curves: usize,
}

pub fn ring_length(ring: &[Point]) -> f64 {
    let n = ring.len();
    (0..n).map(|i| distance(ring[i], ring[(i + 1) % n])).sum()
}

/// Equal spacing, then each tab slides (within its slot, at most a quarter of the spacing)
/// to the nearest straight-enough stretch. Fails if the tabs don't fit.
pub fn place_tabs(
    ring: &[Point],
    count: usize,
    width: f64,
    tool_diameter: f64,
) -> Result<TabLayout, ToolpathError> {
    let length = ring_length(ring);
    let gap = width + tool_diameter;
    let spacing = length / count as f64;
    if count == 0 || spacing - gap < tool_diameter {
        // Small non-negative whole number of tabs.
        let fits = (length / (gap + tool_diameter)).floor() as u64;
        let most = if fits > 0 {
            format!("at most {fits} fit")
        } else {
            "not even one fits".to_string()
        };
        return Err(ToolpathError::Geometry(format!(
            "{count} tabs of {width:.3} mm don't fit on a {length:.1} mm path ({most})"
        )));
    }
    let (positions, turns) = vertex_turns(ring);
    let mut spans = Vec::with_capacity(count);
    let mut on_curves = 0;
    for i in 0..count {
        let i = i as f64;
        let nominal = (i + 0.5) * spacing;
        let lowest = (i * spacing + tool_diameter / 2.0 + gap / 2.0).max(nominal - spacing / 4.0);
        let highest =
            ((i + 1.0) * spacing - tool_diameter / 2.0 - gap / 2.0).min(nominal + spacing / 4.0);
        let centre = straightest_centre(nominal, lowest, highest, gap, &positions, &turns)
            .unwrap_or_else(|| {
                on_curves += 1;
                nominal
            });
        spans.push((centre - gap / 2.0, centre + gap / 2.0));
    }
    Ok(TabLayout {
        spans,
        length,
        on_curves,
    })
}

/// The closed ring from ring[0] back to ring[0], with points inserted at every span
/// boundary. Each entry is (end point of a segment, whether that segment lies in a tab).
pub fn split_ring(ring: &[Point], spans: &[(f64, f64)]) -> Vec<(Point, bool)> {
    let mut cuts: Vec<f64> = spans.iter().flat_map(|&(a, b)| [a, b]).collect();
    cuts.sort_by(f64::total_cmp);
    let mut result = Vec::new();
    let mut travelled = 0.0;
    let n = ring.len();
    for i in 0..n {
        let (a, b) = (ring[i], ring[(i + 1) % n]);
        let seg = distance(a, b);
        let mut stops: Vec<f64> = cuts
            .iter()
            .filter(|&&c| travelled + CHAIN_TOL_MM < c && c < travelled + seg - CHAIN_TOL_MM)
            .map(|&c| (c - travelled) / seg)
            .collect();
        stops.push(1.0);
        let mut previous = 0.0;
        for t in stops {
            let mid = travelled + seg * (previous + t) / 2.0;
            let point = if t == 1.0 {
                b
            } else {
                (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t)
            };
            let in_tab = spans.iter().any(|&(s0, s1)| s0 < mid && mid < s1);
            result.push((point, in_tab));
            previous = t;
        }
        travelled += seg;
    }
    result
}

/// Like `contour_moves`, but passes below the tab top lift over every tab: up at plunge
/// feed, across at feed, back down at plunge feed.
pub fn tabbed_contour_moves(
    ring: &[Point],
    layout: &TabLayout,
    pass_z: &[f64],
    z_tab: f64,
    safe_z: f64,
    feed: f64,
    plunge_feed: f64,
) -> Result<Vec<Move>, ToolpathError> {
    let Some(&start) = ring.first() else {
        return Ok(Vec::new());
    };
    let path = split_ring(ring, &layout.spans);
    let mut moves = vec![Move::rapid_z(safe_z), Move::rapid_xy(start)];
    for &z in pass_z {
        moves.push(Move::feed_z(z, plunge_feed));
        if z >= z_tab - DEPTH_TOL_MM {
            moves.extend(ring[1..].iter().map(|&p| Move::feed_xy(p, feed)));
            moves.push(Move::feed_xy(start, feed));
            continue;
        }
        let mut lifted = false;
        for &(point, in_tab) in &path {
            if in_tab != lifted {
                moves.push(Move::feed_z(if in_tab { z_tab } else { z }, plunge_feed));
                lifted = in_tab;
            }
            moves.push(Move::feed_xy(point, feed));
        }
        if lifted {
            moves.push(Move::feed_z(z, plunge_feed));
        }
    }
    moves.push(Move::rapid_z(safe_z));
    verify(&moves, layout, start, z_tab)?;
    Ok(moves)
}

/// The tool-path stretch of each tab, for display.
pub fn tab_polylines(ring: &[Point], layout: &TabLayout) -> Vec<Vec<Point>> {
    let mut lines = Vec::new();
    let mut current: Vec<Point> = Vec::new();
    let Some(&first) = ring.first() else {
        return lines;
    };
    let mut previous = first;
    for (point, in_tab) in split_ring(ring, &layout.spans) {
        if in_tab {
            if current.is_empty() {
                current.push(previous);
            }
            current.push(point);
        } else if !current.is_empty() {
            lines.push(std::mem::take(&mut current));
        }
        previous = point;
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

/// IEEE remainder of x / TAU: the angle in [-π, π].
fn wrap_angle(x: f64) -> f64 {
    x - TAU * (x / TAU).round_ties_even()
}

/// Arc-length position and absolute turning angle of every vertex.
fn vertex_turns(ring: &[Point]) -> (Vec<f64>, Vec<f64>) {
    let n = ring.len();
    let mut positions = Vec::with_capacity(n);
    let mut turns = Vec::with_capacity(n);
    let mut travelled = 0.0;
    for i in 0..n {
        let (prev, here, next) = (ring[(i + n - 1) % n], ring[i], ring[(i + 1) % n]);
        let a_in = (here.1 - prev.1).atan2(here.0 - prev.0);
        let a_out = (next.1 - here.1).atan2(next.0 - here.0);
        positions.push(travelled);
        turns.push(wrap_angle(a_out - a_in).abs());
        travelled += distance(here, next);
    }
    (positions, turns)
}

/// Closest centre to `nominal` (forward first on ties) whose gap turns < TAB_MAX_TURN_RAD.
fn straightest_centre(
    nominal: f64,
    lowest: f64,
    highest: f64,
    gap: f64,
    positions: &[f64],
    turns: &[f64],
) -> Option<f64> {
    // Small non-negative whole number of slide steps.
    let steps = ((highest - nominal).max(nominal - lowest) / TAB_SLIDE_STEP_MM) as u64;
    for k in 0..=steps {
        let offset = k as f64 * TAB_SLIDE_STEP_MM;
        for centre in [nominal + offset, nominal - offset] {
            if !(lowest <= centre && centre <= highest) {
                continue;
            }
            let (s0, s1) = (centre - gap / 2.0, centre + gap / 2.0);
            let turning: f64 = positions
                .iter()
                .zip(turns)
                .filter(|&(&s, _)| s0 < s && s < s1)
                .map(|(_, &t)| t)
                .sum();
            if turning < TAB_MAX_TURN_RAD {
                return Some(centre);
            }
            if k == 0 {
                break; // nominal: don't test the same centre twice
            }
        }
    }
    None
}

/// Post-condition: on every pass below the tab top, the length cut low equals the ring
/// length minus the tab gaps, and the lifted length equals the tab gaps.
fn verify(
    moves: &[Move],
    layout: &TabLayout,
    start: Point,
    z_tab: f64,
) -> Result<(), ToolpathError> {
    let gaps: f64 = layout.spans.iter().map(|(a, b)| b - a).sum();
    let (mut x, mut y) = start;
    let mut z: Option<f64> = None;
    let mut pass_z: Option<f64> = None;
    // (pass z, length cut low, length lifted), in pass order.
    let mut passes: Vec<(f64, f64, f64)> = Vec::new();
    for m in moves {
        let (nx, ny) = (m.x.unwrap_or(x), m.y.unwrap_or(y));
        if let Some(mz) = m.z {
            z = Some(mz);
            if m.kind == MoveKind::Feed && mz < z_tab - DEPTH_TOL_MM {
                pass_z = Some(mz);
            }
        }
        let step = distance((x, y), (nx, ny));
        if let (MoveKind::Feed, Some(depth), Some(z)) = (m.kind, pass_z, z)
            && step > 0.0
        {
            let index = match passes.iter().position(|p| p.0 == depth) {
                Some(i) => i,
                None => {
                    passes.push((depth, 0.0, 0.0));
                    passes.len() - 1
                }
            };
            if z >= z_tab - DEPTH_TOL_MM {
                passes[index].2 += step;
            } else {
                passes[index].1 += step;
            }
        }
        (x, y) = (nx, ny);
    }
    let slack = CHAIN_TOL_MM * (layout.spans.len() + 1) as f64;
    let expected = layout.length - gaps;
    for (depth, low, lifted) in passes {
        if low > 0.0 && ((low - expected).abs() > slack || (lifted - gaps).abs() > slack) {
            return Err(ToolpathError::Geometry(format!(
                "Tab toolpath failed verification at Z{depth:.3}: cut {low:.3} mm low, \
                 expected {expected:.3}"
            )));
        }
    }
    Ok(())
}
