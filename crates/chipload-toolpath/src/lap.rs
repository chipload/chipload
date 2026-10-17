// SPDX-License-Identifier: GPL-3.0-or-later
//! The moves around one profile ring (docs/specs/profile-spiral.md).
//!
//! The tool goes forward along the ring, lap after lap. Its depth is a straight line along
//! the distance travelled on each stretch (a level lap, a spiral down, the rise-out), but
//! never below the surface of a tab: `z(s) = max(lap(s), tab(s))`. Tabs are flat on top,
//! with flanks that slope down at the flank angle, or that are vertical (straight tabs).
//!
//! - A profile spirals down: each pass ramps at the angle then runs level, the ramped
//!   stretch is cut level at the end and the tool rises out along the ring.
//! - A closed follow path with a plunge entry cuts each pass as a level lap from a plunge
//!   at the ring's start.

use chipload_core::Point;
use chipload_core::primitives::distance;
use chipload_core::tolerances::{CHAIN_TOL_MM, DEPTH_TOL_MM};
use chipload_job::flank_is_straight;

use crate::ToolpathError;
use crate::arcs::ArcTable;
use crate::model::Move;
use crate::ramp::MAX_RAMP_LEGS;
use crate::start::onto;
use crate::tabs::TabLayout;

/// Tabs on a ring: where they are, the Z of their top and the slope of their flanks.
#[derive(Debug, Clone, Copy)]
pub struct TabSurface<'a> {
    pub layout: &'a TabLayout,
    pub z_tab: f64,
    /// Radians; 90° (within the angle tolerance) is a straight tab.
    pub flank_angle: f64,
}

impl TabSurface<'_> {
    /// tan(flank angle); None for a straight (vertical) flank.
    fn slope(&self) -> Option<f64> {
        (!flank_is_straight(self.flank_angle)).then(|| self.flank_angle.tan())
    }
}

/// Everything needed to cut one ring.
#[derive(Debug, Clone, Copy)]
pub struct RingCut<'a> {
    /// The tool-centre ring, starting at ring[0], in cutting order.
    pub ring: &'a [Point],
    pub tabs: Option<TabSurface<'a>>,
    /// The Z of each depth pass, going down.
    pub pass_z: &'a [f64],
    /// The Z of the start depth, cut already (docs/specs/ramp-entry.md).
    pub top_z: f64,
    pub safe_z: f64,
    pub feed: f64,
    pub plunge_feed: f64,
    /// The spiral's angle; None plunges each pass (a closed follow path).
    pub ramp_angle: Option<f64>,
    pub tool_diameter: f64,
}

/// The moves, and where the tool is when they end (retracted above it).
#[derive(Debug, Clone, PartialEq)]
pub struct RingMoves {
    pub moves: Vec<Move>,
    pub end: Point,
}

/// The moves for one ring: retract, travel, cut, retract.
pub fn ring_moves(cut: &RingCut, arcs: &ArcTable) -> Result<RingMoves, ToolpathError> {
    let Some(&start) = cut.ring.first() else {
        return Ok(RingMoves {
            moves: Vec::new(),
            end: (0.0, 0.0),
        });
    };
    let along = Along::new(cut.ring);
    if along.length <= CHAIN_TOL_MM {
        return Err(ToolpathError::Geometry(
            "a profile ring has no length".to_string(),
        ));
    }
    match cut.ramp_angle {
        Some(ramp_angle) => spiral(cut, &along, ramp_angle, arcs),
        None => passes(cut, &along, start, arcs),
    }
}

/// Spiral down from the start depth: each pass ramps at the angle from the ring's start,
/// then runs level for the rest of its lap; at the end the ramped stretch is cut level and
/// the tool rises out along the ring (docs/specs/profile-ramp-level.md §2).
fn spiral(
    cut: &RingCut,
    along: &Along,
    ramp_angle: f64,
    arcs: &ArcTable,
) -> Result<RingMoves, ToolpathError> {
    let length = along.length;
    let mut legs = Vec::new();
    let mut lap: i64 = 0;
    let mut z = cut.top_z;
    // How far into its lap the last pass's ramp reached: the stretch left to level.
    let mut ramped = 0.0;
    for &pass in cut.pass_z {
        let drop = z - pass;
        if drop <= DEPTH_TOL_MM {
            continue;
        }
        let ramp = ramp_length(drop, ramp_angle, length)?;
        let to = ahead((lap, 0.0), ramp, length);
        legs.push(Leg {
            from: (lap, 0.0),
            to,
            z0: z,
            z1: pass,
        });
        if to.1 < length {
            legs.push(Leg {
                from: to,
                to: (to.0, length),
                z0: pass,
                z1: pass,
            });
        }
        ramped = to.1;
        lap = to.0 + 1;
        z = pass;
    }
    // The ramped stretch is cut level, then the tool rises out over one tool diameter of
    // path, which is cut to the end depth already.
    let mut at = (lap, 0.0);
    if ramped > 0.0 {
        let to = ahead(at, ramped, length);
        legs.push(Leg {
            from: at,
            to,
            z0: z,
            z1: z,
        });
        at = lap_start(to, length);
    }
    let rise = cut.tool_diameter.min(length);
    legs.push(Leg {
        from: at,
        to: ahead(at, rise, length),
        z0: z,
        z1: cut.top_z,
    });
    let samples = course(cut.tabs, along, &legs)?;
    let mut moves = vec![
        Move::rapid_z(cut.safe_z),
        Move::rapid_xy(cut.ring[0]),
        Move::feed_z(cut.top_z, cut.plunge_feed),
    ];
    emit(&mut moves, Feeds::of(cut), along, &samples, arcs);
    moves.push(Move::rapid_z(cut.safe_z));
    let end = samples
        .last()
        .map_or(cut.ring[0], |s| along.point(cut.ring, s));
    Ok(RingMoves { moves, end })
}

/// A plunge entry (a closed follow path): each pass a level lap from a plunge at the
/// ring's start.
fn passes(
    cut: &RingCut,
    along: &Along,
    start: Point,
    arcs: &ArcTable,
) -> Result<RingMoves, ToolpathError> {
    let mut moves = vec![Move::rapid_z(cut.safe_z), Move::rapid_xy(start)];
    for &z in cut.pass_z {
        let samples = course(cut.tabs, along, &[Leg::laps(0, 1, along.length, z, z)])?;
        // The lap's start may be on a tab's flank.
        let z_start = samples.first().map_or(z, |s| s.z);
        moves.push(Move::feed_z(z_start, cut.plunge_feed));
        emit(&mut moves, Feeds::of(cut), along, &samples, arcs);
    }
    moves.push(Move::rapid_z(cut.safe_z));
    Ok(RingMoves { moves, end: start })
}

/// A finishing or skin lap at the wall, after the roughing (docs/specs/finish-and-skin.md).
#[derive(Debug, Clone, Copy)]
pub struct WallLap<'a> {
    /// The tool-centre ring at the wall, in cutting order.
    pub ring: &'a [Point],
    pub tabs: Option<TabSurface<'a>>,
    /// The roughing rings: their slot is cleared to `slot_z`, so the tool goes down there.
    /// The ring itself when there was no finishing stock.
    pub slot: &'a [&'a [Point]],
    pub slot_z: f64,
    /// The lap's depth: `slot_z` for a finishing lap, below it for a skin lap.
    pub z: f64,
    pub safe_z: f64,
    pub feed: f64,
    pub plunge_feed: f64,
    /// The profile's ramp angle: a skin lap goes down its thickness at it.
    pub ramp_angle: f64,
    pub tool_diameter: f64,
}

/// The moves for one wall lap: down in the roughing slot, across onto the ring, one lap
/// (a skin lap first goes down its thickness, ramping forward at the angle, then level
/// round, levels the ramped stretch and rises out), back across into the slot, retract (docs/specs/finish-and-skin.md §3,
/// docs/specs/profile-ramp-level.md §2). With tabs the lap starts in the middle of the
/// longest gap between them.
pub fn wall_lap(lap: &WallLap, arcs: &ArcTable) -> Result<RingMoves, ToolpathError> {
    let along = Along::new(lap.ring);
    let length = along.length;
    if lap.ring.is_empty() || length <= CHAIN_TOL_MM {
        return Err(ToolpathError::Geometry(
            "a profile ring has no length".to_string(),
        ));
    }
    let s0 = lap
        .tabs
        .map_or(0.0, |tabs| widest_gap_middle(tabs.layout, &along));
    let drop = lap.slot_z - lap.z;
    let mut legs = Vec::new();
    let mut at: At = (0, s0);
    let ramp = if drop > DEPTH_TOL_MM {
        Some(ramp_length(drop, lap.ramp_angle, length)?)
    } else {
        None
    };
    if let Some(ramp) = ramp {
        let to = ahead(at, ramp, length);
        legs.push(Leg {
            from: at,
            to,
            z0: lap.slot_z,
            z1: lap.z,
        });
        at = lap_start(to, length);
    }
    // Level round to the start: the rest of the ramp's lap, or the whole lap.
    let round = match ramp {
        Some(_) => (s0 - at.1).rem_euclid(length),
        None => length,
    };
    if round > CHAIN_TOL_MM {
        let to = ahead(at, round, length);
        legs.push(Leg {
            from: at,
            to,
            z0: lap.z,
            z1: lap.z,
        });
        at = lap_start(to, length);
    }
    if let Some(ramp) = ramp {
        // The ramped stretch is cut level, then the tool rises out over one tool diameter
        // of path, cut to the lap's depth already.
        let to = ahead(at, ahead((0, 0.0), ramp, length).1, length);
        legs.push(Leg {
            from: at,
            to,
            z0: lap.z,
            z1: lap.z,
        });
        at = lap_start(to, length);
        let rise = lap.tool_diameter.min(length);
        legs.push(Leg {
            from: at,
            to: ahead(at, rise, length),
            z0: lap.z,
            z1: lap.slot_z,
        });
    }
    let samples = course(lap.tabs, &along, &legs)?;
    let (Some(first), Some(last)) = (samples.first(), samples.last()) else {
        return Err(ToolpathError::Geometry(
            "a wall lap has no course".to_string(),
        ));
    };
    let start = along.point(lap.ring, first);
    let finish = along.point(lap.ring, last);
    let into = nearest_on_rings(lap.slot, start).map_or(start, |(p, _)| p);
    let out = nearest_on_rings(lap.slot, finish).map_or(finish, |(p, _)| p);
    // Down in the slot, to the lap's start height if a tab holds it higher.
    let down = lap.slot_z.max(first.z);
    let mut moves = vec![
        Move::rapid_z(lap.safe_z),
        Move::rapid_xy(into),
        Move::feed_z(down, lap.plunge_feed),
    ];
    if distance(into, start) > CHAIN_TOL_MM {
        moves.push(Move::feed_xy(start, lap.feed));
    }
    let feeds = Feeds {
        ring: lap.ring,
        feed: lap.feed,
        plunge_feed: lap.plunge_feed,
    };
    emit(&mut moves, feeds, &along, &samples, arcs);
    if distance(finish, out) > CHAIN_TOL_MM {
        moves.push(Move::feed_xy(out, lap.feed));
    }
    moves.push(Move::rapid_z(lap.safe_z));
    Ok(RingMoves { moves, end: out })
}

/// The path a ramp of `drop` at `ramp_angle` takes; refused when it would take more than
/// `MAX_RAMP_LEGS` laps of a ring `length` long.
fn ramp_length(drop: f64, ramp_angle: f64, length: f64) -> Result<f64, ToolpathError> {
    let ramp = drop / ramp_angle.tan();
    if ramp / length > MAX_RAMP_LEGS as f64 {
        return Err(ToolpathError::Geometry(format!(
            "The ring is too short to ramp down {drop:.3} mm at {:.1}° ({length:.3} mm \
             around); use a steeper ramp angle or a straight plunge with a \
             centre-cutting tool",
            ramp_angle.to_degrees()
        )));
    }
    Ok(ramp)
}

/// `dist` mm on from `at` (a position within its lap): the lap it ends in and how far
/// into it. Landing at a lap's start is given as the end of the lap before, `(lap,
/// length)`, so a leg ends on the lap it ran on; within the chaining tolerance of a lap's
/// end counts as the end.
fn ahead(at: At, dist: f64, length: f64) -> At {
    let s = at.1 + dist;
    // Small whole number: a ramp is at most MAX_RAMP_LEGS laps.
    let mut laps = (s / length).floor() as i64;
    let mut local = s - laps as f64 * length;
    if local <= CHAIN_TOL_MM && laps >= 1 {
        laps -= 1;
        local = length;
    } else if local >= length - CHAIN_TOL_MM {
        local = length;
    }
    (at.0 + laps, local)
}

/// The position as a leg's start: a lap's end is the next lap's start.
fn lap_start(at: At, length: f64) -> At {
    if at.1 >= length { (at.0 + 1, 0.0) } else { at }
}

/// The middle of the longest stretch between two tabs, measured along the ring; on a ring
/// vertex if one is within the chaining tolerance.
fn widest_gap_middle(layout: &TabLayout, along: &Along) -> f64 {
    let spans = &layout.spans;
    let length = along.length;
    let mut best: Option<(f64, f64)> = None; // (gap, middle)
    for k in 0..spans.len() {
        let end = spans[k].1;
        let next = spans[(k + 1) % spans.len()].0;
        let gap = (next - end).rem_euclid(length);
        if best.is_none_or(|(g, _)| gap > g) {
            best = Some((gap, (end + gap / 2.0).rem_euclid(length)));
        }
    }
    let Some((_, middle)) = best else {
        return 0.0;
    };
    along
        .cum
        .iter()
        .copied()
        .take(along.cum.len() - 1)
        .find(|&c| (c - middle).abs() <= CHAIN_TOL_MM)
        .unwrap_or(middle)
}

/// The point on the rings nearest to `p`, and its distance; None without rings.
pub fn nearest_on_rings(rings: &[&[Point]], p: Point) -> Option<(Point, f64)> {
    let mut best: Option<(Point, f64)> = None;
    for ring in rings {
        let n = ring.len();
        for i in 0..n {
            let q = nearest_on_segment(ring[i], ring[(i + 1) % n], p);
            let d = distance(p, q);
            if best.is_none_or(|(_, bd)| d < bd) {
                best = Some((q, d));
            }
        }
    }
    best
}

/// The point on segment `a`-`b` nearest to `p`.
fn nearest_on_segment(a: Point, b: Point, p: Point) -> Point {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let len2 = dx * dx + dy * dy;
    if len2 <= 0.0 {
        return a;
    }
    let t = (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / len2).clamp(0.0, 1.0);
    (a.0 + dx * t, a.1 + dy * t)
}

/// The farthest any point of `ring` is from the `slot` rings: at every vertex and every
/// `step` mm along each segment (docs/specs/finish-and-skin.md §6).
pub fn farthest_from(ring: &[Point], slot: &[&[Point]], step: f64) -> f64 {
    let n = ring.len();
    let mut farthest: f64 = 0.0;
    for i in 0..n {
        let (a, b) = (ring[i], ring[(i + 1) % n]);
        let seg = distance(a, b);
        // Small non-negative whole number: a segment is short against the step.
        let count = (seg / step.max(CHAIN_TOL_MM)).ceil().max(1.0) as usize;
        for k in 0..count {
            let t = k as f64 / count as f64;
            let p = (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t);
            let d = nearest_on_rings(slot, p).map_or(f64::INFINITY, |(_, d)| d);
            farthest = farthest.max(d);
        }
    }
    farthest
}

/// The tabs of a ring at the wall carried over to a roughing ring beside it: each tab end
/// moved to the nearest point of `onto`. Tabs nearer to another of `others` stay off it.
/// Spans that come out overlapping are joined; one across `onto`'s start is split there.
/// Fails if a tab would come out reversed or longer than half the ring.
pub fn carry_tabs(
    ring: &[Point],
    layout: &TabLayout,
    onto: &[Point],
    others: &[&[Point]],
) -> Result<TabLayout, ToolpathError> {
    let from = Along::new(ring);
    let to = Along::new(onto);
    let at = |s: f64| {
        let i = from
            .cum
            .partition_point(|&c| c <= s)
            .saturating_sub(1)
            .min(ring.len() - 1);
        let (a, b) = (ring[i], ring[(i + 1) % ring.len()]);
        let seg = distance(a, b);
        let t = if seg > 0.0 {
            ((s - from.cum[i]) / seg).clamp(0.0, 1.0)
        } else {
            0.0
        };
        (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t)
    };
    let mut spans: Vec<(f64, f64)> = Vec::new();
    for &(a, b) in &layout.spans {
        let middle = at((a + b) / 2.0);
        let mine = nearest_on_rings(&[onto], middle).map_or(f64::INFINITY, |(_, d)| d);
        let elsewhere = nearest_on_rings(others, middle).map_or(f64::INFINITY, |(_, d)| d);
        if elsewhere < mine {
            continue;
        }
        let (pa, pb) = (position_on(&to, onto, at(a)), position_on(&to, onto, at(b)));
        let forward = (pb - pa).rem_euclid(to.length);
        if forward > to.length / 2.0 {
            return Err(ToolpathError::Geometry(format!(
                "could not carry a tab ({a:.3} to {b:.3} mm along the ring) onto the \
                 roughing ring"
            )));
        }
        if pa + forward <= to.length {
            spans.push((pa, pa + forward));
        } else {
            spans.push((pa, to.length));
            spans.push((0.0, pa + forward - to.length));
        }
    }
    spans.sort_by(|x, y| x.0.total_cmp(&y.0));
    let mut joined: Vec<(f64, f64)> = Vec::new();
    for span in spans {
        match joined.last_mut() {
            Some(last) if span.0 <= last.1 => last.1 = last.1.max(span.1),
            _ => joined.push(span),
        }
    }
    Ok(TabLayout {
        spans: joined,
        length: to.length,
        on_curves: 0,
    })
}

/// How far along `ring` (from its start) its point nearest to `p` is.
fn position_on(along: &Along, ring: &[Point], p: Point) -> f64 {
    let n = ring.len();
    let mut best = (f64::INFINITY, 0.0);
    for i in 0..n {
        let (a, b) = (ring[i], ring[(i + 1) % n]);
        let q = nearest_on_segment(a, b, p);
        let d = distance(p, q);
        if d < best.0 {
            best = (d, along.cum[i] + distance(a, q));
        }
    }
    best.1
}

/// The ring measured along its length: `cum[i]` is the distance from ring[0] to ring[i],
/// `cum[n]` the whole length (summed in the same order as `split_ring`).
struct Along {
    cum: Vec<f64>,
    length: f64,
}

impl Along {
    fn new(ring: &[Point]) -> Self {
        let n = ring.len();
        let mut cum = Vec::with_capacity(n + 1);
        let mut travelled = 0.0;
        for i in 0..n {
            cum.push(travelled);
            travelled += distance(ring[i], ring[(i + 1) % n]);
        }
        cum.push(travelled);
        Along {
            cum,
            length: travelled,
        }
    }

    /// The point of a sample: a vertex exactly, else interpolated on its segment as
    /// `split_ring` does.
    fn point(&self, ring: &[Point], sample: &Sample) -> Point {
        self.locate(ring, sample).0
    }

    /// The point of a sample and the chord it lies on (a vertex: the chord leaving it;
    /// the end of a lap: chord 0, as it is ring[0]).
    fn locate(&self, ring: &[Point], sample: &Sample) -> (Point, usize) {
        let n = ring.len();
        if let Some(i) = sample.vertex {
            return (ring[i % n], i % n);
        }
        let s = sample.local;
        // The segment holding s: the last vertex at or before it.
        let i = self
            .cum
            .partition_point(|&c| c <= s)
            .saturating_sub(1)
            .min(n - 1);
        let (a, b) = (ring[i], ring[(i + 1) % n]);
        let seg = distance(a, b);
        if seg <= 0.0 {
            return (a, i);
        }
        let t = (s - self.cum[i]) / seg;
        ((a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t), i)
    }
}

/// A position on the ring: (lap, distance from ring[0] within that lap, 0..=length).
type At = (i64, f64);

/// A stretch of the course whose depth goes evenly from `z0` to `z1` (before the tabs).
#[derive(Debug, Clone, Copy)]
struct Leg {
    from: At,
    /// Must lie after `from`.
    to: At,
    z0: f64,
    z1: f64,
}

impl Leg {
    /// `count` whole laps of a ring `length` long, from the start of lap `first`.
    fn laps(first: i64, count: i64, length: f64, z0: f64, z1: f64) -> Leg {
        Leg {
            from: (first, 0.0),
            to: (first + count - 1, length),
            z0,
            z1,
        }
    }
}

/// A point the tool passes: where, and at what Z.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Sample {
    lap: i64,
    local: f64,
    /// The ring vertex it is on, if any (`n` is ring[0] at the end of a lap).
    vertex: Option<usize>,
    z: f64,
}

/// A piece of the course on which the lap's depth and the tab surface are both straight.
#[derive(Debug, Clone, Copy)]
struct Piece {
    p: Sample,
    q: Sample,
    /// The tab is above the lap here: the tool follows the tab.
    binding: bool,
    /// The first piece of a leg: the depth may bend where it starts.
    starts_leg: bool,
}

/// The tool's path along `legs`, following the tab surface where it is higher, verified.
fn course(
    tabs: Option<TabSurface>,
    along: &Along,
    legs: &[Leg],
) -> Result<Vec<Sample>, ToolpathError> {
    let surface = tabs.map(|tabs| Surface::new(tabs, along));
    let mut pieces: Vec<Piece> = Vec::new();
    for leg in legs {
        let first_piece = pieces.len();
        let to_local = leg.to.1;
        let total = (leg.to.0 - leg.from.0) as f64 * along.length + to_local - leg.from.1;
        let depth = |lap: i64, local: f64| {
            if lap == leg.to.0 && local >= to_local {
                return leg.z1; // exactly the leg's end depth
            }
            let gone = (lap - leg.from.0) as f64 * along.length + local - leg.from.1;
            leg.z0 + (leg.z1 - leg.z0) * gone / total
        };
        for lap in leg.from.0..=leg.to.0 {
            let lo = if lap == leg.from.0 { leg.from.1 } else { 0.0 };
            let hi = if lap == leg.to.0 {
                to_local
            } else {
                along.length
            };
            let stops = breakpoints(surface.as_ref(), along, lo, hi);
            for w in stops.windows(2) {
                lap_pieces(surface.as_ref(), lap, w[0], w[1], &depth, &mut pieces);
            }
        }
        if let Some(piece) = pieces.get_mut(first_piece) {
            piece.starts_leg = true;
        }
    }
    let samples = stitch(&pieces);
    if let Some(tabs) = tabs {
        verify(&samples, tabs, along)?;
    }
    Ok(samples)
}

/// The tabs as the course follows them. A tab end within the chaining tolerance of a ring
/// vertex is moved onto it (the tab grows or shrinks by at most that), so every corner of
/// the surface is a point the tool passes.
struct Surface {
    spans: Vec<(f64, f64)>,
    z_tab: f64,
    /// tan(flank angle); None for straight tabs.
    slope: Option<f64>,
    length: f64,
}

impl Surface {
    fn new(tabs: TabSurface, along: &Along) -> Surface {
        let snap = |c: f64| {
            along
                .cum
                .iter()
                .copied()
                .find(|&v| (v - c).abs() <= CHAIN_TOL_MM)
                .unwrap_or(c)
        };
        Surface {
            spans: tabs
                .layout
                .spans
                .iter()
                .map(|&(a, b)| (snap(a), snap(b)))
                .collect(),
            z_tab: tabs.z_tab,
            slope: tabs.slope(),
            length: along.length,
        }
    }

    /// Sloped tabs: their top over a tab, else down the nearest flank.
    fn sloped(&self, slope: f64, s: f64) -> f64 {
        let mut nearest = f64::INFINITY;
        for &(a, b) in &self.spans {
            let d = if a <= s && s <= b {
                0.0
            } else {
                (a - s)
                    .rem_euclid(self.length)
                    .min((s - b).rem_euclid(self.length))
            };
            nearest = nearest.min(d);
        }
        self.z_tab - slope * nearest
    }
}

/// Where the depth or the tab surface may bend between `lo` and `hi` in one lap: the ring's
/// vertices, the tab ends and (sloped flanks) the middle between neighbouring tabs.
fn breakpoints(
    surface: Option<&Surface>,
    along: &Along,
    lo: f64,
    hi: f64,
) -> Vec<(f64, Option<usize>)> {
    let n = along.cum.len() - 1;
    let mut stops: Vec<(f64, Option<usize>)> =
        vec![(lo, vertex_at(along, lo)), (hi, vertex_at(along, hi))];
    for (i, &c) in along.cum.iter().enumerate().take(n + 1) {
        if lo < c && c < hi {
            stops.push((c, Some(i)));
        }
    }
    if let Some(surface) = surface {
        let spans = &surface.spans;
        let mut extra: Vec<f64> = spans.iter().flat_map(|&(a, b)| [a, b]).collect();
        if surface.slope.is_some() && !spans.is_empty() {
            for w in spans.windows(2) {
                extra.push((w[0].1 + w[1].0) / 2.0);
            }
            let (last, first) = (spans[spans.len() - 1].1, spans[0].0);
            let middle = (last + first + along.length) / 2.0;
            extra.push(if middle > along.length {
                middle - along.length
            } else {
                middle
            });
        }
        for c in extra {
            let near = stops.iter().any(|&(s, _)| (s - c).abs() <= CHAIN_TOL_MM);
            if lo < c && c < hi && !near {
                stops.push((c, None));
            }
        }
    }
    stops.sort_by(|a, b| a.0.total_cmp(&b.0));
    stops
}

/// The vertex at `s` exactly (the ends of a lap are ring[0]).
fn vertex_at(along: &Along, s: f64) -> Option<usize> {
    along.cum.iter().position(|&c| c == s)
}

/// The pieces between two breakpoints of one lap: one, or two where the lap's depth
/// crosses a sloped flank.
fn lap_pieces(
    surface: Option<&Surface>,
    lap: i64,
    (p, p_vertex): (f64, Option<usize>),
    (q, q_vertex): (f64, Option<usize>),
    depth: &impl Fn(i64, f64) -> f64,
    pieces: &mut Vec<Piece>,
) {
    let sample = |local: f64, vertex: Option<usize>, z: f64| Sample {
        lap,
        local,
        vertex,
        z,
    };
    let (lp, lq) = (depth(lap, p), depth(lap, q));
    let Some(surface) = surface else {
        pieces.push(Piece {
            p: sample(p, p_vertex, lp),
            q: sample(q, q_vertex, lq),
            binding: false,
            starts_leg: false,
        });
        return;
    };
    let (tp, tq) = match surface.slope {
        Some(slope) => (surface.sloped(slope, p), surface.sloped(slope, q)),
        None => {
            let middle = (p + q) / 2.0;
            let inside = surface.spans.iter().any(|&(a, b)| a < middle && middle < b);
            let top = if inside {
                surface.z_tab
            } else {
                f64::NEG_INFINITY
            };
            (top, top)
        }
    };
    let (dp, dq) = (tp - lp, tq - lq);
    // The lap crosses the flank inside the piece: split there. A crossing within the
    // chaining tolerance of an end is at that end: the flank is a step there (a steep flank
    // can't be stretched over the whole piece).
    if dp.is_finite() && dq.is_finite() && (dp > 0.0) != (dq > 0.0) && dp != dq {
        let mut r = p + (q - p) * dp / (dp - dq);
        let mut r_vertex = None;
        if r - p <= CHAIN_TOL_MM {
            (r, r_vertex) = (p, p_vertex);
        } else if q - r <= CHAIN_TOL_MM {
            (r, r_vertex) = (q, q_vertex);
        }
        let lr = depth(lap, r);
        let first = dp > DEPTH_TOL_MM;
        let second = dq > DEPTH_TOL_MM;
        pieces.push(Piece {
            p: sample(p, p_vertex, if first { tp.max(lp) } else { lp }),
            q: sample(r, r_vertex, lr),
            binding: first,
            starts_leg: false,
        });
        pieces.push(Piece {
            p: sample(r, r_vertex, lr),
            q: sample(q, q_vertex, if second { tq.max(lq) } else { lq }),
            binding: second,
            starts_leg: false,
        });
        return;
    }
    let binding = dp > DEPTH_TOL_MM || dq > DEPTH_TOL_MM;
    let (zp, zq) = if binding {
        (tp.max(lp), tq.max(lq))
    } else {
        (lp, lq)
    };
    pieces.push(Piece {
        p: sample(p, p_vertex, zp),
        q: sample(q, q_vertex, zq),
        binding,
        starts_leg: false,
    });
}

/// The pieces as one list of samples. A point between two pieces where the lap's depth
/// runs straight on (no tab, no vertex, not where a leg starts) is left out; where the height jumps (a straight
/// tab) both heights are kept, one above the other.
fn stitch(pieces: &[Piece]) -> Vec<Sample> {
    let mut samples: Vec<Sample> = Vec::new();
    let Some(first) = pieces.first() else {
        return samples;
    };
    samples.push(first.p);
    for (k, piece) in pieces.iter().enumerate() {
        let Some(next) = pieces.get(k + 1) else {
            samples.push(piece.q);
            break;
        };
        let end = piece.q;
        let new_leg_or_lap = next.starts_leg || next.p.lap != end.lap || next.p.local != end.local;
        if end.z != next.p.z {
            samples.push(end);
            samples.push(next.p);
        } else if end.vertex.is_some() || new_leg_or_lap || piece.binding || next.binding {
            samples.push(end);
        }
    }
    samples
}

/// No sample, and no tab end passed between two samples, lies below the tab surface (with
/// the tab ends moved in by the chaining tolerance); the course only goes forward.
fn verify(samples: &[Sample], tabs: TabSurface, along: &Along) -> Result<(), ToolpathError> {
    let length = along.length;
    let shrunk: Vec<(f64, f64)> = tabs
        .layout
        .spans
        .iter()
        .map(|&(a, b)| (a + CHAIN_TOL_MM, b - CHAIN_TOL_MM))
        .collect();
    let lowest = |s: f64| match tabs.slope() {
        Some(slope) => {
            let mut nearest = f64::INFINITY;
            for &(a, b) in &shrunk {
                let d = if a <= s && s <= b {
                    0.0
                } else {
                    (a - s).rem_euclid(length).min((s - b).rem_euclid(length))
                };
                nearest = nearest.min(d);
            }
            tabs.z_tab - slope * nearest
        }
        None => {
            if shrunk.iter().any(|&(a, b)| a < s && s < b) {
                tabs.z_tab
            } else {
                f64::NEG_INFINITY
            }
        }
    };
    let fail = |what: String| {
        Err(ToolpathError::Geometry(format!(
            "Tab toolpath failed verification: {what}"
        )))
    };
    for (k, sample) in samples.iter().enumerate() {
        if sample.z < lowest(sample.local) - DEPTH_TOL_MM {
            return fail(format!(
                "Z{:.3} mm at {:.3} mm along the ring is below the tab",
                sample.z, sample.local
            ));
        }
        let Some(next) = samples.get(k + 1) else {
            break;
        };
        let (s0, s1) = (
            sample.lap as f64 * length + sample.local,
            next.lap as f64 * length + next.local,
        );
        if s1 < s0 - CHAIN_TOL_MM {
            return fail(format!(
                "the tool goes back along the ring at {:.3} mm",
                sample.local
            ));
        }
        if next.lap != sample.lap || s1 - s0 <= 0.0 {
            continue;
        }
        for &(a, b) in &shrunk {
            for end in [a, b] {
                if sample.local < end && end < next.local {
                    let t = (end - sample.local) / (next.local - sample.local);
                    let z = sample.z + (next.z - sample.z) * t;
                    if z < tabs.z_tab - DEPTH_TOL_MM {
                        return fail(format!(
                            "Z{z:.3} mm at the tab end {end:.3} mm along the ring is below the \
                             tab top Z{:.3} mm",
                            tabs.z_tab
                        ));
                    }
                }
            }
        }
    }
    Ok(())
}

/// What `emit` needs besides the course.
#[derive(Debug, Clone, Copy)]
struct Feeds<'a> {
    ring: &'a [Point],
    feed: f64,
    plunge_feed: f64,
}

impl<'a> Feeds<'a> {
    fn of(cut: &RingCut<'a>) -> Self {
        Feeds {
            ring: cut.ring,
            feed: cut.feed,
            plunge_feed: cut.plunge_feed,
        }
    }
}

/// Moves along the samples, from the first one (where the tool already is). Straight up or
/// down at the plunge feed; along the ring at the cutting feed, level or not. A move along
/// a chord of an arc keeps its circle, so it can be written as an arc (helical when it
/// descends): a point where a stretch or a tab splits such a chord is moved onto the arc,
/// so both parts are chords of it.
fn emit(moves: &mut Vec<Move>, cut: Feeds, along: &Along, samples: &[Sample], arcs: &ArcTable) {
    let Some(first) = samples.first() else {
        return;
    };
    let n = cut.ring.len();
    // The circle of chord `i`, if it lies on an arc.
    let circle_of = |i: usize| arcs.get(cut.ring[i % n], cut.ring[(i + 1) % n]);
    let (mut here, mut chord) = along.locate(cut.ring, first);
    if first.vertex.is_none()
        && let Some(circle) = circle_of(chord)
    {
        here = onto(circle, here);
    }
    let mut z = first.z;
    for sample in &samples[1..] {
        let (mut point, next_chord) = along.locate(cut.ring, sample);
        // The chord this move runs along: a vertex is the end of the chord before it.
        let moved_on = if sample.vertex.is_some() {
            (next_chord + n - 1) % n
        } else {
            next_chord
        };
        let circle = circle_of(moved_on).filter(|_| moved_on == chord);
        if let Some(circle) = circle
            && sample.vertex.is_none()
        {
            point = onto(circle, point);
        }
        chord = next_chord;
        if point == here {
            if sample.z != z {
                moves.push(Move::feed_z(sample.z, cut.plunge_feed));
                z = sample.z;
            }
            continue;
        }
        let mut m = if sample.z == z {
            Move::feed_xy(point, cut.feed)
        } else {
            Move::feed_xyz(point, sample.z, cut.feed)
        };
        m.circle = circle;
        moves.push(m);
        here = point;
        z = sample.z;
    }
}
