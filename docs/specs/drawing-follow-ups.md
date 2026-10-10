# Spec: drawing follow-ups (handles, snapping, arcs in polylines)

Status: **approved 2026-10-10** (Q1–Q4 as recommended).

Builds on docs/specs/drawing-tools.md, which left these out: dragging handles on the canvas,
snapping to existing geometry, and arcs in polylines.

## 1. Handles (top view)

When the selection is one drawn shape, its handles are drawn as small squares; dragging one
changes the shape's defining values, through the same validation as the form:

| Shape | Handles |
|---|---|
| Circle | centre (moves it), a point on the circle (changes the radius) |
| Rectangle | centre (moves it), each corner (resizes, the opposite corner stays put; keeps the angle), a rotation handle beyond one end (turns it about the centre) |
| Polyline | each point (moves it), the middle of each segment (drag to insert a point there; drag sideways with Shift to bend it into an arc, §3) |

- One drag is one undo step (the existing commit after the pointer is released).
- A drag that would make the shape invalid (radius 0, point on top of its neighbour) stops at
  the last valid position; the form shows the reason.
- Handles only in top view; in 3D, the form still edits values.
- Imported shapes have no handles (read-only, as before).

## 2. Snapping

While drawing (§ drawing-tools 6) and while dragging a handle, the cursor snaps:

1. **To geometry** first, within 8 px on screen: the end points of every contour segment,
   the centres of circles and arcs, and the middles of segments. The snap point is marked
   (a small diamond) and the status bar names it ("end of #4", "centre of S7").
2. Otherwise **to the grid**, as now.
- Hold **Alt** to switch snapping off (free position, no rounding).
- Snapping picks the nearest candidate on screen; ties go to end points, then centres, then
  middles, then the lowest contour or shape id (deterministic).

## 3. Arcs in polylines

A polyline segment can be an arc, given by its **bulge** (as in DXF: tan(sweep / 4); 0 is a
straight line, 1 a half circle, negative bends the other way):

```json
{"kind": "polyline", "closed": true, "points": [[0, 0], [40, 0], [20, 30]],
 "bulges": [0.0, 0.4, 0.0]}
```

- `bulges` is optional (missing: all straight, today's meaning), one per segment: as many
  as points for a closed polyline, one fewer for an open one. Optional with a safe default,
  so no schema bump.
- Rule: |bulge| ≤ 1 (at most a half circle per segment); finite.
- Segments with a bulge become arcs exactly as DXF bulges do (`Arc::from_bulge`), so they
  machine like imported bulged polylines, with G2/G3 output.
- GUI: a *bulge* column in the point table, and the Shift-drag of a segment's middle handle
  (§1). CLI: `chipload shape add/set … --bulges "0 0.4 0"`.

## 4. Command line

Snapping and handles are GUI interactions; everything they produce is a shape value the CLI
already sets (`chipload shape set`). Bulges get the `--bulges` option.

## 5. Tests

- Handles (GUI-free, on the shape values): dragging each handle of each kind gives the
  expected values; the opposite rectangle corner stays fixed (also when rotated); a rotate
  drag changes only the angle; invalid drags stop at the last valid shape.
- Snapping: candidates from a known drawing; nearest wins; ties by the order in §2; Alt
  switches off; grid used when nothing is near.
- Bulges: a polyline with a 0.4 bulge gives the same arc as a DXF LWPOLYLINE with that
  bulge (same segments); |bulge| > 1 and wrong counts refused; round trip; a pocket of it
  passes the checker with G2/G3.
- Golden G-code unchanged.

## 6. Decisions (approved 2026-10-10)

1. **Handles** as in §1, top view only, one undo step per drag. **Yes.**
2. **Snapping** to end points, centres and middles within 8 px, then the grid; Alt turns it
   off. **Yes.**
3. **Bulges** for polyline arcs (optional `bulges`, |b| ≤ 1, DXF meaning), edited in the
   table and by Shift-dragging a segment's middle. **Yes.**
4. **Rectangle corners** resize with the opposite corner fixed (rather than about the
   centre). **Opposite corner fixed**, as in most drawing programs.
