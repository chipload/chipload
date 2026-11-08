# Spec: operation statistics

Status: approved (2026-11-03; decisions 1–6 as recommended, the user: "go ahead"); built.

Builds on docs/specs/cycle-time.md (the time estimate). Guide: docs/book/src/checking/time.md.

## 1. Why

The user (2026-11-02): "id like a report for how long time each operation will take,
similar to how prusa slicer gives a report how much time different operations take. Except
time id like, g1 travelled length, g0 length, etc." And on where (2026-11-03): "each
operation properties could have a section added last called Statistics or something
similar. There the time, length, number of lifts could be summarized for that operation. It
will be invalidated if a parameters changes and instead show something like 'Press F9 to
generate the operations and see statistics'".

The numbers help compare strategies (a smaller stepover, other linking, a second tool for
the rest), find an operation that spends its time in the air, and quote a job. Today only
the time per operation is shown (Browser cards, the Report, `plan`).

## 2. What is counted

From the planned toolpath, as the time estimate counts it: the moves before posting, arcs
still as their chords (whose lengths add up to the arc's within the arc tolerance), lengths
in 3D from where the last move ended. Per operation and per rest or floor pass (an `OpRef`),
summed over its copies; the moves between operations count only in the job's totals.

| Number | Counts |
|---|---|
| **Time** | as today (cycle-time.md §2): cutting at the feed, rapids at the rapid rate |
| **Cutting length** | feed moves that move in X or Y, in 3D: lines, arcs, ramps, helices, tab climbs. The G1/G2/G3 of the program, except plunges |
| **Plunge length** | feed moves along Z alone, down or up: plunges, drilling, a straight feed out of a hole |
| **Rapid length** | rapids (G0), in 3D, up and down included |
| **Rapid time** | their part of the time |
| **Lifts** | rapids that move up: each time the tool leaves the work and goes to the safe height (or to the approach height above the stock). A peck drill's pecks count, one per peck: they are lifts on the machine too |

Not counted: tool changes (counted once for the job in the Report, as today), spindle
waits (the job's, not an operation's), the material removed (the simulation's).

Lengths show in the display units: mm under 1 m (`850 mm`), else m with one decimal
(`3.1 m`, `12.4 m`); in inches, in under 100 in (`33.5 in`), else ft with one decimal
(`41.2 ft`). Lifts as a whole number.

## 3. Where it shows

### 3.1 The operation's properties

A last section, **Statistics**, in every operation's properties and in each rest and floor
pass's. Folded, its summary is the time (`8 min 10 s`), as the other sections give theirs.
Open:

```
Statistics                                8 min 10 s
  Time             8 min 10 s  (cutting 7 min 2 s, rapids 1 min 8 s)
  Cutting          12.4 m
  Plunging         86 mm
  Rapids           3.1 m
  Lifts            23
  For 4 copies.
```

The last line only when the part has copies. Read-only: no fields, nothing to edit.

When there are no numbers, the section says why instead, in its body and its summary:

- **Out of date** (a value changed since the last run, the toolpath is faded): "Generate
  (F5) to see the statistics." With auto-generate on, the run starts by itself, and the
  section says "Generating…" until it ends.
- **Not generated yet**: the same "Generate (F5)…".
- **The run failed**: "Not generated: see the Report."
- **The operation is off** (or its part is): "Off: not cut."
- **Cut nothing** (no shapes, or nothing fits the tool): the numbers are zero; the Report
  already says why.

One flag for the whole toolpath, as today: any change makes every operation's statistics
out of date until the next run (the toolpath is planned as a whole: linking, order and the
moves between operations depend on all of them).

### 3.2 The Report

After the time lines it gives today, one line for the job's totals:
`Cutting 48.2 m, plunging 0.4 m, rapids 12.7 m, 112 lifts.` No line per operation for the
lengths: the Report stays short, and the properties hold the detail.

### 3.3 The command line

- `chipload plan`: after each operation's time, its numbers, one line each:
  `Operation 1 (Pocket 1): 8 min 10 s; cutting 12.4 m, plunging 86 mm, rapids 3.1 m,
  23 lifts`, and the totals line as in the Report.
- `plan --json` and `run --json`: in `estimate`, each operation gets `cutting_mm`,
  `plunge_mm`, `rapid_mm`, `rapid_s`, `lifts` beside its seconds, and `estimate` gets the
  same for the whole job. Lengths in mm, as all of the JSON.
- `chipload run`'s printed lines: unchanged except the totals line.

### 3.4 Not now

- The setup sheet: it gives each operation's time already; the lengths can follow if the
  operator asks.
- A chart (PrusaSlicer's bars of time per feature): the numbers first; a chart can come
  when there is a place for it.
- Per tool and per file: the Report gives each file's time already; the lengths per tool
  are the sum of its operations'.

## 4. Where it is computed

In `chipload_pipeline::estimate`, beside the time and from the same loop over the moves
(`move_times`), so time and lengths can't disagree: `Estimate` gets a `Counts` (cutting,
plunge and rapid length, rapid seconds, lifts) for the job and one per `OpRef`. The GUI and
the CLI only show them (architecture rule 6). Nothing changes in the G-code or the project
file: no schema change.

## 5. Decisions to review

1. **A Statistics section last in each operation's (and pass's) properties**, folded to its
   time. *Recommended:* the user's idea; one place per operation, next to the values that
   change it.
2. **What counts as cutting, plunging and a lift** (§2): cutting is every feed that moves in
   XY (ramps and helices included, in 3D), plunging the feeds along Z alone, a lift every
   rapid up (peck retracts included). *Recommended:* simple to state and to check, and each
   matches what the machine does. The alternative, ramps as plunging, would hide where a
   ramp's time goes into "cutting" anyway.
3. **One out-of-date flag for the whole toolpath**, the hint "Generate (F5) to see the
   statistics" (the app's key is F5, not F9). *Recommended:* the toolpath is planned as a
   whole; a per-operation flag would show numbers that the next run may change.
4. **The Report gets one totals line; `plan` one line per operation; the JSON every
   number.** *Recommended:* the Report stays short; scripts get everything.
5. **Not on the setup sheet, no chart, no per-tool lengths for now** (§3.4). *Recommended.*
6. **Computed in the pipeline's estimate, from the same moves as the time** (§4).
   *Recommended.*

## 6. Tests

- A 100 × 50 mm rectangle profile, two passes by spiral: the cutting length is the spiral's
  3D length; plunging 0 (it ramps in); one lift at the end; rapids from the start to above
  the start and back up.
- A pocket that plunges: its plunge length is the depth per plunge times the plunges.
- A peck-drilled hole of three pecks: three lifts, the plunge length the drilled depth plus
  the pecks' feeds back down.
- Copies double an operation's numbers; a rest pass has its own; the job's totals include
  the moves between operations, so they are at least the sum of the operations'.
- Time and lengths come from the same moves: the cutting time is the cutting and plunge
  lengths each at its feed.
- Units: `850 mm`, `12.4 m`, inches and feet.
- `plan --json` carries the numbers; `plan` prints the lines (the frozen English messages).
- GUI: the section shows the numbers after a run, the hint when out of date, "Generating…"
  with auto-generate, "Off: not cut." for an operation that is off; its summary is the time.
- The guide's page on time (`checking/time.md`) and the operation pages say where the
  statistics are.

## 7. As built (2026-11-03)

Changed 2026-11-06 (the user: "add the time for each category after the length"): the
Time row gives the total alone, and Cutting, Plunging and Rapids each give their length
and their time ("1.4 m, 5 min 5 s"). `Counts::plunge_s` holds the plunges' part of
`cutting_s`; the JSON has it as `plunge_s`.


- `chipload_pipeline::estimate`: `Counts` (`cutting_s`, `rapid_s`, `cutting_mm`,
  `plunge_mm`, `rapid_mm`, `lifts`) from one pass over the moves (`move_counts`, which the
  time also comes from); `Estimate::counts` for the job, `Estimate::operation_counts` per
  `OpRef`. A move is "in XY" when it moves more than `CHAIN_TOL_MM` there; a lift is a rapid
  that rises more than that.
- `estimate_lines(job, estimate, several_files, Lengths)`: `Without` for the setup sheet,
  `Totals` (the "In all" line) for the Report and `chipload run`, `PerOperation` for
  `chipload plan`. `counts_message` gives "cutting …, plunging …, rapids …, N lifts".
- `chipload_i18n::Value::Distance` and `message::distance`: mm under 1 m, else m with one
  decimal; in, else ft from 100 in.
- `plan --json` and `run --json`: each `estimate.operations` entry and `estimate` itself
  carry `cutting_mm`, `plunge_mm`, `rapid_mm`, `lifts`; the entries also `cutting_s` and
  `rapid_s`.
- GUI: `forms::statistics_section`, drawn under the operation's, rest pass's or floor
  pass's form; `ChiploadApp::statistics_of(OpRef)` picks what it shows; `Toolpath::failed`
  marks a failed last run. An operation planned in no run (another part's preview, another
  setup) shows the generate hint.
- The frozen English `plan` messages gained the lines (`tests/corpus/expected/messages`);
  no time changed, and no G-code or setup sheet.
