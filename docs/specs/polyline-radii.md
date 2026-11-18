# Spec: a radius at each corner of a polyline

Status: **draft, 2026-11-18**, for review (decisions in §8).

User request (2026-11-18): "a polyline with tangent radiuses is nice if it is a point list,
and beside each point (not end points) is an option to add a radius, which becomes tangent.
So it becomes like a table, like it is now but with an extra column for radius."

## 1. What changes

- **A polyline gets a radius per point:** 0 is a sharp corner (as now). A radius rounds the
  corner with an arc tangent to both sides.
- **The points stay where they are:** they are the corners as if sharp, so the table keeps
  the numbers you drew and measured. Change a radius and only that corner changes.
- **Ends of an open polyline have no radius.** Every point of a closed one can have one.
- **Today's Fillet tool** cuts the arc in for good: the corner point goes, two points and an
  arc come. With this, rounding a polyline's corner sets its radius instead, so it stays
  editable (decision 3).

## 2. Format (schema 42, not yet released)

```json
{"id": 9, "kind": "polyline", "closed": true,
 "points": [[0,0], [100,0], [100,60], [0,60]],
 "bulges": [0, 0, 0, 0],
 "radii":  [0, 5, 5, 0]}
```

- **`radii`:** optional, one per point, mm, 0 for sharp. It is left out when every radius
  is 0, so existing polylines are written as before.
- **No new schema:** schema 42 (splines) hasn't been released, so the field joins it.
  Chipload 0.38 refuses the file as from a newer version, as it would anyway. If 0.39 is
  released before this is built, it takes schema 43 instead.
- **Validation:**
  - one radius per point, each finite and 0 or more;
  - an open polyline's end points have 0;
  - a radius only where both sides are straight (bulge 0), decision 2;
  - the tangent points of the two corners on a side mustn't pass each other. The message
    gives the most that fits, as the Fillet tool's does.

## 3. The geometry

For a corner P between straight sides towards A and B, turning by θ (the angle between the
sides' directions):

- **Where the arc touches each side:** r / tan((π − θ) / 2) back along it from P.
- **The arc:** from the tangent point on the incoming side to the one on the outgoing side,
  turning by θ, bulging away from P.

The segments are the sides shortened at both ends, with the corners' arcs between them:
exact arcs, so the G-code has G2/G3 at every rounded corner. The calculation moves into
`chipload-core` (a function beside `fit`). `chipload-job`'s `Shape::segments` and
`chipload-draw`'s Fillet tool both use it, so the two always agree.

## 4. In the window

- **The form's points table gets a column *Radius* after *Bulge*:**
  - a length field per point, empty at an open polyline's ends;
  - greyed where a side next to the point is an arc, with a hover that says why.
- **Insert and delete points:** an inserted point starts sharp; deleting a point drops its
  radius.
- **Handles:** they stay on the points, the sharp corners. A rounded corner's handle sits
  just outside the shape, with a faint dashed line from it to where the sides meet the arc,
  so it's clear what it controls.
- **Lengths on the canvas:** a side's length is still from point to point (the sharp
  corners).
- **While drawing a polyline:** a *Radius* field on the status bar (0 to start with, kept
  for the session) gives every new corner that radius. A rounded frame or bracket is then
  drawn in one go (decision 4).
- **The Fillet tool on a polyline corner** sets that corner's radius, and Shift sets every
  corner's. On other kinds it works as today. Chamfer is unchanged (decision 3).

## 5. Everything else

- **Editing tools:** offset, trim, union and the rest work on the segments, arcs included,
  and give polylines without radii, as for any other curve today.
- **Mirror, move, turn, scale, copy:** these keep the radii. Even scaling scales them.
- **DXF export:** writes the segments, so the arcs go out as bulges, which every CAD reads.
- **Explode, Join ends:** the radii become ordinary arc segments when a polyline is joined
  to another, since a joined corner isn't one of its points any more.

## 6. Command line

```
chipload shape add P polyline --points "0,0 100,0 100,60 0,60" --radii "0 5 5 0"
chipload shape set P 9 --radii "0 8 8 0"
chipload shape fillet P 9 --radius 5 --corner 1     # now sets corner 1's radius
```

## 7. Tests

- **Core:**
  - a corner rounded at 90°, at an acute and an obtuse angle;
  - the arc tangent to both sides;
  - the most that fits;
  - two corners on a side meeting exactly.
- **Job:**
  - a round trip with and without radii (no `radii` written when all are 0);
  - validation: ends, arcs beside, too large, negative;
  - segments for open and closed polylines.
- **Window:**
  - the table's column;
  - the status bar radius while drawing;
  - Fillet setting the radius;
  - handles.
- **CLI:** `--radii` on add and set; fillet on a polyline.
- **G-code:**
  - a new golden of a closed polyline with rounded corners, profiled and pocketed, which
    passes the checker;
  - the existing goldens stay the same, since no saved polyline has radii.

## 8. Decisions (recommendations first)

1. **A radius per point, stored beside the points** (`radii`), the points staying at the
   sharp corners. The alternative is today's way: arcs as bulges between new points, which
   is exact but not editable as a radius.
2. **Radii only between straight sides.** The alternative is rounding where a side is an
   arc too (line–arc fillets), which is item 12f's larger fillet work.
3. **Fillet on a polyline sets the radius**, keeping it editable. Chamfer stays as it is.
4. **A *Radius* field on the status bar while drawing a polyline.**
5. **The field joins schema 42**, which isn't released yet, rather than a schema 43.
