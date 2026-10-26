# Spec: tabs placed by hand

Status: **draft** 2026-10-26, for review.

The user's question (2026-10-26): does click and drag, as for a ring's start, also work for
tabs? It doesn't: a profile's tabs are a count, width, height and flank angle, and Chipload
spaces them evenly round each contour, sliding each off corners (docs/specs/tabs.md §2).
tabs.md left "manual tab placement by clicking in the viewer" for later. This is it.

## 1. What the user can do

- **Drag a tab** along its ring on the canvas, as a ring's start is dragged today
  (docs/specs/helix-and-leads.md part A). The other tabs of that ring stay where they are.
- **Add a tab:** right-click the ring of the selected profile, *Add a tab here*.
- **Remove a tab:** right-click it, *Remove tab*.
- **Back to even spacing:** right-click a tab, *Space this ring's tabs evenly*; or *Reset*
  under Tabs in Properties for every ring of the operation.

The width, height and flank angle stay one setting for the operation; only positions are
by hand.

## 2. Format (no version bump)

One optional field in `tabs`, left out when empty (as `rest_passes`), so a project without
hand-placed tabs still opens in older versions:

```json
"tabs": {"count": 4, "width": 8, "height": 3, "flank_angle": 1.5708,
         "at": [[12.5, 40.0], [88.0, 40.0], [50.0, 0.0]]}
```

- `at`: tab centres, points in drawing coordinates (mm), as `starts` are. Each belongs to
  the ring nearest to it.
- **A ring with points gets exactly those tabs** (decision 2); a ring without gets `count`
  tabs, evenly spaced, as today. So one operation can have some contours by hand and the
  rest even.
- Older versions refuse the unknown field, the safe outcome.

## 3. Placing them

For a ring with points, each point is moved onto the ring (its nearest point there) and is
that tab's centre on the tool path. Then, with the tool diameter D and the gap `width + D`
the tool lifts for (tabs.md §2):

- **No sliding:** a tab placed by hand stays where it was put, also on a corner or curve;
  the note "some tabs sit on a corner or curve" is given as today.
- **Errors (never moved quietly), each naming the tab's point:**
  - two tabs closer than `width + 2·D` (centre to centre along the ring): they would
    overlap, or leave less than one tool diameter of normal cutting between them;
  - a tab whose gap covers the ring's start (where every pass plunges), or comes closer
    to it than D/2: "move the tab or the ring's start";
  - a point farther than the tool diameter from every ring of the operation: the drawing
    changed (decision 4).
- The post-condition (tabs.md §4) and the checker are unchanged: the cut length below the
  tab top is `L − n · (width + D)` with n the ring's tabs.

## 4. Moving the drawing

As start points and corner choices (docs/specs/dogbone-per-corner.md §4): a tab point
moves with `place` when the shape nearest to it comes from the DXF that moves, and with
`shape move/turn/scale` and the box when it lies on a drawn shape that moves. Copies get the
tabs of copy 1 (the drawing).

## 5. GUI

- With a profile selected and generated, every tab shows a marker (the tab colour) at its
  centre on the ring, filled where placed by hand, hollow where spaced by Chipload.
- **Dragging a tab** of an evenly spaced ring first stores all of that ring's tabs as points
  where they are, then moves the dragged one, as one undo step: the others stay put
  (decision 3). The drag follows the ring as the start marker does. On release the
  operation generates again.
- Right-click menus as in §1; hovers: "Tab: drag along the ring; right-click to remove".
- Properties, under Tabs: "Tabs placed by hand on 2 contours" and *Reset*, when any are.
  Count stays editable: it is for the contours without points.

## 6. CLI (first, rule 5)

- `chipload op tabs PROJECT --op N`: every ring's tabs, with their centres, and whether
  they are by hand. `--json` too.
- `chipload op tab PROJECT --op N --at X,Y` adds a tab (the first on an evenly spaced ring
  first stores the ring's current tabs, as in the GUI); `--remove --at X,Y` removes the
  nearest; `--reset` removes every point; `--even --at X,Y` resets that ring only.

## 7. Tests

- Placement: a ring with points gets exactly them, a ring without gets `count`; no sliding;
  each error at its boundary (two tabs at exactly `width + 2·D`, and a hair closer; a tab
  just clear of the start, and touching it).
- The post-condition holds with uneven tabs (proptest: random points on a rectangle, kept
  apart as above).
- `place`, `shape move` and copies move them; a point far from every ring is refused.
- CLI and GUI: drag, add, remove, reset; a drag pins the ring's other tabs where they were;
  one undo step each.
- Golden: unchanged (no corpus project has `at`).

## 8. Decisions

1. **Points in drawing coordinates, inside `tabs`** (`tabs.at`). *Recommended*: the same as
   start points and corner choices, so moving the drawing has one rule; nothing depends on
   contour numbers. Alternative: a separate operation field.
2. **Per ring, all by hand or all even.** *Recommended*: a ring with points gets exactly
   those; mixing hand-placed and evenly spaced tabs on one ring would need a rule for
   spacing the rest round the fixed ones, and would move them when one is dragged.
3. **Dragging one tab of an evenly spaced ring pins the others where they are.**
   *Recommended*, so a drag never moves anything but the dragged tab. Alternative: the
   others re-space round it.
4. **A tab point far from every ring is an error,** not moved to the nearest ring.
   *Recommended*, as for corner choices: a tab in the wrong place lets the part come loose
   or leaves a bridge where it shouldn't be.
5. **No sliding off corners for tabs placed by hand.** *Recommended*: the user chose the
   place; the corner note still says so.
6. **No version bump:** one optional field, left out when empty. *Recommended*.
7. **Order of work:** the format, planning and CLI; then the canvas. Each its own commit
   with tests, guide and texts in six languages.
