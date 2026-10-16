# Chipload

2.5D CAM for CNC routers and mills: drawings in (DXF), checked G-code out. Free software,
for Linux and Windows, written in Rust.

![Chipload: a plaque with a pocket round an island, bored holes, CHIPLOAD V-carved in a
built-in font and an outline held by tabs, its simulated stock in 3D](docs/book/src/images/showcase.png)

## Highlights

- **Checked G-code, always.** Every program goes through an independent checker before it
  can be written. The checker looks at:
  - the machine's limits;
  - rapids through the stock;
  - retracts before travel;
  - arcs that don't close;
  - feeds of zero.
- **The operations you cut with:**
  - **profile:** a spiral down the contour, holding tabs, a finishing pass, an onion skin,
    dogbone and T-bone corners;
  - **pocket:** islands, rings that spiral with rounded corners, one island at a time,
    rest machining after a larger tool, helical entry;
  - **V-carve;**
  - **follow path;**
  - **holes:** drilling, pecking and helical boring.
- **See it before you cut:**
  - a material-removal simulation of the stock, in top view and in 3D;
  - playback move by move;
  - the cycle time and the chip load;
  - a setup sheet for the operator.
- **Projects that keep the drawing:**
  - several DXFs, each placed, turned and scaled;
  - text written in built-in fonts, placed and turned, to V-carve, pocket or cut out;
  - pictures (PNG, JPEG, BMP) traced into shapes to V-carve or follow;
  - shapes drawn in the app;
  - a new version of a DXF taken in, keeping the operations.
- **A sheet full of parts:** several parts with their own operations and grids of copies,
  checked against each other's toolpaths.
- **Your tools and machines:** a tool library and machine profiles, copied into each
  project and linked back. A project cuts the same on any computer.
- **Any controller:** GRBL, LinuxCNC and Mach3/4 posts, with arcs as G2/G3. Posts are a
  template and a small config, so you can copy one and make it yours.
- **Scriptable:**
  - every feature is on the command line;
  - `--json` on every command, and `get`/`set` for any value;
  - JSON Schemas for the files;
  - **AI assistants** (Claude and other MCP clients) can drive it through `chipload mcp`,
    while you watch the project update in the app.
- **No account, no network, no install:** an AppImage on Linux, a single `.exe` on
  Windows. Your files stay yours, under GPL-3.0-or-later.

Contributors: see `CLAUDE.md` for the architecture rules, conventions and workflow.

## Safety warning

**CNC machines are dangerous.** A wrong program can break tools, throw parts or swarf, damage
the machine and injure people. Chipload checks every program it writes, but the checker can
only catch the mistakes it knows about: it can't know your stock, your clamps, your fixture,
your tool or your machine's real state.

**You are responsible for verifying every generated program yourself** before you run it:

1. Look at the toolpath in a G-code viewer or simulator, for example
   [NC Viewer](https://ncviewer.com), [gSender](https://sienci.com/gsender/) or
   [CAMotics](https://camotics.org): depths, the work zero, clamps in the way, the cut
   order and the stock left holding the part.
2. Do an **air cut** on the actual machine: set the work zero above the stock (or raise Z)
   and run the program without cutting, with a hand on the stop button.
3. Only then cut, and never leave a running machine unattended.

Chipload comes with no warranty (GPL-3.0, sections 15 and 16).

## Download

Releases provide a Linux AppImage and a single Windows `.exe` for the GUI, plus the command
line for both, and the user guide to read offline (see `docs/releasing.md`).

## Quick start

Install Rust with [rustup](https://rustup.rs) (1.95 or newer), then:

```
cargo run --release -p chipload-gui -- part.dxf job.json    # GUI
cargo run --release -p chipload-cli -- --help               # command line
cargo test --workspace                                      # tests
```

## Usage

```
chipload import part.dxf -o part.chipload        # a project holding the drawing
chipload new -o part.chipload                    # an empty project, to draw in
chipload inspect part.chipload                   # units, contours, issues (also takes a .dxf)
chipload run part.chipload -o part.nc            # generate checked G-code
chipload update part.chipload --from part.dxf    # a new version of the drawing (or its first)
chipload copies part.chipload --grid 3x2 --gap 10   # cut the part six times
chipload check part.nc --job part.chipload       # the independent checker on any G-code
```

A project (`.chipload`) holds the drawing's geometry, so the DXF is only needed to import or
update it. Circles, rectangles and polylines (with arc segments as DXF bulges) can also be
drawn in the project (`chipload shape add …`, or the GUI's Draw menu); in the GUI their
handles can be dragged, and the cursor snaps to end points, centres and middles (Alt: off).
A project can also start empty (`chipload new`, *File > New project*) and take in a DXF later
(`update --from`, *File > Import DXF into project…*); drawn shapes are kept.

A part can be cut several times in a grid (`chipload copies`, or *Copies* in the part's
properties): columns × rows at a fixed pitch, no rotation, operation by operation across all
copies or copy by copy. Every copy is planned and checked like the original, and copies
whose cuts would overlap are refused. In the GUI the copies are drawn faded.

Your tools and machines can be kept between projects (`chipload tool add …`, `chipload
machine add …`, then `tool default` / `machine default`): a new project starts with the
default machine and tool, and `tool use` / `machine use` copy them into a project. They live
in your settings folder (`~/.config/chipload/`, `%APPDATA%\chipload\`, or
`CHIPLOAD_CONFIG_DIR`). A project keeps its own copy, so it cuts the same on any computer.
In the GUI: *This computer* at the top of the Browser holds the tool library and machine
profiles; drag a library tool onto the project's Tools to use it.

The GUI (`chipload-gui [part.chipload | part.dxf]`) is a project editor: import a DXF
(Ctrl+I) or open a project (Ctrl+O, or *File > Open recent*), click contours (Ctrl+click
for more, right-click for a whole layer), then an operation in the ribbon (**Profile**,
**Pocket**, **V-carve**, **Follow**, **Holes**). Operations run in the Browser's order;
drag them to reorder, untick to skip one. **F5** generates and shows the toolpath (each operation numbered
at its first plunge, arrows in the cutting direction, rapids dashed, tabs highlighted, and the simulated
stock showing what the tools leave; with an operation selected, a marker shows where each ring
starts: drag it along the ring to start the cut elsewhere, right-click it to reset); **Ctrl+S** saves
the project, **Ctrl+E** exports G-code, which is only possible after the check passes.
**3D** shows the toolpath at its real depths (left-drag turns, right-drag pans, Z can be
stretched in the View menu); **Top view** (T) goes back.
*File > Update drawing from DXF* takes in a re-exported drawing and keeps the operations.
`chipload run` on a saved project gives the same G-code.

The operations and their settings are described in the user guide (`docs/book/`), and
`docs/specs/` gives their exact behaviour and limits. `tests/corpus/jobs/` has example
projects. G-code is written only if it passes the independent checker.

For scripts: every command takes `--json`, `chipload get`/`set` read and change any project
value by path, and `chipload schema project` prints the project file's JSON Schema. See
`docs/book/src/scripting.md` (user guide). AI assistants (Claude Code, Claude Desktop and other
MCP clients) can drive every command through `chipload mcp` (`docs/book/src/assistants.md`).

The GUI uses installed system fonts (e.g. DejaVu Sans). On Linux, native file dialogs need
an XDG desktop portal or `zenity`; without them use *File → Open path…* or pass the files
on the command line.

The original Python implementation is kept at the git tag `python-final`.

## License

Copyright (C) 2026 Gustav Näslund

Chipload is free software: you can redistribute it and/or modify it under the terms of the
GNU General Public License as published by the Free Software Foundation, either version 3 of
the License, or (at your option) any later version. See [LICENSE](LICENSE).

Chipload generates programs that drive machines. It comes with no warranty: see the
[safety warning](#safety-warning) above.
