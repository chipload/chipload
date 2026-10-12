// SPDX-License-Identifier: GPL-3.0-or-later
//! The moves around one profile ring (docs/specs/profile-spiral.md).
//!
//! The tool goes forward along the ring, lap after lap. Its depth is a straight line along
//! the distance travelled on each stretch (a level lap, a spiral down, the rise-out), but
//! never below the surface of a tab: `z(s) = max(lap(s), tab(s))`. Tabs are flat on top,
//! with flanks that slope down at the flank angle, or that are vertical (straight tabs).
//!
//! - A ramp entry spirals down in whole laps, then cuts one level lap at the last depth and
//!   rises out along the ring.
//! - A plunge cuts each pass as a level lap from a plunge at the ring's start, as before.

use chipload_core::Point;
use chipload_core::primitives::distance;
use chipload_core::tolerances::{CHAIN_TOL_MM, DEPTH_TOL_MM};
use chipload_job::{Entry, flank_is_straight};

use crate::ToolpathError;
use crate::arcs::ArcTable;
use crate::lead::Leads;
use crate::model::Move;
use crate::ramp::MAX_RAMP_LEGS;
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
    pub leads: Option<&'a Leads>,
    /// The Z of each depth pass, going down.
    pub pass_z: &'a [f64],
    /// The Z of the start depth, cut already (docs/specs/ramp-entry.md).
    pub top_z: f64,
    pub safe_z: f64,
    pub feed: f64,
    pub plunge_feed: f64,
    pub entry: Entry,
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
    match cut.entry.ramp_angle() {
        Some(ramp_angle) => spiral(cut, &along, ramp_angle, arcs),
        None => passes(cut, &along, start, arcs),
    }
}

/// Spiral down from the start depth in whole laps per pass, one level lap at the last
/// depth, then rise out along the ring (docs/specs/profile-spiral.md §3).
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
    for &pass in cut.pass_z {
        let drop = z - pass;
        if drop <= DEPTH_TOL_MM {
            continue;
        }
        let needed = ((drop / ramp_angle.tan() - CHAIN_TOL_MM) / length)
            .ceil()
            .max(1.0);
        if needed > MAX_RAMP_LEGS as f64 {
            return Err(ToolpathError::Geometry(format!(
                "The ring is too short to ramp down {drop:.3} mm at {:.1}° ({length:.3} mm \
                 around); use a steeper ramp angle or a straight plunge with a \
                 centre-cutting tool",
                ramp_angle.to_degrees()
            )));
        }
        // Small positive whole number (checked above).
        let laps = needed as i64;
        legs.push(Leg::laps(lap, laps, length, z, pass));
        lap += laps;
        z = pass;
    }
    // The level lap cuts away the spiral's slope.
    legs.push(Leg::laps(lap, 1, length, z, z));
    lap += 1;
    // Rise out over one tool diameter of path, which is cut at every depth already.
    let rise = cut.tool_diameter.min(length);
    legs.push(Leg {
        from: (lap, 0.0),
        to: (lap, rise),
        z0: z,
        z1: cut.top_z,
    });
    let samples = course(cut, along, &legs)?;
    let first = cut.leads.and_then(|l| l.lead_in.first().copied());
    let mut moves = vec![
        Move::rapid_z(cut.safe_z),
        Move::rapid_xy(first.unwrap_or(cut.ring[0])),
        Move::feed_z(cut.top_z, cut.plunge_feed),
    ];
    if let Some(leads) = cut.leads {
        moves.extend(
            leads.lead_in[1..]
                .iter()
                .map(|&p| Move::feed_xy(p, cut.feed)),
        );
    }
    emit(&mut moves, cut, along, &samples, arcs);
    moves.push(Move::rapid_z(cut.safe_z));
    let end = samples
        .last()
        .map_or(cut.ring[0], |s| along.point(cut.ring, s));
    Ok(RingMoves { moves, end })
}

/// A plunge entry: each pass a level lap from a plunge at the ring's start, with leads on
/// every pass when there are any (docs/specs/helix-and-leads.md part B).
fn passes(
    cut: &RingCut,
    along: &Along,
    start: Point,
    arcs: &ArcTable,
) -> Result<RingMoves, ToolpathError> {
    let mut moves = Vec::new();
    if cut.leads.is_none() {
        moves.extend([Move::rapid_z(cut.safe_z), Move::rapid_xy(start)]);
    }
    for &z in cut.pass_z {
        let samples = course(cut, along, &[Leg::laps(0, 1, along.length, z, z)])?;
        // The lap's start may be on a tab's flank.
        let z_start = samples.first().map_or(z, |s| s.z);
        match cut.leads {
            Some(leads) => {
                let from = leads.lead_in.first().copied().unwrap_or(start);
                moves.extend([
                    Move::rapid_z(cut.safe_z),
                    Move::rapid_xy(from),
                    Move::feed_z(z_start, cut.plunge_feed),
                ]);
                moves.extend(
                    leads.lead_in[1..]
                        .iter()
                        .map(|&p| Move::feed_xy(p, cut.feed)),
                );
                emit(&mut moves, cut, along, &samples, arcs);
                moves.extend(
                    leads.lead_out[1..]
                        .iter()
                        .map(|&p| Move::feed_xy(p, cut.feed)),
                );
                moves.push(Move::rapid_z(cut.safe_z));
            }
            None => {
                moves.push(Move::feed_z(z_start, cut.plunge_feed));
                emit(&mut moves, cut, along, &samples, arcs);
            }
        }
    }
    if cut.leads.is_none() {
        moves.push(Move::rapid_z(cut.safe_z));
    }
    let end = match cut.leads {
        Some(leads) => leads.lead_out.last().copied().unwrap_or(start),
        None => start,
    };
    Ok(RingMoves { moves, end })
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
        let n = ring.len();
        if let Some(i) = sample.vertex {
            return ring[i % n];
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
            return a;
        }
        let t = (s - self.cum[i]) / seg;
        (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t)
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
}

/// The tool's path along `legs`, following the tab surface where it is higher, verified.
fn course(cut: &RingCut, along: &Along, legs: &[Leg]) -> Result<Vec<Sample>, ToolpathError> {
    let surface = cut.tabs.map(|tabs| Surface::new(tabs, along));
    let mut pieces: Vec<Piece> = Vec::new();
    for leg in legs {
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
    }
    let samples = stitch(&pieces);
    if let Some(tabs) = cut.tabs {
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
        });
        pieces.push(Piece {
            p: sample(r, r_vertex, lr),
            q: sample(q, q_vertex, if second { tq.max(lq) } else { lq }),
            binding: second,
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
    });
}

/// The pieces as one list of samples. A point between two pieces where the lap's depth
/// runs straight on (no tab, no vertex) is left out; where the height jumps (a straight
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
        let new_leg_or_lap = next.p.lap != end.lap || next.p.local != end.local;
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
                "Z{:.3} at {:.3} mm along the ring is below the tab",
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
                            "Z{z:.3} at the tab end {end:.3} mm along the ring is below the \
                             tab top Z{:.3}",
                            tabs.z_tab
                        ));
                    }
                }
            }
        }
    }
    Ok(())
}

/// Moves along the samples, from the first one (where the tool already is). Straight up or
/// down at the plunge feed; along the ring at the cutting feed, level or not. Whole chords
/// of an arc keep their circle, so they can be written as arcs (helical when they descend).
fn emit(moves: &mut Vec<Move>, cut: &RingCut, along: &Along, samples: &[Sample], arcs: &ArcTable) {
    let Some(first) = samples.first() else {
        return;
    };
    let mut here = along.point(cut.ring, first);
    let mut z = first.z;
    for sample in &samples[1..] {
        let point = along.point(cut.ring, sample);
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
        m.circle = arcs.get(here, point);
        moves.push(m);
        here = point;
        z = sample.z;
    }
}
