# Spec: a V-carve lifts less (short cuts without the trip to safe Z)

Status: **draft 2026-10-31**, decisions in §6 to review.

The user (2026-10-31, V-carve tests with the Edwardian Script font): "I think the voronoi
creates several super short segments in all the corners, this makes the bit go up and down
doing seemingly nothing." Two ideas: filter away super short segments that are irrelevant,
or give super short segments one full-depth pass last of all.

## 1. What happens now

Measured on "Welcome Home" in Rochester (a formal script like Edwardian), 25 mm high, 60°
V-bit, 3 mm deep (cuts counted from a plunge to the next lift to safe Z):

| Passes | Cuts | Under 0.5 mm | Under 1 mm | Time | Rapids |
|---|---|---|---|---|---|
| 1 | 41 | 9 | 14 | 59 s | 10 s |
| 3 | 57 | 19 | 26 | 70 s | 14 s |
| 6 | 119 | 32 | 48 | 118 s | 27 s |

The short cuts come from two places, and neither is noise:

1. **Corner branches.** The centre line (the medial axis, ADR 0015) runs a branch out into
   every corner, at depth 0 where the corner is sharp. It is what makes a V-carve's
   corners crisp. Branches that change the carve by less than the arc tolerance (0.01 mm)
   are already removed (`Graph::prune`). The centre line is cut as a few continuous
   walks; a branch the walk doesn't pass along becomes a **cut of its own**, starting at
   its corner at depth 0. With one pass: 14 cuts under 1 mm, about 0.5 mm deep.
2. **Pass pieces.** A later pass cuts only where the stroke is deeper than the pass before
   reached (decision 12 of v-carve.md). Where the centre line just dips below that level,
   the piece is a fraction of a millimetre long, and **each piece is a cut of its own**.
   That is why the count grows with the passes.

Each cut costs a retract to safe Z, a rapid, a rapid down to 1 mm above, and a plunge,
for a few tenths of a millimetre of cutting. With acceleration on a real machine that is
roughly half a second each, and it looks like the bit going up and down for nothing.

## 2. The ideas

- **Filtering short branches away (the user's first idea)** would round every corner by as
  much as was filtered: the corners are exactly what the short branches carve. Not
  recommended beyond the present 0.01 mm.
- **Short pieces to full depth last (the user's second idea)** removes their lifts from the
  early passes, but each still needs its own lift in the last pass, and it puts the whole
  depth's load on those spots.
- **Not lifting at all for short trips** (below) keeps every corner and the equal load per
  pass, and removes the lifts themselves. Recommended.

## 3. Behaviour (recommended)

### 3.1 Corner branches out and back

While walking the centre line, a **dead-end branch shorter than 5 mm** (decision 2) is cut
**out and back** when the walk reaches the junction it hangs from. The bit runs out along
it to the corner, rising to depth 0 there, and comes back along the same line to the
junction, then the walk goes on. The way back is through the groove just cut: no load.

- Longer branches stay cuts of their own, as now: running back costs more than a lift.
- The rule for steep moves stays: steeper than 45° goes at the plunge feed.
- Every branch is still cut, so the carve is exactly the same shape.

### 3.2 Later passes travel along the groove

In a pass after the first, two pieces of the **same walk** less than **10 mm apart along
it** (decision 3) are joined. Between them, the bit follows the centre line at its own
depth, which the earlier passes already cut to the full: it travels through the groove
without cutting, instead of lifting. Pieces further apart, or of different walks, are
reached through safe Z as now.

### 3.3 What stays

- The carve's shape, the depths of each pass and the area-by-area order (decision 13 of
  v-carve.md).
- Walls and floor passes of wide strokes (vcarve-wide-strokes.md) are unchanged.
- Every program passes the independent checker: the new moves are feed moves inside the
  carved groove, never rapids below safe Z.

## 4. Expected effect

With one pass, the short cuts are corner branches (§1), so 3.1 should remove most of their
lifts; with more passes, 3.2 removes the lifts between nearby pass pieces. How many go
depends on how far apart the pieces lie, so it is measured when built and the table in §1
filled in again under "As built".

## 5. Tests

- **The carve is unchanged:** the set of points and depths cut (the centre line, each pass)
  is the same as before, only the order and the links differ. Checked on the script
  example and the golden V-carves.
- **Branches:** a letter with a sharp corner gives no separate cut for its corner branch;
  a branch longer than the limit still does.
- **Passes:** in three passes, no lift between two pieces of one walk closer than the
  limit; the link stays at or above the depth already cut everywhere along it.
- **Golden G-code:** the V-carve programs change (fewer retracts, the out-and-back moves).
  The diff is reviewed, and every program passes the checker.

## 6. Decisions

1. **Don't lift for short trips (§3) rather than filtering branches or deferring pieces.**
   *Recommended:* the carve keeps every corner and its equal load per pass, and the lifts
   themselves go. Alternatives: the user's ideas in §2.
2. **Out and back for corner branches shorter than 5 mm.** *Recommended:* 5 mm back at a
   typical 1200 mm/min takes 0.25 s, less than a lift. Alternatives: a smaller limit, or
   always (a long branch then costs more than the lift it saves).
3. **Join pass pieces up to 10 mm apart along their walk.** *Recommended:* travelling
   10 mm through the groove at the cutting feed costs about as much as a lift and a
   rapid. Alternatively 5 mm, or a limit computed from the machine's feeds.
4. **Always, no setting** (one best way, as for the pocket's links and its staying down
   between passes). *Recommended.* Alternative: a V-carve setting.
