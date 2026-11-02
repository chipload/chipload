// SPDX-License-Identifier: GPL-3.0-or-later
//! The drawing: its shapes, groups, DXF and SVG sources and their placement.

use super::*;

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
    /// Groups of drawn shapes and texts, selected, moved and cut as one. Left out of the
    /// file when there are none, so projects without groups open in older versions.
    // Spec: docs/specs/shape-groups.md §4.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub groups: Vec<ShapeGroup>,
    /// Groups and layers not drawn on the canvas (they are still cut): `dxf:1`,
    /// `dxf:1/layer:Cut`, `image:1`, `text:1`, `drawn:circle`, `drawn:rectangle`,
    /// `drawn:polyline`.
    // Spec: docs/specs/drawing-sources.md §4.
    #[serde(default)]
    pub hidden: Vec<String>,
}

/// Drawn shapes and texts kept together: one click selects them all, and they move, turn,
/// scale and are cut as one (docs/specs/shape-groups.md). A shape or text is in at most one
/// group; groups don't hold groups.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct ShapeGroup {
    /// The group's id, which operations' selectors name.
    pub id: u64,
    /// Its name in the Browser.
    pub name: String,
    /// Its drawn shapes, by id.
    #[serde(default)]
    pub shapes: Vec<u64>,
    /// Its texts, by id, with their letters.
    #[serde(default)]
    pub texts: Vec<u64>,
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

impl Drawing {
    /// Group `id`'s shapes: its drawn shapes and its texts' letters, by id, as they are
    /// now. Ids no longer in the drawing are left out. Empty for no such group.
    pub fn group_shapes(&self, id: u64) -> Vec<u64> {
        let Some(group) = self.groups.iter().find(|g| g.id == id) else {
            return Vec::new();
        };
        self.shapes
            .iter()
            .filter(|s| {
                group.shapes.contains(&s.id) || s.text().is_some_and(|t| group.texts.contains(&t))
            })
            .map(|s| s.id)
            .collect()
    }

    /// The group shape `id` is in: as a drawn shape, or as a letter of a grouped text.
    pub fn group_of(&self, id: u64) -> Option<u64> {
        let text = self
            .shapes
            .iter()
            .find(|s| s.id == id)
            .and_then(Shape::text);
        self.groups
            .iter()
            .find(|g| g.shapes.contains(&id) || text.is_some_and(|t| g.texts.contains(&t)))
            .map(|g| g.id)
    }

    /// The group text `id` is in.
    pub fn group_of_text(&self, id: u64) -> Option<u64> {
        self.groups
            .iter()
            .find(|g| g.texts.contains(&id))
            .map(|g| g.id)
    }

    /// Take shapes and texts that are gone out of their groups, and groups left with
    /// nothing out of the drawing (docs/specs/shape-groups.md §4). Every edit that
    /// removes shapes or texts calls it.
    pub fn tidy_groups(&mut self) {
        let shapes: std::collections::BTreeSet<u64> = self.shapes.iter().map(|s| s.id).collect();
        let texts: std::collections::BTreeSet<u64> = self.texts.iter().map(|t| t.id).collect();
        for group in &mut self.groups {
            group.shapes.retain(|id| shapes.contains(id));
            group.texts.retain(|id| texts.contains(id));
        }
        self.groups
            .retain(|g| !(g.shapes.is_empty() && g.texts.is_empty()));
    }

    /// What is wrong with the groups (docs/specs/shape-groups.md §4): ids twice, no name,
    /// nothing in a group, a member that isn't drawn, one in two groups. Members that are
    /// gone are not a problem: [`Drawing::tidy_groups`] takes them out.
    fn group_problems(&self) -> Vec<Problem> {
        let mut problems = Vec::new();
        let mut ids = std::collections::BTreeSet::new();
        let mut members = std::collections::BTreeSet::new();
        let mut texts = std::collections::BTreeSet::new();
        for (i, group) in self.groups.iter().enumerate() {
            let at = format!("drawing.groups.{i}");
            if !ids.insert(group.id) {
                problems.push(Problem::at(
                    format!("{at}.id"),
                    Message::new("job-id-twice").arg("id", group.id),
                ));
            }
            if group.name.trim().is_empty() {
                problems.push(Problem::at(
                    format!("{at}.name"),
                    Message::new("job-must-not-be-empty"),
                ));
            }
            if group.shapes.is_empty() && group.texts.is_empty() {
                problems.push(Problem::at(at.clone(), Message::new("job-group-empty")));
            }
            for id in &group.shapes {
                if self.shapes.iter().any(|s| s.id == *id && !s.is_drawn()) {
                    problems.push(Problem::at(
                        format!("{at}.shapes"),
                        Message::new("job-group-not-drawn").arg("shape", *id),
                    ));
                }
                if !members.insert(*id) {
                    problems.push(Problem::at(
                        format!("{at}.shapes"),
                        Message::new("job-group-shape-twice").arg("shape", *id),
                    ));
                }
            }
            for id in &group.texts {
                if !texts.insert(*id) {
                    problems.push(Problem::at(
                        format!("{at}.texts"),
                        Message::new("job-group-text-twice").arg("text", *id),
                    ));
                }
            }
        }
        problems
    }

    /// The box round the shapes that are cut (construction geometry left out), arcs
    /// included, mm: ((x0, y0), (x1, y1)). None for an empty drawing.
    pub fn extents(&self) -> Option<((f64, f64), (f64, f64))> {
        self.extents_of(|_| true)
    }

    /// [`Drawing::extents`] of the shapes `keep` accepts.
    pub fn extents_of(&self, keep: impl Fn(&Shape) -> bool) -> Option<((f64, f64), (f64, f64))> {
        self.shapes
            .iter()
            .filter(|s| !s.construction && keep(s))
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
        problems.extend(self.group_problems());
        problems
    }
}
