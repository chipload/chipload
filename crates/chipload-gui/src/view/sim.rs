// SPDX-License-Identifier: GPL-3.0-or-later
//! The simulation's surface and the pictures shown under the drawing.

use chipload_core::Point;
use chipload_core::math;
use eframe::egui::{Color32, Mesh, Painter, Pos2, Rect, Shape};

use super::{GHOST_FADE, Point3, View};

/// The simulated stock, ready to paint (docs/specs/simulation.md §4): an image for the top
/// view (its texture kept alive by the app) and a coarser surface for 3D.
#[derive(Debug, Clone)]
pub struct SimLayer {
    pub texture: eframe::egui::TextureId,
    /// The grid's lower left and upper right corners, drawing coordinates.
    pub min: Point,
    pub max: Point,
    pub surface: SimSurface,
}

/// A height map as a surface: one vertex per cell centre, row 0 at the lowest Y. A solid
/// (docs/specs/two-sided.md §10.2): cells cut through are left out, so holes are holes, and
/// walls go down to the floors where the surface ends, at the board's edges and round holes.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SimSurface {
    pub origin: Point,
    pub cell: f64,
    pub columns: usize,
    pub rows: usize,
    pub heights: Vec<f32>,
    /// The bottom of the material under each cell.
    pub floors: Vec<f32>,
    pub colors: Vec<Color32>,
    /// The walls' colour.
    pub wall: Color32,
    /// Its points are already as the board lies turned over (the turned simulation): shown
    /// without turning them again.
    pub raw: bool,
}

/// One quad to draw: how far into the screen it lies, and its four vertices.
type Quad = (f64, [u32; 4]);

impl SimSurface {
    /// Cell `k` has no material left: its floor reaches its height.
    fn through(&self, k: usize) -> bool {
        f64::from(self.floors[k]) + chipload_core::tolerances::DEPTH_TOL_MM
            >= f64::from(self.heights[k])
    }

    /// The quad with lower-left vertex (i, j) is drawn: inside the grid, no corner cut
    /// through.
    pub(super) fn kept(&self, i: isize, j: isize) -> bool {
        let (columns, rows) = (self.columns as isize, self.rows as isize);
        if i < 0 || j < 0 || i >= columns - 1 || j >= rows - 1 {
            return false;
        }
        let k = (j * columns + i) as usize;
        let c = self.columns;
        ![k, k + 1, k + c, k + c + 1]
            .iter()
            .any(|&k| self.through(k))
    }

    /// The surface as projected triangles, far quads first so near ones cover them.
    pub fn mesh(&self, view: &View, fade: bool) -> Mesh {
        let mut mesh = Mesh::default();
        if self.columns < 2 || self.rows < 2 {
            return mesh;
        }
        let tint = if fade { GHOST_FADE } else { 1.0 };
        let project = |p: Point3| {
            if self.raw {
                view.to_screen3_raw(p)
            } else {
                view.to_screen3(p)
            }
        };
        let centre = |i: usize, j: usize| {
            (
                self.origin.0 + (i as f64 + 0.5) * self.cell,
                self.origin.1 + (j as f64 + 0.5) * self.cell,
            )
        };
        mesh.reserve_vertices(self.columns * self.rows);
        for j in 0..self.rows {
            for i in 0..self.columns {
                let k = j * self.columns + i;
                let (x, y) = centre(i, j);
                let at = project((x, y, f64::from(self.heights[k])));
                mesh.colored_vertex(at, self.colors[k].gamma_multiply(tint));
            }
        }
        // Depth into the screen grows with x·sin(turn) + y·cos(turn) (View::camera).
        let (sin_t, cos_t) = math::sin_cos(view.turn);
        let depth = |i: f64, j: f64| i * sin_t + j * cos_t;
        let columns = self.columns as u32;
        let mut quads: Vec<Quad> = Vec::with_capacity((self.columns - 1) * (self.rows - 1));
        for j in 0..self.rows - 1 {
            for i in 0..self.columns - 1 {
                if !self.kept(i as isize, j as isize) {
                    continue;
                }
                // The grid's size is far within u32 (at most 250 000 cells).
                let k = (j * self.columns + i) as u32;
                quads.push((
                    depth(i as f64 + 0.5, j as f64 + 0.5),
                    [k, k + 1, k + columns + 1, k + columns],
                ));
            }
        }
        // Walls where the surface ends, facing the viewer; one facing away lies behind the
        // surface or the board and would cover what is seen through a hole.
        if !view.is_top() {
            let wall = self.wall.gamma_multiply(tint);
            let floor_vertex = |mesh: &mut Mesh, i: usize, j: usize| -> u32 {
                let k = j * self.columns + i;
                let (x, y) = centre(i, j);
                let index = mesh.vertices.len() as u32;
                mesh.colored_vertex(project((x, y, f64::from(self.floors[k]))), wall);
                index
            };
            let facing = |n: (f64, f64)| n.0 * sin_t + n.1 * cos_t < 0.0;
            // Edges along X, between vertex rows: quads below (j - 1) and above (j).
            for j in 0..self.rows {
                for i in 0..self.columns - 1 {
                    let (below, above) = (
                        self.kept(i as isize, j as isize - 1),
                        self.kept(i as isize, j as isize),
                    );
                    let normal = match (below, above) {
                        (true, false) => (0.0, 1.0),
                        (false, true) => (0.0, -1.0),
                        _ => continue,
                    };
                    if !facing(normal) {
                        continue;
                    }
                    let (a, b) = (
                        (j * self.columns + i) as u32,
                        (j * self.columns + i + 1) as u32,
                    );
                    let (fa, fb) = (
                        floor_vertex(&mut mesh, i, j),
                        floor_vertex(&mut mesh, i + 1, j),
                    );
                    quads.push((depth(i as f64 + 0.5, j as f64), [a, b, fb, fa]));
                }
            }
            // Edges along Y, between vertex columns: quads left (i - 1) and right (i).
            for j in 0..self.rows - 1 {
                for i in 0..self.columns {
                    let (left, right) = (
                        self.kept(i as isize - 1, j as isize),
                        self.kept(i as isize, j as isize),
                    );
                    let normal = match (left, right) {
                        (true, false) => (1.0, 0.0),
                        (false, true) => (-1.0, 0.0),
                        _ => continue,
                    };
                    if !facing(normal) {
                        continue;
                    }
                    let (a, b) = (
                        (j * self.columns + i) as u32,
                        ((j + 1) * self.columns + i) as u32,
                    );
                    let (fa, fb) = (
                        floor_vertex(&mut mesh, i, j),
                        floor_vertex(&mut mesh, i, j + 1),
                    );
                    quads.push((depth(i as f64, j as f64 + 0.5), [a, b, fb, fa]));
                }
            }
        }
        quads.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
        mesh.reserve_triangles(2 * quads.len());
        for (_, [a, b, c, d]) in quads {
            mesh.add_triangle(a, b, c);
            mesh.add_triangle(a, c, d);
        }
        mesh
    }
}

impl SimLayer {
    pub(super) fn paint(&self, painter: &Painter, view: &View, fade: bool) {
        if view.is_top() {
            let rect = Rect::from_two_pos(
                view.to_screen((self.min.0, self.max.1)),
                view.to_screen((self.max.0, self.min.1)),
            );
            let uv = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
            let tint = if fade {
                Color32::WHITE.gamma_multiply(GHOST_FADE)
            } else {
                Color32::WHITE
            };
            painter.image(self.texture, rect, uv, tint);
        } else {
            painter.add(Shape::mesh(self.surface.mesh(view, fade)));
        }
    }
}

/// A picture traced into shapes, drawn faintly under the drawing in top view
/// (View > Show images; docs/specs/image-trace.md §6).
#[derive(Debug, Clone, PartialEq)]
pub struct PictureLayer {
    pub texture: eframe::egui::TextureId,
    /// The picture's corners in drawing coordinates, turned as the picture is: its
    /// lower-left, lower-right, upper-right and upper-left (docs/specs/direct-placement.md).
    pub corners: [Point; 4],
}

/// How strongly a picture shows under the drawing.
const PICTURE_FADE: f32 = 0.35;

impl PictureLayer {
    /// On the stock top, in top view and 3D.
    pub(super) fn paint(&self, painter: &Painter, view: &View) {
        // Two triangles, so a turned picture is drawn turned. The picture's top row is
        // v = 0.
        let color = Color32::WHITE.gamma_multiply(PICTURE_FADE);
        let uv = [(0.0, 1.0), (1.0, 1.0), (1.0, 0.0), (0.0, 0.0)];
        let mut mesh = eframe::egui::Mesh::with_texture(self.texture);
        for (corner, (u, v)) in self.corners.iter().zip(uv) {
            mesh.vertices.push(eframe::egui::epaint::Vertex {
                pos: view.to_screen(*corner),
                uv: Pos2::new(u, v),
                color,
            });
        }
        mesh.add_triangle(0, 1, 2);
        mesh.add_triangle(0, 2, 3);
        painter.add(Shape::mesh(mesh));
    }
}
