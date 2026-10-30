# Spec: refuse contours that fold back on themselves

Status: **approved 2026-10-10** (Q1–Q3 in §5: all as recommended).

Guide: docs/book/src/drawings/import.md.

User decision 2026-10-10: degenerate outlines are rejected up front, with a clear message,
instead of failing later inside the geometry library.

## 1. The problem

The pocket property test generated this outline (shortened): eight vertices at almost the
same angle from the centre, so the boundary runs out along y ≈ 0, back, out again, back…

```
(43.42, 0.000) → (30.49, 0.005) → (41.94, 0.015) → (54.76, 0.029) → (33.95, 0.024) → …
```

The edges never cross, so import's self-intersection check passes it. But they run side by
side 0.005–0.02 mm apart over 10–25 mm. No tool can follow a boundary like that, and the
drawing almost certainly doesn't mean it.

Correction (2026-10-10, after approval): this outline was first blamed for "Pocket offset
failed inside the geometry library" in the property tests. That was wrong. Pocketing it works
in debug and release builds. Its failure was in the material-left code (already handled
there), and the pocket test failures came from a valid jagged island tripping a debug-only
assertion in cavalier_contours, dealt with in ADR 0012. The rule below still stands, as a
drawing check: refuse what can't be machined as drawn, and say where, instead of producing
a toolpath that silently ignores a needle or slit.

A real drawing gets this from bad exports or from a polyline drawn over itself: a needle,
a slit drawn as two lines on top of each other, or a doubled-back edge.

## 2. Rule

A closed contour **folds back on itself** if two points of its boundary are:
- closer than `ARC_APPROX_TOL_MM` (0.01 mm), **and**
- more than `FOLD_MIN_PATH_MM` (1 mm) apart *along* the boundary, taking the shorter way
  round.

The second condition makes sure ordinary tiny segments, noise at corners, and points next
to each other along the line are never flagged: they are close in space *and* along the path.
A needle or slit is close in space but far apart along the path.

Equivalently, for a sharp V-notch: it's flagged only if the notch is narrower than 0.01 mm
at a point more than 0.5 mm from its tip, i.e. an angle under about 1.1°. No endmill can cut
into that.

## 3. Where

- **On import** (`chipload-dxf`, `classify`), next to the self-intersection check: the
  contour gets `invalid_reason = "folds back on itself: its edges come within 0.010 mm of
  each other near (x, y)"`, plus an import issue (error) with that location. This is the
  existing mechanism for invalid contours:
  - `inspect` lists it;
  - the GUI draws it red;
  - selecting it in an operation fails with
    `Operation 1: contour #4 is invalid: folds back on itself: …`.
- The check is a pure function in `chipload-core` (no I/O), so the toolpath property tests
  use the same rule to generate only valid outlines.
- How: sample the flattened boundary every `ARC_APPROX_TOL_MM`. For each sample, find the
  segments within the tolerance (grid index), the nearest point on each, and the distance
  between the two along the boundary. The first hit in boundary order is the reported
  location (deterministic). Only the samples of an edge inside the other edge's box grown by
  the tolerance are tried (2026-10-30): none outside it can be that near, and a 150 m
  contour from a fuzzed SVG took 17 s sampling all of them.

Existing projects: contours are rebuilt from the embedded shapes every time a project is
loaded, so a project holding such a contour starts refusing it. That's intended: it was
never safe. No schema change.

## 4. Tests

- **Unit (core):**
  - The outline from §1 is flagged, near its spikes.
  - A needle: a square with a 10 mm long, 0.005 mm wide spike → flagged.
  - A 2° V-notch 10 mm deep → not flagged; a 0.5° one → flagged.
  - A square with a 0.005 mm jog at a corner, and a contour with many 0.002 mm segments
    → not flagged.
  - Circles and arcs flattened at the arc tolerance → not flagged.
- **Corpus:** no contour in `tests/corpus/dxf/` is newly invalid (the golden test would also
  show it).
- **Import:** the invalid reason and the issue appear with the location; selecting the
  contour gives the selector error.
- **Property tests:** the pocket generator skips outlines the rule flags, since import never
  passes them on. (The library failures they hit are handled by ADR 0012, not by this rule.)

## 5. Decisions (approved 2026-10-10)

1. **Thresholds**: within 0.01 mm (`ARC_APPROX_TOL_MM`), more than 1 mm apart along the
   boundary (new constant `FOLD_MIN_PATH_MM`). **Yes.**
2. **Error, not warning** on import, like self-intersection, so the contour can't be cut
   until the drawing is fixed. **Error**: Chipload can't offset it reliably.
3. **Open contours** are not checked: no operation cuts them today (selection skips them),
   so they're never offset. **Skip them**; revisit when open contours become
   cuttable.
