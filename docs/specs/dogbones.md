# Spec: dogbone and T-bone corners

Status: **approved 2026-10-13**, all decisions as recommended (§9).
Built 2026-10-13. As built:
- §3: `C′` is the corner of the walls moved `c` into the part, which is `c / sin(θ/2)`
  along the bisector, not `c`. Every wall then holds the clearance up to the corner.
- §3: a leg at the vertex where the ring starts goes at the ring's end, so the ring still
  leaves its start along the wall (leads and spirals start there).
- §5: the ring's vertex for a corner is the nearest one on the tool's side within 1.25 ×
  the straight-wall distance that turns at least half as much as the corner. A wall that
  is an arc moves the vertex a little: 5 % for a line meeting a half circle at 90°.
- §4, pockets: crossing boundaries are separate pieces, each cleared on its own. A corner
  of one that lies inside another boundary of the same pocket is in waste the other
  clears, so it gets no dogbone. The leg check (§6) doesn't count the pocket's other
  boundaries, because both sides of them are waste near a corner. Islands and every other
  contour still count. An island is a contour wholly inside an odd number of the others,
  so crossing contours don't count as islands.
- §6: after each leg is added, the ring must reach the tip from its vertex and leave back
  to it, or planning stops. An early version spliced a start-vertex leg wrongly into a
  pocket ring that doesn't repeat its start, which cut along a wall; the tests caught it.
- §6: the material-left display is geometric, not taken from the moves. Pieces that lie
  wholly within the tool radius of a leg are left out of it.
- Robustness sweep: dogbones and T-bones on every profile and pocket of the corpus
  projects. 56 of 60 runs pass the checker. The other four are all `messy_2_lower`,
  refused by decision 6: a pocket wall runs 0.5 mm from another contour, and a pocket runs
  past the part's outline.

Guide: docs/book/src/operations/profile.md, docs/book/src/operations/pocket.md.

User request (2026-10-13): commercial-CAM features 1–5 and 7; this is 4. A round tool can't
cut a sharp inside corner: it leaves a fillet of its own radius. A square tenon then
doesn't go into a mortise cut with a profile or pocket, and slot-together parts don't fit.
A **dogbone** or **T-bone** lets the tool reach into each such corner, so a square part
fits all the way in.

- **Dogbone:** at the corner the tool goes in along the corner's bisector until its edge
  touches the corner, then back out. It leaves a round notch that cuts a little into both
  walls.
- **T-bone:** the same, but the tool moves along one wall instead. The notch then cuts into
  only the other wall, so it is out of sight along the wall that shows.

## 1. Which corners

A corner of a drawn contour that the operation cuts gets a dogbone when all of these hold:

- the tool can't reach it: the corner is sharp **on the tool's side** (the waste side),
  with an angle `θ` there less than 180°. Examples are every corner of a rectangular hole cut
  inside, the corners of a notch in an outline cut outside, and the corners of a pocket's
  wall;
- `θ ≤ max_angle` (default 100°, decision 3), so a 90° corner drawn slightly off still
  gets one, and an obtuse corner, such as a hexagon's, doesn't;
- the two edges meet at a real corner, not a tangent join (the turn between their tangents
  is more than the angle tolerance). Edges may be lines or arcs; the angle and the
  directions come from their tangents at the corner.

The corners on the part's side (convex from the tool's side), such as an outline's outer
corners, are cut round already and never get one.

## 2. Format (no version bump, decision 7)

One optional field on profiles and pockets. Missing or `null` means none, today's
behaviour:

```json
"corners": {"type": "dogbone", "max_angle": 1.7453}
"corners": {"type": "tbone", "max_angle": 1.7453}
```

- `max_angle`: radians, more than 0 and less than 180°.
- A profile cut "on" the line has no waste side, so it is refused there: "a profile on the
  line has no inside corners to clear".
- Older versions refuse the unknown field, which is the safe outcome. Files written by this
  version always write the field, as `null` when it is unused.

## 3. The move

For a corner `C`, the tool ring has a sharp vertex `V`, where the tool stops in the corner
today. `r` is the tool radius and `c` the operation's clearance. `C′` is the corner moved
`c` along its bisector into the part, so a clearance for a fit holds in the corner too (it
is `C` when `c` is 0).

- **Dogbone:** the tip `T` is the point on the segment from `C′` to `V` at distance `r`
  from `C′`. The ring goes `… → V → T → V → …`.
- **T-bone:** the tip `T` is on the line through `V` parallel to the **longer** of the two
  walls (its tangent at the corner), toward the corner, at distance `r` from `C′`. The
  notch is cut into the shorter wall (decision 4). For walls of equal length (within the
  chaining tolerance), the wall the ring comes in along counts as the longer.
- For a 90° corner and `c = 0`, a dogbone goes in `0.414 · r` and a T-bone goes in `r`.

The two moves are part of the ring, so everything the ring does applies to them. They are
cut at every depth pass and every lap of a spiral, at the ring's feed. On a ramp they
descend as the spiral does.

## 4. Where it applies

- **Profiles** (inside and outside): every ring of the operation, including the roughing
  rings of a finishing pass and the finishing and skin laps
  (docs/specs/finish-and-skin.md). Each reaches its own clearance: the roughing rings stop
  `stock` short of the corner.
- **Pockets:** the wall rings only, at the wall and around islands. With trochoidal and
  adaptive clearing that is their wall pass. The inner rings don't touch the walls.
- **Tabs** are placed after the dogbones are added. They already slide off corners to a
  straight stretch (docs/specs/tabs.md), and dogbone legs are corners too, so no tab sits on
  a dogbone.
- **Start points and leads** work as today: a ring that starts at a corner vertex goes to
  the dogbone first.

## 5. Corners the tool can't reach

- If the ring has no sharp vertex near `C`, there is nothing to extend. This happens when a
  notch is narrower than the tool, or the offset rounded the corner away. Such a corner is
  skipped with a note: "corner at (12.000, 40.000): the tool can't reach into it; no
  dogbone". A vertex counts as near when it lies within `r / sin(θ/2) + chaining tolerance`
  of `C′`, along the corner's bisector within the arc tolerance.
- A corner sharper than about 30° sends the tool far in (`r / sin(θ/2) − r`, almost 3 · r
  at 30°). There is no extra rule, because `max_angle` already chooses the corners. The
  setup sheet and the preview show the result.

## 6. Checks

- **Offset verification** (CLAUDE.md safety rules) stays as it is for the ring without the
  legs. The legs gouge the part on purpose, so each leg is checked on its own:
  - at the tip, the tool is `r` from `C′`, within the chaining tolerance;
  - the leg's tool circles come no closer than `r` to every segment of every contour of the
    drawing, except the two walls that meet at `C`. A leg that would cut into another
    contour (a thin wall next to the corner) is refused with an error, which names the
    corner.
  - In a pocket, a contour the pocket doesn't cut counts only where it lies outside the
    pocket's area: inside it, the pocket clears it anyway (2026-10-26, a user's pocket that
    the part's outline ran across near a corner). The pocket's own contours still count
    everywhere.
  - A contour that lies on the operation's own line (a shared edge, within the arc
    tolerance) doesn't count there: every bone cuts past its wall, and a line on the wall is
    no thin web. Later the same day (the user: "it still complains when there are profiles
    beneath it"), per contour: a contour that touches or crosses the operation's lines, or
    runs into a pocket's area, anywhere within the leg tool's reach doesn't count for that
    leg at all, since the operation has cut through it there and no web is left. A contour
    near the wall that never meets it still counts (the thin-web tests). The same day, on the user's `shared_edges.dxf`: every rectangle sharing a wall
    with a slot or the outline generates; what is still refused (a step of the pocket's
    own, a line leaving the wall 3.2 mm from the corner) goes through with a corner choice
    (docs/specs/dogbone-per-corner.md).
- The material-left display uses the moves. It shows the corners cleared.
- `chipload-check` is unchanged: the legs are ordinary feed moves.

## 7. GUI, CLI and the setup sheet

- **GUI:** a "Corners" row in the Cut section of profiles and pockets: "round (as the tool
  leaves them)", "dogbones" or "T-bones", and the max angle in degrees. It is disabled for a
  profile "on" the line.
- **CLI:** through the project file and `run`, `plan`, `sheet`, `check`.
- **Setup sheet:** "dogbones at corners up to 100°" (or "T-bones …") in the operation's
  kind column.

## 8. Tests

- **Unit:**
  - a rectangle cut inside: four dogbones, each tip `r` from its corner, on the bisector;
  - the same with T-bones: along the longer walls, the notch in the short walls;
  - a square, with equal walls;
  - an outline with a notch cut outside: dogbones only in the notch's two inside corners;
  - a corner at `max_angle` exactly (included), and just over it (not included);
  - a line and arc corner;
  - a corner with a positive and a negative clearance;
  - a notch narrower than the tool (skipped, with a note);
  - a leg that would cut into a neighbouring contour (refused).
- **With the rest:**
  - tabs never on a leg;
  - spiral laps cut the legs while they go down;
  - with a finishing pass, the roughing tips are `stock` short of the corner;
  - a pocket's wall ring and an island's corners get them, the inner rings don't;
  - copies.
- **Golden:** a new `v18_dogbones`: a mortise pocket with dogbones, a slot cut inside with
  T-bones, and an outline with a notch cut outside, with tabs. Its `lines/` twin and a setup
  sheet golden. Every existing golden is unchanged (no project has `corners`).
- **Proptest:** random rectangles and rotations, inside and outside. Every tip is `r` from
  its corner, and the cut never reaches any other contour.

## 9. Decisions

1. **One optional `corners` field** on profiles and pockets, with dogbone or T-bone for every
   qualifying corner of the operation. **Recommended**. Alternative, or later: a choice per
   corner, picked on the canvas.
2. **Profiles and pockets both.** **Recommended**: a mortise is usually a pocket, and a slot
   through the sheet is usually a profile.
3. **Corners up to `max_angle`, default 100°.** **Recommended**: it catches 90° corners even
   when they are drawn slightly off, and leaves obtuse ones round.
4. **A T-bone runs along the longer wall**, so the notch is in the shorter wall (for a
   mortise or slot, its end). **Recommended**. Per-corner choice can come with decision 1's
   alternative.
5. **The tool reaches the corner moved by the clearance**, so a fit allowance holds in the
   corner as on the walls. **Recommended**.
6. **A leg that would cut into another contour is an error, not a skip.** **Recommended**: a
   missing dogbone makes a part that doesn't fit, so it shouldn't pass quietly. The user can
   lower `max_angle` or change the drawing.
7. **No version bump:** one optional field that defaults to today's behaviour.
   **Recommended**, as for finish and skin.
