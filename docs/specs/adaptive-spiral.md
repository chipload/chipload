# Spec: adaptive clearing as spirals and trochoidal channels on the medial axis

Status: **approved 2026-10-11**, all decisions as recommended (§9). **§3.1–3.4 revised
2026-10-11 for stage 2** (option C of §11, chosen by the user): decisions 9–12, for review. Replaces docs/specs/adaptive-fronts.md (its
stage 1, one stepover setting, stays) and the trochoidal strategy (docs/specs/trochoidal.md).

User request (2026-10-11), after the fronts were built and measured: step back. Adaptive
should be what the medial-axis based methods give: the medial axis divides the pocket, open
areas are cleared by a continuous **morphed spiral** from the centre out to the walls,
narrow spaces by **trochoidal** loops; the engagement angle stays constant, so the tool is
never overloaded; small stepover (5–15 % of D) with deep cuts (2–3 × D); soft arc-shaped
moves. The trochoidal strategy is replaced by this. Rest machining with a smaller tool
comes later, as its own spec.

The user's goals stay: **constant chip load** (never overloaded), **fluid motion** (no sudden
changes of direction), **little recutting**.

## 1. Why not the fronts

The fronts (adaptive-fronts.md §10) cut exactly one stepover, so every pass that reaches a
wall has to end there and the tool returns through cleared material. That costs 1.7 × the
ideal length, sharp turns at every return, and up to 2 × w where a pass starts at a wall.
A spiral has none of these: every loop is closed, so the tool never meets a wall halfway.

## 2. Engagement angle

The engagement angle is the arc of the tool's edge in material. On a straight cut of radial
width ae with tool diameter D: θ = acos(1 − 2·ae/D). Stepover 0.1 × D is 37°, 0.25 × D is
60°, 0.4 × D is 78°.

- **The setting stays the stepover** (schema 13), as the user knows it. Planning turns it
  into the target angle θ₀ with the formula above.
- **The rule**: θ ≤ θ₀ everywhere, measured on the real path. On a straight cut that is the
  stepover; on a curve it isn't: going round a tight loop with material on the outside, the
  same spacing gives a larger angle. There the loops come closer.
- Where the pocket narrows the spiral cuts less than θ₀ (§3.3). Less is safe, just slower.

## 3. How the path is made

### 3.1 The core (revised for stage 2, §11)

r = tool radius, w = stepover, R = the area the tool centre may be in (the pocket shrunk by
r + clearance). The medial axis of R, with the clearance radius ρ(m) at each point (built as
for V-carving, ADR 0015; that code becomes shared).

- **The core** is the part of the medial axis that the spiral grows from. It is the medial
  axis **pruned from its leaves**: a leaf branch is shortened as long as every boundary point
  of R stays within `ROUND_ENOUGH` (1.25) × the smallest clearance on the wide part of the
  core. A square's core is its centre point; a 3 : 1 rectangle's is its centre line plus the
  inner parts of its four corner branches; an L's is the centre lines of both arms.
- **Why**: every boundary point's spoke (§3.2) runs from its nearest core point, so spokes
  differ in length by at most 1.25 ×; the spiral then cuts at least about 0.8 of the target
  angle everywhere (stage 1 measured 0.90 on a circle, where all spokes are equal).
- **Channels** are the parts of the core where the opening loops (§3.4) already reach the
  walls: there the opening is the whole cut, and nothing is left for a spiral. A slot is all
  channel; a dumbbell is two wide parts joined by a channel. This replaces the fixed
  clearance limit of decision 5 (ρ ≥ D): what is open follows from the loop size.
- Leaf branches that stay narrow to their end (a slot off a room) are never pruned away:
  their boundary is too far from the rest of the core, so the rule keeps them, and being
  narrow they are channels.

### 3.2 The morphed spiral (one wide part, no islands)

- **Start**: the core is opened first (§3.4); the spiral starts on the boundary of the
  opened band and grows out to the wall. For a core that is one point (a circle, a square)
  the opening is the helix entry and one small loop: stage 1 as built.
- **Spokes**: for each point b of the (rounded) wall, the spoke runs from b's nearest core
  point m(b), starting where it leaves the opened band, straight to b. Spokes to the nearest
  points of a medial axis never cross each other (each lies inside its own maximal circle),
  so neither do the turns. Where the core was pruned, the spokes of a corner fan out from the
  end of the remaining branch.
- **One continuous spiral**: turn t is t of the way out along every spoke (blended as in
  stage 1, so inner turns follow the band's shape and outer turns the wall's); t grows
  steadily as the tool goes round. No links, no lifts; always the same turning direction
  (climb by default).
- **Spacing**: as built in stage 1: each turn as far from the last as the engagement angle
  allows in its tightest place, never more than w; spokes are parametrised by the wall's
  length instead of the angle from a centre, since the area need not be star-shaped.
- **Smooth**: points fitted with arcs within `ARC_APPROX_TOL_MM`, as in stage 1.
- Where a wide part meets a channel, its boundary there is the channel's opened band, which
  is already cut: the spiral's spokes end on it.

### 3.3 What the spiral cuts

Between turns the radial distance varies with the spoke: longest spoke / N at most (≤ w),
at least 0.8 of that by the core rule. So θ ≤ θ₀ everywhere after the spacing correction,
and the cut stays close to it.

### 3.4 The opening: trochoidal loops along the core

- The tool goes down at the core's widest point (the largest circle, with the operation's
  entry) and walks the core depth first, making **trochoidal loops** along it: circles (as
  arcs) moving on along the core between loops.
- **Loop radius**: the tool radius r, so the opened band is 2 × D wide (tool edge to tool
  edge); smaller where the clearance is smaller, so the loop just reaches the wall's band
  (ρ(m) less a margin). Where the loops reach the wall, that stretch is a channel and is
  done.
- **Step per loop**: as far as the engagement angle allows on a loop of that radius (the
  stage 1 formula with the loop's bend), at most w. The loops are within θ₀ like the spiral.
- **Branches**: at a fork the walk goes into one branch to its end and comes back through the
  band it opened (cutting nothing), then takes the next.
- **Too narrow for loops**: where the loop radius would be under a tenth of D, the tool
  follows the core as a full-width slot, with a note (decision 7, unchanged).
- **Cost**: loops are slower than a spiral per area cleared (about 1.5 × the ideal length in
  the band), but the band is a small part of a wide pocket. Measured and reported (§5).

### 3.5 Order, links and the wall pass

- The medial axis is a tree once islands are dealt with (§3.6). The root is the largest
  circle. Open areas and channels are cut depth first from the root, so every part starts
  next to material already cleared, and links run through cleared material at depth.
- **Links** between parts: a smooth arc through cleared material; if there is none, retract
  (as `keep_tool_down` decides today).
- **Wall pass**: the spiral's last turn is the wall of its area; channels' loops reach to
  the band. The ordinary wall pass at each depth then finishes all walls and islands, as
  today: it takes the channels' band and the cusps.

### 3.6 Islands (stage 3)

An island makes a loop in the medial axis. Each loop is cut at its narrowest point (the
smallest circle), which makes the medial axis a tree. Where that cut is, R is split along
the spokes there, so each open area is again free of islands; the split line is cut as a
seam where two spirals meet, both reaching it. Details in the stage 3 review, after the
measurements of stage 1.

## 4. Format and defaults

### 4.1 Trochoidal goes (schema 14)

- `{"type": "trochoidal", "loop_radius", "step"}` is no longer a strategy.
- **Migration v13 → v14**: trochoidal becomes adaptive with `stepover := step / D` (clamped
  to the adaptive range, at most 0.5). Trochoidal's thin bite per loop is about `step`
  wide, so the tool is loaded about as before. **The path changes** (that is the point of
  replacing it): decision 2 asks for the user's yes, as CLAUDE.md requires when a migration
  can't keep machining behaviour exactly.
- Frozen v13 sample with a trochoidal pocket, and a golden test. The v7_trochoidal golden
  file changes (explained in the commit).

### 4.2 Flute length (same schema 14; tools.json 3)

- Tools get an optional `flute_length` (mm, > 0) in projects and the tool library. Tools are
  read strictly (unknown fields refused), so an older Chipload would misread a newer file:
  hence the bumps, as for the V-bit angle (schema 10, tools.json 2). Migrations add `null`.
- A pocket whose step-down is deeper than the tool's flute length gets a warning (not an
  error: the user may know better). No flute length, no check.

### 4.3 Defaults when a pocket is switched to adaptive

- Stepover 0.1 × D and step-down 2 × D, filled in visibly (GUI) or by `chipload` when the
  strategy is set (as trochoidal filled loop radius and step). Rings keep 0.4. The user can
  change both.
- This reverses the earlier request to default adaptive to 0.4 × D (2026-10-11): at full
  depth 0.4 × D overloads the tool; decision 3.

## 5. What "correct" means, and what is measured

Safety, checked, errors otherwise (as for every pocket): no gouge, links at depth only
through cleared material, the material-left check on the real moves, the independent
checker, deterministic output.

Quality, measured on the real moves with the material grid (the angle is the arc of the
tool's circle, one cell outside it, over uncut cells), in 1 mm averages:

| Measure | Target |
|---|---|
| Engagement angle | ≤ 1.1 × θ₀ on 99 % of the cutting length, never above 1.25 × θ₀ (grid noise) |
| In open areas | median ≥ 0.6 × θ₀ (the spiral isn't wasting turns) |
| Reversals > 135° while cutting | none |
| Retracts | none within an open area or channel; joins only where cleared material can't be found |
| Length | ≤ 1.3 × the ideal (area / stepover) on the request's two-island pocket (after stage 3) |
| Planning | 200 × 150 mm pocket under 1 s (release) |

If a target is missed, the measured value is reported and the user decides (as before).

## 6. Out of scope

Rest machining with a smaller tool (next spec, also by the medial axis), feed changes with
engagement, lifting on links, spirals that cross islands without splitting.

## 7. Tests

- Circle, square, rectangle 3:1, L shape (two open areas and a corner), a slot, a dumbbell
  (two rooms joined by a channel), and (stage 3) the request's two-island pocket.
- Each: no gouge, no uncut warning, the targets of §5, deterministic.
- Property tests: random pockets (and, from stage 3, islands), random stepover 0.05–0.5: no
  gouge, no uncut warning, angle never above 1.25 × θ₀, no reversals while cutting.
- Migration: v13 trochoidal sample → adaptive with stepover = step / D; flute length null.
- Golden: adaptive files and v7_trochoidal change (each explained); others unchanged.

## 8. Staging

1. **Spiral** for pockets without islands that are one open area (no channels yet): shared
   medial axis, spokes, the spiral, angle-based spacing, arcs. Replaces the fronts for those
   pockets; others keep the fronts until their stage. Measurements reported.
2. **Core and opening** (§3.1–3.4 as revised): the pruned core, trochoidal opening along it,
   spirals from the opened band, channels, order and links. Square, rectangle 3 : 1, L,
   dumbbell and slot tests.
3. **Islands**: cutting the medial axis's loops, splitting R, seams.
4. **Format**: schema 14 (trochoidal migrated, flute length), tools.json 3, the defaults,
   the flute-length warning; the trochoidal strategy and the fronts removed.
5. **GUI**: strategy list without trochoidal, *Stepover (cut width)* with the angle shown
   beside it (e.g. "≈ 37°"), flute length in the tool sections and library.

## 9. Decisions

1. **Approach**: medial-axis division, a morphed spiral per open area, trochoidal loops along
   the centre line in channels (§3); our own design, written here (no code from others; the
   ideas are in published papers). **Recommended.**
2. **Trochoidal migrates to adaptive** with stepover = step / D, a changed path at about the
   same load (§4.1). The alternative is keeping trochoidal readable but hidden, which keeps
   its code alive for old files. **Recommended: migrate.**
3. **Defaults** for adaptive: stepover 0.1 × D, step-down 2 × D (§4.3), reversing the earlier
   0.4. **Recommended.**
4. **Flute length**: optional tool field, schema 14 and tools.json 3, a warning when the
   step-down is deeper (§4.2). **Recommended.**
5. **Open area or channel**: a medial-axis point is in an open area when its clearance
   radius (tool centre to wall) is at least D (the pocket there is at least 3 × D wide,
   room for a spiral of several turns). **Recommended: D**, a named constant, revisited
   after stage 2's measurements. *Replaced by decision 10 (revision of §3).*
6. **Targets** (§5): to be confirmed by measuring; if one is wrong I report it and ask.
   **Recommended.**
7. **Too narrow for loops**: follow the centre line as a full-width slot with a note (as
   the existing narrow-slot warning). The alternative is refusing the pocket. **Recommended:
   slot with a note.**
8. **Staging** (§8), each stage on main when its tests pass, the fronts kept for what the
   spiral doesn't cover yet. **Recommended.**

Revision of §3 for stage 2 (§11), for review:

9. **The core**: the medial axis pruned from its leaves while every wall point stays within
   1.25 × the core's smallest wide clearance (§3.1), the same limit as stage 1's roundness
   gate. **Recommended.** A smaller limit cuts closer to the target but lengthens the
   opening.
10. **Channels** are where the opening loops reach the walls (§3.1), replacing decision 5.
    **Recommended.**
11. **Opening loop radius**: the tool radius (band 2 × D wide), smaller where the pocket is
    narrower (§3.4). **Recommended.** Larger loops open a wider band at the same load but
    are slower per length.
12. **Fallback in stage 2**: a part the new path can't take (islands until stage 3, a wall
    whose spokes can't be built) keeps the fronts, as in stage 1, until stage 4 removes
    them. **Recommended.**

## 10. As built

### Stage 1 (the spiral, round areas)

`chipload_toolpath::spiral`, called per part of the wall-pass area from
`pocket::adaptive_rings`; parts it doesn't take go to the fronts (adaptive-fronts.md) as
before.

- **Which areas**: no islands, star-shaped from the largest circle's centre once the corners
  are rounded, and **round enough**: the farthest boundary point at most 1.25 × as far as
  the nearest (`ROUND_ENOUGH`). One spiral has as many turns in every direction, so the
  farthest direction sets them and nearer ones get thinner cuts. Measured median angle
  (share of the target): circle 0.90–0.93, ellipse 1.2 : 1 0.73–0.78, hexagon 0.49–0.73,
  square 0.42–0.66, 3 : 1 rectangle about 0.35, the last two too wasteful, so they stay with
  the fronts until stage 2 splits them. Splitting is where the open questions are (§11).
- **Turns** (§3.2, simplified to what round areas need): the spokes are straight lines from
  the start (the area is star-shaped from it); turn t is t of the way out along a blend of
  the inner circle (radius: the nearest boundary point) and the boundary, the boundary's
  share growing with t, so inner turns are round.
- **Corners** are rounded to (stepover / (√2 − 1)): a 90° corner then leaves the wall pass
  about one stepover. The last turn runs on that rounded boundary, pulled in by the arc
  tolerance (its points come from chords and fitted arcs).
- **Spacing**: each turn is as far from the last as the engagement angle allows in its
  tightest direction (§2's formula with the turn's local bend), never more than the
  stepover and never less than a tenth of it (the very first turns round the plunge would
  otherwise need ever smaller steps). The first turn is the opening: an entry, not
  measured.
- **Arcs**: points (at most 0.25 mm apart) fitted greedily with arcs through three of them
  within half the arc tolerance, written as tagged chords (G2/G3).
- **Angle measure**: on its own grid of half the adaptive cell (0.1 mm for a 6 mm tool), at
  the model's cutting radius; with 0.2 mm cells the reading varied by about 30 % where the
  cut was steady.

Measured on a Ø60 circle (6 mm tool, 1 mm averages): median 0.90 / 0.92 / 0.93 of the target
at stepover 0.1 / 0.25 / 0.4, at most 1.1 × on 99.9 % or more, worst 1.16 / 1.09 / 1.06; no
turn sharper than 45° after the first move out; one path plus the wall pass. Golden:
v7_adaptive and v12_adaptive change (their round hole is now a spiral).

## 11. Open: splitting the rest (before stage 2)

Measured in stage 1: the spec's split by clearance radius (§3.1) would make a square or a
3 : 1 rectangle one open area, which one spiral clears poorly (above). Clearing them well
needs either rounder pieces (several spirals, which raises where each starts and how they
meet) or a different treatment of corners and long sides.

Proposed 2026-10-11: A, several spirals in round cells (a plunge per cell, seams cut
twice); B, a spiral in the round middle and trochoidal loops into corners and ends (slow in
wide ends); C, a spiral grown from a pruned medial axis that is first opened with
trochoidal loops (one plunge, no seams, narrow channels done by the opening). **The user
chose C**: §3.1–3.4 revised, decisions 9–12.
