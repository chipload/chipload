// SPDX-License-Identifier: GPL-3.0-or-later
//! A small 3D view of meshes for the *Turn and cut…* window (docs/specs/relief-wizard.md
//! §4): a camera that turns about the meshes' middle as the mouse drags, flat shading
//! worked out on the CPU, triangles painted from the farthest to the nearest. Only for
//! showing: the meshes come from the pipeline, already turned and cut.

use chipload_pipeline::models::Mesh;
use eframe::egui::{self, Color32, Pos2, Rect, Sense, Vec2};

/// How the view looks at the meshes: turned about the vertical by `yaw`, looking down by
/// `pitch`, both in radians.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Camera {
    pub yaw: f32,
    pub pitch: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Camera {
            yaw: -0.5,
            pitch: 0.6,
        }
    }
}

/// Radians turned per point dragged.
const TURN_PER_POINT: f32 = 0.01;
/// The view looks at most this close to straight down or level, radians.
const PITCH_LIMIT: f32 = 1.55;
/// Shade of a face turned away from the light, as a share of its colour.
const SHADE_LOW: f32 = 0.35;
/// The share of the view's smaller side the meshes fill.
const FILL: f32 = 0.9;

/// A flat plate at Z `z` over the XY box from `lo` to `hi`.
pub struct Plate {
    pub z: f64,
    pub lo: [f64; 2],
    pub hi: [f64; 2],
    pub color: Color32,
}

/// What to paint: meshes with their colours, and a flat plate at a height.
pub struct Scene<'a> {
    pub layers: Vec<(&'a Mesh, Color32)>,
    pub plate: Option<Plate>,
    /// The box the camera frames.
    pub bounds: ([f64; 3], [f64; 3]),
}

/// The painted triangles, kept until the camera, the view's size or the scene changes.
#[derive(Default, Clone)]
pub struct Painted {
    key: Option<(u64, [i32; 4], [u32; 2])>,
    mesh: egui::Mesh,
}

/// Show `scene` in a view of `size`, turning `camera` as the mouse drags on it. `version`
/// changes whenever the scene does.
pub fn show(
    ui: &mut egui::Ui,
    size: Vec2,
    camera: &mut Camera,
    painted: &mut Painted,
    version: u64,
    scene: &Scene<'_>,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(size, Sense::drag());
    if response.dragged() {
        let d = response.drag_delta();
        camera.yaw -= d.x * TURN_PER_POINT;
        camera.pitch = (camera.pitch + d.y * TURN_PER_POINT).clamp(-PITCH_LIMIT, PITCH_LIMIT);
    }
    let key = (
        version,
        [
            rect.min.x as i32,
            rect.min.y as i32,
            rect.max.x as i32,
            rect.max.y as i32,
        ],
        [camera.yaw.to_bits(), camera.pitch.to_bits()],
    );
    if painted.key != Some(key) {
        painted.mesh = paint(rect, *camera, scene);
        painted.key = Some(key);
    }
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 4.0, ui.visuals().extreme_bg_color);
    painter.add(egui::Shape::mesh(painted.mesh.clone()));
    response
}

/// A corner seen by the camera: x right and y up on the screen, and how far into it.
struct Seen {
    at: Pos2,
    depth: f32,
    /// x, y up the screen, depth, for the normal.
    view: [f32; 3],
}

/// The scene as one mesh of triangles in `rect`, the farthest painted first.
fn paint(rect: Rect, camera: Camera, scene: &Scene<'_>) -> egui::Mesh {
    let (lo, hi) = scene.bounds;
    let middle = [
        ((lo[0] + hi[0]) / 2.0) as f32,
        ((lo[1] + hi[1]) / 2.0) as f32,
        ((lo[2] + hi[2]) / 2.0) as f32,
    ];
    let radius = (((hi[0] - lo[0]).powi(2) + (hi[1] - lo[1]).powi(2) + (hi[2] - lo[2]).powi(2))
        .sqrt()
        / 2.0)
        .max(1e-3) as f32;
    let scale = FILL * rect.width().min(rect.height()) / (2.0 * radius);
    let center = rect.center();
    let (sy, cy) = camera.yaw.sin_cos();
    let (sp, cp) = camera.pitch.sin_cos();
    let see = |v: [f64; 3]| -> Seen {
        let (x, y, z) = (
            v[0] as f32 - middle[0],
            v[1] as f32 - middle[1],
            v[2] as f32 - middle[2],
        );
        // Turned about Z, then looked at from above by the pitch.
        let (u, w) = (x * cy - y * sy, x * sy + y * cy);
        let up = z * cp + w * sp;
        let depth = w * cp - z * sp;
        Seen {
            at: Pos2::new(center.x + u * scale, center.y - up * scale),
            depth,
            view: [u, up, depth],
        }
    };
    // Every triangle seen, with its colour and depth.
    let mut seen: Vec<([Seen; 3], Color32, f32)> = Vec::new();
    let mut add = |t: [[f64; 3]; 3], color: Color32| {
        let s = [see(t[0]), see(t[1]), see(t[2])];
        let depth = (s[0].depth + s[1].depth + s[2].depth) / 3.0;
        seen.push((s, color, depth));
    };
    for (mesh, color) in &scene.layers {
        for t in &mesh.triangles {
            add(*t, *color);
        }
    }
    if let Some(plate) = &scene.plate {
        let (z, [x0, y0], [x1, y1]) = (plate.z, plate.lo, plate.hi);
        let (a, b, c, d) = ([x0, y0, z], [x1, y0, z], [x1, y1, z], [x0, y1, z]);
        add([a, b, c], plate.color);
        add([a, c, d], plate.color);
    }
    // The farthest first, so nearer faces cover them.
    seen.sort_by(|a, b| b.2.total_cmp(&a.2));
    let mut mesh = egui::Mesh::default();
    for (s, color, _) in seen {
        let shade = shade(&s);
        let shaded = Color32::from_rgba_unmultiplied(
            (f32::from(color.r()) * shade) as u8,
            (f32::from(color.g()) * shade) as u8,
            (f32::from(color.b()) * shade) as u8,
            color.a(),
        );
        let first = mesh.vertices.len() as u32;
        for corner in &s {
            mesh.colored_vertex(corner.at, shaded);
        }
        mesh.add_triangle(first, first + 1, first + 2);
    }
    mesh
}

/// How lit a triangle is: its face's turn toward a light over the viewer's left shoulder,
/// either side lit (the cut leaves open edges).
fn shade(s: &[Seen; 3]) -> f32 {
    let (a, b, c) = (s[0].view, s[1].view, s[2].view);
    let (u, v) = (
        [b[0] - a[0], b[1] - a[1], b[2] - a[2]],
        [c[0] - a[0], c[1] - a[1], c[2] - a[2]],
    );
    let n = [
        u[1] * v[2] - u[2] * v[1],
        u[2] * v[0] - u[0] * v[2],
        u[0] * v[1] - u[1] * v[0],
    ];
    let length = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
    if length <= 0.0 {
        return SHADE_LOW;
    }
    // Toward the light: up, to the left and toward the viewer (depth runs away from it).
    let light = [-0.4f32, 0.6, -0.7];
    let l = (light[0] * light[0] + light[1] * light[1] + light[2] * light[2]).sqrt();
    let facing = ((n[0] * light[0] + n[1] * light[1] + n[2] * light[2]) / (length * l)).abs();
    SHADE_LOW + (1.0 - SHADE_LOW) * facing
}
