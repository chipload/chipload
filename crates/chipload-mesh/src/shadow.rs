// SPDX-License-Identifier: GPL-3.0-or-later
//! The shadow a mesh casts seen from above (spec §4): a grid of cells, each covered or not,
//! from which the model's footprint outline is traced.

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
}
