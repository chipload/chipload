# 0031: Scope: relief carving from an STL model

Status: **proposed** (2026-11-12), for the user's review with docs/specs/stl-relief.md.

## Context

ADR 0017 keeps Chipload a lean 2.5D tool and lists **3D surface machining** as out of scope
("real 3D CAM"). The user asked for STL engraving with a drop cutter as the next feature
(2026-11-12), after 0.35.0.

Carving a relief from the top is the most asked-for 3D job on a hobby router: a sign with
raised lettering, a decorative panel, a coin or a portrait, mostly from STL files people
download or model. Hobby programs offer it as a separate tier. The users Chipload is for
would otherwise leave for another program for that one job. They would also lose the
checker, the simulation, the time estimate and the setup sheet, which they now rely on.

## Decision

Take in **relief carving from an STL model, seen from above**, as a bounded exception to
ADR 0017's "out: 3D surface machining":

- **In:** one mesh placed on the stock like a picture (place, turn, scale, depth). It is
  cut from the top only, in 3 axes. A **finishing** raster is made with the drop cutter
  (flat, ball-nose, tapered ball-nose or V-bit), and **roughing** in levels before it. Both
  are checked, simulated and estimated as any operation is.
- **Still out:**
  - undercuts and 4th/5th axis;
  - waterline/constant-Z finishing, pencil and rest machining in 3D, adaptive clearing;
  - editing meshes or modelling in 3D;
  - formats other than STL (OBJ, STEP, 3MF);
  - STEP import for 2D outlines, a separate idea already noted in TODO.md.
- **Written from first principles and published work**, under the clean-room rule. The
  drop cutter is a well-known method: lower the tool along Z until it first touches the
  surface, testing each triangle's corners, edges and face. It is described in CAM
  textbooks on sculptured surface machining. No other CAM program's source is read.
- **The STL reader is our own**, in a new core crate `chipload-mesh`: bytes in, triangles
  out, no I/O, with limits against hostile input. The format is small and fully public:
  an 80-byte header, a count and 50 bytes per triangle, or the text form. Our own reader
  avoids a dependency for about 200 lines. `chipload-toolpath` gets the drop cutter.

## Consequences

- A schema bump: a new tool type (ball nose), the model in the project, a new operation.
- The checker stays independent. It checks 3D moves as it checks any G1 (limits, rapids,
  feed, depth below the stock). The new cuts get the verification every generated path
  gets: the pipeline tests the finished path against the mesh with a check of its own,
  simpler than the drop cutter (docs/specs/stl-relief.md §7).
- Programs get long: a raster has many short moves. The time estimate and the simulation
  must stay quick enough, which the spec measures.
- ADR 0017's out-of-scope list now names this exception.
