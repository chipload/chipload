# Drawing shapes with the mouse: the status bar while drawing, and snapping
# (crates/chipload-gui/src/draw.rs). Esc, Enter and Backspace are keyboard keys; the
# grid is the step clicks are rounded to, e.g. "1 mm".

draw-status-circle-start = Drawing a circle: click the centre (grid { $grid }, Esc cancels)
draw-status-circle = Drawing a circle radius { $radius }: click a point on the circle (grid { $grid }, Esc cancels)
draw-status-rectangle-start = Drawing a rectangle: click a corner (grid { $grid }, Esc cancels)
# The rectangle's size so far: along X, then along Y.
draw-status-rectangle = Drawing a rectangle { $length } x { $width }: click the opposite corner (grid { $grid }, Esc cancels)
draw-status-polyline-start = Drawing a polyline { $count ->
        [one] { $count } point
       *[other] { $count } points
    }: click the first point (grid { $grid }, Esc cancels)
draw-status-polyline = Drawing a polyline { $count ->
        [one] { $count } point
       *[other] { $count } points
    }: click the next point; Enter or double-click finishes it open, the first point closes it, Backspace removes the last point (grid { $grid }, Esc cancels)

# Why a click did nothing.
draw-no-size-circle = the circle would have no size
draw-no-size-rectangle = the rectangle would have no size
draw-polyline-too-short = a polyline needs at least 2 points

# The point the cursor snapped to, on contour #4.
draw-snap-end = end of #{ $contour }
draw-snap-centre = centre of #{ $contour }
draw-snap-middle = middle of #{ $contour }
