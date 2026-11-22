# Dragging the handles of a picture's or text's box (crates/chipload-gui/src/app/box_drag.rs).

# In the status bar when the box can't be dragged to that size; $error says why.
box-drag-not-that-size = Not that size: { $error }
box-drag-cannot-move-shapes = Could not move the shapes:
box-drag-stretched-fitted = { $count ->
    [one] { $shapes } is now a polyline
   *[other] { $shapes } are now polylines
} of lines and arcs fitted to the stretched curve.
