# Spec: drawn splines

Status: **approved 2026-11-18** (the user: "start with step 1"; §10 as recommended).
All three steps built (see *As built*).

User request (2026-11-18): splines as well, after the drawing variants
(docs/specs/drawing-variants.md); "if possible, research it".

## 1. Research

**What drawing programs offer**, at the level of a feature:

- **Fit-point splines:** the curve passes through every point you click. Quick to draw and
  to hit a size: the points are on the part.
- **Control-point splines** (B-splines, NURBS): the points pull the curve without lying on
  it. Smoother, but the curve misses the points, so sizes are harder to hit.
- **Bézier pens:** each point with two tangent handles, as vector drawing programs have.
  The most control, and the most clicks and drags per point.
- DXF's SPLINE entity carries either fit points or control points.

**What the machine takes:** G-code has no common spline move. LinuxCNC has G5 and G5.1,
but GRBL and Mach3/4 have none. So a curve reaches the machine as lines and arcs, as DXF
splines, ellipses and SVG curves already do in Chipload, within `ARC_APPROX_TOL_MM`
(0.01 mm). Every operation then works unchanged, since offsets, pockets and V-carving run
on lines and arcs.

**What Chipload already has:**

- **Fitting:** `chipload_core::fit::fit_path` fits lines and arcs to dense points within a
  tolerance. Traced pictures use it.
- **DXF splines:** `chipload-dxf`'s `curves.rs` evaluates them (de Boor) and flattens them.
- **SVG curves:** `chipload-svg` fits cubic Béziers.

**Libraries:**

- `kurbo` (MIT/Apache-2.0) is already in the build, but only under egui. It has cubic
  Béziers, flattening and curve fitting.
- `lyon_geom` (MIT/Apache-2.0) has much the same.

The parts a drawn spline needs are short and well known: Catmull–Rom pieces as cubic
Béziers, evaluated and flattened, then the existing `fit_path`. **No new dependency is
recommended** (decision 1). Neither library would replace the fitting to arcs, which is
the part that matters for G-code.

**The curve:** a **centripetal Catmull–Rom spline** through the points (Yuksel, Schaefer
and Keyser, "Parameterization and applications of Catmull–Rom curves", Computer-Aided
Design 43(7), 2011). With the centripetal parameterization (α = ½), a piece never forms a
cusp or a loop. The curve also stays close to the polygon of the points, without the
overshoot the uniform form has. Each piece between two points is one cubic Bézier.

## 2. What the user sees

- **Spline** in the DRAW group and the Draw menu, key **B** (free; "bend"). Click points
  as for a polyline: Enter or a double-click finishes it open; clicking the first point
  closes it. Backspace drops the last point.
- **Drawing:** the preview is the curve through the points and the cursor. Snaps, typed
  values (`x,y`, `@dx,dy`, `d`, `d<a`, `<a`), Shift's 15° steps and the grid work as for
  the polyline.
- **Editing:**
  - A handle on every point; dragging one bends the curve through its new place.
  - The form is a table of X, Y with insert and delete per row, plus *closed*, as the
    polyline's.
  - Lengths on the canvas: none (a curve has no side lengths).
- **Everything a shape takes:** select, box, move, Compass (turn, copy, array), mirror,
  copy and paste, layers, construction, groups, operations (profile, pocket, follow path,
  V-carve…). Snaps on a spline: its points (as ends) and nearest.
- **Editing tools** (offset, fillet, trim, extend, break, union, subtract) work on its
  lines and arcs and give a polyline of arcs, as for a circle today. *Not built yet:*
  **Explode** to turn a spline into that polyline by hand (TODO 12e, with joining
  splines).

## 3. Format: schema 42

```json
{"id": 14, "layer": "0", "kind": "spline", "closed": false,
 "points": [[0.0, 0.0], [20.0, 15.0], [45.0, 5.0], [60.0, 20.0]]}
```

- **Validation:** an open spline needs at least 2 points, a closed one at least 3.
  Consecutive points must be more than `CHAIN_TOL_MM` apart, and every value finite.
- **Schema 42:** older Chiploads would refuse the kind as unknown; the bump makes them
  say "made by a newer Chipload". The migration from 41 to 42 is the identity. A frozen
  v41 sample is added; it migrates and gives its golden G-code unchanged.

## 4. The curve, exactly

For points P0…Pn (closed: wrapped round; open: the ends are extended by mirroring the
neighbour, so the curve starts and ends at P0 and Pn):

- **Knots:** t(i+1) = t(i) + |P(i+1) − P(i)|^½ (centripetal).
- **Pieces:** each from P(i) to P(i+1) is the Catmull–Rom piece over P(i−1)…P(i+2) at
  those knots, written as a cubic Bézier: control points P(i) + d(i)/3 and
  P(i+1) − d(i+1)/3, where d is the tangent from the knot differences (the published
  formula, §1).
- **Two points:** a straight line.
- **To segments:**
  1. Flatten each piece adaptively to within a tenth of the tolerance.
  2. Fit lines and arcs to the whole chain with `fit_path` within `ARC_APPROX_TOL_MM`.
  The cut is then within 0.01 mm of the exact curve.
  - A closed spline is fitted as a closed path; an open one starts and ends exactly on
    its first and last points.
  - Same input, same segments: libm maths, no hashing.
- **Where it lives:** the curve and its segments in `chipload-draw` (a new module
  `spline`), used by `chipload-job`'s `Shape::segments`.
  - That needs `chipload-job` to depend on `chipload-draw`, which only depends on core.
    The direction stays legal (job → draw → core).
  - Or the curve goes into `chipload-core` beside `fit`. That is simpler, with no new edge
    between crates (decision 5).

## 5. Moving, turning, scaling

The centripetal form depends on distances. Moving, turning, mirroring and **even**
scaling the points therefore gives exactly the same curve moved, turned, mirrored or
scaled.

**Stretching one way** would move the points and give a slightly different curve than the
stretched one, still smooth and through the points (decision 4). *As built:* drawn shapes
can only be scaled evenly today (`shape scale --factor`, the box's corner handles), so
this doesn't arise yet; it applies when stretching in X and Y comes.

## 6. Command line

```
chipload shape add part.chipload spline --points "0,0 20,15 45,5 60,20" [--closed]
chipload shape set part.chipload 14 --points "…"   # or --closed true/false
```

Plus `inspect`, `get` and `set` by path, the JSON Schema, and the MCP tools, as every kind
has.

## 7. DXF export

*Changed while building (the user, 2026-11-18):* a drawn spline exports as the lines and
arcs it is cut with, one POLYLINE with bulges, within `ARC_APPROX_TOL_MM` of its curve,
open or closed as it is. The exporter writes the classic R12 layout (`AC1009`,
dxf-export.md §A4), which has no SPLINE entity (that came with R13); strict readers may
refuse one in an R12 file. A true SPLINE would need an R2000 writer with handles and an
objects section, a later item. DXF import is unchanged: a DXF's SPLINE becomes lines and
arcs, as now.

*First planned:* a SPLINE entity of degree 3, its Bézier pieces as the control points and
each inner knot repeated three times, with `--segments` for lines and arcs.

## 8. Steps (one commit each)

1. **The kind:** format, schema 42, the curve and segments, CLI, golden G-code. The goldens
   are a follow path along an open spline and a profile and pocket of a closed one, plus
   a frozen v41 sample.
2. **The window:** the tool (key B), the preview, handles, the form, its icon, the Browser
   group *Splines*, texts in seven languages, the guide's drawing page.
3. **DXF export:** as lines and arcs (§7, changed while building).

## 9. Tests

- **The curve:**
  - it passes through every point;
  - pieces meet with the same tangent (G1);
  - no cusp for close points beside far ones (the centripetal property);
  - two points give a line;
  - a closed curve is smooth across its first point;
  - property test: the fitted segments stay within the tolerance of the exact curve, for
    random points.
- **Format:** a round trip; validation errors; v41 → v42; a newer file refused.
- **Similarity:** turned, mirrored and evenly scaled points give the curve turned,
  mirrored and scaled.
- **G-code:** the new goldens pass the checker; no other golden changes.
- **CLI and GUI:** add, set, the tool by clicks and typed values, handles, undo.
- **DXF export:** read back, the SPLINE gives the same points within the tolerance.

## 10. Decisions (recommendations first)

1. **No library:** write the curve ourselves (§1); kurbo and lyon_geom would only cover the
   easy part.
2. **Fit-point splines** (through the points, centripetal Catmull–Rom) first. Control-point
   splines and a Bézier pen with handles could come later as variants on the spline's
   split button.
3. **Smooth everywhere:** no sharp corner inside a spline. A corner is where two splines,
   or a spline and a line, meet. Alternative: a per-point "corner" flag.
4. **Stretching one way** moves the points and keeps the spline (§5), once stretching
   exists for drawn shapes; the guide will say so then.
5. **The curve in `chipload-core`** beside `fit`, so `chipload-job` needs no new
   dependency. Alternative: in `chipload-draw`, with job depending on it.
6. **DXF export:** *changed while building:* lines and arcs within 0.01 mm, since the R12
   file has no SPLINE (§7). First recommended: SPLINE (exact) by default, lines and arcs
   with `--segments`.
7. **Key B** for the spline tool.

## As built

- **Step 1 (2026-11-18):**
  - **The curve:** `chipload_core::spline`, with `beziers`, `point_at`, `flatten` and
    `segments`. The segments are flattened within a tenth of the tolerance and fitted by
    `fit_path` within the rest.
  - **The kind:** `Geometry::Spline { closed, points }`, schema 42 (`v41_to_v42`, the
    identity), validated as §3.
  - **Editing:** `edit` moves, turns, scales and mirrors the points. A point's handle drags
    it. The form's "+" inserts a point on the curve halfway along the next piece.
  - **Command line:** `shape add … spline`. A new spline is open unless `--closed`; a new
    polyline stays closed unless `--open`.
  - **The window:** it already shows, selects and edits a spline (its form with *Closed*
    and the points table, the Browser group *Splines*); the drawing tool is step 2.
  - **Tests:** the golden `v42_spline` (a pocket and a profile of a closed spline, a follow
    path along an open one) is new, with the frozen v41 sample `nesting_demo` for the
    migration. No other G-code changed; the frozen plan messages gained the new sample's
    lines only.
- **Step 2 (2026-11-18):**
  - **The tool:** `Tool::Spline`, key B, in the ribbon (after the polygon), the Draw menu
    and the shortcut list. It clicks as the polyline does (Enter or double-click open, the
    first point closes, Backspace). The preview is the curve through the points and the
    cursor, flattened within 0.02 mm.
  - **Snapping:** `Snapper::with_curves` marks a spline's fitted pieces, which offer no
    ends, middles, centres or quadrants, and no crossings where two of them meet. The
    spline's own points snap as ends, even where it chains with other open geometry;
    nearest, perpendicular and tangent stay.
- **Step 3 (2026-11-18):** no change to the exporter: a spline already went out through
  its segments as one checked POLYLINE. A test in `chipload-dxf` pins that down (open and
  closed, bulges, read back piece for piece); the guide's export page says so.
