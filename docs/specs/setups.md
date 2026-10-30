# Spec: setups

Status: **approved 2026-10-30**, all decisions as recommended ("we will probably need to
iterate on this"). Step 1 built (decision 7): the model, migration 33 → 34, validation, the
path aliases. In code, `job.stock()` and `job.work_zero()` are the active setup's and
`job.for_setup(id)` is a project of one setup for planning; problem paths are
`setups.N.stock.x`, and the forms name them `stock.x` and `work_zero` as before. The aliases
print no note: they just work.
Step 2 built: `run_job` and planning take the active setup only; `run_setups` runs each
(`-S<id>` in the file names with several); a board that fits fits the shapes its setup's
operations may pick (`Selector::may_pick`, only with two or more setups); `setup
list|add|set|activate|part|remove`; `--setup` on `run`, `plan`, `sheet`, `simulate`; a
guide page. A new setup gets a part of its own (both active), so new operations have a home.
Step 3 built: each sheet's title names its setup when there are several; the simulation
reads the active setup's board; a two-setup golden (tests/corpus/jobs/v34/two_setups:
12 mm sides, a 6 mm bottom with its own point zero).
Step 4 built: the Browser's *Setups* (two or more; a click makes one active), *Add a setup*
on *Parts*, a setup's form (name, on/off, *Open* to its board and zero, parts, remove),
*Move to setup ▸* on a part; other setups' boards and contours drawn as ghosts; the preview
is the active setup's (the active part only when it is in it with operations). Export
writes the active setup's program named `-S<id>` with its sheet, and says to export the
others in turn: planning every setup at once would block the window (to iterate on).
Fixed on the way: every operation is checked against its own setup's board, and safe Z
against the highest board.
Then (the user, 2026-10-30): the Browser always shows *Setups*, each with its *Work zero*
and its parts as leaves; a setup's properties hold its name, on/off, the Stock (with max
depth below stock) and Material sections and the zero's line; *Job settings* keeps the
machine and is renamed *Machine*. Another setup's form offers to make it active first.

## 1. Why

The user, 2026-10-29: a work zero per part. That was left for *setups*
(docs/specs/work-zero-marker.md), because one program has one zero: two zeros in one program
would mean stopping the machine to measure again in the middle, which is where crashes come
from.

What the user wants, in the shop's words: **a project whose parts are cut from more than
one board, or from one board clamped more than once.** For example:

- A box: the sides from 12 mm plywood, the bottom from 6 mm. Today that is two projects with
  the drawing copied between them, or one project with the stock thickness changed by hand
  between runs.
- A long part on a short machine, or a big sheet with parts far apart: one board, but the
  zero set again near each group of parts.
- Later: the same board turned over for its back side (§9).

A **setup** is one clamping of one board on the machine: its stock, its work zero, and the
parts cut from it. Each setup is its own program, run on its own, with its own setup sheet.
Fusion 360 calls the same idea *Setups*, so users coming from it expect the word.

## 2. What a setup is

- **Its own:** a name, on/off, the **stock** (size, thickness, fit, Z zero, max overcut,
  material) and the **work zero**: everything `stock` and `machine.work_zero` hold today.
- **Its parts:** every part belongs to exactly one setup; every setup has at least one
  part (an empty one is allowed while it is being set up, and is skipped with a note).
- **Shared by the project:** the drawing, the machine (its travel, post, units, safe Z,
  tool change), the tools, the materials. One drawing for all setups (decision 2): the user
  draws all parts in one DXF, a layer per part, and each setup's board is placed around its
  parts in drawing coordinates.
- **Always at least one.** A project made today has one setup, and looks and works as now
  (§7).

## 3. The board in the drawing (decision 2)

Each setup's stock is a rectangle in drawing coordinates, as today. With *Fit* on (the
default since schema 33), it is the outline of **its own parts'** cuts plus the margin,
not of the whole drawing; a fixed size is centred on its parts' outline. So two setups' boards
lie side by side in the view, each around its parts, and can overlap: they are different
boards.

The work zero is per setup, chosen and dragged as today (a stock corner, the drawing origin
or a point).

## 4. Running (decision 3)

- **One program per setup, never one across setups.** A setup's operations are planned,
  checked and written exactly as a project is today: its parts in order, copies, tool
  groups, its stock for the checker, its zero for the numbers.
- **Files:** with one setup, the names are as today (`out.nc`, `out-1-T1.nc`, …). With
  more, the setup's number goes first: `out-S1.nc`, `out-S2-1-T1.nc`, …; the setup sheet
  likewise (`out-S1-sheet.html`).
- `run --setup N` writes one setup; *Export G-code of this setup…* in the window, as parts
  have it (docs/specs/part-export.md).
- **Off:** a setup that is off isn't cut, like a part.
- The **time estimate** and report: per setup, and the sum.

## 5. Planning, checking, simulating

- Planning runs per setup, with that setup's stock (top Z, thickness, `through` depths,
  max overcut) and parts. Everything that reads `job.stock` today reads the setup's.
- The **checker** gets the setup's stock and zero, as it gets the project's today.
- The **simulation** is per setup: the active setup's board is simulated and shown. The
  others are **ghosts**, as PrusaSlicer shows its inactive plates (the user, 2026-10-30):
  their boards, shapes and toolpaths drawn faint, not pickable; the active one in full
  colour.
- `through` follows its setup's thickness: the 6 mm bottom's outline goes through 6 mm.

## 6. The file (schema 34, decision 4)

```json
"setups": [
  { "id": 1, "name": "12 mm sides", "enabled": true,
    "stock": { "x": 300, "y": 200, "thickness": 12, "fit": 10, ... },
    "work_zero": { "type": "stock", "corner": "bottom_left" } },
  { "id": 2, "name": "6 mm bottom", ... }
],
"active_setup": 1,
"parts": [ { "id": 1, "name": "sides", "setup": 1, ... }, ... ]
```

- `stock` and `machine.work_zero` move into `setups`. Migration 33 → 34: one setup, id 1,
  "Setup 1", with the old stock and zero; every part gets `setup: 1`. Pure, and the G-code
  stays byte-identical (a frozen v33 sample and its golden).
- Machine profiles (`machines.json`) keep their `work_zero` as the zero a **new** setup
  starts with: its meaning changes from "the project's zero" to "a new setup's zero", which
  doesn't change any file, so no bump there.
- Paths for `get`/`set` change: `stock.thickness` becomes `setups.0.stock.thickness`.
  `stock.*` and `machine.work_zero` stay accepted on the command line as the active setup's,
  with a note (decision 5).

## 7. The window (decision 6)

- **One setup** (nearly every project): nothing changes. Stock and Zero stay in Job settings,
  parts sit under *Parts*. A right-click on *Parts* offers *Add a setup* for when it is
  needed.
- **Two or more:** the Browser shows *Setups*, each with its board (stock and zero, its
  properties) and its parts under it:

```
▾ Setups
  ▾ ☑ 12 mm sides       2 parts
      ⊞ Board  300 × 200 × 12
      ⌖ Work zero  bottom-left
      ▸ ☑ sides ×2
  ▾ ☑ 6 mm bottom       1 part
      ⊞ Board  280 × 180 × 6
      ⌖ Work zero  bottom-left
      ▸ ☑ bottom
```

- The **active setup** (bold) is the one shown in the view and simulated, where new parts
  go, and what *Generate* previews; clicking a setup's row makes it active. The others are
  ghosts in the view (§5).
- A part moves between setups by dragging it, or by *Move to setup ▸* on its right-click
  menu.
- Removing the last-but-one setup brings back the one-setup Browser.

## 8. Command line

- `chipload setup list | add --name NAME [--thickness 6] | set ID … | remove ID |
  activate ID`; `part set ID --setup N`.
- `run`, `plan`, `sheet`, `simulate`: every setup by default, `--setup N` for one.
- `zero` and `place … --align` act on the active setup, or `--setup N`.

## 9. Not now: turning the board over

Two-sided work (a setup that is the same board turned over, its drawing mirrored, located
with dowel pins or a fence) is the next step this makes possible, and its own spec: it needs
mirrored geometry, registration holes and a check that the two sides agree.

## 10. Decisions

1. **A setup is one board clamped once: its stock and work zero, and the parts cut from
   it.** **Recommended.** Alternative: only a work zero per part, without its own stock: no
   second thickness, and still two zeros in one program unless it is split anyway.
2. **One drawing for all setups,** each board placed around its parts in drawing
   coordinates. **Recommended:** how the user draws (a layer per part), nothing to place
   twice. Alternative: each setup its own sheet, with its parts placed on it by hand (a
   nesting view). More flexible, much more to build; could come later on top.
3. **One program per setup,** files named `-S2…` when there are several, unchanged with one.
   **Recommended.** Alternative: one program with a pause (M0) and a message between setups:
   one file, but the operator re-zeroes mid-program, the riskiest moment.
4. **Schema 34 moves `stock` and `work_zero` into `setups`,** with a migration.
   **Recommended:** one way to say it. Alternative: keep the top-level stock as setup 1's and
   add `setups` only for the others: no bump, but two places for the same thing ("no worse
   options").
5. **The old paths `stock.*` and `machine.work_zero` keep working on the command line** as
   the active setup's, with a note. **Recommended:** scripts and the MCP tools the user may
   have keep working. Alternative: only the new paths.
6. **The window shows setups only when there are two or more.** **Recommended:** a
   one-board project, the usual case, stays as simple as today. Alternative: always show the
   Setups level. *Changed by the user after trying it, 2026-10-30: setups always, even one,
   with the parts beneath them as leaves; the stock, material and zero move from Job
   settings into the setup, and Job settings, the machine alone, is renamed Machine.*
7. **Order of work** (each step a commit, the goldens unchanged until step 3):
   1. the model, migration 33 → 34, validation, `get`/`set` aliases;
   2. planning, checking and running per setup; file names; `setup` commands; `--setup`;
   3. the setup sheet and the simulation per setup; a two-setup golden (the box: 12 mm sides
      and a 6 mm bottom);
   4. the window: Browser, properties, active setup in the view, export per setup;
   5. the guide (*Setups* page), schemas, translations.
   **Recommended.**
8. **Turning the board over is a later spec** (§9). **Recommended.**
