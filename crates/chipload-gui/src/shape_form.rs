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
    /// Its name as typed; empty: no name (docs/specs/drawn-shapes-box.md §4).
    pub name: String,
    pub geometry: Geometry,
    pub error: Option<String>,
    /// The name field takes the keyboard (Browser: Rename…).
    pub grab_name: bool,
}

impl ShapeDraft {
    pub fn of(shape: &Shape) -> Self {
        ShapeDraft {
            id: shape.id,
            layer: shape.layer.clone(),
            name: shape.name.clone().unwrap_or_default(),
            geometry: shape.geometry.clone(),
            error: None,
            grab_name: false,
        }
    }

    /// Whether the name as typed differs from `shape`'s (spaces around it don't count).
    pub fn name_differs_from(&self, shape: &Shape) -> bool {
        self.name.trim() != shape.name.as_deref().unwrap_or("")
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
        ref g if chipload_job::edit::is_slot(g) => t!(ui, "shape-form-title-slot", id = id),
        Geometry::Rectangle { .. } => t!(ui, "shape-form-title-rectangle", id = id),
        Geometry::Polyline { .. } => t!(ui, "shape-form-title-polyline", id = id),
        Geometry::Polygon { .. } => t!(ui, "shape-form-title-polygon", id = id),
    };
    ui.label(RichText::new(title).strong());
    Grid::new(("shape-form", draft.id))
        .num_columns(2)
        .show(ui, |ui| {
            name_row(ui, draft);
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
                    angle_field(ui, angle);
                    ui.end_row();
                    corner_radius_row(ui, corner_radius, length.min(*width) / 2.0);
                }
                Geometry::Polygon {
                    center,
                    radius,
                    sides,
                    angle,
                } => {
                    centre_row(ui, center);
                    ui.label(t!(ui, "shape-form-sides"));
                    ui.add(
                        egui::DragValue::new(sides)
                            .range(chipload_job::POLYGON_SIDES)
                            .speed(0.1),
                    );
                    ui.end_row();
                    ui.label(t!(ui, "shape-form-polygon-radius"))
                        .on_hover_text(t!(ui, "shape-form-polygon-radius-hover"));
                    ui.add(mm(radius));
                    ui.end_row();
                    // Across the flats: twice the distance from the centre to a side's
                    // middle; changing it changes the radius.
                    let cos = (std::f64::consts::PI / f64::from((*sides).max(3))).cos();
                    let mut flats = 2.0 * *radius * cos;
                    ui.label(t!(ui, "shape-form-across-flats"))
                        .on_hover_text(t!(ui, "shape-form-across-flats-hover"));
                    if ui.add(mm(&mut flats)).changed() {
                        *radius = flats / 2.0 / cos;
                    }
                    ui.end_row();
                    ui.label(t!(ui, "shape-form-polygon-angle"))
                        .on_hover_text(t!(ui, "shape-form-polygon-angle-hover"));
                    angle_field(ui, angle);
                    ui.end_row();
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
            // A polyline has no position or angle of its own: these move and turn its
            // points (docs/specs/drawn-shapes-box.md §3).
            if matches!(draft.geometry, Geometry::Polyline { .. })
                && let Some(bounds) = geometry_box(&draft.geometry)
                && let Some(edit) = place_rows(ui, egui::Id::new(("polyline-place", id)), bounds)
                && let Some(moved) = edit.apply(&draft.geometry, bounds)
            {
                draft.geometry = moved;
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

/// An angle field in degrees. Written back only when changed, so an untouched angle never
/// drifts through the degree conversion.
fn angle_field(ui: &mut egui::Ui, angle: &mut f64) {
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
}

/// The name row. While the field has the keyboard it shows what was typed, spaces
/// included; the stored name is trimmed.
fn name_row(ui: &mut egui::Ui, draft: &mut ShapeDraft) {
    ui.label(t!(ui, "shape-form-name"));
    let field = egui::Id::new(("shape-name", draft.id));
    let buffer = field.with("typed");
    if ui.memory(|m| m.has_focus(field))
        && let Some(typed) = ui.data(|d| d.get_temp::<String>(buffer))
    {
        draft.name = typed;
    }
    let response = ui
        .add(
            TextEdit::singleline(&mut draft.name)
                .id(field)
                .desired_width(140.0)
                .hint_text(format!("S{}", draft.id)),
        )
        .on_hover_text(t!(ui, "shape-form-name-hover"));
    if std::mem::take(&mut draft.grab_name) {
        response.request_focus();
    }
    ui.data_mut(|d| d.insert_temp(buffer, draft.name.clone()));
    ui.end_row();
}

/// A change asked for in the Position and Turn rows.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PlaceEdit {
    /// Move so that the box's lower-left corner is here, mm.
    CornerTo(f64, f64),
    /// Turn about the middle of the box, degrees counter-clockwise.
    TurnBy(f64),
}

impl PlaceEdit {
    /// What it does to the shapes in box `bounds`: from, to, scale, degrees.
    pub fn transform(self, bounds: ((f64, f64), (f64, f64))) -> ((f64, f64), (f64, f64), f64, f64) {
        let ((x0, y0), (x1, y1)) = bounds;
        match self {
            PlaceEdit::CornerTo(x, y) => ((x0, y0), (x, y), 1.0, 0.0),
            PlaceEdit::TurnBy(degrees) => {
                let middle = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);
                (middle, middle, 1.0, degrees)
            }
        }
    }

    /// `geometry`, in box `bounds`, changed by it.
    fn apply(self, geometry: &Geometry, bounds: ((f64, f64), (f64, f64))) -> Option<Geometry> {
        let (from, to, scale, degrees) = self.transform(bounds);
        chipload_job::edit::transformed(geometry, [from.0, from.1], [to.0, to.1], scale, degrees)
    }
}

/// The exact box round a shape, mm.
pub fn geometry_box(geometry: &Geometry) -> Option<((f64, f64), (f64, f64))> {
    let shape = Shape {
        construction: false,
        id: 0,
        layer: String::new(),
        name: None,
        geometry: geometry.clone(),
    };
    shape
        .segments()
        .iter()
        .map(|s| s.to_primitive().bounds())
        .reduce(|(lo, hi), (a, b)| {
            (
                (lo.0.min(a.0), lo.1.min(a.1)),
                (hi.0.max(b.0), hi.1.max(b.1)),
            )
        })
}

/// Two grid rows: *Position* (the lower-left corner of box `bounds`) and *Turn* (⟲ 90,
/// ⟳ 90 and a field that turns by its value when the edit is finished, then shows 0 again).
pub fn place_rows(
    ui: &mut egui::Ui,
    id: egui::Id,
    bounds: ((f64, f64), (f64, f64)),
) -> Option<PlaceEdit> {
    let mut edit = None;
    let ((x0, y0), _) = bounds;
    ui.label(t!(ui, "shape-form-position"))
        .on_hover_text(t!(ui, "shape-form-position-hover"));
    ui.horizontal(|ui| {
        let (mut x, mut y) = (x0, y0);
        let moved_x = ui.add(mm(&mut x).prefix("X ")).changed();
        let moved_y = ui.add(mm(&mut y).prefix("Y ")).changed();
        if moved_x || moved_y {
            edit = Some(PlaceEdit::CornerTo(x, y));
        }
    });
    ui.end_row();
    ui.label(t!(ui, "shape-form-turn"));
    ui.horizontal(|ui| {
        if ui
            .small_button("⟲ 90")
            .on_hover_text(t!(ui, "shape-form-turn-ccw-hover"))
            .clicked()
        {
            edit = Some(PlaceEdit::TurnBy(90.0));
        }
        if ui
            .small_button("⟳ 90")
            .on_hover_text(t!(ui, "shape-form-turn-cw-hover"))
            .clicked()
        {
            edit = Some(PlaceEdit::TurnBy(-90.0));
        }
        let key = id.with("turn-by");
        let mut degrees: f64 = ui.data(|d| d.get_temp(key)).unwrap_or(0.0);
        let response = ui
            .add(
                crate::units::number(ui, &mut degrees)
                    .speed(1.0)
                    .max_decimals(3)
                    .prefix(t!(ui, "shape-form-turn-by"))
                    .suffix("°"),
            )
            .on_hover_text(t!(ui, "shape-form-turn-by-hover"));
        let finished = response.lost_focus() || response.drag_stopped();
        if finished && degrees != 0.0 {
            edit = Some(PlaceEdit::TurnBy(degrees));
            degrees = 0.0;
        }
        ui.data_mut(|d| d.insert_temp(key, degrees));
    });
    ui.end_row();
    edit
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
