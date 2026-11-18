# Spec: fillets at line–arc corners and between two shapes

Status: **draft, 2026-11-18**, for review (decisions in §6). The first item of step 2 of
the drawing review (TODO 12f).

Builds on drawing-editing.md §3 (Fillet and Chamfer) and polyline-radii.md (a corner's
radius kept as a value).

## 1. What is missing today

- **Corners beside an arc:** a corner between a line and an arc, or between two arcs,
  can't be rounded (drawing-editing.md decision 3; polyline-radii.md decision 2). A
  slotted bracket, a bottle outline or a lever often has exactly those corners.
- **Two separate shapes:** the Fillet tool only works inside one shape. In CAD, the
  everyday fillet is between two lines (or a line and an arc) that meet, cross, or don't
  quite reach each other: both are trimmed or extended to the round, which joins them.

## 2. The geometry (general, from first principles)

A round of radius r tangent to two curves, each a line or an arc:

- **The round's centre:** where the two curves offset by r towards the round's side
  cross. A line's offset is a parallel line; an arc's is a concentric arc of radius
  R ± r.
- **The tangent points:** the feet of the centre on each curve. The arc runs between them
  the short way round.
- **Which of the (up to four) solutions:** the one on the sides the user means, given by
  the parts of the two curves to keep (§3). A corner of a polyline keeps the parts before
  and after the corner.
- **Refused:** no such centre (r too large for an arc's curvature or for the room the
  curves give), or tangent points off the curves' ends after trimming. The message gives
  the most radius that fits where one can be found, as today.

The geometry goes in `chipload-core` beside the corner rounding (`corners`), so a
polyline's segments use it too, and the editing code in `chipload-draw` calls it.

## 3. In the window

**A corner beside an arc, in one polyline:** the *Radius* column (polyline-radii.md) is no
longer greyed beside an arc. A radius there rounds the corner the same way, tangent to the
line and the arc (or both arcs), and the arcs are shortened to meet it. Fillet on such a
corner sets its radius, as for any other.

**Between two shapes:** with the Fillet tool and nothing selected:

1. Click the first shape on the part to keep.
2. Click the second shape on the part to keep. The preview of the round, and of the
   trimmed or extended ends, follows the cursor before the second click.
3. The radius is the status bar's *Radius*, as today.

Then:

- **Two open drawn polylines (lines and arcs) on one layer:** they join into one polyline
  with the round between them, keeping the first one's number; the round is its corner
  radius where both sides are straight, else an arc segment.
- **Otherwise** (different layers, or one is closed): both are trimmed or extended to the
  tangent points where they can be, and the round is a new open polyline of one arc on the
  first shape's layer.
- **Radius 0:** the two are trimmed or extended to meet in a sharp corner (and joined, as
  above). That is the classic way to tidy two lines into a corner.
- **Chamfer between two shapes** works the same, with its distance.

## 4. Command line

```
chipload shape fillet P 3 --radius 5 --corner 2              # as today; now also beside an arc
chipload shape fillet P 3 --with 4 --radius 5 --keep 10,0 --keep 0,10
chipload shape chamfer P 3 --with 4 --distance 2 --keep 10,0 --keep 0,10
```

`--keep` gives a point near the part of each shape to keep, in the order of the shapes.

## 5. Tests

- **Core:**
  - line–line (as today, now through the general path, the same results);
  - line–arc and arc–arc, convex and concave;
  - each of the four solutions picked by the kept parts;
  - refusals with the most that fits;
  - a property test that the round is tangent (G1) at both ends and stays within the
    tolerance of r from its centre.
- **Polylines:** a radius beside an arc in the table; segments and validation; Fillet on
  such a corner.
- **Two shapes:**
  - lines that meet, cross, and fall short;
  - a line and an arc;
  - joined when both are open polylines on one layer, separate otherwise;
  - radius 0 makes a sharp corner;
  - chamfer.
- **CLI and window:** `--with`/`--keep`; clicks with the preview; undo.
- **Golden:** a bracket with line–arc rounds, profiled; no other G-code changes.

## 6. Decisions (recommendations first)

1. **Both features in one piece of work:** the same geometry serves the polyline corner
   beside an arc and the fillet between two shapes.
2. **The parts to keep are where you click,** as in CAD programs, rather than asking which
   of four rounds after the fact.
3. **Two open polylines on one layer join into one;** otherwise the round is a shape of
   its own. Alternative: never join (then *Join ends* does it).
4. **Radius 0 trims or extends to a sharp corner**, the classic way to tidy two lines into
   one corner.
5. **Closed shapes** (a circle, a rectangle) **as one of the two:** not in this step. The
   round would cut into a shape that has to open to take it, which turns it into a
   polyline. Later, if wanted.
6. **Chamfer between two shapes comes with it,** the same flow with a distance.
