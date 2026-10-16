# 0019: An MCP server built on the command line, without an MCP library

Status: accepted (2026-10-16). Spec: docs/specs/mcp.md.

## Context
The user wants AI assistants (Claude Code, Claude Desktop) to work in Chipload. The Model
Context Protocol (MCP) is how assistants call a program's tools. Everything Chipload does is
already a command with `--json` output (docs/specs/scripting.md).

## Decision
- `chipload mcp` serves MCP over **stdio** from `chipload-cli`. The client starts it; nothing
  listens on the network. It works on project files, not on the running GUI.
- **The tools are the commands**, generated from the clap definition: one tool per leaf
  command (`op tool` → `op_tool`), its options as the input schema, its help as the
  description. A call becomes the argument list, clap parses it, and the command runs
  in-process with `--json`. Its envelope is the result, the same as `chipload --json …`
  prints.
- **Left out:**
  - `mcp` itself;
  - `reference`, for development;
  - `render`, whose G-code isn't checked. G-code reaches an assistant only through `run`
    and the independent checker.
- **No MCP library.** The protocol is written in `src/mcp.rs`:
  - JSON-RPC 2.0, one message per line;
  - `initialize`, `ping`, `tools/list`, `tools/call`;
  - protocol versions 2024-11-05 to 2025-06-18.

  It needs only `serde_json` and `clap`, already dependencies. The official SDK (`rmcp`)
  would bring an async runtime for tools that are synchronous and run one at a time.
- `simulate` and `image preview` also return their PNG as image content (base64, written
  here too), so the assistant can see the result.

## Consequences
- Every new command or option is a tool at once, with no second interface to keep up.
- Protocol changes are ours to follow. The surface is four methods. A test drives a real
  `chipload mcp` process through stdin and stdout.
- `Out::into_json` returns the envelope instead of printing it. The server's stdout carries
  only protocol messages; a panic in one command becomes an error result and the server goes
  on.
- Seeing the assistant's edits in the app (the GUI reloading a changed file) is a separate
  step (spec §8).
