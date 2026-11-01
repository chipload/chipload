# Spec: a smoother V-carve (depth wobble and kinks)

Status: **agreed and built 2026-11-01**, all decisions as recommended (the user: "build
as recommended").

As built:
- **The smoothing step is half the tolerance**, 0.005 mm of the cut's radius (0.009 mm of
  depth for a 60° bit), not the whole: the other half is already how far the centre line's
  radius may stray between its points (`set_cut`), and the V-carve's checks (no gouge,
  every part reached) keep passing.
- **Smoothing** (`smooth_radii`) finds the turning points that a swing of the step or more
  separates (a zigzag with that threshold); between two, the radius only rises or only
  falls, the least exact radius between the point and the turning point it heads for. So
  it is never wider than exact and less than the step narrower, and no reversal between
  turning points is smaller than the step. Only a walk's first or last move can turn back
  by less: when its start, a bump and a dip lie within a step, no profile within a step of
  the exact one avoids that. A property test covers it.
- **Kinks** (`Graph::prune_kinks`): the turn is read from how fast the cut's radius grows
  along the branch from its corner (sin(α/2) per mm). Only ends on the outline count (a
  stroke's round end isn't a corner). Exactly the angle keeps its branch.
- **And only a branch the rest covers** (`within_sliver`, the user the same day: a traced
  logo at 20° lost a leg of its "H"): the branch runs from its corner to the next fork,
  and where a stroke's whole centre line ends in a kink (a traced, kinked round end), the
  branch was the whole stroke. Now every point of the branch, its corner too, must lie
  within the cut of the axis that stays, but for the sliver r · (1/cos(θ/2) − 1) of the
  setting's angle at that cut's radius, plus the tolerance. Checked against the axis as it
  is at that moment, so two branches never count on each other. On the script example
  this also keeps the tapered ends of strokes (0.24 to 0.34 mm would have stayed uncut),
  which the first version took for kinks: 20° there now removes nothing, the smoothing
  does the work.
- **Measured** on the example of §1, reversals of Z within cuts (counted by the swing
  between them):

  | | Reversals | Swing under 0.01 mm | Under 0.1 mm | Time (estimate) |
  |---|---|---|---|---|
  | 0.30.0 | 326 | 182 | 223 | 49.9 s |
  | Smoothing, kinks 0° (old projects) | 172 | 2 | 50 | 49.9 s |
  | Kinks 20° (new V-carves) | 172 | 2 | 50 | 49.9 s |
  | Kinks 60° | 159 | 2 | 52 | 48.0 s |

  (The first version gave 138 at 20°, by also cutting off the strokes' tapered ends; see
  the coverage rule below.)

  The reversals left are the strokes' real changes of width and the corners cut out and
  back. (§1's "262 smaller than 0.01 mm" counted against the next point, not the swing.)
- **A traced picture** (the user, the same day: a pixelated ring "gets traced as a seemingly
  round path but the V-carve contains a multitude of up/down branches"): its kinks are the
  pixel steps, 20° to 45°. The golden ring (v19 image_vcarve, 3 passes):

  | Ignore kinks | Reversals | Cuts | Time |
  |---|---|---|---|
  | 0° (and 0.30.0) | 48 | 24 | 71.9 s |
  | 20° | 30 | 24 | 69.3 s |
  | 30° to 60° | 0 | 24 | 65.0 s |
  | 90° | 0 | 12 | 53.2 s |

  (With the first version, before the coverage rule, 45° already gave 12 cuts.) The guide
  advises 30° for traced pictures. A default by what the V-carve cuts is open, to decide
  after the user's tests.

The user (2026-11-01, after V-carve tests with Edwardian Script): "we must improve the
v-carve. Could we add options to the operation so the user can try different parameters,
right now it creates a lot of small up and down movement just because the path is not
entirely smooth. An idea is to prune voronoi branches if the angle is more than say 60°
between the segments." And: "a kink creates a voronoi branch".

## 1. What makes the bit go up and down

Measured on "Welcome Home" in Rochester (a formal script like Edwardian), 25 mm high, 60°
V-bit, one pass, with 0.30.0 (after docs/specs/vcarve-fewer-lifts.md): **326 reversals of
Z within cuts**. Each one is a stop and start of the Z axis, the slowest axis on most
routers.

1. **Depth wobble: 262 of the 326 are smaller than 0.01 mm.** The outline's curves reach
   the Voronoi diagram as short chords (ADR 0015), so the stroke's width along the centre
   line wobbles by hundredths of a millimetre, and the depth, which follows the width,
   with it. Nothing visible is carved by those wobbles.
2. **Kinks: most of the rest.** Every corner of the outline, however blunt, grows a branch
   of the centre line out to it, and since 0.30.0 a short branch is cut out and back: the
   bit rises to the corner and comes down again. The example has 38 such branches. Measured
   by how much the outline turns at their corner: 12 turn less than 20°, 13 less than 60°,
   29 less than 90°. A gentle kink in a script letter is one of those 12.

## 2. Smoothing the depth (always)

Along each cut, the depth is smoothed so that it **never goes up and down by less than a
small step**. Rules:

- **Never deeper than the exact depth** anywhere: a deeper V cuts wider, past the outline.
- **At most the step shallower**, where a wobble is flattened.
- **The step** is the depth that moves the cut's edge by the V-carve tolerance (0.01 mm,
  `VCARVE_TOL_MM`): 0.01 mm ÷ tan(half the bit's angle), 0.017 mm for a 60° bit. So the
  carving stays within the tolerance it is checked against now.
- Real changes of depth (a stroke getting wider) are kept: only reversals smaller than the
  step go.

No setting (decision 1): it is within the tolerance and only removes motion.

## 3. Ignoring kinks (a setting)

A new V-carve setting, **Ignore kinks**: an angle, 0° to 90°. A corner branch is not cut
where the outline turns by less than that angle at its corner. That is the user's idea,
measured on the outline itself (how much it bends) rather than between Voronoi segments,
which is what decides how much is lost:

- At a kink that turns by θ, in a stroke whose centre line is r from the edges, leaving
  the branch out leaves a sliver at most r · (1 / cos(θ/2) − 1) deep in the corner:

  | Turn | r = 1 mm | r = 3 mm |
  |---|---|---|
  | 10° | 0.004 mm | 0.011 mm |
  | 20° | 0.015 mm | 0.046 mm |
  | 45° | 0.082 mm | 0.25 mm |
  | 60° | 0.15 mm | 0.46 mm |
  | 90° | 0.41 mm | 1.2 mm |

  Up to 20° it is a few hundredths of a millimetre; a real corner (90° in a block
  letter) keeps its branch at any setting up to 90°.
- **Default for new V-carves: 20°** (decision 3). **Older projects keep 0°**, so they carve
  as before (a missing value is 0°: no schema change, CLAUDE.md "never silently change
  machining").
- In the window: the V-carve form, beside *Passes*, with a hover saying what it does. On
  the command line: `chipload set p.chipload operations.0.ignore_kinks_deg=30`, and `get`.
- The check that the cut reaches every part of the shape (v-carve.md rule 2) allows the
  slivers the setting leaves.

The user can try different angles and see the effect in the toolpath and the simulation
right away.

## 4. Tests

- **Smoothing:** no Z reversal smaller than the step within a cut; never deeper than the
  exact depth; at most the step shallower. On the example, the reversals counted again
  and given under "As built".
- **Kinks:** a rectangle with one corner bent by 15°: with 20° that corner has no branch,
  with 0° it has; a 90° corner keeps its branch at 90°. What is left uncut is no more than
  the table says.
- **Old projects:** a frozen project without the setting carves as before.
- **Golden G-code:** every V-carve changes (smoothed depths, at most 0.017 mm shallower);
  the diff reviewed; every program passes the checker.

## 5. Decisions

1. **Smooth the depth always, within the V-carve tolerance, no setting.** *Recommended:*
   it removes most of the reversals and carves nothing differently that can be seen.
   Alternative: a setting for the step.
2. **One setting, *Ignore kinks* (an angle), measured as the outline's turn at the
   corner.** *Recommended:* that angle decides what is lost (the table). Alternatives:
   the angle between Voronoi segments (the user's first words; it is the same corner seen
   from the centre line, but harder to explain), or a length in mm (how much a corner may
   be left).
3. **Default 20° for new V-carves, 0° for old projects.** *Recommended:* at 20° at most a
   few hundredths of a millimetre stay in a kink. Alternative: 0° for new ones too, so the
   user turns it on.
4. **More settings to try?** The two lengths of 0.30.0 (corners out and back under 5 mm,
   pieces joined within 10 mm) could become settings too. *Recommended: not now*; add
   them if trying the angle shows they matter.
