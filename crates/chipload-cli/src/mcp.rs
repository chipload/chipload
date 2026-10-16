// SPDX-License-Identifier: GPL-3.0-or-later
//! The MCP server (docs/specs/mcp.md): the commands as tools for an AI assistant, over the
//! Model Context Protocol's stdio transport. Each tool is a command, described and parsed by
//! the same clap definition and run in-process with `--json`; its result is the command's
//! JSON envelope.

use std::any::TypeId;
use std::io::{self, BufRead, Write};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::process::ExitCode;

use clap::{Arg, ArgAction, CommandFactory, Parser};
use serde_json::{Map, Value, json};

use crate::out::{EXIT_FAILED, Out};
use crate::{Cli, execute};

/// The protocol versions this server speaks, newest first.
const PROTOCOLS: [&str; 3] = ["2025-06-18", "2025-03-26", "2024-11-05"];

/// Commands that are not tools: the server itself, the guide's generated pages, and `render`,
/// whose G-code is not checked (§3: G-code only through `run` and its checker).
const LEFT_OUT: [&str; 3] = ["mcp", "reference", "render"];

/// Tools that only read: the client may run them without asking.
const READ_ONLY: [&str; 16] = [
    "inspect",
    "plan",
    "get",
    "check",
    "check_project",
    "schema",
    "licenses",
    "image_list",
    "dxf_list",
    "tool_list",
    "tool_compare",
    "machine_list",
    "machine_compare",
    "post_list",
    "post_show",
    "part_list",
];

/// Tools whose PNG is also returned as an image, so the assistant can look at it.
const PICTURES: [&str; 2] = ["simulate", "image_preview"];

/// How to work with Chipload, sent once at the start.
const INSTRUCTIONS: &str = "Chipload is 2.5D CAM: DXF drawings in, checked G-code out, for CNC \
routers and mills. Every tool is a `chipload` command and returns its JSON result. Lengths are \
millimetres. Work on project files (.chipload) by path: `import` or `new` makes one, and \
`inspect` lists its contours with their ids and layers. Any value can be read and changed with \
`get` and `set` by path, counting from 0 (the format is `schema` with format \"project\"); an \
operation is added by setting a whole one, e.g. `operations.0={\"type\": \"profile\", …}` \
with its part, selection, side, tool and depths. After changes, `check_project` lists the problems with their paths; `plan` gives \
the toolpath's time and notes, and `simulate` with an output PNG shows the cut stock. Use `run` \
only when the user asks for G-code: it writes only programs the independent checker passes. \
Never guess units, cut sides, depths or tools: ask the user. Chipload never moves a machine; \
remind the user to check the program and do an air cut.";

/// One tool: its command's path and how its inputs become arguments.
struct Tool {
    name: String,
    /// The subcommand names, e.g. ["op", "add"].
    path: Vec<String>,
    inputs: Vec<Input>,
    /// The tool as `tools/list` describes it.
    description: Value,
}

/// One argument of a command, as a tool's input.
struct Input {
    name: String,
    long: Option<String>,
    short: Option<char>,
    /// Positional: its place among the positionals.
    index: Option<usize>,
    /// A switch: given or not.
    flag: bool,
    /// Several values (a repeated option, or a positional that takes more than one).
    many: bool,
}

/// Serve on this process's stdin and stdout until stdin closes.
pub fn serve_stdio() -> ExitCode {
    let stdin = io::stdin();
    let stdout = io::stdout();
    match serve(stdin.lock(), stdout.lock()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("error: {err}");
            ExitCode::from(EXIT_FAILED)
        }
    }
}

/// Answer each message (one JSON-RPC message per line) until the input ends. Nothing but
/// the answers is written to `output`.
pub fn serve(input: impl BufRead, mut output: impl Write) -> io::Result<()> {
    let tools = tools();
    for line in input.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        if let Some(reply) = handle(&tools, &line) {
            writeln!(output, "{reply}")?;
            output.flush()?;
        }
    }
    Ok(())
}

/// The answer to one message; None for a notification (or a response from the client).
fn handle(tools: &[Tool], line: &str) -> Option<Value> {
    let message: Value = match serde_json::from_str(line) {
        Ok(message) => message,
        Err(err) => return Some(error(Value::Null, -32700, &format!("parse error: {err}"))),
    };
    let Value::Object(message) = message else {
        return Some(error(
            Value::Null,
            -32600,
            "a request must be a JSON object",
        ));
    };
    let id = message.get("id").cloned()?;
    let params = message.get("params").cloned().unwrap_or(json!({}));
    let reply = match message.get("method").and_then(Value::as_str) {
        Some("initialize") => Ok(initialize(&params)),
        Some("ping") => Ok(json!({})),
        Some("tools/list") => Ok(json!({
            "tools": tools.iter().map(|t| t.description.clone()).collect::<Vec<_>>()
        })),
        Some("tools/call") => call(tools, &params),
        Some(method) => return Some(error(id, -32601, &format!("no method {method}"))),
        None => return None, // a response to a request of ours: there are none
    };
    Some(match reply {
        Ok(result) => json!({"jsonrpc": "2.0", "id": id, "result": result}),
        Err(message) => error(id, -32602, &message),
    })
}

fn error(id: Value, code: i64, message: &str) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}})
}

/// The handshake: the client's protocol version if known, else the newest.
fn initialize(params: &Value) -> Value {
    let asked = params.get("protocolVersion").and_then(Value::as_str);
    let version = asked
        .filter(|v| PROTOCOLS.contains(v))
        .unwrap_or(PROTOCOLS[0]);
    json!({
        "protocolVersion": version,
        "capabilities": {"tools": {"listChanged": false}},
        "serverInfo": {"name": "chipload", "version": env!("CARGO_PKG_VERSION")},
        "instructions": INSTRUCTIONS,
    })
}

// -- the tools

/// Every command that is a tool, from the CLI's own definition.
fn tools() -> Vec<Tool> {
    let mut root = Cli::command();
    root.build();
    let mut tools = Vec::new();
    for sub in visible(&root) {
        if !LEFT_OUT.contains(&sub.get_name()) {
            collect(&mut tools, &[], sub);
        }
    }
    tools
}

/// The subcommands that are commands: not hidden, and not clap's own `help`.
fn visible(cmd: &clap::Command) -> impl Iterator<Item = &clap::Command> {
    cmd.get_subcommands()
        .filter(|c| !c.is_hide_set() && c.get_name() != "help")
}

/// The leaves under `cmd` (`op add`, …), or `cmd` itself.
fn collect(tools: &mut Vec<Tool>, parents: &[String], cmd: &clap::Command) {
    let mut path = parents.to_vec();
    path.push(cmd.get_name().to_string());
    let subs: Vec<&clap::Command> = visible(cmd).collect();
    if !subs.is_empty() {
        for sub in subs {
            collect(tools, &path, sub);
        }
        return;
    }
    let name = path.join("_").replace('-', "_");
    let args: Vec<&Arg> = cmd
        .get_arguments()
        .filter(|a| !a.is_hide_set() && !a.is_global_set())
        .filter(|a| !matches!(a.get_id().as_str(), "help" | "version"))
        .collect();
    let mut properties = Map::new();
    let mut required = Vec::new();
    let mut inputs = Vec::new();
    for arg in args {
        let input = Input {
            name: arg.get_id().as_str().to_string(),
            long: arg.get_long().map(str::to_string),
            short: arg.get_short(),
            index: arg.get_index(),
            flag: matches!(arg.get_action(), ArgAction::SetTrue | ArgAction::SetFalse),
            many: matches!(arg.get_action(), ArgAction::Append)
                || arg.get_num_args().is_some_and(|n| n.max_values() > 1),
        };
        if arg.is_required_set() {
            required.push(json!(input.name));
        }
        properties.insert(input.name.clone(), input_schema(arg, &input));
        inputs.push(input);
    }
    let about = cmd
        .get_long_about()
        .or(cmd.get_about())
        .map(|a| a.to_string())
        .unwrap_or_default();
    let read_only = READ_ONLY.contains(&name.as_str());
    let description = json!({
        "name": name,
        "title": format!("chipload {}", path.join(" ")),
        "description": about.trim(),
        "inputSchema": {
            "type": "object",
            "properties": properties,
            "required": required,
            "additionalProperties": false,
        },
        "annotations": {"readOnlyHint": read_only, "openWorldHint": false},
    });
    tools.push(Tool {
        name,
        path,
        inputs,
        description,
    });
}

/// An input's JSON Schema: its type from clap's parser, its help and default.
fn input_schema(arg: &Arg, input: &Input) -> Value {
    let parser = arg.get_value_parser().type_id();
    let choices: Vec<String> = arg
        .get_possible_values()
        .iter()
        .filter(|v| !v.is_hide_set())
        .map(|v| v.get_name().to_string())
        .collect();
    let mut schema = if input.flag || parser == TypeId::of::<bool>() {
        json!({"type": "boolean"})
    } else if !choices.is_empty() {
        json!({"type": "string", "enum": choices})
    } else if parser == TypeId::of::<f64>() || parser == TypeId::of::<f32>() {
        json!({"type": "number"})
    } else if [
        TypeId::of::<u64>(),
        TypeId::of::<u32>(),
        TypeId::of::<usize>(),
        TypeId::of::<i64>(),
        TypeId::of::<i32>(),
    ]
    .iter()
    .any(|t| parser == *t)
    {
        json!({"type": "integer"})
    } else {
        json!({"type": "string"})
    };
    if input.many {
        schema = json!({"type": "array", "items": schema});
    }
    let mut text = arg
        .get_long_help()
        .or(arg.get_help())
        .map(|h| h.to_string())
        .unwrap_or_default();
    let defaults: Vec<String> = arg
        .get_default_values()
        .iter()
        .map(|v| v.to_string_lossy().into_owned())
        .collect();
    if !defaults.is_empty() && !input.flag {
        text = format!("{} (default: {})", text.trim(), defaults.join(", "));
    }
    if let Value::Object(map) = &mut schema
        && !text.trim().is_empty()
    {
        map.insert("description".into(), json!(text.trim()));
    }
    schema
}

/// `tools/call`: an unknown tool is a protocol error; anything else is the tool's result,
/// marked `isError` when the command or its arguments failed.
fn call(tools: &[Tool], params: &Value) -> Result<Value, String> {
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .ok_or("tools/call needs a tool name")?;
    let tool = tools
        .iter()
        .find(|t| t.name == name)
        .ok_or_else(|| format!("no tool {name}"))?;
    let empty = Map::new();
    let arguments = match params.get("arguments") {
        None | Some(Value::Null) => &empty,
        Some(Value::Object(map)) => map,
        Some(_) => return Err("a tool's arguments must be a JSON object".into()),
    };
    let argv = match command_line(tool, arguments) {
        Ok(argv) => argv,
        Err(message) => return Ok(failed(&message)),
    };
    let cli = match Cli::try_parse_from(&argv) {
        Ok(cli) => cli,
        Err(err) => return Ok(failed(err.to_string().trim())),
    };
    let mut out = Out::new(true);
    let code = match catch_unwind(AssertUnwindSafe(|| execute(&mut out, cli.command))) {
        Ok(code) => code,
        Err(_) => {
            // The server outlives a bug in one command (§4).
            out.err("error: the command stopped unexpectedly; please report it with the files");
            ExitCode::from(EXIT_FAILED)
        }
    };
    let ok = code == ExitCode::SUCCESS;
    let result = out.into_json(code);
    let mut content = vec![json!({"type": "text", "text": result.to_string()})];
    if ok && PICTURES.contains(&tool.name.as_str()) {
        content.extend(pictures(&result));
    }
    Ok(json!({
        "content": content,
        "structuredContent": result,
        "isError": !ok,
    }))
}

/// A result for a call that didn't run: the reason as its text.
fn failed(message: &str) -> Value {
    json!({"content": [{"type": "text", "text": message}], "isError": true})
}

/// The arguments a command line would have: `chipload SUB… --json OPTIONS -- POSITIONALS`.
/// Values go as `--long=VALUE`, so a negative number is never taken for an option.
fn command_line(tool: &Tool, arguments: &Map<String, Value>) -> Result<Vec<String>, String> {
    let mut argv = vec!["chipload".to_string()];
    argv.extend(tool.path.iter().cloned());
    argv.push("--json".into());
    let mut positionals: Vec<(usize, Vec<String>)> = Vec::new();
    for (key, value) in arguments {
        let Some(input) = tool.inputs.iter().find(|i| i.name == *key) else {
            let known: Vec<&str> = tool.inputs.iter().map(|i| i.name.as_str()).collect();
            return Err(format!(
                "{} has no input \"{key}\"; its inputs are: {}",
                tool.name,
                known.join(", ")
            ));
        };
        if value.is_null() {
            continue;
        }
        if input.flag {
            match value {
                Value::Bool(true) => argv.push(option(input)?),
                Value::Bool(false) => {}
                _ => return Err(format!("\"{key}\" is true or false")),
            }
            continue;
        }
        let values = match value {
            Value::Array(items) if input.many => items
                .iter()
                .map(|v| scalar(key, v))
                .collect::<Result<Vec<_>, _>>()?,
            other => vec![scalar(key, other)?],
        };
        match input.index {
            Some(index) => positionals.push((index, values)),
            None => {
                let flag = option(input)?;
                for v in values {
                    argv.push(format!("{flag}={v}"));
                }
            }
        }
    }
    if !positionals.is_empty() {
        positionals.sort_by_key(|(index, _)| *index);
        argv.push("--".into());
        argv.extend(positionals.into_iter().flat_map(|(_, values)| values));
    }
    Ok(argv)
}

/// An option's name on the command line.
fn option(input: &Input) -> Result<String, String> {
    match (&input.long, input.short) {
        (Some(long), _) => Ok(format!("--{long}")),
        (None, Some(short)) => Ok(format!("-{short}")),
        (None, None) => Err(format!("\"{}\" can't be given", input.name)),
    }
}

/// One value as the command line spells it.
fn scalar(key: &str, value: &Value) -> Result<String, String> {
    match value {
        Value::String(text) => Ok(text.clone()),
        Value::Number(number) => Ok(number.to_string()),
        Value::Bool(b) => Ok(b.to_string()),
        _ => Err(format!("\"{key}\" takes a string, a number or true/false")),
    }
}

/// The PNG files a command wrote, as image content.
fn pictures(result: &Value) -> Vec<Value> {
    let written = result
        .get("written")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    written
        .iter()
        .filter_map(Value::as_str)
        .filter(|path| path.to_ascii_lowercase().ends_with(".png"))
        .filter_map(|path| std::fs::read(path).ok())
        .map(|bytes| json!({"type": "image", "data": base64(&bytes), "mimeType": "image/png"}))
        .collect()
}

/// Standard base64 with padding (RFC 4648 §4), for image content.
fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut text = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b = [
            chunk[0],
            chunk.get(1).copied().unwrap_or(0),
            chunk.get(2).copied().unwrap_or(0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        let symbols = chunk.len() + 1;
        for i in 0..4 {
            if i < symbols {
                let index = (n >> (18 - 6 * i)) & 0x3f;
                text.push(char::from(ALPHABET[index as usize]));
            } else {
                text.push('=');
            }
        }
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_matches_the_rfc_examples() {
        // RFC 4648 §10.
        let cases = [
            ("", ""),
            ("f", "Zg=="),
            ("fo", "Zm8="),
            ("foo", "Zm9v"),
            ("foob", "Zm9vYg=="),
            ("fooba", "Zm9vYmE="),
            ("foobar", "Zm9vYmFy"),
        ];
        for (plain, coded) in cases {
            assert_eq!(base64(plain.as_bytes()), coded);
        }
    }

    #[test]
    fn every_command_but_the_left_out_is_a_tool_with_a_schema() {
        let tools = tools();
        let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        for name in [
            "plan",
            "run",
            "check_project",
            "op_tool",
            "shape_set",
            "tool_list",
        ] {
            assert!(names.contains(&name), "{name} in {names:?}");
        }
        for name in LEFT_OUT {
            assert!(!names.contains(&name), "{name}");
        }
        for name in READ_ONLY.iter().chain(PICTURES.iter()) {
            assert!(names.contains(name), "{name} is a tool");
        }
        for tool in &tools {
            let schema = &tool.description["inputSchema"];
            assert_eq!(schema["type"], "object", "{}", tool.name);
            let properties = schema["properties"].as_object().unwrap();
            for required in schema["required"].as_array().unwrap() {
                assert!(properties.contains_key(required.as_str().unwrap()));
            }
            assert!(!tool.description["description"].as_str().unwrap().is_empty());
        }
    }

    #[test]
    fn a_call_becomes_the_command_line() {
        let tools = tools();
        let plan = tools.iter().find(|t| t.name == "plan").unwrap();
        let arguments = json!({"project": "a b.chipload", "sets": ["operations.0.end_depth=-1"]});
        let argv = command_line(plan, arguments.as_object().unwrap()).unwrap();
        assert_eq!(
            argv,
            [
                "chipload",
                "plan",
                "--json",
                "--set=operations.0.end_depth=-1",
                "--",
                "a b.chipload"
            ]
        );
        assert!(Cli::try_parse_from(&argv).is_ok());
        let wrong = json!({"projekt": "a.chipload"});
        let err = command_line(plan, wrong.as_object().unwrap()).unwrap_err();
        assert!(err.contains("projekt") && err.contains("project"), "{err}");
    }
}
