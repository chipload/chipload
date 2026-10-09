// SPDX-License-Identifier: GPL-3.0-or-later
//! Post-processors: render a Program to G-code text through a template.
//!
//! Port of `core/post/render.py` and `core/job/post_config.py`. Only generic work happens
//! here (number formatting and modal compression); anything machine specific lives in the
//! template. The built-in configs and templates are the Python package's own files, compiled
//! in, so both implementations render from the same source.

use std::path::Path;

use chipload_job::Spindle;
use chipload_toolpath::model::{MoveKind, Program};
use minijinja::syntax::SyntaxConfig;
use minijinja::value::Serde;
use minijinja::{AutoEscape, Environment, UndefinedBehavior, Value};
use serde::{Deserialize, Serialize};

/// Post-processor config format version (a persisted format, see CLAUDE.md).
pub const CURRENT_VERSION: i64 = 1;

/// Built-in posts: (name, config TOML).
const BUILTIN_POSTS: [(&str, &str); 1] = [(
    "grbl",
    include_str!("../../../src/chipload/core/post/posts/grbl.toml"),
)];

/// Templates a config may name: (file name, source).
const TEMPLATES: [(&str, &str); 1] = [(
    "grbl.nc.j2",
    include_str!("../../../src/chipload/core/post/templates/grbl.nc.j2"),
)];

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
    // TOML integers only; a bool, float or string is refused, as in Python.
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
    pub f: Option<String>,
}

/// Omit axis and feed words that have not changed; drop moves that go nowhere.
pub fn compress_moves(program: &Program, decimals: u32) -> Result<Vec<PostMove>, PostError> {
    let mut last: [Option<String>; 3] = [None, None, None];
    let mut last_f: Option<String> = None;
    let mut result = Vec::new();
    for m in &program.moves {
        let mut words: [Option<String>; 3] = [None, None, None];
        for (axis, value) in [m.x, m.y, m.z].into_iter().enumerate() {
            let text = value.map(|v| format_number(v, decimals));
            if text.is_some() && text != last[axis] {
                words[axis] = text;
            }
        }
        if words.iter().all(Option::is_none) {
            continue;
        }
        let mut feed = None;
        if m.kind == MoveKind::Feed {
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
            code: if m.kind == MoveKind::Rapid {
                "G0"
            } else {
                "G1"
            },
            x,
            y,
            z,
            f: feed,
        });
    }
    Ok(result)
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
    moves: Vec<PostMove>,
}

pub fn render_program(program: &Program, config: &PostConfig) -> Result<String, PostError> {
    let fmt = |v: f64| format_number(v, config.decimals);
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
        moves: compress_moves(program, config.decimals)?,
    };
    let failed = |e: minijinja::Error| {
        PostError::Render(format!("Template {:?} failed: {e}", config.template))
    };
    let source = TEMPLATES
        .iter()
        .find(|(name, _)| *name == config.template)
        .map(|(_, source)| *source)
        .ok_or_else(|| PostError::Config(format!("Unknown template {:?}", config.template)))?;
    let mut env = Environment::new();
    // The same settings as the Python SandboxedEnvironment. minijinja has no access to
    // anything but the context, so templates are sandboxed by design.
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
    env.add_template(&config.template, source).map_err(failed)?;
    let text = env
        .get_template(&config.template)
        .and_then(|t| t.render(Value::from(Serde(&context))))
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
        assert_eq!(format_number(0.125, 2), "0.12"); // ties to even, as Python
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
