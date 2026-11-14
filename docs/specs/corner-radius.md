# End mills with a corner radius

Status: **approved 2026-11-14** ("do bull nose": decisions 1-6 as recommended); built
(§10). Asked for 2026-11-14 (the user: "I also want to have the possibility to
add corner radius to endmills if possible. It would be good to have such a tool for roughing
and smoothing perhaps. We should have the option at least.")

## 1. What it is for

An end mill with rounded corners, often called a *bull nose* or *radius end mill*: a
flat bottom, and a quarter round of radius *r* where the bottom meets the side. Makers buy
them for reliefs and 3D work:

- **Roughing:** the round corner is stronger than a sharp one, and leaves smaller steps
  on slopes than a square corner does.
- **Smoothing and finishing:** the flat bottom leaves no ridge between lines on flat
  ground, and the round corner leaves a ball nose's soft scallop on slopes. A wide relief
  with a flat background is finished faster than with a ball nose.
- **Pockets:** a floor that meets its walls in a fillet, which is stronger in some parts
  and is what the tool gives anyway.

Today a project can only say *end mill* (square) or *ball nose*. Cutting a relief with a
bull nose described as a square end mill makes the drop cutter keep the square corner off
the model: the path is safe but leaves up to *r* of extra material on every slope. Cutting
a pocket with one described as square is right on the walls but leaves a fillet the
simulation doesn't show.

## 2. The tool

A new optional field on a tool, `corner_radius` (mm), **for end mills only**:

- left out (or 0): a square end mill, as today;
- more than 0 and less than half the diameter: a bull nose;
- half the diameter or more: refused, "that is a ball nose: choose *Ball nose* as its
  kind" (`job-corner-radius-too-large`). On any other kind of tool: refused
  (`job-corner-radius-endmill-only`).

The tool library's tools carry the same field (they are `Tool`s), so a bull nose is added
to the library once and copied into projects as any tool is (ADR 0014).

## 3. Its shape: `Cutter::Bull`

`chipload_core::cutter` gains `Cutter::Bull { radius, corner }`: a flat disc of radius
`radius − corner`, and round it a quarter of a torus with tube radius `corner`. The
height above the tip at distance *d* from the axis is

- 0 for *d* ≤ *R* − *r*;
- *r* − √(*r*² − (*d* − (*R* − *r*))²) beyond, up to *d* = *R*.

The shape is convex, as the drop cutter requires (`cutter.rs`'s rule): the slope is 0 on
the flat and rises without limit towards the side. Its parts, for the drop cutter
(docs/specs/stl-relief.md §6):

- **kinks:** none (the flat meets the round with the same slope, 0); the flat's edge
  *R* − *r* is still offered as a candidate, which costs nothing;
- **`where_slope(m)`:** on the round, *d* = (*R* − *r*) + *r*·*m*/√(1 + *m*²): a plane of
  slope *m* touches the torus there;
- **`level_along(m, p)`:** a line's stationary point on the round has no closed form (it is
  a quartic). It is found by a search instead: the tip's reach along the edge is concave,
  so a golden-section search over the stretch of the edge under the round finds its
  highest point, to well under the chaining tolerance. Written from that property, not
  from any library.

Roughing's allowance (stl-relief.md §14) grows it: `grown(a)` is `Bull { radius: R + a,
corner: r + a }`. A bull nose grown by a ball of radius *a* is exactly that larger bull
nose, raised by *a*.

## 4. Where it counts

1. **Reliefs** (finishing, roughing in levels, smoothing): the drop cutter and the path
   check use the true shape. The default stepover from the ridge (stl-relief.md §15)
   works for a bull nose as it does for a ball nose, as it reads the shape's height at half
   the stepover: a stepover within the flat leaves no ridge on flat ground. The *Relief…*
   wizard's finishing tool list offers bull noses after the ball noses; its roughing list
   offers every end mill, bull noses included, as now.
2. **2.5D operations** (profile, pocket, surface, holes, follow path, rest pass, a
   V-carve's floor pass): the path is the one for an end mill of the same diameter, since
   the walls are the same. Two things change:
   - **A through cut** whose distance below the board (`through`) is less than the corner
     radius leaves a fillet on the bottom edge of the cut. Its part may hold on to the
     board by that edge. This gets a warning naming the radius (`warn-through-corner`).
   - **A pocket's or surface's stepover** wider than the flat (*D* − 2*r*) leaves ridges
     on the floor between passes. This gets a warning saying how high they are
     (`warn-corner-ridges`).
3. **The simulation and the 3D playback** draw the true shape, for every operation: fillets
   at the foot of pocket walls, a relief's scallops.
4. **The setup sheet and the tool lists** name it: "6 mm end mill, R1".
5. **The checker** (`chipload-check`) is unchanged: it checks moves, limits and arcs, not
   the tool's shape.

## 5. Command line and window

- `tool add NAME --type endmill --diameter 6 --corner-radius 1`, `tool set … --corner-radius`,
  `set PROJECT tools.N.corner_radius=1`; the JSON Schemas list the field.
- In the tool's properties (project and library): *Corner radius* under *Diameter*, shown
  for end mills only, empty for a square one.
- The guide's tool page explains the field and when to choose a bull nose; the relief page
  names it for roughing and smoothing.

## 6. Schema

An optional field, left out of the file when none (unlike `angle` and `tip_diameter`, which
are written as null): **no version bump**. A Chipload older than this refuses a project or a tool library that has one
(an unknown field), rather than cutting with a square corner it was never told about.

## 7. Edge cases and tests

- *r* = 0 is a square end mill (no `Bull`); *r* close to *D*/2 behaves like a ball nose
  (a test: the drop cutter's heights match `Ball` as *r* → *R*).
- The drop cutter: a bull nose on a flat plane, a slope, a vertex, an edge; against a ball
  nose and a flat end mill of the same diameter, the bull's heights lie between them
  (a proptest over random triangles and points).
- Roughing's grown bull keeps the allowance (the existing allowance test, with a bull).
- A through profile with `through` < *r* warns; one with `through` ≥ *r* doesn't.
- A pocket with stepover > *D* − 2*r* warns.
- Goldens: one relief with a bull nose finishing and roughing, one pocket with a bull nose
  (its G-code equal to the square end mill's; only the simulation differs).

## 8. Order of building

1. The field, its validation, `Cutter::Bull`, the drop cutter, the command line; a relief
   golden.
2. The simulation and the 3D playback; the warnings for 2.5D operations; the sheet.
3. The window: the tool form, the library, the wizard's lists; the guide.

## 9. Decisions

1. **A field on the end mill, not a new kind of tool.** `corner_radius` on `endmill`, so a
   bull nose works wherever an end mill does. *Recommended.* The alternative, a fifth kind
   `bullnose`, would need every operation to learn that it is an end mill too.
2. **Half the diameter or more is refused**, pointing to *Ball nose*, so there is one way
   to describe a ball nose. *Recommended* (the "one best way" rule).
3. **Reliefs use the true shape**, finishing, roughing and smoothing (§4.1), and the wizard
   offers bull noses for finishing. *Recommended.*
4. **2.5D operations keep an end mill's path and warn** (§4.2) about a through cut that
   doesn't reach *r* below the board and a stepover wider than the flat. *Recommended.*
   The alternative, deepening a through cut by itself or narrowing the stepover by itself,
   would change the user's numbers without asking.
5. **The simulation draws the true shape for every operation** (§4.3). *Recommended.*
6. **No schema bump** (§6). *Recommended.*

## 10. Settled while building

- 2026-11-14: built in one go, all three steps. `Tool::corner_radius` and
  `LibraryTool::corner_radius` (skipped when none); `Tool::is_bull_nose`, `Tool::is_round`
  (ball or bull nose: the relief's ridge rule and the wizard's finishing list); messages
  `job-tool-corner-radius-too-large` and `job-tool-corner-radius-endmill-only` (the
  rectangle's `job-corner-radius-too-large` was taken); `toolpath-through-corner` and
  `toolpath-corner-ridges` as notes when planning. The level-point search returns the foot
  at once for a level line (most 2.5D moves in the simulation), exactly. Golden
  `v36_relief_bull`; `crates/chipload-pipeline/tests/bull_nose.rs`. Lists name the tool
  "Ø6 mm R1 mm"; the sheet adds "corner radius 1 mm".
