# Spec: helical and ramp entry

Status: **draft, awaiting review** (decisions in §9).

User request (2026-10-10): "Spiral plunge for tools that are not end-cutting." Today every
operation plunges straight down. A tool whose end teeth don't reach the centre (not
centre-cutting) can't do that: the uncut core under its centre rubs, burns or breaks the
tool. Pockets refuse such a tool. Profiles don't check at all; they should (§6).

## 1. What changes

The tool goes down into material **while moving sideways**, so the side teeth do the cutting:

- **Helix** (pockets): the tool circles round a point and goes down a little each turn,
  cutting a round hole. Then it clears the pocket as before.
- **Ramp** (profiles, and pockets where a helix doesn't fit): the tool goes down gradually
  *along its own path*, then makes the pass at full depth.

Where the cut goes doesn't change, only how the tool gets down. So the rings, walls,
verification and uncut-material checks stay as they are.

## 2. Format (no version bump)

An optional `entry` per operation; missing means `{"type": "plunge"}`, today's behaviour:

```json
"entry": {"type": "helix", "radius": 1.5, "ramp_angle": 0.0524}
"entry": {"type": "ramp", "ramp_angle": 0.0524}
"entry": {"type": "plunge"}
```

- `ramp_angle`: how steeply the tool descends, in radians in the file and degrees in the
  GUI and CLI. Rule: 0 < angle ≤ 30°.
- `radius` (helix): the radius of the circle the tool *centre* follows, mm. Rule: greater
  than 0 and less than the tool radius, so no core is left standing in the middle.
- Pockets take `helix`, `ramp` or `plunge`; profiles take `ramp` or `plunge`.
- Defaults when switched on (decision 2): helix radius 0.25 × D, ramp angle 3°. Like the
  strategy settings, they are written explicitly.

## 3. Helix (pockets)

1. **Where:** at the piece's point furthest from its walls (as adaptive already finds), so
   the largest helix fits. For adaptive pockets this is the existing plunge point.
2. **Fit:** the helix circle must lie in the allowed area (the pocket shrunk by the tool
   radius, minus clearance), checked against the walls like any move. If `radius` doesn't
   fit, the largest radius that fits is used, down to a minimum of 0.1 × D (a note says
   so). Below that, the piece ramps instead (§4), with a note.
3. **Descent:** feed straight down to the depth already cleared there (the previous pass,
   or the start depth), which only goes through air or cleared material. Then go round the
   helix, dropping `2π · radius · tan(ramp_angle)` per turn, down to the pass depth. Then
   make one full circle at that depth to flatten the ramped floor.
4. **Then** move at depth to the first ring's start. This is a cutting move, checked
   against the walls like the existing links. Then the pass runs as today.
5. Each depth pass of each piece starts with its own helix.

## 4. Ramp (profiles, and pockets as a fallback)

1. From the start of the path, at the depth already cleared there, go along the path while
   descending at `ramp_angle` until the pass depth is reached. The angle is never
   exceeded, so a closed path shorter than the ramp is followed round several times.
2. Then the full pass at depth from where the ramp ended, ending where the ramp started, so
   the whole path is cut at full depth (the ramped part included).
3. The ramp follows the cut path exactly, so it can't gouge: it stays on the checked path.

## 5. The other plunges in a program

Not every straight plunge is a problem. A plunge only matters when the material under the
tool centre at that depth isn't already cut.
- **Pocket re-plunges** between rings (when the tool lifts instead of staying down):
  - stay straight when the point under the tool centre is already cleared at that depth by
    an earlier path in the same pass;
  - otherwise they ramp along the ring.

  The planner proves the "already cleared" case: the plunge point is within r − ε of an
  earlier path at that depth.
- **Tabs:** after lifting over a tab, the tool goes back down into the cut. With `ramp`
  entry, that descent ramps along the path too, at the same angle.
- **Plunges into air** (above the cleared depth) stay straight.

## 6. Tool rules

| Entry | Centre-cutting tool | Not centre-cutting | Unknown |
|---|---|---|---|
| plunge, pocket | as today | error (as today) | note (as today) |
| plunge, profile | as today | **new: error**, suggests ramp (decision 5) | **new: note** |
| helix / ramp | fine | fine | fine |

## 7. Output and checking

- The helix and ramp moves are chords like every other move. Helix chords are tagged with
  their circle, so the post writes them as **helical G2/G3**: an arc with a Z end, at most
  180° per move, as for flat arcs (ADR 0013). Ramps along straight parts are G1 with Z;
  ramps along arcs are helical arcs as well.
- **The independent checker learns helical arcs.** It will accept G2/G3 with a Z change
  when:
  - the XY radius agrees at both ends, as now;
  - Z changes linearly with the angle;
  - Z doesn't change direction within one move;
  - the stock, travel and depth limits hold over the whole arc, including the lowest Z.

  A rapid into material is still refused, as before.
- **Feed** (decision 4): helix and ramp moves use the operation's **cutting feed**. At 3°
  the tool goes down at about 5 % of that feed, far below the plunge feed. The straight
  descents through air and cleared material keep the plunge feed.
- Verification of every move against the walls is unchanged. The ramp's Z changes nothing
  in XY.

## 8. Tests

- Unit:
  - helix turns, drop per turn, final flat circle;
  - a helix shrinks to fit, and falls back to a ramp below 0.1 × D;
  - ramp length = Δz / tan(angle), over several laps on a short ring;
  - a ramp ends with the full lap at depth.
- Boundaries:
  - step down equal to the stock thickness;
  - a start depth of 0 and greater than 0;
  - a pocket just wide enough for the minimum helix;
  - tabs with a ramp after each tab.
- Re-plunges: the cleared-point proof; a stepover > 0.5 forces a ramp.
- Tool rules in §6, including the new profile error and note.
- Checker:
  - helical arcs accepted;
  - wrong radius, Z not linear, or below the depth limit: refused;
  - flat arcs exactly as before.
- Golden: every existing file unchanged (all use `plunge`); new golden programs with a
  helix pocket and a ramped profile with tabs; both pass the checker.
- Property: random pockets with helix entry: no gouge, no uncut warning, checker passes.

## 9. Decisions to review

1. **Helix for pockets, ramp along the path for profiles** (and for pockets where no helix
   fits) (§3, §4). Recommended: a helix needs room beside the path, which a profile
   doesn't have, while a ramp along the path can't hit anything new.
2. **Defaults:** helix radius 0.25 × D, ramp angle 3°, minimum helix 0.1 × D; the user can
   change them per operation. Recommended for wood and plastics with small endmills.
   Please say if you usually use a different angle.
3. **Which plunges are replaced** (§5): the entry at each depth pass, re-plunges whose
   centre isn't proven to be over cleared material, and descents after tabs. Plunges
   through air or cleared material stay straight. Recommended.
4. **Ramp feed = cutting feed** (§7), rather than the plunge feed or a new field.
   Recommended. It keeps the settings simple and the Z speed low; a separate ramp feed can
   come later.
5. **Profiles with a straight plunge and a tool marked not centre-cutting become an error**
   (§6), as pockets already are. The message points to ramp entry. Recommended for safety,
   but it changes existing behaviour: such a project stops generating until ramp is
   switched on. The alternative is a warning only.
6. **New operations** get `ramp`/`helix` switched on automatically when the tool is marked
   not centre-cutting, and `plunge` otherwise. Recommended. It's visible in the form and
   saved explicitly, so nothing is hidden.
7. **Lead-in/out** comes next, as its own spec. It will move the entry point into the waste
   with an arc onto the path. The ramp then runs on the lead-in, so this spec keeps entry
   (going down) separate from lead (going sideways onto the path).
