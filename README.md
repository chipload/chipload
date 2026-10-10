# Chipload

Cross-platform 2.5D CAM application (DXF in, G-code out), written in Rust with an egui GUI.

See `CLAUDE.md` for architecture rules, conventions, and workflow.

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
chipload inspect part.chipload                   # units, contours, issues (also takes a .dxf)
chipload run part.chipload -o part.nc            # generate checked G-code
chipload update part.chipload --from part.dxf    # take in a new version of the drawing
chipload check part.nc --job part.chipload       # the independent checker on any G-code
```

A project (`.chipload`) holds the drawing's geometry, so the DXF is only needed to import or
update it. Circles, rectangles and polylines can also be drawn in the project
(`chipload shape add …`, or the GUI's Draw menu).

The GUI (`chipload-gui [part.chipload | part.dxf]`) is a project editor: import a DXF
(Ctrl+I) or open a project (Ctrl+O), click contours (Ctrl+click for more, right-click for a whole layer), then **New
profile** or **New pocket**. Operations run in list order; drag them (≡) or use ↑/↓ to
reorder, untick to skip one. **F5** generates and shows the toolpath (each operation numbered
at its first plunge, arrows in the cutting direction, rapids dashed, tabs highlighted, material
the tool can't reach shaded); **Ctrl+S** saves
the project, **Ctrl+E** exports G-code, which is only possible after the check passes.
**3D** shows the toolpath at its real depths (left-drag turns, right-drag pans, Z can be
stretched in the View menu); **Top view** (T) goes back.
*File > Update drawing from DXF* takes in a re-exported drawing and keeps the operations.
`chipload run` on a saved project gives the same G-code.

Supported: **profile** operations (inside, outside, on) with optional holding tabs
(`"tabs": {"count": 4, "width": 8, "height": 3}`, width = bridge left on the part), and
**pocket** operations with islands; climb or conventional, a start and end depth with
multiple passes, a clearance (positive: larger pocket or smaller part, negative: stock left),
cuts into the spoilboard up to a set limit, one tool per job, and the GRBL post.
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

Chipload generates programs that drive machines. It comes with no warranty: always check the
toolpath in the viewer and do an air cut before machining.
