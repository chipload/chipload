// SPDX-License-Identifier: GPL-3.0-or-later
//! Mouldings and holes operations.

use super::*;

/// A moulding's cross-section (docs/specs/moulding.md §3): a built-in shape by its size, or
/// a drawn open shape read as the section.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "lowercase", deny_unknown_fields)]
pub enum MouldProfile {
    /// A concave quarter circle: `radius` deep at the path, at the surface `radius` in.
    Cove { radius: f64 },
    /// A convex quarter circle: the edge rounded, `radius` down at the path.
    Roundover { radius: f64 },
    /// An S, `width` wide and `depth` deep: a cove into a round-over, half the width each.
    Ogee { width: f64, depth: f64 },
    /// A raised half round `2 × radius` wide, its top at the surface.
    Bead { radius: f64 },
    /// A half-round groove `2 × radius` wide and `radius` deep.
    Flute { radius: f64 },
    /// A drawn open shape, by id: across to the right, up the page; its leftmost point at
    /// the path, its highest point the surface.
    Shape { shape: u64 },
}

impl MouldProfile {
    /// How deep the section goes, mm; None for a drawn shape (known from the drawing).
    pub fn depth(&self) -> Option<f64> {
        match *self {
            MouldProfile::Cove { radius }
            | MouldProfile::Roundover { radius }
            | MouldProfile::Bead { radius }
            | MouldProfile::Flute { radius } => Some(radius),
            MouldProfile::Ogee { depth, .. } => Some(depth),
            MouldProfile::Shape { .. } => None,
        }
    }
}

/// Which side of the path a moulding's profile lies on (docs/specs/moulding.md §4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum MouldSide {
    /// Left of an open path as drawn.
    Left,
    /// Right of an open path as drawn.
    Right,
    /// Inside a closed path.
    Inside,
    /// Outside a closed path.
    Outside,
}

/// How a moulding ends on an open path (docs/specs/moulding.md §4).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "lowercase", deny_unknown_fields)]
pub enum MouldEnds {
    /// The full profile right to the path's end.
    Square,
    /// The profile turned a quarter round the path's end point.
    Round,
    /// The section's depth fading to the surface over `length` mm at each end.
    Taper { length: f64, shape: RampShape },
}

/// Moulding: a profile swept along every selected path, in passes along it.
// Spec: docs/specs/moulding.md.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct MouldingOp {
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
    /// The paths the moulding runs along: open or closed shapes.
    pub select: Selector,
    /// The id of the project tool it cuts with (`tools`): a ball nose or an endmill.
    pub tool: i64,
    /// The cross-section.
    pub profile: MouldProfile,
    /// The side of the path the profile lies on.
    pub side: MouldSide,
    /// How it ends on open paths.
    pub ends: MouldEnds,
    /// Where the surface is, mm down from the stock top: 0 for the top.
    pub start_depth: f64,
    /// The distance between passes across the profile, mm.
    pub stepover: f64,
    /// The deepest each level goes, mm, in equal levels. Null: one level.
    #[serde(default)]
    pub step_down: Option<f64>,
    /// `climb` or `conventional` milling.
    pub direction: Direction,
    /// The cutting feed, mm/min.
    pub feed: f64,
    /// The feed going straight down, mm/min.
    pub plunge_feed: f64,
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
    /// Dowel holes for turning the board over (docs/specs/two-sided.md §6): drawn circles
    /// on the turn axis, cut through the board and `through` mm into the spoilboard for the
    /// pins, deeper than the stock's max overcut allows any other cut. Missing: false.
    #[serde(default)]
    pub dowels: bool,
}
