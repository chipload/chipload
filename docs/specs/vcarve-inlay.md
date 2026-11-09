# Spec: V-carve inlays

Status: **approved 2026-11-09**, all decisions as recommended (the user: "Go ahead").
Branch `signs`; built after the mirrored part (docs/specs/mirror-part.md).

Builds on docs/specs/v-carve.md (the medial axis, depth(p)), docs/specs/vcarve-wide-strokes.md
(walls and the floor pass), docs/specs/setups.md (a second board with its own program) and
docs/specs/mirror-part.md (the plug's part is mirrored).

## 1. Why

TODO item 7 (the gap review of 2026-11-03), taken up for sign making (the user, 2026-11-09).
An inlay sets one wood into another: a dark word flush in a light board, without paint. The
artwork is V-carved twice:

- a **pocket** in the base board, and
- a **plug** in a second board, mirrored, which is glued face down into the pocket. When the
  glue has set, the plug's back is sawn or planed off down to the base's surface.

The V-bit makes this work: the pocket's walls slope at the bit's angle, and the plug's walls
slope the same way, so the two wedge together and centre themselves. Sharp inside corners fit,
which an end mill inlay can't do. No registration (dowels, corners) is needed: the walls
find the place.

Other programs call this a V-carve inlay, too. This spec is built from the geometry below
alone (first principles, the medial axis of v-carve.md).

## 2. What "correct" means

Let S be the artwork: the regions of the selected closed shapes (an outer contour minus the
holes in it, as for any V-carve). The V-bit has included angle α and tip width w; let
t = tan(α/2). Write E(S, d) for S **shrunk** by d (the points at least d inside its outline)
and G(S, d) for S **grown** by d.

### 2.1 The pocket

An ordinary V-carve of S, `end_depth` P (the **pocket depth**), with walls and a floor pass
where strokes are wide (vcarve-wide-strokes.md). Its hole at depth z below the base's
surface is, for 0 ≤ z ≤ P:

    pocket(z) = E(S, z·t)

That is the property v-carve.md §1 already promises (the cone touches the outline at the
surface, and is nowhere wider than the region), restated at every depth. In a stroke too
narrow for depth z the shrunk region is empty there: the pocket is shallower, a V.

### 2.2 The plug

The plug board carries S′, S mirrored left to right: the plug's part is mirrored (§5). Its carving is a V-carve of the
**rest of the board**, C = (the board grown by the bit's radius) minus S′, with one change:
the outline is taken **D below the surface** (the **plug start depth**), not at it. So the tip
at an axis point p of C goes to

    depth(p) = D + (r(p) − w/2) / t

capped at the **channel depth** D + g (g, the **glue gap**), with walls and a floor pass
where wider (vcarve-wide-strokes.md §3, §4, measured from the outline at depth D).

Above the outline level the cone is wider than its disc, so it cuts into S′ on purpose: that
is the plug's taper. What stands of the plug at depth y below the plug board's surface:

    plug(y) = E(S′, (D − y)·t)     for 0 ≤ y ≤ D      (the taper, narrowing to the face)
    plug(y) = G(S′, (y − D)·t)     for D ≤ y ≤ D + g  (the foot, widening to the channel floor)

### 2.3 The fit

Turn the plug over and push it in until the outline level meets the base's surface. A point
at depth y in the plug lies at depth z = D − y in the pocket, and

    plug(D − z) = E(S, z·t) = pocket(z)      for 0 ≤ z ≤ D

(mirrored back, S′ is S). The walls meet at every depth, including narrow strokes, sharp
inside corners, and the islands inside letters (the counter of an O is part of C: carved in
the plug, left standing in the base). Then:

- the plug's face lies D deep in a pocket P deep: **P − D of space under it** for glue, so it
  seats on its walls, not on the pocket's floor;
- the plug's back (the channel floor) stands **g above the base's surface**: room for glue
  squeeze-out, and for the plug to sink a little further if the wood gives;
- nothing of the plug board stands proud outside the channel (§2.2: C reaches past the
  board's edges by the bit's radius, so no lip is left at the edges).

These are what the tests check (§9): pocket and turned plug, as height maps, touch along the
walls within `VCARVE_TOL_MM`, overlap nowhere by more than it, and leave the two gaps.

### 2.4 What doesn't fit exactly

- **A flat tip** (w > 0) can't reach the last bit of a sharp corner (v-carve.md §1): the
  pocket leaves a sliver in its corners, the plug leaves a sliver on its points, within
  w/2 ÷ sin(θ/2) of a corner of angle θ. They don't meet: a point of the plug is a little
  too full where the pocket is a little too small. With a 0.2 mm tip and a right angle,
  0.14 mm. Decision 6.
- **Ignore kinks** (vcarve-smooth.md) leaves the corners it skips uncarved, differently in
  the pocket and the plug. Decision 6.
- **A depth cap by the bit:** a V-bit can't cut deeper than (diameter − w) / 2 ÷ t. The
  pocket depth and the channel depth must both fit (refused otherwise, as today).

## 3. The operations (decision 1)

Two operations, each an ordinary V-carve:

- **the pocket:** a V-carve in the base board's setup, nothing new;
- **the plug:** a V-carve with a `plug` setting, in a setup of its own (the plug board).

They are not linked in the file. The fit depends only on the artwork, the bit's angle and the
two depths (§2.3); feeds, passes and floor passes don't matter to it. Both select the same
shapes (the same `select`), so editing the artwork changes both. A check pairs them (§6).

```json
{ "type": "vcarve", "name": "Inlay plug", "part": 3, "select": { "layer": "WORD" },
  "tool": 3, "start_depth": 0.0, "end_depth": 4.5, "passes": 2,
  "feed": 1200, "plunge_feed": 400,
  "plug": { "start_depth": 3.0 },
  "floor": { "tool": 2, ... } }
```

- `plug.start_depth`: D, mm, more than the operation's `start_depth` (normally 0) and less
  than its `end_depth`. Its `end_depth` is the channel depth D + g; the form shows the
  **glue gap** g = end − D, and that is what the user sets.
- Missing `plug`: an ordinary V-carve. Optional field, no schema bump: older Chipload refuses
  an operation with an unknown field and reads every other file as before (as for `floor`).
- A plug has no `ignore_kinks_deg` (0, decision 6).

## 4. Making an inlay

One command makes the pair from the selection (decision 2):

    chipload op add-inlay sign.chipload --layer WORD --tool 3 --depth 5 \
        --plug-start 3 --gap 1.5 [--floor-tool 2] [--plug-thickness 10]

- the pocket: a V-carve of the selection in the active part, `end_depth` = the pocket depth;
- a new setup "Inlay plug" (name translated), with a part of its own: its board fits the
  artwork (the stock fit, `StockOrigin::Drawing`) with a margin of 10 mm, `--plug-thickness`
  thick, work zero as the base's; the part mirrored left to right; and in it the plug;
- with `--floor-tool`, both get a floor pass with that end mill (the plug's channel is mostly
  floor, so it matters there most);
- one undo step in the window.

In the window: **Inlay…** next to V-carve, for the selection. A small form asks for the V-bit,
the pocket depth, plug start depth, glue gap, floor end mill and plug board thickness, with
the defaults of decision 5, and a drawing of the two cuts with the depths marked. Afterwards
both are ordinary operations with their own forms; the plug's form has a *Plug* section
(start depth, glue gap).

## 5. Mirroring (decision 4)

The plug's part is a **mirrored part**, left to right (docs/specs/mirror-part.md): cut
mirrored about the vertical line through the middle of its box, the drawing untouched. The
plug's carving (§2.2) is planned on S as drawn, and the part's mirror makes it S′. Mirrored
there, the artwork keeps its bounding box, so the plug board's fit and its zero work
unchanged. The preview and the simulation show the plug as cut.

The `plug` setting itself mirrors nothing. A plug in a part that isn't mirrored gets a
warning (§6).

## 6. Checks and warnings

On the plug, as warnings (none makes a program unsafe; the checker keeps its own rules):

- **No pocket:** no enabled V-carve in another setup selects the same shapes with a bit of the
  same angle: "Inlay plug: no pocket for it (a V-carve of the same shapes with a 60° bit)."
- **Won't seat:** the pocket's depth P is not more than D: the plug's face would hit the
  pocket floor before the walls meet.
- **Not mirrored:** its part has no mirror: the plug would go in the wrong way round.
- **Glue gap small:** g < 0.5 mm: the plug's back may meet the base before the walls do.
- **Thin back:** the plug board leaves less than 2 mm under the channel floor.
- **Kinks ignored:** its pocket has `ignore_kinks_deg` > 0 (§2.4).
- **Flat tip** (decision 6): a note saying how far from sharp corners the fit is short.

The no-gouge rule of v-carve.md §1, which every V-carve move is verified against, is measured
for a plug at the outline level D instead of the surface: there the cone stays outside S′.
Full reach likewise. Cutting past the board's edges by up to the bit's radius stays within
what the checker allows (tool centre within a tool radius of the stock).

## 7. Setup sheet and guide

- The plug's sheet says it is mirrored, and adds the steps after it is cut: glue it face down
  into the pocket of setup *N*, press or clamp until it seats, let it set, saw or plane the
  back off to the surface.
- The guide's V-carve page gains *Inlays*: the picture of §2.3, the four numbers, the steps.

## 8. Steps

1. Model (`plug`), the plug's carving (§2.2) and its verification, `op add-inlay`, the checks
   of §6, tests (§9), a golden pair.
2. The window: *Inlay…*, the *Plug* section, the mirrored preview.
3. The sheet's steps, the guide page.

## 9. Tests

- **Fit, as height maps:** for a word with serifs, an O, a narrow stroke and a wide one: the
  simulation's height map of the pocket and of the plug, turned over and mirrored back, put
  together with the outline level at the surface. Along the walls they touch within
  `VCARVE_TOL_MM`; nowhere do they overlap by more; under the plug's face P − D; the back
  g above the surface.
- **Property test** (proptest): random polygons, angles and depths, the fit as above.
- **The taper:** the plug's moves keep the cone outside S′ at depth D, and reach it.
- **Edges:** nothing of the plug board stands above the channel floor outside S′.
- **Boundaries:** D = start (refused), D + g = the bit's deepest (allowed), one more (refused),
  P = D (warning), a stroke narrower than the tip, a plug board too thin (refused, as any
  cut through the board without `through`).
- **Golden:** the pair as a project (two setups), its two programs and sheets.
- **CLI:** `op add-inlay` makes both, the setup, the floor passes; the checks of §6.

## 10. Out of scope

- End mill inlays (straight walls, rounded corners).
- A sideways clearance on the plug: the vertical slack (P − D and g) does that job.
- Several plugs nested on one board, beyond what the artwork's own layout gives.
- Inlays whose artwork isn't closed shapes (centre lines).

## 11. Decisions

1. **Two V-carves, not linked** (§3): the plug is a V-carve with a `plug` setting in its own
   setup; the pair is matched by selection and angle. *Recommended.* Alternatives: one
   operation holding both (an operation belongs to one part and setup, so it would break
   that rule); or a link from plug to pocket (operations have no ids, and an index breaks
   when operations are reordered).
2. **One command makes the pair** (§4), with a setup for the plug board. *Recommended.*
3. **The plug board is cleared to its edges** (§2.2), the board fitted round the artwork with
   a 10 mm margin, so nothing stands proud and nothing has to be sawn out before gluing.
   *Recommended.* Alternative: clear only within a margin of the artwork and saw the plug
   out along it (less cutting, one more step by hand, a lip to get wrong).
4. **Mirrored by its part** (§5, docs/specs/mirror-part.md), left to right about the
   artwork's centre, no copied shapes. *Recommended.* Alternative: a mirrored copy in the
   drawing (edits to the artwork wouldn't reach it).
5. **Defaults:** pocket depth 5 mm, plug start depth 3 mm (2 mm under the face), glue gap
   1.5 mm, plug board 10 mm thick (5.5 mm of back left), floor end mill the project's
   smallest. *Recommended*, from general practice in hobby inlay work; the user may know
   better numbers from the shop.
6. **Flat tips and kinks:** a flat-tipped V-bit is allowed with a note (the shortfall is a
   tenth of a millimetre or two at corners, which glue and the press take up); a plug never
   ignores kinks, the pocket made by `add-inlay` doesn't either, and a pocket that does is
   warned about. *Recommended.* Alternative: refuse flat tips for inlays.
7. **Steps as §8**, CLI first, after the mirrored part. *Recommended.*
