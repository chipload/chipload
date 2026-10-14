# Spec: start depth, end depth and clearance

Status: **approved 2026-10-10** (Q1–Q5 as recommended; the stock-depth limit changed, see §2).

Guide: docs/book/src/concepts/depths.md.

User request (2026-10-10):
- Each operation gets a **start depth** and an **end depth**. Today the cut always starts
  at the stock top and `depth` is the end depth.
- Each operation gets a **clearance**, default 0. Positive: the cut moves into the material
  that is kept, so a pocket comes out slightly larger and a profiled part slightly smaller.
  Negative: the other way.

## 1. Format: job schema v7

| v6 | v7 | Migration v6 → v7 |
|---|---|---|
| `depth` | `end_depth` (same meaning) | renamed, value kept |
| — | `start_depth`, required | `0` (today's behaviour) |
| — | `clearance`, required | `0` (today's behaviour) |
| — | `stock.max_overcut`, required | `0` (today's behaviour) |

For profiles and pockets. The rename and the new required fields need a version bump
(CLAUDE.md); the migration keeps machining exactly as before, so the frozen v4/v5 projects
must give their golden G-code unchanged. Required rather than optional-with-default: like
`direction`, they are values that decide what is cut, so they're always written.

All depths are mm, measured **down from the stock top**, whatever `z_zero` is (as `depth`
today).

## 2. Start and end depth

- Rule: `0 ≤ start_depth < end_depth`. Anything else is a job error.
- **Below the stock** (user decision 2026-10-10: a warning, not a hard stop, so through-cuts
  can go into the spoilboard). New stock setting `max_overcut` (mm, ≥ 0, "Max depth below
  stock" in the GUI): how far below the stock bottom a cut may go.
  - `end_depth ≤ thickness`: no message.
  - `thickness < end_depth ≤ thickness + max_overcut`: allowed, with a warning naming the
    operation and how far it cuts below the stock.
  - deeper: job error, so a typo can't reach the machine bed.
  - New projects get `max_overcut` 0.5 mm, written in the file and shown in Job settings;
    migrated projects get 0, so nothing changes for them until it's set.
  - The independent checker reads `max_overcut` itself: Z below the stock bottom is a
    warning down to `bottom − max_overcut` and an error below that.
- Passes split `end_depth − start_depth` into equal passes no deeper than `step_down`; the
  last pass is exactly at `end_depth`. Example: start 2, end 6, step-down 2 → passes at 4
  and 6 mm. (Today: start 0, so nothing changes.)
- The tool still goes down at the plunge feed from safe Z, also above the start depth. It
  doesn't rapid down to the start depth: whether the material above it is really gone
  depends on other operations and on the stock, which Chipload doesn't track. *(Question 2.)*
- Tabs: the tab height is still measured up from the bottom of the cut (`end_depth`) and
  must be less than the cut's own depth, `end_depth − start_depth`.
- The checker judges Z against the stock (top, bottom with `max_overcut`) and safe Z, not
  against the operation.

## 3. Clearance

Positive clearance `c` moves the cut `c` mm into the material that is kept:

| Operation | Positive `c` | Negative `c` |
|---|---|---|
| Pocket | pocket larger by `c` all round; islands smaller | pocket smaller, islands larger |
| Profile, outside | part smaller by `c` | part larger (stock to leave) |
| Profile, inside (a hole) | hole larger by `c` | hole smaller |
| Profile, on the line | not allowed (no side) → error if `c ≠ 0` | same |

How: the tool centre runs at `r − c` from the drawn geometry instead of `r`, where `r` is
the tool radius. That's how CAM programs do "stock to leave" (Chipload's `c` is minus that).
Consequences:
- Inside corners of the kept material (e.g. the corners of a rectangular pocket) keep the
  tool's radius `r`, as today; the corner just sits `c` further out. Outside corners of the
  kept material (e.g. a lid's corners, an island's corners) are rounded with radius `c`
  when `c > 0`, because the tool reaches `c` past the drawn corner. *(Question 3.)*
- Rule: `c < r`. At `c = r` the tool centre would run on the drawn line, past it for more.
  `c ≥ r` is a job error (use a profile "on" or a smaller tool instead).
- Negative `c` has no lower limit, but the existing errors still apply ("the tool does not
  fit", narrow parts).
- Verification checks the tool centre against `r − c` instead of `r` (no gouge beyond the
  clearance). Material left and the uncut-material check work on the cleared geometry.

## 4. GUI and CLI

- GUI, operation form: **Start depth**, **End depth**, **Clearance** (mm, can be negative),
  with the rules above shown as errors next to the field.
- The operations list shows the depth range, e.g. `0–6 mm`.
- New operations: start depth 0, clearance 0; end depth as today's depth default.
- CLI: the project file holds the values; `run`, `plan` and `check` use them. (There is no
  CLI for editing operations yet; the file or the GUI sets them.)

## 5. Tests

- Migration: v6 sample → v7 has `end_depth` = old depth, `start_depth` 0, `clearance` 0;
  all golden G-code unchanged. A frozen v6 sample is added (CLAUDE.md).
- Passes at the boundaries: start 0 / end = thickness; start = end − tiny; step-down larger
  than the range; start > end, start < 0 → errors.
- Below the stock: end = thickness + overcut → warning, G-code produced and checked (checker
  warning, not error); end beyond that → error; checker on its own: Z between bottom and
  bottom − overcut → warning, below → error.
- Tabs with a start depth: allowed height is `end − start`.
- Clearance: a square pocket with `c = +0.5` is 1 mm wider and taller than with `c = 0`
  (checked from the ring positions); `c = −0.5` the other way; islands move opposite.
  Outside profile of a square with `c = +0.5`: the toolpath is 0.5 mm closer to the part.
  `c ≥ r` and `c ≠ 0` with side "on" → errors.
- Property test: with clearance `c`, every cutting move keeps at least `r − c − tol` from
  every wall and island.
- New golden file: a pocket with start depth and clearance (commit message explains it).

## 6. Decisions (approved 2026-10-10)

1. **Names and format**: rename `depth` → `end_depth`, add `start_depth` and `clearance`,
   all required, schema v7. **Yes** (unreleased, so a rename is cheap now).
2. **Plunging above the start depth**: keep plunging at the plunge feed from safe Z (safe,
   a little slower). A faster "rapid down to just above the start depth" can come later as
   an explicit option. **Keep it safe for now.**
3. **Corners with clearance**: the standard CAM behaviour in §3 (tool centre at `r − c`;
   outside corners of the kept material get a small radius `c`). The alternative is to
   offset the drawn outline with sharp corners first, then cut that: more work and unusual
   in CAM. **The standard behaviour.**
4. **Limits**: `c < r` (tool radius), and clearance must be 0 for a profile on the line.
   **Yes.**
5. **The word**: "clearance" as you wrote it (positive = more removed). CAM programs often
   say "stock to leave" with the opposite sign. **Keep "clearance"**, since
   it says what you want for fits; the GUI hover text explains the sign.
6. **Below the stock** (added in review): `max_overcut` with a warning inside it and an
   error beyond, checked independently; new projects 0.5 mm, migrated ones 0. **Yes.**

## As built, revised 2026-10-15

The field is shown as **Oversize** in the app and the guide ("how much more than drawn is
cut"), since "clearance" reads as the clearance plane and the opposite of other CAM
programs' "stock to leave". The file keeps `clearance`, with the sign above.
