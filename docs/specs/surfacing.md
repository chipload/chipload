# Spec: surfacing (flatten the spoilboard, face the stock)

Status: **draft 2026-10-17**, decisions in §10 for review.

User request (2026-10-17): "how would the surfacing work?", then "go ahead with the spec".
ADR 0017 has surfacing out of scope since 2026-10-13 ("a few lines of G-code that any
sender or a small script makes"). This spec reopens it (decision 1): a script gives the
moves, but not the checker, the simulation, the time estimate or the setup sheet, and a
surfacing program is the one most often run with the machine's largest tool right at its
travel limits.

## 1. What it does

A new operation, **Surface** (`"type": "surface"`). A wide flat endmill (a surfacing or
fly-cutter bit) sweeps an area in straight passes, a step down at a time, until it is
flat at the end depth. Two uses, one operation:

- **Flatten the spoilboard:** the stock is the spoilboard; the operation takes 0.3–1 mm
  off its top so the next sheet lies flat.
- **Face the stock:** take the top of a board down to a clean, even surface before the
  other operations.

## 2. The area (decision 3)

- **The whole stock** by default: an empty selection means the stock rectangle. Then the
  operation is cut **once**, not per copy of its part (copies are copies of the drawing on
  one stock; facing the stock twice cuts air the second time).
- **Selected closed shapes**, to flatten only part of the bed or a raised area: each shape's
  area, as a pocket's, copied with its part like any operation. Open shapes are skipped
  with a note.

## 3. The pattern (decision 2)

Back and forth along the area's **longer side** (X for a square):

1. The area is grown by the **overrun** on every side (§4), so passes start and end in
   the air past the edge and leave no ridge along it.
2. Passes are straight lines along the long side, a stepover apart across it. The first
   and last passes are placed so the tool's edge reaches the grown area's sides; the
   stepover is shrunk evenly to fit (never grown), as a pocket's step down is.
3. Between passes the tool moves across at cutting depth, outside the area by the overrun:
   no retract.
4. For a shape (not a rectangle), each pass is clipped to the grown shape; where one pass
   splits into several stretches (a shape with a bay), the tool retracts between them,
   since the gap may hold stock that isn't being faced. A shape with holes is faced round
   them the same way.
5. Each depth starts again at the same corner, outside the stock: rapid there at safe
   height, down in the air to the depth, cut. **No entry** (plunge, ramp, helix) is ever
   needed, since the tool never goes down over the area (§5).

Half the passes are climb and half conventional (decision 4). On a facing pass of a wide
flat bit at a shallow depth that is normal practice; the setup sheet says
"back and forth (climb and conventional)".

## 4. Settings

| Field | Default | Note |
|---|---|---|
| `tool` | the project's widest endmill | an endmill; a V-bit or drill is refused |
| `start_depth` | 0 | |
| `end_depth` | 0.5 | how much comes off; must be at most the stock thickness |
| `step_down` | from the tool, as others | usually the whole depth in one pass |
| `stepover` | 0.75 × D | 0.1–0.9 × D, a problem outside |
| `overrun` | 0.5 × D | how far the tool centre goes past the area's edge, 0–D |
| `feed`, `plunge_feed` | from the tool, as others | plunge feed for the move down in the air |
| `direction` | none | not a setting: back and forth (decision 4) |

The overrun is measured from the area's edge to the tool centre; at 0.5 × D the tool just
clears the edge. Less than that leaves a strip along the edge uncut: a warning (amber),
not a problem, as some setups surface only up to a fence or a clamp line (decision 5).

## 5. Safety and checks

- **Machine limits:** the overrun takes the tool past the stock. A pass that would leave
  the machine's travel is refused by the planner with the operation and side named
  ("the overrun reaches X −12 mm, the machine starts at 0: lower the overrun or move the
  stock"), and the independent checker refuses it anyway.
- **Rapids:** every rapid down to cutting depth happens outside the grown area, where there
  is no stock to face. The checker's rapid-through-material rule covers this already; no
  checker change is needed.
- **Clamps:** the setup sheet says "runs 6 mm past the stock edge on every side: keep
  clamps clear".
- **Max depth:** the end depth is checked against the stock thickness like every
  operation; a surface operation can't go below the stock bottom (`max_overcut` doesn't
  apply: flattening the spoilboard *is* its stock).

## 6. Order, and the other operations (decision 6)

- A surface operation is an operation like the others: cut where it is in its part's list,
  so normally it is put first. The project problems warn (amber) when a surface operation
  of the whole stock comes after another operation in the same part: "Operation 3 faces
  the stock after operations 1–2 have cut it".
- Other operations' depths **don't change**: they still count from the stock top as
  entered. Facing 0.5 mm off means their first 0.5 mm cuts air. That is safe and
  predictable; moving every depth would change how a saved project cuts without asking.
  The setup sheet notes it.

## 7. Output

Plain G0/G1, so every post runs it. The G-code comment names it as others:
"Operation 1: Surface, T3". The simulation, playback, time estimate (including the moves
in the air), chip load and setup sheet treat it as any operation; the sheet's picture
draws the grown area dashed.

## 8. App and command line

- **App:** "+ Surface" in the ribbon's OPERATION group, enabled with nothing selected (the
  whole stock) or with closed shapes selected. The Browser row and Properties as other
  operations: Tool, Depths, Step down, Stepover, Overrun, Feeds.
- **CLI:** `chipload op add-surface part.chipload [--tool 3] [--depth 0.5] [--shapes 2,4]`,
  and `get`/`set` paths as other operations (`operations.0.overrun`).
- **Guide:** a new page, docs/book/src/operations/surface.md, with a spoilboard recipe (a
  project whose stock is the bed, Z zero at its top).

## 9. Format: schema 29

```json
{"type": "surface", "name": "Surface", "enabled": true, "part": 1,
 "select": {}, "tool": 3, "start_depth": 0.0, "end_depth": 0.5,
 "step_down": 0.5, "stepover": 19.05, "overrun": 12.7,
 "feed": 3000.0, "plunge_feed": 1000.0}
```

A new operation type: older Chipload would refuse the file, so the version goes to 29.
`v28_to_v29` changes nothing (no older file has one); the frozen sample is
`tests/corpus/jobs/v28/` (already there) and a new golden `v29_surface`.

## Tests

- A rectangle stock: passes cover every point of the stock (the simulation's height map is
  at the end depth everywhere inside), the first and last pass reach past the edges by the
  overrun, no move goes down over the stock.
- Boundaries: depth equal to one step down; depth equal to the stock thickness; a tool wider
  than the stock (one pass); a stepover that divides the width exactly; overrun 0.
- A shape with a bay and one with a hole: retracts between stretches, nothing faced inside
  the hole.
- Copies: the whole-stock surface is cut once; a selected-shape surface per copy.
- Machine limits: an overrun past the travel is refused with the side named.
- Property test: random rectangles and stepovers, the faced area is complete and no move
  lies outside the grown area.
- Golden G-code `v29_surface` (with arcs off and on, the same: no arcs), and its setup sheet.

## 10. Decisions

1. **Build it, and amend ADR 0017.** *Recommended:* the checker, simulation, estimate and
   sheet are what a script can't give, and it is small. Alternative: keep it out.
2. **One pattern: back and forth along the longer side.** *Recommended:* fewest turns, even
   finish, the usual way to surface. Alternatives: a spiral (a pocket's rings without
   walls), or an angle setting. Per "no worse options", only one.
3. **The whole stock by default, cut once; or selected closed shapes.** *Recommended:* the
   stock is what one faces nearly always; shapes cover a raised area or part of the bed.
4. **Back and forth, not one direction.** *Recommended:* one-way passes go back above the
   stock each time and take about twice as long, for no gain at a shallow facing depth.
5. **Overrun 0.5 × D by default, 0 to D, a warning below 0.5 × D.** *Recommended:* the edge
   is clean by default; a fence or clamp line can still be respected.
6. **Other operations' depths don't move.** *Recommended:* safe, and nothing changes
   silently; a warning when facing comes after other cuts.
7. **Schema 29**, a new operation type, as holes was (schema 18).
