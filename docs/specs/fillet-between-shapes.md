# Spec: fillets at line–arc corners and between two shapes

Status: **approved 2026-11-21** (the user: "go ahead"; §8 as recommended). Being built,
see *As built* (§10). The first item of
step 2 of the drawing review (TODO, *Next* 1; docs/specs/drawing-improvements.md).

Builds on drawing-editing.md §3 (Fillet and Chamfer), §4 (Trim, Extend and the joining
after a trim) and polyline-radii.md (a corner's radius kept as a value).

## 1. What is missing today

- **Corners beside an arc:** a corner between a line and an arc, or between two arcs,
  can't be rounded (drawing-editing.md decision 3; polyline-radii.md decision 2). A
  slotted bracket, a bottle outline or a lever often has exactly those corners.
- **Two separate shapes:** the Fillet tool only works inside one shape. The everyday
  fillet in CAD is between two lines (or a line and an arc) that meet, cross, or don't
  quite reach each other. Both are trimmed or extended to the round, which joins them.
- **One click too many:** Fillet and Chamfer need the shape selected first. Trim, Extend
  and Break already work on the shape under the cursor.

## 2. The geometry (general, from first principles)

A round of radius r tangent to two curves, each one a line or an arc (an arc counts as its
whole circle here):

- **Centre:** where the two curves, offset by r towards the round, cross. A line's offset
  is a parallel line; an arc's is the concentric circle of radius R + r or R − r.
- **Tangent points:** the foot of the centre on each curve. The round runs between them
  the short way round.
- **Which round:** up to eight centres exist (two sides of each curve, and up to two
  crossings of the offsets). The rule that picks one (§3) decides each curve's side and,
  where two crossings remain, which of them.
- **Refused:**
  - no such centre (inside an arc, r of R or more; or the curves too far apart for r);
  - two parallel lines, or two concentric arcs (later, if wanted, the half circle across
    the gap);
  - a tangent point outside where the curve may go (§4).

  The message gives the most radius that fits, when there is one.
- **The most that fits:** for a corner between two lines it is worked out directly, as
  today. Beside an arc there is no simple formula, so it is found by halving the interval
  to the chaining tolerance (0.001 mm). The search is deterministic, the same steps every
  time.

The geometry goes in `chipload-core`'s `corners` module, beside the line–line rounding.
`chipload-job` (a polyline's segments), `chipload-draw` (the editing tools) and the
pipeline all call it, so a corner rounded in a table and one rounded by a click are the
same arc.

## 3. Which round: where you click

Each click picks a curve and the part of it to keep (decision 2). The round is the one
that:

1. **has its centre on the same side of curve 2 as click 1, and on the same side of
   curve 1 as click 2.** For a line the side is left or right; for an arc it is inside or
   outside its circle. Two lines crossing give four corners, and the clicks pick one of
   them. A line and an arc give one sign for each offset.
2. **of the (at most two) centres left, has its tangent points nearest the clicks** (the
   sum of the two distances).

**Inside one polyline** the two clicks are implied: the round sits where the path turns
at the corner and touches the sides before and after it. Of the two centres left, it is
the one whose tangent points lie nearest the corner.

## 4. A corner beside an arc, in one polyline (and so in a closed one)

- **The *Radius* column** (polyline-radii.md §4) is no longer greyed beside an arc. A
  radius there rounds the corner by the round of §3: tangent to the line and the arc, or
  to both arcs.
- **The points stay at the sharp corners,** as for straight sides. An arc side keeps its
  bulge, which still means the arc between the two points. Where a corner is rounded, that
  arc is cut back along its own circle to the tangent point.
- **Not a corner:** where the sides already meet tangentially (the path doesn't turn), or
  where the path turns straight back. A radius there is refused, as today.
- **Room:** the two corners on one side mustn't pass each other along it. For an arc side
  this is measured along the arc. An arc side used up entirely is refused, with the most
  that fits.
- **The Fillet tool on such a corner** sets its radius, as on any other corner. Shift+click
  rounds every corner the shape has, beside arcs too. **Chamfer** still does only corners
  between straight sides (decision 6).
- **Format** (decision 5): `radii` itself doesn't change, but a radius beside an arc was
  refused until now. Chipload 0.39 would open such a project with a confusing complaint
  ("only where both sides are straight"). With **schema 43**, it refuses the file as one
  from a newer version instead. The migration from 42 changes nothing in the data. It
  ships with a frozen v42 sample and its golden test.

## 5. Between two shapes

**What can be picked:** a side of a drawn open polyline, which includes a drawn line or
arc. A closed shape, a circle or a spline can't be picked (decision 4). Neither can an
imported shape: *Explode* it first, as for the other editing tools.

**In the window,** with the Fillet tool, near a corner of the shape under the cursor the
corner is marked, and a click rounds it as today. Elsewhere on a side:

1. **Click the first shape** on the part to keep. It is marked, and the status bar asks
   for the second.
2. **Move to the second shape.** The round, and the two shapes trimmed or extended to it,
   are previewed under the cursor. If no round fits, the status bar says why.
3. **Click the second shape** on the part to keep. This is one undo step.

Esc, or a click on empty space, forgets the first pick. A click on the first shape again
picks it anew, somewhere else. The radius is the status bar's *Radius*, as today.

**Each shape, trimmed or extended:**

- **Trimmed:** the tangent point lies on the picked side. What lies beyond it, away from
  the click, goes, including the polyline's further sides when the picked side is in its
  middle.
- **Extended:** the tangent point lies beyond the polyline's end, on the end side's
  continuation. A straight end side grows along its line; an arc end grows along its
  circle, less than a full turn. The *Extend* tool still does lines only (TODO's
  "extending arc ends along their circle" is its own item).
- **Refused:** the tangent point lies on the continuation of a side that isn't an end
  side ("the round would need side 3 longer").

**The round itself:**

- It is a new open polyline of one arc, on the first shape's layer, construction geometry
  if that shape is.
- **Joined as after a Trim** (decision 3): the shapes then join, through the same code
  (`joined_where_ends_meet`), with the drawn open polylines on that layer whose ends meet
  theirs. Two open polylines on one layer therefore become one, keeping the lowest number.
  Different layers, or construction geometry, stay apart.
- **A radius where it is editable:** where the joined round lies between two straight
  sides, it is kept as that corner's radius. The corner's point is where the two lines
  meet, and the radius is r, so the table shows the corner and its radius rather than an
  arc. Beside an arc, it becomes a radius too (§4).

**Radius 0:** the two are trimmed or extended to meet in a sharp corner, and joined as
above. This is the classic way to tidy two lines into one corner. The *Radius* field
accepts 0 for this. On a corner of a polyline, rectangle or polygon, 0 makes it sharp
again.

**Chamfer between two shapes:** the same clicks, with the *Distance* measured back along
each line from where the two meet. Lines only (decision 6). The chamfer is joined like a
round, as a short straight side.

## 6. Command line

```
chipload shape fillet P 3 --radius 5 --corner 2       # as today; now also beside an arc
chipload shape fillet P 3 --radius 0 --corner 2       # that corner sharp again
chipload shape fillet P 3 --with 4 --radius 5 --keep 10,0 --keep 0,10
chipload shape chamfer P 3 --with 4 --distance 2 --keep 10,0 --keep 0,10
```

- `--keep X,Y` (twice, in the shapes' order): a point near each shape's part to keep. It
  plays the role of the clicks: the nearest point on the shape picks the side.
- `--with` and `--corner` exclude each other.
- With `--json`: the shapes changed, removed and made.

## 7. Tests

- **Core (geometry):**
  - line–line through the general code, giving the same arcs as today;
  - line–arc and arc–arc, the round outside and inside the arc;
  - the four corners of two crossing lines, each picked by its clicks;
  - of two crossings, the nearer one;
  - refusals: inside an arc with r ≥ R, too far apart, parallel lines, concentric arcs;
  - the most that fits, found by search, within the tolerance of the true limit;
  - a property test: the round is tangent to both curves at its ends (G1) and lies r from
    its centre, within the tolerances.
- **Polylines (job):**
  - a radius beside an arc: its segments, and the arc cut back on its own circle;
  - validation: tangent sides, a turn straight back, two corners passing on an arc side;
  - the schema 43 migration, with a frozen v42 sample giving the same G-code;
  - Fillet on such a corner; Shift+click doing all of them.
- **Two shapes (pipeline):**
  - lines that meet, cross, and fall short;
  - a line and an arc, the arc trimmed and extended;
  - a middle side picked and trimmed; a refused middle side;
  - joined when on one layer, apart when on different layers or construction geometry;
  - between two lines, the joined corner kept as a radius;
  - radius 0 giving a sharp corner;
  - chamfer; chamfer refused beside an arc.
- **CLI:** `--with`/`--keep`, `--radius 0`, the refusals' messages, `--json`.
- **Window:**
  - corners marked and rounded without selecting first;
  - the two clicks with the preview, Esc forgetting the first, one undo step.
- **Golden:** a bracket with line–arc rounds, profiled and passing the checker. No other
  G-code changes (no saved polyline has a radius beside an arc).

## 8. Decisions (recommendations first)

1. **Both features in one piece of work.** The same geometry serves the polyline corner
   beside an arc and the fillet between two shapes.
2. **The parts to keep are where you click** (the side rule of §3), rather than asking
   which of the rounds after the fact.
3. **The shapes join after a fillet as they do after a Trim:** on one layer they become
   one polyline. Between straight sides, the round is kept as the corner's radius.
   *Alternative:* never join (*Join ends* does it), or join without the radius.
4. **Closed shapes** (a circle, a rectangle) **can't be one of the two** in this step. The
   round would cut into a shape that has to open to take it, turning it into a polyline.
   Fillets of a polyline with itself (its two ends) are left out too. Later, if wanted.
5. **Schema 43** for radii beside arcs, so 0.39 refuses such a file as newer rather than
   with a confusing complaint. *Alternative:* stay on 42 (the file isn't misread, only
   refused with that complaint).
6. **Chamfer between two shapes comes with it, for lines only.** A chamfer beside an arc,
   in a polyline or between shapes, is left out, since its distance along a curve has no
   one obvious meaning.
7. **Fillet and Chamfer work on the shape under the cursor,** with no selection first,
   like Trim. With a shape selected, its corners still come first, so selecting first
   keeps working.
8. **Radius 0 is allowed:** it trims or extends two shapes to a sharp corner, and makes a
   corner of a polyline, rectangle or polygon sharp again.

## 9. Order of work

Each step is one commit, with the guide (drawings/shapes.md), the seven languages and the
tests.

1. **Core:** the round between two curves, the side rule, the most that fits by search
   (§2, §3).
2. **Polylines:** radii beside arcs, schema 43, the table, Fillet on such corners (§4).
3. **Two shapes:** the pipeline and `shape fillet/chamfer --with --keep` (§5, §6).
4. **The window:** corners without selecting first, the two clicks, the preview (§5, §7).

## 10. As built

- **Step 1:** `chipload_core::corners::round`: `Curve` (a line or a circle), `rounds` (on
  given sides), `picked_round` (the clicks' sides, §3), `nearest_round` (a corner),
  `most_that_fits` (doubling, then halving to the chaining tolerance).
- **Step 2:** `chipload_core::corners` rounds a corner beside an arc through
  `nearest_round`, the round on the inside of the turn, the arc side cut back along its
  circle; between straight sides it keeps the direct formula, so no G-code changed. The
  most that fits beside an arc is searched for. Schema 43 (the migration only bumps the
  version; v42's frozen samples migrate through it), golden `v43_bracket`. The Fillet tool
  marks corners beside arcs, Chamfer still only straight ones; *Radius* 0 makes a corner
  sharp again (on the command line too).

