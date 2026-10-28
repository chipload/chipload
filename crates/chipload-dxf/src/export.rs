// SPDX-License-Identifier: GPL-3.0-or-later
//! Writing shapes to a DXF (docs/specs/dxf-export.md, part A): circles as `CIRCLE`, every
//! other shape as `POLYLINE` with its arcs as bulges, on its layer, in the R12 ASCII layout
//! every reader takes, in mm or inches with `$INSUNITS`. Written here, not by the `dxf`
//! crate, whose writer panics in some time zones (its local-time dates) and fills dates and
//! GUIDs from the clock. The file is the same, byte for byte, for the same shapes on any
//! machine, and it is read back with the importer before it is handed on.

use std::collections::{BTreeMap, BTreeSet};
use std::f64::consts::PI;
use std::fmt::Write;

use chipload_core::Point;
use chipload_core::primitives::{Arc, Line, Primitive, distance};
use chipload_core::tolerances::{ANGLE_TOL_RAD, ARC_APPROX_TOL_MM, CHAIN_TOL_MM};
use chipload_job::{Geometry, Shape, Units};

const MM_PER_INCH: f64 = 25.4;
/// Characters a DXF layer name can't hold.
const NOT_IN_LAYER_NAMES: [char; 11] = ['<', '>', '/', '\\', '"', ':', ';', '?', '*', '|', '='];

/// A DXF written from shapes.
#[derive(Debug, Clone, PartialEq)]
pub struct Exported {
    /// The file.
    pub bytes: Vec<u8>,
    /// Entities written, by DXF type.
    pub entities: BTreeMap<String, usize>,
    /// The layers, as written.
    pub layers: Vec<String>,
    /// Layer names a DXF can't hold, and what they were written as.
    pub renamed: Vec<(String, String)>,
}

/// Why shapes can't be exported.
#[derive(Debug, Clone, PartialEq)]
pub enum ExportError {
    /// No shapes to write.
    Nothing,
    /// Shape `shape` didn't come back the same when the file was read again: never written.
    Unverified { shape: u64, why: String },
}

/// One entity to write: the shape it is from, its layer, and its geometry in mm.
#[derive(Debug, Clone)]
struct Item {
    shape: u64,
    layer: String,
    kind: ItemKind,
}

#[derive(Debug, Clone)]
enum ItemKind {
    Circle {
        center: Point,
        radius: f64,
    },
    /// Lines and arcs end to end (arcs at most a half circle), closed when the last ends
    /// where the first starts and the shape is closed.
    Polyline {
        prims: Vec<Primitive>,
        closed: bool,
    },
}

/// Write `shapes` as a DXF in `units`, each on its layer, in their own order.
pub fn export_shapes(shapes: &[Shape], units: Units) -> Result<Exported, ExportError> {
    let items: Vec<Item> = shapes.iter().flat_map(items_of).collect();
    if items.is_empty() {
        return Err(ExportError::Nothing);
    }
    let (names, renamed) = layer_names(&items);
    let scale = match units {
        Units::Mm => 1.0,
        Units::Inch => 1.0 / MM_PER_INCH,
    };
    let layers: Vec<String> = names
        .values()
        .cloned()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let mut out = Writer::default();
    out.header(units);
    out.tables(&layers);
    out.pair(0, "SECTION");
    out.pair(2, "ENTITIES");
    let mut entities: BTreeMap<String, usize> = BTreeMap::new();
    for item in &items {
        let layer = &names[&item.layer];
        match &item.kind {
            ItemKind::Circle { center, radius } => {
                *entities.entry("CIRCLE".into()).or_default() += 1;
                out.pair(0, "CIRCLE");
                out.pair(8, layer);
                out.point(10, (center.0 * scale, center.1 * scale));
                out.number(40, radius * scale);
            }
            ItemKind::Polyline { prims, closed } => {
                *entities.entry("POLYLINE".into()).or_default() += 1;
                out.polyline(layer, prims, *closed, scale);
            }
        }
    }
    out.pair(0, "ENDSEC");
    out.pair(0, "EOF");
    let bytes = out.text.into_bytes();
    verify(&bytes, &items, &names, units)?;
    Ok(Exported {
        bytes,
        entities,
        layers,
        renamed,
    })
}

/// The DXF's text, group code and value on lines of their own (docs/specs/dxf-export.md
/// §A4, as built): the R12 layout every reader takes, with `$INSUNITS`. Numbers are
/// written in Rust's shortest form that reads back exactly, so the file is the same
/// wherever and whenever it is written.
#[derive(Default)]
struct Writer {
    text: String,
}

impl Writer {
    /// A text value, with any letter outside ASCII written as `\U+XXXX`, as AutoCAD writes
    /// it in files of this layout (which have no UTF-8).
    fn pair(&mut self, code: i32, value: &str) {
        let mut escaped = String::with_capacity(value.len());
        for c in value.chars() {
            if c.is_ascii() {
                escaped.push(c);
            } else {
                for unit in c.encode_utf16(&mut [0u16; 2]) {
                    let _ = write!(escaped, "\\U+{unit:04X}");
                }
            }
        }
        let _ = writeln!(self.text, "{code:>3}\n{escaped}");
    }

    fn int(&mut self, code: i32, value: i32) {
        let _ = writeln!(self.text, "{code:>3}\n{value:>6}");
    }

    fn number(&mut self, code: i32, value: f64) {
        // No "-0": the same point always reads the same.
        let value = if value == 0.0 { 0.0 } else { value };
        let _ = writeln!(self.text, "{code:>3}\n{value:?}");
    }

    fn point(&mut self, code: i32, (x, y): Point) {
        self.number(code, x);
        self.number(code + 10, y);
        self.number(code + 20, 0.0);
    }

    fn header(&mut self, units: Units) {
        self.pair(0, "SECTION");
        self.pair(2, "HEADER");
        self.pair(9, "$ACADVER");
        self.pair(1, "AC1009");
        self.pair(9, "$INSUNITS");
        self.int(
            70,
            match units {
                Units::Mm => 4,
                Units::Inch => 1,
            },
        );
        self.pair(0, "ENDSEC");
    }

    /// The line type every layer draws with, and the layers.
    fn tables(&mut self, layers: &[String]) {
        self.pair(0, "SECTION");
        self.pair(2, "TABLES");
        self.pair(0, "TABLE");
        self.pair(2, "LTYPE");
        self.int(70, 1);
        self.pair(0, "LTYPE");
        self.pair(2, "CONTINUOUS");
        self.int(70, 0);
        self.pair(3, "Solid line");
        self.int(72, 65);
        self.int(73, 0);
        self.number(40, 0.0);
        self.pair(0, "ENDTAB");
        self.pair(0, "TABLE");
        self.pair(2, "LAYER");
        self.int(70, i32::try_from(layers.len()).unwrap_or(i32::MAX));
        for layer in layers {
            self.pair(0, "LAYER");
            self.pair(2, layer);
            self.int(70, 0);
            self.int(62, 7);
            self.pair(6, "CONTINUOUS");
        }
        self.pair(0, "ENDTAB");
        self.pair(0, "ENDSEC");
    }

    /// A polyline: a vertex at each piece's start with the piece's bulge (and, open, one
    /// at the last end), closed by its flag.
    fn polyline(&mut self, layer: &str, prims: &[Primitive], closed: bool, scale: f64) {
        self.pair(0, "POLYLINE");
        self.pair(8, layer);
        self.int(66, 1);
        self.point(10, (0.0, 0.0));
        self.int(70, i32::from(closed));
        let mut vertex = |at: Point, bulge: f64| {
            self.pair(0, "VERTEX");
            self.pair(8, layer);
            self.point(10, (at.0 * scale, at.1 * scale));
            if bulge != 0.0 {
                self.number(42, bulge);
            }
        };
        for prim in prims {
            let bulge = match prim {
                Primitive::Line(_) => 0.0,
                Primitive::Arc(arc) => arc.bulge(),
            };
            vertex(prim.start(), bulge);
        }
        if !closed && let Some(last) = prims.last() {
            vertex(last.end(), 0.0);
        }
        self.pair(0, "SEQEND");
        self.pair(8, layer);
    }
}

/// A shape as the entities that draw it exactly: a circle as one; any other shape as one
/// polyline per run of segments that join end to end.
fn items_of(shape: &Shape) -> Vec<Item> {
    if let Geometry::Circle { center, radius } = shape.geometry {
        return vec![Item {
            shape: shape.id,
            layer: shape.layer.clone(),
            kind: ItemKind::Circle {
                center: (center[0], center[1]),
                radius,
            },
        }];
    }
    let prims: Vec<Primitive> = shape
        .segments()
        .iter()
        .map(chipload_job::PathSegment::to_primitive)
        .filter(|p| p.length() > CHAIN_TOL_MM)
        .flat_map(halves)
        .collect();
    let mut runs: Vec<Vec<Primitive>> = Vec::new();
    for prim in prims {
        match runs.last_mut() {
            Some(run)
                if run
                    .last()
                    .is_some_and(|p| distance(p.end(), prim.start()) <= CHAIN_TOL_MM) =>
            {
                run.push(prim);
            }
            _ => runs.push(vec![prim]),
        }
    }
    let whole = runs.len() == 1;
    runs.into_iter()
        .map(|prims| {
            let ends_meet = match (prims.first(), prims.last()) {
                (Some(first), Some(last)) => distance(last.end(), first.start()) <= CHAIN_TOL_MM,
                _ => false,
            };
            Item {
                shape: shape.id,
                layer: shape.layer.clone(),
                kind: ItemKind::Polyline {
                    closed: whole && shape.closed() && ends_meet && prims.len() > 1,
                    prims,
                },
            }
        })
        .collect()
}

/// An arc past a half circle as two halves (a bulge describes at most one); anything else
/// as it is.
fn halves(prim: Primitive) -> Vec<Primitive> {
    match prim {
        Primitive::Arc(arc) if arc.sweep.abs() > PI => {
            let half = arc.sweep / 2.0;
            vec![
                Primitive::Arc(Arc { sweep: half, ..arc }),
                Primitive::Arc(Arc {
                    start_angle: arc.start_angle + half,
                    sweep: half,
                    ..arc
                }),
            ]
        }
        other => vec![other],
    }
}

/// Layer names a DXF can hold: characters it can't hold become `_`, and names that then
/// clash (with each other, in DXF's case-blind way) get `_2`, `_3`.
fn layer_names(items: &[Item]) -> (BTreeMap<String, String>, Vec<(String, String)>) {
    let mut names: BTreeMap<String, String> = BTreeMap::new();
    let mut taken: BTreeSet<String> = BTreeSet::new();
    let mut renamed = Vec::new();
    for item in items {
        if names.contains_key(&item.layer) {
            continue;
        }
        let cleaned: String = item
            .layer
            .chars()
            .map(|c| {
                if NOT_IN_LAYER_NAMES.contains(&c) || c.is_control() {
                    '_'
                } else {
                    c
                }
            })
            .collect();
        let cleaned = if cleaned.trim().is_empty() {
            "0".to_string()
        } else {
            cleaned.trim().to_string()
        };
        let mut name = cleaned.clone();
        let mut n = 1;
        while taken.contains(&name.to_lowercase()) {
            n += 1;
            name = format!("{cleaned}_{n}");
        }
        taken.insert(name.to_lowercase());
        if name != item.layer {
            renamed.push((item.layer.clone(), name.clone()));
        }
        names.insert(item.layer.clone(), name);
    }
    (names, renamed)
}

/// Read the file back with the importer: every entity must come back as written, on its
/// layer, within the tolerances.
fn verify(
    bytes: &[u8],
    items: &[Item],
    names: &BTreeMap<String, String>,
    units: Units,
) -> Result<(), ExportError> {
    let unverified = |shape: u64, why: &str| ExportError::Unverified {
        shape,
        why: why.to_string(),
    };
    let first = items.first().map_or(0, |i| i.shape);
    let back = crate::shapes_from_dxf_bytes(bytes, "export.dxf", Some(units), 1.0)
        .map_err(|e| unverified(first, &e.to_string()))?;
    if back.shapes.len() != items.len() {
        return Err(unverified(
            first,
            &format!(
                "{} entities written, {} read back",
                items.len(),
                back.shapes.len()
            ),
        ));
    }
    for (item, shape) in items.iter().zip(&back.shapes) {
        if !shape.layer.eq_ignore_ascii_case(&names[&item.layer]) {
            return Err(unverified(item.shape, "it came back on another layer"));
        }
        let read: Vec<Primitive> = shape
            .segments()
            .iter()
            .map(chipload_job::PathSegment::to_primitive)
            .collect();
        let expected: Vec<Primitive> = match &item.kind {
            ItemKind::Circle { center, radius } => vec![Primitive::Arc(Arc {
                center: *center,
                radius: *radius,
                start_angle: 0.0,
                sweep: 2.0 * PI,
            })],
            ItemKind::Polyline { prims, closed } => {
                let mut prims = prims.clone();
                if *closed
                    && let (Some(first), Some(last)) = (prims.first(), prims.last())
                    && distance(last.end(), first.start()) > CHAIN_TOL_MM
                {
                    prims.push(Primitive::Line(Line {
                        start: last.end(),
                        end: first.start(),
                    }));
                }
                prims
            }
        };
        if read.len() != expected.len() {
            return Err(unverified(
                item.shape,
                &format!(
                    "{} pieces written, {} read back",
                    expected.len(),
                    read.len()
                ),
            ));
        }
        for (a, b) in expected.iter().zip(&read) {
            if !same(a, b) {
                return Err(unverified(item.shape, "a piece came back elsewhere"));
            }
        }
    }
    Ok(())
}

/// The same piece, within the tolerances (a whole circle by its centre and radius).
fn same(a: &Primitive, b: &Primitive) -> bool {
    match (a, b) {
        (Primitive::Line(x), Primitive::Line(y)) => {
            distance(x.start, y.start) <= CHAIN_TOL_MM && distance(x.end, y.end) <= CHAIN_TOL_MM
        }
        (Primitive::Arc(x), Primitive::Arc(y)) => {
            let whole = |arc: &Arc| (arc.sweep.abs() - 2.0 * PI).abs() <= ANGLE_TOL_RAD;
            let centre = distance(x.center, y.center) <= ARC_APPROX_TOL_MM
                && (x.radius - y.radius).abs() <= ARC_APPROX_TOL_MM;
            if whole(x) && whole(y) {
                return centre;
            }
            centre
                && distance(a.start(), b.start()) <= CHAIN_TOL_MM
                && distance(a.end(), b.end()) <= CHAIN_TOL_MM
        }
        _ => false,
    }
}
