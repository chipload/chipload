# Spec: pocket operation

Status: **approved 2026-10-08**

Clears all material inside selected closed contours down to a depth, leaving islands standing.
It builds on the v0 pipeline (`v0-profile-pipeline.md`): same import, selection, depth passes,
post-processor and independent checker.

Motivating drawing: `tests/corpus/dxf/messy_2.dxf`. Pocket shapes there deliberately cross a
part's outline to leave an opening in the wall, and shapes are picked one by one.

Out of scope (later specs): ramping/helical entry, a separate finishing pass or stock-to-leave,
rest machining with a smaller tool, adaptive/trochoidal clearing, zig-zag/raster clearing,
V-bit pockets.

## 1. Job model (schema v3)

`PocketOp` already exists in v2 (`select`, `tool`, `depth`, `step_down`, `stepover`, `feed`,
`plunge_feed`). v3 adds:

| Field | v3 | Migration from v2 |
|---|---|---|
| `PocketOp.direction` | required, `"climb"` or `"conventional"` | set to `"climb"` |
| `PocketOp.keep_tool_down` | optional bool, default `false` (always retract between rings) | not touched |
| `Tool.center_cutting` | optional bool, default `null` (unknown) | not touched |

CLAUDE.md requires climb vs conventional to be explicit, so it can't be optional. No v2 file
has ever produced pocket G-code (pocket was "not implemented"), so the migration preserves no
machining behaviour. The current v2 sample becomes frozen in `tests/corpus/jobs/v2/` and must
still migrate to the same golden G-code.

- `stepover` stays a **fraction of the tool diameter**, 0 < stepover ≤ 1 (0.4 = 40 % of D).
- Pocket requires an `endmill` tool. `drill` or `vbit` → error.
- `Tool.center_cutting`: `false` → error for pockets (they plunge straight down); `null`
  (unknown) → warning; `true` → fine (decision 2026-10-08).
- Operations still run in job order. Pocketing adds no ordering rule between operations
  (decision 2026-10-08: the user sets the order).

## 2. Which area gets cleared

Selection works as in the v0 spec: open contours are skipped and reported; invalid and partly
matched contours are errors.

The **region** to clear is built from the selected closed contours only. Contours that weren't
selected are ignored, even if they lie inside the pocket (e.g. a circle you'll drill later).

1. Nesting depth is computed **among the selected contours only**: the number of other
   selected contours that strictly contain it.
2. Even depth = boundary, odd depth = island.
3. Region = (union of depth-0 boundaries) − (union of depth-1 islands) + (union of depth-2
   boundaries) − …
4. Contours at the same depth that **overlap or cross** are merged (union). This is how the
   user's openings work: a pocket shape that crosses another selected shape becomes one
   connected area to clear.
5. Two selected contours that are **the same shape** (e.g. a circle drawn twice as different
   entity types) → `SelectorError` naming both. Whether it's a boundary or an island is
   ambiguous.

Disjoint pieces of the region (e.g. three separate keyholes in one operation) are cleared one
after another, see §4.

## 3. Toolpath for one region piece

r = tool radius, s = stepover × D.

### Rings
- Level 0 (wall pass): `region.buffer(−r)`, round joins.
- Level k: `region.buffer(−(r + k·s))`, until the result is empty.
- Each level can be several polygons, each with an exterior ring and interior rings (around islands).
- Arc flattening and the GEOS chord factor work exactly as for profiles.

### Errors and warnings
- Level 0 empty (**tool does not fit anywhere** in the region) → `GeometryError` naming the contours.
- Level 0 has more pieces than the region (**tool doesn't fit through a narrow part**) → warning:
  each piece is cleared, material stays in the narrow part.
- **Uncut material check:** the area the tool could reach is
  `region.buffer(−r).buffer(r)`. The area actually swept is the union of every cutting move
  buffered by r. Any leftover piece that survives `buffer(−REST_TOL_MM)` → **warning**
  (decision 2026-10-08) naming its location and size. This catches corners where successive
  rings are too far apart (a large stepover with sharp corners) and the pocket centre left by
  the last ring.
- **Post-condition (safety):** every cutting move must lie inside `region.buffer(−r)` within
  `ARC_APPROX_TOL_MM`, and cutting moves must never come closer than r − tolerance to the
  region boundary. Otherwise → `GeometryError`. Like profiles, nothing is trusted blindly.

### Order inside a piece: inside out
Rings form a tree: each level-k+1 polygon lies inside exactly one level-k polygon. The cut order
is **children first** (innermost polygons first), then the polygon's island rings, then its
exterior ring. So clearing starts at the centre and the wall pass is the last ring at each depth.
Siblings are ordered nearest-neighbour from the current position, with ties broken by (y, x).

### Direction
With the material on the right for climb (M3, as for profiles):
- Exterior rings (pocket wall side, material outside the ring): same as an **inside** profile,
  so climb + cw spindle = counter-clockwise.
- Island rings (material inside the ring): same as an **outside** profile, so climb + cw =
  clockwise.
- Conventional and ccw spindle flip it, using the same `cut_clockwise` table as profiles.

### Start points
Each ring starts at its vertex nearest the current tool position (ties: lowest y, then lowest x).
That keeps links short. Deterministic.

### Links between rings at the same depth
The user chooses per operation (decision 2026-10-08):
- `keep_tool_down: false` (default): always retract to safe_z, rapid to the next ring's start,
  plunge at `plunge_feed`.
- `keep_tool_down: true`: **feed at depth** when it's safe, i.e. the straight segment lies inside
  `region.buffer(−r)` (the tool can't touch a wall or island) **and** is no longer than D (it
  may cut through one stepover band that isn't cleared yet; the limit keeps that short).
  Otherwise retract as above.

### Depth passes
- Same `pass_depths` as profiles: equal passes no deeper than `step_down`, last one exactly at `depth`.
- `depth > stock.thickness` → error (as v0).
- Per piece: for each depth, rapid to the first ring's start, plunge at `plunge_feed`, cut the
  tree with links, then retract to safe_z. The next depth starts again at the centre, where the
  plunge goes through material already cleared by the level above.
- All depths of one piece are finished before the next piece.

### Plunging
Straight plunge at `plunge_feed`, at the innermost ring. This needs a **centre-cutting
endmill**; see `Tool.center_cutting` in §1. Ramping (later) lifts the restriction.

## 4. Several pieces in one operation
Pieces of the region are ordered nearest-neighbour by their first plunge point, starting from
where the previous operation ended (or (0, 0) for the first). Same ordering helper as profiles.

## 5. Determinism
Shapely set operations may return polygons and ring vertices in any order. All results are
normalised: polygons sorted by (min y, min x), rings oriented explicitly, and consecutive points
closer than `CHAIN_TOL_MM` removed. The same job + drawing must give byte-identical G-code.

## 6. New constants (`core/tolerances.py`)
- `REST_TOL_MM = 0.05`: uncut pieces thinner than this aren't reported (chord and float noise).

## 7. Tests
- **Unit:** square pocket (ring count for a known size, stepover and tool); pocket with a circular
  island (island ring direction is opposite to the wall's); two crossing rectangles become one
  region; selected contour inside an unselected one; same shape twice → error; tool too big →
  error; narrow neck → warning; stepover = 1.0 on a sharp-cornered shape triggers the uncut
  material check; depth passes at the boundaries (depth = step_down, depth = thickness,
  depth > thickness → error); non-endmill tool → error.
- **Direction table:** exterior and island rings × climb/conventional × cw/ccw.
- **Links:** with `keep_tool_down: false` every ring change retracts; with `true` a short link
  inside the region feeds at depth, and a link that would cross an island or is longer than D
  still retracts.
- **Tool checks:** `center_cutting: false` → error; `null` → warning.
- **Hypothesis:** for random simple polygons with random islands inside them: every cutting point
  is inside `region.buffer(−r + tol)` (no gouge into walls or islands), and with stepover ≤ 0.5
  no uncut material is reported.
- **Real drawing:** `messy_2.dxf`, lower part: the crossing pocket shapes (#4 ∪ #6) cleared as
  one region; the result passes the checker; golden G-code.
- **Migration:** frozen v2 sample → v3 → unchanged golden G-code; a v2 pocket op gets
  `direction: "climb"` and keeps `keep_tool_down` false.

## 8. Known limitations
No ramping, so plunging needs a centre-cutting endmill. No finishing pass: the wall is cut at
every depth by the last ring, which leaves step marks on the wall. Contour-parallel clearing
only. Sharp inside corners keep a radius of r (normal for an endmill). Large files, because
arcs are G1 chords.
