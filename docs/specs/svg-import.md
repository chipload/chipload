# Spec: SVG import (drawing phase 3)

Status: **approved 2026-10-28** (the user: "approve"; every recommendation in §12). ADR
0024. Details settled while building are marked *Implemented:*.

Phase 3 of docs/specs/drawing-roadmap.md (§6, added 2026-10-27). Logos and artwork come from
Inkscape, Illustrator, Affinity and the like as SVG, which many laser and sign shops work in.
An SVG becomes a drawing source like a DXF: its paths become shapes on layers, placed,
turned and scaled, updated when the file changes, and explodable into drawn shapes.

## 1. What a user can do

- **Open or import an SVG** as a new project, as a DXF is today (`chipload import logo.svg -o
  logo.chipload`; the window's Open and Import, and dropping the file).
- **Add an SVG to a project** beside its DXFs (`chipload dxf add part.chipload logo.svg`;
  *Add drawing* in the window).
- **Place, update, remove and explode** it exactly as a DXF: per file, the same commands
  and the same Browser group.
- **Inspect** it first: `chipload inspect logo.svg` gives units, contours and the import
  report.

Not in scope: SVG export (phase 5 is DXF export); colours or stroke widths as cut settings
(depth by colour); text in SVGs (§6); embedded pictures (§6).

## 2. Where it lives (decision 1)

An SVG is one more **drawing file** in `drawing.dxfs`, with a new field
`format: "dxf" | "svg"`. Everything that is per DXF today works for it unchanged:
placement, *update from a newer file* (matching shapes), remove, explode into drawn shapes,
the Browser group with its layers, hiding (`dxf:2`, `dxf:2/layer:Cut`), and selectors
limited to one file (`{"layer": "Cut", "dxf": 2}`).

- The names stay as they are in the file format and on the command line (`dxfs`, `"dxf": 2`,
  `chipload dxf add`), which now take an SVG too. The window and the guide say *drawing
  file* where it matters and show the file's name, so users never see "DXF" on an SVG.
- **Schema 31:** `format` is required; migration 30→31 adds `"format": "dxf"` to every
  drawing file. Nothing moves, so every golden stays the same (a frozen v30 sample exists:
  `jobs/v30/polygon.chipload`; the test migrates it).
- *Considered:* a list of its own (`drawing.svgs`, `ShapeFrom::Svg`, selectors `"svg": 1`).
  Cleaner names, but every per-file feature (placement, update, remove, explode, Browser,
  hiding, selectors, problems) would be written twice for the same behaviour.

## 3. Reading: `usvg` (an ADR)

- **`usvg` 0.48.1** (Apache-2.0 OR MIT, the resvg project) parses the file and resolves what
  makes SVG hard: CSS and presentation attributes, `use` and `symbol`, nested transforms,
  units, and basic shapes (`rect`, `circle`, `ellipse`, `line`, `polyline`, `polygon`)
  into plain paths in one coordinate system.
- **Built without its default features** except `svgz` (gzip-compressed `.svgz`): no `text`
  (no font shaping, no system fonts read), no `writer`. Its remaining dependencies are
  MIT, Apache-2.0, BSD-3-Clause or Zlib (roxmltree, svgtypes, kurbo, tiny-skia-path,
  simplecss, data-url, imagesize, strict-num, siphasher, log, pico-args, flate2).
- **A new crate `chipload-svg`** (on core only, no file I/O, like `chipload-image`): bytes
  in, shapes and an import report out, in mm, before placement. The pipeline turns them
  into a drawing file as it does a DXF's. It must still build for the browser
  (docs/specs/web.md).
- `roxmltree` (already under usvg) is also read directly for the one thing usvg drops:
  Inkscape's layer names (§5).

## 4. What becomes a shape

- **Every path, each subpath a shape**, as usvg leaves it after resolving shapes and
  transforms. A subpath is closed when it has `Z` or its ends meet within the chaining
  tolerance; open otherwise.
- **Fill and stroke don't matter for the geometry:** a shape is the path's centre line,
  what a laser or a router follows. Stroke widths, colours and opacity are ignored. A path
  with neither fill nor stroke, or `visibility: hidden`, is still geometry and is imported
  (usvg keeps it, marked hidden), and the report counts these; elements with
  `display: none` are not (usvg drops them).
- **Curves:** quadratic and cubic Béziers (and the SVG arcs and ellipses usvg turns into
  them) become lines and arcs with `chipload_core::fit::fit_path`, within the arc
  tolerance (0.01 mm), measured in mm after the unit scale, as text outlines are. A
  circle comes back as arcs.
- **Holes:** nesting decides outer and hole by geometry, as for a DXF; the fill rule is not
  used. Self-crossing subpaths are reported as a DXF's are (folded contours).
- **The import report** counts paths and subpaths imported, curves fitted, and what was
  left out (§6), with the same problems a DXF gets (zero-length pieces dropped, open
  contours, duplicates).
- **Update matching:** each shape keeps a key of its element's `id` and its subpath's
  number (`logo-path12#2`), in the field a DXF's handle uses; elements without an `id`
  are keyed by their order in the file. *Update* matches by key, as by DXF handle.
- *Implemented:* the key of a path without an `id` is `@n` (the n-th such path), then
  `#k`; a path drawn through `use` comes back from usvg without the element's id, so it is
  keyed by order too. Keys are made unique with `~2`, `~3` (duplicate ids).

## 5. Layers (decision 2)

An operation selects shapes by layer, so an SVG needs layers that mean something:

1. **Inkscape layers:** a path inside a `<g inkscape:groupmode="layer">` goes on that
   layer's label (the outermost one when layers nest). Inkscape users split parts and cut
   kinds this way.
2. **Otherwise the stroke colour,** written as `#rrggbb` (the fill colour for a path with
   no stroke; `none` for neither). Laser workflows often mean "red cuts, blue engraves".
3. A path outside any layer in a file that has layers goes on layer `0`.

## 6. Left out, and said so

- **Text** (`<text>`): not imported. Reading it would need fonts from the user's machine,
  so the same file would cut differently on two computers. The report says how many and
  advises *Path > Object to Path* (Inkscape) or *Create Outlines* (Illustrator), or
  Chipload's own text.
- **Embedded pictures** (`<image>`): not imported; counted. A picture to trace is added
  with `chipload image add`.
- **Clip paths and masks:** the clipped geometry is imported whole, unclipped, with a
  warning naming how many; a clip is a view, not a cut.
- **Patterns, markers, filters:** no geometry; ignored and counted.
- **Links to other files or the web** (`href` to a file or URL) are never followed.

## 7. Units and scale (decision 3)

- usvg gives coordinates in CSS pixels (96 per inch) mapped from the `viewBox` to the
  root's `width` and `height`. When the root size has real units (`mm`, `cm`, `in`, `pt`,
  `pc`) the scale is exact: what Inkscape (`width="210mm"`), Illustrator and Affinity
  write.
- **When it doesn't** (no size, or plain px), the scale is a guess, so it is asked once,
  as a DXF without units is: `--px-per-inch 96` (CSS, Inkscape, browsers) or `72` (older
  Illustrator), any number allowed. The window asks with 96 chosen. The answer is stored
  with the file (`units_from: user`, a new optional `px_per_inch`) and used again on
  update.
- Internally mm, as always (`units: mm` for every SVG).

## 8. Orientation

SVG's Y points down. Chipload's points up. A point (x, y) becomes (x, H − y), H the page
height, so **the page's lower-left corner is the drawing's origin** and the drawing looks
as it does in the editor. *Implemented:* H is usvg's page height, which it always gives
(from `width`/`height`, else the `viewBox`). Placement then moves, turns and scales it as a DXF's.

## 9. Hostile input (as for DXF)

- Parsed under `catch_unwind`; a panic is an import error, never a crash.
- Size limits: files over 50 MB (after gunzip) refused; over 200 000 subpaths refused with
  the count. `roxmltree` already refuses entity reference loops and has a node limit; deep
  nesting is tested (§11) and, if it can overflow the stack, refused by a depth check
  before parsing.
- Non-finite or enormous coordinates (beyond ±10 km after scaling) refused with the
  element's id.
- Degenerate pieces (zero length, collapsed curves) dropped and counted, never silently.

## 10. Command line and window

- `chipload import FILE.svg -o P [--px-per-inch N]`, `chipload dxf add P FILE.svg
  [--px-per-inch N] [--name N]`, `chipload update P --from FILE.svg [--dxf ID]`,
  `chipload inspect FILE.svg`. `dxf list` shows the format.
- Window: Open, Import and *Add drawing* accept `.svg` and `.svgz`; a dropped SVG behaves
  as a dropped DXF. The Browser group shows the SVG's name with a drawing-file icon; its
  layers are below it.

- *Implemented (step 3):* `chipload_dxf::shapes_from_file` reads a DXF or an SVG by the
  extension (`.svg`, `.svgz`); the pipeline's `add_drawing_file`, `update_drawing_file` and
  `update_drawing_with` take the pixels per inch, and the DXF functions call them. An
  SVG's counts are kept in the drawing file (`imported`: path, hidden path; `flattened`:
  curve; `ignored`: text, image, clip path or mask, empty subpath), and its issues are made
  again from them, so the report shows in the app's language (DXF issues are still stored
  in English). The added, removed and exploded messages name the format (`SVG 2`). The
  units line says `from svg`. Placement texts still say "as in the DXF" for an SVG.

- *Implemented (step 4, the window):* Open, Import, *Add DXF or SVG*, Update and dropping
  take `.svg` and `.svgz` (the file dialogs offer "Drawings (DXF, SVG)"); a file in pixels
  asks with a small window: 96, 72, or a number typed. The entry points name SVG beside
  DXF; many other texts (the Browser's and the placement form's hints, "DXF 2" in the
  selection summary) still say DXF for any drawing file.

## 11. Tests

- `chipload-svg`: hand-written SVGs in `tests/corpus/svg/`: an Inkscape file with layers in
  mm; a px-only file; nested group transforms and `use`; rect, circle, ellipse, polygon,
  arcs and Béziers (fitted within 0.01 mm of the curve, a proptest over random cubics);
  text, an image and a clip path (left out and reported); `.svgz`; hostile ones (deep
  nesting, NaN, huge numbers, an entity bomb).
- Pipeline and CLI: import, add, place, update matching by id (an element moved, one
  deleted, one added), explode, remove; the refusal without units.
- Schema 31 migration with the frozen v30 sample, goldens unchanged; one new golden: an
  SVG logo with an Inkscape layer profiled and one pocketed.
- The fuzz targets get an SVG target beside the DXF one.

## 12. Decisions

1. **An SVG in the list of drawing files** with a `format` field (schema 31), not a list of
   its own. *Recommended: yes*: every per-file feature works at once, and the user sees
   one kind of thing, a drawing file, whatever its format.
2. **Layers:** Inkscape layers when the file has them, else the stroke colour.
   *Recommended: yes.* Alternatives: always by colour, or always by top-level group id.
3. **Unitless files ask for px per inch,** 96 offered first. *Recommended: yes*, as DXFs
   without units are never guessed.
4. **Text left out** with advice, never rendered with the computer's fonts.
   *Recommended: yes*: the same file must give the same G-code everywhere.
5. **Clip paths imported unclipped** with a warning. *Recommended: yes*; clipping by
   booleans could come later if users meet it.
6. **The page's lower-left corner as origin.** *Recommended: yes*: the drawing sits where
   it does in the editor, and stays there on update.
