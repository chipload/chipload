# Spec: problems shown where they are, while you edit

Status: approved (2026-10-12; decisions 1–6 as recommended). Item 6 of the general improvement pass (TODO.md, approved
2026-10-12).

Guide: docs/book/src/checking/problems.md.

## 1. Why

A value that makes the project invalid (a feed of 0, a start depth below the end depth, a
clearance as large as the tool radius, a safe Z inside the stock) is only reported after
Generate, as a line in the report such as `operations.1.feed: must be greater than 0, got
0`. The user then has to find operation 2 and its feed field. With the live preview the
report appears by itself, but it still points at a path, not at the field.

## 2. What is checked, and when

- **The rules that need no geometry:** the value rules of the project file
  (`Job::validate`: positive feeds and depths, start above end, stock size, tools, copies),
  what an operation still lacks (no shapes selected, profile side not chosen), and the job
  rules checked before planning (`validate_job`: a tool with no rpm, safe Z inside the stock,
  depth beyond the allowed cut below the stock, clearance or helix radius not smaller than
  the tool radius, lead radius, feed above the machine's max feed, a helix on a profile, a
  lead on an "on the line" profile). Today `validate_job` stops at the first; it will
  collect every one.
- **Not geometry:** whether a pocket's tool fits, offsets, tabs that don't fit and the like
  are found by planning. They keep showing on the operation card after a generation (as
  now), which with the live preview is half a second after the edit.
- **When:** after each finished edit (the moment an undo step is recorded: after a drag or
  a typed value, not at every keystroke), and when a project is opened. The checks take
  well under a millisecond.

## 3. Where problems show

- **At the field:** the field gets a red frame; hovering it shows the problem in plain words
  ("Must be greater than 0", "Must be less than the tool radius (3 mm)"). A section with a
  problem in it shows a red dot in its header, so a folded section isn't silent.
- **On the operation card:** the card gets its red edge (as for failed generations today)
  and the first problem as one short line ("Feed: must be greater than 0").
- **Job settings row** in the sidebar: a red dot and the number of problems in the job's own
  settings (stock, machine, tools, copies).
- **A Problems strip** at the top of the properties panel when there are any: one line per
  problem; clicking one opens its operation or the Job settings.
- **Generate (F5)** with problems reports them in the report as now (with the plain-words
  text). The live preview doesn't generate while there are problems: the strip says why.
- The CLI's messages don't change.

## 4. How

- One kind of problem for every rule: `chipload_job::Problem { path, short, text }`. `path`
  is the field (`operations.1.feed`, `machine.safe_z`, `tools.0.rpm`, or an operation as a
  whole, `operations.1`); `short` is the message for the field ("must be greater than 0,
  got 0"); `text` is the full message exactly as the CLI prints it today, so its output and
  the tests of it stay the same.
- `Job::validate` and the new `chipload_toolpath::job_problems` return problems;
  `validate_job` keeps its behaviour by returning the first of them, in today's order.
- The GUI maps paths to its fields and section labels (a field label table in the forms);
  the plain-words text is the field's label plus `short`.

## 5. Decisions to review

1. **Check after each finished edit**, not at every keystroke. *Recommended:* the same
   moment the live preview and the recovery copy react to; cheap, and no red frame while a
   number is half typed.
2. **The no-geometry rules only**, all of them; geometry problems stay with generation.
   *Recommended.*
3. **Red frame and hover text at the field, a red dot on the section, a line on the card, a
   Problems strip.** *Recommended:* visible from anywhere, without moving the form around.
4. **Live preview pauses while there are problems.** *Recommended:* there is nothing it
   could make.
5. **One problem type with a field path**, CLI text unchanged. *Recommended:* the rules
   stay in one place (job and toolpath crates), the GUI only shows them (rule 6).
6. **Errors only in this step.** Warnings (a cut into the spoilboard, a tool not marked
   centre-cutting) stay in the report after generation; showing them amber at their fields
   can follow with the same machinery. *Recommended:* smaller step.

## 6. Tests

- Job and toolpath crates: every rule gives a problem with the right path; `validate_job`
  returns the same first error as before (existing CLI and pipeline tests unchanged);
  several problems at once are all returned.
- GUI: an edit that makes a feed 0 flags that field's path, the operation card and the
  strip after the edit is finished, not before; fixing it clears them; a job-level problem
  (safe Z inside the stock) flags the Job row; the live preview doesn't start while there
  are problems; clicking a strip line focuses the operation.

## 7. As built (2026-10-12)

- `chipload_job::Problem` (`path`, `short`, `text`; `operation()`, `field()`);
  `Job::validate` (the value rules, each built at its field with `Problem::at`);
  `chipload_toolpath::job_problems` collects every pre-planning rule, and `validate_job`
  returns the first (the same error as before: no CLI, pipeline or golden change);
  `chipload_pipeline::job_problems` gives both.
- GUI `app/problems.rs`: `EditJob::problems` adds what an operation lacks (`side`,
  `select`; the project's own empty-selection rule is then left out, it says the same).
  Problems refresh after each finished edit, undo, redo and when a project opens. The
  form's problems are kept in egui's temporary data while it draws, so its rows mark
  themselves (`flag`) without passing them along; section headers get a red dot from a
  table of the fields each section holds; tool sections from `tools.<i>`.
- Plain words: at the field the short message capitalised; elsewhere the field's label
  first ("Feed: …", "Safe Z: …") unless the message already starts with it.
- The Job row shows a red "● N" beside it rather than turning red (readable when
  selected).
