# Spec: adaptive clearing by fronts, divided by the medial axis

Status: **approved 2026-10-11**, all decisions as recommended (§9). **Superseded** the same
day by docs/specs/adaptive-spiral.md (user: step back to medial-axis spirals); its stage 1,
the one stepover setting (schema 13), stays. Replaces the path
generation of docs/specs/adaptive.md §3 (the grid steering). Its verification, wall pass and
tests stay. Stages 1 (the format) and 3 (the fronts) are built; §10 says how and what was
measured, including the targets of §4 that are not met yet.

User request (2026-10-11, with a screenshot of a rectangle pocket with two round islands):
the adaptive path "doesn't look very efficient", and the Voronoi diagram should be used to
divide the area more cleverly. The user's goals for adaptive:

- **Constant chip load**: the tool cuts the same width all the time.
- **Fluid motion**: few sudden changes of direction, so the controller can keep its speed.
- **Little recutting** of areas that are already cut.
- **One setting**: the pocket's stepover (default 0.4 × D, as for rings). No separate `load`.

## 1. What's wrong today (measured)

The pocket from the screenshot is a 120 × 80 mm rectangle with two Ø24 mm islands, cut with
a 6 mm tool to 2 mm depth (one level). The pocket's area is about 8 700 mm², so at a cut
width w the clearing needs at least 8 700 / w mm of cutting.

| Strategy | Feed (XY) | Ideal | Rapid (XY) | Plunges | Turns > 90° | Reversals > 135° | Segments < 0.5 mm |
|---|---|---|---|---|---|---|---|
| Rings, stepover 0.4 | 3 229 mm | 3 620 mm | 716 mm | 17 | 32 | 17 | 0 |
| Adaptive, load 0.1 | 15 564 mm | 14 500 mm | 552 mm | 18 | 1 627 | 903 | 19 438 |
| Adaptive, load 0.2 | 7 306 mm | 7 250 mm | 499 mm | 18 | 1 137 | 353 | 9 408 |
| Adaptive, load 0.4 | 5 084 mm | 3 620 mm | 457 mm | 22 | 2 427 | 1 403 | 7 166 |

(The rings' feed is below the ideal because the wall pass is not part of the ideal. In the
table, "plunges" counts the straight plunges, one per retract.)

The grid steering (adaptive.md §3) keeps the cut width well, but:

- It decides one 0.2 mm step at a time, looking only a few millimetres ahead. Along a wall
  it zig-zags: it turns into the material, then away, then back. That gives the 900–1 400
  reversals. Every reversal is a near stop for the controller.
- When the material nearby is gone, it seeks or jumps to the *nearest* uncut cell. Nothing
  plans the order, so it leaves pockets of material behind and comes back for them through
  the part (the long links, the clusters of re-entries in corners).
- At 0.4 it cuts 40 % more than needed: much of the time it moves through cleared material.
- The output is thousands of tiny G1 segments, which slows look-ahead on GRBL.

Tuning the steering would not fix this. The problem is that nothing plans the path as a whole.

## 2. Idea

Clear the pocket the way a front spreads out from a starting point, with each **front**
exactly one stepover beyond the previous one. Use the **medial axis** (the Voronoi diagram
of the walls, already built for V-carving, ADR 0015) for three jobs:

1. **Where to start**: at the centre of the largest circle that fits, where a helix fits best.
2. **How to divide the area**: into **open areas** (around a medial-axis point with a large
   circle), which are cleared by a spiral, and **channels** (the branches between them),
   which are cleared by fronts that cross them from wall to wall.
3. **In what order**: the medial axis is a tree (once loops round islands are cut, §3.2).
   Clearing follows it depth first. Each branch is finished before the next one starts, so
   the tool never goes back across the part.

The fronts are built with exact offsets (cavalier_contours, as the rings are), so they
are arcs and lines, not grid steps. The grid stays, but only to **measure** engagement and
leftovers.

## 3. How the path is made

r = tool radius, D = 2r, w = stepover × D (the cut width). **R** is the area where the tool
centre may go during clearing: the pocket shrunk by r + clearance + w. The band of width w
left along the walls is taken by the ordinary wall pass afterwards, as today.

### 3.1 Start

The start point is the medial-axis vertex of R with the largest circle (radius ρ₀). The
tool enters there using the operation's entry (plunge, ramp or helix). The first cleared
area K₀ is the disc of radius min(ρ₀, w) round it, cut as a circle.

If R is empty (the pocket is narrower than 2(r + w)), there are only wall passes, with a
note, as today.

### 3.2 The tree

- The medial axis of R, with the circle radius at each point (as in vcarve.rs).
- Branches whose circles shrink to nothing within w of a wall corner are **pruned**. They
  only reach into corners, and the fronts fill those corners anyway.
- Islands make loops in the medial axis. Each loop is **cut at its narrowest point** (the
  smallest circle). Fronts going round the island on both sides meet there last (§3.4).
- The result is a tree with its root at the start. A point of R belongs to the branch of
  its nearest medial-axis point. That is the division of the area.
- An **open area** is the part of R round a tree vertex whose circle is wider than 2w. It
  can hold at least one full spiral turn.

### 3.3 Fronts

Kₖ is what has been cleared (in tool-centre terms) after pass k. The next area is

    Kₖ₊₁ = (Kₖ grown by w) ∩ R      (only the pieces that overlap Kₖ)

Where the pocket pinches, two separate parts of R can be closer than w, and growing would
jump across. Keeping only the pieces that overlap Kₖ stops that. A part of R that the
fronts can't reach is cleared on its own, from its own largest circle, and linked as a
branch (§3.5). Each point of the new band is at most w from what is already cut, so each
pass cuts a width of at most w, and about w where the front is straight or convex.

- **Closed fronts** (Kₖ₊₁ doesn't touch R's edge yet): this is an open area. The passes are
  joined into a **spiral**: the last part of each loop blends into the next one over one
  turn. There are no links and no reversals, and the tool always turns the same way.
- **Open fronts** (the front meets R's edge): the pass follows the outer edge of the new
  band, Kₖ₊₁ minus Kₖ. That edge is the free front plus the short stretches along R's edge
  at its ends, so nothing is skipped where the front meets a wall at a shallow angle. It is
  cut in the operation's direction (climb or conventional). Then a **return** (§3.5) takes
  the tool back to the start of the next pass.
- **Splits**: when the front breaks into several pieces (one per branch), the pieces are
  queued. The branch the tool is in is finished first, then the nearest waiting piece.
- **Joins** (behind an island, where the two sides meet): the two fronts merge into one.
  The narrow notch where they meet is cut once, in the pass that closes it.

### 3.4 Smooth corners

Growing an area keeps its notches sharp. Where two fronts meet, the next front has a sharp
inward corner, which means a stop and a short burst of extra engagement. Each pass's inward
corners are rounded with an arc of radius at least **ρ_min** (decision 5). The rounding
stays on the **cleared** side, so it cuts less, never more. Kₖ₊₁ is then taken from the real
rounded pass (what the tool really swept), so the next front picks up the bit the rounding
skipped.

### 3.5 Returns and links

- **Return** (end of one open pass to the start of the next): a smooth path through Kₖ₊₁
  (cleared material): a tangent arc out of the pass, then a straight line or arc, then a
  tangent arc into the next pass. It never turns tighter than ρ_min, and it is checked to
  stay in cleared material at least r from any uncut cell. Returns are at the cutting feed,
  at depth.
- **Between branches**: through cleared material if that's short and safe, otherwise
  retract, rapid and plunge into cleared material. This follows `keep_tool_down`, as now.

The returns are the only recutting: the tool travels through cleared material. In channels
the return is about as long as the front (crossing back), and that is the price of cutting
every pass in the same direction. In open areas, the spiral has no returns.

### 3.6 Output

- Every pass and return is a cavalier polyline with arcs: output as G2/G3 with the existing
  arc validation (docs/specs/arc-output.md), or as lines for a post with `arcs = false`.
- Each depth level repeats the same path, as all pockets do today.
- The wall pass at each depth then finishes walls and islands, as today.

## 4. Correct and efficient: what is checked

Safety is checked exactly as for every pocket, and failures are errors:
- No gouge: every move keeps r (less clearance) from walls and islands.
- Returns and links at depth only go through cleared material (checked against the grid).
- Nothing silently left: the material-left check runs on the real moves.
- The independent checker passes. The output is deterministic.

Quality is measured by tests on the screenshot pocket, the corpus pockets and random pockets.
Targets at stepover 0.4 (to be confirmed by the first build, decision 6):

| Measure | Target | Today (screenshot pocket) |
|---|---|---|
| Engagement (from the grid) | ≥ 95 % of cutting length within ±20 % of w, never above 1.5 w | median ok, spikes up to 2 w |
| Reversals > 135° while cutting | 0 | 1 403 |
| Turns tighter than ρ_min | 0 (except the entry) | thousands |
| Cutting + returns | ≤ 1.6 × ideal | 1.4 × ideal, but mostly zig-zag |
| Plunges per level | 1 per part of R (+1 per far branch) | 22 |
| Segments | about 100 × fewer (arcs) | 8 700 |
| Planning time, 200 × 150 mm | < 1 s release | 1.35 s |

## 5. Format: stepover replaces load (schema 13)

- `{"type": "adaptive", "load": L}` becomes `{"type": "adaptive"}`. The pocket's existing
  `stepover` is the cut width for adaptive too.
- **Migration v12 → v13**: `stepover := load`. The cut width stays what the user chose. The
  stepover the file had before was unused by adaptive, so nothing is lost. Without this, an
  old project with load 0.1 would suddenly cut 0.4: 4× the engagement at a full-depth step
  down. That could break a tool.
- Validation: adaptive needs 0 < stepover ≤ 0.5 (as `load` did). Rings keep ≤ 1.
- New adaptive pockets (GUI, CLI) keep the pocket's stepover, which defaults to 0.4.
- Frozen v12 sample with load 0.1, and a golden test.
- **Golden changes**: the adaptive golden files change (new algorithm), which the commit
  explains. All other golden files stay the same.

## 6. Out of scope

- Feed changes with engagement (chip thinning), and a separate faster feed for returns.
- Lifting the tool slightly on returns.
- Rest machining with a smaller tool.
- Adaptive profiles (outside walls).

## 7. Tests

- The screenshot pocket (it goes in the corpus as a project): every target in §4.
- Square, L shape, a pocket with one island, a narrow channel (R a thin strip), a pocket
  barely wider than the wall pass (R empty), and a T shape (a split).
- Property tests (random pockets with islands, random stepover 0.1–0.5): no gouge, no uncut
  warning, no reversals while cutting, engagement never above 1.5 w.
- Determinism: the same input twice gives the same moves.
- Migration: the v12 sample loads as stepover 0.1. Golden G-code for the new path.
- The timing guard (200 × 150 mm).

## 8. Staging

1. **Format**: schema 13 with the migration. The *current* algorithm reads the stepover.
   Golden output is unchanged (stepover = load).
2. **Measure**: the screenshot pocket in the corpus, and the §4 measurements as test helpers,
   asserting today's numbers. This is the baseline.
3. **Tree and fronts**: medial-axis tree, open fronts with returns, spiral in open areas,
   order. This replaces the steering. Adaptive golden files change here.
   *Stage 1 found:* at stepover 0.4 the grid steering re-enters on passes too short for a
   ramp or helix entry, so those two tests (plan.rs, helix.rs) ran adaptive at 0.1.
   *Built:* they run at 0.4 again.
4. **Smoothing and arcs**: ρ_min rounding and G2/G3 output. The targets in §4 become test
   assertions.
5. **GUI**: the adaptive fields show *Stepover (cut width)*, limited to 0.5. Remove the old
   steering code.

## 9. Decisions

1. **Approach**: fronts of exact offsets, divided and ordered by the medial-axis tree (§2–3),
   instead of tuning the grid steering. The other option was cutting along the medial-axis
   centre lines directly (like trochoidal backbones), but the engagement then varies with
   the channel width. **Recommended: fronts + tree.** It uses no new dependency
   (cavalier_contours and boostvoronoi are already used) and is our own design.
2. **One setting**: stepover replaces `load` (schema 13), with a limit of 0.5 for adaptive,
   and new pockets keep the default 0.4. **Recommended.**
3. **Migration**: `stepover := load`, keeping the engagement of existing projects (§5).
   **Recommended.** The alternative, keeping the old stepover, changes how hard the tool
   cuts, so I'd not choose it silently.
4. **Every pass in the operation's direction** (climb by default), with returns through
   cleared material. Cutting back and forth would halve the returns but mix climb and
   conventional. **Recommended: one direction.**
5. **ρ_min, the tightest turn**: 0.1 × D (0.6 mm for a 6 mm tool), and never tighter than
   the arc tolerance. A larger radius is smoother but skips more at front joins, which the
   next pass must cut. **Recommended: 0.1 × D**, a named constant, revisited after
   measuring.
6. **Targets** (§4): a first guess to be confirmed in stage 2–4. If a target turns out
   wrong, I report the measured value and ask before relaxing it. **Recommended.**
7. **Start**: at the largest circle, with the operation's entry. A helix fits best there.
   **Recommended.**

## 10. As built (stage 3)

`chipload_toolpath::fronts`, called from `pocket::adaptive_rings`. Differences from §3:

- **No band for the wall pass.** R is the whole wall-pass area (the pocket shrunk by r +
  clearance), not that less w. With a band, a front meeting a wall at a shallow angle (in
  corners, round islands) has the tool running along R's edge with the band beside it:
  measured at load 0.1, up to 13 × w over a few millimetres, close to a full-width slot. With
  the fronts reaching the walls there is no material beside the edge; the wall pass only
  takes the cusps between front ends.
- **The order needs no medial-axis tree.** A pass waits for the earlier passes within about
  a cut width of it (the ones it cuts against); among the ready ones, a neighbour of the
  last pass goes first, then the nearest. That finishes a branch before the next, as the
  tree would, and handles splits and joins without building it. The start is the centre of
  the largest circle (§3.1), found by bisection on shrinking R: the same point the medial
  axis gives, without it.
- **Returns**: straight if the line stays in R and the material grid says it cuts
  (almost) nothing; otherwise back along the pass just cut until a straight line is clear.
  A new path (after a retract) goes down at a point the tool has already been at and
  moves across, so it never plunges into material.
- **Spiral**: from one closed front to the next (and from the opening circle), the next
  starts 4 w further along and the way across is a gentle diagonal (§3.3). Smoother
  blending is stage 4.
- **Engagement measure**: the grid's cell count per 0.2 mm step varies by a cell or two
  even where the cut is steady (at load 0.1 a step should cut 3 cells), so the tests
  average it over every 1 mm. The opening round the plunge is an entry and isn't measured.

Measured (6 mm tool, 1 mm averages; "worst" is the largest 1 mm average):

| Pocket | Stepover | Median | Within ±20 % | Within ±50 % | Worst |
|---|---|---|---|---|---|
| 100 × 60 square | 0.4 | 1.05 | 82 % | 89 % | 1.8 |
| with a Ø16 island | 0.4 | 1.05 | 76 % | 85 % | 2.4 |
| two islands (the screenshot) | 0.4 | 1.05 | 75 % | 84 % | 2.3 |
| L shape | 0.4 | 1.05 | 71 % | 82 % | 2.1 |
| with a Ø16 island | 0.1 | 1.13 | 73 % | 90 % | 5.1 |
| two islands | 0.1 | 1.13 | 74 % | 90 % | 5.3 |

The screenshot pocket at stepover 0.4, against the grid steering (§1): reversals over 135°
1 403 → 43 (all in returns), plunges 22 → 10, 1 077 G-code lines with 722 arcs; feed 6 684
mm (1.7 × the ideal 3 620 mm, of which about 560 mm is the wall pass). The 200 × 150 mm
pocket plans in 1.6 s (release).

**Not met yet (§4, decision 6: for the user to decide):**

1. *Engagement within ±20 % on 95 %*: 71–82 %. Most of what's outside is low (the ends of
   passes along walls cut less), which is safe.
2. *Never above 1.5 w*: up to about 2 × w at stepover 0.4 and 5 × w at 0.1, for a millimetre
   or two where a pass starts at a wall or an island. The path there has to take the
   lens of material between the old front, the new one and the wall, and moving along the
   edge the way the fronts grow puts the tool's whole leading side into it: about chord / w.
   Taking it with a way in from the cleared side instead did not help (any way in meets
   the lens across its width). Candidates: leave a strip along those edges and take it last
   with a pass along the wall the other way, or slow the feed there (out of scope, §6).
3. *Cutting + returns ≤ 1.6 × ideal*: 1.7 × at stepover 0.4. The returns are what cutting
   every pass in one direction costs (decision 4).
4. Tight turns: the joins from a pass to its return still turn sharply (stage 4, ρ_min).
5. Small loops round the start cut more per millimetre of path ((ρ + r) / ρ for a loop of
   radius ρ): geometry, not a fault of the path; only a slower feed there would help.
