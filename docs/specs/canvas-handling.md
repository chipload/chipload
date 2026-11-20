# Spec: straight drags, several points at once, instances from the canvas

Status: **approved, 2026-11-20** (decisions in §5, all as recommended). From the user's
testing of the drawing tools.

Window behaviour only: no file format, G-code or message changes.

## 1. Shift keeps a drag straight

Shift already keeps drawing and moving whole shapes to 15° steps from where the line or
the move starts (drawing-input.md §2). The same now holds for:

- **a polyline's or any shape's point handle:** the point moves in 15° steps from where it
  was when the drag began; with snapping on, the distance goes in grid steps, as for a
  move;
- **an instance's move** (its badge or outline): the instance moves in 15° steps from where
  it was grabbed;
- **a rectangle's turn handle or a polygon's corner:** 15° steps measured from the
  centre, so the shape turns in 15° steps.

A segment's middle handle keeps Shift for bending it into an arc (decision 2). While a
point or an instance is dragged, the status bar says *Shift: 15° steps*.

## 2. Several points of one shape

- **Ctrl+click** (⌘ on a Mac) a point handle of the selected shape: it joins the selected
  points, drawn filled; Ctrl+click it again to leave it out. A plain click elsewhere, Escape
  or selecting another shape clears them.
- **Dragging any of them** moves all of them by the same amount, as one undo step; Shift
  keeps the move straight (§1), and the grabbed point snaps as a single one does.
- **Which shapes:** polylines and splines (their points). Other shapes' handles are sizes,
  not points, and stay single.
- **Rounded corners keep their radius.** A move that leaves a shape invalid (a radius that
  no longer fits) holds the shape at its last valid place and says why, as a single
  point's drag does.
- Ctrl-*drag* on a segment's middle still inserts a point (drawing-editing.md).

## 3. Instances from the canvas

- **Right-click an instance** (its badge or outline): *Duplicate*, *Turn 90° left*, *Turn
  90° right*, *Mirror*, *Cut* (on or off), *Remove*. The Browser's instance menu gets
  *Duplicate* and *Turn 90° right* as well.
- **Duplicate:** a new instance turned and mirrored as the original, on the same setup,
  placed beside everything the part cuts as *Add instance* does (instances.md §4), and
  selected. Each further duplicate goes beside the last.
- **Ctrl+drag an instance:** the original stays; a new one (as *Duplicate*, turned and
  mirrored alike) follows the pointer and is dropped there. One undo step; Escape drops
  nothing.
- **Ctrl+C, then Ctrl+V,** with an instance selected: as *Duplicate*. The clipboard holds
  which instance it was; pasting with shapes on the clipboard still pastes shapes.
- **Turning** goes about the instance's centre, as its turn handle does.

Command line: `chipload part instance add P --like 3` places a new instance turned,
mirrored and on the setup of instance 3 (`--angle`, `--mirror`, `--setup` and `--at` still
override each).

## 4. Tests

- Shift on a point drag and an instance move: the result lies on a 15° step.
- Ctrl+click toggles points; dragging one moves the selected ones together, one undo step;
  a radius that would no longer fit holds the shape.
- Duplicate, Ctrl+drag and copy-paste of an instance: a new number, same angle, mirror and
  setup; Escape during a Ctrl+drag adds nothing; each is one undo step.
- Turn left and right by a quarter; `--like` on the command line.

## 5. Decisions

1. **15° steps under Shift**, as drawing and moving already do (0° and 90° among them).
2. **On a segment's middle handle, Shift still bends.**
3. **Selected points within one shape** only, for now.
4. **A duplicate goes beside everything the part cuts**, as *Add instance* does, so
   repeated duplicates line up one after the other.
5. **Quarter turns about the instance's centre.**
