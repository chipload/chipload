// SPDX-License-Identifier: GPL-3.0-or-later
//! Surfaces and reliefs, with their passes and rims.

use super::*;

/// Surface: a top taken down flat by a wide flat endmill. With no shapes selected, a board
/// surface: the whole stock, in straight passes back and forth along its longer side, each
/// depth started beside the stock, cut once. With shapes, a part surface: each selected
/// closed shape grown by `margin`, cleared as a pocket under every instance of the part.
// Spec: docs/specs/surfacing.md, surface-levels.md.
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
    /// The id of the part it belongs to (`parts`). A board surface is faced once, whatever
    /// instances the part has; its part only says where it stands in the list.
    // Spec: docs/specs/parts.md.
    pub part: u64,
    /// The closed shapes a part surface faces. Empty: a board surface, the whole stock.
    /// Since schema 40.
    // Spec: docs/specs/surface-levels.md.
    pub select: Selector,
    /// How far a part surface's area reaches past its shapes, mm: 0 or more. Since schema 40.
    pub margin: f64,
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

/// Relief: a 3D model (`drawing.models`) carved from the top in parallel lines, each point
/// as low as the drop cutter lets the tool go without cutting into the model, within the
/// area it selects (by default the model's footprint outline).
// Spec: docs/specs/stl-relief.md §5.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct ReliefOp {
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
    /// The id of the model it carves (`drawing.models`).
    pub model: u64,
    /// The area: closed shapes, by default the model's outline. The tool's centre stays in
    /// it, or with a rim, within the rim's reach round it.
    pub select: Selector,
    /// The id of the project tool it cuts with (`tools`): a ball nose, an endmill or a
    /// V-bit.
    pub tool: i64,
    /// Where the model's top lies, mm down from the stock top: 0 for the top.
    pub start_depth: f64,
    /// The model's lowest point, mm down from the stock top: the start depth plus the
    /// model's depth, made again from the model whenever the project is read or changed.
    pub end_depth: f64,
    /// The distance between lines, as a fraction of the tool diameter: more than 0, at most
    /// 0.9.
    pub stepover: f64,
    /// The lines' direction, radians counter-clockwise from X.
    pub angle: f64,
    /// Back and forth (true), or every line the same way, lifting between them.
    pub zigzag: bool,
    /// The cutting feed, mm/min.
    pub feed: f64,
    /// The feed going down to each line's first point, mm/min.
    pub plunge_feed: f64,
    /// A roughing pass, cut just before: levels one step down apart, the tool kept the
    /// allowance off the model. Missing: none. Left out when missing.
    // Spec: docs/specs/stl-relief.md §5, §14 step 3 (optional).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub roughing: Option<ReliefRoughing>,
    /// A flat round the model at the floor and a ramp up from it to the stock top. Missing:
    /// none, the tool's centre stays within the area. Left out when missing.
    // Spec: docs/specs/stl-relief.md §16 (optional).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rim: Option<ReliefRim>,
    /// A smoothing pass, cut after the roughing pass and before the finish: the surface
    /// followed once, the allowance kept, taking the steps the roughing leaves. Only with a
    /// roughing pass. Missing: none. Left out when missing.
    // Spec: docs/specs/stl-relief.md §17 (optional).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub smoothing: Option<ReliefSmoothing>,
}

/// A relief's smoothing pass (docs/specs/stl-relief.md §17): after the roughing, the surface
/// followed once with the raster, an allowance left on the model for the finish.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct ReliefSmoothing {
    /// The id of the project tool it cuts with (`tools`): an end mill or a plain ball nose.
    pub tool: i64,
    /// The cutting feed, mm/min.
    pub feed: f64,
    /// The feed going down to each line's first point, mm/min.
    pub plunge_feed: f64,
    /// The distance between lines, as a fraction of the tool diameter: more than 0, at most
    /// 0.9.
    pub stepover: f64,
    /// The lines' direction, radians counter-clockwise from X: across the roughing's by
    /// default.
    pub angle: f64,
    /// What is left on the model for the finish, mm, in every direction: 0 or more.
    pub allowance: f64,
    /// False: kept, but not cut. Missing: true.
    #[serde(default = "enabled_default")]
    pub enabled: bool,
}

/// What a relief's rim follows (docs/specs/stl-relief.md §16).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum RimShape {
    /// The area's outer loops, grown with round corners.
    #[default]
    Outline,
    /// The rectangle round the area's outer loops, grown, its corners rounded.
    Box,
}

/// A relief's rim (docs/specs/stl-relief.md §16): a flat at the floor round the area, then a
/// ramp up to the stock top.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct ReliefRim {
    /// `outline`: the area's outer loops grown; `box`: the rectangle round them.
    pub shape: RimShape,
    /// The flat's width, mm from the outline or the rectangle: 0 or more.
    pub width: f64,
    /// The ramp's slope from the flat up to the stock top, radians from horizontal: from 10°
    /// to 90° (a straight wall).
    pub ramp: f64,
    /// `box` only: the radius of the flat's outer corners, mm, from 0 to the width.
    #[serde(default)]
    pub corner_radius: f64,
    /// `outline` only: how far the outline is filled out towards its convex hull before the
    /// flat goes round it, 0 (as it is) to 1 (the hull). Missing: 0.
    // Spec: docs/specs/stl-relief.md §16.7.
    #[serde(default)]
    pub hull: f64,
}

/// A relief rim's gentlest ramp, radians: a gentler one reaches far out.
pub const RIM_MIN_RAMP: f64 = 10.0 * std::f64::consts::PI / 180.0;

/// A new relief's rim (docs/specs/stl-relief.md §16): 5 mm round the outline, a 45° ramp.
pub const DEFAULT_RIM: ReliefRim = ReliefRim {
    shape: RimShape::Outline,
    width: 5.0,
    ramp: std::f64::consts::FRAC_PI_4,
    corner_radius: 0.0,
    hull: 0.0,
};

/// A relief's roughing pass (docs/specs/stl-relief.md §5): the material taken down in levels
/// with the raster, an allowance left on the model for the finish.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct ReliefRoughing {
    /// The id of the project tool it cuts with (`tools`): an end mill or a plain ball nose.
    pub tool: i64,
    /// The cutting feed, mm/min.
    pub feed: f64,
    /// The feed going down to each stretch's first point, mm/min.
    pub plunge_feed: f64,
    /// The distance between levels, mm.
    pub step_down: f64,
    /// The distance between lines, as a fraction of the tool diameter: more than 0, at most
    /// 0.9.
    pub stepover: f64,
    /// What is left on the model for the finish, mm, in every direction: 0 or more.
    pub allowance: f64,
    /// False: kept, but not cut. Missing: true.
    #[serde(default = "enabled_default")]
    pub enabled: bool,
}

/// The smallest and largest stepover of a surface, as a fraction of the tool diameter
/// (docs/specs/surfacing.md §4).
pub const SURFACE_STEPOVER: (f64, f64) = (0.1, 0.9);

/// A surface's own rules (docs/specs/surfacing.md §4, §5): its stepover's range, and no
/// deeper than the stock.
pub(in crate::models) fn surface_problems(i: usize, f: &SurfaceOp, thickness: f64) -> Vec<Problem> {
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
