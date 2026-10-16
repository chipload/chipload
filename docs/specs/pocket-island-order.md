# Spec: a pocket's rings, one island at a time

Status: **approved 2026-10-16** ("i agree, go ahead"), decisions as recommended (§5).
Built the same day (`cut_order` in crates/chipload-toolpath/src/pocket.rs). As built:
- §6: on the test's rectangle with two islands, lifts went from 14 to 7 and step-outs from
  1 to 8. The 7 lifts left are between separate deep lobes (the corners and between the
  islands), which no order can join.
- In `round_corners.chipload`: from 15 to 11 pieces per pass, 31 to 23 retracts, and
  1,256 to 986 mm of rapids. The program is 24.9 s shorter (7:54 to 7:29). The pieces left:
  - the deep lobes;
  - the ring that wraps round the triangle's side at level 3 (the triangle's rings need
    it);
  - the edge rings that have a **lead**: a ring with a lead starts at its lead, and
    nothing steps out onto it (pocket-spiral.md).

User report (2026-10-16), on `tests/corpus/jobs/v14/round_corners.chipload`: "the pocket
operation does not finish the islands when it has started on them, instead it takes a bite
on one island and then on next island and a bit back and forth".

Amends docs/specs/pocket.md §3 (cut order) and docs/specs/pocket-spiral.md §2–3 (which
rings step out).

## 1. Today

- The rings are offset levels: level k is the area that keeps k stepovers more from the
  walls and islands than level 0. Each area of a level is a node in a tree, with its outer
  ring and a ring round each island it surrounds.
- The cut order takes the tree's nodes children first. For each node it cuts the node's
  island rings, then its outer ring.
- So every level is finished before the next one out. Around two islands the order is:
  - circle ring k, triangle ring k, wall ring k;
  - circle ring k−1, triangle ring k−1, wall ring k−1;
  - and so on.
- The tool crosses the pocket between each pair and lifts each time. In the report's
  project, each depth pass is 15 pieces with a retract between them: 30 retracts and
  1,256 mm of rapids in the pocket.
- The spiral (pocket-spiral.md) can't help: it only steps out onto the next ring, and the
  next ring cut is never the one outside.

## 2. What must come before a ring

- A ring at level k cuts the band between itself and the level k+1 rings next to it. It
  never meets more than a stepover of material if those rings were cut first, together
  with everything deeper that they need. That's the only rule the order must keep.
- A ring at level k+1 is **next to** a ring at level k when they come within a **tool
  diameter** (and at least 1.5 stepovers) of each other anywhere. Only the rings of the
  level k area's own children are looked at.
  - The rings next to each other along a side are exactly a stepover apart (√2 at a
    rounded corner).
  - The tool diameter also covers a narrow neck: an island close to the wall, where the
    tool at one ring reaches the other ring's material.
  - In such a neck, the island's rings and the wall's rings must alternate, as they do today.
- Rings farther apart than that don't depend on each other. In particular, an island's
  rings and the wall's rings, where the gap between them is more than a tool diameter, are
  independent.

## 3. The new order

The planner keeps a list of the rings whose rings next to them (§2) are all cut ("ready").
Starting where the tool is, it repeats:
1. **Go on outward:** if a ready ring is next to the ring just cut, one level out, and of the
   same kind (an island's ring after an island's ring, an outer ring after an outer ring),
   take it. The nearest wins if there are several. This follows one island's rings out to
   the island, or the outer rings out to the wall, without crossing the pocket.
2. **Otherwise start elsewhere** (decision 3): take the nearest ready ring, preferring:
   - first, rings that are not along the outer wall: the middle, then the islands;
   - last, the outer rings that end at the wall, so the outer wall is still the last ring
     at each depth, as today.

Each ring starts at its point nearest the tool, as today.

## 4. Spiral round islands too (decision 2)

- A ring taken by rule 1 (going on outward, same kind) is marked to **step out** onto it.
  This now includes an island's rings, which drift toward the island onto the next ring.
- A ring followed by a ring of the other kind, or by one picked by rule 2, closes and links
  as before.
- Everything else in pocket-spiral.md stays:
  - the island's own edge ring and the wall ring never step out;
  - every step-out is checked against the walls and the islands;
  - where a step-out would leave material, the whole piece is cut with closed rings and
    links instead.

## 5. Decisions (2026-10-16)

1. Order by island, as in §3. **Agreed.**
2. Island rings spiral too (§4). **Agreed.**
3. After an island, the nearest unfinished island; the outer wall's rings last. **Agreed.**

## 6. Tests

- **One island at a time:** a rectangle with a circle and a square island, far enough from
  the wall and each other. Each island's rings come one after another. The last ring is the
  wall, and there are fewer separate pieces (links that aren't feeds) than before.
- **Never more than a stepover** (the rule of §2, checked independently of the code):
  - Take any point of any ring that is farther than 1.5 stepovers from every ring cut
    before it.
  - Then no ring cut later lies within 1.5 stepovers of that point on the ring's deeper
    side (inside an outer ring, outside an island's ring).
  - Run on the island test and on a narrow neck.
- **The island rings spiral:** in the island test, island rings step out onto the next
  island ring, except each island's edge ring.
- The property tests (no gouge, nothing left) and the plain-rectangle spiral test stay as
  they are.
- **Goldens:** every pocket with islands changes its G-code; pockets without islands don't
  change. The checker passes on all of them. Each change is looked at and explained in the
  commit.
