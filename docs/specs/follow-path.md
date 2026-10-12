# Spec: follow path (cutting open paths)

Status: **approved 2026-10-13**, all decisions as recommended (§9).

User request (2026-10-13): commercial-CAM features 1–5 and 7; this is 1. Today open contours
are never cut: profiles and pockets skip them with a note ("skipped open contour"). That is
right for centrelines drawn on cutting layers (user-drawing-conventions), but leaves no way
to engrave a line, cut a V-groove, score a fold line or cut a single line through thin
material.

## 1. What it does

A new operation, **Follow path**: the tool centre follows the selected shapes exactly, on
the line, down to the end depth in passes. No offset, no side, no waste side.

- It cuts **open** paths, and **closed** ones too (on the line, as a profile "on" does), so
  one operation can engrave a drawing that mixes both.
- It cuts **only what it selects**. Profiles and pockets keep skipping open contours, so a
  centreline on a cutting layer is still never cut by them. A Follow path that selects a
  whole layer does cut that layer's centrelines; the GUI makes new Follow paths from the
  shapes selected on the canvas.

## 2. Format: schema 17 (decision 7)

A new operation type:

```json
{"type": "path", "name": "Engrave", "enabled": true,
 "select": {"layer": null, "shapes": [4, 5]}, "tool": 2,
 "start_depth": 0.0, "end_depth": 1.5, "step_down": 0.5,
 "feed": 1200.0, "plunge_feed": 400.0,
 "entry": {"type": "plunge"}}
```

- The fields mean what they mean on a profile. `entry` is `plunge` or `ramp` (§4). No
  `side`, `clearance`, `direction`, `tabs`, `lead` or `starts` (decisions 3–5).
- Older Chipload versions must refuse the file as newer rather than trip over an unknown
  operation type, so the schema goes to 17 (as V-carve did for 10). Nothing to migrate:
  16 → 17 only changes the version.

## 3. Order and direction

- Each open path is cut from one of its ends. Paths are cut in the order a greedy
  nearest-neighbour picks from where the tool is, choosing for each path the nearer of its
  two ends (decision 4). Closed contours go in that order too, starting where a profile
  "on" would start them.
- Within a path the tool goes the way it started; for the depth passes see §4.

## 4. Depth passes on an open path (decision 2)

An open path has two ends, so the tool can't go round and round as on a ring.

- **Plunge entry:** plunge at the start end, cut to the other end, plunge there to the next
  depth, cut back, and so on: back and forth, one depth per traversal, no retracts.
  Retract after the last pass.
- **Ramp entry:** the tool **descends while it travels**, back and forth along the whole
  path: each traversal goes down evenly by at most one step, never steeper than the ramp
  angle (a short path takes more traversals per step, never a steeper slope), until the
  end depth. Then one level traversal at the end depth cuts the last slope away. No plunge
  anywhere: the tool is straight down only at the very start, to the start depth.
- Either way, under the tool there is never more than one step-down of uncut material.
- Closed contours in a Follow path are cut exactly as a profile "on" with the same entry
  (the spiral from docs/specs/profile-spiral.md for a ramp), without tabs and leads.

## 5. Tools (decision 6)

- **Endmill:** a slot as wide as the tool. Plunge entry needs a centre-cutting tool (the
  same rule and messages as profiles).
- **V-bit:** a V-groove; its width at the top follows from the depth and the angle, and is
  shown in the report ("groove 2.31 mm wide at the top"). A pointed V-bit always plunges
  fine (it is centre-cutting by shape).
- **Drill:** refused ("use a drilling operation").

## 6. Safety and checks

- Moves follow the drawn geometry exactly; arcs are written as G2/G3 like rings (chords
  tagged as for offsets). Ramp moves on arcs become helical arcs.
- The checker runs as for every program (depths, feeds, rapids, arcs).
- Self-crossing or overlapping paths are cut as drawn; where a path crosses itself or
  another path, the tool passes cut material (no extra rule).
- Zero-length paths (after import cleanup) and paths shorter than the chaining tolerance
  are skipped with a note.
- The material-left display is off for Follow path (it isn't trying to clear anything).

## 7. GUI and CLI

- **GUI:** a "+ Follow path" button beside "+ Profile", "+ Pocket", "+ V-carve", taking
  the canvas selection (open and closed shapes). The form: tool, depths, step down,
  feeds, entry. The operation's paths are highlighted like other operations' contours.
- **CLI:** works through the project file and `run`, `plan`, `sheet`, `check`, as other
  operations do. The setup sheet lists it ("Follow path, on the line").
- Open contours drawn dashed as "open contour (not cut)" keep that look unless a Follow
  path selects them; then they are drawn as the operation's contours.

## 8. Tests

- Unit: back-and-forth passes (plunge), the descending traversals (ramp) never steeper than
  the angle and never more than a step under the tool, a short path takes more
  traversals; the nearer end is chosen; closed contours match a profile "on".
- Plan: a Follow path selecting open and closed shapes; a profile selecting the same layer
  still skips the open ones; a drill is refused; a V-bit groove width note.
- Migration: v16 sample → 17 changes only the version; the frozen samples keep their
  G-code.
- Golden: a new `v17_follow_path` (open and closed paths, ramp entry, a V-bit) and its
  `lines/` twin; every existing golden unchanged.
- Proptest: random polylines with arcs: every cutting move lies on the path (within the
  chaining tolerance), only forward or backward along it, depth per traversal ≤ step.

## 9. Decisions

1. A new operation type (**"Follow path"**, `"type": "path"`), not open paths in the
   profile. **Recommended**: profiles keep skipping centrelines, and the form has no
   meaningless side/tabs/leads.
2. Depth passes back and forth along an open path; with a ramp, descending while it
   travels, then one level pass. **Recommended**: no retracts, no plunges with a ramp.
   Alternative: one way only (retract and return to the start for every pass): cleaner
   for some materials, more air time.
3. On the line only for now: no left/right offset for open paths. **Recommended**; an
   offset to one side (e.g. a pocket-width groove beside a line) can come later.
4. Start at the nearer end of each path, in nearest-neighbour order; no start points or
   leads for open paths yet. **Recommended**: simple and short travel.
5. Closed contours allowed in a Follow path, cut on the line like a profile "on" (without
   tabs or leads). **Recommended**: one operation engraves a whole drawing.
6. Tools: endmill and V-bit, drill refused; the V-groove's top width reported.
   **Recommended.**
7. Schema 17 for the new type (nothing to migrate). **Recommended**, as for V-carve.
