# Spec: instances, a part placed again

Status: **approved 2026-11-16**, all decisions as recommended (the user: "I agree with all
recommendations"). Follows the user's idea: right-click a part, *Add instance*, and a linked,
ghosted copy appears that can be moved and turned; *Add mirrored instance* does the same,
mirrored. More flexible than copies in a grid (docs/specs/copies-grid.md), which this
replaces (decision 1).

## 1. Why

Copies today are a grid: columns × rows, one pitch, every copy the same way round. Real
work wants more than that:
- two sides of a box, one the mirror of the other;
- a part turned 90° to fit the board, or nested in an L-shaped space;
- a few signs placed where there is room, not on a grid.

Today that means drawing the part again, or several parts with copied operations, and
every later change made twice.

An **instance** is the part placed again: the same shapes and operations, moved, turned and
perhaps mirrored. It is linked: change the part, and every instance follows.

## 2. What an instance is

- A part has its shapes where they are drawn: that is the part itself, instance 1.
- `instances` lists the others. Each has:
  - `id`, unique in the part, from 2 (1 is the part as drawn);
  - `mirror`: none, left to right, or top to bottom;
  - `angle`: radians, counter-clockwise;
  - `at`: its move, mm;
  - `enabled`: an instance can be off without being removed.
- **The transform**, in this order, about the **part's centre** (the middle of the box round
  the shapes its operations cut): mirror, then turn, then move by `at`. The centre is worked
  out each time, so an instance follows the part when its shapes change. With
  `mirror: none, angle: 0`, `at` is exactly what a copy's shift was.
- The part's own `mirror` (docs/specs/mirror-part.md) stays: it mirrors the part and with it
  every instance, as a whole.
- **Cut order:** the part's `order` stays: *by operation* (each operation on every instance,
  then the next) or *by instance* (every operation on one instance, then the next), as copies
  have it now.

## 3. Planning (decision 3)

Each instance is planned from the part's **contours transformed** into place, never from the
toolpath copied and turned:
- A mirrored instance cuts in the right direction. Climb stays climb: the same rule as mirrored
  parts, which reverse the contours' direction.
- Everything that is a place on the drawing goes with the instance, transformed the same way:
  tabs placed by hand, ring starts, corner choices, a pocket's start.
- Everything laid out along the board's axes **turns with the instance** (decision 3): tabs
  set along X or Y (docs/specs/tabs-along-grain.md), a fill pattern's lines and its origin
  (docs/specs/fill-pattern.md). So every instance comes out the same as the part. (A
  surface faces the whole board and is cut once, whatever instances its part has.)
- Holes, V-carves, flutes, reliefs and the rest are planned on their transformed shapes like
  any other. A relief's model turns with it.
- **Overlaps** (decision 4): instances whose cuts come into each other, or into another
  part's, are refused, as copies and parts are today (`check_parts_apart`). The message
  names the two instances.
- A free angle is planned exactly. Arcs stay arcs: turning and mirroring keep them circles.

**As built (step 2, 2026-11-16).** Planning the transformed contours would mean turning
every place and direction each operation reads (tab spots, starts, corner choices, tabs
along X or Y, a pattern's lines and origin, a relief's model) inside every planner. The
same result comes more safely the way mirrored parts are cut (docs/specs/mirror-part.md
§3):
- An instance only moved is planned on its moved contours, as copies were, so its G-code
  is what it was.
- A turned or mirrored instance is planned on the part as drawn, then the planned path is
  mirrored, turned about the part's centre and moved (`chipload-toolpath` `place.rs`).
  Mirrored, it is planned with the cutting sense turned first, so climb stays climb.
- Everything in §3 follows: hand-placed and axis-laid things go with the instance. A turn
  and a mirror keep every distance, so each offset that was verified stays verified.
- What other parts cut is brought back into the part's own frame for the dogbone check,
  as for mirrored parts.
- Turning mixes X and Y, so a placed move always gives both. A quarter turn is exact
  (cos and sin 0 or ±1).
- Tests: a property test (`chipload-toolpath/tests/instances.rs`) checks that an instance
  turned and moved cuts the part's profile point for point. A mirrored instance turned
  matches the same mirrored instance unturned, and either way it cuts the part's area,
  turning the same way. Through the pipeline (`chipload-pipeline/tests/instances.rs`),
  five kinds of operation, a mirrored part too, keep climb and pass the independent
  checker. A pocket starts near where the tool is, as copies always have, so pockets are
  compared by area.
- The setup sheet and the window draw turned and mirrored instances where they are cut,
  faded, as mirrored parts are (`parts::placed_outlines`).

## 4. In the window

- A part's right-click menu: **Add instance** and **Add mirrored instance** (left to right).
  The new instance appears beside the part, past its box and 10 mm apart, selected.
- **The view** draws each instance of the active part ghosted: its shapes faint, with its
  number (`#2`). Other parts' instances as they are drawn today for copies.
- **Moving and turning:** an instance is selected by a click on its ghost. It then moves by
  dragging, and turns by a handle (or the Compass: docs/specs/compass.md, its move and turn).
  Turning snaps to quarter turns within 3° (decision 2); typing an angle gives any angle.
- **The Browser:** a part's instances as rows under it, each with its on/off box. A row's
  right-click: *Mirror*, *Turn 90°*, *Remove*.
- **The form** of an instance: its mirror, angle and position, each a field.
- **Generate and the simulation** show every instance cut, as copies are now.

## 5. Command line

- `chipload part instance add part.chipload --part 1 [--at 120,0] [--angle 90] [--mirror
  left-right]` prints the new instance's id.
- `chipload part instance set|remove|list part.chipload --part 1 [ID …]`.
- `chipload part instance array part.chipload --part 1 --grid 3x2 --pitch 120,80` (or
  `--gap`): adds the instances a grid of copies would have been (decision 1). It replaces
  `chipload copies`, which stays as a name that does the same, with a note, for scripts.

## 6. The file (schema 38, decision 1)

```json
"parts": [ { "id": 1, "name": "side", "setup": 1, "mirror": null,
             "order": "by_operation",
             "instances": [
               { "id": 2, "enabled": true, "mirror": "left_right",
                 "angle": 0.0, "at": [320.0, 0.0] } ] } ]
```

- `copies` goes; `order` moves from `copies` into the part (default *by operation*).
- **Migration 37 → 38:** a part with copies gets one instance per copy but the first, in the
  grid's order (row by row from the bottom left), `at` the copy's shift, angle 0, no mirror,
  ids 2, 3, … The G-code stays byte-identical: every golden with copies is unchanged. There
  are frozen samples with copies, and their goldens go through it.
- Validation: ids unique and 2 or more, angle and `at` finite.

## 7. Setup sheet and report

- The sheet lists each part's instances: "side: 3 instances (2 mirrored, 1 turned 90°)", and
  the picture shows them all, as copies today.
- The time estimate is per operation, all instances counted, as copies are counted.

## 8. Not in this step

- Instances in a setup cut in pieces (decision 5): refused, as copies are now.
- Instances of one part in another setup.
- Nesting: placing instances automatically to fill a board.
- An instance with its own operations or depths: it is a copy, not a variant.

## 9. Decisions

1. **Instances are the one way to repeat a part.** A grid becomes a command that adds
   instances, and a project's copies become instances when it opens (schema 38).
   **Recommended**, chosen: one model ("no worse options"); the price is that a converted
   grid isn't respaced in one go. Alternative: keep both, the grid as one more kind of
   instance.
2. **Free angles, quarter turns snapping** while dragging. **Recommended**, chosen.
3. **What is laid out along the board's axes turns with the instance**, so every instance
   comes out the same as the part. **Recommended**, chosen.
4. **Overlapping instances are refused**, as copies are now. **Recommended**, chosen.
5. **Not in a setup cut in pieces yet.** **Recommended**, chosen.
6. **Order of work** (each step a commit):
   1. the model, schema 38 with its migration (copies into instances), validation, the
      goldens unchanged;
   2. planning: contours transformed per instance, mirrored ones reversed, hand-placed
      things and axis-laid things transformed; the overlap check; property tests (an
      instance cuts exactly the part's toolpath, transformed);
   3. the command line: `part instance add|set|remove|list|array`, `copies` kept as a
      name;
   4. the window: the right-click entries, ghosts, selecting, dragging, the turn handle,
      Browser rows, the form;
   5. the sheet and report, the guide (the Copies page becomes Instances), schemas,
      translations.
   **Recommended.**
