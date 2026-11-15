// SPDX-License-Identifier: GPL-3.0-or-later
//! Operations: profile, pocket, V-carve, follow path, holes, surface, and what they share
//! (tabs, corners, entries, leads, passes).

use super::*;
use chipload_core::math;

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
    /// Chamfer: a V-bit's flank on the line of closed shapes, breaking their top edge.
    // Spec: docs/specs/chamfer.md.
    Chamfer(ChamferOp),
    /// Flute: a groove on the line of open shapes that ramps in and out at their ends.
    // Spec: docs/specs/fluting.md.
    Flute(FluteOp),
    /// Pattern: grooves of lines, cross-hatch, rings or waves engraved inside closed shapes.
    // Spec: docs/specs/fill-pattern.md.
    Pattern(PatternOp),
    /// Prism: closed shapes left standing as roofs, a sharp ridge along their centre line.
    // Spec: docs/specs/prism.md.
    Prism(PrismOp),
    /// Holes: drilled with a drill, bored with a helix with an endmill.
    // Spec: docs/specs/holes.md.
    Holes(HolesOp),
    /// Surface: the whole stock top taken down flat, in straight passes back and forth.
    // Spec: docs/specs/surfacing.md.
    Surface(SurfaceOp),
    /// Relief: a 3D model carved from the top with the drop cutter, in parallel lines.
    // Spec: docs/specs/stl-relief.md §5.
    Relief(ReliefOp),
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
            Operation::Chamfer(p) => {
                (p.feed, p.plunge_feed) = (c.slot_feed, c.plunge_feed);
                if let Some(step) = &mut p.step_down {
                    *step = c.slot_step_down;
                }
            }
            Operation::Flute(p) => {
                (p.feed, p.plunge_feed) = (c.slot_feed, c.plunge_feed);
                if let Some(step) = &mut p.step_down {
                    *step = c.slot_step_down;
                }
            }
            Operation::Pattern(p) => {
                (p.feed, p.plunge_feed) = (c.slot_feed, c.plunge_feed);
                if let Some(step) = &mut p.step_down {
                    *step = c.slot_step_down;
                }
            }
            Operation::Prism(p) => {
                (p.feed, p.plunge_feed) = (c.slot_feed, c.plunge_feed);
                if let Some(step) = &mut p.step_down {
                    *step = c.slot_step_down;
                }
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
            Operation::Relief(r) => {
                (r.feed, r.plunge_feed) = (c.feed, c.plunge_feed);
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
            Operation::Chamfer(p) => {
                (c.slot_feed, c.plunge_feed) = (p.feed, p.plunge_feed);
                if let Some(step) = p.step_down {
                    c.slot_step_down = step;
                }
            }
            Operation::Flute(p) => {
                (c.slot_feed, c.plunge_feed) = (p.feed, p.plunge_feed);
                if let Some(step) = p.step_down {
                    c.slot_step_down = step;
                }
            }
            Operation::Pattern(p) => {
                (c.slot_feed, c.plunge_feed) = (p.feed, p.plunge_feed);
                if let Some(step) = p.step_down {
                    c.slot_step_down = step;
                }
            }
            Operation::Prism(p) => {
                (c.slot_feed, c.plunge_feed) = (p.feed, p.plunge_feed);
                if let Some(step) = p.step_down {
                    c.slot_step_down = step;
                }
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
            Operation::Relief(r) => {
                (c.feed, c.plunge_feed) = (r.feed, r.plunge_feed);
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
            Operation::Chamfer(op) => &op.name,
            Operation::Flute(op) => &op.name,
            Operation::Pattern(op) => &op.name,
            Operation::Prism(op) => &op.name,
            Operation::Holes(op) => &op.name,
            Operation::Surface(op) => &op.name,
            Operation::Relief(op) => &op.name,
        }
    }

    pub fn name_mut(&mut self) -> &mut String {
        match self {
            Operation::Profile(op) => &mut op.name,
            Operation::Pocket(op) => &mut op.name,
            Operation::Vcarve(op) => &mut op.name,
            Operation::Path(op) => &mut op.name,
            Operation::Chamfer(op) => &mut op.name,
            Operation::Flute(op) => &mut op.name,
            Operation::Pattern(op) => &mut op.name,
            Operation::Prism(op) => &mut op.name,
            Operation::Holes(op) => &mut op.name,
            Operation::Surface(op) => &mut op.name,
            Operation::Relief(op) => &mut op.name,
        }
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        match self {
            Operation::Profile(op) => op.enabled = enabled,
            Operation::Pocket(op) => op.enabled = enabled,
            Operation::Vcarve(op) => op.enabled = enabled,
            Operation::Path(op) => op.enabled = enabled,
            Operation::Chamfer(op) => op.enabled = enabled,
            Operation::Flute(op) => op.enabled = enabled,
            Operation::Pattern(op) => op.enabled = enabled,
            Operation::Prism(op) => op.enabled = enabled,
            Operation::Holes(op) => op.enabled = enabled,
            Operation::Surface(op) => op.enabled = enabled,
            Operation::Relief(op) => op.enabled = enabled,
        }
    }

    pub fn enabled(&self) -> bool {
        match self {
            Operation::Profile(op) => op.enabled,
            Operation::Pocket(op) => op.enabled,
            Operation::Vcarve(op) => op.enabled,
            Operation::Path(op) => op.enabled,
            Operation::Chamfer(op) => op.enabled,
            Operation::Flute(op) => op.enabled,
            Operation::Pattern(op) => op.enabled,
            Operation::Prism(op) => op.enabled,
            Operation::Holes(op) => op.enabled,
            Operation::Surface(op) => op.enabled,
            Operation::Relief(op) => op.enabled,
        }
    }

    /// The id of its part.
    pub fn part(&self) -> u64 {
        match self {
            Operation::Profile(op) => op.part,
            Operation::Pocket(op) => op.part,
            Operation::Vcarve(op) => op.part,
            Operation::Path(op) => op.part,
            Operation::Chamfer(op) => op.part,
            Operation::Flute(op) => op.part,
            Operation::Pattern(op) => op.part,
            Operation::Prism(op) => op.part,
            Operation::Holes(op) => op.part,
            Operation::Surface(op) => op.part,
            Operation::Relief(op) => op.part,
        }
    }

    pub fn set_part(&mut self, part: u64) {
        match self {
            Operation::Profile(op) => op.part = part,
            Operation::Pocket(op) => op.part = part,
            Operation::Vcarve(op) => op.part = part,
            Operation::Path(op) => op.part = part,
            Operation::Chamfer(op) => op.part = part,
            Operation::Flute(op) => op.part = part,
            Operation::Pattern(op) => op.part = part,
            Operation::Prism(op) => op.part = part,
            Operation::Holes(op) => op.part = part,
            Operation::Surface(op) => op.part = part,
            Operation::Relief(op) => op.part = part,
        }
    }

    /// The library material its feeds come from.
    pub fn material(&self) -> Option<&MaterialRef> {
        match self {
            Operation::Profile(op) => op.material.as_ref(),
            Operation::Pocket(op) => op.material.as_ref(),
            Operation::Vcarve(op) => op.material.as_ref(),
            Operation::Path(op) => op.material.as_ref(),
            Operation::Chamfer(op) => op.material.as_ref(),
            Operation::Flute(op) => op.material.as_ref(),
            Operation::Pattern(op) => op.material.as_ref(),
            Operation::Prism(op) => op.material.as_ref(),
            Operation::Holes(op) => op.material.as_ref(),
            Operation::Surface(op) => op.material.as_ref(),
            Operation::Relief(op) => op.material.as_ref(),
        }
    }

    pub fn set_material(&mut self, material: Option<MaterialRef>) {
        match self {
            Operation::Profile(op) => op.material = material,
            Operation::Pocket(op) => op.material = material,
            Operation::Vcarve(op) => op.material = material,
            Operation::Path(op) => op.material = material,
            Operation::Chamfer(op) => op.material = material,
            Operation::Flute(op) => op.material = material,
            Operation::Pattern(op) => op.material = material,
            Operation::Prism(op) => op.material = material,
            Operation::Holes(op) => op.material = material,
            Operation::Surface(op) => op.material = material,
            Operation::Relief(op) => op.material = material,
        }
    }

    pub fn tool(&self) -> i64 {
        match self {
            Operation::Profile(op) => op.tool,
            Operation::Pocket(op) => op.tool,
            Operation::Vcarve(op) => op.tool,
            Operation::Path(op) => op.tool,
            Operation::Chamfer(op) => op.tool,
            Operation::Flute(op) => op.tool,
            Operation::Pattern(op) => op.tool,
            Operation::Prism(op) => op.tool,
            Operation::Holes(op) => op.tool,
            Operation::Surface(op) => op.tool,
            Operation::Relief(op) => op.tool,
        }
    }

    pub fn set_tool(&mut self, tool: i64) {
        match self {
            Operation::Profile(op) => op.tool = tool,
            Operation::Pocket(op) => op.tool = tool,
            Operation::Vcarve(op) => op.tool = tool,
            Operation::Path(op) => op.tool = tool,
            Operation::Chamfer(op) => op.tool = tool,
            Operation::Flute(op) => op.tool = tool,
            Operation::Pattern(op) => op.tool = tool,
            Operation::Prism(op) => op.tool = tool,
            Operation::Holes(op) => op.tool = tool,
            Operation::Surface(op) => op.tool = tool,
            Operation::Relief(op) => op.tool = tool,
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
            | Operation::Chamfer(_)
            | Operation::Flute(_)
            | Operation::Pattern(_)
            | Operation::Prism(_)
            | Operation::Holes(_)
            | Operation::Surface(_)
            | Operation::Relief(_) => {}
        }
    }

    /// Every project tool it cuts with or names, each once: its own first, then a rest
    /// pocket's larger tool, its rest passes', floor pass's, roughing and smoothing passes'.
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
            Operation::Relief(r) => {
                if let Some(pass) = &r.roughing {
                    push(pass.tool);
                }
                if let Some(pass) = &r.smoothing {
                    push(pass.tool);
                }
            }
            Operation::Profile(_)
            | Operation::Path(_)
            | Operation::Chamfer(_)
            | Operation::Flute(_)
            | Operation::Pattern(_)
            | Operation::Prism(_)
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
            Operation::Relief(r) => {
                if let Some(pass) = &mut r.roughing {
                    pass.tool = new(pass.tool);
                }
                if let Some(pass) = &mut r.smoothing {
                    pass.tool = new(pass.tool);
                }
            }
            Operation::Profile(_)
            | Operation::Path(_)
            | Operation::Chamfer(_)
            | Operation::Flute(_)
            | Operation::Pattern(_)
            | Operation::Prism(_)
            | Operation::Holes(_)
            | Operation::Surface(_) => {}
        }
    }

    /// Set the end depth, mm down from the stock top. A chamfer has none: its depth follows
    /// from its width and its bit.
    pub fn set_end_depth(&mut self, depth: f64) {
        match self {
            Operation::Chamfer(_) => {}
            Operation::Flute(op) => op.end_depth = depth,
            Operation::Pattern(op) => op.end_depth = depth,
            Operation::Prism(_) => {}
            Operation::Profile(op) => op.end_depth = depth,
            Operation::Pocket(op) => op.end_depth = depth,
            Operation::Vcarve(op) => op.end_depth = depth,
            Operation::Path(op) => op.end_depth = depth,
            Operation::Holes(op) => op.end_depth = depth,
            Operation::Surface(op) => op.end_depth = depth,
            Operation::Relief(op) => op.end_depth = depth,
        }
    }

    /// (start_depth, end_depth), mm down from the stock top. A chamfer's end depth comes
    /// from its width and its tool among `tools`; without a V-bit with an angle it is taken
    /// as a 90° bit's, which validation refuses anyway (docs/specs/chamfer.md §4).
    pub fn depths(&self, tools: &[Tool]) -> (f64, f64) {
        match self {
            Operation::Flute(op) => (op.start_depth, op.end_depth),
            Operation::Pattern(op) => (op.start_depth, op.end_depth),
            Operation::Prism(op) => (
                op.start_depth,
                op.start_depth + op.deepest(find_tool(tools, op.tool)),
            ),
            Operation::Chamfer(op) => {
                let depth = find_tool(tools, op.tool)
                    .and_then(|t| op.depth_with(t))
                    .unwrap_or(op.width);
                (op.start_depth, op.start_depth + depth)
            }
            Operation::Profile(op) => (op.start_depth, op.end_depth),
            Operation::Pocket(op) => (op.start_depth, op.end_depth),
            Operation::Vcarve(op) => (op.start_depth, op.end_depth),
            Operation::Path(op) => (op.start_depth, op.end_depth),
            Operation::Holes(op) => (op.start_depth, op.end_depth),
            Operation::Surface(op) => (op.start_depth, op.end_depth),
            Operation::Relief(op) => (op.start_depth, op.end_depth),
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
            Operation::Vcarve(_)
            | Operation::Chamfer(_)
            | Operation::Flute(_)
            | Operation::Pattern(_)
            | Operation::Prism(_)
            | Operation::Surface(_)
            | Operation::Relief(_) => None,
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
            Operation::Vcarve(_)
            | Operation::Chamfer(_)
            | Operation::Flute(_)
            | Operation::Pattern(_)
            | Operation::Prism(_)
            | Operation::Surface(_)
            | Operation::Relief(_) => None,
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
            Operation::Vcarve(_)
            | Operation::Chamfer(_)
            | Operation::Flute(_)
            | Operation::Pattern(_)
            | Operation::Prism(_)
            | Operation::Surface(_)
            | Operation::Relief(_) => None,
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
            Operation::Chamfer(p) => p.entry,
            // The ramps are its entry (docs/specs/fluting.md §4).
            Operation::Flute(_) => Entry::Plunge {},
            Operation::Pattern(_) => Entry::Plunge {},
            Operation::Prism(_) => Entry::Plunge {},
            // Holes go down their own way (docs/specs/holes.md §3, §4).
            Operation::Holes(_) => Entry::Plunge {},
            // Each depth starts beside the stock, in the air (docs/specs/surfacing.md §3).
            Operation::Surface(_) => Entry::Plunge {},
            // Each line starts at its first point (docs/specs/stl-relief.md §5).
            Operation::Relief(_) => Entry::Plunge {},
        }
    }

    pub fn starts(&self) -> &[[f64; 2]] {
        match self {
            Operation::Profile(p) => &p.starts,
            Operation::Pocket(p) => &p.starts,
            Operation::Vcarve(_) => &[],
            Operation::Path(_)
            | Operation::Chamfer(_)
            | Operation::Flute(_)
            | Operation::Pattern(_)
            | Operation::Prism(_)
            | Operation::Holes(_)
            | Operation::Surface(_)
            | Operation::Relief(_) => &[],
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
            | Operation::Chamfer(_)
            | Operation::Flute(_)
            | Operation::Pattern(_)
            | Operation::Prism(_)
            | Operation::Holes(_)
            | Operation::Surface(_)
            | Operation::Relief(_) => None,
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
            | Operation::Chamfer(_)
            | Operation::Flute(_)
            | Operation::Pattern(_)
            | Operation::Prism(_)
            | Operation::Holes(_)
            | Operation::Surface(_)
            | Operation::Relief(_) => None,
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
            | Operation::Chamfer(_)
            | Operation::Flute(_)
            | Operation::Pattern(_)
            | Operation::Prism(_)
            | Operation::Holes(_)
            | Operation::Surface(_)
            | Operation::Relief(_) => None,
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
            // The bit's flank on the line, its tip in the waste (docs/specs/chamfer.md §2).
            Operation::Chamfer(_) => 0.0,
            Operation::Flute(_) => 0.0,
            Operation::Pattern(_) => 0.0,
            Operation::Prism(_) => 0.0,
            // Cut to the drawn size (docs/specs/holes.md decision 9).
            Operation::Holes(_) => 0.0,
            // The whole stock: no wall to clear.
            Operation::Surface(_) => 0.0,
            // The tool's centre within the area; the drop cutter keeps it off the model.
            Operation::Relief(_) => 0.0,
        }
    }

    pub fn select(&self) -> &Selector {
        match self {
            Operation::Profile(op) => &op.select,
            Operation::Pocket(op) => &op.select,
            Operation::Vcarve(op) => &op.select,
            Operation::Path(op) => &op.select,
            Operation::Chamfer(op) => &op.select,
            Operation::Flute(op) => &op.select,
            Operation::Pattern(op) => &op.select,
            Operation::Prism(op) => &op.select,
            Operation::Holes(op) => &op.select,
            Operation::Surface(_) => &NO_SELECTOR,
            Operation::Relief(op) => &op.select,
        }
    }

    /// None for an operation that selects no shapes (a surface).
    pub fn select_mut(&mut self) -> Option<&mut Selector> {
        match self {
            Operation::Profile(op) => Some(&mut op.select),
            Operation::Pocket(op) => Some(&mut op.select),
            Operation::Vcarve(op) => Some(&mut op.select),
            Operation::Path(op) => Some(&mut op.select),
            Operation::Chamfer(op) => Some(&mut op.select),
            Operation::Flute(op) => Some(&mut op.select),
            Operation::Pattern(op) => Some(&mut op.select),
            Operation::Prism(op) => Some(&mut op.select),
            Operation::Holes(op) => Some(&mut op.select),
            Operation::Relief(op) => Some(&mut op.select),
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
