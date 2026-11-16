# Spec: nesting parts on a board

Status: **draft 2026-11-16**, an outline for review; its details are written once
docs/specs/part-placement.md is approved and built. Nesting came into scope on 2026-11-16
(ADR 0017; the user: "Bring nesting into scope").

## 1. What it is

Place a setup's instances (every part's, instance 1 included, part-placement.md) on a board
of a fixed size so they fit without touching, using as little of the board as it can. The
result is only placements (`at`, `angle` per instance): nothing new in the file, and
everything checks it as it checks placements made by hand (the overlap check, the
checker, the simulation). It can be undone in one step and adjusted by hand after.

## 2. What it places

- **What each instance cuts**, not its drawing: the outline of the toolpath with the tool
  (tabs, leads, ramps included), from the part planned once (as `copies::cut_size` already
  does), plus a **gap** between parts (default: the tool's diameter, so a full slot stays
  between neighbours) and a **margin** to the board's edge.
- **Holes inside a part** can hold small parts (an option; later step).
- **Turns allowed**, per part: none (wood grain), 180°, quarter turns, or free in steps.
- **Mirrors** are never added: a mirrored part is a different part.
- **Parts with a number wanted**: "4 of side, 2 of lid" adds or removes instances to match
  (an option), else the instances there are placed.
- **What doesn't fit** is reported by name and left where it was, never dropped.

## 3. Steps

1. **By boxes:** each instance's cut box, turned only by quarter turns, packed with the
   MaxRects algorithm (J. Jylänki, *A Thousand Ways to Pack the Bin – A Practical Approach
   to Two-Dimensional Rectangle Bin Packing*, 2010), best short side fit. Fast and
   predictable; good for signs, box sides, rectangular parts.
2. **By shape:** the real cut outlines, placed bottom-left using no-fit polygons
   (E. K. Burke, R. S. R. Hellier, G. Kendall, G. Whitwell, *Complete and robust no-fit
   polygon generation for the irregular stock cutting problem*, European Journal of
   Operational Research 179, 2007), the order of placing tried a few ways and the best
   kept. Arcs flattened within the arc tolerance for the search only; the placement found
   is exact and checked on the real toolpath.
3. **Several boards:** what doesn't fit goes to a new setup of the same board (a sheet
   each), as an option.

## 4. Where it is

- Window: a setup's right-click *Nest…*: the board, gap, margin, turns, what to place; a
  preview before *Apply*.
- Command line: `chipload nest part.chipload [--setup ID] [--gap 6] [--margin 10]
  [--turns none|half|quarter|free]`, with `--json` for what was placed and what didn't fit.
- Deterministic: the same project and settings give the same placements (no randomness
  without a fixed seed), so the same G-code.

## 5. Clean room

Built from the published papers above and first principles only; no nester's source
(Deepnest, SVGNest and the like) is read, and no other program's output is used as a model
(CLAUDE.md).

## 6. Decisions (to settle when its full spec is written)

1. Boxes first, shapes second. **Recommended.**
2. The gap's default: the tool's diameter of the part's largest tool. **Recommended.**
3. Turns per part or per nesting run. **Recommended:** per part, kept in the file (an
   optional field, no bump) so grain is remembered.
4. Several boards as setups. **Recommended:** a later step.
