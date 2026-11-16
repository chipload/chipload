// SPDX-License-Identifier: GPL-3.0-or-later
//! `chipload op …`: operations added, changed, moved between parts, and their rest and floor
//! passes.

use super::*;
use chipload_job::{Operation, RampShape};
use clap::Args;

mod add;
mod edit;
mod passes;
mod summary;

use add::{AddArgs, AddSurfaceArgs};
use edit::{CornerArgs, CornersArgs, PartArgs, TabArgs, TabsArgs, ToolArgs};
use passes::{
    AddFloorArgs, AddInlayArgs, AddRestArgs, AddRoughingArgs, AddSmoothingArgs, MakeRestPassArgs,
    RemoveFloorArgs, RemoveRoughingArgs, RemoveSmoothingArgs, RestArgs, RestPassArgs,
};
pub(crate) use summary::describe;

#[derive(Subcommand)]
// Parsed once from the command line: the size of `Add`'s many options doesn't matter.
#[allow(clippy::large_enum_variant)]
pub(crate) enum OpAction {
    /// Add an operation for shapes of the drawing, with the defaults the app gives
    /// (docs/specs/styles.md §2): a profile of outlines goes outside and through with tabs,
    /// of holes inside and through; of both, two profiles, the holes first. Feeds and step
    /// downs come from the tool's cutting data. Added at the end of the active part.
    Add(AddArgs),
    /// Set the tool an operation cuts with (docs/specs/tool-per-operation.md §7). The tool
    /// must be in the project: copy one in from your library with `tool use`.
    Tool(ToolArgs),
    /// Add a rest pass to a pocket (docs/specs/rest-passes.md): a smaller endmill that clears
    /// what the pocket's tool (or its last pass's) leaves, in the pocket's shapes and depths.
    /// It is cut after the run of pockets with the pocket's tool.
    AddRest(AddRestArgs),
    /// Give a V-carve a floor pass (docs/specs/vcarve-wide-strokes.md §4): an end mill that
    /// clears the floor between the walls of strokes too wide to carve from their centre
    /// line, cut before the V-carve. Replaces the floor pass it had.
    AddFloor(AddFloorArgs),
    /// Make a V-carve inlay of closed shapes (docs/specs/vcarve-inlay.md): a pocket
    /// V-carved in the active part, and in a new setup, the plug board, the plug: the rest
    /// of that board carved, mirrored left to right, so the shapes stand as a tapered plug
    /// that is glued face down into the pocket.
    AddInlay(AddInlayArgs),
    /// Give a relief a roughing pass (docs/specs/stl-relief.md §14): an end mill or a plain
    /// ball nose that takes the material down in levels before the relief, leaving an
    /// allowance on the model. Replaces the roughing pass it had.
    AddRoughing(AddRoughingArgs),
    /// Give a relief a smoothing pass (docs/specs/stl-relief.md §17), after its roughing
    /// pass: the surface followed once, lines across the roughing's, taking the steps the
    /// roughing leaves on slopes. Replaces the smoothing pass it had.
    AddSmoothing(AddSmoothingArgs),
    /// Take a relief's smoothing pass away.
    RemoveSmoothing(RemoveSmoothingArgs),
    /// Take a relief's roughing pass away, and its smoothing pass with it.
    RemoveRoughing(RemoveRoughingArgs),
    /// Take a V-carve's floor pass away (docs/specs/vcarve-wide-strokes.md §4).
    RemoveFloor(RemoveFloorArgs),
    /// Add a surface (docs/specs/surfacing.md): the whole stock taken down flat with a wide
    /// endmill, in straight passes back and forth, first in the active part. Flattens the
    /// spoilboard or faces the stock.
    AddSurface(AddSurfaceArgs),
    /// A pocket's rest passes (docs/specs/rest-passes.md): without --pass, list them; with
    /// it, change, turn off or on, or remove one.
    RestPass(RestPassArgs),
    /// Make a rest pocket that copies an earlier pocket (same shapes, depths, clearance and
    /// direction, rest after its tool) a rest pass of that pocket (docs/specs/rest-passes.md
    /// §5). It is then cut after the run of pockets it belongs to.
    MakeRestPass(MakeRestPassArgs),
    /// Move an operation into another part, at the end of its operations
    /// (docs/specs/parts.md §7). Operations are numbered again in the new order.
    Part(PartArgs),
    /// List a profile's or pocket's corners on the tool's side, with what each gets (round,
    /// dogbone, T-bone along the longer or shorter wall) and whether that is the corner's
    /// own choice (docs/specs/dogbone-per-corner.md).
    Corners(CornersArgs),
    /// Give one corner of a profile or pocket its own bone, whatever the operation's corners
    /// say (docs/specs/dogbone-per-corner.md): `--bone none` keeps it round. `op corners`
    /// lists the corners.
    Corner(CornerArgs),
    /// List a profile's tabs: each contour's, with their centres on the tool path and
    /// whether they were placed by hand (docs/specs/tabs-by-hand.md). The job is planned.
    /// With --along or --around, first choose where contours without tabs placed by hand
    /// get theirs (docs/specs/tabs-along-grain.md).
    Tabs(TabsArgs),
    /// Place a profile's tabs by hand (docs/specs/tabs-by-hand.md): add a tab at a point, or
    /// remove the one there. The first on a contour keeps that contour's other tabs where
    /// Chipload put them. `--even` spaces one contour's tabs evenly again, `--reset` all.
    Tab(TabArgs),
    /// Make a pocket clear only what a larger tool leaves (docs/specs/rest-machining.md).
    Rest(RestArgs),
}

/// What to cut, for `op add`, `op add-inlay` and `style use`: a layer, shapes, a DXF, a
/// group, a text, a picture or a 3D model's outline.
#[derive(Args)]
pub(crate) struct Shapes {
    /// Every shape on this layer (with --dxf: on that DXF's layer).
    #[arg(long)]
    layer: Option<String>,
    /// These shapes, by id (as `inspect` lists them).
    #[arg(long, num_args = 1.., value_delimiter = ',')]
    shapes: Option<Vec<u64>>,
    /// All of this DXF or SVG (an id from `dxf list`).
    #[arg(long, conflicts_with_all = ["group", "text", "image"])]
    dxf: Option<u64>,
    /// All of this group.
    #[arg(long, conflicts_with_all = ["text", "image"])]
    group: Option<u64>,
    /// All of this text (an id from `text list`).
    #[arg(long, conflicts_with = "image")]
    text: Option<u64>,
    /// All of this traced picture (an id from `image list`).
    #[arg(long, conflicts_with = "model")]
    image: Option<u64>,
    /// The outline of this 3D model (an id from `model list`): a relief of it carves within
    /// it (docs/specs/stl-relief.md §4).
    #[arg(long)]
    model: Option<u64>,
}

impl Shapes {
    /// The selector; refused when it names nothing.
    pub(crate) fn selector(self) -> Result<chipload_job::Selector, &'static str> {
        let source = self
            .dxf
            .map(chipload_job::Source::Dxf)
            .or(self.group.map(chipload_job::Source::Group))
            .or(self.text.map(chipload_job::Source::Text))
            .or(self.image.map(chipload_job::Source::Image))
            .or(self.model.map(chipload_job::Source::Model));
        let select = chipload_job::Selector {
            layer: self.layer,
            shapes: self.shapes,
            source,
            exclude: Vec::new(),
        };
        if select.is_empty() {
            return Err(
                "say what to cut: --layer, --shapes, --dxf, --group, --text, --image or --model",
            );
        }
        Ok(select)
    }
}

/// An axis of the drawing, for `op tabs --along`.
#[derive(Clone, Copy, ValueEnum)]
pub(crate) enum AxisArg {
    X,
    Y,
}

/// The kinds `op add` adds (a surface has `op add-surface`).
#[derive(Clone, Copy, ValueEnum)]
pub(crate) enum KindArg {
    Profile,
    Pocket,
    Vcarve,
    Path,
    /// A V-bit breaks the top edge of closed shapes (docs/specs/chamfer.md): --width.
    Chamfer,
    /// A groove along open shapes that ramps in and out at their ends
    /// (docs/specs/fluting.md): --depth, --ramp.
    Flute,
    /// Grooves of lines, cross-hatch, rings or waves inside closed shapes
    /// (docs/specs/fill-pattern.md): --pattern, --spacing, --angle.
    Pattern,
    /// Closed shapes left standing as roofs with a ridge along their centre
    /// (docs/specs/prism.md): --max-depth.
    Prism,
    /// A profile swept along paths (docs/specs/moulding.md): --profile, --radius, --side,
    /// --ends.
    Moulding,
    Holes,
    /// A 3D model carved from the top (docs/specs/stl-relief.md): with --model, of that
    /// model within its outline.
    Relief,
}

/// A pattern's settings for `op add pattern` (docs/specs/fill-pattern.md §8).
#[derive(Args, Clone, Copy)]
pub(crate) struct PatternArgs {
    /// A pattern's kind, in place of lines.
    #[arg(id = "pattern_kind", long = "pattern")]
    pattern_kind: Option<PatternKindArg>,
    /// A pattern's spacing, groove centre to centre, in place of twice the groove's width.
    #[arg(long, value_parser = crate::lengths::length)]
    spacing: Option<f64>,
    /// A pattern's lines' angle from the X axis, degrees counter-clockwise; 45 if left out.
    #[arg(long, allow_negative_numbers = true)]
    angle: Option<f64>,
    /// A cross-hatch's angle between its two sets of lines, degrees (90 squares, 60
    /// diamonds); 90 if left out.
    #[arg(long)]
    between: Option<f64>,
    /// A wave pattern's wavelength; 4 spacings if left out.
    #[arg(long, value_parser = crate::lengths::length)]
    wavelength: Option<f64>,
    /// A wave pattern's height, top to bottom; one spacing if left out.
    #[arg(long, value_parser = crate::lengths::length)]
    height: Option<f64>,
    /// The grooves run up to the shapes' lines, centred on them, instead of staying inside.
    #[arg(long)]
    on_line: bool,
    /// Dashes', herringbone's or a sunburst's strokes' length; three groove widths (at
    /// least 4 mm) if left out.
    #[arg(long, value_parser = crate::lengths::length)]
    length: Option<f64>,
    /// The gap between dashes; half a stroke if left out.
    #[arg(long, value_parser = crate::lengths::length)]
    gap: Option<f64>,
    /// Scales' radius; twice the spacing if left out.
    #[arg(long, value_parser = crate::lengths::length)]
    radius: Option<f64>,
    /// A sunburst's strokes' distance round its rings; a stroke's length if left out.
    #[arg(long, value_parser = crate::lengths::length)]
    pitch: Option<f64>,
    /// Stroke patterns: strokes the shape's edge cuts are clipped, not left out.
    #[arg(long)]
    clip: bool,
}

impl PatternArgs {
    /// Whether --radius was given, for a moulding's profile.
    fn radius_given(&self) -> Option<f64> {
        self.radius
    }

    /// Whether a pattern option other than --radius was given.
    fn given_except_radius(&self) -> bool {
        let mut rest = *self;
        rest.radius = None;
        rest.given()
    }

    fn given(&self) -> bool {
        self.pattern_kind.is_some()
            || self.spacing.is_some()
            || self.angle.is_some()
            || self.between.is_some()
            || self.wavelength.is_some()
            || self.height.is_some()
            || self.on_line
            || self.length.is_some()
            || self.gap.is_some()
            || self.radius.is_some()
            || self.pitch.is_some()
            || self.clip
    }

    /// The settings over the new pattern's defaults, a new kind's from `tool`.
    fn apply(&self, op: &mut chipload_job::PatternOp, tool: Option<&chipload_job::Tool>) {
        use chipload_job::FillPattern;
        use chipload_pipeline::new_ops::{PatternKind, switched_pattern};
        // The spacing first, so the kind's defaults follow it (scales' radius).
        if let Some(spacing) = self.spacing {
            set_spacing(&mut op.pattern, spacing);
        }
        if let Some(kind) = self.pattern_kind {
            let kind = match kind {
                PatternKindArg::Lines => PatternKind::Lines,
                PatternKindArg::Cross => PatternKind::Cross,
                PatternKindArg::Rings => PatternKind::Rings,
                PatternKindArg::Waves => PatternKind::Waves,
                PatternKindArg::Dashes => PatternKind::Dashes,
                PatternKindArg::Herringbone => PatternKind::Herringbone,
                PatternKindArg::Scales => PatternKind::Scales,
                PatternKindArg::Sunburst => PatternKind::Sunburst,
            };
            *op = switched_pattern(op, kind, tool);
        }
        let given_angle = self.angle.map(f64::to_radians);
        match &mut op.pattern {
            FillPattern::Lines { angle, .. } | FillPattern::Herringbone { angle, .. } => {
                *angle = given_angle.unwrap_or(*angle);
            }
            FillPattern::Cross { angle, between, .. } => {
                *angle = given_angle.unwrap_or(*angle);
                *between = self.between.map_or(*between, f64::to_radians);
            }
            FillPattern::Waves {
                angle,
                wavelength,
                height,
                ..
            } => {
                *angle = given_angle.unwrap_or(*angle);
                *wavelength = self.wavelength.unwrap_or(*wavelength);
                *height = self.height.unwrap_or(*height);
            }
            FillPattern::Rings { .. } => {}
            FillPattern::Dashes {
                angle, length, gap, ..
            } => {
                *angle = given_angle.unwrap_or(*angle);
                *length = self.length.unwrap_or(*length);
                *gap = self.gap.unwrap_or(*gap);
            }
            FillPattern::Scales { angle, radius, .. } => {
                *angle = given_angle.unwrap_or(*angle);
                *radius = self.radius.unwrap_or(*radius);
            }
            FillPattern::Sunburst { length, pitch, .. } => {
                *length = self.length.unwrap_or(*length);
                *pitch = self.pitch.unwrap_or(*pitch);
            }
        }
        if let FillPattern::Herringbone { length, .. } = &mut op.pattern {
            *length = self.length.unwrap_or(*length);
        }
        if self.clip {
            match &mut op.pattern {
                FillPattern::Dashes { whole, .. }
                | FillPattern::Herringbone { whole, .. }
                | FillPattern::Scales { whole, .. }
                | FillPattern::Sunburst { whole, .. } => *whole = false,
                _ => {}
            }
        }
        if self.on_line {
            op.inside = false;
        }
    }
}

/// A pattern's kind, for `op add pattern --pattern`.
#[derive(Clone, Copy, ValueEnum)]
pub(crate) enum PatternKindArg {
    Lines,
    Cross,
    Rings,
    Waves,
    Dashes,
    Herringbone,
    Scales,
    Sunburst,
}

/// The pattern's spacing set to `spacing`.
fn set_spacing(pattern: &mut chipload_job::FillPattern, spacing: f64) {
    use chipload_job::FillPattern;
    match pattern {
        FillPattern::Lines { spacing: s, .. }
        | FillPattern::Cross { spacing: s, .. }
        | FillPattern::Rings { spacing: s }
        | FillPattern::Waves { spacing: s, .. }
        | FillPattern::Dashes { spacing: s, .. }
        | FillPattern::Herringbone { spacing: s, .. }
        | FillPattern::Scales { spacing: s, .. }
        | FillPattern::Sunburst { spacing: s, .. } => *s = spacing,
    }
}

/// A flute's ramps, for `op add flute --ramp-shape`.
#[derive(Clone, Copy, ValueEnum)]
pub(crate) enum RampShapeArg {
    Straight,
    Curved,
}

/// A chamfer's waste side, for `op add chamfer --side`.
#[derive(Clone, Copy, ValueEnum)]
pub(crate) enum EdgeSideArg {
    Outside,
    Inside,
    /// A moulding's profile left of an open path as drawn.
    Left,
    /// A moulding's profile right of an open path as drawn.
    Right,
}

/// A moulding's settings for `op add moulding` (docs/specs/moulding.md §9).
#[derive(Args, Clone, Copy)]
pub(crate) struct MouldingArgs {
    /// A moulding's profile, in place of a 6 mm cove: with --radius, or --width and --depth
    /// for an ogee, or --profile-shape for a drawn one.
    #[arg(long)]
    profile: Option<ProfileArg>,
    /// The drawn open shape a moulding's profile is read from (its id).
    #[arg(long)]
    profile_shape: Option<u64>,
    /// How a moulding ends on open paths: square, round, or taper (--taper long).
    #[arg(long)]
    ends: Option<EndsArg>,
    /// A tapered end's length; 30 mm if left out.
    #[arg(long, value_parser = crate::lengths::length)]
    taper: Option<f64>,
    /// The distance between a moulding's passes across its profile.
    #[arg(long, value_parser = crate::lengths::length)]
    stepover: Option<f64>,
}

/// A moulding's profile, for `op add moulding --profile`.
#[derive(Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum ProfileArg {
    Cove,
    Roundover,
    Ogee,
    Bead,
    Flute,
    Shape,
}

/// A moulding's ends, for `op add moulding --ends`.
#[derive(Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum EndsArg {
    Square,
    Round,
    Taper,
}

/// A tapered end's length when none is given, mm.
const DEFAULT_TAPER_MM: f64 = 30.0;

impl MouldingArgs {
    fn given(&self) -> bool {
        self.profile.is_some()
            || self.profile_shape.is_some()
            || self.ends.is_some()
            || self.taper.is_some()
            || self.stepover.is_some()
    }

    /// The settings over a new moulding's defaults; `given` is (--radius, --width, --depth).
    fn apply(
        &self,
        op: &mut chipload_job::MouldingOp,
        (radius, width, depth): (Option<f64>, Option<f64>, Option<f64>),
        side: Option<EdgeSideArg>,
        shape: Option<RampShapeArg>,
    ) -> Result<(), String> {
        use chipload_job::{MouldEnds, MouldProfile, MouldSide};
        let kind = match (self.profile, self.profile_shape) {
            (Some(kind), _) => kind,
            (None, Some(_)) => ProfileArg::Shape,
            (None, None) => ProfileArg::Cove,
        };
        let r = radius.unwrap_or(crate::ops::DEFAULT_MOULDING_RADIUS_MM);
        op.profile = match kind {
            ProfileArg::Cove => MouldProfile::Cove { radius: r },
            ProfileArg::Roundover => MouldProfile::Roundover { radius: r },
            ProfileArg::Bead => MouldProfile::Bead { radius: r },
            ProfileArg::Flute => MouldProfile::Flute { radius: r },
            ProfileArg::Ogee => MouldProfile::Ogee {
                width: width.unwrap_or(2.0 * r),
                depth: depth.unwrap_or(r),
            },
            ProfileArg::Shape => match self.profile_shape {
                Some(shape) => MouldProfile::Shape { shape },
                None => return Err("--profile shape needs --profile-shape ID".to_string()),
            },
        };
        if kind != ProfileArg::Ogee && (width.is_some() || depth.is_some()) {
            return Err("--width and --depth are for an ogee's profile".to_string());
        }
        match side {
            Some(EdgeSideArg::Left) => op.side = MouldSide::Left,
            Some(EdgeSideArg::Right) => op.side = MouldSide::Right,
            Some(EdgeSideArg::Inside) => op.side = MouldSide::Inside,
            Some(EdgeSideArg::Outside) => op.side = MouldSide::Outside,
            None => {}
        }
        let shape = match shape {
            Some(RampShapeArg::Curved) => RampShape::Curved,
            _ => RampShape::Straight,
        };
        op.ends = match (self.ends, self.taper) {
            (Some(EndsArg::Square), _) => MouldEnds::Square,
            (Some(EndsArg::Round), _) => MouldEnds::Round,
            (Some(EndsArg::Taper), length) | (None, length @ Some(_)) => MouldEnds::Taper {
                length: length.unwrap_or(DEFAULT_TAPER_MM),
                shape,
            },
            (None, None) => op.ends,
        };
        if let Some(stepover) = self.stepover {
            op.stepover = stepover;
        }
        Ok(())
    }
}

/// A new moulding's radius when --radius isn't given, mm (as the app's default).
const DEFAULT_MOULDING_RADIUS_MM: f64 = chipload_pipeline::new_ops::DEFAULT_MOULDING_RADIUS;

impl KindArg {
    fn kind(self) -> chipload_pipeline::new_ops::OpKind {
        use chipload_pipeline::new_ops::OpKind;
        match self {
            KindArg::Profile => OpKind::Profile,
            KindArg::Pocket => OpKind::Pocket,
            KindArg::Vcarve => OpKind::Vcarve,
            KindArg::Path => OpKind::Path,
            KindArg::Chamfer => OpKind::Chamfer,
            KindArg::Flute => OpKind::Flute,
            KindArg::Pattern => OpKind::Pattern,
            KindArg::Prism => OpKind::Prism,
            KindArg::Moulding => OpKind::Moulding,
            KindArg::Holes => OpKind::Holes,
            KindArg::Relief => OpKind::Relief,
        }
    }
}

#[derive(Clone, Copy, ValueEnum)]
pub(crate) enum BoneArg {
    None,
    Dogbone,
    Tbone,
}

#[derive(Clone, Copy, ValueEnum)]
pub(crate) enum WallArg {
    Longer,
    Shorter,
}

pub(crate) fn op(out: &mut Out, action: OpAction) -> ExitCode {
    match action {
        OpAction::Tool(args) => args.run(out),
        OpAction::RestPass(args) => args.run(out),
        OpAction::MakeRestPass(args) => args.run(out),
        OpAction::Add(args) => args.run(out),
        OpAction::AddSurface(args) => args.run(out),
        OpAction::AddRest(args) => args.run(out),
        OpAction::AddFloor(args) => args.run(out),
        OpAction::AddInlay(args) => args.run(out),
        OpAction::AddRoughing(args) => args.run(out),
        OpAction::RemoveRoughing(args) => args.run(out),
        OpAction::AddSmoothing(args) => args.run(out),
        OpAction::RemoveSmoothing(args) => args.run(out),
        OpAction::RemoveFloor(args) => args.run(out),
        OpAction::Part(args) => args.run(out),
        OpAction::Tabs(args) => args.run(out),
        OpAction::Tab(args) => args.run(out),
        OpAction::Corners(args) => args.run(out),
        OpAction::Corner(args) => args.run(out),
        OpAction::Rest(args) => args.run(out),
    }
}

/// What a corner gets, as `op corners` names it.
pub(crate) fn gets_name(gets: Option<chipload_toolpath::corners::Gets>) -> &'static str {
    use chipload_job::Wall;
    use chipload_toolpath::corners::Gets;
    match gets {
        None => "round",
        Some(Gets::Dogbone) => "dogbone",
        Some(Gets::Tbone(Wall::Longer)) => "T-bone along the longer wall",
        Some(Gets::Tbone(Wall::Shorter)) => "T-bone along the shorter wall",
    }
}

/// The operations among `refs`, 0-based, as `--json` always listed them.
pub(crate) fn operations_json(refs: &[chipload_job::OpRef]) -> Value {
    json!(
        refs.iter()
            .filter(|r| r.pass.is_none())
            .map(|r| r.operation)
            .collect::<Vec<_>>()
    )
}

/// The rest passes among `refs` (docs/specs/rest-passes.md): `{operation, pass}`, 0-based.
pub(crate) fn rest_passes_json(refs: &[chipload_job::OpRef]) -> Value {
    json!(
        refs.iter()
            .filter_map(|r| r.pass.map(|k| json!({"operation": r.operation, "pass": k})))
            .collect::<Vec<_>>()
    )
}

/// `op add`'s --depth and --through over the defaults.
fn set_depth(job: &Job, op: &mut Operation, depth: Option<f64>, through: bool) {
    // A relief's depth is its model's (docs/specs/stl-relief.md §3): `model set --depth`; a
    // chamfer's follows from its width (docs/specs/chamfer.md §2), a prism's from its shapes.
    if let Operation::Relief(_) | Operation::Chamfer(_) | Operation::Prism(_) = op {
        return;
    }
    if through {
        if let Some(t) = op.through_mut() {
            *t = Some(job.stock().max_overcut);
        }
    } else if let Some(depth) = depth {
        if let Some(t) = op.through_mut() {
            *t = None;
        }
        op.set_end_depth(depth);
    }
    let point = job.drill_point(op);
    op.follow_thickness(job.stock().thickness, point);
}
