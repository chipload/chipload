// SPDX-License-Identifier: GPL-3.0-or-later
//! V-carving: a V-bit follows the centre line (medial axis) of closed shapes, as deep as
//! makes its cut exactly as wide as the shape. See docs/specs/v-carve.md.
//!
//! The medial axis comes from a segment Voronoi diagram of the outline split into chords
//! (boostvoronoi, ADR 0015). It only says where the tool goes: how wide it cuts at every point
//! is the exact distance to the real outline (arcs, not chords), and every move is verified
//! against that distance afterwards.

use chipload_core::Point;
use chipload_core::chain::Contour;
use chipload_core::order::Ordered;
use chipload_core::primitives::distance;
use chipload_core::tolerances::{CHAIN_TOL_MM, DEPTH_TOL_MM, FLOOR_LEFT_SHARE, VCARVE_TOL_MM};
use chipload_i18n::{Digits, Message, Value};

use crate::ToolpathError;
use crate::cavalier::{Area, geometry};
use crate::floor::Floor;
use crate::medial::{Graph, Outline, fine_ring, medial_axis};
use crate::model::Move;
use crate::pocket::build_region;
use crate::start::nearest_ties;

/// A V-bit's cutting geometry.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VBit {
    /// Included angle, radians, 0 < angle < π.
    pub angle: f64,
    /// Flat tip, mm (0: pointed).
    pub tip_diameter: f64,
    /// Largest cutting diameter, mm.
    pub diameter: f64,
}

impl VBit {
    /// How much wider the cut gets at the surface per mm of depth, on each side.
    pub fn slope(&self) -> f64 {
        (self.angle / 2.0).tan()
    }

    /// Radius of the cut at the surface with the tip `depth` below it.
    pub fn radius_at(&self, depth: f64) -> f64 {
        self.tip_diameter / 2.0 + depth * self.slope()
    }

    /// Tip depth for a cut of `radius` at the surface; negative when the tip is wider.
    pub fn depth_for(&self, radius: f64) -> f64 {
        (radius - self.tip_diameter / 2.0) / self.slope()
    }

    /// The deepest it cuts with its cone: beyond, the shank would rub.
    pub fn deepest(&self) -> f64 {
        self.depth_for(self.diameter / 2.0)
    }
}

/// The longest chord of the centre line when it is cleaned up after a floor pass, mm: where
/// the end mill stops reaching is found on each.
const CLEANUP_STEP_MM: f64 = 0.5;

/// One continuous cut: points and the tip depth below the carved surface at each (>= 0).
#[derive(Debug, Clone, PartialEq)]
pub struct CarvePath {
    pub points: Vec<Point>,
    pub depths: Vec<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VcarvePlan {
    /// In cut order, area by area.
    pub paths: Vec<CarvePath>,
    /// How many of `paths` each area has, in cut order: an area (a letter, the dot of an
    /// i) gets all its passes before the next (docs/specs/v-carve.md decision 13).
    pub areas: Vec<usize>,
    /// Warnings: strokes wider than the depth allows, strokes narrower than the tip.
    pub notes: Vec<Message>,
    /// The deepest any path goes, mm below the carved surface.
    pub deepest: f64,
    /// With a floor pass: each wide region's floor its end mill and the V-bit don't clear
    /// (docs/specs/vcarve-wide-strokes.md §4.1).
    pub floors: Vec<FloorLeft>,
}

/// A region's floor that its floor pass leaves partly standing.
#[derive(Debug, Clone, PartialEq)]
pub struct FloorLeft {
    /// The region's contours, as "#3, #4".
    pub contours: String,
    /// The share of the floor cleared, 0 to 1.
    pub cleared: f64,
    /// The largest end mill diameter that would clear it all, mm.
    pub largest: f64,
}

/// Paths for the regions the selected closed contours make (as pockets do: outer minus the
/// contours inside it). `max_depth` is the cap below the carved surface; `start` is where the
/// tool is, for the cut order. `floor`: the radius of the floor pass's end mill, if there is
/// one (docs/specs/vcarve-wide-strokes.md §4).
pub fn plan_vcarve(
    contours: &[&Contour],
    bit: &VBit,
    max_depth: f64,
    start: Point,
    floor: Option<f64>,
) -> Result<VcarvePlan, ToolpathError> {
    if !(bit.angle > 0.0 && bit.angle < std::f64::consts::PI) {
        return Err(geometry(Message::new("toolpath-error-vbit-angle").arg(
            "got",
            Value::Number(bit.angle.to_degrees(), Digits::Fixed(3)),
        )));
    }
    let cap = max_depth.min(bit.deepest());
    let cap_radius = bit.radius_at(cap);
    // Each area's centre line and walls, cut nearest first after the loop.
    let mut regions: Vec<(Graph, Vec<CarvePath>)> = Vec::new();
    let mut notes = Vec::new();
    let mut floors = Vec::new();
    for region in build_region(contours)? {
        let mut walls = Vec::new();
        let ids = ids_text(&region.contour_ids);
        let outline = Outline::new(&region.area);
        let mut axis = medial_axis(&region.area, &outline, &ids, "V-carve", VCARVE_TOL_MM)?;
        axis.prune(&outline, VCARVE_TOL_MM);
        let widest = axis.widest(&outline);
        if widest > cap_radius + VCARVE_TOL_MM {
            // Too wide to reach both sides: the centre line where the stroke is narrow
            // enough, walls along the outline where it is wider (vcarve-wide-strokes.md §3).
            // Within half the tolerance over the cap the centre line still reaches.
            let reach = cap_radius + VCARVE_TOL_MM / 2.0;
            axis.set_cut(&outline, cap_radius + VCARVE_TOL_MM, VCARVE_TOL_MM);
            walls.extend(wall_rings(&region.area, &outline, cap_radius, cap, &ids)?);
            let inset = cap_radius - bit.tip_diameter / 2.0;
            let floor_of = match floor {
                Some(radius) => {
                    Floor::new(&region.area, inset, bit.slope(), bit.tip_diameter / 2.0)?
                        .map(|f| (f, radius))
                }
                None => None,
            };
            match floor_of {
                Some((floor, radius)) => {
                    // The V-bit cleans up along the centre line where the end mill can't
                    // reach (§4.1, decision 7).
                    axis.densify(CLEANUP_STEP_MM);
                    let full = axis.clone();
                    let reached = floor.reach(radius)?;
                    axis.keep_where(|p, r| r <= reach || !reached.reaches(p));
                    let cleanup =
                        |at: &crate::floor::Reach| full.chords_where(reach, |p| !at.reaches(p));
                    let left = floor.left_share(&reached, &cleanup(&reached));
                    if left > FLOOR_LEFT_SHARE {
                        floors.push(FloorLeft {
                            contours: ids.clone(),
                            cleared: 1.0 - left,
                            largest: 2.0 * floor.largest_clearing(radius, cleanup)?,
                        });
                    }
                }
                None => {
                    axis.keep_within(reach);
                    if floor.is_none() {
                        let note = if bit.deepest() < max_depth {
                            Message::new("toolpath-vcarve-floor-left-bit")
                                .arg("across", fixed3(bit.diameter))
                        } else {
                            Message::new("toolpath-vcarve-floor-left")
                        };
                        notes.push(
                            note.arg("contours", ids.as_str())
                                .arg("cap", fixed3(cap))
                                .arg("wide", fixed3(2.0 * cap_radius)),
                        );
                    }
                }
            }
        } else {
            axis.set_cut(&outline, cap_radius, VCARVE_TOL_MM);
        }
        axis.verify(&region.area, &outline, &ids)?;
        let narrow = axis.narrow_length(bit.tip_diameter / 2.0);
        if narrow > VCARVE_TOL_MM {
            notes.push(
                Message::new("toolpath-vcarve-narrow")
                    .arg("contours", ids.as_str())
                    .arg("narrow", fixed3(narrow))
                    .arg("tip", fixed3(bit.tip_diameter)),
            );
        }
        regions.push((axis, walls));
    }
    let (paths, areas) = areas_in_order(regions, bit, cap, start);
    let deepest = paths
        .iter()
        .flat_map(|p| p.depths.iter().copied())
        .fold(0.0, f64::max);
    Ok(VcarvePlan {
        paths,
        areas,
        notes,
        deepest,
        floors,
    })
}

/// The areas' paths in cut order, and how many each has: the area nearest the tool first
/// (`area_gap`; equally near, the first), its centre line walked from where the tool is,
/// then its walls nearest first; the next area from where that one ends.
fn areas_in_order(
    mut regions: Vec<(Graph, Vec<CarvePath>)>,
    bit: &VBit,
    cap: f64,
    start: Point,
) -> (Vec<CarvePath>, Vec<usize>) {
    let mut paths = Vec::new();
    let mut areas = Vec::new();
    let mut at = start;
    while !regions.is_empty() {
        let here = at;
        let pick = nearest_ties(
            regions
                .iter()
                .enumerate()
                .map(|(i, (graph, walls))| (i, area_gap(graph, walls, here))),
        )
        .into_iter()
        .next()
        .unwrap_or(0);
        let (graph, walls) = regions.remove(pick);
        let before = paths.len();
        for walked in graph.walk(at) {
            paths.extend(carve_pieces(&walked, bit, cap));
        }
        if let Some(&end) = paths[before..].last().and_then(|p| p.points.last()) {
            at = end;
        }
        paths.extend(nearest_first(walls, &mut at));
        if paths.len() > before {
            areas.push(paths.len() - before);
        }
    }
    (paths, areas)
}

/// How far an area is from `at`: to its centre line's nearest node with an edge, or its
/// walls' nearest point.
fn area_gap(graph: &Graph, walls: &[CarvePath], at: Point) -> f64 {
    let degrees = graph.degrees();
    let nodes = graph
        .nodes
        .iter()
        .zip(&degrees)
        .filter(|&(_, &d)| d > 0)
        .map(|(&p, _)| p);
    let wall_points = walls.iter().flat_map(|w| w.points.iter().copied());
    nodes
        .chain(wall_points)
        .map(|p| distance(at, p))
        .fold(f64::INFINITY, f64::min)
}

impl VcarvePlan {
    /// The paths, area by area (`areas`).
    pub fn area_paths(&self) -> Vec<&[CarvePath]> {
        let mut rest = self.paths.as_slice();
        let mut split = Vec::new();
        for &n in &self.areas {
            let (area, after) = rest.split_at(n.min(rest.len()));
            split.push(area);
            rest = after;
        }
        split
    }
}

/// Moves for the areas: each area all its passes before the next, pass by pass, each no
/// deeper than its level (depths below the carved surface, ascending; the last is the
/// deepest). A later pass only runs where the previous one stopped short. Starts and ends
/// retracted to `safe_z`.
pub fn vcarve_moves(
    areas: &[&[CarvePath]],
    levels: &[f64],
    surface_z: f64,
    safe_z: f64,
    feed: f64,
    plunge_feed: f64,
) -> Vec<Move> {
    let mut moves = vec![Move::rapid_z(safe_z)];
    for paths in areas {
        area_moves(
            &mut moves,
            paths,
            levels,
            surface_z,
            safe_z,
            feed,
            plunge_feed,
        );
    }
    moves
}

/// One area's passes.
fn area_moves(
    moves: &mut Vec<Move>,
    paths: &[CarvePath],
    levels: &[f64],
    surface_z: f64,
    safe_z: f64,
    feed: f64,
    plunge_feed: f64,
) {
    let mut previous: Option<f64> = None;
    for &level in levels {
        for path in paths {
            let pieces = match previous {
                None => vec![path.clone()],
                Some(above) => deeper_than(path, above),
            };
            for piece in pieces {
                let Some(&first) = piece.points.first() else {
                    continue;
                };
                let z_of = |d: f64| surface_z - d.min(level);
                moves.push(Move::rapid_xy(first));
                let mut z = z_of(piece.depths[0]);
                moves.push(Move::feed_z(z, plunge_feed));
                let mut at = first;
                for (&p, &d) in piece.points.iter().zip(&piece.depths).skip(1) {
                    let next = z_of(d);
                    // Steeper than 45° down: the plunge feed (decision 10).
                    let rate = if z - next > distance(at, p) {
                        plunge_feed
                    } else {
                        feed
                    };
                    moves.push(Move::feed_xyz(p, next, rate));
                    (at, z) = (p, next);
                }
                moves.push(Move::rapid_z(safe_z));
            }
        }
        previous = Some(level);
    }
}

/// Pass depths below the carved surface: `passes` passes to `depth` that each remove the
/// same cross-section (docs/specs/v-carve.md decision 12). A V groove's cross-section grows
/// as its depth squared, so pass k of n goes to `depth` × √(k / n), the last exactly
/// `depth`. Where a stroke is shallower than a pass, that pass finds nothing left to cut.
pub fn carve_levels(depth: f64, passes: u32) -> Vec<f64> {
    let count = passes.max(1);
    (1..=count)
        .map(|k| {
            if k == count {
                depth
            } else {
                depth * (f64::from(k) / f64::from(count)).sqrt()
            }
        })
        .collect()
}

/// The parts of `path` deeper than `above`, each starting and ending at `above`
/// (interpolated) where it crosses that level.
fn deeper_than(path: &CarvePath, above: f64) -> Vec<CarvePath> {
    split_where(path, |d| d > above + DEPTH_TOL_MM, above)
}

/// Pieces of `path` where `keep(depth)`, cut at `level` where the depth crosses it.
fn split_where(path: &CarvePath, keep: impl Fn(f64) -> bool, level: f64) -> Vec<CarvePath> {
    let mut pieces = Vec::new();
    let mut current = CarvePath {
        points: Vec::new(),
        depths: Vec::new(),
    };
    let n = path.points.len();
    for i in 0..n {
        let (p, d) = (path.points[i], path.depths[i]);
        if i > 0 {
            let (q, e) = (path.points[i - 1], path.depths[i - 1]);
            if keep(e) != keep(d) && (d - e).abs() > 0.0 {
                let t = (level - e) / (d - e);
                let at = (q.0 + (p.0 - q.0) * t, q.1 + (p.1 - q.1) * t);
                current.points.push(at);
                current.depths.push(level);
                if !keep(d) {
                    pieces.push(std::mem::replace(
                        &mut current,
                        CarvePath {
                            points: Vec::new(),
                            depths: Vec::new(),
                        },
                    ));
                }
            }
        }
        if keep(d) {
            current.points.push(p);
            current.depths.push(d);
        }
    }
    pieces.push(current);
    pieces.retain(|piece| piece.points.len() >= 2);
    pieces
}

/// A walked path (points and cut radius) as cuts: the tip depth for each radius, without the
/// parts narrower than the tip.
fn carve_pieces(walked: &[(Point, f64)], bit: &VBit, cap: f64) -> Vec<CarvePath> {
    let path = CarvePath {
        points: walked.iter().map(|w| w.0).collect(),
        depths: walked.iter().map(|w| bit.depth_for(w.1).min(cap)).collect(),
    };
    split_where(&path, |d| d >= -DEPTH_TOL_MM, 0.0)
        .into_iter()
        .map(|mut piece| {
            for d in &mut piece.depths {
                *d = d.max(0.0);
            }
            piece
        })
        .filter(|piece| piece.depths.iter().any(|&d| d > DEPTH_TOL_MM))
        .collect()
}

/// The walls of a region too wide to carve from its centre line: the region shrunk by the
/// cut's radius at the surface, `radius`, each loop a closed path at `depth`, its cone
/// touching the outline at the surface (vcarve-wide-strokes.md §3.1). Every point and the
/// middle of every chord is checked to keep the cone inside the outline.
fn wall_rings(
    area: &Area,
    outline: &Outline,
    radius: f64,
    depth: f64,
    ids: &str,
) -> Result<Vec<CarvePath>, ToolpathError> {
    let mut rings = Vec::new();
    for inner in area.shrunk(radius)? {
        for pline in std::iter::once(&inner.outer).chain(&inner.holes) {
            let mut points = fine_ring(pline, VCARVE_TOL_MM);
            let Some(&first) = points.first() else {
                continue;
            };
            points.push(first);
            for w in points.windows(2) {
                let middle = ((w[0].0 + w[1].0) / 2.0, (w[0].1 + w[1].1) / 2.0);
                for p in [w[0], middle] {
                    if outline.distance(p) + VCARVE_TOL_MM < radius {
                        return Err(geometry(
                            Message::new("toolpath-error-vcarve-verification")
                                .arg("contours", ids)
                                .arg("at", Value::Point(p.0, p.1, Digits::Fixed(3))),
                        ));
                    }
                }
            }
            let depths = vec![depth; points.len()];
            rings.push(CarvePath { points, depths });
        }
    }
    Ok(rings)
}

/// The closed `rings` in cut order: each the nearest to where the tool is (`at`), started at
/// its point nearest that; `at` ends where the last one closes. Equally near
/// (`nearest_ties`): the first ring, the lowest then leftmost point.
fn nearest_first(mut rings: Vec<CarvePath>, at: &mut Point) -> Vec<CarvePath> {
    let mut ordered = Vec::new();
    while !rings.is_empty() {
        let here = *at;
        let gap = |ring: &CarvePath| {
            ring.points
                .iter()
                .map(|&p| distance(here, p))
                .fold(f64::INFINITY, f64::min)
        };
        let pick = nearest_ties(rings.iter().enumerate().map(|(i, r)| (i, gap(r))))
            .into_iter()
            .next()
            .unwrap_or(0);
        let mut ring = rings.remove(pick);
        // Closed: the last point repeats the first. Start at the nearest instead.
        ring.points.pop();
        ring.depths.pop();
        let near = ring
            .points
            .iter()
            .enumerate()
            .map(|(k, &p)| (k, distance(here, p)));
        let k = nearest_ties(near)
            .into_iter()
            .min_by_key(|&k| (Ordered(ring.points[k].1), Ordered(ring.points[k].0)))
            .unwrap_or(0);
        ring.points.rotate_left(k);
        ring.depths.rotate_left(k);
        if let (Some(&p), Some(&d)) = (ring.points.first(), ring.depths.first()) {
            ring.points.push(p);
            ring.depths.push(d);
            *at = p;
        }
        ordered.push(ring);
    }
    ordered
}

/// A length with 3 decimals, for notes.
fn fixed3(mm: f64) -> Value {
    Value::Length(mm, Digits::Fixed(3))
}

fn ids_text(ids: &[usize]) -> String {
    ids.iter()
        .map(|i| format!("#{i}"))
        .collect::<Vec<_>>()
        .join(", ")
}

impl Graph {
    /// The largest distance to the outline along the axis: how wide the widest part is.
    fn widest(&self, outline: &Outline) -> f64 {
        self.edges
            .iter()
            .filter(|e| e.alive)
            .flat_map(|e| e.points.iter().map(|&p| outline.distance(p)))
            .fold(0.0, f64::max)
    }

    /// within the outline, checked at points between the stored ones against the exact
    /// distance; and every point is inside the region.
    fn verify(&self, area: &Area, outline: &Outline, ids: &str) -> Result<(), ToolpathError> {
        for edge in self.edges.iter().filter(|e| e.alive) {
            for k in 0..edge.points.len().saturating_sub(1) {
                let (a, b) = (edge.points[k], edge.points[k + 1]);
                let (ra, rb) = (edge.radii[k], edge.radii[k + 1]);
                for step in 0..=4 {
                    let t = f64::from(step) / 4.0;
                    let p = (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t);
                    let r = ra + (rb - ra) * t;
                    let room = outline.distance(p);
                    let inside = area.contains(p) || room <= VCARVE_TOL_MM;
                    if !inside || r > room + VCARVE_TOL_MM {
                        return Err(geometry(
                            Message::new("toolpath-error-vcarve-verification")
                                .arg("contours", ids)
                                .arg("at", Value::Point(p.0, p.1, Digits::Fixed(3))),
                        ));
                    }
                }
            }
        }
        Ok(())
    }

    /// Length of axis where the stroke is narrower than the tip, not counting the tips of
    /// corners (from the corner, where the width is 0, up to where the tip fits).
    fn narrow_length(&self, tip_radius: f64) -> f64 {
        if tip_radius <= 0.0 {
            return 0.0;
        }
        let mut total = 0.0;
        for edge in self.edges.iter().filter(|e| e.alive) {
            let corner_at = |k: usize| edge.radii[k] <= CHAIN_TOL_MM;
            let mut run = 0.0;
            let mut from_corner = false;
            for k in 0..edge.points.len() {
                let narrow = edge.radii[k] < tip_radius - CHAIN_TOL_MM;
                if narrow && run == 0.0 {
                    from_corner = corner_at(k);
                }
                if k > 0 && narrow && edge.radii[k - 1] < tip_radius - CHAIN_TOL_MM {
                    run += distance(edge.points[k - 1], edge.points[k]);
                }
                let ends = k + 1 == edge.points.len();
                if !narrow || ends {
                    let to_corner = narrow && corner_at(k);
                    if !(from_corner || to_corner) {
                        total += run;
                    }
                    run = 0.0;
                    from_corner = false;
                }
            }
        }
        total
    }

    /// Every alive edge once, in continuous paths found by a greedy walk: each starts at the
    /// nearest end of the axis with an edge left (a corner, at depth 0), else the nearest
    /// node with an odd number of edges left, else any; ties by (y, x). The walk goes on
    /// where it can, so dead ends are left for later paths to start from.
    fn walk(&self, start: Point) -> Vec<Vec<(Point, f64)>> {
        let mut used: Vec<bool> = self.edges.iter().map(|e| !e.alive).collect();
        let mut left: Vec<usize> = self.degrees();
        // Ends of the axis: corners, where the cut starts at depth 0.
        let end: Vec<bool> = left.iter().map(|&d| d == 1).collect();
        let mut at = start;
        let mut paths = Vec::new();
        while left.iter().any(|&d| d > 0) {
            let key = |n: usize| {
                let p = self.nodes[n];
                (
                    !end[n],
                    left[n].is_multiple_of(2),
                    distance(at, p),
                    p.1,
                    p.0,
                )
            };
            let Some(mut node) = (0..self.nodes.len())
                .filter(|&n| left[n] > 0)
                .min_by(|&a, &b| {
                    let (ka, kb) = (key(a), key(b));
                    ka.0.cmp(&kb.0)
                        .then(ka.1.cmp(&kb.1))
                        .then(ka.2.total_cmp(&kb.2))
                        .then(ka.3.total_cmp(&kb.3))
                        .then(ka.4.total_cmp(&kb.4))
                })
            else {
                break;
            };
            let mut path: Vec<(Point, f64)> = Vec::new();
            // Onwards where the walk can go on, so dead ends (corners) are left for later
            // paths to start from, at depth 0.
            let next = |node: usize, used: &[bool], left: &[usize]| {
                let open: Vec<usize> = (0..self.edges.len())
                    .filter(|&e| !used[e] && self.edges[e].ends.contains(&node))
                    .collect();
                open.iter()
                    .copied()
                    .find(|&e| left[self.edges[e].other(node)] > 1)
                    .or_else(|| open.first().copied())
            };
            while let Some(e) = next(node, &used, &left) {
                used[e] = true;
                let edge = &self.edges[e];
                left[edge.ends[0]] -= 1;
                left[edge.ends[1]] -= 1;
                let walked = edge.from(node);
                let skip = usize::from(!path.is_empty());
                path.extend(walked.into_iter().skip(skip));
                node = edge.other(node);
            }
            if let Some(&(end, _)) = path.last() {
                at = end;
            }
            paths.push(path);
        }
        paths
    }
}
