// SPDX-License-Identifier: GPL-3.0-or-later
//! Holding tabs on profile rings. See docs/specs/tabs.md.

use std::f64::consts::TAU;

use chipload_core::Point;
use chipload_core::primitives::distance;
use chipload_core::tolerances::CHAIN_TOL_MM;
use chipload_i18n::{Digits, Message, Value};

use crate::ToolpathError;

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
        return Err(ToolpathError::Geometry(
            Message::new("toolpath-error-tabs-fit")
                .arg("count", count)
                .arg("width", Value::Length(width, Digits::Fixed(3)))
                .arg("length", Value::Length(length, Digits::Fixed(1)))
                .arg("fits", fits),
        ));
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

/// How far along `ring` (from ring[0], arc length) its point nearest to `p` lies, and how
/// far `p` is from it.
pub fn arc_position(ring: &[Point], p: Point) -> Option<(f64, f64)> {
    let (i, q, d) = crate::start::nearest_on_ring(ring, p)?;
    let before: f64 = (0..i).map(|k| distance(ring[k], ring[k + 1])).sum();
    Some((before + distance(ring[i], q), d))
}

/// The point `s` along `ring` (arc length from ring[0], wrapping round).
pub fn point_at_arc(ring: &[Point], s: f64) -> Point {
    let n = ring.len();
    let length = ring_length(ring);
    let mut left = if length > 0.0 {
        s.rem_euclid(length)
    } else {
        0.0
    };
    for i in 0..n {
        let (a, b) = (ring[i], ring[(i + 1) % n]);
        let seg = distance(a, b);
        if left <= seg && seg > 0.0 {
            let t = left / seg;
            return (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t);
        }
        left -= seg;
    }
    ring.first().copied().unwrap_or((0.0, 0.0))
}

/// Tabs placed by hand (docs/specs/tabs-by-hand.md §3): each of `points` (drawing
/// coordinates, already on or near this ring) is a tab's centre on the ring, with no
/// sliding. Refused, naming the tab, when two come closer than `width + 2·D` along the
/// ring, or a tab's gap reaches within D/2 of the ring's start (where every pass plunges).
pub fn tabs_at(
    ring: &[Point],
    points: &[Point],
    width: f64,
    tool_diameter: f64,
) -> Result<TabLayout, ToolpathError> {
    let length = ring_length(ring);
    let gap = width + tool_diameter;
    let mut centres: Vec<(f64, Point)> = points
        .iter()
        .filter_map(|&p| arc_position(ring, p).map(|(s, _)| (s, p)))
        .collect();
    centres.sort_by(|a, b| a.0.total_cmp(&b.0));
    let at = |p: Point| Value::Point(p.0, p.1, Digits::Fixed(3));
    for (k, &(s, p)) in centres.iter().enumerate() {
        if s - gap / 2.0 < tool_diameter / 2.0 - CHAIN_TOL_MM
            || s + gap / 2.0 > length - tool_diameter / 2.0 + CHAIN_TOL_MM
        {
            return Err(ToolpathError::Geometry(
                Message::new("toolpath-tab-on-start").arg("at", at(p)),
            ));
        }
        if let Some(&(next, q)) = centres.get(k + 1)
            && next - s < gap + tool_diameter - CHAIN_TOL_MM
        {
            return Err(ToolpathError::Geometry(
                Message::new("toolpath-tabs-too-close")
                    .arg("at", at(p))
                    .arg("other", at(q)),
            ));
        }
    }
    let (positions, turns) = vertex_turns(ring);
    let mut on_curves = 0;
    let spans: Vec<(f64, f64)> = centres
        .iter()
        .map(|&(s, _)| {
            let (s0, s1) = (s - gap / 2.0, s + gap / 2.0);
            let turning: f64 = positions
                .iter()
                .zip(&turns)
                .filter(|&(&v, _)| s0 < v && v < s1)
                .map(|(_, &t)| t)
                .sum();
            if turning >= TAB_MAX_TURN_RAD {
                on_curves += 1;
            }
            (s0, s1)
        })
        .collect();
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
