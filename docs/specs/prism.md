# Prism carving: raised shapes with a sharp ridge along their centre

Status: **approved 2026-11-15**, every decision as recommended ("Go"); **built
2026-11-15**, both steps of §8. What building settled is in §13. TODO *Ideas*, "More 2.5D
toolpaths", item 3; the user asked for it after the fill pattern.

## 1. What it is for

A V-carve sinks a shape into the board: a groove, deepest along its centre line. A **prism**
is the opposite: the shape stands out as a roof, its sides sloping up from its edges to a
sharp ridge along its centre line, as hand-chiselled raised letters. It suits sign letters,
monograms, stars, borders and ornaments, with the same V-bit and the same drawing as a
V-carve.

## 2. The shape it cuts

The selected closed shapes form regions as for a V-carve or a pocket: a shape minus the
selected shapes inside it (a letter O's counter is a hole). For a V-bit of included angle α
(half angle θ) and flat tip *t* (0 for a pointed bit):

- At a point *p* of a region's outline, let *r(p)* be the radius of the largest circle
  inside the region that touches the outline at *p*: half the shape's width there, or
  less near a corner. Its centre lies on the region's centre line (medial axis), as in
  the V-carve (v-carve.md §1).
- The tool's tip runs along the outline, its centre *t/2* out into the waste (as a
  chamfer's, chamfer.md §2), down to **h(p) = r(p) ÷ tan θ** below the start depth. The
  bit's inner flank then rises from the outline at *p* to the surface exactly at the
  circle's centre: on the centre line.
- Going round the whole outline, the flanks together cut the region into a roof whose
  ridge is the centre line, at the surface.

So a stroke *w* wide comes out as a ridge *w/2* ÷ tan θ high: with a 90° bit, a 10 mm
stroke is a 5 mm ridge; with a 60° bit, a steeper one, 8.7 mm. At a sharp outside corner
*r* goes to 0: the edge rises to the surface there and the corner comes to a point, as a
chiselled serif does. Where strokes meet (a T, an X), the larger circle wins and the
junction rises to a peak.

### Wider than the bit

Where *h(p)* would be deeper than the bit's cone (the shank would rub), or than the **max
depth** if one is set, the tip stays at that depth. The flank then reaches the surface
before the centre line, and the middle stays at the surface, untouched: a **bevelled**
shape with a flat top instead of a ridge. A note says so when the cone limits it. Setting
a max depth on purpose gives bevelled raised letters of one even bevel.

### Round the shape

The tool's other flank cuts outwards into the waste: round each shape it leaves a V-shaped
trench as deep as the edge there. That is the waste, so it is accepted:

- For a carved-in look, leave it: the shape sits in its own trench.
- For a shape standing on a flat background, clear the background first with a pocket as
  deep as the deepest edge (the plan says how deep that is), or cut the prism into an
  area pocketed beforehand.
- **Shapes too close together:** where one shape's trench reaches another selected shape,
  it cuts into that shape's lower edge. A note names the shapes and how much deeper the
  edge goes there; a max depth makes the edges shallower and the trenches narrower.

Open shapes are skipped with a note, as for a V-carve.

## 3. Moves

- Each region's outline and holes are cut as closed rings, the tip's centre on the line
  offset *t/2* into the waste (verified as every offset).
- Along a ring the depth changes continuously with *r*: points are added until the depth
  between two points is within the arc tolerance (0.01 mm) of *h* there. Where the depth
  changes evenly along an arc it stays a helical G2/G3; elsewhere short lines.
- Each ring starts at its shallowest point (a corner at the surface, where there is one),
  so the tool goes down where it cuts least.
- **Passes:** with a step down, each pass is the same ring cut off at its level (deeper
  than *k* steps: *k* steps), as a flute's (fluting.md §4), round and round without
  retracting; then the tool retracts. Without, one pass.
- **Direction:** climb or conventional, as a chamfer.

## 4. Settings and format

A new operation type; no schema bump, as the chamfer, the flute and the pattern.

```json
{"type": "prism", "name": "", "enabled": true, "part": 1, "material": null,
 "select": {"shapes": [1, 2, 3]}, "tool": 2,
 "start_depth": 0.0, "max_depth": null, "step_down": 2.0,
 "direction": "climb", "feed": 1500.0, "plunge_feed": 500.0}
```

- `tool`: a V-bit with its angle; anything else is refused.
- `start_depth`: where the surface is, mm down from the stock top (0 for the top, more for a
  prism in the floor of a pocket).
- `max_depth`: null for as deep as the shapes and the bit's cone need; a number for
  bevelled shapes no deeper than that.
- `step_down`: null for one pass.
- No `end_depth`: the depth follows from the shapes and the bit. The plan, the report and
  the setup sheet give the deepest edge ("the deepest edge is 6.2 mm").

## 5. Command line and window

- **Command line:** `chipload op add part.chipload prism --shapes 1,2,3` (also `--layer`,
  `--tool`, `--max-depth 3`, `--step-down`); `set …operations.N.max_depth=3`.
- **Window:** *Prism* in the S menu (key **R**: P is the profile), the Operation menu
  (*New Prism*) and the Browser's new operations. Not on the ribbon: the OPERATION group's
  small-button columns are full, and another would push the ribbon past the default
  window again (fluting.md §13). Its properties: Tool (V-bits), Start depth, Max depth
  (a tick and a value), the deepest edge shown beneath, Step down, Direction, Feeds.
- **Defaults** for a new prism: the project's first V-bit; no max depth; the tool's step
  down; climb.
- **Setup sheet:** "prism, edges down to 6.2 mm, climb".
- **Guide:** a page *Prism* under Operations, with a picture of prism letters beside the
  same letters V-carved.

## 6. Edge cases

- A rectangle: a ridge along its length, its ends rising to the corners as a hipped roof.
- A square, a circle: a pyramid, a cone, each to a point at the centre.
- A ring (a circle with a hole): a ridge round the middle.
- A star: a point at the centre, ridges out to the tips.
- A stroke wider than the cone allows: a flat top, a note.
- A max depth: bevelled edges, a flat top.
- Two shapes closer than their trenches: a note.
- A flat-tipped bit (the tip in the waste, so no step at the edge).
- Start depth above 0.
- A tool that isn't a V-bit, or a V-bit without an angle: refused.

## 7. Tests

- The ring's depth at each point is *r* ÷ tan θ (or the cap), against *r* found
  independently (the largest touching circle); the tip's centre *t/2* outside the line.
- Simulated: a 10 × 60 mm rectangle with a 90° bit: the ridge along its centre at the
  surface, 2.5 mm down a quarter of the way across, 5 mm at the edge; a circle a cone with
  its point at the surface; a max depth leaves a flat top at the surface as wide as
  expected.
- Passes: each the ring cut off at its level, never more than a step under the tool.
- A proptest: random rectangles, circles and polygons: every cutting point's depth is
  within the arc tolerance of the formula, and nothing inside the shape is cut lower
  than its roof (no gouge of the shape itself).
- Notes and refusals: open shapes, the cone limit, shapes too close, a non-V-bit.
- Golden: a new `v36/prism.chipload` (a rectangle, a star polygon and a ring with a pointed
  90° bit, one pass; a wide rectangle with a 60° bit with a flat, a max depth and a step
  down) and its `lines/` twin; existing goldens unchanged.
- CLI: `op add prism` and its options; the window: the S menu's R, the form.

## 8. Order of building

1. Format, validation, the depth along the outline, the rings and passes, notes, the
   command line, goldens.
2. Window: the form, the S menu, the Operation menu; sheet; guide.

## 9. How *r(p)* is found

For each point of the tip's ring, the inward normal *n* (from the ring itself: the
bisector at a corner) and the foot *p* on the outline, *t/2* along it. The largest circle
touching at *p* has its centre at *p + r·n* and is inside the region while the distance
from that centre to the outline is at least *r*; circles touching at one point grow inside
each other, so *r(p)* is found by halving the interval (to `CHAIN_TOL_MM`), with the
exact distance to the real outline that the V-carve uses (`medial::Outline`). At a convex
corner no circle fits (*r* = 0); at a concave one the circle grows along the bisector.
First principles, no outside source; the medial axis itself is the V-carve's (ADR 0015).

## 10. What is left for later

- A sunken prism (the shape cut down to a valley along its centre) is exactly a V-carve
  without a depth limit; no second mode.
- Clearing the background in the same operation (a pocket to the deepest edge, round the
  shapes) could come later as an option, if asked for.

## 11. Not in scope

Prisms of open shapes (a centre line has no width), and of pictures or models.

## 12. Decisions

1. **A new operation type, *Prism*** (§4), no schema bump. *Recommended:* users look for it
   by name; a V-carve option would give V-carve two opposite meanings.
2. **The tip along the outline, as deep as the largest touching circle allows** (§2), so the
   ridge is the centre line at the surface. *Recommended:* one ring per outline cuts the
   whole roof, and it is exact with any V-bit angle.
3. **Wider than the cone, or than a max depth: a flat top** (§2), with a note for the cone.
   *Recommended:* a part is never refused for one wide stroke; and the same setting gives
   bevelled raised letters on purpose.
4. **The trench round the shapes is left** (§2): the guide says to pocket the background
   first for a flat one. *Recommended*; clearing it in the same operation can come later.
5. **Shapes whose trenches reach each other: a note, not a refusal** (§2). *Recommended:*
   letters set close are common; the note says where, and a max depth fixes it.
6. **Holes as for a V-carve** (§2): a selected shape inside another is a hole (an O's
   counter), its own outline cut with the flank into the letter. *Recommended.*
7. **Passes as a flute's: the ring cut off at each level, round and round, then retract**
   (§3). *Recommended.*
8. **Each ring starts at its shallowest point** (§3). *Recommended:* the least plunge,
   none at a corner.
9. **Only V-bits with an angle** (§4). *Recommended:* the slope comes from the flank; a ball
   nose would need its own geometry.
10. **The S menu's R, the Operation menu, not the ribbon** (§5). *Recommended*; the ribbon
    has no room left without dropping something else.
11. **No end depth; the deepest edge is reported** (§4). *Recommended:* the depth follows from
    the shapes, as a V-carve's.
12. **Defaults** (§5): the first V-bit, no max depth, the tool's step down, climb.
    *Recommended.*

## 13. As built

- **Normals:** the inward normal at a ring point is the bisector of its two edges' normals,
  an edge on an arc giving the arc's exact radius. Two chords of unequal length either side
  of where two arcs meet (a drawn circle's halves) tilt a chord bisector by about a
  milliradian, enough to stop the circle search a millimetre early; the radius doesn't.
- **The circle search** allows `OFFSET_CHECK_SLACK_MM` of slack, so a convex corner comes
  out at the surface (Z0.000), not a few microns down.
- **Points** where the depth changes along an edge are added by halving until the depth is
  within the arc tolerance; points on a straight edge between two of the same slope are
  dropped again, so a straight side is one move.
- **Passes** go to the deepest edge actually found (not to the cone), so a shallow prism
  isn't cut in empty laps.
- **The trench check** looks along the outward normal, out to where the cone meets the
  surface, for a point inside a selected shape (its own other side included, as the inside
  of a U); the note gives the deepest cut into that shape's edge.
- **The star's inner corners:** the largest circle touching an inner corner is larger than
  the inscribed circle of the star's middle, so a star whose middle is just narrower than
  the cone still gets the flat-top note; the golden sample keeps that case.
- **The setup sheet** gives the depths as "0 to at most …" (the cone or the max depth); the
  deepest edge actually cut is in the plan's report.
- **Code:** `chipload-toolpath/src/prism.rs` (rings, depths, notes, moves),
  `plan_prism_op`; `PrismOp::deepest` in the job model; `OpKind::Prism`;
  `op add prism --max-depth`; the form's `prism_sections`; `Icon::Prism`.
