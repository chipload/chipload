# Spec: rounded corners on the clearing rings of a pocket

Status: approved (2026-10-12; decisions 1–5 as recommended). User request 2026-10-12. Applies to the
**rings** strategy only (adaptive has its own smoothing).

## 1. Why

A rings pocket cuts offsets of the pocket, wall ring first at the tool radius r, then
inwards every stepover s. Every convex corner of the pocket stays a sharp corner in every
ring: a 90° corner of the pocket gives a 90° turn in each of its rings. At each turn the
machine slows nearly to a stop (GRBL's junction deviation), which makes the motion jerky,
the cycle longer and the corners chatter. (Inside corners of the pocket, such as island
corners, already give arcs.)

## 2. The idea

Every ring except the wall ring gets its convex corners replaced by a tangent arc. Ring k
(k = 0 is the wall ring, at r + k·s from the walls) with corner radius ρ_k is the
pocket shrunk by r + k·s + ρ_k, then grown by ρ_k (a morphological opening). Its straight
parts and existing arcs are unchanged; only corners sharper than the radius become arcs.
With arc output on, they are G2/G3 moves.

**Corner radius:** ρ_k = min(k · s, D), D the tool diameter. Each ring's corners are as
round as its distance from the wall ring, up to one tool diameter. The rings stay nested
(ρ grows inwards by at most s per ring, so each rounded ring lies inside the one
outside it).

```
 wall ring (sharp)  ┌──────────────
 ring 1  (ρ = s)    │ ╭────────────
 ring 2  (ρ = 2s)   │ │ ╭──────────
                    │ │ │
```

## 3. What it costs, and the safety net

- **Material between rings at corners:** rounding ring k leaves a little more material in
  each corner for ring k − 1 to take. At a 90° corner the gap between consecutive rings
  along the corner's bisector grows from 1.41·s to 1.83·s, still within the tool's reach
  (2r) for stepovers up to 0.55·D. Sharper corners and larger stepovers need less rounding.
- **The uncut-material check** (exact distances, pocket.md) runs as it does now. If it
  finds uncut material only because of the rounding, the pocket is planned again with half
  the radii, then sharp. A note in the report says the corners were rounded less.
- **Narrow parts:** rounding also removes the parts of a ring narrower than 2ρ, and can split
  one ring into two at a narrow neck. Where that would happen, the radius for that ring
  is halved, then 0 (sharp), so nothing that was cut before is skipped.
- **Load:** the ring outside a rounded corner cuts slightly more at that corner (about
  0.4·s wider at 90°). This is far less than the full-width cut at the pocket's first
  ring, which every rings pocket already has.
- **Wall ring:** stays sharp. Rounding it would leave material in the corners of the
  finished pocket.

## 4. Setting and file

- A per-pocket setting **Round corners** (`round_corners: true/false`), on for new
  pockets (GUI and CLI defaults), shown only for the rings strategy.
- **Project schema 14:** older files migrate with `round_corners: false`, so their G-code
  doesn't change (frozen v13 sample, golden test). The bump makes an older Chipload refuse a
  v14 file as "newer" instead of tripping over an unknown field (as v10 did).
- CLI: the field in the project file, as the pocket's other settings (strategy, entry);
  `plan` and `run` use it.

## 5. Decisions to review

1. **Which rings:** all but the wall ring; the wall ring stays sharp. *Recommended:* the
   finish and the corner reach stay as they are.
2. **Radius:** ρ_k = min(k · s, D). *Recommended:* the corners grow rounder towards the
   middle, where it costs nothing, and the steps are small enough to stay covered.
3. **Setting:** on/off per pocket, on for new pockets, off for existing projects; no
   radius field. *Recommended:* one switch to start with, and a maximum radius can be
   added later as an optional field.
4. **Schema 14** with a migration that sets it off. *Recommended* (CLAUDE.md: migrations
   must not change machining).
5. **Fallback** when rounding would leave material: half the radii, then sharp, with a
   note. *Recommended:* never an error, never uncut material.

## 6. Tests

- A square pocket: rings 1… have arcs of radius min(k·s, D) at the corners, the wall ring is
  sharp, the rings are nested, and no uncut material is found.
- A property test (as `pocket_never_gouges_and_clears_everything`) with rounding on:
  no gouge, nothing uncut, rings nested.
- A narrow neck: no ring is dropped, and the radius falls back where needed.
- A stepover where the full radii would leave material: the fallback and its note.
- The frozen v13 sample migrates to `round_corners: false` with byte-identical G-code; a
  new golden with rounding on (arcs and lines posts), passing the independent checker.

## 7. As built (2026-10-12)

- `pocket::round_ring_corners` and `rounded_rings`; `Choices::round_corners`; the project
  field `round_corners` (schema 14, `v13_to_v14` sets false). GUI: *Clearing → Corners*,
  rings only; new pockets have it on.
- **Nesting:** rings are rounded innermost first, and each ring's radius is capped at its
  inner rings' radius plus a stepover, so a rounded ring always lies inside the one outside it
  even where an inner ring had to use a smaller radius.
- **Narrow parts:** a ring whose rounding would split it, lose a part or a hole is tried with
  half the radius, up to six times, then stays sharp (a 1.2 mm wide innermost ring still gets
  a small radius).
- **Uncut material (decision 5, refined):** rather than halving every ring's radius, the
  check says where material would be left and only the rings within r + s of it get half
  their radius (down to an eighth, then sharp), for up to twelve rounds. Only if that fails
  does the whole piece stay sharp, with the note. Smaller radii in places get no note: they
  are not something to act on. Acute corners limit the radius most: in a triangle with 34°
  corners at a stepover of 0.3–0.5 × D most rings still round, with smaller radii.
- **Cost:** each round is one uncut check. A 300 × 200 mm pocket with two islands and 57
  rings plans in 0.33 s with rounding, 0.01 s without (release).
- **Tests:** tests/round_corners.rs (corner radii, wall ring sharp, narrow neck, acute
  corners, a property test against gouges and uncut material, timing); migration test and
  frozen v13 sample `v13/helix_rings` (same G-code as `v7_helix`); golden
  `v14_round_corners` (a rectangle round two islands, rounding on).
