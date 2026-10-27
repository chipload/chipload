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
draw-snap-intersection = intersection of #{ $contour } and #{ $other }
draw-snap-intersection-self = intersection on #{ $contour }
draw-snap-quadrant = quadrant of #{ $contour }
draw-snap-perpendicular = perpendicular to #{ $contour }
draw-snap-tangent = tangent to #{ $contour }
draw-snap-nearest = on #{ $contour }

# A value typed while drawing that can't be used.
draw-entry-unreadable = can't read that: type x,y  @dx,dy  a length, or length<angle
draw-entry-needs-point = the first point: type x,y
draw-entry-no-direction = move the cursor away from the last point to give the direction
draw-status-arc-start = Drawing an arc: click where it starts (grid { $grid }, Esc cancels)
draw-status-arc-end = Drawing an arc, chord { $length }: click where it ends (grid { $grid }, Esc cancels)
draw-status-arc = Drawing an arc radius { $radius }: click a point it passes through, or type its radius (grid { $grid }, Esc cancels)
draw-status-slot-start = Drawing a slot: click the centre of one end (grid { $grid }, Esc cancels)
draw-status-slot-end = Drawing a slot { $length } between the ends: click the centre of the other end (grid { $grid }, Esc cancels)
draw-status-slot = Drawing a slot width { $width }: click to give its width, or type it (grid { $grid }, Esc cancels)
draw-arc-on-a-line = the three points lie on a line: use the polyline tool
draw-no-size-slot = the slot would have no length or no width
draw-entry-radius-too-small = the radius must be at least half the distance between the ends
draw-entry-no-side = move the cursor to the side the arc should bow to
draw-status-polygon-start = Drawing a polygon: click the centre (grid { $grid }, Esc cancels)
draw-status-polygon = Drawing a polygon radius { $radius }: click a corner (grid { $grid }, Esc cancels)
# Editing drawn shapes (docs/specs/drawing-editing.md).
modify-offset-pick = Offset: select the shapes to offset (click one; Ctrl adds), Esc ends
modify-offset-status = Offset { $distance } { $side }: click, or type the distance (Alt: off the grid, Esc ends)
modify-offset-done = Added { $count ->
    [one] an offset
   *[other] { $count } offsets
} at { $distance }.
modify-side-outside = outside
modify-side-inside = inside
modify-side-left = to the left
modify-side-right = to the right
modify-corners-round = Round corners
modify-corners-sharp = Sharp corners
modify-corners-hover = Round: the exact offset, arcs round the corners. Sharp: the sides extended until they meet, as a frame.
modify-offset-count = Count
