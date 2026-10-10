# Spec: helical entry, lead-in/out and start points

Status: **approved 2026-10-10**, all decisions as recommended (§7). Implemented (A, B and C).

User requests (2026-10-10):
- "Spiral plunge for tools that are not end-cutting." The ramp along the path is done
  (docs/specs/ramp-entry.md); the helix was postponed there and is now resumed.
- "Lead-in/lead-out for pockets and profiles: plunge in the waste, arc in tangentially to
  avoid plunge marks. The lead position should be draggable by the user (an algorithm
  can't always choose well)."

Three parts, built and committed in this order: **A** start points, **B** lead-in/out,
**C** helical entry. A comes first because B needs it, and it is useful on its own (it
moves the plunge mark of a plain plunge too).

## A. Start points

Today every ring starts at its lowest (then leftmost) vertex, and the user can't change it.

### Format (no version bump)

An optional list per profile and pocket, in drawing coordinates (mm). Missing or empty
means today's start everywhere:

```json
"starts": [[120.0, 35.5], [40.0, 12.0]]
```

### Meaning

- Each stored point belongs to the **ring nearest to it** (a profile's tool-centre ring, or
  a pocket piece's wall ring). That ring starts at the point of it nearest to the stored
  point (projected onto the ring, between vertices if need be).
- A ring with no point keeps today's start. Rings are still cut in today's order.
- A point that lies more than one tool diameter from its ring gives a note ("start point
  (120.0, 35.5) is 14.2 mm from contour #3; the nearest point is used"). It happens when the
  drawing changes under it.
- Two points on the same ring: the first in the list wins, with a note.
- **Copies:** each copy uses the points shifted with it.
- **Tabs** are spaced from the start, as today, so moving the start moves the tabs with it.
  The start is never inside a tab.
- **Pockets:** the start point moves only the wall ring at each depth. The clearing inside
  (rings, trochoidal loops, adaptive) is unchanged.

### GUI

- With an operation selected, each of its rings shows a **start marker** (a dot with an
  arrow in the cutting direction). Dragging it moves the start along the ring (it follows
  the ring, it can't leave it). Releasing stores the point. It works like the shape handles:
  the toolpath is regenerated, and Undo works.
- Right-click on a marker: *Reset start* (removes the point).

### Command line

The project file is the interface for operation settings, as for tabs and entry: `starts`
is edited in the file; `plan` and `plan --json` show each ring's start.

## B. Lead-in and lead-out

### What it does

Instead of going down on the path, the tool goes down **in the waste**, beside the start,
arcs onto the path tangentially, cuts the ring, and arcs off it again at the end:

```
                waste
          lead-in  .--.  lead-out
                  /    \
   ----->--------*------*------>---  ring (tool centre)
                start
                part
```

- Both are **quarter circles** of radius `radius`, tangent to the ring at the start
  (decision 2). The lead-in begins in the waste and ends on the start going the cutting
  way; the lead-out leaves the start going the same way and ends in the waste.
- The waste side is: outside an outside profile, inside an inside profile, and for a pocket
  the cleared inside of the pocket (the lead onto the wall ring runs in cleared material).
- Profiles cut **on** the line have no waste side: a lead there is refused with a message
  (decision 4).

### Format (no version bump)

An optional `lead` per profile and pocket. Missing means none, today's behaviour:

```json
"lead": {"radius": 3.0}
```

- `radius`: mm, more than 0 and at most 10 × the tool diameter. Default when switched on:
  the tool radius (decision 2). Written explicitly.

### Where a ring with a lead starts

A ring that has a lead but no start point of its own starts in the **middle of its longest
chord** (the first, on a tie), not at today's corner. Beside an inside corner a lead has no
room: its start would come within the tool radius of the other wall. Only rings with a lead
start there, so nothing changes for existing projects. (Found while implementing,
2026-10-10.)

### Each pass

For each depth pass of a ring:

1. Retract to safe Z, rapid to the lead-in's start, go down there (the entry, see below).
2. Lead-in arc, the ring, lead-out arc.
3. Retract to safe Z (decision 3).

So every pass starts and ends in the waste. The descent at the lead-in's start passes only
air or material already cut: the lead-in of the previous pass cut it to the previous
depth.

- **Plunge entry:** straight down at the lead-in's start.
- **Ramp entry:** the ramp runs out and back along the lead-in arc and on along the ring, as
  it does on the ring today. It starts in the waste and ends at the lead-in's start at the
  pass depth.
- **Helix entry** (pockets, part C): the helix opens the pocket as part C says; the wall
  ring's lead-in then starts in cleared material, so it needs no entry of its own.
- **Tabs:** the lead-out begins after the last tab, as the ring's end is past it already.

### Safety

- The lead arcs are checked like every cutting move: every point of them keeps at least the
  tool radius (minus tolerance) from **every contour in the drawing**, not only its own,
  and lies on the waste side. So a lead can't cut into the part, a neighbouring part, or an
  island, and copies count the leads in their size (the overlap check includes them).
- If the arcs don't fit (a small hole, a narrow gap to a neighbour), the radius is halved
  until they fit, down to 0.1 × the tool diameter, with a note. Below that the ring goes
  without a lead, with a note (decision 5). Nothing is ever cut outside the checks.
- Lead moves use the cutting feed. They are arcs, written as G2/G3 like the rings.

## C. Helical entry (pockets)

The first draft of ramp-entry.md (git 2c372f7) designed it; this is that design, updated.

### Format (no version bump)

A new kind of `entry`, for pockets only:

```json
"entry": {"type": "helix", "radius": 1.5, "ramp_angle": 0.0524}
```

- `radius`: of the circle the tool **centre** follows, mm. More than 0 and less than the
  tool radius, so no core is left standing in the middle.
- `ramp_angle`: as for the ramp, 0 < angle ≤ 30°.
- Defaults when switched on: radius 0.25 × D, angle 3° (decision 6). New pocket operations
  with a tool that isn't centre-cutting get a helix (instead of today's ramp).
- Older Chipload versions refuse an unknown entry type, which is the safe outcome.

### The helix

1. **Where:** at each depth pass of each piece, at the point furthest from the walls (as
   adaptive already finds). For adaptive pockets that is the existing plunge point.
2. **Fit:** the helix circle must lie in the allowed area (the pocket shrunk by the tool
   radius minus the clearance), checked like any move. If it doesn't fit, the largest
   radius that fits is used, down to 0.1 × D, with a note. Below that the piece ramps along
   its first path instead (today's ramp), with a note.
3. **Down:** straight to the depth already cut there (plunge feed, only air or cut
   material), then round the helix at the cutting feed, dropping `2π · radius ·
   tan(angle)` per turn, to the pass depth. Then one flat circle at the pass depth, so the
   floor is level.
4. **Then:** feed at depth to the first path's start (checked like the existing links) and
   clear the pass as today.
5. Other descents within a pass (re-plunges between rings, after a link that lifts) keep
   ramping along the ring, at the helix's angle.

### Output and checking

- Helix chords are tagged with their circle, so the post writes them as **helical G2/G3**:
  an arc with a Z end, at most 180° per move, like flat arcs (ADR 0013). With arcs off
  (`expected/lines/`) they stay G1 X Y Z chords.
- **The independent checker learns helical arcs.** It accepts a G2/G3 that changes Z when
  the XY radius agrees at both ends (as now) and the move goes down (or up) only: the lowest
  Z is then its end or start, which the depth and stock checks already see. Today such a
  move is refused ("Arc that changes Z (helical) is not supported"); that message goes.

## 5. Unchanged

- Every existing program is byte-identical: no existing job has `starts`, `lead` or a helix.
  The golden test proves it.
- The ring order, the offsets, the clearing strategies, the wall checks and the
  uncut-material checks are unchanged. Leads and helices add moves in material that is
  waste or already cleared, and are checked themselves.

## 6. Tests

- **Start points:** the ring starts at the projection of the point; the nearest ring takes
  it; a far point and a duplicate give notes; copies shift it; tabs move with it; a pocket
  only moves its wall ring.
- **Leads:**
  - quarter arcs tangent at the start, on the waste side, for inside and outside, climb and
    conventional, both spindle directions;
  - `on` with a lead is refused;
  - a hole too small: the radius shrinks (boundary: exactly fits), then no lead with a note;
  - a neighbouring contour closer than the lead: shrinks;
  - multi-pass: retract between passes, every descent only through cut material;
  - ramp entry on the lead; tabs with leads;
  - copies: the overlap check includes the leads.
- **Helix:** turns, drop per turn, the flat circle; shrinks to fit; falls back to a ramp below
  0.1 × D; a start depth > 0; step down equal to the stock thickness; every strategy.
- **Checker:** helical arcs accepted; wrong radius, or Z out of the limits, refused; flat arcs
  as before.
- **Property:** random pockets and profiles with a lead and a helix: no gouge, checker
  passes.
- **Golden:** unchanged, plus `v7_leads` (a profile with tabs and a moved start, and a
  pocket with a lead and a helix), which passes the checker, with arcs and without.
- **GUI:** the start marker shows, dragging it writes `starts`, *Reset start* removes it;
  the lead and helix fields write the operation.

## 7. Decisions to review

1. **Start points are stored as points (`starts`), each taking the nearest ring.**
   Recommended: a point survives a re-exported drawing (contour numbers and shape ids
   change; the place doesn't), works for layer selections and copies, and needs no new id
   scheme. Alternative: per shape id plus a distance along the contour, which breaks when
   the shape is re-imported.
2. **Lead shape: a quarter circle in and out, radius = the tool radius by default.**
   Recommended: the simplest tangent entry, and short. Alternatives: a straight line plus
   an arc, or a half circle; a lead-out overlap (cutting a little past the start before
   leaving, to remove the last trace of the entry) could come later.
3. **Retract to safe Z between depth passes when leads are on.** Recommended: the lead-out
   ends in the waste, beside where the next lead-in starts, and going back at depth would
   cut new material that has to be proven safe. A retract and a rapid cost a few seconds
   per pass. Alternative: feed back along the lead-out and lead-in at the depth just cut
   (no retract, and only cut material, but more code).
4. **No lead for profiles cut on the line** (`side: on`): refused with a message.
   Recommended: there is no waste side to put it on. Alternative: let the user pick a side.
5. **A lead that doesn't fit shrinks, then is left out with a note**, instead of an error.
   Recommended: either is safe (nothing is cut unchecked), and a note keeps the project
   working where one small hole has no room. Alternative: an error, so the user decides.
6. **Helix defaults: radius 0.25 × D, 3°; minimum radius 0.1 × D; new pocket operations
   with a tool that can't plunge get a helix.** Recommended. Please say if you usually use
   other values.
7. **Order: A (start points), then B (leads), then C (helix)**, one commit each, released
   together or one by one as you like. Recommended: B depends on A. C is independent and
   could go first if you want it sooner.
