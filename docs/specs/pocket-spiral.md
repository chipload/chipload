# Spec: the pocket's rings as one spiral

Status: **built 2026-10-15**; **amended 2026-10-16** by docs/specs/pocket-island-order.md
§4: a ring steps out wherever the cut order goes on outward to a ring of the same kind next
to it, island rings included (the "island rings never step out" rule of §2 and §3 no longer
holds). As first built, as the user asked ("morphing spiral as you mentioned would
be nice") after the pocket clearing review of the same day (docs/specs/pocket.md §9).

Guide: docs/book/src/operations/pocket.md (Rings). Builds on docs/specs/pocket.md and
docs/specs/pocket-round-corners.md.

## 1. Today

The rings of a pocket are closed loops, cut from the centre outwards. Between two rings the
tool feeds across at depth where the link is safe (always, in a plain pocket). So the tool
closes each ring, stops, steps one stepover sideways, and starts the next ring: a short
full-width move into material at every ring, and a direction change at every ring's end.

## 2. The idea

A ring whose next ring is the one outside it **steps out** onto it over its last stretch,
instead of closing: the end of the ring drifts sideways from the ring to the next ring's
start, so the two rings are one continuous spiral. The step-out replaces the link.

- **Which rings:** the cut order (docs/specs/pocket.md §3, inside out) pushes a node's
  children first, then its island rings, then its outer ring. The ring pushed just before a
  node's outer ring steps out onto it when it is the outer ring of one of the node's
  children and no island ring came between. It then lies one stepover inside, with nothing
  cut in between, which is what the step-out crosses. Island rings, the ring before an
  island ring, and the wall ring (the last one) never step out.
- **Where it lands:** the next ring is restarted (docs/specs/helix-and-leads.md part A, `restart`) at its point
  nearest the stepping ring's start, so the step is as short as possible: one stepover
  along a straight side. A wall ring with a chosen start point or leads keeps its start.
  Consecutive rounded corners have the same capped radius, so on their diagonal the next
  ring is √2 stepovers away; a landing point farther than 1.5 stepovers (a rounded corner
  against the wall's sharp corner) is not stepped onto: the ring stays closed and is
  linked as before.
- **Shape:** the last two tool diameters of the ring (at most half its length) drift
  sideways evenly, from nothing to the whole shift at the end, which lands exactly on the
  next ring's start. The stretch begins at a vertex (one is inserted on the chord there).
  The drift is in the direction from the ring's start to the next ring's start, so it
  points outwards across the uncut band. Two diameters along for one stepover sideways is
  a gentle angle (about 11° at a 0.4 stepover); the width of cut rises to a stepover over
  that stretch and never exceeds the full width the link had.
- **Checked like a ring:** every point of the stepped-out path must lie inside the region
  and every chord must keep the tool's clearance from the walls and islands (the same check
  as the rings, `verify`); otherwise that ring stays closed and linked. The uncut-material
  check runs on the result; if it reports material that the closed rings would not have
  left, the piece is cut with closed rings and links instead (the whole piece, so the
  notes stay as before).
- **Moves:** a ring that stepped out is an open path ending on the next ring's start, so no
  link move is written for it (a zero-length feed would come out as a bare `G1`).
- **Arcs:** the chords of the drifting stretch are no longer on their circles, so their arc
  tags are dropped there; the rest of the ring keeps its arcs.

## 3. Not done

- The island rings and the rings between an island ring and the next outer ring still close
  and link: stepping out across an island's neighbourhood needs a check that the step does
  not cross the island, which the plain link already has, and the gain is one short move
  per island per depth.
- The wall ring never steps out (it is the last) and nothing steps out onto a ring with
  leads or a chosen start other than at its start.

## 4. Tests

- `inner_rings_step_out_onto_the_next_as_one_spiral`: a plain rectangle; every ring but the
  wall steps out, is open and ends exactly where the next starts; the links are all feeds;
  no notes; every chord keeps the tool's clearance; one plunge and one continuous path per
  depth, with no zero-length move.
- `the_ring_before_an_island_ring_does_not_step_out`: with an island, island rings, the
  ring before an island ring and the wall ring stay closed; the inner lobes still spiral;
  clearance holds.
- The existing property test (random polygons with islands, no gouge and nothing left at
  stepover ≤ 0.5) covers the stepped-out paths.
- Goldens: every pocket's G-code changed (the ring ends drift and the link moves are gone);
  the checker passes on all of them.
