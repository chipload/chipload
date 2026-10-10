# 0013: Arc output (G2/G3) from tagged chords

Status: accepted (2026-10-10). Supersedes the "G1 output" part of ADR 0002.

## Context
Programs were all G1 chords: large files, and controllers plan many tiny segments worse than
arcs. The offsets already have true arcs (ADR 0006). Everything between the offset and the
moves (start points, ring order, links, tabs, verification, uncut and material-left checks,
the GUI) works on point lists and is well tested.

## Decision
Keep rings as point lists. While flattening an offset, record which chords lie on which
arc in an `ArcTable` (keyed by their exact end points). The planner tags feed moves that are
such chords with their circle. The post-processor, if its config says `arcs = true`, merges
runs of tagged chords back into G2/G3 (I/J, at most 180°, nearly straight arcs stay G1);
otherwise it ignores the tags. Verification also checks each tagged chord's arc midpoint
against the walls, and the independent checker validates arcs on its own (radius, no full
circle, constant Z, extents for the stock and travel checks).

## Consequences
- With `arcs = false` the output is byte-identical to before (tested against the frozen
  line-only golden files in tests/corpus/expected/lines/).
- Every decision still sees exactly the same points, so nothing about ordering or safety
  checks changed; arcs are a change of representation at the end.
- A chord split at a tab boundary is written as a line (see the spec, section 4).
- I/J are chosen among neighbouring roundings so start and end radius agree after rounding
  (worst in the corpus: 0.25 µm).
