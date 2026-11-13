// SPDX-License-Identifier: GPL-3.0-or-later
//! A grid of heights as a top surface (docs/specs/picture-relief.md §3): what a picture
//! read as a depth map becomes, so the relief cuts it as it cuts an STL.

use crate::{Mesh, Vertex};

impl Mesh {
    /// The top surface over a grid of `columns` × `rows` heights, row 0 the lowest, each
    /// cell `cell` wide: each height a corner at its cell's centre, and two triangles
    /// between every four neighbouring corners that all have one (None: no model there).
    /// Empty when no four do.
    pub fn height_field(columns: usize, rows: usize, cell: f64, heights: &[Option<f64>]) -> Mesh {
        let at = |c: usize, r: usize| -> Option<Vertex> {
            let z = (*heights.get(r * columns + c)?)?;
            Some([(c as f64 + 0.5) * cell, (r as f64 + 0.5) * cell, z])
        };
        let mut triangles = Vec::new();
        for r in 0..rows.saturating_sub(1) {
            for c in 0..columns.saturating_sub(1) {
                let corners = (at(c, r), at(c + 1, r), at(c + 1, r + 1), at(c, r + 1));
                if let (Some(a), Some(b), Some(d), Some(e)) = corners {
                    // Counter-clockwise seen from above: facing up.
                    triangles.push([a, b, d]);
                    triangles.push([a, d, e]);
                }
            }
        }
        Mesh { triangles }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_grid_becomes_two_triangles_a_cell_where_all_four_corners_are() {
        // 3 × 2 corners, one missing: only the left cell is whole.
        let heights = [Some(0.0), Some(1.0), Some(2.0), Some(0.0), Some(1.0), None];
        let mesh = Mesh::height_field(3, 2, 2.0, &heights);
        assert_eq!(mesh.triangles.len(), 2);
        let (lo, hi) = mesh.bounds().unwrap();
        assert_eq!((lo, hi), ([1.0, 1.0, 0.0], [3.0, 3.0, 1.0]));
        // Facing up: each triangle counter-clockwise from above.
        for [a, b, c] in &mesh.triangles {
            let cross = (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]);
            assert!(cross > 0.0);
        }
        assert!(
            Mesh::height_field(1, 1, 1.0, &[Some(0.0)])
                .triangles
                .is_empty()
        );
    }
}
