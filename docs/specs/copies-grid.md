# Spec: copies in a grid (basic nesting)

Status: **approved and implemented 2026-10-10.** Decisions 2-4 as recommended; decision
1 changed by the user: the cut order is a setting ("there should be an user option
between operation by operation or part by part").

Guide: docs/book/src/drawings/copies.md.

User request (2026-10-10): "Nesting: basic 2D grid of copies, no rotation; copies shown in a
ghost colour."

## 1. What it does

A project can cut its part several times, in a grid of columns × rows, each copy moved by a
fixed pitch. Every enabled operation is cut on every copy. No rotation and no free
placement: that would be real nesting, later if ever.

```
 row 2   [copy 4] [copy 5] [copy 6]      copy 1 = the drawing as drawn
 row 1   [copy 1] [copy 2] [copy 3]      copy k = shifted by (col·dx, row·dy)
          <-dx->
```

The drawing itself stays one part. The copies exist only in the toolpath and on screen.

## 2. Format (no version bump)

An optional top-level `copies`; missing means a single part, the old behaviour:

```json
"copies": {"columns": 3, "rows": 2, "dx": 120.0, "dy": 80.0, "order": "by_operation"}
```

- `columns`, `rows`: whole numbers from 1 to 100, and at most 1000 copies in all.
- `dx`, `dy`: the pitch in mm, copy to copy (decision 2). It must be more than 0 where there
  is more than one column or row. Negative values are allowed, to grow left or down.
- `order`: `by_operation` or `by_part` (§3). Missing means `by_operation`; saving always
  writes it.
- Older Chipload versions refuse a file with `copies` (unknown field), which is the safe
  outcome: they never cut one part where the file asks for six.

## 3. Cut order

A setting (decision 1):
- **Operation by operation** (`by_operation`, the default): operation 1 on every copy, then
  operation 2 on every copy, and so on. The order the user set (holes and pockets before
  outlines) then holds across the whole sheet, so no part is cut free while others still
  need work.
- **Part by part** (`by_part`): every operation on copy 1, then every operation on copy 2,
  and so on. Each part is finished before the next starts, so a job stopped half-way
  leaves whole parts, at the risk of finished parts moving while others are cut.
- Within one operation, or from part to part, the copies go row by row from the first
  copy, alternating direction (copy 1, 2, 3, then 6, 5, 4), to keep the travel short.
- Between copies the tool retracts to safe Z, as between contours today.

## 4. Safety

- Each copy is planned from the shifted contours with the same code as the original. So
  offsets, tabs, pockets, ramps and wall checks are built and verified per copy, and the
  whole program goes through the independent checker (machine travel included).
- **Overlapping copies are refused** (decision 3). Two copies overlap when the areas their
  tool cuts (the toolpath's extent plus the tool radius) overlap. For a grid this reduces to
  two rules: with more than one column `|dx|` must be at least the width of that area, and
  with more than one row `|dy|` at least its height. Error: "Copies overlap: with columns
  the pitch dx must be at least 63.000 mm (the cut is 63.000 mm wide), got 50.000."
- Copies that reach beyond the stock give a note, as a drawing beyond the stock does. The
  machine travel check stays as it is (an error from the checker).
- The work zero doesn't move: it's a point of the drawing, the stock or the machine, as
  before.

## 5. Command line

```
chipload copies part.chipload --grid 3x2 --pitch 120,80   # set
chipload copies part.chipload --order by-part              # or by-operation
chipload copies part.chipload --gap 10                     # pitch = cut size + 10 mm
chipload copies part.chipload --off                        # back to one part
```

- `--gap G` plans the project once and sets the pitch to the cut's width + G and height +
  G (decision 2), keeping the direction the grid grows in. The file always stores the
  pitch.
- Overlapping copies only give a warning here (the project may still be unfinished); `run`
  refuses them.
- `inspect` and `plan` show the grid.

## 6. GUI

- Job settings gain *Copies* rows: columns, rows, dx, dy, the cut order, and a gap with
  *Set pitch*, which sets the pitch from it as `--gap` does.
- The copies are drawn in a **ghost colour**: the drawing outlines and the toolpath are
  faded. Only the original can be selected, drawn on or dragged.
- *Fit view* includes the copies. The status line shows "6 copies".

## 7. Tests

- Format: round trip, missing = one copy, the limits (0, 101, 1001 copies, zero pitch with
  more than one column).
- Plan:
  - 1 × 1 gives byte-identical G-code to no `copies`;
  - a 2 × 2 grid is operation by operation (or part by part) and serpentine, and each
    copy cuts the original's extent shifted by its pitch;
  - pitch just under the cut size is refused, exactly the cut size is accepted (boundary);
  - negative pitch.
- Material left is reported for the part (copy 1) only: every copy leaves the same.
- Golden: unchanged, plus a new `v7_copies` (a part with a pocket and a tabbed profile in a
  3 × 2 grid) that passes the checker.
- CLI: `--grid`, `--pitch`, `--gap`, `--off`.
- GUI: the copies are shown and can't be selected; the settings row writes the field.

## 8. Decisions

1. **Cut order.** The user's choice: a setting, operation by operation (the default) or
   part by part (§3).
2. **Spacing.** Recommended: **store the pitch** (copy to copy, mm), and offer a gap as a
   one-off helper that sets the pitch. A stored gap would make the pitch change on its own
   when the tool or the drawing changes, moving every copy.
3. **Overlap.** Recommended: **refuse overlapping copies**. Sharing a cut between
   neighbours (common-line cutting) is a deliberate technique, but it needs its own design.
   Silently allowing overlap would cut into neighbouring parts.
4. **Name.** Recommended: **"Copies"** in the file, CLI and GUI, as it says what it does.
   "Nesting" usually means automatic placement with rotation, which this isn't.
