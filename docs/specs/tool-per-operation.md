# Spec: a tool per operation

Status: **approved 2026-10-11**, with all decisions as recommended (§9). Revised before
approval: both one file per tool and one file with tool changes, as a machine setting (user
request). Built as described, with the differences in §11.

Guide: docs/book/src/concepts/tools.md.

User request (2026-10-11): each operation picks its own tool, with a tool change between
them, instead of one tool per project. It is also what the V-carve flat floor needs (a V-bit
for the walls, an endmill for the floor; TODO.md, Later).

## 1. What exists

- The project already holds a **list** of tools (`tools`, each with an `id`), and every
  operation names one (`tool`). The format needs **no change** and no schema bump.
- `validate_job` refuses a project whose enabled operations use more than one tool: "this
  version supports one tool per job". Planning, the post-processor and the checker build
  on that: `Program` has one `tool_id`, `tool_diameter` and `rpm`; the template starts the
  spindle once; the checker's stock margin is the largest tool radius in use.
- The GUI shows one *Tool* section (the project's first tool), and new operations get that
  tool. `tool use NAME project` already copies a library tool into a project and prints
  its id there (ADR 0014); the CLI has no command to set an operation's tool, so that is done
  in the project file or the GUI.
- GRBL 1.1, the only built-in post, refuses `M6` (error 20). An `M0` pause holds the machine,
  and GRBL won't jog while it is held, so the new tool's Z zero can't be set there. Senders
  such as gSender, UGS and CNCjs can handle `M6` themselves, each in its own way.

## 2. Tool groups

The enabled operations run **in the order the user listed them** (decision 1). Each run of
operations next to each other that use the same tool is a **tool group**. A project with one
tool has one group and gives exactly the same G-code as today (every golden file
unchanged).

When a tool comes back after another one (operations 1 and 3 use tool 1, operation 2 tool
2), that's three groups and two changes. The report says so: "Tool 1 is used in two groups
(operations 1, 3); moving them next to each other saves a tool change". Nothing is reordered
automatically: the order often matters (pocket before cutting the part free).

**Copies** (docs/specs/copies-grid.md): the tool group is the outermost loop (decision 4).
Within a group, copies follow the project's order: *operation by operation* cuts each
operation on every copy, *part by part* cuts all of the group's operations on copy 1, then
copy 2. So a grid of 20 parts with two tools needs one tool change, not 20.

## 3. Output: one file per tool, or one file with tool changes

There are two ways to write a project with several tool groups (decision 2). The choice is
the machine's **tool change** setting (§3.3). A project with one tool group writes exactly
what it writes today in both modes, so every existing golden file stays unchanged.

### 3.1 One file per tool group (`files`)

Each tool group becomes **its own G-code program and file**, complete on its own: units, spindle
start with that tool's rpm, dwell, the moves, spindle stop, end of program. Between files the
user changes the tool and sets Z zero with the new one, the way it's done on plain GRBL. X and
Y zero must not move. Every program starts at safe Z and goes to it before the first XY
move, as now.

Each file's header says, as comments:

```
(Chipload - GRBL)
(File 2 of 3: tool 2, 3.000 mm endmill "3 mm downcut")
(Operations 2, 4)
(Set Z zero with this tool. Keep X and Y zero.)
(Tool 2: 3.000 mm)
(Work zero: …)
```

The new lines appear only when there is more than one file. The library name is written
only when the tool has one, with `(`, `)` and `;` replaced as for the work zero.

**File names** (decision 3): `-o part.nc` writes `part-1-T1.nc`, `part-2-T2.nc`, … (the
group number in cut order, then the tool id), and no `part.nc`. Any old `part.nc` is left
alone. Before writing, every file is generated and checked. If one fails, none is written.
Each file is written atomically.

### 3.2 One file with tool changes (`program`)

One program for the whole project. At each tool change the template writes (GRBL post):

```
(Tool change 1 of 2: tool 2, 3.000 mm endmill "3 mm downcut", operations 2, 4)
G0 Z<safe>
M5
T2 M6
M3 S18000
G4 P3
```

- The tool is already at safe Z, since every group ends retracted. The explicit `G0 Z<safe>`
  is there so the change never depends on that.
- The spindle is stopped before `M6` and started again after it, with the new tool's rpm and
  direction and the post's dwell, before any cutting move.
- The program's first tool gets `T1 M6` too, so a sender's tool change routine knows which
  tool is in. The header lists every tool:
  `(Tools: T1 6.000 mm endmill, T2 3.000 mm endmill)` and says
  `(Tool changes use M6: your sender must handle them and set Z zero for each tool. GRBL
  itself refuses M6.)`.
- Where the machine goes for the change (a tool change position) and how Z zero is set
  again (probe, manual touch-off) are **up to the sender** (gSender, UGS, CNCjs and bCNC each
  have a tool change setting). Chipload doesn't move to a change position in this step
  (decision 8).
- These lines come from the post's template, so another post can write its own commands
  (data-driven, CLAUDE.md rule 7). The post config says which modes the template supports:
  `tool_changes = ["files", "program"]`. That's an optional field. Missing means `["files"]`,
  so a post that doesn't know about `M6` never writes it. A project asking for a mode its post
  doesn't support is refused when run.

With one tool group, `program` writes the same file as today: no `T`, no `M6`. The output of
one-tool projects doesn't change at all.

### 3.3 The setting

`machine.tool_change`: `"files"` or `"program"`, in the project's machine and in machine
profiles (`machines.json`), so it travels with the machine like `post` and `safe_z`. It's
a choice of how the user's machine and sender work, not a per-run flag, so the same project
always gives the same output.

- New projects and profiles get `"files"`, which works on every GRBL setup.
- The CLI sets it with `machine set NAME --tool-change program` (profile) and
  `run --tool-change files|program` for one run without changing the project
  (decision 9). The GUI sets it in the *Machine* section: "Tool changes: one file per tool /
  one file with M6".
- **Format:** it's a new field, required when saved. Recommended as **project schema 12**
  with a migration v11 → v12 that adds `"files"`, and **machines.json 2** with the same
  upgrade (decision 7). The v11 sample stays frozen with its golden file.

## 4. Validation per operation

Everything `validate_job` checks against "the tool" is checked against **the operation's
own tool**: it exists, it has an rpm, clearance and helix radius less than its radius, lead
radius, endmill for pockets, V-bit with an angle for V-carves, centre-cutting for plunges.
The messages already name the operation and the tool, so they stay. The "one tool per job"
error goes.

## 5. Planning

- Each operation is planned with its own tool's diameter (offsets, stepover, tabs, leads,
  helix, material left, the V-bit's angle). Today they all read the one tool, so this is
  mostly passing the right tool down.
- The plan keeps **one toolpath for display**, in cut order. It gains the tool groups: the
  tool and the range of moves of each. The post renders each group's moves as one program.
  Each group's moves start with the retract to safe Z, so a group's program never starts
  below it.
- **Material left** and the **3D view** use each operation's own tool radius.
- **Copies:** the overlap check and `cut_size` (gap → pitch) take each move's tool radius,
  not the one tool's.

## 6. Checker

The checker stays independent. Each program is checked **on its own**:

- **One file per group:** with the stock margin of **its** tool's radius.
  `limits_from_job(job)` keeps its meaning (the largest radius of the tools in use). A new
  `limits_for_tool(job, id)` uses one tool's radius. `run_job` checks each file with its own
  tool. `chipload check FILE --job PROJECT` checks with the largest radius, as now, and with
  `--tool ID` with that tool's radius (decision 5).
- **One file with `M6`:** the checker learns `T` and `M6`. Its limits get every tool's
  radius by id from the job (`limits_from_job`), and it checks:
  - `M6` needs a `T` word on the line or before it. A `T` not in the project is an
    error.
  - At `M6` the spindle is off (`M5` before it) and Z is at or above safe Z. Otherwise it's
    an error.
  - After `M6` the spindle is off until `M3`/`M4` with an `S` word. A cutting move before
    that is an error, the same rule as the program start.
  - From `M6` on, the stock margin uses that tool's radius. Before any `M6` (a one-tool
    program) it uses the largest radius, as now.
  - `T` without `M6` changes nothing (GRBL ignores it too).

## 7. CLI

- `run part.chipload -o part.nc` writes the set (§3.1) or the one file (§3.2), as the
  machine says, and prints each file: `Wrote part-1-T1.nc (tool 1, operations 1, 3)`.
  `--tool-change files|program` overrides it for this run.
- `plan --json` prints the groups: `"groups": [{"tool": 1, "operations": [0, 2], "moves":
  [0, 812]}, …]`, next to the program, which keeps its fields. Its `tool_id`, `tool_diameter`
  and `rpm` are the first group's, so `render` of a one-tool plan is unchanged. `render` of a
  multi-tool plan is refused: "this plan uses 2 tools; use run".
- New: `chipload op tool PROJECT --op N --tool ID` sets operation N's tool (1-based, as
  listed), so the feature works without editing the file (CLAUDE.md rule 5). It refuses an id
  that isn't in the project, and says to run `tool use` first. It doesn't change feeds.
- `machine set NAME --tool-change files|program` for a profile. `machine use` copies it into a
  project, as it does every machine value.
- `tool use NAME PROJECT` stays as it is (copy in, print the id).

## 8. GUI

- **Tools** section (replacing *Tool*): the project's tools, each a row "T1 endmill Ø6 mm
  "6 mm upcut"" that opens to the same fields as today. Below the list are *Add from library…*
  (copies the tool in, as `tool use` does) and *New tool*. A tool that an operation uses can't
  be removed: the button is disabled, and its tooltip names the operations. The library rows
  (update from library, save to library) work on the open tool.
- **Each operation** gets a *Tool* combo box listing the project's tools, with "+ From
  library…" at the bottom. Changing it is one undo step.
- A **new operation** gets the tool of the operation above it, else the first tool. Its feeds
  come from that operation, or the tool's library cutting values (as now).
- Changing an operation's tool **keeps its feeds**. When the new tool has library cutting
  values that differ, a *Use the tool's cutting values* button appears (decision 6).
- **Save G-code** writes what the machine's tool change setting says: the set next to the
  chosen name (§3.1) or one file (§3.2). The report lists the files.
- *Machine* section: **Tool changes**: "One file per tool" / "One file with M6 (your sender
  handles the change)".
- The operation list shows the tool on each row ("T2"), and a thin line between groups
  marks a tool change.
- The 2D and 3D views draw each operation's path as now, with that operation's tool.

## 9. Decisions

1. **Order.** Recommended: **keep the user's operation order**, a tool change wherever the
   tool differs from the operation before, plus a note when a tool comes back. Alternative:
   group by tool automatically (fewest changes), but that can cut the part free before its
   pockets.
2. **How a tool change reaches the machine.** Per your request: **both, as a machine setting**:
   one file per tool group (works on plain GRBL; the user sets Z zero with each tool), or one
   file with `T<n> M6` for senders that handle tool changes. Recommended default for new
   projects: **one file per tool**, since plain GRBL refuses `M6`. Alternative for the single
   file: `M0` pause instead of `M6`, but GRBL can't jog in that pause, so Z zero can't be set
   again. That would cut with the wrong tool length.
3. **File names.** Recommended: **`part-1-T1.nc`, `part-2-T2.nc`** (cut order, then tool id),
   and the single file unchanged for one tool. Alternative: a folder per job, or the
   library name in the file name (can hold spaces and odd characters).
4. **Copies with several tools.** Recommended: **tool groups outermost**, the copy order
   within a group. Alternative: part by part across tools, which means a tool change per part
   and is refused.
5. **Checker radius for `check`.** Recommended: **the largest radius by default, `--tool ID`
   for one tool's**, while `run` checks each file with its own. Alternative: the checker
   reads the tool from the file's comments. That's text the generator wrote, so the checker
   would trust the generator.
6. **Feeds when an operation's tool changes.** Recommended: **keep them**, and offer a
   button to use the new tool's library cutting values. Alternative: replace them
   automatically, which could silently overwrite tuned values.
7. **Format.** Using several tools needs no change; the tool change setting does.
   Recommended: **project schema 12** (migration adds `tool_change: "files"`) and
   **machines.json 2** (same). Both are required fields, for the same reason as the
   placement's turn and scale: an older Chipload then says "newer version" instead of
   "unknown field". An older Chipload given a multi-tool project of schema 11 refuses to run
   it with its "one tool per job" message.
8. **Tool change position.** Recommended: **none in this step**. Retract to safe Z, stop the
   spindle, `T M6`, and the sender moves where it's set up to. A machine `tool_change_xy`
   could come later.
9. **Where the choice is made.** Recommended: **in the machine** (project and profile), with
   `run --tool-change` as a one-off override. Alternative: a choice only when saving G-code
   (not stored). Then the same project could give different files depending on a dialog.

## 10. Tests

- Toolpath: a profile with tool 1 and a pocket with tool 2 plan with each tool's diameter
  (each operation's toolpath equals the one planned alone in a one-tool project). Groups:
  1-1-2 gives two groups, 1-2-1 three, plus the note. A disabled operation between two
  operations with the same tool doesn't split them. Validation messages name the
  operation's own tool (V-carve with an endmill, pocket with a V-bit, missing rpm on the
  second tool).
- Copies: two tools with a 2 × 2 grid, both orders: one change, and the copy order inside
  each group.
- Pipeline: `run_job` gives one checked program per group. Each starts at safe Z, has its
  own tool's S value and header, and passes the checker with its own tool's radius. A group
  whose program fails the checker fails the whole run, and nothing is written.
- Post: one-tool output byte-identical in both modes (all golden files unchanged). The header
  lines appear only for several files. `program` mode writes `T M6` at each change with
  `G0 Z`, `M5` before it and `M3 S` and the dwell after. A post without `program` in
  `tool_changes` refuses that mode.
- Checker: `limits_for_tool`. `check --tool`. `M6`: refused with the spindle on, below
  safe Z, without `T`, with an unknown `T`; a cut after `M6` before `M3` is refused; the
  margin follows the tool after `M6`. A move outside the small tool's margin but
  inside the large one's: a warning with `--tool small`, none without.
- CLI: `run` writes `-1-T1`, `-2-T2` and no plain file in `files` mode, one file in
  `program` mode, and `--tool-change` overrides it; `op tool` sets and refuses an
  unknown id; `plan --json` groups; `render` refuses a multi-tool plan.
- Job: v11 → v12 adds `tool_change: "files"`, machines.json 1 → 2 the same. The v11 sample
  migrates with the same golden file.
- Golden: **v12** samples `tests/corpus/jobs/v12/two_tools_files.chipload` (pocket with a
  3 mm tool, profile with 6 mm), with golden files `expected/v12_two_tools_files-1-T2.nc`
  and `-2-T1.nc`, and `two_tools_program.chipload` (the same, one file with `M6`), plus their
  `lines/` versions.
- GUI: the tool combo changes the tool (undo); a tool in use can't be removed; *Add from
  library* adds and selects it; the new operation inherits the tool above.

## 11. As built

- **Steps:** one file per tool (04bc5e3), the setting with project schema 12 and
  machines.json 2 (2f5cf5d), one file with `M6` (08ab49b), then the GUI.
- **Samples:** the one-file-per-tool golden comes from a **v11** sample
  (`jobs/v11/two_tools.chipload`, files `v11_two_tools-1-T2.nc` / `-2-T1.nc`): it was made
  before schema 12 and now also checks that the v11 → v12 migration keeps the output. The
  `M6` sample is `jobs/v12/two_tools_program.chipload`. The golden test collects every
  file a run writes.
- **One file with `M6`:** the header lists each tool once, as `(T2 3.000 mm endmill "…")`,
  in the order first used. A tool group starts with `G0 Z<safe>` itself, so after the
  change's `G0 Z<safe>` the retract can appear twice: harmless, and explicit.
- **`render`** reads only the program, so `plan --json` also writes `"tool_groups": N` into
  the program, and `render` refuses N > 1.
- **Checker:** `CheckLimits.tool_radii` holds every tool of the job by number. `T` alone
  changes nothing. `M6` is checked before the spindle words of its line, the order a
  controller uses.
- **GUI:** *Tools* lists every project tool in a section of its own (type, size, library
  rows, *Remove tool*, disabled with the operations named while it is used). *Add from
  library…* and *New tool* (a 6 mm endmill) sit above them. The operation's *Tool* box lists
  the project's tools and then the library's ("+ name"). Picking one copies it in and
  selects it, as one undo step. The operation cards show `T<n>`, and a line marks each
  tool change.
