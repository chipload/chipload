# Spec: finishing pass and onion skin (profiles)

Status: **approved 2026-10-13**, all decisions as recommended (§8).
Built 2026-10-13. As built:
- The finishing and skin laps **never use the lead**: they always step across from the
  roughing slot (§3). A lead's arc can reach past the slot into waste that is uncut at full
  depth.
- With tabs, the finishing and skin laps start in the middle of the longest gap between
  tabs, so the descent in the slot is never onto a tab. Without tabs they start where the
  ring starts (a start point still applies).
- Tabs are placed on the wall ring; each roughing ring gets them carried over (each tab end
  moved to the nearest point of the roughing ring), so every lap leaves the same tabs.
- §6 as built: the finishing lap is checked against the roughing path at every vertex and
  every half tool radius. It is refused where it runs a tool radius or more from that path
  (the roughing couldn't get there, e.g. a hole the roughing ring doesn't fit in), with a
  note where it runs more than twice the stock from it (sharp inside corners, narrow parts).
- A finishing pass on a profile "on" the line is refused (there is no wall); a skin is fine.
- The skin must be thinner than the tab height (§6); "skin plus tab height equal to the
  cut" in §7 isn't a boundary of these rules, so it isn't refused or tested.
- Found on the way: where two stretches of a lap met in mid-ring at the same depth (the skin
  lap's level lap and its rise-out), the meeting point was dropped and the tool rose early.
  Points where a stretch starts are now always kept (existing programs don't change: their
  stretches all meet at the ring's start).

Guide: docs/book/src/operations/profile.md.

User request (2026-10-13): commercial-CAM features 1–5 and 7; this is 2.

- **Finishing pass:** the profile roughs with a little stock left on the wall, then one light
  lap at the wall cuts it away. The wall comes out clean: the tool isn't loaded on that lap,
  so it doesn't deflect, and there are no step marks from the depth passes.
- **Onion skin:** the profile stops a thin skin above the end depth, and a last lap cuts
  through the skin. Small parts stay held by the skin (on a vacuum table, or instead of
  tabs) until the end, and there are no tab stubs to clean up.

## 1. Format (no version bump, decision 7)

Two optional fields on a profile, missing or `null` meaning none (today's behaviour):

```json
"finish": {"stock": 0.3},
"skin": {"thickness": 0.2}
```

- `finish.stock`: mm left on the wall by the roughing laps; more than 0, less than the tool
  radius.
- `skin.thickness`: mm left above the end depth; more than 0, at most the step down, and
  less than the cut (end depth minus start depth).
- Older versions refuse the unknown field, the safe outcome; files written by this version
  always write both (as `null` when unused).

## 2. What is cut

For each ring of the profile (`D` the tool diameter, `r` its radius):

| Laps | Where (tool centre from the line) | Down to |
|---|---|---|
| Roughing, as today (spiral or passes) | `r − clearance + stock` | end depth − skin |
| Finishing, one level lap | `r − clearance` | end depth − skin |
| Skin, one lap | `r − clearance` (the finished wall) | end depth |

- Without `finish`, stock is 0 and there is no finishing lap. Without `skin`, the skin is 0
  and there is no skin lap.
- The roughing rings are offset from the drawn line like today's, `stock` farther out (into
  the waste); they are verified like every offset.
- Tabs (straight or ramped) are left by every lap, including the finishing and skin laps.

## 3. Going down for the finishing and skin laps (decision 2)

The roughing slot is cleared to its depth all the way round, so the tool goes down **in
that slot**, not on the finished wall:

- Straight down at the roughing ring's start (only air and cut material) to the lap's depth,
  then straight across onto the finishing ring (`stock` mm, into a sliver no thicker than
  `stock`), the lap, and back across into the slot before the retract.
- With a lead, the lap arcs on and off it as the lead does today, from the slot.
- The skin lap goes down `skin` mm in its first stretch: straight with a plunge entry, or
  ramping forward along the ring with a ramp (it is never more than a step, decision 4),
  then one level lap at the end depth to cut the ramp's slope away. *(Changed 2026-10-15,
  docs/specs/profile-ramp-level.md: the ramp is at the angle over the skin's length, then
  level round, then only the ramped stretch is cut level.)*

## 4. Order (decision 5)

Within an operation (and within each copy of it): **all roughing laps first, then all
finishing laps, then all skin laps**, each stage in the operation's ring order (holes before
the outline). So every part of the operation stays held by its skin until the last stage,
and the walls are finished after the heavy cutting is done.

## 5. Feeds (decision 3)

The finishing and skin laps use the operation's cutting feed. (A separate finishing feed can
come later; decision 3.)

## 6. Checks and notes

- A skin thicker than the step down, or thick enough to reach the tabs' top, is refused
  (with tabs, the skin must be thinner than the tab height).
- A note when a finishing stock leaves the roughing ring unable to fit (a narrow part): the
  roughing ring follows the offset rules (narrow necks, verification) like every ring.
- The setup sheet lists them: "finish 0.3 mm, skin 0.2 mm".
- `chipload-check` runs as for every program.

## 7. Tests

- Unit: the three stages in order; roughing rings `stock` farther out; the finishing lap at
  the wall at end − skin; the skin lap at the end depth; no straight descent onto
  uncut material (every straight descent is at a point already cut to that depth, as in
  ramp-entry.md's test).
- With tabs: every lap leaves them; with ramped tabs, the skin lap follows their flanks.
- Boundaries: skin equal to the step down; skin plus tab height equal to the cut (refused);
  stock just under the tool radius; skin with copies (each copy's skins at its end).
- Golden: a new program with a finishing pass and an onion skin, with tabs; every existing
  golden unchanged.

## 8. Decisions

1. Finishing pass as **one level lap at the wall, at full depth** (down to the skin).
   **Recommended**: on a router the finishing lap is light enough at full depth, and a
   single lap leaves no step marks. Alternative: finish in the same depth steps as the
   roughing.
2. Finishing and skin laps **go down in the roughing slot** and step across onto the wall
   (or use the lead). **Recommended**: never a plunge onto the finished wall, and fine for
   tools that aren't centre-cutting (the core under the tool is already cleared).
3. The operation's cutting feed for the finishing and skin laps; a separate finishing feed
   later. **Recommended** (one field less now).
4. The skin at most one step down thick, and thinner than the tabs when both are used.
   **Recommended**.
5. Order within an operation: roughing, then finishing, then skins. **Recommended**.
   Alternative: each ring roughed, finished and skinned before the next (less travel, but
   parts come loose earlier).
6. Pockets get a finishing pass later, with its own spec (walls and floor). **Recommended**.
7. No version bump: two optional fields that default to today's behaviour. **Recommended**
   (as for leads and start points).
