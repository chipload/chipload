// SPDX-License-Identifier: GPL-3.0-or-later
//! Operations: profile, pocket, V-carve, follow path, holes, surface, and what they share
//! (tabs, corners, entries, leads, passes).

use super::*;

mod entry;
mod moulding_holes;
mod profile_pocket;
mod surface_relief;
mod vcarve_path;

pub use entry::*;
pub use moulding_holes::*;
pub use profile_pocket::*;
pub use surface_relief::*;
pub use vcarve_path::*;

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
    /// Moulding: a profile swept along a path, in passes along it.
    // Spec: docs/specs/moulding.md.
    Moulding(MouldingOp),
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
            Operation::Moulding(p) => {
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
            Operation::Moulding(p) => {
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
            Operation::Moulding(op) => &op.name,
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
            Operation::Moulding(op) => &mut op.name,
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
            Operation::Moulding(op) => op.enabled = enabled,
            Operation::Holes(op) => op.enabled = enabled,
            Operation::Surface(op) => op.enabled = enabled,
            Operation::Relief(op) => op.enabled = enabled,
        }
    }

    /// The feed the operation cuts at, mm/min: every kind has one.
    pub fn feed(&self) -> f64 {
        match self {
            Operation::Profile(op) => op.feed,
            Operation::Pocket(op) => op.feed,
            Operation::Vcarve(op) => op.feed,
            Operation::Path(op) => op.feed,
            Operation::Chamfer(op) => op.feed,
            Operation::Flute(op) => op.feed,
            Operation::Pattern(op) => op.feed,
            Operation::Prism(op) => op.feed,
            Operation::Moulding(op) => op.feed,
            Operation::Holes(op) => op.feed,
            Operation::Surface(op) => op.feed,
            Operation::Relief(op) => op.feed,
        }
    }

    /// The feed the operation plunges at, mm/min: every kind has one.
    pub fn plunge_feed(&self) -> f64 {
        match self {
            Operation::Profile(op) => op.plunge_feed,
            Operation::Pocket(op) => op.plunge_feed,
            Operation::Vcarve(op) => op.plunge_feed,
            Operation::Path(op) => op.plunge_feed,
            Operation::Chamfer(op) => op.plunge_feed,
            Operation::Flute(op) => op.plunge_feed,
            Operation::Pattern(op) => op.plunge_feed,
            Operation::Prism(op) => op.plunge_feed,
            Operation::Moulding(op) => op.plunge_feed,
            Operation::Holes(op) => op.plunge_feed,
            Operation::Surface(op) => op.plunge_feed,
            Operation::Relief(op) => op.plunge_feed,
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
            Operation::Moulding(op) => op.enabled,
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
            Operation::Moulding(op) => op.part,
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
            Operation::Moulding(op) => op.part = part,
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
            Operation::Moulding(op) => op.material.as_ref(),
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
            Operation::Moulding(op) => op.material = material,
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
            Operation::Moulding(op) => op.tool,
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
            Operation::Moulding(op) => op.tool = tool,
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
            | Operation::Moulding(_)
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
            | Operation::Moulding(_)
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
            | Operation::Moulding(_)
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
            Operation::Moulding(_) => {}
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
            // A drawn profile's depth is known from the drawing, at planning.
            Operation::Moulding(op) => (
                op.start_depth,
                op.start_depth + op.profile.depth().unwrap_or(0.0),
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
            | Operation::Moulding(_)
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
            | Operation::Moulding(_)
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
            | Operation::Moulding(_)
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
            Operation::Moulding(_) => Entry::Plunge {},
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
            | Operation::Moulding(_)
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
            | Operation::Moulding(_)
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
            | Operation::Moulding(_)
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
            | Operation::Moulding(_)
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
            Operation::Moulding(_) => 0.0,
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
            Operation::Moulding(op) => &op.select,
            Operation::Holes(op) => &op.select,
            Operation::Surface(op) => &op.select,
            Operation::Relief(op) => &op.select,
        }
    }

    /// None for an operation that selects no shapes (none since schema 40).
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
            Operation::Moulding(op) => Some(&mut op.select),
            Operation::Holes(op) => Some(&mut op.select),
            Operation::Relief(op) => Some(&mut op.select),
            Operation::Surface(op) => Some(&mut op.select),
        }
    }

    /// Whether it cuts shapes of the drawing; a board surface faces the stock instead.
    pub fn selects_shapes(&self) -> bool {
        !self.is_board_surface()
    }

    /// A surface with no shapes: the whole board, cut once, whatever its part's instances
    /// (docs/specs/surface-levels.md §2).
    pub fn is_board_surface(&self) -> bool {
        matches!(self, Operation::Surface(op) if op.select.is_empty())
    }
}
