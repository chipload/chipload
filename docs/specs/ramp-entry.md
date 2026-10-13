# Spec: ramp entry

Status: **approved 2026-10-10**, ramp only. The user: "do only ramp down now for both profile
and pocket". The helical entry of the first draft is postponed (§8). Decisions as
recommended.

Guide: docs/book/src/operations/profile.md, docs/book/src/operations/pocket.md.

User request (2026-10-10): "Spiral plunge for tools that are not end-cutting." Until now
every operation plunged straight down. A tool whose end teeth don't reach the centre (not
centre-cutting) can't do that: the uncut core under its centre rubs, burns or breaks the
tool.

> **Profiles changed 2026-10-12** (docs/specs/profile-spiral.md): a ramped profile now
> spirals forward down the ring and follows sloped tabs; the out-and-back ramp below is
> only used by pockets.

## 1. What changes

With ramp entry the tool goes down into the material **while moving along its own path**,
so the side teeth do the cutting. It goes out along the path and back, descending all the
way and never steeper than the ramp angle, until it is back at its starting point at the
pass depth. Then the pass runs exactly as before.

Because the ramp ends where a plunge would have left the tool, nothing else in the
toolpath changes: rings, links, tabs, wall checks and the uncut-material checks all stay as
they are. And because it only moves on the path the pass cuts anyway, it can't hit anything
new.

## 2. Format (no version bump)

An optional `entry` per profile and pocket; missing means `{"type": "plunge"}`, the old
behaviour:

```json
"entry": {"type": "ramp", "ramp_angle": 0.0524}
"entry": {"type": "plunge"}
```

- `ramp_angle`: radians in the file, degrees in the GUI. Rule: 0 < angle ≤ 30°.
- Default when switched on: 3°.

## 3. The ramp

- The tool first goes straight down to the depth **already cut** there, at the plunge
  feed. That only passes through air or cut material. The cut depth is the previous pass,
  or for the first pass the operation's start depth (as docs/specs/depths-and-clearance.md
  defines it: where the cut starts).
- Then it ramps over the next `L` mm of the path and back. `L` is half the length a single
  ramp at the angle would need, so one leg out and one back when the path is long enough.
  It is limited by:
  - the path's length;
  - the next tab, after a tab.
- A shorter path takes more legs, never a steeper angle. The number of legs is even, so the
  ramp ends at its start, exactly at the pass depth.
- Ramp moves use the operation's **cutting feed**. At 3° the tool goes down at about 5 % of
  that feed.
- More than 1000 legs (a path far too short for the angle) is refused with a message.

## 4. Where it ramps

Every descent into a pass:
- the start of each depth pass of a profile, and of each pocket piece;
- every re-plunge in a pocket between rings (when the tool lifts instead of staying down),
  along the ring that follows;
- after each tab, along the path up to the next tab. The tab verification counts only the
  laps; the ramps are cut on top of them.

Re-plunges into material that is already cut ramp too. That is simpler and costs a little
time; proving it isn't needed could come later.

## 5. Tool rules

| Entry | Centre-cutting tool | Not centre-cutting | Unknown |
|---|---|---|---|
| plunge, pocket | fine | error | note |
| plunge, profile | fine | **new: error**, suggests a ramp | **new: note** |
| ramp | fine | fine | fine |

- New operations get a ramp when the project's tool is marked not centre-cutting, and a
  plunge otherwise. It is visible in the form and saved explicitly.
- Pockets still need an endmill.

## 6. Output and checking

- Ramp moves are G1 X Y Z. They are never merged into arcs, since only XY-only moves are,
  so no helical arcs are needed and the independent checker accepts them as they are.
- Existing programs are byte-identical: every existing job plunges. The golden test proves
  it; the new golden program `v7_ramp_entry` adds ramps in a pocket and in a profile with
  tabs, and passes the checker.

## 7. Tests

- Unit (ramp):
  - out and back once on a long path;
  - more legs on a short one, never steeper, always descending, ending exactly at depth;
  - nothing to ramp at depth;
  - too short is refused.
- Plan:
  - a profile with a tool that can't plunge is refused unless it ramps;
  - with ramps and tabs every straight descent reaches only depths already cut, and every
    sideways descent is within the angle;
  - pockets ramp in every strategy (rings, trochoidal, adaptive) from a start depth > 0;
  - an angle over 30° is refused.
- GUI: new operations ramp when the tool can't plunge.
- Golden: unchanged, plus `v7_ramp_entry`.

## 8. Later (postponed by the user)

- **Helical entry for pockets.** A helix round the point furthest from the walls; it needs
  helical arcs in the post and the checker. Designed in the first draft of this spec (git
  history, 2c372f7).
- **Lead-in/lead-out** with a draggable entry point (its own spec). The ramp would then run
  on the lead-in.
