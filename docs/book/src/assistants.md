# AI assistants (MCP)

An AI assistant such as Claude can work in Chipload for you. You describe the part ("cut the
outline out of 12 mm plywood with four tabs, and pocket the slots 4 mm deep"), and the
assistant makes the project, sets up the operations, checks them, plans and simulates them,
and writes the G-code when you ask.

It does this through `chipload mcp`, a server for the Model Context Protocol (MCP), which
assistants use to call programs' tools. (Design: `docs/specs/mcp.md`.)

## Setting it up

The assistant starts `chipload mcp` itself and talks to it over its input and output. Nothing
listens on the network.

- **Claude Code:** in your project's folder, run

      claude mcp add chipload -- chipload mcp

  Give the full path to `chipload` if it isn't on your `PATH`. Paths the assistant uses
  are relative to the folder Claude Code runs in.
- **Claude Desktop** and other MCP clients: add a server whose command is `chipload` with
  the argument `mcp`. In Claude Desktop's `claude_desktop_config.json`:

  ```json
  {
    "mcpServers": {
      "chipload": { "command": "/path/to/chipload", "args": ["mcp"] }
    }
  }
  ```

  On Windows, the command is the full path to `chipload-X.Y.Z-windows-x86_64.exe`. Ask
  the assistant to use full paths to your files there, as it has no project folder.

## What the assistant can do

Every [command](reference/cli.md) is a tool with the same options, apart from `mcp`,
`reference` and `render`. Each tool gives the same JSON as the command with
[`--json`](scripting.md#json-output). So the assistant:
- imports DXFs and pictures, and places them;
- reads and changes any value with `get` and `set` (an operation is added by setting a
  whole one);
- reads the project's problems with their paths, and fixes them;
- plans and simulates. With an output picture, `simulate` also shows the assistant the cut
  stock;
- writes G-code with `run`, which, as always, writes only programs that pass the
  [checker](concepts/checker.md).

Tools that only read (`inspect`, `plan`, `get`, `check-project`, the lists…) are marked so,
and your assistant may run them without asking you. For the others, it may ask first.

## Safety

The assistant can do what the `chipload` command can do, and no more.
- It can't move your machine. Chipload never does.
- `render`, which writes unchecked G-code for testing posts, isn't offered.
- A saved project keeps the previous file as `.bak`.

**Look at what it made before you cut:**
- open the project in the app;
- check the toolpath and the simulation;
- read the setup sheet;
- do an air cut.

An assistant can misread a drawing, or choose a side, a depth or a tool you didn't mean.

## Limits

- The assistant works on project files, not in the open app. Open (or reopen) the file to
  see its changes.
- It does one thing at a time; planning a big project takes a few seconds.
