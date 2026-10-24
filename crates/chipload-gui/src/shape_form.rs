// SPDX-License-Identifier: GPL-3.0-or-later
//! The editor for one drawn shape's defining values (docs/specs/drawing-tools.md section 6).
//! It only edits a draft; the app applies it through the pipeline, which validates.

use chipload_job::edit::{insert_point_after, remove_point, set_closed};
use chipload_job::{Geometry, Shape, polyline_segment_count};
use eframe::egui::{self, Grid, RichText, TextEdit};

use crate::i18n::{Lang, t};

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

fn mm(value: &mut f64) -> crate::units::Field<'_> {
    crate::units::length(value).speed(0.1).max_decimals(3)
}

/// The form. Edits change `draft` in place; the caller applies it if it differs.
pub fn shape_form(ui: &mut egui::Ui, draft: &mut ShapeDraft) -> ShapeAction {
    let mut action = ShapeAction::None;
    let id = draft.id;
    let title = match draft.geometry {
        Geometry::Path { .. } => t!(ui, "shape-form-title-path", id = id),
        Geometry::Circle { .. } => t!(ui, "shape-form-title-circle", id = id),
        Geometry::Rectangle { .. } => t!(ui, "shape-form-title-rectangle", id = id),
        Geometry::Polyline { .. } => t!(ui, "shape-form-title-polyline", id = id),
    };
    ui.label(RichText::new(title).strong());
    Grid::new(("shape-form", draft.id))
        .num_columns(2)
        .show(ui, |ui| {
            ui.label(t!(ui, "shape-form-layer"));
            ui.add(TextEdit::singleline(&mut draft.layer).desired_width(140.0));
            ui.end_row();
            match &mut draft.geometry {
                Geometry::Path { .. } => {}
                Geometry::Circle { center, radius } => {
                    centre_row(ui, center);
                    ui.label(t!(ui, "shape-form-radius"));
                    ui.add(mm(radius));
                    ui.end_row();
                }
                Geometry::Rectangle {
                    center,
                    length,
                    width,
                    angle,
                    corner_radius,
                } => {
                    centre_row(ui, center);
                    ui.label(t!(ui, "shape-form-length"));
                    ui.add(mm(length));
                    ui.end_row();
                    ui.label(t!(ui, "shape-form-width"));
                    ui.add(mm(width));
                    ui.end_row();
                    ui.label(t!(ui, "shape-form-angle"));
                    // Shown in degrees; written back only when changed, so an untouched
                    // angle never drifts through the degree conversion.
                    let mut degrees = angle.to_degrees();
                    let response = ui.add(
                        crate::units::number(ui, &mut degrees)
                            .speed(0.5)
                            .max_decimals(3)
                            .suffix("°"),
                    );
                    if response.changed() {
                        *angle = degrees.to_radians();
                    }
                    ui.end_row();
                    corner_radius_row(ui, corner_radius, length.min(*width) / 2.0);
                }
                Geometry::Polyline { closed, .. } => {
                    let mut close = *closed;
                    ui.label(t!(ui, "shape-form-closed"));
                    ui.checkbox(&mut close, t!(ui, "shape-form-closed-checkbox"));
                    ui.end_row();
                    if close != *closed {
                        set_closed(&mut draft.geometry, close);
                    }
                }
            }
        });
    if matches!(draft.geometry, Geometry::Polyline { .. }) {
        ui.label(egui::RichText::new(polyline_help(ui)).weak());
        points_table(ui, draft.id, &mut draft.geometry);
    }
    if let Some(error) = &draft.error {
        ui.colored_label(ui.visuals().error_fg_color, error);
    }
    if ui
        .button(t!(ui, "shape-form-delete"))
        .on_hover_text(t!(ui, "shape-form-delete-hover"))
        .clicked()
    {
        action = ShapeAction::Delete;
    }
    action
}

/// How to edit a selected polyline on the canvas: shown under its form and in the status bar.
pub fn polyline_help(lang: &impl Lang) -> String {
    t!(lang, "shape-form-polyline-help")
}

/// The rectangle's corner radius, from 0 (sharp) to half its shorter side `most`, with a
/// reset back to sharp (docs/specs/rectangle-corner-radius.md §5).
fn corner_radius_row(ui: &mut egui::Ui, corner_radius: &mut f64, most: f64) {
    ui.label(t!(ui, "shape-form-corner-radius"));
    ui.horizontal(|ui| {
        // A radius that no longer fits is left as it is, to be flagged, not shrunk.
        ui.add(
            mm(corner_radius)
                .range(0.0..=most.max(0.0))
                .clamp_existing_to_range(false),
        )
        .on_hover_text(t!(ui, "shape-form-corner-radius-hover"));
        let sharp = *corner_radius == 0.0;
        let reset = t!(ui, "shape-form-reset");
        if crate::app::header::icon_button(
            ui,
            crate::icons::Icon::Undo,
            &reset,
            !sharp,
            false,
            18.0,
        )
        .on_hover_text(t!(ui, "shape-form-sharp-corners"))
        .clicked()
        {
            *corner_radius = 0.0;
        }
    });
    ui.end_row();
}

fn centre_row(ui: &mut egui::Ui, center: &mut [f64; 2]) {
    ui.label(t!(ui, "shape-form-centre"));
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
            ui.label(t!(ui, "shape-form-bulge"));
            ui.end_row();
            for (i, point) in points.iter_mut().enumerate() {
                ui.label(format!("{}", i + 1));
                ui.add(mm(&mut point[0]).prefix("X "));
                ui.add(mm(&mut point[1]).prefix("Y "));
                match bulges.get_mut(i).filter(|_| i < segments) {
                    Some(bulge) => {
                        ui.add(
                            crate::units::number(ui, bulge)
                                .speed(0.01)
                                .range(-1.0..=1.0)
                                .max_decimals(3),
                        )
                        .on_hover_text(t!(ui, "shape-form-bulge-hover"));
                    }
                    None => {
                        ui.label("");
                    }
                }
                ui.horizontal(|ui| {
                    if ui
                        .small_button("+")
                        .on_hover_text(t!(ui, "shape-form-insert-point"))
                        .clicked()
                    {
                        insert_after = Some(i);
                    }
                    if ui
                        .add_enabled(count > 2, egui::Button::new("−").small())
                        .on_hover_text(t!(ui, "shape-form-delete-point"))
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
pub fn imported_lines(shape: &Shape, lang: &impl Lang) -> Vec<String> {
    let entity = match shape.dxf_type() {
        Some(kind) => kind.to_string(),
        None => t!(lang, "shape-form-entity"),
    };
    vec![
        t!(lang, "shape-form-imported", id = shape.id),
        t!(
            lang,
            "shape-form-imported-from",
            layer = &shape.layer,
            entity = entity,
            handle = shape.dxf_handle().unwrap_or("?"),
        ),
    ]
}
