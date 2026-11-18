# Spec: more ways to draw a shape, on one split button

Status: **approved 2026-11-18** (the user: "sounds good go ahead"; §9 as recommended).
Steps 1 and 2 built (see *As built*).

Guide: docs/book/src/drawings/shapes.md.

User request (2026-11-18, TODO item 12d): more drawing tools that sit on top of the existing
ones, for example a rectangle by its centre, by two points, or aligned (three points). They
share the shape's button, and a small arrow on it shows that there are alternatives, as
drawing programs usually show it.

Builds on drawing-tools.md (the tools, the grid), drawing-input.md (keys, typed values,
snaps, the readout) and `chipload_draw::construct` (arcs through points).

## 1. What changes, and what doesn't

- **Only the way a shape is clicked changes.** Every variant makes a shape kind that exists
  today: a rectangle (centre, length, width, angle), a circle, a polygon, or an open
  polyline with a bulge (an arc). **No schema change**, no migration, and no change to
  G-code or goldens. A shape drawn one way can't be told from the same shape drawn another
  way; afterwards it is edited as today (the form, handles, lengths on the canvas).
- **The command line is unchanged.** `shape add` takes each kind by its defining values
  (a rectangle by its centre, length, width and angle; a polygon by its radius or across
  flats), which is what every variant comes down to. The variants are ways to point, and
  pointing is the window's job. (CLAUDE.md rule 5 is met: every shape they make can be
  made from the command line already.)
- **Geometry stays out of the window** (rule 6): each new construction (the circle through
  three points, the aligned rectangle, the polygon on an edge, the arc by its centre) is a
  pure function in `chipload_draw::construct`, tested there. The window's `draw.rs` keeps
  only the clicks, the preview and the status text, as now.

## 2. The variants

The first variant of each shape is today's, and stays the default.

| Shape | Variant | Clicks | Makes |
|---|---|---|---|
| Rectangle | **Two corners** (today) | corner, opposite corner | a rectangle at angle 0 |
| | **Centre and corner** | centre, a corner | a rectangle at angle 0, twice the offset in size |
| | **Aligned** (3 points) | one end of an edge, its other end, then the width | a rectangle at the edge's angle |
| Circle | **Centre and radius** (today) | centre, a point on it | a circle |
| | **Two points** | two ends of a diameter | a circle |
| | **Three points** | three points on it | a circle |
| Arc | **Three points** (today) | start, end, a point on it | an open polyline of arcs |
| | **Centre, start, end** | centre, start (the radius), end (only its direction) | the same |
| Polygon | **Centre and corner** (today) | centre, a corner | a polygon (radius to the corners) |
| | **Centre and side** | centre, the middle of a side | a polygon (across flats: twice the distance) |
| | **Edge** | the two ends of one side | a polygon on the left of the side |
| Polyline, slot | (none for now) | | |

Details:

- **Aligned rectangle:** the first two clicks are one edge (length and angle); the third
  sets the width as the cursor's distance from that edge, on the cursor's side, as the
  slot's third step does. Stored as centre, length (the edge), width, angle (the edge's
  direction from the first click to the second).
- **Centre-and-corner rectangle:** length = 2·|dx|, width = 2·|dy|.
- **Two-point circle:** centre at the middle, radius half the distance.
- **Three-point circle:** the circle through the three points (the circumcircle, as
  `arc_through` finds it for arcs). Refused when the three lie on one line ("on a line").
- **Arc by centre, start, end:** counter-clockwise from the start to the end, or clockwise
  when the cursor went round clockwise. The arc follows the way the cursor has turned since
  the start was clicked, so moving round either way gives the arc on that side. The end
  click gives only the direction; the radius is the start's. Stored as today's arc (split
  in two when more than a half circle).
- **Polygon by the centre and a side:** the click is the middle of a side. Radius =
  distance / cos(π/n), angle so that the side's middle lies at the click. This is the way
  to draw a nut's hexagon by its across-flats size.
- **Polygon on an edge:** the two clicks are one side; the polygon lies on its left (the
  shapes run counter-clockwise), so clicking right to left puts it below. The preview shows
  which way it goes before the second click.
- **Refusals** as today, with the same status bar texts: no size (two clicks on one point,
  a zero width), three points on a line.

## 3. The split button

- In the ribbon's Draw group, the rectangle, circle, arc and polygon buttons each get a
  **small arrow in their lower right corner**. The button shows the **variant in use**: its
  icon, and its name on hover with the key.
- **Clicking the button** starts that variant. **Clicking the arrow**, or right-clicking
  the button, opens a small menu of the shape's variants, each with its icon and name, the
  one in use ticked. Choosing one starts it and makes it the button's variant.
  (egui has no split button; this is a small ribbon widget over `egui::Popup::menu`, as the
  style button's menu already is.)
- The arrow area is the button's lower right corner, about a third of its width; it is
  drawn as a small filled triangle, as drawing programs show it. The buttons keep their
  size, so the Draw group keeps its width.
- **The menu bar's Draw menu** lists the variants under each shape, as a submenu (Draw ▸
  Rectangle ▸ Two corners / Centre and corner / Aligned).
- **The variant in use is remembered for the session**, as the polygon's number of sides
  is (decision 3).

## 4. Keys and typed values

- **The shape's key** (R, C, A, N) starts its variant in use. **Pressing the key again**
  while the tool waits for its first click moves to the next variant (R, R, R cycles two
  corners → centre → aligned → two corners), and the status bar names it. After the first
  click the key does nothing, so a click is never reinterpreted. Enter with no tool active
  starts the last tool, in its variant.
- **Typed values** keep their grammar (drawing-input.md §2): `x,y`, `@dx,dy`, `d`, `d<a`.
  What each step reads, beyond a point:

| Variant | Step 2 | Step 3 |
|---|---|---|
| Rectangle, centre | `@dx,dy`: half the size (from the centre); `d`: a corner that far from the centre, towards the cursor | |
| Rectangle, aligned | `d`, `d<a`: the edge's length (and angle) | `d`: the width |
| Circle, two points | `d`: the diameter | |
| Circle, three points | points only | points only |
| Arc, centre | `d`: the radius (start towards the cursor) | `d<a`: the end at angle a (d ignored); `<a` alone too (new, §9 decision 5) |
| Polygon, side | `d`: the distance to the side, so across flats is 2·d | |
| Polygon, edge | `d`, `d<a`: the side's length (and angle) | |

- **Shift** keeps the line from the last point to 15° steps, as now, wherever a line is
  being drawn: the aligned rectangle's edge, the polygon's edge, the circle's diameter.
- **Snaps:** perpendicular and tangent need a last point (drawing-input.md §1). It is the
  first click for the aligned rectangle's edge, the two-point circle, the polygon's edge and
  the arc's start (from the centre), and the second click for the three-point circle's
  third point. Tangent snaps make "a circle touching a line" easy without constraints.

## 5. Readout and status bar

Beside the cursor, as now: the edge's length and angle while drawing an edge; length ×
width for rectangles; the radius for circles and arcs (the diameter too for the two-point
circle); the radius and across flats for polygons. The status bar names the variant and the
next click, for example "Rectangle (aligned): click the width. 0.5 mm grid".

## 6. Translations and the guide

- Every new text (variant names, hovers, status texts, the menu) in en, de, fr, es, it, sv,
  ru, as every window text is.
- **Guide:** drawings/shapes.md gets the variants per shape, the arrow, the key that
  cycles, and the typed values; the shortcut table notes the cycling.
- New icons for each variant (icons.rs draws them, as the others), and the guide's picture
  of the Draw group renewed.

## 7. Steps (one commit each, the guide and translations with each)

1. **The split button and the rectangle:** the ribbon widget, the Draw submenu, cycling by
   key, the variant in use; centre-and-corner and aligned rectangles.
2. **Circles:** two points and three points.
3. **Arcs and polygons:** the arc by its centre; the polygon by its side and on an edge.

## 8. Tests

- **Constructions** (`chipload-draw`): the circle through three points (and refused on a
  line); the aligned rectangle at 0°, 90°, 30° and with the third point on either side;
  the polygon on an edge (left side, both click orders, n = 3, 6), by its side (across
  flats); the arc by its centre both ways round, more than a half circle split.
- **Clicks** (the window's `draw.rs`): each variant from clicks and from typed values,
  refusals, the preview before the last click, the status texts.
- **Keys and the button** (app tests): the key cycles before the first click and not after;
  choosing a variant from the menu starts it and makes it the button's; Enter restarts the
  last variant.
- **Pictures:** the Draw group with its arrows in each language and the narrow window
  (`pictures`), so no name spills.
- Goldens unchanged (no new kind, no change to how shapes are cut).

## 9. Decisions (recommendations first)

1. **Write the tools ourselves; no drawing library.** Looked at 2026-11-18:
   - *Constraint solvers:* `ezpz` (Zoo, MIT, pure Rust) and `slvs` (SolveSpace's solver,
     through C, GPL-3.0). They solve "this line tangent to that circle, this length 40";
     they don't draw. That is parametric sketching, a much larger feature with its own
     model; worth its own spec and ADR if wanted, not needed for variants.
   - *Sketch editors:* `arael-sketch` (egui, constraint based, licence not stated, young)
     and `arcs` (a 2D CAD framework, last released 2020). Both are whole applications with
     their own model of shapes, selection and undo, which ours already has, wired to
     operations, ids and the project. The clean-room rule allows using a crate through its
     API, but not lifting its tools.
   - *Geometry:* `kurbo`, `geo`, `lyon` cover curves and tessellation that
     `chipload-core`, `chipload-draw` and `cavalier_contours` already cover.
   - The work in a variant is a dozen lines of construction plus clicks, a preview and
     texts; our snaps, typed values, grid and undo already serve every tool. A library
     would bring its own conventions to bridge, for less than it costs.
2. **The variants of §2**, with polyline and slot left as they are. (A tangent arc from the
   polyline's end, and a slot by its overall length, could come later.)
3. **The variant in use remembered for the session**, not across restarts, as the polygon's
   sides. Alternative: kept in the app settings (an optional field, no bump).
4. **The shape's key cycles its variants** before the first click (§4), rather than new
   keys per variant: no new keys to learn or collide.
5. **`<a` alone** as a new typed form (an angle without a length), for the arc's end by its
   centre. Alternative: only `d<a`, with d ignored, which reads oddly.
6. **The polygon on an edge lies on the edge's left**, chosen by click order, rather than a
   third click for the side.

## As built

- **Step 1 (2026-11-18):**
  - `chipload_draw::construct` has `Rectangle` (centre, length, width, angle) with
    `rectangle_by_corners`, `rectangle_by_centre`, `rectangle_aligned` and `left_of`.
  - The window's `Tool` has `RectangleCentre` and `RectangleAligned`, with `shape()`,
    `variants()` and `next_variant()`. `CanvasTools::variants` keeps each shape's variant
    in use. `draw_shape` is the key's action, and `start_drawing` records the variant.
  - The split button is `ribbon::split_button`: its arrow takes the lower right 38 % of
    the small button and opens an `egui::Popup::menu`. The Draw menu has a submenu for a
    shape with variants.
  - A typed width for the aligned rectangle goes to the cursor's side of the edge (to the
    left when the cursor is on the edge's line).
- **Step 2 (2026-11-18):** `circle_by_diameter` and `circle_through` in
  `chipload_draw::construct` (the latter through `arc_through`'s circumcentre); the tools
  `CircleTwoPoints` and `CircleThreePoints`, both with a last point for perpendicular and
  tangent snaps. The readout shows "Ø d  R r" for the two-point circle. Three points on
  a line are refused with their own text ("no circle goes through them").
