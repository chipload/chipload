// SPDX-License-Identifier: GPL-3.0-or-later
//! The stock, the machine, work zero and the tools.

use super::*;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum ToolType {
    #[default]
    Endmill,
    Drill,
    Vbit,
    /// A ball nose, plain or tapered (docs/specs/stl-relief.md §8): for reliefs and follow
    /// paths only.
    Ballnose,
}

impl ToolType {
    /// The kind as messages and lists name it: `endmill`, `drill`, `V-bit`, `ball nose`.
    pub fn name(self) -> &'static str {
        match self {
            ToolType::Endmill => "endmill",
            ToolType::Drill => "drill",
            ToolType::Vbit => "V-bit",
            ToolType::Ballnose => "ball nose",
        }
    }

    /// The name with its article: `an endmill`, `a drill`, `a V-bit`, `a ball nose`.
    pub fn with_article(self) -> &'static str {
        match self {
            ToolType::Endmill => "an endmill",
            ToolType::Drill => "a drill",
            ToolType::Vbit => "a V-bit",
            ToolType::Ballnose => "a ball nose",
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
            ToolType::Ballnose => "ballnose",
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
    /// `endmill`, `drill`, `vbit` or `ballnose`.
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
    /// Ball noses: the taper's included angle; null: a plain ball nose.
    // Spec: docs/specs/v-carve.md §2, stl-relief.md §8.
    #[serde(default)]
    pub angle: Option<f64>,
    /// V-bits: the flat tip's diameter, mm. Null: pointed. Tapered ball noses: the ball's
    /// diameter at the tip.
    #[serde(default)]
    pub tip_diameter: Option<f64>,
}

/// A drill's point, taken as 118° (docs/specs/simulation.md decision 3).
pub const DRILL_POINT_RAD: f64 = 118.0 * std::f64::consts::PI / 180.0;

impl Tool {
    /// The shape of its cutting end (docs/specs/stl-relief.md §6). None for a V-bit or a
    /// tapered ball nose without its angle, or a tapered ball nose without its ball.
    pub fn cutter(&self) -> Option<chipload_core::cutter::Cutter> {
        use chipload_core::cutter::Cutter;
        let radius = self.diameter / 2.0;
        let half = self
            .angle
            .filter(|a| a.is_finite() && *a > 0.0)
            .map(|a| a / 2.0);
        Some(match self.kind {
            ToolType::Endmill => Cutter::Flat { radius },
            ToolType::Drill => Cutter::Cone {
                radius,
                tip: 0.0,
                half_angle: DRILL_POINT_RAD / 2.0,
            },
            ToolType::Vbit => Cutter::Cone {
                radius,
                tip: (self.tip_diameter.unwrap_or(0.0) / 2.0).clamp(0.0, radius),
                half_angle: half?,
            },
            ToolType::Ballnose => match self.angle {
                None => Cutter::Ball { radius },
                Some(_) => Cutter::TaperedBall {
                    radius,
                    ball: self.tip_diameter? / 2.0,
                    half_angle: half?,
                },
            },
        })
    }

    /// A V-bit's width at `depth` below its tip's end, mm: its tip and twice the depth's
    /// run along the flanks, at most its diameter. None for other tools and a V-bit
    /// without its angle.
    pub fn vbit_width_at(&self, depth: f64) -> Option<f64> {
        let half = self.vbit_half_angle()?;
        let tip = self.tip_diameter.unwrap_or(0.0);
        Some((tip + 2.0 * depth.max(0.0) * half.tan()).min(self.diameter))
    }

    /// How deep a V-bit goes to be `width` wide, mm (docs/specs/sign-maker.md §5): 0 for a
    /// width within its tip. None for other tools, a V-bit without its angle, and a width
    /// wider than the bit.
    pub fn vbit_depth_for(&self, width: f64) -> Option<f64> {
        let half = self.vbit_half_angle()?;
        if !(width.is_finite() && width <= self.diameter) {
            return None;
        }
        let tip = self.tip_diameter.unwrap_or(0.0);
        Some(((width - tip) / 2.0 / half.tan()).max(0.0))
    }

    fn vbit_half_angle(&self) -> Option<f64> {
        let angle = self.angle.filter(|a| a.is_finite() && *a > 0.0)?;
        (self.kind == ToolType::Vbit).then_some(angle / 2.0)
    }
}

#[cfg(test)]
mod vbit_tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    fn vbit(angle_deg: f64, tip: Option<f64>) -> Tool {
        let mut tool: Tool = serde_json::from_value(serde_json::json!({
            "id": 1, "type": "vbit", "diameter": 12.7, "angle": angle_deg.to_radians(),
            "tip_diameter": tip
        }))
        .unwrap();
        tool.name = None;
        tool
    }

    #[test]
    fn a_vbits_width_and_depth_undo_each_other() {
        let v90 = vbit(90.0, None);
        assert!((v90.vbit_width_at(1.5).unwrap() - 3.0).abs() < 1e-12);
        assert!((v90.vbit_depth_for(3.0).unwrap() - 1.5).abs() < 1e-12);
        let flat = vbit(60.0, Some(0.2));
        let d = flat.vbit_depth_for(2.0).unwrap();
        assert!((flat.vbit_width_at(d).unwrap() - 2.0).abs() < 1e-12);
        assert_eq!(flat.vbit_depth_for(0.1), Some(0.0));
        assert_eq!(flat.vbit_depth_for(20.0), None);
        assert_eq!(flat.vbit_width_at(100.0), Some(12.7));
    }
}
