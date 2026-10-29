# Spec: a pocket stays down between its depth passes

Status: **approved 2026-10-29** ("sounds good": both decisions as recommended).

Follows docs/specs/pocket.md (§3 depth passes, §9 links as built 2026-10-15) and
helix-and-leads.md (the helix entry).

## 1. Why

The user (2026-10-29, after the first test cut): "keep tool down for pockets have
disappeared it seems … For boring a deep hole it is convenient to keep the tool down."

`keep_tool_down` went on 2026-10-15 (schema 26): between rings at one depth the tool now
always stays down where the link is safe. It never covered the move **between depth
passes**: each pass ends with a retract to safe Z, a rapid back to the first ring (or the
helix) and the descent from safe Z again, down to the floor just cut at the plunge feed.
In a deep pocket that is most of the time spent off the cut.

## 2. Behaviour

After a pass that isn't the last, instead of the retract:

1. **Lift** straight up by 0.5 mm off the floor just cut (never above safe Z), at the
   plunge feed (decision 2).
2. **Move back** in a straight line, at the cutting feed, to where the next pass starts:
   the first ring's start, or the helix's start with a helix entry.
3. **Go down** as every pass does: the helix or the ramp from the floor just cut, or the
   plunge, starting 0.5 mm above it instead of at safe Z.

It is done only where it is **safe**, otherwise the retract stays as today:

- the piece left **no uncut material** (no "material left" note for it), so everything
  inside the tool's reach of the region is cleared down to that floor;
- the move back starts inside the region and keeps at least the tool's offset from every
  wall and island, checked as the links between rings are (no length limit: it runs
  through material already cut, not an uncut band).

Always, no setting (decision 1: one best way, as for the links). The last pass ends with the
retract as before. The checker checks the program as every program.

## 3. What changes

Pockets with more than one depth pass, so golden G-code with such pockets changes: fewer
retracts, shorter cycle times. Rest pockets work the same way. Nothing in the file format.

## 4. Tests

- A round pocket in three passes: two moves back at the floor + 0.5 mm, no rapid to safe Z
  between passes, one at the end; the next pass starts at the same point as before.
- Helix entry: the move back ends at the helix's start.
- An island between the last ring's end and the first ring's start: retract as before.
- A pocket that leaves uncut material: retract as before.
- One pass: unchanged.
- Golden G-code: the diff reviewed; every program passes the checker.

## 5. Decisions (2026-10-29, as recommended)

1. **Always, no setting**, where safe; or bring back an option.
2. **Lifted 0.5 mm off the floor** at the cutting feed, so it doesn't rub the bottom; or on
   the floor.
