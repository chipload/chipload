// SPDX-License-Identifier: GPL-3.0-or-later
//! Operations: profile, pocket, V-carve, follow path, holes, surface, and what they share
//! (tabs, corners, entries, leads, passes).

use super::*;

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
    /// Where a ring without tabs placed by hand gets its `count` tabs: on its edges along
    /// `x` (bottom and top) or `y` (left and right), half on each. Left out: evenly round
    /// it.
    // Spec: docs/specs/tabs-along-grain.md (optional, no schema bump).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub along: Option<Axis>,
}

/// An axis of the drawing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum Axis {
    X,
    Y,
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

    /// The tabs a profile starts with (docs/specs/styles.md §2.1): 4 per contour, 8 mm
    /// wide, 3 mm high but at most half of `thickness`, the flank from `default_flank`.
    pub fn starting(ramp_angle: f64, center_cutting: Option<bool>, thickness: f64) -> Tabs {
        Tabs {
            count: 4,
            width: 8.0,
            height: STARTING_TAB_HEIGHT.min(0.5 * thickness),
            flank_angle: Tabs::default_flank(ramp_angle, center_cutting),
            at: Vec::new(),
            along: None,
        }
    }
}

/// A new tab's height, mm, in stock at least twice as thick.
pub const STARTING_TAB_HEIGHT: f64 = 3.0;

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
pub(super) static NO_SELECTOR: Selector = Selector {
    layer: None,
    shapes: None,
    source: None,
    exclude: Vec::new(),
};

pub(super) fn enabled_default() -> bool {
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
    /// Whether the operation already cuts with these values: the ones it takes
    /// (docs/specs/cutting-data.md §3). Full-width cuts (profiles, paths, V-carves, bores)
    /// take the slot values, a surface the clearing ones and its own stepover, a pocket
    /// both. The speed is the tool's, not the operation's.
    pub fn has_cutting(&self, c: &crate::library::Cutting) -> bool {
        // A pocket without slot values cuts its full-width rings at its feed and step down.
        let mut own = self.clone();
        if let Operation::Pocket(p) = &mut own {
            p.slot_feed = Some(p.slot_feed.unwrap_or(p.feed));
            p.slot_step_down = Some(p.slot_step_down.unwrap_or(p.step_down));
        }
        let mut applied = own.clone();
        applied.apply_cutting(c);
        applied == own
    }

    /// Cut with these values: the ones the operation takes (as `has_cutting`).
    pub fn apply_cutting(&mut self, c: &crate::library::Cutting) {
        match self {
            Operation::Profile(p) => {
                (p.feed, p.plunge_feed, p.step_down) =
                    (c.slot_feed, c.plunge_feed, c.slot_step_down);
            }
            Operation::Pocket(p) => {
                (p.feed, p.plunge_feed, p.step_down, p.stepover) =
                    (c.feed, c.plunge_feed, c.step_down, c.stepover);
                (p.slot_feed, p.slot_step_down) = (Some(c.slot_feed), Some(c.slot_step_down));
            }
            Operation::Vcarve(v) => {
                (v.feed, v.plunge_feed) = (c.slot_feed, c.plunge_feed);
                if let Some(step) = &mut v.step_down {
                    *step = c.slot_step_down;
                }
            }
            Operation::Path(p) => {
                (p.feed, p.plunge_feed, p.step_down) =
                    (c.slot_feed, c.plunge_feed, c.slot_step_down);
            }
            Operation::Holes(h) => {
                (h.feed, h.plunge_feed) = (c.slot_feed, c.plunge_feed);
                if let HoleMethod::Bore { step_down, .. } = &mut h.method {
                    *step_down = c.slot_step_down;
                }
            }
            Operation::Surface(f) => {
                (f.feed, f.plunge_feed, f.step_down) = (c.feed, c.plunge_feed, c.step_down);
            }
        }
    }

    /// `base` with the operation's own values in place of those it takes (as
    /// `has_cutting`): what saving the operation's values keeps.
    pub fn cutting_over(&self, base: crate::library::Cutting) -> crate::library::Cutting {
        let mut c = base;
        match self {
            Operation::Profile(p) => {
                (c.slot_feed, c.plunge_feed, c.slot_step_down) =
                    (p.feed, p.plunge_feed, p.step_down);
            }
            Operation::Pocket(p) => {
                (c.feed, c.plunge_feed, c.step_down, c.stepover) =
                    (p.feed, p.plunge_feed, p.step_down, p.stepover);
                c.slot_feed = p.slot_feed.unwrap_or(p.feed);
                c.slot_step_down = p.slot_step_down.unwrap_or(p.step_down);
            }
            Operation::Vcarve(v) => {
                (c.slot_feed, c.plunge_feed) = (v.feed, v.plunge_feed);
                if let Some(step) = v.step_down {
                    c.slot_step_down = step;
                }
            }
            Operation::Path(p) => {
                (c.slot_feed, c.plunge_feed, c.slot_step_down) =
                    (p.feed, p.plunge_feed, p.step_down);
            }
            Operation::Holes(h) => {
                (c.slot_feed, c.plunge_feed) = (h.feed, h.plunge_feed);
                if let HoleMethod::Bore { step_down, .. } = h.method {
                    c.slot_step_down = step_down;
                }
            }
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

    /// Its helixes, and its passes', that no longer fit their tool in `tools` get the
    /// default for it ([`Job::fit_helixes`]).
    pub fn fit_helixes(&mut self, tools: &[Tool]) {
        let diameter = |id: i64| find_tool(tools, id).map(|t| t.diameter);
        match self {
            Operation::Pocket(p) => {
                if let Some(d) = diameter(p.tool) {
                    fit_helix(&mut p.entry, d);
                }
                for pass in &mut p.rest_passes {
                    if let Some(d) = diameter(pass.tool) {
                        fit_helix(&mut pass.entry, d);
                    }
                }
            }
            Operation::Vcarve(v) => {
                if let Some(pass) = &mut v.floor
                    && let Some(d) = diameter(pass.tool)
                {
                    fit_helix(&mut pass.entry, d);
                }
            }
            Operation::Profile(_)
            | Operation::Path(_)
            | Operation::Holes(_)
            | Operation::Surface(_) => {}
        }
    }

    /// Every project tool it cuts with or names, each once: its own first, then a rest
    /// pocket's larger tool, its rest passes' and floor pass's.
    pub fn tool_ids(&self) -> Vec<i64> {
        let mut ids = vec![self.tool()];
        let mut push = |id: i64| {
            if !ids.contains(&id) {
                ids.push(id);
            }
        };
        match self {
            Operation::Pocket(p) => {
                if let Some(rest) = &p.rest {
                    push(rest.after_tool);
                }
                for pass in &p.rest_passes {
                    push(pass.tool);
                }
            }
            Operation::Vcarve(v) => {
                if let Some(pass) = &v.floor {
                    push(pass.tool);
                }
            }
            Operation::Profile(_)
            | Operation::Path(_)
            | Operation::Holes(_)
            | Operation::Surface(_) => {}
        }
        ids
    }

    /// Give every tool it names the id `ids` maps it to; ids not in `ids` stay.
    pub fn remap_tools(&mut self, ids: &std::collections::BTreeMap<i64, i64>) {
        let new = |id: i64| ids.get(&id).copied().unwrap_or(id);
        let own = new(self.tool());
        self.set_tool(own);
        match self {
            Operation::Pocket(p) => {
                if let Some(rest) = &mut p.rest {
                    rest.after_tool = new(rest.after_tool);
                }
                for pass in &mut p.rest_passes {
                    pass.tool = new(pass.tool);
                }
            }
            Operation::Vcarve(v) => {
                if let Some(pass) = &mut v.floor {
                    pass.tool = new(pass.tool);
                }
            }
            Operation::Profile(_)
            | Operation::Path(_)
            | Operation::Holes(_)
            | Operation::Surface(_) => {}
        }
    }

    /// Set the end depth, mm down from the stock top.
    pub fn set_end_depth(&mut self, depth: f64) {
        match self {
            Operation::Profile(op) => op.end_depth = depth,
            Operation::Pocket(op) => op.end_depth = depth,
            Operation::Vcarve(op) => op.end_depth = depth,
            Operation::Path(op) => op.end_depth = depth,
            Operation::Holes(op) => op.end_depth = depth,
            Operation::Surface(op) => op.end_depth = depth,
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

    /// Dowel holes for turning the board over (docs/specs/two-sided.md §6).
    pub fn is_dowels(&self) -> bool {
        matches!(self, Operation::Holes(h) if h.dowels)
    }

    /// How far below the stock bottom it may cut with its full diameter: the stock's max
    /// overcut, or a dowel hole's pin depth when that is deeper.
    pub fn allowed_below(&self, max_overcut: f64) -> f64 {
        match self {
            Operation::Holes(h) if h.dowels => {
                h.through.map_or(max_overcut, |t| t.max(max_overcut))
            }
            _ => max_overcut,
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
    /// plus `through`, plus `point` (a drill's point, so its full diameter goes through:
    /// `Job::drill_point`). Nothing changes for one whose end depth is as set.
    pub fn follow_thickness(&mut self, thickness: f64, point: f64) {
        let end = match self {
            Operation::Profile(op) => op.through.map(|t| (&mut op.end_depth, t)),
            Operation::Pocket(op) => op.through.map(|t| (&mut op.end_depth, t)),
            Operation::Path(op) => op.through.map(|t| (&mut op.end_depth, t)),
            Operation::Holes(op) => op.through.map(|t| (&mut op.end_depth, t)),
            Operation::Vcarve(_) | Operation::Surface(_) => None,
        };
        if let Some((end_depth, below)) = end {
            *end_depth = thickness + below + point;
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
pub(super) fn corner_choice_problems(i: usize, op: &Operation) -> Vec<Problem> {
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
        let slope = (bit.angle? / 2.0).tan();
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
pub(super) fn surface_problems(i: usize, f: &SurfaceOp, thickness: f64) -> Vec<Problem> {
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

/// A helix as wide as the tool's radius or wider, for a tool of `diameter`: the default
/// radius, its angle kept (it would leave a core standing).
fn fit_helix(entry: &mut Entry, diameter: f64) {
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
