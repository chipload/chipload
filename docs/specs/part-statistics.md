# Spec: part statistics

Status: approved (2026-11-06; decisions 1–5 as recommended, the user: "go ahead"); built.

Builds on docs/specs/operation-statistics.md (the operation's Statistics section) and
docs/specs/parts.md. Guide: docs/book/src/checking/time.md and the parts page.

## 1. Why

The user (2026-11-06): "the statistics that we have added for each operation last in the
properties panel, i think we could do the same for the part level". A part is what gets
quoted and made: its time, how far it cuts and how often it lifts, without adding up its
operations by hand.

## 2. What is counted

A part's numbers are the **sum of its operations'** and of their rest and floor passes'
(`Estimate::operation_counts`), as the Report's part row already sums their times
(`estimate_rows`). So:

| Number | The part's |
|---|---|
| **Time** | its operations' times, with cutting and rapids |
| **Cutting, plunging, rapids** | its operations' lengths |
| **Lifts** | its operations' lifts |
| **Operations** | how many of its operations (and passes) are cut |

Summed over the part's copies, as an operation's numbers are; "For 4 copies." when it has
some. Not counted, as for an operation: the moves from one operation to the next, tool
changes and spindle waits (the job's, in the Report's total).

## 3. Where it shows

### 3.1 The part's properties

A last section, **Statistics**, in the part's properties (under Copies), folded to the
time, the same rows as an operation's and in the same words, plus *Operations*:

```
Statistics                                12 min 40 s
  Time             12 min 40 s  (cutting 11 min 2 s, rapids 1 min 38 s)
  Cutting          18.6 m
  Plunging         120 mm
  Rapids           4.4 m
  Lifts            41
  Operations       3
  For 2 copies.
```

The same states as an operation's: out of date ("Generate (F5) to see the statistics."),
generating, failed, off (the part or its setup), and the generate hint for a part the last
run didn't plan (another setup, or another part's preview). A part without operations says
"No operations." instead.

### 3.2 The command line

- `plan --json` and `run --json`: `estimate.parts`, one entry per part that is cut: its
  `id`, `name`, `seconds` and the same numbers as an operation (`cutting_s`, `rapid_s`,
  `cutting_mm`, `plunge_mm`, `rapid_mm`, `lifts`).
- `chipload plan`'s printed part line (shown with two parts or more) gets its lengths as
  the operation lines have them: `Part 1 · box: 12 min 40 s; cutting 18.6 m, plunging
  120 mm, rapids 4.4 m, 41 lifts`. The frozen English messages change for that line only.
- The Report and the setup sheet: unchanged.

## 4. Where it is computed

In `chipload_pipeline::estimate`: `Counts` can be added up, and `part_counts(job,
estimate, part)` sums a part's operations, used by the GUI, `plan` and the JSON alike. The
GUI only shows it. No change to the G-code or the project file.

## 5. Decisions to review

1. **A Statistics section last in the part's properties**, folded to its time, with the
   operation's rows plus *Operations*. *Recommended:* the user's idea, and the same section
   reads the same at both levels.
2. **The sum of its operations** (§2), not the part's whole span of moves.
   *Recommended:* it matches the Report's part time, and an operation's numbers then add up
   to its part's. The other way would count the short moves between its operations too,
   but needs each part's span from the planner, and the part's time would then disagree
   with the Report's.
3. **Tool changes stay out** (the job's, in the Report). *Recommended:* parts are cut one
   after the other, so a tool change can sit between two parts and belongs to neither.
4. **The command line first:** `plan --json` gets `estimate.parts`, `plan`'s part line its
   lengths. *Recommended:* rule 5 (every feature from the CLI first); the Report and the
   setup sheet stay as they are.
5. **Not now:** the same for a setup and for the whole job (the project's properties).
   *Recommended:* later, if wanted; with the active-part preview, the last run often
   planned one part only, so a job total there would need a run of the whole job.

## 6. Tests

- Pipeline: a part's counts are the sum of its operations' and passes'; a part of two
  copies doubles; an off operation isn't in it; `part_counts` of a part that wasn't planned
  is None.
- `plan --json` carries `estimate.parts`; `plan` prints the part line with lengths (the
  frozen messages).
- GUI: the part's section shows the numbers after a run, its time as the summary, the
  hint when out of date, "Off: not cut." for a part that is off, "No operations." for an
  empty part.
- The guide says where a part's statistics are.

## 7. As built (2026-11-06)

- `chipload_pipeline::estimate::part_counts(job, estimate, part) -> Option<PartCounts>`
  (`counts`, `operations`): None when the run planned none of the part's operations.
- `estimate_lines` with `Lengths::PerOperation` (`chipload plan`): the part line is
  `pipeline-estimate-part-counted`; the frozen English `plan` messages changed in those
  lines only, no time changed.
- `plan --json` and `run --json`: `estimate.parts` (`estimate_json` takes the job).
- GUI: `ChiploadApp::part_statistics(id)`, drawn under the part's form with
  `statistics_section(ui, "part-statistics", …)`; `Statistics::Counted` has `operations`
  (Some for a part), and `Statistics::Empty` says "No operations.". The *Operations* row
  reuses `forms-operations`.
