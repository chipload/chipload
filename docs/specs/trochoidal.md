# Spec: trochoidal pocket clearing

Status: **approved 2026-10-10** (Q1–Q4 as recommended).

User decisions (2026-10-10): pockets get a choice of clearing strategy, trochoidal rather
than zig-zag; for whole pockets and for slots; arcs first (done, docs/specs/arc-output.md);
loop radius and step per loop with defaults from the tool, shown and saved explicitly; the
strategy is optional and defaults to today's rings.

## 1. What it is

Instead of cutting rings with the tool's full width in the corner of each ring, the tool runs
small circles (loops) that move forward a little each time. Each loop takes only a thin bite
of new material, so the tool is never loaded with its full width, even in corners. That
allows deeper step-downs with less heat and tool load: it suits hard materials and narrow
pockets.

## 2. Format (no version bump)

`PocketOp` gets an optional `strategy`, default today's rings:

```json
"strategy": {"type": "rings"}
"strategy": {"type": "trochoidal", "loop_radius": 1.5, "step": 0.6}
```

- Optional with a safe default (rings, today's behaviour), so no schema bump. Saving writes
  it explicitly.
- `loop_radius` (mm) and `step` (mm, how far each loop moves on) are required for
  trochoidal. When you switch an operation to trochoidal the GUI fills in 0.25 × D and
  0.1 × D for the current tool, visibly, and you can change them.
- Rules: `0 < step ≤ loop_radius`, `loop_radius > 0`. `stepover` keeps a meaning (§3).

## 3. The path for one pocket

With tool radius `r`, loop radius `ρ`, step `p`, stepover `s × D`:

1. **Backbones**: the pocket shrunk by `r + ρ`, then by `r + ρ + k·b` for k = 1, 2, …
   until empty, where the band spacing is `b = 2ρ + s·D`. These are rings, exactly like
   today's pocket rings but further from the walls and further apart, because each loop
   clears a band `2ρ + D` wide around its backbone.
2. **Loops**: along each backbone, loop centres every `p` mm (by length along the ring). At
   each centre one full circle of radius `ρ`, written as two G2/G3 halves, starting and
   ending at the point nearest the next centre. A short straight feed links each loop to the
   next.
3. **Order**: backbones from the inside out (innermost first), as rings are cut today, so
   every band cuts towards uncut material and away from cleared material.
4. **Wall pass**: after the bands, at each depth, one ordinary ring at `r` from the walls
   and islands (today's last ring) finishes the walls the loops left scalloped.
5. **Depths**: each depth pass does all of the above; plunging as pockets do today (straight
   down at the first loop centre, so a centre-cutting endmill is required, as now).
6. **Links between backbones**: retract and plunge, or stay down where safe, exactly by
   today's `keep_tool_down` rule.

Loop direction follows climb/conventional and the spindle, by the same table as ring walls
(the material is outside the loop).

## 4. Narrow parts and slots

- A part of the pocket narrower than `D + 2ρ` has no backbone: the loops can't go there.
  The wall pass still cuts what a full-width tool reaches, and the existing uncut-material
  check warns about anything left. Nothing is cut that the wall pass wouldn't cut today.
- For a slot of width `w`, the loops fit if `ρ < (w − D)/2` (e.g. a 10 mm slot with a
  6 mm tool: `ρ` below 2 mm). The backbone of a slot is a thin loop round its centre line,
  so the loops go along it and back. *(Question 3.)*

## 5. Safety

- Every loop and every link is verified like today's rings: the tool centre keeps at least
  `r` (minus the clearance, if set) from every wall and island, the loop arcs checked where
  they bulge.
- The uncut-material check and the material-left display run on the actual loops and the
  wall pass, so a band spacing that is too wide shows up as missed material and a warning.
- The independent checker is unchanged: it already checks arcs.
- Deterministic: loop centres are placed by length along each backbone from its start
  point, which is chosen as for rings.

## 6. GUI and CLI

- Pocket form: **Strategy**: rings / trochoidal; for trochoidal **Loop radius** and
  **Step per loop**, filled from the tool when switched on.
- The 3D view shows the loops at their depths (they're just more lines).
- The project file carries everything; `run`, `plan` and `check` work unchanged.

## 7. Out of scope

Adaptive clearing (constant engagement along any path), loops that shrink to fit narrow
parts, trochoidal profiles along open lines, helical entry (ramping).

## 8. Tests

- Format: a pocket with and without `strategy` round trips; trochoidal without
  `loop_radius` or with `step > loop_radius` refused.
- Geometry: a 100 × 60 mm square pocket: backbones and loop count as calculated; every loop
  is inside the pocket shrunk by `r`; the wall pass is the last ring at each depth; no
  missed material with default values.
- A slot narrower than `D + 2ρ` gets no loops, only the wall pass, and the warning.
- Direction table: loops CW/CCW × climb/conventional × spindle.
- Property: random pockets with islands, random `ρ` and `p`: no gouge (loops keep `r` from
  walls), every arc valid.
- Rings strategy: every existing golden file unchanged.
- New golden file: the drawn_part pocket with trochoidal clearing; passes the checker.

## 9. Decisions (approved 2026-10-10)

1. **Format**: optional `strategy` object on pockets, default rings, no version bump;
   trochoidal needs `loop_radius` and `step` (GUI fills 0.25 × D and 0.1 × D).
   **Yes.**
2. **Path** (§3): loops along offset backbones spaced `2ρ + stepover·D`, inside out, then
   a wall-finishing ring at each depth. **Yes.**
3. **Narrow parts and slots** (§4): no automatic loop shrinking in this step; where loops
   don't fit, the wall pass cuts and the uncut-material warning tells you. You set a
   smaller loop radius for narrow slots. **Yes**; shrinking loops could come
   later.
4. **Loops as full circles** linked by short straight moves (the common CAM form),
   rather than true cycloid curves. **Yes**: arcs only, simple to verify.
