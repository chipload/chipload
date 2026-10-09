# Spec: showing material the toolpath leaves

Status: **approved 2026-10-09** (answers to Q1–Q4 in §8: all as recommended).

User request (2): material the cutter can't reach, such as sharp inside corners or narrow
necks, is "not an error but could be highlighted discretely".

Today Chipload only reports leftover material as warning text: pockets warn about uncut
material between rings and about narrow parts, and profiles warn about narrow parts. You
can't see *where* the material is, and you get no hint at all about inside corners.
Inside corners matter for parts that fit together, like the lid and box in `messy_2.dxf`.

Out of scope: rest machining with a smaller tool, corner relief (dogbones), any change to
toolpaths or G-code.

## 1. Two kinds of material left

For an area the operation should clear, with tool radius r:

| Kind | What it is | Normal? | Shown as |
|---|---|---|---|
| **Unreachable** | No tool disk of radius r fits there: inside corners (radius r stays), slots and necks narrower than D | Yes, a property of the tool and the drawing | Faint fill, **no warning text** |
| **Missed** | The tool fits, but the rings don't pass close enough: big stepover in corners, the pocket centre | No, the toolpath should have cleared it | Stronger fill, **plus the existing warning** |

The narrow-part warnings ("the tool does not fit through a narrow part…") stay as they are:
in a narrow part the slug may not come free or a wall is left standing, which is worth
telling. Corner fillets get no text.

## 2. Which area, per operation

"Should be cleared" is defined only from the drawing and the operation:

- **Pocket**: each pocket region R (outer contour minus islands, spec pocket.md §2).
  - unreachable = R − opening(R, r), where opening(R, r) = (R shrunk by r) grown by r.
  - missed = the pieces the existing uncut-material check reports (pocket.md §3), no change
    to how they're found.
- **Profile, inside**: the area inside the contour, as for a pocket with no islands.
  - unreachable = inside − opening(inside, r). The corners of a hole stay rounded.
- **Profile, outside**: the area just outside the contour that the cut separates.
  - unreachable = closing(P, r) − P, where P is the inside of the contour and
    closing(P, r) = (P grown by r) shrunk by r. These are the part's inside corners.
- **Profile, on the line**: nothing; there's no side to leave material on.
- **Tabs** are left on purpose: not shown as material left.

Within one pocket operation, crossing pockets clear each other's corners (the crossing bars
in `messy_2.dxf`), so a piece is shown only where none of the operation's pockets reaches.
Missed spots are still judged per pocket.

Between operations, each one is judged on its own. A corner one operation leaves can be
cleared by a later operation, and it's still shown. Telling them apart is rest machining,
out of scope.

## 3. Computing it (chipload-toolpath)

Display only: nothing here changes the toolpath, and a failure must never block G-code.

- The opening's boundary coincides with R's walls along every straight wall, and booleans
  on coincident edges are fragile (slivers of zero width). So the computation leaves a gap
  of δ = `ARC_APPROX_TOL_MM`:
  - pocket / inside: R − (R shrunk by r, grown by r + δ)
  - outside: (P grown by r, shrunk by r + δ) − P

  This misses slivers thinner than δ (0.01 mm), which are invisible anyway.
- Offsets and booleans use cavalier_contours, as the rings do, under `guarded` (a panic
  becomes an error). Results are normalised like the rings (pocket.md §5), so they're
  deterministic.
- **Verified**: every piece must lie inside R (pocket / inside) or outside P (outside),
  within tolerance. If the computation fails or the check fails, that operation shows no
  highlight and gets one note: "could not compute the material left for display". The
  G-code is unaffected.
- Missed pieces come out of the existing quadtree as its leftover cells (axis-aligned
  squares, merged into rows). No new computation.
- Output, added to `Plan` and `RunResult`:

  ```rust
  pub struct MaterialLeft {
      pub operation: usize,          // index in job.operations
      pub kind: LeftKind,            // Unreachable | Missed
      pub rings: Vec<Vec<Point>>,    // closed rings, arcs flattened, filled even-odd
  }
  ```

  An unreachable piece is its outline plus any holes; a missed piece is the quadtree squares
  it is made of. Pieces that hold no disk of diameter `REST_TOL_MM` are dropped (chord and
  float noise).
- Sizes are bounding boxes of what is shown, so the gap makes a square corner left by a
  3 mm radius tool read as about 2.75 mm: the thin tips along the walls are cut off.

## 4. CLI

Every feature in the CLI first (CLAUDE.md):

- `chipload plan --json` gets a `"material_left"` list: `{operation, kind, rings}`.
- `chipload plan` and `run` (text) add one summary line per operation that leaves any, e.g.
  `Operation 2 (lid pocket): the tool can't reach material in 4 places, largest about
  2.75 x 2.75 mm`. This is a plain info line, not a `warning:`. The GUI report shows it too.
- Missed material keeps its existing `warning:` line, unchanged.

## 5. GUI

- Unreachable: a faint, translucent fill in a muted colour, under the toolpath and the
  contours, so it never hides a line. Missed: the same fill in the warning colour, stronger.
- A toggle **Show material left** in the View menu, on by default. It isn't saved between
  sessions (there are no app settings yet).
- Selecting an operation in the list emphasises its own pieces (as its contours are already
  drawn wider) and fades the rest.
- egui only fills convex polygons (`epaint` 0.36.2, `PathShape`: "Fill is only supported for
  convex polygons"), and these pieces are concave (a corner fillet). The view fills them by
  scanlines into a triangle mesh: rendering only, no geometry logic, no new dependency.
- Hovering a piece shows a tooltip: kind, operation and size.

## 6. Determinism and formats

- Nothing is persisted, so no job schema change and no golden G-code change.
- The new JSON list is ordered by operation, then kind, then (min y, min x) of the piece.

## 7. Tests

- **Unit (toolpath)**:
  - Two crossing pockets in one operation (a T): the crossbar's corners inside the upright
    are not shown.
  - Square pocket, sharp corners: 4 unreachable corner pieces, each with area
    r²(1 − π/4) within tolerance; no missed pieces.
  - Pocket with a rounded-corner outline whose corner radius ≥ r: nothing unreachable.
  - Slot narrower than D next to a wide pocket: the slot is unreachable, and the
    narrow-part warning is still given.
  - Pocket with a square island: the island's corners are convex for the pocket, so
    nothing is unreachable there.
  - Inside profile on a square: 4 corners. Outside profile on an L shape: 1 inside corner.
    Outside profile on a square: nothing. Profile on the line: nothing.
  - Stepover 1.0 on a sharp-cornered pocket: missed pieces exist, at the place the warning
    names.
  - Boundaries: tool radius exactly equal to a corner fillet radius (nothing), tool
    larger than the pocket (the existing error, no highlight).
- **Property (proptest)**: for random simple polygons with random islands:
  - every unreachable piece lies inside R;
  - sampled points inside a piece are farther than r − tol from every cutting move;
  - sampled points of R farther than r + 2·`REST_TOL_MM` from every cutting move lie in a
    shown piece (nothing left over is unshown);
  - a geometry-library failure takes the fallback above. Such cases are rejected, so they
    can't pass in bulk; only a degenerate outline (near-collinear spikes) has hit one.
- **Golden**: G-code for every corpus project is unchanged (the test already exists).
- **GUI**: scanline fill of a concave polygon (an L, a corner fillet) gives triangles whose
  total area equals the polygon's.

## 8. Decisions (approved 2026-10-09)

1. **Kinds and presentation** (§1): unreachable = faint fill, no text; missed = stronger fill
   plus the existing warning. **Yes.**
2. **Profiles too** (§2): show inside corners for inside and outside profiles, not only
   pockets. **Yes**: that's where fits between parts go wrong.
3. **CLI text** (§4): one info line per operation in `plan`/`run`, or JSON only?
   **The info line**, so the CLI shows what the GUI shows; it's not a warning.
4. **Toggle** (§5): show by default with a view toggle. **On by default.**
