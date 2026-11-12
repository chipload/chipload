// SPDX-License-Identifier: GPL-3.0-or-later
//! The project's rules (`Job::validate` and the checks it gathers).

use super::*;

/// "<at>.<field>: must be greater than 0, got <value>" unless value > 0.
pub(super) fn positive_problem(at: &str, field: &str, value: f64) -> Option<Problem> {
    (value <= 0.0 || value.is_nan()).then(|| {
        Problem::at(
            format!("{at}.{field}"),
            Message::new("job-must-be-positive").arg("got", Value::Length(value, Digits::Shortest)),
        )
    })
}

/// The rule for `max_overcut`, kept under `stock` in projects and in machine profiles.
pub(crate) fn max_overcut_problems(at: &str, overcut: f64) -> Vec<Problem> {
    if overcut.is_finite() && overcut >= 0.0 {
        Vec::new()
    } else {
        vec![Problem::at(
            format!("{at}.max_overcut"),
            Message::new("job-must-be-0-or-more")
                .arg("got", Value::Length(overcut, Digits::Shortest)),
        )]
    }
}

/// A work zero's rules, at `at`: a point must be a point. Shared by setups and machine
/// profiles.
pub fn work_zero_problems(at: &str, zero: WorkZero) -> Vec<Problem> {
    match zero {
        WorkZero::Point { x, y } if !(x.is_finite() && y.is_finite()) => vec![Problem::at(
            at,
            Message::new("job-work-zero-not-finite")
                .arg("x", Value::LengthNumber(x, Digits::Shortest))
                .arg("y", Value::LengthNumber(y, Digits::Shortest)),
        )],
        _ => Vec::new(),
    }
}

impl Stock {
    /// A stock's rules, each at "<at>.field": a size and thickness, a fit only when centred
    /// on the drawing, a max overcut.
    pub fn problems(&self, at: &str) -> Vec<Problem> {
        let mut problems = Vec::new();
        if let Some(margin) = self.fit {
            if !(margin.is_finite() && margin >= 0.0) {
                problems.push(Problem::at(
                    format!("{at}.fit"),
                    Message::new("job-must-be-0-or-more")
                        .arg("got", Value::LengthNumber(margin, Digits::Shortest)),
                ));
            } else if self.origin != StockOrigin::Drawing {
                problems.push(Problem::at(
                    format!("{at}.fit"),
                    Message::new("job-stock-fit-needs-centred"),
                ));
            }
        }
        for (field, value) in [("x", self.x), ("y", self.y), ("thickness", self.thickness)] {
            if value <= 0.0 || value.is_nan() {
                problems.push(Problem::at(
                    format!("{at}.{field}"),
                    Message::new("job-must-be-positive")
                        .arg("got", Value::Length(value, Digits::Shortest)),
                ));
            }
        }
        problems.extend(max_overcut_problems(at, self.max_overcut));
        problems
    }
}

impl Machine {
    /// Value rules of a machine, each at "<at>.field"; shared by projects and machine
    /// profiles.
    pub fn problems(&self, at: &str) -> Vec<Problem> {
        let mut problems = Vec::new();
        if let Some(t) = &self.travel {
            for (axis, value) in [("x", t.x), ("y", t.y), ("z", t.z)] {
                problems.extend(positive_problem(&format!("{at}.travel"), axis, value));
            }
        }
        problems
    }
}

/// A relief's lines at most this share of the tool apart (docs/specs/stl-relief.md §5).
pub const RELIEF_MAX_STEPOVER: f64 = 0.9;

impl Tool {
    /// Value rules of a tool, each at "<at>.field"; shared by projects and the tool
    /// library.
    pub fn problems(&self, at: &str) -> Vec<Problem> {
        let mut problems: Vec<Problem> = positive_problem(at, "diameter", self.diameter)
            .into_iter()
            .collect();
        if let Some(angle) = self.angle
            && !(angle.is_finite() && angle > 0.0 && angle < std::f64::consts::PI)
        {
            problems.push(Problem::at(
                format!("{at}.angle"),
                Message::new("job-angle-below-180")
                    .arg("got", Value::Number(angle.to_degrees(), Digits::Fixed(3))),
            ));
        }
        if let Some(tip) = self.tip_diameter
            && !(tip.is_finite() && tip >= 0.0 && tip < self.diameter)
        {
            problems.push(Problem::at(
                format!("{at}.tip_diameter"),
                Message::new("job-tip-diameter")
                    .arg("diameter", Value::Length(self.diameter, Digits::Shortest))
                    .arg("got", Value::Length(tip, Digits::Shortest)),
            ));
        }
        // A tapered ball nose: its ball at the tip (docs/specs/stl-relief.md §8).
        if self.kind == ToolType::Ballnose && self.angle.is_some() && self.tip_diameter.is_none() {
            problems.push(Problem::at(
                format!("{at}.tip_diameter"),
                Message::new("job-tapered-ball-needs-tip"),
            ));
        }
        problems
    }
}

impl Job {
    /// The parts' rules (docs/specs/parts.md §4): at least one, unique ids and names,
    /// valid copies, every operation in a part, and the operations grouped by part in the
    /// parts' order.
    /// The setups' rules (docs/specs/setups.md §6): at least one, ids unique and not 0, a
    /// name, and each stock and work zero as the stock and zero have always been checked.
    fn setup_problems(&self) -> Vec<Problem> {
        let mut problems = Vec::new();
        if self.setups.is_empty() {
            problems.push(Problem::at("setups", Message::new("job-no-setups")));
        }
        for (k, setup) in self.setups.iter().enumerate() {
            let at = format!("setups.{k}");
            if setup.id == 0 {
                problems.push(Problem::at(
                    format!("{at}.id"),
                    Message::new("job-must-be-1-or-more"),
                ));
            }
            if self.setups[..k].iter().any(|s| s.id == setup.id) {
                problems.push(Problem::at(
                    format!("{at}.id"),
                    Message::new("job-setup-twice").arg("id", setup.id),
                ));
            }
            if setup.name.trim().is_empty() {
                problems.push(Problem::at(
                    format!("{at}.name"),
                    Message::new("job-must-not-be-empty"),
                ));
            }
            problems.extend(setup.stock.problems(&format!("{at}.stock")));
            problems.extend(work_zero_problems(
                &format!("{at}.work_zero"),
                setup.work_zero,
            ));
            problems.extend(self.back_problems(k));
        }
        problems.extend(self.dowel_problems());
        for (i, part) in self.parts.iter().enumerate() {
            if self.setup(part.setup).is_none() {
                problems.push(Problem::at(
                    format!("parts.{i}.setup"),
                    Message::new("job-no-such-setup").arg("id", part.setup),
                ));
            }
        }
        problems
    }

    /// A back setup's rules (docs/specs/two-sided.md §9): its front is another setup that
    /// is not itself a back, and has no other back.
    fn back_problems(&self, k: usize) -> Vec<Problem> {
        let setup = &self.setups[k];
        let Some(back) = setup.back_of else {
            return Vec::new();
        };
        let at = format!("setups.{k}.back_of.setup");
        let problem = if back.setup == setup.id {
            Message::new("job-back-of-itself")
        } else {
            match self.setup(back.setup) {
                None => Message::new("job-no-such-setup").arg("id", back.setup),
                Some(front) if front.back_of.is_some() => {
                    Message::new("job-back-of-a-back").arg("id", back.setup)
                }
                Some(_)
                    if self.setups[..k]
                        .iter()
                        .any(|s| s.back_of.is_some_and(|b| b.setup == back.setup)) =>
                {
                    Message::new("job-two-backs").arg("id", back.setup)
                }
                Some(_) => return Vec::new(),
            }
        };
        vec![Problem::at(at, problem)]
    }

    /// Dowel holes' rules (docs/specs/two-sided.md §6): cut with the front of a board that
    /// has a back, through it, and drawn circles on the line it turns about, so each meets
    /// its pin on the turned board.
    fn dowel_problems(&self) -> Vec<Problem> {
        let mut problems = Vec::new();
        for (i, op) in self.operations.iter().enumerate() {
            let Operation::Holes(h) = op else {
                continue;
            };
            if !h.dowels {
                continue;
            }
            let at = |field: &str| format!("operations.{i}.{field}");
            let Some(setup) = self.setup_of(op).map(|s| s.id) else {
                continue;
            };
            let Some(back) = self.back_of(setup).and_then(|b| b.back_of) else {
                problems.push(Problem::at(
                    at("dowels"),
                    Message::new("job-dowels-no-back").arg("id", setup),
                ));
                continue;
            };
            if h.through.is_none_or(|t| t <= 0.0) {
                problems.push(Problem::at(
                    at("through"),
                    Message::new("job-dowels-not-through"),
                ));
            }
            let ((x0, y0), (x1, y1)) = self.for_setup(setup).stock_bounds();
            let select = h.select.resolved(&self.drawing);
            for shape in self.drawing.shapes.iter().filter(|s| select.may_pick(s)) {
                let Geometry::Circle { center, .. } = shape.geometry else {
                    problems.push(Problem::at(
                        at("select"),
                        Message::new("job-dowel-not-circle").arg("shape", shape.id),
                    ));
                    continue;
                };
                let off = match back.turn {
                    Turn::LeftRight => center[0] - (x0 + x1) / 2.0,
                    Turn::TopBottom => center[1] - (y0 + y1) / 2.0,
                };
                if off.abs() > CHAIN_TOL_MM {
                    problems.push(Problem::at(
                        at("select"),
                        Message::new("job-dowel-off-axis")
                            .arg("shape", shape.id)
                            .arg("off", Value::Length(off.abs(), Digits::Fixed(3))),
                    ));
                }
            }
        }
        problems
    }

    fn part_problems(&self) -> Vec<Problem> {
        let mut problems = Vec::new();
        if self.parts.is_empty() {
            problems.push(Problem::at("parts", Message::new("job-no-parts")));
        }
        for (i, part) in self.parts.iter().enumerate() {
            if part.id == 0 {
                problems.push(Problem::at(
                    format!("parts.{i}.id"),
                    Message::new("job-must-be-1-or-more"),
                ));
            }
            if self.parts[..i].iter().any(|p| p.id == part.id) {
                problems.push(Problem::at(
                    format!("parts.{i}.id"),
                    Message::new("job-part-twice").arg("id", part.id),
                ));
            }
            let name = part.name.trim();
            if name.is_empty() {
                problems.push(Problem::at(
                    format!("parts.{i}.name"),
                    Message::new("job-must-not-be-empty"),
                ));
            } else if self.parts[..i]
                .iter()
                .any(|p| p.name.trim().eq_ignore_ascii_case(name))
            {
                problems.push(Problem::at(
                    format!("parts.{i}.name"),
                    Message::new("job-part-name-taken").arg("name", name),
                ));
            }
            if let Some(copies) = &part.copies {
                problems.extend(copies.problems(&format!("parts.{i}.copies")));
            }
        }
        let rank = |id: u64| self.parts.iter().position(|p| p.id == id);
        let mut last: Option<(usize, u64)> = None;
        for (i, op) in self.operations.iter().enumerate() {
            let Some(here) = rank(op.part()) else {
                problems.push(Problem::at(
                    format!("operations.{i}.part"),
                    Message::new("job-no-part").arg("id", op.part()),
                ));
                continue;
            };
            if let Some((before, before_id)) = last
                && here < before
            {
                problems.push(Problem::at(
                    format!("operations.{i}.part"),
                    Message::new("job-part-order")
                        .arg("part", op.part())
                        .arg("before", before_id),
                ));
            }
            if last.is_none_or(|(before, _)| here > before) {
                last = Some((here, op.part()));
            }
        }
        problems
    }

    /// Every value rule the project breaks, with the field it is about
    /// (docs/specs/live-problems.md): the constraints the serde types can't express.
    pub fn validate(&self) -> Vec<Problem> {
        let mut problems = self.drawing.validate();
        problems.extend(self.machine.problems("machine"));
        for (k, model) in self.drawing.models.iter().enumerate() {
            problems.extend(model.problems(k));
        }
        for (i, source) in self.drawing.dxfs.iter().enumerate() {
            let at = format!("drawing.dxfs.{i}.placement");
            let Placement {
                scale,
                rotation,
                dx,
                dy,
            } = source.placement;
            if !Placement::scale_in_range(scale) {
                problems.push(Problem::at(
                    format!("{at}.scale"),
                    Message::new("job-scale-range")
                        .arg("low", Value::Number(Placement::MIN_SCALE, Digits::Shortest))
                        .arg(
                            "high",
                            Value::Number(Placement::MAX_SCALE, Digits::Shortest),
                        )
                        .arg("got", Value::Number(scale, Digits::Shortest)),
                ));
            }
            if !(dx.is_finite() && dy.is_finite()) {
                problems.push(Problem::at(
                    &at,
                    Message::new("job-must-be-numbers")
                        .arg("x", Value::LengthNumber(dx, Digits::Shortest))
                        .arg("y", Value::LengthNumber(dy, Digits::Shortest)),
                ));
            }
            if !rotation_in_range(rotation) {
                problems.push(Problem::at(
                    format!("{at}.rotation"),
                    Message::new("job-rotation-range")
                        .arg("got", Value::Number(rotation, Digits::Shortest)),
                ));
            }
        }
        problems.extend(self.setup_problems());
        for (i, tool) in self.tools.iter().enumerate() {
            problems.extend(tool.problems(&format!("tools.{i}")));
        }
        // Whole numbers out of range: added after the operations.
        let mut counts = Vec::new();
        // Feeds in mm/min, a stepover a fraction, everything else a length in mm (written,
        // so messages can be shown in inches: docs/specs/inch-units.md §5.1).
        let mut positive = |path: String, value: f64| {
            if value <= 0.0 || value.is_nan() {
                let got = if path.ends_with("feed") {
                    Value::Feed(value, Digits::Shortest)
                } else if path.ends_with("stepover") {
                    Value::Number(value, Digits::Shortest)
                } else {
                    Value::Length(value, Digits::Shortest)
                };
                problems.push(Problem::at(
                    path,
                    Message::new("job-must-be-positive").arg("got", got),
                ));
            }
        };
        for (i, op) in self.operations.iter().enumerate() {
            let at = |field: &str| format!("operations.{i}.{field}");
            match op {
                Operation::Profile(p) => {
                    positive(at("end_depth"), p.end_depth);
                    positive(at("step_down"), p.step_down);
                    positive(at("feed"), p.feed);
                    positive(at("plunge_feed"), p.plunge_feed);
                    if let Some(tabs) = &p.tabs {
                        positive(at("tabs.width"), tabs.width);
                        positive(at("tabs.height"), tabs.height);
                    }
                    if let Some(finish) = &p.finish {
                        positive(at("finish.stock"), finish.stock);
                    }
                    if let Some(skin) = &p.skin {
                        positive(at("skin.thickness"), skin.thickness);
                    }
                }
                Operation::Pocket(p) => {
                    positive(at("end_depth"), p.end_depth);
                    positive(at("step_down"), p.step_down);
                    positive(at("feed"), p.feed);
                    positive(at("plunge_feed"), p.plunge_feed);
                    positive(at("stepover"), p.stepover);
                    // Full-width rings (docs/specs/cutting-data.md §5).
                    if let Some(feed) = p.slot_feed {
                        positive(at("slot_feed"), feed);
                    }
                    if let Some(step) = p.slot_step_down {
                        positive(at("slot_step_down"), step);
                    }
                    // Rest passes (docs/specs/rest-passes.md §7).
                    for (k, pass) in p.rest_passes.iter().enumerate() {
                        let at = |field: &str| format!("operations.{i}.rest_passes.{k}.{field}");
                        positive(at("step_down"), pass.step_down);
                        positive(at("feed"), pass.feed);
                        positive(at("plunge_feed"), pass.plunge_feed);
                        positive(at("stepover"), pass.stepover);
                        if let Some(feed) = pass.slot_feed {
                            positive(at("slot_feed"), feed);
                        }
                        if let Some(step) = pass.slot_step_down {
                            positive(at("slot_step_down"), step);
                        }
                    }
                }
                Operation::Vcarve(v) => {
                    positive(at("end_depth"), v.end_depth);
                    if !(1..=MAX_VCARVE_PASSES).contains(&v.passes) {
                        counts.push(Problem::at(
                            at("passes"),
                            Message::new("job-vcarve-passes")
                                .arg("most", MAX_VCARVE_PASSES)
                                .arg("got", v.passes),
                        ));
                    }
                    if let Some(step) = v.step_down {
                        positive(at("step_down"), step);
                        let range = v.end_depth - v.start_depth;
                        let least = range / f64::from(MAX_VCARVE_PASSES);
                        // As many passes as the toolpath takes (`carve_levels`).
                        let count = ((range - DEPTH_TOL_MM) / step).ceil();
                        if step > 0.0 && count > f64::from(MAX_VCARVE_PASSES) {
                            counts.push(Problem::at(
                                at("step_down"),
                                Message::new("job-vcarve-step-down")
                                    .arg("most", MAX_VCARVE_PASSES)
                                    .arg("least", Value::Length(least, Digits::Shortest))
                                    .arg("got", Value::Length(step, Digits::Shortest)),
                            ));
                        }
                    }
                    if !(0.0..=MAX_IGNORE_KINKS_DEG).contains(&v.ignore_kinks_deg) {
                        counts.push(Problem::at(
                            at("ignore_kinks_deg"),
                            Message::new("job-vcarve-ignore-kinks")
                                .arg(
                                    "most",
                                    Value::Number(MAX_IGNORE_KINKS_DEG, Digits::Shortest),
                                )
                                .arg("got", Value::Number(v.ignore_kinks_deg, Digits::Shortest)),
                        ));
                    }
                    // An inlay's plug (docs/specs/vcarve-inlay.md §3).
                    if let Some(plug) = v.plug {
                        let start = plug.start_depth;
                        if !(start > v.start_depth + DEPTH_TOL_MM
                            && start < v.end_depth - DEPTH_TOL_MM)
                        {
                            counts.push(Problem::at(
                                format!("operations.{i}.plug.start_depth"),
                                Message::new("job-vcarve-plug-start")
                                    .arg("start", Value::Length(v.start_depth, Digits::Shortest))
                                    .arg("end", Value::Length(v.end_depth, Digits::Shortest))
                                    .arg("got", Value::Length(start, Digits::Shortest)),
                            ));
                        }
                        if v.ignore_kinks_deg > 0.0 {
                            counts.push(Problem::at(
                                at("ignore_kinks_deg"),
                                Message::new("job-vcarve-plug-kinks").arg(
                                    "got",
                                    Value::Number(v.ignore_kinks_deg, Digits::Shortest),
                                ),
                            ));
                        }
                    }
                    positive(at("feed"), v.feed);
                    positive(at("plunge_feed"), v.plunge_feed);
                    if let Some(floor) = &v.floor {
                        let at = |field: &str| format!("operations.{i}.floor.{field}");
                        positive(at("step_down"), floor.step_down);
                        positive(at("feed"), floor.feed);
                        positive(at("plunge_feed"), floor.plunge_feed);
                        positive(at("stepover"), floor.stepover);
                        if let Some(feed) = floor.slot_feed {
                            positive(at("slot_feed"), feed);
                        }
                        if let Some(step) = floor.slot_step_down {
                            positive(at("slot_step_down"), step);
                        }
                    }
                }
                Operation::Path(p) => {
                    positive(at("end_depth"), p.end_depth);
                    positive(at("step_down"), p.step_down);
                    positive(at("feed"), p.feed);
                    positive(at("plunge_feed"), p.plunge_feed);
                }
                Operation::Surface(f) => {
                    positive(at("end_depth"), f.end_depth);
                    positive(at("step_down"), f.step_down);
                    positive(at("feed"), f.feed);
                    positive(at("plunge_feed"), f.plunge_feed);
                }
                Operation::Relief(r) => {
                    positive(at("feed"), r.feed);
                    positive(at("plunge_feed"), r.plunge_feed);
                    positive(at("stepover"), r.stepover);
                }
                Operation::Holes(h) => {
                    positive(at("end_depth"), h.end_depth);
                    positive(at("feed"), h.feed);
                    positive(at("plunge_feed"), h.plunge_feed);
                    match h.method {
                        HoleMethod::Drill { peck } => {
                            if let Some(peck) = peck {
                                positive(at("method.peck"), peck);
                            }
                        }
                        HoleMethod::Bore { step_down, .. } => {
                            positive(at("method.step_down"), step_down);
                        }
                    }
                }
            }
        }
        problems.extend(counts);
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
            } else if (op.depths().1 - (thickness + below + self.drill_point(op))).abs()
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
        problems.extend(self.part_problems());
        for (i, op) in self.operations.iter().enumerate() {
            let (start, end) = op.depths();
            if !(start.is_finite() && start >= 0.0) {
                problems.push(Problem::at(
                    format!("operations.{i}.start_depth"),
                    Message::new("job-must-be-0-or-more")
                        .arg("got", Value::Length(start, Digits::Shortest)),
                ));
            } else if start >= end {
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
            if let Operation::Surface(f) = op {
                let thickness = self.setup_of(op).map_or(0.0, |s| s.stock.thickness);
                problems.extend(surface_problems(i, f, thickness));
            }
            // A relief carves a model of the project, in lines no wider apart than 0.9 of
            // the tool (docs/specs/stl-relief.md §5).
            if let Operation::Relief(r) = op {
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
                if !r.angle.is_finite() {
                    problems.push(Problem::at(
                        format!("operations.{i}.angle"),
                        Message::new("job-must-be-number")
                            .arg("got", Value::Number(r.angle, Digits::Shortest)),
                    ));
                }
            }
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
        }
        for (i, op) in self.operations.iter().enumerate() {
            if !op.selects_shapes() {
                continue;
            }
            let selector = op.select();
            if selector.is_empty() {
                problems.push(Problem::at(
                    format!("operations.{i}.select"),
                    Message::new("job-selector-empty"),
                ));
            }
            if let Some(source) = selector.source {
                let (exists, missing) = match source {
                    Source::Dxf(id) => (self.drawing.dxfs.iter().any(|d| d.id == id), "job-no-dxf"),
                    Source::Image(id) => (
                        self.drawing.images.iter().any(|d| d.id == id),
                        "job-no-image",
                    ),
                    Source::Text(id) => {
                        (self.drawing.texts.iter().any(|t| t.id == id), "job-no-text")
                    }
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
            }
            // A group is resolved to shapes: naming shapes besides it is ambiguous.
            if selector.group().is_some() && selector.shapes.is_some() {
                problems.push(Problem::at(
                    format!("operations.{i}.select"),
                    Message::new("job-selector-one-source"),
                ));
            }
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
                && !(ramp_angle.is_finite()
                    && *ramp_angle > 0.0
                    && *ramp_angle <= MAX_RAMP_ANGLE_RAD)
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
        }
        problems
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
