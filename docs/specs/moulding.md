# Moulding: a profile swept along a path

Status: **approved 2026-11-15**, every decision as recommended ("go"), after the revision
for the user's word on scope (ADR 0017, 2026-11-15); **built 2026-11-15**, both steps of
§12. What building settled is in §14. TODO *Ideas*, "More 2.5D toolpaths", item 4.

## 1. What it is for

A **moulding** is a shaped edge or band: a cove or an ogee round a table top's face, a bead
along a cabinet door's frame, a round-over beside a panel's raised field, a decorative band
across a box lid. Today it needs a moulding bit in a router table, or a 3D model carved as a
relief. A **Moulding** operation takes a path (the line the moulding runs along) and a
**profile** (its cross-section), and cuts the profile along the path with a ball nose.

## 2. How it is cut, and scope (ADR 0017)

ADR 0017 (2026-11-15): what stays out is machining driven by a 3D model (a STEP file);
an operation that cuts a 3D shape from 2D input is judged on its merits. A moulding is
exactly that: a 2D path and a 2D section. So the approach is chosen for the result, and it
is the cut a moulding bit makes:

- **Passes along the path.** The profile is a 2D cross-section. For each distance *c* from
  the path, the depth the tool's tip must go to so that it never cuts below the profile is
  found by dropping the tool's shape onto the section (one dimension, exact, no mesh).
  Each pass is the path **offset** by *c*, cut at that depth: offsets with their arcs, as a
  profile's laps, verified as every offset is. The finish runs along the moulding, arcs
  stay arcs, and there are few moves.
- **3D where the depth changes along the path:** a **tapered end** (§4) fades the profile
  out over a length, so along it each pass goes up and down with the section's depth
  there: 3D moves, from the same 1D drop at each point.
- **Not raster passes across the path over a surface:** more general in principle, but a
  worse finish along the moulding, far more moves, and nothing a moulding needs that the
  passes along it can't do (decision 1).

ADR 0017's in-scope list gets a line for it (built from 2D input, as the relief's 3D moves
come from a model placed in 2D).

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
- **Ends of an open path** (decision 11), the same at both ends:
  - **Square:** the full profile right to the path's end, as a moulding that runs off the
    board's edge (the default).
  - **Round:** the profile turns round the path's end point, a quarter turn on its side,
    as a moulding bit stopped and lifted out leaves it: each pass goes on round the end in
    an arc of its own offset. Still flat passes.
  - **Tapered** over a length: the section's depth fades to the surface along the last
    stretch of the path (straight or curved, as a flute's ramp, fluting.md §3), so the
    moulding dies out into the board, as a stopped flute does. Along it each pass rises
    with the section's depth there (3D moves, §2). Shortened in proportion when the two
    tapers are longer than the path, as a flute's ramps.
- Closed paths have no ends.

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
- **Along a taper**, at a point where the section's depth is scaled by *k* (1 at the
  taper's start, 0 at the path's end), the pass goes to the drop of the tool onto *k·f*.
  Points are added along the pass until *z* between two of them is within the arc
  tolerance (0.01 mm); a pass whose *z* reaches the surface there ends.
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
 "side": "outside", "ends": {"kind": "square"},
 "start_depth": 0.0, "stepover": 0.6, "step_down": 2.0,
 "direction": "climb", "feed": 1500.0, "plunge_feed": 500.0}
```

- `ends`: `{"kind": "square"}`, `{"kind": "round"}` or
  `{"kind": "taper", "length": 30.0, "shape": "straight"}` (`curved` as a flute's ramp);
  for open paths only, ignored on closed ones.

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
  shape --profile-shape 7`, `--ends round`, `--ends taper --taper 30 --taper-shape curved`,
  `--stepover`, `--tool`); `set …operations.N.profile.radius=6`.
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
- Open path ends: square, round (a quarter turn round the end), tapered (straight and
  curved; longer than half the path: shortened).

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
  existing goldens unchanged; plus a line with tapered ends and an arc with round ends.
- CLI: `op add moulding` and its options; the window: the S menu's M, the form.

## 12. Order of building

1. The profiles and the 1D drop (with tests), the offsets and passes, format, validation,
   planning, the command line, goldens.
2. Window: the form with the section drawing, the S menu, the Operation menu; sheet; guide;
   the ADR 0017 note.

## 13. Decisions

1. **Passes along the path, 3D only where the depth changes along it** (§2): offsets one
   stepover apart at the 1D drop's depth, rising and falling along a taper; recorded in
   ADR 0017's in-scope list under the 2026-11-15 rule. *Recommended:* the finish runs
   along the moulding, arcs stay arcs, few moves, and the 3D moves give the stopped ends a
   moulding needs. The alternative, raster passes across the path over a computed surface,
   is more general but leaves a worse finish and far more moves for nothing a moulding
   needs.
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
11. **Ends of open paths: square, round or tapered** (§4), the same at both ends, square by
    default. *Recommended:* square for a moulding running off the edge; round for the end
    a stopped bit leaves; tapered (now possible with 3D moves) for a moulding that dies out
    into the board. Different ends at each end, or a profile that changes along the path,
    can come later if asked for.
12. **No schema bump** (§7). *Recommended*, as before.
13. **The S menu's M, the Operation menu, not the ribbon** (§9). *Recommended.*
14. **Defaults** (§9): the first ball nose, a 6 mm cove, side from the shapes, stepover from
    the tool. *Recommended.*

## 14. As built

- **The drop** reads the section on its own sample points (every 0.01 mm), straight
  between them, and also at the tool's rim on both sides, so a steep wall between two
  samples still stops the tool. A proptest checks that the dropped tool touches the
  section and is nowhere below it, for random profiles, balls and end mills.
- **Before the path** (*d* < 0) is waste when the section starts below the surface (cove,
  ogee, round-over at an edge) and untouched when it starts at the surface (bead, flute, a
  drawn band): decision 4 made exact.
- **A drawn profile** is read from the shape in the project (`Shape::segments`), not from
  the contours cut: construction geometry never reaches them. A missing or closed shape is
  refused, and the shape is never a path of its own moulding. Its depth, unknown to the
  project's checks, is checked against the stock when planning.
- **Open offsets** are cavalier's open-polyline offsets (positive to the left), verified
  as no nearer the path than the pass's distance; round ends add a quarter arc of the
  pass's radius about each end point, recorded as arcs.
- **Tapers** take the flute's ramp (`flute::Profile`) along the path; each pass point's
  place along the path is its nearest point on it. Points are added along a taper until Z
  is within the arc tolerance, and dropped again on straight runs where Z changes evenly.
- **The bridge note** compares the tool's reach (passes at every 0.05 mm) with the section
  and speaks from 0.05 mm: a bead's sides or a drawn wall give it, as they should.
- **Command line:** `--radius`, `--width`, `--depth` (an ogee's), `--side` (now also left
  and right) and `--ramp-shape` (a taper's) are shared with the other operations; new
  are `--profile`, `--profile-shape`, `--ends`, `--taper` and `--stepover`.
- **The section in the form** (the user, 2026-11-15: "a preview of the profile next to it
  and not just the name"): the profile list shows each profile's section beside its name,
  and the chosen one is drawn true to scale under its settings (`moulding::preview` gives
  the points; the form only paints them: the board in a wood tone with the profile cut in,
  a tick at the path). A drawn profile that can't be read shows a question mark.
- **Left for later:** the tool's passes in that drawing, different ends at each end, a
  profile that changes along the path, linking passes without lifting, and a moulding
  step in the relief wizard (the user's idea, 2026-11-15).
- **Code:** `chipload-toolpath/src/moulding.rs` (section, drop, passes, ends, moves),
  `plan_moulding_op`; `MouldingOp`, `MouldProfile`, `MouldSide`, `MouldEnds` in the job
  model; `OpKind::Moulding`; `op add moulding`; the form's `moulding_sections`;
  `Icon::Moulding`.
