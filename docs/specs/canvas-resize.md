# Spec: the canvas follows its size

Status: approved (2026-11-07; decisions 1–2 as recommended, the user: "go ahead and
implement these changes"); built.

## 1. Why

The user (2026-11-07): "when resizing the window i think the canvas zoom should resize as
well to follow, perhaps also run 'fit'". The view was pinned to the canvas's top-left
corner at a fixed scale: a smaller window cut the drawing off, a larger one left it in a
corner. The canvas also changes size when a panel opens or folds (the Report opens by
itself on a warning) or a panel's edge is dragged.

## 2. Decisions

1. **The view keeps what it shows.** When the canvas changes size or place, the area it
   showed is shown again, all of it, centred, as large as fits. A shrink and a grow back
   give the same view as before.
2. **A fitted view stays fitted.** A view as Fit left it (no pan, zoom or turn since) is
   fitted again to the same things. A view zoomed in by hand keeps its zoom: a resize never
   throws it away for a fit.

What the view is meant to show is taken again each time it changes: after a fit (the
points it fitted), after a pan, a zoom, a turn or a view button (the area then visible, in
the camera's units, so in 3D too).

## 3. As built

- `View::visible_area(rect) -> Area` and `View::show_area(area, rect)` (gui `view.rs`).
- `ChiploadApp::keep_view` in `canvas.rs`, each frame after a pending fit: `KeptView`
  (the canvas rect and view as last drawn, and the aim: `Fit(points)` or `Area`). A view
  unlike the one last drawn takes a new `Area` aim; a rect unlike the last is given the aim
  again.
- Tests: `view.rs` (an area shown is the area visible), `three_d_view.rs` (a fitted view
  fitted again, a zoomed view keeps its area and comes back after a shrink and a grow).
