// SPDX-License-Identifier: GPL-3.0-or-later
//! The operations' rules in `Job::validate`: their values, depths, selections and ways in.

use super::*;

impl Job {
    /// Values that must be more than 0, then whole numbers and ranges out of bounds.
    pub(super) fn value_problems(&self) -> Vec<Problem> {
        let mut values = Values::default();
        for (i, op) in self.operations.iter().enumerate() {
            values.at = format!("operations.{i}");
            values.operation(op);
        }
        values.positives.extend(values.counts);
        values.positives
    }

    /// Through cuts: how far below is a length of 0 or more, and the end depth follows the
    /// board's thickness.
    pub(super) fn through_problems(&self) -> Vec<Problem> {
        let mut problems = Vec::new();
        for (i, op) in self.operations.iter().enumerate() {
            let Some(below) = op.through() else {
                continue;
            };
            let thickness = self.setup_of(op).map_or(0.0, |s| s.stock.thickness);
            if !(below.is_finite() && below >= 0.0) {
                problems.push(Problem::at(
                    format!("operations.{i}.through"),
                    Message::new("job-must-be-0-or-more")
                        .arg("got", Value::LengthNumber(below, Digits::Shortest)),
                ));
            } else if (op.depths(&self.tools).1 - (thickness + below + self.drill_point(op))).abs()
                > DEPTH_TOL_MM
            {
                // A project read or edited follows the thickness (`follow_thickness`); one
                // that doesn't is refused rather than cut at an old depth.
                problems.push(Problem::at(
                    format!("operations.{i}.end_depth"),
                    Message::new("job-through-stale").arg(
                        "depth",
                        Value::Length(thickness + below + self.drill_point(op), Digits::Shortest),
                    ),
                ));
            }
        }
        problems
    }

    /// One operation's ranges: pattern and flute values, depths, clearance, surfaces and
    /// reliefs.
    pub(super) fn op_problems(&self, i: usize, op: &Operation) -> Vec<Problem> {
        let mut problems = Vec::new();
        match op {
            Operation::Pattern(f) => problems.extend(pattern_problems(i, f)),
            Operation::Flute(f) => problems.extend(flute_problems(i, f)),
            _ => {}
        }
        problems.extend(self.depth_problems(i, op));
        match op {
            Operation::Surface(f) => {
                let thickness = self.setup_of(op).map_or(0.0, |s| s.stock.thickness);
                problems.extend(surface_problems(i, f, thickness));
            }
            Operation::Relief(r) => problems.extend(self.relief_problems(i, r)),
            _ => {}
        }
        problems.extend(on_line_problems(i, op));
        problems
    }

    /// The start depth is 0 or more and above the end, the clearance a number.
    fn depth_problems(&self, i: usize, op: &Operation) -> Vec<Problem> {
        let mut problems = Vec::new();
        let (start, end) = op.depths(&self.tools);
        if !(start.is_finite() && start >= 0.0) {
            problems.push(Problem::at(
                format!("operations.{i}.start_depth"),
                Message::new("job-must-be-0-or-more")
                    .arg("got", Value::Length(start, Digits::Shortest)),
            ));
        } else if start >= end
            && !matches!(
                op,
                Operation::Chamfer(_) | Operation::Prism(_) | Operation::Moulding(_)
            )
        {
            // A chamfer's depth follows from its width, checked above.
            problems.push(Problem::at(
                format!("operations.{i}.start_depth"),
                Message::new("job-start-below-end")
                    .arg("end", Value::Length(end, Digits::Shortest))
                    .arg("got", Value::Length(start, Digits::Shortest)),
            ));
        }
        if !op.clearance().is_finite() {
            problems.push(Problem::at(
                format!("operations.{i}.clearance"),
                Message::new("job-must-be-number")
                    .arg("got", Value::LengthNumber(op.clearance(), Digits::Shortest)),
            ));
        }
        problems
    }

    /// A relief carves a model of the project, in lines no wider apart than 0.9 of the tool
    /// (docs/specs/stl-relief.md §5).
    fn relief_problems(&self, i: usize, r: &ReliefOp) -> Vec<Problem> {
        let mut problems = Vec::new();
        if !self.drawing.models.iter().any(|m| m.id == r.model) {
            problems.push(Problem::at(
                format!("operations.{i}.model"),
                Message::new("job-no-model").arg("id", r.model),
            ));
        }
        if r.stepover > RELIEF_MAX_STEPOVER {
            problems.push(Problem::at(
                format!("operations.{i}.stepover"),
                Message::new("job-relief-stepover")
                    .arg("most", Value::Number(RELIEF_MAX_STEPOVER, Digits::Shortest))
                    .arg("got", Value::Number(r.stepover, Digits::Shortest)),
            ));
        }
        if let Some(rough) = &r.roughing {
            if rough.stepover > RELIEF_MAX_STEPOVER {
                problems.push(Problem::at(
                    format!("operations.{i}.roughing.stepover"),
                    Message::new("job-relief-stepover")
                        .arg("most", Value::Number(RELIEF_MAX_STEPOVER, Digits::Shortest))
                        .arg("got", Value::Number(rough.stepover, Digits::Shortest)),
                ));
            }
            if !(rough.allowance.is_finite() && rough.allowance >= 0.0) {
                problems.push(Problem::at(
                    format!("operations.{i}.roughing.allowance"),
                    Message::new("job-relief-allowance")
                        .arg("got", Value::Length(rough.allowance, Digits::Shortest)),
                ));
            }
        }
        if let Some(smooth) = &r.smoothing {
            problems.extend(smoothing_problems(i, r, smooth));
        }
        if let Some(rim) = &r.rim {
            problems.extend(rim_problems(i, rim));
        }
        if !r.angle.is_finite() {
            problems.push(Problem::at(
                format!("operations.{i}.angle"),
                Message::new("job-must-be-number")
                    .arg("got", Value::Number(r.angle, Digits::Shortest)),
            ));
        }
        problems
    }

    /// What an operation selects: something, from a source the project has, and no shapes
    /// whose ids change when they are made again.
    pub(super) fn selection_problems(&self, i: usize, op: &Operation) -> Vec<Problem> {
        let mut problems = Vec::new();
        let selector = op.select();
        if selector.is_empty() {
            problems.push(Problem::at(
                format!("operations.{i}.select"),
                Message::new("job-selector-empty"),
            ));
        }
        if let Some(source) = selector.source {
            problems.extend(self.source_problems(i, selector, source));
        }
        // A group is resolved to shapes: naming shapes besides it is ambiguous.
        if selector.group().is_some() && selector.shapes.is_some() {
            problems.push(Problem::at(
                format!("operations.{i}.select"),
                Message::new("job-selector-one-source"),
            ));
        }
        problems.extend(self.renewed_id_problems(i, selector));
        problems.extend(self.centre_line_problems(i, op, selector));
        problems
    }

    /// The selector's source exists, and the shapes it leaves out are of it
    /// (docs/specs/leave-out-shapes.md §2, text.md §3, shape-groups.md §4).
    fn source_problems(&self, i: usize, selector: &Selector, source: Source) -> Vec<Problem> {
        let mut problems = Vec::new();
        let (exists, missing) = match source {
            Source::Dxf(id) => (self.drawing.dxfs.iter().any(|d| d.id == id), "job-no-dxf"),
            Source::Image(id) => (
                self.drawing.images.iter().any(|d| d.id == id),
                "job-no-image",
            ),
            Source::Text(id) => (self.drawing.texts.iter().any(|t| t.id == id), "job-no-text"),
            Source::Group(id) => (
                self.drawing.groups.iter().any(|g| g.id == id),
                "job-no-group",
            ),
            Source::Model(id) => (
                self.drawing.models.iter().any(|m| m.id == id),
                "job-no-model",
            ),
        };
        if !exists {
            problems.push(Problem::at(
                format!("operations.{i}.select.{}", source.key()),
                Message::new(missing).arg("id", source.id()),
            ));
        }
        // Shapes left out of it (docs/specs/leave-out-shapes.md §2, text.md §3,
        // shape-groups.md §4).
        let grouped = selector.group().map(|g| self.drawing.group_shapes(g));
        let of_source = |shape: &Shape| match source {
            Source::Group(_) => grouped.as_ref().is_some_and(|g| g.contains(&shape.id)),
            _ => shape.source() == source.shape_from(),
        };
        if let Some(missing) = selector.exclude.iter().find(|e| {
            !self
                .drawing
                .shapes
                .iter()
                .any(|s| s.id == **e && of_source(s))
        }) {
            problems.push(Problem::at(
                format!("operations.{i}.select.exclude"),
                Message::new("job-exclude-missing")
                    .arg("shape", *missing)
                    .arg("source", source.key())
                    .arg("id", source.id()),
            ));
        }
        problems
    }

    /// Shapes picked by id whose ids change when they are made again: a text's letters, a
    /// picture's traced shapes.
    fn renewed_id_problems(&self, i: usize, selector: &Selector) -> Vec<Problem> {
        let mut problems = Vec::new();
        // A text's letters get new ids each time it is made again (text.md §3).
        let letter = selector.shapes.iter().flatten().find_map(|id| {
            self.drawing
                .shapes
                .iter()
                .find(|s| s.id == *id && s.text().is_some())
        });
        if let Some(shape) = letter {
            problems.push(Problem::at(
                format!("operations.{i}.select"),
                Message::new("job-select-letter")
                    .arg("shape", shape.id)
                    .arg("text", shape.text().unwrap_or_default()),
            ));
        }
        // Traced shapes get new ids each time their image is traced again
        // (docs/specs/image-trace.md §4, decision 6).
        let traced = selector.shapes.iter().flatten().find_map(|id| {
            self.drawing
                .shapes
                .iter()
                .find(|s| s.id == *id && s.image().is_some())
        });
        if let Some(shape) = traced {
            problems.push(Problem::at(
                format!("operations.{i}.select"),
                Message::new("job-select-traced")
                    .arg("shape", shape.id)
                    .arg("image", shape.image().unwrap_or_default()),
            ));
        }
        problems
    }

    fn centre_line_problems(&self, i: usize, op: &Operation, selector: &Selector) -> Vec<Problem> {
        let mut problems = Vec::new();
        // Open centre lines are paths to follow, not areas
        // (docs/specs/image-arcs-centrelines.md §5). A closed one, such as an outline
        // drawn by hand, is an area like any other: refused only when the picture has
        // no closed line at all (user report 2026-10-13).
        let centre_lines = self.drawing.images.iter().find(|image| {
            image.trace == ImageTrace::CentreLine
                && (selector.image() == Some(image.id)
                    || selector.dxf().is_none()
                        && selector.layer.as_ref().is_some_and(|layer| {
                            image.layer.to_lowercase() == layer.to_lowercase()
                        }))
                && !self
                    .drawing
                    .shapes
                    .iter()
                    .any(|s| s.image() == Some(image.id) && s.closed())
        });
        if let Some(image) = centre_lines
            && !matches!(op, Operation::Path(_))
        {
            problems.push(Problem::at(
                format!("operations.{i}.select"),
                Message::new("job-select-centre-lines").arg("image", image.id),
            ));
        }
        problems
    }
}

/// A pattern's values of 0 or more and its crossing angle.
fn pattern_problems(i: usize, f: &PatternOp) -> Vec<Problem> {
    let mut problems = Vec::new();
    let mut zero_or_more = vec![("ramp", f.ramp)];
    if let FillPattern::Waves { height, .. } = f.pattern {
        zero_or_more.push(("pattern.height", height));
    }
    if let FillPattern::Dashes { gap, .. } = f.pattern {
        zero_or_more.push(("pattern.gap", gap));
    }
    for (field, value) in zero_or_more {
        if !(value.is_finite() && value >= 0.0) {
            problems.push(Problem::at(
                format!("operations.{i}.{field}"),
                Message::new("job-must-be-0-or-more")
                    .arg("got", Value::Length(value, Digits::Shortest)),
            ));
        }
    }
    if let FillPattern::Cross { between, .. } = f.pattern
        && !(between > 0.0
            && between <= std::f64::consts::FRAC_PI_2 + chipload_core::tolerances::ANGLE_TOL_RAD)
    {
        problems.push(Problem::at(
            format!("operations.{i}.pattern.between"),
            Message::new("job-pattern-between")
                .arg("got", Value::Number(between.to_degrees(), Digits::Shortest)),
        ));
    }
    problems
}

/// A flute's ramps are lengths of 0 or more.
fn flute_problems(i: usize, f: &FluteOp) -> Vec<Problem> {
    let mut problems = Vec::new();
    for (field, ramp) in [("ramp_start", f.ramp_start), ("ramp_end", f.ramp_end)] {
        if !(ramp.is_finite() && ramp >= 0.0) {
            problems.push(Problem::at(
                format!("operations.{i}.{field}"),
                Message::new("job-must-be-0-or-more")
                    .arg("got", Value::Length(ramp, Digits::Shortest)),
            ));
        }
    }
    problems
}

/// A profile on the line has no clearance.
fn on_line_problems(i: usize, op: &Operation) -> Vec<Problem> {
    let mut problems = Vec::new();
    if let Operation::Profile(p) = op
        && p.side == Side::On
        && p.clearance != 0.0
    {
        problems.push(Problem::at(
            format!("operations.{i}.clearance"),
            Message::new("job-on-line-clearance")
                .arg("got", Value::Length(p.clearance, Digits::Shortest)),
        ));
    }
    problems
}

/// The way in and round corners: entry, ramp and lead sizes, bones, starts.
pub(super) fn way_in_problems(i: usize, op: &Operation) -> Vec<Problem> {
    let mut problems = Vec::new();
    if let Entry::Helix { radius, .. } = op.entry()
        && !(radius.is_finite() && radius > 0.0)
    {
        problems.push(Problem::at(
            format!("operations.{i}.entry.radius"),
            Message::new("job-must-be-more-than-0")
                .arg("got", Value::Length(radius, Digits::Shortest)),
        ));
    }
    if let Some(ramp_angle) = op.entry().ramp_angle()
        && !(ramp_angle.is_finite() && ramp_angle > 0.0 && ramp_angle <= MAX_RAMP_ANGLE_RAD)
    {
        // A profile's angle is its own field; the others' sit in their entry.
        let field = match op {
            Operation::Profile(_) => "ramp_angle",
            _ => "entry.ramp_angle",
        };
        problems.push(Problem::at(
            format!("operations.{i}.{field}"),
            Message::new("job-ramp-angle-range").arg(
                "got",
                Value::Number(ramp_angle.to_degrees(), Digits::Fixed(3)),
            ),
        ));
    }
    if let Some(corners) = op.corners() {
        let angle = corners.max_angle();
        if !(angle.is_finite() && angle > 0.0 && angle < std::f64::consts::PI) {
            problems.push(Problem::at(
                format!("operations.{i}.corners.max_angle"),
                Message::new("job-angle-below-180")
                    .arg("got", Value::Number(angle.to_degrees(), Digits::Fixed(3))),
            ));
        }
        if let Operation::Profile(p) = op
            && p.side == Side::On
        {
            problems.push(Problem::at(
                format!("operations.{i}.corners"),
                Message::new("job-on-line-corners"),
            ));
        }
    }
    problems.extend(corner_choice_problems(i, op));
    if let Operation::Holes(HolesOp {
        method: HoleMethod::Bore { ramp_angle, .. },
        ..
    }) = op
        && !(ramp_angle.is_finite() && *ramp_angle > 0.0 && *ramp_angle <= MAX_RAMP_ANGLE_RAD)
    {
        problems.push(Problem::at(
            format!("operations.{i}.method.ramp_angle"),
            Message::new("job-ramp-angle-range").arg(
                "got",
                Value::Number(ramp_angle.to_degrees(), Digits::Fixed(3)),
            ),
        ));
    }
    if let Some(lead) = op.lead()
        && !(lead.radius.is_finite() && lead.radius > 0.0)
    {
        problems.push(Problem::at(
            format!("operations.{i}.lead.radius"),
            Message::new("job-must-be-more-than-0")
                .arg("got", Value::Length(lead.radius, Digits::Shortest)),
        ));
    }
    if op
        .starts()
        .iter()
        .any(|p| !(p[0].is_finite() && p[1].is_finite()))
    {
        problems.push(Problem::at(
            format!("operations.{i}.starts"),
            Message::new("job-starts-not-finite"),
        ));
    }
    problems
}

/// Stepovers of at most 1, tabs, and the finishing passes' own rules.
pub(super) fn pass_problems(i: usize, op: &Operation) -> Vec<Problem> {
    let mut problems = Vec::new();
    if let Operation::Pocket(p) = op
        && p.stepover > 1.0
    {
        problems.push(Problem::at(
            format!("operations.{i}.stepover"),
            Message::new("job-stepover-at-most-1")
                .arg("got", Value::Number(p.stepover, Digits::Shortest)),
        ));
    }
    if let Operation::Pocket(p) = op {
        for (k, pass) in p.rest_passes.iter().enumerate() {
            if pass.stepover > 1.0 {
                problems.push(Problem::at(
                    format!("operations.{i}.rest_passes.{k}.stepover"),
                    Message::new("job-stepover-at-most-1")
                        .arg("got", Value::Number(pass.stepover, Digits::Shortest)),
                ));
            }
        }
    }
    if let Operation::Vcarve(VcarveOp {
        floor: Some(floor), ..
    }) = op
        && floor.stepover > 1.0
    {
        problems.push(Problem::at(
            format!("operations.{i}.floor.stepover"),
            Message::new("job-stepover-at-most-1")
                .arg("got", Value::Number(floor.stepover, Digits::Shortest)),
        ));
    }
    if let Operation::Profile(ProfileOp {
        tabs: Some(tabs), ..
    }) = op
        && tabs.count < 1
    {
        problems.push(Problem::at(
            format!("operations.{i}.tabs.count"),
            Message::new("job-must-be-at-least-1"),
        ));
    }
    if let Operation::Profile(ProfileOp {
        tabs: Some(tabs), ..
    }) = op
        && !(tabs.flank_angle.is_finite()
            && tabs.flank_angle > 0.0
            && tabs.flank_angle <= STRAIGHT_FLANK_RAD + ANGLE_TOL_RAD)
    {
        problems.push(Problem::at(
            format!("operations.{i}.tabs.flank_angle"),
            Message::new("job-flank-angle-range").arg(
                "got",
                Value::Number(tabs.flank_angle.to_degrees(), Digits::Fixed(3)),
            ),
        ));
    }
    if let Operation::Profile(p) = op {
        problems.extend(finish_skin_problems(p, &format!("operations.{i}")));
    }
    if let Operation::Pocket(p) = op {
        problems.extend(pocket_finish_problems(p, &format!("operations.{i}")));
    }
    problems
}

/// One operation's values read field by field: those that must be more than 0, and whole
/// numbers or ranges out of bounds (reported after all the positives).
#[derive(Default)]
struct Values {
    /// The operation's path, "operations.N".
    at: String,
    positives: Vec<Problem>,
    counts: Vec<Problem>,
}

impl Values {
    fn path(&self, field: &str) -> String {
        format!("{}.{field}", self.at)
    }

    /// Feeds in mm/min, a stepover a fraction, everything else a length in mm (written, so
    /// messages can be shown in inches: docs/specs/inch-units.md §5.1).
    fn positive(&mut self, field: &str, value: f64) {
        if value <= 0.0 || value.is_nan() {
            let got = if field.ends_with("feed") {
                Value::Feed(value, Digits::Shortest)
            } else if field.ends_with("stepover") {
                Value::Number(value, Digits::Shortest)
            } else {
                Value::Length(value, Digits::Shortest)
            };
            self.positives.push(Problem::at(
                self.path(field),
                Message::new("job-must-be-positive").arg("got", got),
            ));
        }
    }

    fn positive_if_set(&mut self, field: &str, value: Option<f64>) {
        if let Some(value) = value {
            self.positive(field, value);
        }
    }

    fn count(&mut self, field: &str, message: Message) {
        self.counts.push(Problem::at(self.path(field), message));
    }

    fn feeds(&mut self, feed: f64, plunge_feed: f64) {
        self.positive("feed", feed);
        self.positive("plunge_feed", plunge_feed);
    }

    fn operation(&mut self, op: &Operation) {
        match op {
            Operation::Profile(p) => self.profile(p),
            Operation::Pocket(p) => self.pocket(p),
            Operation::Vcarve(v) => self.vcarve(v),
            Operation::Path(p) => {
                self.positive("end_depth", p.end_depth);
                self.positive("step_down", p.step_down);
                self.feeds(p.feed, p.plunge_feed);
            }
            Operation::Moulding(m) => self.moulding(m),
            Operation::Prism(f) => {
                self.positive_if_set("max_depth", f.max_depth);
                self.positive_if_set("step_down", f.step_down);
                self.feeds(f.feed, f.plunge_feed);
            }
            Operation::Pattern(f) => self.pattern(f),
            Operation::Flute(f) => {
                self.positive_if_set("step_down", f.step_down);
                self.feeds(f.feed, f.plunge_feed);
            }
            Operation::Chamfer(c) => {
                self.positive("width", c.width);
                self.positive_if_set("step_down", c.step_down);
                self.feeds(c.feed, c.plunge_feed);
            }
            Operation::Surface(f) => {
                self.positive("end_depth", f.end_depth);
                self.positive("step_down", f.step_down);
                self.feeds(f.feed, f.plunge_feed);
            }
            Operation::Relief(r) => self.relief(r),
            Operation::Holes(h) => self.holes(h),
        }
    }

    fn profile(&mut self, p: &ProfileOp) {
        self.positive("end_depth", p.end_depth);
        self.positive("step_down", p.step_down);
        self.feeds(p.feed, p.plunge_feed);
        if let Some(tabs) = &p.tabs {
            self.positive("tabs.width", tabs.width);
            self.positive("tabs.height", tabs.height);
        }
        if let Some(finish) = &p.finish {
            self.positive("finish.stock", finish.stock);
        }
        if let Some(skin) = &p.skin {
            self.positive("skin.thickness", skin.thickness);
        }
    }

    fn pocket(&mut self, p: &PocketOp) {
        self.positive("end_depth", p.end_depth);
        self.positive("step_down", p.step_down);
        self.feeds(p.feed, p.plunge_feed);
        self.positive("stepover", p.stepover);
        // Full-width rings (docs/specs/cutting-data.md §5).
        self.positive_if_set("slot_feed", p.slot_feed);
        self.positive_if_set("slot_step_down", p.slot_step_down);
        // Rest passes (docs/specs/rest-passes.md §7).
        for (k, pass) in p.rest_passes.iter().enumerate() {
            self.clearing_pass(&format!("rest_passes.{k}"), pass);
        }
    }

    /// A rest pass's or floor pass's values, under `prefix`.
    fn clearing_pass(&mut self, prefix: &str, pass: &ClearingPass) {
        self.positive(&format!("{prefix}.step_down"), pass.step_down);
        self.positive(&format!("{prefix}.feed"), pass.feed);
        self.positive(&format!("{prefix}.plunge_feed"), pass.plunge_feed);
        self.positive(&format!("{prefix}.stepover"), pass.stepover);
        self.positive_if_set(&format!("{prefix}.slot_feed"), pass.slot_feed);
        self.positive_if_set(&format!("{prefix}.slot_step_down"), pass.slot_step_down);
    }

    fn vcarve(&mut self, v: &VcarveOp) {
        self.positive("end_depth", v.end_depth);
        if !(1..=MAX_VCARVE_PASSES).contains(&v.passes) {
            self.count(
                "passes",
                Message::new("job-vcarve-passes")
                    .arg("most", MAX_VCARVE_PASSES)
                    .arg("got", v.passes),
            );
        }
        if let Some(step) = v.step_down {
            self.positive("step_down", step);
            let range = v.end_depth - v.start_depth;
            let least = range / f64::from(MAX_VCARVE_PASSES);
            // As many passes as the toolpath takes (`carve_levels`).
            let count = ((range - DEPTH_TOL_MM) / step).ceil();
            if step > 0.0 && count > f64::from(MAX_VCARVE_PASSES) {
                self.count(
                    "step_down",
                    Message::new("job-vcarve-step-down")
                        .arg("most", MAX_VCARVE_PASSES)
                        .arg("least", Value::Length(least, Digits::Shortest))
                        .arg("got", Value::Length(step, Digits::Shortest)),
                );
            }
        }
        if !(0.0..=MAX_IGNORE_KINKS_DEG).contains(&v.ignore_kinks_deg) {
            self.count(
                "ignore_kinks_deg",
                Message::new("job-vcarve-ignore-kinks")
                    .arg(
                        "most",
                        Value::Number(MAX_IGNORE_KINKS_DEG, Digits::Shortest),
                    )
                    .arg("got", Value::Number(v.ignore_kinks_deg, Digits::Shortest)),
            );
        }
        if let Some(plug) = v.plug {
            self.vcarve_plug(v, plug.start_depth);
        }
        self.feeds(v.feed, v.plunge_feed);
        if let Some(floor) = &v.floor {
            self.clearing_pass("floor", floor);
        }
    }

    /// An inlay's plug starts between the pocket's depths, with no kinks ignored
    /// (docs/specs/vcarve-inlay.md §3).
    fn vcarve_plug(&mut self, v: &VcarveOp, start: f64) {
        if !(start > v.start_depth + DEPTH_TOL_MM && start < v.end_depth - DEPTH_TOL_MM) {
            self.count(
                "plug.start_depth",
                Message::new("job-vcarve-plug-start")
                    .arg("start", Value::Length(v.start_depth, Digits::Shortest))
                    .arg("end", Value::Length(v.end_depth, Digits::Shortest))
                    .arg("got", Value::Length(start, Digits::Shortest)),
            );
        }
        if v.ignore_kinks_deg > 0.0 {
            self.count(
                "ignore_kinks_deg",
                Message::new("job-vcarve-plug-kinks")
                    .arg("got", Value::Number(v.ignore_kinks_deg, Digits::Shortest)),
            );
        }
    }

    fn moulding(&mut self, m: &MouldingOp) {
        match m.profile {
            MouldProfile::Cove { radius }
            | MouldProfile::Roundover { radius }
            | MouldProfile::Bead { radius }
            | MouldProfile::Flute { radius } => self.positive("profile.radius", radius),
            MouldProfile::Ogee { width, depth } => {
                self.positive("profile.width", width);
                self.positive("profile.depth", depth);
            }
            MouldProfile::Shape { .. } => {}
        }
        if let MouldEnds::Taper { length, .. } = m.ends {
            self.positive("ends.length", length);
        }
        self.positive("stepover", m.stepover);
        self.positive_if_set("step_down", m.step_down);
        self.feeds(m.feed, m.plunge_feed);
    }

    fn pattern(&mut self, f: &PatternOp) {
        self.positive("pattern.spacing", f.pattern.spacing());
        match f.pattern {
            FillPattern::Waves { wavelength, .. } => {
                self.positive("pattern.wavelength", wavelength);
            }
            FillPattern::Dashes { length, .. } | FillPattern::Herringbone { length, .. } => {
                self.positive("pattern.length", length);
            }
            FillPattern::Scales { radius, .. } => self.positive("pattern.radius", radius),
            FillPattern::Sunburst { length, pitch, .. } => {
                self.positive("pattern.length", length);
                self.positive("pattern.pitch", pitch);
            }
            FillPattern::Lines { .. } | FillPattern::Cross { .. } | FillPattern::Rings { .. } => {}
        }
        self.positive_if_set("step_down", f.step_down);
        self.feeds(f.feed, f.plunge_feed);
    }

    fn relief(&mut self, r: &ReliefOp) {
        self.feeds(r.feed, r.plunge_feed);
        self.positive("stepover", r.stepover);
        if let Some(rough) = &r.roughing {
            self.positive("roughing.feed", rough.feed);
            self.positive("roughing.plunge_feed", rough.plunge_feed);
            self.positive("roughing.stepover", rough.stepover);
            self.positive("roughing.step_down", rough.step_down);
        }
        if let Some(smooth) = &r.smoothing {
            self.positive("smoothing.feed", smooth.feed);
            self.positive("smoothing.plunge_feed", smooth.plunge_feed);
            self.positive("smoothing.stepover", smooth.stepover);
        }
    }

    fn holes(&mut self, h: &HolesOp) {
        self.positive("end_depth", h.end_depth);
        self.feeds(h.feed, h.plunge_feed);
        match h.method {
            HoleMethod::Drill { peck } => self.positive_if_set("method.peck", peck),
            HoleMethod::Bore { step_down, .. } => self.positive("method.step_down", step_down),
        }
    }
}

/// The rules for a profile's finishing pass and skin that need no tool
/// (docs/specs/finish-and-skin.md §1, §6); the tool radius is checked when planning.
pub(super) fn finish_skin_problems(p: &ProfileOp, at: &str) -> Vec<Problem> {
    let mut problems = Vec::new();
    if p.finish.is_some() && p.side == Side::On {
        problems.push(Problem::at(
            format!("{at}.finish"),
            Message::new("job-on-line-finish"),
        ));
    }
    let Some(skin) = p.skin else {
        return problems;
    };
    let t = skin.thickness;
    if !(t.is_finite() && t > 0.0) {
        return problems; // reported as not positive
    }
    if t > p.step_down + DEPTH_TOL_MM {
        problems.push(Problem::at(
            format!("{at}.skin.thickness"),
            Message::new("job-skin-above-step-down")
                .arg(
                    "step_down",
                    Value::LengthNumber(p.step_down, Digits::Shortest),
                )
                .arg("got", Value::LengthNumber(t, Digits::Shortest)),
        ));
    }
    let cut = p.end_depth - p.start_depth;
    if t >= cut {
        problems.push(Problem::at(
            format!("{at}.skin.thickness"),
            Message::new("job-skin-above-cut")
                .arg("cut", Value::LengthNumber(cut, Digits::Shortest))
                .arg("got", Value::LengthNumber(t, Digits::Shortest)),
        ));
    }
    if let Some(tabs) = &p.tabs
        && t >= tabs.height
    {
        problems.push(Problem::at(
            format!("{at}.skin.thickness"),
            Message::new("job-skin-above-tabs")
                .arg("height", Value::LengthNumber(tabs.height, Digits::Shortest))
                .arg("got", Value::LengthNumber(t, Digits::Shortest)),
        ));
    }
    problems
}

/// The rules for a pocket's finishing pass that need no tool (docs/specs/pocket-finish.md
/// §2, §5); the wall allowance against the tool radius is checked when planning.
pub(super) fn pocket_finish_problems(p: &PocketOp, at: &str) -> Vec<Problem> {
    let mut problems = Vec::new();
    let Some(finish) = p.finish else {
        return problems;
    };
    let field = |name: &str| format!("{at}.finish.{name}");
    let mut sound = true;
    for (name, value) in [("wall", finish.wall), ("floor", finish.floor)] {
        if !(value.is_finite() && value >= 0.0) {
            sound = false;
            problems.push(Problem::at(
                field(name),
                Message::new("job-pocket-finish-negative")
                    .arg("got", Value::Length(value, Digits::Shortest)),
            ));
        }
    }
    if p.rest.is_some() {
        problems.push(Problem::at(
            format!("{at}.finish"),
            Message::new("job-pocket-finish-rest"),
        ));
    }
    if !sound {
        return problems;
    }
    if finish.wall <= 0.0 && finish.floor <= 0.0 {
        problems.push(Problem::at(
            format!("{at}.finish"),
            Message::new("job-pocket-finish-zero"),
        ));
    }
    let f = finish.floor;
    if f <= 0.0 {
        return problems;
    }
    if p.through.is_some() {
        problems.push(Problem::at(
            field("floor"),
            Message::new("job-pocket-finish-through"),
        ));
    }
    if f > p.step_down + DEPTH_TOL_MM {
        problems.push(Problem::at(
            field("floor"),
            Message::new("job-skin-above-step-down")
                .arg(
                    "step_down",
                    Value::LengthNumber(p.step_down, Digits::Shortest),
                )
                .arg("got", Value::LengthNumber(f, Digits::Shortest)),
        ));
    }
    let cut = p.end_depth - p.start_depth;
    if f >= cut {
        problems.push(Problem::at(
            field("floor"),
            Message::new("job-skin-above-cut")
                .arg("cut", Value::LengthNumber(cut, Digits::Shortest))
                .arg("got", Value::LengthNumber(f, Digits::Shortest)),
        ));
    }
    problems
}

/// A relief's smoothing pass (docs/specs/stl-relief.md §17): its stepover and allowance
/// as the roughing's, a direction, and cut only after a roughing pass that is cut.
fn smoothing_problems(i: usize, relief: &ReliefOp, smooth: &ReliefSmoothing) -> Vec<Problem> {
    let mut problems = Vec::new();
    let at = |field: &str| format!("operations.{i}.smoothing.{field}");
    if smooth.stepover > RELIEF_MAX_STEPOVER {
        problems.push(Problem::at(
            at("stepover"),
            Message::new("job-relief-stepover")
                .arg("most", Value::Number(RELIEF_MAX_STEPOVER, Digits::Shortest))
                .arg("got", Value::Number(smooth.stepover, Digits::Shortest)),
        ));
    }
    if !(smooth.allowance.is_finite() && smooth.allowance >= 0.0) {
        problems.push(Problem::at(
            at("allowance"),
            Message::new("job-relief-allowance")
                .arg("got", Value::Length(smooth.allowance, Digits::Shortest)),
        ));
    }
    if !smooth.angle.is_finite() {
        problems.push(Problem::at(
            at("angle"),
            Message::new("job-must-be-number")
                .arg("got", Value::Number(smooth.angle, Digits::Shortest)),
        ));
    }
    let roughed = relief.roughing.is_some_and(|r| r.enabled);
    if smooth.enabled && !roughed {
        problems.push(Problem::at(
            at("enabled"),
            Message::new("job-relief-smoothing-alone"),
        ));
    }
    problems
}

/// A relief's rim (docs/specs/stl-relief.md §16): a width of 0 or more, a ramp from 10° to
/// 90°, a box's corner radius from 0 to the width.
fn rim_problems(i: usize, rim: &ReliefRim) -> Vec<Problem> {
    let at = |field: &str| format!("operations.{i}.rim.{field}");
    let degrees = |radians: f64| Value::Number(radians.to_degrees(), Digits::Fixed(1));
    let mut problems = Vec::new();
    if !(rim.width.is_finite() && rim.width >= 0.0) {
        problems.push(Problem::at(
            at("width"),
            Message::new("job-relief-allowance")
                .arg("got", Value::Length(rim.width, Digits::Shortest)),
        ));
    }
    let straight = std::f64::consts::FRAC_PI_2;
    if !(rim.ramp.is_finite()
        && (RIM_MIN_RAMP - chipload_core::tolerances::ANGLE_TOL_RAD
            ..=straight + chipload_core::tolerances::ANGLE_TOL_RAD)
            .contains(&rim.ramp))
    {
        problems.push(Problem::at(
            at("ramp"),
            Message::new("job-relief-ramp")
                .arg("least", degrees(RIM_MIN_RAMP))
                .arg("got", degrees(rim.ramp)),
        ));
    }
    let corner_fits = rim.corner_radius.is_finite()
        && rim.corner_radius >= 0.0
        && rim.corner_radius <= rim.width.max(0.0);
    if !(rim.hull.is_finite() && (0.0..=1.0).contains(&rim.hull)) {
        problems.push(Problem::at(
            at("hull"),
            Message::new("job-relief-rim-hull")
                .arg("got", Value::Number(rim.hull, Digits::Shortest)),
        ));
    }
    if rim.shape == RimShape::Box && !corner_fits {
        problems.push(Problem::at(
            at("corner_radius"),
            Message::new("job-relief-rim-corner")
                .arg("width", Value::Length(rim.width, Digits::Shortest))
                .arg("got", Value::Length(rim.corner_radius, Digits::Shortest)),
        ));
    }
    problems
}
