// SPDX-License-Identifier: GPL-3.0-or-later
//! Independent G-code checker.
//!
//! Parses G-code *text* with its own tokenizer and modal state and checks it against the safety
//! rules in `docs/specs/v0-profile-pipeline.md` section 6. It must never depend on the toolpath
//! generator or post-processor crates: its value is that it can disagree with them.
//!
//! Port of `src/chipload/core/sim/gcode_check.py`; messages are kept identical so both
//! implementations can be compared on the same programs. Deliberate difference: only ASCII
//! digits are accepted in numbers (Python's `\d` also accepts other Unicode digits).

mod parse;

use std::collections::BTreeMap;

use chipload_core::tolerances::{ARC_APPROX_TOL_MM, CHAIN_TOL_MM};
use chipload_job::{Job, ZZero};

/// Positional slack for comparisons against heights and bounds.
pub const POSITION_TOL_MM: f64 = CHAIN_TOL_MM;

const ALLOWED_G: [i64; 11] = [0, 1, 2, 3, 4, 17, 20, 21, 90, 91, 94];
const ALLOWED_M: [i64; 4] = [3, 4, 5, 30];
const MOTION_G: [i64; 4] = [0, 1, 2, 3];
const VALUE_LETTERS: &str = "XYZIJFSPN";
const AXES: [char; 3] = ['X', 'Y', 'Z'];

/// Everything the checker knows about the job. Heights in work coordinates, mm.
#[derive(Debug, Clone, PartialEq)]
pub struct CheckLimits {
    pub safe_z: f64,
    pub stock_top_z: f64,
    pub stock_bottom_z: f64,
    pub stock_min: (f64, f64),
    pub stock_max: (f64, f64),
    pub tool_radius: f64,
    /// Machine travel span per axis.
    pub travel: Option<(f64, f64, f64)>,
}

/// Derived here from the job's raw numbers, deliberately not via any generator code.
pub fn limits_from_job(job: &Job) -> CheckLimits {
    let stock = &job.stock;
    let top = match stock.z_zero {
        ZZero::Top => 0.0,
        ZZero::Bottom => stock.thickness,
    };
    let used: Vec<i64> = job.operations.iter().map(|op| op.tool()).collect();
    let radius = job
        .tools
        .iter()
        .filter(|t| used.contains(&t.id))
        .map(|t| t.diameter / 2.0)
        .fold(0.0, f64::max);
    let (stock_min, stock_max) = stock.xy_bounds();
    CheckLimits {
        safe_z: job.machine.safe_z,
        stock_top_z: top,
        stock_bottom_z: top - stock.thickness,
        stock_min,
        stock_max,
        tool_radius: radius,
        travel: job.machine.travel.as_ref().map(|t| (t.x, t.y, t.z)),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckIssue {
    /// 1-based; 0 for whole-program issues.
    pub line: usize,
    pub message: String,
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
    units_mm: Option<bool>,
    absolute: Option<bool>,
    motion: Option<i64>,
    feed: Option<f64>,
    spindle: SpindleState,
    pos: BTreeMap<char, Option<f64>>,
    lo: BTreeMap<char, f64>,
    hi: BTreeMap<char, f64>,
    outside_stock: usize,
    ended: bool,
}

impl State {
    fn new() -> Self {
        Self {
            units_mm: None,
            absolute: None,
            motion: None,
            feed: None,
            spindle: SpindleState::Off,
            pos: AXES.iter().map(|&a| (a, None)).collect(),
            lo: BTreeMap::new(),
            hi: BTreeMap::new(),
            outside_stock: 0,
            ended: false,
        }
    }
}

pub fn check_gcode(text: &str, limits: &CheckLimits) -> CheckReport {
    let mut report = CheckReport::default();
    let mut state = State::new();
    for (index, raw) in parse::split_lines(text).into_iter().enumerate() {
        let line = index + 1;
        let words = match parse::parse_line(raw) {
            Ok(words) => words,
            Err(problem) => {
                report.errors.push(CheckIssue {
                    line,
                    message: problem,
                });
                continue;
            }
        };
        if words.is_empty() {
            continue;
        }
        if state.ended {
            report.errors.push(CheckIssue {
                line,
                message: "Code after program end (M30)".into(),
            });
        }
        for message in execute(&words, &mut state, limits) {
            report.errors.push(CheckIssue { line, message });
        }
    }

    if state.outside_stock > 0 {
        report.warnings.push(CheckIssue {
            line: 0,
            message: format!(
                "{} cutting moves end outside the stock (expanded by the tool radius)",
                state.outside_stock
            ),
        });
    }
    match limits.travel {
        None => report.warnings.push(CheckIssue {
            line: 0,
            message: "Machine travel not set; travel limits were not checked".into(),
        }),
        Some((tx, ty, tz)) => {
            for (axis, span_limit) in AXES.into_iter().zip([tx, ty, tz]) {
                if let (Some(lo), Some(hi)) = (state.lo.get(&axis), state.hi.get(&axis)) {
                    let span = hi - lo;
                    if span > span_limit + POSITION_TOL_MM {
                        report.errors.push(CheckIssue {
                            line: 0,
                            message: format!(
                                "{axis} span {span:.3} mm exceeds machine travel {span_limit:.3}"
                            ),
                        });
                    }
                }
            }
        }
    }
    if !state.ended {
        report.warnings.push(CheckIssue {
            line: 0,
            message: "No program end (M30)".into(),
        });
    }
    report
}

fn execute(words: &[(char, f64)], state: &mut State, limits: &CheckLimits) -> Vec<String> {
    let mut g_codes: Vec<i64> = Vec::new();
    let mut m_codes: Vec<i64> = Vec::new();
    let mut values: BTreeMap<char, f64> = BTreeMap::new();
    for &(letter, value) in words {
        if letter == 'G' || letter == 'M' {
            if value != value.trunc() {
                return vec![format!(
                    "{letter}{} is not supported",
                    parse::py_float(value)
                )];
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
                return vec![format!("{letter} appears twice on one line")];
            }
            values.insert(letter, value);
        } else {
            return vec![format!("Unsupported word {letter}")];
        }
    }

    let mut problems: Vec<String> = Vec::new();
    for g in &g_codes {
        if !ALLOWED_G.contains(g) {
            problems.push(format!("G{g} is not supported"));
        }
    }
    for m in &m_codes {
        if !ALLOWED_M.contains(m) {
            problems.push(format!("M{m} is not supported"));
        }
    }
    let motions: Vec<i64> = g_codes
        .iter()
        .copied()
        .filter(|g| MOTION_G.contains(g))
        .collect();
    if motions.len() > 1 {
        problems.push("More than one motion command on one line".into());
    }
    if !problems.is_empty() {
        return problems;
    }

    // Modal settings first, in the order a controller applies them.
    if let Some(&feed) = values.get(&'F') {
        if feed <= 0.0 {
            problems.push(format!("Feed F{} must be positive", parse::py_float(feed)));
        }
        state.feed = Some(feed);
    }
    if g_codes.contains(&20) {
        problems.push("G20 (inch) used; programs must be in mm".into());
        state.units_mm = Some(false);
    }
    if g_codes.contains(&21) {
        state.units_mm = Some(true);
    }
    if g_codes.contains(&91) {
        problems.push("G91 (relative) is not supported".into());
        state.absolute = Some(false);
    }
    if g_codes.contains(&90) {
        state.absolute = Some(true);
    }
    if m_codes.contains(&3) {
        state.spindle = SpindleState::Cw;
    }
    if m_codes.contains(&4) {
        state.spindle = SpindleState::Ccw;
    }
    if g_codes.contains(&4) && !values.contains_key(&'P') {
        problems.push("G4 dwell without P".into());
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
            None => problems.push("Axis words without a motion mode".into()),
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

fn move_to(
    motion: i64,
    axes: &BTreeMap<char, f64>,
    values: &BTreeMap<char, f64>,
    state: &mut State,
    limits: &CheckLimits,
) -> Vec<String> {
    let mut problems = Vec::new();
    if state.units_mm != Some(true) {
        problems.push("Motion before G21 (mm)".into());
    }
    if state.absolute != Some(true) {
        problems.push("Motion before G90 (absolute)".into());
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
        if xy_changes && z_now.is_none_or(|z| z < limits.safe_z - POSITION_TOL_MM) {
            problems.push("Rapid XY move while Z is below safe height (retract first)".into());
        }
        if let Some(z) = at('Z')
            && z < limits.stock_top_z - POSITION_TOL_MM
        {
            problems.push(format!(
                "Rapid to Z{z:.3}, below the stock top (rapid into material)"
            ));
        }
    } else {
        if state.feed.is_none() {
            problems.push("Cutting move with no feed set".into());
        }
        if state.spindle == SpindleState::Off {
            problems.push("Cutting move with the spindle off".into());
        }
        if AXES.iter().any(|a| at(*a).is_none()) {
            problems.push("Cutting move from an unknown position".into());
        }
        if motion == 2 || motion == 3 {
            problems.extend(check_arc(&start, &target, values));
        }
        if let (Some(x), Some(y)) = (at('X'), at('Y'))
            && outside_stock(x, y, limits)
        {
            state.outside_stock += 1;
        }
    }

    if let Some(z) = at('Z')
        && z < limits.stock_bottom_z - POSITION_TOL_MM
    {
        problems.push(format!(
            "Z{z:.3} is below the stock bottom ({:.3})",
            limits.stock_bottom_z
        ));
    }

    for (&axis, value) in &target {
        if let Some(v) = *value {
            let lo = state.lo.entry(axis).or_insert(v);
            *lo = lo.min(v);
            let hi = state.hi.entry(axis).or_insert(v);
            *hi = hi.max(v);
        }
    }
    state.pos = target;
    problems
}

fn check_arc(
    start: &BTreeMap<char, Option<f64>>,
    target: &BTreeMap<char, Option<f64>>,
    values: &BTreeMap<char, f64>,
) -> Vec<String> {
    if !values.contains_key(&'I') && !values.contains_key(&'J') {
        return vec!["Arc without I/J centre offsets".into()];
    }
    let get = |m: &BTreeMap<char, Option<f64>>, a: char| m.get(&a).copied().flatten();
    let (Some(sx), Some(sy), Some(ex), Some(ey)) = (
        get(start, 'X'),
        get(start, 'Y'),
        get(target, 'X'),
        get(target, 'Y'),
    ) else {
        return vec!["Arc from an unknown position".into()];
    };
    let cx = sx + values.get(&'I').copied().unwrap_or(0.0);
    let cy = sy + values.get(&'J').copied().unwrap_or(0.0);
    let r_start = (sx - cx).hypot(sy - cy);
    let r_end = (ex - cx).hypot(ey - cy);
    if (r_start - r_end).abs() > ARC_APPROX_TOL_MM {
        return vec![format!(
            "Arc radius mismatch: start {r_start:.4} vs end {r_end:.4}"
        )];
    }
    Vec::new()
}

fn outside_stock(x: f64, y: f64, limits: &CheckLimits) -> bool {
    let margin = limits.tool_radius + POSITION_TOL_MM;
    let inside_x = limits.stock_min.0 - margin <= x && x <= limits.stock_max.0 + margin;
    let inside_y = limits.stock_min.1 - margin <= y && y <= limits.stock_max.1 + margin;
    !(inside_x && inside_y)
}
