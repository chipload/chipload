# Spec: move, turn and size drawn shapes; drawn shapes in the Browser, with names

Status: **approved** 2026-10-25 (the user: "go ahead with plan"; decisions in §5 as
recommended). Built in three steps, in this order.

The user's request (2026-10-25): a handle to move a polyline, and copy and paste it;
perhaps each polyline as a leaf under the Browser's *Polylines* group, so it can be
selected and named, and given a position and rotation.

## 1. Today

- **Copy and paste** of drawn shapes exists (Ctrl+C, Ctrl+V, *Draw > Paste*, `shape copy`;
  small-items.md §4).
- **Handles** (drawing-follow-ups.md §1): a circle has its centre (move) and radius; a
  rectangle its centre, corners and a turn handle; a polyline only a handle per point and per
  segment's middle. Nothing moves a polyline, or several shapes, as a whole.
- **Pictures and texts** have a box with move, turn and size handles (box-handles.md).
- **The Browser** shows drawn shapes as groups (*Rectangles*, *Circles*, *Polylines*), with
  no rows below them. Shapes have an id and a layer, no name.

## 2. Step 1: a box round selected drawn shapes

When the selection is drawn shapes (one or more, all drawn), other than a single circle or a
single rectangle (which have their own move and turn handles), the canvas shows a dashed box
round them with the handles of box-handles.md §2:

- **Move** at the centre, **turn** beyond the middle of the right side, **size** at the
  corners (even scaling, the opposite corner staying put).
- **The box** is the exact box round the shapes (arcs included), axis-aligned, **padded by
  a fixed distance on screen** (10 px), so its corners never sit on a polyline's own point
  handles (an exploded rectangle's corners would). Point and middle handles of a single
  polyline still show and still win where they overlap.
- **Steps** as for pictures and texts: the turn in 15° steps, Alt for 0.1° and no snapping;
  the move snaps the centre as the cursor does. A size drag has no rounding (shapes have no
  one size to round), and the box can't get smaller than 1 mm.
- **While dragging,** the shapes' outlines follow the box (they're cheap to draw); on release
  the shapes are set, as one undo step, keeping their ids, so operations follow.
- **The transform** maps the box at the start onto the box at the end: scale s, turn by a,
  centre to centre. Circle: centre mapped, radius × s. Rectangle: centre mapped, length,
  width and corner radius × s, angle + a. Polyline: every point mapped; bulges unchanged
  (a turn and an even scale keep every arc's sweep). Turns by multiples of 90° are exact.
- **Command line** (first, rule 5): `chipload shape move PROJECT IDS --dx --dy`,
  `chipload shape turn PROJECT IDS --angle-deg A [--about X,Y]` (default: the middle of
  their box), `chipload shape scale PROJECT IDS --factor F [--about X,Y]`. Lengths take a
  unit.

## 3. Step 2: position and turn in Properties

A selected polyline's form (and the Selection section for several drawn shapes) gets:

- **Position X, Y:** the lower-left corner of the shapes' box. Typing a value moves them
  there.
- **Turn:** ⟲ 90 and ⟳ 90 buttons, and a field *Turn by* (degrees) that turns them about
  the middle of their box when the edit is finished, then shows empty again.

Nothing new is stored: a polyline is its points (decision 2).

## 4. Step 3: drawn shapes in the Browser, with names

- **Rows:** under *Polylines*, *Circles* and *Rectangles*, one folder per layer, and under it
  a row per shape: its name, or its id ("S12") without one. A group with one layer shows the
  shapes directly. Folders start closed when a group has more than 20 shapes.
- **A click** selects the shape (Ctrl+click adds or removes), as clicking it on the canvas;
  the selected shapes' rows are highlighted.
- **Right-click:** *Select*, *Rename…* (focuses the name field in Properties), *Copy*,
  *Delete*.
- **Name:** a new optional `name` on drawn shapes in the project file (left out when there is
  none, so no schema bump), edited in the shape's form and with `chipload shape set --name`
  (`--name ""` clears it). Trimmed; an empty name is no name. Not unique: two shapes may share
  one. Imported shapes have none (a DXF can be exploded first). Shown in the Browser, the
  Selection section and `inspect`; not on the canvas.
- Copies start without a name (as built: the copy and paste clipboard carries layer and
  geometry only, and two shapes with one name would only confuse); exploding gives none.

## 5. Decisions (approved as recommended)

1. **One box for any selection of drawn shapes,** not a move handle on the polyline alone:
   one way to move, turn and size one shape or many (an exploded part is many polylines).
2. **Position and rotation are not stored** for polylines; typing them changes the points.
   A stored angle would be a second meaning to keep in step with the points.
3. **Browser rows grouped by layer** under each kind, because an exploded DXF can bring
   hundreds of polylines.
4. **A name is an optional field, no schema bump;** not unique, not on the canvas.
5. **Order of work:** step 1, step 2, step 3, each its own commit with its tests, guide and
   texts in six languages.

## 6. Tests

- Job (`edit`): moving, turning (90° exact, 30°) and scaling each kind; a polyline's bulges
  unchanged; a rectangle's angle and corner radius follow; a turn by 360° gives the shape
  back within rounding.
- Pipeline: `transform_shapes` keeps ids and refuses imported shapes; operations still cut
  the moved shapes (a corpus project's G-code equals the original's moved by the same
  vector).
- CLI: `shape move/turn/scale`, with units, and refusals.
- GUI (the app's tests): the box shows for a polyline and for two circles, not for one
  circle; a move drag moves the shapes by the drag, one undo step; Position and Turn by.
- Step 3: names round-trip through save and load; `--name ""` clears; the Browser rows
  select their shape.

## 7. As built (2026-10-25)

- Step 1: `chipload_job::edit::transformed`, `chipload_pipeline::shapes::{transform_shapes,
  drawn_box, drawn_middle}`; the GUI's box (`box_drag.rs`, `BoxOf::Shapes`). A size drag
  scales about the shapes' own opposite corner: the handle's stand-off is taken off the
  cursor first, so the shapes' corner stays exactly where it was. The box is checked to
  keep at least 1 mm along the shapes' longer side, so a straight line can be scaled.
- Step 2: `shape_form::place_rows` (Position, Turn) in a polyline's form and, for several
  drawn shapes, in the Selection section.
- Step 3: `Shape::name` (optional, `name` in the file for circles, rectangles and
  polylines; refused on a path; an empty or blank one is a problem), `shapes::name_shape`,
  `shape add/set --name`, `inspect` ends a named shape's line with `name "…"`. The Browser
  lists drawn shapes per layer (`Group::members`); the name field keeps what is typed while
  it has the keyboard and stores it trimmed.
