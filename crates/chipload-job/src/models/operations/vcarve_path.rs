// SPDX-License-Identifier: GPL-3.0-or-later
//! V-carves, follow paths, chamfers, flutes, fill patterns and prisms.

use super::*;
use chipload_core::math;

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
    /// How many depth passes, 1 to [`MAX_VCARVE_PASSES`]: each removes the same
    /// cross-section, the last at the end depth (docs/specs/v-carve.md decision 12).
    pub passes: u32,
    /// An ordinary step down instead, mm: equal passes, none deeper than it, the last at
    /// the end depth, as in a pocket. Missing: passes of equal load (`passes`), which is
    /// kept meanwhile. Left out when missing.
    // Spec: docs/specs/vcarve-step-down.md §4 (optional, no schema bump).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub step_down: Option<f64>,
    /// Corners where the outline turns by less than this, degrees (0 to 90), get no branch
    /// of the centre line: the bit doesn't rise into every kink. Missing: 0, every corner
    /// carved, as before the setting existed.
    // Spec: docs/specs/vcarve-smooth.md §3 (optional, no schema bump).
    #[serde(default)]
    pub ignore_kinks_deg: f64,
    /// The feed, mm/min, for moves no steeper than 45°.
    pub feed: f64,
    /// The feed for moves steeper than 45°, mm/min.
    pub plunge_feed: f64,
    /// A floor pass: an end mill that clears the floor between the walls of strokes too
    /// wide to carve from their centre line. Missing: none. Left out when missing.
    // Spec: docs/specs/vcarve-wide-strokes.md §4 (optional, no schema bump).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub floor: Option<ClearingPass>,
    /// The plug of a V-carve inlay: the rest of the board is carved, its outline taken
    /// `plug.start_depth` below the surface, so the shapes stand as a tapered plug that fits
    /// the pocket a V-carve of the same shapes cuts. Missing: an ordinary V-carve. Left out
    /// when missing.
    // Spec: docs/specs/vcarve-inlay.md §3 (optional, no schema bump).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plug: Option<Plug>,
}

/// A V-carve inlay's plug (docs/specs/vcarve-inlay.md §2.2).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Plug {
    /// Where the outline is taken, mm down from the stock top: more than the operation's
    /// start depth, less than its end depth. The end depth less this is the glue gap.
    pub start_depth: f64,
}

impl VcarveOp {
    /// The depth the carving's outline lies at, mm down from the stock top: the plug's start
    /// depth for a plug, otherwise the start depth.
    pub fn outline_depth(&self) -> f64 {
        self.plug.map_or(self.start_depth, |p| p.start_depth)
    }

    /// How far inside the outline the floor of a stroke too wide to carve begins: where the
    /// V-bit `bit`'s walls meet the floor, at its deepest (the end depth, or as deep as its
    /// cone reaches). None for a bit without an angle.
    // Spec: docs/specs/vcarve-wide-strokes.md §4.
    pub fn floor_inset(&self, bit: &Tool) -> Option<f64> {
        let slope = math::tan(bit.angle? / 2.0);
        let tip = bit.tip_diameter.unwrap_or(0.0);
        let deepest = (bit.diameter - tip) / 2.0 / slope;
        // A plug's walls are V-carved from its outline depth down (vcarve-inlay.md §2.2).
        let above = self.outline_depth() - self.start_depth;
        let depth = (self.end_depth - self.outline_depth()).min(deepest - above);
        Some(depth * slope)
    }
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

/// Where a chamfer's waste is: around the shape (a part's outline) or within it (a hole's or
/// a pocket's rim).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum EdgeSide {
    Outside,
    Inside,
}

impl EdgeSide {
    /// The same side as a profile's.
    pub fn side(self) -> Side {
        match self {
            EdgeSide::Outside => Side::Outside,
            EdgeSide::Inside => Side::Inside,
        }
    }
}

/// Chamfer: the top edge of every selected closed shape broken with a V-bit, `width` mm wide
/// at the surface. The bit's flank runs on the line, its tip in the waste; the depth follows
/// from the bit's angle.
// Spec: docs/specs/chamfer.md.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct ChamferOp {
    /// A name for the operation, shown in the list and on the setup sheet. Empty: none.
    #[serde(default)]
    pub name: String,
    /// False: the operation stays in the project but isn't cut. Missing: true.
    #[serde(default = "enabled_default")]
    pub enabled: bool,
    /// The id of the part it belongs to (`parts`).
    pub part: u64,
    /// The library material its feeds come from. Null: its own values.
    #[serde(default)]
    pub material: Option<MaterialRef>,
    /// What the operation cuts: the shapes on a layer, or shapes by id. Closed shapes only;
    /// open ones are skipped with a note.
    pub select: Selector,
    /// The id of the project tool it cuts with (`tools`): a V-bit with its angle.
    pub tool: i64,
    /// Where the waste is: `outside` the shape (a part's outline) or `inside` it (a hole's
    /// or a pocket's rim).
    pub side: EdgeSide,
    /// The chamfer's width at the surface, mm, measured from the line into the part. More
    /// than 0, at most half the bit's cone (its diameter less its tip, halved).
    pub width: f64,
    /// Where the edge is, mm down from the stock top: 0 for the top, more for the edge of a
    /// step an earlier operation cut.
    pub start_depth: f64,
    /// The deepest each pass goes, mm, in equal passes. Null: one pass.
    #[serde(default)]
    pub step_down: Option<f64>,
    /// `climb` or `conventional` milling.
    pub direction: Direction,
    /// The cutting feed, mm/min.
    pub feed: f64,
    /// The feed going down into the material, mm/min.
    pub plunge_feed: f64,
    /// How the tool goes down at the start of each pass: `plunge` or `ramp` along the path.
    pub entry: Entry,
}

impl ChamferOp {
    /// How deep the chamfer goes below its start depth with `tool`, mm: its width over the
    /// tangent of the bit's half angle (docs/specs/chamfer.md §2). None for a tool that
    /// isn't a V-bit with an angle.
    pub fn depth_with(&self, tool: &Tool) -> Option<f64> {
        tool.chamfer_depth_for(self.width)
    }
}

/// How a flute's ramp goes down from its end to the full depth.
// Spec: docs/specs/fluting.md §3.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum RampShape {
    /// Evenly: depth × u, u from 0 at the end to 1 at full depth.
    #[default]
    Straight,
    /// Steepest at the surface, easing into the floor: depth × sin(π/2 × u).
    Curved,
}

/// Flute: a groove on the line of every selected open shape, going down over `ramp_start`
/// mm from its first end to `end_depth`, and up again over `ramp_end` mm to its last end.
// Spec: docs/specs/fluting.md.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct FluteOp {
    /// A name for the operation, shown in the list and on the setup sheet. Empty: none.
    #[serde(default)]
    pub name: String,
    /// False: the operation stays in the project but isn't cut. Missing: true.
    #[serde(default = "enabled_default")]
    pub enabled: bool,
    /// The id of the part it belongs to (`parts`).
    pub part: u64,
    /// The library material its feeds come from. Null: its own values.
    #[serde(default)]
    pub material: Option<MaterialRef>,
    /// What the operation cuts: the shapes on a layer, or shapes by id. Open shapes only;
    /// closed ones are skipped with a note.
    pub select: Selector,
    /// The id of the project tool it cuts with (`tools`): a ball nose, a V-bit or an
    /// endmill.
    pub tool: i64,
    /// Where the flute starts at its ends, mm down from the stock top: 0 for the top.
    pub start_depth: f64,
    /// The flute's depth through its middle, mm down from the stock top.
    pub end_depth: f64,
    /// The deepest each pass goes, mm, in equal passes. Null: one pass.
    #[serde(default)]
    pub step_down: Option<f64>,
    /// How far along the shape from its first end the flute reaches its depth, mm. 0: a
    /// square end.
    pub ramp_start: f64,
    /// How far along the shape before its last end the flute starts to rise, mm. 0: a
    /// square end.
    pub ramp_end: f64,
    /// `straight` or `curved` ramps.
    pub ramp_shape: RampShape,
    /// The cutting feed, mm/min, ramps included.
    pub feed: f64,
    /// The feed going straight down, mm/min.
    pub plunge_feed: f64,
}

/// The pattern a `PatternOp` engraves (docs/specs/fill-pattern.md §3). Angles in radians from
/// the X axis, counter-clockwise; lengths in mm.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "lowercase", deny_unknown_fields)]
pub enum FillPattern {
    /// Parallel straight lines, `spacing` apart (centre to centre), at `angle`.
    Lines { angle: f64, spacing: f64 },
    /// Two sets of lines: at `angle`, and at `angle` + `between` (more than 0, at most 90°).
    Cross {
        angle: f64,
        spacing: f64,
        between: f64,
    },
    /// The shape's outline repeated inwards, `spacing` apart, as closed rings.
    Rings { spacing: f64 },
    /// Parallel sine waves at `angle`, `spacing` apart: `wavelength` long, `height` from top
    /// to bottom.
    Waves {
        angle: f64,
        spacing: f64,
        wavelength: f64,
        height: f64,
    },
    /// Rows `spacing` apart of strokes `length` long with `gap` between, every other row
    /// shifted half a stroke and gap (docs/specs/fill-pattern.md §14).
    Dashes {
        angle: f64,
        spacing: f64,
        length: f64,
        gap: f64,
        /// Only strokes the shape doesn't cut.
        whole: bool,
    },
    /// Columns of strokes `length` long at 45° to `angle`, `spacing` apart along the column,
    /// every other column mirrored, meeting in chevrons.
    Herringbone {
        angle: f64,
        spacing: f64,
        length: f64,
        whole: bool,
    },
    /// Rows `spacing` apart of half circles of `radius` side by side, every other row
    /// shifted by a radius.
    Scales {
        angle: f64,
        spacing: f64,
        radius: f64,
        whole: bool,
    },
    /// Round the middle of each shape: rings `spacing` apart of strokes `length` long
    /// pointing outwards, `pitch` apart round the ring, every other ring turned half a pitch.
    Sunburst {
        spacing: f64,
        length: f64,
        pitch: f64,
        whole: bool,
    },
}

impl FillPattern {
    /// From one groove's centre to the next, mm.
    pub fn spacing(&self) -> f64 {
        match *self {
            FillPattern::Lines { spacing, .. }
            | FillPattern::Cross { spacing, .. }
            | FillPattern::Rings { spacing }
            | FillPattern::Waves { spacing, .. }
            | FillPattern::Dashes { spacing, .. }
            | FillPattern::Herringbone { spacing, .. }
            | FillPattern::Scales { spacing, .. }
            | FillPattern::Sunburst { spacing, .. } => spacing,
        }
    }

    /// The direction of its lines from the X axis, radians; None for rings and a sunburst.
    pub fn angle(&self) -> Option<f64> {
        match *self {
            FillPattern::Lines { angle, .. }
            | FillPattern::Cross { angle, .. }
            | FillPattern::Waves { angle, .. }
            | FillPattern::Dashes { angle, .. }
            | FillPattern::Herringbone { angle, .. }
            | FillPattern::Scales { angle, .. } => Some(angle),
            FillPattern::Rings { .. } | FillPattern::Sunburst { .. } => None,
        }
    }

    /// Whether it is made of short strokes (docs/specs/fill-pattern.md §14).
    pub fn strokes(&self) -> bool {
        matches!(
            self,
            FillPattern::Dashes { .. }
                | FillPattern::Herringbone { .. }
                | FillPattern::Scales { .. }
                | FillPattern::Sunburst { .. }
        )
    }
}

/// Pattern: grooves engraved inside every selected closed shape, its selected holes left
/// alone, in a pattern laid out from the drawing's origin.
// Spec: docs/specs/fill-pattern.md.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct PatternOp {
    /// A name for the operation, shown in the list and on the setup sheet. Empty: none.
    #[serde(default)]
    pub name: String,
    /// False: the operation stays in the project but isn't cut. Missing: true.
    #[serde(default = "enabled_default")]
    pub enabled: bool,
    /// The id of the part it belongs to (`parts`).
    pub part: u64,
    /// The library material its feeds come from. Null: its own values.
    #[serde(default)]
    pub material: Option<MaterialRef>,
    /// What the operation fills: closed shapes, the selected shapes inside them as holes.
    pub select: Selector,
    /// The id of the project tool it cuts with (`tools`): a V-bit, a ball nose or an endmill.
    pub tool: i64,
    /// The pattern and its settings.
    pub pattern: FillPattern,
    /// True: the grooves stay inside the shape's line and out of its holes, clipped half a
    /// groove's width in. False: they run up to the line, centred on it.
    pub inside: bool,
    /// Where the grooves start, mm down from the stock top: 0 for the top.
    pub start_depth: f64,
    /// How deep the grooves go, mm down from the stock top.
    pub end_depth: f64,
    /// The deepest each pass goes, mm, in equal passes. Null: one pass.
    #[serde(default)]
    pub step_down: Option<f64>,
    /// How long each groove ramps in and out at its ends, mm, as a flute. 0: square ends.
    pub ramp: f64,
    /// The cutting feed, mm/min.
    pub feed: f64,
    /// The feed going straight down, mm/min.
    pub plunge_feed: f64,
}

/// Prism: every selected closed shape left standing as a roof, its sides sloping up from its
/// edges to a ridge along its centre line, cut with a V-bit's tip running along the outline.
// Spec: docs/specs/prism.md.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct PrismOp {
    /// A name for the operation, shown in the list and on the setup sheet. Empty: none.
    #[serde(default)]
    pub name: String,
    /// False: the operation stays in the project but isn't cut. Missing: true.
    #[serde(default = "enabled_default")]
    pub enabled: bool,
    /// The id of the part it belongs to (`parts`).
    pub part: u64,
    /// The library material its feeds come from. Null: its own values.
    #[serde(default)]
    pub material: Option<MaterialRef>,
    /// What the operation carves: closed shapes, the selected shapes inside them as holes.
    pub select: Selector,
    /// The id of the project tool it cuts with (`tools`): a V-bit with its angle.
    pub tool: i64,
    /// Where the surface is, mm down from the stock top: 0 for the top.
    pub start_depth: f64,
    /// The deepest an edge goes below the start depth, mm, for bevelled shapes with a flat
    /// top. Null: as deep as the shapes and the bit's cone need.
    #[serde(default)]
    pub max_depth: Option<f64>,
    /// The deepest each pass goes, mm, in equal passes. Null: one pass.
    #[serde(default)]
    pub step_down: Option<f64>,
    /// `climb` or `conventional` milling.
    pub direction: Direction,
    /// The cutting feed, mm/min.
    pub feed: f64,
    /// The feed going straight down, mm/min.
    pub plunge_feed: f64,
}

impl PrismOp {
    /// The deepest an edge can go below the start depth with `tool`, mm: its cone's height
    /// (the shank would rub below it), no deeper than the max depth. With no V-bit with an
    /// angle, the max depth or 0 (validation refuses the tool anyway).
    pub fn deepest(&self, tool: Option<&Tool>) -> f64 {
        let cone = tool
            .and_then(|t| t.widest_chamfer().zip(t.chamfer_depth_for(1.0)))
            .map(|(reach, per_mm)| reach * per_mm);
        match (cone, self.max_depth) {
            (Some(cone), Some(max)) => cone.min(max),
            (Some(cone), None) => cone,
            (None, max) => max.unwrap_or(0.0),
        }
    }
}
