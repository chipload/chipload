# Spec: choosing the G-code zero point (work zero)

Status: **approved 2026-10-10** (Q1–Q4 in §7: all as recommended).

Guide: docs/book/src/concepts/stock.md.

User request (2026-10-10): pick the zero point of the exported G-code independently of how
the geometry is drawn: the stock centre, a stock corner or any point, without moving the
geometry. A translation at export only.

## 1. Today

- The G-code zero **is the drawing's origin**, always.
- `stock.origin` (`bottom_left` / `center`) only says where the stock lies around that
  point: its bottom-left corner or its centre at the drawing origin.
- `stock.z_zero` (`top` / `bottom`) already chooses Z zero independently.

So to have zero at the stock centre while the drawing's origin is at a corner, you'd have
to move the geometry. This spec separates the two.

## 2. Model

New optional field `machine.work_zero`, the drawing point that becomes X0 Y0 in the G-code:

```json
"work_zero": {"at": "drawing_origin"}                      // default: today's behaviour
"work_zero": {"at": "stock", "corner": "center"}           // center | bottom_left |
                                                            // bottom_right | top_left | top_right
"work_zero": {"at": "point", "x": 25.0, "y": -10.0}         // any point, drawing coordinates
```

- Missing → `drawing_origin`, exactly today's output. So it's an optional field with a safe
  default and **no schema bump** (CLAUDE.md). Saving always writes it explicitly.
- Stock corners and the centre come from the stock as placed by `stock.origin` (unchanged
  meaning: where the stock lies relative to the drawing). The UI calls that setting
  **Stock position** to make the difference clear; the file field keeps its name.
- `z_zero` is unchanged.

## 3. Where the translation happens

- **Only in the output.** Geometry, contours, toolpath planning, the canvas and
  `plan --json` stay in drawing coordinates. `run_job` translates a copy of the program by
  −work_zero just before the post-processor, then checks that.
- Translation is a subtraction of one fixed vector from every X and Y: no rotation, no
  scaling, deterministic. Arcs (later) translate their centres the same way.
- **The checker works it out independently**: `limits_from_job` reads `work_zero` and
  shifts the stock bounds into program coordinates itself, without toolpath code
  (architecture rule 3). So `chipload check out.nc --job p.chipload` judges the program
  against the stock where it really is. Travel limits apply to program coordinates, as
  today.
- If the translated program fails the checker (e.g. the travel limits no longer fit), no
  G-code is produced, as for any other failure.

## 4. The G-code says where zero is

Every program gets a header comment, e.g.:

```
(Work zero: stock center = drawing X150.000 Y110.000, Z0 = stock top)
```

A machinist reading the file can then set up the machine without opening the project. This
adds a line to **every** program, so all golden files change once, by that line only
(explained in the commit message, as CLAUDE.md requires). The post template gets the text
as a variable, so a post can place or format it.

## 5. CLI and GUI

- CLI: `chipload run p.chipload -o out.nc` uses the project's work zero. No override flag:
  the project is the one source of truth, so the same project always gives the same
  program (determinism) and `check --job` agrees with it.
- `inspect` (project) and `run` print the work zero line.
- GUI, Job settings → Machine: **Work zero** with the choices of §2 and X/Y fields for a
  point. The canvas draws the X/Y axes at the work zero (today: at the drawing origin), and
  the status bar shows the cursor in work coordinates too.

## 6. Tests

- Default (field missing) → byte-identical to today, apart from the new header line.
- Each choice: the program equals the untranslated one shifted by the expected vector, e.g.
  stock 300 × 220 with its bottom-left corner at the drawing origin, zero at the centre →
  every X −150, Y −110.
- Checker: the same translated program passes with the project's work zero, and fails
  (outside stock) when checked against the project with zero at the drawing origin. Proves
  the checker applies the translation itself.
- Validation: point coordinates finite; unknown `at` or corner refused.
- A frozen sample keeps loading (the field is optional); a v5 project with each choice round
  trips.

## 7. Decisions (approved 2026-10-10)

1. **Saved in the project** (`machine.work_zero`), like Z zero, and not only an export-time
   choice? The G-code must be reproducible from the project, and `check --job` needs it to
   judge the program. **Saved in the project.** Changing it changes no
   geometry.
2. **Choices**: drawing origin (default), stock centre, the four stock corners, any point.
   **Yes.** (Picking the point by clicking on the canvas could come later.)
3. **Header comment** in every program stating the work zero, changing every golden file by
   one line. **Yes**, for the machinist.
4. **Rename** the existing "Origin" in the stock settings to **Stock position** in the UI
   (file unchanged). **Yes**, so "origin" doesn't mean two things.
