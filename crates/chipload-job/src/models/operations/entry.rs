// SPDX-License-Identifier: GPL-3.0-or-later
//! Drill methods, leads and entries.

use super::*;

/// How far a drill's point reaches below its full diameter, per mm of diameter: a 118°
/// point, the common one for twist drills, is (D / 2) / tan(59°) long
/// (docs/specs/holes.md decision 10).
pub const DRILL_POINT_PER_DIAMETER: f64 = 0.3004;

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
    /// (docs/specs/holes.md §7); None for a V-bit or a ball nose.
    pub fn default_for(kind: ToolType, diameter: f64) -> Option<HoleMethod> {
        match kind {
            ToolType::Drill => Some(HoleMethod::Drill { peck: None }),
            ToolType::Endmill => Some(HoleMethod::Bore {
                step_down: 0.5 * diameter,
                ramp_angle: DEFAULT_RAMP_ANGLE_RAD,
                direction: Direction::Climb,
            }),
            ToolType::Vbit | ToolType::Ballnose => None,
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

/// A helix as wide as the tool's radius or wider, for a tool of `diameter`: the default
/// radius, its angle kept (it would leave a core standing).
pub(super) fn fit_helix(entry: &mut Entry, diameter: f64) {
    if let Entry::Helix { radius, ramp_angle } = *entry
        && radius >= diameter / 2.0
    {
        *entry = Entry::Helix {
            radius: DEFAULT_HELIX_DIAMETERS * diameter,
            ramp_angle,
        };
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
