# Spec: rapid to an approach height before a plunge

Status: **implemented 2026-10-15** (user: fix the findings of the 2026-10-14 review).

## 1. What

Every plunge used to start at safe Z and go down at the plunge feed through the air above
the stock, at every entry of every pass. With safe Z at 5 mm and a plunge feed of a few
hundred mm/min that is seconds per entry, and a profile with four tabs and three passes
has dozens of entries.

Now the tool rapids from safe Z down to the **approach height**, `APPROACH_CLEARANCE_MM`
(1 mm) above the stock top, and only then feeds down. Nothing else changes: the XY rapid
still happens at safe Z, the feed from the approach height to the cut is the plunge feed as
before, and the rapid never goes below the stock top, so the checker's rule (no rapid
below the stock top) holds and clamps above the stock are still cleared at safe Z between
cuts.

## 2. Rules

- Applies to a feed move straight down (Z only) that starts at safe Z and ends below the
  approach height. Ramps, helices and the drill's rapids back into its hole start from
  heights the generators set themselves and are not touched.
- The approach height is the stock top (as `z_zero` places it) plus 1 mm. When safe Z is
  not above it, nothing is inserted.
- The pass is applied to every operation's moves as they are planned
  (`chipload_toolpath::plan::lower_approaches`), before move indexes are recorded, so
  spans, groups and comments stay right.
- The start depth plays no part: the air between the stock top and the start depth is
  still crossed at the plunge feed, since Chipload doesn't know whether another operation
  cleared it.

## 3. Effects

- Every G-code golden changes by one `G0 Z<approach>` line before each plunge.
- Cycle times get shorter; the setup-sheet goldens change with them.
- The simulation and playback are unaffected (a rapid above the stock cuts nothing).
