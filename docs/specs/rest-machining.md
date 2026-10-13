# Spec: rest machining

Status: **approved 2026-10-13**, all decisions as recommended (§9).
Built 2026-10-13. As built:
- §3: the rest area is R ∩ (U grown by D plus the wall gap). The leftovers are grown
  together in one shape offset, so leftovers that come near each other become one piece.
  Each piece is pocketed and verified against its own boundary. That boundary lies inside
  R, so the check is stricter than one against R.
- §3: the size in the "too small" note is the size of the leftovers in the skipped piece.
- §8: the golden sample (`jobs/v18/rest.chipload`, `v18_rest`) uses drawn shapes rather
  than a DXF: a pocket with an 8 mm slot and a round island, 12 mm then a 4 mm rest pocket.
  The result test runs the material-removal simulation (pipeline `tests/simulate.rs`).

Guide: docs/book/src/operations/rest.md.

Approved plan (2026-10-13, ADR 0017), item 2. A large tool clears a pocket fast and leaves
material where it doesn't fit: inside corners rounded to its radius, and slots and necks
narrower than it. A **rest pocket** with a smaller tool cuts only that material, instead of
clearing the whole pocket again.

## 1. What the user sets

On a **pocket** operation (decision 1), one new choice: **Rest after tool N**, where N is
a project tool that is larger than the pocket's own tool.

- The pocket then clears only the material that tool N could not reach in the pocket's
  shapes, and nothing else.
- Typical use: Pocket 1 with T1 (12 mm), then Pocket 2 with T2 (3 mm), on the same shapes
  and depths, rest after T1.
- The pocket still has its own depths, step down, stepover, feeds, entry, direction and
  keep-tool-down, used as for any pocket.

The project file gains one optional field on `PocketOp`:
- `rest: { after_tool: 1 }`;
- missing means a normal pocket, as before.

That is an optional field with a safe default, so there is no schema bump (CLAUDE.md), and
no golden changes.

## 2. What counts as "left" (decision 2)

The rest is worked out **from the larger tool's size**, not from the toolpath an earlier
operation actually made:

- R is the pocket region with this operation's clearance (pocket.md §2, as now).
- U, the material left, is R minus the area a disk of the larger tool's radius reaches
  inside R. In other words, R minus the opening of R by that radius. This is exactly the
  "unreachable" area the material-left display already computes (material-left.md §2),
  with the same wall gap.

So the rest pocket doesn't depend on the order of operations, on the earlier pocket's
stepover, or on whether the earlier pocket exists at all. §5 says what is checked about
that.

## 3. The toolpath

1. **The area to cut** is R ∩ (U grown by D + δ), where D is this pocket's tool diameter
   and δ is the wall gap.
   - Every placement of the small tool that touches material in U lies within D of it, so
     the area holds every cut that reaches U.
   - Along R's real walls the area's edge **is** R's wall, so the small tool's wall pass
     cuts the real corner.
   - Its other edges lie in material the large tool already cleared. Rings there cut air,
     at the cost of a little time.
   - Booleans and offsets run under `guarded` and are verified, as for every pocket.
2. **Each piece** of that area (typically one per corner, plus one per narrow slot) is
   pocketed by the existing ring planner, unchanged:
   - rings inside out and the wall pass last;
   - the entry: plunge, ramp, or a helix where one fits, falling back as today;
   - depth passes from start to end depth;
   - pieces ordered nearest-neighbour;
   - the post-condition that no move comes nearer than r to R's walls. It is checked
     against R itself, not the rest area.
3. **Where the small tool doesn't fit either** (a piece whose level 0 is empty): not an
   error, as it would be for a whole pocket. The piece is skipped, and one note is given:
   "rest after T1: material in 4 places is too small for T2 too (largest about 1.2 ×
   1.2 mm)".
4. **Nothing left** (the large tool reaches all of R, e.g. a circle wider than it): no
   moves, and a note: "Operation 2: tool 1 reaches the whole pocket; nothing left to rest
   machine".

## 4. Material-left display and simulation

- The rest pocket's own unreachable material (for the small tool) shows as for any pocket.
- The earlier pocket's corners still show as unreachable for that operation. The display
  judges each operation on its own (material-left.md §2), and the plan already merges that
  display into the simulation (lean plan item 5). The **simulation** shows the true result:
  the corners cut by T2.

## 5. Checks

Problems, which block generating:
- `after_tool` is not a tool of the project: "Operation 2: rest after tool 5, which the
  project doesn't have".
- The rest tool is not larger than this pocket's tool, by more than `REST_TOL_MM`:
  "Operation 2: rest after tool 2 (3 mm) needs a tool larger than this pocket's (3 mm)".
- The rest tool is not an endmill (a pocket's tool must be one).
- With `corners` (dogbones) set (decision 3): "Operation 2: dogbones and rest machining
  can't be combined yet; dogbone the large tool's pocket".

Warnings, which allow generating (decision 4):
- **No earlier pocket for it:** no enabled pocket before this one in the job cuts with
  `after_tool`, selects the same shapes or layer, and goes at least as deep. The warning:
  "Operation 2: rests after tool 1, but no earlier pocket cuts these shapes with it; the
  rest of the pocket will be left uncut".
  - This is still safe: a rest pocket cuts its pieces from the start depth down in passes,
    as for any pocket. It just leaves the bulk.

## 6. CLI, GUI, setup sheet

- **CLI:** `chipload op rest part.chipload --op 2 --after-tool 1` sets it, and `--none`
  removes it. This is the same family as `op tool`. `plan` and `run` print the notes, and
  `plan --json` carries the operation as stored.
- **GUI:** the pocket form gets a **Rest after** choice:
  - "none", then the project's endmills larger than the pocket's tool ("T1 · 12 mm
    endmill");
  - the operation list says "rest after T1" after the pocket's name.
- **Setup sheet:** the operation's row adds "rest after T1 (12 mm)".

## 7. Edge cases

- **Islands:** U includes the corners round islands and the gaps between an island and the
  wall narrower than the large tool. The rest area keeps the islands as islands.
- **Crossing pockets in one operation** (messy_2): each pocket is its own region (pocket.md
  §2). U is computed per region with that region's reach, as the display does. A corner
  the other pocket clears gets cut twice; the second time is air.
- **Clearance:** R includes this operation's clearance, so a rest pocket with a clearance
  of 0.2 mm and a large pocket with 0 leave a 0.2 mm band. That is what was asked, and it
  is the user's choice.
- **Depths differ** from the earlier pocket's: the rest pocket uses its own. The warning in
  §5 covers a rest pocket deeper than any earlier one.
- **Copies:** as for any operation.
- **The rest tool changes diameter** later: the rest follows it, since it is stored by tool
  id.

## 8. Tests

- **Toolpath (unit):**
  - 40 × 40 mm square, T1 12 mm, T2 3 mm: four pieces, one per corner. Every move stays
    inside R shrunk by 1.5 mm. No move lies farther than 6 + 3 mm from a corner.
  - A slot 8 mm wide inside a 12 mm tool's pocket: cleared along its length.
  - An island near the wall: the gap is cleared.
  - A circle larger than the large tool: no moves, and the note.
  - Too small for T2: the pieces are skipped, with the note and its size.
- **Result, using the simulation:** a job with Pocket 12 mm and then Rest 3 mm. The
  simulated floor reaches end depth everywhere in R except within a 3 mm tool's
  unreachable corners. Without the rest pocket, the 12 mm corners stand.
- **Property test:** random polygons with optional islands and two tool sizes. Rest moves
  never come nearer than r to a wall (the pocket post-condition). Large reach plus rest
  sweep covers R minus the small tool's unreachable area, judged by the simulation's
  height map.
- **Checks:** each problem and the warning in §5, by message.
- **CLI:** `op rest` set and remove. The end-to-end run passes the checker.
- **Golden:** one new golden (a corpus DXF with a square pocket and a slot, 6 mm then 2 mm
  rest). Existing goldens don't change.
- **GUI:** the choice lists only larger endmills; choosing one sets the field; the list
  label shows it.

## 9. Decisions

1. **Pockets only, for now.** **Recommended**: corners and narrow slots in pockets are the
   common case. Inside and outside profiles leave corners too, but cutting those with a
   small tool is a dogbone's or a second, smaller-tool profile's job. Rest profiles can
   follow if you miss them.
2. **The rest is from the larger tool's size** (geometric reach), not from the earlier
   toolpath or the simulated stock. **Recommended**: simple, independent of order and
   stepover, and the same computation the display already trusts. Alternative: from the
   simulation's height map. That would catch an earlier pocket's missed spots too, but it
   couples the planner to the simulation and to the exact order of operations.
3. **No dogbones on a rest pocket yet.** **Recommended**: dogbone the large tool's pocket.
   The small tool then reaches less than the dogbones, so there is little to rest-machine.
   Combining them needs the dogbone logic to work on the rest pieces.
4. **A missing earlier pocket is a warning, not a problem.** **Recommended**: the rest
   pocket is safe on its own, and you may cut the large pocket in another program.
   Alternative: a problem that blocks generating.
5. **Stored by tool id** (`after_tool`), not a diameter. **Recommended**: it follows the
   tool's edits, and the warning in §5 can look for the earlier pocket. Alternative: a plain
   diameter, for a tool that isn't in the project.

## 10. Addendum: "Add rest pocket" (2026-10-13, built the same day)

User question: rest after an *operation* rather than a tool, inheriting its tool, shapes and
depths. Two ways were weighed:
- **A, a live link:** "after operation N". Operations have no stable ids, so this needs
  ids, a schema bump, inherited read-only fields, and handling of broken links.
- **B, an action that copies at creation.**

The user chose **B**: no format change. The existing warning (§5) catches a rest pocket that
drifts from its pocket later. A can still come later if drift turns out to be a nuisance.

**The action:** pocket N plus a smaller project endmill T gives a new pocket, inserted right
after N, enabled:
- **Copied from N:** the shapes (`select`), start and end depth, clearance, direction,
  stepover, keep-tool-down and round corners.
- **Set:**
  - `tool` is T;
  - `rest` is `{after_tool: N's tool}`;
  - the name is "N's name (rest)", or "Rest" if N has no name.
- **Feeds and step down:** the library's cutting values for T when it has them, as when an
  operation is created with a tool (tool-library §3); otherwise N's values, for the user to
  check.
- **Entry:** N's, made to fit T:
  - a helix gets T's default helix (N's radius would be too large for T);
  - a plunge with a T marked not centre-cutting becomes a helix;
  - a ramp stays a ramp.
- **Not copied:** start points and the lead (they belong to N's rings), and dogbones (§5:
  not combined with rest).

**Refused:**
- N is not a pocket;
- N is itself a rest pocket (rest after the rest tool is the user's to set);
- T is not in the project, not an endmill, or not smaller than N's tool.

**Where:**
- **GUI:** the operation card's right-click menu, *Add rest pocket ▸*, which lists the
  project's endmills smaller than the pocket's tool.
- **CLI:** `chipload op add-rest part.chipload --op 1 --tool 2`. It prints the new
  operation's number; `--json` gives `operation` (0-based).
