// SPDX-License-Identifier: GPL-3.0-or-later
//! Job model: a declarative description of how to machine a drawing.
//!
//! Job file models (schema v9: a project with its drawing embedded, docs/specs/
//! project-file.md, shapes drawn in Chipload, docs/specs/drawing-tools.md, and the imported
//! drawing's placement, docs/specs/drawing-placement.md and drawing-rotation.md). All lengths are mm, all feeds
//! mm/min, angles radians, `depth` is
//! measured down from the stock top. Unknown fields are rejected; value constraints are
//! checked by [`Job::validate`].

use std::collections::BTreeMap;

use chipload_core::primitives::{Arc, Line, Primitive, distance};
use chipload_core::tolerances::{
    ANGLE_TOL_RAD, ARC_APPROX_TOL_MM, CHAIN_TOL_MM, DEPTH_TOL_MM, REST_TOL_MM,
};

use crate::Problem;
use chipload_i18n::{Digits, Message, Value};
use serde::{Deserialize, Serialize};

/// The schema version this code reads and writes.
pub const CURRENT_VERSION: i64 = 33;

/// The oldest schema version that still loads (decision 2026-10-10: versions from before
/// projects, 1-3, were never released and are no longer read).
pub const OLDEST_VERSION: i64 = 4;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Job {
    /// The version of the file format. Chipload upgrades older projects when it opens them,
    /// and refuses newer ones.
    pub schema_version: i64,
    /// The Chipload version that wrote the file, for reference.
    #[serde(default)]
    pub written_by: Option<String>,
    /// The drawing: the shapes Chipload cuts, imported from a DXF or drawn in Chipload.
    pub drawing: Drawing,
    /// The material on the machine.
    pub stock: Stock,
    /// The machine: post-processor, safe height, limits and zero.
    pub machine: Machine,
    /// The tools the operations cut with.
    pub tools: Vec<Tool>,
    /// The parts: groups of operations, each cut with its copies before the next part.
    /// At least one.
    // Spec: docs/specs/parts.md.
    pub parts: Vec<Part>,
    /// The part new operations go into (its id). Null or no such part: the first part.
    /// Only the app and the command line read it; it changes nothing that is cut.
    #[serde(default)]
    pub active_part: Option<u64>,
    /// The operations, cut in this order: grouped by part, in the parts' order.
    pub operations: Vec<Operation>,
}

/// A part: a named group of operations, cut together, with its own copies
/// (docs/specs/parts.md §2). It owns no geometry: its operations select the shapes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Part {
    /// Its number, unique in the project: what operations name it by.
    pub id: u64,
    /// Its name, unique in the project, shown in the Browser, the report and the sheet.
    pub name: String,
    /// False: the part and all its operations stay in the project but aren't cut.
    pub enabled: bool,
    /// Cut the part several times in a grid. Null: once.
    // Spec: docs/specs/copies-grid.md.
    pub copies: Option<Copies>,
}

impl Part {
    /// How many times the part is cut.
    pub fn count(&self) -> u32 {
        self.copies.map_or(1, |c| c.count())
    }

    /// Its copies as (number, shift) in cut order; one, unshifted, without a grid.
    pub fn in_cut_order(&self) -> Vec<(u32, (f64, f64))> {
        self.copies
            .map_or_else(|| vec![(1, (0.0, 0.0))], |c| c.in_cut_order())
    }
}

/// Most columns or rows of copies, and most copies in all.
pub const MAX_COPIES_PER_SIDE: u32 = 100;
pub const MAX_COPIES: u32 = 1000;

/// A grid of copies of a part: copy (column c, row r) is the part moved by (c·dx, r·dy)
/// mm.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Copies {
    /// Copies along X, at least 1.
    pub columns: u32,
    /// Copies along Y, at least 1.
    pub rows: u32,
    /// Distance from copy to copy along X, mm; negative grows to the left.
    pub dx: f64,
    /// Distance from copy to copy along Y, mm; negative grows downwards.
    pub dy: f64,
    /// `by_operation`: each operation on every copy, then the next; `by_copy`: every
    /// operation on one copy, then the next copy.
    #[serde(default)]
    pub order: CopyOrder,
}

/// In which order the copies are cut.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum CopyOrder {
    /// Each operation on every copy, then the next operation.
    #[default]
    ByOperation,
    /// Every operation on one copy, then the next copy. Called `by_part` before parts
    /// came (schema 22).
    ByCopy,
}

impl Copies {
    pub fn count(&self) -> u32 {
        self.columns.saturating_mul(self.rows)
    }

    /// Every copy as (number, shift), in cut order: row by row from copy 1, alternating
    /// direction (1, 2, 3, then 6, 5, 4). Copies are numbered row by row from 1, the
    /// drawing as drawn.
    pub fn in_cut_order(&self) -> Vec<(u32, (f64, f64))> {
        let mut out = Vec::new();
        for row in 0..self.rows {
            let mut columns: Vec<u32> = (0..self.columns).collect();
            if row % 2 == 1 {
                columns.reverse();
            }
            for column in columns {
                let number = row * self.columns + column + 1;
                out.push((
                    number,
                    (f64::from(column) * self.dx, f64::from(row) * self.dy),
                ));
            }
        }
        out
    }

    /// What is wrong with the grid, as "{at}.field: message" (`at` is its path, e.g.
    /// "parts.0.copies").
    pub fn problems(&self, at: &str) -> Vec<Problem> {
        let mut problems = Vec::new();
        for (name, n) in [("columns", self.columns), ("rows", self.rows)] {
            if !(1..=MAX_COPIES_PER_SIDE).contains(&n) {
                problems.push(Problem::at(
                    format!("{at}.{name}"),
                    Message::new("job-copies-per-side")
                        .arg("most", MAX_COPIES_PER_SIDE)
                        .arg("got", n),
                ));
            }
        }
        if self.count() > MAX_COPIES {
            problems.push(Problem::at(
                at,
                Message::new("job-copies-too-many")
                    .arg("most", MAX_COPIES)
                    .arg("got", self.count()),
            ));
        }
        for (name, pitch, n) in [("dx", self.dx, self.columns), ("dy", self.dy, self.rows)] {
            if !pitch.is_finite() {
                problems.push(Problem::at(
                    format!("{at}.{name}"),
                    Message::new("job-must-be-number")
                        .arg("got", Value::LengthNumber(pitch, Digits::Shortest)),
                ));
            } else if n > 1 && pitch == 0.0 {
                problems.push(Problem::at(
                    format!("{at}.{name}"),
                    Message::new("job-copies-pitch-zero"),
                ));
            }
        }
        problems
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum Units {
    Mm,
    Inch,
}

impl Units {
    pub fn name(self) -> &'static str {
        match self {
            Units::Mm => "mm",
            Units::Inch => "inch",
        }
    }

    /// Millimetres in one of these units.
    pub fn mm_per_unit(self) -> f64 {
        match self {
            Units::Mm => 1.0,
            Units::Inch => chipload_core::MM_PER_INCH,
        }
    }
}

/// The units a message is shown in (docs/specs/translations.md §7).
impl From<Units> for chipload_i18n::Units {
    fn from(units: Units) -> Self {
        match units {
            Units::Mm => chipload_i18n::Units::Mm,
            Units::Inch => chipload_i18n::Units::Inch,
        }
    }
}

/// The project's drawing: shapes in millimetres, or (only for jobs from before v4) a
/// reference to a DXF that still has to be embedded.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Drawing {
    /// The shapes, imported and drawn.
    pub shapes: Vec<Shape>,
    /// The id the next new shape gets. Ids are never reused.
    pub next_shape_id: u64,
    /// The DXFs the imported shapes came from, each as at its last import or update, with
    /// its own placement.
    // Spec: docs/specs/drawing-sources.md §2.
    #[serde(default)]
    pub dxfs: Vec<DrawingSource>,
    /// Pictures traced into shapes, for V-carving.
    // Spec: docs/specs/image-trace.md §4.
    #[serde(default)]
    pub images: Vec<Image>,
    /// Texts written into shapes, for V-carving, pocketing or profiling. Left out of the
    /// file when there are none, so projects without text open in older versions.
    // Spec: docs/specs/text.md §3.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub texts: Vec<Text>,
    /// Groups and layers not drawn on the canvas (they are still cut): `dxf:1`,
    /// `dxf:1/layer:Cut`, `image:1`, `text:1`, `drawn:circle`, `drawn:rectangle`,
    /// `drawn:polyline`.
    // Spec: docs/specs/drawing-sources.md §4.
    #[serde(default)]
    pub hidden: Vec<String>,
}

/// A picture traced into shapes: the picture itself, where it goes, and how it becomes black
/// and white. Its traced shapes are `path` shapes on its layer that name it in `image`.
// Spec: docs/specs/image-trace.md §2, §4.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Image {
    /// The image's id, which its traced shapes name.
    pub id: u64,
    /// The picture's file name.
    pub file: String,
    /// The picture's SHA-256.
    pub sha256: String,
    /// The picture file itself (PNG, JPEG or BMP), base64-encoded.
    pub data: String,
    /// The layer its traced shapes go on.
    pub layer: String,
    /// X of the picture's lower-left corner on the stock, mm.
    pub x: f64,
    /// Y of the picture's lower-left corner on the stock, mm.
    pub y: f64,
    /// The picture's width on the stock, mm; its height follows from the picture.
    pub width: f64,
    /// Turned counter-clockwise about the picture's centre, degrees, at least 0 and less
    /// than 360. `x`, `y` and `width` are before the turn; the traced shapes are turned
    /// after the trace.
    // Spec: docs/specs/direct-placement.md §3.
    pub rotation: f64,
    /// Grey values below this (0–255) are black: 1 to 254, or "auto" to pick it from the
    /// picture (Otsu's method).
    pub threshold: ImageThreshold,
    /// Carve the light parts instead of the dark ones.
    pub invert: bool,
    /// Blur before the threshold, in pixels, 0 to 10; 0: none.
    pub blur: f64,
    /// Black or white specks smaller than this are dropped, mm², 0 to 100.
    pub min_area: f64,
    /// How far a traced edge may be simplified away from the picture's edge, mm, 0.01 to 1.
    pub smoothing: f64,
    /// What it is traced into: the black parts' outlines (for a V-carve) or their centre
    /// lines (for a follow path).
    // Spec: docs/specs/image-arcs-centrelines.md.
    pub trace: ImageTrace,
    /// Traced edges are fitted with arcs as well as lines.
    pub arcs: bool,
}

/// What a picture is traced into.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum ImageTrace {
    /// Closed outlines round the black parts, black inside.
    Outline,
    /// Open (or closed) paths along the black strokes' middles.
    CentreLine,
}

/// Where grey turns black: "auto" or a grey value.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(untagged)]
pub enum ImageThreshold {
    /// Picked from the picture.
    Auto(AutoThreshold),
    /// Grey values below this are black, 1 to 254.
    Level(u8),
}

/// The word "auto".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum AutoThreshold {
    Auto,
}

impl ImageThreshold {
    pub const AUTO: ImageThreshold = ImageThreshold::Auto(AutoThreshold::Auto);
}

/// The ranges of an image's settings (docs/specs/image-trace.md §2).
pub const IMAGE_MAX_BLUR_PX: f64 = 10.0;
pub const IMAGE_MAX_MIN_AREA_MM2: f64 = 100.0;
pub const IMAGE_MIN_SMOOTHING_MM: f64 = 0.01;
pub const IMAGE_MAX_SMOOTHING_MM: f64 = 1.0;

impl Image {
    /// What is wrong with the image's values, as "drawing.images.N.field: message".
    pub fn problems(&self, index: usize) -> Vec<Problem> {
        let at = format!("drawing.images.{index}");
        let mut problems = Vec::new();
        let mut range = |field: &str, value: f64, (low, high): (f64, f64), message: Message| {
            if !(value.is_finite() && (low..=high).contains(&value)) {
                problems.push(Problem::at(format!("{at}.{field}"), message));
            }
        };
        let number = |v: f64| Value::Number(v, Digits::Shortest);
        let length = |v: f64| Value::Length(v, Digits::Shortest);
        range(
            "blur",
            self.blur,
            (0.0, IMAGE_MAX_BLUR_PX),
            Message::new("job-image-range-px")
                .arg("low", number(0.0))
                .arg("high", number(IMAGE_MAX_BLUR_PX))
                .arg("got", number(self.blur)),
        );
        range(
            "min_area",
            self.min_area,
            (0.0, IMAGE_MAX_MIN_AREA_MM2),
            Message::new("job-image-range-area")
                .arg("low", number(0.0))
                .arg("high", number(IMAGE_MAX_MIN_AREA_MM2))
                .arg("got", number(self.min_area)),
        );
        range(
            "smoothing",
            self.smoothing,
            (IMAGE_MIN_SMOOTHING_MM, IMAGE_MAX_SMOOTHING_MM),
            Message::new("job-image-range-length")
                .arg(
                    "low",
                    Value::LengthNumber(IMAGE_MIN_SMOOTHING_MM, Digits::Shortest),
                )
                .arg("high", length(IMAGE_MAX_SMOOTHING_MM))
                .arg("got", length(self.smoothing)),
        );
        if let ImageThreshold::Level(t) = self.threshold
            && !(1..=254).contains(&t)
        {
            problems.push(Problem::at(
                format!("{at}.threshold"),
                Message::new("job-image-threshold").arg("got", u32::from(t)),
            ));
        }
        if !(self.width.is_finite() && self.width > 0.0) {
            problems.push(Problem::at(
                format!("{at}.width"),
                Message::new("job-must-be-positive")
                    .arg("got", Value::Length(self.width, Digits::Shortest)),
            ));
        }
        if !rotation_in_range(self.rotation) {
            problems.push(Problem::at(
                format!("{at}.rotation"),
                Message::new("job-rotation-range")
                    .arg("got", Value::Number(self.rotation, Digits::Shortest)),
            ));
        }
        if !(self.x.is_finite() && self.y.is_finite()) {
            problems.push(Problem::at(
                &at,
                Message::new("job-xy-must-be-numbers")
                    .arg("x", Value::LengthNumber(self.x, Digits::Shortest))
                    .arg("y", Value::LengthNumber(self.y, Digits::Shortest)),
            ));
        }
        if self.layer.trim().is_empty() {
            problems.push(Problem::at(
                format!("{at}.layer"),
                Message::new("job-must-not-be-empty"),
            ));
        }
        if self.data.is_empty() {
            problems.push(Problem::at(
                format!("{at}.data"),
                Message::new("job-image-missing"),
            ));
        }
        problems
    }
}

/// A text written into shapes: what it says, its font and size, and where it goes. Its
/// letters are `path` shapes on its layer that name it in `text`, made again when it
/// changes; the font is needed only for that, not to cut them.
// Spec: docs/specs/text.md §3.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Text {
    /// The text's id, which its letters name.
    pub id: u64,
    /// What it says; a line break ("\n") starts a new line.
    pub text: String,
    /// A built-in font: FreeSans, FreeSans Bold, FreeSerif, FreeSerif Bold, FreeMono or
    /// FreeMono Bold (`chipload text fonts`).
    pub font: String,
    /// The height of a capital letter, mm.
    pub height: f64,
    /// X of the text's anchor on the stock, mm: the lower-left corner of its box (before
    /// the turn) or its centre.
    pub x: f64,
    /// Y of the text's anchor on the stock, mm.
    pub y: f64,
    /// What `x` and `y` are: the box's lower-left corner (before the turn), or its centre.
    /// A centred text stays centred there when its words, font or size change. Missing:
    /// the corner.
    #[serde(default)]
    pub anchor: TextAnchor,
    /// Turned counter-clockwise about the box's centre, degrees, at least 0 and less than
    /// 360.
    pub rotation: f64,
    /// How the lines line up against each other.
    pub align: TextAlign,
    /// Extra space between letters, mm; 0: the font's own spacing, kerning included.
    pub spacing: f64,
    /// From one line's base to the next, in capital heights.
    pub line_spacing: f64,
    /// The layer its letters go on.
    pub layer: String,
}

/// What a text's position is (docs/specs/text.md §3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum TextAnchor {
    /// The lower-left corner of the text's box, before the turn.
    #[default]
    Corner,
    /// The centre of the text's box (the turn keeps it in place).
    Center,
}

/// How a text's lines line up.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum TextAlign {
    Left,
    Center,
    Right,
}

/// The ranges of a text's settings (docs/specs/text.md §3).
pub const TEXT_MAX_HEIGHT_MM: f64 = 1000.0;
pub const TEXT_MAX_LINE_SPACING: f64 = 10.0;
/// Letter spacing, as a share of the height either way.
pub const TEXT_MAX_SPACING_HEIGHTS: f64 = 2.0;
/// A new text's line spacing, in capital heights.
pub const TEXT_DEFAULT_LINE_SPACING: f64 = 1.6;

impl Text {
    /// What is wrong with the text's values, as "drawing.texts.N.field: message".
    pub fn problems(&self, index: usize) -> Vec<Problem> {
        let at = format!("drawing.texts.{index}");
        let mut problems = Vec::new();
        if self.text.trim().is_empty() {
            problems.push(Problem::at(
                format!("{at}.text"),
                Message::new("job-text-empty"),
            ));
        }
        if !(self.height.is_finite() && self.height > 0.0 && self.height <= TEXT_MAX_HEIGHT_MM) {
            problems.push(Problem::at(
                format!("{at}.height"),
                Message::new("job-text-height")
                    .arg("most", Value::Length(TEXT_MAX_HEIGHT_MM, Digits::Shortest))
                    .arg("got", Value::Length(self.height, Digits::Shortest)),
            ));
        }
        if !(self.line_spacing.is_finite()
            && self.line_spacing > 0.0
            && self.line_spacing <= TEXT_MAX_LINE_SPACING)
        {
            problems.push(Problem::at(
                format!("{at}.line_spacing"),
                Message::new("job-more-than-0-at-most")
                    .arg(
                        "most",
                        Value::Number(TEXT_MAX_LINE_SPACING, Digits::Shortest),
                    )
                    .arg("got", Value::Number(self.line_spacing, Digits::Shortest)),
            ));
        }
        let most = TEXT_MAX_SPACING_HEIGHTS * self.height.abs();
        if !(self.spacing.is_finite() && self.spacing.abs() <= most) {
            problems.push(Problem::at(
                format!("{at}.spacing"),
                Message::new("job-text-spacing")
                    .arg(
                        "heights",
                        Value::Number(TEXT_MAX_SPACING_HEIGHTS, Digits::Shortest),
                    )
                    .arg("most", Value::Length(most, Digits::Shortest))
                    .arg("got", Value::Length(self.spacing, Digits::Shortest)),
            ));
        }
        if !rotation_in_range(self.rotation) {
            problems.push(Problem::at(
                format!("{at}.rotation"),
                Message::new("job-rotation-range")
                    .arg("got", Value::Number(self.rotation, Digits::Shortest)),
            ));
        }
        if !(self.x.is_finite() && self.y.is_finite()) {
            problems.push(Problem::at(
                &at,
                Message::new("job-xy-must-be-numbers")
                    .arg("x", Value::LengthNumber(self.x, Digits::Shortest))
                    .arg("y", Value::LengthNumber(self.y, Digits::Shortest)),
            ));
        }
        if self.layer.trim().is_empty() {
            problems.push(Problem::at(
                format!("{at}.layer"),
                Message::new("job-must-not-be-empty"),
            ));
        }
        if self.font.trim().is_empty() {
            problems.push(Problem::at(
                format!("{at}.font"),
                Message::new("job-must-not-be-empty"),
            ));
        }
        problems
    }
}

/// One shape of the drawing: imported from a DXF (`path`: lines and arcs, one shape per DXF
/// entity), or drawn in Chipload (`circle`, `rectangle`, `polyline`). One flat object:
/// `id`, `layer`, `kind`, then the fields of that kind.
// Spec: docs/specs/drawing-tools.md.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(try_from = "RawShape", into = "RawShape")]
pub struct Shape {
    pub id: u64,
    pub layer: String,
    /// A drawn shape's name in the Browser; None: shown by its id
    /// (docs/specs/drawn-shapes-box.md §4).
    pub name: Option<String>,
    pub geometry: Geometry,
    /// Construction geometry: shown and snapped to, never cut (drawn shapes only).
    // Spec: docs/specs/dxf-export.md, part B (schema 32).
    pub construction: bool,
}

/// What a shape is. Drawn kinds store their defining values; their segments are derived.
#[derive(Debug, Clone, PartialEq)]
pub enum Geometry {
    /// Imported: lines and arcs exactly as read from the DXF.
    Path {
        /// Handle of the DXF entity it was imported from (the INSERT's for block content).
        dxf_handle: Option<String>,
        /// DXF type of that entity, e.g. "LWPOLYLINE".
        dxf_type: Option<String>,
        /// The last segment ends where the first starts.
        closed: bool,
        segments: Vec<PathSegment>,
        /// The image it was traced from (docs/specs/image-trace.md §4).
        image: Option<u64>,
        /// The DXF it was imported from (an id in `drawing.dxfs`).
        dxf: Option<u64>,
        /// The text whose letter it is (docs/specs/text.md §3).
        text: Option<u64>,
    },
    Circle {
        center: [f64; 2],
        radius: f64,
    },
    /// `length` along the local X axis, `width` along local Y, rotated by `angle` (radians,
    /// CCW) about `center`; its corners rounded by `corner_radius` (0: sharp), at most half
    /// the shorter side (docs/specs/rectangle-corner-radius.md).
    Rectangle {
        center: [f64; 2],
        length: f64,
        width: f64,
        angle: f64,
        corner_radius: f64,
    },
    /// Segments through `points`, and back to the first if `closed`. Each segment bends by
    /// its bulge, as in DXF: tan(sweep / 4), 0 straight, ±1 a half circle (positive
    /// counter-clockwise); one per segment (docs/specs/drawing-follow-ups.md section 3).
    Polyline {
        closed: bool,
        points: Vec<[f64; 2]>,
        bulges: Vec<f64>,
    },
    /// A regular polygon: `sides` corners on the circle of `radius` about `center`, the first
    /// at `angle` (radians, CCW from +X) (docs/specs/drawing-input.md §5).
    Polygon {
        center: [f64; 2],
        radius: f64,
        sides: u32,
        angle: f64,
    },
}

impl Shape {
    pub fn kind_name(&self) -> &'static str {
        match self.geometry {
            Geometry::Path { .. } => "path",
            Geometry::Circle { .. } => "circle",
            Geometry::Rectangle { .. } => "rectangle",
            Geometry::Polyline { .. } => "polyline",
            Geometry::Polygon { .. } => "polygon",
        }
    }

    /// Drawn in Chipload (editable), as opposed to imported.
    pub fn is_drawn(&self) -> bool {
        !matches!(self.geometry, Geometry::Path { .. })
    }

    /// Whether the shape's segments form a loop (as stored for paths).
    pub fn closed(&self) -> bool {
        match &self.geometry {
            Geometry::Path { closed, .. } | Geometry::Polyline { closed, .. } => *closed,
            Geometry::Circle { .. } | Geometry::Rectangle { .. } | Geometry::Polygon { .. } => true,
        }
    }

    pub fn dxf_handle(&self) -> Option<&str> {
        match &self.geometry {
            Geometry::Path { dxf_handle, .. } => dxf_handle.as_deref(),
            _ => None,
        }
    }

    /// Imported from a DXF: a path not traced from an image nor a text's letter. Its DXF's
    /// placement moves it.
    pub fn from_dxf(&self) -> bool {
        matches!(
            self.geometry,
            Geometry::Path {
                image: None,
                text: None,
                ..
            }
        )
    }

    /// The text this shape is a letter of.
    pub fn text(&self) -> Option<u64> {
        match &self.geometry {
            Geometry::Path { text, .. } => *text,
            _ => None,
        }
    }

    /// The DXF this shape was imported from.
    pub fn dxf(&self) -> Option<u64> {
        match &self.geometry {
            Geometry::Path { dxf, .. } => *dxf,
            _ => None,
        }
    }

    /// The image this shape was traced from.
    pub fn image(&self) -> Option<u64> {
        match &self.geometry {
            Geometry::Path { image, .. } => *image,
            _ => None,
        }
    }

    pub fn dxf_type(&self) -> Option<&str> {
        match &self.geometry {
            Geometry::Path { dxf_type, .. } => dxf_type.as_deref(),
            _ => None,
        }
    }

    /// The shape as lines and arcs (spec drawing-tools.md section 2). Exact and
    /// deterministic: nothing is rounded.
    pub fn segments(&self) -> Vec<PathSegment> {
        match &self.geometry {
            Geometry::Path { segments, .. } => segments.clone(),
            // As an imported DXF CIRCLE: one full arc starting at angle 0.
            Geometry::Circle { center, radius } => vec![PathSegment::Arc(ArcSegment {
                center: *center,
                radius: *radius,
                start_angle: 0.0,
                sweep: std::f64::consts::TAU,
            })],
            Geometry::Rectangle {
                center,
                length,
                width,
                angle,
                corner_radius,
            } => {
                let (sin, cos) = angle.sin_cos();
                let (hl, hw) = (length / 2.0, width / 2.0);
                let place =
                    |x: f64, y: f64| [center[0] + x * cos - y * sin, center[1] + x * sin + y * cos];
                if *corner_radius == 0.0 {
                    // Counter-clockwise from the (-length/2, -width/2) corner.
                    let corners: Vec<[f64; 2]> = [(-hl, -hw), (hl, -hw), (hl, hw), (-hl, hw)]
                        .iter()
                        .map(|&(x, y)| place(x, y))
                        .collect();
                    lines_through(&corners, true)
                } else {
                    rounded_rectangle(hl, hw, *corner_radius, *angle, place)
                }
            }
            Geometry::Polyline {
                closed,
                points,
                bulges,
            } => polyline_segments(points, bulges, *closed),
            Geometry::Polygon {
                center,
                radius,
                sides,
                angle,
            } => lines_through(&polygon_corners(*center, *radius, *sides, *angle), true),
        }
    }
}

/// A regular polygon's corners, counter-clockwise from the one at `angle`.
pub fn polygon_corners(center: [f64; 2], radius: f64, sides: u32, angle: f64) -> Vec<[f64; 2]> {
    let n = f64::from(sides.max(1));
    (0..sides)
        .map(|k| {
            let (sin, cos) = (angle + f64::from(k) * std::f64::consts::TAU / n).sin_cos();
            [center[0] + radius * cos, center[1] + radius * sin]
        })
        .collect()
}

/// The fewest and most sides a polygon may have.
pub const POLYGON_SIDES: std::ops::RangeInclusive<u32> = 3..=1000;

impl Geometry {
    /// A polyline of straight segments (every bulge 0).
    pub fn straight_polyline(closed: bool, points: Vec<[f64; 2]>) -> Geometry {
        let bulges = vec![0.0; polyline_segment_count(points.len(), closed)];
        Geometry::Polyline {
            closed,
            points,
            bulges,
        }
    }
}

/// How many segments a polyline of `points` points has.
pub fn polyline_segment_count(points: usize, closed: bool) -> usize {
    if closed {
        points
    } else {
        points.saturating_sub(1)
    }
}

/// A polyline's segments: straight where the bulge is 0, otherwise the arc a DXF bulge of
/// that value describes (`Arc::from_bulge`). A missing bulge counts as straight.
fn polyline_segments(points: &[[f64; 2]], bulges: &[f64], closed: bool) -> Vec<PathSegment> {
    let n = points.len();
    (0..polyline_segment_count(n, closed))
        .map(|i| {
            let (a, b) = (points[i], points[(i + 1) % n]);
            let bulge = bulges.get(i).copied().unwrap_or(0.0);
            match Arc::from_bulge((a[0], a[1]), (b[0], b[1]), bulge) {
                Some(arc) => PathSegment::Arc(ArcSegment {
                    center: [arc.center.0, arc.center.1],
                    radius: arc.radius,
                    start_angle: arc.start_angle,
                    sweep: arc.sweep,
                }),
                None => PathSegment::Line(LineSegment { start: a, end: b }),
            }
        })
        .collect()
}

/// A rectangle of half sizes `hl`, `hw` with corners of radius `r`, counter-clockwise from
/// `r` right of its lower-left corner: each side's line (left out when the arcs meet), then
/// the quarter circle round the next corner. `place` turns and moves local points.
fn rounded_rectangle(
    hl: f64,
    hw: f64,
    r: f64,
    angle: f64,
    place: impl Fn(f64, f64) -> [f64; 2],
) -> Vec<PathSegment> {
    use std::f64::consts::FRAC_PI_2;
    // Per side: its line's start and end, then the centre of the arc that follows and the
    // arc's start angle (local).
    let (x, y) = (hl - r, hw - r);
    let sides = [
        ((-x, -hw), (x, -hw), (x, -y), -FRAC_PI_2),
        ((hl, -y), (hl, y), (x, y), 0.0),
        ((x, hw), (-x, hw), (-x, y), FRAC_PI_2),
        ((-hl, y), (-hl, -y), (-x, -y), 2.0 * FRAC_PI_2),
    ];
    let mut segments = Vec::with_capacity(8);
    for (start, end, centre, start_angle) in sides {
        if (end.0 - start.0).hypot(end.1 - start.1) > CHAIN_TOL_MM {
            segments.push(PathSegment::Line(LineSegment {
                start: place(start.0, start.1),
                end: place(end.0, end.1),
            }));
        }
        segments.push(PathSegment::Arc(ArcSegment {
            center: place(centre.0, centre.1),
            radius: r,
            start_angle: start_angle + angle,
            sweep: FRAC_PI_2,
        }));
    }
    segments
}

fn lines_through(points: &[[f64; 2]], closed: bool) -> Vec<PathSegment> {
    let n = points.len();
    let count = if closed { n } else { n.saturating_sub(1) };
    (0..count)
        .map(|i| {
            PathSegment::Line(LineSegment {
                start: points[i],
                end: points[(i + 1) % n],
            })
        })
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum ShapeKind {
    Path,
    Circle,
    Rectangle,
    Polyline,
    Polygon,
}

/// A shape as it appears in the file: every field a kind may have. Fields of other kinds are
/// refused, never ignored.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
struct RawShape {
    /// The shape's id, which operations select it by.
    id: u64,
    /// Its layer.
    layer: String,
    /// circle, rectangle, polyline: its name in the Browser; left out when it has none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    /// circle, rectangle, polygon, polyline: construction geometry, shown and snapped to but
    /// never cut; left out when false.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    construction: bool,
    /// `path` (imported from a DXF), or a shape drawn in Chipload: `circle`, `rectangle`,
    /// `polyline`.
    kind: ShapeKind,
    /// path: the handle of the DXF entity it came from, which keeps its id when the DXF is
    /// updated.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    dxf_handle: Option<Option<String>>,
    /// path: the DXF entity's type, e.g. LWPOLYLINE.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    dxf_type: Option<Option<String>>,
    /// path: the image it was traced from; absent for shapes from a DXF.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    image: Option<u64>,
    /// path: the DXF it was imported from (its id in `drawing.dxfs`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    dxf: Option<u64>,
    /// path: the text it is a letter of (its id in `drawing.texts`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    text: Option<u64>,
    /// path, polyline: whether it ends where it starts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    closed: Option<bool>,
    /// path: its lines and arcs, mm.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    segments: Option<Vec<PathSegment>>,
    /// circle, rectangle, polygon: the centre, mm.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    center: Option<[f64; 2]>,
    /// circle: the radius, mm; polygon: the radius to its corners, mm.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    radius: Option<f64>,
    /// polygon: how many sides (and corners) it has, 3 to 1000.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    sides: Option<u32>,
    /// rectangle: its size along its own X axis, mm.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    length: Option<f64>,
    /// rectangle: its size along its own Y axis, mm.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    width: Option<f64>,
    /// rectangle: its rotation about the centre; polygon: the angle of its first corner from
    /// +X; radians, counter-clockwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    angle: Option<f64>,
    /// rectangle: the radius of its corners, mm; 0 for sharp ones.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    corner_radius: Option<f64>,
    /// polyline: its points, mm.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    points: Option<Vec<[f64; 2]>>,
    /// polyline: one per segment, as in DXF: tan(sweep / 4); 0 straight, 1 a
    /// counter-clockwise half circle, -1 clockwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    bulges: Option<Vec<f64>>,
}

impl RawShape {
    fn empty(id: u64, layer: String, kind: ShapeKind) -> Self {
        RawShape {
            id,
            layer,
            name: None,
            construction: false,
            kind,
            dxf_handle: None,
            dxf_type: None,
            image: None,
            dxf: None,
            text: None,
            closed: None,
            segments: None,
            center: None,
            radius: None,
            sides: None,
            length: None,
            width: None,
            angle: None,
            corner_radius: None,
            points: None,
            bulges: None,
        }
    }

    /// Names of the fields that are set, for "not allowed for this kind" messages.
    fn present(&self) -> Vec<&'static str> {
        [
            ("name", self.name.is_some()),
            ("construction", self.construction),
            ("dxf_handle", self.dxf_handle.is_some()),
            ("dxf_type", self.dxf_type.is_some()),
            ("image", self.image.is_some()),
            ("dxf", self.dxf.is_some()),
            ("text", self.text.is_some()),
            ("closed", self.closed.is_some()),
            ("segments", self.segments.is_some()),
            ("center", self.center.is_some()),
            ("radius", self.radius.is_some()),
            ("sides", self.sides.is_some()),
            ("length", self.length.is_some()),
            ("width", self.width.is_some()),
            ("angle", self.angle.is_some()),
            ("corner_radius", self.corner_radius.is_some()),
            ("points", self.points.is_some()),
            ("bulges", self.bulges.is_some()),
        ]
        .into_iter()
        .filter_map(|(name, set)| set.then_some(name))
        .collect()
    }
}

impl From<Shape> for RawShape {
    fn from(shape: Shape) -> Self {
        let kind = match shape.geometry {
            Geometry::Path { .. } => ShapeKind::Path,
            Geometry::Circle { .. } => ShapeKind::Circle,
            Geometry::Rectangle { .. } => ShapeKind::Rectangle,
            Geometry::Polyline { .. } => ShapeKind::Polyline,
            Geometry::Polygon { .. } => ShapeKind::Polygon,
        };
        let mut raw = RawShape::empty(shape.id, shape.layer, kind);
        raw.name = shape.name;
        raw.construction = shape.construction;
        match shape.geometry {
            Geometry::Path {
                dxf_handle,
                dxf_type,
                closed,
                segments,
                image,
                dxf,
                text,
            } => {
                raw.dxf_handle = Some(dxf_handle);
                raw.dxf_type = Some(dxf_type);
                raw.image = image;
                raw.dxf = dxf;
                raw.text = text;
                raw.closed = Some(closed);
                raw.segments = Some(segments);
            }
            Geometry::Circle { center, radius } => {
                raw.center = Some(center);
                raw.radius = Some(radius);
            }
            Geometry::Rectangle {
                center,
                length,
                width,
                angle,
                corner_radius,
            } => {
                raw.center = Some(center);
                raw.length = Some(length);
                raw.width = Some(width);
                raw.angle = Some(angle);
                raw.corner_radius = Some(corner_radius);
            }
            Geometry::Polyline {
                closed,
                points,
                bulges,
            } => {
                raw.closed = Some(closed);
                raw.points = Some(points);
                raw.bulges = Some(bulges);
            }
            Geometry::Polygon {
                center,
                radius,
                sides,
                angle,
            } => {
                raw.center = Some(center);
                raw.radius = Some(radius);
                raw.sides = Some(sides);
                raw.angle = Some(angle);
            }
        }
        raw
    }
}

impl TryFrom<RawShape> for Shape {
    type Error = String;

    fn try_from(raw: RawShape) -> Result<Self, String> {
        let kind = raw.kind;
        let allowed: &[&str] = match kind {
            ShapeKind::Path => &[
                "dxf_handle",
                "dxf_type",
                "image",
                "dxf",
                "text",
                "closed",
                "segments",
            ],
            ShapeKind::Circle => &["name", "construction", "center", "radius"],
            ShapeKind::Rectangle => &[
                "name",
                "construction",
                "center",
                "length",
                "width",
                "angle",
                "corner_radius",
            ],
            ShapeKind::Polyline => &["name", "construction", "closed", "points", "bulges"],
            ShapeKind::Polygon => &["name", "construction", "center", "radius", "sides", "angle"],
        };
        let name = RawShape::from_kind_name(kind);
        if let Some(extra) = raw.present().into_iter().find(|f| !allowed.contains(f)) {
            return Err(format!(
                "shape {}: field `{extra}` is not allowed for a {name}",
                raw.id
            ));
        }
        let missing = |field: &str| format!("shape {}: a {name} needs `{field}`", raw.id);
        let geometry = match kind {
            ShapeKind::Path => Geometry::Path {
                dxf_handle: raw.dxf_handle.flatten(),
                dxf_type: raw.dxf_type.flatten(),
                closed: raw.closed.ok_or_else(|| missing("closed"))?,
                segments: raw.segments.ok_or_else(|| missing("segments"))?,
                image: raw.image,
                dxf: raw.dxf,
                text: raw.text,
            },
            ShapeKind::Circle => Geometry::Circle {
                center: raw.center.ok_or_else(|| missing("center"))?,
                radius: raw.radius.ok_or_else(|| missing("radius"))?,
            },
            ShapeKind::Rectangle => Geometry::Rectangle {
                center: raw.center.ok_or_else(|| missing("center"))?,
                length: raw.length.ok_or_else(|| missing("length"))?,
                width: raw.width.ok_or_else(|| missing("width"))?,
                angle: raw.angle.ok_or_else(|| missing("angle"))?,
                corner_radius: raw.corner_radius.ok_or_else(|| missing("corner_radius"))?,
            },
            ShapeKind::Polyline => {
                let closed = raw.closed.ok_or_else(|| missing("closed"))?;
                let points = raw.points.ok_or_else(|| missing("points"))?;
                // Optional: without it every segment is straight (as before bulges existed).
                let count = polyline_segment_count(points.len(), closed);
                let bulges = raw.bulges.unwrap_or_else(|| vec![0.0; count]);
                Geometry::Polyline {
                    closed,
                    points,
                    bulges,
                }
            }
            ShapeKind::Polygon => Geometry::Polygon {
                center: raw.center.ok_or_else(|| missing("center"))?,
                radius: raw.radius.ok_or_else(|| missing("radius"))?,
                sides: raw.sides.ok_or_else(|| missing("sides"))?,
                angle: raw.angle.ok_or_else(|| missing("angle"))?,
            },
        };
        Ok(Shape {
            id: raw.id,
            layer: raw.layer,
            name: raw.name,
            geometry,
            construction: raw.construction,
        })
    }
}

impl RawShape {
    fn from_kind_name(kind: ShapeKind) -> &'static str {
        match kind {
            ShapeKind::Path => "path",
            ShapeKind::Circle => "circle",
            ShapeKind::Rectangle => "rectangle",
            ShapeKind::Polyline => "polyline",
            ShapeKind::Polygon => "polygon",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum PathSegment {
    Line(LineSegment),
    Arc(ArcSegment),
}

impl PathSegment {
    /// The primitive the stored segment describes (exact: the same f64 values).
    pub fn to_primitive(&self) -> Primitive {
        match self {
            PathSegment::Line(l) => Primitive::Line(Line {
                start: (l.start[0], l.start[1]),
                end: (l.end[0], l.end[1]),
            }),
            PathSegment::Arc(a) => Primitive::Arc(Arc {
                center: (a.center[0], a.center[1]),
                radius: a.radius,
                start_angle: a.start_angle,
                sweep: a.sweep,
            }),
        }
    }

    /// A primitive as a stored segment (exact: the same f64 values).
    pub fn from_primitive(prim: &Primitive) -> PathSegment {
        match prim {
            Primitive::Line(l) => PathSegment::Line(LineSegment {
                start: [l.start.0, l.start.1],
                end: [l.end.0, l.end.1],
            }),
            Primitive::Arc(a) => PathSegment::Arc(ArcSegment {
                center: [a.center.0, a.center.1],
                radius: a.radius,
                start_angle: a.start_angle,
                sweep: a.sweep,
            }),
        }
    }

    /// Whether the last of `segments` ends where the first starts (within the chaining
    /// tolerance).
    pub fn closes(segments: &[PathSegment]) -> bool {
        match (segments.first(), segments.last()) {
            (Some(first), Some(last)) => {
                distance(first.to_primitive().start(), last.to_primitive().end()) <= CHAIN_TOL_MM
            }
            _ => false,
        }
    }

    /// The segment moved by (dx, dy) mm.
    pub fn translated(&self, dx: f64, dy: f64) -> PathSegment {
        let moved = |p: [f64; 2]| [p[0] + dx, p[1] + dy];
        match self {
            PathSegment::Line(l) => PathSegment::Line(LineSegment {
                start: moved(l.start),
                end: moved(l.end),
            }),
            PathSegment::Arc(a) => PathSegment::Arc(ArcSegment {
                center: moved(a.center),
                ..*a
            }),
        }
    }

    /// The segment scaled about the origin by `factor` (> 0): points and radius are
    /// multiplied, angles stay.
    pub fn scaled(&self, factor: f64) -> PathSegment {
        let times = |p: [f64; 2]| [p[0] * factor, p[1] * factor];
        match self {
            PathSegment::Line(l) => PathSegment::Line(LineSegment {
                start: times(l.start),
                end: times(l.end),
            }),
            PathSegment::Arc(a) => PathSegment::Arc(ArcSegment {
                center: times(a.center),
                radius: a.radius * factor,
                ..*a
            }),
        }
    }

    /// The segment turned about the origin by `degrees` counter-clockwise; multiples of 90
    /// as [`PathSegment::quarter_turned`], exactly. An arc's start angle is wrapped into
    /// [0, 2π).
    pub fn rotated(&self, degrees: f64) -> PathSegment {
        if let Some(k) = as_quarter_turns(degrees) {
            return self.quarter_turned(k);
        }
        match self {
            PathSegment::Line(l) => PathSegment::Line(LineSegment {
                start: rotated(l.start, degrees),
                end: rotated(l.end, degrees),
            }),
            PathSegment::Arc(a) => PathSegment::Arc(ArcSegment {
                center: rotated(a.center, degrees),
                start_angle: (a.start_angle + degrees.to_radians())
                    .rem_euclid(std::f64::consts::TAU),
                ..*a
            }),
        }
    }

    /// The segment turned about the origin by `k` × 90° counter-clockwise. Points are
    /// exact; an arc's start angle is wrapped into [0, 2π).
    pub fn quarter_turned(&self, k: u8) -> PathSegment {
        if k.is_multiple_of(4) {
            return self.clone();
        }
        match self {
            PathSegment::Line(l) => PathSegment::Line(LineSegment {
                start: quarter_turned(l.start, k),
                end: quarter_turned(l.end, k),
            }),
            PathSegment::Arc(a) => PathSegment::Arc(ArcSegment {
                center: quarter_turned(a.center, k),
                start_angle: (a.start_angle + f64::from(k % 4) * std::f64::consts::FRAC_PI_2)
                    .rem_euclid(std::f64::consts::TAU),
                ..*a
            }),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct LineSegment {
    /// Start point, mm.
    pub start: [f64; 2],
    /// End point, mm.
    pub end: [f64; 2],
}

/// From `start_angle` through `sweep` radians (positive = counter-clockwise).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct ArcSegment {
    /// Centre, mm.
    pub center: [f64; 2],
    /// Radius, mm.
    pub radius: f64,
    /// Where the arc starts, radians from +X, counter-clockwise.
    pub start_angle: f64,
    /// How far it turns, radians: positive counter-clockwise.
    pub sweep: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum UnitsFrom {
    /// Declared by the file: a DXF's $INSUNITS, an SVG's size in real units.
    Dxf,
    /// The file declared none; the user chose (a DXF's units, an SVG's pixels per inch).
    User,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum IssueSeverity {
    Warning,
    Error,
}

/// An import-time issue that can't be recomputed from the shapes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct SourceIssue {
    /// `warning` or `error`.
    pub severity: IssueSeverity,
    /// What kind of issue, e.g. `duplicate` or `open_contour` (see the import page of the
    /// guide).
    pub code: String,
    /// What it is, in words.
    pub message: String,
}

/// Provenance of the imported shapes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct DrawingSource {
    /// The DXF's id, which its shapes and selectors name.
    pub id: u64,
    /// What kind of file it is: a DXF or an SVG.
    // Spec: docs/specs/svg-import.md §2 (schema 31).
    pub format: DrawingFormat,
    /// Its name in the Browser and messages; the file's name without its extension
    /// at first.
    pub name: String,
    /// The DXF's file name.
    pub file: String,
    /// The DXF's SHA-256, to tell whether it changed.
    pub sha256: String,
    /// The DXF's units: `mm` or `inch`. Inch drawings are converted to mm on import.
    pub units: Units,
    /// Where the units came from: the DXF itself, or the user's choice.
    pub units_from: UnitsFrom,
    /// How many entities of each type were imported.
    #[serde(default)]
    pub imported: BTreeMap<String, usize>,
    /// How many curves of each type (splines, ellipses) were turned into arcs.
    #[serde(default)]
    pub flattened: BTreeMap<String, usize>,
    /// How many entities of each type were left out (text, dimensions, …).
    #[serde(default)]
    pub ignored: BTreeMap<String, usize>,
    /// What the import found: the import report.
    #[serde(default)]
    pub issues: Vec<SourceIssue>,
    /// An SVG without a real size: the pixels per inch chosen for it, used again on update.
    /// None for a DXF and an SVG with real units.
    // Spec: docs/specs/svg-import.md §7.
    #[serde(default)]
    pub px_per_inch: Option<f64>,
    /// How the imported drawing was moved, turned and scaled on the stock.
    // Spec: docs/specs/drawing-placement.md.
    pub placement: Placement,
}

/// A drawing file's format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum DrawingFormat {
    Dxf,
    Svg,
}

impl DrawingFormat {
    /// Its name in messages: `DXF` or `SVG`, the same in every language.
    pub fn name(self) -> &'static str {
        match self {
            DrawingFormat::Dxf => "DXF",
            DrawingFormat::Svg => "SVG",
        }
    }
}

/// Where the imported drawing is put: a DXF point p goes to scale · rotate(p) + (dx, dy),
/// turned and scaled about the DXF's origin.
// Spec: docs/specs/drawing-rotation.md; docs/specs/drawing-scale.md;
// docs/specs/direct-placement.md (any angle).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Placement {
    /// Scaled about the DXF's origin by this factor, 0.001 to 1000; 1: as drawn.
    pub scale: f64,
    /// Turned counter-clockwise about the DXF's origin, degrees, at least 0 and less than
    /// 360, after the scale. Multiples of 90 are exact.
    pub rotation: f64,
    /// Then moved along X, mm.
    pub dx: f64,
    /// Then moved along Y, mm.
    pub dy: f64,
}

impl Default for Placement {
    /// As the DXF has it.
    fn default() -> Self {
        Placement {
            scale: 1.0,
            rotation: 0.0,
            dx: 0.0,
            dy: 0.0,
        }
    }
}

impl Placement {
    /// The smallest total scale of the imported drawing (docs/specs/drawing-scale.md §8.5).
    pub const MIN_SCALE: f64 = 0.001;
    /// The largest total scale.
    pub const MAX_SCALE: f64 = 1000.0;

    /// Whether `scale` is an allowed total scale.
    pub fn scale_in_range(scale: f64) -> bool {
        (Self::MIN_SCALE..=Self::MAX_SCALE).contains(&scale)
    }
}

/// The point turned about the origin by `k` × 90° counter-clockwise. Exact: no trigonometry.
/// A rotation in degrees as the file keeps it: at least 0, less than 360.
pub fn rotation_in_range(degrees: f64) -> bool {
    (0.0..360.0).contains(&degrees)
}

/// `degrees` as whole quarter turns (0–3), when it is an exact multiple of 90.
pub fn as_quarter_turns(degrees: f64) -> Option<u8> {
    if degrees % 90.0 == 0.0 && degrees.is_finite() {
        // A whole number of quarter turns, reduced to 0–3: the cast can't lose anything.
        Some((degrees / 90.0).rem_euclid(4.0) as u8)
    } else {
        None
    }
}

/// `p` turned about the origin by `degrees` counter-clockwise. Multiples of 90 are exact
/// ([`quarter_turned`]), so quarter-turned drawings stay bit for bit what they were.
pub fn rotated(p: [f64; 2], degrees: f64) -> [f64; 2] {
    if let Some(k) = as_quarter_turns(degrees) {
        return quarter_turned(p, k);
    }
    let (sin, cos) = degrees.to_radians().sin_cos();
    [p[0] * cos - p[1] * sin, p[0] * sin + p[1] * cos]
}

pub fn quarter_turned(p: [f64; 2], k: u8) -> [f64; 2] {
    match k % 4 {
        0 => p,
        1 => [-p[1], p[0]],
        2 => [-p[0], -p[1]],
        _ => [p[1], -p[0]],
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum StockOrigin {
    #[default]
    BottomLeft,
    Center,
    /// Centred on the drawing's outline, sticking out equally all round, and following the
    /// drawing when it moves or changes (the user, 2026-10-28). Construction geometry
    /// doesn't count; an empty drawing centres it on the origin.
    // Spec: docs/specs/stock-around-drawing.md (schema 33).
    Drawing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum ZZero {
    #[default]
    Top,
    Bottom,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Stock {
    /// Size along X, mm.
    pub x: f64,
    /// Size along Y, mm.
    pub y: f64,
    /// Thickness, mm. Depths count down from the top.
    pub thickness: f64,
    /// Where the stock lies: its `bottom_left` corner or its `center` at the drawing's
    /// origin, or centred on the drawing itself (`drawing`), following it.
    #[serde(default)]
    pub origin: StockOrigin,
    /// Centred on the drawing only: its size is the drawing's outline plus this margin each
    /// side, mm, following the drawing; `x` and `y` are then used only while the drawing is
    /// empty. Null: the size is `x` × `y`.
    // Spec: docs/specs/stock-around-drawing.md (schema 33).
    #[serde(default)]
    pub fit: Option<f64>,
    /// Where Z0 is: the stock `top` or `bottom`.
    #[serde(default)]
    pub z_zero: ZZero,
    /// How far a cut may go below the stock bottom, into the spoilboard, mm.
    pub max_overcut: f64,
    /// The project's material: new operations take it, with its values. Each operation
    /// names its own (`material` on the operation). Null: none.
    // Spec: docs/specs/materials.md §6 (optional, no schema bump).
    #[serde(default)]
    pub material: Option<MaterialRef>,
}

/// A library material, as a project names it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct MaterialRef {
    /// Its name when chosen, shown when the library no longer has it.
    pub name: String,
    /// The library material's uid.
    pub library_uid: String,
}

impl Stock {
    /// The Z of the stock top: 0 with Z zero at the top, the thickness with it at the
    /// bottom.
    pub fn top_z(&self) -> f64 {
        match self.z_zero {
            ZZero::Top => 0.0,
            ZZero::Bottom => self.thickness,
        }
    }

    /// (min, max) corners in drawing coordinates, for a stock of `size` that sits around
    /// `centre` when it is centred on the drawing. Use [`Job::stock_bounds`], which knows
    /// the drawing.
    pub fn xy_bounds_around(
        &self,
        size: (f64, f64),
        centre: (f64, f64),
    ) -> ((f64, f64), (f64, f64)) {
        let (hx, hy) = (size.0 / 2.0, size.1 / 2.0);
        match self.origin {
            StockOrigin::BottomLeft => ((0.0, 0.0), size),
            StockOrigin::Center => ((-hx, -hy), (hx, hy)),
            StockOrigin::Drawing => (
                (centre.0 - hx, centre.1 - hy),
                (centre.0 + hx, centre.1 + hy),
            ),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Travel {
    /// Travel along X, mm.
    pub x: f64,
    /// Travel along Y, mm.
    pub y: f64,
    /// Travel along Z, mm.
    pub z: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum Spindle {
    /// M3
    Cw,
    /// M4
    Ccw,
}

/// How a job with several tools is written.
// Spec: docs/specs/tool-per-operation.md §3.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum ToolChange {
    /// One G-code file per tool group; the user changes the tool and sets Z zero between.
    #[default]
    Files,
    /// One file with `T<n> M6` at each change, for a sender that handles it.
    Program,
}

impl ToolChange {
    pub fn name(self) -> &'static str {
        match self {
            ToolChange::Files => "files",
            ToolChange::Program => "program",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Machine {
    /// The machine profile these values came from, for reference; nothing reads it.
    // Spec: docs/specs/tool-library-and-machines.md §4.
    #[serde(default)]
    pub name: Option<String>,
    /// The `uid` of the machine profile these values came from, for reference; nothing
    /// that cuts reads it. Null: not from a profile.
    // Spec: docs/specs/library-in-browser.md §2.
    #[serde(default)]
    pub library_uid: Option<String>,
    /// The post-processor: `grbl`, `linuxcnc`, `mach3`, or the name of your own.
    pub post: String,
    /// The units the G-code is written in: `mm` (G21) or `inch` (G20). Everything else in
    /// the project stays in mm. Since schema 28.
    // Spec: docs/specs/inch-units.md §4.
    pub gcode_units: Units,
    /// The height for rapid moves sideways, mm above Z0. Above the stock top.
    pub safe_z: f64,
    /// The spindle's direction: `cw` (M3) or `ccw` (M4).
    pub spindle: Spindle,
    /// The machine's fastest feed, mm/min: higher feeds are refused, and the cycle time
    /// uses it for rapids. Null: unknown.
    #[serde(default)]
    pub max_feed: Option<f64>,
    /// The machine's travel, mm. When set, a program that moves farther is refused. Null:
    /// not checked.
    #[serde(default)]
    pub travel: Option<Travel>,
    /// The point of the drawing that becomes X0 Y0 in the G-code. Missing: the drawing's
    /// origin.
    // Spec: docs/specs/work-zero.md.
    #[serde(default)]
    pub work_zero: WorkZero,
    /// With several tools: `files` (one file per tool) or `program` (one file with tool
    /// changes).
    pub tool_change: ToolChange,
    /// Your own post-processor, copied into the project when chosen, so it gives the same
    /// G-code anywhere. Null: `post` is a built-in one.
    // Spec: docs/specs/user-posts.md §3.
    #[serde(default)]
    pub custom_post: Option<CustomPost>,
}

/// A user post-processor's config and template, exactly as read from the posts folder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct CustomPost {
    /// The post's settings file (TOML), as copied.
    pub config: String,
    /// The post's template, as copied.
    pub template: String,
}

/// Where the G-code's X0 Y0 is. The geometry never moves; only the output is translated.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "at", rename_all = "snake_case", deny_unknown_fields)]
pub enum WorkZero {
    /// The drawing's own origin (the only choice before work_zero existed). An empty struct
    /// variant, not a unit one, so serde refuses stray fields here too.
    DrawingOrigin {},
    /// A corner or the centre of the stock, as placed by `stock.origin`.
    Stock {
        /// `center`, `bottom_left`, `bottom_right`, `top_left` or `top_right` of the stock.
        corner: StockCorner,
    },
    /// Any point, in drawing coordinates (mm).
    Point {
        /// X of the point, drawing coordinates, mm.
        x: f64,
        /// Y of the point, drawing coordinates, mm.
        y: f64,
    },
}

impl Default for WorkZero {
    fn default() -> Self {
        WorkZero::DrawingOrigin {}
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum StockCorner {
    Center,
    BottomLeft,
    BottomRight,
    TopLeft,
    TopRight,
}

impl StockCorner {
    /// Every choice: the four corners, then the centre.
    pub const ALL: [StockCorner; 5] = [
        StockCorner::BottomLeft,
        StockCorner::BottomRight,
        StockCorner::TopLeft,
        StockCorner::TopRight,
        StockCorner::Center,
    ];

    pub fn name(self) -> &'static str {
        match self {
            StockCorner::Center => "center",
            StockCorner::BottomLeft => "bottom left corner",
            StockCorner::BottomRight => "bottom right corner",
            StockCorner::TopLeft => "top left corner",
            StockCorner::TopRight => "top right corner",
        }
    }
}

/// Why a pocket cut with `small` can't rest machine after `large`
/// (docs/specs/rest-machining.md §5): both must be endmills, and `large` larger by more
/// than the rest tolerance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestRefusal {
    /// The larger tool is not an endmill, which a pocket's tool is.
    LargeNotEndmill,
    /// The rest pocket's own tool is not an endmill.
    SmallNotEndmill,
    /// The larger tool is not larger.
    NotLarger,
}

/// The reason `small` can't rest machine after `large`, if any.
pub fn rest_refusal(small: &Tool, large: &Tool) -> Option<RestRefusal> {
    if large.kind != ToolType::Endmill {
        return Some(RestRefusal::LargeNotEndmill);
    }
    if small.kind != ToolType::Endmill {
        return Some(RestRefusal::SmallNotEndmill);
    }
    if large.diameter <= small.diameter + REST_TOL_MM {
        return Some(RestRefusal::NotLarger);
    }
    None
}

/// The tool with this id among `tools`.
pub fn find_tool(tools: &[Tool], id: i64) -> Option<&Tool> {
    tools.iter().find(|t| t.id == id)
}

impl Job {
    /// Every operation cut through the stock ends its thickness plus `through` down
    /// (docs/specs/job-settings-layout.md §5): run wherever a project is read or changed, so
    /// no end depth is left from an older thickness.
    pub fn follow_thickness(&mut self) {
        let thickness = self.stock.thickness;
        for op in &mut self.operations {
            op.follow_thickness(thickness);
        }
    }

    /// The part with this id.
    pub fn part(&self, id: u64) -> Option<&Part> {
        self.parts.iter().find(|p| p.id == id)
    }

    /// The project tool with this id.
    pub fn tool(&self, id: i64) -> Option<&Tool> {
        find_tool(&self.tools, id)
    }

    /// The part operation `op` belongs to.
    pub fn part_of(&self, op: &Operation) -> Option<&Part> {
        self.part(op.part())
    }

    /// Whether operation `index` is cut: it is on and so is its part (docs/specs/parts.md
    /// §3).
    pub fn cuts(&self, index: usize) -> bool {
        self.operations
            .get(index)
            .is_some_and(|op| op.enabled() && self.part_of(op).is_some_and(|p| p.enabled))
    }

    /// The id of the part new operations go into: `active_part` when it names a part, else
    /// the first part.
    pub fn active_part_id(&self) -> Option<u64> {
        self.active_part
            .filter(|id| self.part(*id).is_some())
            .or_else(|| self.parts.first().map(|p| p.id))
    }

    /// Where an operation added to part `id` goes: after the part's last operation, or,
    /// for a part without operations, after the last operation of the parts before it.
    pub fn insert_index(&self, id: u64) -> usize {
        let order: Vec<u64> = self.parts.iter().map(|p| p.id).collect();
        let rank = |part: u64| order.iter().position(|p| *p == part).unwrap_or(usize::MAX);
        let mine = rank(id);
        self.operations
            .iter()
            .rposition(|op| rank(op.part()) <= mine)
            .map_or(0, |i| i + 1)
    }

    /// The stock's (min, max) corners in drawing coordinates, wherever it is placed: at the
    /// drawing origin, or centred on the drawing.
    pub fn stock_bounds(&self) -> ((f64, f64), (f64, f64)) {
        let centre = self
            .drawing
            .extents()
            .map_or((0.0, 0.0), |((x0, y0), (x1, y1))| {
                ((x0 + x1) / 2.0, (y0 + y1) / 2.0)
            });
        self.stock.xy_bounds_around(self.stock_size(), centre)
    }

    /// The stock's size, mm: the drawing's outline plus the margin each side when it fits
    /// the drawing, otherwise as set.
    pub fn stock_size(&self) -> (f64, f64) {
        match (self.stock.origin, self.stock.fit, self.drawing.extents()) {
            (StockOrigin::Drawing, Some(margin), Some(((x0, y0), (x1, y1)))) => {
                (x1 - x0 + 2.0 * margin, y1 - y0 + 2.0 * margin)
            }
            _ => (self.stock.x, self.stock.y),
        }
    }

    /// The work zero in drawing coordinates: subtract it from every X, Y of the toolpath to
    /// get program coordinates.
    pub fn work_zero_xy(&self) -> (f64, f64) {
        self.work_zero_point(self.machine.work_zero)
    }

    /// Where work zero `zero` would be, in drawing coordinates.
    pub fn work_zero_point(&self, zero: WorkZero) -> (f64, f64) {
        match zero {
            WorkZero::DrawingOrigin {} => (0.0, 0.0),
            WorkZero::Point { x, y } => (x, y),
            WorkZero::Stock { corner } => self.stock_corner_xy(corner),
        }
    }

    /// A corner or the centre of the stock, in drawing coordinates.
    pub fn stock_corner_xy(&self, corner: StockCorner) -> (f64, f64) {
        let ((x0, y0), (x1, y1)) = self.stock_bounds();
        match corner {
            StockCorner::Center => ((x0 + x1) / 2.0, (y0 + y1) / 2.0),
            StockCorner::BottomLeft => (x0, y0),
            StockCorner::BottomRight => (x1, y0),
            StockCorner::TopLeft => (x0, y1),
            StockCorner::TopRight => (x1, y1),
        }
    }

    /// The work zero a point dropped at `at` means (docs/specs/work-zero-marker.md §3):
    /// within `reach` mm of a stock corner or its centre, that corner, which follows the
    /// stock; of the drawing origin, the origin; elsewhere the point itself. The nearest
    /// wins.
    pub fn work_zero_at(&self, at: (f64, f64), reach: f64) -> WorkZero {
        let mut named: Vec<WorkZero> = StockCorner::ALL
            .into_iter()
            .map(|corner| WorkZero::Stock { corner })
            .collect();
        named.push(WorkZero::DrawingOrigin {});
        let distance = |zero: WorkZero| {
            let (x, y) = self.work_zero_point(zero);
            (x - at.0).hypot(y - at.1)
        };
        named
            .into_iter()
            .map(|zero| (zero, distance(zero)))
            .filter(|&(_, d)| d <= reach)
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map_or(WorkZero::Point { x: at.0, y: at.1 }, |(zero, _)| zero)
    }

    /// "stock center = drawing X150.000 Y110.000, Z0 = stock top", for the G-code header,
    /// the CLI and the GUI. No parentheses or semicolons, so it fits in a G-code comment.
    pub fn work_zero_text(&self) -> String {
        let (x, y) = self.work_zero_xy();
        let at = match self.machine.work_zero {
            WorkZero::DrawingOrigin {} => "drawing origin".to_string(),
            WorkZero::Stock { corner } => format!("stock {}", corner.name()),
            WorkZero::Point { .. } => "point".to_string(),
        };
        let z = match self.stock.z_zero {
            ZZero::Top => "stock top",
            ZZero::Bottom => "stock bottom",
        };
        format!("{at} = drawing X{x:.3} Y{y:.3}, Z0 = {z}")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum ToolType {
    #[default]
    Endmill,
    Drill,
    Vbit,
}

impl ToolType {
    /// The kind as messages and lists name it: `endmill`, `drill`, `V-bit`.
    pub fn name(self) -> &'static str {
        match self {
            ToolType::Endmill => "endmill",
            ToolType::Drill => "drill",
            ToolType::Vbit => "V-bit",
        }
    }

    /// The name with its article: `an endmill`, `a drill`, `a V-bit`.
    pub fn with_article(self) -> &'static str {
        match self {
            ToolType::Endmill => "an endmill",
            ToolType::Drill => "a drill",
            ToolType::Vbit => "a V-bit",
        }
    }

    /// [`ToolType::name`] as a message, for problems shown in the reader's language.
    pub fn name_message(self) -> Message {
        Message::new("job-tool-type").arg("type", self.key())
    }

    /// [`ToolType::with_article`] as a message.
    pub fn with_article_message(self) -> Message {
        Message::new("job-tool-type-article").arg("type", self.key())
    }

    /// The kind as the project file writes it, which the messages select on.
    fn key(self) -> &'static str {
        match self {
            ToolType::Endmill => "endmill",
            ToolType::Drill => "drill",
            ToolType::Vbit => "vbit",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Tool {
    /// The tool's number: what operations name it by, and its T number in the G-code.
    pub id: i64,
    /// The library tool it came from, for reference.
    // Spec: docs/specs/tool-library-and-machines.md §4.
    #[serde(default)]
    pub name: Option<String>,
    /// The `uid` of the library tool it came from, for reference; nothing that cuts reads
    /// it. Null: not from the library.
    // Spec: docs/specs/library-in-browser.md §2.
    #[serde(default)]
    pub library_uid: Option<String>,
    /// `endmill`, `drill` or `vbit`.
    #[serde(default, rename = "type")]
    pub kind: ToolType,
    /// Diameter, mm. For a V-bit, its widest cutting diameter.
    pub diameter: f64,
    /// Number of flutes, for the chip load. Null: unknown.
    #[serde(default)]
    pub flutes: Option<i64>,
    /// Whether it can plunge straight down. False: profiles ramp in and pockets go down
    /// round a helix. Null: unknown.
    #[serde(default)]
    pub center_cutting: Option<bool>,
    /// Spindle speed, rpm. Needed to generate.
    #[serde(default)]
    pub rpm: Option<f64>,
    /// V-bits: the included angle, radians. Null: unknown; a V-carve then refuses the tool.
    // Spec: docs/specs/v-carve.md §2.
    #[serde(default)]
    pub angle: Option<f64>,
    /// V-bits: the flat tip's diameter, mm. Null: pointed.
    #[serde(default)]
    pub tip_diameter: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Selector {
    /// Every shape on this layer (upper or lower case don't matter).
    #[serde(default)]
    pub layer: Option<String>,
    /// These shapes, by id.
    #[serde(default)]
    pub shapes: Option<Vec<u64>>,
    /// Only shapes from this DXF (an id in `drawing.dxfs`).
    // Spec: docs/specs/drawing-sources.md §2.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dxf: Option<u64>,
    /// Only shapes traced from this image (an id in `drawing.images`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image: Option<u64>,
    /// Only the letters of this text (an id in `drawing.texts`).
    // Spec: docs/specs/text.md §3.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<u64>,
    /// Shapes of that DXF, image or text left out (ids); with `dxf`, `image` or `text`
    /// only. A contour with any of them isn't cut. A traced image's, or a text's, are found
    /// again when it is made again.
    // Spec: docs/specs/leave-out-shapes.md.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exclude: Option<Vec<u64>>,
}

impl Selector {
    /// Whether it picks nothing at all: no DXF, image, layer or shapes. A selector that
    /// names only a DXF or an image picks all of it (docs/specs/drawing-sources.md §2,
    /// leave-out-shapes.md §2).
    pub fn is_empty(&self) -> bool {
        self.layer.is_none()
            && self.dxf.is_none()
            && self.image.is_none()
            && self.text.is_none()
            && self.shapes.as_ref().is_none_or(Vec::is_empty)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Tabs {
    /// Tabs per contour, at least 1.
    pub count: i64,
    /// The bridge left on the part, along its edge, mm (the flat top of a sloped tab).
    pub width: f64,
    /// The bridge's thickness, up from the bottom of the cut, mm.
    pub height: f64,
    /// The slope of the tab's sides, radians: π/2 (90°) straight, less for sloped tabs a
    /// ramped profile follows.
    // Spec: docs/specs/profile-spiral.md §4.
    pub flank_angle: f64,
    /// Tab centres placed by hand, in drawing coordinates (mm): a contour with points gets
    /// exactly those tabs, one without gets `count`, evenly spaced. Left out when empty.
    // Spec: docs/specs/tabs-by-hand.md.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub at: Vec<[f64; 2]>,
}

/// Straight tabs: up and down vertically.
pub const STRAIGHT_FLANK_RAD: f64 = std::f64::consts::FRAC_PI_2;
/// The flank angle new tabs get with a ramp entry and a tool that can plunge: 30°.
pub const DEFAULT_RAMPED_FLANK_RAD: f64 = std::f64::consts::PI / 6.0;

/// A flank angle of 90° (within the angle tolerance): a straight tab.
pub fn flank_is_straight(flank_angle: f64) -> bool {
    flank_angle >= STRAIGHT_FLANK_RAD - ANGLE_TOL_RAD
}

impl Tabs {
    /// Straight up and down (90°, within the angle tolerance).
    pub fn is_straight(&self) -> bool {
        flank_is_straight(self.flank_angle)
    }

    /// The flank angle new tabs get (docs/specs/profile-spiral.md §4): 30°, or the
    /// profile's ramp angle when the tool is marked not centre-cutting (a flank's way down
    /// is a ramp).
    pub fn default_flank(ramp_angle: f64, center_cutting: Option<bool>) -> f64 {
        if center_cutting == Some(false) {
            ramp_angle
        } else {
            DEFAULT_RAMPED_FLANK_RAD
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum Side {
    Inside,
    Outside,
    On,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    Climb,
    Conventional,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Operation {
    Profile(ProfileOp),
    Pocket(PocketOp),
    Vcarve(VcarveOp),
    /// Follow path: the tool centre on the selected shapes, open or closed.
    // Spec: docs/specs/follow-path.md.
    Path(PathOp),
    /// Holes: drilled with a drill, bored with a helix with an endmill.
    // Spec: docs/specs/holes.md.
    Holes(HolesOp),
    /// Surface: the whole stock top taken down flat, in straight passes back and forth.
    // Spec: docs/specs/surfacing.md.
    Surface(SurfaceOp),
}

/// The selector of an operation that selects nothing (a surface faces the whole stock).
static NO_SELECTOR: Selector = Selector {
    layer: None,
    shapes: None,
    dxf: None,
    image: None,
    text: None,
    exclude: None,
};

fn enabled_default() -> bool {
    true
}

/// "Operation 3" or "Operation 3 (lid)": how messages name the operation at `index`.
pub fn operation_label(index: usize, name: &str) -> String {
    match name {
        "" => format!("Operation {}", index + 1),
        name => format!("Operation {} ({name})", index + 1),
    }
}

/// [`operation_label`] as a message, for problems shown in the reader's language.
pub fn operation_label_message(index: usize, name: &str) -> Message {
    match name {
        "" => Message::new("job-operation-label").arg("number", index + 1),
        name => Message::new("job-operation-label-named")
            .arg("number", index + 1)
            .arg("name", name),
    }
}

impl Operation {
    /// Whether the operation already cuts with these feeds, step down and (pockets)
    /// stepover. A hole has only feeds; a surface keeps its own wide stepover.
    pub fn has_cutting(&self, c: &crate::library::Cutting) -> bool {
        match self {
            Operation::Profile(p) => {
                (p.feed, p.plunge_feed, p.step_down) == (c.feed, c.plunge_feed, c.step_down)
            }
            Operation::Pocket(p) => {
                (p.feed, p.plunge_feed, p.step_down, p.stepover)
                    == (c.feed, c.plunge_feed, c.step_down, c.stepover)
            }
            Operation::Vcarve(v) => {
                (v.feed, v.plunge_feed, v.step_down) == (c.feed, c.plunge_feed, Some(c.step_down))
            }
            Operation::Path(p) => {
                (p.feed, p.plunge_feed, p.step_down) == (c.feed, c.plunge_feed, c.step_down)
            }
            Operation::Holes(h) => (h.feed, h.plunge_feed) == (c.feed, c.plunge_feed),
            Operation::Surface(f) => {
                (f.feed, f.plunge_feed, f.step_down) == (c.feed, c.plunge_feed, c.step_down)
            }
        }
    }

    /// Cut with these values: the ones the operation has (as `has_cutting`).
    pub fn apply_cutting(&mut self, c: &crate::library::Cutting) {
        match self {
            Operation::Profile(p) => {
                (p.feed, p.plunge_feed, p.step_down) = (c.feed, c.plunge_feed, c.step_down);
            }
            Operation::Pocket(p) => {
                (p.feed, p.plunge_feed, p.step_down, p.stepover) =
                    (c.feed, c.plunge_feed, c.step_down, c.stepover);
            }
            Operation::Vcarve(v) => {
                (v.feed, v.plunge_feed, v.step_down) = (c.feed, c.plunge_feed, Some(c.step_down));
            }
            Operation::Path(p) => {
                (p.feed, p.plunge_feed, p.step_down) = (c.feed, c.plunge_feed, c.step_down);
            }
            Operation::Holes(h) => (h.feed, h.plunge_feed) = (c.feed, c.plunge_feed),
            Operation::Surface(f) => {
                (f.feed, f.plunge_feed, f.step_down) = (c.feed, c.plunge_feed, c.step_down);
            }
        }
    }

    /// `base` with the operation's own values in place of its (as `has_cutting`): what
    /// saving the operation's values keeps.
    pub fn cutting_over(&self, base: crate::library::Cutting) -> crate::library::Cutting {
        let mut c = base;
        match self {
            Operation::Profile(p) => {
                (c.feed, c.plunge_feed, c.step_down) = (p.feed, p.plunge_feed, p.step_down);
            }
            Operation::Pocket(p) => {
                (c.feed, c.plunge_feed, c.step_down, c.stepover) =
                    (p.feed, p.plunge_feed, p.step_down, p.stepover);
            }
            Operation::Vcarve(v) => {
                (c.feed, c.plunge_feed) = (v.feed, v.plunge_feed);
                if let Some(step_down) = v.step_down {
                    c.step_down = step_down;
                }
            }
            Operation::Path(p) => {
                (c.feed, c.plunge_feed, c.step_down) = (p.feed, p.plunge_feed, p.step_down);
            }
            Operation::Holes(h) => (c.feed, c.plunge_feed) = (h.feed, h.plunge_feed),
            Operation::Surface(f) => {
                (c.feed, c.plunge_feed, c.step_down) = (f.feed, f.plunge_feed, f.step_down);
            }
        }
        c
    }

    /// "Operation 3" or "Operation 3 (lid)", for the operation at `index`.
    pub fn label(&self, index: usize) -> String {
        operation_label(index, self.name())
    }

    /// [`Operation::label`] as a message.
    pub fn label_message(&self, index: usize) -> Message {
        operation_label_message(index, self.name())
    }

    pub fn name(&self) -> &str {
        match self {
            Operation::Profile(op) => &op.name,
            Operation::Pocket(op) => &op.name,
            Operation::Vcarve(op) => &op.name,
            Operation::Path(op) => &op.name,
            Operation::Holes(op) => &op.name,
            Operation::Surface(op) => &op.name,
        }
    }

    pub fn name_mut(&mut self) -> &mut String {
        match self {
            Operation::Profile(op) => &mut op.name,
            Operation::Pocket(op) => &mut op.name,
            Operation::Vcarve(op) => &mut op.name,
            Operation::Path(op) => &mut op.name,
            Operation::Holes(op) => &mut op.name,
            Operation::Surface(op) => &mut op.name,
        }
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        match self {
            Operation::Profile(op) => op.enabled = enabled,
            Operation::Pocket(op) => op.enabled = enabled,
            Operation::Vcarve(op) => op.enabled = enabled,
            Operation::Path(op) => op.enabled = enabled,
            Operation::Holes(op) => op.enabled = enabled,
            Operation::Surface(op) => op.enabled = enabled,
        }
    }

    pub fn enabled(&self) -> bool {
        match self {
            Operation::Profile(op) => op.enabled,
            Operation::Pocket(op) => op.enabled,
            Operation::Vcarve(op) => op.enabled,
            Operation::Path(op) => op.enabled,
            Operation::Holes(op) => op.enabled,
            Operation::Surface(op) => op.enabled,
        }
    }

    /// The id of its part.
    pub fn part(&self) -> u64 {
        match self {
            Operation::Profile(op) => op.part,
            Operation::Pocket(op) => op.part,
            Operation::Vcarve(op) => op.part,
            Operation::Path(op) => op.part,
            Operation::Holes(op) => op.part,
            Operation::Surface(op) => op.part,
        }
    }

    pub fn set_part(&mut self, part: u64) {
        match self {
            Operation::Profile(op) => op.part = part,
            Operation::Pocket(op) => op.part = part,
            Operation::Vcarve(op) => op.part = part,
            Operation::Path(op) => op.part = part,
            Operation::Holes(op) => op.part = part,
            Operation::Surface(op) => op.part = part,
        }
    }

    /// The library material its feeds come from.
    pub fn material(&self) -> Option<&MaterialRef> {
        match self {
            Operation::Profile(op) => op.material.as_ref(),
            Operation::Pocket(op) => op.material.as_ref(),
            Operation::Vcarve(op) => op.material.as_ref(),
            Operation::Path(op) => op.material.as_ref(),
            Operation::Holes(op) => op.material.as_ref(),
            Operation::Surface(op) => op.material.as_ref(),
        }
    }

    pub fn set_material(&mut self, material: Option<MaterialRef>) {
        match self {
            Operation::Profile(op) => op.material = material,
            Operation::Pocket(op) => op.material = material,
            Operation::Vcarve(op) => op.material = material,
            Operation::Path(op) => op.material = material,
            Operation::Holes(op) => op.material = material,
            Operation::Surface(op) => op.material = material,
        }
    }

    pub fn tool(&self) -> i64 {
        match self {
            Operation::Profile(op) => op.tool,
            Operation::Pocket(op) => op.tool,
            Operation::Vcarve(op) => op.tool,
            Operation::Path(op) => op.tool,
            Operation::Holes(op) => op.tool,
            Operation::Surface(op) => op.tool,
        }
    }

    pub fn set_tool(&mut self, tool: i64) {
        match self {
            Operation::Profile(op) => op.tool = tool,
            Operation::Pocket(op) => op.tool = tool,
            Operation::Vcarve(op) => op.tool = tool,
            Operation::Path(op) => op.tool = tool,
            Operation::Holes(op) => op.tool = tool,
            Operation::Surface(op) => op.tool = tool,
        }
    }

    /// (start_depth, end_depth), mm down from the stock top.
    pub fn depths(&self) -> (f64, f64) {
        match self {
            Operation::Profile(op) => (op.start_depth, op.end_depth),
            Operation::Pocket(op) => (op.start_depth, op.end_depth),
            Operation::Vcarve(op) => (op.start_depth, op.end_depth),
            Operation::Path(op) => (op.start_depth, op.end_depth),
            Operation::Holes(op) => (op.start_depth, op.end_depth),
            Operation::Surface(op) => (op.start_depth, op.end_depth),
        }
    }

    /// How far below the stock an operation cut through it ends (`through`); None for one
    /// whose end depth is as set, and always for a V-carve and a surface.
    pub fn through(&self) -> Option<f64> {
        match self {
            Operation::Profile(op) => op.through,
            Operation::Pocket(op) => op.through,
            Operation::Path(op) => op.through,
            Operation::Holes(op) => op.through,
            Operation::Vcarve(_) | Operation::Surface(_) => None,
        }
    }

    /// None for an operation that never cuts through (a V-carve, a surface).
    pub fn through_mut(&mut self) -> Option<&mut Option<f64>> {
        match self {
            Operation::Profile(op) => Some(&mut op.through),
            Operation::Pocket(op) => Some(&mut op.through),
            Operation::Path(op) => Some(&mut op.through),
            Operation::Holes(op) => Some(&mut op.through),
            Operation::Vcarve(_) | Operation::Surface(_) => None,
        }
    }

    /// The end depth of an operation cut through stock `thickness` mm thick: the thickness
    /// plus `through`. Nothing changes for one whose end depth is as set.
    pub fn follow_thickness(&mut self, thickness: f64) {
        let end = match self {
            Operation::Profile(op) => op.through.map(|t| (&mut op.end_depth, t)),
            Operation::Pocket(op) => op.through.map(|t| (&mut op.end_depth, t)),
            Operation::Path(op) => op.through.map(|t| (&mut op.end_depth, t)),
            Operation::Holes(op) => op.through.map(|t| (&mut op.end_depth, t)),
            Operation::Vcarve(_) | Operation::Surface(_) => None,
        };
        if let Some((end_depth, below)) = end {
            *end_depth = thickness + below;
        }
    }

    pub fn entry(&self) -> Entry {
        match self {
            // A profile always spirals (docs/specs/profile-always-spiral.md).
            Operation::Profile(p) => Entry::Ramp {
                ramp_angle: p.ramp_angle,
            },
            Operation::Pocket(p) => p.entry,
            // Paths start at the ends of the axis, mostly at depth 0; others plunge.
            Operation::Vcarve(_) => Entry::Plunge {},
            Operation::Path(p) => p.entry,
            // Holes go down their own way (docs/specs/holes.md §3, §4).
            Operation::Holes(_) => Entry::Plunge {},
            // Each depth starts beside the stock, in the air (docs/specs/surfacing.md §3).
            Operation::Surface(_) => Entry::Plunge {},
        }
    }

    pub fn starts(&self) -> &[[f64; 2]] {
        match self {
            Operation::Profile(p) => &p.starts,
            Operation::Pocket(p) => &p.starts,
            Operation::Vcarve(_) => &[],
            Operation::Path(_) | Operation::Holes(_) | Operation::Surface(_) => &[],
        }
    }

    /// None for an operation without start points (a V-carve, a follow path, holes, a
    /// surface).
    pub fn starts_mut(&mut self) -> Option<&mut Vec<[f64; 2]>> {
        match self {
            Operation::Profile(p) => Some(&mut p.starts),
            Operation::Pocket(p) => Some(&mut p.starts),
            Operation::Vcarve(_)
            | Operation::Path(_)
            | Operation::Holes(_)
            | Operation::Surface(_) => None,
        }
    }

    /// Dogbones or T-bones (docs/specs/dogbones.md): profiles and pockets only.
    pub fn corners(&self) -> Option<Corners> {
        match self {
            Operation::Profile(p) => p.corners,
            Operation::Pocket(p) => p.corners,
            _ => None,
        }
    }

    /// Corners with their own bone (docs/specs/dogbone-per-corner.md): profiles and
    /// pockets only.
    pub fn corner_choices(&self) -> &[CornerChoice] {
        match self {
            Operation::Profile(p) => &p.corner_choices,
            Operation::Pocket(p) => &p.corner_choices,
            _ => &[],
        }
    }

    /// None for an operation without corners (anything but a profile or a pocket).
    pub fn corner_choices_mut(&mut self) -> Option<&mut Vec<CornerChoice>> {
        match self {
            Operation::Profile(p) => Some(&mut p.corner_choices),
            Operation::Pocket(p) => Some(&mut p.corner_choices),
            Operation::Vcarve(_)
            | Operation::Path(_)
            | Operation::Holes(_)
            | Operation::Surface(_) => None,
        }
    }

    /// A profile's tabs placed by hand (docs/specs/tabs-by-hand.md); None without tabs.
    pub fn tab_points_mut(&mut self) -> Option<&mut Vec<[f64; 2]>> {
        match self {
            Operation::Profile(ProfileOp {
                tabs: Some(tabs), ..
            }) => Some(&mut tabs.at),
            _ => None,
        }
    }

    /// Whether any corner gets a dogbone or T-bone: from `corners`, or a corner's own
    /// choice.
    pub fn has_bones(&self) -> bool {
        has_bones(self.corners(), self.corner_choices())
    }

    pub fn lead(&self) -> Option<Lead> {
        match self {
            Operation::Pocket(p) => p.lead,
            // A profile spirals down: no plunge to hide in the waste.
            Operation::Profile(_)
            | Operation::Vcarve(_)
            | Operation::Path(_)
            | Operation::Holes(_)
            | Operation::Surface(_) => None,
        }
    }

    pub fn clearance(&self) -> f64 {
        match self {
            Operation::Profile(op) => op.clearance,
            Operation::Pocket(op) => op.clearance,
            // A V-carve's cut is set by the outline and the angle alone.
            Operation::Vcarve(_) => 0.0,
            // On the line: no offset.
            Operation::Path(_) => 0.0,
            // Cut to the drawn size (docs/specs/holes.md decision 9).
            Operation::Holes(_) => 0.0,
            // The whole stock: no wall to clear.
            Operation::Surface(_) => 0.0,
        }
    }

    pub fn select(&self) -> &Selector {
        match self {
            Operation::Profile(op) => &op.select,
            Operation::Pocket(op) => &op.select,
            Operation::Vcarve(op) => &op.select,
            Operation::Path(op) => &op.select,
            Operation::Holes(op) => &op.select,
            Operation::Surface(_) => &NO_SELECTOR,
        }
    }

    /// None for an operation that selects no shapes (a surface).
    pub fn select_mut(&mut self) -> Option<&mut Selector> {
        match self {
            Operation::Profile(op) => Some(&mut op.select),
            Operation::Pocket(op) => Some(&mut op.select),
            Operation::Vcarve(op) => Some(&mut op.select),
            Operation::Path(op) => Some(&mut op.select),
            Operation::Holes(op) => Some(&mut op.select),
            Operation::Surface(_) => None,
        }
    }

    /// Whether it cuts shapes of the drawing; a surface faces the stock instead.
    pub fn selects_shapes(&self) -> bool {
        !matches!(self, Operation::Surface(_))
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct ProfileOp {
    /// A name for the operation, shown in the list and on the setup sheet. Empty: none.
    #[serde(default)]
    pub name: String,
    /// False: the operation stays in the project but isn't cut. Missing: true.
    #[serde(default = "enabled_default")]
    pub enabled: bool,
    /// The id of the part it belongs to (`parts`).
    // Spec: docs/specs/parts.md.
    pub part: u64,
    /// The library material its feeds come from. Null: its own values.
    // Spec: docs/specs/materials.md §2 (optional, no schema bump).
    #[serde(default)]
    pub material: Option<MaterialRef>,
    /// What the operation cuts: the shapes on a layer, or shapes by id.
    pub select: Selector,
    /// Which side of the line the tool cuts: `outside` (cutting a part out), `inside` (a
    /// hole) or `on` the line.
    pub side: Side,
    /// `climb` or `conventional` milling.
    pub direction: Direction,
    /// The id of the project tool it cuts with (`tools`).
    pub tool: i64,
    /// Where the cut starts, mm down from the stock top: 0 for the top, more when an
    /// earlier operation already cut down to there.
    // Spec: docs/specs/depths-and-clearance.md.
    pub start_depth: f64,
    /// Where the cut ends, mm down from the stock top. More than the start depth; below the
    /// stock bottom only within `stock.max_overcut`.
    pub end_depth: f64,
    /// Cut through the stock: how far below the stock bottom it ends, mm. The end depth is
    /// then the stock thickness plus this, and follows the thickness. Null: the end depth as
    /// set.
    // Spec: docs/specs/job-settings-layout.md §5 (optional, no schema bump).
    #[serde(default)]
    pub through: Option<f64>,
    /// How far the cut moves into the material that is kept, mm: positive removes more (a
    /// larger pocket, a smaller part), negative leaves material. Less than the tool radius.
    pub clearance: f64,
    /// The deepest each pass goes, mm. The passes are equal, and the last one ends exactly
    /// at the end depth.
    pub step_down: f64,
    /// The cutting feed, mm/min.
    pub feed: f64,
    /// The feed going down into the material, mm/min.
    pub plunge_feed: f64,
    /// Holding tabs left at the bottom of the cut, so a part cut out doesn't come loose.
    /// Null: none.
    #[serde(default)]
    pub tabs: Option<Tabs>,
    /// The steepest the spiral ramp goes down along the contour, radians: more than 0, at
    /// most 30° (0.5236). A profile always spirals down (docs/specs/profile-always-spiral.md).
    // Spec: docs/specs/profile-spiral.md, docs/specs/profile-ramp-level.md.
    pub ramp_angle: f64,
    /// Where the cut starts, as points in drawing coordinates (mm): each moves the start of
    /// the ring (contour) nearest to it. Empty: chosen by Chipload.
    // Spec: docs/specs/helix-and-leads.md, part A.
    #[serde(default)]
    pub starts: Vec<[f64; 2]>,
    /// A finishing pass: the roughing passes leave `stock` mm on the wall, and one last lap
    /// at full depth takes it off. Null: none.
    // Spec: docs/specs/finish-and-skin.md.
    #[serde(default)]
    pub finish: Option<Finish>,
    /// An onion skin: the passes stop `thickness` mm above the end depth, and one last lap
    /// cuts through it. Null: none.
    // Spec: docs/specs/finish-and-skin.md.
    #[serde(default)]
    pub skin: Option<Skin>,
    /// Dogbones or T-bones in the inside corners, so square parts fit. Null: the corners
    /// stay round.
    // Spec: docs/specs/dogbones.md.
    #[serde(default)]
    pub corners: Option<Corners>,
    /// Corners with their own bone, whatever `corners` says. Missing or empty: none. Left
    /// out when empty.
    // Spec: docs/specs/dogbone-per-corner.md.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub corner_choices: Vec<CornerChoice>,
}

/// How the tool reaches into the inside corners it would leave round, so a square part
/// fits: every corner on the tool's side up to `max_angle`.
// Spec: docs/specs/dogbones.md.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "type", rename_all = "lowercase", deny_unknown_fields)]
pub enum Corners {
    /// In along the corner's bisector.
    Dogbone {
        /// Only corners this sharp or sharper, radians (1.745 = 100°).
        max_angle: f64,
    },
    /// In along the longer wall: the notch is in the shorter one.
    Tbone {
        /// Only corners this sharp or sharper, radians (1.745 = 100°).
        max_angle: f64,
    },
}

impl Corners {
    pub fn max_angle(&self) -> f64 {
        match *self {
            Corners::Dogbone { max_angle } | Corners::Tbone { max_angle } => max_angle,
        }
    }
}

/// A corner with its own bone, chosen by the user.
// Spec: docs/specs/dogbone-per-corner.md.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct CornerChoice {
    /// The corner, in drawing coordinates (mm).
    pub at: [f64; 2],
    /// `none` (the corner stays round), `dogbone` or `tbone`.
    pub bone: Bone,
    /// The wall a T-bone runs along, `longer` or `shorter`: the notch is in the other.
    /// Required with a T-bone; null otherwise.
    #[serde(default)]
    pub along: Option<Wall>,
}

/// What a chosen corner gets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum Bone {
    None,
    Dogbone,
    Tbone,
}

/// One of a corner's two walls, by length.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum Wall {
    Longer,
    Shorter,
}

/// What is wrong with an operation's corner choices (docs/specs/dogbone-per-corner.md
/// §3): a T-bone without its wall, a wall without a T-bone, two choices at one corner, or
/// choices on a profile on the line. Whether each matches a corner is known only with the
/// drawing (`chipload_toolpath`).
fn corner_choice_problems(i: usize, op: &Operation) -> Vec<Problem> {
    let mut problems = Vec::new();
    let choices = op.corner_choices();
    if let Operation::Profile(p) = op
        && p.side == Side::On
        && !choices.is_empty()
    {
        problems.push(Problem::at(
            format!("operations.{i}.corner_choices"),
            Message::new("job-on-line-corners"),
        ));
    }
    for (k, choice) in choices.iter().enumerate() {
        let at = |field: &str| format!("operations.{i}.corner_choices.{k}.{field}");
        match (choice.bone, choice.along) {
            (Bone::Tbone, None) => {
                problems.push(Problem::at(at("along"), Message::new("job-tbone-wall")));
            }
            (Bone::None | Bone::Dogbone, Some(_)) => {
                problems.push(Problem::at(
                    at("along"),
                    Message::new("job-wall-only-tbone"),
                ));
            }
            _ => {}
        }
        let twice = choices[..k].iter().any(|c| {
            distance((c.at[0], c.at[1]), (choice.at[0], choice.at[1])) <= ARC_APPROX_TOL_MM
        });
        if twice {
            problems.push(Problem::at(
                at("at"),
                Message::new("job-corner-chosen-twice").arg(
                    "at",
                    Value::Point(choice.at[0], choice.at[1], Digits::Fixed(3)),
                ),
            ));
        }
    }
    problems
}

/// Whether any corner gets a dogbone or T-bone: from `corners`, or a corner's own choice.
pub fn has_bones(corners: Option<Corners>, choices: &[CornerChoice]) -> bool {
    corners.is_some() || choices.iter().any(|c| c.bone != Bone::None)
}

/// The largest corner angle new dogbones get: 100°, so 90° corners drawn slightly off count.
pub const DEFAULT_CORNER_ANGLE_RAD: f64 = 100.0 * std::f64::consts::PI / 180.0;

/// A finishing pass on a profile.
// Spec: docs/specs/finish-and-skin.md.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Finish {
    /// Left on the wall by the roughing passes, mm: more than 0, less than the tool radius.
    pub stock: f64,
}

/// An onion skin on a profile.
// Spec: docs/specs/finish-and-skin.md.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Skin {
    /// Left above the end depth until the last lap, mm: more than 0, at most the step down.
    pub thickness: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct PocketOp {
    /// A name for the operation, shown in the list and on the setup sheet. Empty: none.
    #[serde(default)]
    pub name: String,
    /// False: the operation stays in the project but isn't cut. Missing: true.
    #[serde(default = "enabled_default")]
    pub enabled: bool,
    /// The id of the part it belongs to (`parts`).
    // Spec: docs/specs/parts.md.
    pub part: u64,
    /// The library material its feeds come from. Null: its own values.
    // Spec: docs/specs/materials.md §2 (optional, no schema bump).
    #[serde(default)]
    pub material: Option<MaterialRef>,
    /// What the operation cuts: the shapes on a layer, or shapes by id.
    pub select: Selector,
    /// The id of the project tool it cuts with (`tools`).
    pub tool: i64,
    /// Where the cut starts, mm down from the stock top: 0 for the top, more when an
    /// earlier operation already cut down to there.
    // Spec: docs/specs/depths-and-clearance.md.
    pub start_depth: f64,
    /// Where the cut ends, mm down from the stock top. More than the start depth; below the
    /// stock bottom only within `stock.max_overcut`.
    pub end_depth: f64,
    /// Cut through the stock: how far below the stock bottom it ends, mm. The end depth is
    /// then the stock thickness plus this, and follows the thickness. Null: the end depth as
    /// set.
    // Spec: docs/specs/job-settings-layout.md §5 (optional, no schema bump).
    #[serde(default)]
    pub through: Option<f64>,
    /// How far the cut moves into the material that is kept, mm: positive removes more (a
    /// larger pocket, a smaller part), negative leaves material. Less than the tool radius.
    pub clearance: f64,
    /// The deepest each pass goes, mm. The passes are equal, and the last one ends exactly
    /// at the end depth.
    pub step_down: f64,
    /// The distance between the clearing rings, as a fraction of the tool diameter: more
    /// than 0, at most 1.
    pub stepover: f64,
    /// `climb` or `conventional` milling.
    pub direction: Direction,
    /// The cutting feed, mm/min.
    pub feed: f64,
    /// The feed going down into the material, mm/min.
    pub plunge_feed: f64,
    /// How the tool goes down at the start of each pass: `plunge` (straight down; needs a
    /// centre-cutting tool) or `ramp` along the path; pockets also `helix`. Missing:
    /// plunge.
    // Spec: docs/specs/ramp-entry.md.
    #[serde(default)]
    pub entry: Entry,
    /// Where each wall ring starts, as points in drawing coordinates (mm): each moves the
    /// start of the wall ring nearest to it. Empty: chosen by Chipload.
    // Spec: docs/specs/helix-and-leads.md, part A.
    #[serde(default)]
    pub starts: Vec<[f64; 2]>,
    /// A quarter-circle lead-in and lead-out from the cleared side onto the wall. Null:
    /// none.
    // Spec: docs/specs/helix-and-leads.md, part B.
    #[serde(default)]
    pub lead: Option<Lead>,
    /// Dogbones or T-bones in the inside corners, so square parts fit. Null: the corners
    /// stay round.
    // Spec: docs/specs/dogbones.md.
    #[serde(default)]
    pub corners: Option<Corners>,
    /// Corners with their own bone, whatever `corners` says. Missing or empty: none. Left
    /// out when empty.
    // Spec: docs/specs/dogbone-per-corner.md.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub corner_choices: Vec<CornerChoice>,
    /// Rest machining: clear only what a larger tool leaves in these shapes. Null: the
    /// whole pocket.
    // Spec: docs/specs/rest-machining.md.
    #[serde(default)]
    pub rest: Option<Rest>,
    /// Rest passes: smaller endmills that clear what this pocket's tool (or the pass before)
    /// leaves, in its shapes and depths. Missing or empty: none. Left out when empty.
    // Spec: docs/specs/rest-passes.md.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rest_passes: Vec<RestPass>,
}

/// One rest pass of a pocket (docs/specs/rest-passes.md §2): it takes the pocket's shapes,
/// depths, clearance and direction, and clears only what the tool before it leaves.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct RestPass {
    /// The id of the project tool it cuts with (`tools`): an endmill smaller than the tool
    /// before it.
    pub tool: i64,
    /// The cutting feed, mm/min.
    pub feed: f64,
    /// The feed going down into the material, mm/min.
    pub plunge_feed: f64,
    /// The deepest each pass goes, mm.
    pub step_down: f64,
    /// The distance between the clearing rings, as a fraction of the tool diameter.
    pub stepover: f64,
    /// How the tool goes down: `plunge`, `ramp` or `helix`.
    pub entry: Entry,
    /// False: kept, but not cut. Missing: true.
    #[serde(default = "enabled_default")]
    pub enabled: bool,
}

/// Rest machining: the pocket clears only the material the larger tool `after_tool` can't
/// reach in its shapes.
// Spec: docs/specs/rest-machining.md.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Rest {
    /// The id of the larger project tool whose leftovers this pocket clears.
    pub after_tool: i64,
}

/// V-carve: a V-bit follows the centre line of closed shapes, as deep as makes the cut
/// exactly as wide as the shape.
// Spec: docs/specs/v-carve.md.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct VcarveOp {
    /// A name for the operation, shown in the list and on the setup sheet. Empty: none.
    #[serde(default)]
    pub name: String,
    /// False: the operation stays in the project but isn't cut. Missing: true.
    #[serde(default = "enabled_default")]
    pub enabled: bool,
    /// The id of the part it belongs to (`parts`).
    // Spec: docs/specs/parts.md.
    pub part: u64,
    /// The library material its feeds come from. Null: its own values.
    // Spec: docs/specs/materials.md §2 (optional, no schema bump).
    #[serde(default)]
    pub material: Option<MaterialRef>,
    /// What the operation cuts: the shapes on a layer, or shapes by id.
    pub select: Selector,
    /// The id of the project tool it cuts with (`tools`).
    pub tool: i64,
    /// The surface carved, mm down from the stock top: 0 for the top, more to carve on a
    /// floor an earlier pocket cut.
    pub start_depth: f64,
    /// The deepest the V-bit's tip may go, mm down from the stock top. Strokes that need
    /// more are carved to it, narrower than drawn, with a warning.
    pub end_depth: f64,
    /// Carve in passes no deeper than this, mm. Null: one pass.
    #[serde(default)]
    pub step_down: Option<f64>,
    /// The feed, mm/min, for moves no steeper than 45°.
    pub feed: f64,
    /// The feed for moves steeper than 45°, mm/min.
    pub plunge_feed: f64,
}

/// Follow path: the tool centre on the selected shapes, open or closed, in depth passes.
// Spec: docs/specs/follow-path.md.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct PathOp {
    /// A name for the operation, shown in the list and on the setup sheet. Empty: none.
    #[serde(default)]
    pub name: String,
    /// False: the operation stays in the project but isn't cut. Missing: true.
    #[serde(default = "enabled_default")]
    pub enabled: bool,
    /// The id of the part it belongs to (`parts`).
    // Spec: docs/specs/parts.md.
    pub part: u64,
    /// The library material its feeds come from. Null: its own values.
    // Spec: docs/specs/materials.md §2 (optional, no schema bump).
    #[serde(default)]
    pub material: Option<MaterialRef>,
    /// What the operation cuts: the shapes on a layer, or shapes by id.
    pub select: Selector,
    /// The id of the project tool it cuts with (`tools`).
    pub tool: i64,
    /// Where the cut starts, mm down from the stock top: 0 for the top, more when an
    /// earlier operation already cut down to there.
    pub start_depth: f64,
    /// Where the cut ends, mm down from the stock top. More than the start depth; below the
    /// stock bottom only within `stock.max_overcut`.
    pub end_depth: f64,
    /// Cut through the stock: how far below the stock bottom it ends, mm. The end depth is
    /// then the stock thickness plus this, and follows the thickness. Null: the end depth as
    /// set.
    // Spec: docs/specs/job-settings-layout.md §5 (optional, no schema bump).
    #[serde(default)]
    pub through: Option<f64>,
    /// The deepest each pass goes, mm. The passes are equal, and the last one ends exactly
    /// at the end depth.
    pub step_down: f64,
    /// The cutting feed, mm/min.
    pub feed: f64,
    /// The feed going down into the material, mm/min.
    pub plunge_feed: f64,
    /// How the tool goes down at the start of each pass: `plunge` or `ramp` along the path.
    pub entry: Entry,
}

/// Holes: every selected circle, drilled with a drill or bored with a helix with an
/// endmill.
// Spec: docs/specs/holes.md.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct HolesOp {
    /// A name for the operation, shown in the list and on the setup sheet. Empty: none.
    #[serde(default)]
    pub name: String,
    /// False: the operation stays in the project but isn't cut. Missing: true.
    #[serde(default = "enabled_default")]
    pub enabled: bool,
    /// The id of the part it belongs to (`parts`).
    // Spec: docs/specs/parts.md.
    pub part: u64,
    /// The library material its feeds come from. Null: its own values.
    // Spec: docs/specs/materials.md §2 (optional, no schema bump).
    #[serde(default)]
    pub material: Option<MaterialRef>,
    /// What the operation cuts: the circles among the shapes on a layer, or shapes by id.
    /// Other shapes are skipped with a note.
    pub select: Selector,
    /// The id of the project tool it cuts with (`tools`).
    pub tool: i64,
    /// Where the cut starts, mm down from the stock top: 0 for the top, more when an
    /// earlier operation already cut down to there.
    pub start_depth: f64,
    /// Where the hole ends, mm down from the stock top. For a drill, the depth of its
    /// point.
    pub end_depth: f64,
    /// Cut through the stock: how far below the stock bottom it ends, mm. The end depth is
    /// then the stock thickness plus this, and follows the thickness. Null: the end depth as
    /// set.
    // Spec: docs/specs/job-settings-layout.md §5 (optional, no schema bump).
    #[serde(default)]
    pub through: Option<f64>,
    /// A bore's feed round its helix and bottom circle, mm/min. A drill doesn't use it.
    pub feed: f64,
    /// A drill's feed down, or a bore's feed straight down to the start depth, mm/min.
    pub plunge_feed: f64,
    /// How the holes are cut: `drill` with a drill, or `bore` (a helix) with an endmill.
    pub method: HoleMethod,
}

/// Surface: the whole stock top taken down flat by a wide flat endmill, in straight passes
/// back and forth along the stock's longer side, each depth started beside the stock.
// Spec: docs/specs/surfacing.md.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct SurfaceOp {
    /// A name for the operation, shown in the list and on the setup sheet. Empty: none.
    #[serde(default)]
    pub name: String,
    /// False: the operation stays in the project but isn't cut. Missing: true.
    #[serde(default = "enabled_default")]
    pub enabled: bool,
    /// The id of the part it belongs to (`parts`). The stock is faced once, whatever copies
    /// the part has.
    // Spec: docs/specs/parts.md.
    pub part: u64,
    /// The library material its feeds come from. Null: its own values.
    // Spec: docs/specs/materials.md §2 (optional, no schema bump).
    #[serde(default)]
    pub material: Option<MaterialRef>,
    /// The id of the project tool it cuts with (`tools`): an endmill.
    pub tool: i64,
    /// Where the cut starts, mm down from the stock top: 0 for the top.
    pub start_depth: f64,
    /// How far down the stock top is taken, mm. At most the stock thickness.
    pub end_depth: f64,
    /// The deepest each pass goes, mm. The passes are equal, and the last one ends exactly
    /// at the end depth.
    pub step_down: f64,
    /// The distance between passes, as a fraction of the tool diameter: at least 0.1, at
    /// most 0.9.
    pub stepover: f64,
    /// The cutting feed, mm/min.
    pub feed: f64,
    /// The feed going down beside the stock to each depth, mm/min.
    pub plunge_feed: f64,
}

/// The smallest and largest stepover of a surface, as a fraction of the tool diameter
/// (docs/specs/surfacing.md §4).
pub const SURFACE_STEPOVER: (f64, f64) = (0.1, 0.9);

/// A surface's own rules (docs/specs/surfacing.md §4, §5): its stepover's range, and no
/// deeper than the stock.
fn surface_problems(i: usize, f: &SurfaceOp, thickness: f64) -> Vec<Problem> {
    let mut problems = Vec::new();
    let (lo, hi) = SURFACE_STEPOVER;
    if !(f.stepover.is_finite() && (lo..=hi).contains(&f.stepover)) {
        problems.push(Problem::at(
            format!("operations.{i}.stepover"),
            Message::new("job-surface-stepover")
                .arg("low", Value::Number(lo, Digits::Shortest))
                .arg("high", Value::Number(hi, Digits::Shortest))
                .arg("got", Value::Number(f.stepover, Digits::Shortest)),
        ));
    }
    if f.end_depth > thickness + DEPTH_TOL_MM {
        problems.push(Problem::at(
            format!("operations.{i}.end_depth"),
            Message::new("job-surface-too-deep")
                .arg("thickness", Value::Length(thickness, Digits::Shortest))
                .arg("got", Value::Length(f.end_depth, Digits::Shortest)),
        ));
    }
    problems
}

/// How a Holes operation cuts.
// Spec: docs/specs/holes.md §2.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "type", rename_all = "lowercase", deny_unknown_fields)]
pub enum HoleMethod {
    /// Straight down at the hole's centre, `peck` mm at a time.
    Drill {
        /// Drill this many mm at a time, lifting out between pecks to clear chips. Null: in
        /// one go.
        peck: Option<f64>,
    },
    /// A helix at the wall, at most `step_down` mm per turn and never steeper than
    /// `ramp_angle` (radians, 0 < angle <= 30°), then one level circle.
    Bore {
        /// The most the helix goes down per turn, mm.
        step_down: f64,
        /// The steepest the helix goes, radians: more than 0, at most 30° (0.5236).
        ramp_angle: f64,
        /// `climb` or `conventional`, as for a profile inside the hole.
        direction: Direction,
    },
}

impl HoleMethod {
    /// The method a tool of `kind` and `diameter` takes, with its defaults
    /// (docs/specs/holes.md §7); None for a V-bit.
    pub fn default_for(kind: ToolType, diameter: f64) -> Option<HoleMethod> {
        match kind {
            ToolType::Drill => Some(HoleMethod::Drill { peck: None }),
            ToolType::Endmill => Some(HoleMethod::Bore {
                step_down: 0.5 * diameter,
                ramp_angle: DEFAULT_RAMP_ANGLE_RAD,
                direction: Direction::Climb,
            }),
            ToolType::Vbit => None,
        }
    }

    /// The tool type this method needs.
    pub fn tool_type(&self) -> ToolType {
        match self {
            HoleMethod::Drill { .. } => ToolType::Drill,
            HoleMethod::Bore { .. } => ToolType::Endmill,
        }
    }
}

/// Quarter-circle lead-in and lead-out.
// Spec: docs/specs/helix-and-leads.md part B.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Lead {
    /// Radius of the lead arcs, mm: more than 0, at most 10 tool diameters. Smaller where
    /// there's no room.
    pub radius: f64,
}

/// The longest lead allowed, in tool diameters.
pub const MAX_LEAD_DIAMETERS: f64 = 10.0;

/// How the tool goes down into the material at the start of each pass, for pockets and
/// follow paths (a profile always spirals: its `ramp_angle`).
// Spec: docs/specs/ramp-entry.md.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "type", rename_all = "lowercase", deny_unknown_fields)]
pub enum Entry {
    /// Straight down: needs a centre-cutting tool. An empty struct variant, as `Rings`.
    Plunge {},
    /// Down along the path and back, never steeper than `ramp_angle` (radians,
    /// 0 < angle <= 30°).
    Ramp {
        /// The steepest the ramp goes, radians: more than 0, at most 30° (0.5236).
        ramp_angle: f64,
    },
    /// Pockets only: down round a circle of `radius` (the tool centre's), never steeper
    /// than `ramp_angle`.
    // Spec: docs/specs/helix-and-leads.md part C.
    Helix {
        /// The helix's radius, mm: less than the tool radius.
        radius: f64,
        /// The steepest the helix goes, radians: more than 0, at most 30° (0.5236).
        ramp_angle: f64,
    },
}

impl Entry {
    /// How steeply a ramp or helix goes down; None for a straight plunge.
    pub fn ramp_angle(&self) -> Option<f64> {
        match *self {
            Entry::Plunge {} => None,
            Entry::Ramp { ramp_angle } | Entry::Helix { ramp_angle, .. } => Some(ramp_angle),
        }
    }

    /// A helix for a tool of `diameter`, with the default radius and angle.
    pub fn default_helix(diameter: f64) -> Entry {
        Entry::Helix {
            radius: DEFAULT_HELIX_DIAMETERS * diameter,
            ramp_angle: DEFAULT_RAMP_ANGLE_RAD,
        }
    }
}

/// The helix radius a new helix gets, in tool diameters.
pub const DEFAULT_HELIX_DIAMETERS: f64 = 0.25;

impl Default for Entry {
    fn default() -> Self {
        Entry::Plunge {}
    }
}

/// The steepest ramp allowed: 30°.
pub const MAX_RAMP_ANGLE_RAD: f64 = std::f64::consts::PI / 6.0;
/// The ramp angle a new ramp gets: 3°.
pub const DEFAULT_RAMP_ANGLE_RAD: f64 = 3.0 * std::f64::consts::PI / 180.0;

impl Drawing {
    /// The box round the shapes that are cut (construction geometry left out), arcs
    /// included, mm: ((x0, y0), (x1, y1)). None for an empty drawing.
    pub fn extents(&self) -> Option<((f64, f64), (f64, f64))> {
        self.shapes
            .iter()
            .filter(|s| !s.construction)
            .flat_map(Shape::segments)
            .map(|seg| seg.to_primitive().bounds())
            .reduce(|(lo, hi), (a, b)| {
                (
                    (lo.0.min(a.0), lo.1.min(a.1)),
                    (hi.0.max(b.0), hi.1.max(b.1)),
                )
            })
    }

    /// The `hidden` key of the group a shape is in (docs/specs/drawing-sources.md §4):
    /// `dxf:1`, `image:1`, `drawn:circle`, `drawn:rectangle`, `drawn:polyline`, or `paths`
    /// for a path from neither.
    pub fn group_key(shape: &Shape) -> String {
        if let Some(dxf) = shape.dxf() {
            return format!("dxf:{dxf}");
        }
        if let Some(image) = shape.image() {
            return format!("image:{image}");
        }
        if let Some(text) = shape.text() {
            return format!("text:{text}");
        }
        match shape.geometry {
            Geometry::Circle { .. } => "drawn:circle".into(),
            Geometry::Rectangle { .. } => "drawn:rectangle".into(),
            Geometry::Polyline { .. } => "drawn:polyline".into(),
            Geometry::Polygon { .. } => "drawn:polygon".into(),
            Geometry::Path { .. } => "paths".into(),
        }
    }

    /// The `hidden` key of a layer within a group: `dxf:1/layer:Cut`.
    pub fn layer_key(group: &str, layer: &str) -> String {
        format!("{group}/layer:{layer}")
    }

    /// Whether the shape's group, or its layer within the group, is hidden.
    pub fn is_hidden(&self, shape: &Shape) -> bool {
        if self.hidden.is_empty() {
            return false;
        }
        let group = Self::group_key(shape);
        let layer = Self::layer_key(&group, &shape.layer);
        self.hidden.iter().any(|h| *h == group || *h == layer)
    }

    /// Structural rules of the drawing, each with the field it is about.
    pub fn validate(&self) -> Vec<Problem> {
        let mut problems = Vec::new();
        let mut seen = std::collections::BTreeSet::new();
        for (i, shape) in self.shapes.iter().enumerate() {
            let at = format!("drawing.shapes.{i}");
            if shape.id == 0 || shape.id >= self.next_shape_id {
                problems.push(Problem::at(
                    format!("{at}.id"),
                    Message::new("job-shape-id-range")
                        .arg("most", self.next_shape_id.saturating_sub(1))
                        .arg("got", shape.id),
                ));
            }
            if !seen.insert(shape.id) {
                problems.push(Problem::at(
                    format!("{at}.id"),
                    Message::new("job-id-twice").arg("id", shape.id),
                ));
            }
            // The text names the shape as well, which has no field of its own.
            problems.extend(shape_problems(&shape.geometry).into_iter().map(|p| {
                let text = Message::new("job-shape-problem")
                    .arg("at", at.as_str())
                    .arg("id", shape.id)
                    .arg("problem", p.text_message);
                Problem::new(format!("{at}.{}", p.path), p.short_message, text)
            }));
            // No name is None, never "" (docs/specs/drawn-shapes-box.md §4).
            if shape.name.as_ref().is_some_and(|n| n.trim().is_empty()) {
                problems.push(Problem::at(
                    format!("{at}.name"),
                    Message::new("job-must-not-be-empty"),
                ));
            }
            if let Some(image) = shape.image()
                && !self.images.iter().any(|i| i.id == image)
            {
                problems.push(Problem::at(
                    format!("{at}.image"),
                    Message::new("job-no-image").arg("id", image),
                ));
            }
            if let Some(dxf) = shape.dxf()
                && !self.dxfs.iter().any(|d| d.id == dxf)
            {
                problems.push(Problem::at(
                    format!("{at}.dxf"),
                    Message::new("job-no-dxf").arg("id", dxf),
                ));
            }
            if let Some(text) = shape.text()
                && !self.texts.iter().any(|t| t.id == text)
            {
                problems.push(Problem::at(
                    format!("{at}.text"),
                    Message::new("job-no-text").arg("id", text),
                ));
            }
            let sources = [shape.image(), shape.dxf(), shape.text()];
            if sources.iter().flatten().count() > 1 {
                problems.push(Problem::at(at, Message::new("job-shape-one-source")));
            }
        }
        let mut ids = std::collections::BTreeSet::new();
        for (i, image) in self.images.iter().enumerate() {
            if !ids.insert(image.id) {
                problems.push(Problem::at(
                    format!("drawing.images.{i}.id"),
                    Message::new("job-id-twice").arg("id", image.id),
                ));
            }
            problems.extend(image.problems(i));
        }
        let mut ids = std::collections::BTreeSet::new();
        for (i, text) in self.texts.iter().enumerate() {
            if !ids.insert(text.id) {
                problems.push(Problem::at(
                    format!("drawing.texts.{i}.id"),
                    Message::new("job-id-twice").arg("id", text.id),
                ));
            }
            problems.extend(text.problems(i));
        }
        let mut ids = std::collections::BTreeSet::new();
        for (i, dxf) in self.dxfs.iter().enumerate() {
            if !ids.insert(dxf.id) {
                problems.push(Problem::at(
                    format!("drawing.dxfs.{i}.id"),
                    Message::new("job-id-twice").arg("id", dxf.id),
                ));
            }
            if dxf.name.trim().is_empty() {
                problems.push(Problem::at(
                    format!("drawing.dxfs.{i}.name"),
                    Message::new("job-must-not-be-empty"),
                ));
            }
            if let Some(px) = dxf.px_per_inch
                && !(px.is_finite() && px > 0.0)
            {
                problems.push(Problem::at(
                    format!("drawing.dxfs.{i}.px_per_inch"),
                    Message::new("job-must-be-positive-number")
                        .arg("got", Value::Number(px, Digits::Shortest)),
                ));
            }
        }
        problems
    }
}

/// Value rules of one shape, as "field: message" (spec drawing-tools.md section 1).
/// What is wrong with a shape's values, as "field: message"; empty when it is valid.
pub fn shape_problems(geometry: &Geometry) -> Vec<Problem> {
    let mut problems = Vec::new();
    let finite = |p: &[f64; 2]| p.iter().all(|v| v.is_finite());
    let mut positive = |field: &str, value: f64| {
        if !(value.is_finite() && value > 0.0) {
            problems.push(Problem::at(
                field,
                Message::new("job-must-be-positive-number")
                    .arg("got", Value::Length(value, Digits::Shortest)),
            ));
        }
    };
    match geometry {
        Geometry::Path { segments, .. } => {
            if segments.is_empty() {
                problems.push(Problem::at(
                    "segments",
                    Message::new("job-path-no-segments"),
                ));
            }
            for (j, segment) in segments.iter().enumerate() {
                let ok = match segment {
                    PathSegment::Line(l) => finite(&l.start) && finite(&l.end),
                    PathSegment::Arc(a) => {
                        finite(&a.center)
                            && a.start_angle.is_finite()
                            && a.sweep.is_finite()
                            && a.radius.is_finite()
                            && a.radius > 0.0
                    }
                };
                if !ok {
                    problems.push(Problem::at(
                        format!("segments.{j}"),
                        Message::new("job-segment-not-finite"),
                    ));
                }
            }
        }
        Geometry::Circle { center, radius } => {
            positive("radius", *radius);
            if !finite(center) {
                problems.push(Problem::at("center", Message::new("job-must-be-finite")));
            }
        }
        Geometry::Rectangle {
            center,
            length,
            width,
            angle,
            corner_radius,
        } => {
            positive("length", *length);
            positive("width", *width);
            let most = length.min(*width) / 2.0;
            if !corner_radius.is_finite() || *corner_radius < 0.0 {
                problems.push(Problem::at(
                    "corner_radius",
                    Message::new("job-must-be-0-or-more")
                        .arg("got", Value::Length(*corner_radius, Digits::Shortest)),
                ));
            } else if *corner_radius > 0.0 && *corner_radius < CHAIN_TOL_MM {
                problems.push(Problem::at(
                    "corner_radius",
                    Message::new("job-corner-radius-tiny")
                        .arg("least", Value::Length(CHAIN_TOL_MM, Digits::Shortest))
                        .arg("got", Value::Length(*corner_radius, Digits::Shortest)),
                ));
            } else if *corner_radius > most {
                problems.push(Problem::at(
                    "corner_radius",
                    Message::new("job-corner-radius-too-large")
                        .arg("most", Value::Length(most, Digits::Shortest))
                        .arg("got", Value::Length(*corner_radius, Digits::Shortest)),
                ));
            }
            if !finite(center) {
                problems.push(Problem::at("center", Message::new("job-must-be-finite")));
            }
            if !angle.is_finite() {
                problems.push(Problem::at(
                    "angle",
                    Message::new("job-must-be-finite-got")
                        .arg("got", Value::Number(*angle, Digits::Shortest)),
                ));
            }
        }
        Geometry::Polygon {
            center,
            radius,
            sides,
            angle,
        } => {
            positive("radius", *radius);
            if !POLYGON_SIDES.contains(sides) {
                problems.push(Problem::at(
                    "sides",
                    Message::new("job-polygon-sides")
                        .arg("least", *POLYGON_SIDES.start())
                        .arg("most", *POLYGON_SIDES.end())
                        .arg("got", *sides),
                ));
            }
            if !finite(center) {
                problems.push(Problem::at("center", Message::new("job-must-be-finite")));
            }
            if !angle.is_finite() {
                problems.push(Problem::at(
                    "angle",
                    Message::new("job-must-be-finite-got")
                        .arg("got", Value::Number(*angle, Digits::Shortest)),
                ));
            }
        }
        Geometry::Polyline {
            closed,
            points,
            bulges,
        } => {
            // Two points close a loop when a segment bends: a DXF polyline circle of two
            // half circles (docs/specs/explode-and-join.md §2).
            let bends = bulges.iter().any(|b| *b != 0.0);
            let least = if *closed && !(bends && points.len() == 2) {
                3
            } else {
                2
            };
            if points.len() < least {
                problems.push(Problem::at(
                    "points",
                    Message::new("job-polyline-points")
                        .arg("closed", if *closed { "closed" } else { "open" })
                        .arg("least", least)
                        .arg("got", points.len()),
                ));
            }
            if !points.iter().all(finite) {
                problems.push(Problem::at("points", Message::new("job-must-be-finite")));
            }
            let n = points.len();
            let pairs = if *closed { n } else { n.saturating_sub(1) };
            for i in 0..pairs {
                let (a, b) = (points[i], points[(i + 1) % n]);
                if (a[0] - b[0]).hypot(a[1] - b[1]) <= CHAIN_TOL_MM {
                    problems.push(Problem::at(
                        format!("points.{}", (i + 1) % n),
                        Message::new("job-point-on-previous")
                            .arg("tolerance", Value::Length(CHAIN_TOL_MM, Digits::Shortest)),
                    ));
                }
            }
            if bulges.len() != pairs {
                problems.push(Problem::at(
                    "bulges",
                    Message::new("job-bulges-count")
                        .arg("wanted", pairs)
                        .arg("got", bulges.len()),
                ));
            }
            for (i, b) in bulges.iter().enumerate() {
                if !(b.is_finite() && b.abs() <= 1.0) {
                    problems.push(Problem::at(
                        format!("bulges.{i}"),
                        Message::new("job-bulge-range")
                            .arg("got", Value::Number(*b, Digits::Shortest)),
                    ));
                }
            }
        }
    }
    problems
}

/// "<at>.<field>: must be greater than 0, got <value>" unless value > 0.
fn positive_problem(at: &str, field: &str, value: f64) -> Option<Problem> {
    (value <= 0.0 || value.is_nan()).then(|| {
        Problem::at(
            format!("{at}.{field}"),
            Message::new("job-must-be-positive").arg("got", Value::Length(value, Digits::Shortest)),
        )
    })
}

/// The rule for `max_overcut`, kept under `stock` in projects and in machine profiles.
pub(crate) fn max_overcut_problems(at: &str, overcut: f64) -> Vec<Problem> {
    if overcut.is_finite() && overcut >= 0.0 {
        Vec::new()
    } else {
        vec![Problem::at(
            format!("{at}.max_overcut"),
            Message::new("job-must-be-0-or-more")
                .arg("got", Value::Length(overcut, Digits::Shortest)),
        )]
    }
}

impl Machine {
    /// Value rules of a machine, each at "<at>.field"; shared by projects and machine
    /// profiles.
    pub fn problems(&self, at: &str) -> Vec<Problem> {
        let mut problems = Vec::new();
        if let WorkZero::Point { x, y } = self.work_zero
            && !(x.is_finite() && y.is_finite())
        {
            problems.push(Problem::at(
                format!("{at}.work_zero"),
                Message::new("job-work-zero-not-finite")
                    .arg("x", Value::LengthNumber(x, Digits::Shortest))
                    .arg("y", Value::LengthNumber(y, Digits::Shortest)),
            ));
        }
        if let Some(t) = &self.travel {
            for (axis, value) in [("x", t.x), ("y", t.y), ("z", t.z)] {
                problems.extend(positive_problem(&format!("{at}.travel"), axis, value));
            }
        }
        problems
    }
}

impl Tool {
    /// Value rules of a tool, each at "<at>.field"; shared by projects and the tool
    /// library.
    pub fn problems(&self, at: &str) -> Vec<Problem> {
        let mut problems: Vec<Problem> = positive_problem(at, "diameter", self.diameter)
            .into_iter()
            .collect();
        if let Some(angle) = self.angle
            && !(angle.is_finite() && angle > 0.0 && angle < std::f64::consts::PI)
        {
            problems.push(Problem::at(
                format!("{at}.angle"),
                Message::new("job-angle-below-180")
                    .arg("got", Value::Number(angle.to_degrees(), Digits::Fixed(3))),
            ));
        }
        if let Some(tip) = self.tip_diameter
            && !(tip.is_finite() && tip >= 0.0 && tip < self.diameter)
        {
            problems.push(Problem::at(
                format!("{at}.tip_diameter"),
                Message::new("job-tip-diameter")
                    .arg("diameter", Value::Length(self.diameter, Digits::Shortest))
                    .arg("got", Value::Length(tip, Digits::Shortest)),
            ));
        }
        problems
    }
}

impl Job {
    /// The parts' rules (docs/specs/parts.md §4): at least one, unique ids and names,
    /// valid copies, every operation in a part, and the operations grouped by part in the
    /// parts' order.
    fn part_problems(&self) -> Vec<Problem> {
        let mut problems = Vec::new();
        if self.parts.is_empty() {
            problems.push(Problem::at("parts", Message::new("job-no-parts")));
        }
        for (i, part) in self.parts.iter().enumerate() {
            if part.id == 0 {
                problems.push(Problem::at(
                    format!("parts.{i}.id"),
                    Message::new("job-must-be-1-or-more"),
                ));
            }
            if self.parts[..i].iter().any(|p| p.id == part.id) {
                problems.push(Problem::at(
                    format!("parts.{i}.id"),
                    Message::new("job-part-twice").arg("id", part.id),
                ));
            }
            let name = part.name.trim();
            if name.is_empty() {
                problems.push(Problem::at(
                    format!("parts.{i}.name"),
                    Message::new("job-must-not-be-empty"),
                ));
            } else if self.parts[..i]
                .iter()
                .any(|p| p.name.trim().eq_ignore_ascii_case(name))
            {
                problems.push(Problem::at(
                    format!("parts.{i}.name"),
                    Message::new("job-part-name-taken").arg("name", name),
                ));
            }
            if let Some(copies) = &part.copies {
                problems.extend(copies.problems(&format!("parts.{i}.copies")));
            }
        }
        let rank = |id: u64| self.parts.iter().position(|p| p.id == id);
        let mut last: Option<(usize, u64)> = None;
        for (i, op) in self.operations.iter().enumerate() {
            let Some(here) = rank(op.part()) else {
                problems.push(Problem::at(
                    format!("operations.{i}.part"),
                    Message::new("job-no-part").arg("id", op.part()),
                ));
                continue;
            };
            if let Some((before, before_id)) = last
                && here < before
            {
                problems.push(Problem::at(
                    format!("operations.{i}.part"),
                    Message::new("job-part-order")
                        .arg("part", op.part())
                        .arg("before", before_id),
                ));
            }
            if last.is_none_or(|(before, _)| here > before) {
                last = Some((here, op.part()));
            }
        }
        problems
    }

    /// Every value rule the project breaks, with the field it is about
    /// (docs/specs/live-problems.md): the constraints the serde types can't express.
    pub fn validate(&self) -> Vec<Problem> {
        let mut problems = self.drawing.validate();
        problems.extend(self.machine.problems("machine"));
        for (i, source) in self.drawing.dxfs.iter().enumerate() {
            let at = format!("drawing.dxfs.{i}.placement");
            let Placement {
                scale,
                rotation,
                dx,
                dy,
            } = source.placement;
            if !Placement::scale_in_range(scale) {
                problems.push(Problem::at(
                    format!("{at}.scale"),
                    Message::new("job-scale-range")
                        .arg("low", Value::Number(Placement::MIN_SCALE, Digits::Shortest))
                        .arg(
                            "high",
                            Value::Number(Placement::MAX_SCALE, Digits::Shortest),
                        )
                        .arg("got", Value::Number(scale, Digits::Shortest)),
                ));
            }
            if !(dx.is_finite() && dy.is_finite()) {
                problems.push(Problem::at(
                    &at,
                    Message::new("job-must-be-numbers")
                        .arg("x", Value::LengthNumber(dx, Digits::Shortest))
                        .arg("y", Value::LengthNumber(dy, Digits::Shortest)),
                ));
            }
            if !rotation_in_range(rotation) {
                problems.push(Problem::at(
                    format!("{at}.rotation"),
                    Message::new("job-rotation-range")
                        .arg("got", Value::Number(rotation, Digits::Shortest)),
                ));
            }
        }
        if let Some(margin) = self.stock.fit {
            if !(margin.is_finite() && margin >= 0.0) {
                problems.push(Problem::at(
                    "stock.fit",
                    Message::new("job-must-be-0-or-more")
                        .arg("got", Value::LengthNumber(margin, Digits::Shortest)),
                ));
            } else if self.stock.origin != StockOrigin::Drawing {
                problems.push(Problem::at(
                    "stock.fit",
                    Message::new("job-stock-fit-needs-centred"),
                ));
            }
        }
        let mut positive = |path: String, value: f64| {
            if value <= 0.0 || value.is_nan() {
                problems.push(Problem::at(
                    path,
                    Message::new("job-must-be-positive")
                        .arg("got", Value::Length(value, Digits::Shortest)),
                ));
            }
        };
        positive("stock.x".into(), self.stock.x);
        positive("stock.y".into(), self.stock.y);
        positive("stock.thickness".into(), self.stock.thickness);
        for (i, tool) in self.tools.iter().enumerate() {
            problems.extend(tool.problems(&format!("tools.{i}")));
        }
        // Feeds in mm/min, a stepover a fraction, everything else a length in mm (written,
        // so messages can be shown in inches: docs/specs/inch-units.md §5.1).
        let mut positive = |path: String, value: f64| {
            if value <= 0.0 || value.is_nan() {
                let got = if path.ends_with("feed") {
                    Value::Feed(value, Digits::Shortest)
                } else if path.ends_with("stepover") {
                    Value::Number(value, Digits::Shortest)
                } else {
                    Value::Length(value, Digits::Shortest)
                };
                problems.push(Problem::at(
                    path,
                    Message::new("job-must-be-positive").arg("got", got),
                ));
            }
        };
        for (i, op) in self.operations.iter().enumerate() {
            let at = |field: &str| format!("operations.{i}.{field}");
            match op {
                Operation::Profile(p) => {
                    positive(at("end_depth"), p.end_depth);
                    positive(at("step_down"), p.step_down);
                    positive(at("feed"), p.feed);
                    positive(at("plunge_feed"), p.plunge_feed);
                    if let Some(tabs) = &p.tabs {
                        positive(at("tabs.width"), tabs.width);
                        positive(at("tabs.height"), tabs.height);
                    }
                    if let Some(finish) = &p.finish {
                        positive(at("finish.stock"), finish.stock);
                    }
                    if let Some(skin) = &p.skin {
                        positive(at("skin.thickness"), skin.thickness);
                    }
                }
                Operation::Pocket(p) => {
                    positive(at("end_depth"), p.end_depth);
                    positive(at("step_down"), p.step_down);
                    positive(at("feed"), p.feed);
                    positive(at("plunge_feed"), p.plunge_feed);
                    positive(at("stepover"), p.stepover);
                    // Rest passes (docs/specs/rest-passes.md §7).
                    for (k, pass) in p.rest_passes.iter().enumerate() {
                        let at = |field: &str| format!("operations.{i}.rest_passes.{k}.{field}");
                        positive(at("step_down"), pass.step_down);
                        positive(at("feed"), pass.feed);
                        positive(at("plunge_feed"), pass.plunge_feed);
                        positive(at("stepover"), pass.stepover);
                    }
                }
                Operation::Vcarve(v) => {
                    positive(at("end_depth"), v.end_depth);
                    if let Some(step_down) = v.step_down {
                        positive(at("step_down"), step_down);
                    }
                    positive(at("feed"), v.feed);
                    positive(at("plunge_feed"), v.plunge_feed);
                }
                Operation::Path(p) => {
                    positive(at("end_depth"), p.end_depth);
                    positive(at("step_down"), p.step_down);
                    positive(at("feed"), p.feed);
                    positive(at("plunge_feed"), p.plunge_feed);
                }
                Operation::Surface(f) => {
                    positive(at("end_depth"), f.end_depth);
                    positive(at("step_down"), f.step_down);
                    positive(at("feed"), f.feed);
                    positive(at("plunge_feed"), f.plunge_feed);
                }
                Operation::Holes(h) => {
                    positive(at("end_depth"), h.end_depth);
                    positive(at("feed"), h.feed);
                    positive(at("plunge_feed"), h.plunge_feed);
                    match h.method {
                        HoleMethod::Drill { peck } => {
                            if let Some(peck) = peck {
                                positive(at("method.peck"), peck);
                            }
                        }
                        HoleMethod::Bore { step_down, .. } => {
                            positive(at("method.step_down"), step_down);
                        }
                    }
                }
            }
        }
        for (i, op) in self.operations.iter().enumerate() {
            let Some(below) = op.through() else {
                continue;
            };
            if !(below.is_finite() && below >= 0.0) {
                problems.push(Problem::at(
                    format!("operations.{i}.through"),
                    Message::new("job-must-be-0-or-more")
                        .arg("got", Value::LengthNumber(below, Digits::Shortest)),
                ));
            } else if (op.depths().1 - (self.stock.thickness + below)).abs() > DEPTH_TOL_MM {
                // A project read or edited follows the thickness (`follow_thickness`); one
                // that doesn't is refused rather than cut at an old depth.
                problems.push(Problem::at(
                    format!("operations.{i}.end_depth"),
                    Message::new("job-through-stale").arg(
                        "depth",
                        Value::Length(self.stock.thickness + below, Digits::Shortest),
                    ),
                ));
            }
        }
        problems.extend(max_overcut_problems("stock", self.stock.max_overcut));
        problems.extend(self.part_problems());
        for (i, op) in self.operations.iter().enumerate() {
            let (start, end) = op.depths();
            if !(start.is_finite() && start >= 0.0) {
                problems.push(Problem::at(
                    format!("operations.{i}.start_depth"),
                    Message::new("job-must-be-0-or-more")
                        .arg("got", Value::Length(start, Digits::Shortest)),
                ));
            } else if start >= end {
                problems.push(Problem::at(
                    format!("operations.{i}.start_depth"),
                    Message::new("job-start-below-end")
                        .arg("end", Value::Length(end, Digits::Shortest))
                        .arg("got", Value::Length(start, Digits::Shortest)),
                ));
            }
            if !op.clearance().is_finite() {
                problems.push(Problem::at(
                    format!("operations.{i}.clearance"),
                    Message::new("job-must-be-number")
                        .arg("got", Value::LengthNumber(op.clearance(), Digits::Shortest)),
                ));
            }
            if let Operation::Surface(f) = op {
                problems.extend(surface_problems(i, f, self.stock.thickness));
            }
            if let Operation::Profile(p) = op
                && p.side == Side::On
                && p.clearance != 0.0
            {
                problems.push(Problem::at(
                    format!("operations.{i}.clearance"),
                    Message::new("job-on-line-clearance")
                        .arg("got", Value::Length(p.clearance, Digits::Shortest)),
                ));
            }
        }
        for (i, op) in self.operations.iter().enumerate() {
            if !op.selects_shapes() {
                continue;
            }
            let selector = op.select();
            if selector.is_empty() {
                problems.push(Problem::at(
                    format!("operations.{i}.select"),
                    Message::new("job-selector-empty"),
                ));
            }
            if let Some(dxf) = selector.dxf
                && !self.drawing.dxfs.iter().any(|d| d.id == dxf)
            {
                problems.push(Problem::at(
                    format!("operations.{i}.select.dxf"),
                    Message::new("job-no-dxf").arg("id", dxf),
                ));
            }
            if let Some(image) = selector.image
                && !self.drawing.images.iter().any(|d| d.id == image)
            {
                problems.push(Problem::at(
                    format!("operations.{i}.select.image"),
                    Message::new("job-no-image").arg("id", image),
                ));
            }
            if let Some(text) = selector.text
                && !self.drawing.texts.iter().any(|t| t.id == text)
            {
                problems.push(Problem::at(
                    format!("operations.{i}.select.text"),
                    Message::new("job-no-text").arg("id", text),
                ));
            }
            // Shapes left out of a whole DXF, image or text (docs/specs/leave-out-shapes.md
            // §2, text.md §3).
            if let Some(exclude) = selector.exclude.as_ref().filter(|e| !e.is_empty()) {
                let source = match (selector.dxf, selector.image, selector.text) {
                    (Some(dxf), _, _) => Some(("dxf", dxf, 0)),
                    (None, Some(image), _) => Some(("image", image, 1)),
                    (None, None, Some(text)) => Some(("text", text, 2)),
                    (None, None, None) => None,
                };
                match source {
                    None => problems.push(Problem::at(
                        format!("operations.{i}.select.exclude"),
                        Message::new("job-exclude-no-source"),
                    )),
                    Some((what, id, kind)) => {
                        let of_source = |shape: &Shape| match kind {
                            0 => shape.dxf() == Some(id),
                            1 => shape.image() == Some(id),
                            _ => shape.text() == Some(id),
                        };
                        if let Some(missing) = exclude.iter().find(|e| {
                            !self
                                .drawing
                                .shapes
                                .iter()
                                .any(|s| s.id == **e && of_source(s))
                        }) {
                            problems.push(Problem::at(
                                format!("operations.{i}.select.exclude"),
                                Message::new("job-exclude-missing")
                                    .arg("shape", *missing)
                                    .arg("source", what)
                                    .arg("id", id),
                            ));
                        }
                    }
                }
            }
            if [selector.dxf, selector.image, selector.text]
                .iter()
                .flatten()
                .count()
                > 1
            {
                problems.push(Problem::at(
                    format!("operations.{i}.select"),
                    Message::new("job-selector-one-source"),
                ));
            }
            // A text's letters get new ids each time it is made again (text.md §3).
            let letter = selector.shapes.iter().flatten().find_map(|id| {
                self.drawing
                    .shapes
                    .iter()
                    .find(|s| s.id == *id && s.text().is_some())
            });
            if let Some(shape) = letter {
                problems.push(Problem::at(
                    format!("operations.{i}.select"),
                    Message::new("job-select-letter")
                        .arg("shape", shape.id)
                        .arg("text", shape.text().unwrap_or_default()),
                ));
            }
            // Traced shapes get new ids each time their image is traced again
            // (docs/specs/image-trace.md §4, decision 6).
            let traced = selector.shapes.iter().flatten().find_map(|id| {
                self.drawing
                    .shapes
                    .iter()
                    .find(|s| s.id == *id && s.image().is_some())
            });
            if let Some(shape) = traced {
                problems.push(Problem::at(
                    format!("operations.{i}.select"),
                    Message::new("job-select-traced")
                        .arg("shape", shape.id)
                        .arg("image", shape.image().unwrap_or_default()),
                ));
            }
            // Open centre lines are paths to follow, not areas
            // (docs/specs/image-arcs-centrelines.md §5). A closed one, such as an outline
            // drawn by hand, is an area like any other: refused only when the picture has
            // no closed line at all (user report 2026-10-13).
            let centre_lines = self.drawing.images.iter().find(|image| {
                image.trace == ImageTrace::CentreLine
                    && (selector.image == Some(image.id)
                        || selector.dxf.is_none()
                            && selector.layer.as_ref().is_some_and(|layer| {
                                image.layer.to_lowercase() == layer.to_lowercase()
                            }))
                    && !self
                        .drawing
                        .shapes
                        .iter()
                        .any(|s| s.image() == Some(image.id) && s.closed())
            });
            if let Some(image) = centre_lines
                && !matches!(op, Operation::Path(_))
            {
                problems.push(Problem::at(
                    format!("operations.{i}.select"),
                    Message::new("job-select-centre-lines").arg("image", image.id),
                ));
            }
            if let Entry::Helix { radius, .. } = op.entry()
                && !(radius.is_finite() && radius > 0.0)
            {
                problems.push(Problem::at(
                    format!("operations.{i}.entry.radius"),
                    Message::new("job-must-be-more-than-0")
                        .arg("got", Value::Length(radius, Digits::Shortest)),
                ));
            }
            if let Some(ramp_angle) = op.entry().ramp_angle()
                && !(ramp_angle.is_finite() && ramp_angle > 0.0 && ramp_angle <= MAX_RAMP_ANGLE_RAD)
            {
                // A profile's angle is its own field; the others' sit in their entry.
                let field = match op {
                    Operation::Profile(_) => "ramp_angle",
                    _ => "entry.ramp_angle",
                };
                problems.push(Problem::at(
                    format!("operations.{i}.{field}"),
                    Message::new("job-ramp-angle-range").arg(
                        "got",
                        Value::Number(ramp_angle.to_degrees(), Digits::Fixed(3)),
                    ),
                ));
            }
            if let Some(corners) = op.corners() {
                let angle = corners.max_angle();
                if !(angle.is_finite() && angle > 0.0 && angle < std::f64::consts::PI) {
                    problems.push(Problem::at(
                        format!("operations.{i}.corners.max_angle"),
                        Message::new("job-angle-below-180")
                            .arg("got", Value::Number(angle.to_degrees(), Digits::Fixed(3))),
                    ));
                }
                if let Operation::Profile(p) = op
                    && p.side == Side::On
                {
                    problems.push(Problem::at(
                        format!("operations.{i}.corners"),
                        Message::new("job-on-line-corners"),
                    ));
                }
            }
            problems.extend(corner_choice_problems(i, op));
            if let Operation::Holes(HolesOp {
                method: HoleMethod::Bore { ramp_angle, .. },
                ..
            }) = op
                && !(ramp_angle.is_finite()
                    && *ramp_angle > 0.0
                    && *ramp_angle <= MAX_RAMP_ANGLE_RAD)
            {
                problems.push(Problem::at(
                    format!("operations.{i}.method.ramp_angle"),
                    Message::new("job-ramp-angle-range").arg(
                        "got",
                        Value::Number(ramp_angle.to_degrees(), Digits::Fixed(3)),
                    ),
                ));
            }
            if let Some(lead) = op.lead()
                && !(lead.radius.is_finite() && lead.radius > 0.0)
            {
                problems.push(Problem::at(
                    format!("operations.{i}.lead.radius"),
                    Message::new("job-must-be-more-than-0")
                        .arg("got", Value::Length(lead.radius, Digits::Shortest)),
                ));
            }
            if op
                .starts()
                .iter()
                .any(|p| !(p[0].is_finite() && p[1].is_finite()))
            {
                problems.push(Problem::at(
                    format!("operations.{i}.starts"),
                    Message::new("job-starts-not-finite"),
                ));
            }
            if let Operation::Pocket(p) = op
                && p.stepover > 1.0
            {
                problems.push(Problem::at(
                    format!("operations.{i}.stepover"),
                    Message::new("job-stepover-at-most-1")
                        .arg("got", Value::Number(p.stepover, Digits::Shortest)),
                ));
            }
            if let Operation::Pocket(p) = op {
                for (k, pass) in p.rest_passes.iter().enumerate() {
                    if pass.stepover > 1.0 {
                        problems.push(Problem::at(
                            format!("operations.{i}.rest_passes.{k}.stepover"),
                            Message::new("job-stepover-at-most-1")
                                .arg("got", Value::Number(pass.stepover, Digits::Shortest)),
                        ));
                    }
                }
            }
            if let Operation::Profile(ProfileOp {
                tabs: Some(tabs), ..
            }) = op
                && tabs.count < 1
            {
                problems.push(Problem::at(
                    format!("operations.{i}.tabs.count"),
                    Message::new("job-must-be-at-least-1"),
                ));
            }
            if let Operation::Profile(ProfileOp {
                tabs: Some(tabs), ..
            }) = op
                && !(tabs.flank_angle.is_finite()
                    && tabs.flank_angle > 0.0
                    && tabs.flank_angle <= STRAIGHT_FLANK_RAD + ANGLE_TOL_RAD)
            {
                problems.push(Problem::at(
                    format!("operations.{i}.tabs.flank_angle"),
                    Message::new("job-flank-angle-range").arg(
                        "got",
                        Value::Number(tabs.flank_angle.to_degrees(), Digits::Fixed(3)),
                    ),
                ));
            }
            if let Operation::Profile(p) = op {
                problems.extend(finish_skin_problems(p, &format!("operations.{i}")));
            }
        }
        problems
    }
}

/// The rules for a profile's finishing pass and skin that need no tool
/// (docs/specs/finish-and-skin.md §1, §6); the tool radius is checked when planning.
fn finish_skin_problems(p: &ProfileOp, at: &str) -> Vec<Problem> {
    let mut problems = Vec::new();
    if p.finish.is_some() && p.side == Side::On {
        problems.push(Problem::at(
            format!("{at}.finish"),
            Message::new("job-on-line-finish"),
        ));
    }
    let Some(skin) = p.skin else {
        return problems;
    };
    let t = skin.thickness;
    if !(t.is_finite() && t > 0.0) {
        return problems; // reported as not positive
    }
    if t > p.step_down + DEPTH_TOL_MM {
        problems.push(Problem::at(
            format!("{at}.skin.thickness"),
            Message::new("job-skin-above-step-down")
                .arg(
                    "step_down",
                    Value::LengthNumber(p.step_down, Digits::Shortest),
                )
                .arg("got", Value::LengthNumber(t, Digits::Shortest)),
        ));
    }
    let cut = p.end_depth - p.start_depth;
    if t >= cut {
        problems.push(Problem::at(
            format!("{at}.skin.thickness"),
            Message::new("job-skin-above-cut")
                .arg("cut", Value::LengthNumber(cut, Digits::Shortest))
                .arg("got", Value::LengthNumber(t, Digits::Shortest)),
        ));
    }
    if let Some(tabs) = &p.tabs
        && t >= tabs.height
    {
        problems.push(Problem::at(
            format!("{at}.skin.thickness"),
            Message::new("job-skin-above-tabs")
                .arg("height", Value::LengthNumber(tabs.height, Digits::Shortest))
                .arg("got", Value::LengthNumber(t, Digits::Shortest)),
        ));
    }
    problems
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shape(geometry: Geometry) -> Shape {
        Shape {
            construction: false,
            id: 1,
            layer: "0".into(),
            name: None,
            geometry,
        }
    }

    fn line_starts(shape: &Shape) -> Vec<[f64; 2]> {
        shape
            .segments()
            .iter()
            .map(|s| match s {
                PathSegment::Line(l) => l.start,
                PathSegment::Arc(_) => panic!("expected lines"),
            })
            .collect()
    }

    fn close(a: [f64; 2], b: [f64; 2]) -> bool {
        (a[0] - b[0]).abs() < 1e-12 && (a[1] - b[1]).abs() < 1e-12
    }

    #[test]
    fn a_circle_is_one_full_arc_from_angle_zero() {
        let c = shape(Geometry::Circle {
            center: [5.0, 6.0],
            radius: 2.0,
        });
        assert_eq!(
            c.segments(),
            [PathSegment::Arc(ArcSegment {
                center: [5.0, 6.0],
                radius: 2.0,
                start_angle: 0.0,
                sweep: std::f64::consts::TAU,
            })]
        );
        assert!(c.closed() && c.is_drawn() && c.dxf_handle().is_none());
    }

    #[test]
    fn rectangle_corners_run_counter_clockwise_and_rotate_about_the_centre() {
        let rect = |angle: f64| {
            shape(Geometry::Rectangle {
                center: [10.0, 20.0],
                length: 8.0,
                width: 4.0,
                angle,
                corner_radius: 0.0,
            })
        };
        let flat = line_starts(&rect(0.0));
        assert_eq!(flat, [[6.0, 18.0], [14.0, 18.0], [14.0, 22.0], [6.0, 22.0]]);
        // 90°: length now runs along Y.
        let up = line_starts(&rect(std::f64::consts::FRAC_PI_2));
        let expected = [[12.0, 16.0], [12.0, 24.0], [8.0, 24.0], [8.0, 16.0]];
        assert!(
            up.iter().zip(&expected).all(|(a, b)| close(*a, *b)),
            "{up:?}"
        );
        // 30°: the first corner is (-4, -2) rotated by 30° about the centre.
        let (sin, cos) = 30f64.to_radians().sin_cos();
        let first = line_starts(&rect(30f64.to_radians()))[0];
        assert!(close(
            first,
            [10.0 - 4.0 * cos + 2.0 * sin, 20.0 - 4.0 * sin - 2.0 * cos]
        ));
        // Closed: the last line ends at the first corner.
        let segments = rect(0.3).segments();
        let (PathSegment::Line(first), PathSegment::Line(last)) = (&segments[0], &segments[3])
        else {
            panic!("lines");
        };
        assert_eq!(last.end, first.start);
    }

    fn rounded(length: f64, width: f64, angle: f64, corner_radius: f64) -> Geometry {
        Geometry::Rectangle {
            center: [10.0, 20.0],
            length,
            width,
            angle,
            corner_radius,
        }
    }

    /// Start and end of a segment.
    fn ends(segment: &PathSegment) -> ([f64; 2], [f64; 2]) {
        match segment {
            PathSegment::Line(l) => (l.start, l.end),
            PathSegment::Arc(a) => {
                let at = |t: f64| {
                    [
                        a.center[0] + a.radius * t.cos(),
                        a.center[1] + a.radius * t.sin(),
                    ]
                };
                (at(a.start_angle), at(a.start_angle + a.sweep))
            }
        }
    }

    fn joined(segments: &[PathSegment]) -> bool {
        (0..segments.len()).all(|i| {
            let end = ends(&segments[i]).1;
            let next = ends(&segments[(i + 1) % segments.len()]).0;
            (end[0] - next[0]).hypot(end[1] - next[1]) <= CHAIN_TOL_MM
        })
    }

    /// docs/specs/rectangle-corner-radius.md §2.
    #[test]
    fn rounded_corners_are_quarter_arcs_joined_to_the_sides() {
        let segments = shape(rounded(8.0, 4.0, 0.0, 1.0)).segments();
        assert_eq!(segments.len(), 8);
        assert!(joined(&segments));
        // Starts r right of the lower-left corner, along the bottom.
        assert_eq!(ends(&segments[0]), ([7.0, 18.0], [13.0, 18.0]));
        for arc in segments.iter().skip(1).step_by(2) {
            let PathSegment::Arc(arc) = arc else {
                panic!("arc");
            };
            assert_eq!(arc.radius, 1.0);
            assert_eq!(arc.sweep, std::f64::consts::FRAC_PI_2);
        }
        // The same box as the sharp rectangle.
        let xs: Vec<f64> = segments
            .iter()
            .flat_map(|s| [ends(s).0[0], ends(s).1[0]])
            .collect();
        let lowest = xs.iter().copied().fold(f64::INFINITY, f64::min);
        let highest = xs.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        assert!(close([lowest, highest], [6.0, 14.0]));
        // Turned: the unturned segments turned about the centre.
        let angle = 0.4_f64;
        let (sin, cos) = angle.sin_cos();
        let turn = |p: [f64; 2]| {
            let (x, y) = (p[0] - 10.0, p[1] - 20.0);
            [10.0 + x * cos - y * sin, 20.0 + x * sin + y * cos]
        };
        let turned = shape(rounded(8.0, 4.0, angle, 1.0)).segments();
        assert!(joined(&turned));
        for (a, b) in segments.iter().zip(&turned) {
            assert!(close(turn(ends(a).0), ends(b).0) && close(turn(ends(a).1), ends(b).1));
        }
        // Radius 0: the four lines as before.
        assert_eq!(
            shape(rounded(8.0, 4.0, 0.3, 0.0)).segments(),
            shape(Geometry::Rectangle {
                center: [10.0, 20.0],
                length: 8.0,
                width: 4.0,
                angle: 0.3,
                corner_radius: 0.0,
            })
            .segments()
        );
        assert_eq!(shape(rounded(8.0, 4.0, 0.3, 0.0)).segments().len(), 4);
    }

    #[test]
    fn a_radius_of_half_a_side_leaves_that_side_out() {
        // A slot: the short sides are all arc.
        let slot = shape(rounded(8.0, 4.0, 0.0, 2.0)).segments();
        assert_eq!(slot.len(), 6);
        assert!(joined(&slot));
        // A square at half its side: a circle of four arcs.
        let round = shape(rounded(4.0, 4.0, 0.7, 2.0)).segments();
        assert_eq!(round.len(), 4);
        assert!(
            round
                .iter()
                .all(|s| matches!(s, PathSegment::Arc(a) if a.center == [10.0, 20.0]))
        );
        assert!(joined(&round));
    }

    #[test]
    fn a_corner_radius_is_at_most_half_the_shorter_side() {
        let ok = |r: f64| shape_problems(&rounded(8.0, 4.0, 0.0, r)).is_empty();
        assert!(ok(0.0) && ok(1.0) && ok(2.0) && ok(CHAIN_TOL_MM));
        for bad in [2.0001, -1.0, f64::NAN, f64::INFINITY, CHAIN_TOL_MM / 2.0] {
            assert!(!ok(bad), "{bad}");
        }
        let problems = shape_problems(&rounded(8.0, 4.0, 0.0, 3.0));
        assert!(problems[0].text.contains("at most 2 mm"), "{problems:?}");
    }

    #[test]
    fn polylines_close_only_when_asked() {
        let points = vec![[0.0, 0.0], [4.0, 0.0], [2.0, 3.0]];
        let open = shape(Geometry::Polyline {
            closed: false,
            points: points.clone(),
            bulges: vec![0.0; 2],
        });
        let closed = shape(Geometry::Polyline {
            closed: true,
            points,
            bulges: vec![0.0; 3],
        });
        assert_eq!(open.segments().len(), 2);
        assert_eq!(closed.segments().len(), 3);
        assert!(!open.closed() && closed.closed());
    }

    #[test]
    fn a_bulged_segment_is_the_arc_a_dxf_bulge_describes() {
        let bulged = shape(Geometry::Polyline {
            closed: true,
            points: vec![[0.0, 0.0], [40.0, 0.0], [20.0, 30.0]],
            bulges: vec![0.0, 0.4, 0.0],
        });
        let segments = bulged.segments();
        assert!(matches!(segments[0], PathSegment::Line(_)));
        assert!(matches!(segments[2], PathSegment::Line(_)));
        let PathSegment::Arc(arc) = segments[1] else {
            panic!("arc");
        };
        let expected = Arc::from_bulge((40.0, 0.0), (20.0, 30.0), 0.4).unwrap();
        assert_eq!(arc.center, [expected.center.0, expected.center.1]);
        assert_eq!(arc.radius, expected.radius);
        assert_eq!(arc.start_angle, expected.start_angle);
        assert_eq!(arc.sweep, 4.0 * 0.4f64.atan());
        // It ends on the next point.
        let end_angle = arc.start_angle + arc.sweep;
        let end = [
            arc.center[0] + arc.radius * end_angle.cos(),
            arc.center[1] + arc.radius * end_angle.sin(),
        ];
        assert!((end[0] - 20.0).abs() < 1e-9 && (end[1] - 30.0).abs() < 1e-9);
    }
}
