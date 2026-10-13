# Spec: traced pictures with arcs, and centre lines for follow path

Status: **approved and built** (2026-10-13, all recommendations taken; as built: §10).
Follows docs/specs/image-trace.md §10.

Two additions to image tracing:

1. **Arcs:** the traced edges become lines *and arcs*, not only lines.
2. **Centre lines:** a picture of strokes (handwriting, a signature, line art) traces into
   the strokes' centre lines, as open paths for a [follow path](follow-path.md), instead of
   outlines for a V-carve.

## 1. What each is for

**Arcs.** A traced curve is now many short lines: a 40 mm logo at the default smoothing
(0.05 mm) gives hundreds of segments per letter. With arcs:
- **Profiles, pockets and follow paths** of traced shapes get G2/G3 (arc output,
  docs/specs/arc-output.md). That means smaller programs and smoother motion, since a
  controller slows at every short line it can't look past.
- **V-carving gains little:** a V-carve's moves are lines whatever the outline is. Its centre
  line is a little cleaner, because an arc is split into even chords.

So arcs matter most for centre lines and for silhouettes cut out with a profile.

**Centre lines.** A V-carve of a thin stroke (2 mm wide) is a groove that follows the
stroke's middle, but only as wide as the stroke. A **follow path** along the centre line,
with a V-bit or a small end mill, cuts each stroke as one pass at a chosen depth. That's
what engraving handwriting or a single-line drawing wants: an even line, the bit's width,
whatever the pen did. Inkscape's centre-line tracing exists, but it is an extension that
gives poor results and has to be set up by hand. Here, as with outlines, the settings are
judged against the cut (ADR 0017's argument for image tracing).

## 2. Arc fitting

**Input:** each traced loop as traced: the dense points of §3 of image-trace.md, about one
per pixel, before simplifying.

**Output:** the loop as lines and arcs. Every traced point lies within the **smoothing**
(the same setting as now) of the fitted line or arc. The ends of every line and arc are traced
points.

**Method:** greedy, longest piece first. This is our own code (as image-trace.md decision 3).
- From the current point, find the farthest point that a **line** reaches within the
  smoothing, and the farthest that an **arc** reaches:
  - **Line:** every point in between lies within the smoothing of the segment.
  - **Arc:** the circle runs through the run's first, middle and last points. Every point in
    between lies within the smoothing of the arc, and the points go round in order (the
    angle never goes back).
- The search for the farthest point doubles, then halves (binary search).
- Take whichever reaches farther; on a tie, the line. Then go on from where it ended.
- **Limits:**
  - An arc sweeps at most 180°.
  - Its radius is at least the smoothing, and at most 1000 mm. Beyond that, it's a line
    (a nearly straight arc gains nothing and makes poor G2/G3).
- **Where a closed loop starts:** a fit that starts mid-curve leaves a kink there, so the
  loop starts at its sharpest corner (largest turn). With no corner (an O), it starts at
  the lowest-left point.

**Never crossing.** The loops must still never cross or touch (image-trace.md §3).
- **Check:** the fitted loops are flattened to chords within the arc tolerance (0.01 mm), and
  checked with the same grid test as now. They must stay at least twice that tolerance
  (0.02 mm) apart, because a chord can sit 0.01 mm away from its arc.
- **Fallback:** a loop that fails falls back to its lines-only version (Douglas–Peucker, as
  now). If that fails too, it stays as traced.
- **Report:** says how many loops fell back.

**Stored as** `path` shapes with arc segments (`PathSegment::Arc`), which paths already
have.

## 3. Centre lines

**Input:** the traced loops, as traced, after small pieces are dropped (the smallest
piece applies as now).

**Output:** open (and sometimes closed) paths along the black strokes' middles.

**Method:** the medial axis that V-carving uses (`chipload-toolpath`'s `medial`, ADR 0015).
1. **Areas:** the loops are nested into areas (outline and its holes), as for a pocket.
2. **Centre line:** the medial axis of each area: its Voronoi graph, inside the area.
3. **Tidying:** small side branches go. A rough stroke edge, or a square stroke end, gives a
   short branch into each bump or corner. The V-carve pruning (`Graph::prune`) removes a
   branch when it adds nothing beyond its junction's disc. Here the allowance is larger:
   - **Allowance:** the smoothing, or **half the stroke's half-width at the junction**,
     whichever is larger.
   - **Why half:** a square stroke end needs 0.41 × the half-width to lose its two corner
     branches. Real branches, such as a T's leg or a Y's arms, reach much farther.
4. **Paths:** the graph becomes paths.
   - A run of edges between two ends or junctions (nodes that aren't two-way) becomes one
     path.
   - A ring with no junction (an O) becomes a closed path.
   - Paths meet at junctions, end to end; a follow path cuts each as its own path.
5. **Simplified:** each path is fitted with lines and arcs within the smoothing (§2), or
   Douglas–Peucker for lines only, keeping its two ends.
   - **No crossing check:** paths are only cut along, never offset, so a crossing can't make
     bad geometry. Their deviation is bounded by the smoothing.
6. **Dots:** a piece whose centre line is shorter than the smoothing (a round dot, the
   point of an i) gives no path. These are counted, and the report says how many were
   dropped.

**Where it ends.** A centre line ends one half-width short of a round stroke end: at the
centre of the end's circle. A bit as wide as the stroke reaches the end exactly.

**Stroke width.** The report gives the strokes' width range (twice the half-width along
the paths, as the 5th and 95th percentile), so a bit can be picked: "strokes 1.1 to 2.3 mm
wide".

**Limits.** At most 5000 paths (the same refusal as loops).

**Where it lives** (architecture):
- `chipload-image` stays core-only and traces loops.
- `chipload-toolpath` gains `centre_lines(loops, smoothing) -> Result<CentreLines, _>`,
  built on `medial`.
- `chipload-pipeline` puts the two together when an image is set to centre lines.
- The arc fitter is in `chipload-core`, as it serves both. It's plain geometry, and the core
  has the primitives.

## 4. In the project (schema 20)

Two new fields on each image:

| Field | Values | Default for a new image | Migrated v19 images |
|---|---|---|---|
| `trace` | `"outline"`, `"centre_line"` | `"outline"` | `"outline"` |
| `arcs` | `true`, `false` | `true` | `false` |

- **Migration v19→v20** adds both with the values that keep what was traced. The stored
  shapes don't change, and neither do the cuts.
- **Why a version bump:** 0.15 would read a v20 file and retrace an image without knowing
  either field. That would silently turn centre lines into outlines, and so would change
  the cuts.
- **The new samples:** `tests/corpus/jobs/v19/image_vcarve.chipload` stays as it is. A new
  frozen sample, `v20/image_centre_line.chipload`, gets its golden G-code: a follow path,
  with arcs.

## 5. Operations

- **Traced outlines:** as now.
- **Centre lines:** these are open paths, so pockets, V-carves and profiles refuse them as
  they refuse any open shape. A **follow path** is the operation for them.
  - **In the app:** with a centre-line layer selected, *+ Follow path* is the button to use.
  - **Hint:** a V-carve or pocket of that layer gets a problem with a hint: "image 2 is traced
    as centre lines: use a follow path".

## 6. Command line and app

**Command line:**
- `chipload image add|set … --trace outline|centre-line --arcs true|false`.
- `image list` shows both.
- The report adds the stroke widths and dots for centre lines, and the fallbacks for arcs.

**App:** the image window gains two settings.
- **Trace:** *Outlines (V-carve)* or *Centre lines (follow path)*.
- **Arcs:** a checkbox.
- **Preview:** centre lines are drawn as orange lines over the black and white. Their width
  is the stroke-width line in the summary.
- **Import:** selects the layer as now. For centre lines, the hint under Import says
  "+ Follow path".

## 7. Edge cases

- **A thick blob** (a filled area, not a stroke): its centre line is a skeleton with
  branches. That is correct, but rarely wanted. The report's width range shows it (strokes
  "1.2 to 14 mm wide"), and the guide says centre lines are for strokes.
- **Strokes touching each other** (a loop in handwriting that fills in): a junction, or a
  small closed path. That is fine.
- **A stroke touching the picture border:** closed along the border as now, so its centre
  line bends toward the border. The black-at-border note covers it.
- **Arcs on tiny loops** (under 4 traced points): left as traced.
- **A circle traced as an O:** two arcs of 180° or less on each of its two loops. With
  centre lines, a closed path of arcs.

## 8. Tests

- **Property tests (proptest), for the arc fitter on random smooth and jagged point rings
  and open paths:**
  - every input point lies within the tolerance of the fitted path;
  - ends are input points;
  - arcs sweep at most 180° and keep the radius limits;
  - the result has no more pieces than Douglas–Peucker's.
- **Picture tests (chipload-image and pipeline), on drawn pictures:**
  - a circle gives at most 4 arcs;
  - a rectangle gives 4 lines;
  - fitted loops never cross (the existing traced-pictures property, run with arcs);
  - centre lines of a drawn "T" give 3 paths meeting at one point;
  - an "O" gives one closed path;
  - a square-ended bar gives one path, no corner branches;
  - a dot is counted and dropped;
  - the stroke width of a 2 mm bar reads 2 mm.
- **Golden:** `v20_image_centre_line` (a follow path, G2/G3); `v19_image_vcarve` unchanged
  after migration.
- **Size check (in the commit message):** segments and G-code lines for the logo, with and
  without arcs.
- **Fuzzing:** the image fuzz target runs both new settings.

## 9. Decisions for review

1. **Build both now**, in the order: arcs, then centre lines. **Recommended:** centre lines
   need the fitter for their own paths. Alternative: centre lines only, lines only.
2. **Greedy line-or-arc fitting within the smoothing**, our own code. **Recommended:**
   simple and predictable, and it matches the smoothing users already set. Alternative:
   biarcs (smooth joins, about twice as many arcs and more code). Tangent continuity isn't
   needed: arcs join at traced points and the machine sees no corner sharper than the
   tolerance.
3. **Arcs on by default for new images, off for migrated ones.** **Recommended.**
   Alternative: off by default (no surprise in the program's look).
4. **Centre lines from the existing medial axis**, in `chipload-toolpath`, with the larger
   tidying allowance of §3 (half the half-width). **Recommended:** proven code with exact
   distances, and no new dependency. Alternative: thinning the black-and-white pixels
   (Zhang–Suen) in `chipload-image`. That gives staircase lines, spurs, and its own
   tidying.
5. **The arc fitter in `chipload-core`.** **Recommended:** both `chipload-image` and
   `chipload-toolpath` use it, and the core is where shared geometry lives.
6. **Schema 20** with `trace` and `arcs` on each image. **Recommended**, for the reason in
   §4.
7. **Dots are dropped with a count**, not turned into plunges. **Recommended:** a plunge
   isn't a follow path. A hole operation on the dots is a later question if anyone asks.
8. **The stroke width reported as a range (5th to 95th percentile).** **Recommended:** the
   extremes are junctions and ends.

## 10. As built (2026-10-13)

What changed from the draft while building it:

- **Arcs follow the path, not only its points.** Three points always lie on some circle, so
  an arc through a cut pixel corner passed the point test while bulging away from the traced
  edge. The middle of every chord between neighbouring points must also lie within the
  smoothing of the arc. The property test checks those midpoints too.
- **The nearness check** flattens arcs within the chaining tolerance (0.001 mm), not the arc
  tolerance, and a fitted outline must stay twice that (0.002 mm) from any other outline.
  Outlines that are lines only are checked for crossing as before, so a pair of traced loops
  that nearly touch at a saddle doesn't push its neighbours back to lines.
- **No "no more pieces than Douglas–Peucker" property:** greedy lines aren't guaranteed
  fewer than Douglas–Peucker's. Instead, the logo's size check: 152 lines without arcs,
  23 pieces (19 arcs) with them.
- **Tidying judges a pass's branches together** (`centre::tidy`): with one at a time, the
  second corner branch of a square stroke end became part of the stroke once the first was
  gone. A junction keeps its farthest-reaching branch when all of them would go. V-carving's
  pruning is unchanged.
- **Dots:** a piece is a dot when its whole centre line lies within its widest circle grown
  by the tidying allowance (a round dot, a small blob), rather than by the centre line's
  length.
- **The hint is a problem:** a profile, pocket, V-carve or holes operation whose layer holds
  an image's centre lines is refused ("layer "x" holds the centre lines of image 1: cut them
  with a follow path, or trace the image as outlines"). A closed centre line (an O) would
  otherwise be pocketed as if it were an outline.
- **The report** also counts outlines kept as lines (`without_arcs`).
- **Import notes:** centre lines show in the drawing's report as open contours, and their
  junctions as "more than two segments meet at one point". Both are true and harmless.
- **Between centre lines that meet**, a follow path retracts and plunges again at the
  junction, as it does between any two paths (follow path's own behaviour).
