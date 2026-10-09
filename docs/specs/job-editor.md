# Spec: job editing in the GUI

Status: **approved 2026-10-09** (answers to Q1–Q4 below are folded in)

Today jobs are written by hand as JSON; the GUI only views them and copies selectors. This
spec makes the GUI a job editor: pick geometry, create operations, set their parameters,
reorder them, save the job. Covers user requests 4 (select geometry → operations) and 5
(reorder operations freely).

Out of scope: tool library (later, its own persisted format), multiple tools per job (the
pipeline still allows one), new operation types or pocket strategies, drilling.

## 1. Principles
- **Nothing GUI-only.** The editor produces an ordinary job file; `chipload run` on that file
  gives exactly the G-code the GUI shows. No setting lives only in the GUI.
- **No silent choices** (CLAUDE.md). Values the editor fills in are visible in the form and
  written explicitly to the file. Where no safe default exists, the field starts empty and
  the operation can't be generated until it's filled.
- **Operations run in list order**, exactly as listed (decision 2026-10-08). No reordering by
  the program, no warnings about order (decision 2026-10-09).
- The editor only builds a `Job`; everything else is the existing pipeline.

## 2. Job model change (no version bump)
- New optional field on every operation: `"enabled": true` (default `true`). Disabled
  operations are kept in the file and skipped by the pipeline (and by `chipload run`), so you
  can switch an operation off without losing its settings.
- Optional with a safe default, so no `schema_version` bump (CLAUDE.md). Older Chipload
  versions refuse files with unknown fields, which is the safe outcome.
- A job with every operation disabled is an error ("nothing to cut"), like an empty job.

## 3. Window layout
- **Canvas** (as now): drawing, stock, toolpath.
- **Operations panel** (left): the list in cut order. Each row: enabled checkbox, number,
  type, name, the contours it selects (`#4, #6` or `layer upper`), and an error marker if it
  failed. Rows can be dragged to reorder; ↑/↓ buttons and Alt+↑/Alt+↓ do the same.
  Buttons: New profile, New pocket, Duplicate, Delete.
- **Properties panel** (right, replacing the Selection panel): the selected operation's
  fields, or the job settings (stock, machine, tool) when the job row is selected.
- **Report** (right, below): as now.

## 4. Selecting geometry → operations
- Click / Ctrl+click / Esc on the canvas as now. New: **Select layer** (from a contour's
  context menu or a layer list) selects every closed contour on that layer.
- **New profile / New pocket** create an operation from the current selection:
  - a click selection becomes a `handles` selector (as "Copy as selector" today);
  - a layer selection becomes a `layer` selector;
  - open contours in the selection are left out and reported (as the pipeline does);
  - an empty selection is allowed: the operation is created with no geometry and can't run
    until geometry is assigned.
- **Use selection** in the properties panel replaces an existing operation's geometry.
- Selecting an operation highlights its contours on the canvas (and its toolpath, if any).

## 5. Defaults for a new operation
Small, slow values, so a new operation is a shallow test cut until you change it
(decision 2026-10-09). Every value is shown in the form and written to the file.

| Field | Default |
|---|---|
| name | "Profile 1", "Pocket 2", … (editable) |
| profile `side` | `inside` if every selected contour is a hole, `outside` if every one is an outer; **empty (must choose)** if mixed or nothing is selected |
| `direction` | `climb` |
| `tool` | the job's tool |
| `depth` | 1 mm (cut to Z −1 with Z zero at the stock top) |
| `step_down`, `feed`, `plunge_feed`, pocket `stepover` | copied from the nearest operation above it in the list; with none: 1 mm, 500, 150 mm/min (≈ feed/3), 0.4 |
| `keep_tool_down` | `false` |
| tabs | off |

An operation with an empty required field (side, geometry) shows in red, and the job can't be
generated or saved until it's filled.

## 6. Job settings
- **New job** (Ctrl+N) starts from the open drawing: `source.file` = the DXF's file name,
  `units` = the drawing's units (or the user's answer if it has none). Defaults, all editable:
  stock covering the drawing's extents (origin `bottom_left` if the drawing lies in the first
  quadrant, else `center`), 10 mm thick, `z_zero: top`; GRBL post, safe Z 5, spindle `cw`, no
  max feed or travel; one endmill, 6 mm, 18 000 rpm, centre cutting unknown.
- Stock, machine (post, safe Z, spindle, max feed, travel) and the tool are edited in the
  properties panel.
- Opening a job when no drawing is open also opens its `source.file`, looked up next to the
  job file.
- **Lock to the drawing** (decision 2026-10-09, Q2): when a job with handle selectors is saved
  without `source.sha256`, the drawing's sha256 is written, so a changed drawing is refused
  instead of cutting the wrong shapes. *Job → Re-lock to drawing* updates it after a
  deliberate drawing change; *Unlock* removes it.

## 7. Generating and saving
- Edits never generate by themselves (decision 2026-10-09, Q3): they clear the toolpath and the
  report says it is out of date. **F5** validates and generates; errors go to the report and
  mark the operation's row.
- **Save job** (Ctrl+S) / **Save job as…** (Ctrl+Shift+S) write the current schema
  (`save_job`): every field explicit, `written_by`, atomic write. Saving a job opened from an
  older version writes the current version (noted in the report). An incomplete job (empty
  side or geometry) can't be saved; the report says which field is missing.
- **Export G-code** (Ctrl+E; decision 2026-10-09, Q1) writes the checked program, as Save
  G-code did before. It needs a successful F5 since the last edit.
- Unsaved changes show as `*` in the title; closing the window, opening another job or
  starting a new job asks first. Opening another drawing keeps the job (nothing is lost) and
  notes it if the job was made for a different file; the sha256 lock catches a mismatch.
- **Undo / redo** (Ctrl+Z / Ctrl+Shift+Z) for every edit, as job snapshots.

## 8. Tests
- Operations created from click and layer selections produce the expected selectors.
- Side defaults: holes only → inside, outers only → outside, mixed → empty and not runnable.
- Reorder (drag, buttons, keys) changes the generated program's operation order; disabled
  operations are skipped by the GUI and by `run_job` alike.
- Saved job reloads to an equal `Job`; `chipload run` on it gives byte-identical G-code to
  the GUI's export.
- Undo/redo restores exact earlier jobs; the unsaved marker follows edits and saves.
- Migration: `enabled` absent → enabled; all frozen samples still load.

## Decisions (2026-10-09)
- Q1: Ctrl+S saves the job, Ctrl+E exports G-code.
- Q2: lock to the drawing (sha256) automatically when handle selectors are used, for now.
- Q3: generate only on F5.
- Q4: small defaults instead of empty fields: feed 500, plunge 150, depth 1, step-down 1,
  stepover 0.4.
