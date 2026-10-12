// SPDX-License-Identifier: GPL-3.0-or-later
//! The editor for one drawn shape's defining values (docs/specs/drawing-tools.md section 6).
//! It only edits a draft; the app applies it through the pipeline, which validates.

use chipload_job::edit::{insert_point_after, remove_point, set_closed};
use chipload_job::{Geometry, Shape, polyline_segment_count};
use eframe::egui::{self, DragValue, Grid, RichText, TextEdit};

/// A drawn shape as being edited. `error` holds why the last edit couldn't be applied; the
/// draft keeps the user's values until they fix them.
#[derive(Debug, Clone, PartialEq)]
pub struct ShapeDraft {
    pub id: u64,
    pub layer: String,
    pub geometry: Geometry,
    pub error: Option<String>,
}

impl ShapeDraft {
    pub fn of(shape: &Shape) -> Self {
        ShapeDraft {
            id: shape.id,
            layer: shape.layer.clone(),
            geometry: shape.geometry.clone(),
            error: None,
        }
    }

    /// Whether the draft says something different from `shape`.
    pub fn differs_from(&self, shape: &Shape) -> bool {
        self.layer != shape.layer || self.geometry != shape.geometry
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShapeAction {
    None,
    Delete,
}

fn mm(value: &mut f64) -> DragValue<'_> {
    DragValue::new(value)
        .speed(0.1)
        .max_decimals(3)
        .suffix(" mm")
}

/// The form. Edits change `draft` in place; the caller applies it if it differs.
pub fn shape_form(ui: &mut egui::Ui, draft: &mut ShapeDraft) -> ShapeAction {
    let mut action = ShapeAction::None;
    let kind = match draft.geometry {
        Geometry::Path { .. } => "path",
        Geometry::Circle { .. } => "Circle",
        Geometry::Rectangle { .. } => "Rectangle",
        Geometry::Polyline { .. } => "Polyline",
    };
    ui.label(RichText::new(format!("{kind} S{}", draft.id)).strong());
    Grid::new(("shape-form", draft.id))
        .num_columns(2)
        .show(ui, |ui| {
            ui.label("Layer");
            ui.add(TextEdit::singleline(&mut draft.layer).desired_width(140.0));
            ui.end_row();
            match &mut draft.geometry {
                Geometry::Path { .. } => {}
                Geometry::Circle { center, radius } => {
                    centre_row(ui, center);
                    ui.label("Radius");
                    ui.add(mm(radius));
                    ui.end_row();
                }
                Geometry::Rectangle {
                    center,
                    length,
                    width,
                    angle,
                } => {
                    centre_row(ui, center);
                    ui.label("Length (X)");
                    ui.add(mm(length));
                    ui.end_row();
                    ui.label("Width (Y)");
                    ui.add(mm(width));
                    ui.end_row();
                    ui.label("Angle");
                    // Shown in degrees; written back only when changed, so an untouched
                    // angle never drifts through the degree conversion.
                    let mut degrees = angle.to_degrees();
                    let response = ui.add(
                        DragValue::new(&mut degrees)
                            .speed(0.5)
                            .max_decimals(3)
                            .suffix("°"),
                    );
                    if response.changed() {
                        *angle = degrees.to_radians();
                    }
                    ui.end_row();
                }
                Geometry::Polyline { closed, .. } => {
                    let mut close = *closed;
                    ui.label("Closed");
                    ui.checkbox(&mut close, "back to the first point");
                    ui.end_row();
                    if close != *closed {
                        set_closed(&mut draft.geometry, close);
                    }
                }
            }
        });
    if matches!(draft.geometry, Geometry::Polyline { .. }) {
        ui.label(egui::RichText::new(POLYLINE_HELP).weak());
        points_table(ui, draft.id, &mut draft.geometry);
    }
    if let Some(error) = &draft.error {
        ui.colored_label(egui::Color32::from_rgb(0xf2, 0x8b, 0x82), error);
    }
    if ui
        .button("Delete shape  (Del)")
        .on_hover_text("Operations that use it are flagged until their geometry is chosen again")
        .clicked()
    {
        action = ShapeAction::Delete;
    }
    action
}

/// How to edit a selected polyline on the canvas: shown under its form and in the status bar.
pub const POLYLINE_HELP: &str = "On the canvas: drag a square to move a point · drag a round \
     middle handle to insert a new point there · Shift+drag a middle handle to bend the \
     segment into an arc (bulge) · Alt: no snapping";

fn centre_row(ui: &mut egui::Ui, center: &mut [f64; 2]) {
    ui.label("Centre");
    ui.horizontal(|ui| {
        ui.add(mm(&mut center[0]).prefix("X "));
        ui.add(mm(&mut center[1]).prefix("Y "));
    });
    ui.end_row();
}

/// X, Y per point, the bulge of the segment that follows it, and insert-after and delete
/// per row (docs/specs/drawing-follow-ups.md section 3).
fn points_table(ui: &mut egui::Ui, id: u64, geometry: &mut Geometry) {
    let Geometry::Polyline {
        closed,
        points,
        bulges,
    } = geometry
    else {
        return;
    };
    let mut insert_after: Option<usize> = None;
    let mut remove: Option<usize> = None;
    let segments = polyline_segment_count(points.len(), *closed);
    Grid::new(("polyline-points", id))
        .num_columns(5)
        .striped(true)
        .show(ui, |ui| {
            let count = points.len();
            ui.label("");
            ui.label("");
            ui.label("");
            ui.label("Bulge");
            ui.end_row();
            for (i, point) in points.iter_mut().enumerate() {
                ui.label(format!("{}", i + 1));
                ui.add(mm(&mut point[0]).prefix("X "));
                ui.add(mm(&mut point[1]).prefix("Y "));
                match bulges.get_mut(i).filter(|_| i < segments) {
                    Some(bulge) => {
                        ui.add(
                            DragValue::new(bulge)
                                .speed(0.01)
                                .range(-1.0..=1.0)
                                .max_decimals(3),
                        )
                        .on_hover_text(
                            "Bend of the segment to the next point: 0 straight, 1 a half \
                             circle (counter-clockwise), -1 the other way",
                        );
                    }
                    None => {
                        ui.label("");
                    }
                }
                ui.horizontal(|ui| {
                    if ui
                        .small_button("+")
                        .on_hover_text("Insert a point after this one")
                        .clicked()
                    {
                        insert_after = Some(i);
                    }
                    if ui
                        .add_enabled(count > 2, egui::Button::new("−").small())
                        .on_hover_text("Delete this point")
                        .clicked()
                    {
                        remove = Some(i);
                    }
                });
                ui.end_row();
            }
        });
    if let Some(i) = insert_after {
        insert_point_after(geometry, i);
    }
    if let Some(i) = remove {
        remove_point(geometry, i);
    }
}

/// What the selection panel says about an imported shape (not editable).
pub fn imported_lines(shape: &Shape) -> Vec<String> {
    vec![
        format!("Shape S{} (imported, read-only)", shape.id),
        format!(
            "Layer {}, DXF {} {}",
            shape.layer,
            shape.dxf_type().unwrap_or("entity"),
            shape.dxf_handle().unwrap_or("?")
        ),
    ]
}
