// SPDX-License-Identifier: GPL-3.0-or-later
//! Independent G-code checker.
//!
//! Parses G-code *text* with its own tokenizer and modal state and checks it against the safety
//! rules in `docs/specs/v0-profile-pipeline.md` section 6. It must never depend on the toolpath
//! generator or post-processor crates: its value is that it can disagree with them.
//!
//! Only ASCII digits are accepted in numbers.

mod parse;

use std::collections::BTreeMap;

use chipload_core::tolerances::{ARC_APPROX_TOL_MM, CHAIN_TOL_MM};
use chipload_i18n::{Digits, Message, Value};
use chipload_job::{Job, Units};

/// Positional slack for comparisons against heights and bounds.
pub const POSITION_TOL_MM: f64 = CHAIN_TOL_MM;
/// What an inch program may be off by on top of [`POSITION_TOL_MM`]: half of 0.0001", the
/// last decimal of an inch program written with 4 (docs/specs/inch-units.md §4.3).
pub const INCH_ROUNDING_MM: f64 = 0.5 * 0.0001 * chipload_core::MM_PER_INCH;

/// G-codes the checker reads. 40, 49, 54, 61, 64 and 80 only set the controller to a known
/// state (docs/specs/user-posts.md §4); G91.1 is handled apart, as it is not whole.
const ALLOWED_G: [i64; 17] = [
    0, 1, 2, 3, 4, 17, 20, 21, 40, 49, 54, 61, 64, 80, 90, 91, 94,
];
/// M7, M8, M9: coolant on and off.
const ALLOWED_M: [i64; 8] = [3, 4, 5, 6, 7, 8, 9, 30];
/// How far above the depth already drilled a rapid back into a hole must stop, mm
/// (docs/specs/holes.md §6). The generator has its own constant: this copy is the
/// checker's, on purpose.
pub const PECK_CLEARANCE_MM: f64 = 0.5;
/// How far G64 may let the controller blend corners away from the program, mm.
pub const MAX_BLEND_MM: f64 = 0.05;
const MOTION_G: [i64; 4] = [0, 1, 2, 3];
const VALUE_LETTERS: &str = "XYZIJFSPNT";
const AXES: [char; 3] = ['X', 'Y', 'Z'];

/// Everything the checker knows about the job. Heights in work coordinates, mm.
#[derive(Debug, Clone, PartialEq)]
pub struct CheckLimits {
    pub safe_z: f64,
    pub stock_top_z: f64,
    pub stock_bottom_z: f64,
    /// How far below the stock bottom moves may go (spoilboard), mm: a warning within it,
    /// an error beyond.
    pub max_overcut: f64,
    pub stock_min: (f64, f64),
    pub stock_max: (f64, f64),
    /// The stock margin until a tool change (M6) says which tool cuts.
    pub tool_radius: f64,
    /// Every tool of the job by number: the radius after `T<n> M6`
    /// (docs/specs/tool-per-operation.md §6). Empty: tool changes are refused.
    pub tool_radii: BTreeMap<i64, f64>,
    /// Machine travel span per axis.
    pub travel: Option<(f64, f64, f64)>,
    /// The units the machine's G-code is in: a program in others is an error
    /// (docs/specs/inch-units.md §4.3).
    pub units: Units,
}

/// Derived here from the job's raw numbers, deliberately not via any generator code. The
/// stock margin is the largest radius of the tools the operations that are cut use (an
/// operation of a part that is switched off doesn't count).
pub fn limits_from_job(job: &Job) -> CheckLimits {
    let used: Vec<i64> = job
        .operations
        .iter()
        .enumerate()
        .filter(|(i, _)| job.cuts(*i))
        .map(|(_, op)| op.tool())
        .collect();
    let radius = job
        .tools
        .iter()
        .filter(|t| used.contains(&t.id))
        .map(|t| t.diameter / 2.0)
        .fold(0.0, f64::max);
    limits_with_radius(job, radius)
}

/// The same, for a program cut with one tool of the job (docs/specs/tool-per-operation.md
/// §6). None if the job has no tool with that id.
pub fn limits_for_tool(job: &Job, tool: i64) -> Option<CheckLimits> {
    let found = job.tool(tool)?;
    Some(limits_with_radius(job, found.diameter / 2.0))
}

fn limits_with_radius(job: &Job, radius: f64) -> CheckLimits {
    let stock = &job.stock;
    let top = stock.top_z();
    // The program is in work coordinates: the stock as placed in the drawing, shifted so
    // the project's work zero is at X0 Y0 (docs/specs/work-zero.md).
    let ((x0, y0), (x1, y1)) = job.stock_bounds();
    let (zx, zy) = job.work_zero_xy();
    let (stock_min, stock_max) = ((x0 - zx, y0 - zy), (x1 - zx, y1 - zy));
    CheckLimits {
        safe_z: job.machine.safe_z,
        stock_top_z: top,
        stock_bottom_z: top - stock.thickness,
        max_overcut: stock.max_overcut,
        stock_min,
        stock_max,
        tool_radius: radius,
        tool_radii: job.tools.iter().map(|t| (t.id, t.diameter / 2.0)).collect(),
        travel: job.machine.travel.as_ref().map(|t| (t.x, t.y, t.z)),
        units: job.machine.gcode_units,
    }
}

/// A finding. `message` is `text_message` in English and mm, for the command line and
/// `--json`; the app shows `text_message` in its language and units
/// (docs/specs/translations.md §7).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckIssue {
    /// 1-based; 0 for whole-program issues.
    pub line: usize,
    pub message: String,
    pub text_message: Message,
}

impl CheckIssue {
    pub fn new(line: usize, message: Message) -> Self {
        CheckIssue {
            line,
            message: message.english(),
            text_message: message,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CheckReport {
    pub errors: Vec<CheckIssue>,
    pub warnings: Vec<CheckIssue>,
}

impl CheckReport {
    pub fn ok(&self) -> bool {
        self.errors.is_empty()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SpindleState {
    Off,
    Cw,
    Ccw,
}

struct State {
    /// The program's units, from its G20 or G21.
    units: Option<Units>,
    /// How far a position may be off: more in an inch program, whose numbers are rounded
    /// more coarsely.
    tol: f64,
    absolute: Option<bool>,
    motion: Option<i64>,
    feed: Option<f64>,
    spindle: SpindleState,
    pos: BTreeMap<char, Option<f64>>,
    lo: BTreeMap<char, f64>,
    hi: BTreeMap<char, f64>,
    outside_stock: usize,
    /// Moves below the stock bottom within the overcut allowance, and the lowest Z of them.
    below_stock: usize,
    deepest: f64,
    ended: bool,
    /// The tool number of the last T word.
    tool: Option<i64>,
    /// The radius of the tool put in by the last M6; None: `limits.tool_radius`.
    radius: Option<f64>,
    /// After M6 the spindle must be started again with a speed.
    needs_speed: bool,
    /// The hole the tool last fed straight down into, (x, y, deepest z), while it has only
    /// moved straight up and down since: a rapid back into it is allowed
    /// (docs/specs/holes.md §6).
    hole: Option<(f64, f64, f64)>,
}

impl State {
    fn new() -> Self {
        Self {
            units: None,
            tol: POSITION_TOL_MM,
            absolute: None,
            motion: None,
            feed: None,
            spindle: SpindleState::Off,
            pos: AXES.iter().map(|&a| (a, None)).collect(),
            lo: BTreeMap::new(),
            hi: BTreeMap::new(),
            outside_stock: 0,
            below_stock: 0,
            deepest: f64::INFINITY,
            ended: false,
            tool: None,
            radius: None,
            needs_speed: false,
            hole: None,
        }
    }
}

pub fn check_gcode(text: &str, limits: &CheckLimits) -> CheckReport {
    let mut report = CheckReport::default();
    let mut state = State::new();
    for (index, raw) in parse::split_lines(text).into_iter().enumerate() {
        let line = index + 1;
        // `%` alone marks the start or end of a program (LinuxCNC, Mach).
        if raw.trim() == "%" {
            continue;
        }
        let words = match parse::parse_line(raw) {
            Ok(words) => words,
            Err(problem) => {
                report.errors.push(CheckIssue::new(line, problem));
                continue;
            }
        };
        if words.is_empty() {
            continue;
        }
        if state.ended {
            report
                .errors
                .push(CheckIssue::new(line, Message::new("check-after-end")));
        }
        for message in execute(&words, &mut state, limits) {
            report.errors.push(CheckIssue::new(line, message));
        }
    }

    if state.below_stock > 0 {
        let message = Message::new("check-below-stock")
            .arg("count", state.below_stock)
            .arg("deepest", Value::Length(state.deepest, Digits::Fixed(3)))
            .arg(
                "bottom",
                Value::Length(limits.stock_bottom_z, Digits::Fixed(3)),
            )
            .arg(
                "allowed",
                Value::Length(limits.max_overcut, Digits::Fixed(3)),
            );
        report.warnings.push(CheckIssue::new(0, message));
    }
    if state.outside_stock > 0 {
        let message = Message::new("check-outside-stock").arg("count", state.outside_stock);
        report.warnings.push(CheckIssue::new(0, message));
    }
    match limits.travel {
        None => report
            .warnings
            .push(CheckIssue::new(0, Message::new("check-no-travel"))),
        Some((tx, ty, tz)) => {
            for (axis, span_limit) in AXES.into_iter().zip([tx, ty, tz]) {
                if let (Some(lo), Some(hi)) = (state.lo.get(&axis), state.hi.get(&axis)) {
                    let span = hi - lo;
                    if span > span_limit + state.tol {
                        let message = Message::new("check-travel-exceeded")
                            .arg("axis", axis.to_string())
                            .arg("span", Value::Length(span, Digits::Fixed(3)))
                            .arg("travel", Value::Length(span_limit, Digits::Fixed(3)));
                        report.errors.push(CheckIssue::new(0, message));
                    }
                }
            }
        }
    }
    if !state.ended {
        report
            .warnings
            .push(CheckIssue::new(0, Message::new("check-no-end")));
    }
    report
}

fn execute(words: &[(char, f64)], state: &mut State, limits: &CheckLimits) -> Vec<Message> {
    let mut g_codes: Vec<i64> = Vec::new();
    let mut m_codes: Vec<i64> = Vec::new();
    let mut values: BTreeMap<char, f64> = BTreeMap::new();
    for &(letter, value) in words {
        if letter == 'G' && value == 91.1 {
            // Arc centres relative to the start point: what the checker assumes anyway.
            continue;
        }
        if letter == 'G' && value == 90.1 {
            return vec![Message::new("check-absolute-arc-centres")];
        }
        if letter == 'G' || letter == 'M' {
            if value != value.trunc() {
                return vec![not_supported(format!("{letter}{value}"))];
            }
            // `as` saturates for absurd values, which then fail the whitelist anyway.
            let code = value as i64;
            if letter == 'G' {
                g_codes.push(code);
            } else {
                m_codes.push(code);
            }
        } else if VALUE_LETTERS.contains(letter) {
            if values.contains_key(&letter) {
                return vec![Message::new("check-word-twice").arg("letter", letter.to_string())];
            }
            values.insert(letter, value);
        } else {
            return vec![Message::new("check-unsupported-word").arg("letter", letter.to_string())];
        }
    }

    let mut problems: Vec<Message> = Vec::new();
    for g in &g_codes {
        if !ALLOWED_G.contains(g) {
            problems.push(not_supported(format!("G{g}")));
        }
    }
    for m in &m_codes {
        if !ALLOWED_M.contains(m) {
            problems.push(not_supported(format!("M{m}")));
        }
    }
    let motions: Vec<i64> = g_codes
        .iter()
        .copied()
        .filter(|g| MOTION_G.contains(g))
        .collect();
    if motions.len() > 1 {
        problems.push(Message::new("check-several-motions"));
    }
    if !problems.is_empty() {
        return problems;
    }

    // Units first: every length and feed on the line is read in them, and kept in mm.
    for (code, units) in [(20, Units::Inch), (21, Units::Mm)] {
        if !g_codes.contains(&code) {
            continue;
        }
        if state.units.is_some_and(|before| before != units) {
            problems.push(
                Message::new("check-units-changed")
                    .arg("code", format!("G{code}"))
                    .arg("units", units.name()),
            );
        }
        if units != limits.units {
            problems.push(
                Message::new("check-units-differ")
                    .arg("code", format!("G{code}"))
                    .arg("units", units.name())
                    .arg("machine", limits.units.name()),
            );
        }
        state.units = Some(units);
        state.tol = match units {
            Units::Mm => POSITION_TOL_MM,
            Units::Inch => POSITION_TOL_MM + INCH_ROUNDING_MM,
        };
    }
    if let Some(units) = state.units {
        let k = units.mm_per_unit();
        for letter in ['X', 'Y', 'Z', 'I', 'J', 'F'] {
            if let Some(v) = values.get_mut(&letter) {
                *v *= k;
            }
        }
    }

    // Modal settings first, in the order a controller applies them.
    if let Some(&feed) = values.get(&'F') {
        if feed <= 0.0 {
            // The F word as the checker read it, after any inch conversion: not a feed
            // with a unit.
            problems.push(
                Message::new("check-feed-not-positive")
                    .arg("feed", Value::Number(feed, Digits::Shortest)),
            );
        }
        state.feed = Some(feed);
    }
    if g_codes.contains(&91) {
        problems.push(Message::new("check-relative"));
        state.absolute = Some(false);
    }
    if g_codes.contains(&90) {
        state.absolute = Some(true);
    }
    if let Some(&t) = values.get(&'T') {
        if t < 0.0 || t != t.trunc() {
            problems.push(
                Message::new("check-tool-not-whole")
                    .arg("tool", Value::Number(t, Digits::Shortest)),
            );
        } else {
            // `as` saturates for absurd values, which no project has as a tool.
            state.tool = Some(t as i64);
        }
    }
    // A new tool or a spindle start or stop: no rapid back into an earlier hole.
    if [3, 4, 5, 6].iter().any(|m| m_codes.contains(m)) {
        state.hole = None;
    }
    // A controller changes the tool before it starts or stops the spindle.
    if m_codes.contains(&6) {
        problems.extend(tool_change(state, limits));
    }
    if (m_codes.contains(&3) || m_codes.contains(&4)) && state.needs_speed {
        if !values.contains_key(&'S') {
            problems.push(Message::new("check-start-without-speed"));
        }
        state.needs_speed = false;
    }
    if m_codes.contains(&3) {
        state.spindle = SpindleState::Cw;
    }
    if m_codes.contains(&4) {
        state.spindle = SpindleState::Ccw;
    }
    if g_codes.contains(&4) && !values.contains_key(&'P') {
        problems.push(Message::new("check-dwell-without-p"));
    }
    if g_codes.contains(&64) {
        match values.get(&'P') {
            _ if g_codes.contains(&4) => {
                problems.push(Message::new("check-blend-and-dwell"));
            }
            Some(&p) if (0.0..=MAX_BLEND_MM).contains(&p) => {}
            // P is the program's own number, unconverted, as the check compares it.
            Some(&p) => problems.push(
                Message::new("check-blend-too-far")
                    .arg("p", Value::Number(p, Digits::Shortest))
                    .arg("most", Value::Length(MAX_BLEND_MM, Digits::Shortest)),
            ),
            None => problems.push(
                Message::new("check-blend-without-p")
                    .arg("most", Value::Number(MAX_BLEND_MM, Digits::Shortest)),
            ),
        }
    }
    if let Some(&first) = motions.first() {
        state.motion = Some(first);
    }

    let axes: BTreeMap<char, f64> = AXES
        .iter()
        .filter_map(|a| values.get(a).map(|v| (*a, *v)))
        .collect();
    let arc_command = matches!(motions.first(), Some(2 | 3));
    if !axes.is_empty() || arc_command {
        match state.motion {
            None => problems.push(Message::new("check-no-motion-mode")),
            Some(motion) => problems.extend(move_to(motion, &axes, &values, state, limits)),
        }
    }

    if m_codes.contains(&5) {
        state.spindle = SpindleState::Off;
    }
    if m_codes.contains(&30) {
        state.ended = true;
    }
    problems
}

/// M6: a known tool, the spindle stopped, the tool at safe height. From here on the stock
/// margin is the new tool's radius.
fn tool_change(state: &mut State, limits: &CheckLimits) -> Vec<Message> {
    let mut problems = Vec::new();
    match state.tool {
        None => problems.push(Message::new("check-change-without-tool")),
        Some(t) => match limits.tool_radii.get(&t) {
            Some(&radius) => state.radius = Some(radius),
            None => problems.push(Message::new("check-change-unknown-tool").arg("tool", t)),
        },
    }
    if state.spindle != SpindleState::Off {
        problems.push(Message::new("check-change-spindle-on"));
    }
    if state
        .pos
        .get(&'Z')
        .copied()
        .flatten()
        .is_none_or(|z| z < limits.safe_z - state.tol)
    {
        problems.push(Message::new("check-change-below-safe"));
    }
    state.needs_speed = true;
    problems
}

fn move_to(
    motion: i64,
    axes: &BTreeMap<char, f64>,
    values: &BTreeMap<char, f64>,
    state: &mut State,
    limits: &CheckLimits,
) -> Vec<Message> {
    let mut problems = Vec::new();
    if state.units.is_none() {
        problems.push(Message::new("check-motion-before-units"));
    }
    if state.absolute != Some(true) {
        problems.push(Message::new("check-motion-before-absolute"));
    }
    let start = state.pos.clone();
    let current = |a: char| start.get(&a).copied().flatten();
    let target: BTreeMap<char, Option<f64>> = AXES
        .iter()
        .map(|&a| (a, axes.get(&a).copied().or(current(a))))
        .collect();
    let at = |a: char| target.get(&a).copied().flatten();
    let xy_changes = ['X', 'Y']
        .iter()
        .any(|a| axes.get(a).is_some_and(|v| Some(*v) != current(*a)));

    if motion == 0 {
        let z_now = current('Z');
        if xy_changes && z_now.is_none_or(|z| z < limits.safe_z - state.tol) {
            problems.push(Message::new("check-rapid-below-safe"));
        }
        if let Some(z) = at('Z')
            && z < limits.stock_top_z - state.tol
            && !(!xy_changes && back_into_hole(state, current('X'), current('Y'), z))
        {
            problems.push(
                Message::new("check-rapid-into-material")
                    .arg("z", Value::Length(z, Digits::Fixed(3))),
            );
        }
    } else {
        if state.feed.is_none() {
            problems.push(Message::new("check-cut-without-feed"));
        }
        if state.spindle == SpindleState::Off {
            problems.push(Message::new("check-cut-spindle-off"));
        }
        if AXES.iter().any(|a| at(*a).is_none()) {
            problems.push(Message::new("check-cut-unknown-position"));
        }
        // An arc's own extremes count too: it can bulge past both its ends.
        let mut extremes: Vec<(f64, f64)> = Vec::new();
        if motion == 2 || motion == 3 {
            let (arc_problems, arc_extremes) = check_arc(&start, &target, values, motion == 2);
            problems.extend(arc_problems);
            extremes = arc_extremes;
        }
        if let (Some(x), Some(y)) = (at('X'), at('Y'))
            && (outside_stock(x, y, limits, state.radius)
                || extremes
                    .iter()
                    .any(|&(ex, ey)| outside_stock(ex, ey, limits, state.radius)))
        {
            state.outside_stock += 1;
        }
        for (ex, ey) in extremes {
            for (axis, v) in [('X', ex), ('Y', ey)] {
                let lo = state.lo.entry(axis).or_insert(v);
                *lo = lo.min(v);
                let hi = state.hi.entry(axis).or_insert(v);
                *hi = hi.max(v);
            }
        }
    }

    if let Some(z) = at('Z')
        && z < limits.stock_bottom_z - state.tol
    {
        let floor = limits.stock_bottom_z - limits.max_overcut;
        if z < floor - state.tol {
            problems.push(
                Message::new("check-below-stock-too-far")
                    .arg("z", Value::Length(z, Digits::Fixed(3)))
                    .arg(
                        "bottom",
                        Value::Length(limits.stock_bottom_z, Digits::Fixed(3)),
                    )
                    .arg(
                        "allowed",
                        Value::Length(limits.max_overcut, Digits::Fixed(3)),
                    ),
            );
        } else {
            state.below_stock += 1;
            state.deepest = state.deepest.min(z);
        }
    }

    for (&axis, value) in &target {
        if let Some(v) = *value {
            let lo = state.lo.entry(axis).or_insert(v);
            *lo = lo.min(v);
            let hi = state.hi.entry(axis).or_insert(v);
            *hi = hi.max(v);
        }
    }
    update_hole(state, motion, xy_changes, current('Z'), &target);
    state.pos = target;
    problems
}

/// Whether a rapid down to `z` at (x, y) goes back into the hole the tool just came out of,
/// stopping at least [`PECK_CLEARANCE_MM`] above its bottom.
fn back_into_hole(state: &State, x: Option<f64>, y: Option<f64>, z: f64) -> bool {
    let (Some(x), Some(y), Some((hx, hy, bottom))) = (x, y, state.hole) else {
        return false;
    };
    (x - hx).hypot(y - hy) <= state.tol && z >= bottom + PECK_CLEARANCE_MM - state.tol
}

/// After a move: a straight feed down records (or deepens) the hole; any XY move or arc
/// forgets it; straight up and down rapids keep it.
fn update_hole(
    state: &mut State,
    motion: i64,
    xy_changes: bool,
    z_before: Option<f64>,
    target: &BTreeMap<char, Option<f64>>,
) {
    let at = |a: char| target.get(&a).copied().flatten();
    if xy_changes || motion == 2 || motion == 3 {
        state.hole = None;
        return;
    }
    let (Some(x), Some(y), Some(z)) = (at('X'), at('Y'), at('Z')) else {
        state.hole = None;
        return;
    };
    let down = z_before.is_some_and(|before| z < before);
    if motion == 1 && down {
        let bottom = state.hole.map_or(z, |(_, _, b)| b.min(z));
        state.hole = Some((x, y, bottom));
    }
}

/// An arc's problems, and the points where it is furthest left, right, down and up (those
/// its sweep passes), for the stock and travel checks.
fn check_arc(
    start: &BTreeMap<char, Option<f64>>,
    target: &BTreeMap<char, Option<f64>>,
    values: &BTreeMap<char, f64>,
    clockwise: bool,
) -> (Vec<Message>, Vec<(f64, f64)>) {
    if !values.contains_key(&'I') && !values.contains_key(&'J') {
        return (vec![Message::new("check-arc-without-centre")], Vec::new());
    }
    let get = |m: &BTreeMap<char, Option<f64>>, a: char| m.get(&a).copied().flatten();
    let (Some(sx), Some(sy), Some(ex), Some(ey)) = (
        get(start, 'X'),
        get(start, 'Y'),
        get(target, 'X'),
        get(target, 'Y'),
    ) else {
        return (vec![Message::new("check-arc-unknown-position")], Vec::new());
    };
    let mut problems = Vec::new();
    // A helical arc (one that changes Z) goes linearly from its start Z to its end Z, so its
    // lowest point is one of its ends: the depth and stock checks on the end (and on the
    // previous move's end, its start) cover it (docs/specs/helix-and-leads.md part C).
    if (sx - ex).hypot(sy - ey) <= POSITION_TOL_MM {
        problems.push(Message::new("check-arc-full-circle"));
    }
    let cx = sx + values.get(&'I').copied().unwrap_or(0.0);
    let cy = sy + values.get(&'J').copied().unwrap_or(0.0);
    let r_start = (sx - cx).hypot(sy - cy);
    let r_end = (ex - cx).hypot(ey - cy);
    if (r_start - r_end).abs() > ARC_APPROX_TOL_MM {
        problems.push(
            Message::new("check-arc-radius-mismatch")
                .arg("start", Value::Length(r_start, Digits::Fixed(4)))
                .arg("end", Value::Length(r_end, Digits::Fixed(4))),
        );
    }
    // Angles from the start going the arc's way; an axis extreme is on the arc if it comes
    // before the end.
    let angle = |x: f64, y: f64| (y - cy).atan2(x - cx);
    let along = |a: f64| {
        let d = if clockwise {
            angle(sx, sy) - a
        } else {
            a - angle(sx, sy)
        };
        d.rem_euclid(std::f64::consts::TAU)
    };
    let end = along(angle(ex, ey));
    let r = r_start;
    let extremes = [(r, 0.0), (0.0, r), (-r, 0.0), (0.0, -r)]
        .into_iter()
        .filter(|&(dx, dy)| along(dy.atan2(dx)) < end)
        .map(|(dx, dy)| (cx + dx, cy + dy))
        .collect();
    (problems, extremes)
}

/// `word` (a G or M code: `G43`, `M7.5`) is not one the checker reads.
fn not_supported(word: String) -> Message {
    Message::new("check-not-supported").arg("word", word)
}

fn outside_stock(x: f64, y: f64, limits: &CheckLimits, radius: Option<f64>) -> bool {
    let margin = radius.unwrap_or(limits.tool_radius) + POSITION_TOL_MM;
    let inside_x = limits.stock_min.0 - margin <= x && x <= limits.stock_max.0 + margin;
    let inside_y = limits.stock_min.1 - margin <= y && y <= limits.stock_max.1 + margin;
    !(inside_x && inside_y)
}
