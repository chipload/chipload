// SPDX-License-Identifier: GPL-3.0-or-later
//! Profiles and pockets: corners, finishing passes, skins, rest and clearing passes.

use super::*;

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
pub(in crate::models) fn corner_choice_problems(i: usize, op: &Operation) -> Vec<Problem> {
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
    /// The feed of the rings that cut full width (those with no ring inside them), mm/min.
    /// Null: `feed`.
    // Spec: docs/specs/cutting-data.md §5 (optional, no schema bump).
    #[serde(default)]
    pub slot_feed: Option<f64>,
    /// The deepest each step of a full-width ring goes, mm: it goes down in such steps to
    /// each depth pass, before the other rings cut the pass at once. Null: `step_down`.
    // Spec: docs/specs/cutting-data.md §4.
    #[serde(default)]
    pub slot_step_down: Option<f64>,
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
    pub rest_passes: Vec<ClearingPass>,
    /// A finishing pass: the roughing leaves `wall` mm on the walls and `floor` mm on the
    /// floor, then one level pass takes the floor and one lap each wall. Null: none. Left out
    /// when none.
    // Spec: docs/specs/pocket-finish.md (optional, no schema bump).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub finish: Option<PocketFinish>,
}

/// A finishing pass on a pocket.
// Spec: docs/specs/pocket-finish.md §2.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct PocketFinish {
    /// Left on every wall by the roughing, mm: 0 or more, less than the tool radius.
    pub wall: f64,
    /// Left above the end depth by the roughing, mm: 0 or more, at most the step down, less
    /// than the cut; 0 on a pocket cut through.
    pub floor: f64,
}

impl PocketFinish {
    /// The allowances a ticked *Finishing pass* starts at (docs/specs/pocket-finish.md §6).
    pub const DEFAULT: PocketFinish = PocketFinish {
        wall: 0.3,
        floor: 0.2,
    };
}

/// A clearing pass with its own end mill, inside an operation's shapes and depths: a
/// pocket's rest pass (docs/specs/rest-passes.md §2: an endmill smaller than the tool before
/// it, clearing only what that tool leaves) or a V-carve's floor pass
/// (docs/specs/vcarve-wide-strokes.md §4: an end mill clearing the floor between the walls,
/// before the V-carve).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct ClearingPass {
    /// The id of the project tool it cuts with (`tools`): an end mill.
    pub tool: i64,
    /// The cutting feed, mm/min.
    pub feed: f64,
    /// The feed going down into the material, mm/min.
    pub plunge_feed: f64,
    /// The deepest each pass goes, mm.
    pub step_down: f64,
    /// The distance between the clearing rings, as a fraction of the tool diameter.
    pub stepover: f64,
    /// As a pocket's `slot_feed`. Null: `feed`.
    // Spec: docs/specs/cutting-data.md §5 (optional, no schema bump).
    #[serde(default)]
    pub slot_feed: Option<f64>,
    /// As a pocket's `slot_step_down`. Null: `step_down`.
    #[serde(default)]
    pub slot_step_down: Option<f64>,
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
