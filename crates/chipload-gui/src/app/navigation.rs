// SPDX-License-Identifier: GPL-3.0-or-later
//! On the canvas (docs/specs/browser-and-layout.md §6b): a floating navigation bar at the
//! bottom (Fit, Top, 3D, Simulation, Grid) and, in 3D, an orientation cube in the top right
//! corner whose faces turn the view to them. In top view the corner has an X–Y sign.

use eframe::egui::{Align2, FontId, Pos2, Rect, Shape, Stroke, Vec2, pos2, vec2};

use super::header::ribbon_button;
use super::menu::SIMULATION_HINT;
use super::shortcuts::{Action, hover_keys};
use super::*;
use crate::icons::Icon;
use crate::view::MAX_TILT_RAD;

/// The navigation bar's height, and its width before it has been drawn once, points.
const NAV_HEIGHT: f32 = 54.0;
const NAV_ESTIMATE: f32 = 400.0;
/// Half the cube's edge on screen, points.
const CUBE_HALF: f32 = 20.0;
/// Room around the cube in the corner, points.
const CUBE_CORNER: f32 = 96.0;

/// A face of the orientation cube: its label, outward normal, and the view that looks at it.
struct Face {
    label: &'static str,
    normal: (f64, f64, f64),
    /// (turn, tilt) of View::orient; None: back to the top view.
    look: Option<(f64, f64)>,
}

const FACES: [Face; 5] = [
    Face {
        label: "TOP",
        normal: (0.0, 0.0, 1.0),
        look: None,
    },
    Face {
        label: "FRONT",
        normal: (0.0, -1.0, 0.0),
        look: Some((0.0, MAX_TILT_RAD)),
    },
    Face {
        label: "RIGHT",
        normal: (1.0, 0.0, 0.0),
        look: Some((3.0 * std::f64::consts::FRAC_PI_2, MAX_TILT_RAD)),
    },
    Face {
        label: "BACK",
        normal: (0.0, 1.0, 0.0),
        look: Some((std::f64::consts::PI, MAX_TILT_RAD)),
    },
    Face {
        label: "LEFT",
        normal: (-1.0, 0.0, 0.0),
        look: Some((std::f64::consts::FRAC_PI_2, MAX_TILT_RAD)),
    },
];

impl ChiploadApp {
    /// The floating bar at the canvas's bottom centre: how you look (Fit, Top, 3D), then
    /// what is shown (Stock, Toolpath, Images, Grid), each an icon over a word
    /// (docs/specs/view-and-settings.md §2).
    pub(super) fn navigation_bar(&mut self, ui: &mut egui::Ui, rect: Rect) {
        let visuals = ui.visuals().clone();
        // Sized to its buttons, so it sits in the middle: the width they took last frame.
        let size_id = egui::Id::new("navigation-bar-width");
        let width = ui
            .data(|d| d.get_temp::<f32>(size_id))
            .unwrap_or(NAV_ESTIMATE);
        let height = NAV_HEIGHT;
        let at = Rect::from_center_size(
            pos2(rect.center().x, rect.bottom() - 10.0 - height / 2.0),
            vec2(width, height),
        );
        let place = egui::UiBuilder::new()
            .max_rect(at)
            .layout(egui::Layout::left_to_right(egui::Align::Center));
        let shown = ui
            .scope_builder(place, |ui| {
                egui::Frame::new()
                    .fill(visuals.panel_fill.gamma_multiply(0.94))
                    .stroke(visuals.widgets.noninteractive.bg_stroke)
                    .corner_radius(6.0)
                    .inner_margin(egui::Margin::same(4))
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing.x = 2.0;
                        self.view_buttons(ui);
                        ui.add_space(4.0);
                        ui.separator();
                        ui.add_space(4.0);
                        self.show_buttons(ui);
                    })
                    .response
                    .rect
            })
            .inner;
        if (shown.width() - width).abs() > 0.5 {
            ui.data_mut(|d| d.insert_temp(size_id, shown.width()));
            ui.ctx().request_repaint();
        }
    }

    /// Fit, Top and 3D.
    fn view_buttons(&mut self, ui: &mut egui::Ui) {
        if ribbon_button(ui, Icon::Fit, "Fit", true, false)
            .on_hover_text(hover_keys("Fit the view", Action::Fit))
            .clicked()
        {
            self.fit_pending = true;
        }
        if ribbon_button(ui, Icon::Top, "Top", true, !self.three_d)
            .on_hover_text(hover_keys("Look straight down", Action::TopView))
            .clicked()
        {
            self.view_top();
        }
        let three_d = if self.view.z_scale == 1.0 {
            "3D: left-drag turns and tilts; right-drag or Shift+drag pans".to_string()
        } else {
            format!(
                "3D, Z stretched × {}: left-drag turns and tilts; right-drag or Shift+drag \
                 pans",
                self.view.z_scale
            )
        };
        if ribbon_button(ui, Icon::ThreeD, "3D", true, self.three_d)
            .on_hover_text(three_d)
            .clicked()
        {
            self.view_3d();
        }
    }

    /// What is shown: the simulated stock, the toolpath, the pictures and the grid.
    fn show_buttons(&mut self, ui: &mut egui::Ui) {
        let can = self.result.is_some();
        let label = if self.simulating() {
            "Stock…"
        } else {
            "Stock"
        };
        let hover = if self.simulating() {
            "Simulating…"
        } else if can {
            SIMULATION_HINT
        } else {
            "The stock as cut: generate the toolpath first"
        };
        if ribbon_button(
            ui,
            Icon::Simulation,
            label,
            can,
            can && self.show_simulation,
        )
        .on_hover_text(hover)
        .clicked()
        {
            self.set_show_simulation(!self.show_simulation);
        }
        let toolpath = self.show_toolpath;
        if ribbon_button(ui, Icon::Toolpath, "Toolpath", true, toolpath)
            .on_hover_text("Show the toolpath; off, the drawing or the simulated stock alone")
            .clicked()
        {
            self.show_toolpath = !toolpath;
        }
        let images = self.show_images;
        if ribbon_button(ui, Icon::Image, "Images", true, images)
            .on_hover_text("The pictures traced into shapes, faintly under the drawing")
            .clicked()
        {
            self.show_images = !images;
        }
        if ribbon_button(ui, Icon::Grid, "Grid", true, self.show_grid)
            .on_hover_text(hover_keys("Show the grid", Action::Grid))
            .clicked()
        {
            self.set_show_grid(!self.show_grid);
        }
        // With several parts, the toolpath and simulation are of the active part unless
        // this is on (docs/specs/active-part-preview.md §3).
        let parts = self
            .edit
            .as_ref()
            .map_or(0, |e| e.settings.parts.iter().filter(|p| p.enabled).count());
        if parts > 1
            && ribbon_button(ui, Icon::Parts, "All parts", true, self.all_parts)
                .on_hover_text(
                    "Generate and simulate every part, not only the active one. Export always \
                     writes every part.",
                )
                .clicked()
        {
            self.set_all_parts(!self.all_parts);
        }
    }

    /// The top right corner: the orientation cube in 3D, an X–Y sign in top view.
    pub(super) fn orientation(&mut self, ui: &mut egui::Ui, rect: Rect) {
        let centre = pos2(
            rect.right() - CUBE_CORNER / 2.0 - 4.0,
            rect.top() + CUBE_CORNER / 2.0 + 4.0,
        );
        let pal = self.scene.palette;
        if self.view.is_top() {
            axis_sign(ui.painter(), centre, &pal);
            return;
        }
        let area = Rect::from_center_size(centre, Vec2::splat(CUBE_CORNER));
        let response = ui
            .interact(area, egui::Id::new("orientation-cube"), Sense::click())
            .on_hover_text("Click a face to look at it");
        let hover = response.hover_pos();
        let mut picked = None;
        let visuals = ui.visuals().clone();
        let painter = ui.painter_at(area);
        let toward = self.toward_viewer();
        for (k, face) in FACES.iter().enumerate() {
            let facing = dot(face.normal, toward);
            if facing <= 0.02 {
                continue;
            }
            let corners = self.face_corners(face.normal, centre);
            let hovered = hover.is_some_and(|p| inside(p, &corners));
            if hovered {
                picked = Some(k);
            }
            let fill = if hovered {
                visuals.selection.bg_fill
            } else {
                visuals.panel_fill.gamma_multiply(0.92)
            };
            painter.add(Shape::convex_polygon(
                corners.to_vec(),
                fill,
                Stroke::new(1.0, visuals.widgets.noninteractive.fg_stroke.color),
            ));
            let middle = corners.iter().fold(Vec2::ZERO, |a, p| a + p.to_vec2()) / 4.0;
            let text = if hovered {
                visuals.selection.stroke.color
            } else {
                visuals.text_color()
            };
            // A label only where it fits: a face seen nearly edge-on is too narrow.
            let label =
                painter.layout_no_wrap(face.label.to_string(), FontId::proportional(9.0), text);
            let (left, right) = corners
                .iter()
                .fold((f32::INFINITY, f32::NEG_INFINITY), |(l, r), p| {
                    (l.min(p.x), r.max(p.x))
                });
            if facing > 0.3 && label.size().x + 6.0 <= right - left {
                let at = middle.to_pos2() - label.size() / 2.0;
                painter.galley(at, label, text);
            }
        }
        if response.clicked()
            && let Some(face) = picked.and_then(|k| FACES.get(k))
        {
            match face.look {
                None => self.view_top(),
                Some((turn, tilt)) => {
                    self.view_3d();
                    let pivot = self.orbit_pivot();
                    self.view.orient(turn, tilt, pivot);
                }
            }
        }
    }

    /// The direction from the scene towards the viewer, in the world.
    fn toward_viewer(&self) -> (f64, f64, f64) {
        let (sin_t, cos_t) = self.view.turn.sin_cos();
        let (sin_a, cos_a) = self.view.tilt.sin_cos();
        (-sin_a * sin_t, -sin_a * cos_t, cos_a)
    }

    /// A unit cube face's corners on screen, turned as the view is, around `centre`.
    fn face_corners(&self, n: (f64, f64, f64), centre: Pos2) -> [Pos2; 4] {
        // Two directions across the face, then its four corners.
        let (u, v) = match n {
            (x, _, _) if x != 0.0 => ((0.0, 1.0, 0.0), (0.0, 0.0, 1.0)),
            (_, y, _) if y != 0.0 => ((1.0, 0.0, 0.0), (0.0, 0.0, 1.0)),
            _ => ((1.0, 0.0, 0.0), (0.0, 1.0, 0.0)),
        };
        let at = |su: f64, sv: f64| {
            let p = (
                n.0 + su * u.0 + sv * v.0,
                n.1 + su * u.1 + sv * v.1,
                n.2 + su * u.2 + sv * v.2,
            );
            self.cube_point(p, centre)
        };
        [at(-1.0, -1.0), at(1.0, -1.0), at(1.0, 1.0), at(-1.0, 1.0)]
    }

    /// A point of the unit cube on screen: the view's turn and tilt, no Z stretch.
    fn cube_point(&self, (x, y, z): (f64, f64, f64), centre: Pos2) -> Pos2 {
        let (sin_t, cos_t) = self.view.turn.sin_cos();
        let (sin_a, cos_a) = self.view.tilt.sin_cos();
        let across = x * cos_t - y * sin_t;
        let depth = x * sin_t + y * cos_t;
        let up = depth * cos_a + z * sin_a;
        let half = f64::from(CUBE_HALF);
        pos2(
            centre.x + (across * half) as f32,
            centre.y - (up * half) as f32,
        )
    }
}

/// In top view: short X and Y arrows with their letters.
fn axis_sign(painter: &egui::Painter, centre: Pos2, pal: &crate::view::Palette) {
    let origin = centre + vec2(-14.0, 14.0);
    let length = 26.0;
    for (to, color, label, offset) in [
        (origin + vec2(length, 0.0), pal.axis_x, "X", vec2(8.0, 0.0)),
        (
            origin + vec2(0.0, -length),
            pal.axis_y,
            "Y",
            vec2(0.0, -8.0),
        ),
    ] {
        painter.line_segment([origin, to], Stroke::new(2.0, color));
        painter.text(
            to + offset,
            Align2::CENTER_CENTER,
            label,
            FontId::proportional(11.0),
            color,
        );
    }
}

fn dot(a: (f64, f64, f64), b: (f64, f64, f64)) -> f64 {
    a.0 * b.0 + a.1 * b.1 + a.2 * b.2
}

/// Whether `p` is inside the convex quad `corners` (either winding).
fn inside(p: Pos2, corners: &[Pos2; 4]) -> bool {
    let side = |a: Pos2, b: Pos2| (b - a).x * (p - a).y - (b - a).y * (p - a).x;
    let signs: Vec<f32> = (0..4)
        .map(|i| side(corners[i], corners[(i + 1) % 4]))
        .collect();
    signs.iter().all(|&s| s >= 0.0) || signs.iter().all(|&s| s <= 0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_cube_shows_the_faces_turned_to_the_viewer() {
        let mut app = ChiploadApp::default();
        // The default 3D view looks from the front right, above.
        app.view
            .orient(-30f64.to_radians(), 55f64.to_radians(), (0.0, 0.0));
        let toward = app.toward_viewer();
        let seen: Vec<&str> = FACES
            .iter()
            .filter(|f| dot(f.normal, toward) > 0.0)
            .map(|f| f.label)
            .collect();
        assert_eq!(seen, ["TOP", "FRONT", "RIGHT"]);
        // Each face's look turns the view to face it straight on (as far as the tilt goes).
        for face in FACES.iter().filter(|f| f.look.is_some()) {
            let (turn, tilt) = face.look.unwrap();
            app.view.orient(turn, tilt, (0.0, 0.0));
            let toward = app.toward_viewer();
            assert!(dot(face.normal, toward) > 0.99, "{}", face.label);
        }
    }

    #[test]
    fn a_point_is_inside_a_quad_whichever_way_it_winds() {
        let quad = [
            pos2(0.0, 0.0),
            pos2(10.0, 0.0),
            pos2(10.0, 10.0),
            pos2(0.0, 10.0),
        ];
        assert!(inside(pos2(5.0, 5.0), &quad));
        assert!(!inside(pos2(15.0, 5.0), &quad));
        let mut reversed = quad;
        reversed.reverse();
        assert!(inside(pos2(5.0, 5.0), &reversed));
    }
}
