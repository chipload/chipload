# Spec: closed shapes as drawn, chaining per layer

Status: **approved** 2026-10-25 (decisions as recommended; the file may be used as a
test, decision 4). Built the same day.

## 1. The report

A user's DXF (`decoupe caisse_200mm-v3.dxf`, 2026-10-25) is "not read correctly: two
profiles on top of each other". It has a part's outline on layer `15mm`, and pockets on
layer `poche 7.5mm` that share edges and corners with that outline. Every one of them is
a closed LWPOLYLINE in the file. Chipload reads 26 of them as open contours, with 13
"Removed duplicate" and 10 "More than two segments meet at one point" warnings. None of
the outlines can be cut as drawn.

## 2. Why

Chaining (`chipload_core::chain`) puts every segment of the drawing into one graph, on
all layers together, and it does so even for shapes that are closed already:

- **Duplicates across layers:** a pocket's edge that lies on the outline counts as a
  duplicate of the outline's edge, and one of them is dropped. The outline then has a
  gap.
- **Branches across layers:** where a pocket's corner touches the outline, more than two
  segments meet. Chaining stops at such a point, so a closed polyline is cut into open
  pieces. Contour 11, for example, starts and ends at the same point and is still "open".
- **Contours across layers:** a chain may take segments from two layers. The corpus has
  none; the v0 spec refuses to select such a contour by layer as ambiguous.

This goes against the import rule "read the drawing as drawn". A closed polyline, a
circle or a closed drawn shape is a contour as it is, and a layer is the user's way of
saying which lines belong together (one part per layer: user-drawing-conventions).

## 3. The change

1. **A closed shape is a contour as drawn.** Closed LWPOLYLINE/POLYLINE, CIRCLE, closed
   ELLIPSE and SPLINE, closed drawn shapes, closed traced and text loops, and a path whose
   last point meets its first are each one closed contour. They are never cut at a point
   where another shape touches them, and never joined to other segments.
2. **Loose segments chain per layer.** LINEs, ARCs and open polylines chain only with
   segments of the same layer (ignoring upper and lower case, as selectors and joining
   compare layers). Branch points are counted
   per layer.
3. **Duplicates per layer.** A shape exactly on top of another shape on the same layer is
   still removed and reported. On different layers both stay: two operations may cut the
   same line (an outline and a pocket that ends on it).
4. A loose segment on top of an edge of a closed shape on the same layer is a duplicate
   (removed and reported, as today); the closed shape stays whole.
5. Contour numbering stays in drawing order: a contour's number is set by its first
   segment, as today.

Nothing is stored differently, so there's no schema change. Projects that load an affected
DXF see more closed contours and fewer warnings. Operations select by layer and by shape
id, which don't change, so they keep their shapes.

## 4. Decisions

1. **Closed shapes are never split or joined** (§3.1). *Recommended.* The other way,
   keeping one graph and only splitting it per layer, would fix this file but not a pocket
   on the outline's own layer touching it at a corner.
2. **Chaining per layer** (§3.2). *Recommended.* A contour drawn half on one layer and half
   on another becomes two open contours. The corpus has none, and the user's drawings keep
   one part per layer.
3. **Duplicates only within a layer** (§3.3). *Recommended.*
4. **The user's file goes into the corpus**, anonymised (its name and header removed, the
   geometry kept), with a failing test first: as `tests/corpus/dxf/shared_edges.dxf`.
   *Recommended*; the user who sent it agreed.

## 5. Tests

- Core: two closed squares sharing an edge on two layers, and on one layer, stay two
  closed contours; a closed square with a line touching its corner stays closed, and the
  line is its own open contour; an exact duplicate on the same layer is removed, on
  another layer kept; loose lines on two layers meeting at a point make two open contours.
- DXF: the user's file reads with no open contour where it has a closed polyline, and
  without the duplicate and branch warnings.
- Golden: every corpus project's G-code is unchanged (the survey on 2026-10-25 found no
  corpus DXF with a cross-layer contour, a branch point or a duplicate).
- A proptest: shuffling the order of closed shapes doesn't change which contours are
  closed.

## 6. As built (2026-10-25)

- `chipload_core::chain::chain_segments_with`: a run of consecutive segments of one shape
  (`Segment::shape`) that follow each other within the chaining tolerance and end at their
  start, with no segment twice, is a closed contour as drawn. Each layer has its own nodes
  (`LayerGraph`): closed shapes first, then loose segments, so a loose segment on a closed
  shape's edge is the duplicate. A closed shape is a duplicate only when every segment of
  it is. Contours stay numbered by their first segment in drawing order; duplicates are
  reported in drawing order.
- A shape that goes back over itself (a two-point closed polyline) still chains as loose
  segments, as before.
- The user's file: 26 open contours, 13 duplicates and 10 branch points before; no open
  contour, no branch point, and the 9 circles that are really drawn twice on the same
  layer, after. Nesting (outer or hole) still counts contours on every layer.
- The corpus's golden G-code is unchanged. The file is `tests/corpus/dxf/shared_edges.dxf`
  (a LibreCAD export with no personal data in it), tested by `chipload-dxf`'s
  `pockets_drawn_against_the_outline_on_another_layer_stay_closed`.
- Found with it: exploding the file dropped the bent slot's end arc, a half circle whose
  sweep came out a hair over π, so its bulge came out a hair over 1 and the polyline was
  refused. Explode now clamps such a bulge to ±1 (`explode::part_bulge`).
