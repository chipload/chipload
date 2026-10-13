# Spec: scaling the imported drawing

Status: **approved 2026-10-11**, with all decisions as recommended (§8). Built as described,
with the differences in §9.

Guide: docs/book/src/drawings/placement.md.

User request (2026-10-11): scale the imported drawing, as well as move and turn it. This
follows docs/specs/drawing-placement.md (moving) and docs/specs/drawing-rotation.md (turning).

## 1. What exists

- `drawing.source.placement` is `{ quarter_turns, dx, dy }`. A DXF point p is placed at
  `turn(p) + (dx, dy)`. The stored shapes are the placed ones. An update from the DXF places
  the new shapes the same way, and they count as unchanged within `PLACEMENT_ROUNDING_MM`.
- Splines and ellipses are approximated **at import**, within `ARC_APPROX_TOL_MM` (0.01 mm) of
  the curve at the size the DXF gives in mm. The import already divides that tolerance by
  how far block inserts stretch the curve (`max_stretch`). The project stores only lines and
  arcs, not the DXF.
- Shapes are chained into contours from the stored (placed) shapes, joining ends within
  `CHAIN_TOL_MM` (0.001 mm).

## 2. The placement gets a scale

```json
"placement": { "scale": 2.0, "quarter_turns": 1, "dx": 120.0, "dy": -35.5 }
```

- A DXF point p is placed at `scale · turn(p) + (dx, dy)`. With scale 1 this is exactly
  today's placement (multiplying by 1.0 is exact).
- The scale is **uniform** (decision 1): one factor for X and Y. Circles stay circles and arcs
  stay arcs: the centre is scaled, the radius is multiplied, and the angles stay the same. A
  different factor for X and Y would turn arcs into ellipses (approximated, so not exact) and
  is not part of this.
- **Positive only.** A negative factor would be a mirror, and a mirror flips outer/hole
  winding and climb/conventional. Mirroring is not part of this.
- **Units are not scale.** A DXF in inches that says millimetres is fixed by setting its units
  on update (`--units in`), which also fixes the recorded units. A scale of 25.4 or 1/25.4
  gets a note saying so (§3).
- Unlike turning, scaling is **not exact**: multiplying rounds in the last bit. Updates still
  compare within `PLACEMENT_ROUNDING_MM` (1e-9 mm), which is a relative error of about 1e-15
  on a 1 m drawing. That's plenty for a factor well inside the allowed range (decision 5).

## 3. Scaling

`place_drawing` gets one more move, **scale by a factor**, about **the centre of the imported
shapes' bounding box** (decision 2), the same point turning uses. So the drawing grows or
shrinks in place. Under the hood this sets a new scale and a new (dx, dy). Steps add up
the same way turns do: the new placement is the step applied to the old one, with
`scale = step.scale · old.scale`.

There are two ways to ask for a size (decision 3):

- **By a factor**: `×2`, `×0.5`.
- **To a size**: the bounding box's width *or* height in mm, after any turn, with the other
  following from the uniform scale. "Make it 100 mm wide" is the common case. The factor is
  `wanted / current`.

The report says, for example, "Scaled the imported drawing ×2 (now 120.000 × 45.000 mm); it is
now scaled ×2, turned 90°, moved X… Y… from the DXF."

What scales and what doesn't:

| Scales with it | Stays |
|---|---|
| imported shapes (positions and radii) | drawn shapes, work zero at a point |
| start points on imported shapes (same rule as moving: nearest shape the operation selects) | tool sizes, depths, step-overs, feeds |
| | tab width, count and height, lead lengths, clearance |
| | the stock; the copies' pitch (overlapping copies are refused when run, as now) |

**Notes in the report:**

- *Curves approximated at the old size* (decision 4). When the drawing has shapes that came
  from splines or ellipses (their recorded `dxf_type`) and the total scale grows past
  where they were approximated, the error grows with it: ×10 makes 0.01 mm into 0.1 mm. The
  report says "N shapes from splines/ellipses were approximated at the DXF's size; scaled ×10
  they may be up to 0.100 mm off the curve. Update from the DXF to approximate them again at
  this size." **Update** approximates within `ARC_APPROX_TOL_MM / scale` (the same way it
  already handles stretched block inserts), so after an update the placed curve is within
  0.01 mm again.
- *Contours that change.* Chaining stays on the placed shapes, so its tolerance is in real mm
  on the machine (decision 6). Scaling up can open a gap that was joined (0.0008 mm ×10 =
  0.008 mm), and scaling down can join one that was open. When the number of closed contours
  changes, the report says so ("3 closed contours before, 2 now: check the import report"),
  and operations that select a contour that no longer exists show their usual message.
- *Units.* For a factor of 25.4 or 1/25.4 (within 1e-9): "If the DXF's units are wrong, set
  them on update instead (`--units`)."
- The "extends beyond the stock" note, drawn shapes staying, and the work zero, as for moving.

## 4. CLI

```
chipload place part.chipload --scale 2                 # ×2 about the drawing's centre
chipload place part.chipload --width 100               # 100 mm wide, height follows
chipload place part.chipload --height 40 --align bottom-left --margin 10
chipload place part.chipload --scale 0.5 --rotate 90   # turn, then scale, then align
```

- `--scale F` multiplies the current scale, the same way `--rotate` adds to the current turn.
  F must be a number > 0, and the resulting total scale must be within the range (decision 5).
- `--width W` / `--height H` (mm, > 0) scale the bounding box to that size. Only one of
  `--scale`, `--width` and `--height` can be given.
- They combine with `--rotate` and `--align` (turn, then scale, then align), but not with
  `--by` / `--to`, for the same reason as turning.
- `place` with no option prints the placement, now with the scale ("scaled ×2, turned 90°,
  moved X… Y… from the DXF") and the drawing's size.
- `--reset` also puts the scale back to 1 (exactly 1.0, not the product of the steps).

## 5. GUI

The *Drawing position* section gets a **Size** row: the imported drawing's **width** and
**height** in mm, and the **scale** as a factor. Editing any one of them scales uniformly
(the others update). Each edit is one undo step. Pressing Enter or leaving the field applies
it; a value that isn't > 0, or would take the total scale out of range, is refused with the
usual red field. The summary and the *Moved from the DXF* row include the scale. *Reset*
resets the scale too. The report notes from §3 show in the report panel.

## 6. Format

`placement.scale`, a number, required, > 0 and within the range. Recommended as **schema 11**
with a migration v10 → v11 adding `scale: 1.0` (decision 7), for the same reason as the turn:
an older Chipload then says "newer version" instead of "unknown field". The v10 sample
(`tests/corpus/jobs/v10/vcarve_letters.chipload`) is frozen and must keep its golden
G-code. A new v11 sample with a scaled, turned and moved drawing gets its own golden file.

## 7. Tests

- Job: the v10 → v11 migration adds scale 1; scale ≤ 0, NaN, infinite or out of range is
  refused.
- Pipeline:
  - Scaling a square with an arc and a hole by 2 about the bbox centre keeps the centre. It
    doubles the width, the height and every radius, keeps the arc angles, and keeps outer
    and hole.
  - ×2 then ×0.5 is back within `PLACEMENT_ROUNDING_MM`, and `Reset` gives scale exactly 1.
  - `--width` gives that width within `PLACEMENT_ROUNDING_MM`.
  - A profile and a pocket on the drawing scaled ×2 give the same program as on the same
    part drawn twice as large in its DXF (made in the test). The program itself is not a
    scaled copy of the ×1 program, because the tool doesn't scale.
  - Start points scale with the drawing; drawn shapes and a work zero point don't.
  - Update after scale + turn + move keeps the placement and reports every shape unchanged.
  - Update re-approximates a spline at the scaled tolerance: every point of the placed
    polyline lies within `ARC_APPROX_TOL_MM` of the scaled curve.
  - The spline note appears when scaling up a drawing with a spline, and not for one without.
  - A gap that opens at ×10 gives the contour-count note.
  - Scale composed with turns: turning then scaling equals scaling then turning, both about
    the bbox centre.
- Property test: for random factors in range and random turns, placing the DXF once with
  the final placement equals the step-by-step result within `PLACEMENT_ROUNDING_MM` ×
  the drawing's size.
- CLI: `--scale`, `--width`, `--height`; `--scale 0` and `--scale -1` refused; two of them
  together refused; with `--rotate` and `--align`; with `--by` refused; the scaled project
  runs through the checker.
- GUI: the size row edits width, height and factor; undo.
- Golden: the new v11 sample; every existing golden file unchanged.

## 8. Decisions

1. **Uniform scale only.** Recommended: **yes**, one factor, positive. Arcs stay exact arcs,
   and sides and directions keep their meaning. Separate X/Y factors and mirroring would be
   separate changes, if ever wanted.
2. **Scale about what.** Recommended: **the centre of the imported drawing's bounding box**,
   the same as turning, so it stays where it is and *Bottom left* / *Centre* can be pressed
   after. Alternative: the bottom-left corner of the bounding box, which keeps a drawing
   aligned bottom-left with a margin where it is, but moves a centred one.
3. **How to ask.** Recommended: **both a factor and a target width or height** (CLI
   `--scale`, `--width`, `--height`; GUI width, height and factor fields). A factor alone
   makes the user divide.
4. **Splines and ellipses.** Recommended: **a note when scaling up past where they were
   approximated, and updates approximate at the scaled tolerance**. The alternative,
   refusing to scale a drawing with curves until it is re-imported, is safer but annoying.
   Not doing anything would let a ×10 drawing be 0.1 mm off its curves without saying so.
5. **Range.** Recommended: the total scale must be between **0.001 and 1000**. That catches
   typos (a stray zero) and keeps rounding far below `PLACEMENT_ROUNDING_MM`, while 25.4 and
   1/25.4 still fit.
6. **Chaining tolerance after scaling.** Recommended: **chain the placed shapes, as now**, so
   0.001 mm means 0.001 mm on the machine, with the contour-count note when scaling changes
   what joins. Alternative: chain at the DXF's size and scale the contours after. That keeps
   the same contours at any scale, but then a gap the tool sees as 0.01 mm would count as
   joined.
7. **Schema 11 with a migration** adding `scale: 1.0`. Recommended: **bump to 11** (same
   reasoning as schema 9).

## 9. As built

- **The spline note** is given for each step that scales up (factor > 1), with that step's
  factor: the project doesn't record the size its curves were approximated at, so the note
  can't know how far a series of steps has grown them. Every such note says to update from
  the DXF.
- **Updates** read the DXF at the placement's scale (`chipload_dxf::shapes_from_dxf_scaled`),
  then turn and move. So every tolerance of the import holds at the placed size: splines,
  ellipses, and arcs under block inserts that distort them.
- **GUI:** the *Size* rows follow the *move by* row: inputs that start at the drawing's
  current width, height and ×1, with **Set width**, **Set height** and **Scale** buttons, so
  each press is one undo step. The inputs can't go to 0 or below; a factor that takes the
  total out of range is refused in the report.
- `place` with no option also prints `Size: W × H mm`.
- Tests: crates/chipload-pipeline/tests/placement.rs (including a property test that
  placing step by step equals placing the DXF once), crates/chipload-job/tests/jobs.rs,
  crates/chipload-cli/tests/cli.rs, the GUI's placement test, and the golden sample
  `tests/corpus/jobs/v11/scaled_part.chipload` (×0.9, turned 90°, in the corner).
