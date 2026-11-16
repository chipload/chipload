# Spec: surfaces on the board and on a part

Status: **draft 2026-11-16**, for review. From the user's finding while trying instances
(2026-11-16): "if i create two parts from the same geometry and move one part away to a
fresh part of the board, the surface operation in the first part still affects the second
part, this is not so good". The user's rule: **"if it is on the part level i must select
an outline, if i create the surface on the setup level it is valid for the whole board"**.

## 1. Why

A surface (docs/specs/surfacing.md) always faces the whole board, cut once, but it sits in
a part. So a part's surface faces where every other part is cut too, and since a fitted
board grows round instances (2026-11-16), it sweeps everything they spread over. Facing the
spoilboard or a whole board is a board's job; facing where one part will be cut is the
part's.

## 2. Two kinds

- **A board surface** belongs to a setup: the whole board, cut once, as every surface is
  today. It is listed under its setup in the Browser, not under a part, and is cut
  **first** in its setup, before any part (decision 3).
- **A part surface** belongs to a part and faces **selected closed shapes**: each shape's
  area grown by a margin (§3), cut for every instance of the part, placed, turned and
  mirrored with it like any operation of the part. It can't be made without a selection
  (the user's rule): no shapes, no part surface.

## 3. A part surface's area and moves

- **Area:** each selected closed shape, grown by **margin** (default: the tool's radius,
  so the face reaches just past where a profile on the outline cuts). Overlapping grown
  areas are joined. Holes inside a shape are faced over (a surface is a flat top, not a
  pocket's floor round islands).
- **Moves (decision 2):** planned by the pocket's planner on the grown area: rings at the
  surface's stepover, entering each depth by the pocket's helix (or its ramp where the
  helix has no room), never plunging, each ring offset verified as pockets' are. A surface
  is the pocket's clearing without walls to finish, so no new clearing code reaches the
  machine. (Back and forth inside an area, with a ramp entry, can come later.)
- **Depths:** start depth, end depth and step down as now.
- **Checked:** the area must fit the tool (a shape too small for it is refused, naming
  it, as a pocket refuses); instances of a part with a part surface are kept apart as for
  any operation (the margin counts).

## 4. The file (schema 40)

- `SurfaceOp` gets `select` (the shapes, as other operations' `select`) and `margin` (mm).
- **The level is the selection:** an empty selection is a board surface, a selection with
  shapes a part surface (decision 1). A board surface keeps its `part`, which only says
  where it stands in the operation list; the window and the command line keep it in the
  first part of its setup, and moving a part to another setup leaves the board surfaces
  where they are (they go to the setup's new first part).
- **Migration 39 → 40:** every surface gets an empty `select` and `margin` = its tool's
  radius: all of them faced the whole board, so all become board surfaces, cut exactly as
  before. The G-code is unchanged. A surface that isn't first in its setup keeps its place
  (decision 3): the migration doesn't move it, so no program changes; the existing
  warning ("Operation 3 faces the stock after …") stays.
- Frozen sample `tests/corpus/jobs/v39/` with a surface and an instance; its golden
  unchanged through the migration.

## 5. In the window

- **The setup's right-click:** *Face the board…* adds a board surface, first in the
  setup. The Browser shows board surfaces right under the setup's Stock row, before its
  parts.
- **New operation > Surface** on a part (ribbon, Q menu, the part's menu) needs closed
  shapes selected, as Pocket does; disabled otherwise, its hover saying why and that the
  setup's *Face the board…* faces the whole board.
- **The form:** a board surface says "The whole board"; a part surface shows its shapes
  and the margin.
- **Warning** (decision 4): two board surfaces in one setup that overlap in depth face
  the same top twice: "Operations 1 and 5 both face the whole board".

## 6. Command line

- `chipload op add-surface part.chipload [--setup ID] [--depth 0.5]`: a board surface, as
  now, first in the setup.
- `chipload op add part.chipload surface --shapes 1,4 [--margin 3]` (or `--layer`,
  `--group`): a part surface in the active part (or `--part`).
- `get`/`set` paths `operations.N.select`, `operations.N.margin`.

## 7. Setup sheet and report

- The kind column: "Surface, the whole board, …" or "Surface, shapes 1 and 4 + 3 mm,
  rings".

## 8. Tests

- Golden: every frozen project with a surface unchanged through the migration.
- A part surface on a rectangle: the tool stays within the grown area plus its radius, the
  whole area is faced (a property test on random rectangles and stepovers, as surfacing
  has), no plunge (every descent a helix or ramp), the checker passes.
- With an instance moved and turned: the part surface is cut under each instance, nothing
  between them; a second part's area untouched (the user's case).
- A board surface: cut once, first, whatever the parts' instances.
- The window: Surface disabled without a selection; *Face the board…*; the Browser rows.

## 9. Decisions

1. **The level is the selection** (empty = board, shapes = part) rather than a separate
   setup-level list of operations. **Recommended:** the operation list stays one list in
   cut order with every operation in a part, so ordering, parts, instances, the preview of
   the active part and every tool that walks operations keep working; the window shows the
   board ones under the setup. Alternative: operations owned by a setup, a bigger change
   to the file and to everything that groups by part.
2. **A part surface is cut as a pocket's clearing** (rings, helix entry). **Recommended:**
   reuses verified, tested clearing and entry instead of new toolpath code in material.
   Alternative: back and forth with a ramp entry, new code and its own tests.
3. **New board surfaces go first in their setup; old ones keep their place.**
   **Recommended:** facing comes before cutting, and the migration never moves an
   operation (the G-code stays). Alternative: move them first in the migration (programs
   would change where one wasn't first).
4. **Warn about two board surfaces in one setup.** **Recommended.**
5. **Margin default: the tool's radius.** **Recommended.** Alternative: 0, the face
   ending at the outline (a profile's cut then leaves a step at its outer edge).
6. **Order of work** (each step a commit): (1) model, schema 40, migration, sample,
   validation; (2) planning a part surface on the pocket planner, per instance, tests;
   (3) the command line; (4) the window: *Face the board…*, Browser rows, Surface needing
   a selection, the form, the warning; (5) sheet, guide, schemas, translations.
   **Recommended.**
