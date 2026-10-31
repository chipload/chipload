# Spec: V-carving closed shapes

Status: **approved 2026-10-11**, all decisions as recommended (§11). Built as described;
the details in §4 and §8 were settled while building it (noted there).

Guide: docs/book/src/operations/vcarve.md.

User request (2026-10-11): engrave text with serifs (from a DXF, later typed in the app or
traced from a black-and-white picture) with a V-bit that follows the centre line of each
shape and varies its depth so the cut is exactly as wide as the shape.

This spec is **step 1**: a V-carve operation for closed contours that are already in the
drawing (imported from a DXF or drawn). Out of scope, each a later spec of its own: text typed
in the app (fonts), tracing pictures, clearing a flat floor with a second tool, inlays, DXF
`TEXT`/`MTEXT` entities (text has to be converted to outlines in CAD first).

## 1. What "correct" means

The selected contours form **regions**: an outer contour minus the contours inside it, as for
pockets (pocket.md §2). For a V-bit with included angle α and tip diameter w (0 for a pointed
bit), let t = tan(α/2).

The tool follows the region's **medial axis**: the points with two or more nearest points on
the region's boundary. At a point p of it, with r(p) the distance from p to the boundary, the
tip goes to

    depth(p) = (r(p) − w/2) / t        (below the surface where the carving starts)

There, the cone touches the boundary at the surface, and is nowhere wider than the region.
Because every point of a region lies in a disc of the medial axis, the cuts together
reach the whole outline at the surface. Narrow strokes come out shallow, wide strokes deep.
At a sharp outside corner (a serif's tip), the axis runs into the corner and the depth goes to
0, so the corner stays sharp.

So, at the surface, for every cutting move:

1. **No gouge:** the cone's circle at the surface lies inside the region, within `VCARVE_TOL_MM`.
2. **Full reach:** every point of the region lies within `VCARVE_TOL_MM` of some cut's circle,
   except where §5 says otherwise (a depth cap, a stroke narrower than the tip). In those
   places it's a warning, never silent. One exception is inherent and not reported: a bit
   with a flat tip can't reach into the last bit of a sharp corner, where the shape is
   narrower than the tip (within w/2 / sin(θ/2) of a corner of angle θ).

These two rules are what the tests check (§10). The floor of the carving is the V shape the
tool leaves; nothing else is promised about it.

## 2. The V-bit (tool data)

`ToolType::Vbit` exists but has no geometry beyond `diameter`. It gets:

- `angle`: included angle, **radians** (shown in degrees), 0 < angle < π. Optional in the
  file: None means unknown.
- `tip_diameter`: mm, 0 ≤ tip < diameter. Optional: missing means pointed (0) (decision 11).

`diameter` is the largest cutting diameter, as today. It limits the depth: the bit can't cut
deeper than (diameter − tip) / 2 / t, and above that its shank would rub.

A V-carve with a V-bit whose angle is unknown is **refused** ("tool 3 is a v-bit with no
angle; set its angle"). The angle is never guessed. Profiles with V-bits work as before and
don't need the angle. A V-carve with an endmill or drill is refused.

The same two fields go into the tool library (`tools.json`), and `use_tool` copies them.

## 3. The operation (job model)

```json
{
  "type": "vcarve",
  "name": "Lettering",
  "enabled": true,
  "select": { "layer": "TEXT" },
  "tool": 3,
  "start_depth": 0.0,
  "end_depth": 4.0,
  "step_down": 2.0,
  "feed": 1200.0,
  "plunge_feed": 400.0
}
```

- `start_depth`: where the carved surface is, mm down from the stock top (0 for the top; more
  to carve into a pocket floor that an earlier operation cut). As for other operations,
  0 ≤ start_depth < end_depth.
- `end_depth`: the deepest the tip may go, mm down from the stock top: a cap (decision 3).
  Strokes that need more are carved to it and reported (§5).
- `step_down`: optional (decision 4). Missing means one pass. When set, the carve goes down in
  passes that each remove the same amount of material (decision 12), the first no deeper than
  `step_down`, and each only along the parts of the axis that the previous pass left deeper
  than that.
- `feed`, `plunge_feed` mm/min. Moves that go down steeper than 45° use the plunge feed; all
  others use the feed (decision 10).
- No side, direction, clearance, tabs, entry, leads or start points. A V-carve cuts both
  walls at once, and its paths start at the ends of the axis, mostly at depth 0 (§4).

Selection is as for pockets: closed contours only, open ones skipped and reported, nesting
among the selected contours gives regions and their islands, crossing contours are separate
regions.

## 4. The toolpath (chipload-toolpath)

For each region:

1. **Boundary to segments.** Arcs are split into chords within a quarter of `VCARVE_TOL_MM`
   (finer than `ARC_APPROX_TOL_MM`: the centre line between chords wobbles by up to their sag,
   which pruning has to stay well inside). Points are snapped to an integer grid of
   `VORONOI_GRID_MM` (0.0001 mm) for the Voronoi input, which needs integers (decision 2).
   The chords only place the centre line: every depth comes from the distance to the real
   outline (arcs, not chords).
2. **Medial axis.** Take the Voronoi diagram of the boundary's segments, and keep the edges
   inside the region, without the edges between a segment and its own end points. Edges are
   straight (between two segments) or parabolic (between a corner point and a segment). The
   parabolic ones are split into lines within `ARC_APPROX_TOL_MM`.
3. **Pruning.** A chord-split arc has a corner at every chord, and each corner gets a short
   axis branch running to the outline: hundreds of tiny cuts that add nothing. An end branch
   hanging from junction J is removed when the cut of every point p on it lies inside J's
   cut grown by `VCARVE_TOL_MM`: |p − J| + r(p) ≤ r(J) + `VCARVE_TOL_MM`. At the corner C
   itself (r = 0) that is |C − J| ≤ r(J) + `VCARVE_TOL_MM`. Branches into real corners (a 90°
   serif corner: |C − J| = √2 · r(J)) stay. A junction remembers how much of the tolerance
   the branches removed into it used, and a later removal counts that too, so removals never
   add up to more than the tolerance. This repeats until nothing changes (decision 8).
4. **Depth.** Every point of the axis gets depth(p) of §1, capped at `end_depth` and at the
   tool's deepest. Along a straight edge between two segments, r changes linearly, so one move
   does. Other edges are split until the cut's radius at every midpoint is within half of
   `VCARVE_TOL_MM` of linear.
5. **Paths.** The axis is a graph. It is cut as continuous paths found by a greedy walk.
   Each path starts at the nearest end of the axis that has an edge left (a corner at depth
   0, so the tool goes into the material along the path instead of plunging), else at the
   nearest node with an odd number of edges left; at a junction the walk takes an edge that
   leads on rather than into a dead end, so dead ends are left for later paths to start from.
   Between paths the tool goes up to the safe height (decision 7). The order is
   deterministic: ties are broken by coordinates. Parts of a path narrower than the tip are
   left out. A later depth pass runs only where the earlier one stopped short, starting at
   that depth. **Area by area** (decision 13): each region (a letter, the dot of an i) is
   finished, every pass of its centre line and walls, before the tool moves on; the next is
   the region nearest where the tool is, and its walk starts from there.
6. **Verification.** Before the toolpath is used, every cutting move is checked against rule 1
   of §1, at its ends and at points along it, using the region's real boundary (arcs, not
   chords). A failure is an error, and no G-code is made, as for offsets.

The program is moves of X, Y and Z at once (`feed_xyz`). They never become G2/G3: arc merging
only joins moves at constant Z. So the `expected/lines` golden file is the same program.

Copies, work zero, placement and the stock fit note work as for every operation.

## 5. Warnings (not errors)

- **Capped:** "Operation 1 (Letters): contours #3, #4: too wide for a carve at most 3.000 mm
  deep (3.664 mm wide at the surface); the edges of the widest parts aren't reached (they need
  3.481 mm). Allow deeper, use a wider angle, or clear the floor." When the bit's own size is
  the limit, it says "too wide for the V-bit (12.7 mm across), which carves at most …".
- **Narrower than the tip:** where r < w/2, the stroke is narrower than the tip flat. The
  axis there is not cut, and it's reported with its length, except for the tips of corners
  (§1).
- **Open contours** skipped (as for pockets).
- The plan report says when the tip is taken as pointed because it isn't set (decision 11).

The 2D view shows the paths as for other operations. Showing the area not reached
(material-left.md) is a later step.

## 6. Checker

`chipload-check` already understands G1 moves in X, Y and Z (ramps) and checks depth, stock,
limits and rapids. Nothing changes there. It doesn't know the outline, so rule 1 is checked in
the planner (§4.6) and independently in the tests (§10), not in the checker (decision 9).

## 7. CLI

- `chipload tool add --type vbit --diameter 6.35 --angle 60 --tip 0.2` (degrees on the
  command line; stored as radians). `tool list` shows them.
- The operation is written in the project file like profiles and pockets. `plan` and `run`
  carve it, and `plan --json` shows its moves.

## 8. GUI

- The selection panel has **New V-carve** next to *New profile* and *New pocket*; the form has
  the fields of §3 (*Max depth* is `end_depth`; *Passes: in steps* turns `step_down` on).
- A project has one tool for now (TODO: a tool per operation), so there is no list of tools to
  narrow down: the V-carve uses the project's tool, and planning says when it isn't a V-bit
  or has no angle.
- Tool forms (project and library) show *Angle* and *Tip* for V-bits.
- Nothing else is new: the preview and the 3D view draw the moves as they draw others.

## 9. Format

- Jobs: **schema 10**. Operations may be `vcarve`; a tool may have `angle` and `tip_diameter`.
  Both are optional, but an older Chipload would refuse them as "unknown", so the version is
  bumped to say "newer version" instead, as we did before (decision 5). Migration v9 → v10
  changes nothing: existing V-bits get no angle (unknown), and existing operations are
  unchanged. A frozen v9 sample exists (`tests/corpus/jobs/v9/turned_part.chipload`).
- Tool library: `tools.json` version 2, the same fields, upgrade 1 → 2 changes nothing.
- New constants in `chipload_core::tolerances`: `VORONOI_GRID_MM` = 0.0001 and
  `VCARVE_TOL_MM` = `ARC_APPROX_TOL_MM` (0.01).

## 10. Tests

- **Geometry, exact cases** (`crates/chipload-toolpath/tests/vcarve.rs`):
  - A rectangle 10 × 4 with a 90° bit: one centre line at depth 2, plus four branches to the
    corners, rising to 0.
  - A ring between two circles: a circle at the mean radius, one depth, no spokes (pruning).
  - A region with a hole (widest between the two corners: 10 − √48).
  - A 60° bit with a 0.2 tip; a stadium of lines and arcs.
- **Property tests** (proptest): stars of random size and angle, and rectangles with bulged
  sides, with and without a round hole. A test-only re-implementation of rules 1 and 2 of §1
  samples the cone's circle at the surface for no gouge, and grid points of the region for
  full reach. It is written separately from the planner, so the two can disagree.
- **Boundaries:** depth cap hit, the bit's own depth limit, a neck narrower than the tip,
  step-down passes (no move cut twice at the same depth, the last at the full depth, nothing
  above the carved surface), V-bit with no angle refused, endmill refused, tip not set noted.
  Depth limits against the stock and open contours go through the checks every operation
  shares.
- **Job:** migration v9 → v10 is the identity; the angle out of range and the tip ≥ diameter
  are refused; tool library 1 → 2.
- **CLI:** `tool add` with the angle; `run` on a V-carve project passes the checker.
- **Golden:** a new synthetic DXF (`tests/corpus/dxf/vcarve_letters.dxf`) with drawn serif
  letters made of lines and arcs, written for this test (no font outlines copied). Its
  v10 project and golden G-code. Existing golden files unchanged.
- **GUI:** a V-carve added for the selection, generated, duplicated.

## 11. Decisions

Agreed 2026-10-11:

1. **Order:** this operation first, on closed contours from the drawing. Then text in the app,
   then picture tracing, each with its own spec.
2. **Medial axis:** the `boostvoronoi` crate (**BSL-1.0**, allowed since 2026-10-09; its
   dependencies are MIT/Apache). It is a Rust port of Boost.Polygon's segment Voronoi, with
   exact predicates on integer input. It is used behind our own checks (§4.6). If it fails
   on our corpus, we write our own. A short ADR comes with it.
3. **Strokes wider than the bit:** capped at the maximum depth and reported; no flat floor
   clearing yet. *Replaced 2026-10-30 by docs/specs/vcarve-wide-strokes.md: walls along the
   outline, and a floor pass.*
4. **Depth passes:** optional `step_down`.
5. **V-bit data:** the angle (required for V-carving) and an optional tip diameter on the
   tool; schema bump for projects and the tool library.

Agreed 2026-10-11 ("go ahead"), all as recommended:

6. **Depth fields:** `start_depth` (the surface carved, usually 0) and `end_depth` (the cap),
   mm down from the stock top like every operation, so carving into a pocket floor works.
   Recommended as described. Alternative: only a `max_depth` from the stock top.
7. **Between paths:** lift to the safe height (recommended: simple, and obviously safe).
   Alternative: stay down and run along an axis edge already cut, which is air at that depth.
   Fewer lifts, but more to prove; it could come later.
8. **Pruning** of branches into chord corners, by the rule in §4.3 with `VCARVE_TOL_MM`
   (0.01 mm). Recommended. Without it, a letter "O" gets hundreds of spokes.
9. **Where rule 1 (no gouge) is checked:** in the planner after building, and in an
   independent test re-implementation. The G-code checker stays unaware of outlines.
   Recommended. Alternative: teach `chipload-check` the outlines too. It would then depend on
   which shapes an operation selects, which is planning logic it must not share.
10. **Feed on steep moves:** the plunge feed when a move goes down steeper than 45°, else the
    feed. Recommended. At a 90° corner with a 60° bit, the path goes down at about 51°.
11. **Tip diameter not set:** taken as pointed (0) and said so in the plan report.
    Recommended. With a real tip of w, the carving is then up to w/2 wider on each side
    (0.1 mm for a common 0.2 mm tip). Alternative: require it for V-carving like the angle.

Agreed 2026-10-30 (the user: "add the progressive v-bit stepdown"):

12. **Passes of equal load** (the user, 2026-10-30: "each stepdown will cut the entire length of
    the v-bit"). A V groove's cross-section grows as the square of its depth (depth² ×
    tan(angle/2)), so passes of equal depth remove more and more: 1 : 3 : 5 for three. The
    passes now remove equal areas instead. For a carve `D` deep below the carved surface
    (the deepest its paths go) and a step down `s`, the number of passes is the fewest whose
    first pass is no deeper than `s`, n = ⌈(D / s)²⌉, and pass k goes to D × √(k / n), the
    last exactly `D`. 3 mm with a 1.5 mm step: 1.5, 2.12, 2.6, 3.0; with a 2 mm step: 1.73,
    2.45, 3.0. One way, no setting: `step_down` is the first pass's depth, and every later
    pass is no heavier. Projects keep their `step_down`; their passes change (the golden
    G-code with them).

Agreed 2026-10-31 (the user, after a test cut: "could it complete each area before going to
the next, there is a lot of back and forth now"):

13. **Area by area.** Passes went over every region in turn, and the walls of wide strokes
    came after all centre lines, so with several passes the tool crossed the whole carving
    again for each. Now each region gets all its passes, centre line and walls, before the
    next; regions in nearest-first order (equally near: the first in the drawing). The same
    cuts, in another order: the golden G-code changes order and where some paths start,
    and an operation after a V-carve can start elsewhere, since the V-carve ends elsewhere.
    Rapid travel in the golden files drops by up to a quarter. Chosen over finishing each
    centre-line path before the next, which would re-enter a letter.
