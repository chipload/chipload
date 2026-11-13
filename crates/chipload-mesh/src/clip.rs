// SPDX-License-Identifier: GPL-3.0-or-later
//! A mesh cut at a level: the part above it kept, the rest gone (docs/specs/relief-wizard.md
//! §5). Each triangle is clipped against the plane (Sutherland–Hodgman against one plane):
//! wholly above it stays, wholly below it goes, across it is cut along the plane, so a
//! model cut through keeps its whole outline.

use crate::{Mesh, Vertex};

impl Mesh {
    /// The part of the mesh at or above Z `level`, each triangle cut where it crosses the
    /// level; the corners made on the cut lie exactly at `level`. Triangles keep their
    /// order and their turn.
    pub fn above(&self, level: f64) -> Mesh {
        let mut triangles = Vec::with_capacity(self.triangles.len());
        for t in &self.triangles {
            clip(t, level, &mut triangles);
        }
        Mesh { triangles }
    }
}

/// Where edge `a`–`b` crosses Z `level` (`a` above it, `b` below).
fn crossing(a: Vertex, b: Vertex, level: f64) -> Vertex {
    let t = (a[2] - level) / (a[2] - b[2]);
    [a[0] + t * (b[0] - a[0]), a[1] + t * (b[1] - a[1]), level]
}

/// Triangle `t` clipped to Z ≥ `level`, its pieces pushed onto `out`.
fn clip(t: &[Vertex; 3], level: f64, out: &mut Vec<[Vertex; 3]>) {
    let above = [t[0][2] >= level, t[1][2] >= level, t[2][2] >= level];
    let count = above.iter().filter(|&&a| a).count();
    match count {
        3 => out.push(*t),
        0 => {}
        _ => {
            // The polygon of the triangle above the level, corners in the triangle's turn.
            let mut polygon: Vec<Vertex> = Vec::with_capacity(4);
            for i in 0..3 {
                let (a, b) = (t[i], t[(i + 1) % 3]);
                let (in_a, in_b) = (above[i], above[(i + 1) % 3]);
                if in_a {
                    polygon.push(a);
                }
                if in_a != in_b {
                    let point = if in_a {
                        crossing(a, b, level)
                    } else {
                        crossing(b, a, level)
                    };
                    polygon.push(point);
                }
            }
            // A triangle (one corner above) or a quad (two): a fan from the first corner.
            for k in 1..polygon.len().saturating_sub(1) {
                out.push([polygon[0], polygon[k], polygon[k + 1]]);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area(t: &[Vertex; 3]) -> [f64; 3] {
        let (u, v) = (
            [t[1][0] - t[0][0], t[1][1] - t[0][1], t[1][2] - t[0][2]],
            [t[2][0] - t[0][0], t[2][1] - t[0][1], t[2][2] - t[0][2]],
        );
        [
            (u[1] * v[2] - u[2] * v[1]) / 2.0,
            (u[2] * v[0] - u[0] * v[2]) / 2.0,
            (u[0] * v[1] - u[1] * v[0]) / 2.0,
        ]
    }

    #[test]
    fn whole_triangles_stay_or_go() {
        let high = [[0.0, 0.0, 5.0], [1.0, 0.0, 5.0], [0.0, 1.0, 6.0]];
        let low = [[0.0, 0.0, 1.0], [1.0, 0.0, 1.0], [0.0, 1.0, 2.0]];
        let m = Mesh {
            triangles: vec![high, low],
        };
        assert_eq!(m.above(3.0).triangles, vec![high]);
        // On the level counts as above.
        assert_eq!(m.above(5.0).triangles, vec![high]);
    }

    #[test]
    fn a_triangle_across_the_level_is_cut_on_it_and_keeps_its_turn() {
        // Standing upright in the XZ plane, from Z 0 to 10.
        let t = [[0.0, 0.0, 0.0], [10.0, 0.0, 0.0], [0.0, 0.0, 10.0]];
        let one = Mesh { triangles: vec![t] }.above(5.0);
        assert_eq!(one.triangles.len(), 1);
        for v in one.triangles.iter().flatten() {
            assert!(v[2] >= 5.0);
        }
        // Two corners above: a quad in two triangles.
        let t2 = [[0.0, 0.0, 10.0], [10.0, 0.0, 10.0], [0.0, 0.0, 0.0]];
        let two = Mesh {
            triangles: vec![t2],
        }
        .above(5.0);
        assert_eq!(two.triangles.len(), 2);
        // The pieces face as the triangle did, and add up to the part above: the full
        // triangle's area is 50, the part below 5 (a triangle of half its size squared:
        // 50/4 = 12.5 below), so 37.5 above.
        let whole = area(&t2);
        let pieces: f64 = two.triangles.iter().map(|p| area(p)[1]).sum();
        assert!(whole[1].signum() == pieces.signum());
        assert!((pieces.abs() - 37.5).abs() < 1e-9, "{pieces}");
    }

    #[test]
    fn a_closed_box_cut_in_half_keeps_its_outline() {
        // A cube 0..10 as 12 triangles.
        let c = |x: f64, y: f64, z: f64| [x * 10.0, y * 10.0, z * 10.0];
        let quads = [
            [c(0., 0., 0.), c(1., 0., 0.), c(1., 1., 0.), c(0., 1., 0.)],
            [c(0., 0., 1.), c(0., 1., 1.), c(1., 1., 1.), c(1., 0., 1.)],
            [c(0., 0., 0.), c(0., 0., 1.), c(1., 0., 1.), c(1., 0., 0.)],
            [c(0., 1., 0.), c(1., 1., 0.), c(1., 1., 1.), c(0., 1., 1.)],
            [c(0., 0., 0.), c(0., 1., 0.), c(0., 1., 1.), c(0., 0., 1.)],
            [c(1., 0., 0.), c(1., 0., 1.), c(1., 1., 1.), c(1., 1., 0.)],
        ];
        let triangles = quads
            .iter()
            .flat_map(|q| [[q[0], q[1], q[2]], [q[0], q[2], q[3]]])
            .collect();
        let cut = Mesh { triangles }.above(4.0);
        let (lo, hi) = cut.bounds().unwrap();
        assert_eq!((lo, hi), ([0.0, 0.0, 4.0], [10.0, 10.0, 10.0]));
    }
}
