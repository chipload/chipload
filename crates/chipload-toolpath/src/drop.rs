// SPDX-License-Identifier: GPL-3.0-or-later
//! The drop cutter (docs/specs/stl-relief.md §6): for a point in plan and a tool, the
//! highest its tip must stay so that the tool touches a mesh from above and goes into it
//! nowhere. Each triangle under the tool is tested three ways, and the highest answer wins:
//!
//! 1. **corners:** a corner `d` from the axis holds the tip at `z − h(d)`;
//! 2. **the face:** the tool rests on the triangle's plane where the tool's slope matches
//!    the plane's, if that point lies inside the triangle;
//! 3. **edges:** along an edge the tip's reach `z(s) − h(√(p² + s²))` is concave (the edge's
//!    height is linear, the tool's shape convex), so its highest point is among the
//!    interval's ends, the foot of the perpendicular, the shape's kinks and its level
//!    points.
//!
//! Written from the geometry (ADR 0031): the tool is a convex shape turned about its axis
//! (`chipload_core::cutter`), lowered along Z until it first meets the surface.

use chipload_core::cutter::Cutter;
use chipload_core::tolerances::CHAIN_TOL_MM;
use chipload_mesh::Mesh;

type V3 = [f64; 3];

/// A triangle ready for testing: its corners, its plane as `z = a·x + b·y + c` unless it
/// stands upright, and its box in plan.
#[derive(Debug, Clone)]
struct Tri {
    v: [V3; 3],
    plane: Option<(f64, f64, f64)>,
    lo: (f64, f64),
    hi: (f64, f64),
    top: f64,
}

/// A mesh indexed for dropping a tool onto: its triangles in a grid of square cells by
/// their boxes in plan, so a point tests only the triangles near it.
#[derive(Debug, Clone)]
pub struct DropMesh {
    tris: Vec<Tri>,
    origin: (f64, f64),
    cell: f64,
    columns: usize,
    rows: usize,
    /// For each cell, row by row, the triangles whose boxes reach into it.
    cells: Vec<Vec<u32>>,
}

/// At most this many grid cells: a finer grid costs memory for little gain.
const MAX_CELLS: usize = 4_000_000;

impl DropMesh {
    /// Index `mesh` in cells of about `cell` mm: a tool's diameter suits.
    pub fn new(mesh: &Mesh, cell: f64) -> DropMesh {
        let tris: Vec<Tri> = mesh.triangles.iter().map(|t| prepare(*t)).collect();
        let (lo, hi) = tris.iter().fold(
            (
                (f64::INFINITY, f64::INFINITY),
                (f64::NEG_INFINITY, f64::NEG_INFINITY),
            ),
            |(lo, hi), t| {
                (
                    (lo.0.min(t.lo.0), lo.1.min(t.lo.1)),
                    (hi.0.max(t.hi.0), hi.1.max(t.hi.1)),
                )
            },
        );
        if tris.is_empty() {
            return DropMesh {
                tris,
                origin: (0.0, 0.0),
                cell: 1.0,
                columns: 0,
                rows: 0,
                cells: Vec::new(),
            };
        }
        let span = (hi.0 - lo.0).max(hi.1 - lo.1).max(CHAIN_TOL_MM);
        let mut cell = cell.max(CHAIN_TOL_MM);
        let count = |c: f64| {
            let columns = ((hi.0 - lo.0) / c).floor() as usize + 1;
            let rows = ((hi.1 - lo.1) / c).floor() as usize + 1;
            (columns, rows)
        };
        let (mut columns, mut rows) = count(cell);
        while columns.saturating_mul(rows) > MAX_CELLS {
            cell = cell.max(span / 1000.0) * 1.5;
            (columns, rows) = count(cell);
        }
        let mut cells = vec![Vec::new(); columns * rows];
        for (k, t) in tris.iter().enumerate() {
            let i0 = ((t.lo.0 - lo.0) / cell).floor() as usize;
            let j0 = ((t.lo.1 - lo.1) / cell).floor() as usize;
            let i1 = (((t.hi.0 - lo.0) / cell).floor() as usize).min(columns - 1);
            let j1 = (((t.hi.1 - lo.1) / cell).floor() as usize).min(rows - 1);
            for j in j0..=j1 {
                for i in i0..=i1 {
                    cells[j * columns + i].push(k as u32);
                }
            }
        }
        DropMesh {
            tris,
            origin: lo,
            cell,
            columns,
            rows,
            cells,
        }
    }

    /// The highest the tip of `cutter` must stay over `(x, y)`: None when no triangle comes
    /// within its radius.
    pub fn drop(&self, cutter: Cutter, (x, y): (f64, f64)) -> Option<f64> {
        if self.columns == 0 {
            return None;
        }
        let r = cutter.radius();
        let index = |v: f64, o: f64, n: usize| -> Option<usize> {
            let k = ((v - o) / self.cell).floor();
            if k < 0.0 {
                return Some(0);
            }
            let k = k as usize;
            (k < n).then_some(k).or(Some(n - 1))
        };
        // Cells under the tool; none when it lies wholly outside the grid.
        if x + r < self.origin.0
            || y + r < self.origin.1
            || x - r > self.origin.0 + self.cell * self.columns as f64
            || y - r > self.origin.1 + self.cell * self.rows as f64
        {
            return None;
        }
        let i0 = index(x - r, self.origin.0, self.columns)?;
        let i1 = index(x + r, self.origin.0, self.columns)?;
        let j0 = index(y - r, self.origin.1, self.rows)?;
        let j1 = index(y + r, self.origin.1, self.rows)?;
        let mut near: Vec<u32> = Vec::new();
        for j in j0..=j1 {
            for i in i0..=i1 {
                near.extend_from_slice(&self.cells[j * self.columns + i]);
            }
        }
        near.sort_unstable();
        near.dedup();
        // Highest triangles first: their answers rule out lower ones quickly.
        near.sort_by(|a, b| {
            self.tris[*b as usize]
                .top
                .total_cmp(&self.tris[*a as usize].top)
        });
        let mut best: Option<f64> = None;
        for k in near {
            let t = &self.tris[k as usize];
            if t.lo.0 > x + r || t.hi.0 < x - r || t.lo.1 > y + r || t.hi.1 < y - r {
                continue;
            }
            // The tip never stands above the triangle's top (the shape's height is ≥ 0).
            if best.is_some_and(|b| t.top <= b) {
                continue;
            }
            if let Some(z) = triangle_drop(t, cutter, (x, y)) {
                best = Some(best.map_or(z, |b| b.max(z)));
            }
        }
        best
    }
}

fn prepare(v: [V3; 3]) -> Tri {
    let (u, w) = (sub(v[1], v[0]), sub(v[2], v[0]));
    let n = [
        u[1] * w[2] - u[2] * w[1],
        u[2] * w[0] - u[0] * w[2],
        u[0] * w[1] - u[1] * w[0],
    ];
    let length = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
    // Upright (seen edge on from above): no face test; its edges and corners hold the tool.
    let plane = (length > 0.0 && n[2].abs() > UPRIGHT_SHARE * length).then(|| {
        let (a, b) = (-n[0] / n[2], -n[1] / n[2]);
        (a, b, v[0][2] - a * v[0][0] - b * v[0][1])
    });
    let xs = [v[0][0], v[1][0], v[2][0]];
    let ys = [v[0][1], v[1][1], v[2][1]];
    Tri {
        v,
        plane,
        lo: (min3(xs), min3(ys)),
        hi: (max3(xs), max3(ys)),
        top: v[0][2].max(v[1][2]).max(v[2][2]),
    }
}

/// A face whose normal leans further than this from upright, as a share of its length, is
/// taken as standing upright: its plane would be too steep to rest a tool on.
const UPRIGHT_SHARE: f64 = 1e-9;

fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn min3(v: [f64; 3]) -> f64 {
    v[0].min(v[1]).min(v[2])
}

fn max3(v: [f64; 3]) -> f64 {
    v[0].max(v[1]).max(v[2])
}

/// The highest the tip must stay over `c` for this one triangle; None if it lies beyond the
/// tool's reach.
fn triangle_drop(t: &Tri, cutter: Cutter, c: (f64, f64)) -> Option<f64> {
    let mut best: Option<f64> = None;
    let mut take = |z: f64| best = Some(best.map_or(z, |b: f64| b.max(z)));
    for v in &t.v {
        let d = (v[0] - c.0).hypot(v[1] - c.1);
        if let Some(h) = cutter.height_at(d) {
            take(v[2] - h);
        }
    }
    if let Some(z) = face_drop(t, cutter, c) {
        take(z);
    }
    for (a, b) in [(t.v[0], t.v[1]), (t.v[1], t.v[2]), (t.v[2], t.v[0])] {
        if let Some(z) = edge_drop(a, b, cutter, c) {
            take(z);
        }
    }
    best
}

/// The face test: the tool rests on the triangle's plane at the distance `r` from its axis,
/// uphill, where `r·slope − h(r)` is highest (its candidates: the axis, the shape's kinks,
/// its level point, its rim), if that point lies inside the triangle.
fn face_drop(t: &Tri, cutter: Cutter, c: (f64, f64)) -> Option<f64> {
    let (a, b, k) = t.plane?;
    let slope = a.hypot(b);
    let radius = cutter.radius();
    let mut candidates = vec![0.0, radius];
    candidates.extend(cutter.kinks());
    candidates.extend(cutter.where_slope(slope));
    let (r, reach) = candidates
        .into_iter()
        .filter(|r| (0.0..=radius).contains(r))
        .filter_map(|r| cutter.height_at(r).map(|h| (r, r * slope - h)))
        .reduce(|x, y| if y.1 > x.1 { y } else { x })?;
    let (ux, uy) = if slope > 0.0 {
        (a / slope, b / slope)
    } else {
        (0.0, 0.0)
    };
    let contact = (c.0 + r * ux, c.1 + r * uy);
    inside(&t.v, contact).then_some(a * c.0 + b * c.1 + k + reach)
}

/// Whether `q` lies in the triangle seen from above, its edges included, either way round.
fn inside(v: &[V3; 3], q: (f64, f64)) -> bool {
    let side = |a: V3, b: V3| (b[0] - a[0]) * (q.1 - a[1]) - (b[1] - a[1]) * (q.0 - a[0]);
    let (s0, s1, s2) = (side(v[0], v[1]), side(v[1], v[2]), side(v[2], v[0]));
    (s0 >= 0.0 && s1 >= 0.0 && s2 >= 0.0) || (s0 <= 0.0 && s1 <= 0.0 && s2 <= 0.0)
}

/// The edge test for the segment `a`–`b` (module comment, point 3).
fn edge_drop(a: V3, b: V3, cutter: Cutter, c: (f64, f64)) -> Option<f64> {
    let radius = cutter.radius();
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let length = dx.hypot(dy);
    // Upright: its ends are the corners, already tested.
    if length <= CHAIN_TOL_MM {
        return None;
    }
    let u = (dx / length, dy / length);
    let w = (c.0 - a[0], c.1 - a[1]);
    let foot = w.0 * u.0 + w.1 * u.1;
    let off = (w.0 * u.1 - w.1 * u.0).abs();
    if off > radius {
        return None;
    }
    let half = (radius * radius - off * off).sqrt();
    let (s0, s1) = ((-foot).max(-half), (length - foot).min(half));
    if s0 > s1 {
        return None;
    }
    let m = (b[2] - a[2]) / length;
    let reach = |s: f64| {
        let d = off.hypot(s).min(radius);
        cutter.height_at(d).map(|h| a[2] + m * (s + foot) - h)
    };
    let mut candidates = vec![s0, s1, 0.0];
    for k in cutter.kinks() {
        if k > off {
            let s = (k * k - off * off).sqrt();
            candidates.extend([s, -s]);
        }
    }
    candidates.extend(cutter.level_along(m, off));
    candidates
        .into_iter()
        .filter(|s| (s0..=s1).contains(s))
        .filter_map(reach)
        .reduce(f64::max)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn cutters() -> Vec<Cutter> {
        vec![
            Cutter::Flat { radius: 3.0 },
            Cutter::Ball { radius: 3.0 },
            Cutter::Cone {
                radius: 6.0,
                tip: 0.2,
                half_angle: 30f64.to_radians(),
            },
            Cutter::TaperedBall {
                radius: 3.175,
                ball: 0.8,
                half_angle: 10f64.to_radians(),
            },
        ]
    }

    /// The reference: the highest tip over `c` that keeps every sampled point of every
    /// triangle out of the tool. Sampling misses the true contact by a little, so the
    /// reference is a little low.
    /// With `grow`, points up to that far beyond the rim count too, at the rim's height: an
    /// upper reference that catches contacts at the very rim, which sampling misses.
    fn sampled(mesh: &Mesh, cutter: Cutter, c: (f64, f64), n: usize, grow: f64) -> Option<f64> {
        let radius = cutter.radius();
        let mut best: Option<f64> = None;
        for t in &mesh.triangles {
            for i in 0..=n {
                for j in 0..=n - i {
                    let (p, q) = (i as f64 / n as f64, j as f64 / n as f64);
                    let r = 1.0 - p - q;
                    let point = [
                        r * t[0][0] + p * t[1][0] + q * t[2][0],
                        r * t[0][1] + p * t[1][1] + q * t[2][1],
                        r * t[0][2] + p * t[1][2] + q * t[2][2],
                    ];
                    let d = (point[0] - c.0).hypot(point[1] - c.1);
                    let h = if d <= radius + grow {
                        cutter.height_at(d.min(radius))
                    } else {
                        None
                    };
                    if let Some(h) = h {
                        let z = point[2] - h;
                        best = Some(best.map_or(z, |b: f64| b.max(z)));
                    }
                }
            }
        }
        best
    }

    #[test]
    fn a_flat_floor_holds_every_tool_at_its_height() {
        let mesh = Mesh {
            triangles: vec![
                [[0.0, 0.0, -2.0], [20.0, 0.0, -2.0], [20.0, 20.0, -2.0]],
                [[0.0, 0.0, -2.0], [20.0, 20.0, -2.0], [0.0, 20.0, -2.0]],
            ],
        };
        let index = DropMesh::new(&mesh, 6.0);
        for cutter in cutters() {
            let z = index.drop(cutter, (10.0, 10.0)).unwrap();
            assert!((z + 2.0).abs() < 1e-12, "{cutter:?}: {z}");
        }
        assert_eq!(index.drop(Cutter::Ball { radius: 3.0 }, (50.0, 50.0)), None);
    }

    #[test]
    fn a_ball_on_a_slope_rests_its_radius_off_the_plane() {
        // A plane rising 45° along X: a ball's centre stays r·√2 above it, its tip r(√2 − 1).
        let mesh = Mesh {
            triangles: vec![
                [
                    [-50.0, -50.0, -50.0],
                    [50.0, -50.0, 50.0],
                    [50.0, 50.0, 50.0],
                ],
                [
                    [-50.0, -50.0, -50.0],
                    [50.0, 50.0, 50.0],
                    [-50.0, 50.0, -50.0],
                ],
            ],
        };
        let index = DropMesh::new(&mesh, 6.0);
        let z = index
            .drop(Cutter::Ball { radius: 3.0 }, (0.0, 0.0))
            .unwrap();
        assert!((z - 3.0 * (2f64.sqrt() - 1.0)).abs() < 1e-9, "{z}");
        // A flat end mill rests on its rim, uphill: 3 mm up.
        let z = index
            .drop(Cutter::Flat { radius: 3.0 }, (0.0, 0.0))
            .unwrap();
        assert!((z - 3.0).abs() < 1e-9, "{z}");
    }

    #[test]
    fn a_ridge_holds_a_ball_on_its_edge() {
        // Two faces meeting in a ridge along Y at x = 0, z = 0, falling 1 in 2 each way.
        let mesh = Mesh {
            triangles: vec![
                [
                    [0.0, -20.0, 0.0],
                    [-20.0, -20.0, -10.0],
                    [-20.0, 20.0, -10.0],
                ],
                [[0.0, -20.0, 0.0], [-20.0, 20.0, -10.0], [0.0, 20.0, 0.0]],
                [[0.0, -20.0, 0.0], [0.0, 20.0, 0.0], [20.0, 20.0, -10.0]],
                [[0.0, -20.0, 0.0], [20.0, 20.0, -10.0], [20.0, -20.0, -10.0]],
            ],
        };
        let index = DropMesh::new(&mesh, 6.0);
        // Straight over the ridge, a ball sits on the edge: tip at 0.
        let z = index
            .drop(Cutter::Ball { radius: 3.0 }, (0.0, 0.0))
            .unwrap();
        assert!(z.abs() < 1e-9, "{z}");
        // 1 mm to the side it still touches the edge: z = −(3 − √(9 − 1)).
        let z = index
            .drop(Cutter::Ball { radius: 3.0 }, (1.0, 0.0))
            .unwrap();
        assert!((z + 3.0 - 8f64.sqrt()).abs() < 1e-9, "{z}");
    }

    fn triangle() -> impl Strategy<Value = [V3; 3]> {
        let corner = (-10.0..10.0f64, -10.0..10.0f64, -6.0..0.0f64).prop_map(|(x, y, z)| [x, y, z]);
        [corner.clone(), corner.clone(), corner]
    }

    proptest! {
        /// The drop cutter against sampling: never below the sampled reference (it would
        /// cut a sampled point), and not far above it (it would leave the tool hanging).
        #[test]
        fn the_drop_agrees_with_sampling(
            tris in proptest::collection::vec(triangle(), 1..4),
            k in 0usize..4,
            x in -8.0..8.0f64,
            y in -8.0..8.0f64,
        ) {
            let mesh = Mesh { triangles: tris };
            let cutter = cutters()[k];
            let exact = DropMesh::new(&mesh, 6.0).drop(cutter, (x, y));
            let below = sampled(&mesh, cutter, (x, y), 60, 0.0);
            let above = sampled(&mesh, cutter, (x, y), 60, 0.4);
            match (exact, below) {
                (Some(e), Some(r)) => prop_assert!(e >= r - 1e-9, "{cutter:?}: {e} below {r}"),
                (None, Some(r)) => prop_assert!(false, "{cutter:?}: missed {r}"),
                _ => {}
            }
            // Not hanging: never above what a slightly wider tool would touch, sampled every
            // 1/60 of a triangle up to 28 mm long, by that spacing times the steepest slope.
            if let Some(e) = exact {
                let u = above.unwrap_or(f64::NEG_INFINITY);
                prop_assert!(e <= u + 0.6, "{cutter:?}: {e} far above {u}");
            }
        }
    }
}
