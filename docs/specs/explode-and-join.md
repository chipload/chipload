# Spec: explode a DXF into drawn shapes, join close ends

Status: **draft** (2026-10-25), decisions in §8 waiting for review.

The user's request (2026-10-25, from a user): explode a DXF into drawing elements that can
be reworked in the app, losing the connection to the DXF; and an option to join line
segments whose ends are very close into shapes.

## 1. Today

- A DXF's entities are `path` shapes (one per entity: lines and arcs as read, already
  placed on the stock), each naming its DXF (`dxf`) and entity (`dxf_handle`). They are
  **read-only**: no handles, no editing, no deleting one by one (drawing-tools.md §3).
- Drawn shapes (`circle`, `rectangle`, `polyline` with bulges) are editable with handles,
  the form, mirror, copy and delete.
- Open shapes whose ends meet within `CHAIN_TOL_MM` (0.001 mm) are already chained into one
  contour. Ends a little further apart (a sloppy drawing: 0.01–0.5 mm) stay **open
  contours**, which can't be pocketed or profiled inside/outside. Today the only fix is
  redrawing in the CAD program.

## 2. Explode a DXF

*Explode* turns **every shape of one DXF** into drawn shapes and removes the DXF from the
project (`drawing.dxfs`, its hidden-group keys `dxf:N…`). The DXF's file name, import
report and placement go with it; *Update from DXF* is no longer possible for those shapes.

Per shape, in order:

- **A full circle** (one arc of sweep 2π starting at angle 0, what a DXF `CIRCLE` gives)
  becomes a `circle`.
- **Anything else** becomes a `polyline`: its segments' points, `closed` as the path was,
  each line bulge 0 and each arc its bulge (tan(sweep / 4)). An arc of more than a half
  circle is split at its middle into two segments, since a bulge is at most 1
  (drawing-follow-ups.md §3). A full circle that doesn't start at angle 0 becomes a closed
  polyline of two half circles, so its contour starts where it did.
- **A path whose segments don't follow on** (one end more than `CHAIN_TOL_MM` from the next
  start, e.g. a block's content) becomes one polyline per run. The first keeps the id; the
  others get new ids.
- Segments shorter than `CHAIN_TOL_MM` are left out (the polyline rule; the import report
  already listed them).

What stays:

- **Ids and layers.** Every shape keeps its id and layer, so operations that selected it
  still do.
- **Selectors limited to that DXF** (`{"layer": "Cut", "dxf": 2}`, with or without
  `exclude`) are rewritten to the ids they matched (`{"shapes": [...]}`): a layer of the same
  name elsewhere doesn't join in. Ids of split paths include the new ids.
- **The G-code.** The same geometry in the same order: a project's program before and after
  exploding is the same (§7 tests it on the corpus).

Explode is one edit (one undo step in the app) and needs no schema change: the result is
ordinary drawn shapes.

## 3. Join close ends

*Join* merges open drawn polylines whose ends lie within a **gap** of each other into one
polyline, and closes a polyline whose own ends lie within the gap. Available:

- as an option of *Explode* ("Join ends closer than [0.05 mm]"), and
- on its own, on the selected drawn shapes (or all drawn shapes when nothing is selected),
  so a drawing exploded earlier, or drawn by hand, can be joined later.

Rules:

1. **Only open polylines on the same layer** take part. Circles, rectangles and closed
   polylines are left alone.
2. **Only unambiguous ends join.** An end joins another end only when each is the other's
   only end within the gap. Where three or more ends meet (a T, a branch), nothing joins
   there and the report names the point.
3. **The gap is closed by moving both ends to their midpoint**: no tiny bridging segment.
   An arc keeps its bulge, so it changes by at most half the gap.
4. **Order and direction:** a chain starts at the lowest id among its pieces and follows
   that piece's direction; pieces are reversed where needed (points reversed, bulges
   negated). The joined polyline keeps that lowest id and its layer; the other pieces are
   removed.
5. **A chain that comes back to its start** within the gap is closed.
6. **Operations** that selected a removed piece select the kept id instead (once).
7. **Deterministic:** the same shapes and gap always give the same result.

The gap is at least `CHAIN_TOL_MM` and at most 1 mm; the default is 0.05 mm (it's shown in
the display units).

**The report** says what happened, in the app's status line and the command's output:
"Joined 48 shapes into 6, 4 of them closed; widest gap closed 0.032 mm. 3 ends stay open:
the nearest other end is 0.4 mm away at (120.0, 35.5) mm." The open ends' nearest
distances tell the user what gap would close them.

## 4. Command line (first, rule 5)

```
chipload dxf explode PROJECT ID [--join GAP] [-o OUT]       # GAP takes a unit, as every length
chipload shape join PROJECT [IDS…] [--gap GAP] [-o OUT]     # no IDS: every drawn shape
```

With one DXF in the project, `dxf explode PROJECT` needs no ID. `--json` gives the
new and removed ids, the rewritten selectors and the join report. MCP gets both as tools.

## 5. App

- **Browser:** a DXF's context menu gets *Explode into shapes…*. A small dialog: what it
  does ("The shapes become drawn shapes you can edit. The project no longer knows the DXF
  file: *Update from DXF* won't be possible."), the *Join ends closer than* checkbox and
  field, *Explode* and *Cancel*.
- **Draw ribbon / shape context menu:** *Join ends…* with the gap field, on the selection.
- The exploded shapes appear in the drawn groups (*Polylines*, *Circles*), keeping their
  layers.
- One undo step each.
- Every new text in all six languages.

## 6. Where the code lives

- `chipload-job` (`edit`): path → drawn geometry, and the join of polylines (pure, no
  I/O).
- `chipload-pipeline` (`dxfs.rs` beside `remove_dxf`, and `shapes.rs`): explode and join
  on a job, selector rewriting, the report as `Message`s.
- CLI, MCP, then the GUI. Guide: *Drawings > Shapes* (join) and *Drawings > DXF files*
  (explode).

## 7. Tests

- **Exploding keeps the G-code:** every corpus project with a DXF, exploded, gives the same
  G-code as before (golden comparison, no new goldens).
- Paths with lines, arcs below and above a half circle, full circles at angle 0 and not,
  clockwise arcs, and a path in two runs; segment end points within `CHAIN_TOL_MM` of the
  original.
- Selectors: `{"layer", "dxf"}` and `exclude` rewritten to ids; layer-only selectors
  unchanged; the DXF and its hidden keys gone.
- Join: two pieces 0.03 mm apart join with gap 0.05 and not with 0.01; a chain that closes;
  reversed pieces; a T of three ends stays open and is reported; different layers don't
  join; an arc piece keeps its bulge; operations follow the kept id; same input, same output
  (a `proptest`: joining never moves a point by more than half the gap, and never changes the
  number of segments).
- Gap limits: below `CHAIN_TOL_MM` or above 1 mm refused.
- CLI: `dxf explode` and `shape join`, with and without `--json`.

## 8. Decisions to review

1. **Explode the whole DXF, not single shapes.** *Recommended:* a DXF stays either linked
   (updatable) or gone; half-exploded DXFs would make *Update from DXF* ambiguous (does it
   bring an exploded entity back?). Alternative: explode selected shapes only, the rest stay
   linked.
2. **Shapes keep their ids, and DXF-limited selectors become id lists**, so operations keep
   cutting exactly what they cut. *Recommended.*
3. **Full circles become circles; everything else polylines** (no guessing rectangles from
   four lines). *Recommended:* a polyline is fully editable; recognising rectangles is a
   guess that can be wrong (a nearly-square trapezoid).
4. **Join is also a command of its own** on drawn shapes, not only an option of explode.
   *Recommended:* the same function, and useful for hand-drawn or earlier-exploded shapes.
5. **Join only the same layer, and only unambiguous ends.** *Recommended:* different layers
   usually mean different operations; at a branch any choice would be a guess.
6. **Close a gap by moving both ends to the midpoint**, not by a bridging segment.
   *Recommended:* no tiny segments for the toolpath; the change is at most half the gap.
7. **Gap default 0.05 mm, at most 1 mm.** *Recommended:* catches CAD rounding and sloppy
   snaps; above 1 mm a "gap" is usually meant.
8. **No undo of an explode beyond the app's undo** (a saved project can't get its DXF link
   back, short of *DXF > Add* again). *Recommended*, and the dialog says so.
