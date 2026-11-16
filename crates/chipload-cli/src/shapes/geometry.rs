// SPDX-License-Identifier: GPL-3.0-or-later
//! Shapes from the command line's values: points, bulges, a new shape, a shape changed.

use super::*;
use chipload_core::math;

/// "X,Y" in mm, each optionally with a unit (`1in,2in`).
pub(crate) fn point_arg(text: &str) -> Result<[f64; 2], String> {
    lengths::point(text)
}

pub(crate) fn points_arg(text: &str) -> Result<Vec<[f64; 2]>, String> {
    text.split_whitespace().map(point_arg).collect()
}

pub(crate) fn bulges_arg(text: &str) -> Result<Vec<f64>, String> {
    text.split_whitespace()
        .map(|b| {
            b.parse::<f64>()
                .map_err(|_| format!("expected a bulge as a number, got {b:?}"))
        })
        .collect()
}

pub(crate) fn need<T>(value: Option<T>, what: &str, kind: &str) -> Result<T, String> {
    value.ok_or_else(|| format!("a {kind} needs --{what}"))
}

pub(crate) fn new_geometry(kind: KindArg, v: &ShapeValues) -> Result<Geometry, String> {
    let center = v.center.as_deref().map(point_arg).transpose()?;
    if v.corner_radius.is_some() && !matches!(kind, KindArg::Rectangle) {
        return Err("--corner-radius is for a rectangle".into());
    }
    if (v.sides.is_some() || v.across_flats.is_some()) && !matches!(kind, KindArg::Polygon) {
        return Err("--sides and --across-flats are for a polygon".into());
    }
    Ok(match kind {
        KindArg::Circle => Geometry::Circle {
            center: need(center, "center", "circle")?,
            radius: need(v.radius, "radius", "circle")?,
        },
        KindArg::Rectangle => Geometry::Rectangle {
            center: need(center, "center", "rectangle")?,
            length: need(v.length, "length", "rectangle")?,
            width: need(v.width, "width", "rectangle")?,
            angle: v.angle_deg.unwrap_or(0.0).to_radians(),
            corner_radius: v.corner_radius.unwrap_or(0.0),
        },
        KindArg::Polyline => {
            let closed = !v.open;
            let points = points_arg(need(v.points.as_deref(), "points", "polyline")?)?;
            let bulges = match &v.bulges {
                Some(text) => bulges_arg(text)?,
                None => vec![0.0; polyline_segment_count(points.len(), closed)],
            };
            Geometry::Polyline {
                closed,
                points,
                bulges,
            }
        }
        KindArg::Polygon => {
            let sides = need(v.sides, "sides", "polygon")?;
            let radius = match v.across_flats {
                Some(flats) => polygon_radius_across_flats(flats, sides),
                None => need(v.radius, "radius", "polygon")?,
            };
            Geometry::Polygon {
                center: need(center, "center", "polygon")?,
                radius,
                sides,
                angle: v.angle_deg.unwrap_or(0.0).to_radians(),
            }
        }
    })
}

/// A polygon's radius to its corners from its size across the flats.
pub(crate) fn polygon_radius_across_flats(flats: f64, sides: u32) -> f64 {
    flats / 2.0 / math::cos(std::f64::consts::PI / f64::from(sides.max(3)))
}

/// The shape's geometry with the given values replaced; values that don't belong to its
/// kind are refused rather than ignored.
pub(crate) fn changed_geometry(current: &Geometry, v: &ShapeValues) -> Result<Geometry, String> {
    let center = v.center.as_deref().map(point_arg).transpose()?;
    let not_for = |kind: &str, given: &[(&str, bool)]| -> Result<(), String> {
        match given.iter().find(|(_, set)| *set) {
            Some((name, _)) => Err(format!("--{name} doesn't apply to a {kind}")),
            None => Ok(()),
        }
    };
    let mut geometry = current.clone();
    match &mut geometry {
        Geometry::Path { .. } => {}
        Geometry::Circle { center: c, radius } => {
            not_for(
                "circle",
                &[
                    ("length", v.length.is_some()),
                    ("width", v.width.is_some()),
                    ("angle-deg", v.angle_deg.is_some()),
                    ("corner-radius", v.corner_radius.is_some()),
                    ("points", v.points.is_some()),
                    ("bulges", v.bulges.is_some()),
                    ("open", v.open),
                    ("closed", v.closed),
                    ("sides", v.sides.is_some()),
                    ("across-flats", v.across_flats.is_some()),
                ],
            )?;
            *c = center.unwrap_or(*c);
            *radius = v.radius.unwrap_or(*radius);
        }
        Geometry::Polygon {
            center: c,
            radius,
            sides,
            angle,
        } => {
            not_for(
                "polygon",
                &[
                    ("length", v.length.is_some()),
                    ("width", v.width.is_some()),
                    ("corner-radius", v.corner_radius.is_some()),
                    ("points", v.points.is_some()),
                    ("bulges", v.bulges.is_some()),
                    ("open", v.open),
                    ("closed", v.closed),
                ],
            )?;
            *c = center.unwrap_or(*c);
            *sides = v.sides.unwrap_or(*sides);
            *radius = match v.across_flats {
                Some(flats) => polygon_radius_across_flats(flats, *sides),
                None => v.radius.unwrap_or(*radius),
            };
            if let Some(degrees) = v.angle_deg {
                *angle = degrees.to_radians();
            }
        }
        Geometry::Rectangle {
            center: c,
            length,
            width,
            angle,
            corner_radius,
        } => {
            not_for(
                "rectangle",
                &[
                    ("radius", v.radius.is_some()),
                    ("points", v.points.is_some()),
                    ("bulges", v.bulges.is_some()),
                    ("open", v.open),
                    ("closed", v.closed),
                    ("sides", v.sides.is_some()),
                    ("across-flats", v.across_flats.is_some()),
                ],
            )?;
            *c = center.unwrap_or(*c);
            *length = v.length.unwrap_or(*length);
            *width = v.width.unwrap_or(*width);
            *corner_radius = v.corner_radius.unwrap_or(*corner_radius);
            if let Some(degrees) = v.angle_deg {
                *angle = degrees.to_radians();
            }
        }
        Geometry::Polyline {
            closed,
            points,
            bulges,
        } => {
            not_for(
                "polyline",
                &[
                    ("center", center.is_some()),
                    ("radius", v.radius.is_some()),
                    ("length", v.length.is_some()),
                    ("width", v.width.is_some()),
                    ("angle-deg", v.angle_deg.is_some()),
                    ("corner-radius", v.corner_radius.is_some()),
                    ("sides", v.sides.is_some()),
                    ("across-flats", v.across_flats.is_some()),
                ],
            )?;
            if let Some(text) = &v.points {
                *points = points_arg(text)?;
            }
            if v.open {
                *closed = false;
            } else if v.closed {
                *closed = true;
            }
            let count = polyline_segment_count(points.len(), *closed);
            if let Some(text) = &v.bulges {
                *bulges = bulges_arg(text)?;
            } else if bulges.len() != count {
                // The segments changed: straight ones stay straight, but which bend a curved
                // one should keep isn't ours to guess.
                if bulges.iter().any(|&b| b != 0.0) {
                    return Err(format!(
                        "the polyline now has {count} segments and some were curved: \
                         give --bulges too"
                    ));
                }
                *bulges = vec![0.0; count];
            }
        }
    }
    Ok(geometry)
}
