# Spec: a tool per operation

Status: **draft 2026-10-11**, for review. Decisions in §9.

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

## 3. Output: one program per tool group

Each tool group becomes **its own G-code program and file** (decision 2), complete on its own:
units, spindle start with that tool's rpm, dwell, the moves, spindle stop, end of program. Between
files the user changes the tool and sets Z zero with the new one, the way it's done with
GRBL. X and Y zero must not move. Every program starts at safe Z and goes to it before the
first XY move, as now.

Each file's header says, as comments:

```
(Chipload - GRBL)
(File 2 of 3: tool 2, 3.000 mm endmill "3 mm downcut")
(Operations 2, 4)
(Set Z zero with this tool. Keep X and Y zero.)
(Tool 2: 3.000 mm)
(Work zero: …)
```

The new lines appear only when there is more than one file, so a one-tool project's output
is unchanged. The library name is written only when the tool has one, with `(`, `)` and `;`
replaced as for the work zero.

**File names** (decision 3): with one group, the file is the one asked for (`-o part.nc`),
as now. With several groups, `-o part.nc` writes `part-1-T1.nc`, `part-2-T2.nc`, … (the
group number in cut order, then the tool id), and no `part.nc`. Any old `part.nc` is left
alone. Before writing, every file is generated and checked. If one fails, none is
written. Each file is written atomically. `run` lists the files it wrote.

*In the program* tool changes (`M6 T2`, for a sender that handles it) are **not part of this
step** (decision 2). They would need a post setting, template blocks and checker rules for
`T`/`M6`, and the tool length offset is up to the sender. That can be added later as a post
option without changing projects.

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

The checker stays independent. Each program is checked **on its own**, with the stock margin
of **its** tool's radius:

- `limits_from_job(job)` keeps its meaning (the largest radius of the tools in use).
  A new `limits_for_tool(job, id)` uses one tool's radius. `run_job` checks each file with
  its own tool.
- `chipload check FILE --job PROJECT` checks with the largest radius, as now, so any file of
  the set passes or fails as before. With `--tool ID` it checks with that tool's radius
  (decision 5).

## 7. CLI

- `run part.chipload -o part.nc` writes one file or the set (§3) and prints each one:
  `Wrote part-1-T1.nc (tool 1, operations 1, 3)`.
- `plan --json` prints the groups: `"groups": [{"tool": 1, "operations": [0, 2], "moves":
  [0, 812]}, …]`, next to the program, which keeps its fields. Its `tool_id`, `tool_diameter`
  and `rpm` are the first group's, so `render` of a one-tool plan is unchanged. `render` of a
  multi-tool plan is refused: "this plan uses 2 tools; use run".
- New: `chipload op tool PROJECT --op N --tool ID` sets operation N's tool (1-based, as
  listed), so the feature works without editing the file (CLAUDE.md rule 5). It refuses an id
  that isn't in the project, and says to run `tool use` first. It doesn't change feeds.
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
- **Save G-code** with several groups saves the set next to the chosen name, as in §3. It
  lists the files in the report.
- The operation list shows the tool on each row ("T2"), and a thin line between groups
  marks a tool change.
- The 2D and 3D views draw each operation's path as now, with that operation's tool.

## 9. Decisions

1. **Order.** Recommended: **keep the user's operation order**, a tool change wherever the
   tool differs from the operation before, plus a note when a tool comes back. Alternative:
   group by tool automatically (fewest changes), but that can cut the part free before its
   pockets.
2. **How a tool change reaches the machine.** Recommended: **one file per tool group**. That's
   what works on plain GRBL, the user sets Z zero with each tool, and each file is checked on
   its own. `M6` in one program, for senders that handle it, can come later as a post
   option. Alternative: `M0` pause with comments in one file, but GRBL can't jog in that
   pause, so Z zero can't be set.
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
7. **No schema change.** The format already allows it. Recommended: **no bump**. An older
   Chipload opens a multi-tool project and refuses to run it with its "one tool per job"
   message, which is the right outcome.

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
- Post: one-tool output byte-identical (all golden files unchanged). The header lines appear
  only for several files.
- Checker: `limits_for_tool`. `check --tool`. A move outside the small tool's margin but
  inside the large one's: a warning with `--tool small`, none without.
- CLI: `run` writes `-1-T1`, `-2-T2` and no plain file; `op tool` sets and refuses an
  unknown id; `plan --json` groups; `render` refuses a multi-tool plan.
- Golden: a new **v11** sample `tests/corpus/jobs/v11/two_tools.chipload` (pocket with a
  3 mm tool, profile with 6 mm), with golden files `expected/v11_two_tools-1-T2.nc` and
  `-2-T1.nc` (and `lines/`).
- GUI: the tool combo changes the tool (undo); a tool in use can't be removed; *Add from
  library* adds and selects it; the new operation inherits the tool above.
