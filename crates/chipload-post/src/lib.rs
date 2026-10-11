// SPDX-License-Identifier: GPL-3.0-or-later
//! Post-processors: render a Program to G-code text through a template.
//!
//! Only generic work happens
//! here (number formatting and modal compression); anything machine specific lives in the
//! template. Built-in configs and templates live in `posts/` and `templates/` and are
//! compiled in (ADR 0007).

use std::path::Path;

use chipload_job::{Spindle, Tool, ToolChange, ToolType};
use chipload_toolpath::arcs::merge_arcs;
use chipload_toolpath::model::{MoveKind, Program};
use minijinja::syntax::SyntaxConfig;
use minijinja::value::Serde;
use minijinja::{AutoEscape, Environment, UndefinedBehavior, Value};
use serde::{Deserialize, Serialize};

/// Post-processor config format version (a persisted format, see CLAUDE.md).
pub const CURRENT_VERSION: i64 = 1;

/// Built-in posts: (name, config TOML).
const BUILTIN_POSTS: [(&str, &str); 1] = [("grbl", include_str!("../posts/grbl.toml"))];

/// Templates a config may name: (file name, source).
const TEMPLATES: [(&str, &str); 1] = [("grbl.nc.j2", include_str!("../templates/grbl.nc.j2"))];

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

fn default_extension() -> String {
    ".nc".to_string()
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PostConfig {
    pub schema_version: i64,
    #[serde(default)]
    pub written_by: Option<String>,
    pub name: String,
    /// File name of a built-in template.
    pub template: String,
    #[serde(default = "default_decimals")]
    pub decimals: u32,
    #[serde(default = "default_extension")]
    pub file_extension: String,
    /// Required: a silent 0 s default could cut at low rpm.
    pub spindle_dwell_s: f64,
    /// Write arcs as G2/G3 (docs/specs/arc-output.md). Missing: false, chords as G1, which
    /// is what every post wrote before this setting existed.
    #[serde(default)]
    pub arcs: bool,
    /// How the template can write a job with several tools (docs/specs/tool-per-operation.md
    /// §3.2): "files" (one program per tool) and, if it writes the commands, "program" (one
    /// file with tool changes). Missing: files only, so a template that knows nothing of M6
    /// never gets asked for it.
    #[serde(default = "default_tool_changes")]
    pub tool_changes: Vec<ToolChange>,
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
}

/// Omit axis and feed words that have not changed; drop moves that go nowhere. Arcs always
/// state X, Y, I and J; I and J are measured from the start point as written (rounded), so the
/// controller sees the radius the numbers give.
pub fn compress_moves(program: &Program, decimals: u32) -> Result<Vec<PostMove>, PostError> {
    let mut last: [Option<String>; 3] = [None, None, None];
    let mut last_f: Option<String> = None;
    let mut result = Vec::new();
    for m in &program.moves {
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
    /// Indexes in `job.operations`.
    pub operations: Vec<usize>,
}

/// Parentheses would end a G-code comment early, and ';' starts one.
fn comment_safe(text: &str) -> String {
    text.replace('(', "[").replace(')', "]").replace(';', ",")
}

/// `tool 2, 3.000 mm endmill "3 mm downcut"`: the tool as a G-code comment shows it.
pub fn tool_text(tool: &Tool, decimals: u32) -> String {
    let kind = match tool.kind {
        ToolType::Endmill => "endmill",
        ToolType::Drill => "drill",
        ToolType::Vbit => "V-bit",
    };
    let name = tool
        .name
        .as_deref()
        .map_or(String::new(), |n| format!(" \"{}\"", comment_safe(n)));
    format!(
        "tool {}, {} mm {kind}{name}",
        tool.id,
        format_number(tool.diameter, decimals)
    )
}

/// `2, 4`: operation indexes as the user numbers them.
pub fn operations_text(operations: &[usize]) -> String {
    let numbers: Vec<String> = operations.iter().map(|i| (i + 1).to_string()).collect();
    numbers.join(", ")
}

/// Render without a work-zero note (a bare toolpath, e.g. `chipload render`).
pub fn render_program(program: &Program, config: &PostConfig) -> Result<String, PostError> {
    render_program_with(program, config, None)
}

/// Render, stating in the header where the work zero is. The program must already be in
/// program coordinates.
pub fn render_program_with(
    program: &Program,
    config: &PostConfig,
    work_zero: Option<&str>,
) -> Result<String, PostError> {
    render(program, config, work_zero, None)
}

/// Render one file of a set written one file per tool group: the header also says which
/// file and tool it is (docs/specs/tool-per-operation.md §3.1).
pub fn render_tool_file(
    program: &Program,
    config: &PostConfig,
    work_zero: Option<&str>,
    file: &FileHeader,
) -> Result<String, PostError> {
    render(program, config, work_zero, Some(file))
}

fn render(
    program: &Program,
    config: &PostConfig,
    work_zero: Option<&str>,
    file: Option<&FileHeader>,
) -> Result<String, PostError> {
    let fmt = |v: f64| format_number(v, config.decimals);
    // Chords of known arcs become G2/G3 only if the post writes arcs; otherwise the tags are
    // ignored and the output is the chords, as before.
    let output = if config.arcs {
        merge_arcs(program).map_err(|e| PostError::Render(e.to_string()))?
    } else {
        program.clone()
    };
    let context = Context {
        post_name: config.name.clone(),
        tool_id: program.tool_id,
        tool_diameter: fmt(program.tool_diameter),
        rpm: format_number(program.rpm, 0),
        spindle: match program.spindle {
            Spindle::Cw => "cw",
            Spindle::Ccw => "ccw",
        },
        safe_z: fmt(program.safe_z),
        dwell: (config.spindle_dwell_s > 0.0).then(|| fmt(config.spindle_dwell_s)),
        work_zero: work_zero.map(comment_safe),
        file: file.map(|f| FileContext {
            number: f.number,
            count: f.count,
            tool: tool_text(&f.tool, config.decimals),
            operations: operations_text(&f.operations),
        }),
        tools: None,
        segments: vec![Segment {
            change: None,
            moves: compress_moves(&output, config.decimals)?,
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
    /// Indexes in `job.operations`.
    pub operations: Vec<usize>,
}

/// Render a job with several tools as one program with a tool change before each part but
/// the first (docs/specs/tool-per-operation.md §3.2). The post must support it.
pub fn render_with_tool_changes(
    parts: &[ToolPart],
    config: &PostConfig,
    work_zero: Option<&str>,
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
    let fmt = |v: f64| format_number(v, config.decimals);
    let mut segments = Vec::new();
    for (i, part) in parts.iter().enumerate() {
        let output = if config.arcs {
            merge_arcs(&part.program).map_err(|e| PostError::Render(e.to_string()))?
        } else {
            part.program.clone()
        };
        // Each part starts from fresh modal state: after a tool change every word is written
        // again, whatever the sender did to move the machine.
        let change = (i > 0).then(|| ChangeContext {
            comment: comment_safe(&format!(
                "Tool change {i} of {}: {}, operations {}",
                parts.len() - 1,
                tool_text(&part.tool, config.decimals),
                operations_text(&part.operations)
            )),
            tool_id: part.tool.id,
            rpm: format_number(part.program.rpm, 0),
        });
        segments.push(Segment {
            change,
            moves: compress_moves(&output, config.decimals)?,
        });
    }
    let context = Context {
        post_name: config.name.clone(),
        tool_id: first.program.tool_id,
        tool_diameter: fmt(first.program.tool_diameter),
        rpm: format_number(first.program.rpm, 0),
        spindle: match first.program.spindle {
            Spindle::Cw => "cw",
            Spindle::Ccw => "ccw",
        },
        safe_z: fmt(first.program.safe_z),
        dwell: (config.spindle_dwell_s > 0.0).then(|| fmt(config.spindle_dwell_s)),
        work_zero: work_zero.map(comment_safe),
        file: None,
        tools: Some(tool_list(parts, config.decimals)),
        segments,
    };
    fill_template(config, &context)
}

/// `T2 3.000 mm endmill "3 mm downcut"` for each tool once, in the order first used.
fn tool_list(parts: &[ToolPart], decimals: u32) -> Vec<String> {
    let mut seen: Vec<i64> = Vec::new();
    let mut lines = Vec::new();
    for part in parts {
        if seen.contains(&part.tool.id) {
            continue;
        }
        seen.push(part.tool.id);
        let text = tool_text(&part.tool, decimals);
        // "tool 2, 3.000 mm endmill" -> "T2 3.000 mm endmill"
        let rest = text
            .split_once(", ")
            .map_or(text.as_str(), |(_, rest)| rest);
        lines.push(comment_safe(&format!("T{} {rest}", part.tool.id)));
    }
    lines
}

fn fill_template(config: &PostConfig, context: &Context) -> Result<String, PostError> {
    let failed = |e: minijinja::Error| {
        PostError::Render(format!("Template {:?} failed: {e}", config.template))
    };
    let source = TEMPLATES
        .iter()
        .find(|(name, _)| *name == config.template)
        .map(|(_, source)| *source)
        .ok_or_else(|| PostError::Config(format!("Unknown template {:?}", config.template)))?;
    let mut env = Environment::new();
    // Jinja-style whitespace handling. minijinja has no access to anything but the context,
    // so templates are sandboxed by design.
    env.set_syntax(
        SyntaxConfig::builder()
            .trim_blocks(true)
            .lstrip_blocks(true)
            .keep_trailing_newline(true)
            .build()
            .map_err(failed)?,
    );
    env.set_undefined_behavior(UndefinedBehavior::Strict);
    env.set_auto_escape_callback(|_| AutoEscape::None);
    // The same output on every platform, whatever line endings the template was checked
    // out with (git may give a Windows build CRLF).
    let source = source.replace("\r\n", "\n");
    env.add_template(&config.template, &source)
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
        };
        let config = load_builtin_post("grbl").unwrap();
        let text = render_program_with(&program, &config, Some("x (y); z")).unwrap();
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
        };
        let config = load_builtin_post("grbl").unwrap();
        let text = render_program(&program, &config).unwrap();
        let expected = "(Chipload - GRBL)\n(Tool 1: 6.000 mm)\nG21 G90 G17 G94\nM3 S18000\n\
                        G4 P3.000\nG0 Z5.000\nG0 X1.000 Y2.000\nG1 Z-1.000 F300.000\n\
                        G1 X10.000 F1000.000\nG1 Y5.000\nG0 Z5.000\nM5\nM30\n";
        assert_eq!(text, expected);
    }
}
