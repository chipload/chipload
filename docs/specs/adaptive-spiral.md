# Spec: adaptive clearing as spirals and trochoidal channels on the medial axis

Status: **draft 2026-10-11**, for review. Replaces docs/specs/adaptive-fronts.md (its
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

### 3.1 Division by the medial axis

r = tool radius, R = the area the tool centre may be in (the pocket shrunk by r +
clearance). The medial axis of R, with the clearance radius ρ(m) at each point (built as
for V-carving, ADR 0015; that code becomes shared).

- **Open areas**: the parts of R round medial-axis points with ρ ≥ ρ_open (decision 5). Each
  gets one spiral.
- **Channels**: the rest, narrower than an open area: the medial-axis branches between and
  beyond the open areas. Each gets trochoidal loops along its centre line (§3.4).
- A pocket with no open area (a slot) is all channel. A pocket that is one open area is
  one spiral.
- Small branches that only reach into corners (the medial axis runs into every convex
  corner of R) are not channels: their material is taken by the last spiral loops or the
  wall pass. The cut-off is when the branch adds less than the stepover (as V-carve prunes
  by its tolerance).

### 3.2 The morphed spiral (one open area, no islands)

- **Start** at the centre of the area's largest circle, with the operation's entry (a helix
  fits best there).
- **Loops** morph from a small circle round the start to the area's boundary: for each point
  b of the boundary, a **spoke** runs from the start along the medial axis to b's nearest
  medial-axis point and on to b. Loop k is made of the points k/N along every spoke. The
  last loop is the boundary itself.
- **One continuous spiral**: instead of separate loops, the fraction grows steadily as the
  tool goes round, from k/N to (k+1)/N in one turn. No links, no lifts; the tool always
  turns the same way (climb by default).
- **Spacing**: N, the number of turns, is set by the longest spoke so that no two turns are
  farther apart than the stepover; then shortened where the angle rule (§2) needs it
  (tight turns near the start). Along short spokes turns lie closer: the spiral cuts less
  there. Splitting the pocket (§3.1) is what keeps that waste small.
- **Smooth**: the spiral is built as points and fitted with arcs (tangent-continuous) within
  `ARC_APPROX_TOL_MM`, so the controller sees G2/G3 that flow into each other.
- Where the area meets a channel, the boundary is not a wall but the channel's mouth; the
  spiral still ends on it, and the channel takes over from there.

### 3.3 What the spiral cuts

Between turns the radial distance varies with the spoke: longest spoke / N at most (≤
stepover), less elsewhere. So θ ≤ θ₀ on straight parts and, after the spacing correction,
on curves too. Where turns lie much closer than the stepover, the spiral recuts nothing,
it just takes thinner chips.

### 3.4 Channels: trochoidal along the centre line

- Loops along the medial-axis centre line, each a circle (as arcs) whose radius follows the
  channel: ρ(m) less a small margin, so a loop reaches the channel's walls to the wall pass's
  band.
- **Step per loop**: the stepover. A loop moving on by p cuts at most about p sideways, so
  the angle rule holds; the measured angle decides (§5).
- Where a channel narrows below the trochoidal minimum (loop radius under a tenth of D, say),
  it is a plain slot: the tool follows the centre line at full width, with a note (as
  today's narrow-slot warning). Decision 7.
- Channels are cut from the open area they leave, outwards to their ends.

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
2. **Channels**: division into open areas and channels, trochoidal along centre lines,
   order and links. Dumbbell and slot tests.
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
   after stage 2's measurements.
6. **Targets** (§5): to be confirmed by measuring; if one is wrong I report it and ask.
   **Recommended.**
7. **Too narrow for loops**: follow the centre line as a full-width slot with a note (as
   the existing narrow-slot warning). The alternative is refusing the pocket. **Recommended:
   slot with a note.**
8. **Staging** (§8), each stage on main when its tests pass, the fronts kept for what the
   spiral doesn't cover yet. **Recommended.**
