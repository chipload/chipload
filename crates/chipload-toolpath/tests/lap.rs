// SPDX-License-Identifier: GPL-3.0-or-later
//! Spiral profiles and ramped tabs (docs/specs/profile-spiral.md §10).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::f64::consts::{FRAC_PI_2, TAU};

use chipload_core::Point;
use chipload_job::Entry;
use chipload_toolpath::ToolpathError;
use chipload_toolpath::arcs::ArcTable;
use chipload_toolpath::lap::{RingCut, RingMoves, TabSurface, ring_moves};
use chipload_toolpath::model::{Move, MoveKind};
use chipload_toolpath::tabs::{TabLayout, place_tabs};
use proptest::prelude::*;

const D: f64 = 6.0;
const DEG: f64 = std::f64::consts::PI / 180.0;
const TOL: f64 = 1e-6;

/// A 200 × 100 rectangle, counter-clockwise from the origin: s is easy to read off.
fn rect() -> Vec<Point> {
    vec![(0.0, 0.0), (200.0, 0.0), (200.0, 100.0), (0.0, 100.0)]
}

fn cut<'a>(
    ring: &'a [Point],
    tabs: Option<TabSurface<'a>>,
    pass_z: &'a [f64],
    entry: Entry,
) -> RingCut<'a> {
    RingCut {
        ring,
        tabs,
        leads: None,
        pass_z,
        top_z: 0.0,
        safe_z: 5.0,
        feed: 1500.0,
        plunge_feed: 500.0,
        entry,
        tool_diameter: D,
    }
}

fn ramp(degrees: f64) -> Entry {
    Entry::Ramp {
        ramp_angle: degrees * DEG,
    }
}

fn run(cut: &RingCut) -> Result<RingMoves, ToolpathError> {
    ring_moves(cut, &ArcTable::default())
}

/// The position after each move, from (x0, y0, safe).
fn trace(moves: &[Move]) -> Vec<(f64, f64, f64, MoveKind)> {
    let (mut x, mut y, mut z) = (f64::NAN, f64::NAN, f64::NAN);
    moves
        .iter()
        .map(|m| {
            x = m.x.unwrap_or(x);
            y = m.y.unwrap_or(y);
            z = m.z.unwrap_or(z);
            (x, y, z, m.kind)
        })
        .collect()
}

/// Distances along `ring` of the cutting positions, following the tool forward from
/// ring[0]: (s since the start, z). Panics if a position is not on the ring ahead.
fn along(ring: &[Point], moves: &[Move]) -> Vec<(f64, f64)> {
    let n = ring.len();
    let seg = |i: usize| (ring[i % n], ring[(i + 1) % n]);
    let len = |i: usize| {
        let (a, b) = seg(i);
        (b.0 - a.0).hypot(b.1 - a.1)
    };
    let mut i = 0; // current segment, counting on round the laps
    let mut base = 0.0; // s at the start of segment i
    let mut out = Vec::new();
    let mut started = false;
    for &(x, y, z, kind) in &trace(moves) {
        if kind == MoveKind::Rapid {
            continue;
        }
        if !started {
            // The first feed is straight down at ring[0].
            assert!(
                (x - ring[0].0).hypot(y - ring[0].1) < TOL,
                "starts off the ring"
            );
            started = true;
            out.push((0.0, z));
            continue;
        }
        let mut found = None;
        for step in 0..=n {
            let k = i + step;
            let (a, b) = seg(k);
            let l = len(k);
            let t = ((x - a.0) * (b.0 - a.0) + (y - a.1) * (b.1 - a.1)) / (l * l);
            let off = ((x - a.0) - t * (b.0 - a.0)).hypot((y - a.1) - t * (b.1 - a.1));
            // Not behind the tool on the current segment.
            let behind = step == 0 && base + t * l < out.last().unwrap().0 - TOL;
            if off < TOL && (-TOL..=1.0 + TOL).contains(&t) && !behind {
                found = Some((k, t));
                break;
            }
        }
        let (k, t) = found.unwrap_or_else(|| panic!("({x}, {y}) is not on the ring ahead"));
        while i < k {
            base += len(i);
            i += 1;
        }
        out.push((base + t * len(k), z));
    }
    out
}

/// The tab surface, worked out independently: flat over each span, down the flanks.
fn surface(layout: &TabLayout, z_tab: f64, flank: f64, s: f64) -> f64 {
    surface_grown(layout, z_tab, flank, s, 0.0)
}

/// The same with every tab `grow` mm longer at each end (shorter, when negative).
fn surface_grown(layout: &TabLayout, z_tab: f64, flank: f64, s: f64, grow: f64) -> f64 {
    let s = s.rem_euclid(layout.length);
    let mut best = f64::NEG_INFINITY;
    for &(a, b) in &layout.spans {
        for shift in [-layout.length, 0.0, layout.length] {
            let (a, b) = (a + shift - grow, b + shift + grow);
            let d = if s < a {
                a - s
            } else if s > b {
                s - b
            } else {
                0.0
            };
            let h = if flank >= FRAC_PI_2 - 1e-9 {
                if d == 0.0 { z_tab } else { f64::NEG_INFINITY }
            } else {
                z_tab - d * flank.tan()
            };
            best = best.max(h);
        }
    }
    best
}

// -- the spiral

#[test]
fn a_spiral_goes_round_and_down_and_reaches_each_depth_at_the_start() {
    let ring = rect(); // 600 mm round
    let pass_z = [-4.0, -8.0, -12.0];
    let result = run(&cut(&ring, None, &pass_z, ramp(3.0))).unwrap();
    let path = along(&ring, &result.moves);
    // Only forward, and down until the level lap.
    for w in path.windows(2) {
        assert!(w[1].0 >= w[0].0 - TOL);
    }
    // At 3° one lap (600 mm) can go down 31 mm: one lap per pass. Each pass ends at the
    // start (a whole lap), exactly at its depth.
    for (k, &z) in pass_z.iter().enumerate() {
        let lap_end = 600.0 * (k + 1) as f64;
        let at = path
            .iter()
            .find(|p| (p.0 - lap_end).abs() < TOL)
            .unwrap_or_else(|| panic!("no point at {lap_end}"));
        assert_eq!(at.1, z, "pass {k}");
    }
    // The level lap: 600 mm at -12, then the rise-out over one tool diameter to Z0.
    let level: Vec<_> = path
        .iter()
        .filter(|p| p.0 >= 1800.0 - TOL && p.0 <= 2400.0 + TOL)
        .collect();
    assert!(level.iter().all(|p| p.1 == -12.0), "{level:?}");
    let last = path.last().unwrap();
    assert!((last.0 - (2400.0 + D)).abs() < TOL, "{last:?}");
    assert_eq!(last.1, 0.0);
    assert_eq!(*result.moves.last().unwrap(), Move::rapid_z(5.0));
    assert!((result.end.0 - D).abs() < TOL && result.end.1.abs() < TOL);
    // Never steeper than 3°, and no lap goes down more than one step.
    for w in path.windows(2) {
        let run = w[1].0 - w[0].0;
        if run > TOL && w[1].1 < w[0].1 {
            assert!((w[0].1 - w[1].1) / run <= (3.0 * DEG).tan() + 1e-9);
        }
    }
}

#[test]
fn a_short_ring_takes_more_laps_not_a_steeper_slope() {
    // A 40 mm square: one lap at 3° goes down 2.1 mm, so a 6 mm pass takes 3 laps.
    let ring = [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)];
    let result = run(&cut(&ring, None, &[-6.0], ramp(3.0))).unwrap();
    let path = along(&ring, &result.moves);
    let deepest_at = path.iter().find(|p| p.1 == -6.0).unwrap().0;
    assert!((deepest_at - 120.0).abs() < TOL, "{deepest_at}");
    for w in path.windows(2) {
        let run = w[1].0 - w[0].0;
        if run > TOL && w[1].1 < w[0].1 {
            assert!((w[0].1 - w[1].1) / run <= (3.0 * DEG).tan() + 1e-9);
        }
    }
}

#[test]
fn a_ring_far_too_short_for_the_angle_is_refused() {
    let ring = [(0.0, 0.0), (0.01, 0.0), (0.01, 0.01), (0.0, 0.01)];
    let err = run(&cut(&ring, None, &[-6.0], ramp(0.5))).unwrap_err();
    assert!(err.to_string().contains("too short to ramp"), "{err}");
}

#[test]
fn spiral_chords_on_arcs_keep_their_circle() {
    // A circle in 72 chords, all tagged: the spiral's chords stay tagged (helical arcs).
    let n = 72;
    let ring: Vec<Point> = (0..n)
        .map(|i| {
            let a = TAU * i as f64 / n as f64;
            (50.0 * a.cos(), 50.0 * a.sin())
        })
        .collect();
    let mut arcs = ArcTable::default();
    let circle = chipload_toolpath::arcs::Circle {
        center: (0.0, 0.0),
        radius: 50.0,
    };
    for i in 0..n {
        arcs.insert(ring[i], ring[(i + 1) % n], circle);
    }
    let cut = cut(&ring, None, &[-3.0], ramp(3.0));
    let result = ring_moves(&cut, &arcs).unwrap();
    let descending = result
        .moves
        .iter()
        .filter(|m| m.kind == MoveKind::Feed && m.x.is_some() && m.z.is_some())
        .collect::<Vec<_>>();
    assert!(descending.len() >= n);
    // All but the rise-out's partial chord.
    let tagged = descending.iter().filter(|m| m.circle.is_some()).count();
    assert!(tagged >= n, "{tagged}");
}

// -- ramped tabs

fn rect_tabs(ring: &[Point], count: usize) -> TabLayout {
    place_tabs(ring, count, 8.0, D).unwrap()
}

#[test]
fn the_level_lap_follows_trapezoid_tabs() {
    let ring = rect();
    let layout = rect_tabs(&ring, 4);
    let flank = 30.0 * DEG;
    let tabs = TabSurface {
        layout: &layout,
        z_tab: -9.0,
        flank_angle: flank,
    };
    let result = run(&cut(&ring, Some(tabs), &[-6.0, -12.0], ramp(30.0))).unwrap();
    let path = along(&ring, &result.moves);
    for &(s, z) in &path {
        assert!(z >= surface(&layout, -9.0, flank, s) - 1e-3, "{s}: {z}");
    }
    // The level lap (the third lap) is at -12 wherever the tab surface is lower, and at the
    // surface elsewhere: flat on each top, never deeper.
    let level: Vec<_> = path
        .iter()
        .filter(|p| p.0 >= 1200.0 && p.0 <= 1800.0)
        .collect();
    for p in &level {
        let expected = surface(&layout, -9.0, flank, p.0).max(-12.0);
        assert!((p.1 - expected).abs() < 1e-3, "{p:?} {expected}");
    }
    assert!(level.iter().any(|p| p.1 == -9.0));
    assert!(level.iter().any(|p| p.1 == -12.0));
}

#[test]
fn straight_tabs_lift_and_drop_at_the_plunge_feed() {
    let ring = rect();
    let layout = rect_tabs(&ring, 2);
    let tabs = TabSurface {
        layout: &layout,
        z_tab: -9.0,
        flank_angle: FRAC_PI_2,
    };
    let result = run(&cut(&ring, Some(tabs), &[-12.0], Entry::Plunge {})).unwrap();
    let vertical: Vec<&Move> = result
        .moves
        .iter()
        .filter(|m| m.kind == MoveKind::Feed && m.x.is_none())
        .collect();
    // The plunge, then up and down at each of the two tabs.
    assert_eq!(vertical.len(), 1 + 2 * 2);
    assert!(vertical.iter().all(|m| m.feed == Some(500.0)));
}

#[test]
fn a_lap_that_starts_on_a_flank_starts_at_the_flank() {
    // Shallow flanks reach back past the ring's start.
    let ring = rect();
    let layout = rect_tabs(&ring, 4);
    let flank = 3.0 * DEG;
    let tabs = TabSurface {
        layout: &layout,
        z_tab: -9.0,
        flank_angle: flank,
    };
    let start_height = surface(&layout, -9.0, flank, 0.0);
    assert!(start_height > -14.0, "the test needs a foot over the start");
    let result = run(&cut(&ring, Some(tabs), &[-14.0], Entry::Plunge {})).unwrap();
    let plunge = result
        .moves
        .iter()
        .find(|m| m.kind == MoveKind::Feed)
        .unwrap();
    assert!((plunge.z.unwrap() - start_height).abs() < 1e-9);
    let path = along(&ring, &result.moves);
    for &(s, z) in &path {
        assert!(z >= surface(&layout, -9.0, flank, s) - 1e-3, "{s}: {z}");
    }
}

#[test]
fn tabs_as_high_as_the_lap_are_crossed_level() {
    // A pass exactly at the tab top: nothing lifts.
    let ring = rect();
    let layout = rect_tabs(&ring, 4);
    let tabs = TabSurface {
        layout: &layout,
        z_tab: -6.0,
        flank_angle: 45.0 * DEG,
    };
    let result = run(&cut(&ring, Some(tabs), &[-6.0], Entry::Plunge {})).unwrap();
    let levels: Vec<f64> = trace(&result.moves)
        .iter()
        .filter(|p| p.3 == MoveKind::Feed)
        .map(|p| p.2)
        .collect();
    assert!(levels.iter().all(|&z| z == -6.0), "{levels:?}");
    // Only the ring's vertices: no extra points where no tab is in the way.
    assert_eq!(result.moves.len(), 2 + 1 + ring.len() + 1);
}

// -- properties

fn convex_ring() -> impl Strategy<Value = Vec<Point>> {
    (3usize..30).prop_flat_map(|n| {
        (
            prop::collection::btree_set(0u32..36_000, n),
            prop::collection::vec(20.0f64..80.0, n),
        )
            .prop_filter_map("needs n distinct angles", move |(angles, radii)| {
                (angles.len() == n).then(|| {
                    angles
                        .iter()
                        .zip(&radii)
                        .map(|(&a, &r)| {
                            let a = f64::from(a) / 36_000.0 * TAU;
                            let round = |v: f64| (v * 1e6).round() / 1e6;
                            (round(r * a.cos()), round(r * a.sin()))
                        })
                        .collect()
                })
            })
    })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(120))]

    #[test]
    fn the_tool_goes_forward_stays_above_the_tabs_and_cuts_one_step_at_a_time(
        ring in convex_ring(),
        count in 1usize..=6,
        width in 1.0f64..12.0,
        height in 0.5f64..5.0,
        flank_deg in 2.0f64..90.0,
        ramp_deg in 1.0f64..30.0,
        step in 1.0f64..6.0,
        passes in 1usize..5,
        ramped in any::<bool>(),
    ) {
        let Ok(layout) = place_tabs(&ring, count, width, D) else {
            return Ok(());
        };
        let bottom = -step * passes as f64;
        prop_assume!(height < -bottom);
        let pass_z: Vec<f64> = (1..=passes).map(|k| -step * k as f64).collect();
        let z_tab = bottom + height;
        let flank = flank_deg * DEG;
        let tabs = TabSurface { layout: &layout, z_tab, flank_angle: flank };
        let entry = if ramped { ramp(ramp_deg) } else { Entry::Plunge {} };
        let result = match run(&cut(&ring, Some(tabs), &pass_z, entry)) {
            Ok(result) => result,
            Err(err) => {
                prop_assert!(err.to_string().contains("too short to ramp"), "{}", err);
                return Ok(());
            }
        };
        let length = layout.length;
        // Tab ends are placed within the chaining tolerance (0.001 mm) along the ring; on a
        // steep flank that is a lot of height, so the slack is along the ring.
        const ALONG: f64 = 3e-3;
        let lowest = |s: f64| surface_grown(&layout, z_tab, flank, s, -ALONG) - 1e-3;
        let highest = |s: f64| surface_grown(&layout, z_tab, flank, s, ALONG).max(bottom) + 1e-3;
        // A spiral is one course; plunged passes each start again at the ring's start.
        let runs: Vec<Vec<Move>> = if ramped {
            vec![result.moves]
        } else {
            pass_z
                .iter()
                .map(|z| run(&cut(&ring, Some(tabs), std::slice::from_ref(z), entry)).unwrap().moves)
                .collect()
        };
        // What is cut so far, at 2000 points round the ring.
        const BINS: usize = 2000;
        let mut floor = vec![0.0f64; BINS];
        for moves in &runs {
            let path = along(&ring, moves);
            for w in path.windows(2) {
                // Forward only.
                prop_assert!(w[1].0 >= w[0].0 - TOL);
                // Never below the tab surface (at both ends; tops and feet are corners the
                // builder puts points on).
                for &(s, z) in w {
                    prop_assert!(z >= lowest(s), "{} at {}", z, s);
                }
                // At most one step of uncut material under the tool along the way.
                let (s0, s1) = (w[0].0, w[1].0);
                if s1 - s0 <= TOL {
                    continue;
                }
                let first = (s0 / length * BINS as f64).ceil() as i64;
                let last = (s1 / length * BINS as f64).floor() as i64;
                for k in first..=last {
                    let s = k as f64 * length / BINS as f64;
                    let z = w[0].1 + (w[1].1 - w[0].1) * (s - s0) / (s1 - s0);
                    let bin = k.rem_euclid(BINS as i64) as usize;
                    prop_assert!(floor[bin] - z <= step + 1e-3, "{} under the tool at {}", floor[bin] - z, s);
                    floor[bin] = floor[bin].min(z);
                }
            }
        }
        // In the end everything outside the tabs is cut to the bottom.
        for (bin, &z) in floor.iter().enumerate() {
            let s = bin as f64 * length / BINS as f64;
            prop_assert!(z <= highest(s), "{} left at {}, expected {}", z, s, highest(s));
        }
    }
}
