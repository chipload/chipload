<h1><img src="packaging/brand/chipload-wordmark.svg" alt="Chipload" height="72"></h1>

2.5D CAM for CNC routers and mills: drawings in (DXF), checked G-code out. Free software
for Linux, Windows and macOS, written in Rust.

**[User guide](https://chipload.github.io/chipload/)** · **[Download](https://github.com/chipload/chipload/releases)**

![Chipload: a plaque with a pocket round an island, bored holes, the Chipload logo V-carved
from a traced picture and an outline held by tabs, its toolpaths over the simulated stock in
3D](docs/book/src/images/showcase.png)

> **Testers with real machines wanted.** Chipload's programs pass its own checker, but they
> haven't yet run on a real controller (GRBL, LinuxCNC, Mach3/4), nor in inches. If you have
> a CNC router or mill, please try it, starting with an air cut, and
> [tell us how it went](#testing-on-your-machine).

## Highlights

- **Checked G-code, always.** An independent checker reads every program before it is
  written: machine limits, rapids through the stock, retracts, arcs, feeds.
- **Operations:** profile (spiral, tabs, finishing pass, onion skin, dogbones), pocket
  (islands, spiral rings, rest passes, helical entry), V-carve, follow path, holes (drilled
  or bored) and surfacing.
- **See it before you cut:** a material-removal simulation in 3D, playback, cycle time,
  chip load and a setup sheet for the operator.
- **Drawings:** several DXFs, text in built-in fonts, traced pictures and shapes drawn in
  the app; a new version of a DXF keeps the operations.
- **A sheet of parts:** several parts with their own operations and grids of copies.
- **Your tools and machines** in a library, copied into each project, so it cuts the same
  on any computer.
- **Any controller:** GRBL, LinuxCNC and Mach3/4 posts with arcs, in mm or inches. A post is
  a template and a small config, easy to copy and change.
- **Scriptable:** every feature on the command line, `--json` everywhere, and `chipload mcp`
  for AI assistants.
- **No account, no network, no install.** Your files stay on your computer.

## Safety warning

**CNC machines are dangerous.** A wrong program can break tools, throw parts, damage the
machine and injure people. The checker only catches the mistakes it knows about: it can't see
your stock, clamps, tool or machine.

**Check every program yourself before you run it:** look at it in a G-code viewer (such as
[NC Viewer](https://ncviewer.com) or [CAMotics](https://camotics.org)), do an **air cut**
with a hand on the stop button, and never leave a running machine. Chipload comes with no
warranty (GPL-3.0, sections 15 and 16).

## Testing on your machine

Every report helps, "it worked" too. Start with a shallow cut in scrap, after an air cut.
Useful to test: your controller and sender, arcs and helices, holes, several tools, inch
G-code, and whether the work zero and depths match the setup sheet.

[Open an issue](https://github.com/chipload/chipload/issues) with the machine, controller
and sender, the Chipload version and post, the project file and G-code, and what happened.
Problems found this way become test cases.

## Download

[Releases](https://github.com/chipload/chipload/releases) have the app as one file,
`chipload-X.Y.Z-…`, for Linux (AppImage) and Windows (`.exe`), x86_64 and ARM64, and macOS
(universal `.dmg`). The command line for scripts is in the `chipload-cli-X.Y.Z-…` archives,
and the user guide to read offline in `chipload-guide-X.Y.Z.zip`.

## Build from source

With Rust 1.95 or newer ([rustup](https://rustup.rs)):

```
cargo run --release -p chipload-gui                      # the app
cargo run --release -p chipload-cli -- --help            # the command line
cargo test --workspace                                   # the tests
```

```
chipload import part.dxf -o part.chipload        # a project from a DXF
chipload run part.chipload -o part.nc            # checked G-code and a setup sheet
chipload check part.nc --job part.chipload       # the checker on any G-code
```

Everything else is in the [user guide](https://chipload.github.io/chipload/).
Contributors: `CONTRIBUTING.md`, `docs/architecture.md` for an overview, and `CLAUDE.md` for
the rules.

## License

Copyright (C) 2026 Gustav Näslund

Chipload is free software: you can redistribute it and/or modify it under the terms of the
GNU General Public License as published by the Free Software Foundation, either version 3 of
the License, or (at your option) any later version. See [LICENSE](LICENSE).

**What you make with Chipload is yours.** The licence covers Chipload itself, not what you
make with it: your drawings, projects, G-code and parts are yours to use and sell. Text cut
in the built-in fonts (GNU FreeFont) is free of it too, by the fonts' own exception.
