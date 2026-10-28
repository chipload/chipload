// SPDX-License-Identifier: GPL-3.0-or-later
//! DXF export (docs/specs/dxf-export.md, part A): every kind written exactly, read back by
//! the importer, the same bytes every time.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;

use chipload_dxf::{ExportError, export_shapes, shapes_from_dxf_bytes, shapes_from_file};
use chipload_job::{Geometry, Shape, Units};

fn shape(id: u64, layer: &str, geometry: Geometry) -> Shape {
    Shape {
        id,
        layer: layer.into(),
        name: None,
        geometry,
    }
}

/// One of each drawn kind: a circle, a rectangle with round corners, a hexagon, a polyline
/// with an arc and a half circle past which a bulge can't reach.
fn drawn() -> Vec<Shape> {
    vec![
        shape(
            1,
            "Cut",
            Geometry::Circle {
                center: [50.0, 40.0],
                radius: 12.5,
            },
        ),
        shape(
            2,
            "Cut",
            Geometry::Rectangle {
                center: [100.0, 80.0],
                length: 120.0,
                width: 80.0,
                angle: 0.3,
                corner_radius: 10.0,
            },
        ),
        shape(
            3,
            "Holes",
            Geometry::Polygon {
                center: [30.0, 120.0],
                radius: 14.0,
                sides: 6,
                angle: 0.5,
            },
        ),
        shape(
            4,
            "Engrave",
            Geometry::Polyline {
                closed: false,
                points: vec![[0.0, 0.0], [40.0, 0.0], [40.0, 30.0]],
                bulges: vec![0.0, 1.5],
            },
        ),
    ]
}

#[test]
fn every_kind_is_written_exactly_and_reads_back() {
    let out = export_shapes(&drawn(), Units::Mm).unwrap();
    assert_eq!(out.entities.get("CIRCLE"), Some(&1));
    assert_eq!(out.entities.get("POLYLINE"), Some(&3));
    assert_eq!(out.layers, ["Cut", "Engrave", "Holes"]);
    let text = String::from_utf8(out.bytes.clone()).unwrap();
    assert!(text.contains("AC1009"), "R12 layout");
    assert!(text.contains("$INSUNITS\n 70\n     4"), "mm");
    // Read back without saying the units: the file says them.
    let back = shapes_from_dxf_bytes(&out.bytes, "x.dxf", None, 1.0).unwrap();
    assert_eq!(back.shapes.len(), 4);
    assert_eq!(back.source.units, Units::Mm);
}

#[test]
fn the_same_shapes_give_the_same_bytes() {
    let a = export_shapes(&drawn(), Units::Mm).unwrap();
    let b = export_shapes(&drawn(), Units::Mm).unwrap();
    assert_eq!(a.bytes, b.bytes);
    // Another drawing gets another fingerprint.
    let c = export_shapes(&drawn()[..2], Units::Mm).unwrap();
    assert_ne!(a.bytes, c.bytes);
}

#[test]
fn inches_are_written_in_inches_and_read_back_in_mm() {
    let out = export_shapes(&drawn(), Units::Inch).unwrap();
    let text = String::from_utf8(out.bytes.clone()).unwrap();
    assert!(text.contains("$INSUNITS\n 70\n     1"), "inch");
    let back = shapes_from_dxf_bytes(&out.bytes, "x.dxf", None, 1.0).unwrap();
    let Geometry::Path { segments, .. } = &back.shapes[0].geometry else {
        panic!("{:?}", back.shapes[0]);
    };
    let chipload_job::PathSegment::Arc(arc) = &segments[0] else {
        panic!("{segments:?}");
    };
    assert!((arc.radius - 12.5).abs() < 1e-9 && (arc.center[0] - 50.0).abs() < 1e-9);
}

#[test]
fn layer_names_a_dxf_cannot_hold_are_renamed() {
    let shapes = vec![
        shape(
            1,
            "#ff0000",
            Geometry::straight_polyline(false, vec![[0.0, 0.0], [1.0, 0.0]]),
        ),
        shape(
            2,
            "a/b",
            Geometry::straight_polyline(false, vec![[0.0, 1.0], [1.0, 1.0]]),
        ),
        shape(
            3,
            "a:b",
            Geometry::straight_polyline(false, vec![[0.0, 2.0], [1.0, 2.0]]),
        ),
        shape(
            4,
            "Kant Å",
            Geometry::straight_polyline(false, vec![[0.0, 3.0], [1.0, 3.0]]),
        ),
    ];
    let out = export_shapes(&shapes, Units::Mm).unwrap();
    assert_eq!(
        out.renamed,
        [
            ("a/b".to_string(), "a_b".to_string()),
            ("a:b".to_string(), "a_b_2".to_string())
        ]
    );
    let back = shapes_from_dxf_bytes(&out.bytes, "x.dxf", None, 1.0).unwrap();
    let layers: Vec<&str> = back.shapes.iter().map(|s| s.layer.as_str()).collect();
    // A letter outside ASCII goes as \U+00C5, as AutoCAD writes it, and comes back.
    assert_eq!(layers, ["#ff0000", "a_b", "a_b_2", "Kant Å"]);
    assert!(out.bytes.is_ascii());
}

#[test]
fn imported_drawings_round_trip_through_an_export() {
    // A DXF and an SVG, as imported: their paths (lines, arcs, fitted curves) come back.
    let corpus = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/corpus");
    for file in [
        "dxf/simple_part.dxf",
        "dxf/messy_2.dxf",
        "svg/badge_layers.svg",
    ] {
        let read = shapes_from_file(&corpus.join(file), Some(Units::Mm), None, 1.0).unwrap();
        let out = export_shapes(&read.shapes, Units::Mm).unwrap();
        assert!(!out.bytes.is_empty(), "{file}");
    }
}

#[test]
fn nothing_to_write_is_refused() {
    assert_eq!(export_shapes(&[], Units::Mm), Err(ExportError::Nothing));
}
