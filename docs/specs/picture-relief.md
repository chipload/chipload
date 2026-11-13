# A relief from a picture (depth map)

Status: draft for review, 2026-11-13. Asked by the user 2026-11-13 ("generate reliefs from
images … iphone has … depth map information … a separate step from images to stl
perhaps?"), decisions 1–5 as recommended ("Go ahead!"). Builds on docs/specs/stl-relief.md
(the model, the Relief operation, the rim, roughing and smoothing) and
docs/specs/relief-wizard.md.

## 1. What it is for

A grey picture where brightness is height, a **depth map**, carved as a relief: the common
way CNC reliefs are made and shared (PNG or JPEG, white high, black low), and the way to
use a depth map from anywhere else: an iPhone Portrait photo's, saved as a picture by an app
or a Shortcut, or one made from a photo by a depth-estimating program run outside
Chipload.

Not a separate STL step (decision 1): the picture is a **3D model in the project**, kept
as the picture itself with its settings, turned into a surface in memory whenever it is
read. Everything that takes a model takes it unchanged: the Relief operation, its rim,
roughing and smoothing, the wizard, the outline, simulation, the setup sheet. Changing the
depth or the smoothing is one field, not an export and an import.

## 2. The model in the project

A model (`drawing.models`, stl-relief.md §2) gains one optional field:

| Field | Meaning |
|---|---|
| `picture` | missing: the model is an STL file (as now). Present: `data` holds a picture (PNG, JPEG or BMP, base64) read as a depth map with these settings (§3) |

`file`, `sha256`, `data`, `name`, `layer`, `x`, `y`, `width`, `rotation`, `depth` and
`keep` mean what they mean for an STL. For a picture:
- `up` is `z` (a depth map is seen from above): anything else is a problem
  (`job-model-picture-up`);
- `units` is `mm` and plays no part: the picture is sized by `width` (its height follows
  the picture's proportion);
- `proportional` is false: a picture has no depth of its own, `depth` is the relief's.

**Schema:** an optional field with no meaning in older files, and an older Chipload
refuses it (models deny unknown fields), so it never reads a picture as an STL: no version
bump.

## 3. The settings (decision 2)

`picture` holds:

| Field | Meaning | Default |
|---|---|---|
| `white_high` | white is the top, black the bottom; false: the other way round | true |
| `smoothing` | a Gaussian blur over the heights, its radius in mm on the board, 0 to 5 | 0.2 |
| `background` | grey share from 0 to 0.95: the lowest heights up to it are background, not the model; 0: none | 0 |
| `gamma` | the heights raised to this power, 0.2 to 5: above 1 flatter low ground and steeper tops, below 1 the other way | 1 |

**From picture to heights,** in this order, each pixel a share `v` from 0 (black) to 1
(white), 16-bit greys kept (`chipload-image`'s decoding, alpha over white):
1. `v` turned round (`1 − v`) unless `white_high`;
2. smoothed by `smoothing` (a separable Gaussian of σ = radius / 2 in pixels, the edges
   repeated);
3. background: pixels with `v ≤ background` are left out; the rest are stretched to
   `(v − background) / (1 − background)` so the model still spans its whole depth;
4. `h = v^gamma`; the model's top is the highest `h`, so `depth` is measured from the
   brightest pixel kept, and `z = (h − h_max) · depth / (h_max − h_min)` with Z 0 at the
   top (a flat picture, `h_max = h_min`, is a flat model at Z 0).

**From heights to a surface** (`chipload-mesh`): the pixels resampled by area averaging
to a grid of at most `MAX_CELLS` = 800 cells along the longer side (0.15 mm cells on a 120 mm
relief, finer than a quarter of a 1 mm ball nose's radius, the detail threshold of
stl-relief.md §15), never finer than the picture; the grid's corners joined by two
triangles per cell wherever all four corners are kept, the top surface only. That is at most
1.28 million triangles, within what the relief handles (§15: 13 s for 1.14 million). Its
shadow from above is the kept pixels, so the **outline** (stl-relief.md §4) goes round
them: round the whole picture without a background, round the subject with one.

Everything after this is the STL path's: `keep` cuts from the top, `rotation` and the
placement, the light copy for views, the drop cutter.

## 4. Hostile input

The picture is read by `chipload-image`'s decoder as pictures for tracing are: PNG, JPEG,
BMP only, its size and pixel limits, under `catch_unwind`. HEIC (an iPhone's own photos) is
refused with advice (`model-picture-heic`): save the depth map as PNG first, or for an
object, scan it to STL (decision 4). A picture with every pixel background is refused
(`model-picture-empty`).

## 5. Where (decision 5)

- **Command line:** `model add FILE.png --width 120 --depth 4 [--black-high]
  [--smoothing 0.2mm] [--background 10%] [--gamma 1.5] [--at X,Y]`: a picture file makes a
  picture model (by its contents, not its name). `model set ID --smoothing … --background …
  --gamma … --white-high true|false` changes them; `relief add FILE.png …` makes the whole
  relief, its settings with defaults.
- **The window:** *Add 3D model…* and dropping a file take PNG, JPEG and BMP too. A picture
  model's properties show the settings above besides its size and place (no *Up*, no
  *Keep proportions*). The Browser shows it with the models, its icon a picture.
- **The Relief wizard:** step 1 takes a picture; step 2 (*Turn and cut*) shows the picture's
  settings in place of *Up* (quarter turns and *Keep from top* stay), its 3D view showing
  the result as for an STL.

## 6. Later (decision 3)

**Inflated shapes:** a black-and-white picture's shapes given a rounded height, rising from
each edge towards the middle (a distance transform, its profile a quarter circle or a
straight slope), as a second mode of the same picture model (`picture.mode`). Specified
when it is built.

Not planned: reading HEIC files and their embedded depth (a native library, a small and
noisy map), estimating depth from an ordinary photo (large models, out of scope by ADR 0017).

## 7. Tests

- A left-to-right grey ramp gives a model whose heights rise linearly within a cell's
  error; `white_high` false mirrors it; `gamma` 2 bends it as `h²`.
- A white disc on black with `background` 10 %: the outline is the disc, within a cell; the
  black around it is no model.
- Smoothing of 0 leaves a step a step; 1 mm softens it over about that.
- A 4000 × 3000 picture becomes at most 800 cells along: at most 1.28 million triangles.
- HEIC, an empty picture, a picture of background only: refused with their messages.
- A sample project with a small picture model and a relief with a rim: golden G-code
  (`v36_picture_relief`), the CLI's messages, the window's model form and wizard drawn.

## 8. Order of building

1. `chipload-image`: the depth map (§3 steps 1–4) as a grid of optional heights.
2. `chipload-mesh`: a grid of heights into a top surface.
3. `chipload-job`: `picture` on the model, its rules; the model read through it.
4. CLI `model add/set`, `relief add` with pictures; the golden sample.
5. The window: the model form, *Add 3D model…*, drops, the wizard.
6. The guide: *A relief from a picture*, with how to save an iPhone depth map.
