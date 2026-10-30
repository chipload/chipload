# Spec: V-carving strokes wider than the bit can carve

Status: **approved 2026-10-30**, all decisions as recommended (§9). Built 2026-10-30: the
walls (§3), the floor pass with the V-bit's clean-up and the hint (§4), the command line and
the window (§7). Not built: an action on the floor-left warning itself; the Browser's
right-click offers *Add floor pass ▸*.

Builds on docs/specs/v-carve.md (its decision 3, "strokes wider than the bit: capped at the
maximum depth and reported; no flat floor clearing yet", is replaced by this) and on
docs/specs/rest-passes.md (the pattern of a pass that belongs to its operation).

## 1. Why

The user (2026-10-30): when the allowed depth is too shallow for the cut to touch both sides
of a stroke, the V-bit should follow the sides.

Today a V-carve follows the stroke's centre line and goes to depth(p) = (r(p) − w/2) / t
(v-carve.md §1). Where that is deeper than the maximum depth, the tool stays at the
maximum depth on the centre line: a groove 2R wide at the surface, with

    R = w/2 + (end_depth − start_depth) · t        (also no more than the bit's own radius)

and the edges of the stroke are not reached. The warning says so, and the letter comes out
too thin with rounded, wrong walls.

## 2. What "correct" means now

For a region (as in v-carve.md §1), split by the stroke's half width r(p):

- **Narrow parts** (r ≤ R): as today, the centre line at depth(p).
- **Wide parts** (r > R): the bit runs at the maximum depth along the region's outline
  offset inward by R, so its cone touches the outline at the surface all along: **the walls
  are cut to the drawing, sharp, at the bit's angle**, down to the maximum depth.
- What lies between the walls of a wide part, the **floor**, is cleared flat at the maximum
  depth by a **floor pass** (§4), if the operation has one.

The two rules of v-carve.md §1 then hold for the V-bit and its floor pass together: no
gouge (the cone's circle at the surface stays inside the region, within `VCARVE_TOL_MM`;
the floor tool stays inside the floor, §4) and full reach (every point of the region is
cut), except the floor when there is no floor pass, which is a warning (§6).

## 3. The V-bit's paths

1. **Walls.** The region offset inward by R (the offset code profiles and pockets use, with
   its verification), at the maximum depth. Corners of the offset are kept sharp where the
   outline's are: an inside corner of the outline gives a sharp corner of the offset (the
   cone reaches into it at the surface along the bisector, see 3); an outside corner gives
   an arc of radius R around it, as any offset does, which the cone follows exactly.
2. **Centre line where narrow.** The medial axis as today, but only where r ≤ R. An axis
   edge that crosses r = R is cut at that point; the part with r > R is dropped, because the
   walls and the floor cover it.
3. **Joining.** Where a narrow part meets a wide one (a serif into a stem, the end of a
   wide stroke), the centre line arrives at depth = max depth exactly where it meets the
   wall ring (r = R there): the two paths touch, and the tool goes from one to the other
   without lifting where the walk allows (as paths join today, v-carve.md §4.5). At an inside
   corner of a wide part the axis branch running into the corner is narrow near the corner
   (r → 0) and is kept: it carves the corner up to the surface, as today.
4. **Depth passes** (`step_down`): every pass follows the same paths, no deeper than its
   level, and a later pass only where the one before stopped short (as v-carve.md §4.5).
   A wall at a shallower level keeps its cone inside the outline (its radius at the surface
   is smaller), so passes need nothing new. (Settled while building, 2026-10-30.)
5. **Entering a wall:** the bit plunges at the plunge feed where the ring starts, the point
   nearest the tool, as V-bits plunge at the start of any carve.

Rule 1 (no gouge) holds by construction for the walls (offset R, cone radius R at the
surface) and is checked as today for every move (v-carve.md §4.6).

## 4. The floor pass

A V-carve may have a **floor pass**: a flat end mill that clears the floor of the wide parts,
like a rest pass belongs to its pocket (rest-passes.md).

- **The area:** the region offset inward by R − w/2, the line where the V walls meet the
  floor. Its tool centre stays a tool radius inside that, so the end mill never touches a
  wall (the walls slope away from the floor upward, so touching the floor line at the bottom
  is the closest it gets).
- **Depth:** from the V-carve's start depth to its end depth, in steps of its own step down.
- **Taken from the V-carve, always:** the shapes, start and end depth, and whether it is cut.
  **Its own:** the tool (an end mill), feed, plunge feed, step down, stepover, entry, on/off.
- **Cut:** as a pocket of that area (the pocket planner: rings, entry, verification), so
  it brings nothing new to plan or check.
- **Order (decision 4):** before its V-carve. The end mill takes the bulk; the V-bit then
  cuts only the walls and the narrow parts. Tool changes as for rest passes: the floor passes
  of a run of V-carves with one V-bit go first, together.
- What the end mill can't reach (corners of the floor, parts narrower than it) the V-bit
  cleans up as far as it can, and what is still left is reported with a hint (§4.1).

### 4.1 How much of the floor the end mill clears, and a hint (the user, 2026-10-30)

A 6 mm end mill may be too wide for the floor of small letters. Chipload says how much of
the floor the chosen end mill clears and the largest end mill that would clear it all, as
information, never as a limit.

- **Reached:** the end mill clears the floor F opened by its radius ρ: F shrunk by ρ, then
  grown by ρ again (what a round tool can sweep inside F). The rest, F minus that, is left.
- **The V-bit cleans up after it (decision 7):** where the floor pass leaves material, the
  V-bit's centre line continues at the maximum depth (the axis parts with r > R that §3.2
  drops, kept where they run through left material). That clears the corners of the floor
  along their bisectors and the narrow ends where a wide stroke tapers into a narrow one,
  the places no round tool reaches.
- **What counts as left:** material still standing more than `FLOOR_LEFT_MM` (decision 6:
  0.2 mm) above the floor after both: the V-bit's walls and clean-up slope up from the floor
  at its angle, so a leftover point x stands about its distance to the nearest V cut ÷ t.
  Slivers lower than that are the kind sanding or a pass of the V-bit leaves anyway.
- **Coverage** = the floor's area minus the area left, as a share of the floor's area.
- **The hint:** the largest end mill diameter, to 0.1 mm, that leaves nothing (searched by
  halving between 0 and the chosen diameter; coverage only grows as the diameter shrinks).
  If the chosen one already leaves nothing, no hint.
- **Shown:** as a note in the plan report and on the setup sheet, and under the floor pass's
  tool in its form: "The 6 mm end mill clears 85 % of the floor (contours #3, #4). One of at
  most 3.4 mm would clear all of it." With the library's end mills that fit, smallest
  difference first: "In your library: 3.175 mm flat (2 flutes)." (decision 8).
- **Several regions:** coverage over the whole operation's floor; the hint is the smallest
  of the regions' hints, and the note names the contours that need it.

## 5. The file (decision 5)

A V-carve gets an optional `floor`:

```json
"floor": { "tool": 2, "feed": 1500, "plunge_feed": 400, "step_down": 2.0,
           "stepover": 0.45, "entry": {"type": "helix", "radius": 1.5, "ramp_angle": 0.05},
           "enabled": true }
```

Missing means no floor pass. No schema bump: older Chipload refuses an operation with an
unknown field, and reads every other file as before.

The walls (§3) need no field: they replace the capped centre line for every V-carve
(decision 1). Existing projects whose strokes are too wide get walls from now on: their
G-code changes, for the better, and the release notes say so.

## 6. Warnings

- **Floor left:** a V-carve with wide parts and no floor pass: "Operation 3 (Sign): the
  floor between the walls of contours #3, #4 is left at the surface (12.4 cm²): add a floor
  pass with an end mill." (replaces the *Capped* warning, v-carve.md §5).
- **Floor not reached:** with a floor pass that doesn't clear it all, the coverage and the
  hint of §4.1 (a note, not an error).
- The *Narrower than the tip* warning stays.

## 7. Command line, window, guide

- `chipload op add-floor part.chipload --op 3 --tool 2` adds the floor pass (feeds from the
  tool's material, as other new operations get them); `op set-floor …`/`--none` changes or
  removes it; `get`/`set` reach it by path (`operations.2.floor.stepover`).
- The window: in the Browser the floor pass is a leaf under its V-carve, as rest passes are
  under their pocket; right-click a V-carve: *Add floor pass*; the floor-left warning has the
  same action. Its form has the fields of §4.
- The guide's V-carve page gets a section *Wide strokes*: walls, the floor pass, with a
  picture.

## 8. Tests

- **Exact:** a 40 × 10 rectangle, 90° bit, max depth 2 (R = 2): the wall ring is the
  rectangle offset by 2 at depth 2; the four corner branches run from the corners up to
  where r = 2; no centre line in the middle. With a floor pass of a 3 mm end mill: a pocket
  of the rectangle offset by 2 (pointed bit: R − w/2 = 2) at depth 2.
- **A ring between circles** wider than 2R: two circles of walls, the floor between them.
- **Property tests:** the stars and bulged rectangles of v-carve.md §10 with a cap low enough
  to make them wide: no gouge (V-bit and end mill), full reach of V-bit plus floor pass
  except corners the end mill can't reach, which must lie within its radius of an inside
  corner of the floor.
- **Boundaries:** a stroke exactly 2R wide (no walls, no floor), R limited by the bit's
  diameter, a tip wider than the floor (w ≥ 2R: no floor), step-down passes, floor pass off,
  floor pass with a V-bit refused (an end mill is needed).
- **Coverage and hint:** a 40 × 10 floor with a 12 mm end mill (too wide: the hint is the
  floor's width), a 3 mm one (100 %, no hint); a letter-like region with a neck: the hint is
  the neck's width; coverage never falls as the diameter shrinks (property test); the
  library's end mills listed smallest difference first.
- **Golden:** the existing V-carve goldens that hit the cap change (walls instead of the
  capped centre line), each explained; a new golden with a floor pass.
- **CLI and window:** `op add-floor`, the Browser leaf, the warning's action.

## 9. Decisions

1. **Walls always, no option** (the user's request; "no worse options"): every V-carve cuts
   walls where a stroke is wider than its depth allows. **Recommended.** Alternative: an
   option *Wide strokes: walls / centre line only*, defaulting to walls.
2. **The floor: a pass with a flat end mill** belonging to the V-carve (§4). **Recommended:**
   fast and flat, and the user's 6 mm end mill is already in the library. Alternatives:
   (a) the V-bit clears the floor itself in rings at the maximum depth: one tool, but a
   pointed bit leaves ridges s / (2t) high (a 60° bit at 1 mm stepover: 0.87 mm), so a flat
   floor needs a stepover of a tenth of a millimetre and takes very long; (b) no floor
   clearing, the warning only.
3. **Without a floor pass the floor stays at the surface,** with the warning (§6), and
   nothing between the walls is cut. **Recommended:** the walls are right either way, and
   an outline-only carving is a look some signs want. Alternative: keep cutting the capped
   centre line through the middle of the floor as today (a groove down the middle).
4. **The floor pass is cut before its V-carve.** **Recommended** (the end mill takes the
   bulk). Alternative: after it.
5. **The file:** an optional `floor` on the V-carve, no schema bump. **Recommended.**
6. **What counts as left** for coverage and the hint: material more than 0.2 mm above the
   floor (`FLOOR_LEFT_MM`, a named tolerance). **Recommended:** without a height, no round
   tool ever reaches 100 % (sharp corners, tapering ends), so the hint would always be 0.
   Alternative: 0.1 mm, or a field per operation. *Settled while building (2026-10-30):
   and only when it is at least 1 % of the region's floor (`FLOOR_LEFT_SHARE`): a 3 mm end
   mill leaves specks about 0.25 mm high in the sharp corners of a 36 × 6 floor (0.2 %),
   and a note for them would come with nearly every carving.*
7. **The V-bit cleans up after the floor pass** along the centre line, at the maximum depth,
   only where the end mill left material. **Recommended:** corners and tapering ends come
   out clean, and a larger end mill becomes usable. Alternative: no clean-up; the corners
   stay as the end mill leaves them, and the hint gets smaller.
8. **Name the library's end mills that fit** in the hint. **Recommended.** Alternative: the
   diameter only.
