# Spec: pocket operation

Status: **approved 2026-10-08**

Guide: docs/book/src/operations/pocket.md.

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

The areas to clear are built from the selected closed contours only. Contours that weren't
selected are ignored, even if they lie inside a pocket (e.g. a circle you'll drill later).

1. Nesting depth is computed **among the selected contours only**: the number of other
   selected contours that strictly contain it.
2. Every selected contour at even depth is **its own pocket**.
3. Selected contours completely inside it, one level deeper, are its **islands** and are left
   standing (decision 2026-10-08). A contour inside an island is its own pocket again.
4. Contours that **overlap or cross** are **separate pockets**, each cleared completely as its
   own shape. They are never merged (user decision 2026-10-08: crossing shapes such as the
   two bars in `messy_2.dxf` are separate geometries). Where two pockets overlap, the second
   one simply cuts air in the overlap.
5. Two selected contours that are **the same shape** (e.g. a circle drawn twice as different
   entity types) → `SelectorError` naming both. Whether it's a boundary or an island is
   ambiguous.

Pockets (e.g. three separate keyholes in one operation) are cleared one after another, see §4.

## 3. Toolpath for one region piece

r = tool radius, s = stepover × D.

### Rings
- Level 0 (wall pass): the region shrunk by r.
- Level k: the region shrunk by r + k·s, until the result is empty.
- "Shrunk by d" is an offset of the whole region (outer loop and islands together, true arcs,
  cavalier_contours `Shape`). History: the Python reference avoided GEOS's negative buffer,
  which wrongly returned empty near collapse and left pocket centres uncut; the Rust property
  test still covers that case.
- Each level can be several areas, each with an outer ring and island rings.
- Arc flattening works exactly as for profiles.

### Errors and warnings
- Level 0 empty (**tool does not fit anywhere** in the region) → `GeometryError` naming the contours.
- Level 0 has more pieces than the region (**tool doesn't fit through a narrow part**) → warning:
  each piece is cleared, material stays in the narrow part.
- **Uncut material check:** the area the tool could reach is the region shrunk by r, then grown
  by r. The area actually swept is everything within r of a cutting move. Any leftover piece
  that still holds a disk of radius `REST_TOL_MM` → **warning**
  (decision 2026-10-08) naming its location and size. This catches corners where successive
  rings are too far apart (a large stepover with sharp corners) and the pocket centre left by
  the last ring.
  - How (2026-10-09): exact distances, no buffered polygons. The edges of the level-0 areas
    are cut rings, so leftover can only lie inside them; a quadtree finds points farther than
    r from every cutting move (leftover) and farther than r + `REST_TOL_MM` (a piece that
    survives the shrink), and reports the pieces holding such points. Leftover thinner than
    `ARC_APPROX_TOL_MM` is ignored, so chord slivers can't join two real pieces into one (the
    Python reference's GEOS buffers had that problem: 5 places reported where there were 6).
- **Post-condition (safety):** every cutting move must lie inside the region shrunk by r, within
  `ARC_APPROX_TOL_MM`, and cutting moves must never come closer than r − tolerance to the
  region boundary. Otherwise → `GeometryError`. Like profiles, nothing is trusted blindly.

### Order inside a piece: inside out
Rings form a tree: each level-k+1 polygon lies inside exactly one level-k polygon. The cut order
is **children first** (innermost polygons first), then the polygon's island rings, then its
exterior ring. So clearing starts at the centre and the wall pass is the last ring at each depth.
Siblings are ordered nearest-neighbour from the current position, with ties broken by (y, x).

**Amended 2026-10-16** (docs/specs/pocket-island-order.md): a ring waits only for the
rings next to it one level in (within a tool diameter), and an outer ring also for its own
level's island rings, so the wall is still last at each depth. From the ring just cut the
order goes on outward to a ready ring of the same kind next to it, otherwise to the nearest
ready ring. Each island's rings are cut one after another instead of one level at a time
across the whole pocket.

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

**Ties, amended 2026-10-30:** every "nearest" choice in a pocket (cut order, ring starts,
stored start points, region order, the helix's room) counts distances within the chaining
tolerance (0.001 mm) of the nearest as equal, and takes one of those by its second key
(lowest y, then x; or level, node and kind). On a symmetric drawing two candidates are
equally near up to the last bits, which differ between platforms' maths libraries;
compared exactly, the G-code followed them.

### Links between rings at the same depth
The user chooses per operation (decision 2026-10-08):
- `keep_tool_down: false` (default): always retract to safe_z, rapid to the next ring's start,
  plunge at `plunge_feed`.
- `keep_tool_down: true`: **feed at depth** when it's safe, i.e. the straight segment lies inside
  the region shrunk by r (the tool can't touch a wall or island) **and** is no longer than D (it
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
Pieces of the region are ordered nearest-neighbour by their distance from the current tool
position (ties: lowest y, then x), starting from where the previous operation ended, or (0, 0)
for the first. The plunge point itself depends on where the tool comes from, so it can't be
used for ordering.

## 5. Determinism
Offset and boolean operations may return areas and ring vertices in any order. All results are
normalised: polygons sorted by (min y, min x), rings oriented explicitly, and consecutive points
closer than `CHAIN_TOL_MM` removed. The same job + drawing must give byte-identical G-code.

## 6. New constants (`chipload_core::tolerances`)
- `REST_TOL_MM = 0.05`: uncut pieces thinner than this aren't reported (chord and float noise).

## 7. Tests
- **Unit:** square pocket (ring count for a known size, stepover and tool); pocket with a circular
  island (island ring direction is opposite to the wall's); two crossing rectangles stay two
  separate pockets; selected contour inside an unselected one; same shape twice → error; tool too big →
  error; narrow neck → warning; stepover = 1.0 on a sharp-cornered shape triggers the uncut
  material check; depth passes at the boundaries (depth = step_down, depth = thickness,
  depth > thickness → error); non-endmill tool → error.
- **Direction table:** exterior and island rings × climb/conventional × cw/ccw.
- **Links:** with `keep_tool_down: false` every ring change retracts; with `true` a short link
  inside the region feeds at depth, and a link that would cross an island or is longer than D
  still retracts.
- **Tool checks:** `center_cutting: false` → error; `null` → warning.
- **Property (proptest):** for random simple polygons with random islands inside them: every cutting point
  keeps at least r − tol from every wall and island (no gouge), and with stepover ≤ 0.5
  no uncut material is reported.
- **Real drawing:** `messy_2.dxf`, lower part: the crossing bars #4 and #6 cleared as two
  separate pockets; the result passes the checker; golden G-code.
- **Migration:** frozen v2 sample → v3 → unchanged golden G-code; a v2 pocket op gets
  `direction: "climb"` and keeps `keep_tool_down` false.

## 8. Known limitations
No ramping, so plunging needs a centre-cutting endmill. No finishing pass: the wall is cut at
every depth by the last ring, which leaves step marks on the wall. Contour-parallel clearing
only. Sharp inside corners keep a radius of r (normal for an endmill). Large files, because
arcs are G1 chords.

## 9. As built, revised 2026-10-15 (schema 26)

User rule: no option that is objectively worse. Three settings went, with a migration that
drops them (`v25_to_v26`); projects that had them off now cut the same rings with links and
rounded corners.

- **Links:** between rings the tool stays down wherever the link is safe (the rule in §3:
  inside the region, clear of walls and islands, at most a diameter), verified with the
  rings. `keep_tool_down` is gone.
- **Rounded inner rings:** always, with the corner radius capped at two stepovers instead
  of a diameter (pocket-round-corners.md as built). `round_corners` is gone from the file;
  the planner keeps a `Choices::round_corners` only for tests that compare with sharp rings.
- **Strategy:** rings is the one way; `strategy` is gone, and a project still marked
  trochoidal or adaptive (ADR 0017) is a rings pocket.
- The four frozen projects with removed strategies are golden G-code again
  (`v7_trochoidal`, `v7_adaptive`, `v12_adaptive`, `v13_adaptive_two_islands`).
- **Spiral** (same day, docs/specs/pocket-spiral.md): a ring steps out onto the ring
  outside it over its last two diameters instead of closing, where that is checked safe;
  the link move goes with it.
- **Between depth passes** (2026-10-29, docs/specs/pocket-stay-down.md): the tool lifts
  0.5 mm and moves back at feed instead of retracting, where checked safe.
