// SPDX-License-Identifier: GPL-3.0-or-later
//! Pictures traced into shapes and texts written in a font.

use super::*;

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

/// A 3D model (an STL file) carved as a relief from the top: the file itself, which way is
/// up, and where it lies on the board. Its footprint outline is a `path` shape on its layer
/// that names it in `model`.
// Spec: docs/specs/stl-relief.md §2, §3.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Model {
    /// The model's id, which its outline and reliefs name.
    pub id: u64,
    /// Its name in the Browser and messages: the file's name without its extension at
    /// first.
    pub name: String,
    /// The STL file's name.
    pub file: String,
    /// The STL file's SHA-256.
    pub sha256: String,
    /// The STL file itself, base64-encoded.
    #[cfg_attr(feature = "schema", schemars(with = "String"))]
    pub data: ModelData,
    /// The unit the file was read in, for its size on import only: the model is placed by
    /// `width` (decision 2).
    pub units: Units,
    /// Which of the model's axes points up on the board.
    pub up: ModelUp,
    /// The layer its footprint outline goes on.
    pub layer: String,
    /// X of the footprint's lower-left corner on the board, mm, before the turn.
    pub x: f64,
    /// Y of the footprint's lower-left corner on the board, mm, before the turn.
    pub y: f64,
    /// The footprint's width on the board, mm: it scales X and Y alike.
    pub width: f64,
    /// Turned counter-clockwise about the footprint's centre, degrees, at least 0 and less
    /// than 360.
    pub rotation: f64,
    /// How deep the relief goes, from the model's top to its lowest point, mm.
    pub depth: f64,
    /// The depth follows the width, in the model's own proportion: made again whenever the
    /// model changes (decision 4).
    pub proportional: bool,
    /// The share of the turned model's height kept, from its top, more than 0 and at most
    /// 1: what lies below is cut away, its back (docs/specs/relief-wizard.md §5). Width,
    /// depth and the outline are the kept part's. 1 (the whole model) when missing.
    #[serde(default = "whole_model")]
    pub keep: f64,
    /// Missing: the model is an STL file. Present: `data` is a picture (PNG, JPEG or BMP)
    /// read as a depth map with these settings; `up` is `z` and `proportional` false.
    // Spec: docs/specs/picture-relief.md §2, §3 (optional).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub picture: Option<PictureDepth>,
}

/// How a picture model's picture is read as heights (docs/specs/picture-relief.md §3).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct PictureDepth {
    /// White is the top, black the bottom; false: the other way round.
    pub white_high: bool,
    /// A blur over the heights, its radius on the board, mm: 0 to 5.
    pub smoothing: f64,
    /// Grey shares up to this, 0 to 0.95, are background, not the model; 0: none.
    pub background: f64,
    /// The heights raised to this power, 0.2 to 5: above 1 flatter low ground and steeper
    /// tops.
    pub gamma: f64,
}

/// A new picture model's settings (docs/specs/picture-relief.md §3).
pub const DEFAULT_PICTURE_DEPTH: PictureDepth = PictureDepth {
    white_high: true,
    smoothing: 0.2,
    background: 0.0,
    gamma: 1.0,
};

/// A picture model's smoothing, background and gamma ranges (the same as
/// `chipload-image`'s).
pub const PICTURE_SMOOTHING_RANGE: (f64, f64) = (0.0, 5.0);
pub const PICTURE_BACKGROUND_RANGE: (f64, f64) = (0.0, 0.95);
pub const PICTURE_GAMMA_RANGE: (f64, f64) = (0.2, 5.0);

/// A model's `keep` when the file has none: all of it.
fn whole_model() -> f64 {
    1.0
}

/// Which of a model's axes points up on the board.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum ModelUp {
    #[default]
    #[serde(rename = "z")]
    Z,
    #[serde(rename = "y")]
    Y,
    #[serde(rename = "x")]
    X,
    #[serde(rename = "-z")]
    MinusZ,
    #[serde(rename = "-y")]
    MinusY,
    #[serde(rename = "-x")]
    MinusX,
}

impl ModelUp {
    /// As written in the file and on the command line: `z`, `-y` and so on.
    pub fn name(self) -> &'static str {
        match self {
            ModelUp::Z => "z",
            ModelUp::Y => "y",
            ModelUp::X => "x",
            ModelUp::MinusZ => "-z",
            ModelUp::MinusY => "-y",
            ModelUp::MinusX => "-x",
        }
    }
}

impl Model {
    /// What is wrong with the model's values, as "drawing.models.N.field: message".
    pub fn problems(&self, index: usize) -> Vec<Problem> {
        let at = format!("drawing.models.{index}");
        let mut problems = Vec::new();
        let positive = |field: &str, value: f64, problems: &mut Vec<Problem>| {
            if !(value.is_finite() && value > 0.0) {
                problems.push(Problem::at(
                    format!("{at}.{field}"),
                    Message::new("job-must-be-positive")
                        .arg("got", Value::Length(value, Digits::Shortest)),
                ));
            }
        };
        positive("width", self.width, &mut problems);
        positive("depth", self.depth, &mut problems);
        if !(self.keep.is_finite() && self.keep > 0.0 && self.keep <= 1.0) {
            problems.push(Problem::at(
                format!("{at}.keep"),
                Message::new("job-model-keep")
                    .arg("got", Value::Number(self.keep, Digits::Shortest)),
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
                Message::new("job-model-missing"),
            ));
        }
        if let Some(picture) = &self.picture {
            if self.up != ModelUp::Z {
                problems.push(Problem::at(
                    format!("{at}.up"),
                    Message::new("job-model-picture-up"),
                ));
            }
            let ranges = [
                ("smoothing", picture.smoothing, PICTURE_SMOOTHING_RANGE),
                ("background", picture.background, PICTURE_BACKGROUND_RANGE),
                ("gamma", picture.gamma, PICTURE_GAMMA_RANGE),
            ];
            for (field, value, (low, high)) in ranges {
                if !(value.is_finite() && (low..=high).contains(&value)) {
                    problems.push(Problem::at(
                        format!("{at}.picture.{field}"),
                        Message::new("job-model-picture-range")
                            .arg("low", Value::Number(low, Digits::Shortest))
                            .arg("high", Value::Number(high, Digits::Shortest))
                            .arg("got", Value::Number(value, Digits::Shortest)),
                    ));
                }
            }
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
    /// The font: a built-in one (FreeSans, FreeSerif, …) or one on the computer, by its
    /// full name (`chipload text fonts`). Only its name is stored; its letters are the
    /// text's shapes, kept where the font isn't (docs/specs/system-fonts.md).
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
