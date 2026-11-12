// SPDX-License-Identifier: GPL-3.0-or-later
//! What an operation cuts: a selector, and the whole sources it can name.

use super::*;

/// A whole drawing source an operation can select: a DXF (or SVG), a traced picture, a
/// text or a group of drawn shapes and texts.
// Spec: docs/specs/source-model.md §2.2.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Source {
    /// An id in `drawing.dxfs`.
    Dxf(u64),
    /// An id in `drawing.images`.
    Image(u64),
    /// An id in `drawing.texts`.
    Text(u64),
    /// An id in `drawing.groups`.
    Group(u64),
    /// An id in `drawing.models`: its footprint outline (docs/specs/stl-relief.md §4).
    Model(u64),
}

impl Source {
    pub fn id(self) -> u64 {
        match self {
            Source::Dxf(id)
            | Source::Image(id)
            | Source::Text(id)
            | Source::Group(id)
            | Source::Model(id) => id,
        }
    }

    /// Its key in a selector, as in the file: "dxf", "image", "text" or "group".
    pub fn key(self) -> &'static str {
        match self {
            Source::Dxf(_) => "dxf",
            Source::Image(_) => "image",
            Source::Text(_) => "text",
            Source::Group(_) => "group",
            Source::Model(_) => "model",
        }
    }

    /// What segments carry: where they were drawn from. A group isn't that; it is resolved
    /// to its shapes before cutting.
    pub fn shape_from(self) -> Option<ShapeFrom> {
        match self {
            Source::Dxf(id) => Some(ShapeFrom::Dxf(id)),
            Source::Image(id) => Some(ShapeFrom::Image(id)),
            Source::Text(id) => Some(ShapeFrom::Text(id)),
            Source::Model(id) => Some(ShapeFrom::Model(id)),
            Source::Group(_) => None,
        }
    }
}

impl From<ShapeFrom> for Source {
    fn from(from: ShapeFrom) -> Self {
        match from {
            ShapeFrom::Dxf(id) => Source::Dxf(id),
            ShapeFrom::Image(id) => Source::Image(id),
            ShapeFrom::Text(id) => Source::Text(id),
            ShapeFrom::Model(id) => Source::Model(id),
        }
    }
}

// What an operation cuts: a layer, shapes by id, a whole source, or a layer or shapes of
// one source; with a source, shapes of it left out. Written flat in the file
// (`{"dxf": 2, "layer": "HOLES"}`), through `RawSelector`; a plain comment, so the JSON
// Schema stays as it was.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(try_from = "RawSelector", into = "RawSelector")]
pub struct Selector {
    /// Every shape on this layer (upper or lower case don't matter).
    pub layer: Option<String>,
    /// These shapes, by id.
    pub shapes: Option<Vec<u64>>,
    /// Only shapes of this DXF, image, text or group; all of it with no layer or shapes.
    // Spec: docs/specs/drawing-sources.md §2, source-model.md §2.2.
    pub source: Option<Source>,
    /// Shapes of the source left out (ids); empty with no source. A contour with any of
    /// them isn't cut. A traced image's, or a text's, are found again when it is made again.
    // Spec: docs/specs/leave-out-shapes.md.
    pub exclude: Vec<u64>,
}

// A selector as it appears in the file: a key per kind of source, at most one of them
// (a plain comment: the JSON Schema has no description here).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct RawSelector {
    /// Every shape on this layer (upper or lower case don't matter).
    #[serde(default)]
    pub layer: Option<String>,
    /// These shapes, by id.
    #[serde(default)]
    pub shapes: Option<Vec<u64>>,
    /// Only shapes from this DXF (an id in `drawing.dxfs`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dxf: Option<u64>,
    /// Only shapes traced from this image (an id in `drawing.images`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image: Option<u64>,
    /// Only the letters of this text (an id in `drawing.texts`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<u64>,
    /// The shapes and texts' letters of this group (an id in `drawing.groups`), as they
    /// are at each run.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<u64>,
    /// The footprint outline of this 3D model (an id in `drawing.models`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<u64>,
    /// Shapes of that DXF, image, text or group left out (ids); with `dxf`, `image`,
    /// `text` or `group` only. A contour with any of them isn't cut. A traced image's, or a text's, are found
    /// again when it is made again.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exclude: Option<Vec<u64>>,
}

impl TryFrom<RawSelector> for Selector {
    type Error = String;

    fn try_from(raw: RawSelector) -> Result<Self, String> {
        let sources: Vec<Source> = [
            raw.dxf.map(Source::Dxf),
            raw.image.map(Source::Image),
            raw.text.map(Source::Text),
            raw.group.map(Source::Group),
            raw.model.map(Source::Model),
        ]
        .into_iter()
        .flatten()
        .collect();
        // Refused where the selector is read, in the words `validate` used before
        // (docs/specs/source-model.md §2.2); a parse error is English, as for shapes.
        if sources.len() > 1 {
            return Err(Message::new("job-selector-one-source").english());
        }
        let source = sources.first().copied();
        let exclude = raw.exclude.unwrap_or_default();
        if source.is_none() && !exclude.is_empty() {
            return Err(Message::new("job-exclude-no-source").english());
        }
        Ok(Selector {
            layer: raw.layer,
            shapes: raw.shapes,
            source,
            exclude,
        })
    }
}

impl From<Selector> for RawSelector {
    fn from(selector: Selector) -> Self {
        let of = |want: fn(u64) -> Source| {
            selector
                .source
                .filter(|s| *s == want(s.id()))
                .map(Source::id)
        };
        RawSelector {
            dxf: of(Source::Dxf),
            image: of(Source::Image),
            text: of(Source::Text),
            group: of(Source::Group),
            model: of(Source::Model),
            exclude: (!selector.exclude.is_empty()).then_some(selector.exclude),
            layer: selector.layer,
            shapes: selector.shapes,
        }
    }
}

impl Selector {
    /// All of `source`.
    pub fn of(source: Source) -> Selector {
        Selector {
            source: Some(source),
            ..Selector::default()
        }
    }

    /// Every shape on `layer`.
    pub fn on_layer(layer: &str) -> Selector {
        Selector {
            layer: Some(layer.to_string()),
            ..Selector::default()
        }
    }

    /// The DXF it names.
    pub fn dxf(&self) -> Option<u64> {
        match self.source {
            Some(Source::Dxf(id)) => Some(id),
            _ => None,
        }
    }

    /// The image it names.
    pub fn image(&self) -> Option<u64> {
        match self.source {
            Some(Source::Image(id)) => Some(id),
            _ => None,
        }
    }

    /// The text it names.
    pub fn text(&self) -> Option<u64> {
        match self.source {
            Some(Source::Text(id)) => Some(id),
            _ => None,
        }
    }

    /// The group it names.
    pub fn group(&self) -> Option<u64> {
        match self.source {
            Some(Source::Group(id)) => Some(id),
            _ => None,
        }
    }

    /// Whether it picks nothing at all: no source, layer or shapes. A selector that names
    /// only a source picks all of it (docs/specs/drawing-sources.md §2,
    /// leave-out-shapes.md §2).
    pub fn is_empty(&self) -> bool {
        self.layer.is_none()
            && self.source.is_none()
            && self.shapes.as_ref().is_none_or(Vec::is_empty)
    }

    /// The selector with a group resolved to the shapes it holds now (its texts' letters
    /// included); others as they are.
    pub fn resolved(&self, drawing: &Drawing) -> Selector {
        match self.source {
            Some(Source::Group(id)) => Selector {
                shapes: Some(drawing.group_shapes(id)),
                source: None,
                ..self.clone()
            },
            _ => self.clone(),
        }
    }

    /// Whether it could pick `shape`: on its layer or among its shapes, and from its DXF,
    /// image or text when it names one. For where a setup's board goes
    /// (docs/specs/setups.md §3), not for cutting: planning selects contours exactly.
    pub fn may_pick(&self, shape: &Shape) -> bool {
        if let Some(from) = self.source.and_then(Source::shape_from)
            && shape.source() != Some(from)
        {
            return false;
        }
        let on_layer = self
            .layer
            .as_ref()
            .is_some_and(|l| l.eq_ignore_ascii_case(&shape.layer));
        let named = self
            .shapes
            .as_ref()
            .is_some_and(|ids| ids.contains(&shape.id));
        let whole_source = self.layer.is_none() && self.shapes.is_none() && self.source.is_some();
        on_layer || named || whole_source
    }
}
