# Moulding: a profile swept along a path

Status: **draft 2026-11-15**, for review (fourteen decisions, §13). TODO *Ideas*, "More 2.5D
toolpaths", item 4; the user asked for the next feature after the prism.

## 1. What it is for

A **moulding** is a shaped edge or band: a cove or an ogee round a table top's face, a bead
along a cabinet door's frame, a round-over beside a panel's raised field, a decorative band
across a box lid. Today it needs a moulding bit in a router table, or a 3D model carved as a
relief. A **Moulding** operation takes a path (the line the moulding runs along) and a
**profile** (its cross-section), and cuts the profile along the path with a ball nose.

## 2. Scope: 2.5D, not a surface (ADR 0017, ADR 0031)

ADR 0017 keeps 3D surface machining out; ADR 0031 let in relief carving from an STL as one
bounded exception. The TODO note planned moulding as raster passes across the path, which
would stretch that exception. This spec doesn't need it:

- The profile is a 2D cross-section. For each distance *d* from the path, the depth the
  tool's tip must go to so that it never cuts below the profile is found by dropping the
  tool's shape onto the cross-section: one dimension, exact, no mesh.
- Each pass is the path **offset** by *d*, cut at that one depth: flat offsets with their
  arcs, as a profile's laps, verified as every offset is.

So every move is 2.5D, as the cuts a moulding bit makes. It is recorded in ADR 0017's
in-scope list (2026-11-15) as a 2.5D operation, not as a second 3D exception (decision 1).

## 3. The profile

The cross-section across the path, *d* from 0 at the path to *W* (the profile's width), its
depth *f(d)* below the start depth (0 at the surface). Two ways to give it (decision 2):

- **A built-in shape**, by its size:

  | Profile | Settings | Cross-section |
  |---|---|---|
  | **Cove** | radius *R* | a concave quarter circle: *R* deep at the path, rising to the surface at *R* |
  | **Round-over** | radius *R* | a convex quarter circle: the edge rounded, *R* down at the path, at the surface *R* in |
  | **Ogee** | width, depth | an S: a cove into a round-over, half the width each |
  | **Bead** | radius *R* | a raised half round *2R* wide, its top at the surface, a groove *R* deep each side |
  | **Flute** | radius *R* | a half-round groove *2R* wide and *R* deep |

- **A drawn shape:** any open shape of lines and arcs drawn in the project, read as the
  section as you would draw it: across to the right, up the page. Its leftmost point is at
  the path; its highest point is the surface; where it doubles back, the lowest part
  counts. Drawn as construction geometry, it is never cut itself (drawn shapes can be
  construction since schema 32), and it can be anywhere on the canvas. The profile
  follows the shape: change the shape, and the moulding changes with it.

Where *f* is above the surface it is cut off at the surface; *W* is at most 200 mm.

## 4. The path and the side

The selected shapes are the paths: open or closed, lines, arcs, polylines, circles.

- **Side** (decision 3): the profile lies on one side of the path, *d* growing away from
  it: **left** or **right** of an open path as drawn; **inside** or **outside** of a closed
  one. A new moulding takes *outside* for a part's outline and *inside* for a hole's, as a
  new profile does; *left* for open paths.
- **Corners:** at a corner turning away from the profile, the offsets go round it in arcs,
  so the profile turns round the corner as a bit's would. At a corner turning towards it,
  the offsets meet in a mitre, as a mitred frame's corner.
- **Ends of an open path:** the moulding stops square at the path's ends, as a moulding
  that runs off the board's edge. (Stopped ends, rounded as a bit's, are for later.)

## 5. The tool's depth across the profile

With the tool's shape (`Cutter::height_at`, the shape the relief's drop cutter uses: ball,
tapered ball, flat, bull nose), the tip at distance *c* from the path goes to

    z(c) = the highest of  f(d) − height(|d − c|)  over every d within the tool's radius of c

(depths as negative Z below the start depth). The tool's surface then touches the profile
and nowhere goes below it, including near steep or concave parts that a ball can't reach,
which it bridges as a moulding bit of that size would. Beyond the profile's ends (*d* < 0
and *d* > *W*) the section counts as the surface, untouched (decision 4).

- The **passes** are at *c* = 0, *s*, 2*s*, … up to *W*: the stepover *s* across the profile
  (§6), and one at *W* exactly; each pass is the path offset by *c* on the profile's side,
  cut at *z(c)*. Passes whose *z* is at the surface are left out.
- A pass at *c* = 0 runs on the path itself (the tool's centre on it): with a ball nose its
  other half cuts beyond the path, into what is outside the moulding (decision 4).

## 6. Passes, roughing and order

- **Stepover** across the profile: a new moulding takes the relief's rule for a ball nose
  (the widest stepover that leaves ridges no higher than 0.02 mm on flat ground,
  stl-relief.md §15); for a flat end mill a quarter of its diameter.
- **Step down** (roughing): where the profile is deeper than the step down, the passes are
  cut in levels, each pass no deeper than its level (as a pattern's or a flute's passes are
  cut off), level by level from the top, so the tool never takes more than one step. The
  last level is the finished profile.
- **Order:** level by level; within a level, from the passes at the surface edge inwards to
  the deepest, each pass a closed ring (closed paths) or one way along the path (open
  paths, as a follow path's first traversal), a retract between them. Linking passes
  without retracting is for later.
- **Direction:** climb or conventional, as a profile's.

## 7. Format

A new operation type; no schema bump, as the operations since the chamfer.

```json
{"type": "moulding", "name": "", "enabled": true, "part": 1, "material": null,
 "select": {"shapes": [1]}, "tool": 3,
 "profile": {"kind": "cove", "radius": 8.0},
 "side": "outside", "start_depth": 0.0, "stepover": 0.6, "step_down": 2.0,
 "direction": "climb", "feed": 1500.0, "plunge_feed": 500.0}
```

- `profile`: one of `{"kind": "cove", "radius"}`, `{"kind": "roundover", "radius"}`,
  `{"kind": "ogee", "width", "depth"}`, `{"kind": "bead", "radius"}`,
  `{"kind": "flute", "radius"}`, `{"kind": "shape", "shape": ID}` (a drawn open shape's id).
- `side`: `left`, `right`, `inside` or `outside`.
- `stepover`: mm across the profile, more than 0.
- `step_down`: null for one level (the finished profile only: for shallow profiles).
- Refused: a profile wider than 200 mm, a drawn shape that is missing, closed, or not
  read as a section (no width), a drill or a V-bit (§8), a stepover wider than the tool.

## 8. Tools

| Tool | |
|---|---|
| **Ball nose** (plain or tapered) | the usual: smooth curves, ridges set by the stepover |
| **End mill** (flat or bull nose) | allowed: steps on curves, exact on flat and vertical parts; for roughing a deep profile before a ball-nose moulding, or square profiles |
| **V-bit**, **drill** | refused: a V-bit's point can't follow a curve without scoring it; use a chamfer |

A note gives the profile's deepest point and where the tool can't reach the profile (a
concave curve tighter than the tool's radius): "the 6 mm ball can't reach the cove's
bottom: it stays 0.4 mm above it".

## 9. Command line and window

- **Command line:** `chipload op add part.chipload moulding --shapes 1 --profile cove
  --radius 8 --side outside` (also `--profile ogee --width 20 --depth 10`, `--profile
  shape --profile-shape 7`, `--stepover`, `--tool`); `set …operations.N.profile.radius=6`.
- **Window:** *Moulding* in the S menu (key **M**), the Operation menu (*New Moulding*)
  and the Browser's new operations; not on the ribbon (no room). Its properties: Tool,
  Side, Profile (the five shapes and *Drawn shape*, with its settings or a shape picker),
  a small drawing of the section with the tool's passes, Start depth, Stepover, Step down,
  Direction, Feeds.
- **Defaults** for a new moulding: the project's first ball nose, else its first end mill;
  a cove of 6 mm; side from the shapes as a new profile's; the stepover from the tool;
  the tool's step down; climb.
- **Setup sheet:** "moulding, cove R 8, outside, climb", with its depth and stepover.
- **Guide:** a page *Moulding* under Operations, with a picture of the five profiles.

## 10. Edge cases

- A straight path, an arc, a closed rectangle (outside: round corners; inside: mitres).
- A profile wider than a closed path's inside (the offsets close up): the passes stop where
  the offset vanishes, a note.
- A drawn profile with vertical parts, overhangs (the lowest part counts), and arcs.
- A cove tighter than the ball: bridged, a note.
- Step down none, dividing the depth, not dividing it.
- Start depth above 0 (a moulding in a pocket's floor).
- Open path ends: square.

## 11. Tests

- The 1D drop: for a cove and a ball of the same radius, *z(c)* is the cove itself shifted
  by the ball; for a flat end mill, the step shape; against an independent check that the
  tool at each pass touches the profile and is nowhere below it (sampled).
- Moves: each pass is the path's offset at *c* on the right side (within the chaining
  tolerance), at *z(c)*; passes ordered as §6; levels never more than a step.
- Simulated: a straight cove 8 mm along a line with a 6 mm ball: the height map's section
  matches the cove within the ridge height and the bridging the note gives; a closed
  rectangle outside: the corner turned round.
- A proptest: random paths (lines and arcs), profiles and tools: no simulated point below
  the profile (no gouge), and the deepest within the tolerance of the profile's.
- Refusals and notes; the drawn profile follows its shape.
- Golden: a new `v36/moulding.chipload` (a rectangle with an ogee outside with a 6 mm ball
  in levels; a line with a bead; a drawn profile along an arc) and its `lines/` twin;
  existing goldens unchanged.
- CLI: `op add moulding` and its options; the window: the S menu's M, the form.

## 12. Order of building

1. The profiles and the 1D drop (with tests), the offsets and passes, format, validation,
   planning, the command line, goldens.
2. Window: the form with the section drawing, the S menu, the Operation menu; sheet; guide;
   the ADR 0017 note.

## 13. Decisions

1. **Scope: a 2.5D operation, recorded in ADR 0017** (§2), not a second 3D exception.
   *Recommended:* the profile is a 2D section and every pass a flat offset, the cut a
   moulding bit makes. The alternative, raster passes across the path over a surface, is
   real 3D machining, stretches ADR 0031, and leaves a worse finish along the moulding.
2. **Profiles: five built-in shapes and any drawn open shape** (§3). *Recommended:* the
   built-ins cover the usual jobs in a few clicks; a drawn shape covers everything else
   without a profile editor. The alternative, drawn shapes only, makes a simple cove a
   drawing job every time.
3. **The profile to one side of the path** (§4): left/right of an open path, inside/outside
   of a closed one, from the shapes for a new one. *Recommended*, as profiles and
   chamfers.
4. **What lies beyond the profile's ends counts as untouched surface; the first pass runs
   on the path** (§5), so with a ball nose half the tool cuts beyond the path at the
   profile's depth there. *Recommended* for cove, ogee and round-over at a part's edge,
   where beyond the path is waste (the cut-out comes after). For a moulding in the middle
   of a board (a bead, a flute, a band), the profile starts and ends at the surface, so
   nothing is cut beyond it. The alternative, keeping the tool wholly on the profile's
   side, can't reach a profile's deepest point at the path.
5. **The tool's depth from a 1D drop of its own shape** (§5): exact for any cutter shape,
   bridging what it can't reach, with a note. *Recommended.*
6. **Passes along the path, offsets one stepover apart** (§5, §6). *Recommended:* the grain
   of the finish runs along the moulding, the passes keep their arcs, and there are few
   moves. Raster passes across: see decision 1.
7. **The stepover from the relief's ridge rule for a ball nose** (§6). *Recommended:* one
   rule for "smooth enough" across the program.
8. **Roughing in levels within the same operation** (§6), each pass cut off at its level.
   *Recommended:* a deep cove can't be cut in one go, and a separate roughing operation
   would need its own geometry.
9. **A retract between passes** (§6). *Recommended* for now: simple and safe; linking
   passes without lifting is a later optimisation.
10. **Tools: ball nose and end mill; V-bit and drill refused** (§8). *Recommended.*
11. **Square ends on open paths** (§4). *Recommended:* the common case (a moulding running
    off the edge); stopped ends later if asked for.
12. **No schema bump** (§7). *Recommended*, as before.
13. **The S menu's M, the Operation menu, not the ribbon** (§9). *Recommended.*
14. **Defaults** (§9): the first ball nose, a 6 mm cove, side from the shapes, stepover from
    the tool. *Recommended.*
