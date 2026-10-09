// SPDX-License-Identifier: GPL-3.0-or-later
//! Import of individual entity kinds, units and repairs, on drawings built in memory.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::f64::consts::FRAC_PI_2;
use std::path::PathBuf;

use chipload_core::primitives::{Primitive, distance};
use chipload_dxf::{ImportError, ImportResult, UnitsFrom, import_drawing, import_dxf};
use chipload_job::Units;
use dxf::entities::{
    Arc, Circle, Entity, EntityType, Insert, Line, LwPolyline, ModelPoint, Spline, Text,
};
use dxf::enums::Units as DxfUnits;
use dxf::{Block, Drawing, LwPolylineVertex, Point, Vector};

fn drawing(units: DxfUnits) -> Drawing {
    let mut d = Drawing::new();
    d.header.default_drawing_units = units;
    d
}

fn mm() -> Drawing {
    drawing(DxfUnits::Millimeters)
}

fn on_layer(specific: EntityType, layer: &str) -> Entity {
    let mut e = Entity::new(specific);
    e.common.layer = layer.into();
    e
}

fn polyline(points: &[(f64, f64, f64)], closed: bool) -> EntityType {
    let mut p = LwPolyline::default();
    p.set_is_closed(closed);
    p.vertices = points
        .iter()
        .map(|&(x, y, bulge)| LwPolylineVertex {
            x,
            y,
            bulge,
            ..Default::default()
        })
        .collect();
    EntityType::LwPolyline(p)
}

fn square(x: f64, y: f64, size: f64) -> EntityType {
    polyline(
        &[
            (x, y, 0.0),
            (x + size, y, 0.0),
            (x + size, y + size, 0.0),
            (x, y + size, 0.0),
        ],
        true,
    )
}

fn add(d: &mut Drawing, specific: EntityType) -> String {
    d.add_entity(on_layer(specific, "CUT"))
        .common
        .handle
        .as_string()
}

fn import(d: &Drawing, units: Option<Units>) -> ImportResult {
    import_drawing(d, units).unwrap()
}

fn codes(result: &ImportResult) -> Vec<&'static str> {
    result.report.issues.iter().map(|i| i.code).collect()
}

fn xs(result: &ImportResult, contour: usize) -> (f64, f64) {
    let pts = result.contours[contour].points();
    let lo = pts.iter().map(|p| p.0).fold(f64::INFINITY, f64::min);
    let hi = pts.iter().map(|p| p.0).fold(f64::NEG_INFINITY, f64::max);
    (lo, hi)
}

fn near(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol
}

// -- units

#[test]
fn mm_file_with_mm_job() {
    let mut d = mm();
    add(&mut d, square(0.0, 0.0, 10.0));
    let result = import(&d, Some(Units::Mm));
    assert_eq!(result.report.units, Units::Mm);
    assert_eq!(result.report.units_from, UnitsFrom::Dxf);
}

#[test]
fn inch_file_is_scaled_to_mm() {
    let mut d = drawing(DxfUnits::Inches);
    add(&mut d, square(0.0, 0.0, 1.0));
    let result = import(&d, Some(Units::Inch));
    assert!(near(xs(&result, 0).1, 25.4, 1e-9));
}

#[test]
fn unit_mismatch_is_refused() {
    let err = import_drawing(&drawing(DxfUnits::Inches), Some(Units::Mm)).unwrap_err();
    assert!(err.to_string().contains("inch"), "{err}");
}

#[test]
fn unitless_file_uses_job_units_and_says_so() {
    let mut d = drawing(DxfUnits::Unitless);
    add(&mut d, square(0.0, 0.0, 1.0));
    let result = import(&d, Some(Units::Inch));
    assert_eq!(result.report.units_from, UnitsFrom::Job);
    assert!(codes(&result).contains(&"units_from_job"));
    assert!(near(xs(&result, 0).1, 25.4, 1e-9));
}

#[test]
fn unsupported_units_are_refused() {
    let err = import_drawing(&drawing(DxfUnits::Meters), Some(Units::Mm)).unwrap_err();
    assert!(matches!(err, ImportError::UnsupportedUnits(6)), "{err}");
    assert!(err.to_string().contains("INSUNITS=6"));
}

#[test]
fn without_job_units_the_file_units_are_used() {
    let mut d = drawing(DxfUnits::Inches);
    add(&mut d, square(0.0, 0.0, 1.0));
    assert_eq!(import(&d, None).report.units, Units::Inch);
}

#[test]
fn without_job_units_a_unitless_file_is_refused_with_its_own_error() {
    let err = import_drawing(&drawing(DxfUnits::Unitless), None).unwrap_err();
    assert_eq!(err, ImportError::UnitsMissing);
    assert!(err.to_string().contains("no units"));
}

// -- entities

#[test]
fn bulged_polyline_arcs_end_on_vertices() {
    // A slot: two straight sides and two half-circle ends (bulge 1 = 180 degrees).
    let mut d = mm();
    add(
        &mut d,
        polyline(
            &[
                (0.0, 0.0, 0.0),
                (20.0, 0.0, 1.0),
                (20.0, 10.0, 0.0),
                (0.0, 10.0, 1.0),
            ],
            true,
        ),
    );
    let result = import(&d, Some(Units::Mm));
    let contour = &result.contours[0];
    assert!(contour.closed);
    let mut centres: Vec<(f64, f64)> = contour
        .parts
        .iter()
        .filter_map(|p| match p.segment.prim {
            Primitive::Arc(a) => Some((
                (a.center.0 * 1e6).round() / 1e6,
                (a.center.1 * 1e6).round() / 1e6,
            )),
            Primitive::Line(_) => None,
        })
        .collect();
    centres.sort_by(|a, b| a.0.total_cmp(&b.0));
    assert_eq!(centres, [(0.0, 5.0), (20.0, 5.0)]);
    // The slot bulges outwards, so it extends 5 mm beyond the vertices at both ends.
    let (lo, hi) = xs(&result, 0);
    assert!(near(lo, -5.0, 0.01) && near(hi, 25.0, 0.01), "{lo} {hi}");
}

#[test]
fn negative_bulge_curves_the_other_way() {
    let mut d = mm();
    add(
        &mut d,
        polyline(&[(0.0, 0.0, -1.0), (10.0, 0.0, 0.0)], false),
    );
    let prim = import(&d, Some(Units::Mm)).contours[0].parts[0]
        .segment
        .prim;
    assert!(matches!(prim, Primitive::Arc(_)));
    assert!(distance(prim.start(), (0.0, 0.0)) < 1e-9);
    assert!(distance(prim.end(), (10.0, 0.0)) < 1e-9);
    // Bulge −1 from left to right goes up (clockwise).
    assert!(near(prim.midpoint().1, 5.0, 1e-9));
}

#[test]
fn mirrored_arc_extrusion_is_converted_to_wcs() {
    // OCS with extrusion −Z: X is mirrored. CCW 0..90 in OCS is a CW arc from (−5,0) to (0,5).
    let mut d = mm();
    let mut arc = Arc::new(Point::new(0.0, 0.0, 0.0), 5.0, 0.0, 90.0);
    arc.normal = Vector::new(0.0, 0.0, -1.0);
    add(&mut d, EntityType::Arc(arc));
    let prim = import(&d, Some(Units::Mm)).contours[0].parts[0]
        .segment
        .prim;
    let Primitive::Arc(a) = prim else {
        panic!("expected an arc");
    };
    assert!(distance(prim.start(), (-5.0, 0.0)) < 1e-9);
    assert!(distance(prim.end(), (0.0, 5.0)) < 1e-9);
    assert!(near(a.sweep, -FRAC_PI_2, 1e-12));
}

#[test]
fn tilted_entity_is_skipped_and_reported() {
    let mut d = mm();
    let mut circle = Circle::new(Point::new(0.0, 0.0, 0.0), 5.0);
    circle.normal = Vector::new(0.0, 1.0, 1.0);
    add(&mut d, EntityType::Circle(circle));
    let result = import(&d, Some(Units::Mm));
    assert!(result.contours.is_empty());
    assert_eq!(codes(&result), ["entity_skipped"]);
    assert_eq!(result.report.ignored.get("CIRCLE"), Some(&1));
}

#[test]
fn insert_is_exploded_with_transform_and_layer() {
    let mut d = mm();
    let mut block = Block {
        name: "PART".into(),
        ..Default::default()
    };
    block.entities.push(on_layer(square(0.0, 0.0, 10.0), "0"));
    d.add_block(block);
    let insert = Insert {
        name: "PART".into(),
        location: Point::new(100.0, 50.0, 0.0),
        rotation: 90.0,
        x_scale_factor: 2.0,
        y_scale_factor: 2.0,
        ..Default::default()
    };
    let handle = add(&mut d, EntityType::Insert(insert));
    let result = import(&d, Some(Units::Mm));
    let contour = &result.contours[0];
    assert_eq!(contour.layers(), ["CUT"]); // block content on layer 0 takes the INSERT's
    assert_eq!(contour.parts[0].segment.handle, handle);
    let pts = contour.points();
    let (x0, x1) = xs(&result, 0);
    let y0 = pts.iter().map(|p| p.1).fold(f64::INFINITY, f64::min);
    let y1 = pts.iter().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max);
    for (got, want) in [(x0, 80.0), (x1, 100.0), (y0, 50.0), (y1, 70.0)] {
        assert!(near(got, want, 1e-9), "{got} != {want}");
    }
}

#[test]
fn missing_block_is_reported() {
    let mut d = mm();
    let insert = Insert {
        name: "NOPE".into(),
        ..Default::default()
    };
    add(&mut d, EntityType::Insert(insert));
    let result = import(&d, Some(Units::Mm));
    assert_eq!(codes(&result), ["entity_skipped"]);
}

fn clamped_cubic(control: &[(f64, f64)]) -> Spline {
    let mut spline = Spline {
        degree_of_curve: 3,
        ..Default::default()
    };
    spline.control_points = control
        .iter()
        .map(|&(x, y)| Point::new(x, y, 0.0))
        .collect();
    let inner = control.len() - 4;
    spline.knot_values = std::iter::repeat_n(0.0, 4)
        .chain((1..=inner).map(|i| i as f64 / (inner + 1) as f64))
        .chain(std::iter::repeat_n(1.0, 4))
        .collect();
    spline
}

#[test]
fn spline_is_flattened_and_reported() {
    let mut d = mm();
    let spline = clamped_cubic(&[(0.0, 0.0), (10.0, 10.0), (20.0, 0.0), (30.0, 10.0)]);
    add(&mut d, EntityType::Spline(spline));
    let result = import(&d, Some(Units::Mm));
    assert_eq!(result.report.flattened.get("SPLINE"), Some(&1));
    assert_eq!(result.report.imported.get("SPLINE"), Some(&1));
    let contour = &result.contours[0];
    assert!(distance(contour.start(), (0.0, 0.0)) < 1e-9);
    assert!(distance(contour.end(), (30.0, 10.0)) < 1e-9);
}

#[test]
fn fit_point_spline_is_skipped_not_guessed() {
    let mut d = mm();
    let mut spline = Spline {
        degree_of_curve: 3,
        ..Default::default()
    };
    spline.fit_points = vec![Point::new(0.0, 0.0, 0.0), Point::new(10.0, 10.0, 0.0)];
    add(&mut d, EntityType::Spline(spline));
    let result = import(&d, Some(Units::Mm));
    assert_eq!(codes(&result), ["entity_skipped"]);
    assert!(result.report.issues[0].message.contains("fit points only"));
}

#[test]
fn unsupported_entities_are_counted_not_dropped_silently() {
    let mut d = mm();
    add(&mut d, EntityType::Text(Text::default()));
    add(
        &mut d,
        EntityType::ModelPoint(ModelPoint::new(Point::new(1.0, 1.0, 0.0))),
    );
    add(&mut d, square(0.0, 0.0, 10.0));
    let result = import(&d, Some(Units::Mm));
    let ignored: Vec<(&str, usize)> = result
        .report
        .ignored
        .iter()
        .map(|(k, v)| (k.as_str(), *v))
        .collect();
    assert_eq!(ignored, [("POINT", 1), ("TEXT", 1)]);
}

#[test]
fn nonzero_z_is_reported() {
    let mut d = mm();
    add(
        &mut d,
        EntityType::Line(Line::new(
            Point::new(0.0, 0.0, 3.0),
            Point::new(10.0, 0.0, 3.0),
        )),
    );
    assert!(codes(&import(&d, Some(Units::Mm))).contains(&"nonzero_z"));
}

#[test]
fn duplicates_and_zero_length_are_reported() {
    let mut d = mm();
    add(&mut d, square(0.0, 0.0, 10.0));
    // Duplicates a square edge.
    add(
        &mut d,
        EntityType::Line(Line::new(
            Point::new(0.0, 0.0, 0.0),
            Point::new(10.0, 0.0, 0.0),
        )),
    );
    add(
        &mut d,
        EntityType::Line(Line::new(
            Point::new(5.0, 5.0, 0.0),
            Point::new(5.0, 5.0, 0.0),
        )),
    );
    let result = import(&d, Some(Units::Mm));
    let c = codes(&result);
    assert_eq!(c.iter().filter(|&&x| x == "duplicate").count(), 1);
    assert_eq!(c.iter().filter(|&&x| x == "zero_length").count(), 1);
    let closed: Vec<bool> = result.contours.iter().map(|c| c.closed).collect();
    assert_eq!(closed, [true]);
}

#[test]
fn open_contour_is_reported_with_endpoints() {
    let mut d = mm();
    add(
        &mut d,
        polyline(
            &[(0.0, 0.0, 0.0), (10.0, 0.0, 0.0), (10.0, 10.0, 0.0)],
            false,
        ),
    );
    let result = import(&d, Some(Units::Mm));
    assert!(codes(&result).contains(&"open_contour"));
    assert!(!result.contours[0].closed);
    assert!(result.report.issues[0].message.contains("ends at"));
}

#[test]
fn self_intersecting_contour_is_invalid() {
    let mut d = mm();
    add(
        &mut d,
        polyline(
            &[
                (0.0, 0.0, 0.0),
                (10.0, 10.0, 0.0),
                (10.0, 0.0, 0.0),
                (0.0, 10.0, 0.0),
            ],
            true,
        ),
    );
    let result = import(&d, Some(Units::Mm));
    assert!(result.contours[0].invalid_reason.is_some());
    assert!(codes(&result).contains(&"invalid_contour"));
}

#[test]
fn nesting_depth() {
    let mut d = mm();
    add(&mut d, square(0.0, 0.0, 100.0));
    add(&mut d, square(10.0, 10.0, 30.0));
    add(&mut d, square(15.0, 15.0, 5.0));
    add(&mut d, square(200.0, 0.0, 10.0));
    let depths: Vec<Option<usize>> = import(&d, Some(Units::Mm))
        .contours
        .iter()
        .map(|c| c.nesting_depth)
        .collect();
    assert_eq!(depths, [Some(0), Some(1), Some(2), Some(0)]);
}

// -- the real drawing (LibreCAD/dxfrw 0.6.3, mm)

fn messy_2() -> ImportResult {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/corpus/dxf/messy_2.dxf");
    import_dxf(&path, None).unwrap()
}

#[test]
fn messy_2_units_counts_and_extents() {
    let result = messy_2();
    let report = &result.report;
    assert_eq!(report.units, Units::Mm);
    let imported: Vec<(&str, usize)> = report
        .imported
        .iter()
        .map(|(k, v)| (k.as_str(), *v))
        .collect();
    assert_eq!(imported, [("CIRCLE", 5), ("LWPOLYLINE", 17)]);
    assert!(report.ignored.is_empty() && report.flattened.is_empty());
    let ((x0, y0), (x1, y1)) = report.bbox.unwrap();
    for (got, want) in [(x0, -90.499), (y0, -50.0), (x1, 90.499), (y1, 55.0)] {
        assert!(near(got, want, 0.001), "{got} != {want}");
    }
}

#[test]
fn messy_2_only_the_two_single_lines_are_open() {
    let result = messy_2();
    let open: Vec<_> = result
        .contours
        .iter()
        .filter(|c| !c.closed)
        .map(|c| (c.layers(), c.start(), c.end()))
        .collect();
    assert_eq!(open.len(), 2);
    assert_eq!(open[0].0, ["lower"]);
    assert_eq!(open[1].0, ["upper"]);
    for ((_, s, e), x) in open.iter().zip([-64.75, 64.75]) {
        assert!(distance(*s, (x, 50.0)) < 1e-9 && distance(*e, (x, 20.0)) < 1e-9);
    }
}

#[test]
fn messy_2_every_closed_contour_is_valid_and_layers_add_up() {
    let result = messy_2();
    let closed: Vec<_> = result.contours.iter().filter(|c| c.closed).collect();
    assert_eq!(closed.len(), 20);
    assert!(closed.iter().all(|c| c.invalid_reason.is_none()));
    let count = |layer: &str| {
        result
            .contours
            .iter()
            .filter(|c| c.layers() == [layer])
            .count()
    };
    assert_eq!((count("0"), count("lower"), count("upper")), (3, 4, 15));
}
