# 0017: Scope: a lean power tool

Status: accepted (2026-10-13)

Chipload is meant to be a power tool for power users, mean and lean: no features that would
go unused because another tool always does the job better. Every proposed feature is
weighed against that. This record says what is in, what is out, and how to decide.

## What Chipload is
2.5D CAM for CNC routers and mills: a drawing (DXF, or simple shapes drawn in Chipload) in,
checked G-code out, for any machine through data-driven posts. Usable headless (the CLI and
the project file are first-class) as well as in the GUI.

## In scope
- The operations a router shop uses every day, done well: profiles (with tabs, leads,
  spiral laps, finishing passes, onion skins, dogbones), ring pockets, V-carving, follow
  path, holes (drilling, helical boring).
- Safety: the independent checker, verified offsets, explicit units and sides.
- Seeing what will happen: the 3D view, the material-removal simulation, playback.
- Leverage for people who use it a lot: the CLI, a stable and documented project format,
  the tool and machine library, presets (rules keyed to the drawing's layers).

- **Images traced into shapes** for V-carving (added 2026-10-13, docs/specs/image-trace.md):
  another tool can trace, but the threshold and smoothing are judged against the carve,
  which only Chipload shows, and it feeds the existing V-carving. Picture editing and colour
  separation stay out. Centre lines of strokes, for a follow path, were taken in on
  2026-10-13 with the user's approval (docs/specs/image-arcs-centrelines.md): the same
  argument, and they reuse V-carving's medial axis.

## Out of scope (another tool does it better)
- **Nesting** parts on a sheet: dedicated nesters (Deepnest and the like) or CAD.
- **3D surface machining**: real 3D CAM.
- **Machine control**, sending G-code, jogging, probing: senders (UGS, CNCjs, gSender) and
  the controller's own software.
- **CAD** beyond the few shapes Chipload draws (circle, rectangle, polyline): any CAD
  program. The drawing tools are frozen as they are.
- **Surfacing and facing** (decided 2026-10-13): skipped. A flattening program is a few
  lines of G-code that any sender or a small script makes; reconsider only if users ask.

## Removed
- **Trochoidal and adaptive pocket clearing** (2026-10-13). Experimental since 2026-10-12,
  not offered for new pockets, and about 2 000 lines of code with an unfinished second
  stage. On a router, ring pockets with a helix entry cover nearly every pocket. The code
  is kept at the local tag `adaptive-final`. Projects that use them still load unchanged
  (the format keeps the two strategies), and generating is refused with a message until
  the pocket is switched to rings (one click in the GUI). They are not converted to rings
  on load, which would change how a saved project cuts without asking.

## How to decide on a new feature
1. Would a power user reach for something else to do it? Then it is out; say so here.
2. Does it multiply what users can do with what exists (presets, scripting) rather than
   add a separate thing? Prefer that.
3. Does it overlap something Chipload already shows or does? Merge, or replace the old one.
4. A feature that was built and isn't used, or stays experimental, is removed, not kept.
