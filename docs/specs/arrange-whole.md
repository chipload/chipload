# Spec: align and distribute move whole contours and groups

Status: approved (2026-11-07; decisions 1–3 as recommended, the user: "go ahead with the
plan"); built. Issue #13.

Builds on docs/specs/drawing-editing.md §7 (align and distribute) and shape-groups.md.

## 1. Why

Issue #13 (Julien, Chipload 0.30): a slot drawn as four open pieces, which chain into one
closed contour, was centred on a rectangle with Align. Each piece moved to the centre on
its own; the slot fell apart into four open contours, which aren't cut. A group fell
apart the same way, and a group with a text couldn't be aligned at all (its letters aren't
drawn shapes, so the window hid Align).

Reproduced on 0.34: `shared_edges.dxf` exploded, `shape align P 12 13 14 15 47 --to
centre` left contours #10–#13 open.

## 2. Decisions

1. **The unit is a contour.** Selected shapes that chain into one contour line up as one:
   one shift, from the box round them. A shape left out as a duplicate of others (a twin)
   goes with them.
2. **A group selected whole is one unit,** its texts moving with it (`transform_group`).
   A group counts whole when all its shapes, letters included, are selected; then its
   letters may be named too. One shape picked inside a group (a double-click) moves alone,
   as Move does. Any other shape must still be a drawn one.
3. **Counts are of units:** Align needs two units (one with *To the stock*), Distribute
   three. The window enables its buttons by the same count, and offers Align for a group
   picked whole.

The command line gets the same through the pipeline (`shape align`, `shape distribute`).

## 3. As built

- `chipload_pipeline::drawing_edit::arrange_units(job, ids)` and `arrange_units_in(job,
  contours, twins, ids)` (the window's contours) give `ArrangeUnit { shapes, groups }`, in
  the order their first shape is named; `align_shapes` and `distribute_shapes` move units.
- GUI: `arrange_selection` (the selection's shapes, or a whole group's, and the number of
  units).
- Tests: `crates/chipload-pipeline/tests/arrange.rs` (the issue's slot aligned and spaced
  out whole, a group with its text, one shape of a group alone, a lone letter refused);
  GUI `groups.rs` (a group picked whole aligned to the stock, its text with it).
