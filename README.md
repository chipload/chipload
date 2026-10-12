# Chipload

Cross-platform 2.5D CAM application (DXF in, G-code out), written in Rust with an egui GUI.

See `CLAUDE.md` for architecture rules, conventions, and workflow.

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
line for both (see `docs/releasing.md`).

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

A part can be cut several times in a grid (`chipload copies`, or *Copies* under Job
settings): columns × rows at a fixed pitch, no rotation, operation by operation across all
copies or part by part. Every copy is planned and checked like the original, and copies
whose cuts would overlap are refused. In the GUI the copies are drawn faded.

Your tools and machines can be kept between projects (`chipload tool add …`, `chipload
machine add …`, then `tool default` / `machine default`): a new project starts with the
default machine and tool, and `tool use` / `machine use` copy them into a project. They live
in your settings folder (`~/.config/chipload/`, `%APPDATA%\chipload\`, or
`CHIPLOAD_CONFIG_DIR`). A project keeps its own copy, so it cuts the same on any computer.
In the GUI: *Settings > Tool library…* and *Machines…*, and *Use profile…* / *Save as
profile…* (and the same for the tool) under Job settings.

The GUI (`chipload-gui [part.chipload | part.dxf]`) is a project editor: import a DXF
(Ctrl+I) or open a project (Ctrl+O, or *File > Open recent*), click contours (Ctrl+click for more, right-click for a whole layer), then **New
profile** or **New pocket**. Operations run in list order; drag them (≡) or use ↑/↓ to
reorder, untick to skip one. **F5** generates and shows the toolpath (each operation numbered
at its first plunge, arrows in the cutting direction, rapids dashed, tabs highlighted, material
the tool can't reach shaded; with an operation selected, a marker shows where each ring
starts: drag it along the ring to start the cut elsewhere, right-click it to reset); **Ctrl+S** saves
the project, **Ctrl+E** exports G-code, which is only possible after the check passes.
**3D** shows the toolpath at its real depths (left-drag turns, right-drag pans, Z can be
stretched in the View menu); **Top view** (T) goes back.
*File > Update drawing from DXF* takes in a re-exported drawing and keeps the operations.
`chipload run` on a saved project gives the same G-code.

Supported: **profile** operations (inside, outside, on) with optional holding tabs
(`"tabs": {"count": 4, "width": 8, "height": 3}`, width = bridge left on the part), and
**pocket** operations with islands, cleared in rings (trochoidal and adaptive clearing are experimental: still planned for projects that use them, no longer offered); climb or conventional, a start and end depth with
multiple passes, a clearance (positive: larger pocket or smaller part, negative: stock left),
cuts into the spoilboard up to a set limit, straight plunges, ramp entry (down along the path, for tools that aren't centre-cutting) or, for pockets, helical entry (down round a small circle, written as helical G2/G3), start points chosen per ring (`"starts": [[x, y]]`, each moving the start of the ring nearest to it), lead-in and lead-out arcs from the waste (`"lead": {"radius": 3}`; profiles inside or outside, and pocket walls), one tool per job, and the GRBL post (arcs written as G2/G3).
G-code is written only if it passes the independent checker. See
`tests/corpus/jobs/` for example projects and `docs/specs/` for exact behaviour and
limitations.

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
