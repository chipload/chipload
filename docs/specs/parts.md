# Spec: parts

Status: approved 2026-10-13. Cutting order decided by the user: **part by
part**.

## 1. Why

User request (2026-10-13): "We could make Parts a top category instead of operations, each
part would then have operations. Most of the time we only create one part but sometimes we
want several parts from the same drawing material. Each part would then have the copies
option."

Today **copies** (docs/specs/copies-grid.md) belong to the whole job, and every operation
is cut on every copy. A sheet with 4 of part A and 2 of part B can't be made. CamBam calls
the same idea **Parts**: each holds its own operations, can be turned off, and is
copied on its own.

## 2. What a part is

A part is a **named group of operations**, cut together, with its own **copies** and an
**on/off** tick.

- **No geometry of its own.** Its operations select shapes, DXFs, layers and pictures as
  they do now. So one DXF can feed several parts (a layer per part, as the user draws), and
  one part can take shapes from several DXFs.
- **At least one.** Every project has at least one part, and every operation belongs to
  exactly one part.
- **Its area** is the area its operations cut on copy 1: the toolpath's extent plus the
  tool radius, as copies use today. Copies are spaced and checked by it.

```
Parts
 ▾ ☑ lid          ×4      ← 2 × 2 copies
     ☑ 1  Holes       T1
     ☑ 2  Outline     T1
 ▾ ☑ box          ×2
     ☑ 3  Pocket      T2
     ☑ 4  Outline     T1
```

## 3. Cutting order

**Part by part** (user's decision): the parts in their order, and within each part its
operations in order. Each part, with all its copies, is finished before the next starts.

- **Copies within a part:** their order is as today, a setting of the part's copies.
  - **Operation by operation** (`by_operation`): the default.
  - **Copy by copy** (`by_copy`, today's `by_part`, renamed because "part" now means this).
  - Copies are still visited serpentine.
- **Tool changes:** tool groups form as today, from runs of consecutive operations with the
  same tool. Because order is by part, a T1 → T2 project with two parts changes tool three
  times (T1, T2, T1, T2), and "one file per tool" writes four files.
  - The report already counts tool changes.
  - Grouping by tool across parts was considered and left out (user's decision). It can
    come later as a job setting if missed.
- **A part that is off** is left out with all its operations. Each operation's own tick
  still works inside a part that is on.

## 4. Project file (schema 22)

```json
"parts": [
  {"id": 1, "name": "lid", "enabled": true,
   "copies": {"columns": 2, "rows": 2, "dx": 120.0, "dy": 80.0, "order": "by_operation"}},
  {"id": 2, "name": "box", "enabled": true, "copies": null}
],
"operations": [
  {"type": "profile", "part": 1, …},
  …
]
```

- **`parts`:** required, at least one.
  - **`id`:** unique positive whole numbers, as DXF ids.
  - **`name`:** not empty, and unique among the parts.
  - **`enabled`:** on or off.
  - **`copies`:** today's `copies` object, or null for one copy.
- **`operations`:** stays one flat list, so paths like `operations.2.end_depth` keep
  working in `get`, `set`, problem messages and scripts.
  - Each operation gets a required **`part`**, which must name a part.
  - **Operations are kept grouped by part, in the parts' order.** Validation refuses a file
    whose operations are out of that order, naming the first one, e.g. "operations.3: part 1's
    operations must all come before part 2's". The app and the command line always write
    them grouped. The list's order is then the cutting order, and operation numbers stay
    global (1, 2, 3 … across parts), as G-code comments, the report and the sheet use them.
- **The top-level `copies` goes.** It moves into the part.
- **`active_part`:** optional, the id of the active part (§8). It has no effect on the
  G-code. Missing or naming no part means the first part.
- **`CopyOrder`:** `by_part` is renamed `by_copy`.

**Migration v21 → v22** (pure, behaviour kept):
- **One part**, `{"id": 1, "name": "Part 1", "enabled": true}`, takes the job's `copies`,
  with `by_part` becoming `by_copy`.
- **Every operation** gets `"part": 1`.
- **G-code:** with one part, the order is exactly today's, so the G-code is identical. The
  golden test runs every frozen project through this.
- **Frozen sample:** `tests/corpus/jobs/v21/two_dxfs.chipload` already exists. A new
  `v21/copies_by_part.chipload` (a grid in `by_part` order) is frozen with the 0.16 binary,
  with its golden G-code, to prove the rename.

## 5. Safety

- **Copies of a part must not overlap each other.** This is today's rule (refused), now
  per part, against the part's own area.
- **Copies must not overlap other parts.** A copy made by a grid (copy 2 and up) that
  overlaps the area of any copy of another part is **refused**: "Part lid copy 3 overlaps
  part box (X … Y …)". Copy 1 of each part is the drawing as drawn: those may overlap,
  since the user drew them so, and an engraving part over a cut-out part is legitimate.
- **Areas are compared as boxes,** as the grid check is today, so a part nested into
  another's concave corner is refused. That is real nesting, still out of scope.
- **Copies past the stock** give a note, as now, naming the part.
- **Checker:** unchanged. Every program still goes through it.

## 5b. Stock

**One stock for the project,** shared by every part (decision 11). Parts are cut from one
piece of material, in one program, on one machine setup with one work zero. Depths are
measured from that stock's top. A part with a different stock or thickness couldn't be cut
in the same run.

Different material is a different setup, and today that means a separate project. Several
setups in one project, each with its own stock, work zero and parts, would be a level
above parts (as Fusion has). It's out of scope here, and nothing in this design blocks it.

## 6. G-code, report and setup sheet

- **One part:** everything is exactly as today.
- **Two or more parts:**
  - **G-code:** a comment at each part's start, `( Part 2: box, 2 copies )`. This goes
    through `comment_safe`. No golden of an existing project changes, since they all have
    one part.
  - **Report:** the times are split by part as well as by operation.
  - **Setup sheet:** it lists the parts with their copies, and each part's operations
    under it.

## 7. Command line

```
chipload part list part.chipload                    # id, name, on/off, copies, operations
chipload part add part.chipload --name box          # an empty part, last
chipload part set part.chipload 2 --name lid --off  # rename, turn off (--on)
chipload part activate part.chipload 2              # the active part (where new operations go)
chipload part remove part.chipload 2                # refused while it has operations,
chipload part remove part.chipload 2 --with-operations   # … unless asked
chipload op part part.chipload --op 4 --part 2      # move operation 4 to part 2 (to its end)
chipload copies part.chipload --part 2 --grid 2x1 --gap 10   # --part needed with 2+ parts
```

- **`part remove`:** refused while the part has operations, unless `--with-operations` is
  given. The last part can't be removed.
- **`op add-rest`:** puts the rest pocket in the same part, right after its pocket.
- **`get` and `set`:** they reach `parts.N.copies.columns` and the rest.
- **`inspect` and `plan`:** they list the parts.
- **JSON:** every `--json` output that listed copies now lists parts.

## 8. App

- **Browser:**
  - **Parts** takes the Operations folder's place. Each part is a folder row: its tick
    (on or off), a part icon, its name, "×4" when it has copies, and a red dot when one
    of its operations has a problem. Its operations are under it.
  - **The part level always shows,** even with one part, so the tree doesn't change shape
    when a second part is added.
  - **Right-click a part:** *New operation* (for the selection), *Move up* and *Move down*,
    and *Delete part*. *Delete part* also deletes its operations, says how many, and is one
    undo step.
  - **Right-click Parts:** *New part*.
  - **Drag an operation** onto another part's row to move it there (to the end), or onto
    an operation of another part to put it before that one. The file stays grouped.
- **Properties for a part:** its name, on or off, and **Copies** (columns, rows, pitch,
  order, *Set pitch* from a gap). Copies move here from Job settings.
- **The active part** (decision 10). One part is always active.
  - **Shown:** its row is bold with a marker (▸), and the status bar says "Active part:
    box".
  - **Changing it:** right-click > *Activate*, or double-click the part. A new part
    becomes active, and deleting the active part makes the first one active.
  - **New operations** from the ribbon, the Operations menu or a right-click always go
    into the active part, at its end. Clicking an operation of another part opens it but
    doesn't change the active part.
- **Canvas:** each part's copies are drawn faded, made of the contours that part's
  operations cut, not the whole drawing. The status bar shows the pieces in all ("6
  pieces").
- **New projects** start with one part, "Part 1".

## 9. Tests

- **Format:**
  - The round trip.
  - Refusals: no parts, a duplicate id or name, an operation naming a missing part, and
    operations out of part order.
  - The migration from v21, with and without `copies`, and `by_part` to `by_copy`.
- **Golden:** every frozen project gives the same G-code through the migration. A new v22
  project with two parts gives T1, T2, T1, T2 groups and copies per part, and passes the
  checker.
- **Plan:**
  - A part that is off is left out.
  - Part by part order, at the boundary where the tool doesn't change between parts.
  - Copies within a part overlapping is refused at just under the pitch and accepted at
    exactly it.
  - A copy overlapping another part is refused.
  - Copies 1 overlapping is allowed.
- **CLI:** `part add`, `set`, `remove` (refused, and `--with-operations`), `op part`, and
  `copies --part` (needed with two parts).
- **App:**
  - New operations go to the active part, whatever operation is open.
  - Activating a part is remembered in the project, and deleting the active part makes
    the first one active.
  - Dragging an operation moves it between parts and keeps the file grouped.
  - Deleting a part deletes its operations, and undo brings them back.
  - Copies edit from the part's Properties.

## 10. Decisions

1. **What a part is:** a group of operations with copies and on/off, owning no geometry
   (§2). *Recommended.*
   - Alternative: a part owns shapes or a DXF. It's simpler to copy, but it breaks a layer
     per part and parts made from several DXFs.
2. **File shape:** a `parts` list, and a `part` id on each operation in one flat
   `operations` list kept grouped by part (§4). *Recommended,* because script paths
   `operations.N` keep working.
   - Alternative: operations nested inside each part (`parts.0.operations.1`). It's tidier
     JSON, but it breaks every existing path and script.
3. **Operation numbers** stay global across parts (1, 2, 3, 4 above), matching the file,
   the G-code comments and the report. *Recommended.*
   - Alternative: numbering per part (lid 1, 2; box 1, 2) reads nicely but stops matching
     everything else.
4. **Rename `by_part` to `by_copy`** in the migration (§3). *Recommended,* since "part" now
   means a part.
5. **Overlap between parts:** generated copies may not overlap any other part's copies,
   while copies 1 may overlap (§5). *Recommended.*
6. **The part level always shows** in the Browser, even with one part (§8).
   *Recommended.*
7. **Default names:** "Part 1", "Part 2" …, for the migration and new parts.
   *Recommended.*
   - Alternative: name a part after the project's first DXF. That reads better, but it makes
     the migration depend on drawing names.
8. **G-code comment per part,** only with two or more parts (§6). *Recommended.*
9. **Order of work:**
   1. The format, the migration and the planner, with the CLI and golden tests.
   2. The setup sheet and the report.
   3. The app.

   Each step is one commit with its guide pages. *Recommended.*
10. **The active part:** one part is always active. It's chosen explicitly (right-click >
    *Activate*, or a double-click) and saved in the project as `active_part`. New
    operations always go there (§8). *Recommended* (the user asked for a choice).
    - Not now: dimming the other parts' toolpaths on the canvas while one is active.
11. **Stock:** one stock for the whole project, shared by all parts. No stock per part
    (§5b). *Recommended.*
    - Different material is a separate project for now, or later a "setup" level above
      parts.
