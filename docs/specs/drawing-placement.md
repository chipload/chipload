# Spec: placing the imported drawing

Status: **approved 2026-10-11**, with all decisions as recommended (§7).

Guide: docs/book/src/drawings/placement.md.

User report (2026-10-11): in a project started empty, with shapes drawn on the stock, an
imported DXF lands wherever its author drew it, and neither it nor the stock can be moved.

## 1. What exists

- Imported shapes are stored exactly as read from the DXF. Drawn shapes can be moved one at a
  time with their handles; nothing moves the imported drawing.
- The stock has no position of its own: `stock.origin` puts its bottom-left corner or its
  centre at the drawing origin (0, 0). That stays so (decision 3).
- `update --from` re-reads the DXF and matches shapes by DXF handle. So simply editing the
  stored segments would be undone by the next update, which would also report every shape
  as changed.

## 2. The placement

The imported drawing gets a **placement**: how far it is moved from where the DXF has it, in
mm. It is part of the import record:

```json
"source": { "file": "part.dxf", ..., "placement": { "dx": 120.0, "dy": -35.5 } }
```

- **Translation only** (decision 2). A rotation would be a later, separate change.
- The **stored shapes are the placed ones**: every imported `path` segment in the file is
  already moved by the placement. Everything that reads shapes (contours, toolpaths, the
  canvas, snapping) stays as it is; only import, update and placing know about it.
- **Update from DXF** moves the newly read shapes by the project's placement before matching
  them, so the drawing stays where it was put. Shapes count as unchanged when every value is
  within `PLACEMENT_ROUNDING_MM` (1e-9 mm) of the stored one: moving a drawing twice adds the
  moves one after the other, which may differ in the last bit from adding their sum once.
  That is only for the report: the new shapes are always the ones kept.
- The **first import** into a project (no `source` yet) starts at placement (0, 0), as drawn.
- Drawn shapes **never move** with the placement (decision 6). Nor does a work zero at a
  `point`, which is a drawing coordinate the user typed. The report of a move says so when the
  project has either.

## 3. Moving the drawing

One pipeline function, `place_drawing(job, how)`, with `how` one of:

- **By** (dx, dy): move by this much (adds to the placement).
- **To** (dx, dy): set the placement to this (moves by the difference). The GUI's fields.
- **Align bottom-left, margin m**: move so the imported shapes' bounding box has its
  bottom-left corner at the stock's bottom-left corner + (m, m). m >= 0.
- **Align centre**: move so the bounding box's centre is the stock's centre.

It moves every imported shape, adds to the placement, moves start points (below) and returns
report lines: the move, the new placement, the notes of §2, and the "extends beyond the
stock" note when it applies. The project must have imported shapes; otherwise it's an error
("nothing imported to move"). A move that isn't finite is an error. A move of (0, 0) changes
nothing and says so.

**Start points** (operations' `starts`, drawing coordinates) belong to rings of the shapes the
operation selects. A start point moves with the drawing when the shape nearest to it, among
the shapes its operation selects (by layer and ids, as planning matches them), is imported.
When the operation selects no shape at all, the nearest shape of the whole drawing decides.
Points on drawn shapes stay. So a start marker stays on its ring.

The bounding box is that of the imported shapes only (exact for arcs, as the import report's
`bbox`).

## 4. CLI

```
chipload place part.chipload --by 120,-35.5        # move by
chipload place part.chipload --to 0,0               # back to where the DXF has it
chipload place part.chipload --align bottom-left --margin 10
chipload place part.chipload --align center
chipload place part.chipload                        # print the placement, change nothing
```

Exactly one of `--by`, `--to`, `--align` (or none, to print). `--margin` only with
`--align bottom-left` (default 0). `-o` writes elsewhere, as `copies` and `update` do.
`inspect` on a project prints the placement when it isn't (0, 0).

## 5. GUI

Job settings get a **Drawing position** section (only when the project has imported shapes),
summary "as in the DXF" or "moved X… Y… from the DXF":

- *Moved from the DXF*: the placement, with *Reset* (back to the DXF's position, **To** 0, 0)
  when it isn't 0, 0.
- *On the stock*: *Bottom left* with a margin field (10 mm to start with), and *Centre*.
- *Move by X Y* with a *Move* button.

The typed values are only inputs: nothing moves until a button is pressed, so each move is
one undo step (a field applied while dragging would make one per frame). A move marks the
toolpath out of date like any other edit, and its report lines show under the canvas. No
dragging on the canvas for now.

## 6. Format: schema 8

The placement changes where the stored geometry is relative to the DXF, so it's a bump:

- `CURRENT_VERSION` 8. `drawing.source.placement` is required in v8 (`{"dx": 0, "dy": 0}` as
  drawn).
- Migration v7 → v8: a project with a `source` gets placement (0, 0); its shapes are where
  the DXF has them, which is exactly what (0, 0) means. Projects without a source are
  unchanged. Machining is unchanged.
- A frozen v7 sample already exists for every golden case; one of them
  (`tests/corpus/jobs/v7/adaptive.chipload` or another with an imported drawing) must still
  give its golden G-code, which the golden test already checks for every frozen sample.
- A frozen v8 sample with a non-zero placement and a start point, plus its golden G-code.

## 7. Decisions (approved 2026-10-11)

1. What moves: the imported drawing as a whole, by a placement stored with the import and
   applied on every update. Moving a selection of drawn shapes is a separate, later feature.
2. Rotation: translation only for now.
3. Stock: stays tied to the drawing origin; the drawing is moved onto it. The work zero
   already puts X0 Y0 on any stock corner or the centre.
4. How: move by / set to an offset, and align bottom-left (with margin) or centre on the
   stock, in the CLI (`chipload place`) and the GUI job settings.
5. Schema bump to 8 with a migration.
6. Drawn shapes and a work zero at a point stay where they are; the report says so.

## 8. Tests

- Job: migration v7 → v8 adds (0, 0) only with a source; v8 without placement is refused.
- Pipeline: by / to / align bottom-left with margin / align centre give the expected bbox;
  drawn shapes don't move; start points on imported shapes move and on drawn shapes don't;
  update after a move keeps the placement and reports the shapes unchanged (also after two
  moves); first import into an empty project is at (0, 0); no imported shapes is an error;
  non-finite and zero moves.
- CLI: `place` with each option; `--margin` without bottom-left is refused; the moved project
  runs through the checker; `inspect` shows the placement.
- GUI: the section appears only with imported shapes; the buttons move the drawing; undo.
- Golden: the new v8 sample. Existing golden files unchanged.
