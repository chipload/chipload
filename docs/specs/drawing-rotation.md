# Spec: turning the imported drawing in 90° steps

Status: **approved 2026-10-11**, with all decisions as recommended (§7).

Guide: docs/book/src/drawings/placement.md.

User request (2026-10-11): rotate the imported drawing in 90° steps, a follow-up to
docs/specs/drawing-placement.md (moving it).

## 1. What exists

- The imported drawing has a placement (`drawing.source.placement`, `dx`/`dy` mm). The stored
  shapes are the placed ones; an update from the DXF places the new shapes the same way.
- Start points of operations move with the drawing when they belong to imported shapes.
  Drawn shapes and a work zero at a point never move.

## 2. The placement gets a turn

```json
"placement": { "quarter_turns": 1, "dx": 120.0, "dy": -35.5 }
```

- A DXF point p is placed at `turn(p, quarter_turns) + (dx, dy)`: first turned about the DXF
  origin by `quarter_turns` × 90° counter-clockwise (0 to 3), then moved. With 0 turns this is
  exactly today's placement.
- Turning by 90° is **exact**: (x, y) becomes (−y, x), no sine or cosine, so lines and arc
  centres land exactly where they should. An arc's `start_angle` gets + k·90°, wrapped into
  [0, 2π) (its radius and sweep stay).
- A turn keeps the drawing's winding (it isn't a mirror): outer and hole stay outer and hole,
  inside/outside profiles and climb/conventional mean the same as before. A mirror is not part
  of this.
- Updates count a shape as unchanged within `PLACEMENT_ROUNDING_MM` for points and
  `ANGLE_TOL_RAD` for arc angles (turning four times by 90° may differ in the last bit from
  not turning).

## 3. Turning

`place_drawing` gets two more moves: **turn counter-clockwise** and **turn clockwise**, by 90°
each, **about the centre of the imported shapes' bounding box** (decision 2), so the drawing
stays where it is on the stock and only its orientation changes. Under the hood this is a new
`quarter_turns` and a new (dx, dy); the report says "Turned the imported drawing 90°
clockwise; it is now turned 270° and moved X… Y… from the DXF."

As with moving:

- Start points on imported shapes turn with it (same rule: nearest shape the operation
  selects); others stay.
- Drawn shapes and a work zero at a point stay, and the report says so.
- The "extends beyond the stock" note shows when it no longer fits; the user can then press
  *Bottom left* or *Centre*, or change the stock. Copies keep their pitch: if turned copies
  now overlap, running refuses it with the usual message.
- Nothing about cutting order, sides or directions changes.

## 4. CLI

```
chipload place part.chipload --rotate 90           # counter-clockwise; also 180, 270, -90
chipload place part.chipload --rotate -90 --align bottom-left --margin 10
```

`--rotate` takes a multiple of 90 (degrees, counter-clockwise positive, as everywhere in
Chipload); anything else is refused. It combines with `--align` (turn first, then align) but
not with `--by` / `--to`, whose offsets would be ambiguous after a turn (decision 4).
`place` with no option prints "turned 90°, moved X… Y… from the DXF". `--reset` puts the
drawing back where the DXF has it, undoing turn and move (`--to 0,0` only sets the move; the
turn stays).

## 5. GUI

The *Drawing position* section gets two buttons on a *Turn* row: **⟲ 90°** and **⟳ 90°**.
Each press is one undo step. The summary and the *Moved from the DXF* row include the turn
("turned 90°, moved X 120 Y 90 mm"). Its *Reset* button undoes the turn as well as the move
(the CLI's `--reset`), so it really is back where the DXF has it.

## 6. Format

`placement.quarter_turns`, an integer 0-3, required. Recommended as **schema 9** with a
migration v8 → v9 adding `quarter_turns: 0` (decision 5). A frozen v8 sample already exists
(`tests/corpus/jobs/v8/placed_part.chipload`) and must keep its golden G-code; a new v9 sample
with a turned drawing gets its own golden file.

## 7. Decisions

1. **Steps only.** Recommended: **90° steps only**, stored as quarter turns. Exact, and what
   was asked. Any angle could come later as its own change (it needs trigonometry and a
   tolerance, and another format change).
2. **Turn about what.** Recommended: **the centre of the imported drawing's bounding box**, so
   it turns in place. Alternatives: the DXF origin (simple, but the drawing jumps away), or the
   stock centre.
3. **Clockwise and counter-clockwise buttons** in the GUI, both. Recommended: yes (one press
   instead of three for the other direction).
4. **CLI:** `--rotate DEG` (multiples of 90, CCW positive), combinable with `--align`, not
   with `--by`/`--to`. Recommended as described.
5. **Schema 9 with a migration.** Strictly, an optional field with default 0 would do (older
   Chipload refuses unknown fields anyway), but a bump makes an older Chipload say "newer
   version" instead of "unknown field", as we did for the placement. Recommended: **bump to 9**.

## 8. Tests

- Job: migration v8 → v9 adds 0 turns; quarter_turns outside 0-3 is refused.
- Pipeline: turning a square with an arc and a hole four times returns it (within
  tolerance) to where it was; one turn about the bbox centre keeps the centre; lines and arc
  centres match `(−y, x)` exactly; outer and hole stay outer and hole, and a profile's
  side/direction gives the same toolpath turned (compare the turned program with the original
  program turned point by point); start points turn; drawn shapes don't; update after a turn
  keeps it and reports shapes unchanged.
- CLI: `--rotate 90/180/270/-90`, `--rotate 45` refused, `--rotate` with `--by` refused,
  `--rotate` with `--align`; the turned project runs through the checker.
- GUI: both buttons, undo.
- Golden: the v9 sample; existing golden files unchanged (v8_placed_part included).
