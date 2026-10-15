# Spec: the profile's spiral ramps at the angle and levels only the ramp

Status: **approved 2026-10-15** (the user: "Do both", this and docs/specs/profile-always-spiral.md,
"1 first"), from the profile review of the same day.

Guide: docs/book/src/operations/profile.md (Entry). Changes docs/specs/profile-spiral.md §3
and the skin lap of docs/specs/finish-and-skin.md §3.

## 1. Today

A spiral-ramped profile descends each pass **evenly over whole laps**: the smallest number
of laps that keeps the slope within the ramp angle. On a long ring that spreads a 4 mm
step over the whole lap, half a degree where the user set 3°. Because the whole last lap
is sloped, a **whole level lap** at the end depth follows, then the rise-out. The skin lap
is built the same way: a thin skin takes one sloped lap plus one level lap.

Measured on the corpus spiral project (6 mm endmill, 12.3 mm deep, 460 mm round), against a
straight plunge: one pass 28.6 s instead of 15.4 s; three passes 70.7 s instead of 57.4 s.
The difference is almost exactly the extra lap.

## 2. The change

- **Ramp at the angle.** Each pass descends at the ramp angle over the length that needs,
  `L = step / tan(angle)`, starting at the ring's start, then runs **level** at the pass
  depth for the rest of that lap. When `L` is longer than a lap (a small ring), the descent
  runs on over whole laps and the level stretch completes the lap it ends in. The slope is
  always the ramp angle, never steeper; the lap count is never more than today's.
- **One step down under the tool, still.** The lap before ramped the same stretch at the
  same slope (or, for the first pass, the start depth lies there), so the material under
  the tool on a ramp is one step thick throughout, and nothing under the level stretch.
- **Level only the ramp.** After the last pass's lap, the floor is at the end depth
  everywhere except the ramped stretch, where a wedge one step deep at the start tapers to
  nothing at `L` (modulo the ring's length). One level stretch over exactly that length
  cuts it. Then the rise-out as today, over one tool diameter, from the end of that
  stretch; it runs on material cut to the end depth already (the last lap's level part, or
  the stretch just levelled when it wraps past the start).
- **The skin lap** (docs/specs/finish-and-skin.md §3) does the same: down the skin's
  thickness at the ramp angle from the lap's start (the widest gap between tabs), level
  round to the start, level the ramped stretch, rise out. A 0.3 mm skin at 3° ramps over
  5.7 mm instead of a whole lap.
- **Tabs, leads, dogbones, arcs, feeds:** unchanged. The course still follows
  `max(lap depth, tab surface)`, the ramp still uses the cutting feed, chords on arcs are
  still written as helical arcs, and the refusal for a ring far too short for the angle
  (more than 1000 laps of ramp) stays with its message.
- Pockets' out-and-back ramp (docs/specs/ramp-entry.md) and the helix are not touched.

Saved per ring: one lap minus `L`. A one-pass 3 mm sheet profile at 3° (57 mm of ramp)
takes about half the time it did; the measured three-pass profile loses 386 mm of its
1844 mm, 13 s of 71.

## 3. Tests

- `a_spiral_goes_round_and_down_and_reaches_each_depth_at_the_start` becomes: each pass
  reaches its depth at `L` into its lap and stays level to the lap's end; the final level
  stretch covers `[0, L]` at the end depth; the rise-out starts at `L`; the slope on every
  descending chord is the ramp angle within tolerance.
- `a_short_ring_takes_more_laps_not_a_steeper_slope`: the descent spans whole laps at the
  angle, the level stretch completes the lap, the final level stretch is `L mod length`.
- A skin lap test: ramp over the skin's length, level round, level the stretch, rise.
- The property test (forward only, never below a tab, one step under the tool, everything
  cut at the end) covers the new course unchanged.
- Goldens: every spiral profile and every skin lap changes; the checker passes on all.
