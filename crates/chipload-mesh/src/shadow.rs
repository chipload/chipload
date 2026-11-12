// SPDX-License-Identifier: GPL-3.0-or-later
//! The shadow a mesh casts seen from above (spec §4): a grid of cells, each covered or not,
//! from which the model's footprint outline is traced.

use chipload_core::tolerances::MESH_DEGENERATE_SHARE;

use crate::Mesh;

/// Covered cells of a square grid. Row 0 is the lowest; a ring of empty cells surrounds the
/// shadow, so every outline traced from it closes.
#[derive(Debug, Clone, PartialEq)]
pub struct Shadow {
    /// The lower-left corner of cell (0, 0).
    pub origin: (f64, f64),
    /// A cell's side.
    pub cell: f64,
    pub columns: usize,
    pub rows: usize,
    /// Row by row from the lowest, `columns` each.
    pub covered: Vec<bool>,
}

impl Shadow {
    pub fn at(&self, column: usize, row: usize) -> bool {
        self.covered[row * self.columns + column]
    }
}

impl Mesh {
    /// The shadow seen from above, in cells of `cell`: a cell is covered when its centre
    /// lies in a triangle seen from above, or a corner lies in it (so no triangle is lost
    /// between centres). None for no triangles, a cell that isn't positive, or more than
    /// `max_cells` cells.
    pub fn shadow(&self, cell: f64, max_cells: usize) -> Option<Shadow> {
        let (lo, hi) = self.bounds()?;
        if !(cell.is_finite() && cell > 0.0) {
            return None;
        }
        let origin = (lo[0] - cell, lo[1] - cell);
        let columns = ((hi[0] - lo[0]) / cell).ceil() as usize + 3;
        let rows = ((hi[1] - lo[1]) / cell).ceil() as usize + 3;
        if columns.checked_mul(rows)? > max_cells {
            return None;
        }
        let mut covered = vec![false; columns * rows];
        let index = |x: f64, y: f64| -> (usize, usize) {
            let i = ((x - origin.0) / cell)
                .floor()
                .clamp(0.0, (columns - 1) as f64);
            let j = ((y - origin.1) / cell)
                .floor()
                .clamp(0.0, (rows - 1) as f64);
            (i as usize, j as usize)
        };
        for t in &self.triangles {
            let p = [(t[0][0], t[0][1]), (t[1][0], t[1][1]), (t[2][0], t[2][1])];
            for &(x, y) in &p {
                let (i, j) = index(x, y);
                covered[j * columns + i] = true;
            }
            let x0 = p[0].0.min(p[1].0).min(p[2].0);
            let y0 = p[0].1.min(p[1].1).min(p[2].1);
            let x1 = p[0].0.max(p[1].0).max(p[2].0);
            let y1 = p[0].1.max(p[1].1).max(p[2].1);
            let (i0, j0) = index(x0, y0);
            let (i1, j1) = index(x1, y1);
            for j in j0..=j1 {
                for i in i0..=i1 {
                    let centre = (
                        origin.0 + (i as f64 + 0.5) * cell,
                        origin.1 + (j as f64 + 0.5) * cell,
                    );
                    if inside(p, centre) {
                        covered[j * columns + i] = true;
                    }
                }
            }
        }
        Some(Shadow {
            origin,
            cell,
            columns,
            rows,
            covered,
        })
    }
}

/// The top of a mesh seen from above: the highest point of its surface over each cell's
/// centre, None where no triangle lies over it. Row 0 is the lowest.
#[derive(Debug, Clone, PartialEq)]
pub struct TopHeights {
    /// The lower-left corner of cell (0, 0).
    pub origin: (f64, f64),
    pub cell: f64,
    pub columns: usize,
    pub rows: usize,
    /// Row by row from the lowest, `columns` each.
    pub heights: Vec<Option<f64>>,
}

impl Mesh {
    /// The top surface over a grid of square cells across the mesh's bounds, about `along`
    /// cells along its longer side, for showing the model (docs/specs/stl-relief.md §9).
    /// None for no triangles or `along` 0.
    pub fn top_heights(&self, along: usize) -> Option<TopHeights> {
        let (lo, hi) = self.bounds()?;
        let longer = (hi[0] - lo[0]).max(hi[1] - lo[1]);
        if along == 0 || !(longer.is_finite() && longer > 0.0) {
            return None;
        }
        let cell = longer / along as f64;
        let columns = ((hi[0] - lo[0]) / cell).ceil().max(1.0) as usize;
        let rows = ((hi[1] - lo[1]) / cell).ceil().max(1.0) as usize;
        let origin = (lo[0], lo[1]);
        let mut heights: Vec<Option<f64>> = vec![None; columns * rows];
        let index =
            |v: f64, o: f64, n: usize| ((v - o) / cell).floor().clamp(0.0, (n - 1) as f64) as usize;
        for t in &self.triangles {
            let p = [(t[0][0], t[0][1]), (t[1][0], t[1][1]), (t[2][0], t[2][1])];
            let (i0, i1) = (
                index(p[0].0.min(p[1].0).min(p[2].0), origin.0, columns),
                index(p[0].0.max(p[1].0).max(p[2].0), origin.0, columns),
            );
            let (j0, j1) = (
                index(p[0].1.min(p[1].1).min(p[2].1), origin.1, rows),
                index(p[0].1.max(p[1].1).max(p[2].1), origin.1, rows),
            );
            for j in j0..=j1 {
                for i in i0..=i1 {
                    let q = (
                        origin.0 + (i as f64 + 0.5) * cell,
                        origin.1 + (j as f64 + 0.5) * cell,
                    );
                    if let Some(z) = height_in(t, q) {
                        let at = &mut heights[j * columns + i];
                        *at = Some(at.map_or(z, |h: f64| h.max(z)));
                    }
                }
            }
        }
        Some(TopHeights {
            origin,
            cell,
            columns,
            rows,
            heights,
        })
    }
}

/// The height of triangle `t` over `q`, when `q` lies in it seen from above; None for a
/// triangle seen edge on.
fn height_in(t: &[[f64; 3]; 3], q: (f64, f64)) -> Option<f64> {
    let (a, b, c) = (t[0], t[1], t[2]);
    // Twice the area seen from above; a wall seen edge on has none to speak of.
    let det = (b[1] - c[1]) * (a[0] - c[0]) + (c[0] - b[0]) * (a[1] - c[1]);
    let longest = [(a, b), (b, c), (c, a)]
        .iter()
        .map(|(p, q)| (q[0] - p[0]).hypot(q[1] - p[1]))
        .fold(0.0, f64::max);
    if det.abs() <= (MESH_DEGENERATE_SHARE * longest).powi(2) || longest == 0.0 {
        return None;
    }
    let u = ((b[1] - c[1]) * (q.0 - c[0]) + (c[0] - b[0]) * (q.1 - c[1])) / det;
    let v = ((c[1] - a[1]) * (q.0 - c[0]) + (a[0] - c[0]) * (q.1 - c[1])) / det;
    let w = 1.0 - u - v;
    (u >= 0.0 && v >= 0.0 && w >= 0.0).then(|| u * a[2] + v * b[2] + w * c[2])
}

/// Whether `q` lies in the triangle `p` seen from above, its edges included, whichever way
/// round its corners go. A triangle seen edge on (a wall) covers nothing by its centre
/// test; its corners still mark their cells.
fn inside(p: [(f64, f64); 3], q: (f64, f64)) -> bool {
    let side = |a: (f64, f64), b: (f64, f64)| (b.0 - a.0) * (q.1 - a.1) - (b.1 - a.1) * (q.0 - a.0);
    let (s0, s1, s2) = (side(p[0], p[1]), side(p[1], p[2]), side(p[2], p[0]));
    (s0 >= 0.0 && s1 >= 0.0 && s2 >= 0.0) || (s0 <= 0.0 && s1 <= 0.0 && s2 <= 0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_square_casts_a_square_shadow_with_an_empty_ring() {
        let square = Mesh {
            triangles: vec![
                [[0.0, 0.0, 1.0], [10.0, 0.0, 1.0], [10.0, 10.0, 1.0]],
                [[0.0, 0.0, 1.0], [10.0, 10.0, 1.0], [0.0, 10.0, 1.0]],
            ],
        };
        let s = square.shadow(1.0, 10_000).unwrap();
        assert_eq!((s.columns, s.rows), (13, 13));
        let count = s.covered.iter().filter(|c| **c).count();
        // 10 × 10 centres, and the corners' cells on the far edges.
        assert!((100..=121).contains(&count), "{count}");
        for k in 0..s.columns {
            assert!(!s.at(k, 0) && !s.at(k, s.rows - 1) && !s.at(0, k) && !s.at(s.columns - 1, k));
        }
        assert!(square.shadow(0.001, 10_000).is_none(), "too many cells");
    }

    #[test]
    fn the_top_is_the_highest_surface_over_each_cell() {
        // A slope from z 0 at x 0 to z 10 at x 10, and below it a floor at z -5.
        let mesh = Mesh {
            triangles: vec![
                [[0.0, 0.0, 0.0], [10.0, 0.0, 10.0], [10.0, 10.0, 10.0]],
                [[0.0, 0.0, 0.0], [10.0, 10.0, 10.0], [0.0, 10.0, 0.0]],
                [[0.0, 0.0, -5.0], [10.0, 0.0, -5.0], [10.0, 10.0, -5.0]],
            ],
        };
        let top = mesh.top_heights(10).unwrap();
        assert_eq!((top.columns, top.rows, top.cell), (10, 10, 1.0));
        for j in 0..10 {
            for i in 0..10 {
                let z = top.heights[j * 10 + i].unwrap();
                assert!((z - (i as f64 + 0.5)).abs() < 1e-9, "{i} {j}: {z}");
            }
        }
        assert!(mesh.top_heights(0).is_none());
    }
}
