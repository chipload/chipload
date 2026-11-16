# Spec: the part itself placed, as its instances are

Status: **draft 2026-11-16**, for review. Follows the user's finding while trying instances
(2026-11-16): "explore the possibility of also moving/rotating the "original" part. This
would allow for nesting on a fixed size sheet". Decided by the user already: nesting is in
scope (ADR 0017, 2026-11-16), and **shapes are edited at their original location**. The
nesting itself is docs/specs/nesting.md, which builds on this.

## 1. Why

Since schema 38 a part's instances 2, 3, … can be moved, turned and mirrored
(docs/specs/instances.md), but instance 1, the part itself, is cut where its shapes are
drawn. To put it elsewhere the shapes must move, and that isn't always possible or wanted:
- several parts share one DXF, which is placed as a whole (`place`);
- the drawing should stay as designed (a CAD file updated with *Update from DXF*);
- a nester (docs/specs/nesting.md) must be able to place every instance, the first too,
  without rewriting the drawing.

## 2. What changes

- A part's shapes become its **definition**: where they are drawn is where the part is
  designed and edited, as a block in CAD.
- **Instance 1 gets a placement** like the others: `angle` (radians, counter-clockwise) and
  `at` (mm), applied as an instance's are: turned about the part's centre (the middle of
  `part_box`), then moved (instances.md §2). Default angle 0, at (0, 0): cut where drawn,
  as today.
- **Instance 1 can be off** (`enabled` on the part already exists, but switches off the
  whole part with its instances). Recommended: no; an instance 1 that is not wanted is
  simply moved where wanted. See decision 3.
- **The part's `mirror`** (docs/specs/mirror-part.md) stays as it is: it mirrors the
  definition, so instance 1 and every instance with it. Instance 1 needs no mirror of its
  own (decision 2).
- **Instances 2, 3, … stay placed relative to the definition**, not to instance 1: moving
  instance 1 moves only it. Every instance is a placement of the same definition, so a
  nester treats them alike (decision 1).

## 3. Planning

- Instance 1 with angle 0 and at (0, 0): planned exactly as today. **Every golden stays.**
- Only moved: planned on its moved contours, as a moved instance is (its G-code is that of
  the part drawn there).
- Turned: planned on the part as drawn, then the planned path turned about the part's
  centre and moved (`chipload-toolpath` `place.rs`), as for a turned instance. Everything
  instances.md §3 says holds: hand-placed and axis-laid things go with it, climb stays climb,
  each verified offset stays verified (a turn keeps every distance).
- The overlap check (`check_parts_apart`) sees instance 1 where it is cut.
- What other parts cut is brought into the part's own frame for the dogbone check, as for
  instances.
- A part's own `mirror` and instance 1's turn compose as for instances: mirrored about the
  part's centre line first, then turned, then moved.

## 4. Where things are: the drawing and the cut (the user's decision)

The shapes are edited **at their original location**. So for a part whose instance 1 is
placed elsewhere, the drawing and the cut are in two places:
- **The canvas, Edit mode:** the shapes stay where drawn, editable as any shapes. A part
  whose instance 1 is placed away gets a dashed box round its shapes with its name and
  "not cut here" (decision 4). Instance 1 shows as a numbered ghost `#1` where it is cut,
  picked, dragged and turned as instances 2… are.
- **Toolpath, simulation, playback, 3D view:** where things are cut, as always.
- **The board:** a board fitted to the drawing (`fit`) holds what is cut: the placed boxes
  of each instance, instance 1 included, and none of a placed-away part's own shapes
  (decision 5). A fixed-size board doesn't move.
- **Work zero** at the drawing origin stays the drawing origin; work zero at a board corner
  follows the board as before.
- **Snapping** to a ghost: none (as today for instance ghosts). Shapes snap to shapes.

## 5. Interactions

- **Setups:** instance 1 is cut in its part's setup, as now.
- **Two-sided (back setups):** a back's parts are placed in the front's coordinates and the
  board turned over (docs/specs/two-sided.md). A placed instance 1 is placed before the turn,
  as instances are. Tests cover a back setup with a moved and turned instance 1.
- **Inlays** (vcarve-inlay.md): the plug's part is mirrored; it may be placed too.
- **Pieces** (pieces.md): a setup in pieces refuses instances today; it refuses a placed
  instance 1 the same way (decision 6).
- **Dowel holes** belong to the board, not to a part: they are never placed.
- **A surface** faces the whole board once, wherever the part is.
- **Reliefs:** the model turns with the part, as with a turned instance.
- **DXF export** writes the drawing as drawn (the definitions). `dxf export --as-cut` (each
  instance's shapes placed where cut) is a later step, needed by nesting (nesting.md).
- **Setup sheet:** the picture shows what is cut where, as for instances; the parts list
  says "instance 1 moved, turned 90°" when placed.
- **The Compass** (compass.md) moves and turns instance 1 as it will instances (the loose
  end of instances.md §4, built with this).
- **Update from DXF / changed shapes:** the part's centre is worked out each time
  (instances.md §2); a placed instance 1 follows its definition as instances do.

## 6. In the window

- The part's instance rows in the Browser start with **Instance 1** once the part has a
  placement or instances, with its form: angle and position (its on/off box is the part's).
- A part's right-click: **Place…** opens instance 1's form; **Reset placement** puts it back
  where drawn (one undo step).
- Instance 1's ghost, once moved, is dragged and turned like the others (the turn handle,
  quarter turns snapping within 3°, Alt free).
- Dragging the part's own shapes (box drag, Compass on the shapes) still moves the
  definition, and with it every instance, the first too: the same as editing it.

## 7. Command line

- `chipload part place part.chipload --part 1 [--at X,Y] [--angle 90]` (`--reset` back to
  as drawn); `part instance list` lists instance 1 with its placement.
- `part instance set --part 1 1 --at …` works on instance 1 as on the others; `part
  instance remove 1` is refused (the part itself).
- `get`/`set` paths: `parts.0.angle`, `parts.0.at`.

## 8. The file (schema 39)

```json
"parts": [ { "id": 1, "name": "side", "setup": 1, "mirror": null,
             "angle": 1.5707963267948966, "at": [120.0, 0.0],
             "order": "by_operation", "instances": [] } ]
```

- `angle` and `at` on a part: written always (every field written explicitly).
- **Schema bump to 39** with a migration 38 → 39 that gives each part angle 0 and at
  (0, 0): an older Chipload reading a file with a placed part would cut it where drawn,
  in the wrong place, so it must refuse the file (`Part` has `deny_unknown_fields`, and the
  version says it). Frozen sample `tests/corpus/jobs/v38/` with instances; its golden
  unchanged through the migration.
- Validation: angle and at finite.

## 9. Tests

- Golden: every frozen project unchanged (instance 1 not placed).
- Property test (as `toolpath/tests/instances.rs`): instance 1 turned and moved cuts the
  part's profile point for point, turned and moved; mirrored part plus turn too.
- Pipeline: five kinds of operation, instance 1 placed, climb kept, the checker passes; a
  fitted board holds the placed instance 1 and not the definition's shapes; overlap with
  another part refused naming instance 1; back setup.
- Window: drag and turn instance 1's ghost, Reset placement as one undo step, the
  "not cut here" box.
- CLI: `part place`, `--reset`, `get/set parts.0.at`.

## 10. Decisions

1. **Instances 2… stay relative to the definition**, not to instance 1. **Recommended:**
   all instances alike, as a nester wants; moving instance 1 doesn't drag the others.
   Alternative: the others follow instance 1 (a group), which nesting would have to undo.
2. **No mirror of instance 1's own;** the part's `mirror` mirrors the definition.
   **Recommended:** two mirrors on one instance would be confusing and the file keeps one
   meaning per field. Alternative: move `mirror` into instance 1's placement (a migration
   that changes stored values).
3. **Instance 1 can't be switched off alone.** **Recommended:** the part itself is always
   cut once; `enabled` on the part switches off all. Alternative: an `enabled` on instance 1
   too, so a part could be "only instances 2 and 3".
4. **A placed-away part's shapes show a dashed "not cut here" box** in Edit mode.
   **Recommended**, so drawing and cut are never confused. Alternative: shapes faded.
5. **A fitted board holds what is cut, not a placed-away part's shapes.** **Recommended:**
   the board is for cutting. Shapes no part cuts still count, as today.
6. **Not in a setup cut in pieces**, as instances. **Recommended.**
7. **Order of work** (each step a commit): (1) the model, schema 39, migration, sample,
   validation; (2) planning, board extents, overlap, property and pipeline tests; (3) the
   command line; (4) the window: Instance 1 row and form, ghost, drag and turn, *Place…*,
   *Reset placement*, the "not cut here" box, the Compass on instances; (5) the sheet,
   guide, schemas, translations. **Recommended.**
