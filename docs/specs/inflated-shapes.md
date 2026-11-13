# Inflated shapes: a relief from a black-and-white picture

Status: draft for review, 2026-11-13. Asked by the user 2026-11-13 ("Describe inflated
shapes" … "Go ahead"). Builds on docs/specs/picture-relief.md (a picture as a 3D model) and
docs/specs/image-trace.md (black and white from a picture).

## 1. What it is for

A flat black-and-white picture (a logo, lettering, a silhouette) carved as a rounded relief:
each shape rises from its edges towards its middle, as if inflated. Nothing is drawn in 3D:
a point's height comes from how far it lies inside its shape. Signs with raised, rounded
letters, moulded-looking emblems, soft silhouettes.

It makes a grid of heights, as a depth map does, so everything after it is the picture
model's: the surface, outline, Relief, rim, *Fuller*, roughing, smoothing, simulation.

## 2. Decisions (for review)

1. **A mode of the picture model,** not a new kind of model: `picture.mode` is `depth` (as
   now, the default when missing) or `inflate`. Switching keeps the model, its place and
   its reliefs. *Recommended.*
2. **Two profiles,** `round` (a quarter circle: steep at the edge, flat on top, a pillow)
   and `straight` (a straight rise: a ridge along narrow strokes, chiselled). A flat top
   comes from the reach (decision 3), with either. *Recommended* over a third
   "flat-topped" profile, which `round` with a short reach already is.
3. **The reach,** how far in from the edge the rise goes before the height stops growing:
   by default **automatic**, the widest shape's half-width (its largest inscribed circle's
   radius), so nothing is flattened and the widest shape reaches the full depth; or set in
   mm, which levels everything wider off at the full depth (letters of one height, broad
   areas flat). *Recommended.* The guide says to set it to about half a letter's stroke for
   lettering.
4. **Distance on the grid:** the exact Euclidean distance transform of the black-and-white
   grid (Felzenszwalb and Huttenlocher, "Distance Transforms of Sampled Functions", 2012),
   then the existing *Smoothing* takes the pixel steps off. *Recommended* over measuring to
   the traced outline (smoother edges, but far slower without a spatial index).
5. **Black and white as tracing has it:** the same threshold (automatic, Otsu, or a grey
   level) and *invert*; black is the shape by default, as for V-carving. *Recommended.*

## 3. The model in the project

`picture` (picture-relief.md §2) gains optional fields; missing, they mean a depth map as
now:

| Field | Meaning | Default |
|---|---|---|
| `mode` | `depth` or `inflate` | `depth` |
| `threshold` | `inflate`: `auto`, or a grey level 1–254 below which is black | `auto` |
| `invert` | `inflate`: white is the shape | false |
| `profile` | `inflate`: `round` or `straight` | `round` |
| `reach` | `inflate`: mm from the edge, more than 0; null: automatic | null |

`white_high`, `background` and `gamma` belong to `depth` and are kept but unused in
`inflate`; `smoothing` serves both.

**Schema:** optional fields with safe defaults; an older Chipload refuses them (`picture`
denies unknown fields), so it never reads an inflated model as a depth map. No version
bump.

## 4. From picture to heights (`chipload-image`)

1. The picture averaged onto the grid (≤ 800 cells along, picture-relief.md §3), then black
   and white by the threshold (and `invert`), as tracing makes it: shape or not, cell by
   cell.
2. **Distance:** for each shape cell, the distance from its centre to the nearest non-shape
   cell's centre, minus half a cell (so a one-cell stroke's middle is half a cell from its
   edges), in mm: the squared Euclidean transform of Felzenszwalb and Huttenlocher, one pass
   along the rows and one along the columns (lower envelope of parabolas), exact on the
   grid. The picture's border counts as outside: a shape touching it rises from it too.
3. **Reach** `R`: as set, or automatic, the largest distance of all.
4. **Height** `t = min(d, R) / R`, then `h = √(1 − (1 − t)²)` (round) or `h = t` (straight).
5. **Smoothing** (the blur, picture-relief.md §3) over the heights; cells outside the
   shapes stay out (None), so the outline is the shapes'.
6. **Depth:** as for a depth map, the highest point at the top and `depth` below it; with
   `R` set, every shape wider than `2R` reaches the full depth.

Holes in a shape (a letter's counter) are outside, so a stroke rises from both its sides.
Shapes of fewer than four cells make no triangles (picture-relief.md §3) and are left out.

## 5. Where

- **Command line:** `model add logo.png --inflate [--profile round|straight]
  [--reach 3mm] [--threshold auto|128] [--invert] --width 120 --depth 4`; `model set ID
  --mode inflate|depth`, and the same options; `relief add logo.png --inflate …`.
- **The window:** the picture settings get *Kind*: *Depth map* or *Inflated*; inflated
  shows *Threshold*, *Invert*, *Profile*, *Reach* (with *Automatic* ticked by default) and
  *Smoothing*. In the model's properties, *Turn and cut* and the wizard's step 2, whose 3D
  view shows the result.
- **Messages:** a picture with no shape at the threshold is refused
  (`image-error-inflate-empty`); a reach of 0 or less is a problem.

## 6. Tests

- A black disc of radius `r` (round, automatic reach): the height at distance `d` from the
  centre follows `√(1 − (d/r)²)` within a cell; straight: a cone.
- Two discs of radii `r` and `2r`, reach automatic (`2r`): the small one's top is at
  `t = 0.5`, so `√(1 − 0.25)` ≈ 87 % of the full depth (straight: 50 %); reach set to `r`:
  both reach the full depth, the large one flat beyond `r` from its edge.
- An "O": the ring rises from both sides, its middle line the highest.
- `invert`, a threshold level, a shape touching the border.
- The distance transform against brute force on random small grids (proptest).
- An empty picture refused; a golden sample (`v36_inflated_relief`), the CLI's messages,
  the window's forms drawn.

## 7. Order of building

1. `chipload-image`: the distance transform and `inflate_grid` (§4), with tests.
2. `chipload-job`: the fields and their rules; the toolpath's picture surface by mode.
3. CLI options; the golden sample.
4. The window: *Kind* and its fields.
5. The guide: *Inflated shapes* under *A relief from a picture*.
