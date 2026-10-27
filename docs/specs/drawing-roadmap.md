# Proposal: drawing in Chipload, self-sufficient

Status: **approved 2026-10-27** (the user: "sounds good, go ahead"; every decision in §9 as
recommended). Each phase gets its own spec before it is built: phase 1 is
docs/specs/drawing-input.md, phase 2 docs/specs/drawing-editing.md. SVG import added as
phase 3 on 2026-10-27 (the user, after a review of dependencies); parameters and export
moved to phases 4 and 5.

The user (2026-10-27): Chipload was meant to cut premade DXFs from a CAD program, but users
ask for more and more drawing. The app should be self-sufficient for creating geometry, in a
clean and efficient way.

## 1. Today

What a user can draw and do without a CAD program:

- **Shapes:** circle, rectangle (with a corner radius), polyline (lines and arcs by bulge),
  text in 17 fonts, pictures traced into shapes or centre lines.
- **Drawing:** clicks rounded to a zoom-dependent grid; snapping to end points, centres and
  middles (8 px); Alt for free; exact values typed afterwards in the form.
- **Editing:** handles per shape kind; a box with move, turn and size round several drawn
  shapes; Position and Turn in Properties; copy, paste, mirror; join close ends; explode a
  DXF into drawn shapes; names and layers; undo. Every one of them on the command line too.
- **Missing** (what users run into): typing a length or angle *while* drawing; arcs, slots
  and polygons as tools; offsetting a shape; rounding one corner; trimming at a crossing;
  adding or subtracting shapes; rows and circles of copies of a shape; snapping to
  intersections and tangents; selecting with a box; saving the drawing as a DXF.

The base is good: shapes keep their defining values, ids never change under editing, so
operations follow; geometry rules are pure functions with tests; the CLI comes first.
What's missing is mostly **precise input** and **editing operations**, not more shape kinds.

## 2. The scope line (ADR 0017 amended)

"CAD beyond the few shapes Chipload draws" moves. Proposed wording:

> Chipload draws the **2D geometry of parts it cuts**: everything an outline, a pocket, a
> hole pattern or a sign needs, drawn exactly and edited quickly. Out stay what CAD does
> for other purposes: 3D, assemblies, a geometric constraint solver, drafting for print
> (dimensions, hatches, title blocks), splines and ellipses as drawing tools (they import
> fine), and any file format besides DXF and SVG (SVG import added 2026-10-27, §6).

The test for each new drawing feature: *does a maker drawing a part to cut need it often
enough that leaving for a CAD program would be the common case?* Fillets, offsets and
booleans pass; a constraint solver doesn't (it's a project of its own, and parameters, §6,
cover the everyday case: "the stock got thicker").

## 3. One way of working: the drawing command

Today each tool has its own click rules. Proposed: one pattern for every tool, so learning
one teaches all:

1. **Start** a tool: ribbon, Draw menu or a one-key shortcut (L polyline, C circle,
   R rectangle, A arc, S slot, N polygon; F, T, G, M and P are taken by the view). Space
   repeats the last tool. Esc cancels; a second Esc clears the selection.
2. **The status bar prompts** the next step ("Centre of the circle", "Radius, or a point on
   the circle").
3. **Click, or type.** While a tool waits for a point or a value, typing opens a small entry
   at the cursor: `40` is a length along the rubber band, `40<30` length and angle,
   `@20,10` a point relative to the last one, `20,10` an absolute point; Tab moves between
   fields (length, angle). Units follow the project display, and any value takes a unit
   (`1/2in`, as the CLI already does).
4. **Live feedback:** the rubber band shows its length and angle beside the cursor; Shift
   holds it to 15° steps (ortho is the 0/90 case).
5. **Selection-first works too:** with shapes selected, an editing tool (offset, fillet,
   array) applies to them at once and asks only for its value.

Every finished command is one undo step and one CLI-equivalent change to the project; the
command line remains the reference for what a tool does (rule 5).

## 4. Phase 1: precise input and the missing basic tools

The biggest gain for the least code; no new shape kinds except perhaps the polygon (§9 Q4).

- **Typed values while drawing** (§3), for every existing tool and handle drag.
- **More snaps:** intersections (segment with segment), nearest point on a segment,
  quadrant points of circles and arcs, perpendicular and tangent from the last point. Same
  rules as today: 8 px, nearest on screen, ties by a fixed order, Alt off. The snap point
  computation moves out of the GUI into a library (§8).
- **Box selection:** drag on empty canvas; left to right takes shapes wholly inside, right
  to left also those it touches (the convention users know). Shift adds, Ctrl toggles.
- **Moving with the mouse:** a drag that starts on a drawn shape moves it, or the whole
  selection when it is selected, snapping the grabbed point (the user, 2026-10-27).
- **New tools, producing existing kinds:**
  - **Arc**: three points, or centre–start–end. An open polyline with one or two bulged
    segments (a bulge is at most a half circle).
  - **Slot**: two centre points and a width. A rectangle whose corner radius is half its
    width, so it keeps editable length, width, angle (no new kind). The form shows it as a
    slot while the radius stays at its maximum.
  - **Polygon**: centre, corner (or flat) distance, number of sides.
  - **Line** is the polyline tool; one segment is an open polyline (no separate kind).
- **Lengths on the canvas** for a selected polyline's segments and a circle's radius, as
  labels; clicking a label types a new value (the form's field, on the canvas).

## 5. Phase 2: editing operations

Each is a pure function on shapes, on the command line first (`chipload shape …`), then a
tool. Results are drawn shapes with new ids; what happens to the inputs is per operation:

| Operation | What it does | Inputs afterwards |
|---|---|---|
| **Offset** | a shape at a distance, inside or outside, corners round or sharp | kept |
| **Fillet / chamfer** | rounds or cuts one corner (or every corner) of a polyline by a radius or length | replaced (same id) |
| **Trim / extend** | cuts a segment back to the nearest crossing, or extends it to the next shape | replaced (same id) |
| **Break** | splits a shape at a point, or at its crossings with another | first part keeps the id |
| **Union, subtract, intersect** | closed shapes combined into new closed shapes | removed |
| **Linear and circular array** | n copies at a spacing, or round a centre | kept |
| **Align and distribute** | left, centre, right, top, middle, bottom; even spacing | moved (same ids) |

- **Rectangles, circles and polygons** that an operation must change become polylines;
  the GUI says so once ("the rectangle becomes a polyline; its corners can then be rounded
  one by one"). Fillet on *every* corner of a rectangle is its corner radius, so stays a
  rectangle.
- **Offsets and booleans** use `cavalier_contours` (already a dependency; arcs stay arcs),
  and every result is verified as toolpath offsets are (distance, side, no crossing), with
  a panic inside the library becoming an error.
- **Operations follow:** a replaced shape keeps its id; a removed shape flags the
  operations that used it, as deleting does today. Union could pass the first input's id to
  the result, so a profile on it keeps working (§9 Q6).
- **Arrays** of drawn shapes are geometry; the job's grid of copies (`copies`) stays what
  it is: copies of a whole part with its operations.

## 6. Phase 3: SVG import

Logos and artwork come from Inkscape, Illustrator and the like as SVG, which many laser and
sign shops work in. SVG becomes a drawing source like a DXF or a picture: its paths become
shapes on layers (by group or colour), placed, turned and scaled like a DXF, updated when
the file changes, explodable into drawn shapes.

- **Reading through `usvg`** (Apache-2.0 OR MIT, the resvg project; an ADR when it comes):
  it resolves transforms, units, styles and shapes into plain paths. Its Bézier curves are
  fitted with lines and arcs as fonts' are (`chipload_core::fit`), within the arc
  tolerance.
- **Units:** SVG's own (px at 96 per inch, mm, in) where the file gives them; a file in
  plain px asks for the scale once, as a DXF without units asks for its units.
- Its own spec (what becomes a shape, fills versus strokes, text in SVGs, hostile input as
  for DXF), with a schema change for the new source.

## 7. Phase 4: parameters

Project values with names, used in any length field of a shape or an operation:

```
thickness = 18
slot_w    = thickness + 0.2
```

A rectangle's width `slot_w`, a pocket's end depth `thickness / 2`: change `thickness` and
the drawing and the depths follow. This is the leverage version of "parametric": no
solver, just values and arithmetic (+ − × ÷, parentheses, units), evaluated in order,
cycles refused. It needs a schema bump (a field holds a number or an expression), so it is
its own spec and decision. It also fits the CLI (`chipload set part.chipload
params.thickness=19`) and scripting.

## 8. Phase 5: getting the drawing out, and construction lines

- **Export DXF** (`chipload dxf export`, *File > Export drawing…*): drawn shapes as
  LWPOLYLINE (with bulges) and CIRCLE, on their layers; texts as their outlines. Once the
  drawing is made in Chipload, users will want it in a laser cutter, a quote request or a
  CAD program. Writing goes through the `dxf` crate already used.
- **Construction geometry:** a per-shape flag (or a layer setting) for lines that help
  drawing, snap like any shape, are drawn dashed, and are never contours or cut.

## 9. Where the code goes

- A new crate **`chipload-draw`** (core only, like `chipload-text`): snapping candidates
  (including intersections, tangents), trim, extend, break, fillet, chamfer, offset and
  booleans of drawing shapes, arrays, alignment. Pure, no I/O, compiles for wasm32.
- `chipload-pipeline::shapes` maps project shapes to and from it, and applies results to
  the project (ids, flagged operations), as it does for move/turn/scale today.
- The GUI keeps only the interaction: the command state machine (today's `draw.rs`
  `Sketch`, extended to typed input), previews and prompts. Snapping logic moves out of
  `chipload-gui/src/draw.rs` into `chipload-draw`, which rule 6 asks for anyway.
- Tests: unit tests per operation at its edges (tangent fillet larger than the segment,
  trim with no crossing, offset that vanishes, boolean of touching shapes); `proptest`
  invariants (an offset never crosses its shape; union contains both inputs; a trim never
  adds length); golden G-code unchanged; one new golden for a part drawn entirely in
  Chipload.
- The user guide gets a *Drawing* chapter rewritten around the command pattern (§3).

## 10. Decisions

1. **Scope:** amend ADR 0017 as in §2 (2D geometry of parts, no solver, no drafting, no
   splines/ellipses as tools). *Recommended: yes.*
2. **One command pattern** for all tools (§3), with typed values and one-key shortcuts.
   *Recommended: yes; it's the base for everything after.*
3. **Order:** phase 1 (input), 2 (editing), 3 (parameters), 4 (export, construction).
   *Recommended: yes. Phase 4's DXF export is small and could come earlier if users ask.*
   2026-10-27: SVG import inserted as phase 3; parameters are phase 4, export and
   construction lines phase 5.
4. **Polygon:** a new kind with its values (centre, radius, sides, angle; schema bump), or a
   polyline once drawn. *Recommended: a new kind, so the number of sides stays editable,
   as the rectangle's size does.*
5. **Slot as a rectangle** with corner radius at half the width, rather than a new kind.
   *Recommended: yes.*
6. **Ids after union/subtract:** the result takes the first selected shape's id, so its
   operations follow, or new ids and flagged operations. *Recommended: the first id.*
7. **New crate `chipload-draw`** for drawing geometry (§8), snapping moved into it.
   *Recommended: yes* (an ADR, and docs/architecture.md updated).
8. **Parameters** (§6) as a later spec, not decided now. *Recommended: yes, keep it in the
   plan.*
9. **Out:** constraint solver, dimensions for print, hatches, splines and ellipses as tools,
   3D. *Recommended: yes, written into ADR 0017.*
