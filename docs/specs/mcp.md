# Spec: an MCP server (`chipload mcp`)

Status: **approved 2026-10-16** ("I agree, go ahead"), all decisions as recommended (§9).
Built 2026-10-16 (ADR 0019). As built:
- §3: there is no `op add` command: an operation is added with `set operations.N={…}`. The
  server's instructions say so, and point at `inspect` for the contours and layers.
- §3: number options (clap `f64`, integers) get `number`/`integer` in the input schema,
  `bool` values `boolean`; a number or `true`/`false` is accepted for any value and passed as
  text.
- §3: the result also has `structuredContent`: the same envelope as the text.
- §3: `render` is left out too (unchecked G-code), as decided in §9.5.

Guide: docs/book/src/assistants.md.

User request (2026-10-16): "Can we add MCP interface to the app now".

## 1. What it is for

The Model Context Protocol (MCP) lets an AI assistant (Claude Code, Claude Desktop and
other MCP clients) call a program's tools. With it, an assistant can do in Chipload what
a script does today with `--json`. For example:
- make a project from a DXF;
- add and set up operations;
- plan, simulate and look at the result;
- read the problems and fix them;
- write the checked G-code.

The user describes the part ("a 6 mm pocket 4 deep inside the inner rectangle, then cut it
out with tabs"); the assistant does the steps and shows what it did.

Chipload never drives a machine, and this doesn't change that: the result is still a
project file and checked G-code that the user looks at and runs.

## 2. Where it lives

- **A command, `chipload mcp`,** in `chipload-cli`. It speaks MCP over **stdio**: the
  client starts it as a child process and talks over stdin and stdout. There is no network
  port and nothing listens, which fits "no network calls" and keeps it local to the user's
  own permissions.
- **Setting it up,** e.g. for Claude Code:

      claude mcp add chipload -- chipload mcp

  For Claude Desktop, the same command goes in its config file. The guide says how.
- **Headless, on files.** Each tool works on project files by path, as the CLI does. It
  doesn't control a running GUI. Seeing the assistant's work in the app is step 2 (§8).
- **Architecture:** the CLI is a top layer (rule 2), so the server can call everything the
  commands call. No library crate changes, and no rule in `architecture.rs` changes.

## 3. The tools: the CLI's commands

- **One tool per command** (and per subcommand: `op_add`, `shape_set`, `tool_list`, …),
  with the command's name, its help text as the description and its options as the input.
  For example, `plan` takes `project` (a path), `set` (a list of `PATH=VALUE`) and `part`.
- **Generated from the CLI's own definition** (clap), not written by hand. A new command or
  option is a new tool or input at once, and the help texts are the same. Types follow clap:
  - a flag becomes a boolean;
  - a choice (a value enum) becomes a string with its allowed values;
  - an option that repeats becomes a list;
  - anything else becomes a string, which clap parses exactly as on the command line.
- **The same code runs.** A call becomes the argument list the CLI would get, is parsed by
  clap with the same checks, and runs the command in-process with `--json`. The tool's
  result is the command's JSON envelope (docs/specs/scripting.md §3). So `plan` through MCP
  gives byte for byte what `chipload --json plan` prints.
- **Failure:**
  - A command that fails (exit code 1) gives a result marked `isError`, with its envelope,
    whose `error` and `problems` say why. The assistant can read and fix them.
  - A call clap refuses (a missing or unknown option) gives `isError` with clap's message.
- **Left out:**
  - `mcp` itself;
  - `reference`, which writes the guide's generated pages and is for development;
  - `render`, which writes G-code that is **not checked** (for testing posts). An
    assistant gets G-code only from `run`, through the checker.
- **Pictures:** `simulate` and `image preview` also return their PNG as MCP image content,
  so the assistant can see the shaded stock or the black and white picture. The file is still written where asked.
- **Hints for the client:**
  - Read-only commands are marked `readOnlyHint`. These are `inspect`, `plan`, `get`,
    `check`, `check-project`, `schema`, `tool list`, `post show` and the like.
  - The others write files, and clients may ask the user before running them.
- **Server instructions** (sent at start-up) say, in a few lines, how to work:
  1. Use `schema project` and `get`/`set` for any value.
  2. Run `check-project` after changes, then `plan` or `simulate` to look.
  3. Run `run` only when the user asks for G-code.
  4. Never invent units, sides or depths; ask the user.

## 4. Protocol

- **JSON-RPC 2.0 over stdio,** one message per line, as MCP's stdio transport specifies.
- **Methods:**
  - `initialize`: the server answers with its name, version, the tools capability and the
    instructions.
  - `notifications/initialized`;
  - `ping`;
  - `tools/list`;
  - `tools/call`.
  - Anything else gets the standard "method not found" error.
- **The protocol version** is the one the client asks for, if the server knows it;
  otherwise the newest the server knows.
- **One call at a time,** in order. Planning a big project takes seconds, which is fine
  for an assistant. Cancelling (`notifications/cancelled`) is ignored at first.
- **stdout is the protocol only.** Nothing else may print to it. `Out` in JSON mode only
  collects, so `Out::finish` gets a sibling that returns the envelope instead of printing
  it. Any log goes to stderr.
- **A panic in a command** becomes an `isError` result and the server keeps running.
  Library code doesn't panic, but a server must outlive one bad call.

## 5. Files and safety

- **Paths:** relative paths are relative to the server's working directory. That is where
  the client started it (for Claude Code, the project folder).
- **Writes:** the same as the CLI:
  - `save_job` keeps the previous file as `.bak`;
  - `run` writes G-code only if the independent checker passes it, so `run_job` refuses
    anything else.
- **No new powers:** the assistant can do exactly what the user can do with the
  `chipload` command, and nothing else. There is no shell, no file reading beyond what the
  commands read, and no network.

## 6. Dependencies

None new. The protocol is small (four methods, JSON in and out), and `serde_json` and
`clap` are already in the CLI.

The official Rust SDK (`rmcp`) would bring an async runtime (tokio) for a server whose
tools are synchronous and run one at a time (decision 3).

## 7. Tests

- **The protocol, in-process** (lines in, lines out):
  - `initialize` answers with the tools capability;
  - `tools/list` names every command except those left out, each with a valid JSON Schema;
  - an unknown method gives "method not found".
- **The same results:** `plan` and `check-project` through `tools/call` give the same JSON
  as `chipload --json …` on corpus projects.
- **Failures:**
  - a missing required option gives an `isError` result, and the server keeps going;
  - so does a failing command (a project with problems).
- **Pictures:** `simulate` returns image content that decodes as a PNG.
- **Nothing else on stdout:** every line the server writes parses as JSON-RPC.
- **End to end:** a `chipload mcp` child process, driven through stdin and stdout, makes a
  project from a corpus DXF, adds a profile, plans and runs it. The G-code matches the CLI's.

## 8. Step 2 (separate, after this): see it in the app

The GUI reloads the open project when its file changes on disk, so the user watches the
assistant's edits.
- If the app has no unsaved changes, it reloads quietly, with undo history kept as one
  step.
- If it does, the app asks.
- Polled once a second (the file's time and size), with no new dependency.

This gets its own short spec once step 1 is in.

## 9. Decisions

1. **A headless `chipload mcp` over stdio, working on project files.** **Recommended**:
   - local only;
   - built on the CLI as rule 5 asks;
   - the GUI isn't involved.

   Alternative: a server inside the GUI (HTTP on localhost) that edits the open project
   live. That means a listening port, threads into the app's state, and the app must be
   running.
2. **The tools are generated from the CLI's commands, and run the same code.**
   **Recommended**: every feature comes free and never drifts.

   Alternative: a hand-picked, smaller set of tools with their own inputs, which reads a
   little better for the assistant but is a second interface to keep up.
3. **No MCP library; the protocol written here (about 300 lines).** **Recommended**: no
   async runtime, and nothing new to license.

   Alternative: `rmcp`, the official SDK, which follows protocol changes for us.
4. **`simulate` and `image preview` return their picture as image content.** **Recommended**: the
   assistant can see what it did.

   Alternative: paths only.
5. **Left out: `mcp`, `reference` and `render` (unchecked G-code).** **Recommended**.

   Alternative: also leave out `run` so the assistant can't write G-code. It would still
   be checked; the user decides whether to cut it.
6. **Step 2, the GUI reloading a changed file, comes next as its own spec.**
   **Recommended**.

   Alternative: not at all, or in the same step.
