// SPDX-License-Identifier: GPL-3.0-or-later
//! `chipload mcp`, driven as an MCP client drives it: a child process, one JSON-RPC message
//! per line on stdin and stdout (docs/specs/mcp.md §7).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use serde_json::{Value, json};

fn corpus(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/corpus")
        .join(rel)
}

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("chipload-mcp-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// A running `chipload mcp` in `dir`, with `dir/settings` as the settings folder.
struct Session {
    child: Child,
    input: ChildStdin,
    output: BufReader<ChildStdout>,
    next_id: i64,
}

impl Session {
    fn start(dir: &Path) -> Session {
        let mut child = Command::new(env!("CARGO_BIN_EXE_chipload"))
            .arg("mcp")
            .env("CHIPLOAD_CONFIG_DIR", dir.join("settings"))
            .current_dir(dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let input = child.stdin.take().unwrap();
        let output = BufReader::new(child.stdout.take().unwrap());
        let mut session = Session {
            child,
            input,
            output,
            next_id: 0,
        };
        let init = session.request(
            "initialize",
            json!({"protocolVersion": "2025-06-18", "capabilities": {},
                   "clientInfo": {"name": "test", "version": "1"}}),
        );
        assert_eq!(init["result"]["protocolVersion"], "2025-06-18");
        session.send(&json!({"jsonrpc": "2.0", "method": "notifications/initialized"}));
        session
    }

    fn send(&mut self, message: &Value) {
        writeln!(self.input, "{message}").unwrap();
        self.input.flush().unwrap();
    }

    /// The next line the server writes: always one JSON-RPC message.
    fn receive(&mut self) -> Value {
        let mut line = String::new();
        self.output.read_line(&mut line).unwrap();
        let message: Value = serde_json::from_str(&line)
            .unwrap_or_else(|err| panic!("not JSON-RPC: {line:?} ({err})"));
        assert_eq!(message["jsonrpc"], "2.0", "{message}");
        message
    }

    fn request(&mut self, method: &str, params: Value) -> Value {
        self.next_id += 1;
        let id = self.next_id;
        self.send(&json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}));
        let reply = self.receive();
        assert_eq!(reply["id"], json!(id), "{reply}");
        reply
    }

    /// A tool's result (which may be marked isError).
    fn call(&mut self, name: &str, arguments: Value) -> Value {
        let reply = self.request("tools/call", json!({"name": name, "arguments": arguments}));
        assert!(reply.get("error").is_none(), "{name}: {reply}");
        reply["result"].clone()
    }

    /// A tool's result that must have succeeded: its JSON envelope.
    fn ok(&mut self, name: &str, arguments: Value) -> Value {
        let result = self.call(name, arguments);
        assert_eq!(result["isError"], json!(false), "{name}: {result}");
        let envelope = result["structuredContent"].clone();
        let text: Value =
            serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(text, envelope, "the text is the same JSON");
        envelope
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// `chipload --json ARGS` in `dir`, its one JSON object.
fn cli_json(dir: &Path, args: &[&str]) -> Value {
    let out = Command::new(env!("CARGO_BIN_EXE_chipload"))
        .env("CHIPLOAD_CONFIG_DIR", dir.join("settings"))
        .current_dir(dir)
        .arg("--json")
        .args(args)
        .output()
        .unwrap();
    serde_json::from_slice(&out.stdout).unwrap()
}

#[test]
fn the_handshake_the_tool_list_and_protocol_errors() {
    let dir = temp_dir("protocol");
    let mut s = Session::start(&dir);
    // The notification got no answer: the next line answers the ping.
    assert_eq!(s.request("ping", json!({}))["result"], json!({}));

    let tools = s.request("tools/list", json!({}))["result"]["tools"].clone();
    let names: Vec<&str> = tools
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    for name in [
        "plan",
        "run",
        "simulate",
        "set",
        "get",
        "check_project",
        "import",
    ] {
        assert!(names.contains(&name), "{name}");
    }
    for name in ["mcp", "reference", "render"] {
        assert!(!names.contains(&name), "{name} is left out");
    }
    let plan = tools
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["name"] == "plan")
        .unwrap();
    assert_eq!(plan["annotations"]["readOnlyHint"], json!(true));
    assert_eq!(plan["inputSchema"]["required"], json!(["project"]));
    assert_eq!(
        plan["inputSchema"]["properties"]["sets"]["type"],
        json!("array")
    );

    // An unknown method, an unknown tool, a line that isn't JSON.
    assert_eq!(
        s.request("resources/list", json!({}))["error"]["code"],
        json!(-32601)
    );
    assert_eq!(
        s.request("tools/call", json!({"name": "nothing"}))["error"]["code"],
        json!(-32602)
    );
    writeln!(s.input, "not json").unwrap();
    s.input.flush().unwrap();
    assert_eq!(s.receive()["error"]["code"], json!(-32700));

    // An unknown protocol version gets the newest.
    let reply = s.request("initialize", json!({"protocolVersion": "1999-01-01"}));
    assert_eq!(reply["result"]["protocolVersion"], json!("2025-06-18"));
    assert_eq!(reply["result"]["serverInfo"]["name"], json!("chipload"));
}

#[test]
fn a_tool_gives_what_the_command_prints_with_json() {
    let dir = temp_dir("same");
    std::fs::copy(
        corpus("jobs/v18/dogbones.chipload"),
        dir.join("dogbones.chipload"),
    )
    .unwrap();
    let mut s = Session::start(&dir);
    for (tool, args) in [
        ("plan", vec!["plan", "dogbones.chipload"]),
        ("check_project", vec!["check-project", "dogbones.chipload"]),
        (
            "get",
            vec!["get", "dogbones.chipload", "operations.0.end_depth"],
        ),
    ] {
        let arguments = match tool {
            "get" => json!({"project": "dogbones.chipload", "path": "operations.0.end_depth"}),
            _ => json!({"project": "dogbones.chipload"}),
        };
        assert_eq!(s.ok(tool, arguments), cli_json(&dir, &args), "{tool}");
    }
}

#[test]
fn failures_are_results_and_the_server_goes_on() {
    let dir = temp_dir("failures");
    let mut s = Session::start(&dir);
    // clap refuses: a required input is missing.
    let missing = s.call("plan", json!({}));
    assert_eq!(missing["isError"], json!(true));
    assert!(
        missing["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("<PROJECT>"),
        "{missing}"
    );
    // An input the tool doesn't have.
    let unknown = s.call("plan", json!({"projekt": "a.chipload"}));
    assert_eq!(unknown["isError"], json!(true));
    // The command fails: its envelope says why.
    let failed = s.call("plan", json!({"project": "missing.chipload"}));
    assert_eq!(failed["isError"], json!(true));
    assert_eq!(failed["structuredContent"]["ok"], json!(false));
    assert!(failed["structuredContent"]["error"]["message"].is_string());
    // Still serving.
    assert_eq!(s.request("ping", json!({}))["result"], json!({}));
}

#[test]
fn a_part_from_a_dxf_to_checked_g_code_and_a_picture() {
    let dir = temp_dir("end-to-end");
    std::fs::copy(corpus("dxf/simple_part.dxf"), dir.join("part.dxf")).unwrap();
    let mut s = Session::start(&dir);
    s.ok(
        "import",
        json!({"dxf": "part.dxf", "output": "part.chipload"}),
    );
    let profile = json!({
        "type": "profile", "name": "Outline", "part": 1,
        "select": {"layer": "CUT_OUTSIDE"}, "side": "outside", "direction": "climb",
        "tool": 1, "start_depth": 0, "end_depth": 12.3, "clearance": 0, "step_down": 3,
        "feed": 1500, "plunge_feed": 400, "ramp_angle": 0.0524,
        "tabs": {"count": 4, "width": 6, "height": 2, "flank_angle": std::f64::consts::FRAC_PI_2}
    });
    s.ok(
        "set",
        json!({"project": "part.chipload",
               "values": ["stock.thickness=12", format!("operations.0={profile}")]}),
    );
    let problems = s.ok("check_project", json!({"project": "part.chipload"}));
    assert_eq!(problems["ok"], json!(true), "{problems}");
    s.ok(
        "run",
        json!({"project": "part.chipload", "output": "mcp.nc", "no_sheet": true}),
    );
    // The same program as the command line writes.
    cli_json(
        &dir,
        &["run", "part.chipload", "-o", "cli.nc", "--no-sheet"],
    );
    let mcp = std::fs::read_to_string(dir.join("mcp.nc")).unwrap();
    assert!(mcp.contains("G1"), "{mcp}");
    assert_eq!(mcp, std::fs::read_to_string(dir.join("cli.nc")).unwrap());

    // The simulation's picture comes back as image content.
    let result = s.call(
        "simulate",
        json!({"project": "part.chipload", "output": "sim.png"}),
    );
    assert_eq!(result["isError"], json!(false), "{result}");
    let image = result["content"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["type"] == "image")
        .expect("an image");
    assert_eq!(image["mimeType"], json!("image/png"));
    // The PNG signature, base64-encoded.
    assert!(image["data"].as_str().unwrap().starts_with("iVBORw0KGgo"));
}
