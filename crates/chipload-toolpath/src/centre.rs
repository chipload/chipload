// SPDX-License-Identifier: GPL-3.0-or-later
//! Centre lines of a traced picture's black strokes, for a follow path
//! (docs/specs/image-arcs-centrelines.md §3): the medial axis V-carving uses, tidied of the
//! side branches a rough or square stroke end grows, as paths between its ends and
//! junctions.

use chipload_core::Point;
use chipload_core::chain::{Contour, OrientedSegment, Segment};
use chipload_core::order::Ordered;
use chipload_core::primitives::{Line, Primitive, distance};
use chipload_core::tolerances::{ARC_APPROX_TOL_MM, CHAIN_TOL_MM};

use crate::ToolpathError;
use crate::medial::{Graph, Outline, medial_axis};
use crate::pocket::build_region;

/// A side branch goes when it adds less than this fraction of the stroke's half-width at
/// its junction (or the smoothing, if more): a square stroke end needs 0.41 to lose its two
/// corner branches; real branches (a T's leg) reach much farther.
const TIDY_FRACTION: f64 = 0.5;
/// The stroke widths reported: these percentiles of the half-widths along the paths.
const WIDTH_PERCENTILES: (f64, f64) = (0.05, 0.95);

/// One centre line, in mm.
#[derive(Debug, Clone, PartialEq)]
pub struct CentrePath {
    /// Along the path; a closed one doesn't repeat its first point.
    pub points: Vec<Point>,
    pub closed: bool,
}

/// What [`centre_lines`] found.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CentreLines {
    pub paths: Vec<CentrePath>,
    /// Pieces whose centre line is shorter than the smoothing (dots), left out.
    pub dots: usize,
    /// The strokes' widths along the paths, mm: (5th, 95th percentile); None without paths.
    pub widths: Option<(f64, f64)>,
}

/// The centre lines of the black areas `loops` enclose (outlines counter-clockwise, holes
/// clockwise, never crossing), tidied with `smoothing`.
pub fn centre_lines(loops: &[Vec<Point>], smoothing: f64) -> Result<CentreLines, ToolpathError> {
    let contours: Vec<Contour> = loops
        .iter()
        .enumerate()
        .filter(|(_, ring)| ring.len() >= 3)
        .map(|(k, ring)| contour(k + 1, ring))
        .collect();
    let refs: Vec<&Contour> = contours.iter().collect();
    let mut found = CentreLines::default();
    let mut half_widths: Vec<f64> = Vec::new();
    for region in build_region(&refs)? {
        let ids = region
            .contour_ids
            .iter()
            .map(|id| format!("#{id}"))
            .collect::<Vec<_>>()
            .join(", ");
        let outline = Outline::new(&region.area);
        let mut axis = medial_axis(
            &region.area,
            &outline,
            &ids,
            "centre line",
            ARC_APPROX_TOL_MM,
        )?;
        let allowance = |radius: f64| smoothing.max(TIDY_FRACTION * radius);
        if is_dot(&axis, &outline, allowance) {
            found.dots += 1;
            continue;
        }
        tidy(&mut axis, &outline, allowance);
        axis.set_cut(&outline, f64::INFINITY, ARC_APPROX_TOL_MM);
        let paths = chains(&axis);
        for (path, radii) in paths {
            if path_length(&path.points, path.closed) >= CHAIN_TOL_MM {
                half_widths.extend(radii);
                found.paths.push(path);
            }
        }
    }
    found.widths = percentiles(half_widths);
    Ok(found)
}

/// Whether the whole piece lies within its widest circle grown by the allowance (a round
/// dot, a small blob): its centre line would add nothing to a plunge.
fn is_dot(graph: &Graph, outline: &Outline, allowance: impl Fn(f64) -> f64) -> bool {
    let points: Vec<(Point, f64)> = graph
        .edges
        .iter()
        .filter(|e| e.alive)
        .flat_map(|e| e.points.iter().map(|&p| (p, outline.distance(p))))
        .collect();
    let Some(&(centre, widest)) = points.iter().max_by_key(|p| Ordered(p.1)) else {
        return true;
    };
    let limit = widest + allowance(widest);
    points
        .iter()
        .all(|&(p, r)| distance(p, centre) + r <= limit)
}

/// Remove the side branches that add nothing beyond their junction's circle grown by the
/// allowance, as V-carving's pruning does, but judging every branch of a pass together, so
/// the two corner branches of a square stroke end both go (one removed first would make the
/// other part of the stroke). A junction keeps its farthest-reaching branch when all of them
/// would go.
fn tidy(graph: &mut Graph, outline: &Outline, allowance: impl Fn(f64) -> f64) {
    let radius: Vec<f64> = graph.nodes.iter().map(|&p| outline.distance(p)).collect();
    let mut slack = vec![0.0_f64; graph.nodes.len()];
    let mut degree = graph.degrees();
    loop {
        // (junction, chain, how far beyond the junction's circle it reaches)
        let mut found: Vec<(usize, Vec<usize>, f64)> = Vec::new();
        for leaf in 0..graph.nodes.len() {
            if degree[leaf] != 1 {
                continue;
            }
            let Some((chain, junction)) = graph.chain_from(leaf, &degree) else {
                continue;
            };
            let centre = graph.nodes[junction];
            let mut needed = 0.0_f64;
            for &e in &chain {
                let edge = &graph.edges[e];
                for (k, &p) in edge.points.iter().enumerate() {
                    let own = if k == 0 || k + 1 == edge.points.len() {
                        let node = if k == 0 { edge.ends[0] } else { edge.ends[1] };
                        if node == junction {
                            continue;
                        }
                        slack[node]
                    } else {
                        0.0
                    };
                    let reach = distance(p, centre) + outline.distance(p) + own;
                    needed = needed.max(reach - radius[junction]);
                }
            }
            if needed <= allowance(radius[junction]) {
                found.push((junction, chain, needed));
            }
        }
        if found.is_empty() {
            return;
        }
        // A junction all of whose branches would go keeps the farthest-reaching one.
        let mut keep = vec![false; found.len()];
        for (k, &(junction, _, _)) in found.iter().enumerate() {
            let at: Vec<usize> = (0..found.len())
                .filter(|&m| found[m].0 == junction)
                .collect();
            let farthest = at
                .iter()
                .copied()
                .max_by(|&a, &b| found[a].2.total_cmp(&found[b].2).then(b.cmp(&a)));
            if at.len() >= degree[junction] && farthest == Some(k) {
                keep[k] = true;
            }
        }
        let mut removed = false;
        for (k, (junction, chain, needed)) in found.into_iter().enumerate() {
            if keep[k] {
                continue;
            }
            for e in chain {
                graph.edges[e].alive = false;
                let [a, b] = graph.edges[e].ends;
                degree[a] -= 1;
                degree[b] -= 1;
            }
            slack[junction] = slack[junction].max(needed);
            removed = true;
        }
        if !removed {
            return;
        }
    }
}

/// A closed ring of points as a contour of lines.
fn contour(id: usize, ring: &[Point]) -> Contour {
    let n = ring.len();
    let parts = (0..n)
        .map(|k| OrientedSegment {
            segment: Segment {
                construction: false,
                prim: Primitive::Line(Line {
                    start: ring[k],
                    end: ring[(k + 1) % n],
                }),
                layer: String::new(),
                handle: String::new(),
                source_type: "IMAGE".into(),
                shape: None,
                from: None,
            },
            reversed: false,
        })
        .collect();
    Contour {
        id,
        parts,
        closed: true,
        invalid_reason: None,
        nesting_depth: None,
    }
}

/// The graph's live edges as paths: runs between nodes that aren't two-way (ends and
/// junctions), and rings with no such node as closed paths; with the half-width at each
/// point.
fn chains(graph: &Graph) -> Vec<(CentrePath, Vec<f64>)> {
    let mut at: Vec<Vec<usize>> = vec![Vec::new(); graph.nodes.len()];
    for (e, edge) in graph.edges.iter().enumerate().filter(|(_, e)| e.alive) {
        at[edge.ends[0]].push(e);
        if edge.ends[1] != edge.ends[0] {
            at[edge.ends[1]].push(e);
        }
    }
    let mut used = vec![false; graph.edges.len()];
    let mut paths = Vec::new();
    // Open runs, from every end or junction.
    for node in 0..graph.nodes.len() {
        if at[node].len() == 2 {
            continue;
        }
        for &first in &at[node] {
            if used[first] {
                continue;
            }
            let (points, radii, _) = walk(graph, &at, &mut used, node, first);
            paths.push((
                CentrePath {
                    points,
                    closed: false,
                },
                radii,
            ));
        }
    }
    // What is left are rings of two-way nodes.
    for first in 0..graph.edges.len() {
        if !graph.edges[first].alive || used[first] {
            continue;
        }
        let start = graph.edges[first].ends[0];
        let (mut points, mut radii, end) = walk(graph, &at, &mut used, start, first);
        if end == start && points.len() > 1 {
            points.pop();
            radii.pop();
        }
        paths.push((
            CentrePath {
                points,
                closed: end == start,
            },
            radii,
        ));
    }
    paths
}

/// Points and half-widths from `node` along edge `first` and on through two-way nodes, until
/// an end, a junction, or an edge already used; and the node it stopped at.
fn walk(
    graph: &Graph,
    at: &[Vec<usize>],
    used: &mut [bool],
    node: usize,
    first: usize,
) -> (Vec<Point>, Vec<f64>, usize) {
    let mut points: Vec<Point> = Vec::new();
    let mut radii: Vec<f64> = Vec::new();
    let (mut here, mut edge) = (node, first);
    loop {
        used[edge] = true;
        let walked = graph.edges[edge].from(here);
        let skip = usize::from(!points.is_empty());
        for &(p, r) in &walked[skip.min(walked.len())..] {
            points.push(p);
            radii.push(r);
        }
        here = graph.edges[edge].other(here);
        if at[here].len() != 2 {
            return (points, radii, here);
        }
        match at[here].iter().copied().find(|&e| !used[e]) {
            Some(next) => edge = next,
            None => return (points, radii, here),
        }
    }
}

fn path_length(points: &[Point], closed: bool) -> f64 {
    let open: f64 = points.windows(2).map(|w| distance(w[0], w[1])).sum();
    match (closed, points.first(), points.last()) {
        (true, Some(&a), Some(&b)) => open + distance(a, b),
        _ => open,
    }
}

/// Twice the given percentiles of the half-widths.
fn percentiles(mut half_widths: Vec<f64>) -> Option<(f64, f64)> {
    half_widths.retain(|r| r.is_finite());
    if half_widths.is_empty() {
        return None;
    }
    half_widths.sort_by(f64::total_cmp);
    let at = |q: f64| {
        let k = ((half_widths.len() - 1) as f64 * q).round() as usize;
        2.0 * half_widths[k.min(half_widths.len() - 1)]
    };
    Some((at(WIDTH_PERCENTILES.0), at(WIDTH_PERCENTILES.1)))
}
