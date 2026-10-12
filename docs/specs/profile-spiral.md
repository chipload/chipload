# Spec: spiral profiles and ramped tabs

Status: **draft 2026-10-12**, for review. Decisions 1, 3–6 approved by the user as
recommended. The user replaced decision 2: "we need to do ramped tabs, it is not good to go
back and forth after each tab. We have no external users yet so we don't need to worry
about changing this." Decisions 7–11 are new and need review.

User report (2026-10-12): "the ramp function on profile goes back and forth, I think it
probably should just go forward. Also, the lead in/lead out should only be active on the
first and last pass."

## 1. Today

- **Ramp** (docs/specs/ramp-entry.md §3): at the start of each pass, and after each tab, the
  tool ramps forward along the ring and then back, so it reaches the pass depth exactly
  where a plunge would have. That kept everything after the entry unchanged, at the cost of
  the back-and-forth.
- **Tabs** (docs/specs/tabs.md): passes below the tab top lift straight up before each tab,
  cross it at the tab top, and go back down after it (a plunge, or the back-and-forth ramp).
- **Leads** (docs/specs/helix-and-leads.md part B): every pass retracts, goes down in the
  waste, leads in, cuts the lap, leads out and retracts again.

## 2. The idea

A profile with a ramp entry is **one forward spiral** down the ring. Where there is a tab,
the tool follows the tab's surface instead of going below it:

```
z(s) = max(spiral(s), tab surface(s))
```

`s` is the distance travelled along the ring, which only ever grows. The tool never goes
back and never lifts off between passes. The tabs become trapezoids: flat at the tab
height over the set width, with sloping flanks on both sides.

## 3. The spiral (ramp entry)

- **Down to the start:** rapid to the ring's start (or the lead-in's, §5), straight down to
  the start depth (only air or material already cut there, by definition of the start
  depth).
- **The spiral.** The passes stay as they are (`pass_depths`: same depths, same step-down).
  Each pass becomes `m` whole laps that descend evenly with `s`, from the previous pass's
  depth to its own. `m` is the smallest whole number that keeps the slope within the ramp
  angle: `m = ceil(step / (lap · tan(angle)))`. So each pass reaches its depth exactly at
  the ring's start, and under the tool there is never more than one step-down of uncut
  material.
- **The flat lap.** After the last spiral lap, one lap at the last depth (still following
  the tab surfaces) cuts away the spiral's slope, so the floor is level.
- **Rise out** (decision 7). After the flat lap the tool continues forward along the ring,
  which is cut at every depth already, rising evenly to the start depth over one tool
  diameter of path, never below a tab surface. Then it retracts. It doesn't stop against
  the wall at the bottom, and never moves into uncut material.
- Ramp moves use the cutting feed, as today. A ring needing more than 1000 laps for one
  pass is refused, with the message the ramp gives today.

## 4. Ramped tabs (decisions 2, 8–11)

### Shape

- The set **width** stays the width of the flat top, at the tab height, centred where the
  tab is placed. Placement (count, positions, start points) is unchanged.
- **Flanks** slope down from both ends of the top at the **flank angle**, to the end depth.
  So the tab's foot is wider than its top by `2 · tab height / tan(flank angle)` along the
  ring (decision 8).

| Flank angle | Foot, tab height 3 mm, each side |
|---|---|
| 90° | 0 (today's straight tabs) |
| 45° | 3 mm |
| 30° | 5.2 mm |
| 10° | 17 mm |
| 3° | 57 mm |

- Where neighbouring tabs' feet meet, the surfaces simply join (the higher one counts), with
  a note: "tabs N and M join at the bottom". A tab foot covering the ring's start is fine.

### Cutting them

- Every lap, of the spiral or of today's passes, follows `max(lap depth, tab surface)`. On
  the way up the tool cuts with its side as it rises; on the way down a flank it cuts at
  most one step-down of material (the previous lap cut the flank above), at the flank angle,
  which makes it a ramp.
- Points where the slope changes split the chords, as tab ends do today.

### Flank angle (decisions 9 and 10)

- A new `flank_angle` in the tabs: radians in the file, degrees in the GUI,
  0 < angle ≤ 90°. **90° means straight tabs**, as today: up and down at the plunge feed.
- Tool rules, as for ramp entry: a flank's way down is a ramp, so with a tool that isn't
  centre-cutting the flank angle may be **no steeper than the operation's ramp angle**
  (otherwise an error that names both angles); with an unknown tool, a note.
- Defaults for new operations: 90° with a plunge entry; 30° with a ramp entry and a
  centre-cutting or unknown tool; the operation's ramp angle with a tool that isn't
  centre-cutting. The form shows the foot length next to the angle.
- **Plunge entry** profiles keep separate passes and today's retracts, but their laps also
  follow sloped tabs when the flank angle is under 90°.

### Checks

- The tab verification checks that no cutting move goes below the trapezoid (today: below
  the tab height over its width).
- The 3D preview shows the trapezoid; the top view still shows the top's span.

## 5. Leads (decisions 3, 4 and 7)

- **Ramp entry with a lead:** rapid to the lead-in's start, straight down to the start
  depth, the lead-in arc at that depth, then the spiral. No leads and no retracts on the
  passes in between.
- **At the end, no lead-out arc**: the rise-out replaces it (decision 7). A lead-out at the
  last depth would swing the tool into waste that is uncut over the whole depth: today each
  pass's lead-out only cuts one step-down because the previous pass's lead-out cut the
  material above it.
- The lead is still fitted and checked as today, and decides where the ring starts.
- **Plunge entry with a lead:** unchanged, leads on every pass (decision 4).

## 6. Output and checking

- Spiral laps, flanks and the rise-out are G1 X Y Z, or on arc chords that descend evenly
  **helical G2/G3**: `arcs.rs` already merges such chords (helix-and-leads.md part C). With
  arcs off (`expected/lines/`) they stay G1 chords.
- The independent checker already accepts helical arcs; no change. Every program must pass
  it, as always.
- Cycle time, the setup sheet (tabs: "6 × 8 mm wide, 3 mm high, flanks 30°"),
  `plan --json` and the preview follow from the moves and the job.

## 7. Format: schema 16 (decision 11)

- Tabs get a required `flank_angle`, written explicitly, so **schema 15 → 16** with a
  migration, a frozen v15 sample and its golden test (CLAUDE.md).
- Migration from 15:
  - plunge entry → 90°, so those programs stay **byte-identical**;
  - ramp entry → 30°, or the operation's ramp angle when its tool isn't centre-cutting.
    These programs change anyway (the back-and-forth goes), which the user accepted (no
    external users yet).
- Pockets have no tabs; unchanged.

## 8. Existing programs

- Unchanged: every program without a ramped profile (plunge-only tabs migrate to 90°).
- Changed, diffs reviewed and explained in the commit message: the ramped profiles in
  `v7_ramp_entry`, `v7_copies`, `v7_leads`, `v7_helix`, `v13_helix_rings`,
  `v14_round_corners`, `v15_linuxcnc_helix`, `v15_mach3_helix`, and their `lines/` twins.
  Their pockets don't change.
- The next version's release notes mention it.

## 9. Decisions

1. Ramped profiles spiral down in whole laps, then one flat lap. **Approved.**
2. ~~Back-and-forth after tabs~~ → ramped tabs, the spiral follows them (user, 2026-10-12).
3. Ramp entry with a lead: lead-in once at the start depth. **Approved.**
4. Plunge entry with a lead: leads on every pass, as today. **Approved.**
5. Pockets unchanged for now. **Approved.**
6. The old behaviour is replaced, not kept as an option. **Approved.**
7. **New:** the spiral ends with a rise-out along the already-cut ring instead of a lead-out
   arc at the last depth. **Recommended**: a lead-out there would cut uncut waste over the
   whole depth in one go.
8. **New:** tab shape: the width is the flat top, flanks on both sides (symmetric
   trapezoid). **Recommended**: the tab stays centred where it is placed, and it is what
   other CAM programs do. Alternative: straight up in front, a flank only behind
   (asymmetric), which halves the extra foot.
9. **New:** the flank angle is its own tab setting, not the ramp angle. **Recommended**: at
   a 3° ramp angle a 3 mm tab would get 57 mm feet on each side.
10. **New:** defaults 90° (plunge), 30° (ramp), the ramp angle (tool not centre-cutting);
    a flank steeper than the ramp angle with such a tool is an error. **Recommended.**
11. **New:** schema 16 with an explicit `flank_angle`; migration as in §7.
    **Recommended**: an optional field would leave old ramped files to a silent default.

## 10. Tests

- Unit (spiral): laps start and end at the ring's start; each pass ends exactly at its
  depth; never steeper than the ramp angle, never more than one step-down per lap; `s`
  only grows; a short ring takes more laps, not a steeper slope; too short is refused; the
  flat lap is at the last depth; the rise-out ends at the start depth and only rises.
- Unit (tabs): the surface is a trapezoid of the set top width and flank angle; laps never
  go below it (`verify`); 90° gives exactly today's moves; joined feet give a note; the cut
  on a flank's way down is at most one step-down; tab height equal to the cut depth, a tab
  at the ring's start, feet wider than the gap between tabs.
- Plan: a ramped profile with a lead has one lead-in at the start depth and no lead-out
  arc; a plunged profile with a lead is unchanged; a flank steeper than the ramp angle with
  a tool that isn't centre-cutting is refused; copies with ramped profiles pass the overlap
  check; every program in these tests passes `chipload-check`.
- Proptest: random rings, depths, step-downs, tabs and angles: `s` only grows, never below
  a tab surface, never more than one step-down of uncut material under the tool.
- Migration: the frozen v15 sample migrates to 16 with the expected angles, and its golden
  G-code is unchanged.
- Golden: the changed programs above (diffs reviewed), the rest unchanged, and a new
  `v16_profile_spiral` (a ramped profile with ramped tabs and a lead) with its `lines/` twin.
- GUI: the flank angle field, its default per entry and tool, the foot length; a
  screenshot of the 3D preview.
