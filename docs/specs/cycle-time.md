# Spec: cycle time estimate

Status: draft for review. Item 7 of the general improvement pass (TODO.md, approved
2026-10-12).

## 1. Why

Before cutting, the user wants to know how long the job takes: to plan the day, to choose
between strategies or stepovers, to quote a job. Chipload knows every move and its feed, but
says nothing about time.

## 2. What is estimated

From the planned toolpath (the moves before posting), per operation and in total:

- **Cutting moves:** length (in 3D, so ramps and helices count their descent) divided by
  their feed. Arcs are planned as chords, whose lengths add up to the arc's within the arc
  tolerance.
- **Rapids:** length divided by the rapid rate (§3).
- **Spindle waits:** the post's spin-up dwell (`spindle_dwell_s`) at every spindle start:
  once per file, and after each tool change in one file with M6.
- **Tool changes:** counted ("plus 1 tool change"), not timed: how long one takes depends
  on the person and the machine.
- **Copies:** each copy's moves count; an operation's time is the sum over its copies.

The estimate is at the programmed feeds. Acceleration and slowing down for corners make a
real machine slower, the more so for short moves and sharp corners; the report says so
rather than guessing a factor.

## 3. Rapid rate

Projects have no rapid rate. The estimate uses the machine's **max feed** when it is set
(a machine's rapids are at least as fast as its fastest cut), else **5000 mm/min**, and says
which ("rapids at 5000 mm/min, assumed: set the machine's max feed for a better estimate").
A separate rapid-rate setting can come later; it would need a schema change for projects and
machine profiles.

## 4. Where it shows

- **Report** after generating: `Estimated time: 12 min 30 s (cutting 10 min 5 s, rapids
  1 min 55 s, spindle 30 s), plus 1 tool change`, then one line per operation
  (`Operation 1 (Pocket 1): 8 min 10 s`), and, when the job is written one file per tool,
  the time of each file.
- **Operation cards:** the operation's time at the end of the detail line (`· 8 min`) once
  generated; gone (with the faded toolpath) when out of date.
- **CLI:** `chipload run` and `chipload plan` print the same lines; `plan --json` gets an
  `estimate` object (seconds: total, cutting, rapid, spindle; per operation; tool changes;
  the rapid rate used and whether it was assumed).
- Not in the G-code: the golden files and every program stay as they are. The setup sheet
  (item 8) will show it.

## 5. Format

Under a minute: seconds (`45 s`); under an hour: `12 min 30 s`; else `1 h 05 min`.
Rounded to the second; the parts are rounded separately, so they may not add up to the
total by a second.

## 6. Decisions to review

1. **Programmed feeds, no acceleration model**, with the report saying real time is longer.
   *Recommended:* honest and simple; a factor would be a guess.
2. **Rapids at the machine's max feed, else 5000 mm/min (said so)**; no new setting now.
   *Recommended:* no schema change; a rapid-rate field can follow if estimates matter more.
3. **Spindle waits counted, tool changes counted but not timed.** *Recommended.*
4. **Report, operation cards and CLI (`run`, `plan`, `plan --json`); not in the G-code.**
   *Recommended:* nothing about the programs changes.
5. **The estimate lives in the pipeline** (`chipload_pipeline::estimate`), computed from
   the plan's moves, operation spans and tool groups. *Recommended:* the CLI and GUI show the
   same numbers (rule 6).

## 7. Tests

- A 100 × 50 mm rectangle profile, one pass, known feed and plunge: the exact time
  (perimeter / feed + depth / plunge + rapids / rate + dwell).
- Helix and ramp moves count their descent; copies double an operation's time; a job in two
  files counts two spindle waits and one tool change; one file with M6 the same.
- The rapid rate: max feed when set, else 5000 and marked assumed.
- Formatting of 45 s, 12 min 30 s, 1 h 05 min.
- The GUI report and cards show it after generating; `plan --json` carries it.
