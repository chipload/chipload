// SPDX-License-Identifier: GPL-3.0-or-later
//! Post-processors: render a Program to G-code text through a template.
//!
//! Only generic work happens
//! here (number formatting and modal compression); anything machine specific lives in the
//! template. Built-in configs and templates live in `posts/` and `templates/` and are
//! compiled in (ADR 0007).

use std::path::Path;

use chipload_job::{OpRef, Spindle, Tool, ToolChange, Units};
use chipload_toolpath::arcs::merge_arcs;
use chipload_toolpath::model::{MoveKind, Program};
use minijinja::syntax::SyntaxConfig;
use minijinja::value::Serde;
use minijinja::{AutoEscape, Environment, UndefinedBehavior, Value};
use serde::{Deserialize, Serialize};

/// Post-processor config format version (a persisted format, see CLAUDE.md).
pub const CURRENT_VERSION: i64 = 1;

/// Built-in posts: (name, config TOML).
const BUILTIN_POSTS: [(&str, &str); 3] = [
    ("grbl", include_str!("../posts/grbl.toml")),
    ("linuxcnc", include_str!("../posts/linuxcnc.toml")),
    ("mach3", include_str!("../posts/mach3.toml")),
];

/// Templates a built-in config may name: (file name, source).
const TEMPLATES: [(&str, &str); 3] = [
    ("grbl.nc.j2", include_str!("../templates/grbl.nc.j2")),
    (
        "linuxcnc.ngc.j2",
        include_str!("../templates/linuxcnc.ngc.j2"),
    ),
    ("mach3.tap.j2", include_str!("../templates/mach3.tap.j2")),
];

/// Template steps a render may take per move, on top of [`RENDER_FUEL_BASE`]: a template
/// that loops without end fails instead of hanging (docs/specs/user-posts.md §2).
const RENDER_FUEL_PER_MOVE: u64 = 2_000;
const RENDER_FUEL_BASE: u64 = 1_000_000;

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum PostError {
    #[error(
        "Post-processor config schema_version {found} is not supported (this app supports up \
         to {current}). Update the application or check the file."
    )]
    UnsupportedVersion { found: String, current: i64 },
    #[error("{0}")]
    Config(String),
    #[error("{0}")]
    Render(String),
}

fn default_decimals() -> u32 {
    3
}

fn default_inch_decimals() -> u32 {
    4
}

fn default_units() -> Vec<Units> {
    vec![Units::Mm]
}

fn default_extension() -> String {
    ".nc".to_string()
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct PostConfig {
    /// The version of the settings format: 1.
    pub schema_version: i64,
    /// Who or what wrote the file, for reference.
    #[serde(default)]
    pub written_by: Option<String>,
    /// The post's display name, e.g. "GRBL".
    pub name: String,
    /// The template's file name, next to this file, e.g. `myrouter.nc.j2`.
    pub template: String,
    /// Decimals for coordinates and feeds in mm. Missing: 3.
    #[serde(default = "default_decimals")]
    pub decimals: u32,
    /// Decimals for coordinates and feeds in inches. Missing: 4.
    // Spec: docs/specs/inch-units.md §4.2.
    #[serde(default = "default_inch_decimals")]
    pub inch_decimals: u32,
    /// The units the template can write: `mm` (G21) and `inch` (G20) if it writes
    /// `{{ units_word }}` instead of a fixed G21. Missing: mm only, so a machine set to inch
    /// is refused rather than given inch numbers under G21.
    // Spec: docs/specs/inch-units.md §4.2.
    #[serde(default = "default_units")]
    pub units: Vec<Units>,
    /// The G-code file's extension, e.g. `.nc`. Missing: `.nc`.
    #[serde(default = "default_extension")]
    pub file_extension: String,
    /// Seconds to wait after starting the spindle, for it to reach speed; 0 for none.
    /// Required: a silent default could start cutting at low speed.
    pub spindle_dwell_s: f64,
    /// Write arcs as G2/G3; false writes them as short lines (G1). Missing: false.
    // Spec: docs/specs/arc-output.md.
    #[serde(default)]
    pub arcs: bool,
    /// How the template can write a job with several tools: `files` (one program per tool),
    /// and `program` (one file with tool changes) if the template writes them. Missing:
    /// files only.
    // Spec: docs/specs/tool-per-operation.md §3.2.
    #[serde(default = "default_tool_changes")]
    pub tool_changes: Vec<ToolChange>,
    /// A word for the operator, shown on the setup sheet, e.g. that the post isn't tested
    /// on a real controller yet. Missing: none.
    // Spec: docs/specs/setup-sheet.md §2.
    #[serde(default)]
    pub note: Option<String>,
    /// The template's text for a user post (docs/specs/user-posts.md); None: the built-in
    /// template `template` names. Not part of the file.
    #[serde(skip)]
    pub template_text: Option<String>,
}

/// A user post as read from the posts folder, or as copied into a project: the config's
/// and the template's text, exactly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserPost {
    pub config: String,
    pub template: String,
}

fn default_tool_changes() -> Vec<ToolChange> {
    vec![ToolChange::Files]
}

/// `^[A-Za-z0-9_][A-Za-z0-9_.-]*$`: a plain file or post name, no paths.
fn is_safe_name(name: &str) -> bool {
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-'))
}

/// Parse a post config from TOML text: check the version, migrate, validate.
pub fn parse_post_config(text: &str) -> Result<PostConfig, PostError> {
    let data: toml::Table = toml::from_str(text)
        .map_err(|e| PostError::Config(format!("Cannot read post-processor config: {e}")))?;
    let raw = data.get("schema_version");
    // TOML integers only; a bool, float or string is refused, never converted.
    match raw.and_then(toml::Value::as_integer) {
        Some(v) if (1..=CURRENT_VERSION).contains(&v) => {}
        _ => {
            return Err(PostError::UnsupportedVersion {
                found: raw.map_or("None".to_string(), ToString::to_string),
                current: CURRENT_VERSION,
            });
        }
    }
    // No migrations yet: version 1 is the only one.
    let config: PostConfig = toml::Value::Table(data)
        .try_into()
        .map_err(|e| PostError::Config(format!("Invalid post-processor config: {e}")))?;
    let mut problems = Vec::new();
    if !is_safe_name(&config.template) {
        problems.push(format!(
            "template must be a plain file name, got {:?}",
            config.template
        ));
    }
    if config.decimals > 6 {
        problems.push(format!(
            "decimals must be at most 6, got {}",
            config.decimals
        ));
    }
    if config.inch_decimals > 6 {
        problems.push(format!(
            "inch_decimals must be at most 6, got {}",
            config.inch_decimals
        ));
    }
    if config.units.is_empty() {
        problems.push("units must name at least one of mm and inch".into());
    }
    if !(config.spindle_dwell_s >= 0.0 && config.spindle_dwell_s.is_finite()) {
        problems.push(format!(
            "spindle_dwell_s must be at least 0, got {}",
            config.spindle_dwell_s
        ));
    }
    if problems.is_empty() {
        Ok(config)
    } else {
        Err(PostError::Config(format!(
            "Invalid post-processor config: {}",
            problems.join("; ")
        )))
    }
}

pub fn load_post_config(path: &Path) -> Result<PostConfig, PostError> {
    let text = std::fs::read_to_string(path).map_err(|e| {
        PostError::Config(format!(
            "Cannot read post-processor config {}: {e}",
            path.display()
        ))
    })?;
    parse_post_config(&text)
}

/// The names of the posts shipped with Chipload, as a machine's `post` names them.
pub fn builtin_posts() -> Vec<&'static str> {
    BUILTIN_POSTS.iter().map(|(name, _)| *name).collect()
}

/// A user post from its texts (a project's copy, or files just read): the config checked as
/// any post's, and the template compiled, so a syntax error shows now.
pub fn parse_user_post(post: &UserPost) -> Result<PostConfig, PostError> {
    let mut config = parse_post_config(&post.config)?;
    let template = post.template.replace("\r\n", "\n");
    let env = environment()?;
    env.template_from_str(&template).map_err(|e| {
        PostError::Config(format!("Template {:?} can't be read: {e}", config.template))
    })?;
    config.template_text = Some(template);
    Ok(config)
}

/// Read user post `name` from `dir`: `<name>.toml` and the template file it names, beside
/// it. A built-in's name is refused, as is a name that isn't a plain file name.
pub fn read_user_post(dir: &Path, name: &str) -> Result<UserPost, PostError> {
    if !is_safe_name(name) {
        return Err(PostError::Config(format!(
            "Invalid post-processor name {name:?}"
        )));
    }
    if builtin_posts().contains(&name) {
        return Err(PostError::Config(format!(
            "{name:?} is the name of a built-in post-processor; rename your file"
        )));
    }
    let read = |path: &Path| {
        std::fs::read_to_string(path)
            .map_err(|e| PostError::Config(format!("Cannot read {}: {e}", path.display())))
    };
    let config = read(&dir.join(format!("{name}.toml")))?;
    // The template's name comes from the config, checked to be a plain file name.
    let template_name = parse_post_config(&config)?.template;
    let template = read(&dir.join(template_name))?;
    let post = UserPost { config, template };
    parse_user_post(&post)?;
    Ok(post)
}

/// Every user post in `dir` (each `<name>.toml`), by name, with why it can't be used.
pub fn list_user_posts(dir: &Path) -> Vec<(String, Result<UserPost, PostError>)> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "toml"))
        .filter_map(|p| p.file_stem().map(|s| s.to_string_lossy().into_owned()))
        .collect();
    names.sort();
    names
        .into_iter()
        .map(|name| {
            let post = read_user_post(dir, &name);
            (name, post)
        })
        .collect()
}

/// Load a post shipped with Chipload, e.g. "grbl".
pub fn load_builtin_post(name: &str) -> Result<PostConfig, PostError> {
    if !is_safe_name(name) {
        return Err(PostError::Config(format!(
            "Invalid post-processor name {name:?}"
        )));
    }
    let (_, text) = BUILTIN_POSTS
        .iter()
        .find(|(n, _)| *n == name)
        .ok_or_else(|| PostError::Config(format!("Unknown post-processor {name:?}")))?;
    parse_post_config(text)
}

/// Fixed decimals, never "-0.000".
pub fn format_number(value: f64, decimals: u32) -> String {
    let text = format!("{value:.*}", decimals as usize);
    match text.strip_prefix('-') {
        Some(rest) if text.parse::<f64>().is_ok_and(|v| v == 0.0) => rest.to_string(),
        _ => text,
    }
}

/// One move with pre-formatted words. None means "unchanged, omit".
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PostMove {
    pub code: &'static str,
    pub x: Option<String>,
    pub y: Option<String>,
    pub z: Option<String>,
    /// Arc centre offsets from the start point (G2/G3 only).
    pub i: Option<String>,
    pub j: Option<String>,
    pub f: Option<String>,
    /// A comment to write before the move, made safe (a part's start,
    /// docs/specs/parts.md §6). Templates that don't use it lose nothing that is cut.
    pub comment: Option<String>,
}

/// Omit axis and feed words that have not changed; drop moves that go nowhere. Arcs always
/// state X, Y, I and J; I and J are measured from the start point as written (rounded), so the
/// controller sees the radius the numbers give.
pub fn compress_moves(program: &Program, decimals: u32) -> Result<Vec<PostMove>, PostError> {
    let mut last: [Option<String>; 3] = [None, None, None];
    let mut last_f: Option<String> = None;
    let mut result = Vec::new();
    // A comment on a move that goes nowhere waits for the next move that is written.
    let mut pending: Option<String> = None;
    for m in &program.moves {
        if let Some(text) = m.comment.and_then(|k| program.comments.get(k as usize)) {
            pending = Some(comment_safe(text));
        }
        let arc = matches!(m.kind, MoveKind::ArcCw | MoveKind::ArcCcw);
        let mut words: [Option<String>; 3] = [None, None, None];
        for (axis, value) in [m.x, m.y, m.z].into_iter().enumerate() {
            let text = value.map(|v| format_number(v, decimals));
            if text.is_some() && (text != last[axis] || (arc && axis < 2)) {
                words[axis] = text;
            }
        }
        if words.iter().all(Option::is_none) {
            continue;
        }
        let (mut i, mut j) = (None, None);
        if arc {
            let circle = m
                .circle
                .ok_or_else(|| PostError::Render("Arc move without a centre".into()))?;
            let start = |axis: usize| {
                last[axis]
                    .as_deref()
                    .and_then(|t| t.parse::<f64>().ok())
                    .ok_or_else(|| PostError::Render("Arc from an unknown position".into()))
            };
            let (sx, sy) = (start(0)?, start(1)?);
            let end = |axis: usize, value: Option<f64>| {
                words[axis]
                    .as_deref()
                    .or(last[axis].as_deref())
                    .and_then(|t| t.parse::<f64>().ok())
                    .or(value)
                    .ok_or_else(|| PostError::Render("Arc to an unknown position".into()))
            };
            let (ex, ey) = (end(0, m.x)?, end(1, m.y)?);
            let (ci, cj) = centre_offsets((sx, sy), (ex, ey), circle.center, decimals);
            i = Some(ci);
            j = Some(cj);
        }
        let mut feed = None;
        if m.kind != MoveKind::Rapid {
            let rate = m
                .feed
                .ok_or_else(|| PostError::Render("Feed move without a feed rate".into()))?;
            let text = format_number(rate, decimals);
            if Some(&text) != last_f.as_ref() {
                feed = Some(text.clone());
            }
            last_f = Some(text);
        }
        for axis in 0..3 {
            if words[axis].is_some() {
                last[axis].clone_from(&words[axis]);
            }
        }
        let [x, y, z] = words;
        result.push(PostMove {
            code: match m.kind {
                MoveKind::Rapid => "G0",
                MoveKind::Feed => "G1",
                MoveKind::ArcCw => "G2",
                MoveKind::ArcCcw => "G3",
            },
            x,
            y,
            z,
            i,
            j,
            f: feed,
            comment: pending.take(),
        });
    }
    Ok(result)
}

/// I and J as written: the centre offset rounded to the post's decimals, picking among the
/// neighbouring roundings the one whose start and end radius agree best (the ends are
/// already rounded). Deterministic: ties keep the plain rounding, then the first in order.
fn centre_offsets(
    start: (f64, f64),
    end: (f64, f64),
    centre: (f64, f64),
    decimals: u32,
) -> (String, String) {
    let step = 10f64.powi(-(decimals as i32));
    let round = |v: f64| (v / step).round() * step;
    let (i0, j0) = (round(centre.0 - start.0), round(centre.1 - start.1));
    let mismatch = |i: f64, j: f64| {
        let r_start = i.hypot(j);
        let r_end = (end.0 - (start.0 + i)).hypot(end.1 - (start.1 + j));
        (r_start - r_end).abs()
    };
    let mut best = (i0, j0, mismatch(i0, j0));
    for di in [-1.0, 0.0, 1.0] {
        for dj in [-1.0, 0.0, 1.0] {
            let (i, j) = (i0 + di * step, j0 + dj * step);
            let m = mismatch(i, j);
            if m < best.2 - 1e-12 {
                best = (i, j, m);
            }
        }
    }
    (
        format_number(best.0, decimals),
        format_number(best.1, decimals),
    )
}

#[derive(Serialize)]
struct Context {
    post_name: String,
    /// `G21` or `G20`: the program's units (docs/specs/inch-units.md §4.2).
    units_word: &'static str,
    /// `mm` or `inch`, for comments.
    unit: &'static str,
    tool_id: i64,
    tool_diameter: String,
    rpm: String,
    spindle: &'static str,
    safe_z: String,
    dwell: Option<String>,
    /// Where X0 Y0 Z0 is, for a header comment (docs/specs/work-zero.md); None when unknown
    /// (rendering a bare toolpath).
    work_zero: Option<String>,
    /// Which file of a set this is, when a job writes one file per tool group
    /// (docs/specs/tool-per-operation.md §3.1); None for a single file.
    file: Option<FileContext>,
    /// One file with tool changes: every tool, for the header; None otherwise.
    tools: Option<Vec<String>>,
    /// The moves, split where the tool changes; one segment without a change otherwise.
    segments: Vec<Segment>,
}

#[derive(Serialize)]
struct Segment {
    /// The tool change before these moves; None for the first tool.
    change: Option<ChangeContext>,
    moves: Vec<PostMove>,
}

#[derive(Serialize)]
struct ChangeContext {
    /// `Tool change 1 of 2: tool 2, 3.000 mm endmill, operations 2, 4`
    comment: String,
    tool_id: i64,
    rpm: String,
}

#[derive(Serialize)]
struct FileContext {
    number: usize,
    count: usize,
    /// `tool 2, 3.000 mm endmill "3 mm downcut"`
    tool: String,
    /// `2, 4` (1-based, as listed)
    operations: String,
}

/// One file of a job written as one file per tool group.
#[derive(Debug, Clone, PartialEq)]
pub struct FileHeader {
    /// 1-based, in cut order.
    pub number: usize,
    pub count: usize,
    pub tool: Tool,
    /// The operations and rest passes, in cut order.
    pub operations: Vec<OpRef>,
}

/// Text for a G-code comment: parentheses would end it early and ';' starts one, and
/// G-code is ASCII, so letters with accents lose them ("Lådor" -> "Lador"), a few signs
/// become words or letters ("60°" -> "60deg") and anything else becomes '?'.
fn comment_safe(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '(' => out.push('['),
            ')' => out.push(']'),
            ';' => out.push(','),
            c if c.is_ascii() && !c.is_ascii_control() => out.push(c),
            c => out.push_str(ascii_for(c)),
        }
    }
    out
}

/// A plain ASCII stand-in for a character outside it.
fn ascii_for(c: char) -> &'static str {
    match c {
        'å' | 'ä' | 'à' | 'á' | 'â' | 'ã' => "a",
        'Å' | 'Ä' | 'À' | 'Á' | 'Â' | 'Ã' => "A",
        'ö' | 'ø' | 'ò' | 'ó' | 'ô' | 'õ' => "o",
        'Ö' | 'Ø' | 'Ò' | 'Ó' | 'Ô' | 'Õ' => "O",
        'é' | 'è' | 'ê' | 'ë' => "e",
        'É' | 'È' | 'Ê' | 'Ë' => "E",
        'ü' | 'ú' | 'ù' | 'û' => "u",
        'Ü' | 'Ú' | 'Ù' | 'Û' => "U",
        'í' | 'ì' | 'î' | 'ï' => "i",
        'Í' | 'Ì' | 'Î' | 'Ï' => "I",
        'ç' => "c",
        'Ç' => "C",
        'ñ' => "n",
        'Ñ' => "N",
        'ß' => "ss",
        'æ' => "ae",
        'Æ' => "AE",
        '°' => "deg",
        '×' => "x",
        '±' => "+/-",
        'µ' => "u",
        '²' => "2",
        '³' => "3",
        '–' | '—' => "-",
        '‘' | '’' => "'",
        '“' | '”' => "\"",
        '…' => "...",
        ' ' | '\t' => " ",
        _ => "?",
    }
}

/// `tool 2, 3.000 mm endmill "3 mm downcut"`: the tool as a G-code comment shows it, its
/// diameter in the program's units.
pub fn tool_text(tool: &Tool, out: Output) -> String {
    let name = tool
        .name
        .as_deref()
        .map_or(String::new(), |n| format!(" \"{}\"", comment_safe(n)));
    format!(
        "tool {}, {} {} {}{name}",
        tool.id,
        out.number(tool.diameter / out.units.mm_per_unit()),
        out.units.name(),
        tool.kind.name()
    )
}

/// The units a program is written in and the decimals for them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Output {
    pub units: Units,
    pub decimals: u32,
}

impl Output {
    /// `units` as `config` writes them; refused if its template can't
    /// (docs/specs/inch-units.md §4.2).
    pub fn of(config: &PostConfig, units: Units) -> Result<Output, PostError> {
        if !config.units.contains(&units) {
            return Err(PostError::Config(format!(
                "Post-processor {:?} can't write G-code in {}: its config doesn't list {:?} \
                 under units (its template must write {{{{ units_word }}}} instead of G21)",
                config.name,
                units.name(),
                units.name()
            )));
        }
        let decimals = match units {
            Units::Mm => config.decimals,
            Units::Inch => config.inch_decimals,
        };
        Ok(Output { units, decimals })
    }

    fn number(self, value: f64) -> String {
        format_number(value, self.decimals)
    }

    fn units_word(self) -> &'static str {
        match self.units {
            Units::Mm => "G21",
            Units::Inch => "G20",
        }
    }
}

/// `program` in `units`: every coordinate, arc centre and radius, feed, the safe height and
/// the tool's diameter divided by the units' size in mm. Unchanged for mm.
pub fn in_units(program: &Program, units: Units) -> Program {
    let k = units.mm_per_unit();
    if units == Units::Mm {
        return program.clone();
    }
    let mut out = program.clone();
    out.tool_diameter /= k;
    out.safe_z /= k;
    for m in &mut out.moves {
        m.x = m.x.map(|v| v / k);
        m.y = m.y.map(|v| v / k);
        m.z = m.z.map(|v| v / k);
        m.feed = m.feed.map(|v| v / k);
        if let Some(c) = m.circle.as_mut() {
            c.center = (c.center.0 / k, c.center.1 / k);
            c.radius /= k;
        }
    }
    out
}

/// `2, 2.1, 4`: operations and rest passes as the user numbers them.
pub fn operations_text(operations: &[OpRef]) -> String {
    let numbers: Vec<String> = operations.iter().map(ToString::to_string).collect();
    numbers.join(", ")
}

/// Render in mm without a work-zero note (a bare toolpath, e.g. `chipload render`).
pub fn render_program(program: &Program, config: &PostConfig) -> Result<String, PostError> {
    render_program_with(program, config, None, Units::Mm)
}

/// Render in `units`, stating in the header where the work zero is. The program must
/// already be in program coordinates.
pub fn render_program_with(
    program: &Program,
    config: &PostConfig,
    work_zero: Option<&str>,
    units: Units,
) -> Result<String, PostError> {
    render(program, config, work_zero, None, units)
}

/// Render one file of a set written one file per tool group: the header also says which
/// file and tool it is (docs/specs/tool-per-operation.md §3.1).
pub fn render_tool_file(
    program: &Program,
    config: &PostConfig,
    work_zero: Option<&str>,
    file: &FileHeader,
    units: Units,
) -> Result<String, PostError> {
    render(program, config, work_zero, Some(file), units)
}

/// The moves to write: arcs merged (in mm, as their tolerances are) if the post writes
/// them, then in the program's units.
fn written(program: &Program, config: &PostConfig, out: Output) -> Result<Program, PostError> {
    // Chords of known arcs become G2/G3 only if the post writes arcs; otherwise the tags are
    // ignored and the output is the chords, as before.
    let merged = if config.arcs {
        merge_arcs(program).map_err(|e| PostError::Render(e.to_string()))?
    } else {
        program.clone()
    };
    Ok(in_units(&merged, out.units))
}

fn render(
    program: &Program,
    config: &PostConfig,
    work_zero: Option<&str>,
    file: Option<&FileHeader>,
    units: Units,
) -> Result<String, PostError> {
    let out = Output::of(config, units)?;
    let fmt = |v: f64| out.number(v);
    let output = written(program, config, out)?;
    let context = Context {
        post_name: comment_safe(&config.name),
        units_word: out.units_word(),
        unit: out.units.name(),
        tool_id: program.tool_id,
        tool_diameter: fmt(output.tool_diameter),
        rpm: format_number(program.rpm, 0),
        spindle: match program.spindle {
            Spindle::Cw => "cw",
            Spindle::Ccw => "ccw",
        },
        safe_z: fmt(output.safe_z),
        // Seconds: written the same in either units.
        dwell: (config.spindle_dwell_s > 0.0)
            .then(|| format_number(config.spindle_dwell_s, config.decimals)),
        work_zero: work_zero.map(comment_safe),
        file: file.map(|f| FileContext {
            number: f.number,
            count: f.count,
            tool: tool_text(&f.tool, out),
            operations: operations_text(&f.operations),
        }),
        tools: None,
        segments: vec![Segment {
            change: None,
            moves: compress_moves(&output, out.decimals)?,
        }],
    };
    fill_template(config, &context)
}

/// One part of a job written as one file with tool changes: a tool group's moves.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolPart {
    /// The group's moves with its tool, in program coordinates.
    pub program: Program,
    pub tool: Tool,
    /// The operations and rest passes, in cut order.
    pub operations: Vec<OpRef>,
}

/// Render a job with several tools as one program with a tool change before each part but
/// the first (docs/specs/tool-per-operation.md §3.2). The post must support it.
pub fn render_with_tool_changes(
    parts: &[ToolPart],
    config: &PostConfig,
    work_zero: Option<&str>,
    units: Units,
) -> Result<String, PostError> {
    if !config.tool_changes.contains(&ToolChange::Program) {
        return Err(PostError::Config(format!(
            "Post-processor {:?} can't write tool changes in one file; write one file per \
             tool instead (tool changes: files)",
            config.name
        )));
    }
    let first = parts
        .first()
        .ok_or_else(|| PostError::Render("No tool groups to render".into()))?;
    let out = Output::of(config, units)?;
    let fmt = |v: f64| out.number(v);
    let mut segments = Vec::new();
    for (i, part) in parts.iter().enumerate() {
        let output = written(&part.program, config, out)?;
        // Each part starts from fresh modal state: after a tool change every word is written
        // again, whatever the sender did to move the machine.
        let change = (i > 0).then(|| ChangeContext {
            comment: comment_safe(&format!(
                "Tool change {i} of {}: {}, operations {}",
                parts.len() - 1,
                tool_text(&part.tool, out),
                operations_text(&part.operations)
            )),
            tool_id: part.tool.id,
            rpm: format_number(part.program.rpm, 0),
        });
        segments.push(Segment {
            change,
            moves: compress_moves(&output, out.decimals)?,
        });
    }
    let context = Context {
        post_name: comment_safe(&config.name),
        units_word: out.units_word(),
        unit: out.units.name(),
        tool_id: first.program.tool_id,
        tool_diameter: fmt(first.program.tool_diameter / out.units.mm_per_unit()),
        rpm: format_number(first.program.rpm, 0),
        spindle: match first.program.spindle {
            Spindle::Cw => "cw",
            Spindle::Ccw => "ccw",
        },
        safe_z: fmt(first.program.safe_z / out.units.mm_per_unit()),
        // Seconds: written the same in either units.
        dwell: (config.spindle_dwell_s > 0.0)
            .then(|| format_number(config.spindle_dwell_s, config.decimals)),
        work_zero: work_zero.map(comment_safe),
        file: None,
        tools: Some(tool_list(parts, out)),
        segments,
    };
    fill_template(config, &context)
}

/// `T2 3.000 mm endmill "3 mm downcut"` for each tool once, in the order first used.
fn tool_list(parts: &[ToolPart], out: Output) -> Vec<String> {
    let mut seen: Vec<i64> = Vec::new();
    let mut lines = Vec::new();
    for part in parts {
        if seen.contains(&part.tool.id) {
            continue;
        }
        seen.push(part.tool.id);
        let text = tool_text(&part.tool, out);
        // "tool 2, 3.000 mm endmill" -> "T2 3.000 mm endmill"
        let rest = text
            .split_once(", ")
            .map_or(text.as_str(), |(_, rest)| rest);
        lines.push(comment_safe(&format!("T{} {rest}", part.tool.id)));
    }
    lines
}

/// The template environment every post renders in: Jinja-style whitespace, strict
/// undefined, no escaping. minijinja sees nothing but the context, so templates are
/// sandboxed by design.
fn environment() -> Result<Environment<'static>, PostError> {
    let mut env = Environment::new();
    env.set_syntax(
        SyntaxConfig::builder()
            .trim_blocks(true)
            .lstrip_blocks(true)
            .keep_trailing_newline(true)
            .build()
            .map_err(|e| PostError::Config(format!("Template syntax settings: {e}")))?,
    );
    env.set_undefined_behavior(UndefinedBehavior::Strict);
    env.set_auto_escape_callback(|_| AutoEscape::None);
    Ok(env)
}

fn fill_template(config: &PostConfig, context: &Context) -> Result<String, PostError> {
    let failed = |e: minijinja::Error| {
        PostError::Render(format!("Template {:?} failed: {e}", config.template))
    };
    let source = match &config.template_text {
        Some(text) => text.as_str(),
        None => TEMPLATES
            .iter()
            .find(|(name, _)| *name == config.template)
            .map(|(_, source)| *source)
            .ok_or_else(|| PostError::Config(format!("Unknown template {:?}", config.template)))?,
    };
    let mut env = environment()?;
    let moves: usize = context.segments.iter().map(|s| s.moves.len()).sum();
    env.set_fuel(Some(
        RENDER_FUEL_BASE.saturating_add(RENDER_FUEL_PER_MOVE.saturating_mul(moves as u64)),
    ));
    // The same output on every platform, whatever line endings the template was checked
    // out with (git may give a Windows build CRLF).
    let source = source.replace("\r\n", "\n");
    env.add_template_owned(config.template.clone(), source)
        .map_err(failed)?;
    let text = env
        .get_template(&config.template)
        .and_then(|t| t.render(Value::from(Serde(context))))
        .map_err(failed)?;
    Ok(if text.ends_with('\n') {
        text
    } else {
        text + "\n"
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_part_comment_is_written_before_its_move_even_if_that_move_goes_nowhere() {
        // docs/specs/parts.md §6: the comment on a move that is dropped (already at safe Z)
        // goes with the next move written.
        let mut start = Move::rapid_z(5.0);
        start.comment = Some(0);
        let program = Program {
            tool_id: 1,
            tool_diameter: 6.0,
            rpm: 18000.0,
            spindle: Spindle::Cw,
            safe_z: 5.0,
            moves: vec![Move::rapid_z(5.0), start, Move::rapid_xy((1.0, 2.0))],
            comments: vec!["Part 2: Lådor (box), 2 copies".into()],
        };
        let config = load_builtin_post("grbl").unwrap();
        let text = render_program(&program, &config).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        let at = lines
            .iter()
            .position(|l| *l == "(Part 2: Lador [box], 2 copies)")
            .unwrap_or_else(|| panic!("{text}"));
        assert_eq!(lines[at + 1], "G0 X1.000 Y2.000");
    }

    #[test]
    fn comments_are_ascii_and_never_end_early() {
        assert_eq!(comment_safe("60° V-bit (fine)"), "60deg V-bit [fine]");
        assert_eq!(comment_safe("Lådor; Öppning"), "Lador, Oppning");
        assert_eq!(comment_safe("Ø6 ×2 ☃"), "O6 x2 ?");
        assert!(comment_safe("\u{1f600}\nx").is_ascii());
    }
    use chipload_toolpath::model::Move;

    #[test]
    fn numbers_never_show_negative_zero() {
        assert_eq!(format_number(-0.0004, 3), "0.000");
        assert_eq!(format_number(-0.0006, 3), "-0.001");
        assert_eq!(format_number(0.125, 2), "0.12"); // ties to even
        assert_eq!(format_number(18000.0, 0), "18000");
    }

    #[test]
    fn grbl_config_loads() {
        let config = load_builtin_post("grbl").unwrap();
        assert_eq!(config.name, "GRBL");
        assert_eq!(config.decimals, 3);
        assert_eq!(config.spindle_dwell_s, 3.0);
    }

    #[test]
    fn posts_not_tested_on_a_machine_say_so_in_their_note() {
        // Shown on the setup sheet (docs/specs/setup-sheet.md §2).
        assert_eq!(load_builtin_post("grbl").unwrap().note, None);
        for name in ["linuxcnc", "mach3"] {
            let note = load_builtin_post(name).unwrap().note.unwrap();
            assert!(note.starts_with("Not yet tested on a real"), "{note}");
        }
    }

    #[test]
    fn unknown_or_unsafe_post_names_are_refused() {
        assert!(load_builtin_post("../grbl").is_err());
        assert!(load_builtin_post("nope").is_err());
    }

    #[test]
    fn versions_are_checked() {
        let body = "name = \"X\"\ntemplate = \"grbl.nc.j2\"\nspindle_dwell_s = 0\n";
        for version in [
            "",
            "schema_version = 2\n",
            "schema_version = true\n",
            "schema_version = 1.0\n",
        ] {
            let err = parse_post_config(&format!("{version}{body}")).unwrap_err();
            assert!(
                matches!(err, PostError::UnsupportedVersion { .. }),
                "{version}: {err}"
            );
        }
        assert!(parse_post_config(&format!("schema_version = 1\n{body}")).is_ok());
    }

    #[test]
    fn dwell_is_required_and_template_must_be_a_plain_name() {
        let missing = "schema_version = 1\nname = \"X\"\ntemplate = \"grbl.nc.j2\"\n";
        assert!(parse_post_config(missing).is_err());
        let path = "schema_version = 1\nname = \"X\"\ntemplate = \"../x\"\nspindle_dwell_s = 0\n";
        assert!(parse_post_config(path).is_err());
    }

    #[test]
    fn output_has_unix_line_endings_on_every_platform() {
        // A Windows checkout may give the template CRLF; the G-code must not change.
        let program = Program {
            tool_id: 1,
            tool_diameter: 6.0,
            rpm: 18000.0,
            spindle: Spindle::Cw,
            safe_z: 5.0,
            moves: vec![Move::rapid_z(5.0), Move::rapid_xy((1.0, 2.0))],
            comments: Vec::new(),
        };
        let config = load_builtin_post("grbl").unwrap();
        let text = render_program_with(&program, &config, Some("x (y); z"), Units::Mm).unwrap();
        assert!(!text.contains('\r'), "{text:?}");
        assert!(text.contains("(Work zero: x [y], z)\n"), "{text}");
    }

    #[test]
    fn modal_compression() {
        let program = Program {
            tool_id: 1,
            tool_diameter: 6.0,
            rpm: 18000.0,
            spindle: Spindle::Cw,
            safe_z: 5.0,
            moves: vec![
                Move::rapid_z(5.0),
                Move::rapid_xy((1.0, 2.0)),
                Move::feed_z(-1.0, 300.0),
                Move::feed_xy((10.0, 2.0), 1000.0),
                Move::feed_xy((10.0, 2.0), 1000.0), // goes nowhere: dropped
                Move::feed_xy((10.0, 5.0), 1000.0),
                Move::rapid_z(5.0),
            ],
            comments: Vec::new(),
        };
        let config = load_builtin_post("grbl").unwrap();
        let text = render_program(&program, &config).unwrap();
        let expected = "(Chipload - GRBL)\n(Tool 1: 6.000 mm)\nG21 G90 G17 G94\nM3 S18000\n\
                        G4 P3.000\nG0 Z5.000\nG0 X1.000 Y2.000\nG1 Z-1.000 F300.000\n\
                        G1 X10.000 F1000.000\nG1 Y5.000\nG0 Z5.000\nM5\nM30\n";
        assert_eq!(text, expected);
    }
}
