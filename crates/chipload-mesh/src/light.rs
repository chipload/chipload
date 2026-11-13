// SPDX-License-Identifier: GPL-3.0-or-later
//! A light copy of a mesh for showing it (docs/specs/relief-wizard.md §3): at most a given
//! number of triangles, whatever the file, made by vertex clustering (J. Rossignac and
//! P. Borrel, "Multi-resolution 3D approximations for rendering complex scenes", 1993).
//! A grid of cubes over the mesh's box; the corners in each cube merge into their mean;
//! a triangle whose corners fall into fewer than three cubes goes, and so does a second
//! triangle on the same three cubes. Only for seeing: nothing is cut from it.

use std::collections::{HashMap, HashSet};

use crate::{Mesh, Vertex};

/// A light copy has at most this many triangles (spec decision 8).
pub const LIGHT_TRIANGLES: usize = 50_000;

/// Tries at most this many grids to find the finest within the limit.
const MOST_TRIES: usize = 8;

/// A grid finer than this many cubes along the longest side is never tried.
const MOST_CUBES: f64 = 4096.0;

/// Below this share of the limit, a finer grid is tried.
const FILL_WANTED: f64 = 0.7;

/// Each new grid aims this far below the limit, so the next try rarely overshoots.
const AIM: f64 = 0.95;

impl Mesh {
    /// A copy with at most `most` triangles, for showing: the mesh itself when it has no
    /// more; otherwise its corners clustered on the finest cube grid found (in a few tries)
    /// that gives no more. The same mesh always gives the same copy.
    pub fn light(&self, most: usize) -> Mesh {
        if self.triangles.len() <= most || most == 0 {
            return self.clone();
        }
        let Some((lo, hi)) = self.bounds() else {
            return self.clone();
        };
        let longest = (hi[0] - lo[0]).max(hi[1] - lo[1]).max(hi[2] - lo[2]);
        if longest <= 0.0 {
            return Mesh::default();
        }
        // A surface of N triangles has about N/2 corners; a grid of n cubes along its
        // longest side meets about n² of them on an ordinary model.
        let mut cubes = ((most as f64) / 2.0).sqrt().clamp(1.0, MOST_CUBES);
        let mut best: Option<Mesh> = None;
        for _ in 0..MOST_TRIES {
            let copy = clustered(self, lo, longest / cubes);
            let count = copy.triangles.len();
            let fits = count <= most;
            let better = fits && best.as_ref().is_none_or(|b| count > b.triangles.len());
            if better {
                best = Some(copy);
            }
            let full_enough = fits && count as f64 >= FILL_WANTED * most as f64;
            if full_enough || (fits && cubes >= MOST_CUBES) {
                break;
            }
            let factor = if count == 0 {
                2.0
            } else {
                (AIM * most as f64 / count as f64).sqrt()
            };
            let next = (cubes * factor).clamp(1.0, MOST_CUBES);
            if next == cubes {
                break;
            }
            cubes = next;
        }
        // Always one within the limit: a single cube gives no triangles at all.
        best.unwrap_or_else(|| clustered(self, lo, longest))
    }
}

/// `mesh` with its corners clustered on cubes of side `cube` from `lo`.
fn clustered(mesh: &Mesh, lo: Vertex, cube: f64) -> Mesh {
    let index = |v: f64, from: f64| -> i64 { ((v - from) / cube).floor() as i64 };
    // Cube → its cluster, numbered as first met; each cluster's corner sum and count.
    let mut clusters: HashMap<[i64; 3], usize> = HashMap::new();
    let mut sums: Vec<(Vertex, f64)> = Vec::new();
    let mut cluster_of = |v: &Vertex| -> usize {
        let key = [index(v[0], lo[0]), index(v[1], lo[1]), index(v[2], lo[2])];
        let next = sums.len();
        let k = *clusters.entry(key).or_insert(next);
        if k == next {
            sums.push(([0.0; 3], 0.0));
        }
        let (sum, count) = &mut sums[k];
        sum[0] += v[0];
        sum[1] += v[1];
        sum[2] += v[2];
        *count += 1.0;
        k
    };
    let mut kept: Vec<[usize; 3]> = Vec::new();
    let mut seen: HashSet<[usize; 3]> = HashSet::new();
    for t in &mesh.triangles {
        let c = [cluster_of(&t[0]), cluster_of(&t[1]), cluster_of(&t[2])];
        if c[0] == c[1] || c[1] == c[2] || c[0] == c[2] {
            continue;
        }
        // The same three cubes in the same turn: one triangle is enough.
        let first = (0..3).min_by_key(|&i| c[i]).unwrap_or(0);
        let key = [c[first], c[(first + 1) % 3], c[(first + 2) % 3]];
        if seen.insert(key) {
            kept.push(c);
        }
    }
    let mean = |k: usize| -> Vertex {
        let (sum, count) = sums[k];
        [sum[0] / count, sum[1] / count, sum[2] / count]
    };
    Mesh {
        triangles: kept
            .iter()
            .map(|c| [mean(c[0]), mean(c[1]), mean(c[2])])
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A closed sphere of radius `r` with `n` × `n` / 2 quads, as triangles.
    fn sphere(r: f64, n: usize) -> Mesh {
        let at = |i: usize, j: usize| -> Vertex {
            let u = std::f64::consts::TAU * i as f64 / n as f64;
            let v = std::f64::consts::PI * j as f64 / (n / 2) as f64;
            [r * u.cos() * v.sin(), r * u.sin() * v.sin(), r * v.cos()]
        };
        let mut triangles = Vec::new();
        for i in 0..n {
            for j in 0..n / 2 {
                let (a, b, c, d) = (at(i, j), at(i + 1, j), at(i + 1, j + 1), at(i, j + 1));
                triangles.push([a, b, c]);
                triangles.push([a, c, d]);
            }
        }
        Mesh { triangles }
    }

    #[test]
    fn a_small_mesh_is_its_own_light_copy() {
        let m = sphere(10.0, 20);
        assert_eq!(m.light(LIGHT_TRIANGLES), m);
    }

    #[test]
    fn a_heavy_mesh_comes_within_the_limit_and_keeps_its_shape() {
        let m = sphere(50.0, 600); // 180 000 triangles, some with no area at the poles
        let light = m.light(20_000);
        let n = light.triangles.len();
        assert!(n <= 20_000, "{n}");
        assert!(n >= 10_000, "far fewer than the limit: {n}");
        // Its box within a cube of the original's: the cube is at most 100 mm / 20.
        let ((lo, hi), (llo, lhi)) = (m.bounds().unwrap(), light.bounds().unwrap());
        for k in 0..3 {
            assert!((lo[k] - llo[k]).abs() < 5.0 && (hi[k] - lhi[k]).abs() < 5.0);
        }
        // Every corner within a cube's diagonal of the sphere.
        let cube_diagonal = 100.0 / ((20_000.0f64 / 2.0).sqrt()) * 3f64.sqrt();
        for v in light.triangles.iter().flatten() {
            let r = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
            assert!((r - 50.0).abs() < cube_diagonal, "{r}");
        }
        // Seen from above, the same top within a cube or two.
        let full = m.top_heights(50).unwrap();
        let top = light.top_heights(50).unwrap();
        let (cx, cy) = (full.columns / 2, full.rows / 2);
        let middle = |t: &crate::TopHeights| t.heights[cy * t.columns + cx].unwrap();
        assert!((middle(&full) - middle(&top)).abs() < 2.0 * cube_diagonal);
    }

    #[test]
    fn the_same_mesh_gives_the_same_copy() {
        let m = sphere(30.0, 400);
        assert_eq!(m.light(5_000), m.light(5_000));
    }

    #[test]
    fn a_flat_or_empty_mesh_is_safe() {
        assert_eq!(Mesh::default().light(10), Mesh::default());
        let flat = Mesh {
            triangles: vec![[[0.0; 3], [0.0; 3], [0.0; 3]]; 20],
        };
        assert!(flat.light(10).triangles.is_empty());
    }
}
