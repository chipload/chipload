# Spec: images traced into shapes, for V-carving

Status: **approved 2026-10-13** ("Go ahead with recommendations"): all decisions in §11
as recommended. Built 2026-10-13. As built:
- **Pixels:** at most 20 megapixels, not 40. Each working copy of the picture takes 8
  bytes a pixel, so 40 megapixels would need well over a gigabyte while tracing.
- **In the file:** the list is `drawing.images` (paths `drawing.images.0.threshold`): the
  images belong to the drawing, beside its DXF `source`. The DXF itself is not embedded,
  only its shapes; a picture is, because it is traced again when its settings change.
- **Traced shapes:** `path` shapes with `"image": id`, written only when set, so projects
  without images save exactly as before. The DXF's placement moves only the DXF's shapes.
- **In the app:** the settings are changed in the image window (Job settings > the image's
  row > *Edit…*), not as fields in the sidebar: the window shows the preview they need. Its
  preview is traced in the background; *Import* and *Apply* trace once more in the
  foreground (well under a second for pictures of a few megapixels).
- **Operations:** clicking traced shapes and adding an operation selects their layer
  (decision 6).
- **Default width:** the window starts at half the stock's width, shown and editable; the
  command line requires `--width`.
- **Report:** the trace report gives the shapes, the threshold, the specks dropped, black at
  the border, and shapes left unsimplified. The narrowest feature is left to the V-carve,
  which already reports strokes narrower than the bit's tip.

The user (2026-10-13): "I would like to import an image, convert it to black and white with
adjustable parameters, trace the outline, for V-carve use."

## 1. What it is

1. **Import:** a picture (a logo, a scanned sketch, lettering) goes into the project.
2. **Black and white:** a few settings turn it black and white, with a live preview.
3. **Trace:** the edges between black and white become closed shapes.
4. **V-carve:** the shapes are carved like any others, usually with V-carving.

Changing a setting traces the image again, and the operations follow.

**Scope (ADR 0017).** The question is whether a power user would reach for another tool.
Today they do: Inkscape's *Trace bitmap* (potrace), then a DXF export, then an import here.
But the settings that matter (the threshold, the smoothing, the smallest speck) are
judged by how the V-carve comes out. In Inkscape, every change is a round trip: trace,
export, import, check the units, look at the carve. Doing it in Chipload, next to the carve
preview, is the point, and it feeds the existing V-carving rather than adding a separate
thing. **Recommended: in scope**, recorded in ADR 0017 (decision 1). Not part of it: editing
the picture (painting, cropping), colour separation into several layers, and centreline
tracing (§10).

## 2. Black and white

For each pixel:
1. **Grey:** the grey value (Rec. 709 luma, 0–255). Transparent pixels count as white,
   blended over white by their alpha.
2. **Blur:** a Gaussian blur of *blur* pixels (0: none), which smooths ragged edges and
   noise from scans and JPEGs.
3. **Black or white:** the pixel is black when its grey value is below the *threshold*.
4. **Invert:** with *invert* on, black and white swap.

**Black is what gets carved:** for a V-carve, black is the inside of the shapes.

The settings:

| Setting | Range | Default | What it does |
|---|---|---|---|
| Threshold | 1–254, or *auto* | auto | where grey turns black; auto picks it from the picture's histogram (Otsu's method) and shows the value it chose |
| Invert | on/off | off | carve the light parts instead |
| Blur | 0–10 px | 1 | smooths edges before the threshold |
| Smallest piece | 0–100 mm² | 0.5 mm² | black or white specks smaller than this are dropped (dust, JPEG noise) |
| Smoothing | 0.01–1 mm | 0.05 mm | how far a traced edge may deviate from the true edge, to keep the shapes small |

## 3. Tracing

1. **The edge:** the boundary between black and white is traced as the line where the
   blurred grey value equals the threshold, by marching squares with linear interpolation.
   - It lies between pixel centres, so it's smooth, with no pixel staircase.
   - It matches the black-and-white preview exactly: every black pixel centre is inside,
     every white one outside.
   - Where four pixels form a checkerboard (a saddle), the average of the four decides,
     the same way every time.
2. **Loops:** marching squares always gives closed loops that never cross each other or
   themselves.
   - The picture's border counts as white, so black touching the edge is closed along it.
   - Loops are oriented by what they enclose (black inside: counter-clockwise).
3. **Specks:** loops enclosing less than *smallest piece* are dropped, with what they hold.
4. **Simplifying:** each loop is simplified to lines within *smoothing* (Douglas–Peucker,
   in mm after scaling).
   - The result is checked: still closed, not crossing itself or another loop, and
     everywhere within *smoothing* of the traced edge. A loop that fails is kept
     unsimplified, with a note.
   - Lines only, no arcs. V-carving works from the segments, and DXF splines are already
     flattened this way. Fitting arcs can come later (§10).
5. **Size:** the picture is placed at a **width in mm** that you give; the height follows
   from the aspect ratio. There's no guessing from DPI: a picture's DPI is often missing
   or wrong, and the units rule (CLAUDE.md) says never guess silently. Its lower-left corner
   goes where you say (default: the stock's origin), and it can be moved and resized later.

My own implementation, from the published algorithms (decision 3): marching squares,
Otsu's threshold and Douglas–Peucker are short, well known and easy to test, and this way
the shapes' accuracy, closure and determinism are under our control.

## 4. In the project (schema 19)

The project gains `images`, a list. Each entry has:
- **Identity:** `id`, `file` (its name) and `sha256`.
- **The picture:** `data`, the file's bytes in base64. It's embedded like the drawing,
  because projects travel without their files.
- **Where it goes:** `layer`, plus `x`, `y` and `width` (mm).
- **The settings:** `threshold` (a number or `"auto"`), `invert`, `blur`, `min_area`
  and `smoothing`.

**The traced shapes are stored too**, as ordinary `path` shapes on the image's layer, marked
as coming from that image. So the G-code never depends on re-tracing (a later Chipload with a
better tracer doesn't change a saved project's cuts), and everything that works on shapes
works on them.

**Changing a setting** traces again and replaces that image's shapes. Operations select the
layer, so they follow; the shapes get new ids. An operation that selects traced shapes by id
would lose them, so operations made from an image select its layer, and selecting traced
shapes by id is refused with advice (decision 6).

**Schema:**
- The version goes to 19: version 18 would drop the images on save without saying so.
- The migration 18 → 19 adds an empty list, with a frozen v18 sample.
- **Limits:** PNG, JPEG and BMP up to 20 MB and 40 megapixels. Bigger files are refused,
  with the advice to scale them down.

## 5. Command line (first, rule 5)

```
chipload image add part.chipload logo.png --width 120 [--at X,Y] [--layer logo]
        [--threshold auto|N] [--invert] [--blur PX] [--min-area MM2] [--smoothing MM]
chipload image set part.chipload 1 --threshold 140 --blur 2     # trace again
chipload image list part.chipload                               # images, settings, shape counts
chipload image preview part.chipload 1 -o bw.png                # the black-and-white picture
chipload image remove part.chipload 1                           # with its shapes
```

- **Reports:** `add` and `set` print what was traced: loops, the specks dropped, the
  threshold auto picked, the smallest feature, and a note when a feature is narrower than
  the project's V-bits can carve.
- **Scripting:** `--json` works as everywhere, and `get`/`set` reach the settings by path
  (`images.0.threshold=140`). After a `set` of an image's settings, the image is traced
  again.

## 6. In the app

**File > Import image…** opens a dialog:
- **Left:** the picture as black and white, updated as the settings change, with the traced
  outlines drawn over it.
- **Right:** the settings of §2, the width in mm, and the numbers of §5.
- **Buttons:** *Import* puts the shapes on the canvas and offers **+ V-carve** for the
  layer; *Cancel* adds nothing.

**Afterwards:**
- The image's row under *Drawing* opens the same dialog to change it.
- Its settings also sit in the sidebar like any other values; a change there traces again,
  and the live preview regenerates the carve.
- *View > Show images* shows the picture faintly under the canvas, so it's easy to compare.

The tracing runs in the background like generation, so the dialog stays responsive on
large pictures.

## 7. Where it lives

- **A new crate, `chipload-image`:** bytes in, loops in mm out.
  - Decoding (the `image` crate, already in the tree through the DXF reader; add its
    `jpeg` feature, MIT/Apache-2.0), grey, blur, threshold, Otsu, marching squares,
    simplification, the checks, and the preview picture.
  - It sits beside `chipload-dxf` and depends only on `chipload-core`, with no file I/O.
  - The architecture test learns the crate.
- **`chipload-job`:** the `images` model, the migration, validation.
- **`chipload-pipeline`:** add / trace again / remove, turning loops into `path` shapes.
- **CLI and GUI:** thin, as always.

## 8. Edge cases

- **An all-white or all-black picture:** no loops (or one around the edge), with a note.
  Not an error.
- **Black touching the picture's border:** closed along the border (§3.2), with a note,
  since it's usually a frame or a background the user meant to invert.
- **A huge number of loops** (a photo, noise): over 5 000, refused with advice (more blur,
  a larger smallest piece, a higher threshold), rather than flooding the project.
- **Corrupt or hostile files:** decoding errors are typed errors; the decoder runs under
  `catch_unwind`, and the limits of §4 are checked from the header before decoding. The
  fuzzer gets a second target for it.
- **A grey picture with alpha** (a logo on transparency): alpha over white, so the logo
  traces as expected.
- **Very small widths:** a feature smaller than *smoothing* may vanish; the report's
  smallest feature shows it.
- **16-bit or palette images:** converted to 8-bit grey first.

## 9. Tests

- **Units:**
  - grey and alpha;
  - Otsu on a picture with two known levels;
  - the blur kernel sums to 1;
  - a black square gives one loop through the midpoints of its edge pixels;
  - a disk of radius R px traces to within 0.5 px of R everywhere;
  - a checkerboard saddle resolves the same way every time;
  - specks dropped by area;
  - invert swaps.
- **Properties (proptest):** random black-and-white pictures give loops that are closed,
  never cross, have every black pixel centre inside and every white one outside, and stay
  within *smoothing* after simplifying.
- **Golden:** a small logo PNG in `tests/corpus/images/`, a project that traces and
  V-carves it, and golden G-code and setup sheet. The frozen v18 project migrates to 19
  unchanged.
- **Guide:** a new page, *Images*; the reference pages are regenerated.
- **Fuzzing:** a target for decode and trace.

## 10. Later, not now

- **Arc fitting** for traced loops (smaller G-code for profiles of traced shapes).
- **Centreline tracing** of single strokes (handwriting) for follow path.
- **Several thresholds** into several layers (depth by grey level).
- **SVG import:** a vector format, a separate question (ADR 0017's CAD line).

## 11. Decisions for review

1. **In scope, with an addition to ADR 0017.** **Recommended:** as argued in §1: the
   settings are judged against the carve, which only Chipload shows. Alternative: out of
   scope; users trace in Inkscape and import the DXF.
2. **Trace the iso-line of the blurred grey image** (marching squares with interpolation),
   not the pixel edges. **Recommended:** smooth outlines with no staircase, matching the
   preview exactly. Alternative: pixel-edge tracing with potrace-style curve fitting:
   sharper corners, much more code.
3. **Our own implementation, no tracing library.** **Recommended:** the algorithms are
   short and well known, and control over closure, accuracy and determinism matters.
   Alternative: `visioncortex` (vtracer's core, MIT OR Apache-2.0): Bézier output we'd
   still flatten and verify, plus tuning parameters that don't map to what V-carving
   needs.
4. **Lines only for now**, arcs later. **Recommended:** V-carving uses segments anyway.
5. **Store the picture and the traced shapes, schema 19.** **Recommended:** the picture so
   the settings can change later; the shapes so the cuts never depend on the tracer's
   version. Alternative: shapes only (no schema change, but no adjusting after import), or
   the picture only (retraced on load; smaller files, but the cuts could change between
   versions).
6. **Operations select the image's layer;** selecting traced shapes by id is refused.
   **Recommended:** ids change on every retrace. Alternative: keep ids stable by matching
   loops between traces (fragile).
7. **The width in mm is required;** no DPI guessing. **Recommended** (the units rule).
8. **The settings of §2 and their defaults:** threshold auto, invert off, blur 1 px,
   smallest piece 0.5 mm², smoothing 0.05 mm. **Recommended** as a start; tune on real
   logos during testing.
9. **Limits:** 20 MB, 40 megapixels, 5 000 loops. **Recommended.**
10. **Formats:** PNG, JPEG, BMP. **Recommended:** GIF, WebP and TIFF on request (each is
    a feature of the `image` crate, but each one adds code that reads hostile input).
