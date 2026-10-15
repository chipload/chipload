# Chipload

Chipload is 2.5D CAM for CNC routers and mills: a drawing in, checked G-code out.

- **In:** a DXF drawing, or simple shapes drawn in Chipload (circles, rectangles, polylines).
- **Out:** G-code for your controller, through a post-processor: GRBL, LinuxCNC and Mach3/4
  are built in, and you can write your own.
- **Operations:**
  - profiles, spiralling down, with tabs, finishing passes, onion skins and dogbones;
  - ring pockets, with helix entry and rest machining;
  - V-carving;
  - follow path;
  - drilled and helically bored holes.
- **Checked:** every program passes an independent checker before Chipload writes it. The
  checker is separate code that reads the G-code back and refuses rapids through the stock,
  cuts outside the machine, and other mistakes.
- **Seen before cut:** a 3D view, a material-removal simulation, a playback slider, the
  cycle time, the chip load, and a setup sheet for the operator.
- **Scriptable:** everything works from the command line too, with JSON output for scripts.

## Safety

**CNC machines are dangerous.** A wrong program can break tools, throw parts or swarf,
damage the machine and injure people. The checker catches the mistakes it knows about, but
it can't know your clamps, your fixture, your tool or your machine's real state. **You are
responsible for every program you run:**

1. Look at the toolpath: in Chipload's 3D view and simulation, or a G-code viewer such as
   [NC Viewer](https://ncviewer.com) or [CAMotics](https://camotics.org). Check the
   depths, the work zero, clamps in the way, the order of the cuts and what holds the part.
2. **Air-cut** first: set the work zero above the stock (or raise Z) and run the program
   without cutting, with a hand on the stop button.
3. Only then cut, and never leave a running machine unattended.

Chipload comes with no warranty (GPL-3.0, sections 15 and 16). Its licence and the
licences of everything it is built with are under *Help > About and licences*, and
`chipload licenses` prints them.

## What Chipload doesn't do

Chipload is meant to be lean. It leaves out what other tools do better:
- nesting parts on a sheet;
- 3D surface machining;
- sending G-code to the machine (use your controller's sender);
- CAD beyond a few shapes;
- facing the spoilboard.

## This guide

This guide is for Chipload's version in the title.
- **[Getting started](install.md):** install Chipload and make your first part.
- **Concepts:** what every operation shares: units, stock and zero, depths, sides,
  tools, and the checker.
- **One page per kind of operation**, then tools, machines and posts.
- **[Before cutting](checking/problems.md):** problems, the simulation, times and the setup
  sheet.
- **[Scripting](scripting.md)** and the generated **reference** pages, for every command
  and every field of the files.
