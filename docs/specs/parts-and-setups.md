# Spec: parts as definitions, setups hold their instances

Status: **started 2026-11-17** on the user's word ("start the work in a feature branch so
we can work in parallel"), on branch `parts-and-setups`, the decisions as recommended until
the user says otherwise. **Steps 1 to 5 built** (§11). The user's idea (2026-11-17, after nesting): "each
part should have a stock and not be a node under setup … a project has many parts and then
setups only hold instances". It brings TODO's *Setups on sheets of their own* (chosen
2026-10-30 for later) into this scope, and a second board for what nesting can't fit
(docs/specs/nesting.md §11.1). The three questions asked before writing it were left to
this review; their recommended answers are decisions 2, 3 and 9.

## 1. Why

Today a part belongs to one setup (`part.setup`), and its placements belong to the part:
instance 1 is the part's own `angle` and `at`, instances 2… are `part.instances`. That
leaves four things awkward:

- **A part can't be cut on two boards.** "Instances 1–3 of *Side* on board 1, instance 4 on
  board 2" can't be written, so nesting switches off what doesn't fit (nesting.md decision
  10) instead of putting it on a second board, as a slicer adds plates.
- **Instance 1 is a special case.** It is the part's own fields, can't be off alone
  (part-placement.md decision 3), can't be removed, and the window needs *Show placement
  instance* to show it. Every placement should be the same kind of thing.
- **Boards lie on top of each other** in the view: every setup's board is in the same
  drawing coordinates (setups.md decision 2), so two boards of the same size around the
  same shapes cover each other.
- **What a part needs isn't said.** A 6 mm bottom and an 18 mm side can be put in the same
  setup; nothing knows the bottom wants 6 mm plywood.

## 2. The model

- **A part is a definition:** a name, its operations (which select shapes in the drawing),
  its mirror (mirror-part.md), its cut order (by operation or by instance), its turns for
  nesting, and **the stock it is cut from** (§4). It no longer belongs to a setup and has
  no placement of its own.
- **A setup is a board:** its stock, its work zero, its back or pieces as today, and **its
  instances**: each one a part placed on this board, moved, turned, perhaps mirrored, on or
  off. Every instance is the same kind of thing; none is special.
- **The drawing stays one design space** shared by every setup (decision 3): shapes are
  drawn and edited where they are, as today.
- **An instance** is: the part, its number, on/off, mirror, angle, at. The transform is
  today's (instances.md §2): mirrored and turned about the part's centre, then moved by `at`
  in drawing coordinates. A setup's board is in drawing coordinates as today, so a program
  is planned exactly as now.
- **Numbers are per part, across the project:** *Side* #1, #2 on board 1 and *Side* #3 on
  board 2. A number is never reused while the instance exists, and is kept when an instance
  moves to another board.
- **A part with no instance** anywhere is a definition not cut (yet), shown in the Browser
  as such. A new part gets one instance, #1, on the active setup, where it is drawn: the
  same as today.
- **The cut order in a setup:** parts in the project's order; each part's instances on this
  board in their listed order, by operation or by instance as the part says. For a project
  migrated from schema 40 this is exactly today's order (§5).

## 2a. The simple case first (decision 10)

The user, reviewing this spec (2026-11-17): "the workflow for the user should work for a
simple part without Setups at all. So we don't add extra complexity for simple use cases
and workflows." The model always has a board and instances in the file (one way to say
it), but **the window and the command line show neither until they matter**:

- **A simple project** is one board, each part on it once where it is drawn. That is what
  *New*, *Import* and `chipload import` make, silently: the user opens a DXF, adds
  operations, generates, exports, and never meets the words *setup*, *board list* or
  *instance*.
- **One board: no Setups branch.** The Browser shows the project, the drawing, the tools,
  **Stock** and **Work zero** as rows of the project, and the parts with their operations,
  as projects did before setups. *Add board* (the project's menu, *Wizards > Two-sided…*,
  nesting overflow) makes the second board; from then on the Setups branch shows. Removing
  the second board again brings the simple view back.
- **A part once, where drawn: no instance rows,** no ghost, no badge: the part is just the
  part, its shapes are what is cut. Instances show when a part has a second one, or is moved
  or turned (*Place…* in its menu, as today), or the project has a second board.
- **A part's stock** stays out of the way: folded in the part's form, empty, never warned
  about until set (§4).
- **Nothing to choose:** every command that takes a board (`--setup`) defaults to the only
  one, every new part goes on it once, where drawn. File names, the setup sheet and the
  G-code are as today for one board.
- **Measured:** the usual simple job (import a DXF, a profile with tabs, a pocket, generate,
  export) takes no more clicks and shows no more Browser rows than in 0.37, and fewer (no
  *Setups › Setup 1* level). A window test counts the rows.

## 3. What moves where

| Today (schema 40) | Then (schema 41) |
|---|---|
| `part.setup` | gone: a part is on the boards that hold its instances |
| `part.angle`, `part.at` (instance 1) | `setups[k].instances[]` entry `{part, id: 1, …}` |
| `part.instances[]` | `setups[k].instances[]`, each with `part` |
| `part.enabled` | stays: off switches off every instance of the part, on every board |
| `part.mirror`, `part.order`, `part.turns` | stay on the part |
| — | `part.stock`: the stock it is cut from (§4) |
| board surfaces and dowel holes, operations of the setup's first part | unchanged in this spec (decision 8) |

## 4. A part's stock (decision 2)

- **Optional, on the part:** `stock: { material, thickness }`, either may be missing.
  Missing: the part goes on any board.
- **A part's stock picks its board (decision 11).** Setting a part's stock to one its board
  doesn't have moves the part, every instance of it, to a board that has it: an existing
  board with that material and thickness, else a new one made like the board it leaves
  (its material, margin and fit, its work zero rule, Z zero and overcut, the new thickness,
  fitted round the part when the old one was fitted). One undo step; the status bar says
  "*Bottom* is cut from 6 mm: board 2". The instances keep their numbers and placements.
- **Back again:** clearing the part's stock, or setting it to its first board's, moves it
  back the same way. A board that was **made this way** (marked `made_for_stock` in the
  file, optional, no bump) and is left empty is removed; one the user made, renamed or
  changed by hand is kept. With one board left the simple view returns (§2a).
- **The user's example** (2026-11-17): a DXF with *Side* and *Bottom*, the bottom to be cut
  from 6 mm, the rest from 18 mm. Import (one board, 18 mm, no Setups branch); set
  *Bottom*'s thickness to 6 mm in its form; Chipload makes board 2 of 6 mm and moves
  *Bottom* there; the Setups branch shows the two boards; export writes `-S1` and `-S2`
  programs with a setup sheet each; *through* depths follow each board. In 0.37 the same
  takes adding a setup, setting its thickness, moving the part and checking the zero.
- **Placed by hand, checked:** an instance put by hand (dragged, *Move to board…*,
  `instance move`) on a board of another material or thickness stays where it is put, with
  a warning on the instance ("*Bottom* wants 6 mm, the board is 18 mm"). Only changing the
  stock moves a part by itself.
- **No stock set:** the part stays where it is and is cut from whatever its board is.
  Depths given as *through* always follow the board's thickness.
- **Not for every board:** a part on a two-sided board or a board in pieces isn't moved by
  its stock (those boards are made on purpose); a mismatch there is the warning. A
  board's own thickness changed by hand doesn't move parts either: parts whose stock no
  longer matches are warned about.
- **Nesting** places a part only on boards whose stock matches it, and a new board for
  overflow (§9) takes the stock its parts ask for.
- **Set** in the part's form, from the library's materials, and with `part set --material
  … --thickness …` ("made board 2 (6 mm) for *Bottom*" when it moves). A part made by a wizard (two-sided, inlay, sign) or from a setup gets
  the stock of the board it was made on.

## 5. The file (schema 41) and the migration

```json
"parts": [
  { "id": 1, "name": "Side", "enabled": true, "mirror": null, "order": "by_operation",
    "turns": "none", "stock": { "material": { … }, "thickness": 18.0 } } ],
"setups": [
  { "id": 1, "name": "Board 1", "stock": { … }, "work_zero": { … },
    "instances": [
      { "part": 1, "id": 1, "enabled": true, "mirror": null, "angle": 0.0, "at": [0.0, 0.0] },
      { "part": 1, "id": 2, "enabled": true, "mirror": "left_right", "angle": 3.14159…, "at": [400.0, 0.0] } ] } ]
```

- **Migration 40 → 41**, pure and step by step as always: for each part in project order,
  its setup gets `{part, id: 1, enabled: true, mirror: null, angle, at}` from the part's own
  placement, then each of `part.instances` with `part` added, in their order. `part.setup`,
  `angle`, `at` and `instances` go. `part.stock` is left missing (no check, as today).
- **Nothing cut changes:** every frozen project v4…v40 must give byte-identical G-code after
  the migration (the golden test), and a frozen v40 sample with instances on a placed part,
  a mirrored instance, a back setup and a nested setup is added with its golden.
- **Paths change:** `parts.0.instances.1.at` becomes `setups.0.instances.2.at` for `get`,
  `set`, problems and the JSON Schema; the reference page and scripting guide say so.
- **New, optional:** `part.stock` (material, thickness; missing: none) and a setup's
  `made_for_stock` (true for a board made because a part's stock asked for it, §4; missing:
  false). Neither needs its own bump: they come with 41.
- **Validation:** an instance names a part that exists; `(part, id)` unique in the
  project; a setup in pieces holds each part at most once (as today); a back setup holds
  only its own parts' instances (two-sided.md, as today).

## 6. Planning and checking

- A setup's program is planned from its instances, grouped by part in project order: the
  same loops as today with the instance list read from the setup instead of the part.
- Everything keyed by "the part's setup" reads "the setups holding the part": the fitted
  board, the overlap check (per setup, as today), dogbone checks against other parts, the
  sheet, the simulation.
- **Instance 1 is no longer special:** it can be off, removed or moved to another board like
  any other. The rules part-placement.md made for it (no own mirror, can't be off alone)
  go; `part.mirror` still mirrors the definition and every instance with it.
- **Two-sided:** a back setup's instances are placed in its front's coordinates as today;
  the wizard places the back's parts on the back setup.
- **Pieces:** a setup in pieces still refuses more than one instance of a part.

## 7. Sheets side by side (decision 4)

- **The active setup is shown in place**, on the drawing, as today: its board, its
  instances as ghosts, the shapes where drawn. Editing works exactly as now.
- **Every other setup is a sheet beside it:** its board, its instances' outlines and its
  toolpath, ghosted, in a row to the right of the active board (by setup number, 50 mm
  apart, backs beside their fronts), labelled with its name. A click on a sheet makes it
  the active one, and it moves into place; the view keeps the board under the pointer.
  Sheets are not edited where they lie.
- **Only the view moves them:** a sheet's offset is worked out when drawn, never saved, and
  never reaches the G-code, the sheet or the simulation.
- **Rulers and the cursor readout** are the active setup's (drawing) coordinates.

## 8. In the window

- **The Browser**, once there are two boards (§2a; with one, Stock and Work zero are the
  project's rows and there is no Setups branch), has two branches:
  - **Parts:** each part with its operations, its stock, how many instances it has and on
    how many boards; a part with none says "not placed".
  - **Setups:** each board with Stock, Work zero, its board surfaces (as today), and its instances
    listed by part ("*Side* #1", "*Side* #3"), each with its tick and menu (*Mirror*, *Turn
    90°*, *Move to board…*, *Remove*).
- **Placing a part on a board:** drag a part's row onto a setup, or a part's menu *Place on
  board…*: a new instance where the part is drawn, selected for the Compass.
- **Moving an instance to another board:** *Move to board…* in its menu, or dragging its row
  onto another setup: it keeps its number and placement.
- **The part's form** gains *Stock* (material, thickness), its instances listed by board.
- *Show placement instance* goes: instance #1 is an instance like the others, always listed
  and drawn.

## 9. Nesting onto more boards (decision 6)

- `nest` with **overflow**: what doesn't fit on the board goes to the next setup with the
  same stock, or to a new one made like it ("Board 2", the same stock and work zero), and
  so on until everything is placed or a part is larger than any board (reported, switched
  off as now). Instances keep their numbers when they move board.
- In the Nest window: *Overflow: switch off / onto more boards*; the preview shows each
  board as a sheet (§7). On the command line: `nest --overflow new-boards` (default
  `off`, today's behaviour).
- Only boards that match a part's stock (§4) take it; overflow boards are never two-sided
  or in pieces.

## 10. Command line

- `part instance add|set|remove|list|array` keep working; each takes `--setup ID` (default:
  the active setup), and `list` says each instance's board.
- `part instance move PROJECT --part ID N --setup ID`: an instance to another board.
- `part place` becomes `part instance set … 1`, kept as an alias with a note.
- `setup list` shows each board's instances by part; `part list` shows each part's boards.
- `part set --material NAME --thickness 6mm` (or `--stock none`).

## 11. Order of work (decision 9)

Each step a commit with its tests, shippable on its own:

1. **The model:** schema 41, the migration, the frozen v40 sample and its golden, planning
   and checks reading instances from setups, every golden unchanged; the CLI with
   `--setup` (defaulting to the only board) and `instance move`; the window showing it as
   today. **Built 2026-11-17.** As today in this step, instance 1 is still refused a
   mirror, being off alone and removal (part-placement.md); §6's "instance 1 is no longer
   special" comes with step 4, when the window lists every instance alike. A new
   instance without `--setup` goes on the board the part is cut from (else the active
   one), so a part on one board never needs it.
1b. **The simple case (§2a):** one board shows no Setups branch, a part once where drawn
   no instance rows; the row count and click walk measured against 0.37. **Built
   2026-11-17.** The window test counts the Browser's rows as drawn: one board is two rows
   fewer than with the branch (0.37's *Setups › Setup 1*), and a second board brings the
   branch back. The walk from import to export is unchanged: nothing on it moved or was
   added. The setup row's menu (new part, face the board, nest, add a board, back side) is
   the project row's with one board, with *Pieces and back side…* for the board's own form;
   the Nest window's title names no setup.
2. **A part's stock:** the field, moving a part to a matching board (made when there is
   none, removed when emptied), the warning for parts placed by hand, the form and `part
   set`; the user's two-thickness example as a pipeline and a window test. **Built
   2026-11-17**, with three differences from §4, for the user to confirm:
   - A part made by a wizard or with a new setup does **not** get its board's stock yet:
     the usual next step, setting that board's thickness, would at once warn that the part
     wants the old one.
   - The warning for a part placed by hand on a board of another stock is a note in the
     run's report ("*Bottom* is to be cut from 6 mm, but this board is 18 mm"), not yet a
     mark on the instance's row.
   - Nesting choosing boards by stock comes with overflow (step 5): nesting one board
     places what is on it, as before.
   A board made for a stock is the user's once changed by hand: renamed, its board, zero,
   back or pieces changed, in the window, `setup set`, `zero` or `set`.
3. **Sheets side by side** in the view (§7). **Built 2026-11-17**: each other setup's board
   and its instances' outlines (on, placed), named, in a row right of the active board;
   hovered it lights up, a click activates it keeping the point under the pointer; a fit
   (F) shows them; top view only. Not yet: a sheet's toolpath (the window plans the active
   setup only); the shapes only another setup cuts still show faint where drawn, as before.
4. **The window's placing:** drag a part onto a board, *Move to board…*, the part form's
   instances by board. **Built 2026-11-17**, with §6's rule: instance 1 is like any other
   (mirrored, off alone, moved, removed; in the pipeline, the command line, nesting and the
   window). *Show placement instance* is gone: *Place…* selects the part's first instance,
   and a selected instance shows its ghost even where its part is drawn; one where drawn
   and not selected is its shapes, no ghost. In the window the boards are called setups,
   as everywhere else in it (*Place on setup ▸*, *Move to setup ▸*).
5. **Nesting overflow** onto more boards (§9). **Built 2026-11-17**: `nest --overflow
   new-boards`, the window's *What doesn't fit*; what doesn't fit goes by the stock it asks
   for to the next setup with that stock and a size of its own (not two-sided, not in
   pieces), which is nested again with it, else to a new setup made like the first; at most
   50 boards a nest. A part larger than an empty board goes back, switched off. With
   overflow, a part on the board whose stock isn't its own goes to a board of its stock
   (step 2's third difference, done). Nest again doesn't keep the overflow choice.
6. **Guide, sheet, translations, specs** brought up to date (setups.md, instances.md,
   part-placement.md marked superseded where they differ).

## 12. Tests

- **Golden:** every frozen project gives the same G-code through the new migration; a new
  frozen v40 sample (placed part, mirrored instance, back setup, nested setup) and its
  golden.
- **Pipeline:** one part with instances on two boards gives two programs, each with only its
  instances; switching the part off empties both; moving an instance keeps its number;
  overlap is checked per board.
- **Stock:** the user's example: *Side* and *Bottom* imported on one 18 mm board, *Bottom*
  set to 6 mm moves to a new 6 mm board made like the first (fit, zero, material), two
  programs, *through* depths 18 and 6; set back, it returns and the made board goes; a
  second part set to 6 mm joins the same board; a board renamed by hand isn't removed when
  emptied; a part dragged onto a mismatched board stays there with a warning; nesting skips
  a board of the wrong stock; an overflow board takes the part's stock.
- **Nesting overflow:** nothing is switched off while a board can be added; the boards made
  are like the first; numbers kept; deterministic.
- **Window:** sheets side by side, a click activates a sheet; drag a part onto a board;
  *Move to board…*; undo.
- **The simple case (§2a):** an imported DXF with a profile and a pocket shows no Setups
  branch and no instance rows, its Browser has fewer rows than 0.37's, and the walk from
  import to export takes no more clicks; adding a second board shows the branch, removing
  it hides it again; `chipload import`, `op add`, `run` need no `--setup`.
- **CLI:** `instance move`, `--setup`, the old paths refused with the new ones named.

## 13. Decisions

1. **Parts are definitions; setups hold the instances** (§2). **Recommended:** the user's
   model; a part on several boards, overflow, every instance alike. Alternative: keep parts
   in setups and let an instance name another setup (nesting.md §11.1, first way): smaller,
   but instance 1 stays special and a part still "belongs" somewhere.
2. **A part's stock is material and thickness, optional** (§4). **Recommended:** says what
   the part needs; empty, it asks nothing. Alternative: a full stock (size too) on the part,
   which a board already has.
3. **One drawing as the design space**, shapes edited where drawn (§2). **Recommended:** how
   the user draws (a DXF, a layer per part) and how every operation selects shapes today.
   Alternative: each part's shapes move into the part, in its own coordinates: a cleaner
   definition, but every import, selection and shape command changes.
4. **Sheets side by side, the active one in place** (§7). **Recommended:** editing and the
   rulers stay as today; the other boards are seen, not stacked. Alternative: every setup
   on its own sheet in a fixed row with the drawing apart from all of them (a slicer's
   layout exactly); switching boards wouldn't move anything, but a migrated project's
   board would no longer sit on its shapes.
5. **Numbers per part across the project**, kept when an instance moves board (§2).
   **Recommended:** *Side* #3 is the same piece wherever it is cut. Alternative: numbers
   per board.
6. **Nesting overflow onto more boards, off by default** (§9). **Recommended:** today's
   behaviour unless asked; boards made like the first. Alternative: on by default.
7. **The paths change without aliases** in `get`/`set` (§5), the old ones refused with the
   new one named. **Recommended:** one way to say it (no worse options). Alternative: the
   old paths kept as aliases.
8. **Board surfaces and dowel holes stay as they are** (operations of the setup's first
   part) in this spec. **Recommended:** moving them to the setup is a cleanup of its own, and
   doing it so no program changes needs its own migration rules (a board surface that isn't
   first in its setup). Alternative: move them now, with that migration.
9. **Order of work** as §11, the model first with no visible change. **Recommended.**
10. **The simple case shows no boards and no instances** (§2a): one board means no Setups
    branch (Stock and Work zero are the project's rows), a part once where drawn has no
    instance rows. **Recommended:** the user's rule; simpler than today, where a one-board
    project already shows *Setups › Setup 1*. Alternative: always show the Setups branch,
    as now, for one shape of the Browser in every project.
11. **A part's stock picks its board** (§4): changing it moves the part to a matching board,
    made like its board when there is none and removed again when a board made so is
    emptied; placing by hand onto a mismatched board is a warning, not a move.
    **Recommended:** the user's two-thickness DXF is one field, and the user never has to
    choose "a setup". Alternatives: only a warning, the user adds the board and moves the
    part (as 0.37); or a mismatch refused outright.
