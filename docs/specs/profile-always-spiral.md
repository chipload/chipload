# Spec: profiles always spiral (schema 27)

Status: **approved 2026-10-15** (the user: "Do both", after docs/specs/profile-ramp-level.md),
from the profile review of the same day. User rule: an option that is objectively worse is
removed, not offered (docs/specs/pocket.md §9 did the same for pockets).

Guide: docs/book/src/operations/profile.md. Changes docs/specs/profile-form.md,
docs/specs/ramp-entry.md §5 and docs/specs/helix-and-leads.md part B (profiles).

## 1. Why

A profile had two ways down: a spiral ramp along the contour, or a straight plunge at the
start of every pass, with an optional lead-in and lead-out for the plunge. After
docs/specs/profile-ramp-level.md the spiral costs the ramp's length plus the rise-out, about
3 s on the measured corpus profile, against a plunge:

- a plunge without a lead leaves a dwell mark on the part's wall at every pass and needs a
  centre-cutting tool;
- a plunge with a lead removes the mark but retracts and rapids on every pass, which costs
  the same 3 s, needs room in the waste beside the start, and can't be used on a profile on
  the line;
- the spiral works for every side, every tool and every start, with no mark.

The guide already called the spiral the best way down, while a new profile with a
centre-cutting tool still got a plunge.

## 2. Format: schema 27

| v26 | v27 | Migration v26 → v27 |
|---|---|---|
| profile `entry` (`plunge` or `ramp` with `ramp_angle`) | profile `ramp_angle`, required | `entry.ramp_angle` for a ramp; **3°** (the default angle) for a plunge or a missing entry; a helix's angle for a helix (which was refused on a profile) |
| profile `lead` | gone | dropped; a profile's leads were one lead-in at the start depth with a spiral, in the air |
| profile `tabs.flank_angle` | unchanged | kept as it is: 90° tabs stay straight (a plunge of one step at each tab's end, allowed for a centre-cutting tool; a tool marked not centre-cutting is refused as today) |

Pockets keep `entry` (plunge, ramp or helix) and `lead`; follow paths keep `entry`
(plunge or ramp); holes are unaffected.

The migration changes the G-code of every plunged profile on purpose (the user's decision,
as for schema 26): they spiral at 3°. A frozen v26 sample (`v26_straight_tabs`: a plunge
profile with straight tabs and a lead) gets a golden from the new code.

## 3. Rules

- `0 < ramp_angle ≤ 30°`, a problem at `ramp_angle` otherwise (the ramp's rule as before).
- The tool rules for a profile shrink to the flank rule: with a tool marked not
  centre-cutting the tabs' flanks may be no steeper than the ramp angle; unknown, a note.
  "A straight plunge needs a centre-cutting tool" no longer applies to profiles.
- A lead on a profile is no longer a field: a v27 file with one is refused as unknown.

## 4. Planner

- `RingCut` loses its leads and takes `ramp_angle: Option<f64>` (None: a plunge, for a
  closed follow path); the wall lap takes the profile's `ramp_angle`. The profile's ring
  start no longer moves to the longest chord's middle for a lead.
- `profile_leads` and the lead-on-profile checks go; pocket leads are unchanged.

## 5. GUI and CLI

- The profile's Cut section shows **Ramp angle** after the step down (no Entry combo, no
  lead row, no "this tool can't plunge" line). A problem at `ramp_angle` marks the section.
- New profiles: 3°; new tabs: 30° flanks, or the ramp angle with a tool marked not
  centre-cutting (as for a ramp today).
- The setup sheet's entry column shows "ramp 3°" for every profile.
- `set operations.N.ramp_angle=…` (radians, as every angle in the file).

## 6. Tests

- Migration: the frozen v26 sample migrates (plunge → 3°, ramp keeps its angle, `lead`
  gone) and its golden passes the checker; a v27 file with `entry` or `lead` on a profile
  is refused.
- The lead tests for profiles go; the pocket lead tests stay. The finish, dogbone, tab and
  plan tests build profiles with `ramp_angle`.
- GUI: the Cut section draws the ramp angle; a problem at `ramp_angle` marks it; a new
  profile has 3° and 30° flanks when it gets tabs.
- Goldens: every profile that plunged changes (most of the corpus); the checker passes on
  all of them.
