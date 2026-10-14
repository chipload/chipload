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
use chipload_core::primitives::distance;
use chipload_core::tolerances::{CHAIN_TOL_MM, DEPTH_TOL_MM, VCARVE_TOL_MM};

use crate::ToolpathError;
use crate::cavalier::{Area, geometry};
use crate::medial::{Graph, Outline, medial_axis};
use crate::model::Move;
use crate::pocket::build_region;

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
    fn slope(&self) -> f64 {
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

/// One continuous cut: points and the tip depth below the carved surface at each (>= 0).
#[derive(Debug, Clone, PartialEq)]
pub struct CarvePath {
    pub points: Vec<Point>,
    pub depths: Vec<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VcarvePlan {
    /// In cut order.
    pub paths: Vec<CarvePath>,
    /// Warnings: strokes wider than the depth allows, strokes narrower than the tip.
    pub notes: Vec<String>,
    /// The deepest any path goes, mm below the carved surface.
    pub deepest: f64,
}

/// Paths for the regions the selected closed contours make (as pockets do: outer minus the
/// contours inside it). `max_depth` is the cap below the carved surface; `start` is where the
/// tool is, for the cut order.
pub fn plan_vcarve(
    contours: &[&Contour],
    bit: &VBit,
    max_depth: f64,
    start: Point,
) -> Result<VcarvePlan, ToolpathError> {
    if !(bit.angle > 0.0 && bit.angle < std::f64::consts::PI) {
        return Err(geometry(format!(
            "the V-bit angle must be more than 0 and less than 180 degrees, got {:.3}",
            bit.angle.to_degrees()
        )));
    }
    let cap = max_depth.min(bit.deepest());
    let cap_radius = bit.radius_at(cap);
    let mut graph = Graph::default();
    let mut notes = Vec::new();
    for region in build_region(contours)? {
        let ids = ids_text(&region.contour_ids);
        let outline = Outline::new(&region.area);
        let mut axis = medial_axis(&region.area, &outline, &ids, "V-carve", VCARVE_TOL_MM)?;
        axis.prune(&outline, VCARVE_TOL_MM);
        let widest = axis.widest(&outline);
        axis.set_cut(&outline, cap_radius, VCARVE_TOL_MM);
        axis.verify(&region.area, &outline, &ids)?;
        if widest > cap_radius + VCARVE_TOL_MM {
            let limit = if bit.deepest() < max_depth {
                format!(
                    "the V-bit ({:.3} mm across), which carves at most {cap:.3} mm deep",
                    bit.diameter
                )
            } else {
                format!("a carve at most {cap:.3} mm deep")
            };
            notes.push(format!(
                "contours {ids}: too wide for {limit} ({:.3} mm wide at the surface); the \
                 edges of the widest parts aren't reached (they need {:.3} mm). Allow deeper, \
                 use a wider angle, or clear the floor",
                2.0 * cap_radius,
                bit.depth_for(widest)
            ));
        }
        let narrow = axis.narrow_length(bit.tip_diameter / 2.0);
        if narrow > VCARVE_TOL_MM {
            notes.push(format!(
                "contours {ids}: {narrow:.3} mm of centre line is narrower than the tip \
                 ({:.3} mm) and isn't carved",
                bit.tip_diameter
            ));
        }
        graph.append(axis);
    }
    let mut paths = Vec::new();
    for walked in graph.walk(start) {
        paths.extend(carve_pieces(&walked, bit));
    }
    let deepest = paths
        .iter()
        .flat_map(|p| p.depths.iter().copied())
        .fold(0.0, f64::max);
    Ok(VcarvePlan {
        paths,
        notes,
        deepest,
    })
}

/// Moves for the paths: pass by pass, each no deeper than its level (depths below the
/// carved surface, ascending; the last is the deepest). A later pass only runs where the
/// previous one stopped short. Starts and ends retracted to `safe_z`.
pub fn vcarve_moves(
    paths: &[CarvePath],
    levels: &[f64],
    surface_z: f64,
    safe_z: f64,
    feed: f64,
    plunge_feed: f64,
) -> Vec<Move> {
    let mut moves = vec![Move::rapid_z(safe_z)];
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
    moves
}

/// Pass depths below the carved surface: one pass at `deepest`, or equal passes no deeper
/// than `step_down`, the last exactly `deepest`.
pub fn carve_levels(deepest: f64, step_down: Option<f64>) -> Result<Vec<f64>, ToolpathError> {
    match step_down {
        Some(step) if deepest > DEPTH_TOL_MM => crate::profile::pass_depths(0.0, deepest, step),
        _ => Ok(vec![deepest]),
    }
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
fn carve_pieces(walked: &[(Point, f64)], bit: &VBit) -> Vec<CarvePath> {
    let path = CarvePath {
        points: walked.iter().map(|w| w.0).collect(),
        depths: walked.iter().map(|w| bit.depth_for(w.1)).collect(),
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
                        return Err(geometry(format!(
                            "contours {ids}: V-carve verification failed: the cut would \
                             cross the outline near ({:.3}, {:.3})",
                            p.0, p.1
                        )));
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

    /// Add another region's graph (its nodes are never shared).
    fn append(&mut self, other: Graph) {
        let offset = self.nodes.len();
        self.nodes.extend(other.nodes);
        self.edges
            .extend(other.edges.into_iter().filter(|e| e.alive).map(|mut e| {
                e.ends = [e.ends[0] + offset, e.ends[1] + offset];
                e
            }));
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
