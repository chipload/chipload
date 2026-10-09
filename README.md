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
chipload inspect part.dxf                        # units, entities, contours, issues
chipload run part.dxf --job job.json -o part.nc  # generate checked G-code
chipload check part.nc --job job.json            # the independent checker on any G-code
```

The GUI (`chipload-gui [part.dxf] [job.json]`) is a job editor: open a drawing, start a job
(Ctrl+N), click contours (Ctrl+click for more, right-click for a whole layer), then **New
profile** or **New pocket**. Operations run in list order; drag them (≡) or use ↑/↓ to
reorder, untick to skip one. **F5** generates and shows the toolpath (start points numbered in
cut order, arrows in the cutting direction, rapids dashed, tabs highlighted); **Ctrl+S** saves
the job, **Ctrl+E** exports G-code, which is only possible after the check passes. Saved
jobs are ordinary job files: `chipload run` gives the same G-code.

Supported: **profile** operations (inside, outside, on) with optional holding tabs
(`"tabs": {"count": 4, "width": 8, "height": 3}`, width = bridge left on the part), and
**pocket** operations with islands; climb or conventional, multiple depth passes, one tool
per job, and the GRBL post.
G-code is written only if it passes the independent checker. See
`tests/corpus/jobs/v3/` for example jobs and `docs/specs/` for exact behaviour and
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
