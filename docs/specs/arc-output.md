# Spec: arc output (G2/G3)

Status: **approved 2026-10-10** (Q1–Q5 as recommended). Implemented as described in ADR 0013;
notes on where the implementation differs in detail are marked *Implemented:*.

Guide: docs/book/src/posts.md.

Today every arc becomes a run of short G1 chords (ADR 0002 chose G1 for the first version).
The offsets already have true arcs (cavalier_contours, ADR 0006); they're flattened just
before the moves are made. Writing them as G2/G3 gives:
- much smaller programs (a 10 mm hole's ring is one or two lines instead of dozens),
- smoother motion: controllers plan arcs as arcs instead of many tiny segments,
- the base for trochoidal clearing, which is made of arcs.

## 1. Where arcs come from

- **Profile rings and pocket rings**, straight from the offset. Each ring becomes a list of
  lines and arcs instead of points; anything that needs points (picking a start point,
  ordering, the GUI) uses the flattened ring as now.
- **Tiny arcs stay lines**: an arc whose sagitta (bulge over its chord) is at most
  `ARC_APPROX_TOL_MM` (0.01 mm) is written as one G1, as it is cut within tolerance either
  way. Controllers handle very short or nearly straight arcs badly.
- Arcs are only in the XY plane (G17) at constant Z. No helical moves (ramping, later).
- Links between rings, plunges, retracts and rapids are lines, as now.

## 2. Writing an arc

- `G2` clockwise, `G3` counter-clockwise, with the end `X Y` and the centre as `I J`
  (offset from the start point). The `R` form is never used (ambiguous near 180°).
- Arcs are split so no piece sweeps more than 180°. A full circle is two pieces. (The
  offset rings already split at their lowest point, which stays the ring's start.)
- Numbers keep the post's decimals (3 for GRBL). Rounding moves an arc's ends by at most
  0.0005 mm, so the start and end radius agree within about 0.001 mm: well inside GRBL's
  own check (0.005 mm) and the independent checker's (0.01 mm).

## 3. Post-processor setting

A new post config value `arcs = true | false`. Data-driven, as every post setting (CLAUDE.md
rule 7):
- `true`: arcs as G2/G3.
- `false`: arcs flattened to G1 chords, exactly as today (for controllers or machines that
  don't do arcs well, e.g. some plasma/laser setups).
- Optional with default `false` in the file format, so no post schema bump and old configs
  mean what they did. The built-in `grbl.toml` sets `true`. *(Question 3.)*

## 4. Tabs

Tabs cut a ring into stretches at given distances along it. An arc that a tab boundary
falls on is split there into two arcs (same centre and radius), so tabbed rings keep their
arcs. The lift up over the tab and back down stays as now.

*Implemented:* rings stay point lists, so a tab boundary falls inside one chord of the
flattened arc. The arc runs up to the end of the last whole chord before the boundary; the
short piece of that chord up to the boundary is a G1 line (within the arc tolerance, as all
chords are). The same after the tab. Tabbed rings keep their arcs everywhere else.

## 5. Safety

- **Verification** checks what is cut: each arc against the walls and islands (closest
  distance ≥ tool-centre offset − tolerance), not just its chords. An arc bulges up to
  `ARC_APPROX_TOL_MM` past its chord, so checking chords alone isn't enough.
- **The independent checker** (rule 3: it must not depend on the toolpath code):
  - already requires I/J and matching start/end radius;
  - new: an arc must stay in G17 at constant Z; an arc with `R`, or one whose end equals its
    start (a full circle in one move), is refused;
  - new: travel limits and the "cutting outside the stock" warning use the arc's extent
    (its leftmost/rightmost/lowest/highest points), not only its end points, as an arc can
    bulge beyond them.
- **Determinism**: arcs come from the same offsets in the same order; centres and ends are
  formatted like every other number.

## 6. Display and the CLI

- The GUI draws arcs flattened, so the view looks as it does today; plunge numbers, 3D, the
  material-left shading are unaffected.
- `chipload plan --json` shows the planned moves, where a chord of an arc carries its
  `"circle": {"center": [x, y], "radius": r}`; `render` merges those into arcs when the post
  writes arcs (merged moves are `"arc_cw"` / `"arc_ccw"`, with the same `circle`).

## 7. Tests

- Unit: a full circle ring becomes two G2/G3 halves; a ring with a tiny arc keeps a G1;
  start/end radius after rounding within 0.001 mm; split at a tab boundary keeps centre and
  radius; arcs never exceed 180°.
- Verification: an arc that bulges into a wall (constructed) is caught although its chord
  isn't.
- Checker: G2/G3 with I/J pass; `R`, full-circle-in-one, changing Z, missing I/J, radius
  mismatch → errors; an arc bulging past the stock or travel is seen by its extent.
- `arcs = false` gives byte-identical G-code to today (all current golden files).
- Golden: with `arcs = true` (GRBL), every golden file changes; each new one passes the
  checker, and the commit message records how much shorter they are. Spot check by hand:
  the circle hole of `drawn_part` is one ring of two arcs with the expected centre.
- Property: random pockets and profiles with arcs pass verification and the checker.

## 8. Decisions (approved 2026-10-10)

1. **Arcs from offsets** for profile and pocket rings; arcs with a sagitta ≤ 0.01 mm stay
   G1. **Yes.**
2. **I/J form**, pieces of at most 180°, never `R`, never a full circle in one move.
   **Yes**: the form every controller reads the same way.
3. **Post setting `arcs`**: default `false` in the format (old configs unchanged), `true`
   in the built-in GRBL post. That changes every golden file once, reviewed and explained.
   **Yes.**
4. **Tabs**: split arcs at tab boundaries rather than falling back to chords on tabbed
   rings. **Split them**, so tabbed parts get arcs too.
5. **GUI** keeps drawing flattened arcs (no visible change). **Yes.**
