# Spec: holes (drilling and helical boring)

Status: **approved 2026-10-13**, all decisions as recommended (§10).
Built 2026-10-13. As built:
- §3: pecks are even, each at most `peck` mm, the last exactly at the end depth (as depth
  passes are); "merged within the depth tolerance" follows from that.
- §4 step 5: the tool steps off the wall toward the centre only when the centre is cut
  (`rc ≤ r`). With a core it retracts straight up at the wall, as a profile does: any move
  inward would run into the core.
- §4: a pocket's helix gives no centre-cutting note (a helix never plunges), so neither
  does a bore.
- §6: the setup sheet shows "Holes, drilled", "0 to 12 (the drill's point), pecks 4 mm",
  the feed column "300 down" for a drill, and "helix 3°" as the entry; it doesn't count
  the holes (the notes list the ones skipped).
- §6: the checker also forgets the hole at a tool change and when the spindle starts or
  stops.
- §7: the canvas highlights every selected circle; one a drill skips for its size stays
  highlighted (that rule is the planner's, the GUI doesn't repeat it), and the note says
  so.

User request (2026-10-13): commercial-CAM features 1–5 and 7; this is 3. Today a round hole
is cut as an inside profile: a plunge or ramp, then laps round the hole in depth steps. That
is slow for small holes, and a drill can't be used at all (every operation refuses a
drill). This adds one operation for round holes:

- **with a drill:** straight down at the hole's centre, in pecks for deep holes;
- **with an endmill:** a **helical bore**, a helix at the wall down to the end depth and one
  level circle at the bottom. It needs no plunge, the tool cuts with its side, and one
  endmill makes every hole size larger than itself.

## 1. What it does

A new operation, **Holes** (`"type": "holes"`). It selects shapes like any operation and
cuts every selected **circle**:

- a circle drawn in Chipload, or
- an imported closed contour made only of arcs with one centre and one radius (a DXF
  CIRCLE, or two half arcs), all within the chaining tolerance.

Every other selected shape (rectangles, polylines, open paths, other closed contours) is
skipped, with one note per operation: "skipped 3 shapes that aren't circles". DXF POINT
entities are still ignored at import (decision 8).

The way it cuts depends on the operation's tool (decision 1):

| Tool | Method | Which circles |
|---|---|---|
| Drill | drill cycle (§3) | diameter within ±0.1 mm of the drill (decision 2) |
| Endmill | helical bore (§4) | diameter larger than the tool (§4) |
| V-bit | refused: "a V-bit can't cut holes; use a drill or an endmill" | |

## 2. Format: schema 18 (decision 7)

```json
{"type": "holes", "name": "Holes", "enabled": true,
 "select": {"layer": "HOLES", "shapes": null}, "tool": 4,
 "start_depth": 0.0, "end_depth": 19.0,
 "feed": 1500.0, "plunge_feed": 300.0,
 "method": {"type": "drill", "peck": 4.0}}
```

```json
"method": {"type": "bore", "step_down": 2.0, "ramp_angle": 0.0524,
           "direction": "climb"}
```

- `start_depth`, `end_depth`: as on every operation, mm down from the stock top, 0 ≤
  start < end. For a drill, `end_depth` is the depth of the **drill's point** (Z zero is
  set with the point, as for every tool; decision 3).
- `plunge_feed`: the drill's feed down; the bore's straight descent to the start depth.
  `feed`: the bore's helix and circle. A drill doesn't use `feed`. The form doesn't show it
  for a drill. It is still saved, so switching the tool back keeps it.
- `method`, matching the tool. A drill takes `drill`, an endmill takes `bore`. Anything else
  is refused with a message that names the tool type.
  - `drill.peck`: mm drilled per peck, more than 0; `null` drills straight to the end
    depth in one go.
  - `bore.step_down`: most mm the helix goes down per turn, more than 0.
  - `bore.ramp_angle`: steepest slope of the helix, 0 < angle ≤ 30°, as for ramps.
  - `bore.direction`: `climb` or `conventional`, with the meaning profiles give it on an
    inside contour.
- No `side`, `clearance`, `tabs`, `lead`, `starts` or `entry`: a hole is cut to the drawn
  size, from its own starting point. Clearance for a fit can come later (decision 9).
- A new operation type would trip older versions, so the schema goes to 18, as it did for
  follow path at 17. Nothing to migrate: 17 → 18 only changes the version.

## 3. Drill cycle

For each hole:

1. Rapid to the hole's centre at the safe height (as for every operation).
2. Feed straight down at `plunge_feed`, from the safe height to the first peck's depth, or
   to the end depth when `peck` is `null`.
3. **Pecks** (decision 4): rapid up to the safe height, which clears the chips from the
   flutes. Then rapid back down **into the same hole**, to 0.5 mm above the depth
   already drilled (a named constant, `PECK_CLEARANCE_MM`). Feed down one more peck, and
   repeat until the end depth.
4. Rapid up to the safe height.

- Every hole goes to the full end depth. A peck that would end within the depth tolerance of
  the end depth is merged into the last one.
- Holes are written as **plain moves** (G0/G1), not as the controller's drill cycles
  G81/G83 (decision 5). They run the same on GRBL, which has no drill cycles, and on
  LinuxCNC and Mach. The checker reads them like every other move. Canned cycles can come
  later as a post-processor option, with the checker taught to read them.
- A drill is centre-cutting by its shape: no centre-cutting note.

## 4. Helical bore

For a hole of radius `R` and a tool of radius `r`, the tool centre follows a circle of radius
`rc = R − r`. The cut comes out at the drawn size.

1. Rapid to the helix's start at the safe height: the point of the centre circle at angle 0
   (+X from the hole's centre, as a drawn circle starts).
2. Feed straight down at `plunge_feed` to the start depth. Above the stock this is air; below
   it, the start depth is cut by an earlier operation.
3. **Helix down**, as helical G2/G3 arcs at `feed`, turning in the direction `direction`
   gives. Each full turn goes down `min(step_down, 2π · rc · tan(ramp_angle))`, evenly, to
   the end depth.
4. **One level circle** at the end depth, which cuts away the helix's slope and leaves a flat
   bottom and a clean wall.
5. Leave the wall: feed straight toward the hole's centre, by `min(rc, r / 2)`. Then rapid up
   to the safe height. With `rc ≤ r` the centre is already cut, so the move ends inside cut
   material.

Rules:

- **The hole must be larger than the tool:** `R − r` must be at least `0.1 × D`, the smallest
  helix radius pockets accept (helix-and-leads.md decision 6). A smaller circle is skipped,
  with a note: "hole 6.2 mm: too small for the 6 mm tool to bore".
- **The core** (decision 6): with `rc ≤ r` (hole diameter at most twice the tool's), the
  helix cuts the whole hole. With a larger hole the helix cuts a ring and leaves a core:
  - In a **through hole** (end depth at the stock thickness, within the depth tolerance),
    the core comes loose at the last turn, as the waste of an inside profile does. A note
    says so: "4 holes leave a loose core (hole more than twice the tool); hold it down or
    use a pocket".
  - In a **blind hole** the core would stay standing. The hole is refused: "hole 30 mm is
    blind and more than twice the 6 mm tool; use a pocket".
- **Tools that aren't centre-cutting:** the helix never plunges, so any endmill is fine. With
  `rc ≤ r` the tool's centre still passes over uncut material at the helix's slope. That is
  the same as a pocket helix, so the same note applies when the tool is marked not
  centre-cutting.
- Arcs are helical G2/G3, at most 180° per move, as for the pocket helix. With arcs off they
  are chords.

## 5. Order, copies and other operations

- Holes are cut in the order a greedy nearest-neighbour picks, starting from where the tool
  is, as follow path does.
- With copies, each copy's holes are cut in turn, in the copy order the job already uses.
- A Holes operation runs where it sits in the operation list, like every operation, so the
  user decides whether holes come before or after the profiles.

## 6. Checks and notes

- **The checker learns one rule** (decision 4): a rapid down below the stock top is allowed
  only **back into the hole the tool just came out of**. All three of these must hold:
  - since its last feed down, the tool has moved only straight up and down, with no XY
    move;
  - that feed went straight down at this same X and Y;
  - the rapid ends at least 0.5 mm above the deepest point of that feed (the checker's own
    copy of the constant).

  Every other rapid below the stock top is still "rapid into material". The rule lives in
  `chipload-check` and never uses the planner's code.
- Through holes: the end depth may equal the stock thickness. As for every operation,
  deeper than the stock bottom is refused.
- The notes listed in §1 and §4.
- An operation that selects no circle it can cut is reported as other operations report an
  empty selection.
- **Setup sheet:** "Holes: 12 drilled with the 8 mm drill, pecks 4 mm" or "Holes: 4 bored,
  helix 2 mm per turn". For a drill, the depth line says that it is the depth of the point:
  "point at 19 mm; the full diameter is about 0.3 × D less deep".
- The material-left display, the cycle time estimate and the toolpath preview work from the
  moves, as for every operation.

## 7. GUI and CLI

- **GUI:** a "+ Holes" button next to the other operations. It takes the canvas selection
  (circles), or nothing to pick a layer. The form shows:
  - the tool and the depths;
  - for a drill: the plunge feed, a "Peck" checkbox and the peck depth;
  - for an endmill: the feeds, step per turn, ramp angle and direction.

  Changing the tool between a drill and an endmill switches the method, with the default
  values: peck `null`; step 0.5 × D; 3°; climb.
- The canvas highlights the operation's circles. Skipped shapes are drawn as unselected.
- **CLI:** through the project file and `run`, `plan`, `sheet`, `check`, as for other
  operations.

## 8. Tests

- **Unit, drill:**
  - every hole at its centre;
  - pecks to the right depths, the last peck merged at the boundary;
  - `peck` null;
  - peck larger than the cut;
  - the 0.1 mm fit, both inside and outside it.
- **Unit, bore:**
  - every helix move lies on the circle with radius `rc`;
  - the descent per turn is never more than `step_down` or the ramp angle;
  - the level circle at the end depth;
  - the leave move;
  - climb and conventional, with both spindle directions;
  - a hole exactly `D + 0.2 × D` (the smallest that fits) and just under it;
  - `rc = r` exactly (no core);
  - a through hole with a core (note) and a blind one (refused).
- **Circle recognition:**
  - a drawn circle;
  - a DXF CIRCLE;
  - two half arcs;
  - an arc polygon that is almost a circle, just outside the tolerance (not a circle);
  - a rectangle (skipped with a note).
- **Checker:**
  - a peck re-entry is accepted;
  - a rapid down at another XY is refused;
  - a re-entry deeper than drilled less 0.5 mm is refused;
  - a re-entry after an XY move is refused.
- **Plan:**
  - a V-bit is refused;
  - a method that doesn't match the tool is refused;
  - nearest-neighbour order;
  - copies.
- **Migration:** a v17 sample migrates to 18 with only the version changed. The frozen
  samples keep their G-code.
- **Golden:**
  - a new `v18_holes`, with drilled holes with pecks and bored holes (one through hole with a
    core), and its `lines/` twin;
  - a setup sheet golden;
  - every existing golden unchanged.
- **Proptest:** random holes and tools. Every bore move stays within the hole, at radius
  `rc` from its centre, never below the end depth. Every drill move is at a hole's centre.

## 9. Not in this step

- Canned cycles (G81/G83) as a post option.
- Points (DXF POINT) as hole centres.
- Clearance or allowance for a fit.
- Dwell at the bottom.
- Chip-breaking pecks (G73).
- Chamfering holes with a V-bit.
- Counterbores and countersinks as one operation. Today they are two Holes operations with
  different depths and tools.

## 10. Decisions

1. **One "Holes" operation, its method set by the tool:** a drill drills, an endmill bores.
   **Recommended**: one place for holes, and the method is explicit in the file (`method`)
   and checked against the tool. Alternative: two operations (Drill, Bore).
2. **A drill cuts only circles within ±0.1 mm of its diameter.** Other circles are skipped
   with a note. **Recommended**: a 10 mm circle is never drilled with an 8 mm drill by
   mistake. Alternative: drill the centre of every selected circle, whatever its size, with
   a note when the size differs (pilot holes).
3. **A drill's `end_depth` is the depth of the point**, as Z moves, with a note on the sheet
   about the full diameter. **Recommended**: the same meaning as every other depth. Drills
   have no point angle in the tool data. The alternative, a full-diameter depth, needs one.
4. **Pecks retract fully to the safe height**, then rapid back into the hole to 0.5 mm above
   the drilled depth. **The checker gets a narrow rule to allow that rapid.**
   **Recommended**: full retracts clear wood and plastic chips, and the rapid re-entry
   keeps a deep hole from taking minutes. Alternative: feed back down from the safe height
   at the plunge feed. The checker is then unchanged, but the time spent in air grows with
   the square of the depth.
5. **Plain G0/G1 moves, not G81/G83.** **Recommended**: works on GRBL (no drill cycles), and
   the independent checker sees every move. Canned cycles can come later as a post option.
6. **Bores larger than twice the tool:** through holes are bored with a note about the
   loose core; blind holes are refused ("use a pocket"). **Recommended**. Alternative: clear
   the core with more rings, which a pocket already does.
7. **Schema 18** for the new operation type, a migration that only changes the version.
   **Recommended**, as follow path did.
8. **Only circles for now, no DXF points.** **Recommended**: a point has no size, so
   neither the drill's fit check nor a bore can use it. Points can come later, for a drill
   only.
9. **Holes cut to the drawn size, no clearance field.** **Recommended** for now: draw the
   hole at the size wanted. A clearance can come later as an optional field.
